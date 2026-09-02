//! Differential tests for the `POOL_*` thread-pool API.
//!
//! The C library is built with `ZSTD_MULTITHREAD` NOT defined, so `pool.c`
//! compiles the single-threaded fallback:
//!   * `POOL_create*` ignores its arguments and returns a pointer to a static
//!     singleton (never NULL, even for numThreads==0), and never consults the
//!     custom allocator.
//!   * `POOL_free`/`POOL_joinJobs` are no-ops (NULL-safe).
//!   * `POOL_resize` always returns 0 (success), including resize-to-0.
//!   * `POOL_add`/`POOL_tryAdd` run the job synchronously and immediately;
//!     `POOL_tryAdd` returns 1.
//!   * `POOL_sizeof(NULL)==0`, otherwise `sizeof(struct POOL_ctx_s)`.
//!
//! Because the C fallback returns a *static* pointer, we never compare the C
//! and Rust pointer *values* — only whether both are NULL vs non-NULL, and the
//! observable behaviour (sizeof, resize codes, job invocation counts).
//!
//! Covers: POOL_create, POOL_create_advanced, POOL_free, POOL_joinJobs,
//!         POOL_resize, POOL_sizeof, POOL_add, POOL_tryAdd.
#![allow(non_snake_case)]
mod harness;
use harness::*;
use std::os::raw::{c_int, c_void};

// ------------------------------------------------------------------- ffi types

#[repr(C)]
#[derive(Clone, Copy)]
struct ZSTD_customMem {
    customAlloc: Option<unsafe extern "C" fn(*mut c_void, size_t) -> *mut c_void>,
    customFree: Option<unsafe extern "C" fn(*mut c_void, *mut c_void)>,
    opaque: *mut c_void,
}

type PoolFunction = unsafe extern "C" fn(*mut c_void);

type FnCreate = unsafe extern "C" fn(size_t, size_t) -> *mut c_void;
type FnCreateAdv = unsafe extern "C" fn(size_t, size_t, ZSTD_customMem) -> *mut c_void;
type FnFree = unsafe extern "C" fn(*mut c_void);
type FnJoinJobs = unsafe extern "C" fn(*mut c_void);
type FnResize = unsafe extern "C" fn(*mut c_void, size_t) -> c_int;
type FnSizeof = unsafe extern "C" fn(*const c_void) -> size_t;
type FnAdd = unsafe extern "C" fn(*mut c_void, PoolFunction, *mut c_void);
type FnTryAdd = unsafe extern "C" fn(*mut c_void, PoolFunction, *mut c_void) -> c_int;

// ------------------------------------------------------------------ job & alloc

/// A job that increments the u64 counter pointed to by `opaque`.
unsafe extern "C" fn incr_job(opaque: *mut c_void) {
    if !opaque.is_null() {
        let p = opaque as *mut u64;
        *p = (*p).wrapping_add(1);
    }
}

/// Counting allocator: bumps *(opaque as *mut u64) per allocation and forwards
/// to the system allocator. Uses a header to remember the layout for free.
#[repr(C)]
struct AllocHeader {
    size: usize,
    align: usize,
}
const HDR: usize = std::mem::size_of::<AllocHeader>();

unsafe extern "C" fn counting_alloc(opaque: *mut c_void, size: size_t) -> *mut c_void {
    if !opaque.is_null() {
        let p = opaque as *mut u64;
        *p = (*p).wrapping_add(1);
    }
    let align = 16usize;
    let total = HDR + size;
    let layout = std::alloc::Layout::from_size_align(total, align).unwrap();
    let base = std::alloc::alloc(layout);
    if base.is_null() {
        return std::ptr::null_mut();
    }
    (base as *mut AllocHeader).write(AllocHeader { size, align });
    base.add(HDR) as *mut c_void
}

unsafe extern "C" fn counting_free(_opaque: *mut c_void, address: *mut c_void) {
    if address.is_null() {
        return;
    }
    let base = (address as *mut u8).sub(HDR);
    let hdr = (base as *const AllocHeader).read();
    let total = HDR + hdr.size;
    let layout = std::alloc::Layout::from_size_align(total, hdr.align).unwrap();
    std::alloc::dealloc(base, layout);
}

/// An allocator that always returns NULL (simulates OOM).
unsafe extern "C" fn null_alloc(opaque: *mut c_void, _size: size_t) -> *mut c_void {
    if !opaque.is_null() {
        let p = opaque as *mut u64;
        *p = (*p).wrapping_add(1);
    }
    std::ptr::null_mut()
}

unsafe extern "C" fn null_free(_opaque: *mut c_void, _address: *mut c_void) {}

const DIMS: &[usize] = &[0, 1, 2, 4, 8];

// ---------------------------------------------------------------- create/sizeof

/// Sweep numThreads x queueSize; assert NULL-vs-non-NULL agreement and
/// identical POOL_sizeof. Frees through each library.
#[test]
fn pool_create_nullness_and_sizeof() {
    unsafe {
        let (cc, rc) = both::<FnCreate>("POOL_create");
        let (cf, rf) = both::<FnFree>("POOL_free");
        let (csz, rsz) = both::<FnSizeof>("POOL_sizeof");
        for &nt in DIMS {
            for &qs in DIMS {
                let cp = cc(nt, qs);
                let rp = rc(nt, qs);
                assert_eq!(
                    cp.is_null(),
                    rp.is_null(),
                    "POOL_create({nt},{qs}) nullness C={} RS={}",
                    cp.is_null(),
                    rp.is_null()
                );
                // sizeof must agree (both accept the returned pointer, incl. NULL)
                let cs = csz(cp as *const c_void);
                let rs = rsz(rp as *const c_void);
                assert_eq!(cs, rs, "POOL_sizeof after create({nt},{qs}): C={cs} RS={rs}");
                cf(cp);
                rf(rp);
            }
        }
        // POOL_sizeof(NULL) == 0 on both.
        assert_eq!(csz(std::ptr::null()), 0, "C POOL_sizeof(NULL)");
        assert_eq!(rsz(std::ptr::null()), 0, "RS POOL_sizeof(NULL)");
        assert_eq!(csz(std::ptr::null()), rsz(std::ptr::null()));
    }
}

// ---------------------------------------------------------------------- resize

/// POOL_resize over a sweep of thread counts including resize-to-0, plus
/// POOL_resize(NULL). Return codes must be identical.
#[test]
fn pool_resize_return_codes() {
    unsafe {
        let (cc, rc) = both::<FnCreate>("POOL_create");
        let (cf, rf) = both::<FnFree>("POOL_free");
        let (crz, rrz) = both::<FnResize>("POOL_resize");
        for &nt in DIMS {
            for &qs in &[0usize, 1, 4] {
                let cp = cc(nt.max(1), qs);
                let rp = rc(nt.max(1), qs);
                for &target in &[0usize, 1, 2, 4, 8, 16] {
                    let a = crz(cp, target);
                    let b = rrz(rp, target);
                    assert_eq!(a, b, "POOL_resize(create({nt},{qs}) -> {target}): C={a} RS={b}");
                }
                cf(cp);
                rf(rp);
            }
        }
        // POOL_resize(NULL, n)
        for &target in &[0usize, 1, 4] {
            assert_eq!(
                crz(std::ptr::null_mut(), target),
                rrz(std::ptr::null_mut(), target),
                "POOL_resize(NULL,{target})"
            );
        }
    }
}

// --------------------------------------------------------------- free/joinJobs

/// POOL_free(NULL) and POOL_joinJobs on a fresh pool must both be safe no-ops.
#[test]
fn pool_free_null_and_joinjobs_fresh() {
    unsafe {
        let (cc, rc) = both::<FnCreate>("POOL_create");
        let (cf, rf) = both::<FnFree>("POOL_free");
        let (cj, rj) = both::<FnJoinJobs>("POOL_joinJobs");
        // POOL_free(NULL) on both libraries: no crash.
        cf(std::ptr::null_mut());
        rf(std::ptr::null_mut());
        // joinJobs on a fresh pool: no queued jobs, must return promptly.
        for &nt in &[1usize, 2, 4] {
            let cp = cc(nt, 4);
            let rp = rc(nt, 4);
            cj(cp);
            rj(rp);
            cf(cp);
            rf(rp);
        }
    }
}

// ------------------------------------------------------------- add / tryAdd

/// POOL_add and POOL_tryAdd with a counter-incrementing job. Both libraries
/// must invoke the job the same number of times and (single-threaded fallback)
/// leave the counter at the number of adds after joinJobs.
#[test]
fn pool_add_and_tryadd_invocation_counts() {
    unsafe {
        let (cc, rc) = both::<FnCreate>("POOL_create");
        let (cf, rf) = both::<FnFree>("POOL_free");
        let (cj, rj) = both::<FnJoinJobs>("POOL_joinJobs");
        let (cadd, radd) = both::<FnAdd>("POOL_add");
        let (ctry, rtry) = both::<FnTryAdd>("POOL_tryAdd");

        for &nt in &[1usize, 2, 4, 8] {
            for &qs in &[0usize, 1, 2, 4] {
                let cp = cc(nt, qs);
                let rp = rc(nt, qs);

                // POOL_add: run N jobs against each pool with its own counter.
                let mut c_ctr: u64 = 0;
                let mut r_ctr: u64 = 0;
                let n_add = 5usize;
                for _ in 0..n_add {
                    cadd(cp, incr_job, &mut c_ctr as *mut u64 as *mut c_void);
                    radd(rp, incr_job, &mut r_ctr as *mut u64 as *mut c_void);
                }
                cj(cp);
                rj(rp);
                assert_eq!(
                    c_ctr, r_ctr,
                    "POOL_add invocation counts differ create({nt},{qs}): C={c_ctr} RS={r_ctr}"
                );

                // POOL_tryAdd: return code AND invocation count must match.
                let mut c_ctr2: u64 = 0;
                let mut r_ctr2: u64 = 0;
                for i in 0..7usize {
                    let a = ctry(cp, incr_job, &mut c_ctr2 as *mut u64 as *mut c_void);
                    let b = rtry(rp, incr_job, &mut r_ctr2 as *mut u64 as *mut c_void);
                    assert_eq!(a, b, "POOL_tryAdd rc differ create({nt},{qs}) i={i}: C={a} RS={b}");
                }
                cj(cp);
                rj(rp);
                assert_eq!(
                    c_ctr2, r_ctr2,
                    "POOL_tryAdd invocation counts differ create({nt},{qs}): C={c_ctr2} RS={r_ctr2}"
                );
                cf(cp);
                rf(rp);
            }
        }
    }
}

// --------------------------------------------------- create_advanced + alloc

/// POOL_create_advanced with a counting custom allocator. Both libraries must
/// agree on nullness of the result, on POOL_sizeof, and — whether or not the
/// allocator is consulted (the single-threaded fallback ignores it) — the two
/// libraries must consult it the *same* number of times.
#[test]
fn pool_create_advanced_counting_allocator() {
    unsafe {
        let (cca, rca) = both::<FnCreateAdv>("POOL_create_advanced");
        let (cf, rf) = both::<FnFree>("POOL_free");
        let (csz, rsz) = both::<FnSizeof>("POOL_sizeof");
        for &nt in DIMS {
            for &qs in DIMS {
                let mut c_allocs: u64 = 0;
                let mut r_allocs: u64 = 0;
                let cmem_c = ZSTD_customMem {
                    customAlloc: Some(counting_alloc),
                    customFree: Some(counting_free),
                    opaque: &mut c_allocs as *mut u64 as *mut c_void,
                };
                let cmem_r = ZSTD_customMem {
                    customAlloc: Some(counting_alloc),
                    customFree: Some(counting_free),
                    opaque: &mut r_allocs as *mut u64 as *mut c_void,
                };
                let cp = cca(nt, qs, cmem_c);
                let rp = rca(nt, qs, cmem_r);
                assert_eq!(
                    cp.is_null(),
                    rp.is_null(),
                    "POOL_create_advanced({nt},{qs}) nullness"
                );
                assert_eq!(
                    csz(cp as *const c_void),
                    rsz(rp as *const c_void),
                    "POOL_create_advanced sizeof({nt},{qs})"
                );
                cf(cp);
                rf(rp);
                assert_eq!(
                    c_allocs, r_allocs,
                    "custom allocator call counts differ ({nt},{qs}): C={c_allocs} RS={r_allocs}"
                );
            }
        }
    }
}

/// POOL_create_advanced with an allocator that always returns NULL. The two
/// libraries must agree on the resulting pointer's nullness. (In the
/// single-threaded fallback the allocator is never called, so both return a
/// valid singleton; a multi-threaded build would return NULL. Either way, C and
/// Rust must agree.)
#[test]
fn pool_create_advanced_null_allocator() {
    unsafe {
        let (cca, rca) = both::<FnCreateAdv>("POOL_create_advanced");
        let (cf, rf) = both::<FnFree>("POOL_free");
        let (csz, rsz) = both::<FnSizeof>("POOL_sizeof");
        for &nt in DIMS {
            for &qs in DIMS {
                let mut c_allocs: u64 = 0;
                let mut r_allocs: u64 = 0;
                let cmem_c = ZSTD_customMem {
                    customAlloc: Some(null_alloc),
                    customFree: Some(null_free),
                    opaque: &mut c_allocs as *mut u64 as *mut c_void,
                };
                let cmem_r = ZSTD_customMem {
                    customAlloc: Some(null_alloc),
                    customFree: Some(null_free),
                    opaque: &mut r_allocs as *mut u64 as *mut c_void,
                };
                let cp = cca(nt, qs, cmem_c);
                let rp = rca(nt, qs, cmem_r);
                assert_eq!(
                    cp.is_null(),
                    rp.is_null(),
                    "POOL_create_advanced null-alloc ({nt},{qs}) nullness C={} RS={}",
                    cp.is_null(),
                    rp.is_null()
                );
                assert_eq!(
                    csz(cp as *const c_void),
                    rsz(rp as *const c_void),
                    "POOL_create_advanced null-alloc sizeof({nt},{qs})"
                );
                assert_eq!(
                    c_allocs, r_allocs,
                    "null allocator call counts differ ({nt},{qs}): C={c_allocs} RS={r_allocs}"
                );
                cf(cp);
                rf(rp);
            }
        }
    }
}

/// create_advanced with the default `{None,None,null}` customMem, then run jobs.
#[test]
fn pool_create_advanced_default_cmem_jobs() {
    unsafe {
        let (cca, rca) = both::<FnCreateAdv>("POOL_create_advanced");
        let (cf, rf) = both::<FnFree>("POOL_free");
        let (cj, rj) = both::<FnJoinJobs>("POOL_joinJobs");
        let (cadd, radd) = both::<FnAdd>("POOL_add");
        let default = ZSTD_customMem { customAlloc: None, customFree: None, opaque: std::ptr::null_mut() };
        for &nt in &[1usize, 2, 4] {
            let cp = cca(nt, 2, default);
            let rp = rca(nt, 2, default);
            assert_eq!(cp.is_null(), rp.is_null(), "create_advanced default cmem nullness nt={nt}");
            let mut c_ctr: u64 = 0;
            let mut r_ctr: u64 = 0;
            for _ in 0..4 {
                cadd(cp, incr_job, &mut c_ctr as *mut u64 as *mut c_void);
                radd(rp, incr_job, &mut r_ctr as *mut u64 as *mut c_void);
            }
            cj(cp);
            rj(rp);
            assert_eq!(c_ctr, r_ctr, "default-cmem job counts differ nt={nt}");
            cf(cp);
            rf(rp);
        }
    }
}
