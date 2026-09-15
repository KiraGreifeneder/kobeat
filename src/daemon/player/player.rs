use crate::domain::state::PlayerState;

pub trait Player {
    fn get_state(&self) -> PlayerState;
    fn play_from_start(&mut self, path: String) -> Result<(), anyhow::Error>;
    fn pause(&self);
}
