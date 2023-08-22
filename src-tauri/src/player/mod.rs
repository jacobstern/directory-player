use std::collections::VecDeque;

pub struct Player {
    queue: VecDeque<String>,
}

impl Player {
    pub fn new() -> Player {
        Player {
            queue: VecDeque::new(),
        }
    }

    pub fn start_playback(&mut self, file_paths: &[String]) {
        self.queue.clear();
        for path in file_paths {
            self.queue.push_back(path.to_owned());
        }
    }
}
