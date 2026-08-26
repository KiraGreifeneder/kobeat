use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    kobeat::client::ui::run().await
}
