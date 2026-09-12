mod page;
mod routes;

use anyhow::{Context, Result};
use tokio::net::TcpListener;

pub use routes::app;

use crate::domain::state::PlayerState;

const DEFAULT_BIND_ADDR: &str = "0.0.0.0:3000";

/// Resolves the bind address, starts the web UI, and serves it until the
/// server stops or fails. This is the sole entry point the `kobeatui`
/// binary calls from `main()`.
pub async fn run() -> Result<()> {
    let bind_addr =
        std::env::var("KOBEATUI_BIND").unwrap_or_else(|_| DEFAULT_BIND_ADDR.to_string());
    let listener = TcpListener::bind(&bind_addr)
        .await
        .with_context(|| format!("failed to bind web UI to {bind_addr}"))?;

    println!("kobeat web UI listening on http://{bind_addr}");

    axum::serve(listener, app())
        .await
        .context("web UI server stopped unexpectedly")?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::DEFAULT_BIND_ADDR;

    #[test]
    fn default_bind_addr_is_wildcard() {
        assert_eq!(DEFAULT_BIND_ADDR, "0.0.0.0:3000");
    }
}
