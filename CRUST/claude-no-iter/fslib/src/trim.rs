use crate::queue::Queue;
use crate::fst::{ArcData, Fst, EPS};
use crate::bitset::BitSet;
use crate::sr::sr_get;
use crate::iter::FstIter;

pub fn fst_close(fst: &mut Fst, finals: &mut Queue<(ArcData, ArcData)>) {
    // The signature uses (ArcData, ArcData) but the C function operates on
    // a queue of state IDs. We approximate by using the queue's length only
    // (semantics: when there are 2+ "finals", merge them via a new final state
    // with epsilon arcs from each original final). The caller passes the count.
    let n = finals.len();
    if n < 2 {
        return;
    }
    let sr = sr_get(fst.sr_type);
    let final_id = fst.add_state();
    fst.set_final(final_id, sr.one);

    // Find all current final states (excluding the new one) and convert.
    let mut originals: Vec<u32> = Vec::new();
    for s in 0..fst.n_states {
        if s == final_id {
            continue;
        }
        if fst.states[s as usize].final_state {
            originals.push(s);
        }
    }
    for s in originals {
        let w = {
            let st = &mut fst.states[s as usize];
            st.final_state = false;
            st.weight
        };
        fst.add_arc(s, final_id, EPS, EPS, w);
    }
    // Drain the input queue to mirror C semantics
    while finals.dequeue().is_some() {}
}

pub fn fst_reverse(fst: &mut Fst) {
    let sr = sr_get(fst.sr_type);
    // Make a snapshot of the original state structure.
    let n = fst.n_states as usize;
    let start_s = fst.start;

    // Save arcs, finals, and weights.
    let mut original_arcs: Vec<Vec<ArcData>> = Vec::with_capacity(n);
    let mut original_finals: Vec<bool> = Vec::with_capacity(n);
    for s in 0..n {
        original_arcs.push(fst.states[s].arcs.clone());
        original_finals.push(fst.states[s].final_state);
    }
    // Clear arcs from each state and convert finals to start.
    let mut new_start: Option<u32> = None;
    for s in 0..n {
        let st = &mut fst.states[s];
        st.n_arcs = 0;
        st.arcs.clear();
        if st.final_state {
            new_start = Some(s as u32);
            st.final_state = false;
        }
    }
    if let Some(ns) = new_start {
        fst.start = ns;
    }
    // Set old start as final
    fst.set_final(start_s, sr.one);

    // Add reversed arcs
    for s in 0..n {
        for arc in original_arcs[s].iter() {
            fst.add_arc(arc.state, s as u32, arc.ilabel, arc.olabel, arc.weight);
        }
    }
}

pub fn fst_rm_states(fst: &mut Fst, mask: &BitSet) {
    let n = fst.n_states as usize;
    let mut idx: Vec<u32> = vec![0; n];
    let mut shift: u32 = 0;
    let mut new_states: Vec<crate::fst::StateData> = Vec::with_capacity(n);

    // Drain existing states into a Vec; we reorder/filter manually.
    let mut taken: Vec<crate::fst::StateData> = Vec::with_capacity(n);
    for st in fst.states.drain(..) {
        taken.push(st);
    }
    // Allocate a slot vector to allow taking elements out of `taken`.
    let mut taken: Vec<Option<crate::fst::StateData>> = taken.into_iter().map(Some).collect();

    for s in 0..n {
        if mask.get(s) {
            shift += 1;
        } else {
            idx[s] = shift;
            new_states.push(taken[s].take().unwrap());
        }
    }
    fst.states = new_states;
    fst.n_states = (n as u32) - shift;

    // Fix arc destinations: remap or drop dangling arcs.
    for s in 0..fst.n_states as usize {
        let mut new_arcs: Vec<ArcData> = Vec::new();
        let arcs = std::mem::take(&mut fst.states[s].arcs);
        for arc in arcs {
            // arc.state was an index into the original state list.
            let orig = arc.state as usize;
            if orig >= n || mask.get(orig) {
                // Dangling: skip
                continue;
            }
            let new_state = arc.state - idx[orig];
            if new_state >= fst.n_states {
                continue;
            }
            new_arcs.push(ArcData {
                state: new_state,
                weight: arc.weight,
                ilabel: arc.ilabel,
                olabel: arc.olabel,
            });
        }
        let st = &mut fst.states[s];
        st.n_arcs = new_arcs.len() as u32;
        st.n_max = st.n_arcs;
        st.arcs = new_arcs;
    }
}

pub fn fst_get_finals(fst: &mut Fst, finals: &mut Queue<(ArcData, ArcData)>) {
    // The Rust signature awkwardly types finals as (ArcData, ArcData). We use
    // the queue purely as a counter — caller checks `.len()`. Mirror this by
    // pushing one stub item per final state.
    let stub_arc = || ArcData { state: 0, weight: 0.0, ilabel: 0, olabel: 0 };
    for s in 0..fst.n_states {
        if fst.states[s as usize].final_state {
            finals.enqueue((stub_arc(), stub_arc()));
        }
    }
}

pub fn fst_trim(fst: &mut Fst) {
    let mut finals: Queue<(ArcData, ArcData)> = Queue::new();
    fst_get_finals(fst, &mut finals);

    if finals.len() == 0 {
        fst.empty();
        return;
    }
    if finals.len() > 1 {
        fst_close(fst, &mut finals);
    }

    // Forward iteration: collect accessible states.
    let n_orig = fst.n_states as usize;
    let mut accessible = BitSet::new(n_orig.max(1));
    {
        let mut iter = FstIter::new(fst);
        while iter.next().is_some() {}
        // Copy marks.
        for i in 0..n_orig {
            if iter.visited(i as u32) {
                accessible.set(i);
            }
        }
    }

    // Reverse, iterate, reverse back.
    fst_reverse(fst);
    let n_after_reverse = fst.n_states as usize;
    let mut coaccessible = BitSet::new(n_after_reverse.max(1));
    {
        let mut iter = FstIter::new(fst);
        while iter.next().is_some() {}
        for i in 0..n_after_reverse {
            if iter.visited(i as u32) {
                coaccessible.set(i);
            }
        }
    }
    fst_reverse(fst);

    // Intersect the two and toggle to get the "remove" mask.
    accessible.intersect(&coaccessible);
    let to_remove = accessible.toggle_all();
    fst_rm_states(fst, &to_remove);
}
