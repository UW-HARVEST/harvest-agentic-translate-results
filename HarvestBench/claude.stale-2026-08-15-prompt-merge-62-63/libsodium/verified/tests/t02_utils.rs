//! t02_utils.rs — C-vs-Rust differential verification of
//!   * `c_src/libsodium/sodium/utils.c`   (pad/unpad/memcmp/compare/is_zero/
//!                                        increment/add/sub/malloc/free/
//!                                        allocarray/mprotect/mlock/memzero)
//!   * `c_src/libsodium/sodium/core.c`    (sodium_init, crit_enter/leave,
//!                                        sodium_misuse, set_misuse_handler)
//!   * `c_src/libsodium/sodium/runtime.c` (the 12 `sodium_runtime_has_*`)
//!   * `c_src/libsodium/sodium/version.c`
//!   * `c_src/libsodium/randombytes/**`
//!
//! Specification: CONFIGS.md rows 30–53 and ERRORS.md rows 48–80.
//! Every call goes through `dlsym` on BOTH shared objects; no Rust function is
//! ever called directly.
#![allow(clippy::needless_range_loop)]

mod common;
use common::*;
use libc::{c_char, c_int, c_void};
use libloading::Library;
use std::ffi::CStr;
use std::path::PathBuf;

// ---------------------------------------------------------------- boilerplate

/// Sentinel used to prefill every output buffer, so writes outside the
/// documented range are caught by comparing the FULL buffer.
const SENT: u8 = 0xAA;
/// Sentinel for `size_t` out-params (rows that must NOT write them).
const SENT_USIZE: usize = 0xDEAD_BEEF_DEAD_BEEF;
/// `GARBAGE_VALUE` from utils.c.
const GARBAGE: u8 = 0xdb;
/// Guard bytes placed before/after every in-place buffer.
const GUARD: usize = 16;

fn clear_errno() {
    unsafe { *libc::__errno_location() = 0 }
}
fn errno() -> c_int {
    unsafe { *libc::__errno_location() }
}

/// Serialises every test that mutates the process-global
/// `randombytes` implementation pointer (cargo runs tests in parallel threads
/// inside ONE process, and both `.so`s are shared by all of them).
static RNG_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
fn rng_lock() -> std::sync::MutexGuard<'static, ()> {
    RNG_LOCK.lock().unwrap_or_else(|e| e.into_inner())
}

type PadFn = unsafe extern "C" fn(*mut usize, *mut u8, usize, usize, usize) -> c_int;
type UnpadFn = unsafe extern "C" fn(*mut usize, *const u8, usize, usize) -> c_int;
type MemcmpFn = unsafe extern "C" fn(*const c_void, *const c_void, usize) -> c_int;
type CompareFn = unsafe extern "C" fn(*const u8, *const u8, usize) -> c_int;
type IsZeroFn = unsafe extern "C" fn(*const u8, usize) -> c_int;
type IncFn = unsafe extern "C" fn(*mut u8, usize);
type AddSubFn = unsafe extern "C" fn(*mut u8, *const u8, usize);
type MallocFn = unsafe extern "C" fn(usize) -> *mut c_void;
type AllocArrayFn = unsafe extern "C" fn(usize, usize) -> *mut c_void;
type FreeFn = unsafe extern "C" fn(*mut c_void);
type MprotectFn = unsafe extern "C" fn(*mut c_void) -> c_int;
type MlockFn = unsafe extern "C" fn(*mut c_void, usize) -> c_int;
type MemzeroFn = unsafe extern "C" fn(*mut c_void, usize);
type StackzeroFn = unsafe extern "C" fn(usize);
type IntFn = unsafe extern "C" fn() -> c_int;
type SizeFn = unsafe extern "C" fn() -> usize;
type NameFn = unsafe extern "C" fn() -> *const c_char;
type VoidFn = unsafe extern "C" fn();
type SetMisuseFn = unsafe extern "C" fn(Option<extern "C" fn()>) -> c_int;
type BufFn = unsafe extern "C" fn(*mut c_void, usize);
type BufDetFn = unsafe extern "C" fn(*mut c_void, usize, *const u8);
type RandomFn = unsafe extern "C" fn() -> u32;
type UniformFn = unsafe extern "C" fn(u32) -> u32;
type SetImplFn = unsafe extern "C" fn(*const RandombytesImpl) -> c_int;
type NaclFn = unsafe extern "C" fn(*mut u8, u64);

/// `(c_fn, rust_fn)` as plain fn pointers (a `Symbol` derefs to the fn itself).
macro_rules! fns {
    ($name:literal, $t:ty) => {{
        let (c, r) = fnpair!($name, $t);
        (*c, *r)
    }};
}

/// Address of an exported DATA symbol. `libloading` reinterprets the stored
/// address as `T`, so a data object must be fetched as `*const T`.
fn data_ptr<T: 'static>(lib: &'static Library, name: &str) -> *const T {
    let s = unsafe { sym::<*const T>(lib, name) };
    *s
}

fn guarded(body: &[u8]) -> Vec<u8> {
    let mut v = vec![SENT; body.len() + 2 * GUARD];
    v[GUARD..GUARD + body.len()].copy_from_slice(body);
    v
}

// ============================================================================
// CONFIGS 30–34 / ERRORS 48–53 — sodium_pad / sodium_unpad
// ============================================================================

#[derive(Debug)]
struct PadOut {
    rc: c_int,
    out: usize,
    buf: Vec<u8>,
    err: c_int,
}

fn call_pad(f: PadFn, buf0: &[u8], ub: usize, bs: usize, maxb: usize, with_out: bool) -> PadOut {
    let mut buf = buf0.to_vec();
    let mut out = SENT_USIZE;
    clear_errno();
    let rc = unsafe {
        f(
            if with_out {
                &mut out as *mut usize
            } else {
                std::ptr::null_mut()
            },
            buf.as_mut_ptr(),
            ub,
            bs,
            maxb,
        )
    };
    let err = errno();
    PadOut { rc, out, buf, err }
}

fn cmp_pad(what: &str, a: &PadOut, b: &PadOut) {
    assert_eq!(a.rc, b.rc, "{what}: sodium_pad return C={} rust={}", a.rc, b.rc);
    assert_eq!(
        a.out, b.out,
        "{what}: *padded_buflen_p C={:#x} rust={:#x}",
        a.out, b.out
    );
    assert_eq!(a.err, b.err, "{what}: errno C={} rust={}", a.err, b.err);
    assert_eq_bytes(&format!("{what}: buf"), &a.buf, &b.buf);
}

#[derive(Debug)]
struct UnpadOut {
    rc: c_int,
    out: usize,
    buf: Vec<u8>,
    err: c_int,
}

fn call_unpad(f: UnpadFn, buf0: &[u8], pbl: usize, bs: usize) -> UnpadOut {
    let buf = buf0.to_vec();
    let mut out = SENT_USIZE;
    clear_errno();
    let rc = unsafe { f(&mut out as *mut usize, buf.as_ptr(), pbl, bs) };
    let err = errno();
    UnpadOut { rc, out, buf, err }
}

fn cmp_unpad(what: &str, a: &UnpadOut, b: &UnpadOut) {
    assert_eq!(a.rc, b.rc, "{what}: sodium_unpad return C={} rust={}", a.rc, b.rc);
    assert_eq!(
        a.out, b.out,
        "{what}: *unpadded_buflen_p C={:#x} rust={:#x}",
        a.out, b.out
    );
    assert_eq!(a.err, b.err, "{what}: errno C={} rust={}", a.err, b.err);
    assert_eq_bytes(&format!("{what}: buf (must be untouched)"), &a.buf, &b.buf);
}

/// The six shapes CONFIGS rows 30/31 ask for.
fn pad_shapes(bs: usize) -> Vec<usize> {
    vec![0, 1, bs.saturating_sub(1), bs, bs + 1, 2 * bs]
}

fn xpadlen_of(ub: usize, bs: usize) -> usize {
    bs - 1 - (ub % bs)
}

fn pad_sweep(tag: &str, blocksizes: &[usize]) {
    init_both();
    let (cf, rf) = fns!("sodium_pad", PadFn);
    let (cu, ru) = fns!("sodium_unpad", UnpadFn);
    let mut rng = Rng::new(SEED);
    let mut cases = 0usize;

    for &bs in blocksizes {
        for &ub in &pad_shapes(bs) {
            for trial in 0..8 {
                let maxb = ub + 2 * bs + 4;
                let mut base = vec![SENT; maxb + 32];
                let data = rng.bytes(ub);
                base[..ub].copy_from_slice(&data);

                let a = call_pad(cf, &base, ub, bs, maxb, true);
                let b = call_pad(rf, &base, ub, bs, maxb, true);
                let what = format!("{tag} sodium_pad bs={bs} unpadded={ub} max={maxb} trial={trial}");
                cmp_pad(&what, &a, &b);
                assert_eq!(a.rc, 0, "{what}: expected success");
                assert_eq!(
                    a.out,
                    ub + xpadlen_of(ub, bs) + 1,
                    "{what}: padded length does not match the C formula"
                );
                // the original data must be preserved verbatim
                assert_eq_bytes(&format!("{what}: prefix"), &data, &a.buf[..ub]);

                // CONFIGS row 33: round-trip through sodium_unpad.
                let ua = call_unpad(cu, &a.buf, a.out, bs);
                let ub_ = call_unpad(ru, &b.buf, b.out, bs);
                let what2 = format!("{tag} roundtrip sodium_unpad bs={bs} padded={} ", a.out);
                cmp_unpad(&what2, &ua, &ub_);
                assert_eq!(ua.rc, 0, "{what2}: round-trip must validate");
                assert_eq!(ua.out, ub, "{what2}: round-trip length {} != {ub}", ua.out);
                cases += 1;
            }
        }
    }
    assert!(cases >= 64, "{tag}: only {cases} randomized cases");
}

/// CONFIGS row 30: power-of-two blocksizes (the `&` path in the C).
#[test]
fn cfg30_pad_power_of_two_blocksizes() {
    pad_sweep("row30", &[1, 2, 16, 256]);
}

/// CONFIGS row 31: non-power-of-two blocksizes (the `%` path in the C).
#[test]
fn cfg31_pad_non_power_of_two_blocksizes() {
    pad_sweep("row31", &[17, 255]);
}

/// CONFIGS row 32: `max_buflen` exactly `*padded_buflen_p` is accepted;
/// `padded_buflen_p == NULL` is accepted. (`max_buflen` one byte short is
/// ERRORS row 50 and is asserted here too.)
#[test]
fn cfg32_pad_max_buflen_exact_and_null_outparam() {
    init_both();
    let (cf, rf) = fns!("sodium_pad", PadFn);
    let mut rng = Rng::new(SEED ^ 0x32);
    let mut cases = 0usize;

    for &bs in &[1usize, 2, 16, 17, 255, 256] {
        for &ub in &pad_shapes(bs) {
            for trial in 0..4 {
                let padded = ub + xpadlen_of(ub, bs) + 1;
                let mut base = vec![SENT; padded + 32];
                let data = rng.bytes(ub);
                base[..ub].copy_from_slice(&data);

                // max_buflen == padded  -> accepted
                let a = call_pad(cf, &base, ub, bs, padded, true);
                let b = call_pad(rf, &base, ub, bs, padded, true);
                let what = format!("row32 exact-max bs={bs} ub={ub} max={padded} t={trial}");
                cmp_pad(&what, &a, &b);
                assert_eq!(a.rc, 0, "{what}: max_buflen == padded must be accepted");
                assert_eq!(a.out, padded, "{what}");

                // padded_buflen_p == NULL -> accepted, nothing else changes
                let a2 = call_pad(cf, &base, ub, bs, padded, false);
                let b2 = call_pad(rf, &base, ub, bs, padded, false);
                let what2 = format!("row32 NULL-outparam bs={bs} ub={ub} t={trial}");
                cmp_pad(&what2, &a2, &b2);
                assert_eq!(a2.rc, 0, "{what2}: NULL padded_buflen_p must be accepted");
                assert_eq!(a2.out, SENT_USIZE, "{what2}: sentinel clobbered");
                assert_eq_bytes(&format!("{what2}: same bytes as with out-param"), &a.buf, &a2.buf);

                // max_buflen == padded-1 -> rejected (ERRORS row 50)
                let a3 = call_pad(cf, &base, ub, bs, padded - 1, true);
                let b3 = call_pad(rf, &base, ub, bs, padded - 1, true);
                let what3 = format!("row32/err50 short-max bs={bs} ub={ub} max={}", padded - 1);
                cmp_pad(&what3, &a3, &b3);
                assert_eq!(a3.rc, -1, "{what3}: xpadded_len >= max_buflen must return -1");
                assert_eq!(a3.out, SENT_USIZE, "{what3}: out-param must NOT be written");
                assert_eq_bytes(&format!("{what3}: buf untouched"), &base, &a3.buf);
                cases += 1;
            }
        }
    }
    assert!(cases >= 64, "row32: only {cases} cases");
}

/// CONFIGS row 33: `padded_buflen` that is NOT a multiple of `blocksize` is
/// legal (the round-trip half of row 33 lives in `pad_sweep`).
#[test]
fn cfg33_unpad_padded_len_not_a_multiple_of_blocksize() {
    init_both();
    let (cu, ru) = fns!("sodium_unpad", UnpadFn);
    let mut rng = Rng::new(SEED ^ 0x33);
    let mut cases = 0usize;

    for &bs in &[1usize, 2, 16, 17, 255, 256] {
        for extra in [1usize, 2, 3, 7, bs + 1] {
            let pbl = bs + extra; // never a multiple when extra < bs
            for pos in [0usize, 1, bs / 2, bs - 1] {
                if pos >= bs {
                    continue; // the barrier must lie inside the scanned block
                }
                for _ in 0..2 {
                    let mut buf = rng.bytes(pbl);
                    for j in 0..pos {
                        buf[pbl - 1 - j] = 0;
                    }
                    buf[pbl - 1 - pos] = 0x80;
                    let mut full = vec![SENT; pbl + 32];
                    full[..pbl].copy_from_slice(&buf);

                    let a = call_unpad(cu, &full, pbl, bs);
                    let b = call_unpad(ru, &full, pbl, bs);
                    let what = format!("row33 unpad bs={bs} padded={pbl} barrier_pos={pos}");
                    cmp_unpad(&what, &a, &b);
                    assert_eq!(a.rc, 0, "{what}: must validate");
                    assert_eq!(a.out, pbl - 1 - pos, "{what}");
                    cases += 1;
                }
            }
        }
    }
    assert!(cases >= 64, "row33: only {cases} cases");
}

/// CONFIGS row 34: barrier at every position 0..blocksize-1 from the end, plus
/// the lone `{0x80}` / `blocksize == 1` degenerate case.
#[test]
fn cfg34_unpad_barrier_at_every_position() {
    init_both();
    let (cu, ru) = fns!("sodium_unpad", UnpadFn);
    let mut rng = Rng::new(SEED ^ 0x34);
    let mut cases = 0usize;

    for &bs in &[1usize, 2, 16, 17, 255, 256] {
        for pos in 0..bs {
            for &extra in &[0usize, 1, 3, bs] {
                let pbl = bs + extra;
                let mut buf = rng.bytes(pbl);
                for j in 0..pos {
                    buf[pbl - 1 - j] = 0;
                }
                buf[pbl - 1 - pos] = 0x80;
                let mut full = vec![SENT; pbl + 32];
                full[..pbl].copy_from_slice(&buf);

                let a = call_unpad(cu, &full, pbl, bs);
                let b = call_unpad(ru, &full, pbl, bs);
                let what = format!("row34 unpad bs={bs} padded={pbl} pos={pos}");
                cmp_unpad(&what, &a, &b);
                assert_eq!(a.rc, 0, "{what}: barrier at {pos} must validate");
                assert_eq!(a.out, pbl - 1 - pos, "{what}");
                cases += 1;
            }
        }
    }

    // lone {0x80} with blocksize == 1 -> unpadded length 0
    let one = {
        let mut v = vec![SENT; 33];
        v[0] = 0x80;
        v
    };
    let a = call_unpad(cu, &one, 1, 1);
    let b = call_unpad(ru, &one, 1, 1);
    cmp_unpad("row34 lone 0x80 bs=1", &a, &b);
    assert_eq!((a.rc, a.out), (0, 0), "row34: {{0x80}} bs=1 must give (0, 0)");
    assert!(cases >= 64, "row34: only {cases} cases");
}

/// ERRORS row 48: `sodium_pad` with `blocksize == 0` -> -1, out-param untouched.
#[test]
fn err48_pad_blocksize_zero() {
    init_both();
    let (cf, rf) = fns!("sodium_pad", PadFn);
    let mut rng = Rng::new(SEED ^ 0x48);
    for i in 0..64 {
        let ub = rng.below(64);
        let base = {
            let mut v = vec![SENT; 128];
            let d = rng.bytes(ub);
            v[..ub].copy_from_slice(&d);
            v
        };
        let maxb = *rng.pick(&[0usize, 1, 64, 128, usize::MAX]);
        let a = call_pad(cf, &base, ub, 0, maxb, true);
        let b = call_pad(rf, &base, ub, 0, maxb, true);
        let what = format!("err48 pad blocksize=0 ub={ub} max={maxb} i={i}");
        cmp_pad(&what, &a, &b);
        assert_eq!(a.rc, -1, "{what}: must return -1");
        assert_eq!(a.out, SENT_USIZE, "{what}: *padded_buflen_p must NOT be written");
        assert_eq_bytes(&format!("{what}: buf untouched"), &base, &a.buf);
    }
}

/// ERRORS row 49: `SIZE_MAX - unpadded_buflen <= xpadlen` -> `sodium_misuse()`
/// (an abort, NOT -1).
#[test]
fn err49_pad_arith_overflow_is_misuse() {
    init_both();
    let l = libs();
    for &(ub, bs) in &[
        (usize::MAX, 1usize),
        (usize::MAX, 16),
        (usize::MAX, 17),
        (usize::MAX, 255),
        (usize::MAX - 1, 16),
        (usize::MAX - 255, 256),
    ] {
        // Self-check: the input really is inside the `SIZE_MAX - unpadded_buflen
        // <= xpadlen` branch (otherwise the C would walk off the buffer instead
        // of aborting, and the row would be vacuously "passing").
        assert!(
            usize::MAX - ub <= xpadlen_of(ub, bs),
            "err49: ({ub:#x},{bs}) does not reach the overflow branch"
        );
        let run = |lib: &'static Library| -> Outcome {
            forked(|| {
                let f = unsafe { sym::<PadFn>(lib, "sodium_pad") };
                let mut buf = [0u8; 64];
                let mut out: usize = 0;
                unsafe { f(&mut out as *mut usize, buf.as_mut_ptr(), ub, bs, usize::MAX) };
                0
            })
        };
        let (oc, or) = (run(&l.c), run(&l.r));
        let what = format!("err49 sodium_pad ub={ub:#x} bs={bs}");
        assert_same_fatal(&what, oc, or);
        assert_eq!(oc, Outcome::Signaled(SIGABRT), "{what}: expected SIGABRT, got {oc:?}");
    }
}

/// ERRORS row 50: `xpadded_len >= max_buflen` -> -1, out-param untouched.
#[test]
fn err50_pad_xpadded_len_ge_max_buflen() {
    init_both();
    let (cf, rf) = fns!("sodium_pad", PadFn);
    let mut rng = Rng::new(SEED ^ 0x50);
    let mut cases = 0usize;
    for &bs in &[1usize, 2, 16, 17, 255, 256] {
        for &ub in &pad_shapes(bs) {
            let xpadded = ub + xpadlen_of(ub, bs);
            // every max_buflen from 0 up to (and including) xpadded is rejected
            for maxb in [0usize, 1, xpadded / 2, xpadded]
                .into_iter()
                .filter(|&m| m <= xpadded)
            {
                let mut base = vec![SENT; xpadded + 64];
                let d = rng.bytes(ub);
                base[..ub].copy_from_slice(&d);
                let a = call_pad(cf, &base, ub, bs, maxb, true);
                let b = call_pad(rf, &base, ub, bs, maxb, true);
                let what = format!("err50 pad bs={bs} ub={ub} xpadded={xpadded} max={maxb}");
                cmp_pad(&what, &a, &b);
                assert_eq!(a.rc, -1, "{what}: must return -1");
                assert_eq!(a.out, SENT_USIZE, "{what}: out-param must NOT be written");
                assert_eq_bytes(&format!("{what}: buf untouched"), &base, &a.buf);
                cases += 1;
            }
        }
    }
    assert!(cases >= 64, "err50: only {cases} cases");
}

/// ERRORS row 51: `sodium_unpad` with `blocksize == 0` -> -1, out-param untouched.
#[test]
fn err51_unpad_blocksize_zero() {
    init_both();
    let (cu, ru) = fns!("sodium_unpad", UnpadFn);
    let mut rng = Rng::new(SEED ^ 0x51);
    for i in 0..64 {
        let pbl = rng.below(64);
        let base = {
            let mut v = vec![SENT; 128];
            let d = rng.bytes(pbl);
            v[..pbl].copy_from_slice(&d);
            v
        };
        let a = call_unpad(cu, &base, pbl, 0);
        let b = call_unpad(ru, &base, pbl, 0);
        let what = format!("err51 unpad blocksize=0 padded={pbl} i={i}");
        cmp_unpad(&what, &a, &b);
        assert_eq!(a.rc, -1, "{what}: must return -1");
        assert_eq!(a.out, SENT_USIZE, "{what}: *unpadded_buflen_p must NOT be written");
    }
}

/// ERRORS row 52: `padded_buflen < blocksize` -> -1, out-param untouched.
#[test]
fn err52_unpad_padded_shorter_than_blocksize() {
    init_both();
    let (cu, ru) = fns!("sodium_unpad", UnpadFn);
    let mut rng = Rng::new(SEED ^ 0x52);
    let mut cases = 0usize;
    for &bs in &[1usize, 2, 16, 17, 255, 256] {
        for pbl in [0usize, 1, bs / 2, bs - 1] {
            if pbl >= bs {
                continue;
            }
            for _ in 0..4 {
                let mut base = vec![SENT; bs + 64];
                let d = rng.bytes(pbl);
                base[..pbl].copy_from_slice(&d);
                let a = call_unpad(cu, &base, pbl, bs);
                let b = call_unpad(ru, &base, pbl, bs);
                let what = format!("err52 unpad padded={pbl} bs={bs}");
                cmp_unpad(&what, &a, &b);
                assert_eq!(a.rc, -1, "{what}: must return -1");
                assert_eq!(a.out, SENT_USIZE, "{what}: out-param must NOT be written");
                cases += 1;
            }
        }
    }
    assert!(cases >= 64, "err52: only {cases} cases");
}

/// ERRORS row 53: no valid `0x80` barrier -> -1 **and** `*unpadded_buflen_p` IS
/// written with `padded_buflen - 1`.
#[test]
fn err53_unpad_no_barrier_still_writes_outparam() {
    init_both();
    let (cu, ru) = fns!("sodium_unpad", UnpadFn);
    let mut rng = Rng::new(SEED ^ 0x53);
    let mut cases = 0usize;

    for &bs in &[1usize, 2, 16, 17, 255, 256] {
        for &extra in &[0usize, 1, 5, bs] {
            let pbl = bs + extra;
            // 0: last block all-zero  1: all-0xff  2: 0x81 terminator
            // 3: nonzero byte after the barrier   4: random with no 0x80 at all
            for kind in 0..5 {
                let mut buf = rng.bytes(pbl);
                match kind {
                    0 => {
                        for j in 0..bs {
                            buf[pbl - 1 - j] = 0x00;
                        }
                    }
                    1 => {
                        for j in 0..bs {
                            buf[pbl - 1 - j] = 0xff;
                        }
                    }
                    2 => {
                        for j in 0..bs {
                            buf[pbl - 1 - j] = 0x00;
                        }
                        buf[pbl - 1] = 0x81;
                    }
                    3 => {
                        if bs < 2 {
                            continue;
                        }
                        for j in 0..bs {
                            buf[pbl - 1 - j] = 0x00;
                        }
                        buf[pbl - 1 - (bs / 2)] = 0x80; // barrier ...
                        buf[pbl - 1] = 0x37; // ... but a nonzero byte follows it
                    }
                    _ => {
                        for j in 0..bs {
                            if buf[pbl - 1 - j] == 0x80 {
                                buf[pbl - 1 - j] = 0x7f;
                            }
                        }
                        // guarantee at least one nonzero non-barrier byte at the end
                        buf[pbl - 1] = 0x01;
                    }
                }
                let mut full = vec![SENT; pbl + 32];
                full[..pbl].copy_from_slice(&buf);
                let a = call_unpad(cu, &full, pbl, bs);
                let b = call_unpad(ru, &full, pbl, bs);
                let what = format!("err53 unpad bs={bs} padded={pbl} kind={kind}");
                cmp_unpad(&what, &a, &b);
                assert_eq!(a.rc, -1, "{what}: must return -1");
                assert_eq!(
                    a.out,
                    pbl - 1,
                    "{what}: *unpadded_buflen_p must be written with padded_buflen-1"
                );
                cases += 1;
            }
        }
    }
    assert!(cases >= 64, "err53: only {cases} cases");
}

// ============================================================================
// CONFIGS 35–37 / ERRORS 62–64 — memcmp / compare / is_zero
// ============================================================================

const CMP_LENS: &[usize] = &[0, 1, 2, 8, 16, 24, 32, 64];

/// CONFIGS row 35 + ERRORS row 62.
#[test]
fn cfg35_err62_memcmp() {
    init_both();
    let (cf, rf) = fns!("sodium_memcmp", MemcmpFn);
    let mut rng = Rng::new(SEED ^ 0x35);
    let mut cases = 0usize;

    let call = |f: MemcmpFn, a: &[u8], b: &[u8], n: usize| -> c_int {
        unsafe { f(a.as_ptr() as *const c_void, b.as_ptr() as *const c_void, n) }
    };

    for &n in CMP_LENS {
        for trial in 0..16 {
            let a = rng.bytes(n.max(1));
            // equal
            let b = a.clone();
            let (x, y) = (call(cf, &a, &b, n), call(rf, &a, &b, n));
            assert_eq!(x, y, "row35 memcmp equal n={n} t={trial}: C={x} rust={y}");
            assert_eq!(x, 0, "row35 memcmp equal n={n}: expected 0");
            // aliased b1 == b2
            let (x, y) = (call(cf, &a, &a, n), call(rf, &a, &a, n));
            assert_eq!(x, y, "row35 memcmp aliased n={n}: C={x} rust={y}");
            assert_eq!(x, 0, "row35 memcmp aliased n={n}: expected 0");
            // differ in byte 0 / byte n-1 / a random byte
            if n > 0 {
                for &i in &[0usize, n - 1, rng.below(n)] {
                    let mut c = a.clone();
                    c[i] ^= 1 << (rng.below(8));
                    let (x, y) = (call(cf, &a, &c, n), call(rf, &a, &c, n));
                    assert_eq!(x, y, "row35 memcmp differ@{i} n={n}: C={x} rust={y}");
                    assert_eq!(x, -1, "err62 memcmp differ@{i} n={n}: expected -1");
                }
                // all-zero vs all-ff
                let z = vec![0u8; n];
                let f = vec![0xffu8; n];
                let (x, y) = (call(cf, &z, &f, n), call(rf, &z, &f, n));
                assert_eq!(x, y, "row35 memcmp 00-vs-ff n={n}: C={x} rust={y}");
                assert_eq!(x, -1);
            } else {
                // len == 0 -> 0 even for unrelated pointers
                let z = vec![0u8; 1];
                let f = vec![0xffu8; 1];
                let (x, y) = (call(cf, &z, &f, 0), call(rf, &z, &f, 0));
                assert_eq!(x, y, "err62 memcmp len=0: C={x} rust={y}");
                assert_eq!(x, 0, "err62 memcmp len=0 must be 0");
            }
            cases += 1;
        }
    }
    assert!(cases >= 64, "row35: only {cases} cases");
}

/// Reference model for `sodium_compare`: little-endian unsigned comparison.
fn model_compare(a: &[u8], b: &[u8]) -> c_int {
    for i in (0..a.len()).rev() {
        if a[i] < b[i] {
            return -1;
        }
        if a[i] > b[i] {
            return 1;
        }
    }
    0
}

/// CONFIGS row 36 + ERRORS row 63.
#[test]
fn cfg36_err63_compare_little_endian() {
    init_both();
    let (cf, rf) = fns!("sodium_compare", CompareFn);
    let mut rng = Rng::new(SEED ^ 0x36);
    let mut cases = 0usize;

    let call = |f: CompareFn, a: &[u8], b: &[u8], n: usize| -> c_int {
        unsafe { f(a.as_ptr(), b.as_ptr(), n) }
    };

    for &n in CMP_LENS {
        // random pairs
        for trial in 0..16 {
            let a = rng.bytes(n.max(1));
            let b = rng.bytes(n.max(1));
            let (x, y) = (call(cf, &a, &b, n), call(rf, &a, &b, n));
            assert_eq!(
                x, y,
                "row36 compare n={n} t={trial}: C={x} rust={y}\n  a={}\n  b={}",
                hexs(&a[..n]),
                hexs(&b[..n])
            );
            assert_eq!(
                x,
                model_compare(&a[..n], &b[..n]),
                "row36 compare n={n}: not little-endian ordering\n  a={}\n  b={}",
                hexs(&a[..n]),
                hexs(&b[..n])
            );
            // equal / aliased
            let (x, y) = (call(cf, &a, &a, n), call(rf, &a, &a, n));
            assert_eq!(x, y, "row36 compare aliased n={n}");
            assert_eq!(x, 0, "row36 compare aliased n={n} must be 0");
            cases += 1;
        }
        if n == 0 {
            continue;
        }
        // single-byte differences at every position: proves the LE direction
        for i in 0..n {
            let base = vec![0x80u8; n];
            let mut lo = base.clone();
            lo[i] = 0x7f;
            let mut hi = base.clone();
            hi[i] = 0x81;
            for (p, q, want) in [(&lo, &base, -1), (&hi, &base, 1), (&base, &base, 0)] {
                let (x, y) = (call(cf, p, q, n), call(rf, p, q, n));
                assert_eq!(x, y, "err63 compare n={n} pos={i}: C={x} rust={y}");
                assert_eq!(x, want, "err63 compare n={n} pos={i}: expected {want}");
            }
            // the MOST significant byte is the one at index len-1: a buffer that
            // is 0xff everywhere except a 0x00 top byte is SMALLER than one that
            // is 0x00 everywhere except a 0x01 top byte.
            let mut msb_lo = vec![0xffu8; n];
            msb_lo[n - 1] = 0x00;
            let mut msb_hi = vec![0x00u8; n];
            msb_hi[n - 1] = 0x01;
            let (x, y) = (call(cf, &msb_lo, &msb_hi, n), call(rf, &msb_lo, &msb_hi, n));
            assert_eq!(x, y, "err63 compare LE msb n={n}: C={x} rust={y}");
            assert_eq!(x, -1, "err63 compare: index len-1 must dominate (n={n})");
            let (x, y) = (call(cf, &msb_hi, &msb_lo, n), call(rf, &msb_hi, &msb_lo, n));
            assert_eq!(x, y, "err63 compare LE msb reversed n={n}: C={x} rust={y}");
            assert_eq!(x, 1, "err63 compare: index len-1 must dominate (n={n})");
        }
    }
    assert!(cases >= 64, "row36: only {cases} cases");
}

/// CONFIGS row 37 + ERRORS row 64.
#[test]
fn cfg37_err64_is_zero() {
    init_both();
    let (cf, rf) = fns!("sodium_is_zero", IsZeroFn);
    let mut rng = Rng::new(SEED ^ 0x37);
    let mut cases = 0usize;
    let call = |f: IsZeroFn, a: &[u8], n: usize| -> c_int { unsafe { f(a.as_ptr(), n) } };

    for &n in CMP_LENS {
        // all-zero
        let z = vec![0u8; n.max(1)];
        let (x, y) = (call(cf, &z, n), call(rf, &z, n));
        assert_eq!(x, y, "row37 is_zero all-zero n={n}: C={x} rust={y}");
        assert_eq!(x, 1, "row37 is_zero all-zero n={n} (nlen==0 -> 1)");
        // all-0xff
        let f = vec![0xffu8; n.max(1)];
        let (x, y) = (call(cf, &f, n), call(rf, &f, n));
        assert_eq!(x, y, "row37 is_zero all-ff n={n}: C={x} rust={y}");
        assert_eq!(x, if n == 0 { 1 } else { 0 }, "row37 is_zero all-ff n={n}");
        // single nonzero byte at every position, for every bit
        for i in 0..n {
            for bit in 0..8 {
                let mut v = vec![0u8; n];
                v[i] = 1 << bit;
                let (x, y) = (call(cf, &v, n), call(rf, &v, n));
                assert_eq!(x, y, "err64 is_zero nonzero@{i} bit={bit} n={n}: C={x} rust={y}");
                assert_eq!(x, 0, "err64 is_zero nonzero@{i} n={n} must be 0");
            }
        }
        // random
        for _ in 0..16 {
            let v = rng.bytes(n.max(1));
            let (x, y) = (call(cf, &v, n), call(rf, &v, n));
            assert_eq!(x, y, "row37 is_zero random n={n}: C={x} rust={y}");
            let want = if v[..n].iter().all(|&b| b == 0) { 1 } else { 0 };
            assert_eq!(x, want, "row37 is_zero random n={n}");
            cases += 1;
        }
    }
    assert!(cases >= 64, "row37: only {cases} cases");
}

// ============================================================================
// CONFIGS 38–40 — increment / add / sub  (8/12/24/64 are AMD64-asm fast paths)
// ============================================================================

const ARITH_LENS: &[usize] = &[0, 1, 2, 8, 12, 16, 24, 32, 64];

fn model_inc(n: &mut [u8]) {
    let mut c: u16 = 1;
    for x in n.iter_mut() {
        c += *x as u16;
        *x = c as u8;
        c >>= 8;
    }
}
fn model_add(a: &mut [u8], b: &[u8]) {
    let mut c: u16 = 0;
    for i in 0..a.len() {
        c += a[i] as u16 + b[i] as u16;
        a[i] = c as u8;
        c >>= 8;
    }
}
fn model_sub(a: &mut [u8], b: &[u8]) {
    let mut c: u16 = 0;
    for i in 0..a.len() {
        c = (a[i] as u16).wrapping_sub(b[i] as u16).wrapping_sub(c);
        a[i] = c as u8;
        c = (c >> 8) & 1;
    }
}

/// CONFIGS row 38: `sodium_increment`.
#[test]
fn cfg38_increment() {
    init_both();
    let (cf, rf) = fns!("sodium_increment", IncFn);
    let mut rng = Rng::new(SEED ^ 0x38);
    let mut cases = 0usize;

    let call = |f: IncFn, body: &[u8], n: usize| -> Vec<u8> {
        let mut v = guarded(body);
        unsafe { f(v.as_mut_ptr().add(GUARD), n) };
        v
    };

    for &n in ARITH_LENS {
        let mut pats: Vec<Vec<u8>> = vec![
            vec![0u8; n],
            vec![0xffu8; n],
            (0..n).map(|i| i as u8).collect(),
        ];
        if n > 0 {
            // partial carry: 0xff.. up to the middle, then zeros
            let mut p = vec![0u8; n];
            for i in 0..(n / 2).max(1) {
                p[i] = 0xff;
            }
            pats.push(p);
            // only the low byte is 0xff
            let mut p = vec![0u8; n];
            p[0] = 0xff;
            pats.push(p);
            // everything 0xff except the top byte
            let mut p = vec![0xffu8; n];
            p[n - 1] = 0x00;
            pats.push(p);
        }
        for _ in 0..16 {
            pats.push(rng.bytes(n));
        }

        for (k, p) in pats.iter().enumerate() {
            let (a, b) = (call(cf, p, n), call(rf, p, n));
            let what = format!("row38 sodium_increment len={n} pat={k} in={}", hexs(p));
            assert_eq_bytes(&what, &a, &b);
            let mut want = guarded(p);
            model_inc(&mut want[GUARD..GUARD + n]);
            assert_eq_bytes(&format!("{what}: vs model"), &want, &a);
            cases += 1;
        }
    }
    assert!(cases >= 64, "row38: only {cases} cases");
}

/// CONFIGS row 39: `sodium_add`.
#[test]
fn cfg39_add() {
    init_both();
    let (cf, rf) = fns!("sodium_add", AddSubFn);
    let mut rng = Rng::new(SEED ^ 0x39);
    let mut cases = 0usize;

    let call = |f: AddSubFn, body: &[u8], b: &[u8], n: usize| -> Vec<u8> {
        let mut v = guarded(body);
        let bb = guarded(b);
        unsafe { f(v.as_mut_ptr().add(GUARD), bb.as_ptr().add(GUARD), n) };
        v
    };

    for &n in ARITH_LENS {
        let mut pairs: Vec<(Vec<u8>, Vec<u8>)> = vec![
            (vec![0u8; n], vec![0u8; n]),
            (vec![0xffu8; n], vec![1u8; n]),          // carry out of the top byte
            (vec![0xffu8; n], vec![0xffu8; n]),       // maximal wrap
            (vec![0u8; n], vec![0xffu8; n]),
            ((0..n).map(|i| i as u8).collect(), vec![0x80u8; n]),
        ];
        if n > 0 {
            // single carry that ripples exactly one byte
            let mut a = vec![0u8; n];
            a[0] = 0xff;
            let mut b = vec![0u8; n];
            b[0] = 0x01;
            pairs.push((a, b));
            // carry out of the very top byte only
            let mut a = vec![0u8; n];
            a[n - 1] = 0xff;
            let mut b = vec![0u8; n];
            b[n - 1] = 0x01;
            pairs.push((a, b));
        }
        for _ in 0..16 {
            pairs.push((rng.bytes(n), rng.bytes(n)));
        }

        for (k, (p, q)) in pairs.iter().enumerate() {
            let (x, y) = (call(cf, p, q, n), call(rf, p, q, n));
            let what = format!(
                "row39 sodium_add len={n} pair={k}\n  a={}\n  b={}",
                hexs(p),
                hexs(q)
            );
            assert_eq_bytes(&what, &x, &y);
            let mut want = guarded(p);
            model_add(&mut want[GUARD..GUARD + n], q);
            assert_eq_bytes(&format!("{what}\n  vs model"), &want, &x);
            cases += 1;
        }
    }
    assert!(cases >= 64, "row39: only {cases} cases");
}

/// CONFIGS row 40: `sodium_sub` (len 64 is the asm fast path; `0 - 1` borrow).
#[test]
fn cfg40_sub() {
    init_both();
    let (cf, rf) = fns!("sodium_sub", AddSubFn);
    let mut rng = Rng::new(SEED ^ 0x40);
    let mut cases = 0usize;

    let call = |f: AddSubFn, body: &[u8], b: &[u8], n: usize| -> Vec<u8> {
        let mut v = guarded(body);
        let bb = guarded(b);
        unsafe { f(v.as_mut_ptr().add(GUARD), bb.as_ptr().add(GUARD), n) };
        v
    };

    for &n in ARITH_LENS {
        let mut pairs: Vec<(Vec<u8>, Vec<u8>)> = vec![
            (vec![0u8; n], vec![0u8; n]),
            (vec![0u8; n], vec![1u8; n]),
            (vec![0xffu8; n], vec![0xffu8; n]),
            (vec![0u8; n], vec![0xffu8; n]),
            (vec![0xffu8; n], vec![0u8; n]),
        ];
        if n > 0 {
            // 0 - 1: full borrow chain -> all 0xff
            let mut b = vec![0u8; n];
            b[0] = 1;
            pairs.push((vec![0u8; n], b));
            // borrow that stops after one byte
            let mut a = vec![0u8; n];
            a[1 % n] = 1;
            let mut b = vec![0u8; n];
            b[0] = 1;
            pairs.push((a, b));
        }
        for _ in 0..16 {
            pairs.push((rng.bytes(n), rng.bytes(n)));
        }

        for (k, (p, q)) in pairs.iter().enumerate() {
            let (x, y) = (call(cf, p, q, n), call(rf, p, q, n));
            let what = format!(
                "row40 sodium_sub len={n} pair={k}\n  a={}\n  b={}",
                hexs(p),
                hexs(q)
            );
            assert_eq_bytes(&what, &x, &y);
            let mut want = guarded(p);
            model_sub(&mut want[GUARD..GUARD + n], q);
            assert_eq_bytes(&format!("{what}\n  vs model"), &want, &x);
            cases += 1;
        }
    }
    // explicit 0 - 1 == all-0xff for the asm-relevant lengths
    for &n in &[8usize, 12, 24, 64] {
        let a = vec![0u8; n];
        let mut b = vec![0u8; n];
        b[0] = 1;
        let (x, y) = (call(cf, &a, &b, n), call(rf, &a, &b, n));
        assert_eq_bytes(&format!("row40 0-1 len={n}"), &x, &y);
        assert!(
            x[GUARD..GUARD + n].iter().all(|&v| v == 0xff),
            "row40: 0-1 with len={n} must borrow through the whole buffer, got {}",
            hexs(&x[GUARD..GUARD + n])
        );
    }
    assert!(cases >= 64, "row40: only {cases} cases");
}

// ============================================================================
// CONFIGS 41–44 / ERRORS 54–61 — allocation, page protection, mlock, memzero
// ============================================================================

/// CONFIGS row 41: `sodium_malloc` / `sodium_free`.
#[test]
fn cfg41_malloc_free() {
    init_both();
    let (cm, rm) = fns!("sodium_malloc", MallocFn);
    let (cfr, rfr) = fns!("sodium_free", FreeFn);
    let mut rng = Rng::new(SEED ^ 0x41);

    for &size in &[0usize, 1, 16, 4095, 4096, 65535, 65536, 65537] {
        for trial in 0..8 {
            clear_errno();
            let pc = unsafe { cm(size) };
            let ec = errno();
            clear_errno();
            let pr = unsafe { rm(size) };
            let er = errno();
            let what = format!("row41 sodium_malloc({size}) trial={trial}");
            assert_eq!(
                pc.is_null(),
                pr.is_null(),
                "{what}: NULL-ness differs (C null={} rust null={})",
                pc.is_null(),
                pr.is_null()
            );
            assert!(!pc.is_null(), "{what}: C returned NULL");
            assert_eq!(ec, er, "{what}: errno C={ec} rust={er}");

            if size > 0 {
                // GARBAGE prefill must be visible over the WHOLE region
                let sc = unsafe { std::slice::from_raw_parts(pc as *const u8, size) };
                let sr = unsafe { std::slice::from_raw_parts(pr as *const u8, size) };
                assert_eq_bytes(&format!("{what}: 0xdb prefill"), sc, sr);
                assert!(
                    sc.iter().all(|&b| b == GARBAGE),
                    "{what}: C region is not filled with 0x{GARBAGE:02x}"
                );
                // the whole region must be writable and read back verbatim
                let pat = rng.bytes(size);
                unsafe {
                    std::ptr::copy_nonoverlapping(pat.as_ptr(), pc as *mut u8, size);
                    std::ptr::copy_nonoverlapping(pat.as_ptr(), pr as *mut u8, size);
                }
                let sc = unsafe { std::slice::from_raw_parts(pc as *const u8, size) };
                let sr = unsafe { std::slice::from_raw_parts(pr as *const u8, size) };
                assert_eq_bytes(&format!("{what}: read-back"), sc, sr);
                assert_eq_bytes(&format!("{what}: read-back vs written"), &pat, sc);
            }
            unsafe {
                cfr(pc);
                rfr(pr);
            }
        }
    }
}

/// CONFIGS row 42: `sodium_allocarray`.
#[test]
fn cfg42_allocarray() {
    init_both();
    let (ca, ra) = fns!("sodium_allocarray", AllocArrayFn);
    let (cfr, rfr) = fns!("sodium_free", FreeFn);

    for &(count, size) in &[
        (0usize, 7usize),
        (0, 0),
        (7, 0),
        (1, 1),
        (1, 0),
        (1024, 16),
        (16, 1024),
        (65537, 1),
    ] {
        for trial in 0..8 {
            clear_errno();
            let pc = unsafe { ca(count, size) };
            let ec = errno();
            clear_errno();
            let pr = unsafe { ra(count, size) };
            let er = errno();
            let what = format!("row42 sodium_allocarray({count},{size}) t={trial}");
            assert_eq!(pc.is_null(), pr.is_null(), "{what}: NULL-ness differs");
            assert!(!pc.is_null(), "{what}: C returned NULL");
            assert_eq!(ec, er, "{what}: errno C={ec} rust={er}");
            let total = count * size;
            if total > 0 {
                let sc = unsafe { std::slice::from_raw_parts(pc as *const u8, total) };
                let sr = unsafe { std::slice::from_raw_parts(pr as *const u8, total) };
                assert_eq_bytes(&format!("{what}: 0xdb prefill"), sc, sr);
                assert!(
                    sc.iter().all(|&b| b == GARBAGE),
                    "{what}: region not 0x{GARBAGE:02x}"
                );
            }
            unsafe {
                cfr(pc);
                rfr(pr);
            }
        }
    }
}

/// CONFIGS row 43 + ERRORS rows 57/58/59: the `sodium_mprotect_*` lifecycle.
/// This build has no `HAVE_PAGE_PROTECTION`, so all three are -1/ENOSYS and the
/// region stays readable and writable throughout.
#[test]
fn cfg43_err57_58_59_mprotect_lifecycle() {
    init_both();
    let (cm, rm) = fns!("sodium_malloc", MallocFn);
    let (cfr, rfr) = fns!("sodium_free", FreeFn);
    let names = [
        "sodium_mprotect_readonly",
        "sodium_mprotect_noaccess",
        "sodium_mprotect_readwrite",
    ];
    let mut rng = Rng::new(SEED ^ 0x43);
    let mut sizes: Vec<usize> = vec![1, 16, 4096, 65537];
    for _ in 0..20 {
        sizes.push(1 + rng.below(8192));
    }

    for &size in &sizes {
        let pc = unsafe { cm(size) };
        let pr = unsafe { rm(size) };
        assert!(!pc.is_null() && !pr.is_null(), "row43 malloc({size}) failed");
        // write
        unsafe {
            std::ptr::write_bytes(pc as *mut u8, 0x5a, size);
            std::ptr::write_bytes(pr as *mut u8, 0x5a, size);
        }
        for pass in 0..2 {
            for n in names {
                let (cf, rf) = unsafe { pair::<MprotectFn>(n) };
                clear_errno();
                let a = unsafe { cf(pc) };
                let ea = errno();
                clear_errno();
                let b = unsafe { rf(pr) };
                let eb = errno();
                let what = format!("row43/{n} size={size} pass={pass}");
                assert_eq!(a, b, "{what}: return C={a} rust={b}");
                assert_eq!(ea, eb, "{what}: errno C={ea} rust={eb}");
                assert_eq!(a, -1, "{what}: expected -1 (no HAVE_PAGE_PROTECTION)");
                assert_eq!(ea, libc::ENOSYS, "{what}: expected errno=ENOSYS, got {ea}");
            }
            // still fully readable / writable
            let sc = unsafe { std::slice::from_raw_parts(pc as *const u8, size) };
            let sr = unsafe { std::slice::from_raw_parts(pr as *const u8, size) };
            assert_eq_bytes(&format!("row43 region size={size} pass={pass}"), sc, sr);
            assert!(sc.iter().all(|&v| v == 0x5a), "row43: region changed");
        }
        unsafe {
            cfr(pc);
            rfr(pr);
        }
    }
}

/// CONFIGS row 44: `sodium_memzero` / `sodium_stackzero`.
#[test]
fn cfg44_memzero_stackzero() {
    init_both();
    let (cz, rz) = fns!("sodium_memzero", MemzeroFn);
    let (cs, rs) = fns!("sodium_stackzero", StackzeroFn);
    let mut rng = Rng::new(SEED ^ 0x44);

    for &len in &[0usize, 1, 64, 4096] {
        for trial in 0..16 {
            let body = rng.bytes(len);
            let mut a = guarded(&body);
            let mut b = guarded(&body);
            unsafe {
                cz(a.as_mut_ptr().add(GUARD) as *mut c_void, len);
                rz(b.as_mut_ptr().add(GUARD) as *mut c_void, len);
            }
            let what = format!("row44 sodium_memzero len={len} t={trial}");
            assert_eq_bytes(&what, &a, &b);
            let mut want = guarded(&body);
            for x in want[GUARD..GUARD + len].iter_mut() {
                *x = 0;
            }
            assert_eq_bytes(&format!("{what}: vs model"), &want, &a);
            // sodium_stackzero is a no-op in this build (no HAVE_C_VARARRAYS /
            // HAVE_ALLOCA); assert both return normally for the same lengths.
            unsafe {
                cs(len);
                rs(len);
            }
        }
    }
}

/// ERRORS row 54: `sodium_malloc` allocation failure -> NULL + `errno=ENOMEM`.
/// (The documented `size >= SIZE_MAX - page_size*4` pre-check only exists under
/// `HAVE_ALIGNED_MALLOC`, which this build does not define; the reachable
/// equivalent is the underlying `malloc()` failing, which is what is compared.)
#[test]
fn err54_malloc_huge_size_returns_null_enomem() {
    init_both();
    let (cm, rm) = fns!("sodium_malloc", MallocFn);
    let page = unsafe { libc::sysconf(libc::_SC_PAGESIZE) } as usize;
    let mut rng = Rng::new(SEED ^ 0x54);
    let mut sizes: Vec<usize> = vec![
        usize::MAX,
        usize::MAX - 1,
        usize::MAX - page * 4,
        usize::MAX - page * 4 + 1,
        usize::MAX / 2,
    ];
    for _ in 0..64 {
        // uniformly random in [2^63, 2^64): far beyond any possible mapping
        sizes.push((rng.next_u64() | (1u64 << 63)) as usize);
    }
    for &size in &sizes {
        clear_errno();
        let pc = unsafe { cm(size) };
        let ec = errno();
        clear_errno();
        let pr = unsafe { rm(size) };
        let er = errno();
        let what = format!("err54 sodium_malloc({size:#x})");
        assert_eq!(pc.is_null(), pr.is_null(), "{what}: NULL-ness differs");
        assert!(pc.is_null(), "{what}: C unexpectedly succeeded");
        assert_eq!(ec, er, "{what}: errno C={ec} rust={er}");
        assert_eq!(ec, libc::ENOMEM, "{what}: expected ENOMEM, got {ec}");
    }
}

/// ERRORS row 55: `sodium_allocarray` with `count > 0 && size >= SIZE_MAX/count`
/// -> NULL, `errno=ENOMEM`.
#[test]
fn err55_allocarray_overflow_returns_null_enomem() {
    init_both();
    let (ca, ra) = fns!("sodium_allocarray", AllocArrayFn);
    let mut rng = Rng::new(SEED ^ 0x55);
    let mut pairs: Vec<(usize, usize)> = vec![
        (2usize, usize::MAX / 2),
        (2, usize::MAX / 2 + 1),
        (1024, usize::MAX / 1024),
        (usize::MAX, 2),
        (usize::MAX, usize::MAX),
        (1, usize::MAX),
        (3, usize::MAX / 3),
    ];
    for _ in 0..64 {
        let count = 2 + rng.below(1_000_000);
        let size = usize::MAX / count + rng.below(64);
        assert!(size >= usize::MAX / count);
        pairs.push((count, size));
    }
    for &(count, size) in &pairs {
        clear_errno();
        let pc = unsafe { ca(count, size) };
        let ec = errno();
        clear_errno();
        let pr = unsafe { ra(count, size) };
        let er = errno();
        let what = format!("err55 sodium_allocarray({count:#x},{size:#x})");
        assert_eq!(pc.is_null(), pr.is_null(), "{what}: NULL-ness differs");
        assert!(pc.is_null(), "{what}: C unexpectedly succeeded");
        assert_eq!(ec, er, "{what}: errno C={ec} rust={er}");
        assert_eq!(ec, libc::ENOMEM, "{what}: expected ENOMEM, got {ec}");
    }
}

/// ERRORS row 56: `sodium_free(NULL)` is a no-op.
#[test]
fn err56_free_null_is_noop() {
    init_both();
    let (cfr, rfr) = fns!("sodium_free", FreeFn);
    let l = libs();
    // In-process: must not crash, must not disturb errno.
    for _ in 0..8 {
        clear_errno();
        unsafe { cfr(std::ptr::null_mut()) };
        let ec = errno();
        clear_errno();
        unsafe { rfr(std::ptr::null_mut()) };
        let er = errno();
        assert_eq!(ec, er, "err56 sodium_free(NULL): errno C={ec} rust={er}");
        assert_eq!(ec, 0, "err56 sodium_free(NULL) must not set errno");
    }
    // Forked: a crash would be visible as a signal in exactly one library.
    let run = |lib: &'static Library| -> Outcome {
        forked(|| {
            let f = unsafe { sym::<FreeFn>(lib, "sodium_free") };
            for _ in 0..64 {
                unsafe { f(std::ptr::null_mut()) };
            }
            0
        })
    };
    let (oc, or) = (run(&l.c), run(&l.r));
    assert_same_fatal("err56 sodium_free(NULL) forked", oc, or);
    assert_eq!(oc, Outcome::Returned(0), "err56: C died on free(NULL)");
}

/// ERRORS rows 60/61: `sodium_mlock` / `sodium_munlock` -> -1 (this build has no
/// `HAVE_MLOCK`, so `errno=ENOSYS`); `sodium_munlock` zeroes the region first.
#[test]
fn err60_61_mlock_munlock() {
    init_both();
    let (cl, rl) = fns!("sodium_mlock", MlockFn);
    let (cu, ru) = fns!("sodium_munlock", MlockFn);
    let mut rng = Rng::new(SEED ^ 0x60);
    let mut lens: Vec<usize> = vec![0, 1, 16, 64, 4096, 65537];
    for _ in 0..64 {
        lens.push(rng.below(8192));
    }

    for &len in &lens {
        let body = rng.bytes(len);
        let mut a = guarded(&body);
        let mut b = guarded(&body);

        clear_errno();
        let x = unsafe { cl(a.as_mut_ptr().add(GUARD) as *mut c_void, len) };
        let ex = errno();
        clear_errno();
        let y = unsafe { rl(b.as_mut_ptr().add(GUARD) as *mut c_void, len) };
        let ey = errno();
        let what = format!("err60 sodium_mlock len={len}");
        assert_eq!(x, y, "{what}: return C={x} rust={y}");
        assert_eq!(ex, ey, "{what}: errno C={ex} rust={ey}");
        assert_eq!(x, -1, "{what}: expected -1");
        assert_eq!(ex, libc::ENOSYS, "{what}: expected ENOSYS, got {ex}");
        assert_eq_bytes(&format!("{what}: buffer untouched"), &a, &b);
        assert_eq_bytes(&format!("{what}: mlock must not modify"), &guarded(&body), &a);

        clear_errno();
        let x = unsafe { cu(a.as_mut_ptr().add(GUARD) as *mut c_void, len) };
        let ex = errno();
        clear_errno();
        let y = unsafe { ru(b.as_mut_ptr().add(GUARD) as *mut c_void, len) };
        let ey = errno();
        let what = format!("err61 sodium_munlock len={len}");
        assert_eq!(x, y, "{what}: return C={x} rust={y}");
        assert_eq!(ex, ey, "{what}: errno C={ex} rust={ey}");
        assert_eq!(x, -1, "{what}: expected -1");
        assert_eq!(ex, libc::ENOSYS, "{what}: expected ENOSYS, got {ex}");
        assert_eq_bytes(&format!("{what}: zeroed region"), &a, &b);
        let mut want = guarded(&body);
        for v in want[GUARD..GUARD + len].iter_mut() {
            *v = 0;
        }
        assert_eq_bytes(&format!("{what}: must zero the region first"), &want, &a);
    }
}

// ============================================================================
// CONFIGS 45/46 / ERRORS 65–70 — core.c, runtime.c, version.c
// ============================================================================

const RUNTIME_HAS: &[&str] = &[
    "sodium_runtime_has_neon",
    "sodium_runtime_has_armcrypto",
    "sodium_runtime_has_sse2",
    "sodium_runtime_has_sse3",
    "sodium_runtime_has_ssse3",
    "sodium_runtime_has_sse41",
    "sodium_runtime_has_avx",
    "sodium_runtime_has_avx2",
    "sodium_runtime_has_avx512f",
    "sodium_runtime_has_pclmul",
    "sodium_runtime_has_aesni",
    "sodium_runtime_has_rdrand",
];

/// CONFIGS row 45 (second half) + ERRORS row 65: every `sodium_init()` after the
/// first successful one returns 1. The harness already made the first call.
#[test]
fn cfg45_err65_sodium_init_repeated_returns_1() {
    init_both();
    let (c, r) = fns!("sodium_init", IntFn);
    for i in 0..8 {
        let (a, b) = unsafe { (c(), r()) };
        assert_eq!(a, b, "err65 sodium_init call #{i}: C={a} rust={b}");
        assert_eq!(a, 1, "err65 sodium_init after init must return 1, got {a}");
    }
}

/// ERRORS row 66: `sodium_init` returns -1 when `sodium_crit_enter()` /
/// `_leave()` fail. In this build (`c_src/CMakeLists.txt` defines no `HAVE_*`,
/// so core.c takes the `#else` branch) both critical-section helpers can only
/// return 0, hence the -1 return is UNREACHABLE. What is asserted instead is
/// the observable pair: both helpers succeed and `sodium_init` never returns -1.
#[test]
fn err66_init_never_returns_minus_one() {
    init_both();
    let (ci, ri) = fns!("sodium_init", IntFn);
    let (ce, re) = fns!("sodium_crit_enter", IntFn);
    let (cl, rl) = fns!("sodium_crit_leave", IntFn);
    for i in 0..8 {
        let (a, b) = unsafe { (ce(), re()) };
        assert_eq!(a, b, "err66 sodium_crit_enter #{i}: C={a} rust={b}");
        assert_eq!(a, 0, "err66 sodium_crit_enter must succeed, got {a}");
        let (a, b) = unsafe { (cl(), rl()) };
        assert_eq!(a, b, "err66 sodium_crit_leave #{i}: C={a} rust={b}");
        assert_eq!(a, 0, "err66 sodium_crit_leave after enter must succeed, got {a}");
        let (a, b) = unsafe { (ci(), ri()) };
        assert_eq!(a, b, "err66 sodium_init #{i}: C={a} rust={b}");
        assert_ne!(a, -1, "err66 sodium_init must not fail");
    }
}

/// ERRORS row 67: `sodium_crit_leave()` while `locked == 0`.
#[test]
fn err67_crit_leave_while_unlocked() {
    init_both();
    let (cl, rl) = fns!("sodium_crit_leave", IntFn);
    for i in 0..8 {
        clear_errno();
        let a = unsafe { cl() };
        let ea = errno();
        clear_errno();
        let b = unsafe { rl() };
        let eb = errno();
        assert_eq!(a, b, "err67 sodium_crit_leave (unlocked) #{i}: C={a} rust={b}");
        assert_eq!(ea, eb, "err67 sodium_crit_leave errno #{i}: C={ea} rust={eb}");
    }
}

/// ERRORS row 68: `sodium_misuse()` runs the installed handler and then aborts
/// UNCONDITIONALLY — a handler that returns normally must still abort.
#[test]
fn err68_misuse_runs_handler_then_always_aborts() {
    init_both();
    let l = libs();

    // (a) handler returns normally -> the process must still die from SIGABRT.
    extern "C" fn handler_returns() {}
    let run_a = |lib: &'static Library| -> Outcome {
        forked(|| {
            let set = unsafe { sym::<SetMisuseFn>(lib, "sodium_set_misuse_handler") };
            let mis = unsafe { sym::<VoidFn>(lib, "sodium_misuse") };
            assert_eq!(unsafe { set(Some(handler_returns)) }, 0);
            unsafe { mis() };
            0 // unreachable: sodium_misuse() is noreturn
        })
    };
    let (oc, or) = (run_a(&l.c), run_a(&l.r));
    assert_same_fatal("err68 misuse with returning handler", oc, or);
    assert_eq!(oc, Outcome::Signaled(SIGABRT), "err68: expected SIGABRT, got {oc:?}");

    // (b) handler that exits with a marker -> proves the handler IS invoked.
    extern "C" fn handler_exits() {
        unsafe { libc::_exit(37) };
    }
    let run_b = |lib: &'static Library| -> Outcome {
        forked(|| {
            let set = unsafe { sym::<SetMisuseFn>(lib, "sodium_set_misuse_handler") };
            let mis = unsafe { sym::<VoidFn>(lib, "sodium_misuse") };
            assert_eq!(unsafe { set(Some(handler_exits)) }, 0);
            unsafe { mis() };
            0
        })
    };
    let (oc, or) = (run_b(&l.c), run_b(&l.r));
    assert_same_fatal("err68 misuse handler invoked", oc, or);
    assert_eq!(oc, Outcome::Returned(37), "err68: handler was not invoked ({oc:?})");

    // (c) no handler at all -> still SIGABRT.
    let run_c = |lib: &'static Library| -> Outcome {
        forked(|| {
            let set = unsafe { sym::<SetMisuseFn>(lib, "sodium_set_misuse_handler") };
            let mis = unsafe { sym::<VoidFn>(lib, "sodium_misuse") };
            assert_eq!(unsafe { set(None) }, 0);
            unsafe { mis() };
            0
        })
    };
    let (oc, or) = (run_c(&l.c), run_c(&l.r));
    assert_same_fatal("err68 misuse without handler", oc, or);
    assert_eq!(oc, Outcome::Signaled(SIGABRT), "err68: expected SIGABRT, got {oc:?}");
}

/// ERRORS row 69: `sodium_set_misuse_handler(NULL)` is valid and returns 0.
#[test]
fn err69_set_misuse_handler_null_returns_0() {
    init_both();
    extern "C" fn noop_handler() {}
    let (cs, rs) = fns!("sodium_set_misuse_handler", SetMisuseFn);
    for i in 0..8 {
        clear_errno();
        let a = unsafe { cs(None) };
        let ea = errno();
        clear_errno();
        let b = unsafe { rs(None) };
        let eb = errno();
        assert_eq!(a, b, "err69 set_misuse_handler(NULL) #{i}: C={a} rust={b}");
        assert_eq!(a, 0, "err69 set_misuse_handler(NULL) must return 0, got {a}");
        assert_eq!(ea, eb, "err69 errno #{i}: C={ea} rust={eb}");
        // and a non-NULL handler is equally accepted
        let (a, b) = unsafe { (cs(Some(noop_handler)), rs(Some(noop_handler))) };
        assert_eq!(a, b, "err69 set_misuse_handler(fn) #{i}: C={a} rust={b}");
        assert_eq!(a, 0, "err69 set_misuse_handler(fn) must return 0");
    }
    // leave the libraries without a handler
    unsafe {
        cs(None);
        rs(None);
    }
}

/// CONFIGS row 45 (first half) + ERRORS row 70: `sodium_runtime_has_*` BEFORE
/// `sodium_init()` must be 0 (statically zero-initialised `CPUFeatures`), and the
/// FIRST `sodium_init()` returns 0.
///
/// The harness inits both libraries at start-up, so a pristine copy of each `.so`
/// is loaded from a distinct path (distinct inode => fresh copy of all statics).
/// `sodium_init() == 0` on that copy is the proof that the statics really are
/// pristine.
#[test]
fn cfg45_err70_runtime_features_before_init_and_first_init() {
    init_both();
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let rust_so = {
        let d = root.join("target/debug/liblibsodium.so");
        if d.exists() {
            d
        } else {
            root.join("target/release/liblibsodium.so")
        }
    };
    let tmp = std::env::temp_dir();
    let pid = std::process::id();
    let mut loaded = vec![];
    for (tag, src) in [
        ("c", root.join("c_src/build/libsodium.so")),
        ("rust", rust_so),
    ] {
        let dst = tmp.join(format!("t02_fresh_{tag}_{pid}.so"));
        let _ = std::fs::remove_file(&dst);
        std::fs::copy(&src, &dst).unwrap_or_else(|e| panic!("copy {src:?} -> {dst:?}: {e}"));
        let lib = unsafe { Library::new(&dst) }.unwrap_or_else(|e| panic!("dlopen {dst:?}: {e}"));
        loaded.push((tag, dst, lib));
    }

    let mut before: Vec<Vec<c_int>> = vec![];
    let mut after: Vec<Vec<c_int>> = vec![];
    for (tag, _, lib) in &loaded {
        let mut b = vec![];
        for n in RUNTIME_HAS {
            let mut name = n.as_bytes().to_vec();
            name.push(0);
            let f = unsafe { lib.get::<IntFn>(&name) }.unwrap_or_else(|e| panic!("{n}: {e}"));
            b.push(unsafe { f() });
        }
        // first sodium_init() on a pristine library returns 0
        let init = unsafe { lib.get::<IntFn>(b"sodium_init\0") }.expect("sodium_init");
        let rc = unsafe { init() };
        assert_eq!(
            rc, 0,
            "cfg45[{tag}]: first sodium_init() on a fresh copy must return 0 \
             (got {rc}; the copy was NOT loaded with fresh statics, so the \
             before-init observation would be meaningless)"
        );
        let rc2 = unsafe { init() };
        assert_eq!(rc2, 1, "cfg45[{tag}]: second sodium_init() must return 1, got {rc2}");
        let mut a = vec![];
        for n in RUNTIME_HAS {
            let mut name = n.as_bytes().to_vec();
            name.push(0);
            let f = unsafe { lib.get::<IntFn>(&name) }.unwrap_or_else(|e| panic!("{n}: {e}"));
            a.push(unsafe { f() });
        }
        before.push(b);
        after.push(a);
    }

    for (i, n) in RUNTIME_HAS.iter().enumerate() {
        assert_eq!(
            before[0][i], before[1][i],
            "err70 {n} BEFORE sodium_init: C={} rust={}",
            before[0][i], before[1][i]
        );
        assert_eq!(before[0][i], 0, "err70 {n} before init must be 0");
        assert_eq!(
            after[0][i], after[1][i],
            "cfg45 {n} AFTER sodium_init: C={} rust={}",
            after[0][i], after[1][i]
        );
    }

    // Keep the fresh copies mapped (dlclose of a live cdylib is avoided) but
    // remove the temporary files.
    for (_, dst, lib) in loaded {
        std::mem::forget(lib);
        let _ = std::fs::remove_file(dst);
    }
}

/// CONFIGS row 45: all 12 `sodium_runtime_has_*` compared between the libraries
/// in the initialised process.
#[test]
fn cfg45_runtime_has_all_twelve() {
    init_both();
    for n in RUNTIME_HAS {
        let (c, r) = unsafe { pair::<IntFn>(n) };
        for i in 0..4 {
            let (a, b) = unsafe { (c(), r()) };
            assert_eq!(a, b, "cfg45 {n} #{i}: C={a} rust={b}");
            assert!(a == 0 || a == 1, "cfg45 {n}: unexpected value {a}");
        }
    }
    // `_sodium_runtime_get_cpu_features()` is idempotent and returns the same
    // value from both libraries.
    let (c, r) = fns!("_sodium_runtime_get_cpu_features", IntFn);
    for i in 0..3 {
        let (a, b) = unsafe { (c(), r()) };
        assert_eq!(a, b, "cfg45 _sodium_runtime_get_cpu_features #{i}: C={a} rust={b}");
    }
    for n in RUNTIME_HAS {
        let (c, r) = unsafe { pair::<IntFn>(n) };
        let (a, b) = unsafe { (c(), r()) };
        assert_eq!(a, b, "cfg45 {n} after re-probe: C={a} rust={b}");
    }
}

/// CONFIGS row 46: version.c.
#[test]
fn cfg46_version() {
    init_both();
    let (c, r) = fns!("sodium_version_string", NameFn);
    let (cs, rs) = unsafe { (CStr::from_ptr(c()), CStr::from_ptr(r())) };
    assert_eq!(cs, rs, "row46 sodium_version_string: C={cs:?} rust={rs:?}");
    assert_eq!(cs.to_bytes(), b"1.0.23", "row46 version string");
    for (n, want) in [
        ("sodium_library_version_major", 30),
        ("sodium_library_version_minor", 0),
        ("sodium_library_minimal", 0),
    ] {
        let (c, r) = unsafe { pair::<IntFn>(n) };
        let (a, b) = unsafe { (c(), r()) };
        assert_eq!(a, b, "row46 {n}: C={a} rust={b}");
        assert_eq!(a, want, "row46 {n}: expected {want}");
    }
}

// ============================================================================
// CONFIGS 47–53 / ERRORS 71–80 — randombytes
// ============================================================================

/// CONFIGS row 47: `randombytes_buf_deterministic` — fully deterministic, so
/// this is a byte-for-byte comparison over a seed x size sweep.
#[test]
fn cfg47_randombytes_buf_deterministic_sweep() {
    init_both();
    let (cf, rf) = fns!("randombytes_buf_deterministic", BufDetFn);
    let mut rng = Rng::new(SEED ^ 0x47);

    let mut seeds: Vec<Vec<u8>> = vec![
        vec![0x00u8; 32],
        vec![0xffu8; 32],
        (0..32u8).collect(),
        b"0123456789abcdefghijklmnopqrstuv".to_vec(),
    ];
    for _ in 0..8 {
        seeds.push(rng.bytes(32));
    }
    let sizes = [0usize, 1, 31, 32, 33, 63, 64, 65, 1000, 65536];

    let mut cases = 0usize;
    for (si, seed) in seeds.iter().enumerate() {
        assert_eq!(seed.len(), 32);
        for &size in &sizes {
            let mut a = vec![SENT; size + 64];
            let mut b = vec![SENT; size + 64];
            unsafe {
                cf(a.as_mut_ptr() as *mut c_void, size, seed.as_ptr());
                rf(b.as_mut_ptr() as *mut c_void, size, seed.as_ptr());
            }
            let what = format!("row47 buf_deterministic seed#{si} size={size}");
            assert_eq_bytes(&what, &a, &b);
            assert!(
                a[size..].iter().all(|&v| v == SENT),
                "{what}: wrote past `size`"
            );
            if size > 0 {
                assert!(
                    a[..size].iter().any(|&v| v != SENT),
                    "{what}: nothing was written"
                );
            }
            cases += 1;
        }
    }
    // ERRORS row 80 companion: the seed length really is randombytes_SEEDBYTES.
    let (csb, rsb) = fns!("randombytes_seedbytes", SizeFn);
    let (a, b) = unsafe { (csb(), rsb()) };
    assert_eq!(a, b, "row47/err80 randombytes_seedbytes: C={a} rust={b}");
    assert_eq!(a, 32, "err80 randombytes_seedbytes must be 32");
    assert!(cases >= 64, "row47: only {cases} cases");
}

/// ERRORS row 76: `randombytes_buf_deterministic` with `size > 0x4000000000`
/// (2^38) is a misuse (`SIZE_MAX` is larger than that on this target).
#[test]
fn err76_buf_deterministic_size_over_2pow38_is_misuse() {
    init_both();
    let l = libs();
    for &size in &[0x4000000001usize, 0x8000000000, usize::MAX] {
        let run = |lib: &'static Library| -> Outcome {
            forked(|| {
                let f = unsafe { sym::<BufDetFn>(lib, "randombytes_buf_deterministic") };
                let seed = [0x5au8; 32];
                let mut buf = [0u8; 4096];
                unsafe { f(buf.as_mut_ptr() as *mut c_void, size, seed.as_ptr()) };
                0
            })
        };
        let (oc, or) = (run(&l.c), run(&l.r));
        let what = format!("err76 buf_deterministic size={size:#x}");
        assert_same_fatal(&what, oc, or);
        assert_eq!(oc, Outcome::Signaled(SIGABRT), "{what}: expected SIGABRT, got {oc:?}");
    }
}

// --- injected implementations owned by this test file -----------------------

extern "C" fn t_name() -> *const c_char {
    b"t02impl\0".as_ptr() as *const c_char
}
extern "C" fn t_random() -> u32 {
    0x1234_5678
}
extern "C" fn t_buf(p: *mut c_void, n: usize) {
    unsafe { std::ptr::write_bytes(p as *mut u8, 0x77, n) }
}
extern "C" fn t_uniform(ub: u32) -> u32 {
    ub ^ 0xa5a5_a5a5
}

/// `stir == NULL`, `uniform == NULL`, `close == NULL` — all three optional
/// fields absent (ERRORS row 78, CONFIGS row 52).
static IMPL_MINIMAL: RandombytesImpl = RandombytesImpl {
    implementation_name: Some(t_name),
    random: Some(t_random),
    stir: None,
    uniform: None,
    buf: Some(t_buf),
    close: None,
};
/// `uniform != NULL` with a value that could not come from the default sampler.
static IMPL_ODD_UNIFORM: RandombytesImpl = RandombytesImpl {
    implementation_name: Some(t_name),
    random: Some(t_random),
    stir: None,
    uniform: Some(t_uniform),
    buf: Some(t_buf),
    close: None,
};

fn set_impl_both(p: *const RandombytesImpl) -> (c_int, c_int) {
    let (c, r) = fns!("randombytes_set_implementation", SetImplFn);
    unsafe { (c(p), r(p)) }
}

/// Put both libraries back on their own `randombytes_sysrandom_implementation`.
fn restore_sysrandom() {
    let l = libs();
    let (c, r) = fns!("randombytes_set_implementation", SetImplFn);
    let pc = data_ptr::<RandombytesImpl>(&l.c, "randombytes_sysrandom_implementation");
    let pr = data_ptr::<RandombytesImpl>(&l.r, "randombytes_sysrandom_implementation");
    unsafe {
        c(pc);
        r(pr);
    }
}

/// CONFIGS row 48 + ERRORS rows 72/80: the default implementation is
/// `"sysrandom"`; `randombytes_set_implementation(NULL)` returns 0 and makes the
/// next call fall back to `&randombytes_sysrandom_implementation` (+ stir).
#[test]
fn cfg48_err72_err80_default_impl_and_seedbytes() {
    let _g = rng_lock();
    init_both();
    let l = libs();

    let (csb, rsb) = fns!("randombytes_seedbytes", SizeFn);
    for i in 0..8 {
        let (a, b) = unsafe { (csb(), rsb()) };
        assert_eq!(a, b, "row48 randombytes_seedbytes #{i}: C={a} rust={b}");
        assert_eq!(a, 32, "err80 randombytes_seedbytes must be 32");
    }

    // ERRORS row 72: NULL is accepted and resets to the default implementation.
    let (a, b) = set_impl_both(std::ptr::null());
    assert_eq!(a, b, "err72 set_implementation(NULL): C={a} rust={b}");
    assert_eq!(a, 0, "err72 set_implementation(NULL) must return 0");

    let (cn, rn) = fns!("randombytes_implementation_name", NameFn);
    for i in 0..8 {
        let (x, y) = unsafe { (CStr::from_ptr(cn()), CStr::from_ptr(rn())) };
        assert_eq!(x, y, "row48 implementation_name #{i}: C={x:?} rust={y:?}");
        assert_eq!(
            x.to_bytes(),
            b"sysrandom",
            "row48/err72 default implementation must be \"sysrandom\", got {x:?}"
        );
    }

    // The exported struct itself must agree field-for-field on NULL-ness.
    for name in [
        "randombytes_sysrandom_implementation",
        "randombytes_internal_implementation",
    ] {
        let pc = data_ptr::<RandombytesImpl>(&l.c, name);
        let pr = data_ptr::<RandombytesImpl>(&l.r, name);
        let (sc, sr) = unsafe { (&*pc, &*pr) };
        assert_eq!(
            sc.implementation_name.is_some(),
            sr.implementation_name.is_some(),
            "{name}.implementation_name NULL-ness differs"
        );
        assert!(sc.implementation_name.is_some(), "{name}.implementation_name is NULL in C");
        assert_eq!(sc.random.is_some(), sr.random.is_some(), "{name}.random NULL-ness");
        assert!(sc.random.is_some(), "{name}.random is NULL in C");
        assert_eq!(sc.stir.is_some(), sr.stir.is_some(), "{name}.stir NULL-ness");
        assert_eq!(sc.uniform.is_some(), sr.uniform.is_some(), "{name}.uniform NULL-ness");
        assert!(
            sc.uniform.is_none(),
            "{name}.uniform must be NULL in this build"
        );
        assert_eq!(sc.buf.is_some(), sr.buf.is_some(), "{name}.buf NULL-ness");
        assert!(sc.buf.is_some(), "{name}.buf is NULL in C");
        assert_eq!(sc.close.is_some(), sr.close.is_some(), "{name}.close NULL-ness");
        // the name callback of each exported struct must agree
        let (x, y) = unsafe {
            (
                CStr::from_ptr((sc.implementation_name.unwrap())()),
                CStr::from_ptr((sr.implementation_name.unwrap())()),
            )
        };
        assert_eq!(x, y, "{name}: implementation_name() C={x:?} rust={y:?}");
    }
    restore_sysrandom();
}

/// CONFIGS row 49 + ERRORS rows 71/77: an injected counter implementation with
/// `uniform == NULL`; `randombytes_random` / `randombytes_buf` must produce
/// identical streams, and `size == 0` must write nothing and consume nothing.
#[test]
fn cfg49_err71_err77_injected_impl_random_and_buf() {
    let _g = rng_lock();
    init_both();

    // ERRORS row 71: any implementation is accepted, unvalidated.
    for p in [
        &IMPL_MINIMAL as *const RandombytesImpl,
        &IMPL_ODD_UNIFORM as *const RandombytesImpl,
        std::ptr::null(),
    ] {
        let (a, b) = set_impl_both(p);
        assert_eq!(a, b, "err71 set_implementation: C={a} rust={b}");
        assert_eq!(a, 0, "err71 set_implementation must always return 0");
    }

    install_det_rng(false);
    let (crand, rrand) = fns!("randombytes_random", RandomFn);
    let (cbuf, rbuf) = fns!("randombytes_buf", BufFn);

    // lockstep stream of randombytes_random()
    reset_det_rng();
    for i in 0..256 {
        let (a, b) = unsafe { (crand(), rrand()) };
        assert_eq!(a, b, "row49 randombytes_random #{i}: C={a:#010x} rust={b:#010x}");
    }

    // randombytes_buf over a size sweep, sentinel-guarded
    let mut cases = 0usize;
    for &size in &[
        0usize, 1, 2, 3, 7, 8, 15, 16, 17, 31, 32, 33, 63, 64, 65, 127, 128, 129, 255, 256, 1000,
        4096, 65536,
    ] {
        reset_det_rng();
        let mut a = vec![SENT; size + 64];
        let mut b = vec![SENT; size + 64];
        unsafe {
            cbuf(a.as_mut_ptr() as *mut c_void, size);
            rbuf(b.as_mut_ptr() as *mut c_void, size);
        }
        let what = format!("row49 randombytes_buf size={size}");
        assert_eq_bytes(&what, &a, &b);
        assert!(a[size..].iter().all(|&v| v == SENT), "{what}: wrote past size");
        cases += 1;
    }

    // ERRORS row 77: size == 0 writes nothing AND consumes nothing.
    for i in 0..8 {
        reset_det_rng();
        let mut a = vec![SENT; 64];
        let mut b = vec![SENT; 64];
        unsafe {
            cbuf(a.as_mut_ptr() as *mut c_void, 0);
            rbuf(b.as_mut_ptr() as *mut c_void, 0);
        }
        assert_eq_bytes(&format!("err77 randombytes_buf(_, 0) #{i}"), &a, &b);
        assert!(
            a.iter().all(|&v| v == SENT) && b.iter().all(|&v| v == SENT),
            "err77 randombytes_buf(_, 0) wrote to the buffer"
        );
        let (x, y) = unsafe { (crand(), rrand()) };
        assert_eq!(x, y, "err77 stream after size=0: C={x:#010x} rust={y:#010x}");
        // the counter must still be at position 0 for both libraries
        reset_det_rng();
        let (x2, y2) = unsafe { (crand(), rrand()) };
        assert_eq!(
            (x, y),
            (x2, y2),
            "err77 randombytes_buf(_, 0) consumed from the RNG"
        );
    }

    // the same for the NaCl-compat wrapper (CONFIGS row 53, first half)
    for &size in &[0u64, 1, 16, 64, 1000] {
        reset_det_rng();
        let mut a = vec![SENT; size as usize + 64];
        let mut b = vec![SENT; size as usize + 64];
        let (cn, rn) = fns!("randombytes", NaclFn);
        unsafe {
            cn(a.as_mut_ptr(), size);
            rn(b.as_mut_ptr(), size);
        }
        let what = format!("row53 randombytes(NaCl) size={size}");
        assert_eq_bytes(&what, &a, &b);
        assert!(
            a[size as usize..].iter().all(|&v| v == SENT),
            "{what}: wrote past buf_len"
        );
        cases += 1;
    }
    assert!(cases >= 20, "row49: only {cases} buffer cases");
    restore_sysrandom();
}

/// CONFIGS row 50 + ERRORS rows 73/74: `randombytes_uniform` with
/// `impl->uniform == NULL`, i.e. libsodium's own rejection sampler.
#[test]
fn cfg50_err73_err74_uniform_rejection_sampler() {
    let _g = rng_lock();
    init_both();
    install_det_rng(false);
    let (cu, ru) = fns!("randombytes_uniform", UniformFn);
    let mut rng = Rng::new(SEED ^ 0x50);

    let mut bounds: Vec<u32> = vec![
        0, 1, 2, 3, 7, 0x7fffffff, 0x80000000, 0x80000001, 0xfffffffe, 0xffffffff, 4, 5, 6, 8, 10,
        100, 1000, 0x10000, 0x1000000,
    ];
    for _ in 0..96 {
        bounds.push(rng.next_u32());
    }
    for _ in 0..32 {
        bounds.push(rng.next_u32() % 1000 + 1);
    }

    for (i, &ub) in bounds.iter().enumerate() {
        reset_det_rng();
        let (a, b) = unsafe { (cu(ub), ru(ub)) };
        assert_eq!(
            a, b,
            "row50 randombytes_uniform(upper_bound={ub}) #{i}: C={a} rust={b}"
        );
        if ub < 2 {
            // ERRORS rows 73/74
            assert_eq!(a, 0, "err73/74 uniform({ub}) must be 0, got {a}");
        } else {
            assert!(a < ub, "row50 uniform({ub}) returned out-of-range {a}");
        }
    }

    // long lockstep run: both libraries must consume the SAME number of draws
    // from their (identical) streams, otherwise they desynchronise.
    reset_det_rng();
    for i in 0..512 {
        let ub = 1 + (i as u32 % 977);
        let (a, b) = unsafe { (cu(ub), ru(ub)) };
        assert_eq!(a, b, "row50 lockstep uniform({ub}) #{i}: C={a} rust={b}");
    }
    // ... and the streams are still aligned afterwards
    let (crand, rrand) = fns!("randombytes_random", RandomFn);
    for i in 0..16 {
        let (a, b) = unsafe { (crand(), rrand()) };
        assert_eq!(a, b, "row50 stream desynchronised after uniform() #{i}");
    }
    restore_sysrandom();
}

/// CONFIGS row 51 + ERRORS row 75: `impl->uniform != NULL` delegates entirely,
/// including for `upper_bound` 0 and 1.
#[test]
fn cfg51_err75_uniform_delegation() {
    let _g = rng_lock();
    init_both();
    let (cu, ru) = fns!("randombytes_uniform", UniformFn);
    let mut rng = Rng::new(SEED ^ 0x51);

    // (a) the harness' delegating implementation (consumes the counter stream)
    install_det_rng(true);
    let mut bounds: Vec<u32> = vec![0, 1, 2, 3, 7, 0x7fffffff, 0x80000001, 0xffffffff];
    for _ in 0..96 {
        bounds.push(rng.next_u32());
    }
    for (i, &ub) in bounds.iter().enumerate() {
        reset_det_rng();
        let (a, b) = unsafe { (cu(ub), ru(ub)) };
        assert_eq!(
            a, b,
            "row51 delegated uniform({ub}) #{i}: C={a} rust={b}"
        );
        if ub < 2 {
            assert_eq!(a, 0, "row51 delegated uniform({ub}) must be 0");
        }
    }
    reset_det_rng();
    for i in 0..256 {
        let ub = 1 + (i as u32 % 4093);
        let (a, b) = unsafe { (cu(ub), ru(ub)) };
        assert_eq!(a, b, "row51 lockstep delegated uniform({ub}) #{i}");
    }

    // (b) a delegate whose result the default sampler could never produce:
    //     proves the value comes from impl->uniform and nowhere else.
    let (a, b) = set_impl_both(&IMPL_ODD_UNIFORM as *const RandombytesImpl);
    assert_eq!((a, b), (0, 0), "row51 set_implementation");
    for &ub in &[0u32, 1, 2, 3, 7, 0x7fffffff, 0x80000001, 0xffffffff, 12345] {
        let (x, y) = unsafe { (cu(ub), ru(ub)) };
        assert_eq!(x, y, "err75 delegated uniform({ub}): C={x} rust={y}");
        assert_eq!(
            x,
            ub ^ 0xa5a5_a5a5,
            "err75 uniform({ub}) was not delegated to impl->uniform (got {x})"
        );
    }
    restore_sysrandom();
}

/// CONFIGS row 52 + ERRORS rows 78/79: `randombytes_stir` / `randombytes_close`
/// idempotency, and `close` with `impl->close == NULL`.
#[test]
fn cfg52_err78_err79_stir_and_close() {
    let _g = rng_lock();
    init_both();
    let (cs, rs) = fns!("randombytes_stir", VoidFn);
    let (cc, rc) = fns!("randombytes_close", IntFn);

    // (a) implementation with stir != NULL, close != NULL (the harness' one)
    install_det_rng(false);
    for i in 0..4 {
        unsafe {
            cs();
            rs();
        }
        clear_errno();
        let a = unsafe { cc() };
        let ea = errno();
        clear_errno();
        let b = unsafe { rc() };
        let eb = errno();
        assert_eq!(a, b, "row52 randombytes_close #{i}: C={a} rust={b}");
        assert_eq!(ea, eb, "row52 randombytes_close errno #{i}: C={ea} rust={eb}");
    }

    // (b) ERRORS row 78: impl->close == NULL (and impl->stir == NULL) -> 0
    let (a, b) = set_impl_both(&IMPL_MINIMAL as *const RandombytesImpl);
    assert_eq!((a, b), (0, 0), "err78 set_implementation");
    for i in 0..8 {
        unsafe {
            cs();
            rs();
        } // stir == NULL must be skipped, not called
        let (a, b) = unsafe { (cc(), rc()) };
        assert_eq!(a, b, "err78 close (impl->close == NULL) #{i}: C={a} rust={b}");
        assert_eq!(a, 0, "err78 close with impl->close == NULL must return 0");
    }

    // (c) ERRORS row 79: the sysrandom implementation, closed repeatedly.
    // NOTE: on this kernel `getrandom(2)` is available, so
    // `stream.getrandom_available != 0` forces the return value to 0 and the
    // documented `-1` (second close of the /dev/urandom fd) is unreachable in
    // this build. The C and Rust return values are still compared call by call.
    restore_sysrandom();
    for i in 0..4 {
        clear_errno();
        let a = unsafe { cc() };
        let ea = errno();
        clear_errno();
        let b = unsafe { rc() };
        let eb = errno();
        assert_eq!(a, b, "err79 sysrandom close #{i}: C={a} rust={b}");
        assert_eq!(ea, eb, "err79 sysrandom close errno #{i}: C={ea} rust={eb}");
    }
    // the implementation must still work after being closed
    for i in 0..4 {
        unsafe {
            cs();
            rs();
        }
        let (cn, rn) = fns!("randombytes_implementation_name", NameFn);
        let (x, y) = unsafe { (CStr::from_ptr(cn()), CStr::from_ptr(rn())) };
        assert_eq!(x, y, "row52 name after close #{i}: C={x:?} rust={y:?}");
        assert_eq!(x.to_bytes(), b"sysrandom");
    }
    restore_sysrandom();
}

/// CONFIGS row 53: the NaCl-compat `randombytes()` entry point and the exported
/// `randombytes_internal_implementation` (name `"internal"`, `uniform == NULL`).
/// Its output is real entropy, so only the observable contract is compared.
#[test]
fn cfg53_nacl_compat_and_internal_implementation() {
    let _g = rng_lock();
    init_both();
    let l = libs();
    let (csi, rsi) = fns!("randombytes_set_implementation", SetImplFn);
    let pc = data_ptr::<RandombytesImpl>(&l.c, "randombytes_internal_implementation");
    let pr = data_ptr::<RandombytesImpl>(&l.r, "randombytes_internal_implementation");

    // uniform must be NULL in both exported structs
    unsafe {
        assert_eq!(
            (*pc).uniform.is_some(),
            (*pr).uniform.is_some(),
            "row53 internal_implementation.uniform NULL-ness differs"
        );
        assert!(
            (*pc).uniform.is_none(),
            "row53 internal_implementation.uniform must be NULL"
        );
    }

    let (a, b) = unsafe { (csi(pc), rsi(pr)) };
    assert_eq!(a, b, "row53 set_implementation(internal): C={a} rust={b}");
    assert_eq!(a, 0, "row53 set_implementation must return 0");

    let (cn, rn) = fns!("randombytes_implementation_name", NameFn);
    for i in 0..4 {
        let (x, y) = unsafe { (CStr::from_ptr(cn()), CStr::from_ptr(rn())) };
        assert_eq!(x, y, "row53 implementation_name #{i}: C={x:?} rust={y:?}");
        assert_eq!(x.to_bytes(), b"internal", "row53 name must be \"internal\"");
    }

    // randombytes() / randombytes_buf() / randombytes_random(): the bytes are
    // real entropy, so assert the shape contract on BOTH libraries.
    let (cnacl, rnacl) = fns!("randombytes", NaclFn);
    let (cbuf, rbuf) = fns!("randombytes_buf", BufFn);
    let (crand, rrand) = fns!("randombytes_random", RandomFn);
    for &size in &[0usize, 1, 16, 31, 32, 33, 64, 1000, 4096] {
        for (tag, f) in [("C", cnacl), ("rust", rnacl)] {
            let mut v = vec![SENT; size + 64];
            unsafe { f(v.as_mut_ptr(), size as u64) };
            assert!(
                v[size..].iter().all(|&x| x == SENT),
                "row53 randombytes[{tag}] size={size}: wrote past buf_len"
            );
            if size >= 16 {
                assert!(
                    v[..size].iter().any(|&x| x != SENT),
                    "row53 randombytes[{tag}] size={size}: nothing written"
                );
            }
        }
        for (tag, f) in [("C", cbuf), ("rust", rbuf)] {
            let mut v = vec![SENT; size + 64];
            unsafe { f(v.as_mut_ptr() as *mut c_void, size) };
            assert!(
                v[size..].iter().all(|&x| x == SENT),
                "row53 randombytes_buf[{tag}] size={size}: wrote past size"
            );
        }
    }
    // `random` / `uniform` (uniform == NULL -> default sampler over impl->random)
    let (cu, ru) = fns!("randombytes_uniform", UniformFn);
    let mut seen_c = std::collections::HashSet::new();
    let mut seen_r = std::collections::HashSet::new();
    for _ in 0..64 {
        seen_c.insert(unsafe { crand() });
        seen_r.insert(unsafe { rrand() });
        for &ub in &[1u32, 2, 7, 1000, 0x80000001] {
            let (x, y) = unsafe { (cu(ub), ru(ub)) };
            if ub < 2 {
                assert_eq!((x, y), (0, 0), "row53 uniform({ub}) must be 0");
            } else {
                assert!(x < ub, "row53 C uniform({ub}) out of range: {x}");
                assert!(y < ub, "row53 rust uniform({ub}) out of range: {y}");
            }
        }
    }
    assert!(seen_c.len() > 32, "row53 C internal random() is not varying");
    assert!(seen_r.len() > 32, "row53 rust internal random() is not varying");

    // close the internal implementation on both, then compare
    let (cc, rc) = fns!("randombytes_close", IntFn);
    for i in 0..3 {
        clear_errno();
        let a = unsafe { cc() };
        let ea = errno();
        clear_errno();
        let b = unsafe { rc() };
        let eb = errno();
        assert_eq!(a, b, "row53 internal close #{i}: C={a} rust={b}");
        assert_eq!(ea, eb, "row53 internal close errno #{i}: C={ea} rust={eb}");
    }
    restore_sysrandom();
}
