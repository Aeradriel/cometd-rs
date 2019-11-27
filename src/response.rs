use serde::Deserialize;
use std::convert::TryInto;

use crate::advice::Advice;
use crate::config::{COMETD_SUPPORTED_TYPES, COMETD_VERSION};
use crate::error::Error;
use crate::request::{ConnectRequest, HandshakeRequest, Request};

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
    pub version: Option<String>,
    pub minimum_version: Option<String>,
    pub client_id: Option<String>,
    pub supported_connection_types: Option<Vec<String>>,
    pub advice: Option<Advice>,
    pub ext: Option<serde_json::Value>,
    pub id: Option<String>,
    pub auth_successful: Option<bool>,
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

impl Response {
    pub fn advice(&self) -> Option<Advice> {
        match self {
            Self::Handshake(resp) => resp.advice.clone(),
            Self::Publish(resp) => resp.advice.clone(),
            Self::Connect(resp) => resp.advice.clone(),
        }
    }

    pub fn successful(&self) -> bool {
        match self {
            Self::Handshake(resp) => resp.successful,
            Self::Publish(resp) => resp.successful,
            Self::Connect(resp) => resp.successful,
        }
    }

    pub fn error(&self) -> Option<String> {
        match self {
            Self::Handshake(resp) => resp.error.clone(),
            Self::Publish(resp) => resp.error.clone(),
            Self::Connect(resp) => resp.error.clone(),
        }
    }
}

impl TryInto<Request> for &Response {
    type Error = Error;

    fn try_into(self) -> Result<Request, Error> {
        match self {
            Response::Handshake(resp) => Ok(Request::Handshake(HandshakeRequest {
                channel: resp.channel.clone(),
                version: COMETD_VERSION.to_owned(),
                minimum_version: resp.minimum_version.clone(),
                supported_connection_types: COMETD_SUPPORTED_TYPES
                    .to_vec()
                    .into_iter()
                    .map(|ct| ct.to_owned())
                    .collect(),
                ext: resp.ext.clone(),
                id: resp.id.clone(),
            })),
            Response::Publish(_) => Err(Error::new(
                "PublishResponse cannot be converted into a Request",
            )),
            Response::Connect(resp) => Ok(Request::Connect(ConnectRequest {
                channel: resp.channel.clone(),
                ext: resp.ext.clone(),
                client_id: resp.client_id.clone(),
                id: resp.id.clone(),
            })),
        }
    }
}
