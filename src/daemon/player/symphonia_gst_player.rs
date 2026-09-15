use std::{
    io::Write,
    process::{Command, Stdio},
};

use anyhow::{Context, Result};

use crate::daemon::decoder::symphonia_decoder::SymphoniaDecoder;
use crate::daemon::player::Player;
use crate::domain::state::{PlaybackState, PlayerState};

pub struct SymphoniaGstPlayer {
    last_start_time: Option<std::time::Instant>,
    playback_state: PlaybackState,
    decoder: SymphoniaDecoder,
}

impl Player for SymphoniaGstPlayer {
    fn get_state(&self) -> PlayerState {
        !todo!()
    }

    fn play_from_start(&mut self, path: String) -> Result<()> {
        println!("Playing: {}", path);

        self.decoder =
            SymphoniaDecoder::open(&path).context("Couldn't create Symphonia decoder.")?;
        let channels_arg = format!("num-channels={}", self.decoder.channels());
        let sample_rate_arg = format!("sample-rate={}", self.decoder.sample_rate());

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

        while !self.decoder.done && &self.playback_state == &PlaybackState::Playing {
            let mut out = Vec::new();
            self.decoder
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
