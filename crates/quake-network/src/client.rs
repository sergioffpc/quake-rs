use crate::server::ServerManager;
use quinn::VarInt;
use std::sync::Arc;
use tracing::log::info;

pub struct BidirectionalStream {
    send: quinn::SendStream,
    recv: quinn::RecvStream,
}

impl BidirectionalStream {
    pub async fn read(&mut self) -> anyhow::Result<Box<[u8]>> {
        Ok(self.recv.read_to_end(usize::MAX).await?.into_boxed_slice())
    }

    pub async fn write(&mut self, data: &[u8]) -> anyhow::Result<()> {
        self.send.write_all(data).await?;
        Ok(())
    }

    pub fn finish(&mut self) -> anyhow::Result<()> {
        self.send.finish()?;
        Ok(())
    }
}

pub struct ClientManager {
    endpoint: quinn::Endpoint,
    connection: Option<quinn::Connection>,
}

impl ClientManager {
    pub async fn new() -> anyhow::Result<Self> {
        let server_cert = ServerManager::cert()?;
        let mut root_certs = rustls::RootCertStore::empty();
        root_certs.add(rustls::pki_types::CertificateDer::from(server_cert.cert))?;
        let config = quinn::ClientConfig::with_root_certificates(Arc::new(root_certs))?;

        let mut endpoint = quinn::Endpoint::client("0.0.0.0:0".parse()?)?;
        endpoint.set_default_client_config(config);

        Ok(Self {
            endpoint,
            connection: None,
        })
    }

    pub async fn connect(&mut self, address: std::net::SocketAddr) -> anyhow::Result<()> {
        info!("Connecting to {:?}", address);
        let connection = self.endpoint.connect(address, "localhost")?.await?;
        self.connection = Some(connection);

        Ok(())
    }

    pub async fn disconnect(&mut self) -> anyhow::Result<()> {
        if let Some(connection) = self.connection.take() {
            connection.close(VarInt::from_u32(0), b"disconnected");
            self.endpoint.wait_idle().await;
            Ok(())
        } else {
            Err(anyhow::anyhow!("Not connected"))
        }
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

    pub async fn create_bidirectional_stream(&self) -> anyhow::Result<BidirectionalStream> {
        let connection = self.connection.as_ref().ok_or_else(|| {
            anyhow::anyhow!("Unable to create bidirectional stream: not connected")
        })?;
        let (send, recv) = connection.open_bi().await?;
        Ok(BidirectionalStream { send, recv })
    }
}
