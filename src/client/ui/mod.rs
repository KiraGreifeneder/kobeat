mod page;
mod routes;

use anyhow::{Context, Result, bail};
use std::time::Duration;
use tokio::net::TcpListener;
use tower_http::trace::TraceLayer;
use tracing::Level;
use tracing_subscriber::EnvFilter;

pub use routes::app;

const DEFAULT_BIND_ADDR: &str = "0.0.0.0:3000";

/// How long to wait for the initial TCP bind before giving up.
const BIND_TIMEOUT: Duration = Duration::from_secs(5);

/// Resolves the bind address, starts the web UI, and serves it until the
/// server stops or fails.
pub async fn run() -> Result<()> {
    init_logging();

    ensure_loopback_up();

    let bind_addr =
        std::env::var("KOBEATUI_BIND").unwrap_or_else(|_| DEFAULT_BIND_ADDR.to_string());

    println!("kobeat web UI binding to {bind_addr}...");

    let listener = match tokio::time::timeout(BIND_TIMEOUT, TcpListener::bind(&bind_addr)).await {
        Ok(Ok(listener)) => listener,
        Ok(Err(err)) => {
            return Err(err).with_context(|| format!("failed to bind web UI to {bind_addr}"));
        }
        Err(_) => {
            bail!("timed out after {BIND_TIMEOUT:?} binding web UI to {bind_addr}");
        }
    };

    println!("kobeat web UI listening on http://{bind_addr}");

    let app = app().layer(
        TraceLayer::new_for_http()
            .on_request(|request: &axum::http::Request<_>, _span: &tracing::Span| {
                tracing::info!(method = %request.method(), uri = %request.uri(), "incoming request");
            })
            .on_response(
                |response: &axum::http::Response<_>, latency: std::time::Duration, _span: &tracing::Span| {
                    tracing::info!(status = %response.status(), ?latency, "response sent");
                },
            ),
    );

    println!("kobeat web UI app built, starting to accept connections...");

    axum::serve(listener, app)
        .await
        .context("web UI server stopped unexpectedly")?;

    Ok(())
}

/// Sets up console logging. Verbosity is controlled with `RUST_LOG`
/// (e.g. `RUST_LOG=debug`); defaults to `info` so requests are always
/// visible without extra configuration.
fn init_logging() {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));

    let _ = tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_max_level(Level::TRACE)
        .try_init();
}

fn ensure_loopback_up() {
    let status = match run_ip(&["addr", "show", "lo"]) {
        Ok(status) => status,
        Err(err) => {
            tracing::warn!(error = %err, "failed to inspect loopback interface");
            return;
        }
    };

    if !status.contains("inet 127.0.0.1/8") {
        tracing::warn!("loopback interface has no 127.0.0.1 address, adding it");
        if let Err(err) = run_ip(&["addr", "add", "127.0.0.1/8", "dev", "lo"]) {
            tracing::warn!(error = %err, "failed to add 127.0.0.1/8 to loopback interface");
        }
    }

    if !status.contains("LOWER_UP") {
        tracing::warn!("loopback interface is down, bringing it up");
        if let Err(err) = run_ip(&["link", "set", "lo", "up"]) {
            tracing::warn!(error = %err, "failed to bring up loopback interface");
        }
    }
}

fn run_ip(args: &[&str]) -> Result<String> {
    let output = std::process::Command::new("ip")
        .args(args)
        .output()
        .with_context(|| format!("failed to run `ip {}`", args.join(" ")))?;

    if !output.status.success() {
        bail!(
            "`ip {}` exited with {}: {}",
            args.join(" "),
            output.status,
            String::from_utf8_lossy(&output.stderr)
        );
    }

    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

#[cfg(test)]
mod tests {
    use super::DEFAULT_BIND_ADDR;

    #[test]
    fn default_bind_addr_is_wildcard() {
        assert_eq!(DEFAULT_BIND_ADDR, "0.0.0.0:3000");
    }
}
