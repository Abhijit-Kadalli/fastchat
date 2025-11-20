use anyhow::{Result, Context, anyhow};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::fs;
use std::sync::{Arc, Mutex};
use text_splitter::TextSplitter;
use reqwest::Client;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Chunk {
    pub text: String,
    pub source: String, // Filename or Chat ID
    pub embedding: Vec<f32>,
}

#[derive(Clone)]
pub struct RagSystem {
    client: Client,
    chunks: Arc<Mutex<Vec<Chunk>>>,
    index_path: PathBuf,
    pub base_url: String,
}

#[derive(Serialize)]
struct EmbeddingRequest {
    input: Vec<String>,
    model: String,
}

#[derive(Deserialize)]
struct EmbeddingResponse {
    data: Vec<EmbeddingData>,
}

#[derive(Deserialize)]
struct EmbeddingData {
    embedding: Vec<f32>,
}

impl RagSystem {
    pub fn new(base_url: String) -> Result<Self> {
        let index_path = crate::storage::get_chats_dir().join("rag_index.json");
        let chunks = if index_path.exists() {
            let file = fs::File::open(&index_path)?;
            serde_json::from_reader(file).unwrap_or_default()
        } else {
            Vec::new()
        };

        Ok(Self {
            client: Client::new(),
            chunks: Arc::new(Mutex::new(chunks)),
            index_path,
            base_url,
        })
    }
    
    pub fn set_base_url(&mut self, url: String) {
        self.base_url = url;
    }

    async fn get_embeddings(&self, texts: Vec<String>) -> Result<Vec<Vec<f32>>> {
        let model = "all-minilm"; 

        let url = if self.base_url.ends_with("/v1") {
            format!("{}/embeddings", self.base_url)
        } else {
            format!("{}/v1/embeddings", self.base_url)
        };

        let res = self.client.post(&url)
            .json(&EmbeddingRequest {
                input: texts,
                model: model.to_string(),
            })
            .send()
            .await?;

        if !res.status().is_success() {
            let status = res.status();
            let text = res.text().await?;
            return Err(anyhow!("Embedding API failed: {} - {}", status, text));
        }

        let response: EmbeddingResponse = res.json().await?;
        Ok(response.data.into_iter().map(|d| d.embedding).collect())
    }

    pub async fn index_text(&self, text: &str, source: &str) -> Result<()> {
        // TextSplitter::new(max_characters)
        let splitter = TextSplitter::new(500);
        let chunks: Vec<String> = splitter.chunks(text).map(|s| s.to_string()).collect();
        
        if chunks.is_empty() {
            return Ok(());
        }

        // Process in batches if needed, but for now all at once
        let embeddings = self.get_embeddings(chunks.clone()).await?;

        let mut store = self.chunks.lock().unwrap();
        
        // Remove old chunks from same source
        store.retain(|c| c.source != source);

        for (text_chunk, embedding) in chunks.into_iter().zip(embeddings) {
            store.push(Chunk {
                text: text_chunk,
                source: source.to_string(),
                embedding,
            });
        }

        self.save_index(&store)?;
        Ok(())
    }

    pub async fn index_file(&self, path: &Path) -> Result<()> {
        let content = fs::read_to_string(path)?;
        let filename = path.file_name().unwrap_or_default().to_string_lossy().to_string();
        self.index_text(&content, &filename).await
    }

    fn save_index(&self, chunks: &[Chunk]) -> Result<()> {
        let file = fs::File::create(&self.index_path)?;
        serde_json::to_writer(file, chunks)?;
        Ok(())
    }

    pub async fn search(&self, query: &str, limit: usize) -> Result<Vec<(Chunk, f32)>> {
        let embeddings = self.get_embeddings(vec![query.to_string()]).await?;
        if embeddings.is_empty() {
            return Ok(Vec::new());
        }
        let query_vec = &embeddings[0];

        let store = self.chunks.lock().unwrap();
        let mut results: Vec<(Chunk, f32)> = store
            .iter()
            .map(|chunk| {
                let score = cosine_similarity(query_vec, &chunk.embedding);
                (chunk.clone(), score)
            })
            .collect();

        results.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
        results.truncate(limit);

        Ok(results)
    }
    
    pub fn get_document_count(&self) -> usize {
        let store = self.chunks.lock().unwrap();
        let mut sources = std::collections::HashSet::new();
        for chunk in store.iter() {
            sources.insert(chunk.source.clone());
        }
        sources.len()
    }
    
    pub fn get_sources(&self) -> Vec<String> {
        let store = self.chunks.lock().unwrap();
        let mut sources = std::collections::HashSet::new();
        for chunk in store.iter() {
            sources.insert(chunk.source.clone());
        }
        let mut sorted_sources: Vec<String> = sources.into_iter().collect();
        sorted_sources.sort();
        sorted_sources
    }
    
    pub fn clear_index(&self) -> Result<()> {
        let mut store = self.chunks.lock().unwrap();
        store.clear();
        self.save_index(&store)?;
        Ok(())
    }
}

fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    let dot_product: f32 = a.iter().zip(b).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm_a == 0.0 || norm_b == 0.0 {
        0.0
    } else {
        dot_product / (norm_a * norm_b)
    }
}
