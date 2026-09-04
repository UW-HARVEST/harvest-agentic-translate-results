//! Diagnostic: compare the exact sequence of allocation sizes requested by the
//! C and Rust libraries through a custom `js_Alloc`.

mod common;

use common::*;
use std::os::raw::{c_char, c_int, c_void};

unsafe extern "C" {
    #[link_name = "realloc"]
    fn libc_realloc(p: *mut c_void, n: usize) -> *mut c_void;
    #[link_name = "free"]
    fn libc_free(p: *mut c_void);
}

static mut TRACE: *mut Vec<(usize, c_int, bool)> = std::ptr::null_mut();

extern "C" fn trace_alloc(_ctx: *mut c_void, ptr: *mut c_void, size: c_int) -> *mut c_void {
    unsafe {
        if !TRACE.is_null() {
            (*TRACE).push(((*TRACE).len(), size, ptr.is_null()));
        }
        if size == 0 {
            if !ptr.is_null() {
                libc_free(ptr);
            }
            return std::ptr::null_mut();
        }
        libc_realloc(ptr, size as usize)
    }
}

fn collect(api: &Api, flags: c_int, src: &str) -> Vec<(usize, c_int, bool)> {
    let mut v: Vec<(usize, c_int, bool)> = Vec::new();
    unsafe {
        TRACE = &mut v as *mut Vec<(usize, c_int, bool)>;
    }
    let j = (api.js_newstate)(Some(trace_alloc), std::ptr::null_mut(), flags);
    assert!(!j.is_null());
    let z = cstr(src);
    let _ = (api.js_dostring)(j, z.as_ptr() as *const c_char);
    (api.js_freestate)(j);
    unsafe {
        TRACE = std::ptr::null_mut();
    }
    v
}

/// The exact sequence of sizes requested from a caller-supplied `js_Alloc` is
/// observable behaviour: it is what `js_setlimit`'s `memlimit` is charged
/// against. This asserts C and Rust request byte-identical allocations, which
/// in particular pins `sizeof(js_State)` (the very first request).
#[test]
fn alloc_size_sequence_is_identical() {
    let _g = RUN_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let (capi, rapi) = both_apis();
    let programs = [
        "",
        "var x = 1+1;",
        "var a=[]; for (var i=0;i<200;++i) a.push(i*i);",
        "var s=''; for (var i=0;i<100;++i) s += 'xy';",
        "var o={}; for (var i=0;i<100;++i) o['k'+i]=i;",
        "function f(n){ return n<2?n:f(n-1)+f(n-2) } f(15);",
        "JSON.stringify({a:[1,2,3],b:'x'});",
        "'aXbYc'.replace(/[A-Z]/g, '-');",
        "try { null.x } catch (e) { }",
        "new Date(0).toISOString();",
    ];
    for flags in [0, 1] {
        for src in programs {
            let a = collect(capi, flags, src);
            let b = collect(rapi, flags, src);
            let asz: Vec<c_int> = a.iter().map(|r| r.1).collect();
            let bsz: Vec<c_int> = b.iter().map(|r| r.1).collect();
            if asz != bsz {
                let first = (0..asz.len().max(bsz.len()))
                    .find(|&i| asz.get(i) != bsz.get(i))
                    .unwrap();
                panic!(
                    "allocation size sequence differs for flags={flags} src={src:?}\n\
                     C calls={} RUST calls={}\n\
                     first difference at #{first}: C={:?} RUST={:?}\n\
                     C total={} RUST total={}",
                    asz.len(),
                    bsz.len(),
                    asz.get(first),
                    bsz.get(first),
                    asz.iter().filter(|v| **v > 0).map(|v| *v as i64).sum::<i64>(),
                    bsz.iter().filter(|v| **v > 0).map(|v| *v as i64).sum::<i64>(),
                );
            }
        }
    }
}

#[test]
fn diagnose_alloc_size_sequence() {
    let _g = RUN_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let (capi, rapi) = both_apis();
    let src = "var x = 1+1;";
    let a = collect(capi, 0, src);
    let b = collect(rapi, 0, src);
    println!("C calls={} RUST calls={}", a.len(), b.len());
    let sum_a: i64 = a.iter().filter(|r| r.1 > 0).map(|r| r.1 as i64).sum();
    let sum_b: i64 = b.iter().filter(|r| r.1 > 0).map(|r| r.1 as i64).sum();
    println!("C total={sum_a} RUST total={sum_b} diff={}", sum_b - sum_a);
    let mut shown = 0;
    for i in 0..a.len().max(b.len()) {
        let x = a.get(i);
        let y = b.get(i);
        if x.map(|r| (r.1, r.2)) != y.map(|r| (r.1, r.2)) {
            println!("first diff at #{i}: C={x:?} RUST={y:?}");
            for k in i.saturating_sub(3)..(i + 6).min(a.len().max(b.len())) {
                println!("  #{k}: C={:?} RUST={:?}", a.get(k), b.get(k));
            }
            shown += 1;
            if shown >= 5 {
                break;
            }
        }
    }
    // Histogram of sizes that differ in multiplicity
    use std::collections::BTreeMap;
    let mut ha: BTreeMap<c_int, i64> = BTreeMap::new();
    let mut hb: BTreeMap<c_int, i64> = BTreeMap::new();
    for r in &a {
        *ha.entry(r.1).or_default() += 1;
    }
    for r in &b {
        *hb.entry(r.1).or_default() += 1;
    }
    for (k, v) in &ha {
        let w = hb.get(k).copied().unwrap_or(0);
        if *v != w {
            println!("size {k}: C count {v}, RUST count {w}");
        }
    }
    for (k, w) in &hb {
        if !ha.contains_key(k) {
            println!("size {k}: C count 0, RUST count {w}");
        }
    }
}
