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
        let mut bs = BitSet::new(fst.n_states as usize);
        let mut q = Queue::new();
        let s = fst.start;
        if (fst.n_states as usize) > 0 {
            bs.set(s as usize);
        }
        q.enqueue(s);
        FstIter {
            fst,
            marked: bs,
            queue: q,
            state: s,
        }
    }
    pub fn next(&mut self) -> Option<u32> {
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
