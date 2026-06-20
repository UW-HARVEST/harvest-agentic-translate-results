use crate::fst::{Fst,ArcData};
use std::cmp::Ordering;
fn icomp(a: &ArcData, b: &ArcData) -> std::cmp::Ordering {
    a.ilabel.cmp(&b.ilabel)
}
fn ocomp(a: &ArcData, b: &ArcData) -> std::cmp::Ordering {
    a.olabel.cmp(&b.olabel)
}
pub fn fst_arc_sort(fst: &mut Fst, sort_outer: bool) {
    if sort_outer {
        fst.flags |= 0x02;
        for state in &mut fst.states {
            state.arcs.sort_by(ocomp);
        }
    } else {
        fst.flags |= 0x01;
        for state in &mut fst.states {
            state.arcs.sort_by(icomp);
        }
    }
}
