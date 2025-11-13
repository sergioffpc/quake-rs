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

    pub async fn connect<A>(&mut self, address: A) -> anyhow::Result<()>
    where
        A: ToSocketAddrs,
    {
        self.socket.connect(address).await?;

        const QUAKE_CONNECTION_REQUEST: &[u8] = b"\x01QUAKE\x03";
        self.socket.send(QUAKE_CONNECTION_REQUEST).await?;

        Ok(())
    }

    pub async fn reconnect(&mut self) -> anyhow::Result<()> {
        self.disconnect().await?;
        self.connect(self.socket.local_addr().unwrap()).await?;

        Ok(())
    }

    pub async fn disconnect(&mut self) -> anyhow::Result<()> {
        const QUAKE_DISCONNECT_REQUEST: &[u8] = b"\x02";
        self.socket.send(QUAKE_DISCONNECT_REQUEST).await?;

        Ok(())
    }
}
