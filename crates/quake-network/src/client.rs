use crate::{
    ACCEPT_CONNECTION_CONTROL_RESPONSE, CONNECTION_CONTROL_REQUEST, DISCONNECT_REQUEST,
    REJECT_CONNECTION_CONTROL_RESPONSE,
};
use bytes::{BufMut, BytesMut};
use std::net::ToSocketAddrs;
use tracing::log::info;

pub struct ClientManager {
    socket: std::net::UdpSocket,
}

impl ClientManager {
    pub fn new() -> anyhow::Result<Self> {
        Ok(Self {
            socket: std::net::UdpSocket::bind("0.0.0.0:0")?,
        })
    }

    pub fn connect<A>(&self, address: A) -> anyhow::Result<()>
    where
        A: ToSocketAddrs,
    {
        self.socket.connect(address)?;
        // Capture the peer address before receiving data to modify it later
        let mut remote_addr = self.socket.peer_addr()?;

        let mut buf = BytesMut::new();
        buf.put_u8(CONNECTION_CONTROL_REQUEST);
        buf.put_slice(b"QUAKE");
        buf.put_u8(3);
        self.socket.send(&buf)?;

        const BUFFER_SIZE: usize = 1024;
        let mut buf = [0u8; BUFFER_SIZE];
        let n = self.socket.recv(&mut buf)?;

        let data = &buf[..n];
        match data[0] {
            ACCEPT_CONNECTION_CONTROL_RESPONSE => {
                let port_bytes: [u8; 4] = data[1..5].try_into()?;
                let remote_port = u32::from_be_bytes(port_bytes) as u16;

                remote_addr.set_port(remote_port);
                self.socket.connect(remote_addr)?;

                info!("Connected to server at {}", remote_addr);

                Ok(())
            }
            REJECT_CONNECTION_CONTROL_RESPONSE => {
                Err(anyhow::anyhow!("Connection rejected by server"))
            }
            _ => Err(anyhow::anyhow!("Invalid connection control response")),
        }
    }

    pub fn reconnect(&self) -> anyhow::Result<()> {
        self.disconnect()?;
        self.connect(self.socket.local_addr().unwrap())?;

        Ok(())
    }

    pub fn disconnect(&self) -> anyhow::Result<()> {
        let mut buf = BytesMut::new();
        buf.put_u8(DISCONNECT_REQUEST);
        self.socket.send(&buf)?;

        Ok(())
    }
}
