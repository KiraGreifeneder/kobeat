use clap::Subcommand;
use serde::{Deserialize, Serialize};

#[derive(Subcommand, Debug, Serialize, Deserialize)]
pub enum Command {
    Play { path: String },
    Stop,
    Pause,
    Resume,
    Seek { position_secs: u64 },
    Status,
    Shutdown,
}
