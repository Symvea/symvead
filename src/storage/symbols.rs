use std::collections::HashMap;
use std::fs;
use std::path::Path;
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredSymbol {
    pub hash: String,
    pub bytes: Vec<u8>,
    pub size: u64,
    pub first_seen: u64,
    pub usage_count: u64,
    pub content_hash: [u8; 32], // Phase 3: Immutability proof
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SymbolUsage {
    pub symbol_hash: String,
    pub total_bytes_contributed: u64,
    pub total_occurrences: u64,
    pub objects: HashMap<String, u64>, // object_id → count
}

pub struct SymbolStore {
    data_dir: String,
}

impl SymbolStore {
    pub fn new(data_dir: impl Into<String>) -> Self {
        let data_dir = data_dir.into();
        let symbols_dir = format!("{}/symbols", data_dir);
        let usage_dir = format!("{}/symbol_usage", data_dir);
        
        fs::create_dir_all(&symbols_dir).ok();
        fs::create_dir_all(&usage_dir).ok();
        
        let store = Self { data_dir };
        
        // Phase 3: Verify all symbols on startup
        if let Err(e) = store.verify_all_symbols() {
            panic!("FATAL: SYMBOL CORRUPTION DETECTED: {}", e);
        }
        
        store
    }
    
    pub fn store_symbol(&self, hash: &str, bytes: &[u8]) -> Result<(), Box<dyn std::error::Error>> {
        let symbol_path = format!("{}/symbols/{}", self.data_dir, hash);
        
        if !Path::new(&symbol_path).exists() {
            let content_hash = crate::engine::hash::sha256(bytes);
            let symbol = StoredSymbol {
                hash: hash.to_string(),
                bytes: bytes.to_vec(),
                size: bytes.len() as u64,
                first_seen: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)?
                    .as_secs(),
                usage_count: 1,
                content_hash,
            };
            
            let serialized = bincode::serialize(&symbol)?;
            fs::write(symbol_path, serialized)?;
        } else {
            // Increment usage count
            let mut symbol: StoredSymbol = self.load_symbol(hash)?;
            symbol.usage_count += 1;
            let serialized = bincode::serialize(&symbol)?;
            fs::write(format!("{}/symbols/{}", self.data_dir, hash), serialized)?;
        }
        
        Ok(())
    }
    
    pub fn load_symbol(&self, hash: &str) -> Result<StoredSymbol, Box<dyn std::error::Error>> {
        let symbol_path = format!("{}/symbols/{}", self.data_dir, hash);
        let data = fs::read(symbol_path)?;
        Ok(bincode::deserialize(&data)?)
    }
    
    pub fn add_usage(&self, symbol_hash: &str, object_key: &str, symbol_bytes: u64, occurrence_count: u64) -> Result<(), Box<dyn std::error::Error>> {
        let usage_path = format!("{}/symbol_usage/{}", self.data_dir, symbol_hash);
        
        let mut usage = if Path::new(&usage_path).exists() {
            let data = fs::read(&usage_path)?;
            bincode::deserialize::<SymbolUsage>(&data)?
        } else {
            SymbolUsage {
                symbol_hash: symbol_hash.to_string(),
                total_bytes_contributed: 0,
                total_occurrences: 0,
                objects: HashMap::new(),
            }
        };
        
        let prev_count = *usage.objects.get(object_key).unwrap_or(&0);
        usage.objects.insert(object_key.to_string(), occurrence_count);
        
        // Update totals
        usage.total_occurrences = usage.total_occurrences - prev_count + occurrence_count;
        usage.total_bytes_contributed = usage.total_occurrences * symbol_bytes;
        
        let serialized = bincode::serialize(&usage)?;
        fs::write(usage_path, serialized)?;
        
        Ok(())
    }
    
    pub fn get_corpus_usage(&self, symbol_hash: &str) -> Result<SymbolUsage, Box<dyn std::error::Error>> {
        let usage_path = format!("{}/symbol_usage/{}", self.data_dir, symbol_hash);
        
        if !Path::new(&usage_path).exists() {
            return Ok(SymbolUsage {
                symbol_hash: symbol_hash.to_string(),
                total_bytes_contributed: 0,
                total_occurrences: 0,
                objects: HashMap::new(),
            });
        }
        
        let data = fs::read(usage_path)?;
        Ok(bincode::deserialize(&data)?)
    }
    
    pub fn get_corpus_stats(&self) -> Result<(u64, u64), Box<dyn std::error::Error>> {
        let symbols_dir = format!("{}/symbols", self.data_dir);
        let mut total_symbols = 0;
        let mut total_bytes = 0;
        
        if let Ok(entries) = fs::read_dir(symbols_dir) {
            for entry in entries {
                if let Ok(entry) = entry {
                    if let Ok(symbol) = self.load_symbol(&entry.file_name().to_string_lossy()) {
                        total_symbols += 1;
                        total_bytes += symbol.size;
                    }
                }
            }
        }
        
        Ok((total_symbols, total_bytes))
    }
    
    pub fn verify_symbol_integrity(&self, hash: &str) -> Result<bool, Box<dyn std::error::Error>> {
        let symbol = self.load_symbol(hash)?;
        let computed_hash = crate::engine::hash::sha256(&symbol.bytes);
        Ok(computed_hash == symbol.content_hash)
    }
    
    pub fn verify_all_symbols(&self) -> Result<(), Box<dyn std::error::Error>> {
        let symbols_dir = format!("{}/symbols", self.data_dir);
        
        if let Ok(entries) = fs::read_dir(symbols_dir) {
            for entry in entries {
                if let Ok(entry) = entry {
                    let hash = entry.file_name().to_string_lossy().to_string();
                    if !self.verify_symbol_integrity(&hash)? {
                        return Err(format!("Symbol corruption detected: {}", hash).into());
                    }
                }
            }
        }
        
        Ok(())
    }
}