use std::{
    fmt::format,
    path::{Path, PathBuf},
};

use uuid::Uuid;

use crate::domain::track::Track;

pub trait Playlist {
    fn id(&self) -> Uuid;
    fn name(&self) -> &str;
    fn tracks(&self) -> &[Track];
}
