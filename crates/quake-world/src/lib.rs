use crate::world::WorldEvent;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

pub mod universe;
pub mod world;

mod component;
mod query;
mod system;

#[derive(Clone, Debug, Default)]
pub struct EventWriter {
    queue: VecDeque<WorldEvent>,
}

impl EventWriter {
    pub fn push(&mut self, event: WorldEvent) {
        self.queue.push_back(event);
    }

    pub fn commit(&mut self) -> CommittedEvents {
        CommittedEvents {
            queue: std::mem::take(&mut self.queue),
        }
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct CommittedEvents {
    queue: VecDeque<WorldEvent>,
}

#[derive(Clone, Debug)]
pub struct EventReader {
    queue: VecDeque<WorldEvent>,
}

impl Iterator for EventReader {
    type Item = WorldEvent;

    fn next(&mut self) -> Option<Self::Item> {
        self.queue.pop_front()
    }
}

impl From<CommittedEvents> for EventReader {
    fn from(events: CommittedEvents) -> Self {
        Self {
            queue: events.queue,
        }
    }
}
