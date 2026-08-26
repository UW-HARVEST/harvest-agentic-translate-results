//! Phase B — G5: `crypto_box`, `crypto_secretbox`, `crypto_secretstream`,
//! `crypto_kx`, `crypto_sign`, `crypto_auth` (valid-input rows of
//! `CONFIGS.md` section `## G5`).
//!
//! Every public entry point is reached through `dlsym` on both `.so`s, so the
//! `#[no_mangle]` wrappers are part of what is compared. All four `crypto_box`
//! API layers (NaCl padded / easy / detached / sealed) are exercised for both
//! primitives, and the opaque state buffers (which libsodium documents as
//! plain public structs) are compared byte-for-byte as well.

mod common;
use common::*;
use std::ptr;

// ---------------------------------------------------------------------------
// C signatures
// ---------------------------------------------------------------------------

type SizeFn = unsafe extern "C" fn() -> usize;
type ByteFn = unsafe extern "C" fn() -> u8;
type PrimFn = unsafe extern "C" fn() -> *const std::ffi::c_char;
type Keygen = unsafe extern "C" fn(*mut u8);
type Keypair = unsafe extern "C" fn(*mut u8, *mut u8) -> i32;
type SeedKeypair = unsafe extern "C" fn(*mut u8, *mut u8, *const u8) -> i32;
/// `beforenm(k, pk, sk)`, and every `*_to_*` conversion `(out, in)`.
type Two = unsafe extern "C" fn(*mut u8, *const u8) -> i32;
type Three = unsafe extern "C" fn(*mut u8, *const u8, *const u8) -> i32;

/// `(c, m, mlen, n, k)` — NaCl / easy / afternm forms keyed by a shared secret.
type Sym5 = unsafe extern "C" fn(*mut u8, *const u8, u64, *const u8, *const u8) -> i32;
/// `(c, m, mlen, n, pk, sk)` — NaCl / easy forms keyed by a key pair.
type Asym6 =
    unsafe extern "C" fn(*mut u8, *const u8, u64, *const u8, *const u8, *const u8) -> i32;
/// `(c, mac, m, mlen, n, k)`
type Det6 = unsafe extern "C" fn(*mut u8, *mut u8, *const u8, u64, *const u8, *const u8) -> i32;
/// `(c, mac, m, mlen, n, pk, sk)`
type Det7 = unsafe extern "C" fn(
    *mut u8,
    *mut u8,
    *const u8,
    u64,
    *const u8,
    *const u8,
    *const u8,
) -> i32;
/// `(m, c, mac, clen, n, k)`
type ODet6 = unsafe extern "C" fn(*mut u8, *const u8, *const u8, u64, *const u8, *const u8) -> i32;
/// `(m, c, mac, clen, n, pk, sk)`
type ODet7 = unsafe extern "C" fn(
    *mut u8,
    *const u8,
    *const u8,
    u64,
    *const u8,
    *const u8,
    *const u8,
) -> i32;
/// `(c, m, mlen, pk)`
type Seal = unsafe extern "C" fn(*mut u8, *const u8, u64, *const u8) -> i32;
/// `(m, c, clen, pk, sk)`
type SealOpen = unsafe extern "C" fn(*mut u8, *const u8, u64, *const u8, *const u8) -> i32;

type SsInit = unsafe extern "C" fn(*mut u8, *mut u8, *const u8) -> i32;
type SsInitPull = unsafe extern "C" fn(*mut u8, *const u8, *const u8) -> i32;
type SsPush =
    unsafe extern "C" fn(*mut u8, *mut u8, *mut u64, *const u8, u64, *const u8, u64, u8) -> i32;
type SsPull = unsafe extern "C" fn(
    *mut u8,
    *mut u8,
    *mut u64,
    *mut u8,
    *const u8,
    u64,
    *const u8,
    u64,
) -> i32;
type SsRekey = unsafe extern "C" fn(*mut u8);

type KxSession =
    unsafe extern "C" fn(*mut u8, *mut u8, *const u8, *const u8, *const u8) -> i32;

/// `crypto_sign(sm, smlen_p, m, mlen, sk)` and
/// `crypto_sign_open(m, mlen_p, sm, smlen, pk)` share this shape.
type Sign5 = unsafe extern "C" fn(*mut u8, *mut u64, *const u8, u64, *const u8) -> i32;
/// `crypto_sign_verify_detached(sig, m, mlen, pk)`
type Verify4 = unsafe extern "C" fn(*const u8, *const u8, u64, *const u8) -> i32;
type PhInit = unsafe extern "C" fn(*mut u8) -> i32;
type PhUpdate = unsafe extern "C" fn(*mut u8, *const u8, u64) -> i32;
type PhCreate = unsafe extern "C" fn(*mut u8, *mut u8, *mut u64, *const u8) -> i32;
type PhVerify = unsafe extern "C" fn(*mut u8, *const u8, *const u8) -> i32;

/// `crypto_auth*(out, in, inlen, k)` and `crypto_auth*_verify(h, in, inlen, k)`
type Auth4 = unsafe extern "C" fn(*mut u8, *const u8, u64, *const u8) -> i32;
type AuthV4 = unsafe extern "C" fn(*const u8, *const u8, u64, *const u8) -> i32;
type AuthInit = unsafe extern "C" fn(*mut u8, *const u8, usize) -> i32;
type AuthUpdate = unsafe extern "C" fn(*mut u8, *const u8, u64) -> i32;
type AuthFinal = unsafe extern "C" fn(*mut u8, *mut u8) -> i32;

// primitives borrowed (read-only) from other module groups, used to build
// independent reference values.
type Chacha = unsafe extern "C" fn(*mut u8, u64, *const u8, *const u8) -> i32;
type ChachaXorIc =
    unsafe extern "C" fn(*mut u8, *const u8, u64, *const u8, u32, *const u8) -> i32;
type P1305Init = unsafe extern "C" fn(*mut u8, *const u8) -> i32;
type P1305Update = unsafe extern "C" fn(*mut u8, *const u8, u64) -> i32;
type P1305Final = unsafe extern "C" fn(*mut u8, *mut u8) -> i32;

// ---------------------------------------------------------------------------
// helpers
// ---------------------------------------------------------------------------

/// Read a `size_t`-returning accessor from **both** libraries, assert equal,
/// return the value.
#[track_caller]
fn cst(name: &str) -> usize {
    let (a, b) = pair::<SizeFn>(name);
    let (x, y) = unsafe { (a(), b()) };
    eq_usize(name, x, y);
    x
}

#[track_caller]
fn cbyte(name: &str) -> u8 {
    let (a, b) = pair::<ByteFn>(name);
    let (x, y) = unsafe { (a(), b()) };
    assert_eq!(x, y, "{name}: C={x} Rust={y}");
    x
}

#[track_caller]
fn cprim(name: &str) -> String {
    let (a, b) = pair::<PrimFn>(name);
    let s = |p: *const std::ffi::c_char| unsafe {
        std::ffi::CStr::from_ptr(p).to_str().unwrap().to_string()
    };
    let (x, y) = unsafe { (s(a()), s(b())) };
    assert_eq!(x, y, "{name}");
    x
}

/// Message lengths used almost everywhere: block/pad boundaries of Salsa20,
/// ChaCha20, poly1305 and the `block0` payload cutoff (32).
const MLENS: &[usize] = &[0, 1, 15, 16, 17, 31, 32, 33, 63, 64, 65, 127, 128, 1000];

/// `reset_rngs()` mutates state shared by every test in this binary, so the
/// `reset; C-call; reset; Rust-call` sequences must not interleave across the
/// libtest thread pool.
static RNG_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// A deterministic X25519 key pair from a seed byte, via `_seed_keypair`
/// (so it does not consume the RNG).
fn box_kp(seedbyte: u8) -> (Vec<u8>, Vec<u8>) {
    let seed = vec![seedbyte; 32];
    let f = sym::<SeedKeypair>(c_lib(), "crypto_box_seed_keypair");
    let mut pk = vec![0u8; 32];
    let mut sk = vec![0u8; 32];
    unsafe { assert_eq!(f(pk.as_mut_ptr(), sk.as_mut_ptr(), seed.as_ptr()), 0) };
    (pk, sk)
}

fn box_kp_rng(rng: &mut Rng) -> (Vec<u8>, Vec<u8>) {
    let seed = rng.bytes(32);
    let f = sym::<SeedKeypair>(c_lib(), "crypto_box_seed_keypair");
    let mut pk = vec![0u8; 32];
    let mut sk = vec![0u8; 32];
    unsafe { assert_eq!(f(pk.as_mut_ptr(), sk.as_mut_ptr(), seed.as_ptr()), 0) };
    (pk, sk)
}

fn sign_kp(rng: &mut Rng) -> (Vec<u8>, Vec<u8>) {
    let seed = rng.bytes(32);
    let f = sym::<SeedKeypair>(c_lib(), "crypto_sign_ed25519_seed_keypair");
    let mut pk = vec![0u8; 32];
    let mut sk = vec![0u8; 64];
    unsafe { assert_eq!(f(pk.as_mut_ptr(), sk.as_mut_ptr(), seed.as_ptr()), 0) };
    (pk, sk)
}

// ===========================================================================
// constant accessors / API shape
// ===========================================================================

/// G5-001, G5-002, G5-003, G5-004, G5-035, G5-036, G5-037, G5-056, G5-057,
/// G5-078, G5-088, G5-089, G5-117, G5-118, G5-119, G5-120.
#[test]
fn constants_and_api_shape() {
    setup();

    // ---- crypto_box, default primitive (G5-001)
    assert_eq!(cst("crypto_box_seedbytes"), 32);
    assert_eq!(cst("crypto_box_publickeybytes"), 32);
    assert_eq!(cst("crypto_box_secretkeybytes"), 32);
    assert_eq!(cst("crypto_box_beforenmbytes"), 32);
    assert_eq!(cst("crypto_box_noncebytes"), 24);
    assert_eq!(cst("crypto_box_zerobytes"), 32);
    assert_eq!(cst("crypto_box_boxzerobytes"), 16);
    assert_eq!(cst("crypto_box_macbytes"), 16);
    assert_eq!(cst("crypto_box_messagebytes_max"), 18446744073709551599);
    assert_eq!(cst("crypto_box_sealbytes"), 48);
    assert_eq!(cprim("crypto_box_primitive"), "curve25519xsalsa20poly1305");

    // ---- explicit xsalsa20 names must equal the generic ones (G5-002)
    for s in [
        "seedbytes",
        "publickeybytes",
        "secretkeybytes",
        "beforenmbytes",
        "noncebytes",
        "zerobytes",
        "boxzerobytes",
        "macbytes",
        "messagebytes_max",
    ] {
        let g = cst(&format!("crypto_box_{s}"));
        let e = cst(&format!("crypto_box_curve25519xsalsa20poly1305_{s}"));
        assert_eq!(g, e, "crypto_box_{s} vs explicit primitive");
    }

    // ---- xchacha20 primitive (G5-003)
    for (s, want) in [
        ("seedbytes", 32usize),
        ("publickeybytes", 32),
        ("secretkeybytes", 32),
        ("beforenmbytes", 32),
        ("noncebytes", 24),
        ("macbytes", 16),
        ("messagebytes_max", 18446744073709551599),
        ("sealbytes", 48),
    ] {
        assert_eq!(
            cst(&format!("crypto_box_curve25519xchacha20poly1305_{s}")),
            want
        );
    }
    // deliberately absent for xchacha20
    for s in ["zerobytes", "boxzerobytes"] {
        let n = format!("crypto_box_curve25519xchacha20poly1305_{s}");
        assert!(
            unsafe { c_lib().get::<*const ()>(n.as_bytes()) }.is_err(),
            "{n} must not exist in the C reference"
        );
        assert!(
            unsafe { r_lib().get::<*const ()>(n.as_bytes()) }.is_err(),
            "{n} must not exist in the Rust port either"
        );
    }

    // ---- G5-004: API-surface asymmetry, verified symbol by symbol
    for n in [
        "crypto_box_curve25519xchacha20poly1305",
        "crypto_box_curve25519xchacha20poly1305_open",
        "crypto_box_curve25519xchacha20poly1305_afternm",
        "crypto_box_curve25519xchacha20poly1305_open_afternm",
        // xsalsa20's easy/detached/seal layers exist only generically
        "crypto_box_curve25519xsalsa20poly1305_easy",
        "crypto_box_curve25519xsalsa20poly1305_open_easy",
        "crypto_box_curve25519xsalsa20poly1305_detached",
        "crypto_box_curve25519xsalsa20poly1305_open_detached",
        "crypto_box_curve25519xsalsa20poly1305_easy_afternm",
        "crypto_box_curve25519xsalsa20poly1305_detached_afternm",
        "crypto_box_curve25519xsalsa20poly1305_seal",
        "crypto_box_curve25519xsalsa20poly1305_seal_open",
        "crypto_box_curve25519xsalsa20poly1305_sealbytes",
    ] {
        assert!(
            unsafe { c_lib().get::<*const ()>(n.as_bytes()) }.is_err(),
            "{n} must not exist in the C reference"
        );
        assert!(
            unsafe { r_lib().get::<*const ()>(n.as_bytes()) }.is_err(),
            "{n} must not exist in the Rust port"
        );
    }
    for n in [
        "crypto_box_curve25519xsalsa20poly1305",
        "crypto_box_curve25519xsalsa20poly1305_open",
        "crypto_box_curve25519xsalsa20poly1305_afternm",
        "crypto_box_curve25519xsalsa20poly1305_open_afternm",
        "crypto_box_curve25519xchacha20poly1305_easy",
        "crypto_box_curve25519xchacha20poly1305_open_easy",
        "crypto_box_curve25519xchacha20poly1305_detached",
        "crypto_box_curve25519xchacha20poly1305_open_detached",
        "crypto_box_curve25519xchacha20poly1305_easy_afternm",
        "crypto_box_curve25519xchacha20poly1305_open_easy_afternm",
        "crypto_box_curve25519xchacha20poly1305_detached_afternm",
        "crypto_box_curve25519xchacha20poly1305_open_detached_afternm",
        "crypto_box_curve25519xchacha20poly1305_seal",
        "crypto_box_curve25519xchacha20poly1305_seal_open",
    ] {
        assert!(has_sym(n), "{n} must exist in both libraries");
    }

    // ---- crypto_secretbox (G5-035, G5-036, G5-037)
    assert_eq!(cst("crypto_secretbox_keybytes"), 32);
    assert_eq!(cst("crypto_secretbox_noncebytes"), 24);
    assert_eq!(cst("crypto_secretbox_zerobytes"), 32);
    assert_eq!(cst("crypto_secretbox_boxzerobytes"), 16);
    assert_eq!(cst("crypto_secretbox_macbytes"), 16);
    assert_eq!(cst("crypto_secretbox_messagebytes_max"), 18446744073709551599);
    assert_eq!(cprim("crypto_secretbox_primitive"), "xsalsa20poly1305");
    for s in [
        "keybytes",
        "noncebytes",
        "zerobytes",
        "boxzerobytes",
        "macbytes",
        "messagebytes_max",
    ] {
        assert_eq!(
            cst(&format!("crypto_secretbox_{s}")),
            cst(&format!("crypto_secretbox_xsalsa20poly1305_{s}"))
        );
    }
    for (s, want) in [
        ("keybytes", 32usize),
        ("noncebytes", 24),
        ("macbytes", 16),
        ("messagebytes_max", 18446744073709551599),
    ] {
        assert_eq!(
            cst(&format!("crypto_secretbox_xchacha20poly1305_{s}")),
            want
        );
    }
    for s in ["zerobytes", "boxzerobytes"] {
        let n = format!("crypto_secretbox_xchacha20poly1305_{s}");
        assert!(unsafe { c_lib().get::<*const ()>(n.as_bytes()) }.is_err(), "{n}");
        assert!(unsafe { r_lib().get::<*const ()>(n.as_bytes()) }.is_err(), "{n}");
    }
    for n in [
        "crypto_secretbox_xchacha20poly1305",
        "crypto_secretbox_xchacha20poly1305_open",
        "crypto_secretbox_xchacha20poly1305_keygen",
    ] {
        assert!(unsafe { c_lib().get::<*const ()>(n.as_bytes()) }.is_err(), "{n}");
        assert!(unsafe { r_lib().get::<*const ()>(n.as_bytes()) }.is_err(), "{n}");
    }

    // ---- crypto_secretstream (G5-056, G5-057)
    let p = "crypto_secretstream_xchacha20poly1305";
    assert_eq!(cst(&format!("{p}_statebytes")), 52);
    assert_eq!(cst(&format!("{p}_abytes")), 17);
    assert_eq!(cst(&format!("{p}_headerbytes")), 24);
    assert_eq!(cst(&format!("{p}_keybytes")), 32);
    assert_eq!(cst(&format!("{p}_messagebytes_max")), 274877906816);
    assert_eq!(cbyte(&format!("{p}_tag_message")), 0x00);
    assert_eq!(cbyte(&format!("{p}_tag_push")), 0x01);
    assert_eq!(cbyte(&format!("{p}_tag_rekey")), 0x02);
    assert_eq!(cbyte(&format!("{p}_tag_final")), 0x03);
    assert_eq!(
        cbyte(&format!("{p}_tag_final")),
        cbyte(&format!("{p}_tag_push")) | cbyte(&format!("{p}_tag_rekey"))
    );

    // ---- crypto_kx (G5-078)
    assert_eq!(cst("crypto_kx_publickeybytes"), 32);
    assert_eq!(cst("crypto_kx_secretkeybytes"), 32);
    assert_eq!(cst("crypto_kx_seedbytes"), 32);
    assert_eq!(cst("crypto_kx_sessionkeybytes"), 32);
    assert_eq!(cprim("crypto_kx_primitive"), "x25519blake2b");

    // ---- crypto_sign (G5-088, G5-089)
    assert_eq!(cst("crypto_sign_statebytes"), 208);
    assert_eq!(cst("crypto_sign_ed25519ph_statebytes"), 208);
    assert_eq!(cst("crypto_sign_bytes"), 64);
    assert_eq!(cst("crypto_sign_seedbytes"), 32);
    assert_eq!(cst("crypto_sign_publickeybytes"), 32);
    assert_eq!(cst("crypto_sign_secretkeybytes"), 64);
    assert_eq!(cst("crypto_sign_messagebytes_max"), 18446744073709551551);
    assert_eq!(cprim("crypto_sign_primitive"), "ed25519");
    for s in [
        "bytes",
        "seedbytes",
        "publickeybytes",
        "secretkeybytes",
        "messagebytes_max",
    ] {
        assert_eq!(
            cst(&format!("crypto_sign_{s}")),
            cst(&format!("crypto_sign_ed25519_{s}"))
        );
    }

    // ---- crypto_auth (G5-117, G5-118, G5-119, G5-120)
    assert_eq!(cst("crypto_auth_bytes"), 32);
    assert_eq!(cst("crypto_auth_keybytes"), 32);
    assert_eq!(cprim("crypto_auth_primitive"), "hmacsha512256");
    assert_eq!(cst("crypto_auth_hmacsha256_bytes"), 32);
    assert_eq!(cst("crypto_auth_hmacsha256_keybytes"), 32);
    assert_eq!(cst("crypto_auth_hmacsha256_statebytes"), 208);
    assert_eq!(cst("crypto_auth_hmacsha512_bytes"), 64);
    assert_eq!(cst("crypto_auth_hmacsha512_keybytes"), 32);
    assert_eq!(cst("crypto_auth_hmacsha512_statebytes"), 416);
    assert_eq!(cst("crypto_auth_hmacsha512256_bytes"), 32);
    assert_eq!(cst("crypto_auth_hmacsha512256_keybytes"), 32);
    assert_eq!(cst("crypto_auth_hmacsha512256_statebytes"), 416);
}

// ===========================================================================
// key generation
// ===========================================================================

/// G5-005, G5-006, G5-079, G5-080, G5-090, G5-091.
#[test]
fn keypairs_and_seed_keypairs() {
    setup();
    let mut rng = Rng::new(0xB000);
    let smb = sym::<Two>(c_lib(), "crypto_scalarmult_base");

    // ---- randomised keypairs (G5-005, G5-079, G5-090)
    for name in [
        "crypto_box_keypair",
        "crypto_box_curve25519xsalsa20poly1305_keypair",
        "crypto_box_curve25519xchacha20poly1305_keypair",
        "crypto_kx_keypair",
    ] {
        let (c, r) = pair::<Keypair>(name);
        for i in 0..12u64 {
            let seed = 0xB100 + i;
            let mut pk1 = canary(32 + 4);
            let mut sk1 = canary(32 + 4);
            let mut pk2 = canary(32 + 4);
            let mut sk2 = canary(32 + 4);
            let (ra, rb) = {
                let _g = RNG_LOCK.lock().unwrap();
                reset_rngs(seed);
                let ra = unsafe { c(pk1.as_mut_ptr(), sk1.as_mut_ptr()) };
                reset_rngs(seed);
                let rb = unsafe { r(pk2.as_mut_ptr(), sk2.as_mut_ptr()) };
                (ra, rb)
            };
            eq_i32(&format!("{name} rc"), ra, rb);
            assert_eq!(ra, 0);
            eq_bytes(&format!("{name} pk (seed={seed})"), &pk1, &pk2);
            eq_bytes(&format!("{name} sk (seed={seed})"), &sk1, &sk2);
            assert_eq!(&pk1[32..], &[0xA5u8; 4], "{name} wrote past pk");
            assert_eq!(&sk1[32..], &[0xA5u8; 4], "{name} wrote past sk");
            // pk == scalarmult_base(sk)
            let mut chk = [0u8; 32];
            unsafe { assert_eq!(smb(chk.as_mut_ptr(), sk1.as_ptr()), 0) };
            assert_eq!(&chk[..], &pk1[..32], "{name}: pk != scalarmult_base(sk)");
        }
    }

    // sign keypair: sk = seed ‖ pk (G5-090)
    {
        let (c, r) = pair::<Keypair>("crypto_sign_keypair");
        let (c2, r2) = pair::<Keypair>("crypto_sign_ed25519_keypair");
        let to_seed = pair::<Two>("crypto_sign_ed25519_sk_to_seed");
        let to_pk = pair::<Two>("crypto_sign_ed25519_sk_to_pk");
        let seed_kp = sym::<SeedKeypair>(c_lib(), "crypto_sign_ed25519_seed_keypair");
        for i in 0..12u64 {
            let s = 0xB200 + i;
            let mut out: Vec<(Vec<u8>, Vec<u8>)> = Vec::new();
            {
                let _g = RNG_LOCK.lock().unwrap();
                for f in [c, r, c2, r2] {
                    let mut pk = canary(32 + 4);
                    let mut sk = canary(64 + 4);
                    reset_rngs(s);
                    let rc = unsafe { f(pk.as_mut_ptr(), sk.as_mut_ptr()) };
                    assert_eq!(rc, 0);
                    out.push((pk, sk));
                }
            }
            eq_bytes("crypto_sign_keypair pk", &out[0].0, &out[1].0);
            eq_bytes("crypto_sign_keypair sk", &out[0].1, &out[1].1);
            eq_bytes("crypto_sign_ed25519_keypair pk", &out[2].0, &out[3].0);
            eq_bytes("crypto_sign_ed25519_keypair sk", &out[2].1, &out[3].1);
            assert_eq!(out[0].0, out[2].0, "generic vs explicit keypair pk");
            assert_eq!(out[0].1, out[2].1, "generic vs explicit keypair sk");
            assert_eq!(&out[0].0[32..], &[0xA5u8; 4]);
            assert_eq!(&out[0].1[64..], &[0xA5u8; 4]);
            // sk = seed ‖ pk, recoverable with the conversions
            let (pk, sk) = (&out[0].0[..32], &out[0].1[..64]);
            assert_eq!(&sk[32..64], pk, "sk[32..64] must be pk");
            for (which, (ts, tp)) in [(0, (to_seed.0, to_pk.0)), (1, (to_seed.1, to_pk.1))] {
                let mut seed = canary(32);
                let mut pk2 = canary(32);
                unsafe {
                    assert_eq!(ts(seed.as_mut_ptr(), sk.as_ptr()), 0);
                    assert_eq!(tp(pk2.as_mut_ptr(), sk.as_ptr()), 0);
                }
                assert_eq!(&seed[..], &sk[..32], "sk_to_seed (lib={which})");
                assert_eq!(&pk2[..], pk, "sk_to_pk (lib={which})");
                // and the recovered seed regenerates the same key pair
                let mut pk3 = [0u8; 32];
                let mut sk3 = [0u8; 64];
                unsafe {
                    assert_eq!(
                        seed_kp(pk3.as_mut_ptr(), sk3.as_mut_ptr(), seed.as_ptr()),
                        0
                    )
                };
                assert_eq!(&pk3[..], pk);
                assert_eq!(&sk3[..], sk);
            }
        }
    }

    // ---- seeded keypairs (G5-006, G5-080, G5-091)
    let mut seeds: Vec<Vec<u8>> = vec![
        vec![0u8; 32],
        vec![0xffu8; 32],
        vec![1u8; 32],
        (0u8..32).collect(),
    ];
    for _ in 0..10 {
        seeds.push(rng.bytes(32));
    }
    for name in [
        "crypto_box_seed_keypair",
        "crypto_box_curve25519xsalsa20poly1305_seed_keypair",
        "crypto_box_curve25519xchacha20poly1305_seed_keypair",
        "crypto_kx_seed_keypair",
        "crypto_sign_seed_keypair",
        "crypto_sign_ed25519_seed_keypair",
    ] {
        let (c, r) = pair::<SeedKeypair>(name);
        let skb = if name.contains("sign") { 64 } else { 32 };
        for seed in &seeds {
            let mut pk1 = canary(32 + 4);
            let mut sk1 = canary(skb + 4);
            let mut pk2 = canary(32 + 4);
            let mut sk2 = canary(skb + 4);
            let (ra, rb) = unsafe {
                (
                    c(pk1.as_mut_ptr(), sk1.as_mut_ptr(), seed.as_ptr()),
                    r(pk2.as_mut_ptr(), sk2.as_mut_ptr(), seed.as_ptr()),
                )
            };
            eq_i32(&format!("{name} rc"), ra, rb);
            assert_eq!(ra, 0);
            eq_bytes(&format!("{name} pk(seed={})", hex(seed)), &pk1, &pk2);
            eq_bytes(&format!("{name} sk(seed={})", hex(seed)), &sk1, &sk2);
            assert_eq!(&pk1[32..], &[0xA5u8; 4], "{name} wrote past pk");
            assert_eq!(&sk1[skb..], &[0xA5u8; 4], "{name} wrote past sk");
            // determinism: a second call reproduces it exactly
            let mut pk3 = canary(32 + 4);
            let mut sk3 = canary(skb + 4);
            unsafe { c(pk3.as_mut_ptr(), sk3.as_mut_ptr(), seed.as_ptr()) };
            assert_eq!(pk1, pk3, "{name} not deterministic");
            // pk == scalarmult_base(sk) for the X25519 flavours
            if !name.contains("sign") {
                let mut chk = [0u8; 32];
                unsafe { assert_eq!(smb(chk.as_mut_ptr(), sk1.as_ptr()), 0) };
                assert_eq!(&chk[..], &pk1[..32], "{name}: pk != scalarmult_base(sk)");
            }
        }
    }
    // sk = SHA-512(seed)[0..31] for both crypto_box primitives, and the two
    // primitives share the derivation exactly.
    {
        let sha = sym::<unsafe extern "C" fn(*mut u8, *const u8, u64) -> i32>(
            c_lib(),
            "crypto_hash_sha512",
        );
        let a = sym::<SeedKeypair>(c_lib(), "crypto_box_curve25519xsalsa20poly1305_seed_keypair");
        let b = sym::<SeedKeypair>(c_lib(), "crypto_box_curve25519xchacha20poly1305_seed_keypair");
        for seed in &seeds {
            let mut h = [0u8; 64];
            unsafe { sha(h.as_mut_ptr(), seed.as_ptr(), 32) };
            let mut pk1 = [0u8; 32];
            let mut sk1 = [0u8; 32];
            let mut pk2 = [0u8; 32];
            let mut sk2 = [0u8; 32];
            unsafe {
                a(pk1.as_mut_ptr(), sk1.as_mut_ptr(), seed.as_ptr());
                b(pk2.as_mut_ptr(), sk2.as_mut_ptr(), seed.as_ptr());
            }
            assert_eq!(&sk1[..], &h[..32], "box sk must be SHA-512(seed)[0..31]");
            assert_eq!(sk1, sk2, "both box primitives derive the same sk");
            assert_eq!(pk1, pk2);
        }
        // crypto_kx uses BLAKE2b-256, NOT SHA-512
        let gh = sym::<unsafe extern "C" fn(*mut u8, usize, *const u8, u64, *const u8, usize) -> i32>(
            c_lib(),
            "crypto_generichash",
        );
        let k = sym::<SeedKeypair>(c_lib(), "crypto_kx_seed_keypair");
        for seed in &seeds {
            let mut want = [0u8; 32];
            unsafe { gh(want.as_mut_ptr(), 32, seed.as_ptr(), 32, ptr::null(), 0) };
            let mut pk = [0u8; 32];
            let mut sk = [0u8; 32];
            unsafe { k(pk.as_mut_ptr(), sk.as_mut_ptr(), seed.as_ptr()) };
            assert_eq!(sk, want, "crypto_kx sk must be BLAKE2b-256(seed)");
        }
    }
}

/// G5-038, G5-058, G5-121 — every `_keygen` in G5.
#[test]
fn keygens() {
    setup();
    for name in [
        "crypto_secretbox_keygen",
        "crypto_secretbox_xsalsa20poly1305_keygen",
        "crypto_secretstream_xchacha20poly1305_keygen",
        "crypto_auth_keygen",
        "crypto_auth_hmacsha256_keygen",
        "crypto_auth_hmacsha512_keygen",
        "crypto_auth_hmacsha512256_keygen",
    ] {
        let (c, r) = pair::<Keygen>(name);
        for i in 0..16u64 {
            let s = 0xB300 + i;
            let mut a = canary(32 + 8);
            let mut b = canary(32 + 8);
            {
                let _g = RNG_LOCK.lock().unwrap();
                reset_rngs(s);
                unsafe { c(a.as_mut_ptr()) };
                reset_rngs(s);
                unsafe { r(b.as_mut_ptr()) };
            }
            eq_bytes(&format!("{name}(seed={s})"), &a, &b);
            assert_eq!(&a[32..], &[0xA5u8; 8], "{name} wrote past 32 bytes");
        }
    }
}

// ===========================================================================
// crypto_box — precomputation
// ===========================================================================

/// G5-007, G5-008 — `_beforenm` for both primitives: symmetric in (pk, sk),
/// 32-byte output, and the two primitives disagree with each other.
#[test]
fn box_beforenm_agreement() {
    setup();
    let mut rng = Rng::new(0xB400);
    let names: &[&str] = &[
        "crypto_box_beforenm",
        "crypto_box_curve25519xsalsa20poly1305_beforenm",
        "crypto_box_curve25519xchacha20poly1305_beforenm",
    ];
    for _ in 0..16 {
        let (pk_a, sk_a) = box_kp_rng(&mut rng);
        let (pk_b, sk_b) = box_kp_rng(&mut rng);
        let mut per_name: Vec<Vec<u8>> = Vec::new();
        for &name in names {
            let (c, r) = pair::<Three>(name);
            let mut k1 = canary(32 + 4);
            let mut k2 = canary(32 + 4);
            let (ra, rb) = unsafe {
                (
                    c(k1.as_mut_ptr(), pk_b.as_ptr(), sk_a.as_ptr()),
                    r(k2.as_mut_ptr(), pk_b.as_ptr(), sk_a.as_ptr()),
                )
            };
            eq_i32(&format!("{name} rc"), ra, rb);
            assert_eq!(ra, 0);
            eq_bytes(&format!("{name} k"), &k1, &k2);
            assert_eq!(&k1[32..], &[0xA5u8; 4], "{name} wrote past 32 bytes");

            // symmetry: beforenm(pkA, skB) == beforenm(pkB, skA)
            let mut k3 = canary(32 + 4);
            let mut k4 = canary(32 + 4);
            unsafe {
                assert_eq!(c(k3.as_mut_ptr(), pk_a.as_ptr(), sk_b.as_ptr()), 0);
                assert_eq!(r(k4.as_mut_ptr(), pk_a.as_ptr(), sk_b.as_ptr()), 0);
            }
            eq_bytes(&format!("{name} k (swapped)"), &k3, &k4);
            assert_eq!(k1, k3, "{name} must be symmetric in (pk, sk)");
            per_name.push(k1[..32].to_vec());
        }
        // generic == explicit xsalsa20, and xchacha20 differs
        assert_eq!(per_name[0], per_name[1], "generic beforenm == xsalsa20");
        assert_ne!(per_name[0], per_name[2], "xchacha20 beforenm must differ");
    }
}

// ===========================================================================
// crypto_box — (a) NaCl padded API
// ===========================================================================

/// G5-009, G5-010, G5-011, G5-012, G5-013, G5-014 — the NaCl combined and
/// precomputed APIs with the real `ZEROBYTES` / `BOXZEROBYTES` padding, plus
/// generic-vs-explicit and combined-vs-afternm equality.
#[test]
fn box_nacl_padded_api() {
    setup();
    let mut rng = Rng::new(0xB500);
    let zb = 32usize; // crypto_box_ZEROBYTES
    let bzb = 16usize; // crypto_box_BOXZEROBYTES

    let seal_ = pair::<Asym6>("crypto_box");
    let open_ = pair::<Asym6>("crypto_box_open");
    let xseal = pair::<Asym6>("crypto_box_curve25519xsalsa20poly1305");
    let xopen = pair::<Asym6>("crypto_box_curve25519xsalsa20poly1305_open");
    let after = pair::<Sym5>("crypto_box_afternm");
    let oafter = pair::<Sym5>("crypto_box_open_afternm");
    let xafter = pair::<Sym5>("crypto_box_curve25519xsalsa20poly1305_afternm");
    let xoafter = pair::<Sym5>("crypto_box_curve25519xsalsa20poly1305_open_afternm");
    let bnm = sym::<Three>(c_lib(), "crypto_box_beforenm");

    // payload lengths → total mlen = payload + ZEROBYTES
    let payloads: &[usize] = &[0, 1, 15, 16, 17, 31, 32, 33, 63, 64, 65, 127, 128, 1000];
    for &payload in payloads {
        for _ in 0..3 {
            let mlen = payload + zb;
            let (pk_a, sk_a) = box_kp_rng(&mut rng);
            let (pk_b, sk_b) = box_kp_rng(&mut rng);
            let n = rng.bytes(24);
            // the padded plaintext: ZEROBYTES zeros, then the payload
            let mut m = vec![0u8; zb];
            m.extend_from_slice(&rng.bytes(payload));
            assert_eq!(m.len(), mlen);

            let mut k = [0u8; 32];
            unsafe { assert_eq!(bnm(k.as_mut_ptr(), pk_b.as_ptr(), sk_a.as_ptr()), 0) };

            // --- combined
            let mut c1 = canary(mlen + 4);
            let mut c2 = canary(mlen + 4);
            let (ra, rb) = unsafe {
                (
                    seal_.0(c1.as_mut_ptr(), m.as_ptr(), mlen as u64, n.as_ptr(),
                            pk_b.as_ptr(), sk_a.as_ptr()),
                    seal_.1(c2.as_mut_ptr(), m.as_ptr(), mlen as u64, n.as_ptr(),
                            pk_b.as_ptr(), sk_a.as_ptr()),
                )
            };
            eq_i32(&format!("crypto_box rc(mlen={mlen})"), ra, rb);
            assert_eq!(ra, 0);
            eq_bytes(&format!("crypto_box(mlen={mlen})"), &c1, &c2);
            assert_eq!(&c1[mlen..], &[0xA5u8; 4], "crypto_box wrote past mlen");
            assert_eq!(&c1[..bzb], &vec![0u8; bzb][..], "c[0..16] must be zero");

            // --- explicit primitive must be byte-identical
            let mut c3 = canary(mlen + 4);
            let mut c4 = canary(mlen + 4);
            unsafe {
                assert_eq!(
                    xseal.0(c3.as_mut_ptr(), m.as_ptr(), mlen as u64, n.as_ptr(),
                            pk_b.as_ptr(), sk_a.as_ptr()),
                    0
                );
                assert_eq!(
                    xseal.1(c4.as_mut_ptr(), m.as_ptr(), mlen as u64, n.as_ptr(),
                            pk_b.as_ptr(), sk_a.as_ptr()),
                    0
                );
            }
            eq_bytes("crypto_box_curve25519xsalsa20poly1305", &c3, &c4);
            assert_eq!(c1, c3, "generic crypto_box != explicit primitive");

            // --- afternm must be byte-identical (G5-012, G5-014)
            for (what, f) in [
                ("crypto_box_afternm", after),
                ("crypto_box_curve25519xsalsa20poly1305_afternm", xafter),
            ] {
                let mut a1 = canary(mlen + 4);
                let mut a2 = canary(mlen + 4);
                let (x, y) = unsafe {
                    (
                        f.0(a1.as_mut_ptr(), m.as_ptr(), mlen as u64, n.as_ptr(), k.as_ptr()),
                        f.1(a2.as_mut_ptr(), m.as_ptr(), mlen as u64, n.as_ptr(), k.as_ptr()),
                    )
                };
                eq_i32(&format!("{what} rc"), x, y);
                assert_eq!(x, 0);
                eq_bytes(&format!("{what}(mlen={mlen})"), &a1, &a2);
                assert_eq!(a1, c1, "{what} != crypto_box");
            }

            // --- open, all four flavours
            for (what, f, key_is_pair) in [
                ("crypto_box_open", 0u8, true),
                ("crypto_box_curve25519xsalsa20poly1305_open", 1, true),
                ("crypto_box_open_afternm", 2, false),
                ("crypto_box_curve25519xsalsa20poly1305_open_afternm", 3, false),
            ] {
                let mut m1 = canary(mlen + 4);
                let mut m2 = canary(mlen + 4);
                let (x, y) = unsafe {
                    if key_is_pair {
                        let g = if f == 0 { open_ } else { xopen };
                        (
                            g.0(m1.as_mut_ptr(), c1.as_ptr(), mlen as u64, n.as_ptr(),
                                pk_a.as_ptr(), sk_b.as_ptr()),
                            g.1(m2.as_mut_ptr(), c1.as_ptr(), mlen as u64, n.as_ptr(),
                                pk_a.as_ptr(), sk_b.as_ptr()),
                        )
                    } else {
                        let g = if f == 2 { oafter } else { xoafter };
                        (
                            g.0(m1.as_mut_ptr(), c1.as_ptr(), mlen as u64, n.as_ptr(), k.as_ptr()),
                            g.1(m2.as_mut_ptr(), c1.as_ptr(), mlen as u64, n.as_ptr(), k.as_ptr()),
                        )
                    }
                };
                eq_i32(&format!("{what} rc(mlen={mlen})"), x, y);
                assert_eq!(x, 0, "{what} round trip must succeed");
                eq_bytes(&format!("{what}(mlen={mlen})"), &m1, &m2);
                assert_eq!(&m1[..mlen], &m[..], "{what} plaintext");
                assert_eq!(&m1[..zb], &vec![0u8; zb][..], "open must zero m[0..32]");
                assert_eq!(&m1[mlen..], &[0xA5u8; 4], "{what} wrote past mlen");
            }
        }
    }
}

// ===========================================================================
// crypto_box — (b) easy and (c) detached APIs
// ===========================================================================

/// G5-015 … G5-023 — the easy and detached layers, their `_afternm` twins,
/// their mutual consistency, and verify-only (`m == NULL`) mode.
#[test]
fn box_easy_and_detached() {
    setup();
    let mut rng = Rng::new(0xB600);
    let easy = pair::<Asym6>("crypto_box_easy");
    let oeasy = pair::<Asym6>("crypto_box_open_easy");
    let easya = pair::<Sym5>("crypto_box_easy_afternm");
    let oeasya = pair::<Sym5>("crypto_box_open_easy_afternm");
    let det = pair::<Det7>("crypto_box_detached");
    let odet = pair::<ODet7>("crypto_box_open_detached");
    let deta = pair::<Det6>("crypto_box_detached_afternm");
    let odeta = pair::<ODet6>("crypto_box_open_detached_afternm");
    let bnm = sym::<Three>(c_lib(), "crypto_box_beforenm");

    for &mlen in MLENS {
        for _ in 0..3 {
            let (pk_a, sk_a) = box_kp_rng(&mut rng);
            let (pk_b, sk_b) = box_kp_rng(&mut rng);
            let n = rng.bytes(24);
            let m = rng.bytes(mlen);
            let mut k = [0u8; 32];
            unsafe { assert_eq!(bnm(k.as_mut_ptr(), pk_b.as_ptr(), sk_a.as_ptr()), 0) };

            // --- easy (G5-015, G5-016, G5-017)
            let mut c1 = canary(mlen + 16 + 4);
            let mut c2 = canary(mlen + 16 + 4);
            let (ra, rb) = unsafe {
                (
                    easy.0(c1.as_mut_ptr(), m.as_ptr(), mlen as u64, n.as_ptr(),
                           pk_b.as_ptr(), sk_a.as_ptr()),
                    easy.1(c2.as_mut_ptr(), m.as_ptr(), mlen as u64, n.as_ptr(),
                           pk_b.as_ptr(), sk_a.as_ptr()),
                )
            };
            eq_i32(&format!("crypto_box_easy rc(mlen={mlen})"), ra, rb);
            assert_eq!(ra, 0);
            eq_bytes(&format!("crypto_box_easy(mlen={mlen})"), &c1, &c2);
            assert_eq!(&c1[mlen + 16..], &[0xA5u8; 4], "easy wrote past mlen+16");

            // --- easy_afternm must be byte-identical (G5-018)
            let mut e1 = canary(mlen + 16 + 4);
            let mut e2 = canary(mlen + 16 + 4);
            let (x, y) = unsafe {
                (
                    easya.0(e1.as_mut_ptr(), m.as_ptr(), mlen as u64, n.as_ptr(), k.as_ptr()),
                    easya.1(e2.as_mut_ptr(), m.as_ptr(), mlen as u64, n.as_ptr(), k.as_ptr()),
                )
            };
            eq_i32("crypto_box_easy_afternm rc", x, y);
            assert_eq!(x, 0);
            eq_bytes(&format!("crypto_box_easy_afternm(mlen={mlen})"), &e1, &e2);
            assert_eq!(e1, c1, "easy_afternm != easy");

            // --- detached (G5-019, G5-020, G5-021) and its _afternm twin (G5-022)
            for (what, is_after) in [("crypto_box_detached", false), ("crypto_box_detached_afternm", true)] {
                let mut d1 = canary(mlen.max(1) + 4);
                let mut d2 = canary(mlen.max(1) + 4);
                let mut t1 = canary(16 + 4);
                let mut t2 = canary(16 + 4);
                let (x, y) = unsafe {
                    if is_after {
                        (
                            deta.0(d1.as_mut_ptr(), t1.as_mut_ptr(), m.as_ptr(), mlen as u64,
                                   n.as_ptr(), k.as_ptr()),
                            deta.1(d2.as_mut_ptr(), t2.as_mut_ptr(), m.as_ptr(), mlen as u64,
                                   n.as_ptr(), k.as_ptr()),
                        )
                    } else {
                        (
                            det.0(d1.as_mut_ptr(), t1.as_mut_ptr(), m.as_ptr(), mlen as u64,
                                  n.as_ptr(), pk_b.as_ptr(), sk_a.as_ptr()),
                            det.1(d2.as_mut_ptr(), t2.as_mut_ptr(), m.as_ptr(), mlen as u64,
                                  n.as_ptr(), pk_b.as_ptr(), sk_a.as_ptr()),
                        )
                    }
                };
                eq_i32(&format!("{what} rc(mlen={mlen})"), x, y);
                assert_eq!(x, 0);
                eq_bytes(&format!("{what} c(mlen={mlen})"), &d1, &d2);
                eq_bytes(&format!("{what} mac(mlen={mlen})"), &t1, &t2);
                assert_eq!(&t1[16..], &[0xA5u8; 4], "{what} wrote past mac");
                // detached mac ‖ c == the easy output
                assert_eq!(&t1[..16], &c1[..16], "{what}: mac != easy c[0..16]");
                assert_eq!(&d1[..mlen], &c1[16..mlen + 16], "{what}: c != easy c[16..]");

                // round trip through the matching open
                let mut p1 = canary(mlen.max(1) + 4);
                let mut p2 = canary(mlen.max(1) + 4);
                let (x, y) = unsafe {
                    if is_after {
                        (
                            odeta.0(p1.as_mut_ptr(), d1.as_ptr(), t1.as_ptr(), mlen as u64,
                                    n.as_ptr(), k.as_ptr()),
                            odeta.1(p2.as_mut_ptr(), d1.as_ptr(), t1.as_ptr(), mlen as u64,
                                    n.as_ptr(), k.as_ptr()),
                        )
                    } else {
                        (
                            odet.0(p1.as_mut_ptr(), d1.as_ptr(), t1.as_ptr(), mlen as u64,
                                   n.as_ptr(), pk_a.as_ptr(), sk_b.as_ptr()),
                            odet.1(p2.as_mut_ptr(), d1.as_ptr(), t1.as_ptr(), mlen as u64,
                                   n.as_ptr(), pk_a.as_ptr(), sk_b.as_ptr()),
                        )
                    }
                };
                eq_i32(&format!("{what} open rc(mlen={mlen})"), x, y);
                assert_eq!(x, 0);
                eq_bytes(&format!("{what} open(mlen={mlen})"), &p1, &p2);
                assert_eq!(&p1[..mlen], &m[..]);
                assert_eq!(&p1[mlen.max(1)..], &[0xA5u8; 4]);

                // G5-023: m == NULL (verify only)
                let (x, y) = unsafe {
                    if is_after {
                        (
                            odeta.0(ptr::null_mut(), d1.as_ptr(), t1.as_ptr(), mlen as u64,
                                    n.as_ptr(), k.as_ptr()),
                            odeta.1(ptr::null_mut(), d1.as_ptr(), t1.as_ptr(), mlen as u64,
                                    n.as_ptr(), k.as_ptr()),
                        )
                    } else {
                        (
                            odet.0(ptr::null_mut(), d1.as_ptr(), t1.as_ptr(), mlen as u64,
                                   n.as_ptr(), pk_a.as_ptr(), sk_b.as_ptr()),
                            odet.1(ptr::null_mut(), d1.as_ptr(), t1.as_ptr(), mlen as u64,
                                   n.as_ptr(), pk_a.as_ptr(), sk_b.as_ptr()),
                        )
                    }
                };
                eq_i32(&format!("{what} open m=NULL rc(mlen={mlen})"), x, y);
                assert_eq!(x, 0, "{what} verify-only must succeed");
            }

            // --- open_easy / open_easy_afternm
            for (what, is_after) in [
                ("crypto_box_open_easy", false),
                ("crypto_box_open_easy_afternm", true),
            ] {
                let mut p1 = canary(mlen.max(1) + 4);
                let mut p2 = canary(mlen.max(1) + 4);
                let (x, y) = unsafe {
                    if is_after {
                        (
                            oeasya.0(p1.as_mut_ptr(), c1.as_ptr(), (mlen + 16) as u64,
                                     n.as_ptr(), k.as_ptr()),
                            oeasya.1(p2.as_mut_ptr(), c1.as_ptr(), (mlen + 16) as u64,
                                     n.as_ptr(), k.as_ptr()),
                        )
                    } else {
                        (
                            oeasy.0(p1.as_mut_ptr(), c1.as_ptr(), (mlen + 16) as u64,
                                    n.as_ptr(), pk_a.as_ptr(), sk_b.as_ptr()),
                            oeasy.1(p2.as_mut_ptr(), c1.as_ptr(), (mlen + 16) as u64,
                                    n.as_ptr(), pk_a.as_ptr(), sk_b.as_ptr()),
                        )
                    }
                };
                eq_i32(&format!("{what} rc(mlen={mlen})"), x, y);
                assert_eq!(x, 0);
                eq_bytes(&format!("{what}(mlen={mlen})"), &p1, &p2);
                assert_eq!(&p1[..mlen], &m[..]);
            }
        }
    }
}

// ===========================================================================
// crypto_box — (d) sealed boxes
// ===========================================================================

/// G5-024, G5-025, G5-026 — sealed boxes for the default primitive.
#[test]
fn box_seal_roundtrip() {
    setup();
    let mut rng = Rng::new(0xB700);
    let seal = pair::<Seal>("crypto_box_seal");
    let open = pair::<SealOpen>("crypto_box_seal_open");
    let gh_init = sym::<unsafe extern "C" fn(*mut u8, *const u8, usize, usize) -> i32>(
        c_lib(),
        "crypto_generichash_init",
    );
    let gh_upd = sym::<unsafe extern "C" fn(*mut u8, *const u8, u64) -> i32>(
        c_lib(),
        "crypto_generichash_update",
    );
    let gh_fin = sym::<unsafe extern "C" fn(*mut u8, *mut u8, usize) -> i32>(
        c_lib(),
        "crypto_generichash_final",
    );
    let odet = sym::<ODet7>(c_lib(), "crypto_box_open_detached");

    for &mlen in &[0usize, 1, 15, 16, 17, 31, 32, 33, 63, 64, 65, 127, 128, 1000] {
        for i in 0..3u64 {
            let (pk, sk) = box_kp_rng(&mut rng);
            let m = rng.bytes(mlen);
            let s = 0xB780 + i + mlen as u64;
            let mut c1 = canary(mlen + 48 + 4);
            let mut c2 = canary(mlen + 48 + 4);
            let (ra, rb) = {
                let _g = RNG_LOCK.lock().unwrap();
                reset_rngs(s);
                let ra = unsafe { seal.0(c1.as_mut_ptr(), m.as_ptr(), mlen as u64, pk.as_ptr()) };
                reset_rngs(s);
                let rb = unsafe { seal.1(c2.as_mut_ptr(), m.as_ptr(), mlen as u64, pk.as_ptr()) };
                (ra, rb)
            };
            eq_i32(&format!("crypto_box_seal rc(mlen={mlen})"), ra, rb);
            assert_eq!(ra, 0);
            eq_bytes(&format!("crypto_box_seal(mlen={mlen})"), &c1, &c2);
            assert_eq!(&c1[mlen + 48..], &[0xA5u8; 4], "seal wrote past mlen+48");

            // c[0..32] is the ephemeral pk; the nonce is BLAKE2b-24(epk ‖ pk)
            let epk = &c1[..32];
            let mut nonce = [0u8; 24];
            let mut st = State::new(cst("crypto_generichash_statebytes"));
            unsafe {
                assert_eq!(gh_init(st.as_mut_ptr(), ptr::null(), 0, 24), 0);
                assert_eq!(gh_upd(st.as_mut_ptr(), epk.as_ptr(), 32), 0);
                assert_eq!(gh_upd(st.as_mut_ptr(), pk.as_ptr(), 32), 0);
                assert_eq!(gh_fin(st.as_mut_ptr(), nonce.as_mut_ptr(), 24), 0);
            }
            // that nonce, with the recipient sk and the embedded epk, opens the
            // detached box formed by c[32..48] ‖ c[48..]
            let mut chk = canary(mlen.max(1));
            let rc = unsafe {
                odet(
                    chk.as_mut_ptr(),
                    c1[48..].as_ptr(),
                    c1[32..48].as_ptr(),
                    mlen as u64,
                    nonce.as_ptr(),
                    epk.as_ptr(),
                    sk.as_ptr(),
                )
            };
            assert_eq!(rc, 0, "seal layout / nonce derivation (mlen={mlen})");
            assert_eq!(&chk[..mlen], &m[..]);

            // --- seal_open (G5-025; mlen == 0 is the G5-026 boundary)
            let mut p1 = canary(mlen.max(1) + 4);
            let mut p2 = canary(mlen.max(1) + 4);
            let (x, y) = unsafe {
                (
                    open.0(p1.as_mut_ptr(), c1.as_ptr(), (mlen + 48) as u64,
                           pk.as_ptr(), sk.as_ptr()),
                    open.1(p2.as_mut_ptr(), c1.as_ptr(), (mlen + 48) as u64,
                           pk.as_ptr(), sk.as_ptr()),
                )
            };
            eq_i32(&format!("crypto_box_seal_open rc(mlen={mlen})"), x, y);
            assert_eq!(x, 0);
            eq_bytes(&format!("crypto_box_seal_open(mlen={mlen})"), &p1, &p2);
            assert_eq!(&p1[..mlen], &m[..]);
            assert_eq!(&p1[mlen.max(1)..], &[0xA5u8; 4]);
        }
    }
}

// ===========================================================================
// crypto_box — the xchacha20 primitive, all available layers
// ===========================================================================

/// G5-027 … G5-033 — the xchacha20poly1305 easy / detached / precomputed /
/// sealed layers, and their difference from the xsalsa20 default.
#[test]
fn box_xchacha20_layers() {
    setup();
    let mut rng = Rng::new(0xB800);
    let p = "crypto_box_curve25519xchacha20poly1305";
    let easy = pair::<Asym6>(&format!("{p}_easy"));
    let oeasy = pair::<Asym6>(&format!("{p}_open_easy"));
    let easya = pair::<Sym5>(&format!("{p}_easy_afternm"));
    let oeasya = pair::<Sym5>(&format!("{p}_open_easy_afternm"));
    let det = pair::<Det7>(&format!("{p}_detached"));
    let odet = pair::<ODet7>(&format!("{p}_open_detached"));
    let deta = pair::<Det6>(&format!("{p}_detached_afternm"));
    let odeta = pair::<ODet6>(&format!("{p}_open_detached_afternm"));
    let seal = pair::<Seal>(&format!("{p}_seal"));
    let sopen = pair::<SealOpen>(&format!("{p}_seal_open"));
    let bnm = sym::<Three>(c_lib(), &format!("{p}_beforenm"));
    let xs_easy = sym::<Asym6>(c_lib(), "crypto_box_easy");

    for &mlen in MLENS {
        for _ in 0..3 {
            let (pk_a, sk_a) = box_kp_rng(&mut rng);
            let (pk_b, sk_b) = box_kp_rng(&mut rng);
            let n = rng.bytes(24);
            let m = rng.bytes(mlen);
            let mut k = [0u8; 32];
            unsafe { assert_eq!(bnm(k.as_mut_ptr(), pk_b.as_ptr(), sk_a.as_ptr()), 0) };

            // --- easy (G5-027, G5-028, G5-029)
            let mut c1 = canary(mlen + 16 + 4);
            let mut c2 = canary(mlen + 16 + 4);
            let (ra, rb) = unsafe {
                (
                    easy.0(c1.as_mut_ptr(), m.as_ptr(), mlen as u64, n.as_ptr(),
                           pk_b.as_ptr(), sk_a.as_ptr()),
                    easy.1(c2.as_mut_ptr(), m.as_ptr(), mlen as u64, n.as_ptr(),
                           pk_b.as_ptr(), sk_a.as_ptr()),
                )
            };
            eq_i32(&format!("{p}_easy rc(mlen={mlen})"), ra, rb);
            assert_eq!(ra, 0);
            eq_bytes(&format!("{p}_easy(mlen={mlen})"), &c1, &c2);
            assert_eq!(&c1[mlen + 16..], &[0xA5u8; 4]);

            // must differ from the xsalsa20 variant for identical inputs
            let mut xs = canary(mlen + 16);
            unsafe {
                assert_eq!(
                    xs_easy(xs.as_mut_ptr(), m.as_ptr(), mlen as u64, n.as_ptr(),
                            pk_b.as_ptr(), sk_a.as_ptr()),
                    0
                )
            };
            assert_ne!(&xs[..], &c1[..mlen + 16], "xchacha20 must differ from xsalsa20");

            // --- easy_afternm (G5-030)
            let mut e1 = canary(mlen + 16 + 4);
            let mut e2 = canary(mlen + 16 + 4);
            unsafe {
                let x = easya.0(e1.as_mut_ptr(), m.as_ptr(), mlen as u64, n.as_ptr(), k.as_ptr());
                let y = easya.1(e2.as_mut_ptr(), m.as_ptr(), mlen as u64, n.as_ptr(), k.as_ptr());
                eq_i32(&format!("{p}_easy_afternm rc"), x, y);
                assert_eq!(x, 0);
            }
            eq_bytes(&format!("{p}_easy_afternm(mlen={mlen})"), &e1, &e2);
            assert_eq!(e1, c1, "{p}_easy_afternm != _easy");

            // --- detached (G5-031) and detached_afternm (G5-032)
            for (what, is_after) in [(format!("{p}_detached"), false), (format!("{p}_detached_afternm"), true)] {
                let mut d1 = canary(mlen.max(1) + 4);
                let mut d2 = canary(mlen.max(1) + 4);
                let mut t1 = canary(16 + 4);
                let mut t2 = canary(16 + 4);
                let (x, y) = unsafe {
                    if is_after {
                        (
                            deta.0(d1.as_mut_ptr(), t1.as_mut_ptr(), m.as_ptr(), mlen as u64,
                                   n.as_ptr(), k.as_ptr()),
                            deta.1(d2.as_mut_ptr(), t2.as_mut_ptr(), m.as_ptr(), mlen as u64,
                                   n.as_ptr(), k.as_ptr()),
                        )
                    } else {
                        (
                            det.0(d1.as_mut_ptr(), t1.as_mut_ptr(), m.as_ptr(), mlen as u64,
                                  n.as_ptr(), pk_b.as_ptr(), sk_a.as_ptr()),
                            det.1(d2.as_mut_ptr(), t2.as_mut_ptr(), m.as_ptr(), mlen as u64,
                                  n.as_ptr(), pk_b.as_ptr(), sk_a.as_ptr()),
                        )
                    }
                };
                eq_i32(&format!("{what} rc(mlen={mlen})"), x, y);
                assert_eq!(x, 0);
                eq_bytes(&format!("{what} c(mlen={mlen})"), &d1, &d2);
                eq_bytes(&format!("{what} mac(mlen={mlen})"), &t1, &t2);
                assert_eq!(&t1[..16], &c1[..16], "{what}: mac != easy c[0..16]");
                assert_eq!(&d1[..mlen], &c1[16..mlen + 16]);

                let mut q1 = canary(mlen.max(1) + 4);
                let mut q2 = canary(mlen.max(1) + 4);
                let (x, y) = unsafe {
                    if is_after {
                        (
                            odeta.0(q1.as_mut_ptr(), d1.as_ptr(), t1.as_ptr(), mlen as u64,
                                    n.as_ptr(), k.as_ptr()),
                            odeta.1(q2.as_mut_ptr(), d1.as_ptr(), t1.as_ptr(), mlen as u64,
                                    n.as_ptr(), k.as_ptr()),
                        )
                    } else {
                        (
                            odet.0(q1.as_mut_ptr(), d1.as_ptr(), t1.as_ptr(), mlen as u64,
                                   n.as_ptr(), pk_a.as_ptr(), sk_b.as_ptr()),
                            odet.1(q2.as_mut_ptr(), d1.as_ptr(), t1.as_ptr(), mlen as u64,
                                   n.as_ptr(), pk_a.as_ptr(), sk_b.as_ptr()),
                        )
                    }
                };
                eq_i32(&format!("{what} open rc(mlen={mlen})"), x, y);
                assert_eq!(x, 0);
                eq_bytes(&format!("{what} open(mlen={mlen})"), &q1, &q2);
                assert_eq!(&q1[..mlen], &m[..]);

                // verify-only mode
                let (x, y) = unsafe {
                    if is_after {
                        (
                            odeta.0(ptr::null_mut(), d1.as_ptr(), t1.as_ptr(), mlen as u64,
                                    n.as_ptr(), k.as_ptr()),
                            odeta.1(ptr::null_mut(), d1.as_ptr(), t1.as_ptr(), mlen as u64,
                                    n.as_ptr(), k.as_ptr()),
                        )
                    } else {
                        (
                            odet.0(ptr::null_mut(), d1.as_ptr(), t1.as_ptr(), mlen as u64,
                                   n.as_ptr(), pk_a.as_ptr(), sk_b.as_ptr()),
                            odet.1(ptr::null_mut(), d1.as_ptr(), t1.as_ptr(), mlen as u64,
                                   n.as_ptr(), pk_a.as_ptr(), sk_b.as_ptr()),
                        )
                    }
                };
                eq_i32(&format!("{what} open m=NULL rc"), x, y);
                assert_eq!(x, 0);
            }

            // --- open_easy / open_easy_afternm
            let mut p1 = canary(mlen.max(1));
            let mut p2 = canary(mlen.max(1));
            unsafe {
                let x = oeasy.0(p1.as_mut_ptr(), c1.as_ptr(), (mlen + 16) as u64,
                                n.as_ptr(), pk_a.as_ptr(), sk_b.as_ptr());
                let y = oeasy.1(p2.as_mut_ptr(), c1.as_ptr(), (mlen + 16) as u64,
                                n.as_ptr(), pk_a.as_ptr(), sk_b.as_ptr());
                eq_i32(&format!("{p}_open_easy rc"), x, y);
                assert_eq!(x, 0);
            }
            eq_bytes(&format!("{p}_open_easy(mlen={mlen})"), &p1, &p2);
            assert_eq!(&p1[..mlen], &m[..]);
            let mut p3 = canary(mlen.max(1));
            let mut p4 = canary(mlen.max(1));
            unsafe {
                let x = oeasya.0(p3.as_mut_ptr(), c1.as_ptr(), (mlen + 16) as u64,
                                 n.as_ptr(), k.as_ptr());
                let y = oeasya.1(p4.as_mut_ptr(), c1.as_ptr(), (mlen + 16) as u64,
                                 n.as_ptr(), k.as_ptr());
                eq_i32(&format!("{p}_open_easy_afternm rc"), x, y);
                assert_eq!(x, 0);
            }
            eq_bytes(&format!("{p}_open_easy_afternm(mlen={mlen})"), &p3, &p4);
            assert_eq!(&p3[..mlen], &m[..]);
        }
    }

    // --- sealed boxes (G5-033)
    for &mlen in &[0usize, 1, 16, 17, 32, 64, 65, 128, 1000] {
        for i in 0..3u64 {
            let (pk, sk) = box_kp_rng(&mut rng);
            let m = rng.bytes(mlen);
            let s = 0xB8A0 + i + mlen as u64;
            let mut c1 = canary(mlen + 48 + 4);
            let mut c2 = canary(mlen + 48 + 4);
            let (ra, rb) = {
                let _g = RNG_LOCK.lock().unwrap();
                reset_rngs(s);
                let ra = unsafe { seal.0(c1.as_mut_ptr(), m.as_ptr(), mlen as u64, pk.as_ptr()) };
                reset_rngs(s);
                let rb = unsafe { seal.1(c2.as_mut_ptr(), m.as_ptr(), mlen as u64, pk.as_ptr()) };
                (ra, rb)
            };
            eq_i32(&format!("{p}_seal rc(mlen={mlen})"), ra, rb);
            assert_eq!(ra, 0);
            eq_bytes(&format!("{p}_seal(mlen={mlen})"), &c1, &c2);
            assert_eq!(&c1[mlen + 48..], &[0xA5u8; 4]);

            let mut q1 = canary(mlen.max(1) + 4);
            let mut q2 = canary(mlen.max(1) + 4);
            let (x, y) = unsafe {
                (
                    sopen.0(q1.as_mut_ptr(), c1.as_ptr(), (mlen + 48) as u64,
                            pk.as_ptr(), sk.as_ptr()),
                    sopen.1(q2.as_mut_ptr(), c1.as_ptr(), (mlen + 48) as u64,
                            pk.as_ptr(), sk.as_ptr()),
                )
            };
            eq_i32(&format!("{p}_seal_open rc(mlen={mlen})"), x, y);
            assert_eq!(x, 0);
            eq_bytes(&format!("{p}_seal_open(mlen={mlen})"), &q1, &q2);
            assert_eq!(&q1[..mlen], &m[..]);
        }
    }
}

/// G5-034 — nonce shapes across every `crypto_box` layer: the first 16 bytes
/// feed the HSalsa20 / HChaCha20 subkey, the last 8 the stream nonce.
#[test]
fn box_nonce_shapes() {
    setup();
    let mut rng = Rng::new(0xB900);
    let (pk, sk) = box_kp(0x11);
    let (pk2, _sk2) = box_kp(0x22);
    let mlen = 100usize;
    let m = rng.bytes(mlen);

    let mut nonces: Vec<Vec<u8>> = vec![
        vec![0u8; 24],
        (0u8..24).collect(),
        vec![0xffu8; 24],
    ];
    // differ only in the subkey half, then only in the stream half
    let base = rng.bytes(24);
    let mut a = base.clone();
    a[3] ^= 0xff;
    let mut b = base.clone();
    b[20] ^= 0xff;
    nonces.push(base);
    nonces.push(a);
    nonces.push(b);

    for name in [
        "crypto_box_easy",
        "crypto_box_curve25519xchacha20poly1305_easy",
    ] {
        let (c, r) = pair::<Asym6>(name);
        let mut seen: Vec<Vec<u8>> = Vec::new();
        for n in &nonces {
            let mut c1 = canary(mlen + 16);
            let mut c2 = canary(mlen + 16);
            let (x, y) = unsafe {
                (
                    c(c1.as_mut_ptr(), m.as_ptr(), mlen as u64, n.as_ptr(),
                      pk2.as_ptr(), sk.as_ptr()),
                    r(c2.as_mut_ptr(), m.as_ptr(), mlen as u64, n.as_ptr(),
                      pk2.as_ptr(), sk.as_ptr()),
                )
            };
            eq_i32(&format!("{name} rc(n={})", hex(n)), x, y);
            assert_eq!(x, 0);
            eq_bytes(&format!("{name}(n={})", hex(n)), &c1, &c2);
            assert!(
                !seen.contains(&c1),
                "{name}: nonce {} reused an earlier ciphertext",
                hex(n)
            );
            seen.push(c1);
        }
    }
    // and the NaCl layer, whose padded plaintext must still round-trip
    let (cf, rf) = pair::<Asym6>("crypto_box");
    let mut padded = vec![0u8; 32];
    padded.extend_from_slice(&m);
    for n in &nonces {
        let mut c1 = canary(padded.len());
        let mut c2 = canary(padded.len());
        let (x, y) = unsafe {
            (
                cf(c1.as_mut_ptr(), padded.as_ptr(), padded.len() as u64, n.as_ptr(),
                   pk2.as_ptr(), sk.as_ptr()),
                rf(c2.as_mut_ptr(), padded.as_ptr(), padded.len() as u64, n.as_ptr(),
                   pk2.as_ptr(), sk.as_ptr()),
            )
        };
        eq_i32("crypto_box rc (nonce shapes)", x, y);
        assert_eq!(x, 0);
        eq_bytes(&format!("crypto_box(n={})", hex(n)), &c1, &c2);
    }
    let _ = pk;
}

// ===========================================================================
// crypto_secretbox
// ===========================================================================

/// G5-039, G5-040, G5-041, G5-042 — the NaCl padded secretbox API, generic and
/// explicit-primitive.
#[test]
fn secretbox_nacl_padded_api() {
    setup();
    let mut rng = Rng::new(0xC000);
    let seal = pair::<Sym5>("crypto_secretbox");
    let open = pair::<Sym5>("crypto_secretbox_open");
    let xseal = pair::<Sym5>("crypto_secretbox_xsalsa20poly1305");
    let xopen = pair::<Sym5>("crypto_secretbox_xsalsa20poly1305_open");

    // total mlen values from the table: payload + ZEROBYTES(32)
    for &mlen in &[32usize, 33, 47, 48, 49, 63, 64, 65, 95, 96, 97, 159, 160, 1032] {
        for _ in 0..3 {
            let k = rng.bytes(32);
            let n = rng.bytes(24);
            let mut m = vec![0u8; 32];
            m.extend_from_slice(&rng.bytes(mlen - 32));

            let mut c1 = canary(mlen + 4);
            let mut c2 = canary(mlen + 4);
            let (ra, rb) = unsafe {
                (
                    seal.0(c1.as_mut_ptr(), m.as_ptr(), mlen as u64, n.as_ptr(), k.as_ptr()),
                    seal.1(c2.as_mut_ptr(), m.as_ptr(), mlen as u64, n.as_ptr(), k.as_ptr()),
                )
            };
            eq_i32(&format!("crypto_secretbox rc(mlen={mlen})"), ra, rb);
            assert_eq!(ra, 0);
            eq_bytes(&format!("crypto_secretbox(mlen={mlen})"), &c1, &c2);
            assert_eq!(&c1[..16], &[0u8; 16][..], "c[0..16] must be zero");
            assert_eq!(&c1[mlen..], &[0xA5u8; 4], "wrote past mlen");

            let mut c3 = canary(mlen + 4);
            let mut c4 = canary(mlen + 4);
            unsafe {
                let x = xseal.0(c3.as_mut_ptr(), m.as_ptr(), mlen as u64, n.as_ptr(), k.as_ptr());
                let y = xseal.1(c4.as_mut_ptr(), m.as_ptr(), mlen as u64, n.as_ptr(), k.as_ptr());
                eq_i32("crypto_secretbox_xsalsa20poly1305 rc", x, y);
                assert_eq!(x, 0);
            }
            eq_bytes("crypto_secretbox_xsalsa20poly1305", &c3, &c4);
            assert_eq!(c1, c3, "generic != explicit primitive");

            for (what, f) in [
                ("crypto_secretbox_open", open),
                ("crypto_secretbox_xsalsa20poly1305_open", xopen),
            ] {
                let mut m1 = canary(mlen + 4);
                let mut m2 = canary(mlen + 4);
                let (x, y) = unsafe {
                    (
                        f.0(m1.as_mut_ptr(), c1.as_ptr(), mlen as u64, n.as_ptr(), k.as_ptr()),
                        f.1(m2.as_mut_ptr(), c1.as_ptr(), mlen as u64, n.as_ptr(), k.as_ptr()),
                    )
                };
                eq_i32(&format!("{what} rc(mlen={mlen})"), x, y);
                assert_eq!(x, 0);
                eq_bytes(&format!("{what}(mlen={mlen})"), &m1, &m2);
                assert_eq!(&m1[..mlen], &m[..], "{what} plaintext");
                assert_eq!(&m1[..32], &[0u8; 32][..], "{what} must zero m[0..32]");
                assert_eq!(&m1[mlen..], &[0xA5u8; 4]);
            }
        }
    }
}

/// G5-043, G5-044, G5-045, G5-046, G5-050, G5-051, G5-052, G5-053, G5-054 —
/// the easy and detached layers of BOTH secretbox primitives, including
/// verify-only mode.
#[test]
fn secretbox_easy_and_detached() {
    setup();
    let mut rng = Rng::new(0xC100);
    for prim in ["crypto_secretbox", "crypto_secretbox_xchacha20poly1305"] {
        let easy = pair::<Sym5>(&format!("{prim}_easy"));
        let oeasy = pair::<Sym5>(&format!("{prim}_open_easy"));
        let det = pair::<Det6>(&format!("{prim}_detached"));
        let odet = pair::<ODet6>(&format!("{prim}_open_detached"));
        for &mlen in MLENS {
            for _ in 0..3 {
                let k = rng.bytes(32);
                let n = rng.bytes(24);
                let m = rng.bytes(mlen);

                let mut c1 = canary(mlen + 16 + 4);
                let mut c2 = canary(mlen + 16 + 4);
                let (ra, rb) = unsafe {
                    (
                        easy.0(c1.as_mut_ptr(), m.as_ptr(), mlen as u64, n.as_ptr(), k.as_ptr()),
                        easy.1(c2.as_mut_ptr(), m.as_ptr(), mlen as u64, n.as_ptr(), k.as_ptr()),
                    )
                };
                eq_i32(&format!("{prim}_easy rc(mlen={mlen})"), ra, rb);
                assert_eq!(ra, 0);
                eq_bytes(&format!("{prim}_easy(mlen={mlen})"), &c1, &c2);
                assert_eq!(&c1[mlen + 16..], &[0xA5u8; 4]);

                // detached: mac ‖ c must reproduce the easy output
                let mut d1 = canary(mlen.max(1) + 4);
                let mut d2 = canary(mlen.max(1) + 4);
                let mut t1 = canary(16 + 4);
                let mut t2 = canary(16 + 4);
                let (x, y) = unsafe {
                    (
                        det.0(d1.as_mut_ptr(), t1.as_mut_ptr(), m.as_ptr(), mlen as u64,
                              n.as_ptr(), k.as_ptr()),
                        det.1(d2.as_mut_ptr(), t2.as_mut_ptr(), m.as_ptr(), mlen as u64,
                              n.as_ptr(), k.as_ptr()),
                    )
                };
                eq_i32(&format!("{prim}_detached rc(mlen={mlen})"), x, y);
                assert_eq!(x, 0);
                eq_bytes(&format!("{prim}_detached c(mlen={mlen})"), &d1, &d2);
                eq_bytes(&format!("{prim}_detached mac(mlen={mlen})"), &t1, &t2);
                assert_eq!(&t1[..16], &c1[..16]);
                assert_eq!(&d1[..mlen], &c1[16..mlen + 16]);
                assert_eq!(&t1[16..], &[0xA5u8; 4]);

                // open_easy
                let mut p1 = canary(mlen.max(1) + 4);
                let mut p2 = canary(mlen.max(1) + 4);
                let (x, y) = unsafe {
                    (
                        oeasy.0(p1.as_mut_ptr(), c1.as_ptr(), (mlen + 16) as u64,
                                n.as_ptr(), k.as_ptr()),
                        oeasy.1(p2.as_mut_ptr(), c1.as_ptr(), (mlen + 16) as u64,
                                n.as_ptr(), k.as_ptr()),
                    )
                };
                eq_i32(&format!("{prim}_open_easy rc(mlen={mlen})"), x, y);
                assert_eq!(x, 0);
                eq_bytes(&format!("{prim}_open_easy(mlen={mlen})"), &p1, &p2);
                assert_eq!(&p1[..mlen], &m[..]);

                // open_detached
                let mut q1 = canary(mlen.max(1) + 4);
                let mut q2 = canary(mlen.max(1) + 4);
                let (x, y) = unsafe {
                    (
                        odet.0(q1.as_mut_ptr(), d1.as_ptr(), t1.as_ptr(), mlen as u64,
                               n.as_ptr(), k.as_ptr()),
                        odet.1(q2.as_mut_ptr(), d1.as_ptr(), t1.as_ptr(), mlen as u64,
                               n.as_ptr(), k.as_ptr()),
                    )
                };
                eq_i32(&format!("{prim}_open_detached rc(mlen={mlen})"), x, y);
                assert_eq!(x, 0);
                eq_bytes(&format!("{prim}_open_detached(mlen={mlen})"), &q1, &q2);
                assert_eq!(&q1[..mlen], &m[..]);

                // G5-050 / G5-054: m == NULL, verify only
                let (x, y) = unsafe {
                    (
                        odet.0(ptr::null_mut(), d1.as_ptr(), t1.as_ptr(), mlen as u64,
                               n.as_ptr(), k.as_ptr()),
                        odet.1(ptr::null_mut(), d1.as_ptr(), t1.as_ptr(), mlen as u64,
                               n.as_ptr(), k.as_ptr()),
                    )
                };
                eq_i32(&format!("{prim}_open_detached m=NULL rc(mlen={mlen})"), x, y);
                assert_eq!(x, 0);
            }
        }
    }
    // the two primitives must disagree for identical inputs
    let k = rng.bytes(32);
    let n = rng.bytes(24);
    let m = rng.bytes(100);
    let mut a = canary(116);
    let mut b = canary(116);
    unsafe {
        sym::<Sym5>(c_lib(), "crypto_secretbox_easy")(
            a.as_mut_ptr(), m.as_ptr(), 100, n.as_ptr(), k.as_ptr());
        sym::<Sym5>(c_lib(), "crypto_secretbox_xchacha20poly1305_easy")(
            b.as_mut_ptr(), m.as_ptr(), 100, n.as_ptr(), k.as_ptr());
    }
    assert_ne!(a, b, "xsalsa20 and xchacha20 secretbox must differ");
}

/// G5-047 — the encrypt-side 131072-byte `STREAM_POLY1305_CHUNK` restart of
/// `crypto_secretbox_detached` (its `_open_detached` has no chunking, and the
/// xchacha20 variant chunks in neither direction, so all four must agree).
#[test]
fn secretbox_chunk_boundary() {
    setup();
    let mut rng = Rng::new(0xC200);
    for prim in ["crypto_secretbox", "crypto_secretbox_xchacha20poly1305"] {
        let det = pair::<Det6>(&format!("{prim}_detached"));
        let odet = pair::<ODet6>(&format!("{prim}_open_detached"));
        for &mlen in &[
            32usize, 33, 131071, 131072, 131073, 131103, 131104, 131105, 262143, 262144,
            262176, 262177,
        ] {
            let k = rng.bytes(32);
            let n = rng.bytes(24);
            let m = rng.bytes(mlen);
            let mut c1 = vec![0xA5u8; mlen];
            let mut c2 = vec![0xA5u8; mlen];
            let mut t1 = canary(16);
            let mut t2 = canary(16);
            let (x, y) = unsafe {
                (
                    det.0(c1.as_mut_ptr(), t1.as_mut_ptr(), m.as_ptr(), mlen as u64,
                          n.as_ptr(), k.as_ptr()),
                    det.1(c2.as_mut_ptr(), t2.as_mut_ptr(), m.as_ptr(), mlen as u64,
                          n.as_ptr(), k.as_ptr()),
                )
            };
            eq_i32(&format!("{prim}_detached rc(mlen={mlen})"), x, y);
            assert_eq!(x, 0);
            eq_bytes(&format!("{prim}_detached chunk c(mlen={mlen})"), &c1, &c2);
            eq_bytes(&format!("{prim}_detached chunk mac(mlen={mlen})"), &t1, &t2);

            // the un-chunked open must invert the chunked encrypt
            let mut p1 = vec![0xA5u8; mlen];
            let mut p2 = vec![0xA5u8; mlen];
            let (x, y) = unsafe {
                (
                    odet.0(p1.as_mut_ptr(), c1.as_ptr(), t1.as_ptr(), mlen as u64,
                           n.as_ptr(), k.as_ptr()),
                    odet.1(p2.as_mut_ptr(), c1.as_ptr(), t1.as_ptr(), mlen as u64,
                           n.as_ptr(), k.as_ptr()),
                )
            };
            eq_i32(&format!("{prim}_open_detached rc(mlen={mlen})"), x, y);
            assert_eq!(x, 0, "{prim} chunk round trip (mlen={mlen})");
            eq_bytes(&format!("{prim}_open_detached chunk(mlen={mlen})"), &p1, &p2);
            assert_eq!(p1, m);
        }
    }
}

/// G5-048, G5-049, G5-055 — in-place and partially overlapping `c`/`m` in both
/// directions, for both secretbox primitives, encrypt and decrypt.
#[test]
fn secretbox_overlapping_buffers() {
    setup();
    let mut rng = Rng::new(0xC300);
    for prim in ["crypto_secretbox", "crypto_secretbox_xchacha20poly1305"] {
        let det = pair::<Det6>(&format!("{prim}_detached"));
        let odet = pair::<ODet6>(&format!("{prim}_open_detached"));
        let easy = pair::<Sym5>(&format!("{prim}_easy"));
        let oeasy = pair::<Sym5>(&format!("{prim}_open_easy"));
        for &mlen in &[1usize, 16, 32, 33, 64, 65, 1000] {
            let k = rng.bytes(32);
            let n = rng.bytes(24);
            let m = rng.bytes(mlen);

            // disjoint reference
            let mut cref = canary(mlen.max(1));
            let mut tref = canary(16);
            unsafe {
                assert_eq!(
                    det.0(cref.as_mut_ptr(), tref.as_mut_ptr(), m.as_ptr(), mlen as u64,
                          n.as_ptr(), k.as_ptr()),
                    0
                )
            };

            // shifts: 0 = fully in place, then partial overlaps both ways
            let shifts: Vec<usize> = if mlen > 1 {
                vec![0, 1, mlen / 2, mlen - 1]
            } else {
                vec![0]
            };
            for &d in &shifts {
                for c_after_m in [true, false] {
                    if d == 0 && !c_after_m {
                        continue; // same configuration as (0, true)
                    }
                    for which in 0..2usize {
                        let f = if which == 0 { det.0 } else { det.1 };
                        let mut buf = vec![0xA5u8; mlen + d];
                        let (coff, moff) = if c_after_m { (d, 0) } else { (0, d) };
                        buf[moff..moff + mlen].copy_from_slice(&m);
                        let mut tag = canary(16);
                        let rc = unsafe {
                            let p = buf.as_mut_ptr();
                            f(p.add(coff), tag.as_mut_ptr(), p.add(moff) as *const u8,
                              mlen as u64, n.as_ptr(), k.as_ptr())
                        };
                        assert_eq!(rc, 0, "{prim}_detached overlap rc");
                        assert_eq!(
                            &buf[coff..coff + mlen],
                            &cref[..mlen],
                            "{prim}_detached overlap(d={d},c_after_m={c_after_m},lib={which},mlen={mlen}) c"
                        );
                        assert_eq!(
                            &tag[..], &tref[..],
                            "{prim}_detached overlap(d={d},c_after_m={c_after_m},lib={which}) mac"
                        );
                    }

                    // decrypt side (G5-049): m and c overlapping
                    for which in 0..2usize {
                        let f = if which == 0 { odet.0 } else { odet.1 };
                        let mut buf = vec![0xA5u8; mlen + d];
                        let (coff, moff) = if c_after_m { (d, 0) } else { (0, d) };
                        buf[coff..coff + mlen].copy_from_slice(&cref[..mlen]);
                        let rc = unsafe {
                            let p = buf.as_mut_ptr();
                            f(p.add(moff), p.add(coff) as *const u8, tref.as_ptr(),
                              mlen as u64, n.as_ptr(), k.as_ptr())
                        };
                        assert_eq!(rc, 0, "{prim}_open_detached overlap rc");
                        assert_eq!(
                            &buf[moff..moff + mlen], &m[..],
                            "{prim}_open_detached overlap(d={d},c_after_m={c_after_m},lib={which},mlen={mlen})"
                        );
                    }
                }
            }

            // the classic easy in-place shape: c == m, buffer mlen+16.
            // NOTE with c == m the inner `_detached` sees c' = c+16 and m = c,
            // so `c' - m == 16`; the memmove only fires for mlen > 16.
            let mut enc: Vec<Vec<u8>> = Vec::new();
            for which in 0..2usize {
                let f = if which == 0 { easy.0 } else { easy.1 };
                let mut buf = canary(mlen + 16);
                buf[..mlen].copy_from_slice(&m);
                let rc = unsafe {
                    f(buf.as_mut_ptr(), buf.as_ptr(), mlen as u64, n.as_ptr(), k.as_ptr())
                };
                assert_eq!(rc, 0, "{prim}_easy in-place rc");
                enc.push(buf);
            }
            eq_bytes(&format!("{prim}_easy in-place(mlen={mlen})"), &enc[0], &enc[1]);

            let mut dec: Vec<Vec<u8>> = Vec::new();
            for which in 0..2usize {
                let g = if which == 0 { oeasy.0 } else { oeasy.1 };
                let mut buf = enc[0].clone();
                let rc = unsafe {
                    g(buf.as_mut_ptr(), buf.as_ptr(), (mlen + 16) as u64, n.as_ptr(), k.as_ptr())
                };
                assert_eq!(rc, 0, "{prim}_open_easy in-place rc");
                dec.push(buf);
            }
            eq_bytes(&format!("{prim}_open_easy in-place(mlen={mlen})"), &dec[0], &dec[1]);
            assert_eq!(&dec[0][..mlen], &m[..], "{prim} easy in-place round trip");
        }
    }
}

// ===========================================================================
// crypto_secretstream_xchacha20poly1305
// ===========================================================================

const SS: &str = "crypto_secretstream_xchacha20poly1305";

struct SsApi {
    init_push: (SsInit, SsInit),
    init_pull: (SsInitPull, SsInitPull),
    push: (SsPush, SsPush),
    pull: (SsPull, SsPull),
    rekey: (SsRekey, SsRekey),
}

fn ss_api() -> SsApi {
    SsApi {
        init_push: pair::<SsInit>(&format!("{SS}_init_push")),
        init_pull: pair::<SsInitPull>(&format!("{SS}_init_pull")),
        push: pair::<SsPush>(&format!("{SS}_push")),
        pull: pair::<SsPull>(&format!("{SS}_pull")),
        rekey: pair::<SsRekey>(&format!("{SS}_rekey")),
    }
}

/// Two independent state buffers (one per library) initialised from the same
/// header with `_init_pull` — which produces exactly the `_init_push` state.
fn ss_pull_states(api: &SsApi, hdr: &[u8], k: &[u8]) -> (State, State) {
    let mut a = State::for_sym(&format!("{SS}_statebytes"));
    let mut b = State::for_sym(&format!("{SS}_statebytes"));
    unsafe {
        assert_eq!(api.init_pull.0(a.as_mut_ptr(), hdr.as_ptr(), k.as_ptr()), 0);
        assert_eq!(api.init_pull.1(b.as_mut_ptr(), hdr.as_ptr(), k.as_ptr()), 0);
    }
    eq_bytes("init_pull state", a.bytes(), b.bytes());
    (a, b)
}

/// An independent re-implementation of `_push` built from the C library's
/// chacha20-ietf and poly1305 primitives, so the exact AD padding, the
/// (buggy) message padding `(0x10 - 64 + mlen) & 0xf` and the
/// `adlen ‖ 64+mlen` LE64 trailer are pinned normatively.
fn ss_ref_push(state: &[u8], m: &[u8], ad: &[u8], tag: u8) -> Vec<u8> {
    let k = &state[..32];
    let nonce = &state[32..44];
    let chacha = sym::<Chacha>(c_lib(), "crypto_stream_chacha20_ietf");
    let xor_ic = sym::<ChachaXorIc>(c_lib(), "crypto_stream_chacha20_ietf_xor_ic");
    let pi = sym::<P1305Init>(c_lib(), "crypto_onetimeauth_poly1305_init");
    let pu = sym::<P1305Update>(c_lib(), "crypto_onetimeauth_poly1305_update");
    let pf = sym::<P1305Final>(c_lib(), "crypto_onetimeauth_poly1305_final");
    let mut st = State::for_sym("crypto_onetimeauth_poly1305_statebytes");
    let pad0 = [0u8; 16];
    let mut block = [0u8; 64];
    let mlen = m.len() as u64;
    let adlen = ad.len() as u64;
    unsafe {
        assert_eq!(chacha(block.as_mut_ptr(), 64, nonce.as_ptr(), k.as_ptr()), 0);
        assert_eq!(pi(st.as_mut_ptr(), block.as_ptr()), 0);
        pu(st.as_mut_ptr(), ad.as_ptr(), adlen);
        pu(st.as_mut_ptr(), pad0.as_ptr(), 0x10u64.wrapping_sub(adlen) & 0xf);
    }
    block = [0u8; 64];
    block[0] = tag;
    let mut out = vec![0u8; 1 + m.len() + 16];
    unsafe {
        xor_ic(block.as_mut_ptr(), block.as_ptr(), 64, nonce.as_ptr(), 1, k.as_ptr());
        pu(st.as_mut_ptr(), block.as_ptr(), 64);
        out[0] = block[0];
        xor_ic(out.as_mut_ptr().add(1), m.as_ptr(), mlen, nonce.as_ptr(), 2, k.as_ptr());
        pu(st.as_mut_ptr(), out.as_ptr().add(1), mlen);
        pu(
            st.as_mut_ptr(),
            pad0.as_ptr(),
            0x10u64.wrapping_sub(64).wrapping_add(mlen) & 0xf,
        );
        let s1 = adlen.to_le_bytes();
        pu(st.as_mut_ptr(), s1.as_ptr(), 8);
        let s2 = 64u64.wrapping_add(mlen).to_le_bytes();
        pu(st.as_mut_ptr(), s2.as_ptr(), 8);
        let mut mac = [0u8; 16];
        assert_eq!(pf(st.as_mut_ptr(), mac.as_mut_ptr()), 0);
        out[1 + m.len()..].copy_from_slice(&mac);
    }
    out
}

/// G5-059, G5-060 — `_init_push` vs `_init_pull`: identical states, and the
/// documented state layout (`k = HChaCha20(header[0..16], k)`, counter = 1,
/// inonce = `header[16..24]`, `_pad` zeroed).
#[test]
fn secretstream_init_push_pull() {
    setup();
    let api = ss_api();
    let hchacha = sym::<unsafe extern "C" fn(*mut u8, *const u8, *const u8, *const u8) -> i32>(
        c_lib(),
        "crypto_core_hchacha20",
    );
    let mut rng = Rng::new(0xD000);
    let mut keys: Vec<Vec<u8>> = vec![vec![0u8; 32], vec![0xffu8; 32]];
    for _ in 0..6 {
        keys.push(rng.bytes(32));
    }
    for k in &keys {
        for i in 0..4u64 {
            let s = 0xD010 + i;
            let mut h1 = canary(24 + 4);
            let mut h2 = canary(24 + 4);
            let mut st1 = State::for_sym(&format!("{SS}_statebytes"));
            let mut st2 = State::for_sym(&format!("{SS}_statebytes"));
            let (ra, rb) = {
                let _g = RNG_LOCK.lock().unwrap();
                reset_rngs(s);
                let ra = unsafe { api.init_push.0(st1.as_mut_ptr(), h1.as_mut_ptr(), k.as_ptr()) };
                reset_rngs(s);
                let rb = unsafe { api.init_push.1(st2.as_mut_ptr(), h2.as_mut_ptr(), k.as_ptr()) };
                (ra, rb)
            };
            eq_i32("init_push rc", ra, rb);
            assert_eq!(ra, 0);
            eq_bytes("init_push header", &h1, &h2);
            eq_bytes("init_push state", st1.bytes(), st2.bytes());
            assert_eq!(&h1[24..], &[0xA5u8; 4], "init_push wrote past 24 bytes");

            // documented layout
            let mut want_k = [0u8; 32];
            unsafe { hchacha(want_k.as_mut_ptr(), h1.as_ptr(), k.as_ptr(), ptr::null()) };
            let st = st1.bytes();
            assert_eq!(&st[..32], &want_k[..], "state->k must be HChaCha20(hdr, k)");
            assert_eq!(&st[32..36], &[1u8, 0, 0, 0], "counter must start at 1");
            assert_eq!(&st[36..44], &h1[16..24], "inonce must be header[16..24]");
            assert_eq!(&st[44..52], &[0u8; 8], "_pad must be zeroed");

            // _init_pull with that header reproduces the push state exactly
            let (p1, p2) = ss_pull_states(&api, &h1[..24], k);
            eq_bytes("init_pull == init_push state", st1.bytes(), p1.bytes());
            eq_bytes("init_pull state (both libs)", p1.bytes(), p2.bytes());
        }
    }
}

/// G5-061, G5-062, G5-063, G5-064, G5-065, G5-066, G5-067 — one push/pull per
/// fresh stream over the whole (tag × adlen × mlen) grid, checked against the
/// independent reference construction, with the post-operation state compared
/// so the implicit rekey for `tag & TAG_REKEY` is pinned too.
#[test]
fn secretstream_single_message_grid() {
    setup();
    let api = ss_api();
    let mut rng = Rng::new(0xD100);
    let mlens: &[usize] = &[0, 1, 15, 16, 17, 31, 63, 64, 65, 1000];
    let adlens: &[usize] = &[0, 1, 16, 100];
    let mut macs_per_ad: Vec<Vec<u8>> = Vec::new();
    for &tag in &[0u8, 1, 2, 3] {
        for &adlen in adlens {
            for &mlen in mlens {
                let k = rng.bytes(32);
                let hdr = rng.bytes(24);
                let m = rng.bytes(mlen);
                let ad = rng.bytes(adlen);
                let (mut ps, mut ps2) = ss_pull_states(&api, &hdr, &k);
                let before = ps.bytes().to_vec();

                let mut c1 = canary(mlen + 17 + 4);
                let mut c2 = canary(mlen + 17 + 4);
                let mut l1 = 0xDEADu64;
                let mut l2 = 0xDEADu64;
                let adp = if adlen == 0 { ptr::null() } else { ad.as_ptr() };
                let (ra, rb) = unsafe {
                    (
                        api.push.0(ps.as_mut_ptr(), c1.as_mut_ptr(), &mut l1, m.as_ptr(),
                                   mlen as u64, adp, adlen as u64, tag),
                        api.push.1(ps2.as_mut_ptr(), c2.as_mut_ptr(), &mut l2, m.as_ptr(),
                                   mlen as u64, adp, adlen as u64, tag),
                    )
                };
                let what = format!("push(tag={tag},ad={adlen},m={mlen})");
                eq_i32(&format!("{what} rc"), ra, rb);
                assert_eq!(ra, 0);
                eq_usize(&format!("{what} *clen_p"), l1 as usize, l2 as usize);
                assert_eq!(l1 as usize, mlen + 17);
                eq_bytes(&what, &c1, &c2);
                assert_eq!(&c1[mlen + 17..], &[0xA5u8; 4], "{what} wrote past mlen+17");
                eq_bytes(&format!("{what} state"), ps.bytes(), ps2.bytes());

                // independent reference: pins the padding + trailer formulas
                let want = ss_ref_push(&before, &m, &ad, tag);
                eq_bytes(&format!("{what} vs reference"), &want, &c1[..mlen + 17]);

                // tag & TAG_REKEY implies the implicit rekey ran: `k` was
                // replaced and the counter was reset to 1. Otherwise the
                // counter simply advanced 1 -> 2 and `k` is unchanged.
                let after = ps.bytes().to_vec();
                if tag & 2 != 0 {
                    assert_eq!(&after[32..36], &[1u8, 0, 0, 0], "{what}: rekey resets counter");
                    assert_ne!(&after[..32], &before[..32], "{what}: rekey must change k");
                } else {
                    assert_eq!(&after[32..36], &[2u8, 0, 0, 0], "{what}: counter 1 -> 2");
                    assert_eq!(&after[..32], &before[..32], "{what}: k must be unchanged");
                }
                assert_ne!(&after[36..44], &before[36..44], "{what}: inonce must change");
                assert_eq!(&after[44..], &[0u8; 8], "{what}: _pad must stay zero");

                // ---- pull it back on a matching pair of states
                let (mut qs, mut qs2) = ss_pull_states(&api, &hdr, &k);
                let mut m1 = canary(mlen.max(1) + 4);
                let mut m2 = canary(mlen.max(1) + 4);
                let mut o1 = 0xDEADu64;
                let mut o2 = 0xDEADu64;
                let mut t1 = 0xA5u8;
                let mut t2 = 0xA5u8;
                let (x, y) = unsafe {
                    (
                        api.pull.0(qs.as_mut_ptr(), m1.as_mut_ptr(), &mut o1, &mut t1,
                                   c1.as_ptr(), (mlen + 17) as u64, adp, adlen as u64),
                        api.pull.1(qs2.as_mut_ptr(), m2.as_mut_ptr(), &mut o2, &mut t2,
                                   c1.as_ptr(), (mlen + 17) as u64, adp, adlen as u64),
                    )
                };
                eq_i32(&format!("pull(tag={tag},ad={adlen},m={mlen}) rc"), x, y);
                assert_eq!(x, 0, "pull(tag={tag},ad={adlen},m={mlen}) must succeed");
                eq_usize("pull *mlen_p", o1 as usize, o2 as usize);
                assert_eq!(o1 as usize, mlen);
                assert_eq!((t1, t2), (tag, tag), "pull *tag_p");
                eq_bytes(&format!("pull(tag={tag},ad={adlen},m={mlen})"), &m1, &m2);
                assert_eq!(&m1[..mlen], &m[..]);
                assert_eq!(&m1[mlen.max(1)..], &[0xA5u8; 4]);
                eq_bytes("pull state", qs.bytes(), qs2.bytes());
                // push and pull states must stay in lock step
                eq_bytes("push state == pull state", ps.bytes(), qs.bytes());

                if tag == 0 && mlen == 64 {
                    macs_per_ad.push(c1[mlen + 1..mlen + 17].to_vec());
                }
            }
        }
    }
    // G5-065: the four AD lengths give four different MACs
    macs_per_ad.sort();
    let n = macs_per_ad.len();
    macs_per_ad.dedup();
    assert_eq!(macs_per_ad.len(), n, "AD lengths must produce distinct MACs");
}

/// G5-068, G5-069, G5-070, G5-071 — multi-message streams (2 / 3 / 10
/// messages), including a mid-stream `TAG_REKEY`.
#[test]
fn secretstream_multi_message_streams() {
    setup();
    let api = ss_api();
    let mut rng = Rng::new(0xD200);

    let scenarios: Vec<Vec<u8>> = vec![
        vec![0, 3],
        vec![0, 0, 3],
        vec![0, 0, 0, 0, 0, 0, 0, 0, 0, 3],
        vec![0, 0, 0, 0, 2, 0, 0, 0, 0, 3], // mid-stream TAG_REKEY at message 5
        vec![1, 1, 2, 0, 3],
        vec![2, 2, 2],
        vec![3, 3, 3], // pushing after TAG_FINAL is legal
    ];
    let mlen_cycle: &[usize] = &[0, 1, 16, 17, 63, 64, 65, 128, 1000];
    let adlen_cycle: &[usize] = &[0, 1, 16, 100];

    for tags in &scenarios {
        for rep in 0..3usize {
            let k = rng.bytes(32);
            let hdr = rng.bytes(24);
            let (mut ps, mut ps2) = ss_pull_states(&api, &hdr, &k);
            let (mut qs, mut qs2) = ss_pull_states(&api, &hdr, &k);
            let mut sent: Vec<(Vec<u8>, Vec<u8>, Vec<u8>, u8)> = Vec::new();
            for (i, &tag) in tags.iter().enumerate() {
                let mlen = mlen_cycle[(i + rep) % mlen_cycle.len()];
                let adlen = adlen_cycle[(i + rep) % adlen_cycle.len()];
                let m = rng.bytes(mlen);
                let ad = rng.bytes(adlen);
                let before = ps.bytes().to_vec();
                let mut c1 = canary(mlen + 17);
                let mut c2 = canary(mlen + 17);
                let mut l1 = 0u64;
                let mut l2 = 0u64;
                let (ra, rb) = unsafe {
                    (
                        api.push.0(ps.as_mut_ptr(), c1.as_mut_ptr(), &mut l1, m.as_ptr(),
                                   mlen as u64, ad.as_ptr(), adlen as u64, tag),
                        api.push.1(ps2.as_mut_ptr(), c2.as_mut_ptr(), &mut l2, m.as_ptr(),
                                   mlen as u64, ad.as_ptr(), adlen as u64, tag),
                    )
                };
                let what = format!("stream{tags:?} msg{i}(tag={tag},m={mlen},ad={adlen})");
                eq_i32(&format!("{what} push rc"), ra, rb);
                assert_eq!(ra, 0);
                eq_usize(&format!("{what} *clen_p"), l1 as usize, l2 as usize);
                eq_bytes(&format!("{what} push"), &c1, &c2);
                eq_bytes(&format!("{what} push state"), ps.bytes(), ps2.bytes());
                eq_bytes(
                    &format!("{what} push vs reference"),
                    &ss_ref_push(&before, &m, &ad, tag),
                    &c1,
                );
                sent.push((c1, m, ad, tag));
            }
            // pull them back in order
            for (i, (c, m, ad, tag)) in sent.iter().enumerate() {
                let mlen = m.len();
                let mut m1 = canary(mlen.max(1));
                let mut m2 = canary(mlen.max(1));
                let mut o1 = 0u64;
                let mut o2 = 0u64;
                let mut t1 = 0u8;
                let mut t2 = 0u8;
                let (x, y) = unsafe {
                    (
                        api.pull.0(qs.as_mut_ptr(), m1.as_mut_ptr(), &mut o1, &mut t1,
                                   c.as_ptr(), c.len() as u64, ad.as_ptr(), ad.len() as u64),
                        api.pull.1(qs2.as_mut_ptr(), m2.as_mut_ptr(), &mut o2, &mut t2,
                                   c.as_ptr(), c.len() as u64, ad.as_ptr(), ad.len() as u64),
                    )
                };
                let what = format!("stream{tags:?} msg{i} pull");
                eq_i32(&format!("{what} rc"), x, y);
                assert_eq!(x, 0, "{what} must succeed");
                eq_usize(&format!("{what} *mlen_p"), o1 as usize, o2 as usize);
                assert_eq!((t1, t2), (*tag, *tag), "{what} *tag_p");
                eq_bytes(&what, &m1, &m2);
                assert_eq!(&m1[..mlen], &m[..], "{what} plaintext");
                eq_bytes(&format!("{what} state"), qs.bytes(), qs2.bytes());
                if i + 1 == sent.len() {
                    // after the whole stream both directions must have arrived
                    // at exactly the same state
                    eq_bytes(&format!("{what} state == push state"), qs.bytes(), ps.bytes());
                }
            }
        }
    }
}

/// G5-072, G5-073 — the explicit `_rekey`: mid-stream, immediately after
/// `_init_push`, and twice in a row.
#[test]
fn secretstream_explicit_rekey() {
    setup();
    let api = ss_api();
    let mut rng = Rng::new(0xD300);
    // (messages before the rekey, number of consecutive rekeys)
    for &(before_n, rekeys) in &[(2usize, 1usize), (0, 1), (0, 2), (3, 2), (1, 3)] {
        for _ in 0..3 {
            let k = rng.bytes(32);
            let hdr = rng.bytes(24);
            let (mut ps, mut ps2) = ss_pull_states(&api, &hdr, &k);
            let (mut qs, mut qs2) = ss_pull_states(&api, &hdr, &k);
            let mlen = 64usize;
            let send = |ps: &mut State, ps2: &mut State, m: &[u8]| -> Vec<u8> {
                let mut c1 = canary(mlen + 17);
                let mut c2 = canary(mlen + 17);
                unsafe {
                    let a = api.push.0(ps.as_mut_ptr(), c1.as_mut_ptr(), ptr::null_mut(),
                                       m.as_ptr(), m.len() as u64, ptr::null(), 0, 0);
                    let b = api.push.1(ps2.as_mut_ptr(), c2.as_mut_ptr(), ptr::null_mut(),
                                       m.as_ptr(), m.len() as u64, ptr::null(), 0, 0);
                    eq_i32("rekey-stream push rc", a, b);
                    assert_eq!(a, 0);
                }
                eq_bytes("rekey-stream push", &c1, &c2);
                eq_bytes("rekey-stream push state", ps.bytes(), ps2.bytes());
                c1
            };
            let mut msgs: Vec<(Vec<u8>, Vec<u8>)> = Vec::new();
            for _ in 0..before_n {
                let m = rng.bytes(mlen);
                let c = send(&mut ps, &mut ps2, &m);
                msgs.push((c, m));
            }
            // explicit rekey on the push side, and on the pull side later
            for _ in 0..rekeys {
                let s_before = ps.bytes().to_vec();
                unsafe {
                    api.rekey.0(ps.as_mut_ptr());
                    api.rekey.1(ps2.as_mut_ptr());
                }
                eq_bytes("_rekey state", ps.bytes(), ps2.bytes());
                assert_ne!(ps.bytes(), &s_before[..], "_rekey must change the state");
                assert_eq!(&ps.bytes()[32..36], &[1u8, 0, 0, 0], "_rekey resets counter");
            }
            for _ in 0..5 {
                let m = rng.bytes(mlen);
                let c = send(&mut ps, &mut ps2, &m);
                msgs.push((c, m));
            }
            // replay on the pull side with the identical rekey placement
            for (i, (c, m)) in msgs.iter().enumerate() {
                if i == before_n {
                    for _ in 0..rekeys {
                        unsafe {
                            api.rekey.0(qs.as_mut_ptr());
                            api.rekey.1(qs2.as_mut_ptr());
                        }
                        eq_bytes("_rekey pull state", qs.bytes(), qs2.bytes());
                    }
                }
                let mut m1 = canary(mlen);
                let mut m2 = canary(mlen);
                let (x, y) = unsafe {
                    (
                        api.pull.0(qs.as_mut_ptr(), m1.as_mut_ptr(), ptr::null_mut(),
                                   ptr::null_mut(), c.as_ptr(), c.len() as u64, ptr::null(), 0),
                        api.pull.1(qs2.as_mut_ptr(), m2.as_mut_ptr(), ptr::null_mut(),
                                   ptr::null_mut(), c.as_ptr(), c.len() as u64, ptr::null(), 0),
                    )
                };
                eq_i32("rekey-stream pull rc", x, y);
                assert_eq!(x, 0, "rekey-stream msg{i} must decrypt");
                eq_bytes("rekey-stream pull", &m1, &m2);
                assert_eq!(&m1[..], &m[..]);
                eq_bytes("rekey-stream pull state", qs.bytes(), qs2.bytes());
            }
        }
    }
}

/// G5-074, G5-075, G5-076 — NULL vs non-NULL `clen_p` / `mlen_p` / `tag_p`,
/// and the fact that `_pull` has no verify-only (`m == NULL`) mode.
#[test]
fn secretstream_null_out_params() {
    setup();
    let api = ss_api();
    let mut rng = Rng::new(0xD400);
    for &mlen in &[0usize, 1, 64, 1000] {
        let k = rng.bytes(32);
        let hdr = rng.bytes(24);
        let m = rng.bytes(mlen);

        // G5-074: clen_p NULL vs non-NULL must give the identical ciphertext
        let mut cts: Vec<Vec<u8>> = Vec::new();
        for null_len in [false, true] {
            let (mut ps, mut ps2) = ss_pull_states(&api, &hdr, &k);
            let mut c1 = canary(mlen + 17);
            let mut c2 = canary(mlen + 17);
            let mut l1 = 0xDEADu64;
            let mut l2 = 0xDEADu64;
            let (p1, p2) = if null_len {
                (ptr::null_mut(), ptr::null_mut())
            } else {
                (&raw mut l1, &raw mut l2)
            };
            let (a, b) = unsafe {
                (
                    api.push.0(ps.as_mut_ptr(), c1.as_mut_ptr(), p1, m.as_ptr(),
                               mlen as u64, ptr::null(), 0, 0),
                    api.push.1(ps2.as_mut_ptr(), c2.as_mut_ptr(), p2, m.as_ptr(),
                               mlen as u64, ptr::null(), 0, 0),
                )
            };
            eq_i32(&format!("push clen_p_null={null_len} rc"), a, b);
            assert_eq!(a, 0);
            eq_usize("push *clen_p", l1 as usize, l2 as usize);
            if null_len {
                assert_eq!(l1, 0xDEAD, "clen_p == NULL must not write");
            } else {
                assert_eq!(l1 as usize, mlen + 17);
            }
            eq_bytes(&format!("push clen_p_null={null_len}"), &c1, &c2);
            eq_bytes("push state", ps.bytes(), ps2.bytes());
            cts.push(c1);
        }
        assert_eq!(cts[0], cts[1], "clen_p must not affect the ciphertext");
        let ct = cts[0].clone();

        // G5-075: all four mlen_p × tag_p combinations
        let mut plains: Vec<Vec<u8>> = Vec::new();
        for null_mlen in [false, true] {
            for null_tag in [false, true] {
                let (mut qs, mut qs2) = ss_pull_states(&api, &hdr, &k);
                let mut m1 = canary(mlen.max(1));
                let mut m2 = canary(mlen.max(1));
                let mut o1 = 0xDEADu64;
                let mut o2 = 0xDEADu64;
                let mut t1 = 0x5Au8;
                let mut t2 = 0x5Au8;
                let (op1, op2) = if null_mlen {
                    (ptr::null_mut(), ptr::null_mut())
                } else {
                    (&raw mut o1, &raw mut o2)
                };
                let (tp1, tp2) = if null_tag {
                    (ptr::null_mut(), ptr::null_mut())
                } else {
                    (&raw mut t1, &raw mut t2)
                };
                let (x, y) = unsafe {
                    (
                        api.pull.0(qs.as_mut_ptr(), m1.as_mut_ptr(), op1, tp1, ct.as_ptr(),
                                   ct.len() as u64, ptr::null(), 0),
                        api.pull.1(qs2.as_mut_ptr(), m2.as_mut_ptr(), op2, tp2, ct.as_ptr(),
                                   ct.len() as u64, ptr::null(), 0),
                    )
                };
                let what = format!("pull(mlen_p_null={null_mlen},tag_p_null={null_tag},m={mlen})");
                eq_i32(&format!("{what} rc"), x, y);
                assert_eq!(x, 0);
                eq_usize(&format!("{what} *mlen_p"), o1 as usize, o2 as usize);
                assert_eq!((t1, t2), (t1, t1), "tag agreement");
                assert_eq!(t1, t2, "{what} *tag_p");
                if null_mlen {
                    assert_eq!(o1, 0xDEAD, "{what}: mlen_p == NULL must not write");
                } else {
                    assert_eq!(o1 as usize, mlen);
                }
                if null_tag {
                    assert_eq!(t1, 0x5A, "{what}: tag_p == NULL must not write");
                } else {
                    assert_eq!(t1, 0, "{what}: tag must be TAG_MESSAGE");
                }
                eq_bytes(&what, &m1, &m2);
                eq_bytes(&format!("{what} state"), qs.bytes(), qs2.bytes());
                plains.push(m1[..mlen].to_vec());
            }
        }
        for p in &plains {
            assert_eq!(p, &m, "the plaintext must not depend on the NULL-ness of the out params");
        }
        // G5-076: `_pull` always writes `m` (there is no verify-only mode); an
        // `m == NULL` call would dereference NULL inside
        // crypto_stream_chacha20_ietf_xor_ic, so it is not constructible here.
        // What IS observable is that `m` is fully written on success:
        if mlen > 0 {
            assert_eq!(plains[0].len(), mlen);
        }
    }
}

/// G5-077 — aliasing: `_push` writes `out[0]` before XORing the body, so with
/// `m == out + 1` the body is encrypted in place. Both libraries must produce
/// byte-identical results.
#[test]
fn secretstream_in_place() {
    setup();
    let api = ss_api();
    let mut rng = Rng::new(0xD500);
    for &mlen in &[1usize, 16, 64, 65, 1000] {
        let k = rng.bytes(32);
        let hdr = rng.bytes(24);
        let m = rng.bytes(mlen);

        // m == out + 1: `c` inside `_push` is exactly `m`, a well-defined
        // in-place XOR.
        let mut outs: Vec<Vec<u8>> = Vec::new();
        for which in 0..2usize {
            let (mut a, mut b) = ss_pull_states(&api, &hdr, &k);
            let st = if which == 0 { &mut a } else { &mut b };
            let f = if which == 0 { api.push.0 } else { api.push.1 };
            let mut buf = canary(mlen + 17);
            buf[1..1 + mlen].copy_from_slice(&m);
            let rc = unsafe {
                f(st.as_mut_ptr(), buf.as_mut_ptr(), ptr::null_mut(),
                  buf.as_ptr().add(1), mlen as u64, ptr::null(), 0, 0)
            };
            assert_eq!(rc, 0, "push in-place rc");
            outs.push(buf);
        }
        eq_bytes(&format!("push m==out+1 (mlen={mlen})"), &outs[0], &outs[1]);
        // and it decrypts back to the plaintext
        {
            let (mut qs, _q2) = ss_pull_states(&api, &hdr, &k);
            let mut p = canary(mlen);
            let rc = unsafe {
                api.pull.0(qs.as_mut_ptr(), p.as_mut_ptr(), ptr::null_mut(), ptr::null_mut(),
                           outs[0].as_ptr(), (mlen + 17) as u64, ptr::null(), 0)
            };
            assert_eq!(rc, 0);
            assert_eq!(&p[..], &m[..], "in-place push must be a normal ciphertext");
        }

        // out == m: `out[0] = tag` clobbers m[0] before the body is read, and
        // the body XOR runs with dst = src + 1. Purely a record of behaviour.
        let mut outs2: Vec<Vec<u8>> = Vec::new();
        for which in 0..2usize {
            let (mut a, mut b) = ss_pull_states(&api, &hdr, &k);
            let st = if which == 0 { &mut a } else { &mut b };
            let f = if which == 0 { api.push.0 } else { api.push.1 };
            let mut buf = canary(mlen + 17);
            buf[..mlen].copy_from_slice(&m);
            let rc = unsafe {
                f(st.as_mut_ptr(), buf.as_mut_ptr(), ptr::null_mut(),
                  buf.as_ptr(), mlen as u64, ptr::null(), 0, 0)
            };
            assert_eq!(rc, 0, "push out==m rc");
            outs2.push(buf);
        }
        eq_bytes(&format!("push out==m (mlen={mlen})"), &outs2[0], &outs2[1]);

        // `_pull` in place: m == in (well-defined, dst = src)
        let mut outs3: Vec<Vec<u8>> = Vec::new();
        for which in 0..2usize {
            let (mut a, mut b) = ss_pull_states(&api, &hdr, &k);
            let st = if which == 0 { &mut a } else { &mut b };
            let f = if which == 0 { api.pull.0 } else { api.pull.1 };
            let mut buf = outs[0].clone();
            let rc = unsafe {
                f(st.as_mut_ptr(), buf.as_mut_ptr(), ptr::null_mut(), ptr::null_mut(),
                  buf.as_ptr(), (mlen + 17) as u64, ptr::null(), 0)
            };
            assert_eq!(rc, 0, "pull m==in rc");
            outs3.push(buf);
        }
        eq_bytes(&format!("pull m==in (mlen={mlen})"), &outs3[0], &outs3[1]);
    }
}

// ===========================================================================
// crypto_kx
// ===========================================================================

/// G5-081, G5-082, G5-083, G5-084, G5-085, G5-086, G5-087 — session-key
/// agreement, the `rx`/`tx` NULL retargeting, aliasing, and the fixed
/// `q ‖ client_pk ‖ server_pk` hash order in *both* directions.
#[test]
fn kx_session_keys() {
    setup();
    let mut rng = Rng::new(0xE000);
    let cli = pair::<KxSession>("crypto_kx_client_session_keys");
    let srv = pair::<KxSession>("crypto_kx_server_session_keys");
    let seed_kp = sym::<SeedKeypair>(c_lib(), "crypto_kx_seed_keypair");
    let smult = sym::<Three>(c_lib(), "crypto_scalarmult");
    let gh_init = sym::<unsafe extern "C" fn(*mut u8, *const u8, usize, usize) -> i32>(
        c_lib(), "crypto_generichash_init");
    let gh_upd = sym::<unsafe extern "C" fn(*mut u8, *const u8, u64) -> i32>(
        c_lib(), "crypto_generichash_update");
    let gh_fin = sym::<unsafe extern "C" fn(*mut u8, *mut u8, usize) -> i32>(
        c_lib(), "crypto_generichash_final");

    for _ in 0..24 {
        let cs = rng.bytes(32);
        let ss = rng.bytes(32);
        let mut cpk = [0u8; 32];
        let mut csk = [0u8; 32];
        let mut spk = [0u8; 32];
        let mut ssk = [0u8; 32];
        unsafe {
            assert_eq!(seed_kp(cpk.as_mut_ptr(), csk.as_mut_ptr(), cs.as_ptr()), 0);
            assert_eq!(seed_kp(spk.as_mut_ptr(), ssk.as_mut_ptr(), ss.as_ptr()), 0);
        }

        // reference: keys = BLAKE2b-64(q ‖ client_pk ‖ server_pk)  (G5-087)
        let mut q = [0u8; 32];
        unsafe { assert_eq!(smult(q.as_mut_ptr(), csk.as_ptr(), spk.as_ptr()), 0) };
        let mut q2 = [0u8; 32];
        unsafe { assert_eq!(smult(q2.as_mut_ptr(), ssk.as_ptr(), cpk.as_ptr()), 0) };
        assert_eq!(q, q2, "X25519 must agree in both directions");
        let mut keys = [0u8; 64];
        let mut st = State::new(cst("crypto_generichash_statebytes"));
        unsafe {
            assert_eq!(gh_init(st.as_mut_ptr(), ptr::null(), 0, 64), 0);
            assert_eq!(gh_upd(st.as_mut_ptr(), q.as_ptr(), 32), 0);
            assert_eq!(gh_upd(st.as_mut_ptr(), cpk.as_ptr(), 32), 0);
            assert_eq!(gh_upd(st.as_mut_ptr(), spk.as_ptr(), 32), 0);
            assert_eq!(gh_fin(st.as_mut_ptr(), keys.as_mut_ptr(), 64), 0);
        }

        // ---- G5-081: both pointers non-NULL
        let mut crx = [canary(32 + 4), canary(32 + 4)];
        let mut ctx = [canary(32 + 4), canary(32 + 4)];
        let mut srx = [canary(32 + 4), canary(32 + 4)];
        let mut stx = [canary(32 + 4), canary(32 + 4)];
        for which in 0..2usize {
            let f = if which == 0 { cli.0 } else { cli.1 };
            let g = if which == 0 { srv.0 } else { srv.1 };
            unsafe {
                assert_eq!(
                    f(crx[which].as_mut_ptr(), ctx[which].as_mut_ptr(), cpk.as_ptr(),
                      csk.as_ptr(), spk.as_ptr()),
                    0
                );
                assert_eq!(
                    g(srx[which].as_mut_ptr(), stx[which].as_mut_ptr(), spk.as_ptr(),
                      ssk.as_ptr(), cpk.as_ptr()),
                    0
                );
            }
        }
        eq_bytes("client_session_keys rx", &crx[0], &crx[1]);
        eq_bytes("client_session_keys tx", &ctx[0], &ctx[1]);
        eq_bytes("server_session_keys rx", &srx[0], &srx[1]);
        eq_bytes("server_session_keys tx", &stx[0], &stx[1]);
        assert_eq!(&crx[0][..32], &keys[..32], "client rx = keys[0..32]");
        assert_eq!(&ctx[0][..32], &keys[32..], "client tx = keys[32..64]");
        assert_eq!(&stx[0][..32], &keys[..32], "server tx = keys[0..32]");
        assert_eq!(&srx[0][..32], &keys[32..], "server rx = keys[32..64]");
        assert_eq!(&crx[0][..32], &stx[0][..32], "client_rx == server_tx");
        assert_eq!(&ctx[0][..32], &srx[0][..32], "client_tx == server_rx");
        for b in [&crx[0], &ctx[0], &srx[0], &stx[0]] {
            assert_eq!(&b[32..], &[0xA5u8; 4], "wrote past sessionkeybytes");
        }

        // ---- G5-082 … G5-086: NULL retargeting and aliasing.
        // In every one of these shapes the second write of the loop wins, so
        // the single surviving buffer holds keys[32..64].
        for (what, is_client, rx_null, tx_null, alias) in [
            ("client rx=NULL", true, true, false, false),
            ("client tx=NULL", true, false, true, false),
            ("server rx=NULL", false, true, false, false),
            ("server tx=NULL", false, false, true, false),
            ("client rx==tx", true, false, false, true),
            ("server rx==tx", false, false, false, true),
        ] {
            let mut outs: Vec<Vec<u8>> = Vec::new();
            for which in 0..2usize {
                let mut buf = canary(32 + 4);
                let (rp, tp) = {
                    let p = buf.as_mut_ptr();
                    if alias {
                        (p, p)
                    } else if rx_null {
                        (ptr::null_mut(), p)
                    } else if tx_null {
                        (p, ptr::null_mut())
                    } else {
                        unreachable!()
                    }
                };
                let rc = unsafe {
                    if is_client {
                        let f = if which == 0 { cli.0 } else { cli.1 };
                        f(rp, tp, cpk.as_ptr(), csk.as_ptr(), spk.as_ptr())
                    } else {
                        let g = if which == 0 { srv.0 } else { srv.1 };
                        g(rp, tp, spk.as_ptr(), ssk.as_ptr(), cpk.as_ptr())
                    }
                };
                assert_eq!(rc, 0, "{what} rc");
                outs.push(buf);
            }
            eq_bytes(what, &outs[0], &outs[1]);
            assert_eq!(
                &outs[0][..32], &keys[32..],
                "{what}: the surviving buffer must hold keys[32..64]"
            );
            assert_eq!(&outs[0][32..], &[0xA5u8; 4], "{what} wrote past 32 bytes");
        }
    }
}

// ===========================================================================
// crypto_sign — ed25519
// ===========================================================================

const SIGN_MLENS: &[usize] = &[0, 1, 31, 32, 33, 63, 64, 65, 127, 128, 1000];

/// G5-092, G5-093, G5-094, G5-095, G5-096, G5-097, G5-098, G5-099, G5-100 —
/// the combined `crypto_sign` / `crypto_sign_open` API.
#[test]
fn sign_combined_api() {
    setup();
    let mut rng = Rng::new(0xF000);
    let sg = pair::<Sign5>("crypto_sign");
    let op = pair::<Sign5>("crypto_sign_open");
    let esg = pair::<Sign5>("crypto_sign_ed25519");
    let eop = pair::<Sign5>("crypto_sign_ed25519_open");

    for &mlen in SIGN_MLENS {
        for _ in 0..3 {
            let (pk, sk) = sign_kp(&mut rng);
            let m = rng.bytes(mlen);

            // ---- sign, with and without smlen_p (G5-096)
            let mut sms: Vec<Vec<u8>> = Vec::new();
            for null_len in [false, true] {
                let mut s1 = canary(mlen + 64 + 4);
                let mut s2 = canary(mlen + 64 + 4);
                let mut l1 = 0xDEADu64;
                let mut l2 = 0xDEADu64;
                let (p1, p2) = if null_len {
                    (ptr::null_mut(), ptr::null_mut())
                } else {
                    (&raw mut l1, &raw mut l2)
                };
                let (ra, rb) = unsafe {
                    (
                        sg.0(s1.as_mut_ptr(), p1, m.as_ptr(), mlen as u64, sk.as_ptr()),
                        sg.1(s2.as_mut_ptr(), p2, m.as_ptr(), mlen as u64, sk.as_ptr()),
                    )
                };
                eq_i32(&format!("crypto_sign rc(mlen={mlen},null={null_len})"), ra, rb);
                assert_eq!(ra, 0);
                eq_usize("crypto_sign *smlen_p", l1 as usize, l2 as usize);
                if null_len {
                    assert_eq!(l1, 0xDEAD, "smlen_p == NULL must not be written");
                } else {
                    assert_eq!(l1 as usize, mlen + 64);
                }
                eq_bytes(&format!("crypto_sign(mlen={mlen},null={null_len})"), &s1, &s2);
                assert_eq!(&s1[mlen + 64..], &[0xA5u8; 4], "wrote past mlen+64");
                assert_eq!(&s1[64..mlen + 64], &m[..], "sm[64..] must be the message");
                sms.push(s1);
            }
            assert_eq!(sms[0], sms[1], "smlen_p must not affect the output");
            let sm = sms[0].clone();

            // explicit primitive must be byte-identical (G5-095)
            let mut e1 = canary(mlen + 64 + 4);
            let mut e2 = canary(mlen + 64 + 4);
            unsafe {
                let a = esg.0(e1.as_mut_ptr(), ptr::null_mut(), m.as_ptr(), mlen as u64, sk.as_ptr());
                let b = esg.1(e2.as_mut_ptr(), ptr::null_mut(), m.as_ptr(), mlen as u64, sk.as_ptr());
                eq_i32("crypto_sign_ed25519 rc", a, b);
                assert_eq!(a, 0);
            }
            eq_bytes("crypto_sign_ed25519", &e1, &e2);
            assert_eq!(e1, sm, "generic crypto_sign != crypto_sign_ed25519");

            // ---- open, all pointer shapes (G5-092..094, G5-097, G5-098)
            for (what, f) in [("crypto_sign_open", op), ("crypto_sign_ed25519_open", eop)] {
                for &(m_null, len_null) in
                    &[(false, false), (false, true), (true, false), (true, true)]
                {
                    let mut m1 = canary(mlen.max(1) + 4);
                    let mut m2 = canary(mlen.max(1) + 4);
                    let mut o1 = 0xDEADu64;
                    let mut o2 = 0xDEADu64;
                    let (mp1, mp2) = if m_null {
                        (ptr::null_mut(), ptr::null_mut())
                    } else {
                        (m1.as_mut_ptr(), m2.as_mut_ptr())
                    };
                    let (op1, op2) = if len_null {
                        (ptr::null_mut(), ptr::null_mut())
                    } else {
                        (&raw mut o1, &raw mut o2)
                    };
                    let (x, y) = unsafe {
                        (
                            f.0(mp1, op1, sm.as_ptr(), (mlen + 64) as u64, pk.as_ptr()),
                            f.1(mp2, op2, sm.as_ptr(), (mlen + 64) as u64, pk.as_ptr()),
                        )
                    };
                    let tag = format!("{what}(mlen={mlen},m_null={m_null},len_null={len_null})");
                    eq_i32(&format!("{tag} rc"), x, y);
                    assert_eq!(x, 0, "{tag} must verify");
                    eq_usize(&format!("{tag} *mlen_p"), o1 as usize, o2 as usize);
                    if len_null {
                        assert_eq!(o1, 0xDEAD, "{tag}: mlen_p == NULL must not be written");
                    } else {
                        assert_eq!(o1 as usize, mlen);
                    }
                    eq_bytes(&tag, &m1, &m2);
                    if m_null {
                        assert_eq!(m1, canary(mlen.max(1) + 4), "{tag}: m == NULL must not write");
                    } else {
                        assert_eq!(&m1[..mlen], &m[..], "{tag} plaintext");
                        assert_eq!(&m1[mlen.max(1)..], &[0xA5u8; 4]);
                    }
                }
            }

            // ---- G5-099: in-place open, m == sm
            let mut ip: Vec<Vec<u8>> = Vec::new();
            for which in 0..2usize {
                let f = if which == 0 { op.0 } else { op.1 };
                let mut buf = sm[..mlen + 64].to_vec();
                let mut l = 0u64;
                let rc = unsafe {
                    f(buf.as_mut_ptr(), &mut l, buf.as_ptr(), (mlen + 64) as u64, pk.as_ptr())
                };
                assert_eq!(rc, 0, "in-place open rc");
                assert_eq!(l as usize, mlen);
                ip.push(buf);
            }
            eq_bytes(&format!("crypto_sign_open in-place(mlen={mlen})"), &ip[0], &ip[1]);
            assert_eq!(&ip[0][..mlen], &m[..], "in-place open must shift by 64");

            // ---- G5-100: sign with sm and m overlapping
            for &moff in &[0usize, 64] {
                let mut ov: Vec<Vec<u8>> = Vec::new();
                for which in 0..2usize {
                    let f = if which == 0 { sg.0 } else { sg.1 };
                    let mut buf = canary(mlen + 64);
                    buf[moff..moff + mlen].copy_from_slice(&m);
                    let mut l = 0u64;
                    let rc = unsafe {
                        f(buf.as_mut_ptr(), &mut l, buf.as_ptr().add(moff), mlen as u64,
                          sk.as_ptr())
                    };
                    assert_eq!(rc, 0, "overlapping sign rc");
                    assert_eq!(l as usize, mlen + 64);
                    ov.push(buf);
                }
                eq_bytes(
                    &format!("crypto_sign overlapping(moff={moff},mlen={mlen})"),
                    &ov[0],
                    &ov[1],
                );
                assert_eq!(
                    &ov[0][..], &sm[..mlen + 64],
                    "overlapping sign must equal the disjoint result"
                );
            }
        }
    }
}

/// G5-101, G5-102, G5-103, G5-104, G5-105 — the detached API, its equality
/// with `crypto_sign`'s `sm[0..64]`, and full determinism (no `randombytes`).
#[test]
fn sign_detached_api() {
    setup();
    let mut rng = Rng::new(0xF100);
    let det = pair::<Sign5>("crypto_sign_detached");
    let ver = pair::<Verify4>("crypto_sign_verify_detached");
    let edet = pair::<Sign5>("crypto_sign_ed25519_detached");
    let ever = pair::<Verify4>("crypto_sign_ed25519_verify_detached");
    let comb = sym::<Sign5>(c_lib(), "crypto_sign");
    let rbuf = pair::<unsafe extern "C" fn(*mut u8, usize)>("randombytes_buf");

    for &mlen in SIGN_MLENS {
        for _ in 0..3 {
            let (pk, sk) = sign_kp(&mut rng);
            let m = rng.bytes(mlen);

            let mut sigs: Vec<Vec<u8>> = Vec::new();
            for null_len in [false, true] {
                let mut s1 = canary(64 + 4);
                let mut s2 = canary(64 + 4);
                let mut l1 = 0xDEADu64;
                let mut l2 = 0xDEADu64;
                let (p1, p2) = if null_len {
                    (ptr::null_mut(), ptr::null_mut())
                } else {
                    (&raw mut l1, &raw mut l2)
                };
                let (ra, rb) = unsafe {
                    (
                        det.0(s1.as_mut_ptr(), p1, m.as_ptr(), mlen as u64, sk.as_ptr()),
                        det.1(s2.as_mut_ptr(), p2, m.as_ptr(), mlen as u64, sk.as_ptr()),
                    )
                };
                eq_i32(&format!("crypto_sign_detached rc(mlen={mlen})"), ra, rb);
                assert_eq!(ra, 0);
                eq_usize("crypto_sign_detached *siglen_p", l1 as usize, l2 as usize);
                if null_len {
                    assert_eq!(l1, 0xDEAD, "siglen_p == NULL must not be written");
                } else {
                    assert_eq!(l1, 64);
                }
                eq_bytes(&format!("crypto_sign_detached(mlen={mlen})"), &s1, &s2);
                assert_eq!(&s1[64..], &[0xA5u8; 4], "wrote past 64 bytes");
                sigs.push(s1);
            }
            assert_eq!(sigs[0], sigs[1], "siglen_p must not affect the signature");
            let sig = sigs[0][..64].to_vec();

            // explicit primitive (G5-103)
            let mut e1 = canary(64);
            let mut e2 = canary(64);
            unsafe {
                let a = edet.0(e1.as_mut_ptr(), ptr::null_mut(), m.as_ptr(), mlen as u64, sk.as_ptr());
                let b = edet.1(e2.as_mut_ptr(), ptr::null_mut(), m.as_ptr(), mlen as u64, sk.as_ptr());
                eq_i32("crypto_sign_ed25519_detached rc", a, b);
                assert_eq!(a, 0);
            }
            eq_bytes("crypto_sign_ed25519_detached", &e1, &e2);
            assert_eq!(e1, sig, "generic != explicit detached");

            // sm[0..64] from the combined API must be the same signature
            let mut sm = canary(mlen + 64);
            unsafe {
                assert_eq!(
                    comb(sm.as_mut_ptr(), ptr::null_mut(), m.as_ptr(), mlen as u64, sk.as_ptr()),
                    0
                )
            };
            assert_eq!(&sm[..64], &sig[..], "detached sig != crypto_sign sm[0..64]");

            // verify (G5-101, G5-102)
            for (what, f) in [
                ("crypto_sign_verify_detached", ver),
                ("crypto_sign_ed25519_verify_detached", ever),
            ] {
                let (x, y) = unsafe {
                    (
                        f.0(sig.as_ptr(), m.as_ptr(), mlen as u64, pk.as_ptr()),
                        f.1(sig.as_ptr(), m.as_ptr(), mlen as u64, pk.as_ptr()),
                    )
                };
                eq_i32(&format!("{what} rc(mlen={mlen})"), x, y);
                assert_eq!(x, 0, "{what} must accept a valid signature");
            }

            // G5-105: determinism — repeated calls, and `randombytes` untouched
            for _ in 0..3 {
                let mut again = canary(64);
                unsafe {
                    det.0(again.as_mut_ptr(), ptr::null_mut(), m.as_ptr(), mlen as u64, sk.as_ptr())
                };
                assert_eq!(&again[..], &sig[..], "signing must be deterministic");
            }
        }
    }

    // `randombytes` must NOT be consumed by signing: the RNG stream produced
    // after a signature is identical to the one produced without it.
    {
        let _g = RNG_LOCK.lock().unwrap();
        let (pk, sk) = sign_kp(&mut rng);
        let _ = pk;
        let m = rng.bytes(100);
        for (which, (d, rb)) in [(0usize, (det.0, rbuf.0)), (1, (det.1, rbuf.1))] {
            let mut base = [0u8; 32];
            reset_rngs(0xF1F1);
            unsafe { rb(base.as_mut_ptr(), 32) };
            let mut after = [0u8; 32];
            reset_rngs(0xF1F1);
            let mut sig = [0u8; 64];
            unsafe {
                assert_eq!(
                    d(sig.as_mut_ptr(), ptr::null_mut(), m.as_ptr(), 100, sk.as_ptr()),
                    0
                );
                rb(after.as_mut_ptr(), 32);
            }
            assert_eq!(base, after, "signing consumed randombytes (lib={which})");
        }
    }
}

/// G5-106, G5-107, G5-108, G5-109, G5-110 — the multipart ed25519ph API.
#[test]
fn sign_ed25519ph_multipart() {
    setup();
    let mut rng = Rng::new(0xF200);
    let apis: [(&str, (PhInit, PhInit), (PhUpdate, PhUpdate), (PhCreate, PhCreate), (PhVerify, PhVerify)); 2] = [
        (
            "crypto_sign",
            pair::<PhInit>("crypto_sign_init"),
            pair::<PhUpdate>("crypto_sign_update"),
            pair::<PhCreate>("crypto_sign_final_create"),
            pair::<PhVerify>("crypto_sign_final_verify"),
        ),
        (
            "crypto_sign_ed25519ph",
            pair::<PhInit>("crypto_sign_ed25519ph_init"),
            pair::<PhUpdate>("crypto_sign_ed25519ph_update"),
            pair::<PhCreate>("crypto_sign_ed25519ph_final_create"),
            pair::<PhVerify>("crypto_sign_ed25519ph_final_verify"),
        ),
    ];
    let det = sym::<Sign5>(c_lib(), "crypto_sign_detached");
    let sha = sym::<unsafe extern "C" fn(*mut u8, *const u8, u64) -> i32>(
        c_lib(), "crypto_hash_sha512");

    // G5-106 (single update) + G5-108 (zero updates, mlen 0 handled by chunks)
    for &mlen in SIGN_MLENS {
        for _ in 0..2 {
            let (pk, sk) = sign_kp(&mut rng);
            let m = rng.bytes(mlen);
            let mut sig_ref: Option<Vec<u8>> = None;
            for (name, init, upd, create, verify) in &apis {
                // chunkings: one shot, one byte at a time, random split, random
                // walk, plus the SHA-512 128-byte block straddles
                let mut plans: Vec<Vec<usize>> = Vec::new();
                for style in 0..4u32 {
                    plans.push(chunks(&mut rng, mlen, style));
                }
                if mlen == 0 {
                    plans.push(vec![]); // G5-108: zero `_update` calls
                    plans.push(vec![0]); // a single empty update
                    plans.push(vec![0, 0, 0]);
                } else {
                    plans.push(vec![mlen]);
                    plans.push(vec![1, mlen - 1, 0]);
                    if mlen >= 128 {
                        plans.push(vec![127, 1, mlen - 128]);
                        plans.push(vec![128, mlen - 128]);
                    }
                    if mlen >= 129 {
                        plans.push(vec![129, mlen - 129]);
                    }
                }
                for plan in &plans {
                    assert_eq!(plan.iter().sum::<usize>(), mlen);
                    let mut sigs: Vec<Vec<u8>> = Vec::new();
                    for which in 0..2usize {
                        let (i, u, c, _v) = (
                            if which == 0 { init.0 } else { init.1 },
                            if which == 0 { upd.0 } else { upd.1 },
                            if which == 0 { create.0 } else { create.1 },
                            if which == 0 { verify.0 } else { verify.1 },
                        );
                        let mut st = State::for_sym("crypto_sign_ed25519ph_statebytes");
                        let mut sig = canary(64 + 4);
                        let mut sl = 0xDEADu64;
                        let mut off = 0usize;
                        unsafe {
                            assert_eq!(i(st.as_mut_ptr()), 0, "{name}_init rc");
                            for &n in plan {
                                let p = if n == 0 && off == mlen {
                                    ptr::null()
                                } else {
                                    m.as_ptr().add(off)
                                };
                                assert_eq!(u(st.as_mut_ptr(), p, n as u64), 0, "{name}_update rc");
                                off += n;
                            }
                            assert_eq!(
                                c(st.as_mut_ptr(), sig.as_mut_ptr(), &mut sl, sk.as_ptr()),
                                0,
                                "{name}_final_create rc"
                            );
                        }
                        assert_eq!(sl, 64, "{name}_final_create *siglen_p");
                        assert_eq!(&sig[64..], &[0xA5u8; 4], "{name} wrote past 64 bytes");
                        sigs.push(sig[..64].to_vec());
                    }
                    eq_bytes(
                        &format!("{name}_final_create(mlen={mlen},plan={plan:?})"),
                        &sigs[0],
                        &sigs[1],
                    );
                    // G5-107: every chunking must give the identical signature
                    match &sig_ref {
                        None => sig_ref = Some(sigs[0].clone()),
                        Some(r) => assert_eq!(
                            &sigs[0], r,
                            "{name}: chunking {plan:?} changed the signature (mlen={mlen})"
                        ),
                    }

                    // and `_final_verify` accepts it, on both libraries
                    for which in 0..2usize {
                        let (i, u, v) = (
                            if which == 0 { init.0 } else { init.1 },
                            if which == 0 { upd.0 } else { upd.1 },
                            if which == 0 { verify.0 } else { verify.1 },
                        );
                        let mut st = State::for_sym("crypto_sign_ed25519ph_statebytes");
                        let mut off = 0usize;
                        let rc = unsafe {
                            assert_eq!(i(st.as_mut_ptr()), 0);
                            for &n in plan {
                                u(st.as_mut_ptr(), m.as_ptr().add(off), n as u64);
                                off += n;
                            }
                            v(st.as_mut_ptr(), sigs[0].as_ptr(), pk.as_ptr())
                        };
                        assert_eq!(rc, 0, "{name}_final_verify (lib={which}) must accept");
                    }
                }
            }
            // G5-109: the two APIs are interchangeable
            let ref_sig = sig_ref.unwrap();

            // G5-110: the ph signature must DIFFER from a plain detached
            // signature over SHA-512(m) (the dom2 prefix is only in the ph path)
            let mut ph = [0u8; 64];
            unsafe { sha(ph.as_mut_ptr(), m.as_ptr(), mlen as u64) };
            let mut plain = [0u8; 64];
            unsafe {
                assert_eq!(det(plain.as_mut_ptr(), ptr::null_mut(), ph.as_ptr(), 64, sk.as_ptr()), 0)
            };
            assert_ne!(
                &ref_sig[..], &plain[..],
                "ed25519ph must not equal detached-over-SHA512(m) (mlen={mlen})"
            );
        }
    }

    // The two multipart APIs share the state type, so a state built with the
    // generic functions can be finished with the explicit ones and vice versa.
    let (pk, sk) = sign_kp(&mut rng);
    let m = rng.bytes(500);
    for which in 0..2usize {
        let init_g = if which == 0 { apis[0].1.0 } else { apis[0].1.1 };
        let upd_e = if which == 0 { apis[1].2.0 } else { apis[1].2.1 };
        let create_g = if which == 0 { apis[0].3.0 } else { apis[0].3.1 };
        let init_e = if which == 0 { apis[1].1.0 } else { apis[1].1.1 };
        let upd_g = if which == 0 { apis[0].2.0 } else { apis[0].2.1 };
        let verify_e = if which == 0 { apis[1].4.0 } else { apis[1].4.1 };
        let mut st = State::for_sym("crypto_sign_statebytes");
        let mut sig = [0u8; 64];
        unsafe {
            assert_eq!(init_g(st.as_mut_ptr()), 0);
            assert_eq!(upd_e(st.as_mut_ptr(), m.as_ptr(), 500), 0);
            assert_eq!(create_g(st.as_mut_ptr(), sig.as_mut_ptr(), ptr::null_mut(), sk.as_ptr()), 0);
        }
        let mut st2 = State::for_sym("crypto_sign_ed25519ph_statebytes");
        let rc = unsafe {
            assert_eq!(init_e(st2.as_mut_ptr()), 0);
            assert_eq!(upd_g(st2.as_mut_ptr(), m.as_ptr(), 500), 0);
            verify_e(st2.as_mut_ptr(), sig.as_ptr(), pk.as_ptr())
        };
        assert_eq!(rc, 0, "mixed generic/explicit ph state (lib={which})");
        MIXED_PH.with(|s| {
            let mut b = s.borrow_mut();
            if which == 0 {
                *b = sig.to_vec();
            } else {
                eq_bytes("mixed generic/explicit ph signature", &b, &sig);
            }
        });
    }
}

thread_local! {
    static MIXED_PH: std::cell::RefCell<Vec<u8>> = const { std::cell::RefCell::new(Vec::new()) };
}

/// G5-111, G5-112, G5-113, G5-114, G5-115, G5-116 — the four `crypto_sign`
/// conversions, their interoperability with `crypto_box`, and the fact that
/// every generic wrapper forwards to its ed25519 counterpart.
#[test]
fn sign_conversions_and_wrappers() {
    setup();
    let mut rng = Rng::new(0xF300);
    let to_seed = pair::<Two>("crypto_sign_ed25519_sk_to_seed");
    let to_pk = pair::<Two>("crypto_sign_ed25519_sk_to_pk");
    let sk_to_c = pair::<Two>("crypto_sign_ed25519_sk_to_curve25519");
    let pk_to_c = pair::<Two>("crypto_sign_ed25519_pk_to_curve25519");
    let smb = sym::<Two>(c_lib(), "crypto_scalarmult_base");
    let sha = sym::<unsafe extern "C" fn(*mut u8, *const u8, u64) -> i32>(
        c_lib(), "crypto_hash_sha512");
    let bx_easy = pair::<Asym6>("crypto_box_easy");
    let bx_open = pair::<Asym6>("crypto_box_open_easy");

    let mut seeds: Vec<Vec<u8>> = vec![vec![0u8; 32], vec![0xffu8; 32], (0u8..32).collect()];
    for _ in 0..12 {
        seeds.push(rng.bytes(32));
    }
    let seed_kp = sym::<SeedKeypair>(c_lib(), "crypto_sign_ed25519_seed_keypair");

    for seed in &seeds {
        let mut pk = [0u8; 32];
        let mut sk = [0u8; 64];
        unsafe { assert_eq!(seed_kp(pk.as_mut_ptr(), sk.as_mut_ptr(), seed.as_ptr()), 0) };

        // ---- G5-111 / G5-112, including the in-place (overlapping) shapes
        for (what, f, want, outlen) in [
            ("sk_to_seed", to_seed, &sk[..32], 32usize),
            ("sk_to_pk", to_pk, &sk[32..64], 32),
        ] {
            let mut a = canary(outlen + 4);
            let mut b = canary(outlen + 4);
            let (x, y) = unsafe {
                (
                    f.0(a.as_mut_ptr(), sk.as_ptr()),
                    f.1(b.as_mut_ptr(), sk.as_ptr()),
                )
            };
            eq_i32(&format!("{what} rc"), x, y);
            assert_eq!(x, 0);
            eq_bytes(what, &a, &b);
            assert_eq!(&a[..outlen], want, "{what} output");
            assert_eq!(&a[outlen..], &[0xA5u8; 4], "{what} wrote past 32 bytes");

            // in place: out aliases sk
            let mut ip: Vec<Vec<u8>> = Vec::new();
            for which in 0..2usize {
                let g = if which == 0 { f.0 } else { f.1 };
                let mut buf = sk.to_vec();
                let rc = unsafe { g(buf.as_mut_ptr(), buf.as_ptr()) };
                assert_eq!(rc, 0, "{what} in-place rc");
                ip.push(buf);
            }
            eq_bytes(&format!("{what} in-place"), &ip[0], &ip[1]);
            assert_eq!(&ip[0][..outlen], want, "{what} in-place output");
        }

        // ---- G5-113: sk_to_curve25519 = clamped SHA-512(sk[0..32])[0..32]
        let mut want_csk = [0u8; 64];
        unsafe { sha(want_csk.as_mut_ptr(), sk.as_ptr(), 32) };
        want_csk[0] &= 248;
        want_csk[31] &= 127;
        want_csk[31] |= 64;
        let mut csk1 = canary(32 + 4);
        let mut csk2 = canary(32 + 4);
        let (x, y) = unsafe {
            (
                sk_to_c.0(csk1.as_mut_ptr(), sk.as_ptr()),
                sk_to_c.1(csk2.as_mut_ptr(), sk.as_ptr()),
            )
        };
        eq_i32("sk_to_curve25519 rc", x, y);
        assert_eq!(x, 0);
        eq_bytes("sk_to_curve25519", &csk1, &csk2);
        assert_eq!(&csk1[..32], &want_csk[..32], "sk_to_curve25519 derivation");
        assert_eq!(&csk1[32..], &[0xA5u8; 4]);

        // ---- G5-114: pk_to_curve25519 == scalarmult_base(sk_to_curve25519)
        let mut cpk1 = canary(32 + 4);
        let mut cpk2 = canary(32 + 4);
        let (x, y) = unsafe {
            (
                pk_to_c.0(cpk1.as_mut_ptr(), pk.as_ptr()),
                pk_to_c.1(cpk2.as_mut_ptr(), pk.as_ptr()),
            )
        };
        eq_i32("pk_to_curve25519 rc", x, y);
        assert_eq!(x, 0, "a valid ed25519 pk must convert");
        eq_bytes("pk_to_curve25519", &cpk1, &cpk2);
        assert_eq!(&cpk1[32..], &[0xA5u8; 4]);
        let mut chk = [0u8; 32];
        unsafe { assert_eq!(smb(chk.as_mut_ptr(), csk1.as_ptr()), 0) };
        assert_eq!(&chk[..], &cpk1[..32], "pk_to_curve25519 != base(sk_to_curve25519)");
    }

    // ---- G5-115: the converted keys interoperate with crypto_box
    for _ in 0..8 {
        let mut pk_a = [0u8; 32];
        let mut sk_a = [0u8; 64];
        let mut pk_b = [0u8; 32];
        let mut sk_b = [0u8; 64];
        let sa = rng.bytes(32);
        let sb = rng.bytes(32);
        unsafe {
            seed_kp(pk_a.as_mut_ptr(), sk_a.as_mut_ptr(), sa.as_ptr());
            seed_kp(pk_b.as_mut_ptr(), sk_b.as_mut_ptr(), sb.as_ptr());
        }
        let mut cpk_a = [0u8; 32];
        let mut csk_a = [0u8; 32];
        let mut cpk_b = [0u8; 32];
        let mut csk_b = [0u8; 32];
        unsafe {
            assert_eq!(pk_to_c.0(cpk_a.as_mut_ptr(), pk_a.as_ptr()), 0);
            assert_eq!(sk_to_c.0(csk_a.as_mut_ptr(), sk_a.as_ptr()), 0);
            assert_eq!(pk_to_c.0(cpk_b.as_mut_ptr(), pk_b.as_ptr()), 0);
            assert_eq!(sk_to_c.0(csk_b.as_mut_ptr(), sk_b.as_ptr()), 0);
        }
        let n = rng.bytes(24);
        let m = rng.bytes(200);
        let mut c1 = canary(216);
        let mut c2 = canary(216);
        unsafe {
            let a = bx_easy.0(c1.as_mut_ptr(), m.as_ptr(), 200, n.as_ptr(),
                              cpk_b.as_ptr(), csk_a.as_ptr());
            let b = bx_easy.1(c2.as_mut_ptr(), m.as_ptr(), 200, n.as_ptr(),
                              cpk_b.as_ptr(), csk_a.as_ptr());
            eq_i32("converted-key crypto_box_easy rc", a, b);
            assert_eq!(a, 0);
        }
        eq_bytes("converted-key crypto_box_easy", &c1, &c2);
        let mut p1 = canary(200);
        let mut p2 = canary(200);
        unsafe {
            let a = bx_open.0(p1.as_mut_ptr(), c1.as_ptr(), 216, n.as_ptr(),
                              cpk_a.as_ptr(), csk_b.as_ptr());
            let b = bx_open.1(p2.as_mut_ptr(), c1.as_ptr(), 216, n.as_ptr(),
                              cpk_a.as_ptr(), csk_b.as_ptr());
            eq_i32("converted-key crypto_box_open_easy rc", a, b);
            assert_eq!(a, 0, "ed25519 -> X25519 conversion must interoperate");
        }
        eq_bytes("converted-key crypto_box_open_easy", &p1, &p2);
        assert_eq!(&p1[..], &m[..]);
    }
}

// ===========================================================================
// crypto_auth — HMAC-SHA256 / SHA512 / SHA512256
// ===========================================================================

const AUTH_INLENS: &[usize] = &[0, 1, 63, 64, 65, 127, 128, 129, 1000];

struct AuthApi {
    name: &'static str,
    bytes: usize,
    /// HMAC block size — the strict `keylen > block` hashing threshold.
    block: usize,
    /// digest length the hashed key is replaced by
    khash: usize,
    one: (Auth4, Auth4),
    ver: (AuthV4, AuthV4),
    init: (AuthInit, AuthInit),
    upd: (AuthUpdate, AuthUpdate),
    fin: (AuthFinal, AuthFinal),
    statebytes: String,
}

fn auth_api(name: &'static str, bytes: usize, block: usize, khash: usize) -> AuthApi {
    AuthApi {
        name,
        bytes,
        block,
        khash,
        one: pair::<Auth4>(name),
        ver: pair::<AuthV4>(&format!("{name}_verify")),
        init: pair::<AuthInit>(&format!("{name}_init")),
        upd: pair::<AuthUpdate>(&format!("{name}_update")),
        fin: pair::<AuthFinal>(&format!("{name}_final")),
        statebytes: format!("{name}_statebytes"),
    }
}

fn auth_apis() -> Vec<AuthApi> {
    vec![
        auth_api("crypto_auth_hmacsha256", 32, 64, 32),
        auth_api("crypto_auth_hmacsha512", 64, 128, 64),
        auth_api("crypto_auth_hmacsha512256", 32, 128, 64),
    ]
}

/// Run `_init`/`_update`(chunked)/`_final` on one library, returning the tag
/// and the state buffer left behind by `_final`.
fn auth_stream(
    api: &AuthApi,
    which: usize,
    key: Option<&[u8]>,
    keylen: usize,
    input: &[u8],
    plan: &[usize],
) -> (Vec<u8>, Vec<u8>) {
    let (i, u, f) = (
        if which == 0 { api.init.0 } else { api.init.1 },
        if which == 0 { api.upd.0 } else { api.upd.1 },
        if which == 0 { api.fin.0 } else { api.fin.1 },
    );
    let mut st = State::for_sym(&api.statebytes);
    let mut out = canary(api.bytes + 4);
    let kp = match key {
        None => ptr::null(),
        Some(k) => k.as_ptr(),
    };
    let mut off = 0usize;
    unsafe {
        assert_eq!(i(st.as_mut_ptr(), kp, keylen), 0, "{}_init rc", api.name);
        for &n in plan {
            assert_eq!(
                u(st.as_mut_ptr(), input.as_ptr().add(off), n as u64),
                0,
                "{}_update rc",
                api.name
            );
            off += n;
        }
        assert_eq!(f(st.as_mut_ptr(), out.as_mut_ptr()), 0, "{}_final rc", api.name);
    }
    assert_eq!(&out[api.bytes..], &[0xA5u8; 4], "{} wrote past BYTES", api.name);
    (out[..api.bytes].to_vec(), st.bytes().to_vec())
}

/// G5-122, G5-123, G5-129, G5-135, G5-139 — the one-shot APIs of all three
/// primitives plus the generic `crypto_auth` wrapper, over the full `inlen`
/// axis, with `_verify` on the correct tag.
#[test]
fn auth_one_shot() {
    setup();
    let mut rng = Rng::new(0x10000);
    let apis = auth_apis();
    let h512 = sym::<Auth4>(c_lib(), "crypto_auth_hmacsha512");
    let generic = pair::<Auth4>("crypto_auth");
    let gver = pair::<AuthV4>("crypto_auth_verify");
    let h512256 = sym::<Auth4>(c_lib(), "crypto_auth_hmacsha512256");

    for &inlen in AUTH_INLENS {
        for _ in 0..3 {
            let k = rng.bytes(32);
            let input = rng.bytes(inlen);
            let ip = if inlen == 0 { ptr::null() } else { input.as_ptr() };
            let mut tag512: Option<Vec<u8>> = None;
            for api in &apis {
                let mut a = canary(api.bytes + 4);
                let mut b = canary(api.bytes + 4);
                let (x, y) = unsafe {
                    (
                        api.one.0(a.as_mut_ptr(), ip, inlen as u64, k.as_ptr()),
                        api.one.1(b.as_mut_ptr(), ip, inlen as u64, k.as_ptr()),
                    )
                };
                eq_i32(&format!("{} rc(inlen={inlen})", api.name), x, y);
                assert_eq!(x, 0);
                eq_bytes(&format!("{}(inlen={inlen})", api.name), &a, &b);
                assert_eq!(&a[api.bytes..], &[0xA5u8; 4]);

                // one-shot == streaming with keylen = KEYBYTES
                let (s0, _) = auth_stream(api, 0, Some(&k), 32, &input, &[inlen]);
                let (s1, _) = auth_stream(api, 1, Some(&k), 32, &input, &[inlen]);
                eq_bytes(&format!("{} streaming(inlen={inlen})", api.name), &s0, &s1);
                assert_eq!(&s0[..], &a[..api.bytes], "{} one-shot != streaming", api.name);

                // _verify on the correct tag
                let (x, y) = unsafe {
                    (
                        api.ver.0(a.as_ptr(), ip, inlen as u64, k.as_ptr()),
                        api.ver.1(a.as_ptr(), ip, inlen as u64, k.as_ptr()),
                    )
                };
                eq_i32(&format!("{}_verify rc(inlen={inlen})", api.name), x, y);
                assert_eq!(x, 0, "{}_verify must accept the correct tag", api.name);

                if api.name == "crypto_auth_hmacsha512" {
                    tag512 = Some(a[..64].to_vec());
                }
                if api.name == "crypto_auth_hmacsha512256" {
                    // G5-135: the 32-byte tag is HMAC-SHA512's first 32 bytes
                    let mut full = [0u8; 64];
                    unsafe { h512(full.as_mut_ptr(), ip, inlen as u64, k.as_ptr()) };
                    assert_eq!(&a[..32], &full[..32], "hmacsha512256 != hmacsha512[0..32]");
                    assert_eq!(&tag512.as_ref().unwrap()[..32], &a[..32]);
                }
            }

            // G5-139: the generic wrapper
            let mut g1 = canary(32 + 4);
            let mut g2 = canary(32 + 4);
            let (x, y) = unsafe {
                (
                    generic.0(g1.as_mut_ptr(), ip, inlen as u64, k.as_ptr()),
                    generic.1(g2.as_mut_ptr(), ip, inlen as u64, k.as_ptr()),
                )
            };
            eq_i32(&format!("crypto_auth rc(inlen={inlen})"), x, y);
            assert_eq!(x, 0);
            eq_bytes(&format!("crypto_auth(inlen={inlen})"), &g1, &g2);
            let mut want = [0u8; 32];
            unsafe { h512256(want.as_mut_ptr(), ip, inlen as u64, k.as_ptr()) };
            assert_eq!(&g1[..32], &want[..], "crypto_auth != crypto_auth_hmacsha512256");
            let (x, y) = unsafe {
                (
                    gver.0(g1.as_ptr(), ip, inlen as u64, k.as_ptr()),
                    gver.1(g1.as_ptr(), ip, inlen as u64, k.as_ptr()),
                )
            };
            eq_i32("crypto_auth_verify rc", x, y);
            assert_eq!(x, 0);
        }
    }
}

/// G5-124, G5-125, G5-126, G5-130, G5-131, G5-132, G5-136, G5-141 — the
/// streaming `_init` with every documented `keylen`, including the strict
/// `keylen > blocksize` hashing branch and its exact boundary.
#[test]
fn auth_key_lengths() {
    setup();
    let mut rng = Rng::new(0x10100);
    let sha256 = sym::<unsafe extern "C" fn(*mut u8, *const u8, u64) -> i32>(
        c_lib(), "crypto_hash_sha256");
    let sha512 = sym::<unsafe extern "C" fn(*mut u8, *const u8, u64) -> i32>(
        c_lib(), "crypto_hash_sha512");
    let keylens: &[usize] = &[0, 1, 32, 64, 65, 128, 129, 200];

    for api in &auth_apis() {
        for &inlen in &[0usize, 1, 64, 128, 1000] {
            let input = rng.bytes(inlen);
            let bigkey = rng.bytes(256);
            let mut tags: Vec<(usize, Vec<u8>)> = Vec::new();
            for &keylen in keylens {
                let key = &bigkey[..keylen];
                let (t0, st0) = auth_stream(api, 0, Some(key), keylen, &input, &[inlen]);
                let (t1, st1) = auth_stream(api, 1, Some(key), keylen, &input, &[inlen]);
                eq_bytes(
                    &format!("{}_init(keylen={keylen},inlen={inlen})", api.name),
                    &t0,
                    &t1,
                );
                eq_bytes(
                    &format!("{}_init(keylen={keylen}) post-final state", api.name),
                    &st0,
                    &st1,
                );

                // keylen == KEYBYTES must reproduce the one-shot result
                if keylen == 32 {
                    let mut want = canary(api.bytes);
                    let ip = if inlen == 0 { ptr::null() } else { input.as_ptr() };
                    unsafe {
                        assert_eq!(
                            api.one.0(want.as_mut_ptr(), ip, inlen as u64, key.as_ptr()),
                            0
                        )
                    };
                    assert_eq!(t0, want, "{} keylen=32 != one-shot", api.name);
                }

                // keylen > blocksize: the key is replaced by its own digest
                if keylen > api.block {
                    let mut kh = vec![0u8; api.khash];
                    unsafe {
                        if api.khash == 32 {
                            sha256(kh.as_mut_ptr(), key.as_ptr(), keylen as u64);
                        } else {
                            sha512(kh.as_mut_ptr(), key.as_ptr(), keylen as u64);
                        }
                    }
                    let (want, _) = auth_stream(api, 0, Some(&kh), api.khash, &input, &[inlen]);
                    assert_eq!(
                        t0, want,
                        "{}: keylen={keylen} must equal init(digest(key), {})",
                        api.name, api.khash
                    );
                    let (want_r, _) = auth_stream(api, 1, Some(&kh), api.khash, &input, &[inlen]);
                    assert_eq!(t1, want_r, "{} (Rust) hashed-key branch", api.name);
                }

                tags.push((keylen, t0));
            }

            // G5-126 / G5-132: the threshold is strict `>` — `block` and
            // `block + 1` bytes of the SAME key prefix must differ
            let at_block = tags.iter().find(|(l, _)| *l == api.block).unwrap().1.clone();
            let over = tags
                .iter()
                .find(|(l, _)| *l == api.block + 1)
                .map(|(_, t)| t.clone());
            if let Some(over) = over {
                assert_ne!(
                    at_block, over,
                    "{}: keylen={} (verbatim) and {} (hashed) must differ",
                    api.name,
                    api.block,
                    api.block + 1
                );
            }

            // G5-141: key != NULL with keylen == 0 == key == NULL, keylen == 0
            let (a0, sa) = auth_stream(api, 0, Some(&bigkey[..0]), 0, &input, &[inlen]);
            let (b0, sb) = auth_stream(api, 0, None, 0, &input, &[inlen]);
            assert_eq!(a0, b0, "{}: key!=NULL,keylen=0 == key=NULL,keylen=0", api.name);
            assert_eq!(sa, sb);
            let (a1, _) = auth_stream(api, 1, Some(&bigkey[..0]), 0, &input, &[inlen]);
            let (b1, _) = auth_stream(api, 1, None, 0, &input, &[inlen]);
            eq_bytes(&format!("{} keylen=0 non-NULL key", api.name), &a0, &a1);
            eq_bytes(&format!("{} keylen=0 NULL key", api.name), &b0, &b1);
        }
    }
}

/// G5-127, G5-133, G5-137 — chunked `_update`: every chunking of the same
/// input must give the identical tag (equal to the one-shot result).
#[test]
fn auth_chunked_updates() {
    setup();
    let mut rng = Rng::new(0x10200);
    for api in &auth_apis() {
        for &inlen in &[0usize, 1, 64, 128, 129, 1000] {
            let k = rng.bytes(32);
            let input = rng.bytes(inlen);
            let mut plans: Vec<Vec<usize>> = Vec::new();
            for style in 0..4u32 {
                plans.push(chunks(&mut rng, inlen, style));
            }
            if inlen == 0 {
                plans.push(vec![]);
                plans.push(vec![0, 0]);
            } else {
                plans.push(vec![inlen]);
                plans.push(vec![1, inlen - 1, 0]);
                plans.push(vec![0, inlen]);
                if inlen >= 64 {
                    plans.push(vec![63, 1, inlen - 64]);
                    plans.push(vec![64, inlen - 64]);
                }
                if inlen >= 65 {
                    plans.push(vec![65, inlen - 65]);
                }
                if inlen >= 128 {
                    plans.push(vec![127, 1, inlen - 128]);
                    plans.push(vec![128, inlen - 128]);
                }
                if inlen >= 129 {
                    plans.push(vec![129, inlen - 129]);
                }
                if inlen >= 500 {
                    plans.push(vec![inlen / 2, inlen - inlen / 2]);
                }
            }
            let mut want = canary(api.bytes);
            let ip = if inlen == 0 { ptr::null() } else { input.as_ptr() };
            unsafe {
                assert_eq!(api.one.0(want.as_mut_ptr(), ip, inlen as u64, k.as_ptr()), 0)
            };
            for plan in &plans {
                assert_eq!(plan.iter().sum::<usize>(), inlen);
                let (t0, st0) = auth_stream(api, 0, Some(&k), 32, &input, plan);
                let (t1, st1) = auth_stream(api, 1, Some(&k), 32, &input, plan);
                eq_bytes(
                    &format!("{} chunked(inlen={inlen},plan={plan:?})", api.name),
                    &t0,
                    &t1,
                );
                eq_bytes(
                    &format!("{} chunked state(inlen={inlen},plan={plan:?})", api.name),
                    &st0,
                    &st1,
                );
                assert_eq!(
                    t0, want,
                    "{}: chunking {plan:?} changed the tag (inlen={inlen})",
                    api.name
                );
            }
        }
    }
}

/// G5-128, G5-134 — `_verify` on correct and incorrect tags.
#[test]
fn auth_verify_correct_and_wrong() {
    setup();
    let mut rng = Rng::new(0x10300);
    for api in &auth_apis() {
        for &inlen in &[0usize, 1, 64, 128, 1000] {
            let k = rng.bytes(32);
            let input = rng.bytes(inlen);
            let ip = if inlen == 0 { ptr::null() } else { input.as_ptr() };
            let mut good = canary(api.bytes);
            unsafe { assert_eq!(api.one.0(good.as_mut_ptr(), ip, inlen as u64, k.as_ptr()), 0) };

            // correct
            let (x, y) = unsafe {
                (
                    api.ver.0(good.as_ptr(), ip, inlen as u64, k.as_ptr()),
                    api.ver.1(good.as_ptr(), ip, inlen as u64, k.as_ptr()),
                )
            };
            eq_i32(&format!("{}_verify correct", api.name), x, y);
            assert_eq!(x, 0);

            // wrong: bit 0 of byte 0, bit 7 of the last byte, all-zero, all-ff,
            // and a random tag
            let mut bad: Vec<(String, Vec<u8>)> = Vec::new();
            let mut v = good.clone();
            v[0] ^= 0x01;
            bad.push(("byte0bit0".into(), v));
            let mut v = good.clone();
            let last = api.bytes - 1;
            v[last] ^= 0x80;
            bad.push(("lastbyte bit7".into(), v));
            bad.push(("zero".into(), vec![0u8; api.bytes]));
            bad.push(("ff".into(), vec![0xffu8; api.bytes]));
            bad.push(("random".into(), rng.bytes(api.bytes)));
            for (what, h) in &bad {
                if h == &good {
                    continue;
                }
                let (x, y) = unsafe {
                    (
                        api.ver.0(h.as_ptr(), ip, inlen as u64, k.as_ptr()),
                        api.ver.1(h.as_ptr(), ip, inlen as u64, k.as_ptr()),
                    )
                };
                eq_i32(&format!("{}_verify {what} rc", api.name), x, y);
                assert_eq!(x, -1, "{}_verify must reject {what}", api.name);
            }
            // a wrong key and a wrong message also reject
            let k2 = rng.bytes(32);
            let (x, y) = unsafe {
                (
                    api.ver.0(good.as_ptr(), ip, inlen as u64, k2.as_ptr()),
                    api.ver.1(good.as_ptr(), ip, inlen as u64, k2.as_ptr()),
                )
            };
            eq_i32(&format!("{}_verify wrong key rc", api.name), x, y);
            assert_eq!(x, -1);
            if inlen > 0 {
                let mut m2 = input.clone();
                m2[0] ^= 1;
                let (x, y) = unsafe {
                    (
                        api.ver.0(good.as_ptr(), m2.as_ptr(), inlen as u64, k.as_ptr()),
                        api.ver.1(good.as_ptr(), m2.as_ptr(), inlen as u64, k.as_ptr()),
                    )
                };
                eq_i32(&format!("{}_verify wrong msg rc", api.name), x, y);
                assert_eq!(x, -1);
            }
        }
    }
}

/// G5-138, G5-140, G5-142 — the shared `hmacsha512`/`hmacsha512256` state
/// type, key-value shapes, and the `_final` edge cases (aliasing `out`, and a
/// second `_final` on the zeroed state).
#[test]
fn auth_state_sharing_key_shapes_and_final_edges() {
    setup();
    let mut rng = Rng::new(0x10400);

    // ---- G5-138: same state type; `_final` picks the width
    let f512 = pair::<AuthFinal>("crypto_auth_hmacsha512_final");
    let f256 = pair::<AuthFinal>("crypto_auth_hmacsha512256_final");
    let i512 = pair::<AuthInit>("crypto_auth_hmacsha512_init");
    let u512 = pair::<AuthUpdate>("crypto_auth_hmacsha512_update");
    for &inlen in &[0usize, 1, 128, 1000] {
        let k = rng.bytes(32);
        let input = rng.bytes(inlen);
        for which in 0..2usize {
            let (i, u, fa, fb) = (
                if which == 0 { i512.0 } else { i512.1 },
                if which == 0 { u512.0 } else { u512.1 },
                if which == 0 { f512.0 } else { f512.1 },
                if which == 0 { f256.0 } else { f256.1 },
            );
            let mut st_a = State::for_sym("crypto_auth_hmacsha512_statebytes");
            let mut st_b = State::for_sym("crypto_auth_hmacsha512256_statebytes");
            assert_eq!(st_a.len(), st_b.len(), "the two state types must be identical");
            let mut out64 = canary(64 + 4);
            let mut out32 = canary(32 + 4);
            unsafe {
                assert_eq!(i(st_a.as_mut_ptr(), k.as_ptr(), 32), 0);
                assert_eq!(i(st_b.as_mut_ptr(), k.as_ptr(), 32), 0);
                assert_eq!(u(st_a.as_mut_ptr(), input.as_ptr(), inlen as u64), 0);
                assert_eq!(u(st_b.as_mut_ptr(), input.as_ptr(), inlen as u64), 0);
                eq_bytes("512 vs 512256 pre-final state", st_a.bytes(), st_b.bytes());
                assert_eq!(fa(st_a.as_mut_ptr(), out64.as_mut_ptr()), 0);
                assert_eq!(fb(st_b.as_mut_ptr(), out32.as_mut_ptr()), 0);
            }
            assert_eq!(
                &out32[..32], &out64[..32],
                "hmacsha512256_final must be hmacsha512_final[0..32] (lib={which})"
            );
            assert_eq!(&out32[32..], &[0xA5u8; 4], "512256_final wrote past 32 bytes");
            assert_eq!(&out64[64..], &[0xA5u8; 4]);
            // `_final` zeroes the two inner SHA states
            assert_eq!(st_a.bytes(), &vec![0u8; st_a.len()][..], "final must zero the state");
            assert_eq!(st_b.bytes(), &vec![0u8; st_b.len()][..]);
            FINAL_WIDTH.with(|s| {
                let mut b = s.borrow_mut();
                if which == 0 {
                    *b = (out64[..64].to_vec(), out32[..32].to_vec());
                } else {
                    eq_bytes("hmacsha512_final", &b.0, &out64[..64]);
                    eq_bytes("hmacsha512256_final", &b.1, &out32[..32]);
                }
            });
        }
    }

    // ---- G5-140: key-value shapes (fixed keylen = 32) must all differ, and
    // an all-zero 32-byte key must differ from keylen = 0
    for api in &auth_apis() {
        let input = rng.bytes(64);
        let mut keys: Vec<Vec<u8>> = vec![vec![0u8; 32], vec![0xffu8; 32], (0u8..32).collect()];
        for _ in 0..4 {
            keys.push(rng.bytes(32));
        }
        let mut tags: Vec<Vec<u8>> = Vec::new();
        for k in &keys {
            let mut a = canary(api.bytes);
            let mut b = canary(api.bytes);
            let (x, y) = unsafe {
                (
                    api.one.0(a.as_mut_ptr(), input.as_ptr(), 64, k.as_ptr()),
                    api.one.1(b.as_mut_ptr(), input.as_ptr(), 64, k.as_ptr()),
                )
            };
            eq_i32(&format!("{} key shape rc", api.name), x, y);
            assert_eq!(x, 0);
            eq_bytes(&format!("{}(k={})", api.name, hex(k)), &a, &b);
            tags.push(a);
        }
        let n = tags.len();
        let mut sorted = tags.clone();
        sorted.sort();
        sorted.dedup();
        assert_eq!(sorted.len(), n, "{}: distinct keys must give distinct tags", api.name);
        // keylen = 0 vs an all-zero 32-byte key.
        // NOTE the CONFIGS G5-140 remark that these two "MUST produce
        // different tags" is contradicted by the C: both XOR loops are
        // `pad[i] ^= key[i]`, so an all-zero key leaves the 0x36/0x5c pads
        // untouched and the result is byte-identical to keylen = 0. The C is
        // ground truth, so that is what is asserted here.
        let (zero32, _) = auth_stream(api, 0, Some(&keys[0]), 32, &input, &[64]);
        let (klen0, _) = auth_stream(api, 0, None, 0, &input, &[64]);
        assert_eq!(zero32, tags[0]);
        assert_eq!(
            zero32, klen0,
            "{}: an all-zero 32-byte key XORs into the pads as a no-op, so it \
             must equal keylen = 0",
            api.name
        );
        let (zero32r, _) = auth_stream(api, 1, Some(&keys[0]), 32, &input, &[64]);
        let (klen0r, _) = auth_stream(api, 1, None, 0, &input, &[64]);
        eq_bytes(&format!("{} zero key", api.name), &zero32, &zero32r);
        eq_bytes(&format!("{} keylen=0", api.name), &klen0, &klen0r);

        // ---- G5-142: `out` aliasing the message buffer, and a second `_final`
        for &inlen in &[0usize, 64, 1000] {
            let k = rng.bytes(32);
            let msg = rng.bytes(inlen.max(api.bytes));
            let mut aliased: Vec<Vec<u8>> = Vec::new();
            let mut seconds: Vec<Vec<u8>> = Vec::new();
            let mut states: Vec<Vec<u8>> = Vec::new();
            for which in 0..2usize {
                let (i, u, f) = (
                    if which == 0 { api.init.0 } else { api.init.1 },
                    if which == 0 { api.upd.0 } else { api.upd.1 },
                    if which == 0 { api.fin.0 } else { api.fin.1 },
                );
                let mut st = State::for_sym(&api.statebytes);
                let mut buf = msg.clone();
                unsafe {
                    assert_eq!(i(st.as_mut_ptr(), k.as_ptr(), 32), 0);
                    assert_eq!(u(st.as_mut_ptr(), buf.as_ptr(), inlen as u64), 0);
                    // out overlaps the very buffer that was hashed
                    assert_eq!(f(st.as_mut_ptr(), buf.as_mut_ptr()), 0);
                }
                aliased.push(buf);
                states.push(st.bytes().to_vec());
                // a second `_final` on the (now zeroed) state
                let mut again = canary(api.bytes);
                let rc = unsafe { f(st.as_mut_ptr(), again.as_mut_ptr()) };
                assert_eq!(rc, 0, "{}: a second _final still returns 0", api.name);
                seconds.push(again);
            }
            eq_bytes(
                &format!("{} out aliasing input(inlen={inlen})", api.name),
                &aliased[0],
                &aliased[1],
            );
            eq_bytes(
                &format!("{} state after _final(inlen={inlen})", api.name),
                &states[0],
                &states[1],
            );
            eq_bytes(
                &format!("{} second _final(inlen={inlen})", api.name),
                &seconds[0],
                &seconds[1],
            );
            // the first `_final` was still the correct tag
            let mut want = canary(api.bytes);
            let ip = if inlen == 0 { ptr::null() } else { msg.as_ptr() };
            unsafe { assert_eq!(api.one.0(want.as_mut_ptr(), ip, inlen as u64, k.as_ptr()), 0) };
            assert_eq!(
                &aliased[0][..api.bytes], &want[..],
                "{}: aliased _final must still be correct",
                api.name
            );
        }
    }
}

thread_local! {
    static FINAL_WIDTH: std::cell::RefCell<(Vec<u8>, Vec<u8>)> =
        const { std::cell::RefCell::new((Vec::new(), Vec::new())) };
}
