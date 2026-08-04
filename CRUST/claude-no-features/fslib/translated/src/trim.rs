use crate::queue::Queue;
use crate::fst::{ArcData, Fst, EPS};
use crate::bitset::BitSet;
use crate::sr;
use crate::iter::FstIter;

pub fn fst_close(fst: &mut Fst, finals: &mut Queue<u32>) {
    if finals.len() < 2 {
        return;
    }
    let sr_struct = sr::sr_get(fst.sr_type);
    let final_st = fst.add_state();
    fst.set_final(final_st, sr_struct.one);
    while let Some(s) = finals.dequeue() {
        let weight;
        {
            let state = &mut fst.states[s as usize];
            state.final_state = false;
            weight = state.weight;
        }
        fst.add_arc(s, final_st, EPS, EPS, weight);
    }
}

pub fn fst_reverse(fst: &mut Fst) {
    let sr_struct = sr::sr_get(fst.sr_type);
    let mut orig = Fst::new();
    orig.copy(fst);

    let start_s = fst.start;
    for s in 0..fst.n_states {
        let state = &mut fst.states[s as usize];
        state.arcs.clear();
        state.n_arcs = 0;
        if state.final_state {
            fst.start = s;
            // safe to set both inside same iteration; since accessed via index
            fst.states[s as usize].final_state = false;
        }
    }

    fst.set_final(start_s, sr_struct.one);

    for s in 0..orig.n_states {
        let state_arcs: Vec<ArcData> = orig.states[s as usize].arcs.clone();
        for arc in state_arcs {
            fst.add_arc(arc.state, s, arc.ilabel, arc.olabel, arc.weight);
        }
    }
}

pub fn fst_rm_states(fst: &mut Fst, mask: &BitSet) {
    let n = fst.n_states as usize;
    let mut idx: Vec<u32> = vec![0; n];
    let mut shift: u32 = 0;
    for s in 0..n {
        if mask.get(s) {
            shift += 1;
        } else {
            idx[s] = shift;
            // Move state s to s - shift
            if shift > 0 {
                fst.states.swap(s - shift as usize, s);
            }
        }
    }
    let new_n = (fst.n_states as u32 - shift) as usize;
    fst.states.truncate(new_n);
    fst.n_states = new_n as u32;
    fst.n_max = fst.n_states;

    for s in 0..fst.n_states as usize {
        let state = &mut fst.states[s];
        let mut new_arcs: Vec<ArcData> = Vec::new();
        for arc in state.arcs.drain(..) {
            let new_dst = (arc.state as i64) - (idx[arc.state as usize] as i64);
            if new_dst >= 0 && (new_dst as u32) < fst.n_states {
                new_arcs.push(ArcData {
                    state: new_dst as u32,
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

pub fn fst_get_finals(fst: &Fst, finals: &mut Queue<u32>) {
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
    // forward iter
    let mut iter = FstIter::new(fst);
    while iter.next().is_some() {}
    let marked_fwd = iter.marked.toggle_all();
    let _ = marked_fwd; // we'll compute differently below

    // Actually reproduce the C: get marked_fwd & marked_rev, intersect, toggle
    let mut fwd_marked = BitSet::new(fst.n_states as usize);
    {
        let mut iter = FstIter::new(fst);
        while iter.next().is_some() {}
        // copy out
        for w_i in 0..iter.marked.words.len() {
            if w_i < fwd_marked.words.len() {
                fwd_marked.words[w_i] = iter.marked.words[w_i];
            }
        }
    }

    fst_reverse(fst);

    let mut rev_marked = BitSet::new(fst.n_states as usize);
    {
        let mut iter_rev = FstIter::new(fst);
        while iter_rev.next().is_some() {}
        for w_i in 0..iter_rev.marked.words.len() {
            if w_i < rev_marked.words.len() {
                rev_marked.words[w_i] = iter_rev.marked.words[w_i];
            }
        }
    }

    fst_reverse(fst);

    fwd_marked.intersect(&rev_marked);
    let mask = fwd_marked.toggle_all();
    fst_rm_states(fst, &mask);
}
