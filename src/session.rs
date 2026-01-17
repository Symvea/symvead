use tokio::net::TcpStream;
use std::sync::{Arc, Mutex};
use tracing::{info, error, warn};

use crate::protocol::{
    handshake::{read_handshake, write_handshake},
    frame::{Frame, read_frame, write_frame}, MAX_FILE_SIZE,
};
use crate::engine::{compress, decompress};
use crate::storage::{
    StorageEngine,
    dictionary::Dictionary,
    metadata::ObjectMetadata,
    symbols::SymbolStore,
};
use crate::engine::hash::sha256;
use crate::coordination::CoordinationManager;
use crate::metrics::MetricsCollector;

pub struct Session<S: StorageEngine> {
    stream: TcpStream,
    storage: Arc<S>,
    global_dict: Arc<Mutex<Dictionary>>,
    symbol_store: Arc<SymbolStore>,
    user_dict: Dictionary,
    coordination: Option<Arc<CoordinationManager>>,
    metrics: Option<Arc<MetricsCollector>>,
    // Chunked upload state
    chunked_uploads: std::collections::HashMap<String, ChunkedUpload>,
}

#[derive(Debug)]
struct ChunkedUpload {
    key: String,
    total_size: u64,
    chunk_count: u32,
    received_chunks: std::collections::HashMap<u32, Vec<u8>>,
    user_id: Option<String>,
}

impl<S: StorageEngine> Session<S> {
    pub fn new(
        stream: TcpStream,
        storage: Arc<S>,
        global_dict: Arc<Mutex<Dictionary>>,
        symbol_store: Arc<SymbolStore>,
        coordination: Option<Arc<CoordinationManager>>,
        metrics: Option<Arc<MetricsCollector>>,
    ) -> Self {
        Self {
            stream,
            storage,
            global_dict,
            symbol_store,
            user_dict: Dictionary::new("session".to_string()),
            coordination,
            metrics,
            chunked_uploads: std::collections::HashMap::new(),
        }
    }

    pub async fn run(mut self) -> anyhow::Result<()> {

        
        let _client = read_handshake(&mut self.stream).await?;
        
        write_handshake(&mut self.stream).await?;
        
        info!("Handshake completed, entering main loop");

        loop {

            let frame = match read_frame(&mut self.stream).await {
                Ok(f) => f,
                Err(_e) => {
                    break;
                }
            };

            match frame {
                Frame::Upload { key, data, user_id } => {
                    info!("Processing upload: key='{}', size={} bytes", key, data.len());
                    if let Err(e) = self.handle_upload(key.clone(), data, user_id).await {
                        error!("Upload failed for key '{}': {}", key, e);
                        return Err(e);
                    }
                }

                Frame::Download { key } => {
                    info!("Processing download: key='{}", key);
                    if let Err(e) = self.handle_download(key.clone()).await {
                        error!("Download failed for key '{}': {}", key, e);
                        return Err(e);
                    }
                }

                Frame::Verify { key } => {
                    info!("Processing verify: key='{}", key);
                    if let Err(e) = self.handle_verify(key.clone()).await {
                        error!("Verify failed for key '{}': {}", key, e);
                        return Err(e);
                    }
                }

                Frame::FreezeDictionary => {
                    info!("Freezing global dictionary");
                    if let Some(coord) = &self.coordination {
                        coord.with_dictionary_lock(|| {
                            let mut global_dict = self.global_dict.lock().unwrap();
                            if !global_dict.frozen {
                                let dict_id = global_dict.freeze();
                                info!("Dictionary frozen with ID: {}", dict_id);
                                
                                // Save frozen dictionary to disk
                                let dict_path = format!("{}/dictionary_{}.json", "./data", dict_id);
                                if let Ok(dict_json) = serde_json::to_string_pretty(&*global_dict) {
                                    let _ = std::fs::write(&dict_path, dict_json);
                                    info!("Dictionary saved to: {}", dict_path);
                                }
                            }
                            Ok(())
                        }).unwrap_or_else(|e| error!("Dictionary freeze coordination failed: {}", e));
                    } else {
                        let mut global_dict = self.global_dict.lock().unwrap();
                        if !global_dict.frozen {
                            let dict_id = global_dict.freeze();
                            info!("Dictionary frozen with ID: {}", dict_id);
                            
                            // Save frozen dictionary to disk
                            let dict_path = format!("{}/dictionary_{}.json", "./data", dict_id);
                            if let Ok(dict_json) = serde_json::to_string_pretty(&*global_dict) {
                                let _ = std::fs::write(&dict_path, dict_json);
                                info!("Dictionary saved to: {}", dict_path);
                            }
                        }
                    }
                }

                Frame::Close => {
                    info!("Client requested close");
                    break;
                }
                
                Frame::ChunkStart { key, total_size, chunk_count, user_id } => {
                    info!("Starting chunked upload: key='{}', total_size={}, chunks={}", key, total_size, chunk_count);
                    if total_size > MAX_FILE_SIZE as u64 {
                        error!("File too large: {} bytes (max: {})", total_size, MAX_FILE_SIZE);
                        return Err(anyhow::anyhow!("File too large"));
                    }
                    self.chunked_uploads.insert(key.clone(), ChunkedUpload {
                        key: key.clone(),
                        total_size,
                        chunk_count,
                        received_chunks: std::collections::HashMap::new(),
                        user_id,
                    });
                }
                
                Frame::ChunkData { key, chunk_index, data } => {
                    if let Some(upload) = self.chunked_uploads.get_mut(&key) {
                        upload.received_chunks.insert(chunk_index, data);
                        
                        // Check if all chunks received
                        if upload.received_chunks.len() == upload.chunk_count as usize {
                            info!("All chunks received for key '{}', assembling file", key);
                            if let Err(e) = self.handle_chunked_complete(key.clone()).await {
                                error!("Chunked upload assembly failed for key '{}': {}", key, e);
                                return Err(e);
                            }
                        }
                    } else {
                        error!("Received chunk data for unknown upload: {}", key);
                        return Err(anyhow::anyhow!("Unknown chunked upload"));
                    }
                }
                
                Frame::ChunkEnd { key: _ } => {
                    // ChunkEnd is optional - file is complete when all chunks received
                }
                
                Frame::Ack { .. } | Frame::Data { .. } | Frame::NotFound { .. } | Frame::Verified { .. } => {
                    warn!("Received response frame in server context, ignoring");
                }
            }
        }

        info!("Session ended");
        Ok(())
    }

    async fn handle_upload(
        &mut self,
        key: String,
        data: Vec<u8>,
        user_id: Option<String>,
    ) -> anyhow::Result<()> {
        let original_size = data.len() as u64;
        let content_hash = sha256(&data);
        let original_hash = content_hash;

        let (compressed_data, dict_id, symbol_infos, explained_ratio, token_breakdown) = {
            let mut global_dict = self.global_dict.lock().unwrap();
            let (compressed, symbols, ratio, breakdown) = compress(&data, &mut global_dict, &self.symbol_store, &key);
            let dict_id = if global_dict.frozen {
                global_dict.id.clone()
            } else {
                "mutable".to_string()
            };
            (compressed, dict_id, symbols, ratio, breakdown)
        };

        let compressed_size = compressed_data.len() as u64;
        info!("Upload: {} -> {} bytes ({:.1}%), explained: {:.1}%", 
              original_size, compressed_size, 
              (compressed_size as f64 / original_size as f64) * 100.0,
              explained_ratio * 100.0);

        let mut meta = ObjectMetadata::new(
            key.clone(),
            content_hash,
            original_hash,
            dict_id,
            original_size,
            compressed_size,
            user_id,
        );
        
        meta.symbols = symbol_infos;
        meta.explained_ratio = explained_ratio;
        meta.token_breakdown = token_breakdown;

        self.storage.put(&key, &compressed_data, &meta).await?;
        
        // Record metrics
        if let Some(metrics) = &self.metrics {
            let compression_ratio = 1.0 - (compressed_size as f64 / original_size as f64);
            metrics.record_upload(original_size, compression_ratio);
        }
        
        write_frame(
            &mut self.stream,
            Frame::Ack {
                key: key.clone(),
                original_size,
                compressed_size,
            },
        )
        .await?;
        
        Ok(())
    }

    pub async fn handle_download(&mut self, key: String) -> anyhow::Result<()> {
        let Some(obj) = self.storage.get(&key).await? else {
            warn!("Key not found: {}", key);
            write_frame(&mut self.stream, Frame::NotFound { key }).await?;
            return Ok(());
        };
        
        // Use the global dictionary for decompression
        let data = {
            let global_dict = self.global_dict.lock().unwrap();
            decompress(&obj.data, &global_dict)
        };
        
        info!("Decompressed to {} bytes", data.len());
        
        // Record metrics
        if let Some(metrics) = &self.metrics {
            metrics.record_download(data.len() as u64);
        }

        write_frame(
            &mut self.stream,
            Frame::Data {
                key: key.clone(),
                data,
            },
        )
        .await?;
        
        info!("Download completed for key: {}", key);
        Ok(())
    }
    
    pub async fn handle_verify(&mut self, key: String) -> anyhow::Result<()> {
        let Some(obj) = self.storage.get(&key).await? else {
            warn!("Key not found for verification: {}", key);
            write_frame(&mut self.stream, Frame::NotFound { key }).await?;
            return Ok(());
        };
        
        // Decompress the data
        let data = {
            let global_dict = self.global_dict.lock().unwrap();
            decompress(&obj.data, &global_dict)
        };
        
        // Hash the reconstructed data
        let reconstructed_hash = sha256(&data);
        
        // Compare with stored original hash
        let hash_match = reconstructed_hash == obj.metadata.original_hash;
        
        if !hash_match {
            error!("CORRUPTION DETECTED for key '{}': hash mismatch", key);
            panic!("FATAL: CORRUPTION DETECTED - refusing service");
        }
        
        info!("Verification completed for key '{}': hash_match={}", key, hash_match);
        
        write_frame(
            &mut self.stream,
            Frame::Verified {
                key: key.clone(),
                hash_match,
            },
        )
        .await?;
        
        Ok(())
    }
    
    async fn handle_chunked_complete(&mut self, key: String) -> anyhow::Result<()> {
        let upload = self.chunked_uploads.remove(&key)
            .ok_or_else(|| anyhow::anyhow!("Chunked upload not found"))?;
        
        // Assemble chunks in order
        let mut data = Vec::with_capacity(upload.total_size as usize);
        for i in 0..upload.chunk_count {
            if let Some(chunk) = upload.received_chunks.get(&i) {
                data.extend_from_slice(chunk);
            } else {
                return Err(anyhow::anyhow!("Missing chunk {}", i));
            }
        }
        
        if data.len() != upload.total_size as usize {
            return Err(anyhow::anyhow!("Size mismatch: expected {}, got {}", upload.total_size, data.len()));
        }
        
        info!("Assembled chunked upload: key='{}', size={} bytes", key, data.len());
        
        // Process as normal upload
        self.handle_upload(key, data, upload.user_id).await
    }
}
