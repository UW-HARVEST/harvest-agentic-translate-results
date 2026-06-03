use crate::fst::{Fst, ArcData};
use crate::sr::sr_get;
use crate::heap::Heap;
use std::cmp::Ordering;

pub struct ShortestPath;
impl ShortestPath {
    pub fn new(_fst: &Fst) -> Self {
        ShortestPath
    }
    pub fn find_shortest_path(fst: &Fst, path: &mut Fst) {
        assert_eq!(fst.sr_type, crate::fst::SR_TROPICAL);
        let sr = sr_get(fst.sr_type);
        let n = fst.n_states as usize;
        let mut weights: Vec<f32> = vec![sr.zero; n];
        let mut backtrack: Vec<Option<ArcData>> = vec![None; n];

        // Use a min-heap of state indices.
        // We'll track states in an immediate vector and re-insert/update.
        // To compare: state x is less than y iff weights[x] is the better (sum) of both.
        // We can't easily make a `fn` with closure capture, so we'll use a custom approach:
        // capture weights via static? Use a thread-local.

        thread_local! {
            static WEIGHTS: std::cell::RefCell<Vec<f32>> = std::cell::RefCell::new(Vec::new());
        }
        WEIGHTS.with(|w| {
            *w.borrow_mut() = weights.clone();
        });

        fn states_cmp(a: &u32, b: &u32) -> Ordering {
            WEIGHTS.with(|w| {
                let w = w.borrow();
                let wa = w[*a as usize];
                let wb = w[*b as usize];
                if wa < wb { Ordering::Less }
                else if wa > wb { Ordering::Greater }
                else { Ordering::Equal }
            })
        }

        let mut q: Heap<u32> = Heap::new(states_cmp, std::mem::size_of::<u32>(), 0, 0);

        let start = fst.start;
        weights[start as usize] = sr.one;
        WEIGHTS.with(|w| { w.borrow_mut()[start as usize] = sr.one; });
        q.insert(start);

        while let Some(p) = q.pop() {
            let state = &fst.states[p as usize];
            if state.final_state {
                ShortestPath::backtrace(path, p, &backtrack, &sr);
                break;
            }
            for a in 0..state.n_arcs as usize {
                let arc = &state.arcs[a];
                let qs = arc.state;
                if arc.weight == sr.zero {
                    continue;
                }
                if weights[qs as usize] == sr.zero {
                    q.insert(qs);
                }
                let new_w = (sr.prod)(weights[p as usize], arc.weight);
                let combined = (sr.sum)(weights[qs as usize], new_w);
                if weights[qs as usize] != combined {
                    weights[qs as usize] = new_w;
                    WEIGHTS.with(|w| { w.borrow_mut()[qs as usize] = new_w; });
                    let mut r_arc = arc.clone();
                    r_arc.state = p;
                    backtrack[qs as usize] = Some(r_arc);
                    if let Some(idx) = q.find(&qs) {
                        q.update(qs, idx);
                    }
                }
            }
        }
    }
    fn backtrace(path: &mut Fst, f: u32, backtrack: &Vec<Option<ArcData>>, sr: &crate::sr::Sr) {
        path.add_state();
        let mut n: u32 = 0;
        let mut s = f;
        while s != 0 {
            path.add_state();
            n += 1;
            match &backtrack[s as usize] {
                Some(arc) => s = arc.state,
                None => break,
            }
        }
        path.set_final(n, sr.one);
        let mut s = f;
        let mut idx = n;
        while s != 0 {
            let arc_opt = backtrack[s as usize].clone();
            match arc_opt {
                Some(arc) => {
                    path.add_arc(idx - 1, idx, arc.ilabel, arc.olabel, arc.weight);
                    s = arc.state;
                    idx -= 1;
                }
                None => break,
            }
        }
    }
}
fn states_hash(a: &u32) -> u64 {
    *a as u64
}
fn states_key_eq(a: &u32, b: &u32) -> bool {
    a == b
}
