//! Differential tests for `crypto_pwhash` (argon2i / argon2id / scryptsalsa208sha256).
//!
//! Every exported symbol of
//!   crypto_pwhash/crypto_pwhash.c
//!   crypto_pwhash/argon2/{argon2,argon2-core,argon2-encoding,argon2-fill-block-ref,
//!                          blake2b-long,pwhash_argon2i,pwhash_argon2id}.c
//!   crypto_pwhash/scryptsalsa208sha256/{crypto_scrypt-common,pbkdf2-sha256,
//!                          pwhash_scryptsalsa208sha256,scrypt_platform}.c
//!   crypto_pwhash/scryptsalsa208sha256/nosse/pwhash_scryptsalsa208sha256_nosse.c
//! is exercised here.
//!
//! NOTE ON SPEED: pwhash is intentionally slow, so the *minimum* parameters the
//! C accepts are used everywhere (argon2: opslimit 1..3, memlimit 8192;
//! scrypt: tiny N/r/p through the low-level entry points).
//!
//! NOTE ON RANDOMNESS: `crypto_pwhash_*_str` and `escrypt_r` call
//! `randombytes_buf()`. To get byte-exact comparability a deterministic
//! `randombytes_implementation` is installed into BOTH shared objects (each with
//! its own counter, reset to the same value before each C/Rust pair of calls).

#![allow(non_camel_case_types, non_snake_case)]

#[macro_use]
mod common;

use core::ffi::{c_char, c_int, c_void};
use std::ffi::CStr;
use std::sync::atomic::{AtomicU64, Ordering};

// ===========================================================================
// FFI types
// ===========================================================================

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Argon2Context {
    out: *mut u8,
    outlen: u32,
    pwd: *mut u8,
    pwdlen: u32,
    salt: *mut u8,
    saltlen: u32,
    secret: *mut u8,
    secretlen: u32,
    ad: *mut u8,
    adlen: u32,
    t_cost: u32,
    m_cost: u32,
    lanes: u32,
    threads: u32,
    flags: u32,
}

impl Argon2Context {
    fn zeroed() -> Self {
        Argon2Context {
            out: std::ptr::null_mut(),
            outlen: 0,
            pwd: std::ptr::null_mut(),
            pwdlen: 0,
            salt: std::ptr::null_mut(),
            saltlen: 0,
            secret: std::ptr::null_mut(),
            secretlen: 0,
            ad: std::ptr::null_mut(),
            adlen: 0,
            t_cost: 0,
            m_cost: 0,
            lanes: 0,
            threads: 0,
            flags: 0,
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy)]
struct BlockRegion {
    base: *mut c_void,
    memory: *mut c_void,
    size: usize,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct Argon2Instance {
    region: *mut BlockRegion,
    pseudo_rands: *mut u64,
    passes: u32,
    current_pass: u32,
    memory_blocks: u32,
    segment_length: u32,
    lane_length: u32,
    lanes: u32,
    threads: u32,
    type_: c_int,
    print_internals: c_int,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct Argon2Position {
    pass: u32,
    lane: u32,
    slice: u8,
    index: u32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct EscryptRegion {
    base: *mut c_void,
    aligned: *mut c_void,
    size: usize,
}

impl EscryptRegion {
    fn new() -> Self {
        EscryptRegion { base: std::ptr::null_mut(), aligned: std::ptr::null_mut(), size: 0 }
    }
}

#[repr(C)]
struct RbImpl {
    implementation_name: Option<unsafe extern "C" fn() -> *const c_char>,
    random: Option<unsafe extern "C" fn() -> u32>,
    stir: Option<unsafe extern "C" fn()>,
    uniform: Option<unsafe extern "C" fn(u32) -> u32>,
    buf: Option<unsafe extern "C" fn(*mut c_void, usize)>,
    close: Option<unsafe extern "C" fn() -> c_int>,
}
unsafe impl Sync for RbImpl {}

// ---------------------------------------------------------------- fn types --

type FnInt = unsafe extern "C" fn() -> c_int;
type FnSize = unsafe extern "C" fn() -> usize;
type FnU64 = unsafe extern "C" fn() -> u64;
type FnCStr = unsafe extern "C" fn() -> *const c_char;

type FnPwhash =
    unsafe extern "C" fn(*mut u8, u64, *const c_char, u64, *const u8, u64, usize, c_int) -> c_int;
type FnPwhashStr = unsafe extern "C" fn(*mut c_char, *const c_char, u64, u64, usize) -> c_int;
type FnPwhashStrAlg =
    unsafe extern "C" fn(*mut c_char, *const c_char, u64, u64, usize, c_int) -> c_int;
type FnStrVerify = unsafe extern "C" fn(*const c_char, *const c_char, u64) -> c_int;
type FnNeedsRehash = unsafe extern "C" fn(*const c_char, u64, usize) -> c_int;

type FnA2HashEnc = unsafe extern "C" fn(
    u32,
    u32,
    u32,
    *const c_void,
    usize,
    *const c_void,
    usize,
    usize,
    *mut c_char,
    usize,
) -> c_int;
type FnA2HashRaw = unsafe extern "C" fn(
    u32,
    u32,
    u32,
    *const c_void,
    usize,
    *const c_void,
    usize,
    *mut c_void,
    usize,
) -> c_int;
#[allow(clippy::type_complexity)]
type FnA2Hash = unsafe extern "C" fn(
    u32,
    u32,
    u32,
    *const c_void,
    usize,
    *const c_void,
    usize,
    *mut c_void,
    usize,
    *mut c_char,
    usize,
    c_int,
) -> c_int;
type FnA2Verify = unsafe extern "C" fn(*const c_char, *const c_void, usize) -> c_int;
type FnA2VerifyT = unsafe extern "C" fn(*const c_char, *const c_void, usize, c_int) -> c_int;
type FnA2Ctx = unsafe extern "C" fn(*mut Argon2Context, c_int) -> c_int;
type FnValidate = unsafe extern "C" fn(*const Argon2Context) -> c_int;
type FnEncodeString = unsafe extern "C" fn(*mut c_char, usize, *mut Argon2Context, c_int) -> c_int;
type FnDecodeString = unsafe extern "C" fn(*mut Argon2Context, *const c_char, c_int) -> c_int;
type FnInitialize = unsafe extern "C" fn(*mut Argon2Instance, *mut Argon2Context) -> c_int;
type FnFinalize = unsafe extern "C" fn(*const Argon2Context, *mut Argon2Instance);
type FnFillMem = unsafe extern "C" fn(*mut Argon2Instance, u32);
type FnFillSeg = unsafe extern "C" fn(*const Argon2Instance, Argon2Position);
type FnBlake2bLong = unsafe extern "C" fn(*mut c_void, usize, *const c_void, usize) -> c_int;

type FnScryptLL =
    unsafe extern "C" fn(*const u8, usize, *const u8, usize, u64, u32, u32, *mut u8, usize) -> c_int;
#[allow(clippy::type_complexity)]
type FnKdf = unsafe extern "C" fn(
    *mut EscryptRegion,
    *const u8,
    usize,
    *const u8,
    usize,
    u64,
    u32,
    u32,
    *mut u8,
    usize,
) -> c_int;
type FnPbkdf2 = unsafe extern "C" fn(*const u8, usize, *const u8, usize, u64, *mut u8, usize);
type FnParseSetting = unsafe extern "C" fn(*const u8, *mut u32, *mut u32, *mut u32) -> *const u8;
type FnEscryptR =
    unsafe extern "C" fn(*mut EscryptRegion, *const u8, usize, *const u8, *mut u8, usize) -> *mut u8;
type FnGensalt =
    unsafe extern "C" fn(u32, u32, u32, *const u8, usize, *mut u8, usize) -> *mut u8;
type FnAllocRegion = unsafe extern "C" fn(*mut EscryptRegion, usize) -> *mut c_void;
type FnRegionInt = unsafe extern "C" fn(*mut EscryptRegion) -> c_int;
type FnScrypt =
    unsafe extern "C" fn(*mut u8, u64, *const c_char, u64, *const u8, u64, usize) -> c_int;

type FnSetRbImpl = unsafe extern "C" fn(*const RbImpl) -> c_int;

// ===========================================================================
// Constants mirrored from the C headers (asserted against the getters below)
// ===========================================================================

const A2I_ALG: c_int = 1;
const A2ID_ALG: c_int = 2;

const A2_SALTBYTES: usize = 16;
const A2_STRBYTES: usize = 128;

const SC_SALTBYTES: usize = 32;
const SC_STRBYTES: usize = 102;
const SC_STRSETTINGBYTES: usize = 57;

// argon2 error codes
const ARGON2_OK: c_int = 0;
const ARGON2_OUTPUT_PTR_NULL: c_int = -1;
const ARGON2_OUTPUT_TOO_SHORT: c_int = -2;
const ARGON2_OUTPUT_TOO_LONG: c_int = -3;
const ARGON2_PWD_TOO_LONG: c_int = -5;
const ARGON2_SALT_TOO_SHORT: c_int = -6;
const ARGON2_SALT_TOO_LONG: c_int = -7;
const ARGON2_TIME_TOO_SMALL: c_int = -12;
const ARGON2_MEMORY_TOO_LITTLE: c_int = -14;
const ARGON2_LANES_TOO_FEW: c_int = -16;
const ARGON2_LANES_TOO_MANY: c_int = -17;
const ARGON2_PWD_PTR_MISMATCH: c_int = -18;
const ARGON2_SALT_PTR_MISMATCH: c_int = -19;
const ARGON2_SECRET_PTR_MISMATCH: c_int = -20;
const ARGON2_AD_PTR_MISMATCH: c_int = -21;
const ARGON2_INCORRECT_PARAMETER: c_int = -25;
const ARGON2_INCORRECT_TYPE: c_int = -26;
const ARGON2_THREADS_TOO_FEW: c_int = -28;
const ARGON2_THREADS_TOO_MANY: c_int = -29;
const ARGON2_ENCODING_FAIL: c_int = -31;
const ARGON2_DECODING_FAIL: c_int = -32;
const ARGON2_VERIFY_MISMATCH: c_int = -35;

const EINVAL: c_int = 22;
const EFBIG: c_int = 27;
const ENOMEM: c_int = 12;

// ===========================================================================
// deterministic randombytes implementation, installed into both .so's
// ===========================================================================

static CSEED: AtomicU64 = AtomicU64::new(0);
static RSEED: AtomicU64 = AtomicU64::new(0);

fn det_next(s: &AtomicU64) -> u64 {
    let base = s
        .fetch_add(0x9E37_79B9_7F4A_7C15, Ordering::SeqCst)
        .wrapping_add(0x9E37_79B9_7F4A_7C15);
    let mut z = base;
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

unsafe extern "C" fn det_name() -> *const c_char {
    b"harvest-det\0".as_ptr() as *const c_char
}
unsafe extern "C" fn c_random() -> u32 {
    det_next(&CSEED) as u32
}
unsafe extern "C" fn r_random() -> u32 {
    det_next(&RSEED) as u32
}
unsafe extern "C" fn c_buf(buf: *mut c_void, size: usize) {
    let p = buf as *mut u8;
    for i in 0..size {
        *p.add(i) = (det_next(&CSEED) >> 33) as u8;
    }
}
unsafe extern "C" fn r_buf(buf: *mut c_void, size: usize) {
    let p = buf as *mut u8;
    for i in 0..size {
        *p.add(i) = (det_next(&RSEED) >> 33) as u8;
    }
}

static C_IMPL: RbImpl = RbImpl {
    implementation_name: Some(det_name),
    random: Some(c_random),
    stir: None,
    uniform: None,
    buf: Some(c_buf),
    close: None,
};
static R_IMPL: RbImpl = RbImpl {
    implementation_name: Some(det_name),
    random: Some(r_random),
    stir: None,
    uniform: None,
    buf: Some(r_buf),
    close: None,
};

/// The deterministic RNG counters are process-global state shared by every
/// `#[test]`, so any test that resets them must hold this lock for its whole
/// duration (cargo runs tests in parallel threads by default).
static RNG_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

fn rng_guard() -> std::sync::MutexGuard<'static, ()> {
    match RNG_LOCK.lock() {
        Ok(g) => g,
        Err(p) => p.into_inner(),
    }
}

fn install_det_rng() {
    let (c, r) = both!("randombytes_set_implementation", FnSetRbImpl);
    unsafe {
        assert_eq!(c(&C_IMPL as *const RbImpl), 0);
        assert_eq!(r(&R_IMPL as *const RbImpl), 0);
    }
}

/// Reset both deterministic RNG counters to the same seed.
fn reset_rng(seed: u64) {
    CSEED.store(seed, Ordering::SeqCst);
    RSEED.store(seed, Ordering::SeqCst);
}

// ===========================================================================
// errno helpers
// ===========================================================================

extern "C" {
    fn __errno_location() -> *mut c_int;
}
fn errno_get() -> c_int {
    unsafe { *__errno_location() }
}
fn errno_set(v: c_int) {
    unsafe { *__errno_location() = v }
}

/// Run `f` (a C or Rust entry point) with errno pre-cleared to a sentinel and
/// return `(retval, errno)`.
fn with_errno<T>(f: impl FnOnce() -> T) -> (T, c_int) {
    errno_set(0);
    let rv = f();
    (rv, errno_get())
}

fn cstr(p: *const c_char) -> String {
    unsafe { CStr::from_ptr(p).to_string_lossy().into_owned() }
}

// ===========================================================================
// pwhash-1 .. : constants / getters
// ===========================================================================

#[test]
fn getters() {
    macro_rules! g_size {
        ($n:expr, $expect:expr) => {{
            let (c, r) = both!($n, FnSize);
            let (cv, rv) = unsafe { (c(), r()) };
            assert_eq!(cv, rv, "{} mismatch", $n);
            assert_eq!(cv, $expect as usize, "{} value", $n);
        }};
    }
    macro_rules! g_u64 {
        ($n:expr, $expect:expr) => {{
            let (c, r) = both!($n, FnU64);
            let (cv, rv) = unsafe { (c(), r()) };
            assert_eq!(cv, rv, "{} mismatch", $n);
            assert_eq!(cv, $expect as u64, "{} value", $n);
        }};
    }
    macro_rules! g_int {
        ($n:expr, $expect:expr) => {{
            let (c, r) = both!($n, FnInt);
            let (cv, rv) = unsafe { (c(), r()) };
            assert_eq!(cv, rv, "{} mismatch", $n);
            assert_eq!(cv, $expect as c_int, "{} value", $n);
        }};
    }
    macro_rules! g_str {
        ($n:expr, $expect:expr) => {{
            let (c, r) = both!($n, FnCStr);
            let (cv, rv) = unsafe { (cstr(c()), cstr(r())) };
            assert_eq!(cv, rv, "{} mismatch", $n);
            assert_eq!(cv, $expect, "{} value", $n);
        }};
    }

    let size_max_min_u32: usize = 4294967295;
    let memlimit_max: usize = 4398046510080;

    // --- crypto_pwhash.c
    g_int!("crypto_pwhash_alg_argon2i13", 1);
    g_int!("crypto_pwhash_alg_argon2id13", 2);
    g_int!("crypto_pwhash_alg_default", 2);
    g_size!("crypto_pwhash_bytes_min", 16);
    g_size!("crypto_pwhash_bytes_max", size_max_min_u32);
    g_size!("crypto_pwhash_passwd_min", 0);
    g_size!("crypto_pwhash_passwd_max", 4294967295usize);
    g_size!("crypto_pwhash_saltbytes", 16);
    g_size!("crypto_pwhash_strbytes", 128);
    g_str!("crypto_pwhash_strprefix", "$argon2id$");
    g_u64!("crypto_pwhash_opslimit_min", 1);
    g_u64!("crypto_pwhash_opslimit_max", 4294967295u64);
    g_size!("crypto_pwhash_memlimit_min", 8192);
    g_size!("crypto_pwhash_memlimit_max", memlimit_max);
    g_u64!("crypto_pwhash_opslimit_interactive", 2);
    g_size!("crypto_pwhash_memlimit_interactive", 67108864);
    g_u64!("crypto_pwhash_opslimit_moderate", 3);
    g_size!("crypto_pwhash_memlimit_moderate", 268435456);
    g_u64!("crypto_pwhash_opslimit_sensitive", 4);
    g_size!("crypto_pwhash_memlimit_sensitive", 1073741824u64);
    g_str!("crypto_pwhash_primitive", "argon2id,argon2i");

    // --- pwhash_argon2i.c
    g_int!("crypto_pwhash_argon2i_alg_argon2i13", 1);
    g_size!("crypto_pwhash_argon2i_bytes_min", 16);
    g_size!("crypto_pwhash_argon2i_bytes_max", size_max_min_u32);
    g_size!("crypto_pwhash_argon2i_passwd_min", 0);
    g_size!("crypto_pwhash_argon2i_passwd_max", 4294967295usize);
    g_size!("crypto_pwhash_argon2i_saltbytes", 16);
    g_size!("crypto_pwhash_argon2i_strbytes", 128);
    g_str!("crypto_pwhash_argon2i_strprefix", "$argon2i$");
    g_u64!("crypto_pwhash_argon2i_opslimit_min", 3);
    g_u64!("crypto_pwhash_argon2i_opslimit_max", 4294967295u64);
    g_size!("crypto_pwhash_argon2i_memlimit_min", 8192);
    g_size!("crypto_pwhash_argon2i_memlimit_max", memlimit_max);
    g_u64!("crypto_pwhash_argon2i_opslimit_interactive", 4);
    g_size!("crypto_pwhash_argon2i_memlimit_interactive", 33554432);
    g_u64!("crypto_pwhash_argon2i_opslimit_moderate", 6);
    g_size!("crypto_pwhash_argon2i_memlimit_moderate", 134217728);
    g_u64!("crypto_pwhash_argon2i_opslimit_sensitive", 8);
    g_size!("crypto_pwhash_argon2i_memlimit_sensitive", 536870912u64);

    // --- pwhash_argon2id.c
    g_int!("crypto_pwhash_argon2id_alg_argon2id13", 2);
    g_size!("crypto_pwhash_argon2id_bytes_min", 16);
    g_size!("crypto_pwhash_argon2id_bytes_max", size_max_min_u32);
    g_size!("crypto_pwhash_argon2id_passwd_min", 0);
    g_size!("crypto_pwhash_argon2id_passwd_max", 4294967295usize);
    g_size!("crypto_pwhash_argon2id_saltbytes", 16);
    g_size!("crypto_pwhash_argon2id_strbytes", 128);
    g_str!("crypto_pwhash_argon2id_strprefix", "$argon2id$");
    g_u64!("crypto_pwhash_argon2id_opslimit_min", 1);
    g_u64!("crypto_pwhash_argon2id_opslimit_max", 4294967295u64);
    g_size!("crypto_pwhash_argon2id_memlimit_min", 8192);
    g_size!("crypto_pwhash_argon2id_memlimit_max", memlimit_max);
    g_u64!("crypto_pwhash_argon2id_opslimit_interactive", 2);
    g_size!("crypto_pwhash_argon2id_memlimit_interactive", 67108864);
    g_u64!("crypto_pwhash_argon2id_opslimit_moderate", 3);
    g_size!("crypto_pwhash_argon2id_memlimit_moderate", 268435456);
    g_u64!("crypto_pwhash_argon2id_opslimit_sensitive", 4);
    g_size!("crypto_pwhash_argon2id_memlimit_sensitive", 1073741824u64);

    // --- pwhash_scryptsalsa208sha256.c
    g_size!("crypto_pwhash_scryptsalsa208sha256_bytes_min", 16);
    g_size!("crypto_pwhash_scryptsalsa208sha256_bytes_max", 0x1fffffffe0u64);
    g_size!("crypto_pwhash_scryptsalsa208sha256_passwd_min", 0);
    g_size!("crypto_pwhash_scryptsalsa208sha256_passwd_max", usize::MAX);
    g_size!("crypto_pwhash_scryptsalsa208sha256_saltbytes", 32);
    g_size!("crypto_pwhash_scryptsalsa208sha256_strbytes", 102);
    g_str!("crypto_pwhash_scryptsalsa208sha256_strprefix", "$7$");
    g_u64!("crypto_pwhash_scryptsalsa208sha256_opslimit_min", 32768);
    g_u64!("crypto_pwhash_scryptsalsa208sha256_opslimit_max", 4294967295u64);
    g_size!("crypto_pwhash_scryptsalsa208sha256_memlimit_min", 16777216);
    g_size!("crypto_pwhash_scryptsalsa208sha256_memlimit_max", 68719476736u64);
    g_u64!("crypto_pwhash_scryptsalsa208sha256_opslimit_interactive", 524288);
    g_size!("crypto_pwhash_scryptsalsa208sha256_memlimit_interactive", 16777216);
    g_u64!("crypto_pwhash_scryptsalsa208sha256_opslimit_sensitive", 33554432);
    g_size!("crypto_pwhash_scryptsalsa208sha256_memlimit_sensitive", 1073741824u64);

    // --- argon2-core.c: pick_best_implementation
    g_int!("_crypto_pwhash_argon2_pick_best_implementation", 0);
}

// ===========================================================================
// crypto_pwhash dispatcher + argon2i/argon2id one-shot
// ===========================================================================

/// Compare `(rc, errno, full out buffer)` for a one-shot pwhash call.
#[allow(clippy::too_many_arguments)]
fn cmp_pwhash(
    ctx: &str,
    c: FnPwhash,
    r: FnPwhash,
    outlen: u64,
    buflen: usize,
    passwd: &[u8],
    passwdlen: u64,
    salt: &[u8; A2_SALTBYTES],
    opslimit: u64,
    memlimit: usize,
    alg: c_int,
) -> c_int {
    // distinct canary so over/under-writes are caught
    let mut co = vec![0xA5u8; buflen];
    let mut ro = vec![0xA5u8; buflen];
    let (rc, ce) = with_errno(|| unsafe {
        c(
            co.as_mut_ptr(),
            outlen,
            passwd.as_ptr() as *const c_char,
            passwdlen,
            salt.as_ptr(),
            opslimit,
            memlimit,
            alg,
        )
    });
    let (rr, re) = with_errno(|| unsafe {
        r(
            ro.as_mut_ptr(),
            outlen,
            passwd.as_ptr() as *const c_char,
            passwdlen,
            salt.as_ptr(),
            opslimit,
            memlimit,
            alg,
        )
    });
    assert_eq!(rc, rr, "{ctx}: return code");
    assert_eq!(ce, re, "{ctx}: errno (C={ce} Rust={re})");
    common::eqb(ctx, &co, &ro);
    rc
}

#[test]
fn pwhash_dispatcher_and_alg_enum() {
    let (c, r) = both!("crypto_pwhash", FnPwhash);
    let mut rng = common::Rng::new(0x9111_1111);
    let salt: [u8; A2_SALTBYTES] = {
        let v = rng.bytes(A2_SALTBYTES);
        let mut a = [0u8; A2_SALTBYTES];
        a.copy_from_slice(&v);
        a
    };
    let pw = b"correct horse battery staple".to_vec();

    // valid algs
    assert_eq!(
        0,
        cmp_pwhash("pwhash alg=1", c, r, 16, 24, &pw, pw.len() as u64, &salt, 3, 8192, A2I_ALG)
    );
    assert_eq!(
        0,
        cmp_pwhash("pwhash alg=2", c, r, 16, 24, &pw, pw.len() as u64, &salt, 1, 8192, A2ID_ALG)
    );

    // out-of-range enum values: C's `switch` default -> errno=EINVAL, -1
    for alg in [0, 3, -1, 999, i32::MIN, i32::MAX] {
        let rc = cmp_pwhash(
            &format!("pwhash alg={alg}"),
            c,
            r,
            32,
            40,
            &pw,
            pw.len() as u64,
            &salt,
            3,
            8192,
            alg,
        );
        assert_eq!(rc, -1, "alg={alg} must be rejected");
        assert_eq!(errno_get(), EINVAL, "alg={alg} errno");
    }
}

#[test]
fn argon2_oneshot_shapes_and_bounds() {
    let mut rng = common::Rng::new(0x0BAD_F00D);
    let salt: [u8; A2_SALTBYTES] = {
        let mut a = [0u8; A2_SALTBYTES];
        rng.fill(&mut a);
        a
    };

    for (name, alg, ops_min) in [
        ("crypto_pwhash_argon2i", A2I_ALG, 3u64),
        ("crypto_pwhash_argon2id", A2ID_ALG, 1u64),
    ] {
        let (c, r) = if alg == A2I_ALG {
            both!("crypto_pwhash_argon2i", FnPwhash)
        } else {
            both!("crypto_pwhash_argon2id", FnPwhash)
        };

        // --- passwd lengths 0 / 1 / long, several random cases each
        for &pwlen in &[0usize, 1, 63, 64, 65, 200] {
            for case in 0..3 {
                let pw = rng.bytes(pwlen.max(1)); // always have a valid pointer
                let mut s = salt;
                rng.fill(&mut s);
                assert_eq!(
                    0,
                    cmp_pwhash(
                        &format!("{name} pwlen={pwlen} case={case}"),
                        c,
                        r,
                        32,
                        48,
                        &pw,
                        pwlen as u64,
                        &s,
                        ops_min,
                        8192,
                        alg
                    )
                );
            }
        }

        // --- outlen boundaries: MIN-1 (15) rejected, MIN (16) ok, larger ok
        let pw = b"pw".to_vec();
        for &(outlen, expect) in &[(15u64, -1), (16, 0), (17, 0), (31, 0), (32, 0), (64, 0), (200, 0)]
        {
            let rc = cmp_pwhash(
                &format!("{name} outlen={outlen}"),
                c,
                r,
                outlen,
                outlen as usize + 8,
                &pw,
                pw.len() as u64,
                &salt,
                ops_min,
                8192,
                alg,
            );
            assert_eq!(rc, expect, "{name} outlen={outlen}");
            if expect == -1 {
                assert_eq!(errno_get(), EINVAL);
            }
        }

        // --- outlen > BYTES_MAX (4294967295) -> EFBIG.  memset(out,0,outlen)
        //     happens FIRST in the C, so we must not actually pass a huge outlen
        //     with a small buffer. Instead use a 1-byte allocation and outlen
        //     just above the max only via the *_str-free path: not possible.
        //     -> covered by inspection; see _v/errors/pwhash.md (pwhash-E3).

        // --- opslimit below MIN
        for ops in [0u64, ops_min - 1] {
            let rc = cmp_pwhash(
                &format!("{name} ops={ops}"),
                c,
                r,
                32,
                40,
                &pw,
                pw.len() as u64,
                &salt,
                ops,
                8192,
                alg,
            );
            assert_eq!(rc, -1);
            assert_eq!(errno_get(), EINVAL);
        }
        // --- opslimit above MAX
        for ops in [4294967296u64, u64::MAX] {
            let rc = cmp_pwhash(
                &format!("{name} ops={ops}"),
                c,
                r,
                32,
                40,
                &pw,
                pw.len() as u64,
                &salt,
                ops,
                8192,
                alg,
            );
            assert_eq!(rc, -1);
            assert_eq!(errno_get(), EFBIG);
        }
        // --- memlimit below MIN / above MAX
        for ml in [0usize, 1, 8191] {
            let rc = cmp_pwhash(
                &format!("{name} mem={ml}"),
                c,
                r,
                32,
                40,
                &pw,
                pw.len() as u64,
                &salt,
                ops_min,
                ml,
                alg,
            );
            assert_eq!(rc, -1);
            assert_eq!(errno_get(), EINVAL);
        }
        for ml in [4398046510081usize, usize::MAX] {
            let rc = cmp_pwhash(
                &format!("{name} mem={ml}"),
                c,
                r,
                32,
                40,
                &pw,
                pw.len() as u64,
                &salt,
                ops_min,
                ml,
                alg,
            );
            assert_eq!(rc, -1);
            assert_eq!(errno_get(), EFBIG);
        }
        // --- passwdlen above PASSWD_MAX
        let rc = cmp_pwhash(
            &format!("{name} pwlen=2^32"),
            c,
            r,
            32,
            40,
            &pw,
            4294967296,
            &salt,
            ops_min,
            8192,
            alg,
        );
        assert_eq!(rc, -1);
        assert_eq!(errno_get(), EFBIG);

        // --- alg mismatch: argon2i must reject alg=2 and vice versa
        let wrong = if alg == A2I_ALG { A2ID_ALG } else { A2I_ALG };
        for bad in [0, wrong, 3, -1, 999] {
            let rc = cmp_pwhash(
                &format!("{name} alg={bad}"),
                c,
                r,
                32,
                40,
                &pw,
                pw.len() as u64,
                &salt,
                ops_min,
                8192,
                bad,
            );
            assert_eq!(rc, -1, "{name} alg={bad}");
            assert_eq!(errno_get(), EINVAL);
        }

        // --- memlimit that is not a multiple of 1024 (integer-divided m_cost)
        for ml in [8192usize, 8193, 9215, 16384, 65536] {
            assert_eq!(
                0,
                cmp_pwhash(
                    &format!("{name} mem={ml}"),
                    c,
                    r,
                    32,
                    40,
                    &pw,
                    pw.len() as u64,
                    &salt,
                    ops_min,
                    ml,
                    alg
                )
            );
        }

        // --- higher opslimit
        for ops in [ops_min, ops_min + 1, ops_min + 2] {
            assert_eq!(
                0,
                cmp_pwhash(
                    &format!("{name} ops={ops}"),
                    c,
                    r,
                    32,
                    40,
                    &pw,
                    pw.len() as u64,
                    &salt,
                    ops,
                    8192,
                    alg
                )
            );
        }
    }
}

#[test]
fn argon2_oneshot_out_eq_passwd() {
    // (const void *) out == (const void *) passwd -> EINVAL
    let fams: [(&str, (FnPwhash, FnPwhash)); 2] = [
        ("crypto_pwhash_argon2i", both!("crypto_pwhash_argon2i", FnPwhash)),
        ("crypto_pwhash_argon2id", both!("crypto_pwhash_argon2id", FnPwhash)),
    ];
    for (name, (c, r)) in fams {
        let ops = if name.ends_with("2i") { 3u64 } else { 1u64 };
        let salt = [7u8; A2_SALTBYTES];
        let mut cbuf = vec![0u8; 64];
        let mut rbuf = vec![0u8; 64];
        let (rc, ce) = with_errno(|| unsafe {
            c(
                cbuf.as_mut_ptr(),
                32,
                cbuf.as_ptr() as *const c_char,
                8,
                salt.as_ptr(),
                ops,
                8192,
                if name.ends_with("2i") { A2I_ALG } else { A2ID_ALG },
            )
        });
        let (rr, re) = with_errno(|| unsafe {
            r(
                rbuf.as_mut_ptr(),
                32,
                rbuf.as_ptr() as *const c_char,
                8,
                salt.as_ptr(),
                ops,
                8192,
                if name.ends_with("2i") { A2I_ALG } else { A2ID_ALG },
            )
        });
        assert_eq!((rc, ce), (rr, re), "{name} out==passwd");
        assert_eq!(rc, -1);
        assert_eq!(ce, EINVAL);
        common::eqb(&format!("{name} out==passwd buf"), &cbuf, &rbuf);
    }
}

// ===========================================================================
// *_str / *_str_verify / *_str_needs_rehash / *_str_alg
// ===========================================================================

/// Produce a hash string with both libraries under the deterministic RNG and
/// assert they are byte-identical; returns the (C) string bytes.
fn cmp_str(
    ctx: &str,
    c: FnPwhashStr,
    r: FnPwhashStr,
    strbytes: usize,
    passwd: &[u8],
    passwdlen: u64,
    opslimit: u64,
    memlimit: usize,
    seed: u64,
) -> (c_int, Vec<u8>) {
    let mut cb = vec![0x5Au8; strbytes];
    let mut rb = vec![0x5Au8; strbytes];
    reset_rng(seed);
    let (rc, ce) = with_errno(|| unsafe {
        c(
            cb.as_mut_ptr() as *mut c_char,
            passwd.as_ptr() as *const c_char,
            passwdlen,
            opslimit,
            memlimit,
        )
    });
    let cseed_after = CSEED.load(Ordering::SeqCst);
    RSEED.store(seed, Ordering::SeqCst);
    let (rr, re) = with_errno(|| unsafe {
        r(
            rb.as_mut_ptr() as *mut c_char,
            passwd.as_ptr() as *const c_char,
            passwdlen,
            opslimit,
            memlimit,
        )
    });
    assert_eq!(rc, rr, "{ctx}: return code");
    assert_eq!(ce, re, "{ctx}: errno");
    assert_eq!(
        cseed_after,
        RSEED.load(Ordering::SeqCst),
        "{ctx}: randombytes consumption differs"
    );
    common::eqb(ctx, &cb, &rb);
    (rc, cb)
}

#[test]
fn argon2_str_roundtrip() {
    let _rng_lock = rng_guard();
    install_det_rng();
    let mut rng = common::Rng::new(0x5EED_1234);

    for (fam, ops_min, prefix) in [
        ("crypto_pwhash_argon2i", 3u64, "$argon2i$"),
        ("crypto_pwhash_argon2id", 1u64, "$argon2id$"),
    ] {
        let (cs, rs) = if fam.ends_with("2i") {
            both!("crypto_pwhash_argon2i_str", FnPwhashStr)
        } else {
            both!("crypto_pwhash_argon2id_str", FnPwhashStr)
        };
        let (cv, rv) = if fam.ends_with("2i") {
            both!("crypto_pwhash_argon2i_str_verify", FnStrVerify)
        } else {
            both!("crypto_pwhash_argon2id_str_verify", FnStrVerify)
        };
        let (cn, rn) = if fam.ends_with("2i") {
            both!("crypto_pwhash_argon2i_str_needs_rehash", FnNeedsRehash)
        } else {
            both!("crypto_pwhash_argon2id_str_needs_rehash", FnNeedsRehash)
        };

        for &pwlen in &[0usize, 1, 32, 200] {
            for case in 0..3u64 {
                let pw = rng.bytes(pwlen.max(1));
                let seed = 0xC0DE_0000 + case * 977 + pwlen as u64;
                let (rc, s) = cmp_str(
                    &format!("{fam}_str pwlen={pwlen} case={case}"),
                    cs,
                    rs,
                    A2_STRBYTES,
                    &pw,
                    pwlen as u64,
                    ops_min,
                    8192,
                    seed,
                );
                assert_eq!(rc, 0);
                let ss = String::from_utf8_lossy(&s[..s.iter().position(|&b| b == 0).unwrap()])
                    .into_owned();
                assert!(ss.starts_with(prefix), "{fam}: {ss}");

                // both libs verify the (identical) string
                let ok_c = unsafe { cv(s.as_ptr() as *const c_char, pw.as_ptr() as *const c_char, pwlen as u64) };
                let ok_r = unsafe { rv(s.as_ptr() as *const c_char, pw.as_ptr() as *const c_char, pwlen as u64) };
                assert_eq!((ok_c, ok_r), (0, 0), "{fam} verify {ss}");

                // wrong password -> -1 / EINVAL on both
                let bad = rng.bytes(pwlen.max(1) + 1);
                let (bc, bce) = with_errno(|| unsafe {
                    cv(s.as_ptr() as *const c_char, bad.as_ptr() as *const c_char, bad.len() as u64)
                });
                let (br, bre) = with_errno(|| unsafe {
                    rv(s.as_ptr() as *const c_char, bad.as_ptr() as *const c_char, bad.len() as u64)
                });
                assert_eq!((bc, bce), (br, bre), "{fam} wrong-pw verify");
                assert_eq!(bc, -1);
                assert_eq!(bce, EINVAL);

                // needs_rehash: identical params -> 0, different -> 1
                for (ops, mem, expect) in [
                    (ops_min, 8192usize, 0),
                    (ops_min + 1, 8192, 1),
                    (ops_min, 16384, 1),
                ] {
                    let (a, ae) = with_errno(|| unsafe { cn(s.as_ptr() as *const c_char, ops, mem) });
                    let (b, be) = with_errno(|| unsafe { rn(s.as_ptr() as *const c_char, ops, mem) });
                    assert_eq!((a, ae), (b, be), "{fam} needs_rehash ops={ops} mem={mem}");
                    assert_eq!(a, expect, "{fam} needs_rehash ops={ops} mem={mem}");
                }
            }
        }

        // --- error paths of *_str
        let pw = b"abc".to_vec();
        for (ops, mem, want_errno) in [
            (0u64, 8192usize, EINVAL),
            (ops_min - 1, 8192, EINVAL),
            (ops_min, 0, EINVAL),
            (ops_min, 8191, EINVAL),
            (4294967296u64, 8192, EFBIG),
            (ops_min, 4398046510081usize, EFBIG),
            (ops_min, usize::MAX, EFBIG),
        ] {
            let (rc, _) = cmp_str(
                &format!("{fam}_str bad ops={ops} mem={mem}"),
                cs,
                rs,
                A2_STRBYTES,
                &pw,
                pw.len() as u64,
                ops,
                mem,
                7,
            );
            assert_eq!(rc, -1);
            assert_eq!(errno_get(), want_errno, "{fam}_str ops={ops} mem={mem}");
        }
        // passwdlen > PASSWD_MAX
        let (rc, _) = cmp_str(
            &format!("{fam}_str pwlen=2^32"),
            cs,
            rs,
            A2_STRBYTES,
            &pw,
            4294967296,
            ops_min,
            8192,
            7,
        );
        assert_eq!(rc, -1);
        assert_eq!(errno_get(), EFBIG);
    }
}

#[test]
fn argon2_str_verify_malformed() {
    let _rng_lock = rng_guard();
    install_det_rng();

    // build one valid argon2id and one valid argon2i string
    let (cid, rid) = both!("crypto_pwhash_argon2id_str", FnPwhashStr);
    let (ci, ri) = both!("crypto_pwhash_argon2i_str", FnPwhashStr);
    let pw = b"hunter2".to_vec();
    let (_, s_id) = cmp_str("mk id", cid, rid, A2_STRBYTES, &pw, pw.len() as u64, 1, 8192, 42);
    let (_, s_i) = cmp_str("mk i", ci, ri, A2_STRBYTES, &pw, pw.len() as u64, 3, 8192, 43);

    let id_len = s_id.iter().position(|&b| b == 0).unwrap();
    let i_len = s_i.iter().position(|&b| b == 0).unwrap();

    // all three verifiers
    let verifiers: [(&str, (FnStrVerify, FnStrVerify)); 3] = [
        ("crypto_pwhash_str_verify", both!("crypto_pwhash_str_verify", FnStrVerify)),
        (
            "crypto_pwhash_argon2i_str_verify",
            both!("crypto_pwhash_argon2i_str_verify", FnStrVerify),
        ),
        (
            "crypto_pwhash_argon2id_str_verify",
            both!("crypto_pwhash_argon2id_str_verify", FnStrVerify),
        ),
    ];

    let mut cases: Vec<(String, Vec<u8>)> = Vec::new();
    cases.push(("valid argon2id".into(), s_id[..id_len + 1].to_vec()));
    cases.push(("valid argon2i".into(), s_i[..i_len + 1].to_vec()));
    // corrupted byte inside the base64 hash (last char)
    {
        let mut v = s_id[..id_len + 1].to_vec();
        v[id_len - 1] = if v[id_len - 1] == b'A' { b'B' } else { b'A' };
        cases.push(("corrupt last b64 char".into(), v));
    }
    // corrupted byte inside the salt
    {
        let mut v = s_id[..id_len + 1].to_vec();
        let dollar = v.iter().position(|&b| b == b'$').unwrap();
        let p = v[dollar + 1..].iter().position(|&b| b == b'$').unwrap() + dollar + 1;
        let p3 = v[p + 1..].iter().position(|&b| b == b'$').unwrap() + p + 1;
        v[p3 + 2] = if v[p3 + 2] == b'C' { b'D' } else { b'C' };
        cases.push(("corrupt salt byte".into(), v));
    }
    // corrupt a parameter digit
    {
        let mut v = s_id[..id_len + 1].to_vec();
        let pos = v.windows(3).position(|w| w == b"$m=").unwrap();
        v[pos + 3] = b'9';
        cases.push(("corrupt m= digit".into(), v));
    }
    cases.push(("empty string".into(), b"\0".to_vec()));
    cases.push(("no dollar".into(), b"argon2id$v=19$m=8,t=1,p=1$AAAA$AAAA\0".to_vec()));
    cases.push(("only prefix".into(), b"$argon2id\0".to_vec()));
    cases.push(("truncated".into(), s_id[..id_len / 2].iter().copied().chain([0u8]).collect()));
    // truncated exactly after the salt
    {
        let v = s_id[..id_len + 1].to_vec();
        let last = v.iter().rposition(|&b| b == b'$').unwrap();
        let mut t = v[..last].to_vec();
        t.push(0);
        cases.push(("missing hash field".into(), t));
    }
    // over-long base64 (extra chars appended after the hash)
    {
        let mut v = s_id[..id_len].to_vec();
        v.extend_from_slice(b"AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA");
        v.push(0);
        cases.push(("over-long b64".into(), v));
    }
    // over-short base64 in the out field
    {
        let v = s_id[..id_len + 1].to_vec();
        let last = v.iter().rposition(|&b| b == b'$').unwrap();
        let mut t = v[..last + 1].to_vec();
        t.extend_from_slice(b"AAAA");
        t.push(0);
        cases.push(("over-short out b64".into(), t));
    }
    // trailing garbage after a well-formed hash (the C rejects it in
    // argon2_decode_string via `*str != 0`)
    {
        let mut v = s_id[..id_len].to_vec();
        v.extend_from_slice(b"!!!");
        v.push(0);
        cases.push(("trailing garbage".into(), v));
    }
    // wrong version
    cases.push((
        "version 16".into(),
        b"$argon2id$v=16$m=8,t=1,p=1$AAAAAAAAAAAAAAAAAAAAAA$AAAAAAAAAAAAAAAAAAAAAA\0".to_vec(),
    ));
    // non-minimal decimal (leading zero)
    cases.push((
        "leading zero m".into(),
        b"$argon2id$v=19$m=08,t=1,p=1$AAAAAAAAAAAAAAAAAAAAAA$AAAAAAAAAAAAAAAAAAAAAA\0".to_vec(),
    ));
    // decimal out of u32 range
    cases.push((
        "m=2^32".into(),
        b"$argon2id$v=19$m=4294967296,t=1,p=1$AAAAAAAAAAAAAAAAAAAAAA$AAAAAAAAAAAAAAAAAAAAAA\0"
            .to_vec(),
    ));
    // p=0 -> LANES_TOO_FEW inside decode
    cases.push((
        "p=0".into(),
        b"$argon2id$v=19$m=8,t=1,p=0$AAAAAAAAAAAAAAAAAAAAAA$AAAAAAAAAAAAAAAAAAAAAA\0".to_vec(),
    ));
    // completely unrelated prefix (exercises crypto_pwhash_str_verify's
    // strncmp fallthrough -> EINVAL)
    cases.push(("scrypt prefix".into(), b"$7$C6..../....abcdefgh$xyz\0".to_vec()));

    for (label, buf) in &cases {
        for (vname, (c, r)) in verifiers.iter() {
            let (a, ae) = with_errno(|| unsafe {
                c(buf.as_ptr() as *const c_char, pw.as_ptr() as *const c_char, pw.len() as u64)
            });
            let (b, be) = with_errno(|| unsafe {
                r(buf.as_ptr() as *const c_char, pw.as_ptr() as *const c_char, pw.len() as u64)
            });
            assert_eq!((a, ae), (b, be), "{vname}({label})");
        }
    }

    // needs_rehash on the same malformed set
    let nrs: [(&str, (FnNeedsRehash, FnNeedsRehash)); 3] = [
        ("crypto_pwhash_str_needs_rehash", both!("crypto_pwhash_str_needs_rehash", FnNeedsRehash)),
        (
            "crypto_pwhash_argon2i_str_needs_rehash",
            both!("crypto_pwhash_argon2i_str_needs_rehash", FnNeedsRehash),
        ),
        (
            "crypto_pwhash_argon2id_str_needs_rehash",
            both!("crypto_pwhash_argon2id_str_needs_rehash", FnNeedsRehash),
        ),
    ];
    for (label, buf) in &cases {
        for (nname, (c, r)) in nrs.iter() {
            for (ops, mem) in [(1u64, 8192usize), (3, 8192), (4294967296, 8192), (1, usize::MAX)] {
                let (a, ae) = with_errno(|| unsafe { c(buf.as_ptr() as *const c_char, ops, mem) });
                let (b, be) = with_errno(|| unsafe { r(buf.as_ptr() as *const c_char, ops, mem) });
                assert_eq!((a, ae), (b, be), "{nname}({label}, ops={ops}, mem={mem})");
            }
        }
    }

    // needs_rehash with a >= crypto_pwhash_STRBYTES long string -> EINVAL
    let long = {
        let mut v = vec![b'$'; A2_STRBYTES];
        v.push(0);
        v
    };
    for (nname, (c, r)) in nrs.iter() {
        let (a, ae) = with_errno(|| unsafe { c(long.as_ptr() as *const c_char, 1, 8192) });
        let (b, be) = with_errno(|| unsafe { r(long.as_ptr() as *const c_char, 1, 8192) });
        assert_eq!((a, ae), (b, be), "{nname}(128-char string)");
        // crypto_pwhash_str_needs_rehash rejects on the prefix, the family ones
        // on the length; both give -1.
        assert_eq!(a, -1);
        assert_eq!(ae, EINVAL);
    }

    // *_str_verify with passwdlen > PASSWD_MAX -> EFBIG
    for (vname, (c, r)) in verifiers.iter() {
        let (a, ae) = with_errno(|| unsafe {
            c(s_id.as_ptr() as *const c_char, pw.as_ptr() as *const c_char, 4294967296)
        });
        let (b, be) = with_errno(|| unsafe {
            r(s_id.as_ptr() as *const c_char, pw.as_ptr() as *const c_char, 4294967296)
        });
        assert_eq!((a, ae), (b, be), "{vname}(pwlen=2^32)");
        assert_eq!(a, -1);
        assert_eq!(ae, EFBIG);
    }
}

#[test]
fn crypto_pwhash_str_and_str_alg() {
    let _rng_lock = rng_guard();
    install_det_rng();
    let (cs, rs) = both!("crypto_pwhash_str", FnPwhashStr);
    let (ca, ra) = both!("crypto_pwhash_str_alg", FnPwhashStrAlg);
    let (cv, rv) = both!("crypto_pwhash_str_verify", FnStrVerify);
    let pw = b"a passphrase".to_vec();

    // crypto_pwhash_str == crypto_pwhash_argon2id_str
    let (rc, s) = cmp_str("crypto_pwhash_str", cs, rs, A2_STRBYTES, &pw, pw.len() as u64, 1, 8192, 11);
    assert_eq!(rc, 0);
    assert!(String::from_utf8_lossy(&s).starts_with("$argon2id$"));
    let (a, b) = unsafe {
        (
            cv(s.as_ptr() as *const c_char, pw.as_ptr() as *const c_char, pw.len() as u64),
            rv(s.as_ptr() as *const c_char, pw.as_ptr() as *const c_char, pw.len() as u64),
        )
    };
    assert_eq!((a, b), (0, 0));

    // crypto_pwhash_str_alg with the two valid algs
    for (alg, ops, prefix) in [(A2I_ALG, 3u64, "$argon2i$"), (A2ID_ALG, 1u64, "$argon2id$")] {
        let mut cb = vec![0x5Au8; A2_STRBYTES];
        let mut rb = vec![0x5Au8; A2_STRBYTES];
        reset_rng(0xABCD);
        let (x, xe) = with_errno(|| unsafe {
            ca(
                cb.as_mut_ptr() as *mut c_char,
                pw.as_ptr() as *const c_char,
                pw.len() as u64,
                ops,
                8192,
                alg,
            )
        });
        RSEED.store(0xABCD, Ordering::SeqCst);
        let (y, ye) = with_errno(|| unsafe {
            ra(
                rb.as_mut_ptr() as *mut c_char,
                pw.as_ptr() as *const c_char,
                pw.len() as u64,
                ops,
                8192,
                alg,
            )
        });
        assert_eq!((x, xe), (y, ye), "str_alg alg={alg}");
        assert_eq!(x, 0);
        common::eqb(&format!("str_alg alg={alg}"), &cb, &rb);
        assert!(String::from_utf8_lossy(&cb).starts_with(prefix));
        // and cross-verify with the generic verifier
        let (a, b) = unsafe {
            (
                cv(cb.as_ptr() as *const c_char, pw.as_ptr() as *const c_char, pw.len() as u64),
                rv(cb.as_ptr() as *const c_char, pw.as_ptr() as *const c_char, pw.len() as u64),
            )
        };
        assert_eq!((a, b), (0, 0));
    }

    // error paths of str_alg (valid algs, out-of-range limits)
    for (alg, ops, mem, want) in
        [(A2I_ALG, 0u64, 8192usize, EINVAL), (A2ID_ALG, 4294967296u64, 8192usize, EFBIG)]
    {
        let mut cb = vec![0x5Au8; A2_STRBYTES];
        let mut rb = vec![0x5Au8; A2_STRBYTES];
        let (x, xe) = with_errno(|| unsafe {
            ca(cb.as_mut_ptr() as *mut c_char, pw.as_ptr() as *const c_char, 3, ops, mem, alg)
        });
        let (y, ye) = with_errno(|| unsafe {
            ra(rb.as_mut_ptr() as *mut c_char, pw.as_ptr() as *const c_char, 3, ops, mem, alg)
        });
        assert_eq!((x, xe), (y, ye));
        assert_eq!(x, -1);
        assert_eq!(xe, want);
        common::eqb("str_alg err", &cb, &rb);
    }
    // NOTE: crypto_pwhash_str_alg() with an out-of-range alg calls
    // sodium_misuse() -> abort(); not testable in-process (see errors table).
}

// ===========================================================================
// argon2 low level: argon2_hash / argon2_ctx / argon2_verify / validate_inputs
// ===========================================================================

#[test]
fn argon2_hash_lowlevel() {
    let _rng_lock = rng_guard();
    let (ch, rh) = both!("_sodium_argon2_hash", FnA2Hash);
    let (cie, rie) = both!("_sodium_argon2i_hash_encoded", FnA2HashEnc);
    let (cir, rir) = both!("_sodium_argon2i_hash_raw", FnA2HashRaw);
    let (cde, rde) = both!("_sodium_argon2id_hash_encoded", FnA2HashEnc);
    let (cdr, rdr) = both!("_sodium_argon2id_hash_raw", FnA2HashRaw);
    install_det_rng();

    let mut rng = common::Rng::new(0xA2A2_A2A2);

    // --- parameter combinations the C accepts (lanes == threads == parallelism)
    // m_cost >= max(8, 8*lanes) and >= 2*4*lanes
    let combos: &[(u32, u32, u32)] = &[
        (1, 8, 1),
        (2, 8, 1),
        (3, 9, 1),
        (1, 16, 2),
        (2, 16, 2),
        (1, 24, 3),
        (1, 32, 4),
        (2, 33, 4),
        (1, 64, 1),
    ];
    for &(t_cost, m_cost, par) in combos {
        for hashlen in [16usize, 17, 31, 32, 64, 65, 100] {
            let n_pw = 1 + rng.below(40);
            let pw = rng.bytes(n_pw);
            let n_salt = 8 + rng.below(24);
            let salt = rng.bytes(n_salt);
            for (name, cf, rf) in
                [("argon2i_hash_raw", cir, rir), ("argon2id_hash_raw", cdr, rdr)]
            {
                let mut co = vec![0x3Cu8; hashlen + 8];
                let mut ro = vec![0x3Cu8; hashlen + 8];
                reset_rng(0x1000);
                let a = unsafe {
                    cf(
                        t_cost,
                        m_cost,
                        par,
                        pw.as_ptr() as *const c_void,
                        pw.len(),
                        salt.as_ptr() as *const c_void,
                        salt.len(),
                        co.as_mut_ptr() as *mut c_void,
                        hashlen,
                    )
                };
                RSEED.store(0x1000, Ordering::SeqCst);
                let b = unsafe {
                    rf(
                        t_cost,
                        m_cost,
                        par,
                        pw.as_ptr() as *const c_void,
                        pw.len(),
                        salt.as_ptr() as *const c_void,
                        salt.len(),
                        ro.as_mut_ptr() as *mut c_void,
                        hashlen,
                    )
                };
                let ctx = format!("{name} t={t_cost} m={m_cost} p={par} hl={hashlen}");
                assert_eq!(a, b, "{ctx}: rc");
                assert_eq!(a, ARGON2_OK, "{ctx}");
                common::eqb(&ctx, &co, &ro);
            }
            for (name, cf, rf) in
                [("argon2i_hash_encoded", cie, rie), ("argon2id_hash_encoded", cde, rde)]
            {
                let mut cb = vec![0x3Cu8; 256];
                let mut rb = vec![0x3Cu8; 256];
                let a = unsafe {
                    cf(
                        t_cost,
                        m_cost,
                        par,
                        pw.as_ptr() as *const c_void,
                        pw.len(),
                        salt.as_ptr() as *const c_void,
                        salt.len(),
                        hashlen,
                        cb.as_mut_ptr() as *mut c_char,
                        256,
                    )
                };
                let b = unsafe {
                    rf(
                        t_cost,
                        m_cost,
                        par,
                        pw.as_ptr() as *const c_void,
                        pw.len(),
                        salt.as_ptr() as *const c_void,
                        salt.len(),
                        hashlen,
                        rb.as_mut_ptr() as *mut c_char,
                        256,
                    )
                };
                let ctx = format!("{name} t={t_cost} m={m_cost} p={par} hl={hashlen}");
                assert_eq!(a, b, "{ctx}: rc");
                assert_eq!(a, ARGON2_OK, "{ctx}");
                common::eqb(&ctx, &cb, &rb);
            }
        }
    }

    // --- parameter combinations the C rejects, with the EXACT error code
    let pw = b"pw".to_vec();
    let salt = b"saltsalt".to_vec();
    let rejects: &[(u32, u32, u32, c_int)] = &[
        (0, 8, 1, ARGON2_TIME_TOO_SMALL),        // t_cost == 0
        (1, 7, 1, ARGON2_MEMORY_TOO_LITTLE),     // m_cost < ARGON2_MIN_MEMORY
        (1, 0, 1, ARGON2_MEMORY_TOO_LITTLE),     // m_cost == 0
        (1, 8, 0, ARGON2_LANES_TOO_FEW),         // lanes == 0
        (1, 8, 2, ARGON2_MEMORY_TOO_LITTLE),     // m_cost < 8 * lanes
        (1, 16, 3, ARGON2_MEMORY_TOO_LITTLE),    // m_cost < 8 * lanes
        (1, 0xFFFFFFFF, 0x1000000, ARGON2_LANES_TOO_MANY), // lanes > ARGON2_MAX_LANES
    ];
    for &(t_cost, m_cost, par, want) in rejects {
        for (name, cf, rf) in [("i", cir, rir), ("id", cdr, rdr)] {
            let mut co = vec![0x11u8; 40];
            let mut ro = vec![0x11u8; 40];
            reset_rng(0x2000);
            let a = unsafe {
                cf(
                    t_cost,
                    m_cost,
                    par,
                    pw.as_ptr() as *const c_void,
                    pw.len(),
                    salt.as_ptr() as *const c_void,
                    salt.len(),
                    co.as_mut_ptr() as *mut c_void,
                    32,
                )
            };
            RSEED.store(0x2000, Ordering::SeqCst);
            let b = unsafe {
                rf(
                    t_cost,
                    m_cost,
                    par,
                    pw.as_ptr() as *const c_void,
                    pw.len(),
                    salt.as_ptr() as *const c_void,
                    salt.len(),
                    ro.as_mut_ptr() as *mut c_void,
                    32,
                )
            };
            let ctx = format!("argon2{name}_hash_raw t={t_cost} m={m_cost} p={par}");
            assert_eq!(a, b, "{ctx}: rc");
            assert_eq!(a, want, "{ctx}: exact error code");
            common::eqb(&ctx, &co, &ro);
        }
    }

    // --- outlen boundaries at the argon2_hash level
    for (hashlen, want) in [
        (0usize, ARGON2_OUTPUT_TOO_SHORT),
        (15, ARGON2_OUTPUT_TOO_SHORT),
        (16, ARGON2_OK),
    ] {
        let mut cb = vec![0u8; 256];
        let mut rb = vec![0u8; 256];
        let a = unsafe {
            cie(
                1,
                8,
                1,
                pw.as_ptr() as *const c_void,
                pw.len(),
                salt.as_ptr() as *const c_void,
                salt.len(),
                hashlen,
                cb.as_mut_ptr() as *mut c_char,
                256,
            )
        };
        let b = unsafe {
            rie(
                1,
                8,
                1,
                pw.as_ptr() as *const c_void,
                pw.len(),
                salt.as_ptr() as *const c_void,
                salt.len(),
                hashlen,
                rb.as_mut_ptr() as *mut c_char,
                256,
            )
        };
        assert_eq!(a, b);
        assert_eq!(a, want, "hashlen={hashlen}");
        common::eqb(&format!("hashlen={hashlen}"), &cb, &rb);
    }

    // --- hashlen > ARGON2_MAX_OUTLEN (checked before any allocation);
    //     use the encoded (hash == NULL) form so randombytes_buf is not called.
    for hashlen in [0x1_0000_0000usize, usize::MAX] {
        let mut cb = vec![0u8; 64];
        let mut rb = vec![0u8; 64];
        let a = unsafe {
            cie(
                1,
                8,
                1,
                pw.as_ptr() as *const c_void,
                pw.len(),
                salt.as_ptr() as *const c_void,
                salt.len(),
                hashlen,
                cb.as_mut_ptr() as *mut c_char,
                64,
            )
        };
        let b = unsafe {
            rie(
                1,
                8,
                1,
                pw.as_ptr() as *const c_void,
                pw.len(),
                salt.as_ptr() as *const c_void,
                salt.len(),
                hashlen,
                rb.as_mut_ptr() as *mut c_char,
                64,
            )
        };
        assert_eq!((a, b), (ARGON2_OUTPUT_TOO_LONG, ARGON2_OUTPUT_TOO_LONG), "hashlen={hashlen}");
    }
    // --- pwdlen > ARGON2_MAX_PWD_LENGTH
    for pwdlen in [0x1_0000_0000usize, usize::MAX] {
        let a = unsafe {
            cie(
                1,
                8,
                1,
                pw.as_ptr() as *const c_void,
                pwdlen,
                salt.as_ptr() as *const c_void,
                salt.len(),
                32,
                std::ptr::null_mut(),
                0,
            )
        };
        let b = unsafe {
            rie(
                1,
                8,
                1,
                pw.as_ptr() as *const c_void,
                pwdlen,
                salt.as_ptr() as *const c_void,
                salt.len(),
                32,
                std::ptr::null_mut(),
                0,
            )
        };
        assert_eq!((a, b), (ARGON2_PWD_TOO_LONG, ARGON2_PWD_TOO_LONG), "pwdlen={pwdlen}");
    }
    // --- saltlen > ARGON2_MAX_SALT_LENGTH
    for saltlen in [0x1_0000_0000usize, usize::MAX] {
        let a = unsafe {
            cie(
                1,
                8,
                1,
                pw.as_ptr() as *const c_void,
                pw.len(),
                salt.as_ptr() as *const c_void,
                saltlen,
                32,
                std::ptr::null_mut(),
                0,
            )
        };
        let b = unsafe {
            rie(
                1,
                8,
                1,
                pw.as_ptr() as *const c_void,
                pw.len(),
                salt.as_ptr() as *const c_void,
                saltlen,
                32,
                std::ptr::null_mut(),
                0,
            )
        };
        assert_eq!((a, b), (ARGON2_SALT_TOO_LONG, ARGON2_SALT_TOO_LONG), "saltlen={saltlen}");
    }
    // --- saltlen below ARGON2_MIN_SALT_LENGTH
    for saltlen in [0usize, 1, 7] {
        let a = unsafe {
            cie(
                1,
                8,
                1,
                pw.as_ptr() as *const c_void,
                pw.len(),
                salt.as_ptr() as *const c_void,
                saltlen,
                32,
                std::ptr::null_mut(),
                0,
            )
        };
        let b = unsafe {
            rie(
                1,
                8,
                1,
                pw.as_ptr() as *const c_void,
                pw.len(),
                salt.as_ptr() as *const c_void,
                saltlen,
                32,
                std::ptr::null_mut(),
                0,
            )
        };
        assert_eq!((a, b), (ARGON2_SALT_TOO_SHORT, ARGON2_SALT_TOO_SHORT), "saltlen={saltlen}");
    }

    // --- encoded buffer too small -> ARGON2_ENCODING_FAIL.
    //     Only lengths that fail inside an `SS` step are safe to test: once the
    //     header fits, the C reaches sodium_bin2base64() with an undersized
    //     buffer, which calls sodium_misuse() -> abort().
    //     "$argon2id$v=19$m=8,t=1,p=1$" is 27 characters.
    for enclen in [1usize, 5, 12, 13, 20, 26, 27] {
        let mut cb = vec![0x77u8; 128];
        let mut rb = vec![0x77u8; 128];
        let a = unsafe {
            cde(
                1,
                8,
                1,
                pw.as_ptr() as *const c_void,
                pw.len(),
                salt.as_ptr() as *const c_void,
                salt.len(),
                32,
                cb.as_mut_ptr() as *mut c_char,
                enclen,
            )
        };
        let b = unsafe {
            rde(
                1,
                8,
                1,
                pw.as_ptr() as *const c_void,
                pw.len(),
                salt.as_ptr() as *const c_void,
                salt.len(),
                32,
                rb.as_mut_ptr() as *mut c_char,
                enclen,
            )
        };
        assert_eq!(a, b, "enclen={enclen}");
        assert_eq!(a, ARGON2_ENCODING_FAIL, "enclen={enclen}");
        common::eqb(&format!("enclen={enclen}"), &cb, &rb);
    }

    // --- generic argon2_hash: hash AND encoded requested at once; also
    //     type=Argon2_i(1) / Argon2_id(2) / out-of-range enum values.
    for ty in [1, 2, 0, 3, -1, 999] {
        let mut ch_hash = vec![0x22u8; 40];
        let mut rh_hash = vec![0x22u8; 40];
        let mut ch_enc = vec![0x22u8; 128];
        let mut rh_enc = vec![0x22u8; 128];
        reset_rng(0x3000);
        let a = unsafe {
            ch(
                1,
                8,
                1,
                pw.as_ptr() as *const c_void,
                pw.len(),
                salt.as_ptr() as *const c_void,
                salt.len(),
                ch_hash.as_mut_ptr() as *mut c_void,
                32,
                ch_enc.as_mut_ptr() as *mut c_char,
                128,
                ty,
            )
        };
        RSEED.store(0x3000, Ordering::SeqCst);
        let b = unsafe {
            rh(
                1,
                8,
                1,
                pw.as_ptr() as *const c_void,
                pw.len(),
                salt.as_ptr() as *const c_void,
                salt.len(),
                rh_hash.as_mut_ptr() as *mut c_void,
                32,
                rh_enc.as_mut_ptr() as *mut c_char,
                128,
                ty,
            )
        };
        assert_eq!(a, b, "argon2_hash type={ty}: rc");
        if ty == 1 || ty == 2 {
            assert_eq!(a, ARGON2_OK, "type={ty}");
        } else {
            assert_eq!(a, ARGON2_INCORRECT_TYPE, "type={ty} must be ARGON2_INCORRECT_TYPE");
        }
        common::eqb(&format!("argon2_hash type={ty} hash"), &ch_hash, &rh_hash);
        common::eqb(&format!("argon2_hash type={ty} enc"), &ch_enc, &rh_enc);
    }

    // --- hash == NULL && encoded == NULL (neither output requested)
    let a = unsafe {
        ch(
            1,
            8,
            1,
            pw.as_ptr() as *const c_void,
            pw.len(),
            salt.as_ptr() as *const c_void,
            salt.len(),
            std::ptr::null_mut(),
            32,
            std::ptr::null_mut(),
            0,
            1,
        )
    };
    let b = unsafe {
        rh(
            1,
            8,
            1,
            pw.as_ptr() as *const c_void,
            pw.len(),
            salt.as_ptr() as *const c_void,
            salt.len(),
            std::ptr::null_mut(),
            32,
            std::ptr::null_mut(),
            0,
            1,
        )
    };
    assert_eq!((a, b), (ARGON2_OK, ARGON2_OK), "argon2_hash no outputs");

    // --- encoded != NULL but encodedlen == 0 -> encoding skipped
    let mut cb = vec![0x99u8; 32];
    let mut rb = vec![0x99u8; 32];
    let a = unsafe {
        ch(
            1,
            8,
            1,
            pw.as_ptr() as *const c_void,
            pw.len(),
            salt.as_ptr() as *const c_void,
            salt.len(),
            std::ptr::null_mut(),
            32,
            cb.as_mut_ptr() as *mut c_char,
            0,
            2,
        )
    };
    let b = unsafe {
        rh(
            1,
            8,
            1,
            pw.as_ptr() as *const c_void,
            pw.len(),
            salt.as_ptr() as *const c_void,
            salt.len(),
            std::ptr::null_mut(),
            32,
            rb.as_mut_ptr() as *mut c_char,
            0,
            2,
        )
    };
    assert_eq!((a, b), (ARGON2_OK, ARGON2_OK));
    common::eqb("encodedlen==0", &cb, &rb);

    // --- pwd == NULL with pwdlen == 0 (accepted) and pwdlen != 0 (mismatch)
    for (pwdlen, want) in [(0usize, ARGON2_OK), (1, ARGON2_PWD_PTR_MISMATCH)] {
        let a = unsafe {
            cie(
                1,
                8,
                1,
                std::ptr::null(),
                pwdlen,
                salt.as_ptr() as *const c_void,
                salt.len(),
                32,
                std::ptr::null_mut(),
                0,
            )
        };
        let b = unsafe {
            rie(
                1,
                8,
                1,
                std::ptr::null(),
                pwdlen,
                salt.as_ptr() as *const c_void,
                salt.len(),
                32,
                std::ptr::null_mut(),
                0,
            )
        };
        assert_eq!((a, b), (want, want), "pwd=NULL pwdlen={pwdlen}");
    }
    // --- salt == NULL -> SALT_PTR_MISMATCH (saltlen != 0) / SALT_TOO_SHORT (0)
    for (saltlen, want) in [(0usize, ARGON2_SALT_TOO_SHORT), (8, ARGON2_SALT_PTR_MISMATCH)] {
        let a = unsafe {
            cie(
                1,
                8,
                1,
                pw.as_ptr() as *const c_void,
                pw.len(),
                std::ptr::null(),
                saltlen,
                32,
                std::ptr::null_mut(),
                0,
            )
        };
        let b = unsafe {
            rie(
                1,
                8,
                1,
                pw.as_ptr() as *const c_void,
                pw.len(),
                std::ptr::null(),
                saltlen,
                32,
                std::ptr::null_mut(),
                0,
            )
        };
        assert_eq!((a, b), (want, want), "salt=NULL saltlen={saltlen}");
    }
}

#[test]
fn argon2_verify_lowlevel() {
    let (cv, rv) = both!("_sodium_argon2_verify", FnA2VerifyT);
    let (civ, riv) = both!("_sodium_argon2i_verify", FnA2Verify);
    let (cdv, rdv) = both!("_sodium_argon2id_verify", FnA2Verify);
    let (cie, rie) = both!("_sodium_argon2i_hash_encoded", FnA2HashEnc);
    let (cde, rde) = both!("_sodium_argon2id_hash_encoded", FnA2HashEnc);

    let mut rng = common::Rng::new(0x1357_9BDF);

    for (ty, cenc, renc, civf, rivf) in
        [(1, cie, rie, civ, riv), (2, cde, rde, cdv, rdv)]
    {
        for case in 0..3 {
            let n_pw = 1 + rng.below(30);
            let pw = rng.bytes(n_pw);
            let n_salt = 8 + rng.below(16);
            let salt = rng.bytes(n_salt);
            let mut cb = vec![0u8; 160];
            let mut rb = vec![0u8; 160];
            let a = unsafe {
                cenc(
                    1 + case,
                    8 + case,
                    1,
                    pw.as_ptr() as *const c_void,
                    pw.len(),
                    salt.as_ptr() as *const c_void,
                    salt.len(),
                    32,
                    cb.as_mut_ptr() as *mut c_char,
                    160,
                )
            };
            let b = unsafe {
                renc(
                    1 + case,
                    8 + case,
                    1,
                    pw.as_ptr() as *const c_void,
                    pw.len(),
                    salt.as_ptr() as *const c_void,
                    salt.len(),
                    32,
                    rb.as_mut_ptr() as *mut c_char,
                    160,
                )
            };
            assert_eq!((a, b), (ARGON2_OK, ARGON2_OK));
            common::eqb("verify: encoded", &cb, &rb);

            // correct password
            let x = unsafe { civf(cb.as_ptr() as *const c_char, pw.as_ptr() as *const c_void, pw.len()) };
            let y = unsafe { rivf(cb.as_ptr() as *const c_char, pw.as_ptr() as *const c_void, pw.len()) };
            assert_eq!((x, y), (ARGON2_OK, ARGON2_OK));
            // generic argon2_verify with the same type
            let x = unsafe {
                cv(cb.as_ptr() as *const c_char, pw.as_ptr() as *const c_void, pw.len(), ty)
            };
            let y = unsafe {
                rv(cb.as_ptr() as *const c_char, pw.as_ptr() as *const c_void, pw.len(), ty)
            };
            assert_eq!((x, y), (ARGON2_OK, ARGON2_OK));
            // wrong password -> ARGON2_VERIFY_MISMATCH
            let bad = rng.bytes(pw.len() + 1);
            let x = unsafe {
                cv(cb.as_ptr() as *const c_char, bad.as_ptr() as *const c_void, bad.len(), ty)
            };
            let y = unsafe {
                rv(cb.as_ptr() as *const c_char, bad.as_ptr() as *const c_void, bad.len(), ty)
            };
            assert_eq!(x, y);
            assert_eq!(x, ARGON2_VERIFY_MISMATCH);
            // wrong type -> decoding failure
            let other = if ty == 1 { 2 } else { 1 };
            let x = unsafe {
                cv(cb.as_ptr() as *const c_char, pw.as_ptr() as *const c_void, pw.len(), other)
            };
            let y = unsafe {
                rv(cb.as_ptr() as *const c_char, pw.as_ptr() as *const c_void, pw.len(), other)
            };
            assert_eq!(x, y, "verify with wrong type");
            // out-of-range type enum -> ARGON2_INCORRECT_TYPE from decode_string
            for badty in [0, 3, -1, 999] {
                let x = unsafe {
                    cv(cb.as_ptr() as *const c_char, pw.as_ptr() as *const c_void, pw.len(), badty)
                };
                let y = unsafe {
                    rv(cb.as_ptr() as *const c_char, pw.as_ptr() as *const c_void, pw.len(), badty)
                };
                assert_eq!(x, y, "argon2_verify type={badty}");
                assert_eq!(x, ARGON2_INCORRECT_TYPE, "argon2_verify type={badty}");
            }
        }
    }

    // empty encoded string: strlen == 0 -> malloc(0), decode fails
    for ty in [1, 2] {
        let x = unsafe { cv(b"\0".as_ptr() as *const c_char, b"p".as_ptr() as *const c_void, 1, ty) };
        let y = unsafe { rv(b"\0".as_ptr() as *const c_char, b"p".as_ptr() as *const c_void, 1, ty) };
        assert_eq!(x, y, "argon2_verify(\"\")");
        assert_eq!(x, ARGON2_DECODING_FAIL);
    }
}

#[test]
fn argon2_validate_inputs_matrix() {
    let (c, r) = both!("_sodium_argon2_validate_inputs", FnValidate);
    let mut out = [0u8; 64];
    let mut pwd = [0u8; 8];
    let mut salt = [0u8; 16];
    let mut secret = [0u8; 8];
    let mut ad = [0u8; 8];

    let base = Argon2Context {
        out: out.as_mut_ptr(),
        outlen: 32,
        pwd: pwd.as_mut_ptr(),
        pwdlen: 8,
        salt: salt.as_mut_ptr(),
        saltlen: 16,
        secret: std::ptr::null_mut(),
        secretlen: 0,
        ad: std::ptr::null_mut(),
        adlen: 0,
        t_cost: 1,
        m_cost: 8,
        lanes: 1,
        threads: 1,
        flags: 0,
    };

    let mut cases: Vec<(&str, Argon2Context, c_int)> = Vec::new();
    cases.push(("valid", base, ARGON2_OK));
    cases.push(("out NULL", Argon2Context { out: std::ptr::null_mut(), ..base }, ARGON2_OUTPUT_PTR_NULL));
    for ol in [0u32, 1, 15] {
        cases.push(("outlen short", Argon2Context { outlen: ol, ..base }, ARGON2_OUTPUT_TOO_SHORT));
    }
    cases.push(("outlen 16", Argon2Context { outlen: 16, ..base }, ARGON2_OK));
    cases.push(("outlen u32::MAX", Argon2Context { outlen: u32::MAX, ..base }, ARGON2_OK));
    cases.push((
        "pwd NULL len!=0",
        Argon2Context { pwd: std::ptr::null_mut(), pwdlen: 1, ..base },
        ARGON2_PWD_PTR_MISMATCH,
    ));
    cases.push((
        "pwd NULL len==0",
        Argon2Context { pwd: std::ptr::null_mut(), pwdlen: 0, ..base },
        ARGON2_OK,
    ));
    cases.push((
        "salt NULL len!=0",
        Argon2Context { salt: std::ptr::null_mut(), saltlen: 16, ..base },
        ARGON2_SALT_PTR_MISMATCH,
    ));
    for sl in [0u32, 1, 7] {
        cases.push(("saltlen short", Argon2Context { saltlen: sl, ..base }, ARGON2_SALT_TOO_SHORT));
    }
    cases.push(("saltlen 8", Argon2Context { saltlen: 8, ..base }, ARGON2_OK));
    cases.push((
        "secret NULL len!=0",
        Argon2Context { secret: std::ptr::null_mut(), secretlen: 3, ..base },
        ARGON2_SECRET_PTR_MISMATCH,
    ));
    cases.push((
        "secret set",
        Argon2Context { secret: secret.as_mut_ptr(), secretlen: 8, ..base },
        ARGON2_OK,
    ));
    cases.push((
        "secret set len 0",
        Argon2Context { secret: secret.as_mut_ptr(), secretlen: 0, ..base },
        ARGON2_OK,
    ));
    cases.push((
        "ad NULL len!=0",
        Argon2Context { ad: std::ptr::null_mut(), adlen: 5, ..base },
        ARGON2_AD_PTR_MISMATCH,
    ));
    cases.push(("ad set", Argon2Context { ad: ad.as_mut_ptr(), adlen: 8, ..base }, ARGON2_OK));
    cases.push(("ad set len 0", Argon2Context { ad: ad.as_mut_ptr(), adlen: 0, ..base }, ARGON2_OK));
    cases.push(("lanes 0", Argon2Context { lanes: 0, threads: 1, ..base }, ARGON2_LANES_TOO_FEW));
    cases.push((
        "lanes 0x1000000",
        Argon2Context { lanes: 0x1000000, threads: 1, m_cost: u32::MAX, ..base },
        ARGON2_LANES_TOO_MANY,
    ));
    cases.push((
        "lanes 0xFFFFFF ok-range but m_cost too small",
        Argon2Context { lanes: 0xFFFFFF, threads: 1, m_cost: 8, ..base },
        ARGON2_MEMORY_TOO_LITTLE,
    ));
    for m in [0u32, 1, 7] {
        cases.push(("m_cost < MIN_MEMORY", Argon2Context { m_cost: m, ..base }, ARGON2_MEMORY_TOO_LITTLE));
    }
    cases.push(("m_cost 8", Argon2Context { m_cost: 8, ..base }, ARGON2_OK));
    cases.push((
        "m_cost < 8*lanes",
        Argon2Context { lanes: 2, threads: 2, m_cost: 15, ..base },
        ARGON2_MEMORY_TOO_LITTLE,
    ));
    cases.push((
        "m_cost == 8*lanes",
        Argon2Context { lanes: 2, threads: 2, m_cost: 16, ..base },
        ARGON2_OK,
    ));
    cases.push(("t_cost 0", Argon2Context { t_cost: 0, ..base }, ARGON2_TIME_TOO_SMALL));
    cases.push(("t_cost u32::MAX", Argon2Context { t_cost: u32::MAX, ..base }, ARGON2_OK));
    cases.push(("threads 0", Argon2Context { threads: 0, ..base }, ARGON2_THREADS_TOO_FEW));
    cases.push((
        "threads 0x1000000",
        Argon2Context { threads: 0x1000000, ..base },
        ARGON2_THREADS_TOO_MANY,
    ));
    cases.push(("threads 0xFFFFFF", Argon2Context { threads: 0xFFFFFF, ..base }, ARGON2_OK));
    // m_cost == MAX_MEMORY (0xFFFFFFFF) is accepted by validate_inputs
    cases.push(("m_cost u32::MAX", Argon2Context { m_cost: u32::MAX, ..base }, ARGON2_OK));

    for (label, mut ctx, want) in cases {
        let a = unsafe { c(&ctx as *const Argon2Context) };
        let b = unsafe { r(&ctx as *const Argon2Context) };
        assert_eq!(a, b, "validate_inputs({label})");
        assert_eq!(a, want, "validate_inputs({label}) exact code");
        // validate_inputs must not mutate the context
        let before = ctx;
        unsafe { c(&mut ctx as *const Argon2Context) };
        assert_eq!(before, ctx, "validate_inputs({label}) mutated context");
    }

    // context == NULL
    let a = unsafe { c(std::ptr::null()) };
    let b = unsafe { r(std::ptr::null()) };
    assert_eq!((a, b), (ARGON2_INCORRECT_PARAMETER, ARGON2_INCORRECT_PARAMETER));
}

#[test]
fn argon2_ctx_direct() {
    let (c, r) = both!("_sodium_argon2_ctx", FnA2Ctx);
    let mut rng = common::Rng::new(0x0102_0304);

    for &(t_cost, m_cost, lanes) in
        &[(1u32, 8u32, 1u32), (2, 8, 1), (1, 16, 2), (3, 32, 4), (1, 33, 2), (2, 100, 3)]
    {
        for case in 0..3 {
            let n_pwd = 1 + rng.below(20);
            let pwd = rng.bytes(n_pwd);
            let n_salt = 8 + rng.below(20);
            let salt = rng.bytes(n_salt);
            let n_ad = 1 + rng.below(10);
            let ad = rng.bytes(n_ad);
            let n_secret = 1 + rng.below(10);
            let secret = rng.bytes(n_secret);
            for outlen in [16u32, 32, 64, 80] {
                let mut co = vec![0x66u8; outlen as usize + 8];
                let mut ro = vec![0x66u8; outlen as usize + 8];
                let mk = |o: &mut Vec<u8>| Argon2Context {
                    out: o.as_mut_ptr(),
                    outlen,
                    pwd: pwd.as_ptr() as *mut u8,
                    pwdlen: pwd.len() as u32,
                    salt: salt.as_ptr() as *mut u8,
                    saltlen: salt.len() as u32,
                    secret: secret.as_ptr() as *mut u8,
                    secretlen: secret.len() as u32,
                    ad: ad.as_ptr() as *mut u8,
                    adlen: ad.len() as u32,
                    t_cost,
                    m_cost,
                    lanes,
                    threads: lanes,
                    flags: 0,
                };
                for ty in [1, 2] {
                    let mut cctx = mk(&mut co);
                    let mut rctx = mk(&mut ro);
                    let a = unsafe { c(&mut cctx, ty) };
                    let b = unsafe { r(&mut rctx, ty) };
                    let label = format!(
                        "argon2_ctx t={t_cost} m={m_cost} l={lanes} outlen={outlen} ty={ty} case={case}"
                    );
                    assert_eq!(a, b, "{label}: rc");
                    assert_eq!(a, ARGON2_OK, "{label}");
                    common::eqb(&label, &co, &ro);
                    // argon2_ctx must not modify the context (flags==0)
                    assert_eq!(cctx.outlen, rctx.outlen);
                    assert_eq!(cctx.pwdlen, rctx.pwdlen);
                    assert_eq!(cctx.secretlen, rctx.secretlen);
                }
            }
        }
    }

    // out-of-range type enum: validate_inputs passes, then INCORRECT_TYPE
    let mut co = vec![0u8; 32];
    let mut ro = vec![0u8; 32];
    let salt = [3u8; 16];
    let pwd = [4u8; 4];
    for ty in [0, 3, -1, 999, i32::MIN] {
        let mk = |o: &mut Vec<u8>| Argon2Context {
            out: o.as_mut_ptr(),
            outlen: 32,
            pwd: pwd.as_ptr() as *mut u8,
            pwdlen: 4,
            salt: salt.as_ptr() as *mut u8,
            saltlen: 16,
            secret: std::ptr::null_mut(),
            secretlen: 0,
            ad: std::ptr::null_mut(),
            adlen: 0,
            t_cost: 1,
            m_cost: 8,
            lanes: 1,
            threads: 1,
            flags: 0,
        };
        let mut cctx = mk(&mut co);
        let mut rctx = mk(&mut ro);
        let a = unsafe { c(&mut cctx, ty) };
        let b = unsafe { r(&mut rctx, ty) };
        assert_eq!(a, b, "argon2_ctx type={ty}");
        assert_eq!(a, ARGON2_INCORRECT_TYPE, "argon2_ctx type={ty}");
    }

    // validation failures are returned unchanged by argon2_ctx
    for (label, t, m, l, th, want) in [
        ("t=0", 0u32, 8u32, 1u32, 1u32, ARGON2_TIME_TOO_SMALL),
        ("m=7", 1, 7, 1, 1, ARGON2_MEMORY_TOO_LITTLE),
        ("lanes=0", 1, 8, 0, 1, ARGON2_LANES_TOO_FEW),
        ("threads=0", 1, 8, 1, 0, ARGON2_THREADS_TOO_FEW),
        ("m<8*lanes", 1, 8, 2, 2, ARGON2_MEMORY_TOO_LITTLE),
    ] {
        let mk = |o: &mut Vec<u8>| Argon2Context {
            out: o.as_mut_ptr(),
            outlen: 32,
            pwd: pwd.as_ptr() as *mut u8,
            pwdlen: 4,
            salt: salt.as_ptr() as *mut u8,
            saltlen: 16,
            secret: std::ptr::null_mut(),
            secretlen: 0,
            ad: std::ptr::null_mut(),
            adlen: 0,
            t_cost: t,
            m_cost: m,
            lanes: l,
            threads: th,
            flags: 0,
        };
        let mut cctx = mk(&mut co);
        let mut rctx = mk(&mut ro);
        let a = unsafe { c(&mut cctx, 2) };
        let b = unsafe { r(&mut rctx, 2) };
        assert_eq!(a, b, "argon2_ctx {label}");
        assert_eq!(a, want, "argon2_ctx {label}");
    }

    // context == NULL -> INCORRECT_PARAMETER
    let a = unsafe { c(std::ptr::null_mut(), 2) };
    let b = unsafe { r(std::ptr::null_mut(), 2) };
    assert_eq!((a, b), (ARGON2_INCORRECT_PARAMETER, ARGON2_INCORRECT_PARAMETER));
}

// ===========================================================================
// argon2-core internals: initialize / fill_memory_blocks / fill_segment_ref /
// finalize, driven directly with an argon2_instance_t.
// ===========================================================================

fn region_bytes(inst: &Argon2Instance) -> &[u8] {
    unsafe {
        let reg = &*inst.region;
        std::slice::from_raw_parts(reg.memory as *const u8, inst.memory_blocks as usize * 1024)
    }
}

#[test]
fn argon2_core_internals() {
    let (cinit, rinit) = both!("_sodium_argon2_initialize", FnInitialize);
    let (cfill, rfill) = both!("_sodium_argon2_fill_memory_blocks", FnFillMem);
    let (cseg, rseg) = both!("_sodium_argon2_fill_segment_ref", FnFillSeg);
    let (cfin, rfin) = both!("_sodium_argon2_finalize", FnFinalize);

    let mut rng = common::Rng::new(0xFEED_BEEF);

    // (t_cost, m_cost, lanes) — lane_length/segment_length exercise the
    // starting_index / prev_offset wraparound paths of fill_segment_ref.
    for &(t_cost, m_cost, lanes, ty) in
        &[(1u32, 8u32, 1u32, 1i32), (1, 8, 1, 2), (2, 16, 2, 2), (2, 32, 2, 1), (3, 48, 3, 2)]
    {
        let n_pwd = 1 + rng.below(20);
        let pwd = rng.bytes(n_pwd);
        let salt = rng.bytes(16);
        let outlen: u32 = 32;
        let mut co = vec![0x77u8; outlen as usize];
        let mut ro = vec![0x77u8; outlen as usize];

        let mk_ctx = |o: &mut Vec<u8>| Argon2Context {
            out: o.as_mut_ptr(),
            outlen,
            pwd: pwd.as_ptr() as *mut u8,
            pwdlen: pwd.len() as u32,
            salt: salt.as_ptr() as *mut u8,
            saltlen: salt.len() as u32,
            secret: std::ptr::null_mut(),
            secretlen: 0,
            ad: std::ptr::null_mut(),
            adlen: 0,
            t_cost,
            m_cost,
            lanes,
            threads: lanes,
            flags: 0,
        };

        // replicate argon2_ctx's derivation of the instance fields
        let mut memory_blocks = m_cost;
        if memory_blocks < 2 * 4 * lanes {
            memory_blocks = 2 * 4 * lanes;
        }
        let segment_length = memory_blocks / (lanes * 4);
        memory_blocks = segment_length * (lanes * 4);
        let mk_inst = || Argon2Instance {
            region: std::ptr::null_mut(),
            pseudo_rands: std::ptr::null_mut(),
            passes: t_cost,
            current_pass: !0u32,
            memory_blocks,
            segment_length,
            lane_length: segment_length * 4,
            lanes,
            threads: lanes,
            type_: ty,
            print_internals: 0,
        };

        let mut cctx = mk_ctx(&mut co);
        let mut rctx = mk_ctx(&mut ro);
        let mut cinst = mk_inst();
        let mut rinst = mk_inst();

        let a = unsafe { cinit(&mut cinst, &mut cctx) };
        let b = unsafe { rinit(&mut rinst, &mut rctx) };
        let label = format!("initialize t={t_cost} m={m_cost} l={lanes} ty={ty}");
        assert_eq!((a, b), (ARGON2_OK, ARGON2_OK), "{label}");
        // derived fields must be untouched and equal
        assert_eq!(cinst.memory_blocks, rinst.memory_blocks, "{label} memory_blocks");
        assert_eq!(cinst.segment_length, rinst.segment_length, "{label} segment_length");
        assert_eq!(cinst.lane_length, rinst.lane_length, "{label} lane_length");
        assert!(!cinst.region.is_null() && !rinst.region.is_null());
        assert_eq!(
            unsafe { (*cinst.region).size },
            unsafe { (*rinst.region).size },
            "{label} region size"
        );
        // only blocks 0 and 1 of each lane are initialised here
        let cm = region_bytes(&cinst);
        let rm = region_bytes(&rinst);
        for l in 0..lanes as usize {
            for blk in 0..2usize {
                let off = (l * cinst.lane_length as usize + blk) * 1024;
                common::eqb(
                    &format!("{label} first-block l={l} b={blk}"),
                    &cm[off..off + 1024],
                    &rm[off..off + 1024],
                );
            }
        }

        // fill each pass and compare the whole region
        for pass in 0..t_cost {
            unsafe { cfill(&mut cinst, pass) };
            unsafe { rfill(&mut rinst, pass) };
            common::eqb(
                &format!("{label} after fill_memory_blocks pass={pass}"),
                region_bytes(&cinst),
                region_bytes(&rinst),
            );
        }

        unsafe { cfin(&cctx, &mut cinst) };
        unsafe { rfin(&rctx, &mut rinst) };
        common::eqb(&format!("{label} finalize out"), &co, &ro);
        assert!(cinst.region.is_null() && rinst.region.is_null(), "{label} region freed");
        assert!(cinst.pseudo_rands.is_null() && rinst.pseudo_rands.is_null());

        // ---- again, but driving argon2_fill_segment_ref directly
        let mut co2 = vec![0x77u8; outlen as usize];
        let mut ro2 = vec![0x77u8; outlen as usize];
        let mut cctx = mk_ctx(&mut co2);
        let mut rctx = mk_ctx(&mut ro2);
        let mut cinst = mk_inst();
        let mut rinst = mk_inst();
        assert_eq!(unsafe { cinit(&mut cinst, &mut cctx) }, ARGON2_OK);
        assert_eq!(unsafe { rinit(&mut rinst, &mut rctx) }, ARGON2_OK);
        for pass in 0..t_cost {
            for s in 0..4u8 {
                for l in 0..lanes {
                    let pos = Argon2Position { pass, lane: l, slice: s, index: 0 };
                    unsafe { cseg(&cinst, pos) };
                    unsafe { rseg(&rinst, pos) };
                }
            }
            // Compare only after a full pass: within a pass, blocks that have
            // not been filled yet still hold uninitialised malloc contents.
            common::eqb(
                &format!("{label} fill_segment_ref pass={pass}"),
                region_bytes(&cinst),
                region_bytes(&rinst),
            );
        }
        unsafe { cfin(&cctx, &mut cinst) };
        unsafe { rfin(&rctx, &mut rinst) };
        common::eqb(&format!("{label} finalize out (manual segments)"), &co2, &ro2);
    }

    // NULL handling of the core entry points
    let a = unsafe { cinit(std::ptr::null_mut(), std::ptr::null_mut()) };
    let b = unsafe { rinit(std::ptr::null_mut(), std::ptr::null_mut()) };
    assert_eq!((a, b), (ARGON2_INCORRECT_PARAMETER, ARGON2_INCORRECT_PARAMETER));
    // fill_memory_blocks(NULL, _) and finalize(NULL, NULL) are no-ops
    unsafe { cfill(std::ptr::null_mut(), 0) };
    unsafe { rfill(std::ptr::null_mut(), 0) };
    unsafe { cfin(std::ptr::null(), std::ptr::null_mut()) };
    unsafe { rfin(std::ptr::null(), std::ptr::null_mut()) };
    // fill_memory_blocks with lanes == 0 also returns immediately
    let mut z = Argon2Instance {
        region: std::ptr::null_mut(),
        pseudo_rands: std::ptr::null_mut(),
        passes: 1,
        current_pass: 0,
        memory_blocks: 0,
        segment_length: 0,
        lane_length: 0,
        lanes: 0,
        threads: 0,
        type_: 1,
        print_internals: 0,
    };
    unsafe { cfill(&mut z, 0) };
    unsafe { rfill(&mut z, 0) };
    // fill_segment_ref(NULL, pos) is a no-op
    let pos = Argon2Position { pass: 0, lane: 0, slice: 0, index: 0 };
    unsafe { cseg(std::ptr::null(), pos) };
    unsafe { rseg(std::ptr::null(), pos) };
}

// ===========================================================================
// argon2-encoding: encode_string / decode_string
// ===========================================================================

#[test]
fn argon2_encode_string_direct() {
    let (c, r) = both!("_sodium_argon2_encode_string", FnEncodeString);
    let mut rng = common::Rng::new(0xEEEE_1111);

    for case in 0..8 {
        let saltlen = 8 + rng.below(25);
        let outlen = 16 + rng.below(50);
        let salt = rng.bytes(saltlen);
        let mut out = rng.bytes(outlen);
        let m_cost = [8u32, 9, 100, 65536, 4294967295][case % 5];
        let t_cost = [1u32, 2, 7, 4294967295, 3][case % 5];
        let lanes = [1u32, 2, 3, 4, 5][case % 5];
        let m_cost = if m_cost < 8 * lanes { 8 * lanes } else { m_cost };

        let mut ctx = Argon2Context {
            out: out.as_mut_ptr(),
            outlen: outlen as u32,
            pwd: std::ptr::null_mut(),
            pwdlen: 0,
            salt: salt.as_ptr() as *mut u8,
            saltlen: saltlen as u32,
            secret: std::ptr::null_mut(),
            secretlen: 0,
            ad: std::ptr::null_mut(),
            adlen: 0,
            t_cost,
            m_cost,
            lanes,
            threads: lanes,
            flags: 0,
        };
        for ty in [1, 2, 0, 3, -1] {
            // Find the exact required length, and the length of the fixed
            // header up to (and including) the '$' that precedes the salt.
            //
            // IMPORTANT: for header_len < dst_len < need the C reaches
            // sodium_bin2base64() with an undersized buffer, which calls
            // sodium_misuse() -> abort(). That range is therefore untestable
            // in-process (see pwhash-E41 in _v/errors/pwhash.md); only
            // dst_len <= header_len (an `SS` check fails) or dst_len >= need
            // are exercised here.
            let mut probe = vec![0u8; 512];
            let (need, header_len) = unsafe {
                let rc = c(
                    probe.as_mut_ptr() as *mut c_char,
                    512,
                    &mut ctx,
                    if ty == 1 || ty == 2 { ty } else { 1 },
                );
                assert_eq!(rc, ARGON2_OK);
                let z = probe.iter().position(|&b| b == 0).unwrap();
                let mut dollars = 0usize;
                let mut h = 0usize;
                for (i, &b) in probe[..z].iter().enumerate() {
                    if b == b'$' {
                        dollars += 1;
                        if dollars == 4 {
                            h = i + 1;
                            break;
                        }
                    }
                }
                assert!(h > 0);
                (z + 1, h)
            };
            for dst_len in
                [0usize, 1, 5, 11, 12, 13, header_len - 1, header_len, need, need + 1, 512]
            {
                let mut cb = vec![0xCCu8; 600];
                let mut rb = vec![0xCCu8; 600];
                let mut cctx = ctx;
                let mut rctx = ctx;
                let a = unsafe { c(cb.as_mut_ptr() as *mut c_char, dst_len, &mut cctx, ty) };
                let b = unsafe { r(rb.as_mut_ptr() as *mut c_char, dst_len, &mut rctx, ty) };
                let label = format!("encode_string case={case} ty={ty} dst_len={dst_len}");
                assert_eq!(a, b, "{label}: rc");
                common::eqb(&label, &cb, &rb);
                assert_eq!(cctx, rctx, "{label}: ctx");
                if dst_len >= need && (ty == 1 || ty == 2) {
                    assert_eq!(a, ARGON2_OK, "{label}");
                } else if ty != 1 && ty != 2 {
                    assert_eq!(a, ARGON2_ENCODING_FAIL, "{label}: bad type");
                } else {
                    assert_eq!(a, ARGON2_ENCODING_FAIL, "{label}");
                }
            }
        }
    }

    // encode_string with an invalid ctx: the C emits the prefix first, so a
    // large-enough buffer yields the *validation* error code.
    let salt = [0u8; 16];
    let mut out = [0u8; 32];
    for (label, mut ctx, want) in [
        (
            "outlen 0",
            Argon2Context {
                out: out.as_mut_ptr(),
                outlen: 0,
                salt: salt.as_ptr() as *mut u8,
                saltlen: 16,
                t_cost: 1,
                m_cost: 8,
                lanes: 1,
                threads: 1,
                ..Argon2Context::zeroed()
            },
            ARGON2_OUTPUT_TOO_SHORT,
        ),
        (
            "lanes 0",
            Argon2Context {
                out: out.as_mut_ptr(),
                outlen: 32,
                salt: salt.as_ptr() as *mut u8,
                saltlen: 16,
                t_cost: 1,
                m_cost: 8,
                lanes: 0,
                threads: 1,
                ..Argon2Context::zeroed()
            },
            ARGON2_LANES_TOO_FEW,
        ),
        (
            "out NULL",
            Argon2Context {
                out: std::ptr::null_mut(),
                outlen: 32,
                salt: salt.as_ptr() as *mut u8,
                saltlen: 16,
                t_cost: 1,
                m_cost: 8,
                lanes: 1,
                threads: 1,
                ..Argon2Context::zeroed()
            },
            ARGON2_OUTPUT_PTR_NULL,
        ),
    ] {
        let mut cb = vec![0xCCu8; 256];
        let mut rb = vec![0xCCu8; 256];
        let a = unsafe { c(cb.as_mut_ptr() as *mut c_char, 256, &mut ctx, 2) };
        let b = unsafe { r(rb.as_mut_ptr() as *mut c_char, 256, &mut ctx, 2) };
        assert_eq!(a, b, "encode_string invalid {label}");
        assert_eq!(a, want, "encode_string invalid {label}");
        common::eqb(&format!("encode_string invalid {label}"), &cb, &rb);
    }
}

#[test]
fn argon2_decode_string_direct() {
    let (c, r) = both!("_sodium_argon2_decode_string", FnDecodeString);

    let good_id = "$argon2id$v=19$m=8,t=1,p=1$AAAAAAAAAAAAAAAAAAAAAA$AAAAAAAAAAAAAAAAAAAAAA";
    let good_i = "$argon2i$v=19$m=8,t=1,p=1$AAAAAAAAAAAAAAAAAAAAAA$AAAAAAAAAAAAAAAAAAAAAA";

    let strings: Vec<String> = vec![
        good_id.into(),
        good_i.into(),
        "".into(),
        "$".into(),
        "$argon2".into(),
        "$argon2id".into(),
        "$argon2id$".into(),
        "$argon2id$v=".into(),
        "$argon2id$v=19".into(),
        "$argon2id$v=19$".into(),
        "$argon2id$v=19$m=".into(),
        "$argon2id$v=0$m=8,t=1,p=1$AAAAAAAAAAAAAAAAAAAAAA$AAAAAAAAAAAAAAAAAAAAAA".into(),
        "$argon2id$v=18$m=8,t=1,p=1$AAAAAAAAAAAAAAAAAAAAAA$AAAAAAAAAAAAAAAAAAAAAA".into(),
        "$argon2id$v=20$m=8,t=1,p=1$AAAAAAAAAAAAAAAAAAAAAA$AAAAAAAAAAAAAAAAAAAAAA".into(),
        "$argon2id$v=019$m=8,t=1,p=1$AAAAAAAAAAAAAAAAAAAAAA$AAAAAAAAAAAAAAAAAAAAAA".into(),
        "$argon2id$v=4294967295$m=8,t=1,p=1$AAAAAAAAAAAAAAAAAAAAAA$AAAAAAAAAAAAAAAAAAAAAA".into(),
        "$argon2id$v=4294967296$m=8,t=1,p=1$AAAAAAAAAAAAAAAAAAAAAA$AAAAAAAAAAAAAAAAAAAAAA".into(),
        "$argon2id$v=99999999999999999999999$m=8,t=1,p=1$AAAAAAAAAAAAAAAAAAAAAA$AAAAAAAAAAAAAAAAAAAAAA".into(),
        "$argon2id$v=19$m=0,t=1,p=1$AAAAAAAAAAAAAAAAAAAAAA$AAAAAAAAAAAAAAAAAAAAAA".into(),
        "$argon2id$v=19$m=7,t=1,p=1$AAAAAAAAAAAAAAAAAAAAAA$AAAAAAAAAAAAAAAAAAAAAA".into(),
        "$argon2id$v=19$m=08,t=1,p=1$AAAAAAAAAAAAAAAAAAAAAA$AAAAAAAAAAAAAAAAAAAAAA".into(),
        "$argon2id$v=19$m=4294967296,t=1,p=1$AAAAAAAAAAAAAAAAAAAAAA$AAAAAAAAAAAAAAAAAAAAAA".into(),
        "$argon2id$v=19$m=8,t=0,p=1$AAAAAAAAAAAAAAAAAAAAAA$AAAAAAAAAAAAAAAAAAAAAA".into(),
        "$argon2id$v=19$m=8,t=4294967296,p=1$AAAAAAAAAAAAAAAAAAAAAA$AAAAAAAAAAAAAAAAAAAAAA".into(),
        "$argon2id$v=19$m=8,t=1,p=0$AAAAAAAAAAAAAAAAAAAAAA$AAAAAAAAAAAAAAAAAAAAAA".into(),
        "$argon2id$v=19$m=8,t=1,p=16777216$AAAAAAAAAAAAAAAAAAAAAA$AAAAAAAAAAAAAAAAAAAAAA".into(),
        "$argon2id$v=19$m=8,t=1,p=4294967296$AAAAAAAAAAAAAAAAAAAAAA$AAAAAAAAAAAAAAAAAAAAAA".into(),
        "$argon2id$v=19$M=8,t=1,p=1$AAAAAAAAAAAAAAAAAAAAAA$AAAAAAAAAAAAAAAAAAAAAA".into(),
        "$argon2id$v=19$m=8;t=1,p=1$AAAAAAAAAAAAAAAAAAAAAA$AAAAAAAAAAAAAAAAAAAAAA".into(),
        // salt too short (< 8 bytes decoded)
        "$argon2id$v=19$m=8,t=1,p=1$AAAA$AAAAAAAAAAAAAAAAAAAAAA".into(),
        // out too short (< 16 bytes decoded)
        "$argon2id$v=19$m=8,t=1,p=1$AAAAAAAAAAAAAAAAAAAAAA$AAAA".into(),
        // missing final field
        "$argon2id$v=19$m=8,t=1,p=1$AAAAAAAAAAAAAAAAAAAAAA".into(),
        "$argon2id$v=19$m=8,t=1,p=1$AAAAAAAAAAAAAAAAAAAAAA$".into(),
        // trailing junk
        format!("{good_id}!"),
        format!("{good_id}$"),
        format!("{good_id}AAAA"),
        // invalid base64 characters
        "$argon2id$v=19$m=8,t=1,p=1$AAAA!AAAAAAAAAAAAAAAAA$AAAAAAAAAAAAAAAAAAAAAA".into(),
        // base64 with padding (not allowed by VARIANT_ORIGINAL_NO_PADDING)
        "$argon2id$v=19$m=8,t=1,p=1$AAAAAAAAAAAAAAAAAAAAAA==$AAAAAAAAAAAAAAAAAAAAAA".into(),
        // very long base64 -> exceeds the caller-provided max lengths
        format!(
            "$argon2id$v=19$m=8,t=1,p=1${}${}",
            "A".repeat(400),
            "A".repeat(400)
        ),
    ];

    for s in &strings {
        let cs = std::ffi::CString::new(s.as_str()).unwrap();
        for ty in [1, 2, 0, 3, -1, 999] {
            // ctx buffers sized exactly like argon2_verify does it: strlen(str)
            let n = s.len().max(1);
            for maxes in [(n as u32, n as u32), (8u32, 16u32), (0, 0), (32, 16)] {
                let mut csalt = vec![0xABu8; n + 8];
                let mut rsalt = vec![0xABu8; n + 8];
                let mut cout = vec![0xCDu8; n + 8];
                let mut rout = vec![0xCDu8; n + 8];
                let mut cad = vec![0u8; n + 8];
                let mut rad = vec![0u8; n + 8];
                let mk = |salt: &mut Vec<u8>, out: &mut Vec<u8>, ad: &mut Vec<u8>| Argon2Context {
                    out: out.as_mut_ptr(),
                    outlen: maxes.1,
                    pwd: std::ptr::null_mut(),
                    pwdlen: 0,
                    salt: salt.as_mut_ptr(),
                    saltlen: maxes.0,
                    secret: std::ptr::null_mut(),
                    secretlen: 0,
                    ad: ad.as_mut_ptr(),
                    adlen: 0,
                    t_cost: 0,
                    m_cost: 0,
                    lanes: 0,
                    threads: 0,
                    flags: 0,
                };
                let mut cctx = mk(&mut csalt, &mut cout, &mut cad);
                let mut rctx = mk(&mut rsalt, &mut rout, &mut rad);
                let a = unsafe { c(&mut cctx, cs.as_ptr(), ty) };
                let b = unsafe { r(&mut rctx, cs.as_ptr(), ty) };
                let label = format!("decode_string ty={ty} maxes={maxes:?} str={s:?}");
                assert_eq!(a, b, "{label}: rc");
                // out-params must agree
                assert_eq!(cctx.outlen, rctx.outlen, "{label}: outlen");
                assert_eq!(cctx.saltlen, rctx.saltlen, "{label}: saltlen");
                assert_eq!(cctx.m_cost, rctx.m_cost, "{label}: m_cost");
                assert_eq!(cctx.t_cost, rctx.t_cost, "{label}: t_cost");
                assert_eq!(cctx.lanes, rctx.lanes, "{label}: lanes");
                assert_eq!(cctx.threads, rctx.threads, "{label}: threads");
                common::eqb(&format!("{label}: salt buf"), &csalt, &rsalt);
                common::eqb(&format!("{label}: out buf"), &cout, &rout);
                if ty != 1 && ty != 2 {
                    assert_eq!(a, ARGON2_INCORRECT_TYPE, "{label}: bad type code");
                }
            }
        }
    }

    // the two well-formed strings must decode successfully with the right type
    for (s, ty) in [(good_id, 2), (good_i, 1)] {
        let cs = std::ffi::CString::new(s).unwrap();
        let n = s.len();
        let mut csalt = vec![0u8; n];
        let mut cout = vec![0u8; n];
        let mut ctx = Argon2Context {
            out: cout.as_mut_ptr(),
            outlen: n as u32,
            salt: csalt.as_mut_ptr(),
            saltlen: n as u32,
            ..Argon2Context::zeroed()
        };
        let a = unsafe { c(&mut ctx, cs.as_ptr(), ty) };
        assert_eq!(a, ARGON2_OK, "{s} should decode");
        assert_eq!((ctx.m_cost, ctx.t_cost, ctx.lanes, ctx.threads), (8, 1, 1, 1));
        assert_eq!((ctx.saltlen, ctx.outlen), (16, 16));
    }
}

// ===========================================================================
// blake2b-long
// ===========================================================================

#[test]
fn blake2b_long_test() {
    let (c, r) = both!("_sodium_blake2b_long", FnBlake2bLong);
    let mut rng = common::Rng::new(0xB2B2_B2B2);

    for &outlen in &[
        0usize, 1, 2, 15, 16, 31, 32, 63, 64, 65, 66, 95, 96, 97, 127, 128, 129, 200, 1024, 1025,
    ] {
        for &inlen in &[0usize, 1, 4, 63, 64, 72, 128, 1024] {
            let inp = rng.bytes(inlen.max(1));
            let mut co = vec![0x4Du8; outlen + 16];
            let mut ro = vec![0x4Du8; outlen + 16];
            let a = unsafe {
                c(co.as_mut_ptr() as *mut c_void, outlen, inp.as_ptr() as *const c_void, inlen)
            };
            let b = unsafe {
                r(ro.as_mut_ptr() as *mut c_void, outlen, inp.as_ptr() as *const c_void, inlen)
            };
            let label = format!("blake2b_long outlen={outlen} inlen={inlen}");
            assert_eq!(a, b, "{label}: rc");
            common::eqb(&label, &co, &ro);
        }
    }
    // in == NULL with inlen == 0
    let mut co = vec![0u8; 64];
    let mut ro = vec![0u8; 64];
    let a = unsafe { c(co.as_mut_ptr() as *mut c_void, 64, std::ptr::null(), 0) };
    let b = unsafe { r(ro.as_mut_ptr() as *mut c_void, 64, std::ptr::null(), 0) };
    assert_eq!(a, b);
    common::eqb("blake2b_long in=NULL", &co, &ro);

    // outlen == 0 must fail (crypto_generichash_blake2b_init rejects outlen 0)
    let mut co = vec![0xAAu8; 8];
    let mut ro = vec![0xAAu8; 8];
    let a = unsafe { c(co.as_mut_ptr() as *mut c_void, 0, b"x".as_ptr() as *const c_void, 1) };
    let b = unsafe { r(ro.as_mut_ptr() as *mut c_void, 0, b"x".as_ptr() as *const c_void, 1) };
    assert_eq!((a, b), (-1, -1), "blake2b_long outlen=0");
    common::eqb("blake2b_long outlen=0", &co, &ro);

    // outlen > UINT32_MAX -> checked before anything is written
    for outlen in [0x1_0000_0000usize, usize::MAX] {
        let mut co = vec![0xAAu8; 8];
        let mut ro = vec![0xAAu8; 8];
        let a = unsafe {
            c(co.as_mut_ptr() as *mut c_void, outlen, b"x".as_ptr() as *const c_void, 1)
        };
        let b = unsafe {
            r(ro.as_mut_ptr() as *mut c_void, outlen, b"x".as_ptr() as *const c_void, 1)
        };
        assert_eq!((a, b), (-1, -1), "blake2b_long outlen={outlen}");
        common::eqb(&format!("blake2b_long outlen={outlen}"), &co, &ro);
    }
}

// ===========================================================================
// scrypt: low-level (_ll and escrypt_kdf_nosse)
// ===========================================================================

#[test]
fn scrypt_ll_and_kdf() {
    let (cll, rll) = both!("crypto_pwhash_scryptsalsa208sha256_ll", FnScryptLL);
    let (ck, rk) = both!("_sodium_escrypt_kdf_nosse", FnKdf);
    let (cinit, rinit) = both!("_sodium_escrypt_init_local", FnRegionInt);
    let (cfree, rfree) = both!("_sodium_escrypt_free_local", FnRegionInt);
    let mut rng = common::Rng::new(0x5C09_5C09);

    // --- accepted parameter grid (small N/r/p so this stays fast)
    let grid: &[(u64, u32, u32)] = &[
        (2, 1, 1),
        (2, 1, 2),
        (2, 8, 1),
        (4, 1, 1),
        (4, 2, 3),
        (8, 1, 1),
        (16, 1, 1),
        (16, 4, 2),
        (32, 2, 1),
        (64, 1, 1),
        (1024, 8, 1),
    ];
    for &(n, r_, p) in grid {
        for &buflen in &[0usize, 1, 16, 31, 32, 33, 64, 100] {
            for case in 0..2 {
                let pw = rng.bytes(case * 17);
                let salt = rng.bytes(1 + case * 13);
                let mut cb = vec![0x8Eu8; buflen + 8];
                let mut rb = vec![0x8Eu8; buflen + 8];
                let (a, ce) = with_errno(|| unsafe {
                    cll(pw.as_ptr(), pw.len(), salt.as_ptr(), salt.len(), n, r_, p, cb.as_mut_ptr(), buflen)
                });
                let (b, re) = with_errno(|| unsafe {
                    rll(pw.as_ptr(), pw.len(), salt.as_ptr(), salt.len(), n, r_, p, rb.as_mut_ptr(), buflen)
                });
                let label = format!("_ll N={n} r={r_} p={p} buflen={buflen} case={case}");
                assert_eq!(a, b, "{label}: rc");
                assert_eq!(ce, re, "{label}: errno");
                assert_eq!(a, 0, "{label}");
                common::eqb(&label, &cb, &rb);

                // same thing straight through escrypt_kdf_nosse with a caller-
                // managed local region (also exercises region reuse: the second
                // call reuses the already-allocated region).
                let mut clocal = EscryptRegion::new();
                let mut rlocal = EscryptRegion::new();
                assert_eq!(unsafe { cinit(&mut clocal) }, 0);
                assert_eq!(unsafe { rinit(&mut rlocal) }, 0);
                let mut cb2 = vec![0x8Eu8; buflen + 8];
                let mut rb2 = vec![0x8Eu8; buflen + 8];
                for round in 0..2 {
                    let (a2, ce2) = with_errno(|| unsafe {
                        ck(
                            &mut clocal,
                            pw.as_ptr(),
                            pw.len(),
                            salt.as_ptr(),
                            salt.len(),
                            n,
                            r_,
                            p,
                            cb2.as_mut_ptr(),
                            buflen,
                        )
                    });
                    let (b2, re2) = with_errno(|| unsafe {
                        ck(
                            &mut rlocal,
                            pw.as_ptr(),
                            pw.len(),
                            salt.as_ptr(),
                            salt.len(),
                            n,
                            r_,
                            p,
                            rb2.as_mut_ptr(),
                            buflen,
                        )
                    });
                    let _ = (b2, re2);
                    let (b3, re3) = with_errno(|| unsafe {
                        rk(
                            &mut rlocal,
                            pw.as_ptr(),
                            pw.len(),
                            salt.as_ptr(),
                            salt.len(),
                            n,
                            r_,
                            p,
                            rb2.as_mut_ptr(),
                            buflen,
                        )
                    });
                    assert_eq!(a2, b3, "{label} kdf round={round}: rc");
                    assert_eq!(ce2, re3, "{label} kdf round={round}: errno");
                    common::eqb(&format!("{label} kdf round={round}"), &cb2, &rb2);
                    common::eqb(&format!("{label} kdf vs _ll"), &cb2, &cb);
                    assert_eq!(clocal.size, rlocal.size, "{label}: region size");
                }
                assert_eq!(unsafe { cfree(&mut clocal) }, 0);
                assert_eq!(unsafe { rfree(&mut rlocal) }, 0);
                assert_eq!(clocal, EscryptRegion::new());
                assert_eq!(rlocal, EscryptRegion::new());
            }
        }
    }

    // --- rejected parameters, exact errno
    let pw = b"pw".to_vec();
    let salt = b"salt".to_vec();
    let rejects: &[(&str, u64, u32, u32, usize, c_int)] = &[
        ("N=0", 0, 8, 1, 32, EINVAL),
        ("N=1", 1, 8, 1, 32, EINVAL),
        ("N=3", 3, 8, 1, 32, EINVAL),
        ("N=5", 5, 8, 1, 32, EINVAL),
        ("N=6", 6, 8, 1, 32, EINVAL),
        ("N=1000", 1000, 8, 1, 32, EINVAL),
        ("N=0xFFFFFFFF", 0xFFFFFFFF, 8, 1, 32, EINVAL),
        ("N=2^32", 0x1_0000_0000, 8, 1, 32, EFBIG),
        ("N=u64::MAX", u64::MAX, 8, 1, 32, EFBIG),
        ("r=0", 16, 0, 1, 32, EINVAL),
        ("p=0", 16, 8, 0, 32, EINVAL),
        ("r=0,p=0", 16, 0, 0, 32, EINVAL),
        // r*p >= 2^30 (checked before r==0/p==0)
        ("r*p=2^30", 16, 1 << 15, 1 << 15, 32, EFBIG),
        ("r*p huge", 16, 0xFFFFFFFF, 0xFFFFFFFF, 32, EFBIG),
        // r == 0 makes r*p == 0, so the r*p check passes and r==0 wins
        ("r=0 p huge", 16, 0, 0xFFFFFFFF, 32, EINVAL),
        ("p=0 r huge", 16, 0xFFFFFFFF, 0, 32, EINVAL),
        // buflen > (2^32 - 1) * 32
        ("buflen too big", 16, 8, 1, 0x20_0000_0000, EFBIG),
        // N > SIZE_MAX / 128 / r  ->  ENOMEM
        ("N/r ENOMEM", 1 << 31, 1 << 27, 1, 32, ENOMEM),
    ];
    for &(label, n, r_, p, buflen, want_errno) in rejects {
        let mut cb = vec![0x5Du8; 64];
        let mut rb = vec![0x5Du8; 64];
        let (a, ce) = with_errno(|| unsafe {
            cll(pw.as_ptr(), pw.len(), salt.as_ptr(), salt.len(), n, r_, p, cb.as_mut_ptr(), buflen)
        });
        let (b, re) = with_errno(|| unsafe {
            rll(pw.as_ptr(), pw.len(), salt.as_ptr(), salt.len(), n, r_, p, rb.as_mut_ptr(), buflen)
        });
        assert_eq!(a, b, "_ll {label}: rc");
        assert_eq!(ce, re, "_ll {label}: errno");
        assert_eq!(a, -1, "_ll {label}");
        assert_eq!(ce, want_errno, "_ll {label}: expected errno");
        common::eqb(&format!("_ll {label}"), &cb, &rb);
    }

    // --- allocation failure: need ~2^63 bytes
    {
        let mut cb = vec![0u8; 64];
        let mut rb = vec![0u8; 64];
        let (a, _) = with_errno(|| unsafe {
            cll(
                pw.as_ptr(),
                pw.len(),
                salt.as_ptr(),
                salt.len(),
                1 << 30,
                1 << 26,
                1,
                cb.as_mut_ptr(),
                32,
            )
        });
        let (b, _) = with_errno(|| unsafe {
            rll(
                pw.as_ptr(),
                pw.len(),
                salt.as_ptr(),
                salt.len(),
                1 << 30,
                1 << 26,
                1,
                rb.as_mut_ptr(),
                32,
            )
        });
        assert_eq!(a, b, "_ll alloc failure: rc");
        assert_eq!(a, -1, "_ll alloc failure");
    }
}

#[test]
fn escrypt_pbkdf2_sha256() {
    let (c, r) = both!("_sodium_escrypt_PBKDF2_SHA256", FnPbkdf2);
    let mut rng = common::Rng::new(0x9BDF_1357);

    for &c_iters in &[1u64, 2, 3, 10] {
        for &dklen in &[0usize, 1, 2, 31, 32, 33, 63, 64, 65, 100, 128] {
            for case in 0..4 {
                let pw = rng.bytes(case * 40);
                let salt = rng.bytes(case * 37 + 1);
                let mut cb = vec![0x1Fu8; dklen + 8];
                let mut rb = vec![0x1Fu8; dklen + 8];
                unsafe {
                    c(pw.as_ptr(), pw.len(), salt.as_ptr(), salt.len(), c_iters, cb.as_mut_ptr(), dklen)
                };
                unsafe {
                    r(pw.as_ptr(), pw.len(), salt.as_ptr(), salt.len(), c_iters, rb.as_mut_ptr(), dklen)
                };
                common::eqb(
                    &format!("PBKDF2_SHA256 c={c_iters} dkLen={dklen} case={case}"),
                    &cb,
                    &rb,
                );
            }
        }
    }
    // c == 0 : the inner `for (j = 2; j <= c; j++)` loop never runs, so this
    // behaves like c == 1.
    let mut cb = vec![0u8; 32];
    let mut rb = vec![0u8; 32];
    unsafe { c(b"p".as_ptr(), 1, b"s".as_ptr(), 1, 0, cb.as_mut_ptr(), 32) };
    unsafe { r(b"p".as_ptr(), 1, b"s".as_ptr(), 1, 0, rb.as_mut_ptr(), 32) };
    common::eqb("PBKDF2_SHA256 c=0", &cb, &rb);
    // NOTE: dkLen > 0x1fffffffe0 calls sodium_misuse() -> abort(); not tested.
}

#[test]
fn escrypt_regions() {
    let (calloc_, ralloc) = both!("_sodium_escrypt_alloc_region", FnAllocRegion);
    let (cfreer, rfreer) = both!("_sodium_escrypt_free_region", FnRegionInt);
    let (cinit, rinit) = both!("_sodium_escrypt_init_local", FnRegionInt);
    let (cfreel, rfreel) = both!("_sodium_escrypt_free_local", FnRegionInt);

    // init_local zeroes the struct and returns 0
    let mut cl = EscryptRegion { base: 1 as *mut c_void, aligned: 2 as *mut c_void, size: 3 };
    let mut rl = cl;
    assert_eq!(unsafe { cinit(&mut cl) }, unsafe { rinit(&mut rl) });
    assert_eq!(cl, EscryptRegion::new());
    assert_eq!(rl, EscryptRegion::new());

    // free_local / free_region on a zeroed region are no-ops returning 0
    assert_eq!(unsafe { cfreel(&mut cl) }, unsafe { rfreel(&mut rl) });
    assert_eq!(unsafe { cfreer(&mut cl) }, unsafe { rfreer(&mut rl) });
    assert_eq!(cl, EscryptRegion::new());
    assert_eq!(rl, EscryptRegion::new());

    // alloc_region: aligned to 64, size recorded, returns `aligned`
    for size in [0usize, 1, 63, 64, 65, 4096, 1 << 20] {
        let mut c1 = EscryptRegion::new();
        let mut r1 = EscryptRegion::new();
        let ca = unsafe { calloc_(&mut c1, size) };
        let ra = unsafe { ralloc(&mut r1, size) };
        assert_eq!(ca.is_null(), ra.is_null(), "alloc_region({size}) nullness");
        assert!(!ca.is_null(), "alloc_region({size}) should succeed");
        assert_eq!(ca, c1.aligned);
        assert_eq!(ra, r1.aligned);
        assert_eq!(c1.size, r1.size, "alloc_region({size}) size");
        assert_eq!(c1.size, size);
        assert_eq!(ca as usize % 64, 0, "C alignment");
        assert_eq!(ra as usize % 64, 0, "Rust alignment");
        assert!(ca as usize >= c1.base as usize && ca as usize <= c1.base as usize + 63);
        assert!(ra as usize >= r1.base as usize && ra as usize <= r1.base as usize + 63);
        assert_eq!(unsafe { cfreer(&mut c1) }, 0);
        assert_eq!(unsafe { rfreer(&mut r1) }, 0);
        assert_eq!(c1, EscryptRegion::new());
        assert_eq!(r1, EscryptRegion::new());
    }

    // alloc_region failure: size + 63 overflows -> base NULL, size 0, ENOMEM
    for size in [usize::MAX, usize::MAX - 62] {
        let mut c1 = EscryptRegion { base: 9 as *mut c_void, aligned: 9 as *mut c_void, size: 9 };
        let mut r1 = c1;
        let (ca, ce) = with_errno(|| unsafe { calloc_(&mut c1, size) });
        let (ra, re) = with_errno(|| unsafe { ralloc(&mut r1, size) });
        assert!(ca.is_null() && ra.is_null(), "alloc_region({size}) must fail");
        assert_eq!(ce, re, "alloc_region({size}) errno");
        assert_eq!(c1, r1, "alloc_region({size}) region state");
        assert_eq!(c1.size, 0);
    }
    // alloc_region failure: huge (but non-overflowing) malloc
    {
        let mut c1 = EscryptRegion::new();
        let mut r1 = EscryptRegion::new();
        let ca = unsafe { calloc_(&mut c1, 1usize << 62) };
        let ra = unsafe { ralloc(&mut r1, 1usize << 62) };
        assert_eq!(ca.is_null(), ra.is_null(), "alloc_region(2^62) nullness");
        assert_eq!(c1.size, r1.size);
        if !ca.is_null() {
            assert_eq!(unsafe { cfreer(&mut c1) }, 0);
            assert_eq!(unsafe { rfreer(&mut r1) }, 0);
        }
    }
}

#[test]
fn escrypt_parse_setting_and_gensalt() {
    let (cp, rp) = both!("_sodium_escrypt_parse_setting", FnParseSetting);
    let (cg, rg) = both!("_sodium_escrypt_gensalt_r", FnGensalt);
    let mut rng = common::Rng::new(0x7777_3333);

    // --- gensalt_r
    for &srclen in &[0usize, 1, 2, 3, 4, 31, 32, 33, 48] {
        let src = rng.bytes(srclen.max(1));
        let saltlen = (srclen * 8 + 5) / 6;
        let need = 14 + saltlen + 1;
        for &(n_log2, r_, p) in &[
            (0u32, 1u32, 1u32),
            (1, 8, 1),
            (10, 8, 1),
            (14, 8, 1),
            (63, 1, 1),
            (64, 8, 1),           // N_log2 > 63 -> NULL
            (255, 8, 1),          // N_log2 > 63 -> NULL
            (10, 1 << 15, 1 << 15), // r*p >= 2^30 -> NULL
            (10, 0, 0),
            (10, 0x3FFFFFFF, 1),
        ] {
            for &buflen in &[0usize, 1, need.saturating_sub(1), need, need + 1, 128] {
                let mut cb = vec![0x2Bu8; 256];
                let mut rb = vec![0x2Bu8; 256];
                let ca = unsafe { cg(n_log2, r_, p, src.as_ptr(), srclen, cb.as_mut_ptr(), buflen) };
                let ra = unsafe { rg(n_log2, r_, p, src.as_ptr(), srclen, rb.as_mut_ptr(), buflen) };
                let label =
                    format!("gensalt_r n={n_log2} r={r_} p={p} srclen={srclen} buflen={buflen}");
                assert_eq!(ca.is_null(), ra.is_null(), "{label}: nullness");
                if !ca.is_null() {
                    assert_eq!(ca, cb.as_mut_ptr(), "{label}: returns buf");
                    assert_eq!(ra, rb.as_mut_ptr(), "{label}: returns buf");
                }
                common::eqb(&label, &cb, &rb);
            }
        }
    }

    // --- parse_setting on gensalt_r output and on malformed settings
    let mut settings: Vec<Vec<u8>> = Vec::new();
    for &(n_log2, r_, p) in &[(0u32, 1u32, 1u32), (1, 8, 1), (10, 8, 1), (14, 8, 1), (63, 8, 1)] {
        let src = [0x11u8; 32];
        let mut buf = vec![0u8; 128];
        let ok = unsafe { cg(n_log2, r_, p, src.as_ptr(), 32, buf.as_mut_ptr(), 128) };
        assert!(!ok.is_null());
        let z = buf.iter().position(|&b| b == 0).unwrap();
        settings.push(buf[..z + 1].to_vec());
    }
    // NOTE: decode64_one(0) SUCCEEDS in the C, because strchr(itoa64, 0) finds
    // the terminating NUL of itoa64 and yields index 64. A setting that is
    // truncated inside the N_log2/r/p fields therefore makes the C read past
    // the NUL. The truncated cases below are padded with defined bytes so that
    // this (identical) over-read stays inside the allocation.
    settings.push(b"\0".to_vec());
    settings.push(b"$\0".to_vec());
    settings.push(b"$7\0".to_vec());
    settings.push(b"$7$\0AAAAAAAAAAAAAAAAAAAA".to_vec());
    settings.push(b"$8$C6..../....\0".to_vec());
    settings.push(b"7$C6..../....\0".to_vec());
    settings.push(b"$7$!6..../....\0".to_vec());       // invalid N_log2 char
    settings.push(b"$7$C!..../....\0".to_vec());       // invalid r char
    settings.push(b"$7$C6....!....\0".to_vec());       // invalid p char
    settings.push(b"$7$C6..../....salt$hash\0".to_vec());
    settings.push(b"$7$A\0AAAAAAAAAAAAAAAAAAAA".to_vec()); // truncated r field
    settings.push(b"$7$C6...\0AAAAAAAAAAAAAAAA".to_vec()); // truncated p field

    for s in &settings {
        let (mut cn, mut cr, mut cpp) = (0xDEADBEEFu32, 0xDEADBEEFu32, 0xDEADBEEFu32);
        let (mut rn, mut rr, mut rpp) = (0xDEADBEEFu32, 0xDEADBEEFu32, 0xDEADBEEFu32);
        let ca = unsafe { cp(s.as_ptr(), &mut cn, &mut cr, &mut cpp) };
        let ra = unsafe { rp(s.as_ptr(), &mut rn, &mut rr, &mut rpp) };
        let label = format!("parse_setting {:?}", String::from_utf8_lossy(s));
        assert_eq!(ca.is_null(), ra.is_null(), "{label}: nullness");
        if !ca.is_null() {
            assert_eq!(
                ca as usize - s.as_ptr() as usize,
                ra as usize - s.as_ptr() as usize,
                "{label}: offset"
            );
        }
        assert_eq!((cn, cr, cpp), (rn, rr, rpp), "{label}: out params");
    }
}

#[test]
fn escrypt_r_direct() {
    let _rng_lock = rng_guard();
    let (cr_, rr_) = both!("_sodium_escrypt_r", FnEscryptR);
    let (cg, rg) = both!("_sodium_escrypt_gensalt_r", FnGensalt);
    let (cinit, rinit) = both!("_sodium_escrypt_init_local", FnRegionInt);
    let (cfree, rfree) = both!("_sodium_escrypt_free_local", FnRegionInt);
    install_det_rng();
    let mut rng = common::Rng::new(0xE5C0_1234);

    // A 32-byte salt encodes to 43 chars, so
    //   need = 14 (prefix) + 43 (salt) + 1 ($) + 43 (hash) + 1 (NUL) = 102
    for &(n_log2, r_, p) in &[(1u32, 1u32, 1u32), (1, 8, 1), (4, 2, 2), (6, 1, 1), (10, 8, 1)] {
        let src = rng.bytes(SC_SALTBYTES);
        let mut setting = vec![0u8; SC_STRSETTINGBYTES + 1];
        let ok = unsafe {
            cg(n_log2, r_, p, src.as_ptr(), SC_SALTBYTES, setting.as_mut_ptr(), setting.len())
        };
        assert!(!ok.is_null());
        // both libraries must produce the same setting string
        let mut setting_r = vec![0u8; SC_STRSETTINGBYTES + 1];
        let ok2 = unsafe {
            rg(n_log2, r_, p, src.as_ptr(), SC_SALTBYTES, setting_r.as_mut_ptr(), setting_r.len())
        };
        assert!(!ok2.is_null());
        common::eqb("gensalt agreement", &setting, &setting_r);

        for &pwlen in &[0usize, 1, 32] {
            let pw = rng.bytes(pwlen.max(1));
            for &buflen in &[SC_STRBYTES, SC_STRBYTES + 1, 200] {
                let mut cb = vec![0x3Bu8; 256];
                let mut rb = vec![0x3Bu8; 256];
                let mut cl = EscryptRegion::new();
                let mut rl = EscryptRegion::new();
                assert_eq!(unsafe { cinit(&mut cl) }, 0);
                assert_eq!(unsafe { rinit(&mut rl) }, 0);
                reset_rng(0x4242);
                let ca = unsafe {
                    cr_(&mut cl, pw.as_ptr(), pwlen, setting.as_ptr(), cb.as_mut_ptr(), buflen)
                };
                RSEED.store(0x4242, Ordering::SeqCst);
                let ra = unsafe {
                    rr_(&mut rl, pw.as_ptr(), pwlen, setting.as_ptr(), rb.as_mut_ptr(), buflen)
                };
                let label = format!("escrypt_r n={n_log2} r={r_} p={p} pwlen={pwlen} buflen={buflen}");
                assert_eq!(ca.is_null(), ra.is_null(), "{label}: nullness");
                assert!(!ca.is_null(), "{label} should succeed");
                common::eqb(&label, &cb, &rb);
                assert_eq!(unsafe { cfree(&mut cl) }, 0);
                assert_eq!(unsafe { rfree(&mut rl) }, 0);
            }
        }

        // buflen too small -> NULL
        for &buflen in &[0usize, 1, 50, SC_STRBYTES - 1] {
            let mut cb = vec![0x3Bu8; 256];
            let mut rb = vec![0x3Bu8; 256];
            let mut cl = EscryptRegion::new();
            let mut rl = EscryptRegion::new();
            unsafe { cinit(&mut cl) };
            unsafe { rinit(&mut rl) };
            reset_rng(0x99);
            let ca = unsafe { cr_(&mut cl, b"p".as_ptr(), 1, setting.as_ptr(), cb.as_mut_ptr(), buflen) };
            RSEED.store(0x99, Ordering::SeqCst);
            let ra = unsafe { rr_(&mut rl, b"p".as_ptr(), 1, setting.as_ptr(), rb.as_mut_ptr(), buflen) };
            assert!(ca.is_null() && ra.is_null(), "escrypt_r buflen={buflen} must fail");
            common::eqb(&format!("escrypt_r buflen={buflen}"), &cb, &rb);
            unsafe { cfree(&mut cl) };
            unsafe { rfree(&mut rl) };
        }
    }

    // invalid setting -> NULL (before any randombytes/kdf work)
    for bad in [
        b"\0".to_vec(),
        b"$8$C6..../....\0".to_vec(),
        // padded: see the note in escrypt_parse_setting_and_gensalt
        b"$7$\0AAAAAAAAAAAAAAAAAAAA".to_vec(),
        b"$7$!6..../....\0".to_vec(),
    ] {
        let mut cb = vec![0x3Bu8; 256];
        let mut rb = vec![0x3Bu8; 256];
        let mut cl = EscryptRegion::new();
        let mut rl = EscryptRegion::new();
        unsafe { cinit(&mut cl) };
        unsafe { rinit(&mut rl) };
        reset_rng(0x77);
        let ca = unsafe { cr_(&mut cl, b"p".as_ptr(), 1, bad.as_ptr(), cb.as_mut_ptr(), SC_STRBYTES) };
        RSEED.store(0x77, Ordering::SeqCst);
        let ra = unsafe { rr_(&mut rl, b"p".as_ptr(), 1, bad.as_ptr(), rb.as_mut_ptr(), SC_STRBYTES) };
        assert!(ca.is_null() && ra.is_null());
        common::eqb("escrypt_r bad setting", &cb, &rb);
        unsafe { cfree(&mut cl) };
        unsafe { rfree(&mut rl) };
    }

    // setting parses fine but the kdf rejects the parameters
    // (N_log2 = 0 -> N = 1 < 2; r = 0; p = 0) -> escrypt_r returns NULL
    for (n_log2, r_, p) in [(0u32, 1u32, 1u32), (1, 0, 1), (1, 1, 0), (0, 0, 0)] {
        let src = [2u8; SC_SALTBYTES];
        let mut setting = vec![0u8; SC_STRSETTINGBYTES + 1];
        let g = unsafe {
            cg(n_log2, r_, p, src.as_ptr(), SC_SALTBYTES, setting.as_mut_ptr(), setting.len())
        };
        assert!(!g.is_null(), "gensalt_r({n_log2},{r_},{p})");
        let mut cb = vec![0x3Bu8; 256];
        let mut rb = vec![0x3Bu8; 256];
        let mut cl = EscryptRegion::new();
        let mut rl = EscryptRegion::new();
        unsafe { cinit(&mut cl) };
        unsafe { rinit(&mut rl) };
        reset_rng(0x31);
        let ca =
            unsafe { cr_(&mut cl, b"p".as_ptr(), 1, setting.as_ptr(), cb.as_mut_ptr(), SC_STRBYTES) };
        RSEED.store(0x31, Ordering::SeqCst);
        let ra =
            unsafe { rr_(&mut rl, b"p".as_ptr(), 1, setting.as_ptr(), rb.as_mut_ptr(), SC_STRBYTES) };
        assert!(ca.is_null() && ra.is_null(), "escrypt_r kdf-reject n={n_log2} r={r_} p={p}");
        common::eqb(&format!("escrypt_r kdf-reject n={n_log2} r={r_} p={p}"), &cb, &rb);
        unsafe { cfree(&mut cl) };
        unsafe { rfree(&mut rl) };
    }

    // buf == NULL -> NULL, no randombytes consumed
    {
        let mut cl = EscryptRegion::new();
        let mut rl = EscryptRegion::new();
        unsafe { cinit(&mut cl) };
        unsafe { rinit(&mut rl) };
        let src = [1u8; SC_SALTBYTES];
        let mut setting = vec![0u8; SC_STRSETTINGBYTES + 1];
        unsafe { cg(1, 1, 1, src.as_ptr(), SC_SALTBYTES, setting.as_mut_ptr(), setting.len()) };
        reset_rng(0x55);
        let ca = unsafe { cr_(&mut cl, b"p".as_ptr(), 1, setting.as_ptr(), std::ptr::null_mut(), 0) };
        let c_after = CSEED.load(Ordering::SeqCst);
        RSEED.store(0x55, Ordering::SeqCst);
        let ra = unsafe { rr_(&mut rl, b"p".as_ptr(), 1, setting.as_ptr(), std::ptr::null_mut(), 0) };
        assert!(ca.is_null() && ra.is_null());
        assert_eq!(c_after, RSEED.load(Ordering::SeqCst));
        unsafe { cfree(&mut cl) };
        unsafe { rfree(&mut rl) };
    }
}

// ===========================================================================
// scrypt: high level
// ===========================================================================

#[test]
fn scrypt_highlevel() {
    let (c, r) = both!("crypto_pwhash_scryptsalsa208sha256", FnScrypt);
    let mut rng = common::Rng::new(0x5C5C_5C5C);
    let mut salt = [0u8; SC_SALTBYTES];
    rng.fill(&mut salt);

    // pickparams keeps the work tiny for small opslimit/memlimit; there are no
    // MIN checks on opslimit/memlimit in this entry point.
    let params: &[(u64, usize)] = &[
        (0, 0),
        (1, 0),
        (32768, 0),
        (32768, 1024),
        (32768, 16777216),
        (65536, 1048576),
        (524288, 16777216),
    ];
    for &(ops, mem) in params {
        for &outlen in &[16u64, 17, 32, 64, 100] {
            for case in 0..3 {
                let pw = rng.bytes(case * 20);
                let mut cb = vec![0xE1u8; outlen as usize + 8];
                let mut rb = vec![0xE1u8; outlen as usize + 8];
                let (a, ce) = with_errno(|| unsafe {
                    c(
                        cb.as_mut_ptr(),
                        outlen,
                        pw.as_ptr() as *const c_char,
                        pw.len() as u64,
                        salt.as_ptr(),
                        ops,
                        mem,
                    )
                });
                let (b, re) = with_errno(|| unsafe {
                    r(
                        rb.as_mut_ptr(),
                        outlen,
                        pw.as_ptr() as *const c_char,
                        pw.len() as u64,
                        salt.as_ptr(),
                        ops,
                        mem,
                    )
                });
                let label = format!("scrypt ops={ops} mem={mem} outlen={outlen} case={case}");
                assert_eq!(a, b, "{label}: rc");
                assert_eq!(ce, re, "{label}: errno");
                assert_eq!(a, 0, "{label}");
                common::eqb(&label, &cb, &rb);
            }
        }
    }

    // outlen < BYTES_MIN -> EINVAL
    for outlen in [0u64, 1, 15] {
        let mut cb = vec![0xE1u8; 32];
        let mut rb = vec![0xE1u8; 32];
        let (a, ce) = with_errno(|| unsafe {
            c(cb.as_mut_ptr(), outlen, b"p".as_ptr() as *const c_char, 1, salt.as_ptr(), 32768, 1024)
        });
        let (b, re) = with_errno(|| unsafe {
            r(rb.as_mut_ptr(), outlen, b"p".as_ptr() as *const c_char, 1, salt.as_ptr(), 32768, 1024)
        });
        assert_eq!((a, ce), (b, re), "scrypt outlen={outlen}");
        assert_eq!(a, -1);
        assert_eq!(ce, EINVAL);
        common::eqb(&format!("scrypt outlen={outlen}"), &cb, &rb);
    }

    // out == passwd -> EINVAL
    {
        let mut cb = vec![0u8; 64];
        let mut rb = vec![0u8; 64];
        let (a, ce) = with_errno(|| unsafe {
            c(cb.as_mut_ptr(), 32, cb.as_ptr() as *const c_char, 8, salt.as_ptr(), 32768, 1024)
        });
        let (b, re) = with_errno(|| unsafe {
            r(rb.as_mut_ptr(), 32, rb.as_ptr() as *const c_char, 8, salt.as_ptr(), 32768, 1024)
        });
        assert_eq!((a, ce), (b, re));
        assert_eq!(a, -1);
        assert_eq!(ce, EINVAL);
        common::eqb("scrypt out==passwd", &cb, &rb);
    }
}

#[test]
fn scrypt_str_roundtrip() {
    let _rng_lock = rng_guard();
    install_det_rng();
    let (cs, rs) = both!("crypto_pwhash_scryptsalsa208sha256_str", FnPwhashStr);
    let (cv, rv) = both!("crypto_pwhash_scryptsalsa208sha256_str_verify", FnStrVerify);
    let (cn, rn) = both!("crypto_pwhash_scryptsalsa208sha256_str_needs_rehash", FnNeedsRehash);
    let mut rng = common::Rng::new(0x5C57_5C57);

    for &(ops, mem) in &[(0u64, 0usize), (32768, 1024), (32768, 16777216), (65536, 1048576)] {
        for &pwlen in &[0usize, 1, 32] {
            for case in 0..3u64 {
                let pw = rng.bytes(pwlen.max(1));
                let (rc, s) = cmp_str(
                    &format!("scrypt_str ops={ops} mem={mem} pwlen={pwlen} case={case}"),
                    cs,
                    rs,
                    SC_STRBYTES,
                    &pw,
                    pwlen as u64,
                    ops,
                    mem,
                    0xBEEF_0000 + case * 31 + pwlen as u64,
                );
                assert_eq!(rc, 0);
                assert_eq!(&s[..3], b"$7$");
                assert_eq!(s[SC_STRBYTES - 1], 0, "must be NUL-terminated");

                let a = unsafe {
                    cv(s.as_ptr() as *const c_char, pw.as_ptr() as *const c_char, pwlen as u64)
                };
                let b = unsafe {
                    rv(s.as_ptr() as *const c_char, pw.as_ptr() as *const c_char, pwlen as u64)
                };
                assert_eq!((a, b), (0, 0), "scrypt verify");

                // wrong password
                let bad = rng.bytes(pwlen + 1);
                let a = unsafe {
                    cv(s.as_ptr() as *const c_char, bad.as_ptr() as *const c_char, bad.len() as u64)
                };
                let b = unsafe {
                    rv(s.as_ptr() as *const c_char, bad.as_ptr() as *const c_char, bad.len() as u64)
                };
                assert_eq!(a, b, "scrypt verify wrong pw");
                assert_eq!(a, -1);

                // needs_rehash: same params -> 0
                let (a, ae) = with_errno(|| unsafe { cn(s.as_ptr() as *const c_char, ops, mem) });
                let (b, be) = with_errno(|| unsafe { rn(s.as_ptr() as *const c_char, ops, mem) });
                assert_eq!((a, ae), (b, be), "scrypt needs_rehash same");
                assert_eq!(a, 0);
                // different params -> 1 (or 0 if pickparams happens to agree)
                for (o2, m2) in [(524288u64, 16777216usize), (32768, 1024), (0, 0)] {
                    let (a, ae) = with_errno(|| unsafe { cn(s.as_ptr() as *const c_char, o2, m2) });
                    let (b, be) = with_errno(|| unsafe { rn(s.as_ptr() as *const c_char, o2, m2) });
                    assert_eq!((a, ae), (b, be), "scrypt needs_rehash ops={o2} mem={m2}");
                }
            }
        }
    }

    // --- malformed strings for _str_verify / _str_needs_rehash
    let mut cases: Vec<(String, Vec<u8>)> = Vec::new();
    cases.push(("empty".into(), b"\0".to_vec()));
    cases.push(("short".into(), b"$7$C6..../....\0".to_vec()));
    // exactly STRBYTES-1 chars but garbage
    cases.push(("garbage 101".into(), {
        let mut v = vec![b'x'; SC_STRBYTES - 1];
        v.push(0);
        v
    }));
    // right length, right prefix, but invalid base64 in the setting
    cases.push(("bad setting 101".into(), {
        let mut v = b"$7$".to_vec();
        v.extend(std::iter::repeat(b'!').take(SC_STRBYTES - 1 - 3));
        v.push(0);
        v
    }));
    // correct length and prefix, but N_log2 = 0 (`.`) so escrypt_r's kdf call
    // fails (N == 1 < 2) -> escrypt_r returns NULL -> -1
    {
        let (_, mut v) =
            cmp_str("mk scrypt for N=0", cs, rs, SC_STRBYTES, b"pw", 2, 32768, 1024, 0x1234);
        v[3] = b'.';
        cases.push(("N_log2 = 0".into(), v));
    }
    // non-NUL-terminated buffer of exactly STRBYTES bytes: sodium_strnlen()
    // returns STRBYTES (!= STRBYTES-1) and the C returns -1 without reading past.
    cases.push(("no NUL within STRBYTES".into(), vec![b'A'; SC_STRBYTES]));
    // one byte too long
    cases.push(("102 chars + NUL".into(), {
        let mut v = vec![b'A'; SC_STRBYTES];
        v.push(0);
        v
    }));

    for (label, buf) in &cases {
        let (a, ae) = with_errno(|| unsafe {
            cv(buf.as_ptr() as *const c_char, b"p".as_ptr() as *const c_char, 1)
        });
        let (b, be) = with_errno(|| unsafe {
            rv(buf.as_ptr() as *const c_char, b"p".as_ptr() as *const c_char, 1)
        });
        assert_eq!((a, ae), (b, be), "scrypt_str_verify({label})");
        assert_eq!(a, -1, "scrypt_str_verify({label})");

        let (a, ae) = with_errno(|| unsafe { cn(buf.as_ptr() as *const c_char, 32768, 1024) });
        let (b, be) = with_errno(|| unsafe { rn(buf.as_ptr() as *const c_char, 32768, 1024) });
        assert_eq!((a, ae), (b, be), "scrypt_str_needs_rehash({label})");
        // "N_log2 = 0" still parses as a valid *setting*, so needs_rehash only
        // reports "parameters differ" (1); every other case is a hard -1.
        let want = if label == "N_log2 = 0" { 1 } else { -1 };
        assert_eq!(a, want, "scrypt_str_needs_rehash({label})");
    }

    // passwdlen > PASSWD_MAX is impossible (PASSWD_MAX == SIZE_MAX), so the
    // EFBIG branch of *_str is unreachable on 64-bit; see errors table.
}
