use crc32fast::Hasher;

/// Compute CRC32 checksum for a byte slice
pub fn crc32(data: &[u8]) -> u32 {
    let mut hasher = Hasher::new();
    hasher.update(data);
    hasher.finalize()
}

/// Validate payload checksum
pub fn verify_crc32(data: &[u8], expected: u32) -> bool {
    crc32(data) == expected
}
