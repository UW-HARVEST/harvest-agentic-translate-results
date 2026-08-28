//! Allocation-accounting differential tests.
//!
//! Output comparison alone cannot see two classes of translation defect:
//!
//! * an allocation that is **one byte too small** — the overflowing write lands
//!   in `malloc`'s slack and the printed bytes still match;
//! * a **missing `deallocate`** — a leak changes nothing observable.
//!
//! `cJSON_InitHooks` lets a caller supply the allocator, so both are detectable
//! from outside the library: this file installs a guarded allocator that
//! * puts a magic header and a magic footer around every block and validates
//!   them on free (catching under-allocation / overflow), and
//! * counts allocations and frees, so the live-block count and the total call
//!   counts can be required to match between the C and the Rust library.
#![allow(non_snake_case)]

mod harness;

use harness::*;
use std::ffi::{c_char, c_int, c_void};
use std::sync::atomic::{AtomicI64, AtomicU64, Ordering};

const HEADER: usize = 32; // keeps the payload 16-byte aligned
const FOOTER: usize = 16;
const MAGIC_HEAD: u64 = 0xC0FFEE_1234_5678;
const MAGIC_FOOT: u64 = 0xDEADBEEF_FEEDFACE;
const FILL: u8 = 0xA7;

extern "C" {
    #[link_name = "malloc"]
    fn libc_malloc_fn(n: usize) -> *mut c_void;
    #[link_name = "free"]
    fn libc_free_fn(p: *mut c_void);
}

static ALLOCS: AtomicI64 = AtomicI64::new(0);
static FREES: AtomicI64 = AtomicI64::new(0);
static LIVE: AtomicI64 = AtomicI64::new(0);
static LIVE_BYTES: AtomicI64 = AtomicI64::new(0);
static VIOLATIONS: AtomicU64 = AtomicU64::new(0);
static LAST_VIOLATION: AtomicU64 = AtomicU64::new(0);

const V_BAD_HEAD: u64 = 1;
const V_BAD_FOOT: u64 = 2;

fn reset_counters() {
    ALLOCS.store(0, Ordering::SeqCst);
    FREES.store(0, Ordering::SeqCst);
    LIVE.store(0, Ordering::SeqCst);
    LIVE_BYTES.store(0, Ordering::SeqCst);
}

#[derive(Debug, PartialEq, Clone, Copy)]
struct Counts {
    allocs: i64,
    frees: i64,
    live: i64,
    live_bytes: i64,
}

fn counts() -> Counts {
    Counts {
        allocs: ALLOCS.load(Ordering::SeqCst),
        frees: FREES.load(Ordering::SeqCst),
        live: LIVE.load(Ordering::SeqCst),
        live_bytes: LIVE_BYTES.load(Ordering::SeqCst),
    }
}

unsafe extern "C" fn guarded_malloc(size: usize) -> *mut c_void {
    let block = libc_malloc_fn(HEADER + size + FOOTER);
    if block.is_null() {
        return std::ptr::null_mut();
    }
    let b = block as *mut u8;
    std::ptr::write_unaligned(b as *mut u64, MAGIC_HEAD);
    std::ptr::write_unaligned((b.add(8)) as *mut u64, size as u64);
    // poison the payload so that reads of uninitialised bytes are visible
    std::ptr::write_bytes(b.add(HEADER), FILL, size);
    // footer magic, twice
    std::ptr::write_unaligned(b.add(HEADER + size) as *mut u64, MAGIC_FOOT);
    std::ptr::write_unaligned(b.add(HEADER + size + 8) as *mut u64, MAGIC_FOOT);

    ALLOCS.fetch_add(1, Ordering::SeqCst);
    LIVE.fetch_add(1, Ordering::SeqCst);
    LIVE_BYTES.fetch_add(size as i64, Ordering::SeqCst);
    b.add(HEADER) as *mut c_void
}

unsafe extern "C" fn guarded_free(p: *mut c_void) {
    if p.is_null() {
        FREES.fetch_add(1, Ordering::SeqCst);
        return;
    }
    let b = (p as *mut u8).sub(HEADER);
    let head = std::ptr::read_unaligned(b as *const u64);
    if head != MAGIC_HEAD {
        VIOLATIONS.fetch_add(1, Ordering::SeqCst);
        LAST_VIOLATION.store(V_BAD_HEAD, Ordering::SeqCst);
        // Not one of our blocks (or the header was overwritten): leak it rather
        // than corrupting the heap further.
        return;
    }
    let size = std::ptr::read_unaligned(b.add(8) as *const u64) as usize;
    let f1 = std::ptr::read_unaligned(b.add(HEADER + size) as *const u64);
    let f2 = std::ptr::read_unaligned(b.add(HEADER + size + 8) as *const u64);
    if f1 != MAGIC_FOOT || f2 != MAGIC_FOOT {
        VIOLATIONS.fetch_add(1, Ordering::SeqCst);
        LAST_VIOLATION.store(V_BAD_FOOT, Ordering::SeqCst);
    }
    // scrub so a use-after-free shows up as a header mismatch next time
    std::ptr::write_unaligned(b as *mut u64, 0);
    FREES.fetch_add(1, Ordering::SeqCst);
    LIVE.fetch_sub(1, Ordering::SeqCst);
    LIVE_BYTES.fetch_sub(size as i64, Ordering::SeqCst);
    libc_free_fn(b as *mut c_void);
}

fn install_guarded(api: &Api) {
    let mut h = CJsonHooks {
        malloc_fn: Some(guarded_malloc),
        free_fn: Some(guarded_free),
    };
    unsafe { (api.cJSON_InitHooks)(&mut h) };
}

fn install_default(api: &Api) {
    unsafe { (api.cJSON_InitHooks)(std::ptr::null_mut()) };
}

fn check_no_violation(label: &str) {
    let v = VIOLATIONS.swap(0, Ordering::SeqCst);
    if v != 0 {
        let kind = match LAST_VIOLATION.swap(0, Ordering::SeqCst) {
            V_BAD_HEAD => "bad/overwritten block header (invalid free or use-after-free)",
            V_BAD_FOOT => "footer canary overwritten (the library wrote past the end \
                           of an allocation — the allocation is too small)",
            _ => "unknown",
        };
        panic!("{label}: {v} guarded-allocator violation(s): {kind}");
    }
}

/// Runs `body` on `api` under the guarded allocator and returns the allocator
/// statistics for the run.
fn measure(api: &Api, label: &str, body: impl FnOnce(&Api)) -> Counts {
    install_guarded(api);
    reset_counters();
    body(api);
    let c = counts();
    check_no_violation(label);
    install_default(api);
    c
}

/// Runs the same closure on both libraries and requires identical allocator
/// statistics (and no canary violations on either side).
fn compare_alloc_behaviour(c: &Api, r: &Api, label: &str, body: fn(&Api)) {
    let cc = measure(c, &format!("{label} [C]"), body);
    let cr = measure(r, &format!("{label} [Rust]"), body);
    assert_eq!(
        cc, cr,
        "{label}: allocator call accounting differs\nC    = {cc:?}\nRust = {cr:?}"
    );
    assert_eq!(
        cc.live, 0,
        "{label}: the C library left {} block(s) ({} bytes) live",
        cc.live, cc.live_bytes
    );
    assert!(
        cc.allocs > 0,
        "{label}: the guarded allocator was never called — the scenario is vacuous"
    );
    eprintln!("{label}: {} allocations / {} frees through the hooks", cc.allocs, cc.frees);
}

// ---------------------------------------------------------------------------
// scenario bodies (identical code for both libraries)
// ---------------------------------------------------------------------------

fn scenario_build_print_delete(api: &Api) {
    unsafe {
        let mut rng = Rng::new(0x9AA1_5EED);
        for depth in 0..4usize {
            for _ in 0..40 {
                let spec = rand_spec(&mut rng, depth);
                let b = build(api, &spec);
                for p in [
                    (api.cJSON_Print)(b.root),
                    (api.cJSON_PrintUnformatted)(b.root),
                    (api.cJSON_PrintBuffered)(b.root, 0, 1),
                    (api.cJSON_PrintBuffered)(b.root, 1, 0),
                    (api.cJSON_PrintBuffered)(b.root, 256, 1),
                    (api.cJSON_PrintBuffered)(b.root, 4096, 0),
                ] {
                    if !p.is_null() {
                        (api.cJSON_free)(p as *mut c_void);
                    }
                }
                let mut buf = vec![0u8; 8192];
                for fmt in [0, 1] {
                    (api.cJSON_PrintPreallocated)(
                        b.root,
                        buf.as_mut_ptr() as *mut c_char,
                        8192,
                        fmt,
                    );
                }
                b.delete();
            }
        }
    }
}

fn scenario_parse(api: &Api) {
    let docs: &[&[u8]] = &[
        b"null",
        b"true",
        b"false",
        b"0",
        b"-1.5e3",
        b"\"\"",
        b"\"plain\"",
        b"\"\\b\\f\\n\\r\\t\\\"\\\\\\/\"",
        b"\"\\u0041\\u00e9\\u07ff\\u0800\\uffff\"",
        b"\"\\ud800\\udc00\\udbff\\udfff\"",
        b"\"\\uZZZZ\"",
        b"[]",
        b"[1,2,3]",
        b"[[[[[1]]]]]",
        b"{}",
        b"{\"a\":1}",
        b"{\"a\":{\"b\":[1,2,{\"c\":\"d\"}]},\"e\":null}",
        b"\xEF\xBB\xBF{\"bom\":1}",
        // failing parses must also balance their allocations
        b"[1,2",
        b"{\"a\":",
        b"\"unterminated",
        b"\"bad\\escape\"",
        b"\"\\ud800\"",
        b"-",
        b"[1,]",
        b"{x:1}",
    ];
    unsafe {
        for d in docs {
            let b = Bytes::new(d);
            for variant in 0..4 {
                let mut end: *const c_char = std::ptr::null();
                let it = match variant {
                    0 => (api.cJSON_Parse)(b.as_ptr()),
                    1 => (api.cJSON_ParseWithLength)(b.as_ptr(), d.len() + 1),
                    2 => (api.cJSON_ParseWithOpts)(b.as_ptr(), &mut end, 1),
                    _ => (api.cJSON_ParseWithLengthOpts)(b.as_ptr(), d.len() + 1, &mut end, 0),
                };
                if !it.is_null() {
                    let p = (api.cJSON_Print)(it);
                    if !p.is_null() {
                        (api.cJSON_free)(p as *mut c_void);
                    }
                }
                (api.cJSON_Delete)(it);
            }
        }
    }
}

fn scenario_rekey(api: &Api) {
    // Exercises add_item_to_object's "free the previous key" branch and the
    // cJSON_StringIsConst guard around it (cJSON.c:2060) — a missing free here
    // is a pure leak, invisible to output comparison.
    unsafe {
        let const_key = Bytes::new(b"a_constant_key_owned_by_the_caller");
        for use_cs_first in [false, true] {
            for round in 0..8usize {
                let o1 = (api.cJSON_CreateObject)();
                let o2 = (api.cJSON_CreateObject)();
                let n = (api.cJSON_CreateNumber)(round as f64);
                let k1 = Bytes::new(format!("heap_key_number_{round}").as_bytes());
                let k2 = Bytes::new(
                    format!("a_much_longer_heap_key_number_{round}").as_bytes(),
                );
                if use_cs_first {
                    (api.cJSON_AddItemToObjectCS)(o1, const_key.as_ptr(), n);
                } else {
                    (api.cJSON_AddItemToObject)(o1, k1.as_ptr(), n);
                }
                let d = (api.cJSON_DetachItemViaPointer)(o1, n);
                // re-key: the previous key must be freed iff it was heap-owned
                (api.cJSON_AddItemToObject)(o2, k2.as_ptr(), d);
                let d = (api.cJSON_DetachItemViaPointer)(o2, d);
                // and once more, this time with a constant key
                (api.cJSON_AddItemToObjectCS)(o2, const_key.as_ptr(), d);
                (api.cJSON_Delete)(o1);
                (api.cJSON_Delete)(o2);
            }
        }

        // cJSON_ReplaceItemInObject also rewrites `->string`
        for cs_flag in [false, true] {
            let o = (api.cJSON_CreateObject)();
            (api.cJSON_AddNumberToObject)(o, cs("k").as_ptr(), 1.0);
            let n = (api.cJSON_CreateNumber)(2.0);
            let key = Bytes::new(b"k");
            let rc = if cs_flag {
                (api.cJSON_ReplaceItemInObjectCaseSensitive)(o, key.as_ptr(), n)
            } else {
                (api.cJSON_ReplaceItemInObject)(o, key.as_ptr(), n)
            };
            if rc == 0 {
                (api.cJSON_Delete)(n);
            }
            (api.cJSON_Delete)(o);
        }

        // cJSON_SetValuestring's shorter/longer paths
        for new in [&b"x"[..], &b"abcdefgh"[..], &b"a much longer replacement"[..]] {
            let s = (api.cJSON_CreateString)(cs("original").as_ptr());
            let nb = Bytes::new(new);
            (api.cJSON_SetValuestring)(s, nb.as_ptr());
            (api.cJSON_Delete)(s);
        }
    }
}

fn scenario_duplicate_and_compare(api: &Api) {
    unsafe {
        let mut rng = Rng::new(0x5150_D0C5);
        for _ in 0..60 {
            let spec = rand_spec(&mut rng, 3);
            let b = build(api, &spec);
            for recurse in [0, 1] {
                let d = (api.cJSON_Duplicate)(b.root, recurse);
                if !d.is_null() {
                    for cs_flag in [0, 1] {
                        (api.cJSON_Compare)(b.root, d, cs_flag);
                    }
                    let p = (api.cJSON_Print)(d);
                    if !p.is_null() {
                        (api.cJSON_free)(p as *mut c_void);
                    }
                }
                (api.cJSON_Delete)(d);
            }
            b.delete();
        }
    }
}

fn scenario_typed_arrays_and_helpers(api: &Api) {
    unsafe {
        let ints: Vec<c_int> = (0..40).collect();
        let floats: Vec<f32> = (0..40).map(|i| i as f32 / 3.0).collect();
        let doubles: Vec<f64> = (0..40).map(|i| i as f64 / 7.0).collect();
        let sb: Vec<Bytes> = (0..40).map(|i| Bytes::new(format!("s{i}").as_bytes())).collect();
        let sp: Vec<*const c_char> = sb.iter().map(|b| b.as_ptr()).collect();
        for count in [0i32, 1, 2, 17, 40] {
            for a in [
                (api.cJSON_CreateIntArray)(ints.as_ptr(), count),
                (api.cJSON_CreateFloatArray)(floats.as_ptr(), count),
                (api.cJSON_CreateDoubleArray)(doubles.as_ptr(), count),
                (api.cJSON_CreateStringArray)(sp.as_ptr(), count),
            ] {
                let p = (api.cJSON_Print)(a);
                if !p.is_null() {
                    (api.cJSON_free)(p as *mut c_void);
                }
                (api.cJSON_Delete)(a);
            }
        }
        // all nine Add*ToObject helpers plus references
        let o = (api.cJSON_CreateObject)();
        (api.cJSON_AddNullToObject)(o, cs("n").as_ptr());
        (api.cJSON_AddTrueToObject)(o, cs("t").as_ptr());
        (api.cJSON_AddFalseToObject)(o, cs("f").as_ptr());
        (api.cJSON_AddBoolToObject)(o, cs("b").as_ptr(), 1);
        (api.cJSON_AddNumberToObject)(o, cs("num").as_ptr(), 1.5);
        (api.cJSON_AddStringToObject)(o, cs("s").as_ptr(), cs("v\t\"x\"").as_ptr());
        (api.cJSON_AddRawToObject)(o, cs("raw").as_ptr(), cs("[1]").as_ptr());
        let inner = (api.cJSON_AddObjectToObject)(o, cs("obj").as_ptr());
        let arr = (api.cJSON_AddArrayToObject)(o, cs("arr").as_ptr());
        (api.cJSON_AddNumberToObject)(inner, cs("deep").as_ptr(), 2.5);
        (api.cJSON_AddItemToArray)(arr, (api.cJSON_CreateNumber)(3.5));
        (api.cJSON_AddItemReferenceToArray)(arr, inner);
        (api.cJSON_AddItemReferenceToObject)(o, cs("ref").as_ptr(), inner);
        let p = (api.cJSON_Print)(o);
        if !p.is_null() {
            (api.cJSON_free)(p as *mut c_void);
        }
        (api.cJSON_Delete)(o);
    }
}

fn scenario_mutations(api: &Api) {
    unsafe {
        for n in 0..6usize {
            let spec = Spec::Obj(
                (0..n)
                    .map(|i| (format!("k{i}").into_bytes(), Spec::Str(format!("v{i}").into_bytes())))
                    .collect(),
            );
            for which in 0..(n as c_int + 2) {
                let b = build(api, &spec);
                let d = (api.cJSON_DetachItemFromArray)(b.root, which);
                (api.cJSON_Delete)(d);
                b.delete();

                let b = build(api, &spec);
                (api.cJSON_DeleteItemFromArray)(b.root, which);
                b.delete();

                let b = build(api, &spec);
                let item = (api.cJSON_CreateString)(cs("inserted").as_ptr());
                if (api.cJSON_InsertItemInArray)(b.root, which, item) == 0 {
                    (api.cJSON_Delete)(item);
                }
                b.delete();

                let b = build(api, &spec);
                let item = (api.cJSON_CreateString)(cs("replacement").as_ptr());
                if (api.cJSON_ReplaceItemInArray)(b.root, which, item) == 0 {
                    (api.cJSON_Delete)(item);
                }
                b.delete();
            }
            for key in [&b"k0"[..], &b"K0"[..], &b"absent"[..]] {
                let kb = Bytes::new(key);
                let b = build(api, &spec);
                (api.cJSON_DeleteItemFromObject)(b.root, kb.as_ptr());
                b.delete();
                let b = build(api, &spec);
                (api.cJSON_DeleteItemFromObjectCaseSensitive)(b.root, kb.as_ptr());
                b.delete();
                let b = build(api, &spec);
                let item = (api.cJSON_CreateNumber)(9.0);
                if (api.cJSON_ReplaceItemInObject)(b.root, kb.as_ptr(), item) == 0 {
                    (api.cJSON_Delete)(item);
                }
                b.delete();
            }
        }
    }
}

// ---------------------------------------------------------------------------
// tests
// ---------------------------------------------------------------------------

#[test]
fn guarded_allocator_build_print_delete() {
    let (c, r) = both();
    let _g = lock_global_state();
    compare_alloc_behaviour(&c, &r, "build/print/delete", scenario_build_print_delete);
}

#[test]
fn guarded_allocator_parse() {
    let (c, r) = both();
    let _g = lock_global_state();
    compare_alloc_behaviour(&c, &r, "parse", scenario_parse);
}

#[test]
fn guarded_allocator_rekey() {
    let (c, r) = both();
    let _g = lock_global_state();
    compare_alloc_behaviour(&c, &r, "re-key / SetValuestring", scenario_rekey);
}

#[test]
fn guarded_allocator_duplicate_and_compare() {
    let (c, r) = both();
    let _g = lock_global_state();
    compare_alloc_behaviour(
        &c,
        &r,
        "duplicate/compare",
        scenario_duplicate_and_compare,
    );
}

#[test]
fn guarded_allocator_typed_arrays_and_helpers() {
    let (c, r) = both();
    let _g = lock_global_state();
    compare_alloc_behaviour(
        &c,
        &r,
        "typed arrays / Add*ToObject",
        scenario_typed_arrays_and_helpers,
    );
}

#[test]
fn guarded_allocator_mutations() {
    let (c, r) = both();
    let _g = lock_global_state();
    compare_alloc_behaviour(&c, &r, "detach/insert/replace", scenario_mutations);
}
