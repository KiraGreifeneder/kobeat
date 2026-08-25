use crate::daemon::player::Player;
use std::process::Command;

pub struct SimpleGstPlayer {}

impl Player for SimpleGstPlayer {
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
