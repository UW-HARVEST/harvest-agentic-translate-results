use crate::fst::{Fst, ArcData};
use std::cmp::Ordering;

fn icomp(a: &ArcData, b: &ArcData) -> Ordering {
    a.ilabel.cmp(&b.ilabel)
}

fn ocomp(a: &ArcData, b: &ArcData) -> Ordering {
    a.olabel.cmp(&b.olabel)
}

pub const ISORT: u8 = 0x01;
pub const OSORT: u8 = 0x02;

pub fn fst_arc_sort(fst: &mut Fst, sort_outer: bool) {
    let comp: fn(&ArcData, &ArcData) -> Ordering;
    if !sort_outer {
        comp = icomp;
        fst.flags |= ISORT;
    } else {
        comp = ocomp;
        fst.flags |= OSORT;
    }
    for state in fst.states.iter_mut() {
        state.arcs.sort_by(comp);
    }
}
