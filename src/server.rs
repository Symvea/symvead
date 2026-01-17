use tokio::net::TcpListener;
use tracing::{info, error};
use crate::session::Session;
use crate::storage::{
    local::LocalStorage,
    dictionary::Dictionary,
    symbols::SymbolStore,
};
use crate::coordination::CoordinationManager;
use crate::metrics::{MetricsCollector, start_metrics_server};
use std::sync::{Arc, Mutex};
use std::path::PathBuf;

pub async fn run() -> anyhow::Result<()> {
    run_on("0.0.0.0:24096", "./data").await
}

pub async fn run_on(addr: &str, data_dir: &str) -> anyhow::Result<()> {
    let listener = TcpListener::bind(addr).await?;
    info!("Server listening on {}", addr);

    // Create shared storage, dictionary, and symbol store
    let storage = Arc::new(LocalStorage::new(PathBuf::from(data_dir)));
    let coordination = Arc::new(CoordinationManager::new(data_dir));
    let metrics = Arc::new(MetricsCollector::new());
    
    // Start metrics server on port +1
    let metrics_addr = addr.replace(":24096", ":24097");
    let metrics_clone = Arc::clone(&metrics);
    let data_dir_clone = data_dir.to_string();
    tokio::spawn(async move {
        if let Err(e) = start_metrics_server(&metrics_addr, metrics_clone, data_dir_clone).await {
            error!("Metrics server failed: {}", e);
        }
    });
    
    // Try to load existing frozen dictionary with coordination
    let global_dict = Arc::new(Mutex::new({
        coordination.with_dictionary_lock(|| {
            let dict_dir = format!("{}", data_dir);
            let mut loaded_dict = None;
            if let Ok(entries) = std::fs::read_dir(&dict_dir) {
                for entry in entries.flatten() {
                    let filename = entry.file_name();
                    if let Some(name) = filename.to_str() {
                        if name.starts_with("dictionary_") && name.ends_with(".json") {
                            if let Ok(dict_json) = std::fs::read_to_string(entry.path()) {
                                if let Ok(dict) = serde_json::from_str::<Dictionary>(&dict_json) {
                                    info!("Loaded frozen dictionary: {}", name);
                                    loaded_dict = Some(dict);
                                    break;
                                }
                            }
                        }
                    }
                }
            }
            Ok(loaded_dict.unwrap_or_else(|| Dictionary::new("global")))
        }).unwrap_or_else(|_| Dictionary::new("global"))
    }));
    
    let symbol_store = Arc::new(SymbolStore::new(data_dir));

    loop {
        match listener.accept().await {
            Ok((socket, peer)) => {
                info!("New connection from {}", peer);
                metrics.connection_opened();
                
                let storage_clone = Arc::clone(&storage);
                let global_dict_clone = Arc::clone(&global_dict);
                let symbol_store_clone = Arc::clone(&symbol_store);
                let coordination_clone = Arc::clone(&coordination);
                let metrics_clone = Arc::clone(&metrics);
                let metrics_for_cleanup = Arc::clone(&metrics);
                
                tokio::spawn(async move {
                    let session = Session::new(
                        socket, 
                        storage_clone, 
                        global_dict_clone, 
                        symbol_store_clone, 
                        Some(coordination_clone),
                        Some(metrics_clone)
                    );
                    if let Err(e) = session.run().await {
                        error!("Session error for {}: {}", peer, e);
                    } else {
                        info!("Session completed for {}", peer);
                    }
                    metrics_for_cleanup.connection_closed();
                });
            }
            Err(e) => {
                error!("Failed to accept connection: {}", e);
            }
        }
    }
}
