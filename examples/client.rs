
use std::env;
use std::io::{self, Write};

use anthropic_sdk::{Anthropic, ContentBlock, ContentBlockParam, MessageContent, MessageCreateBuilder, Role, Tool};
use rmcp::model::{CallToolRequestParam, ClientCapabilities, ClientInfo, Implementation};
use rmcp::{RoleClient, ServiceExt, service::RunningService};
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
    #[error("Antrophic error: {0}")]
    AntrophicError(#[from] anthropic_sdk::AnthropicError),
}

/// TMCP Client implementation
/// Based on the Python client from the MCP quickstart resources
pub struct TmcpChatClient {
    name: String,
    client: Option<RunningService<RoleClient, ClientInfo>>,
    antropic_client: anthropic_sdk::Anthropic,
    available_tools: Vec<Tool>,
}

impl TmcpChatClient {
    /// Create a new TMCP client with the given name
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            client: None,
            antropic_client: Anthropic::from_env().unwrap(),
            available_tools: vec![],
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

        let tools_response = client.list_tools(None).await?;
        if tools_response.tools.is_empty() {
            return Err(TmcpChatClientError::Client("No tools available on this server".to_string()));
        }
        let mut available_tools = vec![];
        tools_response.tools.iter().for_each(|tool| {
            let input_schema = tool.input_schema.clone().as_ref().to_owned();
            if let Ok(input_schema) = serde_json::from_value(serde_json::Value::Object(input_schema)) {
                available_tools.push(Tool {
                    name: tool.name.to_string(),
                    description: tool.description.clone().unwrap_or_default().as_ref().to_string(),
                    input_schema: input_schema
                });
            }
        });
        self.available_tools = available_tools.clone();
        let response = self.antropic_client.messages().create(
        MessageCreateBuilder::new("claude-3-5-haiku-20241022", 1000)
            .user("Hello, Claude!")
            .tools(available_tools)
            .message(Role::User, MessageContent::Text(query.to_string()))
            .build()
        ).await?;
        let mut assistant_message_content = Vec::new();
        for content in response.content {
           let _ = self.handle_response_content(content, &mut assistant_message_content).await;
        }
        println!("\nProcessing query: {}", query);
        Ok(())
    }

    pub async fn handle_response_content(&self, content: ContentBlock, assistant_message_content: &mut Vec<ContentBlockParam>) -> Result<(), TmcpChatClientError> {
        let Some(client) = &self.client else { return Ok(())};
        match content {
            ContentBlock::Text { text } => {
                println!("{}", text);
                assistant_message_content.push(ContentBlockParam::Text{ text: text.clone() });
            }
            ContentBlock::Image { source: _ } => {},
            ContentBlock::ToolUse { id, name, input } => {
                let tool_request = CallToolRequestParam {
                    name: name.clone().into(),
                    arguments: Some(input.as_object().unwrap().clone()),
                };
                let result = client.call_tool(tool_request).await?;
                log::info!("Calling tool {name} with args {input}");
                println!("{}", result.content.first().and_then(|c| c.as_text()).and_then(|f| Some(f.text.clone())).unwrap_or_default());
                assistant_message_content.push(ContentBlockParam::ToolUse { id , name, input });
                let response = self.antropic_client.messages().create(
                    MessageCreateBuilder::new("claude-3-5-haiku-20241022", 1000)
                        .user("Hello, Claude!")
                        .tools(self.available_tools.clone())
                        .message(Role::Assistant, MessageContent::Blocks(assistant_message_content.clone()))
                        .build()
                    ).await?;
                for content in response.content.into_iter().skip(1) {
                    let _ = Box::pin(self.handle_response_content(content, assistant_message_content)).await;
                }
            }
            ContentBlock::ToolResult { tool_use_id: _ , content: _, is_error: _ } => {

            }
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
        log::info!("Usage: {} <server_url> <other_did>", args[0]);
        log::info!("Example: {} http://localhost:8000/mcp did:web:example.com:user", args[0]);
        std::process::exit(1);
    }
    
    let server_url = &args[1];
    let other_did = &args[2];
    let mut tmcp_client = TmcpClient::new("tmcp", other_did, TmcpSettings{
        wallet_url: "sqlite://./wallets/wallet.sqlite".to_string(), ..Default::default()
    }).await?;
    let mut chat_client = TmcpChatClient::new("tmcp");
    match chat_client.connect_to_server(server_url, &mut tmcp_client).await {
        Ok(()) => {
            if let Err(e) = chat_client.chat_loop().await {
                log::error!("Error in chat loop: {}", e);
            }
        }
        Err(e) => {
            log::error!("Failed to connect to server: {}", e);
            std::process::exit(1);
        }
    }

    // Cleanup
    if let Err(e) = chat_client.cleanup().await {
        log::error!("Error during cleanup: {}", e);
    }

    Ok(())
}
