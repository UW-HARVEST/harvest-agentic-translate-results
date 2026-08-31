//! Area 3 — `crypto_shorthash` / SipHash-2-4 and SipHash-2-4-128.
//!
//! Covers `crypto_shorthash/crypto_shorthash.c`,
//! `crypto_shorthash/siphash24/{shorthash_siphash24.c, shorthash_siphashx24.c}`
//! and `crypto_shorthash/siphash24/ref/{shorthash_siphash24_ref.c,
//! shorthash_siphashx24_ref.c, shorthash_siphash_ref.h}`.
//!
//! CONFIGS rows 3.117–3.127, ERRORS rows 3.101–3.107.
//!
//! In addition to the C-vs-Rust differential comparison, every digest is
//! checked against an independent SipHash-2-4 implementation written here
//! straight from the SipHash specification (`oracle`).  That third oracle is
//! what makes the `inlen << 56` length-byte aliasing an assertion rather than a
//! tautology: the spec says the final block's high byte is `inlen mod 256`, and
//! `aliasing_of_the_length_byte` proves the libraries agree with that for
//! `inlen >= 256`.
mod common;
use common::*;
use std::ffi::c_char;
use std::ffi::c_int;

// --------------------------------------------------------------- signatures

/// `crypto_shorthash{,_siphash24,_siphashx24}(out, in, inlen, k)`
type Sh = unsafe extern "C" fn(*mut u8, *const u8, u64, *const u8) -> c_int;
type SizeFn = unsafe extern "C" fn() -> usize;
type StrFn = unsafe extern "C" fn() -> *const c_char;
type Keygen = unsafe extern "C" fn(*mut u8);

const KEYBYTES: usize = 16;
const BYTES: usize = 8;
const X_BYTES: usize = 16;

// ------------------------------------------------------ independent oracle

fn siprounds(v: &mut [u64; 4], n: usize) {
    for _ in 0..n {
        v[0] = v[0].wrapping_add(v[1]);
        v[2] = v[2].wrapping_add(v[3]);
        v[1] = v[1].rotate_left(13);
        v[3] = v[3].rotate_left(16);
        v[1] ^= v[0];
        v[3] ^= v[2];
        v[0] = v[0].rotate_left(32);
        v[0] = v[0].wrapping_add(v[3]);
        v[2] = v[2].wrapping_add(v[1]);
        v[3] = v[3].rotate_left(21);
        v[1] = v[1].rotate_left(17);
        v[3] ^= v[0];
        v[1] ^= v[2];
        v[2] = v[2].rotate_left(32);
    }
}

/// SipHash-2-4 (`x == false`, 8-byte tag) / SipHash-2-4-128 (`x == true`,
/// 16-byte tag), with the length byte supplied explicitly so that the
/// `inlen mod 256` aliasing can be asserted rather than assumed.
fn oracle_len(msg: &[u8], k: &[u8], x: bool, lenbyte: u64) -> Vec<u8> {
    let k0 = u64::from_le_bytes(k[0..8].try_into().unwrap());
    let k1 = u64::from_le_bytes(k[8..16].try_into().unwrap());
    let mut v = [
        0x736f_6d65_7073_6575u64 ^ k0,
        (if x { 0x646f_7261_6e64_6f83u64 } else { 0x646f_7261_6e64_6f6du64 }) ^ k1,
        0x6c79_6765_6e65_7261u64 ^ k0,
        0x7465_6462_7974_6573u64 ^ k1,
    ];
    let nwords = msg.len() / 8;
    let mut b: u64 = lenbyte << 56;
    for i in 0..nwords {
        let m = u64::from_le_bytes(msg[i * 8..i * 8 + 8].try_into().unwrap());
        v[3] ^= m;
        siprounds(&mut v, 2);
        v[0] ^= m;
    }
    for (i, &byte) in msg[nwords * 8..].iter().enumerate() {
        b |= (byte as u64) << (8 * i);
    }
    v[3] ^= b;
    siprounds(&mut v, 2);
    v[0] ^= b;
    v[2] ^= if x { 0xee } else { 0xff };
    siprounds(&mut v, 4);
    let mut out = Vec::with_capacity(if x { 16 } else { 8 });
    out.extend_from_slice(&(v[0] ^ v[1] ^ v[2] ^ v[3]).to_le_bytes());
    if x {
        v[1] ^= 0xdd;
        siprounds(&mut v, 4);
        out.extend_from_slice(&(v[0] ^ v[1] ^ v[2] ^ v[3]).to_le_bytes());
    }
    out
}

fn oracle(msg: &[u8], k: &[u8], x: bool) -> Vec<u8> {
    oracle_len(msg, k, x, msg.len() as u64)
}

// ------------------------------------------------------------------ helpers

fn pattern(kind: u8, len: usize) -> Vec<u8> {
    match kind {
        0 => vec![0u8; len],
        1 => vec![0xffu8; len],
        2 => (0..len).map(|i| (i & 0xff) as u8).collect(),
        3 => (0..len).map(|i| (0xff - (i & 0xff)) as u8).collect(),
        _ => Rng::new(0x5117_A511_0000 ^ (len as u64) ^ ((kind as u64) << 40)).bytes(len),
    }
}

/// Every key shape used below: the RFC/reference key `00 01 .. 0f`, all-zero,
/// all-`0xff`, and three random keys.
fn keys() -> Vec<[u8; KEYBYTES]> {
    let mut v: Vec<[u8; KEYBYTES]> = Vec::new();
    let mut rfc = [0u8; KEYBYTES];
    for (i, x) in rfc.iter_mut().enumerate() {
        *x = i as u8;
    }
    v.push(rfc);
    v.push([0u8; KEYBYTES]);
    v.push([0xffu8; KEYBYTES]);
    let mut rng = Rng::new(0x5150_4B3A);
    for _ in 0..3 {
        let mut k = [0u8; KEYBYTES];
        rng.fill(&mut k);
        v.push(k);
    }
    v
}

/// Input lengths: every value 0..=300 (which contains 255/256/257 and all eight
/// `inlen & 7` residues at many word counts) plus larger values.
fn lengths() -> Vec<usize> {
    let mut v: Vec<usize> = (0..=300).collect();
    v.extend_from_slice(&[
        303, 383, 384, 385, 503, 504, 505, 511, 512, 513, 767, 768, 1023, 1024, 1025, 4096,
    ]);
    v
}

/// One differential call: C, Rust and the independent oracle must all agree,
/// and neither library may write past `outlen`.
#[track_caller]
unsafe fn call(
    what: &str,
    cf: &libloading::Symbol<'static, Sh>,
    rf: &libloading::Symbol<'static, Sh>,
    outlen: usize,
    msg: &[u8],
    inlen: u64,
    k: &[u8],
) -> Vec<u8> {
    let mut co = padded(outlen);
    let mut ro = padded(outlen);
    let a = cf(co.as_mut_ptr(), msg.as_ptr(), inlen, k.as_ptr());
    let b = rf(ro.as_mut_ptr(), msg.as_ptr(), inlen, k.as_ptr());
    assert_eq!(a, 0, "{what}: C must always return 0, got {a}");
    eqi(&format!("{what}: rc"), a, b);
    check_pad(&format!("{what}: C out"), &co, outlen);
    check_pad(&format!("{what}: Rust out"), &ro, outlen);
    eqb(&format!("{what}: tag"), &co[..outlen], &ro[..outlen]);
    co[..outlen].to_vec()
}

unsafe fn cstr(p: *const c_char) -> String {
    std::ffi::CStr::from_ptr(p).to_string_lossy().into_owned()
}

// ===========================================================================
// accessors — CONFIGS 3.127, ERRORS 3.107
// ===========================================================================

#[test]
fn accessors() {
    let table: &[(&str, usize)] = &[
        ("crypto_shorthash_bytes", BYTES),
        ("crypto_shorthash_keybytes", KEYBYTES),
        ("crypto_shorthash_siphash24_bytes", BYTES),
        ("crypto_shorthash_siphash24_keybytes", KEYBYTES),
        ("crypto_shorthash_siphashx24_bytes", X_BYTES),
        ("crypto_shorthash_siphashx24_keybytes", KEYBYTES),
    ];
    for (name, want) in table {
        assert!(has(name), "{name} must be exported by both");
        let (c, r) = both::<SizeFn>(name);
        unsafe {
            let (a, b) = (c(), r());
            assert_eq!(a, *want, "C {name}() == {a}, expected {want}");
            assert_eq!(a, b, "{name}(): C {a} vs Rust {b}");
        }
    }
    let (c, r) = both::<StrFn>("crypto_shorthash_primitive");
    unsafe {
        let a = cstr(c());
        let b = cstr(r());
        assert_eq!(a, "siphash24", "C crypto_shorthash_primitive() == {a:?}");
        assert_eq!(a, b);
    }
}

#[test]
fn keygen() {
    // ERRORS 3.106: `randombytes_buf(k, 16)`, void return, no error surface.
    let (c, r) = both::<Keygen>("crypto_shorthash_keygen");
    for _ in 0..8 {
        let mut ck = padded(KEYBYTES);
        let mut rk = padded(KEYBYTES);
        rng_reset();
        unsafe {
            c(ck.as_mut_ptr());
            r(rk.as_mut_ptr());
        }
        check_pad("keygen: C", &ck, KEYBYTES);
        check_pad("keygen: Rust", &rk, KEYBYTES);
        eqb("crypto_shorthash_keygen", &ck, &rk);
        assert!(ck[..KEYBYTES].iter().any(|&x| x != 0), "all-zero key");
    }
}

// ===========================================================================
// the oracle anchors the whole file — an independent SipHash-2-4
// ===========================================================================

#[test]
fn oracle_matches_the_reference_siphash_vector() {
    // SipHash-2-4 reference vector: key = 00 01 .. 0f, empty message.
    let mut k = [0u8; KEYBYTES];
    for (i, x) in k.iter_mut().enumerate() {
        *x = i as u8;
    }
    assert_eq!(
        hex(&oracle(&[], &k, false)),
        "310e0edd47db6f72",
        "the in-test SipHash-2-4 oracle is itself wrong"
    );
    // and the same vector through both libraries
    let (c, r) = both::<Sh>("crypto_shorthash_siphash24");
    unsafe {
        let d = call("siphash24(\"\")", &c, &r, BYTES, &[], 0, &k);
        assert_eq!(hex(&d), "310e0edd47db6f72");
    }
}

// ===========================================================================
// siphash24 — CONFIGS 3.117, 3.118, 3.121, 3.126, 3.128; ERRORS 3.101–3.103
// ===========================================================================

#[test]
fn siphash24_all_lengths_all_keys() {
    let (c, r) = both::<Sh>("crypto_shorthash_siphash24");
    let (wc, wr) = both::<Sh>("crypto_shorthash");
    let ks = keys();
    for len in lengths() {
        for kind in [0u8, 1, 2, 3, 4] {
            let msg = pattern(kind, len);
            for (ki, k) in ks.iter().enumerate() {
                let what = format!("siphash24(len={len},kind={kind},key#{ki})");
                let d = unsafe { call(&what, &c, &r, BYTES, &msg, len as u64, k) };
                assert_eq!(
                    d,
                    oracle(&msg, k, false),
                    "{what}: disagrees with the independent SipHash-2-4 oracle"
                );
                // CONFIGS 3.126 / 3.131: the generic wrapper is pure delegation
                let dw = unsafe {
                    call(&format!("{what} via crypto_shorthash"), &wc, &wr, BYTES, &msg, len as u64, k)
                };
                eqb(&format!("{what}: crypto_shorthash == siphash24"), &d, &dw);
            }
        }
    }
}

#[test]
fn siphash24_every_tail_residue() {
    // CONFIGS 3.117/3.118: the `switch (left)` fall-through chain, i.e. every
    // `inlen & 7` residue, at 0, 1, 2, 3, 8, 16, 32 and 128 full words.
    let (c, r) = both::<Sh>("crypto_shorthash_siphash24");
    let (xc, xr) = both::<Sh>("crypto_shorthash_siphashx24");
    let ks = keys();
    for words in [0usize, 1, 2, 3, 8, 16, 32, 128] {
        for left in 0..8usize {
            let len = words * 8 + left;
            for kind in [0u8, 1, 2, 4] {
                let msg = pattern(kind, len);
                for (ki, k) in ks.iter().enumerate() {
                    let what = format!("residue(words={words},left={left},kind={kind},key#{ki})");
                    let d = unsafe { call(&what, &c, &r, BYTES, &msg, len as u64, k) };
                    assert_eq!(d, oracle(&msg, k, false), "{what}");
                    let dx = unsafe { call(&what, &xc, &xr, X_BYTES, &msg, len as u64, k) };
                    assert_eq!(dx, oracle(&msg, k, true), "{what} (x24)");
                }
            }
        }
    }
}

#[test]
fn siphash24_zero_length_with_null_input() {
    // ERRORS 3.102 / CONFIGS 3.119: `end = inlen ? in + inlen - inlen % 8 : in`
    // means `end == in`, `left == 0`, so nothing is dereferenced.
    let (c, r) = both::<Sh>("crypto_shorthash_siphash24");
    let (xc, xr) = both::<Sh>("crypto_shorthash_siphashx24");
    let (wc, wr) = both::<Sh>("crypto_shorthash");
    for k in keys() {
        let empty: [u8; 1] = [0];
        unsafe {
            let mut co = padded(BYTES);
            let mut ro = padded(BYTES);
            let a = c(co.as_mut_ptr(), std::ptr::null(), 0, k.as_ptr());
            let b = r(ro.as_mut_ptr(), std::ptr::null(), 0, k.as_ptr());
            assert_eq!(a, 0);
            eqi("siphash24(NULL, 0) rc", a, b);
            check_pad("siphash24(NULL,0) C", &co, BYTES);
            check_pad("siphash24(NULL,0) Rust", &ro, BYTES);
            eqb("siphash24(NULL, 0)", &co[..BYTES], &ro[..BYTES]);
            assert_eq!(&co[..BYTES], &oracle(&[], &k, false)[..]);
            let non_null = call("siphash24(non-NULL, 0)", &c, &r, BYTES, &empty, 0, &k);
            eqb("in=NULL,inlen=0 == in!=NULL,inlen=0", &co[..BYTES], &non_null);

            let mut xo = padded(X_BYTES);
            let mut xro = padded(X_BYTES);
            let a = xc(xo.as_mut_ptr(), std::ptr::null(), 0, k.as_ptr());
            let b = xr(xro.as_mut_ptr(), std::ptr::null(), 0, k.as_ptr());
            assert_eq!(a, 0);
            eqi("siphashx24(NULL, 0) rc", a, b);
            eqb("siphashx24(NULL, 0)", &xo[..X_BYTES], &xro[..X_BYTES]);
            assert_eq!(&xo[..X_BYTES], &oracle(&[], &k, true)[..]);

            let mut go = padded(BYTES);
            let mut gro = padded(BYTES);
            let a = wc(go.as_mut_ptr(), std::ptr::null(), 0, k.as_ptr());
            let b = wr(gro.as_mut_ptr(), std::ptr::null(), 0, k.as_ptr());
            assert_eq!(a, 0);
            eqi("crypto_shorthash(NULL, 0) rc", a, b);
            eqb("crypto_shorthash(NULL, 0)", &go[..BYTES], &gro[..BYTES]);
            eqb("generic == siphash24 for the empty input", &co[..BYTES], &go[..BYTES]);
        }
    }
}

#[test]
fn aliasing_of_the_length_byte() {
    // CONFIGS 3.120 / ERRORS 3.103: `b = ((uint64_t) inlen) << 56` keeps only
    // `inlen mod 256`, so 256 and 0, 257 and 1, 511 and 255 … contribute the
    // *same* length byte.  Proven by evaluating the oracle twice: once with the
    // real `inlen` and once with `inlen & 0xff`, which must be identical, and by
    // pinning the libraries to that oracle.
    let (c, r) = both::<Sh>("crypto_shorthash_siphash24");
    let (xc, xr) = both::<Sh>("crypto_shorthash_siphashx24");
    let ks = keys();
    let mut lens: Vec<usize> = vec![];
    for base in [0usize, 1, 2, 7, 8, 9, 15, 16, 63, 64, 128, 200, 248, 255] {
        for mul in 0..5usize {
            lens.push(base + 256 * mul);
        }
    }
    lens.push(4096);
    lens.push(65536);
    for len in lens {
        for kind in [2u8, 4] {
            let msg = pattern(kind, len);
            for (ki, k) in ks.iter().enumerate() {
                let full = oracle_len(&msg, k, false, len as u64);
                let aliased = oracle_len(&msg, k, false, (len & 0xff) as u64);
                assert_eq!(
                    full, aliased,
                    "length byte must alias mod 256 (len={len})"
                );
                let what = format!("alias(len={len},kind={kind},key#{ki})");
                let d = unsafe { call(&what, &c, &r, BYTES, &msg, len as u64, k) };
                assert_eq!(d, full, "{what}");
                let dx = unsafe { call(&what, &xc, &xr, X_BYTES, &msg, len as u64, k) };
                assert_eq!(dx, oracle_len(&msg, k, true, len as u64), "{what} (x24)");
                assert_eq!(
                    dx,
                    oracle_len(&msg, k, true, (len & 0xff) as u64),
                    "{what} (x24) length byte must alias mod 256"
                );
            }
        }
    }
}

// ===========================================================================
// siphashx24 — CONFIGS 3.122–3.125
// ===========================================================================

#[test]
fn siphashx24_all_lengths_all_keys() {
    let (c, r) = both::<Sh>("crypto_shorthash_siphashx24");
    let ks = keys();
    for len in lengths() {
        for kind in [0u8, 1, 2, 3, 4] {
            let msg = pattern(kind, len);
            for (ki, k) in ks.iter().enumerate() {
                let what = format!("siphashx24(len={len},kind={kind},key#{ki})");
                let d = unsafe { call(&what, &c, &r, X_BYTES, &msg, len as u64, k) };
                assert_eq!(
                    d,
                    oracle(&msg, k, true),
                    "{what}: disagrees with the independent SipHash-2-4-128 oracle"
                );
            }
        }
    }
}

#[test]
fn siphashx24_differs_from_siphash24_and_has_two_independent_halves() {
    // CONFIGS 3.124: different `v1` initializer (`…646f83`) and `v2 ^= 0xee`
    // instead of `0xff`, so even the first 8 bytes differ from siphash24.
    // CONFIGS 3.125: bytes 8..16 come from `v1 ^= 0xdd` plus 4 more SIPROUNDs
    // *after* bytes 0..8 have been stored, so the two halves are separately
    // reproducible and must never coincide.
    let (c, r) = both::<Sh>("crypto_shorthash_siphash24");
    let (xc, xr) = both::<Sh>("crypto_shorthash_siphashx24");
    let ks = keys();
    for len in 0..=64usize {
        for kind in [0u8, 1, 2, 4] {
            let msg = pattern(kind, len);
            for (ki, k) in ks.iter().enumerate() {
                let what = format!("x24-vs-24(len={len},kind={kind},key#{ki})");
                let d8 = unsafe { call(&what, &c, &r, BYTES, &msg, len as u64, k) };
                let d16 = unsafe { call(&what, &xc, &xr, X_BYTES, &msg, len as u64, k) };
                assert_ne!(
                    &d16[..8],
                    &d8[..],
                    "{what}: siphashx24's first half must differ from siphash24"
                );
                assert_ne!(&d16[..8], &d16[8..], "{what}: the two halves coincided");
                let o = oracle(&msg, k, true);
                assert_eq!(&d16[..8], &o[..8], "{what}: first half");
                assert_eq!(&d16[8..], &o[8..], "{what}: second half");
            }
        }
    }
}

// ===========================================================================
// key sensitivity, aliasing, content axis
// ===========================================================================

#[test]
fn key_bit_sensitivity() {
    // Confirms `k0 = LOAD64_LE(k)` / `k1 = LOAD64_LE(k + 8)`: flipping any one
    // of the 128 key bits must change both tags.
    let (c, r) = both::<Sh>("crypto_shorthash_siphash24");
    let (xc, xr) = both::<Sh>("crypto_shorthash_siphashx24");
    let base_key = keys()[0];
    for &len in &[0usize, 1, 7, 8, 9, 16, 33, 64, 129] {
        let msg = pattern(2, len);
        let base8 = unsafe { call("base", &c, &r, BYTES, &msg, len as u64, &base_key) };
        let base16 = unsafe { call("base", &xc, &xr, X_BYTES, &msg, len as u64, &base_key) };
        for byte in 0..KEYBYTES {
            for bit in 0..8u32 {
                let mut k = base_key;
                k[byte] ^= 1 << bit;
                let what = format!("keyflip({byte},{bit},len={len})");
                let d8 = unsafe { call(&what, &c, &r, BYTES, &msg, len as u64, &k) };
                let d16 = unsafe { call(&what, &xc, &xr, X_BYTES, &msg, len as u64, &k) };
                assert_ne!(d8, base8, "{what}: siphash24 ignored a key bit");
                assert_ne!(d16, base16, "{what}: siphashx24 ignored a key bit");
                assert_eq!(d8, oracle(&msg, &k, false), "{what}");
                assert_eq!(d16, oracle(&msg, &k, true), "{what}");
            }
        }
    }
}

#[test]
fn message_bit_sensitivity() {
    // Every message byte must reach the tag; catches byte-transposition bugs in
    // `LOAD64_LE` and in the `switch (left)` fall-through.
    let (c, r) = both::<Sh>("crypto_shorthash_siphash24");
    let (xc, xr) = both::<Sh>("crypto_shorthash_siphashx24");
    let k = keys()[3];
    for len in 1..=40usize {
        let msg = pattern(2, len);
        let base8 = unsafe { call("base", &c, &r, BYTES, &msg, len as u64, &k) };
        let base16 = unsafe { call("base", &xc, &xr, X_BYTES, &msg, len as u64, &k) };
        for i in 0..len {
            for bit in [0u32, 3, 7] {
                let mut m = msg.clone();
                m[i] ^= 1 << bit;
                let what = format!("msgflip(len={len},byte={i},bit={bit})");
                let d8 = unsafe { call(&what, &c, &r, BYTES, &m, len as u64, &k) };
                let d16 = unsafe { call(&what, &xc, &xr, X_BYTES, &m, len as u64, &k) };
                assert_ne!(d8, base8, "{what}: siphash24 ignored a message bit");
                assert_ne!(d16, base16, "{what}: siphashx24 ignored a message bit");
                assert_eq!(d8, oracle(&m, &k, false), "{what}");
                assert_eq!(d16, oracle(&m, &k, true), "{what}");
            }
        }
    }
}

#[test]
fn randomized_fuzz() {
    // Randomized (fixed seed) keys × messages, all three entry points.
    let (c, r) = both::<Sh>("crypto_shorthash_siphash24");
    let (xc, xr) = both::<Sh>("crypto_shorthash_siphashx24");
    let (wc, wr) = both::<Sh>("crypto_shorthash");
    let mut rng = Rng::new(0x5150_F0F0_2222);
    for i in 0..3000usize {
        let len = match rng.below(4) {
            0 => rng.below(17),
            1 => rng.below(300),
            2 => 240 + rng.below(40),
            _ => rng.below(2000),
        };
        let msg = rng.bytes(len);
        let k = rng.bytes(KEYBYTES);
        let what = format!("fuzz#{i}(len={len})");
        let d8 = unsafe { call(&what, &c, &r, BYTES, &msg, len as u64, &k) };
        assert_eq!(d8, oracle(&msg, &k, false), "{what}");
        let d16 = unsafe { call(&what, &xc, &xr, X_BYTES, &msg, len as u64, &k) };
        assert_eq!(d16, oracle(&msg, &k, true), "{what}");
        let dw = unsafe { call(&what, &wc, &wr, BYTES, &msg, len as u64, &k) };
        eqb(&format!("{what}: generic == siphash24"), &d8, &dw);
    }
}

#[test]
fn aliased_out_and_in() {
    // CONFIGS 3.130: `out` may overlap `in`; the tag is stored only after the
    // whole message has been absorbed.
    let (c, r) = both::<Sh>("crypto_shorthash_siphash24");
    let (xc, xr) = both::<Sh>("crypto_shorthash_siphashx24");
    let (wc, wr) = both::<Sh>("crypto_shorthash");
    let ks = keys();
    for &len in &[8usize, 16, 17, 23, 24, 64, 65, 128, 256, 257, 300] {
        for &off in &[0usize, 1, 7] {
            for (ki, k) in ks.iter().enumerate() {
                let src = pattern(2, len);
                let want8 = oracle(&src, k, false);
                let want16 = oracle(&src, k, true);

                if off + BYTES <= len {
                    let mut cb = src.clone();
                    let mut rb = src.clone();
                    unsafe {
                        let a = c(cb.as_mut_ptr().add(off), cb.as_ptr(), len as u64, k.as_ptr());
                        let b = r(rb.as_mut_ptr().add(off), rb.as_ptr(), len as u64, k.as_ptr());
                        eqi("aliased siphash24 rc", a, b);
                        assert_eq!(a, 0);
                    }
                    eqb(&format!("aliased siphash24(len={len},off={off},key#{ki})"), &cb, &rb);
                    assert_eq!(&cb[off..off + BYTES], &want8[..]);

                    let mut gb = src.clone();
                    let mut grb = src.clone();
                    unsafe {
                        let a = wc(gb.as_mut_ptr().add(off), gb.as_ptr(), len as u64, k.as_ptr());
                        let b = wr(grb.as_mut_ptr().add(off), grb.as_ptr(), len as u64, k.as_ptr());
                        eqi("aliased crypto_shorthash rc", a, b);
                        assert_eq!(a, 0);
                    }
                    eqb("aliased crypto_shorthash", &gb, &grb);
                    eqb("aliased generic == aliased siphash24", &cb, &gb);
                }
                if off + X_BYTES <= len {
                    let mut cb = src.clone();
                    let mut rb = src.clone();
                    unsafe {
                        let a = xc(cb.as_mut_ptr().add(off), cb.as_ptr(), len as u64, k.as_ptr());
                        let b = xr(rb.as_mut_ptr().add(off), rb.as_ptr(), len as u64, k.as_ptr());
                        eqi("aliased siphashx24 rc", a, b);
                        assert_eq!(a, 0);
                    }
                    eqb(&format!("aliased siphashx24(len={len},off={off},key#{ki})"), &cb, &rb);
                    assert_eq!(&cb[off..off + X_BYTES], &want16[..]);
                }
            }
        }
    }
}

#[test]
fn keygen_then_hash_round_trip() {
    // `keygen` output is a usable 16-byte key for all three entry points.
    let (kc, kr) = both::<Keygen>("crypto_shorthash_keygen");
    let (c, r) = both::<Sh>("crypto_shorthash_siphash24");
    let (xc, xr) = both::<Sh>("crypto_shorthash_siphashx24");
    for round in 0..4usize {
        let mut ck = [0u8; KEYBYTES];
        let mut rk = [0u8; KEYBYTES];
        rng_reset();
        unsafe {
            kc(ck.as_mut_ptr());
            kr(rk.as_mut_ptr());
        }
        eqb("keygen", &ck, &rk);
        for len in [0usize, 1, 8, 15, 16, 100, 257] {
            let msg = pattern((round % 4) as u8, len);
            let what = format!("keygen round-trip(round={round},len={len})");
            let d = unsafe { call(&what, &c, &r, BYTES, &msg, len as u64, &ck) };
            assert_eq!(d, oracle(&msg, &ck, false), "{what}");
            let dx = unsafe { call(&what, &xc, &xr, X_BYTES, &msg, len as u64, &ck) };
            assert_eq!(dx, oracle(&msg, &ck, true), "{what}");
        }
    }
}
