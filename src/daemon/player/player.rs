use anyhow::Result;

use crate::domain::state::{PlaybackOutcome, PlayerState};

/// A playback backend. Implementors own whatever thread/process actually
/// produces sound; every method here returns promptly.
pub trait Player: Send {
    fn new() -> Self
    where
        Self: Sized;

    fn get_state(&self) -> PlayerState;

    /// Starts `path` from the beginning, replacing anything already playing.
    fn load_and_play(&mut self, path: String) -> Result<()>;

    fn pause(&mut self);

    /// Continues the current track from where it was paused.
    fn resume(&mut self) -> Result<()>;

    fn seek(&mut self, position_secs: f64) -> Result<()>;

    /// Returns the outcome of the current track once it ends, exactly once.
    /// `None` while it's still going, or if nothing is playing.
    fn poll_finished(&mut self) -> Option<PlaybackOutcome>;
}
