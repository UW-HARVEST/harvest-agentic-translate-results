//! Phase C — error-path / rejection differential tests.
//!
//! One `#[test]` per row of `ERRORS.md`. Each constructs the exact invalid
//! input or out-of-range condition, calls BOTH the C `.so` and the Rust `.so`,
//! and asserts they agree on the concrete return value (not merely "both
//! failed") and on the full post-call struct image.

mod harness;

use harness::{Bitwriter, ITERS, Pair, Rng, SEED};

fn rng_for(row: u64) -> Rng {
    Rng::new(SEED ^ (row.wrapping_mul(0xD1B5_4A32_D192_ED03)))
}

/// Call both `.so`s and additionally assert the *concrete* return sentinel.
#[track_caller]
fn check_ret(p: &Pair, row: &str, iter: u64, init: &Bitwriter, bits: u32, val: u64, expect: i32) {
    p.check(row, iter, init, bits, val);

    let mut c_bw = *init;
    let mut rs_bw = *init;
    let c_ret = unsafe { (p.c_add)(&mut c_bw, bits, val) };
    let rs_ret = unsafe { (p.rs_add)(&mut rs_bw, bits, val) };
    assert_eq!(
        c_ret, expect,
        "{row}: C returned {c_ret}, expected sentinel {expect}"
    );
    assert_eq!(
        rs_ret, expect,
        "{row}: Rust returned {rs_ret}, expected sentinel {expect}"
    );
}

/* --- row 1: the sole `return` is `return 0` — never any error sentinel --- */

#[test]
fn err_row01_return_is_always_zero() {
    let p = Pair::load();
    let mut r = rng_for(1);
    for i in 0..ITERS * 4 {
        let s = r.next_u32();
        let bits = r.next_u32();
        let bw = r.bw_with_bits(s);
        let val = r.next_u64();
        check_ret(&p, "err01 return always 0", i, &bw, bits, val, 0);
    }
    // Also the hand-picked extremes, so the row does not rely on chance.
    for &(s, bits, val) in &[
        (0u32, 0u32, 0u64),
        (0, 0, u64::MAX),
        (0, 64, u64::MAX),
        (63, 1, u64::MAX),
        (64, 64, u64::MAX),
        (u32::MAX, u32::MAX, u64::MAX),
    ] {
        let bw = Bitwriter {
            bits: s,
            ..Bitwriter::default()
        };
        check_ret(&p, "err01 extremes", 0, &bw, bits, val, 0);
    }
}

/* --- row 2: bits == 0 (shift-by-64) --- */

#[test]
fn err_row02_bits_zero() {
    let p = Pair::load();
    let mut r = rng_for(2);
    for i in 0..ITERS {
        // Cover both the loop-not-entered (S<64) and loop-entered (S>=64) cases.
        let s = if i % 2 == 0 {
            r.range_u32(0, 63)
        } else {
            r.next_u32() | 64
        };
        let bw = r.bw_with_bits(s);
        let val = r.next_u64();
        check_ret(&p, "err02 bits==0", i, &bw, 0, val, 0);
    }
}

/* --- row 3: bits == 64 exactly --- */

#[test]
fn err_row03_bits_exactly_64() {
    let p = Pair::load();
    let mut r = rng_for(3);
    for i in 0..ITERS {
        let s = r.range_u32(0, 64);
        let bw = r.bw_with_bits(s);
        let val = r.next_u64();
        check_ret(&p, "err03 bits==64", i, &bw, 64, val, 0);
    }
}

/* --- row 4: bits == 65, one step past the meaningful range --- */

#[test]
fn err_row04_bits_65_one_past_range() {
    let p = Pair::load();
    let mut r = rng_for(4);
    for i in 0..ITERS {
        let s = r.range_u32(0, 64);
        let bw = r.bw_with_bits(s);
        let val = r.next_u64();
        check_ret(&p, "err04 bits==65", i, &bw, 65, val, 0);
    }
}

/* --- row 5: grossly oversized bits --- */

#[test]
fn err_row05_bits_oversized() {
    let p = Pair::load();
    let mut r = rng_for(5);
    const OVERSIZED: [u32; 10] = [
        65,
        66,
        100,
        127,
        128,
        4096,
        0x0001_0000,
        0x7FFF_FFFF,
        0x8000_0000,
        u32::MAX,
    ];
    for i in 0..ITERS {
        let bits = OVERSIZED[(i as usize) % OVERSIZED.len()];
        let s = match i % 4 {
            0 => 0,
            1 => r.range_u32(1, 63),
            2 => 64,
            _ => r.next_u32(),
        };
        let bw = r.bw_with_bits(s);
        let val = r.next_u64();
        check_ret(&p, "err05 oversized bits", i, &bw, bits, val, 0);
    }
}

/* --- row 6: bw->bits == 64 (accumulator at the limit) --- */

#[test]
fn err_row06_bw_bits_at_64() {
    let p = Pair::load();
    let mut r = rng_for(6);
    for i in 0..ITERS {
        let bits = r.range_u32(0, 128);
        let bw = r.bw_with_bits(64);
        let val = r.next_u64();
        check_ret(&p, "err06 bw->bits==64", i, &bw, bits, val, 0);
    }
}

/* --- row 7: bw->bits out of range --- */

#[test]
fn err_row07_bw_bits_out_of_range() {
    let p = Pair::load();
    let mut r = rng_for(7);
    const OOR: [u32; 9] = [
        65,
        66,
        100,
        127,
        128,
        4096,
        0x0001_0000,
        0x8000_0000,
        u32::MAX,
    ];
    for i in 0..ITERS {
        let s = OOR[(i as usize) % OOR.len()];
        let bits = r.range_u32(0, 128);
        let bw = r.bw_with_bits(s);
        let val = r.next_u64();
        check_ret(&p, "err07 bw->bits out of range", i, &bw, bits, val, 0);
    }
}

/* --- row 8: bw->bits + bits overflows u32 --- */

#[test]
fn err_row08_bw_bits_plus_bits_u32_wrap() {
    let p = Pair::load();
    let mut r = rng_for(8);
    for i in 0..ITERS {
        // Construct S + P == 2^32 + k for a small k, so the u32 sum is `k`.
        let k = r.range_u32(0, 100);
        let s = r.range_u32(1, u32::MAX);
        let bits = k.wrapping_sub(s); // s + bits == k (mod 2^32)
        assert_eq!(s.wrapping_add(bits), k);
        let bw = r.bw_with_bits(s);
        let val = r.next_u64();
        check_ret(&p, "err08 S+P wraps u32", i, &bw, bits, val, 0);
    }
    // Explicit documented example: S = U32MAX, P = 1 -> sum 0.
    let bw = Bitwriter {
        bits: u32::MAX,
        ..Bitwriter::default()
    };
    check_ret(&p, "err08 S=U32MAX P=1", 0, &bw, 1, u64::MAX, 0);
}

/* --- row 9: bw->tot + bits overflows u32 --- */

#[test]
fn err_row09_tot_u32_wrap() {
    let p = Pair::load();
    let mut r = rng_for(9);
    for i in 0..ITERS {
        let s = r.range_u32(0, 64);
        let bits = r.range_u32(0, 64);
        let mut bw = r.bw_with_bits(s);
        bw.tot = u32::MAX - r.range_u32(0, 32);
        let val = r.next_u64();
        check_ret(&p, "err09 tot wraps u32", i, &bw, bits, val, 0);
    }
    // Guaranteed wrap: tot = U32MAX, bits = 1 -> tot becomes 0.
    let bw = Bitwriter {
        tot: u32::MAX,
        ..Bitwriter::default()
    };
    check_ret(
        &p,
        "err09 tot=U32MAX bits=1",
        0,
        &bw,
        1,
        0xDEAD_BEEF_CAFE_BABE,
        0,
    );
}

/* --- row 10: the `i < 100` loop cap --- */

#[test]
fn err_row10_loop_cap_100() {
    let p = Pair::load();
    let mut r = rng_for(10);
    // S <= 63 with S + P >= 64 drives bw->bits to 63, after which b == 0 every
    // iteration, so the loop can only terminate via the i == 100 cap.
    for i in 0..ITERS {
        let s = r.range_u32(0, 63);
        let bits = r.range_u32(64u32.saturating_sub(s).max(1), 64);
        assert!(s.wrapping_add(bits) >= 64);
        let bw = r.bw_with_bits(s);
        let val = r.next_u64();
        check_ret(&p, "err10 loop cap i<100", i, &bw, bits, val, 0);
    }
    // Canonical cap case: S = 63, P = 1 -> b == 0 forever.
    let bw = Bitwriter {
        bits: 63,
        val: 0,
        ..Bitwriter::default()
    };
    check_ret(&p, "err10 S=63 P=1 cap", 0, &bw, 1, u64::MAX, 0);
}

/* --- row 11: `bits -= b` underflows u32 --- */

#[test]
fn err_row11_bits_minus_b_underflow() {
    let p = Pair::load();
    let mut r = rng_for(11);
    // `b` is clamped to `bits` by the ternary when `b > bits`, so a genuine
    // underflow needs the wrapped-`b` path where `63 - bw->bits` is huge and
    // still <= bits after the unsigned compare -- exercise the whole space of
    // S > 63 against large `bits` to hit it.
    for i in 0..ITERS * 2 {
        let s = r.next_u32() | 0x40; // force S >= 64 so 63 - S wraps
        let bits = if i % 3 == 0 { u32::MAX } else { r.next_u32() };
        let bw = r.bw_with_bits(s);
        let val = r.next_u64();
        check_ret(&p, "err11 bits-=b underflow path", i, &bw, bits, val, 0);
    }
    // Deterministic: S = 64 -> b = 0xFFFFFFFF; bits = U32MAX -> b not clamped,
    // bits -= 0xFFFFFFFF.
    let bw = Bitwriter {
        bits: 64,
        ..Bitwriter::default()
    };
    check_ret(&p, "err11 S=64 P=U32MAX", 0, &bw, u32::MAX, u64::MAX, 0);
}

/* --- row 12: NULL `bw` -- no null check exists in the C --- */

#[test]
fn err_row12_null_pointer_parity() {
    use std::os::unix::process::ExitStatusExt;
    use std::process::Command;

    // Child mode: perform the null call in a disposable process.
    if let Ok(which) = std::env::var("HARVEST_NULL_CHILD") {
        let p = Pair::load();
        let f = if which == "c" { p.c_add } else { p.rs_add };
        let ret = unsafe { f(std::ptr::null_mut(), 8, 0xFF) };
        // Should be unreachable: the C dereferences `bw` with no null check.
        println!("UNEXPECTEDLY SURVIVED: ret = {ret}");
        std::process::exit(0);
    }

    let exe = std::env::current_exe().expect("current_exe");
    let mut outcome = Vec::new();
    for which in ["c", "rust"] {
        let out = Command::new(&exe)
            .args(["err_row12_null_pointer_parity", "--exact", "--nocapture"])
            .env("HARVEST_NULL_CHILD", which)
            .env("RUST_BACKTRACE", "0")
            .output()
            .expect("spawn child");
        outcome.push((which, out.status.signal(), out.status.code()));
    }

    let (_, c_sig, c_code) = outcome[0];
    let (_, rs_sig, rs_code) = outcome[1];

    assert_eq!(
        c_sig, rs_sig,
        "null-pointer termination signal differs: C {c_sig:?} vs Rust {rs_sig:?} (codes {c_code:?} / {rs_code:?})"
    );
    assert_eq!(
        c_code, rs_code,
        "null-pointer exit code differs: C {c_code:?} vs Rust {rs_code:?}"
    );
    assert_eq!(
        c_sig,
        Some(11),
        "expected both to die with SIGSEGV (11); got {c_sig:?}"
    );
}

/* --- row 13: extreme `val` operands --- */

#[test]
fn err_row13_val_extremes() {
    let p = Pair::load();
    let mut r = rng_for(13);
    for i in 0..ITERS {
        let s = r.range_u32(0, 96);
        let bits = r.range_u32(0, 96);
        let bw = r.bw_with_bits(s);
        for &val in &[
            0u64,
            1,
            u64::MAX,
            1u64 << 63,
            u64::MAX >> 1,
            0xAAAA_AAAA_AAAA_AAAA,
            0x5555_5555_5555_5555,
        ] {
            check_ret(&p, "err13 val extremes", i, &bw, bits, val, 0);
        }
    }
}

/* --- row 14: fields the function never validates or uses --- */

#[test]
fn err_row14_unused_fields_untouched() {
    let p = Pair::load();
    let mut r = rng_for(14);

    // A misaligned, definitely-not-writable-as-a-buffer sentinel, plus a real
    // allocation whose bytes must remain untouched.
    let mut scratch = vec![0x5Au8; 128];
    let real = scratch.as_mut_ptr();

    for i in 0..ITERS {
        let s = r.range_u32(0, 96);
        let bits = r.range_u32(0, 96);
        let buffer = match i % 3 {
            0 => real,
            1 => 0x1usize as *mut u8,   // wild, misaligned
            _ => usize::MAX as *mut u8, // wild, non-canonical
        };
        let bw = Bitwriter {
            val: r.next_u64(),
            bits: s,
            pos: r.next_u32(),
            len: r.next_u32(),
            tot: r.next_u32(),
            buffer,
        };
        let val = r.next_u64();
        check_ret(&p, "err14 unused fields", i, &bw, bits, val, 0);

        // pos / len / buffer must be byte-identical afterwards in BOTH.
        let mut c_bw = bw;
        let mut rs_bw = bw;
        unsafe { (p.c_add)(&mut c_bw, bits, val) };
        unsafe { (p.rs_add)(&mut rs_bw, bits, val) };
        assert_eq!(
            (c_bw.pos, c_bw.len, c_bw.buffer),
            (bw.pos, bw.len, bw.buffer)
        );
        assert_eq!(
            (rs_bw.pos, rs_bw.len, rs_bw.buffer),
            (bw.pos, bw.len, bw.buffer)
        );
    }

    assert!(
        scratch.iter().all(|&b| b == 0x5A),
        "buffer contents were modified"
    );
}
