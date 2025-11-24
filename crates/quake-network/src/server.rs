use crate::connection::{Connection, ConnectionManager};
use crate::dispatcher::{RequestDispatcher, RequestHandler};
use crate::{
    ACCEPT_CONNECTION_CONTROL_RESPONSE, CONNECTION_CONTROL_REQUEST, DISCONNECT_REQUEST,
    PLAYER_INFO_CONTROL_REQUEST, RULE_INFO_CONTROL_REQUEST, SERVER_INFO_CONTROL_REQUEST,
};
use bytes::{BufMut, BytesMut};
use parking_lot::RwLock;
use std::net::ToSocketAddrs;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use tracing::log::{info, warn};

struct ConnectionControlRequestHandler {
    connection_manager: Arc<ConnectionManager>,
}

impl RequestHandler for ConnectionControlRequestHandler {
    fn handle(&self, from: std::net::SocketAddr, data: &[u8]) -> anyhow::Result<()> {
        if data != b"QUAKE\x03" {
            return Err(anyhow::anyhow!("Invalid connection control request"));
        }

        info!("Received connection control request from {}", from);

        let connection_manager = self.connection_manager.clone();
        let connection = Arc::new(Connection::new(connection_manager)?);
        let local_addr = connection.local_addr()?;
        self.connection_manager.add(local_addr, connection.clone());

        let mut buf = BytesMut::new();
        buf.put_u8(ACCEPT_CONNECTION_CONTROL_RESPONSE);
        buf.put_u32(local_addr.port() as u32);
        self.connection_manager.send_to(&buf, from)?;

        info!("Connection established with {}", from);

        connection.listen();

        Ok(())
    }
}

struct ServerInfoControlRequestHandler;

impl RequestHandler for ServerInfoControlRequestHandler {
    fn handle(&self, from: std::net::SocketAddr, data: &[u8]) -> anyhow::Result<()> {
        if data != b"QUAKE\x03" {
            return Err(anyhow::anyhow!("Invalid server info control request"));
        }

        info!("Received server info control request from {}", from);
        Ok(())
    }
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
        let connection_manager = Arc::new(ConnectionManager::new(address)?);
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
            self.connection_manager.local_addr()?
        );

        self.running.store(true, Ordering::Relaxed);
        loop {
            if !self.running.load(Ordering::Relaxed) {
                break;
            }
            self.connection_manager
                .accept(self.request_dispatcher.clone());
        }
        self.connection_manager.close();

        Ok(())
    }

    pub fn stop(&self) {
        self.running.store(false, Ordering::Relaxed);
    }

    pub fn connections_count(&self) -> usize {
        self.connection_manager.count()
    }
}
