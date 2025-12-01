use crate::{RequestDispatcher, RequestHandler};
use rcgen::KeyPair;
use std::net::SocketAddr;
use std::sync::Arc;
use tracing::log::{error, info};

pub struct ServerManager {
    endpoint: quinn::Endpoint,
    dispatcher: Arc<RequestDispatcher>,
}

impl ServerManager {
    pub fn new(address: SocketAddr) -> anyhow::Result<Self> {
        let cert = Self::cert()?;
        let cert_der = quinn::rustls::pki_types::CertificateDer::from(cert.cert);
        let cert_key =
            quinn::rustls::pki_types::PrivatePkcs8KeyDer::from(cert.signing_key.serialize_der())
                .into();
        let config = quinn::ServerConfig::with_single_cert(vec![cert_der], cert_key)?;
        let endpoint = quinn::Endpoint::server(config, address)?;

        let mut dispatcher = RequestDispatcher::default();
        dispatcher.register_handler(0x01, Box::new(ConnectionControlRequestHandler));
        dispatcher.register_handler(0x02, Box::new(ServerInfoControlRequestHandler));

        Ok(Self {
            endpoint,
            dispatcher: Arc::new(dispatcher),
        })
    }

    pub async fn accept(&self) {
        while let Some(incoming) = self.endpoint.accept().await {
            let dispatcher = self.dispatcher.clone();
            tokio::spawn(async move {
                match incoming.await {
                    Ok(connection) => {
                        info!("Incoming connection from {:?}", connection.remote_address());
                        Self::handle_connection(connection, dispatcher).await;
                    }
                    Err(e) => error!("Error accepting connection: {}", e),
                }
            });
        }
    }

    pub fn cert() -> anyhow::Result<rcgen::CertifiedKey<KeyPair>> {
        let subject_alt_names = vec!["localhost".to_string(), "127.0.0.1".to_string()];
        Ok(rcgen::generate_simple_self_signed(subject_alt_names)?)
    }

    async fn handle_connection(connection: quinn::Connection, dispatcher: Arc<RequestDispatcher>) {
        loop {
            match connection.accept_bi().await {
                Ok((tx, rx)) => {
                    Self::handle_stream(tx, rx, &dispatcher).await;
                }
                Err(quinn::ConnectionError::ApplicationClosed { .. }) => {
                    info!("Connection closed");
                    break;
                }
                Err(e) => {
                    error!("Error accepting incoming bidirectional stream: {}", e);
                    break;
                }
            }
        }
    }

    async fn handle_stream(
        mut tx: quinn::SendStream,
        mut rx: quinn::RecvStream,
        dispatcher: &Arc<RequestDispatcher>,
    ) {
        match rx.read_to_end(usize::MAX).await {
            Ok(data) => {
                if data.is_empty() {
                    return;
                }
                match dispatcher.dispatch(&data) {
                    Ok(response) => {
                        if !response.is_empty() {
                            if let Err(e) = tx.write_all(&response).await {
                                error!("Error writing response: {}", e);
                            }
                        }
                    }
                    Err(e) => {
                        error!("Error handling request: {}", e);
                    }
                }
            }
            Err(e) => {
                error!("Error reading incoming stream: {}", e);
            }
        }
    }
}

struct ConnectionControlRequestHandler;

impl RequestHandler for ConnectionControlRequestHandler {
    fn handle(&self, data: &[u8]) -> anyhow::Result<Box<[u8]>> {
        if data != b"QUAKE\x03" {
            return Err(anyhow::anyhow!("Invalid connection control request"));
        }

        info!("Received connection control request");
        Ok(vec![].into_boxed_slice())
    }
}

struct ServerInfoControlRequestHandler;

impl RequestHandler for ServerInfoControlRequestHandler {
    fn handle(&self, data: &[u8]) -> anyhow::Result<Box<[u8]>> {
        if data != b"QUAKE\x03" {
            return Err(anyhow::anyhow!("Invalid server info control request"));
        }

        info!("Received server info control request");
        Ok(vec![].into_boxed_slice())
    }
}
