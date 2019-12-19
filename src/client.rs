use reqwest::{Client as ReqwestClient, Response as ReqwestReponse, Url};
use serde::Serialize;
use std::time::Duration;

use crate::advice::{Advice, Reconnect};
use crate::config::{COMETD_SUPPORTED_TYPES, COMETD_VERSION};
use crate::error::Error;
use crate::response::{ErroredResponse, Response};

/// The cometd client.
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

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct PublishPayload<'a, T>
where
    T: Serialize,
{
    pub channel: &'a str,
    pub client_id: &'a str,
    pub data: T,
}

impl Client {
    /// Creates a new cometd client. It is expected to provide the url of the cometd server,
    /// the access token to allow the communication and the timeout for long-polling requests.
    ///
    /// # Errors
    ///
    /// Will return an error if the http client cannot be initalized.
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

    /// Sets the number of retries the client will attempt in case of an error or a retry advice is
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
                if self.actual_retries <= self.max_retries {
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
                if self.actual_retries <= self.max_retries {
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

    /// The cometd connect method. It will hang for a response from the server according
    /// to the timeout provided to the cometd client.
    /// If one or several sucess responses are returned to the request, it will return a `Vec`
    /// containing those responses.
    /// If an errored response is received but an advice is provided by the server, the client
    /// will try to follow this advice and re-attemp the connection. If the maximum number of retries
    /// is reached and the response still does not succeed, it will return an error.
    ///
    /// # Errors
    ///
    /// The cometd server's response could not be parsed.
    /// The cometd server returned a response that indicated an error and the request could not be
    /// retried or the maximum number of retries has been reached.
    pub fn connect(&mut self) -> Result<Vec<Response>, Error> {
        let resps = self.retry();

        self.actual_retries = 0;
        resps
    }

    /// The cometd disconnect method.
    /// If one or several sucess responses are returned to the request, it will return a `Vec`
    /// containing those responses.
    ///
    /// # Errors
    ///
    /// The cometd server's response could not be parsed.
    /// The cometd server returned a response that indicated an error and the request could not be
    /// retried or the maximum number of retries has been reached.
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

    /// Init the cometd client. It will attempt to establish a handshake between
    /// the client and the server so it can make further requests.
    pub fn init(&mut self) -> Result<Vec<Response>, Error> {
        let resps = self.handshake()?;

        log::info!("Successfully init cometd client");
        Ok(resps)
    }

    /// The cometd subscribe method. It will ask the server to subscribe to a certain channel and therefore
    /// be updated when something is posted on this channel.
    /// If one or several sucess responses are returned to the request, it will return a `Vec`
    /// containing those responses.
    /// If an errored response is received but an advice is provided by the server, the client
    /// will try to follow this advice and re-attemp the connection. If the maximum number of retries
    /// is reached and the response still does not succeed, it will return an error.
    ///
    /// # Errors
    ///
    /// The cometd server's response could not be parsed.
    /// The cometd server returned a response that indicated an error and the request could not be
    /// retried or the maximum number of retries has been reached.
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

    /// The cometd subscribe method. It will ask the server to unsubscribe from a certain channel and therefore
    /// strop being updated when something is posted on this channel.
    /// If one or several sucess responses are returned to the request, it will return a `Vec`
    /// containing those responses.
    /// If an errored response is received but an advice is provided by the server, the client
    /// will try to follow this advice and re-attemp the connection. If the maximum number of retries
    /// is reached and the response still does not succeed, it will return an error.
    ///
    /// # Errors
    ///
    /// The cometd server's response could not be parsed.
    /// The cometd server returned a response that indicated an error and the request could not be
    /// retried or the maximum number of retries has been reached.
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

    /// The cometd plublish method. It will ask the server to publish a message to a certain channel.
    /// If one or several sucess responses are returned to the request, it will return a `Vec`
    /// containing those responses.
    /// If an errored response is received but an advice is provided by the server, the client
    /// will try to follow this advice and re-attemp the connection. If the maximum number of retries
    /// is reached and the response still does not succeed, it will return an error.
    ///
    /// # Errors
    ///
    /// The cometd server's response could not be parsed.
    /// The cometd server returned a response that indicated an error and the request could not be
    /// retried or the maximum number of retries has been reached.
    pub fn publish(&mut self, channel: &str, data: impl Serialize) -> Result<Vec<Response>, Error> {
        match &self.client_id {
            Some(client_id) => {
                let resp = self.send_request(&PublishPayload {
                    channel,
                    client_id,
                    data,
                })?;

                self.handle_response(resp)
            }
            None => Err(Error::new("No client id set for unsubscribe")),
        }
    }
}
