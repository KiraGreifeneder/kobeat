use crate::{config::SOCKET_PATH, entity::player_command::PlayerCommand};
use anyhow::{Context, Result};
use std::io::{Read, Write};
use std::os::unix::net::UnixStream;

pub fn send_command(command: &PlayerCommand) -> Result<String> {
    send_command_to_socket(SOCKET_PATH, command)
}

pub fn send_command_to_socket(socket_path: &str, command: &PlayerCommand) -> Result<String> {
    let mut stream = UnixStream::connect(socket_path)
        .with_context(|| format!("failed to connect to socket at {socket_path}"))?;

    write_command(&mut stream, command)?;

    let mut response = String::new();
    stream.read_to_string(&mut response)?;

    Ok(response)
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
            &PlayerCommand::Play {
                path: "song.mp3".into(),
            },
        )?;

        match server.join().expect("server thread panicked")? {
            PlayerCommand::Play { path } => assert_eq!(path, "song.mp3"),
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
