//! Phase B — valid-path differential tests.
//!
//! One test (or one clearly-labelled sub-loop) per row of `CONFIGS.md`.
//! Every call goes through `libloading` into the C `.so` and the Rust `.so`;
//! return values, mutated state, and stdout bytes are all compared.

mod common;

use common::*;
use std::ffi::{c_int, c_void};

/// Everything a scenario observes: return values, state snapshots, null-ness.
#[derive(Debug, PartialEq, Eq, Default)]
struct Trace {
    rets: Vec<i64>,
    snaps: Vec<StateSnapshot>,
}

impl Trace {
    fn ret(&mut self, v: impl Into<i64>) {
        self.rets.push(v.into());
    }
    fn snap(&mut self, s: StateSnapshot) {
        self.snaps.push(s);
    }
}

/// Runs the scenario against both implementations and asserts total equality.
fn run_both(ctx: &str, f: impl Fn(&Impl) -> Trace) {
    let p = pair();
    let (tc, oc) = capture(|| f(&p.c));
    let (tr, or) = capture(|| f(&p.rs));
    if tc.rets != tr.rets {
        let at = tc
            .rets
            .iter()
            .zip(tr.rets.iter())
            .position(|(a, b)| a != b)
            .unwrap_or_else(|| tc.rets.len().min(tr.rets.len()));
        let lo = at.saturating_sub(3);
        let hi_c = (at + 3).min(tc.rets.len());
        let hi_r = (at + 3).min(tr.rets.len());
        panic!(
            "return-value divergence [{ctx}] at index {at}\n  \
             lengths: C={} Rust={}\n  C   {:?}\n  Rust{:?}",
            tc.rets.len(),
            tr.rets.len(),
            &tc.rets[lo..hi_c],
            &tr.rets[lo..hi_r]
        );
    }
    assert_eq!(
        tc.snaps.len(),
        tr.snaps.len(),
        "snapshot count divergence [{ctx}]"
    );
    for (i, (a, b)) in tc.snaps.iter().zip(tr.snaps.iter()).enumerate() {
        assert_snap_eq(&format!("{ctx} snapshot#{i}"), a, b);
    }
    assert_out_eq(ctx, &oc, &or);
}

// ===========================================================================
// Rows 1–5: create_state / destroy_state across capacity shapes
// ===========================================================================

fn create_scenario(caps: &'static [c_int], seed: u64, n: usize, read_buf: bool) -> impl Fn(&Impl) -> Trace {
    move |imp: &Impl| {
        let mut t = Trace::default();
        let mut rng = Rng::new(seed);
        for _ in 0..n {
            let init = rng.interesting_i32();
            for &cap in caps {
                unsafe {
                    let s = (imp.create_state)(init, cap);
                    t.ret(s.is_null() as i64);
                    if !s.is_null() {
                        t.snap(imp.snapshot(s, read_buf));
                        (imp.destroy_state)(s);
                    }
                }
            }
        }
        t
    }
}

#[test]
fn row01_create_state_capacity_128() {
    run_both("row01 cap=128", create_scenario(&[128], 0x1001, 400, true));
}

#[test]
fn row02_create_state_capacity_17_boundary() {
    run_both("row02 cap=17", create_scenario(&[16, 17, 18], 0x1002, 400, true));
}

#[test]
fn row03_create_state_capacity_truncating() {
    run_both(
        "row03 cap=1..16",
        create_scenario(
            &[1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16],
            0x1003,
            200,
            true,
        ),
    );
}

#[test]
fn row04_create_state_capacity_large() {
    run_both(
        "row04 cap large",
        create_scenario(&[4096, 65536, 1 << 20], 0x1004, 60, true),
    );
}

#[test]
fn row05_create_state_initial_bitfields() {
    run_both("row05 initial bitfields", |imp: &Impl| {
        let mut t = Trace::default();
        let mut rng = Rng::new(0x1005);
        for _ in 0..300 {
            let init = rng.interesting_i32();
            unsafe {
                let s = (imp.create_state)(init, 20);
                assert!(!s.is_null());
                let snap = imp.snapshot(s, true);
                // Documented init: flag1=1 flag2=0 flag3=1 counter=0 mode=3
                // status=15 reserved=0 -> raw 0x00007b05
                t.ret(snap.flags as i64);
                t.snap(snap);
                (imp.destroy_state)(s);
            }
        }
        t
    });
}

// ===========================================================================
// Rows 6–11: update_flags
// ===========================================================================

#[test]
fn row06_update_flags_all_low6_patterns() {
    run_both("row06 update_flags 0..63", |imp: &Impl| {
        let mut t = Trace::default();
        for param in 0..64i32 {
            unsafe {
                let s = (imp.create_state)(param, 128);
                (imp.update_flags)(s, param);
                t.snap(imp.snapshot(s, true));
                (imp.destroy_state)(s);
            }
        }
        t
    });
}

#[test]
fn row07_update_flags_random_full_range() {
    run_both("row07 update_flags random", |imp: &Impl| {
        let mut t = Trace::default();
        let mut rng = Rng::new(0x1007);
        for _ in 0..800 {
            let init = rng.interesting_i32();
            let param = rng.interesting_i32();
            unsafe {
                let s = (imp.create_state)(init, 128);
                (imp.update_flags)(s, param);
                t.snap(imp.snapshot(s, true));
                (imp.destroy_state)(s);
            }
        }
        t
    });
}

fn repeated_update(seed: u64, calls: usize, vary: bool, iters: usize) -> impl Fn(&Impl) -> Trace {
    move |imp: &Impl| {
        let mut t = Trace::default();
        let mut rng = Rng::new(seed);
        for _ in 0..iters {
            let init = rng.interesting_i32();
            let base = rng.interesting_i32();
            unsafe {
                let s = (imp.create_state)(init, 128);
                for k in 0..calls {
                    let p = if vary { rng.interesting_i32() } else { base };
                    (imp.update_flags)(s, p);
                    if k % 8 == 0 || k + 1 == calls {
                        t.snap(imp.snapshot(s, true));
                    }
                }
                t.snap(imp.snapshot(s, true));
                (imp.destroy_state)(s);
            }
        }
        t
    }
}

#[test]
fn row08_update_flags_two_calls() {
    run_both("row08 2 calls", repeated_update(0x1008, 2, false, 300));
}

#[test]
fn row09_update_flags_32_calls_counter_wraps() {
    run_both("row09 32 calls", repeated_update(0x1009, 32, false, 80));
}

#[test]
fn row10_update_flags_33_calls_after_wrap() {
    run_both("row10 33 calls", repeated_update(0x100A, 33, false, 80));
}

#[test]
fn row11_update_flags_40_calls_varying_param() {
    run_both("row11 40 varying calls", repeated_update(0x100B, 40, true, 80));
}

// ===========================================================================
// Rows 12–18: process_buffer
// ===========================================================================

fn process_scenario(
    caps: &'static [c_int],
    targets: &'static [u8],
    seed: u64,
    n: usize,
) -> impl Fn(&Impl) -> Trace {
    move |imp: &Impl| {
        let mut t = Trace::default();
        let mut rng = Rng::new(seed);
        for _ in 0..n {
            let init = rng.interesting_i32();
            for &cap in caps {
                unsafe {
                    let s = (imp.create_state)(init, cap);
                    assert!(!s.is_null());
                    for &tgt in targets {
                        t.ret((imp.process_buffer)(s, tgt as i8) as i64);
                    }
                    t.snap(imp.snapshot(s, true));
                    (imp.destroy_state)(s);
                }
            }
        }
        t
    }
}

#[test]
fn row12_process_buffer_digits() {
    run_both(
        "row12 digits",
        process_scenario(&[128], b"0123456789", 0x100C, 300),
    );
}

#[test]
fn row13_process_buffer_colon() {
    run_both("row13 colon", process_scenario(&[128], b":", 0x100D, 400));
}

#[test]
fn row14_process_buffer_literal_chars() {
    run_both(
        "row14 literal chars",
        process_scenario(&[128], b"StaeMod-", 0x100E, 300),
    );
}

#[test]
fn row15_process_buffer_all_256_bytes() {
    run_both("row15 all bytes", |imp: &Impl| {
        let mut t = Trace::default();
        let mut rng = Rng::new(0x100F);
        for _ in 0..40 {
            let init = rng.interesting_i32();
            unsafe {
                let s = (imp.create_state)(init, 128);
                assert!(!s.is_null());
                for b in 0u16..256 {
                    t.ret((imp.process_buffer)(s, b as u8 as i8) as i64);
                }
                t.snap(imp.snapshot(s, true));
                (imp.destroy_state)(s);
            }
        }
        t
    });
}

#[test]
fn row16_process_buffer_truncated_buffers() {
    run_both("row16 short buffers", |imp: &Impl| {
        let mut t = Trace::default();
        let mut rng = Rng::new(0x1010);
        for _ in 0..200 {
            let init = rng.interesting_i32();
            for cap in 1..=16 {
                unsafe {
                    let s = (imp.create_state)(init, cap);
                    assert!(!s.is_null());
                    for _ in 0..4 {
                        let tgt = rng.next_u8();
                        t.ret((imp.process_buffer)(s, tgt as i8) as i64);
                    }
                    // plus every digit and the separator
                    for &tgt in b"0123456789:SM-" {
                        t.ret((imp.process_buffer)(s, tgt as i8) as i64);
                    }
                    t.snap(imp.snapshot(s, true));
                    (imp.destroy_state)(s);
                }
            }
        }
        t
    });
}

#[test]
fn row17_process_buffer_repeated_same_state() {
    run_both("row17 repeated", |imp: &Impl| {
        let mut t = Trace::default();
        let mut rng = Rng::new(0x1011);
        for _ in 0..200 {
            let init = rng.interesting_i32();
            unsafe {
                let s = (imp.create_state)(init, 128);
                let tgt = b"0123456789:"[rng.below(11) as usize];
                for _ in 0..5 {
                    t.ret((imp.process_buffer)(s, tgt as i8) as i64);
                }
                t.snap(imp.snapshot(s, true));
                (imp.destroy_state)(s);
            }
        }
        t
    });
}

#[test]
fn row18_process_buffer_after_confuse_op0() {
    run_both("row18 after op0", |imp: &Impl| {
        let mut t = Trace::default();
        let mut rng = Rng::new(0x1012);
        for _ in 0..200 {
            let init = rng.interesting_i32();
            unsafe {
                let s = (imp.create_state)(init, 128);
                t.ret((imp.confuse_types)(s, 0) as i64);
                for &tgt in b"0123456789:" {
                    t.ret((imp.process_buffer)(s, tgt as i8) as i64);
                }
                t.snap(imp.snapshot(s, true));
                (imp.destroy_state)(s);
            }
        }
        t
    });
}

// ===========================================================================
// Rows 19–25: confuse_types
// ===========================================================================

fn confuse_scenario(ops: &'static [c_int], seed: u64, n: usize) -> impl Fn(&Impl) -> Trace {
    move |imp: &Impl| {
        let mut t = Trace::default();
        let mut rng = Rng::new(seed);
        for _ in 0..n {
            let init = rng.interesting_i32();
            unsafe {
                let s = (imp.create_state)(init, 128);
                assert!(!s.is_null());
                for &op in ops {
                    t.ret((imp.confuse_types)(s, op) as i64);
                    t.snap(imp.snapshot(s, true));
                }
                (imp.destroy_state)(s);
            }
        }
        t
    }
}

#[test]
fn row19_confuse_op0_write_int() {
    run_both("row19 op0", confuse_scenario(&[0], 0x1013, 500));
}

#[test]
fn row20_confuse_op1_read_float() {
    // Full-range i32 reinterpreted as f32: normals, denormals, NaN, +-Inf,
    // and values whose *100 overflows int -> cvttss2si indefinite (INT_MIN).
    run_both("row20 op1", confuse_scenario(&[1], 0x1014, 3000));
}

#[test]
fn row21_confuse_op2_read_uint() {
    run_both("row21 op2", confuse_scenario(&[2], 0x1015, 500));
}

#[test]
fn row22_confuse_op3_read_bytes() {
    run_both("row22 op3", confuse_scenario(&[3], 0x1016, 500));
}

#[test]
fn row23_confuse_op0_then_op1() {
    run_both("row23 op0->op1", confuse_scenario(&[0, 1], 0x1017, 300));
}

#[test]
fn row24_confuse_op0_then_op2_and_op3() {
    run_both("row24 op0->op2", confuse_scenario(&[0, 2], 0x1018, 300));
    run_both("row24 op0->op3", confuse_scenario(&[0, 3], 0x1019, 300));
}

#[test]
fn row25_confuse_full_sequence() {
    run_both(
        "row25 seq 0,1,2,3",
        confuse_scenario(&[0, 1, 2, 3], 0x101A, 400),
    );
    run_both(
        "row25 seq 3,2,1,0",
        confuse_scenario(&[3, 2, 1, 0], 0x101B, 400),
    );
    run_both(
        "row25 seq 1,3,0,2,1",
        confuse_scenario(&[1, 3, 0, 2, 1], 0x101C, 400),
    );
}

// ===========================================================================
// Row 26: the composed pipeline, driven entirely through low-level exports
// ===========================================================================

#[test]
fn row26_composed_pipeline_low_level() {
    run_both("row26 composed", |imp: &Impl| {
        let mut t = Trace::default();
        let mut rng = Rng::new(0x101D);
        for _ in 0..1200 {
            let p1 = rng.interesting_i32();
            let p2 = rng.interesting_i32();
            let p3 = rng.interesting_i32();
            let p4 = rng.interesting_i32();
            unsafe {
                let s = (imp.create_state)(p1, 128);
                if s.is_null() {
                    t.ret(-1i64);
                    continue;
                }
                (imp.update_flags)(s, p2);
                let search = (b'0' as i32).wrapping_add(p3 % 10) as i8;
                let found = (imp.process_buffer)(s, search);
                let cres = (imp.confuse_types)(s, p4 % 4);
                let snap = imp.snapshot(s, true);
                let counter = ((snap.flags >> 3) & 0x1F) as i32;
                let mode = ((snap.flags >> 8) & 0x7) as i32;
                let result = found
                    .wrapping_mul(10)
                    .wrapping_add(cres)
                    .wrapping_add(counter.wrapping_mul(5))
                    .wrapping_add(mode.wrapping_mul(3));
                t.ret(result as i64);
                t.snap(snap);
                (imp.destroy_state)(s);
            }
        }
        t
    });
}

// A longer composed run that also reuses one state across many operations,
// so counter/mode/data carry over between stages.
#[test]
fn row26b_composed_stateful_sequence() {
    run_both("row26b stateful", |imp: &Impl| {
        let mut t = Trace::default();
        let mut rng = Rng::new(0x101E);
        for _ in 0..250 {
            let init = rng.interesting_i32();
            let cap = [1, 5, 8, 16, 17, 20, 64, 128, 4096][rng.below(9) as usize];
            unsafe {
                let s = (imp.create_state)(init, cap);
                assert!(!s.is_null());
                for _ in 0..12 {
                    match rng.below(3) {
                        0 => (imp.update_flags)(s, rng.interesting_i32()),
                        1 => {
                            let tgt = rng.next_u8();
                            t.ret((imp.process_buffer)(s, tgt as i8) as i64);
                        }
                        _ => {
                            let op = (rng.below(7) as i32) - 2; // -2..4, incl. invalid
                            t.ret((imp.confuse_types)(s, op) as i64);
                        }
                    }
                    t.snap(imp.snapshot(s, true));
                }
                (imp.destroy_state)(s);
            }
        }
        t
    });
}

// ===========================================================================
// Rows 27–32: the `confusion` one-shot wrapper
// ===========================================================================

#[test]
fn row27_confusion_random_full_range() {
    run_both("row27 confusion random", |imp: &Impl| {
        let mut t = Trace::default();
        let mut rng = Rng::new(0x101F);
        for _ in 0..2000 {
            let (a, b, c, d) = (
                rng.interesting_i32(),
                rng.interesting_i32(),
                rng.interesting_i32(),
                rng.interesting_i32(),
            );
            unsafe { t.ret((imp.confusion)(a, b, c, d) as i64) };
        }
        t
    });
}

#[test]
fn row28_confusion_p3_x_p4_cross_product() {
    run_both("row28 p3xp4", |imp: &Impl| {
        let mut t = Trace::default();
        let mut rng = Rng::new(0x1020);
        for p3 in 0..10i32 {
            for p4 in 0..4i32 {
                for _ in 0..12 {
                    let p1 = rng.interesting_i32();
                    let p2 = rng.interesting_i32();
                    unsafe { t.ret((imp.confusion)(p1, p2, p3, p4) as i64) };
                }
            }
        }
        t
    });
}

#[test]
fn row29_confusion_p2_all_low6_patterns() {
    run_both("row29 p2 0..63", |imp: &Impl| {
        let mut t = Trace::default();
        let mut rng = Rng::new(0x1021);
        for p2 in 0..64i32 {
            for _ in 0..12 {
                let p1 = rng.interesting_i32();
                let p3 = rng.interesting_i32();
                let p4 = rng.interesting_i32();
                unsafe { t.ret((imp.confusion)(p1, p2, p3, p4) as i64) };
            }
        }
        t
    });
}

#[test]
fn row30_confusion_p1_boundaries() {
    const P1: [i32; 14] = [
        0,
        1,
        -1,
        i32::MIN,
        i32::MAX,
        1078530011,
        0x0000_0001,
        0x0080_0000,
        0x007F_FFFF,
        0x7F7F_FFFF,
        0x7F80_0000u32 as i32,
        0x7FC0_0000u32 as i32,
        0xFF80_0000u32 as i32,
        0x4248_3F00u32 as i32,
    ];
    run_both("row30 p1 boundaries", |imp: &Impl| {
        let mut t = Trace::default();
        let mut rng = Rng::new(0x1022);
        for &p1 in P1.iter() {
            for _ in 0..20 {
                let p2 = rng.interesting_i32();
                let p3 = rng.interesting_i32();
                let p4 = rng.interesting_i32();
                unsafe { t.ret((imp.confusion)(p1, p2, p3, p4) as i64) };
            }
            for p4 in 0..4i32 {
                unsafe { t.ret((imp.confusion)(p1, 0, 0, p4) as i64) };
            }
        }
        t
    });
}

#[test]
fn row31_confusion_p3_p4_boundaries() {
    const P3: [i32; 8] = [i32::MIN, i32::MIN + 1, -10, -1, 0, 9, 10, i32::MAX];
    const P4: [i32; 11] = [i32::MIN, -4, -3, -2, -1, 0, 1, 2, 3, 4, i32::MAX];
    run_both("row31 p3/p4 boundaries", |imp: &Impl| {
        let mut t = Trace::default();
        let mut rng = Rng::new(0x1023);
        for &p3 in P3.iter() {
            for &p4 in P4.iter() {
                for _ in 0..6 {
                    let p1 = rng.interesting_i32();
                    let p2 = rng.interesting_i32();
                    unsafe { t.ret((imp.confusion)(p1, p2, p3, p4) as i64) };
                }
            }
        }
        t
    });
}

#[test]
fn row32_confusion_repeated_no_carryover() {
    run_both("row32 repeated", |imp: &Impl| {
        let mut t = Trace::default();
        for _ in 0..50 {
            unsafe {
                t.ret((imp.confusion)(42, 42, 4, 1) as i64);
                t.ret((imp.confusion)(42, 42, 4, 1) as i64);
                t.ret((imp.confusion)(-7, -7, -7, -7) as i64);
                t.ret((imp.confusion)(-7, -7, -7, -7) as i64);
            }
        }
        t
    });
}

// A raw sanity check that `confusion` matches the value the C source computes
// for the canonical example, i.e. the harness itself is wired up correctly.
#[test]
fn harness_smoke_both_libraries_loaded() {
    let p = pair();
    let (rc, oc) = capture(|| unsafe { (p.c.confusion)(1, 2, 3, 4) });
    let (rr, or) = capture(|| unsafe { (p.rs.confusion)(1, 2, 3, 4) });
    assert_ret_eq("smoke", rc, rr);
    assert_out_eq("smoke", &oc, &or);
    assert!(!oc.is_empty(), "expected the C library to print diagnostics");
    let text = String::from_utf8_lossy(&oc).to_string();
    assert!(text.contains("Debug: param1 = 1"), "unexpected output: {text}");
    assert!(text.contains("Final result:"), "unexpected output: {text}");
}

// Silence the unused-import warning when only some helpers are used.
const _: Option<fn(*mut c_void, c_int)> = None;
