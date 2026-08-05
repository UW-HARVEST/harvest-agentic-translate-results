//! Differential tests for the STREAM CIPHERS + CORE family.
//!
//! Covers salsa20 / salsa2012 / salsa208, chacha20 (+ietf), xchacha20,
//! xsalsa20 (crypto_stream), the salsa/chacha core primitives, and the
//! exported keccak1600 core.
//!
//! Every call goes through the exported symbol on BOTH the C and Rust
//! cdylib and results are compared byte-for-byte.

#[macro_use]
mod common;
use common::{libs, Rng};

// FFI function-pointer type aliases (exact C signatures from the headers).
type StreamFn = unsafe extern "C" fn(*mut u8, u64, *const u8, *const u8) -> i32;
type XorFn = unsafe extern "C" fn(*mut u8, *const u8, u64, *const u8, *const u8) -> i32;
type XorIcFn = unsafe extern "C" fn(*mut u8, *const u8, u64, *const u8, u64, *const u8) -> i32;
type XorIc32Fn = unsafe extern "C" fn(*mut u8, *const u8, u64, *const u8, u32, *const u8) -> i32;
type CoreFn = unsafe extern "C" fn(*mut u8, *const u8, *const u8, *const u8) -> i32;
type SizeFn = unsafe extern "C" fn() -> usize;

// A representative set of lengths, including the boundary cases the C code
// special-cases (0, exactly one block, one over a block, and large multi-block).
const LENGTHS: &[usize] = &[0, 1, 7, 31, 63, 64, 65, 127, 128, 129, 191, 256, 1000, 4096];

// ---------------------------------------------------------------------------
// Constant / size accessors must agree.
// ---------------------------------------------------------------------------
#[test]
fn constants_match() {
    let l = libs();
    let names: &[&[u8]] = &[
        b"crypto_stream_salsa20_keybytes",
        b"crypto_stream_salsa20_noncebytes",
        b"crypto_stream_salsa20_messagebytes_max",
        b"crypto_stream_salsa2012_keybytes",
        b"crypto_stream_salsa2012_noncebytes",
        b"crypto_stream_salsa2012_messagebytes_max",
        b"crypto_stream_salsa208_keybytes",
        b"crypto_stream_salsa208_noncebytes",
        b"crypto_stream_salsa208_messagebytes_max",
        b"crypto_stream_chacha20_keybytes",
        b"crypto_stream_chacha20_noncebytes",
        b"crypto_stream_chacha20_messagebytes_max",
        b"crypto_stream_chacha20_ietf_keybytes",
        b"crypto_stream_chacha20_ietf_noncebytes",
        b"crypto_stream_chacha20_ietf_messagebytes_max",
        b"crypto_stream_xchacha20_keybytes",
        b"crypto_stream_xchacha20_noncebytes",
        b"crypto_stream_xchacha20_messagebytes_max",
        b"crypto_stream_xsalsa20_keybytes",
        b"crypto_stream_xsalsa20_noncebytes",
        b"crypto_stream_xsalsa20_messagebytes_max",
        b"crypto_stream_keybytes",
        b"crypto_stream_noncebytes",
        b"crypto_stream_messagebytes_max",
        b"crypto_core_salsa20_outputbytes",
        b"crypto_core_salsa20_inputbytes",
        b"crypto_core_salsa20_keybytes",
        b"crypto_core_salsa20_constbytes",
        b"crypto_core_salsa2012_outputbytes",
        b"crypto_core_salsa2012_inputbytes",
        b"crypto_core_salsa2012_keybytes",
        b"crypto_core_salsa2012_constbytes",
        b"crypto_core_salsa208_outputbytes",
        b"crypto_core_salsa208_inputbytes",
        b"crypto_core_salsa208_keybytes",
        b"crypto_core_salsa208_constbytes",
        b"crypto_core_hsalsa20_outputbytes",
        b"crypto_core_hsalsa20_inputbytes",
        b"crypto_core_hsalsa20_keybytes",
        b"crypto_core_hsalsa20_constbytes",
        b"crypto_core_hchacha20_outputbytes",
        b"crypto_core_hchacha20_inputbytes",
        b"crypto_core_hchacha20_keybytes",
        b"crypto_core_hchacha20_constbytes",
        b"crypto_core_keccak1600_statebytes",
    ];
    for name in names {
        let (c, r) = sympair!(l, *name, SizeFn);
        let cv = unsafe { c() };
        let rv = unsafe { r() };
        assert_eq!(
            cv,
            rv,
            "constant {} mismatch: C={cv} Rust={rv}",
            std::str::from_utf8(name).unwrap()
        );
    }

    // primitive() string.
    let (cp, rp) = sympair!(
        l,
        b"crypto_stream_primitive",
        unsafe extern "C" fn() -> *const std::os::raw::c_char
    );
    unsafe {
        let cs = std::ffi::CStr::from_ptr(cp());
        let rs = std::ffi::CStr::from_ptr(rp());
        assert_eq!(cs, rs, "crypto_stream_primitive");
        assert_eq!(cs.to_bytes(), b"xsalsa20");
    }
}

// ---------------------------------------------------------------------------
// Generic keystream (`_stream`) comparison for a given key/nonce size.
// ---------------------------------------------------------------------------
fn check_stream(name: &[u8], keybytes: usize, noncebytes: usize, seed: u64) {
    let l = libs();
    let (c, r) = sympair!(l, name, StreamFn);
    let mut rng = Rng::new(seed);
    for _ in 0..40 {
        for &len in LENGTHS {
            let k = rng.vec(keybytes);
            let n = rng.vec(noncebytes);
            let mut co = vec![0u8; len.max(1)];
            let mut ro = vec![0u8; len.max(1)];
            // prefill with sentinel to detect under-writes
            for b in co.iter_mut() {
                *b = 0xAA;
            }
            for b in ro.iter_mut() {
                *b = 0xAA;
            }
            let rc = unsafe { c(co.as_mut_ptr(), len as u64, n.as_ptr(), k.as_ptr()) };
            let rr = unsafe { r(ro.as_mut_ptr(), len as u64, n.as_ptr(), k.as_ptr()) };
            assert_eq!(rc, rr, "{} rc", std::str::from_utf8(name).unwrap());
            assert_eq!(
                &co[..len],
                &ro[..len],
                "{} keystream len={len}",
                std::str::from_utf8(name).unwrap()
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Generic `_xor` comparison + xor-then-xor==identity for BOTH libs.
// ---------------------------------------------------------------------------
fn check_xor(name: &[u8], keybytes: usize, noncebytes: usize, seed: u64) {
    let l = libs();
    let (c, r) = sympair!(l, name, XorFn);
    let mut rng = Rng::new(seed);
    for _ in 0..40 {
        for &len in LENGTHS {
            let k = rng.vec(keybytes);
            let n = rng.vec(noncebytes);
            let m = rng.vec(len);
            let mut co = vec![0u8; len.max(1)];
            let mut ro = vec![0u8; len.max(1)];
            let rc = unsafe { c(co.as_mut_ptr(), m.as_ptr(), len as u64, n.as_ptr(), k.as_ptr()) };
            let rr = unsafe { r(ro.as_mut_ptr(), m.as_ptr(), len as u64, n.as_ptr(), k.as_ptr()) };
            assert_eq!(rc, rr, "{} rc", std::str::from_utf8(name).unwrap());
            assert_eq!(
                &co[..len],
                &ro[..len],
                "{} xor len={len}",
                std::str::from_utf8(name).unwrap()
            );
            // xor(xor(m)) == m  (using each lib to decrypt its own ciphertext)
            let mut cback = vec![0u8; len.max(1)];
            let mut rback = vec![0u8; len.max(1)];
            unsafe {
                c(cback.as_mut_ptr(), co.as_ptr(), len as u64, n.as_ptr(), k.as_ptr());
                r(rback.as_mut_ptr(), ro.as_ptr(), len as u64, n.as_ptr(), k.as_ptr());
            }
            assert_eq!(&cback[..len], &m[..len], "C roundtrip len={len}");
            assert_eq!(&rback[..len], &m[..len], "Rust roundtrip len={len}");
        }
    }
}

// ---------------------------------------------------------------------------
// Generic `_xor_ic` (u64 counter) comparison + roundtrip.
// ---------------------------------------------------------------------------
fn check_xor_ic(name: &[u8], keybytes: usize, noncebytes: usize, seed: u64) {
    let l = libs();
    let (c, r) = sympair!(l, name, XorIcFn);
    let mut rng = Rng::new(seed);
    for _ in 0..40 {
        for &len in LENGTHS {
            let k = rng.vec(keybytes);
            let n = rng.vec(noncebytes);
            let m = rng.vec(len);
            // mix of small and large initial counters
            let ic = if rng.range(2) == 0 {
                rng.range(1000) as u64
            } else {
                rng.next_u64()
            };
            let mut co = vec![0u8; len.max(1)];
            let mut ro = vec![0u8; len.max(1)];
            let rc = unsafe {
                c(co.as_mut_ptr(), m.as_ptr(), len as u64, n.as_ptr(), ic, k.as_ptr())
            };
            let rr = unsafe {
                r(ro.as_mut_ptr(), m.as_ptr(), len as u64, n.as_ptr(), ic, k.as_ptr())
            };
            assert_eq!(rc, rr, "{} rc", std::str::from_utf8(name).unwrap());
            assert_eq!(
                &co[..len],
                &ro[..len],
                "{} xor_ic len={len} ic={ic}",
                std::str::from_utf8(name).unwrap()
            );
            // roundtrip with same ic
            let mut cback = vec![0u8; len.max(1)];
            let mut rback = vec![0u8; len.max(1)];
            unsafe {
                c(cback.as_mut_ptr(), co.as_ptr(), len as u64, n.as_ptr(), ic, k.as_ptr());
                r(rback.as_mut_ptr(), ro.as_ptr(), len as u64, n.as_ptr(), ic, k.as_ptr());
            }
            assert_eq!(&cback[..len], &m[..len], "C ic roundtrip len={len}");
            assert_eq!(&rback[..len], &m[..len], "Rust ic roundtrip len={len}");
        }
    }
}

// ==== Phase B: stream keystreams ====
#[test]
fn stream_salsa20() {
    check_stream(b"crypto_stream_salsa20", 32, 8, 1);
}
#[test]
fn stream_salsa2012() {
    check_stream(b"crypto_stream_salsa2012", 32, 8, 2);
}
#[test]
fn stream_salsa208() {
    check_stream(b"crypto_stream_salsa208", 32, 8, 3);
}
#[test]
fn stream_chacha20() {
    check_stream(b"crypto_stream_chacha20", 32, 8, 4);
}
#[test]
fn stream_chacha20_ietf() {
    check_stream(b"crypto_stream_chacha20_ietf", 32, 12, 5);
}
#[test]
fn stream_xchacha20() {
    check_stream(b"crypto_stream_xchacha20", 32, 24, 6);
}
#[test]
fn stream_xsalsa20() {
    check_stream(b"crypto_stream_xsalsa20", 32, 24, 7);
}
#[test]
fn stream_generic_xsalsa20() {
    check_stream(b"crypto_stream", 32, 24, 8);
}

// ==== Phase B: _xor ====
#[test]
fn xor_salsa20() {
    check_xor(b"crypto_stream_salsa20_xor", 32, 8, 11);
}
#[test]
fn xor_salsa2012() {
    check_xor(b"crypto_stream_salsa2012_xor", 32, 8, 12);
}
#[test]
fn xor_salsa208() {
    check_xor(b"crypto_stream_salsa208_xor", 32, 8, 13);
}
#[test]
fn xor_chacha20() {
    check_xor(b"crypto_stream_chacha20_xor", 32, 8, 14);
}
#[test]
fn xor_chacha20_ietf() {
    check_xor(b"crypto_stream_chacha20_ietf_xor", 32, 12, 15);
}
#[test]
fn xor_xchacha20() {
    check_xor(b"crypto_stream_xchacha20_xor", 32, 24, 16);
}
#[test]
fn xor_xsalsa20() {
    check_xor(b"crypto_stream_xsalsa20_xor", 32, 24, 17);
}
#[test]
fn xor_generic() {
    check_xor(b"crypto_stream_xor", 32, 24, 18);
}

// ==== Phase B: _xor_ic (u64) ====
#[test]
fn xor_ic_salsa20() {
    check_xor_ic(b"crypto_stream_salsa20_xor_ic", 32, 8, 21);
}
#[test]
fn xor_ic_chacha20() {
    check_xor_ic(b"crypto_stream_chacha20_xor_ic", 32, 8, 22);
}
#[test]
fn xor_ic_xchacha20() {
    check_xor_ic(b"crypto_stream_xchacha20_xor_ic", 32, 24, 23);
}
#[test]
fn xor_ic_xsalsa20() {
    check_xor_ic(b"crypto_stream_xsalsa20_xor_ic", 32, 24, 24);
}

// ==== Phase B: chacha20_ietf_xor_ic (u32 counter) ====
#[test]
fn xor_ic32_chacha20_ietf() {
    let l = libs();
    let (c, r) = sympair!(l, b"crypto_stream_chacha20_ietf_xor_ic", XorIc32Fn);
    let mut rng = Rng::new(31);
    for _ in 0..40 {
        for &len in LENGTHS {
            let k = rng.vec(32);
            let n = rng.vec(12);
            let m = rng.vec(len);
            // ic must stay within the valid window: ic <= 2^32 - ceil(mlen/64)
            let blocks = ((len as u64) + 63) / 64;
            let max_ic = (1u64 << 32) - blocks;
            let ic = if max_ic == 0 { 0 } else { (rng.next_u64() % max_ic) as u32 };
            let mut co = vec![0u8; len.max(1)];
            let mut ro = vec![0u8; len.max(1)];
            let rc = unsafe {
                c(co.as_mut_ptr(), m.as_ptr(), len as u64, n.as_ptr(), ic, k.as_ptr())
            };
            let rr = unsafe {
                r(ro.as_mut_ptr(), m.as_ptr(), len as u64, n.as_ptr(), ic, k.as_ptr())
            };
            assert_eq!(rc, rr, "ietf_xor_ic rc");
            assert_eq!(&co[..len], &ro[..len], "ietf_xor_ic len={len} ic={ic}");
            // roundtrip
            let mut cb = vec![0u8; len.max(1)];
            let mut rb = vec![0u8; len.max(1)];
            unsafe {
                c(cb.as_mut_ptr(), co.as_ptr(), len as u64, n.as_ptr(), ic, k.as_ptr());
                r(rb.as_mut_ptr(), ro.as_ptr(), len as u64, n.as_ptr(), ic, k.as_ptr());
            }
            assert_eq!(&cb[..len], &m[..len], "C ietf roundtrip");
            assert_eq!(&rb[..len], &m[..len], "Rust ietf roundtrip");
        }
    }
}

// ---------------------------------------------------------------------------
// Cross-checks between related APIs (property-based): confirm both libs agree
// that ietf_xor == ietf_xor_ic(ic=0), plain _xor == _xor_ic(ic=0), and that
// keystream == xor against a zero message.
// ---------------------------------------------------------------------------
#[test]
fn relations_hold_both_libs() {
    let l = libs();
    let mut rng = Rng::new(99);
    let (cx, rx) = sympair!(l, b"crypto_stream_salsa20_xor", XorFn);
    let (cxi, rxi) = sympair!(l, b"crypto_stream_salsa20_xor_ic", XorIcFn);
    let (cs, rs) = sympair!(l, b"crypto_stream_salsa20", StreamFn);
    for &len in LENGTHS {
        let k = rng.vec(32);
        let n = rng.vec(8);
        let m = rng.vec(len);
        let zero = vec![0u8; len.max(1)];
        let mut a = vec![0u8; len.max(1)];
        let mut b = vec![0u8; len.max(1)];
        // _xor == _xor_ic(0) for both libs
        unsafe {
            cx(a.as_mut_ptr(), m.as_ptr(), len as u64, n.as_ptr(), k.as_ptr());
            cxi(b.as_mut_ptr(), m.as_ptr(), len as u64, n.as_ptr(), 0, k.as_ptr());
        }
        assert_eq!(&a[..len], &b[..len], "C xor==xor_ic0");
        unsafe {
            rx(a.as_mut_ptr(), m.as_ptr(), len as u64, n.as_ptr(), k.as_ptr());
            rxi(b.as_mut_ptr(), m.as_ptr(), len as u64, n.as_ptr(), 0, k.as_ptr());
        }
        assert_eq!(&a[..len], &b[..len], "Rust xor==xor_ic0");
        // keystream == xor(zeros)
        unsafe {
            cs(a.as_mut_ptr(), len as u64, n.as_ptr(), k.as_ptr());
            cx(b.as_mut_ptr(), zero.as_ptr(), len as u64, n.as_ptr(), k.as_ptr());
        }
        assert_eq!(&a[..len], &b[..len], "C stream==xor(0)");
        unsafe {
            rs(a.as_mut_ptr(), len as u64, n.as_ptr(), k.as_ptr());
            rx(b.as_mut_ptr(), zero.as_ptr(), len as u64, n.as_ptr(), k.as_ptr());
        }
        assert_eq!(&a[..len], &b[..len], "Rust stream==xor(0)");
    }
}

// ---------------------------------------------------------------------------
// Core primitives. `c` (const) is exercised both NULL and non-NULL.
// ---------------------------------------------------------------------------
fn check_core(name: &[u8], outbytes: usize, seed: u64) {
    let l = libs();
    let (c, r) = sympair!(l, name, CoreFn);
    let mut rng = Rng::new(seed);
    for _ in 0..200 {
        let inp = rng.vec(16);
        let k = rng.vec(32);
        let konst = rng.vec(16);
        for use_c in [false, true] {
            let cptr = if use_c { konst.as_ptr() } else { std::ptr::null() };
            let mut co = vec![0u8; outbytes];
            let mut ro = vec![0u8; outbytes];
            let rc = unsafe { c(co.as_mut_ptr(), inp.as_ptr(), k.as_ptr(), cptr) };
            let rr = unsafe { r(ro.as_mut_ptr(), inp.as_ptr(), k.as_ptr(), cptr) };
            assert_eq!(rc, rr, "{} rc", std::str::from_utf8(name).unwrap());
            assert_eq!(
                co,
                ro,
                "{} out (c={use_c})",
                std::str::from_utf8(name).unwrap()
            );
        }
    }
}

#[test]
fn core_salsa20() {
    check_core(b"crypto_core_salsa20", 64, 41);
}
#[test]
fn core_salsa2012() {
    check_core(b"crypto_core_salsa2012", 64, 42);
}
#[test]
fn core_salsa208() {
    check_core(b"crypto_core_salsa208", 64, 43);
}
#[test]
fn core_hsalsa20() {
    check_core(b"crypto_core_hsalsa20", 32, 44);
}
#[test]
fn core_hchacha20() {
    check_core(b"crypto_core_hchacha20", 32, 45);
}

// ---------------------------------------------------------------------------
// Keccak1600 core: drive init -> xor_bytes -> permute -> extract and compare
// the whole 200-byte state for both permute_24 and permute_12.
// ---------------------------------------------------------------------------
#[test]
fn keccak1600_core() {
    use std::os::raw::c_void;
    let l = libs();
    type InitFn = unsafe extern "C" fn(*mut c_void);
    type XorBytesFn = unsafe extern "C" fn(*mut c_void, *const u8, usize, usize);
    type ExtractFn = unsafe extern "C" fn(*const c_void, *mut u8, usize, usize);
    type PermFn = unsafe extern "C" fn(*mut c_void);

    let (c_init, r_init) = sympair!(l, b"crypto_core_keccak1600_init", InitFn);
    let (c_xor, r_xor) = sympair!(l, b"crypto_core_keccak1600_xor_bytes", XorBytesFn);
    let (c_ext, r_ext) = sympair!(l, b"crypto_core_keccak1600_extract_bytes", ExtractFn);
    let (c_sb, r_sb) = sympair!(l, b"crypto_core_keccak1600_statebytes", SizeFn);
    let sb = unsafe { c_sb() };
    assert_eq!(sb, unsafe { r_sb() });
    assert_eq!(sb, 224);

    let mut rng = Rng::new(50);
    for perm in [
        &b"crypto_core_keccak1600_permute_24"[..],
        &b"crypto_core_keccak1600_permute_12"[..],
    ] {
        let (c_perm, r_perm) = sympair!(l, perm, PermFn);
        for _ in 0..50 {
            // 224-byte opaque state, aligned enough via Vec allocation.
            let mut cst = vec![0u8; sb];
            let mut rst = vec![0u8; sb];
            unsafe {
                c_init(cst.as_mut_ptr() as *mut c_void);
                r_init(rst.as_mut_ptr() as *mut c_void);
            }
            // absorb some random data at a random offset (within the 200-byte rate area)
            let dlen = rng.range(200);
            let data = rng.vec(dlen);
            let off = rng.range(200usize.saturating_sub(data.len()) + 1);
            unsafe {
                c_xor(cst.as_mut_ptr() as *mut c_void, data.as_ptr(), off, data.len());
                r_xor(rst.as_mut_ptr() as *mut c_void, data.as_ptr(), off, data.len());
            }
            assert_eq!(cst, rst, "state after xor_bytes");
            unsafe {
                c_perm(cst.as_mut_ptr() as *mut c_void);
                r_perm(rst.as_mut_ptr() as *mut c_void);
            }
            assert_eq!(
                cst,
                rst,
                "state after {}",
                std::str::from_utf8(perm).unwrap()
            );
            // extract from a random offset/length
            let elen = rng.range(200);
            let eoff = rng.range(200usize.saturating_sub(elen) + 1);
            let mut cout = vec![0u8; elen.max(1)];
            let mut rout = vec![0u8; elen.max(1)];
            unsafe {
                c_ext(cst.as_ptr() as *const c_void, cout.as_mut_ptr(), eoff, elen);
                r_ext(rst.as_ptr() as *const c_void, rout.as_mut_ptr(), eoff, elen);
            }
            assert_eq!(&cout[..elen], &rout[..elen], "extract_bytes");
        }
    }
}

// ---------------------------------------------------------------------------
// Phase C: error path. crypto_stream_chacha20_ietf_xor_ic aborts (via
// sodium_misuse) when ic is out of the 2^32-block window. We fork so the
// abort doesn't take down the test runner, and assert BOTH libs abort the
// same way (SIGABRT). Values that are just inside the window must NOT abort.
// ---------------------------------------------------------------------------
extern "C" {
    fn fork() -> i32;
    fn waitpid(pid: i32, status: *mut i32, options: i32) -> i32;
    fn _exit(code: i32) -> !;
}

/// Run `f` in a forked child. Returns Some(signal) if the child was killed by
/// a signal, or None if it exited normally.
fn run_forked<F: FnOnce()>(f: F) -> Option<i32> {
    unsafe {
        let pid = fork();
        assert!(pid >= 0, "fork failed");
        if pid == 0 {
            // child
            f();
            _exit(0); // reached only if f() did NOT abort
        }
        let mut status: i32 = 0;
        let w = waitpid(pid, &mut status as *mut i32, 0);
        assert_eq!(w, pid, "waitpid");
        // WIFSIGNALED: low 7 bits hold the terminating signal (nonzero, != 0x7f)
        let termsig = status & 0x7f;
        if termsig != 0 && termsig != 0x7f {
            Some(termsig)
        } else {
            None
        }
    }
}

const SIGABRT: i32 = 6;

#[test]
fn err_ietf_xor_ic_overflow_aborts_both() {
    let l = libs();
    let (c, r) = sympair!(l, b"crypto_stream_chacha20_ietf_xor_ic", XorIc32Fn);
    // A raw fn-pointer copy so we can move it into the closure without holding
    // the Symbol borrow across fork.
    let cf: XorIc32Fn = *c;
    let rf: XorIc32Fn = *r;

    // mlen = 128 -> 2 blocks; window max ic = 2^32 - 2. ic = 2^32 - 1 overflows.
    let mlen: u64 = 128;
    let bad_ic: u32 = 0xffff_ffff; // definitely out of range for mlen>0

    let key = [7u8; 32];
    let nonce = [3u8; 12];
    let msg = [0u8; 128];

    let c_sig = run_forked(|| unsafe {
        let mut out = [0u8; 128];
        cf(out.as_mut_ptr(), msg.as_ptr(), mlen, nonce.as_ptr(), bad_ic, key.as_ptr());
    });
    let r_sig = run_forked(|| unsafe {
        let mut out = [0u8; 128];
        rf(out.as_mut_ptr(), msg.as_ptr(), mlen, nonce.as_ptr(), bad_ic, key.as_ptr());
    });
    assert_eq!(c_sig, Some(SIGABRT), "C should SIGABRT on ietf ic overflow");
    assert_eq!(r_sig, Some(SIGABRT), "Rust should SIGABRT on ietf ic overflow");

    // Boundary: ic == 2^32 - blocks is the largest valid value -> must NOT abort.
    let ok_ic: u32 = (0xffff_ffffu64 - 1) as u32; // 2^32 - 2, blocks = 2
    let c_ok = run_forked(|| unsafe {
        let mut out = [0u8; 128];
        cf(out.as_mut_ptr(), msg.as_ptr(), mlen, nonce.as_ptr(), ok_ic, key.as_ptr());
    });
    let r_ok = run_forked(|| unsafe {
        let mut out = [0u8; 128];
        rf(out.as_mut_ptr(), msg.as_ptr(), mlen, nonce.as_ptr(), ok_ic, key.as_ptr());
    });
    assert_eq!(c_ok, None, "C boundary ic must not abort");
    assert_eq!(r_ok, None, "Rust boundary ic must not abort");
}
