use clap::Subcommand;
use serde::{Deserialize, Serialize};

use crate::domain::state::PlayerState;

#[derive(Subcommand, Debug, Serialize, Deserialize)]
pub enum PlayerCommand {
    LoadAndPlay { path: String },
    Pause,
    Resume,
    Seek { position_secs: u64 },
    GetPlayerState,
    SetQueue { paths: Vec<String> },
    PlayNext,
    PlayPrevious,
    Shutdown,
}

#[derive(Serialize, Deserialize, Debug)]
pub enum PlayerResponse {
    Ok(String),         // For commands with no data return
    State(PlayerState), // Status command returns data
    Error(String),      // Any error
}
