//! Differential tests for AREA `box`:
//!
//! * `crypto_box/crypto_box.c`
//! * `crypto_box/crypto_box_easy.c`
//! * `crypto_box/crypto_box_seal.c`
//! * `crypto_box/curve25519xsalsa20poly1305/box_curve25519xsalsa20poly1305.c`
//! * `crypto_box/curve25519xchacha20poly1305/box_curve25519xchacha20poly1305.c`
//! * `crypto_box/curve25519xchacha20poly1305/box_seal_curve25519xchacha20poly1305.c`
//! * `crypto_kx/crypto_kx.c`
//! * `crypto_kem/crypto_kem.c`
//! * `crypto_kem/mlkem768/kem_mlkem768.c`
//! * `crypto_kem/mlkem768/ref/kem_mlkem768_ref.c`
//! * `crypto_kem/xwing/kem_xwing.c`
//!
//! Everything goes through `dlopen`/`dlsym` on the two shared objects.
//!
//! Functions that internally call `randombytes` (`*_keypair`, `*_enc`, `seal`)
//! cannot be compared byte-for-byte because each `.so` has its own RNG state;
//! for those we compare the return code and verify *cross-library* round trips
//! (C encrypt -> Rust decrypt and Rust encrypt -> C decrypt), which is a
//! stronger property than "same bytes".

#![allow(dead_code)]
#![allow(clippy::too_many_arguments)]

#[macro_use]
mod common;

use core::ffi::{c_char, c_int};

// ------------------------------------------------------------------ syms -----

/// Look up the same symbol in both libraries using a *runtime* name (the
/// `both!` macro needs a literal, and we build names from prefixes).
fn syms<T: Copy>(name: &str) -> (T, T) {
    let l = common::libs();
    let mut b = Vec::with_capacity(name.len() + 1);
    b.extend_from_slice(name.as_bytes());
    b.push(0);
    unsafe {
        let cs: libloading::Symbol<T> = l
            .c
            .get(&b)
            .unwrap_or_else(|e| panic!("C lib missing symbol {}: {}", name, e));
        let rs: libloading::Symbol<T> = l
            .r
            .get(&b)
            .unwrap_or_else(|e| panic!("Rust lib missing symbol {}: {}", name, e));
        (*cs, *rs)
    }
}

type SizeFn = unsafe extern "C" fn() -> usize;
type StrFn = unsafe extern "C" fn() -> *const c_char;
type SeedKp = unsafe extern "C" fn(*mut u8, *mut u8, *const u8) -> c_int;
type Kp = unsafe extern "C" fn(*mut u8, *mut u8) -> c_int;
type Beforenm = unsafe extern "C" fn(*mut u8, *const u8, *const u8) -> c_int;
/// `afternm(c, m, mlen, n, k)` / `open_afternm(m, c, clen, n, k)`
type Afternm = unsafe extern "C" fn(*mut u8, *const u8, u64, *const u8, *const u8) -> c_int;
/// `box(c, m, mlen, n, pk, sk)` / `open(m, c, clen, n, pk, sk)`
type BoxFull =
    unsafe extern "C" fn(*mut u8, *const u8, u64, *const u8, *const u8, *const u8) -> c_int;
/// `detached(c, mac, m, mlen, n, pk, sk)`
type Detached =
    unsafe extern "C" fn(*mut u8, *mut u8, *const u8, u64, *const u8, *const u8, *const u8) -> c_int;
/// `detached_afternm(c, mac, m, mlen, n, k)`
type DetachedAfternm =
    unsafe extern "C" fn(*mut u8, *mut u8, *const u8, u64, *const u8, *const u8) -> c_int;
/// `open_detached(m, c, mac, clen, n, pk, sk)`
type OpenDetached = unsafe extern "C" fn(
    *mut u8,
    *const u8,
    *const u8,
    u64,
    *const u8,
    *const u8,
    *const u8,
) -> c_int;
/// `open_detached_afternm(m, c, mac, clen, n, k)`
type OpenDetachedAfternm =
    unsafe extern "C" fn(*mut u8, *const u8, *const u8, u64, *const u8, *const u8) -> c_int;
/// `seal(c, m, mlen, pk)`
type Seal = unsafe extern "C" fn(*mut u8, *const u8, u64, *const u8) -> c_int;
/// `seal_open(m, c, clen, pk, sk)`
type SealOpen = unsafe extern "C" fn(*mut u8, *const u8, u64, *const u8, *const u8) -> c_int;
/// `kx_session_keys(rx, tx, pk, sk, peer_pk)`
type KxSession =
    unsafe extern "C" fn(*mut u8, *mut u8, *const u8, *const u8, *const u8) -> c_int;
/// `kem_enc(ct, ss, pk)`
type KemEnc = unsafe extern "C" fn(*mut u8, *mut u8, *const u8) -> c_int;
/// `kem_enc_deterministic(ct, ss, pk, seed)`
type KemEncDet = unsafe extern "C" fn(*mut u8, *mut u8, *const u8, *const u8) -> c_int;
/// `kem_dec(ss, ct, sk)`
type KemDec = unsafe extern "C" fn(*mut u8, *const u8, *const u8) -> c_int;

// ------------------------------------------------------------- constants -----

const PKB: usize = 32; // crypto_box_PUBLICKEYBYTES
const SKB: usize = 32; // crypto_box_SECRETKEYBYTES
const NB: usize = 24; // crypto_box_NONCEBYTES
const MACB: usize = 16; // crypto_box_MACBYTES
const KB: usize = 32; // crypto_box_BEFORENMBYTES
const ZB: usize = 32; // crypto_box_ZEROBYTES
const BZB: usize = 16; // crypto_box_BOXZEROBYTES
const SEALB: usize = 48; // crypto_box_SEALBYTES

const CANARY: u8 = 0x5A;

// ------------------------------------------------------------- utilities -----

fn chk_size(name: &str, expect: usize) {
    let (c, r) = syms::<SizeFn>(name);
    let (a, b) = unsafe { (c(), r()) };
    assert_eq!(a, expect, "{}: C value not the documented constant", name);
    assert_eq!(a, b, "{}: getter mismatch (C={} Rust={})", name, a, b);
}

fn chk_size_eq_only(name: &str) -> usize {
    let (c, r) = syms::<SizeFn>(name);
    let (a, b) = unsafe { (c(), r()) };
    assert_eq!(a, b, "{}: getter mismatch (C={} Rust={})", name, a, b);
    a
}

fn cstr(p: *const c_char) -> String {
    unsafe {
        let mut v = Vec::new();
        let mut i = 0isize;
        loop {
            let b = *p.offset(i) as u8;
            if b == 0 {
                break;
            }
            v.push(b);
            i += 1;
        }
        String::from_utf8(v).unwrap()
    }
}

fn chk_str(name: &str, expect: &str) {
    let (c, r) = syms::<StrFn>(name);
    let (a, b) = unsafe { (cstr(c()), cstr(r())) };
    assert_eq!(a, expect, "{}: C string not the documented constant", name);
    assert_eq!(a, b, "{}: string mismatch", name);
}

/// Public keys that make `crypto_scalarmult_curve25519` fail (all-zero output).
fn bad_public_keys() -> Vec<[u8; 32]> {
    let mut v = Vec::new();
    v.push([0u8; 32]); // 0
    let mut one = [0u8; 32];
    one[0] = 1;
    v.push(one); // 1
    let mut p = [0xffu8; 32];
    p[0] = 0xed;
    p[31] = 0x7f;
    v.push(p); // p == 0 mod p
    let mut p1 = [0xffu8; 32];
    p1[0] = 0xee;
    p1[31] = 0x7f;
    v.push(p1); // p+1 == 1 mod p
    // order-8 points (also produce an all-zero shared secret)
    v.push([
        0xe0, 0xeb, 0x7a, 0x7c, 0x3b, 0x41, 0xb8, 0xae, 0x16, 0x56, 0xe3, 0xfa, 0xf1, 0x9f, 0xc4,
        0x6a, 0xda, 0x09, 0x8d, 0xeb, 0x9c, 0x32, 0xb1, 0xfd, 0x86, 0x62, 0x05, 0x16, 0x5f, 0x49,
        0xb8, 0x00,
    ]);
    v.push([
        0x5f, 0x9c, 0x95, 0xbc, 0xa3, 0x50, 0x8c, 0x24, 0xb1, 0xd0, 0xb1, 0x55, 0x9c, 0x83, 0xef,
        0x5b, 0x04, 0x44, 0x5c, 0xc4, 0x58, 0x1c, 0x8e, 0x86, 0xd8, 0x22, 0x4e, 0xdd, 0xd0, 0x9f,
        0x11, 0x57,
    ]);
    v
}

/// Derive a deterministic key pair from a seed with both libraries and assert
/// byte equality; returns the pair.
fn seed_kp(api: &(SeedKp, SeedKp), seed: &[u8; 32], tag: &str) -> ([u8; PKB], [u8; SKB]) {
    let mut pkc = [CANARY; PKB];
    let mut skc = [CANARY; SKB];
    let mut pkr = [CANARY; PKB];
    let mut skr = [CANARY; SKB];
    let rc = unsafe { api.0(pkc.as_mut_ptr(), skc.as_mut_ptr(), seed.as_ptr()) };
    let rr = unsafe { api.1(pkr.as_mut_ptr(), skr.as_mut_ptr(), seed.as_ptr()) };
    common::eqi(&format!("{} seed_keypair rc", tag), rc, rr);
    assert_eq!(rc, 0, "{} seed_keypair: expected success", tag);
    common::eqb(&format!("{} seed_keypair pk", tag), &pkc, &pkr);
    common::eqb(&format!("{} seed_keypair sk", tag), &skc, &skr);
    (pkc, skc)
}

fn beforenm_both(api: &(Beforenm, Beforenm), pk: &[u8], sk: &[u8], tag: &str) -> (c_int, [u8; KB]) {
    let mut kc = [CANARY; KB];
    let mut kr = [CANARY; KB];
    let rc = unsafe { api.0(kc.as_mut_ptr(), pk.as_ptr(), sk.as_ptr()) };
    let rr = unsafe { api.1(kr.as_mut_ptr(), pk.as_ptr(), sk.as_ptr()) };
    common::eqi(&format!("{} beforenm rc", tag), rc, rr);
    common::eqb(&format!("{} beforenm k", tag), &kc, &kr);
    (rc, kc)
}

// =============================================================== getters =====

#[test]
fn box_getters() {
    // crypto_box.c
    chk_size("crypto_box_seedbytes", 32);
    chk_size("crypto_box_publickeybytes", 32);
    chk_size("crypto_box_secretkeybytes", 32);
    chk_size("crypto_box_beforenmbytes", 32);
    chk_size("crypto_box_noncebytes", 24);
    chk_size("crypto_box_zerobytes", 32);
    chk_size("crypto_box_boxzerobytes", 16);
    chk_size("crypto_box_macbytes", 16);
    chk_str("crypto_box_primitive", "curve25519xsalsa20poly1305");
    // MESSAGEBYTES_MAX == SODIUM_SIZE_MAX - MACBYTES; do not hardcode, just
    // require both libraries to agree and to be the expected huge value.
    let m1 = chk_size_eq_only("crypto_box_messagebytes_max");
    assert_eq!(m1, usize::MAX - 16, "crypto_box_messagebytes_max");
    // crypto_box_seal.c
    chk_size("crypto_box_sealbytes", 48);

    // curve25519xsalsa20poly1305
    let p = "crypto_box_curve25519xsalsa20poly1305_";
    chk_size(&format!("{}seedbytes", p), 32);
    chk_size(&format!("{}publickeybytes", p), 32);
    chk_size(&format!("{}secretkeybytes", p), 32);
    chk_size(&format!("{}beforenmbytes", p), 32);
    chk_size(&format!("{}noncebytes", p), 24);
    chk_size(&format!("{}zerobytes", p), 32);
    chk_size(&format!("{}boxzerobytes", p), 16);
    chk_size(&format!("{}macbytes", p), 16);
    assert_eq!(
        chk_size_eq_only(&format!("{}messagebytes_max", p)),
        usize::MAX - 16
    );

    // curve25519xchacha20poly1305 (no zerobytes/boxzerobytes: no low-level API)
    let p = "crypto_box_curve25519xchacha20poly1305_";
    chk_size(&format!("{}seedbytes", p), 32);
    chk_size(&format!("{}publickeybytes", p), 32);
    chk_size(&format!("{}secretkeybytes", p), 32);
    chk_size(&format!("{}beforenmbytes", p), 32);
    chk_size(&format!("{}noncebytes", p), 24);
    chk_size(&format!("{}macbytes", p), 16);
    chk_size(&format!("{}sealbytes", p), 48);
    assert_eq!(
        chk_size_eq_only(&format!("{}messagebytes_max", p)),
        usize::MAX - 16
    );

    // crypto_kx
    chk_size("crypto_kx_publickeybytes", 32);
    chk_size("crypto_kx_secretkeybytes", 32);
    chk_size("crypto_kx_seedbytes", 32);
    chk_size("crypto_kx_sessionkeybytes", 32);
    chk_str("crypto_kx_primitive", "x25519blake2b");

    // crypto_kem (== xwing)
    chk_size("crypto_kem_publickeybytes", 1216);
    chk_size("crypto_kem_secretkeybytes", 32);
    chk_size("crypto_kem_ciphertextbytes", 1120);
    chk_size("crypto_kem_sharedsecretbytes", 32);
    chk_size("crypto_kem_seedbytes", 32);
    chk_str("crypto_kem_primitive", "xwing");

    chk_size("crypto_kem_mlkem768_publickeybytes", 1184);
    chk_size("crypto_kem_mlkem768_secretkeybytes", 2400);
    chk_size("crypto_kem_mlkem768_ciphertextbytes", 1088);
    chk_size("crypto_kem_mlkem768_sharedsecretbytes", 32);
    chk_size("crypto_kem_mlkem768_seedbytes", 64);

    chk_size("crypto_kem_xwing_publickeybytes", 1216);
    chk_size("crypto_kem_xwing_secretkeybytes", 32);
    chk_size("crypto_kem_xwing_ciphertextbytes", 1120);
    chk_size("crypto_kem_xwing_sharedsecretbytes", 32);
    chk_size("crypto_kem_xwing_seedbytes", 32);
}

// ================================================= keypairs / beforenm =======

fn keypair_suite(prefix: &str, tag: &str) {
    let seed_kp_api = syms::<SeedKp>(&format!("{}seed_keypair", prefix));
    let kp_api = syms::<Kp>(&format!("{}keypair", prefix));
    let bnm = syms::<Beforenm>(&format!("{}beforenm", prefix));

    let mut rng = common::Rng::new(0x5EED_0001 ^ (tag.len() as u64));

    // -- deterministic seed_keypair: byte-exact ---------------------------
    for i in 0..24 {
        let mut seed = [0u8; 32];
        rng.fill(&mut seed);
        if i == 0 {
            seed = [0u8; 32];
        }
        if i == 1 {
            seed = [0xffu8; 32];
        }
        let (pk, sk) = seed_kp(&seed_kp_api, &seed, tag);
        // beforenm on our own key pair must be byte-identical too.
        let (rc, _) = beforenm_both(&bnm, &pk, &sk, tag);
        assert_eq!(rc, 0, "{} beforenm(own pk, own sk)", tag);
    }

    // -- beforenm across two distinct key pairs (both directions) ---------
    for _ in 0..24 {
        let mut s1 = [0u8; 32];
        let mut s2 = [0u8; 32];
        rng.fill(&mut s1);
        rng.fill(&mut s2);
        let (pk1, sk1) = seed_kp(&seed_kp_api, &s1, tag);
        let (pk2, sk2) = seed_kp(&seed_kp_api, &s2, tag);
        let (rc1, k1) = beforenm_both(&bnm, &pk2, &sk1, tag);
        let (rc2, k2) = beforenm_both(&bnm, &pk1, &sk2, tag);
        assert_eq!((rc1, rc2), (0, 0), "{} beforenm rc", tag);
        common::eqb(&format!("{} beforenm DH agreement", tag), &k1, &k2);
    }

    // -- beforenm with a public key that makes scalarmult fail -> -1 ------
    let (pk, sk) = seed_kp(&seed_kp_api, &[7u8; 32], tag);
    for (i, bad) in bad_public_keys().iter().enumerate() {
        let mut kc = [CANARY; KB];
        let mut kr = [CANARY; KB];
        let rc = unsafe { bnm.0(kc.as_mut_ptr(), bad.as_ptr(), sk.as_ptr()) };
        let rr = unsafe { bnm.1(kr.as_mut_ptr(), bad.as_ptr(), sk.as_ptr()) };
        common::eqi(&format!("{} beforenm(bad pk #{}) rc", tag, i), rc, rr);
        assert_eq!(rc, -1, "{} beforenm(bad pk #{}) must be -1", tag, i);
        common::eqb(&format!("{} beforenm(bad pk #{}) k", tag, i), &kc, &kr);
        // C leaves k untouched on the error path.
        assert_eq!(kc, [CANARY; KB], "{} beforenm error must not write k", tag);
    }

    // -- randombytes-driven keypair: return code + cross-library DH -------
    for _ in 0..8 {
        let mut pkc = [CANARY; PKB];
        let mut skc = [CANARY; SKB];
        let mut pkr = [CANARY; PKB];
        let mut skr = [CANARY; SKB];
        let rc = unsafe { kp_api.0(pkc.as_mut_ptr(), skc.as_mut_ptr()) };
        let rr = unsafe { kp_api.1(pkr.as_mut_ptr(), skr.as_mut_ptr()) };
        common::eqi(&format!("{} keypair rc", tag), rc, rr);
        assert_eq!(rc, 0, "{} keypair", tag);
        assert_ne!(skc, [CANARY; SKB], "{} keypair did not write sk", tag);
        assert_ne!(pkc, [CANARY; PKB], "{} keypair did not write pk", tag);
        // C-generated secret x Rust-generated public, computed by the C lib,
        // must equal Rust-generated secret x C-generated public computed by
        // the Rust lib.
        let mut k1 = [0u8; KB];
        let mut k2 = [0u8; KB];
        let a = unsafe { bnm.0(k1.as_mut_ptr(), pkr.as_ptr(), skc.as_ptr()) };
        let b = unsafe { bnm.1(k2.as_mut_ptr(), pkc.as_ptr(), skr.as_ptr()) };
        assert_eq!((a, b), (0, 0), "{} cross keypair beforenm rc", tag);
        common::eqb(&format!("{} cross-library DH", tag), &k1, &k2);
    }

    // sanity: unrelated pk/sk must not agree
    let (pk2, _sk2) = seed_kp(&seed_kp_api, &[9u8; 32], tag);
    let (_, ka) = beforenm_both(&bnm, &pk, &sk, tag);
    let (_, kb) = beforenm_both(&bnm, &pk2, &sk, tag);
    assert_ne!(ka, kb, "{} beforenm should depend on pk", tag);
}

#[test]
fn box_keypairs_salsa_generic() {
    keypair_suite("crypto_box_", "box");
}

#[test]
fn box_keypairs_salsa_named() {
    keypair_suite(
        "crypto_box_curve25519xsalsa20poly1305_",
        "box/xsalsa20poly1305",
    );
}

#[test]
fn box_keypairs_xchacha() {
    keypair_suite(
        "crypto_box_curve25519xchacha20poly1305_",
        "box/xchacha20poly1305",
    );
}

// ============================================ low-level zero-padded API ======

/// Covers `crypto_box`/`crypto_box_open` and `*_afternm`/`*_open_afternm`
/// (the NaCl-style API where the first ZEROBYTES of the plaintext and the
/// first BOXZEROBYTES of the ciphertext must be zero).
fn lowlevel_suite(box_name: &str, prefix: &str, tag: &str) {
    let bx = syms::<BoxFull>(box_name);
    let op = syms::<BoxFull>(&format!("{}open", prefix));
    let anm = syms::<Afternm>(&format!("{}afternm", prefix));
    let oanm = syms::<Afternm>(&format!("{}open_afternm", prefix));
    let seed_kp_api = syms::<SeedKp>(&format!("{}seed_keypair", prefix));
    let bnm = syms::<Beforenm>(&format!("{}beforenm", prefix));

    let mut rng = common::Rng::new(0xB0F1_2233 ^ tag.len() as u64);

    let (pka, ska) = seed_kp(&seed_kp_api, &[1u8; 32], tag);
    let (pkb, skb) = seed_kp(&seed_kp_api, &[2u8; 32], tag);
    let (_, k) = beforenm_both(&bnm, &pkb, &ska, tag);

    let mut nonce = [0u8; NB];
    rng.fill(&mut nonce);

    // mlen < ZEROBYTES is rejected by crypto_secretbox_xsalsa20poly1305.
    for mlen in 0..ZB {
        let m = vec![0u8; mlen.max(1)];
        let mut cc = vec![CANARY; mlen.max(1)];
        let mut cr = vec![CANARY; mlen.max(1)];
        let rc = unsafe {
            bx.0(
                cc.as_mut_ptr(),
                m.as_ptr(),
                mlen as u64,
                nonce.as_ptr(),
                pkb.as_ptr(),
                ska.as_ptr(),
            )
        };
        let rr = unsafe {
            bx.1(
                cr.as_mut_ptr(),
                m.as_ptr(),
                mlen as u64,
                nonce.as_ptr(),
                pkb.as_ptr(),
                ska.as_ptr(),
            )
        };
        common::eqi(&format!("{} box(mlen={}) rc", tag, mlen), rc, rr);
        assert_eq!(rc, -1, "{} box(mlen={}) must be -1", tag, mlen);
        common::eqb(&format!("{} box(mlen={}) out", tag, mlen), &cc, &cr);

        let rc = unsafe {
            anm.0(
                cc.as_mut_ptr(),
                m.as_ptr(),
                mlen as u64,
                nonce.as_ptr(),
                k.as_ptr(),
            )
        };
        let rr = unsafe {
            anm.1(
                cr.as_mut_ptr(),
                m.as_ptr(),
                mlen as u64,
                nonce.as_ptr(),
                k.as_ptr(),
            )
        };
        common::eqi(&format!("{} afternm(mlen={}) rc", tag, mlen), rc, rr);
        assert_eq!(rc, -1, "{} afternm(mlen={}) must be -1", tag, mlen);

        // ... and the same for the open direction (clen < ZEROBYTES).
        let cin = vec![0u8; mlen.max(1)];
        let mut mc = vec![CANARY; mlen.max(1)];
        let mut mr = vec![CANARY; mlen.max(1)];
        let rc = unsafe {
            op.0(
                mc.as_mut_ptr(),
                cin.as_ptr(),
                mlen as u64,
                nonce.as_ptr(),
                pka.as_ptr(),
                skb.as_ptr(),
            )
        };
        let rr = unsafe {
            op.1(
                mr.as_mut_ptr(),
                cin.as_ptr(),
                mlen as u64,
                nonce.as_ptr(),
                pka.as_ptr(),
                skb.as_ptr(),
            )
        };
        common::eqi(&format!("{} open(clen={}) rc", tag, mlen), rc, rr);
        assert_eq!(rc, -1, "{} open(clen={}) must be -1", tag, mlen);
        common::eqb(&format!("{} open(clen={}) out", tag, mlen), &mc, &mr);

        let rc = unsafe {
            oanm.0(
                mc.as_mut_ptr(),
                cin.as_ptr(),
                mlen as u64,
                nonce.as_ptr(),
                k.as_ptr(),
            )
        };
        let rr = unsafe {
            oanm.1(
                mr.as_mut_ptr(),
                cin.as_ptr(),
                mlen as u64,
                nonce.as_ptr(),
                k.as_ptr(),
            )
        };
        common::eqi(&format!("{} open_afternm(clen={}) rc", tag, mlen), rc, rr);
        assert_eq!(rc, -1, "{} open_afternm(clen={}) must be -1", tag, mlen);
    }

    // Valid sizes: mlen = ZEROBYTES + payload.
    for payload in [0usize, 1, 15, 16, 17, 31, 32, 33, 63, 64, 65, 1000] {
        let mlen = ZB + payload;
        let mut m = vec![0u8; mlen];
        rng.fill(&mut m[ZB..]);
        let mut cc = vec![CANARY; mlen + 8];
        let mut cr = vec![CANARY; mlen + 8];
        let rc = unsafe {
            bx.0(
                cc.as_mut_ptr(),
                m.as_ptr(),
                mlen as u64,
                nonce.as_ptr(),
                pkb.as_ptr(),
                ska.as_ptr(),
            )
        };
        let rr = unsafe {
            bx.1(
                cr.as_mut_ptr(),
                m.as_ptr(),
                mlen as u64,
                nonce.as_ptr(),
                pkb.as_ptr(),
                ska.as_ptr(),
            )
        };
        common::eqi(&format!("{} box(payload={}) rc", tag, payload), rc, rr);
        assert_eq!(rc, 0, "{} box(payload={})", tag, payload);
        common::eqb(&format!("{} box(payload={}) c", tag, payload), &cc, &cr);
        assert_eq!(&cc[..BZB], &[0u8; BZB], "{} box: BOXZEROBYTES", tag);
        assert_eq!(
            &cc[mlen..],
            &[CANARY; 8],
            "{} box wrote past the end",
            tag
        );

        // afternm must produce the same ciphertext as the full call
        let mut ca = vec![CANARY; mlen + 8];
        let mut cb = vec![CANARY; mlen + 8];
        let rc = unsafe {
            anm.0(
                ca.as_mut_ptr(),
                m.as_ptr(),
                mlen as u64,
                nonce.as_ptr(),
                k.as_ptr(),
            )
        };
        let rr = unsafe {
            anm.1(
                cb.as_mut_ptr(),
                m.as_ptr(),
                mlen as u64,
                nonce.as_ptr(),
                k.as_ptr(),
            )
        };
        common::eqi(&format!("{} afternm(payload={}) rc", tag, payload), rc, rr);
        common::eqb(&format!("{} afternm vs box", tag), &cc, &ca);
        common::eqb(&format!("{} afternm C vs Rust", tag), &ca, &cb);

        // open
        let mut mc = vec![CANARY; mlen + 8];
        let mut mr = vec![CANARY; mlen + 8];
        let rc = unsafe {
            op.0(
                mc.as_mut_ptr(),
                cc.as_ptr(),
                mlen as u64,
                nonce.as_ptr(),
                pka.as_ptr(),
                skb.as_ptr(),
            )
        };
        let rr = unsafe {
            op.1(
                mr.as_mut_ptr(),
                cc.as_ptr(),
                mlen as u64,
                nonce.as_ptr(),
                pka.as_ptr(),
                skb.as_ptr(),
            )
        };
        common::eqi(&format!("{} open(payload={}) rc", tag, payload), rc, rr);
        assert_eq!(rc, 0, "{} open(payload={})", tag, payload);
        common::eqb(&format!("{} open(payload={}) m", tag, payload), &mc, &mr);
        assert_eq!(&mc[..mlen], &m[..], "{} open round trip", tag);
        assert_eq!(&mc[mlen..], &[CANARY; 8], "{} open overran", tag);

        // open_afternm
        let mut ma = vec![CANARY; mlen + 8];
        let mut mb = vec![CANARY; mlen + 8];
        let rc = unsafe {
            oanm.0(
                ma.as_mut_ptr(),
                cc.as_ptr(),
                mlen as u64,
                nonce.as_ptr(),
                k.as_ptr(),
            )
        };
        let rr = unsafe {
            oanm.1(
                mb.as_mut_ptr(),
                cc.as_ptr(),
                mlen as u64,
                nonce.as_ptr(),
                k.as_ptr(),
            )
        };
        common::eqi(&format!("{} open_afternm rc", tag), rc, rr);
        assert_eq!(rc, 0, "{} open_afternm", tag);
        common::eqb(&format!("{} open_afternm", tag), &ma, &mb);
        common::eqb(&format!("{} open_afternm vs open", tag), &ma, &mc);
    }

    // In-place (c == m).
    for payload in [0usize, 1, 17, 64, 1000] {
        let mlen = ZB + payload;
        let mut m = vec![0u8; mlen];
        rng.fill(&mut m[ZB..]);
        let mut bc = m.clone();
        let mut br = m.clone();
        let rc = unsafe {
            bx.0(
                bc.as_mut_ptr(),
                bc.as_ptr(),
                mlen as u64,
                nonce.as_ptr(),
                pkb.as_ptr(),
                ska.as_ptr(),
            )
        };
        let rr = unsafe {
            bx.1(
                br.as_mut_ptr(),
                br.as_ptr(),
                mlen as u64,
                nonce.as_ptr(),
                pkb.as_ptr(),
                ska.as_ptr(),
            )
        };
        common::eqi(&format!("{} in-place box rc", tag), rc, rr);
        common::eqb(&format!("{} in-place box", tag), &bc, &br);

        let mut dc = bc.clone();
        let mut dr = bc.clone();
        let rc = unsafe {
            op.0(
                dc.as_mut_ptr(),
                dc.as_ptr(),
                mlen as u64,
                nonce.as_ptr(),
                pka.as_ptr(),
                skb.as_ptr(),
            )
        };
        let rr = unsafe {
            op.1(
                dr.as_mut_ptr(),
                dr.as_ptr(),
                mlen as u64,
                nonce.as_ptr(),
                pka.as_ptr(),
                skb.as_ptr(),
            )
        };
        common::eqi(&format!("{} in-place open rc", tag), rc, rr);
        common::eqb(&format!("{} in-place open", tag), &dc, &dr);
        assert_eq!(&dc[..], &m[..], "{} in-place round trip", tag);
    }

    // Tampering: flip one bit in every byte of the ciphertext.
    // Note bytes [0,BOXZEROBYTES) of the ciphertext are not read by `open`
    // at all, so tampering there still succeeds -- we compare both the return
    // code and the output buffer, which covers that.
    for payload in [0usize, 1, 33] {
        let mlen = ZB + payload;
        let mut m = vec![0u8; mlen];
        rng.fill(&mut m[ZB..]);
        let mut ct = vec![0u8; mlen];
        let rc = unsafe {
            bx.0(
                ct.as_mut_ptr(),
                m.as_ptr(),
                mlen as u64,
                nonce.as_ptr(),
                pkb.as_ptr(),
                ska.as_ptr(),
            )
        };
        assert_eq!(rc, 0);
        for i in 0..mlen {
            let mut bad = ct.clone();
            bad[i] ^= 0x80;
            let mut mc = vec![CANARY; mlen];
            let mut mr = vec![CANARY; mlen];
            let rc = unsafe {
                op.0(
                    mc.as_mut_ptr(),
                    bad.as_ptr(),
                    mlen as u64,
                    nonce.as_ptr(),
                    pka.as_ptr(),
                    skb.as_ptr(),
                )
            };
            let rr = unsafe {
                op.1(
                    mr.as_mut_ptr(),
                    bad.as_ptr(),
                    mlen as u64,
                    nonce.as_ptr(),
                    pka.as_ptr(),
                    skb.as_ptr(),
                )
            };
            common::eqi(&format!("{} tamper[{}] rc", tag, i), rc, rr);
            common::eqb(&format!("{} tamper[{}] out", tag, i), &mc, &mr);
            if i >= BZB {
                assert_eq!(rc, -1, "{} tamper at {} should fail", tag, i);
            }
        }
        // wrong nonce / wrong key
        let mut n2 = nonce;
        n2[0] ^= 1;
        let mut mc = vec![CANARY; mlen];
        let mut mr = vec![CANARY; mlen];
        let rc = unsafe {
            op.0(
                mc.as_mut_ptr(),
                ct.as_ptr(),
                mlen as u64,
                n2.as_ptr(),
                pka.as_ptr(),
                skb.as_ptr(),
            )
        };
        let rr = unsafe {
            op.1(
                mr.as_mut_ptr(),
                ct.as_ptr(),
                mlen as u64,
                n2.as_ptr(),
                pka.as_ptr(),
                skb.as_ptr(),
            )
        };
        common::eqi(&format!("{} wrong nonce rc", tag), rc, rr);
        assert_eq!(rc, -1);
        common::eqb(&format!("{} wrong nonce out", tag), &mc, &mr);
    }

    // Bad public key -> -1 from beforenm, propagated by box / open.
    let mlen = ZB + 16;
    let m = vec![0u8; mlen];
    for (i, bad) in bad_public_keys().iter().enumerate() {
        let mut cc = vec![CANARY; mlen];
        let mut cr = vec![CANARY; mlen];
        let rc = unsafe {
            bx.0(
                cc.as_mut_ptr(),
                m.as_ptr(),
                mlen as u64,
                nonce.as_ptr(),
                bad.as_ptr(),
                ska.as_ptr(),
            )
        };
        let rr = unsafe {
            bx.1(
                cr.as_mut_ptr(),
                m.as_ptr(),
                mlen as u64,
                nonce.as_ptr(),
                bad.as_ptr(),
                ska.as_ptr(),
            )
        };
        common::eqi(&format!("{} box(bad pk #{}) rc", tag, i), rc, rr);
        assert_eq!(rc, -1);
        common::eqb(&format!("{} box(bad pk #{}) out", tag, i), &cc, &cr);

        let rc = unsafe {
            op.0(
                cc.as_mut_ptr(),
                m.as_ptr(),
                mlen as u64,
                nonce.as_ptr(),
                bad.as_ptr(),
                ska.as_ptr(),
            )
        };
        let rr = unsafe {
            op.1(
                cr.as_mut_ptr(),
                m.as_ptr(),
                mlen as u64,
                nonce.as_ptr(),
                bad.as_ptr(),
                ska.as_ptr(),
            )
        };
        common::eqi(&format!("{} open(bad pk #{}) rc", tag, i), rc, rr);
        assert_eq!(rc, -1);
    }
}

#[test]
fn box_lowlevel_generic() {
    lowlevel_suite("crypto_box", "crypto_box_", "box(low)");
}

#[test]
fn box_lowlevel_salsa_named() {
    lowlevel_suite(
        "crypto_box_curve25519xsalsa20poly1305",
        "crypto_box_curve25519xsalsa20poly1305_",
        "xsalsa(low)",
    );
}

// ==================================================== easy / detached ========

fn easy_suite(prefix: &str, tag: &str) {
    let easy = syms::<BoxFull>(&format!("{}easy", prefix));
    let open_easy = syms::<BoxFull>(&format!("{}open_easy", prefix));
    let easy_a = syms::<Afternm>(&format!("{}easy_afternm", prefix));
    let open_easy_a = syms::<Afternm>(&format!("{}open_easy_afternm", prefix));
    let det = syms::<Detached>(&format!("{}detached", prefix));
    let open_det = syms::<OpenDetached>(&format!("{}open_detached", prefix));
    let det_a = syms::<DetachedAfternm>(&format!("{}detached_afternm", prefix));
    let open_det_a = syms::<OpenDetachedAfternm>(&format!("{}open_detached_afternm", prefix));
    let seed_kp_api = syms::<SeedKp>(&format!("{}seed_keypair", prefix));
    let bnm = syms::<Beforenm>(&format!("{}beforenm", prefix));

    let mut rng = common::Rng::new(0xEA5F_0000u64 ^ tag.len() as u64);

    let (pka, ska) = seed_kp(&seed_kp_api, &[3u8; 32], tag);
    let (pkb, skb) = seed_kp(&seed_kp_api, &[4u8; 32], tag);
    let (_, k) = beforenm_both(&bnm, &pkb, &ska, tag);
    let (_, k2) = beforenm_both(&bnm, &pka, &skb, tag);
    common::eqb(&format!("{} beforenm symmetry", tag), &k, &k2);

    let sizes = [0usize, 1, 15, 16, 17, 31, 32, 33, 47, 48, 49, 63, 64, 65, 1000];

    for &mlen in sizes.iter() {
        let m = rng.bytes(mlen);
        let mut nonce = [0u8; NB];
        rng.fill(&mut nonce);
        let clen = mlen + MACB;

        // ---- easy -------------------------------------------------------
        let mut cc = vec![CANARY; clen + 8];
        let mut cr = vec![CANARY; clen + 8];
        let rc = unsafe {
            easy.0(
                cc.as_mut_ptr(),
                m.as_ptr(),
                mlen as u64,
                nonce.as_ptr(),
                pkb.as_ptr(),
                ska.as_ptr(),
            )
        };
        let rr = unsafe {
            easy.1(
                cr.as_mut_ptr(),
                m.as_ptr(),
                mlen as u64,
                nonce.as_ptr(),
                pkb.as_ptr(),
                ska.as_ptr(),
            )
        };
        common::eqi(&format!("{} easy(mlen={}) rc", tag, mlen), rc, rr);
        assert_eq!(rc, 0, "{} easy(mlen={})", tag, mlen);
        common::eqb(&format!("{} easy(mlen={}) c", tag, mlen), &cc, &cr);
        assert_eq!(&cc[clen..], &[CANARY; 8], "{} easy overran", tag);

        // ---- easy_afternm ----------------------------------------------
        let mut ac = vec![CANARY; clen + 8];
        let mut ar = vec![CANARY; clen + 8];
        let rc = unsafe {
            easy_a.0(
                ac.as_mut_ptr(),
                m.as_ptr(),
                mlen as u64,
                nonce.as_ptr(),
                k.as_ptr(),
            )
        };
        let rr = unsafe {
            easy_a.1(
                ar.as_mut_ptr(),
                m.as_ptr(),
                mlen as u64,
                nonce.as_ptr(),
                k.as_ptr(),
            )
        };
        common::eqi(&format!("{} easy_afternm rc", tag), rc, rr);
        common::eqb(&format!("{} easy_afternm", tag), &ac, &ar);
        common::eqb(&format!("{} easy_afternm vs easy", tag), &ac, &cc);

        // ---- detached ---------------------------------------------------
        let mut dc = vec![CANARY; mlen + 8];
        let mut dr = vec![CANARY; mlen + 8];
        let mut macc = [CANARY; MACB + 4];
        let mut macr = [CANARY; MACB + 4];
        let rc = unsafe {
            det.0(
                dc.as_mut_ptr(),
                macc.as_mut_ptr(),
                m.as_ptr(),
                mlen as u64,
                nonce.as_ptr(),
                pkb.as_ptr(),
                ska.as_ptr(),
            )
        };
        let rr = unsafe {
            det.1(
                dr.as_mut_ptr(),
                macr.as_mut_ptr(),
                m.as_ptr(),
                mlen as u64,
                nonce.as_ptr(),
                pkb.as_ptr(),
                ska.as_ptr(),
            )
        };
        common::eqi(&format!("{} detached rc", tag), rc, rr);
        assert_eq!(rc, 0);
        common::eqb(&format!("{} detached c", tag), &dc, &dr);
        common::eqb(&format!("{} detached mac", tag), &macc, &macr);
        // easy == mac || c
        assert_eq!(&cc[..MACB], &macc[..MACB], "{} easy layout mac", tag);
        assert_eq!(&cc[MACB..clen], &dc[..mlen], "{} easy layout body", tag);

        // ---- detached_afternm ------------------------------------------
        let mut ec = vec![CANARY; mlen + 8];
        let mut er = vec![CANARY; mlen + 8];
        let mut mc2 = [CANARY; MACB + 4];
        let mut mr2 = [CANARY; MACB + 4];
        let rc = unsafe {
            det_a.0(
                ec.as_mut_ptr(),
                mc2.as_mut_ptr(),
                m.as_ptr(),
                mlen as u64,
                nonce.as_ptr(),
                k.as_ptr(),
            )
        };
        let rr = unsafe {
            det_a.1(
                er.as_mut_ptr(),
                mr2.as_mut_ptr(),
                m.as_ptr(),
                mlen as u64,
                nonce.as_ptr(),
                k.as_ptr(),
            )
        };
        common::eqi(&format!("{} detached_afternm rc", tag), rc, rr);
        common::eqb(&format!("{} detached_afternm c", tag), &ec, &er);
        common::eqb(&format!("{} detached_afternm mac", tag), &mc2, &mr2);
        common::eqb(&format!("{} detached_afternm == detached", tag), &ec, &dc);

        // ---- open_easy --------------------------------------------------
        let mut oc = vec![CANARY; mlen + 8];
        let mut or_ = vec![CANARY; mlen + 8];
        let rc = unsafe {
            open_easy.0(
                oc.as_mut_ptr(),
                cc.as_ptr(),
                clen as u64,
                nonce.as_ptr(),
                pka.as_ptr(),
                skb.as_ptr(),
            )
        };
        let rr = unsafe {
            open_easy.1(
                or_.as_mut_ptr(),
                cc.as_ptr(),
                clen as u64,
                nonce.as_ptr(),
                pka.as_ptr(),
                skb.as_ptr(),
            )
        };
        common::eqi(&format!("{} open_easy rc", tag), rc, rr);
        assert_eq!(rc, 0, "{} open_easy(mlen={})", tag, mlen);
        common::eqb(&format!("{} open_easy m", tag), &oc, &or_);
        assert_eq!(&oc[..mlen], &m[..], "{} open_easy round trip", tag);
        assert_eq!(&oc[mlen..], &[CANARY; 8], "{} open_easy overran", tag);

        // ---- open_easy_afternm -----------------------------------------
        let mut oc2 = vec![CANARY; mlen + 8];
        let mut or2 = vec![CANARY; mlen + 8];
        let rc = unsafe {
            open_easy_a.0(
                oc2.as_mut_ptr(),
                cc.as_ptr(),
                clen as u64,
                nonce.as_ptr(),
                k.as_ptr(),
            )
        };
        let rr = unsafe {
            open_easy_a.1(
                or2.as_mut_ptr(),
                cc.as_ptr(),
                clen as u64,
                nonce.as_ptr(),
                k.as_ptr(),
            )
        };
        common::eqi(&format!("{} open_easy_afternm rc", tag), rc, rr);
        common::eqb(&format!("{} open_easy_afternm", tag), &oc2, &or2);
        common::eqb(&format!("{} open_easy_afternm vs open_easy", tag), &oc2, &oc);

        // ---- open_detached ---------------------------------------------
        let mut pc = vec![CANARY; mlen + 8];
        let mut pr = vec![CANARY; mlen + 8];
        let rc = unsafe {
            open_det.0(
                pc.as_mut_ptr(),
                dc.as_ptr(),
                macc.as_ptr(),
                mlen as u64,
                nonce.as_ptr(),
                pka.as_ptr(),
                skb.as_ptr(),
            )
        };
        let rr = unsafe {
            open_det.1(
                pr.as_mut_ptr(),
                dc.as_ptr(),
                macc.as_ptr(),
                mlen as u64,
                nonce.as_ptr(),
                pka.as_ptr(),
                skb.as_ptr(),
            )
        };
        common::eqi(&format!("{} open_detached rc", tag), rc, rr);
        assert_eq!(rc, 0);
        common::eqb(&format!("{} open_detached m", tag), &pc, &pr);
        assert_eq!(&pc[..mlen], &m[..]);

        // ---- open_detached_afternm -------------------------------------
        let mut qc = vec![CANARY; mlen + 8];
        let mut qr = vec![CANARY; mlen + 8];
        let rc = unsafe {
            open_det_a.0(
                qc.as_mut_ptr(),
                dc.as_ptr(),
                macc.as_ptr(),
                mlen as u64,
                nonce.as_ptr(),
                k.as_ptr(),
            )
        };
        let rr = unsafe {
            open_det_a.1(
                qr.as_mut_ptr(),
                dc.as_ptr(),
                macc.as_ptr(),
                mlen as u64,
                nonce.as_ptr(),
                k.as_ptr(),
            )
        };
        common::eqi(&format!("{} open_detached_afternm rc", tag), rc, rr);
        common::eqb(&format!("{} open_detached_afternm", tag), &qc, &qr);

        // ---- open_detached with m == NULL (C tolerates it) --------------
        let rc = unsafe {
            open_det.0(
                core::ptr::null_mut(),
                dc.as_ptr(),
                macc.as_ptr(),
                mlen as u64,
                nonce.as_ptr(),
                pka.as_ptr(),
                skb.as_ptr(),
            )
        };
        let rr = unsafe {
            open_det.1(
                core::ptr::null_mut(),
                dc.as_ptr(),
                macc.as_ptr(),
                mlen as u64,
                nonce.as_ptr(),
                pka.as_ptr(),
                skb.as_ptr(),
            )
        };
        common::eqi(&format!("{} open_detached(m=NULL) rc", tag), rc, rr);
        assert_eq!(rc, 0, "{} open_detached(m=NULL) must verify", tag);

        let rc = unsafe {
            open_easy.0(
                core::ptr::null_mut(),
                cc.as_ptr(),
                clen as u64,
                nonce.as_ptr(),
                pka.as_ptr(),
                skb.as_ptr(),
            )
        };
        let rr = unsafe {
            open_easy.1(
                core::ptr::null_mut(),
                cc.as_ptr(),
                clen as u64,
                nonce.as_ptr(),
                pka.as_ptr(),
                skb.as_ptr(),
            )
        };
        common::eqi(&format!("{} open_easy(m=NULL) rc", tag), rc, rr);
        assert_eq!(rc, 0);

        // ---- afternm variants with m == NULL ----------------------------
        let rc = unsafe {
            open_easy_a.0(
                core::ptr::null_mut(),
                cc.as_ptr(),
                clen as u64,
                nonce.as_ptr(),
                k.as_ptr(),
            )
        };
        let rr = unsafe {
            open_easy_a.1(
                core::ptr::null_mut(),
                cc.as_ptr(),
                clen as u64,
                nonce.as_ptr(),
                k.as_ptr(),
            )
        };
        common::eqi(&format!("{} open_easy_afternm(m=NULL) rc", tag), rc, rr);
        assert_eq!(rc, 0);

        let rc = unsafe {
            open_det_a.0(
                core::ptr::null_mut(),
                dc.as_ptr(),
                macc.as_ptr(),
                mlen as u64,
                nonce.as_ptr(),
                k.as_ptr(),
            )
        };
        let rr = unsafe {
            open_det_a.1(
                core::ptr::null_mut(),
                dc.as_ptr(),
                macc.as_ptr(),
                mlen as u64,
                nonce.as_ptr(),
                k.as_ptr(),
            )
        };
        common::eqi(
            &format!("{} open_detached_afternm(m=NULL) rc", tag),
            rc,
            rr,
        );
        assert_eq!(rc, 0);

        // ---- detached in place (c == m) --------------------------------
        let mut ipc = m.clone();
        let mut ipr = m.clone();
        let mut mip_c = [CANARY; MACB];
        let mut mip_r = [CANARY; MACB];
        let rc = unsafe {
            det.0(
                ipc.as_mut_ptr(),
                mip_c.as_mut_ptr(),
                ipc.as_ptr(),
                mlen as u64,
                nonce.as_ptr(),
                pkb.as_ptr(),
                ska.as_ptr(),
            )
        };
        let rr = unsafe {
            det.1(
                ipr.as_mut_ptr(),
                mip_r.as_mut_ptr(),
                ipr.as_ptr(),
                mlen as u64,
                nonce.as_ptr(),
                pkb.as_ptr(),
                ska.as_ptr(),
            )
        };
        common::eqi(&format!("{} detached in-place rc", tag), rc, rr);
        common::eqb(&format!("{} detached in-place c", tag), &ipc, &ipr);
        common::eqb(&format!("{} detached in-place mac", tag), &mip_c, &mip_r);
        assert_eq!(&ipc[..], &dc[..mlen], "{} detached in-place == normal", tag);
        assert_eq!(&mip_c[..], &macc[..MACB]);

        // ---- open_detached in place (m == c) ---------------------------
        let mut opc = ipc.clone();
        let mut opr = ipc.clone();
        let rc = unsafe {
            open_det.0(
                opc.as_mut_ptr(),
                opc.as_ptr(),
                mip_c.as_ptr(),
                mlen as u64,
                nonce.as_ptr(),
                pka.as_ptr(),
                skb.as_ptr(),
            )
        };
        let rr = unsafe {
            open_det.1(
                opr.as_mut_ptr(),
                opr.as_ptr(),
                mip_c.as_ptr(),
                mlen as u64,
                nonce.as_ptr(),
                pka.as_ptr(),
                skb.as_ptr(),
            )
        };
        common::eqi(&format!("{} open_detached in-place rc", tag), rc, rr);
        assert_eq!(rc, 0);
        common::eqb(&format!("{} open_detached in-place", tag), &opc, &opr);
        assert_eq!(&opc[..], &m[..], "{} open_detached in-place round trip", tag);
    }

    // ---- in-place encryption: m == c + MACBYTES -------------------------
    for &mlen in [0usize, 1, 16, 17, 64, 1000].iter() {
        let m = rng.bytes(mlen);
        let mut nonce = [0u8; NB];
        rng.fill(&mut nonce);
        let clen = mlen + MACB;

        let mut bc = vec![CANARY; clen];
        bc[MACB..].copy_from_slice(&m);
        let mut br = bc.clone();
        let rc = unsafe {
            easy.0(
                bc.as_mut_ptr(),
                bc.as_ptr().add(MACB),
                mlen as u64,
                nonce.as_ptr(),
                pkb.as_ptr(),
                ska.as_ptr(),
            )
        };
        let rr = unsafe {
            easy.1(
                br.as_mut_ptr(),
                br.as_ptr().add(MACB),
                mlen as u64,
                nonce.as_ptr(),
                pkb.as_ptr(),
                ska.as_ptr(),
            )
        };
        common::eqi(&format!("{} in-place easy rc", tag), rc, rr);
        common::eqb(&format!("{} in-place easy", tag), &bc, &br);

        // reference (non in-place) ciphertext must be identical
        let mut refc = vec![0u8; clen];
        unsafe {
            easy.0(
                refc.as_mut_ptr(),
                m.as_ptr(),
                mlen as u64,
                nonce.as_ptr(),
                pkb.as_ptr(),
                ska.as_ptr(),
            )
        };
        common::eqb(&format!("{} in-place == out-of-place", tag), &bc, &refc);

        // in-place decryption: m == c
        let mut oc = bc.clone();
        let mut or_ = bc.clone();
        let rc = unsafe {
            open_easy.0(
                oc.as_mut_ptr(),
                oc.as_ptr(),
                clen as u64,
                nonce.as_ptr(),
                pka.as_ptr(),
                skb.as_ptr(),
            )
        };
        let rr = unsafe {
            open_easy.1(
                or_.as_mut_ptr(),
                or_.as_ptr(),
                clen as u64,
                nonce.as_ptr(),
                pka.as_ptr(),
                skb.as_ptr(),
            )
        };
        common::eqi(&format!("{} in-place open_easy rc", tag), rc, rr);
        assert_eq!(rc, 0);
        common::eqb(&format!("{} in-place open_easy", tag), &oc, &or_);
        assert_eq!(&oc[..mlen], &m[..], "{} in-place open round trip", tag);
    }

    // ---- clen < MACBYTES -> -1 -----------------------------------------
    let dummy = [0u8; MACB];
    let mut nonce = [0u8; NB];
    rng.fill(&mut nonce);
    for clen in 0..MACB {
        let mut mc = [CANARY; 64];
        let mut mr = [CANARY; 64];
        let rc = unsafe {
            open_easy.0(
                mc.as_mut_ptr(),
                dummy.as_ptr(),
                clen as u64,
                nonce.as_ptr(),
                pka.as_ptr(),
                skb.as_ptr(),
            )
        };
        let rr = unsafe {
            open_easy.1(
                mr.as_mut_ptr(),
                dummy.as_ptr(),
                clen as u64,
                nonce.as_ptr(),
                pka.as_ptr(),
                skb.as_ptr(),
            )
        };
        common::eqi(&format!("{} open_easy(clen={}) rc", tag, clen), rc, rr);
        assert_eq!(rc, -1, "{} open_easy(clen={}) must be -1", tag, clen);
        common::eqb(&format!("{} open_easy(clen={}) out", tag, clen), &mc, &mr);
        assert_eq!(mc, [CANARY; 64], "{} short clen must not write", tag);

        let rc = unsafe {
            open_easy_a.0(
                mc.as_mut_ptr(),
                dummy.as_ptr(),
                clen as u64,
                nonce.as_ptr(),
                k.as_ptr(),
            )
        };
        let rr = unsafe {
            open_easy_a.1(
                mr.as_mut_ptr(),
                dummy.as_ptr(),
                clen as u64,
                nonce.as_ptr(),
                k.as_ptr(),
            )
        };
        common::eqi(
            &format!("{} open_easy_afternm(clen={}) rc", tag, clen),
            rc,
            rr,
        );
        assert_eq!(rc, -1);
        common::eqb(&format!("{} open_easy_afternm short out", tag), &mc, &mr);
    }

    // ---- tampering at every byte ---------------------------------------
    for &mlen in [0usize, 1, 17, 64].iter() {
        let m = rng.bytes(mlen);
        let mut nonce = [0u8; NB];
        rng.fill(&mut nonce);
        let clen = mlen + MACB;
        let mut ct = vec![0u8; clen];
        let rc = unsafe {
            easy.0(
                ct.as_mut_ptr(),
                m.as_ptr(),
                mlen as u64,
                nonce.as_ptr(),
                pkb.as_ptr(),
                ska.as_ptr(),
            )
        };
        assert_eq!(rc, 0);
        for i in 0..clen {
            for bit in [0x01u8, 0x80u8] {
                let mut bad = ct.clone();
                bad[i] ^= bit;
                let mut mc = vec![CANARY; mlen + 4];
                let mut mr = vec![CANARY; mlen + 4];
                let rc = unsafe {
                    open_easy.0(
                        mc.as_mut_ptr(),
                        bad.as_ptr(),
                        clen as u64,
                        nonce.as_ptr(),
                        pka.as_ptr(),
                        skb.as_ptr(),
                    )
                };
                let rr = unsafe {
                    open_easy.1(
                        mr.as_mut_ptr(),
                        bad.as_ptr(),
                        clen as u64,
                        nonce.as_ptr(),
                        pka.as_ptr(),
                        skb.as_ptr(),
                    )
                };
                common::eqi(&format!("{} tamper[{}] rc", tag, i), rc, rr);
                assert_eq!(rc, -1, "{} tamper at byte {} must fail", tag, i);
                common::eqb(&format!("{} tamper[{}] out", tag, i), &mc, &mr);
                assert_eq!(
                    &mc[..],
                    &vec![CANARY; mlen + 4][..],
                    "{} failed open must not write m",
                    tag
                );
            }
        }
        // detached: tamper each mac byte
        for i in 0..MACB {
            let mut mac = [0u8; MACB];
            mac.copy_from_slice(&ct[..MACB]);
            mac[i] ^= 0x40;
            let mut mc = vec![CANARY; mlen + 4];
            let mut mr = vec![CANARY; mlen + 4];
            let rc = unsafe {
                open_det.0(
                    mc.as_mut_ptr(),
                    ct.as_ptr().add(MACB),
                    mac.as_ptr(),
                    mlen as u64,
                    nonce.as_ptr(),
                    pka.as_ptr(),
                    skb.as_ptr(),
                )
            };
            let rr = unsafe {
                open_det.1(
                    mr.as_mut_ptr(),
                    ct.as_ptr().add(MACB),
                    mac.as_ptr(),
                    mlen as u64,
                    nonce.as_ptr(),
                    pka.as_ptr(),
                    skb.as_ptr(),
                )
            };
            common::eqi(&format!("{} mac tamper[{}] rc", tag, i), rc, rr);
            assert_eq!(rc, -1);
            common::eqb(&format!("{} mac tamper[{}] out", tag, i), &mc, &mr);
        }
    }

    // ---- bad public key -------------------------------------------------
    let m = rng.bytes(32);
    let mut nonce = [0u8; NB];
    rng.fill(&mut nonce);
    for (i, bad) in bad_public_keys().iter().enumerate() {
        let mut cc = vec![CANARY; 32 + MACB];
        let mut cr = vec![CANARY; 32 + MACB];
        let rc = unsafe {
            easy.0(
                cc.as_mut_ptr(),
                m.as_ptr(),
                32,
                nonce.as_ptr(),
                bad.as_ptr(),
                ska.as_ptr(),
            )
        };
        let rr = unsafe {
            easy.1(
                cr.as_mut_ptr(),
                m.as_ptr(),
                32,
                nonce.as_ptr(),
                bad.as_ptr(),
                ska.as_ptr(),
            )
        };
        common::eqi(&format!("{} easy(bad pk #{}) rc", tag, i), rc, rr);
        assert_eq!(rc, -1);
        common::eqb(&format!("{} easy(bad pk #{}) out", tag, i), &cc, &cr);
        assert_eq!(cc, vec![CANARY; 32 + MACB]);

        let mut macbuf = [CANARY; MACB];
        let rc = unsafe {
            det.0(
                cc.as_mut_ptr(),
                macbuf.as_mut_ptr(),
                m.as_ptr(),
                32,
                nonce.as_ptr(),
                bad.as_ptr(),
                ska.as_ptr(),
            )
        };
        let rr = unsafe {
            det.1(
                cr.as_mut_ptr(),
                macbuf.as_mut_ptr(),
                m.as_ptr(),
                32,
                nonce.as_ptr(),
                bad.as_ptr(),
                ska.as_ptr(),
            )
        };
        common::eqi(&format!("{} detached(bad pk #{}) rc", tag, i), rc, rr);
        assert_eq!(rc, -1);

        let rc = unsafe {
            open_easy.0(
                cc.as_mut_ptr(),
                m.as_ptr(),
                32,
                nonce.as_ptr(),
                bad.as_ptr(),
                ska.as_ptr(),
            )
        };
        let rr = unsafe {
            open_easy.1(
                cr.as_mut_ptr(),
                m.as_ptr(),
                32,
                nonce.as_ptr(),
                bad.as_ptr(),
                ska.as_ptr(),
            )
        };
        common::eqi(&format!("{} open_easy(bad pk #{}) rc", tag, i), rc, rr);
        assert_eq!(rc, -1);

        let rc = unsafe {
            open_det.0(
                cc.as_mut_ptr(),
                m.as_ptr(),
                m.as_ptr(),
                16,
                nonce.as_ptr(),
                bad.as_ptr(),
                ska.as_ptr(),
            )
        };
        let rr = unsafe {
            open_det.1(
                cr.as_mut_ptr(),
                m.as_ptr(),
                m.as_ptr(),
                16,
                nonce.as_ptr(),
                bad.as_ptr(),
                ska.as_ptr(),
            )
        };
        common::eqi(&format!("{} open_detached(bad pk #{}) rc", tag, i), rc, rr);
        assert_eq!(rc, -1);
    }

    // ---- randomized cross-library round trips (many cases) -------------
    for _ in 0..24 {
        let mlen = rng.below(600);
        let m = rng.bytes(mlen);
        let mut nonce = [0u8; NB];
        rng.fill(&mut nonce);
        let mut s1 = [0u8; 32];
        let mut s2 = [0u8; 32];
        rng.fill(&mut s1);
        rng.fill(&mut s2);
        let (p1, k1) = seed_kp(&seed_kp_api, &s1, tag);
        let (p2, k2) = seed_kp(&seed_kp_api, &s2, tag);
        let clen = mlen + MACB;
        let mut cbuf = vec![0u8; clen];
        // encrypt with C
        let rc = unsafe {
            easy.0(
                cbuf.as_mut_ptr(),
                m.as_ptr(),
                mlen as u64,
                nonce.as_ptr(),
                p2.as_ptr(),
                k1.as_ptr(),
            )
        };
        assert_eq!(rc, 0);
        // decrypt with Rust
        let mut out = vec![CANARY; mlen];
        let rr = unsafe {
            open_easy.1(
                out.as_mut_ptr(),
                cbuf.as_ptr(),
                clen as u64,
                nonce.as_ptr(),
                p1.as_ptr(),
                k2.as_ptr(),
            )
        };
        assert_eq!(rr, 0, "{} cross C->Rust", tag);
        assert_eq!(&out[..], &m[..], "{} cross C->Rust plaintext", tag);
        // encrypt with Rust, decrypt with C
        let mut cbuf2 = vec![0u8; clen];
        let rr = unsafe {
            easy.1(
                cbuf2.as_mut_ptr(),
                m.as_ptr(),
                mlen as u64,
                nonce.as_ptr(),
                p2.as_ptr(),
                k1.as_ptr(),
            )
        };
        assert_eq!(rr, 0);
        common::eqb(&format!("{} easy determinism", tag), &cbuf, &cbuf2);
        let mut out2 = vec![CANARY; mlen];
        let rc = unsafe {
            open_easy.0(
                out2.as_mut_ptr(),
                cbuf2.as_ptr(),
                clen as u64,
                nonce.as_ptr(),
                p1.as_ptr(),
                k2.as_ptr(),
            )
        };
        assert_eq!(rc, 0, "{} cross Rust->C", tag);
        assert_eq!(&out2[..], &m[..]);
    }
}

#[test]
fn box_easy_generic_salsa() {
    easy_suite("crypto_box_", "box(easy)");
}

#[test]
fn box_easy_xchacha() {
    easy_suite("crypto_box_curve25519xchacha20poly1305_", "xchacha(easy)");
}

// ============================================================== sealing ======

fn seal_suite(prefix: &str, tag: &str) {
    let seal = syms::<Seal>(&format!("{}seal", prefix));
    let seal_open = syms::<SealOpen>(&format!("{}seal_open", prefix));
    let seed_kp_api = syms::<SeedKp>(&format!("{}seed_keypair", prefix));

    let mut rng = common::Rng::new(0x53EA_1000u64 ^ tag.len() as u64);

    let (pk, sk) = seed_kp(&seed_kp_api, &[5u8; 32], tag);
    let (pk2, sk2) = seed_kp(&seed_kp_api, &[6u8; 32], tag);

    for &mlen in [0usize, 1, 15, 16, 17, 63, 64, 65, 1000].iter() {
        let m = rng.bytes(mlen);
        let clen = mlen + SEALB;

        // seal is nondeterministic (ephemeral keypair from randombytes):
        // compare return codes and cross-decrypt.
        let mut sc = vec![CANARY; clen + 4];
        let mut sr = vec![CANARY; clen + 4];
        let rc = unsafe { seal.0(sc.as_mut_ptr(), m.as_ptr(), mlen as u64, pk.as_ptr()) };
        let rr = unsafe { seal.1(sr.as_mut_ptr(), m.as_ptr(), mlen as u64, pk.as_ptr()) };
        common::eqi(&format!("{} seal(mlen={}) rc", tag, mlen), rc, rr);
        assert_eq!(rc, 0, "{} seal(mlen={})", tag, mlen);
        assert_eq!(&sc[clen..], &[CANARY; 4], "{} seal overran", tag);
        assert_eq!(&sr[clen..], &[CANARY; 4], "{} seal overran (Rust)", tag);

        // both sealed blobs must open identically in both libraries
        for (which, blob) in [("C-sealed", &sc), ("Rust-sealed", &sr)] {
            let mut oc = vec![CANARY; mlen + 4];
            let mut or_ = vec![CANARY; mlen + 4];
            let a = unsafe {
                seal_open.0(
                    oc.as_mut_ptr(),
                    blob.as_ptr(),
                    clen as u64,
                    pk.as_ptr(),
                    sk.as_ptr(),
                )
            };
            let b = unsafe {
                seal_open.1(
                    or_.as_mut_ptr(),
                    blob.as_ptr(),
                    clen as u64,
                    pk.as_ptr(),
                    sk.as_ptr(),
                )
            };
            common::eqi(&format!("{} {} seal_open rc", tag, which), a, b);
            assert_eq!(a, 0, "{} {} seal_open", tag, which);
            common::eqb(&format!("{} {} seal_open m", tag, which), &oc, &or_);
            assert_eq!(&oc[..mlen], &m[..], "{} {} plaintext", tag, which);
        }

        // wrong recipient key pair -> -1
        let mut oc = vec![CANARY; mlen + 4];
        let mut or_ = vec![CANARY; mlen + 4];
        let a = unsafe {
            seal_open.0(
                oc.as_mut_ptr(),
                sc.as_ptr(),
                clen as u64,
                pk2.as_ptr(),
                sk2.as_ptr(),
            )
        };
        let b = unsafe {
            seal_open.1(
                or_.as_mut_ptr(),
                sc.as_ptr(),
                clen as u64,
                pk2.as_ptr(),
                sk2.as_ptr(),
            )
        };
        common::eqi(&format!("{} seal_open(wrong key) rc", tag), a, b);
        assert_eq!(a, -1);
        common::eqb(&format!("{} seal_open(wrong key) out", tag), &oc, &or_);
    }

    // clen < SEALBYTES -> -1
    let blob = [0u8; SEALB];
    for clen in 0..SEALB {
        let mut oc = [CANARY; 64];
        let mut or_ = [CANARY; 64];
        let a = unsafe {
            seal_open.0(
                oc.as_mut_ptr(),
                blob.as_ptr(),
                clen as u64,
                pk.as_ptr(),
                sk.as_ptr(),
            )
        };
        let b = unsafe {
            seal_open.1(
                or_.as_mut_ptr(),
                blob.as_ptr(),
                clen as u64,
                pk.as_ptr(),
                sk.as_ptr(),
            )
        };
        common::eqi(&format!("{} seal_open(clen={}) rc", tag, clen), a, b);
        assert_eq!(a, -1, "{} seal_open(clen={}) must be -1", tag, clen);
        common::eqb(&format!("{} seal_open(clen={}) out", tag, clen), &oc, &or_);
        assert_eq!(oc, [CANARY; 64], "{} short seal_open must not write", tag);
    }

    // clen == SEALBYTES (empty message) is valid
    {
        let mut blob = vec![0u8; SEALB];
        let empty = [0u8; 1];
        let rc = unsafe { seal.0(blob.as_mut_ptr(), empty.as_ptr(), 0, pk.as_ptr()) };
        assert_eq!(rc, 0);
        let a = unsafe {
            seal_open.0(
                core::ptr::null_mut(),
                blob.as_ptr(),
                SEALB as u64,
                pk.as_ptr(),
                sk.as_ptr(),
            )
        };
        let b = unsafe {
            seal_open.1(
                core::ptr::null_mut(),
                blob.as_ptr(),
                SEALB as u64,
                pk.as_ptr(),
                sk.as_ptr(),
            )
        };
        common::eqi(&format!("{} seal_open(m=NULL) rc", tag), a, b);
        assert_eq!(a, 0);
    }

    // tamper every byte of a sealed blob
    {
        let mlen = 40usize;
        let m = rng.bytes(mlen);
        let clen = mlen + SEALB;
        let mut blob = vec![0u8; clen];
        let rc = unsafe { seal.0(blob.as_mut_ptr(), m.as_ptr(), mlen as u64, pk.as_ptr()) };
        assert_eq!(rc, 0);
        for i in 0..clen {
            let mut bad = blob.clone();
            bad[i] ^= 0x11;
            let mut oc = vec![CANARY; mlen + 4];
            let mut or_ = vec![CANARY; mlen + 4];
            let a = unsafe {
                seal_open.0(
                    oc.as_mut_ptr(),
                    bad.as_ptr(),
                    clen as u64,
                    pk.as_ptr(),
                    sk.as_ptr(),
                )
            };
            let b = unsafe {
                seal_open.1(
                    or_.as_mut_ptr(),
                    bad.as_ptr(),
                    clen as u64,
                    pk.as_ptr(),
                    sk.as_ptr(),
                )
            };
            common::eqi(&format!("{} seal tamper[{}] rc", tag, i), a, b);
            common::eqb(&format!("{} seal tamper[{}] out", tag, i), &oc, &or_);
            assert_eq!(a, -1, "{} seal tamper at {} must fail", tag, i);
        }
    }

    // seal to a public key that makes scalarmult fail -> -1
    for (i, bad) in bad_public_keys().iter().enumerate() {
        let m = [1u8; 8];
        let mut sc = vec![CANARY; 8 + SEALB];
        let mut sr = vec![CANARY; 8 + SEALB];
        let a = unsafe { seal.0(sc.as_mut_ptr(), m.as_ptr(), 8, bad.as_ptr()) };
        let b = unsafe { seal.1(sr.as_mut_ptr(), m.as_ptr(), 8, bad.as_ptr()) };
        common::eqi(&format!("{} seal(bad pk #{}) rc", tag, i), a, b);
        assert_eq!(a, -1, "{} seal(bad pk #{}) must be -1", tag, i);
        // The ephemeral public key is still memcpy'd into c[0..32] and is
        // random, so only the tail can be compared: it must be untouched.
        assert_eq!(&sc[PKB..], &vec![CANARY; 8 + MACB][..]);
        assert_eq!(&sr[PKB..], &vec![CANARY; 8 + MACB][..]);
    }

    // seal_open with an all-zero embedded ephemeral public key -> -1
    {
        let mlen = 24usize;
        let clen = mlen + SEALB;
        let mut blob = vec![0u8; clen];
        let rc = unsafe { seal.0(blob.as_mut_ptr(), [0u8; 24].as_ptr(), 24, pk.as_ptr()) };
        assert_eq!(rc, 0);
        for bad in bad_public_keys() {
            let mut b2 = blob.clone();
            b2[..PKB].copy_from_slice(&bad);
            let mut oc = vec![CANARY; mlen + 4];
            let mut or_ = vec![CANARY; mlen + 4];
            let a = unsafe {
                seal_open.0(
                    oc.as_mut_ptr(),
                    b2.as_ptr(),
                    clen as u64,
                    pk.as_ptr(),
                    sk.as_ptr(),
                )
            };
            let b = unsafe {
                seal_open.1(
                    or_.as_mut_ptr(),
                    b2.as_ptr(),
                    clen as u64,
                    pk.as_ptr(),
                    sk.as_ptr(),
                )
            };
            common::eqi(&format!("{} seal_open(bad epk) rc", tag), a, b);
            assert_eq!(a, -1);
            common::eqb(&format!("{} seal_open(bad epk) out", tag), &oc, &or_);
        }
    }

    // many randomized cross-library round trips
    for _ in 0..24 {
        let mlen = rng.below(300);
        let m = rng.bytes(mlen);
        let mut s = [0u8; 32];
        rng.fill(&mut s);
        let (rpk, rsk) = seed_kp(&seed_kp_api, &s, tag);
        let clen = mlen + SEALB;
        let mut blob = vec![0u8; clen];
        // C seals
        assert_eq!(
            unsafe { seal.0(blob.as_mut_ptr(), m.as_ptr(), mlen as u64, rpk.as_ptr()) },
            0
        );
        let mut out = vec![CANARY; mlen];
        assert_eq!(
            unsafe {
                seal_open.1(
                    out.as_mut_ptr(),
                    blob.as_ptr(),
                    clen as u64,
                    rpk.as_ptr(),
                    rsk.as_ptr(),
                )
            },
            0,
            "{} cross seal C->Rust",
            tag
        );
        assert_eq!(&out[..], &m[..]);
        // Rust seals
        let mut blob2 = vec![0u8; clen];
        assert_eq!(
            unsafe { seal.1(blob2.as_mut_ptr(), m.as_ptr(), mlen as u64, rpk.as_ptr()) },
            0
        );
        let mut out2 = vec![CANARY; mlen];
        assert_eq!(
            unsafe {
                seal_open.0(
                    out2.as_mut_ptr(),
                    blob2.as_ptr(),
                    clen as u64,
                    rpk.as_ptr(),
                    rsk.as_ptr(),
                )
            },
            0,
            "{} cross seal Rust->C",
            tag
        );
        assert_eq!(&out2[..], &m[..]);
    }
}

#[test]
fn box_seal_generic_salsa() {
    seal_suite("crypto_box_", "box(seal)");
}

#[test]
fn box_seal_xchacha() {
    seal_suite("crypto_box_curve25519xchacha20poly1305_", "xchacha(seal)");
}

// ================================================================== kx =======

#[test]
fn kx_all() {
    let seed_kp_api = syms::<SeedKp>("crypto_kx_seed_keypair");
    let kp_api = syms::<Kp>("crypto_kx_keypair");
    let client = syms::<KxSession>("crypto_kx_client_session_keys");
    let server = syms::<KxSession>("crypto_kx_server_session_keys");

    let mut rng = common::Rng::new(0x4B58_0001);

    // ---- seed_keypair: byte-exact --------------------------------------
    let mut pairs = Vec::new();
    for i in 0..24 {
        let mut seed = [0u8; 32];
        rng.fill(&mut seed);
        if i == 0 {
            seed = [0u8; 32];
        }
        if i == 1 {
            seed = [0xffu8; 32];
        }
        let mut pkc = [CANARY; 32];
        let mut skc = [CANARY; 32];
        let mut pkr = [CANARY; 32];
        let mut skr = [CANARY; 32];
        let rc = unsafe { seed_kp_api.0(pkc.as_mut_ptr(), skc.as_mut_ptr(), seed.as_ptr()) };
        let rr = unsafe { seed_kp_api.1(pkr.as_mut_ptr(), skr.as_mut_ptr(), seed.as_ptr()) };
        common::eqi("kx seed_keypair rc", rc, rr);
        assert_eq!(rc, 0);
        common::eqb("kx seed_keypair pk", &pkc, &pkr);
        common::eqb("kx seed_keypair sk", &skc, &skr);
        pairs.push((pkc, skc));
    }

    // ---- keypair (randombytes): rc + cross-library agreement -----------
    for _ in 0..8 {
        let mut pkc = [CANARY; 32];
        let mut skc = [CANARY; 32];
        let mut pkr = [CANARY; 32];
        let mut skr = [CANARY; 32];
        let rc = unsafe { kp_api.0(pkc.as_mut_ptr(), skc.as_mut_ptr()) };
        let rr = unsafe { kp_api.1(pkr.as_mut_ptr(), skr.as_mut_ptr()) };
        common::eqi("kx keypair rc", rc, rr);
        assert_eq!(rc, 0);
        assert_ne!(skc, [CANARY; 32]);
        // C client (its own keypair) vs Rust server (its own keypair)
        let mut crx = [0u8; 32];
        let mut ctx = [0u8; 32];
        let mut srx = [0u8; 32];
        let mut stx = [0u8; 32];
        let a = unsafe {
            client.0(
                crx.as_mut_ptr(),
                ctx.as_mut_ptr(),
                pkc.as_ptr(),
                skc.as_ptr(),
                pkr.as_ptr(),
            )
        };
        let b = unsafe {
            server.1(
                srx.as_mut_ptr(),
                stx.as_mut_ptr(),
                pkr.as_ptr(),
                skr.as_ptr(),
                pkc.as_ptr(),
            )
        };
        assert_eq!((a, b), (0, 0), "kx cross session keys rc");
        common::eqb("kx cross client.rx == server.tx", &crx, &stx);
        common::eqb("kx cross client.tx == server.rx", &ctx, &srx);
    }

    // ---- session keys, deterministic keys, all rx/tx combinations ------
    for w in 0..23usize {
        let (cpk, csk) = pairs[w];
        let (spk, ssk) = pairs[w + 1];

        // both non-NULL
        let mut crx = [CANARY; 32];
        let mut ctx = [CANARY; 32];
        let mut rrx = [CANARY; 32];
        let mut rtx = [CANARY; 32];
        let a = unsafe {
            client.0(
                crx.as_mut_ptr(),
                ctx.as_mut_ptr(),
                cpk.as_ptr(),
                csk.as_ptr(),
                spk.as_ptr(),
            )
        };
        let b = unsafe {
            client.1(
                rrx.as_mut_ptr(),
                rtx.as_mut_ptr(),
                cpk.as_ptr(),
                csk.as_ptr(),
                spk.as_ptr(),
            )
        };
        common::eqi("kx client rc", a, b);
        assert_eq!(a, 0);
        common::eqb("kx client rx", &crx, &rrx);
        common::eqb("kx client tx", &ctx, &rtx);

        let mut srx = [CANARY; 32];
        let mut stx = [CANARY; 32];
        let mut srx2 = [CANARY; 32];
        let mut stx2 = [CANARY; 32];
        let a = unsafe {
            server.0(
                srx.as_mut_ptr(),
                stx.as_mut_ptr(),
                spk.as_ptr(),
                ssk.as_ptr(),
                cpk.as_ptr(),
            )
        };
        let b = unsafe {
            server.1(
                srx2.as_mut_ptr(),
                stx2.as_mut_ptr(),
                spk.as_ptr(),
                ssk.as_ptr(),
                cpk.as_ptr(),
            )
        };
        common::eqi("kx server rc", a, b);
        assert_eq!(a, 0);
        common::eqb("kx server rx", &srx, &srx2);
        common::eqb("kx server tx", &stx, &stx2);

        // agreement
        assert_eq!(crx, stx, "kx client.rx == server.tx");
        assert_eq!(ctx, srx, "kx client.tx == server.rx");

        // rx == NULL: C sets rx = tx, so tx receives keys[32..64] (the tx
        // half), because the `tx[i] = ...` store happens after `rx[i] = ...`.
        let mut tc = [CANARY; 32];
        let mut tr = [CANARY; 32];
        let a = unsafe {
            client.0(
                core::ptr::null_mut(),
                tc.as_mut_ptr(),
                cpk.as_ptr(),
                csk.as_ptr(),
                spk.as_ptr(),
            )
        };
        let b = unsafe {
            client.1(
                core::ptr::null_mut(),
                tr.as_mut_ptr(),
                cpk.as_ptr(),
                csk.as_ptr(),
                spk.as_ptr(),
            )
        };
        common::eqi("kx client(rx=NULL) rc", a, b);
        assert_eq!(a, 0);
        common::eqb("kx client(rx=NULL) tx", &tc, &tr);
        assert_eq!(tc, ctx, "kx client(rx=NULL): tx store wins");

        // tx == NULL: C sets tx = rx -> same buffer, tx store still last
        let mut rc_ = [CANARY; 32];
        let mut rr_ = [CANARY; 32];
        let a = unsafe {
            client.0(
                rc_.as_mut_ptr(),
                core::ptr::null_mut(),
                cpk.as_ptr(),
                csk.as_ptr(),
                spk.as_ptr(),
            )
        };
        let b = unsafe {
            client.1(
                rr_.as_mut_ptr(),
                core::ptr::null_mut(),
                cpk.as_ptr(),
                csk.as_ptr(),
                spk.as_ptr(),
            )
        };
        common::eqi("kx client(tx=NULL) rc", a, b);
        assert_eq!(a, 0);
        common::eqb("kx client(tx=NULL) rx", &rc_, &rr_);
        assert_eq!(rc_, ctx, "kx client(tx=NULL): tx store wins");

        // server, rx == NULL / tx == NULL (store order is reversed there)
        let mut tc = [CANARY; 32];
        let mut tr = [CANARY; 32];
        let a = unsafe {
            server.0(
                core::ptr::null_mut(),
                tc.as_mut_ptr(),
                spk.as_ptr(),
                ssk.as_ptr(),
                cpk.as_ptr(),
            )
        };
        let b = unsafe {
            server.1(
                core::ptr::null_mut(),
                tr.as_mut_ptr(),
                spk.as_ptr(),
                ssk.as_ptr(),
                cpk.as_ptr(),
            )
        };
        common::eqi("kx server(rx=NULL) rc", a, b);
        assert_eq!(a, 0);
        common::eqb("kx server(rx=NULL) tx", &tc, &tr);
        assert_eq!(tc, srx, "kx server(rx=NULL): rx store wins");

        let mut rc_ = [CANARY; 32];
        let mut rr_ = [CANARY; 32];
        let a = unsafe {
            server.0(
                rc_.as_mut_ptr(),
                core::ptr::null_mut(),
                spk.as_ptr(),
                ssk.as_ptr(),
                cpk.as_ptr(),
            )
        };
        let b = unsafe {
            server.1(
                rr_.as_mut_ptr(),
                core::ptr::null_mut(),
                spk.as_ptr(),
                ssk.as_ptr(),
                cpk.as_ptr(),
            )
        };
        common::eqi("kx server(tx=NULL) rc", a, b);
        assert_eq!(a, 0);
        common::eqb("kx server(tx=NULL) rx", &rc_, &rr_);
        assert_eq!(rc_, srx, "kx server(tx=NULL): rx store wins");

        // rx == tx (same non-NULL pointer)
        let mut ac = [CANARY; 32];
        let mut ar = [CANARY; 32];
        let a = unsafe {
            client.0(
                ac.as_mut_ptr(),
                ac.as_mut_ptr(),
                cpk.as_ptr(),
                csk.as_ptr(),
                spk.as_ptr(),
            )
        };
        let b = unsafe {
            client.1(
                ar.as_mut_ptr(),
                ar.as_mut_ptr(),
                cpk.as_ptr(),
                csk.as_ptr(),
                spk.as_ptr(),
            )
        };
        common::eqi("kx client(rx==tx) rc", a, b);
        common::eqb("kx client(rx==tx)", &ac, &ar);
    }

    // ---- peer public key that makes scalarmult fail -> -1 ---------------
    let (cpk, csk) = pairs[0];
    for (i, bad) in bad_public_keys().iter().enumerate() {
        let mut rxc = [CANARY; 32];
        let mut txc = [CANARY; 32];
        let mut rxr = [CANARY; 32];
        let mut txr = [CANARY; 32];
        let a = unsafe {
            client.0(
                rxc.as_mut_ptr(),
                txc.as_mut_ptr(),
                cpk.as_ptr(),
                csk.as_ptr(),
                bad.as_ptr(),
            )
        };
        let b = unsafe {
            client.1(
                rxr.as_mut_ptr(),
                txr.as_mut_ptr(),
                cpk.as_ptr(),
                csk.as_ptr(),
                bad.as_ptr(),
            )
        };
        common::eqi(&format!("kx client(bad peer #{}) rc", i), a, b);
        assert_eq!(a, -1, "kx client(bad peer #{}) must be -1", i);
        common::eqb("kx client(bad peer) rx", &rxc, &rxr);
        common::eqb("kx client(bad peer) tx", &txc, &txr);
        assert_eq!(rxc, [CANARY; 32], "kx error path must not write rx");
        assert_eq!(txc, [CANARY; 32], "kx error path must not write tx");

        let a = unsafe {
            server.0(
                rxc.as_mut_ptr(),
                txc.as_mut_ptr(),
                cpk.as_ptr(),
                csk.as_ptr(),
                bad.as_ptr(),
            )
        };
        let b = unsafe {
            server.1(
                rxr.as_mut_ptr(),
                txr.as_mut_ptr(),
                cpk.as_ptr(),
                csk.as_ptr(),
                bad.as_ptr(),
            )
        };
        common::eqi(&format!("kx server(bad peer #{}) rc", i), a, b);
        assert_eq!(a, -1);
        common::eqb("kx server(bad peer) rx", &rxc, &rxr);
        common::eqb("kx server(bad peer) tx", &txc, &txr);
    }

    // NOTE: rx == NULL && tx == NULL calls sodium_misuse() -> abort();
    // not testable in-process. Verified by inspection (kx.rs mirrors it).
}

// ============================================================= ML-KEM-768 ====

const MLK_PK: usize = 1184;
const MLK_SK: usize = 2400;
const MLK_CT: usize = 1088;
const MLK_SS: usize = 32;
const MLK_SEED: usize = 64;
const POLYVECBYTES: usize = 1152;

/// Set polyvec coefficient `idx` (0..768) in an ML-KEM public key encoding.
fn set_coeff(pk: &mut [u8], idx: usize, v: u16) {
    let poly = idx / 256;
    let c = idx % 256;
    let p = c / 2;
    let off = poly * 384 + 3 * p;
    if c % 2 == 0 {
        pk[off] = (v & 0xff) as u8;
        pk[off + 1] = (pk[off + 1] & 0xf0) | (((v >> 8) & 0x0f) as u8);
    } else {
        pk[off + 1] = (pk[off + 1] & 0x0f) | (((v & 0x0f) as u8) << 4);
        pk[off + 2] = ((v >> 4) & 0xff) as u8;
    }
}

fn get_coeff(pk: &[u8], idx: usize) -> u16 {
    let poly = idx / 256;
    let c = idx % 256;
    let p = c / 2;
    let off = poly * 384 + 3 * p;
    if c % 2 == 0 {
        (pk[off] as u16) | (((pk[off + 1] as u16) << 8) & 0xf00)
    } else {
        ((pk[off + 1] as u16) >> 4) | (((pk[off + 2] as u16) << 4) & 0xff0)
    }
}

/// Runs the whole ML-KEM-768 matrix against a given symbol quadruple, so it can
/// be applied both to the public `crypto_kem_mlkem768_*` wrappers and to the
/// internal `_sodium_mlkem768_ref_*` entry points.
fn mlkem_suite(seed_kp_name: &str, kp_name: &str, enc_name: &str, encd_name: &str, dec_name: &str, tag: &str) {
    let seed_kp_api = syms::<SeedKp>(seed_kp_name);
    let kp_api = syms::<Kp>(kp_name);
    let enc = syms::<KemEnc>(enc_name);
    let encd = syms::<KemEncDet>(encd_name);
    let dec = syms::<KemDec>(dec_name);

    let mut rng = common::Rng::new(0x4D4C_4B45 ^ tag.len() as u64);

    // ---- seed_keypair: byte-exact for many seeds ------------------------
    let mut kps: Vec<(Vec<u8>, Vec<u8>)> = Vec::new();
    for i in 0..16 {
        let mut seed = vec![0u8; MLK_SEED];
        rng.fill(&mut seed);
        if i == 0 {
            seed = vec![0u8; MLK_SEED];
        }
        if i == 1 {
            seed = vec![0xffu8; MLK_SEED];
        }
        let mut pkc = vec![CANARY; MLK_PK + 4];
        let mut skc = vec![CANARY; MLK_SK + 4];
        let mut pkr = vec![CANARY; MLK_PK + 4];
        let mut skr = vec![CANARY; MLK_SK + 4];
        let rc = unsafe { seed_kp_api.0(pkc.as_mut_ptr(), skc.as_mut_ptr(), seed.as_ptr()) };
        let rr = unsafe { seed_kp_api.1(pkr.as_mut_ptr(), skr.as_mut_ptr(), seed.as_ptr()) };
        common::eqi(&format!("{} seed_keypair rc", tag), rc, rr);
        assert_eq!(rc, 0);
        common::eqb(&format!("{} seed_keypair pk", tag), &pkc, &pkr);
        common::eqb(&format!("{} seed_keypair sk", tag), &skc, &skr);
        assert_eq!(&pkc[MLK_PK..], &[CANARY; 4], "{} pk overrun", tag);
        assert_eq!(&skc[MLK_SK..], &[CANARY; 4], "{} sk overrun", tag);
        // structural checks derived from the C: sk = skpv || pk || H(pk) || z
        assert_eq!(&skc[POLYVECBYTES..POLYVECBYTES + MLK_PK], &pkc[..MLK_PK]);
        assert_eq!(
            &skc[POLYVECBYTES + MLK_PK + 32..POLYVECBYTES + MLK_PK + 64],
            &seed[32..64]
        );
        // every polyvec coefficient of pk must be canonical (< q)
        for j in 0..768 {
            assert!(get_coeff(&pkc, j) < 3329, "{} pk coeff {} >= q", tag, j);
        }
        if i < 6 {
            kps.push((pkc[..MLK_PK].to_vec(), skc[..MLK_SK].to_vec()));
        }
    }

    // ---- enc_deterministic + dec: byte-exact ---------------------------
    for (n, (pk, sk)) in kps.iter().enumerate() {
        for _ in 0..4 {
            let mut seed = [0u8; 32];
            rng.fill(&mut seed);
            let mut ctc = vec![CANARY; MLK_CT + 4];
            let mut ctr = vec![CANARY; MLK_CT + 4];
            let mut ssc = vec![CANARY; MLK_SS + 4];
            let mut ssr = vec![CANARY; MLK_SS + 4];
            let rc = unsafe {
                encd.0(
                    ctc.as_mut_ptr(),
                    ssc.as_mut_ptr(),
                    pk.as_ptr(),
                    seed.as_ptr(),
                )
            };
            let rr = unsafe {
                encd.1(
                    ctr.as_mut_ptr(),
                    ssr.as_mut_ptr(),
                    pk.as_ptr(),
                    seed.as_ptr(),
                )
            };
            common::eqi(&format!("{} enc_det rc", tag), rc, rr);
            assert_eq!(rc, 0, "{} enc_det on a valid pk", tag);
            common::eqb(&format!("{} enc_det ct", tag), &ctc, &ctr);
            common::eqb(&format!("{} enc_det ss", tag), &ssc, &ssr);
            assert_eq!(&ctc[MLK_CT..], &[CANARY; 4]);
            assert_eq!(&ssc[MLK_SS..], &[CANARY; 4]);

            // dec must recover the same shared secret in both libraries
            let mut dc = vec![CANARY; MLK_SS + 4];
            let mut dr = vec![CANARY; MLK_SS + 4];
            let a = unsafe { dec.0(dc.as_mut_ptr(), ctc.as_ptr(), sk.as_ptr()) };
            let b = unsafe { dec.1(dr.as_mut_ptr(), ctc.as_ptr(), sk.as_ptr()) };
            common::eqi(&format!("{} dec rc", tag), a, b);
            assert_eq!(a, 0, "{} dec always returns 0", tag);
            common::eqb(&format!("{} dec ss", tag), &dc, &dr);
            assert_eq!(
                &dc[..MLK_SS],
                &ssc[..MLK_SS],
                "{} kem#{} round trip",
                tag,
                n
            );
            assert_eq!(&dc[MLK_SS..], &[CANARY; 4]);
        }
    }

    // ---- enc (randombytes): rc + cross-library round trip --------------
    let (pk0, sk0) = &kps[0];
    for _ in 0..4 {
        let mut ctc = vec![0u8; MLK_CT];
        let mut ssc = vec![0u8; MLK_SS];
        let mut ctr = vec![0u8; MLK_CT];
        let mut ssr = vec![0u8; MLK_SS];
        let rc = unsafe { enc.0(ctc.as_mut_ptr(), ssc.as_mut_ptr(), pk0.as_ptr()) };
        let rr = unsafe { enc.1(ctr.as_mut_ptr(), ssr.as_mut_ptr(), pk0.as_ptr()) };
        common::eqi(&format!("{} enc rc", tag), rc, rr);
        assert_eq!(rc, 0);
        // C encapsulates -> Rust decapsulates
        let mut out = vec![0u8; MLK_SS];
        assert_eq!(
            unsafe { dec.1(out.as_mut_ptr(), ctc.as_ptr(), sk0.as_ptr()) },
            0
        );
        assert_eq!(out, ssc, "{} cross enc C -> dec Rust", tag);
        // Rust encapsulates -> C decapsulates
        let mut out2 = vec![0u8; MLK_SS];
        assert_eq!(
            unsafe { dec.0(out2.as_mut_ptr(), ctr.as_ptr(), sk0.as_ptr()) },
            0
        );
        assert_eq!(out2, ssr, "{} cross enc Rust -> dec C", tag);
    }

    // ---- keypair (randombytes) -----------------------------------------
    for _ in 0..3 {
        let mut pkc = vec![CANARY; MLK_PK];
        let mut skc = vec![CANARY; MLK_SK];
        let mut pkr = vec![CANARY; MLK_PK];
        let mut skr = vec![CANARY; MLK_SK];
        let rc = unsafe { kp_api.0(pkc.as_mut_ptr(), skc.as_mut_ptr()) };
        let rr = unsafe { kp_api.1(pkr.as_mut_ptr(), skr.as_mut_ptr()) };
        common::eqi(&format!("{} keypair rc", tag), rc, rr);
        assert_eq!(rc, 0);
        assert_ne!(pkc, vec![CANARY; MLK_PK]);
        // Rust encapsulates against the C-generated pk; C decapsulates
        let mut ct = vec![0u8; MLK_CT];
        let mut ss1 = vec![0u8; MLK_SS];
        assert_eq!(
            unsafe { enc.1(ct.as_mut_ptr(), ss1.as_mut_ptr(), pkc.as_ptr()) },
            0
        );
        let mut ss2 = vec![0u8; MLK_SS];
        assert_eq!(
            unsafe { dec.0(ss2.as_mut_ptr(), ct.as_ptr(), skc.as_ptr()) },
            0
        );
        assert_eq!(ss1, ss2, "{} cross keypair round trip", tag);
    }

    // ---- non-canonical public keys -> -1 -------------------------------
    let (pk_ok, _) = &kps[1];
    // exact boundary: q-1 accepted, q rejected, and a few beyond
    for idx in [0usize, 1, 2, 255, 256, 511, 512, 766, 767] {
        for (v, want) in [(3328u16, 0i32), (3329, -1), (3330, -1), (4095, -1)] {
            let mut pk = pk_ok.clone();
            set_coeff(&mut pk, idx, v);
            assert_eq!(get_coeff(&pk, idx), v, "set_coeff/get_coeff disagree");
            let mut ctc = vec![CANARY; MLK_CT];
            let mut ctr = vec![CANARY; MLK_CT];
            let mut ssc = vec![CANARY; MLK_SS];
            let mut ssr = vec![CANARY; MLK_SS];
            let seed = [0x42u8; 32];
            let rc = unsafe {
                encd.0(
                    ctc.as_mut_ptr(),
                    ssc.as_mut_ptr(),
                    pk.as_ptr(),
                    seed.as_ptr(),
                )
            };
            let rr = unsafe {
                encd.1(
                    ctr.as_mut_ptr(),
                    ssr.as_mut_ptr(),
                    pk.as_ptr(),
                    seed.as_ptr(),
                )
            };
            common::eqi(
                &format!("{} enc_det(coeff[{}]={}) rc", tag, idx, v),
                rc,
                rr,
            );
            assert_eq!(
                rc, want,
                "{} enc_det(coeff[{}]={}) expected {}",
                tag, idx, v, want
            );
            common::eqb(&format!("{} enc_det(coeff) ct", tag), &ctc, &ctr);
            common::eqb(&format!("{} enc_det(coeff) ss", tag), &ssc, &ssr);
            if want == -1 {
                assert_eq!(ctc, vec![CANARY; MLK_CT], "{} rejection must not write ct", tag);
                assert_eq!(ssc, vec![CANARY; MLK_SS], "{} rejection must not write ss", tag);
            }
        }
    }

    // fully random public keys: virtually always non-canonical
    for _ in 0..8 {
        let pk = rng.bytes(MLK_PK);
        let seed = rng.bytes(32);
        let mut ctc = vec![CANARY; MLK_CT];
        let mut ctr = vec![CANARY; MLK_CT];
        let mut ssc = vec![CANARY; MLK_SS];
        let mut ssr = vec![CANARY; MLK_SS];
        let rc = unsafe {
            encd.0(
                ctc.as_mut_ptr(),
                ssc.as_mut_ptr(),
                pk.as_ptr(),
                seed.as_ptr(),
            )
        };
        let rr = unsafe {
            encd.1(
                ctr.as_mut_ptr(),
                ssr.as_mut_ptr(),
                pk.as_ptr(),
                seed.as_ptr(),
            )
        };
        common::eqi(&format!("{} enc_det(random pk) rc", tag), rc, rr);
        common::eqb(&format!("{} enc_det(random pk) ct", tag), &ctc, &ctr);
        common::eqb(&format!("{} enc_det(random pk) ss", tag), &ssc, &ssr);
    }
    // sweep: flip a bit in EVERY byte of a valid public key. Bytes inside the
    // polyvec encoding may turn a coefficient non-canonical (-> -1); bytes in
    // the trailing 32-byte matrix seed never do (-> 0). Both the return code
    // and the outputs must agree.
    {
        let (pk_base, _) = &kps[4];
        let seed = [0x13u8; 32];
        let mut n_reject = 0usize;
        let mut n_accept = 0usize;
        for i in 0..MLK_PK {
            let mut pk = pk_base.clone();
            pk[i] ^= 0x80;
            let mut ctc = vec![CANARY; MLK_CT];
            let mut ctr = vec![CANARY; MLK_CT];
            let mut ssc = vec![CANARY; MLK_SS];
            let mut ssr = vec![CANARY; MLK_SS];
            let rc = unsafe {
                encd.0(
                    ctc.as_mut_ptr(),
                    ssc.as_mut_ptr(),
                    pk.as_ptr(),
                    seed.as_ptr(),
                )
            };
            let rr = unsafe {
                encd.1(
                    ctr.as_mut_ptr(),
                    ssr.as_mut_ptr(),
                    pk.as_ptr(),
                    seed.as_ptr(),
                )
            };
            common::eqi(&format!("{} enc_det(pk[{}] flipped) rc", tag, i), rc, rr);
            common::eqb(&format!("{} enc_det(pk[{}] flipped) ct", tag, i), &ctc, &ctr);
            common::eqb(&format!("{} enc_det(pk[{}] flipped) ss", tag, i), &ssc, &ssr);
            if rc == 0 {
                n_accept += 1;
            } else {
                assert_eq!(rc, -1, "{} unexpected return {}", tag, rc);
                n_reject += 1;
            }
            if i >= POLYVECBYTES {
                assert_eq!(rc, 0, "{} seed byte {} must stay canonical", tag, i);
            }
        }
        assert!(n_reject > 0 && n_accept > 0, "{} pk sweep degenerate", tag);
    }
    // all-zero public key is canonical (all coefficients 0) -> accepted
    {
        let pk = vec![0u8; MLK_PK];
        let seed = [1u8; 32];
        let mut ctc = vec![CANARY; MLK_CT];
        let mut ctr = vec![CANARY; MLK_CT];
        let mut ssc = vec![CANARY; MLK_SS];
        let mut ssr = vec![CANARY; MLK_SS];
        let rc = unsafe {
            encd.0(
                ctc.as_mut_ptr(),
                ssc.as_mut_ptr(),
                pk.as_ptr(),
                seed.as_ptr(),
            )
        };
        let rr = unsafe {
            encd.1(
                ctr.as_mut_ptr(),
                ssr.as_mut_ptr(),
                pk.as_ptr(),
                seed.as_ptr(),
            )
        };
        common::eqi(&format!("{} enc_det(zero pk) rc", tag), rc, rr);
        assert_eq!(rc, 0, "{} an all-zero pk is canonical", tag);
        common::eqb(&format!("{} enc_det(zero pk) ct", tag), &ctc, &ctr);
        common::eqb(&format!("{} enc_det(zero pk) ss", tag), &ssc, &ssr);
    }

    // ---- implicit rejection: tampered ciphertexts -----------------------
    // ML-KEM never returns -1 from dec; it derives a pseudorandom shared
    // secret instead. C and Rust must produce the SAME bytes.
    {
        let (pk, sk) = &kps[2];
        let seed = [0x77u8; 32];
        let mut ct = vec![0u8; MLK_CT];
        let mut ss = vec![0u8; MLK_SS];
        assert_eq!(
            unsafe { encd.0(ct.as_mut_ptr(), ss.as_mut_ptr(), pk.as_ptr(), seed.as_ptr()) },
            0
        );
        // every single byte of the ciphertext
        for i in 0..MLK_CT {
            let mut bad = ct.clone();
            bad[i] ^= 0x55;
            let mut dc = vec![CANARY; MLK_SS + 4];
            let mut dr = vec![CANARY; MLK_SS + 4];
            let a = unsafe { dec.0(dc.as_mut_ptr(), bad.as_ptr(), sk.as_ptr()) };
            let b = unsafe { dec.1(dr.as_mut_ptr(), bad.as_ptr(), sk.as_ptr()) };
            common::eqi(&format!("{} dec(tamper[{}]) rc", tag, i), a, b);
            assert_eq!(a, 0, "{} dec must still return 0", tag);
            common::eqb(&format!("{} dec(tamper[{}]) ss", tag, i), &dc, &dr);
            assert_ne!(
                &dc[..MLK_SS],
                &ss[..MLK_SS],
                "{} tampered ct should give a different ss",
                tag
            );
        }
        // wholly random ciphertexts
        for _ in 0..8 {
            let bad = rng.bytes(MLK_CT);
            let mut dc = vec![CANARY; MLK_SS];
            let mut dr = vec![CANARY; MLK_SS];
            let a = unsafe { dec.0(dc.as_mut_ptr(), bad.as_ptr(), sk.as_ptr()) };
            let b = unsafe { dec.1(dr.as_mut_ptr(), bad.as_ptr(), sk.as_ptr()) };
            common::eqi(&format!("{} dec(random ct) rc", tag), a, b);
            common::eqb(&format!("{} dec(random ct) ss", tag), &dc, &dr);
        }
        // all-zero and all-ones ciphertexts
        for pat in [0x00u8, 0xff] {
            let bad = vec![pat; MLK_CT];
            let mut dc = vec![CANARY; MLK_SS];
            let mut dr = vec![CANARY; MLK_SS];
            let a = unsafe { dec.0(dc.as_mut_ptr(), bad.as_ptr(), sk.as_ptr()) };
            let b = unsafe { dec.1(dr.as_mut_ptr(), bad.as_ptr(), sk.as_ptr()) };
            common::eqi(&format!("{} dec(ct={:#x}) rc", tag, pat), a, b);
            common::eqb(&format!("{} dec(ct={:#x}) ss", tag, pat), &dc, &dr);
        }
    }

    // ---- dec with a wholly random secret key ---------------------------
    // Exercises indcpa_dec / indcpa_enc with arbitrary (possibly
    // non-canonical) coefficients; still fully deterministic.
    for _ in 0..6 {
        let sk = rng.bytes(MLK_SK);
        let ct = rng.bytes(MLK_CT);
        let mut dc = vec![CANARY; MLK_SS];
        let mut dr = vec![CANARY; MLK_SS];
        let a = unsafe { dec.0(dc.as_mut_ptr(), ct.as_ptr(), sk.as_ptr()) };
        let b = unsafe { dec.1(dr.as_mut_ptr(), ct.as_ptr(), sk.as_ptr()) };
        common::eqi(&format!("{} dec(random sk) rc", tag), a, b);
        common::eqb(&format!("{} dec(random sk) ss", tag), &dc, &dr);
    }
    // extreme secret keys (all zero / all ones)
    for pat in [0x00u8, 0xff] {
        let sk = vec![pat; MLK_SK];
        for ctpat in [0x00u8, 0xff, 0x5a] {
            let ct = vec![ctpat; MLK_CT];
            let mut dc = vec![CANARY; MLK_SS];
            let mut dr = vec![CANARY; MLK_SS];
            let a = unsafe { dec.0(dc.as_mut_ptr(), ct.as_ptr(), sk.as_ptr()) };
            let b = unsafe { dec.1(dr.as_mut_ptr(), ct.as_ptr(), sk.as_ptr()) };
            common::eqi(&format!("{} dec(sk={:#x}) rc", tag, pat), a, b);
            common::eqb(&format!("{} dec(sk={:#x},ct={:#x}) ss", tag, pat, ctpat), &dc, &dr);
        }
    }

    // ---- enc_deterministic seed edge cases ------------------------------
    for seed in [[0u8; 32], [0xffu8; 32]] {
        let (pk, sk) = &kps[3];
        let mut ctc = vec![CANARY; MLK_CT];
        let mut ctr = vec![CANARY; MLK_CT];
        let mut ssc = vec![CANARY; MLK_SS];
        let mut ssr = vec![CANARY; MLK_SS];
        let rc = unsafe {
            encd.0(
                ctc.as_mut_ptr(),
                ssc.as_mut_ptr(),
                pk.as_ptr(),
                seed.as_ptr(),
            )
        };
        let rr = unsafe {
            encd.1(
                ctr.as_mut_ptr(),
                ssr.as_mut_ptr(),
                pk.as_ptr(),
                seed.as_ptr(),
            )
        };
        common::eqi(&format!("{} enc_det(edge seed) rc", tag), rc, rr);
        common::eqb(&format!("{} enc_det(edge seed) ct", tag), &ctc, &ctr);
        common::eqb(&format!("{} enc_det(edge seed) ss", tag), &ssc, &ssr);
        let mut dc = vec![CANARY; MLK_SS];
        let mut dr = vec![CANARY; MLK_SS];
        let a = unsafe { dec.0(dc.as_mut_ptr(), ctc.as_ptr(), sk.as_ptr()) };
        let b = unsafe { dec.1(dr.as_mut_ptr(), ctc.as_ptr(), sk.as_ptr()) };
        common::eqi(&format!("{} dec(edge) rc", tag), a, b);
        common::eqb(&format!("{} dec(edge) ss", tag), &dc, &dr);
        assert_eq!(dc, ssc, "{} edge-seed round trip", tag);
    }
}

#[test]
fn kem_mlkem768_public_api() {
    mlkem_suite(
        "crypto_kem_mlkem768_seed_keypair",
        "crypto_kem_mlkem768_keypair",
        "crypto_kem_mlkem768_enc",
        "crypto_kem_mlkem768_enc_deterministic",
        "crypto_kem_mlkem768_dec",
        "mlkem768",
    );
}

#[test]
fn kem_mlkem768_ref_internals() {
    mlkem_suite(
        "_sodium_mlkem768_ref_seed_keypair",
        "_sodium_mlkem768_ref_keypair",
        "_sodium_mlkem768_ref_enc",
        "_sodium_mlkem768_ref_enc_deterministic",
        "_sodium_mlkem768_ref_dec",
        "mlkem768_ref",
    );
}

/// The public wrapper must be a pure pass-through to the internal `ref` entry
/// point, in both libraries.
#[test]
fn kem_mlkem768_wrapper_matches_ref() {
    let pub_skp = syms::<SeedKp>("crypto_kem_mlkem768_seed_keypair");
    let ref_skp = syms::<SeedKp>("_sodium_mlkem768_ref_seed_keypair");
    let pub_encd = syms::<KemEncDet>("crypto_kem_mlkem768_enc_deterministic");
    let ref_encd = syms::<KemEncDet>("_sodium_mlkem768_ref_enc_deterministic");
    let pub_dec = syms::<KemDec>("crypto_kem_mlkem768_dec");
    let ref_dec = syms::<KemDec>("_sodium_mlkem768_ref_dec");

    let mut rng = common::Rng::new(0xDEAD_1234);
    for _ in 0..4 {
        let seed = rng.bytes(MLK_SEED);
        let mut pk1 = vec![0u8; MLK_PK];
        let mut sk1 = vec![0u8; MLK_SK];
        let mut pk2 = vec![0u8; MLK_PK];
        let mut sk2 = vec![0u8; MLK_SK];
        for (lib, f1, f2) in [(0, pub_skp.0, ref_skp.0), (1, pub_skp.1, ref_skp.1)] {
            assert_eq!(
                unsafe { f1(pk1.as_mut_ptr(), sk1.as_mut_ptr(), seed.as_ptr()) },
                0
            );
            assert_eq!(
                unsafe { f2(pk2.as_mut_ptr(), sk2.as_mut_ptr(), seed.as_ptr()) },
                0
            );
            common::eqb(&format!("lib{} wrapper pk == ref pk", lib), &pk1, &pk2);
            common::eqb(&format!("lib{} wrapper sk == ref sk", lib), &sk1, &sk2);
        }
        let eseed = rng.bytes(32);
        let mut ct1 = vec![0u8; MLK_CT];
        let mut ss1 = vec![0u8; MLK_SS];
        let mut ct2 = vec![0u8; MLK_CT];
        let mut ss2 = vec![0u8; MLK_SS];
        for (lib, f1, f2) in [(0, pub_encd.0, ref_encd.0), (1, pub_encd.1, ref_encd.1)] {
            assert_eq!(
                unsafe {
                    f1(
                        ct1.as_mut_ptr(),
                        ss1.as_mut_ptr(),
                        pk1.as_ptr(),
                        eseed.as_ptr(),
                    )
                },
                0
            );
            assert_eq!(
                unsafe {
                    f2(
                        ct2.as_mut_ptr(),
                        ss2.as_mut_ptr(),
                        pk1.as_ptr(),
                        eseed.as_ptr(),
                    )
                },
                0
            );
            common::eqb(&format!("lib{} wrapper ct == ref ct", lib), &ct1, &ct2);
            common::eqb(&format!("lib{} wrapper ss == ref ss", lib), &ss1, &ss2);
        }
        let mut d1 = vec![0u8; MLK_SS];
        let mut d2 = vec![0u8; MLK_SS];
        for (lib, f1, f2) in [(0, pub_dec.0, ref_dec.0), (1, pub_dec.1, ref_dec.1)] {
            assert_eq!(
                unsafe { f1(d1.as_mut_ptr(), ct1.as_ptr(), sk1.as_ptr()) },
                0
            );
            assert_eq!(
                unsafe { f2(d2.as_mut_ptr(), ct1.as_ptr(), sk1.as_ptr()) },
                0
            );
            common::eqb(&format!("lib{} wrapper dec == ref dec", lib), &d1, &d2);
        }
    }
}

// ================================================================= X-Wing ====

const XW_PK: usize = 1216;
const XW_SK: usize = 32;
const XW_CT: usize = 1120;
const XW_SS: usize = 32;
const XW_SEED: usize = 32;

fn xwing_suite(prefix: &str, has_enc_det: bool, tag: &str) {
    let seed_kp_api = syms::<SeedKp>(&format!("{}seed_keypair", prefix));
    let kp_api = syms::<Kp>(&format!("{}keypair", prefix));
    let enc = syms::<KemEnc>(&format!("{}enc", prefix));
    let dec = syms::<KemDec>(&format!("{}dec", prefix));
    let encd = if has_enc_det {
        Some(syms::<KemEncDet>(&format!("{}enc_deterministic", prefix)))
    } else {
        None
    };

    let mut rng = common::Rng::new(0x5857_494E ^ tag.len() as u64);

    // ---- seed_keypair: byte-exact --------------------------------------
    let mut kps: Vec<(Vec<u8>, Vec<u8>)> = Vec::new();
    for i in 0..16 {
        let mut seed = vec![0u8; XW_SEED];
        rng.fill(&mut seed);
        if i == 0 {
            seed = vec![0u8; XW_SEED];
        }
        if i == 1 {
            seed = vec![0xffu8; XW_SEED];
        }
        let mut pkc = vec![CANARY; XW_PK + 4];
        let mut skc = vec![CANARY; XW_SK + 4];
        let mut pkr = vec![CANARY; XW_PK + 4];
        let mut skr = vec![CANARY; XW_SK + 4];
        let rc = unsafe { seed_kp_api.0(pkc.as_mut_ptr(), skc.as_mut_ptr(), seed.as_ptr()) };
        let rr = unsafe { seed_kp_api.1(pkr.as_mut_ptr(), skr.as_mut_ptr(), seed.as_ptr()) };
        common::eqi(&format!("{} seed_keypair rc", tag), rc, rr);
        assert_eq!(rc, 0);
        common::eqb(&format!("{} seed_keypair pk", tag), &pkc, &pkr);
        common::eqb(&format!("{} seed_keypair sk", tag), &skc, &skr);
        assert_eq!(&pkc[XW_PK..], &[CANARY; 4]);
        assert_eq!(&skc[XW_SK..], &[CANARY; 4]);
        // sk is literally the seed
        assert_eq!(&skc[..XW_SK], &seed[..]);
        if i < 6 {
            kps.push((pkc[..XW_PK].to_vec(), skc[..XW_SK].to_vec()));
        }
    }

    // ---- enc_deterministic + dec: byte-exact ---------------------------
    if let Some(encd) = encd {
        for (pk, sk) in kps.iter() {
            for _ in 0..4 {
                let seed = rng.bytes(64);
                let mut ctc = vec![CANARY; XW_CT + 4];
                let mut ctr = vec![CANARY; XW_CT + 4];
                let mut ssc = vec![CANARY; XW_SS + 4];
                let mut ssr = vec![CANARY; XW_SS + 4];
                let rc = unsafe {
                    encd.0(
                        ctc.as_mut_ptr(),
                        ssc.as_mut_ptr(),
                        pk.as_ptr(),
                        seed.as_ptr(),
                    )
                };
                let rr = unsafe {
                    encd.1(
                        ctr.as_mut_ptr(),
                        ssr.as_mut_ptr(),
                        pk.as_ptr(),
                        seed.as_ptr(),
                    )
                };
                common::eqi(&format!("{} enc_det rc", tag), rc, rr);
                assert_eq!(rc, 0);
                common::eqb(&format!("{} enc_det ct", tag), &ctc, &ctr);
                common::eqb(&format!("{} enc_det ss", tag), &ssc, &ssr);
                assert_eq!(&ctc[XW_CT..], &[CANARY; 4]);
                assert_eq!(&ssc[XW_SS..], &[CANARY; 4]);

                let mut dc = vec![CANARY; XW_SS + 4];
                let mut dr = vec![CANARY; XW_SS + 4];
                let a = unsafe { dec.0(dc.as_mut_ptr(), ctc.as_ptr(), sk.as_ptr()) };
                let b = unsafe { dec.1(dr.as_mut_ptr(), ctc.as_ptr(), sk.as_ptr()) };
                common::eqi(&format!("{} dec rc", tag), a, b);
                assert_eq!(a, 0);
                common::eqb(&format!("{} dec ss", tag), &dc, &dr);
                assert_eq!(&dc[..XW_SS], &ssc[..XW_SS], "{} round trip", tag);
            }
        }

        // seed edge cases
        let (pk, sk) = &kps[0];
        for seed in [vec![0u8; 64], vec![0xffu8; 64]] {
            let mut ctc = vec![CANARY; XW_CT];
            let mut ctr = vec![CANARY; XW_CT];
            let mut ssc = vec![CANARY; XW_SS];
            let mut ssr = vec![CANARY; XW_SS];
            let rc = unsafe {
                encd.0(
                    ctc.as_mut_ptr(),
                    ssc.as_mut_ptr(),
                    pk.as_ptr(),
                    seed.as_ptr(),
                )
            };
            let rr = unsafe {
                encd.1(
                    ctr.as_mut_ptr(),
                    ssr.as_mut_ptr(),
                    pk.as_ptr(),
                    seed.as_ptr(),
                )
            };
            common::eqi(&format!("{} enc_det(edge seed) rc", tag), rc, rr);
            common::eqb(&format!("{} enc_det(edge seed) ct", tag), &ctc, &ctr);
            common::eqb(&format!("{} enc_det(edge seed) ss", tag), &ssc, &ssr);
            let mut dc = vec![CANARY; XW_SS];
            let mut dr = vec![CANARY; XW_SS];
            let a = unsafe { dec.0(dc.as_mut_ptr(), ctc.as_ptr(), sk.as_ptr()) };
            let b = unsafe { dec.1(dr.as_mut_ptr(), ctc.as_ptr(), sk.as_ptr()) };
            common::eqi(&format!("{} dec(edge) rc", tag), a, b);
            common::eqb(&format!("{} dec(edge) ss", tag), &dc, &dr);
        }

        // non-canonical ML-KEM part of the public key -> -1
        for idx in [0usize, 383, 767] {
            let mut pk = kps[1].0.clone();
            set_coeff(&mut pk, idx, 3329);
            let seed = vec![3u8; 64];
            let mut ctc = vec![CANARY; XW_CT];
            let mut ctr = vec![CANARY; XW_CT];
            let mut ssc = vec![CANARY; XW_SS];
            let mut ssr = vec![CANARY; XW_SS];
            let rc = unsafe {
                encd.0(
                    ctc.as_mut_ptr(),
                    ssc.as_mut_ptr(),
                    pk.as_ptr(),
                    seed.as_ptr(),
                )
            };
            let rr = unsafe {
                encd.1(
                    ctr.as_mut_ptr(),
                    ssr.as_mut_ptr(),
                    pk.as_ptr(),
                    seed.as_ptr(),
                )
            };
            common::eqi(&format!("{} enc_det(bad mlkem pk) rc", tag), rc, rr);
            assert_eq!(rc, -1, "{} non-canonical mlkem pk must be -1", tag);
            common::eqb(&format!("{} enc_det(bad mlkem pk) ct", tag), &ctc, &ctr);
            common::eqb(&format!("{} enc_det(bad mlkem pk) ss", tag), &ssc, &ssr);
            assert_eq!(ctc, vec![CANARY; XW_CT]);
            assert_eq!(ssc, vec![CANARY; XW_SS]);
        }

        // X25519 part of the public key with an all-zero/small-order value:
        // crypto_scalarmult_curve25519 fails -> -1
        for (i, bad) in bad_public_keys().iter().enumerate() {
            let mut pk = kps[2].0.clone();
            pk[MLK_PK..].copy_from_slice(bad);
            let seed = vec![4u8; 64];
            let mut ctc = vec![CANARY; XW_CT];
            let mut ctr = vec![CANARY; XW_CT];
            let mut ssc = vec![CANARY; XW_SS];
            let mut ssr = vec![CANARY; XW_SS];
            let rc = unsafe {
                encd.0(
                    ctc.as_mut_ptr(),
                    ssc.as_mut_ptr(),
                    pk.as_ptr(),
                    seed.as_ptr(),
                )
            };
            let rr = unsafe {
                encd.1(
                    ctr.as_mut_ptr(),
                    ssr.as_mut_ptr(),
                    pk.as_ptr(),
                    seed.as_ptr(),
                )
            };
            common::eqi(&format!("{} enc_det(bad x25519 pk #{}) rc", tag, i), rc, rr);
            assert_eq!(rc, -1, "{} bad x25519 pk #{} must be -1", tag, i);
            common::eqb(&format!("{} enc_det(bad x25519) ct", tag), &ctc, &ctr);
            common::eqb(&format!("{} enc_det(bad x25519) ss", tag), &ssc, &ssr);
            assert_eq!(ctc, vec![CANARY; XW_CT], "{} must not write ct", tag);
            assert_eq!(ssc, vec![CANARY; XW_SS], "{} must not write ss", tag);
        }

    }

    // ---- enc against a public key whose X25519 half is 0/small order ---
    // (crypto_kem_xwing_enc / crypto_kem_enc propagate enc_deterministic's -1)
    for (i, bad) in bad_public_keys().iter().enumerate() {
        let mut pk = kps[2].0.clone();
        pk[MLK_PK..].copy_from_slice(bad);
        let mut ctc = vec![CANARY; XW_CT];
        let mut ctr = vec![CANARY; XW_CT];
        let mut ssc = vec![CANARY; XW_SS];
        let mut ssr = vec![CANARY; XW_SS];
        let rc = unsafe { enc.0(ctc.as_mut_ptr(), ssc.as_mut_ptr(), pk.as_ptr()) };
        let rr = unsafe { enc.1(ctr.as_mut_ptr(), ssr.as_mut_ptr(), pk.as_ptr()) };
        common::eqi(&format!("{} enc(bad x25519 pk #{}) rc", tag, i), rc, rr);
        assert_eq!(rc, -1, "{} enc(bad x25519 pk #{}) must be -1", tag, i);
        common::eqb(&format!("{} enc(bad x25519 pk) ct", tag), &ctc, &ctr);
        common::eqb(&format!("{} enc(bad x25519 pk) ss", tag), &ssc, &ssr);
        assert_eq!(ctc, vec![CANARY; XW_CT], "{} must not write ct", tag);
        assert_eq!(ssc, vec![CANARY; XW_SS], "{} must not write ss", tag);
    }
    // ---- enc against a public key with a non-canonical ML-KEM half ------
    for idx in [0usize, 767] {
        let mut pk = kps[1].0.clone();
        set_coeff(&mut pk, idx, 3329);
        let mut ctc = vec![CANARY; XW_CT];
        let mut ctr = vec![CANARY; XW_CT];
        let mut ssc = vec![CANARY; XW_SS];
        let mut ssr = vec![CANARY; XW_SS];
        let rc = unsafe { enc.0(ctc.as_mut_ptr(), ssc.as_mut_ptr(), pk.as_ptr()) };
        let rr = unsafe { enc.1(ctr.as_mut_ptr(), ssr.as_mut_ptr(), pk.as_ptr()) };
        common::eqi(&format!("{} enc(bad mlkem pk) rc", tag), rc, rr);
        assert_eq!(rc, -1, "{} enc(non-canonical mlkem pk) must be -1", tag);
        common::eqb(&format!("{} enc(bad mlkem pk) ct", tag), &ctc, &ctr);
        common::eqb(&format!("{} enc(bad mlkem pk) ss", tag), &ssc, &ssr);
    }

    // ---- enc (randombytes): rc + cross-library round trips -------------
    let (pk0, sk0) = &kps[0];
    for _ in 0..4 {
        let mut ctc = vec![0u8; XW_CT];
        let mut ssc = vec![0u8; XW_SS];
        let mut ctr = vec![0u8; XW_CT];
        let mut ssr = vec![0u8; XW_SS];
        let rc = unsafe { enc.0(ctc.as_mut_ptr(), ssc.as_mut_ptr(), pk0.as_ptr()) };
        let rr = unsafe { enc.1(ctr.as_mut_ptr(), ssr.as_mut_ptr(), pk0.as_ptr()) };
        common::eqi(&format!("{} enc rc", tag), rc, rr);
        assert_eq!(rc, 0);
        let mut out = vec![0u8; XW_SS];
        assert_eq!(
            unsafe { dec.1(out.as_mut_ptr(), ctc.as_ptr(), sk0.as_ptr()) },
            0
        );
        assert_eq!(out, ssc, "{} cross enc C -> dec Rust", tag);
        let mut out2 = vec![0u8; XW_SS];
        assert_eq!(
            unsafe { dec.0(out2.as_mut_ptr(), ctr.as_ptr(), sk0.as_ptr()) },
            0
        );
        assert_eq!(out2, ssr, "{} cross enc Rust -> dec C", tag);
    }

    // ---- keypair (randombytes) -----------------------------------------
    for _ in 0..3 {
        let mut pkc = vec![CANARY; XW_PK];
        let mut skc = vec![CANARY; XW_SK];
        let mut pkr = vec![CANARY; XW_PK];
        let mut skr = vec![CANARY; XW_SK];
        let rc = unsafe { kp_api.0(pkc.as_mut_ptr(), skc.as_mut_ptr()) };
        let rr = unsafe { kp_api.1(pkr.as_mut_ptr(), skr.as_mut_ptr()) };
        common::eqi(&format!("{} keypair rc", tag), rc, rr);
        assert_eq!(rc, 0);
        assert_ne!(pkc, vec![CANARY; XW_PK]);
        let mut ct = vec![0u8; XW_CT];
        let mut ss1 = vec![0u8; XW_SS];
        assert_eq!(
            unsafe { enc.1(ct.as_mut_ptr(), ss1.as_mut_ptr(), pkc.as_ptr()) },
            0
        );
        let mut ss2 = vec![0u8; XW_SS];
        assert_eq!(
            unsafe { dec.0(ss2.as_mut_ptr(), ct.as_ptr(), skc.as_ptr()) },
            0
        );
        assert_eq!(ss1, ss2, "{} cross keypair round trip", tag);
        // the Rust lib must derive the same pk from the C-generated seed/sk
        let mut pk_from_sk = vec![0u8; XW_PK];
        let mut sk_echo = vec![0u8; XW_SK];
        assert_eq!(
            unsafe {
                seed_kp_api.1(
                    pk_from_sk.as_mut_ptr(),
                    sk_echo.as_mut_ptr(),
                    skc.as_ptr(),
                )
            },
            0
        );
        common::eqb(&format!("{} keypair sk expands to pk", tag), &pk_from_sk, &pkc);
    }

    // ---- decapsulation of tampered / malformed ciphertexts -------------
    let (pk, sk) = &kps[3];
    let mut ct = vec![0u8; XW_CT];
    let mut ss = vec![0u8; XW_SS];
    assert_eq!(
        unsafe { enc.0(ct.as_mut_ptr(), ss.as_mut_ptr(), pk.as_ptr()) },
        0
    );
    // every single byte of the ciphertext (ML-KEM part -> implicit rejection,
    // X25519 part -> a different but still valid shared secret)
    for i in 0..XW_CT {
        let mut bad = ct.clone();
        bad[i] ^= 0x33;
        let mut dc = vec![CANARY; XW_SS + 4];
        let mut dr = vec![CANARY; XW_SS + 4];
        let a = unsafe { dec.0(dc.as_mut_ptr(), bad.as_ptr(), sk.as_ptr()) };
        let b = unsafe { dec.1(dr.as_mut_ptr(), bad.as_ptr(), sk.as_ptr()) };
        common::eqi(&format!("{} dec(tamper[{}]) rc", tag, i), a, b);
        common::eqb(&format!("{} dec(tamper[{}]) ss", tag, i), &dc, &dr);
        assert_eq!(a, 0, "{} tampering must not make dec fail", tag);
    }
    // ct_x25519 part replaced by a low-order point -> scalarmult fails -> -1
    for (i, bad) in bad_public_keys().iter().enumerate() {
        let mut b2 = ct.clone();
        b2[MLK_CT..].copy_from_slice(bad);
        let mut dc = vec![CANARY; XW_SS];
        let mut dr = vec![CANARY; XW_SS];
        let a = unsafe { dec.0(dc.as_mut_ptr(), b2.as_ptr(), sk.as_ptr()) };
        let b = unsafe { dec.1(dr.as_mut_ptr(), b2.as_ptr(), sk.as_ptr()) };
        common::eqi(&format!("{} dec(bad ct_x25519 #{}) rc", tag, i), a, b);
        assert_eq!(a, -1, "{} dec(bad ct_x25519 #{}) must be -1", tag, i);
        common::eqb(&format!("{} dec(bad ct_x25519) ss", tag), &dc, &dr);
        assert_eq!(dc, vec![CANARY; XW_SS], "{} error must not write ss", tag);
    }
    // fully random ciphertexts and secret keys
    for _ in 0..8 {
        let bad = rng.bytes(XW_CT);
        let rsk = rng.bytes(XW_SK);
        let mut dc = vec![CANARY; XW_SS];
        let mut dr = vec![CANARY; XW_SS];
        let a = unsafe { dec.0(dc.as_mut_ptr(), bad.as_ptr(), rsk.as_ptr()) };
        let b = unsafe { dec.1(dr.as_mut_ptr(), bad.as_ptr(), rsk.as_ptr()) };
        common::eqi(&format!("{} dec(random ct/sk) rc", tag), a, b);
        common::eqb(&format!("{} dec(random ct/sk) ss", tag), &dc, &dr);
    }
    for pat in [0x00u8, 0xff] {
        let bad = vec![pat; XW_CT];
        let mut dc = vec![CANARY; XW_SS];
        let mut dr = vec![CANARY; XW_SS];
        let a = unsafe { dec.0(dc.as_mut_ptr(), bad.as_ptr(), sk.as_ptr()) };
        let b = unsafe { dec.1(dr.as_mut_ptr(), bad.as_ptr(), sk.as_ptr()) };
        common::eqi(&format!("{} dec(ct={:#x}) rc", tag, pat), a, b);
        common::eqb(&format!("{} dec(ct={:#x}) ss", tag, pat), &dc, &dr);
    }
    // wrong secret key -> succeeds but yields a different shared secret
    {
        let wrong = &kps[4].1;
        let mut dc = vec![CANARY; XW_SS];
        let mut dr = vec![CANARY; XW_SS];
        let a = unsafe { dec.0(dc.as_mut_ptr(), ct.as_ptr(), wrong.as_ptr()) };
        let b = unsafe { dec.1(dr.as_mut_ptr(), ct.as_ptr(), wrong.as_ptr()) };
        common::eqi(&format!("{} dec(wrong sk) rc", tag), a, b);
        common::eqb(&format!("{} dec(wrong sk) ss", tag), &dc, &dr);
        assert_ne!(dc, ss, "{} wrong sk must not recover ss", tag);
    }
}

#[test]
fn kem_xwing() {
    xwing_suite("crypto_kem_xwing_", true, "xwing");
}

#[test]
fn kem_generic_dispatch() {
    // crypto_kem_* has no enc_deterministic; it dispatches to xwing.
    xwing_suite("crypto_kem_", false, "kem");

    // crypto_kem_* must be identical to crypto_kem_xwing_* in both libs.
    let g_skp = syms::<SeedKp>("crypto_kem_seed_keypair");
    let x_skp = syms::<SeedKp>("crypto_kem_xwing_seed_keypair");
    let g_dec = syms::<KemDec>("crypto_kem_dec");
    let x_dec = syms::<KemDec>("crypto_kem_xwing_dec");
    let x_encd = syms::<KemEncDet>("crypto_kem_xwing_enc_deterministic");

    let mut rng = common::Rng::new(0xFEED_5678);
    for _ in 0..4 {
        let seed = rng.bytes(XW_SEED);
        for (lib, gf, xf) in [(0, g_skp.0, x_skp.0), (1, g_skp.1, x_skp.1)] {
            let mut pk1 = vec![0u8; XW_PK];
            let mut sk1 = vec![0u8; XW_SK];
            let mut pk2 = vec![0u8; XW_PK];
            let mut sk2 = vec![0u8; XW_SK];
            assert_eq!(
                unsafe { gf(pk1.as_mut_ptr(), sk1.as_mut_ptr(), seed.as_ptr()) },
                0
            );
            assert_eq!(
                unsafe { xf(pk2.as_mut_ptr(), sk2.as_mut_ptr(), seed.as_ptr()) },
                0
            );
            common::eqb(&format!("lib{} kem pk == xwing pk", lib), &pk1, &pk2);
            common::eqb(&format!("lib{} kem sk == xwing sk", lib), &sk1, &sk2);

            let eseed = vec![0x21u8; 64];
            let mut ct = vec![0u8; XW_CT];
            let mut ss = vec![0u8; XW_SS];
            let ef = if lib == 0 { x_encd.0 } else { x_encd.1 };
            assert_eq!(
                unsafe { ef(ct.as_mut_ptr(), ss.as_mut_ptr(), pk1.as_ptr(), eseed.as_ptr()) },
                0
            );
            let mut d1 = vec![0u8; XW_SS];
            let mut d2 = vec![0u8; XW_SS];
            let (gd, xd) = if lib == 0 {
                (g_dec.0, x_dec.0)
            } else {
                (g_dec.1, x_dec.1)
            };
            assert_eq!(unsafe { gd(d1.as_mut_ptr(), ct.as_ptr(), sk1.as_ptr()) }, 0);
            assert_eq!(unsafe { xd(d2.as_mut_ptr(), ct.as_ptr(), sk1.as_ptr()) }, 0);
            common::eqb(&format!("lib{} kem dec == xwing dec", lib), &d1, &d2);
            assert_eq!(d1, ss, "lib{} round trip", lib);
        }
    }
}
