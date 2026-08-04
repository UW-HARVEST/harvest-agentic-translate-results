use crate::fst::{ArcData, Fst};

const ISORT: u8 = 0x01;
const OSORT: u8 = 0x02;

fn icomp(a: &ArcData, b: &ArcData) -> std::cmp::Ordering {
    a.ilabel.cmp(&b.ilabel)
}

fn ocomp(a: &ArcData, b: &ArcData) -> std::cmp::Ordering {
    a.olabel.cmp(&b.olabel)
}

pub fn fst_arc_sort(fst: &mut Fst, sort_outer: bool) {
    for state in &mut fst.states {
        if sort_outer {
            state.arcs.sort_by(ocomp);
            fst.flags |= OSORT;
        } else {
            state.arcs.sort_by(icomp);
            fst.flags |= ISORT;
        }
    }
}
