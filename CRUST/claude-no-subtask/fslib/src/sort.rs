use crate::fst::{Fst, ArcData};
use std::cmp::Ordering;

const ISORT: u8 = 0x01;
const OSORT: u8 = 0x02;

fn icomp(a: &ArcData, b: &ArcData) -> Ordering {
    a.ilabel.cmp(&b.ilabel)
}
fn ocomp(a: &ArcData, b: &ArcData) -> Ordering {
    a.olabel.cmp(&b.olabel)
}
pub fn fst_arc_sort(fst: &mut Fst, sort_outer: bool) {
    let comp: fn(&ArcData, &ArcData) -> Ordering = if !sort_outer {
        fst.flags |= ISORT;
        icomp
    } else {
        fst.flags |= OSORT;
        ocomp
    };
    for state in fst.states.iter_mut() {
        state.arcs.sort_by(comp);
    }
}
