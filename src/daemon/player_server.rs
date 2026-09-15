use std::io::Write;
use std::net::Shutdown;
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::Path;
use std::time::Duration;

use anyhow::{Context, Result};

use crate::daemon::queue::Queue;
use crate::{
    daemon::player::{Player, symphonia_gst_player::SymphoniaGstPlayer},
    domain::{
        player_command::{PlayerCommand, PlayerResponse},
        state::PlaybackOutcome,
    },
};

/// How long to wait between checks for finished playback while no client is
/// connected.
const POLL_INTERVAL: Duration = Duration::from_millis(200);

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
        listener
            .set_nonblocking(true)
            .context("failed to make listener non-blocking")?;

        loop {
            self.advance_queue_if_finished();

            match listener.accept() {
                Ok((stream, _addr)) => self.serve_client(stream),
                // No client waiting: keep looping so playback is still polled.
                Err(err) if err.kind() == std::io::ErrorKind::WouldBlock => {
                    std::thread::sleep(POLL_INTERVAL);
                }
                Err(err) => eprintln!("failed to accept client connection: {err:#}"),
            }
        }
    }

    /// Reads one command off `stream` and writes back the response. Errors
    /// are logged rather than propagated so one bad client can't take the
    /// daemon down.
    fn serve_client(&mut self, mut stream: UnixStream) {
        // Undo the non-blocking mode inherited from the listener, so reading
        // the command blocks until it has actually arrived.
        if let Err(err) = stream.set_nonblocking(false) {
            eprintln!("failed to make client stream blocking: {err:#}");
            return;
        }

        let response = match serde_json::from_reader(&mut stream) {
            Ok(cmd) => self.handle_command(cmd),
            Err(err) => PlayerResponse::Error(format!("failed to parse command: {err:#}")),
        };

        if let Err(err) = write_response(&mut stream, &response) {
            eprintln!("failed to respond to client: {err:#}");
        }
    }

    fn handle_command(&mut self, cmd: PlayerCommand) -> PlayerResponse {
        match cmd {
            PlayerCommand::LoadAndPlay { path } => reply(self.player.load_and_play(path), "playing"),
            PlayerCommand::Resume => reply(self.player.resume(), "resumed"),
            PlayerCommand::Seek { position_secs } => {
                reply(self.player.seek(position_secs as f64), "seeked")
            }
            PlayerCommand::Pause => {
                self.player.pause();
                PlayerResponse::Ok("paused".into())
            }
            PlayerCommand::GetPlayerState => PlayerResponse::State(self.player.get_state()),
            PlayerCommand::PlayNext => match self.queue.next() {
                Some(next) => reply(self.player.load_and_play(next), "playing next"),
                None => PlayerResponse::Error("no next track in queue".into()),
            },
            PlayerCommand::PlayPrevious => match self.queue.previous() {
                Some(prev) => reply(self.player.load_and_play(prev), "playing previous"),
                None => PlayerResponse::Error("no previous track in queue".into()),
            },
            PlayerCommand::SetQueue { paths } => {
                let count = paths.len();
                self.queue.set_queue(paths);
                PlayerResponse::Ok(format!("queued {count} tracks"))
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

fn reply(result: Result<()>, ok_message: &str) -> PlayerResponse {
    match result {
        Ok(()) => PlayerResponse::Ok(ok_message.to_string()),
        Err(err) => {
            eprintln!("{ok_message} failed: {err:#}");
            PlayerResponse::Error(format!("{err:#}"))
        }
    }
}

fn write_response(stream: &mut UnixStream, response: &PlayerResponse) -> Result<()> {
    let body = serde_json::to_vec(response).context("failed to serialize response")?;

    stream.write_all(&body)?;
    stream.write_all(b"\n")?;
    stream
        .shutdown(Shutdown::Write)
        .context("failed to shutdown client connection")?;

    Ok(())
}

fn cleanup_stale_socket(path: &str) -> Result<()> {
    let p = Path::new(path);
    if p.exists() {
        std::fs::remove_file(p).with_context(|| format!("failed to remove stale socket {path}"))?;
    }
    Ok(())
}
