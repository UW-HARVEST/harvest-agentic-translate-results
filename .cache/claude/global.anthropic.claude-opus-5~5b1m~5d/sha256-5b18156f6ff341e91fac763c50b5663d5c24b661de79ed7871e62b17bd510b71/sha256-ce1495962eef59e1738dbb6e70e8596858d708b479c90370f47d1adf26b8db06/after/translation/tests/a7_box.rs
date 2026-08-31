//! Area 7 — `crypto_box`: `curve25519xsalsa20poly1305` (the default, reached
//! through the generic `crypto_box_*` names) and
//! `curve25519xchacha20poly1305`, in every API shape the C offers: `_easy`,
//! `_detached`, the `_beforenm`/`*_afternm` precomputed forms, the NaCl-style
//! zero-padded `crypto_box`/`crypto_box_open` (xsalsa only) and `_seal`.
//!
//! Covers `configs_7.md` rows 7.60–7.80 and `errors_7.md` rows 7.59–7.91.
//!
//! Output buffers are always pre-filled with a distinctive pattern and
//! compared byte for byte after failures as well as successes: some rejects
//! leave the caller's buffer untouched, others (notably `_seal`, which
//! `memcpy`s the ephemeral public key even when encryption failed) do not.
mod common;
use common::*;
use libloading::{Library, Symbol};
use std::ffi::{c_char, c_int, c_void, CStr};

// --------------------------------------------------------------- signatures

/// `_easy(c, m, mlen, n, pk, sk)`, `_open_easy(m, c, clen, n, pk, sk)`,
/// `crypto_box(c, m, mlen, n, pk, sk)`, `crypto_box_open(...)`.
type F6 = unsafe extern "C" fn(*mut u8, *const u8, u64, *const u8, *const u8, *const u8) -> c_int;
/// `_detached(c, mac, m, mlen, n, pk, sk)`
type F7D =
    unsafe extern "C" fn(*mut u8, *mut u8, *const u8, u64, *const u8, *const u8, *const u8) -> c_int;
/// `_open_detached(m, c, mac, clen, n, pk, sk)`
type F7O = unsafe extern "C" fn(
    *mut u8,
    *const u8,
    *const u8,
    u64,
    *const u8,
    *const u8,
    *const u8,
) -> c_int;
/// `_easy_afternm(c, m, mlen, n, k)`, `_open_easy_afternm(m, c, clen, n, k)`,
/// `_afternm`, `_open_afternm`, and `_seal_open(m, c, clen, pk, sk)`.
type F5 = unsafe extern "C" fn(*mut u8, *const u8, u64, *const u8, *const u8) -> c_int;
/// `_detached_afternm(c, mac, m, mlen, n, k)`
type F6D = unsafe extern "C" fn(*mut u8, *mut u8, *const u8, u64, *const u8, *const u8) -> c_int;
/// `_open_detached_afternm(m, c, mac, clen, n, k)`
type F6O =
    unsafe extern "C" fn(*mut u8, *const u8, *const u8, u64, *const u8, *const u8) -> c_int;
/// `_beforenm(k, pk, sk)`
type F3 = unsafe extern "C" fn(*mut u8, *const u8, *const u8) -> c_int;
/// `_seal(c, m, mlen, pk)`
type F4 = unsafe extern "C" fn(*mut u8, *const u8, u64, *const u8) -> c_int;
type KP = unsafe extern "C" fn(*mut u8, *mut u8) -> c_int;
type SKP = unsafe extern "C" fn(*mut u8, *mut u8, *const u8) -> c_int;
type MultBase = unsafe extern "C" fn(*mut u8, *const u8) -> c_int;
type HashFn = unsafe extern "C" fn(*mut u8, *const u8, u64) -> c_int;
type SizeFn = unsafe extern "C" fn() -> usize;
type StrFn = unsafe extern "C" fn() -> *const c_char;

const MAC: usize = 16;
const NONCE: usize = 24;
const SEAL: usize = 48;
const ZERO: usize = 32; // crypto_box_ZEROBYTES
const BOXZERO: usize = 16; // crypto_box_BOXZEROBYTES

// ------------------------------------------------------------------ helpers

fn hx(s: &str) -> Vec<u8> {
    assert!(s.len() % 2 == 0, "odd hex length");
    (0..s.len() / 2)
        .map(|i| u8::from_str_radix(&s[2 * i..2 * i + 2], 16).unwrap())
        .collect()
}
fn h32(s: &str) -> [u8; 32] {
    let v = hx(s);
    let mut a = [0u8; 32];
    a.copy_from_slice(&v);
    a
}

fn prefill(len: usize) -> Vec<u8> {
    let mut v = padded(len);
    for (i, b) in v[..len].iter_mut().enumerate() {
        *b = 0x5Au8.wrapping_add((i as u8).wrapping_mul(7));
    }
    v
}
fn pattern(len: usize) -> Vec<u8> {
    (0..len)
        .map(|i| 0x5Au8.wrapping_add((i as u8).wrapping_mul(7)))
        .collect()
}

/// Is `name` exported by `lib`?  Used to assert that the Rust port neither
/// omits nor *invents* an entry point.
fn present(lib: &Library, name: &str) -> bool {
    let mut b: Vec<u8> = name.as_bytes().to_vec();
    b.push(0);
    unsafe { lib.get::<*const c_void>(&b).is_ok() }
}
#[track_caller]
fn assert_absent(name: &str) {
    let c = present(c_lib(), name);
    let r = present(rust_lib(), name);
    assert_eq!(c, r, "{name}: exported by C={c} but Rust={r}");
    assert!(!c, "{name} must not exist in this build");
}

// ---------------------------------------------------------------- pair types

macro_rules! pairty {
    ($s:ident, $t:ty) => {
        struct $s {
            name: String,
            c: Symbol<'static, $t>,
            r: Symbol<'static, $t>,
        }
        impl $s {
            fn new(name: &str) -> Self {
                let (c, r) = both::<$t>(name);
                $s { name: name.to_string(), c, r }
            }
        }
    };
}

pairty!(P6, F6);
pairty!(P7D, F7D);
pairty!(P7O, F7O);
pairty!(P5, F5);
pairty!(P6D, F6D);
pairty!(P6O, F6O);
pairty!(P3, F3);
pairty!(P4, F4);

impl P6 {
    /// `f(out, in, inlen, n, pk, sk)` with a pre-filled `outlen`-byte output.
    #[track_caller]
    fn run(&self, outlen: usize, inp: &[u8], n: &[u8], pk: &[u8], sk: &[u8]) -> (c_int, Vec<u8>) {
        let mut oc = prefill(outlen);
        let mut or = prefill(outlen);
        let tag = format!("{}(inlen {})", self.name, inp.len());
        let rc = unsafe {
            (self.c)(oc.as_mut_ptr(), inp.as_ptr(), inp.len() as u64, n.as_ptr(), pk.as_ptr(), sk.as_ptr())
        };
        let rr = unsafe {
            (self.r)(or.as_mut_ptr(), inp.as_ptr(), inp.len() as u64, n.as_ptr(), pk.as_ptr(), sk.as_ptr())
        };
        eqi(&format!("{tag} rc"), rc, rr);
        eqb(&format!("{tag} out"), &oc, &or);
        check_pad(&format!("{tag} C"), &oc, outlen);
        check_pad(&format!("{tag} Rust"), &or, outlen);
        (rc, oc[..outlen].to_vec())
    }
}

impl P5 {
    /// `f(out, in, inlen, a, b)` — `(n, k)` for the `*_afternm` forms and
    /// `(pk, sk)` for `_seal_open`.
    #[track_caller]
    fn run(&self, outlen: usize, inp: &[u8], a: &[u8], b: &[u8]) -> (c_int, Vec<u8>) {
        let mut oc = prefill(outlen);
        let mut or = prefill(outlen);
        let tag = format!("{}(inlen {})", self.name, inp.len());
        let rc = unsafe {
            (self.c)(oc.as_mut_ptr(), inp.as_ptr(), inp.len() as u64, a.as_ptr(), b.as_ptr())
        };
        let rr = unsafe {
            (self.r)(or.as_mut_ptr(), inp.as_ptr(), inp.len() as u64, a.as_ptr(), b.as_ptr())
        };
        eqi(&format!("{tag} rc"), rc, rr);
        eqb(&format!("{tag} out"), &oc, &or);
        check_pad(&format!("{tag} C"), &oc, outlen);
        check_pad(&format!("{tag} Rust"), &or, outlen);
        (rc, oc[..outlen].to_vec())
    }
}

impl P7D {
    /// `f(c, mac, m, mlen, n, pk, sk)`; returns `(rc, c, mac)`.
    #[track_caller]
    fn run(&self, m: &[u8], n: &[u8], pk: &[u8], sk: &[u8]) -> (c_int, Vec<u8>, Vec<u8>) {
        let l = m.len();
        let mut cc = prefill(l);
        let mut cr = prefill(l);
        let mut mc = prefill(MAC);
        let mut mr = prefill(MAC);
        let tag = format!("{}(mlen {l})", self.name);
        let rc = unsafe {
            (self.c)(cc.as_mut_ptr(), mc.as_mut_ptr(), m.as_ptr(), l as u64, n.as_ptr(), pk.as_ptr(), sk.as_ptr())
        };
        let rr = unsafe {
            (self.r)(cr.as_mut_ptr(), mr.as_mut_ptr(), m.as_ptr(), l as u64, n.as_ptr(), pk.as_ptr(), sk.as_ptr())
        };
        eqi(&format!("{tag} rc"), rc, rr);
        eqb(&format!("{tag} c"), &cc, &cr);
        eqb(&format!("{tag} mac"), &mc, &mr);
        check_pad(&format!("{tag} c"), &cc, l);
        check_pad(&format!("{tag} mac"), &mc, MAC);
        (rc, cc[..l].to_vec(), mc[..MAC].to_vec())
    }
}

impl P6D {
    /// `f(c, mac, m, mlen, n, k)`; returns `(rc, c, mac)`.
    #[track_caller]
    fn run(&self, m: &[u8], n: &[u8], k: &[u8]) -> (c_int, Vec<u8>, Vec<u8>) {
        let l = m.len();
        let mut cc = prefill(l);
        let mut cr = prefill(l);
        let mut mc = prefill(MAC);
        let mut mr = prefill(MAC);
        let tag = format!("{}(mlen {l})", self.name);
        let rc = unsafe {
            (self.c)(cc.as_mut_ptr(), mc.as_mut_ptr(), m.as_ptr(), l as u64, n.as_ptr(), k.as_ptr())
        };
        let rr = unsafe {
            (self.r)(cr.as_mut_ptr(), mr.as_mut_ptr(), m.as_ptr(), l as u64, n.as_ptr(), k.as_ptr())
        };
        eqi(&format!("{tag} rc"), rc, rr);
        eqb(&format!("{tag} c"), &cc, &cr);
        eqb(&format!("{tag} mac"), &mc, &mr);
        check_pad(&format!("{tag} c"), &cc, l);
        check_pad(&format!("{tag} mac"), &mc, MAC);
        (rc, cc[..l].to_vec(), mc[..MAC].to_vec())
    }
}

impl P7O {
    /// `f(m, c, mac, clen, n, pk, sk)`; returns `(rc, m)`.
    #[track_caller]
    fn run(&self, c: &[u8], mac: &[u8], n: &[u8], pk: &[u8], sk: &[u8]) -> (c_int, Vec<u8>) {
        let l = c.len();
        let mut oc = prefill(l);
        let mut or = prefill(l);
        let tag = format!("{}(clen {l})", self.name);
        let rc = unsafe {
            (self.c)(oc.as_mut_ptr(), c.as_ptr(), mac.as_ptr(), l as u64, n.as_ptr(), pk.as_ptr(), sk.as_ptr())
        };
        let rr = unsafe {
            (self.r)(or.as_mut_ptr(), c.as_ptr(), mac.as_ptr(), l as u64, n.as_ptr(), pk.as_ptr(), sk.as_ptr())
        };
        eqi(&format!("{tag} rc"), rc, rr);
        eqb(&format!("{tag} m"), &oc, &or);
        check_pad(&tag, &oc, l);
        check_pad(&tag, &or, l);
        (rc, oc[..l].to_vec())
    }
}

impl P6O {
    /// `f(m, c, mac, clen, n, k)`; returns `(rc, m)`.
    #[track_caller]
    fn run(&self, c: &[u8], mac: &[u8], n: &[u8], k: &[u8]) -> (c_int, Vec<u8>) {
        let l = c.len();
        let mut oc = prefill(l);
        let mut or = prefill(l);
        let tag = format!("{}(clen {l})", self.name);
        let rc = unsafe {
            (self.c)(oc.as_mut_ptr(), c.as_ptr(), mac.as_ptr(), l as u64, n.as_ptr(), k.as_ptr())
        };
        let rr = unsafe {
            (self.r)(or.as_mut_ptr(), c.as_ptr(), mac.as_ptr(), l as u64, n.as_ptr(), k.as_ptr())
        };
        eqi(&format!("{tag} rc"), rc, rr);
        eqb(&format!("{tag} m"), &oc, &or);
        check_pad(&tag, &oc, l);
        check_pad(&tag, &or, l);
        (rc, oc[..l].to_vec())
    }
}

impl P3 {
    /// `f(k, pk, sk)`; returns `(rc, k)`.
    #[track_caller]
    fn run(&self, pk: &[u8], sk: &[u8]) -> (c_int, Vec<u8>) {
        let mut kc = prefill(32);
        let mut kr = prefill(32);
        let rc = unsafe { (self.c)(kc.as_mut_ptr(), pk.as_ptr(), sk.as_ptr()) };
        let rr = unsafe { (self.r)(kr.as_mut_ptr(), pk.as_ptr(), sk.as_ptr()) };
        eqi(&format!("{} rc", self.name), rc, rr);
        eqb(&format!("{} k", self.name), &kc, &kr);
        check_pad(&self.name, &kc, 32);
        check_pad(&self.name, &kr, 32);
        (rc, kc[..32].to_vec())
    }
}

impl P4 {
    /// `f(c, m, mlen, pk)` — consumes randomness, so the RNG streams are
    /// rewound once before the pair so that C and Rust see identical bytes.
    #[track_caller]
    fn run(&self, outlen: usize, m: &[u8], pk: &[u8], reset: bool) -> (c_int, Vec<u8>) {
        let mut oc = prefill(outlen);
        let mut or = prefill(outlen);
        let tag = format!("{}(mlen {})", self.name, m.len());
        if reset {
            rng_reset();
        }
        let rc =
            unsafe { (self.c)(oc.as_mut_ptr(), m.as_ptr(), m.len() as u64, pk.as_ptr()) };
        let rr =
            unsafe { (self.r)(or.as_mut_ptr(), m.as_ptr(), m.len() as u64, pk.as_ptr()) };
        eqi(&format!("{tag} rc"), rc, rr);
        eqb(&format!("{tag} out"), &oc, &or);
        check_pad(&format!("{tag} C"), &oc, outlen);
        check_pad(&format!("{tag} Rust"), &or, outlen);
        (rc, oc[..outlen].to_vec())
    }
}

// ------------------------------------------------------------------ fixtures

/// The two families that expose the `_easy` / `_detached` / `_seal` API: the
/// xsalsa one is only reachable through the generic `crypto_box_*` names.
const FAMS: [(&str, &str); 2] = [
    ("xsalsa", "crypto_box"),
    ("xchacha", "crypto_box_curve25519xchacha20poly1305"),
];

/// The 7 curve25519 blocklist encodings (`x25519_ref10.c:19-51`); every one
/// makes `crypto_scalarmult_curve25519` — and therefore `_beforenm` — fail.
const CURVE_SMALL_ORDER: [&str; 7] = [
    "0000000000000000000000000000000000000000000000000000000000000000",
    "0100000000000000000000000000000000000000000000000000000000000000",
    "e0eb7a7c3b41b8ae1656e3faf19fc46ada098deb9c32b1fd866205165f49b800",
    "5f9c95bca3508c24b1d0b1559c83ef5b04445cc4581c8e86d8224eddd09f1157",
    "ecffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff7f",
    "edffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff7f",
    "eeffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff7f",
];

/// `(pk, sk)` from a seed, computed by the C library (its agreement with Rust
/// is verified separately in `box_seed_keypair`).
fn keys(seed: &[u8; 32]) -> ([u8; 32], [u8; 32]) {
    let (c, _) = both::<SKP>("crypto_box_seed_keypair");
    let mut pk = [0u8; 32];
    let mut sk = [0u8; 32];
    assert_eq!(unsafe { c(pk.as_mut_ptr(), sk.as_mut_ptr(), seed.as_ptr()) }, 0);
    (pk, sk)
}

fn dense_lengths() -> Vec<usize> {
    (0..=300).collect()
}
fn big_lengths() -> Vec<usize> {
    vec![1023, 1024, 1025, 4096, 8192, 65536, 131_072, 131_073]
}

// ============================================================== 7.79, 7.80

#[test]
fn box_accessors() {
    let common: &[(&str, usize)] = &[
        ("_seedbytes", 32),
        ("_publickeybytes", 32),
        ("_secretkeybytes", 32),
        ("_beforenmbytes", 32),
        ("_noncebytes", 24),
        ("_macbytes", 16),
        ("_messagebytes_max", usize::MAX - 16),
    ];
    for prefix in [
        "crypto_box",
        "crypto_box_curve25519xsalsa20poly1305",
        "crypto_box_curve25519xchacha20poly1305",
    ] {
        for (suffix, want) in common {
            let name = format!("{prefix}{suffix}");
            let (c, r) = both::<SizeFn>(&name);
            let (a, b) = unsafe { (c(), r()) };
            assert_eq!(a, b, "{name}: C {a} vs Rust {b}");
            assert_eq!(a, *want, "{name}: expected {want}, got {a}");
        }
    }
    // The NaCl-padded constants exist only for xsalsa.
    for prefix in ["crypto_box", "crypto_box_curve25519xsalsa20poly1305"] {
        for (suffix, want) in [("_zerobytes", ZERO), ("_boxzerobytes", BOXZERO)] {
            let name = format!("{prefix}{suffix}");
            let (c, r) = both::<SizeFn>(&name);
            let (a, b) = unsafe { (c(), r()) };
            assert_eq!(a, b, "{name}: C {a} vs Rust {b}");
            assert_eq!(a, want, "{name}");
        }
    }
    // `_sealbytes` exists for the generic name and for xchacha.
    for prefix in ["crypto_box", "crypto_box_curve25519xchacha20poly1305"] {
        let name = format!("{prefix}_sealbytes");
        let (c, r) = both::<SizeFn>(&name);
        let (a, b) = unsafe { (c(), r()) };
        assert_eq!(a, b, "{name}");
        assert_eq!(a, SEAL, "{name}");
    }

    let (c, r) = both::<StrFn>("crypto_box_primitive");
    let (a, b) = unsafe { (CStr::from_ptr(c()), CStr::from_ptr(r())) };
    assert_eq!(a, b, "crypto_box_primitive mismatch");
    assert_eq!(a.to_str().unwrap(), "curve25519xsalsa20poly1305");

    // 7.79 — the xchacha subdirectory has no NaCl-padded API and no
    // zero/boxzero constants; the xsalsa one has no easy/detached/seal API and
    // no `_primitive`.  A port must not invent any of them.
    for name in [
        "crypto_box_curve25519xchacha20poly1305",
        "crypto_box_curve25519xchacha20poly1305_open",
        "crypto_box_curve25519xchacha20poly1305_afternm",
        "crypto_box_curve25519xchacha20poly1305_open_afternm",
        "crypto_box_curve25519xchacha20poly1305_zerobytes",
        "crypto_box_curve25519xchacha20poly1305_boxzerobytes",
        "crypto_box_curve25519xchacha20poly1305_primitive",
        "crypto_box_curve25519xsalsa20poly1305_easy",
        "crypto_box_curve25519xsalsa20poly1305_easy_afternm",
        "crypto_box_curve25519xsalsa20poly1305_open_easy",
        "crypto_box_curve25519xsalsa20poly1305_open_easy_afternm",
        "crypto_box_curve25519xsalsa20poly1305_detached",
        "crypto_box_curve25519xsalsa20poly1305_detached_afternm",
        "crypto_box_curve25519xsalsa20poly1305_open_detached",
        "crypto_box_curve25519xsalsa20poly1305_open_detached_afternm",
        "crypto_box_curve25519xsalsa20poly1305_seal",
        "crypto_box_curve25519xsalsa20poly1305_seal_open",
        "crypto_box_curve25519xsalsa20poly1305_sealbytes",
        "crypto_box_curve25519xsalsa20poly1305_primitive",
    ] {
        assert_absent(name);
    }

    // ... while these *are* present in both libraries.
    for name in [
        "crypto_box_seed_keypair",
        "crypto_box_keypair",
        "crypto_box_beforenm",
        "crypto_box_afternm",
        "crypto_box_open_afternm",
        "crypto_box",
        "crypto_box_open",
        "crypto_box_easy",
        "crypto_box_easy_afternm",
        "crypto_box_open_easy",
        "crypto_box_open_easy_afternm",
        "crypto_box_detached",
        "crypto_box_detached_afternm",
        "crypto_box_open_detached",
        "crypto_box_open_detached_afternm",
        "crypto_box_seal",
        "crypto_box_seal_open",
        "crypto_box_curve25519xsalsa20poly1305",
        "crypto_box_curve25519xsalsa20poly1305_open",
        "crypto_box_curve25519xsalsa20poly1305_afternm",
        "crypto_box_curve25519xsalsa20poly1305_open_afternm",
        "crypto_box_curve25519xchacha20poly1305_easy",
        "crypto_box_curve25519xchacha20poly1305_easy_afternm",
        "crypto_box_curve25519xchacha20poly1305_open_easy",
        "crypto_box_curve25519xchacha20poly1305_open_easy_afternm",
        "crypto_box_curve25519xchacha20poly1305_detached",
        "crypto_box_curve25519xchacha20poly1305_detached_afternm",
        "crypto_box_curve25519xchacha20poly1305_open_detached",
        "crypto_box_curve25519xchacha20poly1305_open_detached_afternm",
        "crypto_box_curve25519xchacha20poly1305_seal",
        "crypto_box_curve25519xchacha20poly1305_seal_open",
    ] {
        assert!(present(c_lib(), name), "C is missing {name}");
        assert!(present(rust_lib(), name), "Rust is missing {name}");
    }
}

// ============================================================== 7.60, 7.72

#[test]
fn box_keypair() {
    let names = [
        "crypto_box_keypair",
        "crypto_box_curve25519xsalsa20poly1305_keypair",
        "crypto_box_curve25519xchacha20poly1305_keypair",
    ];
    let (bc, br) = both::<MultBase>("crypto_scalarmult_curve25519_base");

    for round in 0..12u64 {
        let mut all: Vec<Vec<u8>> = Vec::new();
        for name in names {
            let (c, r) = both::<KP>(name);
            rng_reseed(0x9000 + round);
            let mut pkc = prefill(32);
            let mut skc = prefill(32);
            let rc = unsafe { c(pkc.as_mut_ptr(), skc.as_mut_ptr()) };
            let mut pkr = prefill(32);
            let mut skr = prefill(32);
            let rr = unsafe { r(pkr.as_mut_ptr(), skr.as_mut_ptr()) };
            eqi(&format!("{name} rc"), rc, rr);
            eqb(&format!("{name} pk"), &pkc, &pkr);
            eqb(&format!("{name} sk"), &skc, &skr);
            check_pad(name, &pkc, 32);
            check_pad(name, &skc, 32);
            assert_eq!(rc, 0, "{name} cannot fail");

            // pk == crypto_scalarmult_curve25519_base(sk).
            let mut ec = prefill(32);
            let mut er = prefill(32);
            unsafe {
                bc(ec.as_mut_ptr(), skc.as_ptr());
                br(er.as_mut_ptr(), skr.as_ptr());
            }
            eqb("scalarmult_base agreement", &ec, &er);
            eqb(&format!("{name}: pk == base(sk)"), &pkc[..32], &ec[..32]);

            let mut v = pkc[..32].to_vec();
            v.extend_from_slice(&skc[..32]);
            all.push(v);
        }
        // 7.72 — all three key-generation entry points are the same code.
        eqb("keypair: generic == xsalsa", &all[0], &all[1]);
        eqb("keypair: xsalsa == xchacha", &all[1], &all[2]);
    }

    // Successive draws differ.
    let (c, _) = both::<KP>("crypto_box_keypair");
    rng_reset();
    let mut p1 = [0u8; 32];
    let mut s1 = [0u8; 32];
    let mut p2 = [0u8; 32];
    let mut s2 = [0u8; 32];
    unsafe {
        c(p1.as_mut_ptr(), s1.as_mut_ptr());
        c(p2.as_mut_ptr(), s2.as_mut_ptr());
    }
    assert_ne!(s1, s2, "successive secret keys must differ");
}

// ============================================================== 7.61, 7.72

#[test]
fn box_seed_keypair() {
    let names = [
        "crypto_box_seed_keypair",
        "crypto_box_curve25519xsalsa20poly1305_seed_keypair",
        "crypto_box_curve25519xchacha20poly1305_seed_keypair",
    ];
    let (bc, br) = both::<MultBase>("crypto_scalarmult_curve25519_base");
    let (hc, hr) = both::<HashFn>("crypto_hash_sha512");

    let mut seeds: Vec<[u8; 32]> = vec![
        [0u8; 32],
        [0xffu8; 32],
        h32("0102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f20"),
    ];
    let mut rng = Rng::new(0x0B0_5EED_0001);
    for _ in 0..16 {
        seeds.push(rng.bytes(32).try_into().unwrap());
    }

    for seed in &seeds {
        let mut all: Vec<Vec<u8>> = Vec::new();
        for name in names {
            let (c, r) = both::<SKP>(name);
            let mut pkc = prefill(32);
            let mut skc = prefill(32);
            let rc = unsafe { c(pkc.as_mut_ptr(), skc.as_mut_ptr(), seed.as_ptr()) };
            let mut pkr = prefill(32);
            let mut skr = prefill(32);
            let rr = unsafe { r(pkr.as_mut_ptr(), skr.as_mut_ptr(), seed.as_ptr()) };
            eqi(&format!("{name} rc"), rc, rr);
            eqb(&format!("{name} pk"), &pkc, &pkr);
            eqb(&format!("{name} sk"), &skc, &skr);
            check_pad(name, &pkc, 32);
            check_pad(name, &skc, 32);
            assert_eq!(rc, 0);
            let mut v = pkc[..32].to_vec();
            v.extend_from_slice(&skc[..32]);
            all.push(v);
        }
        eqb("seed_keypair: generic == xsalsa", &all[0], &all[1]);
        eqb("seed_keypair: xsalsa == xchacha", &all[1], &all[2]);

        // sk = SHA-512(seed)[0..32], *unclamped*; pk = base(sk).
        let mut dc = [0u8; 64];
        let mut dr = [0u8; 64];
        unsafe {
            hc(dc.as_mut_ptr(), seed.as_ptr(), 32);
            hr(dr.as_mut_ptr(), seed.as_ptr(), 32);
        }
        eqb("crypto_hash_sha512(seed)", &dc, &dr);
        eqb("sk == SHA-512(seed)[0..32]", &all[0][32..], &dc[..32]);
        let mut ec = prefill(32);
        let mut er = prefill(32);
        unsafe {
            bc(ec.as_mut_ptr(), dc.as_ptr());
            br(er.as_mut_ptr(), dr.as_ptr());
        }
        eqb("base() agreement", &ec, &er);
        eqb("pk == base(sk)", &all[0][..32], &ec[..32]);
    }
}

// ============================================================== 7.59, 7.66, 7.73, 7.81

#[test]
fn box_beforenm() {
    let xs = P3::new("crypto_box_curve25519xsalsa20poly1305_beforenm");
    let gen = P3::new("crypto_box_beforenm");
    let xc = P3::new("crypto_box_curve25519xchacha20poly1305_beforenm");

    let mut rng = Rng::new(0x0BEF_0000_0002);
    for _ in 0..16 {
        let (pk_a, sk_a) = keys(&rng.bytes(32).try_into().unwrap());
        let (pk_b, sk_b) = keys(&rng.bytes(32).try_into().unwrap());

        let (rc, k1) = xs.run(&pk_b, &sk_a);
        eqi("xsalsa beforenm rc", rc, 0);
        let (rc, k2) = xs.run(&pk_a, &sk_b);
        eqi("xsalsa beforenm rc", rc, 0);
        // 7.66 — beforenm(pkB, skA) == beforenm(pkA, skB).
        eqb("beforenm symmetry", &k1, &k2);
        // The generic name is a pure alias.
        let (rc, kg) = gen.run(&pk_b, &sk_a);
        eqi("generic beforenm rc", rc, 0);
        eqb("generic beforenm == xsalsa", &kg, &k1);
        // 7.73 — xchacha differs (HChaCha20 vs HSalsa20 of the same secret).
        let (rc, k3) = xc.run(&pk_b, &sk_a);
        eqi("xchacha beforenm rc", rc, 0);
        let (rc, k4) = xc.run(&pk_a, &sk_b);
        eqi("xchacha beforenm rc", rc, 0);
        eqb("xchacha beforenm symmetry", &k3, &k4);
        assert_ne!(hex(&k1), hex(&k3), "xsalsa and xchacha keys must differ");
    }

    // 7.59 / 7.81 — every blocklisted small-order pk makes beforenm fail and
    // leaves `k` untouched, including with bit 255 set.
    let (_, sk) = keys(&rng.bytes(32).try_into().unwrap());
    for enc in CURVE_SMALL_ORDER {
        for hi in [false, true] {
            let mut pk = h32(enc);
            if hi {
                pk[31] |= 0x80;
            }
            for p in [&xs, &gen, &xc] {
                let (rc, k) = p.run(&pk, &sk);
                eqi(&format!("small-order pk {enc} hi={hi}"), rc, -1);
                eqb("beforenm leaves k untouched", &k, &pattern(32));
            }
        }
    }
    // A degenerate but non-blocklisted pk (all 0xff) is accepted.
    for p in [&xs, &gen, &xc] {
        let (rc, _) = p.run(&[0xffu8; 32], &sk);
        eqi("pk = ff..ff", rc, 0);
    }
    // Degenerate secret keys are always fine (the scalar is clamped).
    for skp in [[0u8; 32], [0xffu8; 32]] {
        let (pk, _) = keys(&rng.bytes(32).try_into().unwrap());
        for p in [&xs, &gen, &xc] {
            let (rc, _) = p.run(&pk, &skp);
            eqi("degenerate sk", rc, 0);
        }
    }
}

// ============================================================== 7.62–7.66, 7.74–7.77

#[test]
fn box_easy_and_detached_dense() {
    let mut rng = Rng::new(0x0EA5_0000_0003);
    let (pk_a, sk_a) = keys(&rng.bytes(32).try_into().unwrap());
    let (pk_b, sk_b) = keys(&rng.bytes(32).try_into().unwrap());
    let nonce = rng.bytes(NONCE);

    for (tag, prefix) in FAMS {
        let easy = P6::new(&format!("{prefix}_easy"));
        let open_easy = P6::new(&format!("{prefix}_open_easy"));
        let det = P7D::new(&format!("{prefix}_detached"));
        let odet = P7O::new(&format!("{prefix}_open_detached"));
        let ea = P5::new(&format!("{prefix}_easy_afternm"));
        let oea = P5::new(&format!("{prefix}_open_easy_afternm"));
        let da = P6D::new(&format!("{prefix}_detached_afternm"));
        let oda = P6O::new(&format!("{prefix}_open_detached_afternm"));
        let bn = P3::new(&format!("{prefix}_beforenm"));

        let (rc, k) = bn.run(&pk_b, &sk_a);
        eqi("beforenm", rc, 0);
        let (rc, k2) = bn.run(&pk_a, &sk_b);
        eqi("beforenm", rc, 0);
        eqb("beforenm symmetry", &k, &k2);

        let mut prev: Option<Vec<u8>> = None;
        for len in dense_lengths() {
            let m = rng.bytes(len);

            // 7.62 / 7.74 — _easy round trip; MAC at c[0..16], body at c[16..].
            let (rc, ct) = easy.run(len + MAC, &m, &nonce, &pk_b, &sk_a);
            eqi(&format!("{tag} easy rc"), rc, 0);
            let (rc, pt) = open_easy.run(len, &ct, &nonce, &pk_a, &sk_b);
            eqi(&format!("{tag} open_easy rc"), rc, 0);
            eqb(&format!("{tag} easy round trip"), &pt, &m);

            // 7.64 — _detached produces the same bytes, reordered.
            let (rc, body, mac) = det.run(&m, &nonce, &pk_b, &sk_a);
            eqi(&format!("{tag} detached rc"), rc, 0);
            eqb(&format!("{tag} detached mac == c[0..16]"), &mac, &ct[..MAC]);
            eqb(&format!("{tag} detached body == c[16..]"), &body, &ct[MAC..]);
            let (rc, pt) = odet.run(&body, &mac, &nonce, &pk_a, &sk_b);
            eqi(&format!("{tag} open_detached rc"), rc, 0);
            eqb(&format!("{tag} detached round trip"), &pt, &m);

            // 7.65 / 7.76 — the precomputed forms must be bit-identical.
            let (rc, ct2) = ea.run(len + MAC, &m, &nonce, &k);
            eqi(&format!("{tag} easy_afternm rc"), rc, 0);
            eqb(&format!("{tag} easy_afternm == easy"), &ct2, &ct);
            let (rc, pt) = oea.run(len, &ct, &nonce, &k);
            eqi(&format!("{tag} open_easy_afternm rc"), rc, 0);
            eqb(&format!("{tag} open_easy_afternm round trip"), &pt, &m);

            // 7.66 / 7.77 — precomputed detached.
            let (rc, body2, mac2) = da.run(&m, &nonce, &k);
            eqi(&format!("{tag} detached_afternm rc"), rc, 0);
            eqb(&format!("{tag} detached_afternm body"), &body2, &body);
            eqb(&format!("{tag} detached_afternm mac"), &mac2, &mac);
            let (rc, pt) = oda.run(&body, &mac, &nonce, &k);
            eqi(&format!("{tag} open_detached_afternm rc"), rc, 0);
            eqb(&format!("{tag} open_detached_afternm round trip"), &pt, &m);

            // Distinct plaintexts give distinct ciphertexts.
            if let Some(p) = &prev {
                if len > 0 {
                    assert_ne!(hex(p), hex(&ct));
                }
            }
            prev = Some(ct);
        }
    }
}

#[test]
fn box_easy_big_messages() {
    let mut rng = Rng::new(0x0B16_0000_0004);
    let (pk_a, sk_a) = keys(&rng.bytes(32).try_into().unwrap());
    let (pk_b, sk_b) = keys(&rng.bytes(32).try_into().unwrap());
    let nonce = rng.bytes(NONCE);
    for (tag, prefix) in FAMS {
        let easy = P6::new(&format!("{prefix}_easy"));
        let open_easy = P6::new(&format!("{prefix}_open_easy"));
        let ea = P5::new(&format!("{prefix}_easy_afternm"));
        let bn = P3::new(&format!("{prefix}_beforenm"));
        let (_, k) = bn.run(&pk_b, &sk_a);
        for len in big_lengths() {
            let m = rng.bytes(len);
            let (rc, ct) = easy.run(len + MAC, &m, &nonce, &pk_b, &sk_a);
            eqi(&format!("{tag} big easy rc"), rc, 0);
            let (rc, pt) = open_easy.run(len, &ct, &nonce, &pk_a, &sk_b);
            eqi(&format!("{tag} big open_easy rc"), rc, 0);
            eqb(&format!("{tag} big round trip"), &pt, &m);
            // Crosses the 131072-byte STREAM_POLY1305_CHUNK boundary.
            let (rc, ct2) = ea.run(len + MAC, &m, &nonce, &k);
            eqi(&format!("{tag} big easy_afternm rc"), rc, 0);
            eqb(&format!("{tag} big easy_afternm == easy"), &ct2, &ct);
            // A flipped bit in the body must be caught.
            let mut bad = ct.clone();
            bad[MAC + len / 2] ^= 0x08;
            let (rc, _) = open_easy.run(len, &bad, &nonce, &pk_a, &sk_b);
            eqi(&format!("{tag} big tampered body"), rc, -1);
        }
    }
}

// ============================================================== 7.63

#[test]
fn box_easy_inplace() {
    let mut rng = Rng::new(0x0011_9000_0009);
    let (pk_a, sk_a) = keys(&rng.bytes(32).try_into().unwrap());
    let (pk_b, sk_b) = keys(&rng.bytes(32).try_into().unwrap());
    let nonce = rng.bytes(NONCE);

    for (tag, prefix) in FAMS {
        let (ec, er) = both::<F6>(&format!("{prefix}_easy"));
        let (oc, or) = both::<F6>(&format!("{prefix}_open_easy"));
        let plain = P6::new(&format!("{prefix}_easy"));

        for len in [0usize, 1, 15, 16, 17, 31, 32, 33, 63, 64, 65, 127, 300, 1024] {
            let m = rng.bytes(len);
            let (_, want_ct) = plain.run(len + MAC, &m, &nonce, &pk_b, &sk_a);

            // The documented in-place pattern: `m == c + MACBYTES`.
            let mut bc = prefill(len + MAC);
            bc[MAC..MAC + len].copy_from_slice(&m);
            let mut br = bc.clone();
            let pc = bc.as_mut_ptr();
            let pr = br.as_mut_ptr();
            let a = unsafe {
                ec(pc, pc.add(MAC) as *const u8, len as u64, nonce.as_ptr(), pk_b.as_ptr(), sk_a.as_ptr())
            };
            let b = unsafe {
                er(pr, pr.add(MAC) as *const u8, len as u64, nonce.as_ptr(), pk_b.as_ptr(), sk_a.as_ptr())
            };
            eqi(&format!("{tag} in-place easy rc"), a, b);
            eqb(&format!("{tag} in-place easy buf"), &bc, &br);
            check_pad("in-place easy", &bc, len + MAC);
            assert_eq!(a, 0);
            eqb(&format!("{tag} in-place easy == out-of-place"), &bc[..len + MAC], &want_ct);

            // And on the way back: `m == c`, which shifts the plaintext down.
            let ct = bc[..len + MAC].to_vec();
            let mut bc = prefill(len + MAC);
            bc[..len + MAC].copy_from_slice(&ct);
            let mut br = bc.clone();
            let pc = bc.as_mut_ptr();
            let pr = br.as_mut_ptr();
            let a = unsafe {
                oc(pc, pc as *const u8, (len + MAC) as u64, nonce.as_ptr(), pk_a.as_ptr(), sk_b.as_ptr())
            };
            let b = unsafe {
                or(pr, pr as *const u8, (len + MAC) as u64, nonce.as_ptr(), pk_a.as_ptr(), sk_b.as_ptr())
            };
            eqi(&format!("{tag} in-place open rc"), a, b);
            eqb(&format!("{tag} in-place open buf"), &bc, &br);
            check_pad("in-place open", &bc, len + MAC);
            assert_eq!(a, 0);
            eqb(&format!("{tag} in-place open plaintext"), &bc[..len], &m);
        }
    }
}

// ============================================================== 7.64, 7.65, 7.83–7.85

#[test]
fn box_clen_edges() {
    let mut rng = Rng::new(0x0C1E_0000_0005);
    let (pk_a, sk_a) = keys(&rng.bytes(32).try_into().unwrap());
    let (pk_b, sk_b) = keys(&rng.bytes(32).try_into().unwrap());
    let nonce = rng.bytes(NONCE);

    for (tag, prefix) in FAMS {
        let easy = P6::new(&format!("{prefix}_easy"));
        let open_easy = P6::new(&format!("{prefix}_open_easy"));
        let oea = P5::new(&format!("{prefix}_open_easy_afternm"));
        let oda = P6O::new(&format!("{prefix}_open_detached_afternm"));
        let odet = P7O::new(&format!("{prefix}_open_detached"));
        let bn = P3::new(&format!("{prefix}_beforenm"));
        let (_, k) = bn.run(&pk_b, &sk_a);

        // A real 1-byte ciphertext to slice up.
        let m = rng.bytes(1);
        let (_, ct) = easy.run(1 + MAC, &m, &nonce, &pk_b, &sk_a);

        // clen ∈ 0..16 → `-1` before any crypto, `m` untouched.
        for clen in 0..MAC {
            let c = &ct[..clen];
            let (rc, out) = open_easy.run(8, c, &nonce, &pk_a, &sk_b);
            eqi(&format!("{tag} open_easy clen={clen}"), rc, -1);
            eqb("m untouched", &out, &pattern(8));
            let (rc, out) = oea.run(8, c, &nonce, &k);
            eqi(&format!("{tag} open_easy_afternm clen={clen}"), rc, -1);
            eqb("m untouched", &out, &pattern(8));
        }

        // clen == 16 is legal: an empty message with only its MAC.
        let (_, ct0) = easy.run(MAC, &[], &nonce, &pk_b, &sk_a);
        let (rc, out) = open_easy.run(0, &ct0, &nonce, &pk_a, &sk_b);
        eqi(&format!("{tag} open_easy clen=16"), rc, 0);
        assert!(out.is_empty());
        let (rc, _) = oea.run(0, &ct0, &nonce, &k);
        eqi(&format!("{tag} open_easy_afternm clen=16"), rc, 0);
        // A tampered 16-byte ciphertext is rejected.
        for pos in 0..MAC {
            let mut bad = ct0.clone();
            bad[pos] ^= 1;
            let (rc, _) = open_easy.run(0, &bad, &nonce, &pk_a, &sk_b);
            eqi(&format!("{tag} clen=16 tampered at {pos}"), rc, -1);
        }

        // clen == 17: a real 1-byte message.
        let (rc, out) = open_easy.run(1, &ct, &nonce, &pk_a, &sk_b);
        eqi(&format!("{tag} open_easy clen=17"), rc, 0);
        eqb("1-byte round trip", &out, &m);

        // 7.68 / 7.85 — the `*_open_detached*` forms have **no** length guard
        // at all, so `clen == 0` is valid.
        let (rc, out) = oda.run(&[], &ct[..MAC], &nonce, &k);
        eqi(&format!("{tag} open_detached_afternm clen=0"), rc, -1);
        assert!(out.is_empty());
        let (_, _, ct0d) = P7D::new(&format!("{prefix}_detached")).run(&[], &nonce, &pk_b, &sk_a);
        let (rc, _) = oda.run(&[], &ct0d, &nonce, &k);
        eqi(&format!("{tag} open_detached_afternm clen=0 valid mac"), rc, 0);
        let (rc, _) = odet.run(&[], &ct0d, &nonce, &pk_a, &sk_b);
        eqi(&format!("{tag} open_detached clen=0 valid mac"), rc, 0);
    }
}

// ============================================================== 7.66–7.71 (MAC tampering)

#[test]
fn box_tag_and_body_corruption() {
    let mut rng = Rng::new(0x0_7A6_0000_0006);
    let (pk_a, sk_a) = keys(&rng.bytes(32).try_into().unwrap());
    let (pk_b, sk_b) = keys(&rng.bytes(32).try_into().unwrap());
    let (pk_x, sk_x) = keys(&rng.bytes(32).try_into().unwrap());
    let nonce = rng.bytes(NONCE);
    let mut other_nonce = nonce.clone();
    other_nonce[0] ^= 1;

    for (tag, prefix) in FAMS {
        let easy = P6::new(&format!("{prefix}_easy"));
        let open_easy = P6::new(&format!("{prefix}_open_easy"));
        let det = P7D::new(&format!("{prefix}_detached"));
        let odet = P7O::new(&format!("{prefix}_open_detached"));
        let bn = P3::new(&format!("{prefix}_beforenm"));
        let oea = P5::new(&format!("{prefix}_open_easy_afternm"));
        let oda = P6O::new(&format!("{prefix}_open_detached_afternm"));
        let (_, k) = bn.run(&pk_b, &sk_a);

        for len in [0usize, 1, 15, 16, 17, 31, 32, 63, 64, 65, 100] {
            let m = rng.bytes(len);
            let (_, ct) = easy.run(len + MAC, &m, &nonce, &pk_b, &sk_a);
            let (_, body, mac) = det.run(&m, &nonce, &pk_b, &sk_a);

            // Every byte position of the full easy ciphertext (tag + body).
            for pos in 0..ct.len() {
                let mut bad = ct.clone();
                bad[pos] ^= 0x80;
                let (rc, out) = open_easy.run(len, &bad, &nonce, &pk_a, &sk_b);
                eqi(&format!("{tag} open_easy corrupt {pos}"), rc, -1);
                // `crypto_secretbox_open_detached` returns before touching `m`.
                eqb("m on MAC failure", &out, &pattern(len));
                let (rc, out) = oea.run(len, &bad, &nonce, &k);
                eqi(&format!("{tag} open_easy_afternm corrupt {pos}"), rc, -1);
                eqb("m on MAC failure", &out, &pattern(len));
            }
            // Every byte position of the detached MAC and body.
            for pos in 0..MAC {
                let mut bm = mac.clone();
                bm[pos] ^= 0x01;
                let (rc, out) = odet.run(&body, &bm, &nonce, &pk_a, &sk_b);
                eqi(&format!("{tag} open_detached corrupt mac {pos}"), rc, -1);
                eqb("m on MAC failure", &out, &pattern(len));
                let (rc, _) = oda.run(&body, &bm, &nonce, &k);
                eqi(&format!("{tag} open_detached_afternm corrupt mac {pos}"), rc, -1);
            }
            for pos in 0..len {
                let mut bb = body.clone();
                bb[pos] ^= 0x40;
                let (rc, _) = odet.run(&bb, &mac, &nonce, &pk_a, &sk_b);
                eqi(&format!("{tag} open_detached corrupt body {pos}"), rc, -1);
            }

            // 7.71 — the right ciphertext with the wrong nonce.
            let (rc, out) = open_easy.run(len, &ct, &other_nonce, &pk_a, &sk_b);
            eqi(&format!("{tag} wrong nonce"), rc, -1);
            eqb("m on MAC failure", &out, &pattern(len));
            // Mismatched key pair.
            let (rc, _) = open_easy.run(len, &ct, &nonce, &pk_x, &sk_b);
            eqi(&format!("{tag} wrong sender pk"), rc, -1);
            let (rc, _) = open_easy.run(len, &ct, &nonce, &pk_a, &sk_x);
            eqi(&format!("{tag} wrong recipient sk"), rc, -1);
            // ... and the correct one still works.
            let (rc, out) = open_easy.run(len, &ct, &nonce, &pk_a, &sk_b);
            eqi(&format!("{tag} baseline"), rc, 0);
            eqb("baseline round trip", &out, &m);
        }
    }
}

// ============================================================== 7.67, 7.68 (NaCl padded)

#[test]
fn box_nacl_padded() {
    // Only xsalsa has the zero-padded convention.
    let bx = P6::new("crypto_box");
    let bxo = P6::new("crypto_box_open");
    let xs = P6::new("crypto_box_curve25519xsalsa20poly1305");
    let xso = P6::new("crypto_box_curve25519xsalsa20poly1305_open");
    let an = P5::new("crypto_box_afternm");
    let ano = P5::new("crypto_box_open_afternm");
    let xan = P5::new("crypto_box_curve25519xsalsa20poly1305_afternm");
    let xano = P5::new("crypto_box_curve25519xsalsa20poly1305_open_afternm");
    let bn = P3::new("crypto_box_beforenm");

    let mut rng = Rng::new(0x00AC_1000_000A);
    let (pk_a, sk_a) = keys(&rng.bytes(32).try_into().unwrap());
    let (pk_b, sk_b) = keys(&rng.bytes(32).try_into().unwrap());
    let nonce = rng.bytes(NONCE);
    let (rc, k) = bn.run(&pk_b, &sk_a);
    eqi("beforenm", rc, 0);

    // mlen < ZEROBYTES (32) is rejected by crypto_secretbox_xsalsa20poly1305.
    for mlen in 0..40usize {
        let mut m = vec![0u8; mlen];
        if mlen > ZERO {
            let tail = rng.bytes(mlen - ZERO);
            m[ZERO..].copy_from_slice(&tail);
        }
        let (rc, c) = bx.run(mlen, &m, &nonce, &pk_b, &sk_a);
        let (rc2, c2) = xs.run(mlen, &m, &nonce, &pk_b, &sk_a);
        eqi("crypto_box == xsalsa rc", rc, rc2);
        eqb("crypto_box == xsalsa out", &c, &c2);
        let (rc3, c3) = an.run(mlen, &m, &nonce, &k);
        let (rc4, c4) = xan.run(mlen, &m, &nonce, &k);
        eqi("afternm == xsalsa afternm rc", rc3, rc4);
        eqb("afternm == xsalsa afternm out", &c3, &c4);
        eqi("crypto_box == crypto_box_afternm rc", rc, rc3);
        if rc == 0 {
            assert!(mlen >= ZERO, "mlen {mlen} should have been rejected");
            eqb("crypto_box == afternm out", &c, &c3);
            // c[0..16] is zero padding on output.
            eqb("c[0..16] == 0", &c[..BOXZERO], &vec![0u8; BOXZERO]);
            // Round trip.
            let (rc, p) = bxo.run(mlen, &c, &nonce, &pk_a, &sk_b);
            eqi("crypto_box_open rc", rc, 0);
            let (rc2, p2) = xso.run(mlen, &c, &nonce, &pk_a, &sk_b);
            eqi("xsalsa open rc", rc2, 0);
            eqb("open == xsalsa open", &p, &p2);
            let (rc3, p3) = ano.run(mlen, &c, &nonce, &k);
            eqi("open_afternm rc", rc3, 0);
            let (rc4, p4) = xano.run(mlen, &c, &nonce, &k);
            eqi("xsalsa open_afternm rc", rc4, 0);
            eqb("open_afternm == xsalsa", &p3, &p4);
            eqb("open == open_afternm", &p, &p3);
            // m[0..32] is zeroed on output, and the body round-trips.
            eqb("m[0..32] == 0", &p[..ZERO], &vec![0u8; ZERO]);
            eqb("padded round trip", &p[ZERO..], &m[ZERO..]);
        } else {
            eqi("short padded message rejected", rc, -1);
            assert!(mlen < ZERO);
            eqb("c untouched", &c, &pattern(mlen));
        }
        // Opening a too-short ciphertext.
        let (rc, out) = bxo.run(mlen, &vec![0u8; mlen], &nonce, &pk_a, &sk_b);
        let (rc2, out2) = xso.run(mlen, &vec![0u8; mlen], &nonce, &pk_a, &sk_b);
        eqi("open all-zero rc agreement", rc, rc2);
        eqb("open all-zero out agreement", &out, &out2);
        assert_eq!(rc, -1, "an all-zero {mlen}-byte box must not open");
    }

    // 7.67 — the documented lengths, with a tampered tag at every position.
    for mlen in [ZERO, 33, 48, 64, 1056] {
        let mut m = vec![0u8; mlen];
        let tail = rng.bytes(mlen - ZERO);
        m[ZERO..].copy_from_slice(&tail);
        let (rc, c) = bx.run(mlen, &m, &nonce, &pk_b, &sk_a);
        eqi("crypto_box rc", rc, 0);
        for pos in BOXZERO..mlen {
            let mut bad = c.clone();
            bad[pos] ^= 0x11;
            let (rc, out) = bxo.run(mlen, &bad, &nonce, &pk_a, &sk_b);
            eqi(&format!("crypto_box_open tampered at {pos}"), rc, -1);
            eqb("m untouched on failure", &out, &pattern(mlen));
        }
        // The leading 16 zero bytes of `c` are not authenticated: flipping one
        // must behave identically in both libraries (whatever that is).
        for pos in 0..BOXZERO {
            let mut bad = c.clone();
            bad[pos] ^= 0x11;
            let (_rc, _out) = bxo.run(mlen, &bad, &nonce, &pk_a, &sk_b);
        }
    }
}

// ============================================================== 7.69–7.71, 7.78, 7.87–7.89

#[test]
fn box_seal() {
    let mut rng = Rng::new(0x0_5EA1_0000_0007);
    let (pk_a, sk_a) = keys(&rng.bytes(32).try_into().unwrap());
    let (pk_x, sk_x) = keys(&rng.bytes(32).try_into().unwrap());
    let (bc, br) = both::<MultBase>("crypto_scalarmult_curve25519_base");

    for (tag, prefix) in FAMS {
        let seal = P4::new(&format!("{prefix}_seal"));
        let open = P5::new(&format!("{prefix}_seal_open"));

        let mut seen: Vec<Vec<u8>> = Vec::new();
        let mut lens: Vec<usize> = (0..=80).collect();
        lens.extend_from_slice(&[100, 127, 128, 129, 255, 256, 300, 1024, 4096]);
        rng_reset();
        for len in lens {
            let m = rng.bytes(len);
            // 7.69 — c is SEALBYTES + mlen; c[0..32] is a fresh ephemeral pk.
            let (rc, c) = seal.run(len + SEAL, &m, &pk_a, false);
            eqi(&format!("{tag} seal rc"), rc, 0);
            assert!(!seen.contains(&c[..32].to_vec()), "ephemeral pk repeated");
            seen.push(c[..32].to_vec());

            // 7.71 — the recipient can open with (pk_a, sk_a) alone.
            let (rc, p) = open.run(len, &c, &pk_a, &sk_a);
            eqi(&format!("{tag} seal_open rc"), rc, 0);
            eqb(&format!("{tag} seal round trip"), &p, &m);

            // The wrong recipient fails (7.73 / 7.88).
            let (rc, out) = open.run(len, &c, &pk_x, &sk_x);
            eqi(&format!("{tag} seal_open wrong recipient"), rc, -1);
            eqb("m on failure", &out, &pattern(len));
            // A mismatched (pk, sk) pair also fails: the derived nonce and the
            // shared secret both change.
            let (rc, _) = open.run(len, &c, &pk_x, &sk_a);
            eqi(&format!("{tag} seal_open pk/sk mismatch"), rc, -1);
            let (rc, _) = open.run(len, &c, &pk_a, &sk_x);
            eqi(&format!("{tag} seal_open wrong sk"), rc, -1);

            if len <= 8 {
                // 7.74 / 7.75 — tamper with every byte of the header, the MAC
                // and the body.
                for pos in 0..c.len() {
                    let mut bad = c.clone();
                    bad[pos] ^= 0x20;
                    let (rc, out) = open.run(len, &bad, &pk_a, &sk_a);
                    eqi(&format!("{tag} seal_open tampered at {pos}"), rc, -1);
                    eqb("m on failure", &out, &pattern(len));
                }
                // 7.76 — a small-order embedded ephemeral pk.
                for enc in CURVE_SMALL_ORDER {
                    let mut bad = c.clone();
                    bad[..32].copy_from_slice(&h32(enc));
                    let (rc, out) = open.run(len, &bad, &pk_a, &sk_a);
                    eqi(&format!("{tag} seal_open small-order epk"), rc, -1);
                    eqb("m on failure", &out, &pattern(len));
                }
            }
        }

        // 7.70 / 7.72 — clen < 48 rejected before any work, clen == 48 legal.
        let (_, c0) = seal.run(SEAL, &[], &pk_a, false);
        for clen in 0..SEAL {
            let (rc, out) = open.run(8, &c0[..clen], &pk_a, &sk_a);
            eqi(&format!("{tag} seal_open clen={clen}"), rc, -1);
            eqb("m untouched", &out, &pattern(8));
        }
        let (rc, out) = open.run(0, &c0, &pk_a, &sk_a);
        eqi(&format!("{tag} seal_open clen=48"), rc, 0);
        assert!(out.is_empty());

        // 7.77 / 7.89 — a small-order recipient pk makes the inner
        // `_easy` fail, but `memcpy(c, epk, 32)` has already run, so `c[0..32]`
        // *is* written while `c[32..]` is not.
        for enc in CURVE_SMALL_ORDER {
            let pk = h32(enc);
            let m = rng.bytes(24);
            let (rc, c) = seal.run(24 + SEAL, &m, &pk, true);
            eqi(&format!("{tag} seal small-order pk rc"), rc, -1);
            // The ephemeral pk written into c[0..32] must be base(esk) for the
            // esk the deterministic RNG produced.
            // Re-derive the ephemeral secret key from the (rewound) stream.
            // Both streams are advanced by the same amount so that the next
            // `seal` pair stays in lockstep.
            rng_reset();
            let mut eskc = [0u8; 32];
            let mut eskr = [0u8; 32];
            {
                type BufFn = unsafe extern "C" fn(*mut c_void, usize);
                let (rbc, rbr) = both::<BufFn>("randombytes_buf");
                unsafe {
                    rbc(eskc.as_mut_ptr() as *mut c_void, 32);
                    rbr(eskr.as_mut_ptr() as *mut c_void, 32);
                }
            }
            eqb("deterministic RNG streams agree", &eskc, &eskr);
            let mut epc = prefill(32);
            let mut epr = prefill(32);
            unsafe {
                bc(epc.as_mut_ptr(), eskc.as_ptr());
                br(epr.as_mut_ptr(), eskr.as_ptr());
            }
            eqb("base(esk) agreement", &epc, &epr);
            eqb(
                &format!("{tag} seal wrote the ephemeral pk despite failing"),
                &c[..32],
                &epc[..32],
            );
            eqb(
                &format!("{tag} seal left the body untouched"),
                &c[32..],
                &pattern(24 + SEAL)[32..],
            );
        }

        // 7.69 / 7.78 — reconstruct the seal from its parts: the ephemeral
        // key comes from `randombytes_buf`, the nonce is
        // BLAKE2b-24(epk ‖ pk), and the body is exactly `_easy` under
        // (pk, esk) with that nonce.
        {
            type GH = unsafe extern "C" fn(*mut u8, usize, *const u8, u64, *const u8, usize) -> c_int;
            type BufFn = unsafe extern "C" fn(*mut c_void, usize);
            let (ghc, ghr) = both::<GH>("crypto_generichash");
            let (rbc, rbr) = both::<BufFn>("randombytes_buf");
            let easy = P6::new(&format!("{prefix}_easy"));
            for len in [0usize, 1, 16, 33, 64, 200] {
                let m = rng.bytes(len);
                rng_reset();
                let mut eskc = [0u8; 32];
                let mut eskr = [0u8; 32];
                unsafe {
                    rbc(eskc.as_mut_ptr() as *mut c_void, 32);
                    rbr(eskr.as_mut_ptr() as *mut c_void, 32);
                }
                eqb("RNG streams agree", &eskc, &eskr);
                let mut epc = prefill(32);
                let mut epr = prefill(32);
                unsafe {
                    bc(epc.as_mut_ptr(), eskc.as_ptr());
                    br(epr.as_mut_ptr(), eskr.as_ptr());
                }
                eqb("epk agreement", &epc, &epr);
                let epk = epc[..32].to_vec();
                let mut pre = epk.clone();
                pre.extend_from_slice(&pk_a);
                let mut nc = prefill(NONCE);
                let mut nr = prefill(NONCE);
                unsafe {
                    ghc(nc.as_mut_ptr(), NONCE, pre.as_ptr(), pre.len() as u64, core::ptr::null(), 0);
                    ghr(nr.as_mut_ptr(), NONCE, pre.as_ptr(), pre.len() as u64, core::ptr::null(), 0);
                }
                eqb("derived nonce agreement", &nc, &nr);
                let nonce = nc[..NONCE].to_vec();
                let (rc, want) = easy.run(len + MAC, &m, &nonce, &pk_a, &eskc);
                eqi("reference easy rc", rc, 0);
                let (rc, c) = seal.run(len + SEAL, &m, &pk_a, true);
                eqi(&format!("{tag} seal rc"), rc, 0);
                eqb(&format!("{tag} seal c[0..32] == epk"), &c[..32], &epk);
                eqb(
                    &format!("{tag} seal body == easy under BLAKE2b-24(epk ‖ pk)"),
                    &c[32..],
                    &want,
                );
            }
        }

        // Two seals of the same message differ (fresh ephemeral key each time).
        rng_reset();
        let m = rng.bytes(40);
        let (_, c1) = seal.run(40 + SEAL, &m, &pk_a, false);
        let (_, c2) = seal.run(40 + SEAL, &m, &pk_a, false);
        assert_ne!(hex(&c1), hex(&c2), "two seals must differ");
        let (rc, p1) = open.run(40, &c1, &pk_a, &sk_a);
        eqi("seal_open 1", rc, 0);
        let (rc, p2) = open.run(40, &c2, &pk_a, &sk_a);
        eqi("seal_open 2", rc, 0);
        eqb("both open to the same message", &p1, &m);
        eqb("both open to the same message", &p2, &m);
    }

    // The two seal families must produce different ciphertexts for the same
    // ephemeral key (different nonce derivation *and* different AEAD).
    let xs = P4::new("crypto_box_seal");
    let xc = P4::new("crypto_box_curve25519xchacha20poly1305_seal");
    let m = b"the same message".to_vec();
    let (_, c1) = xs.run(m.len() + SEAL, &m, &pk_a, true);
    let (_, c2) = xc.run(m.len() + SEAL, &m, &pk_a, true);
    eqb("same ephemeral pk", &c1[..32], &c2[..32]);
    assert_ne!(hex(&c1[32..]), hex(&c2[32..]), "xsalsa and xchacha seals must differ");
}

// ============================================================== 7.60–7.63, 7.82, 7.86

#[test]
fn box_small_order_public_keys() {
    let mut rng = Rng::new(0x0_500_0000_0008);
    let (pk_a, sk_a) = keys(&rng.bytes(32).try_into().unwrap());
    let nonce = rng.bytes(NONCE);
    let m = rng.bytes(37);

    for (tag, prefix) in FAMS {
        let easy = P6::new(&format!("{prefix}_easy"));
        let open_easy = P6::new(&format!("{prefix}_open_easy"));
        let det = P7D::new(&format!("{prefix}_detached"));
        let odet = P7O::new(&format!("{prefix}_open_detached"));

        // Build a valid ciphertext to feed the open paths.
        let (_, ct) = easy.run(m.len() + MAC, &m, &nonce, &pk_a, &sk_a);

        for enc in CURVE_SMALL_ORDER {
            for hi in [false, true] {
                let mut pk = h32(enc);
                if hi {
                    pk[31] |= 0x80;
                }
                // 7.60 / 7.82 — `_easy` fails and leaves `c` untouched.
                let (rc, c) = easy.run(m.len() + MAC, &m, &nonce, &pk, &sk_a);
                eqi(&format!("{tag} easy small-order pk"), rc, -1);
                eqb("c untouched", &c, &pattern(m.len() + MAC));
                // 7.61 — `_detached` fails and leaves both outputs untouched.
                let (rc, c, mac) = det.run(&m, &nonce, &pk, &sk_a);
                eqi(&format!("{tag} detached small-order pk"), rc, -1);
                eqb("c untouched", &c, &pattern(m.len()));
                eqb("mac untouched", &mac, &pattern(MAC));
                // 7.63 / 7.86 — the open paths fail *after* the clen guard.
                let (rc, out) = open_easy.run(m.len(), &ct, &nonce, &pk, &sk_a);
                eqi(&format!("{tag} open_easy small-order pk"), rc, -1);
                eqb("m untouched", &out, &pattern(m.len()));
                let (rc, out) = odet.run(&ct[MAC..], &ct[..MAC], &nonce, &pk, &sk_a);
                eqi(&format!("{tag} open_detached small-order pk"), rc, -1);
                eqb("m untouched", &out, &pattern(m.len()));
                // The clen guard still wins over beforenm.
                let (rc, out) = open_easy.run(4, &ct[..MAC - 1], &nonce, &pk, &sk_a);
                eqi(&format!("{tag} open_easy short + small-order pk"), rc, -1);
                eqb("m untouched", &out, &pattern(4));
            }
        }
    }

    // 7.62 / 7.63 — the NaCl-padded xsalsa entry points.
    let bx = P6::new("crypto_box");
    let bxo = P6::new("crypto_box_open");
    let xs = P6::new("crypto_box_curve25519xsalsa20poly1305");
    let xso = P6::new("crypto_box_curve25519xsalsa20poly1305_open");
    let mut padded_m = vec![0u8; 64];
    let tail = rng.bytes(32);
    padded_m[ZERO..].copy_from_slice(&tail);
    for enc in CURVE_SMALL_ORDER {
        let pk = h32(enc);
        for p in [&bx, &xs] {
            let (rc, c) = p.run(64, &padded_m, &nonce, &pk, &sk_a);
            eqi("padded box small-order pk", rc, -1);
            eqb("c untouched", &c, &pattern(64));
        }
        for p in [&bxo, &xso] {
            let (rc, out) = p.run(64, &vec![0u8; 64], &nonce, &pk, &sk_a);
            eqi("padded open small-order pk", rc, -1);
            eqb("m untouched", &out, &pattern(64));
        }
    }
}

// ============================================================== 7.79, 7.89, 7.90 (misuse)

#[test]
fn box_mlen_misuse_aborts() {
    // The six `mlen > MESSAGEBYTES_MAX` guards call `sodium_misuse()`, which
    // runs the misuse handler and then `abort()`s — it is not a `-1` return.
    // The C and the Rust must terminate the process in the same way.
    let bogus: u64 = u64::MAX - 15; // MESSAGEBYTES_MAX + 1
    let (mm_c, mm_r) = both::<SizeFn>("crypto_box_messagebytes_max");
    unsafe {
        assert_eq!(mm_c() as u64, bogus - 1);
        assert_eq!(mm_r() as u64, bogus - 1);
    }

    let key = [7u8; 32];
    let nonce = [9u8; NONCE];
    let pk = [0x11u8; 32];
    let dummy = [0u8; 64];

    for prefix in ["crypto_box", "crypto_box_curve25519xchacha20poly1305"] {
        // `_easy_afternm(c, m, mlen, n, k)`
        let n = format!("{prefix}_easy_afternm");
        let (c, r) = both::<F5>(&n);
        eq_abort(
            &n,
            || unsafe {
                c(dummy.as_ptr() as *mut u8, dummy.as_ptr(), bogus, nonce.as_ptr(), key.as_ptr());
            },
            || unsafe {
                r(dummy.as_ptr() as *mut u8, dummy.as_ptr(), bogus, nonce.as_ptr(), key.as_ptr());
            },
        );

        // `_easy(c, m, mlen, n, pk, sk)`
        let n = format!("{prefix}_easy");
        let (c, r) = both::<F6>(&n);
        eq_abort(
            &n,
            || unsafe {
                c(dummy.as_ptr() as *mut u8, dummy.as_ptr(), bogus, nonce.as_ptr(), pk.as_ptr(), key.as_ptr());
            },
            || unsafe {
                r(dummy.as_ptr() as *mut u8, dummy.as_ptr(), bogus, nonce.as_ptr(), pk.as_ptr(), key.as_ptr());
            },
        );

        // `_seal(c, m, mlen, pk)`
        let n = format!("{prefix}_seal");
        let (c, r) = both::<F4>(&n);
        eq_abort(
            &n,
            || unsafe {
                c(dummy.as_ptr() as *mut u8, dummy.as_ptr(), bogus, pk.as_ptr());
            },
            || unsafe {
                r(dummy.as_ptr() as *mut u8, dummy.as_ptr(), bogus, pk.as_ptr());
            },
        );
    }

    // The guard is `>`, so `mlen == MESSAGEBYTES_MAX` does *not* abort — but it
    // would require a 2^64-byte buffer, so only the boundary arithmetic is
    // checked here (via the accessor above).
}

// ============================================================== C ↔ Rust interop

#[test]
fn box_cross_library_interop() {
    let mut rng = Rng::new(0x0017_0000_000B);
    let (pk_a, sk_a) = keys(&rng.bytes(32).try_into().unwrap());
    let (pk_b, sk_b) = keys(&rng.bytes(32).try_into().unwrap());
    let nonce = rng.bytes(NONCE);

    for (tag, prefix) in FAMS {
        let (ec, er) = both::<F6>(&format!("{prefix}_easy"));
        let (oc, or) = both::<F6>(&format!("{prefix}_open_easy"));
        let (dc, dr) = both::<F7D>(&format!("{prefix}_detached"));
        let (odc, odr) = both::<F7O>(&format!("{prefix}_open_detached"));
        let (sc, sr) = both::<F4>(&format!("{prefix}_seal"));
        let (soc, sor) = both::<F5>(&format!("{prefix}_seal_open"));

        for len in [0usize, 1, 16, 17, 63, 64, 65, 300, 1024] {
            let m = rng.bytes(len);

            // C encrypts, Rust decrypts.
            let mut ct = vec![0u8; len + MAC];
            assert_eq!(
                unsafe {
                    ec(ct.as_mut_ptr(), m.as_ptr(), len as u64, nonce.as_ptr(), pk_b.as_ptr(), sk_a.as_ptr())
                },
                0
            );
            let mut pt = vec![0u8; len];
            assert_eq!(
                unsafe {
                    or(pt.as_mut_ptr(), ct.as_ptr(), (len + MAC) as u64, nonce.as_ptr(), pk_a.as_ptr(), sk_b.as_ptr())
                },
                0,
                "{tag}: Rust must open a C ciphertext"
            );
            eqb("Rust-opened C ciphertext", &pt, &m);

            // Rust encrypts, C decrypts.
            let mut ct2 = vec![0u8; len + MAC];
            assert_eq!(
                unsafe {
                    er(ct2.as_mut_ptr(), m.as_ptr(), len as u64, nonce.as_ptr(), pk_b.as_ptr(), sk_a.as_ptr())
                },
                0
            );
            eqb("deterministic ciphertext", &ct2, &ct);
            let mut pt = vec![0u8; len];
            assert_eq!(
                unsafe {
                    oc(pt.as_mut_ptr(), ct2.as_ptr(), (len + MAC) as u64, nonce.as_ptr(), pk_a.as_ptr(), sk_b.as_ptr())
                },
                0,
                "{tag}: C must open a Rust ciphertext"
            );
            eqb("C-opened Rust ciphertext", &pt, &m);

            // Detached, both directions.
            let mut body = vec![0u8; len];
            let mut mac = vec![0u8; MAC];
            assert_eq!(
                unsafe {
                    dc(body.as_mut_ptr(), mac.as_mut_ptr(), m.as_ptr(), len as u64, nonce.as_ptr(), pk_b.as_ptr(), sk_a.as_ptr())
                },
                0
            );
            let mut pt = vec![0u8; len];
            assert_eq!(
                unsafe {
                    odr(pt.as_mut_ptr(), body.as_ptr(), mac.as_ptr(), len as u64, nonce.as_ptr(), pk_a.as_ptr(), sk_b.as_ptr())
                },
                0,
                "{tag}: Rust must open a C detached ciphertext"
            );
            eqb("Rust-opened C detached", &pt, &m);
            let mut body2 = vec![0u8; len];
            let mut mac2 = vec![0u8; MAC];
            assert_eq!(
                unsafe {
                    dr(body2.as_mut_ptr(), mac2.as_mut_ptr(), m.as_ptr(), len as u64, nonce.as_ptr(), pk_b.as_ptr(), sk_a.as_ptr())
                },
                0
            );
            eqb("detached body", &body2, &body);
            eqb("detached mac", &mac2, &mac);
            let mut pt = vec![0u8; len];
            assert_eq!(
                unsafe {
                    odc(pt.as_mut_ptr(), body2.as_ptr(), mac2.as_ptr(), len as u64, nonce.as_ptr(), pk_a.as_ptr(), sk_b.as_ptr())
                },
                0,
                "{tag}: C must open a Rust detached ciphertext"
            );
            eqb("C-opened Rust detached", &pt, &m);

            // Seal: C seals, Rust opens; Rust seals, C opens.
            rng_reset();
            let mut s1 = vec![0u8; len + SEAL];
            assert_eq!(
                unsafe { sc(s1.as_mut_ptr(), m.as_ptr(), len as u64, pk_a.as_ptr()) },
                0
            );
            let mut s2 = vec![0u8; len + SEAL];
            assert_eq!(
                unsafe { sr(s2.as_mut_ptr(), m.as_ptr(), len as u64, pk_a.as_ptr()) },
                0
            );
            eqb("seal determinism under the shared RNG", &s2, &s1);
            let mut pt = vec![0u8; len];
            assert_eq!(
                unsafe {
                    sor(pt.as_mut_ptr(), s1.as_ptr(), (len + SEAL) as u64, pk_a.as_ptr(), sk_a.as_ptr())
                },
                0,
                "{tag}: Rust must open a C seal"
            );
            eqb("Rust-opened C seal", &pt, &m);
            let mut pt = vec![0u8; len];
            assert_eq!(
                unsafe {
                    soc(pt.as_mut_ptr(), s2.as_ptr(), (len + SEAL) as u64, pk_a.as_ptr(), sk_a.as_ptr())
                },
                0,
                "{tag}: C must open a Rust seal"
            );
            eqb("C-opened Rust seal", &pt, &m);
        }
    }

    // The NaCl-padded path, both directions.
    let (bc2, br2) = both::<F6>("crypto_box");
    let (oc2, or2) = both::<F6>("crypto_box_open");
    for mlen in [ZERO, 33, 64, 1056] {
        let mut m = vec![0u8; mlen];
        let tail = rng.bytes(mlen - ZERO);
        m[ZERO..].copy_from_slice(&tail);
        let mut c1 = vec![0u8; mlen];
        let mut c2 = vec![0u8; mlen];
        assert_eq!(
            unsafe { bc2(c1.as_mut_ptr(), m.as_ptr(), mlen as u64, nonce.as_ptr(), pk_b.as_ptr(), sk_a.as_ptr()) },
            0
        );
        assert_eq!(
            unsafe { br2(c2.as_mut_ptr(), m.as_ptr(), mlen as u64, nonce.as_ptr(), pk_b.as_ptr(), sk_a.as_ptr()) },
            0
        );
        eqb("padded ciphertext", &c2, &c1);
        let mut p = vec![0u8; mlen];
        assert_eq!(
            unsafe { or2(p.as_mut_ptr(), c1.as_ptr(), mlen as u64, nonce.as_ptr(), pk_a.as_ptr(), sk_b.as_ptr()) },
            0,
            "Rust must open a C padded box"
        );
        eqb("Rust-opened C padded box", &p[ZERO..], &m[ZERO..]);
        let mut p = vec![0u8; mlen];
        assert_eq!(
            unsafe { oc2(p.as_mut_ptr(), c2.as_ptr(), mlen as u64, nonce.as_ptr(), pk_a.as_ptr(), sk_b.as_ptr()) },
            0,
            "C must open a Rust padded box"
        );
        eqb("C-opened Rust padded box", &p[ZERO..], &m[ZERO..]);
    }
}
