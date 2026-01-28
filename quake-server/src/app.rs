use std::net::SocketAddr;

pub fn run(addr: SocketAddr) -> anyhow::Result<()> {
    ServerApp::new(addr)?.run()
}

struct ServerApp {
    universe: quake_world::universe::UniverseServer,
}

impl ServerApp {
    fn new(addr: SocketAddr) -> anyhow::Result<Self> {
        Ok(Self {
            universe: quake_world::universe::UniverseServer::new(addr)?,
        })
    }

    fn run(&mut self) -> anyhow::Result<()> {
        loop {
            self.universe.step()?;
        }
    }
}
