use crate::storage::metadata::ObjectMetadata;

#[derive(Debug)]
pub struct StoredObject {
    pub data: Vec<u8>,
    pub metadata: ObjectMetadata,
}
