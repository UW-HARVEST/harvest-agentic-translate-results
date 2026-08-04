use crate::fst::{ArcData, Fst, ISORT, OSORT};
use std::cmp::Ordering;

fn icomp(a: &ArcData, b: &ArcData) -> std::cmp::Ordering {
    a.ilabel.cmp(&b.ilabel)
}
fn ocomp(a: &ArcData, b: &ArcData) -> std::cmp::Ordering {
    a.olabel.cmp(&b.olabel)
}

pub fn fst_arc_sort(fst: &mut Fst, sort_outer: bool) {
    if !sort_outer {
        fst.flags |= ISORT;
        for state in fst.states.iter_mut() {
            state.arcs.sort_by(|a, b| icomp(a, b));
        }
    } else {
        fst.flags |= OSORT;
        for state in fst.states.iter_mut() {
            state.arcs.sort_by(|a, b| ocomp(a, b));
        }
    }
    let _ = Ordering::Equal;
}
