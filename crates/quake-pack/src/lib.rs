pub mod pak;
pub mod wad;

pub trait FromBytes: Sized {
    fn from_bytes(bytes: &[u8]) -> anyhow::Result<Self>;
}

impl FromBytes for Vec<u8> {
    fn from_bytes(data: &[u8]) -> anyhow::Result<Self> {
        Ok(data.to_vec())
    }
}

impl FromBytes for String {
    fn from_bytes(data: &[u8]) -> anyhow::Result<Self> {
        Ok(String::from_utf8_lossy(data).to_string())
    }
}

pub(crate) fn read_null_terminated_string<R>(
    reader: &mut R,
    buffer_size: usize,
) -> anyhow::Result<String>
where
    R: std::io::Read,
{
    let mut name_buffer = vec![0u8; buffer_size];
    reader.read_exact(&mut name_buffer)?;
    let null_terminated_bytes: Vec<u8> = name_buffer
        .iter()
        .take_while(|&byte| *byte != 0)
        .copied()
        .collect();
    Ok(String::from_utf8_lossy(&null_terminated_bytes).to_string())
}
