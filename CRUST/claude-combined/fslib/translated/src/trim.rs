use crate::queue::Queue;
use crate::fst::{ArcData, Fst, EPS};
use crate::bitset::BitSet;
use crate::sr::sr_get;
use crate::iter::FstIter;

pub fn fst_close(fst: &mut Fst, finals: &mut Queue<u32>) {
    if finals.len() < 2 {
        return;
    }
    let sr = sr_get(fst.sr_type);
    let final_s = fst.add_state();
    fst.set_final(final_s, sr.one);
    while let Some(s) = finals.dequeue() {
        let weight = fst.states[s as usize].weight;
        fst.states[s as usize].final_state = false;
        fst.add_arc(s, final_s, EPS, EPS, weight);
    }
}

pub fn fst_reverse(fst: &mut Fst) {
    let sr = sr_get(fst.sr_type);
    let orig = fst.clone();
    let start_s = fst.start;

    for state in fst.states.iter_mut() {
        state.n_arcs = 0;
        state.arcs.clear();
        // change start to final
        if state.final_state {
            state.final_state = false;
        }
    }
    // Find the final state (post above) — find first final state in orig
    for s in 0..orig.n_states as usize {
        if orig.states[s].final_state {
            fst.start = s as u32;
            break;
        }
    }
    fst.set_final(start_s, sr.one);
    for s in 0..orig.n_states {
        let state = &orig.states[s as usize];
        for arc in &state.arcs {
            fst.add_arc(arc.state, s, arc.ilabel, arc.olabel, arc.weight);
        }
    }
}

pub fn fst_rm_states(fst: &mut Fst, mask: &BitSet) {
    let n = fst.n_states as usize;
    let mut idx: Vec<u32> = vec![0; n];
    let mut shift: u32 = 0;
    let mut new_states: Vec<crate::fst::StateData> = Vec::new();
    for s in 0..n {
        if mask.get(s) {
            shift += 1;
            // idx[s] is unused for removed states
        } else {
            idx[s] = shift;
            new_states.push(fst.states[s].clone());
        }
    }
    fst.states = new_states;
    fst.n_states -= shift;

    // fix arc destinations
    for s in 0..fst.n_states as usize {
        let state = &mut fst.states[s];
        let mut new_arcs: Vec<ArcData> = Vec::new();
        for arc in state.arcs.iter() {
            let dst = arc.state as usize;
            if dst >= n {
                continue;
            }
            if mask.get(dst) {
                continue;
            }
            let mut new_arc = *arc;
            new_arc.state -= idx[dst];
            new_arcs.push(new_arc);
        }
        state.n_arcs = new_arcs.len() as u32;
        state.arcs = new_arcs;
    }
}

pub fn fst_get_finals(fst: &Fst, finals: &mut Queue<u32>) {
    for s in 0..fst.n_states as usize {
        if fst.states[s].final_state {
            finals.enqueue(s as u32);
        }
    }
}

pub fn fst_trim(fst: &mut Fst) {
    let mut finals: Queue<u32> = Queue::new();
    fst_get_finals(fst, &mut finals);
    if finals.len() == 0 {
        fst.empty();
        return;
    }
    if finals.len() > 1 {
        fst_close(fst, &mut finals);
    }
    let n_states = fst.n_states as usize;
    // Forward iter
    let mut marked_fwd = BitSet::new(n_states);
    {
        let mut iter = FstIter::new(fst);
        while iter.next().is_some() {}
        marked_fwd = iter.marked;
    }
    fst_reverse(fst);
    let mut marked_rev = BitSet::new(n_states);
    {
        let mut iter = FstIter::new(fst);
        while iter.next().is_some() {}
        marked_rev = iter.marked;
    }
    fst_reverse(fst);
    // intersect and toggle
    marked_fwd.intersect(&marked_rev);
    marked_fwd.toggle_in_place();
    fst_rm_states(fst, &marked_fwd);
}

// trim.rs original signature has Queue<(ArcData, ArcData)>, but logic uses Queue<state_t>.
// Provide pub helper functions matching the original signatures requested.

#[allow(dead_code)]
pub fn fst_close_compat(_fst: &mut Fst, _finals: &mut Queue<(ArcData, ArcData)>) {
    // unused - the ArcData-based signature in original Rust skeleton was incorrect
}
#[allow(dead_code)]
pub fn fst_get_finals_compat(_fst: &mut Fst, _finals: &mut Queue<(ArcData, ArcData)>) {
    // unused
}
