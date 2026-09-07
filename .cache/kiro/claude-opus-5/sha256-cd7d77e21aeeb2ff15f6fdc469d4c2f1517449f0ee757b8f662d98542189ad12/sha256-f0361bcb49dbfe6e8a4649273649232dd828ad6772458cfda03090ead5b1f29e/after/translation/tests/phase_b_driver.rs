//! Phase B — CONFIGS.md rows 83-86: the `hm_geti` driver.
//!
//! `hm_geti` is self-checking: every one of its 11 `STBDS_ASSERT`s aborts the
//! process on a behavioural mismatch. It is therefore run in a forked child so
//! the C and the Rust termination status can be compared directly, and then
//! again in-process so that the *global* side effect (how far the
//! `stbds_hash_seed` LCG advanced, i.e. how many hash indexes were built) can
//! be read back out of a freshly created table.

mod common;

use common::*;

/// Number of LCG advances is observable through the `seed` field of the next
/// table created, so this reveals whether both libraries built the same number
/// of hash indexes while running `hm_geti`.
fn seed_probe(what: &str) {
    let m = MapPair::from_shmode(16, 8, STBDS_SH_NONE, KeyKind::Binary);
    m.check(what);
    let mut m = m;
    m.free();
}

fn run_both(num: i32, what: &str) {
    let (c, r) = libs();

    let co = in_child(|| unsafe { (c.hm_geti)(num) });
    let ro = in_child(|| unsafe { (r.hm_geti)(num) });
    same(&format!("{what}: hm_geti({num}) child outcome"), co, ro);
    assert_eq!(
        co,
        Outcome::Exited(0),
        "{what}: C hm_geti({num}) did not exit cleanly"
    );

    // in-process, then compare the resulting global seed state
    unsafe {
        (c.hm_geti)(num);
        (r.hm_geti)(num);
    }
    seed_probe(&format!("{what}: hm_geti({num}) seed advance"));
}

/// row 83: `num <= 0` — every loop body is skipped.
#[test]
fn cfg_83_hm_geti_non_positive() {
    let _g = lock();
    for num in [0i32, -1, -2, -1000, i32::MIN, i32::MIN + 1] {
        reset_seeds(DEFAULT_SEED);
        run_both(num, "row83");
    }
}

/// row 84: the small sizes, which straddle every growth and delete boundary.
#[test]
fn cfg_84_hm_geti_small() {
    let _g = lock();
    for num in 1..=20i32 {
        reset_seeds(DEFAULT_SEED);
        run_both(num, "row84");
    }
}

/// row 85: sizes that force multiple grows, shrinks and tombstone rebuilds.
#[test]
fn cfg_85_hm_geti_large() {
    let _g = lock();
    for num in [31i32, 32, 33, 63, 64, 65, 100, 127, 128, 129, 256, 500, 1000] {
        reset_seeds(DEFAULT_SEED);
        run_both(num, "row85");
    }
}

/// row 86: the same runs under non-default global seeds — different table
/// seeds mean different probe orders, so different code paths inside
/// `stbds_hmput_key` / `stbds_hmdel_key`.
#[test]
fn cfg_86_hm_geti_seeded() {
    let _g = lock();
    let mut rng = Rng::new(0x86);
    let mut seeds: Vec<usize> = vec![0, 1, 2, usize::MAX, usize::MAX - 1];
    for _ in 0..8 {
        seeds.push(rng.next_u64() as usize);
    }
    for &s in &seeds {
        for num in [0i32, 1, 7, 8, 17, 64, 200] {
            reset_seeds(s);
            run_both(num, &format!("row86 seed={s:#x}"));
        }
    }
}
