use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackendConfig {
    pub url: String,
    pub model: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub backends: HashMap<String, BackendConfig>,
    pub active_backend: String,
}

impl Default for AppConfig {
    fn default() -> Self {
        let mut backends = HashMap::new();
        
        backends.insert("tabbyapi".to_string(), BackendConfig {
            url: "http://localhost:5000/v1".to_string(),
            model: "default".to_string(),
        });
        
        backends.insert("sglang".to_string(), BackendConfig {
            url: "http://localhost:30000/v1".to_string(),
            model: "default".to_string(),
        });
        
        backends.insert("ollama".to_string(), BackendConfig {
            url: "http://localhost:11434/v1".to_string(),
            model: "qwen2.5:1.5b".to_string(),
        });
        
        backends.insert("vllm".to_string(), BackendConfig {
            url: "http://localhost:8000/v1".to_string(),
            model: "default".to_string(),
        });
        
        backends.insert("lmstudio".to_string(), BackendConfig {
            url: "http://localhost:1234/v1".to_string(),
            model: "default".to_string(),
        });

        Self {
            backends,
            active_backend: "lmstudio".to_string(),
        }
    }
}

impl AppConfig {
    pub fn get_active_backend(&self) -> Option<&BackendConfig> {
        self.backends.get(&self.active_backend)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = AppConfig::default();
        assert_eq!(config.active_backend, "tabbyapi");
        assert!(config.backends.contains_key("tabbyapi"));
        assert!(config.backends.contains_key("ollama"));
        
        let tabby = config.get_active_backend().unwrap();
        assert_eq!(tabby.model, "default");
    }
}
