//! Area 7 — `crypto_sign`: ed25519 (attached / detached) and ed25519ph
//! (multipart prehashed), the key conversions and every accessor.
//!
//! Covers `configs_7.md` rows 7.29–7.59 and `errors_7.md` rows 7.34–7.58.
//!
//! Every call goes through `dlsym` on both the reference C `libsodium.so` and
//! the translated Rust `liblibsodium.so`.  Output buffers are always
//! pre-filled with a distinctive pattern and compared byte for byte *after*
//! failures as well as successes, because the C decides case by case whether a
//! rejected call leaves the caller's buffer untouched or fully written.
mod common;
use common::*;
use libloading::Symbol;
use std::ffi::{c_char, c_int, CStr};

// --------------------------------------------------------------- signatures

/// `crypto_sign_ed25519_keypair(pk, sk)`
type KeypairFn = unsafe extern "C" fn(*mut u8, *mut u8) -> c_int;
/// `crypto_sign_ed25519_seed_keypair(pk, sk, seed)`
type SeedKeypairFn = unsafe extern "C" fn(*mut u8, *mut u8, *const u8) -> c_int;
/// `crypto_sign_ed25519_detached(sig, siglen_p, m, mlen, sk)` — and, with the
/// same shape, `crypto_sign_ed25519(sm, smlen_p, m, mlen, sk)` and
/// `crypto_sign_ed25519_open(m, mlen_p, sm, smlen, pk)`.
type Sign5Fn = unsafe extern "C" fn(*mut u8, *mut u64, *const u8, u64, *const u8) -> c_int;
/// `crypto_sign_ed25519_verify_detached(sig, m, mlen, pk)`
type VerifyFn = unsafe extern "C" fn(*const u8, *const u8, u64, *const u8) -> c_int;
/// `crypto_sign_ed25519_sk_to_seed(seed, sk)` etc.
type ConvFn = unsafe extern "C" fn(*mut u8, *const u8) -> c_int;
type InitFn = unsafe extern "C" fn(*mut u8) -> c_int;
type UpdateFn = unsafe extern "C" fn(*mut u8, *const u8, u64) -> c_int;
type FinalCreateFn = unsafe extern "C" fn(*mut u8, *mut u8, *mut u64, *const u8) -> c_int;
type FinalVerifyFn = unsafe extern "C" fn(*mut u8, *const u8, *const u8) -> c_int;
type SizeFn = unsafe extern "C" fn() -> usize;
type StrFn = unsafe extern "C" fn() -> *const c_char;
/// `_crypto_sign_ed25519_detached(sig, siglen_p, m, mlen, sk, prehashed)`
type Sign6Fn = unsafe extern "C" fn(*mut u8, *mut u64, *const u8, u64, *const u8, c_int) -> c_int;
/// `_crypto_sign_ed25519_verify_detached(sig, m, mlen, pk, prehashed)`
type Verify5Fn = unsafe extern "C" fn(*const u8, *const u8, u64, *const u8, c_int) -> c_int;
/// `_crypto_sign_ed25519_ref10_hinit(hs, prehashed)`
type HinitFn = unsafe extern "C" fn(*mut u8, c_int);
type MultBase = unsafe extern "C" fn(*mut u8, *const u8) -> c_int;
type Add3 = unsafe extern "C" fn(*mut u8, *const u8, *const u8) -> c_int;
type IsValid = unsafe extern "C" fn(*const u8) -> c_int;
type HashFn = unsafe extern "C" fn(*mut u8, *const u8, u64) -> c_int;
/// `crypto_hash_sha512_final(state, out)`
type Sha512Final = unsafe extern "C" fn(*mut u8, *mut u8) -> c_int;

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
fn h64(s: &str) -> [u8; 64] {
    let v = hx(s);
    let mut a = [0u8; 64];
    a.copy_from_slice(&v);
    a
}

/// Distinctive prefill so that "buffer untouched" and "buffer fully written"
/// are both observable, plus `PAD` guard bytes past the end.
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

const NOLEN: *mut u64 = core::ptr::null_mut();
const SENTINEL: u64 = 0xdead_beef_feed_face;

/// A (C, Rust) pair of 5-argument sign/open functions.
struct P5 {
    name: String,
    c: Symbol<'static, Sign5Fn>,
    r: Symbol<'static, Sign5Fn>,
}

impl P5 {
    fn new(name: &str) -> Self {
        let (c, r) = both::<Sign5Fn>(name);
        P5 { name: name.to_string(), c, r }
    }

    /// `f(out, len_p, in, inlen, key)` with a pre-filled `outlen`-byte output
    /// buffer.  Returns `(rc, out[..outlen], *len_p)`.
    fn run(
        &self,
        outlen: usize,
        inp: &[u8],
        key: &[u8],
        with_len: bool,
    ) -> (c_int, Vec<u8>, Option<u64>) {
        let mut oc = prefill(outlen);
        let mut or = prefill(outlen);
        let mut lc = SENTINEL;
        let mut lr = SENTINEL;
        let pc = if with_len { &mut lc as *mut u64 } else { NOLEN };
        let pr = if with_len { &mut lr as *mut u64 } else { NOLEN };
        let tag = format!("{}(inlen {})", self.name, inp.len());
        let rc = unsafe { (self.c)(oc.as_mut_ptr(), pc, inp.as_ptr(), inp.len() as u64, key.as_ptr()) };
        let rr = unsafe { (self.r)(or.as_mut_ptr(), pr, inp.as_ptr(), inp.len() as u64, key.as_ptr()) };
        eqi(&format!("{tag} rc"), rc, rr);
        eqb(&format!("{tag} out"), &oc, &or);
        check_pad(&format!("{tag} C"), &oc, outlen);
        check_pad(&format!("{tag} Rust"), &or, outlen);
        if with_len {
            assert_eq!(lc, lr, "{tag}: *len_p mismatch (C {lc}, Rust {lr})");
        }
        (rc, oc[..outlen].to_vec(), if with_len { Some(lc) } else { None })
    }

    /// Same, but the output pointer is NULL (only legal for
    /// `crypto_sign_ed25519_open`, which explicitly NULL-checks `m`).
    fn run_out_null(&self, inp: &[u8], key: &[u8], with_len: bool) -> (c_int, Option<u64>) {
        let mut lc = SENTINEL;
        let mut lr = SENTINEL;
        let pc = if with_len { &mut lc as *mut u64 } else { NOLEN };
        let pr = if with_len { &mut lr as *mut u64 } else { NOLEN };
        let rc = unsafe {
            (self.c)(core::ptr::null_mut(), pc, inp.as_ptr(), inp.len() as u64, key.as_ptr())
        };
        let rr = unsafe {
            (self.r)(core::ptr::null_mut(), pr, inp.as_ptr(), inp.len() as u64, key.as_ptr())
        };
        eqi(&format!("{} out=NULL rc", self.name), rc, rr);
        if with_len {
            assert_eq!(lc, lr, "{} out=NULL *len_p mismatch", self.name);
        }
        (rc, if with_len { Some(lc) } else { None })
    }
}

/// A (C, Rust) pair of `verify_detached`-shaped functions.
struct PV {
    name: String,
    c: Symbol<'static, VerifyFn>,
    r: Symbol<'static, VerifyFn>,
}
impl PV {
    fn new(name: &str) -> Self {
        let (c, r) = both::<VerifyFn>(name);
        PV { name: name.to_string(), c, r }
    }
    #[track_caller]
    fn run(&self, sig: &[u8], m: &[u8], pk: &[u8]) -> c_int {
        assert_eq!(sig.len(), 64);
        assert_eq!(pk.len(), 32);
        let rc = unsafe { (self.c)(sig.as_ptr(), m.as_ptr(), m.len() as u64, pk.as_ptr()) };
        let rr = unsafe { (self.r)(sig.as_ptr(), m.as_ptr(), m.len() as u64, pk.as_ptr()) };
        eqi(
            &format!("{}(sig {}, mlen {}, pk {})", self.name, hex(&sig[..8]), m.len(), hex(&pk[..8])),
            rc,
            rr,
        );
        rc
    }
}

/// A (C, Rust) pair of 2-argument conversion functions.
struct PC {
    name: String,
    c: Symbol<'static, ConvFn>,
    r: Symbol<'static, ConvFn>,
}
impl PC {
    fn new(name: &str) -> Self {
        let (c, r) = both::<ConvFn>(name);
        PC { name: name.to_string(), c, r }
    }
    fn run(&self, outlen: usize, inp: &[u8]) -> (c_int, Vec<u8>) {
        let mut oc = prefill(outlen);
        let mut or = prefill(outlen);
        let rc = unsafe { (self.c)(oc.as_mut_ptr(), inp.as_ptr()) };
        let rr = unsafe { (self.r)(or.as_mut_ptr(), inp.as_ptr()) };
        eqi(&format!("{}({}) rc", self.name, hex(inp)), rc, rr);
        eqb(&format!("{}({}) out", self.name, hex(inp)), &oc, &or);
        check_pad(&self.name, &oc, outlen);
        check_pad(&self.name, &or, outlen);
        (rc, oc[..outlen].to_vec())
    }
}

/// A heap allocation of `n` bytes with 8-byte (`u64`) alignment, for the
/// opaque `crypto_sign_state`.
struct St(Vec<u64>, usize);
impl St {
    fn new(n: usize) -> Self {
        St(vec![0u64; (n + 7) / 8], n)
    }
    fn p(&mut self) -> *mut u8 {
        self.0.as_mut_ptr() as *mut u8
    }
    fn bytes(&self) -> &[u8] {
        unsafe { core::slice::from_raw_parts(self.0.as_ptr() as *const u8, self.1) }
    }
}

fn statebytes() -> usize {
    let (c, r) = both::<SizeFn>("crypto_sign_statebytes");
    let (a, b) = unsafe { (c(), r()) };
    assert_eq!(a, b, "crypto_sign_statebytes mismatch");
    a
}

// ------------------------------------------------------------------ fixtures

/// The 8 small-order ed25519 point encodings.
const ED_SMALL_ORDER: [&str; 8] = [
    "0100000000000000000000000000000000000000000000000000000000000000",
    "ecffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff7f",
    "0000000000000000000000000000000000000000000000000000000000000000",
    "0000000000000000000000000000000000000000000000000000000000000080",
    "26e8958fc2b227b045c3f489f2ef98f0d5dfac05d3c63339b13802886d53fc05",
    "c7176a703d4dd84fba3c0b760d10670f2a2053fa2c39ccc64ec7fd7792ac037a",
    "26e8958fc2b227b045c3f489f2ef98f0d5dfac05d3c63339b13802886d53fc85",
    "c7176a703d4dd84fba3c0b760d10670f2a2053fa2c39ccc64ec7fd7792ac03fa",
];

/// `L = 2^252 + 27742317777372353535851937790883648493`, little-endian.
const L_HEX: &str = "edd3f55c1a631258d69cf7a2def9de1400000000000000000000000000000010";

/// RFC 8032 §7.1 test 1.
const RFC1_SEED: &str = "9d61b19deffd5a60ba844af492ec2cc44449c5697b326919703bac031cae7f60";
const RFC1_PK: &str = "d75a980182b10ab7d54bfed3c964073a0ee172f3daa62325af021a68f707511a";
const RFC1_SIG: &str = "e5564300c360ac729086e2cc806e828a84877f1eb8e5d974d873e065224901555fb8821590a33bacc61e39701cf9b46bd25bf5f0595bbe24655141438e7a100b";
/// RFC 8032 §7.1 test 2 (1-byte message `72`).
const RFC2_SEED: &str = "4ccd089b28ff96da9db6c346ec114e0f5b8a319f35aba624da8cf6ed4fb8a6fb";
const RFC2_PK: &str = "3d4017c3e843895a92b70aa74d1b7ebc9c982ccf2ec4968cc0cd55f12af4660c";
const RFC2_SIG: &str = "92a009a9f0d4cab8720e820b5f642540a2b27b5416503f8fb3762223ebdb69da085ac1e43e15996e458f3613d0f11d8c387b2eaeb4302aeeb00d291612bb0c00";
/// RFC 8032 §7.1 test 3 (2-byte message `af82`).
const RFC3_SEED: &str = "c5aa8df43f9f837bedb7442f31dcb7b166d38535076f094b85ce3a2e0b4458f7";
const RFC3_PK: &str = "fc51cd8e6218a1a38da47ed00230f0580816ed13ba3303ac5deb911548908025";
const RFC3_SIG: &str = "6291d657deec24024827e69c3abe01a30ce548a284743a445e3680d7db5ac3ac18ff9b538d16f290ae67f760984dc6594a7c15e9716ed28dc027beceea1ec40a";

/// Deterministic `(pk, sk)` from a seed, using the C implementation as the
/// source of truth (its agreement with Rust is checked separately).
fn keys_from_seed(seed: &[u8; 32]) -> ([u8; 32], [u8; 64]) {
    let (c, _) = both::<SeedKeypairFn>("crypto_sign_ed25519_seed_keypair");
    let mut pk = [0u8; 32];
    let mut sk = [0u8; 64];
    let rc = unsafe { c(pk.as_mut_ptr(), sk.as_mut_ptr(), seed.as_ptr()) };
    assert_eq!(rc, 0);
    (pk, sk)
}

/// A detached signature produced by the C implementation.
fn c_sign(m: &[u8], sk: &[u8; 64]) -> [u8; 64] {
    let (c, _) = both::<Sign5Fn>("crypto_sign_ed25519_detached");
    let mut sig = [0u8; 64];
    let mut sl = 0u64;
    let rc = unsafe { c(sig.as_mut_ptr(), &mut sl, m.as_ptr(), m.len() as u64, sk.as_ptr()) };
    assert_eq!(rc, 0);
    assert_eq!(sl, 64);
    sig
}

/// A varied set of message lengths: every value in `0..=300` plus multi-KiB.
fn dense_lengths() -> Vec<usize> {
    (0..=300).collect()
}
fn big_lengths() -> Vec<usize> {
    vec![1023, 1024, 1025, 2048, 4096, 8191, 8192, 8193, 16384, 65536]
}

// =========================================================== 7.55, 7.59

#[test]
fn sign_accessors() {
    let sizes: &[(&str, usize)] = &[
        ("crypto_sign_bytes", 64),
        ("crypto_sign_seedbytes", 32),
        ("crypto_sign_publickeybytes", 32),
        ("crypto_sign_secretkeybytes", 64),
        ("crypto_sign_messagebytes_max", usize::MAX - 64),
        ("crypto_sign_ed25519_bytes", 64),
        ("crypto_sign_ed25519_seedbytes", 32),
        ("crypto_sign_ed25519_publickeybytes", 32),
        ("crypto_sign_ed25519_secretkeybytes", 64),
        ("crypto_sign_ed25519_messagebytes_max", usize::MAX - 64),
    ];
    for (name, want) in sizes {
        let (c, r) = both::<SizeFn>(name);
        let (a, b) = unsafe { (c(), r()) };
        assert_eq!(a, b, "{name}: C {a} vs Rust {b}");
        assert_eq!(a, *want, "{name}: expected {want}, C returned {a}");
    }

    // 7.55 — both statebytes accessors report the same size, and it is the
    // size of a SHA-512 state.
    for name in ["crypto_sign_statebytes", "crypto_sign_ed25519ph_statebytes"] {
        let (c, r) = both::<SizeFn>(name);
        let (a, b) = unsafe { (c(), r()) };
        assert_eq!(a, b, "{name}: C {a} vs Rust {b}");
        assert!(a >= 208, "{name}: implausibly small state ({a})");
    }
    let (c, r) = both::<SizeFn>("crypto_sign_statebytes");
    let (c2, r2) = both::<SizeFn>("crypto_sign_ed25519ph_statebytes");
    unsafe {
        assert_eq!(c(), c2(), "C: crypto_sign_statebytes != ed25519ph_statebytes");
        assert_eq!(r(), r2(), "Rust: crypto_sign_statebytes != ed25519ph_statebytes");
    }
    if has("crypto_hash_sha512_statebytes") {
        let (h, _) = both::<SizeFn>("crypto_hash_sha512_statebytes");
        unsafe { assert_eq!(c(), h(), "statebytes != sizeof(sha512 state)") };
    }

    // crypto_sign_primitive() == "ed25519"; there is no
    // crypto_sign_ed25519_primitive().
    let (c, r) = both::<StrFn>("crypto_sign_primitive");
    let (a, b) = unsafe { (CStr::from_ptr(c()), CStr::from_ptr(r())) };
    assert_eq!(a, b, "crypto_sign_primitive mismatch");
    assert_eq!(a.to_str().unwrap(), "ed25519");
    assert!(
        !has("crypto_sign_ed25519_primitive"),
        "crypto_sign_ed25519_primitive must not exist"
    );
}

// =========================================================== 7.29–7.32

#[test]
fn sign_seed_keypair() {
    let gen = {
        let (c, r) = both::<SeedKeypairFn>("crypto_sign_seed_keypair");
        (c, r)
    };
    let ed = {
        let (c, r) = both::<SeedKeypairFn>("crypto_sign_ed25519_seed_keypair");
        (c, r)
    };

    let mut seeds: Vec<[u8; 32]> = vec![
        h32(RFC1_SEED),
        h32(RFC2_SEED),
        h32(RFC3_SEED),
        [0u8; 32],
        [0xffu8; 32],
        h32(L_HEX),
    ];
    let mut rng = Rng::new(0x0519_EED0_1001);
    for _ in 0..24 {
        seeds.push(rng.bytes(32).try_into().unwrap());
    }

    for seed in &seeds {
        // Both namespaces, both libraries, pre-filled outputs.
        let mut out: Vec<Vec<u8>> = Vec::new();
        for (which, f) in [("ed C", ed.0.clone()), ("ed R", ed.1.clone())]
            .into_iter()
            .chain([("gen C", gen.0.clone()), ("gen R", gen.1.clone())])
        {
            let mut pk = prefill(32);
            let mut sk = prefill(64);
            let rc = unsafe { f(pk.as_mut_ptr(), sk.as_mut_ptr(), seed.as_ptr()) };
            assert_eq!(rc, 0, "{which}: seed_keypair must always succeed");
            check_pad(which, &pk, 32);
            check_pad(which, &sk, 64);
            let mut v = pk[..32].to_vec();
            v.extend_from_slice(&sk[..64]);
            out.push(v);
        }
        eqb("seed_keypair ed25519 C vs Rust", &out[0], &out[1]);
        eqb("seed_keypair generic C vs Rust", &out[2], &out[3]);
        eqb("seed_keypair generic vs ed25519 alias", &out[0], &out[2]);

        // sk = seed ‖ pk (keypair.c:26-27).
        let pk = &out[0][..32];
        let sk = &out[0][32..];
        eqb("sk[0..32] must be the seed", &sk[..32], seed);
        eqb("sk[32..64] must be pk", &sk[32..], pk);
    }

    // 7.29 — RFC 8032 test 1.
    let (pk, sk) = keys_from_seed(&h32(RFC1_SEED));
    eqb("RFC8032-1 pk", &pk, &h32(RFC1_PK));
    eqb("RFC8032-1 sk", &sk, &[h32(RFC1_SEED).as_slice(), h32(RFC1_PK).as_slice()].concat());
    let (pk, _) = keys_from_seed(&h32(RFC2_SEED));
    eqb("RFC8032-2 pk", &pk, &h32(RFC2_PK));
    let (pk, _) = keys_from_seed(&h32(RFC3_SEED));
    eqb("RFC8032-3 pk", &pk, &h32(RFC3_PK));

    // Overlapping / aliased outputs are not legal here (pk and sk are distinct
    // objects in every caller), but sk being written before it is read back is:
    // seed_keypair hashes into sk and then overwrites sk[0..32] from `seed`.
    // Feeding `sk` itself as the seed exercises that ordering.
    for f in [("C", ed.0.clone()), ("R", ed.1.clone())] {
        let mut pk = [0u8; 32];
        let mut sk = [7u8; 64];
        let seed_copy: [u8; 32] = sk[..32].try_into().unwrap();
        unsafe { f.1(pk.as_mut_ptr(), sk.as_mut_ptr(), seed_copy.as_ptr()) };
        assert_eq!(&sk[..32], &seed_copy[..], "{}: sk[0..32] must be the seed", f.0);
    }
}

// =========================================================== 7.31

#[test]
fn sign_keypair_random() {
    let (kc, kr) = both::<KeypairFn>("crypto_sign_ed25519_keypair");
    let (gc, gr) = both::<KeypairFn>("crypto_sign_keypair");

    for round in 0..16u64 {
        for (name, c, r) in [
            ("crypto_sign_ed25519_keypair", &kc, &kr),
            ("crypto_sign_keypair", &gc, &gr),
        ] {
            rng_reseed(0x1000 + round);
            let mut pkc = prefill(32);
            let mut skc = prefill(64);
            let rc = unsafe { c(pkc.as_mut_ptr(), skc.as_mut_ptr()) };
            let mut pkr = prefill(32);
            let mut skr = prefill(64);
            let rr = unsafe { r(pkr.as_mut_ptr(), skr.as_mut_ptr()) };
            eqi(&format!("{name} rc"), rc, rr);
            eqb(&format!("{name} pk"), &pkc, &pkr);
            eqb(&format!("{name} sk"), &skc, &skr);
            check_pad(name, &pkc, 32);
            check_pad(name, &skc, 64);
            assert_eq!(rc, 0);

            // sk[32..64] == pk, and seed_keypair(sk[0..32]) reproduces both.
            eqb("keypair sk[32..64] == pk", &skc[32..64], &pkc[..32]);
            let seed: [u8; 32] = skc[..32].try_into().unwrap();
            let (pk2, sk2) = keys_from_seed(&seed);
            eqb("seed_keypair(sk[0..32]) pk", &pk2, &pkc[..32]);
            eqb("seed_keypair(sk[0..32]) sk", &sk2, &skc[..64]);
        }
    }

    // Two successive draws from the same stream differ.
    rng_reset();
    let mut pk1 = [0u8; 32];
    let mut sk1 = [0u8; 64];
    let mut pk2 = [0u8; 32];
    let mut sk2 = [0u8; 64];
    unsafe {
        kc(pk1.as_mut_ptr(), sk1.as_mut_ptr());
        kc(pk2.as_mut_ptr(), sk2.as_mut_ptr());
    }
    assert_ne!(pk1, pk2, "successive keypairs must differ");
}

// =========================================================== 7.32, 7.57

#[test]
fn sign_sk_to_seed_and_pk() {
    let seed_of = PC::new("crypto_sign_ed25519_sk_to_seed");
    let pk_of = PC::new("crypto_sign_ed25519_sk_to_pk");

    let mut rng = Rng::new(0x5C25_EED0_0002);
    let mut sks: Vec<[u8; 64]> = Vec::new();
    for _ in 0..12 {
        let seed: [u8; 32] = rng.bytes(32).try_into().unwrap();
        let (_, sk) = keys_from_seed(&seed);
        sks.push(sk);
    }
    // 7.57 — no validation at all: structurally bogus 64-byte "secret keys"
    // are accepted and simply copied.
    sks.push([0u8; 64]);
    sks.push([0xffu8; 64]);
    sks.push(rng.bytes(64).try_into().unwrap());

    for sk in &sks {
        let (rc, s) = seed_of.run(32, sk);
        eqi("sk_to_seed rc", rc, 0);
        eqb("sk_to_seed == sk[0..32]", &s, &sk[..32]);
        let (rc, p) = pk_of.run(32, sk);
        eqi("sk_to_pk rc", rc, 0);
        eqb("sk_to_pk == sk[32..64]", &p, &sk[32..]);
    }

    // Both use memmove, so fully overlapping buffers are legal.
    let (sc, sr) = both::<ConvFn>("crypto_sign_ed25519_sk_to_seed");
    let (pc, pr) = both::<ConvFn>("crypto_sign_ed25519_sk_to_pk");
    for sk in &sks {
        // seed = sk (destination == source).
        let mut bc = sk.to_vec();
        let mut br = sk.to_vec();
        unsafe {
            sc(bc.as_mut_ptr(), bc.as_ptr());
            sr(br.as_mut_ptr(), br.as_ptr());
        }
        eqb("sk_to_seed overlapping (dst == src)", &bc, &br);
        assert_eq!(&bc[..32], &sk[..32]);

        // pk = sk (destination overlaps the source range that is read).
        let mut bc = sk.to_vec();
        let mut br = sk.to_vec();
        unsafe {
            pc(bc.as_mut_ptr(), bc.as_ptr());
            pr(br.as_mut_ptr(), br.as_ptr());
        }
        eqb("sk_to_pk overlapping (dst == src)", &bc, &br);
        assert_eq!(&bc[..32], &sk[32..]);

        // pk = sk + 32 (destination == the source of the copy: a no-op).
        let mut bc = sk.to_vec();
        let mut br = sk.to_vec();
        unsafe {
            pc(bc.as_mut_ptr().add(32), bc.as_ptr());
            pr(br.as_mut_ptr().add(32), br.as_ptr());
        }
        eqb("sk_to_pk in-place (dst == sk+32)", &bc, &br);
        assert_eq!(&bc[..], &sk[..]);
    }
}

// =========================================================== 7.33–7.36, 7.45

#[test]
fn sign_detached_vectors() {
    let det = P5::new("crypto_sign_ed25519_detached");
    let gen = P5::new("crypto_sign_detached");
    let ver = PV::new("crypto_sign_ed25519_verify_detached");
    let gver = PV::new("crypto_sign_verify_detached");

    let cases: [(&str, &str, Vec<u8>, &str); 3] = [
        (RFC1_SEED, RFC1_PK, vec![], RFC1_SIG),
        (RFC2_SEED, RFC2_PK, vec![0x72], RFC2_SIG),
        (RFC3_SEED, RFC3_PK, hx("af82"), RFC3_SIG),
    ];
    for (seed, want_pk, m, want_sig) in cases {
        let (pk, sk) = keys_from_seed(&h32(seed));
        eqb("pk", &pk, &h32(want_pk));
        // 7.33 — siglen_p non-NULL receives 64.
        let (rc, sig, sl) = det.run(64, &m, &sk, true);
        eqi("detached rc", rc, 0);
        assert_eq!(sl, Some(64), "*siglen_p must be 64");
        eqb("RFC 8032 signature", &sig, &h64(want_sig));
        // Generic alias must be byte-identical.
        let (rc2, sig2, sl2) = gen.run(64, &m, &sk, true);
        eqi("crypto_sign_detached rc", rc2, 0);
        assert_eq!(sl2, Some(64));
        eqb("crypto_sign_detached == ed25519 form", &sig2, &sig);
        // 7.35 — siglen_p == NULL still succeeds.
        let (rc3, sig3, _) = det.run(64, &m, &sk, false);
        eqi("detached siglen_p=NULL rc", rc3, 0);
        eqb("detached siglen_p=NULL sig", &sig3, &sig);
        // 7.45 — both verify namespaces accept.
        eqi("verify rc", ver.run(&sig, &m, &pk), 0);
        eqi("crypto_sign_verify_detached rc", gver.run(&sig, &m, &pk), 0);
    }
}

#[test]
fn sign_detached_lengths_dense() {
    let det = P5::new("crypto_sign_ed25519_detached");
    let ver = PV::new("crypto_sign_ed25519_verify_detached");
    let gen = P5::new("crypto_sign_detached");
    let gver = PV::new("crypto_sign_verify_detached");
    let mut rng = Rng::new(0x0DE7_AC4E_D003);
    let (pk, sk) = keys_from_seed(&rng.bytes(32).try_into().unwrap());

    for len in dense_lengths() {
        let m = rng.bytes(len);
        let (rc, sig, sl) = det.run(64, &m, &sk, true);
        eqi("detached rc", rc, 0);
        assert_eq!(sl, Some(64));
        eqi("verify rc", ver.run(&sig, &m, &pk), 0);

        // The generic aliases are pure forwarders.
        let (_, sig2, _) = gen.run(64, &m, &sk, true);
        eqb("generic detached == ed25519", &sig2, &sig);
        eqi("generic verify rc", gver.run(&sig, &m, &pk), 0);

        // 7.48 — verifying with a different mlen must fail.
        if len > 0 {
            eqi("verify with mlen-1", ver.run(&sig, &m[..len - 1], &pk), -1);
        }
        let mut longer = m.clone();
        longer.push(0);
        eqi("verify with mlen+1", ver.run(&sig, &longer, &pk), -1);
    }
}

#[test]
fn sign_detached_lengths_big() {
    let det = P5::new("crypto_sign_ed25519_detached");
    let ver = PV::new("crypto_sign_ed25519_verify_detached");
    let mut rng = Rng::new(0x0B16_1E00_0004);
    let (pk, sk) = keys_from_seed(&rng.bytes(32).try_into().unwrap());
    for len in big_lengths() {
        let m = rng.bytes(len);
        let (rc, sig, sl) = det.run(64, &m, &sk, true);
        eqi("detached rc", rc, 0);
        assert_eq!(sl, Some(64));
        eqi("verify rc", ver.run(&sig, &m, &pk), 0);
        // One flipped bit in the middle of a multi-KiB message.
        let mut bad = m.clone();
        bad[len / 2] ^= 0x40;
        eqi("verify tampered body", ver.run(&sig, &bad, &pk), -1);
    }
}

// =========================================================== 7.37–7.44

#[test]
fn sign_attached_roundtrip() {
    let att = P5::new("crypto_sign_ed25519");
    let gatt = P5::new("crypto_sign");
    let opn = P5::new("crypto_sign_ed25519_open");
    let gopn = P5::new("crypto_sign_open");
    let det = P5::new("crypto_sign_ed25519_detached");

    let mut rng = Rng::new(0x0A77_AC4E_D005);
    let (pk, sk) = keys_from_seed(&rng.bytes(32).try_into().unwrap());

    let mut lens = dense_lengths();
    lens.extend_from_slice(&[1024, 4096, 8192]);
    for len in lens {
        let m = rng.bytes(len);

        // 7.37 — sm = sig ‖ m, *smlen_p = mlen + 64.
        let (rc, sm, sl) = att.run(len + 64, &m, &sk, true);
        eqi("crypto_sign_ed25519 rc", rc, 0);
        assert_eq!(sl, Some(len as u64 + 64), "*smlen_p must be mlen + 64");
        eqb("sm[64..] == m", &sm[64..], &m);
        let (_, sig, _) = det.run(64, &m, &sk, true);
        eqb("sm[0..64] == detached signature", &sm[..64], &sig);

        // Generic alias.
        let (rc2, sm2, sl2) = gatt.run(len + 64, &m, &sk, true);
        eqi("crypto_sign rc", rc2, 0);
        assert_eq!(sl2, sl);
        eqb("crypto_sign == crypto_sign_ed25519", &sm2, &sm);

        // 7.38 — smlen_p == NULL.
        let (rc3, sm3, _) = att.run(len + 64, &m, &sk, false);
        eqi("crypto_sign smlen_p=NULL rc", rc3, 0);
        eqb("crypto_sign smlen_p=NULL sm", &sm3, &sm);

        // 7.40 — open round trip.
        let (rc, mm, ml) = opn.run(len, &sm, &pk, true);
        eqi("open rc", rc, 0);
        assert_eq!(ml, Some(len as u64), "*mlen_p must be smlen - 64");
        eqb("recovered message", &mm, &m);
        let (rc, mm2, ml2) = gopn.run(len, &sm, &pk, true);
        eqi("crypto_sign_open rc", rc, 0);
        assert_eq!(ml2, ml);
        eqb("crypto_sign_open message", &mm2, &mm);

        // 7.42 — mlen_p == NULL, m non-NULL.
        let (rc, mm3, _) = opn.run(len, &sm, &pk, false);
        eqi("open mlen_p=NULL rc", rc, 0);
        eqb("open mlen_p=NULL message", &mm3, &m);

        // 7.41 — m == NULL (verify-only), mlen_p non-NULL.
        let (rc, ml) = opn.run_out_null(&sm, &pk, true);
        eqi("open m=NULL rc", rc, 0);
        assert_eq!(ml, Some(len as u64));
        // ... and with both NULL.
        let (rc, _) = opn.run_out_null(&sm, &pk, false);
        eqi("open m=NULL mlen_p=NULL rc", rc, 0);
    }
}

#[test]
fn sign_attached_inplace() {
    let (ac, ar) = both::<Sign5Fn>("crypto_sign_ed25519");
    let (oc, or) = both::<Sign5Fn>("crypto_sign_ed25519_open");
    let mut rng = Rng::new(0x0011_91AC_E006);
    let (pk, sk) = keys_from_seed(&rng.bytes(32).try_into().unwrap());

    for len in [0usize, 1, 31, 32, 33, 63, 64, 65, 127, 128, 129, 255, 300, 1024] {
        let m = rng.bytes(len);

        // 7.38 — m == sm + 64 (the memmove in sign.c:111 is a no-op).
        let mut bc = prefill(len + 64);
        bc[64..64 + len].copy_from_slice(&m);
        let mut br = bc.clone();
        let mut lc = SENTINEL;
        let mut lr = SENTINEL;
        let rc = unsafe {
            ac(bc.as_mut_ptr(), &mut lc, bc.as_ptr().add(64), len as u64, sk.as_ptr())
        };
        let rr = unsafe {
            ar(br.as_mut_ptr(), &mut lr, br.as_ptr().add(64), len as u64, sk.as_ptr())
        };
        eqi("in-place sign rc", rc, rr);
        eqb("in-place sign sm", &bc, &br);
        assert_eq!(lc, lr);
        check_pad("in-place sign", &bc, len + 64);
        assert_eq!(rc, 0);
        assert_eq!(lc, len as u64 + 64);
        eqb("in-place sign preserved m", &bc[64..64 + len], &m);

        // 7.43 — m == sm (open shifts down by 64).
        let sm = bc[..len + 64].to_vec();
        let mut bc = prefill(len + 64);
        bc[..len + 64].copy_from_slice(&sm);
        let mut br = bc.clone();
        let mut lc = SENTINEL;
        let mut lr = SENTINEL;
        let rc =
            unsafe { oc(bc.as_mut_ptr(), &mut lc, bc.as_ptr(), (len + 64) as u64, pk.as_ptr()) };
        let rr =
            unsafe { or(br.as_mut_ptr(), &mut lr, br.as_ptr(), (len + 64) as u64, pk.as_ptr()) };
        eqi("in-place open rc", rc, rr);
        eqb("in-place open buffer", &bc, &br);
        assert_eq!(lc, lr);
        check_pad("in-place open", &bc, len + 64);
        assert_eq!(rc, 0);
        assert_eq!(lc, len as u64);
        eqb("in-place open message", &bc[..len], &m);
    }
}

#[test]
fn sign_open_short_and_corrupt() {
    let opn = P5::new("crypto_sign_ed25519_open");
    let gopn = P5::new("crypto_sign_open");
    let att = P5::new("crypto_sign_ed25519");
    let mut rng = Rng::new(0x009E_4BAD_0007);
    let (pk, sk) = keys_from_seed(&rng.bytes(32).try_into().unwrap());
    let (_, wrong_pk) = {
        let (p, s) = keys_from_seed(&rng.bytes(32).try_into().unwrap());
        (s, p)
    };

    // 7.34 — smlen < 64: `m` untouched, `*mlen_p = 0`.
    let m = rng.bytes(48);
    let (_, sm_full, _) = att.run(48 + 64, &m, &sk, true);
    for smlen in 0..64usize {
        let sm = &sm_full[..smlen];
        // Give the output buffer some room so that a stray memset is visible.
        let (rc, out, ml) = opn.run(64, sm, &pk, true);
        eqi(&format!("open smlen={smlen} rc"), rc, -1);
        assert_eq!(ml, Some(0), "smlen={smlen}: *mlen_p must be 0");
        eqb(
            &format!("open smlen={smlen}: m untouched"),
            &out,
            &pattern(64),
        );
        // mlen_p == NULL on the short path too.
        let (rc, out, _) = opn.run(64, sm, &pk, false);
        eqi("open short mlen_p=NULL rc", rc, -1);
        eqb("open short mlen_p=NULL m untouched", &out, &pattern(64));
        // and via the generic alias.
        let (rc, _, ml) = gopn.run(64, sm, &pk, true);
        eqi("crypto_sign_open short rc", rc, -1);
        assert_eq!(ml, Some(0));
    }

    // 7.44 — smlen == 64 exactly (empty signed message).
    let (_, sm0, _) = att.run(64, &[], &sk, true);
    let (rc, _, ml) = opn.run(0, &sm0, &pk, true);
    eqi("open smlen=64 rc", rc, 0);
    assert_eq!(ml, Some(0));

    // 7.36/7.37 — every single byte of `sm` flipped: `-1`, `*mlen_p = 0` and
    // `m` **zeroed** over `smlen - 64` bytes (the memset in open.c:87).
    for len in [0usize, 1, 16, 31, 32, 48, 63, 64, 65, 127, 200] {
        let m = rng.bytes(len);
        let (_, sm, _) = att.run(len + 64, &m, &sk, true);
        for pos in 0..sm.len() {
            for bit in [0x01u8, 0x80] {
                let mut bad = sm.clone();
                bad[pos] ^= bit;
                let (rc, out, ml) = opn.run(len, &bad, &pk, true);
                if rc == 0 {
                    // Only possible if the flip is a no-op for the verifier;
                    // it never is for ed25519, but do not hard-code that.
                    continue;
                }
                eqi(&format!("open corrupt pos={pos} bit={bit:#x} rc"), rc, -1);
                assert_eq!(ml, Some(0));
                eqb(
                    &format!("open corrupt pos={pos}: m zeroed"),
                    &out,
                    &vec![0u8; len],
                );
            }
        }
        // 7.38 — right signature, wrong public key.
        let (rc, out, ml) = opn.run(len, &sm, &wrong_pk, true);
        eqi("open wrong pk rc", rc, -1);
        assert_eq!(ml, Some(0));
        eqb("open wrong pk: m zeroed", &out, &vec![0u8; len]);
        // m == NULL on the failure path: no memset, no crash.
        let (rc, ml) = opn.run_out_null(&sm, &wrong_pk, true);
        eqi("open wrong pk m=NULL rc", rc, -1);
        assert_eq!(ml, Some(0));
    }
}

// =========================================================== 7.39–7.46

#[test]
fn sign_verify_corrupted_signature() {
    let ver = PV::new("crypto_sign_ed25519_verify_detached");
    let mut rng = Rng::new(0xC044_097E_D008);
    let (pk, sk) = keys_from_seed(&rng.bytes(32).try_into().unwrap());

    for len in [0usize, 1, 32, 64, 100, 255, 300] {
        let m = rng.bytes(len);
        let sig = c_sign(&m, &sk);
        eqi("baseline verify", ver.run(&sig, &m, &pk), 0);
        // Every byte position, two independent bit flips each.
        for pos in 0..64usize {
            for bit in [0x01u8, 0x10, 0x80] {
                let mut bad = sig;
                bad[pos] ^= bit;
                let rc = ver.run(&bad, &m, &pk);
                assert_eq!(rc, -1, "corrupting sig[{pos}] with {bit:#x} must fail");
            }
        }
        // Whole-signature replacement patterns.
        for pat in [[0u8; 64], [0xffu8; 64]] {
            assert_eq!(ver.run(&pat, &m, &pk), -1);
        }
        // A valid signature under a different key.
        let (pk2, sk2) = keys_from_seed(&rng.bytes(32).try_into().unwrap());
        let sig2 = c_sign(&m, &sk2);
        assert_eq!(ver.run(&sig2, &m, &pk), -1, "cross-key must fail");
        assert_eq!(ver.run(&sig, &m, &pk2), -1, "cross-key must fail");
        eqi("sig2 under pk2", ver.run(&sig2, &m, &pk2), 0);
    }
}

#[test]
fn sign_verify_noncanonical_s() {
    let ver = PV::new("crypto_sign_ed25519_verify_detached");
    let mut rng = Rng::new(0x050A_1102_0009);
    let (pk, sk) = keys_from_seed(&rng.bytes(32).try_into().unwrap());
    let m = rng.bytes(37);
    let sig = c_sign(&m, &sk);
    eqi("baseline", ver.run(&sig, &m, &pk), 0);

    let l = h32(L_HEX);

    // 7.39 — S + L (still ≡ S mod L, but non-canonical) → rejected because
    // sig[63] & 240 != 0 gates the sc25519_is_canonical() test.
    let mut s_plus_l = [0u8; 32];
    let mut carry = 0u16;
    for i in 0..32 {
        let v = sig[32 + i] as u16 + l[i] as u16 + carry;
        s_plus_l[i] = v as u8;
        carry = v >> 8;
    }
    let mut bad = sig;
    bad[32..].copy_from_slice(&s_plus_l);
    assert_ne!(bad[63] & 240, 0, "S+L must have a non-zero high nibble");
    eqi("S + L rejected", ver.run(&bad, &m, &pk), -1);

    // 7.40 — S == L exactly is rejected; S == L-1 is *canonical* and accepted
    // by the canonicality test (though the equation then fails).
    let mut bad = sig;
    bad[32..].copy_from_slice(&l);
    eqi("S == L rejected", ver.run(&bad, &m, &pk), -1);
    let mut l_minus_1 = l;
    l_minus_1[0] -= 1;
    let mut bad = sig;
    bad[32..].copy_from_slice(&l_minus_1);
    eqi("S == L-1 rejected by the equation", ver.run(&bad, &m, &pk), -1);

    // 7.41 — the short-circuit: any S with (sig[63] & 240) == 0 skips the
    // canonicality test entirely.  Every S < 2^252 is canonical anyway, so
    // this must not accept anything extra; check a spread of such values.
    for _ in 0..32 {
        let mut bad = sig;
        let s: Vec<u8> = rng.bytes(32);
        bad[32..].copy_from_slice(&s);
        bad[63] &= 0x0f;
        assert_eq!(bad[63] & 240, 0);
        eqi("random S < 2^252", ver.run(&bad, &m, &pk), -1);
    }
    // ... and the same S values with the high nibble forced non-zero, which
    // takes the constant-time canonicality path instead.
    for _ in 0..32 {
        let mut bad = sig;
        let s: Vec<u8> = rng.bytes(32);
        bad[32..].copy_from_slice(&s);
        bad[63] |= 0xf0;
        eqi("random S >= 2^252", ver.run(&bad, &m, &pk), -1);
    }
    // S = all-0xff (definitely >= L) and S = 0.
    let mut bad = sig;
    bad[32..].copy_from_slice(&[0xffu8; 32]);
    eqi("S = ff..ff", ver.run(&bad, &m, &pk), -1);
    let mut bad = sig;
    bad[32..].copy_from_slice(&[0u8; 32]);
    eqi("S = 0", ver.run(&bad, &m, &pk), -1);

    // 7.46 — the strict-vs-compat axis.  `S` is always reduced mod `L`, and
    // `L` is only ~2^252 + 2.8e37, so a *genuine* signature essentially always
    // has `sig[63] & 240 == 0` and short-circuits past `sc25519_is_canonical`.
    // Check that this really is the case for a batch of real signatures ...
    let mut saw_lo = 0usize;
    let mut saw_hi = 0usize;
    for i in 0..64u32 {
        let mm = i.to_le_bytes().to_vec();
        let s = c_sign(&mm, &sk);
        eqi("valid signature", ver.run(&s, &mm, &pk), 0);
        if s[63] & 240 == 0 {
            saw_lo += 1;
        } else {
            saw_hi += 1;
        }
    }
    assert_eq!(saw_hi, 0, "S >= 2^252 is astronomically unlikely for a real signature");
    assert_eq!(saw_lo, 64);

    // ... and that the other side of the conjunction is still exercised: the
    // canonical values `S ∈ [2^252, L)` do have `sig[63] & 240 != 0`, so they
    // *do* run `sc25519_is_canonical`, which accepts them; the signature is
    // then rejected by the group equation instead.  `L-1`, `L-2`, `2^252` and
    // `2^252 + k` are all in that window.
    let mut window: Vec<[u8; 32]> = vec![l_minus_1];
    let mut l_minus_2 = l;
    l_minus_2[0] -= 2;
    window.push(l_minus_2);
    let mut p252 = [0u8; 32];
    p252[31] = 0x10;
    window.push(p252);
    for k in 1..8u8 {
        let mut v = p252;
        v[0] = k;
        window.push(v);
    }
    for s in &window {
        assert_ne!(s[31] & 240, 0, "expected the canonicality test to be reached");
        let mut bad = sig;
        bad[32..].copy_from_slice(s);
        eqi("canonical S >= 2^252", ver.run(&bad, &m, &pk), -1);
    }
}

#[test]
fn sign_verify_noncanonical_r_and_pk() {
    let ver = PV::new("crypto_sign_ed25519_verify_detached");
    let pk2c = PC::new("crypto_sign_ed25519_pk_to_curve25519");
    let mut rng = Rng::new(0x9017_CA90_000A);
    let (pk, sk) = keys_from_seed(&rng.bytes(32).try_into().unwrap());
    let m = rng.bytes(19);
    let sig = c_sign(&m, &sk);
    eqi("baseline", ver.run(&sig, &m, &pk), 0);

    // 7.42 / 7.56 — the 19 non-canonical y encodings (y ∈ [p, 2^255)), with
    // and without bit 255 set.  `verify_detached` runs ge25519_is_canonical()
    // and rejects them all; `pk_to_curve25519` has **no** such check, so the
    // two are structurally different — but no input actually distinguishes
    // them, because a non-canonical encoding reduces to y ∈ [0, 19) and none
    // of those is a non-small-order point on the main subgroup.  Both must
    // therefore reject, and the counts below pin that down.
    let mut pk2c_accepted = 0usize;
    for y in 0..19u8 {
        for hi in [0x7fu8, 0xff] {
            let mut nc = [0xffu8; 32];
            nc[0] = 0xedu8.wrapping_add(y);
            nc[31] = hi;
            // As a public key.
            eqi(
                &format!("non-canonical pk (y={y}, hi={hi:#x})"),
                ver.run(&sig, &m, &nc),
                -1,
            );
            // As R.
            let mut bad = sig;
            bad[..32].copy_from_slice(&nc);
            let rc = ver.run(&bad, &m, &pk);
            assert_eq!(rc, -1, "non-canonical R (y={y}) must not verify");
            // 7.56 — pk_to_curve25519 differential (no canonicality check).
            let (rc2, out2) = pk2c.run(32, &nc);
            if rc2 == 0 {
                pk2c_accepted += 1;
            } else {
                eqb("pk_to_curve25519 output untouched", &out2, &pattern(32));
            }
        }
    }
    assert_eq!(
        pk2c_accepted, 0,
        "no non-canonical encoding decodes to a main-subgroup point, so the \
         missing ge25519_is_canonical() check in pk_to_curve25519 is not \
         observable through the public API"
    );

    // 7.45 — R is not canonicality-checked, only decodability-checked: feed a
    // spread of random 32-byte R values.
    for _ in 0..64 {
        let mut bad = sig;
        let r: Vec<u8> = rng.bytes(32);
        bad[..32].copy_from_slice(&r);
        assert_eq!(ver.run(&bad, &m, &pk), -1);
    }

    // 7.43 — pk that does not decode (x² non-square).  Search for some.
    let mut nondecodable = 0;
    for i in 0..64u32 {
        let mut cand = [0u8; 32];
        cand[..4].copy_from_slice(&i.to_le_bytes());
        cand[31] &= 0x7f;
        let rc = ver.run(&sig, &m, &cand);
        assert_eq!(rc, -1);
        // pk_to_curve25519 differential on the same value.
        let (rc2, _) = pk2c.run(32, &cand);
        if rc2 == -1 {
            nondecodable += 1;
        }
    }
    assert!(nondecodable > 0, "expected some non-decodable candidates");

    // 7.44 / 7.46 — the 8 small-order encodings as pk and as R.
    for enc in ED_SMALL_ORDER {
        let so = h32(enc);
        eqi(&format!("small-order pk {enc}"), ver.run(&sig, &m, &so), -1);
        let mut bad = sig;
        bad[..32].copy_from_slice(&so);
        eqi(&format!("small-order R {enc}"), ver.run(&bad, &m, &pk), -1);
        // 7.54 — pk_to_curve25519 rejects them too.
        let (rc, out) = pk2c.run(32, &so);
        eqi("pk_to_curve25519 small order rc", rc, -1);
        eqb("pk_to_curve25519 leaves the output untouched", &out, &pattern(32));
    }

    // All-zero and all-0xff keys.
    for pat in [[0u8; 32], [0xffu8; 32]] {
        eqi("degenerate pk", ver.run(&sig, &m, &pat), -1);
        let (_rc, _out) = pk2c.run(32, &pat);
        let mut bad = sig;
        bad[..32].copy_from_slice(&pat);
        eqi("degenerate R", ver.run(&bad, &m, &pk), -1);
    }
    // All-zero / all-0xff secret keys are structurally valid inputs to signing.
    for pat in [[0u8; 64], [0xffu8; 64]] {
        let det = P5::new("crypto_sign_ed25519_detached");
        let (rc, s, sl) = det.run(64, &m, &pat, true);
        eqi("sign with degenerate sk rc", rc, 0);
        assert_eq!(sl, Some(64));
        // sig[32..64] is overwritten by S, but the pk embedded in sk is what
        // the hash used; verifying against sk[32..64] is what a caller would do.
        let claimed: [u8; 32] = pat[32..].try_into().unwrap();
        let _ = ver.run(&s, &m, &claimed);
    }
}

// =========================================================== 7.47, 7.48, 7.55

#[test]
fn sign_verify_cofactored_and_offsubgroup() {
    // Cofactored verification: `return ge25519_has_small_order(&check) - 1;`
    //
    // Construct a public key A' = A + T with T of order 8, then sign with the
    // *doctored* secret key `seed ‖ A'`.  `_crypto_sign_ed25519_detached`
    // takes the public key it hashes from `sk[32..64]`, so the resulting
    // signature satisfies S·B = R + h·A with h = SHA-512(R ‖ A' ‖ m).  The
    // verifier then computes check = R - (S·B - h·A') = h·T, which is a
    // non-identity small-order point — accepted only because verification is
    // cofactored.
    if !has("crypto_core_ed25519_add") || !has("crypto_core_ed25519_is_valid_point") {
        return;
    }
    let (addc, _) = both::<Add3>("crypto_core_ed25519_add");
    let (validc, validr) = both::<IsValid>("crypto_core_ed25519_is_valid_point");
    let det = P5::new("crypto_sign_ed25519_detached");
    let ver = PV::new("crypto_sign_ed25519_verify_detached");
    let pk2c = PC::new("crypto_sign_ed25519_pk_to_curve25519");

    let mut rng = Rng::new(0x0C0F_AC70_200B);
    let mut accepted = 0;
    // The order-8 and order-4 torsion generators.
    for t_hex in [ED_SMALL_ORDER[4], ED_SMALL_ORDER[5], ED_SMALL_ORDER[6], ED_SMALL_ORDER[7], ED_SMALL_ORDER[2], ED_SMALL_ORDER[3]] {
        let t = h32(t_hex);
        for _ in 0..3 {
            let seed: [u8; 32] = rng.bytes(32).try_into().unwrap();
            let (pk, sk) = keys_from_seed(&seed);
            let mut pk2 = [0u8; 32];
            if unsafe { addc(pk2.as_mut_ptr(), pk.as_ptr(), t.as_ptr()) } != 0 {
                continue;
            }
            // pk2 must be a valid point that is *not* on the main subgroup.
            let (a, b) = unsafe { (validc(pk2.as_ptr()), validr(pk2.as_ptr())) };
            eqi("is_valid_point(A + T)", a, b);
            if a == 0 {
                // Off the main subgroup: exactly the case we want.
                let mut sk2 = sk;
                sk2[32..].copy_from_slice(&pk2);
                let mlen = 1 + rng.below(64);
                let m = rng.bytes(mlen);
                let (rc, sig, _) = det.run(64, &m, &sk2, true);
                eqi("sign with doctored sk rc", rc, 0);
                // 7.47/7.48 — accepted despite A' having order 8L.
                let rc = ver.run(&sig, &m, &pk2);
                if rc == 0 {
                    accepted += 1;
                    // The same signature must NOT verify under the honest pk.
                    eqi("cofactored sig under honest pk", ver.run(&sig, &m, &pk), -1);
                    // ... and `pk_to_curve25519` *does* apply the subgroup
                    // check, so it rejects the very key verification accepted.
                    let (rc2, out) = pk2c.run(32, &pk2);
                    eqi("pk_to_curve25519(A + T)", rc2, -1);
                    eqb("pk_to_curve25519 output untouched", &out, &pattern(32));
                }
            }
        }
    }
    assert!(
        accepted > 0,
        "expected at least one cofactored acceptance of an order-8L public key"
    );
}

// =========================================================== 7.49–7.54

#[test]
fn sign_multipart() {
    let n = statebytes();
    let (ic, ir) = both::<InitFn>("crypto_sign_init");
    let (uc, ur) = both::<UpdateFn>("crypto_sign_update");
    let (fc, fr) = both::<FinalCreateFn>("crypto_sign_final_create");
    let (vc, vr) = both::<FinalVerifyFn>("crypto_sign_final_verify");
    let (eic, eir) = both::<InitFn>("crypto_sign_ed25519ph_init");
    let (euc, eur) = both::<UpdateFn>("crypto_sign_ed25519ph_update");
    let (efc, efr) = both::<FinalCreateFn>("crypto_sign_ed25519ph_final_create");
    let (evc, evr) = both::<FinalVerifyFn>("crypto_sign_ed25519ph_final_verify");

    let mut rng = Rng::new(0x0111_0170_000C);
    let (pk, sk) = keys_from_seed(&rng.bytes(32).try_into().unwrap());

    /// Run init + the given chunks + final_create in both libraries and in
    /// both namespaces, comparing states and signatures at every step.
    #[allow(clippy::too_many_arguments)]
    fn create(
        n: usize,
        ic: &Symbol<'static, InitFn>,
        ir: &Symbol<'static, InitFn>,
        uc: &Symbol<'static, UpdateFn>,
        ur: &Symbol<'static, UpdateFn>,
        fc: &Symbol<'static, FinalCreateFn>,
        fr: &Symbol<'static, FinalCreateFn>,
        chunks: &[&[u8]],
        sk: &[u8; 64],
        with_len: bool,
    ) -> (c_int, Vec<u8>, Option<u64>) {
        let mut sc = St::new(n);
        let mut sr = St::new(n);
        let a = unsafe { ic(sc.p()) };
        let b = unsafe { ir(sr.p()) };
        eqi("init rc", a, b);
        eqb("state after init", sc.bytes(), sr.bytes());
        for ch in chunks {
            let a = unsafe { uc(sc.p(), ch.as_ptr(), ch.len() as u64) };
            let b = unsafe { ur(sr.p(), ch.as_ptr(), ch.len() as u64) };
            eqi("update rc", a, b);
            eqb("state after update", sc.bytes(), sr.bytes());
        }
        let mut oc = prefill(64);
        let mut or = prefill(64);
        let mut lc = SENTINEL;
        let mut lr = SENTINEL;
        let pc = if with_len { &mut lc as *mut u64 } else { NOLEN };
        let pr = if with_len { &mut lr as *mut u64 } else { NOLEN };
        let a = unsafe { fc(sc.p(), oc.as_mut_ptr(), pc, sk.as_ptr()) };
        let b = unsafe { fr(sr.p(), or.as_mut_ptr(), pr, sk.as_ptr()) };
        eqi("final_create rc", a, b);
        eqb("final_create sig", &oc, &or);
        check_pad("final_create", &oc, 64);
        eqb("state after final_create", sc.bytes(), sr.bytes());
        if with_len {
            assert_eq!(lc, lr, "final_create *siglen_p mismatch");
        }
        (a, oc[..64].to_vec(), if with_len { Some(lc) } else { None })
    }

    #[allow(clippy::too_many_arguments)]
    fn verify(
        n: usize,
        ic: &Symbol<'static, InitFn>,
        ir: &Symbol<'static, InitFn>,
        uc: &Symbol<'static, UpdateFn>,
        ur: &Symbol<'static, UpdateFn>,
        vc: &Symbol<'static, FinalVerifyFn>,
        vr: &Symbol<'static, FinalVerifyFn>,
        chunks: &[&[u8]],
        sig: &[u8],
        pk: &[u8; 32],
    ) -> c_int {
        let mut sc = St::new(n);
        let mut sr = St::new(n);
        unsafe {
            ic(sc.p());
            ir(sr.p());
        }
        for ch in chunks {
            unsafe {
                uc(sc.p(), ch.as_ptr(), ch.len() as u64);
                ur(sr.p(), ch.as_ptr(), ch.len() as u64);
            }
        }
        let a = unsafe { vc(sc.p(), sig.as_ptr(), pk.as_ptr()) };
        let b = unsafe { vr(sr.p(), sig.as_ptr(), pk.as_ptr()) };
        eqi("final_verify rc", a, b);
        eqb("state after final_verify", sc.bytes(), sr.bytes());
        a
    }

    // ---- 7.49: zero update calls ----
    let (rc, sig0, sl) = create(n, &ic, &ir, &uc, &ur, &fc, &fr, &[], &sk, true);
    eqi("0-chunk final_create rc", rc, 0);
    assert_eq!(sl, Some(64));
    eqi(
        "0-chunk final_verify",
        verify(n, &ic, &ir, &uc, &ur, &vc, &vr, &[], &sig0, &pk),
        0,
    );
    // The explicit ed25519ph namespace must be byte-identical.
    let (rc, sig0e, _) = create(n, &eic, &eir, &euc, &eur, &efc, &efr, &[], &sk, true);
    eqi("ph 0-chunk rc", rc, 0);
    eqb("ph 0-chunk == generic", &sig0e, &sig0);
    eqi(
        "ph 0-chunk final_verify",
        verify(n, &eic, &eir, &euc, &eur, &evc, &evr, &[], &sig0, &pk),
        0,
    );
    // A single zero-length update is equivalent to none.
    let empty: &[u8] = &[];
    let (_, sig0b, _) = create(n, &ic, &ir, &uc, &ur, &fc, &fr, &[empty], &sk, true);
    eqb("one empty chunk == zero chunks", &sig0b, &sig0);
    // 7.52 — siglen_p == NULL.
    let (rc, sig0c, _) = create(n, &ic, &ir, &uc, &ur, &fc, &fr, &[], &sk, false);
    eqi("0-chunk siglen_p=NULL rc", rc, 0);
    eqb("0-chunk siglen_p=NULL sig", &sig0c, &sig0);

    // ---- 7.50: one update call, many lengths ----
    for len in [0usize, 1, 31, 32, 33, 63, 64, 65, 111, 112, 113, 127, 128, 129, 255, 256, 300, 1024, 4096] {
        let m = rng.bytes(len);
        let (rc, sig, sl) = create(n, &ic, &ir, &uc, &ur, &fc, &fr, &[&m], &sk, true);
        eqi("1-chunk rc", rc, 0);
        assert_eq!(sl, Some(64));
        eqi(
            "1-chunk verify",
            verify(n, &ic, &ir, &uc, &ur, &vc, &vr, &[&m], &sig, &pk),
            0,
        );
        // 7.53 — the prehashed domain must not cross-verify with the one-shot
        // detached form over the same bytes.
        let one_shot = c_sign(&m, &sk);
        assert_ne!(
            hex(&sig),
            hex(&one_shot),
            "ed25519ph and ed25519 signatures must differ (DOM2PREFIX)"
        );
        let vd = PV::new("crypto_sign_ed25519_verify_detached");
        eqi("ph signature via verify_detached", vd.run(&sig, &m, &pk), -1);
        eqi(
            "one-shot signature via final_verify",
            verify(n, &ic, &ir, &uc, &ur, &vc, &vr, &[&m], &one_shot, &pk),
            -1,
        );
    }

    // ---- 7.51 / 7.54: many update calls, streaming invariance ----
    let msg = rng.bytes(1024);
    let splits: Vec<Vec<usize>> = vec![
        vec![1024],
        vec![0, 1024],
        vec![1024, 0],
        vec![512, 512],
        vec![1023, 1],
        vec![1, 1023],
        vec![127, 1, 128, 1, 255, 512],
        (0..16).map(|_| 64).collect(),
        (0..1024).map(|_| 1).collect(),
        vec![111, 1, 112, 1, 113, 686],
    ];
    let mut reference: Option<Vec<u8>> = None;
    for sp in &splits {
        assert_eq!(sp.iter().sum::<usize>(), 1024);
        let mut off = 0;
        let chunks: Vec<&[u8]> = sp
            .iter()
            .map(|k| {
                let s = &msg[off..off + k];
                off += k;
                s
            })
            .collect();
        let (rc, sig, _) = create(n, &ic, &ir, &uc, &ur, &fc, &fr, &chunks, &sk, true);
        eqi("multi-chunk rc", rc, 0);
        match &reference {
            None => reference = Some(sig.clone()),
            Some(r) => eqb("streaming invariance", &sig, r),
        }
        // 7.54 — chunked differently on the verify side.
        eqi(
            "cross-chunked verify",
            verify(n, &ic, &ir, &uc, &ur, &vc, &vr, &[&msg], &sig, &pk),
            0,
        );
        eqi(
            "cross-chunked verify (byte at a time)",
            verify(
                n,
                &ic,
                &ir,
                &uc,
                &ur,
                &vc,
                &vr,
                &msg.chunks(1).collect::<Vec<_>>(),
                &sig,
                &pk,
            ),
            0,
        );
    }
    // Randomized chunkings.
    let reference = reference.unwrap();
    for _ in 0..24 {
        let mut chunks: Vec<&[u8]> = Vec::new();
        let mut off = 0;
        while off < msg.len() {
            let k = rng.range(1, std::cmp::min(200, msg.len() - off));
            chunks.push(&msg[off..off + k]);
            off += k;
        }
        assert_eq!(off, msg.len());
        let (_, sig, _) = create(n, &ic, &ir, &uc, &ur, &fc, &fr, &chunks, &sk, true);
        eqb("randomized streaming invariance", &sig, &reference);
    }

    // A mismatched prehash must fail, and every byte of a prehashed signature
    // must matter.
    for pos in 0..64usize {
        let mut bad: Vec<u8> = reference.clone();
        bad[pos] ^= 0x20;
        eqi(
            "corrupted ph signature",
            verify(n, &ic, &ir, &uc, &ur, &vc, &vr, &[&msg], &bad, &pk),
            -1,
        );
    }
    let mut other = msg.clone();
    other[0] ^= 1;
    eqi(
        "different prehash",
        verify(n, &ic, &ir, &uc, &ur, &vc, &vr, &[&other], &reference, &pk),
        -1,
    );
    // Wrong public key.
    let (pk2, _) = keys_from_seed(&rng.bytes(32).try_into().unwrap());
    eqi(
        "wrong pk",
        verify(n, &ic, &ir, &uc, &ur, &vc, &vr, &[&msg], &reference, &pk2),
        -1,
    );
    // Small-order / degenerate public keys on the ph path.
    for enc in ED_SMALL_ORDER {
        eqi(
            "small-order pk on ph path",
            verify(n, &ic, &ir, &uc, &ur, &vc, &vr, &[&msg], &reference, &h32(enc)),
            -1,
        );
    }
}

// =========================================================== internal API

#[test]
fn sign_internal_prehashed_entry_points() {
    // `_crypto_sign_ed25519_detached` / `_verify_detached` take the extra
    // `prehashed` flag; `prehashed = 0` must reproduce the public one-shot form
    // and `prehashed = 1` the ed25519ph form.
    if !has("_crypto_sign_ed25519_detached") || !has("_crypto_sign_ed25519_verify_detached") {
        return;
    }
    let (dc, dr) = both::<Sign6Fn>("_crypto_sign_ed25519_detached");
    let (vc, vr) = both::<Verify5Fn>("_crypto_sign_ed25519_verify_detached");
    let mut rng = Rng::new(0x0117_E4A1_000D);
    let (pk, sk) = keys_from_seed(&rng.bytes(32).try_into().unwrap());

    for len in [0usize, 1, 32, 64, 65, 128, 300] {
        let m = rng.bytes(len);
        for prehashed in [0i32, 1] {
            let mut oc = prefill(64);
            let mut or = prefill(64);
            let mut lc = SENTINEL;
            let mut lr = SENTINEL;
            let a = unsafe {
                dc(oc.as_mut_ptr(), &mut lc, m.as_ptr(), len as u64, sk.as_ptr(), prehashed)
            };
            let b = unsafe {
                dr(or.as_mut_ptr(), &mut lr, m.as_ptr(), len as u64, sk.as_ptr(), prehashed)
            };
            eqi("_detached rc", a, b);
            eqb("_detached sig", &oc, &or);
            check_pad("_detached", &oc, 64);
            assert_eq!(lc, lr);
            assert_eq!(lc, 64);
            let sig = &oc[..64];
            let x = unsafe { vc(sig.as_ptr(), m.as_ptr(), len as u64, pk.as_ptr(), prehashed) };
            let y = unsafe { vr(sig.as_ptr(), m.as_ptr(), len as u64, pk.as_ptr(), prehashed) };
            eqi("_verify_detached matching domain", x, y);
            assert_eq!(x, 0);
            // The other domain must reject.
            let other = 1 - prehashed;
            let x = unsafe { vc(sig.as_ptr(), m.as_ptr(), len as u64, pk.as_ptr(), other) };
            let y = unsafe { vr(sig.as_ptr(), m.as_ptr(), len as u64, pk.as_ptr(), other) };
            eqi("_verify_detached crossed domain", x, y);
            assert_eq!(x, -1);
            if prehashed == 0 {
                eqb("prehashed=0 == crypto_sign_ed25519_detached", sig, &c_sign(&m, &sk));
            }
        }
    }

    // `_crypto_sign_ed25519_ref10_hinit` must produce the same SHA-512 state
    // in both libraries, for both `prehashed` values.
    if has("_crypto_sign_ed25519_ref10_hinit")
        && has("crypto_hash_sha512_update")
        && has("crypto_hash_sha512_final")
        && has("crypto_hash_sha512_statebytes")
    {
        let (sbc, sbr) = both::<SizeFn>("crypto_hash_sha512_statebytes");
        let sn = unsafe { sbc() };
        assert_eq!(sn, unsafe { sbr() });
        let (hc, hr) = both::<HinitFn>("_crypto_sign_ed25519_ref10_hinit");
        let (uc, ur) = both::<UpdateFn>("crypto_hash_sha512_update");
        let (fc2, fr2) = both::<Sha512Final>("crypto_hash_sha512_final");
        for prehashed in [0i32, 1] {
            for len in [0usize, 1, 34, 63, 64, 128, 200] {
                let m = rng.bytes(len);
                let mut sc = St::new(sn);
                let mut sr = St::new(sn);
                unsafe {
                    hc(sc.p(), prehashed);
                    hr(sr.p(), prehashed);
                }
                eqb("hinit state", sc.bytes(), sr.bytes());
                unsafe {
                    uc(sc.p(), m.as_ptr(), len as u64);
                    ur(sr.p(), m.as_ptr(), len as u64);
                }
                let mut oc = prefill(64);
                let mut or = prefill(64);
                unsafe {
                    fc2(sc.p(), oc.as_mut_ptr());
                    fr2(sr.p(), or.as_mut_ptr());
                }
                eqb("hinit digest", &oc, &or);
                check_pad("hinit digest", &oc, 64);
            }
        }
    }
}

// =========================================================== 7.56–7.58

#[test]
fn sign_key_conversions() {
    let pk2c = PC::new("crypto_sign_ed25519_pk_to_curve25519");
    let sk2c = PC::new("crypto_sign_ed25519_sk_to_curve25519");
    let (basec, baser) = both::<MultBase>("crypto_scalarmult_curve25519_base");
    let (hashc, hashr) = both::<HashFn>("crypto_hash_sha512");

    let mut rng = Rng::new(0x00C0_4E47_000E);
    for _ in 0..24 {
        let seed: [u8; 32] = rng.bytes(32).try_into().unwrap();
        let (pk, sk) = keys_from_seed(&seed);

        // 7.57 — csk = clamp(SHA-512(sk[0..32])[0..32]); always succeeds.
        let (rc, csk) = sk2c.run(32, &sk);
        eqi("sk_to_curve25519 rc", rc, 0);
        let mut hc = [0u8; 64];
        let mut hr = [0u8; 64];
        unsafe {
            hashc(hc.as_mut_ptr(), sk.as_ptr(), 32);
            hashr(hr.as_mut_ptr(), sk.as_ptr(), 32);
        }
        eqb("crypto_hash_sha512(seed)", &hc, &hr);
        let mut want = hc[..32].to_vec();
        want[0] &= 248;
        want[31] &= 127;
        want[31] |= 64;
        eqb("sk_to_curve25519 value", &csk, &want);

        // 7.56 — pk_to_curve25519(pk) == curve25519_base(sk_to_curve25519(sk)).
        let (rc, cpk) = pk2c.run(32, &pk);
        eqi("pk_to_curve25519 rc", rc, 0);
        let mut bc = prefill(32);
        let mut br = prefill(32);
        unsafe {
            basec(bc.as_mut_ptr(), csk.as_ptr());
            baser(br.as_mut_ptr(), csk.as_ptr());
        }
        eqb("scalarmult_base agreement", &bc, &br);
        eqb("pk_to_curve25519 == base(sk_to_curve25519)", &cpk, &bc[..32]);
    }

    // Degenerate secret keys: 64 zero bytes and 64 0xff bytes.
    for pat in [[0u8; 64], [0xffu8; 64]] {
        let (rc, csk) = sk2c.run(32, &pat);
        eqi("sk_to_curve25519 degenerate rc", rc, 0);
        assert_eq!(csk[0] & 7, 0, "must be clamped");
        assert_eq!(csk[31] & 0xc0, 0x40, "must be clamped");
    }
}

#[test]
fn sign_to_box_bridge() {
    // 7.58 — full cross-protocol bridge: two ed25519 keypairs, converted, then
    // a crypto_box round trip through the converted keys.
    type BeforeNm = unsafe extern "C" fn(*mut u8, *const u8, *const u8) -> c_int;
    type Easy = unsafe extern "C" fn(*mut u8, *const u8, u64, *const u8, *const u8, *const u8) -> c_int;
    let pk2c = PC::new("crypto_sign_ed25519_pk_to_curve25519");
    let sk2c = PC::new("crypto_sign_ed25519_sk_to_curve25519");
    let (bnc, bnr) = both::<BeforeNm>("crypto_box_beforenm");
    let (ec, er) = both::<Easy>("crypto_box_easy");
    let (odc, odr) = both::<Easy>("crypto_box_open_easy");

    let mut rng = Rng::new(0x00B4_1D6E_000F);
    for _ in 0..8 {
        let (pk_a, sk_a) = keys_from_seed(&rng.bytes(32).try_into().unwrap());
        let (pk_b, sk_b) = keys_from_seed(&rng.bytes(32).try_into().unwrap());
        let (_, cpk_a) = pk2c.run(32, &pk_a);
        let (_, csk_a) = sk2c.run(32, &sk_a);
        let (_, cpk_b) = pk2c.run(32, &pk_b);
        let (_, csk_b) = sk2c.run(32, &sk_b);

        // beforenm is symmetric across the bridge.
        let mut kc1 = prefill(32);
        let mut kr1 = prefill(32);
        let a = unsafe { bnc(kc1.as_mut_ptr(), cpk_b.as_ptr(), csk_a.as_ptr()) };
        let b = unsafe { bnr(kr1.as_mut_ptr(), cpk_b.as_ptr(), csk_a.as_ptr()) };
        eqi("beforenm(A→B) rc", a, b);
        eqb("beforenm(A→B)", &kc1, &kr1);
        let mut kc2 = prefill(32);
        let mut kr2 = prefill(32);
        let a = unsafe { bnc(kc2.as_mut_ptr(), cpk_a.as_ptr(), csk_b.as_ptr()) };
        let b = unsafe { bnr(kr2.as_mut_ptr(), cpk_a.as_ptr(), csk_b.as_ptr()) };
        eqi("beforenm(B→A) rc", a, b);
        eqb("beforenm(B→A)", &kc2, &kr2);
        eqb("beforenm symmetry", &kc1, &kc2);

        let nonce = rng.bytes(24);
        for len in [0usize, 1, 33, 64, 300] {
            let m = rng.bytes(len);
            let mut cc = prefill(len + 16);
            let mut cr = prefill(len + 16);
            let a = unsafe {
                ec(cc.as_mut_ptr(), m.as_ptr(), len as u64, nonce.as_ptr(), cpk_b.as_ptr(), csk_a.as_ptr())
            };
            let b = unsafe {
                er(cr.as_mut_ptr(), m.as_ptr(), len as u64, nonce.as_ptr(), cpk_b.as_ptr(), csk_a.as_ptr())
            };
            eqi("bridged box_easy rc", a, b);
            eqb("bridged box_easy ct", &cc, &cr);
            assert_eq!(a, 0);
            let mut mc = prefill(len);
            let mut mr = prefill(len);
            let a = unsafe {
                odc(mc.as_mut_ptr(), cc.as_ptr(), (len + 16) as u64, nonce.as_ptr(), cpk_a.as_ptr(), csk_b.as_ptr())
            };
            let b = unsafe {
                odr(mr.as_mut_ptr(), cr.as_ptr(), (len + 16) as u64, nonce.as_ptr(), cpk_a.as_ptr(), csk_b.as_ptr())
            };
            eqi("bridged box_open_easy rc", a, b);
            eqb("bridged box_open_easy pt", &mc, &mr);
            assert_eq!(a, 0);
            eqb("bridged round trip", &mc[..len], &m);
        }
    }
}

// =========================================================== C ↔ Rust interop

#[test]
fn sign_cross_library_interop() {
    // A signature produced by C must verify under Rust and vice versa, for
    // every API shape, and the same for whole signed messages.
    let (dc, dr) = both::<Sign5Fn>("crypto_sign_ed25519_detached");
    let (vc, vr) = both::<VerifyFn>("crypto_sign_ed25519_verify_detached");
    let (ac, ar) = both::<Sign5Fn>("crypto_sign_ed25519");
    let (oc, or) = both::<Sign5Fn>("crypto_sign_ed25519_open");
    let (kc, kr) = both::<SeedKeypairFn>("crypto_sign_ed25519_seed_keypair");

    let mut rng = Rng::new(0x0117_E40D_0010);
    for _ in 0..8 {
        let seed: [u8; 32] = rng.bytes(32).try_into().unwrap();
        // Keys generated by C, used by Rust, and vice versa.
        let mut pkc = [0u8; 32];
        let mut skc = [0u8; 64];
        let mut pkr = [0u8; 32];
        let mut skr = [0u8; 64];
        unsafe {
            kc(pkc.as_mut_ptr(), skc.as_mut_ptr(), seed.as_ptr());
            kr(pkr.as_mut_ptr(), skr.as_mut_ptr(), seed.as_ptr());
        }
        eqb("interop pk", &pkc, &pkr);
        eqb("interop sk", &skc, &skr);

        for len in [0usize, 1, 63, 64, 65, 127, 128, 300, 1024] {
            let m = rng.bytes(len);
            let mut sigc = [0u8; 64];
            let mut sigr = [0u8; 64];
            unsafe {
                dc(sigc.as_mut_ptr(), NOLEN, m.as_ptr(), len as u64, skc.as_ptr());
                dr(sigr.as_mut_ptr(), NOLEN, m.as_ptr(), len as u64, skr.as_ptr());
            }
            eqb("interop signature", &sigc, &sigr);
            // C-signed → Rust-verified, and Rust-signed → C-verified.
            assert_eq!(
                unsafe { vr(sigc.as_ptr(), m.as_ptr(), len as u64, pkc.as_ptr()) },
                0,
                "Rust must accept a C signature"
            );
            assert_eq!(
                unsafe { vc(sigr.as_ptr(), m.as_ptr(), len as u64, pkr.as_ptr()) },
                0,
                "C must accept a Rust signature"
            );

            // Attached: C signs, Rust opens; Rust signs, C opens.
            let mut smc = vec![0u8; len + 64];
            let mut smr = vec![0u8; len + 64];
            unsafe {
                ac(smc.as_mut_ptr(), NOLEN, m.as_ptr(), len as u64, skc.as_ptr());
                ar(smr.as_mut_ptr(), NOLEN, m.as_ptr(), len as u64, skr.as_ptr());
            }
            eqb("interop signed message", &smc, &smr);
            let mut out = vec![0u8; len];
            let mut ml = 0u64;
            assert_eq!(
                unsafe {
                    or(out.as_mut_ptr(), &mut ml, smc.as_ptr(), (len + 64) as u64, pkc.as_ptr())
                },
                0,
                "Rust must open a C signed message"
            );
            assert_eq!(ml, len as u64);
            eqb("Rust-opened C message", &out, &m);
            let mut out = vec![0u8; len];
            let mut ml = 0u64;
            assert_eq!(
                unsafe {
                    oc(out.as_mut_ptr(), &mut ml, smr.as_ptr(), (len + 64) as u64, pkr.as_ptr())
                },
                0,
                "C must open a Rust signed message"
            );
            assert_eq!(ml, len as u64);
            eqb("C-opened Rust message", &out, &m);
        }
    }
}
