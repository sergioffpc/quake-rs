use std::sync::atomic::{AtomicU64, Ordering};

use serde::{Deserialize, Serialize};

static PLAYER_ID_GENERATOR: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PlayerId(u64);

impl PlayerId {
    pub fn new() -> Self {
        Self(PLAYER_ID_GENERATOR.fetch_add(1, Ordering::Relaxed))
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Hash)]
pub struct EntityId(pub u64);
