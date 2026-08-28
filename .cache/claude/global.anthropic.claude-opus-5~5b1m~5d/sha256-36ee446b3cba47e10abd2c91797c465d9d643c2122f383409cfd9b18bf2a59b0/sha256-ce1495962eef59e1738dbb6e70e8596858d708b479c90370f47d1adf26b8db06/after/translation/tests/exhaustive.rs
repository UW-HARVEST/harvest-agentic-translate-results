//! Exhaustive full-domain sweeps.
//!
//! `encode_quant` has a 2^192 input space, but each individual parameter's whole
//! 2^32 domain can be swept exhaustively while the others are pinned. These
//! tests are `#[ignore]`d because they take minutes; run them with
//! `cargo test --release --test exhaustive -- --ignored --nocapture`.
//!
//! `EXHAUSTIVE_STRIDE` (default 1) can subsample the domain for a quick pass.

mod common;

use common::*;

fn stride() -> u64 {
    std::env::var("EXHAUSTIVE_STRIDE")
        .ok()
        .and_then(|s| s.parse::<u64>().ok())
        .filter(|s| *s > 0)
        .unwrap_or(1)
}

/// Optional cap on how many pinned configurations each sweep uses, so a genuine
/// stride-1 (full 2^32) sweep can be run within a time budget.
fn max_configs() -> usize {
    std::env::var("EXHAUSTIVE_CONFIGS")
        .ok()
        .and_then(|s| s.parse::<usize>().ok())
        .filter(|s| *s > 0)
        .unwrap_or(usize::MAX)
}

/// Pinned configurations covering every `lsbit` branch and both `step` signs,
/// including the overflowing and zero `step` cases.
const PINNED: [(i32, i32, i32, i32, i32); 10] = [
    // (step, pred, tgt, tgt2, lsbit)
    (64, 1000, 1137, 900, 0),
    (64, 1000, 1137, 900, 4),
    (64, 1000, 1137, 900, 1),
    (64, 1000, 1137, 900, 2),
    (0, -5, 5, -5, 0),
    (-255, 0, 12345, -12345, 4),
    (i32::MAX, 0, i32::MIN, i32::MAX, 0),
    (i32::MIN, i32::MAX, i32::MIN, 0, 4),
    (1, i32::MIN, i32::MAX, i32::MIN, -1),
    (7, 100, 100, 100, -4),
];

/// Sweep the entire 2^32 `uni` domain for each pinned configuration.
#[test]
#[ignore = "exhaustive: minutes to run"]
fn exhaustive_all_uni_values() {
    let st = stride();
    let cap = max_configs();
    for (i, &(step, pred, tgt, tgt2, lsbit)) in PINNED.iter().take(cap).enumerate() {
        let mut n: u64 = 0;
        let mut v: i64 = i32::MIN as i64;
        while v <= i32::MAX as i64 {
            let a = Args::new(v as i32, step, pred, tgt, tgt2, lsbit);
            let c = call_c(a);
            let r = call_rust(a);
            if c != r {
                panic!("DIVERGENCE at uni={} config#{i} {a:?}: C={c} Rust={r}", v as i32);
            }
            n += 1;
            v += st as i64;
        }
        eprintln!(
            "config#{i} (step={step}, pred={pred}, tgt={tgt}, tgt2={tgt2}, lsbit={lsbit}): \
             {n} uni values OK"
        );
    }
}

/// Sweep the entire 2^32 `lsbit` domain (the mode "enum") for several `uni`s.
#[test]
#[ignore = "exhaustive: minutes to run"]
fn exhaustive_all_lsbit_values() {
    let st = stride();
    let cap = max_configs();
    for uni in [0i32, 5, 7, 8, 15, -1, i32::MIN, i32::MAX].into_iter().take(cap) {
        let mut n: u64 = 0;
        let mut v: i64 = i32::MIN as i64;
        while v <= i32::MAX as i64 {
            let a = Args::new(uni, 64, 1000, 1137, 900, v as i32);
            let c = call_c(a);
            let r = call_rust(a);
            if c != r {
                panic!("DIVERGENCE at lsbit={} {a:?}: C={c} Rust={r}", v as i32);
            }
            n += 1;
            v += st as i64;
        }
        eprintln!("uni={uni}: {n} lsbit values OK");
    }
}

/// Sweep the entire 2^32 `step` domain for several `uni`/`lsbit` shapes.
#[test]
#[ignore = "exhaustive: minutes to run"]
fn exhaustive_all_step_values() {
    let st = stride();
    let cap = max_configs();
    for (uni, lsbit) in [(5i32, 0i32), (9, 4), (15, 1), (-3, 2), (i32::MAX, 0)].into_iter().take(cap) {
        let mut n: u64 = 0;
        let mut v: i64 = i32::MIN as i64;
        while v <= i32::MAX as i64 {
            let a = Args::new(uni, v as i32, 1000, 1137, 900, lsbit);
            let c = call_c(a);
            let r = call_rust(a);
            if c != r {
                panic!("DIVERGENCE at step={} {a:?}: C={c} Rust={r}", v as i32);
            }
            n += 1;
            v += st as i64;
        }
        eprintln!("uni={uni}, lsbit={lsbit}: {n} step values OK");
    }
}

/// Sweep the entire 2^32 `tgt` domain, and the entire `tgt2` domain.
#[test]
#[ignore = "exhaustive: minutes to run"]
fn exhaustive_all_target_values() {
    let st = stride();
    let cap = max_configs();
    for (uni, lsbit, step) in [(5i32, 0i32, 64i32), (9, 4, -64), (2, 1, i32::MAX)].into_iter().take(cap) {
        let mut n: u64 = 0;
        let mut v: i64 = i32::MIN as i64;
        while v <= i32::MAX as i64 {
            let a = Args::new(uni, step, 1000, v as i32, 900, lsbit);
            let b = Args::new(uni, step, 1000, 1137, v as i32, lsbit);
            for a in [a, b] {
                let c = call_c(a);
                let r = call_rust(a);
                if c != r {
                    panic!("DIVERGENCE {a:?}: C={c} Rust={r}");
                }
            }
            n += 1;
            v += st as i64;
        }
        eprintln!("uni={uni}, lsbit={lsbit}, step={step}: {n} tgt/tgt2 values OK");
    }
}

/// Exhaustive over the joint (uni, lsbit) space restricted to a wide window,
/// which covers every branch interaction densely.
#[test]
#[ignore = "exhaustive: minutes to run"]
fn exhaustive_uni_lsbit_joint_window() {
    let mut n: u64 = 0;
    for uni in -512..=512i32 {
        for lsbit in -512..=512i32 {
            for step in [0i32, 1, 7, 8, 64, -64, i32::MAX, i32::MIN] {
                let a = Args::new(uni, step, 1000, 1137, 900, lsbit);
                let c = call_c(a);
                let r = call_rust(a);
                if c != r {
                    panic!("DIVERGENCE {a:?}: C={c} Rust={r}");
                }
                n += 1;
            }
        }
    }
    eprintln!("joint (uni,lsbit) window: {n} cases OK");
}

// ---------------------------------------------------------------------------
// Precedence-critical coverage: the `diff` expression
//     ((2 * (uni & 7) + 1) * step) / 8
// depends jointly on the multiplier `2*(uni&7)+1` (8 possible odd values) and on
// `step`. A parenthesization/precedence error in the Rust chain
//     2i32.wrapping_mul(uni & 7).wrapping_add(1).wrapping_mul(step) / 8
// would only show up for particular (multiplier, step) pairs, so each multiplier
// gets its own exhaustive sweep over the ENTIRE 2^32 `step` domain. One test per
// multiplier so they run in parallel.
// ---------------------------------------------------------------------------

fn sweep_all_steps_for(uni: i32, lsbit: i32) {
    let st = stride();
    let mut n: u64 = 0;
    let mut v: i64 = i32::MIN as i64;
    while v <= i32::MAX as i64 {
        let a = Args::new(uni, v as i32, 1000, 1137, 900, lsbit);
        let c = call_c(a);
        let r = call_rust(a);
        if c != r {
            panic!("DIVERGENCE {a:?}: C={c} Rust={r}");
        }
        n += 1;
        v += st as i64;
    }
    eprintln!("uni={uni} (mult={}) lsbit={lsbit}: {n} step values OK", 2 * (uni & 7) + 1);
}

macro_rules! mult_sweep {
    ($name:ident, $uni:expr) => {
        #[test]
        #[ignore = "exhaustive: minutes to run"]
        fn $name() {
            // lsbit = 0 keeps uni & 7 exactly as given, so the multiplier is pinned.
            sweep_all_steps_for($uni, 0);
        }
    };
}

// uni & 7 == 0..7 -> multipliers 1, 3, 5, 7, 9, 11, 13, 15. Bit 3 clear, so the
// `diff = -diff` negation is NOT taken here.
mult_sweep!(exh_mult01_uni0, 0);
mult_sweep!(exh_mult03_uni1, 1);
mult_sweep!(exh_mult05_uni2, 2);
mult_sweep!(exh_mult07_uni3, 3);
mult_sweep!(exh_mult09_uni4, 4);
mult_sweep!(exh_mult11_uni5, 5);
mult_sweep!(exh_mult13_uni6, 6);
mult_sweep!(exh_mult15_uni7, 7);

// Same multipliers but with bit 3 SET, so the negation branch runs too.
mult_sweep!(exh_mult01_uni8_neg, 8);
mult_sweep!(exh_mult03_uni9_neg, 9);
mult_sweep!(exh_mult05_uni10_neg, 10);
mult_sweep!(exh_mult07_uni11_neg, 11);
mult_sweep!(exh_mult09_uni12_neg, 12);
mult_sweep!(exh_mult11_uni13_neg, 13);
mult_sweep!(exh_mult13_uni14_neg, 14);
mult_sweep!(exh_mult15_uni15_neg, 15);
