use kobeat::{
    config::SOCKET_PATH,
    daemon::engine::{GstPlayerEngine, PlayerEngine},
};
use std::os::unix::net::UnixListener;
use std::path::Path;

use anyhow::{Context, Result};
use clap::Parser;

// use daemonize::Daemonize;
// const PID_PATH: &str = "/tmp/kobeatd.pid";
// const LOG_PATH: &str = "/tmp/kobeatd.log";

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
    let engine = GstPlayerEngine {};

    loop {
        cleanup_stale_socket(SOCKET_PATH)?;

        let listener = UnixListener::bind(SOCKET_PATH)
            .with_context(|| format!("failed to bind socket at {SOCKET_PATH}"))?;
        let (mut stream, _addr) = listener
            .accept()
            .context("failed to accept client connection")?;

        let cmd =
            serde_json::from_reader(&mut stream).context("failed to parse command from client")?;

        match cmd {
            kobeat::model::player_command::PlayerCommand::Play { path } => {
                engine.play(path);
            }
            kobeat::model::player_command::PlayerCommand::Pause => {
                engine.pause();
            }
            _ => {
                // For now, just print the command and return a generic response.
                println!("Received command: {:?}", cmd);
            }
        }

        // TODO replace this and actually handle the command
        use std::io::Write;
        stream.write_all(b"{\"ok\":true}\n")?;

        stream
            .shutdown(std::net::Shutdown::Write)
            .context("failed to shutdown client connection")?;
    }
}

fn cleanup_stale_socket(path: &str) -> Result<()> {
    let p = Path::new(path);
    if p.exists() {
        std::fs::remove_file(p).with_context(|| format!("failed to remove stale socket {path}"))?;
    }
    Ok(())
}
