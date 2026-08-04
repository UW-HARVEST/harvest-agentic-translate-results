use crate::fst::{ArcData, Fst};
use crate::sr::{self, Sr};
use std::cmp::Ordering;

pub struct ShortestPath {
    sr: Sr,
    weights: Vec<f32>,
    backtrack: Vec<Option<ArcData>>,
}

impl ShortestPath {
    pub fn new(fst: &Fst) -> Self {
        let sr = sr::sr_get(fst.sr_type);
        Self {
            sr,
            weights: vec![sr.zero(); fst.n_states as usize],
            backtrack: vec![None; fst.n_states as usize],
        }
    }

    fn backtrace(&self, path: &mut Fst, final_state: u32) {
        path.empty();
        path.sr_type = 0;
        path.add_state();

        let mut s = final_state;
        let mut n = 0u32;
        while s != 0 {
            path.add_state();
            s = self.backtrack[s as usize].as_ref().map(|arc| arc.state).unwrap_or(0);
            n += 1;
        }

        path.set_final(n, self.sr.one());
        s = final_state;
        while s != 0 {
            if let Some(arc) = self.backtrack[s as usize].as_ref() {
                path.add_arc(n - 1, n, arc.ilabel, arc.olabel, arc.weight);
                s = arc.state;
                n -= 1;
            } else {
                break;
            }
        }
    }

    pub fn find_shortest_path(fst: &Fst, path: &mut Fst) {
        let mut shortest = Self::new(fst);
        shortest.weights.fill(shortest.sr.zero());
        shortest.backtrack.fill(None);

        let mut open = Vec::<u32>::new();
        let start = fst.start;
        shortest.weights[start as usize] = shortest.sr.one();
        open.push(start);

        while !open.is_empty() {
            let mut best_idx = 0usize;
            for i in 1..open.len() {
                let lhs = shortest.weights[open[i] as usize];
                let rhs = shortest.weights[open[best_idx] as usize];
                if lhs < rhs {
                    best_idx = i;
                }
            }
            let p = open.swap_remove(best_idx);
            let state = &fst.states[p as usize];

            if state.final_state {
                shortest.backtrace(path, p);
                break;
            }

            for arc in &state.arcs {
                let q = arc.state;
                if arc.weight == shortest.sr.zero() {
                    continue;
                }

                if shortest.weights[q as usize] == shortest.sr.zero() && !open.contains(&q) {
                    open.push(q);
                }

                let candidate = shortest.sr.prod(shortest.weights[p as usize], arc.weight);
                let current = shortest.weights[q as usize];
                if current != shortest.sr.sum(current, candidate) {
                    shortest.weights[q as usize] = candidate;
                    let mut rev_arc = arc.clone();
                    rev_arc.state = p;
                    shortest.backtrack[q as usize] = Some(rev_arc);
                    if !open.contains(&q) {
                        open.push(q);
                    }
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
