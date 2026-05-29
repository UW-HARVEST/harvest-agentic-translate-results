use crate::fst::{Fst, ArcData};
use crate::sr::Sr;
use crate::heap::Heap;
use std::collections::HashMap;
use std::cmp::Ordering;
pub struct ShortestPath {
    sr: Sr,
    weights: Vec<f32>,
    backtrack: Vec<Option<ArcData>>,
}
impl ShortestPath {
    pub fn new(fst: &Fst) -> Self {
        let sr = crate::sr::sr_get(fst.sr_type);
        let n = fst.n_states as usize;
        ShortestPath {
            sr,
            weights: vec![f32::MAX; n],
            backtrack: vec![None; n],
        }
    }
    fn backtrace(&self, path: &mut Fst, final_state: u32) {
        // Walk backtrack from final_state until reaching a state with no backtrack (start)
        let mut chain: Vec<ArcData> = Vec::new();
        let mut s = final_state;
        while let Some(arc) = self.backtrack[s as usize] {
            chain.push(arc);
            // arc.state holds the predecessor state in our backtrack scheme
            s = arc.state;
        }
        let n = chain.len() as u32;
        // Add states 0..=n
        for _ in 0..=n {
            path.add_state();
        }
        path.set_final(n, self.sr.one);
        // chain is in order from final back to start; reverse it for arc construction
        for (i, arc) in chain.iter().rev().enumerate() {
            path.add_arc(i as u32, (i as u32) + 1, arc.ilabel, arc.olabel, arc.weight);
        }
    }
    pub fn find_shortest_path(fst: &Fst, path: &mut Fst) {
        let mut sp = ShortestPath::new(fst);
        let sr = crate::sr::sr_get(fst.sr_type);
        let q_start = fst.start;
        sp.weights[q_start as usize] = sr.one;
        // Use a min-heap with lazy deletion via stale-check.
        use std::collections::BinaryHeap;
        let mut heap: BinaryHeap<HeapNode> = BinaryHeap::new();
        heap.push(HeapNode { w: sr.one, s: q_start });
        let mut final_state: Option<u32> = None;
        while let Some(node) = heap.pop() {
            let p = node.s;
            if node.w != sp.weights[p as usize] {
                continue;
            }
            let state = &fst.states[p as usize];
            if state.final_state {
                final_state = Some(p);
                break;
            }
            for arc in &state.arcs {
                let q = arc.state;
                if arc.weight == sr.zero {
                    continue;
                }
                let new_w = (sr.prod)(sp.weights[p as usize], arc.weight);
                if (sr.sum)(sp.weights[q as usize], new_w) != sp.weights[q as usize] {
                    sp.weights[q as usize] = new_w;
                    let mut r_arc = *arc;
                    r_arc.state = p;
                    sp.backtrack[q as usize] = Some(r_arc);
                    heap.push(HeapNode { w: new_w, s: q });
                }
            }
        }
        if let Some(f) = final_state {
            sp.backtrace(path, f);
        }
    }
    fn states_cmp(&self, a: &u32, b: &u32) -> Ordering {
        self.weights[*a as usize].partial_cmp(&self.weights[*b as usize]).unwrap_or(Ordering::Equal)
    }
}
fn states_hash(a: &u32) -> u64 {
    *a as u64
}
fn states_key_eq(a: &u32, b: &u32) -> bool {
    a == b
}
#[allow(dead_code)]
fn ensure_unused() {
    let _ = states_hash;
    let _ = states_key_eq;
    let _ = HashMap::<u32, u32>::new;
    let _ = Heap::<u32>::new;
}
#[derive(Clone, Copy, PartialEq)]
struct HeapNode { w: f32, s: u32 }
impl Eq for HeapNode {}
impl PartialOrd for HeapNode {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> { Some(self.cmp(other)) }
}
impl Ord for HeapNode {
    fn cmp(&self, other: &Self) -> Ordering {
        // BinaryHeap is max-heap; we want min by weight => reverse
        other.w.partial_cmp(&self.w).unwrap_or(Ordering::Equal)
            .then_with(|| other.s.cmp(&self.s))
    }
}
