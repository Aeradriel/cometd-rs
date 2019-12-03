use mockito;
use std::time::Duration;

use crate::client::Client;

static VALID_ACCESS_TOKEN: &'static str = "1234";

fn client() -> Client {
    Client::new(
        &format!("{}/cometd/37.0", mockito::server_url()),
        VALID_ACCESS_TOKEN,
        Duration::from_secs(120),
    )
    .expect("Could not build cometd client")
}

mod tests {
    mod init {}
    mod connect {}
    mod subscribe {}
    mod unsubscribe {}
    mod publish {}
    mod retries {}
}
