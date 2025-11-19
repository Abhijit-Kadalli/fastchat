use anyhow::Result;
use std::fs;
use std::path::PathBuf;
use crate::types::ChatSession;

pub fn get_history_path() -> PathBuf {
    let mut path = dirs::config_dir().unwrap_or_else(|| PathBuf::from("."));
    path.push("fastchat-tui");
    if let Err(e) = fs::create_dir_all(&path.parent().unwrap().join("fastchat-tui")) {
         // If we can't create the dir, just use current dir as fallback or ignore if it exists
         // Actually create_dir_all should be fine.
         // Let's just be safe and create the dir.
    }
    // Ensure directory exists
    if !path.exists() {
        let _ = fs::create_dir_all(&path);
    }
    path.push("history.json");
    path
}

pub fn save_chats(chats: &[ChatSession]) -> Result<()> {
    let path = get_history_path();
    let json = serde_json::to_string_pretty(chats)?;
    fs::write(path, json)?;
    Ok(())
}

pub fn load_chats() -> Result<Vec<ChatSession>> {
    let path = get_history_path();
    if !path.exists() {
        return Ok(Vec::new());
    }
    let content = fs::read_to_string(path)?;
    // Handle empty file
    if content.trim().is_empty() {
        return Ok(Vec::new());
    }
    let chats: Vec<ChatSession> = serde_json::from_str(&content)?;
    Ok(chats)
}
