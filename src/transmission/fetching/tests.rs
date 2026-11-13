use lemonfiber_ports::service::{Fetching, Pulling};
use serde_json::json;

use super::super::tests::{answered, call, called, client, holding, DONE};

const RUNNING: &str = r#"[{"status":4},{"status":0}]"#;

const IDLE: &str = r#"[{"status":0}]"#;

const STARTS: &str = r#"{"start_added_torrents":true}"#;

const WAITS: &str = r#"{"start_added_torrents":false}"#;

#[tokio::test]
async fn a_client_is_fetching_while_it_runs_something_or_would_start_the_next() {
    for (running, adding, pulling) in [
        ("[]", STARTS, Pulling::Fetching),
        (RUNNING, WAITS, Pulling::Fetching),
        (IDLE, WAITS, Pulling::Stopped),
        ("[]", WAITS, Pulling::Stopped),
    ] {
        let (transmission, _) = client(vec![holding(running), answered(adding)]);
        assert_eq!(transmission.pulling().await.ok(), Some(pulling));
    }
}

#[tokio::test]
async fn stopping_it_stops_new_torrents_starting_before_it_stops_every_torrent() {
    let (transmission, http) = client(vec![
        answered(DONE),
        answered(DONE),
        holding(IDLE),
        answered(WAITS),
    ]);
    assert_eq!(transmission.stop().await.ok(), Some(Pulling::Stopped));
    assert_eq!(
        called(&http),
        [
            call("session_set", json!({"start_added_torrents": false})),
            call("torrent_stop", json!({})),
            call("torrent_get", json!({"fields": ["status"]})),
            call("session_get", json!({"fields": ["start_added_torrents"]})),
        ]
    );
}

#[tokio::test]
async fn a_client_that_took_both_writes_and_went_on_running_says_so() {
    let (transmission, _) = client(vec![
        answered(DONE),
        answered(DONE),
        holding(RUNNING),
        answered(WAITS),
    ]);
    assert_eq!(transmission.stop().await.ok(), Some(Pulling::Fetching));
}

#[tokio::test]
async fn starting_it_again_lets_new_torrents_start_as_well_as_every_torrent_held() {
    let (transmission, http) = client(vec![
        answered(DONE),
        answered(DONE),
        holding(RUNNING),
        answered(STARTS),
    ]);
    assert_eq!(transmission.resume().await.ok(), Some(Pulling::Fetching));
    assert_eq!(
        called(&http).get(..2),
        Some(
            [
                call("session_set", json!({"start_added_torrents": true})),
                call("torrent_start", json!({})),
            ]
            .as_slice()
        )
    );
}

#[tokio::test]
async fn a_refused_write_stops_there() {
    let (transmission, http) = client(vec![
        answered(DONE),
        lemonfiber_fixtures::http::Answer::reply(500, ""),
    ]);
    assert!(transmission.stop().await.is_err());
    assert_eq!(http.requests().len(), 2);
    let (transmission, http) = client(vec![lemonfiber_fixtures::http::Answer::reply(500, "")]);
    assert!(transmission.resume().await.is_err());
    assert_eq!(http.requests().len(), 1);
}
