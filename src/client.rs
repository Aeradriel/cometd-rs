use reqwest::{Client as ReqwestClient, Response as ReqwestReponse, Url};
use serde::Serialize;
use std::time::Duration;

use crate::advice::{Advice, Reconnect};
use crate::config::{COMETD_SUPPORTED_TYPES, COMETD_VERSION};
use crate::error::Error;
use crate::response::{ErroredResponse, Response};

pub struct Client {
    http_client: ReqwestClient,
    base_url: Url,
    access_token: String,
    client_id: Option<String>,
    cookies: Vec<String>,
    max_retries: i8,
    actual_retries: i8,
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
struct DisconnectPayload<'a> {
    channel: &'a str,
    client_id: &'a str,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SubscribeTopicPayload<'a> {
    pub channel: &'a str,
    pub client_id: &'a str,
    pub subscription: &'a str,
}

impl Client {
    pub fn new(base_url: &str, access_token: &str, timeout: Duration) -> Result<Client, Error> {
        let url = Url::parse(base_url).map_err(|_| Error::new("Could not parse base url"))?;
        let http_client = ReqwestClient::builder()
            .cookie_store(true)
            .timeout(timeout)
            .build()
            .map_err(|_| Error::new("Could not initialize http client"))?;

        log::info!("Successfully created cometd client");
        Ok(Client {
            http_client,
            base_url: url,
            access_token: access_token.to_owned(),
            client_id: None,
            cookies: vec![],
            actual_retries: 0,
            max_retries: 1,
        })
    }

    /// Sets the number of retries the client will attempt in case of an error is
    /// returned by the cometd server.
    pub fn set_retries(mut self, retries: i8) -> Self {
        self.max_retries = retries;
        self
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

        log::debug!(
            "Sending request to cometd with the following body: {:?}",
            serde_json::to_string(body)
        );
        req.send()
            .map_err(|_| Error::new("Could not send request to server"))
    }

    // TODO: Allow disable retry
    fn retry(&mut self) -> Result<Vec<Response>, Error> {
        self.actual_retries += 1;
        log::debug!("Attempt n°{}", self.actual_retries);

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

    fn retry_handshake(&mut self) -> Result<Vec<Response>, Error> {
        self.actual_retries += 1;
        log::debug!("Attempt n°{}", self.actual_retries);

        let resp = self.send_request(&HandshakePayload {
            channel: "/meta/handshake",
            version: COMETD_VERSION,
            supported_connection_types: COMETD_SUPPORTED_TYPES.to_vec(),
        })?;

        self.handle_response(resp)
    }

    fn handle_advice(
        &mut self,
        advice: &Advice,
        error: Option<&str>,
    ) -> Result<Vec<Response>, Error> {
        log::debug!("Following advice from server");
        match advice.reconnect {
            Reconnect::Handshake => {
                if self.actual_retries < self.max_retries {
                    match self.retry_handshake() {
                        Ok(_) => match self.retry() {
                            Ok(resps) => Ok(resps),
                            Err(err) => Err(err),
                        },
                        Err(err) => Err(err),
                    }
                } else {
                    Err(Error::new(error.unwrap_or("Max retries reached")))
                }
            }
            Reconnect::Retry => {
                if self.actual_retries < self.max_retries {
                    match self.retry() {
                        Ok(resps) => Ok(resps),
                        Err(err) => Err(err),
                    }
                } else {
                    Err(Error::new(error.unwrap_or("Max retries reached")))
                }
            }
            Reconnect::None => {
                log::debug!(
                    "Not retrying because the server answered not to reconnect nor handshake"
                );
                Err(Error::new(error.unwrap_or(
                    "Service advised not to reconnect nor handshake",
                )))
            }
        }
    }

    /// Handles the error returned by the cometd server. If possible, it will
    /// automatically retry according to the client configuration. If it still
    /// fails after the retries, the original error will be returned.
    fn handle_error(&mut self, resp: &ErroredResponse) -> Result<Vec<Response>, Error> {
        match resp.advice {
            Some(ref advice) => self.handle_advice(advice, Some(&resp.error)),
            None => {
                log::debug!("Not retrying because the server did not provide advice");
                Err(Error::new(&resp.error))
            }
        }
    }

    fn handle_response(&mut self, mut resp: ReqwestReponse) -> Result<Vec<Response>, Error> {
        let body = resp
            .text()
            .map_err(|_| Error::new("Could not get the response body"))?;
        let cookies = resp
            .cookies()
            .map(|c| c.value().to_owned())
            .collect::<Vec<_>>();
        let mut responses = vec![];

        log::debug!("Received response from cometd server: {:?}", body);
        match serde_json::from_str::<Vec<ErroredResponse>>(&body) {
            Ok(resps) => {
                for resp in resps.into_iter() {
                    let resps = self.handle_error(&resp)?;

                    for resp in resps.into_iter() {
                        responses.push(resp);
                    }
                }
                Ok(responses)
            }
            Err(_) => match serde_json::from_str::<Vec<Response>>(&body) {
                Ok(resps) => {
                    let mut responses = vec![];

                    for resp in resps.into_iter() {
                        if let Some(ref advice) = resp.advice() {
                            for resp in self.handle_advice(advice, None)? {
                                responses.push(resp);
                            }
                        } else {
                            if let Response::Handshake(ref resp) = resp {
                                self.client_id = Some(resp.client_id.clone());
                                self.cookies = cookies.clone();
                            }
                            responses.push(resp);
                        }
                    }
                    Ok(responses)
                }
                Err(_) => {
                    log::error!(
                        "Handle response failed with the following server response: {:?}",
                        body
                    );
                    Err(Error::new("Could not parse response"))
                }
            },
        }
    }

    fn handshake(&mut self) -> Result<Vec<Response>, Error> {
        let resps = self.retry_handshake();

        self.actual_retries = 0;
        resps
    }

    pub fn connect(&mut self) -> Result<Vec<Response>, Error> {
        let resps = self.retry();

        self.actual_retries = 0;
        resps
    }

    pub fn disconnect(&mut self) -> Result<Vec<Response>, Error> {
        match &self.client_id {
            Some(client_id) => {
                let resp = self.send_request(&DisconnectPayload {
                    channel: "/meta/disconnect",
                    client_id,
                })?;

                self.handle_response(resp)
            }
            None => Err(Error::new("No client id set for disconnect")),
        }
    }

    pub fn init(&mut self) -> Result<Vec<Response>, Error> {
        let resps = self.handshake()?;

        log::info!("Successfully init cometd client");
        Ok(resps)
    }

    pub fn subscribe(&mut self, subscription: &str) -> Result<Vec<Response>, Error> {
        match &self.client_id {
            Some(client_id) => {
                let resp = self.send_request(&SubscribeTopicPayload {
                    channel: "/meta/subscribe",
                    client_id,
                    subscription,
                })?;

                self.handle_response(resp)
            }
            None => Err(Error::new("No client id set for subscribe")),
        }
    }

    pub fn unsubscribe(&mut self, subscription: &str) -> Result<Vec<Response>, Error> {
        match &self.client_id {
            Some(client_id) => {
                let resp = self.send_request(&SubscribeTopicPayload {
                    channel: "/meta/unsubscribe",
                    client_id,
                    subscription,
                })?;

                self.handle_response(resp)
            }
            None => Err(Error::new("No client id set for unsubscribe")),
        }
    }
}
