#[derive(Default)]
pub struct Queue {
    songs: Vec<String>,
    /// Index of the current song, or `None` before the first one is taken.
    position: Option<usize>,
}

impl Queue {
    pub fn next(&mut self) -> Option<String> {
        let next = match self.position {
            Some(position) => position + 1,
            None => 0,
        };

        let song = self.songs.get(next)?.clone();
        self.position = Some(next);
        Some(song)
    }

    pub fn previous(&mut self) -> Option<String> {
        let previous = self.position?.checked_sub(1)?;

        let song = self.songs.get(previous)?.clone();
        self.position = Some(previous);
        Some(song)
    }

    pub fn set_queue(&mut self, songs: Vec<String>) {
        self.songs = songs;
        self.position = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn queue_of(songs: &[&str]) -> Queue {
        let mut queue = Queue::default();
        queue.set_queue(songs.iter().map(|s| s.to_string()).collect());
        queue
    }

    #[test]
    fn next_starts_at_the_first_song() {
        let mut queue = queue_of(&["a", "b"]);
        assert_eq!(queue.next().as_deref(), Some("a"));
        assert_eq!(queue.next().as_deref(), Some("b"));
        assert_eq!(queue.next(), None);
    }

    #[test]
    fn previous_walks_back_and_stops_at_the_start() {
        let mut queue = queue_of(&["a", "b"]);
        queue.next();
        queue.next();
        assert_eq!(queue.previous().as_deref(), Some("a"));
        assert_eq!(queue.previous(), None);
    }

    #[test]
    fn previous_before_any_next_is_none() {
        let mut queue = queue_of(&["a", "b"]);
        assert_eq!(queue.previous(), None);
    }

    #[test]
    fn setting_a_queue_restarts_from_the_beginning() {
        let mut queue = queue_of(&["a", "b"]);
        queue.next();
        queue.set_queue(vec!["c".to_string()]);
        assert_eq!(queue.next().as_deref(), Some("c"));
    }

    #[test]
    fn empty_queue_yields_nothing() {
        let mut queue = Queue::default();
        assert_eq!(queue.next(), None);
        assert_eq!(queue.previous(), None);
    }
}
