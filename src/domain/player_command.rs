use clap::Subcommand;
use serde::{Deserialize, Serialize};

#[derive(Subcommand, Debug, Serialize, Deserialize)]
pub enum PlayerCommand {
    LoadAndPlay { path: String },
    Pause,
    Resume,
    Seek { position_secs: u64 },
    Status,
    Shutdown,
}
