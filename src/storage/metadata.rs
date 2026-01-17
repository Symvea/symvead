use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TokenKind {
    Symbol {
        hash: String,
        len: usize,
    },
    Literal {
        len: usize,
        reason: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SymbolInfo {
    pub hash: String,
    pub bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenBreakdown {
    pub symbol_bytes: u64,
    pub literal_bytes: u64,
    pub literal_reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObjectMetadata {
    pub key: String,
    pub object_hash: [u8; 32],
    pub original_hash: [u8; 32], // Phase 3: Lossless proof
    pub dict_id: String,
    pub engine_version: String,
    pub original_size: u64,
    pub compressed_size: u64,
    pub stored_at: u64,
    pub user_id: Option<String>,
    pub codec_version: u16,
    // Phase 2: symbolic indexing
    pub symbols: Vec<SymbolInfo>,
    pub explained_ratio: f64,
    pub token_breakdown: TokenBreakdown,
}

impl ObjectMetadata {
    pub fn new(
        key: String,
        object_hash: [u8; 32],
        original_hash: [u8; 32],
        dict_id: String,
        original_size: u64,
        compressed_size: u64,
        user_id: Option<String>,
    ) -> Self {
        Self {
            key,
            object_hash,
            original_hash,
            dict_id,
            engine_version: "symvea-engine@0.1.0".to_string(),
            original_size,
            compressed_size,
            stored_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            user_id,
            codec_version: 1,
            symbols: Vec::new(),
            explained_ratio: 0.0,
            token_breakdown: TokenBreakdown {
                symbol_bytes: 0,
                literal_bytes: 0,
                literal_reason: "Below promotion threshold".to_string(),
            },
        }
    }
    
    pub fn verify_integrity(&self, reconstructed_data: &[u8]) -> bool {
        use crate::engine::hash::sha256;
        let computed_hash = sha256(reconstructed_data);
        computed_hash == self.object_hash
    }
}
