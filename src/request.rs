use serde::Serialize;

#[derive(Serialize, PartialEq, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Request {
    pub channel: String,
    pub version: String,
    pub subscription: Option<String>,
    pub client_id: Option<String>,
    pub minimum_version: Option<String>,
    pub supported_connection_types: Vec<String>,
    pub ext: Option<serde_json::Value>,
    pub id: Option<String>,
}
