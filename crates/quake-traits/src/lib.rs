pub trait FromBytes: Sized + Sync + Send {
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

pub trait CommandHandler: Send + Sync {
    fn handle_command(&mut self, command: &[&str]) -> anyhow::Result<ControlFlow>;
}

#[derive(Copy, Clone, Debug, Default, PartialEq)]
pub enum ControlFlow {
    #[default]
    Poll,
    Wait,
}
