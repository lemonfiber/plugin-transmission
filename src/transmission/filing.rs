use std::collections::BTreeSet;

use async_trait::async_trait;
use lemonfiber_ports::service::{Failure, Filed, Filing};
use serde::Deserialize;
use serde_json::{Map, Value};

use super::Transmission;

const DOWNLOAD_DIR: &str = "download_dir";

#[derive(Deserialize)]
struct Saving {
    download_dir: String,
}

#[derive(Deserialize)]
struct Labelled {
    #[serde(default)]
    labels: Vec<String>,
}

impl Transmission {
    async fn download_folder(&self) -> Result<String, Failure> {
        let saving: Saving = self
            .session(
                &[DOWNLOAD_DIR],
                "where torrents are saved could not be read",
            )
            .await?;
        Ok(saving.download_dir.trim_end_matches('/').to_owned())
    }

    fn categories_kept(&self) -> Result<BTreeSet<String>, Failure> {
        self.kept.kept().map_err(|why| {
            self.endpoint.refused(&format!(
                "the categories this adapter keeps could not be read: {why}"
            ))
        })
    }

    fn one_parent(&self, folders: &[Filed]) -> Result<Option<String>, Failure> {
        let mut parents = BTreeSet::new();
        for one in folders {
            if !held_to_shape(one) {
                return Err(self.endpoint.refused(&format!(
                    "{:?} under {:?} is not a category and folder Transmission could hold",
                    one.category, one.folder
                )));
            }
            let Some(parent) = parent_of(one) else {
                return Err(self.endpoint.refused(&format!(
                    "Transmission files each category in a folder of its own name under its \
                     download folder, and {} is not a folder named {} under one",
                    one.folder, one.category
                )));
            };
            parents.insert(parent);
        }
        let mut each = parents.into_iter();
        match (each.next(), each.next()) {
            (Some(_), Some(_)) => Err(self.endpoint.refused(
                "Transmission files every category under one download folder, and these \
                 folders lie under more than one",
            )),
            (parent, _) => Ok(parent),
        }
    }
}

#[async_trait]
impl Filing for Transmission {
    async fn filed(&self) -> Result<Vec<Filed>, Failure> {
        let mut categories = self.categories_kept()?;
        let labelled: Vec<Labelled> = self
            .torrents(&["labels"], "the torrents' labels could not be read")
            .await?;
        categories.extend(
            labelled
                .into_iter()
                .flat_map(|one| one.labels)
                .filter(|label| !label.is_empty()),
        );
        let folder = self.download_folder().await?;
        Ok(categories
            .into_iter()
            .map(|category| Filed {
                folder: format!("{folder}/{category}"),
                category,
            })
            .collect())
    }

    async fn file_under(&self, folders: &[Filed]) -> Result<(), Failure> {
        let Some(parent) = self.one_parent(folders)? else {
            return Ok(());
        };
        let mut categories = self.categories_kept()?;
        self.set(
            Map::from_iter([(DOWNLOAD_DIR.to_owned(), Value::from(parent.as_str()))]),
            "the download folder could not be set",
        )
        .await?;
        let held = self.download_folder().await?;
        if held != parent {
            return Err(self.endpoint.refused(&format!(
                "Transmission was told to download into {parent} and downloads into {held}"
            )));
        }
        categories.extend(folders.iter().map(|one| one.category.clone()));
        self.kept.keep(&categories).map_err(|why| {
            self.endpoint
                .refused(&format!("the categories filed could not be kept: {why}"))
        })
    }
}

fn parent_of(one: &Filed) -> Option<String> {
    one.folder
        .trim_end_matches('/')
        .strip_suffix(&format!("/{}", one.category))
        .filter(|parent| parent.starts_with('/'))
        .map(str::to_owned)
}

fn held_to_shape(one: &Filed) -> bool {
    let category = one.category.as_str();
    let named = !matches!(category, "" | "." | "..")
        && category
            .chars()
            .all(|letter| letter.is_alphanumeric() || matches!(letter, '-' | '_' | '.' | ' '));
    let placed = one.folder.starts_with('/')
        && !one.folder.chars().any(char::is_control)
        && !one.folder.split('/').any(|segment| segment == "..");
    named && placed
}

#[cfg(test)]
mod tests;
