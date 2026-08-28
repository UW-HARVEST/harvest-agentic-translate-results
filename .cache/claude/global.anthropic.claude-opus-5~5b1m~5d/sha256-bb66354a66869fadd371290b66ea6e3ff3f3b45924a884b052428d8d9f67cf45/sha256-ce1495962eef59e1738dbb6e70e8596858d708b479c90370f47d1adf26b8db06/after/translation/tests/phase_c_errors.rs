//! Phase C — error-path / boundary differential tests, one test per
//! `ERRORS.md` row.
//!
//! This library has an empty error surface (no `if`, no `assert`, no error
//! return, no enum -- see `ERRORS.md` for the grep that establishes this), so
//! each test asserts that C and Rust agree on the *exact same* well-defined
//! result for the boundary condition, rather than merely "both failed".

mod common;

use common::*;
use std::ffi::c_void;

// ---------------------------------------------------------------------------
// Row 1: there is no error-return path at all.
// ---------------------------------------------------------------------------

#[test]
fn err_no_error_return_path_exists() {
    // Drive a wide spread of inputs and assert C and Rust always agree. Since
    // every `size_t` is a legal hash there is no sentinel to check for; the
    // contract is "always returns, always agrees".
    let mut rng = Rng::new(PRNG_SEED ^ 101);
    let mut buf = vec![0u8; 200];
    let mut distinct = std::collections::HashSet::new();
    for s in 0..2000 {
        rng.fill(&mut buf);
        let len = rng.below(buf.len() + 1);
        let seed = rng.seed_value();
        let v = diff_hash(&buf, len, seed, &format!("err01 sample={s}"));
        distinct.insert(v);
    }
    // Sanity: the function really is returning varied data (not a constant
    // sentinel that would mask divergence).
    assert!(distinct.len() > 1500, "suspiciously few distinct hashes: {}", distinct.len());
}

// ---------------------------------------------------------------------------
// Rows 2-5: NULL / garbage pointer with len == 0 (no dereference happens).
// ---------------------------------------------------------------------------

#[test]
fn err_null_ptr_zero_len() {
    let v = diff_hash_raw(std::ptr::null_mut(), 0, 0, "err02 NULL len=0 seed=0");
    // Must be a real value, produced without touching the pointer.
    println!("hash(NULL, 0, 0) = {v:#018x}");
}

#[test]
fn err_null_ptr_zero_len_seed_sweep() {
    let mut rng = Rng::new(PRNG_SEED ^ 103);
    for seed in [0usize, 1, 2, usize::MAX, usize::MAX - 1, usize::MAX / 2, 1 << 63, 1 << 32] {
        diff_hash_raw(std::ptr::null_mut(), 0, seed, &format!("err03 NULL len=0 seed={seed:#x}"));
    }
    for s in 0..1000 {
        let seed = rng.seed_value();
        diff_hash_raw(std::ptr::null_mut(), 0, seed, &format!("err03 random seed sample={s}"));
    }
}

#[test]
fn err_zero_len_valid_ptr_equals_null_ptr() {
    let mut rng = Rng::new(PRNG_SEED ^ 104);
    for seed in [0usize, 1, usize::MAX, 0xabcd_ef01_2345_6789] {
        let null_v = diff_hash_raw(std::ptr::null_mut(), 0, seed, "err04 null");
        for _ in 0..16 {
            let mut buf = vec![0u8; 32];
            rng.fill(&mut buf);
            let valid_v = diff_hash(&buf, 0, seed, "err04 valid ptr len=0");
            assert_eq!(
                null_v, valid_v,
                "err04: len=0 result depended on the pointer value (seed={seed:#x})"
            );
        }
    }
}

#[test]
fn err_zero_len_garbage_ptr() {
    // Non-null but completely invalid pointers. With len == 0 the C never
    // dereferences, so this must not fault and must match the NULL result.
    for raw in [1usize, 2, 0xdead_beef, usize::MAX, usize::MAX - 7, 0x1000_0000_0000_0000] {
        for seed in [0usize, usize::MAX, 12345] {
            let null_v = diff_hash_raw(std::ptr::null_mut(), 0, seed, "err05 null ref");
            let v = diff_hash_raw(
                raw as *mut c_void,
                0,
                seed,
                &format!("err05 garbage ptr={raw:#x} seed={seed:#x}"),
            );
            assert_eq!(v, null_v, "err05: garbage pointer {raw:#x} changed the len=0 result");
        }
    }
}

// ---------------------------------------------------------------------------
// Row 6: seed boundary values (SIZE_MAX -> ~seed == 0).
// ---------------------------------------------------------------------------

#[test]
fn err_seed_boundary_values() {
    let mut rng = Rng::new(PRNG_SEED ^ 106);
    let boundary_seeds = [
        0usize,
        1,
        usize::MAX,
        usize::MAX - 1,
        usize::MAX / 2,
        usize::MAX / 2 + 1,
        1 << 63,
        (1 << 63) - 1,
        1 << 32,
        (1 << 32) - 1,
        0xffff_ffff_0000_0000,
        0x0000_0000_ffff_ffff,
    ];
    for &seed in &boundary_seeds {
        for len in 0..=40usize {
            let mut buf = vec![0u8; len + 8];
            rng.fill(&mut buf);
            diff_hash(&buf, len, seed, &format!("err06 seed={seed:#018x} len={len}"));
        }
    }
}

// ---------------------------------------------------------------------------
// Row 7: misaligned pointer is accepted (no alignment check exists).
// ---------------------------------------------------------------------------

#[test]
fn err_misaligned_pointer_accepted() {
    let mut rng = Rng::new(PRNG_SEED ^ 107);
    let mut backing = vec![0u8; 96];
    rng.fill(&mut backing);
    for off in 1..8usize {
        for len in 0..=24usize {
            let mut a = backing.clone();
            let mut b = backing.clone();
            let seed = rng.seed_value();
            let cv =
                unsafe { (c_lib().hash_bytes)(a[off..].as_mut_ptr() as *mut c_void, len, seed) };
            let rv = unsafe {
                (rust_lib().hash_bytes)(b[off..].as_mut_ptr() as *mut c_void, len, seed)
            };
            assert_eq!(cv, rv, "err07 misaligned off={off} len={len} seed={seed:#x}");
        }
    }
}

// ---------------------------------------------------------------------------
// Row 8: len one step past each internal block boundary.
// ---------------------------------------------------------------------------

#[test]
fn err_len_one_past_block_boundaries() {
    let mut rng = Rng::new(PRNG_SEED ^ 108);
    let boundaries = [
        0usize, 1, 7, 8, 9, 15, 16, 17, 23, 24, 25, 31, 32, 33, 63, 64, 65, 127, 128, 129, 255,
        256, 257,
    ];
    let mut buf = vec![0u8; 300];
    for &len in &boundaries {
        for _ in 0..32 {
            rng.fill(&mut buf);
            let seed = rng.seed_value();
            diff_hash(&buf, len, seed, &format!("err08 boundary len={len} seed={seed:#x}"));
        }
    }
}

// ---------------------------------------------------------------------------
// Row 9: oversized len inside a genuinely large allocation.
// ---------------------------------------------------------------------------

#[test]
fn err_oversized_len_within_allocation() {
    let mut rng = Rng::new(PRNG_SEED ^ 109);
    let mut buf = vec![0u8; 1 << 20];
    rng.fill(&mut buf);
    for seed in [0usize, usize::MAX, 0x5555_5555_5555_5555] {
        diff_hash(&buf, buf.len(), seed, &format!("err09 len=1MiB seed={seed:#x}"));
    }
    // And a length that is large but not the whole buffer.
    diff_hash(&buf, (1 << 20) - 3, 0, "err09 len=1MiB-3");
}

// ---------------------------------------------------------------------------
// Row 10: the `switch (len - i)` default arm is unreachable -- assert the
// invariant that makes it so, and that both sides agree everywhere.
// ---------------------------------------------------------------------------

#[test]
fn err_switch_default_arm_unreachable() {
    // The C loop is `for (i = 0; i + 8 <= len; i += 8)`, so on exit
    // `len - i` is always in 0..=7. Prove the invariant over a wide range of
    // len values, mirroring the C arithmetic exactly.
    for len in 0..=4096usize {
        let mut i = 0usize;
        while i + core::mem::size_of::<usize>() <= len {
            i += core::mem::size_of::<usize>();
        }
        let rem = len.wrapping_sub(i);
        assert!(
            rem <= 7,
            "invariant broken: len={len} leaves len-i={rem}, which would reach the \
             switch default arm"
        );
    }

    // Every one of the 8 reachable residues, confirmed differentially.
    let mut rng = Rng::new(PRNG_SEED ^ 110);
    let mut seen = [false; 8];
    let mut buf = vec![0u8; 200];
    for len in 0..=128usize {
        rng.fill(&mut buf);
        seen[len % 8] = true;
        diff_hash(&buf, len, rng.seed_value(), &format!("err10 residue len={len}"));
    }
    assert!(seen.iter().all(|&x| x), "not all 8 tail residues were exercised");
}

// ---------------------------------------------------------------------------
// Extra generic FFI boundary probes beyond the table.
// ---------------------------------------------------------------------------

#[test]
fn err_extra_len_one_with_every_byte_value_and_edge_seeds() {
    for b in 0u16..=255 {
        let buf = [b as u8];
        for seed in [0usize, 1, usize::MAX, usize::MAX / 2, 1 << 63] {
            diff_hash(&buf, 1, seed, &format!("errX len=1 byte={b:#04x} seed={seed:#x}"));
        }
    }
}

#[test]
fn err_extra_single_bit_buffers() {
    // Exactly one bit set, at every bit position of a 16-byte buffer. Catches
    // off-by-one byte indexing in the tail arms and the block loader.
    for bit in 0usize..(16 * 8) {
        let mut buf = vec![0u8; 16];
        buf[bit / 8] = 1u8 << (bit % 8);
        for len in 0..=16usize {
            diff_hash(&buf, len, 0, &format!("errX single-bit bit={bit} len={len}"));
        }
    }
}

#[test]
fn err_extra_repeated_calls_do_not_drift_across_libraries() {
    // Alternate C and Rust on the same buffer; neither may leave residue that
    // changes the other's answer (catches accidental shared/static state).
    let mut rng = Rng::new(PRNG_SEED ^ 199);
    let mut buf = vec![0u8; 73];
    rng.fill(&mut buf);
    let mut work = buf.clone();
    let base_c = unsafe { (c_lib().hash_bytes)(work.as_mut_ptr() as *mut c_void, 73, 9) };
    let base_r = unsafe { (rust_lib().hash_bytes)(work.as_mut_ptr() as *mut c_void, 73, 9) };
    assert_eq!(base_c, base_r);
    for i in 0..500 {
        let r = unsafe { (rust_lib().hash_bytes)(work.as_mut_ptr() as *mut c_void, 73, 9) };
        let c = unsafe { (c_lib().hash_bytes)(work.as_mut_ptr() as *mut c_void, 73, 9) };
        assert_eq!((c, r), (base_c, base_r), "drift at iteration {i}");
    }
    assert_eq!(work, buf, "buffer mutated");
}

// ---------------------------------------------------------------------------
// Discovered C quirk: the `seed` parameter provably CANCELS OUT.
//
//   v0 = (C0 ^  seed) ; v0 ^= K0 ^  seed  =>  v0 = C0 ^ K0   (seed gone)
//   v1 = (C1 ^ ~seed) ; v1 ^= K1 ^ ~seed  =>  v1 = C1 ^ K1   (~seed gone)
//   v2, v3 likewise  (c_src/src/lib.c:10-17)
//
// So `stbds_hash_bytes` is seed-INDEPENDENT in the C. This is not a bug to
// "fix" -- the C is ground truth -- but it must be locked down, because a
// well-meaning change that made the Rust actually honour the seed would be a
// silent divergence that no seed-sweeping test could otherwise distinguish
// (every such test would still pass on the C side).
// ---------------------------------------------------------------------------

#[test]
fn quirk_seed_cancels_out_identically_in_both_libraries() {
    let mut rng = Rng::new(PRNG_SEED ^ 777);
    let seeds = [
        0usize,
        1,
        2,
        usize::MAX,
        usize::MAX - 1,
        usize::MAX / 2,
        1 << 63,
        1 << 32,
        0xdead_beef_cafe_babe,
        0x5555_5555_5555_5555,
        0xaaaa_aaaa_aaaa_aaaa,
    ];

    for len in 0..=80usize {
        let mut buf = vec![0u8; len + 8];
        rng.fill(&mut buf);

        // Reference values at seed 0, taken from each library.
        let c_ref = {
            let mut w = buf.clone();
            unsafe { (c_lib().hash_bytes)(w.as_mut_ptr() as *mut c_void, len, 0) }
        };
        let r_ref = {
            let mut w = buf.clone();
            unsafe { (rust_lib().hash_bytes)(w.as_mut_ptr() as *mut c_void, len, 0) }
        };
        assert_eq!(c_ref, r_ref, "quirk: C/Rust differ at seed 0, len={len}");

        for &seed in &seeds {
            let mut cw = buf.clone();
            let mut rw = buf.clone();
            let c = unsafe { (c_lib().hash_bytes)(cw.as_mut_ptr() as *mut c_void, len, seed) };
            let r = unsafe { (rust_lib().hash_bytes)(rw.as_mut_ptr() as *mut c_void, len, seed) };

            // 1. C and Rust agree.
            assert_eq!(c, r, "quirk: C/Rust differ at len={len} seed={seed:#x}");
            // 2. The C really is seed-independent (documents ground truth).
            assert_eq!(
                c, c_ref,
                "quirk: the C became seed-DEPENDENT at len={len} seed={seed:#x}. \
                 The premise of this test changed -- re-derive ERRORS.md/CONFIGS.md."
            );
            // 3. The Rust reproduces that seed-independence exactly.
            assert_eq!(
                r, r_ref,
                "quirk: the Rust became seed-DEPENDENT at len={len} seed={seed:#x}, \
                 but the C is seed-INDEPENDENT. The seed is XORed in twice in \
                 c_src/src/lib.c:10-17 and cancels; do not 'fix' this."
            );
        }
    }

    // Randomized: 5000 (bytes, len, seedA, seedB) quadruples.
    let mut buf = vec![0u8; 150];
    for s in 0..5000 {
        rng.fill(&mut buf);
        let len = rng.below(buf.len() + 1);
        let sa = rng.seed_value();
        let sb = rng.seed_value();
        let mut w1 = buf.clone();
        let mut w2 = buf.clone();
        let mut w3 = buf.clone();
        let mut w4 = buf.clone();
        let ca = unsafe { (c_lib().hash_bytes)(w1.as_mut_ptr() as *mut c_void, len, sa) };
        let cb = unsafe { (c_lib().hash_bytes)(w2.as_mut_ptr() as *mut c_void, len, sb) };
        let ra = unsafe { (rust_lib().hash_bytes)(w3.as_mut_ptr() as *mut c_void, len, sa) };
        let rb = unsafe { (rust_lib().hash_bytes)(w4.as_mut_ptr() as *mut c_void, len, sb) };
        assert_eq!(ca, cb, "quirk sample {s}: C seed-dependent (len={len})");
        assert_eq!(ra, rb, "quirk sample {s}: Rust seed-dependent (len={len})");
        assert_eq!(ca, ra, "quirk sample {s}: C/Rust differ (len={len})");
    }
}

/// The other two provably-equivalent mutations, asserted as properties so the
/// mutation sweep's "survived" verdicts are justified rather than assumed.
#[test]
fn quirk_equivalent_mutant_properties() {
    // (a) In `data |= ((hi as usize) << 16) << 16`, the sign-extension of `hi`
    //     is shifted entirely out by the total shift of 32, so sign- and
    //     zero-extension are indistinguishable there.
    let mut rng = Rng::new(PRNG_SEED ^ 778);
    for _ in 0..100_000 {
        let hi = rng.next_u64() as i32;
        assert_eq!(
            ((hi as usize) << 16) << 16,
            ((hi as u32 as usize) << 16) << 16,
            "sign- vs zero-extension became distinguishable for hi={hi:#x}"
        );
    }
    // Explicit edge cases.
    for hi in [0i32, 1, -1, i32::MIN, i32::MAX, -0x8000_0000i64 as i32, 0x7fff_ffff] {
        assert_eq!(((hi as usize) << 16) << 16, ((hi as u32 as usize) << 16) << 16);
    }

    // (b) The tail residue is always <= 7, so `rem == 7` and `rem >= 7` are
    //     equivalent guards. (Same invariant as
    //     `err_switch_default_arm_unreachable`, stated as the mutation property.)
    for len in 0..=8192usize {
        let sz = core::mem::size_of::<usize>();
        let mut i = 0usize;
        while i + sz <= len {
            i += sz;
        }
        let rem = len - i;
        assert!(rem <= 7);
        assert_eq!(rem == 7, rem >= 7, "guards diverge at len={len}");
    }
}
