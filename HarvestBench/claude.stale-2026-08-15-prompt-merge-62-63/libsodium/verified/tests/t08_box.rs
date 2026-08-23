//! t08_box.rs — C-vs-Rust differential verification of the whole `crypto_box`
//! surface (both primitives, every API shape).
//!
//! CONFIGS.md rows 202–216 and ERRORS.md rows 131–153 are the specification.
//! Every call goes through `dlsym` on BOTH shared objects; no Rust function is
//! ever called directly. Every output buffer is prefilled with 0xAA and carries
//! a 32-byte trailing guard; the FULL buffer (body + guard) is compared.
//!
//! CONFIGS row → test mapping
//! --------------------------
//! * 202 `_keypair` / `_seed_keypair` ......... `cfg202_seed_keypair_deterministic`,
//!                                             `cfg202_keypair_rng`
//! * 203 `_easy` / `_open_easy` .............. `cfg203_easy_open_easy`
//! * 204 `_detached` / `_open_detached` ...... `cfg204_detached_open_detached`
//! * 205 `_beforenm` + `_easy_afternm` ....... `cfg205_beforenm_easy_afternm`
//! * 206 `_detached_afternm` overlap axis .... `cfg206_detached_afternm_overlap`,
//!                                             `cfg206_open_detached_afternm_overlap`,
//!                                             `cfg206_afternm_aliases_secretbox`
//! * 207 `_afternm` / `_open_afternm` ........ `cfg207_afternm_padded`
//! * 208 `crypto_box` / `crypto_box_open` .... `cfg208_209_padded_nacl_api`
//! * 209 primitive-level padded .............. `cfg208_209_padded_nacl_api`
//! * 210 `_seal` / `_seal_open` .............. `cfg210_seal_seal_open`,
//!                                             `cfg210_seal_tamper`
//! * 211 xchacha20 `_keypair`/`_seed_keypair`  `cfg211_xchacha_keypairs`
//! * 212 xchacha20 easy/detached ............. `cfg212_xchacha_easy_detached`
//! * 213 xchacha20 precomputed shapes ........ `cfg213_xchacha_afternm_shapes`,
//!                                             `cfg213_xchacha_has_no_afternm_or_padded_api`
//! * 214 xchacha20 `_seal` / `_seal_open` .... `cfg214_xchacha_seal`
//! * 215 ECDH agreement (both primitives) .... `cfg215_ecdh_agreement`
//! * 216 all constant getters ................ `cfg216_constant_getters`
//!
//! ERRORS row → test mapping
//! -------------------------
//! * 131 `_beforenm` low-order pk ............ `e131_beforenm_low_order`
//! * 132 `_detached` beforenm fails .......... `e132_e135_e136_low_order_easy_detached`
//! * 133 `_easy` mlen > MESSAGEBYTES_MAX ..... `e133_e134_easy_messagebytes_max`
//! * 134 `_easy_afternm` mlen > MAX .......... `e133_e134_easy_messagebytes_max`
//! * 135 `_easy` beforenm fails .............. `e132_e135_e136_low_order_easy_detached`
//! * 136 `_open_detached` beforenm fails ..... `e132_e135_e136_low_order_easy_detached`
//! * 137 `_open_easy` clen < 16 .............. `e137_e138_open_easy_clen_too_short`
//! * 138 `_open_easy_afternm` clen < 16 ...... `e137_e138_open_easy_clen_too_short`
//! * 139 `_open_detached_afternm` MAC fail ... `e139_open_detached_afternm_poly_mismatch`
//! * 140 `crypto_box` mlen < 32 .............. `e140_e141_padded_length_floor`
//! * 141 `crypto_box_open` clen < 32 ......... `e140_e141_padded_length_floor`
//! * 142 `_seal` mlen > MESSAGEBYTES_MAX ..... `e142_e151_seal_messagebytes_max`
//! * 143 `_seal` low-order pk (epk written) .. `e143_seal_low_order_pk_epk_still_written`
//! * 144 `_seal_open` clen < 48 .............. `e144_e152_seal_open_clen_too_short`
//! * 145 `_seal_open` inner open fails ....... `e145_seal_open_inner_failure`
//! * 146 xchacha `_beforenm` scalarmult fail . `e131_beforenm_low_order`
//! * 147 xchacha `_easy` mlen > MAX .......... `e147_e148_xchacha_easy_messagebytes_max`
//! * 148 xchacha `_easy_afternm` mlen > MAX .. `e147_e148_xchacha_easy_messagebytes_max`
//! * 149 xchacha `_open_easy` clen < 16 ...... `e149_e150_xchacha_open_easy_clen_too_short`
//! * 150 xchacha `_open_easy_afternm` clen<16  `e149_e150_xchacha_open_easy_clen_too_short`
//! * 151 xchacha `_seal` mlen > MAX .......... `e142_e151_seal_messagebytes_max`
//! * 152 xchacha `_seal_open` clen < 48 ...... `e144_e152_seal_open_clen_too_short`
//! * 153 xchacha `_detached`/`_open_detached`  `e153_xchacha_detached_low_order`

mod common;
use common::*;
use libc::{c_char, c_int, c_void};
use std::ffi::CStr;

// ------------------------------------------------------------------ fn types

type SizeFn = unsafe extern "C" fn() -> usize;
type PrimFn = unsafe extern "C" fn() -> *const c_char;
type KeypairFn = unsafe extern "C" fn(*mut u8, *mut u8) -> c_int;
type SeedKeypairFn = unsafe extern "C" fn(*mut u8, *mut u8, *const u8) -> c_int;
/// `_beforenm(k, pk, sk)`, also `crypto_scalarmult_curve25519(q, n, p)`.
type Arg3Fn = unsafe extern "C" fn(*mut u8, *const u8, *const u8) -> c_int;
/// `_easy(c, m, mlen, n, pk, sk)` / `_open_easy(m, c, clen, n, pk, sk)` /
/// `crypto_box(c, m, mlen, n, pk, sk)` / `crypto_box_open(...)`.
type EasyFn =
    unsafe extern "C" fn(*mut u8, *const u8, u64, *const u8, *const u8, *const u8) -> c_int;
/// `_easy_afternm(c, m, mlen, n, k)` / `_afternm(c, m, mlen, n, k)` / opens.
type AfternmFn = unsafe extern "C" fn(*mut u8, *const u8, u64, *const u8, *const u8) -> c_int;
/// `_detached(c, mac, m, mlen, n, pk, sk)`.
type DetachedFn = unsafe extern "C" fn(
    *mut u8,
    *mut u8,
    *const u8,
    u64,
    *const u8,
    *const u8,
    *const u8,
) -> c_int;
/// `_open_detached(m, c, mac, clen, n, pk, sk)`.
type OpenDetachedFn = unsafe extern "C" fn(
    *mut u8,
    *const u8,
    *const u8,
    u64,
    *const u8,
    *const u8,
    *const u8,
) -> c_int;
/// `_detached_afternm(c, mac, m, mlen, n, k)` (== `crypto_secretbox_detached`).
type DetachedAfternmFn =
    unsafe extern "C" fn(*mut u8, *mut u8, *const u8, u64, *const u8, *const u8) -> c_int;
/// `_open_detached_afternm(m, c, mac, clen, n, k)`.
type OpenDetachedAfternmFn =
    unsafe extern "C" fn(*mut u8, *const u8, *const u8, u64, *const u8, *const u8) -> c_int;
type SealFn = unsafe extern "C" fn(*mut u8, *const u8, u64, *const u8) -> c_int;
type SealOpenFn = unsafe extern "C" fn(*mut u8, *const u8, u64, *const u8, *const u8) -> c_int;
type ScalarmultBaseFn = unsafe extern "C" fn(*mut u8, *const u8) -> c_int;
type CoreFn = unsafe extern "C" fn(*mut u8, *const u8, *const u8, *const u8) -> c_int;
type Sha512Fn = unsafe extern "C" fn(*mut u8, *const u8, u64) -> c_int;
type GenerichashFn =
    unsafe extern "C" fn(*mut u8, usize, *const u8, u64, *const u8, usize) -> c_int;
type BufFn = unsafe extern "C" fn(*mut c_void, usize);
type SetImplFn = unsafe extern "C" fn(*const RandombytesImpl) -> c_int;

// ------------------------------------------------------------------ constants

/// Prefill byte for every output buffer.
const FILL: u8 = 0xAA;
/// Trailing guard region: must never be touched by the library.
const PAD: usize = 32;

const PKB: usize = 32;
const SKB: usize = 32;
const KB: usize = 32; // BEFORENMBYTES
const NB: usize = 24; // NONCEBYTES
const MB: usize = 16; // MACBYTES
const ZB: usize = 32; // ZEROBYTES
const BZB: usize = 16; // BOXZEROBYTES
const SEALB: usize = 48;
/// `crypto_box_MESSAGEBYTES_MAX == SODIUM_SIZE_MAX - MACBYTES == UINT64_MAX - 16`.
const MSGMAX: u64 = u64::MAX - 16;

/// CONFIGS 203/204/212 `mlen` sweep.
const MLENS: &[usize] = &[0, 1, 31, 32, 33, 64, 1000];
/// CONFIGS 207/208/209 padded-NaCl `mlen` sweep (all >= ZEROBYTES).
const PADDED_MLENS: &[usize] = &[32, 33, 64, 96, 1000];
/// CONFIGS 210/214 seal `mlen` sweep: the row lists {0,1,64,1000}; the full
/// easy-shape sweep is used so the `mlen0 = min(mlen,32)` boundary is covered
/// through the sealed-box path too.
const SEAL_MLENS: &[usize] = &[0, 1, 31, 32, 33, 64, 1000];
/// Sweep that straddles the `mlen0 = min(mlen, 32)` first-block boundary of
/// `crypto_secretbox_detached` (CONFIGS 206).
const OVERLAP_MLENS: &[usize] = &[0, 1, 15, 16, 31, 32, 33, 63, 64, 65, 127, 1000];

/// The 7 blocklisted curve25519 encodings from `has_small_order()` in
/// `c_src/libsodium/crypto_scalarmult/curve25519/ref10/x25519_ref10.c`.
/// `crypto_scalarmult_curve25519()` rejects each of them, so `_beforenm` and
/// everything layered on top of it must return -1 (ERRORS 131 / 146).
const LOW_ORDER: [[u8; 32]; 7] = [
    // 0 (order 4)
    [0x00; 32],
    // 1 (order 1)
    [
        0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00,
    ],
    // 325606250916557431795983626356110631294008115727848805560023387167927233504 (order 8)
    [
        0xe0, 0xeb, 0x7a, 0x7c, 0x3b, 0x41, 0xb8, 0xae, 0x16, 0x56, 0xe3, 0xfa, 0xf1, 0x9f, 0xc4,
        0x6a, 0xda, 0x09, 0x8d, 0xeb, 0x9c, 0x32, 0xb1, 0xfd, 0x86, 0x62, 0x05, 0x16, 0x5f, 0x49,
        0xb8, 0x00,
    ],
    // 39382357235489614581723060781553021112529911719440698176882885853963445705823 (order 8)
    [
        0x5f, 0x9c, 0x95, 0xbc, 0xa3, 0x50, 0x8c, 0x24, 0xb1, 0xd0, 0xb1, 0x55, 0x9c, 0x83, 0xef,
        0x5b, 0x04, 0x44, 0x5c, 0xc4, 0x58, 0x1c, 0x8e, 0x86, 0xd8, 0x22, 0x4e, 0xdd, 0xd0, 0x9f,
        0x11, 0x57,
    ],
    // p-1 (order 2)
    [
        0xec, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
        0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
        0xff, 0x7f,
    ],
    // p (= 0, order 4)
    [
        0xed, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
        0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
        0xff, 0x7f,
    ],
    // p+1 (= 1, order 1)
    [
        0xee, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
        0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
        0xff, 0x7f,
    ],
];

/// `has_small_order()` masks bit 7 of `s[31]`, so every blocklist entry has two
/// encodings. Both must be rejected.
fn low_order_pks() -> Vec<[u8; 32]> {
    let mut v: Vec<[u8; 32]> = LOW_ORDER.to_vec();
    for e in LOW_ORDER.iter() {
        let mut w = *e;
        w[31] |= 0x80;
        v.push(w);
    }
    v
}

// ---------------------------------------------------------------- primitives
//
// (name of the low-level `_easy`-style entry point for each primitive)

struct Prim {
    tag: &'static str,
    seed_keypair: &'static str,
    /// resolved by name in `cfg202_keypair_rng`; kept here for completeness
    #[allow(dead_code)]
    keypair: &'static str,
    beforenm: &'static str,
    easy: &'static str,
    open_easy: &'static str,
    detached: &'static str,
    open_detached: &'static str,
    easy_afternm: &'static str,
    open_easy_afternm: &'static str,
    detached_afternm: &'static str,
    open_detached_afternm: &'static str,
    seal: &'static str,
    seal_open: &'static str,
    /// the `crypto_secretbox_*_detached` the `_detached_afternm` alias resolves to
    secretbox_detached: &'static str,
    secretbox_open_detached: &'static str,
    /// `crypto_core_hsalsa20` or `crypto_core_hchacha20`
    core: &'static str,
}

const XSALSA: Prim = Prim {
    tag: "xsalsa20",
    seed_keypair: "crypto_box_seed_keypair",
    keypair: "crypto_box_keypair",
    beforenm: "crypto_box_beforenm",
    easy: "crypto_box_easy",
    open_easy: "crypto_box_open_easy",
    detached: "crypto_box_detached",
    open_detached: "crypto_box_open_detached",
    easy_afternm: "crypto_box_easy_afternm",
    open_easy_afternm: "crypto_box_open_easy_afternm",
    detached_afternm: "crypto_box_detached_afternm",
    open_detached_afternm: "crypto_box_open_detached_afternm",
    seal: "crypto_box_seal",
    seal_open: "crypto_box_seal_open",
    secretbox_detached: "crypto_secretbox_detached",
    secretbox_open_detached: "crypto_secretbox_open_detached",
    core: "crypto_core_hsalsa20",
};

const XCHACHA: Prim = Prim {
    tag: "xchacha20",
    seed_keypair: "crypto_box_curve25519xchacha20poly1305_seed_keypair",
    keypair: "crypto_box_curve25519xchacha20poly1305_keypair",
    beforenm: "crypto_box_curve25519xchacha20poly1305_beforenm",
    easy: "crypto_box_curve25519xchacha20poly1305_easy",
    open_easy: "crypto_box_curve25519xchacha20poly1305_open_easy",
    detached: "crypto_box_curve25519xchacha20poly1305_detached",
    open_detached: "crypto_box_curve25519xchacha20poly1305_open_detached",
    easy_afternm: "crypto_box_curve25519xchacha20poly1305_easy_afternm",
    open_easy_afternm: "crypto_box_curve25519xchacha20poly1305_open_easy_afternm",
    detached_afternm: "crypto_box_curve25519xchacha20poly1305_detached_afternm",
    open_detached_afternm: "crypto_box_curve25519xchacha20poly1305_open_detached_afternm",
    seal: "crypto_box_curve25519xchacha20poly1305_seal",
    seal_open: "crypto_box_curve25519xchacha20poly1305_seal_open",
    secretbox_detached: "crypto_secretbox_xchacha20poly1305_detached",
    secretbox_open_detached: "crypto_secretbox_xchacha20poly1305_open_detached",
    core: "crypto_core_hchacha20",
};

/// The `crypto_box_curve25519xsalsa20poly1305_*` names — the same primitive as
/// `crypto_box_*` but reached through the primitive-level exports.
const XSALSA_P: Prim = Prim {
    tag: "xsalsa20-primitive",
    seed_keypair: "crypto_box_curve25519xsalsa20poly1305_seed_keypair",
    keypair: "crypto_box_curve25519xsalsa20poly1305_keypair",
    beforenm: "crypto_box_curve25519xsalsa20poly1305_beforenm",
    // the primitive level has no _easy family of its own; the generic names are
    // literal wrappers, so reuse them where an easy shape is needed.
    easy: "crypto_box_easy",
    open_easy: "crypto_box_open_easy",
    detached: "crypto_box_detached",
    open_detached: "crypto_box_open_detached",
    easy_afternm: "crypto_box_easy_afternm",
    open_easy_afternm: "crypto_box_open_easy_afternm",
    detached_afternm: "crypto_box_detached_afternm",
    open_detached_afternm: "crypto_box_open_detached_afternm",
    seal: "crypto_box_seal",
    seal_open: "crypto_box_seal_open",
    secretbox_detached: "crypto_secretbox_detached",
    secretbox_open_detached: "crypto_secretbox_open_detached",
    core: "crypto_core_hsalsa20",
};

// -------------------------------------------------------------- misc helpers

/// Serialises every test that mutates the process-global `randombytes`
/// implementation pointer (cargo runs the tests of one binary as parallel
/// threads inside ONE process, and both `.so`s are shared by all of them).
static RNG_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
fn rng_lock() -> std::sync::MutexGuard<'static, ()> {
    RNG_LOCK.lock().unwrap_or_else(|e| e.into_inner())
}

fn data_ptr<T: 'static>(lib: &'static libloading::Library, name: &str) -> *const T {
    let s = unsafe { sym::<*const T>(lib, name) };
    *s
}

/// Put both libraries back on their default `sysrandom` implementation.
fn restore_sysrandom() {
    let l = libs();
    let (c, r) = unsafe { pair::<SetImplFn>("randombytes_set_implementation") };
    let pc = data_ptr::<RandombytesImpl>(&l.c, "randombytes_sysrandom_implementation");
    let pr = data_ptr::<RandombytesImpl>(&l.r, "randombytes_sysrandom_implementation");
    unsafe {
        c(pc);
        r(pr);
    }
}

/// Position BOTH deterministic counter streams at byte offset `skip`, using each
/// library's own `randombytes_buf` so no assumption is made about the harness'
/// internal PRNG.
fn rng_seek(skip: usize) {
    reset_det_rng();
    if skip == 0 {
        return;
    }
    let (cbuf, rbuf) = unsafe { pair::<BufFn>("randombytes_buf") };
    let mut junk = vec![0u8; skip];
    unsafe {
        cbuf(junk.as_mut_ptr() as *mut c_void, skip);
        rbuf(junk.as_mut_ptr() as *mut c_void, skip);
    }
}

/// The next 32 bytes the deterministic stream will hand out at offset `skip` —
/// i.e. exactly the ephemeral secret key `_seal` / `_keypair` is about to use.
fn rng_peek32(skip: usize) -> [u8; 32] {
    rng_seek(skip);
    let (cbuf, _) = unsafe { pair::<BufFn>("randombytes_buf") };
    let mut out = [0u8; 32];
    unsafe { cbuf(out.as_mut_ptr() as *mut c_void, 32) };
    out
}

fn clear_errno() {
    unsafe { *libc::__errno_location() = 0 };
}
fn errno() -> c_int {
    unsafe { *libc::__errno_location() }
}

/// A guarded output buffer: `len` payload bytes followed by `PAD` guard bytes,
/// everything prefilled with 0xAA.
struct Out {
    v: Vec<u8>,
    len: usize,
}

impl Out {
    fn new(len: usize) -> Self {
        Out { v: vec![FILL; len + PAD], len }
    }
    fn ptr(&mut self) -> *mut u8 {
        self.v.as_mut_ptr()
    }
    fn body(&self) -> &[u8] {
        &self.v[..self.len]
    }
    fn untouched(&self) -> bool {
        self.v.iter().all(|&x| x == FILL)
    }
}

fn guard_intact(what: &str, who: &str, o: &Out) {
    assert!(
        o.v[o.len..].iter().all(|&x| x == FILL),
        "{what}: {who} wrote OUTSIDE the requested {} bytes \
         (0xAA trailing guard clobbered: {})",
        o.len,
        hexs(&o.v[o.len..])
    );
}

/// A message buffer whose `as_ptr()` is always a real allocation, even for
/// `len == 0` (so a zero-length message never hands the library a dangling
/// pointer that the two libraries could treat differently).
fn msg(len: usize, rng: &mut Rng) -> Vec<u8> {
    let mut v = Vec::with_capacity(len + 1);
    for _ in 0..len {
        v.push(rng.byte());
    }
    v
}

fn stable(m: &[u8]) -> Vec<u8> {
    let mut v = Vec::with_capacity(m.len() + 1);
    v.extend_from_slice(m);
    v
}

/// Assert a symbol is absent from BOTH libraries (CONFIGS 213).
fn assert_absent(name: &str) {
    let l = libs();
    for (who, lib) in [("C", &l.c), ("rust", &l.r)] {
        let mut owned: Vec<u8> = name.as_bytes().to_vec();
        owned.push(0);
        let got = unsafe { lib.get::<*const ()>(&owned) };
        assert!(
            got.is_err(),
            "{who} library unexpectedly EXPORTS `{name}`: this primitive has no \
             such entry point in the C headers"
        );
    }
}

// ------------------------------------------------------------------- keypairs

#[derive(Clone, Copy)]
struct Kp {
    pk: [u8; PKB],
    sk: [u8; SKB],
}

/// Derive a keypair with `_seed_keypair` (fully deterministic: no RNG shim
/// needed) through BOTH libraries and assert they agree, so every downstream row
/// starts from key material that is already proven identical.
fn kp(prim: &Prim, seed: &[u8; 32]) -> Kp {
    let (fc, fr) = unsafe { pair::<SeedKeypairFn>(prim.seed_keypair) };
    let mut pc = Out::new(PKB);
    let mut sc = Out::new(SKB);
    let mut pr = Out::new(PKB);
    let mut sr = Out::new(SKB);
    let rc = unsafe { fc(pc.ptr(), sc.ptr(), seed.as_ptr()) };
    let rr = unsafe { fr(pr.ptr(), sr.ptr(), seed.as_ptr()) };
    let what = format!("{} [{}] seed={}", prim.seed_keypair, prim.tag, hexs(seed));
    assert_eq!(rc, rr, "{what}: return differs (C={rc} rust={rr})");
    assert_eq!(rc, 0, "{what}: C returned {rc}, expected 0");
    assert_eq_bytes(&format!("{what} pk"), &pc.v, &pr.v);
    assert_eq_bytes(&format!("{what} sk"), &sc.v, &sr.v);
    guard_intact(&what, "C", &pc);
    guard_intact(&what, "C", &sc);
    guard_intact(&what, "rust", &pr);
    guard_intact(&what, "rust", &sr);
    let mut k = Kp { pk: [0; PKB], sk: [0; SKB] };
    k.pk.copy_from_slice(pc.body());
    k.sk.copy_from_slice(sc.body());
    k
}

/// A small deterministic key/nonce pool shared by the property rows.
fn pool(prim: &Prim, n: usize) -> (Vec<Kp>, Vec<Vec<u8>>) {
    let mut rng = Rng::new(SEED ^ 0x808);
    let mut kps = Vec::new();
    for i in 0..n {
        let mut seed = [0u8; 32];
        rng.fill(&mut seed);
        seed[0] ^= i as u8;
        kps.push(kp(prim, &seed));
    }
    let mut nonces = patterns(NB, &mut rng);
    nonces.push(vec![0x80u8; NB]);
    (kps, nonces)
}

// ------------------------------------------------------- differential drivers
//
// Each driver runs ONE entry point through both `.so` files with 0xAA-prefilled
// guarded output buffers, asserts the return value, errno and the FULL buffers
// agree, checks nothing was written past the payload, and returns the (now
// proven identical) C output so callers can assert algebraic properties that
// then hold for BOTH libraries.

fn d_beforenm(name: &str, pk: &[u8], sk: &[u8], tag: &str) -> (c_int, Vec<u8>) {
    let (fc, fr) = unsafe { pair::<Arg3Fn>(name) };
    let mut oc = Out::new(KB);
    let mut or = Out::new(KB);
    clear_errno();
    let rc = unsafe { fc(oc.ptr(), pk.as_ptr(), sk.as_ptr()) };
    let ec = errno();
    clear_errno();
    let rr = unsafe { fr(or.ptr(), pk.as_ptr(), sk.as_ptr()) };
    let er = errno();
    let what = format!("{name} [{tag}] pk={} sk={}", hexs(pk), hexs(sk));
    assert_eq!(rc, rr, "{what}: return differs (C={rc} rust={rr})");
    assert_eq!(ec, er, "{what}: errno differs (C={ec} rust={er})");
    assert_eq_bytes(&what, &oc.v, &or.v);
    guard_intact(&what, "C", &oc);
    guard_intact(&what, "rust", &or);
    if rc != 0 {
        assert!(oc.untouched(), "{what}: C wrote k on the failure path");
        assert!(or.untouched(), "{what}: rust wrote k on the failure path");
    }
    (rc, oc.body().to_vec())
}

/// `_easy(c, m, mlen, n, pk, sk)` — also used for the padded `crypto_box(...)`.
fn d_easy(name: &str, clen: usize, m: &[u8], n: &[u8], pk: &[u8], sk: &[u8], tag: &str) -> (c_int, Vec<u8>) {
    let (fc, fr) = unsafe { pair::<EasyFn>(name) };
    let mb = stable(m);
    let mut oc = Out::new(clen);
    let mut or = Out::new(clen);
    clear_errno();
    let rc = unsafe { fc(oc.ptr(), mb.as_ptr(), m.len() as u64, n.as_ptr(), pk.as_ptr(), sk.as_ptr()) };
    let ec = errno();
    clear_errno();
    let rr = unsafe { fr(or.ptr(), mb.as_ptr(), m.len() as u64, n.as_ptr(), pk.as_ptr(), sk.as_ptr()) };
    let er = errno();
    let what = format!(
        "{name} [{tag}] mlen={} pk={} sk={} n={}",
        m.len(),
        hexs(pk),
        hexs(sk),
        hexs(n)
    );
    assert_eq!(rc, rr, "{what}: return differs (C={rc} rust={rr})");
    assert_eq!(ec, er, "{what}: errno differs (C={ec} rust={er})");
    assert_eq_bytes(&what, &oc.v, &or.v);
    guard_intact(&what, "C", &oc);
    guard_intact(&what, "rust", &or);
    (rc, oc.body().to_vec())
}

/// `_open_easy(m, c, clen, n, pk, sk)` — also the padded `crypto_box_open`.
fn d_open_easy(name: &str, mlen: usize, c: &[u8], n: &[u8], pk: &[u8], sk: &[u8], tag: &str) -> (c_int, Vec<u8>) {
    let (fc, fr) = unsafe { pair::<EasyFn>(name) };
    let cb = stable(c);
    let mut oc = Out::new(mlen);
    let mut or = Out::new(mlen);
    clear_errno();
    let rc = unsafe { fc(oc.ptr(), cb.as_ptr(), c.len() as u64, n.as_ptr(), pk.as_ptr(), sk.as_ptr()) };
    let ec = errno();
    clear_errno();
    let rr = unsafe { fr(or.ptr(), cb.as_ptr(), c.len() as u64, n.as_ptr(), pk.as_ptr(), sk.as_ptr()) };
    let er = errno();
    let what = format!(
        "{name} [{tag}] clen={} pk={} sk={} n={}",
        c.len(),
        hexs(pk),
        hexs(sk),
        hexs(n)
    );
    assert_eq!(rc, rr, "{what}: return differs (C={rc} rust={rr})");
    assert_eq!(ec, er, "{what}: errno differs (C={ec} rust={er})");
    assert_eq_bytes(&what, &oc.v, &or.v);
    guard_intact(&what, "C", &oc);
    guard_intact(&what, "rust", &or);
    if rc != 0 {
        assert!(oc.untouched(), "{what}: C wrote m on the failure path");
        assert!(or.untouched(), "{what}: rust wrote m on the failure path");
    }
    (rc, oc.body().to_vec())
}

/// `_easy_afternm(c, m, mlen, n, k)` and the padded `_afternm(c, m, mlen, n, k)`.
fn d_afternm(name: &str, clen: usize, m: &[u8], n: &[u8], k: &[u8], tag: &str) -> (c_int, Vec<u8>) {
    let (fc, fr) = unsafe { pair::<AfternmFn>(name) };
    let mb = stable(m);
    let mut oc = Out::new(clen);
    let mut or = Out::new(clen);
    clear_errno();
    let rc = unsafe { fc(oc.ptr(), mb.as_ptr(), m.len() as u64, n.as_ptr(), k.as_ptr()) };
    let ec = errno();
    clear_errno();
    let rr = unsafe { fr(or.ptr(), mb.as_ptr(), m.len() as u64, n.as_ptr(), k.as_ptr()) };
    let er = errno();
    let what = format!("{name} [{tag}] mlen={} k={} n={}", m.len(), hexs(k), hexs(n));
    assert_eq!(rc, rr, "{what}: return differs (C={rc} rust={rr})");
    assert_eq!(ec, er, "{what}: errno differs (C={ec} rust={er})");
    assert_eq_bytes(&what, &oc.v, &or.v);
    guard_intact(&what, "C", &oc);
    guard_intact(&what, "rust", &or);
    (rc, oc.body().to_vec())
}

/// `_open_easy_afternm(m, c, clen, n, k)` / padded `_open_afternm`.
fn d_open_afternm(name: &str, mlen: usize, c: &[u8], n: &[u8], k: &[u8], tag: &str) -> (c_int, Vec<u8>) {
    let (fc, fr) = unsafe { pair::<AfternmFn>(name) };
    let cb = stable(c);
    let mut oc = Out::new(mlen);
    let mut or = Out::new(mlen);
    clear_errno();
    let rc = unsafe { fc(oc.ptr(), cb.as_ptr(), c.len() as u64, n.as_ptr(), k.as_ptr()) };
    let ec = errno();
    clear_errno();
    let rr = unsafe { fr(or.ptr(), cb.as_ptr(), c.len() as u64, n.as_ptr(), k.as_ptr()) };
    let er = errno();
    let what = format!("{name} [{tag}] clen={} k={} n={}", c.len(), hexs(k), hexs(n));
    assert_eq!(rc, rr, "{what}: return differs (C={rc} rust={rr})");
    assert_eq!(ec, er, "{what}: errno differs (C={ec} rust={er})");
    assert_eq_bytes(&what, &oc.v, &or.v);
    guard_intact(&what, "C", &oc);
    guard_intact(&what, "rust", &or);
    if rc != 0 {
        assert!(oc.untouched(), "{what}: C wrote m on the failure path");
        assert!(or.untouched(), "{what}: rust wrote m on the failure path");
    }
    (rc, oc.body().to_vec())
}

/// `_detached(c, mac, m, mlen, n, pk, sk)` -> (ret, c, mac)
fn d_detached(name: &str, m: &[u8], n: &[u8], pk: &[u8], sk: &[u8], tag: &str) -> (c_int, Vec<u8>, Vec<u8>) {
    let (fc, fr) = unsafe { pair::<DetachedFn>(name) };
    let mb = stable(m);
    let mut cc = Out::new(m.len());
    let mut ac = Out::new(MB);
    let mut cr = Out::new(m.len());
    let mut ar = Out::new(MB);
    clear_errno();
    let rc = unsafe {
        fc(cc.ptr(), ac.ptr(), mb.as_ptr(), m.len() as u64, n.as_ptr(), pk.as_ptr(), sk.as_ptr())
    };
    let ec = errno();
    clear_errno();
    let rr = unsafe {
        fr(cr.ptr(), ar.ptr(), mb.as_ptr(), m.len() as u64, n.as_ptr(), pk.as_ptr(), sk.as_ptr())
    };
    let er = errno();
    let what = format!("{name} [{tag}] mlen={} pk={} n={}", m.len(), hexs(pk), hexs(n));
    assert_eq!(rc, rr, "{what}: return differs (C={rc} rust={rr})");
    assert_eq!(ec, er, "{what}: errno differs (C={ec} rust={er})");
    assert_eq_bytes(&format!("{what} c"), &cc.v, &cr.v);
    assert_eq_bytes(&format!("{what} mac"), &ac.v, &ar.v);
    guard_intact(&what, "C", &cc);
    guard_intact(&what, "C", &ac);
    guard_intact(&what, "rust", &cr);
    guard_intact(&what, "rust", &ar);
    if rc != 0 {
        assert!(cc.untouched() && ac.untouched(), "{what}: C wrote output on the failure path");
        assert!(cr.untouched() && ar.untouched(), "{what}: rust wrote output on the failure path");
    }
    (rc, cc.body().to_vec(), ac.body().to_vec())
}

/// `_open_detached(m, c, mac, clen, n, pk, sk)`; `m_null` selects verify-only.
fn d_open_detached(
    name: &str,
    c: &[u8],
    mac: &[u8],
    n: &[u8],
    pk: &[u8],
    sk: &[u8],
    m_null: bool,
    tag: &str,
) -> (c_int, Vec<u8>) {
    let (fc, fr) = unsafe { pair::<OpenDetachedFn>(name) };
    let cb = stable(c);
    let mut oc = Out::new(c.len());
    let mut or = Out::new(c.len());
    let (pc, pr) = if m_null {
        (std::ptr::null_mut(), std::ptr::null_mut())
    } else {
        (oc.ptr(), or.ptr())
    };
    clear_errno();
    let rc = unsafe {
        fc(pc, cb.as_ptr(), mac.as_ptr(), c.len() as u64, n.as_ptr(), pk.as_ptr(), sk.as_ptr())
    };
    let ec = errno();
    clear_errno();
    let rr = unsafe {
        fr(pr, cb.as_ptr(), mac.as_ptr(), c.len() as u64, n.as_ptr(), pk.as_ptr(), sk.as_ptr())
    };
    let er = errno();
    let what = format!(
        "{name} [{tag}] clen={} m_null={m_null} pk={} n={}",
        c.len(),
        hexs(pk),
        hexs(n)
    );
    assert_eq!(rc, rr, "{what}: return differs (C={rc} rust={rr})");
    assert_eq!(ec, er, "{what}: errno differs (C={ec} rust={er})");
    assert_eq_bytes(&what, &oc.v, &or.v);
    guard_intact(&what, "C", &oc);
    guard_intact(&what, "rust", &or);
    if m_null || rc != 0 {
        assert!(oc.untouched(), "{what}: C wrote m although it must not have");
        assert!(or.untouched(), "{what}: rust wrote m although it must not have");
    }
    (rc, oc.body().to_vec())
}

/// `_detached_afternm(c, mac, m, mlen, n, k)` -> (ret, c, mac)
fn d_detached_afternm(name: &str, m: &[u8], n: &[u8], k: &[u8], tag: &str) -> (c_int, Vec<u8>, Vec<u8>) {
    let (fc, fr) = unsafe { pair::<DetachedAfternmFn>(name) };
    let mb = stable(m);
    let mut cc = Out::new(m.len());
    let mut ac = Out::new(MB);
    let mut cr = Out::new(m.len());
    let mut ar = Out::new(MB);
    clear_errno();
    let rc = unsafe { fc(cc.ptr(), ac.ptr(), mb.as_ptr(), m.len() as u64, n.as_ptr(), k.as_ptr()) };
    let ec = errno();
    clear_errno();
    let rr = unsafe { fr(cr.ptr(), ar.ptr(), mb.as_ptr(), m.len() as u64, n.as_ptr(), k.as_ptr()) };
    let er = errno();
    let what = format!("{name} [{tag}] mlen={} k={} n={}", m.len(), hexs(k), hexs(n));
    assert_eq!(rc, rr, "{what}: return differs (C={rc} rust={rr})");
    assert_eq!(ec, er, "{what}: errno differs (C={ec} rust={er})");
    assert_eq_bytes(&format!("{what} c"), &cc.v, &cr.v);
    assert_eq_bytes(&format!("{what} mac"), &ac.v, &ar.v);
    guard_intact(&what, "C", &cc);
    guard_intact(&what, "C", &ac);
    guard_intact(&what, "rust", &cr);
    guard_intact(&what, "rust", &ar);
    (rc, cc.body().to_vec(), ac.body().to_vec())
}

/// `_open_detached_afternm(m, c, mac, clen, n, k)`; `m_null` = verify-only.
fn d_open_detached_afternm(
    name: &str,
    c: &[u8],
    mac: &[u8],
    n: &[u8],
    k: &[u8],
    m_null: bool,
    tag: &str,
) -> (c_int, Vec<u8>) {
    let (fc, fr) = unsafe { pair::<OpenDetachedAfternmFn>(name) };
    let cb = stable(c);
    let mut oc = Out::new(c.len());
    let mut or = Out::new(c.len());
    let (pc, pr) = if m_null {
        (std::ptr::null_mut(), std::ptr::null_mut())
    } else {
        (oc.ptr(), or.ptr())
    };
    clear_errno();
    let rc = unsafe { fc(pc, cb.as_ptr(), mac.as_ptr(), c.len() as u64, n.as_ptr(), k.as_ptr()) };
    let ec = errno();
    clear_errno();
    let rr = unsafe { fr(pr, cb.as_ptr(), mac.as_ptr(), c.len() as u64, n.as_ptr(), k.as_ptr()) };
    let er = errno();
    let what = format!("{name} [{tag}] clen={} m_null={m_null} k={} n={}", c.len(), hexs(k), hexs(n));
    assert_eq!(rc, rr, "{what}: return differs (C={rc} rust={rr})");
    assert_eq!(ec, er, "{what}: errno differs (C={ec} rust={er})");
    assert_eq_bytes(&what, &oc.v, &or.v);
    guard_intact(&what, "C", &oc);
    guard_intact(&what, "rust", &or);
    if m_null || rc != 0 {
        assert!(oc.untouched(), "{what}: C wrote m although it must not have");
        assert!(or.untouched(), "{what}: rust wrote m although it must not have");
    }
    (rc, oc.body().to_vec())
}

/// `_seal(c, m, mlen, pk)` with the deterministic RNG positioned at `skip`.
/// Caller must hold `rng_lock()` and have called `install_det_rng`.
fn d_seal(name: &str, m: &[u8], pk: &[u8], skip: usize, tag: &str) -> (c_int, Vec<u8>) {
    let (fc, fr) = unsafe { pair::<SealFn>(name) };
    let mb = stable(m);
    let clen = m.len() + SEALB;
    let mut oc = Out::new(clen);
    let mut or = Out::new(clen);
    rng_seek(skip);
    clear_errno();
    let rc = unsafe { fc(oc.ptr(), mb.as_ptr(), m.len() as u64, pk.as_ptr()) };
    let ec = errno();
    clear_errno();
    let rr = unsafe { fr(or.ptr(), mb.as_ptr(), m.len() as u64, pk.as_ptr()) };
    let er = errno();
    let what = format!("{name} [{tag}] mlen={} pk={} rngskip={skip}", m.len(), hexs(pk));
    assert_eq!(rc, rr, "{what}: return differs (C={rc} rust={rr})");
    assert_eq!(ec, er, "{what}: errno differs (C={ec} rust={er})");
    assert_eq_bytes(&what, &oc.v, &or.v);
    guard_intact(&what, "C", &oc);
    guard_intact(&what, "rust", &or);
    (rc, oc.body().to_vec())
}

fn d_seal_open(name: &str, mlen: usize, c: &[u8], pk: &[u8], sk: &[u8], tag: &str) -> (c_int, Vec<u8>) {
    let (fc, fr) = unsafe { pair::<SealOpenFn>(name) };
    let cb = stable(c);
    let mut oc = Out::new(mlen);
    let mut or = Out::new(mlen);
    clear_errno();
    let rc = unsafe { fc(oc.ptr(), cb.as_ptr(), c.len() as u64, pk.as_ptr(), sk.as_ptr()) };
    let ec = errno();
    clear_errno();
    let rr = unsafe { fr(or.ptr(), cb.as_ptr(), c.len() as u64, pk.as_ptr(), sk.as_ptr()) };
    let er = errno();
    let what = format!("{name} [{tag}] clen={} pk={}", c.len(), hexs(pk));
    assert_eq!(rc, rr, "{what}: return differs (C={rc} rust={rr})");
    assert_eq!(ec, er, "{what}: errno differs (C={ec} rust={er})");
    assert_eq_bytes(&what, &oc.v, &or.v);
    guard_intact(&what, "C", &oc);
    guard_intact(&what, "rust", &or);
    if rc != 0 {
        assert!(oc.untouched(), "{what}: C wrote m on the failure path");
        assert!(or.untouched(), "{what}: rust wrote m on the failure path");
    }
    (rc, oc.body().to_vec())
}

fn d_size(name: &str) -> usize {
    let (c, r) = unsafe { pair::<SizeFn>(name) };
    let (a, b) = unsafe { (c(), r()) };
    assert_eq!(a, b, "{name}(): C={a} rust={b}");
    a
}

// ------------------------------------------------- forked fatal-path plumbing
//
// `MESSAGEBYTES_MAX == SODIUM_SIZE_MAX - 16 == UINT64_MAX - 16`, so the guard
// `mlen > MESSAGEBYTES_MAX` IS reachable: `mlen = UINT64_MAX` trips it. At
// `mlen == MESSAGEBYTES_MAX` exactly the guard must NOT fire, the call runs on
// and dies on a page fault instead. A SIGSEGV/SIGBUS handler in the forked child
// turns that into `Returned(FAULT)` so the two outcomes are distinguishable.

const FAULT: i64 = 42;

extern "C" fn fault_handler(_sig: c_int) {
    unsafe { libc::_exit(FAULT as c_int) }
}

unsafe fn arm_fault_marker() {
    let rl = libc::rlimit { rlim_cur: 0, rlim_max: 0 };
    libc::setrlimit(libc::RLIMIT_CORE, &rl);
    let mut sa: libc::sigaction = std::mem::zeroed();
    sa.sa_sigaction = fault_handler as extern "C" fn(c_int) as libc::sighandler_t;
    libc::sigemptyset(&mut sa.sa_mask);
    sa.sa_flags = 0;
    libc::sigaction(libc::SIGSEGV, &sa, std::ptr::null_mut());
    libc::sigaction(libc::SIGBUS, &sa, std::ptr::null_mut());
}

const MISUSE: Outcome = Outcome::Signaled(SIGABRT);
const NO_MISUSE: Outcome = Outcome::Returned(FAULT);

/// Resolve `name` in BOTH libraries in the PARENT (so the child needs neither
/// dlsym nor malloc), then run `body` once per library in a forked child.
fn expect_outcome<T: Copy + 'static, B: Fn(T) -> i64 + Copy>(
    what: &str,
    name: &str,
    body: B,
    want: Outcome,
) {
    let l = libs();
    let fc: T = *unsafe { sym::<T>(&l.c, name) };
    let fr: T = *unsafe { sym::<T>(&l.r, name) };
    let oc = forked(move || {
        unsafe { arm_fault_marker() };
        body(fc)
    });
    let or = forked(move || {
        unsafe { arm_fault_marker() };
        body(fr)
    });
    assert_same_fatal(what, oc, or);
    assert_eq!(oc, want, "{what}: C outcome was {oc:?}, expected {want:?}");
}

/// Buffers allocated in the PARENT; the children only read/write them.
struct Scratch {
    _v: Vec<u8>,
    p: *mut u8,
}
fn scratch(n: usize) -> Scratch {
    let mut v = vec![0u8; n];
    let p = v.as_mut_ptr();
    Scratch { _v: v, p }
}

// ============================================================================
// CONFIGS 216 — every constant getter, in both primitives
// ============================================================================

/// CONFIGS 216: `_seedbytes` / `_publickeybytes` / `_secretkeybytes` /
/// `_beforenmbytes` / `_noncebytes` / `_macbytes` / `_zerobytes` /
/// `_boxzerobytes` / `_sealbytes` / `_messagebytes_max` / `_primitive`.
#[test]
fn cfg216_constant_getters() {
    init_both();

    // crypto_box_* (generic) and the xsalsa20 primitive level: same values.
    for pfx in ["crypto_box", "crypto_box_curve25519xsalsa20poly1305"] {
        assert_eq!(d_size(&format!("{pfx}_seedbytes")), 32, "{pfx}_seedbytes");
        assert_eq!(d_size(&format!("{pfx}_publickeybytes")), PKB, "{pfx}_publickeybytes");
        assert_eq!(d_size(&format!("{pfx}_secretkeybytes")), SKB, "{pfx}_secretkeybytes");
        assert_eq!(d_size(&format!("{pfx}_beforenmbytes")), KB, "{pfx}_beforenmbytes");
        assert_eq!(d_size(&format!("{pfx}_noncebytes")), NB, "{pfx}_noncebytes");
        assert_eq!(d_size(&format!("{pfx}_macbytes")), MB, "{pfx}_macbytes");
        assert_eq!(d_size(&format!("{pfx}_zerobytes")), ZB, "{pfx}_zerobytes");
        assert_eq!(d_size(&format!("{pfx}_boxzerobytes")), BZB, "{pfx}_boxzerobytes");
        assert_eq!(
            d_size(&format!("{pfx}_messagebytes_max")) as u64,
            MSGMAX,
            "{pfx}_messagebytes_max"
        );
    }
    // _sealbytes only exists at the generic level for xsalsa20.
    assert_eq!(d_size("crypto_box_sealbytes"), SEALB, "crypto_box_sealbytes");
    assert_eq!(
        d_size("crypto_box_sealbytes"),
        d_size("crypto_box_publickeybytes") + d_size("crypto_box_macbytes")
    );

    // xchacha20 primitive.
    let x = "crypto_box_curve25519xchacha20poly1305";
    assert_eq!(d_size(&format!("{x}_seedbytes")), 32);
    assert_eq!(d_size(&format!("{x}_publickeybytes")), PKB);
    assert_eq!(d_size(&format!("{x}_secretkeybytes")), SKB);
    assert_eq!(d_size(&format!("{x}_beforenmbytes")), KB);
    assert_eq!(d_size(&format!("{x}_noncebytes")), NB);
    assert_eq!(d_size(&format!("{x}_macbytes")), MB);
    assert_eq!(d_size(&format!("{x}_sealbytes")), SEALB);
    assert_eq!(d_size(&format!("{x}_messagebytes_max")) as u64, MSGMAX);

    // crypto_box_primitive()
    let (c, r) = unsafe { pair::<PrimFn>("crypto_box_primitive") };
    let (sc, sr) = unsafe { (CStr::from_ptr(c()), CStr::from_ptr(r())) };
    assert_eq!(sc, sr, "crypto_box_primitive() differs");
    assert_eq!(sc.to_str().unwrap(), "curve25519xsalsa20poly1305");
}

/// CONFIGS 213: the xchacha20 primitive has **no** plain `_afternm` /
/// `_open_afternm` and **no** padded NaCl API — those symbols must be absent
/// from BOTH libraries (a Rust build that exported them would be wrong too).
#[test]
fn cfg213_xchacha_has_no_afternm_or_padded_api() {
    init_both();
    let x = "crypto_box_curve25519xchacha20poly1305";
    for suffix in ["_afternm", "_open_afternm", "_zerobytes", "_boxzerobytes", "_open", ""] {
        assert_absent(&format!("{x}{suffix}"));
    }
    // …while the xsalsa20 primitive DOES have them.
    for name in [
        "crypto_box_curve25519xsalsa20poly1305",
        "crypto_box_curve25519xsalsa20poly1305_open",
        "crypto_box_curve25519xsalsa20poly1305_afternm",
        "crypto_box_curve25519xsalsa20poly1305_open_afternm",
        "crypto_box_curve25519xsalsa20poly1305_zerobytes",
        "crypto_box_curve25519xsalsa20poly1305_boxzerobytes",
    ] {
        let (_c, _r) = unsafe { pair::<SizeFn>(name) }; // panics if either lacks it
    }
}

// ============================================================================
// CONFIGS 202 / 211 — key generation
// ============================================================================

/// CONFIGS 202 + 211 (deterministic half): `_seed_keypair` is
/// `sk = SHA-512(seed)[0..32)` followed by `scalarmult_base`, so it is directly
/// byte-comparable with no RNG shim. 128 seeds per primitive.
#[test]
fn cfg202_seed_keypair_deterministic() {
    init_both();
    let (csha, rsha) = unsafe { pair::<Sha512Fn>("crypto_hash_sha512") };
    let (cbase, rbase) = unsafe { pair::<ScalarmultBaseFn>("crypto_scalarmult_curve25519_base") };
    let mut rng = Rng::new(SEED ^ 202);
    let mut seeds: Vec<[u8; 32]> = vec![[0u8; 32], [0xffu8; 32]];
    {
        let mut inc = [0u8; 32];
        for (i, b) in inc.iter_mut().enumerate() {
            *b = i as u8;
        }
        seeds.push(inc);
    }
    for _ in 0..125 {
        let mut s = [0u8; 32];
        rng.fill(&mut s);
        seeds.push(s);
    }
    assert_eq!(seeds.len(), 128);

    let mut n = 0usize;
    for name in [
        "crypto_box_seed_keypair",
        "crypto_box_curve25519xsalsa20poly1305_seed_keypair",
        "crypto_box_curve25519xchacha20poly1305_seed_keypair",
    ] {
        let (fc, fr) = unsafe { pair::<SeedKeypairFn>(name) };
        for seed in &seeds {
            let mut pc = Out::new(PKB);
            let mut sc = Out::new(SKB);
            let mut pr = Out::new(PKB);
            let mut sr = Out::new(SKB);
            let rc = unsafe { fc(pc.ptr(), sc.ptr(), seed.as_ptr()) };
            let rr = unsafe { fr(pr.ptr(), sr.ptr(), seed.as_ptr()) };
            let what = format!("{name} seed={}", hexs(seed));
            assert_eq!(rc, rr, "{what}: return differs (C={rc} rust={rr})");
            assert_eq!(rc, 0, "{what}: C returned {rc}");
            assert_eq_bytes(&format!("{what} pk"), &pc.v, &pr.v);
            assert_eq_bytes(&format!("{what} sk"), &sc.v, &sr.v);
            guard_intact(&what, "C", &pc);
            guard_intact(&what, "C", &sc);
            guard_intact(&what, "rust", &pr);
            guard_intact(&what, "rust", &sr);

            // sk == SHA-512(seed)[0..32)
            let mut hc = Out::new(64);
            let mut hr = Out::new(64);
            unsafe {
                csha(hc.ptr(), seed.as_ptr(), 32);
                rsha(hr.ptr(), seed.as_ptr(), 32);
            }
            assert_eq_bytes(&format!("{what} sha512"), &hc.v, &hr.v);
            assert_eq_bytes(&format!("{what}: sk != SHA-512(seed)[0..32)"), &hc.body()[..32], sc.body());

            // pk == scalarmult_base(sk)
            let mut bc = Out::new(PKB);
            let mut br = Out::new(PKB);
            let a = unsafe { cbase(bc.ptr(), sc.body().as_ptr()) };
            let b = unsafe { rbase(br.ptr(), sc.body().as_ptr()) };
            assert_eq!(a, b, "{what}: scalarmult_base return differs");
            assert_eq_bytes(&format!("{what} base"), &bc.v, &br.v);
            assert_eq_bytes(&format!("{what}: pk != scalarmult_base(sk)"), bc.body(), pc.body());
            n += 1;
        }
    }
    assert!(n >= 64, "only {n} iterations");
}

/// CONFIGS 202 + 211 (**[RNG]** half): `_keypair` consumes 32 bytes of
/// `randombytes_buf` for the secret key, then `scalarmult_base`. Driven with the
/// injected deterministic RNG, re-seeded before every paired call.
#[test]
fn cfg202_keypair_rng() {
    let _g = rng_lock();
    init_both();
    install_det_rng(false);
    let (cbase, rbase) = unsafe { pair::<ScalarmultBaseFn>("crypto_scalarmult_curve25519_base") };

    let mut n = 0usize;
    for name in [
        "crypto_box_keypair",
        "crypto_box_curve25519xsalsa20poly1305_keypair",
        "crypto_box_curve25519xchacha20poly1305_keypair",
    ] {
        let (fc, fr) = unsafe { pair::<KeypairFn>(name) };
        for i in 0..32usize {
            let skip = i * 7; // a different stream offset (=> different key) each time
            let want_sk = rng_peek32(skip);

            rng_seek(skip);
            let mut pc = Out::new(PKB);
            let mut sc = Out::new(SKB);
            let mut pr = Out::new(PKB);
            let mut sr = Out::new(SKB);
            let rc = unsafe { fc(pc.ptr(), sc.ptr()) };
            let rr = unsafe { fr(pr.ptr(), sr.ptr()) };
            let what = format!("{name} rngskip={skip}");
            assert_eq!(rc, rr, "{what}: return differs (C={rc} rust={rr})");
            assert_eq!(rc, 0, "{what}: C returned {rc}");
            assert_eq_bytes(&format!("{what} pk"), &pc.v, &pr.v);
            assert_eq_bytes(&format!("{what} sk"), &sc.v, &sr.v);
            guard_intact(&what, "C", &pc);
            guard_intact(&what, "C", &sc);
            guard_intact(&what, "rust", &pr);
            guard_intact(&what, "rust", &sr);
            assert_eq_bytes(
                &format!("{what}: sk is not the next 32 RNG bytes"),
                &want_sk,
                sc.body(),
            );
            let mut bc = Out::new(PKB);
            let mut br = Out::new(PKB);
            let a = unsafe { cbase(bc.ptr(), sc.body().as_ptr()) };
            let b = unsafe { rbase(br.ptr(), sc.body().as_ptr()) };
            assert_eq!(a, b, "{what}: scalarmult_base return differs");
            assert_eq_bytes(&format!("{what} base"), &bc.v, &br.v);
            assert_eq_bytes(&format!("{what}: pk != scalarmult_base(sk)"), bc.body(), pc.body());
            n += 1;
        }
    }
    assert!(n >= 64, "only {n} iterations");
    restore_sysrandom();
}

/// CONFIGS 211: the two `_seed_keypair` / `_keypair` implementations are
/// identical across the two primitives (both are plain curve25519), which is a
/// property the C guarantees by construction and the Rust must not break.
#[test]
fn cfg211_xchacha_keypairs() {
    init_both();
    let mut rng = Rng::new(SEED ^ 211);
    for i in 0..80 {
        let mut seed = [0u8; 32];
        rng.fill(&mut seed);
        seed[31] ^= i as u8;
        let a = kp(&XSALSA, &seed);
        let b = kp(&XCHACHA, &seed);
        let c = kp(&XSALSA_P, &seed);
        assert_eq_bytes("xsalsa20 vs xchacha20 seed_keypair pk", &a.pk, &b.pk);
        assert_eq_bytes("xsalsa20 vs xchacha20 seed_keypair sk", &a.sk, &b.sk);
        assert_eq_bytes("generic vs primitive seed_keypair pk", &a.pk, &c.pk);
        assert_eq_bytes("generic vs primitive seed_keypair sk", &a.sk, &c.sk);
    }
}

// ============================================================================
// CONFIGS 205 / 212 / 215 — _beforenm
// ============================================================================

/// CONFIGS 205 + 212: `_beforenm` is
/// `hsalsa20(scalarmult(sk,pk), zero[16])` for the xsalsa20 primitive and
/// **hchacha20** for the xchacha20 one. Recomputed from the two lower-level
/// primitives through both libraries and compared.
#[test]
fn cfg205_cfg212_beforenm_construction() {
    init_both();
    let (csm, rsm) = unsafe { pair::<Arg3Fn>("crypto_scalarmult_curve25519") };
    let zero = [0u8; 16];
    let mut n = 0usize;

    for prim in [&XSALSA, &XSALSA_P, &XCHACHA] {
        let (kps, _) = pool(prim, 6);
        let (ccore, rcore) = unsafe { pair::<CoreFn>(prim.core) };
        for a in 0..kps.len() {
            for b in 0..kps.len() {
                let (rc, k) = d_beforenm(prim.beforenm, &kps[b].pk, &kps[a].sk, prim.tag);
                assert_eq!(rc, 0, "beforenm on valid keys must succeed");

                // s = scalarmult(sk_a, pk_b)
                let mut sc = Out::new(32);
                let mut sr = Out::new(32);
                let x = unsafe { csm(sc.ptr(), kps[a].sk.as_ptr(), kps[b].pk.as_ptr()) };
                let y = unsafe { rsm(sr.ptr(), kps[a].sk.as_ptr(), kps[b].pk.as_ptr()) };
                assert_eq!(x, y, "scalarmult return differs");
                assert_eq!(x, 0, "scalarmult on valid keys must succeed");
                assert_eq_bytes("scalarmult output", &sc.v, &sr.v);

                // k' = core(zero[16], s)
                let mut hc = Out::new(32);
                let mut hr = Out::new(32);
                let x = unsafe {
                    ccore(hc.ptr(), zero.as_ptr(), sc.body().as_ptr(), std::ptr::null())
                };
                let y = unsafe {
                    rcore(hr.ptr(), zero.as_ptr(), sc.body().as_ptr(), std::ptr::null())
                };
                assert_eq!(x, y, "{} return differs", prim.core);
                assert_eq_bytes(&format!("{} output", prim.core), &hc.v, &hr.v);
                assert_eq_bytes(
                    &format!(
                        "{} [{}] != {}(zero16, scalarmult(sk,pk))",
                        prim.beforenm, prim.tag, prim.core
                    ),
                    hc.body(),
                    &k,
                );
                n += 1;
            }
        }
    }
    assert!(n >= 64, "only {n} iterations");
}

/// CONFIGS 215: ECDH agreement — A→B and B→A produce the same `_beforenm` key,
/// for BOTH primitives, in BOTH libraries.
#[test]
fn cfg215_ecdh_agreement() {
    init_both();
    let mut n = 0usize;
    for prim in [&XSALSA, &XCHACHA, &XSALSA_P] {
        let (kps, _) = pool(prim, 8);
        for a in 0..kps.len() {
            for b in 0..kps.len() {
                let (ra, ka) = d_beforenm(prim.beforenm, &kps[b].pk, &kps[a].sk, prim.tag);
                let (rb, kb) = d_beforenm(prim.beforenm, &kps[a].pk, &kps[b].sk, prim.tag);
                assert_eq!(ra, 0, "beforenm A->B failed");
                assert_eq!(rb, 0, "beforenm B->A failed");
                assert_eq_bytes(
                    &format!(
                        "{} [{}] ECDH disagreement A={} B={}",
                        prim.beforenm,
                        prim.tag,
                        hexs(&kps[a].pk),
                        hexs(&kps[b].pk)
                    ),
                    &ka,
                    &kb,
                );
                n += 1;
            }
        }
    }
    assert!(n >= 64, "only {n} iterations");

    // …and the two primitives must NOT agree with each other (different core).
    let (kps, _) = pool(&XSALSA, 2);
    let (_, ks) = d_beforenm(XSALSA.beforenm, &kps[1].pk, &kps[0].sk, "xsalsa20");
    let (_, kx) = d_beforenm(XCHACHA.beforenm, &kps[1].pk, &kps[0].sk, "xchacha20");
    assert_ne!(ks, kx, "hsalsa20 and hchacha20 beforenm keys must differ");
}

// ============================================================================
// CONFIGS 203 / 204 / 205 / 212 / 213 — easy & detached, both primitives
// ============================================================================

/// CONFIGS 203 (+212 for xchacha20): `_easy` / `_open_easy` over
/// `mlen ∈ {0,1,31,32,33,64,1000}`, full round-trip, plus MAC/ciphertext tamper
/// rejection.
#[test]
fn cfg203_easy_open_easy() {
    init_both();
    let mut rng = Rng::new(SEED ^ 203);
    let mut n = 0usize;
    for prim in [&XSALSA, &XCHACHA] {
        let (kps, nonces) = pool(prim, 4);
        for &mlen in MLENS {
            for a in 0..kps.len() {
                for (ni, nn) in nonces.iter().enumerate() {
                    let b = (a + 1 + ni) % kps.len();
                    let m = msg(mlen, &mut rng);
                    let (rc, c) = d_easy(prim.easy, mlen + MB, &m, nn, &kps[b].pk, &kps[a].sk, prim.tag);
                    assert_eq!(rc, 0, "{} must succeed on valid keys", prim.easy);
                    assert_eq!(c.len(), mlen + MB);

                    // round-trip with the recipient's key
                    let (ro, p) =
                        d_open_easy(prim.open_easy, mlen, &c, nn, &kps[a].pk, &kps[b].sk, prim.tag);
                    assert_eq!(ro, 0, "{} round-trip failed", prim.open_easy);
                    assert_eq_bytes(
                        &format!("{} [{}] round-trip mlen={mlen}", prim.open_easy, prim.tag),
                        &m,
                        &p,
                    );
                    // and with the sender's own view (ECDH symmetry)
                    let (ro2, p2) =
                        d_open_easy(prim.open_easy, mlen, &c, nn, &kps[b].pk, &kps[a].sk, prim.tag);
                    assert_eq!(ro2, 0, "{} sender-side open failed", prim.open_easy);
                    assert_eq_bytes("sender-side open", &m, &p2);
                    n += 1;
                }
            }
        }

        // tamper: every MAC byte, and every ciphertext byte for a short message
        let (kps2, nonces2) = pool(prim, 2);
        let nn = &nonces2[3];
        let m = msg(48, &mut rng);
        let (rc, c) = d_easy(prim.easy, 48 + MB, &m, nn, &kps2[1].pk, &kps2[0].sk, prim.tag);
        assert_eq!(rc, 0);
        for i in 0..c.len() {
            let mut bad = c.clone();
            bad[i] ^= 0x01;
            let (r, _) =
                d_open_easy(prim.open_easy, 48, &bad, nn, &kps2[0].pk, &kps2[1].sk, prim.tag);
            assert_eq!(r, -1, "{} accepted a box with byte {i} flipped", prim.open_easy);
        }
        // wrong nonce / wrong key must also be rejected
        let mut bad_n = nn.clone();
        bad_n[0] ^= 0x80;
        let (r, _) =
            d_open_easy(prim.open_easy, 48, &c, &bad_n, &kps2[0].pk, &kps2[1].sk, prim.tag);
        assert_eq!(r, -1, "{} accepted a wrong nonce", prim.open_easy);
        let (r, _) =
            d_open_easy(prim.open_easy, 48, &c, nn, &kps2[1].pk, &kps2[1].sk, prim.tag);
        assert_eq!(r, -1, "{} accepted a wrong public key", prim.open_easy);
    }
    assert!(n >= 64, "only {n} iterations");
}

/// CONFIGS 204 (+212): `_detached` / `_open_detached`, separate MAC, same
/// `mlen` set; includes the `m == NULL` verify-only mode (requirement 7) and the
/// cross-check that `_easy` output == `mac || detached-ciphertext`.
#[test]
fn cfg204_detached_open_detached() {
    init_both();
    let mut rng = Rng::new(SEED ^ 204);
    let mut n = 0usize;
    for prim in [&XSALSA, &XCHACHA] {
        let (kps, nonces) = pool(prim, 4);
        for &mlen in MLENS {
            for a in 0..kps.len() {
                for (ni, nn) in nonces.iter().enumerate() {
                    let b = (a + 1 + ni) % kps.len();
                    let m = msg(mlen, &mut rng);
                    let (rc, c, mac) =
                        d_detached(prim.detached, &m, nn, &kps[b].pk, &kps[a].sk, prim.tag);
                    assert_eq!(rc, 0, "{} must succeed", prim.detached);
                    assert_eq!(c.len(), mlen);
                    assert_eq!(mac.len(), MB);

                    // `_easy` is exactly mac || c
                    let (re, ce) =
                        d_easy(prim.easy, mlen + MB, &m, nn, &kps[b].pk, &kps[a].sk, prim.tag);
                    assert_eq!(re, 0);
                    assert_eq_bytes("_easy prefix is the MAC", &mac, &ce[..MB]);
                    assert_eq_bytes("_easy suffix is the detached ciphertext", &c, &ce[MB..]);

                    // round-trip
                    let (ro, p) = d_open_detached(
                        prim.open_detached, &c, &mac, nn, &kps[a].pk, &kps[b].sk, false, prim.tag,
                    );
                    assert_eq!(ro, 0, "{} round-trip failed", prim.open_detached);
                    assert_eq_bytes("detached round-trip", &m, &p);

                    // requirement 7: m == NULL => verify only, returns 0, writes nothing
                    let (rv, _) = d_open_detached(
                        prim.open_detached, &c, &mac, nn, &kps[a].pk, &kps[b].sk, true, prim.tag,
                    );
                    assert_eq!(rv, 0, "{} verify-only failed", prim.open_detached);

                    // tampered MAC must be rejected in BOTH modes
                    let mut badmac = mac.clone();
                    badmac[ni % MB] ^= 0x40;
                    for m_null in [false, true] {
                        let (rb, _) = d_open_detached(
                            prim.open_detached, &c, &badmac, nn, &kps[a].pk, &kps[b].sk, m_null,
                            prim.tag,
                        );
                        assert_eq!(rb, -1, "{} accepted a forged MAC", prim.open_detached);
                    }
                    n += 1;
                }
            }
        }
    }
    assert!(n >= 64, "only {n} iterations");
}

/// CONFIGS 205 (+213): `_beforenm` then `_easy_afternm` / `_open_easy_afternm`.
/// The precomputed key is reused across many messages and must produce exactly
/// the same bytes as the non-precomputed `_easy`.
#[test]
fn cfg205_beforenm_easy_afternm() {
    init_both();
    let mut rng = Rng::new(SEED ^ 205);
    let mut n = 0usize;
    for prim in [&XSALSA, &XCHACHA] {
        let (kps, nonces) = pool(prim, 4);
        for a in 0..kps.len() {
            let b = (a + 1) % kps.len();
            let (rk, k) = d_beforenm(prim.beforenm, &kps[b].pk, &kps[a].sk, prim.tag);
            assert_eq!(rk, 0);
            for &mlen in MLENS {
                for nn in nonces.iter() {
                    let m = msg(mlen, &mut rng);
                    let (rc, c) = d_afternm(prim.easy_afternm, mlen + MB, &m, nn, &k, prim.tag);
                    assert_eq!(rc, 0, "{} must succeed", prim.easy_afternm);

                    // identical to the non-precomputed path
                    let (re, ce) =
                        d_easy(prim.easy, mlen + MB, &m, nn, &kps[b].pk, &kps[a].sk, prim.tag);
                    assert_eq!(re, 0);
                    assert_eq_bytes(
                        &format!("{} != {} mlen={mlen}", prim.easy_afternm, prim.easy),
                        &ce,
                        &c,
                    );

                    let (ro, p) = d_open_afternm(prim.open_easy_afternm, mlen, &c, nn, &k, prim.tag);
                    assert_eq!(ro, 0, "{} round-trip failed", prim.open_easy_afternm);
                    assert_eq_bytes("afternm round-trip", &m, &p);

                    // forged MAC rejected
                    let mut bad = c.clone();
                    bad[mlen % MB] ^= 0x11;
                    let (rb, _) =
                        d_open_afternm(prim.open_easy_afternm, mlen, &bad, nn, &k, prim.tag);
                    assert_eq!(rb, -1, "{} accepted a forgery", prim.open_easy_afternm);
                    n += 1;
                }
            }
        }
    }
    assert!(n >= 64, "only {n} iterations");
}

/// CONFIGS 213: the xchacha20 precomputation shapes
/// (`_beforenm` + `_easy_afternm` / `_open_easy_afternm` / `_detached_afternm` /
/// `_open_detached_afternm`) as a self-contained round-trip matrix.
#[test]
fn cfg213_xchacha_afternm_shapes() {
    init_both();
    let prim = &XCHACHA;
    let mut rng = Rng::new(SEED ^ 213);
    let (kps, nonces) = pool(prim, 4);
    let mut n = 0usize;
    for a in 0..kps.len() {
        let b = (a + 2) % kps.len();
        let (rk, k) = d_beforenm(prim.beforenm, &kps[b].pk, &kps[a].sk, prim.tag);
        assert_eq!(rk, 0);
        for &mlen in OVERLAP_MLENS {
            for nn in nonces.iter().take(3) {
                let m = msg(mlen, &mut rng);
                let (rd, c, mac) = d_detached_afternm(prim.detached_afternm, &m, nn, &k, prim.tag);
                assert_eq!(rd, 0);
                let (re, ce) = d_afternm(prim.easy_afternm, mlen + MB, &m, nn, &k, prim.tag);
                assert_eq!(re, 0);
                assert_eq_bytes("xchacha easy_afternm prefix == mac", &mac, &ce[..MB]);
                assert_eq_bytes("xchacha easy_afternm suffix == c", &c, &ce[MB..]);

                for m_null in [false, true] {
                    let (ro, p) = d_open_detached_afternm(
                        prim.open_detached_afternm, &c, &mac, nn, &k, m_null, prim.tag,
                    );
                    assert_eq!(ro, 0, "{} failed", prim.open_detached_afternm);
                    if !m_null {
                        assert_eq_bytes("xchacha open_detached_afternm round-trip", &m, &p);
                    }
                }
                let (ro, p) = d_open_afternm(prim.open_easy_afternm, mlen, &ce, nn, &k, prim.tag);
                assert_eq!(ro, 0);
                assert_eq_bytes("xchacha open_easy_afternm round-trip", &m, &p);
                n += 1;
            }
        }
    }
    assert!(n >= 64, "only {n} iterations");
}

/// CONFIGS 212: xchacha20 `_easy` / `_open_easy` / `_detached` /
/// `_open_detached` must NOT interoperate with the xsalsa20 primitive (different
/// AEAD), and both libraries must reject in exactly the same way.
#[test]
fn cfg212_xchacha_easy_detached() {
    init_both();
    let mut rng = Rng::new(SEED ^ 212);
    let (kps, nonces) = pool(&XCHACHA, 3);
    let mut n = 0usize;
    for &mlen in MLENS {
        for (ni, nn) in nonces.iter().enumerate() {
            for a in 0..kps.len() {
                let b = (a + 1 + ni) % kps.len();
                let m = msg(mlen, &mut rng);
                let (rx, cx) =
                    d_easy(XCHACHA.easy, mlen + MB, &m, nn, &kps[b].pk, &kps[a].sk, "xchacha20");
                let (rs, cs) =
                    d_easy(XSALSA.easy, mlen + MB, &m, nn, &kps[b].pk, &kps[a].sk, "xsalsa20");
                assert_eq!((rx, rs), (0, 0));
                assert_ne!(cx, cs, "xchacha20 and xsalsa20 boxes must differ (mlen={mlen})");

                // cross-open must fail: the two primitives are not interoperable
                let (r, _) =
                    d_open_easy(XSALSA.open_easy, mlen, &cx, nn, &kps[a].pk, &kps[b].sk, "cross");
                assert_eq!(r, -1, "xsalsa20 opened an xchacha20 box");
                let (r, _) =
                    d_open_easy(XCHACHA.open_easy, mlen, &cs, nn, &kps[a].pk, &kps[b].sk, "cross");
                assert_eq!(r, -1, "xchacha20 opened an xsalsa20 box");

                let (rd, c, mac) =
                    d_detached(XCHACHA.detached, &m, nn, &kps[b].pk, &kps[a].sk, "xchacha20");
                assert_eq!(rd, 0);
                let (ro, p) = d_open_detached(
                    XCHACHA.open_detached, &c, &mac, nn, &kps[a].pk, &kps[b].sk, false,
                    "xchacha20",
                );
                assert_eq!(ro, 0);
                assert_eq_bytes("xchacha detached round-trip", &m, &p);
                n += 1;
            }
        }
    }
    assert!(n >= 64, "only {n} iterations");
}

// ============================================================================
// CONFIGS 206 — _detached_afternm / _open_detached_afternm overlap axis
// ============================================================================

/// The four overlap shapes of `crypto_secretbox_detached`: `(m_off, c_off)`
/// pairs inside one big buffer, plus the two non-overlapping boundary cases
/// (`|c - m| == mlen`).
fn overlap_shapes(mlen: usize) -> Vec<(usize, usize, String)> {
    const BASE: usize = 512;
    let mut v = vec![
        (0usize, 2048usize, "disjoint".to_string()),
        (BASE, BASE, "in-place".to_string()),
    ];
    let mut ds: Vec<usize> = vec![1, 7, 16, 31, 32, 63];
    if mlen > 0 {
        ds.push(mlen - 1);
        ds.push(mlen); // boundary: NOT an overlap (diff == mlen)
    }
    ds.push(mlen + 1);
    ds.sort_unstable();
    ds.dedup();
    for d in ds {
        if d == 0 {
            continue;
        }
        v.push((BASE, BASE + d, format!("partial-forward d={d}")));
        v.push((BASE + d, BASE, format!("partial-backward d={d}")));
    }
    v
}

const BIGN: usize = 4096;

/// CONFIGS 206: `_detached_afternm` is a literal alias of
/// `crypto_secretbox_detached`, so it inherits the 4-way overlap axis
/// (disjoint / in-place / partial-forward / partial-backward) and the
/// `mlen0 = min(mlen, 32)` first-block axis. Exercised with raw pointers into
/// one big 0xAA-prefilled buffer; the FULL buffer is compared.
#[test]
fn cfg206_detached_afternm_overlap() {
    init_both();
    let mut rng = Rng::new(SEED ^ 206);
    let mut n = 0usize;
    for prim in [&XSALSA, &XCHACHA] {
        let (kps, nonces) = pool(prim, 2);
        let (rk, k) = d_beforenm(prim.beforenm, &kps[1].pk, &kps[0].sk, prim.tag);
        assert_eq!(rk, 0);
        let (fc, fr) = unsafe { pair::<DetachedAfternmFn>(prim.detached_afternm) };
        for &mlen in OVERLAP_MLENS {
            let m = msg(mlen, &mut rng);
            for nn in nonces.iter().take(2) {
                // reference: fully disjoint buffers
                let (_, cref, macref) = d_detached_afternm(prim.detached_afternm, &m, nn, &k, prim.tag);

                for (moff, coff, label) in overlap_shapes(mlen) {
                    assert!(moff + mlen < BIGN && coff + mlen < BIGN);
                    let mut bc = vec![FILL; BIGN];
                    bc[moff..moff + mlen].copy_from_slice(&m);
                    let mut br = bc.clone();
                    let mut ac = Out::new(MB);
                    let mut ar = Out::new(MB);
                    let rc = unsafe {
                        fc(
                            bc.as_mut_ptr().add(coff),
                            ac.ptr(),
                            bc.as_ptr().add(moff),
                            mlen as u64,
                            nn.as_ptr(),
                            k.as_ptr(),
                        )
                    };
                    let rr = unsafe {
                        fr(
                            br.as_mut_ptr().add(coff),
                            ar.ptr(),
                            br.as_ptr().add(moff),
                            mlen as u64,
                            nn.as_ptr(),
                            k.as_ptr(),
                        )
                    };
                    let what = format!(
                        "{} [{}] {label} mlen={mlen} moff={moff} coff={coff}",
                        prim.detached_afternm, prim.tag
                    );
                    assert_eq!(rc, rr, "{what}: return differs (C={rc} rust={rr})");
                    assert_eq!(rc, 0, "{what}: C returned {rc}");
                    assert_eq_bytes(&format!("{what} big buffer"), &bc, &br);
                    assert_eq_bytes(&format!("{what} mac"), &ac.v, &ar.v);
                    guard_intact(&what, "C", &ac);
                    guard_intact(&what, "rust", &ar);
                    // untouched tail of the big buffer
                    let hi = (moff + mlen).max(coff + mlen);
                    assert!(
                        bc[hi..].iter().all(|&x| x == FILL),
                        "{what}: wrote past byte {hi}"
                    );
                    // the MAC never depends on the buffer layout
                    assert_eq_bytes(&format!("{what}: MAC changed with overlap"), &macref, ac.body());
                    // and neither does the ciphertext
                    assert_eq_bytes(
                        &format!("{what}: ciphertext changed with overlap"),
                        &cref,
                        &bc[coff..coff + mlen],
                    );
                    n += 1;
                }
            }
        }
    }
    assert!(n >= 64, "only {n} iterations");
}

/// CONFIGS 206, decrypt direction: `_open_detached_afternm` has the same
/// `memmove` overlap handling and the same `mlen0` first-block split.
#[test]
fn cfg206_open_detached_afternm_overlap() {
    init_both();
    let mut rng = Rng::new(SEED ^ 0x206);
    let mut n = 0usize;
    for prim in [&XSALSA, &XCHACHA] {
        let (kps, nonces) = pool(prim, 2);
        let (rk, k) = d_beforenm(prim.beforenm, &kps[1].pk, &kps[0].sk, prim.tag);
        assert_eq!(rk, 0);
        let (fc, fr) = unsafe { pair::<OpenDetachedAfternmFn>(prim.open_detached_afternm) };
        for &mlen in OVERLAP_MLENS {
            let m = msg(mlen, &mut rng);
            for nn in nonces.iter().take(2) {
                let (rd, c, mac) = d_detached_afternm(prim.detached_afternm, &m, nn, &k, prim.tag);
                assert_eq!(rd, 0);
                for (moff, coff, label) in overlap_shapes(mlen) {
                    let mut bc = vec![FILL; BIGN];
                    bc[coff..coff + mlen].copy_from_slice(&c);
                    let mut br = bc.clone();
                    let rc = unsafe {
                        fc(
                            bc.as_mut_ptr().add(moff),
                            bc.as_ptr().add(coff),
                            mac.as_ptr(),
                            mlen as u64,
                            nn.as_ptr(),
                            k.as_ptr(),
                        )
                    };
                    let rr = unsafe {
                        fr(
                            br.as_mut_ptr().add(moff),
                            br.as_ptr().add(coff),
                            mac.as_ptr(),
                            mlen as u64,
                            nn.as_ptr(),
                            k.as_ptr(),
                        )
                    };
                    let what = format!(
                        "{} [{}] {label} mlen={mlen} moff={moff} coff={coff}",
                        prim.open_detached_afternm, prim.tag
                    );
                    assert_eq!(rc, rr, "{what}: return differs (C={rc} rust={rr})");
                    assert_eq!(rc, 0, "{what}: C returned {rc}");
                    assert_eq_bytes(&format!("{what} big buffer"), &bc, &br);
                    let hi = (moff + mlen).max(coff + mlen);
                    assert!(bc[hi..].iter().all(|&x| x == FILL), "{what}: wrote past byte {hi}");
                    assert_eq_bytes(
                        &format!("{what}: plaintext wrong"),
                        &m,
                        &bc[moff..moff + mlen],
                    );
                    n += 1;
                }
            }
        }
    }
    assert!(n >= 64, "only {n} iterations");
}

/// CONFIGS 206: prove the alias — `_detached_afternm` and
/// `_open_detached_afternm` produce byte-identical results to
/// `crypto_secretbox[_xchacha20poly1305]_detached` / `_open_detached`.
#[test]
fn cfg206_afternm_aliases_secretbox() {
    init_both();
    let mut rng = Rng::new(SEED ^ 0x2060);
    let mut n = 0usize;
    for prim in [&XSALSA, &XCHACHA] {
        let (kps, nonces) = pool(prim, 2);
        let (rk, k) = d_beforenm(prim.beforenm, &kps[1].pk, &kps[0].sk, prim.tag);
        assert_eq!(rk, 0);
        for &mlen in OVERLAP_MLENS {
            for nn in nonces.iter() {
                let m = msg(mlen, &mut rng);
                let (r1, c1, a1) = d_detached_afternm(prim.detached_afternm, &m, nn, &k, prim.tag);
                let (r2, c2, a2) = d_detached_afternm(prim.secretbox_detached, &m, nn, &k, prim.tag);
                assert_eq!(r1, r2, "{} is not an alias of {}", prim.detached_afternm, prim.secretbox_detached);
                assert_eq_bytes("alias ciphertext", &c1, &c2);
                assert_eq_bytes("alias mac", &a1, &a2);

                for m_null in [false, true] {
                    let (o1, p1) = d_open_detached_afternm(
                        prim.open_detached_afternm, &c1, &a1, nn, &k, m_null, prim.tag,
                    );
                    let (o2, p2) = d_open_detached_afternm(
                        prim.secretbox_open_detached, &c1, &a1, nn, &k, m_null, prim.tag,
                    );
                    assert_eq!(o1, o2, "{} is not an alias", prim.open_detached_afternm);
                    assert_eq_bytes("alias plaintext", &p1, &p2);
                }
                n += 1;
            }
        }
    }
    assert!(n >= 64, "only {n} iterations");
}

// ============================================================================
// CONFIGS 207 / 208 / 209 — deprecated padded NaCl API (xsalsa20 only)
// ============================================================================

/// Build a padded NaCl plaintext: `mlen` bytes with 32 leading zeros.
fn padded_msg(mlen: usize, rng: &mut Rng) -> Vec<u8> {
    assert!(mlen >= ZB);
    let mut v = msg(mlen, rng);
    for b in v[..ZB].iter_mut() {
        *b = 0;
    }
    v
}

/// CONFIGS 207: `crypto_box_afternm` / `crypto_box_open_afternm` (deprecated,
/// padded). `mlen ∈ {32,33,64,96,1000}`; output has 16 leading zero bytes,
/// recovered plaintext has 32 leading zero bytes.
#[test]
fn cfg207_afternm_padded() {
    init_both();
    let mut rng = Rng::new(SEED ^ 207);
    let (kps, nonces) = pool(&XSALSA, 4);
    let mut n = 0usize;
    for a in 0..kps.len() {
        let b = (a + 1) % kps.len();
        let (rk, k) = d_beforenm(XSALSA.beforenm, &kps[b].pk, &kps[a].sk, "padded");
        assert_eq!(rk, 0);
        for &mlen in PADDED_MLENS {
            for nn in nonces.iter() {
                let m = padded_msg(mlen, &mut rng);
                let (rc, c) = d_afternm("crypto_box_afternm", mlen, &m, nn, &k, "padded");
                assert_eq!(rc, 0, "crypto_box_afternm(mlen={mlen}) returned {rc}");
                assert_eq!(c.len(), mlen);
                assert!(
                    c[..BZB].iter().all(|&x| x == 0),
                    "crypto_box_afternm must write {BZB} leading zero bytes, got {}",
                    hexs(&c[..BZB])
                );

                // the primitive-level name is a literal wrapper
                let (rp, cp) = d_afternm(
                    "crypto_box_curve25519xsalsa20poly1305_afternm",
                    mlen,
                    &m,
                    nn,
                    &k,
                    "padded",
                );
                assert_eq!(rc, rp);
                assert_eq_bytes("crypto_box_afternm != primitive-level _afternm", &c, &cp);

                let (ro, p) = d_open_afternm("crypto_box_open_afternm", mlen, &c, nn, &k, "padded");
                assert_eq!(ro, 0, "crypto_box_open_afternm(clen={mlen}) returned {ro}");
                assert!(
                    p[..ZB].iter().all(|&x| x == 0),
                    "crypto_box_open_afternm must write {ZB} leading zero bytes"
                );
                assert_eq_bytes("padded afternm round-trip", &m, &p);
                let (ro2, p2) = d_open_afternm(
                    "crypto_box_curve25519xsalsa20poly1305_open_afternm",
                    mlen,
                    &c,
                    nn,
                    &k,
                    "padded",
                );
                assert_eq!(ro, ro2);
                assert_eq_bytes("open_afternm != primitive-level", &p, &p2);

                // forgery
                let mut bad = c.clone();
                bad[BZB + (mlen % MB)] ^= 0x08;
                let (rb, _) = d_open_afternm("crypto_box_open_afternm", mlen, &bad, nn, &k, "padded");
                assert_eq!(rb, -1, "crypto_box_open_afternm accepted a forgery");
                n += 1;
            }
        }
    }
    assert!(n >= 64, "only {n} iterations");
}

/// CONFIGS 208 + 209: the deprecated `crypto_box` / `crypto_box_open` and their
/// primitive-level twins `crypto_box_curve25519xsalsa20poly1305[_open]`.
/// `mlen >= 32` with 32 leading zeros; 16 leading zeros in the output.
#[test]
fn cfg208_209_padded_nacl_api() {
    init_both();
    let mut rng = Rng::new(SEED ^ 208);
    let (kps, nonces) = pool(&XSALSA, 4);
    let mut n = 0usize;
    for a in 0..kps.len() {
        for (ni, nn) in nonces.iter().enumerate() {
            let b = (a + 1 + ni) % kps.len();
            for &mlen in PADDED_MLENS {
                let m = padded_msg(mlen, &mut rng);
                let (rc, c) = d_easy("crypto_box", mlen, &m, nn, &kps[b].pk, &kps[a].sk, "padded");
                assert_eq!(rc, 0, "crypto_box(mlen={mlen}) returned {rc}");
                assert!(
                    c[..BZB].iter().all(|&x| x == 0),
                    "crypto_box must write {BZB} leading zero bytes, got {}",
                    hexs(&c[..BZB])
                );
                let (rp, cp) = d_easy(
                    "crypto_box_curve25519xsalsa20poly1305",
                    mlen,
                    &m,
                    nn,
                    &kps[b].pk,
                    &kps[a].sk,
                    "padded",
                );
                assert_eq!(rc, rp);
                assert_eq_bytes("crypto_box != crypto_box_curve25519xsalsa20poly1305", &c, &cp);

                // the padded ciphertext is the easy one with the MAC moved:
                // c[16..32) is the MAC and c[32..) the stream ciphertext.
                let (re, ce) = d_easy(
                    "crypto_box_easy",
                    mlen - ZB + MB,
                    &m[ZB..],
                    nn,
                    &kps[b].pk,
                    &kps[a].sk,
                    "padded-vs-easy",
                );
                assert_eq!(re, 0);
                assert_eq_bytes("padded MAC != easy MAC", &ce[..MB], &c[BZB..ZB]);
                assert_eq_bytes("padded ciphertext != easy ciphertext", &ce[MB..], &c[ZB..]);

                let (ro, p) =
                    d_open_easy("crypto_box_open", mlen, &c, nn, &kps[a].pk, &kps[b].sk, "padded");
                assert_eq!(ro, 0, "crypto_box_open(clen={mlen}) returned {ro}");
                assert!(
                    p[..ZB].iter().all(|&x| x == 0),
                    "crypto_box_open must write {ZB} leading zero bytes"
                );
                assert_eq_bytes("padded round-trip", &m, &p);
                let (ro2, p2) = d_open_easy(
                    "crypto_box_curve25519xsalsa20poly1305_open",
                    mlen,
                    &c,
                    nn,
                    &kps[a].pk,
                    &kps[b].sk,
                    "padded",
                );
                assert_eq!(ro, ro2);
                assert_eq_bytes("crypto_box_open != primitive-level _open", &p, &p2);

                // every byte of the authenticated region must be covered
                for i in [BZB, BZB + 1, ZB - 1, ZB, ZB + 1, mlen - 1] {
                    if i >= mlen {
                        continue;
                    }
                    let mut bad = c.clone();
                    bad[i] ^= 0x01;
                    let (rb, _) = d_open_easy(
                        "crypto_box_open", mlen, &bad, nn, &kps[a].pk, &kps[b].sk, "padded-tamper",
                    );
                    assert_eq!(rb, -1, "crypto_box_open accepted byte {i} flipped");
                }
                n += 1;
            }
        }
    }
    assert!(n >= 64, "only {n} iterations");
}

// ============================================================================
// CONFIGS 210 / 214 — sealed boxes
// ============================================================================

/// Recompute the sealed-box nonce `BLAKE2b-24(pk1 || pk2)` through both
/// libraries and return the (identical) result.
fn seal_nonce(pk1: &[u8], pk2: &[u8]) -> Vec<u8> {
    let (cf, rf) = unsafe { pair::<GenerichashFn>("crypto_generichash") };
    let mut input = Vec::with_capacity(64);
    input.extend_from_slice(pk1);
    input.extend_from_slice(pk2);
    let mut oc = Out::new(NB);
    let mut or = Out::new(NB);
    let a = unsafe {
        cf(oc.ptr(), NB, input.as_ptr(), input.len() as u64, std::ptr::null(), 0)
    };
    let b = unsafe {
        rf(or.ptr(), NB, input.as_ptr(), input.len() as u64, std::ptr::null(), 0)
    };
    assert_eq!(a, b, "crypto_generichash return differs");
    assert_eq!(a, 0);
    assert_eq_bytes("seal nonce", &oc.v, &or.v);
    oc.body().to_vec()
}

fn seal_body(prim: &Prim, tag_seed: u64) {
    let _g = rng_lock();
    init_both();
    install_det_rng(false);
    let (cbase, rbase) = unsafe { pair::<ScalarmultBaseFn>("crypto_scalarmult_curve25519_base") };
    let mut rng = Rng::new(SEED ^ tag_seed);
    let (kps, _) = pool(prim, 4);

    let mut n = 0usize;
    for &mlen in SEAL_MLENS {
        for (i, recipient) in kps.iter().enumerate() {
            for j in 0..4usize {
                let skip = 11 * (i + 1) + 3 * j + mlen % 5;
                let m = msg(mlen, &mut rng);

                // the ephemeral secret key the RNG is about to hand out
                let esk = rng_peek32(skip);
                let (rc, c) = d_seal(prim.seal, &m, &recipient.pk, skip, prim.tag);
                assert_eq!(rc, 0, "{} must succeed on a valid pk", prim.seal);
                assert_eq!(c.len(), mlen + SEALB);

                // epk == scalarmult_base(esk) and it is prepended verbatim
                let mut ec = Out::new(PKB);
                let mut er = Out::new(PKB);
                let a = unsafe { cbase(ec.ptr(), esk.as_ptr()) };
                let b = unsafe { rbase(er.ptr(), esk.as_ptr()) };
                assert_eq!(a, b);
                assert_eq_bytes("epk", &ec.v, &er.v);
                assert_eq_bytes(
                    &format!("{} [{}]: c[0..32) is not the ephemeral pk", prim.seal, prim.tag),
                    ec.body(),
                    &c[..PKB],
                );

                // nonce == BLAKE2b-24(epk || recipient_pk) and the body is
                // exactly crypto_box_easy(m, nonce, recipient_pk, esk)
                let nonce = seal_nonce(ec.body(), &recipient.pk);
                let (re, ce) =
                    d_easy(prim.easy, mlen + MB, &m, &nonce, &recipient.pk, &esk, "seal-body");
                assert_eq!(re, 0);
                assert_eq_bytes(
                    &format!("{} [{}]: body != easy(m, blake2b24(epk||pk), pk, esk)", prim.seal, prim.tag),
                    &ce,
                    &c[PKB..],
                );

                // round-trip
                let (ro, p) =
                    d_seal_open(prim.seal_open, mlen, &c, &recipient.pk, &recipient.sk, prim.tag);
                assert_eq!(ro, 0, "{} round-trip failed", prim.seal_open);
                assert_eq_bytes("seal round-trip", &m, &p);

                // the wrong recipient must be rejected
                let other = &kps[(i + 1) % kps.len()];
                let (rw, _) =
                    d_seal_open(prim.seal_open, mlen, &c, &other.pk, &other.sk, prim.tag);
                assert_eq!(rw, -1, "{} opened a box for another recipient", prim.seal_open);
                n += 1;
            }
        }
    }
    assert!(n >= 64, "only {n} iterations");
    restore_sysrandom();
}

/// CONFIGS 210: `crypto_box_seal` / `crypto_box_seal_open`. Anonymous sender,
/// ephemeral pk prepended, `nonce = BLAKE2b-24(epk || pk)`, `SEALBYTES == 48`.
#[test]
fn cfg210_seal_seal_open() {
    seal_body(&XSALSA, 210);
}

/// CONFIGS 214: the xchacha20 sealed box.
#[test]
fn cfg214_xchacha_seal() {
    seal_body(&XCHACHA, 214);
}

/// Requirement 6: corrupt the MAC, every ciphertext byte, and the ephemeral pk
/// of a sealed box; both libraries must reject identically.
#[test]
fn cfg210_seal_tamper() {
    let _g = rng_lock();
    init_both();
    install_det_rng(false);
    let mut rng = Rng::new(SEED ^ 0x210);
    for prim in [&XSALSA, &XCHACHA] {
        let (kps, _) = pool(prim, 2);
        let mlen = 40usize;
        let m = msg(mlen, &mut rng);
        let (rc, c) = d_seal(prim.seal, &m, &kps[0].pk, 5, prim.tag);
        assert_eq!(rc, 0);
        let (ro, p) = d_seal_open(prim.seal_open, mlen, &c, &kps[0].pk, &kps[0].sk, prim.tag);
        assert_eq!(ro, 0);
        assert_eq_bytes("seal round-trip", &m, &p);

        // every single byte: the ephemeral pk (0..32) feeds the nonce AND the
        // ECDH, the MAC is 32..48, the ciphertext is 48..
        for i in 0..c.len() {
            let mut bad = c.clone();
            bad[i] ^= 0x01;
            let (r, _) =
                d_seal_open(prim.seal_open, mlen, &bad, &kps[0].pk, &kps[0].sk, prim.tag);
            assert_eq!(
                r, -1,
                "{} accepted a sealed box with byte {i} flipped",
                prim.seal_open
            );
        }
        // truncation by one byte
        let (r, _) = d_seal_open(
            prim.seal_open,
            mlen - 1,
            &c[..c.len() - 1],
            &kps[0].pk,
            &kps[0].sk,
            prim.tag,
        );
        assert_eq!(r, -1, "{} accepted a truncated box", prim.seal_open);
        // replacing the epk with a low-order point
        let mut bad = c.clone();
        bad[..PKB].copy_from_slice(&LOW_ORDER[1]);
        let (r, _) = d_seal_open(prim.seal_open, mlen, &bad, &kps[0].pk, &kps[0].sk, prim.tag);
        assert_eq!(r, -1, "{} accepted a low-order epk", prim.seal_open);
    }
    restore_sysrandom();
}

// ============================================================================
// ERRORS 131 / 146 — _beforenm with a low-order public key
// ============================================================================

/// ERRORS 131 + 146: `crypto_scalarmult_curve25519` rejects the 7 blocklisted
/// low-order encodings (and their bit-7 variants), so `_beforenm` returns -1
/// without writing `k`.
#[test]
fn e131_beforenm_low_order() {
    init_both();
    let bad = low_order_pks();
    let (kps, _) = pool(&XSALSA, 2);
    let mut n = 0usize;
    for prim in [&XSALSA, &XSALSA_P, &XCHACHA] {
        for pk in &bad {
            for sk in [&kps[0].sk, &kps[1].sk, &[0u8; 32], &[0xffu8; 32]] {
                let (rc, k) = d_beforenm(prim.beforenm, pk, sk, prim.tag);
                assert_eq!(
                    rc, -1,
                    "{} [{}] must return -1 for the low-order pk {}",
                    prim.beforenm,
                    prim.tag,
                    hexs(pk)
                );
                assert!(k.iter().all(|&x| x == FILL), "k must be untouched");
                n += 1;
            }
        }
        // sanity: a valid pk succeeds, so the -1 above is really the guard
        let (rc, _) = d_beforenm(prim.beforenm, &kps[1].pk, &kps[0].sk, prim.tag);
        assert_eq!(rc, 0, "{} rejected a VALID pk", prim.beforenm);
    }
    assert!(n >= 64, "only {n} iterations");
}

// ============================================================================
// ERRORS 132 / 135 / 136 / 153 — beforenm failure propagation
// ============================================================================

/// ERRORS 132 / 135 / 136: `crypto_box_detached`, `crypto_box_easy`,
/// `crypto_box_open_detached` (and `crypto_box_open_easy`, `crypto_box`,
/// `crypto_box_open`, which all call `_beforenm` too) return -1 and write
/// nothing when the public key is a low-order point.
#[test]
fn e132_e135_e136_low_order_easy_detached() {
    init_both();
    let bad = low_order_pks();
    let (kps, nonces) = pool(&XSALSA, 2);
    let nn = &nonces[3];
    let mut rng = Rng::new(SEED ^ 132);
    let mut n = 0usize;

    for pk in &bad {
        for &mlen in &[0usize, 1, 32, 64, 1000] {
            let m = msg(mlen, &mut rng);
            // 135: crypto_box_easy
            let (r, c) = d_easy("crypto_box_easy", mlen + MB, &m, nn, pk, &kps[0].sk, "low-order");
            assert_eq!(r, -1, "crypto_box_easy must return -1 for pk={}", hexs(pk));
            assert!(c.iter().all(|&x| x == FILL), "crypto_box_easy wrote output");

            // 132: crypto_box_detached
            let (r, c, mac) = d_detached("crypto_box_detached", &m, nn, pk, &kps[0].sk, "low-order");
            assert_eq!(r, -1, "crypto_box_detached must return -1");
            assert!(c.iter().all(|&x| x == FILL) && mac.iter().all(|&x| x == FILL));

            // 136: crypto_box_open_detached
            let fake_mac = [0u8; MB];
            let fake_c = vec![0u8; mlen];
            for m_null in [false, true] {
                let (r, _) = d_open_detached(
                    "crypto_box_open_detached", &fake_c, &fake_mac, nn, pk, &kps[0].sk, m_null,
                    "low-order",
                );
                assert_eq!(r, -1, "crypto_box_open_detached must return -1");
            }

            // crypto_box_open_easy (clen >= 16 so the length guard is not what fires)
            let fake_box = vec![0u8; mlen + MB];
            let (r, p) = d_open_easy(
                "crypto_box_open_easy", mlen, &fake_box, nn, pk, &kps[0].sk, "low-order",
            );
            assert_eq!(r, -1, "crypto_box_open_easy must return -1");
            assert!(p.iter().all(|&x| x == FILL));

            // deprecated padded API (mlen >= 32 so the length guard is not what fires)
            if mlen >= ZB {
                let pm = padded_msg(mlen, &mut rng);
                let (r, c) = d_easy("crypto_box", mlen, &pm, nn, pk, &kps[0].sk, "low-order");
                assert_eq!(r, -1, "crypto_box must return -1");
                assert!(c.iter().all(|&x| x == FILL));
                let padded_c = vec![0u8; mlen];
                let (r, p) =
                    d_open_easy("crypto_box_open", mlen, &padded_c, nn, pk, &kps[0].sk, "low-order");
                assert_eq!(r, -1, "crypto_box_open must return -1");
                assert!(p.iter().all(|&x| x == FILL));
            }
            n += 1;
        }
    }
    assert!(n >= 64, "only {n} iterations");
}

/// ERRORS 153: the xchacha20 `_detached` / `_open_detached` (and `_easy` /
/// `_open_easy`) propagate the same `_beforenm` failure.
#[test]
fn e153_xchacha_detached_low_order() {
    init_both();
    let bad = low_order_pks();
    let (kps, nonces) = pool(&XCHACHA, 2);
    let nn = &nonces[2];
    let mut rng = Rng::new(SEED ^ 153);
    let mut n = 0usize;
    for pk in &bad {
        for &mlen in &[0usize, 1, 32, 64, 1000] {
            let m = msg(mlen, &mut rng);
            let (r, c, mac) = d_detached(XCHACHA.detached, &m, nn, pk, &kps[0].sk, "low-order");
            assert_eq!(r, -1, "{} must return -1 for pk={}", XCHACHA.detached, hexs(pk));
            assert!(c.iter().all(|&x| x == FILL) && mac.iter().all(|&x| x == FILL));

            let fake_mac = [0u8; MB];
            let fake_c = vec![0u8; mlen];
            for m_null in [false, true] {
                let (r, _) = d_open_detached(
                    XCHACHA.open_detached, &fake_c, &fake_mac, nn, pk, &kps[0].sk, m_null,
                    "low-order",
                );
                assert_eq!(r, -1, "{} must return -1", XCHACHA.open_detached);
            }
            let (r, c) = d_easy(XCHACHA.easy, mlen + MB, &m, nn, pk, &kps[0].sk, "low-order");
            assert_eq!(r, -1, "{} must return -1", XCHACHA.easy);
            assert!(c.iter().all(|&x| x == FILL));
            let fake_box = vec![0u8; mlen + MB];
            let (r, p) =
                d_open_easy(XCHACHA.open_easy, mlen, &fake_box, nn, pk, &kps[0].sk, "low-order");
            assert_eq!(r, -1, "{} must return -1", XCHACHA.open_easy);
            assert!(p.iter().all(|&x| x == FILL));
            n += 1;
        }
    }
    assert!(n >= 64, "only {n} iterations");
}

// ============================================================================
// ERRORS 133 / 134 / 147 / 148 — mlen > MESSAGEBYTES_MAX  =>  sodium_misuse()
// ============================================================================

/// ERRORS 133 + 134: `crypto_box_easy` / `crypto_box_easy_afternm` with
/// `mlen > MESSAGEBYTES_MAX`.
///
/// `MESSAGEBYTES_MAX == SODIUM_SIZE_MAX - 16 == UINT64_MAX - 16`, which is NOT
/// the largest representable `unsigned long long`, so the guard IS reachable:
/// `mlen = UINT64_MAX` (and `UINT64_MAX - 15`) trips it and both libraries must
/// `sodium_misuse()` -> `SIGABRT`. At `mlen == MESSAGEBYTES_MAX` exactly the
/// guard must NOT fire; the call then runs on into a huge `memmove` and dies on
/// a page fault, which the child reports as `Returned(FAULT)`.
#[test]
fn e133_e134_easy_messagebytes_max() {
    init_both();
    // the constant itself, in both libraries
    assert_eq!(d_size("crypto_box_messagebytes_max") as u64, MSGMAX);
    assert_eq!(
        d_size("crypto_box_curve25519xsalsa20poly1305_messagebytes_max") as u64,
        MSGMAX
    );

    let (kps, nonces) = pool(&XSALSA, 2);
    let nn = nonces[3].clone();
    let sc = scratch(4096);
    let pk = kps[1].pk;
    let sk = kps[0].sk;
    let (rk, k) = d_beforenm("crypto_box_beforenm", &pk, &sk, "misuse");
    assert_eq!(rk, 0);

    let np = nn.as_ptr() as usize;
    let kp_ = k.as_ptr() as usize;
    let pkp = pk.as_ptr() as usize;
    let skp = sk.as_ptr() as usize;
    let cp = sc.p as usize;

    for (mlen, want) in [
        (u64::MAX, MISUSE),
        (u64::MAX - 15, MISUSE),
        (MSGMAX, NO_MISUSE),
    ] {
        expect_outcome::<EasyFn, _>(
            &format!("err133 crypto_box_easy mlen={mlen:#x}"),
            "crypto_box_easy",
            move |f: EasyFn| unsafe {
                f(cp as *mut u8, cp as *const u8, mlen, np as *const u8, pkp as *const u8, skp as *const u8) as i64
            },
            want,
        );
        expect_outcome::<AfternmFn, _>(
            &format!("err134 crypto_box_easy_afternm mlen={mlen:#x}"),
            "crypto_box_easy_afternm",
            move |f: AfternmFn| unsafe {
                f(cp as *mut u8, cp as *const u8, mlen, np as *const u8, kp_ as *const u8) as i64
            },
            want,
        );
    }
}

/// ERRORS 147 + 148: the same guard in the xchacha20 primitive.
#[test]
fn e147_e148_xchacha_easy_messagebytes_max() {
    init_both();
    assert_eq!(
        d_size("crypto_box_curve25519xchacha20poly1305_messagebytes_max") as u64,
        MSGMAX
    );
    let (kps, nonces) = pool(&XCHACHA, 2);
    let nn = nonces[3].clone();
    let sc = scratch(4096);
    let pk = kps[1].pk;
    let sk = kps[0].sk;
    let (rk, k) = d_beforenm(XCHACHA.beforenm, &pk, &sk, "misuse");
    assert_eq!(rk, 0);
    let np = nn.as_ptr() as usize;
    let kp_ = k.as_ptr() as usize;
    let pkp = pk.as_ptr() as usize;
    let skp = sk.as_ptr() as usize;
    let cp = sc.p as usize;

    for (mlen, want) in [
        (u64::MAX, MISUSE),
        (u64::MAX - 15, MISUSE),
        (MSGMAX, NO_MISUSE),
    ] {
        expect_outcome::<EasyFn, _>(
            &format!("err147 xchacha _easy mlen={mlen:#x}"),
            XCHACHA.easy,
            move |f: EasyFn| unsafe {
                f(cp as *mut u8, cp as *const u8, mlen, np as *const u8, pkp as *const u8, skp as *const u8) as i64
            },
            want,
        );
        expect_outcome::<AfternmFn, _>(
            &format!("err148 xchacha _easy_afternm mlen={mlen:#x}"),
            XCHACHA.easy_afternm,
            move |f: AfternmFn| unsafe {
                f(cp as *mut u8, cp as *const u8, mlen, np as *const u8, kp_ as *const u8) as i64
            },
            want,
        );
    }
}

/// ERRORS 142 + 151: `_seal` with `mlen > MESSAGEBYTES_MAX` aborts before it
/// even generates the ephemeral keypair.
#[test]
fn e142_e151_seal_messagebytes_max() {
    let _g = rng_lock(); // no other thread may be inside randombytes when we fork
    init_both();
    let (kps, _) = pool(&XSALSA, 1);
    let pk = kps[0].pk;
    let sc = scratch(4096);
    let pkp = pk.as_ptr() as usize;
    let cp = sc.p as usize;

    for name in [XSALSA.seal, XCHACHA.seal] {
        for (mlen, want) in [
            (u64::MAX, MISUSE),
            (u64::MAX - 15, MISUSE),
            (MSGMAX, NO_MISUSE),
        ] {
            expect_outcome::<SealFn, _>(
                &format!("err142/151 {name} mlen={mlen:#x}"),
                name,
                move |f: SealFn| unsafe {
                    f(cp as *mut u8, cp as *const u8, mlen, pkp as *const u8) as i64
                },
                want,
            );
        }
    }
}

// ============================================================================
// ERRORS 137 / 138 / 149 / 150 — clen < MACBYTES
// ============================================================================

/// ERRORS 137 + 138: `crypto_box_open_easy` / `_open_easy_afternm` return -1
/// (without touching `m`) for every `clen < 16`.
#[test]
fn e137_e138_open_easy_clen_too_short() {
    init_both();
    let (kps, nonces) = pool(&XSALSA, 2);
    let (rk, k) = d_beforenm("crypto_box_beforenm", &kps[1].pk, &kps[0].sk, "short");
    assert_eq!(rk, 0);
    let mut rng = Rng::new(SEED ^ 137);
    let mut n = 0usize;
    for clen in 0..MB {
        for nn in nonces.iter() {
            let c = msg(clen, &mut rng);
            let (r, p) =
                d_open_easy("crypto_box_open_easy", 0, &c, nn, &kps[0].pk, &kps[1].sk, "short");
            assert_eq!(r, -1, "crypto_box_open_easy(clen={clen}) must return -1");
            assert!(p.is_empty());
            let (r, _) = d_open_afternm("crypto_box_open_easy_afternm", 0, &c, nn, &k, "short");
            assert_eq!(r, -1, "crypto_box_open_easy_afternm(clen={clen}) must return -1");
            n += 1;
        }
    }
    // clen == 16 is accepted by the length guard (the MAC check then fails)
    let c = vec![0u8; MB];
    let (r, _) = d_open_easy("crypto_box_open_easy", 0, &c, &nonces[0], &kps[0].pk, &kps[1].sk, "boundary");
    assert_eq!(r, -1, "an all-zero MAC must not verify");
    assert!(n >= 64, "only {n} iterations");
}

/// ERRORS 149 + 150: the same guard in the xchacha20 primitive.
#[test]
fn e149_e150_xchacha_open_easy_clen_too_short() {
    init_both();
    let (kps, nonces) = pool(&XCHACHA, 2);
    let (rk, k) = d_beforenm(XCHACHA.beforenm, &kps[1].pk, &kps[0].sk, "short");
    assert_eq!(rk, 0);
    let mut rng = Rng::new(SEED ^ 149);
    let mut n = 0usize;
    for clen in 0..MB {
        for nn in nonces.iter() {
            let c = msg(clen, &mut rng);
            let (r, _) =
                d_open_easy(XCHACHA.open_easy, 0, &c, nn, &kps[0].pk, &kps[1].sk, "short");
            assert_eq!(r, -1, "{}(clen={clen}) must return -1", XCHACHA.open_easy);
            let (r, _) = d_open_afternm(XCHACHA.open_easy_afternm, 0, &c, nn, &k, "short");
            assert_eq!(r, -1, "{}(clen={clen}) must return -1", XCHACHA.open_easy_afternm);
            n += 1;
        }
    }
    assert!(n >= 64, "only {n} iterations");
}

// ============================================================================
// ERRORS 139 — _open_detached_afternm poly1305 mismatch
// ============================================================================

/// ERRORS 139: `crypto_box_open_detached_afternm` (and the xchacha20 twin) with
/// a forged poly1305 tag: -1, `m` untouched, in both verify-only and
/// decrypt modes.
#[test]
fn e139_open_detached_afternm_poly_mismatch() {
    init_both();
    let mut rng = Rng::new(SEED ^ 139);
    let mut n = 0usize;
    for prim in [&XSALSA, &XCHACHA] {
        let (kps, nonces) = pool(prim, 2);
        let (rk, k) = d_beforenm(prim.beforenm, &kps[1].pk, &kps[0].sk, prim.tag);
        assert_eq!(rk, 0);
        for &mlen in &[0usize, 1, 16, 31, 32, 33, 64, 1000] {
            let m = msg(mlen, &mut rng);
            for nn in nonces.iter().take(2) {
                let (rd, c, mac) = d_detached_afternm(prim.detached_afternm, &m, nn, &k, prim.tag);
                assert_eq!(rd, 0);
                // every MAC byte
                for i in 0..MB {
                    let mut bad = mac.clone();
                    bad[i] ^= 1 << (i % 8);
                    for m_null in [false, true] {
                        let (r, _) = d_open_detached_afternm(
                            prim.open_detached_afternm, &c, &bad, nn, &k, m_null, prim.tag,
                        );
                        assert_eq!(
                            r, -1,
                            "{} accepted a MAC with byte {i} flipped",
                            prim.open_detached_afternm
                        );
                        n += 1;
                    }
                }
                // an all-zero MAC, an all-ones MAC and a random MAC
                for bad in [vec![0u8; MB], vec![0xffu8; MB], rng.bytes(MB)] {
                    if bad == mac {
                        continue;
                    }
                    let (r, _) = d_open_detached_afternm(
                        prim.open_detached_afternm, &c, &bad, nn, &k, false, prim.tag,
                    );
                    assert_eq!(r, -1, "{} accepted a forged MAC", prim.open_detached_afternm);
                }
                // a modified ciphertext with the original MAC
                if mlen > 0 {
                    let mut badc = c.clone();
                    badc[mlen - 1] ^= 0x80;
                    let (r, _) = d_open_detached_afternm(
                        prim.open_detached_afternm, &badc, &mac, nn, &k, false, prim.tag,
                    );
                    assert_eq!(r, -1, "{} accepted modified ciphertext", prim.open_detached_afternm);
                }
                // the genuine MAC still verifies
                let (r, p) = d_open_detached_afternm(
                    prim.open_detached_afternm, &c, &mac, nn, &k, false, prim.tag,
                );
                assert_eq!(r, 0);
                assert_eq_bytes("genuine MAC round-trip", &m, &p);
            }
        }
    }
    assert!(n >= 64, "only {n} iterations");
}

// ============================================================================
// ERRORS 140 / 141 — deprecated padded API length floor
// ============================================================================

/// ERRORS 140 + 141: `crypto_box_curve25519xsalsa20poly1305` (= `crypto_box`)
/// with `mlen < 32` and `..._open` (= `crypto_box_open`) with `clen < 32` return
/// -1 without writing anything. `crypto_box_afternm` / `_open_afternm` share the
/// same floor.
#[test]
fn e140_e141_padded_length_floor() {
    init_both();
    let (kps, nonces) = pool(&XSALSA, 2);
    let (rk, k) = d_beforenm("crypto_box_beforenm", &kps[1].pk, &kps[0].sk, "floor");
    assert_eq!(rk, 0);
    let mut rng = Rng::new(SEED ^ 140);
    let mut n = 0usize;
    for len in 0..ZB {
        for nn in nonces.iter() {
            let m = msg(len, &mut rng);
            for name in ["crypto_box", "crypto_box_curve25519xsalsa20poly1305"] {
                let (r, c) = d_easy(name, len, &m, nn, &kps[1].pk, &kps[0].sk, "floor");
                assert_eq!(r, -1, "{name}(mlen={len}) must return -1");
                assert!(c.iter().all(|&x| x == FILL), "{name} wrote output");
            }
            for name in [
                "crypto_box_open",
                "crypto_box_curve25519xsalsa20poly1305_open",
            ] {
                let (r, p) = d_open_easy(name, len, &m, nn, &kps[1].pk, &kps[0].sk, "floor");
                assert_eq!(r, -1, "{name}(clen={len}) must return -1");
                assert!(p.iter().all(|&x| x == FILL), "{name} wrote output");
            }
            let (r, c) = d_afternm("crypto_box_afternm", len, &m, nn, &k, "floor");
            assert_eq!(r, -1, "crypto_box_afternm(mlen={len}) must return -1");
            assert!(c.iter().all(|&x| x == FILL));
            let (r, p) = d_open_afternm("crypto_box_open_afternm", len, &m, nn, &k, "floor");
            assert_eq!(r, -1, "crypto_box_open_afternm(clen={len}) must return -1");
            assert!(p.iter().all(|&x| x == FILL));
            n += 1;
        }
    }
    // mlen == 32 is the first accepted length
    let m = padded_msg(ZB, &mut rng);
    let (r, _) = d_easy("crypto_box", ZB, &m, &nonces[0], &kps[1].pk, &kps[0].sk, "floor");
    assert_eq!(r, 0, "crypto_box(mlen=32) must succeed");
    assert!(n >= 64, "only {n} iterations");
}

// ============================================================================
// ERRORS 143 / 144 / 145 / 152 — sealed-box error surface
// ============================================================================

/// ERRORS 143: with a LOW-ORDER recipient pk the inner `crypto_box_easy` fails,
/// so `_seal` returns -1 — but `memcpy(c, epk, 32)` runs unconditionally
/// afterwards, so `c[0..32)` HAS still been overwritten with the ephemeral pk
/// while `c[32..)` is untouched. Asserted on the buffer, not just the return.
#[test]
fn e143_seal_low_order_pk_epk_still_written() {
    let _g = rng_lock();
    init_both();
    install_det_rng(false);
    let (cbase, rbase) = unsafe { pair::<ScalarmultBaseFn>("crypto_scalarmult_curve25519_base") };
    let mut rng = Rng::new(SEED ^ 143);
    let bad = low_order_pks();
    let mut n = 0usize;

    for prim in [&XSALSA, &XCHACHA] {
        let (fc, fr) = unsafe { pair::<SealFn>(prim.seal) };
        for (i, pk) in bad.iter().enumerate() {
            for &mlen in &[0usize, 1, 33, 100] {
                let skip = 3 * i + mlen;
                let m = msg(mlen, &mut rng);
                let mb = stable(&m);
                let clen = mlen + SEALB;

                let esk = rng_peek32(skip);
                let mut ec = Out::new(PKB);
                let mut er = Out::new(PKB);
                let a = unsafe { cbase(ec.ptr(), esk.as_ptr()) };
                let b = unsafe { rbase(er.ptr(), esk.as_ptr()) };
                assert_eq!(a, b);
                assert_eq_bytes("epk", &ec.v, &er.v);

                let mut oc = Out::new(clen);
                let mut or = Out::new(clen);
                rng_seek(skip);
                let rc = unsafe { fc(oc.ptr(), mb.as_ptr(), mlen as u64, pk.as_ptr()) };
                let rr = unsafe { fr(or.ptr(), mb.as_ptr(), mlen as u64, pk.as_ptr()) };
                let what = format!(
                    "err143 {} [{}] low-order pk={} mlen={mlen}",
                    prim.seal,
                    prim.tag,
                    hexs(pk)
                );
                assert_eq!(rc, rr, "{what}: return differs (C={rc} rust={rr})");
                assert_eq!(rc, -1, "{what}: C returned {rc}, expected -1");
                assert_eq_bytes(&what, &oc.v, &or.v);
                guard_intact(&what, "C", &oc);
                guard_intact(&what, "rust", &or);
                assert_eq_bytes(
                    &format!("{what}: c[0..32) must STILL hold the ephemeral pk"),
                    ec.body(),
                    &oc.body()[..PKB],
                );
                assert!(
                    oc.body()[PKB..].iter().all(|&x| x == FILL),
                    "{what}: c[32..) must be untouched, got {}",
                    hexs(&oc.body()[PKB..])
                );
                n += 1;
            }
        }
    }
    assert!(n >= 64, "only {n} iterations");
    restore_sysrandom();
}

/// ERRORS 144 + 152: `_seal_open` with `clen < SEALBYTES` (48) returns -1
/// without touching `m`.
#[test]
fn e144_e152_seal_open_clen_too_short() {
    init_both();
    let mut rng = Rng::new(SEED ^ 144);
    let mut n = 0usize;
    for prim in [&XSALSA, &XCHACHA] {
        let (kps, _) = pool(prim, 2);
        for clen in 0..SEALB {
            let c = msg(clen, &mut rng);
            let (r, p) = d_seal_open(prim.seal_open, 0, &c, &kps[0].pk, &kps[0].sk, prim.tag);
            assert_eq!(r, -1, "{}(clen={clen}) must return -1", prim.seal_open);
            assert!(p.is_empty());
            n += 1;
        }
        // clen == 48 passes the length guard; the MAC check then fails
        let c = msg(SEALB, &mut rng);
        let (r, _) = d_seal_open(prim.seal_open, 0, &c, &kps[0].pk, &kps[0].sk, prim.tag);
        assert_eq!(r, -1, "{}: a random 48-byte box must not verify", prim.seal_open);
    }
    assert!(n >= 64, "only {n} iterations");
}

/// ERRORS 145: `_seal_open` whose inner `crypto_box_open_easy` fails — wrong
/// recipient key, forged MAC, tampered epk, and a genuine box re-opened with a
/// mismatched `pk`/`sk` pair.
#[test]
fn e145_seal_open_inner_failure() {
    let _g = rng_lock();
    init_both();
    install_det_rng(false);
    let mut rng = Rng::new(SEED ^ 145);
    let mut n = 0usize;
    for prim in [&XSALSA, &XCHACHA] {
        let (kps, _) = pool(prim, 6);
        for &mlen in &[0usize, 1, 17, 32, 64, 500] {
            for i in 0..kps.len() {
                let m = msg(mlen, &mut rng);
                let (rc, c) = d_seal(prim.seal, &m, &kps[i].pk, 7 * i + mlen % 3, prim.tag);
                assert_eq!(rc, 0);

                // wrong sk (right pk)
                let j = (i + 1) % kps.len();
                let (r, p) =
                    d_seal_open(prim.seal_open, mlen, &c, &kps[i].pk, &kps[j].sk, prim.tag);
                assert_eq!(r, -1, "{} accepted a wrong sk", prim.seal_open);
                assert!(p.iter().all(|&x| x == FILL));

                // wrong pk (right sk): the nonce and the DH both change
                let (r, _) =
                    d_seal_open(prim.seal_open, mlen, &c, &kps[j].pk, &kps[i].sk, prim.tag);
                assert_eq!(r, -1, "{} accepted a wrong pk", prim.seal_open);

                // forged MAC
                let mut bad = c.clone();
                bad[PKB] ^= 0x01;
                let (r, _) =
                    d_seal_open(prim.seal_open, mlen, &bad, &kps[i].pk, &kps[i].sk, prim.tag);
                assert_eq!(r, -1, "{} accepted a forged MAC", prim.seal_open);

                // tampered ephemeral pk
                let mut bad = c.clone();
                bad[0] ^= 0x02;
                let (r, _) =
                    d_seal_open(prim.seal_open, mlen, &bad, &kps[i].pk, &kps[i].sk, prim.tag);
                assert_eq!(r, -1, "{} accepted a tampered epk", prim.seal_open);

                // the genuine box still opens
                let (r, p) =
                    d_seal_open(prim.seal_open, mlen, &c, &kps[i].pk, &kps[i].sk, prim.tag);
                assert_eq!(r, 0);
                assert_eq_bytes("seal round-trip", &m, &p);
                n += 1;
            }
        }
    }
    assert!(n >= 64, "only {n} iterations");
    restore_sysrandom();
}
