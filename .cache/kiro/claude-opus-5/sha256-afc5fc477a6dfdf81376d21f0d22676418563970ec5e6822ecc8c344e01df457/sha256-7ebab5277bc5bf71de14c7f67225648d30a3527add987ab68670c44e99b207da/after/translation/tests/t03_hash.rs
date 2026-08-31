//! Hash and MAC layer: sha256, sha512, sha3-256/512, poly1305, HMAC-SHA-2
//! variants and the `crypto_hash` / `crypto_auth` / `crypto_onetimeauth`
//! generic front ends.
mod common;

use common::*;
use std::os::raw::{c_int, c_uchar, c_ulonglong, c_void};

type FnHash3 = unsafe extern "C" fn(*mut c_uchar, *const c_uchar, c_ulonglong) -> c_int;
type FnStInit = unsafe extern "C" fn(*mut c_void) -> c_int;
type FnStUpdate = unsafe extern "C" fn(*mut c_void, *const c_uchar, c_ulonglong) -> c_int;
type FnStFinal = unsafe extern "C" fn(*mut c_void, *mut c_uchar) -> c_int;

/// One-shot + streaming equivalence for an unkeyed hash.
fn hash_suite(prefix: &str, lens: &[usize]) {
    cmp_size(&format!("{prefix}_bytes"));
    cmp_size(&format!("{prefix}_statebytes"));
    unsafe {
        let (cbytes, _): (FnSize, FnSize) = pair(&format!("{prefix}_bytes"));
        let (csb, _): (FnSize, FnSize) = pair(&format!("{prefix}_statebytes"));
        let ob = cbytes();
        let sb = csb();
        let (c1, r1): (FnHash3, FnHash3) = pair(prefix);
        let (ci, ri): (FnStInit, FnStInit) = pair(&format!("{prefix}_init"));
        let (cu, ru): (FnStUpdate, FnStUpdate) = pair(&format!("{prefix}_update"));
        let (cf, rf): (FnStFinal, FnStFinal) = pair(&format!("{prefix}_final"));

        let mut rng = Rng::new(0x1000 + prefix.len() as u64);
        let maxlen = *lens.iter().max().unwrap();
        let msg = rng.vec(maxlen + 1);

        for &len in lens {
            // one-shot
            let mut co = vec![0xAAu8; ob + 8];
            let mut ro = vec![0xAAu8; ob + 8];
            let cr = c1(co.as_mut_ptr(), msg.as_ptr(), len as c_ulonglong);
            let rr = r1(ro.as_mut_ptr(), msg.as_ptr(), len as c_ulonglong);
            assert_eq!(cr, rr, "{prefix} one-shot return len {len}");
            assert_bytes_eq(&format!("{prefix} one-shot len {len}"), &co, &ro);

            // NULL input for zero length
            if len == 0 {
                let mut co2 = vec![0xAAu8; ob + 8];
                let mut ro2 = vec![0xAAu8; ob + 8];
                c1(co2.as_mut_ptr(), std::ptr::null(), 0);
                r1(ro2.as_mut_ptr(), std::ptr::null(), 0);
                assert_bytes_eq(&format!("{prefix} one-shot NULL/0"), &co2, &ro2);
            }

            // streaming, over several chunkings
            for chunks in chunkings(len) {
                let mut cst = AlignedBuf::new(sb, 0xA5);
                let mut rst = AlignedBuf::new(sb, 0xA5);
                let cir = ci(cst.as_mut_ptr() as *mut c_void);
                let rir = ri(rst.as_mut_ptr() as *mut c_void);
                assert_eq!(cir, rir, "{prefix}_init return");
                assert_bytes_eq(&format!("{prefix} state after init"), cst.as_slice(), rst.as_slice());

                let mut off = 0usize;
                for (i, &n) in chunks.iter().enumerate() {
                    let cur = cu(
                        cst.as_mut_ptr() as *mut c_void,
                        msg.as_ptr().add(off),
                        n as c_ulonglong,
                    );
                    let rur = ru(
                        rst.as_mut_ptr() as *mut c_void,
                        msg.as_ptr().add(off),
                        n as c_ulonglong,
                    );
                    assert_eq!(cur, rur, "{prefix}_update return len {len} chunk {i}");
                    assert_bytes_eq(
                        &format!("{prefix} state after update len {len} chunk {i} ({n} bytes)"),
                        cst.as_slice(),
                        rst.as_slice(),
                    );
                    off += n;
                }
                assert_eq!(off, len);

                let mut co2 = vec![0xAAu8; ob + 8];
                let mut ro2 = vec![0xAAu8; ob + 8];
                let cfr = cf(cst.as_mut_ptr() as *mut c_void, co2.as_mut_ptr());
                let rfr = rf(rst.as_mut_ptr() as *mut c_void, ro2.as_mut_ptr());
                assert_eq!(cfr, rfr, "{prefix}_final return len {len}");
                assert_bytes_eq(
                    &format!("{prefix} streaming digest len {len} chunks {chunks:?}"),
                    &co2,
                    &ro2,
                );
                assert_bytes_eq(
                    &format!("{prefix} state after final len {len}"),
                    cst.as_slice(),
                    rst.as_slice(),
                );
                assert_eq!(&co2[..ob], &co[..ob], "{prefix} streaming != one-shot len {len}");
            }
        }
    }
}

#[test]
fn crypto_hash_sha256_matches() {
    hash_suite("crypto_hash_sha256", &msg_lens());
}

#[test]
fn crypto_hash_sha512_matches() {
    hash_suite("crypto_hash_sha512", &msg_lens());
}

#[test]
fn crypto_hash_sha3256_matches() {
    hash_suite("crypto_hash_sha3256", &msg_lens_small());
}

#[test]
fn crypto_hash_sha3512_matches() {
    hash_suite("crypto_hash_sha3512", &msg_lens_small());
}

#[test]
fn crypto_hash_generic_matches() {
    cmp_size("crypto_hash_bytes");
    cmp_cstr("crypto_hash_primitive");
    unsafe {
        let (cbytes, _): (FnSize, FnSize) = pair("crypto_hash_bytes");
        let ob = cbytes();
        let (c, r): (FnHash3, FnHash3) = pair("crypto_hash");
        let mut rng = Rng::new(0x1100);
        let msg = rng.vec(5001);
        for len in msg_lens() {
            let mut co = vec![0xAAu8; ob + 8];
            let mut ro = vec![0xAAu8; ob + 8];
            let cr = c(co.as_mut_ptr(), msg.as_ptr(), len as c_ulonglong);
            let rr = r(ro.as_mut_ptr(), msg.as_ptr(), len as c_ulonglong);
            assert_eq!(cr, rr, "crypto_hash return len {len}");
            assert_bytes_eq(&format!("crypto_hash len {len}"), &co, &ro);
        }
    }
}

// ---------------------------------------------------------------------------
// poly1305 / crypto_onetimeauth
// ---------------------------------------------------------------------------

type FnMac = unsafe extern "C" fn(*mut c_uchar, *const c_uchar, c_ulonglong, *const c_uchar) -> c_int;
type FnMacVerify =
    unsafe extern "C" fn(*const c_uchar, *const c_uchar, c_ulonglong, *const c_uchar) -> c_int;
type FnOtaInit = unsafe extern "C" fn(*mut c_void, *const c_uchar) -> c_int;
type FnKeygen = unsafe extern "C" fn(*mut c_uchar);

fn onetimeauth_suite(prefix: &str) {
    for s in ["bytes", "keybytes", "statebytes"] {
        cmp_size(&format!("{prefix}_{s}"));
    }
    unsafe {
        let (cb, _): (FnSize, FnSize) = pair(&format!("{prefix}_bytes"));
        let (ckb, _): (FnSize, FnSize) = pair(&format!("{prefix}_keybytes"));
        let (csb, _): (FnSize, FnSize) = pair(&format!("{prefix}_statebytes"));
        let ob = cb();
        let kb = ckb();
        let sb = csb();

        let (c1, r1): (FnMac, FnMac) = pair(prefix);
        let (cv, rv): (FnMacVerify, FnMacVerify) = pair(&format!("{prefix}_verify"));
        let (ci, ri): (FnOtaInit, FnOtaInit) = pair(&format!("{prefix}_init"));
        let (cu, ru): (FnStUpdate, FnStUpdate) = pair(&format!("{prefix}_update"));
        let (cf, rf): (FnStFinal, FnStFinal) = pair(&format!("{prefix}_final"));

        let mut rng = Rng::new(0x1200 + prefix.len() as u64);
        let msg = rng.vec(5001);
        let mut keys: Vec<Vec<u8>> = vec![vec![0u8; kb], vec![0xffu8; kb]];
        for _ in 0..3 {
            keys.push(rng.vec(kb));
        }

        for key in &keys {
            for len in msg_lens_small() {
                let mut co = vec![0xAAu8; ob + 8];
                let mut ro = vec![0xAAu8; ob + 8];
                let cr = c1(co.as_mut_ptr(), msg.as_ptr(), len as c_ulonglong, key.as_ptr());
                let rr = r1(ro.as_mut_ptr(), msg.as_ptr(), len as c_ulonglong, key.as_ptr());
                assert_eq!(cr, rr, "{prefix} return len {len}");
                assert_bytes_eq(&format!("{prefix} len {len} key {}", hex(key)), &co, &ro);

                // verify: correct tag, and each single-bit-flipped tag
                assert_eq!(
                    cv(co.as_ptr(), msg.as_ptr(), len as c_ulonglong, key.as_ptr()),
                    rv(co.as_ptr(), msg.as_ptr(), len as c_ulonglong, key.as_ptr()),
                    "{prefix}_verify good len {len}"
                );
                for bit in [0usize, 7, 8, ob * 8 - 1] {
                    let mut bad = co.clone();
                    bad[bit / 8] ^= 1 << (bit % 8);
                    assert_eq!(
                        cv(bad.as_ptr(), msg.as_ptr(), len as c_ulonglong, key.as_ptr()),
                        rv(bad.as_ptr(), msg.as_ptr(), len as c_ulonglong, key.as_ptr()),
                        "{prefix}_verify bad len {len} bit {bit}"
                    );
                }

                for chunks in chunkings(len) {
                    let mut cst = AlignedBuf::new(sb, 0xA5);
                    let mut rst = AlignedBuf::new(sb, 0xA5);
                    let a = ci(cst.as_mut_ptr() as *mut c_void, key.as_ptr());
                    let b = ri(rst.as_mut_ptr() as *mut c_void, key.as_ptr());
                    assert_eq!(a, b, "{prefix}_init return");
                    assert_bytes_eq(
                        &format!("{prefix} state after init"),
                        cst.as_slice(),
                        rst.as_slice(),
                    );
                    let mut off = 0usize;
                    for (i, &n) in chunks.iter().enumerate() {
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
                            &format!("{prefix} state after update len {len} chunk {i} ({n})"),
                            cst.as_slice(),
                            rst.as_slice(),
                        );
                        off += n;
                    }
                    let mut co2 = vec![0xAAu8; ob + 8];
                    let mut ro2 = vec![0xAAu8; ob + 8];
                    let a = cf(cst.as_mut_ptr() as *mut c_void, co2.as_mut_ptr());
                    let b = rf(rst.as_mut_ptr() as *mut c_void, ro2.as_mut_ptr());
                    assert_eq!(a, b, "{prefix}_final return");
                    assert_bytes_eq(
                        &format!("{prefix} streaming tag len {len} chunks {chunks:?}"),
                        &co2,
                        &ro2,
                    );
                    assert_bytes_eq(
                        &format!("{prefix} state after final"),
                        cst.as_slice(),
                        rst.as_slice(),
                    );
                    assert_eq!(&co2[..ob], &co[..ob], "{prefix} streaming != one-shot len {len}");
                }
            }
        }

        // keygen through the shared deterministic RNG
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
fn crypto_onetimeauth_poly1305_matches() {
    onetimeauth_suite("crypto_onetimeauth_poly1305");
}

#[test]
fn crypto_onetimeauth_generic_matches() {
    cmp_cstr("crypto_onetimeauth_primitive");
    onetimeauth_suite("crypto_onetimeauth");
}

// ---------------------------------------------------------------------------
// HMAC-SHA-2 (crypto_auth_hmacsha256/512/512256) + crypto_auth front end
// ---------------------------------------------------------------------------

type FnHmacInit = unsafe extern "C" fn(*mut c_void, *const c_uchar, usize) -> c_int;

fn hmac_suite(prefix: &str) {
    for s in ["bytes", "keybytes", "statebytes"] {
        cmp_size(&format!("{prefix}_{s}"));
    }
    unsafe {
        let (cb, _): (FnSize, FnSize) = pair(&format!("{prefix}_bytes"));
        let (ckb, _): (FnSize, FnSize) = pair(&format!("{prefix}_keybytes"));
        let (csb, _): (FnSize, FnSize) = pair(&format!("{prefix}_statebytes"));
        let ob = cb();
        let kb = ckb();
        let sb = csb();

        let (c1, r1): (FnMac, FnMac) = pair(prefix);
        let (cv, rv): (FnMacVerify, FnMacVerify) = pair(&format!("{prefix}_verify"));
        let (ci, ri): (FnHmacInit, FnHmacInit) = pair(&format!("{prefix}_init"));
        let (cu, ru): (FnStUpdate, FnStUpdate) = pair(&format!("{prefix}_update"));
        let (cf, rf): (FnStFinal, FnStFinal) = pair(&format!("{prefix}_final"));

        let mut rng = Rng::new(0x1300 + prefix.len() as u64);
        let msg = rng.vec(3001);

        // fixed-length key API
        let mut keys: Vec<Vec<u8>> = vec![vec![0u8; kb], vec![0xffu8; kb]];
        for _ in 0..3 {
            keys.push(rng.vec(kb));
        }
        for key in &keys {
            for len in msg_lens_small() {
                let mut co = vec![0xAAu8; ob + 8];
                let mut ro = vec![0xAAu8; ob + 8];
                let a = c1(co.as_mut_ptr(), msg.as_ptr(), len as c_ulonglong, key.as_ptr());
                let b = r1(ro.as_mut_ptr(), msg.as_ptr(), len as c_ulonglong, key.as_ptr());
                assert_eq!(a, b, "{prefix} return len {len}");
                assert_bytes_eq(&format!("{prefix} len {len}"), &co, &ro);

                assert_eq!(
                    cv(co.as_ptr(), msg.as_ptr(), len as c_ulonglong, key.as_ptr()),
                    rv(co.as_ptr(), msg.as_ptr(), len as c_ulonglong, key.as_ptr()),
                    "{prefix}_verify good len {len}"
                );
                let mut bad = co.clone();
                bad[0] ^= 1;
                assert_eq!(
                    cv(bad.as_ptr(), msg.as_ptr(), len as c_ulonglong, key.as_ptr()),
                    rv(bad.as_ptr(), msg.as_ptr(), len as c_ulonglong, key.as_ptr()),
                    "{prefix}_verify bad len {len}"
                );
            }
        }

        // variable-length key streaming API: key shorter than, equal to and
        // longer than the hash block size (the long case is hashed first).
        let keylens: Vec<usize> = vec![
            0, 1, 15, 16, 31, 32, 33, 63, 64, 65, 100, 127, 128, 129, 200, 256,
        ];
        for kl in keylens {
            let key = rng.vec(kl.max(1));
            for len in [0usize, 1, 32, 63, 64, 65, 128, 200, 1000] {
                for chunks in chunkings(len) {
                    let mut cst = AlignedBuf::new(sb, 0xA5);
                    let mut rst = AlignedBuf::new(sb, 0xA5);
                    let a = ci(cst.as_mut_ptr() as *mut c_void, key.as_ptr(), kl);
                    let b = ri(rst.as_mut_ptr() as *mut c_void, key.as_ptr(), kl);
                    assert_eq!(a, b, "{prefix}_init return keylen {kl}");
                    assert_bytes_eq(
                        &format!("{prefix} state after init keylen {kl}"),
                        cst.as_slice(),
                        rst.as_slice(),
                    );
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
                            &format!("{prefix} state keylen {kl} len {len} chunk {n}"),
                            cst.as_slice(),
                            rst.as_slice(),
                        );
                        off += n;
                    }
                    let mut co = vec![0xAAu8; ob + 8];
                    let mut ro = vec![0xAAu8; ob + 8];
                    let a = cf(cst.as_mut_ptr() as *mut c_void, co.as_mut_ptr());
                    let b = rf(rst.as_mut_ptr() as *mut c_void, ro.as_mut_ptr());
                    assert_eq!(a, b, "{prefix}_final return");
                    assert_bytes_eq(
                        &format!("{prefix} tag keylen {kl} len {len} chunks {chunks:?}"),
                        &co,
                        &ro,
                    );
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
fn crypto_auth_hmacsha256_matches() {
    hmac_suite("crypto_auth_hmacsha256");
}

#[test]
fn crypto_auth_hmacsha512_matches() {
    hmac_suite("crypto_auth_hmacsha512");
}

#[test]
fn crypto_auth_hmacsha512256_matches() {
    hmac_suite("crypto_auth_hmacsha512256");
}

#[test]
fn crypto_auth_generic_matches() {
    cmp_cstr("crypto_auth_primitive");
    for s in ["bytes", "keybytes"] {
        cmp_size(&format!("crypto_auth_{s}"));
    }
    unsafe {
        let (cb, _): (FnSize, FnSize) = pair("crypto_auth_bytes");
        let (ckb, _): (FnSize, FnSize) = pair("crypto_auth_keybytes");
        let ob = cb();
        let kb = ckb();
        let (c1, r1): (FnMac, FnMac) = pair("crypto_auth");
        let (cv, rv): (FnMacVerify, FnMacVerify) = pair("crypto_auth_verify");
        let mut rng = Rng::new(0x1400);
        let msg = rng.vec(3001);
        for _ in 0..4 {
            let key = rng.vec(kb);
            for len in msg_lens_small() {
                let mut co = vec![0xAAu8; ob + 8];
                let mut ro = vec![0xAAu8; ob + 8];
                let a = c1(co.as_mut_ptr(), msg.as_ptr(), len as c_ulonglong, key.as_ptr());
                let b = r1(ro.as_mut_ptr(), msg.as_ptr(), len as c_ulonglong, key.as_ptr());
                assert_eq!(a, b, "crypto_auth return len {len}");
                assert_bytes_eq(&format!("crypto_auth len {len}"), &co, &ro);
                assert_eq!(
                    cv(co.as_ptr(), msg.as_ptr(), len as c_ulonglong, key.as_ptr()),
                    rv(co.as_ptr(), msg.as_ptr(), len as c_ulonglong, key.as_ptr()),
                    "crypto_auth_verify len {len}"
                );
            }
        }
        let (ck, rk): (FnKeygen, FnKeygen) = pair("crypto_auth_keygen");
        for _ in 0..4 {
            let mut a = vec![0xAAu8; kb + 8];
            let mut b = vec![0xAAu8; kb + 8];
            det_reset();
            ck(a.as_mut_ptr());
            det_reset();
            rk(b.as_mut_ptr());
            assert_bytes_eq("crypto_auth_keygen", &a, &b);
        }
    }
}
