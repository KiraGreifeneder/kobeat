pub enum Command {
    Play { path: String },
    Stop,
    Pause,
    Resume,
    SeekAbs { position_secs: f64 },
    SeekRel { delta_secs: f64 },
    Volume { percent: u8 },
    Status,
    Shutdown,
}
