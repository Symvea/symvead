use tokio::fs;
use std::path::PathBuf;
use crate::storage::{StorageEngine, StoredObject, ObjectMetadata};

pub struct LocalStorage {
    root: PathBuf,
}

impl LocalStorage {
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }

    fn data_path(&self, key: &str) -> PathBuf {
        self.root.join("files").join(key)
    }

    fn meta_path(&self, key: &str) -> PathBuf {
        self.root.join("files").join(format!("{}.meta", key))
    }
}

#[async_trait::async_trait]
impl StorageEngine for LocalStorage {
    async fn put(
        &self,
        key: &str,
        data: &[u8],
        meta: &ObjectMetadata,
    ) -> anyhow::Result<()> {
        fs::create_dir_all(self.root.join("files")).await?;
        fs::write(self.data_path(key), data).await?;
        fs::write(
            self.meta_path(key),
            serde_json::to_vec(meta)?,
        ).await?;
        Ok(())
    }

    async fn get(
        &self,
        key: &str,
    ) -> anyhow::Result<Option<StoredObject>> {
        let data = match fs::read(self.data_path(key)).await {
            Ok(d) => d,
            Err(_) => return Ok(None),
        };

        let meta: ObjectMetadata =
            serde_json::from_slice(&fs::read(self.meta_path(key)).await?)?;

        Ok(Some(StoredObject {
            data,
            metadata: meta,
        }))
    }

    async fn delete(&self, key: &str) -> anyhow::Result<()> {
        let _ = fs::remove_file(self.data_path(key)).await;
        let _ = fs::remove_file(self.meta_path(key)).await;
        Ok(())
    }
}
