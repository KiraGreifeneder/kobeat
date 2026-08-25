use anyhow::Result;
use clap::Parser;
use kobeat::control;
use kobeat::entity::player_command::PlayerCommand;

#[derive(Parser, Debug)]
struct Args {
    #[command(subcommand)]
    command: PlayerCommand,
}

fn main() -> Result<()> {
    let args = Args::parse();
    let response = control::send_command(&args.command)?;
    println!("{}", response);
    Ok(())
}
