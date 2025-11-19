use crate::config::AppConfig;
use crate::api::{send_message_stream, ApiEvent};
use crate::types::{Message, Role, ChatSession};
use crate::storage;
use tokio::sync::mpsc;

pub enum InputMode {
    Normal,
    Editing,
}

pub struct App {
    pub config: AppConfig,
    pub messages: Vec<Message>,
    pub input: String,
    pub input_mode: InputMode,
    pub scroll: u16,
    pub is_processing: bool,
    pub show_shortcuts: bool,
    pub show_stats: bool,
    pub show_history: bool,
    pub history: Vec<ChatSession>,
    pub history_scroll: usize,
    pub selected_chat_index: usize,
    pub show_backend_selection: bool,
    pub show_url_edit: bool,
    pub available_backends: Vec<String>,
    pub backend_selection_index: usize,
    pub url_input: String,
    pub current_session_id: Option<String>,
    pub session_start: std::time::Instant,
    pub user_msg_count: usize,
    pub assistant_msg_count: usize,
    pub rx: mpsc::Receiver<ApiEvent>,
    pub tx: mpsc::Sender<ApiEvent>,
    pub current_task: Option<tokio::task::JoinHandle<()>>,
    pub pending_leader_key: bool,
}

impl App {
    pub fn new() -> App {
        let (tx, rx) = mpsc::channel(100);
        let history = storage::load_chats().unwrap_or_default();
        App {
            config: AppConfig::default(),
            messages: vec![Message {
                role: Role::System,
                content: "Welcome to Fastchat TUI. Press 'Space' for shortcuts.".to_string(),
            }],
            input: String::new(),
            input_mode: InputMode::Normal,
            scroll: 0,
            is_processing: false,
            show_shortcuts: false,
            show_stats: false,
            show_history: false,
            history,
            history_scroll: 0,
            selected_chat_index: 0,
            show_backend_selection: false,
            show_url_edit: false,
            available_backends: vec![
                "tabbyapi".to_string(),
                "sglang".to_string(),
                "ollama".to_string(),
                "vllm".to_string(),
            ],
            backend_selection_index: 0,
            url_input: String::new(),
            current_session_id: None,
            session_start: std::time::Instant::now(),
            user_msg_count: 0,
            assistant_msg_count: 0,
            rx,
            tx,
            current_task: None,
            pending_leader_key: false,
        }
    }

    pub fn toggle_shortcuts(&mut self) {
        self.show_shortcuts = !self.show_shortcuts;
    }

    pub fn toggle_stats(&mut self) {
        self.show_stats = !self.show_stats;
    }

    pub fn toggle_history(&mut self) {
        self.show_history = !self.show_history;
        if self.show_history {
            // Reload history when opening
            // In a real app we might want to do this async or cache it
            // For now we'll assume it's loaded or we load it here
            // But we can't call async functions easily here without &mut self lifetime issues if we are not careful
            // So we will rely on the main loop or a separate loader.
            // For now, let's just reset selection
            self.selected_chat_index = 0;
            self.history_scroll = 0;
        }
    }

    pub fn clear_history(&mut self) {
        self.save_current_session();
        // Keep the system prompt (index 0)
        if !self.messages.is_empty() {
            self.messages.truncate(1);
        }
        self.user_msg_count = 0;
        self.assistant_msg_count = 0;
        self.current_session_id = None;
    }
    
    pub fn stop_generation(&mut self) {
        if let Some(handle) = self.current_task.take() {
            handle.abort();
        }
        self.is_processing = false;
        self.messages.push(Message {
            role: Role::System,
            content: "Generation stopped by user.".to_string(),
        });
    }

    pub fn set_input_mode(&mut self) {
        self.input_mode = InputMode::Editing;
        self.show_shortcuts = false;
    }

    pub fn set_normal_mode(&mut self) {
        self.input_mode = InputMode::Normal;
    }

    pub fn enter_char(&mut self, c: char) {
        self.input.push(c);
    }

    pub fn delete_char(&mut self) {
        self.input.pop();
    }

    pub fn scroll_up(&mut self) {
        if self.scroll > 0 {
            self.scroll -= 1;
        }
    }

    pub fn scroll_down(&mut self) {
        self.scroll += 1;
    }

    pub async fn submit_message(&mut self) {
        if self.input.trim().is_empty() {
            return;
        }

        let content = self.input.clone();
        self.input.clear();
        self.messages.push(Message {
            role: Role::User,
            content: content.clone(),
        });
        self.user_msg_count += 1;

        // Add empty assistant message to stream into
        self.messages.push(Message {
            role: Role::Assistant,
            content: String::new(),
        });
        self.assistant_msg_count += 1;

        self.is_processing = true;
        let tx = self.tx.clone();
        let config = self.config.clone();
        let history = self.messages.clone(); // Simplified: sending whole history including the new user msg and empty assistant msg

        let handle = tokio::spawn(async move {
            if let Err(e) = send_message_stream(config, history, tx).await {
                // Handle error (maybe send an error event)
                eprintln!("Error sending message: {}", e);
            }
        });
        
        self.current_task = Some(handle);
    }

    pub async fn tick(&mut self) {
        while let Ok(event) = self.rx.try_recv() {
            match event {
                ApiEvent::Token(token) => {
                    if let Some(last_msg) = self.messages.last_mut() {
                        if let Role::Assistant = last_msg.role {
                            last_msg.content.push_str(&token);
                        }
                    }
                }
                ApiEvent::Done => {
                    self.is_processing = false;
                }
                ApiEvent::Error(err) => {
                    self.messages.push(Message {
                        role: Role::System,
                        content: format!("Error: {}", err),
                    });
                    self.is_processing = false;
                }
            }
        }
    }
    
    pub fn is_input_mode(&self) -> bool {
        matches!(self.input_mode, InputMode::Editing)
    }

    pub fn toggle_backend_selection(&mut self) {
        self.show_backend_selection = !self.show_backend_selection;
        if self.show_backend_selection {
            self.show_url_edit = false;
            // Set selection index to current active backend
            if let Some(pos) = self.available_backends.iter().position(|b| b == &self.config.active_backend) {
                self.backend_selection_index = pos;
            }
        }
    }

    pub fn next_backend(&mut self) {
        if self.backend_selection_index < self.available_backends.len() - 1 {
            self.backend_selection_index += 1;
        } else {
            self.backend_selection_index = 0;
        }
    }

    pub fn previous_backend(&mut self) {
        if self.backend_selection_index > 0 {
            self.backend_selection_index -= 1;
        } else {
            self.backend_selection_index = self.available_backends.len() - 1;
        }
    }

    pub fn select_backend(&mut self) {
        let selected_backend = &self.available_backends[self.backend_selection_index];
        
        // Pre-fill URL input with current URL for this backend
        if let Some(config) = self.config.backends.get(selected_backend) {
            self.url_input = config.url.clone();
        }
        
        self.show_backend_selection = false;
        self.show_url_edit = true;
    }

    pub fn confirm_backend_switch(&mut self) {
        let selected_backend = self.available_backends[self.backend_selection_index].clone();
        let new_url = self.url_input.clone();

        // Update config
        if let Some(backend_config) = self.config.backends.get_mut(&selected_backend) {
            backend_config.url = new_url.clone();
        }
        self.config.active_backend = selected_backend.clone();

        self.show_url_edit = false;
        self.messages.push(Message {
            role: Role::System,
            content: format!("Switched to backend: {} (URL: {})", selected_backend, new_url),
        });
    }

    pub fn cancel_url_edit(&mut self) {
        self.show_url_edit = false;
        self.show_backend_selection = true; // Go back to selection
    }

    pub fn enter_url_char(&mut self, c: char) {
        self.url_input.push(c);
    }

    pub fn delete_url_char(&mut self) {
        self.url_input.pop();
    }
    
    pub fn save_current_session(&mut self) {
        if self.messages.len() <= 1 { return; }
        
        let name = self.messages.iter()
            .find(|m| matches!(m.role, Role::User))
            .map(|m| m.content.lines().next().unwrap_or("New Chat").chars().take(50).collect::<String>())
            .unwrap_or_else(|| "New Chat".to_string());

        if let Some(id) = &self.current_session_id {
            if let Some(idx) = self.history.iter().position(|s| &s.id == id) {
                self.history[idx].messages = self.messages.clone();
                self.history[idx].name = name;
            } else {
                let new_id = chrono::Utc::now().to_rfc3339();
                self.current_session_id = Some(new_id.clone());
                let session = ChatSession {
                    id: new_id,
                    name,
                    created_at: chrono::Utc::now(),
                    messages: self.messages.clone(),
                };
                self.history.insert(0, session);
            }
        } else {
            let new_id = chrono::Utc::now().to_rfc3339();
            self.current_session_id = Some(new_id.clone());
            let session = ChatSession {
                id: new_id,
                name,
                created_at: chrono::Utc::now(),
                messages: self.messages.clone(),
            };
            self.history.insert(0, session);
        }
        
        if let Err(e) = storage::save_chats(&self.history) {
            eprintln!("Failed to save history: {}", e);
        }
    }

    pub fn load_selected_chat(&mut self) {
        if self.history.is_empty() || self.selected_chat_index >= self.history.len() {
            return;
        }
        // Save current before loading?
        self.save_current_session();

        let session = &self.history[self.selected_chat_index];
        self.messages = session.messages.clone();
        self.current_session_id = Some(session.id.clone());
        self.show_history = false;
        self.user_msg_count = self.messages.iter().filter(|m| matches!(m.role, Role::User)).count();
        self.assistant_msg_count = self.messages.iter().filter(|m| matches!(m.role, Role::Assistant)).count();
    }

    pub fn history_up(&mut self) {
        if self.selected_chat_index > 0 {
            self.selected_chat_index -= 1;
        }
    }

    pub fn history_down(&mut self) {
        if self.selected_chat_index < self.history.len().saturating_sub(1) {
            self.selected_chat_index += 1;
        }
    }
}