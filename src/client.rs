use crate::error::Error;
use reqwest::{Client as ReqwestClient, Url};
use serde::{Deserialize, Serialize};

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

impl Client {
    pub fn new(base_url: &str, access_token: &str) -> Result<Client, Error> {
        let url = Url::parse(base_url).map_err(|_| Error::new("Could not parse base url"))?;
        let http_client = ReqwestClient::builder()
            .cookie_store(true)
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
            .map_err(|_| Error::new("Could not send request to server"))?;
        let body = resp
            .text()
            .map_err(|_| Error::new("Could not get the response body"))?;
        let cookies = resp
            .cookies()
            .map(|c| c.value().to_owned())
            .collect::<Vec<_>>();

        if let Ok(vals) = serde_json::from_str::<Vec<HandshakeResponse>>(&body) {
            if let Some(val) = vals.get(0) {
                self.client_id = Some(val.client_id.clone());
                self.cookies = cookies;
            }
        }

        Ok(())
    }

    fn connect(&self) -> Result<(), Error> {
        if let Some(client_id) = &self.client_id {
            let mut req = self
                .http_client
                .post(self.base_url.clone())
                .header("Authorization", &format!("OAuth {}", self.access_token));

            for ref cookie in self.cookies.iter() {
                req = req.header(reqwest::header::SET_COOKIE, cookie.clone());
            }

            let req = req.json(&ConnectPayload {
                channel: "/meta/connect",
                client_id: &client_id,
                connection_type: "long-polling",
            });
            let mut resp = req.send().expect("Could not send req");

            println!("CONNECT BODY: {:#?}", resp.text());
            Ok(())
        } else {
            Err(Error::new("No client id set for connect"))
        }
    }

    pub fn init(&mut self) -> Result<(), Error> {
        self.handshake()?;
        self.connect()?;
        Ok(())
    }
}
