use async_trait::async_trait;
use lemonfiber_ports::service::{Failure, Seeded, Seeding};
use serde::Deserialize;
use serde_json::json;

use super::{complete, Transmission};

const FIELDS: [&str; 7] = [
    "hash_string",
    "name",
    "size_when_done",
    "left_until_done",
    "metadata_percent_complete",
    "uploaded_ever",
    "downloaded_ever",
];

const REMOVE: &str = "torrent_remove";

const HUNDRED: u64 = 100;

#[derive(Deserialize)]
struct Held {
    hash_string: String,
    name: String,
    size_when_done: u64,
    left_until_done: u64,
    metadata_percent_complete: f64,
    uploaded_ever: u64,
    downloaded_ever: u64,
}

impl Transmission {
    async fn completed(&self) -> Result<Vec<Held>, Failure> {
        let held: Vec<Held> = self
            .torrents(&FIELDS, "the completed torrent list could not be read")
            .await?;
        Ok(held
            .into_iter()
            .filter(|one| complete(one.left_until_done, one.metadata_percent_complete))
            .collect())
    }
}

#[async_trait]
impl Seeding for Transmission {
    async fn seeding(&self) -> Result<Vec<Seeded>, Failure> {
        Ok(self.completed().await?.into_iter().map(seeded_of).collect())
    }

    async fn stop_seeding(&self, name: &str) -> Result<(), Failure> {
        let holding = self.completed().await?;
        let named: Vec<&Held> = holding.iter().filter(|one| one.name == name).collect();
        let [one] = named.as_slice() else {
            return Err(self.endpoint.refused(&format!(
                "the client is holding {} completed downloads called {name}",
                named.len()
            )));
        };
        self.act(
            REMOVE,
            json!({ "ids": [one.hash_string], "delete_local_data": true }),
            "the torrent could not be removed",
        )
        .await?;
        if self.completed().await?.iter().any(|one| one.name == name) {
            return Err(self
                .endpoint
                .refused(&format!("the client is still holding {name}")));
        }
        Ok(())
    }
}

fn seeded_of(torrent: Held) -> Seeded {
    Seeded {
        name: torrent.name,
        bytes: torrent.size_when_done,
        ratio: hundredths(torrent.uploaded_ever, torrent.downloaded_ever),
    }
}

fn hundredths(uploaded: u64, downloaded: u64) -> u32 {
    uploaded
        .saturating_mul(HUNDRED)
        .checked_div(downloaded)
        .map_or(u32::MAX, |scaled| u32::try_from(scaled).unwrap_or(u32::MAX))
}

#[cfg(test)]
mod tests;
