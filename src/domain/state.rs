use serde::{Deserialize, Serialize};

use crate::domain::track::Track;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum PlaybackState {
    Stopped,
    Playing,
    Paused,
}

/// What happened when `Player::play_from_start` returned successfully.
///
/// This is deliberately a distinct, explicit value rather than something
/// callers infer from `PlaybackState` afterwards: `PlaybackState::Stopped`
/// is also the idle default and could later mean "the user hit stop", so
/// inferring "the track finished" from it would break the moment a real
/// `Stop` command exists. Returning the outcome directly from the call that
/// actually knows what happened avoids that ambiguity entirely.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlaybackOutcome {
    /// The decoder ran out of data and the sink finished rendering it --
    /// the track played all the way through on its own.
    Finished,
    /// Playback ended before the track completed (paused, stopped, or
    /// otherwise interrupted).
    Interrupted,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PlayerState {
    pub current_track: Option<Track>,
    pub playback_state: PlaybackState,
    pub base_position_secs: f64,
}
