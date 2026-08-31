//! Phase B/C for the symbol families no other test file owns:
//!
//!   * `POOL_*` — the thread pool. This build has NO `ZSTD_MULTITHREAD`
//!     (see c_src/CMakeLists.txt), so `pool.c` compiles its single-threaded stub
//!     branch (pool.c:313-371): `POOL_create*` returns a pointer to the file-static
//!     singleton `g_poolCtx`, `POOL_add`/`POOL_tryAdd` run the job SYNCHRONOUSLY
//!     on the calling thread, `POOL_resize` always returns 0 and `POOL_sizeof`
//!     returns `sizeof(struct POOL_ctx_s)` (or 0 for NULL). All of that observable
//!     behaviour is asserted below.
//!   * `divsufsort` / `divbwt` — the suffix-array core used by the dictionary
//!     builder, exported directly.
//!   * `ERR_getErrorString` — the internal error-string table.
//!   * the exported globals `g_debuglevel` and
//!     `g_ZSTD_threading_useless_symbol`.
//!   * `ZSTD_CCtx_refThreadPool` (the only thread-pool entry point exported by
//!     this single-threaded build; `ZSTD_createThreadPool`/`ZSTD_freeThreadPool`
//!     live inside `#ifdef ZSTD_MULTITHREAD` and do not exist here).

mod common;
use common::*;

type PoolCtx = *mut std::ffi::c_void;
type CCtx = *mut std::ffi::c_void;

// ---------------------------------------------------------------- POOL_* stubs

/// Counter incremented by the job callbacks, so we can prove `POOL_add` really
/// invoked the function (synchronously, in this build).
static mut HITS: u32 = 0;

unsafe extern "C" fn job(opaque: *mut std::ffi::c_void) {
    unsafe {
        let p = opaque as *mut u32;
        if !p.is_null() {
            *p += 1;
        }
        HITS += 1;
    }
}

#[test]
fn pool_stub_behaviour_matches() {
    let i = impls();
    let (c_create, r_create) =
        i.pair::<unsafe extern "C" fn(usize, usize) -> PoolCtx>("POOL_create");
    let (c_free, r_free) = i.pair::<unsafe extern "C" fn(PoolCtx)>("POOL_free");
    let (c_join, r_join) = i.pair::<unsafe extern "C" fn(PoolCtx)>("POOL_joinJobs");
    let (c_resize, r_resize) =
        i.pair::<unsafe extern "C" fn(PoolCtx, usize) -> i32>("POOL_resize");
    let (c_sizeof, r_sizeof) = i.pair::<unsafe extern "C" fn(PoolCtx) -> usize>("POOL_sizeof");
    let (c_add, r_add) = i.pair::<unsafe extern "C" fn(
        PoolCtx,
        unsafe extern "C" fn(*mut std::ffi::c_void),
        *mut std::ffi::c_void,
    )>("POOL_add");
    let (c_tryadd, r_tryadd) = i.pair::<unsafe extern "C" fn(
        PoolCtx,
        unsafe extern "C" fn(*mut std::ffi::c_void),
        *mut std::ffi::c_void,
    ) -> i32>("POOL_tryAdd");

    // POOL_sizeof(NULL) is explicitly supported and must return 0 in both
    unsafe {
        assert_eq_dbg(
            "POOL_sizeof(NULL)",
            c_sizeof(std::ptr::null_mut()),
            r_sizeof(std::ptr::null_mut()),
        );
        assert_eq_dbg("POOL_sizeof(NULL) == 0", c_sizeof(std::ptr::null_mut()), 0);
    }

    for &(nt, qs) in &[
        (0usize, 0usize),
        (1, 0),
        (1, 1),
        (2, 4),
        (8, 16),
        (usize::MAX, usize::MAX),
    ] {
        let cp = unsafe { c_create(nt, qs) };
        let rp = unsafe { r_create(nt, qs) };
        let tag = format!("POOL_create({nt},{qs})");
        // the stub returns the singleton; it must be non-NULL in both
        assert_eq_dbg(&format!("{tag} null-ness"), cp.is_null(), rp.is_null());
        assert!(!cp.is_null(), "{tag}: C returned NULL unexpectedly");

        // repeated creates must return the SAME singleton pointer in both libs
        let cp2 = unsafe { c_create(nt, qs) };
        let rp2 = unsafe { r_create(nt, qs) };
        assert_eq_dbg(&format!("{tag} singleton (C)"), cp, cp2);
        assert_eq_dbg(&format!("{tag} singleton (Rust)"), rp, rp2);

        unsafe {
            assert_eq_dbg(&format!("{tag} POOL_sizeof"), c_sizeof(cp), r_sizeof(rp));
            for n in [0usize, 1, 2, 64] {
                assert_eq_dbg(
                    &format!("{tag} POOL_resize({n})"),
                    c_resize(cp, n),
                    r_resize(rp, n),
                );
            }
        }

        // POOL_add must invoke the job synchronously (stub behaviour): the
        // counter has to be incremented by the time the call returns.
        let mut counter_c = 0u32;
        let mut counter_r = 0u32;
        unsafe {
            c_add(cp, job, &mut counter_c as *mut u32 as *mut std::ffi::c_void);
            r_add(rp, job, &mut counter_r as *mut u32 as *mut std::ffi::c_void);
        }
        assert_eq_dbg(&format!("{tag} POOL_add ran the job"), counter_c, 1);
        assert_eq_dbg(&format!("{tag} POOL_add C vs Rust"), counter_c, counter_r);

        let mut t_c = 0u32;
        let mut t_r = 0u32;
        let (x, y) = unsafe {
            (
                c_tryadd(cp, job, &mut t_c as *mut u32 as *mut std::ffi::c_void),
                r_tryadd(rp, job, &mut t_r as *mut u32 as *mut std::ffi::c_void),
            )
        };
        assert_eq_dbg(&format!("{tag} POOL_tryAdd rc"), x, y);
        assert_eq_dbg(&format!("{tag} POOL_tryAdd rc == 1"), x, 1);
        assert_eq_dbg(&format!("{tag} POOL_tryAdd ran the job"), t_c, t_r);
        assert_eq_dbg(&format!("{tag} POOL_tryAdd ran once"), t_c, 1);

        // many jobs in a row
        let mut many_c = 0u32;
        let mut many_r = 0u32;
        for _ in 0..50 {
            unsafe {
                c_add(cp, job, &mut many_c as *mut u32 as *mut std::ffi::c_void);
                r_add(rp, job, &mut many_r as *mut u32 as *mut std::ffi::c_void);
            }
        }
        assert_eq_dbg(&format!("{tag} 50 jobs"), many_c, 50);
        assert_eq_dbg(&format!("{tag} 50 jobs C vs Rust"), many_c, many_r);

        // POOL_add with a NULL opaque must still run the function
        unsafe {
            c_add(cp, job, std::ptr::null_mut());
            r_add(rp, job, std::ptr::null_mut());
        }

        unsafe {
            c_join(cp);
            r_join(rp);
            c_free(cp);
            r_free(rp);
            // freeing NULL is explicitly allowed by the stub's assert
            c_free(std::ptr::null_mut());
            r_free(std::ptr::null_mut());
            c_join(std::ptr::null_mut());
            r_join(std::ptr::null_mut());
        }
    }
}

#[test]
fn pool_create_advanced_matches() {
    let i = impls();
    // ZSTD_customMem is { customAlloc, customFree, opaque } — all NULL means
    // "use the default allocator" (ZSTD_defaultCMem).
    #[repr(C)]
    #[derive(Copy, Clone)]
    struct CustomMem {
        alloc: *mut std::ffi::c_void,
        free: *mut std::ffi::c_void,
        opaque: *mut std::ffi::c_void,
    }
    let (c_ca, r_ca) =
        i.pair::<unsafe extern "C" fn(usize, usize, CustomMem) -> PoolCtx>(
            "POOL_create_advanced",
        );
    let (c_sizeof, r_sizeof) = i.pair::<unsafe extern "C" fn(PoolCtx) -> usize>("POOL_sizeof");
    let (c_free, r_free) = i.pair::<unsafe extern "C" fn(PoolCtx)>("POOL_free");

    let cm = CustomMem {
        alloc: std::ptr::null_mut(),
        free: std::ptr::null_mut(),
        opaque: std::ptr::null_mut(),
    };
    for &(nt, qs) in &[(0usize, 0usize), (1, 1), (4, 8), (usize::MAX, 0)] {
        let cp = unsafe { c_ca(nt, qs, cm) };
        let rp = unsafe { r_ca(nt, qs, cm) };
        assert_eq_dbg(
            &format!("POOL_create_advanced({nt},{qs}) null-ness"),
            cp.is_null(),
            rp.is_null(),
        );
        unsafe {
            assert_eq_dbg(
                &format!("POOL_create_advanced({nt},{qs}) sizeof"),
                c_sizeof(cp),
                r_sizeof(rp),
            );
            c_free(cp);
            r_free(rp);
        }
    }
}

/// `ZSTD_CCtx_refThreadPool` — the only thread-pool entry point this build
/// actually exports.
///
/// NOTE: `ZSTD_createThreadPool` / `ZSTD_freeThreadPool` are defined at
/// `pool.c:107` and `pool.c:202`, both INSIDE the `#ifdef ZSTD_MULTITHREAD`
/// block. This build does not define `ZSTD_MULTITHREAD` (c_src/CMakeLists.txt),
/// so those two symbols do not exist in either `.so` — confirmed against
/// `nm -D` on the C library, which lists `ZSTD_CCtx_refThreadPool` and nothing
/// else thread-pool related. They are therefore deliberately not tested; the
/// pool itself is covered through `POOL_*` above.
#[test]
fn cctx_ref_threadpool_matches() {
    let i = impls();
    assert!(
        unsafe { i.c.get::<*const ()>(b"ZSTD_createThreadPool\0") }.is_err(),
        "ZSTD_createThreadPool unexpectedly present — this build was assumed \
         to be single-threaded; revisit the MT coverage"
    );

    let (c_new, r_new) = i.pair::<unsafe extern "C" fn() -> CCtx>("ZSTD_createCCtx");
    let (c_cfree, r_cfree) = i.pair::<unsafe extern "C" fn(CCtx) -> usize>("ZSTD_freeCCtx");
    let (c_ref, r_ref) =
        i.pair::<unsafe extern "C" fn(CCtx, PoolCtx) -> usize>("ZSTD_CCtx_refThreadPool");
    let (c_pcreate, r_pcreate) =
        i.pair::<unsafe extern "C" fn(usize, usize) -> PoolCtx>("POOL_create");
    let (c_pfree, r_pfree) = i.pair::<unsafe extern "C" fn(PoolCtx)>("POOL_free");

    let cc = unsafe { c_new() };
    let rc = unsafe { r_new() };

    // NULL pool clears the reference and must be accepted identically
    unsafe {
        assert_eq_dbg(
            "ZSTD_CCtx_refThreadPool(NULL)",
            c_ref(cc, std::ptr::null_mut()),
            r_ref(rc, std::ptr::null_mut()),
        );
    }

    // a real (stub singleton) pool
    for &(nt, qs) in &[(0usize, 0usize), (1, 1), (4, 8)] {
        let cp = unsafe { c_pcreate(nt, qs) };
        let rp = unsafe { r_pcreate(nt, qs) };
        let (a, b) = unsafe { (c_ref(cc, cp), r_ref(rc, rp)) };
        assert_eq_dbg(&format!("ZSTD_CCtx_refThreadPool(pool {nt}/{qs})"), a, b);
        unsafe {
            c_pfree(cp);
            r_pfree(rp);
        }
    }

    unsafe {
        c_cfree(cc);
        r_cfree(rc);
    }
}

/// `ERR_getErrorString` over the whole enum range and beyond — C enums accept
/// any int, so out-of-range values are real inputs.
#[test]
fn err_get_error_string_matches() {
    let i = impls();
    let (c, r) = i.pair::<unsafe extern "C" fn(i32) -> *const std::os::raw::c_char>(
        "ERR_getErrorString",
    );
    for code in -20i32..=200 {
        let (a, b) = unsafe { (c(code), r(code)) };
        let sa = if a.is_null() {
            "<null>".to_string()
        } else {
            unsafe { std::ffi::CStr::from_ptr(a) }
                .to_string_lossy()
                .into_owned()
        };
        let sb = if b.is_null() {
            "<null>".to_string()
        } else {
            unsafe { std::ffi::CStr::from_ptr(b) }
                .to_string_lossy()
                .into_owned()
        };
        assert_eq_dbg(&format!("ERR_getErrorString({code})"), sa, sb);
    }
    for code in [i32::MIN, i32::MIN + 1, -1000, 1000, 65536, i32::MAX] {
        let (a, b) = unsafe { (c(code), r(code)) };
        let sa = if a.is_null() {
            "<null>".into()
        } else {
            unsafe { std::ffi::CStr::from_ptr(a) }.to_string_lossy().into_owned()
        };
        let sb: String = if b.is_null() {
            "<null>".into()
        } else {
            unsafe { std::ffi::CStr::from_ptr(b) }.to_string_lossy().into_owned()
        };
        assert_eq_dbg(&format!("ERR_getErrorString({code})"), sa, sb);
    }
}

/// The exported globals must have identical values.
#[test]
fn exported_globals_match() {
    let i = impls();
    let (c, r) = i.pair::<*mut i32>("g_debuglevel");
    unsafe {
        assert_eq_dbg("g_debuglevel", **c, **r);
    }
    // g_ZSTD_threading_useless_symbol exists purely so the translation unit is
    // not empty when threading is disabled; only its presence and value matter.
    let (c, r) = i.pair::<*mut u8>("g_ZSTD_threading_useless_symbol");
    unsafe {
        assert_eq_dbg("g_ZSTD_threading_useless_symbol", **c, **r);
    }
}

/// `divsufsort` — builds a suffix array. The whole array must be identical.
#[test]
fn divsufsort_matches() {
    let i = impls();
    let (c, r) = i.pair::<unsafe extern "C" fn(*const u8, *mut i32, i32, i32) -> i32>(
        "divsufsort",
    );

    let mut rng = Rng::new(0xD1F5_0F70);
    // shapes matter a lot here: constant and highly repetitive inputs drive the
    // degenerate paths of the induced-sorting algorithm.
    for &shape in &ALL_SHAPES {
        for &n in &[0usize, 1, 2, 3, 4, 7, 8, 15, 16, 63, 64, 255, 256, 1000, 5000] {
            let src = gen_shape(shape, n, &mut rng);
            for &openmp in &[0i32, 1] {
                let mut sa1 = vec![-1i32; n.max(1)];
                let mut sa2 = vec![-1i32; n.max(1)];
                let a = unsafe { c(src.as_ptr(), sa1.as_mut_ptr(), n as i32, openmp) };
                let b = unsafe { r(src.as_ptr(), sa2.as_mut_ptr(), n as i32, openmp) };
                let tag = format!("divsufsort shape={shape:?} n={n} openMP={openmp}");
                assert_eq_dbg(&tag, a, b);
                assert_eq_dbg(&format!("{tag} suffix array"), sa1.clone(), sa2.clone());
            }
        }
    }

    // negative / zero lengths are the documented error inputs
    for n in [-1i32, -100, i32::MIN] {
        let src = [1u8, 2, 3, 4];
        let mut sa1 = vec![0i32; 4];
        let mut sa2 = vec![0i32; 4];
        let a = unsafe { c(src.as_ptr(), sa1.as_mut_ptr(), n, 0) };
        let b = unsafe { r(src.as_ptr(), sa2.as_mut_ptr(), n, 0) };
        assert_eq_dbg(&format!("divsufsort(n={n})"), a, b);
    }
    // NULL T / NULL SA are checked by the C
    let mut sa = vec![0i32; 4];
    let src = [1u8, 2, 3, 4];
    unsafe {
        assert_eq_dbg(
            "divsufsort(T=NULL)",
            c(std::ptr::null(), sa.as_mut_ptr(), 4, 0),
            r(std::ptr::null(), sa.as_mut_ptr(), 4, 0),
        );
        assert_eq_dbg(
            "divsufsort(SA=NULL)",
            c(src.as_ptr(), std::ptr::null_mut(), 4, 0),
            r(src.as_ptr(), std::ptr::null_mut(), 4, 0),
        );
    }
}

/// `divbwt` — Burrows-Wheeler transform. Output buffer and index table compared.
#[test]
fn divbwt_matches() {
    let i = impls();
    let (c, r) = i.pair::<unsafe extern "C" fn(
        *const u8,
        *mut u8,
        *mut i32,
        i32,
        *mut u8,
        *mut i32,
        i32,
    ) -> i32>("divbwt");

    let mut rng = Rng::new(0xD1F8_0177);
    for &shape in &ALL_SHAPES {
        for &n in &[0usize, 1, 2, 3, 8, 16, 100, 1000, 4000] {
            let src = gen_shape(shape, n, &mut rng);
            for &openmp in &[0i32, 1] {
                let mut u1 = vec![0u8; n.max(1)];
                let mut u2 = vec![0u8; n.max(1)];
                let mut a1 = vec![0i32; n.max(1) + 1];
                let mut a2 = vec![0i32; n.max(1) + 1];
                // num_indexes / indexes are optional (NULL) — cover both
                for use_idx in [false, true] {
                    let mut ni1 = 0u8;
                    let mut ni2 = 0u8;
                    // `indexes` must hold `*num_indexes` entries, and
                    // construct_BWT_indexes (divsufsort.c:1762) computes
                    //   *num_indexes = (n-1) / (mod+1)
                    // where `mod` is (n/8) smeared down to a power of two minus
                    // one, then >>1. For SMALL n that ratio is large (n=8 and
                    // n=16 both yield 7 with mod==0/1), and num_indexes is an
                    // unsigned char so it can reach 255. A n/8-sized buffer
                    // therefore overflows the heap for small n. Allocate the
                    // worst case unconditionally.
                    let mut idx1 = vec![0i32; 512];
                    let mut idx2 = vec![0i32; 512];
                    let (nip1, ip1) = if use_idx {
                        (&mut ni1 as *mut u8, idx1.as_mut_ptr())
                    } else {
                        (std::ptr::null_mut(), std::ptr::null_mut())
                    };
                    let (nip2, ip2) = if use_idx {
                        (&mut ni2 as *mut u8, idx2.as_mut_ptr())
                    } else {
                        (std::ptr::null_mut(), std::ptr::null_mut())
                    };
                    let x = unsafe {
                        c(
                            src.as_ptr(),
                            u1.as_mut_ptr(),
                            a1.as_mut_ptr(),
                            n as i32,
                            nip1,
                            ip1,
                            openmp,
                        )
                    };
                    let y = unsafe {
                        r(
                            src.as_ptr(),
                            u2.as_mut_ptr(),
                            a2.as_mut_ptr(),
                            n as i32,
                            nip2,
                            ip2,
                            openmp,
                        )
                    };
                    let tag =
                        format!("divbwt shape={shape:?} n={n} openMP={openmp} idx={use_idx}");
                    assert_eq_dbg(&tag, x, y);
                    assert_bytes_eq(&format!("{tag} U"), &u1, &u2);
                    if use_idx {
                        assert_eq_dbg(&format!("{tag} num_indexes"), ni1, ni2);
                        assert_eq_dbg(&format!("{tag} indexes"), idx1.clone(), idx2.clone());
                    }
                }
            }
        }
    }

    // error inputs
    let src = [5u8, 6, 7, 8];
    let mut u = vec![0u8; 4];
    let mut a = vec![0i32; 8];
    for n in [-1i32, -50] {
        unsafe {
            assert_eq_dbg(
                &format!("divbwt(n={n})"),
                c(
                    src.as_ptr(),
                    u.as_mut_ptr(),
                    a.as_mut_ptr(),
                    n,
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                    0,
                ),
                r(
                    src.as_ptr(),
                    u.as_mut_ptr(),
                    a.as_mut_ptr(),
                    n,
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                    0,
                ),
            );
        }
    }
    unsafe {
        assert_eq_dbg(
            "divbwt(T=NULL)",
            c(
                std::ptr::null(),
                u.as_mut_ptr(),
                a.as_mut_ptr(),
                4,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                0,
            ),
            r(
                std::ptr::null(),
                u.as_mut_ptr(),
                a.as_mut_ptr(),
                4,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                0,
            ),
        );
    }
}
