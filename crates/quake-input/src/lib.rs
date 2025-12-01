pub mod commands;

mod bindings;
mod mapping;

use crate::bindings::Bindings;
use crate::mapping::Mappings;
use tracing::log::trace;

#[derive(Debug, Default)]
pub struct InputManager {
    bindings: Bindings,
    mappings: Mappings,
}

impl InputManager {
    pub async fn on_key_pressed(&self, key: &str) -> Option<String> {
        trace!("Key pressed: {}", key);

        self.get_binding(key).await
    }

    pub async fn on_key_released(&self, key: &str) -> Option<String> {
        trace!("Key released: {}", key);

        self.get_binding(key).await
    }

    async fn get_binding(&self, key: &str) -> Option<String> {
        let key = self.mappings.get(key);
        self.bindings.get(&key).await
    }
}
