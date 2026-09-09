//! Phase B — valid-path differential tests for group G6
//! (CONFIGS.md rows 889-995):
//!   * `randombytes/` — `randombytes.c`,
//!     `internal/randombytes_internal_random.c`,
//!     `sysrandom/randombytes_sysrandom.c`
//!   * `crypto_kem/`  — ML-KEM-768 (ref) and X-Wing
//!   * `crypto_ipcrypt/` — `crypto_ipcrypt.c`, `ipcrypt_soft.c`
//!
//! Every test drives BOTH libraries through the dynamic symbol table and
//! compares the results byte-for-byte.
mod common;
use common::*;
use libloading::{Library, Symbol};
use std::ffi::CStr;
use std::os::raw::{c_char, c_int, c_void};
use std::ptr;
use std::sync::atomic::{AtomicU32, AtomicU64, AtomicUsize, Ordering as AtOrd};

const SEED: u64 = 0x6A09_E667_F3BC_C908;

// ===========================================================================
// signatures
// ===========================================================================
type BufDet = unsafe extern "C" fn(*mut c_void, usize, *const u8);
type Buf = unsafe extern "C" fn(*mut c_void, usize);
type NaclRb = unsafe extern "C" fn(*mut u8, u64);
type SizeGet = unsafe extern "C" fn() -> usize;
type Uniform = unsafe extern "C" fn(u32) -> u32;
type Random = unsafe extern "C" fn() -> u32;
type Stir = unsafe extern "C" fn();
type Close = unsafe extern "C" fn() -> c_int;
type SetImpl = unsafe extern "C" fn(*const RandombytesImpl) -> c_int;
type NameGet = unsafe extern "C" fn() -> *const c_char;

type SeedKp = unsafe extern "C" fn(*mut u8, *mut u8, *const u8) -> c_int;
type Kp = unsafe extern "C" fn(*mut u8, *mut u8) -> c_int;
type Enc = unsafe extern "C" fn(*mut u8, *mut u8, *const u8) -> c_int;
type EncDet = unsafe extern "C" fn(*mut u8, *mut u8, *const u8, *const u8) -> c_int;
type Dec = unsafe extern "C" fn(*mut u8, *const u8, *const u8) -> c_int;

type Sha3256 = unsafe extern "C" fn(*mut u8, *const u8, u64) -> c_int;
type Shake256 = unsafe extern "C" fn(*mut u8, usize, *const u8, u64) -> c_int;
type ScalarmultBase = unsafe extern "C" fn(*mut u8, *const u8) -> c_int;

type Ip3 = unsafe extern "C" fn(*mut u8, *const u8, *const u8);
type Ip4 = unsafe extern "C" fn(*mut u8, *const u8, *const u8, *const u8);
type Keygen = unsafe extern "C" fn(*mut u8);
type PickBest = unsafe extern "C" fn() -> c_int;

// ML-KEM-768 / X-Wing sizes
const MLK_PK: usize = 1184;
const MLK_SK: usize = 2400;
const MLK_CT: usize = 1088;
const SS: usize = 32;
const MLK_SEED: usize = 64;
const XW_PK: usize = 1216;
const XW_SK: usize = 32;
const XW_CT: usize = 1120;
const XW_SEED: usize = 32;
const XW_ENC_SEED: usize = 64;

// ===========================================================================
// custom, fully deterministic `randombytes_implementation`s, installed into
// BOTH libraries so that `randombytes_uniform`'s rejection-sampling loop,
// `randombytes_random` and `randombytes_buf` become byte-comparable, and so
// that hook invocation can be *counted*.
// ===========================================================================
const SEQ_MAX: usize = 32;
static SEQ: [AtomicU32; SEQ_MAX] = [const { AtomicU32::new(0) }; SEQ_MAX];
static SEQ_LEN: AtomicUsize = AtomicUsize::new(0);
static SEQ_POS: AtomicUsize = AtomicUsize::new(0);
static RAND_CALLS: AtomicUsize = AtomicUsize::new(0);
static BUF_CALLS: AtomicUsize = AtomicUsize::new(0);
static BUF_BYTES: AtomicUsize = AtomicUsize::new(0);
static STIR_CALLS: AtomicUsize = AtomicUsize::new(0);
static CLOSE_CALLS: AtomicUsize = AtomicUsize::new(0);
static BUF_STREAM: AtomicU64 = AtomicU64::new(1);

/// Load a fixed `random()` sequence; the last element repeats for ever.
fn seq_set(vals: &[u32]) {
    assert!(vals.len() <= SEQ_MAX);
    for (i, v) in vals.iter().enumerate() {
        SEQ[i].store(*v, AtOrd::SeqCst);
    }
    SEQ_LEN.store(vals.len(), AtOrd::SeqCst);
    SEQ_POS.store(0, AtOrd::SeqCst);
    RAND_CALLS.store(0, AtOrd::SeqCst);
}

fn seq_rewind() {
    SEQ_POS.store(0, AtOrd::SeqCst);
    RAND_CALLS.store(0, AtOrd::SeqCst);
}

extern "C" fn cnt_name() -> *const c_char {
    b"det\0".as_ptr() as *const c_char
}

extern "C" fn cnt_random() -> u32 {
    RAND_CALLS.fetch_add(1, AtOrd::SeqCst);
    let len = SEQ_LEN.load(AtOrd::SeqCst);
    if len == 0 {
        return 0;
    }
    let p = SEQ_POS.fetch_add(1, AtOrd::SeqCst);
    SEQ[if p < len { p } else { len - 1 }].load(AtOrd::SeqCst)
}

extern "C" fn cnt_stir() {
    STIR_CALLS.fetch_add(1, AtOrd::SeqCst);
}

fn buf_reset(seed: u64) {
    BUF_STREAM.store(seed | 1, AtOrd::SeqCst);
    BUF_CALLS.store(0, AtOrd::SeqCst);
    BUF_BYTES.store(0, AtOrd::SeqCst);
}

extern "C" fn cnt_buf(buf: *mut c_void, size: usize) {
    BUF_CALLS.fetch_add(1, AtOrd::SeqCst);
    BUF_BYTES.fetch_add(size, AtOrd::SeqCst);
    if buf.is_null() || size == 0 {
        return;
    }
    let out = unsafe { std::slice::from_raw_parts_mut(buf as *mut u8, size) };
    let mut i = 0usize;
    while i < size {
        // splitmix64
        let mut z = BUF_STREAM
            .fetch_add(0x9E37_79B9_7F4A_7C15, AtOrd::SeqCst)
            .wrapping_add(0x9E37_79B9_7F4A_7C15);
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        let v = (z ^ (z >> 31)).to_le_bytes();
        let n = core::cmp::min(8, size - i);
        out[i..i + n].copy_from_slice(&v[..n]);
        i += n;
    }
}

extern "C" fn cnt_close() -> c_int {
    CLOSE_CALLS.fetch_add(1, AtOrd::SeqCst);
    0
}

extern "C" fn hook_uniform(upper_bound: u32) -> u32 {
    // Deliberately returns `upper_bound` verbatim: a value the default path
    // could never produce, proving that the hook is what ran.
    upper_bound
}

/// All hooks present, `.uniform == NULL` (like both shipped implementations),
/// so the library's own rejection-sampling loop runs.
static IMPL_FULL: RandombytesImpl = RandombytesImpl {
    implementation_name: Some(cnt_name),
    random: Some(cnt_random),
    stir: Some(cnt_stir),
    uniform: None,
    buf: Some(cnt_buf),
    close: Some(cnt_close),
};

/// Row 910/646: a non-NULL `.uniform` hook must be delegated to
/// unconditionally.
static IMPL_UNIFORM_HOOK: RandombytesImpl = RandombytesImpl {
    implementation_name: Some(cnt_name),
    random: Some(cnt_random),
    stir: Some(cnt_stir),
    uniform: Some(hook_uniform),
    buf: Some(cnt_buf),
    close: Some(cnt_close),
};

/// Row 915: only the three "required" hooks; `stir`/`uniform`/`close` NULL.
static IMPL_MINIMAL: RandombytesImpl = RandombytesImpl {
    implementation_name: Some(cnt_name),
    random: Some(cnt_random),
    stir: None,
    uniform: None,
    buf: Some(cnt_buf),
    close: None,
};

// ===========================================================================
// helpers
// ===========================================================================
/// Address of an exported DATA object (`randombytes_*_implementation`).
/// `Library::get` type-checks the requested type against a pointer's size, so
/// the struct itself cannot be requested; take the raw symbol address instead.
unsafe fn impl_addr(lib: &'static Library, name: &str) -> *const RandombytesImpl {
    let mut n = name.as_bytes().to_vec();
    n.push(0);
    let s: Symbol<'static, *mut c_void> = lib
        .get(&n)
        .unwrap_or_else(|e| panic!("data symbol {name} missing: {e}"));
    let p = s.into_raw().into_raw() as *const RandombytesImpl;
    assert!(!p.is_null(), "data symbol {name} resolved to NULL");
    p
}

unsafe fn set_impl(lib: &'static Library, p: *const RandombytesImpl) -> c_int {
    let f: Symbol<SetImpl> = lib.get(b"randombytes_set_implementation\0").unwrap();
    f(p)
}

unsafe fn set_impl_both(p: *const RandombytesImpl) {
    for l in [libs().c, libs().rs] {
        eq_i32("randombytes_set_implementation", 0, set_impl(l, p));
    }
}

/// Install the harness' shared deterministic implementation in both libraries,
/// unconditionally (not just once), so tests are order-independent.
unsafe fn use_det() {
    install_det_random();
    set_impl_both(&DET_IMPL as *const RandombytesImpl);
}

unsafe fn name_of(lib: &'static Library) -> String {
    let f: Symbol<NameGet> = lib.get(b"randombytes_implementation_name\0").unwrap();
    CStr::from_ptr(f()).to_string_lossy().into_owned()
}

fn ipv4_mapped(a: u8, b: u8, c: u8, d: u8) -> [u8; 16] {
    let mut ip = [0u8; 16];
    ip[10] = 0xff;
    ip[11] = 0xff;
    ip[12] = a;
    ip[13] = b;
    ip[14] = c;
    ip[15] = d;
    ip
}

/// `2001:0db8::1`
const IPV6_DOC: [u8; 16] = [
    0x20, 0x01, 0x0d, 0xb8, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0x01,
];
/// `::1`
const IPV6_LOOPBACK: [u8; 16] = [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1];

fn incr(n: usize) -> Vec<u8> {
    (0..n).map(|i| i as u8).collect()
}

// ===========================================================================
// randombytes_buf_deterministic — CONFIGS rows 889-899
// ===========================================================================

/// Rows 889-897: fixed seeds x every interesting size, byte equality.
#[test]
fn g6_buf_deterministic_seeds_and_sizes() {
    let sizes = [0usize, 1, 31, 32, 33, 63, 64, 65, 1000, 4096];
    let mut seeds: Vec<[u8; 32]> = Vec::new();
    seeds.push([0x00; 32]);
    seeds.push([0xff; 32]);
    let mut s = [0u8; 32];
    s.copy_from_slice(&incr(32));
    seeds.push(s);
    seeds.push([0x42; 32]);
    let mut rng = Rng::new(SEED);
    for _ in 0..12 {
        let mut s = [0u8; 32];
        rng.fill(&mut s);
        seeds.push(s);
    }
    unsafe {
        let (c, r) = pair::<BufDet>("randombytes_buf_deterministic");
        for (si, seed) in seeds.iter().enumerate() {
            for &n in &sizes {
                let mut cb = vec![0xA5u8; n + 16];
                let mut rb = vec![0xA5u8; n + 16];
                c(cb.as_mut_ptr() as *mut c_void, n, seed.as_ptr());
                r(rb.as_mut_ptr() as *mut c_void, n, seed.as_ptr());
                eq_bytes(
                    &format!("randombytes_buf_deterministic seed#{si} size={n}"),
                    &cb,
                    &rb,
                );
                // the trailing guard bytes must never be touched
                assert!(
                    cb[n..].iter().all(|&x| x == 0xA5),
                    "C overran the output buffer (size={n})"
                );
                if n == 0 {
                    // row 889: a zero-length request must leave the buffer alone
                    assert!(cb.iter().all(|&x| x == 0xA5), "size=0 modified the buffer");
                }
            }
        }
    }
}

/// Row 898: the stream has a fixed nonce and a counter starting at 0, so a
/// shorter request must be a byte-exact prefix of a longer one.
#[test]
fn g6_buf_deterministic_prefix_property() {
    unsafe {
        let (c, r) = pair::<BufDet>("randombytes_buf_deterministic");
        let mut rng = Rng::new(SEED ^ 0x898);
        for _ in 0..8 {
            let mut seed = [0u8; 32];
            rng.fill(&mut seed);
            let long_n = 4096usize;
            let mut cl = vec![0u8; long_n];
            let mut rl = vec![0u8; long_n];
            c(cl.as_mut_ptr() as *mut c_void, long_n, seed.as_ptr());
            r(rl.as_mut_ptr() as *mut c_void, long_n, seed.as_ptr());
            eq_bytes("buf_deterministic long", &cl, &rl);
            for &n in &[0usize, 1, 17, 31, 32, 33, 63, 64, 65, 127, 128, 129, 1000] {
                let mut cs = vec![0u8; n];
                let mut rs = vec![0u8; n];
                c(cs.as_mut_ptr() as *mut c_void, n, seed.as_ptr());
                r(rs.as_mut_ptr() as *mut c_void, n, seed.as_ptr());
                eq_bytes(&format!("buf_deterministic short n={n}"), &cs, &rs);
                assert_eq!(&cs[..], &cl[..n], "C prefix property broken at n={n}");
                assert_eq!(&rs[..], &rl[..n], "Rust prefix property broken at n={n}");
            }
        }
    }
}

/// Row 899: seeds one bit apart must give different (but identical across the
/// two libraries) output.
#[test]
fn g6_buf_deterministic_one_bit_seed_difference() {
    unsafe {
        let (c, r) = pair::<BufDet>("randombytes_buf_deterministic");
        let base = [0x5au8; 32];
        let mut c0 = [0u8; 64];
        let mut r0 = [0u8; 64];
        c(c0.as_mut_ptr() as *mut c_void, 64, base.as_ptr());
        r(r0.as_mut_ptr() as *mut c_void, 64, base.as_ptr());
        eq_bytes("buf_deterministic base", &c0, &r0);
        for byte in 0..32usize {
            for bit in [0u32, 3, 7] {
                let mut seed = base;
                seed[byte] ^= 1u8 << bit;
                let mut c1 = [0u8; 64];
                let mut r1 = [0u8; 64];
                c(c1.as_mut_ptr() as *mut c_void, 64, seed.as_ptr());
                r(r1.as_mut_ptr() as *mut c_void, 64, seed.as_ptr());
                eq_bytes(&format!("buf_deterministic flip {byte}:{bit}"), &c1, &r1);
                assert_ne!(c1, c0, "C: one-bit seed change produced identical output");
                assert_ne!(r1, r0, "Rust: one-bit seed change produced identical output");
            }
        }
    }
}

/// Row 900.
#[test]
fn g6_randombytes_seedbytes() {
    unsafe {
        let (c, r) = pair::<SizeGet>("randombytes_seedbytes");
        let a = c();
        let b = r();
        assert_eq!(a, 32, "C randombytes_seedbytes");
        assert_eq!(a, b, "randombytes_seedbytes differs (C={a}, Rust={b})");
    }
}

// ===========================================================================
// randombytes_uniform — CONFIGS rows 901-910
// ===========================================================================

/// Rows 901-909: the default rejection-sampling path, made byte-comparable by
/// installing one deterministic `random()` sequence in both libraries and
/// rewinding it between the C and the Rust call.
#[test]
fn g6_uniform_default_path() {
    unsafe {
        set_impl_both(&IMPL_FULL as *const RandombytesImpl);
        let (c, r) = pair::<Uniform>("randombytes_uniform");
        let bounds: [u32; 16] = [
            0,
            1,
            2,
            3,
            10,
            255,
            256,
            257,
            1000,
            0x7fff_ffff,
            0x8000_0000,
            0x8000_0001,
            0xffff_fffe,
            0xffff_ffff,
            0x0001_0000,
            0x0000_ffff,
        ];
        let mut rng = Rng::new(SEED ^ 0x901);
        for &ub in &bounds {
            for trial in 0..24 {
                // A pseudorandom feed; the same for C and for Rust.
                let feed: Vec<u32> = (0..8).map(|_| rng.next_u32()).collect();
                seq_set(&feed);
                let a = c(ub);
                let ac = RAND_CALLS.load(AtOrd::SeqCst);
                seq_rewind();
                let b = r(ub);
                let bc = RAND_CALLS.load(AtOrd::SeqCst);
                assert_eq!(
                    a, b,
                    "randombytes_uniform({ub}) trial {trial}: C={a} Rust={b}"
                );
                assert_eq!(
                    ac, bc,
                    "randombytes_uniform({ub}): random() call count differs (C={ac}, Rust={bc})"
                );
                if ub < 2 {
                    // rows 901/902: the `< 2` guard consumes no randomness
                    assert_eq!(a, 0, "randombytes_uniform({ub}) must return 0");
                    assert_eq!(ac, 0, "randombytes_uniform({ub}) consumed randomness");
                } else {
                    assert!(a < ub, "randombytes_uniform({ub}) returned {a}");
                    assert!(ac >= 1, "randombytes_uniform({ub}) drew no randomness");
                }
            }
        }
        use_det();
    }
}

/// Rows 903-909: the exact `min = (1 + ~upper_bound) % upper_bound`
/// rejection-sampling vectors from the table.
#[test]
fn g6_uniform_exact_rejection_vectors() {
    unsafe {
        set_impl_both(&IMPL_FULL as *const RandombytesImpl);
        let (c, r) = pair::<Uniform>("randombytes_uniform");
        // (upper_bound, feed, expected result, expected random() calls)
        let cases: &[(u32, &[u32], u32, usize)] = &[
            // row 903: min = 0, no rejection
            (2, &[0xdead_beef], 1, 1),
            (2, &[0xdead_beee], 0, 1),
            // row 904: min = 1 -> r = 0 rejected, r = 7 -> 1
            (3, &[0, 7], 1, 2),
            // row 905: min = 6 -> r = 3 rejected, r = 25 -> 5
            (10, &[3, 25], 5, 2),
            // row 906: min = 0x7ffffffe -> r = 0 rejected, r = 0x80000002 -> 1
            (0x8000_0001, &[0, 0x8000_0002], 1, 2),
            // row 907: min = 1 -> r = 0 rejected, r = 1 -> 1
            (0xffff_ffff, &[0, 1], 1, 2),
            // row 908: power of two, min = 0, never rejects
            (0x8000_0000, &[0xdead_beef], 0x5ead_beef, 1),
            // row 909: min = 0, result = r & 0xff
            (256, &[0xdead_beef], 0xef, 1),
        ];
        for (ub, feed, want, want_calls) in cases {
            seq_set(feed);
            let a = c(*ub);
            let ac = RAND_CALLS.load(AtOrd::SeqCst);
            seq_rewind();
            let b = r(*ub);
            let bc = RAND_CALLS.load(AtOrd::SeqCst);
            assert_eq!(a, b, "uniform({ub}) feed={feed:?}: C={a} Rust={b}");
            assert_eq!(a, *want, "uniform({ub}) feed={feed:?}: expected {want}, C got {a}");
            assert_eq!(ac, *want_calls, "uniform({ub}): C made {ac} random() calls");
            assert_eq!(bc, *want_calls, "uniform({ub}): Rust made {bc} random() calls");
        }
        use_det();
    }
}

/// Row 910: a non-NULL `.uniform` hook bypasses both the `< 2` guard and the
/// rejection loop.
#[test]
fn g6_uniform_delegates_to_hook() {
    unsafe {
        set_impl_both(&IMPL_UNIFORM_HOOK as *const RandombytesImpl);
        let (c, r) = pair::<Uniform>("randombytes_uniform");
        for ub in [0u32, 1, 2, 3, 255, 256, 0x8000_0000, 0xffff_ffff] {
            seq_set(&[0xdead_beef]);
            let a = c(ub);
            let ac = RAND_CALLS.load(AtOrd::SeqCst);
            seq_rewind();
            let b = r(ub);
            let bc = RAND_CALLS.load(AtOrd::SeqCst);
            assert_eq!(a, b, "hooked uniform({ub}): C={a} Rust={b}");
            assert_eq!(a, ub, "hooked uniform({ub}) must return the hook's value");
            assert_eq!(ac, 0, "hooked uniform({ub}) still called random() in C");
            assert_eq!(bc, 0, "hooked uniform({ub}) still called random() in Rust");
        }
        use_det();
    }
}

// ===========================================================================
// implementation management — CONFIGS rows 911-916, 928-930
// ===========================================================================

/// Rows 911 + 916: after `randombytes_set_implementation(NULL)` the next call
/// re-runs `randombytes_init_if_needed()` and reverts to the built-in default,
/// which on this (non-Emscripten) build is `sysrandom`.
///
/// NOTE the pristine "nothing ever set" state cannot be observed in-process
/// because `sodium_init()` already calls `randombytes_stir()`; the equivalent
/// fresh-process check is `zz_abort_child` case `default_impl_name`.
#[test]
fn g6_set_implementation_null_reverts_to_default() {
    unsafe {
        for l in [libs().c, libs().rs] {
            eq_i32("set_implementation(NULL)", 0, set_impl(l, ptr::null()));
        }
        let cn = name_of(libs().c);
        let rn = name_of(libs().rs);
        assert_eq!(cn, "sysrandom", "C default implementation name");
        assert_eq!(cn, rn, "default implementation name differs");
        // and the fresh-process variant, compared across the two libraries
        let t = diff_abort_case("default_impl_name");
        assert_eq!(
            t.code,
            Some(0),
            "fresh-process default implementation name mismatch: {t:?}"
        );
        use_det();
    }
}

/// Rows 912-914: switching between the two shipped implementations.
#[test]
fn g6_set_implementation_switching() {
    unsafe {
        let l = libs();
        let internal = [impl_addr(l.c, "randombytes_internal_implementation"),
                        impl_addr(l.rs, "randombytes_internal_implementation")];
        let sysrandom = [impl_addr(l.c, "randombytes_sysrandom_implementation"),
                         impl_addr(l.rs, "randombytes_sysrandom_implementation")];
        let libv = [l.c, l.rs];
        for round in 0..3 {
            for (want, addrs) in [("internal", &internal), ("sysrandom", &sysrandom)] {
                let mut names = Vec::new();
                for i in 0..2 {
                    eq_i32(
                        &format!("set_implementation({want}) round {round}"),
                        0,
                        set_impl(libv[i], addrs[i]),
                    );
                    names.push(name_of(libv[i]));
                }
                assert_eq!(names[0], want, "C implementation_name after switch");
                assert_eq!(
                    names[0], names[1],
                    "implementation_name differs (C={}, Rust={})",
                    names[0], names[1]
                );
            }
        }
        use_det();
    }
}

/// Row 915: a fully custom implementation with `stir`/`uniform`/`close` NULL.
#[test]
fn g6_set_implementation_custom_minimal() {
    unsafe {
        set_impl_both(&IMPL_MINIMAL as *const RandombytesImpl);
        assert_eq!(name_of(libs().c), "det");
        assert_eq!(name_of(libs().rs), "det");
        let (cs, rs_) = pair::<Stir>("randombytes_stir");
        STIR_CALLS.store(0, AtOrd::SeqCst);
        cs();
        rs_();
        assert_eq!(
            STIR_CALLS.load(AtOrd::SeqCst),
            0,
            "stir == NULL must be a no-op"
        );
        let (cc, rc) = pair::<Close>("randombytes_close");
        CLOSE_CALLS.store(0, AtOrd::SeqCst);
        let a = cc();
        let b = rc();
        eq_i32("close with .close == NULL", a, b);
        assert_eq!(a, 0, "close with .close == NULL must return 0");
        assert_eq!(CLOSE_CALLS.load(AtOrd::SeqCst), 0);
        use_det();
    }
}

/// Rows 928-930: the two shipped implementation vtables must be
/// field-for-field equivalent, and `randombytes_salsa20_implementation` is a
/// preprocessor alias only (no such dynamic symbol).
#[test]
fn g6_shipped_implementation_structs() {
    unsafe {
        let l = libs();
        for (name, want) in [
            ("randombytes_internal_implementation", "internal"),
            ("randombytes_sysrandom_implementation", "sysrandom"),
        ] {
            for (tag, lib) in [("C", l.c), ("Rust", l.rs)] {
                let p = impl_addr(lib, name);
                let s = &*p;
                let n = CStr::from_ptr((s.implementation_name.expect(
                    "implementation_name must be non-NULL",
                ))())
                .to_string_lossy()
                .into_owned();
                assert_eq!(n, want, "{tag} {name}.implementation_name");
                assert!(s.random.is_some(), "{tag} {name}.random must be non-NULL");
                assert!(s.stir.is_some(), "{tag} {name}.stir must be non-NULL");
                assert!(
                    s.uniform.is_none(),
                    "{tag} {name}.uniform must be NULL (so the default \
                     rejection-sampling loop runs)"
                );
                assert!(s.buf.is_some(), "{tag} {name}.buf must be non-NULL");
                assert!(s.close.is_some(), "{tag} {name}.close must be non-NULL");
            }
        }
        // row 930: `randombytes_salsa20_implementation` is a `#define` in
        // randombytes_internal_random.h, not an exported object.
        assert!(
            !has_sym("randombytes_salsa20_implementation"),
            "randombytes_salsa20_implementation is a compat macro, not a symbol"
        );
    }
}

// ===========================================================================
// randombytes_buf / _random / NaCl alias — CONFIGS rows 917-920
// ===========================================================================

/// Rows 917-918.
#[test]
fn g6_randombytes_buf_sizes() {
    unsafe {
        set_impl_both(&IMPL_FULL as *const RandombytesImpl);
        let (c, r) = pair::<Buf>("randombytes_buf");
        // row 917: size == 0 must not call the hook at all
        let mut probe = [0xEEu8; 8];
        buf_reset(7);
        c(probe.as_mut_ptr() as *mut c_void, 0);
        assert_eq!(BUF_CALLS.load(AtOrd::SeqCst), 0, "C called buf for size 0");
        r(probe.as_mut_ptr() as *mut c_void, 0);
        assert_eq!(BUF_CALLS.load(AtOrd::SeqCst), 0, "Rust called buf for size 0");
        assert_eq!(probe, [0xEEu8; 8], "size 0 modified the buffer");
        // row 918
        for n in [1usize, 31, 32, 33, 63, 64, 65, 256, 257, 1000, 4096] {
            let mut cb = vec![0u8; n];
            let mut rb = vec![0u8; n];
            buf_reset(0x1234_5678 ^ n as u64);
            c(cb.as_mut_ptr() as *mut c_void, n);
            let calls_c = BUF_CALLS.load(AtOrd::SeqCst);
            let bytes_c = BUF_BYTES.load(AtOrd::SeqCst);
            buf_reset(0x1234_5678 ^ n as u64);
            r(rb.as_mut_ptr() as *mut c_void, n);
            eq_bytes(&format!("randombytes_buf n={n}"), &cb, &rb);
            assert_eq!(calls_c, BUF_CALLS.load(AtOrd::SeqCst));
            assert_eq!(bytes_c, BUF_BYTES.load(AtOrd::SeqCst));
            assert_eq!(bytes_c, n, "buf hook received the wrong size");
        }
        use_det();
    }
}

/// Row 919: `randombytes_random` is a straight pass-through of
/// `implementation->random()`.
#[test]
fn g6_randombytes_random_passthrough() {
    unsafe {
        set_impl_both(&IMPL_FULL as *const RandombytesImpl);
        let (c, r) = pair::<Random>("randombytes_random");
        let mut rng = Rng::new(SEED ^ 0x919);
        for _ in 0..64 {
            let v = rng.next_u32();
            seq_set(&[v]);
            let a = c();
            seq_rewind();
            let b = r();
            assert_eq!(a, v, "C randombytes_random did not pass the hook through");
            assert_eq!(a, b, "randombytes_random differs (C={a}, Rust={b})");
            assert_eq!(RAND_CALLS.load(AtOrd::SeqCst), 1);
        }
        use_det();
    }
}

/// Row 920: the NaCl-compat `randombytes(buf, len)` entry point must be
/// identical to `randombytes_buf(buf, (size_t) len)`.
#[test]
fn g6_nacl_randombytes_alias() {
    unsafe {
        set_impl_both(&IMPL_FULL as *const RandombytesImpl);
        let (cn, rn) = pair::<NaclRb>("randombytes");
        let (cb_, rb_) = pair::<Buf>("randombytes_buf");
        for n in [0u64, 1, 8, 31, 32, 64, 65, 1000, 4096] {
            let mut a = vec![0u8; n as usize];
            let mut b = vec![0u8; n as usize];
            let mut d = vec![0u8; n as usize];
            let mut e = vec![0u8; n as usize];
            buf_reset(0xABCD ^ n);
            cn(a.as_mut_ptr(), n);
            buf_reset(0xABCD ^ n);
            rn(b.as_mut_ptr(), n);
            buf_reset(0xABCD ^ n);
            cb_(d.as_mut_ptr() as *mut c_void, n as usize);
            buf_reset(0xABCD ^ n);
            rb_(e.as_mut_ptr() as *mut c_void, n as usize);
            eq_bytes(&format!("randombytes(len={n})"), &a, &b);
            eq_bytes(&format!("randombytes vs randombytes_buf C len={n}"), &a, &d);
            eq_bytes(&format!("randombytes vs randombytes_buf Rust len={n}"), &b, &e);
        }
        use_det();
    }
}

// ===========================================================================
// stir / close — CONFIGS rows 921-927
// ===========================================================================

/// Rows 921-927: `randombytes_stir` / `randombytes_close` on both shipped
/// implementations plus the custom one, with the documented idempotency.
#[test]
fn g6_stir_and_close_shipped_implementations() {
    unsafe {
        let l = libs();
        let libv = [l.c, l.rs];
        for name in [
            "randombytes_sysrandom_implementation",
            "randombytes_internal_implementation",
        ] {
            let mut rets: Vec<(c_int, c_int)> = Vec::new();
            for i in 0..2 {
                let p = impl_addr(libv[i], name);
                eq_i32("set_implementation", 0, set_impl(libv[i], p));
                let stir: Symbol<Stir> = libv[i].get(b"randombytes_stir\0").unwrap();
                let close: Symbol<Close> = libv[i].get(b"randombytes_close\0").unwrap();
                // rows 921/922: stirring twice in a row must be safe
                stir();
                stir();
                // rows 924/925: close is 0 and idempotent once stirred
                let r1 = close();
                let r2 = close();
                rets.push((r1, r2));
                // row 927: a fresh buf request transparently re-stirs
                let f: Symbol<Buf> = libv[i].get(b"randombytes_buf\0").unwrap();
                let mut b1 = [0u8; 64];
                let mut b2 = [0u8; 64];
                f(b1.as_mut_ptr() as *mut c_void, 64);
                f(b2.as_mut_ptr() as *mut c_void, 64);
                assert_ne!(b1, b2, "{name}: repeated randombytes_buf output repeated");
                assert!(
                    b1.iter().any(|&x| x != 0),
                    "{name}: randombytes_buf produced all zeros"
                );
            }
            assert_eq!(
                rets[0], rets[1],
                "{name}: randombytes_close return values differ (C={:?}, Rust={:?})",
                rets[0], rets[1]
            );
            assert_eq!(
                rets[0].0, 0,
                "{name}: randombytes_close should return 0 once stirred"
            );
            assert_eq!(rets[0].1, 0, "{name}: randombytes_close is not idempotent");
        }
        // row 923/926: `.stir == NULL` / `.close == NULL`
        set_impl_both(&IMPL_MINIMAL as *const RandombytesImpl);
        let (cs, rs_) = pair::<Stir>("randombytes_stir");
        let (cc, rc) = pair::<Close>("randombytes_close");
        STIR_CALLS.store(0, AtOrd::SeqCst);
        cs();
        rs_();
        assert_eq!(STIR_CALLS.load(AtOrd::SeqCst), 0);
        eq_i32("close(.close==NULL)", cc(), rc());
        assert_eq!(cc(), 0);
        use_det();
    }
}

/// Rows 931-932: the `internal` backend's ChaCha20 pool. The output is real
/// entropy, so only the structural invariants are comparable: no repeats, no
/// abort, and one forced `rnd32` refill (480 bytes of pool / 4 = 120 draws).
#[test]
fn g6_internal_pool_refill_and_ratchet() {
    unsafe {
        let l = libs();
        let libv = [l.c, l.rs];
        for i in 0..2 {
            let p = impl_addr(libv[i], "randombytes_internal_implementation");
            eq_i32("set_implementation(internal)", 0, set_impl(libv[i], p));
            let stir: Symbol<Stir> = libv[i].get(b"randombytes_stir\0").unwrap();
            stir();
            let buf: Symbol<Buf> = libv[i].get(b"randombytes_buf\0").unwrap();
            let mut seen: Vec<Vec<u8>> = Vec::new();
            for &n in &[1usize, 32, 64, 1024, 1, 32, 64, 1024] {
                let mut b = vec![0u8; n];
                buf(b.as_mut_ptr() as *mut c_void, n);
                // Statistical smoke check on REAL entropy: only apply it where
                // a collision is impossible in practice. Two independent 1-byte
                // draws collide 1 time in 256, which would make this flaky.
                if n >= 8 {
                    assert!(
                        !seen.contains(&b),
                        "internal buf repeated its output (n={n})"
                    );
                }
                seen.push(b);
            }
            // row 932: 120 draws span exactly one refill boundary
            let rnd: Symbol<Random> = libv[i].get(b"randombytes_random\0").unwrap();
            let mut vals = std::collections::HashSet::new();
            for _ in 0..300 {
                vals.insert(rnd());
            }
            assert!(
                vals.len() > 250,
                "internal randombytes_random produced too many duplicates: {}",
                vals.len()
            );
        }
        use_det();
    }
}

// ===========================================================================
// ML-KEM-768 — CONFIGS rows 933-948
// ===========================================================================

fn mlkem_seeds() -> Vec<[u8; MLK_SEED]> {
    let mut v: Vec<[u8; MLK_SEED]> = Vec::new();
    v.push([0x00; MLK_SEED]);
    v.push([0xff; MLK_SEED]);
    let mut s = [0u8; MLK_SEED];
    s.copy_from_slice(&incr(MLK_SEED));
    v.push(s);
    v.push([0x42; MLK_SEED]);
    // "ACVP-style" arbitrary fixed KAT seeds; no published vector is available
    // offline, so the assertion is C-vs-Rust byte equality (row 936).
    let mut rng = Rng::new(0x4D4C_4B45_4D37_3638);
    for _ in 0..16 {
        let mut s = [0u8; MLK_SEED];
        rng.fill(&mut s);
        v.push(s);
    }
    v
}

/// Rows 933-936: `crypto_kem_mlkem768_seed_keypair` is fully deterministic;
/// the secret-key layout is checked against the library's own SHA3-256.
#[test]
fn g6_mlkem768_seed_keypair() {
    unsafe {
        let (c, r) = pair::<SeedKp>("crypto_kem_mlkem768_seed_keypair");
        let (csha, rsha) = pair::<Sha3256>("crypto_hash_sha3256");
        for (i, seed) in mlkem_seeds().iter().enumerate() {
            let mut cpk = vec![0u8; MLK_PK];
            let mut csk = vec![0u8; MLK_SK];
            let mut rpk = vec![0u8; MLK_PK];
            let mut rsk = vec![0u8; MLK_SK];
            let a = c(cpk.as_mut_ptr(), csk.as_mut_ptr(), seed.as_ptr());
            let b = r(rpk.as_mut_ptr(), rsk.as_mut_ptr(), seed.as_ptr());
            eq_i32(&format!("mlkem768_seed_keypair#{i} ret"), a, b);
            assert_eq!(a, 0, "mlkem768_seed_keypair must return 0");
            eq_bytes(&format!("mlkem768_seed_keypair#{i} pk"), &cpk, &rpk);
            eq_bytes(&format!("mlkem768_seed_keypair#{i} sk"), &csk, &rsk);
            // sk layout: skpv ‖ pk ‖ SHA3-256(pk) ‖ z
            assert_eq!(&csk[1152..2336], &cpk[..], "sk[1152..2336] != pk (C)");
            assert_eq!(&rsk[1152..2336], &rpk[..], "sk[1152..2336] != pk (Rust)");
            let mut ch = [0u8; 32];
            let mut rh = [0u8; 32];
            csha(ch.as_mut_ptr(), cpk.as_ptr(), MLK_PK as u64);
            rsha(rh.as_mut_ptr(), rpk.as_ptr(), MLK_PK as u64);
            eq_bytes("sha3256(pk)", &ch, &rh);
            assert_eq!(&csk[2336..2368], &ch[..], "sk hpk != SHA3-256(pk)");
            assert_eq!(&csk[2368..2400], &seed[32..64], "sk z != seed[32..64]");
            assert_eq!(&rsk[2368..2400], &seed[32..64], "Rust sk z != seed[32..64]");
        }
    }
}

/// Rows 937-939: `crypto_kem_mlkem768_enc_deterministic`.
#[test]
fn g6_mlkem768_enc_deterministic() {
    unsafe {
        let (ckp, rkp) = pair::<SeedKp>("crypto_kem_mlkem768_seed_keypair");
        let (cenc, renc) = pair::<EncDet>("crypto_kem_mlkem768_enc_deterministic");
        let enc_seeds: Vec<[u8; 32]> = {
            let mut v = vec![[0x00u8; 32], [0xffu8; 32], [0xaau8; 32]];
            let mut s = [0u8; 32];
            s.copy_from_slice(&incr(32));
            v.push(s);
            let mut rng = Rng::new(SEED ^ 0x937);
            for _ in 0..3 {
                let mut s = [0u8; 32];
                rng.fill(&mut s);
                v.push(s);
            }
            v
        };
        for (ki, kseed) in mlkem_seeds().iter().take(4).enumerate() {
            let mut cpk = vec![0u8; MLK_PK];
            let mut csk = vec![0u8; MLK_SK];
            let mut rpk = vec![0u8; MLK_PK];
            let mut rsk = vec![0u8; MLK_SK];
            ckp(cpk.as_mut_ptr(), csk.as_mut_ptr(), kseed.as_ptr());
            rkp(rpk.as_mut_ptr(), rsk.as_mut_ptr(), kseed.as_ptr());
            eq_bytes("pk", &cpk, &rpk);
            for (ei, eseed) in enc_seeds.iter().enumerate() {
                let mut cct = vec![0u8; MLK_CT];
                let mut css = vec![0u8; SS];
                let mut rct = vec![0u8; MLK_CT];
                let mut rss = vec![0u8; SS];
                let a = cenc(cct.as_mut_ptr(), css.as_mut_ptr(), cpk.as_ptr(), eseed.as_ptr());
                let b = renc(rct.as_mut_ptr(), rss.as_mut_ptr(), rpk.as_ptr(), eseed.as_ptr());
                eq_i32(&format!("mlkem enc_det k{ki}/e{ei} ret"), a, b);
                assert_eq!(a, 0);
                eq_bytes(&format!("mlkem enc_det k{ki}/e{ei} ct"), &cct, &rct);
                eq_bytes(&format!("mlkem enc_det k{ki}/e{ei} ss"), &css, &rss);
            }
        }
    }
}

/// Rows 940-941: derandomised encapsulate/decapsulate round trip.
#[test]
fn g6_mlkem768_deterministic_roundtrip() {
    unsafe {
        let (ckp, rkp) = pair::<SeedKp>("crypto_kem_mlkem768_seed_keypair");
        let (cenc, renc) = pair::<EncDet>("crypto_kem_mlkem768_enc_deterministic");
        let (cdec, rdec) = pair::<Dec>("crypto_kem_mlkem768_dec");
        let mut rng = Rng::new(SEED ^ 0x940);
        for i in 0..5 {
            let mut kseed = [0u8; MLK_SEED];
            let mut eseed = [0u8; 32];
            rng.fill(&mut kseed);
            rng.fill(&mut eseed);
            let mut cpk = vec![0u8; MLK_PK];
            let mut csk = vec![0u8; MLK_SK];
            let mut rpk = vec![0u8; MLK_PK];
            let mut rsk = vec![0u8; MLK_SK];
            ckp(cpk.as_mut_ptr(), csk.as_mut_ptr(), kseed.as_ptr());
            rkp(rpk.as_mut_ptr(), rsk.as_mut_ptr(), kseed.as_ptr());
            let mut cct = vec![0u8; MLK_CT];
            let mut css = vec![0u8; SS];
            let mut rct = vec![0u8; MLK_CT];
            let mut rss = vec![0u8; SS];
            cenc(cct.as_mut_ptr(), css.as_mut_ptr(), cpk.as_ptr(), eseed.as_ptr());
            renc(rct.as_mut_ptr(), rss.as_mut_ptr(), rpk.as_ptr(), eseed.as_ptr());
            eq_bytes(&format!("roundtrip#{i} ct"), &cct, &rct);
            eq_bytes(&format!("roundtrip#{i} ss_enc"), &css, &rss);
            let mut css2 = vec![0u8; SS];
            let mut rss2 = vec![0u8; SS];
            let a = cdec(css2.as_mut_ptr(), cct.as_ptr(), csk.as_ptr());
            let b = rdec(rss2.as_mut_ptr(), rct.as_ptr(), rsk.as_ptr());
            eq_i32(&format!("roundtrip#{i} dec ret"), a, b);
            assert_eq!(a, 0, "mlkem768_dec must return 0");
            eq_bytes(&format!("roundtrip#{i} ss_dec"), &css2, &rss2);
            assert_eq!(css, css2, "C enc/dec shared secrets disagree");
            assert_eq!(rss, rss2, "Rust enc/dec shared secrets disagree");
        }
    }
}

/// Rows 942-946: implicit rejection. `mlkem768_ref_dec` NEVER returns -1; a
/// corrupted ciphertext (or a tampered `hpk`) yields the deterministic
/// pseudorandom secret `SHAKE256(z ‖ ct)[0..32]`, which must be byte-identical
/// in C and Rust.
#[test]
fn g6_mlkem768_implicit_rejection() {
    unsafe {
        let (ckp, rkp) = pair::<SeedKp>("crypto_kem_mlkem768_seed_keypair");
        let (cenc, renc) = pair::<EncDet>("crypto_kem_mlkem768_enc_deterministic");
        let (cdec, rdec) = pair::<Dec>("crypto_kem_mlkem768_dec");
        let kseed = [0x11u8; MLK_SEED];
        let eseed = [0x22u8; 32];
        let mut cpk = vec![0u8; MLK_PK];
        let mut csk = vec![0u8; MLK_SK];
        let mut rpk = vec![0u8; MLK_PK];
        let mut rsk = vec![0u8; MLK_SK];
        ckp(cpk.as_mut_ptr(), csk.as_mut_ptr(), kseed.as_ptr());
        rkp(rpk.as_mut_ptr(), rsk.as_mut_ptr(), kseed.as_ptr());
        let mut ct = vec![0u8; MLK_CT];
        let mut ss_honest = vec![0u8; SS];
        cenc(ct.as_mut_ptr(), ss_honest.as_mut_ptr(), cpk.as_ptr(), eseed.as_ptr());
        {
            let mut rct = vec![0u8; MLK_CT];
            let mut rss = vec![0u8; SS];
            renc(rct.as_mut_ptr(), rss.as_mut_ptr(), rpk.as_ptr(), eseed.as_ptr());
            eq_bytes("implicit-rejection base ct", &ct, &rct);
            eq_bytes("implicit-rejection base ss", &ss_honest, &rss);
        }

        let mut secrets: Vec<Vec<u8>> = Vec::new();
        let mut rng = Rng::new(SEED ^ 0x942);
        let mut positions: Vec<usize> = vec![0, 1, 2, 100, 543, 544, 1000, 1086, 1087];
        for _ in 0..8 {
            positions.push(rng.below(MLK_CT));
        }
        for (i, &pos) in positions.iter().enumerate() {
            let mut bad = ct.clone();
            bad[pos] ^= if i % 2 == 0 { 0x01 } else { 0xff };
            let mut css = vec![0u8; SS];
            let mut rss = vec![0u8; SS];
            let a = cdec(css.as_mut_ptr(), bad.as_ptr(), csk.as_ptr());
            let b = rdec(rss.as_mut_ptr(), bad.as_ptr(), rsk.as_ptr());
            eq_i32(&format!("dec(corrupt@{pos}) ret"), a, b);
            assert_eq!(a, 0, "mlkem768_dec must return 0 even for a bad ct");
            eq_bytes(&format!("dec(corrupt@{pos}) ss"), &css, &rss);
            assert_ne!(
                css, ss_honest,
                "implicit rejection at {pos} returned the honest secret"
            );
            secrets.push(css);
        }
        // rows 944/945: degenerate ciphertexts
        for (tag, fill) in [("zeros", 0x00u8), ("ones", 0xffu8)] {
            let bad = vec![fill; MLK_CT];
            let mut css = vec![0u8; SS];
            let mut rss = vec![0u8; SS];
            let a = cdec(css.as_mut_ptr(), bad.as_ptr(), csk.as_ptr());
            let b = rdec(rss.as_mut_ptr(), bad.as_ptr(), rsk.as_ptr());
            eq_i32(&format!("dec(all-{tag}) ret"), a, b);
            assert_eq!(a, 0);
            eq_bytes(&format!("dec(all-{tag}) ss"), &css, &rss);
            secrets.push(css);
        }
        // row 946: zeroed hpk inside an otherwise valid sk (no hash check
        // exists, so this silently becomes an implicit rejection)
        {
            let mut csk2 = csk.clone();
            let mut rsk2 = rsk.clone();
            for i in 2336..2368 {
                csk2[i] = 0;
                rsk2[i] = 0;
            }
            let mut css = vec![0u8; SS];
            let mut rss = vec![0u8; SS];
            let a = cdec(css.as_mut_ptr(), ct.as_ptr(), csk2.as_ptr());
            let b = rdec(rss.as_mut_ptr(), ct.as_ptr(), rsk2.as_ptr());
            eq_i32("dec(zeroed hpk) ret", a, b);
            assert_eq!(a, 0);
            eq_bytes("dec(zeroed hpk) ss", &css, &rss);
            assert_ne!(css, ss_honest, "zeroed hpk still produced the honest secret");
        }
        // every implicit-rejection secret must be distinct
        for i in 0..secrets.len() {
            for j in i + 1..secrets.len() {
                assert_ne!(
                    secrets[i], secrets[j],
                    "two different bad ciphertexts gave the same secret"
                );
            }
        }
    }
}

/// Row 947: `_keypair` + `_enc` + `_dec`. With the harness' deterministic
/// `randombytes` rewound before each call the byte values are comparable too,
/// which is strictly stronger than the ss-agreement invariant.
#[test]
fn g6_mlkem768_random_roundtrip() {
    unsafe {
        use_det();
        let (ckp, rkp) = pair::<Kp>("crypto_kem_mlkem768_keypair");
        let (cenc, renc) = pair::<Enc>("crypto_kem_mlkem768_enc");
        let (cdec, rdec) = pair::<Dec>("crypto_kem_mlkem768_dec");
        for i in 0..4u64 {
            let mut cpk = vec![0u8; MLK_PK];
            let mut csk = vec![0u8; MLK_SK];
            let mut rpk = vec![0u8; MLK_PK];
            let mut rsk = vec![0u8; MLK_SK];
            det_reseed(0x1000 + i);
            let a = ckp(cpk.as_mut_ptr(), csk.as_mut_ptr());
            det_reseed(0x1000 + i);
            let b = rkp(rpk.as_mut_ptr(), rsk.as_mut_ptr());
            eq_i32("mlkem768_keypair ret", a, b);
            assert_eq!(a, 0);
            eq_bytes("mlkem768_keypair pk", &cpk, &rpk);
            eq_bytes("mlkem768_keypair sk", &csk, &rsk);
            let mut cct = vec![0u8; MLK_CT];
            let mut css = vec![0u8; SS];
            let mut rct = vec![0u8; MLK_CT];
            let mut rss = vec![0u8; SS];
            det_reseed(0x2000 + i);
            let a = cenc(cct.as_mut_ptr(), css.as_mut_ptr(), cpk.as_ptr());
            det_reseed(0x2000 + i);
            let b = renc(rct.as_mut_ptr(), rss.as_mut_ptr(), rpk.as_ptr());
            eq_i32("mlkem768_enc ret", a, b);
            assert_eq!(a, 0);
            eq_bytes("mlkem768_enc ct", &cct, &rct);
            eq_bytes("mlkem768_enc ss", &css, &rss);
            let mut css2 = vec![0u8; SS];
            let mut rss2 = vec![0u8; SS];
            eq_i32(
                "mlkem768_dec ret",
                cdec(css2.as_mut_ptr(), cct.as_ptr(), csk.as_ptr()),
                rdec(rss2.as_mut_ptr(), rct.as_ptr(), rsk.as_ptr()),
            );
            eq_bytes("mlkem768_dec ss", &css2, &rss2);
            assert_eq!(css, css2, "C enc/dec disagree");
            assert_eq!(rss, rss2, "Rust enc/dec disagree");
        }
    }
}

/// Row 948.
#[test]
fn g6_mlkem768_getters() {
    unsafe {
        for (name, want) in [
            ("crypto_kem_mlkem768_publickeybytes", 1184usize),
            ("crypto_kem_mlkem768_secretkeybytes", 2400),
            ("crypto_kem_mlkem768_ciphertextbytes", 1088),
            ("crypto_kem_mlkem768_sharedsecretbytes", 32),
            ("crypto_kem_mlkem768_seedbytes", 64),
        ] {
            let (c, r) = pair::<SizeGet>(name);
            let a = c();
            let b = r();
            assert_eq!(a, want, "C {name}");
            assert_eq!(a, b, "{name} differs (C={a}, Rust={b})");
        }
    }
}

// ===========================================================================
// X-Wing — CONFIGS rows 949-962
// ===========================================================================

fn xwing_seeds() -> Vec<[u8; XW_SEED]> {
    let mut v = vec![[0x00u8; 32], [0xffu8; 32]];
    let mut s = [0u8; 32];
    s.copy_from_slice(&incr(32));
    v.push(s);
    v.push([0x5au8; 32]);
    let mut rng = Rng::new(0x5857_494E_4700_0001);
    for _ in 0..12 {
        let mut s = [0u8; 32];
        rng.fill(&mut s);
        v.push(s);
    }
    v
}

/// Rows 949-952: `crypto_kem_xwing_seed_keypair`. `sk` is the seed verbatim,
/// and `pk[1184..1216]` is X25519-base(SHAKE256(seed, 96)[64..96]) — that
/// structural identity is verified using each library's own SHAKE256 and
/// X25519 (no published X-Wing KAT is available offline, so the primary
/// assertion is C-vs-Rust byte equality).
#[test]
fn g6_xwing_seed_keypair() {
    unsafe {
        let (c, r) = pair::<SeedKp>("crypto_kem_xwing_seed_keypair");
        let (cxof, rxof) = pair::<Shake256>("crypto_xof_shake256");
        let (cbase, rbase) = pair::<ScalarmultBase>("crypto_scalarmult_curve25519_base");
        let (cmlk, rmlk) = pair::<SeedKp>("crypto_kem_mlkem768_seed_keypair");
        for (i, seed) in xwing_seeds().iter().enumerate() {
            let mut cpk = vec![0u8; XW_PK];
            let mut csk = vec![0u8; XW_SK];
            let mut rpk = vec![0u8; XW_PK];
            let mut rsk = vec![0u8; XW_SK];
            let a = c(cpk.as_mut_ptr(), csk.as_mut_ptr(), seed.as_ptr());
            let b = r(rpk.as_mut_ptr(), rsk.as_mut_ptr(), seed.as_ptr());
            eq_i32(&format!("xwing_seed_keypair#{i} ret"), a, b);
            assert_eq!(a, 0);
            eq_bytes(&format!("xwing_seed_keypair#{i} pk"), &cpk, &rpk);
            eq_bytes(&format!("xwing_seed_keypair#{i} sk"), &csk, &rsk);
            assert_eq!(&csk[..], &seed[..], "xwing sk must be the seed verbatim");
            // structural check of the 96-byte SHAKE256 expansion split
            for (tag, xof, base, mlk, pk) in [
                ("C", &cxof, &cbase, &cmlk, &cpk),
                ("Rust", &rxof, &rbase, &rmlk, &rpk),
            ] {
                let mut expanded = [0u8; 96];
                xof(expanded.as_mut_ptr(), 96, seed.as_ptr(), XW_SEED as u64);
                let mut x_pk = [0u8; 32];
                base(x_pk.as_mut_ptr(), expanded[64..].as_ptr());
                assert_eq!(
                    &pk[1184..1216],
                    &x_pk[..],
                    "{tag}: pk x25519 part != base*(SHAKE256(seed,96)[64..96])"
                );
                let mut mpk = vec![0u8; MLK_PK];
                let mut msk = vec![0u8; MLK_SK];
                mlk(mpk.as_mut_ptr(), msk.as_mut_ptr(), expanded.as_ptr());
                assert_eq!(
                    &pk[..1184],
                    &mpk[..],
                    "{tag}: pk mlkem part != mlkem768_seed_keypair(expanded[0..64])"
                );
            }
        }
    }
}

/// Rows 953-955: `crypto_kem_xwing_enc_deterministic` takes a 64-byte seed
/// (32 B ML-KEM coins ‖ 32 B ephemeral X25519 scalar) and produces
/// ct = ct_mlkem(1088) ‖ ct_x25519(32).
#[test]
fn g6_xwing_enc_deterministic() {
    unsafe {
        let (ckp, rkp) = pair::<SeedKp>("crypto_kem_xwing_seed_keypair");
        let (cenc, renc) = pair::<EncDet>("crypto_kem_xwing_enc_deterministic");
        let (cmenc, rmenc) = pair::<EncDet>("crypto_kem_mlkem768_enc_deterministic");
        let (cbase, rbase) = pair::<ScalarmultBase>("crypto_scalarmult_curve25519_base");
        let mut eseeds: Vec<[u8; XW_ENC_SEED]> = vec![[0x00; 64], [0xaa; 64], [0xff; 64]];
        let mut s = [0u8; 64];
        s.copy_from_slice(&incr(64));
        eseeds.push(s);
        let mut rng = Rng::new(SEED ^ 0x953);
        for _ in 0..3 {
            let mut s = [0u8; 64];
            rng.fill(&mut s);
            eseeds.push(s);
        }
        for (ki, kseed) in xwing_seeds().iter().take(4).enumerate() {
            let mut cpk = vec![0u8; XW_PK];
            let mut csk = vec![0u8; XW_SK];
            let mut rpk = vec![0u8; XW_PK];
            let mut rsk = vec![0u8; XW_SK];
            ckp(cpk.as_mut_ptr(), csk.as_mut_ptr(), kseed.as_ptr());
            rkp(rpk.as_mut_ptr(), rsk.as_mut_ptr(), kseed.as_ptr());
            for (ei, eseed) in eseeds.iter().enumerate() {
                let mut cct = vec![0u8; XW_CT];
                let mut css = vec![0u8; SS];
                let mut rct = vec![0u8; XW_CT];
                let mut rss = vec![0u8; SS];
                let a = cenc(cct.as_mut_ptr(), css.as_mut_ptr(), cpk.as_ptr(), eseed.as_ptr());
                let b = renc(rct.as_mut_ptr(), rss.as_mut_ptr(), rpk.as_ptr(), eseed.as_ptr());
                eq_i32(&format!("xwing enc_det k{ki}/e{ei} ret"), a, b);
                assert_eq!(a, 0, "xwing enc_det must succeed for an honest pk");
                eq_bytes(&format!("xwing enc_det k{ki}/e{ei} ct"), &cct, &rct);
                eq_bytes(&format!("xwing enc_det k{ki}/e{ei} ss"), &css, &rss);
                // ct = ct_mlkem ‖ ct_x25519
                for (tag, menc, base, pk, ct) in [
                    ("C", &cmenc, &cbase, &cpk, &cct),
                    ("Rust", &rmenc, &rbase, &rpk, &rct),
                ] {
                    let mut mct = vec![0u8; MLK_CT];
                    let mut mss = vec![0u8; SS];
                    assert_eq!(
                        menc(mct.as_mut_ptr(), mss.as_mut_ptr(), pk.as_ptr(), eseed.as_ptr()),
                        0
                    );
                    assert_eq!(&ct[..1088], &mct[..], "{tag}: ct mlkem part mismatch");
                    let mut xct = [0u8; 32];
                    base(xct.as_mut_ptr(), eseed[32..].as_ptr());
                    assert_eq!(&ct[1088..], &xct[..], "{tag}: ct x25519 part mismatch");
                }
            }
        }
    }
}

/// Rows 956-957: derandomised X-Wing round trip through the combiner.
#[test]
fn g6_xwing_deterministic_roundtrip() {
    unsafe {
        let (ckp, rkp) = pair::<SeedKp>("crypto_kem_xwing_seed_keypair");
        let (cenc, renc) = pair::<EncDet>("crypto_kem_xwing_enc_deterministic");
        let (cdec, rdec) = pair::<Dec>("crypto_kem_xwing_dec");
        let mut rng = Rng::new(SEED ^ 0x956);
        for i in 0..5 {
            let mut kseed = [0u8; XW_SEED];
            let mut eseed = [0u8; XW_ENC_SEED];
            rng.fill(&mut kseed);
            rng.fill(&mut eseed);
            let mut cpk = vec![0u8; XW_PK];
            let mut csk = vec![0u8; XW_SK];
            let mut rpk = vec![0u8; XW_PK];
            let mut rsk = vec![0u8; XW_SK];
            ckp(cpk.as_mut_ptr(), csk.as_mut_ptr(), kseed.as_ptr());
            rkp(rpk.as_mut_ptr(), rsk.as_mut_ptr(), kseed.as_ptr());
            let mut cct = vec![0u8; XW_CT];
            let mut css = vec![0u8; SS];
            let mut rct = vec![0u8; XW_CT];
            let mut rss = vec![0u8; SS];
            let a = cenc(cct.as_mut_ptr(), css.as_mut_ptr(), cpk.as_ptr(), eseed.as_ptr());
            let b = renc(rct.as_mut_ptr(), rss.as_mut_ptr(), rpk.as_ptr(), eseed.as_ptr());
            eq_i32(&format!("xwing rt#{i} enc ret"), a, b);
            eq_bytes(&format!("xwing rt#{i} ct"), &cct, &rct);
            eq_bytes(&format!("xwing rt#{i} ss_enc"), &css, &rss);
            let mut css2 = vec![0u8; SS];
            let mut rss2 = vec![0u8; SS];
            let a = cdec(css2.as_mut_ptr(), cct.as_ptr(), csk.as_ptr());
            let b = rdec(rss2.as_mut_ptr(), rct.as_ptr(), rsk.as_ptr());
            eq_i32(&format!("xwing rt#{i} dec ret"), a, b);
            assert_eq!(a, 0);
            eq_bytes(&format!("xwing rt#{i} ss_dec"), &css2, &rss2);
            assert_eq!(css, css2, "C xwing enc/dec disagree");
            assert_eq!(rss, rss2, "Rust xwing enc/dec disagree");
        }
    }
}

/// Rows 958-960: corrupting the ML-KEM half (or replacing the X25519 half with
/// another valid point) yields a different but fully deterministic secret and
/// still returns 0.
#[test]
fn g6_xwing_dec_corruptions() {
    unsafe {
        let (ckp, rkp) = pair::<SeedKp>("crypto_kem_xwing_seed_keypair");
        let (cenc, renc) = pair::<EncDet>("crypto_kem_xwing_enc_deterministic");
        let (cdec, rdec) = pair::<Dec>("crypto_kem_xwing_dec");
        let kseed = [0x31u8; XW_SEED];
        let eseed = [0x77u8; XW_ENC_SEED];
        let mut cpk = vec![0u8; XW_PK];
        let mut csk = vec![0u8; XW_SK];
        let mut rpk = vec![0u8; XW_PK];
        let mut rsk = vec![0u8; XW_SK];
        ckp(cpk.as_mut_ptr(), csk.as_mut_ptr(), kseed.as_ptr());
        rkp(rpk.as_mut_ptr(), rsk.as_mut_ptr(), kseed.as_ptr());
        let mut ct = vec![0u8; XW_CT];
        let mut ss_enc = vec![0u8; SS];
        cenc(ct.as_mut_ptr(), ss_enc.as_mut_ptr(), cpk.as_ptr(), eseed.as_ptr());
        {
            let mut rct = vec![0u8; XW_CT];
            let mut rss = vec![0u8; SS];
            renc(rct.as_mut_ptr(), rss.as_mut_ptr(), rpk.as_ptr(), eseed.as_ptr());
            eq_bytes("xwing corruption base ct", &ct, &rct);
            eq_bytes("xwing corruption base ss", &ss_enc, &rss);
        }
        let mut honest = vec![0u8; SS];
        assert_eq!(cdec(honest.as_mut_ptr(), ct.as_ptr(), csk.as_ptr()), 0);
        assert_eq!(honest, ss_enc, "honest xwing dec != enc secret");

        let mut secrets: Vec<Vec<u8>> = vec![honest.clone()];
        // rows 958/959: ML-KEM part only
        let mut positions: Vec<usize> = vec![0, 1, 544, 1086, 1087];
        let mut rng = Rng::new(SEED ^ 0x958);
        for _ in 0..5 {
            positions.push(rng.below(1088));
        }
        // row 960: bump the X25519 part to another valid (non-low-order) point
        positions.push(1088);
        positions.push(1100);
        for &pos in &positions {
            let mut bad = ct.clone();
            bad[pos] ^= 0x01;
            let mut css = vec![0u8; SS];
            let mut rss = vec![0u8; SS];
            let a = cdec(css.as_mut_ptr(), bad.as_ptr(), csk.as_ptr());
            let b = rdec(rss.as_mut_ptr(), bad.as_ptr(), rsk.as_ptr());
            eq_i32(&format!("xwing dec(corrupt@{pos}) ret"), a, b);
            assert_eq!(a, 0, "xwing dec must return 0 for ct corrupted at {pos}");
            eq_bytes(&format!("xwing dec(corrupt@{pos}) ss"), &css, &rss);
            for prev in &secrets {
                assert_ne!(
                    &css, prev,
                    "xwing dec(corrupt@{pos}) reproduced an earlier secret"
                );
            }
            secrets.push(css);
        }
    }
}

/// Row 961.
#[test]
fn g6_xwing_random_roundtrip() {
    unsafe {
        use_det();
        let (ckp, rkp) = pair::<Kp>("crypto_kem_xwing_keypair");
        let (cenc, renc) = pair::<Enc>("crypto_kem_xwing_enc");
        let (cdec, rdec) = pair::<Dec>("crypto_kem_xwing_dec");
        for i in 0..4u64 {
            let mut cpk = vec![0u8; XW_PK];
            let mut csk = vec![0u8; XW_SK];
            let mut rpk = vec![0u8; XW_PK];
            let mut rsk = vec![0u8; XW_SK];
            det_reseed(0x3000 + i);
            let a = ckp(cpk.as_mut_ptr(), csk.as_mut_ptr());
            det_reseed(0x3000 + i);
            let b = rkp(rpk.as_mut_ptr(), rsk.as_mut_ptr());
            eq_i32("xwing_keypair ret", a, b);
            assert_eq!(a, 0);
            eq_bytes("xwing_keypair pk", &cpk, &rpk);
            eq_bytes("xwing_keypair sk", &csk, &rsk);
            let mut cct = vec![0u8; XW_CT];
            let mut css = vec![0u8; SS];
            let mut rct = vec![0u8; XW_CT];
            let mut rss = vec![0u8; SS];
            det_reseed(0x4000 + i);
            let a = cenc(cct.as_mut_ptr(), css.as_mut_ptr(), cpk.as_ptr());
            det_reseed(0x4000 + i);
            let b = renc(rct.as_mut_ptr(), rss.as_mut_ptr(), rpk.as_ptr());
            eq_i32("xwing_enc ret", a, b);
            assert_eq!(a, 0);
            eq_bytes("xwing_enc ct", &cct, &rct);
            eq_bytes("xwing_enc ss", &css, &rss);
            let mut css2 = vec![0u8; SS];
            let mut rss2 = vec![0u8; SS];
            eq_i32(
                "xwing_dec ret",
                cdec(css2.as_mut_ptr(), cct.as_ptr(), csk.as_ptr()),
                rdec(rss2.as_mut_ptr(), rct.as_ptr(), rsk.as_ptr()),
            );
            eq_bytes("xwing_dec ss", &css2, &rss2);
            assert_eq!(css, css2, "C xwing enc/dec disagree");
            assert_eq!(rss, rss2, "Rust xwing enc/dec disagree");
        }
    }
}

/// Row 962.
#[test]
fn g6_xwing_getters() {
    unsafe {
        for (name, want) in [
            ("crypto_kem_xwing_publickeybytes", 1216usize),
            ("crypto_kem_xwing_secretkeybytes", 32),
            ("crypto_kem_xwing_ciphertextbytes", 1120),
            ("crypto_kem_xwing_sharedsecretbytes", 32),
            ("crypto_kem_xwing_seedbytes", 32),
        ] {
            let (c, r) = pair::<SizeGet>(name);
            let a = c();
            let b = r();
            assert_eq!(a, want, "C {name}");
            assert_eq!(a, b, "{name} differs (C={a}, Rust={b})");
        }
    }
}

/// Rows 963-964: the generic `crypto_kem_*` layer is an exact pass-through of
/// `crypto_kem_xwing_*`.
#[test]
fn g6_kem_generic_dispatch() {
    unsafe {
        use_det();
        let (cskp, rskp) = pair::<SeedKp>("crypto_kem_seed_keypair");
        let (cxskp, rxskp) = pair::<SeedKp>("crypto_kem_xwing_seed_keypair");
        let (cenc, renc) = pair::<Enc>("crypto_kem_enc");
        let (cdec, rdec) = pair::<Dec>("crypto_kem_dec");
        let (cxdec, rxdec) = pair::<Dec>("crypto_kem_xwing_dec");
        let (ckp, rkp) = pair::<Kp>("crypto_kem_keypair");
        for (i, seed) in xwing_seeds().iter().enumerate() {
            let mut cpk = vec![0u8; XW_PK];
            let mut csk = vec![0u8; XW_SK];
            let mut rpk = vec![0u8; XW_PK];
            let mut rsk = vec![0u8; XW_SK];
            let a = cskp(cpk.as_mut_ptr(), csk.as_mut_ptr(), seed.as_ptr());
            let b = rskp(rpk.as_mut_ptr(), rsk.as_mut_ptr(), seed.as_ptr());
            eq_i32("crypto_kem_seed_keypair ret", a, b);
            eq_bytes(&format!("crypto_kem_seed_keypair#{i} pk"), &cpk, &rpk);
            eq_bytes(&format!("crypto_kem_seed_keypair#{i} sk"), &csk, &rsk);
            // identical to the xwing entry point
            let mut xpk = vec![0u8; XW_PK];
            let mut xsk = vec![0u8; XW_SK];
            cxskp(xpk.as_mut_ptr(), xsk.as_mut_ptr(), seed.as_ptr());
            assert_eq!(cpk, xpk, "crypto_kem_seed_keypair != xwing (C)");
            assert_eq!(csk, xsk, "crypto_kem_seed_keypair != xwing (C)");
            let mut xpk2 = vec![0u8; XW_PK];
            let mut xsk2 = vec![0u8; XW_SK];
            rxskp(xpk2.as_mut_ptr(), xsk2.as_mut_ptr(), seed.as_ptr());
            assert_eq!(rpk, xpk2, "crypto_kem_seed_keypair != xwing (Rust)");
            assert_eq!(rsk, xsk2, "crypto_kem_seed_keypair != xwing (Rust)");
            // enc/dec through the generic layer
            let mut cct = vec![0u8; XW_CT];
            let mut css = vec![0u8; SS];
            let mut rct = vec![0u8; XW_CT];
            let mut rss = vec![0u8; SS];
            det_reseed(0x5000 + i as u64);
            let a = cenc(cct.as_mut_ptr(), css.as_mut_ptr(), cpk.as_ptr());
            det_reseed(0x5000 + i as u64);
            let b = renc(rct.as_mut_ptr(), rss.as_mut_ptr(), rpk.as_ptr());
            eq_i32("crypto_kem_enc ret", a, b);
            eq_bytes("crypto_kem_enc ct", &cct, &rct);
            eq_bytes("crypto_kem_enc ss", &css, &rss);
            let mut css2 = vec![0u8; SS];
            let mut rss2 = vec![0u8; SS];
            eq_i32(
                "crypto_kem_dec ret",
                cdec(css2.as_mut_ptr(), cct.as_ptr(), csk.as_ptr()),
                rdec(rss2.as_mut_ptr(), rct.as_ptr(), rsk.as_ptr()),
            );
            eq_bytes("crypto_kem_dec ss", &css2, &rss2);
            assert_eq!(css, css2);
            assert_eq!(rss, rss2);
            let mut css3 = vec![0u8; SS];
            let mut rss3 = vec![0u8; SS];
            cxdec(css3.as_mut_ptr(), cct.as_ptr(), csk.as_ptr());
            rxdec(rss3.as_mut_ptr(), rct.as_ptr(), rsk.as_ptr());
            assert_eq!(css2, css3, "crypto_kem_dec != crypto_kem_xwing_dec (C)");
            assert_eq!(rss2, rss3, "crypto_kem_dec != crypto_kem_xwing_dec (Rust)");
        }
        // crypto_kem_keypair
        for i in 0..2u64 {
            let mut cpk = vec![0u8; XW_PK];
            let mut csk = vec![0u8; XW_SK];
            let mut rpk = vec![0u8; XW_PK];
            let mut rsk = vec![0u8; XW_SK];
            det_reseed(0x6000 + i);
            let a = ckp(cpk.as_mut_ptr(), csk.as_mut_ptr());
            det_reseed(0x6000 + i);
            let b = rkp(rpk.as_mut_ptr(), rsk.as_mut_ptr());
            eq_i32("crypto_kem_keypair ret", a, b);
            eq_bytes("crypto_kem_keypair pk", &cpk, &rpk);
            eq_bytes("crypto_kem_keypair sk", &csk, &rsk);
        }
        // row 964: getters + primitive
        for (name, want) in [
            ("crypto_kem_publickeybytes", 1216usize),
            ("crypto_kem_secretkeybytes", 32),
            ("crypto_kem_ciphertextbytes", 1120),
            ("crypto_kem_sharedsecretbytes", 32),
            ("crypto_kem_seedbytes", 32),
        ] {
            let (c, r) = pair::<SizeGet>(name);
            let a = c();
            let b = r();
            assert_eq!(a, want, "C {name}");
            assert_eq!(a, b, "{name} differs (C={a}, Rust={b})");
        }
        let (cp, rp) = pair::<NameGet>("crypto_kem_primitive");
        let cs = CStr::from_ptr(cp()).to_string_lossy().into_owned();
        let rs_ = CStr::from_ptr(rp()).to_string_lossy().into_owned();
        assert_eq!(cs, "xwing", "C crypto_kem_primitive");
        assert_eq!(cs, rs_, "crypto_kem_primitive differs");
    }
}

// ===========================================================================
// crypto_ipcrypt — CONFIGS rows 965-995
// ===========================================================================

fn ipcrypt_inputs() -> Vec<(String, [u8; 16])> {
    let mut v: Vec<(String, [u8; 16])> = Vec::new();
    v.push(("ipv4 0.0.0.0".into(), ipv4_mapped(0, 0, 0, 0)));
    v.push(("ipv4 127.0.0.1".into(), ipv4_mapped(127, 0, 0, 1)));
    v.push(("ipv4 192.168.1.1".into(), ipv4_mapped(192, 168, 1, 1)));
    v.push(("ipv4 255.255.255.255".into(), ipv4_mapped(255, 255, 255, 255)));
    v.push(("ipv4 10.0.0.1".into(), ipv4_mapped(10, 0, 0, 1)));
    v.push(("ipv6 2001:db8::1".into(), IPV6_DOC));
    v.push(("ipv6 ::1".into(), IPV6_LOOPBACK));
    v.push(("all zero".into(), [0x00; 16]));
    v.push(("all ff".into(), [0xff; 16]));
    let mut rng = Rng::new(0x4950_4352_5950_5400);
    for i in 0..8 {
        let mut b = [0u8; 16];
        rng.fill(&mut b);
        v.push((format!("random#{i}"), b));
    }
    v
}

fn ipcrypt_keys16() -> Vec<(String, [u8; 16])> {
    let mut v: Vec<(String, [u8; 16])> = Vec::new();
    v.push(("k=00".into(), [0x00; 16]));
    v.push(("k=ff".into(), [0xff; 16]));
    let mut k = [0u8; 16];
    k.copy_from_slice(&incr(16));
    v.push(("k=incr".into(), k));
    let mut rng = Rng::new(0x4B45_5900_0000_0001);
    for i in 0..4 {
        let mut b = [0u8; 16];
        rng.fill(&mut b);
        v.push((format!("k=random#{i}"), b));
    }
    v
}

/// 32-byte keys for the ndx / pfx variants, including the degenerate ones
/// whose two halves collide (`expand_key(k)[5] == expand_key(k+16)[5]`) and so
/// trigger the `k[i] ^ 0x5a` re-derivation branch.
fn ipcrypt_keys32() -> Vec<(String, [u8; 32])> {
    let mut v: Vec<(String, [u8; 32])> = Vec::new();
    v.push(("k32=00 (degenerate)".into(), [0x00; 32]));
    v.push(("k32=ff (degenerate)".into(), [0xff; 32]));
    let mut k = [0u8; 32];
    k.copy_from_slice(&incr(32));
    v.push(("k32=incr".into(), k));
    let mut k = [0u8; 32];
    for i in 0..16 {
        k[i] = i as u8;
        k[16 + i] = i as u8;
    }
    v.push(("k32=incr||incr (degenerate)".into(), k));
    let mut k = [0x5au8; 32];
    v.push(("k32=5a (degenerate)".into(), k));
    k = [0u8; 32];
    let mut rng = Rng::new(0x4B33_3200_0000_0001);
    for i in 0..4 {
        rng.fill(&mut k);
        v.push((format!("k32=random#{i}"), k));
    }
    v
}

/// Rows 965-972 + 974: the deterministic (single-block AES) variant.
#[test]
fn g6_ipcrypt_encrypt_vectors() {
    unsafe {
        let (c, r) = pair::<Ip3>("crypto_ipcrypt_encrypt");
        for (kn, k) in ipcrypt_keys16() {
            for (inn, ip) in ipcrypt_inputs() {
                let mut co = [0xAAu8; 16];
                let mut ro = [0xAAu8; 16];
                c(co.as_mut_ptr(), ip.as_ptr(), k.as_ptr());
                r(ro.as_mut_ptr(), ip.as_ptr(), k.as_ptr());
                eq_bytes(&format!("ipcrypt_encrypt {kn} {inn}"), &co, &ro);
            }
        }
    }
}

/// Rows 973-974: `_decrypt` is the exact inverse, and decrypting an arbitrary
/// block is deterministic.
#[test]
fn g6_ipcrypt_decrypt_and_roundtrip() {
    unsafe {
        let (ce, re) = pair::<Ip3>("crypto_ipcrypt_encrypt");
        let (cd, rd) = pair::<Ip3>("crypto_ipcrypt_decrypt");
        for (kn, k) in ipcrypt_keys16() {
            for (inn, ip) in ipcrypt_inputs() {
                let mut co = [0u8; 16];
                let mut ro = [0u8; 16];
                ce(co.as_mut_ptr(), ip.as_ptr(), k.as_ptr());
                re(ro.as_mut_ptr(), ip.as_ptr(), k.as_ptr());
                eq_bytes(&format!("encrypt {kn} {inn}"), &co, &ro);
                let mut cb = [0u8; 16];
                let mut rb = [0u8; 16];
                cd(cb.as_mut_ptr(), co.as_ptr(), k.as_ptr());
                rd(rb.as_mut_ptr(), ro.as_ptr(), k.as_ptr());
                eq_bytes(&format!("decrypt {kn} {inn}"), &cb, &rb);
                assert_eq!(&cb[..], &ip[..], "C round trip failed for {kn} {inn}");
                // decrypting an arbitrary (non-ciphertext) block
                let mut cx = [0u8; 16];
                let mut rx = [0u8; 16];
                cd(cx.as_mut_ptr(), ip.as_ptr(), k.as_ptr());
                rd(rx.as_mut_ptr(), ip.as_ptr(), k.as_ptr());
                eq_bytes(&format!("decrypt raw {kn} {inn}"), &cx, &rx);
            }
        }
    }
}

/// Row 975: bijectivity spot check over 256 distinct inputs.
#[test]
fn g6_ipcrypt_bijectivity() {
    unsafe {
        let (c, r) = pair::<Ip3>("crypto_ipcrypt_encrypt");
        let k: Vec<u8> = incr(16);
        let mut outs = std::collections::HashSet::new();
        for i in 0..256usize {
            let ip = ipv4_mapped(10, 1, (i >> 8) as u8, i as u8);
            let mut co = [0u8; 16];
            let mut ro = [0u8; 16];
            c(co.as_mut_ptr(), ip.as_ptr(), k.as_ptr());
            r(ro.as_mut_ptr(), ip.as_ptr(), k.as_ptr());
            eq_bytes(&format!("bijectivity i={i}"), &co, &ro);
            assert!(outs.insert(co), "ipcrypt_encrypt collided at i={i}");
        }
        assert_eq!(outs.len(), 256);
    }
}

/// Rows 976-981: the `nd` variant with EXPLICIT tweaks.
#[test]
fn g6_ipcrypt_nd() {
    unsafe {
        let (ce, re) = pair::<Ip4>("crypto_ipcrypt_nd_encrypt");
        let (cd, rd) = pair::<Ip3>("crypto_ipcrypt_nd_decrypt");
        let mut tweaks: Vec<[u8; 8]> = vec![[0x00; 8], [0xff; 8]];
        let mut t = [0u8; 8];
        t.copy_from_slice(&incr(8));
        tweaks.push(t);
        let mut rng = Rng::new(SEED ^ 0x976);
        for _ in 0..4 {
            let mut t = [0u8; 8];
            rng.fill(&mut t);
            tweaks.push(t);
        }
        for (kn, k) in ipcrypt_keys16() {
            for (inn, ip) in ipcrypt_inputs() {
                let mut seen: Vec<[u8; 16]> = Vec::new();
                for (ti, tw) in tweaks.iter().enumerate() {
                    let mut co = [0xAAu8; 24];
                    let mut ro = [0xAAu8; 24];
                    ce(co.as_mut_ptr(), ip.as_ptr(), tw.as_ptr(), k.as_ptr());
                    re(ro.as_mut_ptr(), ip.as_ptr(), tw.as_ptr(), k.as_ptr());
                    eq_bytes(&format!("nd_encrypt {kn} {inn} t{ti}"), &co, &ro);
                    // row 976: the tweak is copied verbatim
                    assert_eq!(&co[..8], &tw[..], "nd_encrypt did not copy the tweak");
                    // row 979: different tweaks must change out[8..24]
                    let mut body = [0u8; 16];
                    body.copy_from_slice(&co[8..]);
                    assert!(
                        !seen.contains(&body),
                        "nd_encrypt: two tweaks gave the same ciphertext ({kn} {inn})"
                    );
                    seen.push(body);
                    // row 980: round trip (the tweak is read back from in[0..8])
                    let mut cb = [0u8; 16];
                    let mut rb = [0u8; 16];
                    cd(cb.as_mut_ptr(), co.as_ptr(), k.as_ptr());
                    rd(rb.as_mut_ptr(), ro.as_ptr(), k.as_ptr());
                    eq_bytes(&format!("nd_decrypt {kn} {inn} t{ti}"), &cb, &rb);
                    assert_eq!(&cb[..], &ip[..], "nd round trip failed {kn} {inn} t{ti}");
                }
                // row 981: hand-crafted 24-byte input
                let mut craft = [0u8; 24];
                craft[8..].copy_from_slice(&ip);
                let mut cb = [0u8; 16];
                let mut rb = [0u8; 16];
                cd(cb.as_mut_ptr(), craft.as_ptr(), k.as_ptr());
                rd(rb.as_mut_ptr(), craft.as_ptr(), k.as_ptr());
                eq_bytes(&format!("nd_decrypt crafted {kn} {inn}"), &cb, &rb);
            }
        }
    }
}

/// Rows 982-987: the `ndx` variant, including the degenerate-key branch.
#[test]
fn g6_ipcrypt_ndx() {
    unsafe {
        let (ce, re) = pair::<Ip4>("crypto_ipcrypt_ndx_encrypt");
        let (cd, rd) = pair::<Ip3>("crypto_ipcrypt_ndx_decrypt");
        let mut tweaks: Vec<[u8; 16]> = vec![[0x00; 16], [0xff; 16]];
        let mut t = [0u8; 16];
        t.copy_from_slice(&incr(16));
        tweaks.push(t);
        let mut rng = Rng::new(SEED ^ 0x982);
        for _ in 0..3 {
            let mut t = [0u8; 16];
            rng.fill(&mut t);
            tweaks.push(t);
        }
        for (kn, k) in ipcrypt_keys32() {
            for (inn, ip) in ipcrypt_inputs() {
                for (ti, tw) in tweaks.iter().enumerate() {
                    let mut co = [0xAAu8; 32];
                    let mut ro = [0xAAu8; 32];
                    ce(co.as_mut_ptr(), ip.as_ptr(), tw.as_ptr(), k.as_ptr());
                    re(ro.as_mut_ptr(), ip.as_ptr(), tw.as_ptr(), k.as_ptr());
                    eq_bytes(&format!("ndx_encrypt {kn} {inn} t{ti}"), &co, &ro);
                    assert_eq!(&co[..16], &tw[..], "ndx_encrypt did not copy the tweak");
                    let mut cb = [0u8; 16];
                    let mut rb = [0u8; 16];
                    cd(cb.as_mut_ptr(), co.as_ptr(), k.as_ptr());
                    rd(rb.as_mut_ptr(), ro.as_ptr(), k.as_ptr());
                    eq_bytes(&format!("ndx_decrypt {kn} {inn} t{ti}"), &cb, &rb);
                    assert_eq!(&cb[..], &ip[..], "ndx round trip failed {kn} {inn} t{ti}");
                }
                // row 987: hand-crafted 32-byte input
                let mut craft = [0u8; 32];
                craft[16..].copy_from_slice(&ip);
                let mut cb = [0u8; 16];
                let mut rb = [0u8; 16];
                cd(cb.as_mut_ptr(), craft.as_ptr(), k.as_ptr());
                rd(rb.as_mut_ptr(), craft.as_ptr(), k.as_ptr());
                eq_bytes(&format!("ndx_decrypt crafted {kn} {inn}"), &cb, &rb);
            }
        }
    }
}

/// Rows 982/985/990 + ERRORS 732/733: prove the degenerate-key branch is
/// really taken, by checking that a key whose halves are equal behaves like
/// the explicit `k[i] ^ 0x5a` re-derivation (i.e. differs from what a
/// non-degenerate key with the same first half would give) — and that C and
/// Rust take the branch on exactly the same keys.
#[test]
fn g6_ipcrypt_degenerate_key_branch() {
    unsafe {
        let (cndx, rndx) = pair::<Ip4>("crypto_ipcrypt_ndx_encrypt");
        let (cpfx, rpfx) = pair::<Ip3>("crypto_ipcrypt_pfx_encrypt");
        let ip = ipv4_mapped(192, 168, 1, 1);
        let tw = [0x00u8; 16];
        for half in [[0x00u8; 16], [0xffu8; 16], {
            let mut h = [0u8; 16];
            h.copy_from_slice(&incr(16));
            h
        }] {
            // degenerate: both halves identical
            let mut kd = [0u8; 32];
            kd[..16].copy_from_slice(&half);
            kd[16..].copy_from_slice(&half);
            // the re-derived equivalent: second half literally k[i] ^ 0x5a
            let mut ke = [0u8; 32];
            ke[..16].copy_from_slice(&half);
            for i in 0..16 {
                ke[16 + i] = half[i] ^ 0x5a;
            }
            // NOTE: `ke` is NOT the same computation (in the degenerate branch
            // it is `rkeys`/`k2keys` that gets re-expanded, not `tkeys`), so we
            // only require C == Rust plus "the branch changed something".
            let mut a = [0u8; 32];
            let mut b = [0u8; 32];
            cndx(a.as_mut_ptr(), ip.as_ptr(), tw.as_ptr(), kd.as_ptr());
            rndx(b.as_mut_ptr(), ip.as_ptr(), tw.as_ptr(), kd.as_ptr());
            eq_bytes("ndx degenerate", &a, &b);
            let mut a2 = [0u8; 32];
            let mut b2 = [0u8; 32];
            cndx(a2.as_mut_ptr(), ip.as_ptr(), tw.as_ptr(), ke.as_ptr());
            rndx(b2.as_mut_ptr(), ip.as_ptr(), tw.as_ptr(), ke.as_ptr());
            eq_bytes("ndx non-degenerate twin", &a2, &b2);
            assert_ne!(a, a2, "degenerate and twin key gave identical ndx output");
            let mut a3 = [0u8; 16];
            let mut b3 = [0u8; 16];
            cpfx(a3.as_mut_ptr(), ip.as_ptr(), kd.as_ptr());
            rpfx(b3.as_mut_ptr(), ip.as_ptr(), kd.as_ptr());
            eq_bytes("pfx degenerate", &a3, &b3);
            let mut a4 = [0u8; 16];
            let mut b4 = [0u8; 16];
            cpfx(a4.as_mut_ptr(), ip.as_ptr(), ke.as_ptr());
            rpfx(b4.as_mut_ptr(), ip.as_ptr(), ke.as_ptr());
            eq_bytes("pfx non-degenerate twin", &a4, &b4);
        }
    }
}

/// Rows 988-992: the `pfx` (prefix-preserving) variant.
#[test]
fn g6_ipcrypt_pfx() {
    unsafe {
        let (ce, re) = pair::<Ip3>("crypto_ipcrypt_pfx_encrypt");
        let (cd, rd) = pair::<Ip3>("crypto_ipcrypt_pfx_decrypt");
        for (kn, k) in ipcrypt_keys32() {
            for (inn, ip) in ipcrypt_inputs() {
                let mut co = [0xAAu8; 16];
                let mut ro = [0xAAu8; 16];
                ce(co.as_mut_ptr(), ip.as_ptr(), k.as_ptr());
                re(ro.as_mut_ptr(), ip.as_ptr(), k.as_ptr());
                eq_bytes(&format!("pfx_encrypt {kn} {inn}"), &co, &ro);
                // row 988/734: for IPv4-mapped input the mapped prefix survives
                let mapped = ip[..10].iter().all(|&x| x == 0) && ip[10] == 0xff && ip[11] == 0xff;
                if mapped {
                    assert!(
                        co[..10].iter().all(|&x| x == 0) && co[10] == 0xff && co[11] == 0xff,
                        "pfx_encrypt did not preserve the IPv4-mapped prefix ({inn})"
                    );
                }
                // row 992: round trip
                let mut cb = [0u8; 16];
                let mut rb = [0u8; 16];
                cd(cb.as_mut_ptr(), co.as_ptr(), k.as_ptr());
                rd(rb.as_mut_ptr(), ro.as_ptr(), k.as_ptr());
                eq_bytes(&format!("pfx_decrypt {kn} {inn}"), &cb, &rb);
                assert_eq!(&cb[..], &ip[..], "pfx round trip failed {kn} {inn}");
            }
        }
    }
}

/// Row 991: prefix preservation — two IPv4-mapped addresses sharing a /24 must
/// produce outputs sharing the corresponding 24 encrypted bits (output bytes
/// 12..15), identically in C and Rust.
#[test]
fn g6_ipcrypt_pfx_prefix_preservation() {
    unsafe {
        let (c, r) = pair::<Ip3>("crypto_ipcrypt_pfx_encrypt");
        let mut k = [0u8; 32];
        k.copy_from_slice(&incr(32));
        let base = ipv4_mapped(192, 168, 1, 1);
        let mut cbase = [0u8; 16];
        let mut rbase = [0u8; 16];
        c(cbase.as_mut_ptr(), base.as_ptr(), k.as_ptr());
        r(rbase.as_mut_ptr(), base.as_ptr(), k.as_ptr());
        eq_bytes("pfx base", &cbase, &rbase);
        for last in [0u8, 2, 7, 99, 255] {
            let ip = ipv4_mapped(192, 168, 1, last);
            let mut co = [0u8; 16];
            let mut ro = [0u8; 16];
            c(co.as_mut_ptr(), ip.as_ptr(), k.as_ptr());
            r(ro.as_mut_ptr(), ip.as_ptr(), k.as_ptr());
            eq_bytes(&format!("pfx 192.168.1.{last}"), &co, &ro);
            assert_eq!(
                &co[..15],
                &cbase[..15],
                "C: /24-shared prefix not preserved for .{last}"
            );
            assert_eq!(
                &ro[..15],
                &rbase[..15],
                "Rust: /24-shared prefix not preserved for .{last}"
            );
        }
        // a /16-shared address must share only the first 16 encrypted bits
        let other = ipv4_mapped(192, 168, 9, 1);
        let mut co = [0u8; 16];
        c(co.as_mut_ptr(), other.as_ptr(), k.as_ptr());
        assert_eq!(&co[..14], &cbase[..14], "/16 prefix not preserved");
    }
}

/// Row 993: the four keygen entry points under a deterministic `randombytes`.
#[test]
fn g6_ipcrypt_keygen() {
    unsafe {
        use_det();
        for (name, len) in [
            ("crypto_ipcrypt_keygen", 16usize),
            ("crypto_ipcrypt_nd_keygen", 16),
            ("crypto_ipcrypt_ndx_keygen", 32),
            ("crypto_ipcrypt_pfx_keygen", 32),
        ] {
            let (c, r) = pair::<Keygen>(name);
            for i in 0..4u64 {
                let mut ck = vec![0xAAu8; len + 8];
                let mut rk = vec![0xAAu8; len + 8];
                det_reseed(0x7000 + i);
                c(ck.as_mut_ptr());
                det_reseed(0x7000 + i);
                r(rk.as_mut_ptr());
                eq_bytes(&format!("{name} #{i}"), &ck, &rk);
                assert!(
                    ck[len..].iter().all(|&x| x == 0xAA),
                    "{name} wrote past {len} bytes"
                );
                assert!(
                    ck[..len].iter().any(|&x| x != 0xAA),
                    "{name} wrote nothing"
                );
            }
        }
    }
}

/// Row 994.
#[test]
fn g6_ipcrypt_getters() {
    unsafe {
        for (name, want) in [
            ("crypto_ipcrypt_bytes", 16usize),
            ("crypto_ipcrypt_keybytes", 16),
            ("crypto_ipcrypt_nd_keybytes", 16),
            ("crypto_ipcrypt_nd_tweakbytes", 8),
            ("crypto_ipcrypt_nd_inputbytes", 16),
            ("crypto_ipcrypt_nd_outputbytes", 24),
            ("crypto_ipcrypt_ndx_keybytes", 32),
            ("crypto_ipcrypt_ndx_tweakbytes", 16),
            ("crypto_ipcrypt_ndx_inputbytes", 16),
            ("crypto_ipcrypt_ndx_outputbytes", 32),
            ("crypto_ipcrypt_pfx_keybytes", 32),
            ("crypto_ipcrypt_pfx_bytes", 16),
        ] {
            let (c, r) = pair::<SizeGet>(name);
            let a = c();
            let b = r();
            assert_eq!(a, want, "C {name}");
            assert_eq!(a, b, "{name} differs (C={a}, Rust={b})");
        }
    }
}

/// Row 995: `_crypto_ipcrypt_pick_best_implementation` returns 0 and (on this
/// build, where neither AES-NI nor ARM crypto is compiled in) leaves the soft
/// backend selected, so the outputs are unchanged.
#[test]
fn g6_ipcrypt_pick_best_implementation() {
    unsafe {
        let (cp, rp) = pair::<PickBest>("_crypto_ipcrypt_pick_best_implementation");
        let (ce, re) = pair::<Ip3>("crypto_ipcrypt_encrypt");
        let k: Vec<u8> = incr(16);
        let ip = ipv4_mapped(1, 2, 3, 4);
        let mut before_c = [0u8; 16];
        let mut before_r = [0u8; 16];
        ce(before_c.as_mut_ptr(), ip.as_ptr(), k.as_ptr());
        re(before_r.as_mut_ptr(), ip.as_ptr(), k.as_ptr());
        eq_bytes("encrypt before pick_best", &before_c, &before_r);
        for _ in 0..3 {
            eq_i32("pick_best_implementation", cp(), rp());
            assert_eq!(cp(), 0, "pick_best_implementation must return 0");
            let mut a = [0u8; 16];
            let mut b = [0u8; 16];
            ce(a.as_mut_ptr(), ip.as_ptr(), k.as_ptr());
            re(b.as_mut_ptr(), ip.as_ptr(), k.as_ptr());
            eq_bytes("encrypt after pick_best", &a, &b);
            assert_eq!(a, before_c, "pick_best changed the C output");
            assert_eq!(b, before_r, "pick_best changed the Rust output");
        }
        // the exported soft backend vtable must exist in both libraries
        assert!(has_sym("ipcrypt_soft_implementation"));
    }
}

// ===========================================================================
// child-process driver (fresh-state cases)
// ===========================================================================
#[test]
fn zz_abort_child() {
    let Some((case, is_c)) = child_case() else {
        return;
    };
    let l = libs();
    let h = if is_c { l.c } else { l.rs };
    unsafe {
        match case.as_str() {
            // row 911: in a fresh process the default implementation must be
            // `sysrandom`. Exit 0 iff so, so that the two children can only
            // terminate identically when C and Rust agree.
            "default_impl_name" => {
                let f: Symbol<NameGet> =
                    h.get(b"randombytes_implementation_name\0").unwrap();
                let n = CStr::from_ptr(f()).to_string_lossy().into_owned();
                if n == "sysrandom" {
                    std::process::exit(0);
                }
                eprintln!("unexpected default implementation name {n:?}");
                std::process::exit(3);
            }
            other => panic!("unknown child case {other:?}"),
        }
    }
}
