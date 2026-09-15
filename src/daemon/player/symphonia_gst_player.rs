use std::time::Instant;

use anyhow::{Context, Result};

use crate::daemon::player::Player;
use crate::daemon::player::symphonia_gst_play_task::SymphoniaGstPlayTask;
use crate::domain::state::{PlaybackOutcome, PlaybackState, PlayerState};
use crate::domain::track::Track;

pub struct SymphoniaGstPlayer {
    task: Option<SymphoniaGstPlayTask>,
    playback_state: PlaybackState,
    /// Position the current task started from + when it started. live position can be derived from this.
    task_start_pos_secs: f64,
    task_started_at: Option<Instant>,
    current_path: Option<String>,
    current_duration_secs: f64,
    current_title: Option<String>,
}

impl Player for SymphoniaGstPlayer {
    fn new() -> Self {
        Self {
            task: None,
            playback_state: PlaybackState::Stopped,
            task_start_pos_secs: 0.0,
            task_started_at: None,
            current_path: None,
            current_duration_secs: 0.0,
            current_title: None,
        }
    }

    fn get_state(&self) -> PlayerState {
        PlayerState {
            current_track: self.get_track(),
            playback_state: self.playback_state,
            base_position_secs: self.position_secs(),
        }
    }

    fn load_and_play(&mut self, path: String) -> Result<()> {
        self.stop_task();
        self.current_path = Some(path);
        self.start_task(0.0)
    }

    fn pause(&mut self) {
        let Some(task) = &self.task else {
            return;
        };

        task.interrupt();
        self.playback_state = PlaybackState::Paused;
    }

    fn resume(&mut self) -> Result<()> {
        if self.playback_state == PlaybackState::Playing {
            return Ok(());
        }

        let resume_from = self.position_secs();
        self.stop_task();
        self.start_task(resume_from)
    }

    fn seek(&mut self, position_secs: f64) -> Result<()> {
        self.stop_task();
        self.start_task(position_secs)
    }

    fn poll_finished(&mut self) -> Option<PlaybackOutcome> {
        if !self.task.as_ref()?.is_finished() {
            return None;
        }

        match self.take_task()? {
            Ok(outcome) => Some(outcome),
            Err(err) => {
                eprintln!("Playback failed: {err:#}");
                None
            }
        }
    }
}

impl SymphoniaGstPlayer {
    fn get_track(&self) -> Option<Track> {
        Some(Track {
            path: self.current_path.clone()?,
            duration_secs: self.current_duration_secs,
            title: self.current_title.clone(),
        })
    }

    /// Live position
    fn position_secs(&self) -> f64 {
        match (self.playback_state, self.task_started_at) {
            (PlaybackState::Playing, Some(started_at)) => {
                self.task_start_pos_secs + started_at.elapsed().as_secs_f64()
            }
            _ => self.task_start_pos_secs,
        }
    }

    /// Spawns a task for the current track starting at `from_secs`.
    fn start_task(&mut self, from_secs: f64) -> Result<()> {
        let path = self
            .current_path
            .clone()
            .context("no track loaded to play")?;

        let (task, info) = SymphoniaGstPlayTask::spawn(&path, from_secs)?;

        self.current_duration_secs = info.duration_secs;
        self.current_title = info.title;
        self.task = Some(task);
        self.task_start_pos_secs = from_secs;
        self.task_started_at = Some(Instant::now());
        self.playback_state = PlaybackState::Playing;

        Ok(())
    }

    /// Interrupts the running task and waits for it to wind down, so the
    /// audio device is free before another task claims it.
    fn stop_task(&mut self) {
        let Some(task) = &self.task else {
            return;
        };

        task.interrupt();

        if let Some(Err(err)) = self.take_task() {
            eprintln!("Playback failed: {err:#}");
        }
    }

    /// Joins the current task and folds its result back into our own state.
    fn take_task(&mut self) -> Option<Result<PlaybackOutcome>> {
        let task = self.task.take()?;
        self.task_started_at = None;

        match task.join() {
            Ok(result) => {
                self.task_start_pos_secs = result.position_secs;

                if result.outcome == PlaybackOutcome::Finished {
                    self.task_start_pos_secs = 0.0;
                    self.playback_state = PlaybackState::Stopped;
                } else if self.playback_state == PlaybackState::Playing {
                    // Ended without anyone asking it to stop.
                    self.playback_state = PlaybackState::Stopped;
                }

                Some(Ok(result.outcome))
            }
            Err(err) => {
                self.playback_state = PlaybackState::Stopped;
                Some(Err(err))
            }
        }
    }
}
