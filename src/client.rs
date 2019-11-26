use reqwest::{Client as ReqwestClient, Url};
use serde::{Deserialize, Serialize};
use std::time::Duration;

use crate::error::Error;

pub struct Client {
    pub http_client: ReqwestClient,
    pub base_url: Url,
    pub access_token: String,
    pub client_id: Option<String>,
    pub cookies: Vec<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct HandshakePayload<'a> {
    channel: &'a str,
    version: &'a str,
    supported_connection_types: Vec<&'a str>,
}

#[derive(Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
struct HandshakeResponse {
    client_id: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ConnectPayload<'a> {
    channel: &'a str,
    client_id: &'a str,
    connection_type: &'a str,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SubscribeTopicPayload {
    pub channel: String,
    pub client_id: String,
    pub subscription: String,
}

impl Client {
    pub fn new(base_url: &str, access_token: &str, timeout: Duration) -> Result<Client, Error> {
        let url = Url::parse(base_url).map_err(|_| Error::new("Could not parse base url"))?;
        let http_client = ReqwestClient::builder()
            .cookie_store(true)
            .timeout(timeout)
            .build()
            .map_err(|_| Error::new("Could not initialize http client"))?;

        Ok(Client {
            http_client: http_client,
            base_url: url,
            access_token: access_token.to_owned(),
            client_id: None,
            cookies: vec![],
        })
    }

    fn handshake(&mut self) -> Result<(), Error> {
        let req = self
            .http_client
            .post(self.base_url.clone())
            .header("Authorization", &format!("OAuth {}", self.access_token))
            .json(&HandshakePayload {
                channel: "/meta/handshake",
                version: "1.0",
                supported_connection_types: vec!["long-polling"],
            });
        let mut resp = req
            .send()
            .map_err(|_| Error::new("Could not send handshake request to server"))?;
        let body = resp
            .text()
            .map_err(|_| Error::new("Could not get the handshake response body"))?;
        let cookies = resp
            .cookies()
            .map(|c| c.value().to_owned())
            .collect::<Vec<_>>();

        match serde_json::from_str::<Vec<HandshakeResponse>>(&body) {
            Ok(vals) => match vals.get(0) {
                Some(val) => {
                    self.client_id = Some(val.client_id.clone());
                    self.cookies = cookies;
                    Ok(())
                }
                None => Err(Error::new("Could not get client id from handshake")),
            },
            Err(_) => Err(Error::new("Could not get client id from handshake")),
        }
    }

    pub fn connect(&self) -> Result<(), Error> {
        match &self.client_id {
            Some(client_id) => {
                let mut req = self
                    .http_client
                    .post(self.base_url.clone())
                    .header("Authorization", &format!("OAuth {}", self.access_token));

                for ref cookie in self.cookies.iter() {
                    req = req.header(reqwest::header::SET_COOKIE, cookie.clone());
                }

                req.json(&ConnectPayload {
                    channel: "/meta/connect",
                    client_id: &client_id,
                    connection_type: "long-polling",
                })
                .send()
                .map_err(|_| Error::new("Could not send connect request to server"))?;

                Ok(())
            }
            None => Err(Error::new("No client id set for connect")),
        }
    }

    pub fn init(&mut self) -> Result<(), Error> {
        self.handshake()?;
        self.connect()?;
        Ok(())
    }

    pub fn subscribe(&mut self, sub: &str) -> Result<(), Error> {
        match &self.client_id {
            Some(client_id) => {
                let mut req = self
                    .http_client
                    .post(self.base_url.clone())
                    .header("Authorization", &format!("Bearer {}", self.access_token));

                for cookie in &self.cookies {
                    req = req.header(reqwest::header::SET_COOKIE, cookie);
                }

                req = req.json(&SubscribeTopicPayload {
                    channel: "/meta/subscribe".to_owned(),
                    client_id: client_id.to_owned(),
                    subscription: sub.to_owned(),
                });
                req.send()
                    .map_err(|_| Error::new("Could not send subscribe request to server"))?;

                Ok(())
            }
            None => Err(Error::new("No client id set for subscribe")),
        }
    }
}
