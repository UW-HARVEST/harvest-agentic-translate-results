//! Phase B (continued) — inputs LARGER than the internal chunking constants.
//!
//! Found by mutation testing: nothing in the per-group suites feeds a message
//! bigger than `STREAM_POLY1305_CHUNK` (131072) in
//! `crypto_aead/chacha20poly1305/aead_chacha20poly1305.c`, so the *multi*-chunk
//! iteration of that loop (pointer/counter advance across chunks) was never
//! executed. The same is true of the 64 KiB-plus paths in the stream ciphers and
//! of hash inputs spanning many compression-function blocks.
//!
//! This file drives every bulk entry point past those boundaries:
//! 131072-1, 131072, 131072+1, 2*131072, 2*131072+65, 400000 bytes.
//!
//! Rows: this reinforces CONFIGS.md rows 289-290, 313-314, 355-357, 424-430,
//! 510-716 (the "large" shape of every AEAD / secretbox / stream / hash row)
//! rather than adding new ones.
mod common;
use common::*;
use std::os::raw::c_int;
use std::ptr;

const CHUNK: usize = 131072;

fn sizes() -> Vec<usize> {
    vec![
        CHUNK - 65,
        CHUNK - 1,
        CHUNK,
        CHUNK + 1,
        CHUNK + 63,
        CHUNK + 64,
        CHUNK + 65,
        2 * CHUNK,
        2 * CHUNK + 65,
        400_000,
    ]
}

// --------------------------------------------------------------- AEADs ------
type AeadEnc = unsafe extern "C" fn(
    *mut u8,
    *mut u64,
    *const u8,
    u64,
    *const u8,
    u64,
    *const u8,
    *const u8,
    *const u8,
) -> c_int;
type AeadDec = unsafe extern "C" fn(
    *mut u8,
    *mut u64,
    *const u8,
    *const u8,
    u64,
    *const u8,
    u64,
    *const u8,
    *const u8,
) -> c_int;

fn aead_large(prefix: &str, npub: usize, key: usize, abytes: usize) {
    unsafe {
        let (ce, re) = pair::<AeadEnc>(&format!("{prefix}_encrypt"));
        let (cd, rd) = pair::<AeadDec>(&format!("{prefix}_decrypt"));
        let mut rng = Rng::new(0xB16_1D_0001 ^ prefix.len() as u64);
        let k = rng.bytes(key);
        let n = rng.bytes(npub);
        for mlen in sizes() {
            let m = rng.bytes(mlen);
            for adlen in [0usize, 17, CHUNK + 3] {
                let ad = rng.bytes(adlen);
                let adp = if adlen == 0 { ptr::null() } else { ad.as_ptr() };
                let mut cc = vec![0u8; mlen + abytes + 8];
                let mut rc = vec![0u8; mlen + abytes + 8];
                let mut cl = 0u64;
                let mut rl = 0u64;
                let a = ce(
                    cc.as_mut_ptr(),
                    &mut cl,
                    m.as_ptr(),
                    mlen as u64,
                    adp,
                    adlen as u64,
                    ptr::null(),
                    n.as_ptr(),
                    k.as_ptr(),
                );
                let b = re(
                    rc.as_mut_ptr(),
                    &mut rl,
                    m.as_ptr(),
                    mlen as u64,
                    adp,
                    adlen as u64,
                    ptr::null(),
                    n.as_ptr(),
                    k.as_ptr(),
                );
                let ctx = format!("{prefix}_encrypt mlen={mlen} adlen={adlen}");
                eq_i32(&ctx, a, b);
                assert_eq!(cl, rl, "{ctx}: clen differs");
                eq_bytes(&ctx, &cc, &rc);

                // decrypt (valid)
                let mut cm = vec![0u8; mlen + 8];
                let mut rm = vec![0u8; mlen + 8];
                let mut cml = 0u64;
                let mut rml = 0u64;
                let a = cd(
                    cm.as_mut_ptr(),
                    &mut cml,
                    ptr::null(),
                    cc.as_ptr(),
                    cl,
                    adp,
                    adlen as u64,
                    n.as_ptr(),
                    k.as_ptr(),
                );
                let b = rd(
                    rm.as_mut_ptr(),
                    &mut rml,
                    ptr::null(),
                    rc.as_ptr(),
                    rl,
                    adp,
                    adlen as u64,
                    n.as_ptr(),
                    k.as_ptr(),
                );
                let ctx = format!("{prefix}_decrypt mlen={mlen} adlen={adlen}");
                eq_i32(&ctx, a, b);
                assert_eq!(cml, rml, "{ctx}: mlen differs");
                eq_bytes(&ctx, &cm, &rm);
                assert_eq!(a, 0, "{ctx}: valid ciphertext must decrypt");
                assert_eq!(&cm[..mlen], &m[..], "{ctx}: round-trip");

                // decrypt (tampered near, at, and past a chunk boundary)
                for pos in [0usize, CHUNK - 1, CHUNK, CHUNK + 1, mlen - 1] {
                    if pos >= cl as usize {
                        continue;
                    }
                    let mut bad_c = cc.clone();
                    let mut bad_r = rc.clone();
                    bad_c[pos] ^= 0x40;
                    bad_r[pos] ^= 0x40;
                    let mut cm = vec![0u8; mlen + 8];
                    let mut rm = vec![0u8; mlen + 8];
                    let mut cml = 0u64;
                    let mut rml = 0u64;
                    let a = cd(
                        cm.as_mut_ptr(),
                        &mut cml,
                        ptr::null(),
                        bad_c.as_ptr(),
                        cl,
                        adp,
                        adlen as u64,
                        n.as_ptr(),
                        k.as_ptr(),
                    );
                    let b = rd(
                        rm.as_mut_ptr(),
                        &mut rml,
                        ptr::null(),
                        bad_r.as_ptr(),
                        rl,
                        adp,
                        adlen as u64,
                        n.as_ptr(),
                        k.as_ptr(),
                    );
                    let ctx = format!("{prefix}_decrypt tampered@{pos} mlen={mlen}");
                    eq_i32(&ctx, a, b);
                    assert_eq!(a, -1, "{ctx}: tampered ciphertext must be rejected");
                    eq_bytes(&ctx, &cm, &rm);
                }
            }
        }
    }
}

#[test]
fn t14_aead_chacha20poly1305_ietf_large() {
    aead_large("crypto_aead_chacha20poly1305_ietf", 12, 32, 16);
}

#[test]
fn t14_aead_chacha20poly1305_original_large() {
    aead_large("crypto_aead_chacha20poly1305", 8, 32, 16);
}

#[test]
fn t14_aead_xchacha20poly1305_ietf_large() {
    aead_large("crypto_aead_xchacha20poly1305_ietf", 24, 32, 16);
}

#[test]
fn t14_aead_aegis128l_large() {
    aead_large("crypto_aead_aegis128l", 16, 16, 32);
}

#[test]
fn t14_aead_aegis256_large() {
    aead_large("crypto_aead_aegis256", 32, 32, 32);
}

// ---------------------------------------------------------- secretbox -------
#[test]
fn t14_secretbox_easy_large() {
    type F = unsafe extern "C" fn(*mut u8, *const u8, u64, *const u8, *const u8) -> c_int;
    unsafe {
        for (e, o, mac) in [
            ("crypto_secretbox_easy", "crypto_secretbox_open_easy", 16usize),
            (
                "crypto_secretbox_xchacha20poly1305_easy",
                "crypto_secretbox_xchacha20poly1305_open_easy",
                16,
            ),
        ] {
            let (ce, re) = pair::<F>(e);
            let (co, ro) = pair::<F>(o);
            let mut rng = Rng::new(0x5EC_B0_1234);
            let k = rng.bytes(32);
            let n = rng.bytes(24);
            for mlen in sizes() {
                let m = rng.bytes(mlen);
                let mut cc = vec![0u8; mlen + mac + 8];
                let mut rc = vec![0u8; mlen + mac + 8];
                let a = ce(cc.as_mut_ptr(), m.as_ptr(), mlen as u64, n.as_ptr(), k.as_ptr());
                let b = re(rc.as_mut_ptr(), m.as_ptr(), mlen as u64, n.as_ptr(), k.as_ptr());
                let ctx = format!("{e} mlen={mlen}");
                eq_i32(&ctx, a, b);
                eq_bytes(&ctx, &cc, &rc);
                let mut cm = vec![0u8; mlen + 8];
                let mut rm = vec![0u8; mlen + 8];
                let a = co(
                    cm.as_mut_ptr(),
                    cc.as_ptr(),
                    (mlen + mac) as u64,
                    n.as_ptr(),
                    k.as_ptr(),
                );
                let b = ro(
                    rm.as_mut_ptr(),
                    rc.as_ptr(),
                    (mlen + mac) as u64,
                    n.as_ptr(),
                    k.as_ptr(),
                );
                let ctx = format!("{o} mlen={mlen}");
                eq_i32(&ctx, a, b);
                assert_eq!(a, 0);
                eq_bytes(&ctx, &cm, &rm);
                assert_eq!(&cm[..mlen], &m[..]);
            }
        }
    }
}

// ------------------------------------------------------------- streams ------
#[test]
fn t14_stream_xor_large() {
    type Xor = unsafe extern "C" fn(*mut u8, *const u8, u64, *const u8, *const u8) -> c_int;
    unsafe {
        for (name, nlen) in [
            ("crypto_stream_chacha20_xor", 8usize),
            ("crypto_stream_chacha20_ietf_xor", 12),
            ("crypto_stream_salsa20_xor", 8),
            ("crypto_stream_salsa2012_xor", 8),
            ("crypto_stream_salsa208_xor", 8),
            ("crypto_stream_xsalsa20_xor", 24),
            ("crypto_stream_xchacha20_xor", 24),
        ] {
            let (c, r) = pair::<Xor>(name);
            let mut rng = Rng::new(0x57_1234 ^ name.len() as u64);
            let k = rng.bytes(32);
            let n = rng.bytes(nlen);
            for mlen in sizes() {
                let m = rng.bytes(mlen);
                let mut cc = vec![0u8; mlen + 8];
                let mut rc = vec![0u8; mlen + 8];
                let a = c(cc.as_mut_ptr(), m.as_ptr(), mlen as u64, n.as_ptr(), k.as_ptr());
                let b = r(rc.as_mut_ptr(), m.as_ptr(), mlen as u64, n.as_ptr(), k.as_ptr());
                let ctx = format!("{name} mlen={mlen}");
                eq_i32(&ctx, a, b);
                eq_bytes(&ctx, &cc, &rc);
            }
        }
    }
}

#[test]
fn t14_stream_xor_ic_large_counters() {
    // ic values that make the 32-bit block counter WRAP part-way through a
    // large message (the ietf variants must reject, the ext/64-bit ones carry).
    type XorIc32 =
        unsafe extern "C" fn(*mut u8, *const u8, u64, *const u8, u32, *const u8) -> c_int;
    type XorIc64 =
        unsafe extern "C" fn(*mut u8, *const u8, u64, *const u8, u64, *const u8) -> c_int;
    unsafe {
        let mut rng = Rng::new(0x1C_9999);
        let k = rng.bytes(32);
        // 64-bit ic variants
        for (name, nlen) in [
            ("crypto_stream_chacha20_xor_ic", 8usize),
            ("crypto_stream_salsa20_xor_ic", 8),
            ("crypto_stream_xsalsa20_xor_ic", 24),
            ("crypto_stream_xchacha20_xor_ic", 24),
        ] {
            let (c, r) = pair::<XorIc64>(name);
            let n = rng.bytes(nlen);
            for mlen in [CHUNK + 65usize, 2 * CHUNK] {
                let m = rng.bytes(mlen);
                for ic in [
                    0u64,
                    1,
                    0xffff_fffeu64,
                    0xffff_ffffu64,
                    0x1_0000_0000u64,
                    u64::MAX - 8192,
                ] {
                    let mut cc = vec![0u8; mlen + 8];
                    let mut rc = vec![0u8; mlen + 8];
                    let a = c(
                        cc.as_mut_ptr(),
                        m.as_ptr(),
                        mlen as u64,
                        n.as_ptr(),
                        ic,
                        k.as_ptr(),
                    );
                    let b = r(
                        rc.as_mut_ptr(),
                        m.as_ptr(),
                        mlen as u64,
                        n.as_ptr(),
                        ic,
                        k.as_ptr(),
                    );
                    let ctx = format!("{name} mlen={mlen} ic={ic:#x}");
                    eq_i32(&ctx, a, b);
                    eq_bytes(&ctx, &cc, &rc);
                }
            }
        }
        // 32-bit ic variant: only counters that cannot overflow (the abort case
        // is covered by t06_g4_aead_errors).
        let (c, r) = pair::<XorIc32>("crypto_stream_chacha20_ietf_xor_ic");
        let n = rng.bytes(12);
        for mlen in [CHUNK + 65usize, 2 * CHUNK] {
            let m = rng.bytes(mlen);
            let blocks = ((mlen + 63) / 64) as u32;
            for ic in [0u32, 1, 1000, 0xffff_ffffu32 - blocks] {
                let mut cc = vec![0u8; mlen + 8];
                let mut rc = vec![0u8; mlen + 8];
                let a = c(
                    cc.as_mut_ptr(),
                    m.as_ptr(),
                    mlen as u64,
                    n.as_ptr(),
                    ic,
                    k.as_ptr(),
                );
                let b = r(
                    rc.as_mut_ptr(),
                    m.as_ptr(),
                    mlen as u64,
                    n.as_ptr(),
                    ic,
                    k.as_ptr(),
                );
                let ctx = format!("chacha20_ietf_xor_ic mlen={mlen} ic={ic:#x}");
                eq_i32(&ctx, a, b);
                eq_bytes(&ctx, &cc, &rc);
            }
        }
    }
}

// -------------------------------------------------------------- hashes ------
#[test]
fn t14_hashes_large() {
    unsafe {
        let mut rng = Rng::new(0x_A51_DEAD);
        for mlen in sizes() {
            let m = rng.bytes(mlen);
            // one-shot hashes with a (u8*, u64) signature
            for (name, outlen) in [
                ("crypto_hash_sha256", 32usize),
                ("crypto_hash_sha512", 64),
                ("crypto_hash", 64),
                ("crypto_hash_sha3_256", 32),
                ("crypto_hash_sha3_512", 64),
            ] {
                if !has_sym(name) {
                    continue;
                }
                let (c, r) =
                    pair::<unsafe extern "C" fn(*mut u8, *const u8, u64) -> c_int>(name);
                let mut co = vec![0u8; outlen + 8];
                let mut ro = vec![0u8; outlen + 8];
                let a = c(co.as_mut_ptr(), m.as_ptr(), mlen as u64);
                let b = r(ro.as_mut_ptr(), m.as_ptr(), mlen as u64);
                let ctx = format!("{name} mlen={mlen}");
                eq_i32(&ctx, a, b);
                eq_bytes(&ctx, &co, &ro);
            }
            // generichash / blake2b
            let (c, r) = pair::<
                unsafe extern "C" fn(*mut u8, usize, *const u8, u64, *const u8, usize) -> c_int,
            >("crypto_generichash");
            for outlen in [16usize, 32, 64] {
                let mut co = vec![0u8; outlen + 8];
                let mut ro = vec![0u8; outlen + 8];
                let a = c(
                    co.as_mut_ptr(),
                    outlen,
                    m.as_ptr(),
                    mlen as u64,
                    ptr::null(),
                    0,
                );
                let b = r(
                    ro.as_mut_ptr(),
                    outlen,
                    m.as_ptr(),
                    mlen as u64,
                    ptr::null(),
                    0,
                );
                let ctx = format!("crypto_generichash mlen={mlen} outlen={outlen}");
                eq_i32(&ctx, a, b);
                eq_bytes(&ctx, &co, &ro);
            }
            // hmac / onetimeauth / shorthash
            let key32 = rng.bytes(32);
            for (name, outlen, klen) in [
                ("crypto_auth", 32usize, 32usize),
                ("crypto_auth_hmacsha256", 32, 32),
                ("crypto_auth_hmacsha512", 64, 32),
                ("crypto_onetimeauth", 16, 32),
                ("crypto_shorthash", 8, 16),
            ] {
                let (c, r) = pair::<
                    unsafe extern "C" fn(*mut u8, *const u8, u64, *const u8) -> c_int,
                >(name);
                let mut co = vec![0u8; outlen + 8];
                let mut ro = vec![0u8; outlen + 8];
                let a = c(co.as_mut_ptr(), m.as_ptr(), mlen as u64, key32[..klen].as_ptr());
                let b = r(ro.as_mut_ptr(), m.as_ptr(), mlen as u64, key32[..klen].as_ptr());
                let ctx = format!("{name} mlen={mlen}");
                eq_i32(&ctx, a, b);
                eq_bytes(&ctx, &co, &ro);
            }
        }
    }
}

// ---------------------------------------------------------------- sign ------
#[test]
fn t14_sign_large() {
    install_det_random();
    unsafe {
        let (ckp, rkp) =
            pair::<unsafe extern "C" fn(*mut u8, *mut u8, *const u8) -> c_int>(
                "crypto_sign_ed25519_seed_keypair",
            );
        let (cs, rs) = pair::<
            unsafe extern "C" fn(*mut u8, *mut u64, *const u8, u64, *const u8) -> c_int,
        >("crypto_sign_ed25519_detached");
        let (cv, rv) = pair::<
            unsafe extern "C" fn(*const u8, *const u8, u64, *const u8) -> c_int,
        >("crypto_sign_ed25519_verify_detached");
        let mut rng = Rng::new(0x516E_0001);
        let seed = rng.bytes(32);
        let mut cpk = [0u8; 32];
        let mut csk = [0u8; 64];
        let mut rpk = [0u8; 32];
        let mut rsk = [0u8; 64];
        assert_eq!(ckp(cpk.as_mut_ptr(), csk.as_mut_ptr(), seed.as_ptr()), 0);
        assert_eq!(rkp(rpk.as_mut_ptr(), rsk.as_mut_ptr(), seed.as_ptr()), 0);
        eq_bytes("seed_keypair pk", &cpk, &rpk);
        eq_bytes("seed_keypair sk", &csk, &rsk);
        for mlen in sizes() {
            let m = rng.bytes(mlen);
            let mut csig = [0u8; 64];
            let mut rsig = [0u8; 64];
            let mut cl = 0u64;
            let mut rl = 0u64;
            let a = cs(
                csig.as_mut_ptr(),
                &mut cl,
                m.as_ptr(),
                mlen as u64,
                csk.as_ptr(),
            );
            let b = rs(
                rsig.as_mut_ptr(),
                &mut rl,
                m.as_ptr(),
                mlen as u64,
                rsk.as_ptr(),
            );
            let ctx = format!("crypto_sign_ed25519_detached mlen={mlen}");
            eq_i32(&ctx, a, b);
            assert_eq!(cl, rl);
            eq_bytes(&ctx, &csig, &rsig);
            let a = cv(csig.as_ptr(), m.as_ptr(), mlen as u64, cpk.as_ptr());
            let b = rv(rsig.as_ptr(), m.as_ptr(), mlen as u64, rpk.as_ptr());
            eq_i32(&format!("verify_detached mlen={mlen}"), a, b);
            assert_eq!(a, 0);
        }
    }
}

// ------------------------------------------------------- secretstream -------
#[test]
fn t14_secretstream_large_messages() {
    // `init_push` draws its 24-byte header from randombytes, so the shared
    // deterministic stream must be REWOUND between the C and the Rust call
    // (otherwise Rust consumes the bytes that follow C's).
    install_det_random();
    unsafe {
        let statebytes =
            pair::<unsafe extern "C" fn() -> usize>("crypto_secretstream_xchacha20poly1305_statebytes");
        let sb = statebytes.0();
        assert_eq!(sb, statebytes.1());
        let (cip, rip) = pair::<
            unsafe extern "C" fn(*mut u8, *mut u8, *const u8) -> c_int,
        >("crypto_secretstream_xchacha20poly1305_init_push");
        let (cpu, rpu) = pair::<
            unsafe extern "C" fn(
                *mut u8,
                *mut u8,
                *mut u64,
                *const u8,
                u64,
                *const u8,
                u64,
                u8,
            ) -> c_int,
        >("crypto_secretstream_xchacha20poly1305_push");
        let (cipl, ripl) = pair::<
            unsafe extern "C" fn(*mut u8, *const u8, *const u8) -> c_int,
        >("crypto_secretstream_xchacha20poly1305_init_pull");
        let (cpl, rpl) = pair::<
            unsafe extern "C" fn(
                *mut u8,
                *mut u8,
                *mut u64,
                *mut u8,
                *const u8,
                u64,
                *const u8,
                u64,
            ) -> c_int,
        >("crypto_secretstream_xchacha20poly1305_pull");
        let mut rng = Rng::new(0x55_7777);
        let k = rng.bytes(32);
        let mut cst = vec![0u64; (sb + 7) / 8 + 2];
        let mut rst = vec![0u64; (sb + 7) / 8 + 2];
        let mut ch = [0u8; 24];
        let mut rh = [0u8; 24];
        det_reseed(0x5EC_57_1234);
        assert_eq!(
            cip(cst.as_mut_ptr() as *mut u8, ch.as_mut_ptr(), k.as_ptr()),
            0
        );
        det_reseed(0x5EC_57_1234);
        assert_eq!(
            rip(rst.as_mut_ptr() as *mut u8, rh.as_mut_ptr(), k.as_ptr()),
            0
        );
        eq_bytes("secretstream header", &ch, &rh);
        let mut cct: Vec<Vec<u8>> = Vec::new();
        for mlen in sizes() {
            let m = rng.bytes(mlen);
            let mut cc = vec![0u8; mlen + 17 + 8];
            let mut rc = vec![0u8; mlen + 17 + 8];
            let mut cl = 0u64;
            let mut rl = 0u64;
            let a = cpu(
                cst.as_mut_ptr() as *mut u8,
                cc.as_mut_ptr(),
                &mut cl,
                m.as_ptr(),
                mlen as u64,
                ptr::null(),
                0,
                0,
            );
            let b = rpu(
                rst.as_mut_ptr() as *mut u8,
                rc.as_mut_ptr(),
                &mut rl,
                m.as_ptr(),
                mlen as u64,
                ptr::null(),
                0,
                0,
            );
            let ctx = format!("secretstream_push mlen={mlen}");
            eq_i32(&ctx, a, b);
            assert_eq!(cl, rl, "{ctx}: clen differs");
            eq_bytes(&ctx, &cc, &rc);
            cct.push(cc[..cl as usize].to_vec());
        }
        // now pull them all back
        let mut cst = vec![0u64; (sb + 7) / 8 + 2];
        let mut rst = vec![0u64; (sb + 7) / 8 + 2];
        assert_eq!(
            cipl(cst.as_mut_ptr() as *mut u8, ch.as_ptr(), k.as_ptr()),
            0
        );
        assert_eq!(
            ripl(rst.as_mut_ptr() as *mut u8, rh.as_ptr(), k.as_ptr()),
            0
        );
        for (i, ct) in cct.iter().enumerate() {
            let mlen = ct.len() - 17;
            let mut cm = vec![0u8; mlen + 8];
            let mut rm = vec![0u8; mlen + 8];
            let mut cl = 0u64;
            let mut rl = 0u64;
            let mut ctag = 0xEEu8;
            let mut rtag = 0xEEu8;
            let a = cpl(
                cst.as_mut_ptr() as *mut u8,
                cm.as_mut_ptr(),
                &mut cl,
                &mut ctag,
                ct.as_ptr(),
                ct.len() as u64,
                ptr::null(),
                0,
            );
            let b = rpl(
                rst.as_mut_ptr() as *mut u8,
                rm.as_mut_ptr(),
                &mut rl,
                &mut rtag,
                ct.as_ptr(),
                ct.len() as u64,
                ptr::null(),
                0,
            );
            let ctx = format!("secretstream_pull msg {i} mlen={mlen}");
            eq_i32(&ctx, a, b);
            assert_eq!(a, 0, "{ctx}: must succeed");
            assert_eq!(cl, rl, "{ctx}: mlen differs");
            assert_eq!(ctag, rtag, "{ctx}: tag differs");
            eq_bytes(&ctx, &cm, &rm);
        }
    }
}
