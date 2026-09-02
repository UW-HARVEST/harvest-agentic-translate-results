//! Phase B — valid-path differential tests.
//!
//! One test per row of `CONFIGS.md`. Every test drives BOTH the C `.so` and the
//! Rust `.so` through `dlsym` and compares outputs byte-for-byte. All inputs
//! come from the fixed-seed PRNG in `common`, so runs are reproducible.

mod common;

use common::*;
use std::ffi::{c_char, c_int};

/// Randomized iterations per row.
const ITERS: usize = 400;

// ===========================================================================
// Rows 1-9: `charinbuf` (the convenience entry point) — return value + stdout
// ===========================================================================

#[test]
fn cfg_01_mode0_in_range() {
    let mut rng = Rng::new(0x0101);
    for _ in 0..ITERS {
        let value = rng.range_i32(0, 65535);
        let (rc, out) = diff_charinbuf(0, value, rng.next_i32(), rng.next_i32());
        assert_eq!(rc, value, "mode 0 in-range must return the value");
        assert!(show(&out).contains("is valid (0 <= value <= 65535)"));
    }
}

#[test]
fn cfg_02_mode0_boundaries() {
    for value in [0, 1, 2, 32767, 32768, 65534, 65535] {
        let mut rng = Rng::new(0x0202 ^ value as u64);
        for _ in 0..8 {
            let (rc, _) = diff_charinbuf(0, value, rng.next_i32(), rng.next_i32());
            assert_eq!(rc, value);
        }
    }
}

#[test]
fn cfg_03_mode0_full_int_range() {
    let mut rng = Rng::new(0x0303);
    for _ in 0..ITERS {
        let value = rng.interesting_i32();
        let (rc, out) = diff_charinbuf(0, value, rng.next_i32(), rng.next_i32());
        let expect = if (0..=65535).contains(&value) { value } else { -1 };
        assert_eq!(rc, expect, "mode 0 value={value}");
        // The UINT16_MAX %u line is printed on every path.
        assert!(show(&out).contains("UINT16_MAX constant value: 65535"));
    }
}

#[test]
fn cfg_04_mode1() {
    let mut rng = Rng::new(0x0404);
    for _ in 0..ITERS {
        // value/opt1/opt2 are all ignored by mode 1 (axis A4).
        let (rc, out) = diff_charinbuf(1, rng.interesting_i32(), rng.next_i32(), rng.next_i32());
        assert_eq!(rc, 10, "mode 1 is input-independent: 0 + 10");
        let s = show(&out);
        assert!(s.contains("Test string is empty (checked with *string)"));
        assert!(s.contains("Non-empty string correctly identified"));
    }
}

#[test]
fn cfg_05_mode2() {
    let mut rng = Rng::new(0x0505);
    for _ in 0..ITERS {
        let (rc, out) = diff_charinbuf(2, rng.interesting_i32(), rng.next_i32(), rng.next_i32());
        assert_eq!(rc, "Testing malloc and free".len() as c_int);
        let s = show(&out);
        assert!(s.contains("Buffer allocated: 'Testing malloc and free'"));
        assert!(s.contains("Buffer length: 23"));
        assert!(s.contains("Buffer freed successfully"));
    }
}

#[test]
fn cfg_06_mode3_full_range() {
    let mut rng = Rng::new(0x0606);
    for _ in 0..ITERS {
        let (value, opt1, opt2) = (
            rng.interesting_i32(),
            rng.interesting_i32(),
            rng.interesting_i32(),
        );
        let (rc, _) = diff_charinbuf(3, value, opt1, opt2);
        // Independent model of the wrapping chain, to catch "both wrong the
        // same way because the harness never varied the input".
        let expect = value
            .wrapping_add(opt1)
            .wrapping_mul(opt2)
            .wrapping_sub(5);
        assert_eq!(rc, expect, "mode 3 ({value}, {opt1}, {opt2})");
    }
}

#[test]
fn cfg_07_mode3_overflow_grid() {
    const POOL: [c_int; 7] = [i32::MIN, i32::MIN + 1, -2, -1, 0, 1, i32::MAX];
    for &value in POOL.iter() {
        for &opt1 in POOL.iter() {
            for &opt2 in POOL.iter() {
                let (rc, _) = diff_charinbuf(3, value, opt1, opt2);
                assert_eq!(
                    rc,
                    value.wrapping_add(opt1).wrapping_mul(opt2).wrapping_sub(5),
                    "mode 3 grid ({value}, {opt1}, {opt2})"
                );
            }
        }
    }
}

#[test]
fn cfg_08_mode4() {
    let mut rng = Rng::new(0x0808);
    let expect_off = "Search for character X in this buffer".find('X').unwrap() as c_int;
    for _ in 0..ITERS {
        let (rc, out) = diff_charinbuf(4, rng.interesting_i32(), rng.next_i32(), rng.next_i32());
        assert_eq!(rc, expect_off);
        let s = show(&out);
        assert!(s.contains("Searching for 'X' in: 'Search for character X in this buffer'"));
        assert!(s.contains(&format!("Found 'X' at position: {expect_off}")));
    }
}

#[test]
fn cfg_09_mode_sequence() {
    // Random *sequences* of modes so each call inherits counter/heap state from
    // the previous one (axis A5, forward direction).
    let mut rng = Rng::new(0x0909);
    let _g = guard();
    for _ in 0..ITERS {
        let mode = rng.range_i32(0, 4);
        diff_charinbuf_locked(mode, rng.interesting_i32(), rng.next_i32(), rng.next_i32());
    }
}

// ===========================================================================
// Row 10: counter state crossing the convenience / low-level boundary
// ===========================================================================

#[test]
fn cfg_10_counter_state_across_charinbuf() {
    let p = pair();
    let mut rng = Rng::new(0x1010);
    let _g = guard();
    for _ in 0..ITERS {
        // Pre-seed the counter, then confirm `charinbuf` throws it away.
        let seed = rng.interesting_i32();
        assert_eq!(p.c.call_mut(MutOp::Reset, seed), p.r.call_mut(MutOp::Reset, seed));

        let (value, opt1, opt2) = (
            rng.interesting_i32(),
            rng.interesting_i32(),
            rng.interesting_i32(),
        );
        let mode = rng.range_i32(0, 4);
        diff_charinbuf_locked(mode, value, opt1, opt2);

        // Read the counter back out through the low-level API: adding 0 returns
        // the current value without changing it.
        let after_c = p.c.call_mut(MutOp::Increment, 0);
        let after_r = p.r.call_mut(MutOp::Increment, 0);
        assert_eq!(
            after_c, after_r,
            "counter after charinbuf({mode}, {value}, {opt1}, {opt2}) with seed {seed}"
        );
        if mode == 3 {
            assert_eq!(
                after_c,
                value.wrapping_add(opt1).wrapping_mul(opt2).wrapping_sub(5),
                "mode 3 must leave the counter at the chain result"
            );
        } else {
            assert_eq!(after_c, 0, "modes 0,1,2,4 must leave the counter at 0");
        }
    }
}

// ===========================================================================
// Rows 11-15: the four low-level counter mutators
// ===========================================================================

fn diff_mut_chain(seed: u64, op: MutOp) {
    let p = pair();
    let mut rng = Rng::new(seed);
    let _g = guard();
    let start = rng.interesting_i32();
    assert_eq!(p.c.call_mut(MutOp::Reset, start), p.r.call_mut(MutOp::Reset, start));
    for _ in 0..ITERS {
        let v = rng.interesting_i32();
        let a = p.c.call_mut(op, v);
        let b = p.r.call_mut(op, v);
        assert_eq!(a, b, "{op:?}({v}) mismatch: C={a} Rust={b}");
    }
}

#[test]
fn cfg_11_increment() {
    diff_mut_chain(0x1111, MutOp::Increment);
}

#[test]
fn cfg_12_decrement() {
    diff_mut_chain(0x1212, MutOp::Decrement);
}

#[test]
fn cfg_13_multiply() {
    diff_mut_chain(0x1313, MutOp::Multiply);
}

#[test]
fn cfg_14_reset() {
    diff_mut_chain(0x1414, MutOp::Reset);
}

#[test]
fn cfg_15_mutator_random_pipeline() {
    let p = pair();
    let mut rng = Rng::new(0x1515);
    let _g = guard();
    let start = rng.interesting_i32();
    p.c.call_mut(MutOp::Reset, start);
    p.r.call_mut(MutOp::Reset, start);
    // Independent wrapping model of the composed pipeline.
    let mut model = start;
    for _ in 0..(ITERS * 8) {
        let op = MutOp::from_u32(rng.next_u32());
        let v = rng.interesting_i32();
        let a = p.c.call_mut(op, v);
        let b = p.r.call_mut(op, v);
        model = match op {
            MutOp::Increment => model.wrapping_add(v),
            MutOp::Decrement => model.wrapping_sub(v),
            MutOp::Multiply => model.wrapping_mul(v),
            MutOp::Reset => v,
        };
        assert_eq!(a, b, "pipeline {op:?}({v}): C={a} Rust={b}");
        assert_eq!(a, model, "pipeline {op:?}({v}) diverged from the wrapping model");
    }
}

// ===========================================================================
// Row 16: validate_uint16_range
// ===========================================================================

#[test]
fn cfg_16_validate_full_range() {
    let p = pair();
    const BOUNDARIES: [c_int; 11] = [
        i32::MIN,
        -2,
        -1,
        0,
        1,
        2,
        65534,
        65535,
        65536,
        65537,
        i32::MAX,
    ];
    for &v in BOUNDARIES.iter() {
        let a = unsafe { (p.c.validate_uint16_range)(v) };
        let b = unsafe { (p.r.validate_uint16_range)(v) };
        assert_eq!(a, b, "validate_uint16_range({v}): C={a} Rust={b}");
        assert_eq!(a, if (0..=65535).contains(&v) { 1 } else { 0 });
    }
    let mut rng = Rng::new(0x1616);
    for _ in 0..(ITERS * 20) {
        let v = rng.interesting_i32();
        let a = unsafe { (p.c.validate_uint16_range)(v) };
        let b = unsafe { (p.r.validate_uint16_range)(v) };
        assert_eq!(a, b, "validate_uint16_range({v}): C={a} Rust={b}");
    }
}

// ===========================================================================
// Rows 17-18: is_string_empty
// ===========================================================================

#[test]
fn cfg_17_is_string_empty_first_byte() {
    let p = pair();
    // Every possible non-NUL first byte, including the high-bit (signed char)
    // half of the range.
    for first in 1u8..=255 {
        let s = cstring(&[first, b'x', b'y']);
        let ptr = s.as_ptr() as *const c_char;
        let a = unsafe { (p.c.is_string_empty)(ptr) };
        let b = unsafe { (p.r.is_string_empty)(ptr) };
        assert_eq!(a, b, "is_string_empty(first byte 0x{first:02x}): C={a} Rust={b}");
        assert_eq!(a, 0);
    }
}

#[test]
fn cfg_18_is_string_empty_shapes() {
    let p = pair();
    let mut rng = Rng::new(0x1818);
    // Fixed shapes first.
    let fixed: Vec<Vec<u8>> = vec![
        cstring(b""),
        cstring(b"a"),
        cstring(b"Hello, World!"),
        vec![0, b'a', b'b', 0],      // NUL at byte 0, data after it
        vec![b'a', 0, b'b', 0],      // NUL at byte 1
        cstring(&[0xFF, 0x80, 0x01]),
    ];
    for s in &fixed {
        let ptr = s.as_ptr() as *const c_char;
        let a = unsafe { (p.c.is_string_empty)(ptr) };
        let b = unsafe { (p.r.is_string_empty)(ptr) };
        assert_eq!(a, b, "is_string_empty({s:?}): C={a} Rust={b}");
    }
    // Random lengths / contents.
    for _ in 0..ITERS {
        let len = rng.below(24) as usize;
        let mut bytes: Vec<u8> = (0..len).map(|_| rng.nonzero_byte()).collect();
        // Sometimes make it empty, sometimes plant a NUL somewhere.
        match rng.below(4) {
            0 => bytes.clear(),
            1 if !bytes.is_empty() => {
                let i = rng.below(bytes.len() as u32) as usize;
                bytes[i] = 0;
            }
            _ => {}
        }
        let s = cstring(&bytes);
        let ptr = s.as_ptr() as *const c_char;
        let a = unsafe { (p.c.is_string_empty)(ptr) };
        let b = unsafe { (p.r.is_string_empty)(ptr) };
        assert_eq!(a, b, "is_string_empty({bytes:?}): C={a} Rust={b}");
    }
}

// ===========================================================================
// Rows 19-21: find_char_in_buffer
// ===========================================================================

/// Compare the two libraries' `find_char_in_buffer` on the *same* caller-owned
/// buffer; results are compared as offsets (or `None`).
fn diff_find(buf: &[u8], size: usize, target: u8) -> Option<usize> {
    let p = pair();
    let base = buf.as_ptr() as *const c_char;
    let a = unsafe { (p.c.find_char_in_buffer)(base, size, target as i8 as c_char) };
    let b = unsafe { (p.r.find_char_in_buffer)(base, size, target as i8 as c_char) };
    assert_eq!(
        a.is_null(),
        b.is_null(),
        "find_char_in_buffer(len={}, size={size}, target=0x{target:02x}) nullness: C={a:?} Rust={b:?}",
        buf.len()
    );
    if a.is_null() {
        return None;
    }
    let oa = unsafe { a.offset_from(base as *mut c_char) };
    let ob = unsafe { b.offset_from(base as *mut c_char) };
    assert_eq!(
        oa, ob,
        "find_char_in_buffer(len={}, size={size}, target=0x{target:02x}) offset: C={oa} Rust={ob}",
        buf.len()
    );
    assert!(oa >= 0 && (oa as usize) < buf.len(), "returned pointer must be inside the buffer");
    Some(oa as usize)
}

#[test]
fn cfg_19_find_char_hit_positions() {
    let mut rng = Rng::new(0x1919);
    for _ in 0..ITERS {
        let len = (rng.below(40) + 1) as usize;
        // Fill with a byte we can then guarantee is unique-ish.
        let filler = 0x41u8;
        let target = 0x5Au8; // 'Z'
        for pos in [0usize, len / 2, len - 1] {
            let mut buf = vec![filler; len];
            buf[pos] = target;
            let got = diff_find(&buf, len, target);
            assert_eq!(got, Some(pos), "hit at {pos} of {len}");
        }
    }
}

#[test]
fn cfg_20_find_char_random() {
    let mut rng = Rng::new(0x2020);
    for _ in 0..(ITERS * 4) {
        let len = (rng.below(64) + 1) as usize;
        let buf: Vec<u8> = (0..len).map(|_| rng.next_u32() as u8).collect();
        let target = rng.next_u32() as u8;
        let size = rng.below(len as u32 + 1) as usize; // 0..=len
        let got = diff_find(&buf, size, target);
        let expect = buf[..size].iter().position(|&b| b == target);
        assert_eq!(got, expect, "memchr model mismatch (len={len}, size={size}, target=0x{target:02x})");
    }
}

#[test]
fn cfg_21_find_char_multi_and_nul() {
    let mut rng = Rng::new(0x2121);
    // Multiple occurrences: first match wins.
    for _ in 0..ITERS {
        let len = (rng.below(30) + 4) as usize;
        let target = rng.nonzero_byte();
        let mut buf: Vec<u8> = (0..len).map(|_| target).collect();
        let first = rng.below(len as u32) as usize;
        for i in 0..first {
            buf[i] = target.wrapping_add(1).max(1);
        }
        // Guard against the "different" filler colliding with the target.
        if buf[..first].iter().any(|&b| b == target) {
            continue;
        }
        assert_eq!(diff_find(&buf, len, target), Some(first));
    }
    // target == '\0' matching an embedded NUL.
    for pos in 0..8usize {
        let mut buf = vec![b'q'; 8];
        buf[pos] = 0;
        assert_eq!(diff_find(&buf, 8, 0), Some(pos));
    }
    // Length-1 buffer, hit and miss.
    assert_eq!(diff_find(&[b'k'], 1, b'k'), Some(0));
    assert_eq!(diff_find(&[b'k'], 1, b'j'), None);
    // Large buffer.
    let mut big = vec![0xAAu8; 100_000];
    big[99_999] = 0x11;
    assert_eq!(diff_find(&big, big.len(), 0x11), Some(99_999));
}

// ===========================================================================
// Row 22: create_buffer
// ===========================================================================

fn diff_create(bytes: &[u8]) {
    let p = pair();
    let src = cstring(bytes);
    let sp = src.as_ptr() as *const c_char;
    let a = unsafe { (p.c.create_buffer)(sp) };
    let b = unsafe { (p.r.create_buffer)(sp) };
    assert!(!a.is_null(), "C create_buffer returned NULL for {} bytes", bytes.len());
    assert!(!b.is_null(), "Rust create_buffer returned NULL for {} bytes", bytes.len());
    assert_ne!(a as *const c_char, sp, "must be a fresh allocation");
    assert_ne!(b as *const c_char, sp, "must be a fresh allocation");
    let ba = unsafe { c_bytes(a) };
    let bb = unsafe { c_bytes(b) };
    assert_eq!(ba, bb, "create_buffer contents differ");
    assert_eq!(ba.as_slice(), bytes, "create_buffer must copy the input verbatim");
    // Both pointers must come from libc malloc, i.e. be free()-able.
    unsafe {
        c_free(a);
        c_free(b);
    }
}

#[test]
fn cfg_22_create_buffer_shapes() {
    diff_create(b"");
    diff_create(b"x");
    diff_create(b"Testing malloc and free");
    diff_create(b"Search for character X in this buffer");
    // Every non-NUL byte value in one string (signed-char range included).
    let all: Vec<u8> = (1u8..=255).collect();
    diff_create(&all);
    // Random lengths / contents.
    let mut rng = Rng::new(0x2222);
    for _ in 0..ITERS {
        let len = rng.below(512) as usize;
        let bytes: Vec<u8> = (0..len).map(|_| rng.nonzero_byte()).collect();
        diff_create(&bytes);
    }
    // One long allocation.
    diff_create(&vec![b'L'; 65_536]);
}

// ===========================================================================
// Rows 23-24: apply_operation (indirect call through operation_func)
// ===========================================================================

#[test]
fn cfg_23_apply_operation_each_op() {
    let p = pair();
    let mut rng = Rng::new(0x2323);
    let _g = guard();
    for op in MutOp::ALL {
        let start = rng.interesting_i32();
        p.c.call_mut(MutOp::Reset, start);
        p.r.call_mut(MutOp::Reset, start);
        for _ in 0..ITERS {
            let v = rng.interesting_i32();
            // Each library gets its OWN function pointer: the two `.so`s have
            // separate `static counter` storage.
            let a = unsafe { (p.c.apply_operation)(p.c.op_ptr(op), v) };
            let b = unsafe { (p.r.apply_operation)(p.r.op_ptr(op), v) };
            assert_eq!(a, b, "apply_operation({op:?}, {v}): C={a} Rust={b}");
        }
    }
}

#[test]
fn cfg_24_apply_operation_pipeline() {
    let p = pair();
    let mut rng = Rng::new(0x2424);
    let _g = guard();
    let start = rng.interesting_i32();
    p.c.call_mut(MutOp::Reset, start);
    p.r.call_mut(MutOp::Reset, start);
    let mut model = start;
    for _ in 0..(ITERS * 8) {
        let op = MutOp::from_u32(rng.next_u32());
        let v = rng.interesting_i32();
        let a = unsafe { (p.c.apply_operation)(p.c.op_ptr(op), v) };
        let b = unsafe { (p.r.apply_operation)(p.r.op_ptr(op), v) };
        model = match op {
            MutOp::Increment => model.wrapping_add(v),
            MutOp::Decrement => model.wrapping_sub(v),
            MutOp::Multiply => model.wrapping_mul(v),
            MutOp::Reset => v,
        };
        assert_eq!(a, b, "apply_operation pipeline {op:?}({v}): C={a} Rust={b}");
        assert_eq!(a, model, "apply_operation pipeline diverged from the model");
    }
}

// ===========================================================================
// Row 25: create_buffer -> find_char_in_buffer composed by hand
// ===========================================================================

#[test]
fn cfg_25_create_then_find_pipeline() {
    let p = pair();
    let mut rng = Rng::new(0x2525);
    for _ in 0..(ITERS * 2) {
        let len = (rng.below(64) + 1) as usize;
        let bytes: Vec<u8> = (0..len).map(|_| rng.nonzero_byte()).collect();
        let src = cstring(&bytes);
        let sp = src.as_ptr() as *const c_char;
        let target = if rng.below(2) == 0 {
            bytes[rng.below(len as u32) as usize]
        } else {
            rng.nonzero_byte()
        };

        let cb = unsafe { (p.c.create_buffer)(sp) };
        let rb = unsafe { (p.r.create_buffer)(sp) };
        assert!(!cb.is_null() && !rb.is_null());

        let csz = unsafe { c_strlen(cb) };
        let rsz = unsafe { c_strlen(rb) };
        assert_eq!(csz, rsz, "strlen of the two fresh buffers must match");

        let cf = unsafe { (p.c.find_char_in_buffer)(cb, csz, target as i8 as c_char) };
        let rf = unsafe { (p.r.find_char_in_buffer)(rb, rsz, target as i8 as c_char) };
        assert_eq!(cf.is_null(), rf.is_null(), "pipeline nullness (target=0x{target:02x})");
        if !cf.is_null() {
            let oc = unsafe { cf.offset_from(cb) };
            let or = unsafe { rf.offset_from(rb) };
            assert_eq!(oc, or, "pipeline offset (target=0x{target:02x})");
            assert_eq!(
                oc as usize,
                bytes.iter().position(|&x| x == target).unwrap(),
                "pipeline offset vs model"
            );
        }
        unsafe {
            c_free(cb);
            c_free(rb);
        }
    }
}

// ===========================================================================
// Row 26: apply_operation is a pure indirect call — it must work with a
// callback that does not belong to the library being called.
// ===========================================================================

#[test]
fn cfg_26_apply_operation_foreign_callback() {
    let p = pair();
    let mut rng = Rng::new(0x2626);
    let _g = guard();
    for op in MutOp::ALL {
        for _ in 0..ITERS {
            let seed = rng.interesting_i32();
            let v = rng.interesting_i32();

            // C's apply_operation driving RUST's counter, and Rust's driving
            // C's. Both must give the same answer as the same op applied to the
            // same starting value, which proves neither `apply_operation`
            // smuggles in state of its own.
            p.r.call_mut(MutOp::Reset, seed);
            let via_c = unsafe { (p.c.apply_operation)(p.r.op_ptr(op), v) };

            p.c.call_mut(MutOp::Reset, seed);
            let via_r = unsafe { (p.r.apply_operation)(p.c.op_ptr(op), v) };

            assert_eq!(
                via_c, via_r,
                "cross-library apply_operation({op:?}, {v}) from {seed}: C-caller={via_c} Rust-caller={via_r}"
            );
            let model = match op {
                MutOp::Increment => seed.wrapping_add(v),
                MutOp::Decrement => seed.wrapping_sub(v),
                MutOp::Multiply => seed.wrapping_mul(v),
                MutOp::Reset => v,
            };
            assert_eq!(via_c, model, "cross-library call diverged from the model");
        }
    }
}

// ===========================================================================
// Guard: the exported `charinbuf` must not depend on anything but its four
// arguments — calling it twice with identical arguments must be idempotent even
// though the library holds mutable global state.
// ===========================================================================

#[test]
fn cfg_27_charinbuf_is_argument_determined() {
    let mut rng = Rng::new(0x2727);
    let _g = guard();
    for _ in 0..ITERS {
        let (mode, value, opt1, opt2) = (
            rng.range_i32(-2, 6),
            rng.interesting_i32(),
            rng.interesting_i32(),
            rng.interesting_i32(),
        );
        // Perturb the global counter between the two rounds; `charinbuf` zeroes
        // it on entry, so the observable result must be unchanged.
        let (rc1, out1) = diff_charinbuf_locked(mode, value, opt1, opt2);
        let p = pair();
        let noise = rng.interesting_i32();
        p.c.call_mut(MutOp::Reset, noise);
        p.r.call_mut(MutOp::Reset, noise);
        let (rc2, out2) = diff_charinbuf_locked(mode, value, opt1, opt2);
        assert_eq!(rc1, rc2, "charinbuf({mode}, {value}, {opt1}, {opt2}) not idempotent");
        assert_eq!(show(&out1), show(&out2), "charinbuf stdout not idempotent");
    }
}
