use std::io::Write;
use std::net::Shutdown;
use std::os::unix::net::UnixListener;
use std::path::Path;
use std::thread::JoinHandle;

use anyhow::{Context, Result};

use crate::{
    daemon::player::{Player, symphonia_gst_player::SymphoniaGstPlayer},
    domain::{
        player_command::PlayerCommand,
        state::{PlaybackState, PlayerState},
    },
};

/// Owns the daemon's command loop: binds the control socket, accepts client
/// connections, and dispatches `PlayerCommand`s to the playback backend.
pub struct PlayerServer {
    playback_thread: Option<JoinHandle<Result<()>>>,
    session_state: PlayerState,
}

impl PlayerServer {
    pub fn new() -> Self {
        Self {
            playback_thread: None,
            session_state: PlayerState {
                current_track: None,
                playback_state: PlaybackState::Stopped,
                base_position_secs: 0.0,
            },
        }
    }

    /// Binds `socket_path` and serves client connections until an error occurs.
    pub fn run(mut self, socket_path: &str) -> Result<()> {
        cleanup_stale_socket(socket_path)?;
        let listener = UnixListener::bind(socket_path)
            .with_context(|| format!("failed to bind socket at {socket_path}"))?;

        loop {
            self.reap_finished_playback();

            let (mut stream, _addr) = listener
                .accept()
                .context("failed to accept client connection")?;

            let cmd: PlayerCommand = serde_json::from_reader(&mut stream)
                .context("failed to parse command from client")?;

            let response = self.handle_command(cmd);

            stream.write_all(response.as_bytes())?;
            stream
                .shutdown(Shutdown::Write)
                .context("failed to shutdown client connection")?;
        }
    }

    fn handle_command(&mut self, cmd: PlayerCommand) -> &'static str {
        match cmd {
            PlayerCommand::LoadAndPlay { path } => match self.start_playback(path) {
                Ok(()) => "{\"ok\":true}\n",
                Err(err) => {
                    eprintln!("{err:#}");
                    "{\"ok\":false,\"error\":\"playback already running\"}\n"
                }
            },
            PlayerCommand::Resume => {
                let path = "/mnt/onboard/audiostuff/street_circus.mp3".to_string();
                match self.start_playback(path) {
                    Ok(()) => "{\"ok\":true}\n",
                    Err(err) => {
                        eprintln!("{err:#}");
                        "{\"ok\":false,\"error\":\"playback already running\"}\n"
                    }
                }
            }
            PlayerCommand::Pause => {
                SymphoniaGstPlayer {}.pause();
                "{\"ok\":true}\n"
            }
            other => {
                println!("Received command: {other:?}");
                "{\"ok\":true}\n"
            }
        }
    }

    fn start_playback(&mut self, path: String) -> Result<()> {
        self.reap_finished_playback();

        if self
            .playback_thread
            .as_ref()
            .is_some_and(|handle| !handle.is_finished())
        {
            anyhow::bail!("playback already running");
        }

        self.playback_thread = Some(std::thread::spawn(move || {
            let result = SymphoniaGstPlayer {}.play_from_start(path);

            if let Err(err) = &result {
                eprintln!("Playback failed immediately: {err:#}");
            }

            result
        }));

        Ok(())
    }

    fn reap_finished_playback(&mut self) {
        if !self
            .playback_thread
            .as_ref()
            .is_some_and(JoinHandle::is_finished)
        {
            return;
        }

        let handle = self
            .playback_thread
            .take()
            .expect("playback thread exists when marked finished");

        match handle.join() {
            Ok(Ok(())) => {}
            Ok(Err(err)) => eprintln!("Playback failed: {err:#}"),
            Err(_) => eprintln!("Playback thread panicked"),
        }
    }
}

impl Default for PlayerServer {
    fn default() -> Self {
        Self::new()
    }
}

fn cleanup_stale_socket(path: &str) -> Result<()> {
    let p = Path::new(path);
    if p.exists() {
        std::fs::remove_file(p).with_context(|| format!("failed to remove stale socket {path}"))?;
    }
    Ok(())
}
