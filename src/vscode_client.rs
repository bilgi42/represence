use serde::{Deserialize, Serialize};
use tokio_tungstenite::{connect_async, tungstenite::Message};
use futures_util::StreamExt;
use std::time::Duration;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FileInfo {
    #[serde(rename = "fileName")]
    pub file_name: String,
    pub extension: String,
    #[serde(rename = "fullPath")]
    pub full_path: String,
    #[serde(rename = "languageId")]
    pub language_id: String,
    #[serde(rename = "lineCount")]
    pub line_count: u32,
    #[serde(rename = "wordCount")]
    pub word_count: u32,
    pub timestamp: u64,
}

/// Connect to VS Code once and get the current file info (optimized)
pub async fn connect_to_vscode_once(port: u16) -> Result<FileInfo, Box<dyn std::error::Error>> {
    let url = format!("ws://localhost:{}", port);

    // Set connection timeout - pass the string directly instead of parsing to URL
    let connect_future = connect_async(&url);
    let (ws_stream, _) = tokio::time::timeout(Duration::from_secs(3), connect_future).await??;
    
    let (_, mut receiver) = ws_stream.split();

    // Wait for the first message with timeout
    let message_future = receiver.next();
    if let Some(msg) = tokio::time::timeout(Duration::from_secs(2), message_future).await? {
        match msg? {
            Message::Text(text) => {
                let file_info = serde_json::from_str::<FileInfo>(&text)?;
                return Ok(file_info);
            }
            _ => return Err("Unexpected message type".into()),
        }
    }

    Err("No file info received within timeout".into())
}

 