use serde::Deserialize;

use crate::advice::Advice;
use crate::config::{COMETD_SUPPORTED_TYPES, COMETD_VERSION};
use crate::request::Request;

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
pub struct ErroredResponse {
    pub channel: String,
    pub successful: bool,
    pub error: String,
    pub client_id: Option<String>,
    pub subscription: Option<String>,
    pub advice: Option<Advice>,
    pub ext: Option<serde_json::Value>,
    pub id: Option<String>,
}

#[derive(Deserialize, PartialEq, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct PublishResponse {
    pub client_id: String,
    pub successful: bool,
    pub error: Option<String>,
    pub advice: Option<Advice>,
    pub data: serde_json::Value,
}

#[derive(Deserialize, PartialEq, Debug)]
#[serde(untagged)]
pub enum Response {
    Handshake(HandshakeResponse),
    Publish(PublishResponse),
    Connect(ConnectResponse),
}

impl Into<Request> for &ErroredResponse {
    fn into(self) -> Request {
        Request {
            channel: self.channel.clone(),
            version: COMETD_VERSION.to_owned(),
            minimum_version: None,
            supported_connection_types: COMETD_SUPPORTED_TYPES
                .to_vec()
                .into_iter()
                .map(|ct| ct.to_owned())
                .collect(),
            ext: self.ext.clone(),
            id: self.id.clone(),
            subscription: self.subscription.clone(),
            client_id: self.client_id.clone(),
        }
    }
}
