use anyhow::Result;
use tokio::task::JoinHandle;

use crate::{
    daemon::player::symphonia_gst_player::SymphoniaGstPlayer,
    entity::{player_command::PlayerCommand, status::PlayerStatus},
};

// Not wired up yet: kobeatd.rs still runs its own inline command loop.
// This struct is the intended home for that logic once it moves into the library.
struct PlayerServer {
    player: SymphoniaGstPlayer,
    playback_thread: Option<JoinHandle<Result<()>>>,
    status: PlayerStatus,
}

impl PlayerServer {
    fn handle_command(&mut self, cmd: PlayerCommand) -> Result<&'static str> {
        todo!("dispatch {cmd:?} once PlayerServer replaces the inline loop in kobeatd")
    }

    fn start_playback(&mut self, path: String) -> Result<()> {
        todo!("start playback for {path}")
    }

    fn reap_finished_playback(&mut self) {
        todo!("reap finished playback thread")
    }
}
