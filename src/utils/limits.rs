/// Maximum dictionary entries per scope
pub const MAX_DICT_ENTRIES: usize = 10_000_000;

/// Maximum symbol length
pub const MAX_SYMBOL_SIZE: usize = 256;

/// Maximum ingest chunk size (streaming)
pub const MAX_INGEST_CHUNK: usize = 8 * 1024 * 1024; // 8MB

/// Maximum concurrent frames per connection
pub const MAX_INFLIGHT_FRAMES: usize = 128;
