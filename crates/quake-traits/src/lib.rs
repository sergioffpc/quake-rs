#[async_trait::async_trait]
pub trait CommandHandler: Send + Sync {
    async fn handle_command(&mut self, command: &[&str]) -> anyhow::Result<ControlFlow>;
}

#[derive(Copy, Clone, Debug, Default, PartialEq)]
pub enum ControlFlow {
    #[default]
    Poll,
    Wait,
}
