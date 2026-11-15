use lemonfiber_fixtures::http::Answer;
use lemonfiber_ports::service::{Failure, Seeded, Seeding};
use serde_json::{json, Value};

use super::super::tests::{answered, call, called, client, holding, DONE};
use super::FIELDS;

fn held(name: &str, hash: &str, left: u64) -> Value {
    json!({
        "hash_string": hash,
        "name": name,
        "size_when_done": 4000,
        "left_until_done": left,
        "metadata_percent_complete": 1.0,
        "uploaded_ever": 6000,
        "downloaded_ever": 4000
    })
}

fn list(torrents: &[Value]) -> Answer {
    holding(&Value::Array(torrents.to_vec()).to_string())
}

#[tokio::test]
async fn what_it_seeds_is_every_complete_torrent_with_its_size_and_ratio() {
    let (transmission, http) = client(vec![list(&[
        held("Done", "aa", 0),
        held("Arriving", "bb", 10),
    ])]);
    assert_eq!(
        transmission.seeding().await.ok(),
        Some(vec![Seeded {
            name: "Done".to_owned(),
            bytes: 4000,
            ratio: 150,
        }])
    );
    assert_eq!(
        called(&http),
        [call("torrent_get", json!({"fields": FIELDS}))]
    );
}

#[test]
fn a_ratio_over_nothing_downloaded_is_the_most_there_is() {
    assert_eq!(super::hundredths(10, 0), u32::MAX);
    assert_eq!(super::hundredths(u64::MAX, 1), u32::MAX);
    assert_eq!(super::hundredths(1, 3), 33);
}

#[tokio::test]
async fn letting_one_go_removes_it_with_its_files_and_reads_back_that_it_is_gone() {
    let (transmission, http) = client(vec![
        list(&[held("Done", "aa", 0), held("Other", "bb", 0)]),
        answered(DONE),
        list(&[held("Other", "bb", 0)]),
    ]);
    assert!(transmission.stop_seeding("Done").await.is_ok());
    assert_eq!(
        called(&http).get(1),
        Some(&call(
            "torrent_remove",
            json!({"ids": ["aa"], "delete_local_data": true})
        ))
    );
}

#[tokio::test]
async fn a_name_held_by_none_or_by_two_is_refused_and_nothing_is_removed() {
    for (holding, count) in [
        (vec![held("Other", "bb", 0)], 0),
        (vec![held("Done", "aa", 0), held("Done", "cc", 0)], 2),
    ] {
        let (transmission, http) = client(vec![list(&holding)]);
        let detail = format!("the client is holding {count} completed downloads called Done");
        assert!(matches!(
            transmission.stop_seeding("Done").await,
            Err(Failure::Refused { detail: said, .. }) if said == detail
        ));
        assert_eq!(http.requests().len(), 1);
    }
}

#[tokio::test]
async fn one_still_held_after_its_removal_is_refused() {
    let (transmission, _) = client(vec![
        list(&[held("Done", "aa", 0)]),
        answered(DONE),
        list(&[held("Done", "aa", 0)]),
    ]);
    assert!(matches!(
        transmission.stop_seeding("Done").await,
        Err(Failure::Refused { detail, .. }) if detail == "the client is still holding Done"
    ));
    let (refused, _) = client(vec![list(&[held("Done", "aa", 0)]), Answer::reply(500, "")]);
    assert!(refused.stop_seeding("Done").await.is_err());
    let (unread, _) = client(vec![Answer::reply(500, "")]);
    assert!(unread.seeding().await.is_err());
}
