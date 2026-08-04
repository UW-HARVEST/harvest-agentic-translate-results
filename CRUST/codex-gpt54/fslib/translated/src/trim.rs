use crate::queue::Queue;
use crate::fst::{ArcData, Fst};
use crate::bitset::BitSet;
use crate::iter::FstIter;
use crate::sr::sr_get;
pub fn fst_close(fst: &mut Fst, finals: &mut Queue<(ArcData, ArcData)>) {
    if finals.len() < 2 {
        return;
    }

    let sr = sr_get(fst.sr_type);
    let final_state = fst.add_state();
    fst.set_final(final_state, sr.one());

    while let Some((arc, _)) = finals.dequeue() {
        let s = arc.state as usize;
        let weight = fst.states[s].weight;
        fst.states[s].final_state = false;
        fst.add_arc(s as u32, final_state, 0, 0, weight);
    }
}
pub fn fst_reverse(fst: &mut Fst) {
    let sr = sr_get(fst.sr_type);
    let orig = clone_fst(fst);
    let start_s = fst.start;

    for (s, state) in fst.states.iter_mut().enumerate() {
        state.n_arcs = 0;
        state.arcs.clear();
        if state.final_state {
            fst.start = s as u32;
            state.final_state = false;
        }
    }

    fst.set_final(start_s, sr.one());

    for (s, state) in orig.states.iter().enumerate() {
        for arc in &state.arcs {
            fst.add_arc(arc.state, s as u32, arc.ilabel, arc.olabel, arc.weight);
        }
    }
}
fn clone_fst(fst: &Fst) -> Fst {
    Fst {
        start: fst.start,
        n_states: fst.n_states,
        n_max: fst.n_max,
        sr_type: fst.sr_type,
        flags: fst.flags,
        states: fst.states.iter().map(clone_state).collect(),
    }
}
pub fn fst_rm_states(fst: &mut Fst, mask: &BitSet) {
    let mut idx = vec![0u32; fst.n_states as usize];
    let mut shift = 0u32;

    for s in 0..fst.n_states as usize {
        if mask.get(s) {
            shift += 1;
        } else {
            idx[s] = shift;
            fst.states[s - shift as usize] = clone_state(&fst.states[s]);
        }
    }

    fst.n_states -= shift;
    fst.states.truncate(fst.n_states as usize);

    for state in &mut fst.states {
        let mut new_arcs = Vec::with_capacity(state.arcs.len());
        for arc in &state.arcs {
            let new_state = arc.state - idx[arc.state as usize];
            if new_state < fst.n_states {
                new_arcs.push(ArcData {
                    state: new_state,
                    weight: arc.weight,
                    ilabel: arc.ilabel,
                    olabel: arc.olabel,
                });
            }
        }
        state.arcs = new_arcs;
        state.n_arcs = state.arcs.len() as u32;
        state.n_max = state.n_arcs;
    }
}
pub fn fst_get_finals(fst: &mut Fst, finals: &mut Queue<(ArcData, ArcData)>) {
    for (s, state) in fst.states.iter().enumerate() {
        if state.final_state {
            finals.enqueue((state_marker(s as u32), state_marker(0)));
        }
    }
}
pub fn fst_trim(fst: &mut Fst) {
    let mut finals = Queue::new();
    fst_get_finals(fst, &mut finals);

    if finals.len() == 0 {
        fst.empty();
        return;
    }

    if finals.len() > 1 {
        fst_close(fst, &mut finals);
    }

    let mut iter = FstIter::<u32>::new(fst);
    while iter.next().is_some() {}
    let mut direct_marked = std::mem::replace(&mut iter.marked, BitSet::new(0));
    iter.remove();

    fst_reverse(fst);

    let mut iter_rev = FstIter::<u32>::new(fst);
    while iter_rev.next().is_some() {}
    let reverse_marked = std::mem::replace(&mut iter_rev.marked, BitSet::new(0));
    iter_rev.remove();

    fst_reverse(fst);

    direct_marked.intersect(&reverse_marked);
    let mask = direct_marked.toggle_all();
    fst_rm_states(fst, &mask);
}
fn state_marker(state: u32) -> ArcData {
    ArcData {
        state,
        weight: 0.0,
        ilabel: 0,
        olabel: 0,
    }
}

fn clone_state(state: &crate::fst::StateData) -> crate::fst::StateData {
    crate::fst::StateData {
        n_arcs: state.n_arcs,
        n_max: state.n_max,
        weight: state.weight,
        final_state: state.final_state,
        arcs: state
            .arcs
            .iter()
            .map(|arc| ArcData {
                state: arc.state,
                weight: arc.weight,
                ilabel: arc.ilabel,
                olabel: arc.olabel,
            })
            .collect(),
    }
}
