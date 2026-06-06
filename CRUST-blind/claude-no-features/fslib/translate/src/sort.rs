use crate::fst::{Fst, ArcData, ISORT, OSORT};
use std::cmp::Ordering;
fn icomp(a: &ArcData, b: &ArcData) -> std::cmp::Ordering {
    a.ilabel.cmp(&b.ilabel)
}
fn ocomp(a: &ArcData, b: &ArcData) -> std::cmp::Ordering {
    a.olabel.cmp(&b.olabel)
}
pub fn fst_arc_sort(fst: &mut Fst, sort_outer: bool) {
    let cmp: fn(&ArcData, &ArcData) -> Ordering = if !sort_outer {
        fst.flags |= ISORT;
        icomp
    } else {
        fst.flags |= OSORT;
        ocomp
    };
    for s in 0..fst.n_states as usize {
        let state = &mut fst.states[s];
        state.arcs.sort_by(cmp);
    }
}
