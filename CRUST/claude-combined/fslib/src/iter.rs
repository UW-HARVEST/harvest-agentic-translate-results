use crate::fst::{Fst, State};
use crate::bitset::BitSet;
use crate::queue::Queue;
pub struct FstIter<'a, T> {
    pub fst: &'a Fst,
    pub marked: BitSet,
    pub queue: Queue<T>,
    pub state: State,
}
impl<'a> FstIter<'a, State> {
    pub fn new(fst: &'a Fst) -> Self {
        let mut iter = FstIter {
            fst,
            marked: BitSet::new(fst.n_states.max(1) as usize),
            queue: Queue::new(),
            state: fst.start,
        };
        iter.queue.enqueue(iter.state);
        iter.marked.set(iter.state as usize);
        iter
    }
    pub fn next(&mut self) -> Option<State> {
        match self.queue.dequeue() {
            Some(s) => {
                self.state = s;
                let state = &self.fst.states[s as usize];
                for arc in &state.arcs {
                    if !self.marked.get(arc.state as usize) {
                        self.queue.enqueue(arc.state);
                        self.marked.set(arc.state as usize);
                    }
                }
                Some(s)
            }
            None => None,
        }
    }
    pub fn remove(self) {
        drop(self);
    }
    pub fn visited(&self, state: State) -> bool {
        self.marked.get(state as usize)
    }
}
