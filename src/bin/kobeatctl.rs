use anyhow::{Context, Result};
use clap::Parser;
use kobeat::config::SOCKET_PATH;
use kobeat::model::player_command::PlayerCommand;
use std::io::{Read, Write};
use std::os::unix::net::UnixStream;

#[derive(Parser, Debug)]
struct Args {
    #[command(subcommand)]
    command: PlayerCommand,
}

fn main() -> Result<()> {
    let args = Args::parse();

    let mut stream = UnixStream::connect(SOCKET_PATH)
        .with_context(|| format!("failed to connect to socket at {SOCKET_PATH}"))?;

    send_command(&mut stream, &args.command)?;

    let mut response = String::new();
    stream.read_to_string(&mut response)?;

    println!("{}", response);

    Ok(())
}

fn send_command(stream: &mut UnixStream, command: &PlayerCommand) -> Result<()> {
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
        send_command(
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
}
