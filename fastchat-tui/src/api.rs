use crate::types::{Message, Role};
use crate::config::AppConfig;
use anyhow::Result;
use futures::StreamExt;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::json;
use tokio::sync::mpsc;

#[derive(Debug)]
pub enum ApiEvent {
    Token(String),
    Done,
    Error(String),
}

#[derive(Serialize)]
struct ApiMessage {
    role: String,
    content: String,
}

#[derive(Deserialize)]
struct ChatCompletionChunk {
    choices: Vec<Choice>,
}

#[derive(Deserialize)]
struct Choice {
    delta: Delta,
}

#[derive(Deserialize)]
struct Delta {
    content: Option<String>,
}

pub async fn send_message_stream(
    config: AppConfig,
    history: Vec<Message>,
    tx: mpsc::Sender<ApiEvent>,
) -> Result<()> {
    let backend = config.get_active_backend().ok_or_else(|| anyhow::anyhow!("No active backend"))?;
    
    let client = Client::new();
    
    // Convert history to API messages, excluding the last empty assistant message we just added
    let api_messages: Vec<ApiMessage> = history
        .iter()
        .filter(|m| !m.content.is_empty()) // Filter out empty messages (like the new assistant one)
        .map(|m| ApiMessage {
            role: match m.role {
                Role::User => "user".to_string(),
                Role::Assistant => "assistant".to_string(),
                Role::System => "system".to_string(),
            },
            content: m.content.clone(),
        })
        .collect();

    let url = format!("{}/chat/completions", backend.url.trim_end_matches('/'));
    
    let body = json!({
        "model": backend.model,
        "messages": api_messages,
        "stream": true,
        "temperature": 0.7,
    });

    let mut stream = client
        .post(url)
        .json(&body)
        .send()
        .await?
        .bytes_stream();

    while let Some(item) = stream.next().await {
        match item {
            Ok(bytes) => {
                let chunk_str = String::from_utf8_lossy(&bytes);
                for line in chunk_str.lines() {
                    if line.starts_with("data: ") {
                        let data = &line[6..];
                        if data == "[DONE]" {
                            let _ = tx.send(ApiEvent::Done).await;
                            return Ok(());
                        }
                        
                        if let Ok(chunk) = serde_json::from_str::<ChatCompletionChunk>(data) {
                            if let Some(choice) = chunk.choices.first() {
                                if let Some(content) = &choice.delta.content {
                                    let _ = tx.send(ApiEvent::Token(content.clone())).await;
                                }
                            }
                        }
                    }
                }
            }
            Err(e) => {
                let _ = tx.send(ApiEvent::Error(e.to_string())).await;
            }
        }
    }
    
    let _ = tx.send(ApiEvent::Done).await;
    Ok(())
}
