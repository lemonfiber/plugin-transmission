use async_trait::async_trait;
use lemonfiber_ports::service::{Failure, Peering};
use serde::Deserialize;
use serde_json::{Map, Value};

use super::Transmission;

const UNFORWARDED: u16 = 0;

const PEER_PORT: &str = "peer_port";

#[derive(Deserialize)]
struct Listening {
    peer_port: u16,
}

impl Transmission {
    async fn listening(&self) -> Result<u16, Failure> {
        let listening: Listening = self
            .session(&[PEER_PORT], "the port it takes peers on could not be read")
            .await?;
        Ok(listening.peer_port)
    }
}

#[async_trait]
impl Peering for Transmission {
    async fn peer_port(&self) -> Result<Option<u16>, Failure> {
        let port = self.listening().await?;
        Ok((port != UNFORWARDED).then_some(port))
    }

    async fn set_peer_port(&self, port: Option<u16>) -> Result<(), Failure> {
        let port = port.unwrap_or(UNFORWARDED);
        self.set(
            Map::from_iter([(PEER_PORT.to_owned(), Value::from(port))]),
            "the port it takes peers on could not be set",
        )
        .await?;
        match self.listening().await? {
            listening if listening == port => Ok(()),
            listening => Err(self.endpoint.refused(&format!(
                "the port was set to {port} and the client is listening on {listening}"
            ))),
        }
    }
}

#[cfg(test)]
mod tests;
