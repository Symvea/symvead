//! Protocol constants for Symvea network communication

#![allow(dead_code)] // Protocol constants may not all be used yet

/// SYMVEA protocol magic bytes: "SYMV"
pub const SYMVEA_MAGIC: [u8; 4] = *b"SYMV";

/// Current protocol version
pub const PROTOCOL_VERSION: u16 = 1;

/// Frame types
pub const FRAME_HANDSHAKE: u8 = 0x01;
pub const FRAME_INGEST: u8 = 0x02;
pub const FRAME_QUERY: u8 = 0x03;
pub const FRAME_RESPONSE: u8 = 0x04;
pub const FRAME_ERROR: u8 = 0x7F;

/// Chunked upload frames
pub const FRAME_CHUNK_START: u8 = 0x10;
pub const FRAME_CHUNK_DATA: u8 = 0x11;
pub const FRAME_CHUNK_END: u8 = 0x12;

/// Hard safety limits
pub const MAX_FRAME_SIZE: usize = usize::MAX; // No limit
pub const MAX_HEADER_SIZE: usize = usize::MAX; // No limit

/// Chunking for large files
pub const CHUNK_SIZE: usize = 32 * 1024 * 1024; // 32MB chunks
pub const MAX_FILE_SIZE: usize = usize::MAX; // No limit

/// Feature flags (bitmask)
pub const FLAG_COMPRESSED: u16 = 0x0001;
pub const FLAG_ENCRYPTED: u16 = 0x0002;
pub const FLAG_DICTIONARY: u16 = 0x0004;
