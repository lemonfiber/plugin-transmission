use std::time::Duration;

use lemonfiber_ports::service::{Download, TransferState, Transfers};
use serde_json::{json, Value};

use super::super::tests::{called, client, holding};
use super::FIELDS;

fn torrent(changed: &Value) -> Value {
    let mut one = json!({
        "name": "Show.S01E01",
        "hash_string": "abc123",
        "labels": ["tv"],
        "status": 4,
        "error": 0,
        "size_when_done": 1000,
        "left_until_done": 250,
        "rate_download": 2048,
        "eta": 90,
        "added_date": 1_700_000_000,
        "metadata_percent_complete": 1.0,
        "peers_sending_to_us": 3,
        "webseeds_sending_to_us": 0
    });
    if let (Some(held), Some(changes)) = (one.as_object_mut(), changed.as_object()) {
        held.extend(changes.clone());
    }
    one
}

async fn read(torrents: Vec<Value>) -> Vec<Download> {
    let (transmission, _) = client(vec![holding(&Value::Array(torrents).to_string())]);
    transmission.transfers().await.unwrap_or_default()
}

#[tokio::test]
async fn each_torrent_names_its_hash_category_state_and_when_it_was_added() {
    let (transmission, http) = client(vec![holding(&json!([torrent(&json!({}))]).to_string())]);
    assert_eq!(
        transmission.transfers().await.ok(),
        Some(vec![Download {
            name: "Show.S01E01".to_owned(),
            progress: 75,
            speed: Some(2048),
            eta: Some(Duration::from_secs(90)),
            remaining: Some(250),
            id: Some("abc123".to_owned()),
            category: Some("tv".to_owned()),
            state: Some(TransferState::Downloading),
            added: Some(1_700_000_000),
        }])
    );
    assert_eq!(
        called(&http)
            .first()
            .and_then(|body| body.pointer("/params/fields").cloned()),
        Some(json!(FIELDS))
    );
}

#[tokio::test]
async fn only_what_is_still_arriving_is_a_transfer() {
    let read = read(vec![
        torrent(&json!({"left_until_done": 0})),
        torrent(
            &json!({"left_until_done": 0, "size_when_done": 0, "metadata_percent_complete": 0.5}),
        ),
    ])
    .await;
    assert_eq!(
        read.iter()
            .map(|one| (one.progress, one.state))
            .collect::<Vec<_>>(),
        [(0, Some(TransferState::Metadata))]
    );
}

#[tokio::test]
async fn transmission_s_statuses_are_said_as_the_contract_names_them() {
    for (changed, state) in [
        (json!({"status": 0}), Some(TransferState::Paused)),
        (json!({"status": 3}), Some(TransferState::Queued)),
        (json!({"status": 2}), None),
        (
            json!({"peers_sending_to_us": 0}),
            Some(TransferState::Stalled),
        ),
        (
            json!({"peers_sending_to_us": 0, "webseeds_sending_to_us": 1}),
            Some(TransferState::Downloading),
        ),
        (json!({"error": 3, "status": 0}), Some(TransferState::Error)),
        (json!({"error": 2}), Some(TransferState::Downloading)),
    ] {
        let read = read(vec![torrent(&changed)]).await;
        assert_eq!(
            read.first().map(|transfer| transfer.state),
            Some(state),
            "{changed}"
        );
    }
}

#[tokio::test]
async fn a_torrent_with_no_hash_label_estimate_or_date_says_none_of_them() {
    let bare = json!({
        "name": "Bare",
        "status": 4,
        "size_when_done": 0,
        "left_until_done": 0,
        "rate_download": 0,
        "eta": -1,
        "metadata_percent_complete": 0.0
    });
    let empty =
        torrent(&json!({"labels": ["", ""], "hash_string": "", "added_date": -5, "eta": -2}));
    for one in [bare, empty] {
        let read = read(vec![one]).await;
        assert_eq!(
            read.first()
                .map(|one| (one.id.clone(), one.category.clone(), one.eta, one.added)),
            Some((None, None, None, None))
        );
    }
}

#[test]
fn progress_is_a_whole_share_of_what_was_wanted_and_never_passes_a_hundred() {
    assert_eq!(super::percent(0, 0), 0);
    assert_eq!(super::percent(1, 3), 33);
    assert_eq!(super::percent(5, 1), 100);
    assert_eq!(super::percent(u64::MAX, u64::MAX), 100);
}
