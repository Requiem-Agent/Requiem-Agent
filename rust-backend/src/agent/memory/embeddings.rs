//! # Embeddings — Hybrid 256-dim semantic embeddings
//!
//! Replaces the basic HashEmbedding with a multi-layer approach:
//!   Layer 0..63   — TF-IDF token hashing (Arabic + English aware)
//!   Layer 64..127 — Character 3-gram fingerprints
//!   Layer 128..191 — Code keyword positions with weights
//!   Layer 192..255 — Structural features (code ratio, language, length)

use std::collections::{HashMap, HashSet};
use tracing::debug;

pub const EMBEDDING_DIM: usize = 256;

/// خوارزمية التضمين
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EmbeddingAlgorithm {
    TfIdf,
    BagOfWords,
    HashEmbedding,
    /// الخوارزمية الجديدة — 256-dim hybrid (default)
    HybridSemantic,
}

// ── Arabic + English stop words ────────────────────────────────────────────
fn arabic_stopwords() -> HashSet<&'static str> {
    [
        "من", "في", "على", "إلى", "عن", "مع", "هذا", "هذه", "ذلك", "تلك",
        "التي", "الذي", "هو", "هي", "نحن", "أنت", "أنا", "كان", "كانت",
        "يكون", "لكن", "أو", "و", "ثم", "قد", "لا", "ما", "هل", "أن",
        "إن", "عند", "بعد", "قبل", "حتى", "كل", "كيف", "لم",
    ].iter().cloned().collect()
}

fn english_stopwords() -> HashSet<&'static str> {
    [
        "the", "a", "an", "is", "it", "in", "on", "at", "to", "of", "for",
        "and", "or", "but", "not", "with", "this", "that", "are", "was",
        "be", "been", "have", "has", "do", "does", "i", "you", "we", "he",
        "she", "they", "my", "your", "our", "its",
    ].iter().cloned().collect()
}

// ── Code keywords with weights ─────────────────────────────────────────────
fn code_keywords() -> HashMap<&'static str, f32> {
    let mut m = HashMap::new();
    // Rust
    for kw in &["fn", "pub", "struct", "impl", "trait", "enum", "crate", "mod"] {
        m.insert(*kw, 0.9f32);
    }
    // TypeScript/JS
    for kw in &["function", "interface", "export", "extends", "promise", "typeof"] {
        m.insert(*kw, 0.85f32);
    }
    // Shared keywords
    for kw in &["async", "await", "return", "class", "import", "type", "let", "const"] {
        m.insert(*kw, 0.75f32);
    }
    // SQL
    for kw in &["select", "where", "insert", "update", "delete", "create", "table"] {
        m.insert(*kw, 0.85f32);
    }
    // Domain
    for kw in &["error", "debug", "test", "memory", "session", "user", "api", "auth"] {
        m.insert(*kw, 0.8f32);
    }
    m
}

// ── FNV-1a hash ────────────────────────────────────────────────────────────
fn fnv1a(s: &str) -> u64 {
    let mut h: u64 = 14695981039346656037;
    for b in s.bytes() {
        h ^= b as u64;
        h = h.wrapping_mul(1099511628211);
    }
    h
}

fn hash_to_index(s: &str, offset: usize, range: usize) -> usize {
    offset + (fnv1a(s) as usize % range)
}

// ── Tokenize ───────────────────────────────────────────────────────────────
fn tokenize(text: &str) -> Vec<String> {
    let ar_stop = arabic_stopwords();
    let en_stop = english_stopwords();

    // Strip code blocks to marker
    let cleaned = {
        let mut s = text.to_lowercase();
        // Replace ```...``` with CODE_BLOCK
        // "```" is 3 ASCII bytes, so start+3 is always a valid char boundary
        while let Some(start) = s.find("```") {
            if let Some(rel) = s[start + 3..].find("```") {
                let end = start + 3 + rel + 3;
                s.replace_range(start..end, " CODE_BLOCK ");
            } else {
                break;
            }
        }
        s
    };

    cleaned
        .split(|c: char| !c.is_alphanumeric() && !('\u{0600}'..='\u{06FF}').contains(&c))
        .flat_map(|w| w.split(|c: char| c.is_ascii_punctuation()))
        .map(|w| w.trim().to_string())
        .filter(|w| w.len() >= 2 && !ar_stop.contains(w.as_str()) && !en_stop.contains(w.as_str()))
        .collect()
}

// ── L2 normalise ───────────────────────────────────────────────────────────
fn l2_normalize(v: &mut Vec<f32>) {
    let norm: f32 = v.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm > 1e-9 { v.iter_mut().for_each(|x| *x /= norm); }
}

// ── Main embedding ─────────────────────────────────────────────────────────
pub fn generate_embedding(text: &str) -> Vec<f32> {
    let mut emb = vec![0.0f32; EMBEDDING_DIM];
    let tokens = tokenize(text);
    let total = tokens.len().max(1) as f32;
    let kws = code_keywords();

    // Layer 1 (0..64): TF-IDF token hashing
    let mut freq: HashMap<String, f32> = HashMap::new();
    for t in &tokens { *freq.entry(t.clone()).or_insert(0.0) += 1.0; }
    for (tok, cnt) in &freq {
        let tf = cnt / total;
        let idf_proxy = 1.0 + (1.0 + tok.len() as f32 * 0.3).ln();
        let idx = hash_to_index(tok, 0, 64);
        emb[idx] += tf * idf_proxy;
    }

    // Layer 2 (64..128): Character 3-gram fingerprints
    let raw: String = text.to_lowercase().chars().take(500).collect();
    let ngrams: HashSet<&str> = (0..raw.len().saturating_sub(2))
        .filter_map(|i| raw.get(i..i + 3))
        .collect();
    let ng_count = ngrams.len().max(1) as f32;
    for ng in &ngrams {
        let idx = hash_to_index(ng, 64, 64);
        emb[idx] += 1.0 / ng_count.sqrt();
    }

    // Layer 3 (128..192): Code keyword positions + weights
    let lower = text.to_lowercase();
    for (kw, weight) in &kws {
        if lower.contains(kw) {
            let idx = hash_to_index(kw, 128, 64);
            emb[idx] += weight;
        }
    }

    // Layer 4 (192..256): Structural features
    let code_blocks = text.matches("```").count() / 2;
    let arabic_chars = text.chars().filter(|c| ('\u{0600}'..='\u{06FF}').contains(c)).count();
    let total_chars = text.len().max(1);
    let url_count = text.matches("http").count();
    let number_count = text.split_whitespace()
        .filter(|w| w.chars().all(|c| c.is_ascii_digit() || c == '.'))
        .count();

    emb[192] = (code_blocks as f32 / 10.0).min(1.0);
    emb[193] = arabic_chars as f32 / total_chars as f32;
    emb[194] = (url_count as f32 / 5.0).min(1.0);
    emb[195] = (number_count as f32 / 20.0).min(1.0);
    emb[196] = (tokens.len() as f32 / 200.0).min(1.0);
    emb[197] = (text.len() as f32 / 2000.0).min(1.0);

    // Prefix positional encoding (192..256 tail)
    let prefix: Vec<char> = text.chars().take(50).collect();
    for (i, ch) in prefix.iter().enumerate() {
        emb[198 + (i % 58)] += (*ch as u32 as f32) / 255.0 / prefix.len() as f32;
    }

    l2_normalize(&mut emb);
    debug!("Generated {}-dim embedding for {} chars", EMBEDDING_DIM, text.len());
    emb
}

// ── Cosine similarity ──────────────────────────────────────────────────────
pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() || a.is_empty() { return 0.0; }
    let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let na: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let nb: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    if na < 1e-9 || nb < 1e-9 { 0.0 } else { dot / (na * nb) }
}

// ── Priority weight ────────────────────────────────────────────────────────
pub fn priority_weight(priority: &str) -> f32 {
    match priority {
        "critical" => 2.0,
        "high"     => 1.5,
        "medium"   => 1.0,
        "low"      => 0.5,
        _          => 1.0,
    }
}

// ── Recency weight ─────────────────────────────────────────────────────────
pub fn recency_weight_from_iso(created_at: &str) -> f32 {
    // Parse ISO 8601 and compute age in days
    if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(created_at) {
        let age_days = (chrono::Utc::now() - dt.with_timezone(&chrono::Utc))
            .num_seconds() as f32 / 86400.0;
        if age_days < 1.0  { return 1.2; }
        if age_days < 7.0  { return 1.0; }
        if age_days < 30.0 { return 0.8; }
        return 0.6;
    }
    1.0
}

// ── Legacy API (keeps existing callers compiling) ──────────────────────────
pub struct EmbeddingGenerator {
    pub algorithm: EmbeddingAlgorithm,
    pub dimension: usize,
    vocabulary: HashMap<String, u32>,
    word_counts: HashMap<String, u32>,
    total_documents: u32,
}

impl EmbeddingGenerator {
    pub fn new(algorithm: EmbeddingAlgorithm, dimension: usize) -> Self {
        Self { algorithm, dimension, vocabulary: HashMap::new(), word_counts: HashMap::new(), total_documents: 0 }
    }

    pub fn embed(&mut self, text: &str) -> Vec<f32> {
        // All algorithms now route to the improved hybrid for consistency
        generate_embedding(text)
    }
}

pub fn quick_embed(text: &str, _dim: usize) -> Vec<f32> {
    generate_embedding(text)
}

pub fn normalize_vector(v: &mut Vec<f32>) {
    l2_normalize(v);
}

pub fn merge_embeddings(embeddings: &[Vec<f32>], weights: &[f32]) -> Vec<f32> {
    if embeddings.is_empty() { return Vec::new(); }
    let dim = embeddings[0].len();
    let mut result = vec![0.0f32; dim];
    let total_w: f32 = weights.iter().sum();
    if total_w < 1e-9 { return result; }
    for (emb, w) in embeddings.iter().zip(weights.iter()) {
        for (i, v) in emb.iter().enumerate() { result[i] += v * w / total_w; }
    }
    l2_normalize(&mut result);
    result
}

pub fn tokenize_pub(text: &str) -> Vec<String> { tokenize(text) }

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_embedding_dim() {
        let e = generate_embedding("hello world test");
        assert_eq!(e.len(), EMBEDDING_DIM);
    }

    #[test]
    fn test_embedding_normalised() {
        let e = generate_embedding("Rust async fn memory session");
        let norm: f32 = e.iter().map(|x| x * x).sum::<f32>().sqrt();
        assert!((norm - 1.0).abs() < 0.01, "norm={}", norm);
    }

    #[test]
    fn test_arabic_support() {
        let e = generate_embedding("المشروع يستخدم Rust backend");
        let norm: f32 = e.iter().map(|x| x * x).sum::<f32>().sqrt();
        assert!((norm - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_similarity_same_text() {
        let a = generate_embedding("TypeScript interface memory");
        let b = generate_embedding("TypeScript interface memory");
        let sim = cosine_similarity(&a, &b);
        assert!(sim > 0.99, "same text similarity={}", sim);
    }

    #[test]
    fn test_similarity_different() {
        let a = generate_embedding("Rust memory management");
        let b = generate_embedding("pizza recipe ingredients");
        let sim = cosine_similarity(&a, &b);
        assert!(sim < 0.8, "unrelated similarity={}", sim);
    }
}
