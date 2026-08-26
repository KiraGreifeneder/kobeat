use crate::domain::track::Track;

pub enum PlaybackState {
    Stopped,
    Playing,
    Paused,
}

pub struct SessionState {
    pub current_track: Option<Track>,
    pub playback_state: PlaybackState,
    pub base_position_secs: f64,
}
