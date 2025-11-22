use anyhow::{Result, Context};
use std::fs;
use std::path::PathBuf;
use crate::types::{ChatSession, Message, Role};
use chrono::{DateTime, Utc};

/// Get the chats directory path (~/.local/share/fastchat-tui/chats/)
pub fn get_chats_dir() -> PathBuf {
    let mut path = dirs::data_local_dir()
        .or_else(|| dirs::home_dir().map(|p| p.join(".local/share")))
        .unwrap_or_else(|| PathBuf::from("."));
    path.push("fastchat-tui");
    path.push("chats");
    path
}

/// Ensure the chats directory exists
fn ensure_chats_dir() -> Result<PathBuf> {
    let path = get_chats_dir();
    fs::create_dir_all(&path)
        .with_context(|| format!("Failed to create chats directory: {}", path.display()))?;
    Ok(path)
}

/// Sanitize a string to be used as a filename
fn sanitize_filename(name: &str) -> String {
    name.chars()
        .take(50)
        .map(|c| match c {
            'a'..='z' | 'A'..='Z' | '0'..='9' | '-' | '_' => c,
            ' ' => '-',
            _ => '_',
        })
        .collect::<String>()
        .trim_matches(|c| c == '-' || c == '_')
        .to_string()
}

/// Convert a ChatSession to markdown format
fn session_to_markdown(session: &ChatSession) -> String {
    let mut md = String::new();

    // Frontmatter with metadata
    md.push_str("---\n");
    md.push_str(&format!("id: {}\n", session.id));
    md.push_str(&format!("created: {}\n", session.created_at.to_rfc3339()));
    md.push_str(&format!("updated: {}\n", session.updated_at.to_rfc3339()));
    md.push_str("---\n\n");

    // Title
    md.push_str(&format!("# {}\n\n", session.name));

    // Messages
    for msg in &session.messages {
        match msg.role {
            Role::User => {
                md.push_str("## 👤 User\n\n");
                md.push_str(&msg.content);
                md.push_str("\n\n");
            }
            Role::Assistant => {
                md.push_str("## 🤖 Assistant\n\n");
                md.push_str(&msg.content);
                md.push_str("\n\n");
            }
            Role::System => {
                md.push_str("## 📢 System\n\n");
                md.push_str(&msg.content);
                md.push_str("\n\n");
            }
        }
    }

    md
}

/// Parse markdown back to ChatSession
fn markdown_to_session(content: &str, file_path: PathBuf) -> Result<ChatSession> {
    let mut lines = content.lines();

    // Parse frontmatter
    let mut id = String::new();
    let mut created_at = Utc::now();
    let mut updated_at = Utc::now();

    // Skip first ---
    if lines.next() == Some("---") {
        while let Some(line) = lines.next() {
            if line == "---" {
                break;
            }
            if let Some((key, value)) = line.split_once(':') {
                let key = key.trim();
                let value = value.trim();
                match key {
                    "id" => id = value.to_string(),
                    "created" => {
                        if let Ok(dt) = DateTime::parse_from_rfc3339(value) {
                            created_at = dt.with_timezone(&Utc);
                        }
                    }
                    "updated" => {
                        if let Ok(dt) = DateTime::parse_from_rfc3339(value) {
                            updated_at = dt.with_timezone(&Utc);
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    // Parse title (first # heading)
    let mut name = String::from("Untitled Chat");
    let mut messages = Vec::new();
    let mut current_role: Option<Role> = None;
    let mut current_content = String::new();

    for line in lines {
        if line.starts_with("# ") {
            name = line[2..].trim().to_string();
        } else if line.starts_with("## ") {
            // Save previous message if exists
            if let Some(role) = current_role.take() {
                if !current_content.trim().is_empty() {
                    messages.push(Message {
                        role,
                        content: current_content.trim().to_string(),
                        thinking_content: None,
                        is_thinking_collapsed: false,
                    });
                }
                current_content.clear();
            }

            // Determine new role
            let header = line[3..].trim();
            current_role = if header.contains("User") || header.starts_with("👤") {
                Some(Role::User)
            } else if header.contains("Assistant") || header.starts_with("🤖") {
                Some(Role::Assistant)
            } else if header.contains("System") || header.starts_with("📢") {
                Some(Role::System)
            } else {
                None
            };
        } else if current_role.is_some() {
            current_content.push_str(line);
            current_content.push('\n');
        }
    }

    // Save last message
    if let Some(role) = current_role {
        if !current_content.trim().is_empty() {
            messages.push(Message {
                role,
                content: current_content.trim().to_string(),
                thinking_content: None,
                is_thinking_collapsed: false,
            });
        }
    }

    Ok(ChatSession {
        id,
        name,
        created_at,
        updated_at,
        messages,
        file_path: Some(file_path),
    })
}

/// Save a chat session to a markdown file
pub fn save_chat(session: &ChatSession) -> Result<PathBuf> {
    let chats_dir = ensure_chats_dir()?;

    // Generate filename if this is a new session
    let file_path = if let Some(ref path) = session.file_path {
        path.clone()
    } else {
        let sanitized_name = sanitize_filename(&session.name);
        let timestamp = session.created_at.format("%Y%m%d_%H%M%S");
        let filename = if sanitized_name.is_empty() {
            format!("{}.md", timestamp)
        } else {
            format!("{}_{}.md", timestamp, sanitized_name)
        };
        chats_dir.join(filename)
    };

    let markdown = session_to_markdown(session);
    fs::write(&file_path, markdown)
        .with_context(|| format!("Failed to write chat to {}", file_path.display()))?;

    Ok(file_path)
}

/// Load all chat sessions from the chats directory
pub fn load_chats() -> Result<Vec<ChatSession>> {
    let chats_dir = get_chats_dir();

    if !chats_dir.exists() {
        return Ok(Vec::new());
    }

    let mut sessions = Vec::new();

    let entries = fs::read_dir(&chats_dir)
        .with_context(|| format!("Failed to read chats directory: {}", chats_dir.display()))?;

    for entry in entries {
        let entry = entry?;
        let path = entry.path();

        // Only process .md files
        if path.extension().and_then(|s| s.to_str()) != Some("md") {
            continue;
        }

        match fs::read_to_string(&path) {
            Ok(content) => {
                match markdown_to_session(&content, path.clone()) {
                    Ok(session) => sessions.push(session),
                    Err(e) => {
                        eprintln!("Failed to parse {}: {}", path.display(), e);
                    }
                }
            }
            Err(e) => {
                eprintln!("Failed to read {}: {}", path.display(), e);
            }
        }
    }

    // Sort by created_at, newest first
    sessions.sort_by(|a, b| b.created_at.cmp(&a.created_at));

    Ok(sessions)
}

/// Delete a chat session file
pub fn delete_chat(session: &ChatSession) -> Result<()> {
    if let Some(ref path) = session.file_path {
        fs::remove_file(path)
            .with_context(|| format!("Failed to delete chat file: {}", path.display()))?;
    }
    Ok(())
}


