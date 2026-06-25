use kobeat::{
    config::SOCKET_PATH,
    engine::{player_engine::PlayerEngine, symphonia_gst_player_engine::SymphoniaGstPlayerEngine},
};
use std::os::unix::net::UnixListener;
use std::path::Path;
use std::thread::JoinHandle;

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
    cleanup_stale_socket(SOCKET_PATH)?;
    let listener = UnixListener::bind(SOCKET_PATH)
        .with_context(|| format!("failed to bind socket at {SOCKET_PATH}"))?;
    let mut playback_thread: Option<JoinHandle<Result<()>>> = None;

    loop {
        reap_finished_playback(&mut playback_thread);
        let (mut stream, _addr) = listener
            .accept()
            .context("failed to accept client connection")?;

        let cmd =
            serde_json::from_reader(&mut stream).context("failed to parse command from client")?;

        let response = match cmd {
            kobeat::model::player_command::PlayerCommand::Play { path } => {
                match start_playback(&mut playback_thread, path) {
                    Ok(()) => "{\"ok\":true}\n",
                    Err(err) => {
                        eprintln!("{err:#}");
                        "{\"ok\":false,\"error\":\"playback already running\"}\n"
                    }
                }
            }
            kobeat::model::player_command::PlayerCommand::Resume => {
                match start_playback(
                    &mut playback_thread,
                    "/mnt/onboard/audiostuff/street_circus.mp3".to_string(),
                ) {
                    Ok(()) => "{\"ok\":true}\n",
                    Err(err) => {
                        eprintln!("{err:#}");
                        "{\"ok\":false,\"error\":\"playback already running\"}\n"
                    }
                }
            }
            kobeat::model::player_command::PlayerCommand::Pause => {
                SymphoniaGstPlayerEngine {}.pause();
                "{\"ok\":true}\n"
            }
            _ => {
                println!("Received command: {:?}", cmd);
                "{\"ok\":true}\n"
            }
        };

        use std::io::Write;
        stream.write_all(response.as_bytes())?;

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

fn start_playback(
    playback_thread: &mut Option<JoinHandle<Result<()>>>,
    path: String,
) -> Result<()> {
    reap_finished_playback(playback_thread);

    if playback_thread
        .as_ref()
        .is_some_and(|handle| !handle.is_finished())
    {
        anyhow::bail!("playback already running");
    }

    *playback_thread = Some(std::thread::spawn(move || {
        SymphoniaGstPlayerEngine {}.play(path)
    }));

    Ok(())
}

fn reap_finished_playback(playback_thread: &mut Option<JoinHandle<Result<()>>>) {
    if !playback_thread
        .as_ref()
        .is_some_and(std::thread::JoinHandle::is_finished)
    {
        return;
    }

    let handle = playback_thread
        .take()
        .expect("playback thread exists when marked finished");

    match handle.join() {
        Ok(Ok(())) => {}
        Ok(Err(err)) => eprintln!("Playback failed: {err:#}"),
        Err(_) => eprintln!("Playback thread panicked"),
    }
}
