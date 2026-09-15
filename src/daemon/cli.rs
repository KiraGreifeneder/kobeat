use anyhow::Result;
use clap::Parser;

use crate::{config::SOCKET_PATH, daemon::player_server::PlayerServer};

// use daemonize::Daemonize;
// const PID_PATH: &str = "/tmp/kobeatd.pid";
// const LOG_PATH: &str = "/tmp/kobeatd.log";

#[derive(Parser, Debug)]
pub struct Args {
    /// Run in foreground for debugging
    #[arg(long)]
    pub foreground: bool,

    /// Daemonize and run in background
    #[arg(long)]
    pub daemon: bool,
}

/// Parses CLI args and runs the daemon accordingly.
pub fn run() -> Result<()> {
    let args = Args::parse();

    if args.foreground && args.daemon {
        anyhow::bail!("use only one of --foreground or --daemon");
    }

    if args.daemon {
        daemonize_self()?;
    }

    PlayerServer::new().run(SOCKET_PATH)
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
