//! Transmission, the torrent client, asked over its JSON-RPC interface with the account
//! lemonfiber minted for this adapter.

use std::sync::{Arc, Mutex, PoisonError};

use base64::engine::general_purpose::STANDARD;
use base64::Engine as _;
use lemonfiber_adapter_kit::endpoint::Endpoint;
use lemonfiber_ports::http::{Http, Method, Request, Response};
use lemonfiber_ports::service::Failure;
use serde::de::DeserializeOwned;
use serde::Deserialize;
use serde_json::{json, Map, Value};

use crate::credential::{Credential, Holding};
use crate::kept::Keeping;

mod fetching;
mod filing;
mod peering;
mod seeding;
mod throttling;
mod transfers;

/// What every failure is blamed on.
pub const SERVICE: &str = "transmission";

const RPC: &str = "/transmission/rpc";

const SESSION: &str = "X-Transmission-Session-Id";

const AUTHORIZATION: &str = "Authorization";

const CONTENT_TYPE: &str = "Content-Type";

const JSON: &str = "application/json";

const JSON_RPC: &str = "2.0";

const CALL: u8 = 1;

const STALE_SESSION: u16 = 409;

const UNAUTHORISED: u16 = 401;

const SESSION_GET: &str = "session_get";

const SESSION_SET: &str = "session_set";

const SESSION_STATS: &str = "session_stats";

const TORRENT_GET: &str = "torrent_get";

const STOPPED: u8 = 0;

const WHOLE_METADATA: f64 = 1.0;

#[derive(Deserialize)]
struct Answered<T> {
    result: Option<T>,
    error: Option<Said>,
}

#[derive(Deserialize)]
struct Said {
    message: String,
    #[serde(default)]
    data: Option<Detail>,
}

#[derive(Deserialize)]
struct Detail {
    #[serde(default)]
    error_string: String,
}

#[derive(Deserialize)]
struct Torrents<T> {
    torrents: Vec<T>,
}

/// A client for one Transmission, asked with the credential `held` supplies, keeping the
/// category names it was asked to file in `kept`.
pub struct Transmission {
    endpoint: Endpoint,
    held: Arc<dyn Holding>,
    kept: Arc<dyn Keeping>,
    session: Mutex<Option<String>>,
}

impl Transmission {
    /// The Transmission reached at `base` over `http`, asked with what `held` supplies,
    /// keeping the categories it files in `kept`.
    #[must_use]
    pub fn new(
        http: Arc<dyn Http>,
        base: impl Into<String>,
        held: Arc<dyn Holding>,
        kept: Arc<dyn Keeping>,
    ) -> Self {
        Self {
            endpoint: Endpoint::new(http, base, SERVICE),
            held,
            kept,
            session: Mutex::new(None),
        }
    }

    /// Whether Transmission's RPC interface answers, signed in or not.
    pub async fn answers(&self) -> bool {
        let request = Request {
            method: Method::Post,
            url: self.endpoint.url(RPC),
            headers: Vec::new(),
            body: None,
            pinned: None,
        };
        self.endpoint.send(&request).await.is_ok_and(|response| {
            response.is_success() || [UNAUTHORISED, STALE_SESSION].contains(&response.status)
        })
    }

    fn credential(&self) -> Result<Credential, Failure> {
        self.held.credential().map_err(|why| {
            self.endpoint.refused(&format!(
                "this adapter holds no credential for Transmission: {why}"
            ))
        })
    }

    fn held_session(&self) -> Option<String> {
        self.session
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }

    fn request(&self, body: &str, credential: &Credential) -> Request {
        let basic = STANDARD.encode(format!("{}:{}", credential.username, credential.password));
        let mut headers = vec![
            (CONTENT_TYPE.to_owned(), JSON.to_owned()),
            (AUTHORIZATION.to_owned(), format!("Basic {basic}")),
        ];
        headers.extend(
            self.held_session()
                .map(|session| (SESSION.to_owned(), session)),
        );
        Request {
            method: Method::Post,
            url: self.endpoint.url(RPC),
            headers,
            body: Some(body.to_owned()),
            pinned: None,
        }
    }

    async fn sent(&self, body: &str) -> Result<Response, Failure> {
        let credential = self.credential()?;
        let response = self.endpoint.send(&self.request(body, &credential)).await?;
        if response.status != STALE_SESSION {
            return Ok(response);
        }
        let Some(session) = response.header(SESSION) else {
            return Err(self
                .endpoint
                .refused("Transmission asked for a session and named none"));
        };
        *self.session.lock().unwrap_or_else(PoisonError::into_inner) = Some(session.to_owned());
        self.endpoint.send(&self.request(body, &credential)).await
    }

    async fn call<T: DeserializeOwned>(
        &self,
        method: &str,
        params: Value,
        whenever: &str,
    ) -> Result<T, Failure> {
        let body = json!({
            "jsonrpc": JSON_RPC,
            "method": method,
            "params": params,
            "id": CALL,
        });
        let response = self.sent(&body.to_string()).await?;
        let answered: Answered<T> = self.endpoint.decode(&response, whenever)?;
        if let Some(said) = answered.error {
            let detail = said
                .data
                .map(|data| data.error_string)
                .filter(|detail| !detail.is_empty())
                .map_or_else(String::new, |detail| format!(": {detail}"));
            return Err(self.endpoint.refused(&format!(
                "{whenever}: Transmission said {}{detail}",
                said.message
            )));
        }
        answered.result.ok_or_else(|| {
            self.endpoint
                .refused(&format!("{whenever}: Transmission said nothing"))
        })
    }

    async fn session<T: DeserializeOwned>(
        &self,
        fields: &[&str],
        whenever: &str,
    ) -> Result<T, Failure> {
        self.call(SESSION_GET, json!({ "fields": fields }), whenever)
            .await
    }

    async fn set(&self, settings: Map<String, Value>, whenever: &str) -> Result<(), Failure> {
        self.act(SESSION_SET, Value::Object(settings), whenever)
            .await
    }

    async fn act(&self, method: &str, params: Value, whenever: &str) -> Result<(), Failure> {
        self.call::<Value>(method, params, whenever).await.map(drop)
    }

    async fn torrents<T: DeserializeOwned>(
        &self,
        fields: &[&str],
        whenever: &str,
    ) -> Result<Vec<T>, Failure> {
        let held: Torrents<T> = self
            .call(TORRENT_GET, json!({ "fields": fields }), whenever)
            .await?;
        Ok(held.torrents)
    }
}

fn complete(left_until_done: u64, metadata: f64) -> bool {
    left_until_done == 0 && metadata >= WHOLE_METADATA
}

#[cfg(test)]
mod tests;
