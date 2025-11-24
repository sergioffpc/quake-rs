use crate::client::ClientManager;
use crate::server::ServerManager;
use quake_traits::ControlFlow;
use std::sync::Arc;

#[derive(Clone)]
pub struct ClientBuiltins {
    inner: Arc<ClientManager>,
}

impl ClientBuiltins {
    pub const BUILTIN_COMMANDS: &'static [&'static str] = &["connect", "disconnect"];

    pub fn new(inner: Arc<ClientManager>) -> Self {
        Self { inner }
    }

    pub fn builtin_connect(&mut self, args: &[&str]) -> anyhow::Result<ControlFlow> {
        self.inner
            .connect(Self::with_default_port(args[0], 26000))?;
        Ok(ControlFlow::Poll)
    }

    pub fn builtin_disconnect(&self) -> anyhow::Result<ControlFlow> {
        self.inner.disconnect()?;
        Ok(ControlFlow::Poll)
    }

    fn with_default_port(addr: &str, default_port: u16) -> String {
        if addr.contains(':') {
            addr.to_string()
        } else {
            format!("{}:{}", addr, default_port)
        }
    }
}

impl quake_traits::CommandHandler for ClientBuiltins {
    fn handle_command(&mut self, command: &[&str]) -> anyhow::Result<ControlFlow> {
        match command[0] {
            "connect" => self.builtin_connect(&command[1..]),
            "disconnect" => self.builtin_disconnect(),
            _ => Ok(ControlFlow::Poll),
        }
    }
}

#[derive(Clone)]
pub struct ServerBuiltins {
    inner: Arc<ServerManager>,
}

impl ServerBuiltins {
    pub const BUILTIN_COMMANDS: &'static [&'static str] = &["status"];

    pub fn new(inner: Arc<ServerManager>) -> Self {
        Self { inner }
    }

    pub fn builtin_status(&mut self) -> anyhow::Result<ControlFlow> {
        let hostname = hostname::get()
            .map(|h| h.into_string().unwrap_or_else(|_| "unknown".to_string()))
            .unwrap_or_else(|_| "unknown".to_string());

        use std::io::Write;
        writeln!(
            std::io::stdout(),
            "host: {}\nplayers: {}",
            hostname,
            self.inner.connections_count()
        )?;

        Ok(ControlFlow::Poll)
    }
}

impl quake_traits::CommandHandler for ServerBuiltins {
    fn handle_command(&mut self, command: &[&str]) -> anyhow::Result<ControlFlow> {
        match command[0] {
            "status" => self.builtin_status(),
            _ => Ok(ControlFlow::Poll),
        }
    }
}
