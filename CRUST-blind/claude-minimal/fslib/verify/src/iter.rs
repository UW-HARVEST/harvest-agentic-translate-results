use crate::fst::{Fst, State};
use crate::bitset::BitSet;
use crate::queue::Queue;

pub struct FstIter<'a> {
    pub fst: &'a Fst,
    pub marked: BitSet,
    pub queue: Queue<State>,
    pub state: i64, // -1 to indicate no current state
}
impl<'a> FstIter<'a> {
    pub fn new(fst: &'a Fst) -> Self {
        let mut marked = BitSet::new(fst.n_states as usize);
        let mut queue = Queue::new();
        let start = fst.start;
        queue.enqueue(start);
        marked.set(start as usize);
        FstIter {
            fst,
            marked,
            queue,
            state: start as i64,
        }
    }
    pub fn next(&mut self) -> Option<State> {
        if let Some(s) = self.queue.dequeue() {
            self.state = s as i64;
            let state = &self.fst.states[s as usize];
            for arc in &state.arcs {
                if !self.marked.get(arc.state as usize) {
                    self.queue.enqueue(arc.state);
                    self.marked.set(arc.state as usize);
                }
            }
            Some(s)
        } else {
            self.state = -1;
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
