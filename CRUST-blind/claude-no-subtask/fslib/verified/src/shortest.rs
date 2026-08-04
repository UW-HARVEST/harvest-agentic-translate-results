use crate::fst::{Fst, ArcData, State};
use crate::sr::{Sr, sr_get};
#[allow(unused_imports)]
use crate::heap::Heap;
#[allow(unused_imports)]
use std::collections::HashMap;
use std::cmp::Ordering;

#[derive(Clone, Copy)]
struct HeapEntry {
    weight: f32,
    state: State,
}
impl Eq for HeapEntry {}
impl PartialEq for HeapEntry {
    fn eq(&self, other: &Self) -> bool {
        self.weight == other.weight && self.state == other.state
    }
}
impl Ord for HeapEntry {
    fn cmp(&self, other: &Self) -> Ordering {
        other.weight.partial_cmp(&self.weight).unwrap_or(Ordering::Equal)
            .then_with(|| other.state.cmp(&self.state))
    }
}
impl PartialOrd for HeapEntry {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

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

    fn backtrace(&self, path: &mut Fst, final_state: u32) {
        // Two-scan approach:
        // First add states for all backtrace
        path.add_state(); // state 0
        let mut n: u32 = 0;
        let mut s = final_state;
        while s != 0 {
            path.add_state();
            if let Some(arc) = &self.backtrack[s as usize] {
                s = arc.state;
            } else {
                break;
            }
            n += 1;
        }
        path.set_final(n, self.sr.one);

        // Now trace back and add arcs
        let mut s = final_state;
        let mut n_arc = n;
        while s != 0 {
            if let Some(arc) = &self.backtrack[s as usize] {
                let arc = *arc;
                if n_arc == 0 {
                    break;
                }
                path.add_arc(n_arc - 1, n_arc, arc.ilabel, arc.olabel, arc.weight);
                s = arc.state;
                n_arc -= 1;
            } else {
                break;
            }
        }
    }

    pub fn find_shortest_path(fst: &Fst, path: &mut Fst) {
        // SR_TROPICAL only
        assert_eq!(fst.sr_type, 0);
        let sr = sr_get(fst.sr_type);

        let mut sp = ShortestPath::new(fst);

        // Heap of states; cmp is by weights[a] vs weights[b], min-heap
        // Use a simpler approach: Plain BinaryHeap with reverse ordering
        // We'll implement Dijkstra-like algorithm
        use std::collections::BinaryHeap;

        let mut heap: BinaryHeap<HeapEntry> = BinaryHeap::new();
        let q = fst.start;
        sp.weights[q as usize] = sr.one;
        heap.push(HeapEntry { weight: sr.one, state: q });

        while let Some(entry) = heap.pop() {
            let p = entry.state;

            // Skip stale entries
            if entry.weight != sp.weights[p as usize] {
                continue;
            }

            let state = &fst.states[p as usize];
            if state.final_state {
                sp.backtrace(path, p);
                return;
            }
            for a in 0..state.n_arcs as usize {
                let arc = &state.arcs[a];
                let q_dst = arc.state;

                if arc.weight == sr.zero {
                    continue;
                }

                let new_weight = (sr.prod)(sp.weights[p as usize], arc.weight);
                let summed = (sr.sum)(sp.weights[q_dst as usize], new_weight);
                if sp.weights[q_dst as usize] != summed {
                    sp.weights[q_dst as usize] = new_weight;

                    let mut r_arc = *arc;
                    r_arc.state = p;
                    sp.backtrack[q_dst as usize] = Some(r_arc);

                    heap.push(HeapEntry { weight: new_weight, state: q_dst });
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

pub fn states_hash(a: &u32) -> u64 {
    *a as u64
}

pub fn states_key_eq(a: &u32, b: &u32) -> bool {
    *a == *b
}
