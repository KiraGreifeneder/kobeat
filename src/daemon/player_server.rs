use tokio::task::JoinHandle;

use crate::{engine::symphonia_gst_player_engine::SymphoniaGstPlayerEngine, model::status::{PlaybackState, PlayerStatus}};

struct PlayerServer {
    engine: SymphoniaGstPlayerEngine,
    playback_thread: Option<JoinHandle<Result<()>>>,
    status: PlayerStatus,
}

impl PlayerServer {
    fn handle_command(&mut self, cmd: PlayerCommand) -> Result<&'static str> {

    }
    fn start_playback(&mut self, path: String) -> Result<()> { ... }
    fn reap_finished_playback(&mut self) { ... }
}
