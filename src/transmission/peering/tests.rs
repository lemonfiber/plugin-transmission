use lemonfiber_fixtures::http::Answer;
use lemonfiber_ports::service::{Failure, Peering};
use serde_json::json;

use super::super::tests::{answered, call, called, client, DONE};

#[tokio::test]
async fn the_port_it_takes_peers_on_is_its_peer_port_and_none_is_zero() {
    let (transmission, http) = client(vec![answered(r#"{"peer_port":51413}"#)]);
    assert_eq!(transmission.peer_port().await.ok(), Some(Some(51413)));
    assert_eq!(
        called(&http),
        [call("session_get", json!({"fields": ["peer_port"]}))]
    );
    let (transmission, _) = client(vec![answered(r#"{"peer_port":0}"#)]);
    assert_eq!(transmission.peer_port().await.ok(), Some(None));
    let (transmission, _) = client(vec![answered(r#"{"peer_port":"high"}"#)]);
    assert!(transmission.peer_port().await.is_err());
}

#[tokio::test]
async fn a_port_is_set_and_read_back_and_none_sets_zero() {
    for (port, written, read) in [
        (Some(51413), 51413, r#"{"peer_port":51413}"#),
        (None, 0, r#"{"peer_port":0}"#),
    ] {
        let (transmission, http) = client(vec![answered(DONE), answered(read)]);
        assert!(transmission.set_peer_port(port).await.is_ok());
        assert_eq!(
            called(&http).first(),
            Some(&call("session_set", json!({"peer_port": written})))
        );
    }
}

#[tokio::test]
async fn a_client_still_on_another_port_after_the_write_is_a_failure() {
    let (transmission, _) = client(vec![answered(DONE), answered(r#"{"peer_port":4464}"#)]);
    assert!(matches!(
        transmission.set_peer_port(Some(70)).await,
        Err(Failure::Refused { detail, .. })
            if detail == "the port was set to 70 and the client is listening on 4464"
    ));
    let (refused, _) = client(vec![Answer::reply(500, "")]);
    assert!(refused.set_peer_port(Some(51413)).await.is_err());
}
