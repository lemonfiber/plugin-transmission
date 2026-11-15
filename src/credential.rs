//! The credential this adapter signs in to Transmission with, as lemonfiber writes it into the
//! adapter's configuration directory.

use std::io::ErrorKind;
use std::path::{Path, PathBuf};

use lemonfiber_contract::adapter::{UPSTREAM_FILE, UPSTREAM_PASSWORD, UPSTREAM_USERNAME};

/// An account on Transmission and its RPC password.
#[derive(Clone, PartialEq, Eq)]
pub struct Credential {
    /// The account's name.
    pub username: String,
    /// Its password.
    pub password: String,
}

impl std::fmt::Debug for Credential {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("Credential")
            .field("username", &self.username)
            .finish_non_exhaustive()
    }
}

/// Where the credential a sign-in uses comes from.
pub trait Holding: Send + Sync {
    /// The credential held now.
    ///
    /// # Errors
    ///
    /// Why no credential is held, in words that name no part of one.
    fn credential(&self) -> Result<Credential, String>;
}

impl Holding for Credential {
    fn credential(&self) -> Result<Credential, String> {
        Ok(self.clone())
    }
}

/// The credential lemonfiber wrote into one configuration directory.
#[derive(Debug, Clone)]
pub struct Upstream(PathBuf);

impl Upstream {
    /// The credential lemonfiber writes into `configuration`.
    #[must_use]
    pub fn in_directory(configuration: &Path) -> Self {
        Self(configuration.to_path_buf())
    }
}

impl Holding for Upstream {
    fn credential(&self) -> Result<Credential, String> {
        let mut parts = lemonfiber_adapter_kit::upstream_in(&self.0).map_err(|why| {
            if why.kind() == ErrorKind::InvalidData {
                format!("{UPSTREAM_FILE} is not an object of named strings")
            } else {
                format!("{UPSTREAM_FILE} could not be read: {why}")
            }
        })?;
        let mut part = |name: &str| {
            parts
                .remove(name)
                .filter(|value| !value.is_empty())
                .ok_or_else(|| format!("{UPSTREAM_FILE} names no {name}"))
        };
        Ok(Credential {
            username: part(UPSTREAM_USERNAME)?,
            password: part(UPSTREAM_PASSWORD)?,
        })
    }
}

#[cfg(test)]
mod tests;
