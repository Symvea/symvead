use std::fs;
use std::path::PathBuf;
use serde::{Serialize, Deserialize};
use anyhow::Result;

#[derive(Debug, Serialize, Deserialize)]
pub struct PersistentStorage {
    pub root_path: PathBuf,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CorpusIndex {
    pub version: u32,
    pub files: Vec<FileEntry>,
    pub symbol_count: u64,
    pub total_size: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FileEntry {
    pub key: String,
    pub file_id: String,
    pub original_hash: String,
    pub symbols: Vec<String>,
}

impl PersistentStorage {
    pub fn new(root_path: impl Into<PathBuf>) -> Result<Self> {
        let root_path = root_path.into();
        
        // Create deterministic directory structure
        fs::create_dir_all(root_path.join("corpus/files"))?;
        fs::create_dir_all(root_path.join("symbols"))?;
        fs::create_dir_all(root_path.join("snapshots"))?;
        
        // Create STATE file if it doesn't exist
        let state_path = root_path.join("STATE");
        if !state_path.exists() {
            fs::write(&state_path, "INITIALIZED")?;
        }
        
        Ok(Self { root_path })
    }
    
    pub fn store_symbol_persistent(&self, hash: &str, bytes: &[u8], metadata: &crate::storage::symbols::StoredSymbol) -> Result<()> {
        let symbol_path = self.root_path.join("symbols").join(format!("sym_{}.bin", hash));
        let meta_path = self.root_path.join("symbols").join(format!("sym_{}.meta.json", hash));
        
        // Append-only: never overwrite existing symbols
        if symbol_path.exists() {
            return Ok(());
        }
        
        // Store binary data
        fs::write(&symbol_path, bytes)?;
        
        // Store metadata as JSON
        let meta_json = serde_json::to_string_pretty(metadata)?;
        fs::write(&meta_path, meta_json)?;
        
        Ok(())
    }
    
    pub fn store_file_metadata(&self, key: &str, metadata: &crate::storage::metadata::ObjectMetadata) -> Result<()> {
        let file_id = format!("{:x}", crc32fast::hash(key.as_bytes()));
        let meta_path = self.root_path.join("corpus/files").join(format!("{}.meta.json", file_id));
        
        let meta_json = serde_json::to_string_pretty(metadata)?;
        fs::write(&meta_path, meta_json)?;
        
        Ok(())
    }
    
    pub fn update_corpus_index(&self, files: Vec<FileEntry>) -> Result<()> {
        let index = CorpusIndex {
            version: 1,
            symbol_count: self.count_symbols()?,
            total_size: self.calculate_total_size()?,
            files,
        };
        
        let index_path = self.root_path.join("corpus/index.json");
        let index_json = serde_json::to_string_pretty(&index)?;
        fs::write(&index_path, index_json)?;
        
        Ok(())
    }
    
    pub fn count_symbols(&self) -> Result<u64> {
        let symbols_dir = self.root_path.join("symbols");
        let mut count = 0;
        
        if let Ok(entries) = fs::read_dir(symbols_dir) {
            for entry in entries {
                if let Ok(entry) = entry {
                    if entry.file_name().to_string_lossy().ends_with(".bin") {
                        count += 1;
                    }
                }
            }
        }
        
        Ok(count)
    }
    
    pub fn count_files(&self) -> Result<u64> {
        let files_dir = self.root_path.join("corpus/files");
        let mut count = 0;
        
        if let Ok(entries) = fs::read_dir(files_dir) {
            for entry in entries {
                if let Ok(entry) = entry {
                    if entry.file_name().to_string_lossy().ends_with(".meta.json") {
                        count += 1;
                    }
                }
            }
        }
        
        Ok(count)
    }
    
    pub fn list_symbols(&self) -> Result<Vec<String>> {
        let symbols_dir = self.root_path.join("symbols");
        let mut symbols = Vec::new();
        
        if let Ok(entries) = fs::read_dir(symbols_dir) {
            for entry in entries {
                if let Ok(entry) = entry {
                    let filename = entry.file_name().to_string_lossy().to_string();
                    // Handle both formats: sym_HASH.bin and direct HASH files
                    if filename.ends_with(".bin") {
                        if let Some(hash) = filename.strip_prefix("sym_").and_then(|s| s.strip_suffix(".bin")) {
                            symbols.push(hash.to_string());
                        }
                    } else if filename.len() == 32 && filename.chars().all(|c| c.is_ascii_hexdigit()) {
                        // Direct hash files (existing format)
                        symbols.push(filename);
                    }
                }
            }
        }
        
        Ok(symbols)
    }
    
    pub fn get_symbol_usage(&self, symbol_hash: &str) -> Result<crate::storage::symbols::SymbolUsage> {
        let usage_path = self.root_path.join("symbol_usage").join(symbol_hash);
        
        if !usage_path.exists() {
            return Ok(crate::storage::symbols::SymbolUsage {
                symbol_hash: symbol_hash.to_string(),
                total_bytes_contributed: 0,
                total_occurrences: 0,
                objects: std::collections::HashMap::new(),
            });
        }
        
        let data = fs::read(usage_path)?;
        Ok(bincode::deserialize(&data)?)
    }
    
    pub fn calculate_total_size(&self) -> Result<u64> {
        let symbols_dir = self.root_path.join("symbols");
        let mut total = 0;
        
        if let Ok(entries) = fs::read_dir(symbols_dir) {
            for entry in entries {
                if let Ok(entry) = entry {
                    if entry.file_name().to_string_lossy().ends_with(".bin") {
                        if let Ok(metadata) = entry.metadata() {
                            total += metadata.len();
                        }
                    }
                }
            }
        }
        
        Ok(total)
    }
    
    pub fn load_symbol(&self, symbol_hash: &str) -> Result<crate::storage::symbols::StoredSymbol> {
        // Try new format first
        let meta_path = self.root_path.join("symbols").join(format!("sym_{}.meta.json", symbol_hash));
        if meta_path.exists() {
            let meta_data = fs::read_to_string(meta_path)?;
            return Ok(serde_json::from_str(&meta_data)?);
        }
        
        // Fall back to existing format - create a minimal StoredSymbol from the hash file
        let symbol_path = self.root_path.join("symbols").join(symbol_hash);
        if symbol_path.exists() {
            let bytes = fs::read(&symbol_path)?;
            let content_hash = crate::engine::hash::sha256(&bytes);
            return Ok(crate::storage::symbols::StoredSymbol {
                hash: symbol_hash.to_string(),
                bytes,
                size: symbol_path.metadata()?.len(),
                first_seen: symbol_path.metadata()?.created()?
                    .duration_since(std::time::UNIX_EPOCH)?
                    .as_secs(),
                usage_count: 1,
                content_hash,
            });
        }
        
        Err(anyhow::anyhow!("Symbol not found: {}", symbol_hash))
    }
}