// Phase B — valid-path differential tests, one test per row of CONFIGS.md.
//
// Everything is driven through the two `.so`s' exported symbols (see
// tests/common/mod.rs); no Rust function is called directly.
//
// Ordering note: the library carries a hidden `static counter`, so every
// state-touching test takes `gate()` and normalizes the counter first.

mod common;

use common::*;
use std::ffi::{c_char, c_int, c_void};

/// Number of randomized inputs per property-style row.
const N: usize = 512;

// ===========================================================================
// Rows 1-2 — validate_uint16_range
// ===========================================================================

#[test]
fn cfg_01_validate_uint16_range_edges() {
    let _g = gate();
    for v in [
        i32::MIN,
        i32::MIN + 1,
        -65536,
        -65535,
        -2,
        -1,
        0,
        1,
        2,
        3,
        32767,
        32768,
        65533,
        65534,
        65535,
        65536,
        65537,
        i32::MAX - 1,
        i32::MAX,
    ] {
        diff_validate(v);
    }
}

#[test]
fn cfg_02_validate_uint16_range_randomized() {
    let _g = gate();
    let mut rng = Rng::new(0x5645_4C49_4441_5445);
    for _ in 0..N * 8 {
        diff_validate(rng.interesting_i32());
    }
    // Deliberately oversample the in-range window so the accept branch is hit
    // as often as the reject branches.
    for _ in 0..N * 4 {
        diff_validate(rng.in_range_i32(-4, 65539));
    }
}

// ===========================================================================
// Rows 3-5 — is_string_empty
// ===========================================================================

#[test]
fn cfg_03_is_string_empty_empty() {
    let _g = gate();
    diff_is_string_empty(b"\0");
}

#[test]
fn cfg_04_is_string_empty_every_first_byte() {
    let _g = gate();
    // Every possible non-zero first byte, including 0x80..=0xFF which are
    // negative when `char` is signed (x86-64) — the `if (*str)` test must still
    // report "non-empty".
    for b in 1u8..=255 {
        let s = [b, 0u8];
        diff_is_string_empty(&s);
        // And the same byte followed by more content.
        let s2 = [b, b'x', b'y', 0u8];
        diff_is_string_empty(&s2);
    }
}

#[test]
fn cfg_05_is_string_empty_embedded_nul_and_long() {
    let _g = gate();
    // strlen and the `*str` dereference disagree here.
    diff_is_string_empty(b"a\0b\0");
    diff_is_string_empty(b"\0ab\0");
    diff_is_string_empty(b"\0\0\0\0");
    diff_is_string_empty(b"Hello, World!\0");

    let mut rng = Rng::new(0x1515_7EEE_0000_0001);
    for _ in 0..N {
        let len = rng.below(64);
        let mut s: Vec<u8> = (0..len).map(|_| rng.next_u8()).collect();
        s.push(0);
        diff_is_string_empty(&s);
    }
}

// ===========================================================================
// Rows 6-8 — create_buffer
// ===========================================================================

#[test]
fn cfg_06_create_buffer_empty_string() {
    let _g = gate();
    diff_create_buffer(b"\0");
}

#[test]
fn cfg_07_create_buffer_randomized_lengths() {
    let _g = gate();
    let mut rng = Rng::new(0x0C0F_FEE0_BEEF_0007);
    for _ in 0..N {
        let len = rng.below(513);
        let mut s: Vec<u8> = Vec::with_capacity(len + 1);
        for _ in 0..len {
            // Any non-NUL byte; NUL would terminate the string early.
            let mut b = rng.next_u8();
            if b == 0 {
                b = 1;
            }
            s.push(b);
        }
        s.push(0);
        diff_create_buffer(&s);
    }
    // The exact literals charinbuf uses internally.
    diff_create_buffer(b"Testing malloc and free\0");
    diff_create_buffer(b"Search for character X in this buffer\0");
}

#[test]
fn cfg_08_create_buffer_high_bit_and_non_utf8() {
    let _g = gate();
    // Invalid UTF-8: lone continuation bytes, truncated sequences, 0xFF/0xFE.
    diff_create_buffer(b"\x80\x81\x82\0");
    diff_create_buffer(b"\xff\xfe\xfd\0");
    diff_create_buffer(b"\xc3\0");
    diff_create_buffer(b"\xe2\x82\0");
    diff_create_buffer(b"caf\xe9 \xff\x80latin1\0");

    for b in 1u8..=255 {
        let s = [b, b.wrapping_add(1).max(1), 0u8];
        diff_create_buffer(&s);
    }
}

// ===========================================================================
// Rows 9-15 — find_char_in_buffer
// ===========================================================================

#[test]
fn cfg_09_find_char_zero_size_with_target_at_front() {
    let _g = gate();
    let buf = b"Xabc";
    // memchr inspects nothing, so even a match at [0] is not found.
    diff_find_char(buf, 0, b'X');
    diff_find_char(b"\0\0\0", 0, 0);
    diff_find_char(&[], 0, b'X');
}

#[test]
fn cfg_10_find_char_size_one() {
    let _g = gate();
    let buf = b"Xy";
    diff_find_char(buf, 1, b'X'); // hit at 0
    diff_find_char(buf, 1, b'y'); // miss: outside the window
    diff_find_char(buf, 2, b'y'); // hit at 1
}

#[test]
fn cfg_11_find_char_first_interior_last() {
    let _g = gate();
    let buf = b"ABCDEFGHIJ";
    diff_find_char(buf, buf.len(), b'A'); // first
    diff_find_char(buf, buf.len(), b'E'); // interior
    diff_find_char(buf, buf.len(), b'J'); // last
    diff_find_char(buf, buf.len(), b'K'); // absent

    // Duplicate occurrences: the *first* must be returned.
    let dup = b"abcabcabc";
    for t in [b'a', b'b', b'c'] {
        for size in 0..=dup.len() {
            diff_find_char(dup, size, t);
        }
    }
}

#[test]
fn cfg_12_find_char_size_window_sweep() {
    let _g = gate();
    let buf = b"0123456789";
    // For each target, sweep `size` across the match position boundary.
    for (i, &t) in buf.iter().enumerate() {
        for size in 0..=buf.len() {
            // hit iff size > i
            diff_find_char(buf, size, t);
            let _ = i;
        }
    }
}

#[test]
fn cfg_13_find_char_nul_target() {
    let _g = gate();
    // A NUL inside the window is a legitimate hit: this is memchr, not strchr.
    let buf = b"abc\0def";
    for size in 0..=buf.len() {
        diff_find_char(buf, size, 0);
    }
    diff_find_char(b"\0", 1, 0);
}

#[test]
fn cfg_14_find_char_high_bit_targets() {
    let _g = gate();
    // 0x80..=0xFF are negative `char`s on x86-64: C sign-extends to `int`,
    // Rust zero-extends via `target as u8`; memchr masks to `unsigned char`, so
    // both must agree. Check every one, present and absent.
    for b in 0x80u8..=0xFF {
        let buf = [b'a', b, b'z', b];
        diff_find_char(&buf, buf.len(), b);
        diff_find_char(&buf, 1, b); // present but outside the window
        let absent = [b'a', b'b', b'c'];
        diff_find_char(&absent, absent.len(), b);
    }
    // Also the full byte range in one buffer.
    let all: Vec<u8> = (0u8..=255).collect();
    for b in 0u8..=255 {
        diff_find_char(&all, all.len(), b);
    }
}

#[test]
fn cfg_15_find_char_randomized() {
    let _g = gate();
    let mut rng = Rng::new(0x4D45_4D43_4852_0015);
    for _ in 0..N * 4 {
        let len = rng.below(257);
        let buf: Vec<u8> = (0..len).map(|_| rng.next_u8()).collect();
        let size = rng.below(len + 1);
        // Half the time pick a byte that is actually present, so hits and
        // misses are both common.
        let target = if len > 0 && rng.next_u64() % 2 == 0 {
            buf[rng.below(len)]
        } else {
            rng.next_u8()
        };
        diff_find_char(&buf, size, target);
    }
    // Small alphabet => many duplicates => first-match position matters.
    for _ in 0..N * 2 {
        let len = rng.below(65);
        let buf: Vec<u8> = (0..len).map(|_| (rng.next_u8() % 3) + b'a').collect();
        let size = rng.below(len + 1);
        diff_find_char(&buf, size, (rng.next_u8() % 4) + b'a');
    }
}

// ===========================================================================
// Rows 16-21 — the hidden static counter
// ===========================================================================

#[test]
fn cfg_16_reset_counter_randomized() {
    let _g = gate();
    let (c, r) = apis();
    let mut rng = Rng::new(0x2E5E_7000_0000_0016);
    for _ in 0..N * 2 {
        let v = rng.interesting_i32();
        let cv = (c.reset_counter)(v);
        let rv = (r.reset_counter)(v);
        assert_eq!(cv, rv, "reset_counter({v}): C={cv} Rust={rv}");
        // And the state it left behind.
        assert_eq!(
            c.peek_counter(),
            r.peek_counter(),
            "state after reset_counter({v})"
        );
    }
}

#[test]
fn cfg_17_increment_counter_accumulates_and_wraps() {
    let _g = gate();
    let (c, r) = apis();
    let mut rng = Rng::new(0x1_C0_0000_0000_0017);

    for base in [0, 1, -1, i32::MAX, i32::MIN, i32::MAX - 3, i32::MIN + 3] {
        (c.reset_counter)(base);
        (r.reset_counter)(base);
        for step in [0, 1, -1, 2, i32::MAX, i32::MIN, 7] {
            let cv = (c.increment_counter)(step);
            let rv = (r.increment_counter)(step);
            assert_eq!(
                cv, rv,
                "increment_counter({step}) from base {base}: C={cv} Rust={rv}"
            );
        }
    }

    (c.reset_counter)(0);
    (r.reset_counter)(0);
    for _ in 0..N * 2 {
        let v = rng.interesting_i32();
        let cv = (c.increment_counter)(v);
        let rv = (r.increment_counter)(v);
        assert_eq!(cv, rv, "increment_counter({v}): C={cv} Rust={rv}");
    }
}

#[test]
fn cfg_18_decrement_counter_accumulates_and_wraps() {
    let _g = gate();
    let (c, r) = apis();
    let mut rng = Rng::new(0x1_DEC_0000_0000_018);

    for base in [0, 1, -1, i32::MAX, i32::MIN, i32::MIN + 3] {
        (c.reset_counter)(base);
        (r.reset_counter)(base);
        for step in [0, 1, -1, 2, i32::MAX, i32::MIN, 5] {
            let cv = (c.decrement_counter)(step);
            let rv = (r.decrement_counter)(step);
            assert_eq!(
                cv, rv,
                "decrement_counter({step}) from base {base}: C={cv} Rust={rv}"
            );
        }
    }

    (c.reset_counter)(0);
    (r.reset_counter)(0);
    for _ in 0..N * 2 {
        let v = rng.interesting_i32();
        let cv = (c.decrement_counter)(v);
        let rv = (r.decrement_counter)(v);
        assert_eq!(cv, rv, "decrement_counter({v}): C={cv} Rust={rv}");
    }
}

#[test]
fn cfg_19_multiply_counter_overflow() {
    let _g = gate();
    let (c, r) = apis();
    let mut rng = Rng::new(0x1_3D_0000_0000_0019);

    for base in [0, 1, -1, 2, -2, 3, 65535, 65536, i32::MAX, i32::MIN, 46341] {
        for factor in [0, 1, -1, 2, -2, 46341, 65536, i32::MAX, i32::MIN, 1_000_003] {
            (c.reset_counter)(base);
            (r.reset_counter)(base);
            let cv = (c.multiply_counter)(factor);
            let rv = (r.multiply_counter)(factor);
            assert_eq!(
                cv, rv,
                "multiply_counter({factor}) from base {base}: C={cv} Rust={rv}"
            );
        }
    }

    for _ in 0..N * 2 {
        let base = rng.interesting_i32();
        let factor = rng.interesting_i32();
        (c.reset_counter)(base);
        (r.reset_counter)(base);
        let cv = (c.multiply_counter)(factor);
        let rv = (r.multiply_counter)(factor);
        assert_eq!(
            cv, rv,
            "multiply_counter({factor}) from base {base}: C={cv} Rust={rv}"
        );
    }
}

#[test]
fn cfg_20_counter_interleaved_random_sequences() {
    let _g = gate();
    let (c, r) = apis();
    let mut rng = Rng::new(0x5E_C0_0000_0000_20);

    for seq in 0..N / 8 {
        let start = rng.interesting_i32();
        (c.reset_counter)(start);
        (r.reset_counter)(start);
        for step in 0..64 {
            let v = rng.interesting_i32();
            let which = rng.below(4);
            let (cv, rv) = match which {
                0 => ((c.increment_counter)(v), (r.increment_counter)(v)),
                1 => ((c.decrement_counter)(v), (r.decrement_counter)(v)),
                2 => ((c.multiply_counter)(v), (r.multiply_counter)(v)),
                _ => ((c.reset_counter)(v), (r.reset_counter)(v)),
            };
            assert_eq!(
                cv, rv,
                "seq {seq} step {step}: op {which} value {v}: C={cv} Rust={rv}"
            );
        }
    }
}

#[test]
fn cfg_21_counter_state_interacts_with_charinbuf() {
    let _g = gate();
    let (c, r) = apis();
    let mut rng = Rng::new(0x21_2121_2121_2121);

    for _ in 0..N / 8 {
        // Dirty the hidden counter through the low-level API...
        let dirty = rng.interesting_i32();
        (c.reset_counter)(dirty);
        (r.reset_counter)(dirty);
        (c.multiply_counter)(3);
        (r.multiply_counter)(3);

        // ...then run mode 3, which starts with `counter = 0`, so the dirty
        // value must not influence the result at all.
        let (v, o1, o2) = (
            rng.interesting_i32(),
            rng.interesting_i32(),
            rng.interesting_i32(),
        );
        diff_charinbuf_with_state(3, v, o1, o2);

        // Any mode zeroes the counter on entry; verify through the low-level API.
        (c.reset_counter)(dirty);
        (r.reset_counter)(dirty);
        diff_charinbuf(0, rng.interesting_i32(), 0, 0);
        assert_eq!(
            c.peek_counter(),
            r.peek_counter(),
            "counter after mode 0 (entry reset)"
        );
    }
}

// ===========================================================================
// Rows 22-23 — apply_operation
// ===========================================================================

#[test]
fn cfg_22_apply_operation_own_callbacks() {
    let _g = gate();
    let (c, r) = apis();
    let mut rng = Rng::new(0x22_A9_0000_0000_22);

    let ops: [(&str, *const c_void, *const c_void); 4] = [
        ("reset_counter", c.p_reset, r.p_reset),
        ("increment_counter", c.p_increment, r.p_increment),
        ("multiply_counter", c.p_multiply, r.p_multiply),
        ("decrement_counter", c.p_decrement, r.p_decrement),
    ];

    for (name, cop, rop) in ops {
        for _ in 0..N / 4 {
            // Normalize so the callback's effect is deterministic.
            let base = rng.interesting_i32();
            (c.reset_counter)(base);
            (r.reset_counter)(base);
            let v = rng.interesting_i32();
            diff_apply_operation_raw(cop, rop, v, name);
            assert_eq!(
                c.peek_counter(),
                r.peek_counter(),
                "state after apply_operation({name}, {v}) from base {base}"
            );
        }
    }

    // Chain all four through apply_operation, mirroring what charinbuf mode 3
    // does, but driving the low-level entry point directly.
    for _ in 0..N / 4 {
        let (v, o1, o2) = (
            rng.interesting_i32(),
            rng.interesting_i32(),
            rng.interesting_i32(),
        );
        diff_apply_operation_raw(c.p_reset, r.p_reset, v, "chain/reset");
        diff_apply_operation_raw(c.p_increment, r.p_increment, o1, "chain/increment");
        diff_apply_operation_raw(c.p_multiply, r.p_multiply, o2, "chain/multiply");
        diff_apply_operation_raw(c.p_decrement, r.p_decrement, 5, "chain/decrement");
    }
}

/// A callback that lives in the test binary — external to both `.so`s. Pins the
/// raw `int (*)(int)` ABI and proves Rust's `Option<extern "C" fn>` is passed as
/// a plain pointer.
extern "C" fn foreign_op(v: c_int) -> c_int {
    v.wrapping_mul(3).wrapping_add(7)
}

extern "C" fn foreign_identity(v: c_int) -> c_int {
    v
}

extern "C" fn foreign_minus_one(_v: c_int) -> c_int {
    -1
}

#[test]
fn cfg_23_apply_operation_foreign_callback() {
    let _g = gate();
    let mut rng = Rng::new(0x23_F0_0000_0000_23);

    let cbs: [(&str, extern "C" fn(c_int) -> c_int); 3] = [
        ("foreign_op", foreign_op),
        ("foreign_identity", foreign_identity),
        ("foreign_minus_one", foreign_minus_one),
    ];

    for (name, f) in cbs {
        let p = f as *const c_void;
        for v in [0, 1, -1, i32::MAX, i32::MIN, 65535, 65536] {
            diff_apply_operation_raw(p, p, v, name);
        }
        for _ in 0..N {
            diff_apply_operation_raw(p, p, rng.interesting_i32(), name);
        }
    }
}

// ===========================================================================
// Silence check: none of the nine lower-level C functions writes to stdout.
// The fast diff helpers above rely on this, so it is pinned explicitly.
// ===========================================================================

#[test]
fn cfg_low_level_functions_write_nothing_to_stdout() {
    let _g = gate();
    let (c, r) = apis();
    let s = b"probe\0";
    let p = s.as_ptr().cast::<c_char>();

    for (label, api) in [("C", c), ("Rust", r)] {
        let (_, out) = capture(|| {
            (api.validate_uint16_range)(7);
            (api.validate_uint16_range)(-7);
            (api.reset_counter)(3);
            (api.increment_counter)(4);
            (api.decrement_counter)(2);
            (api.multiply_counter)(5);
            unsafe {
                (api.is_string_empty)(p);
                (api.is_string_empty)(std::ptr::null());
                let b = (api.create_buffer)(p);
                (api.find_char_in_buffer)(p, 5, b'o' as c_char);
                (api.find_char_in_buffer)(std::ptr::null(), 5, b'o' as c_char);
                if !b.is_null() {
                    libc_free(b);
                }
                (api.apply_operation)(std::ptr::null(), 1);
                (api.apply_operation)(api.p_increment, 1);
            }
        });
        assert!(
            out.is_empty(),
            "{label} lower-level functions unexpectedly printed: \"{}\"",
            show(&out)
        );
    }
}

// ===========================================================================
// Rows 24-35 — charinbuf (the switch-driven driver). Return value AND the
// exact stdout bytes are compared for every call.
// ===========================================================================

#[test]
fn cfg_24_charinbuf_mode0_range_edges() {
    let _g = gate();
    for v in [
        i32::MIN,
        i32::MIN + 1,
        -65536,
        -2,
        -1,
        0,
        1,
        2,
        32768,
        65534,
        65535,
        65536,
        65537,
        i32::MAX - 1,
        i32::MAX,
    ] {
        diff_charinbuf_with_state(0, v, 0, 0);
    }
}

#[test]
fn cfg_25_charinbuf_mode0_randomized() {
    let _g = gate();
    let mut rng = Rng::new(0x0D24_0000_0000_0025);
    for _ in 0..N {
        // opt1/opt2 are unused by mode 0; randomize them to prove it.
        diff_charinbuf_with_state(
            0,
            rng.interesting_i32(),
            rng.interesting_i32(),
            rng.interesting_i32(),
        );
    }
    for _ in 0..N / 2 {
        diff_charinbuf_with_state(
            0,
            rng.in_range_i32(-4, 65539),
            rng.next_i32(),
            rng.next_i32(),
        );
    }
}

#[test]
fn cfg_26_charinbuf_mode1_ignores_params() {
    let _g = gate();
    let mut rng = Rng::new(0x0D26_0000_0000_0026);
    diff_charinbuf_with_state(1, 0, 0, 0);
    for _ in 0..N / 2 {
        diff_charinbuf_with_state(
            1,
            rng.interesting_i32(),
            rng.interesting_i32(),
            rng.interesting_i32(),
        );
    }
}

#[test]
fn cfg_27_charinbuf_mode2_alloc_path() {
    let _g = gate();
    let mut rng = Rng::new(0x0D27_0000_0000_0027);
    diff_charinbuf_with_state(2, 0, 0, 0);
    for _ in 0..N / 2 {
        diff_charinbuf_with_state(
            2,
            rng.interesting_i32(),
            rng.interesting_i32(),
            rng.interesting_i32(),
        );
    }
    // Pin the exact expected bytes, so a change in *both* implementations
    // (e.g. a wrong %zu width) would still be caught.
    let (c, _r) = apis();
    let (v, out) = capture(|| (c.charinbuf)(2, 0, 0, 0));
    assert_eq!(v, 23, "strlen(\"Testing malloc and free\")");
    assert_eq!(
        out,
        b"Mode 2: Dynamic memory allocation and free\n\
          Buffer allocated: 'Testing malloc and free'\n\
          Buffer length: 23\n\
          Buffer freed successfully\n"
            .iter()
            .copied()
            .filter(|_| true)
            .collect::<Vec<u8>>(),
        "C mode 2 stdout was \"{}\"",
        show(&out)
    );
}

#[test]
fn cfg_28_charinbuf_mode3_small_operands() {
    let _g = gate();
    for value in [-3, -1, 0, 1, 2, 7, 100] {
        for opt1 in [-5, -1, 0, 1, 3, 11] {
            for opt2 in [-2, -1, 0, 1, 2, 4] {
                diff_charinbuf_with_state(3, value, opt1, opt2);
            }
        }
    }
}

#[test]
fn cfg_29_charinbuf_mode3_overflow_operands() {
    let _g = gate();
    // Chosen to overflow at `+=` and/or at `*=`. Signed overflow is UB in C but
    // the built .so wraps; the Rust `wrapping_*` must reproduce it exactly.
    let vals = [
        0,
        1,
        -1,
        2,
        -2,
        5,
        46341, // 46341^2 > INT_MAX
        65536,
        1_000_003,
        i32::MAX,
        i32::MAX - 1,
        i32::MIN,
        i32::MIN + 1,
        -46341,
    ];
    for &value in &vals {
        for &opt1 in &vals {
            for &opt2 in &vals {
                diff_charinbuf_with_state(3, value, opt1, opt2);
            }
        }
    }
}

#[test]
fn cfg_30_charinbuf_mode3_randomized() {
    let _g = gate();
    let mut rng = Rng::new(0x0D30_0000_0000_0030);
    for _ in 0..N {
        diff_charinbuf_with_state(
            3,
            rng.interesting_i32(),
            rng.interesting_i32(),
            rng.interesting_i32(),
        );
    }
    for _ in 0..N {
        diff_charinbuf_with_state(3, rng.next_i32(), rng.next_i32(), rng.next_i32());
    }
}

#[test]
fn cfg_31_charinbuf_mode3_repeated_is_independent() {
    let _g = gate();
    let (c, r) = apis();
    let mut rng = Rng::new(0x0D31_0000_0000_0031);

    for _ in 0..N / 8 {
        let (v, o1, o2) = (
            rng.interesting_i32(),
            rng.interesting_i32(),
            rng.interesting_i32(),
        );
        // First call leaves the counter dirty.
        let c1 = capture(|| (c.charinbuf)(3, v, o1, o2));
        let r1 = capture(|| (r.charinbuf)(3, v, o1, o2));
        assert_same_call("mode 3 first call", c1.clone(), r1.clone());

        // Second, identical call must produce identical output because
        // `counter = 0` runs on entry.
        let c2 = capture(|| (c.charinbuf)(3, v, o1, o2));
        let r2 = capture(|| (r.charinbuf)(3, v, o1, o2));
        assert_same_call("mode 3 second call", c2.clone(), r2.clone());
        assert_eq!(
            c1.1, c2.1,
            "C mode 3 is not idempotent — entry reset missing?"
        );
        assert_eq!(
            r1.1, r2.1,
            "Rust mode 3 is not idempotent — entry reset missing?"
        );
    }
}

#[test]
fn cfg_32_charinbuf_mode4_memchr_path() {
    let _g = gate();
    let mut rng = Rng::new(0x0D32_0000_0000_0032);
    diff_charinbuf_with_state(4, 0, 0, 0);
    for _ in 0..N / 2 {
        diff_charinbuf_with_state(
            4,
            rng.interesting_i32(),
            rng.interesting_i32(),
            rng.interesting_i32(),
        );
    }
    // Pin the exact position of 'X' in "Search for character X in this buffer".
    let (c, _r) = apis();
    let (v, out) = capture(|| (c.charinbuf)(4, 0, 0, 0));
    assert_eq!(v, 21, "index of 'X'");
    assert_eq!(
        out,
        b"Mode 4: Using memchr to find character\nSearching for 'X' in: 'Search for character X in this buffer'\nFound 'X' at position: 21\n".to_vec(),
        "C mode 4 stdout was \"{}\"",
        show(&out)
    );
}

#[test]
fn cfg_33_charinbuf_default_mode_randomized() {
    let _g = gate();
    let mut rng = Rng::new(0x0D33_0000_0000_0033);
    for m in [i32::MIN, i32::MIN + 1, -100, -2, -1, 5, 6, 7, 99, i32::MAX] {
        diff_charinbuf_with_state(m, 0, 0, 0);
    }
    for _ in 0..N {
        let mut m = rng.next_i32();
        if (0..=4).contains(&m) {
            m = m.wrapping_add(5); // force outside 0..=4
        }
        diff_charinbuf_with_state(m, rng.next_i32(), rng.next_i32(), rng.next_i32());
    }
}

#[test]
fn cfg_34_charinbuf_mode_param_cross_product() {
    let _g = gate();
    // Full cross-product of a mode sweep spanning the valid cases and both
    // sides of them, against several parameter triples.
    let triples: [(i32, i32, i32); 8] = [
        (0, 0, 0),
        (1, 1, 1),
        (-1, -1, -1),
        (65535, 2, 3),
        (65536, -2, 0),
        (i32::MAX, i32::MAX, i32::MAX),
        (i32::MIN, i32::MIN, i32::MIN),
        (7, 46341, 46341),
    ];
    for mode in -8..=12 {
        for &(v, o1, o2) in &triples {
            diff_charinbuf_with_state(mode, v, o1, o2);
        }
    }
}

#[test]
fn cfg_35_whole_library_random_call_sequence() {
    let _g = gate();
    let (c, r) = apis();
    let mut rng = Rng::new(0x0D35_0000_0000_0035);

    // Start both libraries from the same state.
    (c.reset_counter)(0);
    (r.reset_counter)(0);

    let strings: [&[u8]; 6] = [
        b"\0",
        b"a\0",
        b"Hello, World!\0",
        b"\x80\xff\0",
        b"Testing malloc and free\0",
        b"x\0y\0",
    ];

    for step in 0..N {
        match rng.below(10) {
            0 => {
                let v = rng.interesting_i32();
                let cv = (c.increment_counter)(v);
                let rv = (r.increment_counter)(v);
                assert_eq!(cv, rv, "step {step}: increment_counter({v})");
            }
            1 => {
                let v = rng.interesting_i32();
                let cv = (c.decrement_counter)(v);
                let rv = (r.decrement_counter)(v);
                assert_eq!(cv, rv, "step {step}: decrement_counter({v})");
            }
            2 => {
                let v = rng.interesting_i32();
                let cv = (c.multiply_counter)(v);
                let rv = (r.multiply_counter)(v);
                assert_eq!(cv, rv, "step {step}: multiply_counter({v})");
            }
            3 => {
                let v = rng.interesting_i32();
                let cv = (c.reset_counter)(v);
                let rv = (r.reset_counter)(v);
                assert_eq!(cv, rv, "step {step}: reset_counter({v})");
            }
            4 => diff_validate(rng.interesting_i32()),
            5 => diff_is_string_empty(strings[rng.below(strings.len())]),
            6 => diff_create_buffer(strings[rng.below(strings.len())]),
            7 => {
                let buf = strings[rng.below(strings.len())];
                diff_find_char(buf, rng.below(buf.len() + 1), rng.next_u8());
            }
            8 => {
                let ops = [
                    (c.p_reset, r.p_reset, "reset"),
                    (c.p_increment, r.p_increment, "increment"),
                    (c.p_multiply, r.p_multiply, "multiply"),
                    (c.p_decrement, r.p_decrement, "decrement"),
                    (
                        std::ptr::null::<c_void>(),
                        std::ptr::null::<c_void>(),
                        "null",
                    ),
                ];
                let (cop, rop, name) = ops[rng.below(ops.len())];
                diff_apply_operation_raw(cop, rop, rng.interesting_i32(), name);
            }
            _ => {
                // charinbuf resets the counter, so this deliberately perturbs
                // the shared state mid-sequence.
                let mode = rng.in_range_i32(-2, 7);
                diff_charinbuf(
                    mode,
                    rng.interesting_i32(),
                    rng.interesting_i32(),
                    rng.interesting_i32(),
                );
            }
        }

        // The hidden counter must agree after every single step.
        assert_eq!(
            c.peek_counter(),
            r.peek_counter(),
            "step {step}: hidden counter diverged"
        );
    }
}
