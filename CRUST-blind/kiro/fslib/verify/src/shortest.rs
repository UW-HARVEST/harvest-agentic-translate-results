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
            weights: vec![sr.zero; n],
            backtrack: vec![None; n],
            sr,
        }
    }
    fn backtrace(&self, path: &mut Fst, final_state: u32) {
        // Count path length
        let mut n: u32 = 0;
        let mut s = final_state;
        path.add_state();
        while s != 0 {
            path.add_state();
            if let Some(ref arc) = self.backtrack[s as usize] {
                s = arc.state;
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
        let mut sp = ShortestPath::new(fst);
        // Use a simple BinaryHeap-like approach with Vec
        // The C code uses a min-heap where comparison is based on weights
        let weights_ptr = &sp.weights as *const Vec<f32>;
        let sr_sum = sp.sr.sum;

        let cmp_fn: fn(&u32, &u32) -> Ordering = |a: &u32, b: &u32| -> Ordering {
            // This is a placeholder - we need the weights accessible
            a.cmp(b)
        };

        // We'll use a simpler approach: manual priority queue with Vec
        let mut queue: Vec<u32> = Vec::new();
        let mut in_queue: Vec<bool> = vec![false; fst.n_states as usize];

        let start = fst.start;
        sp.weights[start as usize] = sp.sr.one;
        queue.push(start);
        in_queue[start as usize] = true;

        while !queue.is_empty() {
            // Find min weight element in queue
            let mut min_idx = 0;
            for i in 1..queue.len() {
                let wi = sp.weights[queue[i] as usize];
                let wm = sp.weights[queue[min_idx] as usize];
                if (sp.sr.sum)(wi, wm) == wi {
                    // wi is "better" (smaller in tropical)
                    min_idx = i;
                }
            }
            let p = queue.swap_remove(min_idx);
            in_queue[p as usize] = false;

            let state = &fst.states[p as usize];
            if state.final_state {
                sp.backtrace(path, p);
                return;
            }

            for arc in &state.arcs {
                let q = arc.state;
                if arc.weight == sp.sr.zero {
                    continue;
                }
                let new_w = (sp.sr.prod)(sp.weights[p as usize], arc.weight);
                if sp.weights[q as usize] == sp.sr.zero {
                    sp.weights[q as usize] = new_w;
                    let mut r_arc = arc.clone();
                    r_arc.state = p;
                    sp.backtrack[q as usize] = Some(r_arc);
                    if !in_queue[q as usize] {
                        queue.push(q);
                        in_queue[q as usize] = true;
                    }
                } else if sp.weights[q as usize] != (sp.sr.sum)(sp.weights[q as usize], new_w) {
                    sp.weights[q as usize] = new_w;
                    let mut r_arc = arc.clone();
                    r_arc.state = p;
                    sp.backtrack[q as usize] = Some(r_arc);
                    // already in queue or will be re-added
                    if !in_queue[q as usize] {
                        queue.push(q);
                        in_queue[q as usize] = true;
                    }
                }
            }
        }
    }
    fn states_cmp(&self, a: &u32, b: &u32) -> Ordering {
        let wa = self.weights[*a as usize];
        let wb = self.weights[*b as usize];
        if wa == (self.sr.sum)(wa, wb) {
            Ordering::Less
        } else {
            Ordering::Greater
        }
    }
}
fn states_hash(a: &u32) -> u64 {
    *a as u64
}
fn states_key_eq(a: &u32, b: &u32) -> bool {
    a == b
}
