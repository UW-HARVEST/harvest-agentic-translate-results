use crate::fst::{Fst, ArcData};
use crate::sr::{Sr, sr_get, SR_TROPICAL_TYPE};
use std::cmp::Ordering;

pub struct ShortestPath {
    pub sr: Sr,
    pub weights: Vec<f32>,
    pub backtrack: Vec<Option<ArcData>>,
}

#[derive(Clone, Copy)]
struct Entry {
    weight: f32,
    state: u32,
}
impl PartialEq for Entry {
    fn eq(&self, other: &Self) -> bool {
        self.weight == other.weight
    }
}
impl Eq for Entry {}
impl PartialOrd for Entry {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        // Reverse order so that BinaryHeap produces a min-heap.
        other.weight.partial_cmp(&self.weight)
    }
}
impl Ord for Entry {
    fn cmp(&self, other: &Self) -> Ordering {
        other.weight.partial_cmp(&self.weight).unwrap_or(Ordering::Equal)
    }
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
    fn backtrace(&self, path: &mut Fst, final_state: u32) {
        path.add_state();
        let mut n: u32 = 0;
        let mut s = final_state;
        while s != 0 {
            let arc = match self.backtrack[s as usize] {
                Some(a) => a,
                None => break,
            };
            path.add_state();
            s = arc.state;
            n += 1;
        }
        path.set_final(n, self.sr.one);
        let mut s = final_state;
        let mut idx = n;
        while s != 0 {
            let arc = match self.backtrack[s as usize] {
                Some(a) => a,
                None => break,
            };
            if idx == 0 {
                break;
            }
            path.add_arc(idx - 1, idx, arc.ilabel, arc.olabel, arc.weight);
            s = arc.state;
            idx -= 1;
        }
    }
    pub fn find_shortest_path(fst: &Fst, path: &mut Fst) {
        assert_eq!(fst.sr_type, SR_TROPICAL_TYPE);
        let mut sp = ShortestPath::new(fst);
        let q_start = fst.start;
        sp.weights[q_start as usize] = sp.sr.one;

        use std::collections::BinaryHeap;
        let mut heap: BinaryHeap<Entry> = BinaryHeap::new();
        heap.push(Entry {
            weight: sp.sr.one,
            state: q_start,
        });

        while let Some(entry) = heap.pop() {
            let p = entry.state;
            // Skip stale entries: only handle if it matches the recorded weight.
            if entry.weight != sp.weights[p as usize] {
                continue;
            }
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
                let combined = (sp.sr.sum)(sp.weights[q as usize], new_w);
                if sp.weights[q as usize] != combined {
                    sp.weights[q as usize] = new_w;
                    let mut r_arc = *arc;
                    r_arc.state = p;
                    sp.backtrack[q as usize] = Some(r_arc);
                    heap.push(Entry {
                        weight: new_w,
                        state: q,
                    });
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

#[allow(dead_code)]
fn _force_use() {
    let sp = ShortestPath {
        sr: crate::sr::SR_TROPICAL,
        weights: vec![],
        backtrack: vec![],
    };
    let _ = sp.states_cmp(&0, &0);
    let _ = states_hash(&0);
    let _ = states_key_eq(&0, &0);
}
