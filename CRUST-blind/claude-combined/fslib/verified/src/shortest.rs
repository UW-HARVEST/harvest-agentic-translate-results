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
        let weights = vec![sr.zero; fst.n_states as usize];
        let backtrack = vec![None; fst.n_states as usize];
        ShortestPath {
            sr,
            weights,
            backtrack,
        }
    }
    pub fn backtrace(&self, path: &mut Fst, final_state: u32) {
        // Add the start state (state 0)
        path.add_state();
        let mut n: u32 = 0;
        // Count nodes to add
        let mut s = final_state;
        while s != 0 {
            path.add_state();
            s = self.backtrack[s as usize].as_ref().unwrap().state;
            n += 1;
        }
        path.set_final(n, self.sr.one);
        // Now build arcs
        let mut s = final_state;
        let mut k = n;
        while s != 0 {
            let arc = self.backtrack[s as usize].as_ref().unwrap().clone();
            path.add_arc(k - 1, k, arc.ilabel, arc.olabel, arc.weight);
            s = arc.state;
            k -= 1;
        }
    }
    pub fn find_shortest_path(fst: &Fst, path: &mut Fst) {
        // Tropical only
        let mut sp = ShortestPath::new(fst);
        let sr = sp.sr;
        let n = fst.n_states as usize;
        let mut visited = vec![false; n];
        let start = fst.start as usize;
        sp.weights[start] = sr.one;
        // Use a simple Dijkstra with linear scan (since heap is complex)
        loop {
            // find unvisited with minimum weight
            let mut best: Option<usize> = None;
            for i in 0..n {
                if visited[i] || sp.weights[i] == sr.zero {
                    continue;
                }
                if let Some(b) = best {
                    if sp.weights[i] < sp.weights[b] {
                        best = Some(i);
                    }
                } else {
                    best = Some(i);
                }
            }
            let p = match best {
                Some(p) => p,
                None => break,
            };
            visited[p] = true;
            let state = &fst.states[p];
            if state.final_state {
                sp.backtrace(path, p as u32);
                return;
            }
            for a in 0..state.n_arcs as usize {
                let arc = &state.arcs[a];
                let q = arc.state as usize;
                if arc.weight == sr.zero {
                    continue;
                }
                let new_w = (sr.prod)(sp.weights[p], arc.weight);
                let cur = sp.weights[q];
                let summed = (sr.sum)(cur, new_w);
                if cur != summed {
                    sp.weights[q] = new_w;
                    let mut r_arc = *arc;
                    r_arc.state = p as u32;
                    sp.backtrack[q] = Some(r_arc);
                }
            }
        }
    }
    pub fn states_cmp(&self, a: &u32, b: &u32) -> Ordering {
        self.weights[*a as usize]
            .partial_cmp(&self.weights[*b as usize])
            .unwrap_or(Ordering::Equal)
    }
}
pub fn states_hash(a: &u32) -> u64 {
    *a as u64
}
pub fn states_key_eq(a: &u32, b: &u32) -> bool {
    *a == *b
}
