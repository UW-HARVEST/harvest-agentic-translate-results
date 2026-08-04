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
        let weights = vec![sr.zero; n];
        let backtrack: Vec<Option<ArcData>> = (0..n).map(|_| None).collect();
        ShortestPath {
            sr,
            weights,
            backtrack,
        }
    }
    fn backtrace(&self, path: &mut Fst, final_state: u32) {
        // Mirrors C `backtrace`. The backtrack array contains arcs whose
        // `state` field is the parent state.
        // Add states; final_state is f
        path.add_state();
        let mut n: u32 = 0;
        let mut s = final_state;
        while s != 0 {
            path.add_state();
            let arc = match &self.backtrack[s as usize] {
                Some(a) => a.clone(),
                None => break,
            };
            s = arc.state;
            n += 1;
        }
        path.set_final(n, self.sr.one);
        // backtrack again
        let mut s = final_state;
        let mut nn: u32 = n;
        while s != 0 {
            let arc = match &self.backtrack[s as usize] {
                Some(a) => a.clone(),
                None => break,
            };
            path.add_arc(nn - 1, nn, arc.ilabel, arc.olabel, arc.weight);
            s = arc.state;
            nn -= 1;
        }
    }
    pub fn find_shortest_path(fst: &Fst, path: &mut Fst) {
        assert_eq!(fst.sr_type, crate::fst::SR_TROPICAL);
        let sr = sr_get(fst.sr_type);
        let n_states = fst.n_states as usize;
        let mut weights = vec![sr.zero; n_states];
        let mut backtrack: Vec<Option<ArcData>> = (0..n_states).map(|_| None).collect();

        // Use a min-heap based on weights
        // We'll use a simple binary heap of (weight, state) — but states can
        // be re-added after weight is decreased.
        use std::collections::BinaryHeap;
        #[derive(PartialEq)]
        struct Item {
            weight: f32,
            state: u32,
        }
        impl Eq for Item {}
        impl PartialOrd for Item {
            fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
                Some(self.cmp(other))
            }
        }
        impl Ord for Item {
            fn cmp(&self, other: &Self) -> Ordering {
                // Reverse for min-heap
                other
                    .weight
                    .partial_cmp(&self.weight)
                    .unwrap_or(Ordering::Equal)
                    .then_with(|| other.state.cmp(&self.state))
            }
        }

        let mut heap: BinaryHeap<Item> = BinaryHeap::new();
        let q = fst.start;
        weights[q as usize] = sr.one;
        heap.push(Item {
            weight: weights[q as usize],
            state: q,
        });

        while let Some(Item { weight: w_p, state: p }) = heap.pop() {
            // Skip stale entries
            if w_p != weights[p as usize] {
                continue;
            }
            let state = &fst.states[p as usize];
            if state.final_state {
                // backtrace
                let n: u32 = {
                    let mut count: u32 = 0;
                    let mut s = p;
                    while s != 0 {
                        let a = match &backtrack[s as usize] {
                            Some(a) => a.clone(),
                            None => break,
                        };
                        s = a.state;
                        count += 1;
                    }
                    count
                };
                // add states
                for _ in 0..(n + 1) {
                    path.add_state();
                }
                path.set_final(n, sr.one);
                let mut s = p;
                let mut nn = n;
                while s != 0 {
                    let arc = match &backtrack[s as usize] {
                        Some(a) => a.clone(),
                        None => break,
                    };
                    path.add_arc(nn - 1, nn, arc.ilabel, arc.olabel, arc.weight);
                    s = arc.state;
                    if nn == 0 {
                        break;
                    }
                    nn -= 1;
                }
                return;
            }
            for arc in &state.arcs {
                let qd = arc.state;
                if arc.weight == sr.zero {
                    continue;
                }
                let new_w = (sr.prod)(w_p, arc.weight);
                if weights[qd as usize] == sr.zero {
                    // first visit
                    weights[qd as usize] = new_w;
                    let mut r_arc = arc.clone();
                    r_arc.state = p;
                    backtrack[qd as usize] = Some(r_arc);
                    heap.push(Item {
                        weight: new_w,
                        state: qd,
                    });
                } else if (sr.sum)(weights[qd as usize], new_w) != weights[qd as usize] {
                    weights[qd as usize] = new_w;
                    let mut r_arc = arc.clone();
                    r_arc.state = p;
                    backtrack[qd as usize] = Some(r_arc);
                    heap.push(Item {
                        weight: new_w,
                        state: qd,
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
