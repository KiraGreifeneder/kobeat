use std::{
    io::Write,
    process::{Command, Stdio},
};

use anyhow::{Context, Result};

use crate::daemon::decoder::symphonia_decoder::SymphoniaDecoder;
use crate::daemon::player::Player;
use crate::domain::state::{PlaybackState, PlayerState};
use crate::domain::track::Track;

pub struct SymphoniaGstPlayer {
    last_start_time: Option<std::time::Instant>,
    playback_state: PlaybackState,
    playback_pos_secs: f64,
    current_path: Option<String>,
    current_duration_secs: Option<f64>,
    current_title: Option<String>,
    on_finished_callback: Option<Box<dyn FnMut()>>,
}

impl Player for SymphoniaGstPlayer {
    fn new(on_finished_callback: Option<Box<dyn FnMut()>>) -> Self {
        Self {
            last_start_time: None,
            playback_state: PlaybackState::Stopped,
            playback_pos_secs: 0.0,
            current_path: None,
            current_duration_secs: None,
            current_title: None,
            on_finished_callback,
        }
    }

    fn get_state(&self) -> PlayerState {
        PlayerState {
            current_track: self.get_track(),
            playback_state: self.playback_state.clone(),
            base_position_secs: 0.0,
        }
    }

    fn play_from_start(&mut self, path: String) -> Result<()> {
        println!("Playing: {}", path);

        // The decoder only needs to live for the duration of this call; the
        // track metadata we care about afterwards is copied into our own
        // fields below so `get_track` doesn't need to keep a decoder around.
        let mut decoder =
            SymphoniaDecoder::open(&path).context("Couldn't create Symphonia decoder.")?;

        self.current_path = Some(path);
        self.current_duration_secs = decoder.duration_secs();
        self.current_title = decoder.title();

        let channels_arg = format!("num-channels={}", decoder.channels());
        let sample_rate_arg = format!("sample-rate={}", decoder.sample_rate());

        let mut child = Command::new("gst-launch-1.0")
            .args(&[
                "fdsrc",
                "fd=0",
                "!",
                "rawaudioparse",
                channels_arg.as_str(),
                sample_rate_arg.as_str(),
                "pcm-format=s16le",
                "!",
                "audioconvert",
                "!",
                "audioresample",
                "!",
                "mtkbtmwrpcaudiosink",
            ])
            .stdin(Stdio::piped())
            .spawn()
            .context("Failed to start GStreamer")?;

        let mut stdin = child.stdin.take().expect("Failed to open stdin");

        while !decoder.done && self.playback_state == PlaybackState::Playing {
            let mut out = Vec::new();
            decoder
                .read_pcm_s16le(&mut out)
                .context("Couldn't read decoded audio")?;

            if out.is_empty() {
                continue;
            }

            stdin
                .write_all(&out)
                .context("Broken pipe while writing PCM data to GStreamer")?;
        }

        drop(stdin);
        let status = child
            .wait()
            .context("Failed waiting for GStreamer to exit")?;
        if !status.success() {
            anyhow::bail!("GStreamer exited with status {status}");
        }

        Ok(())
    }

    fn pause(&self) {
        println!("Pretend pausing playback");
    }
}

impl SymphoniaGstPlayer {
    fn get_track(&self) -> Option<Track> {
        let path = self.current_path.clone()?;

        Some(Track {
            path,
            duration_secs: self.current_duration_secs,
            title: self.current_title.clone(),
        })
    }
}
