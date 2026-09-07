//! Phase B — CONFIGS.md rows 61..65: `cJSON_InitHooks` configurations.
//!
//! Every test in this file mutates per-library global state, so they all take
//! the shared `global_lock()` and restore the default hooks before returning.

mod common;
use common::*;
use std::ffi::{c_char, c_int, c_void};
use std::ptr;
use std::sync::atomic::{AtomicUsize, Ordering};

unsafe extern "C" {
    fn malloc(size: usize) -> *mut c_void;
    fn free(p: *mut c_void);
}

static C_MALLOC: AtomicUsize = AtomicUsize::new(0);
static C_FREE: AtomicUsize = AtomicUsize::new(0);
static R_MALLOC: AtomicUsize = AtomicUsize::new(0);
static R_FREE: AtomicUsize = AtomicUsize::new(0);

unsafe extern "C" fn c_malloc(size: usize) -> *mut c_void {
    C_MALLOC.fetch_add(1, Ordering::SeqCst);
    unsafe { malloc(size) }
}
unsafe extern "C" fn c_free(p: *mut c_void) {
    C_FREE.fetch_add(1, Ordering::SeqCst);
    unsafe { free(p) }
}
unsafe extern "C" fn r_malloc(size: usize) -> *mut c_void {
    R_MALLOC.fetch_add(1, Ordering::SeqCst);
    unsafe { malloc(size) }
}
unsafe extern "C" fn r_free(p: *mut c_void) {
    R_FREE.fetch_add(1, Ordering::SeqCst);
    unsafe { free(p) }
}

fn big_tree() -> Node {
    let mut rng = Rng::new(0x610);
    Node::Object(
        (0..40)
            .map(|i| (format!("key{}", i), random_node(&mut rng, 3)))
            .collect(),
    )
}

/// Exercise the whole pipeline and return a transcript of everything observable.
unsafe fn workload(api: &Api, node: &Node) -> String {
    let mut out = String::new();
    let item = build(api, node);
    out.push_str(&show(&print_and_free(api, item)));
    out.push('\n');
    out.push_str(&show(&print_unformatted_and_free(api, item)));
    out.push('\n');
    // prebuffer=1 forces the ensure() growth path many times over
    for prebuffer in [1i32, 4, 4096] {
        for fmt in [0i32, 1] {
            out.push_str(&show(&print_buffered_and_free(api, item, prebuffer, fmt)));
            out.push('\n');
        }
    }
    // parse back, duplicate, compare, minify
    if let Some(text) = print_and_free(api, item) {
        let mut buf = cbytes(&text);
        let parsed = (api.cJSON_Parse)(buf.as_ptr() as *const c_char);
        out.push_str(&show(&print_and_free(api, parsed)));
        out.push('\n');
        let dup = (api.cJSON_Duplicate)(parsed, 1);
        out.push_str(&show(&print_and_free(api, dup)));
        out.push('\n');
        out.push_str(&format!("cmp={}\n", (api.cJSON_Compare)(parsed, dup, 1)));
        (api.cJSON_Delete)(dup);
        (api.cJSON_Delete)(parsed);
        (api.cJSON_Minify)(buf.as_mut_ptr() as *mut c_char);
        out.push_str(&String::from_utf8_lossy(&buf[..buf.len() - 1]));
        out.push('\n');
    }
    // exercise cJSON_malloc / cJSON_free through the configured hooks
    let p = (api.cJSON_malloc)(64);
    out.push_str(&format!("malloc_null={}\n", p.is_null()));
    (api.cJSON_free)(p);
    (api.cJSON_Delete)(item);
    out
}

unsafe fn reset(c: &Api, r: &Api) {
    (c.cJSON_InitHooks)(ptr::null_mut());
    (r.cJSON_InitHooks)(ptr::null_mut());
}

/// Row 61 — `cJSON_InitHooks(NULL)` resets to the libc allocators.
#[test]
fn row61_init_hooks_null() {
    let _g = global_lock();
    unsafe {
        let (c, r) = both();
        (c.cJSON_InitHooks)(ptr::null_mut());
        (r.cJSON_InitHooks)(ptr::null_mut());
        let n = big_tree();
        assert_eq!(workload(&c, &n), workload(&r, &n), "row61");
        reset(&c, &r);
    }
}

/// Rows 62, 63, 64, 65 — every combination of custom / default allocators.
#[test]
fn row62_65_init_hooks_combinations() {
    let _g = global_lock();
    unsafe {
        let (c, r) = both();
        let n = big_tree();

        // (label, custom malloc?, custom free?)
        let combos: [(&str, bool, bool); 4] = [
            ("row62 custom malloc + custom free (reallocate == NULL)", true, true),
            ("row63 custom malloc only", true, false),
            ("row64 custom free only", false, true),
            ("row65 explicit libc malloc+free (reallocate == realloc)", false, false),
        ];

        for (label, cm, cf) in combos {
            C_MALLOC.store(0, Ordering::SeqCst);
            C_FREE.store(0, Ordering::SeqCst);
            R_MALLOC.store(0, Ordering::SeqCst);
            R_FREE.store(0, Ordering::SeqCst);

            let mut hc = CJsonHooks {
                malloc_fn: if cm { Some(c_malloc) } else { Some(malloc) },
                free_fn: if cf { Some(c_free) } else { Some(free) },
            };
            let mut hr = CJsonHooks {
                malloc_fn: if cm { Some(r_malloc) } else { Some(malloc) },
                free_fn: if cf { Some(r_free) } else { Some(free) },
            };
            (c.cJSON_InitHooks)(&mut hc);
            (r.cJSON_InitHooks)(&mut hr);

            let a = workload(&c, &n);
            let b = workload(&r, &n);
            assert_eq!(a, b, "{}: observable output", label);

            if cm {
                assert!(C_MALLOC.load(Ordering::SeqCst) > 0, "{}: C hook unused", label);
                assert!(R_MALLOC.load(Ordering::SeqCst) > 0, "{}: Rust hook unused", label);
                assert_eq!(
                    C_MALLOC.load(Ordering::SeqCst),
                    R_MALLOC.load(Ordering::SeqCst),
                    "{}: allocation count",
                    label
                );
            }
            if cf {
                assert_eq!(
                    C_FREE.load(Ordering::SeqCst),
                    R_FREE.load(Ordering::SeqCst),
                    "{}: free count",
                    label
                );
            }
            reset(&c, &r);
        }
    }
}

/// Row 62b — hooks struct with NULL members must fall back per member.
#[test]
fn row62b_init_hooks_null_members() {
    let _g = global_lock();
    unsafe {
        let (c, r) = both();
        let n = big_tree();
        for (mf_c, ff_c, mf_r, ff_r) in [
            (None, None, None, None),
            (Some(c_malloc as unsafe extern "C" fn(usize) -> *mut c_void), None,
             Some(r_malloc as unsafe extern "C" fn(usize) -> *mut c_void), None),
            (None, Some(c_free as unsafe extern "C" fn(*mut c_void)),
             None, Some(r_free as unsafe extern "C" fn(*mut c_void))),
        ] {
            let mut hc = CJsonHooks {
                malloc_fn: mf_c,
                free_fn: ff_c,
            };
            let mut hr = CJsonHooks {
                malloc_fn: mf_r,
                free_fn: ff_r,
            };
            (c.cJSON_InitHooks)(&mut hc);
            (r.cJSON_InitHooks)(&mut hr);
            assert_eq!(workload(&c, &n), workload(&r, &n), "row62b");
            reset(&c, &r);
        }
        let _: c_int = 0;
    }
}
