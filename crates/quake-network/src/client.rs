use crate::{QUAKE_CONNECTION_REQUEST, QUAKE_DISCONNECT_REQUEST};
use tokio::net::ToSocketAddrs;

pub struct ClientManager {
    socket: tokio::net::UdpSocket,
}

impl ClientManager {
    pub async fn new() -> anyhow::Result<Self> {
        Ok(Self {
            socket: tokio::net::UdpSocket::bind("0.0.0.0:0").await?,
        })
    }

    pub async fn connect<A>(&self, address: A) -> anyhow::Result<()>
    where
        A: ToSocketAddrs,
    {
        self.socket.connect(address).await?;
        // Capture the peer address before receiving data to modify it later
        let mut remote_addr = self.socket.peer_addr()?;

        self.socket.send(QUAKE_CONNECTION_REQUEST).await?;

        const BUFFER_SIZE: usize = 1024;
        let mut buf = [0u8; BUFFER_SIZE];

        let n = self.socket.recv(&mut buf).await?;
        if n != 4 {
            anyhow::bail!("Invalid response size from server");
        }

        let port_bytes: [u8; 4] = buf[..4].try_into()?;
        let remote_port = u32::from_be_bytes(port_bytes) as u16;

        remote_addr.set_port(remote_port);
        self.socket.connect(remote_addr).await?;

        Ok(())
    }

    pub async fn reconnect(&self) -> anyhow::Result<()> {
        self.disconnect().await?;
        self.connect(self.socket.local_addr().unwrap()).await?;

        Ok(())
    }

    pub async fn disconnect(&self) -> anyhow::Result<()> {
        self.socket.send(QUAKE_DISCONNECT_REQUEST).await?;

        Ok(())
    }
}
