use std::fs::{File, OpenOptions};
use std::io::Write;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use anyhow::Result;

pub struct FileLock {
    file: File,
    path: String,
}

impl FileLock {
    pub fn acquire(lock_path: &str, timeout_secs: u64) -> Result<Self> {
        let start = SystemTime::now();
        let timeout = Duration::from_secs(timeout_secs);
        
        loop {
            match Self::try_acquire(lock_path) {
                Ok(lock) => return Ok(lock),
                Err(_) if start.elapsed().unwrap_or(timeout) < timeout => {
                    std::thread::sleep(Duration::from_millis(100));
                    continue;
                }
                Err(e) => return Err(e),
            }
        }
    }
    
    fn try_acquire(lock_path: &str) -> Result<Self> {
        let file = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(lock_path)?;
            
        // Write current timestamp and process info
        let mut lock_file = file;
        let timestamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
        let pid = std::process::id();
        let lock_info = format!("{}:{}", timestamp, pid);
        lock_file.write_all(lock_info.as_bytes())?;
        lock_file.flush()?;
        
        Ok(FileLock {
            file: lock_file,
            path: lock_path.to_string(),
        })
    }
}

impl Drop for FileLock {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
    }
}

pub struct CoordinationManager {
    data_dir: String,
}

impl CoordinationManager {
    pub fn new(data_dir: &str) -> Self {
        Self {
            data_dir: data_dir.to_string(),
        }
    }
    
    pub fn with_dictionary_lock<F, R>(&self, f: F) -> Result<R>
    where
        F: FnOnce() -> Result<R>,
    {
        let lock_path = format!("{}/.dict_lock", self.data_dir);
        let _lock = FileLock::acquire(&lock_path, 30)?;
        f()
    }
    
    pub fn with_symbol_lock<F, R>(&self, symbol_hash: &str, f: F) -> Result<R>
    where
        F: FnOnce() -> Result<R>,
    {
        let lock_path = format!("{}/.symbol_lock_{}", self.data_dir, &symbol_hash[..8]);
        let _lock = FileLock::acquire(&lock_path, 10)?;
        f()
    }
}