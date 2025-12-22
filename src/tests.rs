use tsp_sdk::{AskarSecureStorage, SecureStorage, SecureStore};

use crate::{TmcpClient, settings};

#[tokio::test]
async fn test_tmcp_client() {
    let other_did = "did:webvh:QmTzEvHuLRS1vrkThAxSiovo2CsCz5T1oNBdyRDLEwh6yi:did.teaspoon.world:endpoint:tmcp-83a1d6ab-3130-444b-90a5-9687b45e25fb";
    let client = TmcpClient::new("pigeon", other_did, settings::TmcpSettings::default())
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
