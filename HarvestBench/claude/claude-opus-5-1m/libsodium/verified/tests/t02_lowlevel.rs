//! Phase B — lowest-level entry points: `crypto_verify_*`, the `crypto_core_*`
//! block/HSalsa/HChaCha cores, every `crypto_stream_*` variant (keystream,
//! `_xor`, `_xor_ic`) and `crypto_shorthash_*`.
//!
//! Driven through `dlsym` on both `.so`s. Randomised inputs, fixed seed.

mod common;
use common::*;

type Verify = unsafe extern "C" fn(*const u8, *const u8) -> i32;
type Core = unsafe extern "C" fn(*mut u8, *const u8, *const u8, *const u8) -> i32;
type Stream = unsafe extern "C" fn(*mut u8, u64, *const u8, *const u8) -> i32;
type StreamXor = unsafe extern "C" fn(*mut u8, *const u8, u64, *const u8, *const u8) -> i32;
type StreamXorIc64 =
    unsafe extern "C" fn(*mut u8, *const u8, u64, *const u8, u64, *const u8) -> i32;
type StreamXorIc32 =
    unsafe extern "C" fn(*mut u8, *const u8, u64, *const u8, u32, *const u8) -> i32;
type Shorthash = unsafe extern "C" fn(*mut u8, *const u8, u64, *const u8) -> i32;
type Keygen = unsafe extern "C" fn(*mut u8);

/// Message lengths that straddle every block boundary used by the ref
/// implementations (64-byte ChaCha/Salsa blocks, 16-byte AEGIS/Poly steps).
const LENS: &[usize] = &[
    0, 1, 2, 15, 16, 17, 31, 32, 33, 63, 64, 65, 66, 95, 96, 127, 128, 129, 191, 192, 193, 255,
    256, 257, 383, 384, 511, 512, 513, 1000, 1023, 1024, 1025, 4096,
];

// ---------------------------------------------------------------------------
// CONFIGS rows: crypto_verify_16 / _32 / _64
// ---------------------------------------------------------------------------

#[test]
fn verify_16_32_64() {
    setup();
    let mut rng = Rng::new(0x1111);
    for (name, n) in [
        ("crypto_verify_16", 16usize),
        ("crypto_verify_32", 32),
        ("crypto_verify_64", 64),
    ] {
        let (c, r) = pair::<Verify>(name);
        // equal / differ at each single byte position / random pairs
        let mut cases: Vec<(Vec<u8>, Vec<u8>)> = Vec::new();
        for _ in 0..8 {
            let x = rng.bytes(n);
            cases.push((x.clone(), x.clone())); // equal
            for i in 0..n {
                let mut y = x.clone();
                y[i] ^= 1 << (rng.below(8));
                cases.push((x.clone(), y)); // differ at byte i
            }
        }
        for _ in 0..64 {
            cases.push((rng.bytes(n), rng.bytes(n))); // fully random
        }
        cases.push((vec![0u8; n], vec![0u8; n]));
        cases.push((vec![0xffu8; n], vec![0xffu8; n]));
        cases.push((vec![0u8; n], vec![0xffu8; n]));
        for (x, y) in cases {
            let (a, b) = unsafe { (c(x.as_ptr(), y.as_ptr()), r(x.as_ptr(), y.as_ptr())) };
            eq_i32(&format!("{name}({}, {})", hex(&x), hex(&y)), a, b);
        }
    }
}

// ---------------------------------------------------------------------------
// CONFIGS rows: crypto_core_salsa20 / _salsa2012 / _salsa208 / _hsalsa20 /
//               _hchacha20 — with `c` = NULL and `c` = explicit constant
// ---------------------------------------------------------------------------

#[test]
fn core_salsa_hsalsa_hchacha() {
    setup();
    let mut rng = Rng::new(0x2222);
    // (name, outlen, inlen, klen, clen)
    let specs: &[(&str, usize, usize, usize, usize)] = &[
        ("crypto_core_salsa20", 64, 16, 32, 16),
        ("crypto_core_salsa2012", 64, 16, 32, 16),
        ("crypto_core_salsa208", 64, 16, 32, 16),
        ("crypto_core_hsalsa20", 32, 16, 32, 16),
        ("crypto_core_hchacha20", 32, 16, 32, 16),
    ];
    for &(name, outlen, inlen, klen, clen) in specs {
        // sanity: the accessor-reported sizes must agree with the table
        let ob = sym::<unsafe extern "C" fn() -> usize>(c_lib(), &format!("{name}_outputbytes"));
        assert_eq!(unsafe { ob() }, outlen, "{name}_outputbytes");
        let (c, r) = pair::<Core>(name);
        for iter in 0..200 {
            let inp = match iter % 4 {
                0 => rng.bytes(inlen),
                1 => vec![0u8; inlen],
                2 => vec![0xffu8; inlen],
                _ => rng.bytes(inlen),
            };
            let k = match iter % 3 {
                0 => rng.bytes(klen),
                1 => vec![0u8; klen],
                _ => vec![0xffu8; klen],
            };
            // `c` = NULL takes the built-in sigma path; non-NULL loads from `c`
            let cst: Option<Vec<u8>> = match iter % 3 {
                0 => None,
                1 => Some(b"expand 32-byte k".to_vec()),
                _ => Some(rng.bytes(clen)),
            };
            let cp = cst.as_ref().map_or(std::ptr::null(), |v| v.as_ptr());
            let mut o1 = canary(outlen);
            let mut o2 = canary(outlen);
            let (a, b) = unsafe {
                (
                    c(o1.as_mut_ptr(), inp.as_ptr(), k.as_ptr(), cp),
                    r(o2.as_mut_ptr(), inp.as_ptr(), k.as_ptr(), cp),
                )
            };
            eq_i32(&format!("{name} rc"), a, b);
            eq_bytes(
                &format!(
                    "{name}(in={}, k={}, c={})",
                    hex(&inp),
                    hex(&k),
                    cst.as_ref().map_or("NULL".into(), |v| hex(v))
                ),
                &o1,
                &o2,
            );
        }
    }
}

// ---------------------------------------------------------------------------
// CONFIGS rows: every crypto_stream_* keystream / _xor entry point
// ---------------------------------------------------------------------------

/// (module prefix, key bytes, nonce bytes)
const STREAMS: &[(&str, usize, usize)] = &[
    ("crypto_stream", 32, 24),
    ("crypto_stream_chacha20", 32, 8),
    ("crypto_stream_chacha20_ietf", 32, 12),
    ("crypto_stream_salsa20", 32, 8),
    ("crypto_stream_salsa2012", 32, 8),
    ("crypto_stream_salsa208", 32, 8),
    ("crypto_stream_xchacha20", 32, 24),
    ("crypto_stream_xsalsa20", 32, 24),
];

fn stream_sizes(prefix: &str) -> (usize, usize) {
    let kb = sym::<unsafe extern "C" fn() -> usize>(c_lib(), &format!("{prefix}_keybytes"));
    let nb = sym::<unsafe extern "C" fn() -> usize>(c_lib(), &format!("{prefix}_noncebytes"));
    unsafe { (kb(), nb()) }
}

#[test]
fn stream_keystream_and_xor() {
    setup();
    let mut rng = Rng::new(0x3333);
    for &(prefix, kb_exp, nb_exp) in STREAMS {
        let (kb, nb) = stream_sizes(prefix);
        assert_eq!((kb, nb), (kb_exp, nb_exp), "{prefix} key/nonce sizes");

        let (cks, rks) = pair::<Stream>(prefix);
        let (cx, rx) = pair::<StreamXor>(&format!("{prefix}_xor"));

        for &len in LENS {
            for kind in 0..3 {
                let k = match kind {
                    0 => rng.bytes(kb),
                    1 => vec![0u8; kb],
                    _ => vec![0xffu8; kb],
                };
                let n = match kind {
                    0 => rng.bytes(nb),
                    1 => vec![0u8; nb],
                    _ => vec![0xffu8; nb],
                };
                // keystream
                let mut a = canary(len);
                let mut b = canary(len);
                let (ra, rb) = unsafe {
                    (
                        cks(a.as_mut_ptr(), len as u64, n.as_ptr(), k.as_ptr()),
                        rks(b.as_mut_ptr(), len as u64, n.as_ptr(), k.as_ptr()),
                    )
                };
                eq_i32(&format!("{prefix}({len}) rc"), ra, rb);
                eq_bytes(&format!("{prefix}(len={len}, kind={kind})"), &a, &b);

                // xor
                let m = rng.bytes(len);
                let mut a = canary(len);
                let mut b = canary(len);
                let (ra, rb) = unsafe {
                    (
                        cx(
                            a.as_mut_ptr(),
                            m.as_ptr(),
                            len as u64,
                            n.as_ptr(),
                            k.as_ptr(),
                        ),
                        rx(
                            b.as_mut_ptr(),
                            m.as_ptr(),
                            len as u64,
                            n.as_ptr(),
                            k.as_ptr(),
                        ),
                    )
                };
                eq_i32(&format!("{prefix}_xor({len}) rc"), ra, rb);
                eq_bytes(&format!("{prefix}_xor(len={len}, kind={kind})"), &a, &b);
            }
        }
    }
}

/// In-place `_xor` (`c == m`), which the C code supports.
#[test]
fn stream_xor_in_place() {
    setup();
    let mut rng = Rng::new(0x3334);
    for &(prefix, _, _) in STREAMS {
        let (kb, nb) = stream_sizes(prefix);
        let (cx, rx) = pair::<StreamXor>(&format!("{prefix}_xor"));
        for &len in &[0usize, 1, 63, 64, 65, 128, 200, 1000] {
            let k = rng.bytes(kb);
            let n = rng.bytes(nb);
            let m = rng.bytes(len);
            let mut a = m.clone();
            let mut b = m.clone();
            unsafe {
                let ra = cx(
                    a.as_mut_ptr(),
                    a.as_ptr(),
                    len as u64,
                    n.as_ptr(),
                    k.as_ptr(),
                );
                let rb = rx(
                    b.as_mut_ptr(),
                    b.as_ptr(),
                    len as u64,
                    n.as_ptr(),
                    k.as_ptr(),
                );
                eq_i32(&format!("{prefix}_xor in-place rc"), ra, rb);
            }
            eq_bytes(&format!("{prefix}_xor in-place len={len}"), &a, &b);
        }
    }
}

/// `_xor_ic` with a 64-bit initial counter: chacha20 (original nonce),
/// salsa20, xchacha20, xsalsa20.
#[test]
fn stream_xor_ic_64bit_counter() {
    setup();
    let mut rng = Rng::new(0x4444);
    let names: &[&str] = &[
        "crypto_stream_chacha20_xor_ic",
        "crypto_stream_salsa20_xor_ic",
        "crypto_stream_xchacha20_xor_ic",
        "crypto_stream_xsalsa20_xor_ic",
    ];
    let ics: &[u64] = &[
        0,
        1,
        2,
        7,
        0xffff_ffff,
        0x1_0000_0000,
        0x1_0000_0001,
        u64::MAX - 20,
    ];
    for name in names {
        let prefix = name.trim_end_matches("_xor_ic");
        let (kb, nb) = stream_sizes(prefix);
        let (c, r) = pair::<StreamXorIc64>(name);
        for &len in &[0usize, 1, 63, 64, 65, 127, 128, 129, 256, 300] {
            for &ic in ics {
                let k = rng.bytes(kb);
                let n = rng.bytes(nb);
                let m = rng.bytes(len);
                let mut a = canary(len);
                let mut b = canary(len);
                let (ra, rb) = unsafe {
                    (
                        c(
                            a.as_mut_ptr(),
                            m.as_ptr(),
                            len as u64,
                            n.as_ptr(),
                            ic,
                            k.as_ptr(),
                        ),
                        r(
                            b.as_mut_ptr(),
                            m.as_ptr(),
                            len as u64,
                            n.as_ptr(),
                            ic,
                            k.as_ptr(),
                        ),
                    )
                };
                eq_i32(&format!("{name}(len={len}, ic={ic}) rc"), ra, rb);
                eq_bytes(&format!("{name}(len={len}, ic={ic:#x})"), &a, &b);
            }
        }
    }
}

/// `crypto_stream_chacha20_ietf_xor_ic` — 32-bit counter. `ic` values near the
/// 2^32 wrap must be exercised, but the *overflow* case is an error path
/// (`sodium_misuse`) and lives in the Phase C tests.
#[test]
fn stream_chacha20_ietf_xor_ic_32bit_counter() {
    setup();
    let mut rng = Rng::new(0x5555);
    let (c, r) = pair::<StreamXorIc32>("crypto_stream_chacha20_ietf_xor_ic");
    for &len in &[0usize, 1, 63, 64, 65, 127, 128, 129, 256, 300] {
        for &ic in &[0u32, 1, 2, 7, 1000, 0xffff_fff0, 0xffff_fffe] {
            // stay within the 32-bit counter range: ic + ceil(len/64) <= 2^32
            let blocks = ((len + 63) / 64) as u64;
            if ic as u64 + blocks > 0x1_0000_0000 {
                continue;
            }
            let k = rng.bytes(32);
            let n = rng.bytes(12);
            let m = rng.bytes(len);
            let mut a = canary(len);
            let mut b = canary(len);
            let (ra, rb) = unsafe {
                (
                    c(
                        a.as_mut_ptr(),
                        m.as_ptr(),
                        len as u64,
                        n.as_ptr(),
                        ic,
                        k.as_ptr(),
                    ),
                    r(
                        b.as_mut_ptr(),
                        m.as_ptr(),
                        len as u64,
                        n.as_ptr(),
                        ic,
                        k.as_ptr(),
                    ),
                )
            };
            eq_i32(&format!("ietf_xor_ic(len={len}, ic={ic}) rc"), ra, rb);
            eq_bytes(
                &format!("crypto_stream_chacha20_ietf_xor_ic(len={len}, ic={ic:#x})"),
                &a,
                &b,
            );
        }
    }
}

/// The internal `crypto_stream_chacha20_ietf_ext` / `_ext_xor_ic` exports
/// (used by `crypto_secretstream`): 16-byte nonce, 32-bit counter.
#[test]
fn stream_chacha20_ietf_ext() {
    setup();
    let mut rng = Rng::new(0x5556);
    let (c, r) = pair::<Stream>("crypto_stream_chacha20_ietf_ext");
    for &len in &[0usize, 1, 63, 64, 65, 128, 200, 1000] {
        let k = rng.bytes(32);
        let n = rng.bytes(16);
        let mut a = canary(len);
        let mut b = canary(len);
        let (ra, rb) = unsafe {
            (
                c(a.as_mut_ptr(), len as u64, n.as_ptr(), k.as_ptr()),
                r(b.as_mut_ptr(), len as u64, n.as_ptr(), k.as_ptr()),
            )
        };
        eq_i32("ietf_ext rc", ra, rb);
        eq_bytes(&format!("crypto_stream_chacha20_ietf_ext(len={len})"), &a, &b);
    }

    let (c, r) = pair::<StreamXorIc32>("crypto_stream_chacha20_ietf_ext_xor_ic");
    for &len in &[0usize, 1, 63, 64, 65, 128, 200, 1000] {
        for &ic in &[0u32, 1, 5, 1000] {
            let k = rng.bytes(32);
            let n = rng.bytes(16);
            let m = rng.bytes(len);
            let mut a = canary(len);
            let mut b = canary(len);
            let (ra, rb) = unsafe {
                (
                    c(
                        a.as_mut_ptr(),
                        m.as_ptr(),
                        len as u64,
                        n.as_ptr(),
                        ic,
                        k.as_ptr(),
                    ),
                    r(
                        b.as_mut_ptr(),
                        m.as_ptr(),
                        len as u64,
                        n.as_ptr(),
                        ic,
                        k.as_ptr(),
                    ),
                )
            };
            eq_i32("ietf_ext_xor_ic rc", ra, rb);
            eq_bytes(
                &format!("crypto_stream_chacha20_ietf_ext_xor_ic(len={len}, ic={ic})"),
                &a,
                &b,
            );
        }
    }
}

/// Every `crypto_stream_*_keygen` — deterministic under the installed RNG.
#[test]
fn stream_keygens() {
    setup();
    let mut names: Vec<String> = STREAMS.iter().map(|s| format!("{}_keygen", s.0)).collect();
    names.push("crypto_stream_chacha20_ietf_keygen".into());
    names.sort();
    names.dedup();
    for name in &names {
        let prefix = name.trim_end_matches("_keygen");
        let (kb, _) = stream_sizes(prefix);
        let (c, r) = pair::<Keygen>(name);
        for seed in 0..8u64 {
            let mut a = canary(kb);
            let mut b = canary(kb);
            reset_rngs(0xA000 + seed);
            unsafe { c(a.as_mut_ptr()) };
            reset_rngs(0xA000 + seed);
            unsafe { r(b.as_mut_ptr()) };
            eq_bytes(&format!("{name} seed={seed}"), &a, &b);
            assert_ne!(a, canary(kb), "{name} wrote nothing");
        }
    }
}

// ---------------------------------------------------------------------------
// CONFIGS rows: crypto_shorthash_siphash24 / _siphashx24 / generic
// ---------------------------------------------------------------------------

#[test]
fn shorthash_siphash24_and_x24() {
    setup();
    let mut rng = Rng::new(0x6666);
    for (name, outlen, keylen) in [
        ("crypto_shorthash", 8usize, 16usize),
        ("crypto_shorthash_siphash24", 8, 16),
        ("crypto_shorthash_siphashx24", 16, 16),
    ] {
        let (c, r) = pair::<Shorthash>(name);
        // 0..=32 covers all 8 tail-length cases of the SipHash finaliser
        let lens: Vec<usize> = (0usize..=40)
            .chain([63, 64, 65, 100, 127, 128, 1000])
            .collect();
        for &len in &lens {
            for kind in 0..3 {
                let k = match kind {
                    0 => rng.bytes(keylen),
                    1 => vec![0u8; keylen],
                    _ => vec![0xffu8; keylen],
                };
                let inp = match kind {
                    0 => rng.bytes(len),
                    1 => vec![0u8; len],
                    _ => vec![0xffu8; len],
                };
                let mut a = canary(outlen);
                let mut b = canary(outlen);
                let (ra, rb) = unsafe {
                    (
                        c(a.as_mut_ptr(), inp.as_ptr(), len as u64, k.as_ptr()),
                        r(b.as_mut_ptr(), inp.as_ptr(), len as u64, k.as_ptr()),
                    )
                };
                eq_i32(&format!("{name} rc"), ra, rb);
                eq_bytes(&format!("{name}(len={len}, kind={kind})"), &a, &b);
            }
        }
    }
}

#[test]
fn shorthash_keygen() {
    setup();
    let (c, r) = pair::<Keygen>("crypto_shorthash_keygen");
    for seed in 0..8u64 {
        let mut a = canary(16);
        let mut b = canary(16);
        reset_rngs(0xB000 + seed);
        unsafe { c(a.as_mut_ptr()) };
        reset_rngs(0xB000 + seed);
        unsafe { r(b.as_mut_ptr()) };
        eq_bytes("crypto_shorthash_keygen", &a, &b);
    }
}
