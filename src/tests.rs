use tsp_sdk::{AskarSecureStorage, SecureStorage, SecureStore};

use crate::{TmcpClient, settings};

#[tokio::test]
async fn test_tmcp_client() {
    let client = TmcpClient::new("pigeon".to_string(), settings::TmcpSettings::default())
        .await
        .unwrap();

    {
        let vault = AskarSecureStorage::open("sqlite://wallet.sqlite", b"unsecure")
            .await
            .unwrap();
        let (vids, aliases, keys) = vault.read().await.unwrap();

        assert_eq!(
            aliases.get("pigeon"),
            Some(&"did:web:did.teaspoon.world:endpoint:pigeon".to_string())
        );

        let store = SecureStore::new();
        store.import(vids, aliases, keys).unwrap();
        assert!(store.has_private_vid(&client.my_did).unwrap());

        vault.destroy().await.unwrap();
    }
}
