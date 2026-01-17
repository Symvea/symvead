use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use anyhow::Result;

#[allow(dead_code)] // Protocol constants for future use
pub const SYMVEA_PORT: u16 = 24096;
pub const MAX_FRAME_SIZE: usize = 1024 * 1024 * 1024; // 1GB
pub const PROTOCOL_VERSION: u16 = 1;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    pub data_directory: PathBuf,
    pub listen_address: String,
    pub readonly_mounts: Vec<PathBuf>,
    pub auto_create_directories: bool,
    pub max_file_size: usize,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            data_directory: PathBuf::from("./data"),
            listen_address: "0.0.0.0:24096".to_string(),
            readonly_mounts: Vec::new(),
            auto_create_directories: true,
            max_file_size: MAX_FRAME_SIZE,
        }
    }
}

impl ServerConfig {
    pub fn load_or_create(config_path: Option<&str>) -> Result<Self> {
        let config_file = config_path.unwrap_or("symvea.toml");
        
        if std::path::Path::new(config_file).exists() {
            let content = std::fs::read_to_string(config_file)?;
            let config: ServerConfig = toml::from_str(&content)?;
            Ok(config)
        } else {
            let config = Self::default();
            config.save(config_file)?;
            Ok(config)
        }
    }
    
    pub fn save(&self, config_path: &str) -> Result<()> {
        let content = toml::to_string_pretty(self)?;
        std::fs::write(config_path, content)?;
        Ok(())
    }
    
    pub fn ensure_directories(&self) -> Result<()> {
        if self.auto_create_directories {
            if !self.data_directory.exists() {
                std::fs::create_dir_all(&self.data_directory)?;
                tracing::info!("Created data directory: {:?}", self.data_directory);
            }
            
            for mount in &self.readonly_mounts {
                if !mount.exists() {
                    if let Some(parent) = mount.parent() {
                        std::fs::create_dir_all(parent)?;
                        tracing::info!("Created mount directory: {:?}", mount);
                    }
                }
            }
        }
        Ok(())
    }
}
