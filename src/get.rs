use reqwest::Client;

pub async fn get_did_doc(
    client: &Client,
    did_server: &str,
    alias: &str,
) -> Result<bool, reqwest::Error> {
    match client
        .get(format!("https://{did_server}/endpoint/{alias}/did.json"))
        .send()
        .await?
    {
        resp if resp.status().is_success() => {
            let _did_doc = resp.text().await?;
            // Process the DID document as needed
            return Ok(true);
        }
        resp => {
            eprintln!("Failed to retrieve DID document: HTTP {}", resp.status());
            return Ok(false);
        }
    }
}
