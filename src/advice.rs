use serde::{Deserialize, Serialize};

/// Either the client should make a handshake again, retry to connect
/// or just do nothing. This is part of the [Advice](Advice) struct.
#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
#[serde(rename_all = "lowercase")]
pub enum Reconnect {
    /// The client should retry a `connect` request.
    Retry,
    /// The client should send a handshake request.
    Handshake,
    /// The client should neither reconnect or send a handshake request.
    None,
}

/// Represents an advice returned by the cometd server.
#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub struct Advice {
    pub reconnect: Reconnect,
    pub timeout: Option<u32>,
    pub interval: Option<u32>,
    #[serde(rename = "kebab-case")]
    pub multiple_clients: Option<bool>,
    pub hosts: Option<Vec<String>>,
}
