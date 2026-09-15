use crate::{config::SOCKET_PATH, domain::player_command::PlayerCommand};
use anyhow::{Context, Result, anyhow};
use std::io::{Read, Write};
use std::os::unix::net::UnixStream;
use std::sync::mpsc;
use std::time::{Duration, Instant};

/// How long a client is willing to wait for the daemon to connect to and
/// respond on the control socket before giving up. Without this, a stuck
/// daemon (e.g. blocked in playback backend init) would hang the caller
/// forever with no indication of what went wrong.
const SOCKET_TIMEOUT: Duration = Duration::from_secs(5);

pub fn send_command(command: &PlayerCommand) -> Result<String> {
    send_command_to_socket(SOCKET_PATH, command)
}

pub fn send_command_to_socket(socket_path: &str, command: &PlayerCommand) -> Result<String> {
    let start = Instant::now();
    tracing::info!(socket_path, ?command, "connecting to daemon");

    let mut stream = match connect_with_timeout(socket_path, SOCKET_TIMEOUT) {
        Ok(stream) => stream,
        Err(err) => {
            tracing::error!(socket_path, error = %err, "failed to connect to daemon socket");
            return Err(err);
        }
    };

    if let Err(err) = stream.set_read_timeout(Some(SOCKET_TIMEOUT)) {
        tracing::warn!(error = %err, "failed to set read timeout on daemon socket");
    }
    if let Err(err) = stream.set_write_timeout(Some(SOCKET_TIMEOUT)) {
        tracing::warn!(error = %err, "failed to set write timeout on daemon socket");
    }

    if let Err(err) = write_command(&mut stream, command) {
        tracing::error!(error = %err, "failed to write command to daemon socket");
        return Err(err);
    }

    let mut response = String::new();
    match stream.read_to_string(&mut response) {
        Ok(_) => {
            tracing::info!(elapsed = ?start.elapsed(), response, "received daemon response");
        }
        Err(err) => {
            tracing::error!(
                elapsed = ?start.elapsed(),
                error = %err,
                "timed out or failed reading daemon response (daemon may be stuck)"
            );
            return Err(err).context("failed to read response from daemon");
        }
    }

    Ok(response)
}

fn connect_with_timeout(socket_path: &str, timeout: Duration) -> Result<UnixStream> {
    let (tx, rx) = mpsc::channel();
    let path = socket_path.to_string();

    std::thread::spawn(move || {
        let result = UnixStream::connect(&path);
        // Ignore send errors: means the receiver already timed out and gave up.
        let _ = tx.send(result);
    });

    match rx.recv_timeout(timeout) {
        Ok(Ok(stream)) => Ok(stream),
        Ok(Err(err)) => {
            Err(err).with_context(|| format!("failed to connect to socket at {socket_path}"))
        }
        Err(mpsc::RecvTimeoutError::Timeout) => Err(anyhow!(
            "timed out after {timeout:?} connecting to daemon socket at {socket_path} (daemon may be stuck)"
        )),
        Err(mpsc::RecvTimeoutError::Disconnected) => Err(anyhow!(
            "connect thread for {socket_path} vanished unexpectedly"
        )),
    }
}

fn write_command(stream: &mut UnixStream, command: &PlayerCommand) -> Result<()> {
    let command_json = serde_json::to_vec(command)?;
    stream.write_all(&command_json)?;
    stream
        .shutdown(std::net::Shutdown::Write)
        .context("failed to close command stream")?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::Result;
    use std::fs;
    use std::os::unix::net::UnixListener;
    use std::thread;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn send_command_signals_end_of_request() -> Result<()> {
        let socket_path = std::env::temp_dir().join(format!(
            "kobeatctl-test-{}-{}.sock",
            std::process::id(),
            SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos()
        ));

        let listener = UnixListener::bind(&socket_path)?;

        let server = thread::spawn({
            let socket_path = socket_path.clone();
            move || -> Result<PlayerCommand> {
                let (mut stream, _) = listener.accept()?;
                let command = serde_json::from_reader(&mut stream)?;
                drop(stream);
                fs::remove_file(&socket_path)?;
                Ok(command)
            }
        });

        let mut client = UnixStream::connect(&socket_path)?;
        write_command(
            &mut client,
            &PlayerCommand::LoadAndPlay {
                path: "song.mp3".into(),
            },
        )?;

        match server.join().expect("server thread panicked")? {
            PlayerCommand::LoadAndPlay { path } => assert_eq!(path, "song.mp3"),
            other => panic!("unexpected command: {other:?}"),
        }

        Ok(())
    }

    #[test]
    fn send_command_returns_response_body() -> Result<()> {
        let socket_path = std::env::temp_dir().join(format!(
            "kobeatctl-response-test-{}-{}.sock",
            std::process::id(),
            SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos()
        ));

        let listener = UnixListener::bind(&socket_path)?;

        let server = thread::spawn({
            let socket_path = socket_path.clone();
            move || -> Result<()> {
                let (mut stream, _) = listener.accept()?;
                let _: PlayerCommand = serde_json::from_reader(&mut stream)?;
                stream.write_all(b"{\"ok\":true}\n")?;
                drop(stream);
                fs::remove_file(&socket_path)?;
                Ok(())
            }
        });

        let response = send_command_to_socket(
            socket_path
                .to_str()
                .expect("temp socket path is valid UTF-8"),
            &PlayerCommand::Pause,
        )?;

        server.join().expect("server thread panicked")?;
        assert_eq!(response, "{\"ok\":true}\n");

        Ok(())
    }
}
