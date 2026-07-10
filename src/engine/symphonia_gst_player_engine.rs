use std::{
    io::Write,
    process::{Command, Stdio},
};

use anyhow::{Context, Result};

use crate::{decoder::symphonia_decoder::SymphoniaDecoder, engine::player_engine::PlayerEngine};

pub struct SymphoniaGstPlayerEngine {}

impl PlayerEngine for SymphoniaGstPlayerEngine {
    fn play(&self, path: String) -> Result<()> {
        println!("Playing: {}", path);

        let mut decoder =
            SymphoniaDecoder::open(&path).context("Couldn't create Symphonia decoder.")?;
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

        println!("Opening Stdin");

        let mut stdin = child.stdin.take().expect("Failed to open stdin");

        println!("Loop time");

        while !decoder.done {
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
