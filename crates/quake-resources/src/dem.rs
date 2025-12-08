use crate::{read_f32_vector3, read_string};
use std::io::Cursor;
use tokio::io::AsyncReadExt;

pub struct Dem {
    track: String,
    blocks: Box<[Block]>,
}

impl Dem {
    pub async fn from_slice(data: &[u8]) -> anyhow::Result<Self> {
        let mut reader = Cursor::new(data);

        let track = read_string(&mut reader).await?;

        let mut blocks = Vec::new();
        loop {
            let block_size = reader.read_u32_le().await?;
            if block_size == 0 {
                break;
            }
            let angles = read_f32_vector3(&mut reader).await?;
            let mut message_buffer = vec![0u8; block_size as usize];
            reader.read_exact(&mut message_buffer).await?;

            blocks.push(Block {
                angles,
                messages: message_buffer.into_boxed_slice(),
            });
        }

        Ok(Self {
            track,
            blocks: blocks.into_boxed_slice(),
        })
    }
}

#[async_trait::async_trait]
impl quake_traits::FromBytes for Dem {
    async fn from_bytes(bytes: &[u8]) -> anyhow::Result<Self> {
        Dem::from_slice(bytes).await
    }
}

struct Block {
    angles: glam::Vec3,
    messages: Box<[u8]>,
}
