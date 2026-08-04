use crate::queue::Queue;
use crate::fst::{ArcData, Fst, EPS};
use crate::bitset::BitSet;
use crate::sr::sr_get;
use crate::iter::FstIter;

pub fn fst_close(fst: &mut Fst, finals: &mut Queue<(ArcData, ArcData)>) {
    // The original C version stores final state ids inside the finals queue.
    // To preserve API compatibility our Rust queue holds (ArcData, ArcData)
    // tuples — we encode each final state id by stuffing it into the
    // `state` field of the first arc.
    if finals.len() < 2 {
        return;
    }
    let sr = sr_get(fst.sr_type);
    let final_state = fst.add_state();
    fst.set_final(final_state, sr.one);
    while let Some((s_arc, _)) = finals.dequeue() {
        let s = s_arc.state;
        let weight = fst.states[s as usize].weight;
        fst.states[s as usize].final_state = false;
        fst.add_arc(s, final_state, EPS, EPS, weight);
    }
}

pub fn fst_reverse(fst: &mut Fst) {
    let sr = sr_get(fst.sr_type);
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
        let arcs = orig.states[s].arcs.clone();
        for arc in arcs {
            fst.add_arc(arc.state, s as u32, arc.ilabel, arc.olabel, arc.weight);
        }
    }
    orig.remove();
}

pub fn fst_rm_states(fst: &mut Fst, mask: &BitSet) {
    let n = fst.n_states as usize;
    let mut idx: Vec<u32> = vec![0; n];
    let mut shift: u32 = 0;
    let mut new_states: Vec<crate::fst::StateData> = Vec::with_capacity(n);
    for s in 0..n {
        if mask.get(s) {
            shift += 1;
        } else {
            idx[s] = shift;
            // move state s to new_states
            new_states.push(std::mem::replace(
                &mut fst.states[s],
                crate::fst::StateData {
                    n_arcs: 0,
                    n_max: 0,
                    weight: 0.0,
                    final_state: false,
                    arcs: Vec::new(),
                },
            ));
        }
    }
    fst.states = new_states;
    fst.n_states -= shift;
    // adjust arc destinations & remove invalid ones
    for s in 0..fst.n_states as usize {
        let mut new_arcs: Vec<ArcData> = Vec::new();
        for arc in fst.states[s].arcs.iter() {
            let dest = arc.state;
            if (dest as usize) < idx.len() && !mask.get(dest as usize) {
                let mut new_arc = *arc;
                new_arc.state = dest - idx[dest as usize];
                if new_arc.state < fst.n_states {
                    new_arcs.push(new_arc);
                }
            }
        }
        fst.states[s].n_arcs = new_arcs.len() as u32;
        fst.states[s].arcs = new_arcs;
    }
}

pub fn fst_get_finals(fst: &Fst, finals: &mut Queue<(ArcData, ArcData)>) {
    for s in 0..fst.n_states {
        if fst.states[s as usize].final_state {
            // Encode the state id in the `state` field of an ArcData; only
            // used as a transport medium between fst_get_finals & fst_close.
            let placeholder = ArcData {
                state: s,
                weight: 0.0,
                ilabel: 0,
                olabel: 0,
            };
            finals.enqueue((placeholder, placeholder));
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

    // Forward iteration to find accessible states.
    let marked_forward: BitSet = {
        let mut iter = FstIter::new(fst);
        while iter.next().is_some() {}
        let mut result = BitSet::new(fst.n_states as usize);
        for s in 0..fst.n_states as usize {
            if iter.visited(s as u32) {
                result.set(s);
            }
        }
        result
    };

    fst_reverse(fst);

    let marked_reverse: BitSet = {
        let mut iter = FstIter::new(fst);
        while iter.next().is_some() {}
        let mut result = BitSet::new(fst.n_states as usize);
        for s in 0..fst.n_states as usize {
            if iter.visited(s as u32) {
                result.set(s);
            }
        }
        result
    };

    fst_reverse(fst);

    // intersect and toggle
    let n_words = marked_forward.words.len();
    let mut intersected = BitSet::new(((n_words * 32).saturating_sub(1)).max(0));
    intersected.words = vec![0; n_words];
    for i in 0..n_words {
        intersected.words[i] = marked_forward.words[i] & marked_reverse.words[i];
    }
    let toggled = intersected.toggle_all();
    fst_rm_states(fst, &toggled);
}
