use crate::{ConnectionId, MessageWrapper};
use bytes::{Bytes, BytesMut};
use quinn::RecvStream;
use quinn::rustls::RootCertStore;
use serde::Serialize;
use serde::de::DeserializeOwned;
use std::fmt::Debug;
use std::net::SocketAddr;
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use tokio::sync::mpsc;
use tracing::{error, info, warn};

#[derive(Debug, Default)]
struct Connections {
    connections: dashmap::DashMap<ConnectionId, quinn::Connection>,
}

impl Connections {
    fn insert(&self, connection_id: ConnectionId, connection: quinn::Connection) {
        self.connections.insert(connection_id, connection);
    }

    fn get(&self, connection_id: ConnectionId) -> Option<quinn::Connection> {
        self.connections
            .get(&connection_id)
            .map(|entry| entry.value().clone())
    }
}

pub struct QuicServerSender<M> {
    send_buffer: mpsc::UnboundedSender<MessageWrapper<M>>,
}

impl<M> QuicServerSender<M> {
    pub fn send_message(&self, message: MessageWrapper<M>) -> anyhow::Result<()>
    where
        M: Send + Sync + 'static,
    {
        self.send_buffer.send(message).map_err(Into::into)
    }
}

fn spawn_sender_task<M>(
    connections: Arc<Connections>,
    mut recv_buffer: mpsc::UnboundedReceiver<MessageWrapper<M>>,
) where
    M: Serialize + Send + 'static,
{
    tokio::spawn(async move {
        while let Some(MessageWrapper {
            connection_id,
            message,
        }) = recv_buffer.recv().await
        {
            if let Some(connection) = connections.get(connection_id) {
                spawn_and_send_message(connection.clone(), message);
            } else {
                warn!(?connection_id, "connection not found");
            }
        }
    });
}

pub struct QuicServerReceiver<M> {
    recv_buffer: mpsc::UnboundedReceiver<MessageWrapper<M>>,
}

impl<M> QuicServerReceiver<M> {
    pub fn try_recv_message(&mut self) -> Option<MessageWrapper<M>> {
        self.recv_buffer.try_recv().ok()
    }
}

fn spawn_receiver_task<M>(
    endpoint: quinn::Endpoint,
    connections: Arc<Connections>,
    send_buffer: mpsc::UnboundedSender<MessageWrapper<M>>,
) where
    M: DeserializeOwned + Send + 'static,
{
    tokio::spawn(async move {
        while let Some(incoming) = endpoint.accept().await {
            match incoming.await {
                Ok(connection) => {
                    let connection_id = ConnectionId::new();
                    info!(?connection_id, remote_address=?connection.remote_address(), "accepted connection");

                    connections.insert(connection_id, connection.clone());
                    spawn_stream_handler(connection_id, connection, send_buffer.clone());
                }
                Err(err) => {
                    error!(%err, "failed to accept connection");
                }
            }
        }
    });
}

fn spawn_stream_handler<M>(
    connection_id: ConnectionId,
    connection: quinn::Connection,
    sender: mpsc::UnboundedSender<MessageWrapper<M>>,
) where
    M: DeserializeOwned + Send + 'static,
{
    tokio::spawn(async move {
        loop {
            tokio::select! {
                result = connection.accept_uni() => {
                    match result {
                        Ok(stream) => {
                            if let Err(err) = read_stream(stream, connection_id, &sender).await {
                                error!(%err, "failed to read stream");
                            }
                        }
                        Err(err) => {
                            error!(%err, "failed to accept stream");
                            break;
                        }
                    }
                }
                result = connection.read_datagram() => {
                    match result {
                        Ok(datagram) => {
                            if let Err(err) = read_datagram(datagram, connection_id, &sender) {
                                error!(%err, "failed to read datagram");
                            }
                        }
                        Err(err) => {
                            error!(%err, "failed to receive datagram");
                            break;
                        }
                    }
                }
            }
        }
        error!(?connection_id, "connection closed");
    });
}

async fn read_stream<M>(
    mut stream: RecvStream,
    connection_id: ConnectionId,
    sender: &mpsc::UnboundedSender<MessageWrapper<M>>,
) -> anyhow::Result<()>
where
    M: DeserializeOwned + Send + 'static,
{
    let message_bytes = stream.read_to_end(1048576).await?;
    let message = postcard::from_bytes(&*message_bytes)?;
    sender
        .send(MessageWrapper {
            connection_id,
            message,
        })
        .map_err(|e| anyhow::anyhow!("{}", e))
}

fn read_datagram<M>(
    datagram: Bytes,
    connection_id: ConnectionId,
    sender: &mpsc::UnboundedSender<MessageWrapper<M>>,
) -> anyhow::Result<()>
where
    M: DeserializeOwned + Send + 'static,
{
    let message = postcard::from_bytes(&*datagram)?;
    sender
        .send(MessageWrapper {
            connection_id,
            message,
        })
        .map_err(|e| anyhow::anyhow!("{}", e))
}

pub fn server_channel<P, M>(
    addr: SocketAddr,
    cert_path: P,
    key_path: P,
) -> anyhow::Result<(QuicServerSender<M>, QuicServerReceiver<M>)>
where
    P: AsRef<Path>,
    M: DeserializeOwned + Serialize + Send + 'static,
{
    let mut transport = quinn::TransportConfig::default();
    transport.datagram_send_buffer_size(65536);
    transport.datagram_receive_buffer_size(Some(65536));
    transport.max_idle_timeout(Some(
        std::time::Duration::from_secs(300).try_into().unwrap(),
    ));
    transport.keep_alive_interval(Some(std::time::Duration::from_secs(5)));

    let cert_pem = std::fs::read(cert_path)?;
    let key_pem = std::fs::read(key_path)?;
    let certs = rustls_pemfile::certs(&mut cert_pem.as_slice()).collect::<Result<Vec<_>, _>>()?;
    let key = rustls_pemfile::private_key(&mut key_pem.as_slice())?
        .ok_or_else(|| anyhow::anyhow!("No private key found"))?;

    let mut config = quinn::ServerConfig::with_single_cert(certs, key)?;
    config.transport_config(Arc::new(transport));

    let endpoint = quinn::Endpoint::server(config, addr)?;
    let connections = Arc::new(Connections::default());

    let (sender_tx, sender_rx) = mpsc::unbounded_channel::<MessageWrapper<M>>();
    spawn_sender_task(connections.clone(), sender_rx);

    let (receiver_tx, receiver_rx) = mpsc::unbounded_channel::<MessageWrapper<M>>();
    spawn_receiver_task(endpoint, connections.clone(), receiver_tx);

    info!(%addr, "listening on");

    Ok((
        QuicServerSender {
            send_buffer: sender_tx,
        },
        QuicServerReceiver {
            recv_buffer: receiver_rx,
        },
    ))
}

pub struct QuicClientSender<M> {
    send_buffer: mpsc::UnboundedSender<M>,
}

impl<M> QuicClientSender<M> {
    pub fn send_message(&mut self, message: M) -> anyhow::Result<()>
    where
        M: Serialize + Send + 'static,
    {
        self.send_buffer
            .send(message)
            .map_err(|e| anyhow::anyhow!("{}", e))
    }
}

struct ClientSenderTask;
impl ClientSenderTask {
    fn run<M>(connection: quinn::Connection, mut receiver: mpsc::UnboundedReceiver<M>)
    where
        M: Serialize + Send + 'static,
    {
        tokio::spawn(async move {
            while let Some(message) = receiver.recv().await {
                spawn_and_send_message(connection.clone(), message);
            }
        });
    }
}

pub struct QuicClientReceiver<M> {
    recv_buffer: mpsc::UnboundedReceiver<M>,
}

impl<M> QuicClientReceiver<M> {
    pub fn try_recv_message(&mut self) -> Option<M> {
        self.recv_buffer.try_recv().ok()
    }
}

struct ClientReceiverTask;
impl ClientReceiverTask {
    fn run<M>(connection: quinn::Connection, sender: mpsc::UnboundedSender<M>)
    where
        M: DeserializeOwned + Send + 'static,
    {
        tokio::spawn(async move {
            loop {
                tokio::select! {
                    result = connection.accept_uni() => {
                        match result {
                            Ok(stream) => {
                                if let Err(err) = Self::read_stream(stream, &sender).await {
                                    error!(%err, "failed to read stream");
                                }
                            }
                            Err(err) => {
                                error!(%err, "failed to accept stream");
                            }
                        }
                    }
                    result = connection.read_datagram() => {
                        match result {
                            Ok(datagram) => {
                                if let Err(err) = Self::read_datagram(datagram, &sender) {
                                    error!(%err, "failed to read datagram");
                                }
                            }
                            Err(err) => {
                                error!(%err, "failed to receive datagram");
                            }
                        }
                    }
                }
            }
        });
    }

    async fn read_stream<M>(
        mut stream: RecvStream,
        sender: &mpsc::UnboundedSender<M>,
    ) -> anyhow::Result<()>
    where
        M: DeserializeOwned + Send + 'static,
    {
        let message_bytes = stream.read_to_end(1048576).await?;
        let message = postcard::from_bytes(&*message_bytes)?;
        sender.send(message).map_err(|e| anyhow::anyhow!("{}", e))
    }

    fn read_datagram<M>(datagram: Bytes, sender: &mpsc::UnboundedSender<M>) -> anyhow::Result<()>
    where
        M: DeserializeOwned + Send + 'static,
    {
        let message = postcard::from_bytes(&*datagram)?;
        sender.send(message).map_err(|e| anyhow::anyhow!("{}", e))
    }
}

pub async fn client_channel<P, M>(
    addr: SocketAddr,
    ca_path: P,
) -> anyhow::Result<(QuicClientSender<M>, QuicClientReceiver<M>)>
where
    P: AsRef<Path>,
    M: DeserializeOwned + Serialize + Send + 'static,
{
    let mut transport = quinn::TransportConfig::default();
    transport.datagram_send_buffer_size(65536);
    transport.datagram_receive_buffer_size(Some(65536));
    transport.max_idle_timeout(Some(
        std::time::Duration::from_secs(300).try_into().unwrap(),
    ));
    transport.keep_alive_interval(Some(std::time::Duration::from_secs(5)));

    let ca_pem = tokio::fs::read(ca_path).await?;
    let ca_certs = rustls_pemfile::certs(&mut ca_pem.as_slice()).collect::<Result<Vec<_>, _>>()?;

    let mut roots = RootCertStore::empty();
    for cert in ca_certs {
        roots.add(cert)?;
    }

    let crypto = quinn::rustls::ClientConfig::builder()
        .with_root_certificates(roots)
        .with_no_client_auth();

    let mut config = quinn::ClientConfig::new(Arc::new(
        quinn::crypto::rustls::QuicClientConfig::try_from(crypto)?,
    ));
    config.transport_config(Arc::new(transport));

    let mut endpoint = quinn::Endpoint::client("[::]:0".parse().unwrap())?;
    endpoint.set_default_client_config(config);
    let connection = endpoint.connect(addr, "localhost")?.await?;

    info!(%addr, "connected to");

    let (sender_tx, sender_rx) = mpsc::unbounded_channel::<M>();
    ClientSenderTask::run(connection.clone(), sender_rx);

    let (receiver_tx, receiver_rx) = mpsc::unbounded_channel::<M>();
    ClientReceiverTask::run(connection, receiver_tx);

    Ok((
        QuicClientSender {
            send_buffer: sender_tx,
        },
        QuicClientReceiver {
            recv_buffer: receiver_rx,
        },
    ))
}

fn spawn_and_send_message<M>(connection: quinn::Connection, message: M)
where
    M: Serialize + Send + 'static,
{
    tokio::spawn(serialize_and_send_message(connection, message));
}

async fn serialize_and_send_message<M>(connection: quinn::Connection, message: M)
where
    M: Serialize + Send + 'static,
{
    match serialize_message(&message) {
        Ok(data) => {
            if let Err(err) = send_message_datagram_or_stream(connection, data).await {
                error!(%err, "failed to send message");
            }
        }
        Err(err) => {
            error!(%err, "failed to serialize message");
        }
    }
}

async fn send_message_datagram_or_stream(
    connection: quinn::Connection,
    data: Bytes,
) -> anyhow::Result<()> {
    let max_datagram_size = connection.max_datagram_size();
    if let Some(max_datagram_size) = max_datagram_size
        && data.len() <= max_datagram_size
    {
        send_message_datagram(connection, data).await
    } else {
        send_message_stream(connection, data).await
    }
}

fn serialize_message<M>(message: &M) -> anyhow::Result<Bytes>
where
    M: Serialize,
{
    let serialized = postcard::to_allocvec(message)?;
    let mut data = BytesMut::with_capacity(serialized.len());
    data.extend_from_slice(&serialized);

    Ok(data.freeze())
}

async fn send_message_datagram(connection: quinn::Connection, data: Bytes) -> anyhow::Result<()> {
    connection
        .send_datagram_wait(data)
        .await
        .map_err(|e| anyhow::anyhow!("{}", e))
}

async fn send_message_stream(connection: quinn::Connection, data: Bytes) -> anyhow::Result<()> {
    let mut stream = connection.open_uni().await?;
    stream.write_all(&data).await?;
    stream.finish().map_err(|e| anyhow::anyhow!("{}", e))
}
