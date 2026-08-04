use crate::bitset::BitSet;
use crate::fst::Fst;
use crate::queue::Queue;

pub struct FstIter<'a, T> {
    pub fst: &'a Fst,
    pub marked: BitSet,
    pub queue: Queue<T>,
    pub state: u32,
}

impl<'a, T> FstIter<'a, T>
where
    T: Copy + From<u32> + Into<u32>,
{
    pub fn new(fst: &'a Fst) -> Self {
        let mut queue = Queue::new();
        queue.enqueue(T::from(fst.start));
        let mut marked = BitSet::new(fst.n_states as usize);
        marked.set(fst.start as usize);
        Self {
            fst,
            marked,
            queue,
            state: fst.start,
        }
    }

    pub fn next(&mut self) -> Option<T> {
        if let Some(s) = self.queue.dequeue() {
            let sid: u32 = s.into();
            self.state = sid;
            let state = &self.fst.states[sid as usize];
            for arc in &state.arcs {
                if !self.marked.get(arc.state as usize) {
                    self.queue.enqueue(T::from(arc.state));
                    self.marked.set(arc.state as usize);
                }
            }
            Some(s)
        } else {
            self.state = u32::MAX;
            None
        }
    }

    pub fn remove(self) {}

    pub fn visited(&self, state: T) -> bool {
        self.marked.get(state.into() as usize)
    }
}
