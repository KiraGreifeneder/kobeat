use std::{
    io::{BufRead, BufReader, Write},
    process::{Command, Stdio},
    sync::{Arc, Mutex},
};

use anyhow::{Context, Result};

use crate::daemon::decoder::symphonia_decoder::SymphoniaDecoder;
use crate::daemon::player::Player;
use crate::domain::state::{PlaybackOutcome, PlaybackState, PlayerState};
use crate::domain::track::Track;

pub struct SymphoniaGstPlayer {
    last_start_time: Option<std::time::Instant>,
    last_paused_pos_secs: Option<f64>,
    playback_state: Arc<Mutex<PlaybackState>>,
    current_path: Option<String>,
    current_duration_secs: Option<f64>,
    current_title: Option<String>,
}

impl Player for SymphoniaGstPlayer {
    fn new() -> Self {
        Self {
            last_start_time: None,
            playback_state: Arc::new(Mutex::new(PlaybackState::Stopped)),
            current_path: None,
            current_duration_secs: None,
            last_paused_pos_secs: None,
            current_title: None,
        }
    }

    fn get_state(&self) -> PlayerState {
        PlayerState {
            current_track: self.get_track(),
            playback_state: self
                .playback_state
                .lock()
                .expect("state mutex poisoned")
                .clone(),
            base_position_secs: 0.0,
        }
    }

    fn play_from_start(&mut self, path: String) -> Result<PlaybackOutcome> {
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
            .stdout(Stdio::piped())
            .spawn()
            .context("Failed to start GStreamer")?;

        let mut stdin = child.stdin.take().expect("Failed to open stdin");
        let stdout = child.stdout.take().expect("Failed to open stdout");

        // Drain gst-launch-1.0's own stdout on a background thread. This
        // keeps its stdout pipe buffer from filling up and blocking the
        // pipeline, and surfaces its status lines (e.g. "Got EOS from
        // element...", "Execution ended...") for visibility/debugging.
        let stdout_thread = std::thread::spawn(move || {
            for line in BufReader::new(stdout).lines() {
                match line {
                    Ok(line) => println!("gst-launch-1.0: {line}"),
                    Err(_) => break,
                }
            }
        });

        // Mark playback as actually happening now that the pipeline is up.
        // The loop re-locks `playback_state` on every iteration (a very
        // short critical section) rather than holding a lock for the whole
        // track, so a `pause()` call from another thread takes effect on the
        // next iteration instead of having to wait for the track to finish.
        *self.playback_state.lock().expect("state mutex poisoned") = PlaybackState::Playing;

        loop {
            if decoder.done {
                break;
            }

            {
                let state = self.playback_state.lock().expect("state mutex poisoned");
                if *state != PlaybackState::Playing {
                    break;
                }
            }

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

        // Whether the loop exited because the decoder ran out of data (a
        // natural end-of-track) rather than because playback was paused or
        // stopped partway through. We use this below to decide whether the
        // eventual clean process exit really means "the track finished
        // playing" as opposed to "we cut it short."
        let decoded_to_completion = decoder.done;

        drop(stdin);
        let status = child
            .wait()
            .context("Failed waiting for GStreamer to exit")?;
        let _ = stdout_thread.join();

        if !status.success() {
            anyhow::bail!("GStreamer exited with status {status}");
        }

        // `gst-launch-1.0` only exits (successfully) once its pipeline
        // reports EOS or an error on the bus. Usually EOS
        // is only posted once the sink has finished rendering the last
        // buffer, i.e. track has finished playing.
        if decoded_to_completion {
            *self.playback_state.lock().expect("state mutex poisoned") = PlaybackState::Stopped;
            Ok(PlaybackOutcome::Finished)
        } else {
            Ok(PlaybackOutcome::Interrupted)
        }
    }

    fn pause(&self) {
        *self.playback_state.lock().expect("state mutex poisoned") = PlaybackState::Paused;
    }

    fn playback_state(&self) -> Arc<Mutex<PlaybackState>> {
        Arc::clone(&self.playback_state)
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
