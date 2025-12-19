use serde::{Deserialize, Serialize};
#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub enum DidType {
    #[default]
    Web,
    Peer,
    Webvh,
}
/// TMCP general settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TmcpSettings {
    /// DID publish URL
    pub did_publish_url: String,
    /// DID publish history URL template
    pub did_publish_history_url: String,
    /// DID web format template
    pub did_web_format: String,
    /// DID webvh format template
    pub did_webvh_format: String,
    /// Transport protocol prefix
    pub transport: String,
    /// Whether TSP messages should be printed
    pub verbose: bool,
    /// Wallet URL for secure storage
    pub wallet_url: String,
    /// Wallet password
    pub wallet_password: String,
    /// Whether to use webvh DIDs
    pub use_webvh: bool,
    /// DID server address
    pub did_server: String,
    /// Type of DID to create
    pub did_type: DidType,
}

impl Default for TmcpSettings {
    /*************  ✨ Windsurf Command ⭐  *************/
    /// Returns a default TmcpSettings configuration with the following settings:
    ///
    /// * did_publish_url: https://did.teaspoon.world/add-vid
    /// * did_publish_history_url: https://did.teaspoon.world/add-history/{did}
    /// * did_web_format: did:web:did.teaspoon.world:endpoint:{name}
    /// * did_webvh_format: did.teaspoon.world/endpoint/{name}
    /// * transport: tmcp://
    /// * verbose: true
    /// * wallet_url: sqlite://wallet.sqlite
    /// * wallet_password: unsecure
    /// * use_webvh: true
    /*******  e8082e7b-9eab-4f44-a17e-6a86ab880b84  *******/
    fn default() -> Self {
        Self {
            did_publish_url: "https://did.teaspoon.world/add-vid".to_string(),
            did_publish_history_url: "https://did.teaspoon.world/add-history/{did}".to_string(),
            did_web_format: "did:web:did.teaspoon.world:endpoint:{name}".to_string(),
            did_webvh_format: "did.teaspoon.world/endpoint/{name}".to_string(),
            transport: "tmcp://".to_string(),
            verbose: true,
            wallet_url: "sqlite://wallet.sqlite".to_string(),
            wallet_password: "unsecure".to_string(),
            use_webvh: true,
            did_server: "did.teaspoon.world".to_string(),
            did_type: DidType::Webvh,
        }
    }
}
