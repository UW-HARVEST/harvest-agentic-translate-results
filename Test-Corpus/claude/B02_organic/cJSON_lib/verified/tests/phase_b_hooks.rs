//! Phase B — rows C3..C6 and C89: the allocator-hook configuration axis.
//!
//! `global_hooks` is process-global state inside each library, so every test in
//! this file takes a process-wide lock and always restores the default hooks
//! before releasing it. The file is its own test binary, so it cannot interfere
//! with the other Phase B/C files either.
#![allow(non_snake_case)]

mod common;
use common::*;

use std::ffi::c_void;
use std::fmt::Write as _;
use std::ptr::null_mut;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Mutex, MutexGuard};

extern "C" {
    fn malloc(size: usize) -> *mut c_void;
    fn free(p: *mut c_void);
}

static LOCK: Mutex<()> = Mutex::new(());
fn lock() -> MutexGuard<'static, ()> {
    LOCK.lock().unwrap_or_else(|e| e.into_inner())
}

static N_ALLOC: AtomicUsize = AtomicUsize::new(0);
static N_FREE: AtomicUsize = AtomicUsize::new(0);

unsafe extern "C" fn my_malloc(size: usize) -> *mut c_void {
    N_ALLOC.fetch_add(1, Ordering::SeqCst);
    malloc(size)
}

unsafe extern "C" fn my_free(p: *mut c_void) {
    N_FREE.fetch_add(1, Ordering::SeqCst);
    free(p)
}

#[test]
fn c3_init_hooks_reset() {
    let _g = lock();
    diff("C3 cJSON_InitHooks(NULL)", |api| unsafe {
        let mut log = String::new();
        (api.cJSON_InitHooks)(null_mut());
        let doc = cs("{\"a\":[1,2,3],\"b\":\"x\"}");
        let root = (api.cJSON_Parse)(doc.as_ptr());
        let _ = writeln!(log, "graph:\n{}", dump(root));
        let p = take_print(api, (api.cJSON_Print)(root));
        let _ = writeln!(log, "print={}", p.map(|v| show(&v)).unwrap_or("NULL".into()));
        (api.cJSON_Delete)(root);
        log
    });
}

/// Big document, so `ensure()` has to grow the print buffer several times. With
/// custom hooks `global_hooks.reallocate == NULL`, which selects the
/// copy-into-a-fresh-buffer path in both `ensure()` and `print()`.
fn big_doc() -> String {
    let mut txt = String::from("{\"items\":[");
    for i in 0..200 {
        if i > 0 {
            txt.push(',');
        }
        let _ = write!(txt, "{{\"k{i}\":{i}.5}}");
    }
    txt.push_str("]}");
    txt
}

#[test]
fn c4_init_hooks_custom() {
    let _g = lock();
    diff("C4 cJSON_InitHooks(custom malloc+free)", |api| unsafe {
        let mut log = String::new();
        N_ALLOC.store(0, Ordering::SeqCst);
        N_FREE.store(0, Ordering::SeqCst);
        let mut hooks = CJsonHooks {
            malloc_fn: Some(my_malloc),
            free_fn: Some(my_free),
        };
        (api.cJSON_InitHooks)(&mut hooks);

        let doc = cs(&big_doc());
        let root = (api.cJSON_Parse)(doc.as_ptr());
        let _ = writeln!(log, "parsed null={}", root.is_null());
        let pf = take_print(api, (api.cJSON_Print)(root));
        let pu = take_print(api, (api.cJSON_PrintUnformatted)(root));
        let _ = writeln!(
            log,
            "formatted_len={:?} unformatted_len={:?}",
            pf.as_ref().map(|v| v.len()),
            pu.as_ref().map(|v| v.len())
        );
        let _ = writeln!(log, "formatted={}", pf.map(|v| show(&v)).unwrap_or_default());
        let _ = writeln!(log, "unformatted={}", pu.map(|v| show(&v)).unwrap_or_default());
        for prebuffer in [0, 1, 4, 256, 100000] {
            for fmt in [0, 1] {
                let pb = take_print(api, (api.cJSON_PrintBuffered)(root, prebuffer, fmt));
                let _ = writeln!(
                    log,
                    "buffered({prebuffer},{fmt})={}",
                    pb.map(|v| show(&v)).unwrap_or("NULL".into())
                );
            }
        }
        (api.cJSON_Delete)(root);
        let _ = writeln!(
            log,
            "allocs={} frees={}",
            N_ALLOC.load(Ordering::SeqCst),
            N_FREE.load(Ordering::SeqCst)
        );
        (api.cJSON_InitHooks)(null_mut());
        log
    });
}

#[test]
fn c5_c6_init_hooks_partial() {
    let _g = lock();
    diff("C5/C6 partial hooks", |api| unsafe {
        let mut log = String::new();
        for (which, m, f) in [
            ("malloc only", Some(my_malloc as MallocFn), None),
            ("free only", None, Some(my_free as FreeFn)),
            ("neither", None, None),
        ] {
            N_ALLOC.store(0, Ordering::SeqCst);
            N_FREE.store(0, Ordering::SeqCst);
            let mut hooks = CJsonHooks {
                malloc_fn: m,
                free_fn: f,
            };
            (api.cJSON_InitHooks)(&mut hooks);
            let doc = cs("[1,\"two\",{\"three\":3}]");
            let root = (api.cJSON_Parse)(doc.as_ptr());
            let p = take_print(api, (api.cJSON_Print)(root));
            let _ = writeln!(
                log,
                "{which}: print={} allocs={} frees={}",
                p.map(|v| show(&v)).unwrap_or("NULL".into()),
                N_ALLOC.load(Ordering::SeqCst),
                N_FREE.load(Ordering::SeqCst)
            );
            let _ = writeln!(log, "{}", dump(root));
            (api.cJSON_Delete)(root);
            (api.cJSON_InitHooks)(null_mut());
        }
        log
    });
}

/// `ERRORS.md` row 174 — a hooks struct holding the *real* libc `malloc`/`free`.
///
/// This is the only input for which `cJSON_InitHooks` keeps
/// `global_hooks.reallocate = realloc` (the C code compares the stored function
/// pointers against `malloc`/`free`), so it exercises the function-pointer
/// identity check that the Rust translation has to reproduce.
#[test]
fn row174_hooks_with_real_libc_allocator() {
    let _g = lock();
    diff("ERRORS 174 hooks = {malloc, free}", |api| unsafe {
        let mut log = String::new();
        for (label, m, f) in [
            (
                "both real",
                Some(malloc as MallocFn),
                Some(free as FreeFn),
            ),
            ("real malloc only", Some(malloc as MallocFn), None),
            ("real free only", None, Some(free as FreeFn)),
        ] {
            let mut hooks = CJsonHooks {
                malloc_fn: m,
                free_fn: f,
            };
            (api.cJSON_InitHooks)(&mut hooks);
            let doc = cs(&big_doc());
            let root = (api.cJSON_Parse)(doc.as_ptr());
            let pf = take_print(api, (api.cJSON_Print)(root));
            let pu = take_print(api, (api.cJSON_PrintUnformatted)(root));
            let pb = take_print(api, (api.cJSON_PrintBuffered)(root, 1, 1));
            let _ = writeln!(
                log,
                "{label}: parsed={} fmt_len={:?} unfmt_len={:?} buffered_len={:?}",
                !root.is_null(),
                pf.as_ref().map(|v| v.len()),
                pu.as_ref().map(|v| v.len()),
                pb.as_ref().map(|v| v.len())
            );
            let _ = writeln!(log, "  fmt={}", pf.map(|v| show(&v)).unwrap_or_default());
            let _ = writeln!(log, "  buffered={}", pb.map(|v| show(&v)).unwrap_or_default());
            let _ = write!(log, "  {}", dump(root));
            (api.cJSON_Delete)(root);
            (api.cJSON_InitHooks)(null_mut());
        }
        log
    });
}

/// Row C89 — the complete parse → mutate → print pipeline under custom hooks.
#[test]
fn c89_pipeline_with_custom_hooks() {
    let _g = lock();
    diff("C89 pipeline with custom hooks", |api| unsafe {
        let mut log = String::new();
        N_ALLOC.store(0, Ordering::SeqCst);
        N_FREE.store(0, Ordering::SeqCst);
        let mut hooks = CJsonHooks {
            malloc_fn: Some(my_malloc),
            free_fn: Some(my_free),
        };
        (api.cJSON_InitHooks)(&mut hooks);

        let mut rng = Rng::new(0xABCD_1234_5678_9EF0);
        for round in 0..40 {
            let text = gen_json(&mut rng);
            let buf = CBuf::new(&text);
            let root = (api.cJSON_Parse)(buf.ptr());
            let _ = write!(log, "round {round} src={} graph={}", show(&text), dump(root));
            if !root.is_null() {
                let key = cs("added");
                let _ = writeln!(
                    log,
                    "  add rc={}",
                    (api.cJSON_AddNumberToObject)(root, key.as_ptr(), round as f64).is_null()
                );
                let _ = writeln!(
                    log,
                    "  insert rc={}",
                    (api.cJSON_InsertItemInArray)(root, 0, (api.cJSON_CreateNumber)(1.5))
                );
                let dup = (api.cJSON_Duplicate)(root, 1);
                let _ = write!(log, "  dup={}", dump(dup));
                let _ = writeln!(log, "  compare={}", (api.cJSON_Compare)(root, dup, 1));
                (api.cJSON_Delete)(dup);
            }
            let pf = take_print(api, (api.cJSON_Print)(root));
            let pu = take_print(api, (api.cJSON_PrintUnformatted)(root));
            let _ = writeln!(
                log,
                "  fmt={} unfmt={}",
                pf.map(|v| show(&v)).unwrap_or("NULL".into()),
                pu.map(|v| show(&v)).unwrap_or("NULL".into())
            );
            (api.cJSON_Delete)(root);
        }
        let _ = writeln!(
            log,
            "allocs={} frees={}",
            N_ALLOC.load(Ordering::SeqCst),
            N_FREE.load(Ordering::SeqCst)
        );
        (api.cJSON_InitHooks)(null_mut());
        log
    });
}
