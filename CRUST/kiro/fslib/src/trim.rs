use crate::queue::Queue;
use crate::fst::{ArcData, Fst};
use crate::bitset::BitSet;
use crate::sr::sr_get;
use crate::iter::FstIter;

pub fn fst_close(fst: &mut Fst, finals: &mut Queue<(ArcData, ArcData)>) {
    if finals.len() < 2 { return; }
    let sr = sr_get(fst.sr_type);
    let final_s = fst.add_state();
    fst.set_final(final_s, sr.one);
    while let Some((arc_a, _)) = finals.dequeue() {
        let s = arc_a.state;
        let w = fst.states[s as usize].weight;
        fst.states[s as usize].final_state = false;
        fst.add_arc(s, final_s, 0, 0, w);
    }
}

pub fn fst_reverse(fst: &mut Fst) {
    let sr = sr_get(fst.sr_type);
    let orig = fst.clone();
    let start_s = fst.start;
    for s in 0..fst.n_states {
        let state = &mut fst.states[s as usize];
        state.arcs.clear();
        state.n_arcs = 0;
        if state.final_state {
            fst.start = s;
            state.final_state = false;
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
    let mut idx = vec![0u32; n];
    let mut shift: u32 = 0;
    for s in 0..n {
        if mask.get(s) {
            shift += 1;
        } else {
            idx[s] = shift;
            if shift > 0 {
                fst.states[s - shift as usize] = fst.states[s].clone();
            }
        }
    }
    fst.n_states -= shift;
    fst.states.truncate(fst.n_states as usize);
    for s in 0..fst.n_states as usize {
        let state = &mut fst.states[s];
        let mut sh: u32 = 0;
        let n_arcs = state.n_arcs as usize;
        for a in 0..n_arcs {
            state.arcs[a].state -= idx[state.arcs[a].state as usize];
            if state.arcs[a].state >= fst.n_states {
                sh += 1;
            } else if sh > 0 {
                let src = a;
                let dst = a - sh as usize;
                state.arcs[dst] = state.arcs[src].clone();
            }
        }
        state.n_arcs -= sh;
        state.arcs.truncate(state.n_arcs as usize);
    }
}

pub fn fst_get_finals(fst: &mut Fst, finals: &mut Queue<(ArcData, ArcData)>) {
    for s in 0..fst.n_states {
        if fst.states[s as usize].final_state {
            let dummy = ArcData { state: s, weight: 0.0, ilabel: 0, olabel: 0 };
            finals.enqueue((dummy.clone(), dummy));
        }
    }
}

fn iter_collect_marked(fst: &Fst) -> BitSet {
    let mut iter = FstIter::new(fst);
    while iter.next().is_some() {}
    iter.marked
}

pub fn fst_trim(fst: &mut Fst) {
    let mut finals: Queue<(ArcData, ArcData)> = Queue::new();
    fst_get_finals(fst, &mut finals);
    if finals.len() == 0 {
        fst.empty();
    } else {
        if finals.len() > 1 {
            fst_close(fst, &mut finals);
        }
        // Forward traversal
        let mut forward_marked = iter_collect_marked(fst);
        // Reverse and traverse
        fst_reverse(fst);
        let rev_marked = iter_collect_marked(fst);
        // Reverse back
        fst_reverse(fst);
        // Intersect and toggle
        forward_marked.intersect(&rev_marked);
        let toggled = forward_marked.toggle_all();
        fst_rm_states(fst, &toggled);
    }
}
