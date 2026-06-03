use crate::queue::Queue;
use crate::fst::{ArcData, Fst, EPS};
use crate::bitset::BitSet;
use crate::iter::FstIter;
use crate::sr::sr_get;

pub fn fst_close(fst: &mut Fst, finals: &mut Queue<u32>) {
    if finals.len() < 2 {
        return;
    }
    let sr = sr_get(fst.sr_type);
    let final_state = fst.add_state();
    fst.set_final(final_state, sr.one);
    while let Some(s) = finals.dequeue() {
        let weight = {
            let st = &mut fst.states[s as usize];
            st.final_state = false;
            st.weight
        };
        fst.add_arc(s, final_state, EPS, EPS, weight);
    }
}
pub fn fst_reverse(fst: &mut Fst) {
    let sr = sr_get(fst.sr_type);
    // Save original
    let orig: Fst = fst.clone();
    let start_s = fst.start;

    // Clear arcs and finalness
    for s in 0..fst.n_states as usize {
        let state = &mut fst.states[s];
        state.n_arcs = 0;
        state.arcs.clear();
        if state.final_state {
            fst.start = s as u32;
            state.final_state = false;
        }
    }
    fst.set_final(start_s, sr.one);
    // Add reversed arcs
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
    let mut idx: Vec<u32> = vec![0u32; n];
    let mut shift: u32 = 0;
    let mut new_states: Vec<crate::fst::StateData> = Vec::new();
    for s in 0..n {
        if mask.get(s) {
            shift += 1;
        } else {
            idx[s] = shift;
            new_states.push(fst.states[s].clone());
        }
    }
    fst.n_states -= shift;
    fst.states = new_states;

    // Fix arc destinations in remaining states
    let new_n = fst.n_states;
    for s in 0..new_n as usize {
        let state = &mut fst.states[s];
        let mut sh: u32 = 0;
        let n_arcs = state.n_arcs as usize;
        let mut new_arcs: Vec<ArcData> = Vec::new();
        for a in 0..n_arcs {
            let mut arc = state.arcs[a].clone();
            // Adjust destination
            if (arc.state as usize) >= idx.len() {
                // shouldn't happen, but skip
                sh += 1;
                continue;
            }
            arc.state = arc.state.wrapping_sub(idx[arc.state as usize]);
            if arc.state >= new_n {
                sh += 1;
            } else {
                new_arcs.push(arc);
            }
        }
        state.arcs = new_arcs;
        state.n_arcs -= sh;
    }
}
pub fn fst_get_finals(fst: &mut Fst, finals: &mut Queue<u32>) {
    for s in 0..fst.n_states {
        let state = &fst.states[s as usize];
        if state.final_state {
            finals.enqueue(s);
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
    // forward iteration
    let forward_marked: BitSet = {
        let mut iter = FstIter::<u32>::new(fst);
        while iter.next().is_some() {}
        iter.marked
    };

    fst_reverse(fst);

    let backward_marked: BitSet = {
        let mut iter_rev = FstIter::<u32>::new(fst);
        while iter_rev.next().is_some() {}
        iter_rev.marked
    };

    fst_reverse(fst);

    // Intersect
    let mut both = forward_marked;
    both.intersect(&backward_marked);
    let toggled = both.toggle_all();

    fst_rm_states(fst, &toggled);
}
