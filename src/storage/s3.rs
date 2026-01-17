use crate::storage::{StorageEngine, StoredObject, ObjectMetadata};

pub struct S3Storage {
    pub bucket: String,
}

#[async_trait::async_trait]
impl StorageEngine for S3Storage {
    async fn put(
        &self,
        _key: &str,
        _data: &[u8],
        _meta: &ObjectMetadata,
    ) -> anyhow::Result<()> {
        unimplemented!("S3 backend not yet implemented");
    }

    async fn get(
        &self,
        _key: &str,
    ) -> anyhow::Result<Option<StoredObject>> {
        unimplemented!("S3 backend not yet implemented");
    }

    async fn delete(
        &self,
        _key: &str,
    ) -> anyhow::Result<()> {
        unimplemented!("S3 backend not yet implemented");
    }
}
