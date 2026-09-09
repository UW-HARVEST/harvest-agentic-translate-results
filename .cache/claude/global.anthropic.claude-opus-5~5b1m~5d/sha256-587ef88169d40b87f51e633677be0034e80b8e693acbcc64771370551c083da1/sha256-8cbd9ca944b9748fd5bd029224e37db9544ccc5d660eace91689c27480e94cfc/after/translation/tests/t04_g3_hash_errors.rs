//! Phase C — error/rejection-path differential tests for module group G3.
//!
//! Covers ERRORS.md rows 282-404 plus the generic boundary sweep required by
//! the task (NULL pointers where the C never dereferences them, zero and
//! oversized lengths, one step past every documented range, out-of-range
//! `domain` bytes passed across FFI).
//!
//! Three response classes are distinguished, exactly as ERRORS.md describes:
//!   * return value  -> compared directly;
//!   * `errno`       -> zeroed, call, compare both return value and errno;
//!   * process death -> `diff_abort_case()` re-executes this binary once per
//!                      library and asserts both children died identically.
//!
//! IMPORTANT build note: the reference C library is compiled with an empty
//! `CMAKE_BUILD_TYPE`, i.e. WITHOUT `-DNDEBUG`, so `assert()` is live. That
//! makes ERRORS.md rows 302 (and therefore 301/303, which are the same
//! `assert(outlen <= UINT8_MAX)` site) abort rather than silently truncate.
//! The tests below assert C/Rust *agreement* on those rows instead of the
//! NDEBUG-only behaviour noted in the table.
#![allow(clippy::too_many_arguments)]
mod common;
use common::*;
use std::os::raw::{c_char, c_int, c_void};
use std::ptr;

const SEED: u64 = 0x4E44_5252_5333_0001;

// ---------------------------------------------------------------- signatures
type Sz = unsafe extern "C" fn() -> usize;
type Keygen = unsafe extern "C" fn(*mut u8);
type OneShotK = unsafe extern "C" fn(*mut u8, *const u8, u64, *const u8) -> c_int;
type VerifyK = unsafe extern "C" fn(*const u8, *const u8, u64, *const u8) -> c_int;
type OneShot = unsafe extern "C" fn(*mut u8, *const u8, u64) -> c_int;
type StInit = unsafe extern "C" fn(*mut u8) -> c_int;
type StUpd = unsafe extern "C" fn(*mut u8, *const u8, u64) -> c_int;
type StFin = unsafe extern "C" fn(*mut u8, *mut u8) -> c_int;
type HmacInit = unsafe extern "C" fn(*mut u8, *const u8, usize) -> c_int;
type PolyInit = unsafe extern "C" fn(*mut u8, *const u8) -> c_int;
type Gh = unsafe extern "C" fn(*mut u8, usize, *const u8, u64, *const u8, usize) -> c_int;
type GhSp =
    unsafe extern "C" fn(*mut u8, usize, *const u8, u64, *const u8, usize, *const u8, *const u8)
        -> c_int;
type GhInit = unsafe extern "C" fn(*mut u8, *const u8, usize, usize) -> c_int;
type GhInitSp =
    unsafe extern "C" fn(*mut u8, *const u8, usize, usize, *const u8, *const u8) -> c_int;
type GhFin = unsafe extern "C" fn(*mut u8, *mut u8, usize) -> c_int;
type XofOneShot = unsafe extern "C" fn(*mut u8, usize, *const u8, u64) -> c_int;
type XofInitDom = unsafe extern "C" fn(*mut u8, u8) -> c_int;
type XofSqueeze = unsafe extern "C" fn(*mut u8, *mut u8, usize) -> c_int;
type KdfDerive = unsafe extern "C" fn(*mut u8, usize, u64, *const c_char, *const u8) -> c_int;
type HkdfExtract = unsafe extern "C" fn(*mut u8, *const u8, usize, *const u8, usize) -> c_int;
type HkdfExtractInit = unsafe extern "C" fn(*mut u8, *const u8, usize) -> c_int;
type HkdfExtractUpd = unsafe extern "C" fn(*mut u8, *const u8, usize) -> c_int;
type HkdfExtractFin = unsafe extern "C" fn(*mut u8, *mut u8) -> c_int;
type HkdfExpand = unsafe extern "C" fn(*mut u8, usize, *const c_char, usize, *const u8) -> c_int;
type KecInit = unsafe extern "C" fn(*mut u8);
type KecXor = unsafe extern "C" fn(*mut u8, *const u8, usize, usize);
type KecExtract = unsafe extern "C" fn(*const u8, *mut u8, usize, usize);

// internal blake2b API (exported as `_sodium_blake2b*`); note that `outlen`
// and `keylen` are `uint8_t` there.
type B2Init = unsafe extern "C" fn(*mut u8, u8) -> c_int;
type B2InitSp = unsafe extern "C" fn(*mut u8, u8, *const c_void, *const c_void) -> c_int;
type B2InitKey = unsafe extern "C" fn(*mut u8, u8, *const c_void, u8) -> c_int;
type B2InitKeySp =
    unsafe extern "C" fn(*mut u8, u8, *const c_void, u8, *const c_void, *const c_void) -> c_int;
type B2Final = unsafe extern "C" fn(*mut u8, *mut u8, u8) -> c_int;
type B2 = unsafe extern "C" fn(*mut u8, *const c_void, *const c_void, u8, u64, u8) -> c_int;
type B2Sp = unsafe extern "C" fn(
    *mut u8,
    *const c_void,
    *const c_void,
    u8,
    u64,
    u8,
    *const c_void,
    *const c_void,
) -> c_int;

macro_rules! pair2 {
    ($t:ty, $n:expr) => {{
        let (a, b) = pair::<$t>($n);
        [*a, *b]
    }};
}

/// A 64-byte-aligned, zero-filled opaque state buffer.
struct St {
    p: *mut u8,
    n: usize,
}

impl St {
    fn new(n: usize) -> St {
        assert!(n > 0);
        let layout = std::alloc::Layout::from_size_align(n, 64).unwrap();
        let p = unsafe { std::alloc::alloc_zeroed(layout) };
        assert!(!p.is_null());
        St { p, n }
    }
    fn ptr(&self) -> *mut u8 {
        self.p
    }
    fn bytes(&self) -> &[u8] {
        unsafe { std::slice::from_raw_parts(self.p, self.n) }
    }
}

impl Drop for St {
    fn drop(&mut self) {
        unsafe {
            std::alloc::dealloc(
                self.p,
                std::alloc::Layout::from_size_align(self.n, 64).unwrap(),
            )
        }
    }
}

fn sz2(name: &str) -> usize {
    unsafe {
        let f = pair2!(Sz, name);
        let a = f[0]();
        let b = f[1]();
        assert_eq!(a, b, "{name}(): C={a} Rust={b}");
        a
    }
}

/// Call `f` in both libraries with `errno` zeroed and compare return + errno.
#[track_caller]
fn cmp_errno(ctx: &str, mut call: impl FnMut(usize) -> c_int) -> (c_int, i32) {
    set_errno(0);
    let rc = call(0);
    let ec = errno();
    set_errno(0);
    let rr = call(1);
    let er = errno();
    assert_eq!(rc, rr, "{ctx}: return value differs (C={rc}, Rust={rr})");
    assert_eq!(ec, er, "{ctx}: errno differs (C={ec}, Rust={er})");
    (rc, ec)
}

// ===========================================================================
// child-process driver for the abort rows
// ===========================================================================

/// 384-byte blake2b state (the public opaque size); the internal
/// `blake2b_state` is 361 bytes packed, so this is always large enough.
const B2_STATE: usize = 384;

#[test]
fn zz_abort_child() {
    let Some((case, is_c)) = child_case() else {
        return; // normal test run: nothing to do
    };
    let l = libs();
    let h = if is_c { l.c } else { l.rs };
    macro_rules! sym {
        ($t:ty, $n:expr) => {{
            let mut nm = String::from($n);
            nm.push('\0');
            let s: libloading::Symbol<$t> = unsafe { h.get(nm.as_bytes()) }.unwrap();
            *s
        }};
    }
    unsafe {
        let key = [7u8; 64];
        let msg = [3u8; 100];
        let salt = [1u8; 16];
        let personal = [2u8; 16];
        let mut out = [0u8; 512];

        // helper: a state that has been through init(32)+update
        let ready_state = |outlen: usize| -> St {
            let st = St::new(B2_STATE);
            let init = sym!(GhInit, "crypto_generichash_blake2b_init");
            let upd = sym!(StUpd, "crypto_generichash_blake2b_update");
            assert_eq!(init(st.ptr(), ptr::null(), 0, outlen), 0);
            assert_eq!(upd(st.ptr(), msg.as_ptr(), 100), 0);
            st
        };

        match case.as_str() {
            // -------- rows 299-303: crypto_generichash*_final bad outlen ----
            c if c.starts_with("ghfin_") => {
                let mut it = c.split('_');
                it.next();
                let which = it.next().unwrap(); // "b2" | "wrap"
                let outlen: usize = it.next().unwrap().parse().unwrap();
                let f = if which == "b2" {
                    sym!(GhFin, "crypto_generichash_blake2b_final")
                } else {
                    sym!(GhFin, "crypto_generichash_final")
                };
                let st = ready_state(32);
                let r = f(st.ptr(), out.as_mut_ptr(), outlen);
                std::process::exit(if r == 0 { 0 } else { 3 });
            }

            // -------- rows 307/308: blake2b_init ----------------------------
            c if c.starts_with("b2init_") => {
                let outlen: u8 = c.rsplit('_').next().unwrap().parse().unwrap();
                let f = sym!(B2Init, "_sodium_blake2b_init");
                let st = St::new(B2_STATE);
                let r = f(st.ptr(), outlen);
                std::process::exit(if r == 0 { 0 } else { 3 });
            }
            // -------- rows 309/310: blake2b_init_salt_personal --------------
            c if c.starts_with("b2initsp_") => {
                let outlen: u8 = c.rsplit('_').next().unwrap().parse().unwrap();
                let f = sym!(B2InitSp, "_sodium_blake2b_init_salt_personal");
                let st = St::new(B2_STATE);
                let r = f(
                    st.ptr(),
                    outlen,
                    salt.as_ptr() as *const c_void,
                    personal.as_ptr() as *const c_void,
                );
                std::process::exit(if r == 0 { 0 } else { 3 });
            }
            // -------- rows 311-315: blake2b_init_key ------------------------
            c if c.starts_with("b2initkey_") => {
                let mut it = c.split('_');
                it.next(); // "b2initkey"
                let outlen: u8 = it.next().unwrap().parse().unwrap();
                let keylen: u8 = it.next().unwrap().parse().unwrap();
                let knull: bool = it.next().unwrap() == "null";
                let f = sym!(B2InitKey, "_sodium_blake2b_init_key");
                let st = St::new(B2_STATE);
                let kp = if knull {
                    ptr::null()
                } else {
                    key.as_ptr() as *const c_void
                };
                let r = f(st.ptr(), outlen, kp, keylen);
                std::process::exit(if r == 0 { 0 } else { 3 });
            }
            // -------- rows 317-321: blake2b_init_key_salt_personal ---------
            c if c.starts_with("b2initkeysp_") => {
                let mut it = c.split('_');
                it.next(); // "b2initkeysp"
                let outlen: u8 = it.next().unwrap().parse().unwrap();
                let keylen: u8 = it.next().unwrap().parse().unwrap();
                let knull: bool = it.next().unwrap() == "null";
                let f = sym!(B2InitKeySp, "_sodium_blake2b_init_key_salt_personal");
                let st = St::new(B2_STATE);
                let kp = if knull {
                    ptr::null()
                } else {
                    key.as_ptr() as *const c_void
                };
                let r = f(
                    st.ptr(),
                    outlen,
                    kp,
                    keylen,
                    salt.as_ptr() as *const c_void,
                    personal.as_ptr() as *const c_void,
                );
                std::process::exit(if r == 0 { 0 } else { 3 });
            }
            // -------- rows 323-328: blake2b (simple API) --------------------
            c if c.starts_with("b2simple_") => {
                let mut it = c.split('_');
                it.next();
                let what = it.next().unwrap();
                let f = sym!(B2, "_sodium_blake2b");
                let (op, ip, kp, outlen, inlen, keylen): (
                    *mut u8,
                    *const c_void,
                    *const c_void,
                    u8,
                    u64,
                    u8,
                ) = match what {
                    // row 323
                    "innull" => (
                        out.as_mut_ptr(),
                        ptr::null(),
                        ptr::null(),
                        32,
                        1,
                        0,
                    ),
                    // row 324
                    "outnull" => (
                        ptr::null_mut(),
                        msg.as_ptr() as *const c_void,
                        ptr::null(),
                        32,
                        100,
                        0,
                    ),
                    // row 325
                    "outlen0" => (
                        out.as_mut_ptr(),
                        msg.as_ptr() as *const c_void,
                        ptr::null(),
                        0,
                        100,
                        0,
                    ),
                    // row 326
                    "outlen65" => (
                        out.as_mut_ptr(),
                        msg.as_ptr() as *const c_void,
                        ptr::null(),
                        65,
                        100,
                        0,
                    ),
                    "outlen255" => (
                        out.as_mut_ptr(),
                        msg.as_ptr() as *const c_void,
                        ptr::null(),
                        255,
                        100,
                        0,
                    ),
                    // row 327
                    "keynull" => (
                        out.as_mut_ptr(),
                        msg.as_ptr() as *const c_void,
                        ptr::null(),
                        32,
                        100,
                        16,
                    ),
                    // row 328
                    "keylen65" => (
                        out.as_mut_ptr(),
                        msg.as_ptr() as *const c_void,
                        key.as_ptr() as *const c_void,
                        32,
                        100,
                        65,
                    ),
                    _ => unreachable!(),
                };
                let r = f(op, ip, kp, outlen, inlen, keylen);
                std::process::exit(if r == 0 { 0 } else { 3 });
            }
            // -------- rows 330-335: blake2b_salt_personal ------------------
            c if c.starts_with("b2spsimple_") => {
                let mut it = c.split('_');
                it.next();
                let what = it.next().unwrap();
                let f = sym!(B2Sp, "_sodium_blake2b_salt_personal");
                let (op, ip, kp, outlen, inlen, keylen): (
                    *mut u8,
                    *const c_void,
                    *const c_void,
                    u8,
                    u64,
                    u8,
                ) = match what {
                    "innull" => (out.as_mut_ptr(), ptr::null(), ptr::null(), 32, 1, 0),
                    "outnull" => (
                        ptr::null_mut(),
                        msg.as_ptr() as *const c_void,
                        ptr::null(),
                        32,
                        100,
                        0,
                    ),
                    "outlen0" => (
                        out.as_mut_ptr(),
                        msg.as_ptr() as *const c_void,
                        ptr::null(),
                        0,
                        100,
                        0,
                    ),
                    "outlen65" => (
                        out.as_mut_ptr(),
                        msg.as_ptr() as *const c_void,
                        ptr::null(),
                        65,
                        100,
                        0,
                    ),
                    "keynull" => (
                        out.as_mut_ptr(),
                        msg.as_ptr() as *const c_void,
                        ptr::null(),
                        32,
                        100,
                        16,
                    ),
                    "keylen65" => (
                        out.as_mut_ptr(),
                        msg.as_ptr() as *const c_void,
                        key.as_ptr() as *const c_void,
                        32,
                        100,
                        65,
                    ),
                    _ => unreachable!(),
                };
                let r = f(
                    op,
                    ip,
                    kp,
                    outlen,
                    inlen,
                    keylen,
                    salt.as_ptr() as *const c_void,
                    personal.as_ptr() as *const c_void,
                );
                std::process::exit(if r == 0 { 0 } else { 3 });
            }
            // -------- row 305: blake2b_final on a finalized state ----------
            "b2final_twice" => {
                let f = sym!(B2Final, "_sodium_blake2b_final");
                let st = ready_state(32);
                let r1 = f(st.ptr(), out.as_mut_ptr(), 32);
                let r2 = f(st.ptr(), out.as_mut_ptr(), 32);
                std::process::exit(((r1 & 7) | ((r2 & 7) << 3)) as i32 & 0x3f);
            }

            // -------- row 346: crypto_kdf_blake2b_derive_from_key, key NULL -
            "kdf_key_null" => {
                let f = sym!(KdfDerive, "crypto_kdf_blake2b_derive_from_key");
                let ctx = *b"ctx01234";
                let r = f(
                    out.as_mut_ptr(),
                    32,
                    1,
                    ctx.as_ptr() as *const c_char,
                    ptr::null(),
                );
                std::process::exit(if r == 0 { 0 } else { 3 });
            }

            // -------- rows 353-356: hkdf extract_init with salt == NULL -----
            c if c.starts_with("hkdf_") => {
                let mut it = c.split('_');
                it.next();
                let which = it.next().unwrap(); // "256" | "512"
                let salt_len: usize = it.next().unwrap().parse().unwrap();
                let f = if which == "256" {
                    sym!(HkdfExtractInit, "crypto_kdf_hkdf_sha256_extract_init")
                } else {
                    sym!(HkdfExtractInit, "crypto_kdf_hkdf_sha512_extract_init")
                };
                // ROOMY: the largest hkdf state is sizeof(hmacsha512_state)=416
                let st = St::new(1024);
                let r = f(st.ptr(), ptr::null(), salt_len);
                std::process::exit(if r == 0 { 0 } else { 3 });
            }

            // -------- rows 382-386: hmac init with key == NULL --------------
            c if c.starts_with("hmac_") => {
                let mut it = c.split('_');
                it.next();
                let which = it.next().unwrap(); // 256 | 512 | 512256
                let keylen: usize = it.next().unwrap().parse().unwrap();
                let f = match which {
                    "256" => sym!(HmacInit, "crypto_auth_hmacsha256_init"),
                    "512" => sym!(HmacInit, "crypto_auth_hmacsha512_init"),
                    "512256" => sym!(HmacInit, "crypto_auth_hmacsha512256_init"),
                    _ => unreachable!(),
                };
                // ROOMY: sizeof(crypto_auth_hmacsha512_state) == 416
                let st = St::new(1024);
                let r = f(st.ptr(), ptr::null(), keylen);
                std::process::exit(if r == 0 { 0 } else { 3 });
            }

            other => panic!("unknown abort case {other:?}"),
        }
    }
}

// ===========================================================================
// generichash / blake2b rejection paths (return -1)
// ===========================================================================

/// ERRORS.md rows 282-284, 287-297, 340-342: every `-1` rejection of the
/// generichash family, plus the three documented NON-rejections.
///
/// Row 285 (`inlen > UINT64_MAX`) is a dead branch — `inlen` is already
/// `unsigned long long`, so it can never be taken and is not testable.
/// Rows 286, 298, 316, 322, 329, 336 are `LCOV_EXCL`/unreachable inner
/// failure paths (`blake2b_init*` only ever returns 0 or aborts), and rows
/// 337/338 are `COMPILER_ASSERT`s: none of them are reachable at runtime.
#[test]
fn g3_err_282_297_340_342_generichash_bounds() {
    unsafe {
        let b2 = pair2!(Gh, "crypto_generichash_blake2b");
        let gw = pair2!(Gh, "crypto_generichash");
        let sp = pair2!(GhSp, "crypto_generichash_blake2b_salt_personal");
        let b2i = pair2!(GhInit, "crypto_generichash_blake2b_init");
        let gwi = pair2!(GhInit, "crypto_generichash_init");
        let spi = pair2!(GhInitSp, "crypto_generichash_blake2b_init_salt_personal");
        let nstate = sz2("crypto_generichash_blake2b_statebytes");
        let mut rng = Rng::new(SEED);
        let msg = rng.bytes(100);
        let key = rng.bytes(300);
        let salt = rng.bytes(16);
        let personal = rng.bytes(16);

        // rows 282/283/287/288/289/291/292/294/295/296: bad outlen
        let bad_outlens: [usize; 8] = [0, 65, 66, 128, 255, 256, 1000, usize::MAX];
        for &outlen in &bad_outlens {
            for keylen in [0usize, 32] {
                let kp = if keylen == 0 {
                    ptr::null()
                } else {
                    key.as_ptr()
                };
                // one-shot: `out` is never dereferenced, so NULL is legal here
                // (generic boundary check: NULL where C does not deref).
                let mut guard = [vec![0xAAu8; 96], vec![0xAAu8; 96]];
                let mut r = [0i32; 4];
                for i in 0..2 {
                    r[i] = b2[i](
                        guard[i].as_mut_ptr(),
                        outlen,
                        msg.as_ptr(),
                        100,
                        kp,
                        keylen,
                    );
                    r[2 + i] = gw[i](ptr::null_mut(), outlen, msg.as_ptr(), 100, kp, keylen);
                }
                let ctx = format!("rows282/283/287 outlen={outlen} keylen={keylen}");
                eq_i32(&ctx, r[0], r[1]);
                eq_i32(&format!("{ctx} wrapper+NULL out"), r[2], r[3]);
                assert_eq!(r[0], -1, "{ctx}: C did not reject");
                assert_eq!(r[2], -1, "{ctx}: wrapper did not reject");
                // row 282: `out` untouched
                assert!(
                    guard[0].iter().all(|&b| b == 0xAA),
                    "{ctx}: C wrote to out on rejection"
                );
                assert!(
                    guard[1].iter().all(|&b| b == 0xAA),
                    "{ctx}: Rust wrote to out on rejection"
                );

                // rows 288/289: _salt_personal
                let mut r2 = [0i32; 2];
                for i in 0..2 {
                    r2[i] = sp[i](
                        ptr::null_mut(),
                        outlen,
                        msg.as_ptr(),
                        100,
                        kp,
                        keylen,
                        salt.as_ptr(),
                        personal.as_ptr(),
                    );
                }
                eq_i32(&format!("rows288/289 salt_personal outlen={outlen}"), r2[0], r2[1]);
                assert_eq!(r2[0], -1);

                // rows 291/292/294/295/296: the init entry points. The state
                // is left uninitialised, so we only compare return values.
                let mut r3 = [0i32; 6];
                for i in 0..2 {
                    let st = St::new(nstate);
                    r3[i] = b2i[i](st.ptr(), kp, keylen, outlen);
                    let st2 = St::new(nstate);
                    r3[2 + i] = gwi[i](st2.ptr(), kp, keylen, outlen);
                    let st3 = St::new(nstate);
                    r3[4 + i] = spi[i](
                        st3.ptr(),
                        kp,
                        keylen,
                        outlen,
                        salt.as_ptr(),
                        personal.as_ptr(),
                    );
                }
                eq_i32(&format!("rows291/292 _init outlen={outlen}"), r3[0], r3[1]);
                eq_i32(&format!("row294 _init wrapper outlen={outlen}"), r3[2], r3[3]);
                eq_i32(&format!("rows295/296 _init_sp outlen={outlen}"), r3[4], r3[5]);
                assert_eq!((r3[0], r3[2], r3[4]), (-1, -1, -1), "C accepted outlen={outlen}");
            }
        }

        // rows 284/290/293/297: keylen > 64
        for &keylen in &[65usize, 66, 100, 255, 256, 300] {
            let mut r = [0i32; 8];
            for i in 0..2 {
                r[i] = b2[i](ptr::null_mut(), 32, msg.as_ptr(), 100, key.as_ptr(), keylen);
                r[2 + i] = gw[i](ptr::null_mut(), 32, msg.as_ptr(), 100, key.as_ptr(), keylen);
                r[4 + i] = sp[i](
                    ptr::null_mut(),
                    32,
                    msg.as_ptr(),
                    100,
                    key.as_ptr(),
                    keylen,
                    salt.as_ptr(),
                    personal.as_ptr(),
                );
                let st = St::new(nstate);
                r[6 + i] = b2i[i](st.ptr(), key.as_ptr(), keylen, 32);
            }
            let ctx = format!("rows284/290/293 keylen={keylen}");
            eq_i32(&ctx, r[0], r[1]);
            eq_i32(&format!("{ctx} wrapper"), r[2], r[3]);
            eq_i32(&format!("{ctx} salt_personal"), r[4], r[5]);
            eq_i32(&format!("{ctx} init"), r[6], r[7]);
            assert_eq!((r[0], r[2], r[4], r[6]), (-1, -1, -1, -1), "{ctx}: C accepted");
            // rows 293/297: _init/_init_sp
            let mut r2 = [0i32; 4];
            for i in 0..2 {
                let st = St::new(nstate);
                r2[i] = gwi[i](st.ptr(), key.as_ptr(), keylen, 32);
                let st2 = St::new(nstate);
                r2[2 + i] = spi[i](
                    st2.ptr(),
                    key.as_ptr(),
                    keylen,
                    32,
                    salt.as_ptr(),
                    personal.as_ptr(),
                );
            }
            eq_i32(&format!("row294 keylen={keylen}"), r2[0], r2[1]);
            eq_i32(&format!("row297 keylen={keylen}"), r2[2], r2[3]);
            assert_eq!((r2[0], r2[2]), (-1, -1));
        }

        // --- rows 340/341/342: documented NON-rejections -------------------
        // 340: key != NULL, keylen == 0 -> unkeyed digest, returns 0
        // 341: 0 < keylen < 16 accepted (KEYBYTES_MIN not enforced)
        // 342: 0 < outlen < 16 accepted (BYTES_MIN not enforced)
        for keylen in [0usize, 1, 2, 8, 15] {
            for outlen in [1usize, 2, 8, 15] {
                let mut o = [vec![0u8; outlen], vec![0u8; outlen]];
                let mut r = [0i32; 2];
                for i in 0..2 {
                    r[i] = b2[i](
                        o[i].as_mut_ptr(),
                        outlen,
                        msg.as_ptr(),
                        100,
                        key.as_ptr(),
                        keylen,
                    );
                }
                let ctx = format!("rows340-342 keylen={keylen} outlen={outlen}");
                eq_i32(&ctx, r[0], r[1]);
                eq_i32(&ctx, r[0], 0);
                eq_bytes(&ctx, &o[0], &o[1]);
            }
            // row 340: keylen == 0 with a non-NULL key == the unkeyed digest
            if keylen == 0 {
                for i in 0..2 {
                    let mut a = vec![0u8; 32];
                    let mut b = vec![0u8; 32];
                    assert_eq!(
                        b2[i](a.as_mut_ptr(), 32, msg.as_ptr(), 100, key.as_ptr(), 0),
                        0
                    );
                    assert_eq!(b2[i](b.as_mut_ptr(), 32, msg.as_ptr(), 100, ptr::null(), 0), 0);
                    eq_bytes(&format!("row340 lib{i}"), &a, &b);
                }
            }
        }
    }
}

/// ERRORS.md rows 299-305: `crypto_generichash*_final` misuse.
///
/// Rows 299/300/301/302: `outlen == 0`, `65..=255`, and `>= 256` all end in
/// `sodium_misuse()` / a failed `assert(outlen <= UINT8_MAX)` — the process
/// dies. Row 303's "silently truncated" behaviour is NDEBUG-only; this C build
/// has assertions enabled (see the module comment), so 288 aborts too and the
/// test asserts C/Rust agreement instead.
/// Row 304 (`final` outlen != `init` outlen) and row 305 (`final` twice) are
/// return-value rows and are checked in-process.
/// Row 306 is an internal invariant `assert` that cannot be violated from
/// outside the library.
#[test]
fn g3_err_299_305_generichash_final_misuse() {
    // rows 299/300/301/302/303 — abort paths
    for which in ["b2", "wrap"] {
        for outlen in [0usize, 65, 100, 255, 256, 288, 512] {
            let t = diff_abort_case(&format!("ghfin_{which}_{outlen}"));
            assert!(
                t.signal.is_some(),
                "rows299-303 {which} outlen={outlen}: expected both children to die \
                 from a signal, got {t:?}"
            );
        }
        // control: a legal outlen exits cleanly in both
        let t = diff_abort_case(&format!("ghfin_{which}_64"));
        assert_eq!(t.code, Some(0), "control ghfin_{which}_64: {t:?}");
    }

    unsafe {
        let init = pair2!(GhInit, "crypto_generichash_blake2b_init");
        let upd = pair2!(StUpd, "crypto_generichash_blake2b_update");
        let fin = pair2!(GhFin, "crypto_generichash_blake2b_final");
        let nstate = sz2("crypto_generichash_blake2b_statebytes");
        let mut rng = Rng::new(SEED ^ 0x299);
        let msg = rng.bytes(200);

        // --- row 304: final's outlen is never validated against init's -----
        for (init_len, fin_len) in [(32usize, 64usize), (64, 32), (16, 64), (32, 1)] {
            let mut o = [vec![0xAAu8; fin_len], vec![0xAAu8; fin_len]];
            let mut r = [0i32; 2];
            for i in 0..2 {
                let st = St::new(nstate);
                assert_eq!(init[i](st.ptr(), ptr::null(), 0, init_len), 0);
                assert_eq!(upd[i](st.ptr(), msg.as_ptr(), 200), 0);
                r[i] = fin[i](st.ptr(), o[i].as_mut_ptr(), fin_len);
            }
            let ctx = format!("row304 init={init_len} final={fin_len}");
            eq_i32(&ctx, r[0], r[1]);
            assert_eq!(r[0], 0, "{ctx}: C rejected the mismatch (it should not)");
            eq_bytes(&ctx, &o[0], &o[1]);
        }

        // --- row 305: _final twice / _update+_final after _final -----------
        for extra_update in [false, true] {
            let mut o1 = [vec![0u8; 32], vec![0u8; 32]];
            let mut o2 = [vec![0xAAu8; 32], vec![0xAAu8; 32]];
            let mut r = [0i32; 4];
            for i in 0..2 {
                let st = St::new(nstate);
                assert_eq!(init[i](st.ptr(), ptr::null(), 0, 32), 0);
                assert_eq!(upd[i](st.ptr(), msg.as_ptr(), 200), 0);
                assert_eq!(fin[i](st.ptr(), o1[i].as_mut_ptr(), 32), 0);
                if extra_update {
                    r[2 + i] = upd[i](st.ptr(), msg.as_ptr(), 200);
                }
                r[i] = fin[i](st.ptr(), o2[i].as_mut_ptr(), 32);
            }
            let ctx = format!("row305 final twice extra_update={extra_update}");
            eq_i32(&ctx, r[0], r[1]);
            eq_i32(&format!("{ctx} update-after-final"), r[2], r[3]);
            assert_eq!(r[0], -1, "{ctx}: C did not report the finalized state");
            eq_bytes(&format!("{ctx} first digest"), &o1[0], &o1[1]);
            eq_bytes(&format!("{ctx} second buffer"), &o2[0], &o2[1]);
        }
    }
}

/// ERRORS.md rows 307-336: the internal `_sodium_blake2b*` entry points, whose
/// parameter validation is `sodium_misuse()` (process death) rather than -1.
///
/// Rows 316/322/329/336 are `LCOV_EXCL_LINE` inner-failure branches that
/// `blake2b_init*` can never take (it returns 0 or aborts), and row 337
/// (`COMPILER_ASSERT(sizeof *P == 64)`) is compile-time only: none of them are
/// reachable at runtime.
#[test]
fn g3_err_307_336_blake2b_internal_misuse() {
    // rows 307/308: blake2b_init
    for outlen in [0u8, 65, 66, 128, 255] {
        let t = diff_abort_case(&format!("b2init_{outlen}"));
        assert!(t.signal.is_some(), "row307/308 outlen={outlen}: {t:?}");
    }
    assert_eq!(diff_abort_case("b2init_64").code, Some(0));
    assert_eq!(diff_abort_case("b2init_1").code, Some(0));

    // rows 309/310: blake2b_init_salt_personal
    for outlen in [0u8, 65, 255] {
        let t = diff_abort_case(&format!("b2initsp_{outlen}"));
        assert!(t.signal.is_some(), "row309/310 outlen={outlen}: {t:?}");
    }
    assert_eq!(diff_abort_case("b2initsp_64").code, Some(0));

    // rows 311-315: blake2b_init_key(outlen, key, keylen)
    for (outlen, keylen, knull, row) in [
        (0u8, 32u8, false, "311"),   // outlen == 0
        (65, 32, false, "312"),      // outlen > 64
        (255, 32, false, "312"),
        (32, 32, true, "313"),       // key == NULL
        (32, 0, false, "314"),       // keylen == 0
        (32, 65, false, "315"),      // keylen > 64
        (32, 255, false, "315"),
    ] {
        let case = format!("b2initkey_{outlen}_{keylen}_{}", if knull { "null" } else { "ok" });
        let t = diff_abort_case(&case);
        assert!(t.signal.is_some(), "row{row} {case}: {t:?}");
    }
    assert_eq!(diff_abort_case("b2initkey_32_64_ok").code, Some(0));
    assert_eq!(diff_abort_case("b2initkey_64_1_ok").code, Some(0));

    // rows 317-321: blake2b_init_key_salt_personal
    for (outlen, keylen, knull, row) in [
        (0u8, 32u8, false, "317"),
        (65, 32, false, "318"),
        (32, 32, true, "319"),
        (32, 0, false, "320"),
        (32, 65, false, "321"),
    ] {
        let case = format!(
            "b2initkeysp_{outlen}_{keylen}_{}",
            if knull { "null" } else { "ok" }
        );
        let t = diff_abort_case(&case);
        assert!(t.signal.is_some(), "row{row} {case}: {t:?}");
    }
    assert_eq!(diff_abort_case("b2initkeysp_32_64_ok").code, Some(0));

    // rows 323-328: blake2b (simple API)
    for what in [
        "innull", "outnull", "outlen0", "outlen65", "outlen255", "keynull", "keylen65",
    ] {
        let t = diff_abort_case(&format!("b2simple_{what}"));
        assert!(t.signal.is_some(), "rows323-328 {what}: {t:?}");
    }

    // rows 330-335: blake2b_salt_personal (simple API)
    for what in ["innull", "outnull", "outlen0", "outlen65", "keynull", "keylen65"] {
        let t = diff_abort_case(&format!("b2spsimple_{what}"));
        assert!(t.signal.is_some(), "rows330-335 {what}: {t:?}");
    }

    // row 305 through the internal entry point: _final on a finalized state
    // must return -1 (not abort) in both libraries.
    let t = diff_abort_case("b2final_twice");
    assert_eq!(
        t.signal, None,
        "row305 _sodium_blake2b_final twice must not abort: {t:?}"
    );
    // exit code encodes (r1 & 7) | ((r2 & 7) << 3); r1 == 0, r2 == -1 -> 7
    assert_eq!(t.code, Some(0b111_000), "row305 encoded returns: {t:?}");
}

// ===========================================================================
// crypto_kdf
// ===========================================================================

/// ERRORS.md rows 343-348 (blake2b kdf) and 349-352, 357-360 (hkdf).
///
/// Row 345 (`ctx == NULL`, dereferenced by `memcpy` before any bound check)
/// and row 351 (`ctx == NULL && ctx_len > 0`) are genuine NULL dereferences,
/// i.e. undefined behaviour with no observable rejection; they are documented
/// here but deliberately not executed.
#[test]
fn g3_err_343_360_kdf() {
    unsafe {
        let d = pair2!(KdfDerive, "crypto_kdf_blake2b_derive_from_key");
        let w = pair2!(KdfDerive, "crypto_kdf_derive_from_key");
        let mut rng = Rng::new(SEED ^ 0x343);
        let key = rng.bytes(32);
        let ctx8 = *b"ctx01234";

        // rows 343/344/347: subkey_len out of [16, 64] -> errno EINVAL, -1
        for &len in &[0usize, 1, 2, 15, 65, 66, 128, 1000, usize::MAX] {
            let mut guard = [vec![0xAAu8; 96], vec![0xAAu8; 96]];
            let (rc, ec) = cmp_errno(&format!("rows343/344 subkey_len={len}"), |i| {
                d[i](
                    guard[i].as_mut_ptr(),
                    len,
                    3,
                    ctx8.as_ptr() as *const c_char,
                    key.as_ptr(),
                )
            });
            assert_eq!(rc, -1, "rows343/344 subkey_len={len}: C did not reject");
            assert_eq!(ec, EINVAL, "rows343/344 subkey_len={len}: errno");
            assert!(
                guard[0].iter().all(|&b| b == 0xAA) && guard[1].iter().all(|&b| b == 0xAA),
                "rows343/344 subkey_len={len}: subkey buffer was written"
            );
            let (rc, ec) = cmp_errno(&format!("row347 wrapper subkey_len={len}"), |i| {
                w[i](
                    ptr::null_mut(),
                    len,
                    3,
                    ctx8.as_ptr() as *const c_char,
                    key.as_ptr(),
                )
            });
            assert_eq!(rc, -1);
            assert_eq!(ec, EINVAL);
        }

        // rows 16/64 boundaries are valid — one step inside the range
        for &len in &[16usize, 17, 63, 64] {
            let mut o = [vec![0u8; len], vec![0u8; len]];
            let (rc, _) = cmp_errno(&format!("kdf boundary subkey_len={len}"), |i| {
                d[i](
                    o[i].as_mut_ptr(),
                    len,
                    3,
                    ctx8.as_ptr() as *const c_char,
                    key.as_ptr(),
                )
            });
            assert_eq!(rc, 0, "kdf subkey_len={len} must be accepted");
            eq_bytes(&format!("kdf boundary subkey_len={len}"), &o[0], &o[1]);
        }

        // row 348: subkey_id has no bound check at all
        for &id in &[0u64, 1, u64::MAX, u64::MAX - 1, 1 << 63] {
            let mut o = [vec![0u8; 32], vec![0u8; 32]];
            let mut r = [0i32; 2];
            for i in 0..2 {
                r[i] = d[i](
                    o[i].as_mut_ptr(),
                    32,
                    id,
                    ctx8.as_ptr() as *const c_char,
                    key.as_ptr(),
                );
            }
            eq_i32(&format!("row348 subkey_id={id:#x}"), r[0], r[1]);
            assert_eq!(r[0], 0, "row348 subkey_id={id:#x}: rejected");
            eq_bytes(&format!("row348 subkey_id={id:#x}"), &o[0], &o[1]);
        }

        // row 346: key == NULL -> sodium_misuse() inside
        // blake2b_init_key_salt_personal (keylen is 32, key is NULL)
        let t = diff_abort_case("kdf_key_null");
        assert!(t.signal.is_some(), "row346: expected abort, got {t:?}");

        // rows 349/350: hkdf expand out_len > BYTES_MAX -> errno EINVAL, -1
        for (fam, keybytes, maxv) in [
            ("crypto_kdf_hkdf_sha256", 32usize, 8160usize),
            ("crypto_kdf_hkdf_sha512", 64, 16320),
        ] {
            let expand = pair2!(HkdfExpand, &format!("{fam}_expand"));
            let prk = rng.bytes(keybytes);
            for &ol in &[maxv + 1, maxv + 2, maxv * 2, usize::MAX] {
                let mut guard = [vec![0xAAu8; 64], vec![0xAAu8; 64]];
                let (rc, ec) = cmp_errno(&format!("rows349/350 {fam} out_len={ol}"), |i| {
                    expand[i](
                        guard[i].as_mut_ptr(),
                        ol,
                        ptr::null(),
                        0,
                        prk.as_ptr(),
                    )
                });
                assert_eq!(rc, -1, "rows349/350 {fam} out_len={ol}: not rejected");
                assert_eq!(ec, EINVAL, "rows349/350 {fam} out_len={ol}: errno");
                assert!(
                    guard[0].iter().all(|&b| b == 0xAA) && guard[1].iter().all(|&b| b == 0xAA),
                    "rows349/350 {fam}: buffer written on rejection"
                );
            }
            // one step inside the range must be accepted
            for &ol in &[maxv - 1, maxv] {
                let mut o = [vec![0u8; ol], vec![0u8; ol]];
                let (rc, _) = cmp_errno(&format!("{fam} out_len={ol}"), |i| {
                    expand[i](o[i].as_mut_ptr(), ol, ptr::null(), 0, prk.as_ptr())
                });
                assert_eq!(rc, 0, "{fam} out_len={ol} must be accepted");
                eq_bytes(&format!("{fam} out_len={ol}"), &o[0], &o[1]);
            }
            // row 352: ctx_len has no upper bound
            for &cl in &[0usize, 1, 1 << 16, 1 << 18] {
                let cbuf = vec![0x5Au8; cl.max(1)];
                let cp = if cl == 0 {
                    ptr::null()
                } else {
                    cbuf.as_ptr() as *const c_char
                };
                let mut o = [vec![0u8; 40], vec![0u8; 40]];
                let mut r = [0i32; 2];
                for i in 0..2 {
                    r[i] = expand[i](o[i].as_mut_ptr(), 40, cp, cl, prk.as_ptr());
                }
                eq_i32(&format!("row352 {fam} ctx_len={cl}"), r[0], r[1]);
                assert_eq!(r[0], 0);
                eq_bytes(&format!("row352 {fam} ctx_len={cl}"), &o[0], &o[1]);
            }
            // generic boundary: out_len == 0 writes nothing and returns 0
            let mut z = [vec![0xAAu8; 8], vec![0xAAu8; 8]];
            let mut r = [0i32; 2];
            for i in 0..2 {
                r[i] = expand[i](z[i].as_mut_ptr(), 0, ptr::null(), 0, prk.as_ptr());
            }
            eq_i32(&format!("{fam} out_len=0"), r[0], r[1]);
            assert_eq!(r[0], 0);
            assert!(z[0].iter().all(|&b| b == 0xAA) && z[1].iter().all(|&b| b == 0xAA));

            // rows 357/358/359: streaming extract never rejects
            let einit = pair2!(HkdfExtractInit, &format!("{fam}_extract_init"));
            let eupd = pair2!(HkdfExtractUpd, &format!("{fam}_extract_update"));
            let efin = pair2!(HkdfExtractFin, &format!("{fam}_extract_final"));
            let extract = pair2!(HkdfExtract, &format!("{fam}_extract"));
            let nstate = sz2(&format!("{fam}_statebytes"));
            let ikm = rng.bytes(50);
            for &sl in &[0usize, 1, 32] {
                let salt = rng.bytes(sl.max(1));
                let sptr = if sl == 0 { ptr::null() } else { salt.as_ptr() };
                let mut prk2 = [vec![0xAAu8; keybytes], vec![0xAAu8; keybytes]];
                let mut rets: [Vec<c_int>; 2] = [Vec::new(), Vec::new()];
                let mut st_after: [Vec<u8>; 2] = [Vec::new(), Vec::new()];
                for i in 0..2 {
                    let st = St::new(nstate);
                    rets[i].push(einit[i](st.ptr(), sptr, sl));
                    // row 357: `ikm == NULL, ikm_len == 0` is accepted
                    rets[i].push(eupd[i](st.ptr(), ptr::null(), 0));
                    rets[i].push(eupd[i](st.ptr(), ikm.as_ptr(), 50));
                    rets[i].push(efin[i](st.ptr(), prk2[i].as_mut_ptr()));
                    // row 460/481: _extract_final zeroizes the whole state
                    st_after[i] = st.bytes().to_vec();
                    // rows 358/359: calling _extract_final again still returns 0
                    let mut again = vec![0u8; keybytes];
                    rets[i].push(efin[i](st.ptr(), again.as_mut_ptr()));
                }
                let ctx = format!("rows357-359 {fam} salt_len={sl}");
                assert_eq!(rets[0], rets[1], "{ctx}: return values differ");
                assert!(rets[0].iter().all(|&r| r == 0), "{ctx}: C returned {:?}", rets[0]);
                eq_bytes(&ctx, &prk2[0], &prk2[1]);
                assert!(
                    st_after[0].iter().all(|&b| b == 0),
                    "{ctx}: C did not zeroize the state"
                );
                assert!(
                    st_after[1].iter().all(|&b| b == 0),
                    "{ctx}: Rust did not zeroize the state"
                );
                // row 359: the one-shot _extract has no rejection path either
                let mut p = [vec![0u8; keybytes], vec![0u8; keybytes]];
                let mut r2 = [0i32; 2];
                for i in 0..2 {
                    r2[i] = extract[i](p[i].as_mut_ptr(), sptr, sl, ikm.as_ptr(), 50);
                }
                eq_i32(&format!("row359 {fam} salt_len={sl}"), r2[0], r2[1]);
                assert_eq!(r2[0], 0);
                eq_bytes(&format!("row359 {fam} salt_len={sl}"), &p[0], &p[1]);
            }
        }

        // rows 353/354: extract_init with salt == NULL and 0 < salt_len <= block
        for (which, lens) in [("256", [1usize, 2, 32, 64]), ("512", [1, 2, 64, 128])] {
            for sl in lens {
                let t = diff_abort_case(&format!("hkdf_{which}_{sl}"));
                assert!(
                    t.signal.is_some(),
                    "rows353/354 hkdf_{which} salt_len={sl}: expected abort, got {t:?}"
                );
            }
        }
        // rows 355/356: salt == NULL with salt_len > block takes the
        // key-pre-hashing branch first and dereferences NULL. Both libraries
        // must die the same way (SIGSEGV in practice).
        for (which, sl) in [("256", 65usize), ("256", 200), ("512", 129), ("512", 300)] {
            let t = diff_abort_case(&format!("hkdf_{which}_{sl}"));
            assert!(
                t.signal.is_some(),
                "rows355/356 hkdf_{which} salt_len={sl}: expected a fatal signal, got {t:?}"
            );
        }
        // control: salt == NULL with salt_len == 0 is legal
        assert_eq!(diff_abort_case("hkdf_256_0").code, Some(0));
        assert_eq!(diff_abort_case("hkdf_512_0").code, Some(0));

        // row 360: the keygen entry points return void and cannot fail
        install_det_random();
        for (name, n) in [
            ("crypto_kdf_hkdf_sha256_keygen", 32usize),
            ("crypto_kdf_hkdf_sha512_keygen", 64),
        ] {
            let kg = pair2!(Keygen, name);
            det_reseed(0x3333_4444_5555_6666);
            let mut a = vec![0xAAu8; n + 8];
            kg[0](a.as_mut_ptr());
            det_reseed(0x3333_4444_5555_6666);
            let mut b = vec![0xAAu8; n + 8];
            kg[1](b.as_mut_ptr());
            eq_bytes(&format!("row360 {name}"), &a, &b);
            assert_eq!(&a[n..], &[0xAAu8; 8], "row360 {name}: overrun");
        }
    }
}

// ===========================================================================
// crypto_hash sha2 / sha3
// ===========================================================================

/// ERRORS.md rows 361-371: the SHA-2 and SHA-3 families.
///
/// Rows 364 (`in == NULL && inlen > 0`) and 370 (`state == NULL`) are
/// `nonnull` violations that the C never checks — real undefined behaviour, so
/// they are documented but not executed. Row 371 is a `COMPILER_ASSERT`.
#[test]
fn g3_err_361_371_sha2_sha3() {
    unsafe {
        // row 369: crypto_hash_sha3_384 must not exist in either library
        for name in [
            "crypto_hash_sha3_384",
            "crypto_hash_sha3384",
            "crypto_hash_sha3_384_init",
        ] {
            assert!(
                !has_sym(name),
                "row369: {name} must not be exported by libsodium 1.0.23"
            );
        }

        let mut rng = Rng::new(SEED ^ 0x361);
        let msg = rng.bytes(200);

        // --- rows 361/362/363/365: sha256 / sha512 / crypto_hash -----------
        for (fam, outlen) in [
            ("crypto_hash_sha256", 32usize),
            ("crypto_hash_sha512", 64),
        ] {
            let one = pair2!(OneShot, fam);
            let init = pair2!(StInit, &format!("{fam}_init"));
            let upd = pair2!(StUpd, &format!("{fam}_update"));
            let fin = pair2!(StFin, &format!("{fam}_final"));
            let nstate = sz2(&format!("{fam}_statebytes"));

            let mut rets: [Vec<c_int>; 2] = [Vec::new(), Vec::new()];
            let mut d1 = [vec![0u8; outlen], vec![0u8; outlen]];
            let mut d2 = [vec![0u8; outlen], vec![0u8; outlen]];
            let mut d3 = [vec![0u8; outlen], vec![0u8; outlen]];
            for i in 0..2 {
                // row 361/362: nothing ever rejects
                rets[i].push(one[i](d1[i].as_mut_ptr(), msg.as_ptr(), 200));
                let st = St::new(nstate);
                rets[i].push(init[i](st.ptr()));
                // row 363: in == NULL with inlen == 0 is accepted
                rets[i].push(upd[i](st.ptr(), ptr::null(), 0));
                rets[i].push(upd[i](st.ptr(), msg.as_ptr(), 200));
                rets[i].push(fin[i](st.ptr(), d2[i].as_mut_ptr()));
                // row 365: _final twice is undetected and returns 0 again
                rets[i].push(fin[i](st.ptr(), d3[i].as_mut_ptr()));
            }
            let ctx = format!("rows361-365 {fam}");
            assert_eq!(rets[0], rets[1], "{ctx}: return values differ");
            assert!(rets[0].iter().all(|&r| r == 0), "{ctx}: C returned {:?}", rets[0]);
            eq_bytes(&format!("{ctx} one-shot"), &d1[0], &d1[1]);
            eq_bytes(&format!("{ctx} streaming"), &d2[0], &d2[1]);
            eq_bytes(&format!("row365 {fam} second _final"), &d3[0], &d3[1]);
        }
        // row 362: the crypto_hash alias has no rejection path either
        let ch = pair2!(OneShot, "crypto_hash");
        let mut o = [vec![0u8; 64], vec![0u8; 64]];
        let mut r = [0i32; 2];
        for i in 0..2 {
            r[i] = ch[i](o[i].as_mut_ptr(), ptr::null(), 0);
        }
        eq_i32("row362 crypto_hash empty", r[0], r[1]);
        assert_eq!(r[0], 0);
        eq_bytes("row362 crypto_hash empty", &o[0], &o[1]);

        // --- rows 366/367/368: sha3 phase misuse ---------------------------
        for (fam, outlen) in [
            ("crypto_hash_sha3256", 32usize),
            ("crypto_hash_sha3512", 64),
        ] {
            let one = pair2!(OneShot, fam);
            let init = pair2!(StInit, &format!("{fam}_init"));
            let upd = pair2!(StUpd, &format!("{fam}_update"));
            let fin = pair2!(StFin, &format!("{fam}_final"));
            let nstate = sz2(&format!("{fam}_statebytes"));

            let mut rets: [Vec<c_int>; 2] = [Vec::new(), Vec::new()];
            let mut d: [Vec<Vec<u8>>; 2] = [Vec::new(), Vec::new()];
            for i in 0..2 {
                let st = St::new(nstate);
                rets[i].push(init[i](st.ptr()));
                rets[i].push(upd[i](st.ptr(), msg.as_ptr(), 200));
                let mut a = vec![0u8; outlen];
                rets[i].push(fin[i](st.ptr(), a.as_mut_ptr()));
                d[i].push(a);
                // row 367: _final again -> -1, still writes outlen bytes
                let mut b = vec![0xAAu8; outlen];
                rets[i].push(fin[i](st.ptr(), b.as_mut_ptr()));
                d[i].push(b);
                // row 366: _update after _final -> -1 but still absorbs
                rets[i].push(upd[i](st.ptr(), msg.as_ptr(), 137));
                let mut c = vec![0u8; outlen];
                rets[i].push(fin[i](st.ptr(), c.as_mut_ptr()));
                d[i].push(c);
                // row 366 again with a zero-length update after _final
                let st2 = St::new(nstate);
                init[i](st2.ptr());
                upd[i](st2.ptr(), msg.as_ptr(), 50);
                let mut e = vec![0u8; outlen];
                fin[i](st2.ptr(), e.as_mut_ptr());
                rets[i].push(upd[i](st2.ptr(), ptr::null(), 0));
                let mut g = vec![0u8; outlen];
                rets[i].push(fin[i](st2.ptr(), g.as_mut_ptr()));
                d[i].push(e);
                d[i].push(g);
                // row 368: the one-shot never rejects
                let mut z = vec![0u8; outlen];
                rets[i].push(one[i](z.as_mut_ptr(), ptr::null(), 0));
                d[i].push(z);
            }
            let ctx = format!("rows366-368 {fam}");
            assert_eq!(rets[0], rets[1], "{ctx}: return values differ");
            // init 0, update 0, final 0, final-again -1, update-after-final
            // -1 (the state is silently reset to ABSORBING and the input IS
            // absorbed), the following final 0, then the same pattern with a
            // zero-length update, then the one-shot 0.
            assert_eq!(
                rets[0],
                vec![0, 0, 0, -1, -1, 0, -1, 0, 0],
                "{ctx}: unexpected C return sequence"
            );
            for k in 0..d[0].len() {
                eq_bytes(&format!("{ctx} digest#{k}"), &d[0][k], &d[1][k]);
            }
            // row 366: the post-final update really was absorbed (the digest
            // after it differs from the plain double-final digest)
            assert_ne!(d[0][1], d[0][2], "row366 {fam}: input was not absorbed");
        }
    }
}

// ===========================================================================
// crypto_xof
// ===========================================================================

/// ERRORS.md rows 372-378: XOF phase misuse, the absent squeeze rejection
/// path, and the unvalidated `domain` byte (an out-of-range flag int crossing
/// the FFI boundary).
///
/// Rows 379 (`COMPILER_ASSERT`) and 380 (`state`/`out` `nonnull` violations)
/// have no runtime behaviour to compare.
#[test]
fn g3_err_372_378_xof() {
    unsafe {
        let mut rng = Rng::new(SEED ^ 0x372);
        for (fam, rate) in [
            ("crypto_xof_shake128", 168usize),
            ("crypto_xof_shake256", 136),
            ("crypto_xof_turboshake128", 168),
            ("crypto_xof_turboshake256", 136),
        ] {
            let one = pair2!(XofOneShot, fam);
            let init = pair2!(StInit, &format!("{fam}_init"));
            let initd = pair2!(XofInitDom, &format!("{fam}_init_with_domain"));
            let upd = pair2!(StUpd, &format!("{fam}_update"));
            let sq = pair2!(XofSqueeze, &format!("{fam}_squeeze"));
            let nstate = sz2(&format!("{fam}_statebytes"));
            let msg = rng.bytes(400);

            // --- rows 372-375: _update after _squeeze -> -1, still absorbs --
            let mut rets: [Vec<c_int>; 2] = [Vec::new(), Vec::new()];
            let mut outs: [Vec<Vec<u8>>; 2] = [Vec::new(), Vec::new()];
            for i in 0..2 {
                let st = St::new(nstate);
                rets[i].push(init[i](st.ptr()));
                rets[i].push(upd[i](st.ptr(), msg.as_ptr(), 100));
                let mut a = vec![0u8; 64];
                rets[i].push(sq[i](st.ptr(), a.as_mut_ptr(), 64));
                outs[i].push(a);
                // update after squeeze
                rets[i].push(upd[i](st.ptr(), msg.as_ptr(), 200));
                let mut b = vec![0u8; 64];
                rets[i].push(sq[i](st.ptr(), b.as_mut_ptr(), 64));
                outs[i].push(b);
                // zero-length update after squeeze also reports -1
                rets[i].push(upd[i](st.ptr(), ptr::null(), 0));
                let mut c = vec![0u8; 64];
                rets[i].push(sq[i](st.ptr(), c.as_mut_ptr(), 64));
                outs[i].push(c);
            }
            let ctx = format!("rows372-375 {fam}");
            assert_eq!(rets[0], rets[1], "{ctx}: return values differ");
            assert_eq!(
                rets[0],
                vec![0, 0, 0, -1, 0, -1, 0],
                "{ctx}: unexpected C return sequence"
            );
            for k in 0..outs[0].len() {
                eq_bytes(&format!("{ctx} squeeze#{k}"), &outs[0][k], &outs[1][k]);
            }
            assert_ne!(
                outs[0][0], outs[0][1],
                "rows372-375 {fam}: the post-squeeze update was not absorbed"
            );

            // --- row 376: squeeze has no rejection path --------------------
            let mut rets: [Vec<c_int>; 2] = [Vec::new(), Vec::new()];
            let mut big: [Vec<u8>; 2] = [vec![0u8; 4096], vec![0u8; 4096]];
            let mut zero: [Vec<u8>; 2] = [vec![0xAAu8; 8], vec![0xAAu8; 8]];
            for i in 0..2 {
                let st = St::new(nstate);
                rets[i].push(init[i](st.ptr()));
                // squeeze before any update
                rets[i].push(sq[i](st.ptr(), big[i].as_mut_ptr(), 4096));
                // outlen == 0 is a no-op
                rets[i].push(sq[i](st.ptr(), zero[i].as_mut_ptr(), 0));
                rets[i].push(sq[i](st.ptr(), zero[i].as_mut_ptr(), 0));
            }
            let ctx = format!("row376 {fam}");
            assert_eq!(rets[0], rets[1], "{ctx}: return values differ");
            assert!(rets[0].iter().all(|&r| r == 0), "{ctx}: C returned {:?}", rets[0]);
            eq_bytes(&format!("{ctx} 4096-byte squeeze"), &big[0], &big[1]);
            assert!(
                zero[0].iter().all(|&b| b == 0xAA) && zero[1].iter().all(|&b| b == 0xAA),
                "{ctx}: outlen==0 squeeze wrote bytes"
            );

            // --- row 377: the one-shot never rejects ----------------------
            for (outlen, inlen) in [(0usize, 0usize), (0, 100), (4096, 0), (1, 400)] {
                let mut o = [vec![0xAAu8; outlen + 4], vec![0xAAu8; outlen + 4]];
                let ip = if inlen == 0 { ptr::null() } else { msg.as_ptr() };
                let mut r = [0i32; 2];
                for i in 0..2 {
                    r[i] = one[i](o[i].as_mut_ptr(), outlen, ip, inlen as u64);
                }
                let c = format!("row377 {fam} outlen={outlen} inlen={inlen}");
                eq_i32(&c, r[0], r[1]);
                assert_eq!(r[0], 0);
                eq_bytes(&c, &o[0], &o[1]);
            }

            // --- row 378: `domain` is never validated ---------------------
            // 0x00 and 0x80..=0xFF are accepted; 0x80 XORs to 0x00 in the
            // single-byte-pad special case (`offset == rate - 1`).
            for domain in [0u8, 0x80, 0x81, 0xFE, 0xFF] {
                for &inlen in &[0usize, 1, rate - 1, rate, 2 * rate - 1] {
                    let ip = if inlen == 0 { ptr::null() } else { msg.as_ptr() };
                    let mut o = [vec![0u8; 96], vec![0u8; 96]];
                    let mut r = [0i32; 2];
                    for i in 0..2 {
                        let st = St::new(nstate);
                        r[i] = initd[i](st.ptr(), domain);
                        assert_eq!(upd[i](st.ptr(), ip, inlen as u64), 0);
                        assert_eq!(sq[i](st.ptr(), o[i].as_mut_ptr(), 96), 0);
                    }
                    let c = format!("row378 {fam} domain={domain:#04x} inlen={inlen}");
                    eq_i32(&c, r[0], r[1]);
                    assert_eq!(r[0], 0, "{c}: C rejected the domain byte");
                    eq_bytes(&c, &o[0], &o[1]);
                }
            }
        }
    }
}

// ===========================================================================
// crypto_core_keccak1600
// ===========================================================================

/// ERRORS.md row 381: the keccak1600 primitives return `void` and perform no
/// bound check at all on `offset` / `length`.
///
/// Writing past the end of the 1600-bit lane array is undefined behaviour, so
/// we only step past the *documented* 200-byte rate window while staying
/// inside the 224-byte `crypto_core_keccak1600_state` allocation, and require
/// C and Rust to agree byte-for-byte on the resulting state.
#[test]
fn g3_err_381_keccak1600_no_bounds_check() {
    unsafe {
        let nstate = sz2("crypto_core_keccak1600_statebytes");
        assert!(nstate >= 208, "keccak1600 statebytes = {nstate}");
        let init = pair2!(KecInit, "crypto_core_keccak1600_init");
        let xorb = pair2!(KecXor, "crypto_core_keccak1600_xor_bytes");
        let extr = pair2!(KecExtract, "crypto_core_keccak1600_extract_bytes");
        let mut rng = Rng::new(SEED ^ 0x381);
        let data = rng.bytes(64);

        // (offset, length) pairs: exactly at the rate, zero length, and one
        // step past 200 while remaining inside the state allocation.
        let mut spans: Vec<(usize, usize)> = vec![
            (0, 0),
            (200, 0),
            (0, 200),
            (199, 1),
            (195, 5),
            (199, 2),   // one byte past the 200-byte lane array
            (200, 1),   // wholly past it
            (200, 8),
            (nstate - 1, 1),
        ];
        spans.retain(|&(o, l)| o + l <= nstate);
        for &(off, len) in &spans {
            let mut states: [Vec<u8>; 2] = [Vec::new(), Vec::new()];
            let mut got: [Vec<u8>; 2] = [vec![0xAAu8; 200], vec![0xAAu8; 200]];
            for i in 0..2 {
                let st = St::new(nstate);
                init[i](st.ptr());
                xorb[i](st.ptr(), data.as_ptr(), off, len);
                extr[i](st.ptr(), got[i].as_mut_ptr(), 0, 200);
                states[i] = st.bytes().to_vec();
            }
            let ctx = format!("row381 xor off={off} len={len}");
            eq_bytes(&format!("{ctx} extracted"), &got[0], &got[1]);
            // The C stores the lanes as a plain little-endian byte array, so
            // the raw state bytes are directly comparable.
            eq_bytes(&format!("{ctx} raw state"), &states[0], &states[1]);
        }

        // extract_bytes with the same shapes: also unchecked
        for &(off, len) in &spans {
            let mut e = [vec![0xAAu8; len + 4], vec![0xAAu8; len + 4]];
            for i in 0..2 {
                let st = St::new(nstate);
                init[i](st.ptr());
                xorb[i](st.ptr(), data.as_ptr(), 0, 64);
                extr[i](st.ptr(), e[i].as_mut_ptr(), off, len);
            }
            eq_bytes(&format!("row381 extract off={off} len={len}"), &e[0], &e[1]);
            assert_eq!(&e[0][len..], &[0xAAu8; 4], "row381: extract overran");
        }
    }
}

// ===========================================================================
// crypto_auth (HMAC)
// ===========================================================================

/// ERRORS.md rows 382-395: HMAC key/state misuse and the `_verify` rejection
/// paths.
///
/// Row 394 (`h` aliasing the library's internal stack buffer `correct`) is
/// unreachable through the public API and is documented only.
#[test]
fn g3_err_382_395_hmac() {
    unsafe {
        let mut rng = Rng::new(SEED ^ 0x382);
        let msg = rng.bytes(120);

        // rows 382/384/386: key == NULL with 0 < keylen <= block -> misuse
        for (which, lens) in [
            ("256", vec![1usize, 2, 32, 63, 64]),
            ("512", vec![1, 2, 64, 127, 128]),
            ("512256", vec![1, 2, 64, 127, 128]),
        ] {
            for keylen in lens {
                let t = diff_abort_case(&format!("hmac_{which}_{keylen}"));
                assert!(
                    t.signal.is_some(),
                    "rows382/384/386 hmacsha{which} keylen={keylen}: expected abort, got {t:?}"
                );
            }
        }
        // rows 383/385: key == NULL with keylen > block skips the NULL check
        // and dereferences NULL; both libraries must die identically.
        for (which, keylen) in [
            ("256", 65usize),
            ("256", 200),
            ("512", 129),
            ("512", 300),
            ("512256", 129),
        ] {
            let t = diff_abort_case(&format!("hmac_{which}_{keylen}"));
            assert!(
                t.signal.is_some(),
                "rows383/385 hmacsha{which} keylen={keylen}: expected a fatal signal, got {t:?}"
            );
        }
        // row 387: key == NULL with keylen == 0 is accepted
        for which in ["256", "512", "512256"] {
            assert_eq!(
                diff_abort_case(&format!("hmac_{which}_0")).code,
                Some(0),
                "row387 hmacsha{which}: keylen 0 with a NULL key must be accepted"
            );
        }

        for (fam, outlen) in [
            ("crypto_auth_hmacsha256", 32usize),
            ("crypto_auth_hmacsha512", 64),
            ("crypto_auth_hmacsha512256", 32),
        ] {
            let one = pair2!(OneShotK, fam);
            let ver = pair2!(VerifyK, &format!("{fam}_verify"));
            let init = pair2!(HmacInit, &format!("{fam}_init"));
            let upd = pair2!(StUpd, &format!("{fam}_update"));
            let fin = pair2!(StFin, &format!("{fam}_final"));
            let nstate = sz2(&format!("{fam}_statebytes"));
            let key = rng.bytes(32);

            // rows 387/388/389/390: no validation anywhere
            let mut rets: [Vec<c_int>; 2] = [Vec::new(), Vec::new()];
            let mut d1 = [vec![0u8; outlen], vec![0u8; outlen]];
            let mut d2 = [vec![0u8; outlen], vec![0u8; outlen]];
            let mut d3 = [vec![0u8; outlen], vec![0u8; outlen]];
            for i in 0..2 {
                let st = St::new(nstate);
                // row 387: NULL key, keylen 0
                rets[i].push(init[i](st.ptr(), ptr::null(), 0));
                // row 388: no validation, incl. in == NULL with inlen == 0
                rets[i].push(upd[i](st.ptr(), ptr::null(), 0));
                rets[i].push(upd[i](st.ptr(), msg.as_ptr(), 120));
                rets[i].push(fin[i](st.ptr(), d1[i].as_mut_ptr()));
                // row 389: _final twice is undetected
                rets[i].push(fin[i](st.ptr(), d2[i].as_mut_ptr()));
                // row 390: the one-shot never rejects
                rets[i].push(one[i](d3[i].as_mut_ptr(), ptr::null(), 0, key.as_ptr()));
            }
            let ctx = format!("rows387-390 {fam}");
            assert_eq!(rets[0], rets[1], "{ctx}: return values differ");
            assert!(rets[0].iter().all(|&r| r == 0), "{ctx}: C returned {:?}", rets[0]);
            eq_bytes(&format!("{ctx} first MAC"), &d1[0], &d1[1]);
            eq_bytes(&format!("row389 {fam} second _final"), &d2[0], &d2[1]);
            eq_bytes(&format!("row390 {fam} one-shot empty"), &d3[0], &d3[1]);

            // rows 391/392/393/395: every single-bit / single-byte difference
            // in `h` must be rejected with -1.
            let mut mac = [vec![0u8; outlen], vec![0u8; outlen]];
            for i in 0..2 {
                assert_eq!(
                    one[i](mac[i].as_mut_ptr(), msg.as_ptr(), 120, key.as_ptr()),
                    0
                );
            }
            eq_bytes(&format!("{fam} base MAC"), &mac[0], &mac[1]);
            for pos in 0..outlen {
                for bit in [0u8, 3, 7] {
                    let mut h = mac[0].clone();
                    h[pos] ^= 1 << bit;
                    let mut r = [0i32; 2];
                    for i in 0..2 {
                        r[i] = ver[i](h.as_ptr(), msg.as_ptr(), 120, key.as_ptr());
                    }
                    let c = format!("rows391-395 {fam} flip byte{pos} bit{bit}");
                    eq_i32(&c, r[0], r[1]);
                    assert_eq!(r[0], -1, "{c}: C accepted a corrupted MAC");
                }
            }
            // all-zero and all-0xFF `h`
            for h in [vec![0u8; outlen], vec![0xFFu8; outlen]] {
                let mut r = [0i32; 2];
                for i in 0..2 {
                    r[i] = ver[i](h.as_ptr(), msg.as_ptr(), 120, key.as_ptr());
                }
                eq_i32(&format!("row395 {fam} degenerate h"), r[0], r[1]);
                assert_eq!(r[0], -1);
            }
            // correct MAC, wrong key / wrong message
            let key2 = rng.bytes(32);
            let mut msg2 = msg.clone();
            msg2[0] ^= 1;
            for (label, m, k) in [
                ("wrong key", &msg, &key2),
                ("wrong message", &msg2, &key),
            ] {
                let mut r = [0i32; 2];
                for i in 0..2 {
                    r[i] = ver[i](mac[0].as_ptr(), m.as_ptr(), m.len() as u64, k.as_ptr());
                }
                eq_i32(&format!("rows391-393 {fam} {label}"), r[0], r[1]);
                assert_eq!(r[0], -1);
            }
            // sanity: the correct MAC still verifies
            for i in 0..2 {
                assert_eq!(
                    ver[i](mac[0].as_ptr(), msg.as_ptr(), 120, key.as_ptr()),
                    0,
                    "{fam} lib{i}: correct MAC rejected"
                );
            }
        }

        // row 393: the crypto_auth alias
        let averify = pair2!(VerifyK, "crypto_auth_verify");
        let auth = pair2!(OneShotK, "crypto_auth");
        let key = rng.bytes(32);
        let mut mac = [vec![0u8; 32], vec![0u8; 32]];
        for i in 0..2 {
            assert_eq!(auth[i](mac[i].as_mut_ptr(), msg.as_ptr(), 120, key.as_ptr()), 0);
        }
        eq_bytes("row393 crypto_auth MAC", &mac[0], &mac[1]);
        for pos in 0..32usize {
            let mut h = mac[0].clone();
            h[pos] ^= 0x40;
            let mut r = [0i32; 2];
            for i in 0..2 {
                r[i] = averify[i](h.as_ptr(), msg.as_ptr(), 120, key.as_ptr());
            }
            eq_i32(&format!("row393 crypto_auth_verify flip byte{pos}"), r[0], r[1]);
            assert_eq!(r[0], -1);
        }
    }
}

// ===========================================================================
// crypto_onetimeauth (poly1305)
// ===========================================================================

/// ERRORS.md rows 396-398: poly1305 has exactly one rejection path
/// (`_verify`); everything else always returns 0. Row 398 is a
/// `COMPILER_ASSERT` and has no runtime behaviour.
#[test]
fn g3_err_396_398_poly1305() {
    unsafe {
        let mut rng = Rng::new(SEED ^ 0x396);
        let msg = rng.bytes(80);
        let key = rng.bytes(32);
        let key2 = rng.bytes(32);
        for fam in ["crypto_onetimeauth_poly1305", "crypto_onetimeauth"] {
            let one = pair2!(OneShotK, fam);
            let ver = pair2!(VerifyK, &format!("{fam}_verify"));
            let init = pair2!(PolyInit, &format!("{fam}_init"));
            let upd = pair2!(StUpd, &format!("{fam}_update"));
            let fin = pair2!(StFin, &format!("{fam}_final"));
            let nstate = sz2(&format!("{fam}_statebytes"));

            // row 397: no NULL check, no length check, no state-phase guard
            let mut rets: [Vec<c_int>; 2] = [Vec::new(), Vec::new()];
            let mut t1 = [vec![0u8; 16], vec![0u8; 16]];
            let mut t2 = [vec![0u8; 16], vec![0u8; 16]];
            let mut t3 = [vec![0u8; 16], vec![0u8; 16]];
            for i in 0..2 {
                let st = St::new(nstate);
                rets[i].push(init[i](st.ptr(), key.as_ptr()));
                rets[i].push(upd[i](st.ptr(), ptr::null(), 0));
                rets[i].push(upd[i](st.ptr(), msg.as_ptr(), 80));
                rets[i].push(fin[i](st.ptr(), t1[i].as_mut_ptr()));
                // _final twice: undetected, returns 0
                rets[i].push(fin[i](st.ptr(), t2[i].as_mut_ptr()));
                // one-shot on an empty message
                rets[i].push(one[i](t3[i].as_mut_ptr(), ptr::null(), 0, key.as_ptr()));
            }
            let ctx = format!("row397 {fam}");
            assert_eq!(rets[0], rets[1], "{ctx}: return values differ");
            assert!(rets[0].iter().all(|&r| r == 0), "{ctx}: C returned {:?}", rets[0]);
            eq_bytes(&format!("{ctx} tag"), &t1[0], &t1[1]);
            eq_bytes(&format!("{ctx} second _final"), &t2[0], &t2[1]);
            eq_bytes(&format!("{ctx} empty one-shot"), &t3[0], &t3[1]);

            // row 396: every corrupted tag must be rejected
            let mut tag = [vec![0u8; 16], vec![0u8; 16]];
            for i in 0..2 {
                assert_eq!(one[i](tag[i].as_mut_ptr(), msg.as_ptr(), 80, key.as_ptr()), 0);
            }
            eq_bytes(&format!("row396 {fam} base tag"), &tag[0], &tag[1]);
            for pos in 0..16usize {
                for bit in [0u8, 4, 7] {
                    let mut h = tag[0].clone();
                    h[pos] ^= 1 << bit;
                    let mut r = [0i32; 2];
                    for i in 0..2 {
                        r[i] = ver[i](h.as_ptr(), msg.as_ptr(), 80, key.as_ptr());
                    }
                    let c = format!("row396 {fam} flip byte{pos} bit{bit}");
                    eq_i32(&c, r[0], r[1]);
                    assert_eq!(r[0], -1, "{c}: C accepted a corrupted tag");
                }
            }
            for h in [vec![0u8; 16], vec![0xFFu8; 16]] {
                let mut r = [0i32; 2];
                for i in 0..2 {
                    r[i] = ver[i](h.as_ptr(), msg.as_ptr(), 80, key.as_ptr());
                }
                eq_i32(&format!("row396 {fam} degenerate h"), r[0], r[1]);
                assert_eq!(r[0], -1);
            }
            let mut r = [0i32; 2];
            for i in 0..2 {
                r[i] = ver[i](tag[0].as_ptr(), msg.as_ptr(), 80, key2.as_ptr());
            }
            eq_i32(&format!("row396 {fam} wrong key"), r[0], r[1]);
            assert_eq!(r[0], -1);
            for i in 0..2 {
                assert_eq!(ver[i](tag[0].as_ptr(), msg.as_ptr(), 80, key.as_ptr()), 0);
            }
        }
    }
}

// ===========================================================================
// crypto_shorthash
// ===========================================================================

/// ERRORS.md rows 399-401: `crypto_shorthash_siphash24` / `_siphashx24` /
/// `crypto_shorthash` have no rejection paths, and `in == NULL, inlen == 0` is
/// safe. Row 402 (`in == NULL && inlen > 0`) is a real NULL dereference and is
/// documented only.
#[test]
fn g3_err_399_401_shorthash() {
    unsafe {
        let mut rng = Rng::new(SEED ^ 0x399);
        let key = rng.bytes(16);
        let msg = rng.bytes(64);
        for (fam, outlen) in [
            ("crypto_shorthash_siphash24", 8usize),
            ("crypto_shorthash_siphashx24", 16),
            ("crypto_shorthash", 8),
        ] {
            let f = pair2!(OneShotK, fam);
            // row 401: in == NULL, inlen == 0
            for (ip, inlen) in [
                (ptr::null(), 0usize),
                (msg.as_ptr(), 0),
                (msg.as_ptr(), 1),
                (msg.as_ptr(), 64),
            ] {
                let mut o = [vec![0xAAu8; outlen + 4], vec![0xAAu8; outlen + 4]];
                let mut r = [0i32; 2];
                for i in 0..2 {
                    r[i] = f[i](o[i].as_mut_ptr(), ip, inlen as u64, key.as_ptr());
                }
                let ctx = format!("rows399-401 {fam} inlen={inlen} null={}", ip.is_null());
                eq_i32(&ctx, r[0], r[1]);
                assert_eq!(r[0], 0, "{ctx}: C rejected");
                eq_bytes(&ctx, &o[0], &o[1]);
                assert_eq!(&o[0][outlen..], &[0xAAu8; 4], "{ctx}: overrun");
            }
            // an all-zero key is accepted as well (no key validation exists)
            let zk = vec![0u8; 16];
            let mut o = [vec![0u8; outlen], vec![0u8; outlen]];
            let mut r = [0i32; 2];
            for i in 0..2 {
                r[i] = f[i](o[i].as_mut_ptr(), msg.as_ptr(), 64, zk.as_ptr());
            }
            eq_i32(&format!("row399 {fam} zero key"), r[0], r[1]);
            assert_eq!(r[0], 0);
            eq_bytes(&format!("row399 {fam} zero key"), &o[0], &o[1]);
        }
    }
}

// ===========================================================================
// pure accessors / void entry points
// ===========================================================================

/// ERRORS.md rows 403-404: `*_statebytes()` accessors have no failure mode,
/// and the `*_keygen()` entry points return `void` (their only failure mode is
/// an internal `randombytes` abort, which the harness's deterministic
/// implementation never triggers).
#[test]
fn g3_err_403_404_accessors_and_keygen() {
    // row 403
    for name in [
        "crypto_generichash_statebytes",
        "crypto_generichash_blake2b_statebytes",
        "crypto_hash_sha256_statebytes",
        "crypto_hash_sha512_statebytes",
        "crypto_hash_sha3256_statebytes",
        "crypto_hash_sha3512_statebytes",
        "crypto_xof_shake128_statebytes",
        "crypto_xof_shake256_statebytes",
        "crypto_xof_turboshake128_statebytes",
        "crypto_xof_turboshake256_statebytes",
        "crypto_auth_hmacsha256_statebytes",
        "crypto_auth_hmacsha512_statebytes",
        "crypto_auth_hmacsha512256_statebytes",
        "crypto_onetimeauth_statebytes",
        "crypto_onetimeauth_poly1305_statebytes",
        "crypto_kdf_hkdf_sha256_statebytes",
        "crypto_kdf_hkdf_sha512_statebytes",
        "crypto_core_keccak1600_statebytes",
    ] {
        let n = sz2(name);
        assert!(n > 0, "{name}() returned 0");
        // idempotent: a second call must give the same value in both libraries
        assert_eq!(n, sz2(name), "{name}() is not a pure accessor");
    }

    // row 404
    install_det_random();
    unsafe {
        for (name, n) in [
            ("crypto_generichash_keygen", 32usize),
            ("crypto_kdf_keygen", 32),
            ("crypto_auth_keygen", 32),
            ("crypto_auth_hmacsha256_keygen", 32),
            ("crypto_auth_hmacsha512_keygen", 32),
            ("crypto_auth_hmacsha512256_keygen", 32),
            ("crypto_onetimeauth_keygen", 32),
            ("crypto_onetimeauth_poly1305_keygen", 32),
            ("crypto_shorthash_keygen", 16),
            ("crypto_kdf_hkdf_sha256_keygen", 32),
            ("crypto_kdf_hkdf_sha512_keygen", 64),
        ] {
            let kg = pair2!(Keygen, name);
            det_reseed(0x0A0B_0C0D_0E0F_1011);
            let mut a = vec![0xAAu8; n + 8];
            kg[0](a.as_mut_ptr());
            det_reseed(0x0A0B_0C0D_0E0F_1011);
            let mut b = vec![0xAAu8; n + 8];
            kg[1](b.as_mut_ptr());
            eq_bytes(&format!("row404 {name}"), &a, &b);
            assert_eq!(&a[n..], &[0xAAu8; 8], "row404 {name}: wrote past {n} bytes");
            assert_ne!(&a[..n], &vec![0xAAu8; n][..], "row404 {name}: wrote nothing");
        }
    }
}

// ===========================================================================
// generic boundary sweep (task requirement, beyond the ERRORS.md rows)
// ===========================================================================

/// Generic boundary checks for group G3: NULL pointers in every position the C
/// provably does not dereference, zero and oversized lengths, and one step
/// past each documented range.
#[test]
fn g3_err_generic_boundaries() {
    unsafe {
        let mut rng = Rng::new(SEED ^ 0xB0);
        let key = rng.bytes(64);
        let prk = rng.bytes(64);

        // --- NULL `in` with inlen == 0 in every streaming update -----------
        for fam in [
            "crypto_hash_sha256",
            "crypto_hash_sha512",
            "crypto_hash_sha3256",
            "crypto_hash_sha3512",
            "crypto_xof_shake128",
            "crypto_xof_shake256",
            "crypto_xof_turboshake128",
            "crypto_xof_turboshake256",
        ] {
            let init = pair2!(StInit, &format!("{fam}_init"));
            let upd = pair2!(StUpd, &format!("{fam}_update"));
            let nstate = sz2(&format!("{fam}_statebytes"));
            let mut r = [0i32; 4];
            for i in 0..2 {
                let st = St::new(nstate);
                r[i] = init[i](st.ptr());
                r[2 + i] = upd[i](st.ptr(), ptr::null(), 0);
            }
            eq_i32(&format!("{fam}_init"), r[0], r[1]);
            eq_i32(&format!("{fam}_update(NULL, 0)"), r[2], r[3]);
        }

        // --- crypto_generichash_blake2b_update(state, NULL, 0) -------------
        {
            let init = pair2!(GhInit, "crypto_generichash_blake2b_init");
            let upd = pair2!(StUpd, "crypto_generichash_blake2b_update");
            let fin = pair2!(GhFin, "crypto_generichash_blake2b_final");
            let nstate = sz2("crypto_generichash_blake2b_statebytes");
            let mut o = [vec![0u8; 32], vec![0u8; 32]];
            let mut r = [0i32; 2];
            for i in 0..2 {
                let st = St::new(nstate);
                assert_eq!(init[i](st.ptr(), ptr::null(), 0, 32), 0);
                r[i] = upd[i](st.ptr(), ptr::null(), 0);
                assert_eq!(fin[i](st.ptr(), o[i].as_mut_ptr(), 32), 0);
            }
            eq_i32("blake2b_update(NULL, 0)", r[0], r[1]);
            eq_bytes("blake2b_update(NULL, 0)", &o[0], &o[1]);
        }

        // --- hkdf: NULL salt with salt_len 0, NULL ctx with ctx_len 0 ------
        for (fam, keybytes) in [
            ("crypto_kdf_hkdf_sha256", 32usize),
            ("crypto_kdf_hkdf_sha512", 64),
        ] {
            let extract = pair2!(HkdfExtract, &format!("{fam}_extract"));
            let expand = pair2!(HkdfExpand, &format!("{fam}_expand"));
            let ikm = rng.bytes(8);
            let mut p = [vec![0u8; keybytes], vec![0u8; keybytes]];
            let mut o = [vec![0u8; 16], vec![0u8; 16]];
            let mut r = [0i32; 4];
            for i in 0..2 {
                r[i] = extract[i](p[i].as_mut_ptr(), ptr::null(), 0, ikm.as_ptr(), 0);
                r[2 + i] = expand[i](o[i].as_mut_ptr(), 16, ptr::null(), 0, prk.as_ptr());
            }
            eq_i32(&format!("{fam}_extract(NULL salt, 0)"), r[0], r[1]);
            eq_i32(&format!("{fam}_expand(NULL ctx, 0)"), r[2], r[3]);
            assert_eq!((r[0], r[2]), (0, 0));
            eq_bytes(&format!("{fam}_extract(NULL salt, 0)"), &p[0], &p[1]);
            eq_bytes(&format!("{fam}_expand(NULL ctx, 0)"), &o[0], &o[1]);
        }

        // --- generichash: every step around the documented bounds ---------
        {
            let b2 = pair2!(Gh, "crypto_generichash_blake2b");
            let msg = rng.bytes(10);
            for outlen in [0usize, 1, 15, 16, 17, 63, 64, 65] {
                for keylen in [0usize, 1, 15, 16, 17, 63, 64, 65] {
                    let mut o = [vec![0xAAu8; outlen.max(1)], vec![0xAAu8; outlen.max(1)]];
                    let mut r = [0i32; 2];
                    for i in 0..2 {
                        r[i] = b2[i](
                            o[i].as_mut_ptr(),
                            outlen,
                            msg.as_ptr(),
                            10,
                            key.as_ptr(),
                            keylen,
                        );
                    }
                    let ctx = format!("boundary blake2b outlen={outlen} keylen={keylen}");
                    eq_i32(&ctx, r[0], r[1]);
                    let expect = if outlen == 0 || outlen > 64 || keylen > 64 {
                        -1
                    } else {
                        0
                    };
                    assert_eq!(r[0], expect, "{ctx}: unexpected C return {}", r[0]);
                    eq_bytes(&ctx, &o[0], &o[1]);
                }
            }
        }

        // --- xof: outlen 0 and a huge inlen boundary ----------------------
        {
            let one = pair2!(XofOneShot, "crypto_xof_shake128");
            let mut o = [vec![0xAAu8; 4], vec![0xAAu8; 4]];
            let mut r = [0i32; 2];
            for i in 0..2 {
                r[i] = one[i](o[i].as_mut_ptr(), 0, ptr::null(), 0);
            }
            eq_i32("shake128 outlen=0 inlen=0", r[0], r[1]);
            assert_eq!(r[0], 0);
            eq_bytes("shake128 outlen=0 inlen=0", &o[0], &o[1]);
        }

        // --- kdf: exact BYTES_MIN/BYTES_MAX and one step outside ----------
        {
            let d = pair2!(KdfDerive, "crypto_kdf_blake2b_derive_from_key");
            let ctx8 = *b"boundary";
            let k = rng.bytes(32);
            for len in [15usize, 16, 64, 65] {
                let mut o = [vec![0xAAu8; len.max(1)], vec![0xAAu8; len.max(1)]];
                let (rc, _) = cmp_errno(&format!("kdf boundary len={len}"), |i| {
                    d[i](
                        o[i].as_mut_ptr(),
                        len,
                        0,
                        ctx8.as_ptr() as *const c_char,
                        k.as_ptr(),
                    )
                });
                let expect = if (16..=64).contains(&len) { 0 } else { -1 };
                assert_eq!(rc, expect, "kdf boundary len={len}");
                if expect == 0 {
                    eq_bytes(&format!("kdf boundary len={len}"), &o[0], &o[1]);
                }
            }
        }
    }
}
