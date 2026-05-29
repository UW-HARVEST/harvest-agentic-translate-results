use crate::queue::Queue;
use crate::fst::{ArcData, Fst, State, EPS};
use crate::bitset::BitSet;
use crate::sr::sr_get;
use crate::iter::FstIter;
pub fn fst_close(fst: &mut Fst, finals: &mut Queue<(ArcData, ArcData)>) {
    // The Queue<(ArcData, ArcData)> is given by the public signature; we don't actually use it
    // in the C reference code which uses a Queue<state_t>. Instead, locate finals here directly.
    let _ = finals;
    let mut state_finals: Vec<State> = Vec::new();
    for s in 0..fst.n_states {
        if fst.states[s as usize].final_state {
            state_finals.push(s);
        }
    }
    if state_finals.len() < 2 {
        return;
    }
    let sr = sr_get(fst.sr_type);
    let final_s = fst.add_state();
    fst.set_final(final_s, sr.one);
    for s in state_finals {
        let weight = {
            let st = &mut fst.states[s as usize];
            st.final_state = false;
            st.weight
        };
        fst.add_arc(s, final_s, EPS, EPS, weight);
    }
}
pub fn fst_reverse(fst: &mut Fst) {
    let sr = sr_get(fst.sr_type);
    let orig_states = fst.states.clone();
    let start_s = fst.start;
    let mut new_start: Option<State> = None;
    for (i, state) in fst.states.iter_mut().enumerate() {
        state.n_arcs = 0;
        state.arcs.clear();
        if state.final_state {
            new_start = Some(i as State);
            state.final_state = false;
        }
    }
    if let Some(ns) = new_start {
        fst.start = ns;
    }
    fst.set_final(start_s, sr.one);
    for (s, ostate) in orig_states.iter().enumerate() {
        for arc in &ostate.arcs {
            fst.add_arc(arc.state, s as State, arc.ilabel, arc.olabel, arc.weight);
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
    for state in fst.states.iter_mut() {
        let mut new_arcs: Vec<ArcData> = Vec::with_capacity(state.arcs.len());
        for arc in &state.arcs {
            // shift the destination, only if destination is in the kept set
            let dst = arc.state as usize;
            if dst < idx.len() && !mask.get(dst) {
                let mut new_arc = *arc;
                new_arc.state = arc.state - idx[dst];
                new_arcs.push(new_arc);
            }
        }
        state.n_arcs = new_arcs.len() as u32;
        state.arcs = new_arcs;
    }
}
pub fn fst_get_finals(fst: &mut Fst, finals: &mut Queue<(ArcData, ArcData)>) {
    // Workaround: the original signature uses Queue<(ArcData,ArcData)>. We stuff
    // dummy ArcData with state=s into both fields so callers can recover state ids.
    for s in 0..fst.n_states {
        if fst.states[s as usize].final_state {
            let dummy = ArcData { state: s, weight: 0.0, ilabel: 0, olabel: 0 };
            finals.enqueue((dummy, dummy));
        }
    }
}
pub fn fst_trim(fst: &mut Fst) {
    // Count finals
    let n_finals: usize = (0..fst.n_states)
        .filter(|&s| fst.states[s as usize].final_state)
        .count();
    if n_finals == 0 {
        fst.empty();
        return;
    }
    if n_finals > 1 {
        let mut q: Queue<(ArcData, ArcData)> = Queue::new();
        fst_close(fst, &mut q);
    }
    // Forward iteration
    let forward_marked = {
        let mut iter = FstIter::new(fst);
        while iter.next().is_some() {}
        iter.marked
    };
    fst_reverse(fst);
    let reverse_marked = {
        let mut iter_rev = FstIter::new(fst);
        while iter_rev.next().is_some() {}
        iter_rev.marked
    };
    fst_reverse(fst);
    let mut intersect = forward_marked;
    intersect.intersect(&reverse_marked);
    let removed = intersect.toggle_all();
    fst_rm_states(fst, &removed);
}
