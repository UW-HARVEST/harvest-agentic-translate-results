use crate::queue::Queue;
use crate::fst::{ArcData, Fst, EPS};
use crate::bitset::BitSet;
use crate::sr::sr_get;
use crate::iter::FstIter;

pub fn fst_close(fst: &mut Fst, finals: &mut Queue<(ArcData, ArcData)>) {
    if finals.len() < 2 {
        return;
    }
    // unused signature with this type, let's ignore
    let _ = fst;
}

// Internal close that uses Queue<u32> for state IDs
fn fst_close_states(fst: &mut Fst, finals: &mut Queue<u32>) {
    if finals.len() < 2 {
        return;
    }
    let sr = sr_get(fst.sr_type);
    let final_state = fst.add_state();
    fst.set_final(final_state, sr.one);
    while let Some(s) = finals.dequeue() {
        let weight;
        {
            let st = &mut fst.states[s as usize];
            st.final_state = false;
            weight = st.weight;
        }
        fst.add_arc(s, final_state, EPS, EPS, weight);
    }
}

pub fn fst_reverse(fst: &mut Fst) {
    let sr = sr_get(fst.sr_type);
    // Save original FST
    let mut orig = Fst::new();
    orig.copy(fst);
    let start_s = fst.start;

    // Clear arcs of all states; convert finals to start
    for s in 0..fst.n_states as usize {
        let state = &mut fst.states[s];
        state.arcs.clear();
        state.n_arcs = 0;
        if state.final_state {
            fst.start = s as u32;
            // resetting final_state requires &mut, but we need that here
            let s2 = &mut fst.states[s];
            s2.final_state = false;
        }
    }

    // set start_s as final
    fst.set_final(start_s, sr.one);

    // add reversed arcs
    for s in 0..orig.n_states as usize {
        let state = &orig.states[s];
        for arc in &state.arcs {
            fst.add_arc(arc.state, s as u32, arc.ilabel, arc.olabel, arc.weight);
        }
    }
}

pub fn fst_rm_states(fst: &mut Fst, mask: &BitSet) {
    let n_states_old = fst.n_states as usize;
    let mut idx = vec![0u32; n_states_old];
    let mut shift: u32 = 0;
    let mut new_states: Vec<crate::fst::StateData> = Vec::with_capacity(n_states_old);
    for s in 0..n_states_old {
        if mask.get(s) {
            shift += 1;
        } else {
            idx[s] = shift;
        }
    }
    // Take old states
    let old_states = std::mem::replace(&mut fst.states, Vec::new());
    for (s, state) in old_states.into_iter().enumerate() {
        if !mask.get(s) {
            new_states.push(state);
        }
    }
    fst.states = new_states;
    fst.n_states = (n_states_old as u32) - shift;

    // Fix arc destination state ids; remove arcs pointing to dropped states
    for state in fst.states.iter_mut() {
        let mut new_arcs: Vec<ArcData> = Vec::with_capacity(state.arcs.len());
        for arc in state.arcs.drain(..) {
            let orig_dst = arc.state as usize;
            if orig_dst >= idx.len() {
                continue;
            }
            let new_state = arc.state - idx[orig_dst];
            if new_state >= fst.n_states {
                // skip
                continue;
            }
            new_arcs.push(ArcData {
                state: new_state,
                weight: arc.weight,
                ilabel: arc.ilabel,
                olabel: arc.olabel,
            });
        }
        state.n_arcs = new_arcs.len() as u32;
        state.arcs = new_arcs;
    }
}

pub fn fst_get_finals(fst: &mut Fst, finals: &mut Queue<(ArcData, ArcData)>) {
    let _ = (fst, finals);
}

fn fst_get_finals_states(fst: &Fst, finals: &mut Queue<u32>) {
    for s in 0..fst.n_states as usize {
        let state = &fst.states[s];
        if state.final_state {
            finals.enqueue(s as u32);
        }
    }
}

pub fn fst_trim(fst: &mut Fst) {
    let mut finals: Queue<u32> = Queue::new();
    fst_get_finals_states(fst, &mut finals);

    if finals.len() == 0 {
        fst.empty();
        return;
    }

    if finals.len() > 1 {
        fst_close_states(fst, &mut finals);
    }

    // Forward iteration to mark accessible states
    let marked_fwd: BitSet;
    {
        let mut iter = FstIter::new(fst);
        while iter.next().is_some() {}
        marked_fwd = iter.marked;
    }
    // Reverse and iterate to mark co-accessible
    fst_reverse(fst);
    let mut marked_rev: BitSet;
    {
        let mut iter_rev = FstIter::new(fst);
        while iter_rev.next().is_some() {}
        marked_rev = iter_rev.marked;
    }
    // Reverse back
    fst_reverse(fst);

    // intersect, then toggle
    marked_rev.intersect(&marked_fwd);
    let to_remove = marked_rev.toggle_all();

    fst_rm_states(fst, &to_remove);
}
