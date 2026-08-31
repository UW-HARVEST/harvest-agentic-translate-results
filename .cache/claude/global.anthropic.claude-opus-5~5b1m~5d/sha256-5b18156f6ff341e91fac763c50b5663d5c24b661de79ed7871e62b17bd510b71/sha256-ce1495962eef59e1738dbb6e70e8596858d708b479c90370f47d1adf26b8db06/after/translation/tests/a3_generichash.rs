//! Area 3 — `crypto_generichash` / BLAKE2b.
//!
//! Covers `crypto_generichash/crypto_generichash.c`,
//! `crypto_generichash/blake2b/generichash_blake2.c` and
//! `crypto_generichash/blake2b/ref/{blake2b-ref.c, blake2b-compress-ref.c,
//! generichash_blake2b.c}`.
//!
//! CONFIGS rows 3.84–3.116, ERRORS rows 3.34–3.100.
//!
//! Every call goes through `dlsym` on both shared objects.  Where a state
//! object is involved the *entire* opaque state (`crypto_generichash_statebytes()`
//! bytes) is compared after `init` and after every single `update`, which is a
//! far stronger check than comparing digests only: it pins `h`, the 128-bit
//! counter `t`, the finalization flags `f`, the whole 256-byte lazy buffer,
//! `buflen` and `last_node` byte for byte.
mod common;
use common::*;
use libloading::Symbol;
use std::ffi::{c_char, c_int, c_void};

// --------------------------------------------------------------- signatures

/// `crypto_generichash{,_blake2b}(out, outlen, in, inlen, key, keylen)`
type Gh = unsafe extern "C" fn(*mut u8, usize, *const u8, u64, *const u8, usize) -> c_int;
/// `crypto_generichash_blake2b_salt_personal(out, outlen, in, inlen, key, keylen, salt, personal)`
type GhSp =
    unsafe extern "C" fn(*mut u8, usize, *const u8, u64, *const u8, usize, *const u8, *const u8) -> c_int;
/// `crypto_generichash{,_blake2b}_init(state, key, keylen, outlen)`
type GhInit = unsafe extern "C" fn(*mut c_void, *const u8, usize, usize) -> c_int;
/// `crypto_generichash_blake2b_init_salt_personal(state, key, keylen, outlen, salt, personal)`
type GhInitSp =
    unsafe extern "C" fn(*mut c_void, *const u8, usize, usize, *const u8, *const u8) -> c_int;
/// `crypto_generichash{,_blake2b}_update(state, in, inlen)`
type GhUpdate = unsafe extern "C" fn(*mut c_void, *const u8, u64) -> c_int;
/// `crypto_generichash{,_blake2b}_final(state, out, outlen)`
type GhFinal = unsafe extern "C" fn(*mut c_void, *mut u8, usize) -> c_int;

type SizeFn = unsafe extern "C" fn() -> usize;
type IntFn = unsafe extern "C" fn() -> c_int;
type StrFn = unsafe extern "C" fn() -> *const c_char;
type Keygen = unsafe extern "C" fn(*mut u8);

// low-level `blake2b-ref.c` entry points (exported as `_sodium_blake2b*`)
type B2Init = unsafe extern "C" fn(*mut c_void, u8) -> c_int;
type B2InitSp = unsafe extern "C" fn(*mut c_void, u8, *const c_void, *const c_void) -> c_int;
type B2InitKey = unsafe extern "C" fn(*mut c_void, u8, *const c_void, u8) -> c_int;
type B2InitKeySp =
    unsafe extern "C" fn(*mut c_void, u8, *const c_void, u8, *const c_void, *const c_void) -> c_int;
type B2Update = unsafe extern "C" fn(*mut c_void, *const u8, u64) -> c_int;
type B2Final = unsafe extern "C" fn(*mut c_void, *mut u8, u8) -> c_int;
type B2InitParam = unsafe extern "C" fn(*mut c_void, *const u8) -> c_int;
type B2OneShot = unsafe extern "C" fn(*mut u8, *const c_void, *const c_void, u8, u64, u8) -> c_int;
type B2OneShotSp = unsafe extern "C" fn(
    *mut u8,
    *const c_void,
    *const c_void,
    u8,
    u64,
    u8,
    *const c_void,
    *const c_void,
) -> c_int;

// ------------------------------------------------------------------ constants

/// `sizeof(crypto_generichash_blake2b_state)` == 384, and
/// `statebytes() == (384 + 63) & ~63 == 384`.
const SB: usize = 384;

const BYTES_MIN: usize = 16;
const BYTES: usize = 32;
const BYTES_MAX: usize = 64;
const KEYBYTES_MIN: usize = 16;
const KEYBYTES: usize = 32;
const KEYBYTES_MAX: usize = 64;
const SALTBYTES: usize = 16;
const PERSONALBYTES: usize = 16;

/// Offsets inside the internal `blake2b_state` (`#pragma pack(push, 1)`;
/// every field is naturally aligned so the packed layout equals `repr(C)`
/// field-by-field): `h[8]`@0, `t[2]`@64, `f[2]`@80, `buf[256]`@96,
/// `buflen`@352, `last_node`@360.  Total 361 bytes inside the 384-byte
/// public opaque buffer; bytes 361..384 are never written by either side, so
/// starting from an all-zero buffer makes the full 384-byte comparison exact.
const OFF_F: usize = 80;
const OFF_BUFLEN: usize = 352;
const OFF_LAST_NODE: usize = 360;
const INTERNAL_STATE_SIZE: usize = 361;

// ------------------------------------------------------------------- helpers

/// A 64-byte-aligned public `crypto_generichash_state`, zero-filled so that
/// the 23 trailing bytes neither implementation touches compare equal.
#[repr(C, align(64))]
#[derive(Clone, Copy)]
/// `crypto_generichash_blake2b_state` is declared `CRYPTO_ALIGN(64)` in the
/// public header, so the state buffer MUST be 64-byte aligned; a plain
/// `[u8; SB]` on the stack is not, and the resulting behaviour depends on the
/// thread's stack address (which made this file intermittently fail).
#[repr(C, align(64))]
struct St([u8; SB]);

impl St {
    fn zero() -> Self {
        St([0u8; SB])
    }
    fn p(&mut self) -> *mut c_void {
        self.0.as_mut_ptr() as *mut c_void
    }
    fn b(&self) -> &[u8] {
        &self.0
    }
}

fn pattern(kind: u8, len: usize) -> Vec<u8> {
    match kind {
        0 => vec![0u8; len],
        1 => vec![0xffu8; len],
        2 => (0..len).map(|i| (i & 0xff) as u8).collect(),
        _ => Rng::new(0xC0FFEE ^ (len as u64) ^ ((kind as u64) << 32)).bytes(len),
    }
}

unsafe fn cstr(p: *const c_char) -> String {
    std::ffi::CStr::from_ptr(p).to_string_lossy().into_owned()
}

/// The set of message lengths used throughout: every length 0..=300 plus the
/// block-straddling and multi-KiB values.  128 is `BLAKE2B_BLOCKBYTES`, 256 is
/// the full lazy buffer (`buf[2 * 128]`).
fn lengths() -> Vec<usize> {
    let mut v: Vec<usize> = (0..=300).collect();
    v.extend_from_slice(&[383, 384, 385, 511, 512, 513, 1024, 2000, 4096, 8192]);
    v
}

/// One-shot pair: call `f` on both libraries, assert the return code equals
/// `want`, assert C and Rust agree, and return C's output prefix.
#[allow(clippy::too_many_arguments)]
#[track_caller]
unsafe fn os(
    what: &str,
    cf: &Symbol<'static, Gh>,
    rf: &Symbol<'static, Gh>,
    outlen: usize,
    inp: *const u8,
    inlen: u64,
    kp: *const u8,
    kl: usize,
    want: c_int,
) -> Vec<u8> {
    let mut co = padded(outlen);
    let mut ro = padded(outlen);
    let rc = cf(co.as_mut_ptr(), outlen, inp, inlen, kp, kl);
    let rr = rf(ro.as_mut_ptr(), outlen, inp, inlen, kp, kl);
    assert_eq!(rc, want, "{what}: C returned {rc}, expected sentinel {want}");
    eqi(&format!("{what}: rc"), rc, rr);
    check_pad(&format!("{what}: C out"), &co, outlen);
    check_pad(&format!("{what}: Rust out"), &ro, outlen);
    eqb(&format!("{what}: digest"), &co[..outlen], &ro[..outlen]);
    co[..outlen].to_vec()
}

#[allow(clippy::too_many_arguments)]
#[track_caller]
unsafe fn os_sp(
    what: &str,
    cf: &Symbol<'static, GhSp>,
    rf: &Symbol<'static, GhSp>,
    outlen: usize,
    inp: *const u8,
    inlen: u64,
    kp: *const u8,
    kl: usize,
    salt: *const u8,
    pers: *const u8,
    want: c_int,
) -> Vec<u8> {
    let mut co = padded(outlen);
    let mut ro = padded(outlen);
    let rc = cf(co.as_mut_ptr(), outlen, inp, inlen, kp, kl, salt, pers);
    let rr = rf(ro.as_mut_ptr(), outlen, inp, inlen, kp, kl, salt, pers);
    assert_eq!(rc, want, "{what}: C returned {rc}, expected sentinel {want}");
    eqi(&format!("{what}: rc"), rc, rr);
    check_pad(&format!("{what}: C out"), &co, outlen);
    check_pad(&format!("{what}: Rust out"), &ro, outlen);
    eqb(&format!("{what}: digest"), &co[..outlen], &ro[..outlen]);
    co[..outlen].to_vec()
}

/// The `init`/`update`/`final` triple of one prefix (`crypto_generichash` or
/// `crypto_generichash_blake2b`).
struct Stream {
    init: (Symbol<'static, GhInit>, Symbol<'static, GhInit>),
    upd: (Symbol<'static, GhUpdate>, Symbol<'static, GhUpdate>),
    fin: (Symbol<'static, GhFinal>, Symbol<'static, GhFinal>),
}

impl Stream {
    fn new(prefix: &str) -> Self {
        Stream {
            init: both::<GhInit>(&format!("{prefix}_init")),
            upd: both::<GhUpdate>(&format!("{prefix}_update")),
            fin: both::<GhFinal>(&format!("{prefix}_final")),
        }
    }

    /// `init` + one `update` per chunk + `final`.  Compares the whole opaque
    /// state after `init`, after **every** `update` and after `final`.
    #[track_caller]
    fn run(
        &self,
        what: &str,
        outlen: usize,
        kp: *const u8,
        kl: usize,
        chunks: &[&[u8]],
        finlen: usize,
    ) -> Vec<u8> {
        let mut cs = St::zero();
        let mut rs = St::zero();
        unsafe {
            let rc = (self.init.0)(cs.p(), kp, kl, outlen);
            let rr = (self.init.1)(rs.p(), kp, kl, outlen);
            eqi(&format!("{what}: init rc"), rc, rr);
            assert_eq!(rc, 0, "{what}: init unexpectedly failed");
            eqb(&format!("{what}: state after init"), cs.b(), rs.b());

            for (i, ch) in chunks.iter().enumerate() {
                let rc = (self.upd.0)(cs.p(), ch.as_ptr(), ch.len() as u64);
                let rr = (self.upd.1)(rs.p(), ch.as_ptr(), ch.len() as u64);
                eqi(&format!("{what}: update[{i}] rc"), rc, rr);
                assert_eq!(rc, 0, "{what}: update[{i}] must always return 0");
                eqb(
                    &format!("{what}: state after update[{i}] (len {})", ch.len()),
                    cs.b(),
                    rs.b(),
                );
            }

            let mut co = padded(finlen);
            let mut ro = padded(finlen);
            let rc = (self.fin.0)(cs.p(), co.as_mut_ptr(), finlen);
            let rr = (self.fin.1)(rs.p(), ro.as_mut_ptr(), finlen);
            eqi(&format!("{what}: final rc"), rc, rr);
            assert_eq!(rc, 0, "{what}: final unexpectedly failed");
            check_pad(&format!("{what}: C final out"), &co, finlen);
            check_pad(&format!("{what}: Rust final out"), &ro, finlen);
            eqb(&format!("{what}: digest"), &co[..finlen], &ro[..finlen]);
            eqb(&format!("{what}: state after final"), cs.b(), rs.b());
            co[..finlen].to_vec()
        }
    }
}

// ===========================================================================
// accessors — CONFIGS 3.115, ERRORS 3.98/3.100
// ===========================================================================

#[test]
fn accessors() {
    let table: &[(&str, usize)] = &[
        ("crypto_generichash_bytes_min", BYTES_MIN),
        ("crypto_generichash_bytes_max", BYTES_MAX),
        ("crypto_generichash_bytes", BYTES),
        ("crypto_generichash_keybytes_min", KEYBYTES_MIN),
        ("crypto_generichash_keybytes_max", KEYBYTES_MAX),
        ("crypto_generichash_keybytes", KEYBYTES),
        ("crypto_generichash_statebytes", SB),
        ("crypto_generichash_blake2b_bytes_min", BYTES_MIN),
        ("crypto_generichash_blake2b_bytes_max", BYTES_MAX),
        ("crypto_generichash_blake2b_bytes", BYTES),
        ("crypto_generichash_blake2b_keybytes_min", KEYBYTES_MIN),
        ("crypto_generichash_blake2b_keybytes_max", KEYBYTES_MAX),
        ("crypto_generichash_blake2b_keybytes", KEYBYTES),
        ("crypto_generichash_blake2b_saltbytes", SALTBYTES),
        ("crypto_generichash_blake2b_personalbytes", PERSONALBYTES),
        ("crypto_generichash_blake2b_statebytes", SB),
    ];
    for (name, want) in table {
        assert!(has(name), "{name} must be exported by both");
        let (c, r) = both::<SizeFn>(name);
        unsafe {
            let (a, b) = (c(), r());
            assert_eq!(a, *want, "C {name}() == {a}, expected {want}");
            assert_eq!(a, b, "{name}(): C {a} vs Rust {b}");
        }
    }

    let (c, r) = both::<StrFn>("crypto_generichash_primitive");
    unsafe {
        let a = cstr(c());
        let b = cstr(r());
        assert_eq!(a, "blake2b", "C crypto_generichash_primitive() == {a:?}");
        assert_eq!(a, b);
    }

    // `blake2b_pick_best_implementation` selects `blake2b_compress_ref` in this
    // build (no HAVE_* macros) — CONFIGS 3.116 / ERRORS 3.93.
    for name in [
        "_crypto_generichash_blake2b_pick_best_implementation",
        "_sodium_blake2b_pick_best_implementation",
    ] {
        assert!(has(name), "{name} must be exported by both");
        let (c, r) = both::<IntFn>(name);
        unsafe {
            assert_eq!(c(), 0, "C {name}() must return 0");
            eqi(name, c(), r());
        }
    }
}

#[test]
fn keygen() {
    // CONFIGS 3.114, ERRORS 3.99: `randombytes_buf(k, 32)`, void return.
    for name in ["crypto_generichash_keygen", "crypto_generichash_blake2b_keygen"] {
        let (c, r) = both::<Keygen>(name);
        for _ in 0..4 {
            let mut ck = padded(KEYBYTES);
            let mut rk = padded(KEYBYTES);
            rng_reset();
            unsafe {
                c(ck.as_mut_ptr());
                r(rk.as_mut_ptr());
            }
            check_pad(&format!("{name}: C"), &ck, KEYBYTES);
            check_pad(&format!("{name}: Rust"), &rk, KEYBYTES);
            eqb(name, &ck, &rk);
            assert!(ck[..KEYBYTES].iter().any(|&x| x != 0), "{name}: all-zero key");
        }
    }
    // Both keygens draw exactly 32 bytes, so they agree with each other too.
    let (ca, ra) = both::<Keygen>("crypto_generichash_keygen");
    let (cb, rb) = both::<Keygen>("crypto_generichash_blake2b_keygen");
    let mut a = [0u8; KEYBYTES];
    let mut b = [0u8; KEYBYTES];
    let mut c2 = [0u8; KEYBYTES];
    let mut d = [0u8; KEYBYTES];
    rng_reset();
    unsafe {
        ca(a.as_mut_ptr());
        ra(b.as_mut_ptr());
    }
    rng_reset();
    unsafe {
        cb(c2.as_mut_ptr());
        rb(d.as_mut_ptr());
    }
    eqb("keygen generic vs blake2b (C)", &a, &c2);
    eqb("keygen generic vs blake2b (Rust)", &b, &d);
}

// ===========================================================================
// outlen × keylen matrix — CONFIGS 3.84/3.86/3.108, ERRORS 3.34–3.39, 3.50–3.53
// ===========================================================================

/// Expected sentinel of every `crypto_generichash*` entry point that validates
/// `outlen`/`keylen`.  Note that `BYTES_MIN`/`KEYBYTES_MIN` (16) are **NOT**
/// enforced anywhere in the C: only `outlen == 0`, `outlen > 64` and
/// `keylen > 64` are rejected, all with `-1`.
fn want_rc(outlen: usize, keylen: usize) -> c_int {
    if outlen == 0 || outlen > BYTES_MAX || keylen > KEYBYTES_MAX {
        -1
    } else {
        0
    }
}

#[test]
fn outlen_keylen_matrix_one_shot() {
    let (gc, gr) = both::<Gh>("crypto_generichash_blake2b");
    let (wc, wr) = both::<Gh>("crypto_generichash");
    let (sc, sr) = both::<GhSp>("crypto_generichash_blake2b_salt_personal");

    let mut rng = Rng::new(0x3_8410);
    let key = rng.bytes(66);
    let salt = rng.bytes(SALTBYTES);
    let pers = rng.bytes(PERSONALBYTES);
    let msg = pattern(2, 200);

    for outlen in 0..=66usize {
        for keylen in 0..=66usize {
            let want = want_rc(outlen, keylen);
            unsafe {
                let a = os(
                    &format!("blake2b(outlen={outlen},keylen={keylen})"),
                    &gc,
                    &gr,
                    outlen,
                    msg.as_ptr(),
                    msg.len() as u64,
                    key.as_ptr(),
                    keylen,
                    want,
                );
                // generic wrapper must be byte-identical (CONFIGS 3.108)
                let b = os(
                    &format!("generic(outlen={outlen},keylen={keylen})"),
                    &wc,
                    &wr,
                    outlen,
                    msg.as_ptr(),
                    msg.len() as u64,
                    key.as_ptr(),
                    keylen,
                    want,
                );
                eqb(
                    &format!("crypto_generichash vs _blake2b (outlen={outlen},keylen={keylen})"),
                    &a,
                    &b,
                );
                // salt_personal has the identical 4-way check (ERRORS 3.47)
                os_sp(
                    &format!("salt_personal(outlen={outlen},keylen={keylen})"),
                    &sc,
                    &sr,
                    outlen,
                    msg.as_ptr(),
                    msg.len() as u64,
                    key.as_ptr(),
                    keylen,
                    salt.as_ptr(),
                    pers.as_ptr(),
                    want,
                );
            }
        }
    }

    // 1..15 (below BYTES_MIN / KEYBYTES_MIN) really are accepted.
    for n in 1..BYTES_MIN {
        assert_eq!(want_rc(n, n), 0);
    }
}

#[test]
fn outlen_keylen_matrix_init() {
    let ic = both::<GhInit>("crypto_generichash_blake2b_init");
    let jc = both::<GhInit>("crypto_generichash_init");
    let kc = both::<GhInitSp>("crypto_generichash_blake2b_init_salt_personal");

    let mut rng = Rng::new(0x3_8411);
    let key = rng.bytes(66);
    let salt = rng.bytes(SALTBYTES);
    let pers = rng.bytes(PERSONALBYTES);

    for outlen in 0..=66usize {
        for keylen in 0..=66usize {
            let want = want_rc(outlen, keylen);
            let mut cs = St::zero();
            let mut rs = St::zero();
            let mut cs2 = St::zero();
            let mut rs2 = St::zero();
            let mut cs3 = St::zero();
            let mut rs3 = St::zero();
            unsafe {
                let a = (ic.0)(cs.p(), key.as_ptr(), keylen, outlen);
                let b = (ic.1)(rs.p(), key.as_ptr(), keylen, outlen);
                assert_eq!(a, want, "blake2b_init(outlen={outlen},keylen={keylen}) C rc {a}");
                eqi(&format!("blake2b_init({outlen},{keylen})"), a, b);
                eqb(&format!("blake2b_init state({outlen},{keylen})"), cs.b(), rs.b());

                let a2 = (jc.0)(cs2.p(), key.as_ptr(), keylen, outlen);
                let b2 = (jc.1)(rs2.p(), key.as_ptr(), keylen, outlen);
                assert_eq!(a2, want, "generichash_init(outlen={outlen},keylen={keylen})");
                eqi(&format!("generichash_init({outlen},{keylen})"), a2, b2);
                eqb(&format!("generichash_init state({outlen},{keylen})"), cs2.b(), rs2.b());
                // the generic wrapper keeps the (state, key, keylen, outlen)
                // argument order, so the resulting state must be identical
                eqb(
                    &format!("init wrapper equivalence({outlen},{keylen})"),
                    cs.b(),
                    cs2.b(),
                );

                let a3 = (kc.0)(cs3.p(), key.as_ptr(), keylen, outlen, salt.as_ptr(), pers.as_ptr());
                let b3 = (kc.1)(rs3.p(), key.as_ptr(), keylen, outlen, salt.as_ptr(), pers.as_ptr());
                assert_eq!(a3, want, "init_salt_personal(outlen={outlen},keylen={keylen})");
                eqi(&format!("init_salt_personal({outlen},{keylen})"), a3, b3);
                eqb(
                    &format!("init_salt_personal state({outlen},{keylen})"),
                    cs3.b(),
                    rs3.b(),
                );
            }
        }
    }
}

#[test]
fn huge_outlen_keylen_are_rejected() {
    // ERRORS 3.35/3.36/3.51/3.52 with values far beyond UINT8_MAX; the
    // `assert(outlen <= UINT8_MAX)` in the C is unreachable because the range
    // check fires first (ERRORS 3.40).
    let (gc, gr) = both::<Gh>("crypto_generichash_blake2b");
    let (wc, wr) = both::<Gh>("crypto_generichash");
    let ic = both::<GhInit>("crypto_generichash_blake2b_init");
    let msg = pattern(2, 64);
    let key = pattern(3, 64);
    let big = [65usize, 66, 128, 255, 256, 257, 4096, usize::MAX / 2, usize::MAX];
    unsafe {
        for &n in &big {
            // outlen too large — no write, so a 0-length padded buffer suffices
            os("outlen huge", &gc, &gr, 0, msg.as_ptr(), msg.len() as u64, key.as_ptr(), 0, -1);
            let mut co = padded(0);
            let mut ro = padded(0);
            let a = gc(co.as_mut_ptr(), n, msg.as_ptr(), msg.len() as u64, key.as_ptr(), 0);
            let b = gr(ro.as_mut_ptr(), n, msg.as_ptr(), msg.len() as u64, key.as_ptr(), 0);
            assert_eq!(a, -1, "blake2b(outlen={n}) must be -1");
            eqi(&format!("blake2b(outlen={n})"), a, b);
            check_pad("outlen huge C", &co, 0);
            check_pad("outlen huge Rust", &ro, 0);

            let a = wc(co.as_mut_ptr(), n, msg.as_ptr(), msg.len() as u64, key.as_ptr(), 0);
            let b = wr(ro.as_mut_ptr(), n, msg.as_ptr(), msg.len() as u64, key.as_ptr(), 0);
            assert_eq!(a, -1, "generichash(outlen={n}) must be -1");
            eqi(&format!("generichash(outlen={n})"), a, b);

            // keylen too large
            let mut co = padded(BYTES);
            let mut ro = padded(BYTES);
            let a = gc(co.as_mut_ptr(), BYTES, msg.as_ptr(), msg.len() as u64, key.as_ptr(), n);
            let b = gr(ro.as_mut_ptr(), BYTES, msg.as_ptr(), msg.len() as u64, key.as_ptr(), n);
            assert_eq!(a, -1, "blake2b(keylen={n}) must be -1");
            eqi(&format!("blake2b(keylen={n})"), a, b);
            check_pad("keylen huge C", &co, BYTES);
            check_pad("keylen huge Rust", &ro, BYTES);

            let mut cs = St::zero();
            let mut rs = St::zero();
            let a = (ic.0)(cs.p(), key.as_ptr(), 0, n);
            let b = (ic.1)(rs.p(), key.as_ptr(), 0, n);
            assert_eq!(a, -1, "init(outlen={n}) must be -1");
            eqi(&format!("init(outlen={n})"), a, b);
            eqb("init(outlen huge) leaves state untouched", cs.b(), rs.b());
            assert!(cs.b().iter().all(|&x| x == 0), "init returned -1 but wrote the state");

            let a = (ic.0)(cs.p(), key.as_ptr(), n, BYTES);
            let b = (ic.1)(rs.p(), key.as_ptr(), n, BYTES);
            assert_eq!(a, -1, "init(keylen={n}) must be -1");
            eqi(&format!("init(keylen={n})"), a, b);
            eqb("init(keylen huge) leaves state untouched", cs.b(), rs.b());
        }
    }
}

// ===========================================================================
// NULL key / in / out — ERRORS 3.41–3.46, 3.49, 3.55, 3.58, CONFIGS 3.85/3.87/3.99/3.100/3.103
// ===========================================================================

#[test]
fn null_key_with_zero_keylen_is_unkeyed() {
    // ERRORS 3.41 (key == NULL && keylen == 0) and 3.43 (key != NULL &&
    // keylen == 0): both are legal and produce the *same*, unkeyed digest.
    let (gc, gr) = both::<Gh>("crypto_generichash_blake2b");
    let (wc, wr) = both::<Gh>("crypto_generichash");
    let key = pattern(3, KEYBYTES);
    for &inlen in &[0usize, 1, 127, 128, 129, 255, 256, 257] {
        let msg = pattern(2, inlen);
        for &outlen in &[1usize, 15, 16, 32, 64] {
            unsafe {
                let a = os(
                    "key=NULL keylen=0",
                    &gc,
                    &gr,
                    outlen,
                    msg.as_ptr(),
                    inlen as u64,
                    std::ptr::null(),
                    0,
                    0,
                );
                let b = os(
                    "key!=NULL keylen=0",
                    &gc,
                    &gr,
                    outlen,
                    msg.as_ptr(),
                    inlen as u64,
                    key.as_ptr(),
                    0,
                    0,
                );
                eqb("key!=NULL,keylen=0 must equal key=NULL,keylen=0", &a, &b);
                let c2 = os(
                    "generic key=NULL keylen=0",
                    &wc,
                    &wr,
                    outlen,
                    msg.as_ptr(),
                    inlen as u64,
                    std::ptr::null(),
                    0,
                    0,
                );
                eqb("generic wrapper unkeyed", &a, &c2);
            }
        }
    }
}

#[test]
fn null_in_and_null_out() {
    // ERRORS 3.45: in == NULL && inlen == 0 is legal and equals a non-NULL
    // zero-length input (CONFIGS 3.85).
    let (gc, gr) = both::<Gh>("crypto_generichash_blake2b");
    let (sc, sr) = both::<GhSp>("crypto_generichash_blake2b_salt_personal");
    let salt = pattern(2, SALTBYTES);
    let pers = pattern(1, PERSONALBYTES);
    let empty: [u8; 1] = [0];
    for &outlen in &[1usize, 16, 32, 64] {
        unsafe {
            let a = os(
                "in=NULL inlen=0",
                &gc,
                &gr,
                outlen,
                std::ptr::null(),
                0,
                std::ptr::null(),
                0,
                0,
            );
            let b = os(
                "in!=NULL inlen=0",
                &gc,
                &gr,
                outlen,
                empty.as_ptr(),
                0,
                std::ptr::null(),
                0,
                0,
            );
            eqb("in=NULL,inlen=0 == in!=NULL,inlen=0", &a, &b);
            os_sp(
                "sp in=NULL inlen=0",
                &sc,
                &sr,
                outlen,
                std::ptr::null(),
                0,
                std::ptr::null(),
                0,
                salt.as_ptr(),
                pers.as_ptr(),
                0,
            );
        }
    }
}

#[test]
fn one_shot_null_key_with_positive_keylen_aborts() {
    // ERRORS 3.42 / 3.49: the one-shot reaches `blake2b()`'s
    // `NULL == key && keylen > 0` check, which calls sodium_misuse() -> abort.
    // `crypto_generichash_blake2b_init` instead treats the same arguments as an
    // *unkeyed* init (ERRORS 3.55) — the two entry points genuinely diverge.
    let (gc, gr) = both::<Gh>("crypto_generichash_blake2b");
    let (wc, wr) = both::<Gh>("crypto_generichash");
    let (sc, sr) = both::<GhSp>("crypto_generichash_blake2b_salt_personal");
    let msg = pattern(2, 64);
    let salt = pattern(2, SALTBYTES);
    let pers = pattern(1, PERSONALBYTES);

    for &keylen in &[1usize, 15, 16, 32, 63, 64] {
        let mp = msg.as_ptr() as usize;
        let ml = msg.len() as u64;
        eq_abort(
            &format!("blake2b(key=NULL, keylen={keylen})"),
            || unsafe {
                let mut o = [0u8; BYTES];
                gc(o.as_mut_ptr(), BYTES, mp as *const u8, ml, std::ptr::null(), keylen);
            },
            || unsafe {
                let mut o = [0u8; BYTES];
                gr(o.as_mut_ptr(), BYTES, mp as *const u8, ml, std::ptr::null(), keylen);
            },
        );
        eq_abort(
            &format!("generichash(key=NULL, keylen={keylen})"),
            || unsafe {
                let mut o = [0u8; BYTES];
                wc(o.as_mut_ptr(), BYTES, mp as *const u8, ml, std::ptr::null(), keylen);
            },
            || unsafe {
                let mut o = [0u8; BYTES];
                wr(o.as_mut_ptr(), BYTES, mp as *const u8, ml, std::ptr::null(), keylen);
            },
        );
        let sp = salt.as_ptr() as usize;
        let pp = pers.as_ptr() as usize;
        eq_abort(
            &format!("salt_personal(key=NULL, keylen={keylen})"),
            || unsafe {
                let mut o = [0u8; BYTES];
                sc(
                    o.as_mut_ptr(),
                    BYTES,
                    mp as *const u8,
                    ml,
                    std::ptr::null(),
                    keylen,
                    sp as *const u8,
                    pp as *const u8,
                );
            },
            || unsafe {
                let mut o = [0u8; BYTES];
                sr(
                    o.as_mut_ptr(),
                    BYTES,
                    mp as *const u8,
                    ml,
                    std::ptr::null(),
                    keylen,
                    sp as *const u8,
                    pp as *const u8,
                );
            },
        );
    }

    // keylen > 64 with key == NULL is caught by the wrapper's range check
    // *before* `blake2b()` is entered, so it is a plain -1, not an abort.
    unsafe {
        for &keylen in &[65usize, 256, usize::MAX] {
            let mut co = padded(BYTES);
            let mut ro = padded(BYTES);
            let a = gc(co.as_mut_ptr(), BYTES, msg.as_ptr(), msg.len() as u64, std::ptr::null(), keylen);
            let b = gr(ro.as_mut_ptr(), BYTES, msg.as_ptr(), msg.len() as u64, std::ptr::null(), keylen);
            assert_eq!(a, -1, "key=NULL keylen={keylen} must be -1, not an abort");
            eqi("key=NULL keylen>64", a, b);
        }
    }
}

#[test]
fn one_shot_null_in_with_positive_inlen_aborts() {
    // ERRORS 3.44
    let (gc, gr) = both::<Gh>("crypto_generichash_blake2b");
    for &inlen in &[1u64, 127, 128, 129, 1000] {
        eq_abort(
            &format!("blake2b(in=NULL, inlen={inlen})"),
            || unsafe {
                let mut o = [0u8; BYTES];
                gc(o.as_mut_ptr(), BYTES, std::ptr::null(), inlen, std::ptr::null(), 0);
            },
            || unsafe {
                let mut o = [0u8; BYTES];
                gr(o.as_mut_ptr(), BYTES, std::ptr::null(), inlen, std::ptr::null(), 0);
            },
        );
    }
}

#[test]
fn one_shot_null_out_aborts() {
    // ERRORS 3.46: `out == NULL` is checked at runtime inside `blake2b()`.
    let (gc, gr) = both::<Gh>("crypto_generichash_blake2b");
    let (sc, sr) = both::<GhSp>("crypto_generichash_blake2b_salt_personal");
    let msg = pattern(2, 100);
    let mp = msg.as_ptr() as usize;
    let ml = msg.len() as u64;
    eq_abort(
        "blake2b(out=NULL)",
        || unsafe {
            gc(std::ptr::null_mut(), BYTES, mp as *const u8, ml, std::ptr::null(), 0);
        },
        || unsafe {
            gr(std::ptr::null_mut(), BYTES, mp as *const u8, ml, std::ptr::null(), 0);
        },
    );
    eq_abort(
        "salt_personal(out=NULL)",
        || unsafe {
            sc(
                std::ptr::null_mut(),
                BYTES,
                mp as *const u8,
                ml,
                std::ptr::null(),
                0,
                std::ptr::null(),
                std::ptr::null(),
            );
        },
        || unsafe {
            sr(
                std::ptr::null_mut(),
                BYTES,
                mp as *const u8,
                ml,
                std::ptr::null(),
                0,
                std::ptr::null(),
                std::ptr::null(),
            );
        },
    );
}

#[test]
fn init_with_null_key_and_positive_keylen_is_silently_unkeyed() {
    // CONFIGS 3.99/3.103, ERRORS 3.55/3.58 — the deliberate asymmetry against
    // the aborting one-shot verified in
    // `one_shot_null_key_with_positive_keylen_aborts`.
    let ic = both::<GhInit>("crypto_generichash_blake2b_init");
    let jc = both::<GhInit>("crypto_generichash_init");
    let kc = both::<GhInitSp>("crypto_generichash_blake2b_init_salt_personal");
    let salt = pattern(2, SALTBYTES);
    let pers = pattern(1, PERSONALBYTES);
    let key = pattern(3, KEYBYTES_MAX);

    for &outlen in &[1usize, 15, 16, 32, 64] {
        // reference: the genuinely unkeyed init
        let mut ref_c = St::zero();
        let mut ref_r = St::zero();
        unsafe {
            assert_eq!((ic.0)(ref_c.p(), std::ptr::null(), 0, outlen), 0);
            assert_eq!((ic.1)(ref_r.p(), std::ptr::null(), 0, outlen), 0);
        }
        eqb("unkeyed init reference", ref_c.b(), ref_r.b());

        for &keylen in &[0usize, 1, 15, 16, 32, 63, 64] {
            for (label, kp) in [
                ("key=NULL", std::ptr::null::<u8>()),
                ("key!=NULL", key.as_ptr()),
            ] {
                // `key == NULL || keylen == 0` -> unkeyed
                let unkeyed = kp.is_null() || keylen == 0;
                let mut cs = St::zero();
                let mut rs = St::zero();
                unsafe {
                    let a = (ic.0)(cs.p(), kp, keylen, outlen);
                    let b = (ic.1)(rs.p(), kp, keylen, outlen);
                    assert_eq!(a, 0, "init({label},keylen={keylen},outlen={outlen})");
                    eqi("init rc", a, b);
                }
                eqb(
                    &format!("init state ({label},keylen={keylen},outlen={outlen})"),
                    cs.b(),
                    rs.b(),
                );
                if unkeyed {
                    eqb(
                        &format!("init({label},keylen={keylen}) must be unkeyed"),
                        ref_c.b(),
                        cs.b(),
                    );
                    // an unkeyed init leaves buflen == 0
                    assert_eq!(
                        cs.b()[OFF_BUFLEN], 0,
                        "unkeyed init must leave buflen == 0"
                    );
                } else {
                    assert_ne!(
                        ref_c.b(),
                        cs.b(),
                        "keyed init must differ from unkeyed init"
                    );
                    // ERRORS 3.81: keyed init pre-absorbs a 128-byte block
                    assert_eq!(
                        cs.b()[OFF_BUFLEN], 128,
                        "keyed init must leave buflen == 128"
                    );
                }
                // generic wrapper agrees
                let mut ws = St::zero();
                let mut wsr = St::zero();
                unsafe {
                    assert_eq!((jc.0)(ws.p(), kp, keylen, outlen), 0);
                    assert_eq!((jc.1)(wsr.p(), kp, keylen, outlen), 0);
                }
                eqb("generic init state", ws.b(), wsr.b());
                eqb("generic init == blake2b init", cs.b(), ws.b());

                // init_salt_personal takes the same NULL-key route
                let mut ss = St::zero();
                let mut ssr = St::zero();
                unsafe {
                    let a = (kc.0)(ss.p(), kp, keylen, outlen, salt.as_ptr(), pers.as_ptr());
                    let b = (kc.1)(ssr.p(), kp, keylen, outlen, salt.as_ptr(), pers.as_ptr());
                    assert_eq!(a, 0);
                    eqi("init_salt_personal rc", a, b);
                }
                eqb("init_salt_personal state", ss.b(), ssr.b());
                assert_eq!(
                    ss.b()[OFF_BUFLEN],
                    if unkeyed { 0 } else { 128 },
                    "init_salt_personal buflen"
                );
            }
        }
    }
}

// ===========================================================================
// salt / personal — CONFIGS 3.88–3.93, 3.101, 3.102, ERRORS 3.48/3.60
// ===========================================================================

#[test]
fn salt_personal_null_combinations() {
    let (gc, gr) = both::<Gh>("crypto_generichash_blake2b");
    let (sc, sr) = both::<GhSp>("crypto_generichash_blake2b_salt_personal");
    let mut rng = Rng::new(0x3_8800);
    let salt = rng.bytes(SALTBYTES);
    let pers = rng.bytes(PERSONALBYTES);
    let zero16 = [0u8; SALTBYTES];
    let key = rng.bytes(KEYBYTES_MAX);

    for &inlen in &[0usize, 1, 63, 64, 65, 127, 128, 129, 255, 256, 257] {
        let msg = pattern(2, inlen);
        for &outlen in &[1usize, 16, 32, 64] {
            for &keylen in &[0usize, 1, 16, 32, 64] {
                let kp = if keylen == 0 { std::ptr::null() } else { key.as_ptr() };
                let mut seen: Vec<(String, Vec<u8>)> = Vec::new();
                for (label, sp, pp) in [
                    ("both NULL", std::ptr::null::<u8>(), std::ptr::null::<u8>()),
                    ("salt only", salt.as_ptr(), std::ptr::null()),
                    ("personal only", std::ptr::null(), pers.as_ptr()),
                    ("both set", salt.as_ptr(), pers.as_ptr()),
                    ("both zero", zero16.as_ptr(), zero16.as_ptr()),
                ] {
                    let d = unsafe {
                        os_sp(
                            &format!("sp[{label}](out={outlen},key={keylen},in={inlen})"),
                            &sc,
                            &sr,
                            outlen,
                            msg.as_ptr(),
                            inlen as u64,
                            kp,
                            keylen,
                            sp,
                            pp,
                            0,
                        )
                    };
                    seen.push((label.to_string(), d));
                }
                // CONFIGS 3.89/3.92: NULL == all-zero == plain blake2b
                let plain = unsafe {
                    os(
                        "plain",
                        &gc,
                        &gr,
                        outlen,
                        msg.as_ptr(),
                        inlen as u64,
                        kp,
                        keylen,
                        0,
                    )
                };
                eqb("salt_personal(NULL,NULL) == blake2b", &seen[0].1, &plain);
                eqb("salt_personal(0,0) == blake2b", &seen[4].1, &plain);
                // CONFIGS 3.93: each distinct arm gives a distinct digest
                // (only meaningful when the digest is wide enough to be
                // collision-free by inspection)
                if outlen >= 16 {
                    for i in 0..4 {
                        for j in (i + 1)..4 {
                            if i == 0 {
                                // "both NULL" vs the others
                            }
                            assert_ne!(
                                seen[i].1, seen[j].1,
                                "{} and {} must give different digests",
                                seen[i].0, seen[j].0
                            );
                        }
                    }
                }
            }
        }
    }
}

#[test]
fn salt_personal_field_positions() {
    // CONFIGS 3.93: salt lands at param offset 32, personal at 48.  Flipping a
    // single bit of either must change the digest, and swapping the two must
    // change it as well.
    let (sc, sr) = both::<GhSp>("crypto_generichash_blake2b_salt_personal");
    let mut rng = Rng::new(0x3_8930);
    let salt = rng.bytes(SALTBYTES);
    let pers = rng.bytes(PERSONALBYTES);
    let msg = pattern(2, 137);
    let base = unsafe {
        os_sp(
            "base",
            &sc,
            &sr,
            BYTES,
            msg.as_ptr(),
            msg.len() as u64,
            std::ptr::null(),
            0,
            salt.as_ptr(),
            pers.as_ptr(),
            0,
        )
    };
    let swapped = unsafe {
        os_sp(
            "swapped",
            &sc,
            &sr,
            BYTES,
            msg.as_ptr(),
            msg.len() as u64,
            std::ptr::null(),
            0,
            pers.as_ptr(),
            salt.as_ptr(),
            0,
        )
    };
    assert_ne!(base, swapped, "salt and personal must occupy distinct fields");
    for i in 0..SALTBYTES {
        for which in 0..2 {
            let mut s2 = salt.clone();
            let mut p2 = pers.clone();
            if which == 0 {
                s2[i] ^= 1;
            } else {
                p2[i] ^= 1;
            }
            let d = unsafe {
                os_sp(
                    "bitflip",
                    &sc,
                    &sr,
                    BYTES,
                    msg.as_ptr(),
                    msg.len() as u64,
                    std::ptr::null(),
                    0,
                    s2.as_ptr(),
                    p2.as_ptr(),
                    0,
                )
            };
            assert_ne!(base, d, "flipping bit 0 of {which}[{i}] changed nothing");
        }
    }
}

#[test]
fn init_salt_personal_streaming() {
    // CONFIGS 3.101/3.102 — streaming with every salt/personal NULL combination
    // and both unkeyed and keyed, with the full state compared after init and
    // after each update.
    let init = both::<GhInitSp>("crypto_generichash_blake2b_init_salt_personal");
    let upd = both::<GhUpdate>("crypto_generichash_blake2b_update");
    let fin = both::<GhFinal>("crypto_generichash_blake2b_final");
    let (sc, sr) = both::<GhSp>("crypto_generichash_blake2b_salt_personal");
    let mut rng = Rng::new(0x3_10100);
    let salt = rng.bytes(SALTBYTES);
    let pers = rng.bytes(PERSONALBYTES);
    let key = rng.bytes(KEYBYTES_MAX);

    for &inlen in &[0usize, 1, 128, 129, 256, 257, 300] {
        let msg = pattern(2, inlen);
        for &outlen in &[1usize, 16, 32, 64] {
            for &keylen in &[0usize, 1, 16, 32, 64] {
                let kp = if keylen == 0 { std::ptr::null() } else { key.as_ptr() };
                for (label, sp, pp) in [
                    ("both NULL", std::ptr::null::<u8>(), std::ptr::null::<u8>()),
                    ("salt only", salt.as_ptr(), std::ptr::null()),
                    ("personal only", std::ptr::null(), pers.as_ptr()),
                    ("both set", salt.as_ptr(), pers.as_ptr()),
                ] {
                    let what =
                        format!("init_sp[{label}](out={outlen},key={keylen},in={inlen})");
                    let mut cs = St::zero();
                    let mut rs = St::zero();
                    let dc;
                    unsafe {
                        let a = (init.0)(cs.p(), kp, keylen, outlen, sp, pp);
                        let b = (init.1)(rs.p(), kp, keylen, outlen, sp, pp);
                        assert_eq!(a, 0, "{what}: init");
                        eqi(&format!("{what}: init rc"), a, b);
                        eqb(&format!("{what}: state after init"), cs.b(), rs.b());

                        // two updates, one straddling the 128-byte block
                        let split = inlen.min(129);
                        for (i, part) in [&msg[..split], &msg[split..]].iter().enumerate() {
                            let a = (upd.0)(cs.p(), part.as_ptr(), part.len() as u64);
                            let b = (upd.1)(rs.p(), part.as_ptr(), part.len() as u64);
                            eqi(&format!("{what}: update[{i}] rc"), a, b);
                            eqb(&format!("{what}: state after update[{i}]"), cs.b(), rs.b());
                        }

                        let mut co = padded(outlen);
                        let mut ro = padded(outlen);
                        let a = (fin.0)(cs.p(), co.as_mut_ptr(), outlen);
                        let b = (fin.1)(rs.p(), ro.as_mut_ptr(), outlen);
                        assert_eq!(a, 0, "{what}: final");
                        eqi(&format!("{what}: final rc"), a, b);
                        check_pad(&format!("{what}: C out"), &co, outlen);
                        check_pad(&format!("{what}: Rust out"), &ro, outlen);
                        eqb(&format!("{what}: digest"), &co[..outlen], &ro[..outlen]);
                        eqb(&format!("{what}: state after final"), cs.b(), rs.b());
                        dc = co[..outlen].to_vec();
                    }
                    // CONFIGS 3.104: streaming == one-shot
                    let one = unsafe {
                        os_sp(
                            &format!("{what} one-shot"),
                            &sc,
                            &sr,
                            outlen,
                            msg.as_ptr(),
                            inlen as u64,
                            kp,
                            keylen,
                            sp,
                            pp,
                            0,
                        )
                    };
                    eqb(&format!("{what}: streaming == one-shot"), &one, &dc);
                }
            }
        }
    }
}

// ===========================================================================
// streaming vs one-shot, all message lengths — CONFIGS 3.94–3.98, 3.104
// ===========================================================================

#[test]
fn streaming_single_update_all_lengths() {
    let sb = Stream::new("crypto_generichash_blake2b");
    let sg = Stream::new("crypto_generichash");
    let (gc, gr) = both::<Gh>("crypto_generichash_blake2b");
    let key = pattern(3, KEYBYTES);

    for len in lengths() {
        for kind in [0u8, 1, 2, 3] {
            let msg = pattern(kind, len);
            for &outlen in &[1usize, 16, 32, 64] {
                for &keylen in &[0usize, 32] {
                    let kp = if keylen == 0 { std::ptr::null() } else { key.as_ptr() };
                    let what = format!("stream(len={len},kind={kind},out={outlen},key={keylen})");
                    let d = sb.run(&what, outlen, kp, keylen, &[&msg[..]], outlen);
                    let one = unsafe {
                        os(
                            &format!("{what} one-shot"),
                            &gc,
                            &gr,
                            outlen,
                            msg.as_ptr(),
                            len as u64,
                            kp,
                            keylen,
                            0,
                        )
                    };
                    eqb(&format!("{what}: streaming == one-shot"), &one, &d);
                    // generic wrappers must be byte-identical (CONFIGS 3.109)
                    let dg = sg.run(&format!("{what} generic"), outlen, kp, keylen, &[&msg[..]], outlen);
                    eqb(&format!("{what}: generic == blake2b"), &d, &dg);
                }
            }
        }
    }
}

#[test]
fn streaming_one_byte_updates() {
    // CONFIGS 3.95: walks `buflen` over 0..256 and exercises the lazy
    // `inlen <= fill` arm at every offset, with the full state compared after
    // every single-byte update.
    let sb = Stream::new("crypto_generichash_blake2b");
    let (gc, gr) = both::<Gh>("crypto_generichash_blake2b");
    // KEYBYTES_MAX, not KEYBYTES: this test drives keylen = 64, and a 32-byte
    // key buffer would make the library read 32 bytes past its end.
    let key = pattern(3, KEYBYTES_MAX);
    for &total in &[256usize, 257, 1024] {
        let msg = pattern(2, total);
        let chunks: Vec<&[u8]> = msg.chunks(1).collect();
        for &keylen in &[0usize, 64] {
            let kp = if keylen == 0 { std::ptr::null() } else { key.as_ptr() };
            let what = format!("1-byte updates (total={total},key={keylen})");
            let d = sb.run(&what, BYTES, kp, keylen, &chunks, BYTES);
            let one = unsafe {
                os(
                    &what,
                    &gc,
                    &gr,
                    BYTES,
                    msg.as_ptr(),
                    total as u64,
                    kp,
                    keylen,
                    0,
                )
            };
            eqb(&format!("{what}: == one-shot"), &one, &d);
        }
    }
}

#[test]
fn streaming_two_update_splits() {
    // CONFIGS 3.96: `a` straddles the 128-byte block and the 256-byte lazy
    // buffer, `a + b` lands on and just past both boundaries.
    let sb = Stream::new("crypto_generichash_blake2b");
    let (gc, gr) = both::<Gh>("crypto_generichash_blake2b");
    let key = pattern(3, KEYBYTES);
    let totals = [0usize, 1, 127, 128, 129, 255, 256, 257, 383, 384, 385, 512];
    for &total in &totals {
        let msg = pattern(2, total);
        for a in 0..=total {
            if !(a <= 2
                || a >= total.saturating_sub(2)
                || [1usize, 127, 128, 129, 255, 256, 257, 383, 384].contains(&a))
            {
                continue;
            }
            let chunks: [&[u8]; 2] = [&msg[..a], &msg[a..]];
            for &keylen in &[0usize, 32] {
                let kp = if keylen == 0 { std::ptr::null() } else { key.as_ptr() };
                let what = format!("split({a},{})/{total} key={keylen}", total - a);
                let d = sb.run(&what, BYTES, kp, keylen, &chunks, BYTES);
                let one = unsafe {
                    os(&what, &gc, &gr, BYTES, msg.as_ptr(), total as u64, kp, keylen, 0)
                };
                eqb(&format!("{what}: == one-shot"), &one, &d);
            }
        }
    }
}

#[test]
fn streaming_zero_length_updates_are_noops() {
    // CONFIGS 3.97 / ERRORS 3.64/3.89: `update(inlen = 0)` first, between and
    // last must not touch the state at all.  This is checked directly by
    // comparing the state before and after, in addition to the C/Rust compare
    // that `Stream::run` performs after every update.
    let sb = Stream::new("crypto_generichash_blake2b");
    let upd = both::<GhUpdate>("crypto_generichash_blake2b_update");
    let ini = both::<GhInit>("crypto_generichash_blake2b_init");
    let (gc, gr) = both::<Gh>("crypto_generichash_blake2b");
    let empty: [u8; 0] = [];

    for &total in &[0usize, 1, 128, 129, 256, 300] {
        let msg = pattern(2, total);
        let half = total / 2;
        let chunks: Vec<&[u8]> = vec![&empty, &msg[..half], &empty, &empty, &msg[half..], &empty];
        let what = format!("zero-length updates (total={total})");
        let d = sb.run(&what, BYTES, std::ptr::null(), 0, &chunks, BYTES);
        let one = unsafe {
            os(&what, &gc, &gr, BYTES, msg.as_ptr(), total as u64, std::ptr::null(), 0, 0)
        };
        eqb(&format!("{what}: == one-shot"), &one, &d);
    }

    // in == NULL && inlen == 0 is also a no-op (ERRORS 3.64)
    let mut cs = St::zero();
    let mut rs = St::zero();
    unsafe {
        assert_eq!((ini.0)(cs.p(), std::ptr::null(), 0, BYTES), 0);
        assert_eq!((ini.1)(rs.p(), std::ptr::null(), 0, BYTES), 0);
        let before = cs.0;
        let a = (upd.0)(cs.p(), std::ptr::null(), 0);
        let b = (upd.1)(rs.p(), std::ptr::null(), 0);
        eqi("update(NULL, 0) rc", a, b);
        assert_eq!(a, 0);
        eqb("update(NULL, 0) is a no-op", &before, cs.b());
        eqb("update(NULL, 0) C vs Rust", cs.b(), rs.b());
    }
}

#[test]
fn streaming_randomized_multi_chunk_splits() {
    // Randomized (fixed seed) multi-chunk splits, deliberately biased towards
    // 1-byte and zero-length chunks, over short and multi-KiB messages.
    let sb = Stream::new("crypto_generichash_blake2b");
    let (gc, gr) = both::<Gh>("crypto_generichash_blake2b");
    let key = pattern(3, KEYBYTES_MAX);
    let mut rng = Rng::new(0xB1A_2B_5EED);

    let mut totals: Vec<usize> = (0..=300).collect();
    totals.extend_from_slice(&[511, 512, 513, 1024, 2000, 4096, 8192, 9999]);

    for &total in &totals {
        let msg = pattern(if total % 3 == 0 { 2 } else { 3 }, total);
        for trial in 0..3usize {
            // build a random split
            let mut chunks: Vec<&[u8]> = Vec::new();
            let mut off = 0usize;
            while off < total {
                let n = match rng.below(6) {
                    0 => 0,
                    1 | 2 => 1,
                    3 => rng.below(9),
                    4 => rng.below(140),
                    _ => rng.below(300),
                };
                let n = n.min(total - off);
                chunks.push(&msg[off..off + n]);
                off += n;
                if chunks.len() > 4096 {
                    chunks.push(&msg[off..]);
                    off = total;
                }
            }
            // a few trailing empty updates
            chunks.push(&msg[total..]);
            chunks.push(&msg[total..]);

            let outlen = 1 + rng.below(64);
            let keylen = [0usize, 1, 16, 32, 64][rng.below(5)];
            let kp = if keylen == 0 { std::ptr::null() } else { key.as_ptr() };
            let what = format!("rand split total={total} trial={trial} out={outlen} key={keylen}");
            let d = sb.run(&what, outlen, kp, keylen, &chunks, outlen);
            let one = unsafe {
                os(&what, &gc, &gr, outlen, msg.as_ptr(), total as u64, kp, keylen, 0)
            };
            eqb(&format!("{what}: == one-shot"), &one, &d);
        }
    }
}

// ===========================================================================
// final: outlen != init outlen, and the aborting outlens
// CONFIGS 3.110–3.112, ERRORS 3.66–3.70
// ===========================================================================

#[test]
fn final_outlen_differs_from_init_outlen() {
    // ERRORS 3.70: no check exists.  `final` emits `outlen` bytes of the digest
    // computed with the *init* `digest_length`, so a shorter `final` outlen is
    // a prefix of the longer one and a longer `final` outlen is an extension.
    let sb = Stream::new("crypto_generichash_blake2b");
    let sg = Stream::new("crypto_generichash");
    let sizes = [1usize, 2, 15, 16, 17, 31, 32, 33, 63, 64];
    for &total in &[0usize, 1, 128, 129, 256, 300] {
        let msg = pattern(2, total);
        for &init_len in &sizes {
            // full 64-byte digest for this init length
            let full = sb.run(
                &format!("final full (init={init_len},total={total})"),
                init_len,
                std::ptr::null(),
                0,
                &[&msg[..]],
                BYTES_MAX,
            );
            for &fin_len in &sizes {
                let d = sb.run(
                    &format!("final(init={init_len},fin={fin_len},total={total})"),
                    init_len,
                    std::ptr::null(),
                    0,
                    &[&msg[..]],
                    fin_len,
                );
                assert_eq!(
                    &d[..],
                    &full[..fin_len],
                    "final(outlen={fin_len}) after init(outlen={init_len}) must be a prefix \
                     of the 64-byte digest of the same state"
                );
                let dg = sg.run(
                    &format!("generic final(init={init_len},fin={fin_len},total={total})"),
                    init_len,
                    std::ptr::null(),
                    0,
                    &[&msg[..]],
                    fin_len,
                );
                eqb("generic final == blake2b final", &d, &dg);
            }
            // and it is *not* the digest of an init with the final outlen,
            // whenever the two differ
            for &fin_len in &sizes {
                if fin_len == init_len {
                    continue;
                }
                let native = sb.run(
                    &format!("native({fin_len},total={total})"),
                    fin_len,
                    std::ptr::null(),
                    0,
                    &[&msg[..]],
                    fin_len,
                );
                let d = sb.run(
                    &format!("mismatch({init_len}->{fin_len},total={total})"),
                    init_len,
                    std::ptr::null(),
                    0,
                    &[&msg[..]],
                    fin_len,
                );
                assert_ne!(
                    native, d,
                    "digest_length is part of the parameter block: init({init_len}) then \
                     final({fin_len}) must differ from init({fin_len}) then final({fin_len})"
                );
            }
        }
    }
}

#[test]
fn final_outlen_zero_or_too_large_aborts() {
    // ERRORS 3.66–3.68: `crypto_generichash_blake2b_final` has no `-1` arm for
    // a bad `outlen`.  Asserts are live in this build (no -DNDEBUG), so
    // `outlen > UINT8_MAX` trips `assert(outlen <= UINT8_MAX)`; smaller invalid
    // values reach `blake2b_final`'s `sodium_misuse()`.  Either way the process
    // dies on SIGABRT.
    for prefix in ["crypto_generichash_blake2b", "crypto_generichash"] {
        let ini = both::<GhInit>(&format!("{prefix}_init"));
        let fin = both::<GhFinal>(&format!("{prefix}_final"));
        for &outlen in &[0usize, 65, 66, 100, 255, 256, 257, 512, usize::MAX] {
            let what = format!("{prefix}_final(outlen={outlen})");
            let ci = ini.0.clone();
            let ri = ini.1.clone();
            let cf = fin.0.clone();
            let rf = fin.1.clone();
            eq_abort(
                &what,
                move || unsafe {
                    let mut s = St::zero();
                    let mut o = [0u8; 64];
                    assert_eq!(ci(s.p(), std::ptr::null(), 0, BYTES), 0);
                    cf(s.p(), o.as_mut_ptr(), outlen);
                },
                move || unsafe {
                    let mut s = St::zero();
                    let mut o = [0u8; 64];
                    assert_eq!(ri(s.p(), std::ptr::null(), 0, BYTES), 0);
                    rf(s.p(), o.as_mut_ptr(), outlen);
                },
            );
        }
    }
}

#[test]
fn double_final_returns_minus_one() {
    // ERRORS 3.69: `blake2b_is_lastblock(S)` — the only `-1` in blake2b-ref.c.
    // `out` is not written and the state is left as the first `final` left it
    // (`h` and `buf` zeroed, `f[0] == -1`, `t`/`buflen` intact).
    for prefix in ["crypto_generichash_blake2b", "crypto_generichash"] {
        let ini = both::<GhInit>(&format!("{prefix}_init"));
        let upd = both::<GhUpdate>(&format!("{prefix}_update"));
        let fin = both::<GhFinal>(&format!("{prefix}_final"));
        for &total in &[0usize, 1, 128, 129, 256, 300] {
            let msg = pattern(2, total);
            for &outlen in &[1usize, 16, 32, 64] {
                let mut cs = St::zero();
                let mut rs = St::zero();
                unsafe {
                    assert_eq!((ini.0)(cs.p(), std::ptr::null(), 0, outlen), 0);
                    assert_eq!((ini.1)(rs.p(), std::ptr::null(), 0, outlen), 0);
                    (upd.0)(cs.p(), msg.as_ptr(), total as u64);
                    (upd.1)(rs.p(), msg.as_ptr(), total as u64);

                    let mut co = padded(outlen);
                    let mut ro = padded(outlen);
                    let a = (fin.0)(cs.p(), co.as_mut_ptr(), outlen);
                    let b = (fin.1)(rs.p(), ro.as_mut_ptr(), outlen);
                    eqi("first final rc", a, b);
                    assert_eq!(a, 0);
                    eqb("first digest", &co[..outlen], &ro[..outlen]);
                    eqb("state after first final", cs.b(), rs.b());
                    // f[0] == (uint64_t) -1 after finalization
                    assert_eq!(
                        &cs.b()[OFF_F..OFF_F + 8],
                        &[0xffu8; 8],
                        "f[0] must be all-ones after final"
                    );
                    // ERRORS 3.88: last_node is never set, so f[1] stays 0
                    assert_eq!(
                        &cs.b()[OFF_F + 8..OFF_F + 16],
                        &[0u8; 8],
                        "f[1] must stay zero (last_node is dead code)"
                    );
                    assert_eq!(cs.b()[OFF_LAST_NODE], 0, "last_node must stay 0");

                    // second final: -1, out untouched
                    let mut co2 = padded(outlen);
                    let mut ro2 = padded(outlen);
                    let before_c = cs.0;
                    let a = (fin.0)(cs.p(), co2.as_mut_ptr(), outlen);
                    let b = (fin.1)(rs.p(), ro2.as_mut_ptr(), outlen);
                    assert_eq!(a, -1, "{prefix}: second final must return -1");
                    eqi("second final rc", a, b);
                    eqb("second final leaves out untouched (C)", &padded(outlen), &co2);
                    eqb("second final leaves out untouched (Rust)", &padded(outlen), &ro2);
                    eqb("second final leaves the state untouched", &before_c, cs.b());
                    eqb("state after second final", cs.b(), rs.b());

                    // a third final behaves identically
                    let a = (fin.0)(cs.p(), co2.as_mut_ptr(), outlen);
                    let b = (fin.1)(rs.p(), ro2.as_mut_ptr(), outlen);
                    assert_eq!(a, -1);
                    eqi("third final rc", a, b);
                }
            }
        }
    }
}

#[test]
fn update_after_final() {
    // ERRORS 3.63: `blake2b_update` has no phase guard, so it happily buffers
    // into a finalized state and returns 0; the *next* `final` returns -1.
    let ini = both::<GhInit>("crypto_generichash_blake2b_init");
    let upd = both::<GhUpdate>("crypto_generichash_blake2b_update");
    let fin = both::<GhFinal>("crypto_generichash_blake2b_final");
    for &post in &[0usize, 1, 64, 128, 129, 200, 256, 300] {
        let msg = pattern(2, 137);
        let extra = pattern(1, post);
        let mut cs = St::zero();
        let mut rs = St::zero();
        unsafe {
            assert_eq!((ini.0)(cs.p(), std::ptr::null(), 0, BYTES), 0);
            assert_eq!((ini.1)(rs.p(), std::ptr::null(), 0, BYTES), 0);
            (upd.0)(cs.p(), msg.as_ptr(), msg.len() as u64);
            (upd.1)(rs.p(), msg.as_ptr(), msg.len() as u64);
            let mut co = padded(BYTES);
            let mut ro = padded(BYTES);
            assert_eq!((fin.0)(cs.p(), co.as_mut_ptr(), BYTES), 0);
            assert_eq!((fin.1)(rs.p(), ro.as_mut_ptr(), BYTES), 0);
            eqb("digest before update-after-final", &co[..BYTES], &ro[..BYTES]);

            let a = (upd.0)(cs.p(), extra.as_ptr(), post as u64);
            let b = (upd.1)(rs.p(), extra.as_ptr(), post as u64);
            assert_eq!(a, 0, "update after final must still return 0 (post={post})");
            eqi("update-after-final rc", a, b);
            eqb(
                &format!("state after update-after-final (post={post})"),
                cs.b(),
                rs.b(),
            );

            let mut co2 = padded(BYTES);
            let mut ro2 = padded(BYTES);
            let a = (fin.0)(cs.p(), co2.as_mut_ptr(), BYTES);
            let b = (fin.1)(rs.p(), ro2.as_mut_ptr(), BYTES);
            assert_eq!(a, -1, "final after update-after-final must be -1");
            eqi("final-after-update-after-final rc", a, b);
            eqb("state after that final", cs.b(), rs.b());
        }
    }
}

#[test]
fn state_reuse_reinit_after_final() {
    // CONFIGS 3.129 (blake2b part): `_init` after `_final` must fully reset —
    // `blake2b_init0` clears `t`, `f`, `buf`, `buflen`.
    let ini = both::<GhInit>("crypto_generichash_blake2b_init");
    let upd = both::<GhUpdate>("crypto_generichash_blake2b_update");
    let fin = both::<GhFinal>("crypto_generichash_blake2b_final");
    let key = pattern(3, KEYBYTES);
    let msg = pattern(2, 300);

    let mut cs = St::zero();
    let mut rs = St::zero();
    let mut fresh_c = St::zero();
    let mut fresh_r = St::zero();
    unsafe {
        for round in 0..4 {
            let keylen = if round % 2 == 0 { 0 } else { KEYBYTES };
            let kp = if keylen == 0 { std::ptr::null() } else { key.as_ptr() };
            let outlen = 1 + round * 16;

            let a = (ini.0)(cs.p(), kp, keylen, outlen);
            let b = (ini.1)(rs.p(), kp, keylen, outlen);
            eqi("re-init rc", a, b);
            assert_eq!(a, 0);
            eqb(&format!("state after re-init (round {round})"), cs.b(), rs.b());

            // a fresh state initialized the same way must be byte-identical
            fresh_c = St::zero();
            fresh_r = St::zero();
            assert_eq!((ini.0)(fresh_c.p(), kp, keylen, outlen), 0);
            assert_eq!((ini.1)(fresh_r.p(), kp, keylen, outlen), 0);
            eqb("re-init == fresh init (C)", fresh_c.b(), cs.b());
            eqb("re-init == fresh init (Rust)", fresh_r.b(), rs.b());

            (upd.0)(cs.p(), msg.as_ptr(), msg.len() as u64);
            (upd.1)(rs.p(), msg.as_ptr(), msg.len() as u64);
            eqb("state after update", cs.b(), rs.b());
            let mut co = padded(outlen);
            let mut ro = padded(outlen);
            assert_eq!((fin.0)(cs.p(), co.as_mut_ptr(), outlen), 0);
            assert_eq!((fin.1)(rs.p(), ro.as_mut_ptr(), outlen), 0);
            eqb("digest", &co[..outlen], &ro[..outlen]);
            eqb("state after final", cs.b(), rs.b());
        }
        let _ = (fresh_c, fresh_r);
    }
}

#[test]
fn state_is_a_shared_typedef() {
    // CONFIGS 3.113: `crypto_generichash_state` is a typedef of
    // `crypto_generichash_blake2b_state`, so the generic and blake2b entry
    // points are freely interchangeable on the same state object.
    let gi = both::<GhInit>("crypto_generichash_init");
    let bu = both::<GhUpdate>("crypto_generichash_blake2b_update");
    let gu = both::<GhUpdate>("crypto_generichash_update");
    let bf = both::<GhFinal>("crypto_generichash_blake2b_final");
    let bi = both::<GhInit>("crypto_generichash_blake2b_init");
    let gf = both::<GhFinal>("crypto_generichash_final");
    let (gc, gr) = both::<Gh>("crypto_generichash_blake2b");
    let msg = pattern(2, 300);

    unsafe {
        // generic init -> blake2b update -> blake2b final
        let mut cs = St::zero();
        let mut rs = St::zero();
        assert_eq!((gi.0)(cs.p(), std::ptr::null(), 0, BYTES), 0);
        assert_eq!((gi.1)(rs.p(), std::ptr::null(), 0, BYTES), 0);
        (bu.0)(cs.p(), msg.as_ptr(), msg.len() as u64);
        (bu.1)(rs.p(), msg.as_ptr(), msg.len() as u64);
        let mut co = padded(BYTES);
        let mut ro = padded(BYTES);
        assert_eq!((bf.0)(cs.p(), co.as_mut_ptr(), BYTES), 0);
        assert_eq!((bf.1)(rs.p(), ro.as_mut_ptr(), BYTES), 0);
        eqb("generic init + blake2b final", &co[..BYTES], &ro[..BYTES]);

        // blake2b init -> generic update -> generic final
        let mut cs2 = St::zero();
        let mut rs2 = St::zero();
        assert_eq!((bi.0)(cs2.p(), std::ptr::null(), 0, BYTES), 0);
        assert_eq!((bi.1)(rs2.p(), std::ptr::null(), 0, BYTES), 0);
        (gu.0)(cs2.p(), msg.as_ptr(), msg.len() as u64);
        (gu.1)(rs2.p(), msg.as_ptr(), msg.len() as u64);
        let mut co2 = padded(BYTES);
        let mut ro2 = padded(BYTES);
        assert_eq!((gf.0)(cs2.p(), co2.as_mut_ptr(), BYTES), 0);
        assert_eq!((gf.1)(rs2.p(), ro2.as_mut_ptr(), BYTES), 0);
        eqb("blake2b init + generic final", &co2[..BYTES], &ro2[..BYTES]);
        eqb("both cross-uses agree", &co[..BYTES], &co2[..BYTES]);

        let one = os(
            "one-shot reference",
            &gc,
            &gr,
            BYTES,
            msg.as_ptr(),
            msg.len() as u64,
            std::ptr::null(),
            0,
            0,
        );
        eqb("cross-use == one-shot", &one, &co[..BYTES]);
    }
}

// ===========================================================================
// key-block structure — CONFIGS 3.105, ERRORS 3.81
// ===========================================================================

#[test]
fn keyed_init_absorbs_one_padded_block() {
    let ini = both::<GhInit>("crypto_generichash_blake2b_init");
    let key64 = pattern(2, 64);
    let key1 = [key64[0]];
    let mut states: Vec<[u8; SB]> = Vec::new();
    unsafe {
        for (label, kp, kl) in [
            ("unkeyed", std::ptr::null::<u8>(), 0usize),
            ("keylen=1", key1.as_ptr(), 1),
            ("keylen=64", key64.as_ptr(), 64),
        ] {
            let mut cs = St::zero();
            let mut rs = St::zero();
            assert_eq!((ini.0)(cs.p(), kp, kl, BYTES), 0);
            assert_eq!((ini.1)(rs.p(), kp, kl, BYTES), 0);
            eqb(&format!("keyed init state [{label}]"), cs.b(), rs.b());
            assert_eq!(
                cs.b()[OFF_BUFLEN],
                if kl == 0 { 0 } else { 128 },
                "[{label}] buflen"
            );
            states.push(cs.0);
        }
    }
    assert_ne!(states[0], states[1]);
    assert_ne!(states[1], states[2]);
    assert_ne!(states[0], states[2]);
}

// ===========================================================================
// low-level `blake2b-ref.c` entry points (exported as `_sodium_blake2b*`)
// CONFIGS 3.106, ERRORS 3.74–3.92
// ===========================================================================

#[test]
fn low_level_init_variants_match_public_wrappers() {
    for n in [
        "_sodium_blake2b",
        "_sodium_blake2b_salt_personal",
        "_sodium_blake2b_init",
        "_sodium_blake2b_init_salt_personal",
        "_sodium_blake2b_init_key",
        "_sodium_blake2b_init_key_salt_personal",
        "_sodium_blake2b_init_param",
        "_sodium_blake2b_update",
        "_sodium_blake2b_final",
        "_sodium_blake2b_compress_ref",
    ] {
        assert!(has(n), "{n} must be exported by both");
    }

    let li = both::<B2Init>("_sodium_blake2b_init");
    let lisp = both::<B2InitSp>("_sodium_blake2b_init_salt_personal");
    let lik = both::<B2InitKey>("_sodium_blake2b_init_key");
    let liksp = both::<B2InitKeySp>("_sodium_blake2b_init_key_salt_personal");
    let lu = both::<B2Update>("_sodium_blake2b_update");
    let lf = both::<B2Final>("_sodium_blake2b_final");
    let pi = both::<GhInit>("crypto_generichash_blake2b_init");
    let pisp = both::<GhInitSp>("crypto_generichash_blake2b_init_salt_personal");

    let mut rng = Rng::new(0x3_10600);
    let salt = rng.bytes(SALTBYTES);
    let pers = rng.bytes(PERSONALBYTES);
    let key = rng.bytes(KEYBYTES_MAX);
    let msg = pattern(2, 300);

    unsafe {
        for outlen in 1..=64u8 {
            // blake2b_init
            let mut c1 = St::zero();
            let mut r1 = St::zero();
            let a = (li.0)(c1.p(), outlen);
            let b = (li.1)(r1.p(), outlen);
            eqi("blake2b_init rc", a, b);
            assert_eq!(a, 0);
            eqb(&format!("blake2b_init({outlen}) state"), c1.b(), r1.b());
            // equals the public unkeyed init
            let mut p1 = St::zero();
            assert_eq!((pi.0)(p1.p(), std::ptr::null(), 0, outlen as usize), 0);
            eqb("blake2b_init == public init", c1.b(), p1.b());

            // blake2b_init_salt_personal, all four NULL combinations
            for (label, sp, pp) in [
                ("both NULL", std::ptr::null::<c_void>(), std::ptr::null::<c_void>()),
                ("salt only", salt.as_ptr() as *const c_void, std::ptr::null()),
                ("personal only", std::ptr::null(), pers.as_ptr() as *const c_void),
                (
                    "both set",
                    salt.as_ptr() as *const c_void,
                    pers.as_ptr() as *const c_void,
                ),
            ] {
                let mut c2 = St::zero();
                let mut r2 = St::zero();
                let a = (lisp.0)(c2.p(), outlen, sp, pp);
                let b = (lisp.1)(r2.p(), outlen, sp, pp);
                eqi("init_salt_personal rc", a, b);
                assert_eq!(a, 0);
                eqb(&format!("init_salt_personal[{label}]({outlen})"), c2.b(), r2.b());
                let mut p2 = St::zero();
                assert_eq!(
                    (pisp.0)(
                        p2.p(),
                        std::ptr::null(),
                        0,
                        outlen as usize,
                        sp as *const u8,
                        pp as *const u8
                    ),
                    0
                );
                eqb("low-level == public init_salt_personal", c2.b(), p2.b());
            }

            // blake2b_init_key / _key_salt_personal
            for keylen in [1u8, 15, 16, 32, 63, 64] {
                let mut c3 = St::zero();
                let mut r3 = St::zero();
                let a = (lik.0)(c3.p(), outlen, key.as_ptr() as *const c_void, keylen);
                let b = (lik.1)(r3.p(), outlen, key.as_ptr() as *const c_void, keylen);
                eqi("init_key rc", a, b);
                assert_eq!(a, 0);
                eqb(&format!("init_key({outlen},{keylen})"), c3.b(), r3.b());
                let mut p3 = St::zero();
                assert_eq!(
                    (pi.0)(p3.p(), key.as_ptr(), keylen as usize, outlen as usize),
                    0
                );
                eqb("init_key == public keyed init", c3.b(), p3.b());

                let mut c4 = St::zero();
                let mut r4 = St::zero();
                let a = (liksp.0)(
                    c4.p(),
                    outlen,
                    key.as_ptr() as *const c_void,
                    keylen,
                    salt.as_ptr() as *const c_void,
                    pers.as_ptr() as *const c_void,
                );
                let b = (liksp.1)(
                    r4.p(),
                    outlen,
                    key.as_ptr() as *const c_void,
                    keylen,
                    salt.as_ptr() as *const c_void,
                    pers.as_ptr() as *const c_void,
                );
                eqi("init_key_salt_personal rc", a, b);
                assert_eq!(a, 0);
                eqb(
                    &format!("init_key_salt_personal({outlen},{keylen})"),
                    c4.b(),
                    r4.b(),
                );
                let mut p4 = St::zero();
                assert_eq!(
                    (pisp.0)(
                        p4.p(),
                        key.as_ptr(),
                        keylen as usize,
                        outlen as usize,
                        salt.as_ptr(),
                        pers.as_ptr()
                    ),
                    0
                );
                eqb("low-level == public keyed init_salt_personal", c4.b(), p4.b());
            }
        }

        // low-level update/final streaming
        for &total in &[0usize, 1, 128, 129, 256, 257, 300] {
            let mut cs = St::zero();
            let mut rs = St::zero();
            assert_eq!((li.0)(cs.p(), 32), 0);
            assert_eq!((li.1)(rs.p(), 32), 0);
            for ch in msg[..total].chunks(37) {
                let a = (lu.0)(cs.p(), ch.as_ptr(), ch.len() as u64);
                let b = (lu.1)(rs.p(), ch.as_ptr(), ch.len() as u64);
                eqi("low-level update rc", a, b);
                eqb("low-level update state", cs.b(), rs.b());
            }
            let mut co = padded(32);
            let mut ro = padded(32);
            let a = (lf.0)(cs.p(), co.as_mut_ptr(), 32);
            let b = (lf.1)(rs.p(), ro.as_mut_ptr(), 32);
            eqi("low-level final rc", a, b);
            assert_eq!(a, 0);
            eqb("low-level digest", &co[..32], &ro[..32]);
            eqb("low-level state after final", cs.b(), rs.b());
        }
    }
}

#[test]
fn low_level_one_shots() {
    let (bc, br) = both::<B2OneShot>("_sodium_blake2b");
    let (sc2, sr2) = both::<B2OneShotSp>("_sodium_blake2b_salt_personal");
    let (gc, gr) = both::<Gh>("crypto_generichash_blake2b");
    let (pc, pr) = both::<GhSp>("crypto_generichash_blake2b_salt_personal");
    let mut rng = Rng::new(0x3_10601);
    let salt = rng.bytes(SALTBYTES);
    let pers = rng.bytes(PERSONALBYTES);
    let key = rng.bytes(KEYBYTES_MAX);
    unsafe {
        for &total in &[0usize, 1, 127, 128, 129, 255, 256, 257, 300] {
            let msg = pattern(2, total);
            for outlen in [1u8, 16, 32, 64] {
                for keylen in [0u8, 1, 16, 32, 64] {
                    let kp = if keylen == 0 {
                        std::ptr::null()
                    } else {
                        key.as_ptr() as *const c_void
                    };
                    let mut co = padded(outlen as usize);
                    let mut ro = padded(outlen as usize);
                    let a = bc(
                        co.as_mut_ptr(),
                        msg.as_ptr() as *const c_void,
                        kp,
                        outlen,
                        total as u64,
                        keylen,
                    );
                    let b = br(
                        ro.as_mut_ptr(),
                        msg.as_ptr() as *const c_void,
                        kp,
                        outlen,
                        total as u64,
                        keylen,
                    );
                    eqi("_sodium_blake2b rc", a, b);
                    assert_eq!(a, 0);
                    check_pad("_sodium_blake2b C", &co, outlen as usize);
                    check_pad("_sodium_blake2b Rust", &ro, outlen as usize);
                    eqb("_sodium_blake2b digest", &co[..outlen as usize], &ro[..outlen as usize]);
                    let pub_d = os(
                        "public one-shot",
                        &gc,
                        &gr,
                        outlen as usize,
                        msg.as_ptr(),
                        total as u64,
                        kp as *const u8,
                        keylen as usize,
                        0,
                    );
                    eqb("_sodium_blake2b == public", &pub_d, &co[..outlen as usize]);

                    let mut co2 = padded(outlen as usize);
                    let mut ro2 = padded(outlen as usize);
                    let a = sc2(
                        co2.as_mut_ptr(),
                        msg.as_ptr() as *const c_void,
                        kp,
                        outlen,
                        total as u64,
                        keylen,
                        salt.as_ptr() as *const c_void,
                        pers.as_ptr() as *const c_void,
                    );
                    let b = sr2(
                        ro2.as_mut_ptr(),
                        msg.as_ptr() as *const c_void,
                        kp,
                        outlen,
                        total as u64,
                        keylen,
                        salt.as_ptr() as *const c_void,
                        pers.as_ptr() as *const c_void,
                    );
                    eqi("_sodium_blake2b_salt_personal rc", a, b);
                    assert_eq!(a, 0);
                    eqb(
                        "_sodium_blake2b_salt_personal digest",
                        &co2[..outlen as usize],
                        &ro2[..outlen as usize],
                    );
                    let pub_sp = os_sp(
                        "public salt_personal",
                        &pc,
                        &pr,
                        outlen as usize,
                        msg.as_ptr(),
                        total as u64,
                        kp as *const u8,
                        keylen as usize,
                        salt.as_ptr(),
                        pers.as_ptr(),
                        0,
                    );
                    eqb(
                        "_sodium_blake2b_salt_personal == public",
                        &pub_sp,
                        &co2[..outlen as usize],
                    );
                }
            }
        }
    }
}

#[test]
fn low_level_init_param_accepts_arbitrary_param_blocks() {
    // ERRORS 3.85–3.87 / CONFIGS 3.106: `blake2b_init_param` has no validation
    // whatsoever; the 64-byte block is XOR-ed into `h[0..8]` verbatim, so every
    // field (including nonsensical `digest_length`, `fanout`, `node_offset`,
    // `reserved`) must be reproduced bit-for-bit.
    let (c, r) = both::<B2InitParam>("_sodium_blake2b_init_param");
    let mut rng = Rng::new(0x3_10610);
    let mut blocks: Vec<[u8; 64]> = Vec::new();
    blocks.push([0u8; 64]);
    blocks.push([0xffu8; 64]);
    {
        let mut b = [0u8; 64];
        for (i, x) in b.iter_mut().enumerate() {
            *x = i as u8;
        }
        blocks.push(b);
    }
    // realistic blocks: digest_length x key_length x salt/personal
    for &dl in &[0u8, 1, 16, 32, 64, 65, 255] {
        for &kl in &[0u8, 1, 32, 64, 65, 255] {
            let mut b = [0u8; 64];
            b[0] = dl;
            b[1] = kl;
            b[2] = 1;
            b[3] = 1;
            rng.fill(&mut b[32..64]);
            blocks.push(b);
        }
    }
    for _ in 0..16 {
        let mut b = [0u8; 64];
        rng.fill(&mut b);
        blocks.push(b);
    }
    unsafe {
        for (i, b) in blocks.iter().enumerate() {
            let mut cs = St::zero();
            let mut rs = St::zero();
            let a = c(cs.p(), b.as_ptr());
            let d = r(rs.p(), b.as_ptr());
            eqi(&format!("init_param[{i}] rc"), a, d);
            assert_eq!(a, 0, "blake2b_init_param must always return 0");
            eqb(&format!("init_param[{i}] state"), cs.b(), rs.b());
            // init0 leaves everything from `t` onwards zero and only the first
            // 361 bytes of the 384-byte opaque buffer are ever written
            assert!(
                cs.b()[64..INTERNAL_STATE_SIZE].iter().all(|&x| x == 0),
                "init_param must zero t/f/buf/buflen/last_node"
            );
            assert!(
                cs.b()[INTERNAL_STATE_SIZE..].iter().all(|&x| x == 0),
                "nothing may be written past sizeof(blake2b_state)"
            );
        }
    }
}

#[test]
fn low_level_compress_ref() {
    // ERRORS 3.92 / CONFIGS 3.116: the reference compression function, driven
    // directly over a fully randomized state and block.
    let (c, r) = both::<unsafe extern "C" fn(*mut c_void, *const u8) -> c_int>(
        "_sodium_blake2b_compress_ref",
    );
    let li = both::<B2Init>("_sodium_blake2b_init");
    let mut rng = Rng::new(0x3_11600);
    unsafe {
        for i in 0..64 {
            let mut cs = St::zero();
            let mut rs = St::zero();
            assert_eq!((li.0)(cs.p(), 1 + (i % 64) as u8), 0);
            assert_eq!((li.1)(rs.p(), 1 + (i % 64) as u8), 0);
            // scribble over t/f/buf so that all four v[12..16] inputs vary
            let mut scribble = [0u8; INTERNAL_STATE_SIZE - 64];
            rng.fill(&mut scribble);
            cs.0[64..INTERNAL_STATE_SIZE].copy_from_slice(&scribble);
            rs.0[64..INTERNAL_STATE_SIZE].copy_from_slice(&scribble);
            let block = rng.bytes(128);
            let a = c(cs.p(), block.as_ptr());
            let b = r(rs.p(), block.as_ptr());
            eqi("compress_ref rc", a, b);
            assert_eq!(a, 0);
            eqb(&format!("compress_ref[{i}] state"), cs.b(), rs.b());
        }
    }
}

// ===========================================================================
// misc axes — CONFIGS 3.107, 3.128, 3.130, 3.131
// ===========================================================================

#[test]
fn aliased_out_and_in() {
    // CONFIGS 3.130: `out` overlapping `in` is defined for the one-shots
    // because the digest is only written after the whole input is consumed.
    let (gc, gr) = both::<Gh>("crypto_generichash_blake2b");
    let (sc, sr) = both::<GhSp>("crypto_generichash_blake2b_salt_personal");
    let key = pattern(3, KEYBYTES);
    let salt = pattern(2, SALTBYTES);
    let pers = pattern(1, PERSONALBYTES);
    for &total in &[1usize, 32, 64, 65, 127, 128, 129, 256, 300] {
        for &outlen in &[1usize, 16, 32, 64] {
            for &off in &[0usize, 1, 7] {
                if off + outlen > total {
                    continue;
                }
                for &keylen in &[0usize, 32] {
                    let kp = if keylen == 0 { std::ptr::null() } else { key.as_ptr() };
                    let src = pattern(2, total);
                    // reference digest from a non-aliased call
                    let want = unsafe {
                        os(
                            "aliased reference",
                            &gc,
                            &gr,
                            outlen,
                            src.as_ptr(),
                            total as u64,
                            kp,
                            keylen,
                            0,
                        )
                    };
                    let mut cb = src.clone();
                    let mut rb = src.clone();
                    unsafe {
                        let a = gc(
                            cb.as_mut_ptr().add(off),
                            outlen,
                            cb.as_ptr(),
                            total as u64,
                            kp,
                            keylen,
                        );
                        let b = gr(
                            rb.as_mut_ptr().add(off),
                            outlen,
                            rb.as_ptr(),
                            total as u64,
                            kp,
                            keylen,
                        );
                        eqi("aliased rc", a, b);
                        assert_eq!(a, 0);
                    }
                    eqb(
                        &format!("aliased buffer (total={total},out={outlen},off={off})"),
                        &cb,
                        &rb,
                    );
                    assert_eq!(&cb[off..off + outlen], &want[..], "aliased digest changed");

                    let mut cb2 = src.clone();
                    let mut rb2 = src.clone();
                    unsafe {
                        let a = sc(
                            cb2.as_mut_ptr().add(off),
                            outlen,
                            cb2.as_ptr(),
                            total as u64,
                            kp,
                            keylen,
                            salt.as_ptr(),
                            pers.as_ptr(),
                        );
                        let b = sr(
                            rb2.as_mut_ptr().add(off),
                            outlen,
                            rb2.as_ptr(),
                            total as u64,
                            kp,
                            keylen,
                            salt.as_ptr(),
                            pers.as_ptr(),
                        );
                        eqi("aliased sp rc", a, b);
                        assert_eq!(a, 0);
                    }
                    eqb("aliased salt_personal buffer", &cb2, &rb2);
                }
            }
        }
    }
}

#[test]
fn last_node_is_inert() {
    // CONFIGS 3.107 / ERRORS 3.88: `last_node` is zeroed by `blake2b_init0` and
    // never set, so `f[1]` stays 0 through init, update and final.
    let ini = both::<GhInit>("crypto_generichash_blake2b_init");
    let upd = both::<GhUpdate>("crypto_generichash_blake2b_update");
    let fin = both::<GhFinal>("crypto_generichash_blake2b_final");
    let msg = pattern(2, 400);
    unsafe {
        for &keylen in &[0usize, 32] {
            let key = pattern(3, KEYBYTES);
            let kp = if keylen == 0 { std::ptr::null() } else { key.as_ptr() };
            let mut cs = St::zero();
            let mut rs = St::zero();
            assert_eq!((ini.0)(cs.p(), kp, keylen, BYTES), 0);
            assert_eq!((ini.1)(rs.p(), kp, keylen, BYTES), 0);
            for phase in ["init", "update", "final"] {
                if phase == "update" {
                    (upd.0)(cs.p(), msg.as_ptr(), msg.len() as u64);
                    (upd.1)(rs.p(), msg.as_ptr(), msg.len() as u64);
                } else if phase == "final" {
                    let mut co = padded(BYTES);
                    let mut ro = padded(BYTES);
                    assert_eq!((fin.0)(cs.p(), co.as_mut_ptr(), BYTES), 0);
                    assert_eq!((fin.1)(rs.p(), ro.as_mut_ptr(), BYTES), 0);
                }
                for (lbl, st) in [("C", &cs), ("Rust", &rs)] {
                    assert_eq!(
                        &st.b()[OFF_F + 8..OFF_F + 16],
                        &[0u8; 8],
                        "{lbl}: f[1] must be 0 after {phase}"
                    );
                    assert_eq!(
                        st.b()[OFF_LAST_NODE], 0,
                        "{lbl}: last_node must be 0 after {phase}"
                    );
                }
                eqb(&format!("state after {phase}"), cs.b(), rs.b());
            }
        }
    }
}

#[test]
fn nothing_is_written_past_the_internal_state() {
    // The public state is 384 bytes but `sizeof(blake2b_state)` is 361; neither
    // implementation may write into the 23-byte tail.
    let ini = both::<GhInit>("crypto_generichash_blake2b_init");
    let upd = both::<GhUpdate>("crypto_generichash_blake2b_update");
    let fin = both::<GhFinal>("crypto_generichash_blake2b_final");
    let key = pattern(3, KEYBYTES_MAX);
    let msg = pattern(2, 600);
    unsafe {
        for &keylen in &[0usize, 64] {
            let kp = if keylen == 0 { std::ptr::null() } else { key.as_ptr() };
            let mut cs = St::zero();
            let mut rs = St::zero();
            assert_eq!((ini.0)(cs.p(), kp, keylen, BYTES_MAX), 0);
            assert_eq!((ini.1)(rs.p(), kp, keylen, BYTES_MAX), 0);
            for ch in msg.chunks(97) {
                (upd.0)(cs.p(), ch.as_ptr(), ch.len() as u64);
                (upd.1)(rs.p(), ch.as_ptr(), ch.len() as u64);
            }
            let mut co = padded(BYTES_MAX);
            let mut ro = padded(BYTES_MAX);
            assert_eq!((fin.0)(cs.p(), co.as_mut_ptr(), BYTES_MAX), 0);
            assert_eq!((fin.1)(rs.p(), ro.as_mut_ptr(), BYTES_MAX), 0);
            for (lbl, st) in [("C", &cs), ("Rust", &rs)] {
                assert!(
                    st.b()[INTERNAL_STATE_SIZE..].iter().all(|&x| x == 0),
                    "{lbl}: wrote past sizeof(blake2b_state)"
                );
            }
            eqb("state tails agree", cs.b(), rs.b());
        }
    }
}

#[test]
fn known_answer_sanity() {
    // A cheap independent anchor so that "C == Rust" cannot be satisfied by two
    // identically broken implementations: BLAKE2b-512 of the empty message.
    let (gc, gr) = both::<Gh>("crypto_generichash_blake2b");
    let want = "786a02f742015903c6c6fd852552d272912f4740e15847618a86e217f71f5419\
                d25e1031afee585313896444934eb04b903a685b1448b755d56f701afe9be2ce";
    unsafe {
        let d = os(
            "blake2b-512(\"\")",
            &gc,
            &gr,
            64,
            std::ptr::null(),
            0,
            std::ptr::null(),
            0,
            0,
        );
        assert_eq!(hex(&d), want, "BLAKE2b-512 of the empty message");
    }
    // BLAKE2b-256("abc")
    let msg = b"abc";
    unsafe {
        let d = os(
            "blake2b-256(\"abc\")",
            &gc,
            &gr,
            32,
            msg.as_ptr(),
            3,
            std::ptr::null(),
            0,
            0,
        );
        assert_eq!(
            hex(&d),
            "bddd813c634239723171ef3fee98579b94964e3bb1cb3e427262c8c068d52319"
        );
    }
}

// ===========================================================================
// low-level misuse paths — ERRORS 3.67/3.68, 3.74–3.79, 3.82/3.83
//
// These `sodium_misuse()` calls are all marked LCOV_EXCL / "unreachable" from
// the *public* `crypto_generichash*` API (which pre-filters with `-1`), but
// `blake2b-ref.c`'s own entry points are exported as `_sodium_blake2b*` by both
// shared objects, so every one of them can be driven directly and compared.
// ===========================================================================

/// Every low-level abort site, driven through the exported `_sodium_blake2b*`
/// symbols.  Each entry is `(label, C thunk, Rust thunk)`.
macro_rules! ll_abort {
    ($what:expr, $csym:expr, $rsym:expr, |$f:ident| $body:block) => {{
        let c = $csym.clone();
        let r = $rsym.clone();
        eq_abort(
            $what,
            move || unsafe {
                let $f = c;
                $body
            },
            move || unsafe {
                let $f = r;
                $body
            },
        );
    }};
}

#[test]
fn low_level_init_misuse_aborts() {
    // ERRORS 3.74 / 3.75: `!outlen || outlen > BLAKE2B_OUTBYTES`.
    let li = both::<B2Init>("_sodium_blake2b_init");
    let lisp = both::<B2InitSp>("_sodium_blake2b_init_salt_personal");
    for outlen in [0u8, 65, 66, 128, 200, 255] {
        ll_abort!(
            &format!("_sodium_blake2b_init(outlen={outlen})"),
            li.0,
            li.1,
            |f| {
                let mut s = St::zero();
                f(s.p(), outlen);
            }
        );
        ll_abort!(
            &format!("_sodium_blake2b_init_salt_personal(outlen={outlen})"),
            lisp.0,
            lisp.1,
            |f| {
                let mut s = St::zero();
                let salt = [7u8; SALTBYTES];
                let pers = [9u8; PERSONALBYTES];
                f(
                    s.p(),
                    outlen,
                    salt.as_ptr() as *const c_void,
                    pers.as_ptr() as *const c_void,
                );
            }
        );
    }
}

#[test]
fn low_level_init_key_misuse_aborts() {
    // ERRORS 3.76–3.79 and 3.82/3.83.
    let lik = both::<B2InitKey>("_sodium_blake2b_init_key");
    let liksp = both::<B2InitKeySp>("_sodium_blake2b_init_key_salt_personal");

    // bad outlen with a perfectly good key
    for outlen in [0u8, 65, 255] {
        ll_abort!(
            &format!("_sodium_blake2b_init_key(outlen={outlen})"),
            lik.0,
            lik.1,
            |f| {
                let mut s = St::zero();
                let key = [3u8; 32];
                f(s.p(), outlen, key.as_ptr() as *const c_void, 32);
            }
        );
        ll_abort!(
            &format!("_sodium_blake2b_init_key_salt_personal(outlen={outlen})"),
            liksp.0,
            liksp.1,
            |f| {
                let mut s = St::zero();
                let key = [3u8; 32];
                let salt = [7u8; SALTBYTES];
                let pers = [9u8; PERSONALBYTES];
                f(
                    s.p(),
                    outlen,
                    key.as_ptr() as *const c_void,
                    32,
                    salt.as_ptr() as *const c_void,
                    pers.as_ptr() as *const c_void,
                );
            }
        );
    }

    // key == NULL (ERRORS 3.77 / 3.83)
    for keylen in [1u8, 32, 64] {
        ll_abort!(
            &format!("_sodium_blake2b_init_key(key=NULL,keylen={keylen})"),
            lik.0,
            lik.1,
            |f| {
                let mut s = St::zero();
                f(s.p(), 32, std::ptr::null(), keylen);
            }
        );
        ll_abort!(
            &format!("_sodium_blake2b_init_key_salt_personal(key=NULL,keylen={keylen})"),
            liksp.0,
            liksp.1,
            |f| {
                let mut s = St::zero();
                f(
                    s.p(),
                    32,
                    std::ptr::null(),
                    keylen,
                    std::ptr::null(),
                    std::ptr::null(),
                );
            }
        );
    }

    // keylen == 0 (ERRORS 3.78) and keylen > 64 (ERRORS 3.79)
    for keylen in [0u8, 65, 66, 128, 255] {
        ll_abort!(
            &format!("_sodium_blake2b_init_key(keylen={keylen})"),
            lik.0,
            lik.1,
            |f| {
                let mut s = St::zero();
                let key = [3u8; 255];
                f(s.p(), 32, key.as_ptr() as *const c_void, keylen);
            }
        );
        ll_abort!(
            &format!("_sodium_blake2b_init_key_salt_personal(keylen={keylen})"),
            liksp.0,
            liksp.1,
            |f| {
                let mut s = St::zero();
                let key = [3u8; 255];
                f(
                    s.p(),
                    32,
                    key.as_ptr() as *const c_void,
                    keylen,
                    std::ptr::null(),
                    std::ptr::null(),
                );
            }
        );
    }
}

#[test]
fn low_level_final_misuse_aborts() {
    // ERRORS 3.67 / 3.68 driven directly (`crypto_generichash_blake2b_final`
    // reaches the same two sites after its `(uint8_t)` truncation).
    let li = both::<B2Init>("_sodium_blake2b_init");
    let lf = both::<B2Final>("_sodium_blake2b_final");
    for outlen in [0u8, 65, 66, 128, 255] {
        let ci = li.0.clone();
        let ri = li.1.clone();
        let cf = lf.0.clone();
        let rf = lf.1.clone();
        eq_abort(
            &format!("_sodium_blake2b_final(outlen={outlen})"),
            move || unsafe {
                let mut s = St::zero();
                let mut o = [0u8; 255];
                assert_eq!(ci(s.p(), 32), 0);
                cf(s.p(), o.as_mut_ptr(), outlen);
            },
            move || unsafe {
                let mut s = St::zero();
                let mut o = [0u8; 255];
                assert_eq!(ri(s.p(), 32), 0);
                rf(s.p(), o.as_mut_ptr(), outlen);
            },
        );
    }
}

#[test]
fn low_level_one_shot_misuse_aborts() {
    // Every `sodium_misuse()` in `blake2b()` / `blake2b_salt_personal()`.
    // `outlen`/`keylen` out of range are only reachable here: the public
    // wrappers turn them into `-1` first.
    let b = both::<B2OneShot>("_sodium_blake2b");
    let bsp = both::<B2OneShotSp>("_sodium_blake2b_salt_personal");

    // in == NULL && inlen > 0
    for inlen in [1u64, 128, 1000] {
        ll_abort!(&format!("_sodium_blake2b(in=NULL,inlen={inlen})"), b.0, b.1, |f| {
            let mut o = [0u8; 64];
            f(o.as_mut_ptr(), std::ptr::null(), std::ptr::null(), 32, inlen, 0);
        });
        ll_abort!(
            &format!("_sodium_blake2b_salt_personal(in=NULL,inlen={inlen})"),
            bsp.0,
            bsp.1,
            |f| {
                let mut o = [0u8; 64];
                f(
                    o.as_mut_ptr(),
                    std::ptr::null(),
                    std::ptr::null(),
                    32,
                    inlen,
                    0,
                    std::ptr::null(),
                    std::ptr::null(),
                );
            }
        );
    }

    // out == NULL
    ll_abort!("_sodium_blake2b(out=NULL)", b.0, b.1, |f| {
        let m = [1u8; 64];
        f(
            std::ptr::null_mut(),
            m.as_ptr() as *const c_void,
            std::ptr::null(),
            32,
            64,
            0,
        );
    });
    ll_abort!("_sodium_blake2b_salt_personal(out=NULL)", bsp.0, bsp.1, |f| {
        let m = [1u8; 64];
        f(
            std::ptr::null_mut(),
            m.as_ptr() as *const c_void,
            std::ptr::null(),
            32,
            64,
            0,
            std::ptr::null(),
            std::ptr::null(),
        );
    });

    // outlen 0 / > 64
    for outlen in [0u8, 65, 255] {
        ll_abort!(&format!("_sodium_blake2b(outlen={outlen})"), b.0, b.1, |f| {
            let mut o = [0u8; 255];
            let m = [1u8; 64];
            f(
                o.as_mut_ptr(),
                m.as_ptr() as *const c_void,
                std::ptr::null(),
                outlen,
                64,
                0,
            );
        });
        ll_abort!(
            &format!("_sodium_blake2b_salt_personal(outlen={outlen})"),
            bsp.0,
            bsp.1,
            |f| {
                let mut o = [0u8; 255];
                let m = [1u8; 64];
                f(
                    o.as_mut_ptr(),
                    m.as_ptr() as *const c_void,
                    std::ptr::null(),
                    outlen,
                    64,
                    0,
                    std::ptr::null(),
                    std::ptr::null(),
                );
            }
        );
    }

    // key == NULL && keylen > 0, and keylen > 64
    for keylen in [1u8, 32, 64] {
        ll_abort!(
            &format!("_sodium_blake2b(key=NULL,keylen={keylen})"),
            b.0,
            b.1,
            |f| {
                let mut o = [0u8; 64];
                let m = [1u8; 64];
                f(
                    o.as_mut_ptr(),
                    m.as_ptr() as *const c_void,
                    std::ptr::null(),
                    32,
                    64,
                    keylen,
                );
            }
        );
    }
    for keylen in [65u8, 66, 255] {
        ll_abort!(&format!("_sodium_blake2b(keylen={keylen})"), b.0, b.1, |f| {
            let mut o = [0u8; 64];
            let m = [1u8; 64];
            let k = [2u8; 255];
            f(
                o.as_mut_ptr(),
                m.as_ptr() as *const c_void,
                k.as_ptr() as *const c_void,
                32,
                64,
                keylen,
            );
        });
        ll_abort!(
            &format!("_sodium_blake2b_salt_personal(keylen={keylen})"),
            bsp.0,
            bsp.1,
            |f| {
                let mut o = [0u8; 64];
                let m = [1u8; 64];
                let k = [2u8; 255];
                f(
                    o.as_mut_ptr(),
                    m.as_ptr() as *const c_void,
                    k.as_ptr() as *const c_void,
                    32,
                    64,
                    keylen,
                    std::ptr::null(),
                    std::ptr::null(),
                );
            }
        );
    }
}
