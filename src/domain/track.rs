use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone)]
pub struct Track {
    pub path: String,
    pub duration_secs: Option<f64>,
    pub title: Option<String>,
}
