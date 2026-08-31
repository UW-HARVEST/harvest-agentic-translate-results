//! Area 5 — `crypto_stream`, gap closure.
//!
//! `a5_salsa.rs` covers `configs_5.md` rows 5.11–5.46 and `a5_chacha20.rs`
//! covers 5.47–5.83.  This file closes the remaining rows:
//!
//!   * 5.1–5.10  — the generic `crypto_stream` / `crypto_stream_xor` /
//!                 `crypto_stream_keygen` wrapper and its accessors
//!                 (`crypto_stream.c`, a thin alias over xsalsa20).
//!   * 5.84–5.86 — zero length / return-code exhaustiveness across *every*
//!                 primitive **and** the generic wrapper, with poisoned buffers.
//!   * 5.88      — prefix consistency for the `_xor` and `_xor_ic` forms as well
//!                 (the two other files only sweep the keystream forms).
//!   * the `_xor_ic(ic = k) == tail of a long _xor starting at block k`
//!     identity for the two primitives whose `_xor_ic` takes a 32-bit counter
//!     (`chacha20_ietf`, `chacha20_ietf_ext`); the 64-bit-counter primitives are
//!     already pinned that way in the other two files.
//!
//! Error rows touched: 5.18 (the generic wrapper inherits the salsa20 family's
//! complete absence of length validation), 5.23, 5.24, 5.26, 5.28.
//!
//! Sources of truth:
//!   c_src/libsodium/crypto_stream/crypto_stream.c
//!   c_src/libsodium/crypto_stream/xsalsa20/stream_xsalsa20.c
//!   c_src/libsodium/crypto_stream/chacha20/stream_chacha20.c

mod common;
use common::*;
use std::ffi::{c_char, c_int, c_void};

// ------------------------------------------------------------------ signatures

type Stream = unsafe extern "C" fn(*mut u8, u64, *const u8, *const u8) -> c_int;
type Xor = unsafe extern "C" fn(*mut u8, *const u8, u64, *const u8, *const u8) -> c_int;
type XorIc64 = unsafe extern "C" fn(*mut u8, *const u8, u64, *const u8, u64, *const u8) -> c_int;
type XorIc32 = unsafe extern "C" fn(*mut u8, *const u8, u64, *const u8, u32, *const u8) -> c_int;
type SizeFn = unsafe extern "C" fn() -> usize;
type StrFn = unsafe extern "C" fn() -> *const c_char;
type Keygen = unsafe extern "C" fn(*mut u8);

// ------------------------------------------------------------------- constants

/// The length sweep **L** from `configs_5.md`.
const L: [usize; 16] = [
    0, 1, 63, 64, 65, 127, 128, 129, 191, 192, 193, 255, 256, 257, 511, 512,
];
const BIG: [usize; 5] = [1024, 1025, 4096, 8191, 8192];

const SODIUM_SIZE_MAX: u64 = u64::MAX;

/// NaCl / libsodium `secretbox` test-vector key and 24-byte nonce.
const TV_KEY: [u8; 32] = [
    0x1b, 0x27, 0x55, 0x64, 0x73, 0xe9, 0x85, 0xd4, 0x62, 0xcd, 0x51, 0x19, 0x7a, 0x9a, 0x46, 0xc7,
    0x60, 0x09, 0x54, 0x9e, 0xac, 0x64, 0x74, 0xf2, 0x06, 0xc4, 0xee, 0x08, 0x44, 0xf6, 0x83, 0x89,
];
const TV_NONCE24: [u8; 24] = [
    0x69, 0x69, 0x6e, 0xe9, 0x55, 0xb6, 0x2b, 0x73, 0xcd, 0x62, 0xbd, 0xa8, 0x75, 0xfc, 0x73, 0xd6,
    0x82, 0x19, 0xe0, 0x03, 0x6b, 0x7a, 0x0b, 0x37,
];

/// `(key, nonce)` shapes: all-zero, all-`0xff`, ascending pattern, test vector,
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

/// Every keystream entry point in area 5, with its nonce size.
const KEYSTREAM_FORMS: [(&str, usize); 9] = [
    ("crypto_stream", 24),
    ("crypto_stream_salsa20", 8),
    ("crypto_stream_salsa2012", 8),
    ("crypto_stream_salsa208", 8),
    ("crypto_stream_xsalsa20", 24),
    ("crypto_stream_chacha20", 8),
    ("crypto_stream_chacha20_ietf", 12),
    ("crypto_stream_chacha20_ietf_ext", 12),
    ("crypto_stream_xchacha20", 24),
];

/// Every `_xor` entry point in area 5, with its nonce size.  (`_ietf_ext_xor` is
/// `static` in the C source and therefore not an entry point.)
const XOR_FORMS: [(&str, usize); 8] = [
    ("crypto_stream_xor", 24),
    ("crypto_stream_salsa20_xor", 8),
    ("crypto_stream_salsa2012_xor", 8),
    ("crypto_stream_salsa208_xor", 8),
    ("crypto_stream_xsalsa20_xor", 24),
    ("crypto_stream_chacha20_xor", 8),
    ("crypto_stream_chacha20_ietf_xor", 12),
    ("crypto_stream_xchacha20_xor", 24),
];

/// The `_xor_ic` entry points that take a 64-bit `ic`.
const XOR_IC64_FORMS: [(&str, usize); 4] = [
    ("crypto_stream_salsa20_xor_ic", 8),
    ("crypto_stream_xsalsa20_xor_ic", 24),
    ("crypto_stream_chacha20_xor_ic", 8),
    ("crypto_stream_xchacha20_xor_ic", 24),
];

/// The `_xor_ic` entry points that take a 32-bit `ic`.
const XOR_IC32_FORMS: [(&str, usize); 2] = [
    ("crypto_stream_chacha20_ietf_xor_ic", 12),
    ("crypto_stream_chacha20_ietf_ext_xor_ic", 12),
];

// =========================================== 5.1 / error rows 5.24, 5.25

#[test]
fn generic_accessors_and_primitive() {
    // Row 5.1: `crypto_stream_KEYBYTES` = 32, `_NONCEBYTES` = 24,
    // `_MESSAGEBYTES_MAX` = SODIUM_SIZE_MAX, `_PRIMITIVE` = "xsalsa20"
    // (`crypto_stream.c:6,12,18,24`).  These are the xsalsa20 aliases.
    for (name, expect) in [
        ("crypto_stream_keybytes", 32usize),
        ("crypto_stream_noncebytes", 24),
        ("crypto_stream_messagebytes_max", SODIUM_SIZE_MAX as usize),
    ] {
        let (c, r) = both::<SizeFn>(name);
        let (vc, vr) = unsafe { (c(), r()) };
        assert_eq!(vc, vr, "{name}: C {vc} vs Rust {vr}");
        assert_eq!(vc, expect, "{name}: expected {expect}, C returned {vc}");
    }
    // The generic aliases must agree with the xsalsa20 accessors they alias.
    for (g, s) in [
        ("crypto_stream_keybytes", "crypto_stream_xsalsa20_keybytes"),
        ("crypto_stream_noncebytes", "crypto_stream_xsalsa20_noncebytes"),
        (
            "crypto_stream_messagebytes_max",
            "crypto_stream_xsalsa20_messagebytes_max",
        ),
    ] {
        let (cg, rg) = both::<SizeFn>(g);
        let (cs, rs) = both::<SizeFn>(s);
        unsafe {
            assert_eq!(cg(), cs(), "C: {g} != {s}");
            assert_eq!(rg(), rs(), "Rust: {g} != {s}");
        }
    }

    // `crypto_stream_primitive` returns a pointer to static storage, never NULL.
    let (cp, rp) = both::<StrFn>("crypto_stream_primitive");
    let (pc, pr) = unsafe { (cp(), rp()) };
    assert!(!pc.is_null(), "C crypto_stream_primitive returned NULL");
    assert!(!pr.is_null(), "Rust crypto_stream_primitive returned NULL");
    let sc = unsafe { std::ffi::CStr::from_ptr(pc) };
    let sr = unsafe { std::ffi::CStr::from_ptr(pr) };
    eqb("crypto_stream_primitive", sc.to_bytes(), sr.to_bytes());
    assert_eq!(sc.to_bytes(), b"xsalsa20");
    // Static storage: repeated calls return the very same pointer.
    for _ in 0..4 {
        let (pc2, pr2) = unsafe { (cp(), rp()) };
        assert_eq!(pc2, pc, "C crypto_stream_primitive is not static storage");
        assert_eq!(pr2, pr, "Rust crypto_stream_primitive is not static storage");
    }
}

// ================================================ 5.2–5.8 the generic wrapper

/// Rows 5.2/5.3/5.4 (keystream, all key/nonce shapes), 5.5 (out-of-place),
/// 5.6 (in-place), 5.7 (round-trip), 5.8 (`_xor(0) == keystream`), 5.86 (rc = 0),
/// 5.87 (nothing written past `len`).
fn drive_generic(seed: u64, lens: &[usize], extra: usize) {
    let (cs, rs) = both::<Stream>("crypto_stream");
    let (cx, rx) = both::<Xor>("crypto_stream_xor");
    let mut rng = Rng::new(seed);

    for &len in lens {
        for (si, (k, n)) in shapes(24, &mut rng, extra).into_iter().enumerate() {
            let tag = format!("crypto_stream(len={len},shape={si})");

            // keystream form
            let mut a = padded(len);
            let mut b = padded(len);
            let (rc, rr) = unsafe {
                (
                    cs(a.as_mut_ptr(), len as u64, n.as_ptr(), k.as_ptr()),
                    rs(b.as_mut_ptr(), len as u64, n.as_ptr(), k.as_ptr()),
                )
            };
            eqi(&tag, rc, rr);
            assert_eq!(rc, 0, "{tag}: crypto_stream must return 0 (row 5.86)");
            eqb(&tag, &a, &b);
            check_pad(&tag, &a, len);
            check_pad(&tag, &b, len);
            let ks = a[..len].to_vec();

            // row 5.8: _xor over an all-zero plaintext == the keystream form
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
            assert_eq!(&a2[..len], &ks[..], "{tag}: xor(zero) != keystream (row 5.8)");

            // row 5.5: out-of-place with a pseudorandom message
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
            assert_eq!(rc, 0);
            eqb(&format!("{tag} xor"), &a3, &b3);
            check_pad(&format!("{tag} xor"), &a3, len);
            for i in 0..len {
                assert_eq!(a3[i], m[i] ^ ks[i], "{tag}: xor != m ^ keystream at {i}");
            }

            // row 5.7: XOR twice with the same (n, k) restores m
            let mut a4 = padded(len);
            let mut b4 = padded(len);
            unsafe {
                cx(a4.as_mut_ptr(), a3.as_ptr(), len as u64, n.as_ptr(), k.as_ptr());
                rx(b4.as_mut_ptr(), b3.as_ptr(), len as u64, n.as_ptr(), k.as_ptr());
            }
            eqb(&format!("{tag} xor^2"), &a4, &b4);
            check_pad(&format!("{tag} xor^2"), &a4, len);
            assert_eq!(&a4[..len], &m[..], "{tag}: round-trip failed (row 5.7)");

            // row 5.6: in-place (c == m)
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
                "{tag}: in-place differs from out-of-place (row 5.6)"
            );
        }
    }
}

#[test]
fn generic_stream_and_xor_sweep() {
    // 5.2, 5.3, 5.4, 5.5, 5.6, 5.7, 5.8
    drive_generic(0x5_0002, &L, 6);
    drive_generic(0x5_0003, &BIG, 2);
}

#[test]
fn generic_stream_full_length_sweep() {
    // 5.87 at byte granularity: every length in 0..=600 crosses the bulk/partial
    // split of `salsa20_ref.c` in both directions.
    let lens: Vec<usize> = (0..=600).collect();
    drive_generic(0x5_0004, &lens, 0);
}

#[test]
fn generic_equals_xsalsa20() {
    // 5.9: the generic wrapper is a pure forward to the xsalsa20 entry points
    // (`crypto_stream.c:33,42`), so the outputs must be byte-identical.
    let (cg, rg) = both::<Stream>("crypto_stream");
    let (cs, rs) = both::<Stream>("crypto_stream_xsalsa20");
    let (cgx, rgx) = both::<Xor>("crypto_stream_xor");
    let (csx, rsx) = both::<Xor>("crypto_stream_xsalsa20_xor");
    let mut rng = Rng::new(0x5_0009);
    let mut lens: Vec<usize> = L.to_vec();
    lens.extend_from_slice(&BIG);
    for &len in lens.iter() {
        for (si, (k, n)) in shapes(24, &mut rng, 6).into_iter().enumerate() {
            let tag = format!("generic vs xsalsa20(len={len},shape={si})");
            let m = rng.bytes(len);

            let mut g_c = padded(len);
            let mut g_r = padded(len);
            let mut s_c = padded(len);
            let mut s_r = padded(len);
            unsafe {
                cg(g_c.as_mut_ptr(), len as u64, n.as_ptr(), k.as_ptr());
                rg(g_r.as_mut_ptr(), len as u64, n.as_ptr(), k.as_ptr());
                cs(s_c.as_mut_ptr(), len as u64, n.as_ptr(), k.as_ptr());
                rs(s_r.as_mut_ptr(), len as u64, n.as_ptr(), k.as_ptr());
            }
            eqb(&format!("{tag} generic"), &g_c, &g_r);
            eqb(&format!("{tag} xsalsa20"), &s_c, &s_r);
            assert_eq!(g_c, s_c, "{tag}: crypto_stream != crypto_stream_xsalsa20");

            let mut gx_c = padded(len);
            let mut gx_r = padded(len);
            let mut sx_c = padded(len);
            let mut sx_r = padded(len);
            unsafe {
                cgx(gx_c.as_mut_ptr(), m.as_ptr(), len as u64, n.as_ptr(), k.as_ptr());
                rgx(gx_r.as_mut_ptr(), m.as_ptr(), len as u64, n.as_ptr(), k.as_ptr());
                csx(sx_c.as_mut_ptr(), m.as_ptr(), len as u64, n.as_ptr(), k.as_ptr());
                rsx(sx_r.as_mut_ptr(), m.as_ptr(), len as u64, n.as_ptr(), k.as_ptr());
            }
            eqb(&format!("{tag} generic xor"), &gx_c, &gx_r);
            eqb(&format!("{tag} xsalsa20 xor"), &sx_c, &sx_r);
            assert_eq!(
                gx_c, sx_c,
                "{tag}: crypto_stream_xor != crypto_stream_xsalsa20_xor"
            );
        }
    }
}

#[test]
fn generic_keygen() {
    // 5.10 / error row 5.26: `crypto_stream_keygen` is `randombytes_buf(k, 32)`.
    let (c, r) = both::<Keygen>("crypto_stream_keygen");
    let (cx, rx) = both::<Keygen>("crypto_stream_xsalsa20_keygen");

    let mut a = padded(32);
    let mut b = padded(32);
    rng_reset();
    unsafe {
        c(a.as_mut_ptr());
        r(b.as_mut_ptr());
    }
    eqb("crypto_stream_keygen", &a, &b);
    check_pad("crypto_stream_keygen", &a, 32);
    check_pad("crypto_stream_keygen", &b, 32);
    assert!(a[..32].iter().any(|&x| x != 0), "keygen produced an all-zero key");

    // It must draw exactly 32 bytes from the same RNG position as the xsalsa20
    // keygen it aliases.
    let mut a2 = padded(32);
    let mut b2 = padded(32);
    rng_reset();
    unsafe {
        cx(a2.as_mut_ptr());
        rx(b2.as_mut_ptr());
    }
    eqb("crypto_stream_xsalsa20_keygen", &a2, &b2);
    assert_eq!(
        &a[..32],
        &a2[..32],
        "crypto_stream_keygen must alias crypto_stream_xsalsa20_keygen"
    );

    // Non-constant across RNG stream positions.
    let mut a3 = padded(32);
    let mut b3 = padded(32);
    rng_reseed(0x0bad_c0de_dead_beef);
    unsafe {
        c(a3.as_mut_ptr());
        r(b3.as_mut_ptr());
    }
    eqb("crypto_stream_keygen #2", &a3, &b3);
    check_pad("crypto_stream_keygen #2", &a3, 32);
    assert_ne!(&a[..32], &a3[..32], "crypto_stream_keygen output is constant");
    rng_reset();
}

// ================================================= 5.84 / 5.85 / 5.86 exhaustive

#[test]
fn every_entry_point_accepts_zero_length_without_writing() {
    // Rows 5.84 and 5.85: `if (!clen) return 0;` / `if (!mlen) return 0;` in
    // every implementation.  A fully poisoned buffer proves nothing is written,
    // and row 5.86 requires the return code to be 0 everywhere.
    let mut rng = Rng::new(0x5_0084);
    let k = rng.bytes(32);
    let n = rng.bytes(24);

    for (name, nb) in KEYSTREAM_FORMS {
        let (c, r) = both::<Stream>(name);
        let mut a = padded(0);
        let mut b = padded(0);
        let poison = a.clone();
        let (rc, rr) = unsafe {
            (
                c(a.as_mut_ptr(), 0, n[..nb].as_ptr(), k.as_ptr()),
                r(b.as_mut_ptr(), 0, n[..nb].as_ptr(), k.as_ptr()),
            )
        };
        eqi(&format!("{name}(clen=0)"), rc, rr);
        assert_eq!(rc, 0, "{name}(clen=0) must return 0 (row 5.86)");
        eqb(&format!("{name}(clen=0)"), &a, &b);
        assert_eq!(a, poison, "{name}(clen=0) wrote to the output buffer");
        assert_eq!(b, poison, "{name}(clen=0) wrote to the output buffer (Rust)");
    }

    for (name, nb) in XOR_FORMS {
        let (c, r) = both::<Xor>(name);
        let m = padded(0);
        let mut a = padded(0);
        let mut b = padded(0);
        let poison = a.clone();
        let (rc, rr) = unsafe {
            (
                c(a.as_mut_ptr(), m.as_ptr(), 0, n[..nb].as_ptr(), k.as_ptr()),
                r(b.as_mut_ptr(), m.as_ptr(), 0, n[..nb].as_ptr(), k.as_ptr()),
            )
        };
        eqi(&format!("{name}(mlen=0)"), rc, rr);
        assert_eq!(rc, 0, "{name}(mlen=0) must return 0 (row 5.86)");
        eqb(&format!("{name}(mlen=0)"), &a, &b);
        assert_eq!(a, poison, "{name}(mlen=0) wrote to the output buffer");
        assert_eq!(b, poison, "{name}(mlen=0) wrote to the output buffer (Rust)");
    }

    for (name, nb) in XOR_IC64_FORMS {
        let (c, r) = both::<XorIc64>(name);
        for ic in [0u64, 1, 7, 0xFFFF_FFFF, 0x1_0000_0000, u64::MAX] {
            let m = padded(0);
            let mut a = padded(0);
            let mut b = padded(0);
            let poison = a.clone();
            let (rc, rr) = unsafe {
                (
                    c(a.as_mut_ptr(), m.as_ptr(), 0, n[..nb].as_ptr(), ic, k.as_ptr()),
                    r(b.as_mut_ptr(), m.as_ptr(), 0, n[..nb].as_ptr(), ic, k.as_ptr()),
                )
            };
            eqi(&format!("{name}(mlen=0,ic={ic:#x})"), rc, rr);
            assert_eq!(rc, 0, "{name}(mlen=0,ic={ic:#x}) must return 0");
            eqb(&format!("{name}(mlen=0,ic={ic:#x})"), &a, &b);
            assert_eq!(a, poison, "{name}(mlen=0,ic={ic:#x}) wrote to the buffer");
            assert_eq!(b, poison, "{name}(mlen=0,ic={ic:#x}) wrote (Rust)");
        }
    }

    for (name, nb) in XOR_IC32_FORMS {
        let (c, r) = both::<XorIc32>(name);
        // Row 5.68: at `mlen == 0` the `_ietf_xor_ic` guard limit is exactly 2^32,
        // so even `ic == 0xFFFFFFFF` is accepted.
        for ic in [0u32, 1, 7, 0xFFFF_FFFE, 0xFFFF_FFFF] {
            let m = padded(0);
            let mut a = padded(0);
            let mut b = padded(0);
            let poison = a.clone();
            let (rc, rr) = unsafe {
                (
                    c(a.as_mut_ptr(), m.as_ptr(), 0, n[..nb].as_ptr(), ic, k.as_ptr()),
                    r(b.as_mut_ptr(), m.as_ptr(), 0, n[..nb].as_ptr(), ic, k.as_ptr()),
                )
            };
            eqi(&format!("{name}(mlen=0,ic={ic:#x})"), rc, rr);
            assert_eq!(rc, 0, "{name}(mlen=0,ic={ic:#x}) must return 0");
            eqb(&format!("{name}(mlen=0,ic={ic:#x})"), &a, &b);
            assert_eq!(a, poison, "{name}(mlen=0,ic={ic:#x}) wrote to the buffer");
            assert_eq!(b, poison, "{name}(mlen=0,ic={ic:#x}) wrote (Rust)");
        }
    }
}

#[test]
fn zero_length_with_null_input_pointer_is_benign_everywhere() {
    // Error row 5.23, benign sub-case, for the entry points the other two files
    // do not cover: the generic wrapper and every `_xor_ic` form.
    let k = [0x3du8; 32];
    let n = [0x71u8; 24];

    let (c, r) = both::<Stream>("crypto_stream");
    let (rc, rr) = unsafe {
        (
            c(core::ptr::null_mut(), 0, n.as_ptr(), k.as_ptr()),
            r(core::ptr::null_mut(), 0, n.as_ptr(), k.as_ptr()),
        )
    };
    eqi("crypto_stream(NULL,0)", rc, rr);
    assert_eq!(rc, 0);

    let (c, r) = both::<Xor>("crypto_stream_xor");
    let (rc, rr) = unsafe {
        (
            c(core::ptr::null_mut(), core::ptr::null(), 0, n.as_ptr(), k.as_ptr()),
            r(core::ptr::null_mut(), core::ptr::null(), 0, n.as_ptr(), k.as_ptr()),
        )
    };
    eqi("crypto_stream_xor(NULL,0)", rc, rr);
    assert_eq!(rc, 0);

    for (name, nb) in XOR_IC64_FORMS {
        let (c, r) = both::<XorIc64>(name);
        for ic in [0u64, u64::MAX] {
            let (rc, rr) = unsafe {
                (
                    c(
                        core::ptr::null_mut(),
                        core::ptr::null(),
                        0,
                        n[..nb].as_ptr(),
                        ic,
                        k.as_ptr(),
                    ),
                    r(
                        core::ptr::null_mut(),
                        core::ptr::null(),
                        0,
                        n[..nb].as_ptr(),
                        ic,
                        k.as_ptr(),
                    ),
                )
            };
            eqi(&format!("{name}(NULL,0,ic={ic:#x})"), rc, rr);
            assert_eq!(rc, 0);
        }
    }
    for (name, nb) in XOR_IC32_FORMS {
        let (c, r) = both::<XorIc32>(name);
        for ic in [0u32, 0xFFFF_FFFF] {
            let (rc, rr) = unsafe {
                (
                    c(
                        core::ptr::null_mut(),
                        core::ptr::null(),
                        0,
                        n[..nb].as_ptr(),
                        ic,
                        k.as_ptr(),
                    ),
                    r(
                        core::ptr::null_mut(),
                        core::ptr::null(),
                        0,
                        n[..nb].as_ptr(),
                        ic,
                        k.as_ptr(),
                    ),
                )
            };
            eqi(&format!("{name}(NULL,0,ic={ic:#x})"), rc, rr);
            assert_eq!(rc, 0);
        }
    }
}

// ============================================ 5.88 for the _xor / _xor_ic forms

#[test]
fn xor_forms_prefix_consistency() {
    // Row 5.88 for the `_xor` forms: with the same `(m, n, k)` the output for a
    // shorter length must be a prefix of the output for a longer one.  The other
    // two files only sweep the keystream forms this way.
    let mut rng = Rng::new(0x5_0088);
    let longest = *L.iter().max().unwrap();
    for (name, nb) in XOR_FORMS {
        let (c, r) = both::<Xor>(name);
        for (si, (k, n)) in shapes(nb, &mut rng, 4).into_iter().enumerate() {
            let m = rng.bytes(longest);
            let mut prev: Vec<u8> = Vec::new();
            for &len in L.iter() {
                let mut a = padded(len);
                let mut b = padded(len);
                unsafe {
                    c(a.as_mut_ptr(), m.as_ptr(), len as u64, n.as_ptr(), k.as_ptr());
                    r(b.as_mut_ptr(), m.as_ptr(), len as u64, n.as_ptr(), k.as_ptr());
                }
                eqb(&format!("{name} prefix(len={len},shape={si})"), &a, &b);
                check_pad(&format!("{name} prefix(len={len},shape={si})"), &a, len);
                assert_eq!(
                    &a[..prev.len()],
                    &prev[..],
                    "{name}: length {len} output is not an extension of the shorter one"
                );
                prev = a[..len].to_vec();
            }
        }
    }
}

#[test]
fn xor_ic_forms_prefix_consistency() {
    // Row 5.88 for the `_xor_ic` forms, at several `ic` values.
    let mut rng = Rng::new(0x5_0089);
    let longest = *L.iter().max().unwrap();
    for (name, nb) in XOR_IC64_FORMS {
        let (c, r) = both::<XorIc64>(name);
        for ic in [0u64, 1, 3, 0xFFFF_FFFF] {
            for (si, (k, n)) in shapes(nb, &mut rng, 3).into_iter().enumerate() {
                let m = rng.bytes(longest);
                let mut prev: Vec<u8> = Vec::new();
                for &len in L.iter() {
                    let mut a = padded(len);
                    let mut b = padded(len);
                    unsafe {
                        c(a.as_mut_ptr(), m.as_ptr(), len as u64, n.as_ptr(), ic, k.as_ptr());
                        r(b.as_mut_ptr(), m.as_ptr(), len as u64, n.as_ptr(), ic, k.as_ptr());
                    }
                    let tag = format!("{name} prefix(len={len},ic={ic:#x},shape={si})");
                    eqb(&tag, &a, &b);
                    check_pad(&tag, &a, len);
                    assert_eq!(&a[..prev.len()], &prev[..], "{tag}: not an extension");
                    prev = a[..len].to_vec();
                }
            }
        }
    }
    for (name, nb) in XOR_IC32_FORMS {
        let (c, r) = both::<XorIc32>(name);
        // Keep `ic` small enough that the `_ietf_xor_ic` guard (error row 5.8)
        // accepts every length in L: the limit at mlen = 512 is 2^32 - 8.
        for ic in [0u32, 1, 3, 0xFFFF_FFF0] {
            for (si, (k, n)) in shapes(nb, &mut rng, 3).into_iter().enumerate() {
                let m = rng.bytes(longest);
                let mut prev: Vec<u8> = Vec::new();
                for &len in L.iter() {
                    let mut a = padded(len);
                    let mut b = padded(len);
                    unsafe {
                        c(a.as_mut_ptr(), m.as_ptr(), len as u64, n.as_ptr(), ic, k.as_ptr());
                        r(b.as_mut_ptr(), m.as_ptr(), len as u64, n.as_ptr(), ic, k.as_ptr());
                    }
                    let tag = format!("{name} prefix(len={len},ic={ic:#x},shape={si})");
                    eqb(&tag, &a, &b);
                    check_pad(&tag, &a, len);
                    assert_eq!(&a[..prev.len()], &prev[..], "{tag}: not an extension");
                    prev = a[..len].to_vec();
                }
            }
        }
    }
}

// ================== _xor_ic(ic = k) == tail of a long _xor starting at block k

#[test]
fn ietf_xor_ic_equals_the_tail_of_a_long_xor() {
    // The 32-bit-counter primitives: `crypto_stream_chacha20_ietf_xor_ic(ic)` must
    // equal bytes `[64*ic ..]` of a `crypto_stream_chacha20_ietf_xor` run over a
    // `64*ic`-byte-prefixed message.  (`a5_chacha20.rs` only cross-checks these
    // against `_xor_ic(0)`; this pins them against the independent `_xor` entry
    // point, and `_ietf_ext_xor_ic` against it too — the two agree for every ic
    // the `_ietf` guard permits.)
    let (cxi, rxi) = both::<XorIc32>("crypto_stream_chacha20_ietf_xor_ic");
    let (cei, rei) = both::<XorIc32>("crypto_stream_chacha20_ietf_ext_xor_ic");
    let (cx, rx) = both::<Xor>("crypto_stream_chacha20_ietf_xor");
    let mut rng = Rng::new(0x5_0090);

    for ic in [0u32, 1, 2, 3, 7, 8] {
        for &len in L.iter() {
            if len == 0 {
                continue;
            }
            for (si, (k, n)) in shapes(12, &mut rng, 4).into_iter().enumerate() {
                let tag = format!("ietf tail(len={len},ic={ic},shape={si})");
                let m = rng.bytes(len);

                let mut a = padded(len);
                let mut b = padded(len);
                let mut e = padded(len);
                let mut f = padded(len);
                unsafe {
                    cxi(a.as_mut_ptr(), m.as_ptr(), len as u64, n.as_ptr(), ic, k.as_ptr());
                    rxi(b.as_mut_ptr(), m.as_ptr(), len as u64, n.as_ptr(), ic, k.as_ptr());
                    cei(e.as_mut_ptr(), m.as_ptr(), len as u64, n.as_ptr(), ic, k.as_ptr());
                    rei(f.as_mut_ptr(), m.as_ptr(), len as u64, n.as_ptr(), ic, k.as_ptr());
                }
                eqb(&tag, &a, &b);
                eqb(&format!("{tag} ext"), &e, &f);
                check_pad(&tag, &a, len);
                assert_eq!(&e[..len], &a[..len], "{tag}: _ietf_ext_xor_ic disagrees");

                let off = 64 * ic as usize;
                let total = off + len;
                let mut full = rng.bytes(total);
                full[off..].copy_from_slice(&m);
                let mut fc = padded(total);
                let mut fr = padded(total);
                unsafe {
                    cx(fc.as_mut_ptr(), full.as_ptr(), total as u64, n.as_ptr(), k.as_ptr());
                    rx(fr.as_mut_ptr(), full.as_ptr(), total as u64, n.as_ptr(), k.as_ptr());
                }
                eqb(&format!("{tag} long"), &fc, &fr);
                check_pad(&format!("{tag} long"), &fc, total);
                assert_eq!(
                    &fc[off..total],
                    &a[..len],
                    "{tag}: _xor_ic(ic) != tail of _xor at block ic"
                );
            }
        }
    }
}

// ==================== error row 5.18 for the generic wrapper (guarded pointer)

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
fn generic_wrapper_has_no_length_validation() {
    // Error row 5.18: `crypto_stream` / `crypto_stream_xor` forward straight to
    // xsalsa20 with no length check of their own, and xsalsa20 has none either.
    // A `> 2^38` length therefore does not abort — it runs off the end of the
    // mapping.  `eq_abort` pins C and Rust to the same process outcome, which is
    // the only observable for a `> MESSAGEBYTES_MAX` length that cannot be
    // allocated (`checked-via-guard-only`).
    let p = guarded_region();
    let k = [0x5au8; 32];
    let n = [0x3cu8; 24];
    let huge: u64 = 274_877_906_945; // 2^38 + 1

    let (c, r) = both::<Stream>("crypto_stream");
    eq_abort(
        "crypto_stream(clen=2^38+1)",
        || unsafe {
            c(p, huge, n.as_ptr(), k.as_ptr());
        },
        || unsafe {
            r(p, huge, n.as_ptr(), k.as_ptr());
        },
    );

    let (c, r) = both::<Xor>("crypto_stream_xor");
    eq_abort(
        "crypto_stream_xor(mlen=2^38+1)",
        || unsafe {
            c(p, p, huge, n.as_ptr(), k.as_ptr());
        },
        || unsafe {
            r(p, p, huge, n.as_ptr(), k.as_ptr());
        },
    );

    // The same lengths must be rejected by the *ietf* entry points, so the two
    // outcomes above really are "no check fired" and not an accidental match.
    let (ci, ri) = both::<Stream>("crypto_stream_chacha20_ietf");
    let n12 = [0x3cu8; 12];
    let sc = status_str(in_child(|| unsafe {
        ci(p, huge, n12.as_ptr(), k.as_ptr());
    }));
    let sr = status_str(in_child(|| unsafe {
        ri(p, huge, n12.as_ptr(), k.as_ptr());
    }));
    assert_eq!(sc, sr, "crypto_stream_chacha20_ietf outcome mismatch");
    assert_eq!(sc, "sig:6", "row 5.7: the ietf limit must abort at 2^38+1");
    let sg = status_str(in_child(|| unsafe {
        let (c, _) = both::<Stream>("crypto_stream");
        c(p, huge, n.as_ptr(), k.as_ptr());
    }));
    assert_eq!(
        sg, "sig:11",
        "row 5.18: crypto_stream must NOT abort — it has no length check"
    );
}
