use crate::{StreamHandler, StreamHandlerBuilder};
use rustls::pki_types::{CertificateDer, PrivateKeyDer};
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tabled::Tabled;
use tokio::sync::broadcast;
use tracing::instrument;
use tracing::log::{error, info, warn};

#[derive(Copy, Debug, Clone, Tabled)]
pub struct ServerStats {
    pub open_connections: u64,
    pub accepted_handshakes: u64,
    pub outgoing_handshakes: u64,
    pub refused_handshakes: u64,
    pub ignored_handshakes: u64,
}

pub struct ServerManager {
    endpoint: quinn::Endpoint,
    broadcast_sender: broadcast::Sender<Vec<u8>>,

    console_manager: Arc<quake_console::ConsoleManager>,
}

impl ServerManager {
    pub async fn new<P>(
        address: SocketAddr,
        cert_path: P,
        key_path: P,
        console_manager: Arc<quake_console::ConsoleManager>,
    ) -> anyhow::Result<Self>
    where
        P: AsRef<std::path::Path>,
    {
        let (cert_der, cert_key) = Self::load_cert(cert_path, key_path)?;
        let mut server_config = quinn::ServerConfig::with_single_cert(vec![cert_der], cert_key)?;

        let mut transport_config = quinn::TransportConfig::default();
        let net_messagetimeout = console_manager
            .get::<u64>("net_messagetimeout")
            .await
            .unwrap_or(15);
        transport_config
            .max_idle_timeout(Some(Duration::from_secs(net_messagetimeout).try_into()?));
        let net_keepaliveinterval = console_manager
            .get::<u64>("net_keepaliveinterval")
            .await
            .unwrap_or(5);
        transport_config
            .keep_alive_interval(Some(Duration::from_secs(net_keepaliveinterval).try_into()?));
        server_config.transport = Arc::new(transport_config);

        let endpoint = quinn::Endpoint::server(server_config, address)?;
        info!("Listening on {}", endpoint.local_addr()?);

        let (broadcast_sender, _) = broadcast::channel(512);

        Ok(Self {
            endpoint,
            broadcast_sender,
            console_manager,
        })
    }

    pub async fn accept<B>(&self, builder: B) -> anyhow::Result<()>
    where
        B: StreamHandlerBuilder,
    {
        while let Some(incoming) = self.endpoint.accept().await {
            let net_maxconnections = self
                .console_manager
                .get::<usize>("net_maxconnections")
                .await
                .unwrap_or(128);
            if self.endpoint.open_connections() > net_maxconnections {
                warn!("Maximum number of connections reached, rejecting new connection");
                continue;
            }

            let broadcast_receiver = self.broadcast_sender.subscribe();
            let stream_handler = builder.build().await?;

            tokio::spawn(async move {
                match incoming.await {
                    Ok(connection) => {
                        let remote_addr = connection.remote_address();
                        info!("Incoming connection from {}", remote_addr);

                        Self::handle_connection(connection, broadcast_receiver, stream_handler)
                            .await
                            .unwrap();
                    }
                    Err(e) => error!("Error accepting connection: {}", e),
                }
            });
        }

        Ok(())
    }

    pub async fn broadcast(&self, message: Vec<u8>) -> anyhow::Result<()> {
        self.broadcast_sender.send(message)?;
        Ok(())
    }

    pub fn stats(&self) -> ServerStats {
        let stats = self.endpoint.stats();
        ServerStats {
            open_connections: self.endpoint.open_connections() as u64,
            accepted_handshakes: stats.accepted_handshakes,
            outgoing_handshakes: stats.outgoing_handshakes,
            refused_handshakes: stats.refused_handshakes,
            ignored_handshakes: stats.ignored_handshakes,
        }
    }

    #[instrument(skip_all, fields(remote_addr = %connection.remote_address()))]
    async fn handle_connection(
        connection: quinn::Connection,
        mut broadcast_receiver: broadcast::Receiver<Vec<u8>>,
        stream_handler: Box<dyn StreamHandler>,
    ) -> anyhow::Result<()> {
        loop {
            tokio::select! {
                // Handle incoming streams
                result = connection.accept_bi() => {
                    match result {
                        Ok((mut sender, mut receiver)) => {
                            stream_handler.handle_stream(&mut sender, &mut receiver).await;
                        }
                        Err(quinn::ConnectionError::ApplicationClosed { .. }) => {
                            info!("Connection closed");
                            break;
                        }
                        Err(e) => {
                            error!("Connection closed: {}", e);
                            break;
                        }
                    }
                }

                // Listen for broadcast messages
                result = broadcast_receiver.recv() => {
                    match result {
                        Ok(message) => {
                            if let Ok(mut sender) = connection.open_uni().await {
                                if let Err(e) = sender.write_all(&message).await {
                                    error!("Failed to send broadcast: {}", e);
                                }
                                let _ = sender.finish();
                            }
                        }
                        Err(broadcast::error::RecvError::Lagged(_)) => {
                            warn!("Broadcast channel lagged");
                        }
                        Err(e) => {
                            error!("Broadcast channel closed: {}", e);
                            break
                        },
                    }
                }
            }
        }

        Ok(())
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
