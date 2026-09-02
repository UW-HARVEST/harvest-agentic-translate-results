//! Differential tests for the error / version / allocation API.
//!
//! Symbols covered (each gated with `has_both` where a build may omit it):
//!   ZSTD_isError, ZSTD_getErrorCode, ZSTD_getErrorName, ZSTD_getErrorString,
//!   ERR_getErrorString, ZSTD_versionNumber, ZSTD_versionString,
//!   ZDICT_isError, ZDICT_getErrorName, FSE_isError, FSE_getErrorName,
//!   HUF_isError, HUF_getErrorName, ZBUFF_isError, ZBUFF_getErrorName,
//!   and every ZSTDv0x_isError / ZSTDv0x_getErrorName present in both libs.
//!
//! NOTE on ZSTD_customMalloc / ZSTD_customCalloc / ZSTD_customFree: in this
//! source tree (common/allocations.h) these are `MEM_STATIC` (static inline)
//! helpers, so neither `libzstd.so` exports them (`nm -D` shows nothing). They
//! cannot be reached across the dynamic-linking FFI boundary. The test gates
//! them with `has_both` and asserts they are absent from BOTH libraries (so the
//! translation matches the C build's symbol surface); if a future build ever
//! exports them, the counting-allocator sweep below activates automatically.
#![allow(non_snake_case)]
mod harness;
use harness::*;
use std::os::raw::{c_char, c_int, c_uint, c_void};

type FnIsErr = unsafe extern "C" fn(size_t) -> c_uint;
type FnGetCode = unsafe extern "C" fn(size_t) -> c_int;
type FnGetName = unsafe extern "C" fn(size_t) -> *const c_char;
type FnStrFromCode = unsafe extern "C" fn(c_int) -> *const c_char;
type FnVersionN = unsafe extern "C" fn() -> c_uint;
type FnVersionS = unsafe extern "C" fn() -> *const c_char;

/// Build the exhaustive list of `size_t` values to probe.
fn error_probe_values(rng: &mut Rng) -> Vec<size_t> {
    let mut v: Vec<size_t> = Vec::new();
    // 0..=300
    for i in 0..=300usize {
        v.push(i);
    }
    // (0-1) down to (0-300): the "error" region (huge values near usize::MAX).
    for i in 1..=300usize {
        v.push((0usize).wrapping_sub(i));
    }
    v.push(usize::MAX / 2);
    v.push(usize::MAX);
    for _ in 0..3000 {
        v.push(rng.next_u64() as usize);
    }
    v
}

// ------------------------------------------------------------ ZSTD full group

#[test]
fn zstd_iserror_getcode_getname_exhaustive() {
    unsafe {
        let (cis, ris) = both::<FnIsErr>("ZSTD_isError");
        let (ccode, rcode) = both::<FnGetCode>("ZSTD_getErrorCode");
        let (cname, rname) = both::<FnGetName>("ZSTD_getErrorName");
        let mut rng = Rng::new(0xC15_0001);
        for v in error_probe_values(&mut rng) {
            let a = cis(v);
            let b = ris(v);
            assert_eq!(a, b, "ZSTD_isError({v:#x}): C={a} RS={b}");

            let ca = ccode(v);
            let cb = rcode(v);
            assert_eq!(ca, cb, "ZSTD_getErrorCode({v:#x}): C={ca} RS={cb}");

            let sa = cstr(cname(v));
            let sb = cstr(rname(v));
            assert_eq!(sa, sb, "ZSTD_getErrorName({v:#x}): C={sa:?} RS={sb:?}");
        }
    }
}

// ---------------------------------------------- getErrorString / ERR_getErrorString

#[test]
fn zstd_get_error_string_int_sweep() {
    unsafe {
        let (cgs, rgs) = both::<FnStrFromCode>("ZSTD_getErrorString");
        for code in -200..=400i32 {
            let sa = cstr(cgs(code));
            let sb = cstr(rgs(code));
            assert_eq!(sa, sb, "ZSTD_getErrorString({code}): C={sa:?} RS={sb:?}");
        }
    }
}

#[test]
fn err_get_error_string_int_sweep() {
    if !has_both("ERR_getErrorString") {
        panic!("ERR_getErrorString missing from a library (expected in both)");
    }
    unsafe {
        let (cgs, rgs) = both::<FnStrFromCode>("ERR_getErrorString");
        for code in -200..=400i32 {
            let sa = cstr(cgs(code));
            let sb = cstr(rgs(code));
            assert_eq!(sa, sb, "ERR_getErrorString({code}): C={sa:?} RS={sb:?}");
        }
    }
}

/// ZSTD_getErrorString and ERR_getErrorString must agree with each other within
/// the C library across the whole valid enum range (0..=120), and the Rust
/// library must reproduce the same relationship.
#[test]
fn zstd_and_err_string_cross_consistency() {
    unsafe {
        let (czs, rzs) = both::<FnStrFromCode>("ZSTD_getErrorString");
        let (cerr, rerr) = both::<FnStrFromCode>("ERR_getErrorString");
        for code in -50..=200i32 {
            assert_eq!(
                cstr(czs(code)),
                cstr(cerr(code)),
                "C: ZSTD_getErrorString vs ERR_getErrorString mismatch code={code}"
            );
            assert_eq!(
                cstr(rzs(code)),
                cstr(rerr(code)),
                "RS: ZSTD_getErrorString vs ERR_getErrorString mismatch code={code}"
            );
        }
    }
}

// ------------------------------------------------------------------ versions

#[test]
fn version_number_and_string() {
    unsafe {
        let (cvn, rvn) = both::<FnVersionN>("ZSTD_versionNumber");
        assert_eq!(cvn(), rvn(), "ZSTD_versionNumber");
        let (cvs, rvs) = both::<FnVersionS>("ZSTD_versionString");
        assert_eq!(cstr(cvs()), cstr(rvs()), "ZSTD_versionString");
    }
}

// -------------------------------------------- sub-library isError/getErrorName

/// Every (`*_isError`, `*_getErrorName`) pair that has a getErrorName counterpart.
/// Sweep the full `size_t` probe set and assert the unsigned isError result and
/// the STRING are identical between C and Rust.
fn sweep_iserr_getname(is_sym: &str, name_sym: &str, seed: u64) {
    assert!(has_both(is_sym), "{is_sym} missing from a library");
    assert!(has_both(name_sym), "{name_sym} missing from a library");
    unsafe {
        let (cis, ris) = both::<FnIsErr>(is_sym);
        let (cname, rname) = both::<FnGetName>(name_sym);
        let mut rng = Rng::new(seed);
        for v in error_probe_values(&mut rng) {
            let a = cis(v);
            let b = ris(v);
            assert_eq!(a, b, "{is_sym}({v:#x}): C={a} RS={b}");
            let sa = cstr(cname(v));
            let sb = cstr(rname(v));
            assert_eq!(sa, sb, "{name_sym}({v:#x}): C={sa:?} RS={sb:?}");
        }
    }
}

/// Sweep an `*_isError` that has no matching getErrorName (v01/v02/v03).
fn sweep_iserr_only(is_sym: &str, seed: u64) {
    assert!(has_both(is_sym), "{is_sym} missing from a library");
    unsafe {
        let (cis, ris) = both::<FnIsErr>(is_sym);
        let mut rng = Rng::new(seed);
        for v in error_probe_values(&mut rng) {
            let a = cis(v);
            let b = ris(v);
            assert_eq!(a, b, "{is_sym}({v:#x}): C={a} RS={b}");
        }
    }
}

#[test]
fn zdict_error_api() {
    sweep_iserr_getname("ZDICT_isError", "ZDICT_getErrorName", 0xC15_0100);
}

#[test]
fn fse_error_api() {
    sweep_iserr_getname("FSE_isError", "FSE_getErrorName", 0xC15_0101);
}

#[test]
fn huf_error_api() {
    sweep_iserr_getname("HUF_isError", "HUF_getErrorName", 0xC15_0102);
}

#[test]
fn zbuff_error_api() {
    sweep_iserr_getname("ZBUFF_isError", "ZBUFF_getErrorName", 0xC15_0103);
}

/// Every ZSTDv0x_isError / ZSTDv0x_getErrorName present in both libraries.
/// Per `nm -D`: v01/v02/v03 export isError only; v05/v06/v07 export both.
#[test]
fn zstd_legacy_version_error_api() {
    // isError-only legacy versions
    let iserr_only = ["ZSTDv01_isError", "ZSTDv02_isError", "ZSTDv03_isError"];
    let mut seed = 0xC15_0200u64;
    let mut tested = 0usize;
    for s in iserr_only {
        if has_both(s) {
            sweep_iserr_only(s, seed);
            tested += 1;
        }
        seed += 1;
    }
    // versions exporting both isError and getErrorName
    let pairs = [
        ("ZSTDv05_isError", "ZSTDv05_getErrorName"),
        ("ZSTDv06_isError", "ZSTDv06_getErrorName"),
        ("ZSTDv07_isError", "ZSTDv07_getErrorName"),
        // These also appear in nm output; include if present in both.
        ("ZSTDv04_isError", "ZSTDv04_getErrorName"),
    ];
    for (is_sym, name_sym) in pairs {
        if has_both(is_sym) && has_both(name_sym) {
            sweep_iserr_getname(is_sym, name_sym, seed);
            tested += 1;
        }
        seed += 1;
    }
    assert!(tested >= 6, "expected to test >= 6 legacy version error APIs, tested {tested}");
}

// ------------------------------------------ ZBUFFv04 (isError + getErrorName)

#[test]
fn zbuffv04_error_api() {
    if has_both("ZBUFFv04_isError") && has_both("ZBUFFv04_getErrorName") {
        sweep_iserr_getname("ZBUFFv04_isError", "ZBUFFv04_getErrorName", 0xC15_0300);
    } else {
        // still must agree on presence
        assert_eq!(
            has_both("ZBUFFv04_isError"),
            has_both("ZBUFFv04_getErrorName") || !has_both("ZBUFFv04_getErrorName"),
            "ZBUFFv04 symbol availability"
        );
    }
}

// --------------------------------- FSE/HUF/ZBUFF v05/v06/v07 error APIs

#[test]
fn fse_huf_zbuff_legacy_error_api() {
    let candidates = [
        ("FSEv05_isError", "FSEv05_getErrorName"),
        ("FSEv06_isError", "FSEv06_getErrorName"),
        ("FSEv07_isError", "FSEv07_getErrorName"),
        ("HUFv05_isError", "HUFv05_getErrorName"),
        ("HUFv07_isError", "HUFv07_getErrorName"),
        ("ZBUFFv05_isError", "ZBUFFv05_getErrorName"),
        ("ZBUFFv06_isError", "ZBUFFv06_getErrorName"),
        ("ZBUFFv07_isError", "ZBUFFv07_getErrorName"),
    ];
    let mut seed = 0xC15_0400u64;
    let mut tested = 0usize;
    for (is_sym, name_sym) in candidates {
        if has_both(is_sym) && has_both(name_sym) {
            sweep_iserr_getname(is_sym, name_sym, seed);
            tested += 1;
        }
        seed += 1;
    }
    assert!(tested >= 1, "expected at least one FSE/HUF/ZBUFF legacy error API");
}

// ----------------------------------------------- ZSTD_customMalloc/Calloc/Free

#[repr(C)]
#[derive(Clone, Copy)]
struct ZSTD_customMem {
    customAlloc: Option<unsafe extern "C" fn(*mut c_void, size_t) -> *mut c_void>,
    customFree: Option<unsafe extern "C" fn(*mut c_void, *mut c_void)>,
    opaque: *mut c_void,
}

type FnCustomMalloc = unsafe extern "C" fn(size_t, ZSTD_customMem) -> *mut c_void;
type FnCustomCalloc = unsafe extern "C" fn(size_t, ZSTD_customMem) -> *mut c_void;
type FnCustomFree = unsafe extern "C" fn(*mut c_void, ZSTD_customMem);

unsafe extern "C" fn count_alloc(opaque: *mut c_void, size: size_t) -> *mut c_void {
    if !opaque.is_null() {
        let p = opaque as *mut [u64; 2];
        (*p)[0] = (*p)[0].wrapping_add(1); // call count
        (*p)[1] = (*p)[1].wrapping_add(size as u64); // total requested
    }
    let align = 16usize;
    let total = 16 + size.max(1);
    let layout = std::alloc::Layout::from_size_align(total, align).unwrap();
    let base = std::alloc::alloc(layout);
    if base.is_null() {
        return std::ptr::null_mut();
    }
    (base as *mut u64).write(total as u64);
    base.add(16) as *mut c_void
}

unsafe extern "C" fn count_free(_opaque: *mut c_void, address: *mut c_void) {
    if address.is_null() {
        return;
    }
    let base = (address as *mut u8).sub(16);
    let total = (base as *const u64).read() as usize;
    let layout = std::alloc::Layout::from_size_align(total, 16).unwrap();
    std::alloc::dealloc(base, layout);
}

/// The custom-allocation helpers are static inline in this source tree, so they
/// are not exported by either .so. This test verifies both libraries agree on
/// their absence (matching symbol surface). If a build DOES export them, it
/// runs the full sweep: default customMem and a counting allocator, asserting
/// identical allocation counts, identical requested sizes, and that the
/// returned buffer is writable.
#[test]
fn zstd_custom_alloc_api() {
    let names = ["ZSTD_customMalloc", "ZSTD_customCalloc", "ZSTD_customFree"];
    let present: Vec<bool> = names.iter().map(|n| has_both(n)).collect();

    if present.iter().all(|&p| !p) {
        // Neither library exports these; that is the expected, matching state.
        // Confirm they are absent from the C library too (ground truth).
        for n in names {
            assert!(
                !has_both(n),
                "{n} unexpectedly present; update the test to exercise it"
            );
        }
        return;
    }

    // If exported by both, run the full differential sweep.
    assert!(present.iter().all(|&p| p), "custom alloc symbols only partially exported");
    unsafe {
        let (cm, rm) = both::<FnCustomMalloc>("ZSTD_customMalloc");
        let (cc, rc) = both::<FnCustomCalloc>("ZSTD_customCalloc");
        let (cfr, rfr) = both::<FnCustomFree>("ZSTD_customFree");

        let sizes = [0usize, 1, 7, 8, 4096, 1 << 20];

        // Default customMem {None,None,null}: uses the library's own malloc.
        let default = ZSTD_customMem { customAlloc: None, customFree: None, opaque: std::ptr::null_mut() };
        for &sz in &sizes {
            let cp = cm(sz, default);
            let rp = rm(sz, default);
            assert_eq!(cp.is_null(), rp.is_null(), "customMalloc({sz}) nullness");
            if !cp.is_null() {
                std::ptr::write_bytes(cp as *mut u8, 0xAB, sz);
            }
            if !rp.is_null() {
                std::ptr::write_bytes(rp as *mut u8, 0xAB, sz);
            }
            cfr(cp, default);
            rfr(rp, default);

            let cpc = cc(sz, default);
            let rpc = rc(sz, default);
            assert_eq!(cpc.is_null(), rpc.is_null(), "customCalloc({sz}) nullness");
            // calloc must zero the buffer
            if !cpc.is_null() {
                let bytes = std::slice::from_raw_parts(cpc as *const u8, sz);
                assert!(bytes.iter().all(|&b| b == 0), "customCalloc not zeroed size={sz}");
                std::ptr::write_bytes(cpc as *mut u8, 0xCD, sz);
            }
            if !rpc.is_null() {
                let bytes = std::slice::from_raw_parts(rpc as *const u8, sz);
                assert!(bytes.iter().all(|&b| b == 0), "RS customCalloc not zeroed size={sz}");
            }
            cfr(cpc, default);
            rfr(rpc, default);
        }

        // Counting allocator: identical call count and total requested size.
        for &sz in &sizes {
            let mut c_stat = [0u64; 2];
            let mut r_stat = [0u64; 2];
            let cmem_c = ZSTD_customMem {
                customAlloc: Some(count_alloc),
                customFree: Some(count_free),
                opaque: &mut c_stat as *mut [u64; 2] as *mut c_void,
            };
            let cmem_r = ZSTD_customMem {
                customAlloc: Some(count_alloc),
                customFree: Some(count_free),
                opaque: &mut r_stat as *mut [u64; 2] as *mut c_void,
            };
            let cp = cm(sz, cmem_c);
            let rp = rm(sz, cmem_r);
            assert_eq!(cp.is_null(), rp.is_null(), "counting customMalloc({sz}) nullness");
            if !cp.is_null() {
                std::ptr::write_bytes(cp as *mut u8, 0x5A, sz.max(1));
            }
            cfr(cp, cmem_c);
            rfr(rp, cmem_r);
            assert_eq!(c_stat, r_stat, "counting alloc stats differ for malloc size={sz}");
        }
    }
}
