use serde::{Deserialize, Serialize};
use std::fmt::Display;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct EntityId(usize);

impl From<usize> for EntityId {
    fn from(value: usize) -> Self {
        Self(value)
    }
}

impl From<EntityId> for usize {
    fn from(value: EntityId) -> Self {
        value.0
    }
}

impl Display for EntityId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Dirty;

#[derive(Debug, Default, Serialize, Deserialize, Clone, Copy, PartialEq)]
pub struct Transform {
    pub position: glam::Vec3,
}
