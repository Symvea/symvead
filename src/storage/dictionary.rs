use std::collections::HashMap;
use serde::{Serialize, Deserialize};
use crate::engine::hash::sha256;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Dictionary {
    pub id: String,
    pub encode: HashMap<Vec<u8>, u32>,
    pub decode: HashMap<u32, Vec<u8>>,
    pub frozen: bool,
    pub created_at: u64,
    pub frozen_at: Option<u64>,
    pub version: String,
}

impl Dictionary {
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            encode: HashMap::new(),
            decode: HashMap::new(),
            frozen: false,
            created_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            frozen_at: None,
            version: "symvea-engine@0.1.0".to_string(),
        }
    }
    
    pub fn freeze(&mut self) -> String {
        if self.frozen {
            return self.compute_hash();
        }
        
        self.frozen = true;
        self.frozen_at = Some(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs()
        );
        
        let dict_id = self.compute_hash();
        self.id = dict_id.clone();
        dict_id
    }
    
    pub fn compute_hash(&self) -> String {
        let serialized = bincode::serialize(self).unwrap();
        let hash = sha256(&serialized);
        hex::encode(&hash[..16])
    }
    
    pub fn serialize(&self) -> Vec<u8> {
        bincode::serialize(self).unwrap()
    }
    
    pub fn deserialize(data: &[u8]) -> Result<Self, Box<dyn std::error::Error>> {
        Ok(bincode::deserialize(data)?)
    }
}
