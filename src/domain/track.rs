use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct Track {
    pub path: String,
    pub sample_rate: u32,
    pub channels: u16,
    pub duration_secs: Option<f64>,
    pub title: Option<String>,
}
