use serde::Serialize;

#[derive(Serialize, PartialEq, Debug)]
#[serde(untagged)]
pub enum Request {
    Handshake(HandshakeRequest),
    Connect(ConnectRequest),
}

#[derive(Serialize, PartialEq, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ConnectRequest {
    pub channel: String,
    pub ext: Option<serde_json::Value>,
    pub client_id: Option<String>,
    pub id: Option<String>,
}

#[derive(Serialize, PartialEq, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct HandshakeRequest {
    pub channel: String,
    pub version: String,
    pub minimum_version: Option<String>,
    pub supported_connection_types: Vec<String>,
    pub ext: Option<serde_json::Value>,
    pub id: Option<String>,
}
