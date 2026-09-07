//! Phase C — error-path differential tests, one test per `ERRORS.md` row.
//!
//! Each test constructs the exact rejecting condition, calls BOTH libraries
//! through their exported symbols, and asserts the SAME sentinel / error output.

mod harness;

use harness::*;
use std::ffi::{c_char, c_int};

// --- Row 1 -----------------------------------------------------------------
// `malloc(sizeof(ProcessState))` failing is not reachable from a test (a 24-byte
// allocation always succeeds), but the *code shape* is verified: both libraries
// print the same message and return NULL on the buffer-allocation failure that
// IS reachable (row 2), and both take the same early-return structure.
#[test]
fn row01_state_malloc_failure_unreachable_documented() {
    // Nothing to execute; asserted by inspection + row 2 covering the sibling
    // failure branch. Kept as an explicit, passing placeholder so the row is
    // accounted for.
    let l = libs();
    // both libraries export the function that contains the branch
    let _ = l.c.create_state();
    let _ = l.rust.create_state();
}

// --- Rows 2 & 5 ------------------------------------------------------------
#[test]
fn row02_row05_create_state_negative_capacity_returns_null() {
    let l = libs();
    for &cap in &[
        -1,
        -2,
        -16,
        -128,
        -1024,
        -1_000_000,
        i32::MIN,
        i32::MIN + 1,
        -0x4000_0000,
    ] {
        for &init in &[0, 1, -1, i32::MAX, i32::MIN] {
            let _g = lock();
            let (c_null, c_out) = capture_stdout(|| unsafe {
                let p = l.c.create_state()(init, cap);
                let n = p.is_null();
                if !n {
                    l.c.destroy_state()(p);
                }
                n
            });
            let (r_null, r_out) = capture_stdout(|| unsafe {
                let p = l.rust.create_state()(init, cap);
                let n = p.is_null();
                if !n {
                    l.rust.destroy_state()(p);
                }
                n
            });
            assert!(c_null, "C create_state({init}, {cap}) unexpectedly succeeded");
            assert_eq!(c_null, r_null, "create_state({init}, {cap}) nullness differs");
            assert_eq!(
                show(&c_out),
                show(&r_out),
                "create_state({init}, {cap}) stdout differs"
            );
            assert_eq!(
                c_out, b"Error: Failed to allocate buffer\n",
                "unexpected C message"
            );
        }
    }
}

// --- Row 3 -----------------------------------------------------------------
#[test]
fn row03_create_state_capacity_zero_is_accepted() {
    let l = libs();
    for &init in &[0, 1, -1, 12345, i32::MAX, i32::MIN] {
        let _g = lock();
        let run = |imp: &Impl| unsafe {
            let p = imp.create_state()(init, 0);
            let s = snapshot(p, false); // buffer bytes are uninitialised
            if !p.is_null() {
                imp.destroy_state()(p);
            }
            s
        };
        let (c_s, c_out) = capture_stdout(|| run(&l.c));
        let (r_s, r_out) = capture_stdout(|| run(&l.rust));
        assert!(!c_s.is_null, "C rejected capacity 0");
        assert!(!c_s.buffer_is_null, "C gave a NULL buffer for capacity 0");
        assert_eq!(c_s, r_s, "create_state({init}, 0) state differs");
        assert_eq!(show(&c_out), show(&r_out), "create_state({init}, 0) stdout");
        assert!(c_out.is_empty(), "capacity 0 must not print: {}", show(&c_out));
    }
}

// --- Row 4 -----------------------------------------------------------------
#[test]
fn row04_create_state_truncation_is_accepted() {
    let l = libs();
    for cap in 1..=16 {
        for &init in &[0, 7, -7, 123456, -123456, i32::MAX, i32::MIN] {
            let _g = lock();
            let run = |imp: &Impl| unsafe {
                let p = imp.create_state()(init, cap);
                let s = snapshot(p, true);
                if !p.is_null() {
                    imp.destroy_state()(p);
                }
                s
            };
            let (c_s, c_out) = capture_stdout(|| run(&l.c));
            let (r_s, r_out) = capture_stdout(|| run(&l.rust));
            assert!(!c_s.is_null, "C rejected capacity {cap}");
            assert!(
                c_s.buffer.len() < cap as usize,
                "C buffer must fit in capacity"
            );
            assert_eq!(c_s, r_s, "create_state({init}, {cap}) truncation differs");
            assert_eq!(show(&c_out), show(&r_out));
            assert!(c_out.is_empty());
        }
    }
}

// --- Row 6 -----------------------------------------------------------------
#[test]
fn row06_destroy_state_null_is_noop() {
    let l = libs();
    let _g = lock();
    let ((), c_out) = capture_stdout(|| unsafe { l.c.destroy_state()(std::ptr::null_mut()) });
    let ((), r_out) = capture_stdout(|| unsafe { l.rust.destroy_state()(std::ptr::null_mut()) });
    assert_eq!(show(&c_out), show(&r_out));
    assert!(c_out.is_empty(), "destroy_state(NULL) must be silent");
}

// --- Row 7 -----------------------------------------------------------------
#[test]
fn row07_destroy_state_null_buffer() {
    let l = libs();
    for &(flags, data, cap) in &[(0u32, 0u32, 0 as c_int), (0x7B05, 0xDEADBEEF, -1), (u32::MAX, u32::MAX, i32::MAX)] {
        let _g = lock();
        // Each state is freed by destroy_state, so hand ownership over.
        let c_state = std::mem::ManuallyDrop::new(SyntheticState::with_null_buffer(flags, data, cap));
        let r_state = std::mem::ManuallyDrop::new(SyntheticState::with_null_buffer(flags, data, cap));
        let ((), c_out) = capture_stdout(|| unsafe { l.c.destroy_state()(c_state.ptr) });
        let ((), r_out) = capture_stdout(|| unsafe { l.rust.destroy_state()(r_state.ptr) });
        assert_eq!(show(&c_out), show(&r_out));
        assert!(c_out.is_empty(), "must be silent: {}", show(&c_out));
    }
}

// --- Row 8 -----------------------------------------------------------------
#[test]
fn row08_process_buffer_null_state() {
    let l = libs();
    for &target in &[0u8, b'0', b'5', 0x80, 0xFF, 1] {
        let _g = lock();
        let (c_r, c_out) = capture_stdout(|| unsafe {
            l.c.process_buffer()(std::ptr::null_mut(), target as c_char)
        });
        let (r_r, r_out) = capture_stdout(|| unsafe {
            l.rust.process_buffer()(std::ptr::null_mut(), target as c_char)
        });
        assert_eq!(c_r, -1, "C must return -1 for NULL state");
        assert_eq!(c_r, r_r, "return differs for target={target}");
        assert_eq!(show(&c_out), show(&r_out), "stdout differs");
        assert_eq!(c_out, b"Error: Null pointer in process_buffer\n");
    }
}

// --- Row 9 -----------------------------------------------------------------
#[test]
fn row09_process_buffer_null_buffer() {
    let l = libs();
    for &(flags, data) in &[(0u32, 0u32), (0x7B05, 0x40490FDB), (u32::MAX, u32::MAX)] {
        for &target in &[0u8, b'0', 0xFF] {
            let _g = lock();
            let c_state = SyntheticState::with_null_buffer(flags, data, 128);
            let r_state = SyntheticState::with_null_buffer(flags, data, 128);
            let (c_r, c_out) =
                capture_stdout(|| unsafe { l.c.process_buffer()(c_state.ptr, target as c_char) });
            let (r_r, r_out) =
                capture_stdout(|| unsafe { l.rust.process_buffer()(r_state.ptr, target as c_char) });
            assert_eq!(c_r, -1, "C must return -1 for NULL buffer");
            assert_eq!(c_r, r_r);
            assert_eq!(show(&c_out), show(&r_out));
            assert_eq!(c_out, b"Error: Null pointer in process_buffer\n");
        }
    }
}

// --- Row 10 ----------------------------------------------------------------
#[test]
fn row10_process_buffer_empty_buffer_returns_zero() {
    let l = libs();
    for &target in &[0u8, b'0', b'a', 0x80, 0xFF] {
        let _g = lock();
        let c_state = SyntheticState::new(0, 0, 0, b"");
        let r_state = SyntheticState::new(0, 0, 0, b"");
        let (c_r, c_out) =
            capture_stdout(|| unsafe { l.c.process_buffer()(c_state.ptr, target as c_char) });
        let (r_r, r_out) =
            capture_stdout(|| unsafe { l.rust.process_buffer()(r_state.ptr, target as c_char) });
        assert_eq!(c_r, 0, "empty buffer must give 0");
        assert_eq!(c_r, r_r);
        assert_eq!(show(&c_out), show(&r_out));
        assert!(c_out.is_empty());
    }
}

// --- Row 11 ----------------------------------------------------------------
#[test]
fn row11_process_buffer_nul_target_never_matches() {
    let l = libs();
    let bufs: [&[u8]; 6] = [b"", b"a", b"State:1:Mode:3", b"\xff\xfe\x01", b"0123456789", b"\x01"];
    for content in bufs {
        let _g = lock();
        let c_state = SyntheticState::new(0, 0, content.len() as c_int, content);
        let r_state = SyntheticState::new(0, 0, content.len() as c_int, content);
        let (c_r, c_out) = capture_stdout(|| unsafe { l.c.process_buffer()(c_state.ptr, 0) });
        let (r_r, r_out) = capture_stdout(|| unsafe { l.rust.process_buffer()(r_state.ptr, 0) });
        assert_eq!(c_r, 0, "NUL target must give 0 for {content:?}");
        assert_eq!(c_r, r_r);
        assert_eq!(show(&c_out), show(&r_out));
        assert!(c_out.is_empty());
    }
}

// --- Row 12 ----------------------------------------------------------------
#[test]
fn row12_process_buffer_negative_char_target() {
    let l = libs();
    let mut rng = Rng::new(0xE12);
    for high in [0x80u8, 0x81, 0xC0, 0xFE, 0xFF] {
        // buffers that DO contain the high byte, plus random high-byte soup
        let mut shapes: Vec<Vec<u8>> = vec![
            vec![high],
            vec![high, high],
            vec![high; 7],
            vec![b'a', high, b'b', high, high],
            vec![0x7F, high],
            vec![high ^ 0x01, high],
        ];
        for _ in 0..40 {
            let len = rng.below(24) as usize;
            shapes.push((0..len).map(|_| if rng.below(3) == 0 { high } else { 1 + rng.byte() % 255 }).collect());
        }
        for content in &shapes {
            let _g = lock();
            let c_state = SyntheticState::new(0, 0, content.len() as c_int, content);
            let r_state = SyntheticState::new(0, 0, content.len() as c_int, content);
            let (c_r, c_out) =
                capture_stdout(|| unsafe { l.c.process_buffer()(c_state.ptr, high as c_char) });
            let (r_r, r_out) =
                capture_stdout(|| unsafe { l.rust.process_buffer()(r_state.ptr, high as c_char) });
            assert_eq!(c_r, r_r, "high target {high:#x} on {content:?}");
            assert_eq!(show(&c_out), show(&r_out));
        }
    }
}

// --- Row 13 ----------------------------------------------------------------
#[test]
fn row13_process_buffer_target_absent() {
    let l = libs();
    for content in [b"aaaa".as_ref(), b"State:0:Mode:3".as_ref(), b"zzz".as_ref()] {
        for &target in &[b'Q', 0x80u8, 0xFF, 1, b'\t'] {
            assert!(!content.contains(&target));
            let _g = lock();
            let c_state = SyntheticState::new(0, 0, content.len() as c_int, content);
            let r_state = SyntheticState::new(0, 0, content.len() as c_int, content);
            let (c_r, c_out) =
                capture_stdout(|| unsafe { l.c.process_buffer()(c_state.ptr, target as c_char) });
            let (r_r, r_out) =
                capture_stdout(|| unsafe { l.rust.process_buffer()(r_state.ptr, target as c_char) });
            assert_eq!(c_r, 0, "absent target must give 0");
            assert_eq!(c_r, r_r);
            assert_eq!(show(&c_out), show(&r_out));
            assert!(c_out.is_empty());
        }
    }
}

// --- Row 14 ----------------------------------------------------------------
#[test]
fn row14_update_flags_null_state_is_noop() {
    let l = libs();
    for &param in INTERESTING_I32 {
        let _g = lock();
        let ((), c_out) =
            capture_stdout(|| unsafe { l.c.update_flags()(std::ptr::null_mut(), param) });
        let ((), r_out) =
            capture_stdout(|| unsafe { l.rust.update_flags()(std::ptr::null_mut(), param) });
        assert_eq!(show(&c_out), show(&r_out));
        assert!(c_out.is_empty(), "update_flags(NULL) must be silent");
    }
}

// --- Row 15 ----------------------------------------------------------------
#[test]
fn row15_update_flags_negative_param_arithmetic_shift() {
    let l = libs();
    let mut rng = Rng::new(0xE15);
    let mut params: Vec<c_int> = vec![-1, -2, -4, -8, -9, -16, -64, i32::MIN, i32::MIN + 7, -0x7FFFFFFF];
    for _ in 0..300 {
        params.push(!(rng.next_u32() >> 1) as c_int);
    }
    for param in params {
        let _g = lock();
        let run = |imp: &Impl| unsafe {
            let c_state = SyntheticState::new(0x7B05, 0, 0, b"");
            imp.update_flags()(c_state.ptr, param);
            snapshot(c_state.ptr, false)
        };
        let (c_s, c_out) = capture_stdout(|| run(&l.c));
        let (r_s, r_out) = capture_stdout(|| run(&l.rust));
        assert_eq!(c_s, r_s, "update_flags(param={param}) flags differ");
        assert_eq!(show(&c_out), show(&r_out), "update_flags(param={param}) stdout");
    }
}

// --- Row 16 ----------------------------------------------------------------
#[test]
fn row16_update_flags_counter_at_max_wraps() {
    let l = libs();
    // counter == 31 encoded directly into the bit-field word (bits 3..8)
    let flags_counter31: u32 = (31 << COUNTER_SHIFT) | (3 << MODE_SHIFT) | (15 << STATUS_SHIFT) | 1 | (1 << 2);
    for &param in &[0, 1, 7, 8, 63, -1, i32::MIN, i32::MAX] {
        let _g = lock();
        let run = |imp: &Impl| unsafe {
            let st = SyntheticState::new(flags_counter31, 0, 0, b"");
            imp.update_flags()(st.ptr, param);
            snapshot(st.ptr, false)
        };
        let (c_s, c_out) = capture_stdout(|| run(&l.c));
        let (r_s, r_out) = capture_stdout(|| run(&l.rust));
        assert_eq!(
            field(c_s.flags, COUNTER_SHIFT, 5),
            0,
            "C counter must wrap 31 -> 0"
        );
        assert_eq!(c_s, r_s, "counter wrap differs for param={param}");
        assert_eq!(show(&c_out), show(&r_out));
    }
    // Also verify no neighbouring field is clobbered by the wrap.
    for start in 0u32..32 {
        let flags = (start << COUNTER_SHIFT) | (5 << MODE_SHIFT) | (21 << STATUS_SHIFT) | 0xABCD0000 | 0b011;
        let _g = lock();
        let run = |imp: &Impl| unsafe {
            let st = SyntheticState::new(flags, 0, 0, b"");
            imp.update_flags()(st.ptr, 0);
            snapshot(st.ptr, false)
        };
        let (c_s, c_out) = capture_stdout(|| run(&l.c));
        let (r_s, r_out) = capture_stdout(|| run(&l.rust));
        assert_eq!(c_s, r_s, "counter start={start} differs");
        assert_eq!(show(&c_out), show(&r_out));
    }
}

// --- Row 17 ----------------------------------------------------------------
#[test]
fn row17_confuse_types_null_state_returns_zero() {
    let l = libs();
    for &op in INTERESTING_I32 {
        let _g = lock();
        let (c_r, c_out) =
            capture_stdout(|| unsafe { l.c.confuse_types()(std::ptr::null_mut(), op) });
        let (r_r, r_out) =
            capture_stdout(|| unsafe { l.rust.confuse_types()(std::ptr::null_mut(), op) });
        assert_eq!(c_r, 0, "C must return 0 for NULL state (op={op})");
        assert_eq!(c_r, r_r);
        assert_eq!(show(&c_out), show(&r_out));
        assert!(c_out.is_empty());
    }
}

// --- Row 18 ----------------------------------------------------------------
// Out-of-range "enum" values crossing the FFI boundary: the C `switch` has no
// `default`, so any int outside {0,1,2,3} must silently return 0 and print
// nothing — and must not mutate the state.
#[test]
fn row18_confuse_types_out_of_range_operation() {
    let l = libs();
    let mut ops: Vec<c_int> = vec![
        4, 5, 6, 7, 8, 100, 1000, -1, -2, -3, -4, -100, i32::MAX, i32::MIN, i32::MAX - 1,
        i32::MIN + 1, 0x7FFFFFFF, -0x80000000,
    ];
    let mut rng = Rng::new(0xE18);
    for _ in 0..200 {
        let v = rng.next_i32();
        if !(0..4).contains(&v) {
            ops.push(v);
        }
    }
    for op in ops {
        for &init in &[0, 1, -1, 1078530011, i32::MAX, i32::MIN] {
            let _g = lock();
            let run = |imp: &Impl| unsafe {
                let p = imp.create_state()(init, 128);
                assert!(!p.is_null());
                let r = imp.confuse_types()(p, op);
                let s = snapshot(p, true);
                imp.destroy_state()(p);
                (r, s)
            };
            let ((c_r, c_s), c_out) = capture_stdout(|| run(&l.c));
            let ((r_r, r_s), r_out) = capture_stdout(|| run(&l.rust));
            assert_eq!(c_r, 0, "C must return 0 for op={op}");
            assert_eq!(c_s.data, init as u32, "C must not mutate the union (op={op})");
            assert_eq!((c_r, &c_s), (r_r, &r_s), "op={op} init={init} differs");
            assert_eq!(show(&c_out), show(&r_out));
            assert!(c_out.is_empty(), "op={op} must print nothing: {}", show(&c_out));
        }
    }
}

// --- Rows 19 & 20 ----------------------------------------------------------
// `(int)(float_val * 100)` where the result is NaN / Inf / out of int range:
// x86-64 `cvttss2si` yields the "integer indefinite" value INT_MIN.
#[test]
fn row19_row20_confuse_types_float_cast_undefined_range() {
    let l = libs();
    let mut pats: Vec<i32> = vec![
        0x7F800000u32 as i32, // +inf
        0xFF800000u32 as i32, // -inf
        0x7FC00000u32 as i32, // qNaN
        0x7F800001u32 as i32, // sNaN
        0xFFC00000u32 as i32, // -qNaN
        0x7F7FFFFFu32 as i32, // FLT_MAX
        0xFF7FFFFFu32 as i32, // -FLT_MAX
        0x4EFFFFFFu32 as i32, // just below 2^31 before *100
        0x4F000000u32 as i32,
        0xCF000000u32 as i32, // -2^31 exactly
        0xCEFFFFFFu32 as i32,
        0x4B000000u32 as i32, // 2^23; *100 fits
        0x4C742400u32 as i32, // ~6.4e7 -> *100 = 6.4e9 overflows
        0x00000000,
        0x80000000u32 as i32, // -0.0
        0x00000001,           // denormal
        0x80000001u32 as i32,
    ];
    // sweep exponents so we straddle the int-range boundary densely
    for exp in 0u32..=255 {
        for sign in [0u32, 1] {
            pats.push(((sign << 31) | (exp << 23) | 0x123456) as i32);
        }
    }
    for p in pats {
        let _g = lock();
        let run = |imp: &Impl| unsafe {
            let st = imp.create_state()(p, 128);
            assert!(!st.is_null());
            let r = imp.confuse_types()(st, 1);
            imp.destroy_state()(st);
            r
        };
        let (c_r, c_out) = capture_stdout(|| run(&l.c));
        let (r_r, r_out) = capture_stdout(|| run(&l.rust));
        assert_eq!(
            c_r, r_r,
            "confuse_types(bits=0x{p:08x}, op=1) return differs (C={c_r}, Rust={r_r})"
        );
        assert_eq!(
            show(&c_out),
            show(&r_out),
            "confuse_types(bits=0x{p:08x}, op=1) stdout differs"
        );
    }
}

// --- Row 21 ----------------------------------------------------------------
#[test]
fn row21_confuse_types_signed_char_byte_sum() {
    let l = libs();
    for a in [0x00u32, 0x01, 0x7F, 0x80, 0x81, 0xC0, 0xFF] {
        for b in [0x00u32, 0x01, 0x7F, 0x80, 0x81, 0xC0, 0xFF] {
            for hi in [0x00u32, 0x7F, 0x80, 0xFF] {
                let bits = ((hi << 24) | (hi << 16) | (b << 8) | a) as i32;
                let _g = lock();
                let run = |imp: &Impl| unsafe {
                    let st = imp.create_state()(bits, 128);
                    assert!(!st.is_null());
                    let r = imp.confuse_types()(st, 3);
                    imp.destroy_state()(st);
                    r
                };
                let (c_r, c_out) = capture_stdout(|| run(&l.c));
                let (r_r, r_out) = capture_stdout(|| run(&l.rust));
                assert_eq!(c_r, r_r, "bytes sum for bits=0x{bits:08x}");
                assert_eq!(show(&c_out), show(&r_out), "bytes print for bits=0x{bits:08x}");
            }
        }
    }
}

// --- Row 22 ----------------------------------------------------------------
#[test]
fn row22_confusion_create_state_failure_unreachable_documented() {
    // `confusion` always passes capacity 128, so the `state == NULL` -> `-1`
    // branch is unreachable from the public API. The equivalent early-return is
    // covered directly on `create_state` by row 2. Recorded as accounted for.
    let l = libs();
    let _ = l.c.confusion();
    let _ = l.rust.confusion();
}

// --- Rows 23 & 24 ----------------------------------------------------------
#[test]
fn row23_row24_confusion_negative_param3_nondigit_search_char() {
    let l = libs();
    let mut p3s: Vec<c_int> = (-30..=0).collect();
    p3s.extend_from_slice(&[i32::MIN, i32::MIN + 1, i32::MIN + 7, -1000000007]);
    for p3 in p3s {
        for &p4 in &[0, 1, 2, 3] {
            let _g = lock();
            let (c_r, c_out) = capture_stdout(|| unsafe { l.c.confusion()(48, 0, p3, p4) });
            let (r_r, r_out) = capture_stdout(|| unsafe { l.rust.confusion()(48, 0, p3, p4) });
            assert_eq!(c_r, r_r, "confusion(48,0,{p3},{p4}) return");
            assert_eq!(show(&c_out), show(&r_out), "confusion(48,0,{p3},{p4}) stdout");
        }
    }
}

// --- Rows 25 & 26 ----------------------------------------------------------
#[test]
fn row25_row26_confusion_negative_and_extreme_param4() {
    let l = libs();
    let mut p4s: Vec<c_int> = (-12..=12).collect();
    p4s.extend_from_slice(&[i32::MIN, i32::MIN + 1, i32::MAX, i32::MAX - 1, -1000000007]);
    for p4 in p4s {
        for &p1 in &[0, 1078530011, i32::MIN, i32::MAX] {
            let _g = lock();
            let (c_r, c_out) = capture_stdout(|| unsafe { l.c.confusion()(p1, 5, 3, p4) });
            let (r_r, r_out) = capture_stdout(|| unsafe { l.rust.confusion()(p1, 5, 3, p4) });
            assert_eq!(c_r, r_r, "confusion({p1},5,3,{p4}) return");
            assert_eq!(show(&c_out), show(&r_out), "confusion({p1},5,3,{p4}) stdout");
        }
    }
}

// --- Row 27 ----------------------------------------------------------------
// signed `int` overflow in `result += confusion_result` (wraps at -O0)
#[test]
fn row27_confusion_result_overflow_wraps() {
    let l = libs();
    let mut rng = Rng::new(0xE27);
    // op selector 1 gives huge/INT_MIN results that then get +counter*5+mode*3
    let mut cases: Vec<(c_int, c_int)> = vec![];
    for &p1 in &[
        0x7F800000u32 as i32,
        0xFF800000u32 as i32,
        0x7FC00000u32 as i32,
        0x4F000000u32 as i32,
        0xCF000000u32 as i32,
        i32::MIN,
        i32::MAX,
    ] {
        for p2 in 0..8 {
            cases.push((p1, p2));
        }
    }
    for _ in 0..500 {
        cases.push((rng.next_i32(), rng.next_i32()));
    }
    for (p1, p2) in cases {
        for &p4 in &[1, 5, 9, -3] {
            let _g = lock();
            let (c_r, c_out) = capture_stdout(|| unsafe { l.c.confusion()(p1, p2, 7, p4) });
            let (r_r, r_out) = capture_stdout(|| unsafe { l.rust.confusion()(p1, p2, 7, p4) });
            assert_eq!(c_r, r_r, "confusion({p1},{p2},7,{p4}) return");
            assert_eq!(show(&c_out), show(&r_out), "confusion({p1},{p2},7,{p4}) stdout");
        }
    }
}

// --- Row 28 ----------------------------------------------------------------
#[test]
fn row28_confusion_param1_extremes() {
    let l = libs();
    for &p1 in INTERESTING_I32 {
        for &p2 in &[0, 1, 63, -1, i32::MIN] {
            for p3 in [0, 5, -5] {
                for p4 in [0, 1, 2, 3, -1] {
                    let _g = lock();
                    let (c_r, c_out) =
                        capture_stdout(|| unsafe { l.c.confusion()(p1, p2, p3, p4) });
                    let (r_r, r_out) =
                        capture_stdout(|| unsafe { l.rust.confusion()(p1, p2, p3, p4) });
                    assert_eq!(c_r, r_r, "confusion({p1},{p2},{p3},{p4}) return");
                    assert_eq!(
                        show(&c_out),
                        show(&r_out),
                        "confusion({p1},{p2},{p3},{p4}) stdout"
                    );
                }
            }
        }
    }
}

// --- Extra boundary: oversized (but positive) capacity ---------------------
// `malloc(capacity)` with a huge positive size may succeed or fail depending on
// the system; whichever it does, BOTH libraries must agree, and both must
// report it the same way.
#[test]
fn extra_create_state_oversized_capacity() {
    let l = libs();
    for &cap in &[
        i32::MAX,
        i32::MAX - 1,
        0x4000_0000, // 1 GiB
        0x2000_0000, // 512 MiB
        1 << 20,
        1 << 16,
    ] {
        for &init in &[0, -1, i32::MAX, i32::MIN] {
            let _g = lock();
            let run = |imp: &Impl| unsafe {
                let p = imp.create_state()(init, cap);
                let s = snapshot(p, !p.is_null());
                if !p.is_null() {
                    imp.destroy_state()(p);
                }
                s
            };
            let (c_s, c_out) = capture_stdout(|| run(&l.c));
            let (r_s, r_out) = capture_stdout(|| run(&l.rust));
            assert_eq!(
                c_s, r_s,
                "create_state({init}, {cap}) differs (C null={}, Rust null={})",
                c_s.is_null, r_s.is_null
            );
            assert_eq!(
                show(&c_out),
                show(&r_out),
                "create_state({init}, {cap}) stdout differs"
            );
        }
    }
}

// --- Extra boundary: one step past each valid `confuse_types` selector ------
#[test]
fn extra_confuse_types_one_past_valid_range() {
    let l = libs();
    for op in [-1, 0, 1, 2, 3, 4] {
        for &init in &[0x4EFFFFFFu32 as i32, 0x7FC00000u32 as i32, 1078530011, -1] {
            let _g = lock();
            let run = |imp: &Impl| unsafe {
                let p = imp.create_state()(init, 128);
                let r = imp.confuse_types()(p, op);
                let s = snapshot(p, true);
                imp.destroy_state()(p);
                (r, s)
            };
            let (c, c_out) = capture_stdout(|| run(&l.c));
            let (r, r_out) = capture_stdout(|| run(&l.rust));
            assert_eq!(c, r, "confuse_types(op={op}, init=0x{init:08x})");
            assert_eq!(show(&c_out), show(&r_out));
        }
    }
}
