use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Hash)]
pub struct EntityId(pub u64);
