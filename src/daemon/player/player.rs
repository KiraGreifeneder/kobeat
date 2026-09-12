use crate::domain::state::PlayerState;

pub trait Player {
    fn get_state(&self) -> PlayerState;
    fn play(&mut self, path: String) -> Result<(), anyhow::Error>;
    fn pause(&self);
}
