use anyhow::Result;
use crate::storage::PersistentStorage;

pub struct LayeredStorage {
    writable: PersistentStorage,
    readonly_mounts: Vec<PersistentStorage>,
}

impl LayeredStorage {
    pub fn new(writable_path: &str, readonly_paths: &[String]) -> Result<Self> {
        let writable = PersistentStorage::new(writable_path)?;
        let mut readonly_mounts = Vec::new();
        
        for path in readonly_paths {
            readonly_mounts.push(PersistentStorage::new(path)?);
        }
        
        Ok(Self {
            writable,
            readonly_mounts,
        })
    }
    
    pub fn list_symbols(&self) -> Result<Vec<String>> {
        let mut all_symbols = self.writable.list_symbols()?;
        
        for readonly in &self.readonly_mounts {
            let readonly_symbols = readonly.list_symbols()?;
            all_symbols.extend(readonly_symbols);
        }
        
        all_symbols.sort();
        all_symbols.dedup();
        Ok(all_symbols)
    }
    
    pub fn load_symbol(&self, symbol_hash: &str) -> Result<crate::storage::symbols::StoredSymbol> {
        // Try writable first
        if let Ok(symbol) = self.writable.load_symbol(symbol_hash) {
            return Ok(symbol);
        }
        
        // Try readonly mounts
        for readonly in &self.readonly_mounts {
            if let Ok(symbol) = readonly.load_symbol(symbol_hash) {
                return Ok(symbol);
            }
        }
        
        Err(anyhow::anyhow!("Symbol not found: {}", symbol_hash))
    }
    
    pub fn store_symbol(&self, hash: &str, bytes: &[u8], metadata: &crate::storage::symbols::StoredSymbol) -> Result<()> {
        // Only store to writable storage
        self.writable.store_symbol_persistent(hash, bytes, metadata)
    }
    
    pub fn count_symbols(&self) -> Result<u64> {
        let symbols = self.list_symbols()?;
        Ok(symbols.len() as u64)
    }
}