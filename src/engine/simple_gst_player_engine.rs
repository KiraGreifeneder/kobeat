use crate::engine::player_engine::PlayerEngine;
use std::process::Command;

pub struct SimpleGstPlayerEngine {}

impl PlayerEngine for SimpleGstPlayerEngine {
    fn play(&self, path: String) -> Result<(), anyhow::Error> {
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
