use std::process::Command;

pub trait PlayerEngine {
    fn play(&self, path: String);
    fn pause(&self);
}

pub struct GstPlayerEngine {}

impl PlayerEngine for GstPlayerEngine {
    fn play(&self, path: String) {
        println!("Playing: {}", path);

        Command::new("gst-play-1.0")
            .arg("--audiosink=mtkbtmwrpcaudiosink")
            .arg(path)
            .status()
            .expect("Failed to execute command");
    }

    fn pause(&self) {
        println!("Pretend pausing playback");
    }
}
