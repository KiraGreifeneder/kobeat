use kobeat::config::SOCKET_PATH;
use std::os::unix::net::UnixListener;
use std::path::Path;

use anyhow::{Context, Result};
use clap::Parser;

// use daemonize::Daemonize;
const PID_PATH: &str = "/tmp/kobeatd.pid";
const LOG_PATH: &str = "/tmp/kobeatd.log";

#[derive(Parser, Debug)]
struct Args {
    // --foreground    Run in foreground for debugging
    #[arg(long)]
    foreground: bool,

    // --daemon        Daemonize and run in background
    #[arg(long)]
    daemon: bool,
}

fn main() -> Result<()> {
    let args = Args::parse();

    if args.foreground && args.daemon {
        anyhow::bail!("use only one of --foreground or --daemon");
    }

    if args.daemon {
        daemonize_self()?;
    }

    run_server()
}

fn daemonize_self() -> Result<()> {
    // Minimal outline only.

    // If you decide to use the daemonize crate, it would look roughly like:
    //
    // let stdout = File::create(LOG_PATH)?;
    // let stderr = File::create(LOG_PATH)?;
    //
    // Daemonize::new()
    //     .pid_file(PID_PATH)
    //     .stdout(stdout)
    //     .stderr(stderr)
    //     .start()
    //     .context("failed to daemonize")?;
    //
    // For now, this function is just a placeholder so the rest of the
    // structure is clear.

    Ok(())
}

fn run_server() -> Result<()> {
    cleanup_stale_socket(SOCKET_PATH)?;

    let listener = UnixListener::bind(SOCKET_PATH)
        .with_context(|| format!("failed to bind socket at {SOCKET_PATH}"))?;

    let mut engine = PlayerEngine::new()?;

    loop {
        let (mut stream, _addr) = listener
            .accept()
            .context("failed to accept client connection")?;

        let cmd = read_command(&mut stream)?;
        let response = engine.handle_command(cmd)?;

        write_response(&mut stream, &response)?;

        // TODO replace this and actually handle the command
        use std::io::Write;
        stream.write_all(b"{\"ok\":true}\n")?;

        stream.shutdown(std::net::Shutdown::Write);
    }
}

fn cleanup_stale_socket(path: &str) -> Result<()> {
    let p = Path::new(path);
    if p.exists() {
        std::fs::remove_file(p).with_context(|| format!("failed to remove stale socket {path}"))?;
    }
    Ok(())
}
