use std::env;
use std::io::{self, Write};

use rmcp::model::{CallToolRequestParam, ClientCapabilities, ClientInfo, Implementation};
use rmcp::{RoleClient, ServiceExt, service::RunningService};
use serde_json::json;
use tmcp_rs::TmcpClient;
use tmcp_rs::errors::TmcpError;
use tmcp_rs::settings::TmcpSettings;
use tokio::io::{AsyncBufReadExt, BufReader};

#[derive(Debug, thiserror::Error)]
pub enum TmcpChatClientError {
    /// Error from reqwest HTTP client
    #[error("Tmcp error: {0}")]
    Tmcp(#[from] TmcpError),
    #[error("TmcpChatClient error: {0}")]
    Client(String),
    #[error("IO error: {0}")]
    IoError(#[from] io::Error),
    #[error("Client service error: {0}")]
    RmcpClientInitializeError(#[from] rmcp::service::ServiceError),
}

/// TMCP Client implementation
/// Based on the Python client from the MCP quickstart resources
pub struct TmcpChatClient {
    name: String,
    client: Option<RunningService<RoleClient, ClientInfo>>,
}

impl TmcpChatClient {
    /// Create a new TMCP client with the given name
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            client: None,
        }
    }

    /// Connect to an MCP server
    pub async fn connect_to_server(&mut self, url_or_id: &str, tmcp_client: &TmcpClient) -> Result<(), TmcpError> {
        println!("Server endpoint: {}", url_or_id);

        // Create streamable HTTP transport
        let transport = tmcp_client.create_transport(url_or_id);
        println!("after transport");
        // Create client info
        let client_info = ClientInfo {
            protocol_version: Default::default(),
            capabilities: ClientCapabilities::default(),
            client_info: Implementation {
                name: self.name.clone(),
                version: "1.0.0".to_string(),
                title: Some("TMCP Demo Client".to_string()),
                website_url: None,
                icons: None,
            },
        };

        // Connect to server
        let client = client_info.serve(transport).await?;

        // Get server info
        let server_info = client.peer_info();
        println!(
            "Connected to server: {:?}",
            server_info.map(|info| &info.server_info.name)
        );

        // List available tools
        let tools_response = client.list_tools(None).await?;
        let tool_names: Vec<String> = tools_response
            .tools
            .iter()
            .map(|tool| tool.name.to_string())
            .collect();

        println!("Available tools: {:?}", tool_names);

        self.client = Some(client);

        Ok(())
    }

    /// Process a query using available tools (simplified)
    pub async fn process_query(&mut self, query: &str) -> Result<(), TmcpChatClientError> {
        let client = self
            .client
            .as_ref()
            .ok_or_else(|| TmcpChatClientError::Client("Not connected to server".to_string()))?;

        println!("\nProcessing query: {}", query);

        // Get available tools
        let tools_response = client.list_tools(None).await?;

        if !tools_response.tools.is_empty() {
            let first_tool = &tools_response.tools[0];
            println!("Calling tool: {}", first_tool.name);

            // Try to call the first available tool with empty arguments as a demo
            let tool_request = CallToolRequestParam {
                name: first_tool.name.clone(),
                arguments: Some(json!({"query": query}).as_object().unwrap().clone()),
            };

            match client.call_tool(tool_request).await {
                Ok(result) => {
                    println!("Tool result: {:?}", result);
                }
                Err(e) => {
                    println!("Tool call failed: {}", e);
                }
            }
        } else {
            println!("No tools available on this server");
        }

        Ok(())
    }

    /// Run an interactive chat loop
    pub async fn chat_loop(&mut self) -> Result<(), TmcpChatClientError> {
        println!("\nTMCP Client Started!");
        println!("Type your queries or 'quit' to exit.");

        let stdin = tokio::io::stdin();
        let mut reader = BufReader::new(stdin);
        let mut line = String::new();

        loop {
            print!("\nQuery: ");
            io::stdout().flush()?;

            line.clear();
            match reader.read_line(&mut line).await {
                Ok(0) => break, // EOF
                Ok(_) => {
                    let query = line.trim();

                    if query.to_lowercase() == "quit" {
                        break;
                    }

                    if let Err(e) = self.process_query(query).await {
                        println!("Error processing query: {}", e);
                    }
                }
                Err(e) => {
                    println!("Error reading input: {}", e);
                    break;
                }
            }
        }

        Ok(())
    }

    /// Cleanup and disconnect
    pub async fn cleanup(&mut self) -> Result<(), TmcpChatClientError> {
        // Note: Proper cleanup would be handled by the transport layer
        // when the client is dropped. For explicit cleanup, we'd need
        // access to the transport's close() method
        self.client = None;
        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<(), TmcpChatClientError> {
    env_logger::init();
    
    // Parse command line arguments
    let args: Vec<String> = env::args().collect();
    if args.len() < 3 {
        eprintln!("Usage: {} <server_url> <other_did>", args[0]);
        eprintln!("Example: {} http://localhost:8000/mcp did:web:example.com:user", args[0]);
        std::process::exit(1);
    }
    
    let server_url = &args[1];
    let other_did = &args[2];
    let mut tmcp_client = TmcpClient::new("tmcp", other_did, TmcpSettings::default()).await?;
    let mut chat_client = TmcpChatClient::new("tmcp");
    match chat_client.connect_to_server(server_url, &mut tmcp_client).await {
        Ok(()) => {
            if let Err(e) = chat_client.chat_loop().await {
                eprintln!("Error in chat loop: {}", e);
            }
        }
        Err(e) => {
            eprintln!("Failed to connect to server: {}", e);
            std::process::exit(1);
        }
    }

    // Cleanup
    if let Err(e) = chat_client.cleanup().await {
        eprintln!("Error during cleanup: {}", e);
    }

    Ok(())
}
