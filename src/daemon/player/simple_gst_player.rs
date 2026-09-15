use crate::daemon::player::Player;
use std::process::Command;

pub struct SimpleGstPlayer {}

impl Player for SimpleGstPlayer {
    fn get_state(&self) -> crate::domain::state::PlayerState {
        crate::domain::state::PlayerState {
            current_track: None,
            playback_state: crate::domain::state::PlaybackState::Stopped,
            base_position_secs: 0.0,
        }
    }

    fn play_from_start(&mut self, path: String) -> Result<(), anyhow::Error> {
        println!("Playing: {}", path);

        Command::new("gst-play-1.0")
            .arg("--audiosink=mtkbtmwrpcaudiosink")
            .arg(path)
            .status()
            .expect("Failed to execute command");

        Ok(())
    }

    fn pause(&self) {
        println!("Pretend pausing playback");
    }
}
