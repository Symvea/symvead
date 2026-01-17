//! Versioned symbol storage for tracking symbol evolution over time
//! This module provides advanced versioning capabilities for symbols.

#![allow(dead_code)] // Future feature - not fully integrated yet

use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use anyhow::Result;

pub type Epoch = u64;
pub type Hash256 = [u8; 32];

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SymbolVersion {
    pub version_id: u64,
    pub content_hash: Hash256,
    pub timestamp: Epoch,
    pub parent_hash: Option<Hash256>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StabilityMetrics {
    pub total_versions: u64,
    pub last_change_epoch: Epoch,
    pub stability_score: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DominanceMetrics {
    pub inbound_links: u64,
    pub outbound_links: u64,
    pub dominance_score: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SymbolHistory {
    pub symbol_id: String,
    pub versions: Vec<SymbolVersion>,
    pub stability: StabilityMetrics,
    pub dominance: DominanceMetrics,
}

pub struct VersionedSymbolStore {
    data_dir: String,
}

impl VersionedSymbolStore {
    pub fn new(data_dir: impl Into<String>) -> Self {
        let data_dir = data_dir.into();
        std::fs::create_dir_all(format!("{}/symbol_versions", data_dir)).ok();
        std::fs::create_dir_all(format!("{}/symbol_metrics", data_dir)).ok();
        
        Self { data_dir }
    }
    
    pub fn add_symbol_version(&self, symbol_id: &str, content: &[u8]) -> Result<()> {
        let content_hash = crate::engine::hash::sha256(content);
        let current_epoch = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_secs();
        
        // Load existing history
        let mut history = self.load_symbol_history(symbol_id)
            .unwrap_or_else(|_| SymbolHistory {
                symbol_id: symbol_id.to_string(),
                versions: Vec::new(),
                stability: StabilityMetrics {
                    total_versions: 0,
                    last_change_epoch: current_epoch,
                    stability_score: 0.0,
                },
                dominance: DominanceMetrics {
                    inbound_links: 0,
                    outbound_links: 0,
                    dominance_score: 0,
                },
            });
        
        // Check if content actually changed
        if let Some(latest) = history.versions.last() {
            if latest.content_hash == content_hash {
                return Ok(()); // No change, no new version
            }
        }
        
        // Create new version
        let version_id = history.versions.len() as u64;
        let parent_hash = history.versions.last().map(|v| v.content_hash);
        
        let new_version = SymbolVersion {
            version_id,
            content_hash,
            timestamp: current_epoch,
            parent_hash,
        };
        
        // Append version (never overwrite)
        history.versions.push(new_version);
        
        // Recompute metrics
        history.stability = self.compute_stability(&history.versions, current_epoch);
        
        // Store updated history
        self.store_symbol_history(&history)?;
        
        Ok(())
    }
    
    pub fn load_symbol_history(&self, symbol_id: &str) -> Result<SymbolHistory> {
        let path = format!("{}/symbol_versions/{}.json", self.data_dir, symbol_id);
        let json = std::fs::read_to_string(path)?;
        Ok(serde_json::from_str(&json)?)
    }
    
    fn store_symbol_history(&self, history: &SymbolHistory) -> Result<()> {
        let path = format!("{}/symbol_versions/{}.json", self.data_dir, history.symbol_id);
        let json = serde_json::to_string_pretty(history)?;
        std::fs::write(path, json)?;
        Ok(())
    }
    
    fn compute_stability(&self, versions: &[SymbolVersion], current_epoch: Epoch) -> StabilityMetrics {
        let total_versions = versions.len() as u64;
        
        if versions.is_empty() {
            return StabilityMetrics {
                total_versions: 0,
                last_change_epoch: current_epoch,
                stability_score: 0.0,
            };
        }
        
        let first_epoch = versions[0].timestamp;
        let last_change_epoch = versions.last().unwrap().timestamp;
        let age_in_epochs = current_epoch.saturating_sub(first_epoch);
        let mutations = total_versions.saturating_sub(1); // First version isn't a mutation
        
        // stability = age_in_epochs / (1 + number_of_mutations)
        let stability_score = age_in_epochs as f64 / (1.0 + mutations as f64);
        
        StabilityMetrics {
            total_versions,
            last_change_epoch,
            stability_score,
        }
    }
    
    pub fn compute_dominance(&self, symbol_id: &str, symbol_graph: &HashMap<String, Vec<String>>) -> DominanceMetrics {
        let inbound_links = symbol_graph.values()
            .map(|deps| deps.iter().filter(|&dep| dep == symbol_id).count() as u64)
            .sum();
        
        let outbound_links = symbol_graph.get(symbol_id)
            .map(|deps| deps.len() as u64)
            .unwrap_or(0);
        
        // dominance = references_to_symbol + dependent_symbols
        let dominance_score = inbound_links + outbound_links;
        
        DominanceMetrics {
            inbound_links,
            outbound_links,
            dominance_score,
        }
    }
    
    pub fn get_all_symbol_ids(&self) -> Result<Vec<String>> {
        let versions_dir = format!("{}/symbol_versions", self.data_dir);
        let mut symbol_ids = Vec::new();
        
        if let Ok(entries) = std::fs::read_dir(versions_dir) {
            for entry in entries {
                if let Ok(entry) = entry {
                    let filename = entry.file_name();
                    if let Some(name) = filename.to_str() {
                        if name.ends_with(".json") {
                            let symbol_id = name.strip_suffix(".json").unwrap();
                            symbol_ids.push(symbol_id.to_string());
                        }
                    }
                }
            }
        }
        
        Ok(symbol_ids)
    }
}