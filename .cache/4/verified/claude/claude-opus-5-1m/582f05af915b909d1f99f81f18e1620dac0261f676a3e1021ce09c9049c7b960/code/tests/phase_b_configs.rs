//! Phase B — valid-path differential tests.
//!
//! One test per row of `CONFIGS.md`. Every row drives BOTH the C `.so` and the
//! Rust `.so` through `libloading` and compares stdout byte-for-byte, using many
//! randomized inputs from that row's class (fixed-seed SplitMix64).

mod common;

use common::*;
use std::ffi::c_int;

/// Every row gets its own seed so the rows do not sample identical sequences.
fn rng_for(row: u64) -> Rng {
    Rng::new(0xC0FFEE_0000_0000u64 ^ (row.wrapping_mul(0x9E37_79B9_7F4A_7C15)))
}

// --------------------------------------------------------------------------
// C1 — floors == 0
// --------------------------------------------------------------------------
#[test]
fn c01_zero() {
    let out = assert_same(0);
    assert_eq!(out, expected_bytes(0));
    assert!(out.starts_with(b"00000000"), "got {out:?}");
}

// --------------------------------------------------------------------------
// C2..C6 — increasing numbers of significant bytes (byte order + %02x padding)
// --------------------------------------------------------------------------
fn sweep_range(row: u64, lo: i32, hi: i32) {
    let mut rng = rng_for(row);
    // Always include the two endpoints, then randomize inside the class.
    assert_same(lo);
    assert_same(hi);
    for _ in 0..SAMPLES {
        let x = rng.i32_in_range(lo, hi);
        let out = assert_same(x);
        assert_eq!(out, expected_bytes(x), "model mismatch for {x}");
    }
}

#[test]
fn c02_low_nibble_only() {
    sweep_range(2, 1, 15);
}

#[test]
fn c03_one_byte_high_bit_clear() {
    sweep_range(3, 16, 127);
}

#[test]
fn c04_one_byte_high_bit_set() {
    // 0x80..=0xff: distinguishes `unsigned char` reinterpretation from a signed
    // `char` sign-extension bug (which would print `ffffff80` instead of `80`).
    sweep_range(4, 128, 255);
    for b in 0x80u32..=0xffu32 {
        let x = b as i32;
        let out = assert_same(x);
        let lead = std::str::from_utf8(&out[..2]).unwrap();
        assert_eq!(
            lead,
            format!("{b:02x}"),
            "high-bit byte was not printed as unsigned for {x}"
        );
    }
}

#[test]
fn c05_two_significant_bytes() {
    sweep_range(5, 256, 65_535);
}

#[test]
fn c06_three_significant_bytes() {
    sweep_range(6, 65_536, 16_777_215);
}

// --------------------------------------------------------------------------
// C7 — four pairwise-distinct nonzero bytes
// --------------------------------------------------------------------------
#[test]
fn c07_four_distinct_nonzero_bytes() {
    let mut rng = rng_for(7);
    let mut done = 0;
    while done < SAMPLES {
        let b0 = rng.byte(true);
        let b1 = rng.byte(true);
        let b2 = rng.byte(true);
        let b3 = rng.byte(true);
        if b0 == b1 || b0 == b2 || b0 == b3 || b1 == b2 || b1 == b3 || b2 == b3 {
            continue;
        }
        let x = i32::from_le_bytes([b0, b1, b2, b3]);
        let out = assert_same(x);
        // Pins little-endian ordering explicitly.
        let hex = std::str::from_utf8(&out[..8]).unwrap();
        assert_eq!(hex, format!("{b0:02x}{b1:02x}{b2:02x}{b3:02x}"));
        done += 1;
    }
    // Hand-picked canonical pattern too.
    let x = i32::from_le_bytes([0xaa, 0xbb, 0xcc, 0xdd]);
    let out = assert_same(x);
    assert!(out.starts_with(b"aabbccdd"), "got {out:?}");
}

// --------------------------------------------------------------------------
// C8/C9/C10 — negatives
// --------------------------------------------------------------------------
#[test]
fn c08_minus_one_all_ff() {
    let out = assert_same(-1);
    assert!(out.starts_with(b"ffffffff"), "got {out:?}");
    assert_eq!(out, expected_bytes(-1));
}

#[test]
fn c09_small_negatives() {
    sweep_range(9, -256, -1);
}

#[test]
fn c10_medium_negatives() {
    sweep_range(10, -65_536, -257);
}

// --------------------------------------------------------------------------
// C11/C12 — int boundaries
// --------------------------------------------------------------------------
#[test]
fn c11_int_max() {
    let out = assert_same(i32::MAX);
    assert_eq!(out, expected_bytes(i32::MAX));
    assert!(out.starts_with(b"ffffff7f"), "got {out:?}");
}

#[test]
fn c12_int_min() {
    let out = assert_same(i32::MIN);
    assert_eq!(out, expected_bytes(i32::MIN));
    assert!(out.starts_with(b"00000080"), "got {out:?}");
}

// --------------------------------------------------------------------------
// C13 — embedded NUL bytes: no C-string truncation
// --------------------------------------------------------------------------
#[test]
fn c13_embedded_nul_bytes() {
    let fixed: [i32; 8] = [
        0x00ff00ffu32 as i32,
        0xff00ff00u32 as i32,
        0x00000001,
        0x01000000,
        0x00010000,
        0x0000ff00,
        0xff0000ffu32 as i32,
        0x00ffff00u32 as i32,
    ];
    for &x in &fixed {
        let out = assert_same(x);
        assert_eq!(out.len(), 33, "truncated output for 0x{:08x}", x as u32);
        assert_eq!(out, expected_bytes(x));
    }

    // Randomized: force a random subset of the 4 bytes to zero.
    let mut rng = rng_for(13);
    for _ in 0..SAMPLES {
        let mask = (rng.next_u32() & 0x0f) as u8;
        let mut bytes = [0u8; 4];
        for (i, b) in bytes.iter_mut().enumerate() {
            *b = if mask & (1 << i) != 0 {
                0
            } else {
                rng.byte(true)
            };
        }
        let x = i32::from_le_bytes(bytes);
        let out = assert_same(x);
        assert_eq!(out.len(), 33, "truncated output for 0x{:08x}", x as u32);
        assert_eq!(out, expected_bytes(x));
    }
}

// --------------------------------------------------------------------------
// C14 — payload bytes 0x0a ('\n') and 0x25 ('%')
// --------------------------------------------------------------------------
#[test]
fn c14_newline_and_percent_payload_bytes() {
    let mut rng = rng_for(14);
    for special in [0x0au8, 0x25u8] {
        for pos in 0..4usize {
            // Deterministic pattern with `special` at `pos`.
            let mut bytes = [0x11u8, 0x22, 0x33, 0x44];
            bytes[pos] = special;
            let x = i32::from_le_bytes(bytes);
            let out = assert_same(x);
            assert_eq!(out, expected_bytes(x));
            // Exactly one newline, and it is the final byte.
            assert_eq!(out.iter().filter(|&&b| b == b'\n').count(), 1);
            assert_eq!(*out.last().unwrap(), b'\n');

            // Plus randomized surroundings.
            for _ in 0..(SAMPLES / 8) {
                let mut bytes = [
                    rng.byte(false),
                    rng.byte(false),
                    rng.byte(false),
                    rng.byte(false),
                ];
                bytes[pos] = special;
                let x = i32::from_le_bytes(bytes);
                let out = assert_same(x);
                assert_eq!(out, expected_bytes(x));
                assert_eq!(out.iter().filter(|&&b| b == b'\n').count(), 1);
            }
        }
    }
}

// --------------------------------------------------------------------------
// C15 — full-range property test
// --------------------------------------------------------------------------
#[test]
fn c15_full_i32_range_property() {
    let mut rng = rng_for(15);
    for _ in 0..4000 {
        let x = rng.next_i32();
        let out = assert_same(x);
        assert_eq!(out, expected_bytes(x), "model mismatch for 0x{:08x}", x as u32);
    }
}

// --------------------------------------------------------------------------
// C16 — exhaustive small-magnitude sweep across zero
// --------------------------------------------------------------------------
#[test]
fn c16_exhaustive_small_magnitudes() {
    for x in -512i32..=512i32 {
        let out = assert_same(x);
        assert_eq!(out, expected_bytes(x));
    }
}

// --------------------------------------------------------------------------
// C17 — single-bit values, every bit lane including the sign bit
// --------------------------------------------------------------------------
#[test]
fn c17_single_bit_values() {
    for k in 0..32u32 {
        let x = (1u32 << k) as i32;
        let out = assert_same(x);
        assert_eq!(out, expected_bytes(x), "bit {k}");
        // Also the complement, so every lane is tested with 0xff-ish neighbours.
        let y = !x;
        let out = assert_same(y);
        assert_eq!(out, expected_bytes(y), "bit {k} complement");
    }
}

// --------------------------------------------------------------------------
// C18 — invariant tail from the fixed constants (bedrooms=3, bathrooms=2.0)
// --------------------------------------------------------------------------
#[test]
fn c18_invariant_tail_bytes() {
    const TAIL: &[u8] = b"030000000000000000000040";
    let mut rng = rng_for(18);
    for _ in 0..SAMPLES {
        let x = rng.next_i32();
        let c = run_c(x);
        let r = run_rust(x);
        assert_eq!(c, r);
        // Bytes 4..16 of the struct == hex chars 8..32 of the line.
        assert_eq!(&c[8..32], TAIL, "C tail changed for 0x{:08x}", x as u32);
        assert_eq!(&r[8..32], TAIL, "Rust tail changed for 0x{:08x}", x as u32);
    }
}

// --------------------------------------------------------------------------
// C19 — output framing
// --------------------------------------------------------------------------
#[test]
fn c19_output_framing() {
    let mut rng = rng_for(19);
    for _ in 0..SAMPLES {
        let x = rng.next_i32();
        for (name, out) in [("C", run_c(x)), ("Rust", run_rust(x))] {
            assert_eq!(out.len(), 33, "{name}: wrong length for 0x{:08x}", x as u32);
            assert_eq!(out[32], b'\n', "{name}: missing trailing newline");
            for (i, &b) in out[..32].iter().enumerate() {
                assert!(
                    b.is_ascii_digit() || (b'a'..=b'f').contains(&b),
                    "{name}: byte {i} = {b:#04x} is not a lowercase hex digit"
                );
            }
        }
        assert_eq!(run_c(x), run_rust(x));
    }
}

// --------------------------------------------------------------------------
// C20 — reachable loop bound is exactly sizeof(house_t) == 16
// --------------------------------------------------------------------------
#[test]
fn c20_loop_bound_is_sizeof_house() {
    const SIZEOF_HOUSE: usize = 16;
    let mut rng = rng_for(20);
    for _ in 0..64 {
        let x = rng.next_i32();
        let c = run_c(x);
        let r = run_rust(x);
        assert_eq!(c, r);
        assert_eq!(c.len(), 2 * SIZEOF_HOUSE + 1);
        assert_eq!(r.len(), 2 * SIZEOF_HOUSE + 1);
    }
}

// --------------------------------------------------------------------------
// C21 — batched pipeline: many calls in one buffering window
// --------------------------------------------------------------------------
#[test]
fn c21_batched_calls() {
    let mut rng = rng_for(21);
    for batch_len in [1usize, 2, 3, 7, 16, 64, 257] {
        let xs: Vec<c_int> = (0..batch_len).map(|_| rng.next_i32()).collect();
        let c = run_c_batch(&xs);
        let r = run_rust_batch(&xs);
        assert_eq!(
            c, r,
            "batch of {batch_len} diverged; xs = {:?}",
            &xs[..xs.len().min(8)]
        );
        assert_eq!(c.len(), 33 * batch_len);
        let model: Vec<u8> = xs.iter().flat_map(|&x| expected_bytes(x)).collect();
        assert_eq!(c, model);
    }
}

// --------------------------------------------------------------------------
// C22 — statelessness under repetition
// --------------------------------------------------------------------------
#[test]
fn c22_repeat_is_stateless() {
    let mut rng = rng_for(22);
    for _ in 0..32 {
        let x = rng.next_i32();
        let n = 5usize;
        let xs = vec![x; n];
        let c = run_c_batch(&xs);
        let r = run_rust_batch(&xs);
        assert_eq!(c, r);
        let one = expected_bytes(x);
        for chunk in c.chunks(33) {
            assert_eq!(chunk, &one[..], "residual state after repeat");
        }
    }
}

// --------------------------------------------------------------------------
// C23 — cross-library interleaving in the same process
// --------------------------------------------------------------------------
#[test]
fn c23_cross_library_interleaving() {
    let cf = c_driver();
    let rf = rust_driver();
    let mut rng = rng_for(23);
    let xs: Vec<c_int> = (0..64).map(|_| rng.next_i32()).collect();

    // C,R,C,R,... all inside a single capture window.
    let interleaved = capture("interleaved", || {
        for &x in &xs {
            unsafe {
                cf(x);
                rf(x);
            }
        }
    });

    let mut expected = Vec::new();
    for &x in &xs {
        expected.extend_from_slice(&expected_bytes(x));
        expected.extend_from_slice(&expected_bytes(x));
    }
    assert_eq!(interleaved, expected, "interleaved C/Rust output diverged");

    // And with different arguments per library, to catch shared-state effects.
    let interleaved2 = capture("interleaved2", || {
        for (i, &x) in xs.iter().enumerate() {
            let y = xs[(i + 1) % xs.len()];
            unsafe {
                cf(x);
                rf(y);
            }
        }
    });
    let mut expected2 = Vec::new();
    for (i, &x) in xs.iter().enumerate() {
        let y = xs[(i + 1) % xs.len()];
        expected2.extend_from_slice(&expected_bytes(x));
        expected2.extend_from_slice(&expected_bytes(y));
    }
    assert_eq!(interleaved2, expected2);
}

// --------------------------------------------------------------------------
// C24 — stdout buffering mode axis
// --------------------------------------------------------------------------
#[test]
fn c24_buffering_modes() {
    let mut rng = rng_for(24);
    const SINKS: [Sink; 3] = [Sink::Memory, Sink::TempFile, Sink::TempFileUnbuffered];
    for _ in 0..64 {
        let x = rng.next_i32();
        for s in SINKS {
            let c = run_c_sink(x, s);
            let r = run_rust_sink(x, s);
            assert_eq!(c, r, "diverged for 0x{:08x} with {s:?}", x as u32);
            assert_eq!(c, expected_bytes(x), "model mismatch with {s:?}");
        }
        // The sink / buffering mode must not change the bytes at all.
        for s in SINKS {
            assert_eq!(run_c_sink(x, s), run_c_sink(x, Sink::Memory));
            assert_eq!(run_rust_sink(x, s), run_rust_sink(x, Sink::Memory));
        }
    }
}

// --------------------------------------------------------------------------
// C25 — model cross-check (proves the harness observes value-dependent bytes)
// --------------------------------------------------------------------------
#[test]
fn c25_model_cross_check() {
    let mut rng = rng_for(25);
    let mut seen = std::collections::HashSet::new();
    for _ in 0..SAMPLES {
        let x = rng.next_i32();
        let c = run_c(x);
        assert_eq!(c, expected_bytes(x));
        assert_eq!(run_rust(x), expected_bytes(x));
        seen.insert(c);
    }
    assert!(
        seen.len() > SAMPLES / 2,
        "output barely varied across {SAMPLES} random inputs ({} distinct); \
         the capture harness is probably not observing real output",
        seen.len()
    );
}

// --------------------------------------------------------------------------
// Sanity: the two .so files really are two different files, both loaded.
// --------------------------------------------------------------------------
#[test]
fn c00_harness_loads_two_distinct_libraries() {
    let l = libs();
    assert_ne!(l.c_path, l.rust_path);
    assert!(l.c_path.to_string_lossy().contains("c_src"));
    let _ = c_driver();
    let _ = rust_driver();
    eprintln!("C   .so: {}", l.c_path.display());
    eprintln!("Rust.so: {}", l.rust_path.display());
}
