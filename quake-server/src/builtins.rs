use crate::app::App;
use std::sync::{Arc, Mutex};

#[derive(Clone)]
pub struct AppBuiltins {
    inner: Arc<Mutex<App>>,
}

impl AppBuiltins {
    pub const BUILTIN_COMMANDS: &'static [&'static str] = &["version"];

    pub fn new(app: Arc<Mutex<App>>) -> Self {
        Self { inner: app }
    }

    pub fn builtin_version(&mut self) -> anyhow::Result<()> {
        use std::io::Write;
        writeln!(
            std::io::stdout(),
            "Quake Server Version: {}",
            env!("CARGO_PKG_VERSION")
        )?;

        Ok(())
    }
}
