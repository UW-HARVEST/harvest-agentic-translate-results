use crate::fst::{Fst, ArcData};
use crate::sr::Sr;
use std::cmp::Ordering;
pub struct ShortestPath {
    sr: Sr,
    weights: Vec<f32>,
    backtrack: Vec<Option<ArcData>>,
}
impl ShortestPath {
    pub fn new(fst: &Fst) -> Self {
        let sr = crate::sr::sr_get(fst.sr_type);
        let zero = sr.zero();
        Self {
            sr,
            weights: vec![zero; fst.n_states as usize],
            backtrack: std::iter::repeat_with(|| None)
                .take(fst.n_states as usize)
                .collect(),
        }
    }
    fn backtrace(&self, path: &mut Fst, final_state: u32) {
        let mut arcs = Vec::new();
        let mut current = final_state;
        while let Some(arc) = &self.backtrack[current as usize] {
            arcs.push(ArcData {
                state: arc.state,
                weight: arc.weight,
                ilabel: arc.ilabel,
                olabel: arc.olabel,
            });
            current = arc.state;
        }

        path.add_state();
        for _ in 0..arcs.len() {
            path.add_state();
        }
        path.set_final(arcs.len() as u32, self.sr.one());

        let mut idx = arcs.len();
        for arc in arcs {
            path.add_arc((idx - 1) as u32, idx as u32, arc.ilabel, arc.olabel, arc.weight);
            idx -= 1;
        }
    }
    pub fn find_shortest_path(fst: &Fst, path: &mut Fst) {
        let mut sp = Self::new(fst);
        sp.weights.fill(sp.sr.zero());

        let start = fst.start;
        sp.weights[start as usize] = sp.sr.one();

        let mut frontier = vec![start];
        let mut in_frontier = vec![false; fst.n_states as usize];
        in_frontier[start as usize] = true;

        while !frontier.is_empty() {
            let mut best_idx = 0usize;
            for i in 1..frontier.len() {
                if sp.states_cmp(&frontier[i], &frontier[best_idx]) == Ordering::Less {
                    best_idx = i;
                }
            }
            let p = frontier.swap_remove(best_idx);
            in_frontier[p as usize] = false;
            let state = &fst.states[p as usize];

            if state.final_state {
                sp.backtrace(path, p);
                return;
            }

            for arc in &state.arcs {
                let q = arc.state;
                if arc.weight == sp.sr.zero() {
                    continue;
                }

                if sp.weights[q as usize] == sp.sr.zero() && !in_frontier[q as usize] {
                    frontier.push(q);
                    in_frontier[q as usize] = true;
                }

                let candidate = sp.sr.prod(sp.weights[p as usize], arc.weight);
                if sp.weights[q as usize] != sp.sr.sum(sp.weights[q as usize], candidate) {
                    sp.weights[q as usize] = candidate;
                    sp.backtrack[q as usize] = Some(ArcData {
                        state: p,
                        weight: arc.weight,
                        ilabel: arc.ilabel,
                        olabel: arc.olabel,
                    });
                    if !in_frontier[q as usize] {
                        frontier.push(q);
                        in_frontier[q as usize] = true;
                    }
                }
            }
        }
    }
    fn states_cmp(&self, a: &u32, b: &u32) -> Ordering {
        if self.weights[*a as usize]
            == self
                .sr
                .sum(self.weights[*a as usize], self.weights[*b as usize])
        {
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
