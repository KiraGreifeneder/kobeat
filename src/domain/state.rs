use serde::{Deserialize, Serialize};

use crate::domain::track::Track;

#[derive(Serialize, Deserialize, Clone)]
pub enum PlaybackState {
    Stopped,
    Playing,
    Paused,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct PlayerState {
    pub current_track: Option<Track>,
    pub playback_state: PlaybackState,
    pub base_position_secs: f64,
}
