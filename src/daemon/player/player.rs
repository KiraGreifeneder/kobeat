use crate::domain::state::PlayerState;
use std::sync::{Arc, Mutex};

use crate::domain::state::PlaybackState;

pub trait Player: Send {
    fn new(on_finished_callback: Option<Box<dyn FnMut() + Send>>) -> Self
    where
        Self: Sized;
    fn get_state(&self) -> PlayerState;
    fn play_from_start(&mut self, path: String) -> Result<(), anyhow::Error>;
    fn pause(&self);

    /// A cheap, cloneable handle to this player's live playback state.
    ///
    /// Callers (e.g. the daemon's command loop) can hold onto this and flip
    /// it directly, without needing to lock whatever bigger mutex might be
    /// guarding the player instance itself — which matters because
    /// `play_from_start` blocks for the whole track and may hold such a lock
    /// the entire time.
    fn playback_state(&self) -> Arc<Mutex<PlaybackState>>;
}
