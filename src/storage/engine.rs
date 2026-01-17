use async_trait::async_trait;
use crate::storage::{StoredObject, ObjectMetadata};

#[async_trait]
pub trait StorageEngine: Send + Sync {
    async fn put(
        &self,
        key: &str,
        data: &[u8],
        meta: &ObjectMetadata,
    ) -> anyhow::Result<()>;

    async fn get(
        &self,
        key: &str,
    ) -> anyhow::Result<Option<StoredObject>>;

    async fn delete(
        &self,
        key: &str,
    ) -> anyhow::Result<()>;
}
