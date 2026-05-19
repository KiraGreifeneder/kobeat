use anyhow::Result;
use clap::Parser;
use kobeat::config::SOCKET_PATH;
use kobeat::model::command::Command;
use std::io::Write;
use std::os::unix::net::UnixStream;

#[derive(Parser, Debug)]
struct Args {
    #[command(subcommand)]
    command: Command,
}

fn main() -> Result<(), anyhow::Error> {
    let args = Args::parse();
    println!("Parsed command: {:?}", args.command);

    let mut stream = match UnixStream::connect(SOCKET_PATH) {
        Ok(sock) => sock,
        Err(e) => {
            anyhow::bail!(
                "Failed to connect to kobeatd at {}: {}. Is the daemon (kobeatd) running?",
                SOCKET_PATH,
                e
            )
        }
    };

    let command_json = serde_json::to_string(&args.command)?;
    stream.write_all(command_json.as_bytes())?;

    let mut response = String::new();
    stream.read_to_string(&mut response)?;

    Ok(())
}
