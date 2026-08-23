//! t09_kdf_kem.rs — C-vs-Rust differential verification of `crypto_kdf`
//! (blake2b + the full HKDF-SHA256/SHA512 surface), `crypto_kem`
//! (ML-KEM-768 + X-Wing) and `crypto_kx` (x25519blake2b).
//!
//! CONFIGS.md rows 217–244 and ERRORS.md rows 375–391 are the specification.
//! Every call goes through `dlsym` on BOTH shared objects; no Rust function is
//! ever called directly. Every output buffer is 64-byte aligned, prefilled with
//! 0xAA and carries a 32-byte trailing guard; the FULL buffer (body + guard) is
//! compared between the two libraries and the guard is asserted intact.
//!
//! CONFIGS row → test mapping
//! --------------------------
//! * 217 `crypto_kdf_blake2b_derive_from_key` .. `cfg217_kdf_blake2b_derive_from_key`,
//!                                              `cfg217_kdf_blake2b_salt_personal_layout`
//! * 218 `crypto_kdf_derive_from_key` dispatch . `cfg218_kdf_derive_from_key_dispatch`,
//!                                              `cfg218_kdf_constant_getters`,
//!                                              `cfg218_kdf_keygen_rng`
//! * 219 hkdf-sha256 `_extract` one-shot ...... `cfg219_hkdf_sha256_extract_oneshot`
//! * 220 hkdf-sha256 streaming extract ....... `cfg220_hkdf_sha256_extract_streaming`
//! * 221 hkdf-sha256 `_expand` ............... `cfg221_hkdf_sha256_expand`
//! * 222 hkdf-sha512 `_extract` one-shot ..... `cfg222_hkdf_sha512_extract_oneshot`
//! * 223 hkdf-sha512 streaming extract ....... `cfg223_hkdf_sha512_extract_streaming`
//! * 224 hkdf-sha512 `_expand` ............... `cfg224_hkdf_sha512_expand`
//! * 225 hkdf `_keygen` + constants .......... `cfg225_hkdf_keygen_rng`,
//!                                              `cfg225_hkdf_constant_getters`
//! * 226 mlkem768 `_seed_keypair` ............ `cfg226_mlkem768_seed_keypair`
//! * 227 mlkem768 `_keypair` **[RNG]** ....... `cfg227_mlkem768_keypair_rng`
//! * 228 mlkem768 `_enc_deterministic` ....... `cfg228_mlkem768_enc_deterministic`
//! * 229 mlkem768 `_enc` **[RNG]** ........... `cfg229_mlkem768_enc_rng`
//! * 230 mlkem768 `_dec` valid ct ............ `cfg230_mlkem768_dec_valid`
//! * 231 mlkem768 `_dec` tampered ct ......... `cfg231_e382_mlkem768_dec_tampered`
//! * 232 `rej_uniform` refill-arm seed sweep .. `cfg232_mlkem768_rej_uniform_seed_sweep`
//! * 233 xwing `_seed_keypair` ............... `cfg233_xwing_seed_keypair`
//! * 234 xwing `_keypair` **[RNG]** .......... `cfg234_xwing_keypair_rng`
//! * 235 xwing `_enc_deterministic` .......... `cfg235_xwing_enc_deterministic`
//! * 236 xwing `_enc` / `_dec` round-trip .... `cfg236_xwing_enc_dec_roundtrip`
//! * 237 `crypto_kem_*` dispatch + getters ... `cfg237_kem_dispatch`,
//!                                              `cfg237_cfg238_kem_constant_getters`
//! * 238 mlkem768 `_*bytes` .................. `cfg237_cfg238_kem_constant_getters`
//! * 239 `crypto_kx_seed_keypair` ............ `cfg239_kx_seed_keypair`
//! * 240 `crypto_kx_keypair` **[RNG]** ....... `cfg240_kx_keypair_rng`
//! * 241 full kx handshake ................... `cfg241_kx_full_handshake`
//! * 242 client `rx`/`tx` NULL shapes ........ `cfg242_kx_client_session_keys_null_shapes`
//! * 243 server `rx`/`tx` NULL shapes ........ `cfg243_kx_server_session_keys_null_shapes`
//! * 244 kx constant getters ................. `cfg244_kx_constant_getters`
//!
//! ERRORS row → test mapping
//! -------------------------
//! * 375 blake2b `subkey_len < 16` ........... `e375_e376_kdf_blake2b_subkey_len_range`
//! * 376 blake2b `subkey_len > 64` ........... `e375_e376_kdf_blake2b_subkey_len_range`
//! * 377 dispatch delegates the range check .. `e377_kdf_derive_from_key_subkey_len_range`,
//!                                              `e375_e377_kdf_subkey_len_extreme_forked`
//! * 378 hkdf-sha256 `out_len > 8160` ........ `e378_hkdf_sha256_expand_out_len_too_large`
//! * 379 hkdf-sha512 `out_len > 16320` ....... `e379_hkdf_sha512_expand_out_len_too_large`
//! * 380 mlkem768 non-canonical pk ........... `e380_mlkem768_enc_deterministic_noncanonical_pk`
//! * 381 mlkem768 `_enc` inner failure ....... `e381_mlkem768_enc_noncanonical_pk`
//! * 382 mlkem768 `_dec` never fails ......... `cfg231_e382_mlkem768_dec_tampered`
//! * 383 xwing inner mlkem enc failure ....... `e383_xwing_enc_deterministic_noncanonical_mlkem_pk`
//! * 384 xwing low-order X25519 pk ........... `e384_xwing_enc_deterministic_low_order_x25519_pk`
//! * 385 xwing `_enc` either sub-failure ..... `e385_xwing_enc_rng_sub_failures`
//! * 386 xwing `_dec` low-order ct ........... `e386_xwing_dec_low_order_ct`
//! * 387 `crypto_kem_enc`/`_dec` propagation . `e387_kem_enc_dec_dispatch_propagates`
//! * 388 client `rx == tx == NULL` misuse .... `e388_e390_kx_both_outputs_null_misuse`
//! * 389 client low-order `server_pk` ........ `e389_kx_client_low_order_server_pk`
//! * 390 server `rx == tx == NULL` misuse .... `e388_e390_kx_both_outputs_null_misuse`
//! * 391 server low-order `client_pk` ........ `e391_kx_server_low_order_client_pk`
#![allow(dead_code)]

mod common;
use common::*;
use libc::{c_char, c_int, c_void};
use std::ffi::CStr;

// ------------------------------------------------------------------ fn types

type SizeFn = unsafe extern "C" fn() -> usize;
type PrimFn = unsafe extern "C" fn() -> *const c_char;
/// `crypto_kdf[_blake2b]_derive_from_key(subkey, subkey_len, subkey_id, ctx, key)`
type DeriveFn = unsafe extern "C" fn(*mut u8, usize, u64, *const c_char, *const u8) -> c_int;
type KeygenFn = unsafe extern "C" fn(*mut u8);
/// `crypto_kdf_hkdf_sha*_extract(prk, salt, salt_len, ikm, ikm_len)`
type ExtractFn = unsafe extern "C" fn(*mut u8, *const u8, usize, *const u8, usize) -> c_int;
/// `_extract_init(state, salt, salt_len)` and `_extract_update(state, ikm, ikm_len)`
type StateBytesFn = unsafe extern "C" fn(*mut u8, *const u8, usize) -> c_int;
/// `_extract_final(state, prk)`
type StateFinalFn = unsafe extern "C" fn(*mut u8, *mut u8) -> c_int;
/// `crypto_kdf_hkdf_sha*_expand(out, out_len, ctx, ctx_len, prk)`
type ExpandFn = unsafe extern "C" fn(*mut u8, usize, *const c_char, usize, *const u8) -> c_int;
/// `crypto_generichash_blake2b_salt_personal(out, outlen, in, inlen, key, keylen, salt, personal)`
type SaltPersonalFn = unsafe extern "C" fn(
    *mut u8,
    usize,
    *const u8,
    u64,
    *const u8,
    usize,
    *const u8,
    *const u8,
) -> c_int;
type SeedKeypairFn = unsafe extern "C" fn(*mut u8, *mut u8, *const u8) -> c_int;
type KeypairFn = unsafe extern "C" fn(*mut u8, *mut u8) -> c_int;
type EncFn = unsafe extern "C" fn(*mut u8, *mut u8, *const u8) -> c_int;
type EncDetFn = unsafe extern "C" fn(*mut u8, *mut u8, *const u8, *const u8) -> c_int;
type DecFn = unsafe extern "C" fn(*mut u8, *const u8, *const u8) -> c_int;
/// `crypto_kx_{client,server}_session_keys(rx, tx, pk, sk, peer_pk)`
type SessionFn = unsafe extern "C" fn(*mut u8, *mut u8, *const u8, *const u8, *const u8) -> c_int;
type GenerichashFn =
    unsafe extern "C" fn(*mut u8, usize, *const u8, u64, *const u8, usize) -> c_int;
type ScalarmultFn = unsafe extern "C" fn(*mut u8, *const u8, *const u8) -> c_int;
type ScalarmultBaseFn = unsafe extern "C" fn(*mut u8, *const u8) -> c_int;
/// `crypto_hash_sha3256(out, in, inlen)` / `crypto_hash_sha3512(...)`
type Sha3Fn = unsafe extern "C" fn(*mut u8, *const u8, u64) -> c_int;
/// `crypto_xof_shake256(out, outlen, in, inlen)`
type ShakeFn = unsafe extern "C" fn(*mut u8, usize, *const u8, u64) -> c_int;
type BufFn = unsafe extern "C" fn(*mut c_void, usize);
type SetImplFn = unsafe extern "C" fn(*const RandombytesImpl) -> c_int;

// ------------------------------------------------------------------ constants

/// Prefill byte for every output buffer.
const FILL: u8 = 0xAA;
/// Trailing guard region: must never be touched by the library.
const PAD: usize = 32;

const EINVAL: c_int = 22;

// crypto_kdf (blake2b)
const KDF_BYTES_MIN: usize = 16;
const KDF_BYTES_MAX: usize = 64;
const KDF_CONTEXTBYTES: usize = 8;
const KDF_KEYBYTES: usize = 32;

// HKDF
const HKDF256_KEYBYTES: usize = 32;
const HKDF256_BYTES_MAX: usize = 0xff * 32; // 8160
const HKDF512_KEYBYTES: usize = 64;
const HKDF512_BYTES_MAX: usize = 0xff * 64; // 16320

// ML-KEM-768
const ML_PK: usize = 1184;
const ML_SK: usize = 2400;
const ML_CT: usize = 1088;
const ML_SS: usize = 32;
const ML_SEED: usize = 64;
const ML_POLYVECBYTES: usize = 1152;
const ML_Q: u16 = 3329;

// X-Wing
const XW_PK: usize = 1216;
const XW_SK: usize = 32;
const XW_CT: usize = 1120;
const XW_SS: usize = 32;
const XW_SEED: usize = 32;

// crypto_kx
const KX_B: usize = 32;

/// CONFIGS 217 `subkey_len` sweep.
const SUBKEY_LENS: &[usize] = &[16, 17, 32, 63, 64];
/// CONFIGS 217 `subkey_id` sweep (`2^56` and `u64::MAX` exercise the high bytes
/// of the little-endian store into the BLAKE2b salt).
const SUBKEY_IDS: &[u64] = &[0, 1, 0xFF, 1u64 << 56, u64::MAX];

/// CONFIGS 219 axes (64 == the SHA-256 HMAC block size).
const S256_SALT_LENS: &[usize] = &[0, 1, 32, 64, 65];
const S256_IKM_LENS: &[usize] = &[0, 1, 32, 64, 65, 200];
/// CONFIGS 221 axes (8160 == 255 blocks: the one-byte counter reaches 255).
const S256_OUT_LENS: &[usize] = &[0, 1, 31, 32, 33, 64, HKDF256_BYTES_MAX];
const S256_CTX_LENS: &[usize] = &[0, 1, 32];

/// CONFIGS 222 axes (128 == the SHA-512 HMAC block size).
const S512_SALT_LENS: &[usize] = &[0, 1, 64, 128, 129];
const S512_IKM_LENS: &[usize] = &[0, 1, 64, 128, 129, 300];
/// CONFIGS 224 axes (16320 == 255 blocks).
const S512_OUT_LENS: &[usize] = &[0, 1, 63, 64, 65, 128, HKDF512_BYTES_MAX];
const S512_CTX_LENS: &[usize] = &[0, 1, 64];

/// The 7 blocklisted curve25519 encodings from `has_small_order()` in
/// `c_src/libsodium/crypto_scalarmult/curve25519/ref10/x25519_ref10.c`.
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
/// encodings; both must be rejected.
fn low_order_points() -> Vec<[u8; 32]> {
    let mut v: Vec<[u8; 32]> = LOW_ORDER.to_vec();
    for e in LOW_ORDER.iter() {
        let mut w = *e;
        w[31] |= 0x80;
        v.push(w);
    }
    v
}

// --------------------------------------------------------------- out buffers

/// A 64-byte aligned block, so every `Out` satisfies the alignment of every
/// opaque libsodium state struct (`crypto_generichash_state` is `ALIGN(64)`).
#[repr(C, align(64))]
#[derive(Clone, Copy)]
struct Blk([u8; 64]);

/// A guarded output buffer: `len` payload bytes followed by `PAD` guard bytes,
/// everything prefilled with 0xAA.
struct Out {
    w: Vec<Blk>,
    len: usize,
}

impl Out {
    fn new(len: usize) -> Self {
        let nb = ((len + PAD).div_ceil(64)).max(1);
        Out { w: vec![Blk([FILL; 64]); nb], len }
    }
    fn ptr(&mut self) -> *mut u8 {
        self.w.as_mut_ptr() as *mut u8
    }
    /// Payload + guard.
    fn all(&self) -> &[u8] {
        unsafe { std::slice::from_raw_parts(self.w.as_ptr() as *const u8, self.len + PAD) }
    }
    fn body(&self) -> &[u8] {
        &self.all()[..self.len]
    }
    fn guard(&self) -> &[u8] {
        &self.all()[self.len..]
    }
    fn untouched(&self) -> bool {
        self.all().iter().all(|&x| x == FILL)
    }
}

fn guard_intact(what: &str, who: &str, o: &Out) {
    assert!(
        o.guard().iter().all(|&x| x == FILL),
        "{what}: {who} wrote OUTSIDE the requested {} bytes \
         (0xAA trailing guard clobbered: {})",
        o.len,
        hexs(o.guard())
    );
}

fn assert_untouched(what: &str, who: &str, o: &Out) {
    assert!(
        o.untouched(),
        "{what}: {who} wrote to the output buffer on a failing path \
         (expected all 0xAA, got {})",
        hexs(o.all())
    );
}

/// A byte buffer whose `as_ptr()` is always a real allocation, even for
/// `len == 0`, so a zero-length input never hands the library a dangling
/// pointer the two libraries might treat differently.
fn buf(len: usize, rng: &mut Rng) -> Vec<u8> {
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

fn clear_errno() {
    unsafe { *libc::__errno_location() = 0 };
}
fn errno() -> c_int {
    unsafe { *libc::__errno_location() }
}

/// Intentional `sodium_misuse()` aborts are frequent in this file; without this
/// systemd-coredump slows the run down by two orders of magnitude.
fn no_core_dumps() {
    unsafe {
        let lim = libc::rlimit { rlim_cur: 0, rlim_max: 0 };
        libc::setrlimit(libc::RLIMIT_CORE, &lim);
    }
}

// --------------------------------------------------------------- RNG plumbing

/// Serialises every test that mutates the process-global `randombytes`
/// implementation pointer (cargo runs the tests of one binary as parallel
/// threads inside ONE process, and both `.so`s are shared by all of them).
static RNG_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
fn rng_lock() -> std::sync::MutexGuard<'static, ()> {
    RNG_LOCK.lock().unwrap_or_else(|e| e.into_inner())
}

/// `libloading`'s `Deref for Symbol<T>` reinterprets the *stored symbol
/// address* as a `T`, so `T = *const U` yields the address of the data symbol
/// (not the value at it) — exactly what is needed for a `struct` export.
fn data_ptr<T: 'static>(lib: &'static libloading::Library, name: &str) -> *const T {
    let s = unsafe { sym::<*const T>(lib, name) };
    *s
}

/// Put both libraries back on their own default `sysrandom` implementation.
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

/// The next `n` bytes the deterministic stream hands out at offset `skip`, i.e.
/// exactly the seed/secret a `*_keypair` / `*_enc` is about to consume. Both
/// streams are left positioned at `skip`.
fn rng_peek(skip: usize, n: usize) -> Vec<u8> {
    rng_seek(skip);
    let (cbuf, _) = unsafe { pair::<BufFn>("randombytes_buf") };
    let mut v = vec![0u8; n];
    unsafe { cbuf(v.as_mut_ptr() as *mut c_void, n) };
    rng_seek(skip);
    v
}

// -------------------------------------------------- cross-checked references
//
// Reference values are computed through BOTH libraries and asserted equal, so a
// property assertion can never be satisfied by a shared bug in one library's
// helper primitive.

fn both_sha3(name: &str, out_len: usize, data: &[u8]) -> Vec<u8> {
    let (fc, fr) = unsafe { pair::<Sha3Fn>(name) };
    let d = stable(data);
    let mut a = Out::new(out_len);
    let mut b = Out::new(out_len);
    let x = unsafe { fc(a.ptr(), d.as_ptr(), d.len() as u64) };
    let y = unsafe { fr(b.ptr(), d.as_ptr(), d.len() as u64) };
    assert_eq!(x, y, "{name}: return differs");
    assert_eq_bytes(name, a.all(), b.all());
    guard_intact(name, "C", &a);
    guard_intact(name, "rust", &b);
    a.body().to_vec()
}

fn both_sha3256(data: &[u8]) -> Vec<u8> {
    both_sha3("crypto_hash_sha3256", 32, data)
}
fn both_sha3512(data: &[u8]) -> Vec<u8> {
    both_sha3("crypto_hash_sha3512", 64, data)
}

fn both_shake256(out_len: usize, data: &[u8]) -> Vec<u8> {
    let (fc, fr) = unsafe { pair::<ShakeFn>("crypto_xof_shake256") };
    let d = stable(data);
    let mut a = Out::new(out_len);
    let mut b = Out::new(out_len);
    let x = unsafe { fc(a.ptr(), out_len, d.as_ptr(), d.len() as u64) };
    let y = unsafe { fr(b.ptr(), out_len, d.as_ptr(), d.len() as u64) };
    assert_eq!(x, y, "crypto_xof_shake256: return differs");
    assert_eq_bytes("crypto_xof_shake256", a.all(), b.all());
    guard_intact("crypto_xof_shake256", "C", &a);
    guard_intact("crypto_xof_shake256", "rust", &b);
    a.body().to_vec()
}

fn both_blake2b(out_len: usize, data: &[u8]) -> Vec<u8> {
    let (fc, fr) = unsafe { pair::<GenerichashFn>("crypto_generichash") };
    let d = stable(data);
    let mut a = Out::new(out_len);
    let mut b = Out::new(out_len);
    let x = unsafe {
        fc(a.ptr(), out_len, d.as_ptr(), d.len() as u64, std::ptr::null(), 0)
    };
    let y = unsafe {
        fr(b.ptr(), out_len, d.as_ptr(), d.len() as u64, std::ptr::null(), 0)
    };
    assert_eq!(x, y, "crypto_generichash: return differs");
    assert_eq_bytes("crypto_generichash", a.all(), b.all());
    guard_intact("crypto_generichash", "C", &a);
    guard_intact("crypto_generichash", "rust", &b);
    a.body().to_vec()
}

/// `crypto_scalarmult_base` through both libraries.
fn both_scalarmult_base(n: &[u8]) -> (c_int, Vec<u8>) {
    let (fc, fr) = unsafe { pair::<ScalarmultBaseFn>("crypto_scalarmult_base") };
    let mut a = Out::new(32);
    let mut b = Out::new(32);
    let x = unsafe { fc(a.ptr(), n.as_ptr()) };
    let y = unsafe { fr(b.ptr(), n.as_ptr()) };
    assert_eq!(x, y, "crypto_scalarmult_base: return differs");
    assert_eq_bytes("crypto_scalarmult_base", a.all(), b.all());
    guard_intact("crypto_scalarmult_base", "C", &a);
    guard_intact("crypto_scalarmult_base", "rust", &b);
    (x, a.body().to_vec())
}

/// `crypto_scalarmult` through both libraries.
fn both_scalarmult(n: &[u8], p: &[u8]) -> (c_int, Vec<u8>) {
    let (fc, fr) = unsafe { pair::<ScalarmultFn>("crypto_scalarmult") };
    let mut a = Out::new(32);
    let mut b = Out::new(32);
    let x = unsafe { fc(a.ptr(), n.as_ptr(), p.as_ptr()) };
    let y = unsafe { fr(b.ptr(), n.as_ptr(), p.as_ptr()) };
    assert_eq!(x, y, "crypto_scalarmult: return differs");
    assert_eq_bytes("crypto_scalarmult", a.all(), b.all());
    (x, a.body().to_vec())
}

/// Compare a `size_t`-returning getter between both libraries and against the
/// value the C header hard-codes.
fn getter(name: &str, want: usize) -> usize {
    let (fc, fr) = unsafe { pair::<SizeFn>(name) };
    let (a, b) = unsafe { (fc(), fr()) };
    assert_eq!(a, b, "{name}(): C={a} rust={b}");
    assert_eq!(a, want, "{name}(): C returned {a}, header says {want}");
    a
}

fn primitive(name: &str, want: &str) {
    let (fc, fr) = unsafe { pair::<PrimFn>(name) };
    let (a, b) = unsafe { (fc(), fr()) };
    assert!(!a.is_null() && !b.is_null(), "{name}(): returned NULL");
    let sa = unsafe { CStr::from_ptr(a) }.to_str().unwrap();
    let sb = unsafe { CStr::from_ptr(b) }.to_str().unwrap();
    assert_eq!(sa, sb, "{name}(): C={sa:?} rust={sb:?}");
    assert_eq!(sa, want, "{name}(): C={sa:?}, header says {want:?}");
}

// ------------------------------------------------------- crypto_kdf drivers

/// `crypto_kdf[_blake2b]_derive_from_key` through both libraries. `buf_len` is
/// the size of the allocated payload region and may exceed `subkey_len` so that
/// an out-of-range length still has room to write into if it (wrongly) did.
fn d_derive(
    name: &str,
    buf_len: usize,
    subkey_len: usize,
    subkey_id: u64,
    ctx: &[u8],
    key: &[u8],
) -> (c_int, c_int, Out) {
    assert_eq!(ctx.len(), KDF_CONTEXTBYTES);
    assert_eq!(key.len(), KDF_KEYBYTES);
    let (fc, fr) = unsafe { pair::<DeriveFn>(name) };
    let mut oc = Out::new(buf_len);
    let mut or = Out::new(buf_len);
    clear_errno();
    let rc =
        unsafe { fc(oc.ptr(), subkey_len, subkey_id, ctx.as_ptr() as *const c_char, key.as_ptr()) };
    let ec = errno();
    clear_errno();
    let rr =
        unsafe { fr(or.ptr(), subkey_len, subkey_id, ctx.as_ptr() as *const c_char, key.as_ptr()) };
    let er = errno();
    let what = format!(
        "{name} subkey_len={subkey_len} subkey_id={subkey_id:#x} ctx={} key={}",
        hexs(ctx),
        hexs(key)
    );
    assert_eq!(rc, rr, "{what}: return differs (C={rc} rust={rr})");
    assert_eq_bytes(&format!("{what} subkey"), oc.all(), or.all());
    assert_eq!(ec, er, "{what}: errno differs (C={ec} rust={er})");
    guard_intact(&what, "C", &oc);
    guard_intact(&what, "rust", &or);
    (rc, ec, oc)
}

/// `crypto_kdf_hkdf_sha*_extract` through both libraries. `salt == None` passes
/// a NULL salt pointer (legal: the header only marks `ikm` / `prk` non-null).
fn d_extract(
    name: &str,
    prk_len: usize,
    salt: Option<&[u8]>,
    salt_len: usize,
    ikm: &[u8],
    ikm_len: usize,
) -> (c_int, Out) {
    let (fc, fr) = unsafe { pair::<ExtractFn>(name) };
    let sp = match salt {
        Some(s) => s.as_ptr(),
        None => std::ptr::null(),
    };
    let mut oc = Out::new(prk_len);
    let mut or = Out::new(prk_len);
    let rc = unsafe { fc(oc.ptr(), sp, salt_len, ikm.as_ptr(), ikm_len) };
    let rr = unsafe { fr(or.ptr(), sp, salt_len, ikm.as_ptr(), ikm_len) };
    let what = format!(
        "{name} salt={} salt_len={salt_len} ikm_len={ikm_len}",
        if salt.is_some() { "buf" } else { "NULL" }
    );
    assert_eq!(rc, rr, "{what}: return differs (C={rc} rust={rr})");
    assert_eq_bytes(&format!("{what} prk"), oc.all(), or.all());
    guard_intact(&what, "C", &oc);
    guard_intact(&what, "rust", &or);
    (rc, oc)
}

/// `crypto_kdf_hkdf_sha*_expand` through both libraries.
fn d_expand(
    name: &str,
    buf_len: usize,
    out_len: usize,
    ctx: Option<&[u8]>,
    ctx_len: usize,
    prk: &[u8],
) -> (c_int, c_int, Out) {
    let (fc, fr) = unsafe { pair::<ExpandFn>(name) };
    let cp = match ctx {
        Some(x) => x.as_ptr() as *const c_char,
        None => std::ptr::null(),
    };
    let mut oc = Out::new(buf_len);
    let mut or = Out::new(buf_len);
    clear_errno();
    let rc = unsafe { fc(oc.ptr(), out_len, cp, ctx_len, prk.as_ptr()) };
    let ec = errno();
    clear_errno();
    let rr = unsafe { fr(or.ptr(), out_len, cp, ctx_len, prk.as_ptr()) };
    let er = errno();
    let what = format!(
        "{name} out_len={out_len} ctx={} ctx_len={ctx_len} prk={}",
        if ctx.is_some() { "buf" } else { "NULL" },
        hexs(prk)
    );
    assert_eq!(rc, rr, "{what}: return differs (C={rc} rust={rr})");
    assert_eq_bytes(&format!("{what} out"), oc.all(), or.all());
    assert_eq!(ec, er, "{what}: errno differs (C={ec} rust={er})");
    guard_intact(&what, "C", &oc);
    guard_intact(&what, "rust", &or);
    (rc, ec, oc)
}

// ------------------------------------------------------- crypto_kem drivers

/// `*_seed_keypair` through both libraries.
fn d_seed_keypair(
    name: &str,
    pk_len: usize,
    sk_len: usize,
    seed: &[u8],
) -> (c_int, Out, Out) {
    let (fc, fr) = unsafe { pair::<SeedKeypairFn>(name) };
    let mut pc = Out::new(pk_len);
    let mut sc = Out::new(sk_len);
    let mut pr = Out::new(pk_len);
    let mut sr = Out::new(sk_len);
    let rc = unsafe { fc(pc.ptr(), sc.ptr(), seed.as_ptr()) };
    let rr = unsafe { fr(pr.ptr(), sr.ptr(), seed.as_ptr()) };
    let what = format!("{name} seed={}", hexs(seed));
    assert_eq!(rc, rr, "{what}: return differs (C={rc} rust={rr})");
    assert_eq_bytes(&format!("{what} pk"), pc.all(), pr.all());
    assert_eq_bytes(&format!("{what} sk"), sc.all(), sr.all());
    guard_intact(&what, "C", &pc);
    guard_intact(&what, "C", &sc);
    guard_intact(&what, "rust", &pr);
    guard_intact(&what, "rust", &sr);
    (rc, pc, sc)
}

/// `*_keypair` through both libraries (deterministic RNG must be installed).
fn d_keypair(name: &str, pk_len: usize, sk_len: usize, tag: &str) -> (c_int, Out, Out) {
    let (fc, fr) = unsafe { pair::<KeypairFn>(name) };
    let mut pc = Out::new(pk_len);
    let mut sc = Out::new(sk_len);
    let mut pr = Out::new(pk_len);
    let mut sr = Out::new(sk_len);
    let rc = unsafe { fc(pc.ptr(), sc.ptr()) };
    let rr = unsafe { fr(pr.ptr(), sr.ptr()) };
    let what = format!("{name} [{tag}]");
    assert_eq!(rc, rr, "{what}: return differs (C={rc} rust={rr})");
    assert_eq_bytes(&format!("{what} pk"), pc.all(), pr.all());
    assert_eq_bytes(&format!("{what} sk"), sc.all(), sr.all());
    guard_intact(&what, "C", &pc);
    guard_intact(&what, "C", &sc);
    guard_intact(&what, "rust", &pr);
    guard_intact(&what, "rust", &sr);
    (rc, pc, sc)
}

/// `*_enc_deterministic` through both libraries.
fn d_enc_det(
    name: &str,
    ct_len: usize,
    ss_len: usize,
    pk: &[u8],
    seed: &[u8],
    tag: &str,
) -> (c_int, Out, Out) {
    let (fc, fr) = unsafe { pair::<EncDetFn>(name) };
    let mut cc = Out::new(ct_len);
    let mut sc = Out::new(ss_len);
    let mut cr = Out::new(ct_len);
    let mut sr = Out::new(ss_len);
    let rc = unsafe { fc(cc.ptr(), sc.ptr(), pk.as_ptr(), seed.as_ptr()) };
    let rr = unsafe { fr(cr.ptr(), sr.ptr(), pk.as_ptr(), seed.as_ptr()) };
    let what = format!("{name} [{tag}] seed={}", hexs(seed));
    assert_eq!(rc, rr, "{what}: return differs (C={rc} rust={rr})");
    assert_eq_bytes(&format!("{what} ct"), cc.all(), cr.all());
    assert_eq_bytes(&format!("{what} ss"), sc.all(), sr.all());
    guard_intact(&what, "C", &cc);
    guard_intact(&what, "C", &sc);
    guard_intact(&what, "rust", &cr);
    guard_intact(&what, "rust", &sr);
    (rc, cc, sc)
}

/// `*_enc` through both libraries (deterministic RNG must be installed).
fn d_enc(
    name: &str,
    ct_len: usize,
    ss_len: usize,
    pk: &[u8],
    tag: &str,
) -> (c_int, Out, Out) {
    let (fc, fr) = unsafe { pair::<EncFn>(name) };
    let mut cc = Out::new(ct_len);
    let mut sc = Out::new(ss_len);
    let mut cr = Out::new(ct_len);
    let mut sr = Out::new(ss_len);
    let rc = unsafe { fc(cc.ptr(), sc.ptr(), pk.as_ptr()) };
    let rr = unsafe { fr(cr.ptr(), sr.ptr(), pk.as_ptr()) };
    let what = format!("{name} [{tag}]");
    assert_eq!(rc, rr, "{what}: return differs (C={rc} rust={rr})");
    assert_eq_bytes(&format!("{what} ct"), cc.all(), cr.all());
    assert_eq_bytes(&format!("{what} ss"), sc.all(), sr.all());
    guard_intact(&what, "C", &cc);
    guard_intact(&what, "C", &sc);
    guard_intact(&what, "rust", &cr);
    guard_intact(&what, "rust", &sr);
    (rc, cc, sc)
}

/// `*_dec` through both libraries.
fn d_dec(name: &str, ss_len: usize, ct: &[u8], sk: &[u8], tag: &str) -> (c_int, Out) {
    let (fc, fr) = unsafe { pair::<DecFn>(name) };
    let mut sc = Out::new(ss_len);
    let mut sr = Out::new(ss_len);
    let rc = unsafe { fc(sc.ptr(), ct.as_ptr(), sk.as_ptr()) };
    let rr = unsafe { fr(sr.ptr(), ct.as_ptr(), sk.as_ptr()) };
    let what = format!("{name} [{tag}]");
    assert_eq!(rc, rr, "{what}: return differs (C={rc} rust={rr})");
    assert_eq_bytes(&format!("{what} ss"), sc.all(), sr.all());
    guard_intact(&what, "C", &sc);
    guard_intact(&what, "rust", &sr);
    (rc, sc)
}

// -------------------------------------------------- ML-KEM 12-bit pack helper
//
// `poly_frombytes()` unpacks 3 bytes into two 12-bit coefficients; a public key
// holds MLKEM768_K = 3 polynomials of MLKEM768_N = 256 coefficients in the
// first MLKEM768_POLYVECBYTES = 1152 bytes. `polyvec_is_canonical()` requires
// every one of the 768 coefficients to be < MLKEM768_Q.

fn get_coeff(pk: &[u8], poly: usize, j: usize) -> u16 {
    let base = poly * 384 + (j / 2) * 3;
    if j % 2 == 0 {
        ((pk[base] as u16) | ((pk[base + 1] as u16) << 8)) & 0xFFF
    } else {
        (((pk[base + 1] as u16) >> 4) | ((pk[base + 2] as u16) << 4)) & 0xFFF
    }
}

fn set_coeff(pk: &mut [u8], poly: usize, j: usize, val: u16) {
    let base = poly * 384 + (j / 2) * 3;
    let mut t0 = ((pk[base] as u16) | ((pk[base + 1] as u16) << 8)) & 0xFFF;
    let mut t1 = (((pk[base + 1] as u16) >> 4) | ((pk[base + 2] as u16) << 4)) & 0xFFF;
    if j % 2 == 0 {
        t0 = val & 0xFFF;
    } else {
        t1 = val & 0xFFF;
    }
    pk[base] = (t0 & 0xFF) as u8;
    pk[base + 1] = (((t0 >> 8) & 0x0F) | ((t1 & 0x0F) << 4)) as u8;
    pk[base + 2] = ((t1 >> 4) & 0xFF) as u8;
}

/// Coefficient slots probed by the non-canonical-pk rows: the very first and
/// last coefficient of every polynomial, both parities of the 3-byte group, and
/// the coefficient that straddles the 1152-byte polyvec boundary.
const COEFF_SLOTS: &[(usize, usize)] = &[
    (0, 0),
    (0, 1),
    (0, 2),
    (0, 3),
    (0, 254),
    (0, 255),
    (1, 0),
    (1, 1),
    (1, 128),
    (1, 255),
    (2, 0),
    (2, 1),
    (2, 129),
    (2, 254),
    (2, 255),
];

/// A valid ML-KEM-768 keypair from a fixed seed (`_seed_keypair` is fully
/// deterministic and already proven byte-identical by CONFIGS 226).
fn ml_kp(seed: &[u8]) -> (Vec<u8>, Vec<u8>) {
    let (rc, pk, sk) = d_seed_keypair("crypto_kem_mlkem768_seed_keypair", ML_PK, ML_SK, seed);
    assert_eq!(rc, 0, "crypto_kem_mlkem768_seed_keypair returned {rc}");
    (pk.body().to_vec(), sk.body().to_vec())
}

/// A valid X-Wing keypair from a fixed 32-byte seed.
fn xw_kp(seed: &[u8]) -> (Vec<u8>, Vec<u8>) {
    let (rc, pk, sk) = d_seed_keypair("crypto_kem_xwing_seed_keypair", XW_PK, XW_SK, seed);
    assert_eq!(rc, 0, "crypto_kem_xwing_seed_keypair returned {rc}");
    (pk.body().to_vec(), sk.body().to_vec())
}

// ============================================================================
// CONFIGS 217 — crypto_kdf_blake2b_derive_from_key
// ============================================================================

/// CONFIGS 217: `subkey_len` ∈ {16,17,32,63,64} × `subkey_id` ∈
/// {0,1,0xFF,2^56,u64::MAX} × ctx all-zero / ASCII / embedded NULs / random.
#[test]
fn cfg217_kdf_blake2b_derive_from_key() {
    init_both();
    let mut rng = Rng::new(SEED ^ 217);
    let mut ctxs: Vec<Vec<u8>> = vec![
        vec![0u8; KDF_CONTEXTBYTES],                       // all-zero
        b"Examples".to_vec(),                              // ASCII
        b"ctx\0key\0".to_vec(),                            // embedded NULs
        vec![0xffu8; KDF_CONTEXTBYTES],                    // all-0xff
        (0..KDF_CONTEXTBYTES as u8).collect(),             // counter
    ];
    ctxs.push(rng.bytes(KDF_CONTEXTBYTES));
    ctxs.push(rng.bytes(KDF_CONTEXTBYTES));
    let mut keys = patterns(KDF_KEYBYTES, &mut rng);
    keys.push(rng.bytes(KDF_KEYBYTES));

    let mut n = 0usize;
    let mut seen: Vec<Vec<u8>> = Vec::new();
    for key in &keys {
        for ctx in &ctxs {
            for &slen in SUBKEY_LENS {
                for &sid in SUBKEY_IDS {
                    let (rc, _, out) = d_derive(
                        "crypto_kdf_blake2b_derive_from_key",
                        slen,
                        slen,
                        sid,
                        ctx,
                        key,
                    );
                    assert_eq!(rc, 0, "valid derive_from_key returned {rc}");
                    // a derived subkey is never left at the 0xAA prefill
                    assert!(
                        out.body().iter().any(|&x| x != FILL),
                        "subkey was not written at all"
                    );
                    seen.push(out.body().to_vec());
                    n += 1;
                }
            }
        }
    }
    assert!(n >= 64, "only {n} iterations");
    // distinct (subkey_id, ctx, key, len) must give distinct subkeys
    let mut sorted = seen.clone();
    sorted.sort();
    sorted.dedup();
    assert_eq!(sorted.len(), seen.len(), "derive_from_key produced a collision");
}

/// CONFIGS 217 (structure): `subkey_id` is stored little-endian into the first 8
/// bytes of the 16-byte BLAKE2b **salt** with `salt[8..16)` zeroed, and the
/// 8-byte `ctx` is zero-padded into the 16-byte **personal** field. Proven by
/// reconstructing the call to `crypto_generichash_blake2b_salt_personal`
/// through both libraries.
#[test]
fn cfg217_kdf_blake2b_salt_personal_layout() {
    init_both();
    let (spc, spr) =
        unsafe { pair::<SaltPersonalFn>("crypto_generichash_blake2b_salt_personal") };
    let mut rng = Rng::new(SEED ^ 0x217);
    let mut n = 0usize;
    for i in 0..8 {
        let key = rng.bytes(KDF_KEYBYTES);
        let ctx = if i == 0 {
            vec![0u8; KDF_CONTEXTBYTES]
        } else {
            rng.bytes(KDF_CONTEXTBYTES)
        };
        for &slen in SUBKEY_LENS {
            for &sid in SUBKEY_IDS {
                let (rc, _, out) = d_derive(
                    "crypto_kdf_blake2b_derive_from_key",
                    slen,
                    slen,
                    sid,
                    &ctx,
                    &key,
                );
                assert_eq!(rc, 0);

                let mut salt = [0u8; 16];
                salt[..8].copy_from_slice(&sid.to_le_bytes());
                let mut personal = [0u8; 16];
                personal[..KDF_CONTEXTBYTES].copy_from_slice(&ctx);

                let mut ec = Out::new(slen);
                let mut er = Out::new(slen);
                let a = unsafe {
                    spc(
                        ec.ptr(),
                        slen,
                        std::ptr::null(),
                        0,
                        key.as_ptr(),
                        KDF_KEYBYTES,
                        salt.as_ptr(),
                        personal.as_ptr(),
                    )
                };
                let b = unsafe {
                    spr(
                        er.ptr(),
                        slen,
                        std::ptr::null(),
                        0,
                        key.as_ptr(),
                        KDF_KEYBYTES,
                        salt.as_ptr(),
                        personal.as_ptr(),
                    )
                };
                assert_eq!(a, b, "salt_personal return differs");
                assert_eq_bytes("salt_personal", ec.all(), er.all());
                assert_eq_bytes(
                    &format!(
                        "derive_from_key(len={slen}, id={sid:#x}) != \
                         blake2b_salt_personal(salt=LE64(id)||0^8, personal=ctx||0^8)"
                    ),
                    ec.body(),
                    out.body(),
                );
                n += 1;
            }
        }
    }
    assert!(n >= 64, "only {n} iterations");
}

// ============================================================================
// CONFIGS 218 — crypto_kdf_derive_from_key dispatch, getters, keygen
// ============================================================================

/// CONFIGS 218: `crypto_kdf_derive_from_key` is a literal delegation to the
/// blake2b primitive, so for every input the two entry points must agree — in
/// both libraries.
#[test]
fn cfg218_kdf_derive_from_key_dispatch() {
    init_both();
    let mut rng = Rng::new(SEED ^ 218);
    let mut n = 0usize;
    for i in 0..8 {
        let key = rng.bytes(KDF_KEYBYTES);
        let ctx = if i == 1 { b"ctx\0\0\0\0\0".to_vec() } else { rng.bytes(KDF_CONTEXTBYTES) };
        for &slen in SUBKEY_LENS {
            for &sid in SUBKEY_IDS {
                let (r1, e1, o1) =
                    d_derive("crypto_kdf_derive_from_key", slen, slen, sid, &ctx, &key);
                let (r2, e2, o2) = d_derive(
                    "crypto_kdf_blake2b_derive_from_key",
                    slen,
                    slen,
                    sid,
                    &ctx,
                    &key,
                );
                assert_eq!(r1, 0, "crypto_kdf_derive_from_key returned {r1}");
                assert_eq!(r1, r2, "dispatch return differs from primitive");
                assert_eq!(e1, e2, "dispatch errno differs from primitive");
                assert_eq_bytes(
                    "crypto_kdf_derive_from_key != crypto_kdf_blake2b_derive_from_key",
                    o1.all(),
                    o2.all(),
                );
                n += 1;
            }
        }
    }
    assert!(n >= 64, "only {n} iterations");
}

/// CONFIGS 218: `crypto_kdf_BYTES_MIN`/`_MAX`/`_CONTEXTBYTES`/`_KEYBYTES` and
/// `crypto_kdf_PRIMITIVE`, at the generic and the blake2b level.
#[test]
fn cfg218_kdf_constant_getters() {
    init_both();
    getter("crypto_kdf_bytes_min", 16);
    getter("crypto_kdf_bytes_max", 64);
    getter("crypto_kdf_contextbytes", 8);
    getter("crypto_kdf_keybytes", 32);
    getter("crypto_kdf_blake2b_bytes_min", 16);
    getter("crypto_kdf_blake2b_bytes_max", 64);
    getter("crypto_kdf_blake2b_contextbytes", 8);
    getter("crypto_kdf_blake2b_keybytes", 32);
    primitive("crypto_kdf_primitive", "blake2b");
}

/// CONFIGS 218: `crypto_kdf_keygen` is `randombytes_buf(k, 32)` — with the
/// deterministic RNG installed in both libraries the 32 bytes must match each
/// other and the raw stream.
#[test]
fn cfg218_kdf_keygen_rng() {
    let _g = rng_lock();
    init_both();
    install_det_rng(false);
    let (fc, fr) = unsafe { pair::<KeygenFn>("crypto_kdf_keygen") };
    let mut n = 0usize;
    for skip in 0..70usize {
        let want = rng_peek(skip, KDF_KEYBYTES);
        let mut oc = Out::new(KDF_KEYBYTES);
        let mut or = Out::new(KDF_KEYBYTES);
        unsafe { fc(oc.ptr()) };
        unsafe { fr(or.ptr()) };
        let what = format!("crypto_kdf_keygen rngskip={skip}");
        assert_eq_bytes(&what, oc.all(), or.all());
        guard_intact(&what, "C", &oc);
        guard_intact(&what, "rust", &or);
        assert_eq_bytes(&format!("{what}: k is not the next 32 RNG bytes"), &want, oc.body());
        n += 1;
    }
    assert!(n >= 64, "only {n} iterations");
    restore_sysrandom();
}

// ============================================================================
// CONFIGS 219 / 222 — HKDF one-shot extract
// ============================================================================

fn extract_oneshot_body(
    name: &str,
    prk_len: usize,
    block_len: usize,
    salt_lens: &[usize],
    ikm_lens: &[usize],
    tweak: u64,
) {
    init_both();
    let mut rng = Rng::new(SEED ^ tweak);
    let mut n = 0usize;
    // Only the random round feeds the distinctness check: HMAC zero-pads a
    // short key to the block size, so an all-zero salt of length 0, 1, ... up to
    // the block size is by construction the SAME HMAC key (asserted below).
    let mut seen: Vec<Vec<u8>> = Vec::new();
    for round in 0..3 {
        for &sl in salt_lens {
            for &il in ikm_lens {
                let salt = match round {
                    0 => vec![0u8; sl],
                    1 => vec![0xffu8; sl],
                    _ => buf(sl, &mut rng),
                };
                let salt = stable(&salt);
                let ikm = match round {
                    0 => vec![0u8; il],
                    1 => (0..il).map(|i| i as u8).collect(),
                    _ => buf(il, &mut rng),
                };
                let ikm = stable(&ikm);
                let (rc, prk) = d_extract(name, prk_len, Some(&salt), sl, &ikm, il);
                assert_eq!(rc, 0, "{name} returned {rc}");
                assert!(prk.body().iter().any(|&x| x != FILL), "{name}: prk not written");
                if round == 2 {
                    seen.push(prk.body().to_vec());
                }
                n += 1;
            }
        }
    }
    // Structural consequence of `crypto_auth_hmacsha*_init`: a key shorter than
    // the block size is XORed into a zero-filled pad, so an ALL-ZERO salt of any
    // length <= block_len is the same key, while a longer salt is first hashed
    // down to KEYBYTES and therefore differs.
    {
        let ikm = stable(&rng.bytes(37));
        let zero = vec![0u8; block_len + 1];
        let mut base: Option<Vec<u8>> = None;
        for sl in [0usize, 1, block_len / 2, block_len] {
            let (rc, prk) = d_extract(name, prk_len, Some(&zero), sl, &ikm, ikm.len());
            assert_eq!(rc, 0);
            match &base {
                None => base = Some(prk.body().to_vec()),
                Some(b) => assert_eq_bytes(
                    &format!(
                        "{name}: an all-zero salt of length {sl} must be the same HMAC key \
                         as a zero-length salt (HMAC zero-pads to {block_len})"
                    ),
                    b,
                    prk.body(),
                ),
            }
        }
        let (rc, over) = d_extract(name, prk_len, Some(&zero), block_len + 1, &ikm, ikm.len());
        assert_eq!(rc, 0);
        assert_ne!(
            base.as_deref(),
            Some(over.body()),
            "{name}: a salt longer than the block size must be hashed first"
        );
    }
    // `salt == NULL` with `salt_len == 0` is the documented shape (the header
    // only marks `prk` / `ikm` non-null) and must equal a zero-length buffer.
    for &il in ikm_lens {
        let ikm = stable(&buf(il, &mut rng));
        let (r1, p1) = d_extract(name, prk_len, None, 0, &ikm, il);
        let empty: Vec<u8> = Vec::with_capacity(1);
        let (r2, p2) = d_extract(name, prk_len, Some(&empty), 0, &ikm, il);
        assert_eq!(r1, 0);
        assert_eq!(r1, r2, "{name}: NULL salt return differs from empty salt");
        assert_eq_bytes(&format!("{name}: NULL salt != empty salt"), p1.all(), p2.all());
        n += 1;
    }
    assert!(n >= 64, "only {n} iterations");
    let mut sorted = seen.clone();
    sorted.sort();
    sorted.dedup();
    assert_eq!(
        sorted.len(),
        seen.len(),
        "{name} produced a collision across distinct random (salt, ikm)"
    );
}

/// CONFIGS 219: `salt_len` ∈ {0,1,32,64 (== HMAC block),65} × `ikm_len` ∈
/// {0,1,32,64,65,200}.
#[test]
fn cfg219_hkdf_sha256_extract_oneshot() {
    extract_oneshot_body(
        "crypto_kdf_hkdf_sha256_extract",
        HKDF256_KEYBYTES,
        64,
        S256_SALT_LENS,
        S256_IKM_LENS,
        219,
    );
}

/// CONFIGS 222: `salt_len` ∈ {0,1,64,128 (== HMAC block),129} × `ikm_len` ∈
/// {0,1,64,128,129,300}.
#[test]
fn cfg222_hkdf_sha512_extract_oneshot() {
    extract_oneshot_body(
        "crypto_kdf_hkdf_sha512_extract",
        HKDF512_KEYBYTES,
        128,
        S512_SALT_LENS,
        S512_IKM_LENS,
        222,
    );
}

// ============================================================================
// CONFIGS 220 / 223 — HKDF streaming extract (opaque state compared in full)
// ============================================================================

fn extract_streaming_body(
    tag: &str,
    prk_len: usize,
    salt_lens: &[usize],
    ikm_lens: &[usize],
    tweak: u64,
) {
    init_both();
    let sb_name = format!("crypto_kdf_hkdf_{tag}_statebytes");
    let init_name = format!("crypto_kdf_hkdf_{tag}_extract_init");
    let upd_name = format!("crypto_kdf_hkdf_{tag}_extract_update");
    let fin_name = format!("crypto_kdf_hkdf_{tag}_extract_final");
    let one_name = format!("crypto_kdf_hkdf_{tag}_extract");

    // The state size is part of the ABI: it must match between the libraries.
    let (sbc, sbr) = unsafe { pair::<SizeFn>(&sb_name) };
    let (nc, nr) = unsafe { (sbc(), sbr()) };
    assert_eq!(nc, nr, "{sb_name}(): C={nc} rust={nr}");
    let sbytes = nc;
    assert!(sbytes > 0);

    let (ic, ir) = unsafe { pair::<StateBytesFn>(&init_name) };
    let (uc, ur) = unsafe { pair::<StateBytesFn>(&upd_name) };
    let (fc, fr) = unsafe { pair::<StateFinalFn>(&fin_name) };

    let mut rng = Rng::new(SEED ^ tweak);
    let mut n = 0usize;
    for &sl in salt_lens {
        for &il in ikm_lens {
            let salt = stable(&buf(sl, &mut rng));
            let ikm = stable(&buf(il, &mut rng));
            // chunkings: one shot, halves, thirds, and a leading empty chunk
            let mut splits: Vec<Vec<(usize, usize)>> = vec![
                vec![(0, il)],
                vec![(0, il / 2), (il / 2, il)],
                vec![(0, 0), (0, il)],
            ];
            if il >= 3 {
                let a = il / 3;
                let b = 2 * il / 3;
                splits.push(vec![(0, a), (a, b), (b, il)]);
            }
            if il > 0 && il <= 8 {
                splits.push((0..il).map(|i| (i, i + 1)).collect());
            }
            for chunks in &splits {
                let mut stc = Out::new(sbytes);
                let mut str_ = Out::new(sbytes);
                let what = format!(
                    "{init_name} salt_len={sl} ikm_len={il} chunks={}",
                    chunks.len()
                );
                let a = unsafe { ic(stc.ptr(), salt.as_ptr(), sl) };
                let b = unsafe { ir(str_.ptr(), salt.as_ptr(), sl) };
                assert_eq!(a, b, "{what}: return differs (C={a} rust={b})");
                assert_eq_bytes(&format!("{what}: state after _extract_init"), stc.all(), str_.all());
                guard_intact(&what, "C", &stc);
                guard_intact(&what, "rust", &str_);

                for (k, &(lo, hi)) in chunks.iter().enumerate() {
                    let a = unsafe { uc(stc.ptr(), ikm[lo..].as_ptr(), hi - lo) };
                    let b = unsafe { ur(str_.ptr(), ikm[lo..].as_ptr(), hi - lo) };
                    assert_eq!(a, b, "{what}: _extract_update #{k} return differs");
                    assert_eq_bytes(
                        &format!("{what}: state after _extract_update #{k} ({}..{hi})", lo),
                        stc.all(),
                        str_.all(),
                    );
                    guard_intact(&what, "C", &stc);
                    guard_intact(&what, "rust", &str_);
                }

                let mut pc = Out::new(prk_len);
                let mut pr = Out::new(prk_len);
                let a = unsafe { fc(stc.ptr(), pc.ptr()) };
                let b = unsafe { fr(str_.ptr(), pr.ptr()) };
                assert_eq!(a, b, "{fin_name}: return differs (C={a} rust={b})");
                assert_eq!(a, 0, "{fin_name}: returned {a}");
                assert_eq_bytes(&format!("{what}: prk"), pc.all(), pr.all());
                assert_eq_bytes(&format!("{what}: state after _extract_final"), stc.all(), str_.all());
                guard_intact(&what, "C", &pc);
                guard_intact(&what, "rust", &pr);
                // `_extract_final` zeroes the whole state and nothing past it.
                for (who, st) in [("C", &stc), ("rust", &str_)] {
                    assert!(
                        st.body().iter().all(|&x| x == 0),
                        "{what}: {who} did not zero the state in _extract_final: {}",
                        hexs(st.body())
                    );
                    guard_intact(&what, who, st);
                }

                // must equal the one-shot extract (CONFIGS 219 / 222)
                let (rc1, p1) = d_extract(&one_name, prk_len, Some(&salt), sl, &ikm, il);
                assert_eq!(rc1, 0);
                assert_eq_bytes(
                    &format!("{what}: streaming prk != one-shot {one_name}"),
                    p1.body(),
                    pc.body(),
                );
                n += 1;
            }
        }
    }
    assert!(n >= 64, "only {n} iterations");
}

/// CONFIGS 220: `_extract_init` / `_extract_update`×N / `_extract_final`. The
/// WHOLE opaque state buffer (allocated at `_statebytes()`, oversized and
/// 0xAA-prefilled) is compared after `_extract_init` and after every
/// `_extract_update`.
#[test]
fn cfg220_hkdf_sha256_extract_streaming() {
    extract_streaming_body("sha256", HKDF256_KEYBYTES, S256_SALT_LENS, S256_IKM_LENS, 220);
}

/// CONFIGS 223: the SHA-512 streaming extract.
#[test]
fn cfg223_hkdf_sha512_extract_streaming() {
    extract_streaming_body("sha512", HKDF512_KEYBYTES, S512_SALT_LENS, S512_IKM_LENS, 223);
}

// ============================================================================
// CONFIGS 221 / 224 — HKDF expand
// ============================================================================

fn expand_body(name: &str, key_len: usize, out_lens: &[usize], ctx_lens: &[usize], tweak: u64) {
    init_both();
    let mut rng = Rng::new(SEED ^ tweak);
    let mut n = 0usize;
    let mut prks = patterns(key_len, &mut rng);
    prks.push(rng.bytes(key_len));
    for prk in &prks {
        for &ol in out_lens {
            for &cl in ctx_lens {
                let ctx = stable(&buf(cl, &mut rng));
                let (rc, _, out) = d_expand(name, ol, ol, Some(&ctx), cl, prk);
                assert_eq!(rc, 0, "{name} out_len={ol} returned {rc}");
                if ol > 0 {
                    assert!(
                        out.body().iter().any(|&x| x != FILL),
                        "{name}: nothing written for out_len={ol}"
                    );
                }
                n += 1;
            }
        }
        // `ctx == NULL` with `ctx_len == 0` (the header marks only `out`
        // non-null) must equal an empty context buffer.
        let empty: Vec<u8> = Vec::with_capacity(1);
        let (r1, _, o1) = d_expand(name, 64, 64, None, 0, prk);
        let (r2, _, o2) = d_expand(name, 64, 64, Some(&empty), 0, prk);
        assert_eq!(r1, 0);
        assert_eq!(r1, r2, "{name}: NULL ctx return differs from empty ctx");
        assert_eq_bytes(&format!("{name}: NULL ctx != empty ctx"), o1.all(), o2.all());
        n += 1;
    }
    assert!(n >= 64, "only {n} iterations");

    // out_len == 0 must leave the buffer untouched in BOTH libraries.
    let (rc, _, out) = d_expand(name, 64, 0, Some(b"ctx"), 3, &prks[0]);
    assert_eq!(rc, 0);
    assert!(out.untouched(), "{name}: out_len=0 wrote to the buffer");

    // Successive blocks must chain (out[i-32] feeds block i), so a longer
    // request is a prefix-extension of a shorter one whenever the shorter one
    // is a whole number of blocks.
    let blk = key_len;
    let (r1, _, long) = d_expand(name, 4 * blk, 4 * blk, Some(b"chain"), 5, &prks[0]);
    let (r2, _, short) = d_expand(name, 2 * blk, 2 * blk, Some(b"chain"), 5, &prks[0]);
    assert_eq!((r1, r2), (0, 0));
    assert_eq_bytes(
        &format!("{name}: block chaining is not prefix-stable"),
        short.body(),
        &long.body()[..2 * blk],
    );
}

/// CONFIGS 221: `out_len` ∈ {0,1,31,32,33,64,8160} × `ctx_len` ∈ {0,1,32}.
/// 8160 == 255 × 32, so the single-byte counter reaches 255.
#[test]
fn cfg221_hkdf_sha256_expand() {
    expand_body("crypto_kdf_hkdf_sha256_expand", 32, S256_OUT_LENS, S256_CTX_LENS, 221);
}

/// CONFIGS 224: `out_len` ∈ {0,1,63,64,65,128,16320} × `ctx_len` ∈ {0,1,64}.
#[test]
fn cfg224_hkdf_sha512_expand() {
    expand_body("crypto_kdf_hkdf_sha512_expand", 64, S512_OUT_LENS, S512_CTX_LENS, 224);
}

/// Requirement 5: `extract` → `expand` round-trip for both hashes.
#[test]
fn hkdf_extract_expand_roundtrip() {
    init_both();
    let mut rng = Rng::new(SEED ^ 0x1234);
    let mut n = 0usize;
    for (ex, exp, klen) in [
        ("crypto_kdf_hkdf_sha256_extract", "crypto_kdf_hkdf_sha256_expand", 32usize),
        ("crypto_kdf_hkdf_sha512_extract", "crypto_kdf_hkdf_sha512_expand", 64usize),
    ] {
        for i in 0..40 {
            let salt = stable(&rng.bytes(i % 17));
            let ikm = stable(&rng.bytes(1 + i % 71));
            let (rc, prk) = d_extract(ex, klen, Some(&salt), salt.len(), &ikm, ikm.len());
            assert_eq!(rc, 0);
            let ctx = stable(&rng.bytes(i % 9));
            let ol = 1 + i * 3;
            let (re, _, out) = d_expand(exp, ol, ol, Some(&ctx), ctx.len(), prk.body());
            assert_eq!(re, 0, "{exp} returned {re}");
            // deriving twice from the same prk is stable
            let (re2, _, out2) = d_expand(exp, ol, ol, Some(&ctx), ctx.len(), prk.body());
            assert_eq!(re2, 0);
            assert_eq_bytes(&format!("{exp}: not deterministic"), out.all(), out2.all());
            n += 1;
        }
    }
    assert!(n >= 64, "only {n} iterations");
}

// ============================================================================
// CONFIGS 225 — HKDF keygen + constants
// ============================================================================

/// CONFIGS 225: `crypto_kdf_hkdf_sha256_keygen` / `_sha512_keygen` are
/// `randombytes_buf(prk, KEYBYTES)`.
#[test]
fn cfg225_hkdf_keygen_rng() {
    let _g = rng_lock();
    init_both();
    install_det_rng(false);
    let mut n = 0usize;
    for (name, klen) in
        [("crypto_kdf_hkdf_sha256_keygen", 32usize), ("crypto_kdf_hkdf_sha512_keygen", 64usize)]
    {
        let (fc, fr) = unsafe { pair::<KeygenFn>(name) };
        for skip in 0..40usize {
            let want = rng_peek(skip, klen);
            let mut oc = Out::new(klen);
            let mut or = Out::new(klen);
            unsafe { fc(oc.ptr()) };
            unsafe { fr(or.ptr()) };
            let what = format!("{name} rngskip={skip}");
            assert_eq_bytes(&what, oc.all(), or.all());
            guard_intact(&what, "C", &oc);
            guard_intact(&what, "rust", &or);
            assert_eq_bytes(
                &format!("{what}: prk is not the next {klen} RNG bytes"),
                &want,
                oc.body(),
            );
            n += 1;
        }
    }
    assert!(n >= 64, "only {n} iterations");
    restore_sysrandom();
}

/// CONFIGS 225: `_statebytes` / `_bytes_min` / `_bytes_max` / `_keybytes` for
/// both HKDF hashes (8160 / 16320).
#[test]
fn cfg225_hkdf_constant_getters() {
    init_both();
    getter("crypto_kdf_hkdf_sha256_keybytes", 32);
    getter("crypto_kdf_hkdf_sha256_bytes_min", 0);
    getter("crypto_kdf_hkdf_sha256_bytes_max", 8160);
    getter("crypto_kdf_hkdf_sha512_keybytes", 64);
    getter("crypto_kdf_hkdf_sha512_bytes_min", 0);
    getter("crypto_kdf_hkdf_sha512_bytes_max", 16320);
    // `_statebytes` is `sizeof(struct { crypto_auth_hmacsha*_state st; })`:
    // 2 * (8*4 + 8 + 64) = 208 and 2 * (8*8 + 2*8 + 128) = 416.
    getter("crypto_kdf_hkdf_sha256_statebytes", 208);
    getter("crypto_kdf_hkdf_sha512_statebytes", 416);
    // and it must agree with the underlying HMAC state size
    let s256 = getter("crypto_auth_hmacsha256_statebytes", 208);
    let s512 = getter("crypto_auth_hmacsha512_statebytes", 416);
    assert_eq!(s256, 208);
    assert_eq!(s512, 416);
}

// ============================================================================
// ERRORS 375 / 376 / 377 — crypto_kdf subkey_len out of range
// ============================================================================

fn subkey_len_range_body(name: &str) {
    init_both();
    let mut rng = Rng::new(SEED ^ 0x375);
    let key = rng.bytes(KDF_KEYBYTES);
    let ctx = rng.bytes(KDF_CONTEXTBYTES);
    // A generous payload region: even a (buggy) write of the requested length
    // stays inside the allocation, so a divergence is reported instead of
    // corrupting the test process.
    const BIG: usize = 4096;
    let mut n = 0usize;
    // ERRORS 375: subkey_len < BYTES_MIN (16)
    for slen in 0..KDF_BYTES_MIN {
        for &sid in SUBKEY_IDS {
            let (rc, en, out) = d_derive(name, BIG, slen, sid, &ctx, &key);
            assert_eq!(rc, -1, "{name} subkey_len={slen}: expected -1, got {rc}");
            assert_eq!(en, EINVAL, "{name} subkey_len={slen}: errno={en}, expected EINVAL");
            assert_untouched(&format!("{name} subkey_len={slen}"), "C", &out);
            n += 1;
        }
    }
    // ERRORS 376: subkey_len > BYTES_MAX (64)
    for slen in [65usize, 66, 67, 80, 96, 127, 128, 200, 255, 256, 1000, 2048, BIG] {
        for &sid in SUBKEY_IDS {
            let (rc, en, out) = d_derive(name, BIG, slen, sid, &ctx, &key);
            assert_eq!(rc, -1, "{name} subkey_len={slen}: expected -1, got {rc}");
            assert_eq!(en, EINVAL, "{name} subkey_len={slen}: errno={en}, expected EINVAL");
            assert_untouched(&format!("{name} subkey_len={slen}"), "C", &out);
            n += 1;
        }
    }
    // the two in-range boundaries stay accepted
    for slen in [KDF_BYTES_MIN, KDF_BYTES_MAX] {
        let (rc, _, _) = d_derive(name, slen, slen, 7, &ctx, &key);
        assert_eq!(rc, 0, "{name} subkey_len={slen} must be accepted");
    }
    assert!(n >= 64, "only {n} iterations");
}

/// ERRORS 375 + 376: `crypto_kdf_blake2b_derive_from_key` rejects
/// `subkey_len < 16` and `subkey_len > 64` with -1 / `EINVAL`, without touching
/// the output buffer.
#[test]
fn e375_e376_kdf_blake2b_subkey_len_range() {
    subkey_len_range_body("crypto_kdf_blake2b_derive_from_key");
}

/// ERRORS 377: the generic `crypto_kdf_derive_from_key` inherits the range
/// check through its delegation.
#[test]
fn e377_kdf_derive_from_key_subkey_len_range() {
    subkey_len_range_body("crypto_kdf_derive_from_key");
}

/// ERRORS 375 + 377 (extreme lengths): a `subkey_len` that could never be
/// written is still rejected before any store. Run in a forked child so that a
/// divergent (wild-writing) implementation is observed as a signal rather than
/// killing the test runner.
#[test]
fn e375_e377_kdf_subkey_len_extreme_forked() {
    init_both();
    no_core_dumps();
    let mut rng = Rng::new(SEED ^ 0x377);
    let key = rng.bytes(KDF_KEYBYTES);
    let ctx = rng.bytes(KDF_CONTEXTBYTES);
    for name in ["crypto_kdf_blake2b_derive_from_key", "crypto_kdf_derive_from_key"] {
        let (fc, fr) = unsafe { pair::<DeriveFn>(name) };
        let cf = *fc;
        let rf = *fr;
        for slen in [usize::MAX, usize::MAX - 1, usize::MAX / 2, 1usize << 40, 1usize << 32] {
            for &sid in &[0u64, u64::MAX] {
                let mut ob = Out::new(4096);
                let p = ob.ptr();
                let cp = ctx.as_ptr() as *const c_char;
                let kp = key.as_ptr();
                let run = |f: DeriveFn| -> i64 {
                    unsafe {
                        *libc::__errno_location() = 0;
                        let rc = f(p, slen, sid, cp, kp);
                        let en = *libc::__errno_location();
                        if rc == -1 && en == EINVAL {
                            1
                        } else if rc == -1 {
                            2
                        } else {
                            3
                        }
                    }
                };
                let a = forked(|| run(cf));
                let b = forked(|| run(rf));
                assert_same_fatal(&format!("{name} subkey_len={slen}"), a, b);
                assert_eq!(
                    a,
                    Outcome::Returned(1),
                    "{name} subkey_len={slen}: C did not return -1/EINVAL"
                );
            }
        }
    }
}

// ============================================================================
// ERRORS 378 / 379 — HKDF expand out_len > BYTES_MAX
// ============================================================================

fn expand_too_large_body(name: &str, max: usize, extra: &[usize]) {
    init_both();
    let mut rng = Rng::new(SEED ^ 0x378);
    let prk = rng.bytes(if max == HKDF256_BYTES_MAX { 32 } else { 64 });
    // Payload region large enough that every probed out_len could be written.
    let big = max + 4096;
    let mut n = 0usize;
    for &ol in extra {
        assert!(ol > max && ol <= big);
        let (rc, en, out) = d_expand(name, big, ol, Some(b"ctx"), 3, &prk);
        assert_eq!(rc, -1, "{name} out_len={ol}: expected -1, got {rc}");
        assert_eq!(en, EINVAL, "{name} out_len={ol}: errno={en}, expected EINVAL");
        assert_untouched(&format!("{name} out_len={ol}"), "C", &out);
        n += 1;
    }
    // `out_len == BYTES_MAX` is still accepted (255 blocks, counter == 255)
    let (rc, _, out) = d_expand(name, max, max, Some(b"ctx"), 3, &prk);
    assert_eq!(rc, 0, "{name} out_len={max} must be accepted");
    assert!(out.body().iter().any(|&x| x != FILL));
    assert!(n >= 4, "only {n} iterations");

    // Extreme values, forked: rejected before any store.
    no_core_dumps();
    let (fc, fr) = unsafe { pair::<ExpandFn>(name) };
    let cf = *fc;
    let rf = *fr;
    for ol in [usize::MAX, usize::MAX / 2, 1usize << 40, max + 1] {
        let mut ob = Out::new(big);
        let p = ob.ptr();
        let pp = prk.as_ptr();
        let run = |f: ExpandFn| -> i64 {
            unsafe {
                *libc::__errno_location() = 0;
                let rc = f(p, ol, b"ctx\0".as_ptr() as *const c_char, 3, pp);
                let en = *libc::__errno_location();
                if rc == -1 && en == EINVAL {
                    1
                } else if rc == -1 {
                    2
                } else {
                    3
                }
            }
        };
        let a = forked(|| run(cf));
        let b = forked(|| run(rf));
        assert_same_fatal(&format!("{name} out_len={ol}"), a, b);
        assert_eq!(a, Outcome::Returned(1), "{name} out_len={ol}: C did not return -1/EINVAL");
    }
}

/// ERRORS 378: `crypto_kdf_hkdf_sha256_expand` with `out_len > 0xff*32 = 8160`.
#[test]
fn e378_hkdf_sha256_expand_out_len_too_large() {
    expand_too_large_body(
        "crypto_kdf_hkdf_sha256_expand",
        HKDF256_BYTES_MAX,
        &[8161, 8162, 8191, 8192, 10000, 12256],
    );
}

/// ERRORS 379: `crypto_kdf_hkdf_sha512_expand` with `out_len > 0xff*64 = 16320`.
#[test]
fn e379_hkdf_sha512_expand_out_len_too_large() {
    expand_too_large_body(
        "crypto_kdf_hkdf_sha512_expand",
        HKDF512_BYTES_MAX,
        &[16321, 16322, 16383, 16384, 20000, 20416],
    );
}

// ============================================================================
// CONFIGS 226 / 238 — ML-KEM-768 seed_keypair
// ============================================================================

/// CONFIGS 226: `crypto_kem_mlkem768_seed_keypair` over 64-byte seeds
/// (all-0x00, all-0xff, counter, random×64+). Fully deterministic, so this is a
/// direct byte-for-byte comparison; the secret-key layout
/// `skpv ‖ pk ‖ SHA3-256(pk) ‖ z` and the `indseed[32] = K = 3` domain byte are
/// asserted as well.
#[test]
fn cfg226_mlkem768_seed_keypair() {
    init_both();
    let mut rng = Rng::new(SEED ^ 226);
    let mut seeds: Vec<Vec<u8>> = vec![
        vec![0u8; ML_SEED],
        vec![0xffu8; ML_SEED],
        (0..ML_SEED as u8).collect(),
        (0..ML_SEED).map(|i| (255 - i) as u8).collect(),
    ];
    for _ in 0..70 {
        seeds.push(rng.bytes(ML_SEED));
    }
    let mut n = 0usize;
    let mut pks: Vec<Vec<u8>> = Vec::new();
    for seed in &seeds {
        let (rc, pk, sk) =
            d_seed_keypair("crypto_kem_mlkem768_seed_keypair", ML_PK, ML_SK, seed);
        assert_eq!(rc, 0, "seed_keypair returned {rc}");
        let pkb = pk.body();
        let skb = sk.body();

        // sk = polyvec(1152) || pk(1184) || SHA3-256(pk)(32) || z(32)
        assert_eq_bytes(
            "sk[1152..2336] must be a verbatim copy of pk",
            pkb,
            &skb[ML_POLYVECBYTES..ML_POLYVECBYTES + ML_PK],
        );
        let hpk = both_sha3256(pkb);
        assert_eq_bytes(
            "sk[2336..2368] must be SHA3-256(pk)",
            &hpk,
            &skb[ML_POLYVECBYTES + ML_PK..ML_POLYVECBYTES + ML_PK + 32],
        );
        assert_eq_bytes(
            "sk[2368..2400] must be seed[32..64) verbatim (z)",
            &seed[32..64],
            &skb[ML_POLYVECBYTES + ML_PK + 32..ML_SK],
        );

        // pk[1152..1184] == publicseed == SHA3-512(seed[0..32] || K=3)[0..32]
        let mut indseed = Vec::with_capacity(33);
        indseed.extend_from_slice(&seed[..32]);
        indseed.push(3);
        let buf64 = both_sha3512(&indseed);
        assert_eq_bytes(
            "pk[1152..1184] must be SHA3-512(seed[0..32] || 0x03)[0..32]",
            &buf64[..32],
            &pkb[ML_POLYVECBYTES..ML_PK],
        );

        // every packed coefficient of a freshly generated pk is canonical
        for poly in 0..3 {
            for j in 0..256 {
                let c = get_coeff(pkb, poly, j);
                assert!(c < ML_Q, "generated pk has non-canonical coeff[{poly}][{j}]={c}");
            }
        }
        pks.push(pkb.to_vec());
        n += 1;
    }
    assert!(n >= 64, "only {n} iterations");
    let mut s = pks.clone();
    s.sort();
    s.dedup();
    assert_eq!(s.len(), pks.len(), "distinct seeds produced identical public keys");
}

// ============================================================================
// CONFIGS 227 — ML-KEM-768 keypair [RNG]
// ============================================================================

/// CONFIGS 227: `crypto_kem_mlkem768_keypair` == `_seed_keypair` over the next
/// 64 bytes of `randombytes_buf`.
#[test]
fn cfg227_mlkem768_keypair_rng() {
    let _g = rng_lock();
    init_both();
    install_det_rng(false);
    let mut n = 0usize;
    for skip in 0..70usize {
        let seed = rng_peek(skip, ML_SEED);
        let (rc, pk, sk) = d_keypair("crypto_kem_mlkem768_keypair", ML_PK, ML_SK, "mlkem768");
        assert_eq!(rc, 0, "keypair returned {rc}");
        let (rc2, pk2, sk2) =
            d_seed_keypair("crypto_kem_mlkem768_seed_keypair", ML_PK, ML_SK, &seed);
        assert_eq!(rc2, 0);
        assert_eq_bytes(
            &format!("keypair(rngskip={skip}) pk != seed_keypair(next 64 RNG bytes)"),
            pk2.all(),
            pk.all(),
        );
        assert_eq_bytes(
            &format!("keypair(rngskip={skip}) sk != seed_keypair(next 64 RNG bytes)"),
            sk2.all(),
            sk.all(),
        );
        n += 1;
    }
    assert!(n >= 64, "only {n} iterations");
    restore_sysrandom();
}

// ============================================================================
// CONFIGS 228 — ML-KEM-768 enc_deterministic
// ============================================================================

/// CONFIGS 228: `crypto_kem_mlkem768_enc_deterministic` over valid public keys
/// × 32-byte seeds. Also asserts the FO-transform identity
/// `ss == SHA3-512(seed ‖ SHA3-256(pk))[0..32]`.
#[test]
fn cfg228_mlkem768_enc_deterministic() {
    init_both();
    let mut rng = Rng::new(SEED ^ 228);
    let mut n = 0usize;
    let mut cts: Vec<Vec<u8>> = Vec::new();
    for i in 0..9 {
        let mut kseed = rng.bytes(ML_SEED);
        kseed[0] ^= i as u8;
        let (pk, _sk) = ml_kp(&kseed);
        let mut seeds: Vec<Vec<u8>> = vec![vec![0u8; 32], vec![0xffu8; 32], (0..32u8).collect()];
        for _ in 0..6 {
            seeds.push(rng.bytes(32));
        }
        for seed in &seeds {
            let (rc, ct, ss) = d_enc_det(
                "crypto_kem_mlkem768_enc_deterministic",
                ML_CT,
                ML_SS,
                &pk,
                seed,
                "mlkem768",
            );
            assert_eq!(rc, 0, "enc_deterministic returned {rc} for a valid pk");
            let mut m = Vec::with_capacity(64);
            m.extend_from_slice(seed);
            m.extend_from_slice(&both_sha3256(&pk));
            let kr = both_sha3512(&m);
            assert_eq_bytes(
                "ss != SHA3-512(seed || SHA3-256(pk))[0..32]",
                &kr[..32],
                ss.body(),
            );
            cts.push(ct.body().to_vec());
            n += 1;
        }
    }
    assert!(n >= 64, "only {n} iterations");
    let mut s = cts.clone();
    s.sort();
    s.dedup();
    assert_eq!(s.len(), cts.len(), "distinct (pk,seed) produced identical ciphertexts");
}

// ============================================================================
// CONFIGS 229 — ML-KEM-768 enc [RNG]
// ============================================================================

/// CONFIGS 229: `crypto_kem_mlkem768_enc` == `_enc_deterministic` over the next
/// 32 bytes of `randombytes_buf`.
#[test]
fn cfg229_mlkem768_enc_rng() {
    let _g = rng_lock();
    init_both();
    install_det_rng(false);
    let mut rng = Rng::new(SEED ^ 229);
    let mut n = 0usize;
    for i in 0..8 {
        let mut kseed = rng.bytes(ML_SEED);
        kseed[1] ^= i as u8;
        let (pk, sk) = ml_kp(&kseed);
        for skip in 0..9usize {
            let seed = rng_peek(skip, 32);
            let (rc, ct, ss) = d_enc("crypto_kem_mlkem768_enc", ML_CT, ML_SS, &pk, "mlkem768");
            assert_eq!(rc, 0, "enc returned {rc}");
            let (rc2, ct2, ss2) = d_enc_det(
                "crypto_kem_mlkem768_enc_deterministic",
                ML_CT,
                ML_SS,
                &pk,
                &seed,
                "mlkem768",
            );
            assert_eq!(rc2, 0);
            assert_eq_bytes("enc ct != enc_deterministic(next 32 RNG bytes)", ct2.all(), ct.all());
            assert_eq_bytes("enc ss != enc_deterministic(next 32 RNG bytes)", ss2.all(), ss.all());
            // and the ciphertext decapsulates to the same shared secret
            let (rd, ssd) = d_dec("crypto_kem_mlkem768_dec", ML_SS, ct.body(), &sk, "mlkem768");
            assert_eq!(rd, 0, "dec returned {rd}");
            assert_eq_bytes("enc/dec shared-secret mismatch", ss.body(), ssd.body());
            n += 1;
        }
    }
    assert!(n >= 64, "only {n} iterations");
    restore_sysrandom();
}

// ============================================================================
// CONFIGS 230 — ML-KEM-768 dec, valid ciphertext
// ============================================================================

/// CONFIGS 230: with an untampered ciphertext `fail_mask == 0`, so `dec`
/// returns the true shared secret and NOT `SHAKE256(z ‖ ct)`.
#[test]
fn cfg230_mlkem768_dec_valid() {
    init_both();
    let mut rng = Rng::new(SEED ^ 230);
    let mut n = 0usize;
    for i in 0..12 {
        let mut kseed = rng.bytes(ML_SEED);
        kseed[2] ^= i as u8;
        let (pk, sk) = ml_kp(&kseed);
        let z = &sk[ML_POLYVECBYTES + ML_PK + 32..ML_SK];
        for _ in 0..6 {
            let eseed = rng.bytes(32);
            let (rc, ct, ss) = d_enc_det(
                "crypto_kem_mlkem768_enc_deterministic",
                ML_CT,
                ML_SS,
                &pk,
                &eseed,
                "mlkem768",
            );
            assert_eq!(rc, 0);
            let (rd, ssd) = d_dec("crypto_kem_mlkem768_dec", ML_SS, ct.body(), &sk, "mlkem768");
            assert_eq!(rd, 0, "dec returned {rd} for a valid ciphertext");
            assert_eq_bytes("ss_enc != ss_dec", ss.body(), ssd.body());

            let mut zct = Vec::with_capacity(32 + ML_CT);
            zct.extend_from_slice(z);
            zct.extend_from_slice(ct.body());
            let kbar = both_shake256(32, &zct);
            assert_ne!(
                kbar.as_slice(),
                ssd.body(),
                "a VALID ciphertext must not decapsulate to the implicit-rejection \
                 secret SHAKE256(z || ct)"
            );
            n += 1;
        }
    }
    assert!(n >= 64, "only {n} iterations");
}

// ============================================================================
// CONFIGS 231 / ERRORS 382 — ML-KEM-768 dec, tampered ciphertext
// ============================================================================

/// CONFIGS 231 + ERRORS 382: `crypto_kem_mlkem768_dec` has NO rejection branch.
/// A tampered ciphertext is handled by constant-time implicit rejection
/// (`cmov` of `SHAKE256(z ‖ ct)`), so BOTH libraries must return 0 and produce
/// the SAME (wrong) shared secret — which must equal `SHAKE256(z ‖ ct)`.
#[test]
fn cfg231_e382_mlkem768_dec_tampered() {
    init_both();
    let mut rng = Rng::new(SEED ^ 231);
    // byte positions across both compressed halves of the ciphertext:
    // b (du, 3*320 = 960 bytes) and v (dv, 128 bytes)
    let mut positions: Vec<usize> =
        vec![0, 1, 2, 3, 159, 160, 319, 320, 321, 639, 640, 959, 960, 961, 1023, 1024, 1086, 1087];
    for _ in 0..22 {
        positions.push(rng.below(ML_CT));
    }
    let mut n = 0usize;
    for i in 0..3 {
        let mut kseed = rng.bytes(ML_SEED);
        kseed[3] ^= i as u8;
        let (pk, sk) = ml_kp(&kseed);
        let z = sk[ML_POLYVECBYTES + ML_PK + 32..ML_SK].to_vec();
        let eseed = rng.bytes(32);
        let (rc, ct, ss) = d_enc_det(
            "crypto_kem_mlkem768_enc_deterministic",
            ML_CT,
            ML_SS,
            &pk,
            &eseed,
            "mlkem768",
        );
        assert_eq!(rc, 0);
        let good = ct.body().to_vec();

        for &p in &positions {
            for delta in [0x01u8, 0x80, 0xff] {
                let mut bad = good.clone();
                bad[p] ^= delta;
                let (rd, ssd) =
                    d_dec("crypto_kem_mlkem768_dec", ML_SS, &bad, &sk, "mlkem768");
                assert_eq!(
                    rd, 0,
                    "crypto_kem_mlkem768_dec must ALWAYS return 0 \
                     (tampered byte {p} ^= {delta:#02x}), got {rd}"
                );
                let mut zct = Vec::with_capacity(32 + ML_CT);
                zct.extend_from_slice(&z);
                zct.extend_from_slice(&bad);
                let kbar = both_shake256(32, &zct);
                assert_eq_bytes(
                    &format!(
                        "implicit rejection: dec(tampered ct byte {p} ^= {delta:#02x}) \
                         must be SHAKE256(z || ct)"
                    ),
                    &kbar,
                    ssd.body(),
                );
                assert_ne!(
                    ssd.body(),
                    ss.body(),
                    "tampered ct byte {p} ^= {delta:#02x} decapsulated to the TRUE secret"
                );
                n += 1;
            }
        }
        // truncating is not possible through this fixed-size API, but flipping
        // every bit of the first byte is
        for bit in 0..8 {
            let mut bad = good.clone();
            bad[0] ^= 1 << bit;
            let (rd, _) = d_dec("crypto_kem_mlkem768_dec", ML_SS, &bad, &sk, "mlkem768");
            assert_eq!(rd, 0);
            n += 1;
        }
    }
    assert!(n >= 64, "only {n} iterations");
}

// ============================================================================
// CONFIGS 232 — rejection-sampling axis
// ============================================================================

/// CONFIGS 232: `gen_matrix` squeezes GEN_MATRIX_NBLOCKS = 3 SHAKE-128 blocks
/// (504 bytes → 336 twelve-bit candidates) per polynomial and keeps those
/// `< MLKEM768_Q`; the acceptance rate is 3329/4096, so ≈0.9% of polynomials
/// need the `while (ctr < 256)` refill squeeze and ≈8% of the 9-polynomial
/// matrices hit it at least once. Sweeping several hundred independent seeds
/// through `_seed_keypair` (`transposed = 0`) and `_enc_deterministic`
/// (`transposed = 1`) therefore exercises the refill arm many times over.
#[test]
fn cfg232_mlkem768_rej_uniform_seed_sweep() {
    init_both();
    let mut rng = Rng::new(SEED ^ 232);
    let mut n_kp = 0usize;
    let mut n_enc = 0usize;
    let mut last_pk: Option<Vec<u8>> = None;
    for i in 0..400usize {
        let mut seed = rng.bytes(ML_SEED);
        // decorrelate: also sweep the publicseed-determining first bytes
        seed[0] = (i & 0xff) as u8;
        seed[1] = ((i >> 8) & 0xff) as u8;
        let (rc, pk, _sk) =
            d_seed_keypair("crypto_kem_mlkem768_seed_keypair", ML_PK, ML_SK, &seed);
        assert_eq!(rc, 0);
        n_kp += 1;
        last_pk = Some(pk.body().to_vec());
    }
    // transposed = 1: gen_matrix is re-run from pk[1152..1184] inside enc
    let base = last_pk.unwrap();
    for i in 0..400usize {
        // vary the public seed embedded in the pk so gen_matrix(transposed=1)
        // sees 400 independent matrix seeds; the polyvec part stays canonical
        let mut pk = base.clone();
        pk[ML_POLYVECBYTES] = (i & 0xff) as u8;
        pk[ML_POLYVECBYTES + 1] = ((i >> 8) & 0xff) as u8;
        pk[ML_POLYVECBYTES + 2] = rng.byte();
        let eseed = rng.bytes(32);
        let (rc, _ct, _ss) = d_enc_det(
            "crypto_kem_mlkem768_enc_deterministic",
            ML_CT,
            ML_SS,
            &pk,
            &eseed,
            "mlkem768/transposed",
        );
        assert_eq!(rc, 0, "enc_deterministic rejected a canonical pk");
        n_enc += 1;
    }
    assert!(n_kp >= 64, "only {n_kp} keypair iterations");
    assert!(n_enc >= 64, "only {n_enc} enc iterations");
}

// ============================================================================
// CONFIGS 233 — X-Wing seed_keypair
// ============================================================================

/// CONFIGS 233: `crypto_kem_xwing_seed_keypair` over 32-byte seeds.
/// `expand_decaps_key` is `SHAKE256(seed, 32) -> 96` split into a 64-byte
/// ML-KEM seed and a 32-byte X25519 scalar; `sk` is the seed verbatim.
#[test]
fn cfg233_xwing_seed_keypair() {
    init_both();
    let mut rng = Rng::new(SEED ^ 233);
    let mut seeds: Vec<Vec<u8>> = vec![
        vec![0u8; XW_SEED],
        vec![0xffu8; XW_SEED],
        (0..XW_SEED as u8).collect(),
    ];
    for _ in 0..70 {
        seeds.push(rng.bytes(XW_SEED));
    }
    let mut n = 0usize;
    let mut pks: Vec<Vec<u8>> = Vec::new();
    for seed in &seeds {
        let (rc, pk, sk) =
            d_seed_keypair("crypto_kem_xwing_seed_keypair", XW_PK, XW_SK, seed);
        assert_eq!(rc, 0, "xwing seed_keypair returned {rc}");
        assert_eq_bytes("xwing sk must be the seed verbatim", seed, sk.body());

        let expanded = both_shake256(96, seed);
        let (rc2, mlpk, _mlsk) =
            d_seed_keypair("crypto_kem_mlkem768_seed_keypair", ML_PK, ML_SK, &expanded[..64]);
        assert_eq!(rc2, 0);
        assert_eq_bytes(
            "xwing pk[0..1184] != mlkem768 pk from SHAKE256(seed,32)[0..64]",
            mlpk.body(),
            &pk.body()[..ML_PK],
        );
        let (rb, xpk) = both_scalarmult_base(&expanded[64..96]);
        assert_eq!(rb, 0);
        assert_eq_bytes(
            "xwing pk[1184..1216] != X25519 base * SHAKE256(seed,32)[64..96]",
            &xpk,
            &pk.body()[ML_PK..XW_PK],
        );
        pks.push(pk.body().to_vec());
        n += 1;
    }
    assert!(n >= 64, "only {n} iterations");
    let mut s = pks.clone();
    s.sort();
    s.dedup();
    assert_eq!(s.len(), pks.len(), "distinct seeds produced identical xwing public keys");
}

// ============================================================================
// CONFIGS 234 — X-Wing keypair [RNG]
// ============================================================================

/// CONFIGS 234: `crypto_kem_xwing_keypair` == `_seed_keypair` over the next 32
/// bytes of `randombytes_buf`.
#[test]
fn cfg234_xwing_keypair_rng() {
    let _g = rng_lock();
    init_both();
    install_det_rng(false);
    let mut n = 0usize;
    for skip in 0..70usize {
        let seed = rng_peek(skip, XW_SEED);
        let (rc, pk, sk) = d_keypair("crypto_kem_xwing_keypair", XW_PK, XW_SK, "xwing");
        assert_eq!(rc, 0, "xwing keypair returned {rc}");
        let (rc2, pk2, sk2) =
            d_seed_keypair("crypto_kem_xwing_seed_keypair", XW_PK, XW_SK, &seed);
        assert_eq!(rc2, 0);
        assert_eq_bytes(
            &format!("xwing keypair(rngskip={skip}) pk != seed_keypair(next 32 RNG bytes)"),
            pk2.all(),
            pk.all(),
        );
        assert_eq_bytes(
            &format!("xwing keypair(rngskip={skip}) sk != seed_keypair(next 32 RNG bytes)"),
            sk2.all(),
            sk.all(),
        );
        n += 1;
    }
    assert!(n >= 64, "only {n} iterations");
    restore_sysrandom();
}

// ============================================================================
// CONFIGS 235 / 236 — X-Wing encapsulation and round-trip
// ============================================================================

/// The X-Wing combiner: `SHA3-256(ss_ml ‖ ss_x ‖ ct_x ‖ pk_x ‖ label)`.
const XWING_LABEL: [u8; 6] = [0x5c, 0x2e, 0x2f, 0x2f, 0x5e, 0x5c];

/// CONFIGS 235: `crypto_kem_xwing_enc_deterministic` over valid public keys ×
/// 64-byte seeds (`[0..32)` = ML-KEM `m`, `[32..64)` = X25519 scalar). The full
/// combiner is recomputed from its documented inputs.
#[test]
fn cfg235_xwing_enc_deterministic() {
    init_both();
    let mut rng = Rng::new(SEED ^ 235);
    let mut n = 0usize;
    let mut cts: Vec<Vec<u8>> = Vec::new();
    for i in 0..9 {
        let mut kseed = rng.bytes(XW_SEED);
        kseed[0] ^= i as u8;
        let (pk, _sk) = xw_kp(&kseed);
        let mut seeds: Vec<Vec<u8>> = vec![vec![0u8; 64], vec![0xffu8; 64], (0..64u8).collect()];
        for _ in 0..6 {
            seeds.push(rng.bytes(64));
        }
        for seed in &seeds {
            let (rc, ct, ss) = d_enc_det(
                "crypto_kem_xwing_enc_deterministic",
                XW_CT,
                XW_SS,
                &pk,
                seed,
                "xwing",
            );
            assert_eq!(rc, 0, "xwing enc_deterministic returned {rc}");
            let ctb = ct.body();

            // ct = ct_mlkem(1088) || ct_x25519(32); ct_x25519 = base * seed[32..64]
            let (rb, ctx) = both_scalarmult_base(&seed[32..64]);
            assert_eq!(rb, 0);
            assert_eq_bytes("xwing ct[1088..1120] != X25519 base * seed[32..64]", &ctx, &ctb[ML_CT..]);

            // ct_mlkem / ss_mlkem from the ML-KEM part of the pk and seed[0..32]
            let (rm, mlct, mlss) = d_enc_det(
                "crypto_kem_mlkem768_enc_deterministic",
                ML_CT,
                ML_SS,
                &pk[..ML_PK],
                &seed[..32],
                "xwing/inner",
            );
            assert_eq!(rm, 0);
            assert_eq_bytes("xwing ct[0..1088] != mlkem768 ct", mlct.body(), &ctb[..ML_CT]);

            let (rs, ssx) = both_scalarmult(&seed[32..64], &pk[ML_PK..]);
            assert_eq!(rs, 0);
            let mut m = Vec::with_capacity(32 + 32 + 32 + 32 + 6);
            m.extend_from_slice(mlss.body());
            m.extend_from_slice(&ssx);
            m.extend_from_slice(&ctx);
            m.extend_from_slice(&pk[ML_PK..]);
            m.extend_from_slice(&XWING_LABEL);
            let want = both_sha3256(&m);
            assert_eq_bytes(
                "xwing ss != SHA3-256(ss_ml || ss_x || ct_x || pk_x || label)",
                &want,
                ss.body(),
            );
            cts.push(ctb.to_vec());
            n += 1;
        }
    }
    assert!(n >= 64, "only {n} iterations");
    let mut s = cts.clone();
    s.sort();
    s.dedup();
    assert_eq!(s.len(), cts.len(), "distinct (pk,seed) produced identical xwing ciphertexts");
}

/// CONFIGS 236: `crypto_kem_xwing_enc` / `_dec` full round-trip, including the
/// `[RNG]` entry point, plus decapsulation with the wrong secret key.
#[test]
fn cfg236_xwing_enc_dec_roundtrip() {
    let _g = rng_lock();
    init_both();
    install_det_rng(false);
    let mut rng = Rng::new(SEED ^ 236);
    let mut n = 0usize;
    let mut kps: Vec<(Vec<u8>, Vec<u8>)> = Vec::new();
    for i in 0..10 {
        let mut kseed = rng.bytes(XW_SEED);
        kseed[1] ^= i as u8;
        kps.push(xw_kp(&kseed));
    }
    for (i, (pk, sk)) in kps.iter().enumerate() {
        for skip in 0..7usize {
            let seed = rng_peek(skip, 64);
            let (rc, ct, ss) = d_enc("crypto_kem_xwing_enc", XW_CT, XW_SS, pk, "xwing");
            assert_eq!(rc, 0, "xwing enc returned {rc}");
            let (rc2, ct2, ss2) = d_enc_det(
                "crypto_kem_xwing_enc_deterministic",
                XW_CT,
                XW_SS,
                pk,
                &seed,
                "xwing",
            );
            assert_eq!(rc2, 0);
            assert_eq_bytes("xwing enc ct != enc_deterministic(next 64 RNG bytes)", ct2.all(), ct.all());
            assert_eq_bytes("xwing enc ss != enc_deterministic(next 64 RNG bytes)", ss2.all(), ss.all());

            let (rd, ssd) = d_dec("crypto_kem_xwing_dec", XW_SS, ct.body(), sk, "xwing");
            assert_eq!(rd, 0, "xwing dec returned {rd}");
            assert_eq_bytes("xwing enc/dec shared-secret mismatch", ss.body(), ssd.body());

            // the wrong secret key yields a different (but identical across
            // libraries) shared secret and still returns 0
            let other = &kps[(i + 1) % kps.len()].1;
            let (rw, ssw) = d_dec("crypto_kem_xwing_dec", XW_SS, ct.body(), other, "xwing");
            assert_eq!(rw, 0, "xwing dec with the wrong sk returned {rw}");
            assert_ne!(ssw.body(), ss.body(), "xwing dec accepted the wrong secret key");
            n += 1;
        }
    }
    assert!(n >= 64, "only {n} iterations");
    restore_sysrandom();
}

// ============================================================================
// CONFIGS 237 / 238 — crypto_kem dispatch and constants
// ============================================================================

/// CONFIGS 237: `crypto_kem_seed_keypair` / `_keypair` / `_enc` / `_dec` are
/// literal delegations to `crypto_kem_xwing_*`.
#[test]
fn cfg237_kem_dispatch() {
    let _g = rng_lock();
    init_both();
    install_det_rng(false);
    let mut rng = Rng::new(SEED ^ 237);
    let mut n = 0usize;
    for i in 0..14 {
        let mut seed = rng.bytes(XW_SEED);
        seed[2] ^= i as u8;
        let (r1, pk1, sk1) = d_seed_keypair("crypto_kem_seed_keypair", XW_PK, XW_SK, &seed);
        let (r2, pk2, sk2) =
            d_seed_keypair("crypto_kem_xwing_seed_keypair", XW_PK, XW_SK, &seed);
        assert_eq!(r1, 0);
        assert_eq!(r1, r2, "crypto_kem_seed_keypair return differs from xwing");
        assert_eq_bytes("crypto_kem_seed_keypair pk != xwing", pk2.all(), pk1.all());
        assert_eq_bytes("crypto_kem_seed_keypair sk != xwing", sk2.all(), sk1.all());

        for skip in 0..5usize {
            // _keypair
            rng_seek(skip);
            let (k1, kpk1, ksk1) = d_keypair("crypto_kem_keypair", XW_PK, XW_SK, "dispatch");
            rng_seek(skip);
            let (k2, kpk2, ksk2) = d_keypair("crypto_kem_xwing_keypair", XW_PK, XW_SK, "xwing");
            assert_eq!(k1, 0);
            assert_eq!(k1, k2, "crypto_kem_keypair return differs from xwing");
            assert_eq_bytes("crypto_kem_keypair pk != xwing", kpk2.all(), kpk1.all());
            assert_eq_bytes("crypto_kem_keypair sk != xwing", ksk2.all(), ksk1.all());

            // _enc
            rng_seek(skip);
            let (e1, ct1, ss1) = d_enc("crypto_kem_enc", XW_CT, XW_SS, pk1.body(), "dispatch");
            rng_seek(skip);
            let (e2, ct2, ss2) =
                d_enc("crypto_kem_xwing_enc", XW_CT, XW_SS, pk1.body(), "xwing");
            assert_eq!(e1, 0);
            assert_eq!(e1, e2, "crypto_kem_enc return differs from xwing");
            assert_eq_bytes("crypto_kem_enc ct != xwing", ct2.all(), ct1.all());
            assert_eq_bytes("crypto_kem_enc ss != xwing", ss2.all(), ss1.all());

            // _dec
            let (d1, sa) = d_dec("crypto_kem_dec", XW_SS, ct1.body(), sk1.body(), "dispatch");
            let (d2, sb) =
                d_dec("crypto_kem_xwing_dec", XW_SS, ct1.body(), sk1.body(), "xwing");
            assert_eq!(d1, 0);
            assert_eq!(d1, d2, "crypto_kem_dec return differs from xwing");
            assert_eq_bytes("crypto_kem_dec ss != xwing", sb.all(), sa.all());
            assert_eq_bytes("crypto_kem enc/dec round-trip", ss1.body(), sa.body());
            n += 1;
        }
    }
    assert!(n >= 64, "only {n} iterations");
    restore_sysrandom();
}

/// CONFIGS 237 + 238: every `crypto_kem*` constant getter and `_primitive`.
#[test]
fn cfg237_cfg238_kem_constant_getters() {
    init_both();
    // CONFIGS 238: mlkem768 1184 / 2400 / 1088 / 32 / 64
    getter("crypto_kem_mlkem768_publickeybytes", 1184);
    getter("crypto_kem_mlkem768_secretkeybytes", 2400);
    getter("crypto_kem_mlkem768_ciphertextbytes", 1088);
    getter("crypto_kem_mlkem768_sharedsecretbytes", 32);
    getter("crypto_kem_mlkem768_seedbytes", 64);
    // xwing 1216 / 32 / 1120 / 32 / 32
    getter("crypto_kem_xwing_publickeybytes", 1216);
    getter("crypto_kem_xwing_secretkeybytes", 32);
    getter("crypto_kem_xwing_ciphertextbytes", 1120);
    getter("crypto_kem_xwing_sharedsecretbytes", 32);
    getter("crypto_kem_xwing_seedbytes", 32);
    // CONFIGS 237: the generic names alias xwing
    getter("crypto_kem_publickeybytes", 1216);
    getter("crypto_kem_secretkeybytes", 32);
    getter("crypto_kem_ciphertextbytes", 1120);
    getter("crypto_kem_sharedsecretbytes", 32);
    getter("crypto_kem_seedbytes", 32);
    primitive("crypto_kem_primitive", "xwing");
    // xwing pk = mlkem pk + an X25519 point; ct likewise
    assert_eq!(XW_PK, ML_PK + 32);
    assert_eq!(XW_CT, ML_CT + 32);
}

// ============================================================================
// ERRORS 380 / 381 — ML-KEM-768 non-canonical public key
// ============================================================================

/// ERRORS 380: `polyvec_frombytes(pk)` yielding any coefficient ≥ `MLKEM768_Q`
/// (3329) makes `polyvec_is_canonical()` return 0 and
/// `crypto_kem_mlkem768_enc_deterministic` return -1 without writing `ct`/`ss`.
/// Only the first 1152 bytes are validated; the trailing 32-byte seed is not.
#[test]
fn e380_mlkem768_enc_deterministic_noncanonical_pk() {
    init_both();
    let mut rng = Rng::new(SEED ^ 380);
    let (pk0, _sk0) = ml_kp(&rng.bytes(ML_SEED));
    let eseed = rng.bytes(32);

    // the pack/unpack helper must agree with poly_frombytes / poly_tobytes
    {
        let mut probe = pk0.clone();
        for &(poly, j) in COEFF_SLOTS {
            for v in [0u16, 1, 3328, 3329, 4095] {
                set_coeff(&mut probe, poly, j, v);
                assert_eq!(get_coeff(&probe, poly, j), v, "12-bit pack helper is wrong");
            }
        }
    }

    let mut n = 0usize;
    for &(poly, j) in COEFF_SLOTS {
        // >= Q is rejected
        for v in [ML_Q, ML_Q + 1, 3400, 4000, 4095] {
            let mut pk = pk0.clone();
            set_coeff(&mut pk, poly, j, v);
            let (fc, fr) =
                unsafe { pair::<EncDetFn>("crypto_kem_mlkem768_enc_deterministic") };
            let mut cc = Out::new(ML_CT);
            let mut sc = Out::new(ML_SS);
            let mut cr = Out::new(ML_CT);
            let mut sr = Out::new(ML_SS);
            let rc = unsafe { fc(cc.ptr(), sc.ptr(), pk.as_ptr(), eseed.as_ptr()) };
            let rr = unsafe { fr(cr.ptr(), sr.ptr(), pk.as_ptr(), eseed.as_ptr()) };
            let what = format!("enc_deterministic non-canonical coeff[{poly}][{j}]={v}");
            assert_eq!(rc, rr, "{what}: return differs (C={rc} rust={rr})");
            assert_eq!(rc, -1, "{what}: expected -1, got {rc}");
            assert_eq_bytes(&format!("{what} ct"), cc.all(), cr.all());
            assert_eq_bytes(&format!("{what} ss"), sc.all(), sr.all());
            assert_untouched(&what, "C", &cc);
            assert_untouched(&what, "C", &sc);
            assert_untouched(&what, "rust", &cr);
            assert_untouched(&what, "rust", &sr);
            n += 1;
        }
        // < Q stays canonical: accepted, and the two libraries agree on the
        // (different) ciphertext
        for v in [0u16, 1, 1234, ML_Q - 1] {
            let mut pk = pk0.clone();
            set_coeff(&mut pk, poly, j, v);
            let (rc, _ct, _ss) = d_enc_det(
                "crypto_kem_mlkem768_enc_deterministic",
                ML_CT,
                ML_SS,
                &pk,
                &eseed,
                "canonical-tweak",
            );
            assert_eq!(rc, 0, "coeff[{poly}][{j}]={v} (< Q) must stay canonical");
            n += 1;
        }
    }
    // the trailing 32-byte public seed is NEVER validated
    for b in 0..32usize {
        let mut pk = pk0.clone();
        pk[ML_POLYVECBYTES + b] ^= 0xff;
        let (rc, _ct, _ss) = d_enc_det(
            "crypto_kem_mlkem768_enc_deterministic",
            ML_CT,
            ML_SS,
            &pk,
            &eseed,
            "seed-tweak",
        );
        assert_eq!(rc, 0, "pk[{}] (public seed) must not be validated", ML_POLYVECBYTES + b);
        n += 1;
    }
    // every coefficient slot in an all-0xff polyvec is 0xFFF >= Q
    {
        let mut pk = pk0.clone();
        for b in pk[..ML_POLYVECBYTES].iter_mut() {
            *b = 0xff;
        }
        let (fc, fr) = unsafe { pair::<EncDetFn>("crypto_kem_mlkem768_enc_deterministic") };
        let mut cc = Out::new(ML_CT);
        let mut sc = Out::new(ML_SS);
        let mut cr = Out::new(ML_CT);
        let mut sr = Out::new(ML_SS);
        let rc = unsafe { fc(cc.ptr(), sc.ptr(), pk.as_ptr(), eseed.as_ptr()) };
        let rr = unsafe { fr(cr.ptr(), sr.ptr(), pk.as_ptr(), eseed.as_ptr()) };
        assert_eq!(rc, rr);
        assert_eq!(rc, -1, "an all-0xff polyvec must be rejected");
        assert_untouched("all-0xff polyvec", "C", &cc);
        assert_untouched("all-0xff polyvec", "rust", &cr);
        assert_untouched("all-0xff polyvec", "C", &sc);
        assert_untouched("all-0xff polyvec", "rust", &sr);
        n += 1;
    }
    assert!(n >= 64, "only {n} iterations");
}

/// ERRORS 381: `crypto_kem_mlkem768_enc` propagates the inner
/// `_enc_deterministic` failure (non-canonical pk) regardless of the RNG.
#[test]
fn e381_mlkem768_enc_noncanonical_pk() {
    let _g = rng_lock();
    init_both();
    install_det_rng(false);
    let mut rng = Rng::new(SEED ^ 381);
    let (pk0, _sk0) = ml_kp(&rng.bytes(ML_SEED));
    let (fc, fr) = unsafe { pair::<EncFn>("crypto_kem_mlkem768_enc") };
    let mut n = 0usize;
    for &(poly, j) in COEFF_SLOTS {
        for v in [ML_Q, 4095] {
            for skip in 0..3usize {
                let mut pk = pk0.clone();
                set_coeff(&mut pk, poly, j, v);
                rng_seek(skip);
                let mut cc = Out::new(ML_CT);
                let mut sc = Out::new(ML_SS);
                let mut cr = Out::new(ML_CT);
                let mut sr = Out::new(ML_SS);
                let rc = unsafe { fc(cc.ptr(), sc.ptr(), pk.as_ptr()) };
                let rr = unsafe { fr(cr.ptr(), sr.ptr(), pk.as_ptr()) };
                let what =
                    format!("crypto_kem_mlkem768_enc non-canonical coeff[{poly}][{j}]={v}");
                assert_eq!(rc, rr, "{what}: return differs (C={rc} rust={rr})");
                assert_eq!(rc, -1, "{what}: expected -1, got {rc}");
                assert_eq_bytes(&format!("{what} ct"), cc.all(), cr.all());
                assert_eq_bytes(&format!("{what} ss"), sc.all(), sr.all());
                assert_untouched(&what, "C", &cc);
                assert_untouched(&what, "C", &sc);
                assert_untouched(&what, "rust", &cr);
                assert_untouched(&what, "rust", &sr);
                n += 1;
            }
        }
    }
    assert!(n >= 64, "only {n} iterations");
    restore_sysrandom();
}

// ============================================================================
// ERRORS 383 / 384 / 385 / 386 / 387 — X-Wing failure paths
// ============================================================================

/// ERRORS 383: `crypto_kem_xwing_enc_deterministic` returns -1 when the ML-KEM
/// part of the public key is non-canonical (the inner
/// `crypto_kem_mlkem768_enc_deterministic` fails).
#[test]
fn e383_xwing_enc_deterministic_noncanonical_mlkem_pk() {
    init_both();
    let mut rng = Rng::new(SEED ^ 383);
    let (pk0, _sk0) = xw_kp(&rng.bytes(XW_SEED));
    let eseed = rng.bytes(64);
    let (fc, fr) = unsafe { pair::<EncDetFn>("crypto_kem_xwing_enc_deterministic") };
    let mut n = 0usize;
    for &(poly, j) in COEFF_SLOTS {
        for v in [ML_Q, ML_Q + 1, 3330, 3999, 4095] {
            let mut pk = pk0.clone();
            set_coeff(&mut pk[..ML_PK], poly, j, v);
            let mut cc = Out::new(XW_CT);
            let mut sc = Out::new(XW_SS);
            let mut cr = Out::new(XW_CT);
            let mut sr = Out::new(XW_SS);
            let rc = unsafe { fc(cc.ptr(), sc.ptr(), pk.as_ptr(), eseed.as_ptr()) };
            let rr = unsafe { fr(cr.ptr(), sr.ptr(), pk.as_ptr(), eseed.as_ptr()) };
            let what = format!("xwing enc_deterministic non-canonical coeff[{poly}][{j}]={v}");
            assert_eq!(rc, rr, "{what}: return differs (C={rc} rust={rr})");
            assert_eq!(rc, -1, "{what}: expected -1, got {rc}");
            assert_eq_bytes(&format!("{what} ct"), cc.all(), cr.all());
            assert_eq_bytes(&format!("{what} ss"), sc.all(), sr.all());
            assert_untouched(&what, "C", &cc);
            assert_untouched(&what, "C", &sc);
            assert_untouched(&what, "rust", &cr);
            assert_untouched(&what, "rust", &sr);
            n += 1;
        }
        // a canonical tweak of the same slot is still accepted
        let mut pk = pk0.clone();
        set_coeff(&mut pk[..ML_PK], poly, j, ML_Q - 1);
        let (rc, _ct, _ss) =
            d_enc_det("crypto_kem_xwing_enc_deterministic", XW_CT, XW_SS, &pk, &eseed, "xwing");
        assert_eq!(rc, 0, "canonical coefficient tweak must be accepted");
        n += 1;
    }
    assert!(n >= 64, "only {n} iterations");
}

/// ERRORS 384: `crypto_kem_xwing_enc_deterministic` returns -1 when the X25519
/// part of the public key is one of the 7 blocklisted low-order encodings (or
/// their bit-7 variants).
#[test]
fn e384_xwing_enc_deterministic_low_order_x25519_pk() {
    init_both();
    let mut rng = Rng::new(SEED ^ 384);
    let (pk0, _sk0) = xw_kp(&rng.bytes(XW_SEED));
    let (fc, fr) = unsafe { pair::<EncDetFn>("crypto_kem_xwing_enc_deterministic") };
    let mut seeds: Vec<Vec<u8>> = vec![vec![0u8; 64], vec![0xffu8; 64]];
    for _ in 0..3 {
        seeds.push(rng.bytes(64));
    }
    let mut n = 0usize;
    for bad in low_order_points() {
        for seed in &seeds {
            let mut pk = pk0.clone();
            pk[ML_PK..].copy_from_slice(&bad);
            let mut cc = Out::new(XW_CT);
            let mut sc = Out::new(XW_SS);
            let mut cr = Out::new(XW_CT);
            let mut sr = Out::new(XW_SS);
            let rc = unsafe { fc(cc.ptr(), sc.ptr(), pk.as_ptr(), seed.as_ptr()) };
            let rr = unsafe { fr(cr.ptr(), sr.ptr(), pk.as_ptr(), seed.as_ptr()) };
            let what = format!("xwing enc_deterministic low-order pk_x={}", hexs(&bad));
            assert_eq!(rc, rr, "{what}: return differs (C={rc} rust={rr})");
            assert_eq!(rc, -1, "{what}: expected -1, got {rc}");
            assert_eq_bytes(&format!("{what} ct"), cc.all(), cr.all());
            assert_eq_bytes(&format!("{what} ss"), sc.all(), sr.all());
            // the inner ML-KEM encapsulation succeeded but `ct` is only copied
            // after the X25519 agreement, so nothing may have been written
            assert_untouched(&what, "C", &cc);
            assert_untouched(&what, "C", &sc);
            assert_untouched(&what, "rust", &cr);
            assert_untouched(&what, "rust", &sr);
            n += 1;
        }
    }
    assert!(n >= 64, "only {n} iterations");
}

/// ERRORS 385: `crypto_kem_xwing_enc` propagates either sub-failure.
#[test]
fn e385_xwing_enc_rng_sub_failures() {
    let _g = rng_lock();
    init_both();
    install_det_rng(false);
    let mut rng = Rng::new(SEED ^ 385);
    let (pk0, _sk0) = xw_kp(&rng.bytes(XW_SEED));
    let (fc, fr) = unsafe { pair::<EncFn>("crypto_kem_xwing_enc") };
    let mut n = 0usize;

    let mut bad_pks: Vec<(String, Vec<u8>)> = Vec::new();
    for &(poly, j) in COEFF_SLOTS {
        let mut pk = pk0.clone();
        set_coeff(&mut pk[..ML_PK], poly, j, 4095);
        bad_pks.push((format!("non-canonical coeff[{poly}][{j}]"), pk));
    }
    for bad in low_order_points() {
        let mut pk = pk0.clone();
        pk[ML_PK..].copy_from_slice(&bad);
        bad_pks.push((format!("low-order pk_x={}", hexs(&bad)), pk));
    }
    for (why, pk) in &bad_pks {
        for skip in 0..3usize {
            rng_seek(skip);
            let mut cc = Out::new(XW_CT);
            let mut sc = Out::new(XW_SS);
            let mut cr = Out::new(XW_CT);
            let mut sr = Out::new(XW_SS);
            let rc = unsafe { fc(cc.ptr(), sc.ptr(), pk.as_ptr()) };
            let rr = unsafe { fr(cr.ptr(), sr.ptr(), pk.as_ptr()) };
            let what = format!("crypto_kem_xwing_enc {why} rngskip={skip}");
            assert_eq!(rc, rr, "{what}: return differs (C={rc} rust={rr})");
            assert_eq!(rc, -1, "{what}: expected -1, got {rc}");
            assert_eq_bytes(&format!("{what} ct"), cc.all(), cr.all());
            assert_eq_bytes(&format!("{what} ss"), sc.all(), sr.all());
            assert_untouched(&what, "C", &cc);
            assert_untouched(&what, "C", &sc);
            assert_untouched(&what, "rust", &cr);
            assert_untouched(&what, "rust", &sr);
            n += 1;
        }
    }
    assert!(n >= 64, "only {n} iterations");
    restore_sysrandom();
}

/// ERRORS 386: `crypto_kem_xwing_dec` returns -1 when the X25519 half of the
/// ciphertext is low order. (`crypto_kem_mlkem768_dec` never fails, so this is
/// the only reachable -1 in `_dec`.)
#[test]
fn e386_xwing_dec_low_order_ct() {
    init_both();
    let mut rng = Rng::new(SEED ^ 386);
    let (fc, fr) = unsafe { pair::<DecFn>("crypto_kem_xwing_dec") };
    let mut n = 0usize;
    for i in 0..5 {
        let mut kseed = rng.bytes(XW_SEED);
        kseed[0] ^= i as u8;
        let (pk, sk) = xw_kp(&kseed);
        let (rc, ct, _ss) = d_enc_det(
            "crypto_kem_xwing_enc_deterministic",
            XW_CT,
            XW_SS,
            &pk,
            &rng.bytes(64),
            "xwing",
        );
        assert_eq!(rc, 0);
        for bad in low_order_points() {
            let mut c = ct.body().to_vec();
            c[ML_CT..].copy_from_slice(&bad);
            let mut sc = Out::new(XW_SS);
            let mut sr = Out::new(XW_SS);
            let a = unsafe { fc(sc.ptr(), c.as_ptr(), sk.as_ptr()) };
            let b = unsafe { fr(sr.ptr(), c.as_ptr(), sk.as_ptr()) };
            let what = format!("xwing dec low-order ct_x={}", hexs(&bad));
            assert_eq!(a, b, "{what}: return differs (C={a} rust={b})");
            assert_eq!(a, -1, "{what}: expected -1, got {a}");
            assert_eq_bytes(&format!("{what} ss"), sc.all(), sr.all());
            assert_untouched(&what, "C", &sc);
            assert_untouched(&what, "rust", &sr);
            n += 1;
        }
        // tampering the ML-KEM half instead still returns 0 (implicit rejection)
        for p in [0usize, 1, 500, 1087] {
            let mut c = ct.body().to_vec();
            c[p] ^= 0x01;
            let (r, _s) = d_dec("crypto_kem_xwing_dec", XW_SS, &c, &sk, "xwing");
            assert_eq!(r, 0, "xwing dec must return 0 for a tampered ML-KEM ct byte {p}");
            n += 1;
        }
    }
    assert!(n >= 64, "only {n} iterations");
}

/// ERRORS 387: `crypto_kem_enc` / `crypto_kem_dec` are thin dispatches to
/// `crypto_kem_xwing_*`; there is no unknown-primitive path, so they simply
/// propagate 0 / -1.
#[test]
fn e387_kem_enc_dec_dispatch_propagates() {
    let _g = rng_lock();
    init_both();
    install_det_rng(false);
    let mut rng = Rng::new(SEED ^ 387);
    let (pk0, sk0) = xw_kp(&rng.bytes(XW_SEED));
    let (ec, er) = unsafe { pair::<EncFn>("crypto_kem_enc") };
    let (dc, dr) = unsafe { pair::<DecFn>("crypto_kem_dec") };
    let mut n = 0usize;

    // failing enc: non-canonical ML-KEM pk and low-order X25519 pk
    let mut bad_pks: Vec<(String, Vec<u8>)> = Vec::new();
    for &(poly, j) in COEFF_SLOTS {
        let mut pk = pk0.clone();
        set_coeff(&mut pk[..ML_PK], poly, j, ML_Q);
        bad_pks.push((format!("non-canonical coeff[{poly}][{j}]"), pk));
    }
    for bad in low_order_points() {
        let mut pk = pk0.clone();
        pk[ML_PK..].copy_from_slice(&bad);
        bad_pks.push((format!("low-order pk_x={}", hexs(&bad)), pk));
    }
    for (why, pk) in &bad_pks {
        for skip in 0..2usize {
            rng_seek(skip);
            let mut cc = Out::new(XW_CT);
            let mut sc = Out::new(XW_SS);
            let mut cr = Out::new(XW_CT);
            let mut sr = Out::new(XW_SS);
            let a = unsafe { ec(cc.ptr(), sc.ptr(), pk.as_ptr()) };
            let b = unsafe { er(cr.ptr(), sr.ptr(), pk.as_ptr()) };
            let what = format!("crypto_kem_enc {why}");
            assert_eq!(a, b, "{what}: return differs (C={a} rust={b})");
            assert_eq!(a, -1, "{what}: expected -1, got {a}");
            assert_eq_bytes(&format!("{what} ct"), cc.all(), cr.all());
            assert_eq_bytes(&format!("{what} ss"), sc.all(), sr.all());
            assert_untouched(&what, "C", &cc);
            assert_untouched(&what, "rust", &cr);
            n += 1;
        }
    }
    // failing dec: low-order X25519 ciphertext half
    rng_seek(0);
    let (rc, ct, ss) = d_enc("crypto_kem_enc", XW_CT, XW_SS, &pk0, "dispatch");
    assert_eq!(rc, 0);
    for bad in low_order_points() {
        let mut c = ct.body().to_vec();
        c[ML_CT..].copy_from_slice(&bad);
        let mut sc = Out::new(XW_SS);
        let mut sr = Out::new(XW_SS);
        let a = unsafe { dc(sc.ptr(), c.as_ptr(), sk0.as_ptr()) };
        let b = unsafe { dr(sr.ptr(), c.as_ptr(), sk0.as_ptr()) };
        let what = format!("crypto_kem_dec low-order ct_x={}", hexs(&bad));
        assert_eq!(a, b, "{what}: return differs (C={a} rust={b})");
        assert_eq!(a, -1, "{what}: expected -1, got {a}");
        assert_eq_bytes(&format!("{what} ss"), sc.all(), sr.all());
        assert_untouched(&what, "C", &sc);
        assert_untouched(&what, "rust", &sr);
        n += 1;
    }
    // and the success path still propagates 0 with the right shared secret
    let (a, sa) = d_dec("crypto_kem_dec", XW_SS, ct.body(), &sk0, "dispatch");
    assert_eq!(a, 0);
    assert_eq_bytes("crypto_kem_enc/dec round-trip", ss.body(), sa.body());
    assert!(n >= 64, "only {n} iterations");
    restore_sysrandom();
}

// ============================================================================
// CONFIGS 239 — crypto_kx_seed_keypair
// ============================================================================

/// CONFIGS 239: `sk = BLAKE2b-32(seed)` then `pk = X25519 base * sk`.
#[test]
fn cfg239_kx_seed_keypair() {
    init_both();
    let mut rng = Rng::new(SEED ^ 239);
    let mut seeds: Vec<Vec<u8>> =
        vec![vec![0u8; KX_B], vec![0xffu8; KX_B], (0..KX_B as u8).collect()];
    for _ in 0..70 {
        seeds.push(rng.bytes(KX_B));
    }
    let mut n = 0usize;
    let mut pks: Vec<Vec<u8>> = Vec::new();
    for seed in &seeds {
        let (rc, pk, sk) = d_seed_keypair("crypto_kx_seed_keypair", KX_B, KX_B, seed);
        assert_eq!(rc, 0, "crypto_kx_seed_keypair returned {rc}");
        let want_sk = both_blake2b(KX_B, seed);
        assert_eq_bytes("kx sk != BLAKE2b-32(seed)", &want_sk, sk.body());
        let (rb, want_pk) = both_scalarmult_base(sk.body());
        assert_eq!(rb, 0);
        assert_eq_bytes("kx pk != crypto_scalarmult_base(sk)", &want_pk, pk.body());
        pks.push(pk.body().to_vec());
        n += 1;
    }
    assert!(n >= 64, "only {n} iterations");
    let mut s = pks.clone();
    s.sort();
    s.dedup();
    assert_eq!(s.len(), pks.len(), "distinct seeds produced identical kx public keys");
}

// ============================================================================
// CONFIGS 240 — crypto_kx_keypair [RNG]
// ============================================================================

/// CONFIGS 240: `sk` is the next 32 bytes of `randombytes_buf` (NOT hashed) and
/// `pk = X25519 base * sk`.
#[test]
fn cfg240_kx_keypair_rng() {
    let _g = rng_lock();
    init_both();
    install_det_rng(false);
    let mut n = 0usize;
    for skip in 0..70usize {
        let want_sk = rng_peek(skip, KX_B);
        let (rc, pk, sk) = d_keypair("crypto_kx_keypair", KX_B, KX_B, "x25519blake2b");
        assert_eq!(rc, 0, "crypto_kx_keypair returned {rc}");
        assert_eq_bytes(
            &format!("kx keypair(rngskip={skip}): sk is not the next 32 RNG bytes"),
            &want_sk,
            sk.body(),
        );
        let (rb, want_pk) = both_scalarmult_base(sk.body());
        assert_eq!(rb, 0);
        assert_eq_bytes("kx pk != crypto_scalarmult_base(sk)", &want_pk, pk.body());
        n += 1;
    }
    assert!(n >= 64, "only {n} iterations");
    restore_sysrandom();
}

// ============================================================================
// CONFIGS 241 — full crypto_kx handshake
// ============================================================================

/// Run `crypto_kx_{client,server}_session_keys` through both libraries with the
/// requested NULL-ness of `rx` / `tx`. Returns `(rc, rx_out, tx_out)` where a
/// NULL side is reported as `None`.
fn d_session(
    name: &str,
    want_rx: bool,
    want_tx: bool,
    pk: &[u8],
    sk: &[u8],
    peer: &[u8],
) -> (c_int, Option<Out>, Option<Out>) {
    let (fc, fr) = unsafe { pair::<SessionFn>(name) };
    let mut rxc = Out::new(KX_B);
    let mut txc = Out::new(KX_B);
    let mut rxr = Out::new(KX_B);
    let mut txr = Out::new(KX_B);
    let (prxc, ptxc) = (
        if want_rx { rxc.ptr() } else { std::ptr::null_mut() },
        if want_tx { txc.ptr() } else { std::ptr::null_mut() },
    );
    let (prxr, ptxr) = (
        if want_rx { rxr.ptr() } else { std::ptr::null_mut() },
        if want_tx { txr.ptr() } else { std::ptr::null_mut() },
    );
    let rc = unsafe { fc(prxc, ptxc, pk.as_ptr(), sk.as_ptr(), peer.as_ptr()) };
    let rr = unsafe { fr(prxr, ptxr, pk.as_ptr(), sk.as_ptr(), peer.as_ptr()) };
    let what = format!("{name} rx={} tx={}", want_rx, want_tx);
    assert_eq!(rc, rr, "{what}: return differs (C={rc} rust={rr})");
    assert_eq_bytes(&format!("{what} rx"), rxc.all(), rxr.all());
    assert_eq_bytes(&format!("{what} tx"), txc.all(), txr.all());
    guard_intact(&what, "C", &rxc);
    guard_intact(&what, "C", &txc);
    guard_intact(&what, "rust", &rxr);
    guard_intact(&what, "rust", &txr);
    (rc, if want_rx { Some(rxc) } else { None }, if want_tx { Some(txc) } else { None })
}

/// CONFIGS 241: the full handshake. `client.tx == server.rx`,
/// `client.rx == server.tx`, and both sides are
/// `BLAKE2b-64(q ‖ client_pk ‖ server_pk)` split in opposite orders.
#[test]
fn cfg241_kx_full_handshake() {
    init_both();
    let mut rng = Rng::new(SEED ^ 241);
    let mut n = 0usize;
    let mut all_rx: Vec<Vec<u8>> = Vec::new();
    for i in 0..70 {
        let mut cs = rng.bytes(KX_B);
        let mut ss = rng.bytes(KX_B);
        cs[0] ^= i as u8;
        ss[31] ^= i as u8;
        let (rc1, cpk, csk) = d_seed_keypair("crypto_kx_seed_keypair", KX_B, KX_B, &cs);
        let (rc2, spk, ssk) = d_seed_keypair("crypto_kx_seed_keypair", KX_B, KX_B, &ss);
        assert_eq!((rc1, rc2), (0, 0));
        let (cpk, csk) = (cpk.body().to_vec(), csk.body().to_vec());
        let (spk, ssk) = (spk.body().to_vec(), ssk.body().to_vec());

        let (rc, crx, ctx) =
            d_session("crypto_kx_client_session_keys", true, true, &cpk, &csk, &spk);
        assert_eq!(rc, 0, "client_session_keys returned {rc}");
        let (rs, srx, stx) =
            d_session("crypto_kx_server_session_keys", true, true, &spk, &ssk, &cpk);
        assert_eq!(rs, 0, "server_session_keys returned {rs}");
        let (crx, ctx) = (crx.unwrap(), ctx.unwrap());
        let (srx, stx) = (srx.unwrap(), stx.unwrap());

        assert_eq_bytes("client.tx != server.rx", ctx.body(), srx.body());
        assert_eq_bytes("client.rx != server.tx", crx.body(), stx.body());
        assert_ne!(crx.body(), ctx.body(), "rx and tx must differ");

        // keys = BLAKE2b(outlen=64, q || client_pk || server_pk)
        let (rq, q) = both_scalarmult(&csk, &spk);
        assert_eq!(rq, 0);
        let (rq2, q2) = both_scalarmult(&ssk, &cpk);
        assert_eq!(rq2, 0);
        assert_eq_bytes("ECDH is not symmetric", &q, &q2);
        let mut m = Vec::with_capacity(96);
        m.extend_from_slice(&q);
        m.extend_from_slice(&cpk);
        m.extend_from_slice(&spk);
        let keys = both_blake2b(64, &m);
        assert_eq_bytes("client.rx != keys[0..32]", &keys[..32], crx.body());
        assert_eq_bytes("client.tx != keys[32..64]", &keys[32..], ctx.body());
        assert_eq_bytes("server.tx != keys[0..32]", &keys[..32], stx.body());
        assert_eq_bytes("server.rx != keys[32..64]", &keys[32..], srx.body());

        all_rx.push(crx.body().to_vec());
        n += 1;
    }
    assert!(n >= 64, "only {n} iterations");
    let mut s = all_rx.clone();
    s.sort();
    s.dedup();
    assert_eq!(s.len(), all_rx.len(), "distinct handshakes produced identical session keys");
}

// ============================================================================
// CONFIGS 242 / 243 — one NULL output buffer (VALID, aliased)
// ============================================================================

fn session_null_shapes_body(name: &str, is_client: bool, tweak: u64) {
    init_both();
    let mut rng = Rng::new(SEED ^ tweak);
    let mut n = 0usize;
    for i in 0..70 {
        let mut a = rng.bytes(KX_B);
        let mut b = rng.bytes(KX_B);
        a[1] ^= i as u8;
        b[2] ^= i as u8;
        let (r1, pk, sk) = d_seed_keypair("crypto_kx_seed_keypair", KX_B, KX_B, &a);
        let (r2, ppk, _psk) = d_seed_keypair("crypto_kx_seed_keypair", KX_B, KX_B, &b);
        assert_eq!((r1, r2), (0, 0));
        let (pk, sk, ppk) = (pk.body().to_vec(), sk.body().to_vec(), ppk.body().to_vec());

        let (rc, rx, tx) = d_session(name, true, true, &pk, &sk, &ppk);
        assert_eq!(rc, 0);
        let (rx, tx) = (rx.unwrap().body().to_vec(), tx.unwrap().body().to_vec());

        // `rx == NULL` aliases rx onto tx; the loop writes rx[i] then tx[i], so
        // for the client the last write wins and the single buffer holds
        // `keys[32..64)` (== tx). For the server the order is tx[i] then rx[i],
        // so the single buffer holds `keys[32..64)` (== rx).
        let winner: &[u8] = if is_client { &tx } else { &rx };

        let (r, orx, otx) = d_session(name, false, true, &pk, &sk, &ppk);
        assert_eq!(r, 0, "{name} with rx == NULL must succeed, got {r}");
        assert!(orx.is_none());
        let otx = otx.unwrap();
        assert_eq_bytes(
            &format!("{name}: rx == NULL (tx-only) wrote the wrong aliased half"),
            winner,
            otx.body(),
        );
        guard_intact(&format!("{name} rx=NULL"), "C", &otx);

        let (r, orx, otx) = d_session(name, true, false, &pk, &sk, &ppk);
        assert_eq!(r, 0, "{name} with tx == NULL must succeed, got {r}");
        assert!(otx.is_none());
        let orx = orx.unwrap();
        assert_eq_bytes(
            &format!("{name}: tx == NULL (rx-only) wrote the wrong aliased half"),
            winner,
            orx.body(),
        );
        guard_intact(&format!("{name} tx=NULL"), "C", &orx);
        n += 1;
    }
    assert!(n >= 64, "only {n} iterations");
}

/// CONFIGS 242: `crypto_kx_client_session_keys` with `rx == NULL` (tx-only) and
/// with `tx == NULL` (rx-only) — VALID shapes where the two output arguments
/// are aliased onto one buffer.
#[test]
fn cfg242_kx_client_session_keys_null_shapes() {
    session_null_shapes_body("crypto_kx_client_session_keys", true, 242);
}

/// CONFIGS 243: the same two shapes for `crypto_kx_server_session_keys`.
#[test]
fn cfg243_kx_server_session_keys_null_shapes() {
    session_null_shapes_body("crypto_kx_server_session_keys", false, 243);
}

// ============================================================================
// CONFIGS 244 — crypto_kx constants
// ============================================================================

/// CONFIGS 244: `_publickeybytes` / `_secretkeybytes` / `_seedbytes` /
/// `_sessionkeybytes` are all 32; `_primitive` is `"x25519blake2b"`.
#[test]
fn cfg244_kx_constant_getters() {
    init_both();
    getter("crypto_kx_publickeybytes", 32);
    getter("crypto_kx_secretkeybytes", 32);
    getter("crypto_kx_seedbytes", 32);
    getter("crypto_kx_sessionkeybytes", 32);
    primitive("crypto_kx_primitive", "x25519blake2b");
    // the COMPILER_ASSERTs in the C source: kx keys are plain curve25519 keys
    getter("crypto_scalarmult_scalarbytes", 32);
    getter("crypto_scalarmult_bytes", 32);
}

// ============================================================================
// ERRORS 388 / 390 — rx == NULL && tx == NULL is a sodium_misuse()
// ============================================================================

/// ERRORS 388 + 390: with BOTH output buffers NULL the aliasing dance leaves
/// `rx == NULL`, which reaches `sodium_misuse()` -> `abort()`. Both libraries
/// must die with SIGABRT.
#[test]
fn e388_e390_kx_both_outputs_null_misuse() {
    init_both();
    no_core_dumps();
    let mut rng = Rng::new(SEED ^ 388);
    let (r1, pk, sk) = d_seed_keypair("crypto_kx_seed_keypair", KX_B, KX_B, &rng.bytes(KX_B));
    let (r2, ppk, _s) = d_seed_keypair("crypto_kx_seed_keypair", KX_B, KX_B, &rng.bytes(KX_B));
    assert_eq!((r1, r2), (0, 0));
    let (pk, sk, ppk) = (pk.body().to_vec(), sk.body().to_vec(), ppk.body().to_vec());

    for name in ["crypto_kx_client_session_keys", "crypto_kx_server_session_keys"] {
        // Resolve the symbols and freeze every pointer in the PARENT; the child
        // closure only calls a pre-resolved function pointer.
        let (fc, fr) = unsafe { pair::<SessionFn>(name) };
        let cf = *fc;
        let rf = *fr;
        let ppk_p = pk.as_ptr();
        let psk_p = sk.as_ptr();
        let peer_p = ppk.as_ptr();
        let run = |f: SessionFn| -> i64 {
            unsafe { f(std::ptr::null_mut(), std::ptr::null_mut(), ppk_p, psk_p, peer_p) as i64 }
        };
        let a = forked(|| run(cf));
        let b = forked(|| run(rf));
        assert_same_fatal(&format!("{name} rx == tx == NULL"), a, b);
        assert_eq!(
            a,
            Outcome::Signaled(SIGABRT),
            "{name} with rx == tx == NULL must sodium_misuse() -> SIGABRT, got {a:?}"
        );
        // a low-order peer AND both outputs NULL: the misuse check runs first
        let bad = LOW_ORDER[1];
        let bad_p = bad.as_ptr();
        let run2 = |f: SessionFn| -> i64 {
            unsafe { f(std::ptr::null_mut(), std::ptr::null_mut(), ppk_p, psk_p, bad_p) as i64 }
        };
        let a2 = forked(|| run2(cf));
        let b2 = forked(|| run2(rf));
        assert_same_fatal(&format!("{name} rx == tx == NULL, low-order peer"), a2, b2);
        assert_eq!(
            a2,
            Outcome::Signaled(SIGABRT),
            "{name}: the NULL check must precede the scalarmult, got {a2:?}"
        );
    }
}

// ============================================================================
// ERRORS 389 / 391 — low-order peer public key
// ============================================================================

fn session_low_order_body(name: &str, tweak: u64) {
    init_both();
    let mut rng = Rng::new(SEED ^ tweak);
    let mut n = 0usize;
    for i in 0..6 {
        let mut s = rng.bytes(KX_B);
        s[0] ^= i as u8;
        let (rc, pk, sk) = d_seed_keypair("crypto_kx_seed_keypair", KX_B, KX_B, &s);
        assert_eq!(rc, 0);
        let (pk, sk) = (pk.body().to_vec(), sk.body().to_vec());
        for bad in low_order_points() {
            // all three output shapes
            for (wrx, wtx) in [(true, true), (false, true), (true, false)] {
                let (r, orx, otx) = d_session(name, wrx, wtx, &pk, &sk, &bad);
                let what = format!("{name} low-order peer={} rx={wrx} tx={wtx}", hexs(&bad));
                assert_eq!(r, -1, "{what}: expected -1, got {r}");
                if let Some(o) = &orx {
                    assert_untouched(&what, "C", o);
                }
                if let Some(o) = &otx {
                    assert_untouched(&what, "C", o);
                }
                n += 1;
            }
        }
        // a legitimate peer key is still accepted
        let (r2, ppk, _p) = d_seed_keypair("crypto_kx_seed_keypair", KX_B, KX_B, &rng.bytes(KX_B));
        assert_eq!(r2, 0);
        let (r, _, _) = d_session(name, true, true, &pk, &sk, ppk.body());
        assert_eq!(r, 0, "{name} rejected a valid peer public key");
        n += 1;
    }
    assert!(n >= 64, "only {n} iterations");
}

/// ERRORS 389: `crypto_kx_client_session_keys` returns -1 when
/// `crypto_scalarmult(q, client_sk, server_pk)` rejects a low-order
/// `server_pk`; no output byte may be written.
#[test]
fn e389_kx_client_low_order_server_pk() {
    session_low_order_body("crypto_kx_client_session_keys", 389);
}

/// ERRORS 391: the same for `crypto_kx_server_session_keys` and a low-order
/// `client_pk`.
#[test]
fn e391_kx_server_low_order_client_pk() {
    session_low_order_body("crypto_kx_server_session_keys", 391);
}
