use std::collections::HashMap;
use tokio::sync::RwLock;

#[derive(Debug, Default)]
pub struct Bindings {
    bindings: RwLock<HashMap<String, String>>,
}

impl Bindings {
    pub async fn bind(&self, key: &str, binding: &str) {
        self.bindings
            .write()
            .await
            .insert(key.to_string(), binding.to_string());
    }

    pub async fn unbind(&self, key: &str) {
        self.bindings.write().await.remove(key);
    }

    pub async fn get(&self, key: &str) -> Option<String> {
        self.bindings.read().await.get(key).cloned()
    }

    pub async fn clear(&self) {
        self.bindings.write().await.clear();
    }

    pub async fn iter(&self) -> impl Iterator<Item = (String, String)> {
        let bindings = self.bindings.read().await;
        let data: Vec<(String, String)> = bindings
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
        data.into_iter()
    }
}
