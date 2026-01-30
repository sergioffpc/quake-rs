use crate::quic::{QuicClientReceiver, QuicClientSender, QuicServerReceiver, QuicServerSender};
use serde::Serialize;
use serde::de::DeserializeOwned;
use std::net::SocketAddr;
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};

mod quic;

static CONNECTION_ID_GENERATOR: AtomicU64 = AtomicU64::new(0);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ConnectionId(u64);

impl ConnectionId {
    pub fn new() -> Self {
        Self(CONNECTION_ID_GENERATOR.fetch_add(1, Ordering::Relaxed))
    }
}

#[derive(Debug)]
pub struct MessageWrapper<M> {
    pub connection_id: ConnectionId,
    pub message: M,
}

pub enum NetworkClient<M> {
    Quic {
        sender: QuicClientSender<M>,
        receiver: QuicClientReceiver<M>,
    },
}

impl<M> NetworkClient<M> {
    pub async fn quic<P>(addr: SocketAddr, certs_path: P) -> anyhow::Result<Self>
    where
        P: AsRef<Path>,
        M: DeserializeOwned + Serialize + Send + 'static,
    {
        let (sender, receiver) =
            quic::client_channel(addr, certs_path.as_ref().to_path_buf().join("ca.crt")).await?;
        Ok(NetworkClient::Quic { sender, receiver })
    }

    pub fn send_message(&mut self, message: M) -> anyhow::Result<()>
    where
        M: Serialize + Send + 'static,
    {
        match self {
            NetworkClient::Quic { sender, .. } => sender.send_message(message),
        }
    }

    pub fn try_recv_message(&mut self) -> Option<M> {
        match self {
            NetworkClient::Quic { receiver, .. } => receiver.try_recv_message(),
        }
    }
}

pub enum NetworkServer<M> {
    Quic {
        sender: QuicServerSender<M>,
        receiver: QuicServerReceiver<M>,
    },
}

impl<M> NetworkServer<M> {
    pub fn quic<P>(addr: SocketAddr, certs_path: P) -> anyhow::Result<Self>
    where
        P: AsRef<Path>,
        M: DeserializeOwned + Serialize + Send + 'static,
    {
        let (sender, receiver) = quic::server_channel(
            addr,
            certs_path.as_ref().to_path_buf().join("server.crt"),
            certs_path.as_ref().to_path_buf().join("server.key"),
        )?;
        Ok(NetworkServer::Quic { sender, receiver })
    }

    pub fn send_message(&self, message: MessageWrapper<M>) -> anyhow::Result<()>
    where
        M: Send + Sync + 'static,
    {
        match self {
            NetworkServer::Quic { sender, .. } => sender.send_message(message),
        }
    }

    pub fn try_recv_message(&mut self) -> Option<MessageWrapper<M>> {
        match self {
            NetworkServer::Quic { receiver, .. } => receiver.try_recv_message(),
        }
    }
}
