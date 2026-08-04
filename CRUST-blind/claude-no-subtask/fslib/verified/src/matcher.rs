use crate::fst::ArcData;
use crate::queue::Queue;

pub fn match_unsorted(a: &[ArcData], b: &[ArcData], q: &mut Queue<(ArcData, ArcData)>) {
    crate::fst::match_unsorted(a, b, a.len() as u32, b.len() as u32, q);
}

pub fn match_half_sorted(a: &[ArcData], b: &[ArcData], q: &mut Queue<(ArcData, ArcData)>) {
    crate::fst::match_half_sorted(a, b, a.len() as u32, b.len() as u32, q);
}

pub fn match_half_sorted_rev(a: &[ArcData], b: &[ArcData], q: &mut Queue<(ArcData, ArcData)>) {
    crate::fst::match_half_sorted_rev(a, b, a.len() as u32, b.len() as u32, q);
}

pub fn match_full_sorted(a: &[ArcData], b: &[ArcData], q: &mut Queue<(ArcData, ArcData)>) {
    crate::fst::match_full_sorted(a, b, a.len() as u32, b.len() as u32, q);
}
