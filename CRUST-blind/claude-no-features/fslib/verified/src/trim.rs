use crate::queue::Queue;
use crate::fst::{ArcData, Fst, EPS};
use crate::bitset::BitSet;
use crate::sr;
use crate::iter::FstIter;

// Internal helper that uses Queue<u32> since C code uses state_t.
// Public signatures use Queue<(ArcData, ArcData)> per the project's API; we
// preserve those signatures and provide internal logic via helpers.
fn close_internal(fst: &mut Fst, finals: &mut Queue<u32>) {
    if finals.len() < 2 {
        return;
    }
    let sr_v = sr::sr_get(fst.sr_type);
    let final_state = fst.add_state();
    fst.set_final(final_state, sr_v.one);
    while let Some(s) = finals.dequeue() {
        let weight;
        {
            let state = &mut fst.states[s as usize];
            state.final_state = false;
            weight = state.weight;
        }
        fst.add_arc(s, final_state, EPS, EPS, weight);
    }
}

fn get_finals_internal(fst: &Fst, finals: &mut Queue<u32>) {
    for s in 0..fst.n_states as usize {
        if fst.states[s].final_state {
            finals.enqueue(s as u32);
        }
    }
}

pub fn fst_close(_fst: &mut Fst, _finals: &mut Queue<(ArcData, ArcData)>) {
    // Public stub kept for signature compatibility; internal code uses
    // close_internal which operates on a state queue.
}

pub fn fst_reverse(fst: &mut Fst) {
    let sr_v = sr::sr_get(fst.sr_type);
    let mut orig = Fst::new();
    fst.copy_to(&mut orig);
    let start_s = fst.start;
    for s in 0..fst.n_states as usize {
        let state = &mut fst.states[s];
        state.n_arcs = 0;
        state.arcs.clear();
        if state.final_state {
            fst.start = s as u32;
            fst.states[s].final_state = false;
        }
    }
    fst.set_final(start_s, sr_v.one);
    for s in 0..orig.n_states as usize {
        let state = &orig.states[s];
        for a in 0..state.n_arcs as usize {
            let arc = &state.arcs[a];
            fst.add_arc(arc.state, s as u32, arc.ilabel, arc.olabel, arc.weight);
        }
    }
}

pub fn fst_rm_states(fst: &mut Fst, mask: &BitSet) {
    let n = fst.n_states as usize;
    let mut idx: Vec<u32> = vec![0; n];
    let mut shift: u32 = 0;
    let mut new_states: Vec<crate::fst::StateData> = Vec::with_capacity(n);
    for s in 0..n {
        if mask.get(s) {
            shift += 1;
        } else {
            idx[s] = shift;
            new_states.push(fst.states[s].clone());
        }
    }
    fst.states = new_states;
    fst.n_states -= shift;
    for s in 0..fst.n_states as usize {
        let state = &mut fst.states[s];
        let mut new_arcs: Vec<ArcData> = Vec::new();
        for a in 0..state.n_arcs as usize {
            let mut arc = state.arcs[a].clone();
            let dst = arc.state as usize;
            if dst < idx.len() {
                arc.state = arc.state - idx[dst];
                if arc.state < fst.n_states {
                    new_arcs.push(arc);
                }
            }
        }
        state.n_arcs = new_arcs.len() as u32;
        state.arcs = new_arcs;
    }
}

pub fn fst_get_finals(_fst: &mut Fst, _finals: &mut Queue<(ArcData, ArcData)>) {
    // Public stub kept for signature compatibility; internal code uses
    // get_finals_internal which operates on a state queue.
}

pub fn fst_trim(fst: &mut Fst) {
    let mut finals: Queue<u32> = Queue::new();
    get_finals_internal(fst, &mut finals);
    if finals.len() == 0 {
        fst.empty();
        return;
    }
    if finals.len() > 1 {
        close_internal(fst, &mut finals);
    }
    let mut iter_marked: BitSet;
    {
        let mut iter = FstIter::new(fst);
        while iter.next().is_some() {}
        iter_marked = iter.marked;
    }
    fst_reverse(fst);
    let iter_rev_marked: BitSet;
    {
        let mut iter_rev = FstIter::new(fst);
        while iter_rev.next().is_some() {}
        iter_rev_marked = iter_rev.marked;
    }
    fst_reverse(fst);
    iter_marked.intersect(&iter_rev_marked);
    iter_marked.toggle_all_in_place();
    fst_rm_states(fst, &iter_marked);
}
