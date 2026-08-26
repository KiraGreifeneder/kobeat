use anyhow::Result;
use clap::Parser;
use kobeat::client::client;
use kobeat::domain::player_command::PlayerCommand;

#[derive(Parser, Debug)]
struct Args {
    #[command(subcommand)]
    command: PlayerCommand,
}

fn main() -> Result<()> {
    let args = Args::parse();
    let response = client::send_command(&args.command)?;
    println!("{}", response);
    Ok(())
}
