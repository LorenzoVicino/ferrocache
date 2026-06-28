use std::collections::HashMap;

use tokio::sync::RwLock;

#[derive(Debug, Default)]
pub struct MemoryStore {
    entries: RwLock<HashMap<String, Vec<u8>>>,
}

impl MemoryStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub async fn set(&self, key: String, value: Vec<u8>) {
        self.entries.write().await.insert(key, value);
    }

    pub async fn get(&self, key: &str) -> Option<Vec<u8>> {
        self.entries.read().await.get(key).cloned()
    }

    pub async fn del(&self, keys: &[String]) -> usize {
        let mut entries = self.entries.write().await;
        keys.iter().filter(|key| entries.remove(*key).is_some()).count()
    }

    pub async fn exists(&self, keys: &[String]) -> usize {
        let entries = self.entries.read().await;
        keys.iter().filter(|key| entries.contains_key(*key)).count()
    }
}

