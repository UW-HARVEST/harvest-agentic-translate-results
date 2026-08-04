use crate::fst::{Fst, ArcData, ISORT, OSORT};
use std::cmp::Ordering;

fn icomp(a: &ArcData, b: &ArcData) -> Ordering {
    a.ilabel.cmp(&b.ilabel)
}

fn ocomp(a: &ArcData, b: &ArcData) -> Ordering {
    a.olabel.cmp(&b.olabel)
}

pub fn fst_arc_sort(fst: &mut Fst, sort_outer: bool) {
    if !sort_outer {
        fst.flags |= ISORT;
        for s in 0..fst.n_states as usize {
            fst.states[s].arcs.sort_by(icomp);
        }
    } else {
        fst.flags |= OSORT;
        for s in 0..fst.n_states as usize {
            fst.states[s].arcs.sort_by(ocomp);
        }
    }
}
