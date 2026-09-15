use serde::{Deserialize, Serialize};

use crate::domain::track::Track;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum PlaybackState {
    Stopped,
    Playing,
    Paused,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PlayerState {
    pub current_track: Option<Track>,
    pub playback_state: PlaybackState,
    pub base_position_secs: f64,
}
