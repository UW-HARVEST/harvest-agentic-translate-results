// Phase B — valid-path differential tests, one test per row of CONFIGS.md.
//
// Every call goes through both shared objects via `libloading`; return values
// and (for `charinbuf`) the exact stdout byte stream are compared.

mod support;

use support::*;

// ---------------------------------------------------------------------------
// C1 — validate_uint16_range over the whole interesting domain + random
// ---------------------------------------------------------------------------
#[test]
fn c1_validate_uint16_range() {
    for v in [
        -1, 0, 1, 2, 254, 255, 256, 32767, 32768, 65534, 65535, 65536, 65537, i32::MAX, i32::MIN,
        i32::MIN + 1, i32::MAX - 1,
    ] {
        diff_validate(v);
    }
    let mut rng = Rng::new(SEED ^ 1);
    for _ in 0..4096 {
        diff_validate(rng.next_i32());
    }
    for _ in 0..512 {
        // Dense sampling around the boundary.
        diff_validate(rng.range_i32(-4, 65540));
    }
}

// ---------------------------------------------------------------------------
// C2 / C3 / C4 / C5 — is_string_empty
// ---------------------------------------------------------------------------
#[test]
fn c2_is_string_empty_empty() {
    diff_is_string_empty(b"\0");
}

#[test]
fn c3_is_string_empty_single_byte_all_values() {
    for b in 1u16..=255 {
        let s = [b as u8, 0];
        diff_is_string_empty(&s);
    }
}

#[test]
fn c4_is_string_empty_random_strings() {
    let mut rng = Rng::new(SEED ^ 2);
    for _ in 0..256 {
        let len = 1 + rng.below(64);
        let s = rng.cstring(len);
        diff_is_string_empty(&s);
    }
}

#[test]
fn c5_is_string_empty_leading_nul_and_long() {
    diff_is_string_empty(b"\0abc\0");
    let mut long = vec![b'z'; 4096];
    long.push(0);
    diff_is_string_empty(&long);
    // Long string whose first byte is NUL.
    let mut long_nul = vec![0u8; 1];
    long_nul.extend(std::iter::repeat(b'q').take(4096));
    long_nul.push(0);
    diff_is_string_empty(&long_nul);
}

// ---------------------------------------------------------------------------
// C6 / C7 / C8 / C9 — create_buffer
// ---------------------------------------------------------------------------
#[test]
fn c6_create_buffer_empty() {
    diff_create_buffer(b"\0");
}

#[test]
fn c7_create_buffer_random_bytes() {
    let mut rng = Rng::new(SEED ^ 3);
    for _ in 0..256 {
        let len = 1 + rng.below(128);
        let s = rng.cstring(len);
        diff_create_buffer(&s);
    }
    // Every single-byte string, including bytes >= 0x80.
    for b in 1u16..=255 {
        diff_create_buffer(&[b as u8, 0]);
    }
}

#[test]
fn c8_create_buffer_long() {
    for len in [4096usize, 8192] {
        let mut s = vec![b'A'; len];
        s.push(0);
        diff_create_buffer(&s);
    }
}

#[test]
fn c9_create_buffer_embedded_nul() {
    diff_create_buffer(b"abc\0def\0");
    diff_create_buffer(b"\0hidden\0");
}

// ---------------------------------------------------------------------------
// C10..C18 — find_char_in_buffer
// ---------------------------------------------------------------------------
#[test]
fn c10_find_first_byte() {
    let buf = b"Xabcdef";
    diff_find(buf, buf.len(), b'X');
}

#[test]
fn c11_find_middle_multiple_occurrences() {
    let buf = b"abXcdXef";
    diff_find(buf, buf.len(), b'X');
    let buf2 = b"aaaaaaXXXXaaaa";
    diff_find(buf2, buf2.len(), b'X');
}

#[test]
fn c12_find_last_byte() {
    let buf = b"abcdefX";
    diff_find(buf, buf.len(), b'X');
}

#[test]
fn c13_find_single_byte_buffer() {
    diff_find(b"X", 1, b'X'); // hit
    diff_find(b"Y", 1, b'X'); // miss
    diff_find(b"\0", 1, 0); // NUL hit
}

#[test]
fn c14_find_size_smaller_than_strlen() {
    let buf = b"abcdefX";
    // Occurrence at index 6 is outside the scanned prefix.
    for size in 0..=6 {
        diff_find(buf, size, b'X');
    }
    diff_find(buf, 7, b'X');
}

#[test]
fn c15_find_size_larger_than_strlen() {
    // `size` deliberately scans past the terminating NUL.
    let buf = b"abc\0Xyz";
    diff_find(buf, buf.len(), b'X');
    diff_find(buf, 4, b'X');
    diff_find(buf, buf.len(), 0);
}

#[test]
fn c16_find_nul_target() {
    let buf = b"abc\0def";
    diff_find(buf, buf.len(), 0);
    diff_find(buf, 3, 0);
}

#[test]
fn c17_find_all_byte_values() {
    let buf: Vec<u8> = (0u16..256).map(|b| b as u8).collect();
    for t in 0u16..256 {
        diff_find(&buf, buf.len(), t as u8);
    }
    // Reverse order buffer as well, so the match index differs per target.
    let rev: Vec<u8> = (0u16..256).rev().map(|b| b as u8).collect();
    for t in 0u16..256 {
        diff_find(&rev, rev.len(), t as u8);
    }
}

#[test]
fn c18_find_randomized() {
    let mut rng = Rng::new(SEED ^ 4);
    for _ in 0..1024 {
        let len = 1 + rng.below(256);
        let mut buf: Vec<u8> = (0..len).map(|_| rng.byte()).collect();
        // Occasionally force a hit somewhere in the buffer.
        let target = if rng.below(3) == 0 {
            let idx = rng.below(len);
            let t = rng.byte();
            buf[idx] = t;
            t
        } else {
            rng.byte()
        };
        let size = rng.below(len + 1);
        diff_find(&buf, size, target);
    }
}

// ---------------------------------------------------------------------------
// C19..C23 — the four counter operations and the static counter state machine
// ---------------------------------------------------------------------------
#[test]
fn c19_reset_counter() {
    for v in EXTREMES {
        diff_op(0, v);
    }
    let mut rng = Rng::new(SEED ^ 5);
    for _ in 0..256 {
        diff_op(0, rng.interesting_i32());
    }
}

#[test]
fn c20_increment_counter_with_wrap() {
    let mut rng = Rng::new(SEED ^ 6);
    for seed in [0, 1, -1, i32::MAX, i32::MIN, 123456] {
        seed_counters(seed);
        for _ in 0..512 {
            diff_op(1, rng.interesting_i32());
        }
    }
    // Explicit wrap past INT_MAX.
    seed_counters(i32::MAX);
    diff_op(1, 1);
    diff_op(1, i32::MAX);
}

#[test]
fn c21_decrement_counter_with_wrap() {
    let mut rng = Rng::new(SEED ^ 7);
    for seed in [0, 1, -1, i32::MAX, i32::MIN, -98765] {
        seed_counters(seed);
        for _ in 0..512 {
            diff_op(3, rng.interesting_i32());
        }
    }
    seed_counters(i32::MIN);
    diff_op(3, 1);
    diff_op(3, i32::MAX);
}

#[test]
fn c22_multiply_counter_with_overflow() {
    let mut rng = Rng::new(SEED ^ 8);
    for seed in [0, 1, -1, 2, i32::MAX, i32::MIN, 46341] {
        seed_counters(seed);
        for f in [0, 1, -1, 2, -2, i32::MAX, i32::MIN] {
            diff_op(2, f);
            seed_counters(seed);
        }
        for _ in 0..512 {
            diff_op(2, rng.interesting_i32());
        }
    }
}

#[test]
fn c23_randomized_op_sequences() {
    let mut rng = Rng::new(SEED ^ 9);
    for _ in 0..256 {
        seed_counters(rng.interesting_i32());
        let steps = 1 + rng.below(32);
        for _ in 0..steps {
            let op = rng.below(4);
            diff_op(op, rng.interesting_i32());
        }
    }
}

// ---------------------------------------------------------------------------
// C24 / C25 — apply_operation (function-pointer dispatch)
// ---------------------------------------------------------------------------
#[test]
fn c24_apply_operation_each_op() {
    for op in 0..4 {
        for v in [0, 1, -1, i32::MAX, i32::MIN, 65535, 65536] {
            seed_counters(7);
            diff_apply(op, v);
        }
    }
    let mut rng = Rng::new(SEED ^ 10);
    seed_counters(0);
    for _ in 0..256 {
        diff_apply(rng.below(4), rng.interesting_i32());
    }
}

#[test]
fn c25_apply_operation_sequences() {
    let mut rng = Rng::new(SEED ^ 11);
    for _ in 0..256 {
        seed_counters(rng.interesting_i32());
        let steps = 1 + rng.below(16);
        for _ in 0..steps {
            diff_apply(rng.below(4), rng.interesting_i32());
        }
    }
}

// ---------------------------------------------------------------------------
// C26..C31 — charinbuf, one test per mode
// ---------------------------------------------------------------------------
#[test]
fn c26_charinbuf_mode0() {
    for v in EXTREMES {
        diff_charinbuf(0, v, 0, 0);
    }
    let mut rng = Rng::new(SEED ^ 12);
    for _ in 0..512 {
        diff_charinbuf(0, rng.interesting_i32(), rng.next_i32(), rng.next_i32());
    }
    for _ in 0..256 {
        diff_charinbuf(0, rng.range_i32(-3, 65539), rng.next_i32(), rng.next_i32());
    }
}

#[test]
fn c27_charinbuf_mode1() {
    let mut rng = Rng::new(SEED ^ 13);
    diff_charinbuf(1, 0, 0, 0);
    for _ in 0..256 {
        diff_charinbuf(
            1,
            rng.interesting_i32(),
            rng.interesting_i32(),
            rng.interesting_i32(),
        );
    }
    // Mode 1 always yields 10 (empty test string + non-empty literal).
    let (rc, out) = diff_charinbuf_capture(1, 42, -7, 3);
    assert_eq!(rc, 10, "C mode 1 result changed: {rc}");
    assert!(String::from_utf8_lossy(&out).contains("Non-empty string correctly identified"));
}

#[test]
fn c28_charinbuf_mode2() {
    let mut rng = Rng::new(SEED ^ 14);
    diff_charinbuf(2, 0, 0, 0);
    for _ in 0..256 {
        diff_charinbuf(
            2,
            rng.interesting_i32(),
            rng.interesting_i32(),
            rng.interesting_i32(),
        );
    }
    let (rc, out) = diff_charinbuf_capture(2, 1, 2, 3);
    assert_eq!(rc, 23, "strlen(\"Testing malloc and free\") == 23, got {rc}");
    assert!(String::from_utf8_lossy(&out).contains("Buffer freed successfully"));
}

#[test]
fn c29_charinbuf_mode3_randomized() {
    let mut rng = Rng::new(SEED ^ 15);
    for _ in 0..1024 {
        diff_charinbuf(
            3,
            rng.interesting_i32(),
            rng.interesting_i32(),
            rng.interesting_i32(),
        );
    }
    for _ in 0..512 {
        diff_charinbuf(3, rng.next_i32(), rng.next_i32(), rng.next_i32());
    }
}

#[test]
fn c30_charinbuf_mode3_multiply_edge_cases() {
    for opt2 in [0, 1, -1, 2, -2, i32::MAX, i32::MIN] {
        for value in [0, 1, -1, 1000, i32::MAX, i32::MIN] {
            for opt1 in [0, 1, -1, i32::MAX, i32::MIN] {
                diff_charinbuf(3, value, opt1, opt2);
            }
        }
    }
}

#[test]
fn c31_charinbuf_mode4() {
    let mut rng = Rng::new(SEED ^ 16);
    diff_charinbuf(4, 0, 0, 0);
    for _ in 0..256 {
        diff_charinbuf(
            4,
            rng.interesting_i32(),
            rng.interesting_i32(),
            rng.interesting_i32(),
        );
    }
    let (rc, out) = diff_charinbuf_capture(4, 0, 0, 0);
    assert_eq!(rc, 21, "'X' sits at index 21 of the literal, got {rc}");
    assert!(String::from_utf8_lossy(&out).contains("Found 'X' at position: 21"));
}

// ---------------------------------------------------------------------------
// C32 / C33 — cross-call static-counter state
// ---------------------------------------------------------------------------
#[test]
fn c32_charinbuf_resets_and_leaves_counter_state() {
    let (c, r) = both();
    // Put both counters into a distinctive state, then confirm `charinbuf`
    // zeroes it on entry (observed through a follow-up increment).
    seed_counters(999_999);
    diff_charinbuf(1, 0, 0, 0);
    unsafe {
        let rc = (c.increment_counter)(1);
        let rr = (r.increment_counter)(1);
        assert_eq!(rc, rr, "counter after charinbuf(mode 1) mismatch");
        assert_eq!(rc, 1, "C charinbuf must reset the static counter to 0");
    }

    // Mode 3 leaves the counter at its final value; verify the value the next
    // direct call observes.
    for (value, opt1, opt2) in [(5, 3, 2), (-1, -2, -3), (i32::MAX, 1, 2), (0, 0, 0)] {
        diff_charinbuf(3, value, opt1, opt2);
        unsafe {
            let rc = (c.increment_counter)(0);
            let rr = (r.increment_counter)(0);
            assert_eq!(
                rc, rr,
                "counter left behind by charinbuf(3, {value}, {opt1}, {opt2}) mismatch"
            );
        }
    }
}

#[test]
fn c33_random_pipeline_sequences() {
    let mut rng = Rng::new(SEED ^ 17);
    let (c, r) = both();
    for _ in 0..128 {
        for _ in 0..8 {
            match rng.below(3) {
                0 => diff_charinbuf(
                    rng.range_i32(-2, 6),
                    rng.interesting_i32(),
                    rng.interesting_i32(),
                    rng.interesting_i32(),
                ),
                1 => diff_op(rng.below(4), rng.interesting_i32()),
                _ => diff_apply(rng.below(4), rng.interesting_i32()),
            }
        }
        // Observe the accumulated counter without changing it.
        unsafe {
            let rc = (c.increment_counter)(0);
            let rr = (r.increment_counter)(0);
            assert_eq!(rc, rr, "counter diverged during the mixed sequence");
        }
    }
}

// ---------------------------------------------------------------------------
// C34 — full cross-product of mode x value x opt1 x opt2
// ---------------------------------------------------------------------------
#[test]
fn c34_full_cross_product() {
    const VALUES: [i32; 7] = [i32::MIN, -1, 0, 1, 65535, 65536, i32::MAX];
    const OPTS: [i32; 5] = [i32::MIN, -1, 0, 1, i32::MAX];
    let mut count = 0usize;
    for mode in 0..=4 {
        for v in VALUES {
            for o1 in OPTS {
                for o2 in OPTS {
                    diff_charinbuf(mode, v, o1, o2);
                    count += 1;
                }
            }
        }
    }
    assert_eq!(count, 5 * 7 * 5 * 5);
}
