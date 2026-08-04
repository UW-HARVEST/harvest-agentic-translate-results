use crate::queue::Queue;
use crate::fst::{ArcData, Fst};
use crate::bitset::BitSet;
use crate::iter::FstIter;
use crate::sr::sr_get;

const EPS: u32 = 0;

pub fn fst_close(fst: &mut Fst, finals: &mut Queue<u32>) {
    if finals.len() < 2 {
        return;
    }
    let sr = sr_get(fst.sr_type);
    let final_s = fst.add_state();
    fst.set_final(final_s, sr.one);
    while let Some(s) = finals.dequeue() {
        let w = fst.states[s as usize].weight;
        fst.states[s as usize].final_state = false;
        fst.add_arc(s, final_s, EPS, EPS, w);
    }
}

pub fn fst_reverse(fst: &mut Fst) {
    let sr = sr_get(fst.sr_type);
    let orig = fst.clone();
    let start_s = fst.start;

    for s in 0..fst.n_states as usize {
        fst.states[s].arcs.clear();
        fst.states[s].n_arcs = 0;
        if fst.states[s].final_state {
            fst.start = s as u32;
            fst.states[s].final_state = false;
        }
    }
    fst.set_final(start_s, sr.one);

    for s in 0..orig.n_states as usize {
        for arc in &orig.states[s].arcs {
            fst.add_arc(arc.state, s as u32, arc.ilabel, arc.olabel, arc.weight);
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
        let mut sh = 0usize;
        let n_arcs = fst.states[s].arcs.len();
        for a in 0..n_arcs {
            let dst = fst.states[s].arcs[a].state;
            let new_dst = dst - idx[dst as usize];
            if new_dst >= fst.n_states {
                sh += 1;
            } else {
                fst.states[s].arcs[a].state = new_dst;
                if sh > 0 {
                    fst.states[s].arcs[a - sh] = fst.states[s].arcs[a].clone();
                }
            }
        }
        fst.states[s].arcs.truncate(n_arcs - sh);
        fst.states[s].n_arcs = fst.states[s].arcs.len() as u32;
    }
}

pub fn fst_get_finals(fst: &mut Fst, finals: &mut Queue<u32>) {
    for s in 0..fst.n_states {
        if fst.states[s as usize].final_state {
            finals.enqueue(s);
        }
    }
}

// Wrapper for the old Queue<(ArcData, ArcData)> signature - we adapt internally
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

    // Forward traversal - use a clone to avoid borrow issues
    let forward_marked = {
        let snapshot = fst.clone();
        let mut iter = FstIter::new(&snapshot);
        while iter.next().is_some() {}
        iter.marked
    };

    // Reverse and traverse
    fst_reverse(fst);
    let rev_marked = {
        let snapshot = fst.clone();
        let mut iter_rev = FstIter::new(&snapshot);
        while iter_rev.next().is_some() {}
        iter_rev.marked
    };

    // Reverse back
    fst_reverse(fst);

    // Intersect forward and reverse reachability
    let mut combined = forward_marked;
    combined.intersect(&rev_marked);
    let toggled = combined.toggle_all();
    fst_rm_states(fst, &toggled);
}
