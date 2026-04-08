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
        let mut n: u32 = 0;
        let mut s = final_state;
        path.add_state();
        while s != 0 {
            if let Some(ref arc) = self.backtrack[s as usize] {
                s = arc.state;
                n += 1;
                path.add_state();
            } else {
                break;
            }
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
        let sr = crate::sr::sr_get(fst.sr_type);

        // We need a wrapper for the heap that uses the weights array for comparison
        // Since the C code uses global statics, we'll use a different approach
        let weights_ptr = &sp.weights as *const Vec<f32>;

        let cmp_fn: fn(&u32, &u32) -> Ordering = |a: &u32, b: &u32| -> Ordering {
            // We can't access weights here directly in a fn pointer
            // Use natural ordering as placeholder - actual comparison done differently
            a.cmp(b)
        };

        // Use a simple priority approach: Vec-based with manual extraction
        let mut queue: Vec<u32> = Vec::new();
        let mut in_queue: HashMap<u32, bool> = HashMap::new();

        sp.weights[fst.start as usize] = sr.one;
        queue.push(fst.start);
        in_queue.insert(fst.start, true);

        while !queue.is_empty() {
            // Find minimum weight element
            let mut min_idx = 0;
            for i in 1..queue.len() {
                if sp.weights[queue[i] as usize] == (sr.sum)(sp.weights[queue[i] as usize], sp.weights[queue[min_idx] as usize]) {
                    // queue[i] has smaller or equal weight (tropical: min)
                    if sp.weights[queue[i] as usize] < sp.weights[queue[min_idx] as usize] {
                        min_idx = i;
                    }
                }
            }
            let p = queue.swap_remove(min_idx);
            in_queue.remove(&p);

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
                    // Unexplored
                    sp.weights[q as usize] = new_w;
                    let mut r_arc = arc.clone();
                    r_arc.state = p;
                    sp.backtrack[q as usize] = Some(r_arc);
                    queue.push(q);
                    in_queue.insert(q, true);
                } else if sp.weights[q as usize] != (sr.sum)(sp.weights[q as usize], new_w) {
                    sp.weights[q as usize] = new_w;
                    let mut r_arc = arc.clone();
                    r_arc.state = p;
                    sp.backtrack[q as usize] = Some(r_arc);
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
    *a == *b
}
