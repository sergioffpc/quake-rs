use crate::Source;
use serde::Deserialize;

#[derive(Default, Deserialize)]
pub(crate) struct Mappings {
    #[serde(default)]
    mappings: Vec<(Source, Source)>,
}

impl Mappings {
    pub(crate) fn from_str(content: &str) -> anyhow::Result<Self> {
        toml::from_str(content).map_err(Into::into)
    }

    pub(crate) fn get(&self, source: Source) -> Source {
        self.mappings
            .iter()
            .find(|mapping| mapping.0 == source)
            .map(|mapping| mapping.1)
            .unwrap_or(source)
    }
}
