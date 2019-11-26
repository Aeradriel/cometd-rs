use reqwest::{Client as ReqwestClient, Response as ReqwestReponse, Url};
use serde::Serialize;
use std::time::Duration;

use crate::error::Error;
use crate::response::Response;

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

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ConnectPayload<'a> {
    channel: &'a str,
    client_id: &'a str,
    connection_type: &'a str,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SubscribeTopicPayload<'a> {
    pub channel: &'a str,
    pub client_id: &'a str,
    pub subscription: &'a str,
}

// TODO: Logs
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

    fn send_request(&self, body: &impl Serialize) -> Result<ReqwestReponse, Error> {
        let mut req = self
            .http_client
            .post(self.base_url.clone())
            .header("Authorization", &format!("OAuth {}", self.access_token))
            .json(body);

        for ref cookie in self.cookies.iter() {
            req = req.header(reqwest::header::SET_COOKIE, cookie.clone());
        }

        req.send()
            .map_err(|_| Error::new("Could not send request to server"))
    }

    fn handle_response(&mut self, mut resp: ReqwestReponse) -> Result<Vec<Response>, Error> {
        let body = resp
            .text()
            .map_err(|_| Error::new("Could not get the response body"))?;
        let cookies = resp
            .cookies()
            .map(|c| c.value().to_owned())
            .collect::<Vec<_>>();

        // TODO: Retry if needed
        // TODO: Handshake if needed
        match serde_json::from_str::<Vec<Response>>(&body) {
            Ok(resps) => {
                let mut responses = vec![];

                for resp in resps.into_iter() {
                    if let Response::Handshake(ref resp) = resp {
                        self.client_id = Some(resp.client_id.clone());
                        self.cookies = cookies.clone();
                    }
                    responses.push(resp);
                }
                Ok(responses)
            }
            Err(_) => Err(Error::new("Could not parse response")),
        }
    }

    fn handshake(&mut self) -> Result<Vec<Response>, Error> {
        let resp = self.send_request(&HandshakePayload {
            channel: "/meta/handshake",
            version: "1.0",
            supported_connection_types: vec!["long-polling"],
        })?;

        self.handle_response(resp)
    }

    pub fn connect(&mut self) -> Result<Vec<Response>, Error> {
        match &self.client_id {
            Some(client_id) => {
                let resp = self.send_request(&ConnectPayload {
                    channel: "/meta/connect",
                    client_id: &client_id,
                    connection_type: "long-polling",
                })?;

                self.handle_response(resp)
            }
            None => Err(Error::new("No client id set for connect")),
        }
    }

    pub fn init(&mut self) -> Result<Vec<Response>, Error> {
        let mut responses = vec![];

        responses.push(self.handshake()?);
        responses.push(self.connect()?);
        Ok(responses.into_iter().flatten().collect())
    }

    pub fn subscribe(&mut self, sub: &str) -> Result<Vec<Response>, Error> {
        match &self.client_id {
            Some(client_id) => {
                let resp = self.send_request(&SubscribeTopicPayload {
                    channel: "/meta/subscribe",
                    client_id: client_id,
                    subscription: sub,
                })?;

                self.handle_response(resp)
            }
            None => Err(Error::new("No client id set for subscribe")),
        }
    }
}
