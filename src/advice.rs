use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
#[serde(rename_all = "lowercase")]
pub enum Reconnect {
    Retry,
    Handshake,
    None,
}

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub struct Advice {
    pub reconnect: Reconnect,
    pub timeout: Option<u32>,
    pub interval: Option<u32>,
    #[serde(rename = "kebab-case")]
    pub multiple_clients: Option<bool>,
    pub hosts: Option<Vec<String>>,
}
