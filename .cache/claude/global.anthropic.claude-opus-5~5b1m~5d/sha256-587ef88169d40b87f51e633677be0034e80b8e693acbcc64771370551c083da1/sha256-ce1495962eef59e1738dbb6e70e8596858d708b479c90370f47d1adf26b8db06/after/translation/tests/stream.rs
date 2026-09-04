//! Differential tests for the `stream` area:
//!
//!   * `crypto_stream/crypto_stream.c`            (generic xsalsa20 dispatchers)
//!   * `crypto_stream/salsa20/**`                 (ref impl + implementation struct)
//!   * `crypto_stream/salsa2012/**`
//!   * `crypto_stream/salsa208/**`
//!   * `crypto_stream/xsalsa20/**`
//!   * `crypto_stream/chacha20/**`                (original + ietf + ietf_ext)
//!   * `crypto_stream/xchacha20/**`
//!   * `crypto_core/salsa/ref/core_salsa_ref.c`   (salsa20 / 2012 / 208 cores)
//!   * `crypto_core/hsalsa20/**`
//!   * `crypto_core/hchacha20/**`
//!
//! Everything is called through `dlopen`'d C and Rust shared objects.

#[macro_use]
mod common;

use core::ffi::{c_char, c_int, c_void};

// ------------------------------------------------------------- signatures ----

type StreamFn = unsafe extern "C" fn(*mut u8, u64, *const u8, *const u8) -> c_int;
type XorFn = unsafe extern "C" fn(*mut u8, *const u8, u64, *const u8, *const u8) -> c_int;
type XorIcFn = unsafe extern "C" fn(*mut u8, *const u8, u64, *const u8, u64, *const u8) -> c_int;
type XorIc32Fn = unsafe extern "C" fn(*mut u8, *const u8, u64, *const u8, u32, *const u8) -> c_int;
type CoreFn = unsafe extern "C" fn(*mut u8, *const u8, *const u8, *const u8) -> c_int;
type SizeFn = unsafe extern "C" fn() -> usize;
type KeygenFn = unsafe extern "C" fn(*mut u8);
type IntFn = unsafe extern "C" fn() -> c_int;
type PrimFn = unsafe extern "C" fn() -> *const c_char;

/// `crypto_stream_salsa20_implementation` (crypto_stream/salsa20/stream_salsa20.h)
#[repr(C)]
struct Salsa20Impl {
    stream: StreamFn,
    stream_xor_ic: XorIcFn,
}

/// `crypto_stream_chacha20_implementation` (crypto_stream/chacha20/stream_chacha20.h)
#[repr(C)]
struct Chacha20Impl {
    stream: StreamFn,
    stream_ietf_ext: StreamFn,
    stream_xor_ic: XorIcFn,
    stream_ietf_ext_xor_ic: XorIc32Fn,
}

/// Runtime-name version of `both!` (the macro only accepts string literals).
fn pair<T: Copy>(name: &str) -> (T, T) {
    let l = common::libs();
    let mut nm = name.as_bytes().to_vec();
    nm.push(0);
    unsafe {
        let cs: libloading::Symbol<T> = l
            .c
            .get(&nm)
            .unwrap_or_else(|e| panic!("C library missing {}: {}", name, e));
        let rs: libloading::Symbol<T> = l
            .r
            .get(&nm)
            .unwrap_or_else(|e| panic!("Rust library missing {}: {}", name, e));
        (*cs, *rs)
    }
}

/// Compare a `size_t (*)(void)` getter across both libraries and return the value.
fn sz(name: &str) -> usize {
    let (c, r) = pair::<SizeFn>(name);
    let cv = unsafe { c() };
    let rv = unsafe { r() };
    assert_eq!(cv, rv, "{}: value mismatch", name);
    cv
}

// ------------------------------------------------------------- input sets ----

/// Message/keystream lengths: 0, 1, block-1/block/block+1 for the 64-byte salsa
/// and chacha block, multi-block, and plenty of non-multiples of 64.
const LENS: &[usize] = &[
    0, 1, 2, 3, 31, 32, 33, 63, 64, 65, 66, 100, 127, 128, 129, 130, 191, 192, 193, 255, 256, 257,
    320, 383, 384, 385, 511, 512, 513, 1000, 1023, 1024, 1025,
];

/// 64-bit initial counters. Includes the 32-bit rollover point (0xffffffff, so
/// the low counter word wraps into the high word after the first block) and the
/// full 64-bit rollover point (u64::MAX).
const ICS: &[u64] = &[
    0,
    1,
    2,
    7,
    0xffff_fffe,
    0xffff_ffff,
    0x1_0000_0000,
    0x1_0000_0001,
    0xdead_beef_1234_5678,
    0xffff_ffff_ffff_fffe,
    u64::MAX,
];

/// 32-bit initial counters for the ietf_ext entry points (the counter is a
/// single 32-bit word there and is allowed to overflow into the IV).
const ICS32: &[u32] = &[0, 1, 2, 0x7fff_ffff, 0x8000_0000, 0xffff_fffe, 0xffff_ffff];

const PAD: usize = 32;
const CAN: u8 = 0x5A;

fn lens_with_random(rng: &mut common::Rng) -> Vec<usize> {
    let mut v = LENS.to_vec();
    for _ in 0..20 {
        v.push(rng.below(1500));
    }
    v
}

// --------------------------------------------------------------- helpers -----

/// Keystream-generation entry points: `crypto_stream*(c, clen, n, k)`.
fn check_stream(name: &str, cf: StreamFn, rf: StreamFn, noncebytes: usize, rng: &mut common::Rng) {
    for &len in &lens_with_random(rng) {
        let k = rng.bytes(32);
        let n = rng.bytes(noncebytes);
        let mut cb = vec![CAN; len + PAD];
        let mut rb = vec![CAN; len + PAD];
        let rc = unsafe { cf(cb.as_mut_ptr(), len as u64, n.as_ptr(), k.as_ptr()) };
        let rr = unsafe { rf(rb.as_mut_ptr(), len as u64, n.as_ptr(), k.as_ptr()) };
        let tag = format!("{} len={}", name, len);
        common::eqi(&tag, rc, rr);
        common::eqb(&tag, &cb, &rb);
        assert!(
            cb[len..].iter().all(|&b| b == CAN),
            "{}: wrote past the requested length",
            tag
        );
    }
}

/// `*_xor(c, m, mlen, n, k)` — out-of-place and in-place.
fn check_xor(name: &str, cf: XorFn, rf: XorFn, noncebytes: usize, rng: &mut common::Rng) {
    for &len in &lens_with_random(rng) {
        let k = rng.bytes(32);
        let n = rng.bytes(noncebytes);
        let m = rng.bytes(len);
        let mut cb = vec![CAN; len + PAD];
        let mut rb = vec![CAN; len + PAD];
        let rc = unsafe { cf(cb.as_mut_ptr(), m.as_ptr(), len as u64, n.as_ptr(), k.as_ptr()) };
        let rr = unsafe { rf(rb.as_mut_ptr(), m.as_ptr(), len as u64, n.as_ptr(), k.as_ptr()) };
        let tag = format!("{} len={}", name, len);
        common::eqi(&tag, rc, rr);
        common::eqb(&tag, &cb, &rb);
        assert!(cb[len..].iter().all(|&b| b == CAN), "{}: wrote past mlen", tag);

        // in-place (c == m), which the C reference code supports
        let mut ci = m.clone();
        ci.extend(core::iter::repeat(CAN).take(PAD));
        let mut ri = ci.clone();
        let rc2 = unsafe {
            cf(
                ci.as_mut_ptr(),
                ci.as_ptr(),
                len as u64,
                n.as_ptr(),
                k.as_ptr(),
            )
        };
        let rr2 = unsafe {
            rf(
                ri.as_mut_ptr(),
                ri.as_ptr(),
                len as u64,
                n.as_ptr(),
                k.as_ptr(),
            )
        };
        let tag2 = format!("{} inplace len={}", name, len);
        common::eqi(&tag2, rc2, rr2);
        common::eqb(&tag2, &ci, &ri);
        common::eqb(
            &format!("{} inplace==oop len={}", name, len),
            &ci[..len],
            &cb[..len],
        );
    }
}

/// `*_xor_ic(c, m, mlen, n, ic, k)` with a 64-bit counter.
fn check_xor_ic(name: &str, cf: XorIcFn, rf: XorIcFn, noncebytes: usize, rng: &mut common::Rng) {
    for &len in &lens_with_random(rng) {
        for &ic in ICS {
            let k = rng.bytes(32);
            let n = rng.bytes(noncebytes);
            let m = rng.bytes(len);
            let mut cb = vec![CAN; len + PAD];
            let mut rb = vec![CAN; len + PAD];
            let rc = unsafe {
                cf(
                    cb.as_mut_ptr(),
                    m.as_ptr(),
                    len as u64,
                    n.as_ptr(),
                    ic,
                    k.as_ptr(),
                )
            };
            let rr = unsafe {
                rf(
                    rb.as_mut_ptr(),
                    m.as_ptr(),
                    len as u64,
                    n.as_ptr(),
                    ic,
                    k.as_ptr(),
                )
            };
            let tag = format!("{} len={} ic={:#x}", name, len, ic);
            common::eqi(&tag, rc, rr);
            common::eqb(&tag, &cb, &rb);
            assert!(cb[len..].iter().all(|&b| b == CAN), "{}: wrote past mlen", tag);

            // in-place
            let mut ci = m.clone();
            ci.extend(core::iter::repeat(CAN).take(PAD));
            let mut ri = ci.clone();
            let rc2 = unsafe {
                cf(
                    ci.as_mut_ptr(),
                    ci.as_ptr(),
                    len as u64,
                    n.as_ptr(),
                    ic,
                    k.as_ptr(),
                )
            };
            let rr2 = unsafe {
                rf(
                    ri.as_mut_ptr(),
                    ri.as_ptr(),
                    len as u64,
                    n.as_ptr(),
                    ic,
                    k.as_ptr(),
                )
            };
            let tag2 = format!("{} inplace len={} ic={:#x}", name, len, ic);
            common::eqi(&tag2, rc2, rr2);
            common::eqb(&tag2, &ci, &ri);
            common::eqb(&tag2, &ci[..len], &cb[..len]);
        }
    }
}

/// `*_xor_ic(c, m, mlen, n, ic, k)` with a 32-bit counter (ietf_ext).
fn check_xor_ic32(
    name: &str,
    cf: XorIc32Fn,
    rf: XorIc32Fn,
    noncebytes: usize,
    rng: &mut common::Rng,
) {
    for &len in &lens_with_random(rng) {
        for &ic in ICS32 {
            let k = rng.bytes(32);
            let n = rng.bytes(noncebytes);
            let m = rng.bytes(len);
            let mut cb = vec![CAN; len + PAD];
            let mut rb = vec![CAN; len + PAD];
            let rc = unsafe {
                cf(
                    cb.as_mut_ptr(),
                    m.as_ptr(),
                    len as u64,
                    n.as_ptr(),
                    ic,
                    k.as_ptr(),
                )
            };
            let rr = unsafe {
                rf(
                    rb.as_mut_ptr(),
                    m.as_ptr(),
                    len as u64,
                    n.as_ptr(),
                    ic,
                    k.as_ptr(),
                )
            };
            let tag = format!("{} len={} ic={:#x}", name, len, ic);
            common::eqi(&tag, rc, rr);
            common::eqb(&tag, &cb, &rb);
            assert!(cb[len..].iter().all(|&b| b == CAN), "{}: wrote past mlen", tag);

            let mut ci = m.clone();
            ci.extend(core::iter::repeat(CAN).take(PAD));
            let mut ri = ci.clone();
            let rc2 = unsafe {
                cf(
                    ci.as_mut_ptr(),
                    ci.as_ptr(),
                    len as u64,
                    n.as_ptr(),
                    ic,
                    k.as_ptr(),
                )
            };
            let rr2 = unsafe {
                rf(
                    ri.as_mut_ptr(),
                    ri.as_ptr(),
                    len as u64,
                    n.as_ptr(),
                    ic,
                    k.as_ptr(),
                )
            };
            let tag2 = format!("{} inplace len={} ic={:#x}", name, len, ic);
            common::eqi(&tag2, rc2, rr2);
            common::eqb(&tag2, &ci, &ri);
            common::eqb(&tag2, &ci[..len], &cb[..len]);
        }
    }
}

// =====================================================================
// crypto_core_* (salsa20 / salsa2012 / salsa208 / hsalsa20 / hchacha20)
// =====================================================================

fn check_core(name: &str, cf: CoreFn, rf: CoreFn, outlen: usize, rng: &mut common::Rng) {
    // 40 random cases per `c == NULL` / `c != NULL` branch, plus all-zero and
    // all-0xff extremes.
    for case in 0..44 {
        let (in_, k, konst) = match case {
            0 => (vec![0u8; 16], vec![0u8; 32], vec![0u8; 16]),
            1 => (vec![0xffu8; 16], vec![0xffu8; 32], vec![0xffu8; 16]),
            2 => (vec![0u8; 16], vec![0xffu8; 32], vec![0x5au8; 16]),
            3 => (vec![0xffu8; 16], vec![0u8; 32], vec![0xa5u8; 16]),
            _ => (rng.bytes(16), rng.bytes(32), rng.bytes(16)),
        };
        for use_const in [false, true] {
            let cp = if use_const {
                konst.as_ptr()
            } else {
                core::ptr::null()
            };
            let mut co = vec![CAN; outlen + PAD];
            let mut ro = vec![CAN; outlen + PAD];
            let rc = unsafe { cf(co.as_mut_ptr(), in_.as_ptr(), k.as_ptr(), cp) };
            let rr = unsafe { rf(ro.as_mut_ptr(), in_.as_ptr(), k.as_ptr(), cp) };
            let tag = format!("{} case={} const={}", name, case, use_const);
            common::eqi(&tag, rc, rr);
            common::eqb(&tag, &co, &ro);
            assert!(
                co[outlen..].iter().all(|&b| b == CAN),
                "{}: wrote past outputbytes",
                tag
            );
        }
    }
}

fn core_getters(prefix: &str, out: usize, inb: usize, keyb: usize, constb: usize) {
    assert_eq!(sz(&format!("{}_outputbytes", prefix)), out);
    assert_eq!(sz(&format!("{}_inputbytes", prefix)), inb);
    assert_eq!(sz(&format!("{}_keybytes", prefix)), keyb);
    assert_eq!(sz(&format!("{}_constbytes", prefix)), constb);
}

#[test]
fn core_salsa20() {
    core_getters("crypto_core_salsa20", 64, 16, 32, 16);
    let (c, r) = pair::<CoreFn>("crypto_core_salsa20");
    check_core("crypto_core_salsa20", c, r, 64, &mut common::Rng::new(0x0001));
}

#[test]
fn core_salsa2012() {
    core_getters("crypto_core_salsa2012", 64, 16, 32, 16);
    let (c, r) = pair::<CoreFn>("crypto_core_salsa2012");
    check_core(
        "crypto_core_salsa2012",
        c,
        r,
        64,
        &mut common::Rng::new(0x0002),
    );
}

#[test]
fn core_salsa208() {
    core_getters("crypto_core_salsa208", 64, 16, 32, 16);
    let (c, r) = pair::<CoreFn>("crypto_core_salsa208");
    check_core(
        "crypto_core_salsa208",
        c,
        r,
        64,
        &mut common::Rng::new(0x0003),
    );
}

#[test]
fn core_hsalsa20() {
    core_getters("crypto_core_hsalsa20", 32, 16, 32, 16);
    let (c, r) = pair::<CoreFn>("crypto_core_hsalsa20");
    check_core(
        "crypto_core_hsalsa20",
        c,
        r,
        32,
        &mut common::Rng::new(0x0004),
    );
}

#[test]
fn core_hchacha20() {
    core_getters("crypto_core_hchacha20", 32, 16, 32, 16);
    let (c, r) = pair::<CoreFn>("crypto_core_hchacha20");
    check_core(
        "crypto_core_hchacha20",
        c,
        r,
        32,
        &mut common::Rng::new(0x0005),
    );
}

// =====================================================================
// crypto_stream/salsa20
// =====================================================================

#[test]
fn stream_salsa20() {
    let mut rng = common::Rng::new(0x1001);
    assert_eq!(sz("crypto_stream_salsa20_keybytes"), 32);
    assert_eq!(sz("crypto_stream_salsa20_noncebytes"), 8);
    assert_eq!(sz("crypto_stream_salsa20_messagebytes_max"), usize::MAX);

    let (c, r) = pair::<StreamFn>("crypto_stream_salsa20");
    check_stream("crypto_stream_salsa20", c, r, 8, &mut rng);
    let (c, r) = pair::<XorFn>("crypto_stream_salsa20_xor");
    check_xor("crypto_stream_salsa20_xor", c, r, 8, &mut rng);
    let (c, r) = pair::<XorIcFn>("crypto_stream_salsa20_xor_ic");
    check_xor_ic("crypto_stream_salsa20_xor_ic", c, r, 8, &mut rng);
}

#[test]
fn stream_salsa20_ref_implementation_struct() {
    // Exported implementation struct: exercise every function pointer inside it
    // through both libraries.
    let (cp, rp) = both_data!("crypto_stream_salsa20_ref_implementation", Salsa20Impl);
    let (ci, ri) = unsafe { (&*cp, &*rp) };
    let mut rng = common::Rng::new(0x1002);
    check_stream("salsa20_ref_impl.stream", ci.stream, ri.stream, 8, &mut rng);
    check_xor_ic(
        "salsa20_ref_impl.stream_xor_ic",
        ci.stream_xor_ic,
        ri.stream_xor_ic,
        8,
        &mut rng,
    );
}

#[test]
fn stream_salsa20_pick_best_implementation() {
    let (c, r) = pair::<IntFn>("_crypto_stream_salsa20_pick_best_implementation");
    let rc = unsafe { c() };
    let rr = unsafe { r() };
    common::eqi("_crypto_stream_salsa20_pick_best_implementation", rc, rr);
    assert_eq!(rc, 0);
    // and the dispatchers still agree after re-picking
    let mut rng = common::Rng::new(0x1003);
    let (c, r) = pair::<StreamFn>("crypto_stream_salsa20");
    check_stream("crypto_stream_salsa20 (post-pick)", c, r, 8, &mut rng);
    let (c, r) = pair::<XorIcFn>("crypto_stream_salsa20_xor_ic");
    check_xor_ic("crypto_stream_salsa20_xor_ic (post-pick)", c, r, 8, &mut rng);
}

// =====================================================================
// crypto_stream/salsa2012
// =====================================================================

#[test]
fn stream_salsa2012() {
    let mut rng = common::Rng::new(0x2001);
    assert_eq!(sz("crypto_stream_salsa2012_keybytes"), 32);
    assert_eq!(sz("crypto_stream_salsa2012_noncebytes"), 8);
    assert_eq!(sz("crypto_stream_salsa2012_messagebytes_max"), usize::MAX);
    let (c, r) = pair::<StreamFn>("crypto_stream_salsa2012");
    check_stream("crypto_stream_salsa2012", c, r, 8, &mut rng);
    let (c, r) = pair::<XorFn>("crypto_stream_salsa2012_xor");
    check_xor("crypto_stream_salsa2012_xor", c, r, 8, &mut rng);
}

// =====================================================================
// crypto_stream/salsa208
// =====================================================================

#[test]
fn stream_salsa208() {
    let mut rng = common::Rng::new(0x3001);
    assert_eq!(sz("crypto_stream_salsa208_keybytes"), 32);
    assert_eq!(sz("crypto_stream_salsa208_noncebytes"), 8);
    assert_eq!(sz("crypto_stream_salsa208_messagebytes_max"), usize::MAX);
    let (c, r) = pair::<StreamFn>("crypto_stream_salsa208");
    check_stream("crypto_stream_salsa208", c, r, 8, &mut rng);
    let (c, r) = pair::<XorFn>("crypto_stream_salsa208_xor");
    check_xor("crypto_stream_salsa208_xor", c, r, 8, &mut rng);
}

// =====================================================================
// crypto_stream/xsalsa20
// =====================================================================

#[test]
fn stream_xsalsa20() {
    let mut rng = common::Rng::new(0x4001);
    assert_eq!(sz("crypto_stream_xsalsa20_keybytes"), 32);
    assert_eq!(sz("crypto_stream_xsalsa20_noncebytes"), 24);
    assert_eq!(sz("crypto_stream_xsalsa20_messagebytes_max"), usize::MAX);
    let (c, r) = pair::<StreamFn>("crypto_stream_xsalsa20");
    check_stream("crypto_stream_xsalsa20", c, r, 24, &mut rng);
    let (c, r) = pair::<XorFn>("crypto_stream_xsalsa20_xor");
    check_xor("crypto_stream_xsalsa20_xor", c, r, 24, &mut rng);
    let (c, r) = pair::<XorIcFn>("crypto_stream_xsalsa20_xor_ic");
    check_xor_ic("crypto_stream_xsalsa20_xor_ic", c, r, 24, &mut rng);
}

// =====================================================================
// crypto_stream/crypto_stream.c (generic dispatchers -> xsalsa20)
// =====================================================================

#[test]
fn stream_generic() {
    let mut rng = common::Rng::new(0x5001);
    assert_eq!(sz("crypto_stream_keybytes"), 32);
    assert_eq!(sz("crypto_stream_noncebytes"), 24);
    assert_eq!(sz("crypto_stream_messagebytes_max"), usize::MAX);

    let (c, r) = pair::<PrimFn>("crypto_stream_primitive");
    unsafe {
        let cs = std::ffi::CStr::from_ptr(c());
        let rs = std::ffi::CStr::from_ptr(r());
        assert_eq!(cs, rs, "crypto_stream_primitive mismatch");
        assert_eq!(cs.to_bytes(), b"xsalsa20");
    }

    let (c, r) = pair::<StreamFn>("crypto_stream");
    check_stream("crypto_stream", c, r, 24, &mut rng);
    let (c, r) = pair::<XorFn>("crypto_stream_xor");
    check_xor("crypto_stream_xor", c, r, 24, &mut rng);
}

// =====================================================================
// crypto_stream/chacha20 — original 8-byte-nonce variant
// =====================================================================

#[test]
fn stream_chacha20() {
    let mut rng = common::Rng::new(0x6001);
    assert_eq!(sz("crypto_stream_chacha20_keybytes"), 32);
    assert_eq!(sz("crypto_stream_chacha20_noncebytes"), 8);
    assert_eq!(sz("crypto_stream_chacha20_messagebytes_max"), usize::MAX);
    let (c, r) = pair::<StreamFn>("crypto_stream_chacha20");
    check_stream("crypto_stream_chacha20", c, r, 8, &mut rng);
    let (c, r) = pair::<XorFn>("crypto_stream_chacha20_xor");
    check_xor("crypto_stream_chacha20_xor", c, r, 8, &mut rng);
    let (c, r) = pair::<XorIcFn>("crypto_stream_chacha20_xor_ic");
    check_xor_ic("crypto_stream_chacha20_xor_ic", c, r, 8, &mut rng);
}

// =====================================================================
// crypto_stream/chacha20 — ietf (12-byte nonce, 32-bit counter)
// =====================================================================

#[test]
fn stream_chacha20_ietf() {
    let mut rng = common::Rng::new(0x7001);
    assert_eq!(sz("crypto_stream_chacha20_ietf_keybytes"), 32);
    assert_eq!(sz("crypto_stream_chacha20_ietf_noncebytes"), 12);
    // min(SODIUM_SIZE_MAX, 64 * 2^32) == 64 * 2^32 on 64-bit
    assert_eq!(
        sz("crypto_stream_chacha20_ietf_messagebytes_max"),
        64usize * (1usize << 32)
    );

    let (c, r) = pair::<StreamFn>("crypto_stream_chacha20_ietf");
    check_stream("crypto_stream_chacha20_ietf", c, r, 12, &mut rng);
    let (c, r) = pair::<XorFn>("crypto_stream_chacha20_ietf_xor");
    check_xor("crypto_stream_chacha20_ietf_xor", c, r, 12, &mut rng);

    // ietf_ext_* variants (internal, but exported): counter may overflow into IV
    let (c, r) = pair::<StreamFn>("crypto_stream_chacha20_ietf_ext");
    check_stream("crypto_stream_chacha20_ietf_ext", c, r, 12, &mut rng);
    let (c, r) = pair::<XorIc32Fn>("crypto_stream_chacha20_ietf_ext_xor_ic");
    check_xor_ic32("crypto_stream_chacha20_ietf_ext_xor_ic", c, r, 12, &mut rng);
}

/// `crypto_stream_chacha20_ietf_xor_ic` rejects `ic` when
/// `ic > (64*2^32)/64 - (mlen+63)/64`.  Here we hit the *largest accepted* `ic`
/// for a range of message lengths (the exact boundary of that check), plus
/// values below it.  The rejecting side aborts and is covered out-of-process by
/// `misuse_paths_abort_in_both_libraries`.
#[test]
fn stream_chacha20_ietf_xor_ic_counter_boundary() {
    let (c, r) = pair::<XorIc32Fn>("crypto_stream_chacha20_ietf_xor_ic");
    let mut rng = common::Rng::new(0x7002);

    // (mlen, largest ic still accepted) = (mlen, 2^32 - ceil(mlen/64)),
    // clamped to u32::MAX because ic is a uint32_t.
    let cases: &[(usize, u32)] = &[
        (0, 0xffff_ffff), // limit is 2^32, so any u32 ic is accepted
        (1, 0xffff_ffff),
        (63, 0xffff_ffff),
        (64, 0xffff_ffff),
        (65, 0xffff_fffe),
        (127, 0xffff_fffe),
        (128, 0xffff_fffe),
        (129, 0xffff_fffd),
        (192, 0xffff_fffd),
        (256, 0xffff_fffc),
        (1000, 0xffff_fff0),
        (1024, 0xffff_fff0),
        (1025, 0xffff_ffef),
    ];
    for &(len, ic_max) in cases {
        for ic in [ic_max, ic_max.wrapping_sub(1), 0, 1] {
            let k = rng.bytes(32);
            let n = rng.bytes(12);
            let m = rng.bytes(len);
            let mut cb = vec![CAN; len + PAD];
            let mut rb = vec![CAN; len + PAD];
            let rc = unsafe {
                c(
                    cb.as_mut_ptr(),
                    m.as_ptr(),
                    len as u64,
                    n.as_ptr(),
                    ic,
                    k.as_ptr(),
                )
            };
            let rr = unsafe {
                r(
                    rb.as_mut_ptr(),
                    m.as_ptr(),
                    len as u64,
                    n.as_ptr(),
                    ic,
                    k.as_ptr(),
                )
            };
            let tag = format!("ietf_xor_ic len={} ic={:#x}", len, ic);
            common::eqi(&tag, rc, rr);
            common::eqb(&tag, &cb, &rb);
            assert!(cb[len..].iter().all(|&b| b == CAN), "{}: overrun", tag);
        }
    }
}

#[test]
fn stream_chacha20_ref_implementation_struct() {
    let (cp, rp) = both_data!("crypto_stream_chacha20_ref_implementation", Chacha20Impl);
    let (ci, ri) = unsafe { (&*cp, &*rp) };
    let mut rng = common::Rng::new(0x6002);
    check_stream("chacha20_ref_impl.stream", ci.stream, ri.stream, 8, &mut rng);
    check_stream(
        "chacha20_ref_impl.stream_ietf_ext",
        ci.stream_ietf_ext,
        ri.stream_ietf_ext,
        12,
        &mut rng,
    );
    check_xor_ic(
        "chacha20_ref_impl.stream_xor_ic",
        ci.stream_xor_ic,
        ri.stream_xor_ic,
        8,
        &mut rng,
    );
    check_xor_ic32(
        "chacha20_ref_impl.stream_ietf_ext_xor_ic",
        ci.stream_ietf_ext_xor_ic,
        ri.stream_ietf_ext_xor_ic,
        12,
        &mut rng,
    );
}

#[test]
fn stream_chacha20_pick_best_implementation() {
    let (c, r) = pair::<IntFn>("_crypto_stream_chacha20_pick_best_implementation");
    let rc = unsafe { c() };
    let rr = unsafe { r() };
    common::eqi("_crypto_stream_chacha20_pick_best_implementation", rc, rr);
    assert_eq!(rc, 0);
    let mut rng = common::Rng::new(0x6003);
    let (c, r) = pair::<StreamFn>("crypto_stream_chacha20");
    check_stream("crypto_stream_chacha20 (post-pick)", c, r, 8, &mut rng);
    let (c, r) = pair::<XorIc32Fn>("crypto_stream_chacha20_ietf_ext_xor_ic");
    check_xor_ic32("ietf_ext_xor_ic (post-pick)", c, r, 12, &mut rng);
}

// =====================================================================
// crypto_stream/xchacha20
// =====================================================================

#[test]
fn stream_xchacha20() {
    let mut rng = common::Rng::new(0x8001);
    assert_eq!(sz("crypto_stream_xchacha20_keybytes"), 32);
    assert_eq!(sz("crypto_stream_xchacha20_noncebytes"), 24);
    assert_eq!(sz("crypto_stream_xchacha20_messagebytes_max"), usize::MAX);
    let (c, r) = pair::<StreamFn>("crypto_stream_xchacha20");
    check_stream("crypto_stream_xchacha20", c, r, 24, &mut rng);
    let (c, r) = pair::<XorFn>("crypto_stream_xchacha20_xor");
    check_xor("crypto_stream_xchacha20_xor", c, r, 24, &mut rng);
    let (c, r) = pair::<XorIcFn>("crypto_stream_xchacha20_xor_ic");
    check_xor_ic("crypto_stream_xchacha20_xor_ic", c, r, 24, &mut rng);
}

// =====================================================================
// Zero-length / NULL-pointer tolerance
// =====================================================================

#[test]
fn zero_length_null_pointers() {
    let mut rng = common::Rng::new(0x9001);
    let k = rng.bytes(32);
    let n24 = rng.bytes(24);
    let nul: *mut u8 = core::ptr::null_mut();
    let cnul: *const u8 = core::ptr::null();

    // The salsa20/2012/208 and chacha20 reference bodies do `if (!clen) return 0;`
    // *before* dereferencing any pointer, so all four pointers may be NULL.
    for name in [
        "crypto_stream_salsa20",
        "crypto_stream_salsa2012",
        "crypto_stream_salsa208",
        "crypto_stream_chacha20",
        "crypto_stream_chacha20_ietf",
        "crypto_stream_chacha20_ietf_ext",
    ] {
        let (c, r) = pair::<StreamFn>(name);
        let rc = unsafe { c(nul, 0, cnul, cnul) };
        let rr = unsafe { r(nul, 0, cnul, cnul) };
        common::eqi(&format!("{} clen=0 all-NULL", name), rc, rr);
        assert_eq!(rc, 0, "{} clen=0 should return 0", name);
    }
    for name in [
        "crypto_stream_salsa20_xor",
        "crypto_stream_salsa2012_xor",
        "crypto_stream_salsa208_xor",
        "crypto_stream_chacha20_xor",
        "crypto_stream_chacha20_ietf_xor",
    ] {
        let (c, r) = pair::<XorFn>(name);
        let rc = unsafe { c(nul, cnul, 0, cnul, cnul) };
        let rr = unsafe { r(nul, cnul, 0, cnul, cnul) };
        common::eqi(&format!("{} mlen=0 all-NULL", name), rc, rr);
        assert_eq!(rc, 0, "{} mlen=0 should return 0", name);
    }
    for name in [
        "crypto_stream_salsa20_xor_ic",
        "crypto_stream_chacha20_xor_ic",
    ] {
        let (c, r) = pair::<XorIcFn>(name);
        for ic in [0u64, 1, u64::MAX] {
            let rc = unsafe { c(nul, cnul, 0, cnul, ic, cnul) };
            let rr = unsafe { r(nul, cnul, 0, cnul, ic, cnul) };
            common::eqi(&format!("{} mlen=0 all-NULL ic={:#x}", name, ic), rc, rr);
            assert_eq!(rc, 0);
        }
    }
    for name in [
        "crypto_stream_chacha20_ietf_ext_xor_ic",
        "crypto_stream_chacha20_ietf_xor_ic",
    ] {
        let (c, r) = pair::<XorIc32Fn>(name);
        for ic in [0u32, 1, 0xffff_ffff] {
            let rc = unsafe { c(nul, cnul, 0, cnul, ic, cnul) };
            let rr = unsafe { r(nul, cnul, 0, cnul, ic, cnul) };
            common::eqi(&format!("{} mlen=0 all-NULL ic={:#x}", name, ic), rc, rr);
            assert_eq!(rc, 0);
        }
    }

    // x-variants derive a subkey *first*, so n and k must be valid; only the
    // message/output pointers may be NULL when the length is 0.
    for name in [
        "crypto_stream_xsalsa20",
        "crypto_stream_xchacha20",
        "crypto_stream",
    ] {
        let (c, r) = pair::<StreamFn>(name);
        let rc = unsafe { c(nul, 0, n24.as_ptr(), k.as_ptr()) };
        let rr = unsafe { r(nul, 0, n24.as_ptr(), k.as_ptr()) };
        common::eqi(&format!("{} clen=0 c=NULL", name), rc, rr);
        assert_eq!(rc, 0);
    }
    for name in [
        "crypto_stream_xsalsa20_xor",
        "crypto_stream_xchacha20_xor",
        "crypto_stream_xor",
    ] {
        let (c, r) = pair::<XorFn>(name);
        let rc = unsafe { c(nul, cnul, 0, n24.as_ptr(), k.as_ptr()) };
        let rr = unsafe { r(nul, cnul, 0, n24.as_ptr(), k.as_ptr()) };
        common::eqi(&format!("{} mlen=0 c/m=NULL", name), rc, rr);
        assert_eq!(rc, 0);
    }
    for name in [
        "crypto_stream_xsalsa20_xor_ic",
        "crypto_stream_xchacha20_xor_ic",
    ] {
        let (c, r) = pair::<XorIcFn>(name);
        for ic in [0u64, u64::MAX] {
            let rc = unsafe { c(nul, cnul, 0, n24.as_ptr(), ic, k.as_ptr()) };
            let rr = unsafe { r(nul, cnul, 0, n24.as_ptr(), ic, k.as_ptr()) };
            common::eqi(&format!("{} mlen=0 c/m=NULL", name), rc, rr);
            assert_eq!(rc, 0);
        }
    }
}

// =====================================================================
// *_keygen — made deterministic by installing a fixed randombytes backend
// in BOTH libraries, so the outputs can be compared byte-for-byte.
// =====================================================================

extern "C" fn det_name() -> *const c_char {
    b"stream-test-det\0".as_ptr() as *const c_char
}
extern "C" fn det_random() -> u32 {
    0x1234_5678
}
extern "C" fn det_stir() {}
extern "C" fn det_buf(buf: *mut c_void, size: usize) {
    // Stateless, so both libraries produce identical bytes.
    unsafe {
        let p = buf as *mut u8;
        for i in 0..size {
            *p.add(i) = (i as u8).wrapping_mul(37).wrapping_add(11);
        }
    }
}
extern "C" fn det_close() -> c_int {
    0
}

#[repr(C)]
struct RbImpl {
    implementation_name: extern "C" fn() -> *const c_char,
    random: extern "C" fn() -> u32,
    stir: extern "C" fn(),
    uniform: Option<extern "C" fn(u32) -> u32>,
    buf: extern "C" fn(*mut c_void, usize),
    close: extern "C" fn() -> c_int,
}

static DET_IMPL: RbImpl = RbImpl {
    implementation_name: det_name,
    random: det_random,
    stir: det_stir,
    uniform: None,
    buf: det_buf,
    close: det_close,
};

#[test]
fn keygen_all_variants() {
    type SetImpl = unsafe extern "C" fn(*const RbImpl) -> c_int;
    let (cset, rset) = pair::<SetImpl>("randombytes_set_implementation");
    let p: *const RbImpl = &DET_IMPL;
    assert_eq!(unsafe { cset(p) }, 0);
    assert_eq!(unsafe { rset(p) }, 0);

    let expected: Vec<u8> = (0..32u8)
        .map(|i| i.wrapping_mul(37).wrapping_add(11))
        .collect();

    for name in [
        "crypto_stream_keygen",
        "crypto_stream_salsa20_keygen",
        "crypto_stream_salsa2012_keygen",
        "crypto_stream_salsa208_keygen",
        "crypto_stream_xsalsa20_keygen",
        "crypto_stream_chacha20_keygen",
        "crypto_stream_chacha20_ietf_keygen",
        "crypto_stream_xchacha20_keygen",
    ] {
        let (c, r) = pair::<KeygenFn>(name);
        let mut cb = vec![CAN; 32 + PAD];
        let mut rb = vec![CAN; 32 + PAD];
        unsafe { c(cb.as_mut_ptr()) };
        unsafe { r(rb.as_mut_ptr()) };
        common::eqb(name, &cb, &rb);
        common::eqb(&format!("{} pattern", name), &cb[..32], &expected);
        assert!(
            cb[32..].iter().all(|&b| b == CAN),
            "{}: wrote more than KEYBYTES",
            name
        );
    }
}

// =====================================================================
// sodium_misuse() paths — verified out-of-process (they abort()).
// =====================================================================

#[cfg(unix)]
#[test]
fn misuse_paths_abort_in_both_libraries() {
    use std::os::unix::process::ExitStatusExt;
    use std::process::Command;

    const IETF_MAX: u64 = 64u64 * (1u64 << 32);

    if let Ok(spec) = std::env::var("STREAM_MISUSE_CASE") {
        // Child: perform one misusing call against one library; it must abort.
        let (which, case) = spec.split_once(':').unwrap();
        let l = common::libs();
        let lib = if which == "c" { &l.c } else { &l.r };
        let k = [0u8; 32];
        let n = [0u8; 12];
        let mut out = [0u8; 128];
        match case {
            // crypto_stream_chacha20_ietf: clen > ietf_MESSAGEBYTES_MAX.
            // The check runs before any memory is touched.
            "ietf" => {
                let nm = b"crypto_stream_chacha20_ietf\0".to_vec();
                let f: libloading::Symbol<StreamFn> = unsafe { lib.get(&nm) }.unwrap();
                unsafe { f(out.as_mut_ptr(), IETF_MAX + 1, n.as_ptr(), k.as_ptr()) };
            }
            // crypto_stream_chacha20_ietf_xor: mlen > ietf_MESSAGEBYTES_MAX
            "ietf_xor" => {
                let nm = b"crypto_stream_chacha20_ietf_xor\0".to_vec();
                let f: libloading::Symbol<XorFn> = unsafe { lib.get(&nm) }.unwrap();
                unsafe {
                    f(
                        out.as_mut_ptr(),
                        out.as_ptr(),
                        IETF_MAX + 1,
                        n.as_ptr(),
                        k.as_ptr(),
                    )
                };
            }
            // crypto_stream_chacha20_ietf_xor_ic:
            // ic > (64*2^32)/64 - (mlen+63)/64  ->  65 bytes needs ic <= 2^32-2
            "ietf_xor_ic" => {
                let nm = b"crypto_stream_chacha20_ietf_xor_ic\0".to_vec();
                let f: libloading::Symbol<XorIc32Fn> = unsafe { lib.get(&nm) }.unwrap();
                unsafe {
                    f(
                        out.as_mut_ptr(),
                        out.as_ptr(),
                        65,
                        n.as_ptr(),
                        0xffff_ffff,
                        k.as_ptr(),
                    )
                };
            }
            other => panic!("unknown case {}", other),
        }
        // Reaching here means the misuse check did NOT fire.
        eprintln!("NO_ABORT");
        std::process::exit(17);
    }

    let exe = std::env::current_exe().unwrap();
    for case in ["ietf", "ietf_xor", "ietf_xor_ic"] {
        let mut sigs = Vec::new();
        for which in ["c", "r"] {
            let out = Command::new(&exe)
                .args([
                    "misuse_paths_abort_in_both_libraries",
                    "--exact",
                    "--nocapture",
                    "--test-threads=1",
                ])
                .env("STREAM_MISUSE_CASE", format!("{}:{}", which, case))
                .output()
                .expect("spawn self");
            assert_ne!(
                out.status.code(),
                Some(17),
                "{}/{}: misuse check did not fire",
                which,
                case
            );
            sigs.push(out.status.signal());
        }
        assert_eq!(
            sigs[0], sigs[1],
            "case {}: C aborted with {:?} but Rust with {:?}",
            case, sigs[0], sigs[1]
        );
        assert_eq!(
            sigs[0],
            Some(6),
            "case {}: expected SIGABRT from sodium_misuse()",
            case
        );
    }
}
