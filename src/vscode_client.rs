use serde::{Deserialize, Serialize};
use tokio_tungstenite::{connect_async, tungstenite::Message};
use futures_util::StreamExt;
use url::Url;
use std::time::Duration;
use tokio::time;

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

/// Connect to VS Code once and get the current file info
pub async fn connect_to_vscode_once(port: u16) -> Result<FileInfo, Box<dyn std::error::Error>> {
    let url = format!("ws://localhost:{}", port);
    let url = Url::parse(&url)?;

    let (ws_stream, _) = connect_async(&url).await?;
    let (_, mut receiver) = ws_stream.split();

    // Wait for the first message
    if let Some(msg) = receiver.next().await {
        match msg? {
            Message::Text(text) => {
                let file_info = serde_json::from_str::<FileInfo>(&text)?;
                return Ok(file_info);
            }
            _ => {}
        }
    }

    Err("No file info received".into())
}

pub async fn connect_to_vscode(port: u16) -> Result<(), Box<dyn std::error::Error>> {
    let url = format!("ws://localhost:{}", port);
    let url = Url::parse(&url)?;

    println!("Attempting to connect to VS Code WebSocket server at {}", url);
    
    // Try to connect with retries
    let mut retries = 0;
    let max_retries = 5;
    
    while retries < max_retries {
        match connect_async(&url).await {
            Ok((ws_stream, _)) => {
                println!("Successfully connected to VS Code WebSocket server");
                let (_, mut receiver) = ws_stream.split();

                while let Some(msg) = receiver.next().await {
                    match msg? {
                        Message::Text(text) => {
                            match serde_json::from_str::<FileInfo>(&text) {
                                Ok(file_info) => {
                                    println!("\n=== File Information ===");
                                    println!("File: {}", file_info.file_name);
                                    println!("Extension: {}", file_info.extension);
                                    println!("Path: {}", file_info.full_path);
                                    println!("Language: {}", file_info.language_id);
                                    println!("Lines: {}", file_info.line_count);
                                    println!("Words: {}", file_info.word_count);
                                    println!("Timestamp: {}", file_info.timestamp);
                                    println!("=====================\n");
                                }
                                Err(e) => {
                                    eprintln!("Failed to parse file info: {}", e);
                                    eprintln!("Raw message: {}", text);
                                }
                            }
                        }
                        Message::Close(frame) => {
                            println!("Connection closed: {:?}", frame);
                            return Ok(());
                        }
                        Message::Ping(data) => {
                            println!("Received ping: {:?}", data);
                        }
                        Message::Pong(data) => {
                            println!("Received pong: {:?}", data);
                        }
                        _ => {}
                    }
                }
                return Ok(());
            }
            Err(e) => {
                retries += 1;
                if retries < max_retries {
                    println!("Connection attempt {} failed: {}. Retrying in 2 seconds...", retries, e);
                    time::sleep(Duration::from_secs(2)).await;
                } else {
                    return Err(format!("Failed to connect after {} attempts: {}", max_retries, e).into());
                }
            }
        }
    }

    Err("Failed to connect to VS Code WebSocket server".into())
} 