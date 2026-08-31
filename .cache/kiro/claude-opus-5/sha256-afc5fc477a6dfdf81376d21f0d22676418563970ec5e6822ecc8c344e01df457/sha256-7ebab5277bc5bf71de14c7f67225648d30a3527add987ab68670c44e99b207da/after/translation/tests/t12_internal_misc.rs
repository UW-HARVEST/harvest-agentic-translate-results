//! Remaining ABI surface: argon2 / escrypt / mlkem768 / ed25519-sign internals,
//! hash-to-curve string expansion, the exported `*_implementation` data
//! symbols, the secure-memory API, the extended chacha20 entry points, and
//! `sodium_misuse` parity (checked in a child process, since it aborts).
mod common;

use common::*;
use std::os::raw::{c_char, c_int, c_uchar, c_ulonglong, c_void};

// ---------------------------------------------------------------------------
// argon2 internals
// ---------------------------------------------------------------------------

#[repr(C)]
struct Argon2Context {
    out: *mut u8,
    outlen: u32,
    pwd: *mut u8,
    pwdlen: u32,
    salt: *mut u8,
    saltlen: u32,
    secret: *mut u8,
    secretlen: u32,
    ad: *mut u8,
    adlen: u32,
    t_cost: u32,
    m_cost: u32,
    lanes: u32,
    threads: u32,
    flags: u32,
}

type FnArgon2Ctx = unsafe extern "C" fn(*mut Argon2Context, c_int) -> c_int;
type FnArgon2ValidateInputs = unsafe extern "C" fn(*const Argon2Context) -> c_int;
type FnArgon2Encode =
    unsafe extern "C" fn(*mut c_char, usize, *mut Argon2Context, c_int) -> c_int;
type FnArgon2Decode = unsafe extern "C" fn(*mut Argon2Context, *const c_char, c_int) -> c_int;
type FnArgon2HashRaw = unsafe extern "C" fn(
    u32,
    u32,
    u32,
    *const c_void,
    usize,
    *const c_void,
    usize,
    *mut c_void,
    usize,
) -> c_int;
type FnArgon2HashEncoded = unsafe extern "C" fn(
    u32,
    u32,
    u32,
    *const c_void,
    usize,
    *const c_void,
    usize,
    usize,
    *mut c_char,
    usize,
) -> c_int;
type FnArgon2Hash = unsafe extern "C" fn(
    u32,
    u32,
    u32,
    *const c_void,
    usize,
    *const c_void,
    usize,
    *mut c_void,
    usize,
    *mut c_char,
    usize,
    c_int,
) -> c_int;
type FnArgon2Verify =
    unsafe extern "C" fn(*const c_char, *const c_void, usize, c_int) -> c_int;
type FnArgon2VerifyT = unsafe extern "C" fn(*const c_char, *const c_void, usize) -> c_int;

/// Build a context describing one Argon2 invocation. Buffers are owned by the
/// caller and must outlive the call.
#[allow(clippy::too_many_arguments)]
fn make_ctx(
    out: &mut [u8],
    pwd: &mut [u8],
    salt: &mut [u8],
    secret: &mut [u8],
    ad: &mut [u8],
    t_cost: u32,
    m_cost: u32,
    lanes: u32,
    threads: u32,
) -> Argon2Context {
    Argon2Context {
        out: out.as_mut_ptr(),
        outlen: out.len() as u32,
        pwd: pwd.as_mut_ptr(),
        pwdlen: pwd.len() as u32,
        salt: salt.as_mut_ptr(),
        saltlen: salt.len() as u32,
        secret: if secret.is_empty() {
            std::ptr::null_mut()
        } else {
            secret.as_mut_ptr()
        },
        secretlen: secret.len() as u32,
        ad: if ad.is_empty() {
            std::ptr::null_mut()
        } else {
            ad.as_mut_ptr()
        },
        adlen: ad.len() as u32,
        t_cost,
        m_cost,
        lanes,
        threads,
        flags: 0,
    }
}


/// A context prepared to receive the result of `argon2_decode_string`: `out`
/// and `salt` are output buffers, everything else is empty.
fn decode_ctx(out: &mut [u8], salt: &mut [u8]) -> Argon2Context {
    Argon2Context {
        out: out.as_mut_ptr(),
        outlen: out.len() as u32,
        pwd: std::ptr::null_mut(),
        pwdlen: 0,
        salt: salt.as_mut_ptr(),
        saltlen: salt.len() as u32,
        secret: std::ptr::null_mut(),
        secretlen: 0,
        ad: std::ptr::null_mut(),
        adlen: 0,
        t_cost: 0,
        m_cost: 0,
        lanes: 0,
        threads: 0,
        flags: 0,
    }
}

#[test]
fn internal_argon2_matches() {
    unsafe {
        let (cvi, rvi): (FnArgon2ValidateInputs, FnArgon2ValidateInputs) =
            pair("_sodium_argon2_validate_inputs");
        let (cctx, rctx): (FnArgon2Ctx, FnArgon2Ctx) = pair("_sodium_argon2_ctx");
        let (cenc, renc): (FnArgon2Encode, FnArgon2Encode) = pair("_sodium_argon2_encode_string");
        let (cdec, rdec): (FnArgon2Decode, FnArgon2Decode) = pair("_sodium_argon2_decode_string");
        let (chash, rhash): (FnArgon2Hash, FnArgon2Hash) = pair("_sodium_argon2_hash");
        let (cver, rver): (FnArgon2Verify, FnArgon2Verify) = pair("_sodium_argon2_verify");

        let mut rng = Rng::new(0xA000);

        // validate_inputs over legal and illegal parameter sets
        let param_sets: Vec<(usize, usize, usize, u32, u32, u32, u32)> = vec![
            // (outlen, pwdlen, saltlen, t_cost, m_cost, lanes, threads)
            (32, 8, 16, 3, 4096, 1, 1),
            (32, 0, 16, 3, 4096, 1, 1),
            (32, 8, 8, 3, 4096, 1, 1),   // salt too short
            (3, 8, 16, 3, 4096, 1, 1),   // out too short
            (32, 8, 16, 0, 4096, 1, 1),  // t_cost too small
            (32, 8, 16, 3, 4, 1, 1),     // m_cost too small
            (32, 8, 16, 3, 4096, 0, 1),  // lanes zero
            (32, 8, 16, 3, 4096, 1, 0),  // threads zero
            (32, 8, 16, 3, 8192, 4, 4),
            (64, 200, 32, 1, 8, 1, 1),
            (16, 8, 16, 2, 65536, 1, 1),
        ];
        for &(outlen, pwdlen, saltlen, t, m, lanes, threads) in &param_sets {
            let mut out = vec![0u8; outlen.max(1)];
            let mut pwd = rng.vec(pwdlen.max(1));
            pwd.truncate(pwdlen.max(1));
            let mut salt = rng.vec(saltlen.max(1));
            let mut secret: Vec<u8> = Vec::new();
            let mut ad: Vec<u8> = Vec::new();
            let mut ctx = make_ctx(
                &mut out, &mut pwd, &mut salt, &mut secret, &mut ad, t, m, lanes, threads,
            );
            ctx.outlen = outlen as u32;
            ctx.pwdlen = pwdlen as u32;
            ctx.saltlen = saltlen as u32;
            let a = cvi(&ctx as *const _);
            let b = rvi(&ctx as *const _);
            assert_eq!(
                a, b,
                "argon2_validate_inputs(outlen={outlen},pwdlen={pwdlen},saltlen={saltlen},t={t},m={m},lanes={lanes},threads={threads})"
            );
        }

        // argon2_ctx: full hash through the low-level context API
        for &alg in &[1 as c_int, 2, 0, 3] {
            for &(t, m, lanes) in &[(1u32, 8u32, 1u32), (3, 4096, 1), (2, 8192, 4)] {
                for &outlen in &[16usize, 32, 64] {
                    let mut cout = vec![0xAAu8; outlen + 8];
                    let mut rout = vec![0xAAu8; outlen + 8];
                    let mut pwd = b"password".to_vec();
                    let mut salt = b"0123456789abcdef".to_vec();
                    let mut secret = b"secret".to_vec();
                    let mut ad = b"associated".to_vec();

                    let mut cc = make_ctx(
                        &mut cout, &mut pwd, &mut salt, &mut secret, &mut ad, t, m, lanes, lanes,
                    );
                    cc.outlen = outlen as u32;
                    let a = cctx(&mut cc as *mut _, alg);

                    let mut pwd2 = b"password".to_vec();
                    let mut salt2 = b"0123456789abcdef".to_vec();
                    let mut secret2 = b"secret".to_vec();
                    let mut ad2 = b"associated".to_vec();
                    let mut rc = make_ctx(
                        &mut rout, &mut pwd2, &mut salt2, &mut secret2, &mut ad2, t, m, lanes,
                        lanes,
                    );
                    rc.outlen = outlen as u32;
                    let b = rctx(&mut rc as *mut _, alg);

                    let tag = format!("argon2_ctx(alg={alg},t={t},m={m},lanes={lanes},outlen={outlen})");
                    assert_eq!(a, b, "{tag} return");
                    assert_bytes_eq(&tag, &cout, &rout);
                    // the password buffer is wiped by argon2_ctx on success
                    assert_bytes_eq(&format!("{tag} pwd wipe"), &pwd, &pwd2);
                }
            }
        }

        // encode_string / decode_string
        for &alg in &[1 as c_int, 2] {
            for &(t, m, lanes) in &[(1u32, 8u32, 1u32), (3, 4096, 1), (2, 65536, 4)] {
                let mut out = rng.vec(32);
                let mut pwd = b"pw".to_vec();
                let mut salt = rng.vec(16);
                let mut secret: Vec<u8> = Vec::new();
                let mut ad: Vec<u8> = Vec::new();
                let mut ctx = make_ctx(
                    &mut out, &mut pwd, &mut salt, &mut secret, &mut ad, t, m, lanes, lanes,
                );
                // argon2_encode_string base64-encodes into the tail of `dst`
                // via sodium_bin2base64, which calls sodium_misuse() (abort)
                // when the remaining space is too small. Only use lengths that
                // either fail early in the textual prefix (<= 16) or are large
                // enough for the whole string.
                for &dstlen in &[0usize, 1, 8, 16, 256, 512] {
                    let mut cs = vec![0xAAu8; dstlen + 8];
                    let mut rs = vec![0xAAu8; dstlen + 8];
                    let a = cenc(
                        cs.as_mut_ptr() as *mut c_char,
                        dstlen,
                        &mut ctx as *mut _,
                        alg,
                    );
                    let b = renc(
                        rs.as_mut_ptr() as *mut c_char,
                        dstlen,
                        &mut ctx as *mut _,
                        alg,
                    );
                    let tag = format!("argon2_encode_string(alg={alg},t={t},m={m},dstlen={dstlen})");
                    assert_eq!(a, b, "{tag} return");
                    assert_bytes_eq(&tag, &cs, &rs);

                    if a != 0 {
                        continue;
                    }
                    // decode it back
                    let mut cout2 = vec![0u8; 32];
                    let mut csalt2 = vec![0u8; 64];
                    let mut cctx2 = decode_ctx(&mut cout2, &mut csalt2);

                    let mut rout2 = vec![0u8; 32];
                    let mut rsalt2 = vec![0u8; 64];
                    let mut rctx2 = decode_ctx(&mut rout2, &mut rsalt2);

                    let a = cdec(
                        &mut cctx2 as *mut _,
                        cs.as_ptr() as *const c_char,
                        alg,
                    );
                    let b = rdec(
                        &mut rctx2 as *mut _,
                        cs.as_ptr() as *const c_char,
                        alg,
                    );
                    let dtag = format!("argon2_decode_string(alg={alg})");
                    assert_eq!(a, b, "{dtag} return");
                    assert_bytes_eq(&format!("{dtag} out"), &cout2, &rout2);
                    assert_bytes_eq(&format!("{dtag} salt"), &csalt2, &rsalt2);
                    assert_eq!(
                        (cctx2.t_cost, cctx2.m_cost, cctx2.lanes, cctx2.threads, cctx2.outlen, cctx2.saltlen),
                        (rctx2.t_cost, rctx2.m_cost, rctx2.lanes, rctx2.threads, rctx2.outlen, rctx2.saltlen),
                        "{dtag} decoded parameters"
                    );
                }
            }
        }

        // decode_string on malformed inputs
        let bad_strings: Vec<&[u8]> = vec![
            b"\0",
            b"$\0",
            b"$argon2i$\0",
            b"$argon2id$v=19$m=8,t=1,p=1\0",
            b"$argon2id$v=19$m=8,t=1,p=1$\0",
            b"$argon2id$v=18$m=8,t=1,p=1$aaaaaaaaaaaaaaaaaaaaaa$aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\0",
            b"$argon2id$v=19$m=0,t=0,p=0$aaaa$bbbb\0",
            b"$argon2xx$v=19$m=8,t=1,p=1$aaaa$bbbb\0",
            b"garbage\0",
        ];
        for &alg in &[1 as c_int, 2] {
            for s in &bad_strings {
                let mut cout = vec![0u8; 32];
                let mut csalt = vec![0u8; 64];
                let mut c2 = decode_ctx(&mut cout, &mut csalt);

                let mut rout = vec![0u8; 32];
                let mut rsalt = vec![0u8; 64];
                let mut r2 = decode_ctx(&mut rout, &mut rsalt);

                let a = cdec(&mut c2 as *mut _, s.as_ptr() as *const c_char, alg);
                let b = rdec(&mut r2 as *mut _, s.as_ptr() as *const c_char, alg);
                assert_eq!(
                    a,
                    b,
                    "argon2_decode_string bad({:?}) return",
                    String::from_utf8_lossy(s)
                );
                assert_bytes_eq("argon2_decode_string bad out", &cout, &rout);
                assert_bytes_eq("argon2_decode_string bad salt", &csalt, &rsalt);
            }
        }

        // argon2{i,id}_hash_raw / _hash_encoded / argon2_hash / _verify
        let pwd = b"my password";
        let salt = b"0123456789abcdef";
        for (raw_name, enc_name, ver_name, alg) in [
            (
                "_sodium_argon2i_hash_raw",
                "_sodium_argon2i_hash_encoded",
                "_sodium_argon2i_verify",
                1 as c_int,
            ),
            (
                "_sodium_argon2id_hash_raw",
                "_sodium_argon2id_hash_encoded",
                "_sodium_argon2id_verify",
                2,
            ),
        ] {
            let (craw, rraw): (FnArgon2HashRaw, FnArgon2HashRaw) = pair(raw_name);
            let (cenc2, renc2): (FnArgon2HashEncoded, FnArgon2HashEncoded) = pair(enc_name);
            let (cver2, rver2): (FnArgon2VerifyT, FnArgon2VerifyT) = pair(ver_name);
            for &(t, m, p) in &[(1u32, 8u32, 1u32), (3, 4096, 1), (2, 8192, 1)] {
                for &hashlen in &[16usize, 32, 64, 3] {
                    // argon2_hash() prefills `hash` with randombytes_buf()
                    // before hashing, and leaves that prefill in place when the
                    // parameters are rejected, so rewind the shared stream.
                    let mut ch = vec![0xAAu8; hashlen + 8];
                    let mut rh = vec![0xAAu8; hashlen + 8];
                    det_reset();
                    let a = craw(
                        t,
                        m,
                        p,
                        pwd.as_ptr() as *const c_void,
                        pwd.len(),
                        salt.as_ptr() as *const c_void,
                        salt.len(),
                        ch.as_mut_ptr() as *mut c_void,
                        hashlen,
                    );
                    det_reset();
                    let b = rraw(
                        t,
                        m,
                        p,
                        pwd.as_ptr() as *const c_void,
                        pwd.len(),
                        salt.as_ptr() as *const c_void,
                        salt.len(),
                        rh.as_mut_ptr() as *mut c_void,
                        hashlen,
                    );
                    let tag = format!("{raw_name}(t={t},m={m},p={p},hashlen={hashlen})");
                    assert_eq!(a, b, "{tag} return");
                    assert_bytes_eq(&tag, &ch, &rh);

                    // Same abort hazard as argon2_encode_string: 0 skips
                    // encoding entirely, <= 16 fails in the prefix, >= 256 fits.
                    for &enclen in &[0usize, 8, 16, 256] {
                        let mut ce = vec![0xAAu8; enclen + 8];
                        let mut re = vec![0xAAu8; enclen + 8];
                        det_reset();
                        let a = cenc2(
                            t,
                            m,
                            p,
                            pwd.as_ptr() as *const c_void,
                            pwd.len(),
                            salt.as_ptr() as *const c_void,
                            salt.len(),
                            hashlen,
                            ce.as_mut_ptr() as *mut c_char,
                            enclen,
                        );
                        det_reset();
                        let b = renc2(
                            t,
                            m,
                            p,
                            pwd.as_ptr() as *const c_void,
                            pwd.len(),
                            salt.as_ptr() as *const c_void,
                            salt.len(),
                            hashlen,
                            re.as_mut_ptr() as *mut c_char,
                            enclen,
                        );
                        let etag = format!("{enc_name}(t={t},m={m},hashlen={hashlen},enclen={enclen})");
                        assert_eq!(a, b, "{etag} return");
                        assert_bytes_eq(&etag, &ce, &re);
                        if a != 0 {
                            continue;
                        }
                        let a = cver2(
                            ce.as_ptr() as *const c_char,
                            pwd.as_ptr() as *const c_void,
                            pwd.len(),
                        );
                        let b = rver2(
                            ce.as_ptr() as *const c_char,
                            pwd.as_ptr() as *const c_void,
                            pwd.len(),
                        );
                        assert_eq!(a, b, "{ver_name} good return");
                        let wrong = b"nope";
                        let a = cver2(
                            ce.as_ptr() as *const c_char,
                            wrong.as_ptr() as *const c_void,
                            wrong.len(),
                        );
                        let b = rver2(
                            ce.as_ptr() as *const c_char,
                            wrong.as_ptr() as *const c_void,
                            wrong.len(),
                        );
                        assert_eq!(a, b, "{ver_name} wrong-pw return");
                        for bad in &bad_strings {
                            let a = cver2(
                                bad.as_ptr() as *const c_char,
                                pwd.as_ptr() as *const c_void,
                                pwd.len(),
                            );
                            let b = rver2(
                                bad.as_ptr() as *const c_char,
                                pwd.as_ptr() as *const c_void,
                                pwd.len(),
                            );
                            assert_eq!(a, b, "{ver_name} bad-string return");
                        }
                        // generic argon2_verify with an explicit type
                        let a = cver(
                            ce.as_ptr() as *const c_char,
                            pwd.as_ptr() as *const c_void,
                            pwd.len(),
                            alg,
                        );
                        let b = rver(
                            ce.as_ptr() as *const c_char,
                            pwd.as_ptr() as *const c_void,
                            pwd.len(),
                            alg,
                        );
                        assert_eq!(a, b, "argon2_verify(alg={alg}) return");
                    }
                }
            }
        }

        // argon2_hash: raw and encoded outputs together
        for &alg in &[1 as c_int, 2] {
            for &hashlen in &[16usize, 32] {
                let mut ch = vec![0xAAu8; hashlen + 8];
                let mut rh = vec![0xAAu8; hashlen + 8];
                let mut ce = vec![0xAAu8; 200];
                let mut re = vec![0xAAu8; 200];
                det_reset();
                let a = chash(
                    2,
                    4096,
                    1,
                    pwd.as_ptr() as *const c_void,
                    pwd.len(),
                    salt.as_ptr() as *const c_void,
                    salt.len(),
                    ch.as_mut_ptr() as *mut c_void,
                    hashlen,
                    ce.as_mut_ptr() as *mut c_char,
                    192,
                    alg,
                );
                det_reset();
                let b = rhash(
                    2,
                    4096,
                    1,
                    pwd.as_ptr() as *const c_void,
                    pwd.len(),
                    salt.as_ptr() as *const c_void,
                    salt.len(),
                    rh.as_mut_ptr() as *mut c_void,
                    hashlen,
                    re.as_mut_ptr() as *mut c_char,
                    192,
                    alg,
                );
                let tag = format!("argon2_hash(alg={alg},hashlen={hashlen})");
                assert_eq!(a, b, "{tag} return");
                assert_bytes_eq(&format!("{tag} raw"), &ch, &rh);
                assert_bytes_eq(&format!("{tag} encoded"), &ce, &re);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// escrypt internals
// ---------------------------------------------------------------------------

#[repr(C)]
#[derive(Copy, Clone)]
struct EscryptRegion {
    base: *mut c_void,
    aligned: *mut c_void,
    size: usize,
}

type FnPbkdf2 =
    unsafe extern "C" fn(*const u8, usize, *const u8, usize, u64, *mut u8, usize);
type FnEsInitLocal = unsafe extern "C" fn(*mut EscryptRegion) -> c_int;
type FnEsFreeLocal = unsafe extern "C" fn(*mut EscryptRegion) -> c_int;
type FnEsAllocRegion = unsafe extern "C" fn(*mut EscryptRegion, usize) -> *mut c_void;
type FnEsFreeRegion = unsafe extern "C" fn(*mut EscryptRegion) -> c_int;
type FnEsKdf = unsafe extern "C" fn(
    *mut EscryptRegion,
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
type FnEsR = unsafe extern "C" fn(
    *mut EscryptRegion,
    *const u8,
    usize,
    *const u8,
    *mut u8,
    usize,
) -> *mut u8;
type FnEsGensalt = unsafe extern "C" fn(u32, u32, u32, *const u8, usize, *mut u8, usize) -> *mut u8;
type FnEsParseSetting =
    unsafe extern "C" fn(*const u8, *mut u32, *mut u32, *mut u32) -> *const u8;

#[test]
fn internal_escrypt_matches() {
    unsafe {
        let (cpb, rpb): (FnPbkdf2, FnPbkdf2) = pair("_sodium_escrypt_PBKDF2_SHA256");
        let (cil, ril): (FnEsInitLocal, FnEsInitLocal) = pair("_sodium_escrypt_init_local");
        let (cfl, rfl): (FnEsFreeLocal, FnEsFreeLocal) = pair("_sodium_escrypt_free_local");
        let (car, rar): (FnEsAllocRegion, FnEsAllocRegion) = pair("_sodium_escrypt_alloc_region");
        let (cfr, rfr): (FnEsFreeRegion, FnEsFreeRegion) = pair("_sodium_escrypt_free_region");
        let (ckdf, rkdf): (FnEsKdf, FnEsKdf) = pair("_sodium_escrypt_kdf_nosse");
        let (cr, rr): (FnEsR, FnEsR) = pair("_sodium_escrypt_r");
        let (cgs, rgs): (FnEsGensalt, FnEsGensalt) = pair("_sodium_escrypt_gensalt_r");
        let (cps, rps): (FnEsParseSetting, FnEsParseSetting) =
            pair("_sodium_escrypt_parse_setting");

        let mut rng = Rng::new(0xA100);

        // PBKDF2-HMAC-SHA256
        for &c in &[1u64, 2, 10, 1000] {
            for &pwlen in &[0usize, 1, 32, 64, 65, 128, 200] {
                for &saltlen in &[0usize, 1, 16, 32, 64, 200] {
                    for &dklen in &[0usize, 1, 31, 32, 33, 64, 65, 100] {
                        let pw = rng.vec(pwlen.max(1));
                        let salt = rng.vec(saltlen.max(1));
                        let mut cb = vec![0xAAu8; dklen + 8];
                        let mut rb = vec![0xAAu8; dklen + 8];
                        cpb(
                            pw.as_ptr(),
                            pwlen,
                            salt.as_ptr(),
                            saltlen,
                            c,
                            cb.as_mut_ptr(),
                            dklen,
                        );
                        rpb(
                            pw.as_ptr(),
                            pwlen,
                            salt.as_ptr(),
                            saltlen,
                            c,
                            rb.as_mut_ptr(),
                            dklen,
                        );
                        assert_bytes_eq(
                            &format!("escrypt_PBKDF2_SHA256(c={c},pw={pwlen},salt={saltlen},dk={dklen})"),
                            &cb,
                            &rb,
                        );
                    }
                }
            }
        }

        // local/region lifecycle: only the return codes are comparable (the
        // pointers are allocation-dependent), plus the recorded size.
        let mut cl = EscryptRegion {
            base: std::ptr::null_mut(),
            aligned: std::ptr::null_mut(),
            size: 0,
        };
        let mut rl = cl;
        assert_eq!(cil(&mut cl), ril(&mut rl), "escrypt_init_local return");
        assert_eq!(cl.size, rl.size, "escrypt_init_local size");

        for &size in &[1usize, 16, 4096, 1 << 20] {
            let mut cregion = EscryptRegion {
                base: std::ptr::null_mut(),
                aligned: std::ptr::null_mut(),
                size: 0,
            };
            let mut rregion = cregion;
            let cp = car(&mut cregion, size);
            let rp = rar(&mut rregion, size);
            assert_eq!(
                cp.is_null(),
                rp.is_null(),
                "escrypt_alloc_region({size}) null-ness"
            );
            assert_eq!(cregion.size, rregion.size, "escrypt_alloc_region({size}) size");
            assert_eq!(
                cfr(&mut cregion),
                rfr(&mut rregion),
                "escrypt_free_region({size}) return"
            );
        }

        // kdf_nosse
        let kdf_cases: Vec<(u64, u32, u32)> = vec![
            (2, 1, 1),
            (16, 1, 1),
            (16, 8, 1),
            (16, 1, 4),
            (1024, 8, 1),
            (0, 1, 1),
            (1, 1, 1),
            (3, 1, 1),
            (16, 0, 1),
            (16, 1, 0),
        ];
        for &(n, r, p) in &kdf_cases {
            for &buflen in &[0usize, 1, 32, 64, 100] {
                let pw = rng.vec(16);
                let salt = rng.vec(16);
                let mut cb = vec![0xAAu8; buflen + 8];
                let mut rb = vec![0xAAu8; buflen + 8];
                let a = ckdf(
                    &mut cl, pw.as_ptr(), 16, salt.as_ptr(), 16, n, r, p, cb.as_mut_ptr(), buflen,
                );
                let b = rkdf(
                    &mut rl, pw.as_ptr(), 16, salt.as_ptr(), 16, n, r, p, rb.as_mut_ptr(), buflen,
                );
                let tag = format!("escrypt_kdf_nosse(N={n},r={r},p={p},buflen={buflen})");
                assert_eq!(a, b, "{tag} return");
                assert_bytes_eq(&tag, &cb, &rb);
            }
        }

        // gensalt_r + parse_setting + escrypt_r
        for &(n_log2, r, p) in &[(1u32, 1u32, 1u32), (4, 8, 1), (10, 8, 1), (0, 1, 1), (64, 1, 1)] {
            for &srclen in &[0usize, 1, 16, 32] {
                for &buflen in &[0usize, 8, 32, 64, 128] {
                    let src = rng.vec(srclen.max(1));
                    let mut cb = vec![0xAAu8; buflen + 8];
                    let mut rb = vec![0xAAu8; buflen + 8];
                    let cp = cgs(n_log2, r, p, src.as_ptr(), srclen, cb.as_mut_ptr(), buflen);
                    let rp = rgs(n_log2, r, p, src.as_ptr(), srclen, rb.as_mut_ptr(), buflen);
                    let tag = format!(
                        "escrypt_gensalt_r(N_log2={n_log2},r={r},p={p},srclen={srclen},buflen={buflen})"
                    );
                    assert_eq!(cp.is_null(), rp.is_null(), "{tag} null-ness");
                    assert_bytes_eq(&tag, &cb, &rb);
                    if cp.is_null() {
                        continue;
                    }

                    // parse_setting on the generated setting
                    let mut cn = 0u32;
                    let mut crr = 0u32;
                    let mut cpp = 0u32;
                    let mut rn = 0u32;
                    let mut rrr = 0u32;
                    let mut rpp = 0u32;
                    let cend = cps(cb.as_ptr(), &mut cn, &mut crr, &mut cpp);
                    let rend = rps(cb.as_ptr(), &mut rn, &mut rrr, &mut rpp);
                    assert_eq!(cend.is_null(), rend.is_null(), "escrypt_parse_setting null-ness");
                    if !cend.is_null() {
                        assert_eq!(
                            cend as usize - cb.as_ptr() as usize,
                            rend as usize - cb.as_ptr() as usize,
                            "escrypt_parse_setting end offset"
                        );
                    }
                    assert_eq!(
                        (cn, crr, cpp),
                        (rn, rrr, rpp),
                        "escrypt_parse_setting outputs"
                    );

                    // escrypt_r using the generated setting
                    for &obuflen in &[0usize, 16, 64, 128] {
                        let pw = b"password";
                        // escrypt_r prefills `buf` with randombytes_buf() and
                        // leaves the prefill when it bails out, so rewind the
                        // shared deterministic stream before each side.
                        let mut co = vec![0xAAu8; obuflen + 8];
                        let mut ro = vec![0xAAu8; obuflen + 8];
                        det_reset();
                        let cq = cr(
                            &mut cl,
                            pw.as_ptr(),
                            pw.len(),
                            cb.as_ptr(),
                            co.as_mut_ptr(),
                            obuflen,
                        );
                        det_reset();
                        let rq = rr(
                            &mut rl,
                            pw.as_ptr(),
                            pw.len(),
                            cb.as_ptr(),
                            ro.as_mut_ptr(),
                            obuflen,
                        );
                        assert_eq!(cq.is_null(), rq.is_null(), "escrypt_r null-ness");
                        assert_bytes_eq(&format!("escrypt_r(obuflen={obuflen})"), &co, &ro);
                    }
                }
            }
        }

        // parse_setting on malformed settings
        let bad: Vec<&[u8]> = vec![
            b"\0",
            b"$\0",
            b"$7\0",
            b"$7$\0",
            b"$7$A\0",
            b"$8$A0000000\0",
            b"$7$C6..../....\0",
            b"$7$zzzzzzzzzzzzzzzzzzzzzzzz\0",
            b"nonsense\0",
        ];
        for s in &bad {
            let mut cn = 0xdeadu32;
            let mut crr = 0xdeadu32;
            let mut cpp = 0xdeadu32;
            let mut rn = 0xdeadu32;
            let mut rrr = 0xdeadu32;
            let mut rpp = 0xdeadu32;
            let cend = cps(s.as_ptr(), &mut cn, &mut crr, &mut cpp);
            let rend = rps(s.as_ptr(), &mut rn, &mut rrr, &mut rpp);
            assert_eq!(
                cend.is_null(),
                rend.is_null(),
                "escrypt_parse_setting bad({:?}) null-ness",
                String::from_utf8_lossy(s)
            );
            assert_eq!((cn, crr, cpp), (rn, rrr, rpp), "escrypt_parse_setting bad outputs");
        }

        assert_eq!(cfl(&mut cl), rfl(&mut rl), "escrypt_free_local return");
    }
}

// ---------------------------------------------------------------------------
// mlkem768 reference implementation, sign internals, hash-to-curve expansion
// ---------------------------------------------------------------------------

type FnKemSeedKeypair = unsafe extern "C" fn(*mut c_uchar, *mut c_uchar, *const c_uchar) -> c_int;
type FnKemKeypair = unsafe extern "C" fn(*mut c_uchar, *mut c_uchar) -> c_int;
type FnKemEnc = unsafe extern "C" fn(*mut c_uchar, *mut c_uchar, *const c_uchar) -> c_int;
type FnKemEncDet =
    unsafe extern "C" fn(*mut c_uchar, *mut c_uchar, *const c_uchar, *const c_uchar) -> c_int;
type FnKemDec = unsafe extern "C" fn(*mut c_uchar, *const c_uchar, *const c_uchar) -> c_int;

#[test]
fn internal_mlkem768_ref_matches() {
    unsafe {
        let (cpk, _): (FnSize, FnSize) = pair("crypto_kem_mlkem768_publickeybytes");
        let (csk, _): (FnSize, FnSize) = pair("crypto_kem_mlkem768_secretkeybytes");
        let (cct, _): (FnSize, FnSize) = pair("crypto_kem_mlkem768_ciphertextbytes");
        let (css, _): (FnSize, FnSize) = pair("crypto_kem_mlkem768_sharedsecretbytes");
        let (csd, _): (FnSize, FnSize) = pair("crypto_kem_mlkem768_seedbytes");
        let pkb = cpk();
        let skb = csk();
        let ctb = cct();
        let ssb = css();
        let sdb = csd();

        let (cskp, rskp): (FnKemSeedKeypair, FnKemSeedKeypair) =
            pair("_sodium_mlkem768_ref_seed_keypair");
        let (ckp, rkp): (FnKemKeypair, FnKemKeypair) = pair("_sodium_mlkem768_ref_keypair");
        let (ce, re): (FnKemEnc, FnKemEnc) = pair("_sodium_mlkem768_ref_enc");
        let (ced, red): (FnKemEncDet, FnKemEncDet) = pair("_sodium_mlkem768_ref_enc_deterministic");
        let (cd, rd): (FnKemDec, FnKemDec) = pair("_sodium_mlkem768_ref_dec");

        let mut rng = Rng::new(0xA200);
        let mut seeds: Vec<Vec<u8>> = vec![vec![0u8; sdb], vec![0xffu8; sdb]];
        for _ in 0..4 {
            seeds.push(rng.vec(sdb));
        }
        let mut kps: Vec<(Vec<u8>, Vec<u8>)> = Vec::new();
        for seed in &seeds {
            let mut cp = vec![0xAAu8; pkb + 8];
            let mut rp = vec![0xAAu8; pkb + 8];
            let mut cs = vec![0xAAu8; skb + 8];
            let mut rs = vec![0xAAu8; skb + 8];
            let a = cskp(cp.as_mut_ptr(), cs.as_mut_ptr(), seed.as_ptr());
            let b = rskp(rp.as_mut_ptr(), rs.as_mut_ptr(), seed.as_ptr());
            assert_eq!(a, b, "mlkem768_ref_seed_keypair return");
            assert_bytes_eq("mlkem768_ref_seed_keypair pk", &cp, &rp);
            assert_bytes_eq("mlkem768_ref_seed_keypair sk", &cs, &rs);
            kps.push((cp[..pkb].to_vec(), cs[..skb].to_vec()));
        }
        for _ in 0..4 {
            let mut cp = vec![0xAAu8; pkb + 8];
            let mut rp = vec![0xAAu8; pkb + 8];
            let mut cs = vec![0xAAu8; skb + 8];
            let mut rs = vec![0xAAu8; skb + 8];
            det_reset();
            let a = ckp(cp.as_mut_ptr(), cs.as_mut_ptr());
            det_reset();
            let b = rkp(rp.as_mut_ptr(), rs.as_mut_ptr());
            assert_eq!(a, b, "mlkem768_ref_keypair return");
            assert_bytes_eq("mlkem768_ref_keypair pk", &cp, &rp);
            assert_bytes_eq("mlkem768_ref_keypair sk", &cs, &rs);
        }

        let mut dseeds: Vec<Vec<u8>> = vec![vec![0u8; 32], vec![0xffu8; 32]];
        for _ in 0..4 {
            dseeds.push(rng.vec(32));
        }
        for (pk, sk) in &kps {
            let mut cct2 = vec![0xAAu8; ctb + 8];
            let mut rct2 = vec![0xAAu8; ctb + 8];
            let mut css2 = vec![0xAAu8; ssb + 8];
            let mut rss2 = vec![0xAAu8; ssb + 8];
            det_reset();
            let a = ce(cct2.as_mut_ptr(), css2.as_mut_ptr(), pk.as_ptr());
            det_reset();
            let b = re(rct2.as_mut_ptr(), rss2.as_mut_ptr(), pk.as_ptr());
            assert_eq!(a, b, "mlkem768_ref_enc return");
            assert_bytes_eq("mlkem768_ref_enc ct", &cct2, &rct2);
            assert_bytes_eq("mlkem768_ref_enc ss", &css2, &rss2);

            for ds in &dseeds {
                let mut cct3 = vec![0xAAu8; ctb + 8];
                let mut rct3 = vec![0xAAu8; ctb + 8];
                let mut css3 = vec![0xAAu8; ssb + 8];
                let mut rss3 = vec![0xAAu8; ssb + 8];
                let a = ced(
                    cct3.as_mut_ptr(),
                    css3.as_mut_ptr(),
                    pk.as_ptr(),
                    ds.as_ptr(),
                );
                let b = red(
                    rct3.as_mut_ptr(),
                    rss3.as_mut_ptr(),
                    pk.as_ptr(),
                    ds.as_ptr(),
                );
                assert_eq!(a, b, "mlkem768_ref_enc_deterministic return");
                assert_bytes_eq("mlkem768_ref_enc_deterministic ct", &cct3, &rct3);
                assert_bytes_eq("mlkem768_ref_enc_deterministic ss", &css3, &rss3);

                let mut cs4 = vec![0xAAu8; ssb + 8];
                let mut rs4 = vec![0xAAu8; ssb + 8];
                let a = cd(cs4.as_mut_ptr(), cct3.as_ptr(), sk.as_ptr());
                let b = rd(rs4.as_mut_ptr(), cct3.as_ptr(), sk.as_ptr());
                assert_eq!(a, b, "mlkem768_ref_dec return");
                assert_bytes_eq("mlkem768_ref_dec ss", &cs4, &rs4);

                // implicit rejection on tampered ciphertexts
                for pos in [0usize, ctb / 2, ctb - 1] {
                    let mut bad = cct3[..ctb].to_vec();
                    bad[pos] ^= 1;
                    let mut cs5 = vec![0xAAu8; ssb + 8];
                    let mut rs5 = vec![0xAAu8; ssb + 8];
                    let a = cd(cs5.as_mut_ptr(), bad.as_ptr(), sk.as_ptr());
                    let b = rd(rs5.as_mut_ptr(), bad.as_ptr(), sk.as_ptr());
                    assert_eq!(a, b, "mlkem768_ref_dec tampered@{pos} return");
                    assert_bytes_eq(&format!("mlkem768_ref_dec tampered@{pos}"), &cs5, &rs5);
                }
            }
        }
    }
}

type FnHinit = unsafe extern "C" fn(*mut c_void, c_int);
type FnSignDetachedPh = unsafe extern "C" fn(
    *mut c_uchar,
    *mut c_ulonglong,
    *const c_uchar,
    c_ulonglong,
    *const c_uchar,
    c_int,
) -> c_int;
type FnVerifyDetachedPh = unsafe extern "C" fn(
    *const c_uchar,
    *const c_uchar,
    c_ulonglong,
    *const c_uchar,
    c_int,
) -> c_int;

#[test]
fn internal_sign_ed25519_matches() {
    unsafe {
        let (chi, rhi): (FnHinit, FnHinit) = pair("_crypto_sign_ed25519_ref10_hinit");
        let (csd, rsd): (FnSignDetachedPh, FnSignDetachedPh) = pair("_crypto_sign_ed25519_detached");
        let (cvd, rvd): (FnVerifyDetachedPh, FnVerifyDetachedPh) =
            pair("_crypto_sign_ed25519_verify_detached");
        let (csb, _): (FnSize, FnSize) = pair("crypto_hash_sha512_statebytes");
        let sb = csb();

        // hinit primes a SHA-512 state (with the Ed25519ph dom2 prefix when
        // prehashed != 0)
        for prehashed in [0 as c_int, 1, 2, -1] {
            let mut cst = AlignedBuf::new(sb, 0xA5);
            let mut rst = AlignedBuf::new(sb, 0xA5);
            chi(cst.as_mut_ptr() as *mut c_void, prehashed);
            rhi(rst.as_mut_ptr() as *mut c_void, prehashed);
            assert_bytes_eq(
                &format!("_crypto_sign_ed25519_ref10_hinit({prehashed})"),
                cst.as_slice(),
                rst.as_slice(),
            );
        }

        let (cskkp, _): (FnKemSeedKeypair, FnKemSeedKeypair) =
            pair("crypto_sign_ed25519_seed_keypair");
        let mut rng = Rng::new(0xA300);
        let msg = rng.vec(1001);
        let mut kps: Vec<(Vec<u8>, Vec<u8>)> = Vec::new();
        for _ in 0..3 {
            let seed = rng.vec(32);
            let mut pk = vec![0u8; 32];
            let mut sk = vec![0u8; 64];
            assert_eq!(cskkp(pk.as_mut_ptr(), sk.as_mut_ptr(), seed.as_ptr()), 0);
            kps.push((pk, sk));
        }

        for (pk, sk) in &kps {
            for prehashed in [0 as c_int, 1] {
                // In prehashed mode the "message" is a 64-byte SHA-512 digest.
                let mlens: Vec<usize> = if prehashed == 0 {
                    vec![0, 1, 32, 64, 128, 1000]
                } else {
                    vec![64]
                };
                for &mlen in &mlens {
                    let mut csig = vec![0xAAu8; 64 + 8];
                    let mut rsig = vec![0xAAu8; 64 + 8];
                    let mut cl: c_ulonglong = 0xdead;
                    let mut rl: c_ulonglong = 0xdead;
                    let a = csd(
                        csig.as_mut_ptr(),
                        &mut cl,
                        msg.as_ptr(),
                        mlen as c_ulonglong,
                        sk.as_ptr(),
                        prehashed,
                    );
                    let b = rsd(
                        rsig.as_mut_ptr(),
                        &mut rl,
                        msg.as_ptr(),
                        mlen as c_ulonglong,
                        sk.as_ptr(),
                        prehashed,
                    );
                    let tag = format!("_crypto_sign_ed25519_detached(prehashed={prehashed},mlen={mlen})");
                    assert_eq!(a, b, "{tag} return");
                    assert_eq!(cl, rl, "{tag} siglen");
                    assert_bytes_eq(&tag, &csig, &rsig);

                    let a = cvd(
                        csig.as_ptr(),
                        msg.as_ptr(),
                        mlen as c_ulonglong,
                        pk.as_ptr(),
                        prehashed,
                    );
                    let b = rvd(
                        csig.as_ptr(),
                        msg.as_ptr(),
                        mlen as c_ulonglong,
                        pk.as_ptr(),
                        prehashed,
                    );
                    assert_eq!(a, b, "{tag} verify return");
                    assert_eq!(a, 0, "{tag} verify should succeed");

                    let mut badsigs: Vec<Vec<u8>> = vec![vec![0u8; 64], vec![0xffu8; 64]];
                    for bit in [0usize, 255, 256, 511] {
                        let mut v = csig[..64].to_vec();
                        v[bit / 8] ^= 1 << (bit % 8);
                        badsigs.push(v);
                    }
                    for bad in &badsigs {
                        let a = cvd(
                            bad.as_ptr(),
                            msg.as_ptr(),
                            mlen as c_ulonglong,
                            pk.as_ptr(),
                            prehashed,
                        );
                        let b = rvd(
                            bad.as_ptr(),
                            msg.as_ptr(),
                            mlen as c_ulonglong,
                            pk.as_ptr(),
                            prehashed,
                        );
                        assert_eq!(a, b, "{tag} verify bad-sig return");
                    }
                }
            }
        }
    }
}

type FnH2C = unsafe extern "C" fn(
    *mut c_uchar,
    usize,
    *const c_uchar,
    usize,
    *const c_uchar,
    usize,
    c_int,
) -> c_int;

#[test]
fn internal_core_h2c_matches() {
    unsafe {
        let (c, r): (FnH2C, FnH2C) = pair("_sodium_core_h2c_string_to_hash");
        let mut rng = Rng::new(0xA400);
        let msg = rng.vec(1001);
        let ctx = rng.vec(300);
        for alg in [0 as c_int, 1, 2, 3, -1] {
            // core_h2c_string_to_hash asserts h_len <= 0xff (asserts are live:
            // the reference build does not define NDEBUG).
            for &hlen in &[0usize, 1, 32, 33, 64, 65, 96, 128, 200, 254, 255] {
                for &ctxlen in &[0usize, 1, 16, 200, 255, 256] {
                    for &msglen in &[0usize, 1, 32, 100, 1000] {
                        let mut co = vec![0xAAu8; hlen + 8];
                        let mut ro = vec![0xAAu8; hlen + 8];
                        let a = c(
                            co.as_mut_ptr(),
                            hlen,
                            ctx.as_ptr(),
                            ctxlen,
                            msg.as_ptr(),
                            msglen,
                            alg,
                        );
                        let b = r(
                            ro.as_mut_ptr(),
                            hlen,
                            ctx.as_ptr(),
                            ctxlen,
                            msg.as_ptr(),
                            msglen,
                            alg,
                        );
                        let tag = format!(
                            "core_h2c_string_to_hash(alg={alg},hlen={hlen},ctxlen={ctxlen},msglen={msglen})"
                        );
                        assert_eq!(a, b, "{tag} return");
                        assert_bytes_eq(&tag, &co, &ro);
                    }
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// secure memory API
// ---------------------------------------------------------------------------

#[test]
fn sodium_memory_api_matches() {
    unsafe {
        type FnMalloc = unsafe extern "C" fn(usize) -> *mut c_void;
        type FnAllocArray = unsafe extern "C" fn(usize, usize) -> *mut c_void;
        type FnFree = unsafe extern "C" fn(*mut c_void);
        type FnLock = unsafe extern "C" fn(*mut c_void, usize) -> c_int;
        type FnMprotect = unsafe extern "C" fn(*mut c_void) -> c_int;
        type FnStackzero = unsafe extern "C" fn(usize);
        type FnInit = unsafe extern "C" fn() -> c_int;

        let (cm, rm): (FnMalloc, FnMalloc) = pair("sodium_malloc");
        let (ca, ra): (FnAllocArray, FnAllocArray) = pair("sodium_allocarray");
        let (cf, rf): (FnFree, FnFree) = pair("sodium_free");
        let (cml, rml): (FnLock, FnLock) = pair("sodium_mlock");
        let (cmu, rmu): (FnLock, FnLock) = pair("sodium_munlock");
        let (cpn, rpn): (FnMprotect, FnMprotect) = pair("sodium_mprotect_noaccess");
        let (cpr, rpr): (FnMprotect, FnMprotect) = pair("sodium_mprotect_readonly");
        let (cpw, rpw): (FnMprotect, FnMprotect) = pair("sodium_mprotect_readwrite");
        let (csz, rsz): (FnStackzero, FnStackzero) = pair("sodium_stackzero");
        let (ci, ri): (FnInit, FnInit) = pair("sodium_init");

        // sodium_init is idempotent and must report the same thing in both
        assert_eq!(ci(), ri(), "sodium_init (second call) return");
        assert_eq!(ci(), ri(), "sodium_init (third call) return");

        for &size in &[0usize, 1, 7, 16, 4095, 4096, 4097, 100_000] {
            let cp = cm(size);
            let rp = rm(size);
            assert_eq!(cp.is_null(), rp.is_null(), "sodium_malloc({size}) null-ness");
            if cp.is_null() {
                continue;
            }
            // guarded allocations are readable/writable and start out as the
            // same canary byte pattern in both implementations
            let cs = std::slice::from_raw_parts(cp as *const u8, size);
            let rs = std::slice::from_raw_parts(rp as *const u8, size);
            assert_bytes_eq(&format!("sodium_malloc({size}) initial contents"), cs, rs);

            // write, then exercise the mprotect transitions
            std::ptr::write_bytes(cp as *mut u8, 0x5A, size);
            std::ptr::write_bytes(rp as *mut u8, 0x5A, size);
            assert_eq!(cpr(cp), rpr(rp), "sodium_mprotect_readonly({size}) return");
            let cs = std::slice::from_raw_parts(cp as *const u8, size);
            let rs = std::slice::from_raw_parts(rp as *const u8, size);
            assert_bytes_eq(&format!("sodium_malloc({size}) readonly contents"), cs, rs);
            assert_eq!(cpn(cp), rpn(rp), "sodium_mprotect_noaccess({size}) return");
            assert_eq!(cpw(cp), rpw(rp), "sodium_mprotect_readwrite({size}) return");
            let cs = std::slice::from_raw_parts(cp as *const u8, size);
            let rs = std::slice::from_raw_parts(rp as *const u8, size);
            assert_bytes_eq(&format!("sodium_malloc({size}) rw contents"), cs, rs);

            assert_eq!(cml(cp, size), rml(rp, size), "sodium_mlock({size}) return");
            assert_eq!(cmu(cp, size), rmu(rp, size), "sodium_munlock({size}) return");

            cf(cp);
            rf(rp);
        }

        // sodium_free(NULL) must be a no-op in both
        cf(std::ptr::null_mut());
        rf(std::ptr::null_mut());

        for &(count, size) in &[
            (0usize, 0usize),
            (1, 1),
            (10, 16),
            (1000, 32),
            (usize::MAX, 2), // overflow: must fail identically
            (2, usize::MAX),
        ] {
            let cp = ca(count, size);
            let rp = ra(count, size);
            assert_eq!(
                cp.is_null(),
                rp.is_null(),
                "sodium_allocarray({count},{size}) null-ness"
            );
            if !cp.is_null() {
                let n = count * size;
                let cs = std::slice::from_raw_parts(cp as *const u8, n);
                let rs = std::slice::from_raw_parts(rp as *const u8, n);
                assert_bytes_eq(
                    &format!("sodium_allocarray({count},{size}) contents"),
                    cs,
                    rs,
                );
                cf(cp);
                rf(rp);
            }
        }

        // sodium_stackzero has no observable output; just check both survive
        for &n in &[0usize, 1, 64, 1024] {
            csz(n);
            rsz(n);
        }

        // mlock/munlock on ordinary memory
        let mut buf = vec![0u8; 8192];
        assert_eq!(
            cml(buf.as_mut_ptr() as *mut c_void, buf.len()),
            rml(buf.as_mut_ptr() as *mut c_void, buf.len()),
            "sodium_mlock plain buffer"
        );
        assert_eq!(
            cmu(buf.as_mut_ptr() as *mut c_void, buf.len()),
            rmu(buf.as_mut_ptr() as *mut c_void, buf.len()),
            "sodium_munlock plain buffer"
        );

        // crit section enter/leave
        type FnCrit = unsafe extern "C" fn() -> c_int;
        let (cen, ren): (FnCrit, FnCrit) = pair("sodium_crit_enter");
        let (cle, rle): (FnCrit, FnCrit) = pair("sodium_crit_leave");
        assert_eq!(cen(), ren(), "sodium_crit_enter return");
        assert_eq!(cle(), rle(), "sodium_crit_leave return");
    }
}

// ---------------------------------------------------------------------------
// crypto_stream_chacha20_ietf_ext (extended-nonce entry points)
// ---------------------------------------------------------------------------

#[test]
fn crypto_stream_chacha20_ietf_ext_matches() {
    unsafe {
        type FnStream = unsafe extern "C" fn(*mut c_uchar, c_ulonglong, *const c_uchar, *const c_uchar) -> c_int;
        type FnXorIc = unsafe extern "C" fn(
            *mut c_uchar,
            *const c_uchar,
            c_ulonglong,
            *const c_uchar,
            u32,
            *const c_uchar,
        ) -> c_int;
        let (cs, rs): (FnStream, FnStream) = pair("crypto_stream_chacha20_ietf_ext");
        let (cx, rx): (FnXorIc, FnXorIc) = pair("crypto_stream_chacha20_ietf_ext_xor_ic");

        let mut rng = Rng::new(0xA500);
        let msg = rng.vec(3001);
        // ietf_ext takes the 12-byte IETF nonce
        let mut keys: Vec<Vec<u8>> = vec![vec![0u8; 32], vec![0xffu8; 32]];
        keys.push(rng.vec(32));
        let mut nonces: Vec<Vec<u8>> = vec![vec![0u8; 12], vec![0xffu8; 12]];
        nonces.push(rng.vec(12));

        for key in &keys {
            for nonce in &nonces {
                for &len in &[0usize, 1, 63, 64, 65, 127, 128, 129, 1000, 3000] {
                    let mut co = vec![0xAAu8; len + 8];
                    let mut ro = vec![0xAAu8; len + 8];
                    let a = cs(co.as_mut_ptr(), len as c_ulonglong, nonce.as_ptr(), key.as_ptr());
                    let b = rs(ro.as_mut_ptr(), len as c_ulonglong, nonce.as_ptr(), key.as_ptr());
                    assert_eq!(a, b, "chacha20_ietf_ext({len}) return");
                    assert_bytes_eq(&format!("chacha20_ietf_ext({len})"), &co, &ro);

                    // _ext_xor_ic performs no counter-overflow check, so the
                    // full uint32 range is legal here.
                    for &ic in &[0u32, 1, 0xffff, 0xffff_fffe, 0xffff_ffff] {
                        let mut co = vec![0xAAu8; len + 8];
                        let mut ro = vec![0xAAu8; len + 8];
                        let a = cx(
                            co.as_mut_ptr(),
                            msg.as_ptr(),
                            len as c_ulonglong,
                            nonce.as_ptr(),
                            ic,
                            key.as_ptr(),
                        );
                        let b = rx(
                            ro.as_mut_ptr(),
                            msg.as_ptr(),
                            len as c_ulonglong,
                            nonce.as_ptr(),
                            ic,
                            key.as_ptr(),
                        );
                        assert_eq!(a, b, "chacha20_ietf_ext_xor_ic(ic={ic},len={len}) return");
                        assert_bytes_eq(
                            &format!("chacha20_ietf_ext_xor_ic(ic={ic:#x},len={len})"),
                            &co,
                            &ro,
                        );
                    }
                }
            }
        }
    }
}
