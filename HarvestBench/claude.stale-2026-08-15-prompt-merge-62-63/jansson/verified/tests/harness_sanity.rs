//! Negative/positive controls for the differential harness itself.
//!
//! Without these, a bug in the harness (e.g. every call returning None, or the
//! same library being loaded twice) would make every other test pass vacuously.

mod common;

use common::*;
use libloading::{Library, Symbol};
use std::ffi::c_char;

#[test]
fn the_two_libraries_are_distinct_files() {
    let l = libs();
    // Different Library handles must come from different paths, else every
    // "differential" test would be comparing a library with itself.
    let c_path = std::env::var("C_JANSSON_SO")
        .unwrap_or_else(|_| format!("{}/cbuild/libjansson.so", env!("CARGO_MANIFEST_DIR")));
    let r_path = std::env::var("RUST_JANSSON_SO").unwrap_or_default();
    assert!(std::path::Path::new(&c_path).exists(), "C .so missing at {}", c_path);
    // Both handles must resolve json_dumps independently.
    unsafe {
        let _c: Symbol<FnDumps> = sym(&l.c, "json_dumps");
        let _r: Symbol<FnDumps> = sym(&l.r, "json_dumps");
    }
    assert_ne!(c_path, r_path, "C and Rust .so paths must differ");
}

/// The C library must produce these exact bytes. This pins the harness to real
/// observed behaviour so a vacuous `None == None` cannot pass unnoticed.
#[test]
fn c_library_produces_expected_concrete_output() {
    let l = libs();
    unsafe {
        let obj: Symbol<unsafe extern "C" fn() -> *mut json_t> = sym(&l.c, "json_object");
        let int: Symbol<FnInt> = sym(&l.c, "json_integer");
        let oset: Symbol<FnObjSetNew> = sym(&l.c, "json_object_set_new");
        let o = obj();
        oset(o, cs("b").as_ptr(), int(2));
        oset(o, cs("a").as_ptr(), int(1));

        let plain = dumps_to_string(&l.c, o, 0).expect("C json_dumps returned NULL");
        assert_eq!(plain, r#"{"b": 2, "a": 1}"#, "C insertion-order dump");

        let compact = dumps_to_string(&l.c, o, JSON_COMPACT).unwrap();
        assert_eq!(compact, r#"{"b":2,"a":1}"#, "C compact dump");

        let sorted = dumps_to_string(&l.c, o, JSON_SORT_KEYS).unwrap();
        assert_eq!(sorted, r#"{"a": 1, "b": 2}"#, "C sorted dump");

        let indented = dumps_to_string(&l.c, o, json_indent(2)).unwrap();
        assert_eq!(indented, "{\n  \"b\": 2,\n  \"a\": 1\n}", "C indented dump");

        decref(&l.c, o);
    }
}

/// Same pins, but against the Rust `.so`, so a Rust-side regression that turns
/// everything into NULL is caught even if the comparison logic were broken.
#[test]
fn rust_library_produces_expected_concrete_output() {
    let l = libs();
    unsafe {
        let obj: Symbol<unsafe extern "C" fn() -> *mut json_t> = sym(&l.r, "json_object");
        let int: Symbol<FnInt> = sym(&l.r, "json_integer");
        let oset: Symbol<FnObjSetNew> = sym(&l.r, "json_object_set_new");
        let o = obj();
        oset(o, cs("b").as_ptr(), int(2));
        oset(o, cs("a").as_ptr(), int(1));

        assert_eq!(dumps_to_string(&l.r, o, 0).expect("Rust json_dumps NULL"), r#"{"b": 2, "a": 1}"#);
        assert_eq!(dumps_to_string(&l.r, o, JSON_COMPACT).unwrap(), r#"{"b":2,"a":1}"#);
        assert_eq!(dumps_to_string(&l.r, o, JSON_SORT_KEYS).unwrap(), r#"{"a": 1, "b": 2}"#);
        assert_eq!(
            dumps_to_string(&l.r, o, json_indent(2)).unwrap(),
            "{\n  \"b\": 2,\n  \"a\": 1\n}"
        );
        decref(&l.r, o);
    }
}

/// Loading is deterministic across the seeded hash: object iteration order must
/// be identical in both libraries for a many-key object. If `json_object_seed`
/// were not applied to both, this would fail (or flake run to run).
#[test]
fn hash_seed_makes_iteration_order_deterministic() {
    diff("seeded iteration order", |lib: &Library| unsafe {
        let obj: Symbol<unsafe extern "C" fn() -> *mut json_t> = sym(lib, "json_object");
        let int: Symbol<FnInt> = sym(lib, "json_integer");
        let osetn: Symbol<FnObjSetNNew> = sym(lib, "json_object_setn_new");
        let iter: Symbol<FnIter> = sym(lib, "json_object_iter");
        let next: Symbol<FnIterNext> = sym(lib, "json_object_iter_next");
        let key: Symbol<FnIterKey> = sym(lib, "json_object_iter_key");
        let o = obj();
        for i in 0..40 {
            let k = format!("key{}", i);
            osetn(o, k.as_ptr() as *const c_char, k.len(), int(i));
        }
        let mut order = Vec::new();
        let mut it = iter(o);
        while !it.is_null() {
            order.push(cstr_to_string(key(it)));
            it = next(o, it);
        }
        decref(lib, o);
        order
    });

    // And the observed order must be the real insertion order (a stronger claim
    // than "both agree"), proving the iterator is actually walking data.
    let l = libs();
    unsafe {
        let obj: Symbol<unsafe extern "C" fn() -> *mut json_t> = sym(&l.c, "json_object");
        let int: Symbol<FnInt> = sym(&l.c, "json_integer");
        let osetn: Symbol<FnObjSetNNew> = sym(&l.c, "json_object_setn_new");
        let iter: Symbol<FnIter> = sym(&l.c, "json_object_iter");
        let next: Symbol<FnIterNext> = sym(&l.c, "json_object_iter_next");
        let key: Symbol<FnIterKey> = sym(&l.c, "json_object_iter_key");
        let o = obj();
        for i in 0..12 {
            let k = format!("k{}", i);
            osetn(o, k.as_ptr() as *const c_char, k.len(), int(i));
        }
        let mut order = Vec::new();
        let mut it = iter(o);
        while !it.is_null() {
            order.push(cstr_to_string(key(it)));
            it = next(o, it);
        }
        decref(&l.c, o);
        let expected: Vec<String> = (0..12).map(|i| format!("k{}", i)).collect();
        assert_eq!(order, expected, "jansson preserves insertion order");
    }
}

/// Proves `diff` actually fails when the two sides disagree — otherwise a
/// broken assert would silently green-light everything.
#[test]
#[should_panic(expected = "C/Rust divergence")]
fn diff_detects_a_real_divergence() {
    // Deliberately return something library-dependent: the library's own base
    // address differs between the two handles, so this MUST trip the assert.
    diff("intentional mismatch", |lib: &Library| unsafe {
        let f: Symbol<FnDumps> = sym(lib, "json_dumps");
        // Address of the resolved symbol differs between the two .so files.
        let raw: *mut std::ffi::c_void = f.into_raw().into_raw();
        raw as usize
    });
}
