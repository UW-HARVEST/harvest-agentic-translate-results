use crate::fst::{Fst,ArcData};
use std::cmp::Ordering;
fn icomp(a: &ArcData, b: &ArcData) -> std::cmp::Ordering {
    a.ilabel.cmp(&b.ilabel)
}
fn ocomp(a: &ArcData, b: &ArcData) -> std::cmp::Ordering {
    a.olabel.cmp(&b.olabel)
}
pub fn fst_arc_sort(fst: &mut Fst, sort_outer: bool) {
    if !sort_outer {
        fst.flags |= 0x01; // ISORT
        for state in fst.states.iter_mut() {
            state.arcs.sort_by(icomp);
        }
    } else {
        fst.flags |= 0x02; // OSORT
        for state in fst.states.iter_mut() {
            state.arcs.sort_by(ocomp);
        }
    }
}
