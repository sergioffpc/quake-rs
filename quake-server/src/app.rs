use std::time::Duration;
use tokio::time::sleep;

pub struct App;

pub async fn run_app() -> anyhow::Result<()> {
    loop {
        println!("Daemon tick");
        sleep(Duration::from_secs(5)).await;
    }
}
