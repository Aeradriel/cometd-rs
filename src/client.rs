use reqwest::{Client as ReqwestClient, Response as ReqwestReponse, Url};
use serde::Serialize;
use std::convert::TryInto;
use std::time::Duration;

use crate::advice::Reconnect;
use crate::error::Error;
use crate::request::Request;
use crate::response::Response;

pub struct Client {
    http_client: ReqwestClient,
    base_url: Url,
    access_token: String,
    client_id: Option<String>,
    cookies: Vec<String>,
    retries: i8,
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
            http_client: http_client,
            base_url: url,
            access_token: access_token.to_owned(),
            client_id: None,
            cookies: vec![],
            retries: 1,
        })
    }

    pub fn set_retries(mut self, retries: i8) -> Self {
        self.retries = retries;
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

        log::info!(
            "Sending request to cometd with the following body:\n{:#?}",
            serde_json::to_string(body)
        );
        req.send()
            .map_err(|_| Error::new("Could not send request to server"))
    }

    fn retry(&mut self, resp: &Response, retry: i8) -> Result<Vec<Response>, Error> {
        let req: Request = resp.try_into()?;

        let resp = self.send_request(&req)?;

        log::info!("Retrying attempt n°{} for {:#?}", retry + 1, req);
        self.handle_response(resp, retry + 1)
    }

    /// Handles the error returned by the cometd server. If possible, it will
    /// automatically retry according to the client configuration. If it still
    /// fails after the retries, the original error will be returned.
    fn handle_error(&mut self, resp: &Response, retry: i8) -> Result<Vec<Response>, Error> {
        match resp.advice() {
            Some(advice) => {
                match advice.reconnect {
                    Reconnect::Retry | Reconnect::Handshake => {
                        if retry < self.retries {
                            match self.retry(resp, retry) {
                                Ok(resps) => Ok(resps),
                                Err(err) => Err(err),
                            }
                        } else {
                            let error = match resp.error() {
                                Some(err) => err,
                                None => "Response was unsuccessful even after retries".to_owned(),
                            };

                            Err(Error::new(&error))
                        }
                    }
                    Reconnect::None => {
                        let error = match resp.error() {
                        Some(err) => err,
                        None => "Response was unsuccessful and the server indicated not to retry nor handshake".to_owned(),
                    };

                        log::info!("Not retrying because the server answered not to reconnect nor handshake");
                        Err(Error::new(&error))
                    }
                }
            }
            None => {
                let error = match resp.error() {
                    Some(err) => err,
                    None => "Response was unsuccessful and no advice was provided".to_owned(),
                };

                Err(Error::new(&error))
            }
        }
    }

    fn handle_response(
        &mut self,
        mut resp: ReqwestReponse,
        retry: i8,
    ) -> Result<Vec<Response>, Error> {
        let body = resp
            .text()
            .map_err(|_| Error::new("Could not get the response body"))?;
        let cookies = resp
            .cookies()
            .map(|c| c.value().to_owned())
            .collect::<Vec<_>>();

        log::info!("Received response from cometd server:\n{:#?}", body);
        match serde_json::from_str::<Vec<Response>>(&body) {
            Ok(resps) => {
                let mut responses = vec![];

                for resp in resps.into_iter() {
                    if resp.successful() {
                        if let Response::Handshake(ref resp) = resp {
                            self.client_id = Some(resp.client_id.clone().ok_or(Error::new(
                                "Handshake was successful but not client id was provided",
                            ))?);
                            self.cookies = cookies.clone();
                        }
                        responses.push(resp);
                    } else {
                        let resps = self.handle_error(&resp, retry)?;

                        for resp in resps.into_iter() {
                            responses.push(resp);
                        }
                    }
                }
                Ok(responses)
            }
            Err(_) => {
                log::error!(
                    "Handle response failed with the following server response:\n{:#?}",
                    body
                );
                Err(Error::new("Could not parse response"))
            }
        }
    }

    fn handshake(&mut self) -> Result<Vec<Response>, Error> {
        let resp = self.send_request(&HandshakePayload {
            channel: "/meta/handshake",
            version: "1.0",
            supported_connection_types: vec!["long-polling"],
        })?;

        self.handle_response(resp, 0)
    }

    pub fn connect(&mut self) -> Result<Vec<Response>, Error> {
        match &self.client_id {
            Some(client_id) => {
                let resp = self.send_request(&ConnectPayload {
                    channel: "/meta/connect",
                    client_id: &client_id,
                    connection_type: "long-polling",
                })?;

                self.handle_response(resp, 0)
            }
            None => Err(Error::new("No client id set for connect")),
        }
    }

    pub fn init(&mut self) -> Result<Vec<Response>, Error> {
        let mut responses = vec![];

        responses.push(self.handshake()?);
        responses.push(self.connect()?);
        log::info!("Successfully init cometd client");
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

                self.handle_response(resp, 0)
            }
            None => Err(Error::new("No client id set for subscribe")),
        }
    }
}
