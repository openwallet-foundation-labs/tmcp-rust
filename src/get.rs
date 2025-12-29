use reqwest::Client;
use tsp_sdk::vid::did::web::DidDocument;
use crate::errors;
/// Retrieves a DID document from the server and processes it as needed.
///
/// # Parameters
///
/// * client - The HTTP client used to make the request.
/// * did_server - The URL of the server from which to retrieve the DID document.
/// * user - The username of the user whose DID document to retrieve.
///
/// # Returns
///
/// A Result containing Ok(true) if the DID document was successfully retrieved and processed, otherwise an error.
pub async fn get_did_doc(
    client: &Client,
    did_server: &str,
    user: &str,
) -> Result<String, errors::TmcpError> {
    match client
        .get(format!("https://{did_server}/endpoint/{user}/did.jsonl"))
        .send()
        .await?
    {
        resp if resp.status().is_success() => {
            let did_doc = resp.text().await?;
            // Process the DID document as needed
            let did_doc: serde_json::Value = serde_json::from_str(&did_doc)?;
            let Some(did_doc) = did_doc.get("state") else {
                return Err(errors::TmcpError::TspError(tsp_sdk::Error::DecodeState("State not found")));
            };

            let did_doc: DidDocument = serde_json::from_value(did_doc.clone())?;
            Ok(did_doc.id)
        }
        resp => {
            log::warn!("Failed to retrieve DID document: HTTP {}", resp.status());
            Err(errors::TmcpError::TspError(tsp_sdk::Error::DecodeState("Response error")))
        }
    }
}
