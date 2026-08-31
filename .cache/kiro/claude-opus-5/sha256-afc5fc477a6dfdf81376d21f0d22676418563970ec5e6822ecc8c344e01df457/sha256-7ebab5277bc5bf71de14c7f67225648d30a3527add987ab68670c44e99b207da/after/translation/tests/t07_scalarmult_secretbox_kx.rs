//! Scalar multiplication (curve25519 / ed25519 / ristretto255), secretbox
//! (xsalsa20poly1305 + xchacha20poly1305, easy/detached/NaCl APIs) and
//! crypto_kx key exchange.
mod common;

use common::*;
use std::os::raw::{c_int, c_uchar, c_ulonglong};

type FnSm = unsafe extern "C" fn(*mut c_uchar, *const c_uchar, *const c_uchar) -> c_int;
type FnSmBase = unsafe extern "C" fn(*mut c_uchar, *const c_uchar) -> c_int;
type FnKeygen = unsafe extern "C" fn(*mut c_uchar);

/// Scalars covering zero, one, the group order, low-order edge cases and
/// clamping-relevant bit patterns.
fn sm_scalars(sz: usize, seed: u64) -> Vec<Vec<u8>> {
    let mut v: Vec<Vec<u8>> = vec![vec![0u8; sz], vec![0xffu8; sz]];
    for i in [0usize, 1, sz - 1] {
        let mut s = vec![0u8; sz];
        s[i] = 1;
        v.push(s);
        let mut s = vec![0u8; sz];
        s[i] = 0x80;
        v.push(s);
    }
    // ed25519 group order L and L-1 / L+1
    let l: [u8; 32] = [
        0xed, 0xd3, 0xf5, 0x5c, 0x1a, 0x63, 0x12, 0x58, 0xd6, 0x9c, 0xf7, 0xa2, 0xde, 0xf9, 0xde,
        0x14, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x10,
    ];
    for delta in [-1i32, 0, 1] {
        let mut s = vec![0u8; sz];
        for (i, b) in l.iter().enumerate() {
            if i < sz {
                s[i] = *b;
            }
        }
        if sz > 0 {
            s[0] = (s[0] as i32 + delta) as u8;
        }
        v.push(s);
    }
    let mut rng = Rng::new(seed);
    for _ in 0..24 {
        v.push(rng.vec(sz));
    }
    v
}

/// Group elements: zero, small-order curve25519 points, random bytes and
/// values produced by `*_base` so at least some inputs are valid.
fn sm_points(prefix: &str, sz: usize, scalars: &[Vec<u8>], seed: u64) -> Vec<Vec<u8>> {
    let mut v: Vec<Vec<u8>> = vec![vec![0u8; sz], vec![0xffu8; sz]];
    for i in [0usize, 1, sz - 1] {
        let mut p = vec![0u8; sz];
        p[i] = 1;
        v.push(p);
    }
    // curve25519 small-order points from the libsodium blacklist
    let small: [[u8; 32]; 7] = [
        [0; 32],
        {
            let mut a = [0u8; 32];
            a[0] = 1;
            a
        },
        [
            0xe0, 0xeb, 0x7a, 0x7c, 0x3b, 0x41, 0xb8, 0xae, 0x16, 0x56, 0xe3, 0xfa, 0xf1, 0x9f,
            0xc4, 0x6a, 0xda, 0x09, 0x8d, 0xeb, 0x9c, 0x32, 0xb1, 0xfd, 0x86, 0x62, 0x05, 0x16,
            0x5f, 0x49, 0xb8, 0x00,
        ],
        [
            0x5f, 0x9c, 0x95, 0xbc, 0xa3, 0x50, 0x8c, 0x24, 0xb1, 0xd0, 0xb1, 0x55, 0x9c, 0x83,
            0xef, 0x5b, 0x04, 0x44, 0x5c, 0xc4, 0x58, 0x1c, 0x8e, 0x86, 0xd8, 0x22, 0x4e, 0xdd,
            0xd0, 0x9f, 0x11, 0x57,
        ],
        [
            0xec, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
            0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
            0xff, 0xff, 0xff, 0x7f,
        ],
        [
            0xed, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
            0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
            0xff, 0xff, 0xff, 0x7f,
        ],
        [
            0xee, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
            0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
            0xff, 0xff, 0xff, 0x7f,
        ],
    ];
    if sz == 32 {
        for s in &small {
            v.push(s.to_vec());
            // and with the high bit set (ignored by X25519)
            let mut t = s.to_vec();
            t[31] |= 0x80;
            v.push(t);
        }
    }
    // real points from _base
    unsafe {
        if has(&format!("{prefix}_base")) {
            let (cbase, _): (FnSmBase, FnSmBase) = pair(&format!("{prefix}_base"));
            for s in scalars.iter().take(12) {
                let mut p = vec![0u8; sz];
                if cbase(p.as_mut_ptr(), s.as_ptr()) == 0 {
                    v.push(p);
                }
            }
        }
    }
    let mut rng = Rng::new(seed);
    for _ in 0..24 {
        v.push(rng.vec(sz));
    }
    v
}

fn scalarmult_suite(prefix: &str, extra: &[&str]) {
    cmp_size(&format!("{prefix}_bytes"));
    cmp_size(&format!("{prefix}_scalarbytes"));
    unsafe {
        let (cb, _): (FnSize, FnSize) = pair(&format!("{prefix}_bytes"));
        let (csb, _): (FnSize, FnSize) = pair(&format!("{prefix}_scalarbytes"));
        let ob = cb();
        let sb = csb();

        let scalars = sm_scalars(sb, 0x5000 + prefix.len() as u64);
        let points = sm_points(prefix, ob, &scalars, 0x5100 + prefix.len() as u64);

        let mut names: Vec<String> = vec![prefix.to_string()];
        for e in extra {
            if !e.ends_with("base") && !e.ends_with("base_noclamp") {
                names.push(format!("{prefix}_{e}"));
            }
        }
        for name in &names {
            let (c, r): (FnSm, FnSm) = pair(name);
            for n in &scalars {
                for p in &points {
                    let mut co = vec![0xAAu8; ob + 8];
                    let mut ro = vec![0xAAu8; ob + 8];
                    let a = c(co.as_mut_ptr(), n.as_ptr(), p.as_ptr());
                    let b = r(ro.as_mut_ptr(), n.as_ptr(), p.as_ptr());
                    let tag = format!("{name}(n={},p={})", hex(n), hex(p));
                    assert_eq!(a, b, "{tag} return");
                    assert_bytes_eq(&tag, &co, &ro);
                }
            }
        }

        let mut base_names: Vec<String> = Vec::new();
        if has(&format!("{prefix}_base")) {
            base_names.push(format!("{prefix}_base"));
        }
        for e in extra {
            if e.ends_with("base_noclamp") {
                base_names.push(format!("{prefix}_{e}"));
            }
        }
        for name in &base_names {
            let (c, r): (FnSmBase, FnSmBase) = pair(name);
            for n in &scalars {
                let mut co = vec![0xAAu8; ob + 8];
                let mut ro = vec![0xAAu8; ob + 8];
                let a = c(co.as_mut_ptr(), n.as_ptr());
                let b = r(ro.as_mut_ptr(), n.as_ptr());
                let tag = format!("{name}(n={})", hex(n));
                assert_eq!(a, b, "{tag} return");
                assert_bytes_eq(&tag, &co, &ro);
            }
        }
    }
}

#[test]
fn crypto_scalarmult_curve25519_matches() {
    scalarmult_suite("crypto_scalarmult_curve25519", &[]);
}

#[test]
fn crypto_scalarmult_generic_matches() {
    cmp_cstr("crypto_scalarmult_primitive");
    scalarmult_suite("crypto_scalarmult", &[]);
}

#[test]
fn crypto_scalarmult_ed25519_matches() {
    scalarmult_suite(
        "crypto_scalarmult_ed25519",
        &["noclamp", "base_noclamp"],
    );
}

#[test]
fn crypto_scalarmult_ristretto255_matches() {
    scalarmult_suite("crypto_scalarmult_ristretto255", &[]);
}

// ---------------------------------------------------------------------------
// secretbox
// ---------------------------------------------------------------------------

type FnEasy = unsafe extern "C" fn(
    *mut c_uchar,
    *const c_uchar,
    c_ulonglong,
    *const c_uchar,
    *const c_uchar,
) -> c_int;
type FnDetached = unsafe extern "C" fn(
    *mut c_uchar,
    *mut c_uchar,
    *const c_uchar,
    c_ulonglong,
    *const c_uchar,
    *const c_uchar,
) -> c_int;
type FnOpenDetached = unsafe extern "C" fn(
    *mut c_uchar,
    *const c_uchar,
    *const c_uchar,
    c_ulonglong,
    *const c_uchar,
    *const c_uchar,
) -> c_int;

fn secretbox_suite(prefix: &str, easy_suffix: &str) {
    for s in ["keybytes", "noncebytes", "macbytes", "messagebytes_max"] {
        cmp_size(&format!("{prefix}_{s}"));
    }
    unsafe {
        let (ckb, _): (FnSize, FnSize) = pair(&format!("{prefix}_keybytes"));
        let (cnb, _): (FnSize, FnSize) = pair(&format!("{prefix}_noncebytes"));
        let (cmb, _): (FnSize, FnSize) = pair(&format!("{prefix}_macbytes"));
        let kb = ckb();
        let nb = cnb();
        let mb = cmb();

        let ename = if easy_suffix.is_empty() {
            prefix.to_string()
        } else {
            format!("{prefix}_{easy_suffix}")
        };
        let oname = if easy_suffix.is_empty() {
            format!("{prefix}_open")
        } else {
            format!("{prefix}_open_{easy_suffix}")
        };

        let (ce, re): (FnEasy, FnEasy) = pair(&ename);
        let (co_, ro_): (FnEasy, FnEasy) = pair(&oname);

        let mut rng = Rng::new(0x5200 + prefix.len() as u64);
        let msg = rng.vec(3001);
        let mut keys: Vec<Vec<u8>> = vec![vec![0u8; kb], vec![0xffu8; kb]];
        keys.push(rng.vec(kb));
        let mut nonces: Vec<Vec<u8>> = vec![vec![0u8; nb], vec![0xffu8; nb]];
        nonces.push(rng.vec(nb));

        let mlens: Vec<usize> = vec![
            0, 1, 2, 15, 16, 17, 31, 32, 33, 63, 64, 65, 100, 127, 128, 129, 255, 256, 257, 1000,
            2048, 3000,
        ];

        for key in &keys {
            for nonce in &nonces {
                for &mlen in &mlens {
                    let mut cc = vec![0xAAu8; mlen + mb + 8];
                    let mut rc = vec![0xAAu8; mlen + mb + 8];
                    let a = ce(
                        cc.as_mut_ptr(),
                        msg.as_ptr(),
                        mlen as c_ulonglong,
                        nonce.as_ptr(),
                        key.as_ptr(),
                    );
                    let b = re(
                        rc.as_mut_ptr(),
                        msg.as_ptr(),
                        mlen as c_ulonglong,
                        nonce.as_ptr(),
                        key.as_ptr(),
                    );
                    let tag = format!("{ename}(mlen={mlen})");
                    assert_eq!(a, b, "{tag} return");
                    assert_bytes_eq(&tag, &cc, &rc);

                    let clen = mlen + mb;
                    let mut cm = vec![0xAAu8; clen + 8];
                    let mut rm = vec![0xAAu8; clen + 8];
                    let a = co_(
                        cm.as_mut_ptr(),
                        cc.as_ptr(),
                        clen as c_ulonglong,
                        nonce.as_ptr(),
                        key.as_ptr(),
                    );
                    let b = ro_(
                        rm.as_mut_ptr(),
                        cc.as_ptr(),
                        clen as c_ulonglong,
                        nonce.as_ptr(),
                        key.as_ptr(),
                    );
                    let otag = format!("{oname}(mlen={mlen})");
                    assert_eq!(a, b, "{otag} return");
                    assert_bytes_eq(&otag, &cm, &rm);

                    // tampered / truncated ciphertexts
                    let mut bads: Vec<Vec<u8>> = Vec::new();
                    let mut v = cc[..clen].to_vec();
                    v[0] ^= 1;
                    bads.push(v);
                    let mut v = cc[..clen].to_vec();
                    v[clen - 1] ^= 0x80;
                    bads.push(v);
                    bads.push(cc[..clen.saturating_sub(1)].to_vec());
                    bads.push(cc[..mb.saturating_sub(1)].to_vec());
                    bads.push(Vec::new());
                    for bad in bads {
                        let mut cm = vec![0xAAu8; clen + 8];
                        let mut rm = vec![0xAAu8; clen + 8];
                        let a = co_(
                            cm.as_mut_ptr(),
                            bad.as_ptr(),
                            bad.len() as c_ulonglong,
                            nonce.as_ptr(),
                            key.as_ptr(),
                        );
                        let b = ro_(
                            rm.as_mut_ptr(),
                            bad.as_ptr(),
                            bad.len() as c_ulonglong,
                            nonce.as_ptr(),
                            key.as_ptr(),
                        );
                        assert_eq!(a, b, "{otag} tampered(len={}) return", bad.len());
                        assert_bytes_eq(&format!("{otag} tampered(len={})", bad.len()), &cm, &rm);
                    }
                }
            }
        }

        // detached API
        if has(&format!("{prefix}_detached")) {
            let (cd, rd): (FnDetached, FnDetached) = pair(&format!("{prefix}_detached"));
            let (cod, rod): (FnOpenDetached, FnOpenDetached) =
                pair(&format!("{prefix}_open_detached"));
            for key in &keys {
                for nonce in &nonces {
                    for &mlen in &mlens {
                        let mut cc = vec![0xAAu8; mlen + 8];
                        let mut rc = vec![0xAAu8; mlen + 8];
                        let mut cmac = vec![0xAAu8; mb + 8];
                        let mut rmac = vec![0xAAu8; mb + 8];
                        let a = cd(
                            cc.as_mut_ptr(),
                            cmac.as_mut_ptr(),
                            msg.as_ptr(),
                            mlen as c_ulonglong,
                            nonce.as_ptr(),
                            key.as_ptr(),
                        );
                        let b = rd(
                            rc.as_mut_ptr(),
                            rmac.as_mut_ptr(),
                            msg.as_ptr(),
                            mlen as c_ulonglong,
                            nonce.as_ptr(),
                            key.as_ptr(),
                        );
                        let tag = format!("{prefix}_detached(mlen={mlen})");
                        assert_eq!(a, b, "{tag} return");
                        assert_bytes_eq(&format!("{tag} c"), &cc, &rc);
                        assert_bytes_eq(&format!("{tag} mac"), &cmac, &rmac);

                        let mut cm = vec![0xAAu8; mlen + 8];
                        let mut rm = vec![0xAAu8; mlen + 8];
                        let a = cod(
                            cm.as_mut_ptr(),
                            cc.as_ptr(),
                            cmac.as_ptr(),
                            mlen as c_ulonglong,
                            nonce.as_ptr(),
                            key.as_ptr(),
                        );
                        let b = rod(
                            rm.as_mut_ptr(),
                            cc.as_ptr(),
                            cmac.as_ptr(),
                            mlen as c_ulonglong,
                            nonce.as_ptr(),
                            key.as_ptr(),
                        );
                        let otag = format!("{prefix}_open_detached(mlen={mlen})");
                        assert_eq!(a, b, "{otag} return");
                        assert_bytes_eq(&otag, &cm, &rm);

                        let mut badmac = cmac[..mb].to_vec();
                        badmac[0] ^= 1;
                        let mut cm = vec![0xAAu8; mlen + 8];
                        let mut rm = vec![0xAAu8; mlen + 8];
                        let a = cod(
                            cm.as_mut_ptr(),
                            cc.as_ptr(),
                            badmac.as_ptr(),
                            mlen as c_ulonglong,
                            nonce.as_ptr(),
                            key.as_ptr(),
                        );
                        let b = rod(
                            rm.as_mut_ptr(),
                            cc.as_ptr(),
                            badmac.as_ptr(),
                            mlen as c_ulonglong,
                            nonce.as_ptr(),
                            key.as_ptr(),
                        );
                        assert_eq!(a, b, "{otag} bad mac return");
                        assert_bytes_eq(&format!("{otag} bad mac"), &cm, &rm);
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
fn crypto_secretbox_easy_matches() {
    cmp_cstr("crypto_secretbox_primitive");
    cmp_size("crypto_secretbox_zerobytes");
    cmp_size("crypto_secretbox_boxzerobytes");
    secretbox_suite("crypto_secretbox", "easy");
}

#[test]
fn crypto_secretbox_xchacha20poly1305_matches() {
    secretbox_suite("crypto_secretbox_xchacha20poly1305", "easy");
}

/// The deprecated NaCl-style API: the message must be prefixed with ZEROBYTES
/// zero bytes and the ciphertext with BOXZEROBYTES zero bytes.
fn nacl_secretbox_suite(prefix: &str) {
    cmp_size(&format!("{prefix}_zerobytes"));
    cmp_size(&format!("{prefix}_boxzerobytes"));
    unsafe {
        let (ckb, _): (FnSize, FnSize) = pair(&format!("{prefix}_keybytes"));
        let (cnb, _): (FnSize, FnSize) = pair(&format!("{prefix}_noncebytes"));
        let (czb, _): (FnSize, FnSize) = pair(&format!("{prefix}_zerobytes"));
        let (cbzb, _): (FnSize, FnSize) = pair(&format!("{prefix}_boxzerobytes"));
        let kb = ckb();
        let nb = cnb();
        let zb = czb();
        let bzb = cbzb();

        let (ce, re): (FnEasy, FnEasy) = pair(prefix);
        let (co_, ro_): (FnEasy, FnEasy) = pair(&format!("{prefix}_open"));

        let mut rng = Rng::new(0x5300);
        let key = rng.vec(kb);
        let nonce = rng.vec(nb);

        for &plen in &[0usize, 1, 16, 31, 32, 33, 64, 128, 1000] {
            let mlen = zb + plen;
            let mut m = vec![0u8; mlen];
            rng.fill(&mut m[zb..]);
            let mut cc = vec![0xAAu8; mlen + 8];
            let mut rc = vec![0xAAu8; mlen + 8];
            let a = ce(
                cc.as_mut_ptr(),
                m.as_ptr(),
                mlen as c_ulonglong,
                nonce.as_ptr(),
                key.as_ptr(),
            );
            let b = re(
                rc.as_mut_ptr(),
                m.as_ptr(),
                mlen as c_ulonglong,
                nonce.as_ptr(),
                key.as_ptr(),
            );
            let tag = format!("{prefix}(mlen={mlen})");
            assert_eq!(a, b, "{tag} return");
            assert_bytes_eq(&tag, &cc, &rc);

            let mut cm = vec![0xAAu8; mlen + 8];
            let mut rm = vec![0xAAu8; mlen + 8];
            let a = co_(
                cm.as_mut_ptr(),
                cc.as_ptr(),
                mlen as c_ulonglong,
                nonce.as_ptr(),
                key.as_ptr(),
            );
            let b = ro_(
                rm.as_mut_ptr(),
                cc.as_ptr(),
                mlen as c_ulonglong,
                nonce.as_ptr(),
                key.as_ptr(),
            );
            let otag = format!("{prefix}_open(clen={mlen})");
            assert_eq!(a, b, "{otag} return");
            assert_bytes_eq(&otag, &cm, &rm);

            // too-short and tampered
            let mut bads: Vec<Vec<u8>> = vec![
                cc[..mlen.min(zb.saturating_sub(1))].to_vec(),
                Vec::new(),
            ];
            let mut v = cc[..mlen].to_vec();
            if mlen > bzb {
                v[bzb] ^= 1;
                bads.push(v);
            }
            for bad in bads {
                let mut cm = vec![0xAAu8; mlen + 8];
                let mut rm = vec![0xAAu8; mlen + 8];
                let a = co_(
                    cm.as_mut_ptr(),
                    bad.as_ptr(),
                    bad.len() as c_ulonglong,
                    nonce.as_ptr(),
                    key.as_ptr(),
                );
                let b = ro_(
                    rm.as_mut_ptr(),
                    bad.as_ptr(),
                    bad.len() as c_ulonglong,
                    nonce.as_ptr(),
                    key.as_ptr(),
                );
                assert_eq!(a, b, "{otag} bad(len={}) return", bad.len());
                assert_bytes_eq(&format!("{otag} bad(len={})", bad.len()), &cm, &rm);
            }
        }
    }
}

#[test]
fn crypto_secretbox_nacl_api_matches() {
    nacl_secretbox_suite("crypto_secretbox");
}

#[test]
fn crypto_secretbox_xsalsa20poly1305_matches() {
    for s in ["keybytes", "noncebytes", "macbytes", "messagebytes_max"] {
        cmp_size(&format!("crypto_secretbox_xsalsa20poly1305_{s}"));
    }
    nacl_secretbox_suite("crypto_secretbox_xsalsa20poly1305");
    unsafe {
        let (ckb, _): (FnSize, FnSize) = pair("crypto_secretbox_xsalsa20poly1305_keybytes");
        let kb = ckb();
        let (ck, rk): (FnKeygen, FnKeygen) = pair("crypto_secretbox_xsalsa20poly1305_keygen");
        for _ in 0..4 {
            let mut a = vec![0xAAu8; kb + 8];
            let mut b = vec![0xAAu8; kb + 8];
            det_reset();
            ck(a.as_mut_ptr());
            det_reset();
            rk(b.as_mut_ptr());
            assert_bytes_eq("crypto_secretbox_xsalsa20poly1305_keygen", &a, &b);
        }
    }
}

// ---------------------------------------------------------------------------
// crypto_kx
// ---------------------------------------------------------------------------

type FnSeedKeypair = unsafe extern "C" fn(*mut c_uchar, *mut c_uchar, *const c_uchar) -> c_int;
type FnKeypair = unsafe extern "C" fn(*mut c_uchar, *mut c_uchar) -> c_int;
type FnSessionKeys = unsafe extern "C" fn(
    *mut c_uchar,
    *mut c_uchar,
    *const c_uchar,
    *const c_uchar,
    *const c_uchar,
) -> c_int;

#[test]
fn crypto_kx_matches() {
    cmp_cstr("crypto_kx_primitive");
    for s in [
        "publickeybytes",
        "secretkeybytes",
        "seedbytes",
        "sessionkeybytes",
    ] {
        cmp_size(&format!("crypto_kx_{s}"));
    }
    unsafe {
        let (cpk, _): (FnSize, FnSize) = pair("crypto_kx_publickeybytes");
        let (csk, _): (FnSize, FnSize) = pair("crypto_kx_secretkeybytes");
        let (csd, _): (FnSize, FnSize) = pair("crypto_kx_seedbytes");
        let (cses, _): (FnSize, FnSize) = pair("crypto_kx_sessionkeybytes");
        let pkb = cpk();
        let skb = csk();
        let sdb = csd();
        let sesb = cses();

        let (csk_kp, rsk_kp): (FnSeedKeypair, FnSeedKeypair) = pair("crypto_kx_seed_keypair");
        let (ckp, rkp): (FnKeypair, FnKeypair) = pair("crypto_kx_keypair");
        let (ccl, rcl): (FnSessionKeys, FnSessionKeys) = pair("crypto_kx_client_session_keys");
        let (csv, rsv): (FnSessionKeys, FnSessionKeys) = pair("crypto_kx_server_session_keys");

        let mut rng = Rng::new(0x5400);
        let mut seeds: Vec<Vec<u8>> = vec![vec![0u8; sdb], vec![0xffu8; sdb]];
        for _ in 0..6 {
            seeds.push(rng.vec(sdb));
        }

        // seed_keypair
        let mut kps: Vec<(Vec<u8>, Vec<u8>)> = Vec::new();
        for seed in &seeds {
            let mut cpk_b = vec![0xAAu8; pkb + 8];
            let mut rpk_b = vec![0xAAu8; pkb + 8];
            let mut csk_b = vec![0xAAu8; skb + 8];
            let mut rsk_b = vec![0xAAu8; skb + 8];
            let a = csk_kp(cpk_b.as_mut_ptr(), csk_b.as_mut_ptr(), seed.as_ptr());
            let b = rsk_kp(rpk_b.as_mut_ptr(), rsk_b.as_mut_ptr(), seed.as_ptr());
            assert_eq!(a, b, "crypto_kx_seed_keypair return");
            assert_bytes_eq("crypto_kx_seed_keypair pk", &cpk_b, &rpk_b);
            assert_bytes_eq("crypto_kx_seed_keypair sk", &csk_b, &rsk_b);
            kps.push((cpk_b[..pkb].to_vec(), csk_b[..skb].to_vec()));
        }

        // keypair (uses the shared deterministic RNG)
        for _ in 0..6 {
            let mut cpk_b = vec![0xAAu8; pkb + 8];
            let mut rpk_b = vec![0xAAu8; pkb + 8];
            let mut csk_b = vec![0xAAu8; skb + 8];
            let mut rsk_b = vec![0xAAu8; skb + 8];
            det_reset();
            let a = ckp(cpk_b.as_mut_ptr(), csk_b.as_mut_ptr());
            det_reset();
            let b = rkp(rpk_b.as_mut_ptr(), rsk_b.as_mut_ptr());
            assert_eq!(a, b, "crypto_kx_keypair return");
            assert_bytes_eq("crypto_kx_keypair pk", &cpk_b, &rpk_b);
            assert_bytes_eq("crypto_kx_keypair sk", &csk_b, &rsk_b);
        }

        // session keys, including degenerate peer public keys
        let mut peer_pks: Vec<Vec<u8>> = kps.iter().map(|(p, _)| p.clone()).collect();
        peer_pks.push(vec![0u8; pkb]);
        peer_pks.push(vec![0xffu8; pkb]);
        peer_pks.push({
            let mut v = vec![0u8; pkb];
            v[0] = 1;
            v
        });
        peer_pks.push(rng.vec(pkb));

        for (mypk, mysk) in &kps {
            for peer in &peer_pks {
                for (name, cf, rf) in [
                    ("crypto_kx_client_session_keys", ccl, rcl),
                    ("crypto_kx_server_session_keys", csv, rsv),
                ] {
                    // both rx and tx present
                    let mut crx = vec![0xAAu8; sesb + 8];
                    let mut rrx = vec![0xAAu8; sesb + 8];
                    let mut ctx = vec![0xAAu8; sesb + 8];
                    let mut rtx = vec![0xAAu8; sesb + 8];
                    let a = cf(
                        crx.as_mut_ptr(),
                        ctx.as_mut_ptr(),
                        mypk.as_ptr(),
                        mysk.as_ptr(),
                        peer.as_ptr(),
                    );
                    let b = rf(
                        rrx.as_mut_ptr(),
                        rtx.as_mut_ptr(),
                        mypk.as_ptr(),
                        mysk.as_ptr(),
                        peer.as_ptr(),
                    );
                    assert_eq!(a, b, "{name} return");
                    assert_bytes_eq(&format!("{name} rx"), &crx, &rrx);
                    assert_bytes_eq(&format!("{name} tx"), &ctx, &rtx);

                    // NULL rx and NULL tx variants
                    let mut ctx2 = vec![0xAAu8; sesb + 8];
                    let mut rtx2 = vec![0xAAu8; sesb + 8];
                    let a = cf(
                        std::ptr::null_mut(),
                        ctx2.as_mut_ptr(),
                        mypk.as_ptr(),
                        mysk.as_ptr(),
                        peer.as_ptr(),
                    );
                    let b = rf(
                        std::ptr::null_mut(),
                        rtx2.as_mut_ptr(),
                        mypk.as_ptr(),
                        mysk.as_ptr(),
                        peer.as_ptr(),
                    );
                    assert_eq!(a, b, "{name} NULL rx return");
                    assert_bytes_eq(&format!("{name} NULL rx tx"), &ctx2, &rtx2);

                    let mut crx2 = vec![0xAAu8; sesb + 8];
                    let mut rrx2 = vec![0xAAu8; sesb + 8];
                    let a = cf(
                        crx2.as_mut_ptr(),
                        std::ptr::null_mut(),
                        mypk.as_ptr(),
                        mysk.as_ptr(),
                        peer.as_ptr(),
                    );
                    let b = rf(
                        rrx2.as_mut_ptr(),
                        std::ptr::null_mut(),
                        mypk.as_ptr(),
                        mysk.as_ptr(),
                        peer.as_ptr(),
                    );
                    assert_eq!(a, b, "{name} NULL tx return");
                    assert_bytes_eq(&format!("{name} NULL tx rx"), &crx2, &rrx2);

                    // Passing both rx and tx as NULL calls sodium_misuse()
                    // (which aborts); that path is covered by the dedicated
                    // misuse-parity test in t10.
                }
            }
        }
    }
}
