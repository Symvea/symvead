//! Symbol explanation system for tracking how symbols contribute to file compression
//! This module provides detailed analysis of symbol usage and contribution.

#![allow(dead_code)] // Future feature - not fully integrated yet

use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use anyhow::Result;

pub type SymbolId = String;
pub type FileHash = [u8; 32];

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Explanation {
    pub symbol_id: SymbolId,
    pub bytes_contributed: u64,
    pub percent_of_total: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExplanationGraph {
    pub file_hash: FileHash,
    pub total_bytes: u64,
    pub explained_bytes: u64,
    pub unexplained_bytes: u64,
    pub explanations: Vec<Explanation>,
    pub symbol_versions_used: HashMap<SymbolId, u64>,
    pub snapshot_epoch: u64,
}

impl ExplanationGraph {
    pub fn new(file_hash: FileHash, total_bytes: u64) -> Self {
        let current_epoch = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
            
        Self {
            file_hash,
            total_bytes,
            explained_bytes: 0,
            unexplained_bytes: total_bytes,
            explanations: Vec::new(),
            symbol_versions_used: HashMap::new(),
            snapshot_epoch: current_epoch,
        }
    }
    
    pub fn add_explanation(&mut self, symbol_id: SymbolId, bytes_contributed: u64, version_id: u64) -> Result<()> {
        // Safety invariant: explanation cannot exceed 100%
        let new_explained = self.explained_bytes + bytes_contributed;
        if new_explained > self.total_bytes {
            panic!("FATAL: Explanation > 100% - explained={}, total={}", new_explained, self.total_bytes);
        }
        
        let percent_of_total = (bytes_contributed as f64 / self.total_bytes as f64) * 100.0;
        
        self.explanations.push(Explanation {
            symbol_id: symbol_id.clone(),
            bytes_contributed,
            percent_of_total,
        });
        
        self.symbol_versions_used.insert(symbol_id, version_id);
        self.explained_bytes = new_explained;
        self.unexplained_bytes = self.total_bytes - self.explained_bytes;
        
        Ok(())
    }
    
    pub fn finalize(&mut self) -> Result<()> {
        // Sort by bytes contributed (descending)
        self.explanations.sort_by(|a, b| b.bytes_contributed.cmp(&a.bytes_contributed));
        
        // Verify total doesn't exceed 100%
        let total_percent: f64 = self.explanations.iter().map(|e| e.percent_of_total).sum();
        if total_percent > 100.1 { // Allow small floating point error
            panic!("FATAL: Total explanation > 100%: {:.2}%", total_percent);
        }
        
        Ok(())
    }
    
    pub fn get_explanations_by_stability(&self, stability_scores: &HashMap<SymbolId, f64>) -> Vec<&Explanation> {
        let mut sorted = self.explanations.iter().collect::<Vec<_>>();
        sorted.sort_by(|a, b| {
            let stability_a = stability_scores.get(&a.symbol_id).unwrap_or(&0.0);
            let stability_b = stability_scores.get(&b.symbol_id).unwrap_or(&0.0);
            stability_b.partial_cmp(stability_a).unwrap_or(std::cmp::Ordering::Equal)
        });
        sorted
    }
    
    pub fn get_explanations_by_dominance(&self, dominance_scores: &HashMap<SymbolId, u64>) -> Vec<&Explanation> {
        let mut sorted = self.explanations.iter().collect::<Vec<_>>();
        sorted.sort_by(|a, b| {
            let dominance_a = dominance_scores.get(&a.symbol_id).unwrap_or(&0);
            let dominance_b = dominance_scores.get(&b.symbol_id).unwrap_or(&0);
            dominance_b.cmp(dominance_a)
        });
        sorted
    }
}

pub struct ExplanationEngine {
    data_dir: String,
}

impl ExplanationEngine {
    pub fn new(data_dir: impl Into<String>) -> Self {
        let data_dir = data_dir.into();
        std::fs::create_dir_all(format!("{}/explanations", data_dir)).ok();
        Self { data_dir }
    }
    
    pub fn create_explanation(&self, file_key: &str, file_data: &[u8], symbol_contributions: Vec<(SymbolId, u64, u64)>) -> Result<ExplanationGraph> {
        let file_hash = crate::engine::hash::sha256(file_data);
        let total_bytes = file_data.len() as u64;
        
        let mut graph = ExplanationGraph::new(file_hash, total_bytes);
        
        for (symbol_id, bytes_contributed, version_id) in symbol_contributions {
            graph.add_explanation(symbol_id, bytes_contributed, version_id)?;
        }
        
        graph.finalize()?;
        
        // Store explanation
        self.store_explanation(file_key, &graph)?;
        
        Ok(graph)
    }
    
    pub fn load_explanation(&self, file_key: &str) -> Result<ExplanationGraph> {
        let path = format!("{}/explanations/{}.json", self.data_dir, file_key);
        let json = std::fs::read_to_string(path)?;
        Ok(serde_json::from_str(&json)?)
    }
    
    fn store_explanation(&self, file_key: &str, graph: &ExplanationGraph) -> Result<()> {
        let path = format!("{}/explanations/{}.json", self.data_dir, file_key);
        let json = serde_json::to_string_pretty(graph)?;
        std::fs::write(path, json)?;
        Ok(())
    }
    
    pub fn verify_explanation_reproducible(&self, file_key: &str, file_data: &[u8]) -> Result<bool> {
        let stored_graph = self.load_explanation(file_key)?;
        let computed_hash = crate::engine::hash::sha256(file_data);
        
        // Explanation must be reproducible from same file
        Ok(stored_graph.file_hash == computed_hash)
    }
}