use crate::fst::{Fst, ArcData};
use crate::sr::{self, Sr};
use std::cmp::Ordering;
use std::collections::BinaryHeap;

pub struct ShortestPath {
    pub sr: Sr,
    pub weights: Vec<f32>,
    pub backtrack: Vec<Option<ArcData>>,
}

impl ShortestPath {
    pub fn new(fst: &Fst) -> Self {
        let sr_struct = sr::sr_get(fst.sr_type);
        Self {
            weights: vec![sr_struct.zero; fst.n_states as usize],
            backtrack: vec![None; fst.n_states as usize],
            sr: sr_struct,
        }
    }
    fn backtrace(&self, fst_orig: &Fst, path: &mut Fst, final_state: u32) {
        let start = fst_orig.start;
        let sr_one = self.sr.one;

        // First state
        path.add_state();

        // Walk back from final to start counting states
        let mut s = final_state;
        let mut n: u32 = 0;
        while s != start {
            path.add_state();
            n += 1;
            if let Some(arc) = &self.backtrack[s as usize] {
                s = arc.state;
            } else {
                break;
            }
        }
        path.set_final(n, sr_one);

        // Walk back again adding arcs
        let mut s = final_state;
        let mut n: i64 = n as i64;
        while s != start {
            if let Some(arc) = &self.backtrack[s as usize] {
                let prev = arc.state;
                path.add_arc((n - 1) as u32, n as u32, arc.ilabel, arc.olabel, arc.weight);
                s = prev;
                n -= 1;
            } else {
                break;
            }
        }
    }
    pub fn find_shortest_path(fst: &Fst, path: &mut Fst) {
        let mut sp = ShortestPath::new(fst);
        let sr = sr::sr_get(fst.sr_type);

        let start = fst.start;
        sp.weights[start as usize] = sr.one;

        // Min-heap ordered by weight
        let mut heap: BinaryHeap<HeapItem> = BinaryHeap::new();
        heap.push(HeapItem { weight: sr.one, state: start });

        // Track which states have been popped
        let mut visited = vec![false; fst.n_states as usize];

        while let Some(HeapItem { weight: pw, state: p }) = heap.pop() {
            if visited[p as usize] {
                continue;
            }
            // Skip if not the latest weight
            if pw != sp.weights[p as usize] {
                continue;
            }
            visited[p as usize] = true;

            let state = &fst.states[p as usize];
            if state.final_state {
                sp.backtrace(fst, path, p);
                return;
            }

            for arc in &state.arcs {
                let q = arc.state;
                if arc.weight == sr.zero {
                    continue;
                }
                let new_w = (sr.prod)(sp.weights[p as usize], arc.weight);
                let q_w = sp.weights[q as usize];
                let combined = (sr.sum)(q_w, new_w);
                if q_w != combined {
                    sp.weights[q as usize] = new_w;
                    let mut r_arc = arc.clone();
                    r_arc.state = p;
                    sp.backtrack[q as usize] = Some(r_arc);
                    heap.push(HeapItem { weight: new_w, state: q });
                }
            }
        }
    }
    fn states_cmp(&self, a: &u32, b: &u32) -> Ordering {
        self.weights[*a as usize].partial_cmp(&self.weights[*b as usize]).unwrap_or(Ordering::Equal)
    }
}

// Min-heap wrapper: items with smaller weight have higher priority
struct HeapItem {
    weight: f32,
    state: u32,
}

impl PartialEq for HeapItem {
    fn eq(&self, other: &Self) -> bool {
        self.weight == other.weight && self.state == other.state
    }
}
impl Eq for HeapItem {}

impl Ord for HeapItem {
    fn cmp(&self, other: &Self) -> Ordering {
        // Reverse for min-heap
        other.weight.partial_cmp(&self.weight).unwrap_or(Ordering::Equal)
            .then_with(|| other.state.cmp(&self.state))
    }
}
impl PartialOrd for HeapItem {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

pub fn states_hash(a: &u32) -> u64 {
    *a as u64
}
pub fn states_key_eq(a: &u32, b: &u32) -> bool {
    a == b
}
