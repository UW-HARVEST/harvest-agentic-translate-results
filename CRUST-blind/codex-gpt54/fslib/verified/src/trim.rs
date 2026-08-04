use crate::bitset::BitSet;
use crate::fst::{ArcData, Fst};
use crate::queue::Queue;
use crate::sr;

pub fn fst_close(fst: &mut Fst, finals: &mut Queue<(ArcData, ArcData)>) {
    if finals.len() < 2 {
        return;
    }

    let sr = sr::sr_get(fst.sr_type);
    let final_state = fst.add_state();
    fst.set_final(final_state, sr.one());

    while let Some((arc, _)) = finals.dequeue() {
        let s = arc.state;
        let weight = fst.states[s as usize].weight;
        fst.states[s as usize].final_state = false;
        fst.add_arc(s, final_state, 0, 0, weight);
    }
}

pub fn fst_reverse(fst: &mut Fst) {
    fst.reverse();
}

pub fn fst_rm_states(fst: &mut Fst, mask: &BitSet) {
    fst.rm_states(mask);
}

pub fn fst_get_finals(fst: &mut Fst, finals: &mut Queue<(ArcData, ArcData)>) {
    finals.empty();
    for (idx, state) in fst.states.iter().enumerate() {
        if state.final_state {
            finals.enqueue((
                ArcData {
                    state: idx as u32,
                    weight: state.weight,
                    ilabel: 0,
                    olabel: 0,
                },
                ArcData {
                    state: idx as u32,
                    weight: state.weight,
                    ilabel: 0,
                    olabel: 0,
                },
            ));
        }
    }
}

pub fn fst_trim(fst: &mut Fst) {
    fst.trim();
}
