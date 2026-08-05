//! Differential tests for the AUTH / MAC / VERIFY family.
//!
//! Covers:
//!   - crypto_verify_16 / 32 / 64  (+ _bytes)
//!   - crypto_auth / crypto_auth_verify (+ _bytes/_keybytes/_primitive)
//!   - crypto_auth_hmacsha256 / 512 / 512256 one-shot + verify
//!   - the hmacsha* _init / _update / _final streaming state APIs
//!   - crypto_onetimeauth / _poly1305 one-shot + verify
//!   - the onetimeauth _init / _update / _final streaming state APIs
//!
//! Every call is issued against BOTH the C `.so` and the Rust `.so` (loaded via
//! `libloading`) and the return code + output bytes are compared byte-for-byte.
//! The C library is ground truth.

#[macro_use]
mod common;
use common::{libs, Rng};

use std::os::raw::{c_char, c_int};

// ---- FFI type aliases for the loaded symbols ----------------------------

// one-shot MAC:  fn(out, in, inlen, k) -> int
type MacFn = unsafe extern "C" fn(*mut u8, *const u8, u64, *const u8) -> c_int;
// verify:        fn(h, in, inlen, k) -> int
type VerifyFn = unsafe extern "C" fn(*const u8, *const u8, u64, *const u8) -> c_int;
// verify pair:   fn(x, y) -> int
type Verify2Fn = unsafe extern "C" fn(*const u8, *const u8) -> c_int;
// size query:    fn() -> size_t
type SizeFn = unsafe extern "C" fn() -> usize;

// hmac streaming
type HmacInitFn = unsafe extern "C" fn(*mut u8, *const u8, usize) -> c_int;
type HmacUpdateFn = unsafe extern "C" fn(*mut u8, *const u8, u64) -> c_int;
type HmacFinalFn = unsafe extern "C" fn(*mut u8, *mut u8) -> c_int;

// onetimeauth streaming (init takes no keylen)
type OtaInitFn = unsafe extern "C" fn(*mut u8, *const u8) -> c_int;
type OtaUpdateFn = unsafe extern "C" fn(*mut u8, *const u8, u64) -> c_int;
type OtaFinalFn = unsafe extern "C" fn(*mut u8, *mut u8) -> c_int;

/// A 16-byte-aligned opaque state buffer big enough for any of the streaming
/// states in this family (poly1305 needs 16-byte alignment).
#[repr(align(16))]
struct AlignedState([u8; 512]);

impl AlignedState {
    fn new() -> Self {
        AlignedState([0u8; 512])
    }
    fn as_mut_ptr(&mut self) -> *mut u8 {
        self.0.as_mut_ptr()
    }
}

/// The set of input lengths we exercise everywhere: empty, tiny, block
/// boundaries for SHA-256(64)/SHA-512(128)/poly1305(16), and larger sizes.
const LENGTHS: &[usize] = &[
    0, 1, 2, 15, 16, 17, 31, 32, 33, 63, 64, 65, 100, 127, 128, 129, 200, 255, 256, 257, 1000, 4096,
    5000,
];

// =========================================================================
// crypto_verify_16 / 32 / 64
// =========================================================================

fn verify_case(name: &[u8], n: usize) {
    let l = libs();
    unsafe {
        let (c, r) = sympair!(l, name, Verify2Fn);
        let mut rng = Rng::new(0xFE_u64.wrapping_add(n as u64));

        // Equal buffers: C returns 0.
        for _ in 0..64 {
            let x = rng.vec(n);
            let y = x.clone();
            let rc = c(x.as_ptr(), y.as_ptr());
            let rr = r(x.as_ptr(), y.as_ptr());
            assert_eq!(rc, rr, "{:?} equal", std::str::from_utf8(name));
            assert_eq!(rc, 0, "{:?} equal should be 0", std::str::from_utf8(name));
        }

        // Fully random pairs (almost surely differ): expect -1.
        for _ in 0..256 {
            let x = rng.vec(n);
            let y = rng.vec(n);
            let rc = c(x.as_ptr(), y.as_ptr());
            let rr = r(x.as_ptr(), y.as_ptr());
            assert_eq!(rc, rr, "{:?} random pair", std::str::from_utf8(name));
        }

        // Single-bit / single-byte differences at every position: expect -1.
        for _ in 0..64 {
            let x = rng.vec(n);
            for pos in 0..n {
                let mut y = x.clone();
                y[pos] ^= 1 << (rng.range(8) as u8);
                let rc = c(x.as_ptr(), y.as_ptr());
                let rr = r(x.as_ptr(), y.as_ptr());
                assert_eq!(
                    rc, rr,
                    "{:?} diff at byte {}",
                    std::str::from_utf8(name),
                    pos
                );
                assert_eq!(rc, -1, "differing input must be -1");
            }
        }
    }
}

#[test]
fn verify_16() {
    verify_case(b"crypto_verify_16", 16);
}
#[test]
fn verify_32() {
    verify_case(b"crypto_verify_32", 32);
}
#[test]
fn verify_64() {
    verify_case(b"crypto_verify_64", 64);
}

#[test]
fn verify_bytes_constants() {
    let l = libs();
    unsafe {
        for (name, want) in [
            (&b"crypto_verify_16_bytes"[..], 16usize),
            (&b"crypto_verify_32_bytes"[..], 32),
            (&b"crypto_verify_64_bytes"[..], 64),
        ] {
            let (c, r) = sympair!(l, name, SizeFn);
            assert_eq!(c(), r(), "{:?}", std::str::from_utf8(name));
            assert_eq!(c(), want);
        }
    }
}

// =========================================================================
// Generic one-shot MAC + verify differential engine
// =========================================================================

/// Runs Phase B (valid) + Phase C (error) for a one-shot MAC + its verify fn.
/// `taglen` = MAC output size, `keylen` = key size expected by the one-shot.
fn mac_oneshot_case(mac_name: &[u8], verify_name: &[u8], taglen: usize, keylen: usize, seed: u64) {
    let l = libs();
    unsafe {
        let (cmac, rmac) = sympair!(l, mac_name, MacFn);
        let (cver, rver) = sympair!(l, verify_name, VerifyFn);
        let mut rng = Rng::new(seed);

        for &len in LENGTHS {
            for _ in 0..12 {
                let msg = rng.vec(len);
                let key = rng.vec(keylen);
                let msg_ptr = if len == 0 { std::ptr::null() } else { msg.as_ptr() };

                // --- one-shot output parity ---
                let mut ctag = vec![0u8; taglen];
                let mut rtag = vec![0u8; taglen];
                let rc = cmac(ctag.as_mut_ptr(), msg_ptr, len as u64, key.as_ptr());
                let rr = rmac(rtag.as_mut_ptr(), msg_ptr, len as u64, key.as_ptr());
                assert_eq!(rc, rr, "{:?} rc @len {}", std::str::from_utf8(mac_name), len);
                assert_eq!(ctag, rtag, "{:?} tag @len {}", std::str::from_utf8(mac_name), len);

                // --- verify with correct tag: expect 0 on both ---
                let vc = cver(ctag.as_ptr(), msg_ptr, len as u64, key.as_ptr());
                let vr = rver(rtag.as_ptr(), msg_ptr, len as u64, key.as_ptr());
                assert_eq!(vc, vr, "{:?} verify good", std::str::from_utf8(verify_name));
                assert_eq!(vc, 0, "good tag must verify == 0");

                // --- Phase C: tampered tag (flip one bit) => -1 on both ---
                let mut bad = ctag.clone();
                let bit = rng.range(taglen * 8);
                bad[bit / 8] ^= 1 << (bit % 8) as u8;
                let vc = cver(bad.as_ptr(), msg_ptr, len as u64, key.as_ptr());
                let vr = rver(bad.as_ptr(), msg_ptr, len as u64, key.as_ptr());
                assert_eq!(vc, vr, "{:?} tampered tag", std::str::from_utf8(verify_name));
                assert_eq!(vc, -1, "tampered tag must be -1");

                // --- Phase C: wrong key => -1 on both ---
                let mut wrong_key = key.clone();
                wrong_key[rng.range(keylen)] ^= 0x80;
                let vc = cver(ctag.as_ptr(), msg_ptr, len as u64, wrong_key.as_ptr());
                let vr = rver(rtag.as_ptr(), msg_ptr, len as u64, wrong_key.as_ptr());
                assert_eq!(vc, vr, "{:?} wrong key", std::str::from_utf8(verify_name));
                assert_eq!(vc, -1, "wrong key must be -1");

                // --- Phase C: truncated message (verify over shorter len) ---
                if len > 0 {
                    let short = (len - 1) as u64;
                    let vc = cver(ctag.as_ptr(), msg_ptr, short, key.as_ptr());
                    let vr = rver(rtag.as_ptr(), msg_ptr, short, key.as_ptr());
                    assert_eq!(vc, vr, "{:?} truncated msg", std::str::from_utf8(verify_name));
                    assert_eq!(vc, -1, "truncated msg must be -1");
                }
            }
        }
    }
}

// =========================================================================
// HMAC-SHA family (256 / 512 / 512256): one-shot + verify + streaming
// =========================================================================

/// Streaming (init/update/final) vs one-shot equivalence + verify, chunked
/// arbitrarily, for a given HMAC variant. keylen for init is varied to exercise
/// the `keylen > 64` rehash branch and the `<= blocksize` normal branch.
fn hmac_stream_case(
    mac_name: &[u8],
    verify_name: &[u8],
    init_name: &[u8],
    update_name: &[u8],
    final_name: &[u8],
    taglen: usize,
    oneshot_keylen: usize,
    seed: u64,
) {
    let l = libs();
    unsafe {
        let (cmac, rmac) = sympair!(l, mac_name, MacFn);
        let (cver, rver) = sympair!(l, verify_name, VerifyFn);
        let (cinit, rinit) = sympair!(l, init_name, HmacInitFn);
        let (cupd, rupd) = sympair!(l, update_name, HmacUpdateFn);
        let (cfin, rfin) = sympair!(l, final_name, HmacFinalFn);
        let mut rng = Rng::new(seed);

        // Streaming init keylens: normal, boundary (64), and > blocksize (rehash).
        let keylens = [0usize, 1, oneshot_keylen, 32, 64, 65, 100, 200];

        for &len in LENGTHS {
            for &klen in &keylens {
                let msg = rng.vec(len);
                let key = rng.vec(klen.max(1));
                let key_ptr = if klen == 0 { std::ptr::null() } else { key.as_ptr() };

                // Split message into random chunks.
                let mut cstate = AlignedState::new();
                let mut rstate = AlignedState::new();
                assert_eq!(
                    cinit(cstate.as_mut_ptr(), key_ptr, klen),
                    rinit(rstate.as_mut_ptr(), key_ptr, klen),
                    "{:?} init rc",
                    std::str::from_utf8(init_name)
                );

                let mut off = 0usize;
                while off < len {
                    let remaining = len - off;
                    let chunk = rng.range(remaining) + 1;
                    let chunk = chunk.min(remaining);
                    let p = msg.as_ptr().add(off);
                    assert_eq!(
                        cupd(cstate.as_mut_ptr(), p, chunk as u64),
                        rupd(rstate.as_mut_ptr(), p, chunk as u64),
                        "{:?} update rc",
                        std::str::from_utf8(update_name)
                    );
                    off += chunk;
                }
                // also feed a zero-length update to make sure it is a no-op on both
                let z = cupd(cstate.as_mut_ptr(), msg.as_ptr(), 0);
                let zr = rupd(rstate.as_mut_ptr(), msg.as_ptr(), 0);
                assert_eq!(z, zr);

                let mut ctag = vec![0u8; taglen];
                let mut rtag = vec![0u8; taglen];
                assert_eq!(
                    cfin(cstate.as_mut_ptr(), ctag.as_mut_ptr()),
                    rfin(rstate.as_mut_ptr(), rtag.as_mut_ptr()),
                    "{:?} final rc",
                    std::str::from_utf8(final_name)
                );
                assert_eq!(
                    ctag,
                    rtag,
                    "{:?} streaming tag @len {} klen {}",
                    std::str::from_utf8(init_name),
                    len,
                    klen
                );

                // For the one-shot key size, streaming (single-chunk key) must
                // equal the one-shot output.
                if klen == oneshot_keylen {
                    let msg_ptr = if len == 0 { std::ptr::null() } else { msg.as_ptr() };
                    let mut c1 = vec![0u8; taglen];
                    let mut r1 = vec![0u8; taglen];
                    cmac(c1.as_mut_ptr(), msg_ptr, len as u64, key.as_ptr());
                    rmac(r1.as_mut_ptr(), msg_ptr, len as u64, key.as_ptr());
                    assert_eq!(c1, ctag, "one-shot vs streaming (C)");
                    assert_eq!(r1, rtag, "one-shot vs streaming (Rust)");

                    // verify good + tampered against the one-shot verify fn
                    let msgp = msg_ptr;
                    assert_eq!(
                        cver(ctag.as_ptr(), msgp, len as u64, key.as_ptr()),
                        rver(rtag.as_ptr(), msgp, len as u64, key.as_ptr())
                    );
                    let mut bad = ctag.clone();
                    bad[0] ^= 1;
                    assert_eq!(
                        cver(bad.as_ptr(), msgp, len as u64, key.as_ptr()),
                        rver(bad.as_ptr(), msgp, len as u64, key.as_ptr())
                    );
                }
            }
        }
    }
}

#[test]
fn auth_hmacsha256_oneshot() {
    mac_oneshot_case(
        b"crypto_auth_hmacsha256",
        b"crypto_auth_hmacsha256_verify",
        32,
        32,
        0x256,
    );
}

#[test]
fn auth_hmacsha256_stream() {
    hmac_stream_case(
        b"crypto_auth_hmacsha256",
        b"crypto_auth_hmacsha256_verify",
        b"crypto_auth_hmacsha256_init",
        b"crypto_auth_hmacsha256_update",
        b"crypto_auth_hmacsha256_final",
        32,
        32,
        0x2560,
    );
}

#[test]
fn auth_hmacsha512_oneshot() {
    mac_oneshot_case(
        b"crypto_auth_hmacsha512",
        b"crypto_auth_hmacsha512_verify",
        64,
        32,
        0x512,
    );
}

#[test]
fn auth_hmacsha512_stream() {
    hmac_stream_case(
        b"crypto_auth_hmacsha512",
        b"crypto_auth_hmacsha512_verify",
        b"crypto_auth_hmacsha512_init",
        b"crypto_auth_hmacsha512_update",
        b"crypto_auth_hmacsha512_final",
        64,
        32,
        0x5120,
    );
}

#[test]
fn auth_hmacsha512256_oneshot() {
    mac_oneshot_case(
        b"crypto_auth_hmacsha512256",
        b"crypto_auth_hmacsha512256_verify",
        32,
        32,
        0x512256,
    );
}

#[test]
fn auth_hmacsha512256_stream() {
    hmac_stream_case(
        b"crypto_auth_hmacsha512256",
        b"crypto_auth_hmacsha512256_verify",
        b"crypto_auth_hmacsha512256_init",
        b"crypto_auth_hmacsha512256_update",
        b"crypto_auth_hmacsha512256_final",
        32,
        32,
        0x5122560,
    );
}

// crypto_auth is the default primitive (hmacsha512256).
#[test]
fn auth_default_oneshot() {
    mac_oneshot_case(b"crypto_auth", b"crypto_auth_verify", 32, 32, 0xA07);
}

// =========================================================================
// crypto_onetimeauth / poly1305: one-shot + verify + streaming
// =========================================================================

/// Poly1305 one-shot + verify differential, plus init/update/final streaming
/// equivalence. Note: the same (key, message) may only be authenticated once
/// per key in real usage, but for differential testing we just compare C vs
/// Rust with identical inputs, which is valid.
fn ota_case(
    mac_name: &[u8],
    verify_name: &[u8],
    init_name: &[u8],
    update_name: &[u8],
    final_name: &[u8],
    seed: u64,
) {
    const TAGLEN: usize = 16;
    const KEYLEN: usize = 32;
    let l = libs();
    unsafe {
        let (cmac, rmac) = sympair!(l, mac_name, MacFn);
        let (cver, rver) = sympair!(l, verify_name, VerifyFn);
        let (cinit, rinit) = sympair!(l, init_name, OtaInitFn);
        let (cupd, rupd) = sympair!(l, update_name, OtaUpdateFn);
        let (cfin, rfin) = sympair!(l, final_name, OtaFinalFn);
        let mut rng = Rng::new(seed);

        for &len in LENGTHS {
            for _ in 0..12 {
                let msg = rng.vec(len);
                let key = rng.vec(KEYLEN);
                let msg_ptr = if len == 0 { std::ptr::null() } else { msg.as_ptr() };

                // one-shot parity
                let mut ctag = [0u8; TAGLEN];
                let mut rtag = [0u8; TAGLEN];
                let rc = cmac(ctag.as_mut_ptr(), msg_ptr, len as u64, key.as_ptr());
                let rr = rmac(rtag.as_mut_ptr(), msg_ptr, len as u64, key.as_ptr());
                assert_eq!(rc, rr, "{:?} rc", std::str::from_utf8(mac_name));
                assert_eq!(ctag, rtag, "{:?} tag @len {}", std::str::from_utf8(mac_name), len);

                // verify good
                assert_eq!(
                    cver(ctag.as_ptr(), msg_ptr, len as u64, key.as_ptr()),
                    rver(rtag.as_ptr(), msg_ptr, len as u64, key.as_ptr())
                );
                assert_eq!(cver(ctag.as_ptr(), msg_ptr, len as u64, key.as_ptr()), 0);

                // Phase C: tampered tag => -1
                let mut bad = ctag;
                let bit = rng.range(TAGLEN * 8);
                bad[bit / 8] ^= 1 << (bit % 8) as u8;
                let vc = cver(bad.as_ptr(), msg_ptr, len as u64, key.as_ptr());
                let vr = rver(bad.as_ptr(), msg_ptr, len as u64, key.as_ptr());
                assert_eq!(vc, vr, "tampered");
                assert_eq!(vc, -1);

                // Phase C: wrong key => -1.
                // NB: poly1305 clamps `r`, so a single-bit key flip can land in
                // a clamped bit and yield an identical tag. Use a fully fresh
                // independent key so a tag collision is cryptographically
                // negligible (~2^-128).
                let wrong = rng.vec(KEYLEN);
                let vc = cver(ctag.as_ptr(), msg_ptr, len as u64, wrong.as_ptr());
                let vr = rver(rtag.as_ptr(), msg_ptr, len as u64, wrong.as_ptr());
                assert_eq!(vc, vr, "wrong key");
                assert_eq!(vc, -1);

                // Phase C: truncated message => -1
                if len > 0 {
                    let vc = cver(ctag.as_ptr(), msg_ptr, (len - 1) as u64, key.as_ptr());
                    let vr = rver(rtag.as_ptr(), msg_ptr, (len - 1) as u64, key.as_ptr());
                    assert_eq!(vc, vr, "truncated");
                    assert_eq!(vc, -1);
                }

                // streaming equivalence (random chunking)
                let mut cstate = AlignedState::new();
                let mut rstate = AlignedState::new();
                assert_eq!(
                    cinit(cstate.as_mut_ptr(), key.as_ptr()),
                    rinit(rstate.as_mut_ptr(), key.as_ptr())
                );
                let mut off = 0usize;
                while off < len {
                    let remaining = len - off;
                    let chunk = (rng.range(remaining) + 1).min(remaining);
                    let p = msg.as_ptr().add(off);
                    assert_eq!(
                        cupd(cstate.as_mut_ptr(), p, chunk as u64),
                        rupd(rstate.as_mut_ptr(), p, chunk as u64)
                    );
                    off += chunk;
                }
                let mut cs = [0u8; TAGLEN];
                let mut rs = [0u8; TAGLEN];
                assert_eq!(
                    cfin(cstate.as_mut_ptr(), cs.as_mut_ptr()),
                    rfin(rstate.as_mut_ptr(), rs.as_mut_ptr())
                );
                assert_eq!(cs, rs, "streaming tag @len {}", len);
                assert_eq!(cs, ctag, "streaming == one-shot (C) @len {}", len);
                assert_eq!(rs, rtag, "streaming == one-shot (Rust) @len {}", len);
            }
        }
    }
}

#[test]
fn onetimeauth_default() {
    ota_case(
        b"crypto_onetimeauth",
        b"crypto_onetimeauth_verify",
        b"crypto_onetimeauth_init",
        b"crypto_onetimeauth_update",
        b"crypto_onetimeauth_final",
        0x1_1E,
    );
}

#[test]
fn onetimeauth_poly1305() {
    ota_case(
        b"crypto_onetimeauth_poly1305",
        b"crypto_onetimeauth_poly1305_verify",
        b"crypto_onetimeauth_poly1305_init",
        b"crypto_onetimeauth_poly1305_update",
        b"crypto_onetimeauth_poly1305_final",
        0x0107_u64,
    );
}

// =========================================================================
// size / primitive constant parity
// =========================================================================

#[test]
fn size_and_primitive_constants() {
    let l = libs();
    unsafe {
        for name in [
            &b"crypto_auth_bytes"[..],
            b"crypto_auth_keybytes",
            b"crypto_auth_hmacsha256_bytes",
            b"crypto_auth_hmacsha256_keybytes",
            b"crypto_auth_hmacsha256_statebytes",
            b"crypto_auth_hmacsha512_bytes",
            b"crypto_auth_hmacsha512_keybytes",
            b"crypto_auth_hmacsha512_statebytes",
            b"crypto_auth_hmacsha512256_bytes",
            b"crypto_auth_hmacsha512256_keybytes",
            b"crypto_auth_hmacsha512256_statebytes",
            b"crypto_onetimeauth_bytes",
            b"crypto_onetimeauth_keybytes",
            b"crypto_onetimeauth_statebytes",
            b"crypto_onetimeauth_poly1305_bytes",
            b"crypto_onetimeauth_poly1305_keybytes",
            b"crypto_onetimeauth_poly1305_statebytes",
        ] {
            let (c, r) = sympair!(l, name, SizeFn);
            assert_eq!(c(), r(), "size mismatch for {:?}", std::str::from_utf8(name));
        }

        for name in [
            &b"crypto_auth_primitive"[..],
            b"crypto_onetimeauth_primitive",
        ] {
            let (c, r) = sympair!(l, name, unsafe extern "C" fn() -> *const c_char);
            let cs = std::ffi::CStr::from_ptr(c());
            let rs = std::ffi::CStr::from_ptr(r());
            assert_eq!(cs, rs, "primitive string {:?}", std::str::from_utf8(name));
        }
    }
}
