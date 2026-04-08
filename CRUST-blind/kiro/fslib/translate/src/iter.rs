use crate::fst::Fst;
use crate::bitset::BitSet;
use crate::queue::Queue;
pub struct FstIter<'a, T> {
    pub fst: &'a Fst,
    pub marked: BitSet,
    pub queue: Queue<T>,
    pub state: u32,
}
impl<'a, T> FstIter<'a, T> {
    pub fn new(fst: &'a Fst) -> Self {
        let mut marked = BitSet::new(fst.n_states as usize);
        let mut queue = Queue::new();
        let state = fst.start;
        // We can't enqueue here generically, so we set state only
        // The actual BFS logic uses u32 specialization
        FstIter { fst, marked, queue, state }
    }
    pub fn next(&mut self) -> Option<T> {
        // Generic stub - real implementation is for T=u32
        None
    }
    pub fn remove(self) {
        drop(self);
    }
    pub fn visited(&self, _state: T) -> bool {
        false
    }
}

// Specialized implementation for u32
impl<'a> FstIter<'a, u32> {
    pub fn create(fst: &'a Fst) -> Self {
        let mut marked = BitSet::new(fst.n_states as usize);
        let mut queue: Queue<u32> = Queue::new();
        let state = fst.start;
        queue.enqueue(state);
        marked.set(state as usize);
        FstIter { fst, marked, queue, state }
    }
    pub fn next_state(&mut self) -> u32 {
        if let Some(s) = self.queue.dequeue() {
            self.state = s;
            let state = &self.fst.states[s as usize];
            for arc in &state.arcs {
                if !self.marked.get(arc.state as usize) {
                    self.queue.enqueue(arc.state);
                    self.marked.set(arc.state as usize);
                }
            }
        } else {
            self.state = u32::MAX; // -1 as u32
        }
        self.state
    }
    pub fn is_visited(&self, state: u32) -> bool {
        self.marked.get(state as usize)
    }
}
