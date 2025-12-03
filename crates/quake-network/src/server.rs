use crate::requests::{
    ConnectionControlRequestHandler, RequestDispatcher, ServerInfoControlRequestHandler,
};
use rustls::pki_types::{CertificateDer, PrivateKeyDer};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::broadcast;
use tracing::instrument;
use tracing::log::{error, info, warn};

pub struct ServerManager {
    endpoint: quinn::Endpoint,
    dispatcher: Arc<RequestDispatcher>,
    broadcast_tx: broadcast::Sender<Vec<u8>>,
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

        let (broadcast_tx, _) = broadcast::channel(512);

        Ok(Self {
            endpoint,
            dispatcher: Arc::new(dispatcher),
            broadcast_tx,
        })
    }

    pub async fn accept(&self) {
        while let Some(incoming) = self.endpoint.accept().await {
            let dispatcher = self.dispatcher.clone();
            let broadcast_tx = self.broadcast_tx.clone();

            tokio::spawn(async move {
                match incoming.await {
                    Ok(connection) => {
                        info!("Incoming connection from {:?}", connection.remote_address());
                        Self::handle_connection(connection, dispatcher, broadcast_tx).await;
                    }
                    Err(e) => error!("Error accepting connection: {}", e),
                }
            });
        }
    }

    pub async fn broadcast(&self, message: Vec<u8>) -> anyhow::Result<()> {
        self.broadcast_tx.send(message)?;
        Ok(())
    }

    #[instrument(skip_all, fields(remote_addr = %connection.remote_address()))]
    async fn handle_connection(
        connection: quinn::Connection,
        dispatcher: Arc<RequestDispatcher>,
        broadcast_tx: broadcast::Sender<Vec<u8>>,
    ) {
        let mut broadcast_rx = broadcast_tx.subscribe();

        loop {
            tokio::select! {
                // Handle incoming streams
                result = connection.accept_bi() => {
                    match result {
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

                // Listen for broadcast messages
                result = broadcast_rx.recv() => {
                    match result {
                        Ok(message) => {
                            if let Ok(mut tx) = connection.open_uni().await {
                                if let Err(e) = tx.write_all(&message).await {
                                    error!("Failed to send broadcast: {}", e);
                                }
                                let _ = tx.finish();
                            }
                        }
                        Err(broadcast::error::RecvError::Lagged(_)) => {
                            warn!("Broadcast channel lagged");
                        }
                        Err(_) => break,
                    }
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
