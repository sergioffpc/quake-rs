use clap::Parser;
use std::net::SocketAddr;
use std::num::NonZero;
use std::path::PathBuf;

#[derive(Parser, Debug)]
struct Args {
    #[arg(long, default_value = "res/")]
    resources_oath: PathBuf,
    #[arg(long, default_value = "certs/")]
    certs_path: PathBuf,
    #[arg(long, default_value = "[::1]:30512")]
    listen_addr: SocketAddr,
}

pub fn run() -> anyhow::Result<()> {
    let args = Args::parse();
    ServerApp::new(args)?.run()
}

struct ServerApp {
    universe: quake_world::universe::UniverseServer,
}

impl ServerApp {
    fn new(args: Args) -> anyhow::Result<Self> {
        let num_shards = std::thread::available_parallelism().unwrap_or(NonZero::new(1).unwrap());
        let network_manager =
            quake_network::NetworkServer::quic(args.listen_addr, args.certs_path)?;
        let asset_manager = quake_asset::AssetManager::new(args.resources_oath)?;

        Ok(Self {
            universe: quake_world::universe::UniverseServer::new(
                num_shards,
                network_manager,
                asset_manager,
            )?,
        })
    }

    fn run(&mut self) -> anyhow::Result<()> {
        loop {
            self.universe.step()?;
        }
    }
}
