use crate::{
    ACCEPT_CONNECTION_CONTROL_RESPONSE, CONNECTION_CONTROL_REQUEST, DISCONNECT_CLIENT_REQUEST,
    PLAYER_INFO_CONTROL_REQUEST, RULE_INFO_CONTROL_REQUEST, SERVER_INFO_CONTROL_REQUEST,
};
use bytes::{BufMut, BytesMut};
use std::net::ToSocketAddrs;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use tracing::log::{error, info, warn};

struct Connection {
    running: Arc<AtomicBool>,
    socket: std::net::UdpSocket,
}

impl Connection {
    fn new() -> anyhow::Result<Self> {
        Ok(Self {
            running: Arc::new(AtomicBool::new(false)),
            socket: std::net::UdpSocket::bind("0.0.0.0:0")?,
        })
    }

    fn start(&self) {
        self.running.store(true, Ordering::Relaxed);
        loop {
            if !self.running.load(Ordering::Relaxed) {
                break;
            }

            let mut buf = [0u8; 1024];
            match self.socket.recv_from(&mut buf) {
                Ok((n, from)) => {
                    if n == 0 {
                        continue;
                    }
                    let data = &buf[..n];
                    match data[0] {
                        DISCONNECT_CLIENT_REQUEST => self.stop(),
                        _ => warn!("Received unknown packet from {}", from),
                    }
                }
                Err(e) => error!("Error receiving UDP packet: {}", e),
            }
        }
    }

    fn stop(&self) {
        self.running.store(false, Ordering::Relaxed);
    }

    fn is_running(&self) -> bool {
        self.running.load(Ordering::Relaxed)
    }

    fn local_addr(&self) -> anyhow::Result<std::net::SocketAddr> {
        self.socket
            .local_addr()
            .map_err(|e| anyhow::anyhow!("Failed to get local address: {}", e))
    }
}

struct ConnectionManager {
    socket: std::net::UdpSocket,
    connections: dashmap::DashMap<std::net::SocketAddr, Arc<Connection>>,
}

impl ConnectionManager {
    pub fn new<A>(address: A) -> anyhow::Result<Self>
    where
        A: ToSocketAddrs,
    {
        let socket = std::net::UdpSocket::bind(address)?;
        let connections = dashmap::DashMap::default();
        let connections_cleanup = connections.clone();

        thread::spawn(move || {
            loop {
                thread::sleep(std::time::Duration::from_secs(5));
                connections_cleanup.retain(|_, conn: &mut Arc<Connection>| {
                    if conn.is_running() {
                        true
                    } else {
                        info!(
                            "Connection {} has been removed from the list of connections",
                            conn.local_addr().unwrap()
                        );
                        false
                    }
                });
            }
        });

        Ok(Self {
            socket,
            connections,
        })
    }

    fn add(&mut self, connection: Arc<Connection>) -> anyhow::Result<()> {
        self.connections
            .insert(connection.local_addr()?, connection);
        Ok(())
    }

    fn local_addr(&self) -> anyhow::Result<std::net::SocketAddr> {
        self.socket
            .local_addr()
            .map_err(|e| anyhow::anyhow!("Failed to get local address: {}", e))
    }
}

pub struct ServerManager {
    running: Arc<AtomicBool>,
    connection_manager: Arc<ConnectionManager>,
}

impl ServerManager {
    pub fn new<A>(address: A) -> anyhow::Result<Self>
    where
        A: ToSocketAddrs,
    {
        let connection_manager = Arc::new(ConnectionManager::new(address)?);

        Ok(Self {
            running: Arc::new(AtomicBool::new(false)),
            connection_manager,
        })
    }

    pub fn start(&self) -> anyhow::Result<()> {
        info!(
            "Listening on {:?} for UDP packets...",
            self.connection_manager.local_addr()?
        );

        const BUFFER_SIZE: usize = 1024;
        let mut buf = [0u8; BUFFER_SIZE];

        self.running.store(true, Ordering::Relaxed);
        loop {
            if !self.running.load(Ordering::Relaxed) {
                break;
            }
            match self.connection_manager.socket.recv_from(&mut buf) {
                Ok((n, from)) => {
                    if n == 0 {
                        continue;
                    }
                    let data = buf[..n].to_vec();
                    let connection_manager = self.connection_manager.clone();
                    thread::spawn(move || {
                        Self::handle_control_request(from, data, connection_manager)
                    });
                }
                Err(e) => error!("Error receiving UDP packet: {}", e),
            }
        }
        Ok(())
    }

    pub fn stop(&self) {
        self.running.store(false, Ordering::Relaxed);
    }

    fn handle_control_request(
        from: std::net::SocketAddr,
        data: Vec<u8>,
        connection_manager: Arc<ConnectionManager>,
    ) -> anyhow::Result<()> {
        match data[0] {
            CONNECTION_CONTROL_REQUEST => {
                if &data[1..] == b"QUAKE\x03" {
                    info!("Received connection control request from {}", from);
                    Self::handle_connection_control_request(from, connection_manager)?;
                } else {
                    warn!("Invalid connection control request from {}", from);
                }
            }
            SERVER_INFO_CONTROL_REQUEST => {
                if &data[1..] == b"QUAKE\x03" {
                    info!("Received server info control request from {}", from);
                } else {
                    warn!("Invalid server info control request from {}", from);
                }
            }
            PLAYER_INFO_CONTROL_REQUEST => {
                info!("Received player info control request from {}", from);
            }
            RULE_INFO_CONTROL_REQUEST => {
                info!("Received rule info control request from {}", from);
            }
            _ => warn!("Received unknown packet from {}", from),
        }

        Ok(())
    }

    fn handle_connection_control_request(
        from: std::net::SocketAddr,
        connection_manager: Arc<ConnectionManager>,
    ) -> anyhow::Result<()> {
        let connection = Arc::new(Connection::new()?);
        let local_addr = connection.local_addr()?;
        connection_manager
            .connections
            .insert(local_addr, connection.clone());

        thread::spawn(move || connection.start());

        let mut buf = BytesMut::new();
        buf.put_u8(ACCEPT_CONNECTION_CONTROL_RESPONSE);
        buf.put_u32(local_addr.port() as u32);
        connection_manager.socket.send_to(&buf, from)?;

        info!("Connection established with {}", from);

        Ok(())
    }
}
