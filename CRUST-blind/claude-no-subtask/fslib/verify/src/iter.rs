use crate::fst::{Fst, State};
use crate::bitset::BitSet;
use crate::queue::Queue;

pub struct FstIter<'a, T> {
    pub fst: &'a Fst,
    pub marked: BitSet,
    pub queue: Queue<T>,
    pub state: u32,
}

impl<'a> FstIter<'a, State> {
    pub fn new(fst: &'a Fst) -> Self {
        let mut marked = BitSet::new(fst.n_states.max(1) as usize);
        let mut queue: Queue<State> = Queue::new();
        let start = fst.start;
        queue.enqueue(start);
        marked.set(start as usize);
        FstIter {
            fst,
            marked,
            queue,
            state: start,
        }
    }

    pub fn next(&mut self) -> Option<State> {
        if let Some(s) = self.queue.dequeue() {
            self.state = s;
            let state = &self.fst.states[s as usize];
            for a in 0..state.n_arcs as usize {
                let arc = &state.arcs[a];
                if !self.marked.get(arc.state as usize) {
                    self.queue.enqueue(arc.state);
                    self.marked.set(arc.state as usize);
                }
            }
            Some(self.state)
        } else {
            self.state = u32::MAX;
            None
        }
    }

    pub fn remove(self) {
        drop(self);
    }

    pub fn visited(&self, state: State) -> bool {
        self.marked.get(state as usize)
    }
}
