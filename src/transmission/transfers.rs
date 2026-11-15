use std::time::Duration;

use async_trait::async_trait;
use lemonfiber_ports::service::{Download, Failure, TransferState, Transfers};
use serde::Deserialize;

use super::{complete, Transmission, STOPPED, WHOLE_METADATA};

const FIELDS: [&str; 13] = [
    "hash_string",
    "name",
    "labels",
    "status",
    "error",
    "size_when_done",
    "left_until_done",
    "rate_download",
    "eta",
    "added_date",
    "metadata_percent_complete",
    "peers_sending_to_us",
    "webseeds_sending_to_us",
];

const QUEUED: u8 = 3;

const DOWNLOADING: u8 = 4;

const LOCAL_ERROR: u8 = 3;

const WHOLE: u128 = 100;

#[derive(Deserialize)]
struct Torrent {
    name: String,
    #[serde(default)]
    hash_string: String,
    #[serde(default)]
    labels: Vec<String>,
    status: u8,
    #[serde(default)]
    error: u8,
    size_when_done: u64,
    left_until_done: u64,
    rate_download: u64,
    eta: i64,
    #[serde(default)]
    added_date: i64,
    metadata_percent_complete: f64,
    #[serde(default)]
    peers_sending_to_us: u64,
    #[serde(default)]
    webseeds_sending_to_us: u64,
}

#[async_trait]
impl Transfers for Transmission {
    async fn transfers(&self) -> Result<Vec<Download>, Failure> {
        let held: Vec<Torrent> = self
            .torrents(&FIELDS, "the torrent list could not be read")
            .await?;
        Ok(held
            .into_iter()
            .filter(|one| !complete(one.left_until_done, one.metadata_percent_complete))
            .map(download_of)
            .collect())
    }
}

fn download_of(torrent: Torrent) -> Download {
    let state = state_of(&torrent);
    Download {
        progress: percent(
            torrent
                .size_when_done
                .saturating_sub(torrent.left_until_done),
            torrent.size_when_done,
        ),
        speed: Some(torrent.rate_download),
        eta: u64::try_from(torrent.eta).ok().map(Duration::from_secs),
        remaining: Some(torrent.left_until_done),
        id: Some(torrent.hash_string).filter(|hash| !hash.is_empty()),
        category: torrent.labels.into_iter().find(|label| !label.is_empty()),
        state,
        added: u64::try_from(torrent.added_date)
            .ok()
            .filter(|added| *added > 0),
        name: torrent.name,
    }
}

fn state_of(torrent: &Torrent) -> Option<TransferState> {
    if torrent.error == LOCAL_ERROR {
        return Some(TransferState::Error);
    }
    match torrent.status {
        STOPPED => Some(TransferState::Paused),
        QUEUED => Some(TransferState::Queued),
        DOWNLOADING if torrent.metadata_percent_complete < WHOLE_METADATA => {
            Some(TransferState::Metadata)
        }
        DOWNLOADING if torrent.peers_sending_to_us == 0 && torrent.webseeds_sending_to_us == 0 => {
            Some(TransferState::Stalled)
        }
        DOWNLOADING => Some(TransferState::Downloading),
        _ => None,
    }
}

fn percent(done: u64, total: u64) -> u8 {
    let whole = u128::from(done)
        .saturating_mul(WHOLE)
        .checked_div(u128::from(total))
        .map_or(0, |ratio| ratio.min(WHOLE));
    u8::try_from(whole).unwrap_or(u8::MAX)
}

#[cfg(test)]
mod tests;
