use rand::seq::SliceRandom;

#[derive(Clone)]
pub struct Queue<T> {
    elements: Vec<T>,
    ordering: Vec<usize>,
    index: usize,
    is_shuffled: bool,
}

#[derive(Eq, PartialEq, PartialOrd, Ord)]
pub enum GoNextMode {
    Default,
    RepeatAll,
}

impl<T> Queue<T> {
    pub fn from_iter<I: IntoIterator<Item = T>>(elements: I, start_index: usize) -> Option<Self> {
        let elements = Vec::from_iter(elements.into_iter());
        if elements.len() <= start_index {
            None
        } else {
            Some(Queue {
                ordering: (0..elements.len()).collect(),
                elements,
                index: start_index,
                is_shuffled: false,
            })
        }
    }

    fn from_iter_shuffled_unchecked(elements: Vec<T>, start_index: usize) -> Self {
        let n = elements.len();
        let mut ordering = Vec::with_capacity(n);
        ordering.push(start_index);
        for i in 0..start_index {
            ordering.push(i);
        }
        for i in (start_index + 1)..n {
            ordering.push(i);
        }
        assert_eq!(ordering.len(), n);
        let mut rng = rand::thread_rng();
        ordering[1..].shuffle(&mut rng);
        Queue {
            ordering,
            elements,
            index: 0,
            is_shuffled: true,
        }
    }

    pub fn from_iter_shuffled<I: IntoIterator<Item = T>>(
        elements: I,
        start_index: usize,
    ) -> Option<Self> {
        let elements = Vec::from_iter(elements.into_iter());
        if elements.len() <= start_index {
            None
        } else {
            Some(Queue::from_iter_shuffled_unchecked(elements, start_index))
        }
    }

    pub fn to_shuffled(self) -> Self {
        if self.is_shuffled {
            self
        } else {
            Queue::from_iter_shuffled_unchecked(self.elements, self.index)
        }
    }

    pub fn to_unshuffled(self) -> Self {
        if self.is_shuffled {
            Queue {
                ordering: (0..self.elements.len()).collect(),
                elements: self.elements,
                index: self.ordering[self.index],
                is_shuffled: false,
            }
        } else {
            self
        }
    }

    pub fn has_previous(&self) -> bool {
        self.index > 0
    }

    pub fn current(&self) -> &T {
        &self.elements[self.ordering[self.index]]
    }

    pub fn go_next(&mut self, mode: GoNextMode) -> Option<&T> {
        if mode == GoNextMode::RepeatAll {
            self.index = (self.index + 1) % self.elements.len();
            Some(self.current())
        } else if self.index + 1 < self.elements.len() {
            self.index += 1;
            Some(self.current())
        } else {
            None
        }
    }

    pub fn go_previous_clamped(&mut self) -> &T {
        if self.index > 0 {
            self.index -= 1;
        }
        self.current()
    }
}
