pub const SOCKET_PATH: &str = "/tmp/kobeatctl.sock";

/// GStreamer sink element used for playback. The Kobo's Bluetooth sink is
/// the default; override with `KOBEAT_AUDIO_SINK` to test elsewhere.
pub const DEFAULT_AUDIO_SINK: &str = "mtkbtmwrpcaudiosink";

pub fn audio_sink() -> String {
    std::env::var("KOBEAT_AUDIO_SINK").unwrap_or_else(|_| DEFAULT_AUDIO_SINK.to_string())
}
// pub const MUSIC_DIR: &str = "/mnt/onboard/music";
pub const MUSIC_DIR: &str =
    "/home/agc/Insync/agreencat.kirag@outlook.com/OneDrive/Music PDFs, tabs etc/Spotify/";
