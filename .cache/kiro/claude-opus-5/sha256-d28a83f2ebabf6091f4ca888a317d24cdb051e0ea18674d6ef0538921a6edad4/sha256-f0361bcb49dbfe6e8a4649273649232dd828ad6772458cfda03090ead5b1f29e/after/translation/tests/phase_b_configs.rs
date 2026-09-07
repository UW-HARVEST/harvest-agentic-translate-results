//! Phase B — valid-path differential tests, one test per row of `CONFIGS.md`.
//!
//! Every test drives BOTH shared objects through their exported C symbols and
//! compares results byte-for-byte. Randomized rows use a fixed-seed PRNG.

mod common;

use common::*;

/// Per-row PRNG seeds, derived from the master seed so each row is
/// independently reproducible.
fn rng_for(row: u64) -> Rng {
    Rng::new(SEED ^ (row.wrapping_mul(0x9E37_79B9_7F4A_7C15)))
}

const ASCII: &[u8] = b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789 .,;:!?-_";
const SMALL: &[u8] = b"Axb";

// ---------------------------------------------------------------------------
// Row 1 — empty input, every needle
// ---------------------------------------------------------------------------

#[test]
fn cfg_row1_empty_input_all_needles() {
    for needle in 1u16..=255 {
        let v = assert_foo_eq("row1", b"", needle as u8);
        assert_eq!(v, 0, "empty input must yield 0 for needle {needle:#04x}");
    }
}

// ---------------------------------------------------------------------------
// Row 2 — full single-byte matrix: 255 payloads x 255 needles
// ---------------------------------------------------------------------------

#[test]
fn cfg_row2_single_byte_matrix() {
    for payload in 1u16..=255 {
        let buf = [payload as u8];
        for needle in 1u16..=255 {
            let v = assert_foo_eq("row2", &buf, needle as u8);
            let expect = if needle == payload { 1 } else { 0 };
            assert_eq!(
                v, expect,
                "payload {payload:#04x} needle {needle:#04x}: expected {expect}, got {v}"
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Row 3 — needle absent (zero occurrences), randomized
// ---------------------------------------------------------------------------

#[test]
fn cfg_row3_zero_occurrences_random() {
    let mut rng = rng_for(3);
    for _ in 0..400 {
        let len = rng.below(200) as usize;
        // Alphabet deliberately excludes the needle.
        let needle = b'Q';
        let alphabet: Vec<u8> = ASCII.iter().copied().filter(|&b| b != needle).collect();
        let buf: Vec<u8> = (0..len).map(|_| rng.byte_from(&alphabet)).collect();
        let v = assert_foo_eq("row3", &buf, needle);
        assert_eq!(v, 0);
    }
}

// ---------------------------------------------------------------------------
// Row 4 — exactly one occurrence at a random position
// ---------------------------------------------------------------------------

#[test]
fn cfg_row4_exactly_one_occurrence_random() {
    let mut rng = rng_for(4);
    for _ in 0..400 {
        let len = 1 + rng.below(200) as usize;
        let needle = b'Z';
        let alphabet: Vec<u8> = ASCII.iter().copied().filter(|&b| b != needle).collect();
        let mut buf: Vec<u8> = (0..len).map(|_| rng.byte_from(&alphabet)).collect();
        let pos = rng.below(len as u64) as usize;
        buf[pos] = needle;
        let v = assert_foo_eq("row4", &buf, needle);
        assert_eq!(v, 1, "one occurrence at {pos} of {len}");
    }
}

// ---------------------------------------------------------------------------
// Row 5 — many scattered occurrences, high density
// ---------------------------------------------------------------------------

#[test]
fn cfg_row5_many_occurrences_random() {
    let mut rng = rng_for(5);
    for _ in 0..400 {
        let len = rng.below(300) as usize;
        let buf: Vec<u8> = (0..len).map(|_| rng.byte_from(SMALL)).collect();
        for &needle in SMALL {
            let v = assert_foo_eq("row5", &buf, needle);
            let expect = buf.iter().filter(|&&b| b == needle).count() as i32;
            assert_eq!(v, expect);
        }
    }
}

// ---------------------------------------------------------------------------
// Row 6 — consecutive runs of the needle
// ---------------------------------------------------------------------------

#[test]
fn cfg_row6_consecutive_runs_random() {
    let mut rng = rng_for(6);
    let needle = b'A';
    for _ in 0..400 {
        let mut buf = Vec::new();
        let runs = rng.below(12) as usize;
        for _ in 0..runs {
            // filler, then a run of the needle
            for _ in 0..rng.below(5) {
                buf.push(b'-');
            }
            for _ in 0..(1 + rng.below(6)) {
                buf.push(needle);
            }
        }
        for _ in 0..rng.below(5) {
            buf.push(b'-');
        }
        let v = assert_foo_eq("row6", &buf, needle);
        let expect = buf.iter().filter(|&&b| b == needle).count() as i32;
        assert_eq!(v, expect);
    }
}

// ---------------------------------------------------------------------------
// Row 7 — occurrence at index 0
// ---------------------------------------------------------------------------

#[test]
fn cfg_row7_match_at_first_byte_random() {
    let mut rng = rng_for(7);
    for _ in 0..400 {
        let needle = rng.nonzero_byte();
        let len = rng.below(120) as usize;
        let mut buf = vec![needle];
        for _ in 0..len {
            let mut b = rng.nonzero_byte();
            if b == needle {
                b = needle.wrapping_add(1).max(1);
            }
            buf.push(b);
        }
        let v = assert_foo_eq("row7", &buf, needle);
        let expect = buf.iter().filter(|&&b| b == needle).count() as i32;
        assert_eq!(v, expect);
    }
}

// ---------------------------------------------------------------------------
// Row 8 — occurrence at the last byte before the terminator
// ---------------------------------------------------------------------------

#[test]
fn cfg_row8_match_at_last_byte_random() {
    let mut rng = rng_for(8);
    for _ in 0..400 {
        let needle = rng.nonzero_byte();
        let len = rng.below(120) as usize;
        let mut buf = Vec::new();
        for _ in 0..len {
            let mut b = rng.nonzero_byte();
            if b == needle {
                b = needle.wrapping_add(1).max(1);
            }
            buf.push(b);
        }
        buf.push(needle);
        let v = assert_foo_eq("row8", &buf, needle);
        let expect = buf.iter().filter(|&&b| b == needle).count() as i32;
        assert_eq!(v, expect, "needle {needle:#04x} at final index {}", buf.len() - 1);
    }
}

// ---------------------------------------------------------------------------
// Row 9 — every byte matches (saturated)
// ---------------------------------------------------------------------------

#[test]
fn cfg_row9_all_bytes_match_random() {
    let mut rng = rng_for(9);
    for _ in 0..400 {
        let needle = rng.nonzero_byte();
        let len = 1 + rng.below(300) as usize;
        let buf = vec![needle; len];
        let v = assert_foo_eq("row9", &buf, needle);
        assert_eq!(v, len as i32);
    }
}

// ---------------------------------------------------------------------------
// Row 10 — full 0x01..=0xFF alphabet (non-UTF-8), random needle
// ---------------------------------------------------------------------------

#[test]
fn cfg_row10_full_byte_alphabet_random() {
    let mut rng = rng_for(10);
    for _ in 0..600 {
        let len = rng.below(250) as usize;
        let buf: Vec<u8> = (0..len).map(|_| rng.nonzero_byte()).collect();
        let needle = rng.nonzero_byte();
        let v = assert_foo_eq("row10", &buf, needle);
        let expect = buf.iter().filter(|&&b| b == needle).count() as i32;
        assert_eq!(v, expect);
    }
}

// ---------------------------------------------------------------------------
// Row 11 — high-bit needle (negative as signed char)
// ---------------------------------------------------------------------------

#[test]
fn cfg_row11_high_bit_needle_random() {
    let mut rng = rng_for(11);
    let high: Vec<u8> = (0x80u16..=0xFF).map(|b| b as u8).collect();
    // Exhaustive over the high-bit needles, randomized inputs for each.
    for &needle in &high {
        for _ in 0..8 {
            let len = rng.below(160) as usize;
            let buf: Vec<u8> = (0..len)
                .map(|_| {
                    if rng.below(2) == 0 {
                        rng.byte_from(&high)
                    } else {
                        rng.byte_from(ASCII)
                    }
                })
                .collect();
            let v = assert_foo_eq("row11", &buf, needle);
            let expect = buf.iter().filter(|&&b| b == needle).count() as i32;
            assert_eq!(v, expect, "high-bit needle {needle:#04x}");
        }
    }
}

// ---------------------------------------------------------------------------
// Row 12 — needle '\0': matches the terminator, so `s++` walks out of bounds
// forever. `strchr(s, 0)` can never return NULL, so the loop never exits and
// both libraries walk memory until they fault. Verified out-of-process.
// ---------------------------------------------------------------------------

#[test]
fn cfg_row12_nul_needle_padded_arena() {
    use std::ffi::CString;
    let p = pair();
    for input in ["", "a", "hello world", "AAAAxxxx"] {
        let s = CString::new(input).unwrap();
        let mut outs = Vec::new();
        for imp in [&p.c, &p.rust] {
            let f = imp.foo();
            let ptr = s.as_ptr();
            outs.push(run_in_child(|| {
                let _ = unsafe { f(ptr, 0) };
            }, 30, None));
        }
        assert_eq!(
            outs[0], outs[1],
            "[row12] NUL-needle outcome divergence for input {input:?}: \
             C {:?} vs Rust {:?}",
            outs[0], outs[1]
        );
        assert_eq!(
            outs[0],
            Outcome::Signalled(SIGSEGV),
            "[row12] expected SIGSEGV from the unbounded walk, got {:?}",
            outs[0]
        );
    }
}

// ---------------------------------------------------------------------------
// Row 13 — large input (1 MiB), varying needle density
// ---------------------------------------------------------------------------

#[test]
fn cfg_row13_large_input_random() {
    let mut rng = rng_for(13);
    const N: usize = 1 << 20;
    let needle = b'A';
    // densities: never, rare, ~1/16, ~1/2, always
    for &(num, den) in &[(0u64, 1u64), (1, 4096), (1, 16), (1, 2), (1, 1)] {
        let mut buf = Vec::with_capacity(N);
        for _ in 0..N {
            let hit = num > 0 && rng.below(den) < num;
            buf.push(if hit { needle } else { b'.' });
        }
        let v = assert_foo_eq("row13", &buf, needle);
        let expect = buf.iter().filter(|&&b| b == needle).count() as i32;
        assert_eq!(v, expect, "density {num}/{den}");
    }
}

// ---------------------------------------------------------------------------
// Rows 14-18 — `driver` end to end, stdout compared byte-for-byte
// ---------------------------------------------------------------------------

#[test]
fn cfg_row14_driver_stdout_no_matches() {
    let out = assert_driver_stdout_eq("row14", b"no matches here at all!");
    assert_eq!(out, b"A: 0\nx: 0\n");
}

#[test]
fn cfg_row15_driver_stdout_only_a_needle() {
    let out = assert_driver_stdout_eq("row15", b"AAA bbb ccc");
    assert_eq!(out, b"A: 3\nx: 0\n");
}

#[test]
fn cfg_row16_driver_stdout_only_x_needle() {
    let out = assert_driver_stdout_eq("row16", b"xx yy zz x");
    assert_eq!(out, b"A: 0\nx: 3\n");
}

#[test]
fn cfg_row17_driver_stdout_multi_digit() {
    let mut buf = Vec::new();
    buf.extend(std::iter::repeat_n(b'A', 123));
    buf.extend(b"-----");
    buf.extend(std::iter::repeat_n(b'x', 4567));
    let out = assert_driver_stdout_eq("row17", &buf);
    assert_eq!(out, b"A: 123\nx: 4567\n");
}

#[test]
fn cfg_row18_driver_stdout_empty() {
    let out = assert_driver_stdout_eq("row18", b"");
    assert_eq!(out, b"A: 0\nx: 0\n");
}

#[test]
fn cfg_row19_driver_stdout_random() {
    let mut rng = rng_for(19);
    for _ in 0..300 {
        let len = rng.below(400) as usize;
        let mut buf: Vec<u8> = (0..len)
            .map(|_| match rng.below(4) {
                0 => b'A',
                1 => b'x',
                2 => rng.byte_from(ASCII),
                _ => rng.nonzero_byte(),
            })
            .collect();
        // Force boundary placements and adjacent runs some of the time.
        if !buf.is_empty() {
            match rng.below(4) {
                0 => buf[0] = b'A',
                1 => {
                    let last = buf.len() - 1;
                    buf[last] = b'x';
                }
                2 => {
                    buf.extend(b"AAxxAAxx");
                }
                _ => {}
            }
        }
        let out = assert_driver_stdout_eq("row19", &buf);
        let a = buf.iter().filter(|&&b| b == b'A').count();
        let x = buf.iter().filter(|&&b| b == b'x').count();
        assert_eq!(out, format!("A: {a}\nx: {x}\n").into_bytes());
    }
}

// ---------------------------------------------------------------------------
// Row 20 — composed pipeline consistency: driver's printed numbers must equal
// the directly-called low-level `foo` results, cross-checked across libraries.
// ---------------------------------------------------------------------------

#[test]
fn cfg_row20_composed_consistency_random() {
    let mut rng = rng_for(20);
    for _ in 0..300 {
        let len = rng.below(250) as usize;
        let buf: Vec<u8> = (0..len)
            .map(|_| match rng.below(3) {
                0 => b'A',
                1 => b'x',
                _ => rng.byte_from(ASCII),
            })
            .collect();

        // Low-level entry point, called directly on both libraries.
        let a = assert_foo_eq("row20", &buf, b'A');
        let x = assert_foo_eq("row20", &buf, b'x');

        // Convenience wrapper, stdout compared on both libraries.
        let out = assert_driver_stdout_eq("row20", &buf);

        // The wrapper must be a faithful composition of the low-level calls,
        // in that order, with those labels.
        let text = String::from_utf8(out.clone()).expect("driver output not UTF-8");
        let mut lines = text.lines();
        let l1 = lines.next().expect("missing first line");
        let l2 = lines.next().expect("missing second line");
        assert!(lines.next().is_none(), "unexpected extra output: {text:?}");
        assert_eq!(l1, format!("A: {a}"), "first line must be foo(in,'A')");
        assert_eq!(l2, format!("x: {x}"), "second line must be foo(in,'x')");
    }
}
