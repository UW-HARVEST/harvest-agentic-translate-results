//! Differential tests for the PWHASH + KDF family:
//!   - crypto_pwhash (argon2i / argon2id): raw derivation + _str / _str_verify
//!   - crypto_pwhash_scryptsalsa208sha256: ll / raw / str / str_verify
//!   - crypto_kdf (blake2b): derive_from_key, keygen
//!   - crypto_kdf_hkdf_sha256 / sha512: extract (one-shot + init/update/final) + expand
//!
//! The C library is ground truth. Every call goes through the exported symbol
//! loaded from each .so; we compare return codes and output buffers
//! byte-for-byte. Password-hashing params are kept MINIMAL for speed.

#[macro_use]
mod common;
use common::{libs, Rng};

use std::os::raw::{c_char, c_int};

// ---- FFI signature type aliases -------------------------------------------

type PwhashFn = unsafe extern "C" fn(
    *mut u8,          // out
    u64,              // outlen
    *const c_char,    // passwd
    u64,              // passwdlen
    *const u8,        // salt
    u64,              // opslimit
    usize,            // memlimit
    c_int,            // alg
) -> c_int;

type PwhashScryptFn = unsafe extern "C" fn(
    *mut u8,
    u64,
    *const c_char,
    u64,
    *const u8,
    u64,
    usize,
) -> c_int;

type ScryptLlFn = unsafe extern "C" fn(
    *const u8, // passwd
    usize,     // passwdlen
    *const u8, // salt
    usize,     // saltlen
    u64,       // N
    u32,       // r
    u32,       // p
    *mut u8,   // buf
    usize,     // buflen
) -> c_int;

type StrFn = unsafe extern "C" fn(*mut c_char, *const c_char, u64, u64, usize) -> c_int;
type StrAlgFn =
    unsafe extern "C" fn(*mut c_char, *const c_char, u64, u64, usize, c_int) -> c_int;
type StrVerifyFn = unsafe extern "C" fn(*const c_char, *const c_char, u64) -> c_int;

type KdfDeriveFn =
    unsafe extern "C" fn(*mut u8, usize, u64, *const c_char, *const u8) -> c_int;

type HkdfExtractFn =
    unsafe extern "C" fn(*mut u8, *const u8, usize, *const u8, usize) -> c_int;
type HkdfExpandFn =
    unsafe extern "C" fn(*mut u8, usize, *const c_char, usize, *const u8) -> c_int;
type HkdfInitFn = unsafe extern "C" fn(*mut u8, *const u8, usize) -> c_int;
type HkdfUpdateFn = unsafe extern "C" fn(*mut u8, *const u8, usize) -> c_int;
type HkdfFinalFn = unsafe extern "C" fn(*mut u8, *mut u8) -> c_int;
type StateBytesFn = unsafe extern "C" fn() -> usize;

// Argon2 alg identifiers (from headers).
const ALG_ARGON2I13: c_int = 1;
const ALG_ARGON2ID13: c_int = 2;

// Minimal argon2 params (from headers): both memlimit min = 8192.
const ARGON2I_OPS_MIN: u64 = 3;
const ARGON2ID_OPS_MIN: u64 = 1;
const ARGON2_MEM_MIN: usize = 8192;
const ARGON2_SALTBYTES: usize = 16;

// ===========================================================================
// Phase B — valid paths
// ===========================================================================

/// argon2i and argon2id raw derivation across many random passwords/salts and
/// several output lengths, using both the generic `crypto_pwhash` entry point
/// and the algorithm-specific ones. Compares derived keys byte-for-byte.
#[test]
fn argon2_raw_derivation_matches() {
    let l = libs();
    unsafe {
        let (c_generic, r_generic) = sympair!(l, b"crypto_pwhash", PwhashFn);
        let (c_i, r_i) = sympair!(l, b"crypto_pwhash_argon2i", PwhashFn);
        let (c_id, r_id) = sympair!(l, b"crypto_pwhash_argon2id", PwhashFn);

        let mut rng = Rng::new(0x1234_5678);
        // out lengths spanning min(16) up to a modest 40.
        let outlens = [16usize, 24, 32, 40];

        for iter in 0..24 {
            let pwlen = 1 + rng.range(24);
            let passwd = rng.vec(pwlen);
            let salt = rng.vec(ARGON2_SALTBYTES);
            let outlen = outlens[iter % outlens.len()];

            // Cheaply exercise "multiple passes" by bumping opslimit sometimes.
            let ops_i = ARGON2I_OPS_MIN + (iter as u64 % 2); // 3 or 4
            let ops_id = ARGON2ID_OPS_MIN + (iter as u64 % 3); // 1,2,3

            for &(cf, rf, alg, ops) in &[
                (&c_i, &r_i, ALG_ARGON2I13, ops_i),
                (&c_id, &r_id, ALG_ARGON2ID13, ops_id),
                (&c_generic, &r_generic, ALG_ARGON2I13, ops_i),
                (&c_generic, &r_generic, ALG_ARGON2ID13, ops_id),
            ] {
                let mut co = vec![0u8; outlen];
                let mut ro = vec![0u8; outlen];
                let rc = cf(
                    co.as_mut_ptr(),
                    outlen as u64,
                    passwd.as_ptr() as *const c_char,
                    pwlen as u64,
                    salt.as_ptr(),
                    ops,
                    ARGON2_MEM_MIN,
                    alg,
                );
                let rr = rf(
                    ro.as_mut_ptr(),
                    outlen as u64,
                    passwd.as_ptr() as *const c_char,
                    pwlen as u64,
                    salt.as_ptr(),
                    ops,
                    ARGON2_MEM_MIN,
                    alg,
                );
                assert_eq!(rc, rr, "argon2 rc alg={alg} iter={iter}");
                assert_eq!(rc, 0, "argon2 should succeed alg={alg} iter={iter}");
                assert_eq!(co, ro, "argon2 derived key alg={alg} iter={iter}");
            }
        }
    }
}

/// argon2 string hashing + verify. The salt is random inside C, so the C and
/// Rust hash strings differ — but each library must verify its OWN string with
/// the right password (return 0) and reject a wrong password (return -1).
/// We also cross-verify: Rust must accept the C-produced string and vice versa.
#[test]
fn argon2_str_and_verify_matches() {
    let l = libs();
    unsafe {
        let (c_str, r_str) = sympair!(l, b"crypto_pwhash_str", StrFn);
        let (c_str_alg, r_str_alg) = sympair!(l, b"crypto_pwhash_str_alg", StrAlgFn);
        let (c_ver, r_ver) = sympair!(l, b"crypto_pwhash_str_verify", StrVerifyFn);

        let strbytes = 128usize;
        let mut rng = Rng::new(0xABCD_0001);

        for iter in 0..8 {
            let pw_len = 1 + rng.range(20);
            let pw = rng.vec(pw_len);
            let alg = if iter % 2 == 0 { ALG_ARGON2I13 } else { ALG_ARGON2ID13 };
            let ops = if alg == ALG_ARGON2I13 { ARGON2I_OPS_MIN } else { ARGON2ID_OPS_MIN };

            // Produce a hash string with each library.
            let mut cbuf = vec![0i8 as c_char; strbytes];
            let mut rbuf = vec![0i8 as c_char; strbytes];
            let rc = c_str_alg(
                cbuf.as_mut_ptr(),
                pw.as_ptr() as *const c_char,
                pw.len() as u64,
                ops,
                ARGON2_MEM_MIN,
                alg,
            );
            let rr = r_str_alg(
                rbuf.as_mut_ptr(),
                pw.as_ptr() as *const c_char,
                pw.len() as u64,
                ops,
                ARGON2_MEM_MIN,
                alg,
            );
            assert_eq!(rc, rr, "str_alg rc");
            assert_eq!(rc, 0, "str_alg success");

            // Each verifier accepts its own string with correct password.
            assert_eq!(c_ver(cbuf.as_ptr(), pw.as_ptr() as *const c_char, pw.len() as u64), 0);
            assert_eq!(r_ver(rbuf.as_ptr(), pw.as_ptr() as *const c_char, pw.len() as u64), 0);

            // Cross-verify: strings are interoperable through the FFI boundary.
            assert_eq!(
                r_ver(cbuf.as_ptr(), pw.as_ptr() as *const c_char, pw.len() as u64),
                0,
                "Rust must verify C-produced argon2 string"
            );
            assert_eq!(
                c_ver(rbuf.as_ptr(), pw.as_ptr() as *const c_char, pw.len() as u64),
                0,
                "C must verify Rust-produced argon2 string"
            );

            // Wrong password -> -1 on both, both directions.
            let mut wrong = pw.clone();
            wrong.push(0x2a);
            assert_eq!(
                c_ver(cbuf.as_ptr(), wrong.as_ptr() as *const c_char, wrong.len() as u64),
                -1
            );
            assert_eq!(
                r_ver(rbuf.as_ptr(), wrong.as_ptr() as *const c_char, wrong.len() as u64),
                -1
            );
            assert_eq!(
                r_ver(cbuf.as_ptr(), wrong.as_ptr() as *const c_char, wrong.len() as u64),
                -1
            );
        }

        // Also exercise the default crypto_pwhash_str (argon2id).
        let pw = b"default-alg-password";
        let mut cbuf = vec![0i8 as c_char; strbytes];
        let mut rbuf = vec![0i8 as c_char; strbytes];
        assert_eq!(
            c_str(cbuf.as_mut_ptr(), pw.as_ptr() as *const c_char, pw.len() as u64, ARGON2ID_OPS_MIN, ARGON2_MEM_MIN),
            0
        );
        assert_eq!(
            r_str(rbuf.as_mut_ptr(), pw.as_ptr() as *const c_char, pw.len() as u64, ARGON2ID_OPS_MIN, ARGON2_MEM_MIN),
            0
        );
        assert_eq!(r_ver(cbuf.as_ptr(), pw.as_ptr() as *const c_char, pw.len() as u64), 0);
        assert_eq!(c_ver(rbuf.as_ptr(), pw.as_ptr() as *const c_char, pw.len() as u64), 0);
    }
}

/// scrypt low-level `_ll` with small N/r/p over many random inputs.
#[test]
fn scrypt_ll_matches() {
    let l = libs();
    unsafe {
        let (c, r) = sympair!(l, b"crypto_pwhash_scryptsalsa208sha256_ll", ScryptLlFn);
        let mut rng = Rng::new(0x5c12_9a00);

        // (N, r, p) combos — all cheap. N must be a power of two >= 2.
        let params = [(16u64, 1u32, 1u32), (16, 4, 1), (32, 2, 2), (8, 8, 1), (64, 1, 3)];

        for iter in 0..30 {
            let pw_len = rng.range(20);
            let pw = rng.vec(pw_len);
            let salt_len = rng.range(24);
            let salt = rng.vec(salt_len);
            let (n, rr_, p) = params[iter % params.len()];
            let buflen = 16 + rng.range(48);

            let mut cb = vec![0u8; buflen];
            let mut rb = vec![0u8; buflen];
            let rc = c(
                pw.as_ptr(), pw.len(), salt.as_ptr(), salt.len(),
                n, rr_, p, cb.as_mut_ptr(), buflen,
            );
            let rrr = r(
                pw.as_ptr(), pw.len(), salt.as_ptr(), salt.len(),
                n, rr_, p, rb.as_mut_ptr(), buflen,
            );
            assert_eq!(rc, rrr, "scrypt_ll rc iter={iter}");
            assert_eq!(rc, 0, "scrypt_ll should succeed iter={iter}");
            assert_eq!(cb, rb, "scrypt_ll buf iter={iter} N={n} r={rr_} p={p}");
        }
    }
}

/// scrypt raw derivation (`crypto_pwhash_scryptsalsa208sha256`) at the minimum
/// opslimit/memlimit, many random passwords/salts.
#[test]
fn scrypt_raw_matches() {
    let l = libs();
    unsafe {
        let (c, r) = sympair!(l, b"crypto_pwhash_scryptsalsa208sha256", PwhashScryptFn);
        // MIN values from the header.
        let ops_min: u64 = 32768;
        let mem_min: usize = 16777216;
        let saltbytes = 32usize;
        let mut rng = Rng::new(0x5cab_7700);

        let outlens = [16usize, 32, 48];
        for iter in 0..9 {
            let pw_len = 1 + rng.range(20);
            let pw = rng.vec(pw_len);
            let salt = rng.vec(saltbytes);
            let outlen = outlens[iter % outlens.len()];
            let mut cb = vec![0u8; outlen];
            let mut rb = vec![0u8; outlen];
            let rc = c(
                cb.as_mut_ptr(), outlen as u64,
                pw.as_ptr() as *const c_char, pw.len() as u64,
                salt.as_ptr(), ops_min, mem_min,
            );
            let rrr = r(
                rb.as_mut_ptr(), outlen as u64,
                pw.as_ptr() as *const c_char, pw.len() as u64,
                salt.as_ptr(), ops_min, mem_min,
            );
            assert_eq!(rc, rrr, "scrypt raw rc iter={iter}");
            assert_eq!(rc, 0, "scrypt raw success iter={iter}");
            assert_eq!(cb, rb, "scrypt raw buf iter={iter}");
        }
    }
}

/// scrypt string hashing + verify (interoperable across libraries, wrong pw -> -1).
#[test]
fn scrypt_str_and_verify_matches() {
    let l = libs();
    unsafe {
        let (c_str, r_str) =
            sympair!(l, b"crypto_pwhash_scryptsalsa208sha256_str", StrFn);
        let (c_ver, r_ver) =
            sympair!(l, b"crypto_pwhash_scryptsalsa208sha256_str_verify", StrVerifyFn);
        let strbytes = 102usize;
        let ops_min: u64 = 32768;
        let mem_min: usize = 16777216;
        let mut rng = Rng::new(0x5c57_0001);

        for _ in 0..4 {
            let pw_len = 1 + rng.range(18);
            let pw = rng.vec(pw_len);
            let mut cbuf = vec![0i8 as c_char; strbytes];
            let mut rbuf = vec![0i8 as c_char; strbytes];
            assert_eq!(
                c_str(cbuf.as_mut_ptr(), pw.as_ptr() as *const c_char, pw.len() as u64, ops_min, mem_min),
                0
            );
            assert_eq!(
                r_str(rbuf.as_mut_ptr(), pw.as_ptr() as *const c_char, pw.len() as u64, ops_min, mem_min),
                0
            );
            // self + cross verify with correct password.
            assert_eq!(c_ver(cbuf.as_ptr(), pw.as_ptr() as *const c_char, pw.len() as u64), 0);
            assert_eq!(r_ver(rbuf.as_ptr(), pw.as_ptr() as *const c_char, pw.len() as u64), 0);
            assert_eq!(r_ver(cbuf.as_ptr(), pw.as_ptr() as *const c_char, pw.len() as u64), 0);
            assert_eq!(c_ver(rbuf.as_ptr(), pw.as_ptr() as *const c_char, pw.len() as u64), 0);

            // wrong password -> -1
            let mut wrong = pw.clone();
            wrong.push(0x21);
            assert_eq!(c_ver(cbuf.as_ptr(), wrong.as_ptr() as *const c_char, wrong.len() as u64), -1);
            assert_eq!(r_ver(rbuf.as_ptr(), wrong.as_ptr() as *const c_char, wrong.len() as u64), -1);
        }
    }
}

/// crypto_kdf (blake2b) derive_from_key: many subkey ids and lengths, both the
/// generic and blake2b-specific entry points.
#[test]
fn kdf_derive_from_key_matches() {
    let l = libs();
    unsafe {
        let (c, r) = sympair!(l, b"crypto_kdf_derive_from_key", KdfDeriveFn);
        let (cb, rb) = sympair!(l, b"crypto_kdf_blake2b_derive_from_key", KdfDeriveFn);
        let mut rng = Rng::new(0xBEEF_0001);

        let key = rng.vec(32); // crypto_kdf_KEYBYTES
        let ctx = b"testctx0"; // crypto_kdf_CONTEXTBYTES = 8

        // subkey lengths across the full valid range [16, 64].
        let lens = [16usize, 17, 24, 32, 48, 63, 64];
        let ids: [u64; 6] = [0, 1, 2, 42, 0xFFFF_FFFF, 0xFFFF_FFFF_FFFF_FFFF];

        for &len in &lens {
            for &id in &ids {
                for &(cf, rf) in &[(&c, &r), (&cb, &rb)] {
                    let mut co = vec![0u8; len];
                    let mut ro = vec![0u8; len];
                    let rc = cf(co.as_mut_ptr(), len, id, ctx.as_ptr() as *const c_char, key.as_ptr());
                    let rr = rf(ro.as_mut_ptr(), len, id, ctx.as_ptr() as *const c_char, key.as_ptr());
                    assert_eq!(rc, rr, "kdf derive rc len={len} id={id}");
                    assert_eq!(rc, 0);
                    assert_eq!(co, ro, "kdf subkey len={len} id={id}");
                }
            }
        }
    }
}

/// crypto_kdf_keygen produces KEYBYTES without crashing on both libraries.
/// (Output is random, so we only check it does not error and fills the buffer.)
#[test]
fn kdf_keygen_runs() {
    let l = libs();
    unsafe {
        let (c, r) = sympair!(l, b"crypto_kdf_keygen", unsafe extern "C" fn(*mut u8));
        let mut ck = [0u8; 32];
        let mut rk = [0u8; 32];
        c(ck.as_mut_ptr());
        r(rk.as_mut_ptr());
        // Very unlikely to be all-zero after keygen.
        assert!(ck.iter().any(|&b| b != 0));
        assert!(rk.iter().any(|&b| b != 0));
    }
}

/// HKDF-SHA256 and HKDF-SHA512: extract (one-shot) + expand across many random
/// salts/ikm/ctx and output lengths. Compare PRK and expanded output.
#[test]
fn hkdf_extract_expand_matches() {
    let l = libs();
    unsafe {
        for suite in &["sha256", "sha512"] {
            let keybytes = if *suite == "sha256" { 32usize } else { 64 };
            let extract_sym = format!("crypto_kdf_hkdf_{suite}_extract");
            let expand_sym = format!("crypto_kdf_hkdf_{suite}_expand");
            let (c_ex, r_ex) = sympair!(l, extract_sym.as_bytes(), HkdfExtractFn);
            let (c_xp, r_xp) = sympair!(l, expand_sym.as_bytes(), HkdfExpandFn);

            let mut rng = Rng::new(0x4444_0000 + keybytes as u64);
            for iter in 0..16 {
                let salt_len = rng.range(40);
                let salt = rng.vec(salt_len);
                let ikm_len = 1 + rng.range(48);
                let ikm = rng.vec(ikm_len);
                let ctx_len = rng.range(16);
                let ctx = rng.vec(ctx_len);

                let mut c_prk = vec![0u8; keybytes];
                let mut r_prk = vec![0u8; keybytes];
                let rc = c_ex(c_prk.as_mut_ptr(), salt.as_ptr(), salt.len(), ikm.as_ptr(), ikm.len());
                let rr = r_ex(r_prk.as_mut_ptr(), salt.as_ptr(), salt.len(), ikm.as_ptr(), ikm.len());
                assert_eq!(rc, rr, "{suite} extract rc");
                assert_eq!(rc, 0);
                assert_eq!(c_prk, r_prk, "{suite} PRK iter={iter}");

                // expand at several lengths including 0 and > one block.
                let outlen = [0usize, 1, keybytes, keybytes + 5, 3 * keybytes + 7][iter % 5];
                let mut c_out = vec![0u8; outlen];
                let mut r_out = vec![0u8; outlen];
                let rc2 = c_xp(c_out.as_mut_ptr(), outlen, ctx.as_ptr() as *const c_char, ctx.len(), c_prk.as_ptr());
                let rr2 = r_xp(r_out.as_mut_ptr(), outlen, ctx.as_ptr() as *const c_char, ctx.len(), r_prk.as_ptr());
                assert_eq!(rc2, rr2, "{suite} expand rc outlen={outlen}");
                assert_eq!(rc2, 0);
                assert_eq!(c_out, r_out, "{suite} expand out iter={iter} outlen={outlen}");
            }
        }
    }
}

/// HKDF streaming extract via init/update/final must equal one-shot extract,
/// and the C and Rust streaming paths must agree byte-for-byte.
#[test]
fn hkdf_extract_streaming_matches() {
    let l = libs();
    unsafe {
        for suite in &["sha256", "sha512"] {
            let keybytes = if *suite == "sha256" { 32usize } else { 64 };
            let (c_sb, r_sb) = sympair!(
                l,
                format!("crypto_kdf_hkdf_{suite}_statebytes").as_bytes(),
                StateBytesFn
            );
            let cstate_bytes = c_sb();
            let rstate_bytes = r_sb();
            assert_eq!(cstate_bytes, rstate_bytes, "{suite} statebytes");

            let (c_init, r_init) =
                sympair!(l, format!("crypto_kdf_hkdf_{suite}_extract_init").as_bytes(), HkdfInitFn);
            let (c_upd, r_upd) =
                sympair!(l, format!("crypto_kdf_hkdf_{suite}_extract_update").as_bytes(), HkdfUpdateFn);
            let (c_fin, r_fin) =
                sympair!(l, format!("crypto_kdf_hkdf_{suite}_extract_final").as_bytes(), HkdfFinalFn);
            let (c_ex, r_ex) =
                sympair!(l, format!("crypto_kdf_hkdf_{suite}_extract").as_bytes(), HkdfExtractFn);

            let mut rng = Rng::new(0x7777_0000 + keybytes as u64);
            for _ in 0..8 {
                let salt_len = rng.range(32);
                let salt = rng.vec(salt_len);
                // Feed ikm in three random chunks.
                let ikm1_len = 1 + rng.range(20);
                let ikm1 = rng.vec(ikm1_len);
                let ikm2_len = rng.range(20);
                let ikm2 = rng.vec(ikm2_len);
                let ikm3_len = 1 + rng.range(20);
                let ikm3 = rng.vec(ikm3_len);

                let run = |init: &dyn Fn(*mut u8, *const u8, usize) -> c_int,
                           upd: &dyn Fn(*mut u8, *const u8, usize) -> c_int,
                           fin: &dyn Fn(*mut u8, *mut u8) -> c_int|
                 -> Vec<u8> {
                    let mut st = vec![0u8; cstate_bytes];
                    assert_eq!(init(st.as_mut_ptr(), salt.as_ptr(), salt.len()), 0);
                    assert_eq!(upd(st.as_mut_ptr(), ikm1.as_ptr(), ikm1.len()), 0);
                    assert_eq!(upd(st.as_mut_ptr(), ikm2.as_ptr(), ikm2.len()), 0);
                    assert_eq!(upd(st.as_mut_ptr(), ikm3.as_ptr(), ikm3.len()), 0);
                    let mut prk = vec![0u8; keybytes];
                    assert_eq!(fin(st.as_mut_ptr(), prk.as_mut_ptr()), 0);
                    prk
                };

                let c_prk = run(&|s, p, n| c_init(s, p, n), &|s, p, n| c_upd(s, p, n), &|s, p| c_fin(s, p));
                let r_prk = run(&|s, p, n| r_init(s, p, n), &|s, p, n| r_upd(s, p, n), &|s, p| r_fin(s, p));
                assert_eq!(c_prk, r_prk, "{suite} streaming PRK");

                // Streaming must match one-shot over the concatenated ikm.
                let mut all = ikm1.clone();
                all.extend_from_slice(&ikm2);
                all.extend_from_slice(&ikm3);
                let mut c_one = vec![0u8; keybytes];
                let mut r_one = vec![0u8; keybytes];
                c_ex(c_one.as_mut_ptr(), salt.as_ptr(), salt.len(), all.as_ptr(), all.len());
                r_ex(r_one.as_mut_ptr(), salt.as_ptr(), salt.len(), all.as_ptr(), all.len());
                assert_eq!(c_prk, c_one, "{suite} C streaming == one-shot");
                assert_eq!(r_prk, r_one, "{suite} Rust streaming == one-shot");
            }
        }
    }
}

// ===========================================================================
// Phase C — error paths (both libraries must return the same sentinel)
// ===========================================================================

/// argon2 raw: opslimit / memlimit below min, outlen out of range, bad alg id.
#[test]
fn argon2_raw_errors_match() {
    let l = libs();
    unsafe {
        let (c, r) = sympair!(l, b"crypto_pwhash", PwhashFn);
        let pw = b"password";
        let salt = [0u8; ARGON2_SALTBYTES];

        // Helper to call both and assert same rc.
        let mut check = |outlen: u64, ops: u64, mem: usize, alg: c_int, label: &str| {
            let mut co = vec![0u8; outlen.max(1) as usize];
            let mut ro = vec![0u8; outlen.max(1) as usize];
            let rc = c(co.as_mut_ptr(), outlen, pw.as_ptr() as *const c_char, pw.len() as u64, salt.as_ptr(), ops, mem, alg);
            let rr = r(ro.as_mut_ptr(), outlen, pw.as_ptr() as *const c_char, pw.len() as u64, salt.as_ptr(), ops, mem, alg);
            assert_eq!(rc, rr, "{label}: rc mismatch (C={rc}, Rust={rr})");
            assert_eq!(rc, -1, "{label}: expected -1");
        };

        // opslimit below argon2id min (0 < 1)
        check(32, 0, ARGON2_MEM_MIN, ALG_ARGON2ID13, "opslimit 0 argon2id");
        // opslimit below argon2i min (2 < 3)
        check(32, 2, ARGON2_MEM_MIN, ALG_ARGON2I13, "opslimit 2 argon2i");
        // memlimit below min (8191 < 8192)
        check(32, ARGON2ID_OPS_MIN, ARGON2_MEM_MIN - 1, ALG_ARGON2ID13, "memlimit below min");
        // outlen below BYTES_MIN (15 < 16)
        check(15, ARGON2ID_OPS_MIN, ARGON2_MEM_MIN, ALG_ARGON2ID13, "outlen below min");
        // bad alg id
        check(32, ARGON2ID_OPS_MIN, ARGON2_MEM_MIN, 99, "bad alg id");
        check(32, ARGON2ID_OPS_MIN, ARGON2_MEM_MIN, 0, "alg id 0");
    }
}

/// argon2 str_verify: wrong password and invalid/garbage hash strings -> -1.
#[test]
fn argon2_str_verify_errors_match() {
    let l = libs();
    unsafe {
        let (c_ver, r_ver) = sympair!(l, b"crypto_pwhash_str_verify", StrVerifyFn);

        // A garbage / invalid hash string (no valid prefix) must be rejected.
        let bad = b"not-a-valid-hash-string\0";
        let pw = b"whatever";
        assert_eq!(c_ver(bad.as_ptr() as *const c_char, pw.as_ptr() as *const c_char, pw.len() as u64), -1);
        assert_eq!(r_ver(bad.as_ptr() as *const c_char, pw.as_ptr() as *const c_char, pw.len() as u64), -1);

        // Right prefix but corrupt body.
        let bad2 = b"$argon2id$v=19$m=8,t=1,p=1$aaaa$bbbb\0";
        assert_eq!(c_ver(bad2.as_ptr() as *const c_char, pw.as_ptr() as *const c_char, pw.len() as u64), -1);
        assert_eq!(r_ver(bad2.as_ptr() as *const c_char, pw.as_ptr() as *const c_char, pw.len() as u64), -1);

        // Empty string.
        let empty = b"\0";
        assert_eq!(c_ver(empty.as_ptr() as *const c_char, pw.as_ptr() as *const c_char, pw.len() as u64), -1);
        assert_eq!(r_ver(empty.as_ptr() as *const c_char, pw.as_ptr() as *const c_char, pw.len() as u64), -1);
    }
}

/// scrypt _ll invalid parameters: N not a power of two, N < 2, r == 0, p == 0.
#[test]
fn scrypt_ll_errors_match() {
    let l = libs();
    unsafe {
        let (c, r) = sympair!(l, b"crypto_pwhash_scryptsalsa208sha256_ll", ScryptLlFn);
        let pw = b"pw";
        let salt = b"salt";

        let mut check = |n: u64, rr_: u32, p: u32, label: &str| {
            let mut cb = [0u8; 32];
            let mut rb = [0u8; 32];
            let rc = c(pw.as_ptr(), pw.len(), salt.as_ptr(), salt.len(), n, rr_, p, cb.as_mut_ptr(), 32);
            let rrc = r(pw.as_ptr(), pw.len(), salt.as_ptr(), salt.len(), n, rr_, p, rb.as_mut_ptr(), 32);
            assert_eq!(rc, rrc, "{label}: rc mismatch (C={rc}, Rust={rrc})");
            assert_eq!(rc, -1, "{label}: expected -1");
        };

        check(3, 1, 1, "N not power of two");
        check(1, 1, 1, "N < 2");
        check(16, 0, 1, "r == 0");
        check(16, 1, 0, "p == 0");
    }
}

/// scrypt str_verify: wrong password and invalid hash string -> -1.
#[test]
fn scrypt_str_verify_errors_match() {
    let l = libs();
    unsafe {
        let (c_str, r_str) =
            sympair!(l, b"crypto_pwhash_scryptsalsa208sha256_str", StrFn);
        let (c_ver, r_ver) =
            sympair!(l, b"crypto_pwhash_scryptsalsa208sha256_str_verify", StrVerifyFn);
        let strbytes = 102usize;
        let ops_min: u64 = 32768;
        let mem_min: usize = 16777216;

        let pw = b"correct horse";
        let mut cbuf = vec![0i8 as c_char; strbytes];
        assert_eq!(
            c_str(cbuf.as_mut_ptr(), pw.as_ptr() as *const c_char, pw.len() as u64, ops_min, mem_min),
            0
        );
        let _ = r_str; // keep symbol referenced

        // wrong password
        let wrong = b"incorrect horse";
        assert_eq!(c_ver(cbuf.as_ptr(), wrong.as_ptr() as *const c_char, wrong.len() as u64), -1);
        assert_eq!(r_ver(cbuf.as_ptr(), wrong.as_ptr() as *const c_char, wrong.len() as u64), -1);

        // invalid hash string (wrong length / garbage)
        let bad = b"$7$garbage\0";
        assert_eq!(c_ver(bad.as_ptr() as *const c_char, pw.as_ptr() as *const c_char, pw.len() as u64), -1);
        assert_eq!(r_ver(bad.as_ptr() as *const c_char, pw.as_ptr() as *const c_char, pw.len() as u64), -1);
    }
}

/// crypto_kdf derive_from_key: subkey_len below BYTES_MIN and above BYTES_MAX -> -1.
#[test]
fn kdf_derive_errors_match() {
    let l = libs();
    unsafe {
        let (c, r) = sympair!(l, b"crypto_kdf_derive_from_key", KdfDeriveFn);
        let key = [7u8; 32];
        let ctx = b"ctx_pad0";

        let mut check = |len: usize, label: &str| {
            let mut co = vec![0u8; len.max(1)];
            let mut ro = vec![0u8; len.max(1)];
            let rc = c(co.as_mut_ptr(), len, 0, ctx.as_ptr() as *const c_char, key.as_ptr());
            let rr = r(ro.as_mut_ptr(), len, 0, ctx.as_ptr() as *const c_char, key.as_ptr());
            assert_eq!(rc, rr, "{label}: rc mismatch (C={rc}, Rust={rr})");
            assert_eq!(rc, -1, "{label}: expected -1");
        };

        check(15, "subkey_len below BYTES_MIN(16)");
        check(65, "subkey_len above BYTES_MAX(64)");
        check(0, "subkey_len 0");
    }
}

/// HKDF expand: out_len above BYTES_MAX (0xff * KEYBYTES) -> -1 on both.
#[test]
fn hkdf_expand_errors_match() {
    let l = libs();
    unsafe {
        for suite in &["sha256", "sha512"] {
            let keybytes = if *suite == "sha256" { 32usize } else { 64 };
            let max = 0xff * keybytes;
            let (c_xp, r_xp) = sympair!(
                l,
                format!("crypto_kdf_hkdf_{suite}_expand").as_bytes(),
                HkdfExpandFn
            );
            let prk = vec![0x11u8; keybytes];
            let ctx = b"c";
            let outlen = max + 1;
            // We only need the return code; allocate a small buffer since C
            // rejects before writing (checks out_len first).
            let mut co = vec![0u8; 1];
            let mut ro = vec![0u8; 1];
            let rc = c_xp(co.as_mut_ptr(), outlen, ctx.as_ptr() as *const c_char, ctx.len(), prk.as_ptr());
            let rr = r_xp(ro.as_mut_ptr(), outlen, ctx.as_ptr() as *const c_char, ctx.len(), prk.as_ptr());
            assert_eq!(rc, rr, "{suite} expand out_len too big rc mismatch");
            assert_eq!(rc, -1, "{suite} expand out_len too big expected -1");
        }
    }
}
