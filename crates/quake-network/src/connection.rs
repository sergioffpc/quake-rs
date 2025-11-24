use crate::DISCONNECT_REQUEST;
use crate::dispatcher::{RequestDispatcher, RequestHandler};
use parking_lot::RwLock;
use std::net::ToSocketAddrs;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::Duration;
use tracing::log::{error, info};

pub struct Connection {
    running: Arc<AtomicBool>,
    socket: std::net::UdpSocket,
    request_dispatcher: Arc<RwLock<RequestDispatcher>>,
}

impl Connection {
    pub fn new(connection_manager: Arc<ConnectionManager>) -> anyhow::Result<Self> {
        let socket = std::net::UdpSocket::bind("0.0.0.0:0")?;
        socket.set_read_timeout(Some(Duration::from_millis(100)))?;

        let request_dispatcher = Arc::new(RwLock::new(RequestDispatcher::default()));
        request_dispatcher.write().register_handler(
            DISCONNECT_REQUEST,
            Box::new(DisconnectRequestHandler {
                address: socket.local_addr()?,
                connection_manager,
            }),
        );

        Ok(Self {
            running: Arc::new(AtomicBool::new(false)),
            socket,
            request_dispatcher,
        })
    }

    pub fn listen(&self) {
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

    pub fn local_addr(&self) -> anyhow::Result<std::net::SocketAddr> {
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

    fn len(&self) -> usize {
        self.connections.len()
    }
}

pub struct ConnectionManager {
    socket: std::net::UdpSocket,
    connections: Arc<Connections>,
}

impl ConnectionManager {
    pub fn new<A>(address: A) -> anyhow::Result<Self>
    where
        A: ToSocketAddrs,
    {
        let socket = std::net::UdpSocket::bind(address)?;
        socket.set_read_timeout(Some(Duration::from_millis(100)))?;

        let connections = Arc::new(Connections::default());

        Ok(Self {
            socket,
            connections,
        })
    }

    pub fn add(&self, addr: std::net::SocketAddr, connection: Arc<Connection>) {
        self.connections.add(addr, connection);
    }

    pub fn accept(&self, dispatcher: Arc<RwLock<RequestDispatcher>>) {
        const BUFFER_SIZE: usize = 1024;
        let mut buf = [0u8; BUFFER_SIZE];
        match self.socket.recv_from(&mut buf) {
            Ok((n, from)) => {
                if n == 0 {
                    return;
                }
                let data = buf[..n].to_vec();
                thread::spawn(move || {
                    dispatcher.read().dispatch(from, data.as_slice());
                });
            }
            Err(ref e)
                if e.kind() == std::io::ErrorKind::WouldBlock
                    || e.kind() == std::io::ErrorKind::TimedOut =>
            {
                // recv_from() will block indefinitely. If stop() is called, the main loop
                // won't wake up until the next packet arrives.
                return;
            }
            Err(e) => error!("Error receiving UDP packet: {}", e),
        }
    }

    pub fn send_to(&self, data: &[u8], addr: std::net::SocketAddr) -> anyhow::Result<usize> {
        Ok(self.socket.send_to(data, addr)?)
    }

    pub fn close(&self) {
        self.connections.close();
    }

    pub fn local_addr(&self) -> anyhow::Result<std::net::SocketAddr> {
        self.socket
            .local_addr()
            .map_err(|e| anyhow::anyhow!("Failed to get local address: {}", e))
    }

    pub fn count(&self) -> usize {
        self.connections.len()
    }
}

struct DisconnectRequestHandler {
    address: std::net::SocketAddr,
    connection_manager: Arc<ConnectionManager>,
}

impl RequestHandler for DisconnectRequestHandler {
    fn handle(&self, from: std::net::SocketAddr, _data: &[u8]) -> anyhow::Result<()> {
        info!("Received disconnect request from {}", from);
        self.connection_manager.connections.remove(self.address);
        Ok(())
    }
}
