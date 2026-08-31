//! BLAKE2b (crypto_generichash), KDFs (blake2b + HKDF-SHA-2) and the XOF
//! family (SHAKE / TurboSHAKE).
mod common;

use common::*;
use std::os::raw::{c_char, c_int, c_uchar, c_ulonglong, c_void};

type FnGh = unsafe extern "C" fn(
    *mut c_uchar,
    usize,
    *const c_uchar,
    c_ulonglong,
    *const c_uchar,
    usize,
) -> c_int;
type FnGhSaltPersonal = unsafe extern "C" fn(
    *mut c_uchar,
    usize,
    *const c_uchar,
    c_ulonglong,
    *const c_uchar,
    usize,
    *const c_uchar,
    *const c_uchar,
) -> c_int;
type FnGhInit = unsafe extern "C" fn(*mut c_void, *const c_uchar, usize, usize) -> c_int;
type FnGhInitSp = unsafe extern "C" fn(
    *mut c_void,
    *const c_uchar,
    usize,
    usize,
    *const c_uchar,
    *const c_uchar,
) -> c_int;
type FnGhUpdate = unsafe extern "C" fn(*mut c_void, *const c_uchar, c_ulonglong) -> c_int;
type FnGhFinal = unsafe extern "C" fn(*mut c_void, *mut c_uchar, usize) -> c_int;
type FnKeygen = unsafe extern "C" fn(*mut c_uchar);

fn generichash_suite(prefix: &str, has_salt_personal: bool) {
    for s in [
        "bytes_min",
        "bytes_max",
        "bytes",
        "keybytes_min",
        "keybytes_max",
        "keybytes",
        "statebytes",
    ] {
        cmp_size(&format!("{prefix}_{s}"));
    }
    if has_salt_personal {
        cmp_size(&format!("{prefix}_saltbytes"));
        cmp_size(&format!("{prefix}_personalbytes"));
    }
    unsafe {
        let (cmin, _): (FnSize, FnSize) = pair(&format!("{prefix}_bytes_min"));
        let (cmax, _): (FnSize, FnSize) = pair(&format!("{prefix}_bytes_max"));
        let (ckmin, _): (FnSize, FnSize) = pair(&format!("{prefix}_keybytes_min"));
        let (ckmax, _): (FnSize, FnSize) = pair(&format!("{prefix}_keybytes_max"));
        let (csb, _): (FnSize, FnSize) = pair(&format!("{prefix}_statebytes"));
        let omin = cmin();
        let omax = cmax();
        let kmin = ckmin();
        let kmax = ckmax();
        let sb = csb();

        let (c1, r1): (FnGh, FnGh) = pair(prefix);
        let (ci, ri): (FnGhInit, FnGhInit) = pair(&format!("{prefix}_init"));
        let (cu, ru): (FnGhUpdate, FnGhUpdate) = pair(&format!("{prefix}_update"));
        let (cf, rf): (FnGhFinal, FnGhFinal) = pair(&format!("{prefix}_final"));

        let mut rng = Rng::new(0x3000 + prefix.len() as u64);
        let msg = rng.vec(3001);
        let bigkey = rng.vec(kmax + 8);

        // outlen sweep, including out-of-range values which must fail identically
        let mut outlens: Vec<usize> = (0..=omax + 2).collect();
        outlens.push(0);
        outlens.dedup();
        // keylen sweep, including out-of-range
        let keylens: Vec<usize> = vec![0, kmin, 1, 16, 31, 32, 33, kmax, kmax + 1, kmax + 2];

        for &outlen in &outlens {
            for &keylen in &keylens {
                for &inlen in &[0usize, 1, 63, 64, 65, 127, 128, 129, 1000] {
                    let kptr = if keylen == 0 {
                        std::ptr::null()
                    } else {
                        bigkey.as_ptr()
                    };
                    let mut co = vec![0xAAu8; omax + 8];
                    let mut ro = vec![0xAAu8; omax + 8];
                    let a = c1(
                        co.as_mut_ptr(),
                        outlen,
                        msg.as_ptr(),
                        inlen as c_ulonglong,
                        kptr,
                        keylen,
                    );
                    let b = r1(
                        ro.as_mut_ptr(),
                        outlen,
                        msg.as_ptr(),
                        inlen as c_ulonglong,
                        kptr,
                        keylen,
                    );
                    let tag = format!("{prefix}(outlen={outlen},keylen={keylen},inlen={inlen})");
                    assert_eq!(a, b, "{tag} return");
                    assert_bytes_eq(&tag, &co, &ro);
                }
            }
        }

        // streaming, with state comparison at every step
        for &outlen in &[omin, 1, 16, 32, omax, 0, omax + 1] {
            for &keylen in &[0usize, 1, 32, kmax, kmax + 1] {
                let kptr = if keylen == 0 {
                    std::ptr::null()
                } else {
                    bigkey.as_ptr()
                };
                for &inlen in &[0usize, 1, 127, 128, 129, 256, 1000] {
                    for chunks in chunkings(inlen) {
                        let mut cst = AlignedBuf::new(sb, 0xA5);
                        let mut rst = AlignedBuf::new(sb, 0xA5);
                        let a = ci(cst.as_mut_ptr() as *mut c_void, kptr, keylen, outlen);
                        let b = ri(rst.as_mut_ptr() as *mut c_void, kptr, keylen, outlen);
                        let tag = format!("{prefix}_init(keylen={keylen},outlen={outlen})");
                        assert_eq!(a, b, "{tag} return");
                        assert_bytes_eq(&tag, cst.as_slice(), rst.as_slice());
                        if a != 0 {
                            continue;
                        }
                        let mut off = 0usize;
                        for &n in &chunks {
                            let a = cu(
                                cst.as_mut_ptr() as *mut c_void,
                                msg.as_ptr().add(off),
                                n as c_ulonglong,
                            );
                            let b = ru(
                                rst.as_mut_ptr() as *mut c_void,
                                msg.as_ptr().add(off),
                                n as c_ulonglong,
                            );
                            assert_eq!(a, b, "{prefix}_update return");
                            assert_bytes_eq(
                                &format!("{prefix} state keylen={keylen} inlen={inlen} chunk={n}"),
                                cst.as_slice(),
                                rst.as_slice(),
                            );
                            off += n;
                        }
                        let mut co = vec![0xAAu8; omax + 8];
                        let mut ro = vec![0xAAu8; omax + 8];
                        let a = cf(cst.as_mut_ptr() as *mut c_void, co.as_mut_ptr(), outlen);
                        let b = rf(rst.as_mut_ptr() as *mut c_void, ro.as_mut_ptr(), outlen);
                        assert_eq!(a, b, "{prefix}_final return");
                        assert_bytes_eq(
                            &format!(
                                "{prefix} streaming digest outlen={outlen} keylen={keylen} inlen={inlen} chunks={chunks:?}"
                            ),
                            &co,
                            &ro,
                        );
                        assert_bytes_eq(
                            &format!("{prefix} state after final"),
                            cst.as_slice(),
                            rst.as_slice(),
                        );
                        // mismatched final outlen must be rejected identically
                        let bad = if outlen == omax { omin } else { outlen + 1 };
                        let mut co2 = vec![0xAAu8; omax + 8];
                        let mut ro2 = vec![0xAAu8; omax + 8];
                        let a = cf(cst.as_mut_ptr() as *mut c_void, co2.as_mut_ptr(), bad);
                        let b = rf(rst.as_mut_ptr() as *mut c_void, ro2.as_mut_ptr(), bad);
                        assert_eq!(a, b, "{prefix}_final second-call return");
                        assert_bytes_eq(&format!("{prefix}_final second call"), &co2, &ro2);
                    }
                }
            }
        }

        if has_salt_personal {
            let (c, r): (FnGhSaltPersonal, FnGhSaltPersonal) =
                pair(&format!("{prefix}_salt_personal"));
            let (csalt, _): (FnSize, FnSize) = pair(&format!("{prefix}_saltbytes"));
            let (cpers, _): (FnSize, FnSize) = pair(&format!("{prefix}_personalbytes"));
            let saltb = csalt();
            let persb = cpers();
            let salts: Vec<Vec<u8>> = vec![vec![0u8; saltb], vec![0xffu8; saltb], rng.vec(saltb)];
            let perss: Vec<Vec<u8>> = vec![vec![0u8; persb], vec![0xffu8; persb], rng.vec(persb)];
            for &outlen in &[omin, 16, 32, omax, 0, omax + 1] {
                for &keylen in &[0usize, 32, kmax, kmax + 1] {
                    for salt in &salts {
                        for pers in &perss {
                            for sptr in [salt.as_ptr(), std::ptr::null()] {
                                for pptr in [pers.as_ptr(), std::ptr::null()] {
                                    for &inlen in &[0usize, 1, 128, 500] {
                                        let kptr = if keylen == 0 {
                                            std::ptr::null()
                                        } else {
                                            bigkey.as_ptr()
                                        };
                                        let mut co = vec![0xAAu8; omax + 8];
                                        let mut ro = vec![0xAAu8; omax + 8];
                                        let a = c(
                                            co.as_mut_ptr(),
                                            outlen,
                                            msg.as_ptr(),
                                            inlen as c_ulonglong,
                                            kptr,
                                            keylen,
                                            sptr,
                                            pptr,
                                        );
                                        let b = r(
                                            ro.as_mut_ptr(),
                                            outlen,
                                            msg.as_ptr(),
                                            inlen as c_ulonglong,
                                            kptr,
                                            keylen,
                                            sptr,
                                            pptr,
                                        );
                                        let tag = format!(
                                            "{prefix}_salt_personal(outlen={outlen},keylen={keylen},inlen={inlen},salt={},pers={})",
                                            !sptr.is_null(),
                                            !pptr.is_null()
                                        );
                                        assert_eq!(a, b, "{tag} return");
                                        assert_bytes_eq(&tag, &co, &ro);
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // init_salt_personal + streaming
            let (ci2, ri2): (FnGhInitSp, FnGhInitSp) =
                pair(&format!("{prefix}_init_salt_personal"));
            for &outlen in &[omin, 32, omax, 0, omax + 1] {
                for &keylen in &[0usize, 32, kmax + 1] {
                    for salt in salts.iter().take(2) {
                        for pers in perss.iter().take(2) {
                            for &inlen in &[0usize, 129, 300] {
                                let kptr = if keylen == 0 {
                                    std::ptr::null()
                                } else {
                                    bigkey.as_ptr()
                                };
                                let mut cst = AlignedBuf::new(sb, 0xA5);
                                let mut rst = AlignedBuf::new(sb, 0xA5);
                                let a = ci2(
                                    cst.as_mut_ptr() as *mut c_void,
                                    kptr,
                                    keylen,
                                    outlen,
                                    salt.as_ptr(),
                                    pers.as_ptr(),
                                );
                                let b = ri2(
                                    rst.as_mut_ptr() as *mut c_void,
                                    kptr,
                                    keylen,
                                    outlen,
                                    salt.as_ptr(),
                                    pers.as_ptr(),
                                );
                                assert_eq!(a, b, "{prefix}_init_salt_personal return");
                                assert_bytes_eq(
                                    &format!("{prefix}_init_salt_personal state"),
                                    cst.as_slice(),
                                    rst.as_slice(),
                                );
                                if a != 0 {
                                    continue;
                                }
                                cu(
                                    cst.as_mut_ptr() as *mut c_void,
                                    msg.as_ptr(),
                                    inlen as c_ulonglong,
                                );
                                ru(
                                    rst.as_mut_ptr() as *mut c_void,
                                    msg.as_ptr(),
                                    inlen as c_ulonglong,
                                );
                                let mut co = vec![0xAAu8; omax + 8];
                                let mut ro = vec![0xAAu8; omax + 8];
                                let a = cf(cst.as_mut_ptr() as *mut c_void, co.as_mut_ptr(), outlen);
                                let b = rf(rst.as_mut_ptr() as *mut c_void, ro.as_mut_ptr(), outlen);
                                assert_eq!(a, b, "{prefix} sp final return");
                                assert_bytes_eq(
                                    &format!("{prefix} sp streaming outlen={outlen} keylen={keylen} inlen={inlen}"),
                                    &co,
                                    &ro,
                                );
                            }
                        }
                    }
                }
            }
        }

        let (ckb, _): (FnSize, FnSize) = pair(&format!("{prefix}_keybytes"));
        let kb = ckb();
        let (ck, rk): (FnKeygen, FnKeygen) = pair(&format!("{prefix}_keygen"));
        for _ in 0..4 {
            let mut a = vec![0xAAu8; kb + 8];
            let mut b = vec![0xAAu8; kb + 8];
            det_reset();
            ck(a.as_mut_ptr());
            det_reset();
            rk(b.as_mut_ptr());
            assert_bytes_eq(&format!("{prefix}_keygen"), &a, &b);
        }
    }
}

#[test]
fn crypto_generichash_blake2b_matches() {
    generichash_suite("crypto_generichash_blake2b", true);
}

#[test]
fn crypto_generichash_generic_matches() {
    cmp_cstr("crypto_generichash_primitive");
    generichash_suite("crypto_generichash", false);
}

// ---------------------------------------------------------------------------
// crypto_kdf (blake2b)
// ---------------------------------------------------------------------------

type FnKdfDerive = unsafe extern "C" fn(
    *mut c_uchar,
    usize,
    u64,
    *const c_char,
    *const c_uchar,
) -> c_int;

fn kdf_suite(prefix: &str) {
    for s in ["bytes_min", "bytes_max", "contextbytes", "keybytes"] {
        cmp_size(&format!("{prefix}_{s}"));
    }
    unsafe {
        let (cmin, _): (FnSize, FnSize) = pair(&format!("{prefix}_bytes_min"));
        let (cmax, _): (FnSize, FnSize) = pair(&format!("{prefix}_bytes_max"));
        let (cctx, _): (FnSize, FnSize) = pair(&format!("{prefix}_contextbytes"));
        let (ckb, _): (FnSize, FnSize) = pair(&format!("{prefix}_keybytes"));
        let smin = cmin();
        let smax = cmax();
        let ctxb = cctx();
        let kb = ckb();

        let (c, r): (FnKdfDerive, FnKdfDerive) = pair(&format!("{prefix}_derive_from_key"));
        let mut rng = Rng::new(0x3100);
        let mut keys: Vec<Vec<u8>> = vec![vec![0u8; kb], vec![0xffu8; kb]];
        keys.push(rng.vec(kb));
        let mut ctxs: Vec<Vec<u8>> = vec![vec![0u8; ctxb], vec![0x41u8; ctxb]];
        ctxs.push(rng.vec(ctxb));

        let mut lens: Vec<usize> = (0..=smin + 2).collect();
        lens.extend([16usize, 32, 33, 63, 64, 65, smax, smax + 1]);
        lens.sort_unstable();
        lens.dedup();

        for key in &keys {
            for ctx in &ctxs {
                for &sl in &lens {
                    for &id in &[0u64, 1, 2, 0xff, 0xffff_ffff, u64::MAX, 0x0123_4567_89ab_cdef] {
                        let mut co = vec![0xAAu8; smax + 8];
                        let mut ro = vec![0xAAu8; smax + 8];
                        let a = c(
                            co.as_mut_ptr(),
                            sl,
                            id,
                            ctx.as_ptr() as *const c_char,
                            key.as_ptr(),
                        );
                        let b = r(
                            ro.as_mut_ptr(),
                            sl,
                            id,
                            ctx.as_ptr() as *const c_char,
                            key.as_ptr(),
                        );
                        let tag = format!("{prefix}_derive_from_key(len={sl},id={id})");
                        assert_eq!(a, b, "{tag} return");
                        assert_bytes_eq(&tag, &co, &ro);
                    }
                }
            }
        }

        if has(&format!("{prefix}_keygen")) {
            let (ck, rk): (FnKeygen, FnKeygen) = pair(&format!("{prefix}_keygen"));
            for _ in 0..4 {
                let mut a = vec![0xAAu8; kb + 8];
                let mut b = vec![0xAAu8; kb + 8];
                det_reset();
                ck(a.as_mut_ptr());
                det_reset();
                rk(b.as_mut_ptr());
                assert_bytes_eq(&format!("{prefix}_keygen"), &a, &b);
            }
        }
    }
}

#[test]
fn crypto_kdf_blake2b_matches() {
    kdf_suite("crypto_kdf_blake2b");
}

#[test]
fn crypto_kdf_generic_matches() {
    cmp_cstr("crypto_kdf_primitive");
    kdf_suite("crypto_kdf");
}

// ---------------------------------------------------------------------------
// HKDF-SHA-256 / HKDF-SHA-512
// ---------------------------------------------------------------------------

type FnHkdfExtract =
    unsafe extern "C" fn(*mut c_uchar, *const c_uchar, usize, *const c_uchar, usize) -> c_int;
type FnHkdfExpand =
    unsafe extern "C" fn(*mut c_uchar, usize, *const c_char, usize, *const c_uchar) -> c_int;
type FnHkdfExtractInit = unsafe extern "C" fn(*mut c_void, *const c_uchar, usize) -> c_int;
type FnHkdfExtractUpdate = unsafe extern "C" fn(*mut c_void, *const c_uchar, usize) -> c_int;
type FnHkdfExtractFinal = unsafe extern "C" fn(*mut c_void, *mut c_uchar) -> c_int;

fn hkdf_suite(prefix: &str) {
    for s in ["keybytes", "bytes_min", "bytes_max", "statebytes"] {
        cmp_size(&format!("{prefix}_{s}"));
    }
    unsafe {
        let (ckb, _): (FnSize, FnSize) = pair(&format!("{prefix}_keybytes"));
        let (cmax, _): (FnSize, FnSize) = pair(&format!("{prefix}_bytes_max"));
        let (csb, _): (FnSize, FnSize) = pair(&format!("{prefix}_statebytes"));
        let kb = ckb();
        let omax = cmax();
        let sb = csb();

        let (cex, rex): (FnHkdfExtract, FnHkdfExtract) = pair(&format!("{prefix}_extract"));
        let (cxp, rxp): (FnHkdfExpand, FnHkdfExpand) = pair(&format!("{prefix}_expand"));
        let (cii, rii): (FnHkdfExtractInit, FnHkdfExtractInit) =
            pair(&format!("{prefix}_extract_init"));
        let (ciu, riu): (FnHkdfExtractUpdate, FnHkdfExtractUpdate) =
            pair(&format!("{prefix}_extract_update"));
        let (cif, rif): (FnHkdfExtractFinal, FnHkdfExtractFinal) =
            pair(&format!("{prefix}_extract_final"));

        let mut rng = Rng::new(0x3200 + prefix.len() as u64);
        let ikm = rng.vec(600);
        let salt = rng.vec(600);

        for &saltlen in &[0usize, 1, 16, 32, 63, 64, 65, 127, 128, 129, 200, 500] {
            for &ikmlen in &[0usize, 1, 16, 32, 64, 65, 128, 200, 500] {
                let sptr = if saltlen == 0 {
                    std::ptr::null()
                } else {
                    salt.as_ptr()
                };
                let mut cprk = vec![0xAAu8; kb + 8];
                let mut rprk = vec![0xAAu8; kb + 8];
                let a = cex(cprk.as_mut_ptr(), sptr, saltlen, ikm.as_ptr(), ikmlen);
                let b = rex(rprk.as_mut_ptr(), sptr, saltlen, ikm.as_ptr(), ikmlen);
                let tag = format!("{prefix}_extract(salt={saltlen},ikm={ikmlen})");
                assert_eq!(a, b, "{tag} return");
                assert_bytes_eq(&tag, &cprk, &rprk);

                // streaming extract must agree with one-shot
                let mut cst = AlignedBuf::new(sb, 0xA5);
                let mut rst = AlignedBuf::new(sb, 0xA5);
                let a = cii(cst.as_mut_ptr() as *mut c_void, sptr, saltlen);
                let b = rii(rst.as_mut_ptr() as *mut c_void, sptr, saltlen);
                assert_eq!(a, b, "{prefix}_extract_init return");
                assert_bytes_eq(
                    &format!("{prefix}_extract_init state salt={saltlen}"),
                    cst.as_slice(),
                    rst.as_slice(),
                );
                let mut off = 0usize;
                for &n in &[ikmlen / 3, ikmlen - ikmlen / 3 - ikmlen / 3, ikmlen / 3] {
                    let a = ciu(cst.as_mut_ptr() as *mut c_void, ikm.as_ptr().add(off), n);
                    let b = riu(rst.as_mut_ptr() as *mut c_void, ikm.as_ptr().add(off), n);
                    assert_eq!(a, b, "{prefix}_extract_update return");
                    assert_bytes_eq(
                        &format!("{prefix}_extract_update state chunk={n}"),
                        cst.as_slice(),
                        rst.as_slice(),
                    );
                    off += n;
                }
                let mut cprk2 = vec![0xAAu8; kb + 8];
                let mut rprk2 = vec![0xAAu8; kb + 8];
                let a = cif(cst.as_mut_ptr() as *mut c_void, cprk2.as_mut_ptr());
                let b = rif(rst.as_mut_ptr() as *mut c_void, rprk2.as_mut_ptr());
                assert_eq!(a, b, "{prefix}_extract_final return");
                assert_bytes_eq(&format!("{prefix} streaming extract"), &cprk2, &rprk2);
                assert_eq!(&cprk2[..kb], &cprk[..kb], "{prefix} streaming != one-shot");
            }
        }

        // expand
        let prk = rng.vec(kb);
        let ctx = rng.vec(600);
        let mut lens: Vec<usize> = (0..=70).collect();
        lens.extend([100usize, 127, 128, 129, 255, 256, 300, 600, 1000, 8160, omax, omax + 1]);
        lens.sort_unstable();
        lens.dedup();
        for &outlen in &lens {
            for &ctxlen in &[0usize, 1, 8, 32, 200] {
                let cap = outlen.min(omax) + 16;
                let mut co = vec![0xAAu8; cap];
                let mut ro = vec![0xAAu8; cap];
                let a = cxp(
                    co.as_mut_ptr(),
                    outlen,
                    ctx.as_ptr() as *const c_char,
                    ctxlen,
                    prk.as_ptr(),
                );
                let b = rxp(
                    ro.as_mut_ptr(),
                    outlen,
                    ctx.as_ptr() as *const c_char,
                    ctxlen,
                    prk.as_ptr(),
                );
                let tag = format!("{prefix}_expand(outlen={outlen},ctxlen={ctxlen})");
                assert_eq!(a, b, "{tag} return");
                if a == 0 {
                    assert_bytes_eq(&tag, &co, &ro);
                }
            }
        }

        let (ck, rk): (FnKeygen, FnKeygen) = pair(&format!("{prefix}_keygen"));
        for _ in 0..4 {
            let mut a = vec![0xAAu8; kb + 8];
            let mut b = vec![0xAAu8; kb + 8];
            det_reset();
            ck(a.as_mut_ptr());
            det_reset();
            rk(b.as_mut_ptr());
            assert_bytes_eq(&format!("{prefix}_keygen"), &a, &b);
        }
    }
}

#[test]
fn crypto_kdf_hkdf_sha256_matches() {
    hkdf_suite("crypto_kdf_hkdf_sha256");
}

#[test]
fn crypto_kdf_hkdf_sha512_matches() {
    hkdf_suite("crypto_kdf_hkdf_sha512");
}

// ---------------------------------------------------------------------------
// XOF: SHAKE128/256, TurboSHAKE128/256
// ---------------------------------------------------------------------------

type FnXof = unsafe extern "C" fn(*mut c_uchar, usize, *const c_uchar, c_ulonglong) -> c_int;
type FnXofInit = unsafe extern "C" fn(*mut c_void) -> c_int;
type FnXofInitDomain = unsafe extern "C" fn(*mut c_void, c_uchar) -> c_int;
type FnXofUpdate = unsafe extern "C" fn(*mut c_void, *const c_uchar, c_ulonglong) -> c_int;
type FnXofSqueeze = unsafe extern "C" fn(*mut c_void, *mut c_uchar, usize) -> c_int;
type FnDomain = unsafe extern "C" fn() -> c_uchar;

fn xof_suite(prefix: &str) {
    cmp_size(&format!("{prefix}_blockbytes"));
    cmp_size(&format!("{prefix}_statebytes"));
    unsafe {
        let (cdom, rdom): (FnDomain, FnDomain) = pair(&format!("{prefix}_domain_standard"));
        assert_eq!(cdom(), rdom(), "{prefix}_domain_standard");
        let (cbb, _): (FnSize, FnSize) = pair(&format!("{prefix}_blockbytes"));
        let (csb, _): (FnSize, FnSize) = pair(&format!("{prefix}_statebytes"));
        let bb = cbb();
        let sb = csb();

        let (c1, r1): (FnXof, FnXof) = pair(prefix);
        let (ci, ri): (FnXofInit, FnXofInit) = pair(&format!("{prefix}_init"));
        let (cid, rid): (FnXofInitDomain, FnXofInitDomain) =
            pair(&format!("{prefix}_init_with_domain"));
        let (cu, ru): (FnXofUpdate, FnXofUpdate) = pair(&format!("{prefix}_update"));
        let (cs, rs): (FnXofSqueeze, FnXofSqueeze) = pair(&format!("{prefix}_squeeze"));

        let mut rng = Rng::new(0x3300 + prefix.len() as u64);
        let msg = rng.vec(3001);

        // one-shot with a wide range of output lengths, crossing block boundaries
        let mut outlens: Vec<usize> = (0..=70).collect();
        outlens.extend([
            bb - 1,
            bb,
            bb + 1,
            2 * bb - 1,
            2 * bb,
            2 * bb + 1,
            3 * bb,
            500,
            1000,
        ]);
        outlens.sort_unstable();
        outlens.dedup();
        for &outlen in &outlens {
            for &inlen in &[0usize, 1, bb - 1, bb, bb + 1, 2 * bb, 200, 1000] {
                let mut co = vec![0xAAu8; outlen + 8];
                let mut ro = vec![0xAAu8; outlen + 8];
                let a = c1(co.as_mut_ptr(), outlen, msg.as_ptr(), inlen as c_ulonglong);
                let b = r1(ro.as_mut_ptr(), outlen, msg.as_ptr(), inlen as c_ulonglong);
                let tag = format!("{prefix}(outlen={outlen},inlen={inlen})");
                assert_eq!(a, b, "{tag} return");
                assert_bytes_eq(&tag, &co, &ro);
            }
        }

        // streaming: init / update* / squeeze*
        for &inlen in &[0usize, 1, bb - 1, bb, bb + 1, 2 * bb + 5, 500] {
            for chunks in chunkings(inlen) {
                let mut cst = AlignedBuf::new(sb, 0xA5);
                let mut rst = AlignedBuf::new(sb, 0xA5);
                let a = ci(cst.as_mut_ptr() as *mut c_void);
                let b = ri(rst.as_mut_ptr() as *mut c_void);
                assert_eq!(a, b, "{prefix}_init return");
                assert_bytes_eq(&format!("{prefix}_init state"), cst.as_slice(), rst.as_slice());
                let mut off = 0usize;
                for &n in &chunks {
                    let a = cu(
                        cst.as_mut_ptr() as *mut c_void,
                        msg.as_ptr().add(off),
                        n as c_ulonglong,
                    );
                    let b = ru(
                        rst.as_mut_ptr() as *mut c_void,
                        msg.as_ptr().add(off),
                        n as c_ulonglong,
                    );
                    assert_eq!(a, b, "{prefix}_update return");
                    assert_bytes_eq(
                        &format!("{prefix} state inlen={inlen} chunk={n}"),
                        cst.as_slice(),
                        rst.as_slice(),
                    );
                    off += n;
                }
                // multiple squeezes of varying size must be identical, and must
                // concatenate to the one-shot output
                let mut cacc = Vec::new();
                let mut racc = Vec::new();
                for &sq in &[1usize, 0, bb - 1, 3, bb + 2, 17, 2 * bb] {
                    let mut co = vec![0xAAu8; sq + 8];
                    let mut ro = vec![0xAAu8; sq + 8];
                    let a = cs(cst.as_mut_ptr() as *mut c_void, co.as_mut_ptr(), sq);
                    let b = rs(rst.as_mut_ptr() as *mut c_void, ro.as_mut_ptr(), sq);
                    assert_eq!(a, b, "{prefix}_squeeze return sq={sq}");
                    assert_bytes_eq(
                        &format!("{prefix} squeeze inlen={inlen} sq={sq} chunks={chunks:?}"),
                        &co,
                        &ro,
                    );
                    assert_bytes_eq(
                        &format!("{prefix} state after squeeze sq={sq}"),
                        cst.as_slice(),
                        rst.as_slice(),
                    );
                    cacc.extend_from_slice(&co[..sq]);
                    racc.extend_from_slice(&ro[..sq]);
                }
                assert_bytes_eq(&format!("{prefix} squeeze accumulation"), &cacc, &racc);
                // must equal one-shot output of the same length
                let mut one = vec![0u8; cacc.len()];
                c1(one.as_mut_ptr(), cacc.len(), msg.as_ptr(), inlen as c_ulonglong);
                assert_eq!(&one, &cacc, "{prefix} streaming != one-shot inlen={inlen}");
            }
        }

        // init_with_domain over every possible domain byte
        for dom in 0u16..=255 {
            let dom = dom as u8;
            let mut cst = AlignedBuf::new(sb, 0xA5);
            let mut rst = AlignedBuf::new(sb, 0xA5);
            let a = cid(cst.as_mut_ptr() as *mut c_void, dom);
            let b = rid(rst.as_mut_ptr() as *mut c_void, dom);
            assert_eq!(a, b, "{prefix}_init_with_domain({dom}) return");
            assert_bytes_eq(
                &format!("{prefix}_init_with_domain({dom}) state"),
                cst.as_slice(),
                rst.as_slice(),
            );
            if a != 0 {
                continue;
            }
            cu(cst.as_mut_ptr() as *mut c_void, msg.as_ptr(), 300);
            ru(rst.as_mut_ptr() as *mut c_void, msg.as_ptr(), 300);
            let mut co = vec![0xAAu8; 200];
            let mut ro = vec![0xAAu8; 200];
            let a = cs(cst.as_mut_ptr() as *mut c_void, co.as_mut_ptr(), 192);
            let b = rs(rst.as_mut_ptr() as *mut c_void, ro.as_mut_ptr(), 192);
            assert_eq!(a, b, "{prefix} domain {dom} squeeze return");
            assert_bytes_eq(&format!("{prefix} domain {dom} output"), &co, &ro);
        }
    }
}

#[test]
fn crypto_xof_shake128_matches() {
    xof_suite("crypto_xof_shake128");
}

#[test]
fn crypto_xof_shake256_matches() {
    xof_suite("crypto_xof_shake256");
}

#[test]
fn crypto_xof_turboshake128_matches() {
    xof_suite("crypto_xof_turboshake128");
}

#[test]
fn crypto_xof_turboshake256_matches() {
    xof_suite("crypto_xof_turboshake256");
}
