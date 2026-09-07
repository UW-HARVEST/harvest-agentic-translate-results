//! Phase B — valid-path differential tests.
//!
//! One test per row of `CONFIGS.md`. Every test loads BOTH `libdriver.so`
//! (C) and `libdriver.so` (Rust) via `libloading` and calls only the exported
//! `driver` symbol, then compares the captured libc `stdout` byte-for-byte.

use crate::common::*;
use std::ffi::c_int;

// ---------------------------------------------------------------------------
// Row 1 — x = 0 (all four bytes 0x00; every byte needs %02x zero padding)
// ---------------------------------------------------------------------------
pub fn cfg_row01_zero() {
    let out = assert_same("row01", 0);
    // Structural cross-check against the C semantics we read from driver.c.
    assert_eq!(out, b"00000000\n".to_vec());
}

// ---------------------------------------------------------------------------
// Row 2 — x = -1 (all four bytes 0xff; maximum sign-extension exposure)
// ---------------------------------------------------------------------------
pub fn cfg_row02_all_ones() {
    let out = assert_same("row02", -1);
    assert_eq!(
        out,
        b"ffffffff\n".to_vec(),
        "an i8/c_char translation would have printed sign-extended bytes here"
    );
}

// ---------------------------------------------------------------------------
// Row 3 — x = INT_MAX
// ---------------------------------------------------------------------------
pub fn cfg_row03_int_max() {
    let out = assert_same("row03", i32::MAX);
    assert_eq!(out, b"ffffff7f\n".to_vec());
}

// ---------------------------------------------------------------------------
// Row 4 — x = INT_MIN (top byte exactly 0x80: the sign-bit boundary)
// ---------------------------------------------------------------------------
pub fn cfg_row04_int_min() {
    let out = assert_same("row04", i32::MIN);
    assert_eq!(out, b"00000080".to_vec().into_iter().chain(*b"\n").collect::<Vec<u8>>());
}

// ---------------------------------------------------------------------------
// Row 5 — one byte < 0x10 at each of the 4 positions (padding x byte order)
// ---------------------------------------------------------------------------
pub fn cfg_row05_single_low_nibble_byte_each_position() {
    let mut rng = Rng::new(5);
    for pos in 0..4usize {
        for _ in 0..256 {
            let mut b = [0u8; 4];
            b[pos] = (rng.next_u8() & 0x0f).max(1); // 0x01..=0x0f
            assert_same("row05", from_le_bytes(b));
        }
    }
}

// ---------------------------------------------------------------------------
// Row 6 — one byte in 0x10..=0x7f at each of the 4 positions
// ---------------------------------------------------------------------------
pub fn cfg_row06_single_midrange_byte_each_position() {
    let mut rng = Rng::new(6);
    for pos in 0..4usize {
        for _ in 0..256 {
            let mut b = [0u8; 4];
            b[pos] = 0x10 + (rng.next_u8() % 0x70); // 0x10..=0x7f
            assert_same("row06", from_le_bytes(b));
        }
    }
}

// ---------------------------------------------------------------------------
// Row 7 — one byte in 0x80..=0xff at each of the 4 positions
// ---------------------------------------------------------------------------
pub fn cfg_row07_single_high_bit_byte_each_position() {
    let mut rng = Rng::new(7);
    for pos in 0..4usize {
        for _ in 0..256 {
            let mut b = [0u8; 4];
            b[pos] = 0x80 | (rng.next_u8() & 0x7f); // 0x80..=0xff
            assert_same("row07", from_le_bytes(b));
        }
    }
}

// ---------------------------------------------------------------------------
// Row 8 — EXHAUSTIVE: every byte value 0x00..=0xff in every one of the 4 lanes
// ---------------------------------------------------------------------------
pub fn cfg_row08_exhaustive_byte_per_lane() {
    for pos in 0..4usize {
        let xs: Vec<c_int> = (0u16..=0xff)
            .map(|v| {
                let mut b = [0u8; 4];
                b[pos] = v as u8;
                from_le_bytes(b)
            })
            .collect();
        // Compared as one stream so record framing is verified as well.
        assert_same_batch("row08", &xs);
    }
}

// ---------------------------------------------------------------------------
// Row 9 — every single-bit value 1<<k for k in 0..32 (bit -> nibble -> byte map)
// ---------------------------------------------------------------------------
pub fn cfg_row09_every_single_bit() {
    let xs: Vec<c_int> = (0..32).map(|k| (1u32 << k) as i32).collect();
    let out = assert_same_batch("row09", &xs);
    assert_eq!(out.len(), 32 * 9, "32 records of 8 hex digits + newline");
    // The lowest bit must show up in the FIRST printed byte on this
    // little-endian target.
    assert!(out.starts_with(b"01000000\n"));
}

// ---------------------------------------------------------------------------
// Row 10 — random negative x
// ---------------------------------------------------------------------------
pub fn cfg_row10_random_negative() {
    let mut rng = Rng::new(10);
    let xs: Vec<c_int> = (0..2000)
        .map(|_| {
            let v = rng.next_i32();
            if v < 0 { v } else { -(v.max(1)) }
        })
        .collect();
    for &x in &xs {
        assert!(x < 0);
    }
    assert_same_batch("row10", &xs);
}

// ---------------------------------------------------------------------------
// Row 11 — random positive x
// ---------------------------------------------------------------------------
pub fn cfg_row11_random_positive() {
    let mut rng = Rng::new(11);
    let xs: Vec<c_int> = (0..2000)
        .map(|_| (rng.next_u32() & 0x7fff_ffff).max(1) as i32)
        .collect();
    for &x in &xs {
        assert!(x > 0);
    }
    assert_same_batch("row11", &xs);
}

// ---------------------------------------------------------------------------
// Row 12 — random over the FULL 2^32 domain
// ---------------------------------------------------------------------------
pub fn cfg_row12_random_full_range() {
    let mut rng = Rng::new(12);
    let xs: Vec<c_int> = (0..5000).map(|_| rng.next_i32()).collect();
    assert_same_batch("row12", &xs);
    // Also one-at-a-time, so a per-call divergence cannot hide inside a stream.
    let mut rng2 = Rng::new(1212);
    for _ in 0..1000 {
        assert_same("row12-single", rng2.next_i32());
    }
}

// ---------------------------------------------------------------------------
// Row 13 — ALL four bytes < 0x10 simultaneously (all lanes zero-padded)
// ---------------------------------------------------------------------------
pub fn cfg_row13_all_bytes_low_nibble() {
    let mut rng = Rng::new(13);
    for _ in 0..1000 {
        let b = [
            rng.next_u8() & 0x0f,
            rng.next_u8() & 0x0f,
            rng.next_u8() & 0x0f,
            rng.next_u8() & 0x0f,
        ];
        let out = assert_same("row13", from_le_bytes(b));
        assert_eq!(out.len(), 9, "%02x must pad: 8 digits + newline");
        // Every high nibble must be '0'.
        for i in (0..8).step_by(2) {
            assert_eq!(out[i], b'0');
        }
    }
}

// ---------------------------------------------------------------------------
// Row 14 — ALL four bytes >= 0x80 simultaneously
// ---------------------------------------------------------------------------
pub fn cfg_row14_all_bytes_high_bit() {
    let mut rng = Rng::new(14);
    for _ in 0..1000 {
        let b = [
            0x80 | (rng.next_u8() & 0x7f),
            0x80 | (rng.next_u8() & 0x7f),
            0x80 | (rng.next_u8() & 0x7f),
            0x80 | (rng.next_u8() & 0x7f),
        ];
        let out = assert_same("row14", from_le_bytes(b));
        assert_eq!(out.len(), 9, "no sign extension may widen the output");
    }
}

// ---------------------------------------------------------------------------
// Row 15 — all 24 permutations of the bytes {0x00, 0x0f, 0x80, 0xff}
// ---------------------------------------------------------------------------
pub fn cfg_row15_all_byte_permutations() {
    let base = [0x00u8, 0x0f, 0x80, 0xff];
    let mut xs = Vec::new();
    for i in 0..4 {
        for j in 0..4 {
            if j == i {
                continue;
            }
            for k in 0..4 {
                if k == i || k == j {
                    continue;
                }
                let l = 6 - i - j - k;
                xs.push(from_le_bytes([base[i], base[j], base[k], base[l]]));
            }
        }
    }
    assert_eq!(xs.len(), 24, "4! permutations");
    assert_same_batch("row15", &xs);
    for &x in &xs {
        assert_same("row15-single", x);
    }
}

// ---------------------------------------------------------------------------
// Row 16 — "one": a single call must emit exactly 9 bytes and nothing else
// ---------------------------------------------------------------------------
pub fn cfg_row16_single_call_exact_framing() {
    let mut rng = Rng::new(16);
    for _ in 0..200 {
        let x = rng.next_i32();
        let out = assert_same("row16", x);
        assert_eq!(
            out.len(),
            9,
            "driver({x}) must emit 8 hex digits + one '\\n', got {out:02x?}"
        );
        assert_eq!(*out.last().unwrap(), b'\n');
        assert!(out[..8].iter().all(|c| c.is_ascii_hexdigit()));
        // Lowercase only: the C format is %02x, not %02X.
        assert!(out[..8].iter().all(|c| !c.is_ascii_uppercase()));
    }
}

// ---------------------------------------------------------------------------
// Row 17 — "many": 500 calls compared as one concatenated stdout stream
// ---------------------------------------------------------------------------
pub fn cfg_row17_many_calls_one_stream() {
    let mut rng = Rng::new(17);
    let xs: Vec<c_int> = (0..500).map(|_| rng.next_i32()).collect();
    let out = assert_same_batch("row17", &xs);
    assert_eq!(out.len(), 500 * 9, "no missing or extra newline");
}

// ---------------------------------------------------------------------------
// Row 18 — interleaving C and Rust calls in the SAME captured stdout.
//
// Proves both libraries write through the identical libc `stdout` object: an
// alternating C/Rust stream must be byte-identical to the all-C stream. A
// translation using Rust's `std::io::stdout()` would buffer separately and
// reorder or duplicate output here.
// ---------------------------------------------------------------------------
pub fn cfg_row18_interleaved_c_and_rust_same_stdout() {
    let mut rng = Rng::new(18);
    let xs: Vec<c_int> = (0..400).map(|_| rng.next_i32()).collect();

    let cf = c_driver();
    let rf = rust_driver();

    let all_c = capture_stdout(|| {
        for &x in &xs {
            unsafe { cf(x) }
        }
    });
    let all_rust = capture_stdout(|| {
        for &x in &xs {
            unsafe { rf(x) }
        }
    });
    let interleaved = capture_stdout(|| {
        for (i, &x) in xs.iter().enumerate() {
            if i % 2 == 0 {
                unsafe { cf(x) }
            } else {
                unsafe { rf(x) }
            }
        }
    });

    assert_eq!(all_c, all_rust, "[row18] all-C vs all-Rust stream");
    assert_eq!(
        all_c, interleaved,
        "[row18] interleaved C/Rust stream must equal the all-C stream"
    );
}

// ---------------------------------------------------------------------------
// Row 19 — full lifecycle: load, use, dlclose, dlopen again, use again
// ---------------------------------------------------------------------------
pub fn cfg_row19_full_lifecycle_reload() {
    let mut rng = Rng::new(19);
    let xs: Vec<c_int> = (0..64).map(|_| rng.next_i32()).collect();

    for round in 0..3 {
        let c_lib = unsafe { libloading::Library::new(c_so_path()) }.expect("dlopen C");
        let r_lib = unsafe { libloading::Library::new(rust_so_path()) }.expect("dlopen Rust");
        let cf: libloading::Symbol<DriverFn> =
            unsafe { c_lib.get(b"driver\0") }.expect("dlsym C driver");
        let rf: libloading::Symbol<DriverFn> =
            unsafe { r_lib.get(b"driver\0") }.expect("dlsym Rust driver");

        let c_out = capture_stdout(|| {
            for &x in &xs {
                unsafe { cf(x) }
            }
        });
        let r_out = capture_stdout(|| {
            for &x in &xs {
                unsafe { rf(x) }
            }
        });
        assert_eq!(c_out, r_out, "[row19] round {round} after reload");
        assert_eq!(c_out.len(), xs.len() * 9);

        drop(cf);
        drop(rf);
        drop(c_lib);
        drop(r_lib);
    }
}
