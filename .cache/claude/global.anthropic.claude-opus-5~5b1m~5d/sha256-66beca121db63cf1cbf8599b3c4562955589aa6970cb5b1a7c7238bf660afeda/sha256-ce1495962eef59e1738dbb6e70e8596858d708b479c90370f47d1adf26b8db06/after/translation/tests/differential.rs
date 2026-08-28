//! Phase B — valid-path differential tests for the *public* ABI.
//!
//! Every call goes through `dlopen` + `dlsym` on both shared objects, so the
//! Rust `#[no_mangle]` export wrapper is part of what is under test.
//!
//! Covers CONFIGS.md rows C1..C15, C36, C37.

mod common;

use common::*;

/// Assert `call_predict(pfcn)` agrees between the two libraries, and report the
/// concrete values on divergence.
fn assert_same(c: &Lib, r: &Lib, pfcn: i32) {
    let cf = c.call_predict();
    let rf = r.call_predict();
    let cv = unsafe { cf(pfcn) };
    let rv = unsafe { rf(pfcn) };
    assert_eq!(
        cv, rv,
        "call_predict({pfcn}): {} returned {cv}, {} returned {rv}",
        c.name, r.name
    );
}

fn sweep(c: &Lib, r: &Lib, values: impl IntoIterator<Item = i32>) {
    for v in values {
        assert_same(c, r, v);
    }
}

// ---------------------------------------------------------------------------
// C1..C12 — one row per in-range `pfcn` selector
// ---------------------------------------------------------------------------

macro_rules! row_pfcn {
    ($name:ident, $pfcn:expr) => {
        #[test]
        fn $name() {
            let (c, r) = open_pair();
            // repeat: the function-pointer identity must be stable
            for _ in 0..64 {
                assert_same(&c, &r, $pfcn);
            }
            // and it must be non-zero for the in-range selectors (sanity that
            // the row is actually exercising the specialised path in C)
            let cf = c.call_predict();
            assert_eq!(unsafe { cf($pfcn) }, 1, "C should select a specialised Pfn");
        }
    };
}

row_pfcn!(cfg_c1_pfcn_0, 0);
row_pfcn!(cfg_c2_pfcn_1, 1);
row_pfcn!(cfg_c3_pfcn_2, 2);
row_pfcn!(cfg_c4_pfcn_3, 3);
row_pfcn!(cfg_c5_pfcn_4, 4);
row_pfcn!(cfg_c6_pfcn_5, 5);
row_pfcn!(cfg_c7_pfcn_6, 6);
row_pfcn!(cfg_c8_pfcn_7, 7);
row_pfcn!(cfg_c9_pfcn_8, 8);
row_pfcn!(cfg_c10_pfcn_9, 9);
row_pfcn!(cfg_c11_pfcn_10, 10);
row_pfcn!(cfg_c12_pfcn_11, 11);

// ---------------------------------------------------------------------------
// C13 — contiguous sweep across both range boundaries
// ---------------------------------------------------------------------------

#[test]
fn cfg_c13_contiguous_sweep_minus8_to_32() {
    let (c, r) = open_pair();
    sweep(&c, &r, -8..=32);
}

// ---------------------------------------------------------------------------
// C14 — randomized property sweep over the whole i32 domain + powers of two
// ---------------------------------------------------------------------------

#[test]
fn cfg_c14_random_and_power_of_two_selectors() {
    let (c, r) = open_pair();

    let mut vals: Vec<i32> = Vec::new();
    for k in 0..31 {
        vals.push(1i32 << k);
        vals.push(-(1i32 << k));
        vals.push((1i32 << k) - 1);
        vals.push(-(1i32 << k) + 1);
    }
    vals.push(i32::MIN);
    vals.push(i32::MAX);

    let mut rng = Rng::new(0x5EED_0001);
    for _ in 0..20_000 {
        vals.push(rng.next_i32());
    }
    // plus a dense band around the interesting boundary
    for v in -32..=64 {
        vals.push(v);
    }

    sweep(&c, &r, vals);
}

// ---------------------------------------------------------------------------
// C15 — statelessness: repeated calls, interleaved order
// ---------------------------------------------------------------------------

#[test]
fn cfg_c15_stateless_repeated_and_interleaved() {
    let (c, r) = open_pair();
    let cf = c.call_predict();
    let rf = r.call_predict();

    for round in 0..1000 {
        let pfcn = (round % 40) - 14; // cycles through in- and out-of-range
        let cv = unsafe { cf(pfcn) };
        let rv = unsafe { rf(pfcn) };
        assert_eq!(cv, rv, "round {round}, pfcn {pfcn}");
    }

    // same value 1000x must never change answer
    for pfcn in [0, 5, 11, 12, -1, i32::MIN] {
        let first_c = unsafe { cf(pfcn) };
        let first_r = unsafe { rf(pfcn) };
        assert_eq!(first_c, first_r);
        for _ in 0..1000 {
            assert_eq!(unsafe { cf(pfcn) }, first_c, "C not stateless at {pfcn}");
            assert_eq!(unsafe { rf(pfcn) }, first_r, "Rust not stateless at {pfcn}");
        }
    }
}

// ---------------------------------------------------------------------------
// C36 / C37 — both Rust artifacts (release: predictors inlined & pointer
// comparisons constant-folded; debug: real distinct functions) must agree with
// the C `-O0` build.
// ---------------------------------------------------------------------------

#[test]
fn cfg_c36_release_artifact_matches_c() {
    let c = Lib::open("C", c_so_path(), false);
    let r = Lib::open("Rust(release)", rust_so_release(), true);
    sweep(&c, &r, -64..=64);
    let mut rng = Rng::new(0x5EED_0036);
    sweep(&c, &r, (0..5000).map(|_| rng.next_i32()));
}

#[test]
fn cfg_c37_debug_artifact_matches_c() {
    let Some(path) = rust_so_debug() else {
        eprintln!("skipping C37: debug .so not built");
        return;
    };
    let c = Lib::open("C", c_so_path(), false);
    let r = Lib::open("Rust(debug)", path, true);
    sweep(&c, &r, -64..=64);
    let mut rng = Rng::new(0x5EED_0037);
    sweep(&c, &r, (0..5000).map(|_| rng.next_i32()));
}

// ---------------------------------------------------------------------------
// Struct-layout guard: the Rust ABI mirror of `btac1c_idxstate` must match the
// C compiler's layout (size=74 align=2), otherwise the deep predictor tests
// would compare garbage.
// ---------------------------------------------------------------------------

#[test]
fn idxstate_layout_matches_c() {
    assert_eq!(std::mem::size_of::<IdxState>(), 74, "sizeof(btac1c_idxstate)");
    assert_eq!(std::mem::align_of::<IdxState>(), 2, "alignof(btac1c_idxstate)");
    let s = IdxState::zeroed();
    let base = &s as *const IdxState as usize;
    assert_eq!(&s.idx as *const _ as usize - base, 0);
    assert_eq!(&s.lpred as *const _ as usize - base, 2);
    assert_eq!(&s.rpred as *const _ as usize - base, 4);
    assert_eq!(&s.tag as *const _ as usize - base, 6);
    assert_eq!(&s.bcfcn as *const _ as usize - base, 7);
    assert_eq!(&s.bsfcn as *const _ as usize - base, 8);
    assert_eq!(&s.usefx as *const _ as usize - base, 9);
    assert_eq!(&s.firfx as *const _ as usize - base, 10);
}
