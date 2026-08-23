//! t10_pwhash.rs — C-vs-Rust differential verification of `crypto_pwhash`
//! (argon2i / argon2id / dispatch) and `crypto_pwhash_scryptsalsa208sha256`.
//!
//! CONFIGS.md rows 245–264 and ERRORS.md rows 258–374 are the specification.
//! Every call goes through `dlsym` on BOTH shared objects; no Rust function is
//! ever called directly. Every output buffer is prefilled with 0xAA and carries
//! a 32-byte trailing guard; the FULL buffer (body + guard) is compared and the
//! guard is asserted intact.
//!
//! The C `.so` exports the argon2 / escrypt internals under a `_sodium_` prefix,
//! so most of the "not directly exported" rows in ERRORS.md are in fact driven
//! DIRECTLY and asserted against their exact `ARGON2_*` numeric code:
//!   `_sodium_argon2_validate_inputs` `_sodium_argon2_ctx` `_sodium_argon2_hash`
//!   `_sodium_argon2_verify` `_sodium_argon2_initialize` `_sodium_argon2_finalize`
//!   `_sodium_argon2_encode_string` `_sodium_argon2_decode_string`
//!   `_sodium_argon2{i,id}_hash_raw` `_sodium_argon2{i,id}_hash_encoded`
//!   `_sodium_argon2{i,id}_verify` `_sodium_blake2b_long`
//!   `_sodium_escrypt_parse_setting` `_sodium_escrypt_gensalt_r` `_sodium_escrypt_r`
//!   `_sodium_escrypt_PBKDF2_SHA256` `_sodium_escrypt_kdf_nosse`
//!   `_sodium_escrypt_{alloc,free}_region` `_sodium_escrypt_{init,free}_local`
//! `argon2_error_message` is NOT exported by either library.
//!
//! CONFIGS row → test mapping
//! --------------------------
//! * 245 `crypto_pwhash` alg=ARGON2I13 ......... `cfg245_pwhash_argon2i13`
//! * 246 `crypto_pwhash` alg=ARGON2ID13 ........ `cfg246_pwhash_argon2id13`
//! * 247 `crypto_pwhash_argon2i` primitive ..... `cfg247_argon2i_primitive`,
//!                                               `cfg247_argon2i_mcost_rounding`
//! * 248 `crypto_pwhash_argon2id` primitive .... `cfg248_argon2id_primitive`
//! * 249 `outlen` 65/128 → blake2b_long chunk .. `cfg249_blake2b_long_chunked`,
//!                                               `cfg249_blake2b_long_direct`
//! * 250 argon2i `_str` / `_str_verify` ........ `cfg250_argon2i_str_roundtrip`
//! * 251 argon2id `_str` / `_str_verify` ....... `cfg251_argon2id_str_roundtrip`
//! * 252 `_str` / `_str_alg` / `_str_verify` ... `cfg252_str_alg_dispatch`
//! * 253 `_str_needs_rehash` ................... `cfg253_needs_rehash`
//! * 254 encode/decode round-trip ............. `cfg254_encode_decode_roundtrip`
//! * 255 argon2 constant getters .............. `cfg255_argon2_constant_getters`
//! * 256 `_ll` N×r×buflen×saltlen (p=1) ....... `cfg256_ll_matrix_p1`
//! * 257 `_ll` p ∈ {2,3,4} .................... `cfg257_ll_matrix_multi_p`
//! * 258 scrypt `pickparams` branch A ......... `cfg258_scrypt_branch_a`
//! * 259 scrypt `pickparams` branch B ......... `cfg259_scrypt_branch_b`
//! * 260 scrypt opslimit clamped up ........... `cfg260_scrypt_opslimit_clamped`
//! * 261 scrypt `_str` / `_str_verify` ........ `cfg261_scrypt_str_roundtrip`
//! * 262 scrypt `_str_needs_rehash` ........... `cfg262_scrypt_needs_rehash`
//! * 263 `escrypt_parse_setting`/`_gensalt_r` .. `cfg263_escrypt_itoa64_and_salt_end`
//! * 264 scrypt constant getters .............. `cfg264_scrypt_constant_getters`
//!
//! ERRORS row → test mapping
//! -------------------------
//! * 258–279 `argon2_validate_inputs` ......... `e258_279_validate_inputs`
//! * 280 `argon2_ctx` bad type ................ `e280_argon2_ctx_bad_type`
//! * 281 `argon2_initialize` NULL args ........ `e281_283_initialize`
//! * 282 `allocate_memory` m_cost == 0 ........ `e281_283_initialize`
//! * 283 `allocate_memory` mmap/malloc fail ... `e281_283_initialize`
//! * 284–287 `argon2_hash` bounds/encoding .... `e284_287_argon2_hash`
//! * 288 `argon2_verify` strlen > UINT32_MAX .. `e288_289_argon2_verify` (documented
//!                                               unreachable: needs a 4 GiB string)
//! * 289 `argon2_verify` mismatch ............. `e288_289_argon2_verify`
//! * 290–309 `argon2_decode_string` ........... `e290_309_decode_string`,
//!                                               `e290_309_via_str_verify`
//! * 310–312 `argon2_encode_string` ........... `e310_312_encode_string`
//! * 313 `SB()` too-small dst ................. `e313_encode_string_sb_misuse` (it is a
//!                                               `sodium_misuse()` abort, not -31)
//! * 314–315 `blake2b_long` ................... `e314_315_blake2b_long`
//! * 316/326 `outlen > BYTES_MAX` ............. `e316_e326_outlen_above_bytes_max`
//! * 317–325 `crypto_pwhash_argon2i` .......... `e317_325_argon2i_public`
//! * 326–332 `crypto_pwhash_argon2id` ......... `e326_332_argon2id_public`
//! * 333–338 `_str` / `_str_verify` ........... `e333_338_str_and_str_verify`
//! * 339–343 `_needs_rehash` .................. `e339_343_needs_rehash`
//! * 344/346/347 dispatch ..................... `e344_346_347_dispatch`
//! * 345 `_str_alg` bad alg → misuse .......... `e345_str_alg_misuse`
//! * 348–351 scrypt one-shot .................. `e348_351_scrypt_public`
//! * 352–359 scrypt `_str*` ................... `e352_359_scrypt_str`
//! * 360–367 `_ll` bounds ..................... `e360_367_ll_bounds`
//! * 368–373 escrypt internals ................ `e368_373_escrypt_internals`
//! * 374 `escrypt_PBKDF2_SHA256` misuse ....... `e374_pbkdf2_misuse`

mod common;
use common::*;
use libc::{c_char, c_int, c_void};

// ---------------------------------------------------------------- fn types

type IntFn0 = unsafe extern "C" fn() -> c_int;
type SizeFn0 = unsafe extern "C" fn() -> usize;
type U64Fn0 = unsafe extern "C" fn() -> u64;
type CharPFn0 = unsafe extern "C" fn() -> *const c_char;
type SetImplFn = unsafe extern "C" fn(*const RandombytesImpl) -> c_int;

/// `crypto_pwhash(out, outlen, passwd, passwdlen, salt, opslimit, memlimit, alg)`
/// and `crypto_pwhash_argon2i` / `crypto_pwhash_argon2id`.
type PwhashFn =
    unsafe extern "C" fn(*mut u8, u64, *const c_char, u64, *const u8, u64, usize, c_int) -> c_int;
/// `crypto_pwhash_scryptsalsa208sha256(out, outlen, passwd, passwdlen, salt, ops, mem)`
type ScryptFn = unsafe extern "C" fn(*mut u8, u64, *const c_char, u64, *const u8, u64, usize) -> c_int;
/// `*_str(out, passwd, passwdlen, opslimit, memlimit)`
type StrFn = unsafe extern "C" fn(*mut c_char, *const c_char, u64, u64, usize) -> c_int;
/// `crypto_pwhash_str_alg(out, passwd, passwdlen, opslimit, memlimit, alg)`
type StrAlgFn = unsafe extern "C" fn(*mut c_char, *const c_char, u64, u64, usize, c_int) -> c_int;
/// `*_str_verify(str, passwd, passwdlen)`
type StrVerifyFn = unsafe extern "C" fn(*const c_char, *const c_char, u64) -> c_int;
/// `*_str_needs_rehash(str, opslimit, memlimit)`
type RehashFn = unsafe extern "C" fn(*const c_char, u64, usize) -> c_int;
/// `crypto_pwhash_scryptsalsa208sha256_ll(passwd, plen, salt, slen, N, r, p, buf, buflen)`
type LlFn =
    unsafe extern "C" fn(*const u8, usize, *const u8, usize, u64, u32, u32, *mut u8, usize) -> c_int;

/// `argon2_validate_inputs(const argon2_context *)`
type ValidateFn = unsafe extern "C" fn(*const Ctx) -> c_int;
/// `argon2_ctx(argon2_context *, argon2_type)`
type Argon2CtxFn = unsafe extern "C" fn(*mut Ctx, c_int) -> c_int;
/// `argon2_initialize(argon2_instance_t *, argon2_context *)`
type InitializeFn = unsafe extern "C" fn(*mut Inst, *mut Ctx) -> c_int;
/// `argon2_hash(t,m,par,pwd,plen,salt,slen,hash,hlen,enc,enclen,type)`
type Argon2HashFn = unsafe extern "C" fn(
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
/// `argon2{i,id}_hash_raw(t,m,par,pwd,plen,salt,slen,hash,hlen)`
type HashRawFn = unsafe extern "C" fn(
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
/// `argon2{i,id}_hash_encoded(t,m,par,pwd,plen,salt,slen,hlen,enc,enclen)`
type HashEncFn = unsafe extern "C" fn(
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
/// `argon2_verify(encoded, pwd, pwdlen, type)`
type Argon2VerifyFn = unsafe extern "C" fn(*const c_char, *const c_void, usize, c_int) -> c_int;
/// `argon2{i,id}_verify(encoded, pwd, pwdlen)`
type Argon2VerifyTFn = unsafe extern "C" fn(*const c_char, *const c_void, usize) -> c_int;
/// `argon2_encode_string(dst, dst_len, ctx, type)`
type EncodeFn = unsafe extern "C" fn(*mut c_char, usize, *mut Ctx, c_int) -> c_int;
/// `argon2_decode_string(ctx, str, type)`
type DecodeFn = unsafe extern "C" fn(*mut Ctx, *const c_char, c_int) -> c_int;
/// `blake2b_long(out, outlen, in, inlen)`
type Blake2bLongFn = unsafe extern "C" fn(*mut c_void, usize, *const c_void, usize) -> c_int;

/// `escrypt_parse_setting(setting, &N_log2, &r, &p)`
type ParseSettingFn = unsafe extern "C" fn(*const u8, *mut u32, *mut u32, *mut u32) -> *const u8;
/// `escrypt_gensalt_r(N_log2, r, p, src, srclen, buf, buflen)`
type GensaltFn =
    unsafe extern "C" fn(u32, u32, u32, *const u8, usize, *mut u8, usize) -> *mut u8;
/// `escrypt_r(local, passwd, passwdlen, setting, buf, buflen)`
type EscryptRFn =
    unsafe extern "C" fn(*mut Region, *const u8, usize, *const u8, *mut u8, usize) -> *mut u8;
/// `escrypt_PBKDF2_SHA256(passwd, plen, salt, slen, c, buf, dkLen)`
type Pbkdf2Fn = unsafe extern "C" fn(*const u8, usize, *const u8, usize, u64, *mut u8, usize);
/// `escrypt_alloc_region(region, size)`
type AllocRegionFn = unsafe extern "C" fn(*mut Region, usize) -> *mut c_void;
/// `escrypt_init_local` / `escrypt_free_local` / `escrypt_free_region`
type RegionFn = unsafe extern "C" fn(*mut Region) -> c_int;

// ------------------------------------------------------------------ structs

/// `argon2_context` — field order verbatim from
/// `c_src/libsodium/crypto_pwhash/argon2/argon2.h`.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct Ctx {
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

/// `argon2_instance_t` — verbatim from `argon2-core.h`.
#[repr(C)]
#[derive(Clone, Copy)]
struct Inst {
    region: *mut c_void,
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

/// `escrypt_region_t` / `escrypt_local_t` — verbatim from `crypto_scrypt.h`.
#[repr(C)]
#[derive(Clone, Copy)]
struct Region {
    base: *mut c_void,
    aligned: *mut c_void,
    size: usize,
}

impl Region {
    fn zero() -> Self {
        Region { base: std::ptr::null_mut(), aligned: std::ptr::null_mut(), size: 0 }
    }
}

// ------------------------------------------------------ argon2 numeric codes

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
const ARGON2_MEMORY_ALLOCATION_ERROR: c_int = -22;
const ARGON2_INCORRECT_PARAMETER: c_int = -25;
const ARGON2_INCORRECT_TYPE: c_int = -26;
const ARGON2_THREADS_TOO_FEW: c_int = -28;
const ARGON2_THREADS_TOO_MANY: c_int = -29;
const ARGON2_ENCODING_FAIL: c_int = -31;
const ARGON2_DECODING_FAIL: c_int = -32;
const ARGON2_VERIFY_MISMATCH: c_int = -35;

const ARGON2_I: c_int = 1;
const ARGON2_ID: c_int = 2;

// ---------------------------------------------------------- misc constants

const ARGON2_SALTBYTES: usize = 16;
const ARGON2_STRBYTES: usize = 128;
const SCRYPT_SALTBYTES: usize = 32;
const SCRYPT_STRBYTES: usize = 102;

/// argon2i, password "password", salt 01..10, m_cost 8, t_cost 3, lanes 1.
const V_I: &str =
    "$argon2i$v=19$m=8,t=3,p=1$AQIDBAUGBwgJCgsMDQ4PEA$ITIZ35frlobU+nY8/61/sbfTj7yOIaa40ZZPYeKGn5Q";
/// argon2id, password "password", salt 01..10, m_cost 8, t_cost 1, lanes 1.
const V_ID: &str =
    "$argon2id$v=19$m=8,t=1,p=1$AQIDBAUGBwgJCgsMDQ4PEA$C2fnILzcT+o9I0ezc1Uao3WSxjd5obVwP6tbm4pZSwI";

// ------------------------------------------------------------------ helpers

const GUARD: usize = 32;
/// Value errno is set to before every call so "errno untouched" is observable.
const ESENT: c_int = 0x5A5A;

fn errno_set(v: c_int) {
    unsafe { *libc::__errno_location() = v };
}
fn errno_get() -> c_int {
    unsafe { *libc::__errno_location() }
}

/// An output buffer prefilled with 0xAA plus a 32-byte trailing guard.
struct OutBuf {
    v: Vec<u8>,
    n: usize,
}

impl OutBuf {
    fn new(n: usize) -> Self {
        OutBuf { v: vec![0xAAu8; n + GUARD], n }
    }
    fn ptr(&mut self) -> *mut u8 {
        self.v.as_mut_ptr()
    }
    fn cptr(&mut self) -> *mut c_char {
        self.v.as_mut_ptr() as *mut c_char
    }
    fn refill(&mut self) {
        for x in self.v.iter_mut() {
            *x = 0xAA;
        }
    }
    fn check_guard(&self, what: &str) {
        assert!(
            self.v[self.n..].iter().all(|&b| b == 0xAA),
            "{what}: trailing guard clobbered: {}",
            hexs(&self.v[self.n..])
        );
    }
}

/// NUL-terminated byte vector for a `&str`.
fn cs(s: &str) -> Vec<u8> {
    let mut v = s.as_bytes().to_vec();
    v.push(0);
    v
}

/// Read the NUL-terminated string out of a buffer (for cross-verification).
fn take_cstr(b: &[u8]) -> Vec<u8> {
    let n = b.iter().position(|&x| x == 0).unwrap_or(b.len());
    let mut v = b[..n].to_vec();
    v.push(0);
    v
}

/// Run `fc` (C) then `fr` (Rust), each with its own freshly 0xAA-prefilled
/// `n`-byte output buffer + guard. Asserts the return value, errno and the FULL
/// buffer agree, and that both guards are intact. Returns `(ret, errno)`.
fn diff_buf(
    what: &str,
    n: usize,
    fc: &dyn Fn(&mut OutBuf) -> c_int,
    fr: &dyn Fn(&mut OutBuf) -> c_int,
) -> (c_int, c_int) {
    let mut bc = OutBuf::new(n);
    let mut br = OutBuf::new(n);
    errno_set(ESENT);
    let rc = fc(&mut bc);
    let ec = errno_get();
    errno_set(ESENT);
    let rr = fr(&mut br);
    let er = errno_get();
    assert_eq!(rc, rr, "{what}: return value differs (C={rc} rust={rr})");
    assert_eq!(ec, er, "{what}: errno differs (C={ec} rust={er})");
    assert_eq_bytes(&format!("{what}: out buffer"), &bc.v, &br.v);
    bc.check_guard(what);
    br.check_guard(what);
    (rc, ec)
}

/// Same, for calls with no output buffer.
fn diff_ret(what: &str, fc: &dyn Fn() -> c_int, fr: &dyn Fn() -> c_int) -> (c_int, c_int) {
    errno_set(ESENT);
    let rc = fc();
    let ec = errno_get();
    errno_set(ESENT);
    let rr = fr();
    let er = errno_get();
    assert_eq!(rc, rr, "{what}: return value differs (C={rc} rust={rr})");
    assert_eq!(ec, er, "{what}: errno differs (C={ec} rust={er})");
    (rc, ec)
}

fn expect(what: &str, got: c_int, want: c_int) {
    assert_eq!(got, want, "{what}: both libraries returned {got}, spec says {want}");
}

fn expect_errno(what: &str, got: c_int, want: c_int) {
    assert_eq!(got, want, "{what}: both libraries left errno={got}, spec says {want}");
}

// ----------------------------------------------------------- constant getters

fn cmp_int(name: &str, want: c_int) {
    let (c, r) = unsafe { pair::<IntFn0>(name) };
    let (vc, vr) = unsafe { (c(), r()) };
    assert_eq!(vc, vr, "{name}: C={vc} rust={vr}");
    assert_eq!(vc, want, "{name}: returned {vc}, spec says {want}");
}
fn cmp_size(name: &str, want: usize) {
    let (c, r) = unsafe { pair::<SizeFn0>(name) };
    let (vc, vr) = unsafe { (c(), r()) };
    assert_eq!(vc, vr, "{name}: C={vc} rust={vr}");
    assert_eq!(vc, want, "{name}: returned {vc}, spec says {want}");
}
fn cmp_u64(name: &str, want: u64) {
    let (c, r) = unsafe { pair::<U64Fn0>(name) };
    let (vc, vr) = unsafe { (c(), r()) };
    assert_eq!(vc, vr, "{name}: C={vc} rust={vr}");
    assert_eq!(vc, want, "{name}: returned {vc}, spec says {want}");
}
fn cmp_str(name: &str, want: &str) {
    let (c, r) = unsafe { pair::<CharPFn0>(name) };
    let (vc, vr) = unsafe {
        (
            std::ffi::CStr::from_ptr(c()).to_bytes().to_vec(),
            std::ffi::CStr::from_ptr(r()).to_bytes().to_vec(),
        )
    };
    assert_eq_bytes(name, &vc, &vr);
    assert_eq!(vc, want.as_bytes(), "{name}: returned {vc:?}, spec says {want:?}");
}

// -------------------------------------------------------------- RNG plumbing

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

/// A `randombytes` implementation that is a pure function of the OFFSET inside
/// each request, with NO global counter.
///
/// `common::install_det_rng` keeps a per-library counter, which is not usable
/// here: cargo runs this binary's tests as parallel threads and the `crypto_pwhash`
/// sweeps call `randombytes_buf` internally (`argon2_hash` pre-randomises the
/// raw-hash buffer), so a shared counter drifts between the two libraries. A
/// stateless stream makes the produced salts — and therefore the encoded strings
/// — byte-identical regardless of how the threads interleave.
extern "C" fn fixed_buf(p: *mut c_void, n: usize) {
    let s = unsafe { std::slice::from_raw_parts_mut(p as *mut u8, n) };
    for (i, x) in s.iter_mut().enumerate() {
        *x = (i as u8).wrapping_mul(0x6D).wrapping_add(0x5A);
    }
}
extern "C" fn fixed_random() -> u32 {
    0x1234_5678
}
extern "C" fn fixed_name() -> *const c_char {
    b"t10fixed\0".as_ptr() as *const c_char
}
extern "C" fn fixed_stir() {}
extern "C" fn fixed_close() -> c_int {
    0
}
static FIXED_IMPL: RandombytesImpl = RandombytesImpl {
    implementation_name: Some(fixed_name),
    random: Some(fixed_random),
    stir: Some(fixed_stir),
    uniform: None,
    buf: Some(fixed_buf),
    close: Some(fixed_close),
};

/// Install the stateless RNG into BOTH libraries. Caller must hold `rng_lock()`.
fn install_fixed_rng() {
    let (c, r) = unsafe { pair::<SetImplFn>("randombytes_set_implementation") };
    unsafe {
        c(&FIXED_IMPL);
        r(&FIXED_IMPL);
    }
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

/// `setrlimit(RLIMIT_CORE, 0)` — the fork-heavy tests deliberately abort.
fn no_cores() {
    unsafe {
        let rl = libc::rlimit { rlim_cur: 0, rlim_max: 0 };
        libc::setrlimit(libc::RLIMIT_CORE, &rl);
    }
}

// ===========================================================================
// CONFIGS 255 / 264 — constant getters
// ===========================================================================

#[test]
fn cfg255_argon2_constant_getters() {
    init_both();

    // ---- dispatch level (aliases of argon2id) ----
    cmp_int("crypto_pwhash_alg_argon2i13", 1);
    cmp_int("crypto_pwhash_alg_argon2id13", 2);
    cmp_int("crypto_pwhash_alg_default", 2);
    cmp_size("crypto_pwhash_bytes_min", 16);
    cmp_size("crypto_pwhash_bytes_max", 4294967295);
    cmp_size("crypto_pwhash_passwd_min", 0);
    cmp_size("crypto_pwhash_passwd_max", 4294967295);
    cmp_size("crypto_pwhash_saltbytes", 16);
    cmp_size("crypto_pwhash_strbytes", 128);
    cmp_str("crypto_pwhash_strprefix", "$argon2id$");
    cmp_u64("crypto_pwhash_opslimit_min", 1);
    cmp_u64("crypto_pwhash_opslimit_max", 4294967295);
    cmp_u64("crypto_pwhash_opslimit_interactive", 2);
    cmp_u64("crypto_pwhash_opslimit_moderate", 3);
    cmp_u64("crypto_pwhash_opslimit_sensitive", 4);
    cmp_size("crypto_pwhash_memlimit_min", 8192);
    cmp_size("crypto_pwhash_memlimit_max", 4398046510080);
    cmp_size("crypto_pwhash_memlimit_interactive", 67108864);
    cmp_size("crypto_pwhash_memlimit_moderate", 268435456);
    cmp_size("crypto_pwhash_memlimit_sensitive", 1073741824);
    cmp_str("crypto_pwhash_primitive", "argon2id,argon2i");

    // ---- argon2i primitive ----
    cmp_int("crypto_pwhash_argon2i_alg_argon2i13", 1);
    cmp_size("crypto_pwhash_argon2i_bytes_min", 16);
    cmp_size("crypto_pwhash_argon2i_bytes_max", 4294967295);
    cmp_size("crypto_pwhash_argon2i_passwd_min", 0);
    cmp_size("crypto_pwhash_argon2i_passwd_max", 4294967295);
    cmp_size("crypto_pwhash_argon2i_saltbytes", 16);
    cmp_size("crypto_pwhash_argon2i_strbytes", 128);
    cmp_str("crypto_pwhash_argon2i_strprefix", "$argon2i$");
    cmp_u64("crypto_pwhash_argon2i_opslimit_min", 3);
    cmp_u64("crypto_pwhash_argon2i_opslimit_max", 4294967295);
    cmp_u64("crypto_pwhash_argon2i_opslimit_interactive", 4);
    cmp_u64("crypto_pwhash_argon2i_opslimit_moderate", 6);
    cmp_u64("crypto_pwhash_argon2i_opslimit_sensitive", 8);
    cmp_size("crypto_pwhash_argon2i_memlimit_min", 8192);
    cmp_size("crypto_pwhash_argon2i_memlimit_max", 4398046510080);
    cmp_size("crypto_pwhash_argon2i_memlimit_interactive", 33554432);
    cmp_size("crypto_pwhash_argon2i_memlimit_moderate", 134217728);
    cmp_size("crypto_pwhash_argon2i_memlimit_sensitive", 536870912);

    // ---- argon2id primitive ----
    cmp_int("crypto_pwhash_argon2id_alg_argon2id13", 2);
    cmp_size("crypto_pwhash_argon2id_bytes_min", 16);
    cmp_size("crypto_pwhash_argon2id_bytes_max", 4294967295);
    cmp_size("crypto_pwhash_argon2id_passwd_min", 0);
    cmp_size("crypto_pwhash_argon2id_passwd_max", 4294967295);
    cmp_size("crypto_pwhash_argon2id_saltbytes", 16);
    cmp_size("crypto_pwhash_argon2id_strbytes", 128);
    cmp_str("crypto_pwhash_argon2id_strprefix", "$argon2id$");
    cmp_u64("crypto_pwhash_argon2id_opslimit_min", 1);
    cmp_u64("crypto_pwhash_argon2id_opslimit_max", 4294967295);
    cmp_u64("crypto_pwhash_argon2id_opslimit_interactive", 2);
    cmp_u64("crypto_pwhash_argon2id_opslimit_moderate", 3);
    cmp_u64("crypto_pwhash_argon2id_opslimit_sensitive", 4);
    cmp_size("crypto_pwhash_argon2id_memlimit_min", 8192);
    cmp_size("crypto_pwhash_argon2id_memlimit_max", 4398046510080);
    cmp_size("crypto_pwhash_argon2id_memlimit_interactive", 67108864);
    cmp_size("crypto_pwhash_argon2id_memlimit_moderate", 268435456);
    cmp_size("crypto_pwhash_argon2id_memlimit_sensitive", 1073741824);

    // argon2's MIN opslimit really does differ between the two primitives.
    let (ci, _) = unsafe { pair::<U64Fn0>("crypto_pwhash_argon2i_opslimit_min") };
    let (cd, _) = unsafe { pair::<U64Fn0>("crypto_pwhash_argon2id_opslimit_min") };
    assert_ne!(unsafe { ci() }, unsafe { cd() });
}

#[test]
fn cfg264_scrypt_constant_getters() {
    init_both();
    cmp_size("crypto_pwhash_scryptsalsa208sha256_bytes_min", 16);
    cmp_size("crypto_pwhash_scryptsalsa208sha256_bytes_max", 137438953440);
    cmp_size("crypto_pwhash_scryptsalsa208sha256_passwd_min", 0);
    cmp_size("crypto_pwhash_scryptsalsa208sha256_passwd_max", usize::MAX);
    cmp_size("crypto_pwhash_scryptsalsa208sha256_saltbytes", 32);
    cmp_size("crypto_pwhash_scryptsalsa208sha256_strbytes", 102);
    cmp_str("crypto_pwhash_scryptsalsa208sha256_strprefix", "$7$");
    cmp_u64("crypto_pwhash_scryptsalsa208sha256_opslimit_min", 32768);
    cmp_u64("crypto_pwhash_scryptsalsa208sha256_opslimit_max", 4294967295);
    cmp_size("crypto_pwhash_scryptsalsa208sha256_memlimit_min", 16777216);
    cmp_size("crypto_pwhash_scryptsalsa208sha256_memlimit_max", 68719476736);
    cmp_u64("crypto_pwhash_scryptsalsa208sha256_opslimit_interactive", 524288);
    cmp_size("crypto_pwhash_scryptsalsa208sha256_memlimit_interactive", 16777216);
    cmp_u64("crypto_pwhash_scryptsalsa208sha256_opslimit_sensitive", 33554432);
    cmp_size("crypto_pwhash_scryptsalsa208sha256_memlimit_sensitive", 1073741824);

    // There is NO MODERATE preset for scrypt — assert BOTH libraries agree that
    // the symbols do not exist.
    let l = libs();
    for n in [
        b"crypto_pwhash_scryptsalsa208sha256_opslimit_moderate\0".as_ref(),
        b"crypto_pwhash_scryptsalsa208sha256_memlimit_moderate\0".as_ref(),
    ] {
        let ec = unsafe { l.c.get::<SizeFn0>(n) }.is_err();
        let er = unsafe { l.r.get::<SizeFn0>(n) }.is_err();
        assert!(ec, "C unexpectedly exports {:?}", String::from_utf8_lossy(n));
        assert_eq!(ec, er, "scrypt MODERATE symbol presence differs for {:?}", String::from_utf8_lossy(n));
    }
}

// ===========================================================================
// CONFIGS 245–249 — argon2 hashing (public API + primitives + blake2b_long)
// ===========================================================================

/// Deterministic input material shared by the hashing sweeps.
fn pw_inputs(rng: &mut Rng, n: usize) -> Vec<Vec<u8>> {
    let mut v = vec![
        vec![],
        vec![0u8],
        vec![0xffu8],
        b"password".to_vec(),
        (0..n as u8).collect(),
    ];
    for _ in 0..3 {
        let k = 1 + rng.below(64);
        v.push(rng.bytes(k));
    }
    v
}

/// One `crypto_pwhash`-shaped differential call.
fn pwhash_case(
    what: &str,
    cf: PwhashFn,
    rf: PwhashFn,
    outlen: usize,
    passwd: &[u8],
    salt: &[u8],
    opslimit: u64,
    memlimit: usize,
    alg: c_int,
) -> (c_int, c_int) {
    let pw = passwd.to_vec();
    let sa = salt.to_vec();
    let pl = pw.len() as u64;
    let call = move |f: PwhashFn, b: &mut OutBuf| unsafe {
        f(
            b.ptr(),
            outlen as u64,
            pw.as_ptr() as *const c_char,
            pl,
            sa.as_ptr(),
            opslimit,
            memlimit,
            alg,
        )
    };
    diff_buf(what, outlen, &|b| call(cf, b), &|b| call(rf, b))
}

#[test]
fn cfg245_pwhash_argon2i13() {
    init_both();
    let (c, r) = fnpair!("crypto_pwhash", PwhashFn);
    let (cf, rf) = (*c, *r);
    let mut rng = Rng::new(SEED);
    let salts = patterns(ARGON2_SALTBYTES, &mut rng);
    let pws = pw_inputs(&mut rng, 64);
    let mut n = 0usize;
    // opslimit = 3 (argon2i MIN) x memlimit = 8192 (MIN) x outlen x passwdlen.
    for &outlen in &[16usize, 32, 64] {
        for pw in &pws {
            for salt in &salts {
                let what = format!(
                    "CONFIGS245 crypto_pwhash(alg=1, ops=3, mem=8192, outlen={outlen}, \
                     passwdlen={}, salt={})",
                    pw.len(),
                    hexs(salt)
                );
                let (ret, _) = pwhash_case(&what, cf, rf, outlen, pw, salt, 3, 8192, 1);
                expect(&what, ret, 0);
                n += 1;
            }
        }
    }
    assert!(n >= 64, "only {n} randomized inputs");
}

#[test]
fn cfg246_pwhash_argon2id13() {
    init_both();
    let (c, r) = fnpair!("crypto_pwhash", PwhashFn);
    let (cf, rf) = (*c, *r);
    let mut rng = Rng::new(SEED ^ 0x246);
    let salts = patterns(ARGON2_SALTBYTES, &mut rng);
    let pws = pw_inputs(&mut rng, 64);
    let mut n = 0usize;
    // argon2id OPSLIMIT_MIN is 1; `pass == 0 && slice < 2` keeps the first half
    // of pass 0 data-INdependent, the rest data-dependent.
    for &outlen in &[16usize, 32, 64] {
        for pw in &pws {
            for salt in &salts {
                let what = format!(
                    "CONFIGS246 crypto_pwhash(alg=2, ops=1, mem=8192, outlen={outlen}, \
                     passwdlen={}, salt={})",
                    pw.len(),
                    hexs(salt)
                );
                let (ret, _) = pwhash_case(&what, cf, rf, outlen, pw, salt, 1, 8192, 2);
                expect(&what, ret, 0);
                n += 1;
            }
        }
    }
    assert!(n >= 64, "only {n} randomized inputs");
}

#[test]
fn cfg247_argon2i_primitive() {
    init_both();
    let (c, r) = fnpair!("crypto_pwhash_argon2i", PwhashFn);
    let (cf, rf) = (*c, *r);
    let mut rng = Rng::new(SEED ^ 0x247);
    let salts = patterns(ARGON2_SALTBYTES, &mut rng);
    let pws = pw_inputs(&mut rng, 32);
    let mut n = 0usize;
    for &ops in &[3u64, 4] {
        for &mem in &[8192usize, 16384, 32768] {
            for pw in &pws {
                for salt in salts.iter().take(3) {
                    let what = format!(
                        "CONFIGS247 crypto_pwhash_argon2i(ops={ops}, mem={mem}, \
                         passwdlen={})",
                        pw.len()
                    );
                    let (ret, _) = pwhash_case(&what, cf, rf, 32, pw, salt, ops, mem, 1);
                    expect(&what, ret, 0);
                    n += 1;
                }
            }
        }
    }
    assert!(n >= 64, "only {n} randomized inputs");
}

#[test]
fn cfg247_argon2i_mcost_rounding() {
    init_both();
    // Drive `argon2i_hash_raw` directly so `m_cost` is in BLOCKS: `argon2_ctx`
    // rounds `memory_blocks` DOWN to a multiple of `4 * lanes`
    // (segment_length = memory_blocks / (lanes * 4); memory_blocks =
    // segment_length * lanes * 4), and bumps it UP to `8 * lanes` first.
    let (c, r) = fnpair!("_sodium_argon2i_hash_raw", HashRawFn);
    let (cf, rf) = (*c, *r);
    let (c2, r2) = fnpair!("_sodium_argon2id_hash_raw", HashRawFn);
    let (cf2, rf2) = (*c2, *r2);
    let mut rng = Rng::new(SEED ^ 0x2470);
    let salt = rng.bytes(16);
    let pw = b"password".to_vec();
    let mut n = 0usize;
    for &(lanes, mcosts) in &[
        (1u32, &[8u32, 9, 10, 11, 12, 13, 15, 16, 17, 19, 31, 32, 33][..]),
        (2u32, &[16u32, 17, 18, 23, 24, 25, 31, 32, 40][..]),
        (4u32, &[32u32, 33, 39, 40, 47, 48, 63, 64][..]),
    ] {
        for &m in mcosts {
            for &t in &[1u32, 3] {
                for (label, a, b) in [
                    ("argon2i_hash_raw", cf, rf),
                    ("argon2id_hash_raw", cf2, rf2),
                ] {
                    let what = format!(
                        "CONFIGS247 _sodium_{label}(t_cost={t}, m_cost={m}, lanes={lanes})"
                    );
                    let call = |f: HashRawFn, o: &mut OutBuf| unsafe {
                        f(
                            t,
                            m,
                            lanes,
                            pw.as_ptr() as *const c_void,
                            pw.len(),
                            salt.as_ptr() as *const c_void,
                            salt.len(),
                            o.ptr() as *mut c_void,
                            32,
                        )
                    };
                    let (ret, _) = diff_buf(&what, 32, &|o| call(a, o), &|o| call(b, o));
                    expect(&what, ret, ARGON2_OK);
                    n += 1;
                }
            }
        }
    }
    assert!(n >= 64, "only {n} inputs");
}

#[test]
fn cfg248_argon2id_primitive() {
    init_both();
    let (c, r) = fnpair!("crypto_pwhash_argon2id", PwhashFn);
    let (cf, rf) = (*c, *r);
    let mut rng = Rng::new(SEED ^ 0x248);
    let salts = patterns(ARGON2_SALTBYTES, &mut rng);
    let pws = pw_inputs(&mut rng, 32);
    let mut n = 0usize;
    // t_cost 1 = single pass; t_cost 3 = multi-pass, so from pass 1 on argon2id
    // uses data-DEPENDENT addressing for every slice.
    for &ops in &[1u64, 3] {
        for &mem in &[8192usize, 16384] {
            for pw in &pws {
                for salt in salts.iter().take(4) {
                    let what = format!(
                        "CONFIGS248 crypto_pwhash_argon2id(ops={ops}, mem={mem}, \
                         passwdlen={})",
                        pw.len()
                    );
                    let (ret, _) = pwhash_case(&what, cf, rf, 32, pw, salt, ops, mem, 2);
                    expect(&what, ret, 0);
                    n += 1;
                }
            }
        }
    }
    assert!(n >= 64, "only {n} randomized inputs");
}

#[test]
fn cfg249_blake2b_long_chunked() {
    init_both();
    let (c, r) = fnpair!("crypto_pwhash", PwhashFn);
    let (cf, rf) = (*c, *r);
    let mut rng = Rng::new(SEED ^ 0x249);
    let salt = rng.bytes(ARGON2_SALTBYTES);
    let pws: Vec<Vec<u8>> = vec![vec![], b"password".to_vec(), rng.bytes(37)];
    let mut n = 0usize;
    // outlen <= 64 -> single blake2b; outlen > 64 -> CHUNKED mode: 32 bytes,
    // then a 32-byte loop, then a short final block of `toproduce` bytes.
    for &outlen in &[
        16usize, 17, 31, 32, 33, 47, 48, 63, 64, 65, 66, 79, 80, 95, 96, 97, 127, 128, 129, 159,
        160, 161, 191, 192, 193, 255, 256, 257,
    ] {
        for &alg in &[1i32, 2] {
            for pw in &pws {
                let ops = if alg == 1 { 3 } else { 1 };
                let what = format!(
                    "CONFIGS249 crypto_pwhash(alg={alg}, outlen={outlen}, passwdlen={})",
                    pw.len()
                );
                let (ret, _) = pwhash_case(&what, cf, rf, outlen, pw, &salt, ops, 8192, alg);
                expect(&what, ret, 0);
                n += 1;
            }
        }
    }
    assert!(n >= 64, "only {n} inputs");
}

#[test]
fn cfg249_blake2b_long_direct() {
    init_both();
    let (c, r) = fnpair!("_sodium_blake2b_long", Blake2bLongFn);
    let (cf, rf) = (*c, *r);
    let mut rng = Rng::new(SEED ^ 0x2490);
    let ins: Vec<Vec<u8>> = vec![
        vec![],
        vec![0u8; 1],
        rng.bytes(4),
        rng.bytes(64),
        rng.bytes(72),
        rng.bytes(1024),
    ];
    // Sweep every outlen through 200 so the chunked loop runs 0..4 times and the
    // final short block takes every length 1..64.
    for outlen in 1usize..=200 {
        for inp in &ins {
            let what = format!("CONFIGS249 blake2b_long(outlen={outlen}, inlen={})", inp.len());
            let call = |f: Blake2bLongFn, o: &mut OutBuf| unsafe {
                f(
                    o.ptr() as *mut c_void,
                    outlen,
                    inp.as_ptr() as *const c_void,
                    inp.len(),
                )
            };
            let (ret, _) = diff_buf(&what, outlen, &|o| call(cf, o), &|o| call(rf, o));
            expect(&what, ret, 0);
        }
    }
}

// ===========================================================================
// CONFIGS 250–254 — encoded strings: _str / _str_verify / _needs_rehash
// ===========================================================================

/// Produce one encoded string from each library under the deterministic RNG and
/// assert they are IDENTICAL (the random salt is what makes this interesting).
/// Returns the NUL-terminated string.
fn str_pair(what: &str, cf: StrFn, rf: StrFn, pw: &[u8], ops: u64, mem: usize, n: usize) -> Vec<u8> {
    let mut bc = OutBuf::new(n);
    let mut br = OutBuf::new(n);
    // (stateless RNG: no counter to reset)
    errno_set(ESENT);
    let rc = unsafe { cf(bc.cptr(), pw.as_ptr() as *const c_char, pw.len() as u64, ops, mem) };
    let ec = errno_get();
    // (stateless RNG: no counter to reset)
    errno_set(ESENT);
    let rr = unsafe { rf(br.cptr(), pw.as_ptr() as *const c_char, pw.len() as u64, ops, mem) };
    let er = errno_get();
    assert_eq!(rc, rr, "{what}: _str return differs (C={rc} rust={rr})");
    assert_eq!(ec, er, "{what}: _str errno differs (C={ec} rust={er})");
    assert_eq_bytes(&format!("{what}: _str out"), &bc.v, &br.v);
    bc.check_guard(what);
    br.check_guard(what);
    assert_eq!(rc, 0, "{what}: _str failed with {rc}");
    take_cstr(&bc.v)
}

/// `_str_verify` differentially, plus CROSS-verification: the string produced by
/// one library must verify under the other (guaranteed because `str_pair`
/// already asserted the two strings are byte-identical, but the verify call is
/// made against BOTH libraries explicitly).
fn verify_pair(what: &str, cf: StrVerifyFn, rf: StrVerifyFn, s: &[u8], pw: &[u8]) -> (c_int, c_int) {
    let sv = s.to_vec();
    let pv = pw.to_vec();
    let call = move |f: StrVerifyFn| unsafe {
        f(sv.as_ptr() as *const c_char, pv.as_ptr() as *const c_char, pv.len() as u64)
    };
    diff_ret(what, &|| call(cf), &|| call(rf))
}

#[test]
fn cfg250_argon2i_str_roundtrip() {
    init_both();
    let _g = rng_lock();
    install_fixed_rng();
    let (c, r) = fnpair!("crypto_pwhash_argon2i_str", StrFn);
    let (cf, rf) = (*c, *r);
    let (vc, vr) = fnpair!("crypto_pwhash_argon2i_str_verify", StrVerifyFn);
    let (vcf, vrf) = (*vc, *vr);

    let mut rng = Rng::new(SEED ^ 0x250);
    let pws = pw_inputs(&mut rng, 40);
    for pw in &pws {
        let what = format!("CONFIGS250 argon2i_str(passwdlen={})", pw.len());
        let s = str_pair(&what, cf, rf, pw, 3, 8192, ARGON2_STRBYTES);
        assert!(
            s.starts_with(b"$argon2i$v=19$m=8,t=3,p=1$"),
            "{what}: unexpected encoding {:?}",
            String::from_utf8_lossy(&s)
        );
        // correct password -> 0 in BOTH libraries (cross-verification)
        let (ok, _) = verify_pair(&format!("{what} correct"), vcf, vrf, &s, pw);
        expect(&format!("{what} correct"), ok, 0);
        // wrong password -> -1 + EINVAL (ARGON2_VERIFY_MISMATCH)
        let mut wrong = pw.clone();
        wrong.push(b'X');
        let (bad, en) = verify_pair(&format!("{what} wrong"), vcf, vrf, &s, &wrong);
        expect(&format!("{what} wrong"), bad, -1);
        expect_errno(&format!("{what} wrong"), en, libc::EINVAL);
    }

    // Known-answer vector: also exercises decoding a string this process did not
    // produce, through both libraries.
    let v = cs(V_I);
    let (ok, _) = verify_pair("CONFIGS250 KAT argon2i", vcf, vrf, &v, b"password");
    expect("CONFIGS250 KAT argon2i", ok, 0);

    restore_sysrandom();
}

#[test]
fn cfg251_argon2id_str_roundtrip() {
    init_both();
    let _g = rng_lock();
    install_fixed_rng();
    let (c, r) = fnpair!("crypto_pwhash_argon2id_str", StrFn);
    let (cf, rf) = (*c, *r);
    let (vc, vr) = fnpair!("crypto_pwhash_argon2id_str_verify", StrVerifyFn);
    let (vcf, vrf) = (*vc, *vr);

    let mut rng = Rng::new(SEED ^ 0x251);
    let pws = pw_inputs(&mut rng, 40);
    for pw in &pws {
        let what = format!("CONFIGS251 argon2id_str(passwdlen={})", pw.len());
        let s = str_pair(&what, cf, rf, pw, 1, 8192, ARGON2_STRBYTES);
        assert!(
            s.starts_with(b"$argon2id$v=19$m=8,t=1,p=1$"),
            "{what}: unexpected encoding {:?}",
            String::from_utf8_lossy(&s)
        );
        let (ok, _) = verify_pair(&format!("{what} correct"), vcf, vrf, &s, pw);
        expect(&format!("{what} correct"), ok, 0);
        let mut wrong = pw.clone();
        wrong.push(b'X');
        let (bad, en) = verify_pair(&format!("{what} wrong"), vcf, vrf, &s, &wrong);
        expect(&format!("{what} wrong"), bad, -1);
        expect_errno(&format!("{what} wrong"), en, libc::EINVAL);
    }

    let v = cs(V_ID);
    let (ok, _) = verify_pair("CONFIGS251 KAT argon2id", vcf, vrf, &v, b"password");
    expect("CONFIGS251 KAT argon2id", ok, 0);

    restore_sysrandom();
}

#[test]
fn cfg252_str_alg_dispatch() {
    init_both();
    let _g = rng_lock();
    install_fixed_rng();
    let (c, r) = fnpair!("crypto_pwhash_str", StrFn);
    let (cf, rf) = (*c, *r);
    let (ac, ar) = fnpair!("crypto_pwhash_str_alg", StrAlgFn);
    let (acf, arf) = (*ac, *ar);
    let (vc, vr) = fnpair!("crypto_pwhash_str_verify", StrVerifyFn);
    let (vcf, vrf) = (*vc, *vr);

    let mut rng = Rng::new(SEED ^ 0x252);
    let pws = pw_inputs(&mut rng, 24);

    for pw in &pws {
        // crypto_pwhash_str == crypto_pwhash_argon2id_str: STRPREFIX "$argon2id$"
        let what = format!("CONFIGS252 crypto_pwhash_str(passwdlen={})", pw.len());
        let s = str_pair(&what, cf, rf, pw, 1, 8192, ARGON2_STRBYTES);
        assert!(s.starts_with(b"$argon2id$"), "{what}: prefix {:?}", String::from_utf8_lossy(&s));
        let (ok, _) = verify_pair(&format!("{what} verify"), vcf, vrf, &s, pw);
        expect(&format!("{what} verify"), ok, 0);

        // _str_alg with alg = 1 and alg = 2
        for (alg, ops, prefix) in [(1i32, 3u64, "$argon2i$"), (2i32, 1u64, "$argon2id$")] {
            let what = format!("CONFIGS252 crypto_pwhash_str_alg(alg={alg}, passwdlen={})", pw.len());
            let mut bc = OutBuf::new(ARGON2_STRBYTES);
            let mut br = OutBuf::new(ARGON2_STRBYTES);
            // (stateless RNG: no counter to reset)
            errno_set(ESENT);
            let rc = unsafe {
                acf(bc.cptr(), pw.as_ptr() as *const c_char, pw.len() as u64, ops, 8192, alg)
            };
            let ec = errno_get();
            // (stateless RNG: no counter to reset)
            errno_set(ESENT);
            let rr = unsafe {
                arf(br.cptr(), pw.as_ptr() as *const c_char, pw.len() as u64, ops, 8192, alg)
            };
            let er = errno_get();
            assert_eq!(rc, rr, "{what}: return differs");
            assert_eq!(ec, er, "{what}: errno differs");
            assert_eq_bytes(&what, &bc.v, &br.v);
            bc.check_guard(&what);
            br.check_guard(&what);
            expect(&what, rc, 0);
            let s = take_cstr(&bc.v);
            assert!(
                s.starts_with(prefix.as_bytes()),
                "{what}: prefix {:?}",
                String::from_utf8_lossy(&s)
            );
            let (ok, _) = verify_pair(&format!("{what} verify"), vcf, vrf, &s, pw);
            expect(&format!("{what} verify"), ok, 0);
        }
    }
    restore_sysrandom();
}

#[test]
fn cfg253_needs_rehash() {
    init_both();
    let _g = rng_lock();
    install_fixed_rng();
    let (dc, dr) = fnpair!("crypto_pwhash_str", StrFn);
    let (ic, ir) = fnpair!("crypto_pwhash_argon2i_str", StrFn);
    let (rc_, rr_) = fnpair!("crypto_pwhash_str_needs_rehash", RehashFn);
    let (ric, rir) = fnpair!("crypto_pwhash_argon2i_str_needs_rehash", RehashFn);
    let (rdc, rdr) = fnpair!("crypto_pwhash_argon2id_str_needs_rehash", RehashFn);

    let sid = str_pair("CONFIGS253 argon2id_str", *dc, *dr, b"pw", 2, 16384, ARGON2_STRBYTES);
    let si = str_pair("CONFIGS253 argon2i_str", *ic, *ir, b"pw", 3, 16384, ARGON2_STRBYTES);
    // m_cost is memlimit/1024 = 16.
    assert!(sid.starts_with(b"$argon2id$v=19$m=16,t=2,p=1$"));
    assert!(si.starts_with(b"$argon2i$v=19$m=16,t=3,p=1$"));

    let cases: &[(&str, &Vec<u8>, u64, usize, c_int)] = &[
        ("dispatch/id matching", &sid, 2, 16384, 0),
        ("dispatch/id differing opslimit", &sid, 3, 16384, 1),
        ("dispatch/id differing memlimit", &sid, 2, 32768, 1),
        ("dispatch/i matching", &si, 3, 16384, 0),
        ("dispatch/i differing opslimit", &si, 4, 16384, 1),
        ("dispatch/i differing memlimit", &si, 3, 8192, 1),
    ];
    for (label, s, ops, mem, want) in cases {
        let what = format!("CONFIGS253 crypto_pwhash_str_needs_rehash {label}");
        let sv = (*s).clone();
        let call = move |f: RehashFn| unsafe { f(sv.as_ptr() as *const c_char, *ops, *mem) };
        let (got, _) = diff_ret(&what, &|| call(*rc_), &|| call(*rr_));
        expect(&what, got, *want);
    }

    // Primitive-level getters (type is fixed, so the other variant must fail).
    let prim: &[(&str, RehashFn, RehashFn, &Vec<u8>, u64, usize, c_int)] = &[
        ("argon2i matching", *ric, *rir, &si, 3, 16384, 0),
        ("argon2i differing ops", *ric, *rir, &si, 6, 16384, 1),
        ("argon2i differing mem", *ric, *rir, &si, 3, 65536, 1),
        ("argon2id matching", *rdc, *rdr, &sid, 2, 16384, 0),
        ("argon2id differing ops", *rdc, *rdr, &sid, 1, 16384, 1),
        ("argon2id differing mem", *rdc, *rdr, &sid, 2, 8192, 1),
    ];
    for (label, a, b, s, ops, mem, want) in prim {
        let what = format!("CONFIGS253 needs_rehash {label}");
        let sv = (*s).clone();
        let call = move |f: RehashFn| unsafe { f(sv.as_ptr() as *const c_char, *ops, *mem) };
        let (got, _) = diff_ret(&what, &|| call(*a), &|| call(*b));
        expect(&what, got, *want);
    }
    restore_sysrandom();
}

#[test]
fn cfg254_encode_decode_roundtrip() {
    init_both();
    let (ec, er) = fnpair!("_sodium_argon2_encode_string", EncodeFn);
    let (dc, dr) = fnpair!("_sodium_argon2_decode_string", DecodeFn);
    let (ecf, erf) = (*ec, *er);
    let (dcf, drf) = (*dc, *dr);

    let mut rng = Rng::new(SEED ^ 0x254);
    // 1-digit and 10-digit decimals for m/t/p, and base64 ORIGINAL_NO_PADDING
    // salt/out lengths that hit every remainder class mod 3.
    let params: &[(u32, u32, u32)] = &[
        (8, 1, 1),
        (8, 3, 1),
        (16, 9, 1),
        (4294967295, 1, 1),
        (4294967295, 4294967295, 1),
        (134217720, 4294967295, 16777215),
        (1000000000, 1000000000, 1),
        (65536, 10, 2),
        (99999, 99999, 3),
    ];
    for &(m_cost, t_cost, lanes) in params {
        for &saltlen in &[8usize, 9, 10, 16, 17, 18, 32] {
            for &outlen in &[16usize, 17, 18, 32, 64] {
                let salt = rng.bytes(saltlen);
                let out = rng.bytes(outlen);
                let what = format!(
                    "CONFIGS254 encode/decode m={m_cost},t={t_cost},p={lanes} \
                     saltlen={saltlen} outlen={outlen}"
                );

                // ---- encode with both libraries ----
                let mut sc = salt.clone();
                let mut oc = out.clone();
                let mut sr = salt.clone();
                let mut or_ = out.clone();
                let mk = |s: &mut Vec<u8>, o: &mut Vec<u8>| Ctx {
                    out: o.as_mut_ptr(),
                    outlen: o.len() as u32,
                    pwd: std::ptr::null_mut(),
                    pwdlen: 0,
                    salt: s.as_mut_ptr(),
                    saltlen: s.len() as u32,
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
                let mut ctxc = mk(&mut sc, &mut oc);
                let mut ctxr = mk(&mut sr, &mut or_);
                for &ty in &[ARGON2_I, ARGON2_ID] {
                    let mut bc = OutBuf::new(256);
                    let mut br = OutBuf::new(256);
                    errno_set(ESENT);
                    let rc = unsafe { ecf(bc.cptr(), 256, &mut ctxc, ty) };
                    errno_set(ESENT);
                    let rr = unsafe { erf(br.cptr(), 256, &mut ctxr, ty) };
                    assert_eq!(rc, rr, "{what} type={ty}: encode return differs");
                    assert_eq_bytes(&format!("{what} type={ty} encoded"), &bc.v, &br.v);
                    bc.check_guard(&what);
                    br.check_guard(&what);
                    expect(&format!("{what} type={ty} encode"), rc, ARGON2_OK);
                    let enc = take_cstr(&bc.v);

                    // sanity: the decimals really are the width we asked for
                    let s = String::from_utf8(enc[..enc.len() - 1].to_vec()).unwrap();
                    assert!(s.contains(&format!("m={m_cost},t={t_cost},p={lanes}$")), "{what}: {s}");

                    // ---- decode with both libraries ----
                    let n = enc.len();
                    let mut dsc = vec![0xAAu8; n + GUARD];
                    let mut doc = vec![0xAAu8; n + GUARD];
                    let mut dsr = vec![0xAAu8; n + GUARD];
                    let mut dor = vec![0xAAu8; n + GUARD];
                    let mkd = |s: &mut Vec<u8>, o: &mut Vec<u8>| Ctx {
                        out: o.as_mut_ptr(),
                        outlen: n as u32,
                        pwd: std::ptr::null_mut(),
                        pwdlen: 0,
                        salt: s.as_mut_ptr(),
                        saltlen: n as u32,
                        secret: std::ptr::null_mut(),
                        secretlen: 0,
                        ad: std::ptr::null_mut(),
                        adlen: n as u32,
                        t_cost: 0,
                        m_cost: 0,
                        lanes: 0,
                        threads: 0,
                        flags: 0,
                    };
                    let mut dc2 = mkd(&mut dsc, &mut doc);
                    let mut dr2 = mkd(&mut dsr, &mut dor);
                    // adlen != 0 with ad == NULL would be AD_PTR_MISMATCH; mimic
                    // argon2_verify and give ad a buffer.
                    let mut adc = vec![0u8; n];
                    let mut adr = vec![0u8; n];
                    dc2.ad = adc.as_mut_ptr();
                    dr2.ad = adr.as_mut_ptr();
                    errno_set(ESENT);
                    let rc = unsafe { dcf(&mut dc2, enc.as_ptr() as *const c_char, ty) };
                    errno_set(ESENT);
                    let rr = unsafe { drf(&mut dr2, enc.as_ptr() as *const c_char, ty) };
                    assert_eq!(rc, rr, "{what} type={ty}: decode return differs");
                    expect(&format!("{what} type={ty} decode"), rc, ARGON2_OK);
                    assert_eq_bytes(&format!("{what} decoded salt"), &dsc, &dsr);
                    assert_eq_bytes(&format!("{what} decoded out"), &doc, &dor);
                    assert!(dsc[n..].iter().all(|&b| b == 0xAA), "{what}: salt guard");
                    assert!(doc[n..].iter().all(|&b| b == 0xAA), "{what}: out guard");
                    for (f, a, b) in [
                        ("saltlen", dc2.saltlen, dr2.saltlen),
                        ("outlen", dc2.outlen, dr2.outlen),
                        ("m_cost", dc2.m_cost, dr2.m_cost),
                        ("t_cost", dc2.t_cost, dr2.t_cost),
                        ("lanes", dc2.lanes, dr2.lanes),
                        ("threads", dc2.threads, dr2.threads),
                    ] {
                        assert_eq!(a, b, "{what} type={ty}: decoded ctx.{f} differs");
                    }
                    // and the round-trip really recovered the inputs
                    assert_eq!(dc2.m_cost, m_cost, "{what}: m_cost");
                    assert_eq!(dc2.t_cost, t_cost, "{what}: t_cost");
                    assert_eq!(dc2.lanes, lanes, "{what}: lanes");
                    assert_eq!(dc2.threads, lanes, "{what}: threads := lanes");
                    assert_eq!(dc2.saltlen as usize, saltlen, "{what}: saltlen");
                    assert_eq!(dc2.outlen as usize, outlen, "{what}: outlen");
                    assert_eq!(&dsc[..saltlen], &salt[..], "{what}: salt round-trip");
                    assert_eq!(&doc[..outlen], &out[..], "{what}: out round-trip");
                }
            }
        }
    }
}

// ===========================================================================
// CONFIGS 256–263 — scryptsalsa208sha256
// ===========================================================================

#[test]
fn cfg256_ll_matrix_p1() {
    init_both();
    let (c, r) = fnpair!("crypto_pwhash_scryptsalsa208sha256_ll", LlFn);
    let (cf, rf) = (*c, *r);
    let mut rng = Rng::new(SEED ^ 0x256);
    let pw = b"pleaseletmein".to_vec();
    let salt32 = rng.bytes(32);
    let mut n = 0usize;
    // `_ll` is the LOWEST-level entry point: it does NOT enforce SALTBYTES, so
    // saltlen 0 is a legal shape.
    for &nn in &[2u64, 4, 16, 1024] {
        for &rr in &[1u32, 8] {
            for &buflen in &[32usize, 33, 64, 80] {
                for &saltlen in &[0usize, 32] {
                    let what = format!(
                        "CONFIGS256 _ll(N={nn}, r={rr}, p=1, buflen={buflen}, saltlen={saltlen})"
                    );
                    let call = |f: LlFn, o: &mut OutBuf| unsafe {
                        f(
                            pw.as_ptr(),
                            pw.len(),
                            salt32.as_ptr(),
                            saltlen,
                            nn,
                            rr,
                            1,
                            o.ptr(),
                            buflen,
                        )
                    };
                    let (ret, _) = diff_buf(&what, buflen, &|o| call(cf, o), &|o| call(rf, o));
                    expect(&what, ret, 0);
                    n += 1;
                }
            }
        }
    }
    assert!(n >= 64, "only {n} inputs");
}

#[test]
fn cfg257_ll_matrix_multi_p() {
    init_both();
    let (c, r) = fnpair!("crypto_pwhash_scryptsalsa208sha256_ll", LlFn);
    let (cf, rf) = (*c, *r);
    let mut rng = Rng::new(SEED ^ 0x257);
    let salt = rng.bytes(32);
    let pws: Vec<Vec<u8>> = vec![vec![], b"pw".to_vec(), rng.bytes(71)];
    let mut n = 0usize;
    // p > 1 exercises the multi-block `for (i = 0; i < p; i++) smix(&B[128*i*r], ...)`.
    for &p in &[2u32, 3, 4] {
        for &nn in &[2u64, 16] {
            for &rr in &[1u32, 8] {
                for &buflen in &[32usize, 33, 80] {
                    for pw in &pws {
                        let what = format!(
                            "CONFIGS257 _ll(N={nn}, r={rr}, p={p}, buflen={buflen}, \
                             passwdlen={})",
                            pw.len()
                        );
                        let call = |f: LlFn, o: &mut OutBuf| unsafe {
                            f(
                                pw.as_ptr(),
                                pw.len(),
                                salt.as_ptr(),
                                salt.len(),
                                nn,
                                rr,
                                p,
                                o.ptr(),
                                buflen,
                            )
                        };
                        let (ret, _) = diff_buf(&what, buflen, &|o| call(cf, o), &|o| call(rf, o));
                        expect(&what, ret, 0);
                        n += 1;
                    }
                }
            }
        }
    }
    assert!(n >= 64, "only {n} inputs");
}

/// One `crypto_pwhash_scryptsalsa208sha256` differential call.
fn scrypt_case(
    what: &str,
    cf: ScryptFn,
    rf: ScryptFn,
    outlen: usize,
    passwd: &[u8],
    salt: &[u8],
    ops: u64,
    mem: usize,
) -> (c_int, c_int) {
    let pw = passwd.to_vec();
    let sa = salt.to_vec();
    let pl = pw.len() as u64;
    let call = move |f: ScryptFn, b: &mut OutBuf| unsafe {
        f(b.ptr(), outlen as u64, pw.as_ptr() as *const c_char, pl, sa.as_ptr(), ops, mem)
    };
    diff_buf(what, outlen, &|b| call(cf, b), &|b| call(rf, b))
}

#[test]
fn cfg258_scrypt_branch_a() {
    init_both();
    let (c, r) = fnpair!("crypto_pwhash_scryptsalsa208sha256", ScryptFn);
    let (cf, rf) = (*c, *r);
    let mut rng = Rng::new(SEED ^ 0x258);
    let salts = patterns(SCRYPT_SALTBYTES, &mut rng);
    let pws = pw_inputs(&mut rng, 32);
    let mut n = 0usize;
    // pickparams branch A: opslimit < memlimit / 32  =>  p == 1.
    // OPSLIMIT_MIN 32768 / MEMLIMIT_MIN 16777216 => N=1024, r=8, p=1 (1 MiB).
    for &outlen in &[16usize, 32, 64] {
        for pw in &pws {
            for salt in salts.iter().take(3) {
                let what = format!(
                    "CONFIGS258 scrypt(ops=32768, mem=16777216, outlen={outlen}, \
                     passwdlen={})",
                    pw.len()
                );
                let (ret, _) =
                    scrypt_case(&what, cf, rf, outlen, pw, salt, 32768, 16777216);
                expect(&what, ret, 0);
                n += 1;
            }
        }
    }
    assert!(n >= 64, "only {n} inputs");
}

#[test]
fn cfg259_scrypt_branch_b() {
    init_both();
    let (c, r) = fnpair!("crypto_pwhash_scryptsalsa208sha256", ScryptFn);
    let (cf, rf) = (*c, *r);
    let mut rng = Rng::new(SEED ^ 0x259);
    let salts = patterns(SCRYPT_SALTBYTES, &mut rng);
    let pws = pw_inputs(&mut rng, 32);
    // pickparams branch B: opslimit >= memlimit / 32, so `p` is derived from
    // maxrp and may exceed 1. `crypto_pwhash_scryptsalsa208sha256` does NOT
    // enforce MEMLIMIT_MIN, so small memlimits are legal and keep branch B cheap:
    //   (32768,  1048576) -> N=1024, r=8, p=1
    //   (65536,  1048576) -> N=1024, r=8, p=2   (multi-block smix)
    //   (262144, 1048576) -> N=1024, r=8, p=8
    //   (1048576, 16777216) -> N=16384, r=8, p=2 (the CONFIGS row's own pair)
    let cheap: &[(u64, usize)] = &[(32768, 1048576), (65536, 1048576), (262144, 1048576)];
    let mut n = 0usize;
    for &(ops, mem) in cheap {
        for &outlen in &[16usize, 32, 64] {
            for pw in &pws {
                for salt in salts.iter().take(2) {
                    let what = format!(
                        "CONFIGS259 scrypt branch B (ops={ops}, mem={mem}, outlen={outlen}, \
                         passwdlen={})",
                        pw.len()
                    );
                    let (ret, _) = scrypt_case(&what, cf, rf, outlen, pw, salt, ops, mem);
                    expect(&what, ret, 0);
                    n += 1;
                }
            }
        }
    }
    // The literal pair named by CONFIGS 259 costs ~1 s per library (N=16384,
    // p=2), so it gets exactly one input rather than the full sweep.
    let what = "CONFIGS259 scrypt branch B (ops=1048576, mem=16777216)";
    let (ret, _) = scrypt_case(what, cf, rf, 32, b"password", &salts[3], 1048576, 16777216);
    expect(what, ret, 0);
    n += 1;
    assert!(n >= 64, "only {n} inputs");
}

#[test]
fn cfg260_scrypt_opslimit_clamped() {
    init_both();
    let (c, r) = fnpair!("crypto_pwhash_scryptsalsa208sha256", ScryptFn);
    let (cf, rf) = (*c, *r);
    let (lc, lr) = fnpair!("crypto_pwhash_scryptsalsa208sha256_ll", LlFn);
    let mut rng = Rng::new(SEED ^ 0x260);
    let salt = rng.bytes(SCRYPT_SALTBYTES);

    // pickparams clamps `opslimit` UP to 32768 instead of rejecting it, so every
    // opslimit <= 32768 must give byte-identical output to opslimit == 32768.
    let mut reference = OutBuf::new(32);
    errno_set(ESENT);
    let rr0 = unsafe {
        cf(
            reference.ptr(),
            32,
            b"password\0".as_ptr() as *const c_char,
            8,
            salt.as_ptr(),
            32768,
            16777216,
        )
    };
    expect("CONFIGS260 reference ops=32768", rr0, 0);
    reference.check_guard("CONFIGS260 reference");

    for &ops in &[0u64, 1, 2, 1000, 32767, 32768] {
        let what = format!("CONFIGS260 scrypt(ops={ops} <= MIN, mem=16777216)");
        let (ret, _) = scrypt_case(&what, cf, rf, 32, b"password", &salt, ops, 16777216);
        expect(&what, ret, 0);
        // and it really equals the OPSLIMIT_MIN result
        let mut b = OutBuf::new(32);
        errno_set(ESENT);
        let x = unsafe {
            cf(
                b.ptr(),
                32,
                b"password\0".as_ptr() as *const c_char,
                8,
                salt.as_ptr(),
                ops,
                16777216,
            )
        };
        expect(&what, x, 0);
        assert_eq_bytes(&format!("{what}: clamped to OPSLIMIT_MIN"), &reference.v, &b.v);
    }

    // Cross-check the clamped parameters explicitly through `_ll`: N=1024,r=8,p=1.
    let what = "CONFIGS260 clamped params == _ll(N=1024, r=8, p=1)";
    let call = |f: LlFn, o: &mut OutBuf| unsafe {
        f(
            b"password".as_ptr(),
            8,
            salt.as_ptr(),
            SCRYPT_SALTBYTES,
            1024,
            8,
            1,
            o.ptr(),
            32,
        )
    };
    let (ret, _) = diff_buf(what, 32, &|o| call(*lc, o), &|o| call(*lr, o));
    expect(what, ret, 0);
    let mut b = OutBuf::new(32);
    unsafe {
        (*lc)(b"password".as_ptr(), 8, salt.as_ptr(), SCRYPT_SALTBYTES, 1024, 8, 1, b.ptr(), 32)
    };
    assert_eq_bytes(what, &reference.v, &b.v);
}

#[test]
fn cfg261_scrypt_str_roundtrip() {
    init_both();
    let _g = rng_lock();
    install_fixed_rng();
    let (c, r) = fnpair!("crypto_pwhash_scryptsalsa208sha256_str", StrFn);
    let (cf, rf) = (*c, *r);
    let (vc, vr) = fnpair!("crypto_pwhash_scryptsalsa208sha256_str_verify", StrVerifyFn);
    let (vcf, vrf) = (*vc, *vr);

    let mut rng = Rng::new(SEED ^ 0x261);
    let pws = pw_inputs(&mut rng, 40);
    for pw in &pws {
        let what = format!("CONFIGS261 scrypt_str(passwdlen={})", pw.len());
        let s = str_pair(&what, cf, rf, pw, 32768, 16777216, SCRYPT_STRBYTES);
        // setting = "$7$" + 1 N_log2 + 5 r + 5 p + 43 salt = 57, then '$' + 43
        // hash chars = 101 characters + NUL.
        assert_eq!(s.len(), 102, "{what}: encoded length {} (want 101+NUL)", s.len() - 1);
        assert!(s.starts_with(b"$7$"), "{what}: prefix {:?}", String::from_utf8_lossy(&s));
        assert_eq!(s[57], b'$', "{what}: setting/hash separator not at offset 57");
        // itoa64 alphabet only
        const ITOA64: &[u8] = b"./0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz";
        for (i, &b) in s[3..57].iter().chain(s[58..101].iter()).enumerate() {
            assert!(ITOA64.contains(&b), "{what}: non-itoa64 byte {b:#x} at {i}");
        }

        let (ok, _) = verify_pair(&format!("{what} correct"), vcf, vrf, &s, pw);
        expect(&format!("{what} correct"), ok, 0);
        let mut wrong = pw.clone();
        wrong.push(b'X');
        let (bad, _) = verify_pair(&format!("{what} wrong"), vcf, vrf, &s, &wrong);
        expect(&format!("{what} wrong"), bad, -1);
    }
    restore_sysrandom();
}

#[test]
fn cfg262_scrypt_needs_rehash() {
    init_both();
    let _g = rng_lock();
    install_fixed_rng();
    let (c, r) = fnpair!("crypto_pwhash_scryptsalsa208sha256_str", StrFn);
    let (rc_, rr_) = fnpair!("crypto_pwhash_scryptsalsa208sha256_str_needs_rehash", RehashFn);
    // base: pickparams(32768, 16777216) -> N_log2=10, r=8, p=1
    let s = str_pair("CONFIGS262 scrypt_str", *c, *r, b"pw", 32768, 16777216, SCRYPT_STRBYTES);
    restore_sysrandom();

    let cases: &[(&str, u64, usize, c_int)] = &[
        ("matching", 32768, 16777216, 0),
        ("opslimit clamped up to MIN still matches", 1, 16777216, 0),
        ("differing p (branch B: p=2)", 65536, 1048576, 1),
        ("differing p (branch B: p=8)", 262144, 1048576, 1),
        ("differing N_log2 (branch B)", 524288, 16777216, 1),
        // branch A's N depends only on opslimit (maxN = opslimit / 32), so a
        // bigger memlimit alone changes nothing — only opslimit moves N_log2.
        ("same N_log2 despite bigger memlimit", 32768, 33554432, 0),
        ("differing N_log2 (branch A, ops x4)", 131072, 16777216, 1),
    ];
    for (label, ops, mem, want) in cases {
        let what = format!("CONFIGS262 scrypt needs_rehash {label} (ops={ops}, mem={mem})");
        let sv = s.clone();
        let call = move |f: RehashFn| unsafe { f(sv.as_ptr() as *const c_char, *ops, *mem) };
        let (got, _) = diff_ret(&what, &|| call(*rc_), &|| call(*rr_));
        expect(&what, got, *want);
    }
}

#[test]
fn cfg263_escrypt_itoa64_and_salt_end() {
    init_both();
    let (gc, gr) = fnpair!("_sodium_escrypt_gensalt_r", GensaltFn);
    let (gcf, grf) = (*gc, *gr);
    let (pc, pr) = fnpair!("_sodium_escrypt_parse_setting", ParseSettingFn);
    let (pcf, prf) = (*pc, *pr);
    const ITOA64: &[u8] = b"./0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz";

    let mut rng = Rng::new(SEED ^ 0x263);
    let mut n = 0usize;
    // ---- gensalt_r over the whole itoa64 range of N_log2, plus r/p widths ----
    for n_log2 in 0u32..=63 {
        for &(r_, p_) in &[(1u32, 1u32), (8, 1), (8, 2), (1, 1073741823), (16777215, 1)] {
            for &srclen in &[0usize, 1, 2, 3, 32, 64] {
                let src = rng.bytes(srclen.max(1));
                let what = format!(
                    "CONFIGS263 escrypt_gensalt_r(N_log2={n_log2}, r={r_}, p={p_}, \
                     srclen={srclen})"
                );
                // buflen 3 + 1 + 5 + 5 + BYTES2CHARS(srclen) + 1
                let need = 14 + (srclen * 8 + 5) / 6 + 1;
                let mut bc = OutBuf::new(need + 8);
                let mut br = OutBuf::new(need + 8);
                errno_set(ESENT);
                let okc =
                    unsafe { gcf(n_log2, r_, p_, src.as_ptr(), srclen, bc.ptr(), need + 8) };
                errno_set(ESENT);
                let okr =
                    unsafe { grf(n_log2, r_, p_, src.as_ptr(), srclen, br.ptr(), need + 8) };
                assert_eq!(
                    okc.is_null(),
                    okr.is_null(),
                    "{what}: NULL-ness differs (C={okc:?} rust={okr:?})"
                );
                assert_eq_bytes(&what, &bc.v, &br.v);
                bc.check_guard(&what);
                br.check_guard(&what);
                if (r_ as u64) * (p_ as u64) >= (1u64 << 30) {
                    assert!(okc.is_null(), "{what}: r*p >= 2^30 must fail");
                } else {
                    assert!(!okc.is_null(), "{what}: unexpected failure");
                    let s = take_cstr(&bc.v);
                    assert_eq!(s[0..3], *b"$7$", "{what}: prefix");
                    assert_eq!(s[3], ITOA64[n_log2 as usize], "{what}: itoa64[N_log2]");
                    for &b in &s[3..s.len() - 1] {
                        assert!(ITOA64.contains(&b), "{what}: non-itoa64 byte {b:#x}");
                    }
                    // parse_setting must recover exactly what gensalt_r wrote
                    let mut a = [0u32; 3];
                    let mut b = [0u32; 3];
                    let ec = unsafe { pcf(s.as_ptr(), &mut a[0], &mut a[1], &mut a[2]) };
                    let er = unsafe { prf(s.as_ptr(), &mut b[0], &mut b[1], &mut b[2]) };
                    assert_eq!(ec.is_null(), er.is_null(), "{what}: parse NULL-ness differs");
                    assert_eq!(a, b, "{what}: parsed (N_log2,r,p) differ C={a:?} rust={b:?}");
                    assert!(!ec.is_null(), "{what}: parse_setting failed on own output");
                    assert_eq!(a, [n_log2, r_, p_], "{what}: parse round-trip {a:?}");
                    // both must return the same offset into the setting (14 chars)
                    let offc = ec as usize - s.as_ptr() as usize;
                    let offr = er as usize - s.as_ptr() as usize;
                    assert_eq!(offc, offr, "{what}: parse end offset differs");
                    assert_eq!(offc, 14, "{what}: parse end offset {offc}");
                }
                n += 1;
            }
        }
    }
    assert!(n >= 64, "only {n} inputs");

    // ---- escrypt_r salt termination: strrchr(salt,'$') vs end-of-string ----
    let _g = rng_lock();
    install_fixed_rng();
    let (ec_, er_) = fnpair!("_sodium_escrypt_r", EscryptRFn);
    let (il_c, il_r) = fnpair!("_sodium_escrypt_init_local", RegionFn);
    let (fl_c, fl_r) = fnpair!("_sodium_escrypt_free_local", RegionFn);
    // N_log2 = 1 keeps this trivially cheap; r = p = 1.
    let base = b"$7$1////////////".to_vec(); // "$7$" + N_log2 + 5*r + 5*p = 14 chars + 2 salt chars
    let variants: &[(&str, Vec<u8>)] = &[
        ("no '$' after the setting: saltlen = strlen(salt)", {
            let mut v = base.clone();
            v.extend_from_slice(b"abcdefghij");
            v.push(0);
            v
        }),
        ("trailing '$': saltlen = strrchr(salt,'$') - salt", {
            let mut v = base.clone();
            v.extend_from_slice(b"abcdefghij$");
            v.push(0);
            v
        }),
        ("two '$': strrchr takes the LAST one", {
            let mut v = base.clone();
            v.extend_from_slice(b"abc$defghij$0123");
            v.push(0);
            v
        }),
        ("zero-length salt", {
            let mut v = base[..14].to_vec();
            v.push(0);
            v
        }),
    ];
    for (label, setting) in variants {
        let what = format!("CONFIGS263 escrypt_r {label}");
        let mut regc = Region::zero();
        let mut regr = Region::zero();
        unsafe {
            (*il_c)(&mut regc);
            (*il_r)(&mut regr);
        }
        let mut bc = OutBuf::new(SCRYPT_STRBYTES);
        let mut br = OutBuf::new(SCRYPT_STRBYTES);
        errno_set(ESENT);
        let okc = unsafe {
            (*ec_)(&mut regc, b"pw".as_ptr(), 2, setting.as_ptr(), bc.ptr(), SCRYPT_STRBYTES)
        };
        errno_set(ESENT);
        let okr = unsafe {
            (*er_)(&mut regr, b"pw".as_ptr(), 2, setting.as_ptr(), br.ptr(), SCRYPT_STRBYTES)
        };
        assert_eq!(okc.is_null(), okr.is_null(), "{what}: NULL-ness differs");
        assert_eq_bytes(&what, &bc.v, &br.v);
        bc.check_guard(&what);
        br.check_guard(&what);
        unsafe {
            (*fl_c)(&mut regc);
            (*fl_r)(&mut regr);
        }
    }
    restore_sysrandom();
}

// ===========================================================================
// ERRORS 258–289 — argon2_validate_inputs / argon2_ctx / argon2_initialize /
//                  argon2_hash / argon2_verify  (driven DIRECTLY, exact codes)
// ===========================================================================

/// Backing storage for a valid `argon2_context`.
struct CtxBufs {
    out: Vec<u8>,
    pwd: Vec<u8>,
    salt: Vec<u8>,
    secret: Vec<u8>,
    ad: Vec<u8>,
}

impl CtxBufs {
    fn new() -> Self {
        CtxBufs {
            out: vec![0xAAu8; 32],
            pwd: b"password".to_vec(),
            salt: (1u8..=16).collect(),
            secret: vec![0x11u8; 8],
            ad: vec![0x22u8; 8],
        }
    }
    /// A context that `argon2_validate_inputs` accepts.
    fn ctx(&mut self) -> Ctx {
        Ctx {
            out: self.out.as_mut_ptr(),
            outlen: 32,
            pwd: self.pwd.as_mut_ptr(),
            pwdlen: self.pwd.len() as u32,
            salt: self.salt.as_mut_ptr(),
            saltlen: self.salt.len() as u32,
            secret: std::ptr::null_mut(),
            secretlen: 0,
            ad: std::ptr::null_mut(),
            adlen: 0,
            t_cost: 3,
            m_cost: 8,
            lanes: 1,
            threads: 1,
            flags: 0,
        }
    }
}

#[test]
fn e258_279_validate_inputs() {
    init_both();
    let (c, r) = fnpair!("_sodium_argon2_validate_inputs", ValidateFn);
    let (cf, rf) = (*c, *r);

    // ERRORS 258 — context == NULL
    let what = "ERRORS258 argon2_validate_inputs(NULL)";
    let (got, _) = diff_ret(what, &|| unsafe { cf(std::ptr::null()) }, &|| unsafe {
        rf(std::ptr::null())
    });
    expect(what, got, ARGON2_INCORRECT_PARAMETER);

    // Sanity: the base context is accepted.
    let mut b = CtxBufs::new();
    let base = b.ctx();
    let what = "ERRORS258.. base context";
    let (got, _) = diff_ret(what, &|| unsafe { cf(&base) }, &|| unsafe { rf(&base) });
    expect(what, got, ARGON2_OK);

    // (row label, mutation, expected code)
    type Mut = fn(&mut Ctx, &mut CtxBufs);
    let rows: &[(&str, Mut, c_int)] = &[
        ("ERRORS259 out == NULL", |c, _| c.out = std::ptr::null_mut(), ARGON2_OUTPUT_PTR_NULL),
        ("ERRORS260 outlen == 0", |c, _| c.outlen = 0, ARGON2_OUTPUT_TOO_SHORT),
        ("ERRORS260 outlen == 15", |c, _| c.outlen = 15, ARGON2_OUTPUT_TOO_SHORT),
        // ERRORS 261: outlen > ARGON2_MAX_OUTLEN is UNREACHABLE — the field is a
        // uint32_t and MAX_OUTLEN is 0xFFFFFFFF, so ARGON2_OUTPUT_TOO_LONG (-3)
        // cannot be produced. Assert the closest reachable behaviour: the maximum
        // representable outlen is ACCEPTED.
        ("ERRORS261 outlen == 0xFFFFFFFF (-3 unreachable)", |c, _| c.outlen = 0xFFFFFFFF, ARGON2_OK),
        (
            "ERRORS262 pwd == NULL && pwdlen != 0",
            |c, _| {
                c.pwd = std::ptr::null_mut();
                c.pwdlen = 8;
            },
            ARGON2_PWD_PTR_MISMATCH,
        ),
        ("ERRORS262 pwd == NULL && pwdlen == 0 is OK", |c, _| {
            c.pwd = std::ptr::null_mut();
            c.pwdlen = 0;
        }, ARGON2_OK),
        // ERRORS 263: pwdlen > ARGON2_MAX_PWD_LENGTH unreachable (uint32_t field).
        ("ERRORS263 pwdlen == 0xFFFFFFFF (-5 unreachable)", |c, _| c.pwdlen = 0xFFFFFFFF, ARGON2_OK),
        (
            "ERRORS264 salt == NULL && saltlen != 0",
            |c, _| {
                c.salt = std::ptr::null_mut();
                c.saltlen = 16;
            },
            ARGON2_SALT_PTR_MISMATCH,
        ),
        ("ERRORS265 saltlen == 0", |c, _| c.saltlen = 0, ARGON2_SALT_TOO_SHORT),
        ("ERRORS265 saltlen == 7", |c, _| c.saltlen = 7, ARGON2_SALT_TOO_SHORT),
        // ERRORS 266: saltlen > ARGON2_MAX_SALT_LENGTH unreachable (uint32_t field).
        ("ERRORS266 saltlen == 0xFFFFFFFF (-7 unreachable)", |c, _| c.saltlen = 0xFFFFFFFF, ARGON2_OK),
        (
            "ERRORS267 secret == NULL && secretlen != 0",
            |c, _| {
                c.secret = std::ptr::null_mut();
                c.secretlen = 8;
            },
            ARGON2_SECRET_PTR_MISMATCH,
        ),
        // ERRORS 268: secretlen > ARGON2_MAX_SECRET unreachable (uint32_t field).
        (
            "ERRORS268 secret != NULL && secretlen == 0xFFFFFFFF (-11 unreachable)",
            |c, b| {
                c.secret = b.secret.as_mut_ptr();
                c.secretlen = 0xFFFFFFFF;
            },
            ARGON2_OK,
        ),
        (
            "ERRORS269 ad == NULL && adlen != 0",
            |c, _| {
                c.ad = std::ptr::null_mut();
                c.adlen = 8;
            },
            ARGON2_AD_PTR_MISMATCH,
        ),
        // ERRORS 270: adlen > ARGON2_MAX_AD_LENGTH unreachable (uint32_t field).
        (
            "ERRORS270 ad != NULL && adlen == 0xFFFFFFFF (-9 unreachable)",
            |c, b| {
                c.ad = b.ad.as_mut_ptr();
                c.adlen = 0xFFFFFFFF;
            },
            ARGON2_OK,
        ),
        ("ERRORS271 lanes == 0", |c, _| c.lanes = 0, ARGON2_LANES_TOO_FEW),
        (
            "ERRORS272 lanes == 0x1000000 (> ARGON2_MAX_LANES)",
            |c, _| {
                c.lanes = 0x1000000;
                c.m_cost = 0xFFFFFFFF;
            },
            ARGON2_LANES_TOO_MANY,
        ),
        ("ERRORS273 m_cost == 0", |c, _| c.m_cost = 0, ARGON2_MEMORY_TOO_LITTLE),
        ("ERRORS273 m_cost == 7 (< ARGON2_MIN_MEMORY)", |c, _| c.m_cost = 7, ARGON2_MEMORY_TOO_LITTLE),
        // ERRORS 274: m_cost > ARGON2_MAX_MEMORY unreachable — ARGON2_MAX_MEMORY
        // is min(0xFFFFFFFF, 1<<32) = 0xFFFFFFFF on a 64-bit host and the field
        // is a uint32_t, so ARGON2_MEMORY_TOO_MUCH (-15) cannot be produced.
        ("ERRORS274 m_cost == 0xFFFFFFFF (-15 unreachable)", |c, _| c.m_cost = 0xFFFFFFFF, ARGON2_OK),
        (
            "ERRORS275 m_cost < 8 * lanes (lanes=4, m_cost=16)",
            |c, _| {
                c.lanes = 4;
                c.threads = 4;
                c.m_cost = 16;
            },
            ARGON2_MEMORY_TOO_LITTLE,
        ),
        (
            "ERRORS275 m_cost < 8 * lanes (lanes=2, m_cost=15)",
            |c, _| {
                c.lanes = 2;
                c.threads = 2;
                c.m_cost = 15;
            },
            ARGON2_MEMORY_TOO_LITTLE,
        ),
        ("ERRORS276 t_cost == 0", |c, _| c.t_cost = 0, ARGON2_TIME_TOO_SMALL),
        // ERRORS 277: t_cost > ARGON2_MAX_TIME unreachable (uint32_t field).
        ("ERRORS277 t_cost == 0xFFFFFFFF (-13 unreachable)", |c, _| c.t_cost = 0xFFFFFFFF, ARGON2_OK),
        ("ERRORS278 threads == 0", |c, _| c.threads = 0, ARGON2_THREADS_TOO_FEW),
        (
            "ERRORS279 threads == 0x1000000 (> ARGON2_MAX_THREADS)",
            |c, _| c.threads = 0x1000000,
            ARGON2_THREADS_TOO_MANY,
        ),
    ];

    for (label, f, want) in rows {
        let mut bufs = CtxBufs::new();
        let mut ctx = bufs.ctx();
        f(&mut ctx, &mut bufs);
        let (got, _) = diff_ret(label, &|| unsafe { cf(&ctx) }, &|| unsafe { rf(&ctx) });
        expect(label, got, *want);
    }
}

#[test]
fn e280_argon2_ctx_bad_type() {
    init_both();
    let (c, r) = fnpair!("_sodium_argon2_ctx", Argon2CtxFn);
    let (cf, rf) = (*c, *r);

    // ERRORS 280 — type not in { Argon2_i = 1, Argon2_id = 2 }: out-of-range
    // enum crossing FFI.
    for &ty in &[0i32, 3, -1, 255, 0x7fffffff, i32::MIN] {
        let label = format!("ERRORS280 argon2_ctx(type={ty})");
        let mut bc = CtxBufs::new();
        let mut br = CtxBufs::new();
        let mut cc = bc.ctx();
        let mut cr = br.ctx();
        let (pc, pr) = (&mut cc as *mut Ctx, &mut cr as *mut Ctx);
        let (got, _) = diff_ret(
            &label,
            &|| unsafe { cf(pc, ty) },
            &|| unsafe { rf(pr, ty) },
        );
        expect(&label, got, ARGON2_INCORRECT_TYPE);
        // the output buffer must be untouched (validate + type check happen first)
        assert_eq_bytes(&label, &bc.out, &br.out);
        assert!(bc.out.iter().all(|&x| x == 0xAA), "{label}: out was written");
    }

    // Positive control: type 1 and 2 hash successfully and identically.
    for &ty in &[ARGON2_I, ARGON2_ID] {
        let label = format!("ERRORS280 argon2_ctx(type={ty}) positive control");
        let mut bc = CtxBufs::new();
        let mut br = CtxBufs::new();
        let mut cc = bc.ctx();
        let mut cr = br.ctx();
        let (pc, pr) = (&mut cc as *mut Ctx, &mut cr as *mut Ctx);
        let (got, _) = diff_ret(
            &label,
            &|| unsafe { cf(pc, ty) },
            &|| unsafe { rf(pr, ty) },
        );
        expect(&label, got, ARGON2_OK);
        assert_eq_bytes(&label, &bc.out, &br.out);
        assert!(bc.out.iter().any(|&x| x != 0xAA), "{label}: out not written");
    }
}

#[test]
fn e281_283_initialize() {
    init_both();
    let (c, r) = fnpair!("_sodium_argon2_initialize", InitializeFn);
    let (cf, rf) = (*c, *r);

    fn inst(memory_blocks: u32, segment_length: u32) -> Inst {
        Inst {
            region: std::ptr::null_mut(),
            pseudo_rands: std::ptr::null_mut(),
            passes: 3,
            current_pass: !0u32,
            memory_blocks,
            segment_length,
            lane_length: segment_length * 4,
            lanes: 1,
            threads: 1,
            type_: ARGON2_I,
            print_internals: 0,
        }
    }

    // ERRORS 281 — instance == NULL / context == NULL
    let mut bc = CtxBufs::new();
    let mut br = CtxBufs::new();
    let mut cc = bc.ctx();
    let mut cr = br.ctx();
    let (pcc, pcr) = (&mut cc as *mut Ctx, &mut cr as *mut Ctx);
    let what = "ERRORS281 argon2_initialize(NULL, ctx)";
    let (got, _) = diff_ret(
        what,
        &|| unsafe { cf(std::ptr::null_mut(), pcc) },
        &|| unsafe { rf(std::ptr::null_mut(), pcr) },
    );
    expect(what, got, ARGON2_INCORRECT_PARAMETER);

    let mut ic = inst(8, 2);
    let mut ir = inst(8, 2);
    let (pic, pir) = (&mut ic as *mut Inst, &mut ir as *mut Inst);
    let what = "ERRORS281 argon2_initialize(inst, NULL)";
    let (got, _) = diff_ret(
        what,
        &|| unsafe { cf(pic, std::ptr::null_mut()) },
        &|| unsafe { rf(pir, std::ptr::null_mut()) },
    );
    expect(what, got, ARGON2_INCORRECT_PARAMETER);

    // ERRORS 282 — allocate_memory with m_cost == 0 (`memory_blocks == 0`).
    let mut ic = inst(0, 2);
    let mut ir = inst(0, 2);
    let (pic, pir) = (&mut ic as *mut Inst, &mut ir as *mut Inst);
    let what = "ERRORS282 allocate_memory(m_cost == 0)";
    let (got, _) = diff_ret(
        what,
        &|| unsafe { cf(pic, pcc) },
        &|| unsafe { rf(pir, pcr) },
    );
    expect(what, got, ARGON2_MEMORY_ALLOCATION_ERROR);
    assert!(ic.region.is_null() && ir.region.is_null(), "{what}: region not reset");
    assert!(
        ic.pseudo_rands.is_null() && ir.pseudo_rands.is_null(),
        "{what}: pseudo_rands not reset"
    );

    // ERRORS 283 — the mmap (C) / malloc (Rust) of `memory_blocks` 1 KiB blocks
    // fails. 0xFFFFFFFF blocks is 4 TiB, which heuristic overcommit refuses.
    let mut ic = inst(0xFFFFFFFF, 2);
    let mut ir = inst(0xFFFFFFFF, 2);
    let (pic, pir) = (&mut ic as *mut Inst, &mut ir as *mut Inst);
    let what = "ERRORS283 allocate_memory(4 TiB) fails";
    let (got, _) = diff_ret(
        what,
        &|| unsafe { cf(pic, pcc) },
        &|| unsafe { rf(pir, pcr) },
    );
    expect(what, got, ARGON2_MEMORY_ALLOCATION_ERROR);

    // ...and the same failure surfaced through the public API is ERRORS 325.
    let (pc, pr) = fnpair!("crypto_pwhash_argon2i", PwhashFn);
    let _g = rng_lock();
    install_fixed_rng();
    let salt: Vec<u8> = (1u8..=16).collect();
    let what = "ERRORS325 crypto_pwhash_argon2i inner argon2i_hash_raw != ARGON2_OK";
    let (got, en) = pwhash_case(what, *pc, *pr, 32, b"password", &salt, 3, 4398046510080, 1);
    expect(what, got, -1);
    // errno is NOT set by crypto_pwhash_argon2i on this path; whatever the
    // allocator left behind must match between the two libraries (diff_buf
    // already asserted that) and must not be the sentinel.
    assert_ne!(en, ESENT, "{what}: expected the allocator to have set errno");
    restore_sysrandom();
}

#[test]
fn e284_287_argon2_hash() {
    init_both();
    let _g = rng_lock();
    install_fixed_rng();
    let (c, r) = fnpair!("_sodium_argon2_hash", Argon2HashFn);
    let (cf, rf) = (*c, *r);
    let (ec_, er_) = fnpair!("_sodium_argon2i_hash_encoded", HashEncFn);
    let salt: Vec<u8> = (1u8..=16).collect();
    let pw = b"password".to_vec();

    // ERRORS 284 — pwdlen > ARGON2_MAX_PWD_LENGTH. `hash == NULL` so the
    // `randombytes_buf(hash, hashlen)` prologue is skipped and the bound checks
    // are reached without needing a 4 GiB buffer.
    let what = "ERRORS284 argon2_hash(pwdlen = 2^32)";
    let call = |f: Argon2HashFn, o: &mut OutBuf| unsafe {
        f(
            3,
            8,
            1,
            pw.as_ptr() as *const c_void,
            0x1_0000_0000,
            salt.as_ptr() as *const c_void,
            16,
            std::ptr::null_mut(),
            32,
            o.cptr(),
            128,
            ARGON2_I,
        )
    };
    let (got, _) = diff_buf(what, 128, &|o| call(cf, o), &|o| call(rf, o));
    expect(what, got, ARGON2_PWD_TOO_LONG);

    // ERRORS 285 — hashlen > ARGON2_MAX_OUTLEN
    let what = "ERRORS285 argon2_hash(hashlen = 2^32)";
    let call = |f: Argon2HashFn, o: &mut OutBuf| unsafe {
        f(
            3,
            8,
            1,
            pw.as_ptr() as *const c_void,
            8,
            salt.as_ptr() as *const c_void,
            16,
            std::ptr::null_mut(),
            0x1_0000_0000,
            o.cptr(),
            128,
            ARGON2_I,
        )
    };
    let (got, _) = diff_buf(what, 128, &|o| call(cf, o), &|o| call(rf, o));
    expect(what, got, ARGON2_OUTPUT_TOO_LONG);

    // ERRORS 286 — saltlen > ARGON2_MAX_SALT_LENGTH
    let what = "ERRORS286 argon2_hash(saltlen = 2^32)";
    let call = |f: Argon2HashFn, o: &mut OutBuf| unsafe {
        f(
            3,
            8,
            1,
            pw.as_ptr() as *const c_void,
            8,
            salt.as_ptr() as *const c_void,
            0x1_0000_0000,
            std::ptr::null_mut(),
            32,
            o.cptr(),
            128,
            ARGON2_I,
        )
    };
    let (got, _) = diff_buf(what, 128, &|o| call(cf, o), &|o| call(rf, o));
    expect(what, got, ARGON2_SALT_TOO_LONG);

    // ERRORS 287 — argon2_encode_string fails because `encoded` is too small.
    // The encoding of these parameters is 92 characters + NUL. Walking
    // argon2_encode_string with dst_len = L:
    //   fixed SS()/SX() segments consume 26 bytes and the last one needs L >= 27;
    //   SB(salt,16) then needs L >= 50 to leave room for the following "$";
    //   SB(out,32) needs L >= 93.
    // So `ARGON2_ENCODING_FAIL` (an SS() that does not fit) is produced for
    // L <= 26 and for L == 49; the intermediate L values instead reach
    // sodium_bin2base64 with too small a buffer, which is a sodium_misuse()
    // abort (ERRORS313), not -31.
    for &enclen in &[1usize, 2, 5, 11, 12, 15, 20, 25, 26, 49] {
        let what = format!("ERRORS287 argon2i_hash_encoded(encodedlen={enclen})");
        let call = |f: HashEncFn, o: &mut OutBuf| unsafe {
            f(
                3,
                8,
                1,
                pw.as_ptr() as *const c_void,
                8,
                salt.as_ptr() as *const c_void,
                16,
                32,
                o.cptr(),
                enclen,
            )
        };
        let (got, _) = diff_buf(&what, enclen, &|o| call(*ec_, o), &|o| call(*er_, o));
        expect(&what, got, ARGON2_ENCODING_FAIL);
        // argon2_hash zeroises `encoded` on the ENCODING_FAIL path.
    }
    // and 93 bytes is enough
    let what = "ERRORS287 argon2i_hash_encoded(encodedlen=93) succeeds";
    let call = |f: HashEncFn, o: &mut OutBuf| unsafe {
        f(
            3,
            8,
            1,
            pw.as_ptr() as *const c_void,
            8,
            salt.as_ptr() as *const c_void,
            16,
            32,
            o.cptr(),
            93,
        )
    };
    let (got, _) = diff_buf(what, 93, &|o| call(*ec_, o), &|o| call(*er_, o));
    expect(what, got, ARGON2_OK);
    restore_sysrandom();
}

#[test]
fn e288_289_argon2_verify() {
    init_both();
    let _g = rng_lock();
    install_fixed_rng();
    let (c, r) = fnpair!("_sodium_argon2_verify", Argon2VerifyFn);
    let (cf, rf) = (*c, *r);
    let (ic, ir) = fnpair!("_sodium_argon2i_verify", Argon2VerifyTFn);
    let (dc, dr) = fnpair!("_sodium_argon2id_verify", Argon2VerifyTFn);

    // ERRORS 288 — `strlen(encoded) > UINT32_MAX` -> ARGON2_DECODING_LENGTH_FAIL
    // (-34) is NOT reachable in a test: it needs a 4 GiB NUL-terminated string,
    // and the Rust translation walks it one byte at a time. Assert instead that a
    // long-but-representable string still takes the ordinary decode path.
    let long = {
        let mut v = vec![b'x'; 100_000];
        v.push(0);
        v
    };
    let what = "ERRORS288 argon2_verify(100 kB string) — -34 needs a 4 GiB string";
    let call = |f: Argon2VerifyFn| unsafe {
        f(long.as_ptr() as *const c_char, b"pw".as_ptr() as *const c_void, 2, ARGON2_I)
    };
    let (got, _) = diff_ret(what, &|| call(cf), &|| call(rf));
    expect(what, got, ARGON2_DECODING_FAIL);

    // ERRORS 289 — decode + recompute succeed but sodium_memcmp differs.
    for (label, s, ty, tf_c, tf_r) in [
        ("argon2i", cs(V_I), ARGON2_I, *ic, *ir),
        ("argon2id", cs(V_ID), ARGON2_ID, *dc, *dr),
    ] {
        let what = format!("ERRORS289 argon2_verify({label}) wrong password");
        let wrong = b"passworD".to_vec();
        let call = |f: Argon2VerifyFn| unsafe {
            f(s.as_ptr() as *const c_char, wrong.as_ptr() as *const c_void, wrong.len(), ty)
        };
        let (got, _) = diff_ret(&what, &|| call(cf), &|| call(rf));
        expect(&what, got, ARGON2_VERIFY_MISMATCH);

        // ...and the same through the type-specific wrappers
        let what = format!("ERRORS289 argon2{label}_verify wrong password");
        let call = |f: Argon2VerifyTFn| unsafe {
            f(s.as_ptr() as *const c_char, wrong.as_ptr() as *const c_void, wrong.len())
        };
        let (got, _) = diff_ret(&what, &|| call(tf_c), &|| call(tf_r));
        expect(&what, got, ARGON2_VERIFY_MISMATCH);

        // positive control
        let what = format!("ERRORS289 argon2{label}_verify correct password");
        let right = b"password".to_vec();
        let call = |f: Argon2VerifyTFn| unsafe {
            f(s.as_ptr() as *const c_char, right.as_ptr() as *const c_void, right.len())
        };
        let (got, _) = diff_ret(&what, &|| call(tf_c), &|| call(tf_r));
        expect(&what, got, ARGON2_OK);
    }
    restore_sysrandom();
}

// ===========================================================================
// ERRORS 290–315 — argon2_decode_string / argon2_encode_string / blake2b_long
// ===========================================================================

/// One `argon2_decode_string` differential call.
///
/// `ctx.out` / `ctx.salt` are sized like `argon2_verify` does it (strlen of the
/// encoded string), unless `maxsalt` / `maxout` override them, which is how the
/// "decoded salt/hash longer than the caller's buffer" branches are reached.
/// Both the returned code AND every field the decoder writes are compared.
fn decode_case(
    what: &str,
    cf: DecodeFn,
    rf: DecodeFn,
    s: &[u8],
    ty: c_int,
    maxsalt: Option<u32>,
    maxout: Option<u32>,
) -> c_int {
    let n = s.iter().position(|&x| x == 0).unwrap_or(s.len()).max(1);
    let sl = maxsalt.unwrap_or(n as u32) as usize;
    let ol = maxout.unwrap_or(n as u32) as usize;

    let mut salt_c = vec![0xAAu8; sl + GUARD];
    let mut out_c = vec![0xAAu8; ol + GUARD];
    let mut ad_c = vec![0u8; n];
    let mut salt_r = vec![0xAAu8; sl + GUARD];
    let mut out_r = vec![0xAAu8; ol + GUARD];
    let mut ad_r = vec![0u8; n];

    let mk = |salt: &mut Vec<u8>, out: &mut Vec<u8>, ad: &mut Vec<u8>| Ctx {
        out: out.as_mut_ptr(),
        outlen: ol as u32,
        pwd: std::ptr::null_mut(),
        pwdlen: 0,
        salt: salt.as_mut_ptr(),
        saltlen: sl as u32,
        secret: std::ptr::null_mut(),
        secretlen: 0,
        ad: ad.as_mut_ptr(),
        adlen: n as u32,
        t_cost: 0,
        m_cost: 0,
        lanes: 0,
        threads: 0,
        flags: 0,
    };
    let mut cc = mk(&mut salt_c, &mut out_c, &mut ad_c);
    let mut cr = mk(&mut salt_r, &mut out_r, &mut ad_r);

    errno_set(ESENT);
    let rc = unsafe { cf(&mut cc, s.as_ptr() as *const c_char, ty) };
    let ec = errno_get();
    errno_set(ESENT);
    let rr = unsafe { rf(&mut cr, s.as_ptr() as *const c_char, ty) };
    let er = errno_get();

    assert_eq!(rc, rr, "{what}: return value differs (C={rc} rust={rr})");
    assert_eq!(ec, er, "{what}: errno differs (C={ec} rust={er})");
    assert_eq_bytes(&format!("{what}: decoded salt buffer"), &salt_c, &salt_r);
    assert_eq_bytes(&format!("{what}: decoded out buffer"), &out_c, &out_r);
    assert!(salt_c[sl..].iter().all(|&b| b == 0xAA), "{what}: salt guard clobbered");
    assert!(out_c[ol..].iter().all(|&b| b == 0xAA), "{what}: out guard clobbered");
    for (f, a, b) in [
        ("outlen", cc.outlen, cr.outlen),
        ("saltlen", cc.saltlen, cr.saltlen),
        ("m_cost", cc.m_cost, cr.m_cost),
        ("t_cost", cc.t_cost, cr.t_cost),
        ("lanes", cc.lanes, cr.lanes),
        ("threads", cc.threads, cr.threads),
    ] {
        assert_eq!(a, b, "{what}: ctx.{f} differs after decode (C={a} rust={b})");
    }
    rc
}

#[test]
fn e290_309_decode_string() {
    init_both();
    let (c, r) = fnpair!("_sodium_argon2_decode_string", DecodeFn);
    let (cf, rf) = (*c, *r);

    // positive controls
    for (label, s, ty) in [("argon2i", V_I, ARGON2_I), ("argon2id", V_ID, ARGON2_ID)] {
        let what = format!("ERRORS290.. decode_string({label}) positive control");
        let got = decode_case(&what, cf, rf, &cs(s), ty, None, None);
        expect(&what, got, ARGON2_OK);
    }

    // ERRORS 290 — type not in {1,2}
    for &ty in &[0i32, 3, -1, 255, 0x7fffffff] {
        let what = format!("ERRORS290 decode_string(type={ty})");
        let got = decode_case(&what, cf, rf, &cs(V_I), ty, None, None);
        expect(&what, got, ARGON2_INCORRECT_TYPE);
    }

    // (label, string, type, maxsalt, maxout, expected)
    type Row = (&'static str, String, c_int, Option<u32>, Option<u32>, c_int);
    let salt22 = "AQIDBAUGBwgJCgsMDQ4PEA";
    let hash43 = "ITIZ35frlobU+nY8/61/sbfTj7yOIaa40ZZPYeKGn5Q";
    let mut rows: Vec<Row> = vec![];
    let mut push = |l: &'static str, s: String, ty: c_int, ms: Option<u32>, mo: Option<u32>, e: c_int| {
        rows.push((l, s, ty, ms, mo, e));
    };

    // ---- ERRORS 291: prefix mismatch (including wrong variant) ----
    push("ERRORS291 empty string", String::new(), ARGON2_I, None, None, ARGON2_DECODING_FAIL);
    push("ERRORS291 no '$'", "argon2i$v=19$m=8,t=3,p=1$x$y".into(), ARGON2_I, None, None, ARGON2_DECODING_FAIL);
    push("ERRORS291 wrong algorithm name", format!("$argon2x$v=19$m=8,t=3,p=1${salt22}${hash43}"), ARGON2_I, None, None, ARGON2_DECODING_FAIL);
    push("ERRORS291 argon2id string decoded as argon2i", V_ID.into(), ARGON2_I, None, None, ARGON2_DECODING_FAIL);
    push("ERRORS291 argon2i string decoded as argon2id", V_I.into(), ARGON2_ID, None, None, ARGON2_DECODING_FAIL);
    push("ERRORS291 truncated prefix", "$argon2".into(), ARGON2_I, None, None, ARGON2_DECODING_FAIL);
    // ---- ERRORS 292: missing "$v=" ----
    push("ERRORS292 missing $v=", format!("$argon2i$m=8,t=3,p=1${salt22}${hash43}"), ARGON2_I, None, None, ARGON2_DECODING_FAIL);
    push("ERRORS292 $V= wrong case", format!("$argon2i$V=19$m=8,t=3,p=1${salt22}${hash43}"), ARGON2_I, None, None, ARGON2_DECODING_FAIL);
    push("ERRORS292 nothing after prefix", "$argon2i".into(), ARGON2_I, None, None, ARGON2_DECODING_FAIL);
    // ---- ERRORS 293 / 307 / 308 / 309: version decimal ----
    push("ERRORS293/307 $v= with no digit", format!("$argon2i$v=$m=8,t=3,p=1${salt22}${hash43}"), ARGON2_I, None, None, ARGON2_DECODING_FAIL);
    push("ERRORS293/308 $v=019 non-minimal", format!("$argon2i$v=019$m=8,t=3,p=1${salt22}${hash43}"), ARGON2_I, None, None, ARGON2_DECODING_FAIL);
    push("ERRORS293 $v=4294967296 > UINT32_MAX", format!("$argon2i$v=4294967296$m=8,t=3,p=1${salt22}${hash43}"), ARGON2_I, None, None, ARGON2_DECODING_FAIL);
    push("ERRORS293/309 $v= overflows unsigned long", format!("$argon2i$v=99999999999999999999999$m=8,t=3,p=1${salt22}${hash43}"), ARGON2_I, None, None, ARGON2_DECODING_FAIL);
    // ---- ERRORS 294: version != ARGON2_VERSION_NUMBER (0x13 = 19) ----
    push("ERRORS294 $v=16", format!("$argon2i$v=16$m=8,t=3,p=1${salt22}${hash43}"), ARGON2_I, None, None, ARGON2_INCORRECT_TYPE);
    push("ERRORS294 $v=0", format!("$argon2i$v=0$m=8,t=3,p=1${salt22}${hash43}"), ARGON2_I, None, None, ARGON2_INCORRECT_TYPE);
    push("ERRORS294 $v=20", format!("$argon2i$v=20$m=8,t=3,p=1${salt22}${hash43}"), ARGON2_I, None, None, ARGON2_INCORRECT_TYPE);
    push("ERRORS294 $v=4294967295", format!("$argon2i$v=4294967295$m=8,t=3,p=1${salt22}${hash43}"), ARGON2_I, None, None, ARGON2_INCORRECT_TYPE);
    // ---- ERRORS 295 / 296: m= ----
    push("ERRORS295 missing $m=", format!("$argon2i$v=19$t=3,p=1${salt22}${hash43}"), ARGON2_I, None, None, ARGON2_DECODING_FAIL);
    push("ERRORS295 $n= instead of $m=", format!("$argon2i$v=19$n=8,t=3,p=1${salt22}${hash43}"), ARGON2_I, None, None, ARGON2_DECODING_FAIL);
    push("ERRORS296/307 m= with no digit", format!("$argon2i$v=19$m=,t=3,p=1${salt22}${hash43}"), ARGON2_I, None, None, ARGON2_DECODING_FAIL);
    push("ERRORS296/308 m=065536 non-minimal", format!("$argon2i$v=19$m=065536,t=3,p=1${salt22}${hash43}"), ARGON2_I, None, None, ARGON2_DECODING_FAIL);
    push("ERRORS296/308 m=08 non-minimal", format!("$argon2i$v=19$m=08,t=3,p=1${salt22}${hash43}"), ARGON2_I, None, None, ARGON2_DECODING_FAIL);
    push("ERRORS296 m=4294967296 > UINT32_MAX", format!("$argon2i$v=19$m=4294967296,t=3,p=1${salt22}${hash43}"), ARGON2_I, None, None, ARGON2_DECODING_FAIL);
    push("ERRORS296/309 m= overflows unsigned long", format!("$argon2i$v=19$m=18446744073709551616,t=3,p=1${salt22}${hash43}"), ARGON2_I, None, None, ARGON2_DECODING_FAIL);
    push("ERRORS296/309 m= 40 nines", format!("$argon2i$v=19$m={},t=3,p=1${salt22}${hash43}", "9".repeat(40)), ARGON2_I, None, None, ARGON2_DECODING_FAIL);
    // ---- ERRORS 297 / 298: ,t= ----
    push("ERRORS297 missing ,t=", format!("$argon2i$v=19$m=8,p=1${salt22}${hash43}"), ARGON2_I, None, None, ARGON2_DECODING_FAIL);
    push("ERRORS297 ';t=' instead of ',t='", format!("$argon2i$v=19$m=8;t=3,p=1${salt22}${hash43}"), ARGON2_I, None, None, ARGON2_DECODING_FAIL);
    push("ERRORS298/307 t= with no digit", format!("$argon2i$v=19$m=8,t=,p=1${salt22}${hash43}"), ARGON2_I, None, None, ARGON2_DECODING_FAIL);
    push("ERRORS298/308 t=03 non-minimal", format!("$argon2i$v=19$m=8,t=03,p=1${salt22}${hash43}"), ARGON2_I, None, None, ARGON2_DECODING_FAIL);
    push("ERRORS298 t=4294967296 > UINT32_MAX", format!("$argon2i$v=19$m=8,t=4294967296,p=1${salt22}${hash43}"), ARGON2_I, None, None, ARGON2_DECODING_FAIL);
    push("ERRORS298/309 t= overflows unsigned long", format!("$argon2i$v=19$m=8,t=99999999999999999999,p=1${salt22}${hash43}"), ARGON2_I, None, None, ARGON2_DECODING_FAIL);
    // ---- ERRORS 299 / 300: ,p= ----
    push("ERRORS299 missing ,p=", format!("$argon2i$v=19$m=8,t=3${salt22}${hash43}"), ARGON2_I, None, None, ARGON2_DECODING_FAIL);
    push("ERRORS299 ';p=' instead of ',p='", format!("$argon2i$v=19$m=8,t=3;p=1${salt22}${hash43}"), ARGON2_I, None, None, ARGON2_DECODING_FAIL);
    push("ERRORS300/307 p= with no digit", format!("$argon2i$v=19$m=8,t=3,p=${salt22}${hash43}"), ARGON2_I, None, None, ARGON2_DECODING_FAIL);
    push("ERRORS300/308 p=01 non-minimal", format!("$argon2i$v=19$m=8,t=3,p=01${salt22}${hash43}"), ARGON2_I, None, None, ARGON2_DECODING_FAIL);
    push("ERRORS300 p=4294967296 > UINT32_MAX", format!("$argon2i$v=19$m=8,t=3,p=4294967296${salt22}${hash43}"), ARGON2_I, None, None, ARGON2_DECODING_FAIL);
    push("ERRORS300/309 p= overflows unsigned long", format!("$argon2i$v=19$m=8,t=3,p=123456789012345678901234${salt22}${hash43}"), ARGON2_I, None, None, ARGON2_DECODING_FAIL);
    // ---- ERRORS 301: missing "$" before the salt ----
    push("ERRORS301 '#' instead of '$' before salt", format!("$argon2i$v=19$m=8,t=3,p=1#{salt22}${hash43}"), ARGON2_I, None, None, ARGON2_DECODING_FAIL);
    push("ERRORS301 string ends after p=", "$argon2i$v=19$m=8,t=3,p=1".into(), ARGON2_I, None, None, ARGON2_DECODING_FAIL);
    // ---- ERRORS 302: bad base64 salt / decoded salt longer than ctx->saltlen ----
    // A single leftover base64 character leaves acc_len == 6 > 4 -> base642bin -1.
    push("ERRORS302 salt with a dangling b64 char", format!("$argon2i$v=19$m=8,t=3,p=1$A${hash43}"), ARGON2_I, None, None, ARGON2_DECODING_FAIL);
    push("ERRORS302 salt with non-canonical trailing bits", format!("$argon2i$v=19$m=8,t=3,p=1$AB${hash43}"), ARGON2_I, None, None, ARGON2_DECODING_FAIL);
    push("ERRORS302 decoded salt longer than ctx->saltlen", V_I.into(), ARGON2_I, Some(4), None, ARGON2_DECODING_FAIL);
    push("ERRORS302 ctx->saltlen == 0", V_I.into(), ARGON2_I, Some(0), None, ARGON2_DECODING_FAIL);
    // ---- ERRORS 303: missing "$" before the hash ----
    push("ERRORS303 '#' instead of '$' before hash", format!("$argon2i$v=19$m=8,t=3,p=1${salt22}#{hash43}"), ARGON2_I, None, None, ARGON2_DECODING_FAIL);
    push("ERRORS303 string ends after the salt", format!("$argon2i$v=19$m=8,t=3,p=1${salt22}"), ARGON2_I, None, None, ARGON2_DECODING_FAIL);
    // ---- ERRORS 304: bad base64 hash / decoded hash longer than ctx->outlen ----
    push("ERRORS304 hash with a dangling b64 char", format!("$argon2i$v=19$m=8,t=3,p=1${salt22}$A"), ARGON2_I, None, None, ARGON2_DECODING_FAIL);
    push("ERRORS304 hash with non-canonical trailing bits", format!("$argon2i$v=19$m=8,t=3,p=1${salt22}$AB"), ARGON2_I, None, None, ARGON2_DECODING_FAIL);
    push("ERRORS304 decoded hash longer than ctx->outlen", V_I.into(), ARGON2_I, None, Some(8), ARGON2_DECODING_FAIL);
    push("ERRORS304 ctx->outlen == 0", V_I.into(), ARGON2_I, None, Some(0), ARGON2_DECODING_FAIL);
    // ---- ERRORS 305: argon2_validate_inputs on the DECODED context ----
    // Note the check order inside validate_inputs: outlen, saltlen, lanes, m_cost, t_cost.
    push("ERRORS305 decoded outlen < 16 -> -2", format!("$argon2i$v=19$m=8,t=3,p=1${salt22}$AAAAAAAAAAA"), ARGON2_I, None, None, ARGON2_OUTPUT_TOO_SHORT);
    push("ERRORS305 decoded saltlen < 8 -> -6", format!("$argon2i$v=19$m=8,t=3,p=1$AAAAAAAA${hash43}"), ARGON2_I, None, None, ARGON2_SALT_TOO_SHORT);
    push("ERRORS305 decoded saltlen == 0 -> -6", format!("$argon2i$v=19$m=8,t=3,p=1$${hash43}"), ARGON2_I, None, None, ARGON2_SALT_TOO_SHORT);
    push("ERRORS305 decoded p == 0 -> -16", format!("$argon2i$v=19$m=8,t=3,p=0${salt22}${hash43}"), ARGON2_I, None, None, ARGON2_LANES_TOO_FEW);
    push("ERRORS305 decoded p > 16777215 -> -17", format!("$argon2i$v=19$m=4294967295,t=3,p=16777216${salt22}${hash43}"), ARGON2_I, None, None, ARGON2_LANES_TOO_MANY);
    push("ERRORS305 decoded m_cost < 8 -> -14", format!("$argon2i$v=19$m=7,t=3,p=1${salt22}${hash43}"), ARGON2_I, None, None, ARGON2_MEMORY_TOO_LITTLE);
    push("ERRORS305 decoded m_cost == 0 -> -14", format!("$argon2i$v=19$m=0,t=3,p=1${salt22}${hash43}"), ARGON2_I, None, None, ARGON2_MEMORY_TOO_LITTLE);
    push("ERRORS305 decoded m_cost < 8*p -> -14", format!("$argon2i$v=19$m=8,t=3,p=2${salt22}${hash43}"), ARGON2_I, None, None, ARGON2_MEMORY_TOO_LITTLE);
    push("ERRORS305 decoded m_cost < 8*p (p=4) -> -14", format!("$argon2i$v=19$m=31,t=3,p=4${salt22}${hash43}"), ARGON2_I, None, None, ARGON2_MEMORY_TOO_LITTLE);
    push("ERRORS305 decoded t_cost == 0 -> -12", format!("$argon2i$v=19$m=8,t=0,p=1${salt22}${hash43}"), ARGON2_I, None, None, ARGON2_TIME_TOO_SMALL);
    // ---- ERRORS 306: trailing garbage after the hash ----
    push("ERRORS306 trailing '$'", format!("$argon2i$v=19$m=8,t=3,p=1${salt22}${hash43}$"), ARGON2_I, None, None, ARGON2_DECODING_FAIL);
    push("ERRORS306 trailing '$x'", format!("$argon2i$v=19$m=8,t=3,p=1${salt22}${hash43}$x"), ARGON2_I, None, None, ARGON2_DECODING_FAIL);
    push("ERRORS306 trailing newline", format!("$argon2i$v=19$m=8,t=3,p=1${salt22}${hash43}\n"), ARGON2_I, None, None, ARGON2_DECODING_FAIL);
    push("ERRORS306 trailing space", format!("$argon2i$v=19$m=8,t=3,p=1${salt22}${hash43} "), ARGON2_I, None, None, ARGON2_DECODING_FAIL);
    // ---- the same battery against the argon2id variant ----
    let s22 = salt22;
    let h43 = "C2fnILzcT+o9I0ezc1Uao3WSxjd5obVwP6tbm4pZSwI";
    push("ERRORS294 argon2id $v=16", format!("$argon2id$v=16$m=8,t=1,p=1${s22}${h43}"), ARGON2_ID, None, None, ARGON2_INCORRECT_TYPE);
    push("ERRORS296 argon2id m=08", format!("$argon2id$v=19$m=08,t=1,p=1${s22}${h43}"), ARGON2_ID, None, None, ARGON2_DECODING_FAIL);
    push("ERRORS305 argon2id t=0", format!("$argon2id$v=19$m=8,t=0,p=1${s22}${h43}"), ARGON2_ID, None, None, ARGON2_TIME_TOO_SMALL);
    push("ERRORS306 argon2id trailing garbage", format!("$argon2id$v=19$m=8,t=1,p=1${s22}${h43}#"), ARGON2_ID, None, None, ARGON2_DECODING_FAIL);

    for (label, s, ty, ms, mo, want) in &rows {
        let got = decode_case(label, cf, rf, &cs(s), *ty, *ms, *mo);
        expect(label, got, *want);
    }
    assert!(rows.len() >= 60, "only {} decode rows", rows.len());
}

#[test]
fn e290_309_via_str_verify() {
    init_both();
    // The same malformed strings, this time through the PUBLIC verify entry
    // points, where every decode failure collapses to -1. ERRORS 338 says errno
    // must be left UNCHANGED for anything other than ARGON2_VERIFY_MISMATCH.
    let (vc, vr) = fnpair!("crypto_pwhash_argon2i_str_verify", StrVerifyFn);
    let (dc, dr) = fnpair!("crypto_pwhash_argon2id_str_verify", StrVerifyFn);
    let (tc, tr) = fnpair!("crypto_pwhash_str_verify", StrVerifyFn);
    let salt22 = "AQIDBAUGBwgJCgsMDQ4PEA";
    let hash43 = "ITIZ35frlobU+nY8/61/sbfTj7yOIaa40ZZPYeKGn5Q";

    let bad: Vec<(String, String)> = vec![
        ("ERRORS291 wrong prefix".into(), format!("$argon2x$v=19$m=8,t=3,p=1${salt22}${hash43}")),
        ("ERRORS292 missing $v=".into(), format!("$argon2i$m=8,t=3,p=1${salt22}${hash43}")),
        ("ERRORS294 $v=16".into(), format!("$argon2i$v=19$m=8,t=3,p=1${salt22}${hash43}").replace("v=19", "v=16")),
        ("ERRORS295 missing $m=".into(), format!("$argon2i$v=19$t=3,p=1${salt22}${hash43}")),
        ("ERRORS296 m=065536".into(), format!("$argon2i$v=19$m=065536,t=3,p=1${salt22}${hash43}")),
        ("ERRORS297 missing ,t=".into(), format!("$argon2i$v=19$m=8,p=1${salt22}${hash43}")),
        ("ERRORS298 t=03".into(), format!("$argon2i$v=19$m=8,t=03,p=1${salt22}${hash43}")),
        ("ERRORS299 missing ,p=".into(), format!("$argon2i$v=19$m=8,t=3${salt22}${hash43}")),
        ("ERRORS300 p=01".into(), format!("$argon2i$v=19$m=8,t=3,p=01${salt22}${hash43}")),
        ("ERRORS301 no $ before salt".into(), format!("$argon2i$v=19$m=8,t=3,p=1#{salt22}${hash43}")),
        ("ERRORS302 bad b64 salt".into(), format!("$argon2i$v=19$m=8,t=3,p=1$AB${hash43}")),
        ("ERRORS303 no $ before hash".into(), format!("$argon2i$v=19$m=8,t=3,p=1${salt22}#{hash43}")),
        ("ERRORS304 bad b64 hash".into(), format!("$argon2i$v=19$m=8,t=3,p=1${salt22}$AB")),
        ("ERRORS305 saltlen<8".into(), format!("$argon2i$v=19$m=8,t=3,p=1$AAAAAAAA${hash43}")),
        ("ERRORS305 outlen<16".into(), format!("$argon2i$v=19$m=8,t=3,p=1${salt22}$AAAAAAAAAAA")),
        ("ERRORS305 m_cost<8".into(), format!("$argon2i$v=19$m=7,t=3,p=1${salt22}${hash43}")),
        ("ERRORS305 m_cost<8*p".into(), format!("$argon2i$v=19$m=8,t=3,p=2${salt22}${hash43}")),
        ("ERRORS305 t=0".into(), format!("$argon2i$v=19$m=8,t=0,p=1${salt22}${hash43}")),
        ("ERRORS305 p=0".into(), format!("$argon2i$v=19$m=8,t=3,p=0${salt22}${hash43}")),
        ("ERRORS305 p>16777215".into(), format!("$argon2i$v=19$m=4294967295,t=3,p=16777216${salt22}${hash43}")),
        ("ERRORS306 trailing garbage".into(), format!("$argon2i$v=19$m=8,t=3,p=1${salt22}${hash43}$x")),
        ("ERRORS307 no digit".into(), format!("$argon2i$v=19$m=,t=3,p=1${salt22}${hash43}")),
        ("ERRORS309 decimal overflow".into(), format!("$argon2i$v=19$m={},t=3,p=1${salt22}${hash43}", "9".repeat(30))),
    ];

    for (label, s) in &bad {
        let sv = cs(s);
        let what = format!("{label} via crypto_pwhash_argon2i_str_verify");
        let call = |f: StrVerifyFn| unsafe {
            f(sv.as_ptr() as *const c_char, b"password\0".as_ptr() as *const c_char, 8)
        };
        let (got, en) = diff_ret(&what, &|| call(*vc), &|| call(*vr));
        expect(&what, got, -1);
        // ERRORS 338: errno untouched for every non-MISMATCH failure.
        expect_errno(&format!("{what} (ERRORS338 errno untouched)"), en, ESENT);

        // and through the dispatching wrapper, which finds the "$argon2i$"
        // prefix and forwards (except where the prefix itself is broken).
        let what = format!("{label} via crypto_pwhash_str_verify");
        let (got, _) = diff_ret(&what, &|| call(*tc), &|| call(*tr));
        expect(&what, got, -1);
    }

    // A non-NUL-terminated buffer: the decoder walks past where the string
    // "should" end into 0xAA filler, which is not base64, so the trailing-garbage
    // branch fires. Both libraries must agree.
    let mut nonterm = V_I.as_bytes().to_vec();
    nonterm.resize(300, 0xAA);
    nonterm[299] = 0;
    let what = "ERRORS338 argon2i_str_verify(non-NUL-terminated str)";
    let call = |f: StrVerifyFn| unsafe {
        f(nonterm.as_ptr() as *const c_char, b"password\0".as_ptr() as *const c_char, 8)
    };
    let (got, en) = diff_ret(what, &|| call(*vc), &|| call(*vr));
    expect(what, got, -1);
    expect_errno(what, en, ESENT);

    // ...and the argon2id verifier rejects an argon2i string (wrong variant).
    let sv = cs(V_I);
    let what = "ERRORS291 argon2id_str_verify(argon2i string)";
    let call = |f: StrVerifyFn| unsafe {
        f(sv.as_ptr() as *const c_char, b"password\0".as_ptr() as *const c_char, 8)
    };
    let (got, en) = diff_ret(what, &|| call(*dc), &|| call(*dr));
    expect(what, got, -1);
    expect_errno(what, en, ESENT);
}

#[test]
fn e310_312_encode_string() {
    init_both();
    let (c, r) = fnpair!("_sodium_argon2_encode_string", EncodeFn);
    let (cf, rf) = (*c, *r);

    // ERRORS 310 — type not in {1,2} hits the switch default -> -31.
    for &ty in &[0i32, 3, -1, 255, 0x7fffffff] {
        let what = format!("ERRORS310 argon2_encode_string(type={ty})");
        let mut bufs_c = CtxBufs::new();
        let mut bufs_r = CtxBufs::new();
        let mut cc = bufs_c.ctx();
        let mut cr = bufs_r.ctx();
        let (pc, pr) = (&mut cc as *mut Ctx, &mut cr as *mut Ctx);
        let call = |f: EncodeFn, p: *mut Ctx, o: &mut OutBuf| unsafe { f(o.cptr(), 256, p, ty) };
        let (got, _) = diff_buf(&what, 256, &|o| call(cf, pc, o), &|o| call(rf, pr, o));
        expect(&what, got, ARGON2_ENCODING_FAIL);
    }

    // ERRORS 311 — argon2_validate_inputs on the caller's context fails, and its
    // code (NOT -31) is returned. Note the "$argon2i$v=" segment is emitted
    // BEFORE validation, so dst_len must be at least 12.
    type Mut = fn(&mut Ctx);
    let rows: &[(&str, Mut, c_int)] = &[
        ("ERRORS311 outlen == 8 -> -2", |c| c.outlen = 8, ARGON2_OUTPUT_TOO_SHORT),
        ("ERRORS311 out == NULL -> -1", |c| c.out = std::ptr::null_mut(), ARGON2_OUTPUT_PTR_NULL),
        ("ERRORS311 saltlen == 4 -> -6", |c| c.saltlen = 4, ARGON2_SALT_TOO_SHORT),
        ("ERRORS311 t_cost == 0 -> -12", |c| c.t_cost = 0, ARGON2_TIME_TOO_SMALL),
        ("ERRORS311 m_cost == 1 -> -14", |c| c.m_cost = 1, ARGON2_MEMORY_TOO_LITTLE),
        ("ERRORS311 lanes == 0 -> -16", |c| c.lanes = 0, ARGON2_LANES_TOO_FEW),
        ("ERRORS311 threads == 0 -> -28", |c| c.threads = 0, ARGON2_THREADS_TOO_FEW),
    ];
    for (label, f, want) in rows {
        let mut bufs_c = CtxBufs::new();
        let mut bufs_r = CtxBufs::new();
        let mut cc = bufs_c.ctx();
        let mut cr = bufs_r.ctx();
        f(&mut cc);
        f(&mut cr);
        let (pc, pr) = (&mut cc as *mut Ctx, &mut cr as *mut Ctx);
        let call = |g: EncodeFn, p: *mut Ctx, o: &mut OutBuf| unsafe { g(o.cptr(), 256, p, ARGON2_I) };
        let (got, _) = diff_buf(label, 256, &|o| call(cf, pc, o), &|o| call(rf, pr, o));
        expect(label, got, *want);
    }

    // ERRORS 312 — an SS()/SX() fixed segment does not fit in the remaining
    // dst_len. The fixed part of this encoding is 26 characters and the final
    // "$" before the hash needs dst_len >= 50, so L <= 26 and L == 49 fail here.
    for &l in &[1usize, 2, 5, 10, 11, 12, 13, 20, 25, 26, 49] {
        let what = format!("ERRORS312 argon2_encode_string(dst_len={l})");
        let mut bufs_c = CtxBufs::new();
        let mut bufs_r = CtxBufs::new();
        let mut cc = bufs_c.ctx();
        let mut cr = bufs_r.ctx();
        let (pc, pr) = (&mut cc as *mut Ctx, &mut cr as *mut Ctx);
        let call = |f: EncodeFn, p: *mut Ctx, o: &mut OutBuf| unsafe { f(o.cptr(), l, p, ARGON2_I) };
        let (got, _) = diff_buf(&what, l, &|o| call(cf, pc, o), &|o| call(rf, pr, o));
        expect(&what, got, ARGON2_ENCODING_FAIL);
    }
    // ...and dst_len == 93 is exactly enough.
    let what = "ERRORS312 argon2_encode_string(dst_len=93) succeeds";
    let mut bufs_c = CtxBufs::new();
    let mut bufs_r = CtxBufs::new();
    let mut cc = bufs_c.ctx();
    let mut cr = bufs_r.ctx();
    let (pc, pr) = (&mut cc as *mut Ctx, &mut cr as *mut Ctx);
    let call = |f: EncodeFn, p: *mut Ctx, o: &mut OutBuf| unsafe { f(o.cptr(), 93, p, ARGON2_I) };
    let (got, _) = diff_buf(what, 93, &|o| call(cf, pc, o), &|o| call(rf, pr, o));
    expect(what, got, ARGON2_OK);
}

#[test]
fn e313_encode_string_sb_misuse() {
    init_both();
    no_cores();
    // ERRORS 313 — `SB()` is documented as returning ARGON2_ENCODING_FAIL when
    // `sodium_bin2base64` returns NULL, but sodium_bin2base64 NEVER returns NULL
    // for a too-small buffer: it calls `sodium_misuse()`. So the real observable
    // behaviour is a SIGABRT, and the `return ARGON2_ENCODING_FAIL` inside SB is
    // dead code. dst_len 27..48 trips it on the salt, 50..92 on the hash.
    let (c, r) = fnpair!("_sodium_argon2_encode_string", EncodeFn);
    let (cf, rf) = (*c, *r);
    let mut bufs_c = CtxBufs::new();
    let mut bufs_r = CtxBufs::new();
    let mut cc = bufs_c.ctx();
    let mut cr = bufs_r.ctx();
    let (pc, pr) = (&mut cc as *mut Ctx, &mut cr as *mut Ctx);
    let mut dst = vec![0u8; 256];
    let dp = dst.as_mut_ptr() as *mut c_char;

    for &l in &[27usize, 30, 40, 48, 50, 60, 70, 92] {
        let what = format!("ERRORS313 argon2_encode_string(dst_len={l}) -> sodium_misuse");
        let oc = forked(|| unsafe { cf(dp, l, pc, ARGON2_I) as i64 });
        let or_ = forked(|| unsafe { rf(dp, l, pr, ARGON2_I) as i64 });
        assert_same_fatal(&what, oc, or_);
        assert_eq!(oc, Outcome::Signaled(SIGABRT), "{what}: expected SIGABRT, got {oc:?}");
    }
}

#[test]
fn e314_315_blake2b_long() {
    init_both();
    let (c, r) = fnpair!("_sodium_blake2b_long", Blake2bLongFn);
    let (cf, rf) = (*c, *r);
    let input = vec![0x42u8; 128];

    // ERRORS 314 — outlen > UINT32_MAX. The bound is checked FIRST, so no
    // 4 GiB buffer is needed.
    for &outlen in &[0x1_0000_0000usize, 0x1_0000_0001, usize::MAX] {
        let what = format!("ERRORS314 blake2b_long(outlen={outlen})");
        let call = |f: Blake2bLongFn, o: &mut OutBuf| unsafe {
            f(o.ptr() as *mut c_void, outlen, input.as_ptr() as *const c_void, input.len())
        };
        let (got, _) = diff_buf(&what, 64, &|o| call(cf, o), &|o| call(rf, o));
        expect(&what, got, -1);
    }
    // 0xFFFFFFFF is exactly ARGON2_MAX_OUTLEN and would be accepted, but it needs
    // a 4 GiB destination, so it is covered by e316_e326 instead.

    // ERRORS 315 — an inner crypto_generichash_blake2b_* call returns < 0.
    // outlen == 0 makes `crypto_generichash_blake2b_init` fail.
    let what = "ERRORS315 blake2b_long(outlen=0)";
    let call = |f: Blake2bLongFn, o: &mut OutBuf| unsafe {
        f(o.ptr() as *mut c_void, 0, input.as_ptr() as *const c_void, input.len())
    };
    let (got, _) = diff_buf(what, 0, &|o| call(cf, o), &|o| call(rf, o));
    expect(what, got, -1);
}
