use std::fs;
use std::sync::Arc;
use tracing::log::{error, info};

pub struct ClientManager {
    endpoint: quinn::Endpoint,
    connection: Option<quinn::Connection>,
}

impl ClientManager {
    pub async fn new<P>(ca_path: P) -> anyhow::Result<Self>
    where
        P: AsRef<std::path::Path>,
    {
        let ca_pem = fs::read_to_string(ca_path.as_ref())?;
        let mut root_certs = rustls::RootCertStore::empty();
        let certs = rustls_pemfile::certs(&mut ca_pem.as_bytes()).collect::<Result<Vec<_>, _>>()?;
        for cert in certs {
            root_certs.add(cert)?;
        }
        let client_config = quinn::ClientConfig::with_root_certificates(Arc::new(root_certs))?;

        let mut endpoint = quinn::Endpoint::client("0.0.0.0:0".parse()?)?;
        endpoint.set_default_client_config(client_config);

        Ok(Self {
            endpoint,
            connection: None,
        })
    }

    pub async fn connect(&mut self, address: std::net::SocketAddr) -> anyhow::Result<()> {
        info!("Connecting to {:?}", address);
        let connection = self.endpoint.connect(address, "localhost")?.await?;
        self.connection = Some(connection);

        let (mut tx, mut rx) = self.open_stream().await?;
        tx.write(b"\x01QUAKE\x03").await?;
        tx.finish()?;

        match rx.read_to_end(usize::MAX).await?.as_slice() {
            b"\x81" => {
                info!("Connection control accepted");
                tokio::spawn(Self::broadcast_listener(
                    self.connection.as_ref().unwrap().clone(),
                ));
            }
            _ => unreachable!("Invalid connection control response"),
        }

        Ok(())
    }

    pub async fn disconnect(&mut self) -> anyhow::Result<()> {
        if let Some(connection) = self.connection.take() {
            connection.close(quinn::VarInt::from_u32(0), b"disconnected");
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

    pub async fn open_stream(&self) -> anyhow::Result<(quinn::SendStream, quinn::RecvStream)> {
        let connection = self.connection.as_ref().ok_or_else(|| {
            anyhow::anyhow!("Unable to create bidirectional stream: not connected")
        })?;
        Ok(connection.open_bi().await?)
    }

    async fn broadcast_listener(connection: quinn::Connection) {
        info!("Start listening for messages on broadcast channel");
        loop {
            match connection.accept_uni().await {
                Ok(mut recv) => match recv.read_to_end(usize::MAX).await {
                    Ok(data) => {
                        info!("Server message: {:?}", String::from_utf8_lossy(&data));
                    }
                    Err(e) => {
                        error!("Error reading: {}", e);
                        break;
                    }
                },
                Err(e) => {
                    error!("Connection closed: {}", e);
                    break;
                }
            }
        }
    }
}
