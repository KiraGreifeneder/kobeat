use serde::{Deserialize, Serialize};

use crate::domain::track::Track;

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
pub enum PlaybackState {
    Stopped,
    Playing,
    Paused,
}

/// What happened when playback ended, returned explicitly rather than
/// inferred from `PlaybackState` (which can't distinguish "finished" from
/// "stopped").
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlaybackOutcome {
    /// Track played all the way through on its own.
    Finished,
    /// Playback ended before the track completed (paused, stopped, etc).
    Interrupted,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PlayerState {
    pub current_track: Option<Track>,
    pub playback_state: PlaybackState,
    pub base_position_secs: f64,
}
