//! Phase C — error-path differential tests for group G6
//! (ERRORS.md rows 639-739): `randombytes/`, `crypto_kem/`, `crypto_ipcrypt/`.
//!
//! Rows whose rejection is a *return value* are compared directly. Rows whose
//! rejection is a `sodium_misuse()` / NULL-function-pointer call kills the
//! process, so they are compared by re-executing this binary in two children
//! (one per library) and comparing how the two children died — the same
//! mechanism is also used, with meaningful exit codes, for the few checks that
//! need a pristine process state.
//!
//! Rows that are unreachable through the public API on this platform/build are
//! enumerated (with the reason) in `g6e_unreachable_rows_documented`.
mod common;
use common::*;
use libloading::{Library, Symbol};
use std::ffi::CStr;
use std::os::raw::{c_char, c_int, c_void};
use std::ptr;
use std::sync::atomic::{AtomicU32, AtomicUsize, Ordering as AtOrd};

const SEED: u64 = 0x1F83_D9AB_FB41_BD6B;

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

type Ip3 = unsafe extern "C" fn(*mut u8, *const u8, *const u8);
type Ip4 = unsafe extern "C" fn(*mut u8, *const u8, *const u8, *const u8);
type Keygen = unsafe extern "C" fn(*mut u8);
type PickBest = unsafe extern "C" fn() -> c_int;

const MLK_PK: usize = 1184;
const MLK_SK: usize = 2400;
const MLK_CT: usize = 1088;
const SS: usize = 32;
const MLK_SEED: usize = 64;
const XW_PK: usize = 1216;
const XW_SK: usize = 32;
const XW_CT: usize = 1120;
const XW_ENC_SEED: usize = 64;

/// `randombytes_BYTES_MAX` for `randombytes_buf_deterministic` on a 64-bit
/// target (`0x4000000000` = 274877906944 = 256 GiB).
const BUFDET_MAX: usize = 0x4000_0000_00;

// ===========================================================================
// custom implementations (counting / deliberately broken)
// ===========================================================================
const SEQ_MAX: usize = 32;
static SEQ: [AtomicU32; SEQ_MAX] = [const { AtomicU32::new(0) }; SEQ_MAX];
static SEQ_LEN: AtomicUsize = AtomicUsize::new(0);
static SEQ_POS: AtomicUsize = AtomicUsize::new(0);
static RAND_CALLS: AtomicUsize = AtomicUsize::new(0);
static BUF_CALLS: AtomicUsize = AtomicUsize::new(0);
static STIR_CALLS: AtomicUsize = AtomicUsize::new(0);
static CLOSE_CALLS: AtomicUsize = AtomicUsize::new(0);

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

extern "C" fn cnt_buf(buf: *mut c_void, size: usize) {
    BUF_CALLS.fetch_add(1, AtOrd::SeqCst);
    if buf.is_null() || size == 0 {
        return;
    }
    let out = unsafe { std::slice::from_raw_parts_mut(buf as *mut u8, size) };
    for (i, b) in out.iter_mut().enumerate() {
        *b = (i as u8) ^ 0x5a;
    }
}

/// A `buf` hook that ONLY records the requested size (it writes nothing), so
/// absurd lengths can be pushed through the FFI boundary safely.
static BUF_LAST_SIZE: AtomicUsize = AtomicUsize::new(0);

extern "C" fn rec_buf(_buf: *mut c_void, size: usize) {
    BUF_CALLS.fetch_add(1, AtOrd::SeqCst);
    BUF_LAST_SIZE.store(size, AtOrd::SeqCst);
}

static IMPL_RECORD: RandombytesImpl = RandombytesImpl {
    implementation_name: Some(cnt_name),
    random: Some(cnt_random),
    stir: Some(cnt_stir),
    uniform: None,
    buf: Some(rec_buf),
    close: Some(cnt_close),
};

extern "C" fn cnt_close() -> c_int {
    CLOSE_CALLS.fetch_add(1, AtOrd::SeqCst);
    0
}

extern "C" fn hook_uniform(upper_bound: u32) -> u32 {
    upper_bound
}

static IMPL_FULL: RandombytesImpl = RandombytesImpl {
    implementation_name: Some(cnt_name),
    random: Some(cnt_random),
    stir: Some(cnt_stir),
    uniform: None,
    buf: Some(cnt_buf),
    close: Some(cnt_close),
};

/// Row 646: non-NULL `.uniform` -> unconditional delegation.
static IMPL_UNIFORM_HOOK: RandombytesImpl = RandombytesImpl {
    implementation_name: Some(cnt_name),
    random: Some(cnt_random),
    stir: Some(cnt_stir),
    uniform: Some(hook_uniform),
    buf: Some(cnt_buf),
    close: Some(cnt_close),
};

/// Rows 649/651: `.stir == NULL`, `.close == NULL` are legal.
static IMPL_NO_STIR_CLOSE: RandombytesImpl = RandombytesImpl {
    implementation_name: Some(cnt_name),
    random: Some(cnt_random),
    stir: None,
    uniform: None,
    buf: Some(cnt_buf),
    close: None,
};

// Row 640: the "required" hooks are documented but never checked; a NULL one
// is accepted by `randombytes_set_implementation` and blows up on first use.
static IMPL_NULL_NAME: RandombytesImpl = RandombytesImpl {
    implementation_name: None,
    random: Some(cnt_random),
    stir: None,
    uniform: None,
    buf: Some(cnt_buf),
    close: None,
};
static IMPL_NULL_RANDOM: RandombytesImpl = RandombytesImpl {
    implementation_name: Some(cnt_name),
    random: None,
    stir: None,
    uniform: None,
    buf: Some(cnt_buf),
    close: None,
};
static IMPL_NULL_BUF: RandombytesImpl = RandombytesImpl {
    implementation_name: Some(cnt_name),
    random: Some(cnt_random),
    stir: None,
    uniform: None,
    buf: None,
    close: None,
};

// ===========================================================================
// helpers
// ===========================================================================
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

unsafe fn use_det() {
    install_det_random();
    set_impl_both(&DET_IMPL as *const RandombytesImpl);
}

unsafe fn name_of(lib: &'static Library) -> String {
    let f: Symbol<NameGet> = lib.get(b"randombytes_implementation_name\0").unwrap();
    CStr::from_ptr(f()).to_string_lossy().into_owned()
}

fn incr(n: usize) -> Vec<u8> {
    (0..n).map(|i| i as u8).collect()
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

/// Curve25519 encodings whose X25519 output is all-zero (or that are otherwise
/// classic small-order points), i.e. the only reachable `-1` trigger for
/// `crypto_kem_xwing_enc`/`_dec`.
const LOW_ORDER: [[u8; 32]; 5] = [
    [0u8; 32],
    {
        let mut p = [0u8; 32];
        p[0] = 1;
        p
    },
    [
        0xe0, 0xeb, 0x7a, 0x7c, 0x3b, 0x41, 0xb8, 0xae, 0x16, 0x56, 0xe3, 0xfa, 0xf1, 0x9f, 0xc4,
        0x6a, 0xda, 0x09, 0x8d, 0xeb, 0x9c, 0x32, 0xb1, 0xfd, 0x86, 0x62, 0x05, 0x16, 0x5f, 0x49,
        0xb8, 0x00,
    ],
    [
        0x5f, 0x9c, 0x95, 0xbc, 0xa3, 0x50, 0x8c, 0x24, 0xb1, 0xd0, 0xb1, 0x55, 0x9c, 0x83, 0xef,
        0x5b, 0x04, 0x44, 0x5c, 0xc4, 0x58, 0x1c, 0x8e, 0x86, 0xd8, 0x22, 0x4e, 0xdd, 0xd0, 0x9f,
        0x11, 0x57,
    ],
    [0xffu8; 32],
];

// ===========================================================================
// randombytes_set_implementation — ERRORS rows 639, 640
// ===========================================================================

/// Row 639: `randombytes_set_implementation` never validates its argument —
/// not even for NULL (it is declared `nonnull`, but nothing checks it). It
/// always returns 0 and stores the pointer verbatim; a NULL implementation
/// makes the next `randombytes_*` call fall back to the built-in default.
#[test]
fn g6e_set_implementation_never_validates() {
    unsafe {
        let l = libs();
        // NULL
        for lib in [l.c, l.rs] {
            eq_i32("set_implementation(NULL)", 0, set_impl(lib, ptr::null()));
        }
        let cn = name_of(l.c);
        let rn = name_of(l.rs);
        assert_eq!(cn, rn, "post-NULL implementation name differs");
        assert_eq!(cn, "sysrandom", "NULL did not revert to the default");
        // an implementation missing every optional hook is accepted, and so is
        // one missing the "required" ones (row 640) — the failure only shows up
        // when the hook is actually invoked (see g6e_null_hooks_crash_alike).
        for p in [
            &IMPL_FULL as *const RandombytesImpl,
            &IMPL_NO_STIR_CLOSE as *const RandombytesImpl,
            &IMPL_NULL_NAME as *const RandombytesImpl,
            &IMPL_NULL_RANDOM as *const RandombytesImpl,
            &IMPL_NULL_BUF as *const RandombytesImpl,
            ptr::null(),
        ] {
            for lib in [l.c, l.rs] {
                eq_i32("set_implementation", 0, set_impl(lib, p));
            }
        }
        // repeated NULL stores are also fine
        for _ in 0..3 {
            for lib in [l.c, l.rs] {
                eq_i32("set_implementation(NULL) again", 0, set_impl(lib, ptr::null()));
            }
        }
        use_det();
    }
}

/// Row 640: with a NULL "required" hook installed, the first use dereferences a
/// NULL function pointer. Both libraries must die the same way.
#[test]
fn g6e_null_hooks_crash_alike() {
    for case in ["impl_null_name", "impl_null_random", "impl_null_buf"] {
        let t = diff_abort_case(case);
        assert!(
            t.signal.is_some() || t.code.map(|c| c != 0).unwrap_or(false),
            "case {case:?}: expected abnormal termination in both libraries, got {t:?}"
        );
    }
}

// ===========================================================================
// randombytes_buf_deterministic — ERRORS rows 641, 642
// ===========================================================================

/// Row 641: `size > 0x4000000000` -> `sodium_misuse()` -> `abort()`; one step
/// past the documented maximum. (`size == 0x4000000000` itself is accepted but
/// would require a 256 GiB buffer, so only the rejecting side is exercised.)
///
/// Row 642: there is no runtime check that `seed` really has 32 bytes (the
/// array-parameter length is not enforced) — reading a short seed is an
/// out-of-bounds read, i.e. undefined behaviour with no error path, so it is
/// deliberately NOT exercised.
#[test]
fn g6e_buf_deterministic_oversize_aborts() {
    for case in ["bufdet_oversize", "bufdet_sizemax"] {
        let t = diff_abort_case(case);
        assert!(
            t.signal.is_some() || t.code.map(|c| c != 0).unwrap_or(false),
            "{case}: expected abort in both libraries, got {t:?}"
        );
    }
    // and NULL/zero is fine (the chacha20 stream short-circuits on clen == 0)
    let t = diff_abort_case("bufdet_null_zero");
    assert_eq!(t.code, Some(0), "bufdet_null_zero must not crash: {t:?}");
}

/// Row 643: the NaCl-compat wrapper asserts `buf_len <= SIZE_MAX`, which can
/// only fail where `unsigned long long` is wider than `size_t` (32-bit
/// targets); on this 64-bit target the assertion is a tautology. Only the
/// benign lengths are comparable, including 0 and a NULL buffer with length 0.
#[test]
fn g6e_nacl_randombytes_lengths() {
    unsafe {
        set_impl_both(&IMPL_FULL as *const RandombytesImpl);
        let (c, r) = pair::<NaclRb>("randombytes");
        for n in [0u64, 1, 2, 63, 64, 65, 1000] {
            let mut a = vec![0xAAu8; n as usize + 8];
            let mut b = vec![0xAAu8; n as usize + 8];
            BUF_CALLS.store(0, AtOrd::SeqCst);
            c(a.as_mut_ptr(), n);
            let cc = BUF_CALLS.load(AtOrd::SeqCst);
            BUF_CALLS.store(0, AtOrd::SeqCst);
            r(b.as_mut_ptr(), n);
            let rc = BUF_CALLS.load(AtOrd::SeqCst);
            eq_bytes(&format!("randombytes(len={n})"), &a, &b);
            assert_eq!(cc, rc, "randombytes(len={n}): buf hook call count differs");
            assert_eq!(
                cc,
                if n == 0 { 0 } else { 1 },
                "randombytes(len={n}) called the buf hook {cc} times"
            );
            assert!(
                a[n as usize..].iter().all(|&x| x == 0xAA),
                "randombytes(len={n}) wrote past the requested length"
            );
        }
        // NULL + zero length must be harmless in both libraries
        c(ptr::null_mut(), 0);
        r(ptr::null_mut(), 0);
        use_det();
    }
}

/// Generic FFI boundary check for rows 643/648/920: the `unsigned long long`
/// length of the NaCl wrapper and the `size_t` of `randombytes_buf` must be
/// forwarded to the implementation hook unchanged, including values above
/// 2^32 and `SIZE_MAX`. A record-only `buf` hook makes the absurd lengths safe.
#[test]
fn g6e_length_conversion_across_ffi() {
    unsafe {
        set_impl_both(&IMPL_RECORD as *const RandombytesImpl);
        let (cn, rn) = pair::<NaclRb>("randombytes");
        let (cb, rb) = pair::<Buf>("randombytes_buf");
        let mut sink = [0u8; 8];
        for n in [
            0u64,
            1,
            0xffff_ffff,
            0x1_0000_0000,
            0x4000_0000_00,
            0x4000_0000_01,
            u64::MAX - 1,
            u64::MAX,
        ] {
            BUF_CALLS.store(0, AtOrd::SeqCst);
            BUF_LAST_SIZE.store(0xdead, AtOrd::SeqCst);
            cn(sink.as_mut_ptr(), n);
            let (cc, cs) = (
                BUF_CALLS.load(AtOrd::SeqCst),
                BUF_LAST_SIZE.load(AtOrd::SeqCst),
            );
            BUF_CALLS.store(0, AtOrd::SeqCst);
            BUF_LAST_SIZE.store(0xdead, AtOrd::SeqCst);
            rn(sink.as_mut_ptr(), n);
            let (rc, rs) = (
                BUF_CALLS.load(AtOrd::SeqCst),
                BUF_LAST_SIZE.load(AtOrd::SeqCst),
            );
            assert_eq!(cc, rc, "randombytes({n}): hook call counts differ");
            assert_eq!(cs, rs, "randombytes({n}): forwarded size differs");
            if n == 0 {
                assert_eq!(cc, 0, "randombytes(0) called the hook");
            } else {
                assert_eq!(cc, 1);
                assert_eq!(cs, n as usize, "randombytes({n}) forwarded {cs}");
            }
            // the same through randombytes_buf itself
            BUF_LAST_SIZE.store(0xdead, AtOrd::SeqCst);
            cb(sink.as_mut_ptr() as *mut c_void, n as usize);
            let cs = BUF_LAST_SIZE.load(AtOrd::SeqCst);
            BUF_LAST_SIZE.store(0xdead, AtOrd::SeqCst);
            rb(sink.as_mut_ptr() as *mut c_void, n as usize);
            let rs = BUF_LAST_SIZE.load(AtOrd::SeqCst);
            assert_eq!(cs, rs, "randombytes_buf({n}): forwarded size differs");
        }
        use_det();
    }
}

// ===========================================================================
// randombytes_uniform — ERRORS rows 644-647
// ===========================================================================

/// Rows 644/645: `upper_bound < 2` returns 0 without consuming randomness.
/// Row 647: `upper_bound == 0x80000001` (the documented worst case,
/// min = 2147483646) still terminates.
/// Also: one step either side of every boundary in the table.
#[test]
fn g6e_uniform_guard_and_worst_case() {
    unsafe {
        set_impl_both(&IMPL_FULL as *const RandombytesImpl);
        let (c, r) = pair::<Uniform>("randombytes_uniform");
        // rows 644/645
        for ub in [0u32, 1] {
            seq_set(&[0xdead_beef, 0x1234_5678]);
            let a = c(ub);
            let ac = RAND_CALLS.load(AtOrd::SeqCst);
            seq_rewind();
            let b = r(ub);
            let bc = RAND_CALLS.load(AtOrd::SeqCst);
            eq_i32(&format!("uniform({ub})"), a as c_int, b as c_int);
            assert_eq!(a, 0, "uniform({ub}) must return 0");
            assert_eq!(ac, 0, "uniform({ub}) consumed randomness in C");
            assert_eq!(bc, 0, "uniform({ub}) consumed randomness in Rust");
        }
        // boundaries and one step past them
        let bounds = [
            2u32,
            3,
            0x7fff_ffff,
            0x8000_0000,
            0x8000_0001,
            0x8000_0002,
            0xffff_fffd,
            0xffff_fffe,
            0xffff_ffff,
        ];
        let mut rng = Rng::new(SEED);
        for &ub in &bounds {
            for _ in 0..16 {
                let feed: Vec<u32> = (0..12).map(|_| rng.next_u32()).collect();
                seq_set(&feed);
                let a = c(ub);
                let ac = RAND_CALLS.load(AtOrd::SeqCst);
                seq_rewind();
                let b = r(ub);
                let bc = RAND_CALLS.load(AtOrd::SeqCst);
                assert_eq!(a, b, "uniform({ub}): C={a} Rust={b}");
                assert_eq!(ac, bc, "uniform({ub}): draw counts differ ({ac} vs {bc})");
                assert!(a < ub, "uniform({ub}) returned {a}");
            }
        }
        // row 647 with the harness' real (non-sequenced) implementation: the
        // rejection loop must terminate for the worst-case bound.
        use_det();
        for _ in 0..200 {
            let a = c(0x8000_0001);
            let b = r(0x8000_0001);
            assert!(a < 0x8000_0001 && b < 0x8000_0001);
        }
    }
}

/// Row 646: a non-NULL `.uniform` hook is delegated to unconditionally — even
/// for `upper_bound < 2`, and without any rejection sampling.
#[test]
fn g6e_uniform_hook_bypasses_guard() {
    unsafe {
        set_impl_both(&IMPL_UNIFORM_HOOK as *const RandombytesImpl);
        let (c, r) = pair::<Uniform>("randombytes_uniform");
        for ub in [0u32, 1, 2, 0x8000_0001, 0xffff_ffff] {
            seq_set(&[0]);
            let a = c(ub);
            let ac = RAND_CALLS.load(AtOrd::SeqCst);
            seq_rewind();
            let b = r(ub);
            let bc = RAND_CALLS.load(AtOrd::SeqCst);
            assert_eq!(a, b, "hooked uniform({ub}): C={a} Rust={b}");
            assert_eq!(a, ub, "hooked uniform({ub}) must return the hook's value");
            assert_eq!(ac, 0);
            assert_eq!(bc, 0);
        }
        use_det();
    }
}

// ===========================================================================
// randombytes_buf / _stir / _close — ERRORS rows 648-651
// ===========================================================================

/// Row 648: `size == 0` short-circuits — the `buf` hook is not called at all
/// and the buffer is untouched (even a NULL buffer is fine).
#[test]
fn g6e_buf_zero_size_short_circuits() {
    unsafe {
        set_impl_both(&IMPL_FULL as *const RandombytesImpl);
        let (c, r) = pair::<Buf>("randombytes_buf");
        let mut probe = [0x3Cu8; 16];
        BUF_CALLS.store(0, AtOrd::SeqCst);
        c(probe.as_mut_ptr() as *mut c_void, 0);
        assert_eq!(BUF_CALLS.load(AtOrd::SeqCst), 0, "C called buf(size=0)");
        r(probe.as_mut_ptr() as *mut c_void, 0);
        assert_eq!(BUF_CALLS.load(AtOrd::SeqCst), 0, "Rust called buf(size=0)");
        assert_eq!(probe, [0x3Cu8; 16], "buf(size=0) modified the buffer");
        c(ptr::null_mut(), 0);
        r(ptr::null_mut(), 0);
        assert_eq!(BUF_CALLS.load(AtOrd::SeqCst), 0);
        // one step past zero really does call the hook
        let mut one = [0u8; 1];
        BUF_CALLS.store(0, AtOrd::SeqCst);
        c(one.as_mut_ptr() as *mut c_void, 1);
        r(one.as_mut_ptr() as *mut c_void, 1);
        assert_eq!(BUF_CALLS.load(AtOrd::SeqCst), 2);
        use_det();
    }
}

/// Rows 649-651: `.stir == NULL` is a no-op, `.close == NULL` returns 0, and
/// `implementation == NULL` makes `randombytes_close` return 0 *without*
/// running `randombytes_init_if_needed()` (so no hook of any kind runs).
#[test]
fn g6e_stir_and_close_null_paths() {
    unsafe {
        let l = libs();
        let (cs, rs_) = pair::<Stir>("randombytes_stir");
        let (cc, rc) = pair::<Close>("randombytes_close");
        // row 649 / 651
        set_impl_both(&IMPL_NO_STIR_CLOSE as *const RandombytesImpl);
        STIR_CALLS.store(0, AtOrd::SeqCst);
        CLOSE_CALLS.store(0, AtOrd::SeqCst);
        cs();
        rs_();
        assert_eq!(STIR_CALLS.load(AtOrd::SeqCst), 0, ".stir == NULL was called");
        let a = cc();
        let b = rc();
        eq_i32("close(.close == NULL)", a, b);
        assert_eq!(a, 0, "close with .close == NULL must return 0");
        assert_eq!(CLOSE_CALLS.load(AtOrd::SeqCst), 0);
        // row 650: implementation == NULL
        for lib in [l.c, l.rs] {
            eq_i32("set_implementation(NULL)", 0, set_impl(lib, ptr::null()));
        }
        BUF_CALLS.store(0, AtOrd::SeqCst);
        STIR_CALLS.store(0, AtOrd::SeqCst);
        CLOSE_CALLS.store(0, AtOrd::SeqCst);
        let a = cc();
        let b = rc();
        eq_i32("close(implementation == NULL)", a, b);
        assert_eq!(a, 0, "close with a NULL implementation must return 0");
        assert_eq!(
            (
                BUF_CALLS.load(AtOrd::SeqCst),
                STIR_CALLS.load(AtOrd::SeqCst),
                CLOSE_CALLS.load(AtOrd::SeqCst)
            ),
            (0, 0, 0),
            "close(NULL implementation) must not run any hook"
        );
        // a second close is still 0, and the implementation is still NULL
        eq_i32("close(NULL) twice", cc(), rc());
        // ... until something else forces the re-init
        assert_eq!(name_of(l.c), "sysrandom");
        assert_eq!(name_of(l.rs), "sysrandom");
        use_det();
    }
}

/// Rows 652-655 (sysrandom close) and 700-703 (internal close): the *reachable*
/// halves. On Linux with `getrandom()` available, `sysrandom`'s close returns 0
/// and is idempotent; `internal`'s close returns -1 until the backend has been
/// stirred at least once (`global.getrandom_available == 0`), then 0 for ever.
#[test]
fn g6e_close_return_values_per_backend() {
    unsafe {
        let l = libs();
        let libv = [l.c, l.rs];
        // fresh-process check for the -1 side (row 700/701)
        let t = diff_abort_case("internal_close_fresh");
        assert_eq!(
            t.code,
            Some(0),
            "fresh internal randombytes_close must return -1 in both: {t:?}"
        );
        // sysrandom: 0, idempotent (row 654)
        for name in [
            "randombytes_sysrandom_implementation",
            "randombytes_internal_implementation",
        ] {
            let mut rets = Vec::new();
            for i in 0..2 {
                let p = impl_addr(libv[i], name);
                eq_i32("set_implementation", 0, set_impl(libv[i], p));
                let stir: Symbol<Stir> = libv[i].get(b"randombytes_stir\0").unwrap();
                let close: Symbol<Close> = libv[i].get(b"randombytes_close\0").unwrap();
                stir();
                rets.push([close(), close(), close()]);
            }
            assert_eq!(
                rets[0], rets[1],
                "{name}: close() sequence differs (C={:?}, Rust={:?})",
                rets[0], rets[1]
            );
            assert_eq!(
                rets[0],
                [0, 0, 0],
                "{name}: close() should be 0 and idempotent once stirred"
            );
        }
        use_det();
    }
}

/// Rows 664/666 (sysrandom) and 675/678/680/682 (internal): the `.buf` hook of
/// a shipped implementation, invoked DIRECTLY with size 0 — something
/// `randombytes_buf()` can never do because it short-circuits (row 648), but
/// the vtable is public API.
///
/// The reference build compiles with assertions enabled (no `NDEBUG`), so for
/// `sysrandom` the 256-byte chunking wrapper trips
/// `assert(chunk_size > (size_t) 0U)` and aborts; the `internal` backend's
/// `.buf` goes through ChaCha20 instead and returns quietly. Both libraries
/// must behave identically.
#[test]
fn g6e_vtable_buf_zero_size() {
    let t = diff_abort_case("vtable_buf_zero_sysrandom");
    assert!(
        t.signal == Some(6) || t.code.map(|c| c != 0).unwrap_or(false),
        "sysrandom .buf(size=0) must abort in both libraries, got {t:?}"
    );
    let t = diff_abort_case("vtable_buf_zero_internal");
    assert_eq!(
        t.code,
        Some(0),
        "internal .buf(size=0) must return quietly in both libraries, got {t:?}"
    );
}

/// Rows 656-698 success side: drive both shipped backends through the chunking
/// loops (`randombytes_linux_getrandom` splits into 256-byte chunks, the
/// `/dev/urandom` fallback through `safe_read`) with sizes far beyond one
/// chunk, plus the `rnd32` pool refill. No error path is reachable here, so the
/// assertion is "no abort, no repetition, both libraries behave alike".
#[test]
fn g6e_entropy_backends_large_requests() {
    unsafe {
        let l = libs();
        let libv = [l.c, l.rs];
        for name in [
            "randombytes_sysrandom_implementation",
            "randombytes_internal_implementation",
        ] {
            for i in 0..2 {
                let p = impl_addr(libv[i], name);
                eq_i32("set_implementation", 0, set_impl(libv[i], p));
                let buf: Symbol<Buf> = libv[i].get(b"randombytes_buf\0").unwrap();
                let rnd: Symbol<Random> = libv[i].get(b"randombytes_random\0").unwrap();
                let mut prev: Option<Vec<u8>> = None;
                for &n in &[1usize, 255, 256, 257, 511, 512, 513, 4096, 65536] {
                    let mut b = vec![0u8; n];
                    buf(b.as_mut_ptr() as *mut c_void, n);
                    // These are statistical smoke checks on REAL kernel
                    // entropy, so they must only be applied where a false
                    // positive is impossible in practice: an all-zero (or
                    // repeated) buffer of >= 16 bytes has probability 2^-128,
                    // whereas a single zero byte happens 1 time in 256 and
                    // would make this test flaky.
                    if n >= 16 {
                        assert!(
                            b.iter().any(|&x| x != 0),
                            "{name}: buf({n}) returned all zeros"
                        );
                        if let Some(p) = &prev {
                            if p.len() == n {
                                assert_ne!(p, &b, "{name}: buf({n}) repeated");
                            }
                        }
                    }
                    prev = Some(b);
                }
                for _ in 0..500 {
                    let _ = rnd();
                }
            }
        }
        use_det();
    }
}

// ===========================================================================
// ML-KEM-768 — ERRORS rows 707-716, 728
// ===========================================================================

/// Rows 707/708: a public key that is not "modulus-check compliant" (some
/// 12-bit coefficient >= 3329) is rejected with -1 by
/// `crypto_kem_mlkem768_enc_deterministic` *before any hashing*, leaving `ct`
/// and `ss` untouched; `crypto_kem_mlkem768_enc` propagates it.
/// Row 709: bytes 1152..1184 (the `publicseed`) are never validated.
#[test]
fn g6e_mlkem_enc_rejects_noncanonical_pk() {
    unsafe {
        use_det();
        let (ckp, rkp) = pair::<SeedKp>("crypto_kem_mlkem768_seed_keypair");
        let (cenc, renc) = pair::<EncDet>("crypto_kem_mlkem768_enc_deterministic");
        let (cencr, rencr) = pair::<Enc>("crypto_kem_mlkem768_enc");
        let seed = [0x13u8; MLK_SEED];
        let eseed = [0x37u8; 32];
        let mut cpk = vec![0u8; MLK_PK];
        let mut csk = vec![0u8; MLK_SK];
        let mut rpk = vec![0u8; MLK_PK];
        let mut rsk = vec![0u8; MLK_SK];
        ckp(cpk.as_mut_ptr(), csk.as_mut_ptr(), seed.as_ptr());
        rkp(rpk.as_mut_ptr(), rsk.as_mut_ptr(), seed.as_ptr());
        eq_bytes("pk", &cpk, &rpk);

        // every 3-byte group packs two 12-bit coefficients; force each of the
        // two in turn to 0xfff (4095 >= 3329), at the start / middle / end of
        // each of the three polynomials.
        let mut bad_pks: Vec<(String, Vec<u8>)> = Vec::new();
        for &base in &[0usize, 3, 381, 384, 576, 1149] {
            let mut p = cpk.clone();
            p[base] = 0xff;
            p[base + 1] = 0xff;
            bad_pks.push((format!("coeff0@{base}"), p));
            let mut p = cpk.clone();
            p[base + 1] = 0xff;
            p[base + 2] = 0xff;
            bad_pks.push((format!("coeff1@{base}"), p));
        }
        bad_pks.push(("all 0xff".into(), vec![0xff; MLK_PK]));
        for (tag, pk) in &bad_pks {
            let mut cct = vec![0xA5u8; MLK_CT];
            let mut css = vec![0xA5u8; SS];
            let mut rct = vec![0xA5u8; MLK_CT];
            let mut rss = vec![0xA5u8; SS];
            let a = cenc(cct.as_mut_ptr(), css.as_mut_ptr(), pk.as_ptr(), eseed.as_ptr());
            let b = renc(rct.as_mut_ptr(), rss.as_mut_ptr(), pk.as_ptr(), eseed.as_ptr());
            eq_i32(&format!("mlkem enc_det({tag})"), a, b);
            assert_eq!(a, -1, "mlkem enc_det({tag}) should reject");
            assert!(
                cct.iter().all(|&x| x == 0xA5) && css.iter().all(|&x| x == 0xA5),
                "C wrote to ct/ss on the rejection path ({tag})"
            );
            eq_bytes(&format!("mlkem enc_det({tag}) ct untouched"), &cct, &rct);
            eq_bytes(&format!("mlkem enc_det({tag}) ss untouched"), &css, &rss);
            // row 708: the randomised entry point propagates -1
            let a = cencr(cct.as_mut_ptr(), css.as_mut_ptr(), pk.as_ptr());
            let b = rencr(rct.as_mut_ptr(), rss.as_mut_ptr(), pk.as_ptr());
            eq_i32(&format!("mlkem enc({tag})"), a, b);
            assert_eq!(a, -1, "mlkem enc({tag}) should reject");
        }
        // row 709: an arbitrary publicseed is accepted
        for fill in [0x00u8, 0xff, 0x5a] {
            let mut pk = cpk.clone();
            for i in 1152..MLK_PK {
                pk[i] = fill;
            }
            let mut cct = vec![0u8; MLK_CT];
            let mut css = vec![0u8; SS];
            let mut rct = vec![0u8; MLK_CT];
            let mut rss = vec![0u8; SS];
            let a = cenc(cct.as_mut_ptr(), css.as_mut_ptr(), pk.as_ptr(), eseed.as_ptr());
            let b = renc(rct.as_mut_ptr(), rss.as_mut_ptr(), pk.as_ptr(), eseed.as_ptr());
            eq_i32(&format!("mlkem enc_det(publicseed={fill:#x})"), a, b);
            assert_eq!(a, 0, "an arbitrary publicseed must be accepted");
            eq_bytes("ct", &cct, &rct);
            eq_bytes("ss", &css, &rss);
        }
        // the largest still-canonical coefficient (3328 = 0xd00) must pass
        {
            let mut pk = cpk.clone();
            pk[0] = 0x00;
            pk[1] = 0x0d; // coeff0 = 0xd00 = 3328 < 3329
            let mut cct = vec![0u8; MLK_CT];
            let mut css = vec![0u8; SS];
            let mut rct = vec![0u8; MLK_CT];
            let mut rss = vec![0u8; SS];
            let a = cenc(cct.as_mut_ptr(), css.as_mut_ptr(), pk.as_ptr(), eseed.as_ptr());
            let b = renc(rct.as_mut_ptr(), rss.as_mut_ptr(), pk.as_ptr(), eseed.as_ptr());
            eq_i32("mlkem enc_det(coeff0=3328)", a, b);
            assert_eq!(a, 0, "coefficient 3328 must be accepted");
            eq_bytes("ct", &cct, &rct);
            eq_bytes("ss", &css, &rss);
            // ... and one step past it (3329) must be rejected
            let mut pk = cpk.clone();
            pk[0] = 0x01;
            pk[1] = 0x0d; // coeff0 = 0xd01 = 3329
            let a = cenc(cct.as_mut_ptr(), css.as_mut_ptr(), pk.as_ptr(), eseed.as_ptr());
            let b = renc(rct.as_mut_ptr(), rss.as_mut_ptr(), pk.as_ptr(), eseed.as_ptr());
            eq_i32("mlkem enc_det(coeff0=3329)", a, b);
            assert_eq!(a, -1, "coefficient 3329 must be rejected");
        }
    }
}

/// Rows 711-713: `crypto_kem_mlkem768_dec` NEVER returns -1. A corrupted
/// ciphertext or a tampered `hpk` silently produces the implicit-rejection
/// secret `SHAKE256(sk[2368..2400] ‖ ct)[0..32]`, which must match byte for
/// byte between C and Rust.
/// Row 714: there is no length check at all; passing anything other than
/// 2400/1088 bytes is an out-of-bounds read, so it is not exercised.
#[test]
fn g6e_mlkem_dec_never_returns_error() {
    unsafe {
        let (ckp, rkp) = pair::<SeedKp>("crypto_kem_mlkem768_seed_keypair");
        let (cenc, renc) = pair::<EncDet>("crypto_kem_mlkem768_enc_deterministic");
        let (cdec, rdec) = pair::<Dec>("crypto_kem_mlkem768_dec");
        let mut rng = Rng::new(SEED ^ 0x711);
        for round in 0..3 {
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
            let mut ct = vec![0u8; MLK_CT];
            let mut ss = vec![0u8; SS];
            cenc(ct.as_mut_ptr(), ss.as_mut_ptr(), cpk.as_ptr(), eseed.as_ptr());
            // row 712: the honest path
            let mut css = vec![0u8; SS];
            let mut rss = vec![0u8; SS];
            let a = cdec(css.as_mut_ptr(), ct.as_ptr(), csk.as_ptr());
            let b = rdec(rss.as_mut_ptr(), ct.as_ptr(), rsk.as_ptr());
            eq_i32("dec(valid)", a, b);
            assert_eq!(a, 0);
            eq_bytes("dec(valid) ss", &css, &rss);
            assert_eq!(css, ss, "honest dec != enc secret");
            // row 711: many corruptions
            for k in 0..12 {
                let pos = rng.below(MLK_CT);
                let mut bad = ct.clone();
                bad[pos] ^= 1u8 << (k % 8);
                let mut css = vec![0u8; SS];
                let mut rss = vec![0u8; SS];
                let a = cdec(css.as_mut_ptr(), bad.as_ptr(), csk.as_ptr());
                let b = rdec(rss.as_mut_ptr(), bad.as_ptr(), rsk.as_ptr());
                eq_i32(&format!("dec(corrupt r{round}@{pos})"), a, b);
                assert_eq!(a, 0, "mlkem768_dec must never return -1");
                eq_bytes(&format!("dec(corrupt r{round}@{pos}) ss"), &css, &rss);
                assert_ne!(css, ss, "corrupted ct produced the honest secret");
            }
            // row 713: inconsistent hpk
            for (tag, patch) in [("zeroed", 0x00u8), ("ones", 0xff)] {
                let mut csk2 = csk.clone();
                let mut rsk2 = rsk.clone();
                for i in 2336..2368 {
                    csk2[i] = patch;
                    rsk2[i] = patch;
                }
                let mut css = vec![0u8; SS];
                let mut rss = vec![0u8; SS];
                let a = cdec(css.as_mut_ptr(), ct.as_ptr(), csk2.as_ptr());
                let b = rdec(rss.as_mut_ptr(), ct.as_ptr(), rsk2.as_ptr());
                eq_i32(&format!("dec(hpk {tag})"), a, b);
                assert_eq!(a, 0, "a tampered hpk must not be reported as an error");
                eq_bytes(&format!("dec(hpk {tag}) ss"), &css, &rss);
                assert_ne!(css, ss);
            }
            // a fully random sk is also "valid" (no consistency check exists)
            let mut csk3 = vec![0u8; MLK_SK];
            rng.fill(&mut csk3);
            let mut css = vec![0u8; SS];
            let mut rss = vec![0u8; SS];
            let a = cdec(css.as_mut_ptr(), ct.as_ptr(), csk3.as_ptr());
            let b = rdec(rss.as_mut_ptr(), ct.as_ptr(), csk3.as_ptr());
            eq_i32("dec(random sk)", a, b);
            assert_eq!(a, 0);
            eq_bytes("dec(random sk) ss", &css, &rss);
            let _ = renc;
        }
    }
}

/// Rows 715/716: every 64-byte seed is accepted and `_keypair` cannot fail.
#[test]
fn g6e_mlkem_keypair_never_fails() {
    unsafe {
        use_det();
        let (ckp, rkp) = pair::<SeedKp>("crypto_kem_mlkem768_seed_keypair");
        let (ck, rk) = pair::<Kp>("crypto_kem_mlkem768_keypair");
        let mut seeds: Vec<[u8; MLK_SEED]> = vec![[0x00; 64], [0xff; 64]];
        let mut s = [0u8; 64];
        s.copy_from_slice(&incr(64));
        seeds.push(s);
        let mut rng = Rng::new(SEED ^ 0x715);
        for _ in 0..4 {
            let mut s = [0u8; 64];
            rng.fill(&mut s);
            seeds.push(s);
        }
        for (i, seed) in seeds.iter().enumerate() {
            let mut cpk = vec![0u8; MLK_PK];
            let mut csk = vec![0u8; MLK_SK];
            let mut rpk = vec![0u8; MLK_PK];
            let mut rsk = vec![0u8; MLK_SK];
            let a = ckp(cpk.as_mut_ptr(), csk.as_mut_ptr(), seed.as_ptr());
            let b = rkp(rpk.as_mut_ptr(), rsk.as_mut_ptr(), seed.as_ptr());
            eq_i32(&format!("seed_keypair#{i}"), a, b);
            assert_eq!(a, 0, "mlkem768_seed_keypair must always return 0");
            eq_bytes(&format!("seed_keypair#{i} pk"), &cpk, &rpk);
            eq_bytes(&format!("seed_keypair#{i} sk"), &csk, &rsk);
        }
        for i in 0..3u64 {
            let mut cpk = vec![0u8; MLK_PK];
            let mut csk = vec![0u8; MLK_SK];
            let mut rpk = vec![0u8; MLK_PK];
            let mut rsk = vec![0u8; MLK_SK];
            det_reseed(0x9000 + i);
            let a = ck(cpk.as_mut_ptr(), csk.as_mut_ptr());
            det_reseed(0x9000 + i);
            let b = rk(rpk.as_mut_ptr(), rsk.as_mut_ptr());
            eq_i32("mlkem768_keypair", a, b);
            assert_eq!(a, 0);
            eq_bytes("keypair pk", &cpk, &rpk);
            eq_bytes("keypair sk", &csk, &rsk);
        }
    }
}

// ===========================================================================
// X-Wing — ERRORS rows 717-726
// ===========================================================================

/// Rows 717/718/720: the two reachable `-1` causes of
/// `crypto_kem_xwing_enc_deterministic` (a non-canonical ML-KEM public key, and
/// an X25519 public key that makes the shared secret all-zero), both propagated
/// by `crypto_kem_xwing_enc` and by the generic `crypto_kem_enc`.
/// Row 719: the 64-byte seed length is not checked (OOB read, not exercised).
#[test]
fn g6e_xwing_enc_rejections() {
    unsafe {
        use_det();
        let (ckp, rkp) = pair::<SeedKp>("crypto_kem_xwing_seed_keypair");
        let (cenc, renc) = pair::<EncDet>("crypto_kem_xwing_enc_deterministic");
        let (cencr, rencr) = pair::<Enc>("crypto_kem_xwing_enc");
        let (cgen, rgen) = pair::<Enc>("crypto_kem_enc");
        let seed = [0x24u8; XW_SK];
        let eseed = [0x99u8; XW_ENC_SEED];
        let mut cpk = vec![0u8; XW_PK];
        let mut csk = vec![0u8; XW_SK];
        let mut rpk = vec![0u8; XW_PK];
        let mut rsk = vec![0u8; XW_SK];
        ckp(cpk.as_mut_ptr(), csk.as_mut_ptr(), seed.as_ptr());
        rkp(rpk.as_mut_ptr(), rsk.as_mut_ptr(), seed.as_ptr());
        eq_bytes("xwing pk", &cpk, &rpk);

        let mut cases: Vec<(String, Vec<u8>, bool)> = Vec::new();
        // row 717: non-canonical ML-KEM half
        for &base in &[0usize, 384, 1149] {
            let mut p = cpk.clone();
            p[base] = 0xff;
            p[base + 1] = 0xff;
            cases.push((format!("noncanonical mlkem@{base}"), p, true));
        }
        // row 718: low-order X25519 half
        for (i, low) in LOW_ORDER.iter().enumerate() {
            let mut p = cpk.clone();
            p[1184..1216].copy_from_slice(low);
            cases.push((format!("low-order x25519 #{i}"), p, false));
        }
        for (tag, pk, must_fail) in &cases {
            let mut cct = vec![0xC3u8; XW_CT];
            let mut css = vec![0xC3u8; SS];
            let mut rct = vec![0xC3u8; XW_CT];
            let mut rss = vec![0xC3u8; SS];
            let a = cenc(cct.as_mut_ptr(), css.as_mut_ptr(), pk.as_ptr(), eseed.as_ptr());
            let b = renc(rct.as_mut_ptr(), rss.as_mut_ptr(), pk.as_ptr(), eseed.as_ptr());
            eq_i32(&format!("xwing enc_det({tag})"), a, b);
            if *must_fail {
                assert_eq!(a, -1, "xwing enc_det({tag}) should reject");
            }
            if a == -1 {
                assert!(
                    css.iter().all(|&x| x == 0xC3),
                    "C wrote ss on the rejection path ({tag})"
                );
            }
            eq_bytes(&format!("xwing enc_det({tag}) ct"), &cct, &rct);
            eq_bytes(&format!("xwing enc_det({tag}) ss"), &css, &rss);
            // row 720: the randomised and the generic entry points propagate it
            det_reseed(0xAB01);
            let a2 = cencr(cct.as_mut_ptr(), css.as_mut_ptr(), pk.as_ptr());
            det_reseed(0xAB01);
            let b2 = rencr(rct.as_mut_ptr(), rss.as_mut_ptr(), pk.as_ptr());
            eq_i32(&format!("xwing enc({tag})"), a2, b2);
            assert_eq!(a2, a, "xwing enc did not propagate enc_deterministic");
            det_reseed(0xAB02);
            let a3 = cgen(cct.as_mut_ptr(), css.as_mut_ptr(), pk.as_ptr());
            det_reseed(0xAB02);
            let b3 = rgen(rct.as_mut_ptr(), rss.as_mut_ptr(), pk.as_ptr());
            eq_i32(&format!("crypto_kem_enc({tag})"), a3, b3);
            assert_eq!(a3, a, "crypto_kem_enc did not propagate the failure");
        }
        // at least the all-zero X25519 point must really be a rejection
        {
            let mut p = cpk.clone();
            p[1184..1216].copy_from_slice(&[0u8; 32]);
            let mut cct = vec![0u8; XW_CT];
            let mut css = vec![0u8; SS];
            assert_eq!(
                cenc(cct.as_mut_ptr(), css.as_mut_ptr(), p.as_ptr(), eseed.as_ptr()),
                -1,
                "an all-zero X25519 public key must be rejected"
            );
        }
    }
}

/// Row 722: `crypto_kem_xwing_dec` (and the generic `crypto_kem_dec`) returns
/// -1 when `ct[1088..1120]` is a low-order point, because the X25519 shared
/// secret becomes all-zero. Row 723: corrupting only the ML-KEM part is NOT an
/// error — it yields a deterministic secret derived from the implicit-rejection
/// value. Row 721 (`crypto_kem_mlkem768_dec != 0`) is unreachable, since that
/// function never fails.
#[test]
fn g6e_xwing_dec_rejections() {
    unsafe {
        let (ckp, rkp) = pair::<SeedKp>("crypto_kem_xwing_seed_keypair");
        let (cenc, renc) = pair::<EncDet>("crypto_kem_xwing_enc_deterministic");
        let (cdec, rdec) = pair::<Dec>("crypto_kem_xwing_dec");
        let (cgen, rgen) = pair::<Dec>("crypto_kem_dec");
        let seed = [0x68u8; XW_SK];
        let eseed = [0x1du8; XW_ENC_SEED];
        let mut cpk = vec![0u8; XW_PK];
        let mut csk = vec![0u8; XW_SK];
        let mut rpk = vec![0u8; XW_PK];
        let mut rsk = vec![0u8; XW_SK];
        ckp(cpk.as_mut_ptr(), csk.as_mut_ptr(), seed.as_ptr());
        rkp(rpk.as_mut_ptr(), rsk.as_mut_ptr(), seed.as_ptr());
        let mut ct = vec![0u8; XW_CT];
        let mut ss = vec![0u8; SS];
        assert_eq!(
            cenc(ct.as_mut_ptr(), ss.as_mut_ptr(), cpk.as_ptr(), eseed.as_ptr()),
            0
        );
        {
            let mut rct = vec![0u8; XW_CT];
            let mut rss = vec![0u8; SS];
            renc(rct.as_mut_ptr(), rss.as_mut_ptr(), rpk.as_ptr(), eseed.as_ptr());
            eq_bytes("xwing ct", &ct, &rct);
            eq_bytes("xwing ss", &ss, &rss);
        }
        // row 722
        for (i, low) in LOW_ORDER.iter().enumerate() {
            let mut bad = ct.clone();
            bad[1088..1120].copy_from_slice(low);
            let mut css = vec![0x7Eu8; SS];
            let mut rss = vec![0x7Eu8; SS];
            let a = cdec(css.as_mut_ptr(), bad.as_ptr(), csk.as_ptr());
            let b = rdec(rss.as_mut_ptr(), bad.as_ptr(), rsk.as_ptr());
            eq_i32(&format!("xwing dec(low-order #{i})"), a, b);
            eq_bytes(&format!("xwing dec(low-order #{i}) ss"), &css, &rss);
            if a == -1 {
                assert!(
                    css.iter().all(|&x| x == 0x7E),
                    "C wrote ss on the rejection path"
                );
            }
            let a2 = cgen(css.as_mut_ptr(), bad.as_ptr(), csk.as_ptr());
            let b2 = rgen(rss.as_mut_ptr(), bad.as_ptr(), rsk.as_ptr());
            eq_i32(&format!("crypto_kem_dec(low-order #{i})"), a2, b2);
            assert_eq!(a2, a, "crypto_kem_dec != crypto_kem_xwing_dec");
        }
        // the all-zero point must definitely be rejected
        {
            let mut bad = ct.clone();
            for i in 1088..1120 {
                bad[i] = 0;
            }
            let mut css = vec![0u8; SS];
            assert_eq!(
                cdec(css.as_mut_ptr(), bad.as_ptr(), csk.as_ptr()),
                -1,
                "an all-zero X25519 ciphertext must be rejected"
            );
        }
        // row 723: corrupting only the ML-KEM part is not an error
        let mut rng = Rng::new(SEED ^ 0x723);
        let mut seen: Vec<Vec<u8>> = vec![ss.clone()];
        for _ in 0..10 {
            let pos = rng.below(1088);
            let mut bad = ct.clone();
            bad[pos] ^= 0x80;
            let mut css = vec![0u8; SS];
            let mut rss = vec![0u8; SS];
            let a = cdec(css.as_mut_ptr(), bad.as_ptr(), csk.as_ptr());
            let b = rdec(rss.as_mut_ptr(), bad.as_ptr(), rsk.as_ptr());
            eq_i32(&format!("xwing dec(mlkem corrupt@{pos})"), a, b);
            assert_eq!(a, 0, "corrupting the ML-KEM half must NOT be an error");
            eq_bytes(&format!("xwing dec(mlkem corrupt@{pos}) ss"), &css, &rss);
            for prev in &seen {
                assert_ne!(&css, prev, "implicit-rejection secret repeated");
            }
            seen.push(css);
        }
    }
}

/// Rows 724-726: every 32-byte seed and every 32-byte secret key is "valid" —
/// the decapsulation key is re-expanded from `sk` on each call, so no rejection
/// is possible, and the discarded inner return values are always 0.
#[test]
fn g6e_xwing_accepts_any_seed_and_sk() {
    unsafe {
        use_det();
        let (ckp, rkp) = pair::<SeedKp>("crypto_kem_xwing_seed_keypair");
        let (cgkp, rgkp) = pair::<SeedKp>("crypto_kem_seed_keypair");
        let (ck, rk) = pair::<Kp>("crypto_kem_xwing_keypair");
        let (cgk, rgk) = pair::<Kp>("crypto_kem_keypair");
        let (cenc, renc) = pair::<EncDet>("crypto_kem_xwing_enc_deterministic");
        let (cdec, rdec) = pair::<Dec>("crypto_kem_xwing_dec");
        let mut seeds: Vec<[u8; 32]> = vec![[0x00; 32], [0xff; 32]];
        let mut s = [0u8; 32];
        s.copy_from_slice(&incr(32));
        seeds.push(s);
        let mut rng = Rng::new(SEED ^ 0x724);
        for _ in 0..4 {
            let mut s = [0u8; 32];
            rng.fill(&mut s);
            seeds.push(s);
        }
        for (i, seed) in seeds.iter().enumerate() {
            let mut cpk = vec![0u8; XW_PK];
            let mut csk = vec![0u8; XW_SK];
            let mut rpk = vec![0u8; XW_PK];
            let mut rsk = vec![0u8; XW_SK];
            let a = ckp(cpk.as_mut_ptr(), csk.as_mut_ptr(), seed.as_ptr());
            let b = rkp(rpk.as_mut_ptr(), rsk.as_mut_ptr(), seed.as_ptr());
            eq_i32(&format!("xwing seed_keypair#{i}"), a, b);
            assert_eq!(a, 0, "xwing_seed_keypair must always return 0");
            eq_bytes(&format!("xwing seed_keypair#{i} pk"), &cpk, &rpk);
            eq_bytes(&format!("xwing seed_keypair#{i} sk"), &csk, &rsk);
            let a = cgkp(cpk.as_mut_ptr(), csk.as_mut_ptr(), seed.as_ptr());
            let b = rgkp(rpk.as_mut_ptr(), rsk.as_mut_ptr(), seed.as_ptr());
            eq_i32("crypto_kem_seed_keypair", a, b);
            assert_eq!(a, 0);
            // row 726: decapsulate with a *random* (unrelated) 32-byte sk
            let mut ct = vec![0u8; XW_CT];
            let mut ss = vec![0u8; SS];
            assert_eq!(
                cenc(
                    ct.as_mut_ptr(),
                    ss.as_mut_ptr(),
                    cpk.as_ptr(),
                    [0x42u8; XW_ENC_SEED].as_ptr()
                ),
                0
            );
            let mut alien = [0u8; 32];
            rng.fill(&mut alien);
            let mut css = vec![0u8; SS];
            let mut rss = vec![0u8; SS];
            let a = cdec(css.as_mut_ptr(), ct.as_ptr(), alien.as_ptr());
            let b = rdec(rss.as_mut_ptr(), ct.as_ptr(), alien.as_ptr());
            eq_i32("xwing dec(alien sk)", a, b);
            assert_eq!(a, 0, "any 32-byte sk must be accepted");
            eq_bytes("xwing dec(alien sk) ss", &css, &rss);
            let _ = renc;
        }
        // row 725: `_keypair` always returns 0
        for i in 0..3u64 {
            let mut cpk = vec![0u8; XW_PK];
            let mut csk = vec![0u8; XW_SK];
            let mut rpk = vec![0u8; XW_PK];
            let mut rsk = vec![0u8; XW_SK];
            det_reseed(0xA000 + i);
            let a = ck(cpk.as_mut_ptr(), csk.as_mut_ptr());
            det_reseed(0xA000 + i);
            let b = rk(rpk.as_mut_ptr(), rsk.as_mut_ptr());
            eq_i32("xwing_keypair", a, b);
            assert_eq!(a, 0);
            eq_bytes("xwing_keypair pk", &cpk, &rpk);
            det_reseed(0xA100 + i);
            let a = cgk(cpk.as_mut_ptr(), csk.as_mut_ptr());
            det_reseed(0xA100 + i);
            let b = rgk(rpk.as_mut_ptr(), rsk.as_mut_ptr());
            eq_i32("crypto_kem_keypair", a, b);
            assert_eq!(a, 0);
            eq_bytes("crypto_kem_keypair pk", &cpk, &rpk);
            eq_bytes("crypto_kem_keypair sk", &csk, &rsk);
        }
    }
}

/// Rows 727/728: the constant getters cannot fail.
#[test]
fn g6e_kem_getters_cannot_fail() {
    unsafe {
        for (name, want) in [
            ("crypto_kem_publickeybytes", 1216usize),
            ("crypto_kem_secretkeybytes", 32),
            ("crypto_kem_ciphertextbytes", 1120),
            ("crypto_kem_sharedsecretbytes", 32),
            ("crypto_kem_seedbytes", 32),
            ("crypto_kem_mlkem768_publickeybytes", 1184),
            ("crypto_kem_mlkem768_secretkeybytes", 2400),
            ("crypto_kem_mlkem768_ciphertextbytes", 1088),
            ("crypto_kem_mlkem768_sharedsecretbytes", 32),
            ("crypto_kem_mlkem768_seedbytes", 64),
        ] {
            let (c, r) = pair::<SizeGet>(name);
            for _ in 0..3 {
                let a = c();
                let b = r();
                assert_eq!(a, want, "C {name}");
                assert_eq!(a, b, "{name} differs (C={a}, Rust={b})");
            }
        }
        let (cp, rp) = pair::<NameGet>("crypto_kem_primitive");
        let a = CStr::from_ptr(cp()).to_string_lossy().into_owned();
        let b = CStr::from_ptr(rp()).to_string_lossy().into_owned();
        assert_eq!(a, "xwing");
        assert_eq!(a, b);
    }
}

// ===========================================================================
// crypto_ipcrypt — ERRORS rows 729-737
// ===========================================================================

/// Rows 729-731 + 735-737: `crypto_ipcrypt` has no error path whatsoever —
/// every entry point returns `void`, there is no length/key validation, and the
/// getters are constants. All that can be compared is that both libraries
/// produce the same bytes for every input class (including the all-zero key)
/// and never write outside the documented output length.
#[test]
fn g6e_ipcrypt_has_no_error_paths() {
    unsafe {
        let (ce, re) = pair::<Ip3>("crypto_ipcrypt_encrypt");
        let (cd, rd) = pair::<Ip3>("crypto_ipcrypt_decrypt");
        let (cnde, rnde) = pair::<Ip4>("crypto_ipcrypt_nd_encrypt");
        let (cndd, rndd) = pair::<Ip3>("crypto_ipcrypt_nd_decrypt");
        let (cnxe, rnxe) = pair::<Ip4>("crypto_ipcrypt_ndx_encrypt");
        let (cnxd, rnxd) = pair::<Ip3>("crypto_ipcrypt_ndx_decrypt");
        let (cpe, rpe) = pair::<Ip3>("crypto_ipcrypt_pfx_encrypt");
        let (cpd, rpd) = pair::<Ip3>("crypto_ipcrypt_pfx_decrypt");
        let mut rng = Rng::new(SEED ^ 0x729);
        let ips: [[u8; 16]; 4] = [
            [0x00; 16],
            [0xff; 16],
            ipv4_mapped(127, 0, 0, 1),
            {
                let mut b = [0u8; 16];
                rng.fill(&mut b);
                b
            },
        ];
        let k16s: [[u8; 16]; 3] = [[0x00; 16], [0xff; 16], {
            let mut b = [0u8; 16];
            rng.fill(&mut b);
            b
        }];
        let k32s: [[u8; 32]; 3] = [[0x00; 32], [0xff; 32], {
            let mut b = [0u8; 32];
            rng.fill(&mut b);
            b
        }];
        let t8s: [[u8; 8]; 2] = [[0x00; 8], [0xff; 8]];
        let t16s: [[u8; 16]; 2] = [[0x00; 16], [0xff; 16]];
        for ip in &ips {
            for k in &k16s {
                for (f, g, tag) in [(&ce, &re, "encrypt"), (&cd, &rd, "decrypt")] {
                    let mut a = [0x11u8; 16 + 8];
                    let mut b = [0x11u8; 16 + 8];
                    f(a.as_mut_ptr(), ip.as_ptr(), k.as_ptr());
                    g(b.as_mut_ptr(), ip.as_ptr(), k.as_ptr());
                    eq_bytes(&format!("ipcrypt_{tag}"), &a, &b);
                    assert!(
                        a[16..].iter().all(|&x| x == 0x11),
                        "ipcrypt_{tag} wrote past 16 bytes"
                    );
                }
                for t in &t8s {
                    let mut a = [0x11u8; 24 + 8];
                    let mut b = [0x11u8; 24 + 8];
                    cnde(a.as_mut_ptr(), ip.as_ptr(), t.as_ptr(), k.as_ptr());
                    rnde(b.as_mut_ptr(), ip.as_ptr(), t.as_ptr(), k.as_ptr());
                    eq_bytes("nd_encrypt", &a, &b);
                    assert!(
                        a[24..].iter().all(|&x| x == 0x11),
                        "nd_encrypt wrote past 24 bytes"
                    );
                    let mut a2 = [0x11u8; 16 + 8];
                    let mut b2 = [0x11u8; 16 + 8];
                    cndd(a2.as_mut_ptr(), a.as_ptr(), k.as_ptr());
                    rndd(b2.as_mut_ptr(), b.as_ptr(), k.as_ptr());
                    eq_bytes("nd_decrypt", &a2, &b2);
                    assert!(
                        a2[16..].iter().all(|&x| x == 0x11),
                        "nd_decrypt wrote past 16 bytes"
                    );
                    assert_eq!(&a2[..16], &ip[..]);
                }
            }
            for k in &k32s {
                for t in &t16s {
                    let mut a = [0x11u8; 32 + 8];
                    let mut b = [0x11u8; 32 + 8];
                    cnxe(a.as_mut_ptr(), ip.as_ptr(), t.as_ptr(), k.as_ptr());
                    rnxe(b.as_mut_ptr(), ip.as_ptr(), t.as_ptr(), k.as_ptr());
                    eq_bytes("ndx_encrypt", &a, &b);
                    assert!(
                        a[32..].iter().all(|&x| x == 0x11),
                        "ndx_encrypt wrote past 32 bytes"
                    );
                    let mut a2 = [0x11u8; 16 + 8];
                    let mut b2 = [0x11u8; 16 + 8];
                    cnxd(a2.as_mut_ptr(), a.as_ptr(), k.as_ptr());
                    rnxd(b2.as_mut_ptr(), b.as_ptr(), k.as_ptr());
                    eq_bytes("ndx_decrypt", &a2, &b2);
                    assert!(a2[16..].iter().all(|&x| x == 0x11));
                    assert_eq!(&a2[..16], &ip[..]);
                }
                let mut a = [0x11u8; 16 + 8];
                let mut b = [0x11u8; 16 + 8];
                cpe(a.as_mut_ptr(), ip.as_ptr(), k.as_ptr());
                rpe(b.as_mut_ptr(), ip.as_ptr(), k.as_ptr());
                eq_bytes("pfx_encrypt", &a, &b);
                assert!(
                    a[16..].iter().all(|&x| x == 0x11),
                    "pfx_encrypt wrote past 16 bytes"
                );
                let mut a2 = [0x11u8; 16 + 8];
                let mut b2 = [0x11u8; 16 + 8];
                cpd(a2.as_mut_ptr(), a.as_ptr(), k.as_ptr());
                rpd(b2.as_mut_ptr(), b.as_ptr(), k.as_ptr());
                eq_bytes("pfx_decrypt", &a2, &b2);
                assert_eq!(&a2[..16], &ip[..]);
            }
        }
        // row 736: `_crypto_ipcrypt_pick_best_implementation` always returns 0
        let (cp, rp) = pair::<PickBest>("_crypto_ipcrypt_pick_best_implementation");
        for _ in 0..3 {
            let a = cp();
            let b = rp();
            eq_i32("pick_best_implementation", a, b);
            assert_eq!(a, 0);
        }
        // row 737: the getters
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
        // row 735: the keygens are `void` and delegate to randombytes_buf
        use_det();
        for (name, len) in [
            ("crypto_ipcrypt_keygen", 16usize),
            ("crypto_ipcrypt_nd_keygen", 16),
            ("crypto_ipcrypt_ndx_keygen", 32),
            ("crypto_ipcrypt_pfx_keygen", 32),
        ] {
            let (c, r) = pair::<Keygen>(name);
            let mut a = vec![0x11u8; len + 8];
            let mut b = vec![0x11u8; len + 8];
            det_reseed(0xB000);
            c(a.as_mut_ptr());
            det_reseed(0xB000);
            r(b.as_mut_ptr());
            eq_bytes(name, &a, &b);
            assert!(
                a[len..].iter().all(|&x| x == 0x11),
                "{name} wrote past {len} bytes"
            );
        }
    }
}

/// Rows 732/733: the degenerate-key branch (`expand_key(k)[5] ==
/// expand_key(k+16)[5]`, in particular `k[0..16] == k[16..32]`) is not an
/// error — the second schedule is silently re-derived from `k[i] ^ 0x5a`. Both
/// libraries must take the branch for exactly the same keys, which shows up as
/// byte-identical output for the degenerate keys and as a *different* output
/// from a neighbouring non-degenerate key.
#[test]
fn g6e_ipcrypt_degenerate_key_branch() {
    unsafe {
        let (cnxe, rnxe) = pair::<Ip4>("crypto_ipcrypt_ndx_encrypt");
        let (cnxd, rnxd) = pair::<Ip3>("crypto_ipcrypt_ndx_decrypt");
        let (cpe, rpe) = pair::<Ip3>("crypto_ipcrypt_pfx_encrypt");
        let (cpd, rpd) = pair::<Ip3>("crypto_ipcrypt_pfx_decrypt");
        let ips = [
            ipv4_mapped(192, 168, 1, 1),
            [0x00u8; 16],
            [0xffu8; 16],
            [
                0x20, 0x01, 0x0d, 0xb8, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1,
            ],
        ];
        let tweak = [0x5cu8; 16];
        let mut rng = Rng::new(SEED ^ 0x732);
        let mut halves: Vec<[u8; 16]> = vec![[0x00; 16], [0xff; 16], [0x5a; 16]];
        let mut h = [0u8; 16];
        h.copy_from_slice(&incr(16));
        halves.push(h);
        for _ in 0..3 {
            let mut h = [0u8; 16];
            rng.fill(&mut h);
            halves.push(h);
        }
        for half in &halves {
            let mut kdeg = [0u8; 32];
            kdeg[..16].copy_from_slice(half);
            kdeg[16..].copy_from_slice(half);
            // a non-degenerate neighbour: flip one bit of the second half
            let mut knear = kdeg;
            knear[31] ^= 0x01;
            for ip in &ips {
                let mut a = [0u8; 32];
                let mut b = [0u8; 32];
                cnxe(a.as_mut_ptr(), ip.as_ptr(), tweak.as_ptr(), kdeg.as_ptr());
                rnxe(b.as_mut_ptr(), ip.as_ptr(), tweak.as_ptr(), kdeg.as_ptr());
                eq_bytes("ndx_encrypt(degenerate key)", &a, &b);
                let mut a2 = [0u8; 32];
                let mut b2 = [0u8; 32];
                cnxe(a2.as_mut_ptr(), ip.as_ptr(), tweak.as_ptr(), knear.as_ptr());
                rnxe(b2.as_mut_ptr(), ip.as_ptr(), tweak.as_ptr(), knear.as_ptr());
                eq_bytes("ndx_encrypt(neighbour key)", &a2, &b2);
                assert_ne!(a, a2, "the degenerate branch changed nothing");
                // and the round trip still works through the same branch
                let mut r1 = [0u8; 16];
                let mut r2 = [0u8; 16];
                cnxd(r1.as_mut_ptr(), a.as_ptr(), kdeg.as_ptr());
                rnxd(r2.as_mut_ptr(), b.as_ptr(), kdeg.as_ptr());
                eq_bytes("ndx_decrypt(degenerate key)", &r1, &r2);
                assert_eq!(&r1[..], &ip[..], "degenerate-key ndx round trip failed");
                // pfx: same branch, 16-byte output
                let mut p1 = [0u8; 16];
                let mut p2 = [0u8; 16];
                cpe(p1.as_mut_ptr(), ip.as_ptr(), kdeg.as_ptr());
                rpe(p2.as_mut_ptr(), ip.as_ptr(), kdeg.as_ptr());
                eq_bytes("pfx_encrypt(degenerate key)", &p1, &p2);
                let mut q1 = [0u8; 16];
                let mut q2 = [0u8; 16];
                cpe(q1.as_mut_ptr(), ip.as_ptr(), knear.as_ptr());
                rpe(q2.as_mut_ptr(), ip.as_ptr(), knear.as_ptr());
                eq_bytes("pfx_encrypt(neighbour key)", &q1, &q2);
                assert_ne!(p1, q1, "the pfx degenerate branch changed nothing");
                let mut d1 = [0u8; 16];
                let mut d2 = [0u8; 16];
                cpd(d1.as_mut_ptr(), p1.as_ptr(), kdeg.as_ptr());
                rpd(d2.as_mut_ptr(), p2.as_ptr(), kdeg.as_ptr());
                eq_bytes("pfx_decrypt(degenerate key)", &d1, &d2);
                assert_eq!(&d1[..], &ip[..], "degenerate-key pfx round trip failed");
            }
        }
    }
}

/// Row 734: `pfx_encrypt`/`pfx_decrypt` treat a non-IPv4-mapped input as a full
/// 128-bit prefix (`prefix_start = 0`); for an IPv4-mapped input
/// (`00*10 ff ff`) the mapped prefix is forced into the output. Every one-byte
/// deviation from the mapped prefix must switch branches.
#[test]
fn g6e_ipcrypt_pfx_ipv4_mapped_detection() {
    unsafe {
        let (ce, re) = pair::<Ip3>("crypto_ipcrypt_pfx_encrypt");
        let (cd, rd) = pair::<Ip3>("crypto_ipcrypt_pfx_decrypt");
        let mut k = [0u8; 32];
        k.copy_from_slice(&incr(32));
        let mapped = ipv4_mapped(203, 0, 113, 5);
        // every single-byte deviation inside the 12-byte prefix leaves the
        // mapped class
        let mut cases: Vec<(String, [u8; 16], bool)> = vec![("mapped".into(), mapped, true)];
        for i in 0..12usize {
            let mut ip = mapped;
            ip[i] ^= 0x01;
            cases.push((format!("prefix byte {i} flipped"), ip, false));
        }
        // a 16-byte input that only *looks* close: prefix bytes 10/11 = fe ff
        {
            let mut ip = mapped;
            ip[10] = 0xfe;
            cases.push(("10=0xfe".into(), ip, false));
            let mut ip = mapped;
            ip[11] = 0x00;
            cases.push(("11=0x00".into(), ip, false));
        }
        for (tag, ip, is_mapped) in &cases {
            let mut a = [0u8; 16];
            let mut b = [0u8; 16];
            ce(a.as_mut_ptr(), ip.as_ptr(), k.as_ptr());
            re(b.as_mut_ptr(), ip.as_ptr(), k.as_ptr());
            eq_bytes(&format!("pfx_encrypt({tag})"), &a, &b);
            if *is_mapped {
                assert!(
                    a[..10].iter().all(|&x| x == 0) && a[10] == 0xff && a[11] == 0xff,
                    "pfx_encrypt({tag}) did not preserve the IPv4-mapped prefix"
                );
            } else {
                // prefix_start == 0: the whole 128-bit value is processed, so
                // the output almost certainly does not look IPv4-mapped
                assert!(
                    !(a[..10].iter().all(|&x| x == 0) && a[10] == 0xff && a[11] == 0xff),
                    "pfx_encrypt({tag}) unexpectedly took the mapped branch"
                );
            }
            let mut c1 = [0u8; 16];
            let mut r1 = [0u8; 16];
            cd(c1.as_mut_ptr(), a.as_ptr(), k.as_ptr());
            rd(r1.as_mut_ptr(), b.as_ptr(), k.as_ptr());
            eq_bytes(&format!("pfx_decrypt({tag})"), &c1, &r1);
            assert_eq!(&c1[..], &ip[..], "pfx round trip failed for {tag}");
        }
    }
}

// ===========================================================================
// absent entry points — ERRORS rows 738, 739
// ===========================================================================

/// Row 738: there is no IP-text parsing API in libsodium 1.0.23.
/// Row 739: `randombytes_random_stir` / `randombytes_random_close` do not
/// exist; the public API is `randombytes_stir` (void) and `randombytes_close`
/// (int).
#[test]
fn g6e_absent_entry_points() {
    for name in [
        "crypto_ipcrypt_str_to_ip16",
        "crypto_ipcrypt_ip16_to_str",
        "crypto_ipcrypt_encrypt_str",
        "crypto_ipcrypt_decrypt_str",
        "randombytes_random_stir",
        "randombytes_random_close",
        "randombytes_salsa20_implementation",
        "crypto_core_ed25519_from_uniform",
    ] {
        let l = libs();
        let mut n = name.as_bytes().to_vec();
        n.push(0);
        let inc = unsafe { l.c.get::<*const ()>(&n).is_ok() };
        let inr = unsafe { l.rs.get::<*const ()>(&n).is_ok() };
        assert!(!inc, "{name} unexpectedly exists in the C library");
        assert_eq!(
            inc, inr,
            "{name}: presence differs (C={inc}, Rust={inr})"
        );
    }
    // the real entry points do exist, with the documented signatures
    assert!(has_sym("randombytes_stir"));
    assert!(has_sym("randombytes_close"));
    unsafe {
        let (cs, rs_) = pair::<Stir>("randombytes_stir");
        let (cc, rc) = pair::<Close>("randombytes_close");
        set_impl_both(&IMPL_FULL as *const RandombytesImpl);
        STIR_CALLS.store(0, AtOrd::SeqCst);
        cs();
        rs_();
        assert_eq!(STIR_CALLS.load(AtOrd::SeqCst), 2, "stir was not delegated");
        CLOSE_CALLS.store(0, AtOrd::SeqCst);
        eq_i32("close", cc(), rc());
        assert_eq!(CLOSE_CALLS.load(AtOrd::SeqCst), 2);
        use_det();
    }
}

/// Bookkeeping for the ERRORS.md rows in 639-739 that cannot be reached through
/// the public API on this platform/build. Kept as a test so the list stays
/// visible in the run output.
///
/// * 642, 710, 714, 719 — "no length check": exercising them means an
///   out-of-bounds read (UB), not an observable error.
/// * 643 — `assert(buf_len <= SIZE_MAX)` in the NaCl wrapper: a tautology on
///   64-bit targets.
/// * 652, 653, 655 — `randombytes_sysrandom_close` failure branches: need
///   `getrandom()` to be unavailable (or a `_WIN32` build).
/// * 656-663, 664-672 — `safe_read` / `_randombytes_linux_getrandom` /
///   `randombytes_block_on_dev_random` / `randombytes_sysrandom_init` failure
///   branches: all `static`, only reachable if the kernel's entropy source
///   fails or `/dev/urandom` cannot be opened.
/// * 673 — `sodium_hrtime` when `gettimeofday` fails.
/// * 674-679 — the `getentropy`/CommonCrypto variants are not compiled in.
/// * 680-698, 704-706 — the `internal` backend's `static` failure branches and
///   `assert`s (short getrandom read, `/dev/random` fallback, fork-safety guard
///   which needs `HAVE_GETPID`, ChaCha20 returning non-zero).
/// * 699 — fork-safety `sodium_misuse()`: `HAVE_GETPID` is not defined in this
///   build, so `randombytes_internal_random_stir_if_needed` has no pid check.
/// * 703 — `_WIN32`-only.
/// * 721 — `crypto_kem_mlkem768_dec` never returns non-zero, so the X-Wing
///   error branch guarding it is dead code (`LCOV_EXCL`).
/// * 730, 731 — ipcrypt length mismatches are OOB reads/writes, not errors.
#[test]
fn g6e_unreachable_rows_documented() {
    // Positive evidence for the two claims that *can* be checked cheaply:
    // the entropy backends work (so their failure paths are unreachable here),
    // and `crypto_kem_mlkem768_dec` returns 0 even for garbage.
    unsafe {
        let l = libs();
        for lib in [l.c, l.rs] {
            let p = impl_addr(lib, "randombytes_sysrandom_implementation");
            eq_i32("set_implementation", 0, set_impl(lib, p));
            let buf: Symbol<Buf> = lib.get(b"randombytes_buf\0").unwrap();
            let mut b = [0u8; 64];
            buf(b.as_mut_ptr() as *mut c_void, 64);
            assert!(b.iter().any(|&x| x != 0), "sysrandom produced all zeros");
        }
        let (cdec, rdec) = pair::<Dec>("crypto_kem_mlkem768_dec");
        let sk = vec![0u8; MLK_SK];
        let ct = vec![0u8; MLK_CT];
        let mut a = vec![0u8; SS];
        let mut b = vec![0u8; SS];
        let x = cdec(a.as_mut_ptr(), ct.as_ptr(), sk.as_ptr());
        let y = rdec(b.as_mut_ptr(), ct.as_ptr(), sk.as_ptr());
        eq_i32("dec(all-zero sk/ct)", x, y);
        assert_eq!(x, 0, "mlkem768_dec must return 0 even for an all-zero sk/ct");
        eq_bytes("dec(all-zero sk/ct) ss", &a, &b);
        use_det();
    }
}

// ===========================================================================
// child-process driver
// ===========================================================================
#[test]
fn zz_abort_child() {
    let Some((case, is_c)) = child_case() else {
        return;
    };
    let l = libs();
    let h = if is_c { l.c } else { l.rs };
    unsafe {
        macro_rules! sym {
            ($t:ty, $n:literal) => {{
                let s: Symbol<$t> = h.get(concat!($n, "\0").as_bytes()).unwrap();
                s
            }};
        }
        match case.as_str() {
            // row 641: one step past randombytes_BYTES_MAX -> sodium_misuse()
            "bufdet_oversize" => {
                let f = sym!(BufDet, "randombytes_buf_deterministic");
                let seed = [0u8; 32];
                let mut small = [0u8; 64];
                f(
                    small.as_mut_ptr() as *mut c_void,
                    BUFDET_MAX + 1,
                    seed.as_ptr(),
                );
            }
            // row 641, upper extreme: SIZE_MAX is also > randombytes_BYTES_MAX
            "bufdet_sizemax" => {
                let f = sym!(BufDet, "randombytes_buf_deterministic");
                let seed = [0xffu8; 32];
                let mut small = [0u8; 64];
                f(small.as_mut_ptr() as *mut c_void, usize::MAX, seed.as_ptr());
            }
            // NULL buffer with size 0 must be harmless (the ChaCha20 stream
            // short-circuits before dereferencing `c`).
            "bufdet_null_zero" => {
                let f = sym!(BufDet, "randombytes_buf_deterministic");
                let seed = [0x5au8; 32];
                f(ptr::null_mut(), 0, seed.as_ptr());
                let g = sym!(Buf, "randombytes_buf");
                g(ptr::null_mut(), 0);
                let n = sym!(NaclRb, "randombytes");
                n(ptr::null_mut(), 0);
                std::process::exit(0);
            }
            // row 640: NULL "required" hooks
            "impl_null_name" => {
                let s = sym!(SetImpl, "randombytes_set_implementation");
                assert_eq!(s(&IMPL_NULL_NAME as *const RandombytesImpl), 0);
                let f = sym!(NameGet, "randombytes_implementation_name");
                let p = f();
                eprintln!("survived: {p:?}");
            }
            "impl_null_random" => {
                let s = sym!(SetImpl, "randombytes_set_implementation");
                assert_eq!(s(&IMPL_NULL_RANDOM as *const RandombytesImpl), 0);
                let f = sym!(Random, "randombytes_random");
                let v = f();
                eprintln!("survived: {v}");
            }
            "impl_null_buf" => {
                let s = sym!(SetImpl, "randombytes_set_implementation");
                assert_eq!(s(&IMPL_NULL_BUF as *const RandombytesImpl), 0);
                let f = sym!(Buf, "randombytes_buf");
                let mut b = [0u8; 32];
                f(b.as_mut_ptr() as *mut c_void, 32);
                eprintln!("survived: {b:?}");
            }
            // rows 700/701: in a pristine process the `internal` backend has
            // never been stirred, so `global.getrandom_available == 0` and
            // close() reports -1. Exit 0 iff that is what happened.
            "internal_close_fresh" => {
                let mut n = b"randombytes_internal_implementation".to_vec();
                n.push(0);
                let sy: Symbol<*mut c_void> = h.get(&n).unwrap();
                let p = sy.into_raw().into_raw() as *const RandombytesImpl;
                let s = sym!(SetImpl, "randombytes_set_implementation");
                assert_eq!(s(p), 0);
                let close = sym!(Close, "randombytes_close");
                let r1 = close();
                let r2 = close();
                if r1 == -1 && r2 == -1 {
                    std::process::exit(0);
                }
                eprintln!("unexpected fresh internal close: {r1}, {r2}");
                std::process::exit(3);
            }
            // rows 664/666 (sysrandom) and 675/678/680/682 (internal): the
            // `.buf` hook itself, called with size 0. `randombytes_buf()` can
            // never do this (it short-circuits), but the vtable is public, and
            // in a build with assertions enabled the 256-byte chunking wrapper
            // trips `assert(chunk_size > 0)`.
            "vtable_buf_zero_sysrandom" | "vtable_buf_zero_internal" => {
                let which = if case.ends_with("sysrandom") {
                    "randombytes_sysrandom_implementation"
                } else {
                    "randombytes_internal_implementation"
                };
                let mut n = which.as_bytes().to_vec();
                n.push(0);
                let sy: Symbol<*mut c_void> = h.get(&n).unwrap();
                let p = sy.into_raw().into_raw() as *const RandombytesImpl;
                let s = sym!(SetImpl, "randombytes_set_implementation");
                assert_eq!(s(p), 0);
                let stir = sym!(Stir, "randombytes_stir");
                stir(); // make sure the backend is initialised
                let hook = (*p).buf.expect("buf hook");
                let mut b = [0x5au8; 8];
                hook(b.as_mut_ptr() as *mut c_void, 0);
                assert_eq!(b, [0x5au8; 8], "buf(size=0) modified the buffer");
                std::process::exit(0);
            }
            other => panic!("unknown child case {other:?}"),
        }
        // reached only when the dangerous call did NOT kill the process
        std::process::exit(0);
    }
}
