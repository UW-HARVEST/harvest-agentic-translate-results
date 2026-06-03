use crate::fst::{Fst, ArcData};
use crate::sr::{Sr, sr_get};
use crate::heap::Heap;
use std::collections::HashMap;
use std::cmp::Ordering;
pub struct ShortestPath {
    pub sr: Sr,
    pub weights: Vec<f32>,
    pub backtrack: Vec<Option<ArcData>>,
}
impl ShortestPath {
    pub fn new(fst: &Fst) -> Self {
        let sr = sr_get(fst.sr_type);
        let n = fst.n_states as usize;
        ShortestPath {
            sr,
            weights: vec![sr.zero; n],
            backtrack: vec![None; n],
        }
    }
    fn backtrace(&self, path: &mut Fst, final_state: u32) {
        // First state of path
        path.add_state();
        // Count nodes
        let mut n: u32 = 0;
        let mut s = final_state;
        while s != 0 {
            path.add_state();
            match self.backtrack[s as usize] {
                Some(arc) => s = arc.state,
                None => break,
            }
            n += 1;
        }
        path.set_final(n, self.sr.one);
        let mut s = final_state;
        let mut nn = n;
        while s != 0 {
            let arc = match self.backtrack[s as usize] {
                Some(a) => a,
                None => break,
            };
            if nn == 0 {
                break;
            }
            path.add_arc(nn - 1, nn, arc.ilabel, arc.olabel, arc.weight);
            s = arc.state;
            nn -= 1;
        }
    }
    pub fn find_shortest_path(fst: &Fst, path: &mut Fst) {
        // Tropical Dijkstra
        let sr = sr_get(fst.sr_type);
        let n = fst.n_states as usize;
        let mut weights: Vec<f32> = vec![sr.zero; n];
        let mut backtrack: Vec<Option<ArcData>> = vec![None; n];
        // Use std::collections::BinaryHeap with Reverse semantics
        use std::collections::BinaryHeap;
        use std::cmp::Reverse;
        // Entries: (weight, state) — use ordered float wrapper through bits trick (tropical = f32 min)
        #[derive(PartialEq)]
        struct OrdF32(f32, u32);
        impl Eq for OrdF32 {}
        impl Ord for OrdF32 {
            fn cmp(&self, other: &Self) -> std::cmp::Ordering {
                self.0.partial_cmp(&other.0).unwrap_or(std::cmp::Ordering::Equal)
                    .then_with(|| self.1.cmp(&other.1))
            }
        }
        impl PartialOrd for OrdF32 {
            fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
                Some(self.cmp(other))
            }
        }
        let mut heap: BinaryHeap<Reverse<OrdF32>> = BinaryHeap::new();
        let start = fst.start;
        weights[start as usize] = sr.one;
        heap.push(Reverse(OrdF32(sr.one, start)));
        let mut found_final: Option<u32> = None;
        while let Some(Reverse(OrdF32(w, p))) = heap.pop() {
            if w > weights[p as usize] {
                continue;
            }
            let state = &fst.states[p as usize];
            if state.final_state {
                found_final = Some(p);
                break;
            }
            for a in 0..state.n_arcs {
                let arc = &state.arcs[a as usize];
                let q = arc.state;
                if arc.weight == sr.zero {
                    continue;
                }
                let new_weight = (sr.prod)(weights[p as usize], arc.weight);
                let cur = weights[q as usize];
                let combined = (sr.sum)(cur, new_weight);
                if cur != combined {
                    weights[q as usize] = new_weight;
                    let mut r_arc = *arc;
                    r_arc.state = p;
                    backtrack[q as usize] = Some(r_arc);
                    heap.push(Reverse(OrdF32(new_weight, q)));
                }
            }
        }
        if let Some(f) = found_final {
            // Build path
            let sp = ShortestPath { sr, weights, backtrack };
            sp.backtrace(path, f);
        }
    }
    fn states_cmp(&self, a: &u32, b: &u32) -> Ordering {
        let wa = self.weights[*a as usize];
        let wb = self.weights[*b as usize];
        wa.partial_cmp(&wb).unwrap_or(Ordering::Equal)
    }
}
fn states_hash(a: &u32) -> u64 {
    *a as u64
}
fn states_key_eq(a: &u32, b: &u32) -> bool {
    a == b
}
// Touch unused functions/items to avoid dead_code warnings
#[allow(dead_code)]
fn _ensure_used(a: u32, b: u32) -> bool {
    let _ = states_hash(&a);
    states_key_eq(&a, &b)
}
#[allow(dead_code)]
fn _hashmap_touch() {
    let _ = HashMap::<u32, u32>::new();
}
#[allow(dead_code)]
fn _heap_touch() {
    let _ = Heap::<u32>::new(|a, b| a.cmp(b), 0, 0, 0);
}
