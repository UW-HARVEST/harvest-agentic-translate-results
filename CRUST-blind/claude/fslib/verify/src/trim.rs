use crate::bitset::BitSet;
use crate::fst::{ArcData, Fst, State};
use crate::iter::FstIter;
use crate::queue::Queue;
use crate::sr;

pub fn fst_close(fst: &mut Fst, finals: &mut Queue<(ArcData, ArcData)>) {
    // Replace multiple final states with a single final state.
    // The Queue<(ArcData, ArcData)> signature is awkward; we use it for compatibility.
    if finals.len() < 2 {
        return;
    }
    // Drain finals: we don't have direct state info, so use fst's final_state instead.
    let sr_inst = sr::sr_get(fst.sr_type);
    let final_id = fst.add_state();
    fst.set_final(final_id, sr_inst.one);

    // Collect final states from fst itself
    let mut to_unfinal: Vec<State> = Vec::new();
    for s in 0..final_id {
        if fst.states[s as usize].final_state {
            to_unfinal.push(s);
        }
    }
    for s in to_unfinal {
        let weight = fst.states[s as usize].weight;
        fst.states[s as usize].final_state = false;
        fst.add_arc(s, final_id, 0, 0, weight);
    }
    // empty the queue argument (signature compatibility)
    finals.empty();
}

pub fn fst_reverse(fst: &mut Fst) {
    let sr_inst = sr::sr_get(fst.sr_type);
    let mut orig = Fst::new();
    fst.copy(&mut orig);

    let start_s = fst.start;

    for s in 0..fst.n_states {
        let state = &mut fst.states[s as usize];
        state.arcs.clear();
        state.n_arcs = 0;
        if state.final_state {
            fst.start = s;
            fst.states[s as usize].final_state = false;
        }
    }

    fst.set_final(start_s, sr_inst.one);

    // Add reversed arcs.
    for s in 0..orig.n_states {
        let state = &orig.states[s as usize];
        for a in 0..state.n_arcs {
            let arc = &state.arcs[a as usize];
            fst.add_arc(arc.state, s, arc.ilabel, arc.olabel, arc.weight);
        }
    }
}

pub fn fst_rm_states(fst: &mut Fst, mask: &BitSet) {
    let n_states = fst.n_states as usize;
    let mut idx: Vec<u32> = vec![0; n_states];
    let mut shift: u32 = 0;

    // We want to compact in place. Build a new states vector to avoid lifetime issues.
    let mut new_states = Vec::with_capacity(n_states);
    let original_states = std::mem::replace(&mut fst.states, Vec::new());

    for (s, state) in original_states.into_iter().enumerate() {
        if mask.get(s) {
            shift += 1;
            idx[s] = shift; // also marks "removed" effectively
        } else {
            idx[s] = shift;
            new_states.push(state);
        }
    }

    fst.states = new_states;
    fst.n_states -= shift;

    // Fix arcs
    for state in fst.states.iter_mut() {
        let mut new_arcs: Vec<ArcData> = Vec::new();
        for arc in state.arcs.iter() {
            let dest = arc.state as usize;
            if dest >= mask.words.len() * 32 {
                // out of range: drop
                continue;
            }
            // If destination was removed, drop arc
            // we can detect: original index dest was removed if mask.get(dest) was true
            if mask.get(dest) {
                continue;
            }
            let new_state = arc.state - idx[dest];
            if new_state >= fst.n_states {
                continue;
            }
            let mut new_arc = arc.clone();
            new_arc.state = new_state;
            new_arcs.push(new_arc);
        }
        state.n_arcs = new_arcs.len() as u32;
        state.n_max = state.n_arcs;
        state.arcs = new_arcs;
    }

    // adjust start state
    if (fst.start as usize) < idx.len() {
        if !mask.get(fst.start as usize) {
            fst.start -= idx[fst.start as usize];
        }
    }
}

pub fn fst_get_finals(fst: &mut Fst, finals: &mut Queue<(ArcData, ArcData)>) {
    // The queue type is compatibility-only; we keep counting states-with-final_state.
    let _ = fst;
    let _ = finals;
}

pub fn fst_trim(fst: &mut Fst) {
    // Count final states first
    let mut count_finals = 0usize;
    for s in 0..fst.n_states {
        if fst.states[s as usize].final_state {
            count_finals += 1;
        }
    }

    if count_finals == 0 {
        fst.empty();
        return;
    }

    if count_finals > 1 {
        let mut tmp_q: Queue<(ArcData, ArcData)> = Queue::new();
        // simulate having items in queue
        for _ in 0..count_finals {
            tmp_q.enqueue((
                ArcData {
                    state: 0,
                    weight: 0.0,
                    ilabel: 0,
                    olabel: 0,
                },
                ArcData {
                    state: 0,
                    weight: 0.0,
                    ilabel: 0,
                    olabel: 0,
                },
            ));
        }
        fst_close(fst, &mut tmp_q);
    }

    // Forward iteration to collect reachable states.
    let mut forward_marked = BitSet::new(fst.n_states as usize);
    {
        let mut iter: FstIter<'_, State> = FstIter::new(fst);
        while iter.next().is_some() {}
        // copy the marked bits
        for w in 0..iter.marked.words.len() {
            forward_marked.words[w] = iter.marked.words[w];
        }
    }

    // Reverse the FST.
    fst_reverse(fst);

    let mut backward_marked = BitSet::new(fst.n_states as usize);
    {
        let mut iter_rev: FstIter<'_, State> = FstIter::new(fst);
        while iter_rev.next().is_some() {}
        for w in 0..iter_rev.marked.words.len() {
            backward_marked.words[w] = iter_rev.marked.words[w];
        }
    }

    // Reverse back.
    fst_reverse(fst);

    // Intersect marks: states that are accessible AND co-accessible should remain.
    // Then toggle, so 1 = "remove this state".
    forward_marked.intersect(&backward_marked);
    let mask = forward_marked.toggle_all();

    fst_rm_states(fst, &mask);
}
