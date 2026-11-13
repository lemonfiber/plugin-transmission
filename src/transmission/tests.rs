use std::sync::Arc;

use lemonfiber_fixtures::http::{Answer, Fake};
use lemonfiber_ports::http::Request;
use lemonfiber_ports::service::{Failure, Transfers};

use super::Transmission;
use crate::credential::{Credential, Holding};
use crate::kept::tests::Remembered;

pub(super) const BASE: &str = "http://127.0.0.1:9091";

pub(super) const DONE: &str = "{}";

pub(super) fn a_password() -> String {
    ('c'..='r').collect()
}

pub(super) fn answered(result: &str) -> Answer {
    Answer::reply(
        200,
        format!(r#"{{"jsonrpc":"2.0","id":1,"result":{result}}}"#),
    )
}

pub(super) fn holding(torrents: &str) -> Answer {
    answered(&format!(r#"{{"torrents":{torrents}}}"#))
}

pub(super) fn keeping(answers: Vec<Answer>, kept: Arc<Remembered>) -> (Transmission, Arc<Fake>) {
    let http = Fake::in_turn(answers);
    let held = Credential {
        username: "admin".to_owned(),
        password: a_password(),
    };
    (
        Transmission::new(http.clone(), BASE, Arc::new(held), kept),
        http,
    )
}

pub(super) fn client(answers: Vec<Answer>) -> (Transmission, Arc<Fake>) {
    keeping(answers, Arc::new(Remembered::default()))
}

pub(super) fn called(http: &Fake) -> Vec<serde_json::Value> {
    http.requests()
        .into_iter()
        .filter_map(|one| one.body)
        .filter_map(|body| serde_json::from_str(&body).ok())
        .collect()
}

pub(super) fn call(method: &str, params: serde_json::Value) -> serde_json::Value {
    serde_json::Value::Object(
        [
            ("jsonrpc", serde_json::json!("2.0")),
            ("method", serde_json::json!(method)),
            ("params", params),
            ("id", serde_json::json!(1)),
        ]
        .into_iter()
        .map(|(key, value)| (key.to_owned(), value))
        .collect(),
    )
}

fn header<'a>(request: &'a Request, name: &str) -> Option<&'a str> {
    request
        .headers
        .iter()
        .find(|(held, _)| held == name)
        .map(|(_, value)| value.as_str())
}

fn stale(session: &str) -> Answer {
    Answer::Headed {
        status: 409,
        headers: vec![("x-transmission-session-id".to_owned(), session.to_owned())],
        body: "<h1>409: Conflict</h1>".to_owned(),
    }
}

struct Nothing;

impl Holding for Nothing {
    fn credential(&self) -> Result<Credential, String> {
        Err("upstream.json could not be read: not found".to_owned())
    }
}

#[tokio::test]
async fn every_call_is_a_json_rpc_post_signed_with_the_minted_account() {
    let (transmission, http) = client(vec![holding("[]")]);
    assert_eq!(transmission.transfers().await.ok(), Some(Vec::new()));
    let sent = http.requests();
    let seen: Vec<_> = sent
        .iter()
        .map(|one| {
            (
                one.url.as_str(),
                header(one, "Content-Type"),
                header(one, "Authorization"),
                header(one, "X-Transmission-Session-Id"),
            )
        })
        .collect();
    let url = format!("{BASE}/transmission/rpc");
    assert_eq!(
        seen,
        [(
            url.as_str(),
            Some("application/json"),
            Some("Basic YWRtaW46Y2RlZmdoaWprbG1ub3Bxcg=="),
            None
        )]
    );
    assert_eq!(
        called(&http)
            .first()
            .and_then(|body| body.get("method").cloned()),
        Some(serde_json::json!("torrent_get"))
    );
}

#[tokio::test]
async fn a_stale_session_is_taken_from_the_refusal_asked_again_and_kept() {
    let (transmission, http) = client(vec![
        stale("first"),
        holding("[]"),
        holding("[]"),
        stale("second"),
        holding("[]"),
    ]);
    for _ in 0..3 {
        assert_eq!(transmission.transfers().await.ok(), Some(Vec::new()));
    }
    let sent = http.requests();
    let sessions: Vec<Option<&str>> = sent
        .iter()
        .map(|one| header(one, "X-Transmission-Session-Id"))
        .collect();
    assert_eq!(
        sessions,
        [
            None,
            Some("first"),
            Some("first"),
            Some("first"),
            Some("second")
        ]
    );
}

#[tokio::test]
async fn a_session_refusal_naming_no_session_is_refused_and_a_second_one_is_said() {
    let (named_none, _) = client(vec![Answer::reply(409, "Conflict")]);
    assert!(matches!(
        named_none.transfers().await,
        Err(Failure::Refused { detail, .. }) if detail == "Transmission asked for a session and named none"
    ));
    let (again, _) = client(vec![stale("one"), stale("two")]);
    assert!(matches!(
        again.transfers().await,
        Err(Failure::Refused { detail, .. }) if detail.starts_with("HTTP 409")
    ));
}

#[tokio::test]
async fn a_refused_account_is_unauthorised() {
    for refused in [
        Answer::reply(401, "<h1>401: Unauthorized</h1>"),
        Answer::reply(403, "<h1>403: Forbidden</h1>"),
    ] {
        let (transmission, http) = client(vec![refused]);
        assert!(matches!(
            transmission.transfers().await,
            Err(Failure::Unauthorised { .. })
        ));
        assert_eq!(http.requests().len(), 1);
    }
}

#[tokio::test]
async fn an_error_transmission_answers_is_refused_in_its_own_words() {
    let (transmission, _) = client(vec![Answer::reply(
        200,
        r#"{"jsonrpc":"2.0","id":1,"error":{"code":7,"message":"HTTP error from backend service","data":{"error_string":"Couldn't test port"}}}"#,
    )]);
    assert!(matches!(
        transmission.transfers().await,
        Err(Failure::Refused { detail, .. })
            if detail == "the torrent list could not be read: Transmission said HTTP error from backend service: Couldn't test port"
    ));
    for bare in [
        r#"{"jsonrpc":"2.0","id":1,"error":{"code":-32601,"message":"Method not found"}}"#,
        r#"{"jsonrpc":"2.0","id":1,"error":{"code":-32601,"message":"Method not found","data":{}}}"#,
    ] {
        let (transmission, _) = client(vec![Answer::reply(200, bare)]);
        assert!(matches!(
            transmission.transfers().await,
            Err(Failure::Refused { detail, .. })
                if detail == "the torrent list could not be read: Transmission said Method not found"
        ));
    }
}

#[tokio::test]
async fn an_answer_with_neither_a_result_nor_an_error_is_refused() {
    let (transmission, _) = client(vec![Answer::reply(200, r#"{"jsonrpc":"2.0","id":1}"#)]);
    assert!(matches!(
        transmission.transfers().await,
        Err(Failure::Refused { detail, .. })
            if detail == "the torrent list could not be read: Transmission said nothing"
    ));
    let (misshapen, _) = client(vec![Answer::reply(200, "<html>")]);
    assert!(matches!(
        misshapen.transfers().await,
        Err(Failure::Refused { .. })
    ));
}

#[tokio::test]
async fn without_a_credential_nothing_is_asked_and_the_refusal_says_why() {
    let http = Fake::silent();
    let refused = Transmission::new(
        http.clone(),
        BASE,
        Arc::new(Nothing),
        Arc::new(Remembered::default()),
    )
    .transfers()
    .await;
    assert!(
        matches!(&refused, Err(Failure::Refused { detail, .. })
            if detail == "this adapter holds no credential for Transmission: upstream.json could not be read: not found"),
        "{refused:?}"
    );
    assert!(http.requests().is_empty());
}

#[tokio::test]
async fn it_answers_while_its_rpc_interface_answers_signed_in_or_not() {
    for answer in [
        Answer::reply(401, "<h1>401: Unauthorized</h1>"),
        Answer::reply(409, "<h1>409: Conflict</h1>"),
        Answer::reply(200, "{}"),
    ] {
        let up = Fake::always(answer);
        let transmission = Transmission::new(
            up.clone(),
            BASE,
            Arc::new(Nothing),
            Arc::new(Remembered::default()),
        );
        assert!(transmission.answers().await);
        assert!(up
            .requests()
            .iter()
            .all(|one| one.url == format!("{BASE}/transmission/rpc")
                && one.headers.is_empty()
                && one.body.is_none()));
    }
    for down in [Fake::silent(), Fake::always(Answer::reply(503, ""))] {
        let transmission = Transmission::new(
            down,
            BASE,
            Arc::new(Nothing),
            Arc::new(Remembered::default()),
        );
        assert!(!transmission.answers().await);
    }
}

#[tokio::test]
async fn a_client_that_stops_answering_is_unavailable() {
    let (transmission, _) = client(vec![stale("one")]);
    assert!(matches!(
        transmission.transfers().await,
        Err(Failure::Unavailable { .. })
    ));
}
