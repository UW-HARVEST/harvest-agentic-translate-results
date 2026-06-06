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
        let mut marked = BitSet::new(fst.n_states as usize + 1);
        let mut queue = Queue::new();
        let state = fst.start;
        queue.enqueue(state);
        marked.set(state as usize);
        FstIter {
            fst,
            marked,
            queue,
            state,
        }
    }
    pub fn next(&mut self) -> Option<u32> {
        if let Some(s) = self.queue.dequeue() {
            self.state = s;
            if (s as usize) < self.fst.states.len() {
                let state = &self.fst.states[s as usize];
                for arc in &state.arcs {
                    if !self.marked.get(arc.state as usize) {
                        self.queue.enqueue(arc.state);
                        self.marked.set(arc.state as usize);
                    }
                }
            }
            Some(s)
        } else {
            None
        }
    }
    pub fn remove(self) {
        // drop
    }
    pub fn visited(&self, state: u32) -> bool {
        self.marked.get(state as usize)
    }
}
