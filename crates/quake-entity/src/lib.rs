use std::any::Any;
use std::collections::HashMap;

#[derive(Debug, Default)]
pub struct PlayerState {
    properties: HashMap<String, Box<dyn Any + Send + Sync + 'static>>,
}

impl PlayerState {
    pub fn set<T>(&mut self, key: &str, value: T)
    where
        T: Any + Send + Sync + 'static,
    {
        self.properties.insert(key.to_string(), Box::new(value));
    }

    pub fn get<T>(&self, key: &str) -> Option<&T>
    where
        T: Any + Send + Sync + 'static,
    {
        self.properties.get(key).and_then(|v| v.downcast_ref::<T>())
    }
}
