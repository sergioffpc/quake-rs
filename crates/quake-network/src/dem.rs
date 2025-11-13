use quake_resources::FromBytes;
use std::io::BufRead;

#[derive(Clone, Debug)]
pub struct Dem;

impl Dem {
    pub fn from_bytes(data: &[u8]) -> anyhow::Result<Self> {
        let mut reader = std::io::Cursor::new(data);

        let mut track_number_buffer = vec![];
        reader.read_until(b'\n', &mut track_number_buffer)?;
        let track_number = String::from_utf8_lossy(&track_number_buffer)
            .trim()
            .parse::<i32>()?;
        dbg!(&track_number);

        Ok(Self)
    }
}

impl FromBytes for Dem {
    fn from_bytes(bytes: &[u8]) -> anyhow::Result<Self> {
        Dem::from_bytes(bytes)
    }
}

struct MessageBlock {
    angles: glam::Vec3,
    messages: Vec<Message>,
}

struct Message;
