use mockito::mock;
use std::time::Duration;

use crate::client::Client;

static VALID_ACCESS_TOKEN: &'static str = "1234";
static RETRIES_MAX: i8 = 3;

fn client() -> Client {
    Client::new(
        &format!("{}", mockito::server_url()),
        VALID_ACCESS_TOKEN,
        Duration::from_secs(120),
    )
    .expect("Could not build cometd client")
    .set_retries(RETRIES_MAX)
}

mod init {
    use super::*;

    #[test]
    fn returns_error_on_failure() {
        let _m = mock("POST", "/")
            .with_status(200)
            .with_body("[{\"channel\":\"/meta/handshake\",\"error\":\"406::Unsupported version, or unsupported minimum version\",\"successful\":false}]")
            .create();
        let mut client = client();

        assert!(client.init().is_err());
    }

    #[test]
    fn works() {
        let _m = mock("POST", "/")
            .with_status(200)
            .with_body(
                "[{\"channel\":\"/meta/handshake\",\"version\":\"1.0\",\"successful\":true,\"clientId\":\"1234\",\"supportedConnectionTypes\":[\"long-polling\"]}]",
            )
            .create();
        let mut client = client();

        assert!(client.init().is_ok());
    }
}

mod connect {
    use super::*;

    #[test]
    fn retries_if_server_advises_to() {
        let _m = mock("POST", "/")
            .match_body(
                "{\"channel\":\"/meta/handshake\",\"version\":\"1.0\",\"supportedConnectionTypes\":[\"long-polling\"]}"
            )
            .with_status(200)
            .with_body(
                "[{\"channel\":\"/meta/handshake\",\"version\":\"1.0\",\"successful\":true,\"clientId\":\"1234\",\"supportedConnectionTypes\":[\"long-polling\"]}]",
            )
            .create();
        let connect_mock = mock("POST", "/")
            .match_body(
                "{\"channel\":\"/meta/connect\",\"clientId\":\"1234\",\"connectionType\":\"long-polling\"}"
            )
            .with_status(200)
            .with_body("[{\"advice\":{\"reconnect\":\"retry\"},\"channel\":\"/meta/connect\",\"error\":\"400::Error\",\"successful\":false}]")
            .expect(RETRIES_MAX as usize + 1)
            .create();
        let mut client = client();

        client.init().expect("Could not init client");
        client.connect().expect_err("Connect should not return Ok");
        connect_mock.assert();
    }

    #[test]
    fn handshake_if_advises_to() {
        let hs_mock = mock("POST", "/")
            .match_body(
                "{\"channel\":\"/meta/handshake\",\"version\":\"1.0\",\"supportedConnectionTypes\":[\"long-polling\"]}"
            )
            .with_status(200)
            .with_body(
                "[{\"channel\":\"/meta/handshake\",\"version\":\"1.0\",\"successful\":true,\"clientId\":\"1234\",\"supportedConnectionTypes\":[\"long-polling\"]}]",
            )
            .expect(RETRIES_MAX as usize) // Will do : Handshake + Connect + Retry HS (1) + Connect (2) + Retry HS (3)
            .create();
        let _m = mock("POST", "/")
            .match_body(
                "{\"channel\":\"/meta/connect\",\"clientId\":\"1234\",\"connectionType\":\"long-polling\"}"
            )
            .with_status(200)
            .with_body(
                "[{\"advice\":{\"reconnect\":\"handshake\"},\"channel\":\"/meta/connect\",\"successful\":false,\"error\":\"error\"}]",
            )
            .create();
        let mut client = client();

        client.init().expect("Could not init client");
        let resp = client.connect().expect_err("Connect should not return Ok");
        println!("{:#?}", resp);
        hs_mock.assert();
    }
}

mod subscribe {}
mod unsubscribe {}
mod publish {}
