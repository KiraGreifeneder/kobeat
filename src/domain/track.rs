use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Track {
    pub path: String,
    pub duration_secs: f64,
    pub title: Option<String>,
}
