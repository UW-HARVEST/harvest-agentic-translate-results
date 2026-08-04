use crate::fst::{Fst, ArcData};
use crate::sr::{Sr, sr_get};
use std::cmp::Ordering;
use std::collections::HashSet;

pub struct ShortestPath {
    sr: Sr,
    weights: Vec<f32>,
    backtrack: Vec<Option<ArcData>>,
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
        // Walk B[] from final_state back to start (state 0).
        // First add a single state to mirror the C version (start node).
        path.add_state();

        let mut s = final_state;
        let mut n: u32 = 0;
        while s != 0 {
            path.add_state();
            let prev = match &self.backtrack[s as usize] {
                Some(arc) => arc.state,
                None => break,
            };
            s = prev;
            n += 1;
        }
        path.set_final(n, self.sr.one);

        // Second walk: emit arcs.
        let mut s = final_state;
        let mut k = n;
        while s != 0 {
            let arc = match &self.backtrack[s as usize] {
                Some(arc) => arc.clone(),
                None => break,
            };
            // arc.state holds the *predecessor* state id (set in
            // find_shortest_path).
            let prev = arc.state;
            path.add_arc(k - 1, k, arc.ilabel, arc.olabel, arc.weight);
            s = prev;
            if k == 0 { break; }
            k -= 1;
        }
    }

    pub fn find_shortest_path(fst: &Fst, path: &mut Fst) {
        // Tropical-only Dijkstra-style shortest path.
        let mut sp = ShortestPath::new(fst);
        let sr = sp.sr;

        let n = fst.n_states as usize;
        if n == 0 {
            return;
        }

        let start = fst.start;
        sp.weights[start as usize] = sr.one;

        // Use a binary heap with custom ordering: lower weight has higher priority.
        let mut in_heap: HashSet<u32> = HashSet::new();
        let mut frontier: Vec<u32> = Vec::new();
        frontier.push(start);
        in_heap.insert(start);

        // Simple priority-queue: linear scan for the min weight (n is small in tests).
        let mut final_state: Option<u32> = None;
        loop {
            // Pop min
            if frontier.is_empty() {
                break;
            }
            let mut min_idx = 0usize;
            for i in 1..frontier.len() {
                let wi = sp.weights[frontier[i] as usize];
                let wmin = sp.weights[frontier[min_idx] as usize];
                if (sr.sum)(wi, wmin) == wi && wi != wmin {
                    min_idx = i;
                } else if wi < wmin {
                    min_idx = i;
                }
            }
            let p = frontier.swap_remove(min_idx);
            in_heap.remove(&p);

            let state = &fst.states[p as usize];
            if state.final_state {
                final_state = Some(p);
                break;
            }

            for arc in state.arcs.iter() {
                let q = arc.state;
                if arc.weight == sr.zero {
                    continue;
                }
                if sp.weights[q as usize] == sr.zero {
                    if !in_heap.contains(&q) {
                        frontier.push(q);
                        in_heap.insert(q);
                    }
                }
                let candidate = (sr.prod)(sp.weights[p as usize], arc.weight);
                let current = sp.weights[q as usize];
                let new_w = (sr.sum)(current, candidate);
                if current != new_w {
                    sp.weights[q as usize] = candidate;
                    let mut r_arc = arc.clone();
                    r_arc.state = p; // record predecessor
                    sp.backtrack[q as usize] = Some(r_arc);
                    if !in_heap.contains(&q) {
                        frontier.push(q);
                        in_heap.insert(q);
                    }
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
