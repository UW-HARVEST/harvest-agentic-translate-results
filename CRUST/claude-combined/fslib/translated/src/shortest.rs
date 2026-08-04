use crate::fst::{Fst, ArcData};
use crate::sr::{Sr, sr_get};
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
        // Count distance from start to final_state
        path.add_state();
        let mut s = final_state;
        let mut n: u32 = 0;
        while s != 0 {
            let arc = self.backtrack[s as usize].as_ref().unwrap();
            path.add_state();
            s = arc.state;
            n += 1;
        }
        path.set_final(n, self.sr.one);
        let mut s = final_state;
        let mut idx = n;
        while s != 0 {
            let arc = self.backtrack[s as usize].as_ref().unwrap();
            path.add_arc(idx - 1, idx, arc.ilabel, arc.olabel, arc.weight);
            s = arc.state;
            idx -= 1;
        }
    }
    pub fn find_shortest_path(fst: &Fst, path: &mut Fst) {
        let mut sp = ShortestPath::new(fst);
        let sr = sp.sr;
        // Use a Vec as priority queue: collect candidates with weights
        // The C code uses a heap keyed by state with cmp based on W[].
        // Simpler: use a binary heap of (weight, state). But weight comparison needs careful handling.
        // Actually we'll implement Dijkstra directly:
        let n = fst.n_states as usize;
        let mut visited = vec![false; n];

        let q = fst.start;
        sp.weights[q as usize] = sr.one;

        loop {
            // Find unvisited state with minimum weight
            let mut p_opt: Option<u32> = None;
            let mut best_w = sr.zero;
            for i in 0..n {
                if !visited[i] && sp.weights[i] != sr.zero {
                    if p_opt.is_none() || sp.weights[i] < best_w {
                        best_w = sp.weights[i];
                        p_opt = Some(i as u32);
                    }
                }
            }
            let p = match p_opt {
                Some(x) => x,
                None => break,
            };
            visited[p as usize] = true;

            let state = &fst.states[p as usize];

            if state.final_state {
                sp.backtrace(path, p);
                break;
            }

            for arc in &state.arcs {
                let q = arc.state;
                if arc.weight == sr.zero {
                    continue;
                }
                let new_w = (sr.prod)(sp.weights[p as usize], arc.weight);
                if sp.weights[q as usize] != (sr.sum)(sp.weights[q as usize], new_w) {
                    sp.weights[q as usize] = new_w;
                    let mut r_arc = *arc;
                    r_arc.state = p;
                    sp.backtrack[q as usize] = Some(r_arc);
                }
            }
        }
    }
    fn states_cmp(&self, a: &u32, b: &u32) -> Ordering {
        self.weights[*a as usize]
            .partial_cmp(&self.weights[*b as usize])
            .unwrap_or(Ordering::Equal)
    }
}
fn states_hash(a: &u32) -> u64 {
    *a as u64
}
fn states_key_eq(a: &u32, b: &u32) -> bool {
    a == b
}
#[allow(dead_code)]
fn _use_helpers() {
    let _ = states_hash;
    let _ = states_key_eq;
}
