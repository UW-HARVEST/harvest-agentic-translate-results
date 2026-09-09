//! Phase B — CONFIGS rows 9 & 10: custom allocator hooks.
//!
//! These mutate PROCESS-GLOBAL state inside each library, so they live in
//! their own test binary and must not run alongside other tests.
#![allow(unused_unsafe, dead_code, unsafe_op_in_unsafe_fn)]
mod common;
use common::*;
use std::os::raw::c_void;

// ------------------------------------------------------------- rows 9 & 10

unsafe extern "C" fn my_malloc(n: usize) -> *mut c_void {
    unsafe { libc_malloc(n) }
}
unsafe extern "C" fn my_realloc(p: *mut c_void, n: usize) -> *mut c_void {
    unsafe { libc_realloc(p, n) }
}
unsafe extern "C" fn my_free(p: *mut c_void) {
    unsafe { libc_free(p) }
}

unsafe extern "C" {
    #[link_name = "malloc"]
    fn libc_malloc(n: usize) -> *mut c_void;
    #[link_name = "realloc"]
    fn libc_realloc(p: *mut c_void, n: usize) -> *mut c_void;
    #[link_name = "free"]
    fn libc_free(p: *mut c_void);
}

#[test]
fn row09_row10_alloc_funcs() {
    let (c, r) = both();
    let mut logs = Vec::new();
    for l in [c, r] {
        let get1 = sym!(l, "json_get_alloc_funcs", (*mut Pfn, *mut Pfn));
        let set1 = sym!(l, "json_set_alloc_funcs", (Pfn, Pfn));
        let get2 = sym!(l, "json_get_alloc_funcs2", (*mut Pfn, *mut Pfn, *mut Pfn));
        let set2 = sym!(l, "json_set_alloc_funcs2", (Pfn, Pfn, Pfn));
        let mut log: Vec<String> = Vec::new();
        unsafe {
            // read defaults (should be libc malloc/realloc/free)
            let (mut m, mut re, mut fr): (Pfn, Pfn, Pfn) = (
                std::ptr::null(),
                std::ptr::null(),
                std::ptr::null(),
            );
            get2(&mut m, &mut re, &mut fr);
            log.push(format!(
                "defaults: malloc==libc {} realloc==libc {} free==libc {}",
                m == libc_malloc as Pfn,
                re == libc_realloc as Pfn,
                fr == libc_free as Pfn
            ));
            let saved = (m, re, fr);

            // json_set_alloc_funcs must NULL out realloc
            set1(my_malloc as Pfn, my_free as Pfn);
            let (mut m2, mut re2, mut fr2): (Pfn, Pfn, Pfn) =
                (std::ptr::null(), std::ptr::null(), std::ptr::null());
            get2(&mut m2, &mut re2, &mut fr2);
            log.push(format!(
                "after set1: malloc_ok={} realloc_null={} free_ok={}",
                m2 == my_malloc as Pfn,
                re2.is_null(),
                fr2 == my_free as Pfn
            ));
            // exercise the realloc-emulation path (do_realloc == NULL)
            let malloc = sym!(l, "jsonp_malloc", (usize) -> *mut c_void);
            let realloc = sym!(l, "jsonp_realloc", (*mut c_void, usize, usize) -> *mut c_void);
            let p = malloc(16) as *mut u8;
            for i in 0..16u8 {
                *p.add(i as usize) = i;
            }
            let q = realloc(p as *mut c_void, 16, 32) as *mut u8;
            log.push(format!(
                "emulated realloc grow keeps data: {}",
                (0..16u8).all(|i| *q.add(i as usize) == i)
            ));
            let z = realloc(q as *mut c_void, 32, 0);
            log.push(format!("emulated realloc to 0 -> null: {}", z.is_null()));

            // get_alloc_funcs (2-arg form) and NULL-tolerance
            let (mut m3, mut fr3): (Pfn, Pfn) = (std::ptr::null(), std::ptr::null());
            get1(&mut m3, &mut fr3);
            log.push(format!(
                "get1: {} {}",
                m3 == my_malloc as Pfn,
                fr3 == my_free as Pfn
            ));
            get1(std::ptr::null_mut(), std::ptr::null_mut());
            get2(std::ptr::null_mut(), std::ptr::null_mut(), std::ptr::null_mut());
            log.push("null-out-ptr tolerated".into());

            // set2 with custom realloc
            set2(my_malloc as Pfn, my_realloc as Pfn, my_free as Pfn);
            let (mut m4, mut re4, mut fr4): (Pfn, Pfn, Pfn) =
                (std::ptr::null(), std::ptr::null(), std::ptr::null());
            get2(&mut m4, &mut re4, &mut fr4);
            log.push(format!(
                "after set2: {} {} {}",
                m4 == my_malloc as Pfn,
                re4 == my_realloc as Pfn,
                fr4 == my_free as Pfn
            ));

            // restore
            set2(saved.0, saved.1, saved.2);
            let (mut m5, mut re5, mut fr5): (Pfn, Pfn, Pfn) =
                (std::ptr::null(), std::ptr::null(), std::ptr::null());
            get2(&mut m5, &mut re5, &mut fr5);
            log.push(format!("restored: {}", (m5, re5, fr5) == saved));
        }
        logs.push(log);
    }
    assert_eq!(logs[0], logs[1], "alloc-func surface mismatch");
}

