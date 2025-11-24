use crate::{
    ACCEPT_CONNECTION_CONTROL_RESPONSE, CONNECTION_CONTROL_REQUEST, DISCONNECT_REQUEST,
    PLAYER_INFO_CONTROL_REQUEST, RULE_INFO_CONTROL_REQUEST, SERVER_INFO_CONTROL_REQUEST,
};
use bytes::{BufMut, BytesMut};
use parking_lot::RwLock;
use std::collections::HashMap;
use std::net::ToSocketAddrs;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::Duration;
use tracing::log::{error, info, warn};

#[derive(Default)]
struct RequestDispatcher {
    handlers: HashMap<u8, Box<dyn RequestHandler>>,
}

impl RequestDispatcher {
    fn dispatch(&self, from: std::net::SocketAddr, data: &[u8]) {
        if data.is_empty() {
            return;
        }

        if let Some(handler) = self.handlers.get(&data[0]) {
            if let Err(e) = handler.handle(from, &data[1..]) {
                error!("Error handling request from {}: {}", from, e);
            }
        } else {
            warn!("Received unknown packet from {}", from);
        }
    }

    fn register_handler(&mut self, packet_type: u8, handler: Box<dyn RequestHandler>) {
        self.handlers.insert(packet_type, handler);
    }

    fn unregister_handler(&mut self, packet_type: u8) {
        self.handlers.remove(&packet_type);
    }
}

trait RequestHandler: Send + Sync {
    fn handle(&self, from: std::net::SocketAddr, data: &[u8]) -> anyhow::Result<()>;
}

struct ConnectionControlRequestHandler {
    connection_manager: Arc<ConnectionManager>,
}

impl RequestHandler for ConnectionControlRequestHandler {
    fn handle(&self, from: std::net::SocketAddr, data: &[u8]) -> anyhow::Result<()> {
        if &data[1..] != b"QUAKE\x03" {
            return Err(anyhow::anyhow!("Invalid connection control request"));
        }

        info!("Received connection control request from {}", from);

        let connection_manager = self.connection_manager.clone();
        let connection = Arc::new(Connection::new(connection_manager)?);
        let local_addr = connection.local_addr()?;
        self.connection_manager
            .connections
            .add(local_addr, connection.clone());

        let mut buf = BytesMut::new();
        buf.put_u8(ACCEPT_CONNECTION_CONTROL_RESPONSE);
        buf.put_u32(local_addr.port() as u32);
        self.connection_manager.socket.send_to(&buf, from)?;

        info!("Connection established with {}", from);

        connection.listen();

        Ok(())
    }
}

struct ServerInfoControlRequestHandler;

impl RequestHandler for ServerInfoControlRequestHandler {
    fn handle(&self, from: std::net::SocketAddr, data: &[u8]) -> anyhow::Result<()> {
        if &data[1..] != b"QUAKE\x03" {
            return Err(anyhow::anyhow!("Invalid server info control request"));
        }

        info!("Received server info control request from {}", from);
        Ok(())
    }
}

struct DisconnectRequestHandler {
    connection_manager: Arc<ConnectionManager>,
}

impl RequestHandler for DisconnectRequestHandler {
    fn handle(&self, from: std::net::SocketAddr, data: &[u8]) -> anyhow::Result<()> {
        info!("Received disconnect request from {}", from);
        self.connection_manager.connections.remove(from);
        Ok(())
    }
}

struct Connection {
    running: Arc<AtomicBool>,
    socket: std::net::UdpSocket,
    request_dispatcher: Arc<RwLock<RequestDispatcher>>,
}

impl Connection {
    fn new(connection_manager: Arc<ConnectionManager>) -> anyhow::Result<Self> {
        let socket = std::net::UdpSocket::bind("0.0.0.0:0")?;
        socket.set_read_timeout(Some(Duration::from_millis(100)))?;
        let request_dispatcher = Arc::new(RwLock::new(RequestDispatcher::default()));
        request_dispatcher.write().register_handler(
            DISCONNECT_REQUEST,
            Box::new(DisconnectRequestHandler { connection_manager }),
        );

        Ok(Self {
            running: Arc::new(AtomicBool::new(false)),
            socket,
            request_dispatcher,
        })
    }

    fn listen(&self) {
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
                    self.request_dispatcher.read().dispatch(from, data);
                }
                Err(ref e)
                    if e.kind() == std::io::ErrorKind::WouldBlock
                        || e.kind() == std::io::ErrorKind::TimedOut =>
                {
                    // recv_from() will block indefinitely. If stop() is called, the main loop
                    // won't wake up until the next packet arrives.
                    continue;
                }
                Err(e) => error!("Error receiving UDP packet: {}", e),
            }
        }
    }

    fn close(&self) {
        self.running.store(false, Ordering::Relaxed);
    }

    fn local_addr(&self) -> anyhow::Result<std::net::SocketAddr> {
        self.socket
            .local_addr()
            .map_err(|e| anyhow::anyhow!("Failed to get local address: {}", e))
    }
}

#[derive(Default)]
struct Connections {
    connections: dashmap::DashMap<std::net::SocketAddr, Arc<Connection>>,
}

impl Connections {
    fn add(&self, addr: std::net::SocketAddr, connection: Arc<Connection>) {
        self.connections.insert(addr, connection);
    }

    fn remove(&self, addr: std::net::SocketAddr) {
        self.connections.remove(&addr);
    }

    fn close(&self) {
        let addrs: Vec<_> = self.connections.iter().map(|e| *e.key()).collect();
        for addr in addrs {
            if let Some((_, conn)) = self.connections.remove(&addr) {
                conn.close();
            }
        }
    }
}

struct ConnectionManager {
    socket: std::net::UdpSocket,
    connections: Arc<Connections>,
}

pub struct ServerManager {
    running: Arc<AtomicBool>,
    connection_manager: Arc<ConnectionManager>,
    request_dispatcher: Arc<RwLock<RequestDispatcher>>,
}

impl ServerManager {
    pub fn new<A>(address: A) -> anyhow::Result<Self>
    where
        A: ToSocketAddrs,
    {
        let socket = std::net::UdpSocket::bind(address)?;
        socket.set_read_timeout(Some(Duration::from_millis(100)))?;

        let connections = Arc::new(Connections::default());
        let connection_manager = Arc::new(ConnectionManager {
            socket,
            connections,
        });
        let request_dispatcher = Arc::new(RwLock::new(RequestDispatcher::default()));
        request_dispatcher.write().register_handler(
            CONNECTION_CONTROL_REQUEST,
            Box::new(ConnectionControlRequestHandler {
                connection_manager: connection_manager.clone(),
            }),
        );
        request_dispatcher.write().register_handler(
            SERVER_INFO_CONTROL_REQUEST,
            Box::new(ServerInfoControlRequestHandler),
        );

        Ok(Self {
            running: Arc::new(AtomicBool::new(false)),
            connection_manager,
            request_dispatcher,
        })
    }

    pub fn start(&self) -> anyhow::Result<()> {
        info!(
            "Listening on {:?} for UDP packets...",
            self.connection_manager.socket.local_addr()?
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
                    let request_dispatcher = self.request_dispatcher.clone();
                    let data = buf[..n].to_vec();
                    thread::spawn(move || {
                        request_dispatcher.read().dispatch(from, data.as_slice());
                    });
                }
                Err(ref e)
                    if e.kind() == std::io::ErrorKind::WouldBlock
                        || e.kind() == std::io::ErrorKind::TimedOut =>
                {
                    // recv_from() will block indefinitely. If stop() is called, the main loop
                    // won't wake up until the next packet arrives.
                    continue;
                }
                Err(e) => error!("Error receiving UDP packet: {}", e),
            }
        }

        self.connection_manager.connections.close();

        Ok(())
    }

    pub fn stop(&self) {
        self.running.store(false, Ordering::Relaxed);
    }
}
