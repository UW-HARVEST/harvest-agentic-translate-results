use crate::fst::{Fst, ArcData};
use crate::sr::{self, Sr};
use std::cmp::Ordering;

#[derive(Clone)]
struct HeapEntry {
    state: usize,
    weight: f32,
    seq: u64,
}
impl PartialEq for HeapEntry {
    fn eq(&self, o: &Self) -> bool { self.state == o.state && self.weight == o.weight && self.seq == o.seq }
}
impl Eq for HeapEntry {}
impl Ord for HeapEntry {
    fn cmp(&self, o: &Self) -> Ordering {
        // BinaryHeap is a max-heap; we want min by weight
        if self.weight < o.weight { Ordering::Greater }
        else if self.weight > o.weight { Ordering::Less }
        else { o.seq.cmp(&self.seq) }
    }
}
impl PartialOrd for HeapEntry {
    fn partial_cmp(&self, o: &Self) -> Option<Ordering> { Some(self.cmp(o)) }
}

pub struct ShortestPath {
    sr: Sr,
    weights: Vec<f32>,
    backtrack: Vec<Option<ArcData>>,
}
impl ShortestPath {
    pub fn new(fst: &Fst) -> Self {
        let sr_v = sr::sr_get(fst.sr_type);
        let n = fst.n_states as usize;
        ShortestPath {
            sr: sr_v,
            weights: vec![sr_v.zero; n],
            backtrack: vec![None; n],
        }
    }
    fn backtrace(&self, path: &mut Fst, final_state: u32) {
        path.add_state();
        let mut n: u32 = 0;
        let mut s = final_state as usize;
        while s != 0 {
            path.add_state();
            if let Some(arc) = &self.backtrack[s] {
                s = arc.state as usize;
            } else {
                break;
            }
            n += 1;
        }
        path.set_final(n, self.sr.one);
        let mut s = final_state as usize;
        let mut idx = n;
        while s != 0 {
            if let Some(arc) = self.backtrack[s].clone() {
                if idx == 0 { break; }
                let prev_idx = idx - 1;
                path.add_arc(prev_idx, idx, arc.ilabel, arc.olabel, arc.weight);
                s = arc.state as usize;
                idx = prev_idx;
            } else {
                break;
            }
        }
    }
    pub fn find_shortest_path(fst: &Fst, path: &mut Fst) {
        let mut sp = ShortestPath::new(fst);
        let n = fst.n_states as usize;
        if n == 0 {
            return;
        }
        let sr_v = sp.sr;
        let mut q = fst.start as usize;
        sp.weights[q] = sr_v.one;
        use std::collections::BinaryHeap;
        let mut heap: BinaryHeap<HeapEntry> = BinaryHeap::new();
        let mut seq: u64 = 0;
        heap.push(HeapEntry { state: q, weight: sp.weights[q], seq });
        seq += 1;
        while let Some(top) = heap.pop() {
            let p = top.state;
            if top.weight != sp.weights[p] {
                continue;
            }
            let state = &fst.states[p];
            if state.final_state {
                sp.backtrace(path, p as u32);
                return;
            }
            for a in 0..state.n_arcs as usize {
                let arc = &state.arcs[a];
                q = arc.state as usize;
                if arc.weight == sr_v.zero {
                    continue;
                }
                let new_w = (sr_v.prod)(sp.weights[p], arc.weight);
                let cur_w = sp.weights[q];
                let combined = (sr_v.sum)(cur_w, new_w);
                if cur_w != combined {
                    sp.weights[q] = new_w;
                    let mut r_arc = arc.clone();
                    r_arc.state = p as u32;
                    sp.backtrack[q] = Some(r_arc);
                    heap.push(HeapEntry { state: q, weight: new_w, seq });
                    seq += 1;
                }
            }
        }
    }
    fn states_cmp(&self, a: &u32, b: &u32) -> Ordering {
        let wa = self.weights[*a as usize];
        let wb = self.weights[*b as usize];
        if wa < wb {
            Ordering::Less
        } else if wa > wb {
            Ordering::Greater
        } else {
            Ordering::Equal
        }
    }
}
fn states_hash(a: &u32) -> u64 {
    *a as u64
}
fn states_key_eq(a: &u32, b: &u32) -> bool {
    *a == *b
}
