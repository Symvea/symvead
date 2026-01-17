use crate::engine::hash::sha256;

#[derive(Debug, Clone)]
pub struct Symbol {
    pub bytes: Vec<u8>,
    pub token: u32,
    pub gain: isize,
    pub hash: String, // Phase 2: addressable symbol hash
}

impl Symbol {
    pub fn new(bytes: Vec<u8>, token: u32, gain: isize) -> Self {
        let hash = hex::encode(&sha256(&bytes)[..16]);
        Self { bytes, token, gain, hash }
    }
    
    pub fn symbol_id(&self) -> String {
        format!("sym:{}", self.hash)
    }
}
