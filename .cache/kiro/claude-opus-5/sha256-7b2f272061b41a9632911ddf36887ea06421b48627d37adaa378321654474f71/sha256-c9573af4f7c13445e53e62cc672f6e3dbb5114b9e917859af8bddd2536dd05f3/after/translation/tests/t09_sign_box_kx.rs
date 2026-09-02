//! Phase B + C for `crypto_sign/` (ed25519), `crypto_box/` (both
//! curve25519xsalsa20poly1305 and curve25519xchacha20poly1305 plus the generic
//! `crypto_box_*` facade) and `crypto_kx/`.
//!
//! Every entry point is exercised through the `#[no_mangle]` `extern "C"`
//! exports of BOTH shared objects and compared byte-for-byte, exactly as an
//! external C consumer would call them. The C at `c_src/libsodium/` is the
//! ground truth.
//!
//! Determinism contract (see the module-level notes in the task):
//!   * `*_keypair()` and `crypto_box_seal()` draw from the CSPRNG, so their raw
//!     bytes are NOT comparable. They are handled by (a) comparing the return
//!     code + the output-buffer canary and (b) CROSS-CHECKING: one library
//!     produces the value, the other consumes/verifies it, in both directions.
//!   * everything driven from a fixed seed (`seed_keypair`, `sign`, `detached`,
//!     `open`, `verify_detached`, `beforenm`, `afternm`, `easy`, `detached`
//!     box, `seal_open`, the ph streaming API, the key-conversion helpers) IS
//!     deterministic and is compared byte-for-byte.
//!
//! The C `.so` is built WITHOUT `-DNDEBUG`, so `assert()` is live and the
//! `sodium_misuse()` paths abort the process. Anything that can reach those is
//! routed through `same_outcome()` (forked children).

mod harness;
use harness::*;

use std::ffi::{c_char, c_int};
use std::ptr;

const SEED: u64 = 0x5EED_0009;

// ed25519 sizes
const SIGN_BYTES: usize = 64;
const SIGN_SEEDBYTES: usize = 32;
const SIGN_PK: usize = 32;
const SIGN_SK: usize = 64;

// curve25519 box sizes (identical across xsalsa / xchacha / generic)
const BOX_PK: usize = 32;
const BOX_SK: usize = 32;
const BOX_SEED: usize = 32;
const BOX_NONCE: usize = 24;
const BOX_MAC: usize = 16;
const BOX_BEFORENM: usize = 32;
const BOX_SEALBYTES: usize = BOX_PK + BOX_MAC; // 48
const BOX_ZEROBYTES: usize = 32;
const BOX_BOXZEROBYTES: usize = 16;

// kx sizes
const KX_PK: usize = 32;
const KX_SK: usize = 32;
const KX_SEED: usize = 32;
const KX_SESSION: usize = 32;

/// The 8 known small-order ed25519 point encodings (canonical). Must be
/// rejected by `crypto_sign_ed25519_verify_detached` (public key or R) and by
/// `crypto_sign_ed25519_pk_to_curve25519`. The comparison is C-vs-Rust either
/// way; the C is ground truth.
const SMALL_ORDER_POINTS: [[u8; 32]; 8] = [
    [
        0x01, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0,
    ],
    [
        0xec, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
        0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
        0xff, 0x7f,
    ],
    [
        0x00, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0,
    ],
    [
        0x00, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0x80,
    ],
    [
        0x26, 0xe8, 0x95, 0x8f, 0xc2, 0xb2, 0x27, 0xb0, 0x45, 0xc3, 0xf4, 0x89, 0xf2, 0xef, 0x98,
        0xf0, 0xd5, 0xdf, 0xac, 0x05, 0xd3, 0xc6, 0x33, 0x39, 0xb1, 0x38, 0x02, 0x88, 0x6d, 0x53,
        0xfc, 0x05,
    ],
    [
        0xc7, 0x17, 0x6a, 0x70, 0x3d, 0x4d, 0xd8, 0x4f, 0xba, 0x3c, 0x0b, 0x76, 0x0d, 0x10, 0x67,
        0x0f, 0x2a, 0x20, 0x53, 0xfa, 0x2c, 0x39, 0xcc, 0xc6, 0x4e, 0xc7, 0xfd, 0x77, 0x92, 0xac,
        0x03, 0x7a,
    ],
    [
        0x13, 0xe8, 0x95, 0x8f, 0xc2, 0xb2, 0x27, 0xb0, 0x45, 0xc3, 0xf4, 0x89, 0xf2, 0xef, 0x98,
        0xf0, 0xd5, 0xdf, 0xac, 0x05, 0xd3, 0xc6, 0x33, 0x39, 0xb1, 0x38, 0x02, 0x88, 0x6d, 0x53,
        0xfc, 0x85,
    ],
    [
        0xb4, 0x17, 0x6a, 0x70, 0x3d, 0x4d, 0xd8, 0x4f, 0xba, 0x3c, 0x0b, 0x76, 0x0d, 0x10, 0x67,
        0x0f, 0x2a, 0x20, 0x53, 0xfa, 0x2c, 0x39, 0xcc, 0xc6, 0x4e, 0xc7, 0xfd, 0x77, 0x92, 0xac,
        0x03, 0xfa,
    ],
];

// Message lengths spanning the SHA-512 block boundary (128) used throughout.
fn sign_mlens() -> Vec<usize> {
    vec![0, 1, 31, 32, 33, 63, 64, 65, 127, 128, 129, 1000]
}

// Common function pointer signatures.
type SizeFn = unsafe extern "C" fn() -> usize;
type PrimFn = unsafe extern "C" fn() -> *const c_char;

type SeedKp = unsafe extern "C" fn(*mut u8, *mut u8, *const u8) -> c_int;
type Kp = unsafe extern "C" fn(*mut u8, *mut u8) -> c_int;

type SignFn = unsafe extern "C" fn(*mut u8, *mut u64, *const u8, u64, *const u8) -> c_int;
type OpenFn = unsafe extern "C" fn(*mut u8, *mut u64, *const u8, u64, *const u8) -> c_int;
type DetachedFn = unsafe extern "C" fn(*mut u8, *mut u64, *const u8, u64, *const u8) -> c_int;
type VerifyDetFn = unsafe extern "C" fn(*const u8, *const u8, u64, *const u8) -> c_int;

type PhInit = unsafe extern "C" fn(*mut u8) -> c_int;
type PhUpdate = unsafe extern "C" fn(*mut u8, *const u8, u64) -> c_int;
type PhFinalCreate = unsafe extern "C" fn(*mut u8, *mut u8, *mut u64, *const u8) -> c_int;
type PhFinalVerify = unsafe extern "C" fn(*mut u8, *const u8, *const u8) -> c_int;

type Conv1 = unsafe extern "C" fn(*mut u8, *const u8) -> c_int; // sk_to_seed/pk, pk/sk_to_curve

// box
type Beforenm = unsafe extern "C" fn(*mut u8, *const u8, *const u8) -> c_int;
type Afternm = unsafe extern "C" fn(*mut u8, *const u8, u64, *const u8, *const u8) -> c_int;
type BoxFull = unsafe extern "C" fn(*mut u8, *const u8, u64, *const u8, *const u8, *const u8) -> c_int;
type EasyAfternm = unsafe extern "C" fn(*mut u8, *const u8, u64, *const u8, *const u8) -> c_int;
type DetAfternm =
    unsafe extern "C" fn(*mut u8, *mut u8, *const u8, u64, *const u8, *const u8) -> c_int;
type OpenDetAfternm =
    unsafe extern "C" fn(*mut u8, *const u8, *const u8, u64, *const u8, *const u8) -> c_int;
type DetFull =
    unsafe extern "C" fn(*mut u8, *mut u8, *const u8, u64, *const u8, *const u8, *const u8) -> c_int;
type OpenDetFull =
    unsafe extern "C" fn(*mut u8, *const u8, *const u8, u64, *const u8, *const u8, *const u8) -> c_int;
type SealFn = unsafe extern "C" fn(*mut u8, *const u8, u64, *const u8) -> c_int;
type SealOpenFn =
    unsafe extern "C" fn(*mut u8, *const u8, u64, *const u8, *const u8) -> c_int;

// kx
type KxSession = unsafe extern "C" fn(*mut u8, *mut u8, *const u8, *const u8, *const u8) -> c_int;

fn size_of(name: &str) -> usize {
    let (c, r) = sym::<SizeFn>(name);
    let (a, b) = unsafe { (c(), r()) };
    assert_eq!(a, b, "{name} size disagree");
    a
}

// ===========================================================================
// crypto_sign — constants & primitive
// ===========================================================================

#[test]
fn sign_constants_agree() {
    for (name, want) in [
        ("crypto_sign_bytes", SIGN_BYTES),
        ("crypto_sign_seedbytes", SIGN_SEEDBYTES),
        ("crypto_sign_publickeybytes", SIGN_PK),
        ("crypto_sign_secretkeybytes", SIGN_SK),
        ("crypto_sign_ed25519_bytes", SIGN_BYTES),
        ("crypto_sign_ed25519_seedbytes", SIGN_SEEDBYTES),
        ("crypto_sign_ed25519_publickeybytes", SIGN_PK),
        ("crypto_sign_ed25519_secretkeybytes", SIGN_SK),
    ] {
        assert_eq!(size_of(name), want, "{name}");
    }
    // statebytes agree (both generic and ed25519ph)
    let sb = size_of("crypto_sign_statebytes");
    let sbp = size_of("crypto_sign_ed25519ph_statebytes");
    assert_eq!(sb, sbp, "statebytes generic vs ed25519ph");
    // messagebytes_max agree
    assert_eq!(
        size_of("crypto_sign_messagebytes_max"),
        size_of("crypto_sign_ed25519_messagebytes_max"),
        "sign messagebytes_max"
    );
    // primitive string
    for name in ["crypto_sign_primitive"] {
        let (c, r) = sym::<PrimFn>(name);
        unsafe {
            let cs = std::ffi::CStr::from_ptr(c());
            let rs = std::ffi::CStr::from_ptr(r());
            assert_eq!(cs, rs, "{name}");
            assert_eq!(cs.to_str().unwrap(), "ed25519");
        }
    }
}

// ===========================================================================
// 1. seed_keypair — byte-exact pk and sk
// ===========================================================================

#[test]
fn sign_seed_keypair_byte_exact() {
    let mut rng = Rng::new(SEED);
    for name in ["crypto_sign_ed25519_seed_keypair", "crypto_sign_seed_keypair"] {
        let (c, r) = sym::<SeedKp>(name);
        // many random seeds plus the two extreme seeds
        let mut seeds: Vec<Vec<u8>> = (0..64).map(|_| rng.bytes(SIGN_SEEDBYTES)).collect();
        seeds.push(vec![0u8; SIGN_SEEDBYTES]);
        seeds.push(vec![0xffu8; SIGN_SEEDBYTES]);
        for (i, seed) in seeds.iter().enumerate() {
            let mut pkc = out_buf(SIGN_PK);
            let mut pkr = out_buf(SIGN_PK);
            let mut skc = out_buf(SIGN_SK);
            let mut skr = out_buf(SIGN_SK);
            unsafe {
                let rc = c(pkc.as_mut_ptr(), skc.as_mut_ptr(), seed.as_ptr());
                let rr = r(pkr.as_mut_ptr(), skr.as_mut_ptr(), seed.as_ptr());
                assert_eq!(rc, rr, "{name} rc seed#{i}");
                assert_eq!(rc, 0, "{name} rc seed#{i}");
            }
            eqb(&format!("{name} pk seed#{i}"), &pkc, &pkr);
            eqb(&format!("{name} sk seed#{i}"), &skc, &skr);
            // the sk layout is seed(32) || pk(32); verify the pk copy internally
            eqb(&format!("{name} sk-embeds-seed seed#{i}"), &seed[..], &skc[..32]);
            eqb(&format!("{name} sk-embeds-pk seed#{i}"), &pkc[..32], &skc[32..64]);
        }
    }
}

// ===========================================================================
// 2. keypair — cross-check only (CSPRNG)
// ===========================================================================

#[test]
fn sign_keypair_cross_check() {
    for name in ["crypto_sign_ed25519_keypair", "crypto_sign_keypair"] {
        let (ckp, rkp) = sym::<Kp>(name);
        let (csign, _) = sym::<DetachedFn>("crypto_sign_ed25519_detached");
        let (_, rsign) = sym::<DetachedFn>("crypto_sign_ed25519_detached");
        let (cver, rver) = sym::<VerifyDetFn>("crypto_sign_ed25519_verify_detached");
        for i in 0..16 {
            // C makes a keypair; Rust verifies a signature made with it, and
            // vice-versa. Only the canary + rc are checked on the keypair call.
            let mut pkc = out_buf(SIGN_PK);
            let mut skc = out_buf(SIGN_SK);
            let mut pkr = out_buf(SIGN_PK);
            let mut skr = out_buf(SIGN_SK);
            unsafe {
                let rc = ckp(pkc.as_mut_ptr(), skc.as_mut_ptr());
                let rr = rkp(pkr.as_mut_ptr(), skr.as_mut_ptr());
                assert_eq!(rc, rr, "{name} rc#{i}");
                assert_eq!(rc, 0);
            }
            eqb(&format!("{name} pk canary#{i}"), &pkc[SIGN_PK..], &pkr[SIGN_PK..]);
            eqb(&format!("{name} sk canary#{i}"), &skc[SIGN_SK..], &skr[SIGN_SK..]);

            let msg = vec![0x42u8; 40 + i];
            // sign with C key, verify with both libs using C's pk
            let mut sig = vec![0u8; SIGN_BYTES];
            unsafe {
                assert_eq!(
                    csign(sig.as_mut_ptr(), ptr::null_mut(), msg.as_ptr(), msg.len() as u64, skc.as_ptr()),
                    0
                );
                let a = cver(sig.as_ptr(), msg.as_ptr(), msg.len() as u64, pkc.as_ptr());
                let b = rver(sig.as_ptr(), msg.as_ptr(), msg.len() as u64, pkc.as_ptr());
                assert_eq!(a, b, "{name} cross C-key verify#{i}");
                assert_eq!(a, 0, "{name} C-key sig must verify#{i}");
            }
            // sign with Rust key, verify with both using Rust's pk
            let mut sig2 = vec![0u8; SIGN_BYTES];
            unsafe {
                assert_eq!(
                    rsign(sig2.as_mut_ptr(), ptr::null_mut(), msg.as_ptr(), msg.len() as u64, skr.as_ptr()),
                    0
                );
                let a = cver(sig2.as_ptr(), msg.as_ptr(), msg.len() as u64, pkr.as_ptr());
                let b = rver(sig2.as_ptr(), msg.as_ptr(), msg.len() as u64, pkr.as_ptr());
                assert_eq!(a, b, "{name} cross Rust-key verify#{i}");
                assert_eq!(a, 0, "{name} Rust-key sig must verify#{i}");
            }
        }
    }
}

// ===========================================================================
// 3. crypto_sign / _open (combined)
// ===========================================================================

#[test]
fn sign_combined_and_open() {
    let mut rng = Rng::new(SEED ^ 3);
    let (skp, _) = sym::<SeedKp>("crypto_sign_ed25519_seed_keypair");
    for (sign_n, open_n) in [
        ("crypto_sign", "crypto_sign_open"),
        ("crypto_sign_ed25519", "crypto_sign_ed25519_open"),
    ] {
        let (cs, rs) = sym::<SignFn>(sign_n);
        let (co, ro) = sym::<OpenFn>(open_n);
        for mlen in sign_mlens() {
            let seed = rng.bytes(SIGN_SEEDBYTES);
            let mut pk = vec![0u8; SIGN_PK];
            let mut sk = vec![0u8; SIGN_SK];
            unsafe {
                skp(pk.as_mut_ptr(), sk.as_mut_ptr(), seed.as_ptr());
            }
            let m = rng.bytes(mlen);
            let smlen = mlen + SIGN_BYTES;

            let mut smc = out_buf(smlen);
            let mut smr = out_buf(smlen);
            let mut lc = u64::MAX;
            let mut lr = u64::MAX;
            unsafe {
                let rc = cs(smc.as_mut_ptr(), &mut lc, m.as_ptr(), mlen as u64, sk.as_ptr());
                let rr = rs(smr.as_mut_ptr(), &mut lr, m.as_ptr(), mlen as u64, sk.as_ptr());
                assert_eq!(rc, rr, "{sign_n} rc mlen={mlen}");
                assert_eq!(lc, lr, "{sign_n} smlen mlen={mlen}");
                assert_eq!(lc, smlen as u64, "{sign_n} smlen value mlen={mlen}");
            }
            eqb(&format!("{sign_n} signed-message mlen={mlen}"), &smc, &smr);

            // smlen_p == NULL is allowed
            let mut smc2 = out_buf(smlen);
            let mut smr2 = out_buf(smlen);
            unsafe {
                cs(smc2.as_mut_ptr(), ptr::null_mut(), m.as_ptr(), mlen as u64, sk.as_ptr());
                rs(smr2.as_mut_ptr(), ptr::null_mut(), m.as_ptr(), mlen as u64, sk.as_ptr());
            }
            eqb(&format!("{sign_n} smlen_p=NULL mlen={mlen}"), &smc2, &smr2);

            // open the good signed message
            let mut oc = out_buf(mlen.max(1));
            let mut or = out_buf(mlen.max(1));
            let mut oc_l = u64::MAX;
            let mut or_l = u64::MAX;
            unsafe {
                let rc = co(oc.as_mut_ptr(), &mut oc_l, smc.as_ptr(), smlen as u64, pk.as_ptr());
                let rr = ro(or.as_mut_ptr(), &mut or_l, smr.as_ptr(), smlen as u64, pk.as_ptr());
                assert_eq!(rc, rr, "{open_n} rc mlen={mlen}");
                assert_eq!(rc, 0, "{open_n} should verify mlen={mlen}");
                assert_eq!(oc_l, or_l, "{open_n} mlen out mlen={mlen}");
                assert_eq!(oc_l, mlen as u64, "{open_n} mlen out value");
            }
            eqb(&format!("{open_n} recovered mlen={mlen}"), &oc, &or);
            eqb(&format!("{open_n} roundtrip mlen={mlen}"), &m, &oc[..mlen]);

            // mlen_p == NULL, and m == NULL (verify-only)
            let mut oc2 = out_buf(mlen.max(1));
            let mut or2 = out_buf(mlen.max(1));
            unsafe {
                let rc = co(oc2.as_mut_ptr(), ptr::null_mut(), smc.as_ptr(), smlen as u64, pk.as_ptr());
                let rr = ro(or2.as_mut_ptr(), ptr::null_mut(), smr.as_ptr(), smlen as u64, pk.as_ptr());
                assert_eq!(rc, rr, "{open_n} mlen_p=NULL rc mlen={mlen}");
            }
            eqb(&format!("{open_n} mlen_p=NULL out mlen={mlen}"), &oc2, &or2);
            unsafe {
                let rc = co(ptr::null_mut(), ptr::null_mut(), smc.as_ptr(), smlen as u64, pk.as_ptr());
                let rr = ro(ptr::null_mut(), ptr::null_mut(), smr.as_ptr(), smlen as u64, pk.as_ptr());
                assert_eq!(rc, rr, "{open_n} m=NULL rc mlen={mlen}");
                assert_eq!(rc, 0);
            }

            // every smlen from 0..=64 fed to _open (the smlen<64 rejection)
            for shortlen in 0u64..=64 {
                let mut oc = out_buf(mlen.max(1));
                let mut or = out_buf(mlen.max(1));
                let mut a = 0u64;
                let mut b = 0u64;
                unsafe {
                    let rc = co(oc.as_mut_ptr(), &mut a, smc.as_ptr(), shortlen, pk.as_ptr());
                    let rr = ro(or.as_mut_ptr(), &mut b, smr.as_ptr(), shortlen, pk.as_ptr());
                    assert_eq!(rc, rr, "{open_n} short smlen={shortlen} mlen={mlen}");
                    assert_eq!(a, b, "{open_n} short mlen_out smlen={shortlen}");
                }
                eqb(&format!("{open_n} short smlen={shortlen} m={mlen}"), &oc, &or);
            }

            if smlen > 64 {
                // smlen not matching (truncate / extend by 1)
                for wrong in [smlen - 1, smlen + 1] {
                    // build a buffer of at least `wrong` bytes
                    let mut buf_c = smc[..smlen].to_vec();
                    let mut buf_r = smr[..smlen].to_vec();
                    buf_c.resize(wrong.max(smlen), 0);
                    buf_r.resize(wrong.max(smlen), 0);
                    let mut oc = out_buf(wrong.max(1));
                    let mut or = out_buf(wrong.max(1));
                    let mut a = 0u64;
                    let mut b = 0u64;
                    unsafe {
                        let rc = co(oc.as_mut_ptr(), &mut a, buf_c.as_ptr(), wrong as u64, pk.as_ptr());
                        let rr = ro(or.as_mut_ptr(), &mut b, buf_r.as_ptr(), wrong as u64, pk.as_ptr());
                        assert_eq!(rc, rr, "{open_n} wrong smlen={wrong} mlen={mlen}");
                    }
                    eqb(&format!("{open_n} wrong smlen={wrong} m={mlen}"), &oc[..mlen.max(1)], &or[..mlen.max(1)]);
                }
            }

            // single-bit corruption of the signature, the message body and the pk
            for target in ["sig", "body", "pk"] {
                if target == "body" && mlen == 0 {
                    continue;
                }
                let mut bad = smc[..smlen].to_vec();
                let mut bad_pk = pk.clone();
                match target {
                    "sig" => bad[rng.below(SIGN_BYTES)] ^= 1 << (rng.below(8)),
                    "body" => bad[SIGN_BYTES + rng.below(mlen)] ^= 1 << (rng.below(8)),
                    _ => bad_pk[rng.below(SIGN_PK)] ^= 1 << (rng.below(8)),
                }
                let mut oc = out_buf(mlen.max(1));
                let mut or = out_buf(mlen.max(1));
                let mut a = 0u64;
                let mut b = 0u64;
                unsafe {
                    let rc = co(oc.as_mut_ptr(), &mut a, bad.as_ptr(), smlen as u64, bad_pk.as_ptr());
                    let rr = ro(or.as_mut_ptr(), &mut b, bad.as_ptr(), smlen as u64, bad_pk.as_ptr());
                    assert_eq!(rc, rr, "{open_n} corrupt {target} mlen={mlen}");
                }
                eqb(&format!("{open_n} corrupt {target} out mlen={mlen}"), &oc, &or);
            }
        }
    }
}

// ===========================================================================
// 4. detached / verify_detached
// ===========================================================================

#[test]
fn sign_detached_and_verify() {
    let mut rng = Rng::new(SEED ^ 4);
    let (skp, _) = sym::<SeedKp>("crypto_sign_ed25519_seed_keypair");
    for (det_n, ver_n) in [
        ("crypto_sign_detached", "crypto_sign_verify_detached"),
        ("crypto_sign_ed25519_detached", "crypto_sign_ed25519_verify_detached"),
    ] {
        let (cd, rd) = sym::<DetachedFn>(det_n);
        let (cv, rv) = sym::<VerifyDetFn>(ver_n);
        for mlen in sign_mlens() {
            let seed = rng.bytes(SIGN_SEEDBYTES);
            let mut pk = vec![0u8; SIGN_PK];
            let mut sk = vec![0u8; SIGN_SK];
            unsafe {
                skp(pk.as_mut_ptr(), sk.as_mut_ptr(), seed.as_ptr());
            }
            let m = rng.bytes(mlen);

            let mut sc = out_buf(SIGN_BYTES);
            let mut sr = out_buf(SIGN_BYTES);
            let mut lc = u64::MAX;
            let mut lr = u64::MAX;
            unsafe {
                let rc = cd(sc.as_mut_ptr(), &mut lc, m.as_ptr(), mlen as u64, sk.as_ptr());
                let rr = rd(sr.as_mut_ptr(), &mut lr, m.as_ptr(), mlen as u64, sk.as_ptr());
                assert_eq!(rc, rr, "{det_n} rc mlen={mlen}");
                assert_eq!(lc, lr, "{det_n} siglen mlen={mlen}");
                assert_eq!(lc, SIGN_BYTES as u64);
            }
            eqb(&format!("{det_n} signature mlen={mlen}"), &sc, &sr);

            // siglen_p == NULL
            let mut sc2 = out_buf(SIGN_BYTES);
            let mut sr2 = out_buf(SIGN_BYTES);
            unsafe {
                cd(sc2.as_mut_ptr(), ptr::null_mut(), m.as_ptr(), mlen as u64, sk.as_ptr());
                rd(sr2.as_mut_ptr(), ptr::null_mut(), m.as_ptr(), mlen as u64, sk.as_ptr());
            }
            eqb(&format!("{det_n} siglen_p=NULL mlen={mlen}"), &sc2, &sr2);

            // good verification
            unsafe {
                let a = cv(sc.as_ptr(), m.as_ptr(), mlen as u64, pk.as_ptr());
                let b = rv(sr.as_ptr(), m.as_ptr(), mlen as u64, pk.as_ptr());
                assert_eq!(a, b, "{ver_n} good rc mlen={mlen}");
                assert_eq!(a, 0, "{ver_n} good must verify mlen={mlen}");
            }

            // corrupted signature (each of a few bit flips)
            for _ in 0..3 {
                let mut bad = sc[..SIGN_BYTES].to_vec();
                bad[rng.below(SIGN_BYTES)] ^= 1 << rng.below(8);
                unsafe {
                    let a = cv(bad.as_ptr(), m.as_ptr(), mlen as u64, pk.as_ptr());
                    let b = rv(bad.as_ptr(), m.as_ptr(), mlen as u64, pk.as_ptr());
                    assert_eq!(a, b, "{ver_n} corrupt sig mlen={mlen}");
                }
            }
            // corrupted message
            if mlen > 0 {
                let mut badm = m.clone();
                badm[rng.below(mlen)] ^= 1 << rng.below(8);
                unsafe {
                    let a = cv(sc.as_ptr(), badm.as_ptr(), mlen as u64, pk.as_ptr());
                    let b = rv(sr.as_ptr(), badm.as_ptr(), mlen as u64, pk.as_ptr());
                    assert_eq!(a, b, "{ver_n} corrupt msg mlen={mlen}");
                    assert_eq!(a, -1);
                }
            }

            // non-canonical S scalar: set high bits of sig[63]
            for hi in [0x20u8, 0x40, 0x80, 0xe0, 0xf0] {
                let mut ncs = sc[..SIGN_BYTES].to_vec();
                ncs[63] |= hi;
                unsafe {
                    let a = cv(ncs.as_ptr(), m.as_ptr(), mlen as u64, pk.as_ptr());
                    let b = rv(ncs.as_ptr(), m.as_ptr(), mlen as u64, pk.as_ptr());
                    assert_eq!(a, b, "{ver_n} non-canonical S hi={hi:#x} mlen={mlen}");
                }
            }

            // all-zero signature
            let zsig = vec![0u8; SIGN_BYTES];
            unsafe {
                let a = cv(zsig.as_ptr(), m.as_ptr(), mlen as u64, pk.as_ptr());
                let b = rv(zsig.as_ptr(), m.as_ptr(), mlen as u64, pk.as_ptr());
                assert_eq!(a, b, "{ver_n} all-zero sig mlen={mlen}");
                assert_eq!(a, -1);
            }

            // small-order public keys
            for (i, sop) in SMALL_ORDER_POINTS.iter().enumerate() {
                unsafe {
                    let a = cv(sc.as_ptr(), m.as_ptr(), mlen as u64, sop.as_ptr());
                    let b = rv(sr.as_ptr(), m.as_ptr(), mlen as u64, sop.as_ptr());
                    assert_eq!(a, b, "{ver_n} small-order pk[{i}] mlen={mlen}");
                }
            }
            // non-canonical public keys: field element >= 2^255-19
            for k in 0u8..8 {
                let mut ncpk = vec![0xffu8; SIGN_PK];
                ncpk[0] = 0xed_u8.wrapping_add(k);
                ncpk[31] = 0xff;
                unsafe {
                    let a = cv(sc.as_ptr(), m.as_ptr(), mlen as u64, ncpk.as_ptr());
                    let b = rv(sr.as_ptr(), m.as_ptr(), mlen as u64, ncpk.as_ptr());
                    assert_eq!(a, b, "{ver_n} non-canonical pk[{k}] mlen={mlen}");
                }
            }
        }
    }
}

// ===========================================================================
// 5. ph streaming API
// ===========================================================================

#[test]
fn sign_ph_streaming() {
    let mut rng = Rng::new(SEED ^ 5);
    let statebytes = size_of("crypto_sign_ed25519ph_statebytes");
    assert!(statebytes > 0 && statebytes <= 512, "state size {statebytes}");
    let (skp, _) = sym::<SeedKp>("crypto_sign_ed25519_seed_keypair");

    // Chunkings that straddle the 128-byte SHA-512 block boundary.
    let chunkings: Vec<Vec<usize>> = vec![
        vec![0],
        vec![1],
        vec![127],
        vec![128],
        vec![129],
        vec![1, 127],
        vec![64, 64],
        vec![127, 1, 1],
        vec![63, 64, 1, 200],
        vec![128, 128, 128],
        vec![100, 28, 100, 300],
        vec![255, 1, 256],
    ];

    for (init_n, upd_n, fc_n, fv_n) in [
        (
            "crypto_sign_init",
            "crypto_sign_update",
            "crypto_sign_final_create",
            "crypto_sign_final_verify",
        ),
        (
            "crypto_sign_ed25519ph_init",
            "crypto_sign_ed25519ph_update",
            "crypto_sign_ed25519ph_final_create",
            "crypto_sign_ed25519ph_final_verify",
        ),
    ] {
        let (ci, ri) = sym::<PhInit>(init_n);
        let (cu, ru) = sym::<PhUpdate>(upd_n);
        let (cfc, rfc) = sym::<PhFinalCreate>(fc_n);
        let (cfv, rfv) = sym::<PhFinalVerify>(fv_n);

        for (ci_idx, chunks) in chunkings.iter().enumerate() {
            let seed = rng.bytes(SIGN_SEEDBYTES);
            let mut pk = vec![0u8; SIGN_PK];
            let mut sk = vec![0u8; SIGN_SK];
            unsafe {
                skp(pk.as_mut_ptr(), sk.as_mut_ptr(), seed.as_ptr());
            }
            let total: usize = chunks.iter().sum();
            let full = rng.bytes(total);

            // init; compare state after init
            let mut stc = vec![0xa5u8; statebytes + CANARY];
            let mut str_ = vec![0xa5u8; statebytes + CANARY];
            unsafe {
                let rc = ci(stc.as_mut_ptr());
                let rr = ri(str_.as_mut_ptr());
                assert_eq!(rc, rr, "{init_n} rc chunk#{ci_idx}");
            }
            eqb(&format!("{init_n} state chunk#{ci_idx}"), &stc, &str_);

            // update, comparing the state buffer after every call
            let mut off = 0usize;
            for (j, &clen) in chunks.iter().enumerate() {
                let piece = &full[off..off + clen];
                off += clen;
                unsafe {
                    let rc = cu(stc.as_mut_ptr(), piece.as_ptr(), clen as u64);
                    let rr = ru(str_.as_mut_ptr(), piece.as_ptr(), clen as u64);
                    assert_eq!(rc, rr, "{upd_n} rc chunk#{ci_idx}.{j}");
                }
                eqb(&format!("{upd_n} state chunk#{ci_idx}.{j}"), &stc, &str_);
            }

            // final_create: state is consumed; compare the state after, and the sig
            let mut stc_c = stc.clone();
            let mut str_c = str_.clone();
            let mut sigc = out_buf(SIGN_BYTES);
            let mut sigr = out_buf(SIGN_BYTES);
            let mut lc = u64::MAX;
            let mut lr = u64::MAX;
            unsafe {
                let rc = cfc(stc_c.as_mut_ptr(), sigc.as_mut_ptr(), &mut lc, sk.as_ptr());
                let rr = rfc(str_c.as_mut_ptr(), sigr.as_mut_ptr(), &mut lr, sk.as_ptr());
                assert_eq!(rc, rr, "{fc_n} rc chunk#{ci_idx}");
                assert_eq!(rc, 0);
                assert_eq!(lc, lr, "{fc_n} siglen chunk#{ci_idx}");
                assert_eq!(lc, SIGN_BYTES as u64);
            }
            eqb(&format!("{fc_n} signature chunk#{ci_idx}"), &sigc, &sigr);
            eqb(&format!("{fc_n} state-after chunk#{ci_idx}"), &stc_c, &str_c);

            // siglen_p == NULL
            let mut stc_c2 = stc.clone();
            let mut str_c2 = str_.clone();
            let mut sigc2 = out_buf(SIGN_BYTES);
            let mut sigr2 = out_buf(SIGN_BYTES);
            unsafe {
                cfc(stc_c2.as_mut_ptr(), sigc2.as_mut_ptr(), ptr::null_mut(), sk.as_ptr());
                rfc(str_c2.as_mut_ptr(), sigr2.as_mut_ptr(), ptr::null_mut(), sk.as_ptr());
            }
            eqb(&format!("{fc_n} siglen_p=NULL sig chunk#{ci_idx}"), &sigc2, &sigr2);

            // final_verify (good) — rebuild fresh state, feed the same data
            let mut vstc = vec![0u8; statebytes];
            let mut vstr = vec![0u8; statebytes];
            unsafe {
                ci(vstc.as_mut_ptr());
                ri(vstr.as_mut_ptr());
                cu(vstc.as_mut_ptr(), full.as_ptr(), total as u64);
                ru(vstr.as_mut_ptr(), full.as_ptr(), total as u64);
                let a = cfv(vstc.as_mut_ptr(), sigc.as_ptr(), pk.as_ptr());
                let b = rfv(vstr.as_mut_ptr(), sigr.as_ptr(), pk.as_ptr());
                assert_eq!(a, b, "{fv_n} good rc chunk#{ci_idx}");
                assert_eq!(a, 0, "{fv_n} good must verify chunk#{ci_idx}");
            }

            // final_verify (corrupted signature)
            let mut badsig = sigc[..SIGN_BYTES].to_vec();
            badsig[rng.below(SIGN_BYTES)] ^= 1 << rng.below(8);
            let mut vstc2 = vec![0u8; statebytes];
            let mut vstr2 = vec![0u8; statebytes];
            unsafe {
                ci(vstc2.as_mut_ptr());
                ri(vstr2.as_mut_ptr());
                cu(vstc2.as_mut_ptr(), full.as_ptr(), total as u64);
                ru(vstr2.as_mut_ptr(), full.as_ptr(), total as u64);
                let a = cfv(vstc2.as_mut_ptr(), badsig.as_ptr(), pk.as_ptr());
                let b = rfv(vstr2.as_mut_ptr(), badsig.as_ptr(), pk.as_ptr());
                assert_eq!(a, b, "{fv_n} corrupt rc chunk#{ci_idx}");
                assert_eq!(a, -1, "{fv_n} corrupt must fail chunk#{ci_idx}");
            }
        }
    }
}

// ===========================================================================
// 6. key-conversion helpers
// ===========================================================================

#[test]
fn sign_key_conversions() {
    let mut rng = Rng::new(SEED ^ 6);
    let (skp, _) = sym::<SeedKp>("crypto_sign_ed25519_seed_keypair");
    let (c_seed, r_seed) = sym::<Conv1>("crypto_sign_ed25519_sk_to_seed");
    let (c_pk, r_pk) = sym::<Conv1>("crypto_sign_ed25519_sk_to_pk");
    let (c_pkc, r_pkc) = sym::<Conv1>("crypto_sign_ed25519_pk_to_curve25519");
    let (c_skc, r_skc) = sym::<Conv1>("crypto_sign_ed25519_sk_to_curve25519");

    for i in 0..64 {
        let seed = if i == 0 {
            vec![0u8; SIGN_SEEDBYTES]
        } else if i == 1 {
            vec![0xffu8; SIGN_SEEDBYTES]
        } else {
            rng.bytes(SIGN_SEEDBYTES)
        };
        let mut pk = vec![0u8; SIGN_PK];
        let mut sk = vec![0u8; SIGN_SK];
        unsafe {
            skp(pk.as_mut_ptr(), sk.as_mut_ptr(), seed.as_ptr());
        }

        // sk_to_seed
        let mut sc = out_buf(SIGN_SEEDBYTES);
        let mut sr = out_buf(SIGN_SEEDBYTES);
        unsafe {
            let rc = c_seed(sc.as_mut_ptr(), sk.as_ptr());
            let rr = r_seed(sr.as_mut_ptr(), sk.as_ptr());
            assert_eq!(rc, rr, "sk_to_seed rc#{i}");
        }
        eqb(&format!("sk_to_seed#{i}"), &sc, &sr);
        eqb(&format!("sk_to_seed matches seed#{i}"), &seed[..], &sc[..SIGN_SEEDBYTES]);

        // sk_to_pk
        let mut pc = out_buf(SIGN_PK);
        let mut pr = out_buf(SIGN_PK);
        unsafe {
            let rc = c_pk(pc.as_mut_ptr(), sk.as_ptr());
            let rr = r_pk(pr.as_mut_ptr(), sk.as_ptr());
            assert_eq!(rc, rr, "sk_to_pk rc#{i}");
        }
        eqb(&format!("sk_to_pk#{i}"), &pc, &pr);
        eqb(&format!("sk_to_pk matches pk#{i}"), &pk[..], &pc[..SIGN_PK]);

        // sk_to_curve25519
        let mut cc = out_buf(32);
        let mut cr = out_buf(32);
        unsafe {
            let rc = c_skc(cc.as_mut_ptr(), sk.as_ptr());
            let rr = r_skc(cr.as_mut_ptr(), sk.as_ptr());
            assert_eq!(rc, rr, "sk_to_curve25519 rc#{i}");
        }
        eqb(&format!("sk_to_curve25519#{i}"), &cc, &cr);

        // pk_to_curve25519 (valid pk)
        let mut kc = out_buf(32);
        let mut kr = out_buf(32);
        unsafe {
            let rc = c_pkc(kc.as_mut_ptr(), pk.as_ptr());
            let rr = r_pkc(kr.as_mut_ptr(), pk.as_ptr());
            assert_eq!(rc, rr, "pk_to_curve25519 valid rc#{i}");
            assert_eq!(rc, 0, "pk_to_curve25519 valid must succeed#{i}");
        }
        eqb(&format!("pk_to_curve25519 valid#{i}"), &kc, &kr);
    }

    // pk_to_curve25519 rejection cases: small-order pk, non-canonical pk, and
    // pk off the main subgroup.
    for (i, sop) in SMALL_ORDER_POINTS.iter().enumerate() {
        let mut kc = out_buf(32);
        let mut kr = out_buf(32);
        unsafe {
            let rc = c_pkc(kc.as_mut_ptr(), sop.as_ptr());
            let rr = r_pkc(kr.as_mut_ptr(), sop.as_ptr());
            assert_eq!(rc, rr, "pk_to_curve25519 small-order[{i}] rc");
        }
        eqb(&format!("pk_to_curve25519 small-order[{i}] out"), &kc, &kr);
    }
    for k in 0u8..8 {
        let mut ncpk = vec![0xffu8; SIGN_PK];
        ncpk[0] = 0xed_u8.wrapping_add(k);
        ncpk[31] = 0xff;
        let mut kc = out_buf(32);
        let mut kr = out_buf(32);
        unsafe {
            let rc = c_pkc(kc.as_mut_ptr(), ncpk.as_ptr());
            let rr = r_pkc(kr.as_mut_ptr(), ncpk.as_ptr());
            assert_eq!(rc, rr, "pk_to_curve25519 non-canonical[{k}] rc");
        }
        eqb(&format!("pk_to_curve25519 non-canonical[{k}] out"), &kc, &kr);
    }
    // pk off the main subgroup: valid curve points that are not in the prime
    // subgroup. Take a valid pk and add a small-order component is non-trivial
    // to construct by hand; instead feed several random 32-byte encodings that
    // decode but are not on the main subgroup. Whether each decodes is
    // C-vs-Rust identical; the C is ground truth.
    let mut rng2 = Rng::new(SEED ^ 0x66);
    for k in 0u32..64 {
        let mut pk = rng2.bytes(SIGN_PK);
        pk[31] &= 0x7f; // keep the sign bit clear-ish; still may be non-canonical
        let mut kc = out_buf(32);
        let mut kr = out_buf(32);
        unsafe {
            let rc = c_pkc(kc.as_mut_ptr(), pk.as_ptr());
            let rr = r_pkc(kr.as_mut_ptr(), pk.as_ptr());
            assert_eq!(rc, rr, "pk_to_curve25519 random[{k}] rc pk={}", hex(&pk));
        }
        eqb(&format!("pk_to_curve25519 random[{k}] out"), &kc, &kr);
    }
}

// ===========================================================================
// 7. crypto_box — the three families
// ===========================================================================

/// Capability table for one box family. Fields hold the fully-qualified export
/// name of each entry point, or "" when the family does not expose it.
struct BoxFam {
    pfx: &'static str,
    has_easy: bool,      // _easy / _open_easy / _easy_afternm / _open_easy_afternm
    has_detached: bool,  // _detached / _open_detached / *_afternm
    has_raw: bool,       // bare crypto_box_* / _open / _afternm / _open_afternm (ZEROBYTES)
    has_seal: bool,      // _seal / _seal_open
}

const BOX_FAMS: &[BoxFam] = &[
    // generic facade: everything
    BoxFam { pfx: "crypto_box", has_easy: true, has_detached: true, has_raw: true, has_seal: true },
    // xsalsa primitive: raw ZEROBYTES form + afternm, no easy/detached/seal
    BoxFam {
        pfx: "crypto_box_curve25519xsalsa20poly1305",
        has_easy: false,
        has_detached: false,
        has_raw: true,
        has_seal: false,
    },
    // xchacha primitive: easy/detached/seal + afternm, no raw ZEROBYTES form
    BoxFam {
        pfx: "crypto_box_curve25519xchacha20poly1305",
        has_easy: true,
        has_detached: true,
        has_raw: false,
        has_seal: true,
    },
];

fn box_mlens() -> Vec<usize> {
    vec![0, 1, 31, 32, 33, 63, 64, 65, 1000]
}

/// seed_keypair (byte-exact) + keypair (cross-check) for every family.
#[test]
fn box_keypairs() {
    let mut rng = Rng::new(SEED ^ 7);
    for fam in BOX_FAMS {
        let skp_n = format!("{}_seed_keypair", fam.pfx);
        let kp_n = format!("{}_keypair", fam.pfx);
        let (csk, rsk) = sym::<SeedKp>(&skp_n);
        // byte-exact seed_keypair
        let mut seeds: Vec<Vec<u8>> = (0..48).map(|_| rng.bytes(BOX_SEED)).collect();
        seeds.push(vec![0u8; BOX_SEED]);
        seeds.push(vec![0xffu8; BOX_SEED]);
        for (i, seed) in seeds.iter().enumerate() {
            let mut pkc = out_buf(BOX_PK);
            let mut pkr = out_buf(BOX_PK);
            let mut skc = out_buf(BOX_SK);
            let mut skr = out_buf(BOX_SK);
            unsafe {
                let rc = csk(pkc.as_mut_ptr(), skc.as_mut_ptr(), seed.as_ptr());
                let rr = rsk(pkr.as_mut_ptr(), skr.as_mut_ptr(), seed.as_ptr());
                assert_eq!(rc, rr, "{skp_n} rc seed#{i}");
                assert_eq!(rc, 0);
            }
            eqb(&format!("{skp_n} pk seed#{i}"), &pkc, &pkr);
            eqb(&format!("{skp_n} sk seed#{i}"), &skc, &skr);
        }
        // keypair: cross-check via beforenm (see box_beforenm_afternm) — here
        // just check rc + canary; the keys themselves are used below.
        let (ckp, rkp) = sym::<Kp>(&kp_n);
        for i in 0..8 {
            let mut pkc = out_buf(BOX_PK);
            let mut skc = out_buf(BOX_SK);
            let mut pkr = out_buf(BOX_PK);
            let mut skr = out_buf(BOX_SK);
            unsafe {
                let rc = ckp(pkc.as_mut_ptr(), skc.as_mut_ptr());
                let rr = rkp(pkr.as_mut_ptr(), skr.as_mut_ptr());
                assert_eq!(rc, rr, "{kp_n} rc#{i}");
                assert_eq!(rc, 0);
            }
            eqb(&format!("{kp_n} pk canary#{i}"), &pkc[BOX_PK..], &pkr[BOX_PK..]);
            eqb(&format!("{kp_n} sk canary#{i}"), &skc[BOX_SK..], &skr[BOX_SK..]);
        }
    }
}

/// beforenm (byte-exact shared key) + afternm/open_afternm forms.
#[test]
fn box_beforenm_and_afternm() {
    let mut rng = Rng::new(SEED ^ 8);
    for fam in BOX_FAMS {
        let bn_n = format!("{}_beforenm", fam.pfx);
        let (cbn, rbn) = sym::<Beforenm>(&bn_n);
        let (cskp, _) = sym::<SeedKp>(&format!("{}_seed_keypair", fam.pfx));

        for mlen in box_mlens() {
            // deterministic keypairs from fixed seeds
            let s1 = rng.bytes(BOX_SEED);
            let s2 = rng.bytes(BOX_SEED);
            let mut pk1 = vec![0u8; BOX_PK];
            let mut sk1 = vec![0u8; BOX_SK];
            let mut pk2 = vec![0u8; BOX_PK];
            let mut sk2 = vec![0u8; BOX_SK];
            unsafe {
                cskp(pk1.as_mut_ptr(), sk1.as_mut_ptr(), s1.as_ptr());
                cskp(pk2.as_mut_ptr(), sk2.as_mut_ptr(), s2.as_ptr());
            }
            let n = rng.bytes(BOX_NONCE);
            let m = rng.bytes(mlen);

            // beforenm: byte-exact
            let mut kc = out_buf(BOX_BEFORENM);
            let mut kr = out_buf(BOX_BEFORENM);
            unsafe {
                let rc = cbn(kc.as_mut_ptr(), pk2.as_ptr(), sk1.as_ptr());
                let rr = rbn(kr.as_mut_ptr(), pk2.as_ptr(), sk1.as_ptr());
                assert_eq!(rc, rr, "{bn_n} rc mlen={mlen}");
                assert_eq!(rc, 0);
            }
            eqb(&format!("{bn_n} shared-key mlen={mlen}"), &kc, &kr);
            let k = kc[..BOX_BEFORENM].to_vec();

            // ---- easy_afternm / open_easy_afternm ----
            if fam.has_easy {
                let ea_n = format!("{}_easy_afternm", fam.pfx);
                let oea_n = format!("{}_open_easy_afternm", fam.pfx);
                let (cea, rea) = sym::<EasyAfternm>(&ea_n);
                let (coea, roea) = sym::<EasyAfternm>(&oea_n);
                let clen = mlen + BOX_MAC;
                let mut cc = out_buf(clen);
                let mut cr = out_buf(clen);
                unsafe {
                    let rc = cea(cc.as_mut_ptr(), m.as_ptr(), mlen as u64, n.as_ptr(), k.as_ptr());
                    let rr = rea(cr.as_mut_ptr(), m.as_ptr(), mlen as u64, n.as_ptr(), k.as_ptr());
                    assert_eq!(rc, rr, "{ea_n} rc mlen={mlen}");
                }
                eqb(&format!("{ea_n} mlen={mlen}"), &cc, &cr);
                let mut pc = out_buf(mlen.max(1));
                let mut pr = out_buf(mlen.max(1));
                unsafe {
                    let rc = coea(pc.as_mut_ptr(), cc.as_ptr(), clen as u64, n.as_ptr(), k.as_ptr());
                    let rr = roea(pr.as_mut_ptr(), cr.as_ptr(), clen as u64, n.as_ptr(), k.as_ptr());
                    assert_eq!(rc, rr, "{oea_n} rc mlen={mlen}");
                    assert_eq!(rc, 0, "{oea_n} must succeed mlen={mlen}");
                }
                eqb(&format!("{oea_n} mlen={mlen}"), &pc, &pr);
                eqb(&format!("{oea_n} roundtrip mlen={mlen}"), &m, &pc[..mlen]);
                // truncated clen up to MAC+2
                for shortlen in 0..=(clen.min(BOX_MAC + 2)) {
                    let mut pc = out_buf(mlen.max(1));
                    let mut pr = out_buf(mlen.max(1));
                    unsafe {
                        let rc = coea(pc.as_mut_ptr(), cc.as_ptr(), shortlen as u64, n.as_ptr(), k.as_ptr());
                        let rr = roea(pr.as_mut_ptr(), cr.as_ptr(), shortlen as u64, n.as_ptr(), k.as_ptr());
                        assert_eq!(rc, rr, "{oea_n} short={shortlen} mlen={mlen}");
                    }
                    eqb(&format!("{oea_n} short={shortlen} mlen={mlen}"), &pc, &pr);
                }
            }

            // ---- detached_afternm / open_detached_afternm ----
            if fam.has_detached {
                let da_n = format!("{}_detached_afternm", fam.pfx);
                let oda_n = format!("{}_open_detached_afternm", fam.pfx);
                let (cda, rda) = sym::<DetAfternm>(&da_n);
                let (coda, roda) = sym::<OpenDetAfternm>(&oda_n);
                let mut cc = out_buf(mlen);
                let mut cr = out_buf(mlen);
                let mut macc = out_buf(BOX_MAC);
                let mut macr = out_buf(BOX_MAC);
                unsafe {
                    let rc = cda(cc.as_mut_ptr(), macc.as_mut_ptr(), m.as_ptr(), mlen as u64, n.as_ptr(), k.as_ptr());
                    let rr = rda(cr.as_mut_ptr(), macr.as_mut_ptr(), m.as_ptr(), mlen as u64, n.as_ptr(), k.as_ptr());
                    assert_eq!(rc, rr, "{da_n} rc mlen={mlen}");
                }
                eqb(&format!("{da_n} c mlen={mlen}"), &cc, &cr);
                eqb(&format!("{da_n} mac mlen={mlen}"), &macc, &macr);
                let mut pc = out_buf(mlen.max(1));
                let mut pr = out_buf(mlen.max(1));
                unsafe {
                    let rc = coda(pc.as_mut_ptr(), cc.as_ptr(), macc.as_ptr(), mlen as u64, n.as_ptr(), k.as_ptr());
                    let rr = roda(pr.as_mut_ptr(), cr.as_ptr(), macr.as_ptr(), mlen as u64, n.as_ptr(), k.as_ptr());
                    assert_eq!(rc, rr, "{oda_n} rc mlen={mlen}");
                    assert_eq!(rc, 0);
                }
                eqb(&format!("{oda_n} mlen={mlen}"), &pc, &pr);
                // tampered mac
                let mut badmac = macc[..BOX_MAC].to_vec();
                badmac[rng.below(BOX_MAC)] ^= 0x40;
                let mut pc = out_buf(mlen.max(1));
                let mut pr = out_buf(mlen.max(1));
                unsafe {
                    let rc = coda(pc.as_mut_ptr(), cc.as_ptr(), badmac.as_ptr(), mlen as u64, n.as_ptr(), k.as_ptr());
                    let rr = roda(pr.as_mut_ptr(), cr.as_ptr(), badmac.as_ptr(), mlen as u64, n.as_ptr(), k.as_ptr());
                    assert_eq!(rc, rr, "{oda_n} bad mac mlen={mlen}");
                    assert_eq!(rc, -1);
                }
                eqb(&format!("{oda_n} bad mac mlen={mlen}"), &pc, &pr);
            }

            // ---- raw ZEROBYTES afternm / open_afternm ----
            if fam.has_raw {
                let af_n = format!("{}_afternm", fam.pfx);
                let oaf_n = format!("{}_open_afternm", fam.pfx);
                let (caf, raf) = sym::<Afternm>(&af_n);
                let (coaf, roaf) = sym::<Afternm>(&oaf_n);
                // padded length: ZEROBYTES leading zeros + payload
                let plen = BOX_ZEROBYTES + mlen;
                let mut pm = vec![0u8; plen];
                pm[BOX_ZEROBYTES..].copy_from_slice(&m);
                let mut cc = out_buf(plen);
                let mut cr = out_buf(plen);
                unsafe {
                    let rc = caf(cc.as_mut_ptr(), pm.as_ptr(), plen as u64, n.as_ptr(), k.as_ptr());
                    let rr = raf(cr.as_mut_ptr(), pm.as_ptr(), plen as u64, n.as_ptr(), k.as_ptr());
                    assert_eq!(rc, rr, "{af_n} rc mlen={mlen}");
                }
                eqb(&format!("{af_n} mlen={mlen}"), &cc, &cr);
                let mut pc = out_buf(plen);
                let mut pr = out_buf(plen);
                unsafe {
                    let rc = coaf(pc.as_mut_ptr(), cc.as_ptr(), plen as u64, n.as_ptr(), k.as_ptr());
                    let rr = roaf(pr.as_mut_ptr(), cr.as_ptr(), plen as u64, n.as_ptr(), k.as_ptr());
                    assert_eq!(rc, rr, "{oaf_n} rc mlen={mlen}");
                    assert_eq!(rc, 0);
                }
                eqb(&format!("{oaf_n} mlen={mlen}"), &pc, &pr);
                eqb(&format!("{oaf_n} roundtrip mlen={mlen}"), &m, &pc[BOX_ZEROBYTES..plen]);
            }
        }
        // beforenm with an all-zero peer public key (scalarmult -> all-zero
        // shared secret; C does NOT reject in beforenm — hsalsa of zero — so
        // both must agree on whatever it returns).
        let mut kc = out_buf(BOX_BEFORENM);
        let mut kr = out_buf(BOX_BEFORENM);
        let sk = rng.bytes(BOX_SK);
        let zpk = vec![0u8; BOX_PK];
        unsafe {
            let rc = cbn(kc.as_mut_ptr(), zpk.as_ptr(), sk.as_ptr());
            let rr = rbn(kr.as_mut_ptr(), zpk.as_ptr(), sk.as_ptr());
            assert_eq!(rc, rr, "{bn_n} zero-peer rc");
        }
        eqb(&format!("{bn_n} zero-peer out"), &kc, &kr);
    }
}

/// The end-to-end key-based forms: _easy/_open_easy, _detached/_open_detached,
/// the raw crypto_box/crypto_box_open ZEROBYTES form, with tamper/short/wrong
/// key/wrong nonce cases.
#[test]
fn box_easy_detached_raw() {
    let mut rng = Rng::new(SEED ^ 9);
    for fam in BOX_FAMS {
        let (cskp, _) = sym::<SeedKp>(&format!("{}_seed_keypair", fam.pfx));
        for mlen in box_mlens() {
            let s1 = rng.bytes(BOX_SEED);
            let s2 = rng.bytes(BOX_SEED);
            let mut pk1 = vec![0u8; BOX_PK];
            let mut sk1 = vec![0u8; BOX_SK];
            let mut pk2 = vec![0u8; BOX_PK];
            let mut sk2 = vec![0u8; BOX_SK];
            unsafe {
                cskp(pk1.as_mut_ptr(), sk1.as_mut_ptr(), s1.as_ptr());
                cskp(pk2.as_mut_ptr(), sk2.as_mut_ptr(), s2.as_ptr());
            }
            let n = rng.bytes(BOX_NONCE);
            let m = rng.bytes(mlen);

            // ---- easy / open_easy ----
            if fam.has_easy {
                let e_n = format!("{}_easy", fam.pfx);
                let oe_n = format!("{}_open_easy", fam.pfx);
                let (ce, re) = sym::<BoxFull>(&e_n);
                let (coe, roe) = sym::<BoxFull>(&oe_n);
                let clen = mlen + BOX_MAC;
                let mut cc = out_buf(clen);
                let mut cr = out_buf(clen);
                unsafe {
                    let rc = ce(cc.as_mut_ptr(), m.as_ptr(), mlen as u64, n.as_ptr(), pk2.as_ptr(), sk1.as_ptr());
                    let rr = re(cr.as_mut_ptr(), m.as_ptr(), mlen as u64, n.as_ptr(), pk2.as_ptr(), sk1.as_ptr());
                    assert_eq!(rc, rr, "{e_n} rc mlen={mlen}");
                }
                eqb(&format!("{e_n} mlen={mlen}"), &cc, &cr);
                // open with recipient's sk2 + sender's pk1
                let mut pc = out_buf(mlen.max(1));
                let mut pr = out_buf(mlen.max(1));
                unsafe {
                    let rc = coe(pc.as_mut_ptr(), cc.as_ptr(), clen as u64, n.as_ptr(), pk1.as_ptr(), sk2.as_ptr());
                    let rr = roe(pr.as_mut_ptr(), cr.as_ptr(), clen as u64, n.as_ptr(), pk1.as_ptr(), sk2.as_ptr());
                    assert_eq!(rc, rr, "{oe_n} rc mlen={mlen}");
                    assert_eq!(rc, 0, "{oe_n} must succeed mlen={mlen}");
                }
                eqb(&format!("{oe_n} mlen={mlen}"), &pc, &pr);
                eqb(&format!("{oe_n} roundtrip mlen={mlen}"), &m, &pc[..mlen]);
                // truncated ciphertext up to MAC+2
                for shortlen in 0..=(clen.min(BOX_MAC + 2)) {
                    let mut pc = out_buf(mlen.max(1));
                    let mut pr = out_buf(mlen.max(1));
                    unsafe {
                        let rc = coe(pc.as_mut_ptr(), cc.as_ptr(), shortlen as u64, n.as_ptr(), pk1.as_ptr(), sk2.as_ptr());
                        let rr = roe(pr.as_mut_ptr(), cr.as_ptr(), shortlen as u64, n.as_ptr(), pk1.as_ptr(), sk2.as_ptr());
                        assert_eq!(rc, rr, "{oe_n} short={shortlen} mlen={mlen}");
                    }
                    eqb(&format!("{oe_n} short={shortlen} mlen={mlen}"), &pc, &pr);
                }
                // tamper MAC / body, wrong key, wrong nonce
                for pos in [0usize, BOX_MAC - 1, BOX_MAC, clen - 1] {
                    if pos >= clen {
                        continue;
                    }
                    let mut badc = cc[..clen].to_vec();
                    let mut badr = cr[..clen].to_vec();
                    badc[pos] ^= 1;
                    badr[pos] ^= 1;
                    let mut pc = out_buf(mlen.max(1));
                    let mut pr = out_buf(mlen.max(1));
                    unsafe {
                        let rc = coe(pc.as_mut_ptr(), badc.as_ptr(), clen as u64, n.as_ptr(), pk1.as_ptr(), sk2.as_ptr());
                        let rr = roe(pr.as_mut_ptr(), badr.as_ptr(), clen as u64, n.as_ptr(), pk1.as_ptr(), sk2.as_ptr());
                        assert_eq!(rc, rr, "{oe_n} tamper@{pos} mlen={mlen}");
                        assert_eq!(rc, -1);
                    }
                    eqb(&format!("{oe_n} tamper@{pos} mlen={mlen}"), &pc, &pr);
                }
                let wrong_pk = rng.bytes(BOX_PK);
                let n2 = rng.bytes(BOX_NONCE);
                for (tag, pkp, np) in
                    [("wrong-key", wrong_pk.as_ptr(), n.as_ptr()), ("wrong-nonce", pk1.as_ptr(), n2.as_ptr())]
                {
                    let mut pc = out_buf(mlen.max(1));
                    let mut pr = out_buf(mlen.max(1));
                    unsafe {
                        let rc = coe(pc.as_mut_ptr(), cc.as_ptr(), clen as u64, np, pkp, sk2.as_ptr());
                        let rr = roe(pr.as_mut_ptr(), cr.as_ptr(), clen as u64, np, pkp, sk2.as_ptr());
                        assert_eq!(rc, rr, "{oe_n} {tag} mlen={mlen}");
                    }
                    eqb(&format!("{oe_n} {tag} mlen={mlen}"), &pc, &pr);
                }
            }

            // ---- detached / open_detached ----
            if fam.has_detached {
                let d_n = format!("{}_detached", fam.pfx);
                let od_n = format!("{}_open_detached", fam.pfx);
                let (cd, rd) = sym::<DetFull>(&d_n);
                let (cod, rod) = sym::<OpenDetFull>(&od_n);
                let mut cc = out_buf(mlen);
                let mut cr = out_buf(mlen);
                let mut macc = out_buf(BOX_MAC);
                let mut macr = out_buf(BOX_MAC);
                unsafe {
                    let rc = cd(cc.as_mut_ptr(), macc.as_mut_ptr(), m.as_ptr(), mlen as u64, n.as_ptr(), pk2.as_ptr(), sk1.as_ptr());
                    let rr = rd(cr.as_mut_ptr(), macr.as_mut_ptr(), m.as_ptr(), mlen as u64, n.as_ptr(), pk2.as_ptr(), sk1.as_ptr());
                    assert_eq!(rc, rr, "{d_n} rc mlen={mlen}");
                }
                eqb(&format!("{d_n} c mlen={mlen}"), &cc, &cr);
                eqb(&format!("{d_n} mac mlen={mlen}"), &macc, &macr);
                let mut pc = out_buf(mlen.max(1));
                let mut pr = out_buf(mlen.max(1));
                unsafe {
                    let rc = cod(pc.as_mut_ptr(), cc.as_ptr(), macc.as_ptr(), mlen as u64, n.as_ptr(), pk1.as_ptr(), sk2.as_ptr());
                    let rr = rod(pr.as_mut_ptr(), cr.as_ptr(), macr.as_ptr(), mlen as u64, n.as_ptr(), pk1.as_ptr(), sk2.as_ptr());
                    assert_eq!(rc, rr, "{od_n} rc mlen={mlen}");
                    assert_eq!(rc, 0);
                }
                eqb(&format!("{od_n} mlen={mlen}"), &pc, &pr);
                eqb(&format!("{od_n} roundtrip mlen={mlen}"), &m, &pc[..mlen]);
                // tampered mac / wrong key
                let mut badmac = macc[..BOX_MAC].to_vec();
                badmac[rng.below(BOX_MAC)] ^= 0x11;
                let mut pc = out_buf(mlen.max(1));
                let mut pr = out_buf(mlen.max(1));
                unsafe {
                    let rc = cod(pc.as_mut_ptr(), cc.as_ptr(), badmac.as_ptr(), mlen as u64, n.as_ptr(), pk1.as_ptr(), sk2.as_ptr());
                    let rr = rod(pr.as_mut_ptr(), cr.as_ptr(), badmac.as_ptr(), mlen as u64, n.as_ptr(), pk1.as_ptr(), sk2.as_ptr());
                    assert_eq!(rc, rr, "{od_n} bad mac mlen={mlen}");
                    assert_eq!(rc, -1);
                }
                eqb(&format!("{od_n} bad mac mlen={mlen}"), &pc, &pr);
            }

            // ---- raw ZEROBYTES crypto_box / crypto_box_open ----
            if fam.has_raw {
                let b_n = fam.pfx.to_string();
                let bo_n = format!("{}_open", fam.pfx);
                let (cb, rb) = sym::<BoxFull>(&b_n);
                let (cbo, rbo) = sym::<BoxFull>(&bo_n);
                let plen = BOX_ZEROBYTES + mlen;
                let mut pm = vec![0u8; plen];
                pm[BOX_ZEROBYTES..].copy_from_slice(&m);
                let mut cc = out_buf(plen);
                let mut cr = out_buf(plen);
                unsafe {
                    let rc = cb(cc.as_mut_ptr(), pm.as_ptr(), plen as u64, n.as_ptr(), pk2.as_ptr(), sk1.as_ptr());
                    let rr = rb(cr.as_mut_ptr(), pm.as_ptr(), plen as u64, n.as_ptr(), pk2.as_ptr(), sk1.as_ptr());
                    assert_eq!(rc, rr, "{b_n} raw rc mlen={mlen}");
                }
                eqb(&format!("{b_n} raw mlen={mlen}"), &cc, &cr);
                let mut pc = out_buf(plen);
                let mut pr = out_buf(plen);
                unsafe {
                    let rc = cbo(pc.as_mut_ptr(), cc.as_ptr(), plen as u64, n.as_ptr(), pk1.as_ptr(), sk2.as_ptr());
                    let rr = rbo(pr.as_mut_ptr(), cr.as_ptr(), plen as u64, n.as_ptr(), pk1.as_ptr(), sk2.as_ptr());
                    assert_eq!(rc, rr, "{bo_n} raw rc mlen={mlen}");
                    assert_eq!(rc, 0);
                }
                eqb(&format!("{bo_n} raw mlen={mlen}"), &pc, &pr);
                eqb(&format!("{bo_n} raw roundtrip mlen={mlen}"), &m, &pc[BOX_ZEROBYTES..plen]);
                // corrupted box body
                if mlen > 0 {
                    let mut badc = cc[..plen].to_vec();
                    badc[BOX_BOXZEROBYTES] ^= 1;
                    let mut pc = out_buf(plen);
                    let mut pr = out_buf(plen);
                    unsafe {
                        let rc = cbo(pc.as_mut_ptr(), badc.as_ptr(), plen as u64, n.as_ptr(), pk1.as_ptr(), sk2.as_ptr());
                        let rr = rbo(pr.as_mut_ptr(), badc.as_ptr(), plen as u64, n.as_ptr(), pk1.as_ptr(), sk2.as_ptr());
                        assert_eq!(rc, rr, "{bo_n} raw corrupt rc mlen={mlen}");
                    }
                    eqb(&format!("{bo_n} raw corrupt mlen={mlen}"), &pc, &pr);
                }
            }
        }
    }
}

/// seal (cross-check) + seal_open (byte-exact given a C-produced sealed box,
/// in both directions).
#[test]
fn box_seal_and_seal_open() {
    let mut rng = Rng::new(SEED ^ 10);
    for fam in BOX_FAMS {
        if !fam.has_seal {
            continue;
        }
        let seal_n = format!("{}_seal", fam.pfx);
        let so_n = format!("{}_seal_open", fam.pfx);
        let (cseal, rseal) = sym::<SealFn>(&seal_n);
        let (cso, rso) = sym::<SealOpenFn>(&so_n);
        let (cskp, _) = sym::<SeedKp>(&format!("{}_seed_keypair", fam.pfx));

        for mlen in box_mlens() {
            let seed = rng.bytes(BOX_SEED);
            let mut pk = vec![0u8; BOX_PK];
            let mut sk = vec![0u8; BOX_SK];
            unsafe {
                cskp(pk.as_mut_ptr(), sk.as_mut_ptr(), seed.as_ptr());
            }
            let m = rng.bytes(mlen);
            let clen = mlen + BOX_SEALBYTES;

            // seal is non-deterministic (ephemeral CSPRNG keypair): compare rc
            // and canary only, then CROSS-CHECK by opening in both libs.
            let mut cc = out_buf(clen);
            let mut cr = out_buf(clen);
            unsafe {
                let rc = cseal(cc.as_mut_ptr(), m.as_ptr(), mlen as u64, pk.as_ptr());
                let rr = rseal(cr.as_mut_ptr(), m.as_ptr(), mlen as u64, pk.as_ptr());
                assert_eq!(rc, rr, "{seal_n} rc mlen={mlen}");
                assert_eq!(rc, 0);
            }
            eqb(&format!("{seal_n} canary mlen={mlen}"), &cc[clen..], &cr[clen..]);

            // seal_open must recover m byte-for-byte in BOTH libs, from EITHER
            // library's sealed box (cross-check both directions).
            for (tag, sealed) in [("C-sealed", &cc), ("Rust-sealed", &cr)] {
                let mut pc = out_buf(mlen.max(1));
                let mut pr = out_buf(mlen.max(1));
                unsafe {
                    let rc = cso(pc.as_mut_ptr(), sealed.as_ptr(), clen as u64, pk.as_ptr(), sk.as_ptr());
                    let rr = rso(pr.as_mut_ptr(), sealed.as_ptr(), clen as u64, pk.as_ptr(), sk.as_ptr());
                    assert_eq!(rc, rr, "{so_n} {tag} rc mlen={mlen}");
                    assert_eq!(rc, 0, "{so_n} {tag} must open mlen={mlen}");
                }
                eqb(&format!("{so_n} {tag} out mlen={mlen}"), &pc, &pr);
                eqb(&format!("{so_n} {tag} roundtrip mlen={mlen}"), &m, &pc[..mlen]);
            }

            // truncated sealed box up to SEALBYTES+2
            for shortlen in 0..=(clen.min(BOX_SEALBYTES + 2)) {
                let mut pc = out_buf(mlen.max(1));
                let mut pr = out_buf(mlen.max(1));
                unsafe {
                    let rc = cso(pc.as_mut_ptr(), cc.as_ptr(), shortlen as u64, pk.as_ptr(), sk.as_ptr());
                    let rr = rso(pr.as_mut_ptr(), cr.as_ptr(), shortlen as u64, pk.as_ptr(), sk.as_ptr());
                    assert_eq!(rc, rr, "{so_n} short={shortlen} mlen={mlen}");
                }
                eqb(&format!("{so_n} short={shortlen} mlen={mlen}"), &pc, &pr);
            }
            // tampered ephemeral pk / body / mac, wrong sk
            for pos in [0usize, BOX_PK - 1, BOX_PK, clen - 1] {
                if pos >= clen {
                    continue;
                }
                let mut bad = cc[..clen].to_vec();
                bad[pos] ^= 1;
                let mut pc = out_buf(mlen.max(1));
                let mut pr = out_buf(mlen.max(1));
                unsafe {
                    let rc = cso(pc.as_mut_ptr(), bad.as_ptr(), clen as u64, pk.as_ptr(), sk.as_ptr());
                    let rr = rso(pr.as_mut_ptr(), bad.as_ptr(), clen as u64, pk.as_ptr(), sk.as_ptr());
                    assert_eq!(rc, rr, "{so_n} tamper@{pos} mlen={mlen}");
                    assert_eq!(rc, -1);
                }
                eqb(&format!("{so_n} tamper@{pos} mlen={mlen}"), &pc, &pr);
            }
            let wrong_seed = rng.bytes(BOX_SEED);
            let mut wpk = vec![0u8; BOX_PK];
            let mut wsk = vec![0u8; BOX_SK];
            unsafe {
                cskp(wpk.as_mut_ptr(), wsk.as_mut_ptr(), wrong_seed.as_ptr());
            }
            let mut pc = out_buf(mlen.max(1));
            let mut pr = out_buf(mlen.max(1));
            unsafe {
                let rc = cso(pc.as_mut_ptr(), cc.as_ptr(), clen as u64, wpk.as_ptr(), wsk.as_ptr());
                let rr = rso(pr.as_mut_ptr(), cr.as_ptr(), clen as u64, wpk.as_ptr(), wsk.as_ptr());
                assert_eq!(rc, rr, "{so_n} wrong-key mlen={mlen}");
                assert_eq!(rc, -1);
            }
            eqb(&format!("{so_n} wrong-key mlen={mlen}"), &pc, &pr);
        }
    }
}

/// The `_easy` / `_easy_afternm` / `_seal` paths call `sodium_misuse()` when
/// `mlen > MESSAGEBYTES_MAX`. Route through `same_outcome`.
#[test]
fn box_oversized_aborts_identically() {
    for fam in BOX_FAMS {
        let maxn = format!("{}_messagebytes_max", fam.pfx);
        let cv = size_of(&maxn);
        if cv == usize::MAX {
            continue;
        }
        let over = (cv as u64).wrapping_add(1);

        if fam.has_easy {
            let e_n = format!("{}_easy", fam.pfx);
            let a = e_n.clone();
            let b = e_n.clone();
            same_outcome(
                &format!("{e_n} mlen=MAX+1"),
                move || {
                    let (c, _) = sym::<BoxFull>(&a);
                    let pk = vec![0u8; BOX_PK];
                    let sk = vec![0u8; BOX_SK];
                    let n = vec![0u8; BOX_NONCE];
                    let m = vec![0u8; 64];
                    let mut o = vec![0u8; 128];
                    unsafe { c(o.as_mut_ptr(), m.as_ptr(), over, n.as_ptr(), pk.as_ptr(), sk.as_ptr()) }
                },
                move || {
                    let (_, r) = sym::<BoxFull>(&b);
                    let pk = vec![0u8; BOX_PK];
                    let sk = vec![0u8; BOX_SK];
                    let n = vec![0u8; BOX_NONCE];
                    let m = vec![0u8; 64];
                    let mut o = vec![0u8; 128];
                    unsafe { r(o.as_mut_ptr(), m.as_ptr(), over, n.as_ptr(), pk.as_ptr(), sk.as_ptr()) }
                },
            );
            // easy_afternm too
            let ea_n = format!("{}_easy_afternm", fam.pfx);
            let a = ea_n.clone();
            let b = ea_n.clone();
            same_outcome(
                &format!("{ea_n} mlen=MAX+1"),
                move || {
                    let (c, _) = sym::<EasyAfternm>(&a);
                    let k = vec![0u8; BOX_BEFORENM];
                    let n = vec![0u8; BOX_NONCE];
                    let m = vec![0u8; 64];
                    let mut o = vec![0u8; 128];
                    unsafe { c(o.as_mut_ptr(), m.as_ptr(), over, n.as_ptr(), k.as_ptr()) }
                },
                move || {
                    let (_, r) = sym::<EasyAfternm>(&b);
                    let k = vec![0u8; BOX_BEFORENM];
                    let n = vec![0u8; BOX_NONCE];
                    let m = vec![0u8; 64];
                    let mut o = vec![0u8; 128];
                    unsafe { r(o.as_mut_ptr(), m.as_ptr(), over, n.as_ptr(), k.as_ptr()) }
                },
            );
        }
        if fam.has_seal {
            let s_n = format!("{}_seal", fam.pfx);
            let a = s_n.clone();
            let b = s_n.clone();
            same_outcome(
                &format!("{s_n} mlen=MAX+1"),
                move || {
                    let (c, _) = sym::<SealFn>(&a);
                    let pk = vec![0u8; BOX_PK];
                    let m = vec![0u8; 64];
                    let mut o = vec![0u8; 128];
                    unsafe { c(o.as_mut_ptr(), m.as_ptr(), over, pk.as_ptr()) }
                },
                move || {
                    let (_, r) = sym::<SealFn>(&b);
                    let pk = vec![0u8; BOX_PK];
                    let m = vec![0u8; 64];
                    let mut o = vec![0u8; 128];
                    unsafe { r(o.as_mut_ptr(), m.as_ptr(), over, pk.as_ptr()) }
                },
            );
        }
    }
}

// ===========================================================================
// 8. crypto_kx
// ===========================================================================

#[test]
fn kx_constants_and_seed_keypair() {
    for (name, want) in [
        ("crypto_kx_publickeybytes", KX_PK),
        ("crypto_kx_secretkeybytes", KX_SK),
        ("crypto_kx_seedbytes", KX_SEED),
        ("crypto_kx_sessionkeybytes", KX_SESSION),
    ] {
        assert_eq!(size_of(name), want, "{name}");
    }
    // primitive
    let (c, r) = sym::<PrimFn>("crypto_kx_primitive");
    unsafe {
        let cs = std::ffi::CStr::from_ptr(c());
        let rs = std::ffi::CStr::from_ptr(r());
        assert_eq!(cs, rs, "crypto_kx_primitive");
    }

    // seed_keypair: byte-exact for many seeds incl. extremes
    let (csk, rsk) = sym::<SeedKp>("crypto_kx_seed_keypair");
    let mut rng = Rng::new(SEED ^ 11);
    let mut seeds: Vec<Vec<u8>> = (0..64).map(|_| rng.bytes(KX_SEED)).collect();
    seeds.push(vec![0u8; KX_SEED]);
    seeds.push(vec![0xffu8; KX_SEED]);
    for (i, seed) in seeds.iter().enumerate() {
        let mut pkc = out_buf(KX_PK);
        let mut pkr = out_buf(KX_PK);
        let mut skc = out_buf(KX_SK);
        let mut skr = out_buf(KX_SK);
        unsafe {
            let rc = csk(pkc.as_mut_ptr(), skc.as_mut_ptr(), seed.as_ptr());
            let rr = rsk(pkr.as_mut_ptr(), skr.as_mut_ptr(), seed.as_ptr());
            assert_eq!(rc, rr, "crypto_kx_seed_keypair rc seed#{i}");
            assert_eq!(rc, 0);
        }
        eqb(&format!("crypto_kx_seed_keypair pk seed#{i}"), &pkc, &pkr);
        eqb(&format!("crypto_kx_seed_keypair sk seed#{i}"), &skc, &skr);
    }

    // keypair: cross-check only (CSPRNG). Use both keys through session_keys.
    let (ckp, rkp) = sym::<Kp>("crypto_kx_keypair");
    for i in 0..8 {
        let mut pkc = out_buf(KX_PK);
        let mut skc = out_buf(KX_SK);
        let mut pkr = out_buf(KX_PK);
        let mut skr = out_buf(KX_SK);
        unsafe {
            let rc = ckp(pkc.as_mut_ptr(), skc.as_mut_ptr());
            let rr = rkp(pkr.as_mut_ptr(), skr.as_mut_ptr());
            assert_eq!(rc, rr, "crypto_kx_keypair rc#{i}");
            assert_eq!(rc, 0);
        }
        eqb(&format!("crypto_kx_keypair pk canary#{i}"), &pkc[KX_PK..], &pkr[KX_PK..]);
        eqb(&format!("crypto_kx_keypair sk canary#{i}"), &skc[KX_SK..], &skr[KX_SK..]);
    }
}

#[test]
fn kx_session_keys() {
    let mut rng = Rng::new(SEED ^ 12);
    let (csk, _) = sym::<SeedKp>("crypto_kx_seed_keypair");
    let (ccli, rcli) = sym::<KxSession>("crypto_kx_client_session_keys");
    let (csrv, rsrv) = sym::<KxSession>("crypto_kx_server_session_keys");

    for i in 0..48 {
        // deterministic client + server keypairs from fixed seeds
        let cs = rng.bytes(KX_SEED);
        let ss = rng.bytes(KX_SEED);
        let mut cpk = vec![0u8; KX_PK];
        let mut csk_ = vec![0u8; KX_SK];
        let mut spk = vec![0u8; KX_PK];
        let mut ssk = vec![0u8; KX_SK];
        unsafe {
            csk(cpk.as_mut_ptr(), csk_.as_mut_ptr(), cs.as_ptr());
            csk(spk.as_mut_ptr(), ssk.as_mut_ptr(), ss.as_ptr());
        }

        // both rx and tx non-NULL, client side
        let mut rxc = out_buf(KX_SESSION);
        let mut txc = out_buf(KX_SESSION);
        let mut rxr = out_buf(KX_SESSION);
        let mut txr = out_buf(KX_SESSION);
        unsafe {
            let rc = ccli(rxc.as_mut_ptr(), txc.as_mut_ptr(), cpk.as_ptr(), csk_.as_ptr(), spk.as_ptr());
            let rr = rcli(rxr.as_mut_ptr(), txr.as_mut_ptr(), cpk.as_ptr(), csk_.as_ptr(), spk.as_ptr());
            assert_eq!(rc, rr, "client rc#{i}");
            assert_eq!(rc, 0);
        }
        eqb(&format!("client rx#{i}"), &rxc, &rxr);
        eqb(&format!("client tx#{i}"), &txc, &txr);

        // server side
        let mut rxc2 = out_buf(KX_SESSION);
        let mut txc2 = out_buf(KX_SESSION);
        let mut rxr2 = out_buf(KX_SESSION);
        let mut txr2 = out_buf(KX_SESSION);
        unsafe {
            let rc = csrv(rxc2.as_mut_ptr(), txc2.as_mut_ptr(), spk.as_ptr(), ssk.as_ptr(), cpk.as_ptr());
            let rr = rsrv(rxr2.as_mut_ptr(), txr2.as_mut_ptr(), spk.as_ptr(), ssk.as_ptr(), cpk.as_ptr());
            assert_eq!(rc, rr, "server rc#{i}");
            assert_eq!(rc, 0);
        }
        eqb(&format!("server rx#{i}"), &rxc2, &rxr2);
        eqb(&format!("server tx#{i}"), &txc2, &txr2);
        // client.rx == server.tx and client.tx == server.rx (protocol invariant)
        eqb(&format!("client.rx==server.tx#{i}"), &rxc[..KX_SESSION], &txc2[..KX_SESSION]);
        eqb(&format!("client.tx==server.rx#{i}"), &txc[..KX_SESSION], &rxc2[..KX_SESSION]);

        // rx == NULL (only tx wanted). rx is aliased to tx internally.
        let mut txc3 = out_buf(KX_SESSION);
        let mut txr3 = out_buf(KX_SESSION);
        unsafe {
            let rc = ccli(ptr::null_mut(), txc3.as_mut_ptr(), cpk.as_ptr(), csk_.as_ptr(), spk.as_ptr());
            let rr = rcli(ptr::null_mut(), txr3.as_mut_ptr(), cpk.as_ptr(), csk_.as_ptr(), spk.as_ptr());
            assert_eq!(rc, rr, "client rx=NULL rc#{i}");
            assert_eq!(rc, 0);
        }
        eqb(&format!("client rx=NULL tx#{i}"), &txc3, &txr3);

        // tx == NULL (only rx wanted).
        let mut rxc4 = out_buf(KX_SESSION);
        let mut rxr4 = out_buf(KX_SESSION);
        unsafe {
            let rc = ccli(rxc4.as_mut_ptr(), ptr::null_mut(), cpk.as_ptr(), csk_.as_ptr(), spk.as_ptr());
            let rr = rcli(rxr4.as_mut_ptr(), ptr::null_mut(), cpk.as_ptr(), csk_.as_ptr(), spk.as_ptr());
            assert_eq!(rc, rr, "client tx=NULL rc#{i}");
            assert_eq!(rc, 0);
        }
        eqb(&format!("client tx=NULL rx#{i}"), &rxc4, &rxr4);

        // all-zero peer public key -> internal scalarmult fails -> return -1
        let zpk = vec![0u8; KX_PK];
        let mut rxc5 = out_buf(KX_SESSION);
        let mut txc5 = out_buf(KX_SESSION);
        let mut rxr5 = out_buf(KX_SESSION);
        let mut txr5 = out_buf(KX_SESSION);
        unsafe {
            let rc = ccli(rxc5.as_mut_ptr(), txc5.as_mut_ptr(), cpk.as_ptr(), csk_.as_ptr(), zpk.as_ptr());
            let rr = rcli(rxr5.as_mut_ptr(), txr5.as_mut_ptr(), cpk.as_ptr(), csk_.as_ptr(), zpk.as_ptr());
            assert_eq!(rc, rr, "client zero-peer rc#{i}");
            assert_eq!(rc, -1, "client zero-peer must fail#{i}");
        }
        unsafe {
            let rc = csrv(rxc5.as_mut_ptr(), txc5.as_mut_ptr(), spk.as_ptr(), ssk.as_ptr(), zpk.as_ptr());
            let rr = rsrv(rxr5.as_mut_ptr(), txr5.as_mut_ptr(), spk.as_ptr(), ssk.as_ptr(), zpk.as_ptr());
            assert_eq!(rc, rr, "server zero-peer rc#{i}");
            assert_eq!(rc, -1, "server zero-peer must fail#{i}");
        }
    }
}

/// BOTH rx and tx NULL calls `sodium_misuse()` -> abort. Route through
/// `same_outcome` (client and server).
#[test]
fn kx_both_null_aborts_identically() {
    let (csk, _) = sym::<SeedKp>("crypto_kx_seed_keypair");
    let seed = [7u8; KX_SEED];
    let mut pk = vec![0u8; KX_PK];
    let mut sk = vec![0u8; KX_SK];
    unsafe {
        csk(pk.as_mut_ptr(), sk.as_mut_ptr(), seed.as_ptr());
    }
    let pk2 = pk.clone();
    let sk2 = sk.clone();

    for name in ["crypto_kx_client_session_keys", "crypto_kx_server_session_keys"] {
        let a = name.to_string();
        let b = name.to_string();
        // Third arg is the local pk, fourth the local sk, fifth the peer pk.
        // With both rx/tx NULL the peer scalarmult is never reached: the misuse
        // check fires first. Provide well-formed keys anyway.
        let pkc = pk.clone();
        let skc = sk.clone();
        let peerc = pk2.clone();
        let pkr = pk.clone();
        let skr = sk.clone();
        let peerr = pk2.clone();
        let _ = &sk2;
        same_outcome(
            &format!("{name} both NULL"),
            move || {
                let (c, _) = sym::<KxSession>(&a);
                unsafe { c(ptr::null_mut(), ptr::null_mut(), pkc.as_ptr(), skc.as_ptr(), peerc.as_ptr()) }
            },
            move || {
                let (_, r) = sym::<KxSession>(&b);
                unsafe { r(ptr::null_mut(), ptr::null_mut(), pkr.as_ptr(), skr.as_ptr(), peerr.as_ptr()) }
            },
        );
    }
}
