use std::{
    io::{BufRead, BufReader, Write},
    process::{Command, Stdio},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    thread::JoinHandle,
    time::Instant,
};

use anyhow::{Context, Result};

use crate::daemon::decoder::symphonia_decoder::SymphoniaDecoder;
use crate::domain::state::PlaybackOutcome;

/// Track metadata, read when the task opens the file.
pub struct TrackInfo {
    pub duration_secs: f64,
    pub title: Option<String>,
}

/// What a finished task reports back to its player.
pub struct TaskResult {
    pub outcome: PlaybackOutcome,
    /// Position in the track where playback ended.
    pub position_secs: f64,
}

/// One running track: owns the thread streaming decoded PCM into GStreamer,
/// plus the flag used to interrupt it.
pub struct SymphoniaGstPlayTask {
    thread: JoinHandle<Result<TaskResult>>,
    interrupted: Arc<AtomicBool>,
}

impl SymphoniaGstPlayTask {
    /// Opens `path`, seeks to `start_pos_secs`, and starts streaming it in
    /// the background. Returns once playback is under way.
    pub fn spawn(path: &str, start_pos_secs: f64) -> Result<(Self, TrackInfo)> {
        let mut decoder =
            SymphoniaDecoder::open(path).context("Couldn't create Symphonia decoder.")?;

        if start_pos_secs > 0.0 {
            decoder
                .seek(start_pos_secs)
                .context("Couldn't seek to start position")?;
        }

        let info = TrackInfo {
            duration_secs: decoder.duration_secs().unwrap_or(0.0),
            title: decoder.title(),
        };

        let interrupted = Arc::new(AtomicBool::new(false));
        let thread = std::thread::spawn({
            let interrupted = Arc::clone(&interrupted);
            move || stream(decoder, start_pos_secs, interrupted)
        });

        Ok((
            Self {
                thread,
                interrupted,
            },
            info,
        ))
    }

    /// Asks the task to stop. Returns immediately; the task winds down once
    /// it notices, after which `is_finished` reports true.
    pub fn interrupt(&self) {
        self.interrupted.store(true, Ordering::Relaxed);
    }

    pub fn is_finished(&self) -> bool {
        self.thread.is_finished()
    }

    pub fn join(self) -> Result<TaskResult> {
        self.thread
            .join()
            .map_err(|_| anyhow::anyhow!("playback thread panicked"))?
    }
}

fn stream(
    mut decoder: SymphoniaDecoder,
    start_pos_secs: f64,
    interrupted: Arc<AtomicBool>,
) -> Result<TaskResult> {
    let channels_arg = format!("num-channels={}", decoder.channels());
    let sample_rate_arg = format!("sample-rate={}", decoder.sample_rate());

    let sink = crate::config::audio_sink();

    let mut args = vec![
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
    ];
    // The sink may carry properties, e.g. "alsasink device=hw:0".
    args.extend(sink.split_whitespace());

    let mut child = Command::new("gst-launch-1.0")
        .args(&args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .context("Failed to start GStreamer")?;

    let mut stdin = child.stdin.take().expect("Failed to open stdin");
    let stdout = child.stdout.take().expect("Failed to open stdout");

    // Drain gst-launch-1.0's stdout so its pipe doesn't fill up and block
    // the pipeline; also surfaces its status lines for debugging.
    let stdout_thread = std::thread::spawn(move || {
        for line in BufReader::new(stdout).lines() {
            match line {
                Ok(line) => println!("gst-launch-1.0: {line}"),
                Err(_) => break,
            }
        }
    });

    let started_at = Instant::now();
    let mut out = Vec::new();

    while !decoder.done && !interrupted.load(Ordering::Relaxed) {
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

    let decoded_to_completion = decoder.done;
    let position_secs = start_pos_secs + started_at.elapsed().as_secs_f64();

    drop(stdin);
    let status = child
        .wait()
        .context("Failed waiting for GStreamer to exit")?;
    let _ = stdout_thread.join();

    if !status.success() {
        anyhow::bail!("GStreamer exited with status {status}");
    }

    // gst-launch-1.0 only exits once EOS/error hits the bus, which for a
    // synced sink means it finished rendering, i.e. finished playing.
    if decoded_to_completion {
        Ok(TaskResult {
            outcome: PlaybackOutcome::Finished,
            position_secs,
        })
    } else {
        Ok(TaskResult {
            outcome: PlaybackOutcome::Interrupted,
            position_secs,
        })
    }
}
