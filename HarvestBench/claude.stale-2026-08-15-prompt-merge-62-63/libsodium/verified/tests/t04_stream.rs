//! t04_stream.rs — C-vs-Rust differential verification of the whole
//! `crypto_stream` surface.
//!
//! CONFIGS.md rows 83–97 and ERRORS.md rows 245–257 are the specification.
//! Every call goes through `dlsym` on BOTH shared objects; no Rust function is
//! ever called directly.
//!
//! Row → test mapping
//! ------------------
//! * 83  `crypto_stream_chacha20` keystream .............. `r83_chacha20_keystream`
//! * 84  `crypto_stream_chacha20_xor` ..................... `r84_chacha20_xor`
//! * 85  `crypto_stream_chacha20_xor_ic` (u64 ic) ........ `r85_chacha20_xor_ic`,
//!                                                         `r85_chacha20_j12_j13_carry`
//! * 86  `crypto_stream_chacha20_ietf` ................... `r86_chacha20_ietf_keystream`
//! * 87  `crypto_stream_chacha20_ietf_xor` .............. `r87_chacha20_ietf_xor`
//! * 88  `crypto_stream_chacha20_ietf_xor_ic` (u32 ic) .. `r88_chacha20_ietf_xor_ic`,
//!                                                         `r88_ietf_xor_ic_last_legal_ic`
//! * 89  `_ietf_ext` / `_ietf_ext_xor_ic` ............... `r89_chacha20_ietf_ext`,
//!                                                         `r89_ietf_ext_xor_ic`,
//!                                                         `r89_ietf_ext_counter_overflows_into_iv`
//! * 90  `crypto_stream_salsa20` / `_xor` ............... `r90_salsa20`
//! * 91  `crypto_stream_salsa20_xor_ic` ................ `r91_salsa20_xor_ic`,
//!                                                         `r91_salsa20_carry_chain`
//! * 92  `crypto_stream_salsa2012` / `_xor` ............ `r92_salsa2012`
//! * 93  `crypto_stream_salsa208` / `_xor` ............. `r93_salsa208`
//! * 94  `crypto_stream_xsalsa20*` ..................... `r94_xsalsa20`
//! * 95  `crypto_stream_xchacha20*` ................... `r95_xchacha20`
//! * 96  `crypto_stream` / `_xor` / `_primitive` ....... `r96_crypto_stream_dispatch`,
//!                                                         `r83_97_constants`, `r83_97_keygen`
//! * 97  key × nonce pattern matrix ................... `r97_key_nonce_pattern_matrix`
//! * 245/246/247/252 `> SODIUM_SIZE_MAX` guards ....... `e245_247_252_size_max_guards_unreachable`
//! * 248 `ietf` clen bound ............................ `e248_ietf_clen_bound`
//! * 249 `ietf_xor_ic` ic guard ....................... `e249_ietf_xor_ic_guard`
//! * 250 ietf ic-guard underflow QUIRK ................ `e250_ietf_xor_ic_guard_underflow_quirk`
//! * 251 `ietf_xor` mlen bound ........................ `e251_ietf_xor_mlen_bound`
//! * 253 salsa20 has no bounds check .................. `e253_salsa20_no_bounds_check`
//! * 254 salsa2012/208 have no bounds check ........... `e254_salsa2012_salsa208_no_bounds_check`
//! * 255 xsalsa20 never fails ......................... `e255_xsalsa20_never_fails`
//! * 256 xchacha20 delegates to chacha20 .............. `e256_xchacha20_delegates`
//! * 257 `len == 0` ⇒ early return, nothing written ... `e257_zero_length_writes_nothing`
//! * internal `_pick_best_implementation` ............. `internal_pick_best_implementation`

mod common;
use common::*;
use libc::{c_char, c_int};
use std::ffi::CStr;

// ------------------------------------------------------------------ fn types

type StreamFn = unsafe extern "C" fn(*mut u8, u64, *const u8, *const u8) -> c_int;
type XorFn = unsafe extern "C" fn(*mut u8, *const u8, u64, *const u8, *const u8) -> c_int;
type XorIc64Fn = unsafe extern "C" fn(*mut u8, *const u8, u64, *const u8, u64, *const u8) -> c_int;
type XorIc32Fn = unsafe extern "C" fn(*mut u8, *const u8, u64, *const u8, u32, *const u8) -> c_int;
type SizeFn = unsafe extern "C" fn() -> usize;
type KeygenFn = unsafe extern "C" fn(*mut u8);
type CoreFn = unsafe extern "C" fn(*mut u8, *const u8, *const u8, *const u8) -> c_int;

/// Prefill byte for every output buffer.
const FILL: u8 = 0xAA;
/// Trailing guard region: must never be touched by the library.
const PAD: usize = 32;

/// CONFIGS 83–97 length sweep: the harness sweep (which already contains every
/// block boundary in the spec list) plus the explicit 4096 from the row text.
fn lens() -> Vec<usize> {
    let mut v: Vec<usize> = LENS.to_vec();
    v.push(4096);
    v
}

/// Lengths used for the (much wider) pattern cross-product rows.
const PLENS: &[usize] = &[0, 1, 31, 32, 63, 64, 65, 127, 128, 129, 191, 192, 255, 256, 512, 513];

fn guard_intact(what: &str, who: &str, b: &[u8], len: usize) {
    assert!(
        b[len..].iter().all(|&x| x == FILL),
        "{what}: {who} wrote OUTSIDE the requested {len} bytes \
         (0xAA trailing guard clobbered: {})",
        hexs(&b[len..])
    );
}

// ---------------------------------------------------------------- primitives
//
// Each helper drives ONE entry point through both `.so` files with a
// 0xAA-prefilled output buffer, asserts the return values and the FULL buffers
// agree, checks nothing was written past `len`, and returns the (now proven
// identical) output so callers can assert algebraic properties on it. Any
// property asserted on a returned value therefore holds for BOTH libraries.

fn ks(name: &str, len: usize, n: &[u8], k: &[u8], tag: &str) -> Vec<u8> {
    let (fc, fr) = unsafe { pair::<StreamFn>(name) };
    let mut bc = vec![FILL; len + PAD];
    let mut br = vec![FILL; len + PAD];
    let rc = unsafe { fc(bc.as_mut_ptr(), len as u64, n.as_ptr(), k.as_ptr()) };
    let rr = unsafe { fr(br.as_mut_ptr(), len as u64, n.as_ptr(), k.as_ptr()) };
    let what = format!("{name} [{tag}] len={len} k={} n={}", hexs(k), hexs(n));
    assert_eq!(rc, rr, "{what}: return value differs (C={rc} rust={rr})");
    assert_eq!(rc, 0, "{what}: C returned {rc}, expected 0");
    assert_eq_bytes(&what, &bc, &br);
    guard_intact(&what, "C", &bc, len);
    guard_intact(&what, "rust", &br, len);
    bc.truncate(len);
    bc
}

fn xor(name: &str, m: &[u8], n: &[u8], k: &[u8], tag: &str) -> Vec<u8> {
    let (fc, fr) = unsafe { pair::<XorFn>(name) };
    let len = m.len();
    let mut bc = vec![FILL; len + PAD];
    let mut br = vec![FILL; len + PAD];
    let rc = unsafe { fc(bc.as_mut_ptr(), m.as_ptr(), len as u64, n.as_ptr(), k.as_ptr()) };
    let rr = unsafe { fr(br.as_mut_ptr(), m.as_ptr(), len as u64, n.as_ptr(), k.as_ptr()) };
    let what = format!("{name} [{tag}] len={len} k={} n={}", hexs(k), hexs(n));
    assert_eq!(rc, rr, "{what}: return value differs (C={rc} rust={rr})");
    assert_eq!(rc, 0, "{what}: C returned {rc}, expected 0");
    assert_eq_bytes(&what, &bc, &br);
    guard_intact(&what, "C", &bc, len);
    guard_intact(&what, "rust", &br, len);
    bc.truncate(len);
    bc
}

/// `_xor` with `c == m` (in place).
fn xor_ip(name: &str, m: &[u8], n: &[u8], k: &[u8], tag: &str) -> Vec<u8> {
    let (fc, fr) = unsafe { pair::<XorFn>(name) };
    let len = m.len();
    let mut bc = vec![FILL; len + PAD];
    bc[..len].copy_from_slice(m);
    let mut br = bc.clone();
    let pc = bc.as_mut_ptr();
    let pr = br.as_mut_ptr();
    let rc = unsafe { fc(pc, pc, len as u64, n.as_ptr(), k.as_ptr()) };
    let rr = unsafe { fr(pr, pr, len as u64, n.as_ptr(), k.as_ptr()) };
    let what = format!("{name} [{tag}/in-place] len={len} k={} n={}", hexs(k), hexs(n));
    assert_eq!(rc, rr, "{what}: return value differs (C={rc} rust={rr})");
    assert_eq!(rc, 0, "{what}: C returned {rc}, expected 0");
    assert_eq_bytes(&what, &bc, &br);
    guard_intact(&what, "C", &bc, len);
    guard_intact(&what, "rust", &br, len);
    bc.truncate(len);
    bc
}

fn xor_ic64(name: &str, m: &[u8], n: &[u8], ic: u64, k: &[u8], tag: &str) -> Vec<u8> {
    let (fc, fr) = unsafe { pair::<XorIc64Fn>(name) };
    let len = m.len();
    let mut bc = vec![FILL; len + PAD];
    let mut br = vec![FILL; len + PAD];
    let rc = unsafe { fc(bc.as_mut_ptr(), m.as_ptr(), len as u64, n.as_ptr(), ic, k.as_ptr()) };
    let rr = unsafe { fr(br.as_mut_ptr(), m.as_ptr(), len as u64, n.as_ptr(), ic, k.as_ptr()) };
    let what = format!("{name} [{tag}] len={len} ic={ic:#x} k={} n={}", hexs(k), hexs(n));
    assert_eq!(rc, rr, "{what}: return value differs (C={rc} rust={rr})");
    assert_eq!(rc, 0, "{what}: C returned {rc}, expected 0");
    assert_eq_bytes(&what, &bc, &br);
    guard_intact(&what, "C", &bc, len);
    guard_intact(&what, "rust", &br, len);
    bc.truncate(len);
    bc
}

fn xor_ic64_ip(name: &str, m: &[u8], n: &[u8], ic: u64, k: &[u8], tag: &str) -> Vec<u8> {
    let (fc, fr) = unsafe { pair::<XorIc64Fn>(name) };
    let len = m.len();
    let mut bc = vec![FILL; len + PAD];
    bc[..len].copy_from_slice(m);
    let mut br = bc.clone();
    let pc = bc.as_mut_ptr();
    let pr = br.as_mut_ptr();
    let rc = unsafe { fc(pc, pc, len as u64, n.as_ptr(), ic, k.as_ptr()) };
    let rr = unsafe { fr(pr, pr, len as u64, n.as_ptr(), ic, k.as_ptr()) };
    let what = format!("{name} [{tag}/in-place] len={len} ic={ic:#x} k={}", hexs(k));
    assert_eq!(rc, rr, "{what}: return value differs (C={rc} rust={rr})");
    assert_eq!(rc, 0, "{what}: C returned {rc}, expected 0");
    assert_eq_bytes(&what, &bc, &br);
    guard_intact(&what, "C", &bc, len);
    guard_intact(&what, "rust", &br, len);
    bc.truncate(len);
    bc
}

fn xor_ic32(name: &str, m: &[u8], n: &[u8], ic: u32, k: &[u8], tag: &str) -> Vec<u8> {
    let (fc, fr) = unsafe { pair::<XorIc32Fn>(name) };
    let len = m.len();
    let mut bc = vec![FILL; len + PAD];
    let mut br = vec![FILL; len + PAD];
    let rc = unsafe { fc(bc.as_mut_ptr(), m.as_ptr(), len as u64, n.as_ptr(), ic, k.as_ptr()) };
    let rr = unsafe { fr(br.as_mut_ptr(), m.as_ptr(), len as u64, n.as_ptr(), ic, k.as_ptr()) };
    let what = format!("{name} [{tag}] len={len} ic={ic:#x} k={} n={}", hexs(k), hexs(n));
    assert_eq!(rc, rr, "{what}: return value differs (C={rc} rust={rr})");
    assert_eq!(rc, 0, "{what}: C returned {rc}, expected 0");
    assert_eq_bytes(&what, &bc, &br);
    guard_intact(&what, "C", &bc, len);
    guard_intact(&what, "rust", &br, len);
    bc.truncate(len);
    bc
}

fn xor_ic32_ip(name: &str, m: &[u8], n: &[u8], ic: u32, k: &[u8], tag: &str) -> Vec<u8> {
    let (fc, fr) = unsafe { pair::<XorIc32Fn>(name) };
    let len = m.len();
    let mut bc = vec![FILL; len + PAD];
    bc[..len].copy_from_slice(m);
    let mut br = bc.clone();
    let pc = bc.as_mut_ptr();
    let pr = br.as_mut_ptr();
    let rc = unsafe { fc(pc, pc, len as u64, n.as_ptr(), ic, k.as_ptr()) };
    let rr = unsafe { fr(pr, pr, len as u64, n.as_ptr(), ic, k.as_ptr()) };
    let what = format!("{name} [{tag}/in-place] len={len} ic={ic:#x} k={}", hexs(k));
    assert_eq!(rc, rr, "{what}: return value differs (C={rc} rust={rr})");
    assert_eq!(rc, 0, "{what}: C returned {rc}, expected 0");
    assert_eq_bytes(&what, &bc, &br);
    guard_intact(&what, "C", &bc, len);
    guard_intact(&what, "rust", &br, len);
    bc.truncate(len);
    bc
}

fn x(a: &[u8], b: &[u8]) -> Vec<u8> {
    a.iter().zip(b.iter()).map(|(p, q)| p ^ q).collect()
}

// ------------------------------------------------- forked fatal-path plumbing
//
// `SODIUM_SIZE_MAX == UINT64_MAX` on this target, so the `len > MESSAGEBYTES_MAX`
// guards of rows 245/246/247/252 can never fire: the call runs on to touch
// `len` bytes of memory and dies on a page fault instead of `abort()`ing.
// To tell the two apart deterministically we install a SIGSEGV/SIGBUS handler in
// the forked child that exits with a marker: `Returned(FAULT)` means "the guard
// did NOT fire, memory access happened", `Signaled(SIGABRT)` means
// "sodium_misuse()".

const FAULT: i64 = 42;

extern "C" fn fault_handler(_sig: c_int) {
    unsafe { libc::_exit(FAULT as c_int) }
}

/// Async-signal-safe, allocation-free. Also disables core dumps: without this
/// every `abort()`/fault child would dump its whole address space.
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

/// Resolve `name` in both libraries in the PARENT (so the child needs neither
/// dlsym nor malloc), then run `body` once per library in a forked child.
fn both_forked<T: Copy + 'static, B: Fn(T) -> i64 + Copy>(name: &str, body: B) -> (Outcome, Outcome) {
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
    (oc, or)
}

fn expect_outcome<T: Copy + 'static, B: Fn(T) -> i64 + Copy>(what: &str, name: &str, body: B, want: Outcome) {
    let (oc, or) = both_forked::<T, B>(name, body);
    assert_same_fatal(what, oc, or);
    assert_eq!(oc, want, "{what}: C outcome was {oc:?}, expected {want:?}");
}

const MISUSE: Outcome = Outcome::Signaled(SIGABRT);
const NO_MISUSE: Outcome = Outcome::Returned(FAULT);

/// A scratch buffer allocated in the PARENT; children only read/write it.
struct Scratch {
    _v: Vec<u8>,
    p: *mut u8,
}
fn scratch(n: usize) -> Scratch {
    let mut v = vec![0u8; n];
    let p = v.as_mut_ptr();
    Scratch { _v: v, p }
}

// ------------------------------------------------------------- cipher families

struct Fam {
    ks: &'static str,
    xor: &'static str,
    ic64: Option<&'static str>,
    nb: usize,
}

const FAMS: &[Fam] = &[
    Fam {
        ks: "crypto_stream_chacha20",
        xor: "crypto_stream_chacha20_xor",
        ic64: Some("crypto_stream_chacha20_xor_ic"),
        nb: 8,
    },
    Fam {
        ks: "crypto_stream_chacha20_ietf",
        xor: "crypto_stream_chacha20_ietf_xor",
        ic64: None,
        nb: 12,
    },
    Fam {
        ks: "crypto_stream_salsa20",
        xor: "crypto_stream_salsa20_xor",
        ic64: Some("crypto_stream_salsa20_xor_ic"),
        nb: 8,
    },
    Fam {
        ks: "crypto_stream_salsa2012",
        xor: "crypto_stream_salsa2012_xor",
        ic64: None,
        nb: 8,
    },
    Fam {
        ks: "crypto_stream_salsa208",
        xor: "crypto_stream_salsa208_xor",
        ic64: None,
        nb: 8,
    },
    Fam {
        ks: "crypto_stream_xsalsa20",
        xor: "crypto_stream_xsalsa20_xor",
        ic64: Some("crypto_stream_xsalsa20_xor_ic"),
        nb: 24,
    },
    Fam {
        ks: "crypto_stream_xchacha20",
        xor: "crypto_stream_xchacha20_xor",
        ic64: Some("crypto_stream_xchacha20_xor_ic"),
        nb: 24,
    },
    Fam {
        ks: "crypto_stream",
        xor: "crypto_stream_xor",
        ic64: None,
        nb: 24,
    },
];

/// Requirement 3 for one family / one input: `xor(m) == m ^ keystream()`,
/// in-place == disjoint, and `xor_ic(ic=0) == xor()`.
fn family_consistency(fam: &Fam, m: &[u8], n: &[u8], k: &[u8], tag: &str) {
    let len = m.len();
    let stream = ks(fam.ks, len, n, k, tag);
    let d = xor(fam.xor, m, n, k, tag);
    assert_eq_bytes(
        &format!("{} [{tag}] len={len}: xor(m) != m ^ {}()", fam.xor, fam.ks),
        &x(m, &stream),
        &d,
    );
    let ip = xor_ip(fam.xor, m, n, k, tag);
    assert_eq_bytes(
        &format!("{} [{tag}] len={len}: in-place != disjoint", fam.xor),
        &d,
        &ip,
    );
    if let Some(icn) = fam.ic64 {
        let z = xor_ic64(icn, m, n, 0, k, tag);
        assert_eq_bytes(
            &format!("{icn} [{tag}] len={len}: ic=0 != {}", fam.xor),
            &d,
            &z,
        );
        let zip = xor_ic64_ip(icn, m, n, 0, k, tag);
        assert_eq_bytes(
            &format!("{icn} [{tag}] len={len}: ic=0 in-place != disjoint"),
            &d,
            &zip,
        );
    }
}

/// Requirement 4: splitting a buffer at a 64-byte boundary and bumping `ic` by
/// the number of consumed blocks must reproduce the single-shot output.
fn chunked_ic64(name: &str, m: &[u8], n: &[u8], k: &[u8], ic: u64, split: usize) {
    assert_eq!(split % 64, 0, "split must be a block multiple");
    let tag = "chunked";
    let whole = xor_ic64(name, m, n, ic, k, tag);
    let a = xor_ic64(name, &m[..split], n, ic, k, tag);
    let b = xor_ic64(name, &m[split..], n, ic.wrapping_add((split / 64) as u64), k, tag);
    let mut cat = a;
    cat.extend_from_slice(&b);
    assert_eq_bytes(
        &format!("{name}: chunked(split={split}, ic={ic:#x}) != single-shot"),
        &whole,
        &cat,
    );
}

/// 32-bit counter version. Only valid below the 32-bit wrap: past it the `_ext`
/// entry point carries into the IV, which a fresh call cannot reproduce (that
/// behaviour is pinned by `r89_ietf_ext_counter_overflows_into_iv` instead).
fn chunked_ic32(name: &str, m: &[u8], n: &[u8], k: &[u8], ic: u32, split: usize) {
    assert_eq!(split % 64, 0, "split must be a block multiple");
    assert!(
        (ic as u64) + (m.len() as u64 + 63) / 64 <= 1u64 << 32,
        "chunked_ic32 called across the 32-bit counter wrap"
    );
    let tag = "chunked";
    let whole = xor_ic32(name, m, n, ic, k, tag);
    let a = xor_ic32(name, &m[..split], n, ic, k, tag);
    let b = xor_ic32(name, &m[split..], n, ic + (split / 64) as u32, k, tag);
    let mut cat = a;
    cat.extend_from_slice(&b);
    assert_eq_bytes(
        &format!("{name}: chunked(split={split}, ic={ic:#x}) != single-shot"),
        &whole,
        &cat,
    );
}

/// The exact C expression from `crypto_stream_chacha20_ietf_xor_ic`:
/// `(64ULL * (1ULL << 32)) / 64ULL - (mlen + 63ULL) / 64ULL`.
fn ietf_ic_limit(mlen: u64) -> u64 {
    (1u64 << 32) - (mlen + 63) / 64
}

// =========================================================== rows 83–97

/// Rows 83–96: every `*_keybytes` / `*_noncebytes` / `*_messagebytes_max`
/// accessor plus `crypto_stream_primitive`.
#[test]
fn r83_97_constants() {
    init_both();
    let sizes = [
        ("crypto_stream_keybytes", 32usize),
        ("crypto_stream_noncebytes", 24),
        ("crypto_stream_chacha20_keybytes", 32),
        ("crypto_stream_chacha20_noncebytes", 8),
        ("crypto_stream_chacha20_ietf_keybytes", 32),
        ("crypto_stream_chacha20_ietf_noncebytes", 12),
        ("crypto_stream_salsa20_keybytes", 32),
        ("crypto_stream_salsa20_noncebytes", 8),
        ("crypto_stream_salsa2012_keybytes", 32),
        ("crypto_stream_salsa2012_noncebytes", 8),
        ("crypto_stream_salsa208_keybytes", 32),
        ("crypto_stream_salsa208_noncebytes", 8),
        ("crypto_stream_xsalsa20_keybytes", 32),
        ("crypto_stream_xsalsa20_noncebytes", 24),
        ("crypto_stream_xchacha20_keybytes", 32),
        ("crypto_stream_xchacha20_noncebytes", 24),
    ];
    for (name, want) in sizes {
        let (c, r) = unsafe { pair::<SizeFn>(name) };
        let (vc, vr) = unsafe { (c(), r()) };
        assert_eq!(vc, vr, "{name}: C={vc} rust={vr}");
        assert_eq!(vc, want, "{name}: C returned {vc}, header says {want}");
    }
    // MESSAGEBYTES_MAX: SODIUM_SIZE_MAX everywhere except the ietf variant,
    // which is min(SODIUM_SIZE_MAX, 64 * 2^32).
    let smax = usize::MAX.min(u64::MAX as usize);
    for name in [
        "crypto_stream_messagebytes_max",
        "crypto_stream_chacha20_messagebytes_max",
        "crypto_stream_salsa20_messagebytes_max",
        "crypto_stream_salsa2012_messagebytes_max",
        "crypto_stream_salsa208_messagebytes_max",
        "crypto_stream_xsalsa20_messagebytes_max",
        "crypto_stream_xchacha20_messagebytes_max",
    ] {
        let (c, r) = unsafe { pair::<SizeFn>(name) };
        let (vc, vr) = unsafe { (c(), r()) };
        assert_eq!(vc, vr, "{name}: C={vc} rust={vr}");
        assert_eq!(vc, smax, "{name}: C returned {vc}, expected SODIUM_SIZE_MAX");
    }
    let (c, r) = unsafe { pair::<SizeFn>("crypto_stream_chacha20_ietf_messagebytes_max") };
    let (vc, vr) = unsafe { (c(), r()) };
    assert_eq!(vc, vr, "ietf messagebytes_max: C={vc} rust={vr}");
    assert_eq!(vc, 64usize * (1usize << 32), "ietf messagebytes_max = 64*2^32");

    // row 96: the dispatch primitive name
    let (c, r) = unsafe { pair::<unsafe extern "C" fn() -> *const c_char>("crypto_stream_primitive") };
    let (sc, sr) = unsafe { (CStr::from_ptr(c()), CStr::from_ptr(r())) };
    assert_eq!(sc, sr, "crypto_stream_primitive differs");
    assert_eq!(sc.to_str().unwrap(), "xsalsa20");
}

/// Rows 83–96: every `*_keygen` entry point, driven by the injected
/// deterministic RNG so the produced keys are byte-comparable.
#[test]
fn r83_97_keygen() {
    init_both();
    install_det_rng(false);
    for name in [
        "crypto_stream_keygen",
        "crypto_stream_chacha20_keygen",
        "crypto_stream_chacha20_ietf_keygen",
        "crypto_stream_salsa20_keygen",
        "crypto_stream_salsa2012_keygen",
        "crypto_stream_salsa208_keygen",
        "crypto_stream_xsalsa20_keygen",
        "crypto_stream_xchacha20_keygen",
    ] {
        let (c, r) = unsafe { pair::<KeygenFn>(name) };
        for _ in 0..8 {
            reset_det_rng();
            let mut kc = [FILL; 32 + PAD];
            let mut kr = [FILL; 32 + PAD];
            unsafe {
                c(kc.as_mut_ptr());
                r(kr.as_mut_ptr());
            }
            assert_eq_bytes(&format!("{name} output"), &kc, &kr);
            guard_intact(name, "C", &kc, 32);
            assert!(kc[..32] != [FILL; 32], "{name} wrote nothing");
        }
    }
}

/// Row 83: `crypto_stream_chacha20` keystream over the full length sweep,
/// 8-byte nonce. `len == 0` must write nothing at all.
#[test]
fn r83_chacha20_keystream() {
    init_both();
    let mut rng = Rng::new(SEED ^ 83);
    let keys = patterns(32, &mut rng);
    let nonces = patterns(8, &mut rng);
    let mut iters = 0usize;
    for (li, &len) in lens().iter().enumerate() {
        for (ki, k) in keys.iter().enumerate() {
            let n = &nonces[(li + ki) % nonces.len()];
            let stream = ks("crypto_stream_chacha20", len, n, k, "row83");
            // the keystream is memset(0)+encrypt, i.e. XOR against zeros
            let z = vec![0u8; len];
            let xz = xor("crypto_stream_chacha20_xor", &z, n, k, "row83");
            assert_eq_bytes("row83: chacha20() != chacha20_xor(zeros)", &stream, &xz);
            iters += 1;
        }
    }
    assert!(iters >= 64, "row83 only ran {iters} inputs");
}

/// Row 84: `crypto_stream_chacha20_xor`, in-place and disjoint.
#[test]
fn r84_chacha20_xor() {
    init_both();
    let mut rng = Rng::new(SEED ^ 84);
    let keys = patterns(32, &mut rng);
    let nonces = patterns(8, &mut rng);
    let fam = &FAMS[0];
    let mut iters = 0usize;
    for (li, &len) in lens().iter().enumerate() {
        let msgs = patterns(len, &mut rng);
        for (mi, m) in msgs.iter().enumerate() {
            let k = &keys[(li + mi) % keys.len()];
            let n = &nonces[(li + 2 * mi) % nonces.len()];
            family_consistency(fam, m, n, k, "row84");
            iters += 1;
        }
    }
    assert!(iters >= 64, "row84 only ran {iters} inputs");
}

/// Row 85: `crypto_stream_chacha20_xor_ic`, `ic` (u64) across the wrap points.
#[test]
fn r85_chacha20_xor_ic() {
    init_both();
    let mut rng = Rng::new(SEED ^ 85);
    let keys = patterns(32, &mut rng);
    let nonces = patterns(8, &mut rng);
    const ICS: &[u64] = &[
        0,
        1,
        0xFFFF_FFFF,          // 2^32-1  : next increment carries j12 -> j13
        0x1_0000_0000,        // 2^32
        0x1_0000_0001,        // 2^32+1
        u64::MAX,             // wraps the whole 64-bit counter
    ];
    let mut iters = 0usize;
    for (li, &len) in lens().iter().enumerate() {
        let m = rng.bytes(len);
        for (ii, &ic) in ICS.iter().enumerate() {
            let k = &keys[(li + ii) % keys.len()];
            let n = &nonces[(li + 2 * ii) % nonces.len()];
            let d = xor_ic64("crypto_stream_chacha20_xor_ic", &m, n, ic, k, "row85");
            let ip = xor_ic64_ip("crypto_stream_chacha20_xor_ic", &m, n, ic, k, "row85");
            assert_eq_bytes("row85: in-place != disjoint", &d, &ip);
            // xor_ic is a pure XOR: applying it twice restores the plaintext
            let back = xor_ic64("crypto_stream_chacha20_xor_ic", &d, n, ic, k, "row85/inv");
            assert_eq_bytes("row85: xor_ic is not an involution", &m, &back);
            iters += 1;
        }
        // requirement 4: chunked == single-shot for every ic, at every block split
        if len >= 128 {
            for &ic in ICS {
                chunked_ic64("crypto_stream_chacha20_xor_ic", &m, &nonces[3], &keys[3], ic, 64);
                chunked_ic64(
                    "crypto_stream_chacha20_xor_ic",
                    &m,
                    &nonces[3],
                    &keys[3],
                    ic,
                    (len / 64 / 2) * 64,
                );
            }
        }
    }
    assert!(iters >= 64, "row85 only ran {iters} inputs");
}

/// Row 85 (the classic bug): incrementing the 32-bit `j12` must carry into
/// `j13`, i.e. the block after `ic = 2^32-1` is the block at `ic = 2^32`,
/// and the block after `ic = 2^64-1` is the block at `ic = 0`.
#[test]
fn r85_chacha20_j12_j13_carry() {
    init_both();
    let mut rng = Rng::new(SEED ^ 0x85_CA);
    for _ in 0..64 {
        let k = rng.bytes(32);
        let n = rng.bytes(8);
        let m = vec![0u8; 192];
        let at_max = xor_ic64("crypto_stream_chacha20_xor_ic", &m, &n, 0xFFFF_FFFF, &k, "carry");
        let at_2_32 = xor_ic64(
            "crypto_stream_chacha20_xor_ic",
            &m[..128],
            &n,
            0x1_0000_0000,
            &k,
            "carry",
        );
        assert_eq_bytes(
            "row85 j12->j13 carry: blocks after ic=2^32-1 must equal blocks at ic=2^32",
            &at_max[64..],
            &at_2_32,
        );
        let at_u64max = xor_ic64("crypto_stream_chacha20_xor_ic", &m, &n, u64::MAX, &k, "carry");
        let at_zero = xor_ic64("crypto_stream_chacha20_xor_ic", &m[..128], &n, 0, &k, "carry");
        assert_eq_bytes(
            "row85 counter must wrap 2^64 -> 0",
            &at_u64max[64..],
            &at_zero,
        );
    }
}

/// Row 86: `crypto_stream_chacha20_ietf` keystream, 12-byte nonce.
#[test]
fn r86_chacha20_ietf_keystream() {
    init_both();
    let mut rng = Rng::new(SEED ^ 86);
    let keys = patterns(32, &mut rng);
    let nonces = patterns(12, &mut rng);
    let mut iters = 0usize;
    for (li, &len) in lens().iter().enumerate() {
        for (ki, k) in keys.iter().enumerate() {
            let n = &nonces[(li + ki) % nonces.len()];
            let stream = ks("crypto_stream_chacha20_ietf", len, n, k, "row86");
            let z = vec![0u8; len];
            let xz = xor("crypto_stream_chacha20_ietf_xor", &z, n, k, "row86");
            assert_eq_bytes("row86: ietf() != ietf_xor(zeros)", &stream, &xz);
            // The two nonce layouts are (j13,j14,j15) = (n0,n1,n2) for ietf but
            // (0,n0,n1) for the 64-bit-nonce variant, so the keystreams coincide
            // exactly when all three nonce words are zero.
            if len >= 16 {
                let s8 = ks("crypto_stream_chacha20", len, &n[..8], k, "row86");
                if n[..12].iter().any(|&b| b != 0) {
                    assert!(
                        s8 != stream,
                        "row86: ietf and non-ietf keystreams must differ for a non-zero nonce"
                    );
                } else {
                    assert_eq_bytes(
                        "row86: with an all-zero nonce both layouts must coincide",
                        &s8,
                        &stream,
                    );
                }
            }
            iters += 1;
        }
    }
    assert!(iters >= 64, "row86 only ran {iters} inputs");
}

/// Row 87: `crypto_stream_chacha20_ietf_xor`, in-place and disjoint.
#[test]
fn r87_chacha20_ietf_xor() {
    init_both();
    let mut rng = Rng::new(SEED ^ 87);
    let keys = patterns(32, &mut rng);
    let nonces = patterns(12, &mut rng);
    let fam = &FAMS[1];
    let mut iters = 0usize;
    for (li, &len) in lens().iter().enumerate() {
        let msgs = patterns(len, &mut rng);
        for (mi, m) in msgs.iter().enumerate() {
            let k = &keys[(li + mi) % keys.len()];
            let n = &nonces[(li + 2 * mi) % nonces.len()];
            family_consistency(fam, m, n, k, "row87");
            // ic == 0 must equal the plain _xor entry point
            let d = xor("crypto_stream_chacha20_ietf_xor", m, n, k, "row87");
            let z = xor_ic32("crypto_stream_chacha20_ietf_xor_ic", m, n, 0, k, "row87");
            assert_eq_bytes("row87: ietf_xor_ic(ic=0) != ietf_xor", &d, &z);
            iters += 1;
        }
    }
    assert!(iters >= 64, "row87 only ran {iters} inputs");
}

/// Row 88: `crypto_stream_chacha20_ietf_xor_ic` with a u32 `ic`. Only values
/// satisfying the guard `ic <= 2^32 - ceil(mlen/64)` are legal here; the
/// illegal side is row 249.
#[test]
fn r88_chacha20_ietf_xor_ic() {
    init_both();
    let mut rng = Rng::new(SEED ^ 88);
    let keys = patterns(32, &mut rng);
    let nonces = patterns(12, &mut rng);
    const ICS: &[u64] = &[0, 1, 0xFFFF_FFFE, 0xFFFF_FFFF];
    let mut iters = 0usize;
    for (li, &len) in lens().iter().enumerate() {
        let m = rng.bytes(len);
        let limit = ietf_ic_limit(len as u64);
        for (ii, &ic) in ICS.iter().enumerate() {
            if ic > limit {
                continue; // row 249 territory
            }
            let k = &keys[(li + ii) % keys.len()];
            let n = &nonces[(li + 2 * ii) % nonces.len()];
            let d = xor_ic32("crypto_stream_chacha20_ietf_xor_ic", &m, n, ic as u32, k, "row88");
            let ip = xor_ic32_ip("crypto_stream_chacha20_ietf_xor_ic", &m, n, ic as u32, k, "row88");
            assert_eq_bytes("row88: in-place != disjoint", &d, &ip);
            let back = xor_ic32(
                "crypto_stream_chacha20_ietf_xor_ic",
                &d,
                n,
                ic as u32,
                k,
                "row88/inv",
            );
            assert_eq_bytes("row88: ietf_xor_ic is not an involution", &m, &back);
            iters += 1;
        }
        // random legal ic values, plus the chunked property
        for _ in 0..2 {
            let ic = if limit == 0 { 0 } else { (rng.next_u32() as u64) % limit };
            let k = &keys[3];
            let n = &nonces[4];
            xor_ic32("crypto_stream_chacha20_ietf_xor_ic", &m, n, ic as u32, k, "row88/rand");
            if len >= 128 {
                chunked_ic32("crypto_stream_chacha20_ietf_xor_ic", &m, n, k, ic as u32, 64);
            }
            iters += 1;
        }
    }
    assert!(iters >= 64, "row88 only ran {iters} inputs");
}

/// Row 88 boundary: `ic == 2^32 - ceil(mlen/64)` is the LAST legal value and
/// must succeed identically in both libraries.
#[test]
fn r88_ietf_xor_ic_last_legal_ic() {
    init_both();
    let mut rng = Rng::new(SEED ^ 0x88_1C);
    let mut iters = 0usize;
    for &len in lens().iter() {
        let limit = ietf_ic_limit(len as u64);
        if limit > u32::MAX as u64 {
            // only mlen == 0 (limit == 2^32): every u32 ic is legal
            assert_eq!(len, 0);
            let m: Vec<u8> = vec![];
            for ic in [0u32, 1, u32::MAX] {
                let out = xor_ic32(
                    "crypto_stream_chacha20_ietf_xor_ic",
                    &m,
                    &rng.bytes(12),
                    ic,
                    &rng.bytes(32),
                    "row88/limit",
                );
                assert!(out.is_empty());
                iters += 1;
            }
            continue;
        }
        for _ in 0..2 {
            let k = rng.bytes(32);
            let n = rng.bytes(12);
            let m = rng.bytes(len);
            // last legal ic
            let a = xor_ic32(
                "crypto_stream_chacha20_ietf_xor_ic",
                &m,
                &n,
                limit as u32,
                &k,
                "row88/last-legal",
            );
            // the low-level _ext entry point has no such guard and must agree
            let b = xor_ic32(
                "crypto_stream_chacha20_ietf_ext_xor_ic",
                &m,
                &n,
                limit as u32,
                &k,
                "row88/last-legal",
            );
            assert_eq_bytes(
                "row88: ietf_xor_ic(last legal ic) != ietf_ext_xor_ic(same ic)",
                &a,
                &b,
            );
            iters += 1;
        }
    }
    assert!(iters >= 64, "row88 boundary only ran {iters} inputs");
}

/// Row 89: the low-level extended-counter entry points.
#[test]
fn r89_chacha20_ietf_ext() {
    init_both();
    let mut rng = Rng::new(SEED ^ 89);
    let keys = patterns(32, &mut rng);
    let nonces = patterns(12, &mut rng);
    let mut iters = 0usize;
    for (li, &len) in lens().iter().enumerate() {
        for (ki, k) in keys.iter().enumerate() {
            let n = &nonces[(li + ki) % nonces.len()];
            let ext = ks("crypto_stream_chacha20_ietf_ext", len, n, k, "row89");
            // crypto_stream_chacha20_ietf is exactly _ietf_ext plus a bound check
            let ietf = ks("crypto_stream_chacha20_ietf", len, n, k, "row89");
            assert_eq_bytes("row89: ietf_ext() != ietf()", &ext, &ietf);
            let z = vec![0u8; len];
            let xz = xor_ic32("crypto_stream_chacha20_ietf_ext_xor_ic", &z, n, 0, k, "row89");
            assert_eq_bytes("row89: ietf_ext() != ietf_ext_xor_ic(zeros, ic=0)", &ext, &xz);
            iters += 1;
        }
    }
    assert!(iters >= 64, "row89 only ran {iters} inputs");
}

/// Row 89: `_ietf_ext_xor_ic` over the length sweep and the whole u32 ic range
/// (it has NO ic guard, only the SODIUM_SIZE_MAX one).
#[test]
fn r89_ietf_ext_xor_ic() {
    init_both();
    let mut rng = Rng::new(SEED ^ 0x89_E0);
    let keys = patterns(32, &mut rng);
    let nonces = patterns(12, &mut rng);
    const ICS: &[u32] = &[0, 1, 0xFFFF_FFFD, 0xFFFF_FFFE, 0xFFFF_FFFF];
    let mut iters = 0usize;
    for (li, &len) in lens().iter().enumerate() {
        let m = rng.bytes(len);
        for (ii, &ic) in ICS.iter().enumerate() {
            let k = &keys[(li + ii) % keys.len()];
            let n = &nonces[(li + 2 * ii) % nonces.len()];
            let d = xor_ic32("crypto_stream_chacha20_ietf_ext_xor_ic", &m, n, ic, k, "row89");
            let ip = xor_ic32_ip("crypto_stream_chacha20_ietf_ext_xor_ic", &m, n, ic, k, "row89");
            assert_eq_bytes("row89: in-place != disjoint", &d, &ip);
            let back = xor_ic32(
                "crypto_stream_chacha20_ietf_ext_xor_ic",
                &d,
                n,
                ic,
                k,
                "row89/inv",
            );
            assert_eq_bytes("row89: ext_xor_ic is not an involution", &m, &back);
            iters += 1;
        }
        if len >= 128 {
            // chunked property, staying below the 32-bit counter wrap
            chunked_ic32(
                "crypto_stream_chacha20_ietf_ext_xor_ic",
                &m,
                &nonces[3],
                &keys[3],
                7,
                64,
            );
        }
    }
    assert!(iters >= 64, "row89 ext_xor_ic only ran {iters} inputs");
}

/// Row 89 quirk (documented in `private/chacha20_ietf_ext.h`): when the 32-bit
/// counter overflows, `_ext` lets the carry run into the IV. The block after
/// `ic = 2^32-1` is therefore the `ic = 0` block of the nonce whose first
/// 32-bit little-endian word has been incremented.
#[test]
fn r89_ietf_ext_counter_overflows_into_iv() {
    init_both();
    let mut rng = Rng::new(SEED ^ 0x89_1F);
    for _ in 0..64 {
        let k = rng.bytes(32);
        let n = rng.bytes(12);
        let m = vec![0u8; 192];
        let at_max = xor_ic32(
            "crypto_stream_chacha20_ietf_ext_xor_ic",
            &m,
            &n,
            0xFFFF_FFFF,
            &k,
            "row89/ov",
        );
        let mut n2 = n.clone();
        let w = u32::from_le_bytes([n[0], n[1], n[2], n[3]]).wrapping_add(1);
        n2[0..4].copy_from_slice(&w.to_le_bytes());
        let carried = xor_ic32(
            "crypto_stream_chacha20_ietf_ext_xor_ic",
            &m[..128],
            &n2,
            0,
            &k,
            "row89/ov",
        );
        assert_eq_bytes(
            "row89: the ext 32-bit counter must carry into the IV",
            &at_max[64..],
            &carried,
        );
    }
}

/// Row 90: `crypto_stream_salsa20` / `_xor`, 8-byte nonce.
#[test]
fn r90_salsa20() {
    init_both();
    let mut rng = Rng::new(SEED ^ 90);
    let keys = patterns(32, &mut rng);
    let nonces = patterns(8, &mut rng);
    let fam = &FAMS[2];
    let mut iters = 0usize;
    for (li, &len) in lens().iter().enumerate() {
        let msgs = patterns(len, &mut rng);
        for (mi, m) in msgs.iter().enumerate() {
            let k = &keys[(li + mi) % keys.len()];
            let n = &nonces[(li + 2 * mi) % nonces.len()];
            family_consistency(fam, m, n, k, "row90");
            iters += 1;
        }
    }
    assert!(iters >= 64, "row90 only ran {iters} inputs");
}

/// Row 91: `crypto_stream_salsa20_xor_ic` — the counter is a manual 8-byte
/// little-endian carry chain with no wrap detection.
#[test]
fn r91_salsa20_xor_ic() {
    init_both();
    let mut rng = Rng::new(SEED ^ 91);
    let keys = patterns(32, &mut rng);
    let nonces = patterns(8, &mut rng);
    const ICS: &[u64] = &[0, 1, 0xFFFF_FFFF, 0x1_0000_0000, u64::MAX - 1, u64::MAX];
    let mut iters = 0usize;
    for (li, &len) in lens().iter().enumerate() {
        let m = rng.bytes(len);
        for (ii, &ic) in ICS.iter().enumerate() {
            let k = &keys[(li + ii) % keys.len()];
            let n = &nonces[(li + 2 * ii) % nonces.len()];
            let d = xor_ic64("crypto_stream_salsa20_xor_ic", &m, n, ic, k, "row91");
            let ip = xor_ic64_ip("crypto_stream_salsa20_xor_ic", &m, n, ic, k, "row91");
            assert_eq_bytes("row91: in-place != disjoint", &d, &ip);
            let back = xor_ic64("crypto_stream_salsa20_xor_ic", &d, n, ic, k, "row91/inv");
            assert_eq_bytes("row91: salsa20_xor_ic is not an involution", &m, &back);
            iters += 1;
        }
        if len >= 128 {
            for &ic in ICS {
                chunked_ic64("crypto_stream_salsa20_xor_ic", &m, &nonces[3], &keys[3], ic, 64);
            }
        }
    }
    assert!(iters >= 64, "row91 only ran {iters} inputs");
}

/// Row 91: the 8-byte carry chain must ripple across every byte boundary and
/// silently wrap past 2^64.
#[test]
fn r91_salsa20_carry_chain() {
    init_both();
    let mut rng = Rng::new(SEED ^ 0x91_CC);
    for _ in 0..64 {
        let k = rng.bytes(32);
        let n = rng.bytes(8);
        let m = vec![0u8; 128];
        // every byte boundary of the counter
        for sh in 0..8 {
            let ic = (1u64 << (8 * sh)) - 1; // 0xff, 0xffff, ... all-ones prefix
            let two = xor_ic64("crypto_stream_salsa20_xor_ic", &m, &n, ic, &k, "row91/carry");
            let next = xor_ic64(
                "crypto_stream_salsa20_xor_ic",
                &m[..64],
                &n,
                ic.wrapping_add(1),
                &k,
                "row91/carry",
            );
            assert_eq_bytes(
                &format!("row91: carry out of byte {sh} of the salsa20 counter is wrong"),
                &two[64..],
                &next,
            );
        }
        // wrap 2^64 -> 0 (no wrap detection in the C: the carry is just dropped)
        let w = xor_ic64("crypto_stream_salsa20_xor_ic", &m, &n, u64::MAX, &k, "row91/wrap");
        let z = xor_ic64("crypto_stream_salsa20_xor_ic", &m[..64], &n, 0, &k, "row91/wrap");
        assert_eq_bytes("row91: salsa20 counter must wrap 2^64 -> 0", &w[64..], &z);
    }
}

/// Row 92: `crypto_stream_salsa2012` / `_xor` (12 rounds, no `_xor_ic`).
#[test]
fn r92_salsa2012() {
    init_both();
    let mut rng = Rng::new(SEED ^ 92);
    let keys = patterns(32, &mut rng);
    let nonces = patterns(8, &mut rng);
    let fam = &FAMS[3];
    assert!(fam.ic64.is_none(), "salsa2012 has no _xor_ic");
    // there is no crypto_stream_salsa2012_xor_ic symbol in the C library
    let l = libs();
    for lib in [&l.c, &l.r] {
        assert!(
            unsafe { lib.get::<XorIc64Fn>(b"crypto_stream_salsa2012_xor_ic\0") }.is_err(),
            "row92: crypto_stream_salsa2012_xor_ic must not exist"
        );
    }
    let mut iters = 0usize;
    for (li, &len) in lens().iter().enumerate() {
        let msgs = patterns(len, &mut rng);
        for (mi, m) in msgs.iter().enumerate() {
            let k = &keys[(li + mi) % keys.len()];
            let n = &nonces[(li + 2 * mi) % nonces.len()];
            family_consistency(fam, m, n, k, "row92");
            // 12 rounds must differ from 20 rounds
            if len > 0 {
                let s20 = ks("crypto_stream_salsa20", len, n, k, "row92");
                let s12 = ks("crypto_stream_salsa2012", len, n, k, "row92");
                assert!(s20 != s12, "row92: salsa2012 == salsa20 keystream?!");
            }
            iters += 1;
        }
    }
    assert!(iters >= 64, "row92 only ran {iters} inputs");
}

/// Row 93: `crypto_stream_salsa208` / `_xor` (8 rounds, no `_xor_ic`).
#[test]
fn r93_salsa208() {
    init_both();
    let mut rng = Rng::new(SEED ^ 93);
    let keys = patterns(32, &mut rng);
    let nonces = patterns(8, &mut rng);
    let fam = &FAMS[4];
    assert!(fam.ic64.is_none(), "salsa208 has no _xor_ic");
    let l = libs();
    for lib in [&l.c, &l.r] {
        assert!(
            unsafe { lib.get::<XorIc64Fn>(b"crypto_stream_salsa208_xor_ic\0") }.is_err(),
            "row93: crypto_stream_salsa208_xor_ic must not exist"
        );
    }
    let mut iters = 0usize;
    for (li, &len) in lens().iter().enumerate() {
        let msgs = patterns(len, &mut rng);
        for (mi, m) in msgs.iter().enumerate() {
            let k = &keys[(li + mi) % keys.len()];
            let n = &nonces[(li + 2 * mi) % nonces.len()];
            family_consistency(fam, m, n, k, "row93");
            if len > 0 {
                let s12 = ks("crypto_stream_salsa2012", len, n, k, "row93");
                let s8 = ks("crypto_stream_salsa208", len, n, k, "row93");
                assert!(s12 != s8, "row93: salsa208 == salsa2012 keystream?!");
            }
            iters += 1;
        }
    }
    assert!(iters >= 64, "row93 only ran {iters} inputs");
}

/// Row 94: `crypto_stream_xsalsa20` / `_xor` / `_xor_ic`, 24-byte nonce;
/// HSalsa20 over `n[0..16)` then salsa20 with `n[16..24)`.
#[test]
fn r94_xsalsa20() {
    init_both();
    let mut rng = Rng::new(SEED ^ 94);
    let keys = patterns(32, &mut rng);
    let nonces = patterns(24, &mut rng);
    let fam = &FAMS[5];
    let hsalsa = unsafe { pair::<CoreFn>("crypto_core_hsalsa20") };
    let mut iters = 0usize;
    for (li, &len) in lens().iter().enumerate() {
        let msgs = patterns(len, &mut rng);
        for (mi, m) in msgs.iter().enumerate() {
            let k = &keys[(li + mi) % keys.len()];
            let n = &nonces[(li + 2 * mi) % nonces.len()];
            family_consistency(fam, m, n, k, "row94");
            // delegation: xsalsa20 == salsa20(subkey = HSalsa20(n[0..16), k), n[16..24))
            let mut sk_c = [0u8; 32];
            let mut sk_r = [0u8; 32];
            unsafe {
                hsalsa.0(sk_c.as_mut_ptr(), n.as_ptr(), k.as_ptr(), std::ptr::null());
                hsalsa.1(sk_r.as_mut_ptr(), n.as_ptr(), k.as_ptr(), std::ptr::null());
            }
            assert_eq_bytes("row94: crypto_core_hsalsa20 subkey differs", &sk_c, &sk_r);
            let via = ks("crypto_stream_salsa20", len, &n[16..24], &sk_c, "row94");
            let direct = ks("crypto_stream_xsalsa20", len, n, k, "row94");
            assert_eq_bytes("row94: xsalsa20 != salsa20(HSalsa20 subkey)", &via, &direct);
            // ic is passed straight through to salsa20
            for ic in [0u64, 1, 0x1_0000_0000, u64::MAX] {
                let a = xor_ic64("crypto_stream_xsalsa20_xor_ic", m, n, ic, k, "row94");
                let b = xor_ic64("crypto_stream_salsa20_xor_ic", m, &n[16..24], ic, &sk_c, "row94");
                assert_eq_bytes("row94: xsalsa20_xor_ic does not forward ic", &a, &b);
            }
            iters += 1;
        }
    }
    assert!(iters >= 64, "row94 only ran {iters} inputs");
}

/// Row 95: `crypto_stream_xchacha20` / `_xor` / `_xor_ic`, 24-byte nonce;
/// HChacha20 over `n[0..16)` then chacha20 with `n[16..24)`.
#[test]
fn r95_xchacha20() {
    init_both();
    let mut rng = Rng::new(SEED ^ 95);
    let keys = patterns(32, &mut rng);
    let nonces = patterns(24, &mut rng);
    let fam = &FAMS[6];
    let hchacha = unsafe { pair::<CoreFn>("crypto_core_hchacha20") };
    let mut iters = 0usize;
    for (li, &len) in lens().iter().enumerate() {
        let msgs = patterns(len, &mut rng);
        for (mi, m) in msgs.iter().enumerate() {
            let k = &keys[(li + mi) % keys.len()];
            let n = &nonces[(li + 2 * mi) % nonces.len()];
            family_consistency(fam, m, n, k, "row95");
            let mut sk_c = [0u8; 32];
            let mut sk_r = [0u8; 32];
            unsafe {
                hchacha.0(sk_c.as_mut_ptr(), n.as_ptr(), k.as_ptr(), std::ptr::null());
                hchacha.1(sk_r.as_mut_ptr(), n.as_ptr(), k.as_ptr(), std::ptr::null());
            }
            assert_eq_bytes("row95: crypto_core_hchacha20 subkey differs", &sk_c, &sk_r);
            let via = ks("crypto_stream_chacha20", len, &n[16..24], &sk_c, "row95");
            let direct = ks("crypto_stream_xchacha20", len, n, k, "row95");
            assert_eq_bytes("row95: xchacha20 != chacha20(HChacha20 subkey)", &via, &direct);
            for ic in [0u64, 1, 0xFFFF_FFFF, 0x1_0000_0000, u64::MAX] {
                let a = xor_ic64("crypto_stream_xchacha20_xor_ic", m, n, ic, k, "row95");
                let b = xor_ic64("crypto_stream_chacha20_xor_ic", m, &n[16..24], ic, &sk_c, "row95");
                assert_eq_bytes("row95: xchacha20_xor_ic does not forward ic", &a, &b);
            }
            iters += 1;
        }
    }
    assert!(iters >= 64, "row95 only ran {iters} inputs");
}

/// Row 96: `crypto_stream` / `crypto_stream_xor` must be exactly xsalsa20.
#[test]
fn r96_crypto_stream_dispatch() {
    init_both();
    let mut rng = Rng::new(SEED ^ 96);
    let keys = patterns(32, &mut rng);
    let nonces = patterns(24, &mut rng);
    let fam = &FAMS[7];
    let mut iters = 0usize;
    for (li, &len) in lens().iter().enumerate() {
        let msgs = patterns(len, &mut rng);
        for (mi, m) in msgs.iter().enumerate() {
            let k = &keys[(li + mi) % keys.len()];
            let n = &nonces[(li + 2 * mi) % nonces.len()];
            family_consistency(fam, m, n, k, "row96");
            let a = ks("crypto_stream", len, n, k, "row96");
            let b = ks("crypto_stream_xsalsa20", len, n, k, "row96");
            assert_eq_bytes("row96: crypto_stream != crypto_stream_xsalsa20", &a, &b);
            let c = xor("crypto_stream_xor", m, n, k, "row96");
            let d = xor("crypto_stream_xsalsa20_xor", m, n, k, "row96");
            assert_eq_bytes("row96: crypto_stream_xor != crypto_stream_xsalsa20_xor", &c, &d);
            iters += 1;
        }
    }
    assert!(iters >= 64, "row96 only ran {iters} inputs");
}

/// Row 97: the full key-pattern × nonce-pattern cross product for every entry
/// point of every family (all-0x00 / all-0xff / counter / 2 random each).
#[test]
fn r97_key_nonce_pattern_matrix() {
    init_both();
    let mut rng = Rng::new(SEED ^ 97);
    let keys = patterns(32, &mut rng);
    let mut iters = 0usize;
    for fam in FAMS {
        let nonces = patterns(fam.nb, &mut rng);
        for &len in PLENS {
            let m = rng.bytes(len);
            for k in &keys {
                for n in &nonces {
                    family_consistency(fam, &m, n, k, "row97");
                    if let Some(icn) = fam.ic64 {
                        for ic in [1u64, 0xFFFF_FFFF, 0x1_0000_0000, u64::MAX] {
                            xor_ic64(icn, &m, n, ic, k, "row97");
                        }
                    }
                    iters += 1;
                }
            }
        }
    }
    // the two u32-ic entry points
    let nonces = patterns(12, &mut rng);
    for &len in PLENS {
        let m = rng.bytes(len);
        let limit = ietf_ic_limit(len as u64);
        for k in &keys {
            for n in &nonces {
                for ic in [0u64, 1, 0xFFFF_FFFE, 0xFFFF_FFFF] {
                    xor_ic32("crypto_stream_chacha20_ietf_ext_xor_ic", &m, n, ic as u32, k, "row97");
                    if ic <= limit {
                        xor_ic32("crypto_stream_chacha20_ietf_xor_ic", &m, n, ic as u32, k, "row97");
                    }
                }
                ks("crypto_stream_chacha20_ietf_ext", len, n, k, "row97");
                iters += 1;
            }
        }
    }
    assert!(iters >= 64, "row97 only ran {iters} inputs");
}

// =========================================================== rows 245–257

/// Rows 245 / 246 / 247 / 252: `len > crypto_stream_chacha20_MESSAGEBYTES_MAX`.
/// `SODIUM_SIZE_MAX == min(UINT64_MAX, SIZE_MAX) == UINT64_MAX` on this target
/// and the argument is `unsigned long long`, so the comparison is provably
/// always false: the guard can NEVER fire. Both libraries must therefore walk
/// into the buffer and fault instead of calling `sodium_misuse()`.
#[test]
fn e245_247_252_size_max_guards_unreachable() {
    init_both();
    // the bound really is UINT64_MAX in both libraries
    let (c, r) = unsafe { pair::<SizeFn>("crypto_stream_chacha20_messagebytes_max") };
    let (vc, vr) = unsafe { (c(), r()) };
    assert_eq!(vc, vr);
    assert_eq!(vc as u64, u64::MAX, "SODIUM_SIZE_MAX is not UINT64_MAX here");

    let s = scratch(4096);
    let p = s.p;
    let k = [0x42u8; 32];
    let n = [0x24u8; 12];
    let kp = k.as_ptr();
    let np = n.as_ptr();

    // row 245: keystream
    expect_outcome::<StreamFn, _>(
        "ERRORS 245 crypto_stream_chacha20(clen=2^64-1) must NOT misuse",
        "crypto_stream_chacha20",
        move |f| unsafe { f(p, u64::MAX, np, kp) as i64 },
        NO_MISUSE,
    );
    // row 246: _xor_ic
    expect_outcome::<XorIc64Fn, _>(
        "ERRORS 246 crypto_stream_chacha20_xor_ic(mlen=2^64-1) must NOT misuse",
        "crypto_stream_chacha20_xor_ic",
        move |f| unsafe { f(p, p, u64::MAX, np, 7, kp) as i64 },
        NO_MISUSE,
    );
    // row 247: _xor
    expect_outcome::<XorFn, _>(
        "ERRORS 247 crypto_stream_chacha20_xor(mlen=2^64-1) must NOT misuse",
        "crypto_stream_chacha20_xor",
        move |f| unsafe { f(p, p, u64::MAX, np, kp) as i64 },
        NO_MISUSE,
    );
    // row 252: the _ext pair uses the NON-ietf bound, so also unreachable
    expect_outcome::<StreamFn, _>(
        "ERRORS 252 crypto_stream_chacha20_ietf_ext(clen=2^64-1) must NOT misuse",
        "crypto_stream_chacha20_ietf_ext",
        move |f| unsafe { f(p, u64::MAX, np, kp) as i64 },
        NO_MISUSE,
    );
    expect_outcome::<XorIc32Fn, _>(
        "ERRORS 252 crypto_stream_chacha20_ietf_ext_xor_ic(mlen=2^64-1) must NOT misuse",
        "crypto_stream_chacha20_ietf_ext_xor_ic",
        move |f| unsafe { f(p, p, u64::MAX, np, 3, kp) as i64 },
        NO_MISUSE,
    );
    // ... and a plain in-range call still returns 0 in both libraries
    let mut rng = Rng::new(SEED ^ 245);
    for _ in 0..64 {
        let k = rng.bytes(32);
        let n = rng.bytes(8);
        ks("crypto_stream_chacha20", 1024, &n, &k, "row245");
    }
}

/// Row 248: `crypto_stream_chacha20_ietf` rejects `clen > 64*2^32`.
/// The boundary is exact: `clen == 64*2^32` passes the guard.
#[test]
fn e248_ietf_clen_bound() {
    init_both();
    const IETF_MAX: u64 = 64 * (1u64 << 32);
    let s = scratch(4096);
    let p = s.p;
    let k = [0x11u8; 32];
    let n = [0x22u8; 12];
    let kp = k.as_ptr();
    let np = n.as_ptr();

    expect_outcome::<StreamFn, _>(
        "ERRORS 248 crypto_stream_chacha20_ietf(clen=64*2^32+1) must sodium_misuse()",
        "crypto_stream_chacha20_ietf",
        move |f| unsafe { f(p, IETF_MAX + 1, np, kp) as i64 },
        MISUSE,
    );
    expect_outcome::<StreamFn, _>(
        "ERRORS 248 crypto_stream_chacha20_ietf(clen=2^64-1) must sodium_misuse()",
        "crypto_stream_chacha20_ietf",
        move |f| unsafe { f(p, u64::MAX, np, kp) as i64 },
        MISUSE,
    );
    // exactly at the limit the guard must NOT fire (it is `>` not `>=`)
    expect_outcome::<StreamFn, _>(
        "ERRORS 248 crypto_stream_chacha20_ietf(clen=64*2^32) must NOT misuse",
        "crypto_stream_chacha20_ietf",
        move |f| unsafe { f(p, IETF_MAX, np, kp) as i64 },
        NO_MISUSE,
    );
    // in-range lengths keep returning 0
    let mut rng = Rng::new(SEED ^ 248);
    for _ in 0..64 {
        let k = rng.bytes(32);
        let n = rng.bytes(12);
        ks("crypto_stream_chacha20_ietf", 4096, &n, &k, "row248");
    }
}

/// Row 251: `crypto_stream_chacha20_ietf_xor` rejects `mlen > 64*2^32`.
#[test]
fn e251_ietf_xor_mlen_bound() {
    init_both();
    const IETF_MAX: u64 = 64 * (1u64 << 32);
    let s = scratch(4096);
    let p = s.p;
    let k = [0x33u8; 32];
    let n = [0x44u8; 12];
    let kp = k.as_ptr();
    let np = n.as_ptr();

    expect_outcome::<XorFn, _>(
        "ERRORS 251 crypto_stream_chacha20_ietf_xor(mlen=64*2^32+1) must sodium_misuse()",
        "crypto_stream_chacha20_ietf_xor",
        move |f| unsafe { f(p, p, IETF_MAX + 1, np, kp) as i64 },
        MISUSE,
    );
    expect_outcome::<XorFn, _>(
        "ERRORS 251 crypto_stream_chacha20_ietf_xor(mlen=2^64-1) must sodium_misuse()",
        "crypto_stream_chacha20_ietf_xor",
        move |f| unsafe { f(p, p, u64::MAX, np, kp) as i64 },
        MISUSE,
    );
    expect_outcome::<XorFn, _>(
        "ERRORS 251 crypto_stream_chacha20_ietf_xor(mlen=64*2^32) must NOT misuse",
        "crypto_stream_chacha20_ietf_xor",
        move |f| unsafe { f(p, p, IETF_MAX, np, kp) as i64 },
        NO_MISUSE,
    );
    let mut rng = Rng::new(SEED ^ 251);
    for _ in 0..64 {
        let k = rng.bytes(32);
        let n = rng.bytes(12);
        let m = rng.bytes(4096);
        xor("crypto_stream_chacha20_ietf_xor", &m, &n, &k, "row251");
    }
}

/// Row 249: `crypto_stream_chacha20_ietf_xor_ic` aborts when the 32-bit block
/// counter would wrap, i.e. `ic > 2^32 - ceil(mlen/64)`. The last legal value
/// must succeed (row 88) and `+1` must `sodium_misuse()` in BOTH libraries.
#[test]
fn e249_ietf_xor_ic_guard() {
    init_both();
    let s = scratch(8192);
    let p = s.p;
    let k = [0x55u8; 32];
    let n = [0x66u8; 12];
    let kp = k.as_ptr();
    let np = n.as_ptr();

    for mlen in [65u64, 128, 129, 192, 1024, 4096, 8192] {
        let limit = ietf_ic_limit(mlen);
        assert!(limit < u32::MAX as u64, "mlen={mlen} has no illegal ic");
        let bad = (limit + 1) as u32;
        expect_outcome::<XorIc32Fn, _>(
            &format!(
                "ERRORS 249 ietf_xor_ic(mlen={mlen}, ic={bad:#x} = limit+1) must sodium_misuse()"
            ),
            "crypto_stream_chacha20_ietf_xor_ic",
            move |f| unsafe { f(p, p, mlen, np, bad, kp) as i64 },
            MISUSE,
        );
        // u32::MAX is illegal for every mlen > 64
        expect_outcome::<XorIc32Fn, _>(
            &format!("ERRORS 249 ietf_xor_ic(mlen={mlen}, ic=2^32-1) must sodium_misuse()"),
            "crypto_stream_chacha20_ietf_xor_ic",
            move |f| unsafe { f(p, p, mlen, np, u32::MAX, kp) as i64 },
            MISUSE,
        );
    }
    // mlen <= 64 => limit == 2^32-1 (or 2^32 for mlen == 0): nothing is illegal
    let mut rng = Rng::new(SEED ^ 249);
    let mut iters = 0usize;
    for mlen in [0usize, 1, 32, 63, 64] {
        for _ in 0..14 {
            let k = rng.bytes(32);
            let n = rng.bytes(12);
            let m = rng.bytes(mlen);
            xor_ic32(
                "crypto_stream_chacha20_ietf_xor_ic",
                &m,
                &n,
                u32::MAX,
                &k,
                "row249/legal",
            );
            iters += 1;
        }
    }
    assert!(iters >= 64, "row249 legal side only ran {iters} inputs");
    // last-legal succeeds, +1 aborts: exact boundary, checked above and in
    // r88_ietf_xor_ic_last_legal_ic.
}

/// Row 250 QUIRK: `crypto_stream_chacha20_ietf_xor_ic` has no `mlen` check of
/// its own, so for `mlen > 64*2^32` the unsigned subtraction
/// `2^32 - (mlen+63)/64` underflows to a huge value and the ic guard can never
/// fire. Not allocatable (256 GiB), so the observable consequence is that the
/// call runs on and faults instead of aborting — exactly what a forked child
/// with a SIGSEGV marker distinguishes.
#[test]
fn e250_ietf_xor_ic_guard_underflow_quirk() {
    init_both();
    const IETF_MAX: u64 = 64 * (1u64 << 32); // 274877906944
    let s = scratch(4096);
    let p = s.p;
    let k = [0x77u8; 32];
    let n = [0x88u8; 12];
    let kp = k.as_ptr();
    let np = n.as_ptr();

    // mlen == 64*2^32 exactly: (mlen+63)/64 == 2^32, so the limit is 0.
    // ic == 0 is legal, ic == 1 aborts. This pins the arithmetic one step
    // before the underflow.
    expect_outcome::<XorIc32Fn, _>(
        "ERRORS 250 ietf_xor_ic(mlen=64*2^32, ic=0): limit is exactly 0, no misuse",
        "crypto_stream_chacha20_ietf_xor_ic",
        move |f| unsafe { f(p, p, IETF_MAX, np, 0, kp) as i64 },
        NO_MISUSE,
    );
    expect_outcome::<XorIc32Fn, _>(
        "ERRORS 250 ietf_xor_ic(mlen=64*2^32, ic=1) must sodium_misuse()",
        "crypto_stream_chacha20_ietf_xor_ic",
        move |f| unsafe { f(p, p, IETF_MAX, np, 1, kp) as i64 },
        MISUSE,
    );
    // one byte further the subtraction underflows: the guard NEVER fires again,
    // not even for ic == 2^32-1.
    expect_outcome::<XorIc32Fn, _>(
        "ERRORS 250 QUIRK ietf_xor_ic(mlen=64*2^32+1, ic=2^32-1) must NOT misuse \
         (2^32 - (mlen+63)/64 underflows)",
        "crypto_stream_chacha20_ietf_xor_ic",
        move |f| unsafe { f(p, p, IETF_MAX + 1, np, u32::MAX, kp) as i64 },
        NO_MISUSE,
    );
    // and `mlen + 63` itself wraps for mlen close to 2^64-1, which also
    // defeats the guard.
    expect_outcome::<XorIc32Fn, _>(
        "ERRORS 250 QUIRK ietf_xor_ic(mlen=2^64-1, ic=2^32-1) must NOT misuse \
         (mlen+63 wraps to 62)",
        "crypto_stream_chacha20_ietf_xor_ic",
        move |f| unsafe { f(p, p, u64::MAX, np, u32::MAX, kp) as i64 },
        NO_MISUSE,
    );
    expect_outcome::<XorIc32Fn, _>(
        "ERRORS 250 QUIRK ietf_xor_ic(mlen=2^64-64, ic=2^32-1) must NOT misuse",
        "crypto_stream_chacha20_ietf_xor_ic",
        move |f| unsafe { f(p, p, u64::MAX - 63, np, u32::MAX, kp) as i64 },
        NO_MISUSE,
    );
}

/// Row 253: salsa20 has NO bounds check at all — every allocatable length
/// returns 0, and even `2^64-1` does not abort (it faults).
#[test]
fn e253_salsa20_no_bounds_check() {
    init_both();
    let mut rng = Rng::new(SEED ^ 253);
    for it in 0..64 {
        let k = rng.bytes(32);
        let n = rng.bytes(8);
        // "large but allocatable" (one pass at 1 MiB is enough for the biggest)
        let big: &[usize] = if it == 0 { &[1 << 16, 1 << 20] } else { &[1 << 16] };
        for &len in big {
            let stream = ks("crypto_stream_salsa20", len, &n, &k, "row253");
            let m = vec![0x5Au8; len];
            let d = xor("crypto_stream_salsa20_xor", &m, &n, &k, "row253");
            assert_eq_bytes("row253: xor != m ^ keystream", &x(&m, &stream), &d);
            let ic = xor_ic64("crypto_stream_salsa20_xor_ic", &m, &n, 0, &k, "row253");
            assert_eq_bytes("row253: xor_ic(0) != xor", &d, &ic);
        }
    }
    let s = scratch(4096);
    let p = s.p;
    let k = [0x99u8; 32];
    let n = [0xAAu8; 8];
    let kp = k.as_ptr();
    let np = n.as_ptr();
    expect_outcome::<StreamFn, _>(
        "ERRORS 253 crypto_stream_salsa20(clen=2^64-1): no bounds check, must NOT misuse",
        "crypto_stream_salsa20",
        move |f| unsafe { f(p, u64::MAX, np, kp) as i64 },
        NO_MISUSE,
    );
    expect_outcome::<XorFn, _>(
        "ERRORS 253 crypto_stream_salsa20_xor(mlen=2^64-1): no bounds check, must NOT misuse",
        "crypto_stream_salsa20_xor",
        move |f| unsafe { f(p, p, u64::MAX, np, kp) as i64 },
        NO_MISUSE,
    );
    expect_outcome::<XorIc64Fn, _>(
        "ERRORS 253 crypto_stream_salsa20_xor_ic(mlen=2^64-1): no bounds check, must NOT misuse",
        "crypto_stream_salsa20_xor_ic",
        move |f| unsafe { f(p, p, u64::MAX, np, u64::MAX, kp) as i64 },
        NO_MISUSE,
    );
}

/// Row 254: salsa2012 / salsa208 have NO bounds check either, and `len == 0`
/// is an early return.
#[test]
fn e254_salsa2012_salsa208_no_bounds_check() {
    init_both();
    let mut rng = Rng::new(SEED ^ 254);
    for it in 0..64 {
        let k = rng.bytes(32);
        let n = rng.bytes(8);
        for name in ["crypto_stream_salsa2012", "crypto_stream_salsa208"] {
            let len = if it == 0 { 1usize << 20 } else { 1usize << 16 };
            let stream = ks(name, len, &n, &k, "row254");
            let m = vec![0xA5u8; len];
            let xn = format!("{name}_xor");
            let d = xor(&xn, &m, &n, &k, "row254");
            assert_eq_bytes("row254: xor != m ^ keystream", &x(&m, &stream), &d);
            // len == 0 early return
            let e = ks(name, 0, &n, &k, "row254/zero");
            assert!(e.is_empty());
        }
    }
    let s = scratch(4096);
    let p = s.p;
    let k = [0xBBu8; 32];
    let n = [0xCCu8; 8];
    let kp = k.as_ptr();
    let np = n.as_ptr();
    for name in ["crypto_stream_salsa2012", "crypto_stream_salsa208"] {
        expect_outcome::<StreamFn, _>(
            &format!("ERRORS 254 {name}(clen=2^64-1): no bounds check, must NOT misuse"),
            name,
            move |f| unsafe { f(p, u64::MAX, np, kp) as i64 },
            NO_MISUSE,
        );
        let xn = format!("{name}_xor");
        expect_outcome::<XorFn, _>(
            &format!("ERRORS 254 {xn}(mlen=2^64-1): no bounds check, must NOT misuse"),
            &xn,
            move |f| unsafe { f(p, p, u64::MAX, np, kp) as i64 },
            NO_MISUSE,
        );
    }
}

/// Row 255: xsalsa20 has no check of its own and delegates to salsa20, which
/// never fails — every entry point returns 0.
#[test]
fn e255_xsalsa20_never_fails() {
    init_both();
    let mut rng = Rng::new(SEED ^ 255);
    let mut iters = 0usize;
    for _ in 0..64 {
        let k = rng.bytes(32);
        let n = rng.bytes(24);
        let len = *rng.pick(&[0usize, 1, 63, 64, 65, 1024, 65536]);
        let m = rng.bytes(len);
        ks("crypto_stream_xsalsa20", len, &n, &k, "row255");
        xor("crypto_stream_xsalsa20_xor", &m, &n, &k, "row255");
        xor_ic64("crypto_stream_xsalsa20_xor_ic", &m, &n, u64::MAX, &k, "row255");
        ks("crypto_stream", len, &n, &k, "row255");
        xor("crypto_stream_xor", &m, &n, &k, "row255");
        iters += 1;
    }
    assert!(iters >= 64);
    let s = scratch(4096);
    let p = s.p;
    let k = [0xDDu8; 32];
    let n = [0xEEu8; 24];
    let kp = k.as_ptr();
    let np = n.as_ptr();
    expect_outcome::<StreamFn, _>(
        "ERRORS 255 crypto_stream_xsalsa20(clen=2^64-1) must NOT misuse",
        "crypto_stream_xsalsa20",
        move |f| unsafe { f(p, u64::MAX, np, kp) as i64 },
        NO_MISUSE,
    );
    expect_outcome::<XorIc64Fn, _>(
        "ERRORS 255 crypto_stream_xsalsa20_xor_ic(mlen=2^64-1) must NOT misuse",
        "crypto_stream_xsalsa20_xor_ic",
        move |f| unsafe { f(p, p, u64::MAX, np, 1, kp) as i64 },
        NO_MISUSE,
    );
    expect_outcome::<StreamFn, _>(
        "ERRORS 255 crypto_stream(clen=2^64-1) must NOT misuse",
        "crypto_stream",
        move |f| unsafe { f(p, u64::MAX, np, kp) as i64 },
        NO_MISUSE,
    );
}

/// Row 256: xchacha20 delegates to chacha20, whose `> SODIUM_SIZE_MAX` guard is
/// unreachable, so it returns 0 for every reachable length and never aborts.
#[test]
fn e256_xchacha20_delegates() {
    init_both();
    let mut rng = Rng::new(SEED ^ 256);
    let mut iters = 0usize;
    for _ in 0..64 {
        let k = rng.bytes(32);
        let n = rng.bytes(24);
        let len = *rng.pick(&[0usize, 1, 63, 64, 65, 1024, 65536]);
        let m = rng.bytes(len);
        ks("crypto_stream_xchacha20", len, &n, &k, "row256");
        xor("crypto_stream_xchacha20_xor", &m, &n, &k, "row256");
        xor_ic64("crypto_stream_xchacha20_xor_ic", &m, &n, u64::MAX, &k, "row256");
        iters += 1;
    }
    assert!(iters >= 64);
    let s = scratch(4096);
    let p = s.p;
    let k = [0x0Fu8; 32];
    let n = [0xF0u8; 24];
    let kp = k.as_ptr();
    let np = n.as_ptr();
    expect_outcome::<StreamFn, _>(
        "ERRORS 256 crypto_stream_xchacha20(clen=2^64-1) must NOT misuse",
        "crypto_stream_xchacha20",
        move |f| unsafe { f(p, u64::MAX, np, kp) as i64 },
        NO_MISUSE,
    );
    expect_outcome::<XorIc64Fn, _>(
        "ERRORS 256 crypto_stream_xchacha20_xor_ic(mlen=2^64-1) must NOT misuse",
        "crypto_stream_xchacha20_xor_ic",
        move |f| unsafe { f(p, p, u64::MAX, np, u64::MAX, kp) as i64 },
        NO_MISUSE,
    );
}

/// Row 257: `clen == 0` / `mlen == 0` is an early return in every `stream_ref*`
/// and every wrapper — NOTHING may be written. The output buffer is prefilled
/// with 0xAA and must come back untouched from both libraries.
#[test]
fn e257_zero_length_writes_nothing() {
    init_both();
    let mut rng = Rng::new(SEED ^ 257);
    let mut iters = 0usize;
    for _ in 0..8 {
        let k = rng.bytes(32);
        for fam in FAMS {
            let n = rng.bytes(fam.nb);
            // `ks`/`xor`/`xor_ic*` all prefill len+PAD bytes with 0xAA, compare
            // the FULL buffer across libraries and assert nothing past `len`
            // (here: nothing at all) was written.
            assert!(ks(fam.ks, 0, &n, &k, "row257").is_empty());
            assert!(xor(fam.xor, &[], &n, &k, "row257").is_empty());
            assert!(xor_ip(fam.xor, &[], &n, &k, "row257").is_empty());
            if let Some(icn) = fam.ic64 {
                for ic in [0u64, 1, u64::MAX] {
                    assert!(xor_ic64(icn, &[], &n, ic, &k, "row257").is_empty());
                    assert!(xor_ic64_ip(icn, &[], &n, ic, &k, "row257").is_empty());
                }
            }
            iters += 1;
        }
        let n12 = rng.bytes(12);
        assert!(ks("crypto_stream_chacha20_ietf_ext", 0, &n12, &k, "row257").is_empty());
        for ic in [0u32, 1, u32::MAX] {
            assert!(
                xor_ic32("crypto_stream_chacha20_ietf_ext_xor_ic", &[], &n12, ic, &k, "row257")
                    .is_empty()
            );
            // mlen == 0 makes the ietf ic guard limit 2^32, so every ic is legal
            assert!(
                xor_ic32("crypto_stream_chacha20_ietf_xor_ic", &[], &n12, ic, &k, "row257")
                    .is_empty()
            );
            assert!(
                xor_ic32_ip("crypto_stream_chacha20_ietf_xor_ic", &[], &n12, ic, &k, "row257")
                    .is_empty()
            );
        }
        iters += 1;
    }
    assert!(iters >= 64, "row257 only ran {iters} inputs");
}

/// The internal implementation pickers. This build defines no `HAVE_*` macros,
/// so both must select the portable reference implementation, return 0, and
/// leave the produced keystream unchanged.
#[test]
fn internal_pick_best_implementation() {
    init_both();
    let mut rng = Rng::new(SEED ^ 0xB357);
    let k = rng.bytes(32);
    let n8 = rng.bytes(8);
    let n12 = rng.bytes(12);
    let before_cc = ks("crypto_stream_chacha20", 512, &n8, &k, "pick/before");
    let before_ietf = ks("crypto_stream_chacha20_ietf", 512, &n12, &k, "pick/before");
    let before_s20 = ks("crypto_stream_salsa20", 512, &n8, &k, "pick/before");

    for name in [
        "_crypto_stream_chacha20_pick_best_implementation",
        "_crypto_stream_salsa20_pick_best_implementation",
    ] {
        let (c, r) = unsafe { pair::<unsafe extern "C" fn() -> c_int>(name) };
        for _ in 0..64 {
            let (vc, vr) = unsafe { (c(), r()) };
            assert_eq!(vc, vr, "{name}: C={vc} rust={vr}");
            assert_eq!(vc, 0, "{name}: C returned {vc}, expected 0");
        }
    }
    // the exported implementation structs must exist in both libraries
    let l = libs();
    for name in [
        "crypto_stream_chacha20_ref_implementation",
        "crypto_stream_salsa20_ref_implementation",
    ] {
        unsafe {
            sym::<*const u8>(&l.c, name);
            sym::<*const u8>(&l.r, name);
        }
    }
    let after_cc = ks("crypto_stream_chacha20", 512, &n8, &k, "pick/after");
    let after_ietf = ks("crypto_stream_chacha20_ietf", 512, &n12, &k, "pick/after");
    let after_s20 = ks("crypto_stream_salsa20", 512, &n8, &k, "pick/after");
    assert_eq_bytes("pick_best changed the chacha20 keystream", &before_cc, &after_cc);
    assert_eq_bytes("pick_best changed the ietf keystream", &before_ietf, &after_ietf);
    assert_eq_bytes("pick_best changed the salsa20 keystream", &before_s20, &after_s20);
}

/// Requirement 4 across every `_xor_ic` entry point: this is precisely how the
/// AEAD constructions drive the stream ciphers (block 0 for the Poly1305 key,
/// then the payload from block 1).
#[test]
fn chunked_vs_single_all_entry_points() {
    init_both();
    let mut rng = Rng::new(SEED ^ 0xC4C4);
    let mut iters = 0usize;
    for _ in 0..16 {
        let k = rng.bytes(32);
        let n8 = rng.bytes(8);
        let n12 = rng.bytes(12);
        let n24 = rng.bytes(24);
        for &len in &[128usize, 192, 256, 512, 1024] {
            let m = rng.bytes(len);
            for &ic in &[0u64, 1, 0xFFFF_FFFE, 0xFFFF_FFFF, 0x1_0000_0000, u64::MAX] {
                chunked_ic64("crypto_stream_chacha20_xor_ic", &m, &n8, &k, ic, 64);
                chunked_ic64("crypto_stream_salsa20_xor_ic", &m, &n8, &k, ic, 64);
                chunked_ic64("crypto_stream_xsalsa20_xor_ic", &m, &n24, &k, ic, 64);
                chunked_ic64("crypto_stream_xchacha20_xor_ic", &m, &n24, &k, ic, 64);
                chunked_ic64("crypto_stream_chacha20_xor_ic", &m, &n8, &k, ic, (len / 128) * 64);
                iters += 1;
            }
            // u32 counters: stay inside the legal window for the guarded entry
            for &ic in &[0u32, 1, 0x7FFF_FFFF] {
                chunked_ic32("crypto_stream_chacha20_ietf_xor_ic", &m, &n12, &k, ic, 64);
                chunked_ic32("crypto_stream_chacha20_ietf_ext_xor_ic", &m, &n12, &k, ic, 64);
                iters += 1;
            }
            // ... and right up against the 32-bit wrap: the last block consumed
            // is block 2^32-1, the last one before the counter carries into the
            // IV (that carry itself is pinned by
            // `r89_ietf_ext_counter_overflows_into_iv`).
            let top = 0xFFFF_FFFFu32 - (len / 64) as u32 + 1;
            chunked_ic32("crypto_stream_chacha20_ietf_ext_xor_ic", &m, &n12, &k, top, 64);
            iters += 1;
        }
    }
    assert!(iters >= 64, "chunked property only ran {iters} inputs");
}
