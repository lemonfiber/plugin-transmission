use std::sync::Arc;

use axum::body::Body;
use axum::http::{header, Request, StatusCode};
use lemonfiber_contract::adapter::{about_path, ready_path, About};
use lemonfiber_fixtures::http::{Answer, Fake};
use tower::ServiceExt as _;

use super::{about, kit, DIGEST, RELEASE, UPSTREAM};
use crate::credential::Credential;
use crate::kept::tests::Remembered;
use crate::transmission::Transmission;

fn a_key() -> String {
    ('e'..='t').collect()
}

fn transmission(fake: &Arc<Fake>) -> Arc<Transmission> {
    Arc::new(Transmission::new(
        fake.clone(),
        "http://transmission:9091",
        Arc::new(Credential {
            username: "admin".to_owned(),
            password: ('a'..='p').collect(),
        }),
        Arc::new(Remembered::default()),
    ))
}

async fn answered_by(
    fake: &Arc<Fake>,
    method: &str,
    path: &str,
    key: Option<&str>,
    body: &str,
) -> (StatusCode, Vec<u8>) {
    let mut request = Request::builder().method(method).uri(path);
    if let Some(key) = key {
        request = request.header(header::AUTHORIZATION, format!("Bearer {key}"));
    }
    let Ok(request) = request.body(Body::from(body.to_owned())) else {
        return (StatusCode::IM_A_TEAPOT, Vec::new());
    };
    let Ok(response) = kit(a_key(), transmission(fake))
        .router()
        .oneshot(request)
        .await;
    let status = response.status();
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .map(|bytes| bytes.to_vec())
        .unwrap_or_default();
    (status, bytes)
}

async fn answered(
    method: &str,
    path: &str,
    key: Option<&str>,
    body: &str,
) -> (StatusCode, Vec<u8>) {
    answered_by(&Fake::silent(), method, path, key, body).await
}

#[test]
fn it_speaks_the_torrent_contract_and_names_the_release_its_recordings_were_taken_from() {
    let said = about();
    assert_eq!(said.speaks, ["download.torrent@1"]);
    assert_eq!(said.upstream, UPSTREAM);
    assert_eq!(
        said.releases
            .iter()
            .map(|release| (release.version.as_str(), release.digest.as_str()))
            .collect::<Vec<_>>(),
        [(RELEASE, DIGEST)]
    );
}

#[tokio::test]
async fn about_is_answered_under_the_plugin_s_key() {
    let key = a_key();
    let (status, body) = answered("GET", &about_path(), Some(&key), "").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(serde_json::from_slice::<About>(&body).ok(), Some(about()));
}

#[tokio::test]
async fn nothing_is_answered_without_the_key_or_under_another() {
    let operation = "/lemonfiber/download.torrent/v1/transfers";
    for key in [None, Some("not-the-plugin-s-key")] {
        assert_eq!(
            answered("POST", operation, key, "{}").await.0,
            StatusCode::UNAUTHORIZED
        );
        assert_eq!(
            answered("GET", &about_path(), key, "").await.0,
            StatusCode::UNAUTHORIZED
        );
    }
}

#[tokio::test]
async fn an_unreachable_transmission_is_answered_as_the_refusal_the_contract_declares() {
    let key = a_key();
    let (status, body) = answered(
        "POST",
        "/lemonfiber/download.torrent/v1/transfers",
        Some(&key),
        "{}",
    )
    .await;
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
    let refusal: Option<serde_json::Value> = serde_json::from_slice(&body).ok();
    assert_eq!(
        refusal.and_then(|refusal| refusal.get("type").cloned()),
        Some(serde_json::json!("unavailable"))
    );
}

#[tokio::test]
async fn an_operation_is_answered_from_what_transmission_says() {
    let key = a_key();
    let fake = Fake::always(Answer::reply(
        200,
        r#"{"jsonrpc":"2.0","id":1,"result":{"peer_port":51413}}"#,
    ));
    let (status, body) = answered_by(
        &fake,
        "POST",
        "/lemonfiber/download.torrent/v1/peer_port",
        Some(&key),
        "{}",
    )
    .await;
    assert_eq!(
        (status, body.as_slice()),
        (StatusCode::OK, b"51413".as_slice())
    );
}

#[tokio::test]
async fn a_capability_it_does_not_speak_is_refused_as_unknown() {
    let key = a_key();
    let (status, _) = answered(
        "POST",
        "/lemonfiber/download.usenet/v1/transfers",
        Some(&key),
        "{}",
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn it_is_ready_while_transmission_answers_its_rpc_interface() {
    let key = a_key();
    let up = Fake::always(Answer::reply(401, "<h1>401: Unauthorized</h1>"));
    let (status, body) = answered_by(&up, "GET", &ready_path(), Some(&key), "").await;
    assert_eq!(
        (status, body.as_slice()),
        (StatusCode::OK, b"true".as_slice())
    );
    assert!(up
        .requests()
        .iter()
        .all(|asked| asked.url == "http://transmission:9091/transmission/rpc"));
    for down in [Fake::silent(), Fake::always(Answer::reply(503, ""))] {
        let (status, body) = answered_by(&down, "GET", &ready_path(), Some(&key), "").await;
        assert_eq!(
            (status, body.as_slice()),
            (StatusCode::OK, b"false".as_slice())
        );
    }
}
