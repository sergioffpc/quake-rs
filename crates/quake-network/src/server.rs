use crate::{QUAKE_CONNECTION_REQUEST, QUAKE_DISCONNECT_REQUEST};
use std::sync::Arc;
use tokio::net::ToSocketAddrs;
use tracing::log::{error, info, warn};

pub struct ServerManager {
    socket: Arc<tokio::net::UdpSocket>,
    connections: Arc<dashmap::DashMap<std::net::SocketAddr, tokio::net::UdpSocket>>,
}

impl ServerManager {
    pub async fn new<A>(address: A) -> anyhow::Result<Self>
    where
        A: ToSocketAddrs,
    {
        let socket = Arc::new(tokio::net::UdpSocket::bind(address).await?);
        let connections = Arc::new(dashmap::DashMap::default());

        Ok(Self {
            socket,
            connections,
        })
    }

    pub async fn listen(&self) -> anyhow::Result<()> {
        info!(
            "Listening on {:?} for UDP packets...",
            self.socket.local_addr()?
        );

        const BUFFER_SIZE: usize = 1024;
        let mut buf = [0u8; BUFFER_SIZE];
        loop {
            match self.socket.recv_from(&mut buf).await {
                Ok((n, addr)) => {
                    if n == 0 {
                        continue;
                    }

                    let data = buf[..n].to_vec();
                    let socket = self.socket.clone();
                    let connections = self.connections.clone();

                    tokio::spawn(async move {
                        match data.as_slice() {
                            QUAKE_CONNECTION_REQUEST => {
                                let conn =
                                    Self::handle_connection_request(socket, addr).await.unwrap();
                                connections.insert(addr, conn);
                            }
                            QUAKE_DISCONNECT_REQUEST => {
                                connections.remove(&addr);
                            }
                            _ => warn!("Received unknown packet from {}", addr),
                        }
                    });
                }
                Err(e) => error!("Error receiving UDP packet: {}", e),
            }
        }
    }

    async fn handle_connection_request(
        socket: Arc<tokio::net::UdpSocket>,
        addr: std::net::SocketAddr,
    ) -> anyhow::Result<tokio::net::UdpSocket> {
        info!("Received connection request from {}", addr);

        let conn = tokio::net::UdpSocket::bind("0.0.0.0:0").await?;
        conn.connect(addr).await?;

        let port = conn.local_addr()?.port();
        info!("Assigned port {} to client {}", port, addr);

        let response = (port as u32).to_be_bytes();
        socket.send_to(&response, addr).await?;

        Ok(conn)
    }
}
