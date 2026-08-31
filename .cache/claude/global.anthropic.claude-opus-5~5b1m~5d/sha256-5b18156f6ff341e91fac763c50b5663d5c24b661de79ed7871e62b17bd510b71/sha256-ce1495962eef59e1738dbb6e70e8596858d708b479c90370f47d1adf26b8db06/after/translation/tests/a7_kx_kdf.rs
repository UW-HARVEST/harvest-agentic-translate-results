//! Area 7 — `crypto_kx` and `crypto_kdf` (blake2b, hkdf-sha256, hkdf-sha512).
//!
//! Covers `configs_7.md` rows 7.81–7.89 (kx) and 7.90–7.112 (kdf), and
//! `errors_7.md` rows 7.92–7.98 (kx) and 7.99–7.109 (kdf).
//!
//! Everything is called through `dlsym` on both the reference C `libsodium.so`
//! and the translated Rust `liblibsodium.so`; every output buffer is padded and
//! pre-filled so that "buffer untouched on failure" and "no out-of-bounds
//! write" are both observable, and the *full* buffer (not just the return code)
//! is compared after every call — including after failures.
mod common;
use common::*;
use libloading::Symbol;
use std::ffi::{c_char, c_int, CStr};
use std::ptr;

// ------------------------------------------------------------------- types

type SizeFn = unsafe extern "C" fn() -> usize;
type StrFn = unsafe extern "C" fn() -> *const c_char;
type Keypair = unsafe extern "C" fn(*mut u8, *mut u8) -> c_int;
type SeedKeypair = unsafe extern "C" fn(*mut u8, *mut u8, *const u8) -> c_int;
type Session = unsafe extern "C" fn(*mut u8, *mut u8, *const u8, *const u8, *const u8) -> c_int;
type MultBase = unsafe extern "C" fn(*mut u8, *const u8) -> c_int;
type Derive = unsafe extern "C" fn(*mut u8, usize, u64, *const c_char, *const u8) -> c_int;
type Keygen = unsafe extern "C" fn(*mut u8);
type Extract = unsafe extern "C" fn(*mut u8, *const u8, usize, *const u8, usize) -> c_int;
type Expand = unsafe extern "C" fn(*mut u8, usize, *const c_char, usize, *const u8) -> c_int;
type ExInit = unsafe extern "C" fn(*mut u8, *const u8, usize) -> c_int;
type ExUpdate = unsafe extern "C" fn(*mut u8, *const u8, usize) -> c_int;
type ExFinal = unsafe extern "C" fn(*mut u8, *mut u8) -> c_int;

/// An errno value libsodium never sets, so that "errno untouched" is
/// distinguishable from "errno set to EINVAL".
const SENT: c_int = 0x7f5a;

// ----------------------------------------------------------------- helpers

fn hx(s: &str) -> Vec<u8> {
    assert!(s.len() % 2 == 0, "odd hex length");
    (0..s.len() / 2)
        .map(|i| u8::from_str_radix(&s[2 * i..2 * i + 2], 16).unwrap())
        .collect()
}

/// Distinctive prefill so that "buffer untouched" is observable.
fn pat(n: usize) -> Vec<u8> {
    (0..n)
        .map(|i| 0x3Cu8.wrapping_add((i as u8).wrapping_mul(11)))
        .collect()
}

fn prefilled(n: usize) -> Vec<u8> {
    let mut v = padded(n);
    v[..n].copy_from_slice(&pat(n));
    v
}

#[track_caller]
fn size_eq(name: &str, expect: usize) {
    let (c, r) = both::<SizeFn>(name);
    let (a, b) = unsafe { (c(), r()) };
    assert_eq!(a, expect, "{name}: C returned {a}, expected {expect}");
    assert_eq!(b, a, "{name}: Rust returned {b}, C returned {a}");
}

#[track_caller]
fn str_eq(name: &str, expect: &str) {
    let (c, r) = both::<StrFn>(name);
    unsafe {
        let pc = c();
        let pr = r();
        assert!(!pc.is_null(), "{name}: C returned NULL");
        assert!(!pr.is_null(), "{name}: Rust returned NULL");
        let a = CStr::from_ptr(pc).to_str().unwrap();
        let b = CStr::from_ptr(pr).to_str().unwrap();
        assert_eq!(a, expect, "{name}: C string");
        assert_eq!(b, expect, "{name}: Rust string");
    }
}

/// 8-byte-aligned, padded scratch buffer for opaque C structs.
///
/// The hkdf states contain `uint64_t` members, so a plain `Vec<u8>` would only
/// be aligned by luck.  The trailing bytes carry the `check_pad` guard pattern.
struct SBuf {
    w: Vec<u64>,
    len: usize,
}

impl SBuf {
    fn new(len: usize) -> Self {
        let words = (len + PAD + 7) / 8;
        let mut s = SBuf { w: vec![0u64; words.max(1)], len };
        let total = s.total();
        {
            let b = s.bytes_mut();
            for i in len..total {
                b[i] = 0xA5u8.wrapping_add((i - len) as u8);
            }
        }
        s
    }
    fn total(&self) -> usize {
        self.w.len() * 8
    }
    fn bytes(&self) -> &[u8] {
        unsafe { std::slice::from_raw_parts(self.w.as_ptr() as *const u8, self.total()) }
    }
    fn bytes_mut(&mut self) -> &mut [u8] {
        unsafe { std::slice::from_raw_parts_mut(self.w.as_mut_ptr() as *mut u8, self.total()) }
    }
    fn ptr(&mut self) -> *mut u8 {
        self.w.as_mut_ptr() as *mut u8
    }
    /// The live (non-guard) portion.
    fn st(&self) -> &[u8] {
        &self.bytes()[..self.len]
    }
    #[track_caller]
    fn check(&self, what: &str) {
        check_pad(what, self.bytes(), self.len);
    }
}

// ===========================================================================
//                                 crypto_kx
// ===========================================================================

const KX_PK: usize = 32;
const KX_SK: usize = 32;
const KX_SEED: usize = 32;
const KX_SESSION: usize = 32;

/// The 7 curve25519 small-order blocklist encodings (`x25519_ref10.c:19-51`).
const SMALL_ORDER: [&str; 7] = [
    "0000000000000000000000000000000000000000000000000000000000000000",
    "0100000000000000000000000000000000000000000000000000000000000000",
    "e0eb7a7c3b41b8ae1656e3faf19fc46ada098deb9c32b1fd866205165f49b800",
    "5f9c95bca3508c24b1d0b1559c83ef5b04445cc4581c8e86d8224eddd09f1157",
    "ecffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff7f",
    "edffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff7f",
    "eeffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff7f",
];

/// NULL pattern for the `rx`/`tx` output pointers.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
enum Ptrs {
    /// both non-NULL, distinct buffers
    Both,
    /// `rx == NULL`, `tx` non-NULL
    RxNull,
    /// `tx == NULL`, `rx` non-NULL
    TxNull,
    /// both non-NULL but deliberately the *same* buffer
    Aliased,
}

struct Kx {
    name: &'static str,
    c: Symbol<'static, Session>,
    r: Symbol<'static, Session>,
}

impl Kx {
    fn new(name: &'static str) -> Self {
        let (c, r) = both::<Session>(name);
        Kx { name, c, r }
    }

    /// Differentially invoke the session-keys function.
    ///
    /// Returns `(rc, buf_a, buf_b)` where `buf_a` is the buffer that was handed
    /// to `rx` (or the shared buffer, when `rx == NULL`/aliased) and `buf_b` the
    /// one handed to `tx`.  Buffers not handed to the callee at all keep the
    /// prefill pattern, which is what makes "untouched" checkable.
    fn run(
        &self,
        label: &str,
        p: Ptrs,
        pk: &[u8],
        sk: &[u8],
        peer: &[u8],
    ) -> (c_int, Vec<u8>, Vec<u8>) {
        assert_eq!(pk.len(), KX_PK);
        assert_eq!(sk.len(), KX_SK);
        assert_eq!(peer.len(), KX_PK);

        let mut ac = prefilled(KX_SESSION);
        let mut bc = prefilled(KX_SESSION);
        let mut ar = prefilled(KX_SESSION);
        let mut br = prefilled(KX_SESSION);

        let (pac, pbc) = (ac.as_mut_ptr(), bc.as_mut_ptr());
        let (par, pbr) = (ar.as_mut_ptr(), br.as_mut_ptr());
        let (rxc, txc, rxr, txr) = match p {
            Ptrs::Both => (pac, pbc, par, pbr),
            Ptrs::RxNull => (ptr::null_mut(), pbc, ptr::null_mut(), pbr),
            Ptrs::TxNull => (pac, ptr::null_mut(), par, ptr::null_mut()),
            Ptrs::Aliased => (pac, pac, par, par),
        };

        let rc = unsafe { (self.c)(rxc, txc, pk.as_ptr(), sk.as_ptr(), peer.as_ptr()) };
        let rr = unsafe { (self.r)(rxr, txr, pk.as_ptr(), sk.as_ptr(), peer.as_ptr()) };

        let w = format!("{} [{label}] {p:?}", self.name);
        eqi(&format!("{w} rc"), rc, rr);
        eqb(&format!("{w} rx-buffer"), &ac, &ar);
        eqb(&format!("{w} tx-buffer"), &bc, &br);
        check_pad(&format!("{w} rx-buffer(C)"), &ac, KX_SESSION);
        check_pad(&format!("{w} rx-buffer(Rust)"), &ar, KX_SESSION);
        check_pad(&format!("{w} tx-buffer(C)"), &bc, KX_SESSION);
        check_pad(&format!("{w} tx-buffer(Rust)"), &br, KX_SESSION);

        (rc, ac[..KX_SESSION].to_vec(), bc[..KX_SESSION].to_vec())
    }
}

/// A deterministic kx keypair derived from `seed`, produced by the C library.
fn kx_pair(seed: &[u8]) -> (Vec<u8>, Vec<u8>) {
    let (c, _r) = both::<SeedKeypair>("crypto_kx_seed_keypair");
    let mut pk = vec![0u8; KX_PK];
    let mut sk = vec![0u8; KX_SK];
    let rc = unsafe { c(pk.as_mut_ptr(), sk.as_mut_ptr(), seed.as_ptr()) };
    assert_eq!(rc, 0, "C crypto_kx_seed_keypair failed");
    (pk, sk)
}

// ------------------------------------------------- config 7.89 / error 7.127

#[test]
fn kx_accessors() {
    size_eq("crypto_kx_publickeybytes", 32);
    size_eq("crypto_kx_secretkeybytes", 32);
    size_eq("crypto_kx_seedbytes", 32);
    size_eq("crypto_kx_sessionkeybytes", 32);
    str_eq("crypto_kx_primitive", "x25519blake2b");
    // `_primitive()` must be stable across calls (static storage, never NULL).
    let (c, r) = both::<StrFn>("crypto_kx_primitive");
    unsafe {
        assert_eq!(c(), c(), "C crypto_kx_primitive is not stable");
        assert_eq!(r(), r(), "Rust crypto_kx_primitive is not stable");
    }
}

// --------------------------------------------------------------- config 7.81

#[test]
fn kx_keypair_randomised() {
    let (ck, rk) = both::<Keypair>("crypto_kx_keypair");
    let (cb, rb) = both::<MultBase>("crypto_scalarmult_base");

    let mut prev: Option<Vec<u8>> = None;
    for i in 0..32u64 {
        // Both libraries draw from independent streams rewound to the same
        // seed, so the random secret key must come out identical.
        rng_reseed(0x1000 + i);
        let mut pkc = prefilled(KX_PK);
        let mut skc = prefilled(KX_SK);
        let mut pkr = prefilled(KX_PK);
        let mut skr = prefilled(KX_SK);
        let rc = unsafe { ck(pkc.as_mut_ptr(), skc.as_mut_ptr()) };
        let rr = unsafe { rk(pkr.as_mut_ptr(), skr.as_mut_ptr()) };
        eqi(&format!("7.81 keypair[{i}] rc"), rc, rr);
        assert_eq!(rc, 0, "7.81 keypair[{i}] must succeed (error row 7.98)");
        eqb(&format!("7.81 keypair[{i}] pk"), &pkc, &pkr);
        eqb(&format!("7.81 keypair[{i}] sk"), &skc, &skr);
        check_pad(&format!("7.81 keypair[{i}] pk(C)"), &pkc, KX_PK);
        check_pad(&format!("7.81 keypair[{i}] sk(C)"), &skc, KX_SK);
        check_pad(&format!("7.81 keypair[{i}] pk(Rust)"), &pkr, KX_PK);
        check_pad(&format!("7.81 keypair[{i}] sk(Rust)"), &skr, KX_SK);

        // pk == crypto_scalarmult_base(sk), in both libraries.
        let mut qc = padded(32);
        let mut qr = padded(32);
        assert_eq!(unsafe { cb(qc.as_mut_ptr(), skc.as_ptr()) }, 0);
        assert_eq!(unsafe { rb(qr.as_mut_ptr(), skr.as_ptr()) }, 0);
        eqb(&format!("7.81 keypair[{i}] pk == base(sk) (C)"), &qc[..32], &pkc[..KX_PK]);
        eqb(&format!("7.81 keypair[{i}] pk == base(sk) (Rust)"), &qr[..32], &pkr[..KX_PK]);

        // Distinct seeds must give distinct keys (config 7.95-style sanity).
        if let Some(p) = &prev {
            assert_ne!(p, &skc[..KX_SK].to_vec(), "7.81: successive keypairs identical");
        }
        prev = Some(skc[..KX_SK].to_vec());
    }
}

// --------------------------------------------------------------- config 7.82

#[test]
fn kx_seed_keypair() {
    let (c, r) = both::<SeedKeypair>("crypto_kx_seed_keypair");
    let (cb, rb) = both::<MultBase>("crypto_scalarmult_base");
    let (cgh, rgh) = both::<
        unsafe extern "C" fn(*mut u8, usize, *const u8, u64, *const u8, usize) -> c_int,
    >("crypto_generichash");

    let mut rng = Rng::new(0x5EED_0001);
    let mut seeds: Vec<Vec<u8>> = vec![
        vec![0u8; KX_SEED],
        vec![0xffu8; KX_SEED],
        (0..KX_SEED as u8).collect(),
    ];
    for _ in 0..24 {
        seeds.push(rng.bytes(KX_SEED));
    }

    let mut seen: Vec<Vec<u8>> = Vec::new();
    for (i, seed) in seeds.iter().enumerate() {
        let mut pkc = prefilled(KX_PK);
        let mut skc = prefilled(KX_SK);
        let mut pkr = prefilled(KX_PK);
        let mut skr = prefilled(KX_SK);
        let rc = unsafe { c(pkc.as_mut_ptr(), skc.as_mut_ptr(), seed.as_ptr()) };
        let rr = unsafe { r(pkr.as_mut_ptr(), skr.as_mut_ptr(), seed.as_ptr()) };
        eqi(&format!("7.82 seed[{i}] rc"), rc, rr);
        assert_eq!(rc, 0, "7.82 seed[{i}] must succeed (error row 7.98)");
        eqb(&format!("7.82 seed[{i}] pk"), &pkc, &pkr);
        eqb(&format!("7.82 seed[{i}] sk"), &skc, &skr);
        check_pad(&format!("7.82 seed[{i}] pk(C)"), &pkc, KX_PK);
        check_pad(&format!("7.82 seed[{i}] sk(C)"), &skc, KX_SK);
        check_pad(&format!("7.82 seed[{i}] pk(Rust)"), &pkr, KX_PK);
        check_pad(&format!("7.82 seed[{i}] sk(Rust)"), &skr, KX_SK);

        // sk == BLAKE2b-32(seed) with no key, pk == scalarmult_base(sk).
        let mut hc = vec![0u8; KX_SK];
        let mut hr = vec![0u8; KX_SK];
        unsafe {
            assert_eq!(
                cgh(hc.as_mut_ptr(), KX_SK, seed.as_ptr(), KX_SEED as u64, ptr::null(), 0),
                0
            );
            assert_eq!(
                rgh(hr.as_mut_ptr(), KX_SK, seed.as_ptr(), KX_SEED as u64, ptr::null(), 0),
                0
            );
        }
        eqb(&format!("7.82 seed[{i}] sk == blake2b(seed) (C)"), &hc, &skc[..KX_SK]);
        eqb(&format!("7.82 seed[{i}] sk == blake2b(seed) (Rust)"), &hr, &skr[..KX_SK]);

        let mut qc = padded(32);
        let mut qr = padded(32);
        assert_eq!(unsafe { cb(qc.as_mut_ptr(), skc.as_ptr()) }, 0);
        assert_eq!(unsafe { rb(qr.as_mut_ptr(), skr.as_ptr()) }, 0);
        eqb(&format!("7.82 seed[{i}] pk == base(sk) (C)"), &qc[..32], &pkc[..KX_PK]);
        eqb(&format!("7.82 seed[{i}] pk == base(sk) (Rust)"), &qr[..32], &pkr[..KX_PK]);

        // Deterministic: repeating the same seed reproduces the same keys.
        let mut pk2 = vec![0u8; KX_PK];
        let mut sk2 = vec![0u8; KX_SK];
        assert_eq!(unsafe { c(pk2.as_mut_ptr(), sk2.as_mut_ptr(), seed.as_ptr()) }, 0);
        eqb(&format!("7.82 seed[{i}] deterministic"), &pk2, &pkc[..KX_PK]);

        assert!(!seen.contains(&pkc[..KX_PK].to_vec()), "7.82: seed collision");
        seen.push(pkc[..KX_PK].to_vec());
    }
}

// ---------------------------------------------------- config 7.83 / 7.88

#[test]
fn kx_session_keys_round_trip() {
    let client = Kx::new("crypto_kx_client_session_keys");
    let server = Kx::new("crypto_kx_server_session_keys");

    let mut rng = Rng::new(0x5EED_0002);
    for i in 0..24 {
        let (cpk, csk) = kx_pair(&rng.bytes(KX_SEED));
        let (spk, ssk) = kx_pair(&rng.bytes(KX_SEED));

        let label = format!("7.83 pair[{i}]");
        let (rc0, crx, ctx) = client.run(&label, Ptrs::Both, &cpk, &csk, &spk);
        let (rc1, srx, stx) = server.run(&label, Ptrs::Both, &spk, &ssk, &cpk);
        assert_eq!(rc0, 0, "{label}: client must succeed");
        assert_eq!(rc1, 0, "{label}: server must succeed");

        // Full agreement.
        eqb(&format!("{label}: client_rx == server_tx"), &crx, &stx);
        eqb(&format!("{label}: client_tx == server_rx"), &ctx, &srx);
        assert_ne!(crx, ctx, "{label}: rx and tx keys must differ");

        // config 7.88: role asymmetry — the hash absorbs `client_pk ‖
        // server_pk` in a *fixed* order, so if the two peers disagree about who
        // is the client the session keys do not match at all.
        let (rc4, xrx, xtx) = client.run(&format!("{label} swapped"), Ptrs::Both, &spk, &ssk, &cpk);
        assert_eq!(rc4, 0);
        assert_ne!(xrx, stx, "{label}: swapped roles must not agree (rx)");
        assert_ne!(xtx, srx, "{label}: swapped roles must not agree (tx)");
        assert_ne!(xrx, crx, "{label}: swapped roles must not agree with client rx");
        assert_ne!(xtx, ctx, "{label}: swapped roles must not agree with client tx");
    }
}

// ------------------------------------ configs 7.84–7.87 / errors 7.95–7.97

#[test]
fn kx_session_keys_null_and_aliased_outputs() {
    let client = Kx::new("crypto_kx_client_session_keys");
    let server = Kx::new("crypto_kx_server_session_keys");

    let mut rng = Rng::new(0x5EED_0003);
    for i in 0..12 {
        let (cpk, csk) = kx_pair(&rng.bytes(KX_SEED));
        let (spk, ssk) = kx_pair(&rng.bytes(KX_SEED));
        let untouched = pat(KX_SESSION);

        // ---- client
        let label = format!("7.84-7.87 client[{i}]");
        let (_, crx, ctx) = client.run(&label, Ptrs::Both, &cpk, &csk, &spk);
        // keys[0..32] == rx, keys[32..64] == tx for the client.

        // 7.84 / error 7.95: rx == NULL — `rx` is retargeted to `tx`, the loop
        // writes keys[0..32] then keys[32..64] into that one buffer byte by
        // byte, so the caller's `tx` ends up holding the (correct) tx key.
        let (rc, a, b) = client.run(&label, Ptrs::RxNull, &cpk, &csk, &spk);
        assert_eq!(rc, 0);
        eqb(&format!("{label} rx==NULL: unused buffer untouched"), &a, &untouched);
        eqb(&format!("{label} rx==NULL: tx holds keys[32..64]"), &b, &ctx);

        // 7.85 / error 7.96: tx == NULL — the FOOTGUN.  `tx` is retargeted to
        // `rx`, so the caller's `rx` buffer ends up holding keys[32..64], i.e.
        // the *tx* key, NOT the rx key.
        let (rc, a, b) = client.run(&label, Ptrs::TxNull, &cpk, &csk, &spk);
        assert_eq!(rc, 0);
        eqb(&format!("{label} tx==NULL: unused buffer untouched"), &b, &untouched);
        eqb(&format!("{label} tx==NULL: rx holds the TX key"), &a, &ctx);
        assert_ne!(a, crx, "{label} tx==NULL: rx must NOT hold the rx key");

        // 7.87: rx == tx (two non-NULL equal pointers) — same interleaving.
        let (rc, a, b) = client.run(&label, Ptrs::Aliased, &cpk, &csk, &spk);
        assert_eq!(rc, 0);
        eqb(&format!("{label} rx==tx: unused buffer untouched"), &b, &untouched);
        eqb(&format!("{label} rx==tx: buffer holds keys[32..64]"), &a, &ctx);

        // ---- server: the loop order is reversed (`tx[i]=keys[i]` first), so
        // the surviving buffer holds keys[32..64] = the server's RX key.
        let label = format!("7.86-7.87 server[{i}]");
        let (_, srx, stx) = server.run(&label, Ptrs::Both, &spk, &ssk, &cpk);

        let (rc, a, b) = server.run(&label, Ptrs::RxNull, &spk, &ssk, &cpk);
        assert_eq!(rc, 0);
        eqb(&format!("{label} rx==NULL: unused buffer untouched"), &a, &untouched);
        eqb(&format!("{label} rx==NULL: buffer holds the server RX key"), &b, &srx);
        assert_ne!(b, stx, "{label} rx==NULL: must NOT hold the tx key");

        let (rc, a, b) = server.run(&label, Ptrs::TxNull, &spk, &ssk, &cpk);
        assert_eq!(rc, 0);
        eqb(&format!("{label} tx==NULL: unused buffer untouched"), &b, &untouched);
        eqb(&format!("{label} tx==NULL: buffer holds the server RX key"), &a, &srx);

        let (rc, a, b) = server.run(&label, Ptrs::Aliased, &spk, &ssk, &cpk);
        assert_eq!(rc, 0);
        eqb(&format!("{label} rx==tx: unused buffer untouched"), &b, &untouched);
        eqb(&format!("{label} rx==tx: buffer holds keys[32..64]"), &a, &srx);
    }
}

// ------------------------------------------------------------- error 7.94

#[test]
fn kx_session_keys_both_null_aborts() {
    let (cc, cr) = both::<Session>("crypto_kx_client_session_keys");
    let (sc, sr) = both::<Session>("crypto_kx_server_session_keys");
    let (pk, sk) = kx_pair(&[7u8; KX_SEED]);
    let (peer, _) = kx_pair(&[9u8; KX_SEED]);

    // Raw fn pointers (Symbol derefs to a Copy fn pointer) so that the abort
    // closures do not have to borrow the Symbols.
    let fns: [(&str, Session, Session); 2] = [
        ("7.94 client rx==tx==NULL", *cc, *cr),
        ("7.94 server rx==tx==NULL", *sc, *sr),
    ];

    // `rx = tx` (NULL) then `tx = rx` (NULL) then `if (rx == NULL)
    // sodium_misuse()` → the misuse handler and then abort().
    for (what, c, r) in fns {
        let (pk2, sk2, peer2) = (pk.clone(), sk.clone(), peer.clone());
        let (pk3, sk3, peer3) = (pk.clone(), sk.clone(), peer.clone());
        eq_abort(
            what,
            move || unsafe {
                c(
                    ptr::null_mut(),
                    ptr::null_mut(),
                    pk2.as_ptr(),
                    sk2.as_ptr(),
                    peer2.as_ptr(),
                );
            },
            move || unsafe {
                r(
                    ptr::null_mut(),
                    ptr::null_mut(),
                    pk3.as_ptr(),
                    sk3.as_ptr(),
                    peer3.as_ptr(),
                );
            },
        );
    }
}

// -------------------------------------------------------- errors 7.92 / 7.93

#[test]
fn kx_session_keys_bad_peer_public_key() {
    let client = Kx::new("crypto_kx_client_session_keys");
    let server = Kx::new("crypto_kx_server_session_keys");
    let (pk, sk) = kx_pair(&[0x11u8; KX_SEED]);
    let untouched = pat(KX_SESSION);

    // The 7 blocklisted small-order encodings, all-zero and all-0xff.
    let mut peers: Vec<(String, Vec<u8>)> = SMALL_ORDER
        .iter()
        .enumerate()
        .map(|(i, s)| (format!("small-order[{i}]"), hx(s)))
        .collect();
    peers.push(("all-zero".into(), vec![0u8; KX_PK]));
    peers.push(("all-0xff".into(), vec![0xffu8; KX_PK]));
    // Same blocklisted encodings with the (ignored) high bit set: the guard
    // masks `s[31] & 0x7f`, so these must be rejected too.
    for (i, s) in SMALL_ORDER.iter().enumerate() {
        let mut v = hx(s);
        v[31] |= 0x80;
        peers.push((format!("small-order[{i}]|highbit"), v));
    }

    for (name, peer) in &peers {
        for kx in [&client, &server] {
            for p in [Ptrs::Both, Ptrs::RxNull, Ptrs::TxNull, Ptrs::Aliased] {
                let label = format!("7.92/7.93 {name}");
                let (rc, a, b) = kx.run(&label, p, &pk, &sk, peer);
                if rc == 0 {
                    // all-0xff is *not* blocklisted; it reduces to a valid
                    // x-coordinate, so success is legal.  What matters is that
                    // C and Rust agree, which `run` already asserted.
                    continue;
                }
                assert_eq!(rc, -1, "{label}: only -1 is expected on failure");
                // The write loop is after the guard: both buffers untouched.
                eqb(&format!("{label} {p:?}: rx untouched on failure"), &a, &untouched);
                eqb(&format!("{label} {p:?}: tx untouched on failure"), &b, &untouched);
            }
        }
    }

    // Every one of the 7 blocklist entries (and its high-bit variant) must
    // actually be rejected, otherwise this test proves nothing.
    for (i, s) in SMALL_ORDER.iter().enumerate() {
        let (rc, _, _) = client.run("blocklist", Ptrs::Both, &pk, &sk, &hx(s));
        assert_eq!(rc, -1, "small-order[{i}] must be rejected");
        let mut v = hx(s);
        v[31] |= 0x80;
        let (rc, _, _) = client.run("blocklist|highbit", Ptrs::Both, &pk, &sk, &v);
        assert_eq!(rc, -1, "small-order[{i}]|highbit must be rejected too");
        let (rc, _, _) = server.run("blocklist", Ptrs::Both, &pk, &sk, &hx(s));
        assert_eq!(rc, -1, "server small-order[{i}] must be rejected");
    }
}

// ===========================================================================
//                          crypto_kdf (blake2b)
// ===========================================================================

const KDF_MIN: usize = 16;
const KDF_MAX: usize = 64;
const KDF_CTX: usize = 8;
const KDF_KEY: usize = 32;

struct KdfPair {
    name: &'static str,
    c: Symbol<'static, Derive>,
    r: Symbol<'static, Derive>,
}

impl KdfPair {
    fn new(name: &'static str) -> Self {
        let (c, r) = both::<Derive>(name);
        KdfPair { name, c, r }
    }

    /// Returns `(rc, errno, subkey_bytes)` after asserting C/Rust agreement on
    /// all three (plus the guard pattern and the untouched-on-failure state).
    fn run(
        &self,
        label: &str,
        subkey_len: usize,
        buf_len: usize,
        subkey_id: u64,
        ctx: &[u8],
        key: &[u8],
    ) -> (c_int, c_int, Vec<u8>) {
        assert_eq!(ctx.len(), KDF_CTX);
        assert_eq!(key.len(), KDF_KEY);
        let mut oc = prefilled(buf_len);
        let mut or = prefilled(buf_len);
        set_errno(SENT);
        let rc = unsafe {
            (self.c)(
                oc.as_mut_ptr(),
                subkey_len,
                subkey_id,
                ctx.as_ptr() as *const c_char,
                key.as_ptr(),
            )
        };
        let ec = errno();
        set_errno(SENT);
        let rr = unsafe {
            (self.r)(
                or.as_mut_ptr(),
                subkey_len,
                subkey_id,
                ctx.as_ptr() as *const c_char,
                key.as_ptr(),
            )
        };
        let er = errno();
        let w = format!("{} [{label}] len={subkey_len} id={subkey_id}", self.name);
        eqi(&format!("{w} rc"), rc, rr);
        assert_eq!(ec, er, "{w}: errno mismatch (C {ec}, Rust {er})");
        eqb(&format!("{w} subkey"), &oc, &or);
        check_pad(&format!("{w} subkey(C)"), &oc, buf_len);
        check_pad(&format!("{w} subkey(Rust)"), &or, buf_len);
        if rc != 0 {
            // errors 7.99–7.101: `subkey` is untouched on rejection.
            eqb(&format!("{w}: subkey untouched on failure"), &oc[..buf_len], &pat(buf_len));
        }
        (rc, ec, oc[..buf_len].to_vec())
    }
}

// ------------------------------------------------- config 7.96 / error 7.109

#[test]
fn kdf_accessors() {
    size_eq("crypto_kdf_bytes_min", 16);
    size_eq("crypto_kdf_bytes_max", 64);
    size_eq("crypto_kdf_contextbytes", 8);
    size_eq("crypto_kdf_keybytes", 32);
    str_eq("crypto_kdf_primitive", "blake2b");

    size_eq("crypto_kdf_blake2b_bytes_min", 16);
    size_eq("crypto_kdf_blake2b_bytes_max", 64);
    size_eq("crypto_kdf_blake2b_contextbytes", 8);
    size_eq("crypto_kdf_blake2b_keybytes", 32);

    let (c, r) = both::<StrFn>("crypto_kdf_primitive");
    unsafe {
        assert_eq!(c(), c(), "C crypto_kdf_primitive is not stable");
        assert_eq!(r(), r(), "Rust crypto_kdf_primitive is not stable");
    }
}

// ------------------------------------ configs 7.90/7.91 / errors 7.99–7.102

#[test]
fn kdf_derive_subkey_lengths() {
    let generic = KdfPair::new("crypto_kdf_derive_from_key");
    let blake2b = KdfPair::new("crypto_kdf_blake2b_derive_from_key");
    let ctx = *b"context1";
    let key: Vec<u8> = (0..KDF_KEY as u8).map(|i| i.wrapping_mul(3).wrapping_add(1)).collect();

    // The buffer is always KDF_MAX+PAD long so that a rogue write past
    // `subkey_len` is caught by `check_pad`/the untouched-prefix comparison.
    let buf = KDF_MAX;

    // errors 7.99 / 7.101: subkey_len < BYTES_MIN → -1, errno = EINVAL.
    for len in 0..KDF_MIN {
        for k in [&generic, &blake2b] {
            let (rc, ec, _) = k.run("7.99 below MIN", len, buf, 0, &ctx, &key);
            assert_eq!(rc, -1, "7.99 subkey_len={len} must be rejected");
            assert_eq!(ec, EINVAL, "7.99 subkey_len={len}: exact sentinel errno");
        }
    }

    // errors 7.100 / 7.101: subkey_len > BYTES_MAX → -1, errno = EINVAL.
    for len in [
        KDF_MAX + 1,
        KDF_MAX + 2,
        65,
        100,
        1000,
        usize::MAX / 2,
        usize::MAX - 1,
        usize::MAX,
    ] {
        for k in [&generic, &blake2b] {
            let (rc, ec, _) = k.run("7.100 above MAX", len, buf, 0, &ctx, &key);
            assert_eq!(rc, -1, "7.100 subkey_len={len} must be rejected");
            assert_eq!(ec, EINVAL, "7.100 subkey_len={len}: exact sentinel errno");
        }
    }

    // config 7.91 / error 7.102: every legal length succeeds, the generic
    // alias is byte-identical, and a shorter subkey is NOT a prefix of a
    // longer one (BLAKE2b puts `outlen` in the parameter block).
    let mut outs: Vec<(usize, Vec<u8>)> = Vec::new();
    for len in KDF_MIN..=KDF_MAX {
        let (rc, _, ob) = blake2b.run("7.91 legal", len, buf, 0, &ctx, &key);
        assert_eq!(rc, 0, "7.91 subkey_len={len} must succeed");
        let (rcg, _, og) = generic.run("7.90 alias", len, buf, 0, &ctx, &key);
        assert_eq!(rcg, 0);
        eqb(
            &format!("7.90 generic alias == blake2b (len={len})"),
            &ob,
            &og,
        );
        // Bytes past `subkey_len` must still carry the prefill pattern.
        eqb(
            &format!("7.91 no write past subkey_len={len}"),
            &ob[len..],
            &pat(buf)[len..],
        );
        outs.push((len, ob[..len].to_vec()));
    }
    for (i, (la, a)) in outs.iter().enumerate() {
        for (lb, b) in outs.iter().skip(i + 1) {
            assert!(
                !b.starts_with(a),
                "7.91: subkey({la}) is a prefix of subkey({lb}) — outlen must be domain-separated"
            );
        }
    }
}

// -------------------------------------------- configs 7.92–7.94 (kdf inputs)

#[test]
fn kdf_derive_subkey_ids_contexts_keys() {
    let generic = KdfPair::new("crypto_kdf_derive_from_key");
    let blake2b = KdfPair::new("crypto_kdf_blake2b_derive_from_key");

    let ctx = *b"context1";
    let key: Vec<u8> = (0..KDF_KEY as u8).map(|i| i ^ 0x5a).collect();

    // config 7.92: subkey_id boundaries, STORE64_LE into salt[0..8].
    let mut ids: Vec<u64> = vec![
        0,
        1,
        2,
        255,
        256,
        0xffff_ffff,
        0x1_0000_0000,
        0x8000_0000_0000_0000,
        u64::MAX,
    ];
    let mut rng = Rng::new(0x5EED_0010);
    for _ in 0..32 {
        ids.push(rng.next_u64());
    }
    let mut seen: Vec<Vec<u8>> = Vec::new();
    for id in &ids {
        let (rc, _, ob) = blake2b.run("7.92 subkey_id", 32, 32, *id, &ctx, &key);
        assert_eq!(rc, 0);
        let (rcg, _, og) = generic.run("7.92 subkey_id", 32, 32, *id, &ctx, &key);
        assert_eq!(rcg, 0);
        eqb(&format!("7.92 alias id={id}"), &ob, &og);
        assert!(!seen.contains(&ob), "7.92: subkey_id={id} collides");
        seen.push(ob);
    }

    // config 7.93: 8-byte contexts, including all-zero and all-0xff.
    let mut ctxs: Vec<Vec<u8>> = vec![
        vec![0u8; KDF_CTX],
        b"12345678".to_vec(),
        vec![0xffu8; KDF_CTX],
    ];
    for _ in 0..32 {
        ctxs.push(rng.bytes(KDF_CTX));
    }
    let mut seen: Vec<Vec<u8>> = Vec::new();
    for (i, c) in ctxs.iter().enumerate() {
        let (rc, _, ob) = blake2b.run(&format!("7.93 ctx[{i}]"), 40, 40, 1, c, &key);
        assert_eq!(rc, 0);
        let (rcg, _, og) = generic.run(&format!("7.93 ctx[{i}]"), 40, 40, 1, c, &key);
        assert_eq!(rcg, 0);
        eqb(&format!("7.93 alias ctx[{i}]"), &ob, &og);
        assert!(!seen.contains(&ob), "7.93: ctx[{i}] collides");
        seen.push(ob);
    }

    // config 7.94: keys, including all-zero and all-0xff.
    let mut keys: Vec<Vec<u8>> = vec![vec![0u8; KDF_KEY], vec![0xffu8; KDF_KEY]];
    for _ in 0..48 {
        keys.push(rng.bytes(KDF_KEY));
    }
    let mut seen: Vec<Vec<u8>> = Vec::new();
    for (i, k) in keys.iter().enumerate() {
        let len = KDF_MIN + (i % (KDF_MAX - KDF_MIN + 1));
        let (rc, _, ob) = blake2b.run(&format!("7.94 key[{i}]"), len, KDF_MAX, 7, &ctx, k);
        assert_eq!(rc, 0);
        let (rcg, _, og) = generic.run(&format!("7.94 key[{i}]"), len, KDF_MAX, 7, &ctx, k);
        assert_eq!(rcg, 0);
        eqb(&format!("7.94 alias key[{i}]"), &ob, &og);
        assert!(!seen.contains(&ob), "7.94: key[{i}] collides");
        seen.push(ob);
    }

    // Fully randomised sweep across all four axes at once.
    let mut rng = Rng::new(0x5EED_0011);
    for i in 0..256 {
        let len = rng.range(0, 80); // includes illegal lengths
        let id = rng.next_u64();
        let c = rng.bytes(KDF_CTX);
        let k = rng.bytes(KDF_KEY);
        let (rc, ec, _) = blake2b.run(&format!("random[{i}]"), len, 80, id, &c, &k);
        if (KDF_MIN..=KDF_MAX).contains(&len) {
            assert_eq!(rc, 0, "random[{i}] len={len} must succeed");
        } else {
            assert_eq!(rc, -1, "random[{i}] len={len} must fail");
            assert_eq!(ec, EINVAL, "random[{i}] len={len}: errno");
        }
        let (rcg, ecg, _) = generic.run(&format!("random[{i}] alias"), len, 80, id, &c, &k);
        assert_eq!(rcg, rc);
        assert_eq!(ecg, ec);
    }
}

// ------------------------------------------- config 7.95 / error 7.108 (kdf)

#[test]
fn kdf_keygen() {
    let (c, r) = both::<Keygen>("crypto_kdf_keygen");
    let mut prev: Option<Vec<u8>> = None;
    for i in 0..32u64 {
        rng_reseed(0x2000 + i);
        let mut kc = prefilled(KDF_KEY);
        let mut kr = prefilled(KDF_KEY);
        unsafe { c(kc.as_mut_ptr()) };
        unsafe { r(kr.as_mut_ptr()) };
        eqb(&format!("7.95 keygen[{i}]"), &kc, &kr);
        check_pad(&format!("7.95 keygen[{i}](C)"), &kc, KDF_KEY);
        check_pad(&format!("7.95 keygen[{i}](Rust)"), &kr, KDF_KEY);
        if let Some(p) = &prev {
            assert_ne!(p, &kc[..KDF_KEY].to_vec(), "7.95: successive keys identical");
        }
        prev = Some(kc[..KDF_KEY].to_vec());
    }

    // Two successive calls on the *same* stream must also differ.
    rng_reset();
    let mut a = vec![0u8; KDF_KEY];
    let mut b = vec![0u8; KDF_KEY];
    unsafe {
        c(a.as_mut_ptr());
        c(b.as_mut_ptr());
    }
    assert_ne!(a, b, "7.95: successive crypto_kdf_keygen outputs identical");
}

// ===========================================================================
//                        crypto_kdf_hkdf_sha{256,512}
// ===========================================================================

struct Hkdf {
    name: &'static str,
    keybytes: usize,
    bytes_max: usize,
    statebytes: usize,
    c_extract: Symbol<'static, Extract>,
    r_extract: Symbol<'static, Extract>,
    c_expand: Symbol<'static, Expand>,
    r_expand: Symbol<'static, Expand>,
    c_init: Symbol<'static, ExInit>,
    r_init: Symbol<'static, ExInit>,
    c_update: Symbol<'static, ExUpdate>,
    r_update: Symbol<'static, ExUpdate>,
    c_final: Symbol<'static, ExFinal>,
    r_final: Symbol<'static, ExFinal>,
    c_keygen: Symbol<'static, Keygen>,
    r_keygen: Symbol<'static, Keygen>,
}

impl Hkdf {
    fn new(name: &'static str, keybytes: usize) -> Self {
        let f = |s: &str| format!("{name}_{s}");
        let (cs, rs) = both::<SizeFn>(&f("statebytes"));
        let sb = unsafe { cs() };
        assert_eq!(sb, unsafe { rs() }, "{name}_statebytes mismatch");
        let (c_extract, r_extract) = both::<Extract>(&f("extract"));
        let (c_expand, r_expand) = both::<Expand>(&f("expand"));
        let (c_init, r_init) = both::<ExInit>(&f("extract_init"));
        let (c_update, r_update) = both::<ExUpdate>(&f("extract_update"));
        let (c_final, r_final) = both::<ExFinal>(&f("extract_final"));
        let (c_keygen, r_keygen) = both::<Keygen>(&f("keygen"));
        Hkdf {
            name,
            keybytes,
            bytes_max: 0xff * keybytes,
            statebytes: sb,
            c_extract,
            r_extract,
            c_expand,
            r_expand,
            c_init,
            r_init,
            c_update,
            r_update,
            c_final,
            r_final,
            c_keygen,
            r_keygen,
        }
    }

    /// One-shot `_extract`, differentially.  `salt == None` means a NULL salt
    /// pointer with `salt_len == 0` (the legal `key == NULL, keylen == 0`
    /// branch of `crypto_auth_hmacsha*_init`).
    fn extract(&self, label: &str, salt: Option<&[u8]>, ikm: &[u8]) -> Vec<u8> {
        let (sp, sl) = match salt {
            None => (ptr::null(), 0usize),
            Some(s) => (s.as_ptr(), s.len()),
        };
        let mut pc = prefilled(self.keybytes);
        let mut pr = prefilled(self.keybytes);
        let rc = unsafe { (self.c_extract)(pc.as_mut_ptr(), sp, sl, ikm.as_ptr(), ikm.len()) };
        let rr = unsafe { (self.r_extract)(pr.as_mut_ptr(), sp, sl, ikm.as_ptr(), ikm.len()) };
        let w = format!("{}_extract [{label}] salt={sl} ikm={}", self.name, ikm.len());
        eqi(&format!("{w} rc"), rc, rr);
        assert_eq!(rc, 0, "{w}: error row 7.107 — extract cannot fail");
        eqb(&format!("{w} prk"), &pc, &pr);
        check_pad(&format!("{w} prk(C)"), &pc, self.keybytes);
        check_pad(&format!("{w} prk(Rust)"), &pr, self.keybytes);
        pc[..self.keybytes].to_vec()
    }

    /// Streaming `_extract_init` + N × `_extract_update` + `_extract_final`.
    ///
    /// The FULL opaque state is compared between C and Rust after `init` and
    /// after every single `update`, and again after `final` (where the C
    /// `sodium_memzero`s it).
    fn extract_streaming(&self, label: &str, salt: Option<&[u8]>, chunks: &[&[u8]]) -> Vec<u8> {
        let (sp, sl) = match salt {
            None => (ptr::null(), 0usize),
            Some(s) => (s.as_ptr(), s.len()),
        };
        let mut sc = SBuf::new(self.statebytes);
        let mut sr = SBuf::new(self.statebytes);
        let w = format!("{}_extract_init/update/final [{label}]", self.name);

        let rc = unsafe { (self.c_init)(sc.ptr(), sp, sl) };
        let rr = unsafe { (self.r_init)(sr.ptr(), sp, sl) };
        eqi(&format!("{w} init rc"), rc, rr);
        assert_eq!(rc, 0);
        eqb(&format!("{w} state after init"), sc.st(), sr.st());
        sc.check(&format!("{w} state after init(C)"));
        sr.check(&format!("{w} state after init(Rust)"));

        for (i, ch) in chunks.iter().enumerate() {
            let rc = unsafe { (self.c_update)(sc.ptr(), ch.as_ptr(), ch.len()) };
            let rr = unsafe { (self.r_update)(sr.ptr(), ch.as_ptr(), ch.len()) };
            eqi(&format!("{w} update[{i}] rc"), rc, rr);
            assert_eq!(rc, 0);
            eqb(
                &format!("{w} state after update[{i}] (len {})", ch.len()),
                sc.st(),
                sr.st(),
            );
            sc.check(&format!("{w} state after update[{i}](C)"));
            sr.check(&format!("{w} state after update[{i}](Rust)"));
        }

        let mut pc = prefilled(self.keybytes);
        let mut pr = prefilled(self.keybytes);
        let rc = unsafe { (self.c_final)(sc.ptr(), pc.as_mut_ptr()) };
        let rr = unsafe { (self.r_final)(sr.ptr(), pr.as_mut_ptr()) };
        eqi(&format!("{w} final rc"), rc, rr);
        assert_eq!(rc, 0);
        eqb(&format!("{w} prk"), &pc, &pr);
        check_pad(&format!("{w} prk(C)"), &pc, self.keybytes);
        check_pad(&format!("{w} prk(Rust)"), &pr, self.keybytes);
        // error row 7.103/7.107: `_extract_final` wipes the state.
        eqb(&format!("{w} state after final"), sc.st(), sr.st());
        assert!(
            sc.st().iter().all(|b| *b == 0),
            "{w}: C did not sodium_memzero the state"
        );
        assert!(
            sr.st().iter().all(|b| *b == 0),
            "{w}: Rust did not sodium_memzero the state"
        );
        sc.check(&format!("{w} state after final(C)"));
        sr.check(&format!("{w} state after final(Rust)"));
        pc[..self.keybytes].to_vec()
    }

    /// `_expand`, differentially.  Returns `(rc, errno, out)`.
    fn expand(
        &self,
        label: &str,
        out_len: usize,
        buf_len: usize,
        ctx: Option<&[u8]>,
        prk: &[u8],
    ) -> (c_int, c_int, Vec<u8>) {
        assert_eq!(prk.len(), self.keybytes);
        let (cp, cl) = match ctx {
            None => (ptr::null(), 0usize),
            Some(c) => (c.as_ptr() as *const c_char, c.len()),
        };
        let mut oc = prefilled(buf_len);
        let mut or = prefilled(buf_len);
        set_errno(SENT);
        let rc = unsafe { (self.c_expand)(oc.as_mut_ptr(), out_len, cp, cl, prk.as_ptr()) };
        let ec = errno();
        set_errno(SENT);
        let rr = unsafe { (self.r_expand)(or.as_mut_ptr(), out_len, cp, cl, prk.as_ptr()) };
        let er = errno();
        let w = format!("{}_expand [{label}] out_len={out_len} ctx_len={cl}", self.name);
        eqi(&format!("{w} rc"), rc, rr);
        assert_eq!(ec, er, "{w}: errno mismatch (C {ec}, Rust {er})");
        eqb(&format!("{w} out"), &oc, &or);
        check_pad(&format!("{w} out(C)"), &oc, buf_len);
        check_pad(&format!("{w} out(Rust)"), &or, buf_len);
        if rc != 0 {
            eqb(&format!("{w}: out untouched on failure"), &oc[..buf_len], &pat(buf_len));
        } else {
            // error row 7.106: never write past `out_len`.
            eqb(
                &format!("{w}: no write past out_len"),
                &oc[out_len..buf_len],
                &pat(buf_len)[out_len..],
            );
        }
        (rc, ec, oc[..out_len.min(buf_len)].to_vec())
    }
}

fn hkdf256() -> Hkdf {
    Hkdf::new("crypto_kdf_hkdf_sha256", 32)
}
fn hkdf512() -> Hkdf {
    Hkdf::new("crypto_kdf_hkdf_sha512", 64)
}

// ------------------------------------------------- config 7.112 / error 7.109

#[test]
fn hkdf_accessors() {
    size_eq("crypto_kdf_hkdf_sha256_keybytes", 32);
    size_eq("crypto_kdf_hkdf_sha256_bytes_min", 0);
    size_eq("crypto_kdf_hkdf_sha256_bytes_max", 0xff * 32);
    size_eq("crypto_kdf_hkdf_sha512_keybytes", 64);
    size_eq("crypto_kdf_hkdf_sha512_bytes_min", 0);
    size_eq("crypto_kdf_hkdf_sha512_bytes_max", 0xff * 64);

    // `sizeof(crypto_kdf_hkdf_sha*_state)` == 2 × sizeof(the sha state).
    size_eq("crypto_kdf_hkdf_sha256_statebytes", 2 * 104);
    size_eq("crypto_kdf_hkdf_sha512_statebytes", 2 * 208);
}

// ------------------------------------------ configs 7.97/7.98 (RFC 5869)

#[test]
fn hkdf_sha256_rfc5869_vectors() {
    let h = hkdf256();

    // Test case 1.
    let ikm = vec![0x0bu8; 22];
    let salt = hx("000102030405060708090a0b0c");
    let info = hx("f0f1f2f3f4f5f6f7f8f9");
    let prk = h.extract("RFC5869 #1", Some(&salt), &ikm);
    eqb(
        "7.97 RFC5869 #1 prk",
        &hx("077709362c2e32df0ddc3f0dc47bba6390b6c73bb50f9c3122ec844ad7c2b3e5"),
        &prk,
    );
    let (rc, _, okm) = h.expand("RFC5869 #1", 42, 96, Some(&info), &prk);
    assert_eq!(rc, 0);
    eqb(
        "7.98 RFC5869 #1 okm",
        &hx("3cb25f25faacd57a90434f64d0362f2a2d2d0a90cf1a5a4c5db02d56ecc4c5bf34007208d5b887185865"),
        &okm,
    );

    // Test case 3: empty salt, empty info.
    let prk3 = h.extract("RFC5869 #3", Some(&[]), &ikm);
    eqb(
        "7.99 RFC5869 #3 prk (salt_len == 0)",
        &hx("19ef24a32c717b167f33a91d6f648bdf96596776afdb6377ac434c1c293ccb04"),
        &prk3,
    );
    let (rc, _, okm3) = h.expand("RFC5869 #3", 42, 96, Some(&[]), &prk3);
    assert_eq!(rc, 0);
    eqb(
        "7.98 RFC5869 #3 okm",
        &hx("8da4e775a563c18f715f802a063c5a31b8a11f5c5ee1879ec3454e5f3c738d2d9d201395faa4b61a96c8"),
        &okm3,
    );

    // A NULL salt with salt_len == 0 must behave exactly like an empty salt.
    let prk_null = h.extract("NULL salt", None, &ikm);
    eqb("NULL salt == empty salt", &prk3, &prk_null);
    // Likewise a NULL ctx with ctx_len == 0 (config 7.105).
    let (rc, _, okm_null) = h.expand("NULL ctx", 42, 96, None, &prk3);
    assert_eq!(rc, 0);
    eqb("NULL ctx == empty ctx", &okm3, &okm_null);
}

// ------------------------------- configs 7.99/7.100/7.108 / error 7.107

fn hkdf_extract_matrix(h: &Hkdf, seed: u64) {
    let mut rng = Rng::new(seed);
    let salt_lens: Vec<usize> = vec![0, 1, 2, 13, 31, 32, 63, 64, 65, 80, 127, 128, 129, 200, 1000];
    let ikm_lens: Vec<usize> = vec![0, 1, 22, 31, 32, 63, 64, 65, 80, 127, 128, 129, 1000];

    let mut seen: Vec<Vec<u8>> = Vec::new();
    for sl in &salt_lens {
        for il in &ikm_lens {
            let salt = rng.bytes(*sl);
            let ikm = rng.bytes(*il);
            let one = h.extract(&format!("matrix s={sl} i={il}"), Some(&salt), &ikm);
            assert_eq!(one.len(), h.keybytes);

            // Streaming with 0 / 1 / many chunks must reproduce the one-shot.
            if *il == 0 {
                let z = h.extract_streaming(&format!("0 updates s={sl}"), Some(&salt), &[]);
                eqb("7.101 zero updates == one-shot with ikm_len 0", &one, &z);
            }
            let s1 = h.extract_streaming(&format!("1 update s={sl} i={il}"), Some(&salt), &[&ikm]);
            eqb("7.102 one update == one-shot", &one, &s1);

            // Randomised chunking.
            let mut cuts: Vec<usize> = Vec::new();
            let mut pos = 0usize;
            while pos < ikm.len() {
                let k = rng.range(0, (ikm.len() - pos).min(70));
                pos += k;
                cuts.push(pos);
                if k == 0 && cuts.len() > 12 {
                    break;
                }
            }
            if cuts.last().copied() != Some(ikm.len()) {
                cuts.push(ikm.len());
            }
            let mut chunks: Vec<&[u8]> = Vec::new();
            let mut prev = 0usize;
            for c in &cuts {
                chunks.push(&ikm[prev..*c]);
                prev = *c;
            }
            let sn =
                h.extract_streaming(&format!("{} updates s={sl} i={il}", chunks.len()), Some(&salt), &chunks);
            eqb("7.102 many updates == one-shot", &one, &sn);

            if !seen.contains(&one) {
                seen.push(one);
            }
        }
    }
    assert!(
        seen.len() > salt_lens.len(),
        "{}: extract outputs suspiciously collide",
        h.name
    );

    // Fixed splittings of a 128-byte ikm: 1+1+…, 31+1+32, 32×N, 63+1.
    let ikm = rng.bytes(128);
    let salt = rng.bytes(37);
    let one = h.extract("split base", Some(&salt), &ikm);
    let splittings: Vec<Vec<usize>> = vec![
        vec![1; 128],
        vec![31, 1, 32, 64],
        vec![32, 32, 32, 32],
        vec![63, 1, 64],
        vec![0, 128, 0],
        vec![127, 1],
        vec![64, 0, 64],
        vec![128],
    ];
    for (i, sp) in splittings.iter().enumerate() {
        assert_eq!(sp.iter().sum::<usize>(), 128);
        let mut chunks: Vec<&[u8]> = Vec::new();
        let mut pos = 0usize;
        for k in sp {
            chunks.push(&ikm[pos..pos + k]);
            pos += k;
        }
        let got = h.extract_streaming(&format!("splitting[{i}]"), Some(&salt), &chunks);
        eqb(&format!("7.102 splitting[{i}] == one-shot"), &one, &got);
    }
}

#[test]
fn hkdf_sha256_extract_matrix() {
    hkdf_extract_matrix(&hkdf256(), 0x5EED_0100);
}

#[test]
fn hkdf_sha512_extract_matrix() {
    hkdf_extract_matrix(&hkdf512(), 0x5EED_0101);
}

// -------------------------------------------------- configs 7.101/7.102/7.109
//
// `extract_init` + N × `extract_update` + `extract_final` must equal the
// one-shot `_extract` *within each library separately* (the matrix test above
// only establishes it transitively through the C/Rust equality).

fn hkdf_streaming_equals_oneshot(h: &Hkdf, seed: u64) {
    let mut rng = Rng::new(seed);
    for i in 0..64 {
        let sl = rng.range(0, 200);
        let salt = rng.bytes(sl);
        let il = rng.range(0, 400);
        let ikm = rng.bytes(il);
        let nchunks = rng.range(0, 9);

        // Random chunk boundaries covering the whole ikm (0 chunks only when
        // ikm is empty, which is exactly config row 7.101).
        let mut bounds: Vec<usize> = (0..nchunks).map(|_| rng.below(ikm.len() + 1)).collect();
        bounds.push(ikm.len());
        bounds.sort_unstable();
        let mut chunks: Vec<&[u8]> = Vec::new();
        let mut prev = 0usize;
        for b in &bounds {
            chunks.push(&ikm[prev..*b]);
            prev = *b;
        }
        if ikm.is_empty() && i % 3 == 0 {
            chunks.clear(); // zero `_extract_update` calls at all
        }

        for (which, ex, ini, upd, fin) in [
            ("C", &h.c_extract, &h.c_init, &h.c_update, &h.c_final),
            ("Rust", &h.r_extract, &h.r_init, &h.r_update, &h.r_final),
        ] {
            let mut one = prefilled(h.keybytes);
            let mut str_ = prefilled(h.keybytes);
            let mut st = SBuf::new(h.statebytes);
            unsafe {
                assert_eq!(
                    ex(one.as_mut_ptr(), salt.as_ptr(), salt.len(), ikm.as_ptr(), ikm.len()),
                    0
                );
                assert_eq!(ini(st.ptr(), salt.as_ptr(), salt.len()), 0);
                for c in &chunks {
                    assert_eq!(upd(st.ptr(), c.as_ptr(), c.len()), 0);
                }
                assert_eq!(fin(st.ptr(), str_.as_mut_ptr()), 0);
            }
            eqb(
                &format!(
                    "{} [{which}] streaming({} chunks) == one-shot (salt={sl}, ikm={il})",
                    h.name,
                    chunks.len()
                ),
                &one,
                &str_,
            );
            st.check(&format!("{} [{which}] state guard", h.name));
        }
    }
}

#[test]
fn hkdf_sha256_streaming_equals_oneshot() {
    hkdf_streaming_equals_oneshot(&hkdf256(), 0x5EED_0500);
}

#[test]
fn hkdf_sha512_streaming_equals_oneshot() {
    hkdf_streaming_equals_oneshot(&hkdf512(), 0x5EED_0501);
}

// ------------------- configs 7.104–7.106 / errors 7.103–7.106 (expand)

fn hkdf_expand_matrix(h: &Hkdf, seed: u64) {
    let mut rng = Rng::new(seed);
    let prk = rng.bytes(h.keybytes);

    let mut lens: Vec<usize> = vec![0, 1, 2, 15, 16, 31, 32, 33, 63, 64, 65, 96, 127, 128, 129, 255, 256, 257];
    lens.push(h.keybytes - 1);
    lens.push(h.keybytes);
    lens.push(h.keybytes + 1);
    lens.push(h.bytes_max - 1);
    lens.push(h.bytes_max);
    lens.sort_unstable();
    lens.dedup();

    let ctx_lens: Vec<usize> = vec![0, 1, 8, 10, 64];

    for out_len in &lens {
        for cl in &ctx_lens {
            let ctx = rng.bytes(*cl);
            let (rc, ec, out) = h.expand(
                &format!("7.104 out_len={out_len}"),
                *out_len,
                out_len + 64,
                Some(&ctx),
                &prk,
            );
            assert_eq!(rc, 0, "out_len={out_len} <= BYTES_MAX must succeed");
            assert_eq!(ec, SENT, "success must not touch errno (out_len={out_len})");
            assert_eq!(out.len(), *out_len);
            // config 7.106: prefix property — expand(n) is a prefix of
            // expand(m) for n < m, because the counter/chaining is the same.
            if *out_len >= h.keybytes {
                let (_, _, shorter) = h.expand(
                    &format!("prefix out_len={}", h.keybytes),
                    h.keybytes,
                    h.keybytes + 64,
                    Some(&ctx),
                    &prk,
                );
                assert!(
                    out.starts_with(&shorter),
                    "{}: expand({out_len}) does not extend expand({})",
                    h.name,
                    h.keybytes
                );
            }
        }
    }

    // errors 7.103 / 7.104: out_len > BYTES_MAX → -1, errno = EINVAL, out
    // untouched.
    for out_len in [
        h.bytes_max + 1,
        h.bytes_max + 2,
        h.bytes_max * 2,
        usize::MAX / 2,
        usize::MAX - 1,
        usize::MAX,
    ] {
        let ctx = rng.bytes(9);
        let (rc, ec, _) = h.expand(
            &format!("7.103/7.104 out_len={out_len}"),
            out_len,
            0,
            Some(&ctx),
            &prk,
        );
        assert_eq!(rc, -1, "{}: out_len={out_len} must be rejected", h.name);
        assert_eq!(ec, EINVAL, "{}: out_len={out_len} exact sentinel errno", h.name);
    }

    // error 7.105: out_len == 0 is legal and writes nothing.
    let (rc, ec, out) = h.expand("7.105 out_len=0", 0, 64, Some(b"ctx"), &prk);
    assert_eq!(rc, 0);
    assert_eq!(ec, SENT);
    assert!(out.is_empty());

    // Randomised sweep over prk / ctx / out_len.
    let mut seen: Vec<Vec<u8>> = Vec::new();
    for i in 0..96 {
        let p = rng.bytes(h.keybytes);
        let cl = rng.range(0, 200);
        let ctx = rng.bytes(cl);
        let out_len = rng.range(0, 300);
        let (rc, _, out) = h.expand(&format!("random[{i}]"), out_len, out_len + 48, Some(&ctx), &p);
        assert_eq!(rc, 0);
        if out_len >= h.keybytes && !seen.contains(&out) {
            seen.push(out);
        }
    }
    assert!(seen.len() > 40, "{}: randomised expand outputs collide", h.name);
}

#[test]
fn hkdf_sha256_expand_matrix() {
    hkdf_expand_matrix(&hkdf256(), 0x5EED_0200);
}

#[test]
fn hkdf_sha512_expand_matrix() {
    hkdf_expand_matrix(&hkdf512(), 0x5EED_0201);
}

// ------------------------------------------------------------- config 7.106

#[test]
fn hkdf_expand_counter_exhaustion() {
    // out_len == BYTES_MAX drives the one-byte counter all the way to 0xff.
    // The last block must equal an independent HMAC(prk, T(254) || ctx || 255)
    // computed through the public `crypto_auth_hmacsha*` streaming API.
    for (h, init, update, fin) in [
        (
            hkdf256(),
            "crypto_auth_hmacsha256_init",
            "crypto_auth_hmacsha256_update",
            "crypto_auth_hmacsha256_final",
        ),
        (
            hkdf512(),
            "crypto_auth_hmacsha512_init",
            "crypto_auth_hmacsha512_update",
            "crypto_auth_hmacsha512_final",
        ),
    ] {
        let sbname = if h.keybytes == 32 {
            "crypto_auth_hmacsha256_statebytes"
        } else {
            "crypto_auth_hmacsha512_statebytes"
        };
        if !(has(init) && has(update) && has(fin) && has(sbname)) {
            continue;
        }
        let (ci, ri) = both::<ExInit>(init);
        let (cu, ru) = both::<unsafe extern "C" fn(*mut u8, *const u8, u64) -> c_int>(update);
        let (cf, rf) = both::<ExFinal>(fin);
        let (csb, rsb) = both::<SizeFn>(sbname);
        let sb = unsafe { csb() };
        assert_eq!(sb, unsafe { rsb() }, "{sbname} mismatch");

        let mut rng = Rng::new(0x5EED_0300 + h.keybytes as u64);
        let prk = rng.bytes(h.keybytes);
        let ctx = rng.bytes(11);
        let (rc, _, out) = h.expand("7.106 counter=0xff", h.bytes_max, h.bytes_max + 32, Some(&ctx), &prk);
        assert_eq!(rc, 0);
        assert_eq!(out.len(), 0xff * h.keybytes);

        let kb = h.keybytes;
        let prev = &out[(0xfe - 1) * kb..0xfe * kb]; // T(254)
        let counter: u8 = 0xff;
        for (which, i, u, f) in [("C", &ci, &cu, &cf), ("Rust", &ri, &ru, &rf)] {
            let mut st = SBuf::new(sb);
            let mut mac = prefilled(kb);
            unsafe {
                assert_eq!(i(st.ptr(), prk.as_ptr(), kb), 0);
                assert_eq!(u(st.ptr(), prev.as_ptr(), kb as u64), 0);
                assert_eq!(u(st.ptr(), ctx.as_ptr(), ctx.len() as u64), 0);
                assert_eq!(u(st.ptr(), &counter as *const u8, 1), 0);
                assert_eq!(f(st.ptr(), mac.as_mut_ptr()), 0);
            }
            eqb(
                &format!("7.106 {} {which}: last block == HMAC(.., counter=255)", h.name),
                &out[0xfe * kb..],
                &mac[..kb],
            );
            st.check(&format!("7.106 {} {which} hmac state", h.name));
        }
    }
}

// --------------------------------------------- configs 7.107 / 7.110 keygen

#[test]
fn hkdf_keygen() {
    for h in [hkdf256(), hkdf512()] {
        let mut prev: Option<Vec<u8>> = None;
        for i in 0..24u64 {
            rng_reseed(0x3000 + i + h.keybytes as u64 * 1000);
            let mut kc = prefilled(h.keybytes);
            let mut kr = prefilled(h.keybytes);
            unsafe { (h.c_keygen)(kc.as_mut_ptr()) };
            unsafe { (h.r_keygen)(kr.as_mut_ptr()) };
            eqb(&format!("{}_keygen[{i}]", h.name), &kc, &kr);
            check_pad(&format!("{}_keygen[{i}](C)", h.name), &kc, h.keybytes);
            check_pad(&format!("{}_keygen[{i}](Rust)", h.name), &kr, h.keybytes);
            if let Some(p) = &prev {
                assert_ne!(p, &kc[..h.keybytes].to_vec(), "{}_keygen: repeat", h.name);
            }
            prev = Some(kc[..h.keybytes].to_vec());

            // config 7.107: the freshly generated prk feeds `_expand`.
            let (rc, _, _) = h.expand("7.107 keygen -> expand", 100, 164, Some(b"ctx"), &kc[..h.keybytes]);
            assert_eq!(rc, 0);
        }
    }
}

// ------------------------------------------------------------- config 7.111

#[test]
fn hkdf_sha256_vs_sha512_namespaces() {
    let a = hkdf256();
    let b = hkdf512();
    let mut rng = Rng::new(0x5EED_0400);
    for i in 0..16 {
        let sl = rng.range(0, 200);
        let salt = rng.bytes(sl);
        let il = rng.range(0, 200);
        let ikm = rng.bytes(il);
        let pa = a.extract(&format!("cross[{i}]"), Some(&salt), &ikm);
        let pb = b.extract(&format!("cross[{i}]"), Some(&salt), &ikm);
        assert_eq!(pa.len(), 32);
        assert_eq!(pb.len(), 64);
        assert_ne!(pa, pb[..32].to_vec(), "7.111: sha256/sha512 prk prefixes collide");

        let ctx = rng.bytes(13);
        let mut prk_a = pa.clone();
        let mut prk_b = pb.clone();
        prk_a.resize(32, 0);
        prk_b.resize(64, 0);
        let (_, _, oa) = a.expand(&format!("cross[{i}]"), 64, 128, Some(&ctx), &prk_a);
        let (_, _, ob) = b.expand(&format!("cross[{i}]"), 64, 128, Some(&ctx), &prk_b);
        assert_ne!(oa, ob, "7.111: sha256/sha512 expand outputs collide");
    }
}

// ------------------------------------------------------ symbol completeness

#[test]
fn kx_kdf_symbols_all_exported() {
    // Every symbol exported by the C `.so` in this scope must also be exported
    // by the Rust `.so` (this is what `both()` asserts).
    for s in [
        "crypto_kx_keypair",
        "crypto_kx_seed_keypair",
        "crypto_kx_client_session_keys",
        "crypto_kx_server_session_keys",
        "crypto_kx_publickeybytes",
        "crypto_kx_secretkeybytes",
        "crypto_kx_seedbytes",
        "crypto_kx_sessionkeybytes",
        "crypto_kx_primitive",
        "crypto_kdf_primitive",
        "crypto_kdf_bytes_min",
        "crypto_kdf_bytes_max",
        "crypto_kdf_contextbytes",
        "crypto_kdf_keybytes",
        "crypto_kdf_derive_from_key",
        "crypto_kdf_keygen",
        "crypto_kdf_blake2b_bytes_min",
        "crypto_kdf_blake2b_bytes_max",
        "crypto_kdf_blake2b_contextbytes",
        "crypto_kdf_blake2b_keybytes",
        "crypto_kdf_blake2b_derive_from_key",
        "crypto_kdf_hkdf_sha256_keybytes",
        "crypto_kdf_hkdf_sha256_bytes_min",
        "crypto_kdf_hkdf_sha256_bytes_max",
        "crypto_kdf_hkdf_sha256_statebytes",
        "crypto_kdf_hkdf_sha256_extract",
        "crypto_kdf_hkdf_sha256_extract_init",
        "crypto_kdf_hkdf_sha256_extract_update",
        "crypto_kdf_hkdf_sha256_extract_final",
        "crypto_kdf_hkdf_sha256_expand",
        "crypto_kdf_hkdf_sha256_keygen",
        "crypto_kdf_hkdf_sha512_keybytes",
        "crypto_kdf_hkdf_sha512_bytes_min",
        "crypto_kdf_hkdf_sha512_bytes_max",
        "crypto_kdf_hkdf_sha512_statebytes",
        "crypto_kdf_hkdf_sha512_extract",
        "crypto_kdf_hkdf_sha512_extract_init",
        "crypto_kdf_hkdf_sha512_extract_update",
        "crypto_kdf_hkdf_sha512_extract_final",
        "crypto_kdf_hkdf_sha512_expand",
        "crypto_kdf_hkdf_sha512_keygen",
    ] {
        assert!(has(s), "symbol `{s}` missing from one of the two libraries");
    }
    // There is deliberately no `crypto_kdf_statebytes` / `crypto_kdf_blake2b_*`
    // streaming API, and no `crypto_kdf_hkdf_*_bytes_min` lower-bound check.
    assert!(!has("crypto_kdf_statebytes"));
    assert!(!has("crypto_kdf_blake2b_statebytes"));
}
