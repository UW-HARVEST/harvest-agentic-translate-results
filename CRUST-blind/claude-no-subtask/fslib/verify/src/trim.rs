use crate::queue::Queue;
use crate::fst::{ArcData, Fst, EPS, State};
use crate::bitset::BitSet;
use crate::sr::sr_get;

pub fn fst_close(_fst: &mut Fst, finals: &mut Queue<(ArcData, ArcData)>) {
    // The C interface expects a queue of state_t. Here finals has been
    // adapted in our higher-level API; we'll handle the actual close logic
    // in fst_trim by directly working on state finals.
    // For backward compatibility, this method is provided.
    if finals.len() < 2 {
        return;
    }
}

pub fn fst_close_states(fst: &mut Fst, finals: &mut Queue<State>) {
    if finals.len() < 2 {
        return;
    }
    let sr = sr_get(fst.sr_type);
    let final_state = fst.add_state();
    fst.set_final(final_state, sr.one);

    while let Some(s) = finals.dequeue() {
        let weight = {
            let state = &mut fst.states[s as usize];
            state.final_state = false;
            state.weight
        };
        fst.add_arc(s, final_state, EPS, EPS, weight);
    }
}

pub fn fst_reverse(fst: &mut Fst) {
    let sr = sr_get(fst.sr_type);
    // Make a copy of the original FST
    let mut orig = Fst::new();
    fst.copy(&mut orig);
    let start_s = fst.start;

    // Clear arcs and convert finals to start
    for s in 0..fst.n_states as usize {
        let state = &mut fst.states[s];
        state.arcs.clear();
        state.n_arcs = 0;
        state.n_max = 0;
        if state.final_state {
            fst.start = s as State;
            fst.states[s].final_state = false;
        }
    }

    // Make original start state final
    fst.set_final(start_s, sr.one);

    // Add reversed arcs
    for s in 0..orig.n_states as usize {
        let state = &orig.states[s];
        for a in 0..state.n_arcs as usize {
            let arc = &state.arcs[a];
            fst.add_arc(arc.state, s as State, arc.ilabel, arc.olabel, arc.weight);
        }
    }
}

pub fn fst_rm_states(fst: &mut Fst, mask: &BitSet) {
    let n = fst.n_states as usize;
    let mut idx: Vec<u32> = vec![0u32; n];
    let mut shift: u32 = 0;

    for s in 0..n {
        if mask.get(s) {
            shift += 1;
        } else {
            idx[s] = shift;
            // Move state down
            let state = std::mem::replace(
                &mut fst.states[s],
                crate::fst::StateData {
                    n_arcs: 0,
                    n_max: 0,
                    weight: 0.0,
                    final_state: false,
                    arcs: Vec::new(),
                },
            );
            let target = s - shift as usize;
            fst.states[target] = state;
        }
    }

    fst.n_states -= shift;
    fst.states.truncate(fst.n_states as usize);

    // Fix arc destination state ids
    for s in 0..fst.n_states as usize {
        let state = &mut fst.states[s];
        let mut sh: u32 = 0;
        let mut new_arcs: Vec<ArcData> = Vec::with_capacity(state.arcs.len());
        for a in 0..state.n_arcs as usize {
            let mut arc = state.arcs[a];
            let dst = arc.state;
            arc.state = arc.state.saturating_sub(idx[dst as usize]);
            // Determine if dst was removed
            let dst_orig = dst as usize;
            if dst_orig < idx.len() && mask.get(dst_orig) {
                sh += 1;
            } else {
                new_arcs.push(arc);
            }
        }
        state.arcs = new_arcs;
        state.n_arcs -= sh;
        state.n_max = state.n_arcs;
    }
}

pub fn fst_get_finals(fst: &mut Fst, _finals: &mut Queue<(ArcData, ArcData)>) {
    // legacy - kept for signature compatibility
    let _ = fst;
}

pub fn fst_get_finals_states(fst: &Fst, finals: &mut Queue<State>) {
    for s in 0..fst.n_states as usize {
        if fst.states[s].final_state {
            finals.enqueue(s as State);
        }
    }
}

pub fn fst_trim(fst: &mut Fst) {
    let mut finals: Queue<State> = Queue::new();
    fst_get_finals_states(fst, &mut finals);

    if finals.len() == 0 {
        fst.empty();
    } else {
        if finals.len() > 1 {
            fst_close_states(fst, &mut finals);
        }

        // Forward iteration to find reachable states
        let n_states = fst.n_states as usize;
        let mut forward_marked = BitSet::new(n_states.max(1));
        {
            let mut iter: crate::iter::FstIter<State> = crate::iter::FstIter::new(fst);
            while iter.next().is_some() {}
            // Take ownership of marked
            let marked = iter.marked;
            for s in 0..n_states {
                if marked.get(s) {
                    forward_marked.set(s);
                }
            }
        }

        // Reverse fst, iterate, then reverse back
        fst_reverse(fst);
        let n_states_rev = fst.n_states as usize;
        let mut rev_marked = BitSet::new(n_states_rev.max(1));
        {
            let mut iter: crate::iter::FstIter<State> = crate::iter::FstIter::new(fst);
            while iter.next().is_some() {}
            let marked = iter.marked;
            for s in 0..n_states_rev {
                if marked.get(s) {
                    rev_marked.set(s);
                }
            }
        }
        fst_reverse(fst);

        // intersect forward_marked with rev_marked
        // Need same size; both should be size of n_states.
        // Intersect manually
        let final_n = fst.n_states as usize;
        let mut intersected = BitSet::new(final_n.max(1));
        for s in 0..final_n {
            if forward_marked.get(s) && rev_marked.get(s) {
                intersected.set(s);
            }
        }
        // Toggle all (states to remove are those NOT in intersection)
        let mut to_remove = BitSet::new(final_n.max(1));
        for s in 0..final_n {
            if !intersected.get(s) {
                to_remove.set(s);
            }
        }

        fst_rm_states(fst, &to_remove);
    }
}
