use crate::fst::Fst;
use crate::bitset::BitSet;
use crate::queue::Queue;

pub struct FstIter<'a, T> {
    pub fst: &'a Fst,
    pub marked: BitSet,
    pub queue: Queue<T>,
    pub state: u32,
}

impl<'a> FstIter<'a, u32> {
    pub fn new(fst: &'a Fst) -> Self {
        let mut iter = FstIter {
            fst,
            marked: BitSet::new(fst.n_states as usize),
            queue: Queue::new(),
            state: fst.start,
        };
        iter.queue.enqueue(fst.start);
        iter.marked.set(fst.start as usize);
        iter
    }
    pub fn next(&mut self) -> Option<u32> {
        if let Some(s) = self.queue.dequeue() {
            self.state = s;
            let state_data = &self.fst.states[s as usize];
            for arc in &state_data.arcs {
                if !self.marked.get(arc.state as usize) {
                    self.queue.enqueue(arc.state);
                    self.marked.set(arc.state as usize);
                }
            }
            Some(s)
        } else {
            self.state = u32::MAX;
            None
        }
    }
    pub fn remove(self) {
        drop(self);
    }
    pub fn visited(&self, state: u32) -> bool {
        self.marked.get(state as usize)
    }
}
