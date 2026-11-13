//! What this adapter says it is, and the kit that serves its contract.

use std::sync::Arc;

use lemonfiber_adapter_kit::{Kit, Readiness};
use lemonfiber_contract::adapter::{About, Release};
use lemonfiber_contract::capabilities::download;

use crate::transmission::Transmission;

/// Where Transmission answers, by its service id and the port it listens on.
pub const ADDRESS: &str = "http://transmission:9091";

/// The upstream this adapter fronts, as its project names itself.
pub const UPSTREAM: &str = "Transmission";

/// The Transmission release this adapter's recordings were taken from.
pub const RELEASE: &str = "4.1.4-r0-ls369";

/// The digest of the image that release's recordings were taken from.
pub const DIGEST: &str = "sha256:aaf349cd859ca9d5f34f394c56b8097c267a513283698adc4d07ee61d7cb1a85";

/// What this adapter says it is: the contract it speaks and the release it supports.
#[must_use]
pub fn about() -> About {
    About {
        speaks: vec![lemonfiber_contract::spoken(
            download::torrent::CAPABILITY,
            download::torrent::MAJOR,
        )],
        upstream: UPSTREAM.to_owned(),
        releases: vec![Release {
            version: RELEASE.to_owned(),
            digest: DIGEST.to_owned(),
        }],
    }
}

/// The contract, served from `transmission` under `key`, ready while Transmission answers.
#[must_use]
pub fn kit(key: String, transmission: Arc<Transmission>) -> Kit {
    let asked = Arc::clone(&transmission);
    let ready: Readiness = Arc::new(move || {
        let asked = Arc::clone(&asked);
        Box::pin(async move { asked.answers().await })
    });
    Kit::new(key, about())
        .ready_when(ready)
        .serving(download::torrent::served(transmission))
}

#[cfg(test)]
mod tests;
