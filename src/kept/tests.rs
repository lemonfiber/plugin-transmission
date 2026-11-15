use std::collections::BTreeSet;
use std::path::PathBuf;
use std::sync::Mutex;

use super::{Keeping, Kept, KEPT_FILE};

#[derive(Default)]
pub(crate) struct Remembered(Mutex<BTreeSet<String>>);

impl Remembered {
    pub(crate) fn holding(categories: &[&str]) -> Self {
        Self(Mutex::new(
            categories.iter().map(|one| (*one).to_owned()).collect(),
        ))
    }

    pub(crate) fn now(&self) -> BTreeSet<String> {
        self.0.lock().map(|held| held.clone()).unwrap_or_default()
    }
}

impl Keeping for Remembered {
    fn kept(&self) -> Result<BTreeSet<String>, String> {
        Ok(self.now())
    }

    fn keep(&self, categories: &BTreeSet<String>) -> Result<(), String> {
        if let Ok(mut held) = self.0.lock() {
            held.clone_from(categories);
        }
        Ok(())
    }
}

pub(crate) struct Failing {
    pub(crate) reads: bool,
}

impl Keeping for Failing {
    fn kept(&self) -> Result<BTreeSet<String>, String> {
        if self.reads {
            Ok(BTreeSet::new())
        } else {
            Err("categories.json is not a list of category names".to_owned())
        }
    }

    fn keep(&self, _categories: &BTreeSet<String>) -> Result<(), String> {
        Err("categories.json could not be written: read-only file system".to_owned())
    }
}

fn directory(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "transmission-adapter-kept-{name}-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    let _ = std::fs::create_dir_all(&dir);
    dir
}

fn named(names: &[&str]) -> BTreeSet<String> {
    names.iter().map(|one| (*one).to_owned()).collect()
}

#[test]
fn nothing_kept_yet_is_no_category() {
    assert_eq!(
        Kept::in_directory(&directory("fresh")).kept(),
        Ok(BTreeSet::new())
    );
}

#[test]
fn what_is_kept_is_read_back_whole() {
    let dir = directory("kept");
    let kept = Kept::in_directory(&dir);
    assert_eq!(kept.keep(&named(&["tv", "movies"])), Ok(()));
    assert_eq!(kept.kept(), Ok(named(&["movies", "tv"])));
    assert_eq!(
        std::fs::read_to_string(dir.join(KEPT_FILE)).ok().as_deref(),
        Some(r#"["movies","tv"]"#)
    );
    assert_eq!(kept.keep(&named(&["music"])), Ok(()));
    assert_eq!(kept.kept(), Ok(named(&["music"])));
    assert!(!dir.join("categories.json.part").exists());
}

#[test]
fn a_file_that_is_not_a_list_of_names_is_refused_by_name() {
    let dir = directory("misshapen");
    for written in ["tv,movies", r#"{"tv":"/data"}"#, "[1]"] {
        let _ = std::fs::write(dir.join(KEPT_FILE), written);
        assert_eq!(
            Kept::in_directory(&dir).kept(),
            Err("categories.json is not a list of category names".to_owned())
        );
    }
}

#[test]
fn a_file_that_cannot_be_read_or_written_is_refused_by_name() {
    let dir = directory("blocked");
    let _ = std::fs::create_dir_all(dir.join(KEPT_FILE));
    let read = Kept::in_directory(&dir).kept();
    assert!(
        read.as_ref()
            .is_err_and(|why| why.starts_with("categories.json could not be read")),
        "{read:?}"
    );
    let written = Kept::in_directory(&dir.join("absent")).keep(&named(&["tv"]));
    assert!(
        written
            .as_ref()
            .is_err_and(|why| why.starts_with("categories.json could not be written")),
        "{written:?}"
    );
}
