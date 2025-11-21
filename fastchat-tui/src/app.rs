use crate::config::AppConfig;
use crate::api::{send_message_stream, ApiEvent};
use crate::types::{Message, Role, ChatSession};
use crate::storage;
use tokio::sync::mpsc;

pub enum InputMode {
    Normal,
    Editing,
    Command,
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
    pub show_leader_menu: bool,
    pub command_input: String,
    pub show_line_numbers: bool,
    pub auto_scroll: bool,
    pub total_lines: usize,
    pub viewport_height: u16,
    
    // Model selection fields
    pub show_model_selection: bool,
    pub model_input: String,
}

impl App {
    pub fn new() -> App {
        let (tx, rx) = mpsc::channel(100);
        let history = storage::load_chats().unwrap_or_default();
        let config = AppConfig::default();

        App {
            config,
            messages: vec![Message {
                role: Role::System,
                content: "Welcome to Fastchat TUI. Press 'Space' for shortcuts.".to_string(),
                thinking_content: None,
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
                "lmstudio".to_string(),
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
            show_leader_menu: false,
            command_input: String::new(),
            show_line_numbers: true,
            auto_scroll: true,
            total_lines: 0,
            viewport_height: 0,
            show_model_selection: false,
            model_input: String::new(),
        }
    }

    pub fn toggle_shortcuts(&mut self) {
        self.show_shortcuts = !self.show_shortcuts;
    }

    pub fn toggle_leader_menu(&mut self) {
        self.show_leader_menu = !self.show_leader_menu;
        if self.show_leader_menu {
            self.pending_leader_key = true;
        } else {
            self.pending_leader_key = false;
        }
    }
    


    pub fn toggle_stats(&mut self) {
        self.show_stats = !self.show_stats;
    }

    pub fn toggle_history(&mut self) {
        self.show_history = !self.show_history;
        if self.show_history {
            // Reload history from disk when opening
            self.reload_history();
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
            thinking_content: None,
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
            self.auto_scroll = false;
        }
    }

    pub fn scroll_down(&mut self) {
        self.scroll += 1;
        self.auto_scroll = false;
    }

    pub async fn submit_message(&mut self) {
        if self.input.trim().is_empty() {
            return;
        }

        let mut content = self.input.clone();
        self.input.clear();
        
        self.messages.push(Message {
            role: Role::User,
            content: content.clone(),
            thinking_content: None,
        });
        self.user_msg_count += 1;

        // Add empty assistant message to stream into
        self.messages.push(Message {
            role: Role::Assistant,
            content: String::new(),
            thinking_content: None,
        });
        self.assistant_msg_count += 1;

        self.is_processing = true;
        let tx = self.tx.clone();
        let config = self.config.clone();
        let history = self.messages.clone(); 

        let handle = tokio::spawn(async move {
            if let Err(e) = send_message_stream(config, history, tx).await {
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
                ApiEvent::ThinkingToken(token) => {
                    if let Some(last_msg) = self.messages.last_mut() {
                        if let Role::Assistant = last_msg.role {
                            if let Some(ref mut thinking) = last_msg.thinking_content {
                                thinking.push_str(&token);
                            } else {
                                last_msg.thinking_content = Some(token);
                            }
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
                        thinking_content: None,
                    });
                    self.is_processing = false;
                }
            }
        }
    }
    
    pub fn is_input_mode(&self) -> bool {
        matches!(self.input_mode, InputMode::Editing)
    }

    pub fn is_command_mode(&self) -> bool {
        matches!(self.input_mode, InputMode::Command)
    }

    pub fn set_command_mode(&mut self) {
        self.input_mode = InputMode::Command;
        self.command_input.clear();
    }

    pub fn enter_command_char(&mut self, c: char) {
        self.command_input.push(c);
    }

    pub fn delete_command_char(&mut self) {
        self.command_input.pop();
    }

    pub fn execute_command(&mut self) {
        let cmd = self.command_input.trim();

        // Check if it's a line number command
        if let Ok(line_num) = cmd.parse::<u16>() {
            // Jump to line number
            self.scroll = line_num.saturating_sub(1);
            self.auto_scroll = false;
        } else if cmd == "top" || cmd == "gg" {
            self.scroll = 0;
            self.auto_scroll = false;
        } else if cmd == "bottom" || cmd == "G" {
            self.scroll_to_bottom();
        } else if cmd == "auto" {
            self.auto_scroll = !self.auto_scroll;
            if self.auto_scroll {
                self.scroll_to_bottom();
            }
        }

        self.command_input.clear();
        self.input_mode = InputMode::Normal;
    }

    pub fn scroll_to_bottom(&mut self) {
        if self.total_lines > self.viewport_height as usize {
            self.scroll = (self.total_lines - self.viewport_height as usize) as u16;
        } else {
            self.scroll = 0;
        }
        self.auto_scroll = true;
    }

    pub fn scroll_page_down(&mut self) {
        let page_size = self.viewport_height.saturating_sub(2);
        self.scroll = self.scroll.saturating_add(page_size);
        self.auto_scroll = false;
    }

    pub fn scroll_page_up(&mut self) {
        let page_size = self.viewport_height.saturating_sub(2);
        self.scroll = self.scroll.saturating_sub(page_size);
        self.auto_scroll = false;
    }

    pub fn update_viewport(&mut self, height: u16, total_lines: usize) {
        self.viewport_height = height;
        self.total_lines = total_lines;

        // Auto-scroll if enabled and new content arrived
        if self.auto_scroll {
            self.scroll_to_bottom();
        }
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
            thinking_content: None,
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

        let now = chrono::Utc::now();

        if let Some(id) = &self.current_session_id {
            if let Some(idx) = self.history.iter().position(|s| &s.id == id) {
                self.history[idx].messages = self.messages.clone();
                self.history[idx].name = name;
                self.history[idx].updated_at = now;

                // Save individual chat file
                if let Err(e) = storage::save_chat(&self.history[idx]) {
                    eprintln!("Failed to save chat: {}", e);
                }
            } else {
                let new_id = chrono::Utc::now().to_rfc3339();
                self.current_session_id = Some(new_id.clone());
                let session = ChatSession {
                    id: new_id,
                    name,
                    created_at: now,
                    updated_at: now,
                    messages: self.messages.clone(),
                    file_path: None,
                };

                // Save chat file and get path
                if let Ok(path) = storage::save_chat(&session) {
                    let mut saved_session = session;
                    saved_session.file_path = Some(path);
                    self.history.insert(0, saved_session);
                } else {
                    eprintln!("Failed to save new chat");
                }
            }
        } else {
            let new_id = chrono::Utc::now().to_rfc3339();
            self.current_session_id = Some(new_id.clone());
            let session = ChatSession {
                id: new_id,
                name,
                created_at: now,
                updated_at: now,
                messages: self.messages.clone(),
                file_path: None,
            };

            // Save chat file and get path
            if let Ok(path) = storage::save_chat(&session) {
                let mut saved_session = session;
                saved_session.file_path = Some(path);
                self.history.insert(0, saved_session);
            } else {
                eprintln!("Failed to save new chat");
            }
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

    pub fn new_chat(&mut self) {
        // Save current session before starting new
        self.save_current_session();

        // Reset to fresh chat
        self.messages = vec![Message {
            role: Role::System,
            content: "Welcome to Fastchat TUI. Press 'Space' for shortcuts.".to_string(),
            thinking_content: None,
        }];
        self.current_session_id = None;
        self.user_msg_count = 0;
        self.assistant_msg_count = 0;
        self.session_start = std::time::Instant::now();
        self.show_history = false;

        // Reload history to get fresh list
        self.history = storage::load_chats().unwrap_or_default();
        self.selected_chat_index = 0;
    }

    pub fn delete_selected_chat(&mut self) {
        if self.history.is_empty() || self.selected_chat_index >= self.history.len() {
            return;
        }

        let session_to_delete = &self.history[self.selected_chat_index];

        // Don't allow deleting the currently active session
        if let Some(current_id) = &self.current_session_id {
            if &session_to_delete.id == current_id {
                self.messages.push(Message {
                    role: Role::System,
                    content: "Cannot delete the currently active chat. Switch to another chat first.".to_string(),
                    thinking_content: None,
                });
                return;
            }
        }

        // Delete the file
        if let Err(e) = storage::delete_chat(session_to_delete) {
            eprintln!("Failed to delete chat: {}", e);
            return;
        }

        // Remove from history
        self.history.remove(self.selected_chat_index);

        // Adjust selection
        if self.selected_chat_index >= self.history.len() && self.selected_chat_index > 0 {
            self.selected_chat_index -= 1;
        }
    }

    pub fn reload_history(&mut self) {
        self.history = storage::load_chats().unwrap_or_default();
        if self.selected_chat_index >= self.history.len() {
            self.selected_chat_index = self.history.len().saturating_sub(1);
        }
    }
    
    
    // Model selection methods
    pub fn toggle_model_selection(&mut self) {
        self.show_model_selection = !self.show_model_selection;
        if self.show_model_selection {
            // Pre-fill with current model
            if let Some(backend) = self.config.get_active_backend() {
                self.model_input = backend.model.clone();
            }
        }
    }
    
    pub fn enter_model_char(&mut self, c: char) {
        self.model_input.push(c);
    }
    
    pub fn delete_model_char(&mut self) {
        self.model_input.pop();
    }
    
    pub fn confirm_model_change(&mut self) {
        let new_model = self.model_input.trim().to_string();
        if !new_model.is_empty() {
            if let Some(backend) = self.config.backends.get_mut(&self.config.active_backend) {
                backend.model = new_model.clone();
                self.messages.push(Message {
                    role: Role::System,
                    content: format!("Model changed to: {}", new_model),
                    thinking_content: None,
                });
            }
        }
        self.show_model_selection = false;
        self.model_input.clear();
    }
    
    pub fn cancel_model_selection(&mut self) {
        self.show_model_selection = false;
        self.model_input.clear();
    }
}