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
    pub fn new(fst: &'a Fst) -> Self
    where
        T: From<u32> + Into<u32> + Copy,
    {
        Self::new_with(fst)
    }
    pub fn next(&mut self) -> Option<T>
    where
        T: From<u32> + Into<u32> + Copy,
    {
        self.next_with()
    }
    pub fn remove(self) {
        drop(self);
    }
    pub fn visited(&self, state: T) -> bool
    where
        T: From<u32> + Into<u32> + Copy,
    {
        self.visited_with(state)
    }
}
impl<'a, T> FstIter<'a, T>
where
    T: From<u32> + Into<u32> + Copy,
{
    fn new_with(fst: &'a Fst) -> Self {
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

    fn next_with(&mut self) -> Option<T> {
        if let Some(s) = self.queue.dequeue() {
            let state_id: u32 = s.into();
            self.state = state_id;
            let state = &self.fst.states[state_id as usize];
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

    fn visited_with(&self, state: T) -> bool {
        self.marked.get(state.into() as usize)
    }
}
