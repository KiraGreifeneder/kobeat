use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct PlayerStatus {
    pub state: PlaybackState,
    pub path: Option<String>,
    pub position_secs: f64,
    pub duration_secs: Option<f64>,
    pub backend: String,
}

#[derive(Serialize, Deserialize)]
pub enum PlaybackState {
    Playing,
    Paused,
    Stopped,
}
