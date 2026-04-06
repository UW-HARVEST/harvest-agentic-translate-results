use crate::fst::{Fst, ArcData};
use crate::sr::{Sr, sr_get};
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
        let sr = sr_get(fst.sr_type);
        let n = fst.n_states as usize;
        let weights = vec![sr.zero; n];
        let backtrack = vec![None; n];
        ShortestPath { sr, weights, backtrack }
    }
    fn backtrace(&self, path: &mut Fst, final_state: u32) {
        let mut n: u32 = 0;
        path.add_state();
        let mut s = final_state;
        while s != 0 {
            path.add_state();
            if let Some(ref arc) = self.backtrack[s as usize] {
                s = arc.state; // state field stores the predecessor
            } else {
                break;
            }
            n += 1;
        }
        path.set_final(n, self.sr.one);
        s = final_state;
        while s != 0 {
            if let Some(ref arc) = self.backtrack[s as usize] {
                path.add_arc(n - 1, n, arc.ilabel, arc.olabel, arc.weight);
                s = arc.state;
                n -= 1;
            } else {
                break;
            }
        }
    }
    pub fn find_shortest_path(fst: &Fst, path: &mut Fst) {
        assert!(fst.sr_type == 0); // SR_TROPICAL
        let mut sp = ShortestPath::new(fst);
        let sr = &sp.sr;

        // Custom comparator using weights
        // We need to use a wrapper to pass weights into the comparator
        // Since HeapCmp is fn(&T, &T) -> Ordering, we'll use the state index
        // and compare via a global-like approach. But we can't use closures.
        // Instead, let's just use a simple approach with a BinaryHeap-like structure.

        // Actually, the Heap type uses fn pointers. We need to compare by weight.
        // The C code uses global W array. We'll use a different approach:
        // store (weight, state) pairs in the heap so comparison works naturally.

        // But the Heap<T> requires T: Ord. Let's use a wrapper.
        #[derive(Clone, Eq, PartialEq, Hash)]
        struct WState {
            state: u32,
            weight_bits: u32, // f32 bits for ordering
        }
        impl Ord for WState {
            fn cmp(&self, other: &Self) -> Ordering {
                // Compare as f32 - lower weight = higher priority (min heap)
                let a = f32::from_bits(self.weight_bits);
                let b = f32::from_bits(other.weight_bits);
                a.partial_cmp(&b).unwrap_or(Ordering::Equal)
                    .then(self.state.cmp(&other.state))
            }
        }
        impl PartialOrd for WState {
            fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
                Some(self.cmp(other))
            }
        }

        fn ws_cmp(a: &WState, b: &WState) -> Ordering {
            a.cmp(b)
        }

        let mut heap = Heap::new(ws_cmp, 0, 0, 0);

        let q = fst.start;
        sp.weights[q as usize] = sr.one;

        heap.insert(WState { state: q, weight_bits: sr.one.to_bits() });

        while let Some(ws) = heap.pop() {
            let p = ws.state;
            let state = &fst.states[p as usize];
            if state.final_state {
                sp.backtrace(path, p);
                return;
            }
            for arc in &state.arcs {
                let q = arc.state;
                if arc.weight == sr.zero { continue; }
                let new_w = (sr.prod)(sp.weights[p as usize], arc.weight);
                if sp.weights[q as usize] == sr.zero {
                    sp.weights[q as usize] = new_w;
                    let mut r_arc = arc.clone();
                    r_arc.state = p;
                    sp.backtrack[q as usize] = Some(r_arc);
                    heap.insert(WState { state: q, weight_bits: new_w.to_bits() });
                } else if sp.weights[q as usize] != (sr.sum)(sp.weights[q as usize], new_w) {
                    sp.weights[q as usize] = new_w;
                    let mut r_arc = arc.clone();
                    r_arc.state = p;
                    sp.backtrack[q as usize] = Some(r_arc);
                    // Update in heap - find and update
                    let old_ws = WState { state: q, weight_bits: 0 }; // won't find by bits
                    // Linear search for state q in heap
                    let mut found_idx = None;
                    for i in 0..heap.n_items {
                        if heap.items[i].state == q {
                            found_idx = Some(i);
                            break;
                        }
                    }
                    if let Some(idx) = found_idx {
                        heap.update(WState { state: q, weight_bits: new_w.to_bits() }, idx);
                    }
                }
            }
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
