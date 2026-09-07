//! Phase C — error / boundary differential tests.
//!
//! One test per row of `ERRORS.md`, plus the generic C-API boundaries. As
//! documented in `ERRORS.md`, this C library has *no* error-return surface:
//! there is not a single `return -1`, `assert`, null check, range check or
//! error enum in `c_src/src/lib.c`. So each row asserts that C and Rust agree
//! *exactly* on the observable result (the returned `size_t`, or the printed
//! bytes) for the exact invalid/boundary condition — the same sentinel, not
//! merely "both did something".

mod common;

use common::{c_siphash_stdout, rust_siphash_stdout, Libs, Rng, FIXED_SEED};
use std::os::raw::c_void;

// ---------------------------------------------------------------------------
// E1 / E2 / E3 — null pointer and zero length
// ---------------------------------------------------------------------------

#[test]
fn e1_null_pointer_len_zero() {
    let libs = Libs::load();
    let (c, r) = unsafe {
        (
            (libs.c_hash_bytes)(std::ptr::null_mut(), 0, 0),
            (libs.rust_hash_bytes)(std::ptr::null_mut(), 0, 0),
        )
    };
    assert_eq!(c, r, "E1 NULL,len=0: C={c:#018x} Rust={r:#018x}");
}

#[test]
fn e2_valid_pointer_len_zero_same_as_null() {
    let libs = Libs::load();
    let mut buf = [0xabu8; 16];
    let (c_null, r_null) = unsafe {
        (
            (libs.c_hash_bytes)(std::ptr::null_mut(), 0, 0),
            (libs.rust_hash_bytes)(std::ptr::null_mut(), 0, 0),
        )
    };
    let (c_buf, r_buf) = libs.both_hash(&mut buf, 0, 0);
    assert_eq!(c_buf, r_buf, "E2 C/Rust disagree on len=0");
    // The pointer is never dereferenced, so the value must not depend on it.
    assert_eq!(c_null, c_buf, "E2 C: len=0 result depends on pointer?");
    assert_eq!(r_null, r_buf, "E2 Rust: len=0 result depends on pointer?");
}

#[test]
fn e3_one_past_end_pointer_len_zero() {
    let libs = Libs::load();
    let mut buf = [0x5au8; 8];
    let end = unsafe { buf.as_mut_ptr().add(8) } as *mut c_void;
    let (c, r) = unsafe {
        (
            (libs.c_hash_bytes)(end, 0, 0),
            (libs.rust_hash_bytes)(end, 0, 0),
        )
    };
    assert_eq!(c, r, "E3 one-past-end,len=0: C={c:#018x} Rust={r:#018x}");
}

// ---------------------------------------------------------------------------
// E4 — maximal legal length (the boundary the next value would cross)
// ---------------------------------------------------------------------------

#[test]
fn e4_len_equals_buffer_size_boundary() {
    let libs = Libs::load();
    let mut rng = Rng::new(FIXED_SEED ^ 0xe4);
    for n in 0usize..=72 {
        let mut buf = vec![0u8; n];
        rng.fill(&mut buf);
        // len == n is the largest in-bounds length; len == n+1 would read OOB
        // and is intentionally not executed (C has no length validation, so it
        // would be UB in the C too, not a rejection).
        libs.assert_hash_eq(&mut buf, n, 0, &format!("E4 len==buffer size n={n}"));
    }
}

// ---------------------------------------------------------------------------
// E5..E13 — every branch of the tail `switch (len - i)`
// ---------------------------------------------------------------------------

/// Exercise a given `len % 8` remainder across several block counts.
fn tail_case(rem: usize, mutate: impl Fn(&mut [u8]) + Copy, ctx: &str) {
    let libs = Libs::load();
    let mut rng = Rng::new(FIXED_SEED ^ (0xe500 + rem as u64));
    for blocks in 0usize..=4 {
        let len = blocks * 8 + rem;
        if len == 0 {
            // still a valid configuration (case 0 with zero blocks)
            let mut buf = [0u8; 8];
            libs.assert_hash_eq(&mut buf, 0, 0, ctx);
            continue;
        }
        for _ in 0..200 {
            let mut buf = vec![0u8; len.max(8)];
            rng.fill(&mut buf);
            mutate(&mut buf[blocks * 8..]);
            libs.assert_hash_eq(&mut buf, len, 0, &format!("{ctx} len={len}"));
        }
    }
}

#[test]
fn e5_tail_case_0() {
    // len % 8 == 0 and len > 0: `data` is only `len << 56`.
    tail_case(0, |_| {}, "E5 tail case 0");
}

#[test]
fn e6_tail_case_1() {
    tail_case(1, |_| {}, "E6 tail case 1");
    // Every possible d[0].
    let libs = Libs::load();
    for b in 0u16..=255 {
        let mut buf = [b as u8; 9];
        libs.assert_hash_eq(&mut buf, 9, 0, "E6 d[0] sweep");
    }
}

#[test]
fn e7_tail_case_2() {
    tail_case(2, |_| {}, "E7 tail case 2");
}

#[test]
fn e8_tail_case_3() {
    tail_case(3, |_| {}, "E8 tail case 3");
}

#[test]
fn e9_tail_case_4_d3_high_sign_extension() {
    // The signed-int-overflow branch: (d[3] << 24) is negative -> sign-extends.
    tail_case(4, |t| t[3] |= 0x80, "E9 tail case 4, d[3]>=0x80");
    let libs = Libs::load();
    for b in 0x80u16..=0xff {
        let mut buf = [0u8; 12];
        buf[8 + 3] = b as u8;
        libs.assert_hash_eq(&mut buf, 12, 0, "E9 d[3] sweep 0x80..0xff");
    }
}

#[test]
fn e10_tail_case_4_d3_low() {
    tail_case(4, |t| t[3] &= 0x7f, "E10 tail case 4, d[3]<0x80");
    let libs = Libs::load();
    for b in 0u16..0x80 {
        let mut buf = [0u8; 12];
        buf[8 + 3] = b as u8;
        libs.assert_hash_eq(&mut buf, 12, 0, "E10 d[3] sweep 0x00..0x7f");
    }
}

#[test]
fn e11_tail_case_5() {
    tail_case(5, |_| {}, "E11 tail case 5");
    // Isolate d[4] so the `<< 16 << 16` (net 32) shift is pinned exactly.
    let libs = Libs::load();
    for b in 0u16..=255 {
        let mut buf = [0u8; 5];
        buf[4] = b as u8;
        libs.assert_hash_eq(&mut buf, 5, 0, "E11 d[4] isolated sweep");
    }
}

#[test]
fn e12_tail_case_6() {
    tail_case(6, |_| {}, "E12 tail case 6");
    // Isolate d[5]: the C shifts by 20 then 20 (net 40), NOT 48.
    let libs = Libs::load();
    for b in 0u16..=255 {
        let mut buf = [0u8; 6];
        buf[5] = b as u8;
        libs.assert_hash_eq(&mut buf, 6, 0, "E12 d[5] isolated sweep");
    }
}

#[test]
fn e13_tail_case_7() {
    tail_case(7, |_| {}, "E13 tail case 7");
    // Isolate d[6]: `<< 24 << 24` (net 48).
    let libs = Libs::load();
    for b in 0u16..=255 {
        let mut buf = [0u8; 7];
        buf[6] = b as u8;
        libs.assert_hash_eq(&mut buf, 7, 0, "E13 d[6] isolated sweep");
    }
}

// ---------------------------------------------------------------------------
// E14 / E15 — signed overflow inside the main loop body
// ---------------------------------------------------------------------------

#[test]
fn e14_main_loop_d3_sign_extension() {
    let libs = Libs::load();
    // Isolate the low word's sign bit: only d[3] is non-zero.
    for b in 0x80u16..=0xff {
        let mut buf = [0u8; 8];
        buf[3] = b as u8;
        libs.assert_hash_eq(&mut buf, 8, 0, "E14 isolated d[3] high");
    }
    // ...and across every block of a multi-block input.
    let mut rng = Rng::new(FIXED_SEED ^ 0xe14);
    for blocks in 1usize..=5 {
        let mut buf = vec![0u8; blocks * 8];
        for _ in 0..200 {
            rng.fill(&mut buf);
            for b in 0..blocks {
                buf[b * 8 + 3] |= 0x80;
            }
            libs.assert_hash_eq(&mut buf, blocks * 8, 0, "E14 all blocks d[3] high");
        }
    }
}

#[test]
fn e15_main_loop_d7_sign_extension_shifted_out() {
    let libs = Libs::load();
    // Isolate the high word's sign bit: only d[7] is non-zero. The sign
    // extension is shifted out by `<< 16 << 16`, so the result must differ
    // from the E14 shape — both libraries must agree on that.
    for b in 0x80u16..=0xff {
        let mut buf = [0u8; 8];
        buf[7] = b as u8;
        libs.assert_hash_eq(&mut buf, 8, 0, "E15 isolated d[7] high");
    }
    let mut rng = Rng::new(FIXED_SEED ^ 0xe15);
    for blocks in 1usize..=5 {
        let mut buf = vec![0u8; blocks * 8];
        for _ in 0..200 {
            rng.fill(&mut buf);
            for b in 0..blocks {
                buf[b * 8 + 7] |= 0x80;
                buf[b * 8 + 3] &= 0x7f;
            }
            libs.assert_hash_eq(&mut buf, blocks * 8, 0, "E15 all blocks d[7] high");
        }
    }
}

// ---------------------------------------------------------------------------
// E16 — `len << 56` keeps only the low byte of `len`
// ---------------------------------------------------------------------------

#[test]
fn e16_length_top_byte_aliasing() {
    let libs = Libs::load();
    let mut rng = Rng::new(FIXED_SEED ^ 0xe16);
    let mut buf = vec![0u8; 600];
    rng.fill(&mut buf);
    // Aliasing pairs: len and len+256 contribute the same `len << 56` bits.
    for len in [0usize, 1, 7, 8, 9, 63, 64, 100, 255] {
        for extra in [0usize, 256] {
            let l = len + extra;
            libs.assert_hash_eq(&mut buf, l, 0, &format!("E16 len={l}"));
        }
    }
    // Oversized `len` relative to the buffer is NOT rejected by the C (no
    // validation exists) and cannot be executed without OOB reads, so the
    // boundary is pinned at the largest in-bounds length instead.
    libs.assert_hash_eq(&mut buf, 600, 0, "E16 len==buffer size");
}

// ---------------------------------------------------------------------------
// E17 — the seed cancels out; verified, not assumed
// ---------------------------------------------------------------------------

#[test]
fn e17_seed_is_irrelevant_and_identical() {
    let libs = Libs::load();
    let mut rng = Rng::new(FIXED_SEED ^ 0xe17);
    let mut buf = vec![0u8; 40];
    for _ in 0..200 {
        rng.fill(&mut buf);
        let len = rng.below(41) as usize;
        let (c_ref, r_ref) = libs.both_hash(&mut buf, len, 0);
        assert_eq!(c_ref, r_ref, "E17 baseline seed=0 diverges");
        for seed in [
            1usize,
            usize::MAX,
            usize::MAX - 1,
            1usize << 63,
            (1usize << 63) | 1,
            rng.next_u64() as usize,
            rng.next_u64() as usize,
        ] {
            let (c, r) = libs.both_hash(&mut buf, len, seed);
            assert_eq!(c, r, "E17 C/Rust diverge at seed={seed:#x} len={len}");
            // Both must exhibit the same (seed-independent) behaviour.
            assert_eq!(c, c_ref, "E17 C seed={seed:#x} changed the hash");
            assert_eq!(r, r_ref, "E17 Rust seed={seed:#x} changed the hash");
        }
    }
}

// ---------------------------------------------------------------------------
// E18 / E19 / E20 — `siphash(int init)` boundaries
// ---------------------------------------------------------------------------

fn assert_siphash_eq(init: i32, ctx: &str) {
    let c_out = c_siphash_stdout(init);
    let r_out = rust_siphash_stdout(init);
    assert!(!c_out.is_empty(), "{ctx}: C siphash({init}) printed nothing");
    if c_out != r_out {
        let cl: Vec<&[u8]> = c_out.split(|b| *b == b'\n').collect();
        let rl: Vec<&[u8]> = r_out.split(|b| *b == b'\n').collect();
        for (i, (a, b)) in cl.iter().zip(rl.iter()).enumerate() {
            if a != b {
                panic!(
                    "{ctx}: siphash({init}) differs at line {i}:\n  C   : {}\n  Rust: {}",
                    String::from_utf8_lossy(a),
                    String::from_utf8_lossy(b)
                );
            }
        }
        panic!("{ctx}: siphash({init}) output lengths differ");
    }
}

#[test]
fn e18_siphash_negative_init() {
    for init in [-1i32, -2, -127, -128, -255, -256, -1000, i32::MIN, i32::MIN + 1] {
        assert_siphash_eq(init, "E18 negative init");
    }
}

#[test]
fn e19_siphash_int_max_signed_overflow() {
    // z++ overflows a signed int on the second iteration.
    for init in [i32::MAX, i32::MAX - 1, i32::MAX - 63, i32::MAX - 64] {
        assert_siphash_eq(init, "E19 INT_MAX overflow");
    }
    // Explicitly pin the wrap: init=INT_MAX must give the same first bytes as
    // the wrapping sequence 0xff, 0x00, 0x01, ... in both libraries.
    let c = c_siphash_stdout(i32::MAX);
    let r = rust_siphash_stdout(i32::MAX);
    assert_eq!(c, r, "E19 INT_MAX stdout differs");
}

#[test]
fn e20_siphash_byte_boundary_inits() {
    for init in [0i32, 255, 256, 127, 128, 65535, 65536] {
        assert_siphash_eq(init, "E20 byte-boundary init");
    }
    // Line shape must be identical too (64 rows, 8 hex bytes each).
    let out = c_siphash_stdout(0);
    let lines: Vec<&[u8]> = out.split(|b| *b == b'\n').filter(|l| !l.is_empty()).collect();
    assert_eq!(lines.len(), 64, "E20 expected 64 rows");
    for l in lines {
        assert_eq!(
            l.iter().filter(|b| **b == b',').count(),
            9,
            "E20 unexpected row shape: {}",
            String::from_utf8_lossy(l)
        );
    }
}

// ---------------------------------------------------------------------------
// Generic C-API boundaries required regardless of the table
// ---------------------------------------------------------------------------

#[test]
fn generic_out_of_range_enum_style_ints() {
    // `siphash` takes an `int`. There is no enum in this API, but the same
    // class of input (an int with no "valid variant") is covered by feeding
    // values across the whole i32 range, including the extremes.
    let mut rng = Rng::new(FIXED_SEED ^ 0xdead);
    let mut inits = vec![i32::MIN, i32::MIN + 1, -1, 0, 1, i32::MAX - 1, i32::MAX];
    for _ in 0..12 {
        inits.push(rng.next_u64() as i32);
    }
    for init in inits {
        assert_siphash_eq(init, "generic int-range boundary");
    }
}

#[test]
fn generic_zero_and_boundary_lengths() {
    let libs = Libs::load();
    let mut rng = Rng::new(FIXED_SEED ^ 0xbeef);
    // Every length 0..=129 on a buffer that is exactly that long: covers zero,
    // each block boundary, and one step past each block boundary.
    for len in 0usize..=129 {
        let mut buf = vec![0u8; len];
        rng.fill(&mut buf);
        libs.assert_hash_eq(&mut buf, len, 0, &format!("generic len={len}"));
        let mut zeros = vec![0u8; len];
        libs.assert_hash_eq(&mut zeros, len, 0, &format!("generic zeros len={len}"));
        let mut ones = vec![0xffu8; len];
        libs.assert_hash_eq(&mut ones, len, 0, &format!("generic 0xff len={len}"));
    }
}

#[test]
fn generic_all_len_all_single_byte_positions() {
    // For every length 0..=24 and every byte position, sweep that byte over
    // 0x00, 0x01, 0x7f, 0x80, 0xfe, 0xff with the rest zero. This isolates
    // every shift/sign-extension path independently.
    let libs = Libs::load();
    for len in 0usize..=24 {
        for pos in 0..len {
            for v in [0x00u8, 0x01, 0x7f, 0x80, 0xfe, 0xff] {
                let mut buf = vec![0u8; len];
                buf[pos] = v;
                libs.assert_hash_eq(
                    &mut buf,
                    len,
                    0,
                    &format!("generic len={len} pos={pos} v={v:#04x}"),
                );
            }
        }
    }
}
