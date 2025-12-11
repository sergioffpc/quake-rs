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
            match reader.read_u32_le().await {
                Ok(block_size) if block_size > 0 => {
                    let angles = read_f32_vector3(&mut reader).await?;
                    let mut messages = vec![0u8; block_size as usize];
                    reader.read_exact(&mut messages).await?;

                    blocks.push(Block { angles, messages });
                }
                _ => break,
            }
        }

        Ok(Self {
            track,
            blocks: blocks.into_boxed_slice(),
        })
    }

    pub fn into_iter(self) -> impl Iterator<Item = Block> {
        DemIterator {
            blocks: self.blocks,
            index: 0,
        }
    }
}

#[async_trait::async_trait]
impl quake_traits::FromBytes for Dem {
    async fn from_bytes(bytes: &[u8]) -> anyhow::Result<Self> {
        Dem::from_slice(bytes).await
    }
}

#[derive(Clone, Debug)]
pub struct Block {
    pub angles: glam::Vec3,
    pub messages: Vec<u8>,
}

struct DemIterator {
    blocks: Box<[Block]>,
    index: usize,
}

impl Iterator for DemIterator {
    type Item = Block;

    fn next(&mut self) -> Option<Self::Item> {
        if self.index >= self.blocks.len() {
            None
        } else {
            let block = self.blocks[self.index].clone();
            self.index += 1;
            Some(block)
        }
    }
}
