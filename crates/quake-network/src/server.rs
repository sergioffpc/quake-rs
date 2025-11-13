use tokio::net::ToSocketAddrs;
use tracing::log::error;

pub struct ServerManager {
    socket: tokio::net::UdpSocket,
}

impl ServerManager {
    pub async fn new<A>(address: A) -> anyhow::Result<Self>
    where
        A: ToSocketAddrs,
    {
        let socket = tokio::net::UdpSocket::bind(address).await?;

        Ok(Self { socket })
    }

    pub async fn run(&self) -> anyhow::Result<()> {
        let mut buf = [0u8; 1024];
        loop {
            match self.socket.recv_from(&mut buf).await {
                Ok((n, addr)) => {
                    let data = &buf[..n];

                    if let Err(e) = self.handle_message(data, addr).await {
                        error!("Error handling message from {}: {}", addr, e);
                    }
                }
                Err(e) => error!("Error receiving UDP packet: {}", e),
            }
        }
    }

    async fn handle_message(&self, data: &[u8], addr: std::net::SocketAddr) -> anyhow::Result<()> {
        match data[0] {
            0x01 => println!("Received connection request from {}", addr),
            _ => println!("Received unknown packet from {}", addr),
        }

        Ok(())
    }
}
