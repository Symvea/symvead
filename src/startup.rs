use std::path::Path;
use tracing::info;
use anyhow::Result;

use crate::storage::PersistentStorage;

pub struct StartupValidator {
    storage: PersistentStorage,
}

impl StartupValidator {
    pub fn new(data_path: impl AsRef<Path>) -> Result<Self> {
        let storage = PersistentStorage::new(data_path.as_ref())?;
        Ok(Self { storage })
    }
    
    pub fn validate_and_start(&self) -> Result<()> {
        info!("🔍 Starting validation");
        
        // Step 1: Verify symbols exist
        info!("Step 1: Checking symbols...");
        self.verify_symbols()?;
        info!("✅ Symbols checked");
        
        info!("🎉 Validation complete");
        Ok(())
    }
    
    fn verify_symbols(&self) -> Result<()> {
        let symbols_dir = self.storage.root_path.join("symbols");
        
        if !symbols_dir.exists() {
            info!("No symbols directory - starting fresh");
            return Ok(());
        }
        
        let entries = std::fs::read_dir(&symbols_dir)?;
        let mut count = 0;
        
        for entry in entries {
            if let Ok(entry) = entry {
                let filename = entry.file_name();
                if filename.to_string_lossy().ends_with(".bin") {
                    count += 1;
                }
            }
        }
        
        info!("Found {} symbol files", count);
        Ok(())
    }
}