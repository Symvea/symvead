mod session;
mod storage;
mod protocol;
mod utils;
mod engine;

use anyhow::Result;
use tokio::net::TcpListener;
use session::Session;
use storage::{local::LocalStorage, dictionary::Dictionary, symbols::SymbolStore};
use std::sync::{Arc, Mutex};
use std::path::PathBuf;
use tracing::{info, error, debug};

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter("symvead=trace,symvea=trace")
        .init();
    
    let listener = TcpListener::bind("0.0.0.0:24096").await?;
    info!("symvead listening on port 24096");

    // Create storage and global dictionary
    let storage = Arc::new(LocalStorage::new(PathBuf::from("./data")));
    let global_dict = Arc::new(Mutex::new(Dictionary::new("global")));
    let symbol_store = Arc::new(SymbolStore::new("./data"));
    debug!("Storage, dictionary, and symbol store initialized");

    loop {
        match listener.accept().await {
            Ok((socket, addr)) => {
                info!("connection from {}", addr);

                let storage_clone = Arc::clone(&storage);
                let global_dict_clone = Arc::clone(&global_dict);
                let symbol_store_clone = Arc::clone(&symbol_store);

                tokio::spawn(async move {
                    debug!("Starting session for {}", addr);
                    if let Err(e) = Session::new(socket, storage_clone, global_dict_clone, symbol_store_clone, None).run().await {
                        error!("session error for {}: {:?}", addr, e);
                    } else {
                        info!("session completed for {}", addr);
                    }
                });
            }
            Err(e) => {
                error!("Failed to accept connection: {}", e);
            }
        }
    }
}
