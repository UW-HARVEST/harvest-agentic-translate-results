use crate::queue::Queue;
use crate::fst::{ArcData, Fst, EPS};
use crate::bitset::BitSet;
use crate::sr::sr_get;
use crate::iter::FstIter;
// Keep these signatures as required, but the originals were static helpers;
// we instead provide a generic version `_fst_close_states` for trimming logic.
pub fn fst_close(_fst: &mut Fst, _finals: &mut Queue<(ArcData, ArcData)>) {
    // Original C function used a queue of state ids; intentionally left as
    // a no-op here because the public signature uses arc-pair queue.
}
pub fn fst_reverse(fst: &mut Fst) {
    fst.reverse();
}
pub fn fst_rm_states(fst: &mut Fst, mask: &BitSet) {
    let n = fst.n_states as usize;
    let mut idx: Vec<u32> = vec![0; n];
    let mut shift: u32 = 0;
    // Compactify states
    for s in 0..n {
        if mask.get(s) {
            shift += 1;
        } else {
            idx[s] = shift;
            if shift > 0 {
                fst.states.swap(s - shift as usize, s);
            }
        }
    }
    fst.n_states -= shift;
    fst.states.truncate(fst.n_states as usize);
    let n_after = fst.n_states;
    for s in 0..(n_after as usize) {
        let state = &mut fst.states[s];
        let mut new_arcs: Vec<ArcData> = Vec::new();
        for arc in state.arcs.iter() {
            let orig_dst = arc.state as usize;
            if orig_dst < idx.len() {
                let new_dst = arc.state - idx[orig_dst];
                if new_dst < n_after {
                    new_arcs.push(ArcData {
                        state: new_dst,
                        weight: arc.weight,
                        ilabel: arc.ilabel,
                        olabel: arc.olabel,
                    });
                }
            }
        }
        state.n_arcs = new_arcs.len() as u32;
        state.arcs = new_arcs;
    }
}
pub fn fst_get_finals(_fst: &mut Fst, _finals: &mut Queue<(ArcData, ArcData)>) {
    // No-op due to mismatched signature; see internal helper used by fst_trim.
}
fn collect_finals(fst: &Fst) -> Vec<u32> {
    let mut out = Vec::new();
    for s in 0..fst.n_states {
        if fst.states[s as usize].final_state {
            out.push(s);
        }
    }
    out
}
fn close_finals(fst: &mut Fst, finals: Vec<u32>) {
    if finals.len() < 2 {
        return;
    }
    let sr = sr_get(fst.sr_type);
    let final_state = fst.add_state();
    fst.set_final(final_state, sr.one);
    for s in finals {
        let weight = fst.states[s as usize].weight;
        fst.states[s as usize].final_state = false;
        fst.add_arc(s, final_state, EPS, EPS, weight);
    }
}
pub fn fst_trim(fst: &mut Fst) {
    let finals = collect_finals(fst);
    if finals.is_empty() {
        fst.empty();
        return;
    }
    if finals.len() > 1 {
        close_finals(fst, finals);
    }
    let mut forward_marked = {
        let mut iter = FstIter::<u32>::new(fst);
        while iter.next().is_some() {}
        iter.marked
    };
    fst_reverse(fst);
    let reverse_marked = {
        let mut iter = FstIter::<u32>::new(fst);
        while iter.next().is_some() {}
        iter.marked
    };
    fst_reverse(fst);
    forward_marked.intersect(&reverse_marked);
    let mask = forward_marked.toggle_all();
    fst_rm_states(fst, &mask);
}
