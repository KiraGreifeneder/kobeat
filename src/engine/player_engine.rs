pub trait PlayerEngine {
    fn play(&self, path: String) -> Result<(), anyhow::Error>;
    fn pause(&self);
}
