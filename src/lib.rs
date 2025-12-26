//! # TMCP - Transport Extensions for Model Context Protocol
//!
//! This crate provides transport layer extensions and utilities for the Model Context Protocol (MCP).
//! It builds upon the core `rmcp` crate to offer additional transport mechanisms and helpers.
//!

use std::sync::Arc;

use crate::create::create;
use errors::TmcpError;
use futures::{StreamExt, stream::BoxStream};
use http::header::CONTENT_TYPE;
use reqwest::header::ACCEPT;
use rmcp::model::ServerJsonRpcMessage;
use rmcp::transport::common::http_header::{EVENT_STREAM_MIME_TYPE, HEADER_LAST_EVENT_ID, HEADER_SESSION_ID, JSON_MIME_TYPE};
use rmcp::transport::streamable_http_client::SseError;
use rmcp::{
    model::ClientJsonRpcMessage,
    transport::streamable_http_client::{
        StreamableHttpClient, StreamableHttpError, StreamableHttpPostResponse,
    },
};
use sse_stream::{Sse, SseStream};
use tsp_sdk::{AskarSecureStorage, AsyncSecureStore, SecureStorage, VerifiedVid};
use uuid::Uuid;
mod create;
pub mod errors;
mod get;
mod tsp_messages;
pub mod settings;
#[cfg(test)]
mod tests;
mod verify;

#[derive(Clone)]
pub struct TmcpClient {
    inner: reqwest::Client,
    my_did: String,
    other_did: String,
    wallet: AsyncSecureStore,
    settings: settings::TmcpSettings,
}

impl TmcpClient {
    pub async fn new(alias: &str, other_did: &str, settings: settings::TmcpSettings) -> Result<Self, TmcpError> {
        let wallet_alias = if settings.use_webvh {
            format!("{}vh", alias)
        } else {
            alias.to_string()
        };
        println!("settings.wallet_url: {}", settings.wallet_url);
        // TODO: Create AskarSecureStorage if not exists
        let storage =
            AskarSecureStorage::open(&settings.wallet_url, settings.wallet_password.as_bytes())
                .await;
        println!("after open storage");
        let storage = match storage {
            Err(e) => {
                println!("unable to open storage {:?}", e);
                AskarSecureStorage::new(&settings.wallet_url, &settings.wallet_password.as_bytes())
                    .await
            }
            _ => storage,
        };
        let storage = storage?;
        let (vids, aliases, keys) = storage.read().await?;
        let mut wallet = AsyncSecureStore::new();
        wallet.import(vids, aliases, keys)?;
        
        println!("wallet_alias: {}", wallet_alias);
        let mut my_did: Option<String> = wallet.resolve_alias(&wallet_alias)?;
        let did_server = settings.did_server.to_string();
        let client = reqwest::Client::new();
        if let Some(my_did) = &my_did {
            //Resolve and verify public key material for a VID identified by vid and add it to the wallet as a relationship
            verify::verify_did(&my_did, &wallet, None).await?;
        } else {
            let address = settings.did_server.to_string();
            let username = format!("{}-{}", alias, Uuid::new_v4());
            let published_did = match get::get_did_doc(&client, &did_server, &username).await {
                Ok(published_did) => Some(published_did),
                Err(e) => {
                    println!("get_did_doc error: {}", e);
                    None
                }
            };
            println!("published_did: {:?}", published_did);
            if let Some(published_did) = published_did {
                my_did = Some(published_did.clone());
                verify::verify_did(&published_did, &wallet, None).await?;
            } else {
                let private_vid = create(
                    Some(&settings.did_server),
                    &address,
                    Some(alias),
                    &mut wallet,
                    &settings.did_type,
                    &client,
                    &did_server,
                )
                .await?;
                my_did = Some(private_vid.identifier().to_string());
                let meta_data = verify::verify_did(&private_vid.identifier().to_string(), &wallet, None).await?;
                wallet.add_private_vid(private_vid, meta_data)?;   
            }
            verify::verify_did(&other_did.to_string(), &wallet, None).await?;
            let v = wallet.export()?;
            let (vids, aliases, keys) = v.clone();
            storage.persist(v).await?;
        }
        let my_did = my_did.unwrap_or_default();
        // TODO: Verify vid
        // TODO: Create vid if not exists
        // For now, copy the wallet.sqlite from the tmcp-python's client.
        //verify::verify_did(&my_did, &wallet, Some(wallet_alias)).await?;
        Ok(Self {
            inner: reqwest::Client::new(),
            my_did,
            other_did: other_did.to_string(),
            wallet,
            settings,
        })
    }

    /// Handle HTTP response and apply TSP transformations
    async fn handle_response(
        &self,
        response: reqwest::Response,
    ) -> Result<StreamableHttpPostResponse, StreamableHttpError<TmcpError>> {
        use http::header::WWW_AUTHENTICATE;
        use rmcp::transport::common::http_header::{
            EVENT_STREAM_MIME_TYPE, HEADER_SESSION_ID, JSON_MIME_TYPE,
        };
        use std::borrow::Cow;

        if response.status() == reqwest::StatusCode::UNAUTHORIZED {
            if let Some(header) = response.headers().get(WWW_AUTHENTICATE) {
                let header = header
                    .to_str()
                    .map_err(|_| {
                        StreamableHttpError::UnexpectedServerResponse(Cow::from(
                            "invalid www-authenticate header value",
                        ))
                    })?
                    .to_string();
                return Err(StreamableHttpError::AuthRequired(
                    rmcp::transport::streamable_http_client::AuthRequiredError {
                        www_authenticate_header: header,
                    },
                ));
            }
        }

        let status = response.status();
        let response = response
            .error_for_status()
            .map_err(|e| StreamableHttpError::Client(TmcpError::Reqwest(e)))?;

        if matches!(
            status,
            reqwest::StatusCode::ACCEPTED | reqwest::StatusCode::NO_CONTENT
        ) {
            return Ok(StreamableHttpPostResponse::Accepted);
        }

        let content_type = response.headers().get(reqwest::header::CONTENT_TYPE);
        let session_id = response.headers().get(HEADER_SESSION_ID);
        let session_id = session_id
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string());

        match content_type {
            Some(ct) if ct.as_bytes().starts_with(EVENT_STREAM_MIME_TYPE.as_bytes()) => {
                let event_stream = SseStream::from_byte_stream(response.bytes_stream());
                // Apply TSP open_message transformation to SSE stream if available
                let wallet_clone = self.wallet.clone();
                let wrapped_stream = event_stream.map(move |result| {
                    result.map(|mut sse| {
                        let Some(data) = sse.data.take() else {
                            return sse;
                        };
                        let processed_data = tsp_messages::open_message(data, &wallet_clone);
                        match processed_data {
                            Ok(processed_data) => {
                                sse.data = Some(processed_data);
                            }
                            Err(e) => {
                                log::error!("failed to open message: {}", e);
                            }
                        }
                        return sse;
                    })
                });
                Ok(StreamableHttpPostResponse::Sse(
                    wrapped_stream.boxed(),
                    session_id,
                ))
            }
            Some(ct) if ct.as_bytes().starts_with(JSON_MIME_TYPE.as_bytes()) => {
                let body = response
                    .text()
                    .await
                    .map_err(|e| StreamableHttpError::Client(TmcpError::Reqwest(e)))?;

                // Apply TSP open_message transformation if available
                let processed_body = tsp_messages::open_message(body, &self.wallet.clone())
                    .map_err(|e| StreamableHttpError::Client(e))?;

                let message: ServerJsonRpcMessage = serde_json::from_str(&processed_body)
                    .map_err(|e| StreamableHttpError::Deserialize(e))?;
                Ok(StreamableHttpPostResponse::Json(message, session_id))
            }
            _ => {
                log::error!("unexpected content type: {:?}", content_type);
                Err(StreamableHttpError::UnexpectedContentType(
                    content_type.map(|ct| String::from_utf8_lossy(ct.as_bytes()).to_string()),
                ))
            }
        }
    }

    pub fn create_transport(
        &self,
        uri: impl Into<Arc<str>>,
    ) -> rmcp::transport::StreamableHttpClientTransport<Self> {
        use rmcp::transport::streamable_http_client::{
            StreamableHttpClientTransport, StreamableHttpClientTransportConfig,
        };
        let config = StreamableHttpClientTransportConfig::with_uri(format!("{}?did={}", uri.into(), self.my_did));
    
        StreamableHttpClientTransport::with_client(self.clone(), config)
    }
}

impl StreamableHttpClient for TmcpClient {
    type Error = TmcpError;

    async fn post_message(
        &self,
        uri: Arc<str>,
        message: ClientJsonRpcMessage,
        session_id: Option<Arc<str>>,
        auth_token: Option<String>,
    ) -> Result<StreamableHttpPostResponse, StreamableHttpError<Self::Error>> {
        // Apply TSP seal_message transformation if transport hook is available
        let message_to_send = {
            let json_str =
                serde_json::to_string(&message).map_err(|e| StreamableHttpError::Deserialize(e))?;
            // Use the transport hook to seal the message
            let sealed_data = tsp_messages::seal_message(json_str, &self.wallet, &self.my_did, &self.other_did);
            let sealed_data = match sealed_data {
                Ok(s) => s,
                Err(e) => {
                    log::error!("failed to seal message: {}", e);
                    "".to_string()
                }
            };
            // Try to parse the sealed data back to a message, or use raw body
            match serde_json::from_str::<ClientJsonRpcMessage>(&sealed_data) {
                Ok(sealed_message) => sealed_message,
                Err(_) => {
                    // If sealed data is not valid JSON, we need to send it as raw body
                    let mut request = self
                        .inner
                        .post(uri.as_ref())
                        .header(ACCEPT, [EVENT_STREAM_MIME_TYPE, JSON_MIME_TYPE].join(", "))
                        .header(CONTENT_TYPE, JSON_MIME_TYPE)
                        .body(sealed_data);

                    if let Some(auth_header) = auth_token {
                        request = request.bearer_auth(auth_header);
                    }
                    if let Some(session_id) = session_id {
                        request = request.header(HEADER_SESSION_ID, session_id.as_ref());
                    }

                    let response = request
                        .send()
                        .await
                        .map_err(|e| StreamableHttpError::Client(TmcpError::Reqwest(e)))?;

                    return self.handle_response(response).await;
                }
            }
        };

        // Standard JSON request path
        let mut request = self
            .inner
            .post(uri.as_ref())
            .header(ACCEPT, [EVENT_STREAM_MIME_TYPE, JSON_MIME_TYPE].join(", "));

        if let Some(auth_header) = auth_token {
            request = request.bearer_auth(auth_header);
        }
        if let Some(session_id) = session_id {
            request = request.header(HEADER_SESSION_ID, session_id.as_ref());
        }

        let response = request
            .json(&message_to_send)
            .send()
            .await
            .map_err(|e| StreamableHttpError::Client(TmcpError::Reqwest(e)))?;

        self.handle_response(response).await
    }

    /// Get SSE stream from the server
    async fn get_stream(
        &self,
        uri: Arc<str>,
        session_id: Arc<str>,
        last_event_id: Option<String>,
        auth_token: Option<String>,
    ) -> Result<BoxStream<'static, Result<Sse, SseError>>, StreamableHttpError<Self::Error>> {
        let mut request_builder = self.inner
            .get(uri.as_ref())
            .header(ACCEPT, [EVENT_STREAM_MIME_TYPE, JSON_MIME_TYPE].join(", "))
            .header(HEADER_SESSION_ID, session_id.as_ref());
        if let Some(last_event_id) = last_event_id {
            request_builder = request_builder.header(HEADER_LAST_EVENT_ID, last_event_id);
        }
        if let Some(auth_header) = auth_token {
            request_builder = request_builder.bearer_auth(auth_header);
        }
        let response = request_builder.send().await
            .map_err(|e| StreamableHttpError::Client(TmcpError::Reqwest(e)))?;
        if response.status() == reqwest::StatusCode::METHOD_NOT_ALLOWED {
            return Err(StreamableHttpError::ServerDoesNotSupportSse);
        }
        let response = response.error_for_status()
            .map_err(|e| StreamableHttpError::Client(TmcpError::Reqwest(e)))?;
        match response.headers().get(reqwest::header::CONTENT_TYPE) {
            Some(ct) => {
                if !ct.as_bytes().starts_with(EVENT_STREAM_MIME_TYPE.as_bytes())
                    && !ct.as_bytes().starts_with(JSON_MIME_TYPE.as_bytes())
                {
                    return Err(StreamableHttpError::UnexpectedContentType(Some(
                        String::from_utf8_lossy(ct.as_bytes()).to_string(),
                    )));
                }
            }
            None => {
                return Err(StreamableHttpError::UnexpectedContentType(None));
            }
        }
        let event_stream = SseStream::from_byte_stream(response.bytes_stream()).boxed();
        let wallet_clone = self.wallet.clone();
        let wrapped_stream = event_stream.map(move |result| {
            result.map(|mut sse| {
                if let Some(data) = sse.data.take() {
                    let processed_data = tsp_messages::open_message(data, &wallet_clone);
                    match processed_data {
                        Ok(processed_data) => {
                            sse.data = Some(processed_data);
                        }
                        Err(e) => {
                            log::error!("failed to open message: {}", e);
                        }
                    }
                }
                sse
            })
        });
        Ok(wrapped_stream.boxed())
    }

    async fn delete_session(
        &self,
        uri: Arc<str>,
        session_id: Arc<str>,
        auth_token: Option<String>,
    ) -> Result<(), StreamableHttpError<Self::Error>> {
        let mut request_builder = self.inner.delete(uri.as_ref());
        if let Some(auth_header) = auth_token {
            request_builder = request_builder.bearer_auth(auth_header);
        }
        let response = request_builder
            .header(HEADER_SESSION_ID, session_id.as_ref())
            .send()
            .await
            .map_err(|e| StreamableHttpError::Client(TmcpError::Reqwest(e)))?;

        if response.status() == reqwest::StatusCode::METHOD_NOT_ALLOWED {
            log::debug!("this server doesn't support deleting session");
            return Ok(());
        }
        let _response = response
            .error_for_status()
            .map_err(|e| StreamableHttpError::Client(TmcpError::Reqwest(e)))?;
        Ok(())
    }
}
