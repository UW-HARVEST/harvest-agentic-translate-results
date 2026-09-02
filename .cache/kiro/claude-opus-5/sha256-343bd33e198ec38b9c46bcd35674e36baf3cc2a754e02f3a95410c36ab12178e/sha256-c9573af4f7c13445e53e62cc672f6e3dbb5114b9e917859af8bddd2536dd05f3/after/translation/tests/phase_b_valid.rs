//! Phase B — valid-path differential tests, one test per `CONFIGS.md` row.
//!
//! Every call is made through `dlsym` on both shared objects; nothing is called
//! directly. Randomized rows use a fixed PRNG seed so failures reproduce.

mod common;

use common::*;
use std::ffi::c_void;

const N: usize = 400; // randomized inputs per row
const MODES: [i32; 3] = [0, 1, 2];

// ---------------------------------------------------------------------------
// C1..C7 — the three low-level operation entry points, called directly.
// ---------------------------------------------------------------------------

fn op_random(row: &str, name: &[u8], seed: u64) {
    let libs = Pair::load();
    let (cf, rf) = libs.op(name);
    let n = std::str::from_utf8(name).unwrap();
    let mut rng = Rng::new(seed);
    for _ in 0..N * 8 {
        let v = rng.i32_any();
        let p = rng.i32_any();
        assert_op_eq(&cf, &rf, row, n, v, p, std::ptr::null_mut());
    }
}

fn op_boundaries(row: &str, name: &[u8], values: &[i32]) {
    let libs = Pair::load();
    let (cf, rf) = libs.op(name);
    let n = std::str::from_utf8(name).unwrap();
    // A deliberately non-null, unreadable-looking pointer: the C code casts it
    // to (void) and must never dereference it, so this is a valid input.
    let garbage = 0xdead_beef_usize as *mut c_void;
    for &v in values {
        for &p in &[i32::MIN, -1, 0, 1, i32::MAX] {
            assert_op_eq(&cf, &rf, row, n, v, p, std::ptr::null_mut());
            assert_op_eq(&cf, &rf, row, n, v, p, garbage);
        }
    }
}

#[test]
fn c1_process_value_random_full_range() {
    op_random("C1", b"process_value", 0x1111_1111);
}

#[test]
fn c2_process_value_boundaries() {
    op_boundaries(
        "C2",
        b"process_value",
        &[
            0,
            1,
            -1,
            9,
            10,
            11,
            -10,
            i32::MAX - 10,
            i32::MAX - 9,
            i32::MAX - 1,
            i32::MAX,
            i32::MIN,
            i32::MIN + 1,
            i32::MIN + 10,
        ],
    );
}

#[test]
fn c3_double_value_random_full_range() {
    op_random("C3", b"double_value", 0x2222_2222);
}

#[test]
fn c4_double_value_boundaries() {
    op_boundaries(
        "C4",
        b"double_value",
        &[
            0,
            1,
            -1,
            i32::MAX,
            i32::MAX - 1,
            i32::MAX / 2,
            i32::MAX / 2 + 1,
            i32::MIN,
            i32::MIN + 1,
            i32::MIN / 2,
            i32::MIN / 2 - 1,
            65535,
            -65535,
        ],
    );
}

#[test]
fn c5_triple_value_random_full_range() {
    op_random("C5", b"triple_value", 0x3333_3333);
}

#[test]
fn c6_triple_value_boundaries() {
    op_boundaries(
        "C6",
        b"triple_value",
        &[
            0,
            1,
            -1,
            i32::MAX,
            i32::MAX - 1,
            i32::MAX / 3,
            i32::MAX / 3 + 1,
            i32::MIN,
            i32::MIN + 1,
            i32::MIN / 3,
            i32::MIN / 3 - 1,
            65535,
            -65535,
        ],
    );
}

#[test]
fn c7_all_ops_same_value_unused_param_swept() {
    let libs = Pair::load();
    let mut rng = Rng::new(0x7777_7777);
    let garbage = 0x1234_5678_usize as *mut c_void;
    for _ in 0..N {
        let v = rng.i32_any();
        for name in OP_NAMES {
            let (cf, rf) = libs.op(name);
            let n = std::str::from_utf8(name).unwrap();
            for &p in &[i32::MIN, -7, 0, 7, i32::MAX] {
                let a = assert_op_eq(&cf, &rf, "C7", n, v, p, std::ptr::null_mut());
                let b = assert_op_eq(&cf, &rf, "C7", n, v, p, garbage);
                assert_eq!(a, b, "[C7] {n}: unused_context changed the result");
            }
        }
    }
}

// ---------------------------------------------------------------------------
// C8..C15 — `gotomach` with the empty (iterations == 0) and single-element
// (iterations == 1) input shapes, across every mode branch.
// ---------------------------------------------------------------------------

fn goto_fixed_iterations(row: &str, iterations: i32, mode_kind: ModeKind, seed: u64) {
    let libs = Pair::load();
    let (cf, rf) = libs.gotomach();
    let mut rng = Rng::new(seed);
    for _ in 0..N {
        let mode = mode_kind.pick(&mut rng);
        let s = rng.i32_in(0, 65535);
        let t = rng.i32_any();
        assert_goto_eq(&cf, &rf, row, iterations, s, mode, t);
    }
}

#[derive(Copy, Clone)]
enum ModeKind {
    Fixed(i32),
    Invalid,
}

impl ModeKind {
    fn pick(self, rng: &mut Rng) -> i32 {
        match self {
            ModeKind::Fixed(m) => m,
            ModeKind::Invalid => rng.invalid_mode(),
        }
    }
}

#[test]
fn c8_empty_mode0() {
    goto_fixed_iterations("C8", 0, ModeKind::Fixed(0), 0x0801);
}

#[test]
fn c9_empty_mode1() {
    goto_fixed_iterations("C9", 0, ModeKind::Fixed(1), 0x0901);
}

#[test]
fn c10_empty_mode2() {
    goto_fixed_iterations("C10", 0, ModeKind::Fixed(2), 0x1001);
}

#[test]
fn c11_empty_mode_invalid() {
    goto_fixed_iterations("C11", 0, ModeKind::Invalid, 0x1101);
}

#[test]
fn c12_one_mode0() {
    goto_fixed_iterations("C12", 1, ModeKind::Fixed(0), 0x1201);
}

#[test]
fn c13_one_mode1() {
    goto_fixed_iterations("C13", 1, ModeKind::Fixed(1), 0x1301);
}

#[test]
fn c14_one_mode2() {
    goto_fixed_iterations("C14", 1, ModeKind::Fixed(2), 0x1401);
}

#[test]
fn c15_one_mode_invalid() {
    goto_fixed_iterations("C15", 1, ModeKind::Invalid, 0x1501);
}

// ---------------------------------------------------------------------------
// C16..C25 — the "many" shape crossed with the three distinct threshold
// regimes (append none / append all / append some).
// ---------------------------------------------------------------------------

#[derive(Copy, Clone)]
enum Threshold {
    Min,
    Max,
    Interleaving,
    Any,
}

impl Threshold {
    fn pick(self, rng: &mut Rng) -> i32 {
        match self {
            Threshold::Min => i32::MIN,
            Threshold::Max => i32::MAX,
            // The ops emit values in roughly -2997..=3000 once `current_value`
            // has entered its `% 1000` orbit, so this band makes the
            // `temp_buffer[i] < threshold` predicate flip within a single run.
            Threshold::Interleaving => rng.i32_in(-2000, 4000),
            Threshold::Any => rng.i32_any(),
        }
    }
}

fn goto_many(row: &str, mode_kind: ModeKind, thr: Threshold, seed: u64) {
    let libs = Pair::load();
    let (cf, rf) = libs.gotomach();
    let mut rng = Rng::new(seed);
    for _ in 0..N {
        let mode = mode_kind.pick(&mut rng);
        let it = rng.i32_in(2, 512);
        let s = rng.i32_in(0, 65535);
        let t = thr.pick(&mut rng);
        assert_goto_eq(&cf, &rf, row, it, s, mode, t);
    }
}

#[test]
fn c16_many_mode0_threshold_min() {
    goto_many("C16", ModeKind::Fixed(0), Threshold::Min, 0x1601);
}

#[test]
fn c17_many_mode0_threshold_max() {
    goto_many("C17", ModeKind::Fixed(0), Threshold::Max, 0x1701);
}

#[test]
fn c18_many_mode0_threshold_interleaving() {
    goto_many("C18", ModeKind::Fixed(0), Threshold::Interleaving, 0x1801);
}

#[test]
fn c19_many_mode1_threshold_min() {
    goto_many("C19", ModeKind::Fixed(1), Threshold::Min, 0x1901);
}

#[test]
fn c20_many_mode1_threshold_max() {
    goto_many("C20", ModeKind::Fixed(1), Threshold::Max, 0x2001);
}

#[test]
fn c21_many_mode1_threshold_interleaving() {
    goto_many("C21", ModeKind::Fixed(1), Threshold::Interleaving, 0x2101);
}

#[test]
fn c22_many_mode2_threshold_min() {
    goto_many("C22", ModeKind::Fixed(2), Threshold::Min, 0x2201);
}

#[test]
fn c23_many_mode2_threshold_max() {
    goto_many("C23", ModeKind::Fixed(2), Threshold::Max, 0x2301);
}

#[test]
fn c24_many_mode2_threshold_interleaving() {
    goto_many("C24", ModeKind::Fixed(2), Threshold::Interleaving, 0x2401);
}

#[test]
fn c25_many_mode_invalid_threshold_any() {
    goto_many("C25", ModeKind::Invalid, Threshold::Any, 0x2501);
}

// ---------------------------------------------------------------------------
// C26 — fully randomized valid tuples over the entire accepted domain.
// ---------------------------------------------------------------------------

#[test]
fn c26_fully_randomized_valid_domain() {
    let libs = Pair::load();
    let (cf, rf) = libs.gotomach();
    let mut rng = Rng::new(0x2601);
    for _ in 0..N {
        let mode = MODES[(rng.next_u64() % 3) as usize];
        // Bias towards small counts so this row stays fast, but include the
        // full 0..=65535 range.
        let it = if rng.next_u64() % 8 == 0 {
            rng.i32_in(0, 65535)
        } else {
            rng.i32_in(0, 256)
        };
        let s = rng.i32_in(0, 65535);
        let t = rng.i32_any();
        assert_goto_eq(&cf, &rf, "C26", it, s, mode, t);
    }
}

// ---------------------------------------------------------------------------
// C27..C30 — the capacity ceiling: `state->count >= UINT16_MAX`.
// ---------------------------------------------------------------------------

#[test]
fn c27_max_capacity_threshold_max_triggers_ceiling() {
    let libs = Pair::load();
    let (cf, rf) = libs.gotomach();
    let mut rng = Rng::new(0x2701);
    for &mode in &MODES {
        for _ in 0..24 {
            let s = rng.i32_in(0, 65535);
            assert_goto_eq(&cf, &rf, "C27", 65535, s, mode, i32::MAX);
        }
        // Deterministic seed boundaries too.
        for &s in &[0, 1, 999, 1000, 65535] {
            assert_goto_eq(&cf, &rf, "C27", 65535, s, mode, i32::MAX);
        }
    }
    for _ in 0..24 {
        let s = rng.i32_in(0, 65535);
        assert_goto_eq(&cf, &rf, "C27", 65535, s, rng.invalid_mode(), i32::MAX);
    }
}

#[test]
fn c28_max_capacity_threshold_min() {
    let libs = Pair::load();
    let (cf, rf) = libs.gotomach();
    let mut rng = Rng::new(0x2801);
    for &mode in &MODES {
        for _ in 0..24 {
            let s = rng.i32_in(0, 65535);
            assert_goto_eq(&cf, &rf, "C28", 65535, s, mode, i32::MIN);
        }
    }
}

#[test]
fn c29_max_capacity_threshold_interleaving() {
    let libs = Pair::load();
    let (cf, rf) = libs.gotomach();
    let mut rng = Rng::new(0x2901);
    for &mode in &MODES {
        for _ in 0..24 {
            let s = rng.i32_in(0, 65535);
            let t = rng.i32_in(-2000, 4000);
            assert_goto_eq(&cf, &rf, "C29", 65535, s, mode, t);
        }
    }
}

#[test]
fn c30_one_below_ceiling() {
    let libs = Pair::load();
    let (cf, rf) = libs.gotomach();
    let mut rng = Rng::new(0x3001);
    for &mode in &MODES {
        for &it in &[65533, 65534, 65535] {
            for _ in 0..8 {
                let s = rng.i32_in(0, 65535);
                assert_goto_eq(&cf, &rf, "C30", it, s, mode, i32::MAX);
            }
            for &s in &[0, 1, 65535] {
                assert_goto_eq(&cf, &rf, "C30", it, s, mode, i32::MAX);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// C31 — `seed` boundary sweep: entry points into the `% 1000` orbit.
// ---------------------------------------------------------------------------

#[test]
fn c31_seed_boundary_sweep() {
    let libs = Pair::load();
    let (cf, rf) = libs.gotomach();
    let seeds = [0, 1, 2, 499, 500, 989, 990, 999, 1000, 1001, 65534, 65535];
    let thresholds = [i32::MIN, -1, 0, 1, 500, 1000, 3000, i32::MAX];
    for &mode in &MODES {
        for &s in &seeds {
            for &t in &thresholds {
                for &it in &[0, 1, 2, 3, 64, 257] {
                    assert_goto_eq(&cf, &rf, "C31", it, s, mode, t);
                }
            }
        }
    }
    // Also with the invalid-mode fallback.
    for &s in &seeds {
        for &t in &thresholds {
            assert_goto_eq(&cf, &rf, "C31", 64, s, -1, t);
            assert_goto_eq(&cf, &rf, "C31", 64, s, 3, t);
        }
    }
}

// ---------------------------------------------------------------------------
// C32 — `threshold` boundary sweep, exactly on/around the values the ops emit.
// ---------------------------------------------------------------------------

#[test]
fn c32_threshold_boundary_sweep() {
    let libs = Pair::load();
    let (cf, rf) = libs.gotomach();
    let thresholds = [
        -1, 0, 1, 2, 9, 10, 11, 12, 999, 1000, 1001, 1008, 1009, 1010, 1997, 1998, 1999, 2000,
        2001, 2996, 2997, 2998, 2999, 3000, 3001,
    ];
    for &mode in &MODES {
        for &t in &thresholds {
            for &s in &[0, 1, 2, 500, 999, 1000, 65535] {
                for &it in &[1, 2, 5, 33, 128] {
                    assert_goto_eq(&cf, &rf, "C32", it, s, mode, t);
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// C33 — repeated / interleaved calls: no state may leak across calls.
// ---------------------------------------------------------------------------

#[test]
fn c33_repeated_and_interleaved_calls() {
    let libs = Pair::load();
    let (cf, rf) = libs.gotomach();
    let mut rng = Rng::new(0x3301);

    let mut cases = Vec::new();
    for _ in 0..200 {
        let mode = MODES[(rng.next_u64() % 3) as usize];
        cases.push((
            rng.i32_in(0, 300),
            rng.i32_in(0, 65535),
            mode,
            rng.i32_in(-2000, 4000),
        ));
    }

    // Pass 1: all C calls, then all Rust calls.
    let c_first: Vec<i32> = cases
        .iter()
        .map(|&(i, s, m, t)| unsafe { cf(i, s, m, t) })
        .collect();
    let r_first: Vec<i32> = cases
        .iter()
        .map(|&(i, s, m, t)| unsafe { rf(i, s, m, t) })
        .collect();
    assert_eq!(c_first, r_first, "[C33] batched C run != batched Rust run");

    // Pass 2: interleaved C / Rust / C — catches cross-call state and also
    // proves repeated invocation is idempotent in both libraries.
    for (idx, &(i, s, m, t)) in cases.iter().enumerate() {
        let a = unsafe { cf(i, s, m, t) };
        let b = unsafe { rf(i, s, m, t) };
        let a2 = unsafe { cf(i, s, m, t) };
        assert_eq!(a, b, "[C33] interleaved mismatch at {idx}: {a} vs {b}");
        assert_eq!(a, a2, "[C33] C not idempotent at {idx}");
        assert_eq!(
            a, c_first[idx],
            "[C33] C result drifted from the batched run at {idx}"
        );
    }
}

// ---------------------------------------------------------------------------
// C35 — the low-level entry points must agree with what the composed pipeline
// computes internally: re-derive `gotomach`'s result from the exported ops.
// ---------------------------------------------------------------------------

#[test]
fn c35_low_level_ops_match_composed_pipeline() {
    let libs = Pair::load();
    let (cf, rf) = libs.gotomach();
    let mut rng = Rng::new(0x3501);

    for _ in 0..N {
        let mode = MODES[(rng.next_u64() % 3) as usize];
        let it = rng.i32_in(0, 300);
        let seed = rng.i32_in(0, 65535);
        let threshold = rng.i32_in(-2000, 4000);

        // Re-run the C algorithm using the *exported* op symbols, called one at
        // a time through dlsym, and compare against both libraries' gotomach.
        let name = OP_NAMES[mode as usize];
        let (cop, rop) = libs.op(name);
        let n = std::str::from_utf8(name).unwrap();

        let mut current = seed;
        let mut count: usize = 0;
        let mut sum: i32 = 0;
        let mut i = 0;
        while i < it {
            let produced = assert_op_eq(&cop, &rop, "C35", n, current, 0, std::ptr::null_mut());
            if produced < threshold {
                sum = sum.wrapping_add(produced);
                count += 1;
            }
            current = produced.wrapping_rem(1000);
            if count >= 65535 {
                break;
            }
            i += 1;
        }

        let expected = sum;
        let got = assert_goto_eq(&cf, &rf, "C35", it, seed, mode, threshold);
        assert_eq!(
            expected, got,
            "[C35] composed pipeline disagrees with the low-level ops for \
             (iterations={it}, seed={seed}, mode={mode}, threshold={threshold})"
        );
    }
}
