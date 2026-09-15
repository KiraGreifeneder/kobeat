use crate::domain::state::PlayerState;

pub trait Player {
    fn new(on_finished_callback: Option<Box<dyn FnMut()>>) -> Self
    where
        Self: Sized;
    fn get_state(&self) -> PlayerState;
    fn play_from_start(&mut self, path: String) -> Result<(), anyhow::Error>;
    fn pause(&self);
}
