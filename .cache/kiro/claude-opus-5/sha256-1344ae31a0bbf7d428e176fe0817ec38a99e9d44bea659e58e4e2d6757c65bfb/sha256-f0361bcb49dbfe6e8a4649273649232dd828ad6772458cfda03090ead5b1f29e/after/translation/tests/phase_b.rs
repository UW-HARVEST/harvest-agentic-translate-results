//! Phase B — valid-path differential tests.
//!
//! One test (or test group) per row of `CONFIGS.md`. Every scenario is executed
//! against BOTH `.so`s through their exported symbols and compared on:
//!   * every intermediate return value,
//!   * the NULL-ness of the returned `ProcessState*`,
//!   * the `flags` bit-field storage unit, the `data` union word and `capacity`,
//!   * the NUL-terminated `buffer` bytes,
//!   * the exact bytes written to stdout.

mod common;
use common::*;

use std::ffi::c_char;

#[derive(Debug, PartialEq, Eq)]
struct Snapshot {
    rets: Vec<i64>,
    state_null: bool,
    flags: Option<u32>,
    data: Option<u32>,
    capacity: Option<i32>,
    buffer: Option<Vec<u8>>,
    stdout: Vec<u8>,
}

type Script = dyn Fn(&Impl, &mut Vec<i64>) -> *mut ProcessState;

fn exec(im: &Impl, script: &Script) -> Snapshot {
    let mut rets: Vec<i64> = Vec::new();
    let (state, stdout) = capture(|| script(im, &mut rets));

    let (flags, data, capacity, buffer) = if state.is_null() {
        (None, None, None, None)
    } else {
        let s = unsafe { *state };
        (
            Some(s.flags),
            Some(s.data),
            Some(s.capacity),
            unsafe { im.buffer_bytes(state) },
        )
    };
    let snap = Snapshot {
        rets,
        state_null: state.is_null(),
        flags,
        data,
        capacity,
        buffer,
        stdout,
    };
    if !state.is_null() {
        let _ = capture(|| unsafe { (im.destroy_state)(state) });
    }
    snap
}

/// Run one scenario against both libraries and assert full equality.
#[track_caller]
fn diff(label: &str, script: &Script) {
    let p = pair();
    let c = exec(&p.c, script);
    let r = exec(&p.r, script);

    assert_eq!(c.rets, r.rets, "[{label}] return values differ: C={:?} Rust={:?}", c.rets, r.rets);
    assert_eq!(c.state_null, r.state_null, "[{label}] state NULL-ness differs");
    assert_eq!(
        c.flags.map(decode_flags),
        r.flags.map(decode_flags),
        "[{label}] flags bit-fields differ (raw C={:#010x?} Rust={:#010x?})",
        c.flags,
        r.flags
    );
    assert_eq!(c.flags, r.flags, "[{label}] raw flags word differs");
    assert_eq!(c.data, r.data, "[{label}] union data word differs");
    assert_eq!(c.capacity, r.capacity, "[{label}] capacity differs");
    assert_eq!(
        c.buffer, r.buffer,
        "[{label}] buffer differs: C={:?} Rust={:?}",
        c.buffer.as_deref().map(String::from_utf8_lossy),
        r.buffer.as_deref().map(String::from_utf8_lossy)
    );
    assert_eq!(
        c.stdout,
        r.stdout,
        "[{label}] stdout differs:\n  C   = {}\n  Rust= {}",
        show(&c.stdout),
        show(&r.stdout)
    );
}

// ===========================================================================
// Rows 1-6 — create_state / destroy_state across the capacity x initial_val
//            matrix.
// ===========================================================================

fn script_create(initial: i32, cap: i32) -> impl Fn(&Impl, &mut Vec<i64>) -> *mut ProcessState {
    move |im, _rets| unsafe { (im.create_state)(initial, cap) }
}

#[test]
fn row01_create_state_generous_capacity() {
    let mut rng = Rng::new(0xB0001);
    for _ in 0..400 {
        let v = rng.next_i32();
        diff(&format!("row1 create_state({v},128)"), &script_create(v, 128));
    }
}

#[test]
fn row02_create_state_capacity_one() {
    let mut rng = Rng::new(0xB0002);
    for _ in 0..300 {
        let v = rng.next_i32();
        diff(&format!("row2 create_state({v},1)"), &script_create(v, 1));
    }
}

#[test]
fn row03_create_state_truncating_capacity() {
    let mut rng = Rng::new(0xB0003);
    for _ in 0..600 {
        let v = rng.next_i32();
        let cap = rng.range_i32(2, 16);
        diff(&format!("row3 create_state({v},{cap})"), &script_create(v, cap));
    }
}

#[test]
fn row04_create_state_exactly_fitting_capacity() {
    let mut rng = Rng::new(0xB0004);
    let mut vals: Vec<i32> = vec![0, 1, -1, 9, -9, 10, -10, i32::MIN, i32::MAX, 100000, -100000];
    for _ in 0..100 {
        vals.push(rng.next_i32());
    }
    for v in vals {
        // create_state always formats mode 3.
        let text = format!("State:{v}:Mode:3");
        let n = text.len() as i32;
        for cap in [n - 1, n, n + 1, n + 2] {
            if cap < 1 {
                continue;
            }
            diff(&format!("row4 create_state({v},{cap})"), &script_create(v, cap));
        }
    }
}

#[test]
fn row05_create_state_huge_capacity() {
    for v in [0i32, 1, -1, i32::MIN, i32::MAX, 123456789] {
        diff(&format!("row5 create_state({v},1<<20)"), &script_create(v, 1 << 20));
    }
}

#[test]
fn row06_create_state_boundary_matrix() {
    for v in [0i32, 1, -1, i32::MIN, i32::MAX] {
        for cap in [1i32, 2, 7, 8, 15, 16, 17, 128, 4096] {
            diff(&format!("row6 create_state({v},{cap})"), &script_create(v, cap));
        }
    }
}

// ===========================================================================
// Rows 7-10 — update_flags.
// ===========================================================================

#[test]
fn row07_update_flags_random_param() {
    let mut rng = Rng::new(0xB0007);
    for _ in 0..800 {
        let v = rng.next_i32();
        let param = rng.next_i32();
        diff(&format!("row7 update_flags({param})"), &move |im: &Impl, rets: &mut Vec<i64>| unsafe {
            let st = (im.create_state)(v, 128);
            rets.push(st.is_null() as i64);
            (im.update_flags)(st, param);
            st
        });
    }
}

#[test]
fn row08_update_flags_all_low_six_bits() {
    for param in 0..64i32 {
        diff(&format!("row8 update_flags({param})"), &move |im: &Impl, _r: &mut Vec<i64>| unsafe {
            let st = (im.create_state)(7, 128);
            (im.update_flags)(st, param);
            st
        });
    }
}

#[test]
fn row09_update_flags_counter_wraps_at_32() {
    // The 5-bit `counter` field must wrap 31 -> 0 after 32 calls.
    for n in [1usize, 2, 30, 31, 32, 33, 40, 64, 65] {
        diff(&format!("row9 update_flags x{n}"), &move |im: &Impl, rets: &mut Vec<i64>| unsafe {
            let st = (im.create_state)(1234, 128);
            for i in 0..n {
                (im.update_flags)(st, i as i32);
                rets.push((*st).flags as i64);
            }
            st
        });
    }
}

#[test]
fn row10_update_flags_negative_param() {
    let mut rng = Rng::new(0xB0010);
    let mut params: Vec<i32> = vec![-1, -2, -3, -4, -7, -8, -9, -16, -64, i32::MIN, i32::MIN + 1];
    for _ in 0..300 {
        params.push(-(rng.next_u32() as i64 % (i32::MAX as i64 + 1)) as i32);
    }
    for param in params {
        diff(&format!("row10 update_flags({param})"), &move |im: &Impl, _r: &mut Vec<i64>| unsafe {
            let st = (im.create_state)(42, 128);
            (im.update_flags)(st, param);
            st
        });
    }
}

// ===========================================================================
// Rows 11-17 — process_buffer.
// ===========================================================================

fn script_process(
    initial: i32,
    cap: i32,
    target: c_char,
) -> impl Fn(&Impl, &mut Vec<i64>) -> *mut ProcessState {
    move |im, rets| unsafe {
        let st = (im.create_state)(initial, cap);
        rets.push((im.process_buffer)(st, target) as i64);
        st
    }
}

#[test]
fn row11_process_buffer_present_once() {
    // 'S' and 'M' each appear exactly once; ':' appears three times.
    let mut rng = Rng::new(0xB0011);
    for _ in 0..300 {
        let v = rng.next_i32();
        for t in [b'S' as c_char, b'M' as c_char, b'a' as c_char, b'o' as c_char] {
            diff(
                &format!("row11 process_buffer({v},128,{t})"),
                &script_process(v, 128, t),
            );
        }
    }
}

#[test]
fn row12_process_buffer_present_many() {
    let mut vals: Vec<i32> = vec![1111111, 3333, 0, -1111, 1212121, 999999999, -999999999];
    let mut rng = Rng::new(0xB0012);
    for _ in 0..200 {
        vals.push(rng.next_i32());
    }
    for v in vals {
        for t in [b':' as c_char, b'1' as c_char, b'3' as c_char, b'-' as c_char] {
            diff(
                &format!("row12 process_buffer({v},128,{t})"),
                &script_process(v, 128, t),
            );
        }
    }
}

#[test]
fn row13_process_buffer_absent() {
    let mut rng = Rng::new(0xB0013);
    for _ in 0..300 {
        let v = rng.next_i32();
        for t in [b'Z' as c_char, b'~' as c_char, 0x7f as c_char, b'@' as c_char] {
            diff(
                &format!("row13 process_buffer({v},128,{t})"),
                &script_process(v, 128, t),
            );
        }
    }
}

#[test]
fn row14_process_buffer_nul_target() {
    let mut rng = Rng::new(0xB0014);
    for _ in 0..200 {
        let v = rng.next_i32();
        let cap = rng.range_i32(1, 64);
        diff(
            &format!("row14 process_buffer({v},{cap},0)"),
            &script_process(v, cap, 0),
        );
    }
}

#[test]
fn row15_process_buffer_high_bit_target() {
    let mut rng = Rng::new(0xB0015);
    let mut targets: Vec<c_char> = vec![-1, -128, -2, -64, -127];
    for _ in 0..100 {
        targets.push(rng.range_i32(-128, -1) as c_char);
    }
    for t in targets {
        for v in [0i32, 42, -42, i32::MIN, i32::MAX] {
            diff(
                &format!("row15 process_buffer({v},128,{t})"),
                &script_process(v, 128, t),
            );
        }
    }
}

#[test]
fn row16_process_buffer_on_truncated_buffer() {
    let mut rng = Rng::new(0xB0016);
    for _ in 0..600 {
        let v = rng.next_i32();
        let cap = rng.range_i32(1, 20);
        let t = rng.range_i32(0x20, 0x7e) as c_char;
        diff(
            &format!("row16 process_buffer({v},{cap},{t})"),
            &script_process(v, cap, t),
        );
    }
}

#[test]
fn row17_process_buffer_called_twice() {
    let mut rng = Rng::new(0xB0017);
    for _ in 0..200 {
        let v = rng.next_i32();
        let t = rng.range_i32(0x20, 0x7e) as c_char;
        diff(
            &format!("row17 process_buffer x2 ({v},{t})"),
            &move |im: &Impl, rets: &mut Vec<i64>| unsafe {
                let st = (im.create_state)(v, 128);
                rets.push((im.process_buffer)(st, t) as i64);
                rets.push((im.process_buffer)(st, t) as i64);
                st
            },
        );
    }
}

// ===========================================================================
// Rows 18-25 — confuse_types (the type-punning arms).
// ===========================================================================

fn script_confuse(initial: i32, op: i32) -> impl Fn(&Impl, &mut Vec<i64>) -> *mut ProcessState {
    move |im, rets| unsafe {
        let st = (im.create_state)(initial, 128);
        rets.push((im.confuse_types)(st, op) as i64);
        st
    }
}

fn script_confuse_after_zero(
    initial: i32,
    op: i32,
) -> impl Fn(&Impl, &mut Vec<i64>) -> *mut ProcessState {
    move |im, rets| unsafe {
        let st = (im.create_state)(initial, 128);
        rets.push((im.confuse_types)(st, 0) as i64);
        rets.push((im.confuse_types)(st, op) as i64);
        st
    }
}

#[test]
fn row18_confuse_types_op0_fresh() {
    let mut rng = Rng::new(0xB0018);
    for _ in 0..400 {
        let v = rng.next_i32();
        diff(&format!("row18 confuse_types({v},0)"), &script_confuse(v, 0));
    }
}

#[test]
fn row19_confuse_types_op1_fresh_random_bit_patterns() {
    let mut rng = Rng::new(0xB0019);
    for _ in 0..2000 {
        let v = rng.next_i32();
        diff(&format!("row19 confuse_types({v},1)"), &script_confuse(v, 1));
    }
}

#[test]
fn row20_confuse_types_op1_after_op0() {
    for v in [0i32, 1, -1, i32::MIN, i32::MAX, 1078530011] {
        diff(&format!("row20 confuse_types({v},0 then 1)"), &script_confuse_after_zero(v, 1));
    }
}

#[test]
fn row21_confuse_types_op2_fresh() {
    let mut rng = Rng::new(0xB0021);
    for _ in 0..600 {
        let v = rng.next_i32();
        diff(&format!("row21 confuse_types({v},2)"), &script_confuse(v, 2));
    }
}

#[test]
fn row22_confuse_types_op2_after_op0() {
    for v in [0i32, 7, -7, i32::MIN, i32::MAX] {
        diff(&format!("row22 confuse_types({v},0 then 2)"), &script_confuse_after_zero(v, 2));
    }
}

#[test]
fn row23_confuse_types_op3_fresh() {
    let mut rng = Rng::new(0xB0023);
    for _ in 0..600 {
        let v = rng.next_i32();
        diff(&format!("row23 confuse_types({v},3)"), &script_confuse(v, 3));
    }
}

#[test]
fn row24_confuse_types_op3_after_op0() {
    for v in [0i32, 7, -7, i32::MIN, i32::MAX] {
        diff(&format!("row24 confuse_types({v},0 then 3)"), &script_confuse_after_zero(v, 3));
    }
}

#[test]
fn row25_confuse_types_op1_targeted_float_bit_patterns() {
    // Every interesting IEEE-754 binary32 class, plus values whose *100
    // overflows `int` (where gcc's cvttss2si yields the integer-indefinite
    // value INT_MIN rather than a saturating result).
    let mut patterns: Vec<u32> = vec![
        0x0000_0000, // +0
        0x8000_0000, // -0
        0x0000_0001, // smallest denormal
        0x807f_ffff, // largest negative denormal
        0x0080_0000, // smallest normal
        0x3f80_0000, // 1.0
        0xbf80_0000, // -1.0
        0x4048_f5c3, // 3.14
        0x402d_f854, // 2.7182817
        0x7f7f_ffff, // FLT_MAX
        0xff7f_ffff, // -FLT_MAX
        0x7f80_0000, // +inf
        0xff80_0000, // -inf
        0x7fc0_0000, // quiet NaN
        0xffc0_0000, // negative quiet NaN
        0x7f80_0001, // signalling NaN
        0xff80_0001, // negative signalling NaN
        0x4f00_0000, // 2^31 exactly
        0x4eff_ffff, // just below 2^31
        0xcf00_0000, // -2^31 exactly
        0xcf00_0001, // just below -2^31
        0x4b00_0000, // 2^23
        0x4d3c_0000, // ~1.97e8 (*100 overflows)
        0x4b18_9680, // 1e7 (*100 = 1e9, fits)
        0x4c18_9680, // 4e7 (*100 overflows)
        0x3c23_d70a, // 0.01
        0xbc23_d70a, // -0.01
        0x3727_c5ac, // 1e-5
        0x4283_126f, // 65.5238
    ];
    let mut rng = Rng::new(0xB0025);
    // Bias more samples into the exponent range where `*100` straddles INT_MAX.
    for _ in 0..500 {
        let exp = rng.range_i32(120, 160) as u32;
        let sign = (rng.next_u32() & 1) << 31;
        let mant = rng.next_u32() & 0x007f_ffff;
        patterns.push(sign | (exp << 23) | mant);
    }
    for bits in patterns {
        let v = bits as i32;
        diff(&format!("row25 confuse_types({v:#010x} as f32, 1)"), &script_confuse(v, 1));
    }
}

// ===========================================================================
// Rows 26-27 — the full low-level pipeline, driven the way `confusion` does.
// ===========================================================================

#[test]
fn row26_full_low_level_pipeline() {
    let mut rng = Rng::new(0xB0026);
    for _ in 0..1200 {
        let p1 = rng.next_i32();
        let p2 = rng.next_i32();
        let p3 = rng.next_i32();
        let p4 = rng.next_i32();
        let cap = if rng.next_u32() % 4 == 0 {
            rng.range_i32(1, 24)
        } else {
            128
        };
        let nupd = (rng.next_u32() % 5) as usize;
        diff(
            &format!("row26 pipeline({p1},{p2},{p3},{p4},cap={cap},nupd={nupd})"),
            &move |im: &Impl, rets: &mut Vec<i64>| unsafe {
                let st = (im.create_state)(p1, cap);
                if st.is_null() {
                    rets.push(-999);
                    return st;
                }
                for _ in 0..=nupd {
                    (im.update_flags)(st, p2);
                    rets.push((*st).flags as i64);
                }
                let ch = (b'0' as i32).wrapping_add(p3 % 10) as c_char;
                rets.push((im.process_buffer)(st, ch) as i64);
                rets.push((im.confuse_types)(st, p4 % 4) as i64);
                rets.push((*st).flags as i64);
                rets.push((*st).data as i64);
                st
            },
        );
    }
}

#[test]
fn row27_pipeline_reordered() {
    let mut rng = Rng::new(0xB0027);
    for _ in 0..600 {
        let p1 = rng.next_i32();
        let p2 = rng.next_i32();
        let p3 = rng.next_i32();
        let op = rng.range_i32(0, 3);
        diff(
            &format!("row27 reordered({p1},{p2},{p3},{op})"),
            &move |im: &Impl, rets: &mut Vec<i64>| unsafe {
                let st = (im.create_state)(p1, 128);
                rets.push((im.confuse_types)(st, 0) as i64);
                rets.push((im.confuse_types)(st, op) as i64);
                (im.update_flags)(st, p2);
                let ch = (b'0' as i32).wrapping_add(p3 % 10) as c_char;
                rets.push((im.process_buffer)(st, ch) as i64);
                rets.push((im.confuse_types)(st, op) as i64);
                st
            },
        );
    }
}

// ===========================================================================
// Rows 28-32 — the `confusion` one-shot entry point.
// ===========================================================================

fn diff_confusion(label: &str, a: i32, b: i32, c: i32, d: i32) {
    let p = pair();
    let (rc, oc) = capture(|| unsafe { (p.c.confusion)(a, b, c, d) });
    let (rr, or) = capture(|| unsafe { (p.r.confusion)(a, b, c, d) });
    assert_eq!(rc, rr, "[{label}] confusion({a},{b},{c},{d}) return differs");
    assert_eq!(
        oc,
        or,
        "[{label}] confusion({a},{b},{c},{d}) stdout differs:\n  C   = {}\n  Rust= {}",
        show(&oc),
        show(&or)
    );
}

#[test]
fn row28_confusion_random_full_range() {
    let mut rng = Rng::new(0xB0028);
    for i in 0..1500 {
        let (a, b, c, d) = (rng.next_i32(), rng.next_i32(), rng.next_i32(), rng.next_i32());
        diff_confusion(&format!("row28 #{i}"), a, b, c, d);
    }
}

#[test]
fn row29_confusion_search_char_sign() {
    let mut rng = Rng::new(0xB0029);
    for p3 in -20..=20i32 {
        for _ in 0..8 {
            let a = rng.next_i32();
            let b = rng.next_i32();
            let d = rng.next_i32();
            diff_confusion(&format!("row29 p3={p3}"), a, b, p3, d);
        }
    }
}

#[test]
fn row30_confusion_all_arms_forced() {
    let mut rng = Rng::new(0xB0030);
    for p4 in -8..=8i32 {
        for _ in 0..25 {
            let a = rng.next_i32();
            let b = rng.next_i32();
            let c = rng.next_i32();
            diff_confusion(&format!("row30 p4={p4}"), a, b, c, p4);
        }
    }
}

#[test]
fn row31_confusion_boundary_matrix() {
    for a in [0i32, 1, -1, i32::MIN, i32::MAX] {
        for b in [0i32, 1, 2, 4, 7, 8, 24, 56, 63, -1, i32::MIN] {
            for c in [-10i32, -1, 0, 1, 5, 9, 10] {
                for d in [-4i32, -3, -2, -1, 0, 1, 2, 3, 4] {
                    diff_confusion("row31", a, b, c, d);
                }
            }
        }
    }
}

#[test]
fn row32_confusion_repeated_invocations() {
    // No cross-call state may leak: interleave C and Rust calls and also run
    // the same arguments repeatedly.
    let p = pair();
    let mut rng = Rng::new(0xB0032);
    for _ in 0..200 {
        let (a, b, c, d) = (rng.next_i32(), rng.next_i32(), rng.next_i32(), rng.next_i32());
        for _ in 0..3 {
            let (rc, oc) = capture(|| unsafe { (p.c.confusion)(a, b, c, d) });
            let (rr, or) = capture(|| unsafe { (p.r.confusion)(a, b, c, d) });
            assert_eq!(rc, rr, "row32 return differs for ({a},{b},{c},{d})");
            assert_eq!(oc, or, "row32 stdout differs for ({a},{b},{c},{d})");
        }
    }
}
