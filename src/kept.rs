//! The categories this adapter was asked to file, kept in its own configuration directory.
//!
//! Transmission holds the folder every category is filed under, its download folder, and
//! no list of categories. So the names asked for are kept here, beside the files lemonfiber
//! writes, and never in them.

use std::collections::BTreeSet;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

/// The file the categories are kept in.
pub const KEPT_FILE: &str = "categories.json";

/// What a category list is written to first, and renamed from once it is whole.
const WRITING: &str = "categories.json.part";

/// Where the categories this adapter was asked to file are kept.
pub trait Keeping: Send + Sync {
    /// Every category kept, none where nothing was kept yet.
    ///
    /// # Errors
    ///
    /// Why what was kept could not be read.
    fn kept(&self) -> Result<BTreeSet<String>, String>;

    /// Keep `categories` in place of what was kept.
    ///
    /// # Errors
    ///
    /// Why they could not be kept.
    fn keep(&self, categories: &BTreeSet<String>) -> Result<(), String>;
}

/// The categories kept in one configuration directory.
#[derive(Debug, Clone)]
pub struct Kept(PathBuf);

impl Kept {
    /// The categories kept in `configuration`.
    #[must_use]
    pub fn in_directory(configuration: &Path) -> Self {
        Self(configuration.to_path_buf())
    }
}

impl Keeping for Kept {
    fn kept(&self) -> Result<BTreeSet<String>, String> {
        match std::fs::read_to_string(self.0.join(KEPT_FILE)) {
            Ok(text) => serde_json::from_str(&text)
                .map_err(|_| format!("{KEPT_FILE} is not a list of category names")),
            Err(why) if why.kind() == ErrorKind::NotFound => Ok(BTreeSet::new()),
            Err(why) => Err(format!("{KEPT_FILE} could not be read: {why}")),
        }
    }

    fn keep(&self, categories: &BTreeSet<String>) -> Result<(), String> {
        let text = categories
            .iter()
            .cloned()
            .collect::<serde_json::Value>()
            .to_string();
        let writing = self.0.join(WRITING);
        std::fs::write(&writing, text)
            .and_then(|()| std::fs::rename(&writing, self.0.join(KEPT_FILE)))
            .map_err(|why| format!("{KEPT_FILE} could not be written: {why}"))
    }
}

#[cfg(test)]
pub(crate) mod tests;
