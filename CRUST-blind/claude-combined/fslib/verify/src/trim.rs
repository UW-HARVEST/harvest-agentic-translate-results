use crate::queue::Queue;
use crate::fst::{ArcData, Fst, EPS};
use crate::bitset::BitSet;
use crate::sr::sr_get;
use crate::iter::FstIter;

// The signatures with `Queue<(ArcData, ArcData)>` are kept as defined but they
// don't match C's `Queue<state_t>` semantics. We provide additional helpers
// that operate on `Queue<u32>` (state queue) like the C code does.

#[allow(unused_variables)]
pub fn fst_close(fst: &mut Fst, finals: &mut Queue<(ArcData, ArcData)>) {
    // The signature doesn't match the C version which uses Queue<state_t>.
    // Use fst_close_states for that.
}

pub fn fst_close_states(fst: &mut Fst, finals: &mut Queue<u32>) {
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
    Fst::reverse(fst);
}

pub fn fst_rm_states(fst: &mut Fst, mask: &BitSet) {
    let n = fst.n_states as usize;
    let mut idx: Vec<u32> = vec![0; n];
    let mut shift = 0u32;
    let mut new_states = Vec::new();
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

    for s in 0..fst.n_states as usize {
        let mut new_arcs: Vec<ArcData> = Vec::new();
        let n_arcs = fst.states[s].n_arcs as usize;
        for a in 0..n_arcs {
            let mut arc = fst.states[s].arcs[a];
            let dst = arc.state as usize;
            if dst < idx.len() {
                if mask.get(dst) {
                    continue;
                }
                arc.state = arc.state - idx[dst];
            }
            if (arc.state) < fst.n_states {
                new_arcs.push(arc);
            }
        }
        fst.states[s].n_arcs = new_arcs.len() as u32;
        fst.states[s].arcs = new_arcs;
    }
}

#[allow(unused_variables)]
pub fn fst_get_finals(fst: &mut Fst, finals: &mut Queue<(ArcData, ArcData)>) {
    // signature mismatch; use fst_get_finals_states
}

pub fn fst_get_finals_states(fst: &Fst, finals: &mut Queue<u32>) {
    for s in 0..fst.n_states {
        let state = &fst.states[s as usize];
        if state.final_state {
            finals.enqueue(s);
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

    // Forward iteration
    let forward_marked = {
        let mut iter = FstIter::new(fst);
        while iter.next().is_some() {}
        let mut bs = BitSet::new(fst.n_states as usize);
        for (i, w) in iter.marked.words.iter().enumerate() {
            if i < bs.words.len() {
                bs.words[i] = *w;
            } else {
                bs.words.push(*w);
            }
        }
        bs
    };

    fst_reverse(fst);

    let backward_marked = {
        let mut iter = FstIter::new(fst);
        while iter.next().is_some() {}
        let mut bs = BitSet::new(fst.n_states as usize);
        for (i, w) in iter.marked.words.iter().enumerate() {
            if i < bs.words.len() {
                bs.words[i] = *w;
            } else {
                bs.words.push(*w);
            }
        }
        bs
    };

    fst_reverse(fst);

    let mut a = forward_marked;
    let mut b = backward_marked;
    let max_len = a.words.len().max(b.words.len());
    while a.words.len() < max_len {
        a.words.push(0);
    }
    while b.words.len() < max_len {
        b.words.push(0);
    }
    a.intersect(&b);
    let toggled = a.toggle_all();
    fst_rm_states(fst, &toggled);
}
