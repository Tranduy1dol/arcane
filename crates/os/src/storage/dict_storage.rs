use crate::storage::error::StorageError;
use crate::storage::storage::Storage;
use futures::FutureExt;
use std::collections::HashMap;

#[derive(Debug, Clone, Default, PartialEq)]
pub struct DictStorage {
    pub db: HashMap<Vec<u8>, Vec<u8>>,
}

impl Storage for DictStorage {
    async fn set_value(&mut self, key: Vec<u8>, value: Vec<u8>) -> Result<(), StorageError> {
        self.db.insert(key.to_vec(), value.to_vec());
        Ok(())
    }

    fn get_value(
        &self,
        key: &[u8],
    ) -> impl futures::Future<Output = Result<Option<Vec<u8>>, StorageError>> + Send {
        let result = Ok(self.db.get(key).cloned());
        async move { result }.boxed()
    }
}
