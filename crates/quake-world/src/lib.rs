use crate::world::WorldEvent;
use serde::{Deserialize, Serialize};
use std::vec::Drain;

pub mod universe;
pub mod world;

mod component;
mod query;
mod system;

#[derive(Clone, Debug, Default)]
pub struct EventWriter {
    queue: Vec<WorldEvent>,
}

impl EventWriter {
    pub fn push(&mut self, event: WorldEvent) {
        self.queue.push(event);
    }

    pub fn commit(&mut self) -> CommittedEvents {
        CommittedEvents {
            queue: std::mem::take(&mut self.queue),
        }
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct CommittedEvents {
    queue: Vec<WorldEvent>,
}

#[derive(Clone, Debug)]
pub struct EventReader {
    queue: Vec<WorldEvent>,
}

impl EventReader {
    pub fn iter(&self) -> impl Iterator<Item = &WorldEvent> {
        self.queue.iter()
    }

    pub fn drain(&mut self) -> Drain<'_, WorldEvent> {
        self.queue.drain(..)
    }
}

impl From<CommittedEvents> for EventReader {
    fn from(events: CommittedEvents) -> Self {
        Self {
            queue: events.queue,
        }
    }
}
