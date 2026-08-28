//! Phase B — valid-path differential tests.
//!
//! One test per row of `CONFIGS.md`. Every test drives BOTH `.so`s (the C one
//! built by CMake and the Rust `cdylib`) through their exported `bin2hex`
//! symbol with identical inputs and compares the destination buffers
//! byte-for-byte plus the returned pointer.

mod common;

use common::*;
use std::ffi::c_char;

/// Helper: canary-filled destination sized to the exact C contract.
fn diff_exact(ctx: &str, bin: &[u8], hex_maxlen: usize) {
    let dst = bin.len() * 2 + 1 + 16; // 16 canary bytes past the output window
    diff_disjoint(ctx, bin, bin.len(), hex_maxlen, dst, 0);
}

// --------------------------------------------------------------------------
// Row 1 — bin_len = 0, hex_maxlen = 1 (minimum valid), NULL and valid `bin`
// --------------------------------------------------------------------------
#[test]
fn cfg01_empty_min_maxlen() {
    // valid, non-null bin pointer
    let data = [0xDEu8, 0xAD];
    diff_disjoint("cfg01/valid-bin", &data, 0, 1, 8, 0);

    // NULL bin: bin_len == 0 means `bin` is never dereferenced, so this is a
    // valid call in both implementations.
    let f = impls();
    let mut c_buf = [CANARY; 8];
    let mut r_buf = [CANARY; 8];
    let c_hex = c_buf.as_mut_ptr() as *mut c_char;
    let r_hex = r_buf.as_mut_ptr() as *mut c_char;
    let c_ret = unsafe { (f.c)(c_hex, 1, std::ptr::null(), 0) };
    let r_ret = unsafe { (f.rust)(r_hex, 1, std::ptr::null(), 0) };
    assert_eq!(c_ret, c_hex, "cfg01/null-bin: C return value");
    assert_eq!(r_ret, r_hex, "cfg01/null-bin: Rust return value");
    assert_buffers_eq("cfg01/null-bin", &c_buf, &r_buf);
    assert_eq!(c_buf[0], 0, "cfg01/null-bin: C should write the NUL");
    assert_eq!(c_buf[1], CANARY, "cfg01/null-bin: C wrote past the NUL");
}

// --------------------------------------------------------------------------
// Row 2 — bin_len = 0, various slack hex_maxlen (incl. usize::MAX)
// --------------------------------------------------------------------------
#[test]
fn cfg02_empty_slack_maxlen() {
    let data = [0x12u8, 0x34, 0x56];
    for &maxlen in &[2usize, 3, 64, 1024, usize::MAX / 2, usize::MAX - 1, usize::MAX] {
        diff_disjoint(&format!("cfg02/maxlen={maxlen}"), &data, 0, maxlen, 32, 0);
    }
}

// --------------------------------------------------------------------------
// Row 3 — bin_len = 1, hex_maxlen = 3 (exact minimum), all 256 byte values
// --------------------------------------------------------------------------
#[test]
fn cfg03_single_byte_all_256_values() {
    for v in 0u16..=255 {
        let bin = [v as u8];
        diff_disjoint(&format!("cfg03/byte=0x{v:02x}"), &bin, 1, 3, 3 + 8, 0);
    }
}

// --------------------------------------------------------------------------
// Row 4 — bin_len = 1, generous hex_maxlen, canary must survive
// --------------------------------------------------------------------------
#[test]
fn cfg04_single_byte_slack_canary() {
    for v in 0u16..=255 {
        let bin = [v as u8];
        // hex_maxlen claims 64 usable bytes but only 3 may be written.
        diff_disjoint(&format!("cfg04/byte=0x{v:02x}"), &bin, 1, 64, 64, 0);
    }
}

// --------------------------------------------------------------------------
// Row 5 — bin_len = 2, exact minimum hex_maxlen, all 65536 byte pairs
// --------------------------------------------------------------------------
#[test]
fn cfg05_two_bytes_all_pairs() {
    let f = impls();
    let mut c_buf = [CANARY; 8];
    let mut r_buf = [CANARY; 8];
    for hi in 0u16..=255 {
        for lo in 0u16..=255 {
            let bin = [hi as u8, lo as u8];
            c_buf.fill(CANARY);
            r_buf.fill(CANARY);
            let c_hex = c_buf.as_mut_ptr() as *mut c_char;
            let r_hex = r_buf.as_mut_ptr() as *mut c_char;
            let c_ret = unsafe { (f.c)(c_hex, 5, bin.as_ptr(), 2) };
            let r_ret = unsafe { (f.rust)(r_hex, 5, bin.as_ptr(), 2) };
            assert_eq!(c_ret, c_hex);
            assert_eq!(r_ret, r_hex);
            if c_buf != r_buf {
                panic!(
                    "cfg05/[0x{hi:02x},0x{lo:02x}]: C={:02x?} Rust={:02x?}",
                    c_buf, r_buf
                );
            }
            // canary intact past index 4
            assert_eq!(&c_buf[5..], &[CANARY; 3], "cfg05: C wrote past the NUL");
            assert_eq!(&r_buf[5..], &[CANARY; 3], "cfg05: Rust wrote past the NUL");
        }
    }
}

// --------------------------------------------------------------------------
// Row 6 — bin_len = 3 (odd), exact minimum, randomized
// --------------------------------------------------------------------------
#[test]
fn cfg06_three_bytes_random() {
    let mut rng = Rng::new(SEED ^ 6);
    for it in 0..4096 {
        let mut bin = [0u8; 3];
        rng.fill(&mut bin);
        diff_disjoint(&format!("cfg06/it={it} bin={bin:02x?}"), &bin, 3, 7, 7 + 8, 0);
    }
}

// --------------------------------------------------------------------------
// Row 7 — bin_len sweep 0..=64, hex_maxlen = 2n+1 (exact minimum), randomized
// --------------------------------------------------------------------------
#[test]
fn cfg07_len_sweep_min_maxlen() {
    let mut rng = Rng::new(SEED ^ 7);
    for n in 0usize..=64 {
        for it in 0..64 {
            let mut bin = vec![0u8; n];
            rng.fill(&mut bin);
            diff_exact(&format!("cfg07/n={n} it={it}"), &bin, 2 * n + 1);
        }
    }
}

// --------------------------------------------------------------------------
// Row 8 — bin_len sweep 0..=64, hex_maxlen = 2n+2 (one byte of slack)
// --------------------------------------------------------------------------
#[test]
fn cfg08_len_sweep_one_slack() {
    let mut rng = Rng::new(SEED ^ 8);
    for n in 0usize..=64 {
        for it in 0..64 {
            let mut bin = vec![0u8; n];
            rng.fill(&mut bin);
            diff_exact(&format!("cfg08/n={n} it={it}"), &bin, 2 * n + 2);
        }
    }
}

// --------------------------------------------------------------------------
// Row 9 — bin_len sweep 0..=64, hex_maxlen = usize::MAX (maximum allowed)
// --------------------------------------------------------------------------
#[test]
fn cfg09_len_sweep_maxlen_usize_max() {
    let mut rng = Rng::new(SEED ^ 9);
    for n in 0usize..=64 {
        for it in 0..32 {
            let mut bin = vec![0u8; n];
            rng.fill(&mut bin);
            diff_exact(&format!("cfg09/n={n} it={it}"), &bin, usize::MAX);
        }
    }
}

// --------------------------------------------------------------------------
// Row 10 — bin_len = 256, bin[i] = i
// --------------------------------------------------------------------------
#[test]
fn cfg10_sequential_256() {
    let bin: Vec<u8> = (0..256).map(|i| i as u8).collect();
    diff_exact("cfg10", &bin, 513);
    diff_exact("cfg10/slack", &bin, 4096);
}

// --------------------------------------------------------------------------
// Row 11 — bin_len = 256, bin[i] = 255 - i
// --------------------------------------------------------------------------
#[test]
fn cfg11_reverse_sequential_256() {
    let bin: Vec<u8> = (0..256).map(|i| (255 - i) as u8).collect();
    diff_exact("cfg11", &bin, 513);
}

// --------------------------------------------------------------------------
// Row 12 — all bytes 0x00
// --------------------------------------------------------------------------
#[test]
fn cfg12_all_zero_bytes() {
    for n in [1usize, 2, 63, 64] {
        let bin = vec![0x00u8; n];
        diff_exact(&format!("cfg12/n={n}"), &bin, 2 * n + 1);
    }
}

// --------------------------------------------------------------------------
// Row 13 — all bytes 0xFF
// --------------------------------------------------------------------------
#[test]
fn cfg13_all_ff_bytes() {
    for n in [1usize, 2, 63, 64] {
        let bin = vec![0xFFu8; n];
        diff_exact(&format!("cfg13/n={n}"), &bin, 2 * n + 1);
    }
}

// --------------------------------------------------------------------------
// Row 14 — bytes only in 0x00..=0x0F (high nibble always 0)
// --------------------------------------------------------------------------
#[test]
fn cfg14_low_nibble_only() {
    let mut rng = Rng::new(SEED ^ 14);
    for it in 0..256 {
        let mut bin = vec![0u8; 64];
        for b in bin.iter_mut() {
            *b = rng.next_u8() & 0x0F;
        }
        diff_exact(&format!("cfg14/it={it}"), &bin, 129);
    }
}

// --------------------------------------------------------------------------
// Row 15 — bytes only with low nibble 0
// --------------------------------------------------------------------------
#[test]
fn cfg15_high_nibble_only() {
    let mut rng = Rng::new(SEED ^ 15);
    for it in 0..256 {
        let mut bin = vec![0u8; 64];
        for b in bin.iter_mut() {
            *b = rng.next_u8() & 0xF0;
        }
        diff_exact(&format!("cfg15/it={it}"), &bin, 129);
    }
}

// --------------------------------------------------------------------------
// Row 16 — both nibbles are decimal digits (0..=9): wrap-around path twice
// --------------------------------------------------------------------------
#[test]
fn cfg16_digit_digit_nibbles() {
    let mut rng = Rng::new(SEED ^ 16);
    for it in 0..256 {
        let mut bin = vec![0u8; 64];
        for b in bin.iter_mut() {
            let hi = (rng.next_u8() % 10) as u8;
            let lo = (rng.next_u8() % 10) as u8;
            *b = (hi << 4) | lo;
        }
        diff_exact(&format!("cfg16/it={it}"), &bin, 129);
    }
}

// --------------------------------------------------------------------------
// Row 17 — both nibbles are letters (0xA..=0xF): zero-correction path twice
// --------------------------------------------------------------------------
#[test]
fn cfg17_letter_letter_nibbles() {
    let mut rng = Rng::new(SEED ^ 17);
    for it in 0..256 {
        let mut bin = vec![0u8; 64];
        for b in bin.iter_mut() {
            let hi = 10 + (rng.next_u8() % 6);
            let lo = 10 + (rng.next_u8() % 6);
            *b = (hi << 4) | lo;
        }
        diff_exact(&format!("cfg17/it={it}"), &bin, 129);
    }
}

// --------------------------------------------------------------------------
// Row 18 — mixed nibble classes (digit/letter and letter/digit)
// --------------------------------------------------------------------------
#[test]
fn cfg18_mixed_nibble_classes() {
    let mut rng = Rng::new(SEED ^ 18);
    for it in 0..512 {
        let mut bin = vec![0u8; 64];
        for (k, b) in bin.iter_mut().enumerate() {
            let (hi, lo) = if (k + it) % 2 == 0 {
                (rng.next_u8() % 10, 10 + rng.next_u8() % 6)
            } else {
                (10 + rng.next_u8() % 6, rng.next_u8() % 10)
            };
            *b = (hi << 4) | lo;
        }
        diff_exact(&format!("cfg18/it={it}"), &bin, 129);
    }
}

// --------------------------------------------------------------------------
// Row 19 — the nibble-boundary byte set, cycled over 512 bytes
// --------------------------------------------------------------------------
#[test]
fn cfg19_nibble_boundary_bytes() {
    const BOUNDARY: [u8; 12] = [
        0x09, 0x0A, 0x90, 0xA0, 0x99, 0x9A, 0xA9, 0xAA, 0x0F, 0xF0, 0xFF, 0x00,
    ];
    // every rotation of the pattern, so each boundary byte lands at each parity
    for rot in 0..BOUNDARY.len() {
        let bin: Vec<u8> = (0..512).map(|i| BOUNDARY[(i + rot) % BOUNDARY.len()]).collect();
        diff_exact(&format!("cfg19/rot={rot}"), &bin, 1025);
    }
    // and all boundary pairs as 2-byte inputs
    for &a in BOUNDARY.iter() {
        for &b in BOUNDARY.iter() {
            diff_exact(&format!("cfg19/pair=0x{a:02x},0x{b:02x}"), &[a, b], 5);
        }
    }
}

// --------------------------------------------------------------------------
// Row 20 — bin_len = 4096, randomized
// --------------------------------------------------------------------------
#[test]
fn cfg20_large_4096_random() {
    let mut rng = Rng::new(SEED ^ 20);
    for it in 0..16 {
        let mut bin = vec![0u8; 4096];
        rng.fill(&mut bin);
        diff_exact(&format!("cfg20/it={it}"), &bin, 8193);
    }
}

// --------------------------------------------------------------------------
// Row 21 — bin_len = 65536, randomized
// --------------------------------------------------------------------------
#[test]
fn cfg21_very_large_65536_random() {
    let mut rng = Rng::new(SEED ^ 21);
    for it in 0..4 {
        let mut bin = vec![0u8; 65536];
        rng.fill(&mut bin);
        diff_exact(&format!("cfg21/it={it}"), &bin, 131073);
    }
}

// --------------------------------------------------------------------------
// Row 22 — full randomized property sweep over (bin_len, hex_maxlen, bytes)
// --------------------------------------------------------------------------
#[test]
fn cfg22_property_sweep() {
    let mut rng = Rng::new(SEED ^ 22);
    for it in 0..2000 {
        let n = rng.below(1025) as usize;
        let slack = rng.below(64) as usize;
        let hex_maxlen = 2 * n + 1 + slack;
        let mut bin = vec![0u8; n];
        rng.fill(&mut bin);
        // random offset for the `hex` pointer inside the destination allocation
        let hex_off = rng.below(8) as usize;
        let dst = hex_off + 2 * n + 1 + slack.max(1) + 8;
        diff_disjoint(
            &format!("cfg22/it={it} n={n} maxlen={hex_maxlen} off={hex_off}"),
            &bin,
            n,
            hex_maxlen,
            dst,
            hex_off,
        );
    }
}

// --------------------------------------------------------------------------
// Row 23 — in-place aliasing: hex == bin
// --------------------------------------------------------------------------
#[test]
fn cfg23_alias_hex_eq_bin() {
    let mut rng = Rng::new(SEED ^ 23);
    for n in 0usize..=32 {
        for it in 0..32 {
            let mut seed = vec![0u8; n];
            rng.fill(&mut seed);
            let arena = 2 * n + 1 + 8;
            diff_overlapping(
                &format!("cfg23/n={n} it={it}"),
                arena,
                &seed,
                0,
                n,
                0,
                2 * n + 1,
            );
        }
    }
}

// --------------------------------------------------------------------------
// Row 24 — partial overlap, `hex` starts before `bin`
// --------------------------------------------------------------------------
#[test]
fn cfg24_overlap_hex_before_bin() {
    let mut rng = Rng::new(SEED ^ 24);
    for n in 1usize..=24 {
        for k in 0usize..=8 {
            for it in 0..8 {
                let mut seed = vec![0u8; n];
                rng.fill(&mut seed);
                let arena = k + n + 2 * n + 1 + 8;
                diff_overlapping(
                    &format!("cfg24/n={n} k={k} it={it}"),
                    arena,
                    &seed,
                    k,
                    n,
                    0,
                    2 * n + 1,
                );
            }
        }
    }
}

// --------------------------------------------------------------------------
// Row 25 — partial overlap, `hex` starts after `bin`
// --------------------------------------------------------------------------
#[test]
fn cfg25_overlap_hex_after_bin() {
    let mut rng = Rng::new(SEED ^ 25);
    for n in 1usize..=24 {
        for k in 0usize..=8 {
            for it in 0..8 {
                let mut seed = vec![0u8; n];
                rng.fill(&mut seed);
                let arena = k + 2 * n + 1 + n + 8;
                diff_overlapping(
                    &format!("cfg25/n={n} k={k} it={it}"),
                    arena,
                    &seed,
                    0,
                    n,
                    k,
                    2 * n + 1,
                );
            }
        }
    }
}

// --------------------------------------------------------------------------
// Row 26 — unaligned `bin` and `hex` pointers
// --------------------------------------------------------------------------
#[test]
fn cfg26_unaligned_pointers() {
    let f = impls();
    let mut rng = Rng::new(SEED ^ 26);
    for n in 0usize..=48 {
        for bin_off in [1usize, 3, 5, 7] {
            for hex_off in [1usize, 3, 5, 7] {
                let mut src = vec![0u8; bin_off + n + 8];
                rng.fill(&mut src);
                let dst_len = hex_off + 2 * n + 1 + 8;
                let mut c_buf = vec![CANARY; dst_len];
                let mut r_buf = vec![CANARY; dst_len];
                let c_hex = unsafe { c_buf.as_mut_ptr().add(hex_off) } as *mut c_char;
                let r_hex = unsafe { r_buf.as_mut_ptr().add(hex_off) } as *mut c_char;
                let binp = unsafe { src.as_ptr().add(bin_off) };
                let c_ret = unsafe { (f.c)(c_hex, 2 * n + 1, binp, n) };
                let r_ret = unsafe { (f.rust)(r_hex, 2 * n + 1, binp, n) };
                assert_eq!(c_ret, c_hex);
                assert_eq!(r_ret, r_hex);
                assert_buffers_eq(
                    &format!("cfg26/n={n} bin_off={bin_off} hex_off={hex_off}"),
                    &c_buf,
                    &r_buf,
                );
            }
        }
    }
}

// --------------------------------------------------------------------------
// Row 27 — output ends exactly at the last writable byte before a guard page
// --------------------------------------------------------------------------
#[test]
fn cfg27_exact_fit_against_guard_page() {
    let f = impls();
    let ps = sys::page_size();
    let mut rng = Rng::new(SEED ^ 27);

    // Largest n with 2n+1 <= ps
    let n = (ps - 1) / 2;
    let out_len = 2 * n + 1;
    let off = ps - out_len; // last written byte is the last writable byte

    let mut bin = vec![0u8; n];
    rng.fill(&mut bin);

    let c_g = Guarded::new(1, false);
    let r_g = Guarded::new(1, false);
    c_g.fill(CANARY);
    r_g.fill(CANARY);

    let c_ret = unsafe { (f.c)(c_g.at(off) as *mut c_char, out_len, bin.as_ptr(), n) };
    let r_ret = unsafe { (f.rust)(r_g.at(off) as *mut c_char, out_len, bin.as_ptr(), n) };
    assert_eq!(c_ret as usize, c_g.at(off) as usize, "cfg27: C return");
    assert_eq!(r_ret as usize, r_g.at(off) as usize, "cfg27: Rust return");
    assert_buffers_eq("cfg27", &c_g.snapshot(), &r_g.snapshot());
    let snap = c_g.snapshot();
    assert_eq!(snap[ps - 1], 0, "cfg27: NUL must be the final writable byte");
    for i in 0..off {
        assert_eq!(snap[i], CANARY, "cfg27: wrote before `hex` at {i}");
    }

    // Same thing with the maximum permitted hex_maxlen, which the C code only
    // range-checks and never uses as a write bound.
    let c_g2 = Guarded::new(1, false);
    let r_g2 = Guarded::new(1, false);
    c_g2.fill(CANARY);
    r_g2.fill(CANARY);
    unsafe { (f.c)(c_g2.at(off) as *mut c_char, usize::MAX, bin.as_ptr(), n) };
    unsafe { (f.rust)(r_g2.at(off) as *mut c_char, usize::MAX, bin.as_ptr(), n) };
    assert_buffers_eq("cfg27/maxlen=usize::MAX", &c_g2.snapshot(), &r_g2.snapshot());
}

// --------------------------------------------------------------------------
// Row 28 — the return value is the identical `hex` pointer in every shape
// --------------------------------------------------------------------------
#[test]
fn cfg28_returns_same_pointer() {
    let f = impls();
    let mut rng = Rng::new(SEED ^ 28);
    // 2*2047 + 1 + 17 = 4112 bytes worst case; keep plenty of head room.
    let mut buf_c = vec![0u8; 8192];
    let mut buf_r = vec![0u8; 8192];
    for n in [0usize, 1, 2, 7, 100, 1000, 2047] {
        for off in [0usize, 1, 2, 3, 17] {
            let mut bin = vec![0u8; n];
            rng.fill(&mut bin);
            let c_hex = unsafe { buf_c.as_mut_ptr().add(off) } as *mut c_char;
            let r_hex = unsafe { buf_r.as_mut_ptr().add(off) } as *mut c_char;
            let c_ret = unsafe { (f.c)(c_hex, 2 * n + 1, bin.as_ptr(), n) };
            let r_ret = unsafe { (f.rust)(r_hex, 2 * n + 1, bin.as_ptr(), n) };
            assert_eq!(c_ret, c_hex, "cfg28/n={n} off={off}: C return != hex");
            assert_eq!(r_ret, r_hex, "cfg28/n={n} off={off}: Rust return != hex");
            // and the produced text agrees
            let a = &buf_c[off..off + 2 * n + 1];
            let b = &buf_r[off..off + 2 * n + 1];
            assert_buffers_eq(&format!("cfg28/n={n} off={off}"), a, b);
        }
    }
}
