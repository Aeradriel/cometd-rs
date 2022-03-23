# cometd-rs
Cometd implementation in Rust (only supports long-polling connections)

# SF implementation example

The first thing to do is to log into SF to retrieve your credentials.
Then you should create and initialize the cometd client to listen to the topic you previously created.
One this is done, all you have to do is call the `connect` function on you cometdclient and loop through the responses to handle it.

```rust
#[derive(Serialize)]
struct LoginPayload<'a> {
    grant_type: &'a str,
    client_id: &'a str,
    client_secret: &'a str,
    username: &'a str,
    password: &'a str,
}

#[derive(Deserialize)]
pub struct AuthInfos {
    pub access_token: String,
    pub instance_url: String,
}

pub fn log_in(client: &ReqwestClient) -> Result<AuthInfos, reqwest::Error> {
    let url =
        reqwest::Url::parse(&env::var("SF_URL").expect("SF_URL env var not set")).expect("Bad URI");
    let req = client
        .post(url)
        .header("Content-type", "application/x-www-form-urlencoded")
        .body(
            serde_qs::to_string(&LoginPayload {
                grant_type: "password",
                client_id: &env::var("CLIENT_ID").expect("Client id not set"),
                client_secret: &env::var("CLIENT_SECRET").expect("Client secret not set"),
                username: &env::var("SF_LOGIN").expect("SF_LOGIN env var not set"),
                password: &format!(
                    "{}{}",
                    &env::var("SF_PASS").expect("SF_PASS env var not set"),
                    &env::var("SECURITY_TOKEN").expect("Security token not set")
                ),
            })
            .expect("Could not serialize login payload"),
        );
    let mut resp = req.send()?;
    let body = resp.text()?;
    let auth_infos = serde_json::from_str(&body).expect("Could not deserialize login response");

    Ok(auth_infos)
}
      
fn create_cometd_client(base_url: &str, access_token: &str) -> CometdClient {
    let sf_url = format!("{}/cometd/37.0", base_url);
    let access_token = access_token;
    let timeout = std::time::Duration::from_secs(120);

    CometdClient::new(&sf_url, access_token, timeout)
        .expect("Failed to create cometd client")
        .set_retries(3)
}

// Subscribe to the channels you created in Salesforce
fn init_cometd_client(mut cometd_client: CometdClient) -> CometdClient {
    cometd_client.init().expect("Could not init cometd client");
    cometd_client
        .subscribe("/topic/StatusUpdate")
        .expect("Could not subscribe to StatusUpdate topic");
    cometd_client
        .subscribe("/topic/OpportunityUpdate")
        .expect("Could not subscribe to OpportunityUpdate topic");
    cometd_client
        .subscribe("/topic/AccountUpdate")
        .expect("Could not subscribe to AccountUpdate topic");
    cometd_client
        .subscribe("/topic/ContactUpdate")
        .expect("Could not subscribe to ContactUpdate topic");
    log::debug!("Cometd client successfully initialized");
    cometd_client
}
      
pub fn listen_sf(mut client: CometdClient) {
    info!("Listen SF loop started");
    loop {
        let responses = client.connect();

        match responses {
            Ok(responses) => {
                for response in responses {
                    if let Response::Delivery(resp) = response {
                        match serde_json::from_value::<SFDelivery>(resp.data.clone()) {
                            Ok(data) => match data.sobject {
                                // Here you should have your patterns matching your own objects
                            },
                            Err(err) => error!(
                                "SF delivery data could not be parsed: {:?}\nData:{:?}",
                                err, resp
                            ),
                        }
                    }
                }
            }
            Err(err) => error!("{}", err.message),
        }
    }
}

fn main() {
    let reqwest_client = ReqwestClient::builder()
        .cookie_store(true)
        .build()
        .expect("Client failed to initialize");
    let auth_infos = log_in(&reqwest_client).expect("Could not log into SF");
    let cometd_client = create_cometd_client(&auth_infos.instance_url, &auth_infos.access_token);
    let cometd_client = init_cometd_client(cometd_client);

    listen_sf(cometd_client);
}
```
