use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use serde::Serialize;
use tokio::net::TcpListener;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

#[derive(Debug, Clone, Serialize)]
pub struct Metrics {
    pub uptime_seconds: u64,
    pub total_uploads: u64,
    pub total_downloads: u64,
    pub total_bytes_stored: u64,
    pub total_bytes_served: u64,
    pub active_connections: u64,
    pub compression_ratio_avg: f64,
    pub symbols_count: u64,
    pub dictionary_frozen: bool,
}

pub struct MetricsCollector {
    start_time: std::time::SystemTime,
    uploads: AtomicU64,
    downloads: AtomicU64,
    bytes_stored: AtomicU64,
    bytes_served: AtomicU64,
    active_connections: AtomicU64,
    compression_ratios: Arc<std::sync::Mutex<Vec<f64>>>,
}

impl MetricsCollector {
    pub fn new() -> Self {
        Self {
            start_time: std::time::SystemTime::now(),
            uploads: AtomicU64::new(0),
            downloads: AtomicU64::new(0),
            bytes_stored: AtomicU64::new(0),
            bytes_served: AtomicU64::new(0),
            active_connections: AtomicU64::new(0),
            compression_ratios: Arc::new(std::sync::Mutex::new(Vec::new())),
        }
    }
    
    pub fn record_upload(&self, bytes: u64, compression_ratio: f64) {
        self.uploads.fetch_add(1, Ordering::Relaxed);
        self.bytes_stored.fetch_add(bytes, Ordering::Relaxed);
        if let Ok(mut ratios) = self.compression_ratios.lock() {
            ratios.push(compression_ratio);
            if ratios.len() > 1000 { // Keep last 1000 ratios
                ratios.remove(0);
            }
        }
    }
    
    pub fn record_download(&self, bytes: u64) {
        self.downloads.fetch_add(1, Ordering::Relaxed);
        self.bytes_served.fetch_add(bytes, Ordering::Relaxed);
    }
    
    pub fn connection_opened(&self) {
        self.active_connections.fetch_add(1, Ordering::Relaxed);
    }
    
    pub fn connection_closed(&self) {
        self.active_connections.fetch_sub(1, Ordering::Relaxed);
    }
    
    pub fn get_metrics(&self, symbols_count: u64, dictionary_frozen: bool) -> Metrics {
        let uptime = self.start_time.elapsed().unwrap_or_default().as_secs();
        let avg_ratio = if let Ok(ratios) = self.compression_ratios.lock() {
            if ratios.is_empty() { 0.0 } else { ratios.iter().sum::<f64>() / ratios.len() as f64 }
        } else { 0.0 };
        
        Metrics {
            uptime_seconds: uptime,
            total_uploads: self.uploads.load(Ordering::Relaxed),
            total_downloads: self.downloads.load(Ordering::Relaxed),
            total_bytes_stored: self.bytes_stored.load(Ordering::Relaxed),
            total_bytes_served: self.bytes_served.load(Ordering::Relaxed),
            active_connections: self.active_connections.load(Ordering::Relaxed),
            compression_ratio_avg: avg_ratio,
            symbols_count,
            dictionary_frozen,
        }
    }
}

pub async fn start_metrics_server(
    addr: &str, 
    metrics: Arc<MetricsCollector>,
    data_dir: String,
) -> anyhow::Result<()> {
    let listener = TcpListener::bind(addr).await?;
    tracing::info!("Metrics server listening on {}", addr);
    
    loop {
        let (mut socket, _) = listener.accept().await?;
        let metrics = Arc::clone(&metrics);
        let data_dir = data_dir.clone();
        
        tokio::spawn(async move {
            let mut buffer = [0; 1024];
            if let Ok(n) = socket.read(&mut buffer).await {
                let request = String::from_utf8_lossy(&buffer[..n]);
                
                if request.starts_with("GET /metrics") {
                    // Get symbol count from storage
                    let symbols_count = std::fs::read_dir(format!("{}/symbols", data_dir))
                        .map(|entries| entries.count() as u64)
                        .unwrap_or(0);
                    
                    // Check if dictionary is frozen
                    let dict_frozen = std::fs::read_dir(&data_dir)
                        .map(|mut entries| {
                            entries.any(|entry| {
                                entry.map(|e| e.file_name().to_string_lossy().starts_with("dictionary_"))
                                    .unwrap_or(false)
                            })
                        })
                        .unwrap_or(false);
                    
                    let metrics_data = metrics.get_metrics(symbols_count, dict_frozen);
                    let json = serde_json::to_string_pretty(&metrics_data).unwrap_or_default();
                    
                    let response = format!(
                        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
                        json.len(),
                        json
                    );
                    
                    let _ = socket.write_all(response.as_bytes()).await;
                } else if request.starts_with("GET /health") {
                    let response = "HTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\nOK";
                    let _ = socket.write_all(response.as_bytes()).await;
                }
            }
        });
    }
}