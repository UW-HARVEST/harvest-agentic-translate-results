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
        if fst.n_states > 0 {
            queue.enqueue(start);
            marked.set(start as usize);
        }
        FstIter { fst, marked, queue, state: start }
    }

    pub fn next(&mut self) -> Option<State> {
        match self.queue.dequeue() {
            Some(s) => {
                self.state = s;
                let state = &self.fst.states[s as usize];
                for arc in state.arcs.iter() {
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
