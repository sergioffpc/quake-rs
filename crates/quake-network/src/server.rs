use crate::{RequestDispatcher, RequestHandler};
use rustls::pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer};
use std::net::SocketAddr;
use std::sync::Arc;
use tracing::log::{error, info};

pub struct ServerManager {
    endpoint: quinn::Endpoint,
    dispatcher: Arc<RequestDispatcher>,
}

impl ServerManager {
    pub async fn new<P>(address: SocketAddr, cert_path: P, key_path: P) -> anyhow::Result<Self>
    where
        P: AsRef<std::path::Path>,
    {
        let (cert_der, cert_key) = Self::load_cert(cert_path, key_path)?;
        let mut server_config = quinn::ServerConfig::with_single_cert(vec![cert_der], cert_key)?;
        server_config.transport = Arc::new(quinn::TransportConfig::default());

        let endpoint = quinn::Endpoint::server(server_config, address)?;
        info!("Listening on {}", endpoint.local_addr()?);

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

    fn load_cert<P>(
        cert_path: P,
        key_path: P,
    ) -> anyhow::Result<(CertificateDer<'static>, PrivateKeyDer<'static>)>
    where
        P: AsRef<std::path::Path>,
    {
        let cert_pem = std::fs::read_to_string(cert_path.as_ref())
            .map_err(|e| anyhow::anyhow!("Failed to read certificate: {}", e))?;
        let key_pem = std::fs::read_to_string(key_path.as_ref())
            .map_err(|e| anyhow::anyhow!("Failed to read key: {}", e))?;

        let cert_der = rustls_pemfile::certs(&mut cert_pem.as_bytes())
            .collect::<Result<Vec<_>, _>>()?
            .into_iter()
            .next()
            .ok_or_else(|| anyhow::anyhow!("No certificate found in PEM"))?;

        let key_der = rustls_pemfile::pkcs8_private_keys(&mut key_pem.as_bytes())
            .collect::<Result<Vec<_>, _>>()?
            .into_iter()
            .next()
            .ok_or_else(|| anyhow::anyhow!("No key found in PEM"))?;

        info!(
            "Loaded certificates from files: {}, {}",
            cert_path.as_ref().display(),
            key_path.as_ref().display()
        );
        Ok((
            quinn::rustls::pki_types::CertificateDer::from(cert_der),
            quinn::rustls::pki_types::PrivateKeyDer::from(key_der),
        ))
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
