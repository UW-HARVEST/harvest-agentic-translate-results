//! Meta-test: proves the differential harness is not vacuous.
//!
//! If these fail, every other "passing" test in the suite is suspect.
mod common;
use common::*;

#[test]
fn both_libraries_are_distinct_files_and_load() {
    let i = impls();
    assert!(i.has("ZSTD_compress"), "ZSTD_compress resolvable in both");
    assert!(i.has("ZSTD_versionNumber"));
}

/// The comparator must actually report differences.
#[test]
fn comparator_detects_divergence() {
    let a = [1u8, 2, 3, 4];
    let b = [1u8, 2, 9, 4];
    assert!(std::panic::catch_unwind(|| assert_bytes_eq("x", &a, &b)).is_err());
    assert!(std::panic::catch_unwind(|| assert_bytes_eq("x", &a, &a[..3])).is_err());
    assert!(std::panic::catch_unwind(|| assert_eq_dbg("x", 1u32, 2u32)).is_err());
    // and must accept equal inputs
    assert_bytes_eq("x", &a, &a);
}

/// A missing symbol must be a hard failure, not a silent skip.
#[test]
fn missing_symbol_panics() {
    let i = impls();
    assert!(!i.has("ZSTD_this_symbol_does_not_exist"));
    let r = std::panic::catch_unwind(|| {
        i.pair::<unsafe extern "C" fn()>("ZSTD_this_symbol_does_not_exist");
    });
    assert!(r.is_err(), "pair() must panic on a missing symbol");
}

/// Sanity: compression levels and strategies must actually change the output,
/// otherwise the configuration sweep would be comparing identical no-ops.
#[test]
fn configurations_actually_change_output() {
    let i = impls();
    let (c_comp, _) = i.pair::<unsafe extern "C" fn(*mut u8, usize, *const u8, usize, i32) -> usize>("ZSTD_compress");
    let (bound, _) = i.pair::<unsafe extern "C" fn(usize) -> usize>("ZSTD_compressBound");
    let mut rng = Rng::new(7);
    let src = gen_shape(Shape::SkewedText, 40_000, &mut rng);
    let cap = unsafe { bound(src.len()) };
    let mut seen = std::collections::HashSet::new();
    for lvl in [1i32, 3, 9, 19] {
        let mut b = vec![0u8; cap];
        let n = unsafe { c_comp(b.as_mut_ptr(), cap, src.as_ptr(), src.len(), lvl) };
        seen.insert(b[..n].to_vec());
    }
    assert!(seen.len() >= 3, "levels must produce distinct frames, got {}", seen.len());
}
