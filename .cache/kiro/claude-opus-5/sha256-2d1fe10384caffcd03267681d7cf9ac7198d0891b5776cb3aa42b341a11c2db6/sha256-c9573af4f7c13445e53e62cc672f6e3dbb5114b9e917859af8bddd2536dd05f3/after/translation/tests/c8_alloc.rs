//! Differential tests for the CUSTOM-ALLOCATOR, STATIC-CONTEXT and estimation
//! ERROR paths. Uses instrumented custom allocators to assert the C and Rust
//! `libzstd.so` request the SAME allocations in the SAME order and fail
//! identically under injected allocation failures.
//!
//! Every call crosses the FFI boundary via `both::<T>("name")`.
#![allow(non_snake_case)]
mod harness;
use harness::*;
use std::os::raw::{c_int, c_ulonglong, c_void};

// NOTE: ZSTD_customMalloc / ZSTD_customCalloc / ZSTD_customFree are `MEM_STATIC`
// inline in the C source (common/allocations.h) and are exported by NEITHER
// .so; `has_both()` confirms this and we never look them up.

#[repr(C)]
pub struct ZSTD_customMem {
    pub customAlloc: Option<unsafe extern "C" fn(*mut c_void, size_t) -> *mut c_void>,
    pub customFree: Option<unsafe extern "C" fn(*mut c_void, *mut c_void)>,
    pub opaque: *mut c_void,
}

const ZSTD_dlm_byCopy: c_int = 0;
const ZSTD_dct_auto: c_int = 0;

// --------------------------------------------------------- allocator machinery
//
// A single global instrumented allocator. Because the C and Rust libraries are
// exercised sequentially (never concurrently within one recorded run) we drive
// a global recorder guarded by a Mutex. `install()` resets it, then the calls
// happen, then we snapshot. Each library run uses a fresh recorder so we can
// compare the two recordings.

struct Recorder {
    /// requested sizes in call order
    sizes: Vec<size_t>,
    /// number of live (not-yet-freed) allocations
    live: usize,
    total_frees: usize,
    /// fail after this many *successful* allocations (usize::MAX = never)
    fail_after: usize,
    /// if true, every allocation returns NULL
    always_null: bool,
    successful: usize,
}

impl Recorder {
    fn new() -> Self {
        Recorder {
            sizes: Vec::new(),
            live: 0,
            total_frees: 0,
            fail_after: usize::MAX,
            always_null: false,
            successful: 0,
        }
    }
    fn with(fail_after: usize, always_null: bool) -> Box<Recorder> {
        let mut r = Recorder::new();
        r.fail_after = fail_after;
        r.always_null = always_null;
        Box::new(r)
    }
}

// Per-run recorder passed through the allocator's `opaque` pointer, so there is
// NO shared global state and tests are safe to run in parallel.

unsafe extern "C" fn counting_alloc(opaque: *mut c_void, size: size_t) -> *mut c_void {
    let r = unsafe { &mut *(opaque as *mut Recorder) };
    r.sizes.push(size);
    if r.always_null || r.successful >= r.fail_after {
        return std::ptr::null_mut();
    }
    r.successful += 1;
    r.live += 1;
    unsafe { libc_malloc(size.max(1)) }
}

unsafe extern "C" fn counting_free(opaque: *mut c_void, addr: *mut c_void) {
    if addr.is_null() {
        return;
    }
    if !opaque.is_null() {
        let r = unsafe { &mut *(opaque as *mut Recorder) };
        r.total_frees += 1;
        if r.live > 0 {
            r.live -= 1;
        }
    }
    unsafe { libc_free(addr) };
}

// Minimal libc malloc/free bindings (avoid an extra crate).
extern "C" {
    #[link_name = "malloc"]
    fn libc_malloc(size: size_t) -> *mut c_void;
    #[link_name = "free"]
    fn libc_free(ptr: *mut c_void);
}

/// Build a customMem whose opaque points at `rec`.
fn counting_mem_for(rec: &mut Recorder) -> ZSTD_customMem {
    ZSTD_customMem {
        customAlloc: Some(counting_alloc),
        customFree: Some(counting_free),
        opaque: rec as *mut Recorder as *mut c_void,
    }
}

// --------------------------------------------------------------- FFI typedefs

type FnCreateAdv = unsafe extern "C" fn(ZSTD_customMem) -> *mut c_void;
type FnCreateCDictAdv = unsafe extern "C" fn(
    *const c_void, size_t, c_int, c_int, ZSTD_compressionParameters, ZSTD_customMem,
) -> *mut c_void;
type FnCreateCDictAdv2 = unsafe extern "C" fn(
    *const c_void, size_t, c_int, c_int, *const c_void, ZSTD_customMem,
) -> *mut c_void;
type FnCreateDDictAdv = unsafe extern "C" fn(
    *const c_void, size_t, c_int, c_int, ZSTD_customMem,
) -> *mut c_void;
type FnPtrToSize = unsafe extern "C" fn(*mut c_void) -> size_t;
type FnFreeCDict = unsafe extern "C" fn(*mut c_void) -> size_t;
type FnVoidToPtr = unsafe extern "C" fn() -> *mut c_void;
type FnCompress = unsafe extern "C" fn(*mut c_void, size_t, *const c_void, size_t, c_int) -> size_t;
type FnCompressBound = unsafe extern "C" fn(size_t) -> size_t;
type FnIntToSize = unsafe extern "C" fn(c_int) -> size_t;
type FnVoidToSize = unsafe extern "C" fn() -> size_t;
type FnSizeToSize = unsafe extern "C" fn(size_t) -> size_t;
type FnConstPtrSizeToSize = unsafe extern "C" fn(*const c_void, size_t) -> size_t;
type FnEstCParams = unsafe extern "C" fn(ZSTD_compressionParameters) -> size_t;
type FnGetCParams = unsafe extern "C" fn(c_int, c_ulonglong, size_t) -> ZSTD_compressionParameters;
type FnInitStatic = unsafe extern "C" fn(*mut c_void, size_t) -> *mut c_void;
type FnInitStaticCDict = unsafe extern "C" fn(
    *mut c_void, size_t, *const c_void, size_t, c_int, c_int, ZSTD_compressionParameters,
) -> *mut c_void;
type FnInitStaticDDict = unsafe extern "C" fn(
    *mut c_void, size_t, *const c_void, size_t, c_int, c_int,
) -> *mut c_void;

// ---------------------------------------------------------- constructor list

/// Runs one `*_advanced` constructor on the given library, using the currently
/// installed global recorder. Returns whether the returned pointer was NULL,
/// and frees it (through the constructor's own free path) if non-NULL.
enum Ctor {
    CCtx,
    CStream,
    DCtx,
    DStream,
    CDict,
    CDict2,
    DDict,
}

impl Ctor {
    fn name(&self) -> &'static str {
        match self {
            Ctor::CCtx => "ZSTD_createCCtx_advanced",
            Ctor::CStream => "ZSTD_createCStream_advanced",
            Ctor::DCtx => "ZSTD_createDCtx_advanced",
            Ctor::DStream => "ZSTD_createDStream_advanced",
            Ctor::CDict => "ZSTD_createCDict_advanced",
            Ctor::CDict2 => "ZSTD_createCDict_advanced2",
            Ctor::DDict => "ZSTD_createDDict_advanced",
        }
    }
    fn all() -> Vec<Ctor> {
        vec![Ctor::CCtx, Ctor::CStream, Ctor::DCtx, Ctor::DStream, Ctor::CDict, Ctor::CDict2, Ctor::DDict]
    }
    /// Construct on library `lib_is_c` (true=C, false=Rust) with `mem`.
    /// Returns the produced pointer (may be null). Does NOT free.
    unsafe fn run(&self, lib_is_c: bool, mem: ZSTD_customMem, dict: &[u8], cparams: ZSTD_compressionParameters,
                  cctx_params: *const c_void) -> *mut c_void {
        let pick = |c: fn() -> *mut c_void| c;
        let _ = pick;
        match self {
            Ctor::CCtx | Ctor::CStream | Ctor::DCtx | Ctor::DStream => {
                let (cf, rf) = both::<FnCreateAdv>(self.name());
                if lib_is_c { cf(mem) } else { rf(mem) }
            }
            Ctor::CDict => {
                let (cf, rf) = both::<FnCreateCDictAdv>(self.name());
                let f = if lib_is_c { cf } else { rf };
                f(dict.as_ptr() as *const c_void, dict.len(), ZSTD_dlm_byCopy, ZSTD_dct_auto, cparams, mem)
            }
            Ctor::CDict2 => {
                let (cf, rf) = both::<FnCreateCDictAdv2>(self.name());
                let f = if lib_is_c { cf } else { rf };
                f(dict.as_ptr() as *const c_void, dict.len(), ZSTD_dlm_byCopy, ZSTD_dct_auto, cctx_params, mem)
            }
            Ctor::DDict => {
                let (cf, rf) = both::<FnCreateDDictAdv>(self.name());
                let f = if lib_is_c { cf } else { rf };
                f(dict.as_ptr() as *const c_void, dict.len(), ZSTD_dlm_byCopy, ZSTD_dct_auto, mem)
            }
        }
    }
    /// Free a non-null pointer produced by this constructor, on library
    /// `lib_is_c`, using the matching free function (so custom_free runs).
    unsafe fn free(&self, lib_is_c: bool, ptr: *mut c_void) {
        if ptr.is_null() { return; }
        let fname = match self {
            Ctor::CCtx => "ZSTD_freeCCtx",
            Ctor::CStream => "ZSTD_freeCStream",
            Ctor::DCtx => "ZSTD_freeDCtx",
            Ctor::DStream => "ZSTD_freeDStream",
            Ctor::CDict | Ctor::CDict2 => "ZSTD_freeCDict",
            Ctor::DDict => "ZSTD_freeDDict",
        };
        let (cf, rf) = both::<FnPtrToSize>(fname);
        if lib_is_c { cf(ptr); } else { rf(ptr); }
    }
}

fn make_cctx_params(lib_is_c: bool, level: c_int) -> *mut c_void {
    unsafe {
        let (cc, rc) = both::<FnVoidToPtr>("ZSTD_createCCtxParams");
        let p = if lib_is_c { cc() } else { rc() };
        type FnInit = unsafe extern "C" fn(*mut c_void, c_int) -> size_t;
        let (ci, ri) = both::<FnInit>("ZSTD_CCtxParams_init");
        if lib_is_c { ci(p, level); } else { ri(p, level); }
        p
    }
}
fn free_cctx_params(lib_is_c: bool, p: *mut c_void) {
    unsafe {
        let (cf, rf) = both::<FnPtrToSize>("ZSTD_freeCCtxParams");
        if lib_is_c { cf(p); } else { rf(p); }
    }
}

// ================================================================= tests

#[test]
fn custom_alloc_helpers_are_not_exported() {
    assert!(!has_both("ZSTD_customMalloc"));
    assert!(!has_both("ZSTD_customCalloc"));
    assert!(!has_both("ZSTD_customFree"));
    // ZSTD_createCCtxParams_advanced is exported by neither .so; skip it.
    assert!(!has_both("ZSTD_createCCtxParams_advanced"),
            "ZSTD_createCCtxParams_advanced unexpectedly present");
}

/// COUNTING allocator: C and Rust must make the SAME number of allocations with
/// the SAME requested sizes in the same order for each *_advanced constructor,
/// and a subsequent free must release them all. Recorder state is per-run
/// (passed via `opaque`), so the test is parallel-safe with no shared globals.
#[test]
fn counting_allocator_matches_per_constructor() {
    unsafe {
        let mut rng = Rng::new(0xc8_0001);
        let dict = gen(Shape::LongMatches, 20_000, &mut rng);
        let (cgc, _) = both::<FnGetCParams>("ZSTD_getCParams");
        let cparams = cgc(3, 0, dict.len());

        for ctor in Ctor::all() {
            // C run
            let mut rc_c = Recorder::with(usize::MAX, false);
            let cp_c = if matches!(ctor, Ctor::CDict2) { make_cctx_params(true, 3) } else { std::ptr::null_mut() };
            let pc = ctor.run(true, counting_mem_for(&mut rc_c), &dict, cparams, cp_c);
            let c_sizes = rc_c.sizes.clone();
            assert!(!pc.is_null(), "{}: C construction failed unexpectedly", ctor.name());
            ctor.free(true, pc);
            let c_live_after = rc_c.live;
            let c_frees = rc_c.total_frees;
            if !cp_c.is_null() { free_cctx_params(true, cp_c); }

            // Rust run
            let mut rc_r = Recorder::with(usize::MAX, false);
            let cp_r = if matches!(ctor, Ctor::CDict2) { make_cctx_params(false, 3) } else { std::ptr::null_mut() };
            let pr = ctor.run(false, counting_mem_for(&mut rc_r), &dict, cparams, cp_r);
            let r_sizes = rc_r.sizes.clone();
            assert!(!pr.is_null(), "{}: Rust construction failed unexpectedly", ctor.name());
            ctor.free(false, pr);
            let r_live_after = rc_r.live;
            let r_frees = rc_r.total_frees;
            if !cp_r.is_null() { free_cctx_params(false, cp_r); }

            assert_eq!(
                c_sizes, r_sizes,
                "{}: allocation sizes/count/order differ\n C={:?}\n R={:?}",
                ctor.name(), c_sizes, r_sizes
            );
            assert_eq!(c_live_after, 0, "{}: C leaked {} allocs", ctor.name(), c_live_after);
            assert_eq!(r_live_after, 0, "{}: Rust leaked {} allocs", ctor.name(), r_live_after);
            assert_eq!(c_frees, r_frees, "{}: free-count differs", ctor.name());
        }
    }
}

/// COUNTING allocator across a full compress + decompress cycle: same
/// allocation sizes/order between C and Rust.
#[test]
fn counting_allocator_compress_decompress_cycle() {
    unsafe {
        let mut rng = Rng::new(0xc8_0002);
        let src = gen(Shape::Text, 60_000, &mut rng);
        let (cb, _) = both::<FnCompressBound>("ZSTD_compressBound");
        let (ccadv, rcadv) = both::<FnCreateAdv>("ZSTD_createCCtx_advanced");
        let (cdadv, rdadv) = both::<FnCreateAdv>("ZSTD_createDCtx_advanced");
        let (cfc, rfc) = both::<FnPtrToSize>("ZSTD_freeCCtx");
        let (cfd, rfd) = both::<FnPtrToSize>("ZSTD_freeDCtx");
        type FnCompressCCtx = unsafe extern "C" fn(*mut c_void, *mut c_void, size_t, *const c_void, size_t, c_int) -> size_t;
        type FnDecompressDCtx = unsafe extern "C" fn(*mut c_void, *mut c_void, size_t, *const c_void, size_t) -> size_t;
        let (ccc, rcc) = both::<FnCompressCCtx>("ZSTD_compressCCtx");
        let (cdc, rdc) = both::<FnDecompressDCtx>("ZSTD_decompressDCtx");

        let run = |is_c: bool| -> (Vec<size_t>, usize) {
            let mut rec = Recorder::with(usize::MAX, false);
            let cctx = if is_c { ccadv(counting_mem_for(&mut rec)) } else { rcadv(counting_mem_for(&mut rec)) };
            assert!(!cctx.is_null());
            let cap = cb(src.len()) + 64;
            let mut out = vec![0u8; cap];
            let n = if is_c {
                ccc(cctx, out.as_mut_ptr() as *mut c_void, cap, src.as_ptr() as *const c_void, src.len(), 5)
            } else {
                rcc(cctx, out.as_mut_ptr() as *mut c_void, cap, src.as_ptr() as *const c_void, src.len(), 5)
            };
            assert!(!Err2::new().c.is_err(n));
            out.truncate(n);
            let dctx = if is_c { cdadv(counting_mem_for(&mut rec)) } else { rdadv(counting_mem_for(&mut rec)) };
            let mut dec = vec![0u8; src.len() + 16];
            let m = if is_c {
                cdc(dctx, dec.as_mut_ptr() as *mut c_void, dec.len(), out.as_ptr() as *const c_void, n)
            } else {
                rdc(dctx, dec.as_mut_ptr() as *mut c_void, dec.len(), out.as_ptr() as *const c_void, n)
            };
            assert_eq!(m, src.len());
            if is_c { cfc(cctx); cfd(dctx); } else { rfc(cctx); rfd(dctx); }
            (rec.sizes.clone(), rec.live)
        };
        let (c_sizes, c_live) = run(true);
        let (r_sizes, r_live) = run(false);
        assert_eq!(c_live, 0, "C leaked allocations after full cycle");
        assert_eq!(r_live, 0, "Rust leaked allocations after full cycle");
        assert_eq!(c_sizes, r_sizes,
                   "compress+decompress allocation sizes/order differ\n C={:?}\n R={:?}",
                   c_sizes, r_sizes);
    }
}

/// Allocator that always returns NULL: every *_advanced constructor must return
/// NULL identically for C and Rust.
#[test]
fn always_null_allocator_all_ctors_return_null() {
    unsafe {
        let mut rng = Rng::new(0xc8_0003);
        let dict = gen(Shape::Random, 5000, &mut rng);
        let (cgc, _) = both::<FnGetCParams>("ZSTD_getCParams");
        let cparams = cgc(3, 0, dict.len());
        for ctor in Ctor::all() {
            let mut rc_c = Recorder::with(0, true);
            let cp_c = if matches!(ctor, Ctor::CDict2) { make_cctx_params(true, 3) } else { std::ptr::null_mut() };
            let pc = ctor.run(true, counting_mem_for(&mut rc_c), &dict, cparams, cp_c);
            let cnull = pc.is_null();
            ctor.free(true, pc);
            if !cp_c.is_null() { free_cctx_params(true, cp_c); }

            let mut rc_r = Recorder::with(0, true);
            let cp_r = if matches!(ctor, Ctor::CDict2) { make_cctx_params(false, 3) } else { std::ptr::null_mut() };
            let pr = ctor.run(false, counting_mem_for(&mut rc_r), &dict, cparams, cp_r);
            let rnull = pr.is_null();
            ctor.free(false, pr);
            if !cp_r.is_null() { free_cctx_params(false, cp_r); }

            assert!(cnull, "{}: C should return NULL under always-null alloc", ctor.name());
            assert_eq!(cnull, rnull, "{}: NULL-agreement under always-null alloc", ctor.name());
        }
    }
}

/// Allocator that returns NULL after the Nth successful allocation, N=0..=12:
/// C and Rust must fail identically at each N.
#[test]
fn fail_after_n_allocations_matches() {
    unsafe {
        let mut rng = Rng::new(0xc8_0004);
        let dict = gen(Shape::LongMatches, 12000, &mut rng);
        let (cgc, _) = both::<FnGetCParams>("ZSTD_getCParams");
        let cparams = cgc(3, 0, dict.len());
        for ctor in Ctor::all() {
            for n in 0..=12usize {
                let mut rc_c = Recorder::with(n, false);
                let cp_c = if matches!(ctor, Ctor::CDict2) { make_cctx_params(true, 3) } else { std::ptr::null_mut() };
                let pc = ctor.run(true, counting_mem_for(&mut rc_c), &dict, cparams, cp_c);
                let c_sizes = rc_c.sizes.clone();
                let cnull = pc.is_null();
                ctor.free(true, pc);
                let c_live_after = rc_c.live;
                if !cp_c.is_null() { free_cctx_params(true, cp_c); }

                let mut rc_r = Recorder::with(n, false);
                let cp_r = if matches!(ctor, Ctor::CDict2) { make_cctx_params(false, 3) } else { std::ptr::null_mut() };
                let pr = ctor.run(false, counting_mem_for(&mut rc_r), &dict, cparams, cp_r);
                let r_sizes = rc_r.sizes.clone();
                let rnull = pr.is_null();
                ctor.free(false, pr);
                let r_live_after = rc_r.live;
                if !cp_r.is_null() { free_cctx_params(false, cp_r); }

                assert_eq!(cnull, rnull,
                           "{} fail_after={n}: NULL-agreement (C null={cnull} R null={rnull})",
                           ctor.name());
                assert_eq!(c_sizes, r_sizes,
                           "{} fail_after={n}: allocation sizes/order differ\n C={:?}\n R={:?}",
                           ctor.name(), c_sizes, r_sizes);
                // whatever partial allocations happened must be freed on failure
                assert_eq!(c_live_after, 0, "{} fail_after={n}: C leaked on failure", ctor.name());
                assert_eq!(r_live_after, 0, "{} fail_after={n}: Rust leaked on failure", ctor.name());
            }
        }
    }
}

/// fail-after-N across a full compress attempt: when a context is constructed
/// but a later allocation fails during compression, C and Rust must report the
/// same error code.
#[test]
fn fail_after_n_compress_error_codes_match() {
    unsafe {
        let e = Err2::new();
        let mut rng = Rng::new(0xc8_0005);
        let src = gen(Shape::Text, 40_000, &mut rng);
        let (cb, _) = both::<FnCompressBound>("ZSTD_compressBound");
        let (ccadv, rcadv) = both::<FnCreateAdv>("ZSTD_createCCtx_advanced");
        let (cfc, rfc) = both::<FnPtrToSize>("ZSTD_freeCCtx");
        type FnCompressCCtx = unsafe extern "C" fn(*mut c_void, *mut c_void, size_t, *const c_void, size_t, c_int) -> size_t;
        let (ccc, rcc) = both::<FnCompressCCtx>("ZSTD_compressCCtx");

        for n in 0..=12usize {
            let attempt = |is_c: bool| -> (bool, size_t) {
                let mut rec = Recorder::with(n, false);
                let cctx = if is_c { ccadv(counting_mem_for(&mut rec)) } else { rcadv(counting_mem_for(&mut rec)) };
                if cctx.is_null() {
                    return (true, 0); // creation itself failed
                }
                let cap = cb(src.len()) + 64;
                let mut out = vec![0u8; cap];
                let r = if is_c {
                    ccc(cctx, out.as_mut_ptr() as *mut c_void, cap, src.as_ptr() as *const c_void, src.len(), 5)
                } else {
                    rcc(cctx, out.as_mut_ptr() as *mut c_void, cap, src.as_ptr() as *const c_void, src.len(), 5)
                };
                if is_c { cfc(cctx); } else { rfc(cctx); }
                (false, r)
            };
            let (c_creationfail, c_r) = attempt(true);
            let (r_creationfail, r_r) = attempt(false);
            assert_eq!(c_creationfail, r_creationfail,
                       "fail_after={n}: creation-failure agreement");
            if !c_creationfail {
                e.eq(&format!("compress under fail_after={n}"), c_r, r_r);
            }
        }
    }
}

/// ZSTD_customMem with only one of customAlloc/customFree set (invalid combo):
/// assert identical behaviour.
#[test]
fn half_set_customMem_invalid_combo() {
    unsafe {
        let mut rng = Rng::new(0xc8_0006);
        let dict = gen(Shape::Random, 3000, &mut rng);
        let (cgc, _) = both::<FnGetCParams>("ZSTD_getCParams");
        let cparams = cgc(3, 0, dict.len());
        // alloc-only: customAlloc set (records into opaque), customFree NULL.
        // free-only: customAlloc NULL, customFree set. Since the library never
        // uses a custom alloc in the free-only case, no custom pointers are
        // produced, so customFree's opaque is irrelevant there.
        for ctor in Ctor::all() {
            for label in ["alloc-only", "free-only"] {
                let mut rc_c = Recorder::with(usize::MAX, false);
                let mem_c = if label == "alloc-only" {
                    ZSTD_customMem { customAlloc: Some(counting_alloc), customFree: None,
                                     opaque: &mut *rc_c as *mut Recorder as *mut c_void }
                } else {
                    ZSTD_customMem { customAlloc: None, customFree: Some(counting_free),
                                     opaque: std::ptr::null_mut() }
                };
                let cp_c = if matches!(ctor, Ctor::CDict2) { make_cctx_params(true, 3) } else { std::ptr::null_mut() };
                let pc = ctor.run(true, mem_c, &dict, cparams, cp_c);
                let cnull = pc.is_null();
                ctor.free(true, pc);
                if !cp_c.is_null() { free_cctx_params(true, cp_c); }

                let mut rc_r = Recorder::with(usize::MAX, false);
                let mem_r = if label == "alloc-only" {
                    ZSTD_customMem { customAlloc: Some(counting_alloc), customFree: None,
                                     opaque: &mut *rc_r as *mut Recorder as *mut c_void }
                } else {
                    ZSTD_customMem { customAlloc: None, customFree: Some(counting_free),
                                     opaque: std::ptr::null_mut() }
                };
                let cp_r = if matches!(ctor, Ctor::CDict2) { make_cctx_params(false, 3) } else { std::ptr::null_mut() };
                let pr = ctor.run(false, mem_r, &dict, cparams, cp_r);
                let rnull = pr.is_null();
                ctor.free(false, pr);
                if !cp_r.is_null() { free_cctx_params(false, cp_r); }

                assert_eq!(cnull, rnull,
                           "{} {label}: NULL-agreement (C null={cnull} R null={rnull})",
                           ctor.name());
            }
        }
    }
}

// ---------------------------------------------------------- static ctx errors

fn make_ws(size: size_t, misalign: usize) -> (Vec<u64>, *mut c_void) {
    let words = (size + misalign) / 8 + 2;
    let mut v = vec![0u64; words.max(1)];
    let base = v.as_mut_ptr() as *mut u8;
    let ptr = unsafe { base.add(misalign) } as *mut c_void;
    (v, ptr)
}

/// All six ZSTD_initStatic* with workspace==NULL, workspaceSize==0, and
/// misaligned workspace pointers: C and Rust must agree on NULL-vs-non-NULL.
#[test]
fn init_static_error_paths() {
    unsafe {
        let mut rng = Rng::new(0xc8_0007);
        let dict = gen(Shape::LongMatches, 4000, &mut rng);
        let (cgc, _) = both::<FnGetCParams>("ZSTD_getCParams");
        let cparams = cgc(3, 0, dict.len());

        // helpers with plain (ws, size) signature
        let plain = ["ZSTD_initStaticCCtx", "ZSTD_initStaticCStream",
                     "ZSTD_initStaticDCtx", "ZSTD_initStaticDStream"];
        for name in plain {
            let (cf, rf) = both::<FnInitStatic>(name);
            // NULL workspace with size 0 is the well-defined error case: both
            // return NULL. NOTE: NULL workspace with a NONZERO size is undefined
            // by the C contract (the initializer writes into the workspace) and
            // makes the C ground-truth .so segfault — verified independently
            // (ZSTD_initStaticDCtx(NULL, 1<<20) dumps core in the C .so). Since
            // the reference crashes, that input is outside the comparable domain
            // and is not exercised here.
            assert_eq!(cf(std::ptr::null_mut(), 0).is_null(), rf(std::ptr::null_mut(), 0).is_null(),
                       "{name}(NULL,0) null-agreement");
            // zero size with a real pointer
            for misalign in 0..=8usize {
                let (mut _wc, pc) = make_ws(1 << 18, misalign);
                let (mut _wr, pr) = make_ws(1 << 18, misalign);
                for &sz in &[0usize, 1, 100, 1 << 18] {
                    assert_eq!(cf(pc, sz).is_null(), rf(pr, sz).is_null(),
                               "{name}(mis={misalign}, sz={sz}) null-agreement");
                }
                let _ = (&mut _wc, &mut _wr);
            }
        }

        // CDict static
        {
            let (cf, rf) = both::<FnInitStaticCDict>("ZSTD_initStaticCDict");
            assert_eq!(
                cf(std::ptr::null_mut(), 0, dict.as_ptr() as *const c_void, dict.len(), ZSTD_dlm_byCopy, ZSTD_dct_auto, cparams).is_null(),
                rf(std::ptr::null_mut(), 0, dict.as_ptr() as *const c_void, dict.len(), ZSTD_dlm_byCopy, ZSTD_dct_auto, cparams).is_null(),
                "initStaticCDict(NULL,0) null-agreement"
            );
            for misalign in 0..=8usize {
                let (mut _wc, pc) = make_ws(1 << 18, misalign);
                let (mut _wr, pr) = make_ws(1 << 18, misalign);
                for &sz in &[0usize, 1, 100, 1 << 18] {
                    let a = cf(pc, sz, dict.as_ptr() as *const c_void, dict.len(), ZSTD_dlm_byCopy, ZSTD_dct_auto, cparams);
                    let b = rf(pr, sz, dict.as_ptr() as *const c_void, dict.len(), ZSTD_dlm_byCopy, ZSTD_dct_auto, cparams);
                    assert_eq!(a.is_null(), b.is_null(),
                               "initStaticCDict(mis={misalign}, sz={sz}) null-agreement");
                }
                let _ = (&mut _wc, &mut _wr);
            }
        }
        // DDict static
        {
            let (cf, rf) = both::<FnInitStaticDDict>("ZSTD_initStaticDDict");
            assert_eq!(
                cf(std::ptr::null_mut(), 0, dict.as_ptr() as *const c_void, dict.len(), ZSTD_dlm_byCopy, ZSTD_dct_auto).is_null(),
                rf(std::ptr::null_mut(), 0, dict.as_ptr() as *const c_void, dict.len(), ZSTD_dlm_byCopy, ZSTD_dct_auto).is_null(),
                "initStaticDDict(NULL,0) null-agreement"
            );
            for misalign in 0..=8usize {
                let (mut _wc, pc) = make_ws(1 << 16, misalign);
                let (mut _wr, pr) = make_ws(1 << 16, misalign);
                for &sz in &[0usize, 1, 100, 1 << 16] {
                    let a = cf(pc, sz, dict.as_ptr() as *const c_void, dict.len(), ZSTD_dlm_byCopy, ZSTD_dct_auto);
                    let b = rf(pr, sz, dict.as_ptr() as *const c_void, dict.len(), ZSTD_dlm_byCopy, ZSTD_dct_auto);
                    assert_eq!(a.is_null(), b.is_null(),
                               "initStaticDDict(mis={misalign}, sz={sz}) null-agreement");
                }
                let _ = (&mut _wc, &mut _wr);
            }
        }
    }
}

// ------------------------------------------------------------ estimate errors

/// ZSTD_estimate* with out-of-range compression levels and invalid cParams.
#[test]
fn estimate_error_levels_and_cparams() {
    unsafe {
        let (cc, rc) = both::<FnIntToSize>("ZSTD_estimateCCtxSize");
        let (ccs, rcs) = both::<FnIntToSize>("ZSTD_estimateCStreamSize");
        let (ecp, ercp) = both::<FnEstCParams>("ZSTD_estimateCCtxSize_usingCParams");
        let (ecss, ercss) = both::<FnEstCParams>("ZSTD_estimateCStreamSize_usingCParams");

        // NOTE: i32::MAX / i32::MAX-1 are deliberately excluded here: BOTH the C
        // ground-truth .so and the Rust .so enter an unbounded loop inside the
        // compression-level -> cParams derivation for levels very close to
        // INT_MAX (verified: estimateCCtxSize(2147483647) and (2147483646) both
        // time out in the C .so, while 1000000 returns promptly). Since the C
        // reference never returns, there is no value to compare. The other
        // out-of-range levels (INT_MIN, -1000000, 23, 100) return promptly and
        // are compared below.
        for lvl in [i32::MIN, -1_000_000, 23, 100] {
            assert_eq!(cc(lvl), rc(lvl), "estimateCCtxSize(oob {lvl})");
            assert_eq!(ccs(lvl), rcs(lvl), "estimateCStreamSize(oob {lvl})");
        }

        // invalid cParams: each field one step outside its bound, plus u32::MAX.
        // NOTE: enableLongDistanceMatching is not part of ZSTD_compressionParameters
        // so these structs don't hit the LDM divide-by-zero path.
        let base = ZSTD_compressionParameters {
            windowLog: 20, chainLog: 16, hashLog: 17, searchLog: 1,
            minMatch: 5, targetLength: 0, strategy: 1,
        };
        let bounds: [(u32, u32); 7] = [(10, 31), (6, 30), (6, 30), (1, 30), (3, 7), (0, 131072), (1, 9)];
        for (i, (lo, hi)) in bounds.iter().enumerate() {
            for &bad in &[lo.wrapping_sub(1), hi + 1, u32::MAX, 0] {
                let mut c = base;
                match i {
                    0 => c.windowLog = bad,
                    1 => c.chainLog = bad,
                    2 => c.hashLog = bad,
                    3 => c.searchLog = bad,
                    4 => c.minMatch = bad,
                    5 => c.targetLength = bad,
                    _ => c.strategy = bad,
                }
                assert_eq!(ecp(c), ercp(c), "estCCtxSize_usingCParams(bad {c:?})");
                assert_eq!(ecss(c), ercss(c), "estCStreamSize_usingCParams(bad {c:?})");
            }
        }
    }
}

/// ZSTD_estimateDStreamSize_fromFrame with tiny/garbage/random buffers.
#[test]
fn estimate_dstream_from_frame_errors() {
    unsafe {
        let e = Err2::new();
        let (cf, rf) = both::<FnConstPtrSizeToSize>("ZSTD_estimateDStreamSize_fromFrame");
        // tiny sizes over a small buffer
        let buf = [0x28u8, 0xB5, 0x2F, 0xFD, 0, 0, 0, 0];
        for sz in 0..=5usize {
            let p = buf.as_ptr() as *const c_void;
            e.eq(&format!("fromFrame tiny sz={sz}"), cf(p, sz), rf(p, sz));
        }
        // garbage / random
        let mut rng = Rng::new(0xc8_0008);
        for i in 0..2000 {
            let n = rng.below(40);
            let b: Vec<u8> = (0..n).map(|_| rng.byte()).collect();
            let p = b.as_ptr() as *const c_void;
            e.eq(&format!("fromFrame rand #{i} n={n}"), cf(p, b.len()), rf(p, b.len()));
        }
        // NULL / zero
        e.eq("fromFrame null/0", cf(std::ptr::null(), 0), rf(std::ptr::null(), 0));
    }
}

/// ZSTD_decompressionMargin with truncated and single-byte-corrupted frames.
#[test]
fn decompression_margin_errors() {
    unsafe {
        let e = Err2::new();
        let (cm, rm) = both::<FnConstPtrSizeToSize>("ZSTD_decompressionMargin");
        let (cc, _) = both::<FnCompress>("ZSTD_compress");
        let (cb, _) = both::<FnCompressBound>("ZSTD_compressBound");
        let mut rng = Rng::new(0xc8_0009);
        let src = gen(Shape::Text, 5000, &mut rng);
        let mut frame = vec![0u8; cb(src.len()) + 64];
        let n = cc(frame.as_mut_ptr() as *mut c_void, frame.len(),
                   src.as_ptr() as *const c_void, src.len(), 3);
        assert!(!e.c.is_err(n));
        frame.truncate(n);

        // sweep every truncation length
        for cut in 0..=frame.len() {
            let p = frame.as_ptr() as *const c_void;
            e.eq(&format!("margin truncate cut={cut}"), cm(p, cut), rm(p, cut));
        }
        // every single-byte mutation of the first 32 bytes
        let limit = 32.min(frame.len());
        for pos in 0..limit {
            for delta in [1u8, 0x7f, 0x80, 0xff] {
                let mut f = frame.clone();
                f[pos] = f[pos].wrapping_add(delta);
                let p = f.as_ptr() as *const c_void;
                e.eq(&format!("margin mutate pos={pos} d={delta}"), cm(p, f.len()), rm(p, f.len()));
            }
        }
    }
}

/// ZSTD_sizeof_* with a NULL pointer argument: must return identical values.
#[test]
fn sizeof_null_pointer() {
    unsafe {
        for name in ["ZSTD_sizeof_CCtx", "ZSTD_sizeof_CStream", "ZSTD_sizeof_DCtx",
                     "ZSTD_sizeof_DStream", "ZSTD_sizeof_CDict", "ZSTD_sizeof_DDict"] {
            let (cf, rf) = both::<FnPtrToSize>(name);
            assert_eq!(cf(std::ptr::null_mut()), rf(std::ptr::null_mut()), "{name}(NULL)");
        }
    }
}
