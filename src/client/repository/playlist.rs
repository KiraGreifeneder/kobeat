use crate::domain::track::Track;

pub trait Playlist {
    fn id(&self) -> uuid::Uuid;
    fn name(&self) -> &str;
    fn tracks(&self) -> &[Track];
}
