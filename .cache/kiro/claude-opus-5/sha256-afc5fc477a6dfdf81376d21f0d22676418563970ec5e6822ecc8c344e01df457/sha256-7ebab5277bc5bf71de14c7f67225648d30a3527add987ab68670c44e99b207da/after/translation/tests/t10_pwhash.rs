//! Password hashing: Argon2i / Argon2id (raw + string form) and
//! scrypt-salsa208-sha256 (raw, string form, and the low-level `_ll` API).
//!
//! Argon2/scrypt are intentionally expensive, so the parameter sweeps here use
//! the documented minimum ops/mem limits plus a few larger points rather than
//! the "interactive"/"moderate"/"sensitive" presets.
mod common;

use common::*;
use std::os::raw::{c_char, c_int, c_uchar, c_ulonglong};

type FnUll = unsafe extern "C" fn() -> c_ulonglong;
type FnPwhash = unsafe extern "C" fn(
    *mut c_uchar,      // out
    c_ulonglong,       // outlen
    *const c_char,     // passwd
    c_ulonglong,       // passwdlen
    *const c_uchar,    // salt
    c_ulonglong,       // opslimit
    usize,             // memlimit
    c_int,             // alg
) -> c_int;
type FnPwhashAlgless = unsafe extern "C" fn(
    *mut c_uchar,
    c_ulonglong,
    *const c_char,
    c_ulonglong,
    *const c_uchar,
    c_ulonglong,
    usize,
) -> c_int;
type FnStr = unsafe extern "C" fn(
    *mut c_char,
    *const c_char,
    c_ulonglong,
    c_ulonglong,
    usize,
) -> c_int;
type FnStrAlg = unsafe extern "C" fn(
    *mut c_char,
    *const c_char,
    c_ulonglong,
    c_ulonglong,
    usize,
    c_int,
) -> c_int;
type FnStrVerify = unsafe extern "C" fn(*const c_char, *const c_char, c_ulonglong) -> c_int;
type FnNeedsRehash = unsafe extern "C" fn(*const c_char, c_ulonglong, usize) -> c_int;

fn cmp_ull(name: &str) {
    unsafe {
        let (c, r): (FnUll, FnUll) = pair(name);
        assert_eq!(c(), r(), "{name}() differs");
    }
}

fn pwhash_limits(prefix: &str, has_moderate: bool) {
    for s in ["bytes_min", "bytes_max", "saltbytes", "strbytes"] {
        cmp_size(&format!("{prefix}_{s}"));
    }
    for s in ["passwd_min", "passwd_max"] {
        cmp_size(&format!("{prefix}_{s}"));
    }
    for s in [
        "memlimit_min",
        "memlimit_max",
        "memlimit_interactive",
        "memlimit_sensitive",
    ] {
        cmp_size(&format!("{prefix}_{s}"));
    }
    for s in [
        "opslimit_min",
        "opslimit_max",
        "opslimit_interactive",
        "opslimit_sensitive",
    ] {
        cmp_ull(&format!("{prefix}_{s}"));
    }
    if has_moderate {
        cmp_size(&format!("{prefix}_memlimit_moderate"));
        cmp_ull(&format!("{prefix}_opslimit_moderate"));
    }
    cmp_cstr(&format!("{prefix}_strprefix"));
}

/// Raw-hash + string-form comparison for one Argon2 flavour.
fn argon2_suite(prefix: &str, alg: c_int) {
    pwhash_limits(prefix, true);
    unsafe {
        cmp_int(&format!(
            "{prefix}_alg_{}",
            if alg == 1 { "argon2i13" } else { "argon2id13" }
        ));
        let gs = |s: &str| -> usize {
            let (c, _): (FnSize, FnSize) = pair(&format!("{prefix}_{s}"));
            c()
        };
        let gu = |s: &str| -> u64 {
            let (c, _): (FnUll, FnUll) = pair(&format!("{prefix}_{s}"));
            c()
        };
        let saltb = gs("saltbytes");
        let strb = gs("strbytes");
        let outmin = gs("bytes_min");
        let outmax = gs("bytes_max");
        let memmin = gs("memlimit_min");
        let opsmin = gu("opslimit_min");

        let (craw, rraw): (FnPwhashAlgless, FnPwhashAlgless) = pair(prefix);
        let (cstr, rstr): (FnStr, FnStr) = pair(&format!("{prefix}_str"));
        let (cver, rver): (FnStrVerify, FnStrVerify) = pair(&format!("{prefix}_str_verify"));
        let (cnr, rnr): (FnNeedsRehash, FnNeedsRehash) =
            pair(&format!("{prefix}_str_needs_rehash"));

        let mut rng = Rng::new(0x8000 + prefix.len() as u64);
        let mut salts: Vec<Vec<u8>> = vec![vec![0u8; saltb], vec![0xffu8; saltb]];
        salts.push(rng.vec(saltb));
        let passwords: Vec<Vec<u8>> = vec![
            b"".to_vec(),
            b"a".to_vec(),
            b"password".to_vec(),
            b"correct horse battery staple".to_vec(),
            (0..200u8).collect(),
            vec![0u8; 5],
        ];

        // (opslimit, memlimit) pairs: minimum, and a couple of larger points
        let params: Vec<(u64, usize)> = vec![
            (opsmin, memmin),
            (opsmin + 1, memmin),
            (opsmin, memmin * 2),
            (3, 1 << 16),
            (2, 1 << 18),
            // out-of-range values must be rejected identically
            (0, memmin),
            (opsmin, memmin - 1),
            (opsmin, 0),
        ];

        for salt in &salts {
            for pw in &passwords {
                for &(ops, mem) in &params {
                    for &outlen in &[outmin, outmin + 1, 32, 64, outmin - 1, 0] {
                        let cap = outlen.max(outmax.min(200)) + 8;
                        let mut co = vec![0xAAu8; cap];
                        let mut ro = vec![0xAAu8; cap];
                        let a = craw(
                            co.as_mut_ptr(),
                            outlen as c_ulonglong,
                            pw.as_ptr() as *const c_char,
                            pw.len() as c_ulonglong,
                            salt.as_ptr(),
                            ops,
                            mem,
                        );
                        let b = rraw(
                            ro.as_mut_ptr(),
                            outlen as c_ulonglong,
                            pw.as_ptr() as *const c_char,
                            pw.len() as c_ulonglong,
                            salt.as_ptr(),
                            ops,
                            mem,
                        );
                        let tag = format!(
                            "{prefix}(outlen={outlen},pwlen={},ops={ops},mem={mem})",
                            pw.len()
                        );
                        assert_eq!(a, b, "{tag} return");
                        assert_bytes_eq(&tag, &co, &ro);
                    }
                }
            }
        }

        // string form: identical only because it draws its salt from the
        // (shared, deterministic) RNG, so reset before each side.
        for pw in &passwords {
            for &(ops, mem) in &params {
                let mut cs = vec![0xAAu8; strb + 8];
                let mut rs = vec![0xAAu8; strb + 8];
                det_reset();
                let a = cstr(
                    cs.as_mut_ptr() as *mut c_char,
                    pw.as_ptr() as *const c_char,
                    pw.len() as c_ulonglong,
                    ops,
                    mem,
                );
                det_reset();
                let b = rstr(
                    rs.as_mut_ptr() as *mut c_char,
                    pw.as_ptr() as *const c_char,
                    pw.len() as c_ulonglong,
                    ops,
                    mem,
                );
                let tag = format!("{prefix}_str(pwlen={},ops={ops},mem={mem})", pw.len());
                assert_eq!(a, b, "{tag} return");
                assert_bytes_eq(&tag, &cs, &rs);
                if a != 0 {
                    continue;
                }

                // verify: right password, wrong password, corrupted strings
                let a = cver(
                    cs.as_ptr() as *const c_char,
                    pw.as_ptr() as *const c_char,
                    pw.len() as c_ulonglong,
                );
                let b = rver(
                    cs.as_ptr() as *const c_char,
                    pw.as_ptr() as *const c_char,
                    pw.len() as c_ulonglong,
                );
                assert_eq!(a, b, "{prefix}_str_verify good return");
                assert_eq!(a, 0, "{prefix}_str_verify should succeed");

                let wrong = b"not-the-password";
                let a = cver(
                    cs.as_ptr() as *const c_char,
                    wrong.as_ptr() as *const c_char,
                    wrong.len() as c_ulonglong,
                );
                let b = rver(
                    cs.as_ptr() as *const c_char,
                    wrong.as_ptr() as *const c_char,
                    wrong.len() as c_ulonglong,
                );
                assert_eq!(a, b, "{prefix}_str_verify wrong-pw return");

                for corrupt in corrupt_strings(&cs) {
                    let a = cver(
                        corrupt.as_ptr() as *const c_char,
                        pw.as_ptr() as *const c_char,
                        pw.len() as c_ulonglong,
                    );
                    let b = rver(
                        corrupt.as_ptr() as *const c_char,
                        pw.as_ptr() as *const c_char,
                        pw.len() as c_ulonglong,
                    );
                    assert_eq!(
                        a,
                        b,
                        "{prefix}_str_verify corrupt({:?}) return",
                        String::from_utf8_lossy(&corrupt[..corrupt.len().min(64)])
                    );
                }

                // needs_rehash across parameter changes
                for &(ops2, mem2) in &params {
                    let a = cnr(cs.as_ptr() as *const c_char, ops2, mem2);
                    let b = rnr(cs.as_ptr() as *const c_char, ops2, mem2);
                    assert_eq!(
                        a, b,
                        "{prefix}_str_needs_rehash(ops={ops2},mem={mem2}) return"
                    );
                }
                for corrupt in corrupt_strings(&cs) {
                    let a = cnr(corrupt.as_ptr() as *const c_char, ops, mem);
                    let b = rnr(corrupt.as_ptr() as *const c_char, ops, mem);
                    assert_eq!(a, b, "{prefix}_str_needs_rehash corrupt return");
                }
            }
        }
    }
}

/// NUL-terminated mutations of a stored password hash string.
fn corrupt_strings(s: &[u8]) -> Vec<Vec<u8>> {
    let n = s.iter().position(|&c| c == 0).unwrap_or(s.len());
    let base = &s[..n];
    let mut out: Vec<Vec<u8>> = Vec::new();
    let mk = |v: Vec<u8>| -> Vec<u8> {
        let mut v = v;
        v.push(0);
        v
    };
    out.push(mk(Vec::new()));
    out.push(mk(b"$".to_vec()));
    out.push(mk(b"not-a-hash".to_vec()));
    out.push(mk(b"$argon2id$".to_vec()));
    out.push(mk(b"$argon2xx$v=19$m=8,t=1,p=1$aaaa$bbbb".to_vec()));
    if n > 0 {
        // flip a character in the middle (usually the encoded hash)
        let mut v = base.to_vec();
        v[n / 2] = if v[n / 2] == b'A' { b'B' } else { b'A' };
        out.push(mk(v));
        // truncate
        out.push(mk(base[..n / 2].to_vec()));
        out.push(mk(base[..n - 1].to_vec()));
        // extra trailing garbage
        let mut v = base.to_vec();
        v.extend_from_slice(b"XYZ");
        out.push(mk(v));
        // corrupt the algorithm identifier
        let mut v = base.to_vec();
        v[1] = b'Z';
        out.push(mk(v));
    }
    out
}

#[test]
fn crypto_pwhash_argon2i_matches() {
    argon2_suite("crypto_pwhash_argon2i", 1);
}

#[test]
fn crypto_pwhash_argon2id_matches() {
    argon2_suite("crypto_pwhash_argon2id", 2);
}

#[test]
fn crypto_pwhash_generic_matches() {
    pwhash_limits("crypto_pwhash", true);
    cmp_cstr("crypto_pwhash_primitive");
    cmp_int("crypto_pwhash_alg_argon2i13");
    cmp_int("crypto_pwhash_alg_argon2id13");
    cmp_int("crypto_pwhash_alg_default");
    unsafe {
        let gs = |s: &str| -> usize {
            let (c, _): (FnSize, FnSize) = pair(&format!("crypto_pwhash_{s}"));
            c()
        };
        let gu = |s: &str| -> u64 {
            let (c, _): (FnUll, FnUll) = pair(&format!("crypto_pwhash_{s}"));
            c()
        };
        let saltb = gs("saltbytes");
        let strb = gs("strbytes");
        let outmin = gs("bytes_min");
        let memmin = gs("memlimit_min");
        let opsmin = gu("opslimit_min");

        let (c1, r1): (FnPwhash, FnPwhash) = pair("crypto_pwhash");
        let (cstr, rstr): (FnStr, FnStr) = pair("crypto_pwhash_str");
        let (csa, rsa): (FnStrAlg, FnStrAlg) = pair("crypto_pwhash_str_alg");
        let (cver, rver): (FnStrVerify, FnStrVerify) = pair("crypto_pwhash_str_verify");
        let (cnr, rnr): (FnNeedsRehash, FnNeedsRehash) = pair("crypto_pwhash_str_needs_rehash");

        let mut rng = Rng::new(0x8100);
        let salt = rng.vec(saltb);
        let salt0 = vec![0u8; saltb];
        let passwords: Vec<Vec<u8>> = vec![
            b"".to_vec(),
            b"pw".to_vec(),
            b"a longer password with spaces".to_vec(),
            (0..100u8).collect(),
        ];
        // alg identifiers: valid ones plus invalid values that must be rejected
        let algs: Vec<c_int> = vec![-1, 0, 1, 2, 3, 100];
        let params: Vec<(u64, usize)> = vec![
            (opsmin, memmin),
            (opsmin + 1, memmin * 2),
            (3, 1 << 16),
            (0, memmin),
            (opsmin, memmin - 1),
        ];

        for s in [&salt, &salt0] {
            for pw in &passwords {
                for &alg in &algs {
                    for &(ops, mem) in &params {
                        for &outlen in &[outmin, 32, 64, outmin - 1] {
                            let mut co = vec![0xAAu8; outlen.max(64) + 8];
                            let mut ro = vec![0xAAu8; outlen.max(64) + 8];
                            let a = c1(
                                co.as_mut_ptr(),
                                outlen as c_ulonglong,
                                pw.as_ptr() as *const c_char,
                                pw.len() as c_ulonglong,
                                s.as_ptr(),
                                ops,
                                mem,
                                alg,
                            );
                            let b = r1(
                                ro.as_mut_ptr(),
                                outlen as c_ulonglong,
                                pw.as_ptr() as *const c_char,
                                pw.len() as c_ulonglong,
                                s.as_ptr(),
                                ops,
                                mem,
                                alg,
                            );
                            let tag = format!(
                                "crypto_pwhash(outlen={outlen},pwlen={},ops={ops},mem={mem},alg={alg})",
                                pw.len()
                            );
                            assert_eq!(a, b, "{tag} return");
                            assert_bytes_eq(&tag, &co, &ro);
                        }
                    }
                }
            }
        }

        for pw in &passwords {
            for &(ops, mem) in &params {
                let mut cs = vec![0xAAu8; strb + 8];
                let mut rs = vec![0xAAu8; strb + 8];
                det_reset();
                let a = cstr(
                    cs.as_mut_ptr() as *mut c_char,
                    pw.as_ptr() as *const c_char,
                    pw.len() as c_ulonglong,
                    ops,
                    mem,
                );
                det_reset();
                let b = rstr(
                    rs.as_mut_ptr() as *mut c_char,
                    pw.as_ptr() as *const c_char,
                    pw.len() as c_ulonglong,
                    ops,
                    mem,
                );
                assert_eq!(a, b, "crypto_pwhash_str return");
                assert_bytes_eq("crypto_pwhash_str", &cs, &rs);

                // Only ARGON2I13/ARGON2ID13 are accepted here: any other alg
                // makes crypto_pwhash_str_alg call sodium_misuse() (abort),
                // which is covered by the misuse-parity test instead.
                for &alg in &[1 as c_int, 2] {
                    let mut cs2 = vec![0xAAu8; strb + 8];
                    let mut rs2 = vec![0xAAu8; strb + 8];
                    det_reset();
                    let a = csa(
                        cs2.as_mut_ptr() as *mut c_char,
                        pw.as_ptr() as *const c_char,
                        pw.len() as c_ulonglong,
                        ops,
                        mem,
                        alg,
                    );
                    det_reset();
                    let b = rsa(
                        rs2.as_mut_ptr() as *mut c_char,
                        pw.as_ptr() as *const c_char,
                        pw.len() as c_ulonglong,
                        ops,
                        mem,
                        alg,
                    );
                    let tag = format!("crypto_pwhash_str_alg(ops={ops},mem={mem},alg={alg})");
                    assert_eq!(a, b, "{tag} return");
                    assert_bytes_eq(&tag, &cs2, &rs2);
                    if a != 0 {
                        continue;
                    }
                    let a = cver(
                        cs2.as_ptr() as *const c_char,
                        pw.as_ptr() as *const c_char,
                        pw.len() as c_ulonglong,
                    );
                    let b = rver(
                        cs2.as_ptr() as *const c_char,
                        pw.as_ptr() as *const c_char,
                        pw.len() as c_ulonglong,
                    );
                    assert_eq!(a, b, "{tag} verify return");
                    assert_eq!(a, 0, "{tag} verify should succeed");
                    let a = cnr(cs2.as_ptr() as *const c_char, ops, mem);
                    let b = rnr(cs2.as_ptr() as *const c_char, ops, mem);
                    assert_eq!(a, b, "{tag} needs_rehash return");
                }

                if a != 0 {
                    continue;
                }
                for corrupt in corrupt_strings(&cs) {
                    let a = cver(
                        corrupt.as_ptr() as *const c_char,
                        pw.as_ptr() as *const c_char,
                        pw.len() as c_ulonglong,
                    );
                    let b = rver(
                        corrupt.as_ptr() as *const c_char,
                        pw.as_ptr() as *const c_char,
                        pw.len() as c_ulonglong,
                    );
                    assert_eq!(a, b, "crypto_pwhash_str_verify corrupt return");
                    let a = cnr(corrupt.as_ptr() as *const c_char, ops, mem);
                    let b = rnr(corrupt.as_ptr() as *const c_char, ops, mem);
                    assert_eq!(a, b, "crypto_pwhash_str_needs_rehash corrupt return");
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// scrypt
// ---------------------------------------------------------------------------

type FnScryptLl = unsafe extern "C" fn(
    *const u8,
    usize,
    *const u8,
    usize,
    u64,
    u32,
    u32,
    *mut u8,
    usize,
) -> c_int;

#[test]
fn crypto_pwhash_scryptsalsa208sha256_matches() {
    let p = "crypto_pwhash_scryptsalsa208sha256";
    pwhash_limits(p, false);
    unsafe {
        let gs = |s: &str| -> usize {
            let (c, _): (FnSize, FnSize) = pair(&format!("{p}_{s}"));
            c()
        };
        let gu = |s: &str| -> u64 {
            let (c, _): (FnUll, FnUll) = pair(&format!("{p}_{s}"));
            c()
        };
        let saltb = gs("saltbytes");
        let strb = gs("strbytes");
        let outmin = gs("bytes_min");
        let memmin = gs("memlimit_min");
        let opsmin = gu("opslimit_min");

        let (craw, rraw): (FnPwhashAlgless, FnPwhashAlgless) = pair(p);
        let (cll, rll): (FnScryptLl, FnScryptLl) = pair(&format!("{p}_ll"));
        let (cstr, rstr): (FnStr, FnStr) = pair(&format!("{p}_str"));
        let (cver, rver): (FnStrVerify, FnStrVerify) = pair(&format!("{p}_str_verify"));
        let (cnr, rnr): (FnNeedsRehash, FnNeedsRehash) = pair(&format!("{p}_str_needs_rehash"));

        let mut rng = Rng::new(0x8200);
        let mut salts: Vec<Vec<u8>> = vec![vec![0u8; saltb], vec![0xffu8; saltb]];
        salts.push(rng.vec(saltb));
        let passwords: Vec<Vec<u8>> = vec![
            b"".to_vec(),
            b"x".to_vec(),
            b"password".to_vec(),
            (0..150u8).collect(),
        ];
        let params: Vec<(u64, usize)> = vec![
            (opsmin, memmin),
            (opsmin * 2, memmin),
            (opsmin, memmin * 4),
            (32768, 1 << 20),
            (0, memmin),
            (opsmin, memmin - 1),
        ];

        for salt in &salts {
            for pw in &passwords {
                for &(ops, mem) in &params {
                    for &outlen in &[outmin, 32, 64, outmin - 1] {
                        let mut co = vec![0xAAu8; outlen.max(64) + 8];
                        let mut ro = vec![0xAAu8; outlen.max(64) + 8];
                        let a = craw(
                            co.as_mut_ptr(),
                            outlen as c_ulonglong,
                            pw.as_ptr() as *const c_char,
                            pw.len() as c_ulonglong,
                            salt.as_ptr(),
                            ops,
                            mem,
                        );
                        let b = rraw(
                            ro.as_mut_ptr(),
                            outlen as c_ulonglong,
                            pw.as_ptr() as *const c_char,
                            pw.len() as c_ulonglong,
                            salt.as_ptr(),
                            ops,
                            mem,
                        );
                        let tag = format!("{p}(outlen={outlen},ops={ops},mem={mem})");
                        assert_eq!(a, b, "{tag} return");
                        assert_bytes_eq(&tag, &co, &ro);
                    }
                }
            }
        }

        // low-level scrypt: N must be a power of two > 1; also exercise invalid
        // parameter combinations which must be rejected identically.
        let ll_cases: Vec<(u64, u32, u32)> = vec![
            (2, 1, 1),
            (4, 1, 1),
            (16, 1, 1),
            (16, 2, 1),
            (16, 1, 2),
            (16, 8, 1),
            (16, 1, 8),
            (256, 8, 1),
            (1024, 8, 1),
            (2, 8, 16),
            // invalid
            (0, 1, 1),
            (1, 1, 1),
            (3, 1, 1),
            (6, 1, 1),
            (16, 0, 1),
            (16, 1, 0),
            (u64::MAX, 1, 1),
        ];
        for &(n, r, pp) in &ll_cases {
            for &saltlen in &[0usize, 1, 16, 32] {
                for &pwlen in &[0usize, 1, 8, 64] {
                    for &buflen in &[0usize, 1, 16, 32, 64, 100] {
                        let pw = rng.vec(pwlen.max(1));
                        let salt = rng.vec(saltlen.max(1));
                        let mut cb = vec![0xAAu8; buflen + 8];
                        let mut rb = vec![0xAAu8; buflen + 8];
                        let a = cll(
                            pw.as_ptr(),
                            pwlen,
                            salt.as_ptr(),
                            saltlen,
                            n,
                            r,
                            pp,
                            cb.as_mut_ptr(),
                            buflen,
                        );
                        let b = rll(
                            pw.as_ptr(),
                            pwlen,
                            salt.as_ptr(),
                            saltlen,
                            n,
                            r,
                            pp,
                            rb.as_mut_ptr(),
                            buflen,
                        );
                        let tag = format!(
                            "{p}_ll(N={n},r={r},p={pp},saltlen={saltlen},pwlen={pwlen},buflen={buflen})"
                        );
                        assert_eq!(a, b, "{tag} return");
                        assert_bytes_eq(&tag, &cb, &rb);
                    }
                }
            }
        }

        // string form
        for pw in &passwords {
            for &(ops, mem) in &params {
                let mut cs = vec![0xAAu8; strb + 8];
                let mut rs = vec![0xAAu8; strb + 8];
                det_reset();
                let a = cstr(
                    cs.as_mut_ptr() as *mut c_char,
                    pw.as_ptr() as *const c_char,
                    pw.len() as c_ulonglong,
                    ops,
                    mem,
                );
                det_reset();
                let b = rstr(
                    rs.as_mut_ptr() as *mut c_char,
                    pw.as_ptr() as *const c_char,
                    pw.len() as c_ulonglong,
                    ops,
                    mem,
                );
                let tag = format!("{p}_str(ops={ops},mem={mem})");
                assert_eq!(a, b, "{tag} return");
                assert_bytes_eq(&tag, &cs, &rs);
                if a != 0 {
                    continue;
                }
                let a = cver(
                    cs.as_ptr() as *const c_char,
                    pw.as_ptr() as *const c_char,
                    pw.len() as c_ulonglong,
                );
                let b = rver(
                    cs.as_ptr() as *const c_char,
                    pw.as_ptr() as *const c_char,
                    pw.len() as c_ulonglong,
                );
                assert_eq!(a, b, "{tag} verify return");
                assert_eq!(a, 0, "{tag} verify should succeed");

                let wrong = b"wrong";
                let a = cver(
                    cs.as_ptr() as *const c_char,
                    wrong.as_ptr() as *const c_char,
                    wrong.len() as c_ulonglong,
                );
                let b = rver(
                    cs.as_ptr() as *const c_char,
                    wrong.as_ptr() as *const c_char,
                    wrong.len() as c_ulonglong,
                );
                assert_eq!(a, b, "{tag} verify wrong-pw return");

                for corrupt in corrupt_strings(&cs) {
                    let a = cver(
                        corrupt.as_ptr() as *const c_char,
                        pw.as_ptr() as *const c_char,
                        pw.len() as c_ulonglong,
                    );
                    let b = rver(
                        corrupt.as_ptr() as *const c_char,
                        pw.as_ptr() as *const c_char,
                        pw.len() as c_ulonglong,
                    );
                    assert_eq!(a, b, "{tag} verify corrupt return");
                    let a = cnr(corrupt.as_ptr() as *const c_char, ops, mem);
                    let b = rnr(corrupt.as_ptr() as *const c_char, ops, mem);
                    assert_eq!(a, b, "{tag} needs_rehash corrupt return");
                }
                for &(ops2, mem2) in &params {
                    let a = cnr(cs.as_ptr() as *const c_char, ops2, mem2);
                    let b = rnr(cs.as_ptr() as *const c_char, ops2, mem2);
                    assert_eq!(a, b, "{p}_str_needs_rehash(ops={ops2},mem={mem2}) return");
                }
            }
        }
    }
}
