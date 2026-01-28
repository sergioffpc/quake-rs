use crate::{Intent, Source, TimedSource};
use indexmap::IndexSet;
use serde::Deserialize;
use std::time::{Duration, Instant};

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Eq)]
pub(crate) struct Bindings {
    bindings: Vec<IntentBinding>,
}

impl Bindings {
    pub(crate) fn from_str(content: &str) -> anyhow::Result<Self> {
        toml::from_str(content).map_err(Into::into)
    }

    pub(crate) fn evaluate(&self, timed_sources: &IndexSet<TimedSource>) -> Option<Intent> {
        self.bindings
            .iter()
            .find(|binding| binding.trigger.matches(timed_sources))
            .map(|binding| binding.intent.clone())
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
struct IntentBinding {
    intent: Intent,
    trigger: IntentTrigger,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "snake_case")]
enum IntentTrigger {
    Single(Source),
    Chord {
        chord: Vec<Source>,
    },
    Sequence {
        sequence: Vec<Source>,
        duration: Duration,
    },
}

impl IntentTrigger {
    fn matches(&self, timed_sources: &IndexSet<TimedSource>) -> bool {
        match self {
            IntentTrigger::Single(source) => {
                timed_sources.len() == 1 && timed_sources.iter().any(|ts| &ts.source == source)
            }
            IntentTrigger::Chord { chord } => chord
                .iter()
                .all(|s| timed_sources.iter().any(|ts| ts.source == *s)),
            IntentTrigger::Sequence { sequence, duration } => {
                let now = Instant::now();
                let sources_window = timed_sources
                    .iter()
                    .filter(|ts| now.duration_since(ts.timestamp) < *duration)
                    .map(|ts| ts.source)
                    .collect::<Vec<Source>>();
                sequence.iter().all(|s| sources_window.contains(s))
            }
        }
    }
}
