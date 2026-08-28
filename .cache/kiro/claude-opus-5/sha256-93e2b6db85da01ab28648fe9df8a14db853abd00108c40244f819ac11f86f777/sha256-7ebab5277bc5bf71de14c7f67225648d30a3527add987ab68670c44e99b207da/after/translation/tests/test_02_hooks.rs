//! `cJSON_InitHooks` changes process-global state inside each library, so it
//! lives in its own test binary where nothing else can run concurrently.
mod common;

use common::*;
use std::os::raw::c_void;

/// `cJSON_InitHooks` with NULL must restore the defaults; with custom hooks it
/// must route allocations through them.  We verify observable behaviour only.
#[test]
fn init_hooks() {
    let _guard = serial();
    let a = apis();
    unsafe {
        // NULL resets to malloc/free
        a.c.cJSON_InitHooks(std::ptr::null_mut());
        a.rust.cJSON_InitHooks(std::ptr::null_mut());

        let cp = a.c.cJSON_CreateNumber(7.0);
        let rp = a.rust.cJSON_CreateNumber(7.0);
        assert_tree_eq("after InitHooks(NULL)", cp, rp);
        a.c.cJSON_Delete(cp);
        a.rust.cJSON_Delete(rp);

        // hooks struct with NULL members also resets to malloc/free
        let mut hooks = cJSON_Hooks {
            malloc_fn: None,
            free_fn: None,
        };
        a.c.cJSON_InitHooks(&mut hooks);
        a.rust.cJSON_InitHooks(&mut hooks);
        let cp = a.c.cJSON_CreateNumber(8.0);
        let rp = a.rust.cJSON_CreateNumber(8.0);
        assert_tree_eq("after InitHooks(empty)", cp, rp);
        a.c.cJSON_Delete(cp);
        a.rust.cJSON_Delete(rp);

        // custom hooks
        unsafe extern "C" fn my_malloc(n: usize) -> *mut c_void {
            COUNT.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            unsafe { libc_malloc(n) }
        }
        unsafe extern "C" fn my_free(p: *mut c_void) {
            unsafe { libc_free(p) }
        }
        static COUNT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
        unsafe extern "C" {
            #[link_name = "malloc"]
            fn libc_malloc(n: usize) -> *mut c_void;
            #[link_name = "free"]
            fn libc_free(p: *mut c_void);
        }

        let mut hooks = cJSON_Hooks {
            malloc_fn: Some(my_malloc),
            free_fn: Some(my_free),
        };
        COUNT.store(0, std::sync::atomic::Ordering::SeqCst);
        a.c.cJSON_InitHooks(&mut hooks);
        let cp = a.c.cJSON_CreateString(cs("abc").as_ptr());
        let c_allocs = COUNT.swap(0, std::sync::atomic::Ordering::SeqCst);
        a.c.cJSON_Delete(cp);

        a.rust.cJSON_InitHooks(&mut hooks);
        let rp = a.rust.cJSON_CreateString(cs("abc").as_ptr());
        let r_allocs = COUNT.swap(0, std::sync::atomic::Ordering::SeqCst);
        a.rust.cJSON_Delete(rp);
        assert_eq!(c_allocs, r_allocs, "custom hook allocation count");
        assert!(c_allocs > 0, "custom malloc hook was never called");

        // cJSON_malloc must use the hook too
        COUNT.store(0, std::sync::atomic::Ordering::SeqCst);
        let p = a.c.cJSON_malloc(16);
        let c_n = COUNT.swap(0, std::sync::atomic::Ordering::SeqCst);
        a.c.cJSON_free(p);
        let p = a.rust.cJSON_malloc(16);
        let r_n = COUNT.swap(0, std::sync::atomic::Ordering::SeqCst);
        a.rust.cJSON_free(p);
        assert_eq!(c_n, r_n, "cJSON_malloc hook usage");

        // restore defaults for other tests in this binary
        a.c.cJSON_InitHooks(std::ptr::null_mut());
        a.rust.cJSON_InitHooks(std::ptr::null_mut());
    }
}

/// `cJSON_InitHooks` only enables the `realloc` fast path when the supplied
/// hooks are *exactly* libc's `malloc`/`free`:
///
/// ```c
/// if ((global_hooks.allocate == malloc) && (global_hooks.deallocate == free))
///     global_hooks.reallocate = realloc;
/// ```
///
/// Whether that comparison succeeds decides whether `ensure()`/`print()` use
/// `realloc` or allocate-and-copy, which is observable through the address of
/// the returned buffer.  Both libraries must reach the same conclusion.
#[test]
fn realloc_path_selection_matches() {
    let _guard = serial();
    let a = apis();

    // Canonical libc symbol addresses (what the shared libraries see for
    // `malloc` / `free`), obtained through the dynamic linker.
    let libc = unsafe { libloading::Library::new("libc.so.6") }.expect("dlopen libc");
    let (real_malloc, real_free, plain_malloc, plain_free) = unsafe {
        let m: libloading::Symbol<unsafe extern "C" fn(usize) -> *mut c_void> =
            libc.get(b"malloc\0").unwrap();
        let f: libloading::Symbol<unsafe extern "C" fn(*mut c_void)> = libc.get(b"free\0").unwrap();
        (Some(*m), Some(*f), *m, *f)
    };

    unsafe {
        let mut hooks = cJSON_Hooks {
            malloc_fn: real_malloc,
            free_fn: real_free,
        };
        let key = cs("k");
        let mut verdicts = Vec::new();
        for api in [&a.c, &a.rust] {
            api.cJSON_InitHooks(&mut hooks);
            let item = api.cJSON_CreateObject();
            api.cJSON_AddNumberToObject(item, key.as_ptr(), 1.0);

            // Prime the allocator so that the internal 256 byte print buffer is
            // very likely to land on `probe`.
            let probe = plain_malloc(256);
            plain_free(probe);

            let printed = api.cJSON_Print(item);
            assert!(!printed.is_null());
            let same = printed as *mut c_void == probe;
            api.cJSON_free(printed as *mut c_void);
            api.cJSON_Delete(item);
            api.cJSON_InitHooks(std::ptr::null_mut());
            verdicts.push(same);
        }
        let libc_verdicts = verdicts;
        assert_eq!(
            libc_verdicts[0], libc_verdicts[1],
            "C and Rust disagree about whether the realloc path is used when the \
             hooks are libc malloc/free (C: {}, Rust: {})",
            libc_verdicts[0], libc_verdicts[1]
        );

        // Sanity check the probe: with hooks that are *not* libc malloc/free the
        // realloc path must be disabled in both libraries.
        unsafe extern "C" fn wrapper_malloc(n: usize) -> *mut c_void {
            unsafe { w_malloc(n) }
        }
        unsafe extern "C" fn wrapper_free(p: *mut c_void) {
            unsafe { w_free(p) }
        }
        unsafe extern "C" {
            #[link_name = "malloc"]
            fn w_malloc(n: usize) -> *mut c_void;
            #[link_name = "free"]
            fn w_free(p: *mut c_void);
        }
        let mut hooks = cJSON_Hooks {
            malloc_fn: Some(wrapper_malloc),
            free_fn: Some(wrapper_free),
        };
        let mut verdicts = Vec::new();
        for api in [&a.c, &a.rust] {
            api.cJSON_InitHooks(&mut hooks);
            let item = api.cJSON_CreateObject();
            api.cJSON_AddNumberToObject(item, key.as_ptr(), 1.0);
            let probe = plain_malloc(256);
            plain_free(probe);
            let printed = api.cJSON_Print(item);
            let same = printed as *mut c_void == probe;
            api.cJSON_free(printed as *mut c_void);
            api.cJSON_Delete(item);
            api.cJSON_InitHooks(std::ptr::null_mut());
            verdicts.push(same);
        }
        assert_eq!(
            verdicts[0], verdicts[1],
            "C and Rust disagree about the non-libc hook case"
        );
        // The probe only proves anything if the two hook configurations lead to
        // different observable behaviour in the C library.
        assert_ne!(
            libc_verdicts[0], verdicts[0],
            "the probe no longer distinguishes the realloc path from the \
             allocate-and-copy path; this test has become vacuous"
        );
    }
}
