use crate::fst::ArcData;
use crate::queue::Queue;
use crate::fst::{match_unsorted as fst_match_unsorted, match_half_sorted as fst_match_half_sorted, match_half_sorted_rev as fst_match_half_sorted_rev, match_full_sorted as fst_match_full_sorted};
pub fn match_unsorted(a: &[ArcData], b: &[ArcData], q: &mut Queue<(ArcData, ArcData)>) {
    fst_match_unsorted(a, b, a.len() as u32, b.len() as u32, q);
}
pub fn match_half_sorted(a: &[ArcData], b: &[ArcData], q: &mut Queue<(ArcData, ArcData)>) {
    fst_match_half_sorted(a, b, a.len() as u32, b.len() as u32, q);
}
pub fn match_half_sorted_rev(a: &[ArcData], b: &[ArcData], q: &mut Queue<(ArcData, ArcData)>) {
    fst_match_half_sorted_rev(a, b, a.len() as u32, b.len() as u32, q);
}
pub fn match_full_sorted(a: &[ArcData], b: &[ArcData], q: &mut Queue<(ArcData, ArcData)>) {
    fst_match_full_sorted(a, b, a.len() as u32, b.len() as u32, q);
}
