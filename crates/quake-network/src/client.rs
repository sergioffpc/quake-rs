use crate::PacketDispatcher;
use std::fs;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::log::{error, info, warn};

pub struct ClientManager {
    endpoint: quinn::Endpoint,
    connection: Option<quinn::Connection>,
    packet_dispatcher: Arc<Mutex<PacketDispatcher>>,
}

impl ClientManager {
    pub async fn new<P>(
        ca_path: Option<P>,
        packet_dispatcher: Arc<Mutex<PacketDispatcher>>,
    ) -> anyhow::Result<Self>
    where
        P: AsRef<std::path::Path>,
    {
        let client_config = match ca_path {
            Some(ca_path) => {
                let ca_pem = fs::read_to_string(ca_path.as_ref())?;
                let mut root_certs = rustls::RootCertStore::empty();
                let certs =
                    rustls_pemfile::certs(&mut ca_pem.as_bytes()).collect::<Result<Vec<_>, _>>()?;
                for cert in certs {
                    root_certs.add(cert)?;
                }
                quinn::ClientConfig::with_root_certificates(Arc::new(root_certs))?
            }
            None => quinn::ClientConfig::new(Arc::new(
                quinn_proto::crypto::rustls::QuicClientConfig::try_from(
                    rustls::ClientConfig::builder()
                        .dangerous()
                        .with_custom_certificate_verifier(SkipServerVerification::new())
                        .with_no_client_auth(),
                )?,
            )),
        };

        let mut endpoint = quinn::Endpoint::client("0.0.0.0:0".parse()?)?;
        endpoint.set_default_client_config(client_config);

        Ok(Self {
            endpoint,
            connection: None,
            packet_dispatcher,
        })
    }

    pub async fn connect(&mut self, address: std::net::SocketAddr) -> anyhow::Result<()> {
        info!("Connecting to {:?}", address);
        let connection = self.endpoint.connect(address, "localhost")?.await?;

        let (mut tx, mut rx) = connection.open_bi().await?;
        tx.write(b"\x01QUAKE\x03").await?;
        tx.finish()?;

        match rx.read_to_end(usize::MAX).await?.as_slice() {
            b"OK" => {
                info!("Connection control accepted");
                self.connection = Some(connection);
                tokio::spawn(Self::broadcast_listener(
                    self.connection.as_ref().unwrap().clone(),
                    self.packet_dispatcher.clone(),
                ));
            }
            _ => {
                warn!("Connection refused");
                self.connection = None;
            }
        }

        Ok(())
    }

    pub async fn disconnect(&mut self) -> anyhow::Result<()> {
        if let Some(connection) = self.connection.take() {
            connection.close(quinn::VarInt::from_u32(0), b"disconnected");
            self.endpoint.wait_idle().await;
        } else {
            info!("Not connected");
        }

        Ok(())
    }

    pub async fn reconnect(&mut self) -> anyhow::Result<()> {
        let address = self
            .connection
            .as_ref()
            .map(|conn| conn.remote_address())
            .ok_or_else(|| anyhow::anyhow!("Not connected"))?;
        self.disconnect().await?;
        self.connect(address).await
    }

    pub async fn channel(&self) -> anyhow::Result<(quinn::SendStream, quinn::RecvStream)> {
        self.connection
            .as_ref()
            .unwrap()
            .open_bi()
            .await
            .map_err(|e| anyhow::anyhow!(e))
    }

    async fn broadcast_listener(
        connection: quinn::Connection,
        packet_dispatcher: Arc<Mutex<PacketDispatcher>>,
    ) {
        info!("Start listening for messages on broadcast channel");
        loop {
            match connection.accept_uni().await {
                Ok(mut recv) => match recv.read_to_end(usize::MAX).await {
                    Ok(data) => {
                        if let Err(e) = packet_dispatcher
                            .lock()
                            .await
                            .dispatch(data.as_slice())
                            .await
                        {
                            error!("Error dispatching packet: {}", e);
                        }
                    }
                    Err(e) => {
                        error!("Error reading: {}", e);
                        break;
                    }
                },
                Err(quinn::ConnectionError::ApplicationClosed { .. }) => {
                    info!("Connection closed");
                    break;
                }
                Err(e) => {
                    error!("Connection error: {}", e);
                    break;
                }
            }
        }
    }
}

#[derive(Debug)]
struct SkipServerVerification(Arc<rustls::crypto::CryptoProvider>);

impl SkipServerVerification {
    fn new() -> Arc<Self> {
        Arc::new(Self(Arc::new(rustls::crypto::ring::default_provider())))
    }
}

impl rustls::client::danger::ServerCertVerifier for SkipServerVerification {
    fn verify_server_cert(
        &self,
        _end_entity: &rustls::pki_types::CertificateDer<'_>,
        _intermediates: &[rustls::pki_types::CertificateDer<'_>],
        _server_name: &rustls::pki_types::ServerName<'_>,
        _ocsp: &[u8],
        _now: rustls::pki_types::UnixTime,
    ) -> Result<rustls::client::danger::ServerCertVerified, rustls::Error> {
        Ok(rustls::client::danger::ServerCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        message: &[u8],
        cert: &rustls::pki_types::CertificateDer<'_>,
        dss: &rustls::DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        rustls::crypto::verify_tls12_signature(
            message,
            cert,
            dss,
            &self.0.signature_verification_algorithms,
        )
    }

    fn verify_tls13_signature(
        &self,
        message: &[u8],
        cert: &rustls::pki_types::CertificateDer<'_>,
        dss: &rustls::DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        rustls::crypto::verify_tls13_signature(
            message,
            cert,
            dss,
            &self.0.signature_verification_algorithms,
        )
    }

    fn supported_verify_schemes(&self) -> Vec<rustls::SignatureScheme> {
        self.0.signature_verification_algorithms.supported_schemes()
    }
}
