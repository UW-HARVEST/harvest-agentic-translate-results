use crate::fst::{ArcData, Fst, State};
use crate::sr::{self, Sr};
use std::cmp::Ordering;

pub struct ShortestPath {
    pub sr: Sr,
    pub weights: Vec<f32>,
    pub backtrack: Vec<Option<ArcData>>,
}

#[derive(Clone, Copy)]
struct Node {
    w: f32,
    s: State,
}
impl PartialEq for Node {
    fn eq(&self, other: &Self) -> bool {
        self.w == other.w
    }
}
impl Eq for Node {}
impl PartialOrd for Node {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for Node {
    fn cmp(&self, other: &Self) -> Ordering {
        // For min-heap on f32, we reverse the comparison.
        other
            .w
            .partial_cmp(&self.w)
            .unwrap_or(Ordering::Equal)
    }
}

impl ShortestPath {
    pub fn new(fst: &Fst) -> Self {
        let sr_inst = sr::sr_get(fst.sr_type);
        let zero = sr_inst.zero;
        ShortestPath {
            sr: sr_inst,
            weights: vec![zero; fst.n_states as usize],
            backtrack: vec![None; fst.n_states as usize],
        }
    }

    fn backtrace(&self, path: &mut Fst, final_state: u32) {
        // Compute path length first.
        let mut n: u32 = 0;
        let mut s = final_state;
        path.add_state();
        while s != 0 {
            let arc = match &self.backtrack[s as usize] {
                Some(a) => a.clone(),
                None => break,
            };
            path.add_state();
            s = arc.state;
            n += 1;
        }
        path.set_final(n, self.sr.one);

        // Now walk back again to add arcs.
        let mut s = final_state;
        let mut n2 = n;
        while s != 0 {
            let arc = match &self.backtrack[s as usize] {
                Some(a) => a.clone(),
                None => break,
            };
            path.add_arc(n2 - 1, n2, arc.ilabel, arc.olabel, arc.weight);
            s = arc.state;
            n2 -= 1;
        }
    }

    pub fn find_shortest_path(fst: &Fst, path: &mut Fst) {
        // Tropical only
        let sr_inst = sr::sr_get(fst.sr_type);
        let mut weights: Vec<f32> = vec![sr_inst.zero; fst.n_states as usize];
        let mut backtrack: Vec<Option<ArcData>> = vec![None; fst.n_states as usize];

        use std::collections::BinaryHeap;
        let mut heap: BinaryHeap<Node> = BinaryHeap::new();

        let q = fst.start;
        weights[q as usize] = sr_inst.one;
        heap.push(Node { w: sr_inst.one, s: q });

        let mut final_state_found: Option<State> = None;

        while let Some(Node { w: cur_w, s: p }) = heap.pop() {
            if cur_w != weights[p as usize] {
                continue;
            }
            let state = &fst.states[p as usize];
            if state.final_state {
                final_state_found = Some(p);
                break;
            }
            for a in 0..state.n_arcs {
                let arc = &state.arcs[a as usize];
                let q = arc.state;
                if arc.weight == sr_inst.zero {
                    continue;
                }
                let new_w = (sr_inst.prod)(weights[p as usize], arc.weight);
                let summed = (sr_inst.sum)(weights[q as usize], new_w);
                if weights[q as usize] != summed {
                    weights[q as usize] = new_w;
                    let mut r_arc = arc.clone();
                    r_arc.state = p;
                    backtrack[q as usize] = Some(r_arc);
                    heap.push(Node { w: new_w, s: q });
                }
            }
        }

        if let Some(f) = final_state_found {
            let sp = ShortestPath {
                sr: sr_inst,
                weights,
                backtrack,
            };
            sp.backtrace(path, f);
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

pub fn states_hash(a: &u32) -> u64 {
    *a as u64
}

pub fn states_key_eq(a: &u32, b: &u32) -> bool {
    *a == *b
}

#[allow(dead_code)]
fn _ref(sp: &ShortestPath) {
    let _ = sp.states_cmp(&0, &0);
}
