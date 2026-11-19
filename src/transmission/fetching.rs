use async_trait::async_trait;
use lemonfiber_ports::service::{Failure, Fetching, Pulling};
use serde::Deserialize;
use serde_json::{json, Map, Value};
use tokio::time::{Duration, Instant};

use super::{Transmission, STOPPED};

const START_ADDED: &str = "start_added_torrents";

const STOP: &str = "torrent_stop";

const START: &str = "torrent_start";

const SETTLING: Duration = Duration::from_secs(5);

const LOOKING: Duration = Duration::from_millis(250);

#[derive(Deserialize)]
struct Running {
    status: u8,
}

#[derive(Deserialize)]
struct Adding {
    start_added_torrents: bool,
}

impl Transmission {
    async fn starting_added(&self, starts: bool) -> Result<(), Failure> {
        self.set(
            Map::from_iter([(START_ADDED.to_owned(), Value::Bool(starts))]),
            "how new torrents are added could not be set",
        )
        .await
    }

    async fn every_torrent(&self, method: &str) -> Result<(), Failure> {
        self.act(method, json!({}), "the torrents could not be told")
            .await
    }

    /// What the client reads as once it reads as `told`, or what it last read as once
    /// [`SETTLING`] has passed.
    async fn settled(&self, told: Pulling) -> Result<Pulling, Failure> {
        let until = Instant::now() + SETTLING;
        loop {
            let read = self.pulling().await?;
            if read == told || Instant::now() >= until {
                return Ok(read);
            }
            tokio::time::sleep(LOOKING).await;
        }
    }
}

#[async_trait]
impl Fetching for Transmission {
    async fn pulling(&self) -> Result<Pulling, Failure> {
        let running: Vec<Running> = self
            .torrents(&["status"], "what the client is running could not be read")
            .await?;
        let adding: Adding = self
            .session(
                &[START_ADDED],
                "how new torrents are added could not be read",
            )
            .await?;
        Ok(Pulling::of(
            running.iter().any(|one| one.status != STOPPED),
            adding.start_added_torrents,
        ))
    }

    async fn stop(&self) -> Result<Pulling, Failure> {
        self.starting_added(false).await?;
        self.every_torrent(STOP).await?;
        self.settled(Pulling::Stopped).await
    }

    async fn resume(&self) -> Result<Pulling, Failure> {
        self.starting_added(true).await?;
        self.every_torrent(START).await?;
        self.settled(Pulling::Fetching).await
    }
}

#[cfg(test)]
mod tests;
