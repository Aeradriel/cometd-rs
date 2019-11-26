use crate::advice::Advice;
use serde::Deserialize;

#[derive(Deserialize, PartialEq, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ConnectResponse {
    pub channel: String,
    pub successful: bool,
    pub error: Option<String>,
    pub advice: Option<Advice>,
    pub ext: Option<serde_json::Value>,
    pub client_id: Option<String>,
    pub id: Option<String>,
}

#[derive(Deserialize, PartialEq, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct HandshakeResponse {
    pub channel: String,
    pub successful: bool,
    pub error: Option<String>,
    pub version: String,
    pub minimum_version: Option<String>,
    pub client_id: String,
    pub supported_connection_types: Vec<String>,
    pub advice: Option<Advice>,
    pub ext: Option<serde_json::Value>,
    pub id: Option<String>,
    pub auth_successful: Option<bool>,
}

#[derive(Deserialize, PartialEq, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct PublishResponse {
    pub client_id: String,
    pub data: serde_json::Value,
}

#[derive(Deserialize, PartialEq, Debug)]
#[serde(untagged)]
pub enum Response {
    Handshake(HandshakeResponse),
    Publish(PublishResponse),
    Connect(ConnectResponse),
}
