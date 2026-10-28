use rand::seq::SliceRandom;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Repeat {
    #[default]
    Off,
    All,
    One,
}
impl Repeat {
    pub fn cycle(self) -> Self {
        match self {
            Self::Off => Self::All,
            Self::All => Self::One,
            Self::One => Self::Off,
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            Self::Off => "Повтор выключен",
            Self::All => "Повтор папки",
            Self::One => "Повтор трека",
        }
    }
}

#[derive(Clone, Default)]
pub struct Queue {
    pub order: Vec<usize>,
    pub cursor: usize,
}
impl Queue {
    pub fn start(&mut self, len: usize, selected: usize, shuffle: bool) {
        self.order = (0..len).collect();
        if len == 0 {
            self.cursor = 0;
            return;
        }
        let selected = selected.min(len - 1);
        if shuffle {
            self.order.retain(|&i| i != selected);
            self.order.shuffle(&mut rand::rng());
            self.order.insert(0, selected);
            self.cursor = 0;
        } else {
            self.cursor = selected;
        }
    }
    pub fn current(&self) -> Option<usize> {
        self.order.get(self.cursor).copied()
    }
    pub fn advance(&mut self, repeat: Repeat, manual: bool) -> Option<usize> {
        if self.order.is_empty() {
            return None;
        }
        if repeat == Repeat::One && !manual {
            return self.current();
        }
        if self.cursor + 1 < self.order.len() {
            self.cursor += 1;
        } else if repeat == Repeat::All || manual {
            self.cursor = 0;
        } else {
            return None;
        }
        self.current()
    }
    pub fn previous(&mut self) -> Option<usize> {
        if self.order.is_empty() {
            return None;
        }
        self.cursor = if self.cursor == 0 {
            self.order.len() - 1
        } else {
            self.cursor - 1
        };
        self.current()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn repeat_and_end_of_queue() {
        let mut q = Queue::default();
        q.start(3, 1, false);
        assert_eq!(q.advance(Repeat::One, false), Some(1));
        assert_eq!(q.advance(Repeat::One, true), Some(2));
        assert_eq!(q.advance(Repeat::Off, false), None);
        assert_eq!(q.advance(Repeat::All, false), Some(0));
        assert_eq!(q.previous(), Some(2));
    }
    #[test]
    fn shuffle_visits_every_track_once_and_keeps_selected() {
        let mut q = Queue::default();
        q.start(100, 37, true);
        assert_eq!(q.current(), Some(37));
        let mut seen = vec![37];
        while let Some(i) = q.advance(Repeat::Off, false) {
            seen.push(i);
        }
        seen.sort();
        assert_eq!(seen, (0..100).collect::<Vec<_>>());
        q.start(0, 0, true);
        assert_eq!(q.advance(Repeat::All, true), None);
    }
}
