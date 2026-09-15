pub struct Queue {
    songs: Vec<String>,
    position: usize,
}

impl Default for Queue {
    fn default() -> Self {
        Queue {
            songs: Vec::new(),
            position: 0,
        }
    }
}

impl Queue {
    pub fn next(&mut self) -> Option<String> {
        if self.position + 1 < self.songs.len() {
            self.position += 1;
            Some(self.songs[self.position].clone())
        } else {
            None
        }
    }

    pub fn previous(&mut self) -> Option<String> {
        if self.position > 0 {
            self.position -= 1;
            Some(self.songs[self.position].clone())
        } else {
            None
        }
    }

    pub fn set_queue(&mut self, songs: Vec<String>) {
        self.songs = songs;
        self.position = 0;
    }
}
