use crate::settings::DidType;
use log::{debug, error, info};
use reqwest::Url;
use tsp_sdk::{AsyncSecureStore, Error, OwnedVid, VerifiedVid, Vid, vid::VidError};
use uuid::Uuid;

/// Creates a DID and stores it in the provided wallet.
///
/// The `vid_wallet` parameter is the wallet in which to store the created DID.
///
/// The `client` parameter is the HTTP client used to publish the DID to the server.
///
/// The `did_server` parameter is the server URL used to publish the DID and DID history.
///
/// Returns the created DID if successful, otherwise an error.
pub async fn create(
    address: Option<&str>,
    server: String,
    alias: Option<String>,
    vid_wallet: &mut AsyncSecureStore,
    r#type: &DidType,
    client: &reqwest::Client,
    did_server: &str,
) -> Result<OwnedVid, Error> {
    let username: String = format!(
        "{}-{}",
        alias.as_ref().unwrap_or(&String::new()).replace(' ', ""),
        Uuid::new_v4()
    )
    .chars()
    .take(63)
    .collect();
    let transport = if let Some(address) = address {
        Url::parse(&format!("tcp://{address}")).unwrap()
    } else {
        Url::parse(&format!("https://{server}/endpoint/[vid_placeholder]",)).unwrap()
    };

    let private_vid = match r#type {
        DidType::Web => {
            create_did_web(
                &did_server,
                transport,
                &vid_wallet,
                &username,
                alias,
                &client,
            )
            .await?
        }
        DidType::Peer => {
            let private_vid = OwnedVid::new_did_peer(transport);

            vid_wallet.set_alias(username.clone(), private_vid.identifier().to_string())?;

            info!("created peer identity {}", private_vid.identifier());
            private_vid
        }
        DidType::Webvh => {
            println!("did_server: {did_server} username: {username} transport: {transport}");
            let (private_vid, history, update_kid, update_key) =
                tsp_sdk::vid::did::webvh::create_webvh(
                    &format!("{did_server}/endpoint/{username}"),
                    transport,
                )
                .await?;
            println!("private_vid: {:?}", private_vid);
            vid_wallet
                .add_secret_key(update_kid, update_key)
                .expect("Cannot store update key");

            let _: Vid = match client
                .post(format!("https://{did_server}/add-vid"))
                .json(&private_vid.vid())
                .send()
                .await
                .inspect(|r| debug!("DID server responded with status code {}", r.status()))
                .expect("Could not publish VID on server")
                .error_for_status()
            {
                Ok(response) => response.json().await.expect("Could not decode VID"),
                Err(e) => {
                    error!(
                        "{e}\nAn error occurred while publishing the DID. Maybe this DID exists already?"
                    );
                    return Err(Error::Vid(VidError::InvalidVid(
                            "An error occurred while publishing the DID. Maybe this DID exists already?"
                                .to_string(),
                        )));
                }
            };
            info!(
                "published DID document at {}",
                tsp_sdk::vid::did::get_resolve_url(private_vid.vid().identifier())?.to_string()
            );

            match client
                .post(format!(
                    "https://{did_server}/add-history/{}",
                    private_vid.vid().identifier()
                ))
                .json(&history)
                .send()
                .await
                .inspect(|r| debug!("DID server responded with status code {}", r.status()))
                .expect("Could not publish history on server")
                .error_for_status()
            {
                Ok(_) => {}
                Err(e) => {
                    error!(
                        "{e}\nAn error occurred while publishing the DID. Maybe this DID exists already?"
                    );
                    return Err(Error::Vid(VidError::InvalidVid(
                            "An error occurred while publishing the DID. Maybe this DID exists already?"
                                .to_string(),
                        )));
                }
            };
            info!("published DID history");
            if let Some(alias) = alias {
                vid_wallet.set_alias(alias, private_vid.identifier().to_string())?;
            }

            private_vid
        }
    };
    Ok(private_vid)
}

/// Creates a DID document on the server and binds it to the given transport.
async fn create_did_web(
    did_server: &str,
    transport: Url,
    vid_wallet: &AsyncSecureStore,
    username: &str,
    alias: Option<String>,
    client: &reqwest::Client,
) -> Result<OwnedVid, Error> {
    let did = format!(
        "did:web:{}:endpoint:{username}",
        did_server.replace(":", "%3A").replace("/", ":")
    );

    if let Some(alias) = alias {
        vid_wallet.set_alias(alias.clone(), did.clone())?;
        info!("added alias {alias} -> {did}");
    }

    let transport = Url::parse(
        &transport
            .as_str()
            .replace("[vid_placeholder]", &did.replace("%", "%25")),
    )
    .unwrap();

    let private_vid = OwnedVid::bind(&did, transport);
    info!("created identity {}", private_vid.identifier());

    let response = client
        .post(format!("https://{did_server}/add-vid"))
        .json(&private_vid.vid())
        .send()
        .await
        .inspect(|r| debug!("DID server responded with status code {}", r.status()))
        .expect("Could not publish VID on server");

    let _: Vid = match response.status() {
        r if r.is_success() => response.json().await.expect("Could not decode VID"),
        _ => {
            error!("An error occurred while publishing the DID. Maybe this DID exists already?");
            error!("Response: {}", response.text().await.unwrap());
            return Err(Error::Vid(VidError::InvalidVid(
                "An error occurred while publishing the DID. Maybe this DID exists already?"
                    .to_string(),
            )));
        }
    };
    info!(
        "published DID document at {}",
        tsp_sdk::vid::did::get_resolve_url(&did)?.to_string()
    );

    Ok(private_vid)
}
