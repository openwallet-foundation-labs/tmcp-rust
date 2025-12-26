use serde_json::Value;
use tsp_sdk::{AsyncSecureStore, Error, VerifiedVid};

/// Resolve and verify public key material for a VID identified by vid and add it to the wallet as a relationship
///
/// This function will verify the DID document and store it in the wallet with the given alias.
///
/// # Parameters
///
/// * vid - The DID document to verify and store.
/// * wallet - The wallet in which to store the DID document.
/// * alias - The alias with which to store the DID document in the wallet.
///
/// # Returns
///
/// A Result containing Ok if the DID document was successfully verified and stored, otherwise an error.
pub async fn verify_did(
    did: &String,
    wallet: &AsyncSecureStore,
    alias: Option<String>,
) -> Result<Option<Value>, Error> {
    //Resolve and verify the vid identified by id, by using online and offline methods
    let (vid, metadata) = tsp_sdk::vid::verify_vid(did).await?;
    //Resolve and verify public key material for a VID identified by vid and add it to the wallet as a relationship
    wallet.verify_vid(&vid.identifier(), alias).await?;
    Ok(metadata)
}
