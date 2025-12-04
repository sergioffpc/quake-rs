#[async_trait::async_trait]
pub trait FromBytes: Sized + Sync + Send {
    async fn from_bytes(bytes: &[u8]) -> anyhow::Result<Self>;
}

#[async_trait::async_trait]
impl FromBytes for Vec<u8> {
    async fn from_bytes(data: &[u8]) -> anyhow::Result<Self> {
        Ok(data.to_vec())
    }
}

#[async_trait::async_trait]
impl FromBytes for String {
    async fn from_bytes(data: &[u8]) -> anyhow::Result<Self> {
        Ok(String::from_utf8_lossy(data).to_string())
    }
}

#[async_trait::async_trait]
pub trait CommandHandler: Send + Sync {
    async fn handle_command(&mut self, command: &[&str]) -> anyhow::Result<(&[u8], ControlFlow)>;
}

#[derive(Copy, Clone, Debug, Default, PartialEq)]
pub enum ControlFlow {
    #[default]
    Poll,
    Wait,
}
