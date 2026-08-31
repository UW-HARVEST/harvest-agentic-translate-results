//! Area 5 — `crypto_stream`, salsa20 family.
//!
//! Covers `configs_5.md` rows 5.11–5.46 (salsa20, salsa2012, salsa208,
//! xsalsa20) plus the salsa-side error rows of `errors_5.md`
//! (5.17, 5.18, 5.19, 5.23, 5.25, 5.26, 5.28).
//!
//! Sources of truth:
//!   c_src/libsodium/crypto_stream/salsa20/stream_salsa20.c + ref/salsa20_ref.c
//!   c_src/libsodium/crypto_stream/salsa2012/{stream_salsa2012.c,ref/stream_salsa2012_ref.c}
//!   c_src/libsodium/crypto_stream/salsa208/{stream_salsa208.c,ref/stream_salsa208_ref.c}
//!   c_src/libsodium/crypto_stream/xsalsa20/stream_xsalsa20.c

mod common;
use common::*;
use std::ffi::{c_int, c_void};

// ------------------------------------------------------------------ signatures

type Stream = unsafe extern "C" fn(*mut u8, u64, *const u8, *const u8) -> c_int;
type Xor = unsafe extern "C" fn(*mut u8, *const u8, u64, *const u8, *const u8) -> c_int;
type XorIc64 = unsafe extern "C" fn(*mut u8, *const u8, u64, *const u8, u64, *const u8) -> c_int;
type SizeFn = unsafe extern "C" fn() -> usize;
type Keygen = unsafe extern "C" fn(*mut u8);
type Core = unsafe extern "C" fn(*mut u8, *const u8, *const u8, *const u8) -> c_int;

// ------------------------------------------------------------------- constants

/// The length sweep **L** from `configs_5.md`.
const L: [usize; 16] = [
    0, 1, 63, 64, 65, 127, 128, 129, 191, 192, 193, 255, 256, 257, 511, 512,
];
/// A few multi-KiB lengths.
const BIG: [usize; 5] = [1024, 1025, 4096, 8191, 8192];

const SODIUM_SIZE_MAX: u64 = u64::MAX;

/// NaCl / libsodium `secretbox` test-vector key.
const TV_KEY: [u8; 32] = [
    0x1b, 0x27, 0x55, 0x64, 0x73, 0xe9, 0x85, 0xd4, 0x62, 0xcd, 0x51, 0x19, 0x7a, 0x9a, 0x46, 0xc7,
    0x60, 0x09, 0x54, 0x9e, 0xac, 0x64, 0x74, 0xf2, 0x06, 0xc4, 0xee, 0x08, 0x44, 0xf6, 0x83, 0x89,
];
/// NaCl / libsodium `secretbox` test-vector 24-byte nonce.
const TV_NONCE24: [u8; 24] = [
    0x69, 0x69, 0x6e, 0xe9, 0x55, 0xb6, 0x2b, 0x73, 0xcd, 0x62, 0xbd, 0xa8, 0x75, 0xfc, 0x73, 0xd6,
    0x82, 0x19, 0xe0, 0x03, 0x6b, 0x7a, 0x0b, 0x37,
];

/// `(key, nonce)` shapes: all-zero, all-0xff, ascending pattern, test vector,
/// then `extra` pseudorandom pairs.
fn shapes(nb: usize, rng: &mut Rng, extra: usize) -> Vec<(Vec<u8>, Vec<u8>)> {
    let mut v: Vec<(Vec<u8>, Vec<u8>)> = Vec::new();
    v.push((vec![0u8; 32], vec![0u8; nb]));
    v.push((vec![0xffu8; 32], vec![0xffu8; nb]));
    let mut k = vec![0u8; 32];
    for (i, b) in k.iter_mut().enumerate() {
        *b = i as u8 + 1;
    }
    let mut n = vec![0u8; nb];
    for (i, b) in n.iter_mut().enumerate() {
        *b = i as u8 + 1;
    }
    v.push((k, n));
    v.push((TV_KEY.to_vec(), TV_NONCE24[..nb].to_vec()));
    for _ in 0..extra {
        v.push((rng.bytes(32), rng.bytes(nb)));
    }
    v
}

// ---------------------------------------------------------------- generic driver

/// Exercises the keystream form and the `_xor` form of `base`:
///   * C vs Rust byte equality and return-code equality (row 5.86)
///   * `_xor` against an all-zero plaintext equals the keystream form
///   * `_xor` out-of-place matches `m ^ keystream`
///   * `_xor` is an involution
///   * in-place (`c == m`) equals out-of-place
///   * no writes past `len` (rows 5.87, guard bytes via `padded`/`check_pad`)
fn drive_keystream_and_xor(base: &str, nb: usize, seed: u64, lens: &[usize], extra: usize) {
    let (cs, rs) = both::<Stream>(base);
    let xor_name = format!("{base}_xor");
    let (cx, rx) = both::<Xor>(&xor_name);
    let mut rng = Rng::new(seed);

    for &len in lens {
        for (si, (k, n)) in shapes(nb, &mut rng, extra).into_iter().enumerate() {
            let tag = format!("{base}(len={len},shape={si})");

            // --- keystream form, out-of-place, poisoned tail
            let mut a = padded(len);
            let mut b = padded(len);
            let (rc, rr) = unsafe {
                (
                    cs(a.as_mut_ptr(), len as u64, n.as_ptr(), k.as_ptr()),
                    rs(b.as_mut_ptr(), len as u64, n.as_ptr(), k.as_ptr()),
                )
            };
            eqi(&tag, rc, rr);
            assert_eq!(rc, 0, "{tag}: C keystream form must return 0");
            eqb(&tag, &a, &b);
            check_pad(&tag, &a, len);
            check_pad(&tag, &b, len);
            let ks = a[..len].to_vec();

            // --- _xor with an all-zero plaintext == keystream form
            let zero = vec![0u8; len];
            let mut a2 = padded(len);
            let mut b2 = padded(len);
            let (rc, rr) = unsafe {
                (
                    cx(a2.as_mut_ptr(), zero.as_ptr(), len as u64, n.as_ptr(), k.as_ptr()),
                    rx(b2.as_mut_ptr(), zero.as_ptr(), len as u64, n.as_ptr(), k.as_ptr()),
                )
            };
            eqi(&format!("{tag} xor(zero)"), rc, rr);
            assert_eq!(rc, 0);
            eqb(&format!("{tag} xor(zero)"), &a2, &b2);
            check_pad(&format!("{tag} xor(zero)"), &a2, len);
            assert_eq!(
                &a2[..len],
                &ks[..],
                "{tag}: xor(zero) must equal the keystream form"
            );

            // --- _xor out-of-place with a pseudorandom message
            let m = rng.bytes(len);
            let mut a3 = padded(len);
            let mut b3 = padded(len);
            let (rc, rr) = unsafe {
                (
                    cx(a3.as_mut_ptr(), m.as_ptr(), len as u64, n.as_ptr(), k.as_ptr()),
                    rx(b3.as_mut_ptr(), m.as_ptr(), len as u64, n.as_ptr(), k.as_ptr()),
                )
            };
            eqi(&format!("{tag} xor"), rc, rr);
            eqb(&format!("{tag} xor"), &a3, &b3);
            check_pad(&format!("{tag} xor"), &a3, len);
            for i in 0..len {
                assert_eq!(a3[i], m[i] ^ ks[i], "{tag}: xor != m ^ keystream at {i}");
            }

            // --- involution
            let mut a4 = padded(len);
            let mut b4 = padded(len);
            unsafe {
                cx(a4.as_mut_ptr(), a3.as_ptr(), len as u64, n.as_ptr(), k.as_ptr());
                rx(b4.as_mut_ptr(), b3.as_ptr(), len as u64, n.as_ptr(), k.as_ptr());
            }
            eqb(&format!("{tag} xor^2"), &a4, &b4);
            assert_eq!(&a4[..len], &m[..], "{tag}: xor is not an involution");

            // --- in-place (c == m)
            let mut ip_c = padded(len);
            ip_c[..len].copy_from_slice(&m);
            let mut ip_r = ip_c.clone();
            unsafe {
                let pc = ip_c.as_mut_ptr();
                let pr = ip_r.as_mut_ptr();
                cx(pc, pc, len as u64, n.as_ptr(), k.as_ptr());
                rx(pr, pr, len as u64, n.as_ptr(), k.as_ptr());
            }
            eqb(&format!("{tag} xor in-place"), &ip_c, &ip_r);
            check_pad(&format!("{tag} xor in-place"), &ip_c, len);
            assert_eq!(
                &ip_c[..len],
                &a3[..len],
                "{tag}: in-place xor differs from out-of-place"
            );
        }
    }
}

/// Exercises `{base}_xor_ic` (64-bit `ic`).
fn drive_xor_ic64(
    base: &str,
    nb: usize,
    seed: u64,
    lens: &[usize],
    ics: &[u64],
    tail_check: bool,
    extra: usize,
) {
    let ic_name = format!("{base}_xor_ic");
    let (cxi, rxi) = both::<XorIc64>(&ic_name);
    let (cx, rx) = both::<Xor>(&format!("{base}_xor"));
    let mut rng = Rng::new(seed);

    for &ic in ics {
        for &len in lens {
            for (si, (k, n)) in shapes(nb, &mut rng, extra).into_iter().enumerate() {
                let tag = format!("{ic_name}(len={len},ic={ic:#x},shape={si})");
                let m = rng.bytes(len);

                let mut a = padded(len);
                let mut b = padded(len);
                let (rc, rr) = unsafe {
                    (
                        cxi(a.as_mut_ptr(), m.as_ptr(), len as u64, n.as_ptr(), ic, k.as_ptr()),
                        rxi(b.as_mut_ptr(), m.as_ptr(), len as u64, n.as_ptr(), ic, k.as_ptr()),
                    )
                };
                eqi(&tag, rc, rr);
                assert_eq!(rc, 0, "{tag}: must return 0");
                eqb(&tag, &a, &b);
                check_pad(&tag, &a, len);
                check_pad(&tag, &b, len);

                // in-place
                let mut ip_c = padded(len);
                ip_c[..len].copy_from_slice(&m);
                let mut ip_r = ip_c.clone();
                unsafe {
                    let pc = ip_c.as_mut_ptr();
                    let pr = ip_r.as_mut_ptr();
                    cxi(pc, pc, len as u64, n.as_ptr(), ic, k.as_ptr());
                    rxi(pr, pr, len as u64, n.as_ptr(), ic, k.as_ptr());
                }
                eqb(&format!("{tag} in-place"), &ip_c, &ip_r);
                check_pad(&format!("{tag} in-place"), &ip_c, len);
                assert_eq!(&ip_c[..len], &a[..len], "{tag}: in-place != out-of-place");

                // involution
                let mut inv_c = padded(len);
                let mut inv_r = padded(len);
                unsafe {
                    cxi(inv_c.as_mut_ptr(), a.as_ptr(), len as u64, n.as_ptr(), ic, k.as_ptr());
                    rxi(inv_r.as_mut_ptr(), b.as_ptr(), len as u64, n.as_ptr(), ic, k.as_ptr());
                }
                eqb(&format!("{tag} inv"), &inv_c, &inv_r);
                assert_eq!(&inv_c[..len], &m[..], "{tag}: xor_ic is not an involution");

                // ic == 0 must byte-equal the plain _xor form
                if ic == 0 {
                    let mut z_c = padded(len);
                    let mut z_r = padded(len);
                    unsafe {
                        cx(z_c.as_mut_ptr(), m.as_ptr(), len as u64, n.as_ptr(), k.as_ptr());
                        rx(z_r.as_mut_ptr(), m.as_ptr(), len as u64, n.as_ptr(), k.as_ptr());
                    }
                    eqb(&format!("{tag} vs _xor"), &z_c, &z_r);
                    assert_eq!(&z_c[..len], &a[..len], "{tag}: ic=0 must equal _xor");
                }

                // xor_ic(ic=k) must equal the tail of a long _xor starting at block k
                if tail_check && ic <= 8 && len > 0 {
                    let off = 64 * ic as usize;
                    let total = off + len;
                    let mut full = rng.bytes(total);
                    full[off..].copy_from_slice(&m);
                    let mut fc = vec![0u8; total];
                    let mut fr = vec![0u8; total];
                    unsafe {
                        cx(fc.as_mut_ptr(), full.as_ptr(), total as u64, n.as_ptr(), k.as_ptr());
                        rx(fr.as_mut_ptr(), full.as_ptr(), total as u64, n.as_ptr(), k.as_ptr());
                    }
                    eqb(&format!("{tag} tail"), &fc, &fr);
                    assert_eq!(
                        &fc[off..],
                        &a[..len],
                        "{tag}: xor_ic(ic) != tail of _xor at block ic"
                    );
                }
            }
        }
    }
}

/// Row 5.88: output for length `n1` must be a prefix of output for `n2 > n1`.
fn prefix_consistency(base: &str, nb: usize, seed: u64) {
    let (cs, rs) = both::<Stream>(base);
    let mut rng = Rng::new(seed);
    for (si, (k, n)) in shapes(nb, &mut rng, 3).into_iter().enumerate() {
        let mut prev: Vec<u8> = Vec::new();
        for &len in L.iter() {
            let mut a = padded(len);
            let mut b = padded(len);
            unsafe {
                cs(a.as_mut_ptr(), len as u64, n.as_ptr(), k.as_ptr());
                rs(b.as_mut_ptr(), len as u64, n.as_ptr(), k.as_ptr());
            }
            eqb(&format!("{base} prefix(len={len},shape={si})"), &a, &b);
            assert_eq!(
                &a[..prev.len()],
                &prev[..],
                "{base}: length {len} output is not an extension of the shorter one"
            );
            prev = a[..len].to_vec();
        }
    }
}

// ============================================================ accessors (5.11,
// 5.25, 5.32, 5.38 and error rows 5.25)

fn accessor(name: &str, expect: usize) {
    let (c, r) = both::<SizeFn>(name);
    let (vc, vr) = unsafe { (c(), r()) };
    assert_eq!(vc, vr, "{name}: C {vc} vs Rust {vr}");
    assert_eq!(vc, expect, "{name}: expected {expect}, C returned {vc}");
}

#[test]
fn salsa_family_accessors() {
    // 5.11
    accessor("crypto_stream_salsa20_keybytes", 32);
    accessor("crypto_stream_salsa20_noncebytes", 8);
    accessor("crypto_stream_salsa20_messagebytes_max", SODIUM_SIZE_MAX as usize);
    // 5.25
    accessor("crypto_stream_salsa2012_keybytes", 32);
    accessor("crypto_stream_salsa2012_noncebytes", 8);
    accessor("crypto_stream_salsa2012_messagebytes_max", SODIUM_SIZE_MAX as usize);
    // 5.32 (deprecated but present)
    accessor("crypto_stream_salsa208_keybytes", 32);
    accessor("crypto_stream_salsa208_noncebytes", 8);
    accessor("crypto_stream_salsa208_messagebytes_max", SODIUM_SIZE_MAX as usize);
    // 5.38
    accessor("crypto_stream_xsalsa20_keybytes", 32);
    accessor("crypto_stream_xsalsa20_noncebytes", 24);
    accessor("crypto_stream_xsalsa20_messagebytes_max", SODIUM_SIZE_MAX as usize);
}

// =================================================================== salsa20

#[test]
fn salsa20_keystream_and_xor_sweep() {
    // 5.12, 5.13, 5.14, 5.15, 5.16
    drive_keystream_and_xor("crypto_stream_salsa20", 8, 0x5_0012, &L, 4);
    drive_keystream_and_xor("crypto_stream_salsa20", 8, 0x5_0013, &BIG, 1);
}

#[test]
fn salsa20_full_length_sweep() {
    let lens: Vec<usize> = (0..=600).collect();
    drive_keystream_and_xor("crypto_stream_salsa20", 8, 0x5_0014, &lens, 0);
}

#[test]
fn salsa20_xor_ic_small() {
    // 5.17 (ic = 0), 5.18 (ic = 1), 5.19 (ic in {2,3,7})
    drive_xor_ic64(
        "crypto_stream_salsa20",
        8,
        0x5_0017,
        &L,
        &[0, 1, 2, 3, 7],
        true,
        3,
    );
}

#[test]
fn salsa20_xor_ic_counter_boundaries() {
    // 5.20: ic = 0xFFFFFFFF, carry from in[11] into in[12]
    // 5.21: ic = 0xFFFFFFFFFFFFFFFF, 64-bit block counter rolls over mid-message
    // 5.22: ic = 0xFFFFFFFFFFFFFFFE, two wraps across a 3-block message
    drive_xor_ic64(
        "crypto_stream_salsa20",
        8,
        0x5_0020,
        &[64, 65, 128, 129, 192],
        &[0xFFFF_FFFF, 0x1_0000_0000, 0xFFFF_FFFF_FFFF_FFFF],
        false,
        3,
    );
    drive_xor_ic64(
        "crypto_stream_salsa20",
        8,
        0x5_0022,
        &[129, 192, 193],
        &[0xFFFF_FFFF_FFFF_FFFE],
        false,
        3,
    );
    // Self-consistency: with ic = 2^64-1 the second block must use counter 0
    // (the carry out of in[15] is dropped) — error row 5.17.
    let (cxi, rxi) = both::<XorIc64>("crypto_stream_salsa20_xor_ic");
    let mut rng = Rng::new(0x5_0021);
    for (si, (k, n)) in shapes(8, &mut rng, 3).into_iter().enumerate() {
        let m = vec![0u8; 128];
        let mut wrap_c = vec![0u8; 128];
        let mut wrap_r = vec![0u8; 128];
        let mut zero_c = vec![0u8; 64];
        let mut zero_r = vec![0u8; 64];
        unsafe {
            let rc = cxi(
                wrap_c.as_mut_ptr(),
                m.as_ptr(),
                128,
                n.as_ptr(),
                u64::MAX,
                k.as_ptr(),
            );
            let rr = rxi(
                wrap_r.as_mut_ptr(),
                m.as_ptr(),
                128,
                n.as_ptr(),
                u64::MAX,
                k.as_ptr(),
            );
            eqi("salsa20 wrap rv", rc, rr);
            assert_eq!(rc, 0, "64-bit counter wrap must be silent (error row 5.17)");
            cxi(zero_c.as_mut_ptr(), m.as_ptr(), 64, n.as_ptr(), 0, k.as_ptr());
            rxi(zero_r.as_mut_ptr(), m.as_ptr(), 64, n.as_ptr(), 0, k.as_ptr());
        }
        eqb(&format!("salsa20 wrap(shape={si})"), &wrap_c, &wrap_r);
        eqb(&format!("salsa20 ic0(shape={si})"), &zero_c, &zero_r);
        assert_eq!(
            &wrap_c[64..],
            &zero_c[..],
            "salsa20: block after the 2^64-1 counter must be the counter-0 block"
        );
    }
}

#[test]
fn salsa20_xor_ic_zero_length() {
    // 5.23: mlen == 0 with ic in {0, 1, 2^64-1}; output buffer untouched.
    let (cxi, rxi) = both::<XorIc64>("crypto_stream_salsa20_xor_ic");
    let k = [7u8; 32];
    let n = [9u8; 8];
    for ic in [0u64, 1, u64::MAX] {
        let mut a = padded(0);
        let mut b = padded(0);
        let ref_a = a.clone();
        let (rc, rr) = unsafe {
            (
                cxi(a.as_mut_ptr(), core::ptr::null(), 0, n.as_ptr(), ic, k.as_ptr()),
                rxi(b.as_mut_ptr(), core::ptr::null(), 0, n.as_ptr(), ic, k.as_ptr()),
            )
        };
        eqi(&format!("salsa20_xor_ic(mlen=0,ic={ic:#x})"), rc, rr);
        assert_eq!(rc, 0);
        eqb(&format!("salsa20_xor_ic(mlen=0,ic={ic:#x})"), &a, &b);
        assert_eq!(a, ref_a, "mlen=0 must not touch the output buffer");
    }
}

// ================================================================= salsa2012

#[test]
fn salsa2012_keystream_and_xor_sweep() {
    // 5.26, 5.27, 5.28, 5.29
    drive_keystream_and_xor("crypto_stream_salsa2012", 8, 0x5_0026, &L, 4);
    drive_keystream_and_xor("crypto_stream_salsa2012", 8, 0x5_0027, &BIG, 1);
}

#[test]
fn salsa2012_full_length_sweep() {
    let lens: Vec<usize> = (0..=600).collect();
    drive_keystream_and_xor("crypto_stream_salsa2012", 8, 0x5_0029, &lens, 0);
}

// ================================================================== salsa208

#[test]
fn salsa208_keystream_and_xor_sweep() {
    // 5.33, 5.34, 5.35
    drive_keystream_and_xor("crypto_stream_salsa208", 8, 0x5_0033, &L, 4);
    drive_keystream_and_xor("crypto_stream_salsa208", 8, 0x5_0034, &BIG, 1);
}

#[test]
fn salsa208_full_length_sweep() {
    let lens: Vec<usize> = (0..=600).collect();
    drive_keystream_and_xor("crypto_stream_salsa208", 8, 0x5_0035, &lens, 0);
}

#[test]
fn salsa_variants_are_distinct_and_count_blocks() {
    // 5.30 / 5.36: no _xor_ic entry point → the counter always starts at 0 and
    // 512 bytes exercises 8 counter increments; the three round counts must
    // produce different keystreams for the same (n, k).
    let (c20, r20) = both::<Stream>("crypto_stream_salsa20");
    let (c12, r12) = both::<Stream>("crypto_stream_salsa2012");
    let (c08, r08) = both::<Stream>("crypto_stream_salsa208");
    let mut rng = Rng::new(0x5_0030);
    for (si, (k, n)) in shapes(8, &mut rng, 4).into_iter().enumerate() {
        let mut a20 = vec![0u8; 512];
        let mut b20 = vec![0u8; 512];
        let mut a12 = vec![0u8; 512];
        let mut b12 = vec![0u8; 512];
        let mut a08 = vec![0u8; 512];
        let mut b08 = vec![0u8; 512];
        unsafe {
            c20(a20.as_mut_ptr(), 512, n.as_ptr(), k.as_ptr());
            r20(b20.as_mut_ptr(), 512, n.as_ptr(), k.as_ptr());
            c12(a12.as_mut_ptr(), 512, n.as_ptr(), k.as_ptr());
            r12(b12.as_mut_ptr(), 512, n.as_ptr(), k.as_ptr());
            c08(a08.as_mut_ptr(), 512, n.as_ptr(), k.as_ptr());
            r08(b08.as_mut_ptr(), 512, n.as_ptr(), k.as_ptr());
        }
        eqb(&format!("salsa20 512(shape={si})"), &a20, &b20);
        eqb(&format!("salsa2012 512(shape={si})"), &a12, &b12);
        eqb(&format!("salsa208 512(shape={si})"), &a08, &b08);
        assert_ne!(a20, a12, "salsa20 and salsa2012 keystreams must differ");
        assert_ne!(a20, a08, "salsa20 and salsa208 keystreams must differ");
        assert_ne!(a12, a08, "salsa2012 and salsa208 keystreams must differ");
        // 8 distinct blocks: no block may repeat (the counter really advances).
        for i in 0..8usize {
            for j in (i + 1)..8usize {
                assert_ne!(
                    &a20[i * 64..(i + 1) * 64],
                    &a20[j * 64..(j + 1) * 64],
                    "salsa20 block {i} == block {j}"
                );
                assert_ne!(
                    &a12[i * 64..(i + 1) * 64],
                    &a12[j * 64..(j + 1) * 64],
                    "salsa2012 block {i} == block {j}"
                );
                assert_ne!(
                    &a08[i * 64..(i + 1) * 64],
                    &a08[j * 64..(j + 1) * 64],
                    "salsa208 block {i} == block {j}"
                );
            }
        }
    }
}

// ================================================================== xsalsa20

#[test]
fn xsalsa20_keystream_and_xor_sweep() {
    // 5.39, 5.40, 5.41
    drive_keystream_and_xor("crypto_stream_xsalsa20", 24, 0x5_0039, &L, 4);
    drive_keystream_and_xor("crypto_stream_xsalsa20", 24, 0x5_0040, &BIG, 1);
}

#[test]
fn xsalsa20_full_length_sweep() {
    let lens: Vec<usize> = (0..=600).collect();
    drive_keystream_and_xor("crypto_stream_xsalsa20", 24, 0x5_0041, &lens, 0);
}

#[test]
fn xsalsa20_xor_ic_small() {
    // 5.42 (ic = 0), 5.43 (ic = 1 and small)
    drive_xor_ic64(
        "crypto_stream_xsalsa20",
        24,
        0x5_0042,
        &L,
        &[0, 1, 2, 3, 7],
        true,
        3,
    );
}

#[test]
fn xsalsa20_xor_ic_counter_boundaries() {
    // 5.44: ic = 0xFFFFFFFF (32-bit boundary) and 2^64-1 (rollover mid-message)
    drive_xor_ic64(
        "crypto_stream_xsalsa20",
        24,
        0x5_0044,
        &[64, 65, 128, 129, 192],
        &[0xFFFF_FFFF, 0xFFFF_FFFF_FFFF_FFFF],
        false,
        3,
    );
    // Error row 5.17 for xsalsa20: silent 64-bit wraparound.
    let (cxi, rxi) = both::<XorIc64>("crypto_stream_xsalsa20_xor_ic");
    let mut rng = Rng::new(0x5_0045);
    for (si, (k, n)) in shapes(24, &mut rng, 3).into_iter().enumerate() {
        let m = vec![0u8; 128];
        let mut wrap_c = vec![0u8; 128];
        let mut wrap_r = vec![0u8; 128];
        let mut zero_c = vec![0u8; 64];
        let mut zero_r = vec![0u8; 64];
        unsafe {
            let rc = cxi(wrap_c.as_mut_ptr(), m.as_ptr(), 128, n.as_ptr(), u64::MAX, k.as_ptr());
            let rr = rxi(wrap_r.as_mut_ptr(), m.as_ptr(), 128, n.as_ptr(), u64::MAX, k.as_ptr());
            eqi("xsalsa20 wrap rv", rc, rr);
            assert_eq!(rc, 0);
            cxi(zero_c.as_mut_ptr(), m.as_ptr(), 64, n.as_ptr(), 0, k.as_ptr());
            rxi(zero_r.as_mut_ptr(), m.as_ptr(), 64, n.as_ptr(), 0, k.as_ptr());
        }
        eqb(&format!("xsalsa20 wrap(shape={si})"), &wrap_c, &wrap_r);
        eqb(&format!("xsalsa20 ic0(shape={si})"), &zero_c, &zero_r);
        assert_eq!(&wrap_c[64..], &zero_c[..]);
    }
}

#[test]
fn xsalsa20_equals_hsalsa20_plus_salsa20() {
    // 5.45
    let (chs, rhs) = both::<Core>("crypto_core_hsalsa20");
    let (cxs, rxs) = both::<Stream>("crypto_stream_xsalsa20");
    let (cs, rs) = both::<Stream>("crypto_stream_salsa20");
    let (cxsi, rxsi) = both::<XorIc64>("crypto_stream_xsalsa20_xor_ic");
    let (csi, rsi) = both::<XorIc64>("crypto_stream_salsa20_xor_ic");
    let mut rng = Rng::new(0x5_0046);
    for (si, (k, n)) in shapes(24, &mut rng, 5).into_iter().enumerate() {
        let mut sk_c = [0u8; 32];
        let mut sk_r = [0u8; 32];
        unsafe {
            chs(sk_c.as_mut_ptr(), n.as_ptr(), k.as_ptr(), core::ptr::null());
            rhs(sk_r.as_mut_ptr(), n.as_ptr(), k.as_ptr(), core::ptr::null());
        }
        eqb(&format!("hsalsa20 subkey(shape={si})"), &sk_c, &sk_r);
        for &len in [0usize, 64, 65, 512].iter() {
            let mut x_c = vec![0u8; len];
            let mut x_r = vec![0u8; len];
            let mut s_c = vec![0u8; len];
            let mut s_r = vec![0u8; len];
            unsafe {
                cxs(x_c.as_mut_ptr(), len as u64, n.as_ptr(), k.as_ptr());
                rxs(x_r.as_mut_ptr(), len as u64, n.as_ptr(), k.as_ptr());
                cs(s_c.as_mut_ptr(), len as u64, n.as_ptr().add(16), sk_c.as_ptr());
                rs(s_r.as_mut_ptr(), len as u64, n.as_ptr().add(16), sk_r.as_ptr());
            }
            eqb(&format!("xsalsa20({len},shape={si})"), &x_c, &x_r);
            eqb(&format!("salsa20 subkey({len},shape={si})"), &s_c, &s_r);
            assert_eq!(x_c, s_c, "xsalsa20 != salsa20(n+16, hsalsa20(n,k))");

            for ic in [0u64, 1, 5, 0xFFFF_FFFF] {
                let m = {
                    let mut r2 = Rng::new(len as u64 ^ ic);
                    r2.bytes(len)
                };
                let mut xi_c = vec![0u8; len];
                let mut xi_r = vec![0u8; len];
                let mut si_c = vec![0u8; len];
                let mut si_r = vec![0u8; len];
                unsafe {
                    cxsi(xi_c.as_mut_ptr(), m.as_ptr(), len as u64, n.as_ptr(), ic, k.as_ptr());
                    rxsi(xi_r.as_mut_ptr(), m.as_ptr(), len as u64, n.as_ptr(), ic, k.as_ptr());
                    csi(
                        si_c.as_mut_ptr(),
                        m.as_ptr(),
                        len as u64,
                        n.as_ptr().add(16),
                        ic,
                        sk_c.as_ptr(),
                    );
                    rsi(
                        si_r.as_mut_ptr(),
                        m.as_ptr(),
                        len as u64,
                        n.as_ptr().add(16),
                        ic,
                        sk_r.as_ptr(),
                    );
                }
                eqb(&format!("xsalsa20_xor_ic({len},{ic:#x},shape={si})"), &xi_c, &xi_r);
                eqb(&format!("salsa20_xor_ic({len},{ic:#x},shape={si})"), &si_c, &si_r);
                assert_eq!(xi_c, si_c, "xsalsa20_xor_ic != salsa20_xor_ic on the subkey");
            }
        }
    }
}

// ============================================================ prefix / exactness

#[test]
fn salsa_family_prefix_consistency() {
    // 5.87 / 5.88
    prefix_consistency("crypto_stream_salsa20", 8, 0x5_0088);
    prefix_consistency("crypto_stream_salsa2012", 8, 0x5_0089);
    prefix_consistency("crypto_stream_salsa208", 8, 0x5_008a);
    prefix_consistency("crypto_stream_xsalsa20", 24, 0x5_008b);
}

// ==================================================================== keygen

fn keygen_check(name: &str) {
    let (c, r) = both::<Keygen>(name);
    let mut a = padded(32);
    let mut b = padded(32);
    rng_reset();
    unsafe {
        c(a.as_mut_ptr());
        r(b.as_mut_ptr());
    }
    eqb(name, &a, &b);
    check_pad(name, &a, 32);
    check_pad(name, &b, 32);
    assert!(a[..32].iter().any(|&x| x != 0), "{name}: all-zero key");

    // Non-constant across calls with a different RNG stream position.
    let mut a2 = padded(32);
    let mut b2 = padded(32);
    rng_reseed(0x1234_5678_9abc_def1);
    unsafe {
        c(a2.as_mut_ptr());
        r(b2.as_mut_ptr());
    }
    eqb(&format!("{name} #2"), &a2, &b2);
    check_pad(&format!("{name} #2"), &a2, 32);
    assert_ne!(&a[..32], &a2[..32], "{name}: keygen output is constant");
    rng_reset();
}

#[test]
fn salsa_family_keygen() {
    keygen_check("crypto_stream_salsa20_keygen"); // 5.24
    keygen_check("crypto_stream_salsa2012_keygen"); // 5.31
    keygen_check("crypto_stream_salsa208_keygen"); // 5.37
    keygen_check("crypto_stream_xsalsa20_keygen"); // 5.46
}

// ======================================================== error rows (salsa)

extern "C" {
    fn mmap(
        addr: *mut c_void,
        len: usize,
        prot: c_int,
        flags: c_int,
        fd: c_int,
        off: i64,
    ) -> *mut c_void;
    fn mprotect(addr: *mut c_void, len: usize, prot: c_int) -> c_int;
}
const PROT_NONE: c_int = 0;
const PROT_READ: c_int = 1;
const PROT_WRITE: c_int = 2;
const MAP_PRIVATE: c_int = 0x02;
const MAP_ANONYMOUS: c_int = 0x20;
const MAP_NORESERVE: c_int = 0x4000;
const GUARD_USABLE: usize = 1 << 20;

/// A 1 MiB writable region immediately followed by a 1 MiB `PROT_NONE` guard.
///
/// Any implementation that keeps writing past the first MiB dies with SIGSEGV
/// *deterministically*, which lets us distinguish "the length check fired"
/// (SIGABRT from `sodium_misuse`) from "the length check did not fire".
fn guarded_region() -> *mut u8 {
    unsafe {
        let p = mmap(
            core::ptr::null_mut(),
            2 * GUARD_USABLE,
            PROT_NONE,
            MAP_PRIVATE | MAP_ANONYMOUS | MAP_NORESERVE,
            -1,
            0,
        );
        assert!(p as isize != -1, "mmap failed");
        assert_eq!(
            mprotect(p, GUARD_USABLE, PROT_READ | PROT_WRITE),
            0,
            "mprotect failed"
        );
        p as *mut u8
    }
}

#[test]
fn salsa_family_has_no_length_validation() {
    // Error rows 5.18 / 5.19: the salsa family performs no length validation at
    // all, so an absurd length does NOT abort — it just runs off the end of the
    // buffer.  With the guard mapping both libraries must die the same way
    // (SIGSEGV, not SIGABRT).  This is `checked-via-guard-only`: allocating a
    // > MESSAGEBYTES_MAX buffer is impossible.
    let p = guarded_region();
    let k = [0x5au8; 32];
    let n24 = [0x3cu8; 24];
    let huge: u64 = 274_877_906_945; // 2^38 + 1, way past the 1 MiB mapping
    for (name, nb) in [
        ("crypto_stream_salsa20", 8usize),
        ("crypto_stream_salsa2012", 8),
        ("crypto_stream_salsa208", 8),
        ("crypto_stream_xsalsa20", 24),
    ] {
        let (c, r) = both::<Stream>(name);
        let np = n24[..nb].as_ptr();
        eq_abort(
            &format!("{name}(clen=2^38+1) no validation"),
            || unsafe {
                c(p, huge, np, k.as_ptr());
            },
            || unsafe {
                r(p, huge, np, k.as_ptr());
            },
        );
    }
    for (name, nb) in [
        ("crypto_stream_salsa20_xor", 8usize),
        ("crypto_stream_salsa2012_xor", 8),
        ("crypto_stream_salsa208_xor", 8),
        ("crypto_stream_xsalsa20_xor", 24),
    ] {
        let (c, r) = both::<Xor>(name);
        let np = n24[..nb].as_ptr();
        eq_abort(
            &format!("{name}(mlen=2^38+1) no validation"),
            || unsafe {
                c(p, p, huge, np, k.as_ptr());
            },
            || unsafe {
                r(p, p, huge, np, k.as_ptr());
            },
        );
    }
    for name in [
        "crypto_stream_salsa20_xor_ic",
        "crypto_stream_xsalsa20_xor_ic",
    ] {
        let nb = if name.contains("xsalsa") { 24 } else { 8 };
        let (c, r) = both::<XorIc64>(name);
        let np = n24[..nb].as_ptr();
        eq_abort(
            &format!("{name}(mlen=2^38+1) no validation"),
            || unsafe {
                c(p, p, huge, np, u64::MAX, k.as_ptr());
            },
            || unsafe {
                r(p, p, huge, np, u64::MAX, k.as_ptr());
            },
        );
    }
}

#[test]
fn salsa_family_zero_length_with_null_pointers() {
    // Error row 5.23 (benign sub-case): mlen == 0 short-circuits before any
    // dereference, so (NULL, 0) happens to be safe in every implementation.
    let k = [1u8; 32];
    let n = [2u8; 24];
    for (name, nb) in [
        ("crypto_stream_salsa20", 8usize),
        ("crypto_stream_salsa2012", 8),
        ("crypto_stream_salsa208", 8),
        ("crypto_stream_xsalsa20", 24),
    ] {
        let (c, r) = both::<Stream>(name);
        let (rc, rr) = unsafe {
            (
                c(core::ptr::null_mut(), 0, n[..nb].as_ptr(), k.as_ptr()),
                r(core::ptr::null_mut(), 0, n[..nb].as_ptr(), k.as_ptr()),
            )
        };
        eqi(&format!("{name}(NULL,0)"), rc, rr);
        assert_eq!(rc, 0);
    }
    for (name, nb) in [
        ("crypto_stream_salsa20_xor", 8usize),
        ("crypto_stream_salsa2012_xor", 8),
        ("crypto_stream_salsa208_xor", 8),
        ("crypto_stream_xsalsa20_xor", 24),
    ] {
        let (c, r) = both::<Xor>(name);
        let (rc, rr) = unsafe {
            (
                c(
                    core::ptr::null_mut(),
                    core::ptr::null(),
                    0,
                    n[..nb].as_ptr(),
                    k.as_ptr(),
                ),
                r(
                    core::ptr::null_mut(),
                    core::ptr::null(),
                    0,
                    n[..nb].as_ptr(),
                    k.as_ptr(),
                ),
            )
        };
        eqi(&format!("{name}(NULL,0)"), rc, rr);
        assert_eq!(rc, 0);
    }
}

// ================================================= implementation selection

#[test]
fn salsa20_pick_best_implementation_is_stable() {
    // 5.89 / error row 5.27
    type Pick = unsafe extern "C" fn() -> c_int;
    let (cp, rp) = both::<Pick>("_crypto_stream_salsa20_pick_best_implementation");
    let (cs, rs) = both::<Stream>("crypto_stream_salsa20");
    let k = [0x11u8; 32];
    let n = [0x22u8; 8];
    let mut before_c = vec![0u8; 256];
    let mut before_r = vec![0u8; 256];
    unsafe {
        cs(before_c.as_mut_ptr(), 256, n.as_ptr(), k.as_ptr());
        rs(before_r.as_mut_ptr(), 256, n.as_ptr(), k.as_ptr());
    }
    eqb("salsa20 before pick", &before_c, &before_r);
    for _ in 0..3 {
        let (rc, rr) = unsafe { (cp(), rp()) };
        eqi("_crypto_stream_salsa20_pick_best_implementation", rc, rr);
        assert_eq!(rc, 0);
        let mut after_c = vec![0u8; 256];
        let mut after_r = vec![0u8; 256];
        unsafe {
            cs(after_c.as_mut_ptr(), 256, n.as_ptr(), k.as_ptr());
            rs(after_r.as_mut_ptr(), 256, n.as_ptr(), k.as_ptr());
        }
        eqb("salsa20 after pick", &after_c, &after_r);
        assert_eq!(after_c, before_c, "pick_best changed the output");
    }
}
