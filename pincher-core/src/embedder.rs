//! Embedder — produces deterministic vector embeddings from text.
//!
//! Uses a hash-based approach: splits text into character trigrams,
//! hashes each trigram with SHA-256, and projects into a fixed-dimension
//! vector space. This is a deterministic, zero-dependency embedding that
//! preserves some locality (similar inputs → similar trigrams → similar vectors).

use anyhow::Result;
use sha2::{Digest, Sha256};
use std::collections::HashMap;

/// The dimensionality of embedding vectors.
pub const EMBED_DIM: usize = 256;

/// Embedder produces fixed-dimension vector embeddings from text.
pub struct Embedder {
    dim: usize,
}

impl Embedder {
    /// Create a new Embedder with the default dimension.
    pub fn new() -> Result<Self> {
        Ok(Self {
            dim: EMBED_DIM,
        })
    }

    /// Return the embedding dimension.
    pub fn dimension(&self) -> usize {
        self.dim
    }

    /// Embed a single text string into a vector.
    pub async fn embed(&self, text: &str) -> Result<Vec<f32>> {
        let text = text.to_lowercase();
        let trigrams = Self::extract_trigrams(&text);
        let mut vector = vec![0.0f32; self.dim];

        for tri in &trigrams {
            let hash = Self::hash_trigram(tri);
            // Use hash bytes to set multiple positions in the vector
            for i in 0..8 {
                let idx = (hash[i] as usize) % self.dim;
                let sign = if hash[(i + 8) % 32] % 2 == 0 { 1.0f32 } else { -1.0f32 };
                vector[idx] += sign * 0.1;
            }
        }

        // Add character frequency signal
        let char_freqs = Self::char_frequencies(&text);
        for (ch, count) in &char_freqs {
            let idx = (*ch as usize) % self.dim;
            vector[idx] += *count as f32 * 0.05;
        }

        // Add word-level signal
        let words: Vec<&str> = text.split_whitespace().collect();
        for (i, word) in words.iter().enumerate() {
            let word_hash = Self::hash_trigram(word);
            let idx = (word_hash[0] as usize + i) % self.dim;
            vector[idx] += 0.3;
        }

        // Normalize to unit vector
        Self::normalize(&mut vector);

        Ok(vector)
    }

    /// Embed a batch of text strings.
    pub async fn embed_batch(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>> {
        let mut results = Vec::with_capacity(texts.len());
        for text in texts {
            results.push(self.embed(text).await?);
        }
        Ok(results)
    }

    /// Compute cosine similarity between two vectors.
    pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f64 {
        if a.len() != b.len() || a.is_empty() {
            return 0.0;
        }
        let dot: f64 = a.iter().zip(b.iter()).map(|(x, y)| (*x as f64) * (*y as f64)).sum();
        let mag_a: f64 = a.iter().map(|x| (*x as f64).powi(2)).sum::<f64>().sqrt();
        let mag_b: f64 = b.iter().map(|x| (*x as f64).powi(2)).sum::<f64>().sqrt();
        if mag_a == 0.0 || mag_b == 0.0 {
            return 0.0;
        }
        dot / (mag_a * mag_b)
    }

    fn extract_trigrams(text: &str) -> Vec<String> {
        let chars: Vec<char> = text.chars().collect();
        if chars.len() < 3 {
            return vec![text.to_string()];
        }
        let mut trigrams = Vec::new();
        for i in 0..=chars.len() - 3 {
            trigrams.push(chars[i..i + 3].iter().collect());
        }
        trigrams
    }

    fn hash_trigram(input: &str) -> [u8; 32] {
        let mut hasher = Sha256::new();
        hasher.update(input.as_bytes());
        hasher.finalize().into()
    }

    fn char_frequencies(text: &str) -> HashMap<char, f32> {
        let mut freqs = HashMap::new();
        let total = text.len() as f32;
        if total == 0.0 {
            return freqs;
        }
        for ch in text.chars() {
            *freqs.entry(ch).or_insert(0.0f32) += 1.0 / total;
        }
        freqs
    }

    fn normalize(vector: &mut [f32]) {
        let mag: f64 = vector.iter().map(|x| (*x as f64).powi(2)).sum::<f64>().sqrt();
        if mag > 0.0 {
            for v in vector.iter_mut() {
                *v = (*v as f64 / mag) as f32;
            }
        }
    }
}
