use serde::Deserialize;

use crate::advice::Advice;

#[derive(Deserialize, PartialEq, Debug)]
#[serde(rename_all = "camelCase")]
pub struct BasicResponse {
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
    pub channel: String,
    pub client_id: String,
    pub successful: bool,
    pub error: Option<String>,
    pub advice: Option<Advice>,
    pub ext: Option<serde_json::Value>,
    pub data: serde_json::Value,
    pub id: Option<String>,
}

#[derive(Deserialize, PartialEq, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct DeliveryResponse {
    pub channel: String,
    pub advice: Option<Advice>,
    pub data: serde_json::Value,
    pub ext: Option<serde_json::Value>,
    pub id: Option<String>,
}

#[derive(Deserialize, PartialEq, Debug)]
#[serde(untagged)]
pub enum Response {
    Handshake(HandshakeResponse),
    Publish(PublishResponse),
    Delivery(DeliveryResponse),
    Basic(BasicResponse),
}

impl Response {
    pub fn advice(&self) -> Option<Advice> {
        match self {
            Response::Handshake(resp) => resp.advice.clone(),
            Response::Publish(resp) => resp.advice.clone(),
            Response::Delivery(resp) => resp.advice.clone(),
            Response::Basic(resp) => resp.advice.clone(),
        }
    }
}
