use tsp_sdk::{AsyncSecureStore, ReceivedTspMessage};
use base64::{engine::general_purpose, Engine as _};
use crate::errors::{self, TmcpError};

/// Open a TSP message using the given wallet.
///
/// The function takes a URL-safe base64 encoded string as input, decodes it, extracts the sender and receiver from the message, and then uses the wallet to open the message.
///
/// If the message is of type `ReceivedTspMessage::GenericMessage`, the function returns the decrypted message as a UTF-8 string. Otherwise, an error of type `TmcpError` is returned with the message "Unsupported TSP message type".
pub fn open_message(data: String, wallet: &AsyncSecureStore) -> Result<String, errors::TmcpError> {
    let mut data = general_purpose::URL_SAFE.decode(&data)?;
    let tsp_message = wallet.open_message(&mut data)?;
    if let ReceivedTspMessage::GenericMessage{
        message,..
    } = tsp_message {
        Ok(String::from_utf8(message.to_vec())?)
    } else {
        Err(TmcpError::TmcpError("Unsupported TSP message type".into()))
    }
}

/// Seal a message using the TSP SDK, returning the sealed message as a URL-safe base64-encoded string.
///
/// This function takes a raw message as a string, and seals it using the TSP SDK's `seal_message` function.
///
/// The sealed message is then encoded as a URL-safe base64 string using the `general_purpose::URL_SAFE` engine.
///
/// If the sealing process fails, an error of type `TmcpError` is returned.
///
/// # Arguments
///
/// * `data`: The raw message to be sealed, as a string.
/// * `wallet`: A reference to an `AsyncSecureStore` instance, used to perform the sealing operation.

pub fn seal_message(data: String, wallet: &AsyncSecureStore, my_did: &str, other_did: &str) -> Result<String, errors::TmcpError> {
    let (_url, data) = wallet.seal_message(&my_did, &other_did, None, &data.into_bytes())?;
    Ok(general_purpose::URL_SAFE.encode(&data))
}