use serde::{Serialize, Deserialize};
use anyhow::Result;
use std::fs;

#[derive(Debug, Serialize, Deserialize)]
pub struct Snapshot {
    pub epoch: u64,
    pub timestamp: u64,
    pub symbols: Vec<SymbolRef>,
    pub files: Vec<FileRef>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SymbolRef {
    pub id: String,
    pub hash: String,
    pub size: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FileRef {
    pub key: String,
    pub symbols: Vec<String>,
    pub original_hash: String,
}

pub struct SnapshotManager {
    data_path: String,
}

impl SnapshotManager {
    pub fn new(data_path: impl Into<String>) -> Self {
        Self {
            data_path: data_path.into(),
        }
    }
    
    pub fn create_snapshot(&self) -> Result<Snapshot> {
        let epoch = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_secs();
            
        let symbols = self.collect_symbol_refs()?;
        let files = self.collect_file_refs()?;
        
        let snapshot = Snapshot {
            epoch,
            timestamp: epoch,
            symbols,
            files,
        };
        
        // Save snapshot to disk
        std::fs::create_dir_all(format!("{}/snapshots", self.data_path))?;
        let snapshot_path = format!("{}/snapshots/snapshot_{}.json", self.data_path, epoch);
        let snapshot_json = serde_json::to_string_pretty(&snapshot)?;
        fs::write(&snapshot_path, snapshot_json)?;
        
        Ok(snapshot)
    }
    
    pub fn load_latest_snapshot(&self) -> Result<Option<Snapshot>> {
        let snapshots_dir = format!("{}/snapshots", self.data_path);
        
        if !std::path::Path::new(&snapshots_dir).exists() {
            return Ok(None);
        }
        
        let mut latest_epoch = 0;
        let mut latest_path = None;
        
        for entry in fs::read_dir(&snapshots_dir)? {
            let entry = entry?;
            let filename = entry.file_name();
            let filename_str = filename.to_string_lossy();
            
            if filename_str.starts_with("snapshot_") && filename_str.ends_with(".json") {
                if let Some(epoch_str) = filename_str.strip_prefix("snapshot_").and_then(|s| s.strip_suffix(".json")) {
                    if let Ok(epoch) = epoch_str.parse::<u64>() {
                        if epoch > latest_epoch {
                            latest_epoch = epoch;
                            latest_path = Some(entry.path());
                        }
                    }
                }
            }
        }
        
        if let Some(path) = latest_path {
            let snapshot_json = fs::read_to_string(&path)?;
            let snapshot: Snapshot = serde_json::from_str(&snapshot_json)?;
            Ok(Some(snapshot))
        } else {
            Ok(None)
        }
    }
    
    pub fn restore_snapshot(&self, snapshot_path: &str) -> Result<()> {
        let snapshot_json = fs::read_to_string(snapshot_path)?;
        let snapshot: Snapshot = serde_json::from_str(&snapshot_json)?;
        
        println!("Restoring {} symbols and {} files from epoch {}", 
                snapshot.symbols.len(), snapshot.files.len(), snapshot.epoch);
        
        // Note: This is a basic restore that just validates the snapshot exists
        // Full restore would need to recreate symbol files and verify file integrity
        // For now, we just confirm the snapshot is valid
        
        Ok(())
    }
    
    fn collect_symbol_refs(&self) -> Result<Vec<SymbolRef>> {
        let mut symbols = Vec::new();
        let symbols_dir = format!("{}/symbols", self.data_path);
        
        if let Ok(entries) = fs::read_dir(&symbols_dir) {
            for entry in entries {
                if let Ok(entry) = entry {
                    let path = entry.path();
                    if let Some(filename) = path.file_name().and_then(|n| n.to_str()) {
                        if filename.starts_with("sym_") && filename.ends_with(".bin") {
                            if let Ok(data) = fs::read(&path) {
                                let hash = filename.strip_prefix("sym_").unwrap().strip_suffix(".bin").unwrap();
                                symbols.push(SymbolRef {
                                    id: format!("sym:{}", &hash[..8]),
                                    hash: hash.to_string(),
                                    size: data.len() as u64,
                                });
                            }
                        }
                    }
                }
            }
        }
        
        Ok(symbols)
    }
    
    fn collect_file_refs(&self) -> Result<Vec<FileRef>> {
        let mut files = Vec::new();
        let files_dir = format!("{}/files", self.data_path);
        
        if let Ok(entries) = fs::read_dir(&files_dir) {
            for entry in entries {
                if let Ok(entry) = entry {
                    let path = entry.path();
                    if let Some(filename) = path.file_name().and_then(|n| n.to_str()) {
                        if filename.ends_with(".meta") {
                            if let Ok(meta_json) = fs::read_to_string(&path) {
                                if let Ok(file_meta) = serde_json::from_str::<crate::storage::metadata::ObjectMetadata>(&meta_json) {
                                    let symbol_hashes: Vec<String> = file_meta.symbols.iter()
                                        .map(|s| s.hash.clone())
                                        .collect();
                                    
                                    files.push(FileRef {
                                        key: file_meta.key.clone(),
                                        symbols: symbol_hashes,
                                        original_hash: hex::encode(&file_meta.original_hash),
                                    });
                                }
                            }
                        }
                    }
                }
            }
        }
        
        Ok(files)
    }
}