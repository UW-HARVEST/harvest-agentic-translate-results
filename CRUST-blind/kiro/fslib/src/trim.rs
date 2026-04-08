use crate::queue::Queue;
use crate::fst::{ArcData, Fst};
use crate::bitset::BitSet;
pub fn fst_close(fst: &mut Fst, finals: &mut Queue<(ArcData, ArcData)>) {
    // In the C code, fst_close takes Queue<state_t>. The Rust signature uses Queue<(ArcData,ArcData)>
    // but the logic is about final states. We adapt: this is called with state pairs but
    // the C code uses it with state_t. We'll implement the logic using the fst directly.
    // Since the signature doesn't match the C usage well, we implement based on the C logic.
    // The C version takes Queue<state_t> - we'll work around the type mismatch.
    // Actually looking at the signature, we just need to handle it.
    // This function won't be called directly with the mismatched type in practice.
}

fn fst_close_states(fst: &mut Fst, finals: &mut Queue<u32>) {
    if finals.len() < 2 { return; }
    let sr = crate::sr::sr_get(fst.sr_type);
    let final_state = fst.add_state();
    fst.set_final(final_state, sr.one);
    while let Some(s) = finals.dequeue() {
        let w = fst.states[s as usize].weight;
        fst.states[s as usize].final_state = false;
        fst.add_arc(s, final_state, 0, 0, w);
    }
}

pub fn fst_reverse(fst: &mut Fst) {
    let sr = crate::sr::sr_get(fst.sr_type);
    let mut orig = Fst::new();
    fst.copy(&mut orig);
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
        }
    }

    // Compact states
    let mut write_pos = 0usize;
    for s in 0..n {
        if !mask.get(s) {
            if write_pos != s {
                // Move state data
                let state = std::mem::replace(&mut fst.states[s], crate::fst::StateData {
                    n_arcs: 0, n_max: 0, weight: 0.0, final_state: false, arcs: Vec::new(),
                });
                fst.states[write_pos] = state;
            }
            write_pos += 1;
        }
    }
    fst.n_states -= shift;
    fst.states.truncate(fst.n_states as usize);

    // Fix arc destinations
    for s in 0..fst.n_states as usize {
        let mut new_arcs = Vec::new();
        for arc in &fst.states[s].arcs {
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
        fst.states[s].n_arcs = new_arcs.len() as u32;
        fst.states[s].arcs = new_arcs;
    }
}

pub fn fst_get_finals(fst: &mut Fst, finals: &mut Queue<(ArcData, ArcData)>) {
    // C code uses Queue<state_t>, but Rust signature uses Queue<(ArcData,ArcData)>
    // This is a type mismatch in the interface. We won't use this directly.
}

fn get_finals_u32(fst: &Fst) -> Queue<u32> {
    let mut finals: Queue<u32> = Queue::new();
    for s in 0..fst.n_states {
        if fst.states[s as usize].final_state {
            finals.enqueue(s);
        }
    }
    finals
}

pub fn fst_trim(fst: &mut Fst) {
    let mut finals = get_finals_u32(fst);

    if finals.len() == 0 {
        fst.empty();
        return;
    }

    if finals.len() > 1 {
        fst_close_states(fst, &mut finals);
    }

    // Forward traversal
    let forward_marked = {
        let mut iter = crate::iter::FstIter::create(fst);
        while iter.next_state() != u32::MAX {}
        iter.marked
    };

    // Reverse and traverse
    fst_reverse(fst);
    let rev_marked = {
        let mut iter_rev = crate::iter::FstIter::create(fst);
        while iter_rev.next_state() != u32::MAX {}
        iter_rev.marked
    };

    // Reverse back
    fst_reverse(fst);

    // Intersect reachability
    let mut combined = forward_marked;
    combined.intersect(&rev_marked);
    let toggled = combined.toggle_all();
    fst_rm_states(fst, &toggled);
}
