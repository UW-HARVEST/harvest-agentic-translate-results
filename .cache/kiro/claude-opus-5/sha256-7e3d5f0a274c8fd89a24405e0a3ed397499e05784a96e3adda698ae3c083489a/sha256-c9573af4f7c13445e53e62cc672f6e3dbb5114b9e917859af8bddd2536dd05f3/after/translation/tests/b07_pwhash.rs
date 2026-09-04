//! Phase B — CONFIGS.md rows 129–141: the crypto_pwhash surface.
//!
//! Differentially tests libsodium's password-hashing primitives (argon2i,
//! argon2id, scrypt) by calling the identically-named export in the C `.so`
//! and the Rust `.so` and comparing return codes and output bytes.
//!
//! Where the raw hash output is deterministic (`crypto_pwhash`,
//! `crypto_pwhash_argon2i`, `crypto_pwhash_argon2id`,
//! `crypto_pwhash_scryptsalsa208sha256`, `_ll`) the two libraries MUST agree
//! byte-for-byte (`eq_bytes`).
//!
//! The `*_str` functions embed a *random* salt, so their output strings cannot
//! be compared directly. Instead we cross-verify: a string produced by C must
//! verify under Rust and vice-versa (both return 0), and a wrong password must
//! be rejected by both with the same return code.
//!
//! RUNTIME: argon2/scrypt are deliberately slow, so we use only the SMALL
//! opslimit/memlimit values from CONFIGS.md and keep 2–3 iterations per config.

mod common;
use common::*;

// ---------------------------------------------------------------------------
// C signatures — see c_src/libsodium/include/sodium/crypto_pwhash*.h
//
//   int crypto_pwhash(unsigned char *out, unsigned long long outlen,
//                     const char *passwd, unsigned long long passwdlen,
//                     const unsigned char *salt,
//                     unsigned long long opslimit, size_t memlimit, int alg)
//   int crypto_pwhash_argon2i / _argon2id: same 8-arg shape
//   int crypto_pwhash_scryptsalsa208sha256(out, outlen, passwd, passwdlen,
//                     salt, opslimit(ull), memlimit(size_t))   — no alg
//   int crypto_pwhash_str(out[STRBYTES], passwd, passwdlen, opslimit, memlimit)
//   int crypto_pwhash_str_alg(out, passwd, passwdlen, opslimit, memlimit, alg)
//   int crypto_pwhash_*_str(out, passwd, passwdlen, opslimit, memlimit)
//   int crypto_pwhash_*_str_verify(str, passwd, passwdlen)
//   int crypto_pwhash_*_str_needs_rehash(str, opslimit, memlimit)
//   int crypto_pwhash_scryptsalsa208sha256_ll(passwd, passwdlen, salt, saltlen,
//                     N(uint64_t), r(uint32_t), p(uint32_t), buf, buflen)
// outlen/opslimit are `unsigned long long` (u64); memlimit/saltlen are size_t.
// ---------------------------------------------------------------------------

// (out, outlen, passwd, passwdlen, salt, opslimit, memlimit, alg)
type PwhashAlg =
    unsafe extern "C" fn(*mut u8, u64, *const u8, u64, *const u8, u64, usize, i32) -> i32;
// scrypt high level: (out, outlen, passwd, passwdlen, salt, opslimit, memlimit)
type PwhashScrypt =
    unsafe extern "C" fn(*mut u8, u64, *const u8, u64, *const u8, u64, usize) -> i32;
// scrypt low level _ll: (passwd, passwdlen, salt, saltlen, N, r, p, buf, buflen)
type PwhashScryptLl =
    unsafe extern "C" fn(*const u8, usize, *const u8, usize, u64, u32, u32, *mut u8, usize) -> i32;

// str builders: (out, passwd, passwdlen, opslimit, memlimit)
type PwhashStr = unsafe extern "C" fn(*mut u8, *const u8, u64, u64, usize) -> i32;
// str_alg builder: (out, passwd, passwdlen, opslimit, memlimit, alg)
type PwhashStrAlg = unsafe extern "C" fn(*mut u8, *const u8, u64, u64, usize, i32) -> i32;
// str_verify: (str, passwd, passwdlen)
type PwhashStrVerify = unsafe extern "C" fn(*const u8, *const u8, u64) -> i32;
// str_needs_rehash: (str, opslimit, memlimit)
type PwhashNeedsRehash = unsafe extern "C" fn(*const u8, u64, usize) -> i32;

type SizeFn = unsafe extern "C" fn() -> usize;
type UllFn = unsafe extern "C" fn() -> u64;
type IntFn = unsafe extern "C" fn() -> i32;
type StrFn = unsafe extern "C" fn() -> *const libc::c_char;

const ALG_ARGON2I13: i32 = 1;
const ALG_ARGON2ID13: i32 = 2;

fn size_of(d: &'static Duo, name: &str) -> usize {
    let (f, _) = d.pair::<SizeFn>(name);
    unsafe { f() }
}
fn ull_of(d: &'static Duo, name: &str) -> u64 {
    let (f, _) = d.pair::<UllFn>(name);
    unsafe { f() }
}

// ===========================================================================
// Rows 129–132: deterministic raw hash. C and Rust MUST agree byte-for-byte.
//
//   outlen   ∈ {16,17,32,64}
//   opslimit ∈ (per row)
//   memlimit ∈ {8192,16384}
//   passwdlen∈ {0,1,32}
// The 16-byte salt is randomized with a fixed seed.
// ===========================================================================
fn run_argon2_raw(seed: u64, symbol: &str, alg: i32, opslimits: &[u64]) {
    let d = duo();
    let mut rng = Rng::new(seed);
    let (cf, rf) = d.pair::<PwhashAlg>(symbol);

    let saltbytes = size_of(d, "crypto_pwhash_argon2i_saltbytes"); // 16
    assert_eq!(saltbytes, 16);

    let outlens = [16u64, 17, 32, 64];
    let memlimits = [8192usize, 16384];
    let passwdlens = [0usize, 1, 32];
    let mut ok_count = 0u32;

    for &outlen in &outlens {
        for &ops in opslimits {
            for &mem in &memlimits {
                for &plen in &passwdlens {
                    // 2 randomized (passwd,salt) pairs per configuration.
                    for _ in 0..2 {
                        let passwd = rng.bytes(plen);
                        let salt = rng.bytes(saltbytes);
                        let mut oc = vec![0u8; outlen as usize];
                        let mut or = vec![0u8; outlen as usize];
                        let (rc, rr) = unsafe {
                            (
                                cf(
                                    oc.as_mut_ptr(),
                                    outlen,
                                    passwd.as_ptr() as *const u8,
                                    plen as u64,
                                    salt.as_ptr(),
                                    ops,
                                    mem,
                                    alg,
                                ),
                                rf(
                                    or.as_mut_ptr(),
                                    outlen,
                                    passwd.as_ptr() as *const u8,
                                    plen as u64,
                                    salt.as_ptr(),
                                    ops,
                                    mem,
                                    alg,
                                ),
                            )
                        };
                        let ctx = format!(
                            "{symbol} alg={alg} outlen={outlen} ops={ops} mem={mem} plen={plen}"
                        );
                        eq_i32(&format!("{ctx} ret"), rc, rr);
                        if rc == 0 {
                            eq_bytes(&format!("{ctx} out"), &oc, &or);
                            ok_count += 1;
                        }
                    }
                }
            }
        }
    }
    assert!(
        ok_count > 0,
        "{symbol}: no successful hash was produced — byte-equality never checked"
    );
}

#[test]
fn pwhash_argon2i13_generic() {
    // Row 129: crypto_pwhash, alg = ARGON2I13(1), opslimit ∈ {3,4}.
    run_argon2_raw(0x9100_01, "crypto_pwhash", ALG_ARGON2I13, &[3, 4]);
}

#[test]
fn pwhash_argon2id13_generic() {
    // Row 130: crypto_pwhash, alg = ARGON2ID13(2), opslimit ∈ {1,2,3}.
    run_argon2_raw(0x9100_02, "crypto_pwhash", ALG_ARGON2ID13, &[1, 2, 3]);
}

#[test]
fn pwhash_argon2i_direct() {
    // Row 131: crypto_pwhash_argon2i direct, alg = 1, opslimit ∈ {3,4}.
    run_argon2_raw(0x9100_03, "crypto_pwhash_argon2i", ALG_ARGON2I13, &[3, 4]);
}

#[test]
fn pwhash_argon2id_direct() {
    // Row 132: crypto_pwhash_argon2id direct, alg = 2, opslimit ∈ {1,2,3}.
    run_argon2_raw(
        0x9100_04,
        "crypto_pwhash_argon2id",
        ALG_ARGON2ID13,
        &[1, 2, 3],
    );
}

// ===========================================================================
// Rows 133/134: argon2i_str / argon2id_str + _str_verify — cross-library.
//   The string embeds a random salt, so we cannot compare strings directly.
//   Instead: produce with C, verify with Rust (== 0) and vice-versa; a WRONG
//   password must be rejected by both with the same return code.
// ===========================================================================
fn run_argon2_str_cross(seed: u64, str_sym: &str, verify_sym: &str, ops: u64, mem: usize) {
    let d = duo();
    let mut rng = Rng::new(seed);
    let (c_str, r_str) = d.pair::<PwhashStr>(str_sym);
    let (c_ver, r_ver) = d.pair::<PwhashStrVerify>(verify_sym);
    let strbytes = size_of(d, "crypto_pwhash_argon2i_strbytes"); // 128

    for iter in 0..2u32 {
        let plen = 8 + (iter as usize);
        let passwd = rng.bytes(plen);
        // A different wrong password of the same length.
        let mut wrong = rng.bytes(plen);
        if wrong == passwd {
            wrong[0] ^= 0xFF;
        }

        // Build a hash string with each library.
        let mut cbuf = vec![0u8; strbytes];
        let mut rbuf = vec![0u8; strbytes];
        let (rc, rr) = unsafe {
            (
                c_str(cbuf.as_mut_ptr(), passwd.as_ptr(), plen as u64, ops, mem),
                r_str(rbuf.as_mut_ptr(), passwd.as_ptr(), plen as u64, ops, mem),
            )
        };
        eq_i32(&format!("{str_sym} build ret iter={iter}"), rc, rr);
        assert_eq!(rc, 0, "{str_sym} build should succeed");

        // Cross-verify: C string under Rust verify, Rust string under C verify.
        let cross1 = unsafe { r_ver(cbuf.as_ptr(), passwd.as_ptr(), plen as u64) };
        let cross2 = unsafe { c_ver(rbuf.as_ptr(), passwd.as_ptr(), plen as u64) };
        assert_eq!(
            cross1, 0,
            "{verify_sym}: Rust must verify C-produced string"
        );
        assert_eq!(
            cross2, 0,
            "{verify_sym}: C must verify Rust-produced string"
        );

        // Self-verify both directions too (sanity).
        let self_c = unsafe { c_ver(cbuf.as_ptr(), passwd.as_ptr(), plen as u64) };
        let self_r = unsafe { r_ver(rbuf.as_ptr(), passwd.as_ptr(), plen as u64) };
        assert_eq!(self_c, 0, "{verify_sym}: C self-verify");
        assert_eq!(self_r, 0, "{verify_sym}: Rust self-verify");

        // Wrong password must be rejected identically by both libraries.
        let wc = unsafe { c_ver(cbuf.as_ptr(), wrong.as_ptr(), plen as u64) };
        let wr = unsafe { r_ver(cbuf.as_ptr(), wrong.as_ptr(), plen as u64) };
        eq_i32(&format!("{verify_sym} wrong-pw ret iter={iter}"), wc, wr);
        assert_ne!(wc, 0, "{verify_sym}: wrong password must be rejected");
    }
}

#[test]
fn pwhash_argon2i_str_cross() {
    // Row 133.
    run_argon2_str_cross(
        0x9100_05,
        "crypto_pwhash_argon2i_str",
        "crypto_pwhash_argon2i_str_verify",
        3,
        8192,
    );
}

#[test]
fn pwhash_argon2id_str_cross() {
    // Row 134.
    run_argon2_str_cross(
        0x9100_06,
        "crypto_pwhash_argon2id_str",
        "crypto_pwhash_argon2id_str_verify",
        2,
        8192,
    );
}

// ===========================================================================
// Row 135: crypto_pwhash_str_alg (alg ∈ {1,2}) + crypto_pwhash_str_verify,
//          cross-library.
// ===========================================================================
#[test]
fn pwhash_str_alg_cross() {
    let d = duo();
    let mut rng = Rng::new(0x9100_07);
    let (c_str, r_str) = d.pair::<PwhashStrAlg>("crypto_pwhash_str_alg");
    let (c_ver, r_ver) = d.pair::<PwhashStrVerify>("crypto_pwhash_str_verify");
    let strbytes = size_of(d, "crypto_pwhash_strbytes"); // 128

    // argon2i needs opslimit >= 3; argon2id needs opslimit >= 1.
    for &(alg, ops, mem) in &[
        (ALG_ARGON2I13, 3u64, 8192usize),
        (ALG_ARGON2ID13, 2u64, 8192usize),
    ] {
        for iter in 0..2u32 {
            let plen = 10 + iter as usize;
            let passwd = rng.bytes(plen);
            let mut wrong = rng.bytes(plen);
            if wrong == passwd {
                wrong[0] ^= 0xFF;
            }

            let mut cbuf = vec![0u8; strbytes];
            let mut rbuf = vec![0u8; strbytes];
            let (rc, rr) = unsafe {
                (
                    c_str(
                        cbuf.as_mut_ptr(),
                        passwd.as_ptr(),
                        plen as u64,
                        ops,
                        mem,
                        alg,
                    ),
                    r_str(
                        rbuf.as_mut_ptr(),
                        passwd.as_ptr(),
                        plen as u64,
                        ops,
                        mem,
                        alg,
                    ),
                )
            };
            eq_i32(&format!("str_alg build ret alg={alg} iter={iter}"), rc, rr);
            assert_eq!(rc, 0, "str_alg build should succeed alg={alg}");

            let cross1 = unsafe { r_ver(cbuf.as_ptr(), passwd.as_ptr(), plen as u64) };
            let cross2 = unsafe { c_ver(rbuf.as_ptr(), passwd.as_ptr(), plen as u64) };
            assert_eq!(cross1, 0, "str_verify: Rust must verify C string alg={alg}");
            assert_eq!(cross2, 0, "str_verify: C must verify Rust string alg={alg}");

            let wc = unsafe { c_ver(cbuf.as_ptr(), wrong.as_ptr(), plen as u64) };
            let wr = unsafe { r_ver(cbuf.as_ptr(), wrong.as_ptr(), plen as u64) };
            eq_i32(
                &format!("str_verify wrong-pw ret alg={alg} iter={iter}"),
                wc,
                wr,
            );
            assert_ne!(wc, 0, "str_verify wrong pw must be rejected alg={alg}");
        }
    }
}

// ===========================================================================
// Row 136: needs_rehash for generic + argon2i + argon2id str prefixes.
//   With SAME / LOWER / HIGHER opslimit & memlimit than the embedded params,
//   the return code must match between C and Rust.
// ===========================================================================
fn run_argon2_needs_rehash(
    seed: u64,
    str_sym: &str,
    rehash_syms: &[&str],
    build_ops: u64,
    build_mem: usize,
) {
    let d = duo();
    let mut rng = Rng::new(seed);
    let (c_str, _r_str) = d.pair::<PwhashStr>(str_sym);
    let strbytes = size_of(d, "crypto_pwhash_strbytes");

    // Build one reference string (C is ground truth) at (build_ops, build_mem).
    let passwd = rng.bytes(12);
    let mut buf = vec![0u8; strbytes];
    let rc = unsafe { c_str(buf.as_mut_ptr(), passwd.as_ptr(), 12, build_ops, build_mem) };
    assert_eq!(rc, 0, "{str_sym} build for needs_rehash");

    // SAME, LOWER, HIGHER for both opslimit and memlimit.
    let cases: &[(u64, usize)] = &[
        (build_ops, build_mem),         // same
        (build_ops - 1, build_mem),     // lower ops
        (build_ops + 1, build_mem),     // higher ops
        (build_ops, build_mem / 2),     // lower mem
        (build_ops, build_mem * 2),     // higher mem
        (build_ops + 1, build_mem * 2), // higher both
        (build_ops - 1, build_mem / 2), // lower both
    ];

    for &sym in rehash_syms {
        let (cf, rf) = d.pair::<PwhashNeedsRehash>(sym);
        for &(ops, mem) in cases {
            let rc = unsafe { cf(buf.as_ptr(), ops, mem) };
            let rr = unsafe { rf(buf.as_ptr(), ops, mem) };
            eq_i32(&format!("{sym} needs_rehash ops={ops} mem={mem}"), rc, rr);
        }
        // A string of the WRONG algorithm family must be handled identically
        // (e.g. an argon2id-specific needs_rehash given an argon2i string
        // returns -1). We reuse the same buffer; return codes must still match.
    }
}

#[test]
fn pwhash_needs_rehash() {
    // Row 136. Reference string built with argon2id (the default) at ops=3,
    // mem=16384 so that both LOWER and HIGHER neighbours are valid.
    run_argon2_needs_rehash(
        0x9100_08,
        "crypto_pwhash_str", // generic (argon2id default) builder
        &[
            "crypto_pwhash_str_needs_rehash",
            "crypto_pwhash_argon2i_str_needs_rehash",
            "crypto_pwhash_argon2id_str_needs_rehash",
        ],
        3,
        16384,
    );
}

// ===========================================================================
// Row 137: crypto_pwhash_scryptsalsa208sha256 (deterministic).
//   outlen ∈ {16,17,32,64} × (opslimit,memlimit) at INTERACTIVE and at the
//   library's own *_opslimit_min()/_memlimit_min().
// ===========================================================================
#[test]
fn pwhash_scrypt_raw() {
    let d = duo();
    let mut rng = Rng::new(0x9100_09);
    let (cf, rf) = d.pair::<PwhashScrypt>("crypto_pwhash_scryptsalsa208sha256");

    let saltbytes = size_of(d, "crypto_pwhash_scryptsalsa208sha256_saltbytes"); // 32
    assert_eq!(saltbytes, 32);

    let ops_min = ull_of(d, "crypto_pwhash_scryptsalsa208sha256_opslimit_min");
    let mem_min = size_of(d, "crypto_pwhash_scryptsalsa208sha256_memlimit_min");
    let ops_int = ull_of(d, "crypto_pwhash_scryptsalsa208sha256_opslimit_interactive");
    let mem_int = size_of(d, "crypto_pwhash_scryptsalsa208sha256_memlimit_interactive");

    let params = [(ops_min, mem_min), (ops_int, mem_int)];
    let outlens = [16u64, 17, 32, 64];
    let mut ok_count = 0u32;

    for &outlen in &outlens {
        for &(ops, mem) in &params {
            // 2 randomized (passwd,salt) pairs per configuration.
            for _ in 0..2 {
                let plen = 16usize;
                let passwd = rng.bytes(plen);
                let salt = rng.bytes(saltbytes);
                let mut oc = vec![0u8; outlen as usize];
                let mut or = vec![0u8; outlen as usize];
                let (rc, rr) = unsafe {
                    (
                        cf(
                            oc.as_mut_ptr(),
                            outlen,
                            passwd.as_ptr(),
                            plen as u64,
                            salt.as_ptr(),
                            ops,
                            mem,
                        ),
                        rf(
                            or.as_mut_ptr(),
                            outlen,
                            passwd.as_ptr(),
                            plen as u64,
                            salt.as_ptr(),
                            ops,
                            mem,
                        ),
                    )
                };
                let ctx = format!("scrypt outlen={outlen} ops={ops} mem={mem}");
                eq_i32(&format!("{ctx} ret"), rc, rr);
                if rc == 0 {
                    eq_bytes(&format!("{ctx} out"), &oc, &or);
                    ok_count += 1;
                }
            }
        }
    }
    assert!(
        ok_count > 0,
        "scrypt: no successful hash produced — byte-equality never checked"
    );
}

// ===========================================================================
// Row 138: crypto_pwhash_scryptsalsa208sha256_ll — LOWEST-LEVEL entry point.
//   N ∈ {2,4,16,1024} × r ∈ {1,2,8} × p ∈ {1,2,4} ×
//   buflen ∈ {1,32,64,100} × saltlen ∈ {0,1,32}   (deterministic)
// ===========================================================================
#[test]
fn pwhash_scrypt_ll() {
    let d = duo();
    let mut rng = Rng::new(0x9100_0A);
    let (cf, rf) = d.pair::<PwhashScryptLl>("crypto_pwhash_scryptsalsa208sha256_ll");

    let ns = [2u64, 4, 16, 1024];
    let rs = [1u32, 2, 8];
    let ps = [1u32, 2, 4];
    let buflens = [1usize, 32, 64, 100];
    let saltlens = [0usize, 1, 32];
    let mut ok_count = 0u32;

    for &n in &ns {
        for &r in &rs {
            for &p in &ps {
                for &buflen in &buflens {
                    for &saltlen in &saltlens {
                        let plen = 8usize;
                        let passwd = rng.bytes(plen);
                        let salt = rng.bytes(saltlen);
                        let mut oc = vec![0u8; buflen];
                        let mut or = vec![0u8; buflen];
                        let (rc, rr) = unsafe {
                            (
                                cf(
                                    passwd.as_ptr(),
                                    plen,
                                    salt.as_ptr(),
                                    saltlen,
                                    n,
                                    r,
                                    p,
                                    oc.as_mut_ptr(),
                                    buflen,
                                ),
                                rf(
                                    passwd.as_ptr(),
                                    plen,
                                    salt.as_ptr(),
                                    saltlen,
                                    n,
                                    r,
                                    p,
                                    or.as_mut_ptr(),
                                    buflen,
                                ),
                            )
                        };
                        let ctx = format!(
                            "scrypt_ll N={n} r={r} p={p} buflen={buflen} saltlen={saltlen}"
                        );
                        eq_i32(&format!("{ctx} ret"), rc, rr);
                        if rc == 0 {
                            eq_bytes(&format!("{ctx} out"), &oc, &or);
                            ok_count += 1;
                        }
                    }
                }
            }
        }
    }
    assert!(
        ok_count > 0,
        "scrypt_ll: no successful hash produced — byte-equality never checked"
    );
}

// ===========================================================================
// Row 139: crypto_pwhash_scryptsalsa208sha256_str + _str_verify — cross-lib.
// ===========================================================================
#[test]
fn pwhash_scrypt_str_cross() {
    let d = duo();
    let mut rng = Rng::new(0x9100_0B);
    let (c_str, r_str) = d.pair::<PwhashStr>("crypto_pwhash_scryptsalsa208sha256_str");
    let (c_ver, r_ver) = d.pair::<PwhashStrVerify>("crypto_pwhash_scryptsalsa208sha256_str_verify");
    let strbytes = size_of(d, "crypto_pwhash_scryptsalsa208sha256_strbytes"); // 102

    let ops = ull_of(d, "crypto_pwhash_scryptsalsa208sha256_opslimit_min");
    let mem = size_of(d, "crypto_pwhash_scryptsalsa208sha256_memlimit_min");

    for iter in 0..2u32 {
        let plen = 9 + iter as usize;
        let passwd = rng.bytes(plen);
        let mut wrong = rng.bytes(plen);
        if wrong == passwd {
            wrong[0] ^= 0xFF;
        }

        let mut cbuf = vec![0u8; strbytes];
        let mut rbuf = vec![0u8; strbytes];
        let (rc, rr) = unsafe {
            (
                c_str(cbuf.as_mut_ptr(), passwd.as_ptr(), plen as u64, ops, mem),
                r_str(rbuf.as_mut_ptr(), passwd.as_ptr(), plen as u64, ops, mem),
            )
        };
        eq_i32(&format!("scrypt_str build ret iter={iter}"), rc, rr);
        assert_eq!(rc, 0, "scrypt_str build should succeed");

        let cross1 = unsafe { r_ver(cbuf.as_ptr(), passwd.as_ptr(), plen as u64) };
        let cross2 = unsafe { c_ver(rbuf.as_ptr(), passwd.as_ptr(), plen as u64) };
        assert_eq!(cross1, 0, "scrypt str_verify: Rust must verify C string");
        assert_eq!(cross2, 0, "scrypt str_verify: C must verify Rust string");

        let wc = unsafe { c_ver(cbuf.as_ptr(), wrong.as_ptr(), plen as u64) };
        let wr = unsafe { r_ver(cbuf.as_ptr(), wrong.as_ptr(), plen as u64) };
        eq_i32(
            &format!("scrypt str_verify wrong-pw ret iter={iter}"),
            wc,
            wr,
        );
        assert_ne!(wc, 0, "scrypt str_verify wrong pw must be rejected");
    }
}

// ===========================================================================
// Row 140: crypto_pwhash_scryptsalsa208sha256_str_needs_rehash —
//          matching and differing params.
// ===========================================================================
#[test]
fn pwhash_scrypt_needs_rehash() {
    let d = duo();
    let mut rng = Rng::new(0x9100_0C);
    let (c_str, _) = d.pair::<PwhashStr>("crypto_pwhash_scryptsalsa208sha256_str");
    let (cf, rf) =
        d.pair::<PwhashNeedsRehash>("crypto_pwhash_scryptsalsa208sha256_str_needs_rehash");
    let strbytes = size_of(d, "crypto_pwhash_scryptsalsa208sha256_strbytes");

    let ops = ull_of(d, "crypto_pwhash_scryptsalsa208sha256_opslimit_interactive");
    let mem = size_of(d, "crypto_pwhash_scryptsalsa208sha256_memlimit_interactive");

    let passwd = rng.bytes(11);
    let mut buf = vec![0u8; strbytes];
    let rc = unsafe { c_str(buf.as_mut_ptr(), passwd.as_ptr(), 11, ops, mem) };
    assert_eq!(rc, 0, "scrypt_str build for needs_rehash");

    let cases: &[(u64, usize)] = &[
        (ops, mem),         // matching
        (ops / 2, mem),     // lower ops
        (ops * 2, mem),     // higher ops
        (ops, mem / 2),     // lower mem
        (ops, mem * 2),     // higher mem
        (ops * 2, mem * 2), // higher both
    ];
    for &(o, m) in cases {
        let rc = unsafe { cf(buf.as_ptr(), o, m) };
        let rr = unsafe { rf(buf.as_ptr(), o, m) };
        eq_i32(&format!("scrypt needs_rehash ops={o} mem={m}"), rc, rr);
    }
}

// ===========================================================================
// Row 141: all pwhash constant accessors — exact values, C == Rust.
// ===========================================================================
#[test]
fn pwhash_constants() {
    let d = duo();

    // int-returning: alg identifiers.
    let int_consts: &[(&str, i32)] = &[
        ("crypto_pwhash_alg_argon2i13", 1),
        ("crypto_pwhash_alg_argon2id13", 2),
        ("crypto_pwhash_alg_default", 2), // ALG_DEFAULT == ARGON2ID13
        ("crypto_pwhash_argon2i_alg_argon2i13", 1),
        ("crypto_pwhash_argon2id_alg_argon2id13", 2),
    ];
    for &(name, expect) in int_consts {
        let (cf, rf) = d.pair::<IntFn>(name);
        let (c, r) = unsafe { (cf(), rf()) };
        eq_i32(&format!("{name} C==Rust"), c, r);
        assert_eq!(c, expect, "{name}: expected exact value {expect}, got {c}");
    }

    // size_t-returning: bytes/passwd/salt/str/memlimit accessors with exact
    // values taken directly from the C headers.
    let size_consts: &[(&str, usize)] = &[
        ("crypto_pwhash_argon2i_bytes_min", 16),
        ("crypto_pwhash_argon2i_saltbytes", 16),
        ("crypto_pwhash_argon2i_strbytes", 128),
        ("crypto_pwhash_argon2i_passwd_min", 0),
        ("crypto_pwhash_argon2i_memlimit_min", 8192),
        ("crypto_pwhash_argon2i_memlimit_interactive", 33554432),
        ("crypto_pwhash_argon2i_memlimit_moderate", 134217728),
        ("crypto_pwhash_argon2i_memlimit_sensitive", 536870912),
        ("crypto_pwhash_argon2id_bytes_min", 16),
        ("crypto_pwhash_argon2id_saltbytes", 16),
        ("crypto_pwhash_argon2id_strbytes", 128),
        ("crypto_pwhash_argon2id_passwd_min", 0),
        ("crypto_pwhash_argon2id_memlimit_min", 8192),
        ("crypto_pwhash_argon2id_memlimit_interactive", 67108864),
        ("crypto_pwhash_argon2id_memlimit_moderate", 268435456),
        ("crypto_pwhash_argon2id_memlimit_sensitive", 1073741824),
        ("crypto_pwhash_bytes_min", 16),
        ("crypto_pwhash_saltbytes", 16),
        ("crypto_pwhash_strbytes", 128),
        ("crypto_pwhash_memlimit_min", 8192),
        ("crypto_pwhash_memlimit_interactive", 67108864),
        ("crypto_pwhash_memlimit_moderate", 268435456),
        ("crypto_pwhash_memlimit_sensitive", 1073741824),
        ("crypto_pwhash_scryptsalsa208sha256_bytes_min", 16),
        ("crypto_pwhash_scryptsalsa208sha256_saltbytes", 32),
        ("crypto_pwhash_scryptsalsa208sha256_strbytes", 102),
        ("crypto_pwhash_scryptsalsa208sha256_passwd_min", 0),
        ("crypto_pwhash_scryptsalsa208sha256_memlimit_min", 16777216),
        (
            "crypto_pwhash_scryptsalsa208sha256_memlimit_interactive",
            16777216,
        ),
        (
            "crypto_pwhash_scryptsalsa208sha256_memlimit_sensitive",
            1073741824,
        ),
    ];
    for &(name, expect) in size_consts {
        let (cf, rf) = d.pair::<SizeFn>(name);
        let (c, r) = unsafe { (cf(), rf()) };
        assert_eq!(c, r, "{name}: C={c} Rust={r}");
        assert_eq!(c, expect, "{name}: expected exact value {expect}, got {c}");
    }

    // *_bytes_max / *_memlimit_max are platform-derived; just require C == Rust.
    for name in [
        "crypto_pwhash_argon2i_bytes_max",
        "crypto_pwhash_argon2i_memlimit_max",
        "crypto_pwhash_argon2i_passwd_max",
        "crypto_pwhash_argon2id_bytes_max",
        "crypto_pwhash_argon2id_memlimit_max",
        "crypto_pwhash_argon2id_passwd_max",
        "crypto_pwhash_bytes_max",
        "crypto_pwhash_memlimit_max",
        "crypto_pwhash_passwd_max",
        "crypto_pwhash_scryptsalsa208sha256_bytes_max",
        "crypto_pwhash_scryptsalsa208sha256_memlimit_max",
        "crypto_pwhash_scryptsalsa208sha256_passwd_max",
    ] {
        let (cf, rf) = d.pair::<SizeFn>(name);
        let (c, r) = unsafe { (cf(), rf()) };
        assert_eq!(c, r, "{name}: C={c} Rust={r}");
    }

    // ull-returning: opslimit accessors with exact values.
    let ull_consts: &[(&str, u64)] = &[
        ("crypto_pwhash_argon2i_opslimit_min", 3),
        ("crypto_pwhash_argon2i_opslimit_interactive", 4),
        ("crypto_pwhash_argon2i_opslimit_moderate", 6),
        ("crypto_pwhash_argon2i_opslimit_sensitive", 8),
        ("crypto_pwhash_argon2i_opslimit_max", 4294967295),
        ("crypto_pwhash_argon2id_opslimit_min", 1),
        ("crypto_pwhash_argon2id_opslimit_interactive", 2),
        ("crypto_pwhash_argon2id_opslimit_moderate", 3),
        ("crypto_pwhash_argon2id_opslimit_sensitive", 4),
        ("crypto_pwhash_argon2id_opslimit_max", 4294967295),
        ("crypto_pwhash_opslimit_min", 1),
        ("crypto_pwhash_opslimit_interactive", 2),
        ("crypto_pwhash_opslimit_moderate", 3),
        ("crypto_pwhash_opslimit_sensitive", 4),
        ("crypto_pwhash_opslimit_max", 4294967295),
        ("crypto_pwhash_scryptsalsa208sha256_opslimit_min", 32768),
        (
            "crypto_pwhash_scryptsalsa208sha256_opslimit_interactive",
            524288,
        ),
        (
            "crypto_pwhash_scryptsalsa208sha256_opslimit_sensitive",
            33554432,
        ),
        (
            "crypto_pwhash_scryptsalsa208sha256_opslimit_max",
            4294967295,
        ),
    ];
    for &(name, expect) in ull_consts {
        let (cf, rf) = d.pair::<UllFn>(name);
        let (c, r) = unsafe { (cf(), rf()) };
        assert_eq!(c, r, "{name}: C={c} Rust={r}");
        assert_eq!(c, expect, "{name}: expected exact value {expect}, got {c}");
    }

    // str-returning: strprefix accessors with exact values.
    let str_consts: &[(&str, &str)] = &[
        ("crypto_pwhash_argon2i_strprefix", "$argon2i$"),
        ("crypto_pwhash_argon2id_strprefix", "$argon2id$"),
        ("crypto_pwhash_strprefix", "$argon2id$"),
        ("crypto_pwhash_scryptsalsa208sha256_strprefix", "$7$"),
    ];
    for &(name, expect) in str_consts {
        let (cf, rf) = d.pair::<StrFn>(name);
        let (cp, rp) = unsafe { (cf(), rf()) };
        let cs = unsafe { std::ffi::CStr::from_ptr(cp) }.to_str().unwrap();
        let rs = unsafe { std::ffi::CStr::from_ptr(rp) }.to_str().unwrap();
        assert_eq!(cs, rs, "{name}: C={cs:?} Rust={rs:?}");
        assert_eq!(cs, expect, "{name}: expected {expect:?}, got {cs:?}");
    }
}
