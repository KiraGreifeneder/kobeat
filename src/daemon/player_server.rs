use std::io::Write;
use std::net::Shutdown;
use std::os::unix::net::UnixListener;
use std::path::Path;

use anyhow::{Context, Result};

use crate::daemon::queue::Queue;
use crate::{
    daemon::player::{Player, symphonia_gst_player::SymphoniaGstPlayer},
    domain::{player_command::PlayerCommand, state::PlaybackOutcome},
};

/// Owns the daemon's command loop: binds the control socket, accepts client
/// connections, and dispatches `PlayerCommand`s to the playback backend.
pub struct PlayerServer {
    player: Box<dyn Player>,
    queue: Queue,
}

impl PlayerServer {
    pub fn new() -> Self {
        Self {
            player: Box::new(SymphoniaGstPlayer::new()),
            queue: Queue::default(),
        }
    }

    /// Binds `socket_path` and serves client connections until an error occurs.
    pub fn run(mut self, socket_path: &str) -> Result<()> {
        cleanup_stale_socket(socket_path)?;
        let listener = UnixListener::bind(socket_path)
            .with_context(|| format!("failed to bind socket at {socket_path}"))?;

        loop {
            self.advance_queue_if_finished();

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
            PlayerCommand::LoadAndPlay { path } => {
                reply(self.player.load_and_play(path), "failed to start playback")
            }
            PlayerCommand::Resume => reply(self.player.resume(), "failed to resume playback"),
            PlayerCommand::Seek { position_secs } => {
                reply(self.player.seek(position_secs as f64), "failed to seek")
            }
            PlayerCommand::Pause => {
                self.player.pause();
                "{\"ok\":true}\n"
            }
            PlayerCommand::GetPlayerState => {
                self.player.get_state();
                "{\"ok\":true}\n"
            }
            PlayerCommand::PlayNext => match self.queue.next() {
                Some(next) => reply(
                    self.player.load_and_play(next),
                    "failed to start next track",
                ),
                None => "{\"ok\":false,\"error\":\"no next track in queue\"}\n",
            },
            PlayerCommand::PlayPrevious => match self.queue.previous() {
                Some(prev) => reply(
                    self.player.load_and_play(prev),
                    "failed to start previous track",
                ),
                None => "{\"ok\":false,\"error\":\"no previous track in queue\"}\n",
            },
            PlayerCommand::SetQueue { paths } => {
                self.queue.set_queue(paths);
                "{\"ok\":true}\n"
            }
            PlayerCommand::Shutdown => {
                println!("Shutting down player server");
                std::process::exit(0);
            }
        }
    }

    /// Moves to the next queued track once the current one plays out.
    fn advance_queue_if_finished(&mut self) {
        if self.player.poll_finished() != Some(PlaybackOutcome::Finished) {
            return;
        }

        let Some(next) = self.queue.next() else {
            println!("Finished playback, no more tracks in queue");
            return;
        };

        println!("Finished playback, starting next track: {next}");
        if let Err(err) = self.player.load_and_play(next) {
            eprintln!("failed to start next track in queue: {err:#}");
        }
    }
}

impl Default for PlayerServer {
    fn default() -> Self {
        Self::new()
    }
}

fn reply(result: Result<()>, context: &'static str) -> &'static str {
    match result {
        Ok(()) => "{\"ok\":true}\n",
        Err(err) => {
            eprintln!("{context}: {err:#}");
            "{\"ok\":false,\"error\":\"playback command failed\"}\n"
        }
    }
}

fn cleanup_stale_socket(path: &str) -> Result<()> {
    let p = Path::new(path);
    if p.exists() {
        std::fs::remove_file(p).with_context(|| format!("failed to remove stale socket {path}"))?;
    }
    Ok(())
}
