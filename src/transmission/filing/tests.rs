use std::collections::BTreeSet;
use std::sync::Arc;

use lemonfiber_fixtures::http::Answer;
use lemonfiber_ports::service::{Failure, Filed, Filing};
use serde_json::json;

use super::super::tests::{answered, call, called, client, holding, keeping, BASE, DONE};
use super::super::Transmission;
use crate::credential::Credential;
use crate::kept::tests::{Failing, Remembered};

const SAVING: &str = r#"{"download_dir":"/data/downloads/complete/"}"#;

fn filed(category: &str, folder: &str) -> Filed {
    Filed {
        category: category.to_owned(),
        folder: folder.to_owned(),
    }
}

fn named(names: &[&str]) -> BTreeSet<String> {
    names.iter().map(|one| (*one).to_owned()).collect()
}

fn failing(answers: Vec<Answer>, reads: bool) -> Transmission {
    Transmission::new(
        lemonfiber_fixtures::http::Fake::in_turn(answers),
        BASE,
        Arc::new(Credential {
            username: "admin".to_owned(),
            password: "secret".to_owned(),
        }),
        Arc::new(Failing { reads }),
    )
}

#[tokio::test]
async fn a_fresh_client_files_nothing() {
    let (transmission, http) = client(vec![holding("[]"), answered(SAVING)]);
    assert_eq!(transmission.filed().await.ok(), Some(Vec::new()));
    assert_eq!(
        called(&http),
        [
            call("torrent_get", json!({"fields": ["labels"]})),
            call("session_get", json!({"fields": ["download_dir"]})),
        ]
    );
}

#[tokio::test]
async fn every_category_kept_or_labelled_is_filed_in_its_own_folder_under_the_download_folder() {
    let kept = Arc::new(Remembered::holding(&["tv"]));
    let (transmission, _) = keeping(
        vec![
            holding(r#"[{"labels":["movies","tv"]},{"labels":[""]},{}]"#),
            answered(SAVING),
        ],
        kept,
    );
    assert_eq!(
        transmission.filed().await.ok(),
        Some(vec![
            filed("movies", "/data/downloads/complete/movies"),
            filed("tv", "/data/downloads/complete/tv"),
        ])
    );
}

#[tokio::test]
async fn filing_sets_the_download_folder_keeps_the_names_and_a_fresh_client_reads_them_back() {
    let kept = Arc::new(Remembered::default());
    let (transmission, http) = keeping(
        vec![
            answered(DONE),
            answered(SAVING),
            holding("[]"),
            answered(SAVING),
        ],
        Arc::clone(&kept),
    );
    let asked = [
        filed("tv", "/data/downloads/complete/tv"),
        filed("movies", "/data/downloads/complete/movies/"),
    ];
    assert!(transmission.file_under(&asked).await.is_ok());
    assert_eq!(kept.now(), named(&["movies", "tv"]));
    assert_eq!(
        called(&http).first(),
        Some(&call(
            "session_set",
            json!({"download_dir": "/data/downloads/complete"})
        ))
    );
    assert_eq!(
        transmission.filed().await.ok(),
        Some(vec![
            filed("movies", "/data/downloads/complete/movies"),
            filed("tv", "/data/downloads/complete/tv"),
        ])
    );
}

#[tokio::test]
async fn nothing_to_file_asks_nothing() {
    let (transmission, http) = client(vec![]);
    assert!(transmission.file_under(&[]).await.is_ok());
    assert!(http.requests().is_empty());
}

#[tokio::test]
async fn a_folder_transmission_could_not_hold_is_refused_before_anything_is_asked() {
    for (asked, said) in [
        (
            vec![filed("", "/data/x")],
            r#""" under "/data/x" is not a category and folder Transmission could hold"#,
        ),
        (
            vec![filed("..", "/data/..")],
            r#"".." under "/data/.." is not a category and folder Transmission could hold"#,
        ),
        (
            vec![filed("tv/x", "/data/tv/x")],
            r#""tv/x" under "/data/tv/x" is not a category and folder Transmission could hold"#,
        ),
        (
            vec![filed("tv", "data/tv")],
            r#""tv" under "data/tv" is not a category and folder Transmission could hold"#,
        ),
        (
            vec![filed("tv", "/data/../tv")],
            r#""tv" under "/data/../tv" is not a category and folder Transmission could hold"#,
        ),
        (
            vec![filed("tv", "/data/\ttv")],
            r#""tv" under "/data/\ttv" is not a category and folder Transmission could hold"#,
        ),
        (
            vec![filed("tv", "/data/series")],
            "Transmission files each category in a folder of its own name under its download \
             folder, and /data/series is not a folder named tv under one",
        ),
        (
            vec![filed("tv", "/tv")],
            "Transmission files each category in a folder of its own name under its download \
             folder, and /tv is not a folder named tv under one",
        ),
        (
            vec![filed("tv", "/data/a/tv"), filed("movies", "/data/b/movies")],
            "Transmission files every category under one download folder, and these folders \
             lie under more than one",
        ),
    ] {
        let (transmission, http) = client(vec![]);
        let refused = transmission.file_under(&asked).await;
        assert!(
            matches!(&refused, Err(Failure::Refused { detail, .. }) if detail == said),
            "{refused:?}"
        );
        assert!(http.requests().is_empty());
    }
}

#[tokio::test]
async fn a_download_folder_that_did_not_take_is_refused_and_nothing_is_kept() {
    let kept = Arc::new(Remembered::default());
    let (transmission, _) = keeping(
        vec![
            answered(DONE),
            answered(r#"{"download_dir":"/downloads/complete"}"#),
        ],
        Arc::clone(&kept),
    );
    let refused = transmission
        .file_under(&[filed("tv", "/data/downloads/complete/tv")])
        .await;
    assert!(matches!(
        refused,
        Err(Failure::Refused { detail, .. })
            if detail == "Transmission was told to download into /data/downloads/complete and downloads into /downloads/complete"
    ));
    assert!(kept.now().is_empty());
    let (written, _) = client(vec![Answer::reply(500, "")]);
    assert!(written
        .file_under(&[filed("tv", "/data/downloads/complete/tv")])
        .await
        .is_err());
}

#[tokio::test]
async fn categories_that_cannot_be_read_or_kept_are_refused() {
    let unread = failing(vec![], false);
    assert!(matches!(
        unread.filed().await,
        Err(Failure::Refused { detail, .. })
            if detail == "the categories this adapter keeps could not be read: categories.json is not a list of category names"
    ));
    assert!(unread
        .file_under(&[filed("tv", "/data/downloads/complete/tv")])
        .await
        .is_err());
    let unkept = failing(vec![answered(DONE), answered(SAVING)], true);
    assert!(matches!(
        unkept.file_under(&[filed("tv", "/data/downloads/complete/tv")]).await,
        Err(Failure::Refused { detail, .. })
            if detail == "the categories filed could not be kept: categories.json could not be written: read-only file system"
    ));
}
