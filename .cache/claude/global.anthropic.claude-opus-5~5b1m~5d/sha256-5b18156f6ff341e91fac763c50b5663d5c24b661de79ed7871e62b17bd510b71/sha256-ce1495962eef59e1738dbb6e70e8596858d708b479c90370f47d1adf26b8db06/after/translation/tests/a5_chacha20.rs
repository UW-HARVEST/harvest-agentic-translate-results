//! Area 5 — `crypto_stream`, chacha20 family.
//!
//! Covers `configs_5.md` rows 5.47–5.83 (chacha20 "original", chacha20_ietf,
//! the private-but-exported `_ietf_ext*` entry points, and xchacha20) plus the
//! chacha-side error rows of `errors_5.md` (5.1–5.16, 5.20–5.22, 5.28, 5.29).
//!
//! Sources of truth:
//!   c_src/libsodium/crypto_stream/chacha20/stream_chacha20.c
//!   c_src/libsodium/crypto_stream/chacha20/ref/chacha20_ref.c
//!   c_src/libsodium/crypto_stream/xchacha20/stream_xchacha20.c
//!   c_src/libsodium/include/sodium/private/chacha20_ietf_ext.h

mod common;
use common::*;
use std::ffi::{c_int, c_void};

// ------------------------------------------------------------------ signatures

type Stream = unsafe extern "C" fn(*mut u8, u64, *const u8, *const u8) -> c_int;
type Xor = unsafe extern "C" fn(*mut u8, *const u8, u64, *const u8, *const u8) -> c_int;
type XorIc64 = unsafe extern "C" fn(*mut u8, *const u8, u64, *const u8, u64, *const u8) -> c_int;
type XorIc32 = unsafe extern "C" fn(*mut u8, *const u8, u64, *const u8, u32, *const u8) -> c_int;
type SizeFn = unsafe extern "C" fn() -> usize;
type Keygen = unsafe extern "C" fn(*mut u8);
type Core = unsafe extern "C" fn(*mut u8, *const u8, *const u8, *const u8) -> c_int;

// ------------------------------------------------------------------- constants

/// The length sweep **L** from `configs_5.md`.
const L: [usize; 16] = [
    0, 1, 63, 64, 65, 127, 128, 129, 191, 192, 193, 255, 256, 257, 511, 512,
];
const BIG: [usize; 5] = [1024, 1025, 4096, 8191, 8192];

const SODIUM_SIZE_MAX: u64 = u64::MAX;
/// `crypto_stream_chacha20_ietf_MESSAGEBYTES_MAX` = 64 * 2^32 = 2^38.
const IETF_MAX: u64 = 274_877_906_944;

/// RFC 7539 §2.4.2 key (00..1f) and 12-byte nonce.
const RFC_KEY: [u8; 32] = [
    0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x0d, 0x0e, 0x0f,
    0x10, 0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17, 0x18, 0x19, 0x1a, 0x1b, 0x1c, 0x1d, 0x1e, 0x1f,
];
const RFC_NONCE: [u8; 24] = [
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x4a, 0x00, 0x00, 0x00, 0x00, 0x0e, 0x1d, 0x2c, 0x3b,
    0x4a, 0x59, 0x68, 0x77, 0x86, 0x95, 0xa4, 0xb3,
];

/// `(key, nonce)` shapes: all-zero, all-0xff, ascending pattern, RFC vector,
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
    v.push((RFC_KEY.to_vec(), RFC_NONCE[..nb].to_vec()));
    for _ in 0..extra {
        v.push((rng.bytes(32), rng.bytes(nb)));
    }
    v
}

// ---------------------------------------------------------------- generic driver

/// Keystream form + `_xor` form: C/Rust equality, `_xor(zero) == keystream`,
/// `_xor` involution, in-place == out-of-place, no writes past `len`.
fn drive_keystream_and_xor(base: &str, nb: usize, seed: u64, lens: &[usize], extra: usize) {
    let (cs, rs) = both::<Stream>(base);
    let xor_name = format!("{base}_xor");
    let (cx, rx) = both::<Xor>(&xor_name);
    let mut rng = Rng::new(seed);

    for &len in lens {
        for (si, (k, n)) in shapes(nb, &mut rng, extra).into_iter().enumerate() {
            let tag = format!("{base}(len={len},shape={si})");

            let mut a = padded(len);
            let mut b = padded(len);
            let (rc, rr) = unsafe {
                (
                    cs(a.as_mut_ptr(), len as u64, n.as_ptr(), k.as_ptr()),
                    rs(b.as_mut_ptr(), len as u64, n.as_ptr(), k.as_ptr()),
                )
            };
            eqi(&tag, rc, rr);
            assert_eq!(rc, 0, "{tag}: keystream form must return 0");
            eqb(&tag, &a, &b);
            check_pad(&tag, &a, len);
            check_pad(&tag, &b, len);
            let ks = a[..len].to_vec();

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
            assert_eq!(&a2[..len], &ks[..], "{tag}: xor(zero) != keystream form");

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

            let mut a4 = padded(len);
            let mut b4 = padded(len);
            unsafe {
                cx(a4.as_mut_ptr(), a3.as_ptr(), len as u64, n.as_ptr(), k.as_ptr());
                rx(b4.as_mut_ptr(), b3.as_ptr(), len as u64, n.as_ptr(), k.as_ptr());
            }
            eqb(&format!("{tag} xor^2"), &a4, &b4);
            assert_eq!(&a4[..len], &m[..], "{tag}: xor is not an involution");

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
            assert_eq!(&ip_c[..len], &a3[..len], "{tag}: in-place != out-of-place");
        }
    }
}

/// `{base}_xor_ic` with a 64-bit `ic` (chacha20 "original", xchacha20).
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

                let mut inv_c = padded(len);
                let mut inv_r = padded(len);
                unsafe {
                    cxi(inv_c.as_mut_ptr(), a.as_ptr(), len as u64, n.as_ptr(), ic, k.as_ptr());
                    rxi(inv_r.as_mut_ptr(), b.as_ptr(), len as u64, n.as_ptr(), ic, k.as_ptr());
                }
                eqb(&format!("{tag} inv"), &inv_c, &inv_r);
                assert_eq!(&inv_c[..len], &m[..], "{tag}: not an involution");

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

/// `{base}_xor_ic` with a 32-bit `ic` (chacha20_ietf, chacha20_ietf_ext).
///
/// `xor_ref` is the name of the `_xor` entry point to cross-check `ic == 0`
/// against (`None` for `_ietf_ext_xor_ic`, whose `_xor` sibling is `static`).
fn drive_xor_ic32(
    base: &str,
    nb: usize,
    seed: u64,
    lens: &[usize],
    ics: &[u32],
    xor_ref: Option<&str>,
    tail_check: bool,
    extra: usize,
) {
    let ic_name = format!("{base}_xor_ic");
    let (cxi, rxi) = both::<XorIc32>(&ic_name);
    let xr = xor_ref.map(|s| both::<Xor>(s));
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

                let mut inv_c = padded(len);
                let mut inv_r = padded(len);
                unsafe {
                    cxi(inv_c.as_mut_ptr(), a.as_ptr(), len as u64, n.as_ptr(), ic, k.as_ptr());
                    rxi(inv_r.as_mut_ptr(), b.as_ptr(), len as u64, n.as_ptr(), ic, k.as_ptr());
                }
                eqb(&format!("{tag} inv"), &inv_c, &inv_r);
                assert_eq!(&inv_c[..len], &m[..], "{tag}: not an involution");

                if ic == 0 {
                    if let Some((cx, rx)) = xr.as_ref() {
                        let mut z_c = padded(len);
                        let mut z_r = padded(len);
                        unsafe {
                            cx(z_c.as_mut_ptr(), m.as_ptr(), len as u64, n.as_ptr(), k.as_ptr());
                            rx(z_r.as_mut_ptr(), m.as_ptr(), len as u64, n.as_ptr(), k.as_ptr());
                        }
                        eqb(&format!("{tag} vs _xor"), &z_c, &z_r);
                        assert_eq!(&z_c[..len], &a[..len], "{tag}: ic=0 must equal _xor");
                    }
                }

                if tail_check && ic <= 8 && len > 0 {
                    let off = 64 * ic as usize;
                    let total = off + len;
                    let mut full = rng.bytes(total);
                    full[off..].copy_from_slice(&m);
                    let mut fc = vec![0u8; total];
                    let mut fr = vec![0u8; total];
                    unsafe {
                        cxi(fc.as_mut_ptr(), full.as_ptr(), total as u64, n.as_ptr(), 0, k.as_ptr());
                        rxi(fr.as_mut_ptr(), full.as_ptr(), total as u64, n.as_ptr(), 0, k.as_ptr());
                    }
                    eqb(&format!("{tag} tail"), &fc, &fr);
                    assert_eq!(
                        &fc[off..],
                        &a[..len],
                        "{tag}: xor_ic(ic) != tail of xor_ic(0) at block ic"
                    );
                }
            }
        }
    }
}

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

// ================================================================= accessors

fn accessor(name: &str, expect: usize) {
    let (c, r) = both::<SizeFn>(name);
    let (vc, vr) = unsafe { (c(), r()) };
    assert_eq!(vc, vr, "{name}: C {vc} vs Rust {vr}");
    assert_eq!(vc, expect, "{name}: expected {expect}, C returned {vc}");
}

#[test]
fn chacha20_family_accessors() {
    // 5.47
    accessor("crypto_stream_chacha20_keybytes", 32);
    accessor("crypto_stream_chacha20_noncebytes", 8);
    accessor("crypto_stream_chacha20_messagebytes_max", SODIUM_SIZE_MAX as usize);
    // 5.59 — note the distinct IETF message limit
    accessor("crypto_stream_chacha20_ietf_keybytes", 32);
    accessor("crypto_stream_chacha20_ietf_noncebytes", 12);
    accessor("crypto_stream_chacha20_ietf_messagebytes_max", IETF_MAX as usize);
    assert_eq!(IETF_MAX, 64u64 * (1u64 << 32));
    // 5.75
    accessor("crypto_stream_xchacha20_keybytes", 32);
    accessor("crypto_stream_xchacha20_noncebytes", 24);
    accessor("crypto_stream_xchacha20_messagebytes_max", SODIUM_SIZE_MAX as usize);
}

#[test]
fn chacha20_ietf_legacy_aliases() {
    // 5.74: `crypto_stream_chacha20_IETF_*` are header-only #defines that expand
    // to their lowercase counterparts, so there is no separate symbol to load —
    // the accessors are the only observable surface and must agree.
    let (ck, rk) = both::<SizeFn>("crypto_stream_chacha20_ietf_keybytes");
    let (cn, rn) = both::<SizeFn>("crypto_stream_chacha20_ietf_noncebytes");
    let (cm, rm) = both::<SizeFn>("crypto_stream_chacha20_ietf_messagebytes_max");
    unsafe {
        assert_eq!(ck(), 32);
        assert_eq!(rk(), 32);
        assert_eq!(cn(), 12);
        assert_eq!(rn(), 12);
        assert_eq!(cm(), IETF_MAX as usize);
        assert_eq!(rm(), IETF_MAX as usize);
    }
    assert!(
        !has("crypto_stream_chacha20_IETF_KEYBYTES"),
        "the legacy alias must remain a macro, not a symbol"
    );
}

// ====================================================== chacha20 (original)

#[test]
fn chacha20_keystream_and_xor_sweep() {
    // 5.48, 5.49, 5.50, 5.51
    drive_keystream_and_xor("crypto_stream_chacha20", 8, 0x5_0048, &L, 4);
    drive_keystream_and_xor("crypto_stream_chacha20", 8, 0x5_0049, &BIG, 1);
}

#[test]
fn chacha20_full_length_sweep() {
    let lens: Vec<usize> = (0..=600).collect();
    drive_keystream_and_xor("crypto_stream_chacha20", 8, 0x5_0050, &lens, 0);
}

#[test]
fn chacha20_xor_ic_small() {
    // 5.52 (ic = 0), 5.53 (ic = 1), 5.54 (ic in {2,3,7})
    drive_xor_ic64(
        "crypto_stream_chacha20",
        8,
        0x5_0052,
        &L,
        &[0, 1, 2, 3, 7],
        true,
        3,
    );
}

#[test]
fn chacha20_xor_ic_counter_boundaries() {
    // 5.55: ic = 0xFFFFFFFF — j12 wraps and carries into j13 (counter high word)
    // 5.56: ic = 2^64-1 — the full 64-bit counter rolls over mid-message
    drive_xor_ic64(
        "crypto_stream_chacha20",
        8,
        0x5_0055,
        &[64, 65, 128, 129, 192],
        &[0xFFFF_FFFF, 0x1_0000_0000, 0xFFFF_FFFF_FFFF_FFFF, 0xFFFF_FFFF_FFFF_FFFE],
        false,
        3,
    );

    let (cxi, rxi) = both::<XorIc64>("crypto_stream_chacha20_xor_ic");
    let mut rng = Rng::new(0x5_0056);
    for (si, (k, n)) in shapes(8, &mut rng, 3).into_iter().enumerate() {
        let m = vec![0u8; 192];

        // Error row 5.14: ic = 0xFFFFFFFF crossing a 2^32 boundary is a *correct*
        // 64-bit increment for the original nonce layout, so block 1 must equal
        // the block at counter 2^32.
        let mut a = vec![0u8; 128];
        let mut b = vec![0u8; 128];
        let mut nxt_c = vec![0u8; 64];
        let mut nxt_r = vec![0u8; 64];
        unsafe {
            let rc = cxi(a.as_mut_ptr(), m.as_ptr(), 128, n.as_ptr(), 0xFFFF_FFFF, k.as_ptr());
            let rr = rxi(b.as_mut_ptr(), m.as_ptr(), 128, n.as_ptr(), 0xFFFF_FFFF, k.as_ptr());
            eqi("chacha20 32->64 carry rv", rc, rr);
            assert_eq!(rc, 0, "the 32->64 carry must not be an error (row 5.14)");
            cxi(nxt_c.as_mut_ptr(), m.as_ptr(), 64, n.as_ptr(), 0x1_0000_0000, k.as_ptr());
            rxi(nxt_r.as_mut_ptr(), m.as_ptr(), 64, n.as_ptr(), 0x1_0000_0000, k.as_ptr());
        }
        eqb(&format!("chacha20 carry(shape={si})"), &a, &b);
        eqb(&format!("chacha20 ic=2^32(shape={si})"), &nxt_c, &nxt_r);
        assert_eq!(&a[64..], &nxt_c[..], "32->64 carry produced the wrong block");

        // Error row 5.13: ic = 2^64-1 wraps silently to counter 0.
        let mut w_c = vec![0u8; 128];
        let mut w_r = vec![0u8; 128];
        let mut z_c = vec![0u8; 64];
        let mut z_r = vec![0u8; 64];
        unsafe {
            let rc = cxi(w_c.as_mut_ptr(), m.as_ptr(), 128, n.as_ptr(), u64::MAX, k.as_ptr());
            let rr = rxi(w_r.as_mut_ptr(), m.as_ptr(), 128, n.as_ptr(), u64::MAX, k.as_ptr());
            eqi("chacha20 wrap rv", rc, rr);
            assert_eq!(rc, 0, "the 64-bit wraparound must be silent (row 5.13)");
            cxi(z_c.as_mut_ptr(), m.as_ptr(), 64, n.as_ptr(), 0, k.as_ptr());
            rxi(z_r.as_mut_ptr(), m.as_ptr(), 64, n.as_ptr(), 0, k.as_ptr());
        }
        eqb(&format!("chacha20 wrap(shape={si})"), &w_c, &w_r);
        eqb(&format!("chacha20 ic0(shape={si})"), &z_c, &z_r);
        assert_eq!(
            &w_c[64..],
            &z_c[..],
            "chacha20: the block after counter 2^64-1 must be the counter-0 block"
        );
    }
}

#[test]
fn chacha20_xor_ic_zero_length() {
    // 5.57
    let (cxi, rxi) = both::<XorIc64>("crypto_stream_chacha20_xor_ic");
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
        eqi(&format!("chacha20_xor_ic(mlen=0,ic={ic:#x})"), rc, rr);
        assert_eq!(rc, 0);
        eqb(&format!("chacha20_xor_ic(mlen=0,ic={ic:#x})"), &a, &b);
        assert_eq!(a, ref_a, "mlen=0 must not touch the output buffer");
    }
}

// ========================================================== chacha20_ietf

#[test]
fn chacha20_ietf_keystream_and_xor_sweep() {
    // 5.60, 5.61, 5.62
    drive_keystream_and_xor("crypto_stream_chacha20_ietf", 12, 0x5_0060, &L, 4);
    drive_keystream_and_xor("crypto_stream_chacha20_ietf", 12, 0x5_0061, &BIG, 1);
}

#[test]
fn chacha20_ietf_full_length_sweep() {
    let lens: Vec<usize> = (0..=600).collect();
    drive_keystream_and_xor("crypto_stream_chacha20_ietf", 12, 0x5_0062, &lens, 0);
}

#[test]
fn chacha20_ietf_xor_ic_small() {
    // 5.63 (ic = 0), 5.64 (ic = 1, the RFC 7539 §2.4.2 counter), 5.65 (2, 3, 7)
    drive_xor_ic32(
        "crypto_stream_chacha20_ietf",
        12,
        0x5_0063,
        &L,
        &[0, 1, 2, 3, 7],
        Some("crypto_stream_chacha20_ietf_xor"),
        true,
        3,
    );
}

#[test]
fn chacha20_ietf_xor_ic_exact_accepted_boundary() {
    // 5.66: ic = 2^32 - ceil(mlen/64) is the largest accepted value; the counter
    // reaches 0xFFFFFFFF on the final block and never wraps.
    let cases: [(usize, u32); 7] = [
        (1, 0xFFFF_FFFF),
        (63, 0xFFFF_FFFF),
        (64, 0xFFFF_FFFF),
        (65, 0xFFFF_FFFE),
        (128, 0xFFFF_FFFE),
        (129, 0xFFFF_FFFD),
        (512, 0xFFFF_FFF8),
    ];
    let (cxi, rxi) = both::<XorIc32>("crypto_stream_chacha20_ietf_xor_ic");
    let (cei, rei) = both::<XorIc32>("crypto_stream_chacha20_ietf_ext_xor_ic");
    let mut rng = Rng::new(0x5_0066);
    for (mlen, ic) in cases {
        // Sanity: the C guard is `ic > 2^32 - ceil(mlen/64)`.
        let limit = 4_294_967_296u64 - ((mlen as u64 + 63) / 64);
        assert!(ic as u64 <= limit, "case ({mlen},{ic:#x}) is not accepted");
        assert_eq!(
            ic as u64 + ((mlen as u64 + 63) / 64),
            4_294_967_296u64,
            "case ({mlen},{ic:#x}) is not exactly on the boundary"
        );
        for (si, (k, n)) in shapes(12, &mut rng, 3).into_iter().enumerate() {
            let m = rng.bytes(mlen);
            let tag = format!("ietf_xor_ic boundary(mlen={mlen},ic={ic:#x},shape={si})");
            let mut a = padded(mlen);
            let mut b = padded(mlen);
            let (rc, rr) = unsafe {
                (
                    cxi(a.as_mut_ptr(), m.as_ptr(), mlen as u64, n.as_ptr(), ic, k.as_ptr()),
                    rxi(b.as_mut_ptr(), m.as_ptr(), mlen as u64, n.as_ptr(), ic, k.as_ptr()),
                )
            };
            eqi(&tag, rc, rr);
            assert_eq!(rc, 0, "{tag}: the exact boundary must be accepted");
            eqb(&tag, &a, &b);
            check_pad(&tag, &a, mlen);
            // Row 5.70: `_ext_xor_ic` must agree wherever the guard permits.
            let mut e_c = padded(mlen);
            let mut e_r = padded(mlen);
            unsafe {
                cei(e_c.as_mut_ptr(), m.as_ptr(), mlen as u64, n.as_ptr(), ic, k.as_ptr());
                rei(e_r.as_mut_ptr(), m.as_ptr(), mlen as u64, n.as_ptr(), ic, k.as_ptr());
            }
            eqb(&format!("{tag} ext"), &e_c, &e_r);
            assert_eq!(&e_c[..mlen], &a[..mlen], "{tag}: _ext_xor_ic disagrees");
        }
    }
}

#[test]
fn chacha20_ietf_xor_ic_one_past_boundary_aborts() {
    // 5.67 / error row 5.8: `ic = 2^32 + 1 - ceil(mlen/64)` must hit
    // sodium_misuse() -> abort().
    let cases: [(usize, u32); 4] = [
        (65, 0xFFFF_FFFF),
        (128, 0xFFFF_FFFF),
        (129, 0xFFFF_FFFE),
        (512, 0xFFFF_FFF9),
    ];
    let k = [0x33u8; 32];
    let n = [0x44u8; 12];
    for (mlen, ic) in cases {
        let limit = 4_294_967_296u64 - ((mlen as u64 + 63) / 64);
        assert!(ic as u64 > limit, "case ({mlen},{ic:#x}) is not past the limit");
        let m = vec![0u8; mlen];
        let mut out = vec![0u8; mlen];
        let (mp, op) = (m.as_ptr(), out.as_mut_ptr());
        let kp = k.as_ptr();
        let np = n.as_ptr();
        let (c, r) = both::<XorIc32>("crypto_stream_chacha20_ietf_xor_ic");
        eq_abort(
            &format!("ietf_xor_ic past boundary(mlen={mlen},ic={ic:#x})"),
            || unsafe {
                c(op, mp, mlen as u64, np, ic, kp);
            },
            || unsafe {
                r(op, mp, mlen as u64, np, ic, kp);
            },
        );
        // The abort happened in the child, so the parent's buffer is untouched.
        assert!(out.iter().all(|&x| x == 0));
    }
}

#[test]
fn chacha20_ietf_xor_ic_zero_length() {
    // 5.68: mlen = 0 → the guard limit is 2^32 and never fires, then the early
    // `return 0` leaves the output untouched.
    let (cxi, rxi) = both::<XorIc32>("crypto_stream_chacha20_ietf_xor_ic");
    let k = [0x5eu8; 32];
    let n = [0x6fu8; 12];
    for ic in [0u32, 1, 0xFFFF_FFFF] {
        let mut a = padded(0);
        let mut b = padded(0);
        let ref_a = a.clone();
        let (rc, rr) = unsafe {
            (
                cxi(a.as_mut_ptr(), core::ptr::null(), 0, n.as_ptr(), ic, k.as_ptr()),
                rxi(b.as_mut_ptr(), core::ptr::null(), 0, n.as_ptr(), ic, k.as_ptr()),
            )
        };
        eqi(&format!("ietf_xor_ic(mlen=0,ic={ic:#x})"), rc, rr);
        assert_eq!(rc, 0);
        eqb(&format!("ietf_xor_ic(mlen=0,ic={ic:#x})"), &a, &b);
        assert_eq!(a, ref_a);
    }
}

// ================================================== chacha20_ietf_ext (private)

#[test]
fn chacha20_ietf_ext_matches_ietf() {
    // 5.69
    let (ce, re) = both::<Stream>("crypto_stream_chacha20_ietf_ext");
    let (ci, ri) = both::<Stream>("crypto_stream_chacha20_ietf");
    let mut rng = Rng::new(0x5_0069);
    let mut lens: Vec<usize> = L.to_vec();
    lens.extend_from_slice(&BIG);
    for &len in lens.iter() {
        for (si, (k, n)) in shapes(12, &mut rng, 4).into_iter().enumerate() {
            let tag = format!("ietf_ext(len={len},shape={si})");
            let mut e_c = padded(len);
            let mut e_r = padded(len);
            let mut i_c = padded(len);
            let mut i_r = padded(len);
            let (rc, rr) = unsafe {
                (
                    ce(e_c.as_mut_ptr(), len as u64, n.as_ptr(), k.as_ptr()),
                    re(e_r.as_mut_ptr(), len as u64, n.as_ptr(), k.as_ptr()),
                )
            };
            eqi(&tag, rc, rr);
            assert_eq!(rc, 0);
            eqb(&tag, &e_c, &e_r);
            check_pad(&tag, &e_c, len);
            unsafe {
                ci(i_c.as_mut_ptr(), len as u64, n.as_ptr(), k.as_ptr());
                ri(i_r.as_mut_ptr(), len as u64, n.as_ptr(), k.as_ptr());
            }
            eqb(&format!("{tag} ietf"), &i_c, &i_r);
            assert_eq!(e_c, i_c, "{tag}: _ietf_ext != _ietf below 2^38");
        }
    }
}

#[test]
fn chacha20_ietf_ext_xor_ic_small() {
    // 5.70
    drive_xor_ic32(
        "crypto_stream_chacha20_ietf_ext",
        12,
        0x5_0070,
        &L,
        &[0, 1, 2, 3, 7],
        None,
        true,
        3,
    );
    // and it must byte-equal `_ietf_xor_ic` wherever the latter's guard permits
    let (cei, rei) = both::<XorIc32>("crypto_stream_chacha20_ietf_ext_xor_ic");
    let (cxi, rxi) = both::<XorIc32>("crypto_stream_chacha20_ietf_xor_ic");
    let mut rng = Rng::new(0x5_0071);
    for ic in [0u32, 1, 2, 3, 7] {
        for &len in L.iter() {
            for (si, (k, n)) in shapes(12, &mut rng, 2).into_iter().enumerate() {
                let m = rng.bytes(len);
                let mut e_c = padded(len);
                let mut e_r = padded(len);
                let mut x_c = padded(len);
                let mut x_r = padded(len);
                unsafe {
                    cei(e_c.as_mut_ptr(), m.as_ptr(), len as u64, n.as_ptr(), ic, k.as_ptr());
                    rei(e_r.as_mut_ptr(), m.as_ptr(), len as u64, n.as_ptr(), ic, k.as_ptr());
                    cxi(x_c.as_mut_ptr(), m.as_ptr(), len as u64, n.as_ptr(), ic, k.as_ptr());
                    rxi(x_r.as_mut_ptr(), m.as_ptr(), len as u64, n.as_ptr(), ic, k.as_ptr());
                }
                let tag = format!("ext_vs_ietf(len={len},ic={ic},shape={si})");
                eqb(&format!("{tag} ext"), &e_c, &e_r);
                eqb(&format!("{tag} ietf"), &x_c, &x_r);
                assert_eq!(e_c, x_c, "{tag}");
            }
        }
    }
}

#[test]
fn chacha20_ietf_ext_xor_ic_counter_overflows_into_the_iv() {
    // 5.71 / error row 5.16: `_ietf_ext_xor_ic` has no counter guard, so
    // ic = 0xFFFFFFFF with mlen > 64 wraps j12 to 0 and carries into j13, which
    // under `chacha_ietf_ivsetup` is nonce word 0.  The tail must therefore equal
    // a counter-0 run with the first nonce word incremented.
    let (cei, rei) = both::<XorIc32>("crypto_stream_chacha20_ietf_ext_xor_ic");
    let mut rng = Rng::new(0x5_0072);
    for &mlen in [65usize, 128, 129, 192].iter() {
        for (si, (k, n)) in shapes(12, &mut rng, 4).into_iter().enumerate() {
            let tag = format!("ext_overflow(mlen={mlen},shape={si})");
            let m = vec![0u8; mlen]; // zero plaintext → output is the keystream
            let mut a = padded(mlen);
            let mut b = padded(mlen);
            let (rc, rr) = unsafe {
                (
                    cei(
                        a.as_mut_ptr(),
                        m.as_ptr(),
                        mlen as u64,
                        n.as_ptr(),
                        0xFFFF_FFFF,
                        k.as_ptr(),
                    ),
                    rei(
                        b.as_mut_ptr(),
                        m.as_ptr(),
                        mlen as u64,
                        n.as_ptr(),
                        0xFFFF_FFFF,
                        k.as_ptr(),
                    ),
                )
            };
            eqi(&tag, rc, rr);
            assert_eq!(rc, 0, "{tag}: the overflow must be silent (no misuse)");
            eqb(&tag, &a, &b);
            check_pad(&tag, &a, mlen);

            // nonce with word 0 incremented (little-endian)
            let mut n2 = n.clone();
            let w0 = u32::from_le_bytes([n[0], n[1], n[2], n[3]]).wrapping_add(1);
            n2[..4].copy_from_slice(&w0.to_le_bytes());
            let tail = mlen - 64;
            let m2 = vec![0u8; tail];
            let mut t_c = padded(tail);
            let mut t_r = padded(tail);
            unsafe {
                cei(t_c.as_mut_ptr(), m2.as_ptr(), tail as u64, n2.as_ptr(), 0, k.as_ptr());
                rei(t_r.as_mut_ptr(), m2.as_ptr(), tail as u64, n2.as_ptr(), 0, k.as_ptr());
            }
            eqb(&format!("{tag} tail"), &t_c, &t_r);
            assert_eq!(
                &a[64..mlen],
                &t_c[..tail],
                "{tag}: the overflow did not carry into nonce word 0"
            );
        }
    }
}

// ========================================================== cross-variant

#[test]
fn ietf_and_original_layouts_are_distinct() {
    // 5.73
    let (ci, ri) = both::<Stream>("crypto_stream_chacha20_ietf");
    let (co, ro) = both::<Stream>("crypto_stream_chacha20");
    let mut rng = Rng::new(0x5_0073);
    for si in 0..8 {
        let k = rng.bytes(32);
        let n8 = rng.bytes(8);
        let mut n12 = n8.clone();
        n12.extend_from_slice(&[0u8; 4]);
        for &len in [64usize, 128].iter() {
            let mut i_c = vec![0u8; len];
            let mut i_r = vec![0u8; len];
            let mut o_c = vec![0u8; len];
            let mut o_r = vec![0u8; len];
            unsafe {
                ci(i_c.as_mut_ptr(), len as u64, n12.as_ptr(), k.as_ptr());
                ri(i_r.as_mut_ptr(), len as u64, n12.as_ptr(), k.as_ptr());
                co(o_c.as_mut_ptr(), len as u64, n8.as_ptr(), k.as_ptr());
                ro(o_r.as_mut_ptr(), len as u64, n8.as_ptr(), k.as_ptr());
            }
            eqb(&format!("ietf({len},{si})"), &i_c, &i_r);
            eqb(&format!("orig({len},{si})"), &o_c, &o_r);
            assert_ne!(
                i_c, o_c,
                "the ietf and original counter/nonce layouts must differ"
            );
        }
    }
}

// ================================================================ xchacha20

#[test]
fn xchacha20_keystream_and_xor_sweep() {
    // 5.76, 5.77, 5.78
    drive_keystream_and_xor("crypto_stream_xchacha20", 24, 0x5_0076, &L, 4);
    drive_keystream_and_xor("crypto_stream_xchacha20", 24, 0x5_0077, &BIG, 1);
}

#[test]
fn xchacha20_full_length_sweep() {
    let lens: Vec<usize> = (0..=600).collect();
    drive_keystream_and_xor("crypto_stream_xchacha20", 24, 0x5_0078, &lens, 0);
}

#[test]
fn xchacha20_xor_ic_small() {
    // 5.79 (ic = 0), 5.80 (ic = 1 and small)
    drive_xor_ic64(
        "crypto_stream_xchacha20",
        24,
        0x5_0079,
        &L,
        &[0, 1, 2, 3, 7],
        true,
        3,
    );
}

#[test]
fn xchacha20_xor_ic_no_ietf_guard() {
    // 5.81 / error row 5.12: `ic` is uint64_t and reaches the *original*
    // chacha20 path, so the IETF 32-bit constraints do not apply and no misuse
    // may occur.
    drive_xor_ic64(
        "crypto_stream_xchacha20",
        24,
        0x5_0081,
        &[64, 65, 128, 129, 192],
        &[0xFFFF_FFFF, 0x1_0000_0000, 0xFFFF_FFFF_FFFF_FFFF],
        false,
        3,
    );
}

#[test]
fn xchacha20_equals_hchacha20_plus_chacha20() {
    // 5.82
    let (chc, rhc) = both::<Core>("crypto_core_hchacha20");
    let (cxc, rxc) = both::<Stream>("crypto_stream_xchacha20");
    let (cc, rc_) = both::<Stream>("crypto_stream_chacha20");
    let (cxci, rxci) = both::<XorIc64>("crypto_stream_xchacha20_xor_ic");
    let (cci, rci) = both::<XorIc64>("crypto_stream_chacha20_xor_ic");
    let mut rng = Rng::new(0x5_0082);
    for (si, (k, n)) in shapes(24, &mut rng, 5).into_iter().enumerate() {
        let mut k2_c = [0u8; 32];
        let mut k2_r = [0u8; 32];
        unsafe {
            chc(k2_c.as_mut_ptr(), n.as_ptr(), k.as_ptr(), core::ptr::null());
            rhc(k2_r.as_mut_ptr(), n.as_ptr(), k.as_ptr(), core::ptr::null());
        }
        eqb(&format!("hchacha20 subkey(shape={si})"), &k2_c, &k2_r);
        for &len in [0usize, 64, 65, 512].iter() {
            let mut x_c = vec![0u8; len];
            let mut x_r = vec![0u8; len];
            let mut c_c = vec![0u8; len];
            let mut c_r = vec![0u8; len];
            unsafe {
                cxc(x_c.as_mut_ptr(), len as u64, n.as_ptr(), k.as_ptr());
                rxc(x_r.as_mut_ptr(), len as u64, n.as_ptr(), k.as_ptr());
                cc(c_c.as_mut_ptr(), len as u64, n.as_ptr().add(16), k2_c.as_ptr());
                rc_(c_r.as_mut_ptr(), len as u64, n.as_ptr().add(16), k2_r.as_ptr());
            }
            eqb(&format!("xchacha20({len},shape={si})"), &x_c, &x_r);
            eqb(&format!("chacha20 subkey({len},shape={si})"), &c_c, &c_r);
            assert_eq!(x_c, c_c, "xchacha20 != chacha20(n+16, hchacha20(n,k))");

            for ic in [0u64, 1, 5, 0xFFFF_FFFF, u64::MAX] {
                let m = {
                    let mut r2 = Rng::new(len as u64 ^ ic);
                    r2.bytes(len)
                };
                let mut xi_c = vec![0u8; len];
                let mut xi_r = vec![0u8; len];
                let mut ci_c = vec![0u8; len];
                let mut ci_r = vec![0u8; len];
                unsafe {
                    cxci(xi_c.as_mut_ptr(), m.as_ptr(), len as u64, n.as_ptr(), ic, k.as_ptr());
                    rxci(xi_r.as_mut_ptr(), m.as_ptr(), len as u64, n.as_ptr(), ic, k.as_ptr());
                    cci(
                        ci_c.as_mut_ptr(),
                        m.as_ptr(),
                        len as u64,
                        n.as_ptr().add(16),
                        ic,
                        k2_c.as_ptr(),
                    );
                    rci(
                        ci_r.as_mut_ptr(),
                        m.as_ptr(),
                        len as u64,
                        n.as_ptr().add(16),
                        ic,
                        k2_r.as_ptr(),
                    );
                }
                eqb(&format!("xchacha20_xor_ic({len},{ic:#x},{si})"), &xi_c, &xi_r);
                eqb(&format!("chacha20_xor_ic({len},{ic:#x},{si})"), &ci_c, &ci_r);
                assert_eq!(xi_c, ci_c, "xchacha20_xor_ic != chacha20_xor_ic on the subkey");
            }
        }
    }
}

// ============================================================ prefix / keygen

#[test]
fn chacha20_family_prefix_consistency() {
    // 5.87 / 5.88
    prefix_consistency("crypto_stream_chacha20", 8, 0x5_0087);
    prefix_consistency("crypto_stream_chacha20_ietf", 12, 0x5_0088);
    prefix_consistency("crypto_stream_chacha20_ietf_ext", 12, 0x5_0089);
    prefix_consistency("crypto_stream_xchacha20", 24, 0x5_008a);
}

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

    let mut a2 = padded(32);
    let mut b2 = padded(32);
    rng_reseed(0x0f1e_2d3c_4b5a_6978);
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
fn chacha20_family_keygen() {
    keygen_check("crypto_stream_chacha20_keygen"); // 5.58
    keygen_check("crypto_stream_chacha20_ietf_keygen"); // 5.72
    keygen_check("crypto_stream_xchacha20_keygen"); // 5.83
}

// ================================================ implementation selection

#[test]
fn chacha20_pick_best_implementation_is_stable() {
    // 5.89 / error row 5.27
    type Pick = unsafe extern "C" fn() -> c_int;
    let (cp, rp) = both::<Pick>("_crypto_stream_chacha20_pick_best_implementation");
    let (cs, rs) = both::<Stream>("crypto_stream_chacha20");
    let k = [0x11u8; 32];
    let n = [0x22u8; 8];
    let mut before_c = vec![0u8; 256];
    let mut before_r = vec![0u8; 256];
    unsafe {
        cs(before_c.as_mut_ptr(), 256, n.as_ptr(), k.as_ptr());
        rs(before_r.as_mut_ptr(), 256, n.as_ptr(), k.as_ptr());
    }
    eqb("chacha20 before pick", &before_c, &before_r);
    for _ in 0..3 {
        let (rc, rr) = unsafe { (cp(), rp()) };
        eqi("_crypto_stream_chacha20_pick_best_implementation", rc, rr);
        assert_eq!(rc, 0);
        let mut after_c = vec![0u8; 256];
        let mut after_r = vec![0u8; 256];
        unsafe {
            cs(after_c.as_mut_ptr(), 256, n.as_ptr(), k.as_ptr());
            rs(after_r.as_mut_ptr(), 256, n.as_ptr(), k.as_ptr());
        }
        eqb("chacha20 after pick", &after_c, &after_r);
        assert_eq!(after_c, before_c, "pick_best changed the output");
    }
}

// ============================================ length-limit error rows (guarded)

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
/// Buffers of `> 2^38` bytes cannot be allocated, so the length-limit rows are
/// exercised by handing in this bounded mapping together with an out-of-range
/// length.  The `sodium_misuse()` check always happens *before* the first
/// dereference (verified against the C source), so:
///   * "the check fired"     → SIGABRT (`sig:6`)
///   * "the check did not fire" → the implementation walks off the end of the
///     mapping and dies with SIGSEGV (`sig:11`)
/// and `eq_abort` pins C and Rust to the same outcome either way.
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

/// Runs `f` on both libraries under `in_child` and returns the two statuses.
fn outcomes<FC: FnOnce(), FR: FnOnce()>(what: &str, c: FC, r: FR) -> String {
    let sc = status_str(in_child(c));
    let sr = status_str(in_child(r));
    assert_eq!(sc, sr, "{what}: process outcome mismatch (C {sc}, Rust {sr})");
    sc
}

const SIGABRT_STATUS: &str = "sig:6";
const SIGSEGV_STATUS: &str = "sig:11";

#[test]
fn chacha20_ietf_messagebytes_max_is_enforced() {
    // Error rows 5.7 and 5.10 (both *reachable*): `_ietf` / `_ietf_xor` reject
    // clen/mlen > 2^38 with sodium_misuse(); exactly 2^38 is accepted.
    let p = guarded_region();
    let k = [0x21u8; 32];
    let n = [0x43u8; 12];
    let (kp, np) = (k.as_ptr(), n.as_ptr());

    let (ci, ri) = both::<Stream>("crypto_stream_chacha20_ietf");
    let s = outcomes(
        "crypto_stream_chacha20_ietf(clen=2^38+1)",
        || unsafe {
            ci(p, IETF_MAX + 1, np, kp);
        },
        || unsafe {
            ri(p, IETF_MAX + 1, np, kp);
        },
    );
    assert_eq!(s, SIGABRT_STATUS, "row 5.7: clen > 2^38 must abort");

    let (ci, ri) = both::<Stream>("crypto_stream_chacha20_ietf");
    let s = outcomes(
        "crypto_stream_chacha20_ietf(clen=2^38)",
        || unsafe {
            ci(p, IETF_MAX, np, kp);
        },
        || unsafe {
            ri(p, IETF_MAX, np, kp);
        },
    );
    assert_eq!(s, SIGSEGV_STATUS, "row 5.7: clen == 2^38 must be accepted");

    let (cx, rx) = both::<Xor>("crypto_stream_chacha20_ietf_xor");
    let s = outcomes(
        "crypto_stream_chacha20_ietf_xor(mlen=2^38+1)",
        || unsafe {
            cx(p, p, IETF_MAX + 1, np, kp);
        },
        || unsafe {
            rx(p, p, IETF_MAX + 1, np, kp);
        },
    );
    assert_eq!(s, SIGABRT_STATUS, "row 5.10: mlen > 2^38 must abort");

    let (cx, rx) = both::<Xor>("crypto_stream_chacha20_ietf_xor");
    let s = outcomes(
        "crypto_stream_chacha20_ietf_xor(mlen=2^38)",
        || unsafe {
            cx(p, p, IETF_MAX, np, kp);
        },
        || unsafe {
            rx(p, p, IETF_MAX, np, kp);
        },
    );
    assert_eq!(s, SIGSEGV_STATUS, "row 5.10: mlen == 2^38 must be accepted");
}

#[test]
fn chacha20_ietf_xor_ic_counter_guard_at_the_message_limit() {
    // Error row 5.8, the `mlen == 2^38` corner: the limit becomes 0, so any
    // ic >= 1 aborts while ic == 0 is accepted.
    let p = guarded_region();
    let k = [0x21u8; 32];
    let n = [0x43u8; 12];
    let (kp, np) = (k.as_ptr(), n.as_ptr());

    let (c1, r1) = both::<XorIc32>("crypto_stream_chacha20_ietf_xor_ic");
    let s = outcomes(
        "ietf_xor_ic(mlen=2^38, ic=1)",
        || unsafe {
            c1(p, p, IETF_MAX, np, 1, kp);
        },
        || unsafe {
            r1(p, p, IETF_MAX, np, 1, kp);
        },
    );
    assert_eq!(s, SIGABRT_STATUS, "row 5.8: limit is 0 at mlen == 2^38");

    let (c0, r0) = both::<XorIc32>("crypto_stream_chacha20_ietf_xor_ic");
    let s = outcomes(
        "ietf_xor_ic(mlen=2^38, ic=0)",
        || unsafe {
            c0(p, p, IETF_MAX, np, 0, kp);
        },
        || unsafe {
            r0(p, p, IETF_MAX, np, 0, kp);
        },
    );
    assert_eq!(s, SIGSEGV_STATUS, "row 5.8: ic == 0 is accepted at mlen == 2^38");
}

#[test]
fn chacha20_ietf_xor_ic_guard_underflow_hole() {
    // Error row 5.9: for mlen > 2^38 the RHS `2^32 - ceil(mlen/64)` underflows in
    // unsigned long long, so the guard silently passes.  `_ietf_xor` at the very
    // same length *does* abort — the asymmetry is the hole and must be preserved.
    let p = guarded_region();
    let k = [0x21u8; 32];
    let n = [0x43u8; 12];
    let (kp, np) = (k.as_ptr(), n.as_ptr());

    for mlen in [IETF_MAX + 1, IETF_MAX + 64, IETF_MAX * 2] {
        let (c, r) = both::<XorIc32>("crypto_stream_chacha20_ietf_xor_ic");
        let s = outcomes(
            &format!("ietf_xor_ic(mlen={mlen}, ic=0xFFFFFFFF) guard hole"),
            || unsafe {
                c(p, p, mlen, np, 0xFFFF_FFFF, kp);
            },
            || unsafe {
                r(p, p, mlen, np, 0xFFFF_FFFF, kp);
            },
        );
        assert_eq!(
            s, SIGSEGV_STATUS,
            "row 5.9: the underflowed guard must NOT abort at mlen={mlen}"
        );

        let (cx, rx) = both::<Xor>("crypto_stream_chacha20_ietf_xor");
        let s2 = outcomes(
            &format!("ietf_xor(mlen={mlen})"),
            || unsafe {
                cx(p, p, mlen, np, kp);
            },
            || unsafe {
                rx(p, p, mlen, np, kp);
            },
        );
        assert_eq!(s2, SIGABRT_STATUS);
        assert_ne!(
            s, s2,
            "row 5.9: _ietf_xor_ic must be more permissive than _ietf_xor"
        );
    }
}

#[test]
fn chacha20_ietf_ext_ignores_the_ietf_limit() {
    // Error rows 5.4 / 5.5 / 5.15: the `_ext` entry points deliberately check the
    // *non*-ietf maximum (UINT64_MAX, a dead branch on LP64), so they accept
    // clen/mlen far beyond 2^38 and let the 32-bit counter overflow into the IV.
    let p = guarded_region();
    let k = [0x21u8; 32];
    let n = [0x43u8; 12];
    let (kp, np) = (k.as_ptr(), n.as_ptr());

    let (ce, re) = both::<Stream>("crypto_stream_chacha20_ietf_ext");
    let s = outcomes(
        "crypto_stream_chacha20_ietf_ext(clen=2^38+1)",
        || unsafe {
            ce(p, IETF_MAX + 1, np, kp);
        },
        || unsafe {
            re(p, IETF_MAX + 1, np, kp);
        },
    );
    assert_eq!(s, SIGSEGV_STATUS, "row 5.4: _ietf_ext must not enforce 2^38");

    let (cei, rei) = both::<XorIc32>("crypto_stream_chacha20_ietf_ext_xor_ic");
    let s = outcomes(
        "crypto_stream_chacha20_ietf_ext_xor_ic(mlen=2^38+1, ic=0xFFFFFFFF)",
        || unsafe {
            cei(p, p, IETF_MAX + 1, np, 0xFFFF_FFFF, kp);
        },
        || unsafe {
            rei(p, p, IETF_MAX + 1, np, 0xFFFF_FFFF, kp);
        },
    );
    assert_eq!(s, SIGSEGV_STATUS, "row 5.5: _ietf_ext_xor_ic has no effective check");
}

#[test]
fn chacha20_original_and_xchacha20_accept_the_maximum_length() {
    // Error rows 5.1, 5.2, 5.3, 5.6, 5.11, 5.12: the `> UINT64_MAX` comparisons
    // are dead branches on LP64.  Feeding the largest representable length shows
    // that no misuse fires — the call just runs off the end of the mapping.
    let p = guarded_region();
    let k = [0x21u8; 32];
    let n = [0x43u8; 24];
    let (kp, np8, np24) = (k.as_ptr(), n.as_ptr(), n.as_ptr());

    // keystream forms: use 2^38+1 rather than UINT64_MAX so that the internal
    // memset() stays a plain forward walk off the end of the mapping.
    for name in ["crypto_stream_chacha20", "crypto_stream_xchacha20"] {
        let (c, r) = both::<Stream>(name);
        let np = if name.contains("xchacha") { np24 } else { np8 };
        let s = outcomes(
            &format!("{name}(clen=2^38+1)"),
            || unsafe {
                c(p, IETF_MAX + 1, np, kp);
            },
            || unsafe {
                r(p, IETF_MAX + 1, np, kp);
            },
        );
        assert_eq!(s, SIGSEGV_STATUS, "{name}: no length check may fire");
    }

    for name in [
        "crypto_stream_chacha20_xor",
        "crypto_stream_xchacha20_xor",
    ] {
        let np = if name.contains("xchacha") { np24 } else { np8 };
        for mlen in [IETF_MAX + 1, u64::MAX] {
            let (c, r) = both::<Xor>(name);
            let s = outcomes(
                &format!("{name}(mlen={mlen})"),
                || unsafe {
                    c(p, p, mlen, np, kp);
                },
                || unsafe {
                    r(p, p, mlen, np, kp);
                },
            );
            assert_eq!(s, SIGSEGV_STATUS, "{name}: no length check may fire");
        }
    }

    for name in [
        "crypto_stream_chacha20_xor_ic",
        "crypto_stream_xchacha20_xor_ic",
    ] {
        let (c, r) = both::<XorIc64>(name);
        let np = if name.contains("xchacha") { np24 } else { np8 };
        let s = outcomes(
            &format!("{name}(mlen=UINT64_MAX)"),
            || unsafe {
                c(p, p, u64::MAX, np, u64::MAX, kp);
            },
            || unsafe {
                r(p, p, u64::MAX, np, u64::MAX, kp);
            },
        );
        assert_eq!(s, SIGSEGV_STATUS, "{name}: no length check may fire");
    }
}

#[test]
fn chacha20_family_zero_length_with_null_pointers() {
    // Error rows 5.23 / 5.28 (benign sub-case): every implementation
    // short-circuits on a zero length before dereferencing `m` or `c`.
    let k = [1u8; 32];
    let n = [2u8; 24];
    for (name, nb) in [
        ("crypto_stream_chacha20", 8usize),
        ("crypto_stream_chacha20_ietf", 12),
        ("crypto_stream_chacha20_ietf_ext", 12),
        ("crypto_stream_xchacha20", 24),
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
        ("crypto_stream_chacha20_xor", 8usize),
        ("crypto_stream_chacha20_ietf_xor", 12),
        ("crypto_stream_xchacha20_xor", 24),
    ] {
        let (c, r) = both::<Xor>(name);
        let (rc, rr) = unsafe {
            (
                c(core::ptr::null_mut(), core::ptr::null(), 0, n[..nb].as_ptr(), k.as_ptr()),
                r(core::ptr::null_mut(), core::ptr::null(), 0, n[..nb].as_ptr(), k.as_ptr()),
            )
        };
        eqi(&format!("{name}(NULL,0)"), rc, rr);
        assert_eq!(rc, 0);
    }
    let (c, r) = both::<XorIc32>("crypto_stream_chacha20_ietf_ext_xor_ic");
    let (rc, rr) = unsafe {
        (
            c(
                core::ptr::null_mut(),
                core::ptr::null(),
                0,
                n[..12].as_ptr(),
                0xFFFF_FFFF,
                k.as_ptr(),
            ),
            r(
                core::ptr::null_mut(),
                core::ptr::null(),
                0,
                n[..12].as_ptr(),
                0xFFFF_FFFF,
                k.as_ptr(),
            ),
        )
    };
    eqi("ietf_ext_xor_ic(NULL,0)", rc, rr);
    assert_eq!(rc, 0);
}

#[test]
fn chacha20_partial_block_path_never_overwrites() {
    // Error row 5.29: the `bytes < 64` tail is staged through a zero-filled
    // `tmp[64]` and only `bytes` bytes are copied back; `bytes == 64` exactly
    // takes the direct path.  Poisoned tails pin both behaviours.
    let (cx, rx) = both::<Xor>("crypto_stream_chacha20_xor");
    let mut rng = Rng::new(0x5_0029);
    for len in 0..=200usize {
        let k = rng.bytes(32);
        let n = rng.bytes(8);
        let m = rng.bytes(len);
        let mut a = padded(len);
        let mut b = padded(len);
        unsafe {
            cx(a.as_mut_ptr(), m.as_ptr(), len as u64, n.as_ptr(), k.as_ptr());
            rx(b.as_mut_ptr(), m.as_ptr(), len as u64, n.as_ptr(), k.as_ptr());
        }
        eqb(&format!("chacha20_xor tail(len={len})"), &a, &b);
        check_pad(&format!("chacha20_xor tail(len={len})"), &a, len);
        check_pad(&format!("chacha20_xor tail(len={len})"), &b, len);
    }
}
