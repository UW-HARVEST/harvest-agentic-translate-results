// Phase C — error-path differential tests, one test per row of ERRORS.md
// (the three out-of-memory rows E7/E15/E16 live in tests/alloc_failure.rs).

mod support;

use std::ffi::{c_char, c_int, c_void};
use support::*;

extern "C" {
    fn free(p: *mut c_void);
}

// ---------------------------------------------------------------------------
// E1 / E2 — is_string_empty rejections
// ---------------------------------------------------------------------------
#[test]
fn e1_is_string_empty_null() {
    let (c, r) = both();
    unsafe {
        let rc = (c.is_string_empty)(std::ptr::null());
        let rr = (r.is_string_empty)(std::ptr::null());
        assert_eq!(rc, 1, "C is_string_empty(NULL) must return 1, got {rc}");
        assert_eq!(rr, rc, "is_string_empty(NULL) mismatch: C {rc} vs Rust {rr}");
    }
}

#[test]
fn e2_is_string_empty_empty() {
    let (c, r) = both();
    unsafe {
        let s = b"\0";
        let rc = (c.is_string_empty)(s.as_ptr() as *const c_char);
        let rr = (r.is_string_empty)(s.as_ptr() as *const c_char);
        assert_eq!(rc, 1, "C is_string_empty(\"\") must return 1, got {rc}");
        assert_eq!(rr, rc, "is_string_empty(\"\") mismatch: C {rc} vs Rust {rr}");
    }
}

// ---------------------------------------------------------------------------
// E3 / E4 / E5 — find_char_in_buffer rejections
// ---------------------------------------------------------------------------
#[test]
fn e3_find_char_null_buffer() {
    for size in [0usize, 1, 16, 4096, usize::MAX / 2, usize::MAX] {
        for target in [0u8, b'a', 0x7f, 0x80, 0xff] {
            diff_find_null(size, target);
        }
    }
}

#[test]
fn e4_find_char_zero_size() {
    let buf = b"Xabc";
    diff_find(buf, 0, b'X'); // target is the very first byte, still a miss
    diff_find(buf, 0, 0);
    let (c, r) = both();
    unsafe {
        let pc = (c.find_char_in_buffer)(buf.as_ptr() as *const c_char, 0, b'X' as c_char);
        let pr = (r.find_char_in_buffer)(buf.as_ptr() as *const c_char, 0, b'X' as c_char);
        assert!(pc.is_null(), "C must return NULL for size == 0");
        assert!(pr.is_null(), "Rust must return NULL for size == 0");
    }
}

#[test]
fn e5_find_char_absent() {
    let buf = b"abcdef";
    diff_find(buf, buf.len(), b'X');
    // present, but only after `size`
    let buf2 = b"abcdefX";
    diff_find(buf2, 6, b'X');
    // randomized misses
    let mut rng = Rng::new(SEED ^ 21);
    for _ in 0..512 {
        let len = 1 + rng.below(64);
        let buf: Vec<u8> = (0..len).map(|_| 1 + (rng.byte() % 100)).collect(); // bytes 1..=100
        diff_find(&buf, len, 200); // never present
    }
}

// ---------------------------------------------------------------------------
// E6 — create_buffer(NULL)
// ---------------------------------------------------------------------------
#[test]
fn e6_create_buffer_null() {
    let (c, r) = both();
    unsafe {
        let pc = (c.create_buffer)(std::ptr::null());
        let pr = (r.create_buffer)(std::ptr::null());
        assert!(pc.is_null(), "C create_buffer(NULL) must return NULL");
        assert!(pr.is_null(), "Rust create_buffer(NULL) must return NULL");
    }
}

// ---------------------------------------------------------------------------
// E8 / E9 — validate_uint16_range rejections
// ---------------------------------------------------------------------------
#[test]
fn e8_validate_negative() {
    let (c, r) = both();
    for v in [-1, -2, -255, -65535, -65536, i32::MIN, i32::MIN + 1] {
        unsafe {
            let rc = (c.validate_uint16_range)(v);
            let rr = (r.validate_uint16_range)(v);
            assert_eq!(rc, 0, "C validate_uint16_range({v}) must reject");
            assert_eq!(rr, rc, "validate_uint16_range({v}) mismatch");
        }
    }
}

#[test]
fn e9_validate_too_large() {
    let (c, r) = both();
    for v in [65536, 65537, 70000, 1 << 20, i32::MAX - 1, i32::MAX] {
        unsafe {
            let rc = (c.validate_uint16_range)(v);
            let rr = (r.validate_uint16_range)(v);
            assert_eq!(rc, 0, "C validate_uint16_range({v}) must reject");
            assert_eq!(rr, rc, "validate_uint16_range({v}) mismatch");
        }
    }
}

// ---------------------------------------------------------------------------
// E10 — apply_operation(NULL, value)
// ---------------------------------------------------------------------------
#[test]
fn e10_apply_operation_null() {
    let (c, r) = both();
    for v in [0, 1, -1, i32::MAX, i32::MIN, 12345] {
        seed_counters(4242);
        unsafe {
            let rc = (c.apply_operation)(None, v);
            let rr = (r.apply_operation)(None, v);
            assert_eq!(rc, -1, "C apply_operation(NULL, {v}) must return -1");
            assert_eq!(rr, rc, "apply_operation(NULL, {v}) mismatch");
            // The counter must not have been touched by the rejected call.
            let sc = (c.increment_counter)(0);
            let sr = (r.increment_counter)(0);
            assert_eq!(sc, 4242, "C counter changed by a rejected apply_operation");
            assert_eq!(sr, sc, "counter mismatch after apply_operation(NULL)");
        }
    }
}

// ---------------------------------------------------------------------------
// E11 — charinbuf default: (out-of-range "enum" values for `mode`)
// ---------------------------------------------------------------------------
#[test]
fn e11_charinbuf_invalid_mode() {
    for mode in [-1, 5, 6, 100, -100, i32::MIN, i32::MIN + 1, i32::MAX, i32::MAX - 1] {
        let (rc, out) = diff_charinbuf_capture(mode, 1, 2, 3);
        assert_eq!(rc, -1, "C charinbuf(mode={mode}) must return -1");
        assert_eq!(
            String::from_utf8_lossy(&out),
            format!("Invalid mode: {mode}\n"),
            "unexpected C output for mode {mode}"
        );
    }
    // 0x8000_0000 as i32 (the value a C caller gets from (int)0x80000000)
    let mode = 0x8000_0000u32 as i32;
    let (rc, _) = diff_charinbuf_capture(mode, 0, 0, 0);
    assert_eq!(rc, -1);
}

#[test]
fn e11b_charinbuf_invalid_mode_random() {
    let mut rng = Rng::new(SEED ^ 22);
    // Dense sweep around the valid range.
    for mode in -8..=12 {
        diff_charinbuf(mode, rng.next_i32(), rng.next_i32(), rng.next_i32());
    }
    // 4096 arbitrary ints: essentially all of them hit `default:`.
    for _ in 0..4096 {
        let mode = rng.next_i32();
        diff_charinbuf(mode, rng.interesting_i32(), rng.next_i32(), rng.next_i32());
    }
}

// ---------------------------------------------------------------------------
// E12 / E13 / E14 — charinbuf mode 0 range rejection
// ---------------------------------------------------------------------------
#[test]
fn e12_charinbuf_mode0_negative() {
    for v in [-1, -2, -65535, -65536, i32::MIN, i32::MIN + 1] {
        let (rc, out) = diff_charinbuf_capture(0, v, 0, 0);
        assert_eq!(rc, -1, "C charinbuf(0, {v}) must return -1");
        let text = String::from_utf8_lossy(&out);
        assert!(
            text.contains(&format!("Value {v} is out of range for uint16_t")),
            "unexpected C output for value {v}: {text:?}"
        );
        assert!(text.contains("UINT16_MAX constant value: 65535"));
    }
}

#[test]
fn e13_charinbuf_mode0_too_large() {
    for v in [65536, 65537, 1 << 20, i32::MAX - 1, i32::MAX] {
        let (rc, out) = diff_charinbuf_capture(0, v, 0, 0);
        assert_eq!(rc, -1, "C charinbuf(0, {v}) must return -1");
        let text = String::from_utf8_lossy(&out);
        assert!(
            text.contains(&format!("Value {v} is out of range for uint16_t")),
            "unexpected C output for value {v}: {text:?}"
        );
    }
}

#[test]
fn e14_charinbuf_mode0_boundaries() {
    for (v, expect) in [(-1, -1), (0, 0), (1, 1), (65534, 65534), (65535, 65535), (65536, -1)] {
        let (rc, _) = diff_charinbuf_capture(0, v, 0, 0);
        assert_eq!(rc, expect, "C charinbuf(0, {v}) expected {expect}, got {rc}");
    }
}

// ---------------------------------------------------------------------------
// E17 / E18 — branches that are unreachable through charinbuf
// ---------------------------------------------------------------------------
#[test]
fn e17_mode4_never_reports_miss() {
    let mut rng = Rng::new(SEED ^ 23);
    for _ in 0..64 {
        let (rc, out) = diff_charinbuf_capture(4, rng.next_i32(), rng.next_i32(), rng.next_i32());
        let text = String::from_utf8_lossy(&out);
        assert_eq!(rc, 21);
        assert!(!text.contains("not found"), "C reported a memchr miss: {text:?}");
    }
    // The same C statement reached directly: a genuine miss returns NULL.
    diff_find(b"Search for character Y in this buffer", 36, b'X');
}

#[test]
fn e18_mode1_always_takes_success_branch() {
    let mut rng = Rng::new(SEED ^ 24);
    for _ in 0..64 {
        let (rc, out) = diff_charinbuf_capture(1, rng.next_i32(), rng.next_i32(), rng.next_i32());
        let text = String::from_utf8_lossy(&out);
        assert_eq!(rc, 10);
        assert!(
            !text.contains("Non-empty string check failed!"),
            "C took the impossible branch: {text:?}"
        );
    }
    // Reached directly through the export instead.
    diff_is_string_empty(b"Hello, World!\0");
    diff_is_string_empty(b"\0");
}

// ---------------------------------------------------------------------------
// G1..G8 — generic FFI boundary cases
// ---------------------------------------------------------------------------
#[test]
fn g1_all_null_pointers() {
    let (c, r) = both();
    unsafe {
        assert_eq!((c.is_string_empty)(std::ptr::null()), 1);
        assert_eq!((r.is_string_empty)(std::ptr::null()), 1);
        assert!((c.create_buffer)(std::ptr::null()).is_null());
        assert!((r.create_buffer)(std::ptr::null()).is_null());
        assert!((c.find_char_in_buffer)(std::ptr::null(), 10, b'a' as c_char).is_null());
        assert!((r.find_char_in_buffer)(std::ptr::null(), 10, b'a' as c_char).is_null());
        assert_eq!((c.apply_operation)(None, 7), -1);
        assert_eq!((r.apply_operation)(None, 7), -1);
    }
}

#[test]
fn g2_zero_len_and_empty() {
    diff_find(b"abc", 0, b'a');
    diff_create_buffer(b"\0");
    diff_is_string_empty(b"\0");
    // create_buffer("") must yield an empty, individually free-able string.
    let (c, r) = both();
    unsafe {
        let pc = (c.create_buffer)(b"\0".as_ptr() as *const c_char);
        let pr = (r.create_buffer)(b"\0".as_ptr() as *const c_char);
        assert!(!pc.is_null() && !pr.is_null());
        assert_eq!(*pc, 0, "C create_buffer(\"\") must be an empty string");
        assert_eq!(*pr, 0, "Rust create_buffer(\"\") must be an empty string");
        free(pc as *mut c_void);
        free(pr as *mut c_void);
    }
}

#[test]
fn g3_oversized_len() {
    // NULL buffer short-circuits before memchr, so any size is safe.
    diff_find_null(usize::MAX, b'a');
    diff_find_null(usize::MAX / 2, 0);

    // Non-NULL buffer with an absurd size: memchr stops at the first match, so
    // the match must sit at the very beginning of a large, aligned allocation.
    let mut buf = vec![b'#'; 65536];
    buf[0] = b'X';
    for size in [usize::MAX, usize::MAX / 2, 1usize << 40, 1usize << 20] {
        diff_find(&buf, size, b'X');
    }
    // Sizes that merely exceed the buffer's string length but stay inside the
    // allocation.
    let mut buf2 = vec![0u8; 4096];
    buf2[10] = b'Z';
    diff_find(&buf2, 4096, b'Z');
    diff_find(&buf2, 4096, 0);
}

#[test]
fn g4_one_past_range() {
    for v in [-1, 0, 65535, 65536] {
        diff_validate(v);
        diff_charinbuf(0, v, 0, 0);
    }
    diff_charinbuf(-1, 0, 0, 0);
    diff_charinbuf(5, 0, 0, 0);
}

#[test]
fn g5_extremes_all_modes() {
    for mode in [-1, 0, 1, 2, 3, 4, 5] {
        for v in [i32::MIN, i32::MIN + 1, -1, 0, 1, i32::MAX - 1, i32::MAX] {
            for o in [i32::MIN, -1, 0, 1, i32::MAX] {
                diff_charinbuf(mode, v, o, o);
                diff_charinbuf(mode, v, o, v);
            }
        }
    }
}

#[test]
fn g6_char_sign_edges() {
    // Bytes with the high bit set become negative `char` values and are
    // sign-extended by the C integer promotion in the `memchr` call.
    let buf: Vec<u8> = vec![0x00, 0x01, 0x7f, 0x80, 0x81, 0xfe, 0xff, b'a'];
    for t in [0x00u8, 0x01, 0x7f, 0x80, 0x81, 0xfe, 0xff, b'a', b'b'] {
        diff_find(&buf, buf.len(), t);
        diff_find(&buf, 4, t);
    }
    // ... and they must not be confused with their sign-extended int form.
    let only_high = vec![0x80u8, 0x80, 0x80];
    for t in [0x80u8, 0x00, 0xff] {
        diff_find(&only_high, only_high.len(), t);
    }
}

#[test]
fn g7_counter_overflow() {
    let (c, r) = both();
    // increment past INT_MAX
    seed_counters(i32::MAX);
    diff_op(1, 1);
    diff_op(1, i32::MAX);
    // decrement past INT_MIN
    seed_counters(i32::MIN);
    diff_op(3, 1);
    diff_op(3, i32::MAX);
    // multiply overflow
    seed_counters(i32::MAX);
    diff_op(2, 2);
    diff_op(2, -1);
    seed_counters(i32::MIN);
    diff_op(2, -1);
    diff_op(2, i32::MIN);
    // through apply_operation as well
    seed_counters(i32::MAX);
    diff_apply(1, 1);
    diff_apply(2, i32::MIN);
    diff_apply(3, i32::MIN);
    unsafe {
        let sc = (c.increment_counter)(0);
        let sr = (r.increment_counter)(0);
        assert_eq!(sc, sr, "counter diverged after the overflow sequence");
    }
}

#[test]
fn g8_embedded_nul() {
    diff_create_buffer(b"abc\0def\0");
    diff_is_string_empty(b"\0def\0");
    diff_find(b"abc\0def", 7, b'd');
}

// ---------------------------------------------------------------------------
// G9 — function pointer supplied by a *third party* (the test binary):
// exercises the raw C ABI of the callback slot in both libraries.
// ---------------------------------------------------------------------------
static CALLS: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
static LAST_ARG: std::sync::atomic::AtomicI32 = std::sync::atomic::AtomicI32::new(0);

unsafe extern "C" fn probe(value: c_int) -> c_int {
    CALLS.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    LAST_ARG.store(value, std::sync::atomic::Ordering::SeqCst);
    value.wrapping_mul(3).wrapping_add(1)
}

#[test]
fn g9_external_callback_pointer() {
    use std::sync::atomic::Ordering::SeqCst;
    let (c, r) = both();
    let mut rng = Rng::new(SEED ^ 25);
    for _ in 0..256 {
        let v = rng.interesting_i32();
        CALLS.store(0, SeqCst);
        let rc = unsafe { (c.apply_operation)(Some(probe), v) };
        assert_eq!(CALLS.load(SeqCst), 1, "C called the callback {} times", CALLS.load(SeqCst));
        assert_eq!(LAST_ARG.load(SeqCst), v, "C passed the wrong argument");

        CALLS.store(0, SeqCst);
        let rr = unsafe { (r.apply_operation)(Some(probe), v) };
        assert_eq!(
            CALLS.load(SeqCst),
            1,
            "Rust called the callback {} times",
            CALLS.load(SeqCst)
        );
        assert_eq!(LAST_ARG.load(SeqCst), v, "Rust passed the wrong argument");
        assert_eq!(rc, rr, "apply_operation(probe, {v}) mismatch: C {rc} vs Rust {rr}");
        assert_eq!(rc, v.wrapping_mul(3).wrapping_add(1));
    }
}

// ---------------------------------------------------------------------------
// G10 — the exported counter ops of one library must not disturb the other's
// state (i.e. each `.so` keeps its own `static counter`).
// ---------------------------------------------------------------------------
#[test]
fn g10_counter_state_is_per_library() {
    let (c, r) = both();
    unsafe {
        (c.reset_counter)(100);
        (r.reset_counter)(100);
        assert_eq!((c.increment_counter)(1), 101);
        assert_eq!((c.increment_counter)(1), 102);
        assert_eq!((r.increment_counter)(1), 101);
        assert_eq!((r.increment_counter)(1), 102);
        // charinbuf resets to 0 on entry in both.
        let _ = capture(|| (c.charinbuf)(1, 0, 0, 0));
        let _ = capture(|| (r.charinbuf)(1, 0, 0, 0));
        assert_eq!((c.increment_counter)(7), 7);
        assert_eq!((r.increment_counter)(7), 7);
    }
}
