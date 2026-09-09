//! Phase C — error-path differential tests for the non-OOM rows of ERRORS.md.
//! (The allocator-injection rows live in `c_errors_oom.rs`.)
//!
//! Every test constructs one exact invalid input/condition and asserts C and
//! Rust return the SAME sentinel / `json_error_code` — not merely that both
//! failed.

mod common;
use common::*;
use libloading::Symbol;
use std::os::raw::{c_char, c_double, c_int, c_void};

const R_ANY: usize = JSON_ENCODE_ANY;

/// One representative value of every `json_type`, plus NULL, as the "wrong
/// type" argument for a type-checking entry point.
fn all_types() -> Vec<(&'static str, Node)> {
    vec![
        ("object", Node::Obj(vec![])),
        ("array", Node::Arr(vec![])),
        ("string", Node::Str(b"s".to_vec())),
        ("integer", Node::Int(1)),
        ("real", Node::Real(1.0)),
        ("true", Node::True),
        ("false", Node::False),
        ("null", Node::Null),
    ]
}

// ===========================================================================
// A — value.c objects
// ===========================================================================

/// A3, A5, A14, A16, A26, A27, A29, A34: every object entry point given a
/// non-object (including NULL).
#[test]
fn a_object_entry_points_reject_non_objects() {
    unsafe {
        let p = pair();
        let key = cs("k");
        for (name, node) in all_types() {
            let cj = node.build(&p.c);
            let rj = node.build(&p.r);
            let ctx = |f: &str| format!("{f} on {name}");

            // A3: json_object_size -> 0
            assert_eq!(
                p.c.json_object_size(cj),
                p.r.json_object_size(rj),
                "{}",
                ctx("json_object_size")
            );
            // A4/A5: json_object_get / getn -> NULL
            assert_eq!(
                p.c.json_object_get(cj, key.as_ptr()).is_null(),
                p.r.json_object_get(rj, key.as_ptr()).is_null(),
                "{}",
                ctx("json_object_get")
            );
            assert_eq!(
                p.c.json_object_getn(cj, key.as_ptr(), 1).is_null(),
                p.r.json_object_getn(rj, key.as_ptr(), 1).is_null(),
                "{}",
                ctx("json_object_getn")
            );
            // A9: setters -> -1 (and the value is decref'd)
            let cv = p.c.json_integer(1);
            let rv = p.r.json_integer(1);
            assert_eq!(
                p.c.json_object_setn_new_nocheck(cj, key.as_ptr(), 1, cv),
                p.r.json_object_setn_new_nocheck(rj, key.as_ptr(), 1, rv),
                "{}",
                ctx("json_object_setn_new_nocheck")
            );
            let cv = p.c.json_integer(1);
            let rv = p.r.json_integer(1);
            assert_eq!(
                p.c.json_object_setn_new(cj, key.as_ptr(), 1, cv),
                p.r.json_object_setn_new(rj, key.as_ptr(), 1, rv),
                "{}",
                ctx("json_object_setn_new")
            );
            let cv = p.c.json_integer(1);
            let rv = p.r.json_integer(1);
            assert_eq!(
                p.c.json_object_set_new(cj, key.as_ptr(), cv),
                p.r.json_object_set_new(rj, key.as_ptr(), rv),
                "{}",
                ctx("json_object_set_new")
            );
            let cv = p.c.json_integer(1);
            let rv = p.r.json_integer(1);
            assert_eq!(
                p.c.json_object_set_new_nocheck(cj, key.as_ptr(), cv),
                p.r.json_object_set_new_nocheck(rj, key.as_ptr(), rv),
                "{}",
                ctx("json_object_set_new_nocheck")
            );
            // A13..A15: delete -> -1
            assert_eq!(
                p.c.json_object_del(cj, key.as_ptr()),
                p.r.json_object_del(rj, key.as_ptr()),
                "{}",
                ctx("json_object_del")
            );
            assert_eq!(
                p.c.json_object_deln(cj, key.as_ptr(), 1),
                p.r.json_object_deln(rj, key.as_ptr(), 1),
                "{}",
                ctx("json_object_deln")
            );
            // A16: clear -> -1
            assert_eq!(
                p.c.json_object_clear(cj),
                p.r.json_object_clear(rj),
                "{}",
                ctx("json_object_clear")
            );
            // A26..A29: iterators -> NULL
            assert_eq!(
                p.c.json_object_iter(cj).is_null(),
                p.r.json_object_iter(rj).is_null(),
                "{}",
                ctx("json_object_iter")
            );
            assert_eq!(
                p.c.json_object_iter_at(cj, key.as_ptr()).is_null(),
                p.r.json_object_iter_at(rj, key.as_ptr()).is_null(),
                "{}",
                ctx("json_object_iter_at")
            );
            // A29: iter_next on a non-object, with a NULL iterator (a bogus
            // non-NULL iterator would be UB in the C, which dereferences it).
            assert_eq!(
                p.c.json_object_iter_next(cj, std::ptr::null_mut()).is_null(),
                p.r.json_object_iter_next(rj, std::ptr::null_mut()).is_null(),
                "{}",
                ctx("json_object_iter_next(non-object)")
            );
            p.c.json_decref(cj);
            p.r.json_decref(rj);
        }

        // NULL object pointer for every one of the above.
        let nil: json_ptr = std::ptr::null_mut();
        assert_eq!(p.c.json_object_size(nil), p.r.json_object_size(nil));
        assert_eq!(
            p.c.json_object_get(nil, key.as_ptr()).is_null(),
            p.r.json_object_get(nil, key.as_ptr()).is_null()
        );
        assert_eq!(
            p.c.json_object_getn(nil, key.as_ptr(), 1).is_null(),
            p.r.json_object_getn(nil, key.as_ptr(), 1).is_null()
        );
        assert_eq!(
            p.c.json_object_del(nil, key.as_ptr()),
            p.r.json_object_del(nil, key.as_ptr())
        );
        assert_eq!(
            p.c.json_object_deln(nil, key.as_ptr(), 1),
            p.r.json_object_deln(nil, key.as_ptr(), 1)
        );
        assert_eq!(p.c.json_object_clear(nil), p.r.json_object_clear(nil));
        assert_eq!(
            p.c.json_object_iter(nil).is_null(),
            p.r.json_object_iter(nil).is_null()
        );
        assert_eq!(
            p.c.json_object_iter_at(nil, key.as_ptr()).is_null(),
            p.r.json_object_iter_at(nil, key.as_ptr()).is_null()
        );
        let cv = p.c.json_integer(1);
        let rv = p.r.json_integer(1);
        assert_eq!(
            p.c.json_object_setn_new_nocheck(nil, key.as_ptr(), 1, cv),
            p.r.json_object_setn_new_nocheck(nil, key.as_ptr(), 1, rv)
        );
    }
}

/// A4, A7, A11, A12, A13, A14, A27, A35: NULL key, and an invalid-UTF-8 key for
/// the *checked* setter only.
#[test]
fn a_object_null_and_invalid_keys() {
    unsafe {
        let p = pair();
        let co = p.c.json_object();
        let ro = p.r.json_object();
        let nk: *const c_char = std::ptr::null();

        // A4/A5: get with NULL key
        assert_eq!(
            p.c.json_object_get(co, nk).is_null(),
            p.r.json_object_get(ro, nk).is_null(),
            "json_object_get(NULL key)"
        );
        assert_eq!(
            p.c.json_object_getn(co, nk, 0).is_null(),
            p.r.json_object_getn(ro, nk, 0).is_null(),
            "json_object_getn(NULL key)"
        );
        // A7/A9/A11/A12: setters with NULL key -> -1, value decref'd
        for variant in 0..4 {
            let cv = p.c.json_integer(1);
            let rv = p.r.json_integer(1);
            let (cr, rr) = match variant {
                0 => (
                    p.c.json_object_set_new_nocheck(co, nk, cv),
                    p.r.json_object_set_new_nocheck(ro, nk, rv),
                ),
                1 => (
                    p.c.json_object_setn_new_nocheck(co, nk, 0, cv),
                    p.r.json_object_setn_new_nocheck(ro, nk, 0, rv),
                ),
                2 => (
                    p.c.json_object_set_new(co, nk, cv),
                    p.r.json_object_set_new(ro, nk, rv),
                ),
                _ => (
                    p.c.json_object_setn_new(co, nk, 0, cv),
                    p.r.json_object_setn_new(ro, nk, 0, rv),
                ),
            };
            assert_eq!(cr, rr, "setter variant {variant} with NULL key");
        }
        // A13/A14: delete with NULL key
        assert_eq!(
            p.c.json_object_del(co, nk),
            p.r.json_object_del(ro, nk),
            "json_object_del(NULL key)"
        );
        assert_eq!(
            p.c.json_object_deln(co, nk, 0),
            p.r.json_object_deln(ro, nk, 0),
            "json_object_deln(NULL key)"
        );
        // A27: iter_at with NULL key
        assert_eq!(
            p.c.json_object_iter_at(co, nk).is_null(),
            p.r.json_object_iter_at(ro, nk).is_null(),
            "json_object_iter_at(NULL key)"
        );
        // A35: key_to_iter(NULL)
        assert_eq!(
            p.c.json_object_key_to_iter(nk).is_null(),
            p.r.json_object_key_to_iter(nk).is_null(),
            "json_object_key_to_iter(NULL)"
        );

        // A12: invalid-UTF-8 key rejected by the CHECKED setter, accepted by nocheck
        for bad in [
            vec![0x80u8],
            vec![0xC0, 0x80],
            vec![0xFF],
            vec![0xED, 0xA0, 0x80],
            vec![0xC3],
            vec![0xF5, 0x80, 0x80, 0x80],
        ] {
            let cv = p.c.json_integer(1);
            let rv = p.r.json_integer(1);
            assert_eq!(
                p.c.json_object_setn_new(co, bad.as_ptr() as *const c_char, bad.len(), cv),
                p.r.json_object_setn_new(ro, bad.as_ptr() as *const c_char, bad.len(), rv),
                "json_object_setn_new(bad UTF-8 key {bad:02x?})"
            );
            let z = raw_z(&bad);
            let cv = p.c.json_integer(1);
            let rv = p.r.json_integer(1);
            assert_eq!(
                p.c.json_object_set_new(co, z.as_ptr() as *const c_char, cv),
                p.r.json_object_set_new(ro, z.as_ptr() as *const c_char, rv),
                "json_object_set_new(bad UTF-8 key {bad:02x?})"
            );
        }
        assert_eq!(p.c.dumps(co, R_ANY), p.r.dumps(ro, R_ANY), "object contents");
        p.c.json_decref(co);
        p.r.json_decref(ro);
    }
}

/// A8, A34: NULL value; A9: self-insertion (`json == value`).
#[test]
fn a_object_null_value_and_self_insertion() {
    unsafe {
        let p = pair();
        let key = cs("k");
        let co = p.c.json_object();
        let ro = p.r.json_object();
        let nilv: json_ptr = std::ptr::null_mut();

        // A8: NULL value -> -1
        assert_eq!(
            p.c.json_object_setn_new_nocheck(co, key.as_ptr(), 1, nilv),
            p.r.json_object_setn_new_nocheck(ro, key.as_ptr(), 1, nilv),
            "setn_new_nocheck(NULL value)"
        );
        assert_eq!(
            p.c.json_object_setn_new(co, key.as_ptr(), 1, nilv),
            p.r.json_object_setn_new(ro, key.as_ptr(), 1, nilv),
            "setn_new(NULL value)"
        );
        assert_eq!(
            p.c.json_object_set_new(co, key.as_ptr(), nilv),
            p.r.json_object_set_new(ro, key.as_ptr(), nilv),
            "set_new(NULL value)"
        );
        // A9: self-insertion -> -1
        assert_eq!(
            p.c.json_object_setn_new_nocheck(co, key.as_ptr(), 1, co),
            p.r.json_object_setn_new_nocheck(ro, key.as_ptr(), 1, ro),
            "self-insertion"
        );
        assert_eq!(p.c.dumps(co, R_ANY), p.r.dumps(ro, R_ANY));

        // A34: iter_set_new with NULL iter / NULL value / non-object
        p.c.json_object_setn_new_nocheck(co, key.as_ptr(), 1, p.c.json_integer(1));
        p.r.json_object_setn_new_nocheck(ro, key.as_ptr(), 1, p.r.json_integer(1));
        let ci = p.c.json_object_iter(co);
        let ri = p.r.json_object_iter(ro);
        let cv = p.c.json_integer(2);
        let rv = p.r.json_integer(2);
        assert_eq!(
            p.c.json_object_iter_set_new(co, std::ptr::null_mut(), cv),
            p.r.json_object_iter_set_new(ro, std::ptr::null_mut(), rv),
            "iter_set_new(NULL iter)"
        );
        assert_eq!(
            p.c.json_object_iter_set_new(co, ci, nilv),
            p.r.json_object_iter_set_new(ro, ri, nilv),
            "iter_set_new(NULL value)"
        );
        let ca = p.c.json_array();
        let ra = p.r.json_array();
        let cv = p.c.json_integer(2);
        let rv = p.r.json_integer(2);
        assert_eq!(
            p.c.json_object_iter_set_new(ca, ci, cv),
            p.r.json_object_iter_set_new(ra, ri, rv),
            "iter_set_new(non-object)"
        );
        p.c.json_decref(ca);
        p.r.json_decref(ra);
        p.c.json_decref(co);
        p.r.json_decref(ro);
    }
}

/// A6, A15, A28, A30, A31, A32, A33: lookups that miss and iterators at the end
/// / on NULL.
#[test]
fn a_object_misses_and_null_iterators() {
    unsafe {
        let p = pair();
        let co = p.c.json_object();
        let ro = p.r.json_object();
        let k = cs("present");
        let miss = cs("absent");
        p.c.json_object_set_new(co, k.as_ptr(), p.c.json_integer(1));
        p.r.json_object_set_new(ro, k.as_ptr(), p.r.json_integer(1));

        // A6: getn miss
        assert_eq!(
            p.c.json_object_getn(co, miss.as_ptr(), 6).is_null(),
            p.r.json_object_getn(ro, miss.as_ptr(), 6).is_null(),
            "getn miss"
        );
        // A15: deln miss -> -1
        assert_eq!(
            p.c.json_object_deln(co, miss.as_ptr(), 6),
            p.r.json_object_deln(ro, miss.as_ptr(), 6),
            "deln miss"
        );
        assert_eq!(
            p.c.json_object_del(co, miss.as_ptr()),
            p.r.json_object_del(ro, miss.as_ptr()),
            "del miss"
        );
        // A28: iter_at miss
        assert_eq!(
            p.c.json_object_iter_at(co, miss.as_ptr()).is_null(),
            p.r.json_object_iter_at(ro, miss.as_ptr()).is_null(),
            "iter_at miss"
        );
        // A30: iter_next at the end of the ordered list
        let ci = p.c.json_object_iter(co);
        let ri = p.r.json_object_iter(ro);
        let cn = p.c.json_object_iter_next(co, ci);
        let rn = p.r.json_object_iter_next(ro, ri);
        assert_eq!(cn.is_null(), rn.is_null(), "iter_next at end");
        // A29: iter_next with NULL iter
        assert_eq!(
            p.c.json_object_iter_next(co, std::ptr::null_mut()).is_null(),
            p.r.json_object_iter_next(ro, std::ptr::null_mut()).is_null(),
            "iter_next(NULL iter)"
        );
        // A31/A32/A33: accessors with NULL iter
        assert_eq!(
            p.c.json_object_iter_key(std::ptr::null_mut()).is_null(),
            p.r.json_object_iter_key(std::ptr::null_mut()).is_null(),
            "iter_key(NULL)"
        );
        assert_eq!(
            p.c.json_object_iter_key_len(std::ptr::null_mut()),
            p.r.json_object_iter_key_len(std::ptr::null_mut()),
            "iter_key_len(NULL)"
        );
        assert_eq!(
            p.c.json_object_iter_value(std::ptr::null_mut()).is_null(),
            p.r.json_object_iter_value(std::ptr::null_mut()).is_null(),
            "iter_value(NULL)"
        );
        // A14: json_object_iter on an EMPTY object -> NULL (H14 too)
        let ce = p.c.json_object();
        let re = p.r.json_object();
        assert_eq!(
            p.c.json_object_iter(ce).is_null(),
            p.r.json_object_iter(re).is_null(),
            "iter on empty object"
        );
        p.c.json_decref(ce);
        p.r.json_decref(re);
        p.c.json_decref(co);
        p.r.json_decref(ro);
    }
}

/// A17, A19, A20, A21: the four update variants with a non-object target or
/// source (including NULL).
#[test]
fn a_object_update_type_errors() {
    unsafe {
        let p = pair();
        for (an, a) in all_types() {
            for (bn, b) in all_types() {
                if an == "object" && bn == "object" {
                    continue;
                }
                let ca = a.build(&p.c);
                let ra = a.build(&p.r);
                let cb = b.build(&p.c);
                let rb = b.build(&p.r);
                for which in 0..4 {
                    let (cr, rr) = match which {
                        0 => (
                            p.c.json_object_update(ca, cb),
                            p.r.json_object_update(ra, rb),
                        ),
                        1 => (
                            p.c.json_object_update_existing(ca, cb),
                            p.r.json_object_update_existing(ra, rb),
                        ),
                        2 => (
                            p.c.json_object_update_missing(ca, cb),
                            p.r.json_object_update_missing(ra, rb),
                        ),
                        _ => (
                            p.c.json_object_update_recursive(ca, cb),
                            p.r.json_object_update_recursive(ra, rb),
                        ),
                    };
                    assert_eq!(cr, rr, "update variant {which} on ({an}, {bn})");
                }
                p.c.json_decref(ca);
                p.r.json_decref(ra);
                p.c.json_decref(cb);
                p.r.json_decref(rb);
            }
        }
        // NULL operands
        let co = p.c.json_object();
        let ro = p.r.json_object();
        let nil: json_ptr = std::ptr::null_mut();
        for which in 0..4 {
            for (a, b, ra_, rb_) in [(co, nil, ro, nil), (nil, co, nil, ro), (nil, nil, nil, nil)] {
                let (cr, rr) = match which {
                    0 => (p.c.json_object_update(a, b), p.r.json_object_update(ra_, rb_)),
                    1 => (
                        p.c.json_object_update_existing(a, b),
                        p.r.json_object_update_existing(ra_, rb_),
                    ),
                    2 => (
                        p.c.json_object_update_missing(a, b),
                        p.r.json_object_update_missing(ra_, rb_),
                    ),
                    _ => (
                        p.c.json_object_update_recursive(a, b),
                        p.r.json_object_update_recursive(ra_, rb_),
                    ),
                };
                assert_eq!(cr, rr, "update variant {which} with NULL operand");
            }
        }
        p.c.json_decref(co);
        p.r.json_decref(ro);
    }
}

/// A22, A39, A42, B25, D16, D23: cyclic structures.
///
/// A *direct* self-reference cannot be built through the public API — the C
/// rejects `json == value` in `json_object_setn_new_nocheck`,
/// `json_array_set_new`, `json_array_append_new` and `json_array_insert_new`
/// (rows A9 / B7 / B11 / B14). So the cycles below are all INDIRECT, which is
/// exactly what `jsonp_loop_check` exists to catch.
/// A callback that accepts every chunk, for use where the C would dereference a
/// NULL callback.
unsafe extern "C" fn sink_cb(_buf: *const c_char, _size: usize, _data: *mut c_void) -> c_int {
    0
}

#[test]
fn a_cycles_are_detected() {
    unsafe {
        let p = pair();
        let devnull = {
            use std::os::unix::io::IntoRawFd;
            std::fs::OpenOptions::new()
                .write(true)
                .open("/dev/null")
                .unwrap()
                .into_raw_fd()
        };

        // First pin the self-insertion guard itself (A9 / B7 / B11 / B14).
        let co = p.c.json_object();
        let ro = p.r.json_object();
        let k = cs("self");
        assert_eq!(
            p.c.json_object_set_new(co, k.as_ptr(), p.c.json_incref(co)),
            p.r.json_object_set_new(ro, k.as_ptr(), p.r.json_incref(ro)),
            "direct object self-insertion must be rejected"
        );
        assert_eq!(p.c.json_object_size(co), 0, "C rejected the self-insert");
        assert_eq!(p.c.dumps(co, R_ANY), p.r.dumps(ro, R_ANY));
        let ca = p.c.json_array();
        let ra = p.r.json_array();
        assert_eq!(
            p.c.json_array_append_new(ca, p.c.json_incref(ca)),
            p.r.json_array_append_new(ra, p.r.json_incref(ra)),
            "direct array self-append must be rejected"
        );
        assert_eq!(p.c.json_array_size(ca), 0);
        p.c.json_decref(co);
        p.r.json_decref(ro);
        p.c.json_decref(ca);
        p.r.json_decref(ra);

        // Helper: build one of several INDIRECT cycle shapes in a library and
        // return the root.
        unsafe fn build_cycle(l: &Lib, shape: usize) -> json_ptr {
            let ka = cs("a");
            let kb = cs("b");
            match shape {
                // obj -> obj -> obj (2-node cycle)
                0 => {
                    let x = l.json_object();
                    let y = l.json_object();
                    l.json_object_set_new(x, ka.as_ptr(), l.json_incref(y));
                    l.json_object_set_new(y, kb.as_ptr(), l.json_incref(x));
                    l.json_decref(y);
                    x
                }
                // arr -> arr -> arr
                1 => {
                    let x = l.json_array();
                    let y = l.json_array();
                    l.json_array_append_new(x, l.json_incref(y));
                    l.json_array_append_new(y, l.json_incref(x));
                    l.json_decref(y);
                    x
                }
                // obj -> arr -> obj
                2 => {
                    let x = l.json_object();
                    let y = l.json_array();
                    l.json_object_set_new(x, ka.as_ptr(), l.json_incref(y));
                    l.json_array_append_new(y, l.json_incref(x));
                    l.json_decref(y);
                    x
                }
                // arr -> obj -> arr
                3 => {
                    let x = l.json_array();
                    let y = l.json_object();
                    l.json_array_append_new(x, l.json_incref(y));
                    l.json_object_set_new(y, ka.as_ptr(), l.json_incref(x));
                    l.json_decref(y);
                    x
                }
                // 3-node cycle with extra non-cyclic siblings
                4 => {
                    let x = l.json_object();
                    let y = l.json_object();
                    let z = l.json_array();
                    l.json_object_set_new(x, ka.as_ptr(), l.json_integer(1));
                    l.json_object_set_new(x, kb.as_ptr(), l.json_incref(y));
                    l.json_object_set_new(y, ka.as_ptr(), l.json_incref(z));
                    l.json_array_append_new(z, l.json_integer(2));
                    l.json_array_append_new(z, l.json_incref(x));
                    l.json_decref(y);
                    l.json_decref(z);
                    x
                }
                // the cycle is buried one level below the root
                _ => {
                    let root = l.json_object();
                    let x = l.json_object();
                    let y = l.json_object();
                    l.json_object_set_new(x, ka.as_ptr(), l.json_incref(y));
                    l.json_object_set_new(y, kb.as_ptr(), l.json_incref(x));
                    l.json_object_set_new(root, ka.as_ptr(), l.json_incref(x));
                    l.json_decref(x);
                    l.json_decref(y);
                    root
                }
            }
        }

        for shape in 0..6usize {
            let cx = build_cycle(&p.c, shape);
            let rx = build_cycle(&p.r, shape);

            // A39 / B25: deep copy must fail
            let ccp = p.c.json_deep_copy(cx);
            let rcp = p.r.json_deep_copy(rx);
            assert_eq!(
                ccp.is_null(),
                rcp.is_null(),
                "json_deep_copy(cycle shape {shape})"
            );
            assert!(ccp.is_null(), "C must reject cycle shape {shape} in deep_copy");

            // D16 / D23: every dump entry point must fail
            for f in [
                0usize,
                JSON_COMPACT,
                JSON_INDENT(2),
                JSON_SORT_KEYS,
                JSON_ENSURE_ASCII,
                R_ANY,
                JSON_EMBED,
            ] {
                assert_eq!(
                    p.c.dumps(cx, f),
                    p.r.dumps(rx, f),
                    "json_dumps(cycle shape {shape}, 0x{f:x})"
                );
                assert!(
                    p.c.dumps(cx, f).is_none(),
                    "C must reject cycle shape {shape} at flags 0x{f:x}"
                );
                assert_eq!(
                    p.c.json_dumpb(cx, std::ptr::null_mut(), 0, f),
                    p.r.json_dumpb(rx, std::ptr::null_mut(), 0, f),
                    "json_dumpb(cycle shape {shape}, 0x{f:x})"
                );
                // Dump to /dev/null rather than stdout, and use a REAL
                // callback: `json_dump_callback` with a NULL callback is UB in
                // the C once the root passes the JSON_ENCODE_ANY gate
                // (ERRORS D46), so it is not a testable rejection.
                assert_eq!(
                    p.c.json_dumpfd(cx, devnull, f),
                    p.r.json_dumpfd(rx, devnull, f),
                    "json_dumpfd(cycle shape {shape}, 0x{f:x})"
                );
                assert_eq!(
                    p.c.json_dump_callback(cx, Some(sink_cb), std::ptr::null_mut(), f),
                    p.r.json_dump_callback(rx, Some(sink_cb), std::ptr::null_mut(), f),
                    "json_dump_callback(cycle shape {shape}, 0x{f:x})"
                );
            }

            // A22: update_recursive with a cyclic source
            let co2 = p.c.json_object();
            let ro2 = p.r.json_object();
            if typeof_json(cx) == JSON_OBJECT {
                assert_eq!(
                    p.c.json_object_update_recursive(co2, cx),
                    p.r.json_object_update_recursive(ro2, rx),
                    "update_recursive(cyclic other, shape {shape})"
                );
            }
            p.c.json_decref(co2);
            p.r.json_decref(ro2);

            // json_equal on two structurally identical cycles must not be
            // compared (the C would recurse forever); only the copy/dump paths
            // carry a loop check. So stop here and leak the cycle deliberately
            // — breaking it would require reaching inside the graph.
            let _ = (cx, rx);
        }

        // A42: jsonp_loop_check in isolation — 0 the first time, -1 after.
        let mut cslab = [0u8; 256];
        let mut rslab = [0u8; 256];
        let ch = cslab.as_mut_ptr() as *mut c_void;
        let rh = rslab.as_mut_ptr() as *mut c_void;
        assert_eq!(p.c.hashtable_init(ch), p.r.hashtable_init(rh));
        let cobj = p.c.json_object();
        let robj = p.r.json_object();
        let mut ck = [0u8; 64];
        let mut rk = [0u8; 64];
        let mut cl_ = 0usize;
        let mut rl_ = 0usize;
        for round in 0..3 {
            let cr =
                p.c.jsonp_loop_check(ch, cobj, ck.as_mut_ptr() as *mut c_char, ck.len(), &mut cl_);
            let rr =
                p.r.jsonp_loop_check(rh, robj, rk.as_mut_ptr() as *mut c_char, rk.len(), &mut rl_);
            assert_eq!(cr, rr, "jsonp_loop_check round {round}");
            assert_eq!(cl_, rl_, "jsonp_loop_check key_len round {round}");
            assert_eq!(
                cr,
                if round == 0 { 0 } else { -1 },
                "C: jsonp_loop_check round {round}"
            );
        }
        p.c.hashtable_close(ch);
        p.r.hashtable_close(rh);
        p.c.json_decref(cobj);
        p.r.json_decref(robj);
    }
}

// ===========================================================================
// B — value.c arrays
// ===========================================================================

/// B3, B4, B7, B11, B14, B17, B19, B20: array entry points with a non-array.
#[test]
fn b_array_entry_points_reject_non_arrays() {
    unsafe {
        let p = pair();
        for (name, node) in all_types() {
            let cj = node.build(&p.c);
            let rj = node.build(&p.r);
            assert_eq!(
                p.c.json_array_size(cj),
                p.r.json_array_size(rj),
                "json_array_size on {name}"
            );
            for i in [0usize, 1, usize::MAX] {
                assert_eq!(
                    p.c.json_array_get(cj, i).is_null(),
                    p.r.json_array_get(rj, i).is_null(),
                    "json_array_get({i}) on {name}"
                );
                assert_eq!(
                    p.c.json_array_remove(cj, i),
                    p.r.json_array_remove(rj, i),
                    "json_array_remove({i}) on {name}"
                );
                let cv = p.c.json_integer(1);
                let rv = p.r.json_integer(1);
                assert_eq!(
                    p.c.json_array_set_new(cj, i, cv),
                    p.r.json_array_set_new(rj, i, rv),
                    "json_array_set_new({i}) on {name}"
                );
                let cv = p.c.json_integer(1);
                let rv = p.r.json_integer(1);
                assert_eq!(
                    p.c.json_array_insert_new(cj, i, cv),
                    p.r.json_array_insert_new(rj, i, rv),
                    "json_array_insert_new({i}) on {name}"
                );
            }
            let cv = p.c.json_integer(1);
            let rv = p.r.json_integer(1);
            assert_eq!(
                p.c.json_array_append_new(cj, cv),
                p.r.json_array_append_new(rj, rv),
                "json_array_append_new on {name}"
            );
            assert_eq!(
                p.c.json_array_clear(cj),
                p.r.json_array_clear(rj),
                "json_array_clear on {name}"
            );
            let cok = p.c.json_array();
            let rok = p.r.json_array();
            assert_eq!(
                p.c.json_array_extend(cj, cok),
                p.r.json_array_extend(rj, rok),
                "json_array_extend(non-array target {name})"
            );
            assert_eq!(
                p.c.json_array_extend(cok, cj),
                p.r.json_array_extend(rok, rj),
                "json_array_extend(non-array source {name})"
            );
            p.c.json_decref(cok);
            p.r.json_decref(rok);
            p.c.json_decref(cj);
            p.r.json_decref(rj);
        }
        // NULL
        let nil: json_ptr = std::ptr::null_mut();
        assert_eq!(p.c.json_array_size(nil), p.r.json_array_size(nil));
        assert_eq!(
            p.c.json_array_get(nil, 0).is_null(),
            p.r.json_array_get(nil, 0).is_null()
        );
        assert_eq!(p.c.json_array_remove(nil, 0), p.r.json_array_remove(nil, 0));
        assert_eq!(p.c.json_array_clear(nil), p.r.json_array_clear(nil));
        assert_eq!(
            p.c.json_array_extend(nil, nil),
            p.r.json_array_extend(nil, nil)
        );
    }
}

/// B5, B8, B15, B18, Q4: index out of range at every boundary.
#[test]
fn b_array_index_out_of_range() {
    unsafe {
        let p = pair();
        for n in [0usize, 1, 2, 8, 9] {
            let ca = p.c.json_array();
            let ra = p.r.json_array();
            for i in 0..n {
                p.c.json_array_append_new(ca, p.c.json_integer(i as i64));
                p.r.json_array_append_new(ra, p.r.json_integer(i as i64));
            }
            for idx in [n, n + 1, n + 2, usize::MAX, usize::MAX - 1, usize::MAX / 2] {
                // B5: get
                assert_eq!(
                    p.c.json_array_get(ca, idx).is_null(),
                    p.r.json_array_get(ra, idx).is_null(),
                    "get({idx}) on size {n}"
                );
                // B8: set_new
                let cv = p.c.json_integer(9);
                let rv = p.r.json_integer(9);
                assert_eq!(
                    p.c.json_array_set_new(ca, idx, cv),
                    p.r.json_array_set_new(ra, idx, rv),
                    "set_new({idx}) on size {n}"
                );
                // B18: remove
                assert_eq!(
                    p.c.json_array_remove(ca, idx),
                    p.r.json_array_remove(ra, idx),
                    "remove({idx}) on size {n}"
                );
                // B15: insert_new — `index > entries` is rejected, `== entries` is OK
                let cv = p.c.json_integer(9);
                let rv = p.r.json_integer(9);
                assert_eq!(
                    p.c.json_array_insert_new(ca, idx, cv),
                    p.r.json_array_insert_new(ra, idx, rv),
                    "insert_new({idx}) on size {n}"
                );
                assert_eq!(
                    p.c.json_array_size(ca),
                    p.r.json_array_size(ra),
                    "size after insert({idx}) on {n}"
                );
            }
            assert_eq!(p.c.dumps(ca, R_ANY), p.r.dumps(ra, R_ANY));
            p.c.json_decref(ca);
            p.r.json_decref(ra);
        }
    }
}

/// B6, B10, B13: NULL value; B7, B11, B14: self-insertion.
#[test]
fn b_array_null_value_and_self_insertion() {
    unsafe {
        let p = pair();
        let ca = p.c.json_array();
        let ra = p.r.json_array();
        p.c.json_array_append_new(ca, p.c.json_integer(0));
        p.r.json_array_append_new(ra, p.r.json_integer(0));
        let nil: json_ptr = std::ptr::null_mut();

        assert_eq!(
            p.c.json_array_append_new(ca, nil),
            p.r.json_array_append_new(ra, nil),
            "append_new(NULL)"
        );
        assert_eq!(
            p.c.json_array_set_new(ca, 0, nil),
            p.r.json_array_set_new(ra, 0, nil),
            "set_new(NULL)"
        );
        assert_eq!(
            p.c.json_array_insert_new(ca, 0, nil),
            p.r.json_array_insert_new(ra, 0, nil),
            "insert_new(NULL)"
        );
        // self-insertion
        assert_eq!(
            p.c.json_array_append_new(ca, ca),
            p.r.json_array_append_new(ra, ra),
            "append_new(self)"
        );
        assert_eq!(
            p.c.json_array_set_new(ca, 0, ca),
            p.r.json_array_set_new(ra, 0, ra),
            "set_new(self)"
        );
        assert_eq!(
            p.c.json_array_insert_new(ca, 0, ca),
            p.r.json_array_insert_new(ra, 0, ra),
            "insert_new(self)"
        );
        assert_eq!(p.c.dumps(ca, R_ANY), p.r.dumps(ra, R_ANY));
        p.c.json_decref(ca);
        p.r.json_decref(ra);
    }
}

// ===========================================================================
// C — value.c strings, numbers, misc
// ===========================================================================

/// C1, C4, C5, C6, C7, C8, C11, C12, C14, C15: NULL and invalid-UTF-8 string
/// inputs; C9, C10: accessors on non-strings.
#[test]
fn c_string_constructors_and_setters() {
    unsafe {
        let p = pair();
        let nilp: *const c_char = std::ptr::null();

        // C1/C4/C7: NULL value to each constructor
        assert_eq!(
            p.c.json_string(nilp).is_null(),
            p.r.json_string(nilp).is_null(),
            "json_string(NULL)"
        );
        assert_eq!(
            p.c.json_stringn(nilp, 0).is_null(),
            p.r.json_stringn(nilp, 0).is_null(),
            "json_stringn(NULL, 0)"
        );
        assert_eq!(
            p.c.json_stringn(nilp, 5).is_null(),
            p.r.json_stringn(nilp, 5).is_null(),
            "json_stringn(NULL, 5)"
        );
        assert_eq!(
            p.c.json_string_nocheck(nilp).is_null(),
            p.r.json_string_nocheck(nilp).is_null(),
            "json_string_nocheck(NULL)"
        );
        assert_eq!(
            p.c.json_stringn_nocheck(nilp, 0).is_null(),
            p.r.json_stringn_nocheck(nilp, 0).is_null(),
            "json_stringn_nocheck(NULL, 0)"
        );
        // C6: jsonp_stringn_nocheck_own(NULL)
        assert_eq!(
            p.c.jsonp_stringn_nocheck_own(std::ptr::null_mut(), 0).is_null(),
            p.r.jsonp_stringn_nocheck_own(std::ptr::null_mut(), 0).is_null(),
            "jsonp_stringn_nocheck_own(NULL)"
        );

        // C8: invalid UTF-8 rejected by the checked constructors
        for bad in [
            vec![0x80u8],
            vec![0xBF],
            vec![0xC0, 0x80],
            vec![0xC1, 0xBF],
            vec![0xFF],
            vec![0xFE],
            vec![0xE0, 0x80, 0x80],
            vec![0xED, 0xA0, 0x80],
            vec![0xF4, 0x90, 0x80, 0x80],
            vec![0xF5, 0x80, 0x80, 0x80],
            vec![0xC3],
            vec![0xE2, 0x82],
            b"ok\xFFbad".to_vec(),
        ] {
            assert_eq!(
                p.c.json_stringn(bad.as_ptr() as *const c_char, bad.len()).is_null(),
                p.r.json_stringn(bad.as_ptr() as *const c_char, bad.len()).is_null(),
                "json_stringn({bad:02x?})"
            );
            let z = raw_z(&bad);
            assert_eq!(
                p.c.json_string(z.as_ptr() as *const c_char).is_null(),
                p.r.json_string(z.as_ptr() as *const c_char).is_null(),
                "json_string({bad:02x?})"
            );
            // The nocheck variants must ACCEPT them.
            let cj = p.c.json_stringn_nocheck(bad.as_ptr() as *const c_char, bad.len());
            let rj = p.r.json_stringn_nocheck(bad.as_ptr() as *const c_char, bad.len());
            assert_eq!(
                cj.is_null(),
                rj.is_null(),
                "json_stringn_nocheck({bad:02x?}) must succeed"
            );
            // D9: dumping such a string must then fail identically.
            for f in [R_ANY, R_ANY | JSON_ENSURE_ASCII, R_ANY | JSON_COMPACT] {
                assert_eq!(
                    p.c.dumps(cj, f),
                    p.r.dumps(rj, f),
                    "dumps(invalid-UTF-8 string {bad:02x?}, 0x{f:x})"
                );
            }
            p.c.json_decref(cj);
            p.r.json_decref(rj);
        }

        // C11/C12/C14/C15: setters with NULL value and with invalid UTF-8
        let cs_ = p.c.json_string(cs("ok").as_ptr());
        let rs_ = p.r.json_string(cs("ok").as_ptr());
        assert_eq!(
            p.c.json_string_set(cs_, nilp),
            p.r.json_string_set(rs_, nilp),
            "json_string_set(NULL)"
        );
        assert_eq!(
            p.c.json_string_setn(cs_, nilp, 0),
            p.r.json_string_setn(rs_, nilp, 0),
            "json_string_setn(NULL, 0)"
        );
        assert_eq!(
            p.c.json_string_set_nocheck(cs_, nilp),
            p.r.json_string_set_nocheck(rs_, nilp),
            "json_string_set_nocheck(NULL)"
        );
        assert_eq!(
            p.c.json_string_setn_nocheck(cs_, nilp, 0),
            p.r.json_string_setn_nocheck(rs_, nilp, 0),
            "json_string_setn_nocheck(NULL, 0)"
        );
        for bad in [vec![0x80u8], vec![0xFF], vec![0xC3]] {
            assert_eq!(
                p.c.json_string_setn(cs_, bad.as_ptr() as *const c_char, bad.len()),
                p.r.json_string_setn(rs_, bad.as_ptr() as *const c_char, bad.len()),
                "json_string_setn({bad:02x?})"
            );
            let z = raw_z(&bad);
            assert_eq!(
                p.c.json_string_set(cs_, z.as_ptr() as *const c_char),
                p.r.json_string_set(rs_, z.as_ptr() as *const c_char),
                "json_string_set({bad:02x?})"
            );
        }
        assert_eq!(p.c.dumps(cs_, R_ANY), p.r.dumps(rs_, R_ANY));
        p.c.json_decref(cs_);
        p.r.json_decref(rs_);

        // C9/C10/C12: accessors and setters on non-strings (and NULL)
        for (name, node) in all_types() {
            let cj = node.build(&p.c);
            let rj = node.build(&p.r);
            assert_eq!(
                p.c.json_string_value(cj).is_null(),
                p.r.json_string_value(rj).is_null(),
                "json_string_value on {name}"
            );
            assert_eq!(
                p.c.json_string_length(cj),
                p.r.json_string_length(rj),
                "json_string_length on {name}"
            );
            let v = cs("x");
            assert_eq!(
                p.c.json_string_set(cj, v.as_ptr()),
                p.r.json_string_set(rj, v.as_ptr()),
                "json_string_set on {name}"
            );
            assert_eq!(
                p.c.json_string_setn(cj, v.as_ptr(), 1),
                p.r.json_string_setn(rj, v.as_ptr(), 1),
                "json_string_setn on {name}"
            );
            assert_eq!(
                p.c.json_string_set_nocheck(cj, v.as_ptr()),
                p.r.json_string_set_nocheck(rj, v.as_ptr()),
                "json_string_set_nocheck on {name}"
            );
            assert_eq!(
                p.c.json_string_setn_nocheck(cj, v.as_ptr(), 1),
                p.r.json_string_setn_nocheck(rj, v.as_ptr(), 1),
                "json_string_setn_nocheck on {name}"
            );
            p.c.json_decref(cj);
            p.r.json_decref(rj);
        }
        let nil: json_ptr = std::ptr::null_mut();
        assert_eq!(
            p.c.json_string_value(nil).is_null(),
            p.r.json_string_value(nil).is_null()
        );
        assert_eq!(p.c.json_string_length(nil), p.r.json_string_length(nil));
        let v = cs("x");
        assert_eq!(
            p.c.json_string_set(nil, v.as_ptr()),
            p.r.json_string_set(nil, v.as_ptr())
        );
    }
}

/// C16, C18, C19, C20: json_sprintf / json_vsprintf failure paths.
#[test]
fn c_sprintf_failures() {
    unsafe {
        let p = pair();
        let csp = p.c.json_sprintf_sym();
        let rsp = p.r.json_sprintf_sym();
        // C18: the formatted result is not valid UTF-8 -> NULL
        for bad in [
            vec![0x80u8, 0],
            vec![0xFFu8, 0],
            vec![0xC3u8, 0],
            vec![0xEDu8, 0xA0, 0x80, 0],
        ] {
            let f = cs("%s");
            let cj = csp(f.as_ptr(), bad.as_ptr() as *const c_char);
            let rj = rsp(f.as_ptr(), bad.as_ptr() as *const c_char);
            assert_eq!(
                cj.is_null(),
                rj.is_null(),
                "json_sprintf(%s, {bad:02x?}) NULL-ness"
            );
            assert!(cj.is_null(), "C must reject invalid UTF-8 output");
            p.c.json_decref(cj);
            p.r.json_decref(rj);
        }
        // C20: an EMPTY result is not an error — it yields the "" string.
        let f = cs("");
        let cj = csp(f.as_ptr());
        let rj = rsp(f.as_ptr());
        assert_eq!(cj.is_null(), rj.is_null(), "json_sprintf(\"\") NULL-ness");
        assert!(!cj.is_null(), "C must accept an empty format");
        assert_eq!(p.c.json_string_length(cj), p.r.json_string_length(rj));
        assert_eq!(p.c.dumps(cj, R_ANY), p.r.dumps(rj, R_ANY));
        p.c.json_decref(cj);
        p.r.json_decref(rj);
    }
}

/// C22, C23, C24, C26, C27, C28: number accessors/setters on the wrong type, and
/// NaN/Inf rejection.
#[test]
fn c_number_type_and_nonfinite_errors() {
    unsafe {
        let p = pair();
        // C24: json_real rejects NaN and ±Inf
        for v in [f64::NAN, -f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            assert_eq!(
                p.c.json_real(v).is_null(),
                p.r.json_real(v).is_null(),
                "json_real({v:?})"
            );
            assert!(p.c.json_real(v).is_null(), "C must reject {v:?}");
        }
        // C27: json_real_set rejects NaN/Inf and non-reals
        let cr_ = p.c.json_real(1.0);
        let rr_ = p.r.json_real(1.0);
        for v in [f64::NAN, -f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            assert_eq!(
                p.c.json_real_set(cr_, v),
                p.r.json_real_set(rr_, v),
                "json_real_set({v:?})"
            );
        }
        assert_eq!(
            p.c.json_real_value(cr_).to_bits(),
            p.r.json_real_value(rr_).to_bits(),
            "value unchanged after rejected sets"
        );
        p.c.json_decref(cr_);
        p.r.json_decref(rr_);

        // C22/C23/C26/C27/C28 on every type and on NULL
        for (name, node) in all_types() {
            let cj = node.build(&p.c);
            let rj = node.build(&p.r);
            assert_eq!(
                p.c.json_integer_value(cj),
                p.r.json_integer_value(rj),
                "json_integer_value on {name}"
            );
            assert_eq!(
                p.c.json_real_value(cj).to_bits(),
                p.r.json_real_value(rj).to_bits(),
                "json_real_value on {name}"
            );
            assert_eq!(
                p.c.json_number_value(cj).to_bits(),
                p.r.json_number_value(rj).to_bits(),
                "json_number_value on {name}"
            );
            assert_eq!(
                p.c.json_integer_set(cj, 5),
                p.r.json_integer_set(rj, 5),
                "json_integer_set on {name}"
            );
            assert_eq!(
                p.c.json_real_set(cj, 5.0),
                p.r.json_real_set(rj, 5.0),
                "json_real_set on {name}"
            );
            assert_eq!(
                p.c.json_real_set(cj, f64::NAN),
                p.r.json_real_set(rj, f64::NAN),
                "json_real_set(NaN) on {name}"
            );
            p.c.json_decref(cj);
            p.r.json_decref(rj);
        }
        let nil: json_ptr = std::ptr::null_mut();
        assert_eq!(p.c.json_integer_value(nil), p.r.json_integer_value(nil));
        assert_eq!(
            p.c.json_real_value(nil).to_bits(),
            p.r.json_real_value(nil).to_bits()
        );
        assert_eq!(
            p.c.json_number_value(nil).to_bits(),
            p.r.json_number_value(nil).to_bits()
        );
        assert_eq!(p.c.json_integer_set(nil, 1), p.r.json_integer_set(nil, 1));
        assert_eq!(p.c.json_real_set(nil, 1.0), p.r.json_real_set(nil, 1.0));
    }
}

/// C29, C30, C31, C32, C34, C37: json_delete(NULL) and on singletons;
/// json_equal / json_copy / do_deep_copy with NULL and type mismatches.
#[test]
fn c_delete_equal_copy_null_paths() {
    unsafe {
        let p = pair();
        let nil: json_ptr = std::ptr::null_mut();
        // C29: json_delete(NULL) is a no-op
        p.c.json_delete(nil);
        p.r.json_delete(nil);
        // C31: json_equal with a NULL operand
        let cj = p.c.json_integer(1);
        let rj = p.r.json_integer(1);
        assert_eq!(p.c.json_equal(cj, nil), p.r.json_equal(rj, nil));
        assert_eq!(p.c.json_equal(nil, cj), p.r.json_equal(nil, rj));
        assert_eq!(p.c.json_equal(nil, nil), p.r.json_equal(nil, nil));
        p.c.json_decref(cj);
        p.r.json_decref(rj);
        // C32: type mismatch -> 0
        for (an, a) in all_types() {
            for (bn, b) in all_types() {
                if an == bn {
                    continue;
                }
                let ca = a.build(&p.c);
                let ra = a.build(&p.r);
                let cb = b.build(&p.c);
                let rb = b.build(&p.r);
                assert_eq!(
                    p.c.json_equal(ca, cb),
                    p.r.json_equal(ra, rb),
                    "json_equal({an}, {bn})"
                );
                assert_eq!(
                    p.c.json_equal(ca, cb),
                    0,
                    "C: differing types must not be equal ({an}, {bn})"
                );
                p.c.json_decref(ca);
                p.r.json_decref(ra);
                p.c.json_decref(cb);
                p.r.json_decref(rb);
            }
        }
        // C34/C36: json_copy / json_deep_copy with NULL
        assert_eq!(
            p.c.json_copy(nil).is_null(),
            p.r.json_copy(nil).is_null(),
            "json_copy(NULL)"
        );
        assert_eq!(
            p.c.json_deep_copy(nil).is_null(),
            p.r.json_deep_copy(nil).is_null(),
            "json_deep_copy(NULL)"
        );
        // C37: do_deep_copy(NULL)
        type Fn2 = unsafe extern "C" fn(json_ptr, *mut c_void) -> json_ptr;
        let cf: Symbol<Fn2> = p.c.lib.get(b"do_deep_copy\0").unwrap();
        let rf: Symbol<Fn2> = p.r.lib.get(b"do_deep_copy\0").unwrap();
        let mut cslab = [0u8; 256];
        let mut rslab = [0u8; 256];
        let ch = cslab.as_mut_ptr() as *mut c_void;
        let rh = rslab.as_mut_ptr() as *mut c_void;
        assert_eq!(p.c.hashtable_init(ch), p.r.hashtable_init(rh));
        assert_eq!(
            cf(nil, ch).is_null(),
            rf(nil, rh).is_null(),
            "do_deep_copy(NULL)"
        );
        p.c.hashtable_close(ch);
        p.r.hashtable_close(rh);
    }
}

// ===========================================================================
// D — dump.c
// ===========================================================================

/// D2, D38, Q5, Q6: json_dumpb with a buffer that cannot hold the output.
#[test]
fn d2_d38_dumpb_short_buffer() {
    unsafe {
        let p = pair();
        let node = Node::Obj(vec![
            (b"alpha".to_vec(), Node::Str(b"a rather long value".to_vec())),
            (b"beta".to_vec(), Node::Arr((0..20).map(Node::Int).collect())),
            (b"gamma".to_vec(), Node::Real(1.0 / 3.0)),
        ]);
        with_both(&node, |p2, cj, rj| {
            for f in [0usize, JSON_COMPACT, JSON_INDENT(4), JSON_SORT_KEYS, JSON_ENSURE_ASCII] {
                let need = p2.c.json_dumpb(cj, std::ptr::null_mut(), 0, f);
                assert_eq!(
                    need,
                    p2.r.json_dumpb(rj, std::ptr::null_mut(), 0, f),
                    "required length (flags 0x{f:x})"
                );
                for size in 0..=need + 2 {
                    let mut cb = vec![0x5Au8; need + 8];
                    let mut rb = vec![0x5Au8; need + 8];
                    let cr = p2.c.json_dumpb(cj, cb.as_mut_ptr() as *mut c_char, size, f);
                    let rr = p2.r.json_dumpb(rj, rb.as_mut_ptr() as *mut c_char, size, f);
                    assert_eq!(cr, rr, "json_dumpb(size {size}, flags 0x{f:x}) return");
                    assert_eq!(cb, rb, "json_dumpb(size {size}, flags 0x{f:x}) buffer");
                }
            }
        });
        // D38: json_dumpb returns 0 when the dump is refused outright.
        for (name, n) in all_types() {
            with_both(&n, |p2, cj, rj| {
                // Without JSON_ENCODE_ANY a scalar root is refused.
                let cr = p2.c.json_dumpb(cj, std::ptr::null_mut(), 0, 0);
                let rr = p2.r.json_dumpb(rj, std::ptr::null_mut(), 0, 0);
                assert_eq!(cr, rr, "json_dumpb refusal for {name}");
            });
        }
    }
}

/// D4, D40, Q9: json_dumpfd on an invalid / closed fd.
#[test]
fn d4_d40_dumpfd_bad_fd() {
    unsafe {
        let p = pair();
        let node = Node::Arr(vec![Node::Int(1), Node::Str(b"x".to_vec())]);
        with_both(&node, |p2, cj, rj| {
            for fd in [-1i32, -2, 100000, i32::MAX] {
                for f in [0usize, JSON_COMPACT, JSON_INDENT(2)] {
                    assert_eq!(
                        p2.c.json_dumpfd(cj, fd, f),
                        p2.r.json_dumpfd(rj, fd, f),
                        "json_dumpfd(fd {fd}, flags 0x{f:x})"
                    );
                }
            }
            // A real, then closed, fd.
            let path = {
                let mut d = std::env::temp_dir();
                d.push(format!("jansson_closedfd_{}", std::process::id()));
                d
            };
            let raw = {
                use std::os::unix::io::AsRawFd;
                let file = std::fs::File::create(&path).unwrap();
                file.as_raw_fd()
            }; // file dropped -> fd closed
            assert_eq!(
                p2.c.json_dumpfd(cj, raw, 0),
                p2.r.json_dumpfd(rj, raw, 0),
                "json_dumpfd(closed fd)"
            );
            let _ = std::fs::remove_file(&path);
            // Write end of a pipe whose read end is closed -> EPIPE. Ignore
            // SIGPIPE so the process survives.
            libc_signal_ignore_sigpipe();
            let mut fds = [0i32; 2];
            if libc_pipe(&mut fds) == 0 {
                libc_close(fds[0]);
                assert_eq!(
                    p2.c.json_dumpfd(cj, fds[1], 0),
                    p2.r.json_dumpfd(rj, fds[1], 0),
                    "json_dumpfd(EPIPE)"
                );
                libc_close(fds[1]);
            }
        });
        let _ = p;
    }
}

extern "C" {
    fn pipe(fds: *mut c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn signal(sig: c_int, handler: usize) -> usize;
}
fn libc_pipe(fds: &mut [i32; 2]) -> i32 {
    unsafe { pipe(fds.as_mut_ptr()) }
}
fn libc_close(fd: i32) -> i32 {
    unsafe { close(fd) }
}
fn libc_signal_ignore_sigpipe() {
    const SIGPIPE: c_int = 13;
    const SIG_IGN: usize = 1;
    unsafe {
        signal(SIGPIPE, SIG_IGN);
    }
}

/// D41, D42, D3, D39: json_dump_file / json_dumpf failure paths.
#[test]
fn d41_d42_dump_file_failures() {
    unsafe {
        let p = pair();
        let node = Node::Arr(vec![Node::Int(1)]);
        with_both(&node, |p2, cj, rj| {
            // D41: fopen fails
            for path in [
                "/nonexistent-directory-xyz/out.json",
                "/proc/self/cmdline/nope",
                "",
                "/",
            ] {
                let ps = cs(path);
                assert_eq!(
                    p2.c.json_dump_file(cj, ps.as_ptr(), 0),
                    p2.r.json_dump_file(rj, ps.as_ptr(), 0),
                    "json_dump_file({path:?})"
                );
            }
            // D39/D3: json_dumpf to a FILE* opened read-only -> fwrite fails
            let path = {
                let mut d = std::env::temp_dir();
                d.push(format!("jansson_ro_{}", std::process::id()));
                d
            };
            std::fs::write(&path, b"x").unwrap();
            let ps = cs(path.to_str().unwrap());
            let m = cs("rb");
            let cfp = fopen(ps.as_ptr(), m.as_ptr());
            let rfp = fopen(ps.as_ptr(), m.as_ptr());
            assert!(!cfp.is_null() && !rfp.is_null());
            assert_eq!(
                p2.c.json_dumpf(cj, cfp, 0),
                p2.r.json_dumpf(rj, rfp, 0),
                "json_dumpf(read-only FILE*)"
            );
            fclose(cfp);
            fclose(rfp);
            let _ = std::fs::remove_file(&path);
        });
        let _ = p;
    }
}

extern "C" {
    fn fopen(path: *const c_char, mode: *const c_char) -> *mut c_void;
    fn fclose(f: *mut c_void) -> c_int;
}

/// D13, D43: json_dump_callback with a NULL json, and a non-container root
/// without JSON_ENCODE_ANY.
#[test]
fn d13_d43_dump_rejections() {
    unsafe {
        let p = pair();
        let nil: json_ptr = std::ptr::null_mut();
        let devnull = {
            use std::os::unix::io::IntoRawFd;
            std::fs::OpenOptions::new().write(true).open("/dev/null").unwrap().into_raw_fd()
        };
        for f in [0usize, JSON_ENCODE_ANY, JSON_COMPACT, usize::MAX] {
            // D13: NULL root — rejected even with JSON_ENCODE_ANY
            assert_eq!(
                p.c.dumps(nil, f),
                p.r.dumps(nil, f),
                "json_dumps(NULL, 0x{f:x})"
            );
            assert!(p.c.dumps(nil, f).is_none(), "C must reject a NULL root");
            assert_eq!(
                p.c.json_dumpb(nil, std::ptr::null_mut(), 0, f),
                p.r.json_dumpb(nil, std::ptr::null_mut(), 0, f),
                "json_dumpb(NULL, 0x{f:x})"
            );
            assert_eq!(
                p.c.json_dumpfd(nil, devnull, f),
                p.r.json_dumpfd(nil, devnull, f),
                "json_dumpfd(NULL, 0x{f:x})"
            );
            let ps = cs("/dev/null");
            assert_eq!(
                p.c.json_dump_file(nil, ps.as_ptr(), f),
                p.r.json_dump_file(nil, ps.as_ptr(), f),
                "json_dump_file(NULL, 0x{f:x})"
            );
            assert_eq!(
                p.c.json_dump_callback(nil, Some(sink_cb), std::ptr::null_mut(), f),
                p.r.json_dump_callback(nil, Some(sink_cb), std::ptr::null_mut(), f),
                "json_dump_callback(NULL root, 0x{f:x})"
            );
        }
        // D43: every scalar root without JSON_ENCODE_ANY
        for (name, node) in all_types() {
            with_both(&node, |p2, cj, rj| {
                for f in [0usize, JSON_COMPACT, JSON_INDENT(3), JSON_SORT_KEYS, JSON_EMBED] {
                    assert_eq!(
                        p2.c.dumps(cj, f),
                        p2.r.dumps(rj, f),
                        "json_dumps({name}, 0x{f:x}) without ENCODE_ANY"
                    );
                    assert_eq!(
                        p2.c.json_dumpb(cj, std::ptr::null_mut(), 0, f),
                        p2.r.json_dumpb(rj, std::ptr::null_mut(), 0, f),
                        "json_dumpb({name}, 0x{f:x})"
                    );
                    assert_eq!(
                        p2.c.json_dump_callback(cj, Some(sink_cb), std::ptr::null_mut(), f),
                        p2.r.json_dump_callback(rj, Some(sink_cb), std::ptr::null_mut(), f),
                        "json_dump_callback({name}, 0x{f:x})"
                    );
                }
            });
        }
    }
}

// ===========================================================================
// E — error.c
// ===========================================================================

/// E1..E7: covered exhaustively by `b_internals::m6_m7_m8_error_api`; this test
/// pins the specific rows with their exact expectations.
#[test]
fn e1_e7_error_api_rejections() {
    unsafe {
        let p = pair();
        // E1: NULL error struct is a no-op
        p.c.jsonp_error_init(std::ptr::null_mut(), cs("x").as_ptr());
        p.r.jsonp_error_init(std::ptr::null_mut(), cs("x").as_ptr());
        // E3: NULL error or NULL source
        p.c.jsonp_error_set_source(std::ptr::null_mut(), cs("x").as_ptr());
        p.r.jsonp_error_set_source(std::ptr::null_mut(), cs("x").as_ptr());
        let mut ce = json_error_t::default();
        let mut re = json_error_t::default();
        p.c.jsonp_error_init(&mut ce, std::ptr::null());
        p.r.jsonp_error_init(&mut re, std::ptr::null());
        // E2: NULL source -> empty source, line/column -1, position 0
        assert_eq!(ce.snap(), re.snap(), "jsonp_error_init(NULL source)");
        assert_eq!(ce.snap().line, -1);
        assert_eq!(ce.snap().column, -1);
        assert_eq!(ce.snap().position, 0);
        p.c.jsonp_error_set_source(&mut ce, std::ptr::null());
        p.r.jsonp_error_set_source(&mut re, std::ptr::null());
        assert_eq!(ce.snap(), re.snap(), "jsonp_error_set_source(NULL)");

        // E4: source longer than 80 bytes -> "..." prefix
        for n in [79usize, 80, 81, 83, 84, 200] {
            let src = cs(&(0..n).map(|i| (b'a' + (i % 26) as u8) as char).collect::<String>());
            let mut ce = json_error_t::default();
            let mut re = json_error_t::default();
            p.c.jsonp_error_init(&mut ce, std::ptr::null());
            p.r.jsonp_error_init(&mut re, std::ptr::null());
            p.c.jsonp_error_set_source(&mut ce, src.as_ptr());
            p.r.jsonp_error_set_source(&mut re, src.as_ptr());
            assert_eq!(ce.snap(), re.snap(), "jsonp_error_set_source(len {n})");
            if n >= 80 {
                assert!(
                    ce.snap().source.starts_with(b"..."),
                    "C must prefix a long source with '...'"
                );
            }
        }

        // E5: jsonp_error_set with a NULL error struct
        let cset = p.c.jsonp_error_set_sym();
        let rset = p.r.jsonp_error_set_sym();
        cset(std::ptr::null_mut(), 1, 2, 3, 4, cs("x").as_ptr());
        rset(std::ptr::null_mut(), 1, 2, 3, 4, cs("x").as_ptr());

        // E6: the first error wins
        let mut ce = json_error_t::default();
        let mut re = json_error_t::default();
        p.c.jsonp_error_init(&mut ce, cs("s").as_ptr());
        p.r.jsonp_error_init(&mut re, cs("s").as_ptr());
        cset(&mut ce, 1, 2, 3, json_error_invalid_syntax as c_int, cs("first").as_ptr());
        rset(&mut re, 1, 2, 3, json_error_invalid_syntax as c_int, cs("first").as_ptr());
        let after_first = ce.snap();
        cset(&mut ce, 9, 9, 9, json_error_wrong_type as c_int, cs("second").as_ptr());
        rset(&mut re, 9, 9, 9, json_error_wrong_type as c_int, cs("second").as_ptr());
        assert_eq!(ce.snap(), after_first, "C: the second error must be discarded");
        assert_eq!(ce.snap(), re.snap(), "E6: first error wins");

        // E7: truncation at 158 bytes, with text[159] holding the code
        for n in [155usize, 156, 157, 158, 159, 160, 400] {
            for code in [0i32, 8, 17] {
                let msg = cs(&"m".repeat(n));
                let mut ce = json_error_t::default();
                let mut re = json_error_t::default();
                p.c.jsonp_error_init(&mut ce, std::ptr::null());
                p.r.jsonp_error_init(&mut re, std::ptr::null());
                cset(&mut ce, 0, 0, 0, code, cs("%s").as_ptr(), msg.as_ptr());
                rset(&mut re, 0, 0, 0, code, cs("%s").as_ptr(), msg.as_ptr());
                assert_eq!(ce.snap(), re.snap(), "E7 truncation (len {n}, code {code})");
                assert_eq!(
                    ce.snap().code, code as u8,
                    "C: text[159] must hold the code"
                );
            }
        }
    }
}

// ===========================================================================
// F — memory.c
// ===========================================================================

/// F1, F3, F4, F8, F9, F11: the non-allocator-injection memory rows.
#[test]
fn f_memory_rejections() {
    unsafe {
        let p = pair();
        // F1: zero-size allocation -> NULL
        assert_eq!(
            p.c.jsonp_malloc(0).is_null(),
            p.r.jsonp_malloc(0).is_null(),
            "jsonp_malloc(0)"
        );
        assert!(p.c.jsonp_malloc(0).is_null(), "C: malloc(0) must be NULL");
        // F3: free(NULL) is a no-op
        p.c.jsonp_free(std::ptr::null_mut());
        p.r.jsonp_free(std::ptr::null_mut());
        // F4/F11: realloc to 0, and from NULL
        for (old, new) in [(0usize, 0usize), (16, 0), (0, 16)] {
            let cp = if old == 0 {
                std::ptr::null_mut()
            } else {
                p.c.jsonp_malloc(old)
            };
            let rp = if old == 0 {
                std::ptr::null_mut()
            } else {
                p.r.jsonp_malloc(old)
            };
            let cn = p.c.jsonp_realloc(cp, old, new);
            let rn = p.r.jsonp_realloc(rp, old, new);
            assert_eq!(
                cn.is_null(),
                rn.is_null(),
                "jsonp_realloc({old} -> {new}) NULL-ness"
            );
            p.c.jsonp_free(cn);
            p.r.jsonp_free(rn);
        }
        // F7: jsonp_strndup with len 0 must still return a 1-byte "" buffer.
        let src = cs("abc");
        let cp = p.c.jsonp_strndup(src.as_ptr(), 0);
        let rp = p.r.jsonp_strndup(src.as_ptr(), 0);
        assert_eq!(cp.is_null(), rp.is_null(), "jsonp_strndup(len 0)");
        assert_eq!(cstr_bytes(cp), cstr_bytes(rp));
        p.c.jsonp_free(cp as *mut c_void);
        p.r.jsonp_free(rp as *mut c_void);
        // F8/F9: NULL out-params must be tolerated
        p.c.json_get_alloc_funcs(std::ptr::null_mut(), std::ptr::null_mut());
        p.r.json_get_alloc_funcs(std::ptr::null_mut(), std::ptr::null_mut());
        p.c.json_get_alloc_funcs2(std::ptr::null_mut(), std::ptr::null_mut(), std::ptr::null_mut());
        p.r.json_get_alloc_funcs2(std::ptr::null_mut(), std::ptr::null_mut(), std::ptr::null_mut());
        // Only one of the three at a time.
        let mut m: usize = 0;
        let mut m2: usize = 0;
        p.c.json_get_alloc_funcs2(&mut m, std::ptr::null_mut(), std::ptr::null_mut());
        p.r.json_get_alloc_funcs2(&mut m2, std::ptr::null_mut(), std::ptr::null_mut());
        assert_eq!(m != 0, m2 != 0, "malloc-only get_alloc_funcs2");
    }
}

// ===========================================================================
// G — strbuffer.c
// ===========================================================================

/// G2, G3, G4, G6, G7, G8, G9: the strbuffer overflow guards (checked BEFORE any
/// memory is touched), pop-from-empty, value-after-close, zero-length append.
#[test]
fn g_strbuffer_guards() {
    unsafe {
        let p = pair();
        let data = b"xxxx";

        // G2: `size > SIZE_MAX - 1`
        let mut cb = strbuffer_t::zeroed();
        let mut rb = strbuffer_t::zeroed();
        assert_eq!(
            p.c.strbuffer_init(&mut cb as *mut _ as *mut c_void),
            p.r.strbuffer_init(&mut rb as *mut _ as *mut c_void)
        );
        assert_eq!(
            p.c.strbuffer_append_bytes(
                &mut cb as *mut _ as *mut c_void,
                data.as_ptr() as *const c_char,
                usize::MAX
            ),
            p.r.strbuffer_append_bytes(
                &mut rb as *mut _ as *mut c_void,
                data.as_ptr() as *const c_char,
                usize::MAX
            ),
            "strbuffer_append_bytes(size = SIZE_MAX)"
        );
        // G4: `length > SIZE_MAX - 1 - size` with a huge but not maximal size
        for size in [usize::MAX - 1, usize::MAX - 2, usize::MAX / 2] {
            assert_eq!(
                p.c.strbuffer_append_bytes(
                    &mut cb as *mut _ as *mut c_void,
                    data.as_ptr() as *const c_char,
                    size
                ),
                p.r.strbuffer_append_bytes(
                    &mut rb as *mut _ as *mut c_void,
                    data.as_ptr() as *const c_char,
                    size
                ),
                "strbuffer_append_bytes(size {size})"
            );
            assert_eq!(
                (cb.length, cb.size),
                (rb.length, rb.size),
                "state after the rejected append(size {size})"
            );
        }
        // G9: zero-length append succeeds and changes nothing
        assert_eq!(
            p.c.strbuffer_append_bytes(
                &mut cb as *mut _ as *mut c_void,
                data.as_ptr() as *const c_char,
                0
            ),
            p.r.strbuffer_append_bytes(
                &mut rb as *mut _ as *mut c_void,
                data.as_ptr() as *const c_char,
                0
            ),
            "strbuffer_append_bytes(size 0)"
        );
        assert_eq!((cb.length, cb.size), (rb.length, rb.size));
        // G7: pop from empty -> '\0'
        assert_eq!(
            p.c.strbuffer_pop(&mut cb as *mut _ as *mut c_void),
            p.r.strbuffer_pop(&mut rb as *mut _ as *mut c_void),
            "strbuffer_pop(empty)"
        );
        // G8: value after close is NULL
        p.c.strbuffer_close(&mut cb as *mut _ as *mut c_void);
        p.r.strbuffer_close(&mut rb as *mut _ as *mut c_void);
        assert_eq!(
            p.c.strbuffer_value(&cb as *const _ as *const c_void).is_null(),
            p.r.strbuffer_value(&rb as *const _ as *const c_void).is_null(),
            "strbuffer_value after close"
        );
        // Double close must be safe in both.
        p.c.strbuffer_close(&mut cb as *mut _ as *mut c_void);
        p.r.strbuffer_close(&mut rb as *mut _ as *mut c_void);
        assert_eq!((cb.length, cb.size), (rb.length, rb.size));

        // G8 via steal_value.
        let mut cb = strbuffer_t::zeroed();
        let mut rb = strbuffer_t::zeroed();
        assert_eq!(
            p.c.strbuffer_init(&mut cb as *mut _ as *mut c_void),
            p.r.strbuffer_init(&mut rb as *mut _ as *mut c_void)
        );
        let cv = p.c.strbuffer_steal_value(&mut cb as *mut _ as *mut c_void);
        let rv = p.r.strbuffer_steal_value(&mut rb as *mut _ as *mut c_void);
        assert_eq!(cv.is_null(), rv.is_null());
        assert_eq!(
            p.c.strbuffer_value(&cb as *const _ as *const c_void).is_null(),
            p.r.strbuffer_value(&rb as *const _ as *const c_void).is_null(),
            "strbuffer_value after steal"
        );
        p.c.jsonp_free(cv as *mut c_void);
        p.r.jsonp_free(rv as *mut c_void);

        // G6: strbuffer_append_byte inherits the guard state.
        let mut cb = strbuffer_t::zeroed();
        let mut rb = strbuffer_t::zeroed();
        assert_eq!(
            p.c.strbuffer_init(&mut cb as *mut _ as *mut c_void),
            p.r.strbuffer_init(&mut rb as *mut _ as *mut c_void)
        );
        for b in [0u8, 1, 0x7F, 0x80, 0xFF] {
            assert_eq!(
                p.c.strbuffer_append_byte(&mut cb as *mut _ as *mut c_void, b as c_char),
                p.r.strbuffer_append_byte(&mut rb as *mut _ as *mut c_void, b as c_char),
                "strbuffer_append_byte({b})"
            );
        }
        assert_eq!(cb.bytes(), rb.bytes());
        p.c.strbuffer_close(&mut cb as *mut _ as *mut c_void);
        p.r.strbuffer_close(&mut rb as *mut _ as *mut c_void);
    }
}

// ===========================================================================
// H — hashtable.c
// ===========================================================================

/// H1, H2, H3, H10, H11, H12, H13, H14, H15: misses, end-of-iteration, empty
/// table, and the zero-length key (which is valid).
#[test]
fn h_hashtable_misses_and_boundaries() {
    unsafe {
        let p = pair();
        let mut cslab = [0u8; 256];
        let mut rslab = [0u8; 256];
        let ch = cslab.as_mut_ptr() as *mut c_void;
        let rh = rslab.as_mut_ptr() as *mut c_void;
        assert_eq!(p.c.hashtable_init(ch), p.r.hashtable_init(rh));

        // H14: iter on an empty table -> NULL
        assert_eq!(
            p.c.hashtable_iter(ch).is_null(),
            p.r.hashtable_iter(rh).is_null(),
            "hashtable_iter(empty)"
        );
        // H1/H10/H11/H12: get / del / iter_at on an empty table
        let key = b"nope";
        assert_eq!(
            p.c.hashtable_get(ch, key.as_ptr() as *const c_char, 4).is_null(),
            p.r.hashtable_get(rh, key.as_ptr() as *const c_char, 4).is_null(),
            "hashtable_get(empty)"
        );
        assert_eq!(
            p.c.hashtable_del(ch, key.as_ptr() as *const c_char, 4),
            p.r.hashtable_del(rh, key.as_ptr() as *const c_char, 4),
            "hashtable_del(empty)"
        );
        assert_eq!(
            p.c.hashtable_iter_at(ch, key.as_ptr() as *const c_char, 4).is_null(),
            p.r.hashtable_iter_at(rh, key.as_ptr() as *const c_char, 4).is_null(),
            "hashtable_iter_at(empty)"
        );

        // H15: zero-length key is VALID
        let empty: [u8; 1] = [0];
        assert_eq!(
            p.c.hashtable_set(ch, empty.as_ptr() as *const c_char, 0, p.c.json_integer(1)),
            p.r.hashtable_set(rh, empty.as_ptr() as *const c_char, 0, p.r.json_integer(1)),
            "hashtable_set(key_len 0)"
        );
        assert_eq!(
            p.c.hashtable_get(ch, empty.as_ptr() as *const c_char, 0).is_null(),
            p.r.hashtable_get(rh, empty.as_ptr() as *const c_char, 0).is_null(),
            "hashtable_get(key_len 0)"
        );
        assert!(
            !p.c.hashtable_get(ch, empty.as_ptr() as *const c_char, 0).is_null(),
            "C must accept a zero-length key"
        );

        // Fill enough to have populated and empty buckets, then probe misses.
        for i in 0..40usize {
            let k = format!("k{i:03}").into_bytes();
            p.c.hashtable_set(
                ch, k.as_ptr() as *const c_char, k.len(), p.c.json_integer(i as i64),
            );
            p.r.hashtable_set(
                rh, k.as_ptr() as *const c_char, k.len(), p.r.json_integer(i as i64),
            );
        }
        // H2: same hash bucket, different key — must miss.
        for probe in ["k000x", "k00", "K000", "", "zzz"] {
            let b = probe.as_bytes();
            assert_eq!(
                p.c.hashtable_get(ch, b.as_ptr() as *const c_char, b.len()).is_null(),
                p.r.hashtable_get(rh, b.as_ptr() as *const c_char, b.len()).is_null(),
                "hashtable_get({probe:?})"
            );
            assert_eq!(
                p.c.hashtable_del(ch, b.as_ptr() as *const c_char, b.len()),
                p.r.hashtable_del(rh, b.as_ptr() as *const c_char, b.len()),
                "hashtable_del({probe:?})"
            );
            assert_eq!(
                p.c.hashtable_iter_at(ch, b.as_ptr() as *const c_char, b.len()).is_null(),
                p.r.hashtable_iter_at(rh, b.as_ptr() as *const c_char, b.len()).is_null(),
                "hashtable_iter_at({probe:?})"
            );
        }
        // Same bytes, wrong length -> miss.
        let k = b"k000";
        for len in [0usize, 1, 2, 3, 5] {
            if len > k.len() {
                continue;
            }
            assert_eq!(
                p.c.hashtable_get(ch, k.as_ptr() as *const c_char, len).is_null(),
                p.r.hashtable_get(rh, k.as_ptr() as *const c_char, len).is_null(),
                "hashtable_get(\"k000\", len {len})"
            );
        }
        // H13: iter_next past the end -> NULL
        assert_eq!(
            hashtable_entries(&p.c, ch),
            hashtable_entries(&p.r, rh),
            "traversal"
        );
        let mut ci = p.c.hashtable_iter(ch);
        let mut ri = p.r.hashtable_iter(rh);
        while !ci.is_null() {
            ci = p.c.hashtable_iter_next(ch, ci);
            ri = p.r.hashtable_iter_next(rh, ri);
            assert_eq!(ci.is_null(), ri.is_null(), "iter_next lock-step");
        }
        p.c.hashtable_close(ch);
        p.r.hashtable_close(rh);
    }
}

// ===========================================================================
// I — version.c
// ===========================================================================

/// I1..I4: covered by `b_internals::n1_n2_version`, pinned here with the exact
/// signed-difference expectations including INT_MIN/INT_MAX overflow.
#[test]
fn i1_i4_version_cmp() {
    unsafe {
        let p = pair();
        for (a, b, c) in [
            (2, 15, 0),
            (1, 15, 0),
            (3, 15, 0),
            (2, 14, 0),
            (2, 16, 0),
            (2, 15, 1),
            (2, 15, -1),
            (i32::MIN, 0, 0),
            (i32::MAX, 0, 0),
            (2, i32::MIN, 0),
            (2, i32::MAX, 0),
            (2, 15, i32::MIN),
            (2, 15, i32::MAX),
        ] {
            assert_eq!(
                p.c.jansson_version_cmp(a, b, c),
                p.r.jansson_version_cmp(a, b, c),
                "jansson_version_cmp({a},{b},{c})"
            );
        }
    }
}

// ===========================================================================
// J — utf.c  /  Q7, Q8
// ===========================================================================

/// J1, J2, J3, Q7: utf8_encode with out-of-range codepoints (a C `int` accepts
/// any value, so `-1`, `0x110000`, `INT_MAX`, `INT_MIN` are all real inputs),
/// and the fact that surrogates ARE accepted.
#[test]
fn j1_j3_q7_utf8_encode_range() {
    unsafe {
        let p = pair();
        let cf: Symbol<unsafe extern "C" fn(c_int, *mut c_char, *mut usize) -> c_int> =
            p.c.lib.get(b"utf8_encode\0").unwrap();
        let rf: Symbol<unsafe extern "C" fn(c_int, *mut c_char, *mut usize) -> c_int> =
            p.r.lib.get(b"utf8_encode\0").unwrap();
        let mut cps: Vec<i32> = vec![
            -1, -2, -100, i32::MIN, i32::MIN + 1, 0x110000, 0x110001, 0x200000, 0x7FFFFFFF,
            i32::MAX, i32::MAX - 1,
            // J3: surrogates — accepted by utf8_encode (no check)
            0xD800, 0xDBFF, 0xDC00, 0xDFFF,
            // exact boundaries
            0, 0x7F, 0x80, 0x7FF, 0x800, 0xFFFF, 0x10000, 0x10FFFF,
        ];
        let mut rng = Rng::new(0x07_07);
        for _ in 0..5000 {
            cps.push(rng.next_u64() as i32);
        }
        for cp in cps {
            let mut cb = [0x5Au8; 8];
            let mut rb = [0x5Au8; 8];
            let mut cl: usize = 0xDEAD;
            let mut rl: usize = 0xDEAD;
            let cr = cf(cp, cb.as_mut_ptr() as *mut c_char, &mut cl);
            let rr = rf(cp, rb.as_mut_ptr() as *mut c_char, &mut rl);
            assert_eq!(cr, rr, "utf8_encode({cp}) return");
            assert_eq!(cl, rl, "utf8_encode({cp}) length");
            assert_eq!(cb, rb, "utf8_encode({cp}) bytes");
            if cp < 0 || cp > 0x10FFFF {
                assert_eq!(cr, -1, "C must reject codepoint {cp}");
            }
        }
    }
}

/// J4..J11, J16..J19, Q8: the utf8 validation rejections, with `size` values one
/// step past the valid 2..4 range.
#[test]
fn j4_j19_q8_utf8_validation_rejections() {
    unsafe {
        let p = pair();
        let cff: Symbol<unsafe extern "C" fn(c_char) -> usize> =
            p.c.lib.get(b"utf8_check_first\0").unwrap();
        let rff: Symbol<unsafe extern "C" fn(c_char) -> usize> =
            p.r.lib.get(b"utf8_check_first\0").unwrap();
        // J4/J5/J6: the rejecting lead-byte classes
        for b in (0x80u16..=0xBF).chain([0xC0, 0xC1]).chain(0xF5..=0xFF) {
            let bb = b as u8 as c_char;
            assert_eq!(cff(bb), rff(bb), "utf8_check_first(0x{b:02X})");
            assert_eq!(cff(bb), 0, "C must reject lead byte 0x{b:02X}");
        }

        let cfu: Symbol<unsafe extern "C" fn(*const c_char, usize, *mut i32) -> c_int> =
            p.c.lib.get(b"utf8_check_full\0").unwrap();
        let rfu: Symbol<unsafe extern "C" fn(*const c_char, usize, *mut i32) -> c_int> =
            p.r.lib.get(b"utf8_check_full\0").unwrap();
        let check = |buf: &[u8], size: usize, expect_zero: bool, why: &str| {
            let mut cc: i32 = -1;
            let mut rc: i32 = -1;
            let cr = cfu(buf.as_ptr() as *const c_char, size, &mut cc);
            let rr = rfu(buf.as_ptr() as *const c_char, size, &mut rc);
            assert_eq!(cr, rr, "utf8_check_full({buf:02x?}, {size}) [{why}]");
            if expect_zero {
                assert_eq!(cr, 0, "C must reject {buf:02x?} size {size} [{why}]");
            }
        };
        // J7 / Q8: size not 2, 3 or 4
        for size in [0usize, 1, 5, 6, 100, usize::MAX] {
            check(&[0xE2, 0x82, 0xAC, 0, 0, 0, 0, 0], size, true, "bad size");
        }
        // J8: non-continuation byte in position i
        check(&[0xC2, 0x41], 2, true, "bad cont");
        check(&[0xE2, 0x41, 0xAC], 3, true, "bad cont 1");
        check(&[0xE2, 0x82, 0x41], 3, true, "bad cont 2");
        check(&[0xF0, 0x41, 0x80, 0x80], 4, true, "bad cont 1/4");
        check(&[0xF0, 0x90, 0x41, 0x80], 4, true, "bad cont 2/4");
        check(&[0xF0, 0x90, 0x80, 0x41], 4, true, "bad cont 3/4");
        // J9: value > 0x10FFFF
        check(&[0xF4, 0x90, 0x80, 0x80], 4, true, "> U+10FFFF");
        check(&[0xF7, 0xBF, 0xBF, 0xBF], 4, true, "> U+10FFFF");
        // J10: surrogate halves
        check(&[0xED, 0xA0, 0x80], 3, true, "high surrogate");
        check(&[0xED, 0xBF, 0xBF], 3, true, "low surrogate");
        // J11: overlong encodings
        check(&[0xC2, 0x80], 2, false, "shortest 2-byte is fine");
        check(&[0xE0, 0x80, 0x80], 3, true, "overlong 3-byte");
        check(&[0xE0, 0x9F, 0xBF], 3, true, "overlong 3-byte");
        check(&[0xF0, 0x80, 0x80, 0x80], 4, true, "overlong 4-byte");
        check(&[0xF0, 0x8F, 0xBF, 0xBF], 4, true, "overlong 4-byte");

        // J12..J15: utf8_iterate
        let cfi: Symbol<unsafe extern "C" fn(*const c_char, usize, *mut i32) -> *const c_char> =
            p.c.lib.get(b"utf8_iterate\0").unwrap();
        let rfi: Symbol<unsafe extern "C" fn(*const c_char, usize, *mut i32) -> *const c_char> =
            p.r.lib.get(b"utf8_iterate\0").unwrap();
        let iter = |buf: &[u8], size: usize, why: &str| {
            let base = buf.as_ptr() as *const c_char;
            let mut cc: i32 = -1;
            let mut rc: i32 = -1;
            let cr = cfi(base, size, &mut cc);
            let rr = rfi(base, size, &mut rc);
            let coff = if cr.is_null() { None } else { Some(cr as usize - base as usize) };
            let roff = if rr.is_null() { None } else { Some(rr as usize - base as usize) };
            assert_eq!((coff, cc), (roff, rc), "utf8_iterate({buf:02x?}, {size}) [{why}]");
            coff
        };
        // J12: bufsize 0 returns `buffer` unchanged
        assert_eq!(iter(&[0x41, 0, 0, 0], 0, "bufsize 0"), Some(0));
        // J13: invalid lead byte -> NULL
        for lead in [0x80u8, 0xBF, 0xC0, 0xC1, 0xF5, 0xFF] {
            assert_eq!(iter(&[lead, 0x80, 0x80, 0x80], 4, "bad lead"), None);
        }
        // J14: count > bufsize (truncated sequence) -> NULL
        assert_eq!(iter(&[0xC2, 0x80, 0, 0], 1, "truncated 2-byte"), None);
        assert_eq!(iter(&[0xE2, 0x82, 0xAC, 0], 1, "truncated 3-byte"), None);
        assert_eq!(iter(&[0xE2, 0x82, 0xAC, 0], 2, "truncated 3-byte"), None);
        assert_eq!(iter(&[0xF0, 0x9F, 0x98, 0x80], 3, "truncated 4-byte"), None);
        // J15: invalid continuation -> NULL
        assert_eq!(iter(&[0xC2, 0x41, 0, 0], 2, "bad cont"), None);

        // J16..J19: utf8_check_string
        let cfs: Symbol<unsafe extern "C" fn(*const c_char, usize) -> c_int> =
            p.c.lib.get(b"utf8_check_string\0").unwrap();
        let rfs: Symbol<unsafe extern "C" fn(*const c_char, usize) -> c_int> =
            p.r.lib.get(b"utf8_check_string\0").unwrap();
        let strchk = |b: &[u8], len: usize, why: &str| {
            let cr = cfs(b.as_ptr() as *const c_char, len);
            let rr = rfs(b.as_ptr() as *const c_char, len);
            assert_eq!(cr, rr, "utf8_check_string({b:02x?}, {len}) [{why}]");
            cr
        };
        // J19: empty string is valid
        assert_eq!(strchk(b"", 0, "empty"), 1);
        // J16: invalid lead byte
        assert_eq!(strchk(&[0x80], 1, "bad lead"), 0);
        assert_eq!(strchk(&[b'a', 0xFF, b'b'], 3, "bad lead mid"), 0);
        // J17: sequence truncated at the end of the string
        assert_eq!(strchk(&[0xC2], 1, "truncated"), 0);
        assert_eq!(strchk(&[b'a', 0xE2, 0x82], 3, "truncated at end"), 0);
        // J18: invalid full sequence
        assert_eq!(strchk(&[0xED, 0xA0, 0x80], 3, "surrogate"), 0);
        assert_eq!(strchk(&[0xC0, 0x80], 2, "overlong"), 0);
    }
}

// ===========================================================================
// K — strconv.c  /  Q10, Q13
// ===========================================================================

/// K2: jsonp_strtod numeric overflow -> -1 (underflow is NOT an error).
#[test]
fn k2_jsonp_strtod_overflow() {
    unsafe {
        let p = pair();
        for (text, expect_fail) in [
            ("1e309", true),
            ("-1e309", true),
            ("1e999", true),
            ("-1e999", true),
            ("1e10000", true),
            ("1.7976931348623157e309", true),
            ("1e-400", false),   // underflow -> 0.0, no error
            ("-1e-400", false),
            ("1e-100000", false),
            ("1e308", false),
            ("0", false),
        ] {
            let b = text.as_bytes();
            let mut cb = strbuffer_t::zeroed();
            let mut rb = strbuffer_t::zeroed();
            assert_eq!(
                p.c.strbuffer_init(&mut cb as *mut _ as *mut c_void),
                p.r.strbuffer_init(&mut rb as *mut _ as *mut c_void)
            );
            p.c.strbuffer_append_bytes(
                &mut cb as *mut _ as *mut c_void,
                b.as_ptr() as *const c_char,
                b.len(),
            );
            p.r.strbuffer_append_bytes(
                &mut rb as *mut _ as *mut c_void,
                b.as_ptr() as *const c_char,
                b.len(),
            );
            let mut cv: c_double = -1.0;
            let mut rv: c_double = -1.0;
            let cr = p.c.jsonp_strtod(&mut cb as *mut _ as *mut c_void, &mut cv);
            let rr = p.r.jsonp_strtod(&mut rb as *mut _ as *mut c_void, &mut rv);
            assert_eq!(cr, rr, "jsonp_strtod({text:?}) return");
            assert_eq!(
                cr == -1,
                expect_fail,
                "C: jsonp_strtod({text:?}) expected fail={expect_fail}, got {cr}"
            );
            if cr == 0 {
                assert_eq!(cv.to_bits(), rv.to_bits(), "jsonp_strtod({text:?}) value");
            }
            p.c.strbuffer_close(&mut cb as *mut _ as *mut c_void);
            p.r.strbuffer_close(&mut rb as *mut _ as *mut c_void);
        }
        // L34: the same overflow through the decoder.
        for text in ["[1e309]", "[1e999]", "[-1e999]", "[1e-400]"] {
            let mut ce = json_error_t::default();
            let mut re = json_error_t::default();
            let cj = p.c.json_loadb(
                text.as_ptr() as *const c_char, text.len(), 0, &mut ce,
            );
            let rj = p.r.json_loadb(
                text.as_ptr() as *const c_char, text.len(), 0, &mut re,
            );
            assert_eq!(cj.is_null(), rj.is_null(), "load({text})");
            assert_eq!(ce.snap(), re.snap(), "load({text}) error");
            p.c.json_decref(cj);
            p.r.json_decref(rj);
        }
    }
}

/// K4, K5, K6, K7, Q10, Q13: jsonp_dtostr with a too-short buffer, size 0, and
/// out-of-range precisions; `dtoa` with out-of-range mode / negative ndigits.
#[test]
fn k4_k7_q10_q13_dtostr_and_dtoa_ranges() {
    unsafe {
        let p = pair();
        let cf: Symbol<unsafe extern "C" fn(*mut c_char, usize, c_double, c_int) -> c_int> =
            p.c.lib.get(b"jsonp_dtostr\0").unwrap();
        let rf: Symbol<unsafe extern "C" fn(*mut c_char, usize, c_double, c_int) -> c_int> =
            p.r.lib.get(b"jsonp_dtostr\0").unwrap();
        let vals = [
            0.0f64, -0.0, 1.0, -1.0, 0.5, 1e-5, 1e16, 1e17, 1e300, 1e-300, f64::MAX, f64::MIN,
            5e-324, 1.0 / 3.0,
        ];
        // K5: size 0; K4: sizes short of the requirement; Q10: precision out of range.
        for &v in &vals {
            for prec in [-2i32, -1, 0, 1, 15, 17, 24, 25, 26, 31, 32, 33, 100, i32::MAX, i32::MIN] {
                for size in [0usize, 1, 2, 3, 4, 5, 10, 20, 24, 25, 26, 40] {
                    let mut cb = [0x5Au8; 128];
                    let mut rb = [0x5Au8; 128];
                    let cr = cf(cb.as_mut_ptr() as *mut c_char, size, v, prec);
                    let rr = rf(rb.as_mut_ptr() as *mut c_char, size, v, prec);
                    assert_eq!(
                        cr, rr,
                        "jsonp_dtostr({v:?}, size {size}, prec {prec}) return"
                    );
                    if cr >= 0 {
                        assert_eq!(
                            &cb[..cr as usize],
                            &rb[..cr as usize],
                            "jsonp_dtostr({v:?}, size {size}, prec {prec}) bytes"
                        );
                    }
                    if size == 0 {
                        assert_eq!(cr, -1, "C must fail with size 0");
                    }
                }
            }
        }
        // Q13: dtoa with mode / ndigits outside the documented range.
        let cd = p.c.dtoa_sym();
        let rd = p.r.dtoa_sym();
        for &v in &vals {
            for mode in [-2i32, -1, 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 100, i32::MAX] {
                for nd in [-100i32, -2, -1, 0, 1, 17, 18, 24, 25, 26, 100, i32::MAX] {
                    let mut cdec: c_int = -9999;
                    let mut csign: c_int = -9999;
                    let mut cend: *mut c_char = std::ptr::null_mut();
                    let mut rdec: c_int = -9999;
                    let mut rsign: c_int = -9999;
                    let mut rend: *mut c_char = std::ptr::null_mut();
                    let cs_ = cd(v, mode, nd, &mut cdec, &mut csign, &mut cend);
                    let rs_ = rd(v, mode, nd, &mut rdec, &mut rsign, &mut rend);
                    assert_eq!(
                        cs_.is_null(),
                        rs_.is_null(),
                        "dtoa({v:?}, mode {mode}, nd {nd}) NULL-ness"
                    );
                    if cs_.is_null() {
                        continue;
                    }
                    assert_eq!(
                        (cstr_bytes(cs_), cdec, csign),
                        (cstr_bytes(rs_), rdec, rsign),
                        "dtoa({v:?}, mode {mode}, nd {nd})"
                    );
                    p.c.freedtoa(cs_);
                    p.r.freedtoa(rs_);
                }
            }
        }
    }
}

// ===========================================================================
// L, M — load.c
// ===========================================================================

/// Each distinct lexer/parser rejection, asserted by its exact
/// `json_error_code` AND the full error text/line/column/position.
#[test]
fn l_m_load_rejections_exact_codes() {
    unsafe {
        let p = pair();
        // (input, flags, expected json_error_code)
        let cases: Vec<(&[u8], usize, u8)> = vec![
            // L6/L7/L35: invalid UTF-8 (outside and inside strings)
            (b"[\x80]", 0, json_error_invalid_utf8),
            (b"[\xC0\x80]", 0, json_error_invalid_utf8),
            (b"[\xFF]", 0, json_error_invalid_utf8),
            (b"[\xF5\x80\x80\x80]", 0, json_error_invalid_utf8),
            (b"[\"\x80\"]", 0, json_error_invalid_utf8),
            (b"[\"\xED\xA0\x80\"]", 0, json_error_invalid_utf8),
            (b"[\"\xF4\x90\x80\x80\"]", 0, json_error_invalid_utf8),
            (b"[\"\xC3\"]", 0, json_error_invalid_utf8),
            (b"\x80", 0, json_error_invalid_utf8),
            // L14: EOF before the closing quote
            (b"[\"abc", 0, json_error_premature_end_of_input),
            (b"\"abc", JSON_DECODE_ANY, json_error_premature_end_of_input),
            // L15: raw newline in a string
            (b"[\"a\nb\"]", 0, json_error_invalid_syntax),
            // L16: other raw control chars
            (b"[\"a\x01b\"]", 0, json_error_invalid_syntax),
            (b"[\"a\tb\"]", 0, json_error_invalid_syntax),
            (b"[\"a\x00b\"]", 0, json_error_invalid_syntax),
            (b"[\"a\x1fb\"]", 0, json_error_invalid_syntax),
            // L17: \u with fewer than 4 hex digits
            (b"[\"\\u12g4\"]", 0, json_error_invalid_syntax),
            (b"[\"\\u1\"]", 0, json_error_invalid_syntax),
            (b"[\"\\u\"]", 0, json_error_invalid_syntax),
            (b"[\"\\u123\"]", 0, json_error_invalid_syntax),
            // L18: unknown escape
            (b"[\"\\x\"]", 0, json_error_invalid_syntax),
            (b"[\"\\ \"]", 0, json_error_invalid_syntax),
            (b"[\"\\a\"]", 0, json_error_invalid_syntax),
            // L22/L23/L24: surrogate-pair errors
            (b"[\"\\uD800\\u0041\"]", 0, json_error_invalid_syntax),
            (b"[\"\\uD800\"]", 0, json_error_invalid_syntax),
            (b"[\"\\uD800x\"]", 0, json_error_invalid_syntax),
            (b"[\"\\uDC00\"]", 0, json_error_invalid_syntax),
            (b"[\"\\uDFFF\"]", 0, json_error_invalid_syntax),
            (b"[\"\\uD800\\uD800\"]", 0, json_error_invalid_syntax),
            (b"[\"\\uDBFF\\uDBFF\"]", 0, json_error_invalid_syntax),
            // L27/L28/L32/L33/L36/L37: number & token grammar -> "invalid token"
            (b"[01]", 0, json_error_invalid_syntax),
            (b"[-012]", 0, json_error_invalid_syntax),
            (b"[-]", 0, json_error_invalid_syntax),
            (b"[-x]", 0, json_error_invalid_syntax),
            (b"[-.5]", 0, json_error_invalid_syntax),
            (b"[1.]", 0, json_error_invalid_syntax),
            (b"[1.e5]", 0, json_error_invalid_syntax),
            (b"[1e]", 0, json_error_invalid_syntax),
            (b"[1e+]", 0, json_error_invalid_syntax),
            (b"[1ex]", 0, json_error_invalid_syntax),
            (b"[nul]", 0, json_error_invalid_syntax),
            (b"[True]", 0, json_error_invalid_syntax),
            (b"[NaN]", 0, json_error_invalid_syntax),
            (b"[Infinity]", 0, json_error_invalid_syntax),
            (b"[#]", 0, json_error_invalid_syntax),
            (b"['a']", 0, json_error_invalid_syntax),
            (b"[@]", 0, json_error_invalid_syntax),
            (b"[+1]", 0, json_error_invalid_syntax),
            (b"[.5]", 0, json_error_invalid_syntax),
            (b"[(]", 0, json_error_invalid_syntax),
            // L29/L30: integer overflow
            (b"[99999999999999999999]", 0, json_error_numeric_overflow),
            (b"[-99999999999999999999]", 0, json_error_numeric_overflow),
            (b"[9223372036854775808]", 0, json_error_numeric_overflow),
            (b"[-9223372036854775809]", 0, json_error_numeric_overflow),
            // L34: real overflow
            (b"[1e999]", 0, json_error_numeric_overflow),
            (b"[-1e999]", 0, json_error_numeric_overflow),
            (b"[1e309]", 0, json_error_numeric_overflow),
            // M2: "string or '}' expected"
            (b"{1:2}", 0, json_error_invalid_syntax),
            (b"{,}", 0, json_error_invalid_syntax),
            (b"{[]:1}", 0, json_error_invalid_syntax),
            (b"{", 0, json_error_premature_end_of_input),
            // M4: NUL byte in an object key
            (b"{\"a\\u0000b\":1}", 0, json_error_null_byte_in_key),
            (b"{\"a\\u0000b\":1}", JSON_ALLOW_NUL, json_error_null_byte_in_key),
            (b"{\"\\u0000\":1}", JSON_ALLOW_NUL, json_error_null_byte_in_key),
            // M5: duplicate key with JSON_REJECT_DUPLICATES
            (b"{\"a\":1,\"a\":2}", JSON_REJECT_DUPLICATES, json_error_duplicate_key),
            (b"{\"a\":1,\"\\u0061\":2}", JSON_REJECT_DUPLICATES, json_error_duplicate_key),
            // M6: "':' expected"
            (b"{\"a\" 1}", 0, json_error_invalid_syntax),
            (b"{\"a\",1}", 0, json_error_invalid_syntax),
            (b"{\"a\"}", 0, json_error_invalid_syntax),
            // M9: "'}' expected"
            (b"{\"a\":1", 0, json_error_premature_end_of_input),
            (b"{\"a\":1]", 0, json_error_invalid_syntax),
            (b"{\"a\":1,", 0, json_error_premature_end_of_input),
            // M13: "']' expected"
            (b"[1,2", 0, json_error_premature_end_of_input),
            (b"[1 2]", 0, json_error_invalid_syntax),
            (b"[1}", 0, json_error_invalid_syntax),
            (b"[1,", 0, json_error_premature_end_of_input),
            // M17: "unexpected token"
            (b"[,1]", 0, json_error_invalid_syntax),
            (b"[:]", 0, json_error_invalid_syntax),
            (b"[1,]", 0, json_error_invalid_syntax),
            (b"[]]", 0, json_error_end_of_input_expected),
            // M14: stack overflow at depth 2049
            (b"", 0, json_error_premature_end_of_input),
            // M15: \u0000 in a value without JSON_ALLOW_NUL
            (b"[\"\\u0000\"]", 0, json_error_null_character),
            (b"[\"a\\u0000b\"]", 0, json_error_null_character),
            // M19: root is not a container without JSON_DECODE_ANY
            (b"42", 0, json_error_invalid_syntax),
            (b"\"s\"", 0, json_error_invalid_syntax),
            (b"true", 0, json_error_invalid_syntax),
            (b"false", 0, json_error_invalid_syntax),
            (b"null", 0, json_error_invalid_syntax),
            (b"1.5", 0, json_error_invalid_syntax),
            // M21: trailing garbage without JSON_DISABLE_EOF_CHECK
            (b"[1] garbage", 0, json_error_end_of_input_expected),
            (b"{} {}", 0, json_error_end_of_input_expected),
            (b"[1][2]", 0, json_error_end_of_input_expected),
            (b"42 43", JSON_DECODE_ANY, json_error_end_of_input_expected),
            // L3: a "near" context longer than 20 chars suppresses the suffix
            (b"[abcdefghijklmnopqrst]", 0, json_error_invalid_syntax),
            (b"[abcdefghijklmnopqrstu]", 0, json_error_invalid_syntax),
            (b"[verylongidentifierthatexceedstwentychars]", 0, json_error_invalid_syntax),
        ];

        for (input, flags, expect_code) in cases {
            let mut ce = json_error_t::default();
            let mut re = json_error_t::default();
            let cj = p.c.json_loadb(input.as_ptr() as *const c_char, input.len(), flags, &mut ce);
            let rj = p.r.json_loadb(input.as_ptr() as *const c_char, input.len(), flags, &mut re);
            let ctx = format!(
                "json_loadb({:?}, flags 0x{flags:x})",
                String::from_utf8_lossy(input)
            );
            assert!(cj.is_null(), "{ctx}: C should have rejected this input");
            assert!(
                rj.is_null(),
                "{ctx}: Rust ACCEPTED input the C rejected (C err {:?})",
                String::from_utf8_lossy(&ce.snap().text)
            );
            // The whole error struct, not just "both failed".
            assert_eq!(ce.snap(), re.snap(), "{ctx}: json_error_t");
            // And the code we derived from the C source.
            if expect_code != json_error_unknown {
                assert_eq!(
                    ce.snap().code, expect_code,
                    "{ctx}: expected C code {expect_code}, got {} (text {:?})",
                    ce.snap().code,
                    String::from_utf8_lossy(&ce.snap().text)
                );
            }
        }

        // M14: stack overflow — depth 2049 rejected, 2048 accepted.
        for (d, should_fail) in [(2048usize, false), (2049, true), (3000, true), (5000, true)] {
            let input = format!("{}{}", "[".repeat(d), "]".repeat(d)).into_bytes();
            let mut ce = json_error_t::default();
            let mut re = json_error_t::default();
            let cj = p.c.json_loadb(input.as_ptr() as *const c_char, input.len(), 0, &mut ce);
            let rj = p.r.json_loadb(input.as_ptr() as *const c_char, input.len(), 0, &mut re);
            assert_eq!(cj.is_null(), rj.is_null(), "depth {d}");
            assert_eq!(ce.snap(), re.snap(), "depth {d} error");
            assert_eq!(cj.is_null(), should_fail, "C at depth {d}");
            if should_fail {
                assert_eq!(
                    ce.snap().code, json_error_stack_overflow,
                    "depth {d} must be json_error_stack_overflow"
                );
            }
            p.c.json_decref(cj);
            p.r.json_decref(rj);
        }
    }
}

/// M22, M24, M26, M28, M31, M33: NULL / invalid arguments to each decoder.
#[test]
fn m_decoder_argument_rejections() {
    unsafe {
        let p = pair();
        for flags in [0usize, JSON_DECODE_ANY, 31] {
            // M22: json_loads(NULL)
            let mut ce = json_error_t::default();
            let mut re = json_error_t::default();
            let cj = p.c.json_loads(std::ptr::null(), flags, &mut ce);
            let rj = p.r.json_loads(std::ptr::null(), flags, &mut re);
            assert!(cj.is_null() && rj.is_null(), "json_loads(NULL)");
            assert_eq!(ce.snap(), re.snap(), "json_loads(NULL) error");
            assert_eq!(ce.snap().code, json_error_invalid_argument);
            assert_eq!(ce.snap().source, b"<string>".to_vec());

            // M24: json_loadb(NULL)
            let mut ce = json_error_t::default();
            let mut re = json_error_t::default();
            let cj = p.c.json_loadb(std::ptr::null(), 0, flags, &mut ce);
            let rj = p.r.json_loadb(std::ptr::null(), 0, flags, &mut re);
            assert!(cj.is_null() && rj.is_null(), "json_loadb(NULL)");
            assert_eq!(ce.snap(), re.snap(), "json_loadb(NULL) error");
            assert_eq!(ce.snap().code, json_error_invalid_argument);
            assert_eq!(ce.snap().source, b"<buffer>".to_vec());
            // Also with a nonzero buflen.
            let mut ce = json_error_t::default();
            let mut re = json_error_t::default();
            p.c.json_loadb(std::ptr::null(), 10, flags, &mut ce);
            p.r.json_loadb(std::ptr::null(), 10, flags, &mut re);
            assert_eq!(ce.snap(), re.snap(), "json_loadb(NULL, 10) error");

            // M26: json_loadf(NULL)
            let mut ce = json_error_t::default();
            let mut re = json_error_t::default();
            let cj = p.c.json_loadf(std::ptr::null_mut(), flags, &mut ce);
            let rj = p.r.json_loadf(std::ptr::null_mut(), flags, &mut re);
            assert!(cj.is_null() && rj.is_null(), "json_loadf(NULL)");
            assert_eq!(ce.snap(), re.snap(), "json_loadf(NULL) error");
            assert_eq!(ce.snap().code, json_error_invalid_argument);
            assert_eq!(ce.snap().source, b"<stream>".to_vec());

            // M28: json_loadfd(negative fd)
            for fd in [-1i32, -2, i32::MIN] {
                let mut ce = json_error_t::default();
                let mut re = json_error_t::default();
                let cj = p.c.json_loadfd(fd, flags, &mut ce);
                let rj = p.r.json_loadfd(fd, flags, &mut re);
                assert!(cj.is_null() && rj.is_null(), "json_loadfd({fd})");
                assert_eq!(ce.snap(), re.snap(), "json_loadfd({fd}) error");
                assert_eq!(ce.snap().code, json_error_invalid_argument);
            }
            // A valid-looking but closed / bogus fd: read() fails -> EOF -> parse error
            for fd in [100000i32, i32::MAX] {
                let mut ce = json_error_t::default();
                let mut re = json_error_t::default();
                let cj = p.c.json_loadfd(fd, flags, &mut ce);
                let rj = p.r.json_loadfd(fd, flags, &mut re);
                assert_eq!(cj.is_null(), rj.is_null(), "json_loadfd({fd})");
                assert_eq!(ce.snap(), re.snap(), "json_loadfd({fd}) error");
                p.c.json_decref(cj);
                p.r.json_decref(rj);
            }

            // M31: json_load_file(NULL path)
            let mut ce = json_error_t::default();
            let mut re = json_error_t::default();
            let cj = p.c.json_load_file(std::ptr::null(), flags, &mut ce);
            let rj = p.r.json_load_file(std::ptr::null(), flags, &mut re);
            assert!(cj.is_null() && rj.is_null(), "json_load_file(NULL)");
            assert_eq!(ce.snap(), re.snap(), "json_load_file(NULL) error");
            assert_eq!(ce.snap().code, json_error_invalid_argument);

            // M32: json_load_file with an unopenable path
            // NOTE: "/" is deliberately excluded — `fopen("/", "rb")` SUCCEEDS
            // on Linux, so the failure surfaces later as a read error (which
            // `fd_get_func`/`fgetc` cannot distinguish from EOF) rather than as
            // json_error_cannot_open_file. It is covered by the
            // `directory_paths` loop below instead.
            for path in [
                "/nonexistent-dir-xyz/file.json",
                "/definitely/not/here",
                "",
                "/proc/self/mem/nope",
            ] {
                let ps = cs(path);
                let mut ce = json_error_t::default();
                let mut re = json_error_t::default();
                let cj = p.c.json_load_file(ps.as_ptr(), flags, &mut ce);
                let rj = p.r.json_load_file(ps.as_ptr(), flags, &mut re);
                assert!(cj.is_null() && rj.is_null(), "json_load_file({path:?})");
                assert_eq!(
                    ce.snap(),
                    re.snap(),
                    "json_load_file({path:?}) error — the text embeds strerror(errno)"
                );
                assert_eq!(
                    ce.snap().code, json_error_cannot_open_file,
                    "json_load_file({path:?}) code"
                );
            }
            // Paths that DO open but cannot be read as JSON: whatever the C
            // reports, the Rust must report identically.
            for path in ["/", "/tmp", "/proc/self"] {
                let ps = cs(path);
                let mut ce = json_error_t::default();
                let mut re = json_error_t::default();
                let cj = p.c.json_load_file(ps.as_ptr(), flags, &mut ce);
                let rj = p.r.json_load_file(ps.as_ptr(), flags, &mut re);
                assert_eq!(cj.is_null(), rj.is_null(), "json_load_file({path:?})");
                assert_eq!(ce.snap(), re.snap(), "json_load_file({path:?}) error");
                p.c.json_decref(cj);
                p.r.json_decref(rj);
            }

            // M33: json_load_callback(NULL callback)
            let mut ce = json_error_t::default();
            let mut re = json_error_t::default();
            let cj = p.c.json_load_callback(None, std::ptr::null_mut(), flags, &mut ce);
            let rj = p.r.json_load_callback(None, std::ptr::null_mut(), flags, &mut re);
            assert!(cj.is_null() && rj.is_null(), "json_load_callback(NULL)");
            assert_eq!(ce.snap(), re.snap(), "json_load_callback(NULL) error");
            assert_eq!(ce.snap().code, json_error_invalid_argument);
            assert_eq!(ce.snap().source, b"<callback>".to_vec());
        }
    }
}

/// M36, M37, M38: EOF-producing edge cases — embedded NUL in `json_loads`,
/// `buflen == 0`, and an exhausted buffer.
#[test]
fn m36_m38_eof_edge_cases() {
    unsafe {
        let p = pair();
        for flags in [0usize, JSON_DECODE_ANY, JSON_DISABLE_EOF_CHECK, 31] {
            // M38: buflen == 0
            let mut ce = json_error_t::default();
            let mut re = json_error_t::default();
            let buf = b"[1]";
            let cj = p.c.json_loadb(buf.as_ptr() as *const c_char, 0, flags, &mut ce);
            let rj = p.r.json_loadb(buf.as_ptr() as *const c_char, 0, flags, &mut re);
            assert_eq!(cj.is_null(), rj.is_null(), "json_loadb(buflen 0)");
            assert_eq!(ce.snap(), re.snap(), "json_loadb(buflen 0) error");
            p.c.json_decref(cj);
            p.r.json_decref(rj);

            // M36: json_loads stops at the first NUL
            for s in [
                b"\x00[1]".to_vec(),
                b"[1\x00]".to_vec(),
                b"[\x001]".to_vec(),
                b"[1]\x00garbage".to_vec(),
                b"\x00".to_vec(),
            ] {
                let z = raw_z(&s);
                let mut ce = json_error_t::default();
                let mut re = json_error_t::default();
                let cj = p.c.json_loads(z.as_ptr() as *const c_char, flags, &mut ce);
                let rj = p.r.json_loads(z.as_ptr() as *const c_char, flags, &mut re);
                assert_eq!(
                    cj.is_null(),
                    rj.is_null(),
                    "json_loads({:?})",
                    String::from_utf8_lossy(&s)
                );
                assert_eq!(
                    ce.snap(),
                    re.snap(),
                    "json_loads({:?}) error",
                    String::from_utf8_lossy(&s)
                );
                if !cj.is_null() {
                    assert_eq!(p.c.dumps(cj, R_ANY), p.r.dumps(rj, R_ANY));
                }
                p.c.json_decref(cj);
                p.r.json_decref(rj);
            }
        }
    }
}

// ===========================================================================
// N, O — pack_unpack.c
// ===========================================================================

/// N9, N10, N13, N22, N23, N25: pack format-string errors, by exact code.
#[test]
fn n_pack_format_errors() {
    unsafe {
        let p = pair();
        let cpk = p.c.json_pack_ex_sym();
        let rpk = p.r.json_pack_ex_sym();
        // (format, expected code)
        let cases: Vec<(&str, u8)> = vec![
            // N23: NULL / empty format
            ("", json_error_invalid_argument),
            // N9/N13: unterminated container
            ("{", json_error_invalid_format),
            ("[", json_error_invalid_format),
            ("{s:i", json_error_invalid_format),
            ("[i", json_error_invalid_format),
            ("[[", json_error_invalid_format),
            ("{s:{", json_error_invalid_format),
            // N10: key position is not `s`
            ("{i:i}", json_error_invalid_format),
            ("{n:i}", json_error_invalid_format),
            ("{[]:i}", json_error_invalid_format),
            ("{{}:i}", json_error_invalid_format),
            ("{b:i}", json_error_invalid_format),
            // N22: unknown format char in value position
            ("x", json_error_invalid_format),
            ("F", json_error_invalid_format),
            ("}", json_error_invalid_format),
            ("]", json_error_invalid_format),
            ("#", json_error_invalid_format),
            ("%", json_error_invalid_format),
            ("+", json_error_invalid_format),
            ("!", json_error_invalid_format),
            ("?", json_error_invalid_format),
            ("*", json_error_invalid_format),
            ("[x]", json_error_invalid_format),
            ("[F]", json_error_invalid_format),
            ("[!]", json_error_invalid_format),
            ("{s:x}", json_error_invalid_format),
            ("{s:F}", json_error_invalid_format),
            // N25: garbage after the top-level value
            ("[]{}", json_error_invalid_format),
            ("nn", json_error_invalid_format),
            ("[]x", json_error_invalid_format),
            ("{}]", json_error_invalid_format),
            ("n]", json_error_invalid_format),
            // N4: optional combined with a length/concat modifier
            ("s?#", json_error_invalid_format),
            ("s?%", json_error_invalid_format),
            ("s?+", json_error_invalid_format),
            ("s*#", json_error_invalid_format),
            ("s*%", json_error_invalid_format),
            ("s*+", json_error_invalid_format),
        ];
        for (fmt, code) in cases {
            let f = cs(fmt);
            let sv = cs("v");
            let kk = cs("k");
            for &fl in &[0usize, JSON_VALIDATE_ONLY, JSON_STRICT] {
                let mut ce = json_error_t::default();
                let mut re = json_error_t::default();
                // Supply a generous, identically-typed argument list; the format
                // errors above are all detected before the extras are read.
                let cj = cpk(
                    &mut ce, fl, f.as_ptr(), kk.as_ptr(), sv.as_ptr(), sv.as_ptr(), sv.as_ptr(),
                );
                let rj = rpk(
                    &mut re, fl, f.as_ptr(), kk.as_ptr(), sv.as_ptr(), sv.as_ptr(), sv.as_ptr(),
                );
                let ctx = format!("json_pack_ex({fmt:?}, flags 0x{fl:x})");
                assert!(cj.is_null(), "{ctx}: C should have rejected");
                assert!(rj.is_null(), "{ctx}: Rust accepted what C rejected");
                assert_eq!(ce.snap(), re.snap(), "{ctx}: json_error_t");
                assert_eq!(ce.snap().code, code, "{ctx}: expected code {code}");
            }
        }
        // N23: literal NULL format pointer
        for &fl in &[0usize, JSON_STRICT] {
            let mut ce = json_error_t::default();
            let mut re = json_error_t::default();
            let cj = cpk(&mut ce, fl, std::ptr::null::<c_char>());
            let rj = rpk(&mut re, fl, std::ptr::null::<c_char>());
            assert!(cj.is_null() && rj.is_null(), "json_pack_ex(NULL fmt)");
            assert_eq!(ce.snap(), re.snap(), "json_pack_ex(NULL fmt) error");
            assert_eq!(ce.snap().code, json_error_invalid_argument);
        }
        // A NULL error out-param must be tolerated.
        for fmt in ["", "x", "{", "[]{}"] {
            let f = cs(fmt);
            let cj = cpk(std::ptr::null_mut(), 0, f.as_ptr());
            let rj = rpk(std::ptr::null_mut(), 0, f.as_ptr());
            assert_eq!(
                cj.is_null(),
                rj.is_null(),
                "json_pack_ex({fmt:?}, NULL error)"
            );
        }
    }
}

/// N1, N3, N8, N11, N17, N21: pack *argument* errors, by exact code.
#[test]
fn n_pack_argument_errors() {
    unsafe {
        let p = pair();
        let cpk = p.c.json_pack_ex_sym();
        let rpk = p.r.json_pack_ex_sym();
        let nilp: *const c_char = std::ptr::null();

        // N1: NULL string argument for a non-optional `s`
        for fmt in ["s", "[s]", "{s:s}"] {
            let f = cs(fmt);
            let kk = cs("k");
            let mut ce = json_error_t::default();
            let mut re = json_error_t::default();
            let (cj, rj) = if fmt.starts_with('{') {
                (
                    cpk(&mut ce, 0, f.as_ptr(), kk.as_ptr(), nilp),
                    rpk(&mut re, 0, f.as_ptr(), kk.as_ptr(), nilp),
                )
            } else {
                (
                    cpk(&mut ce, 0, f.as_ptr(), nilp),
                    rpk(&mut re, 0, f.as_ptr(), nilp),
                )
            };
            assert!(cj.is_null() && rj.is_null(), "pack({fmt:?}, NULL string)");
            assert_eq!(ce.snap(), re.snap(), "pack({fmt:?}, NULL string) error");
            assert_eq!(ce.snap().code, json_error_null_value);
        }
        // N1: NULL object KEY -> "NULL object key"
        let f = cs("{s:i}");
        let mut ce = json_error_t::default();
        let mut re = json_error_t::default();
        let cj = cpk(&mut ce, 0, f.as_ptr(), nilp, 1i32);
        let rj = rpk(&mut re, 0, f.as_ptr(), nilp, 1i32);
        assert!(cj.is_null() && rj.is_null(), "pack({{s:i}}, NULL key)");
        assert_eq!(ce.snap(), re.snap());
        assert_eq!(ce.snap().code, json_error_null_value);

        // N3: invalid UTF-8 in the `s` argument
        for bad in [
            vec![0x80u8, 0],
            vec![0xFFu8, 0],
            vec![0xC0u8, 0x80, 0],
            vec![0xC3u8, 0],
            vec![0xEDu8, 0xA0, 0x80, 0],
        ] {
            for fmt in ["s", "[s]", "{s:s}"] {
                let f = cs(fmt);
                let kk = cs("k");
                let a = bad.as_ptr() as *const c_char;
                let mut ce = json_error_t::default();
                let mut re = json_error_t::default();
                let (cj, rj) = if fmt.starts_with('{') {
                    (
                        cpk(&mut ce, 0, f.as_ptr(), kk.as_ptr(), a),
                        rpk(&mut re, 0, f.as_ptr(), kk.as_ptr(), a),
                    )
                } else {
                    (cpk(&mut ce, 0, f.as_ptr(), a), rpk(&mut re, 0, f.as_ptr(), a))
                };
                assert!(cj.is_null() && rj.is_null(), "pack({fmt:?}, {bad:02x?})");
                assert_eq!(ce.snap(), re.snap(), "pack({fmt:?}, {bad:02x?}) error");
                assert_eq!(ce.snap().code, json_error_invalid_utf8);
            }
            // Invalid-UTF-8 object KEY
            let f = cs("{s:i}");
            let mut ce = json_error_t::default();
            let mut re = json_error_t::default();
            let cj = cpk(&mut ce, 0, f.as_ptr(), bad.as_ptr() as *const c_char, 1i32);
            let rj = rpk(&mut re, 0, f.as_ptr(), bad.as_ptr() as *const c_char, 1i32);
            assert!(cj.is_null() && rj.is_null(), "pack({{s:i}}, bad key)");
            assert_eq!(ce.snap(), re.snap());
            assert_eq!(ce.snap().code, json_error_invalid_utf8);
        }

        // N8: a length/concat that SPLITS a UTF-8 sequence
        {
            let f = cs("s#");
            let a = cs("é"); // 2 bytes
            let mut ce = json_error_t::default();
            let mut re = json_error_t::default();
            let cj = cpk(&mut ce, 0, f.as_ptr(), a.as_ptr(), 1i32);
            let rj = rpk(&mut re, 0, f.as_ptr(), a.as_ptr(), 1i32);
            assert!(cj.is_null() && rj.is_null(), "pack(s#, \"é\", 1)");
            assert_eq!(ce.snap(), re.snap());
            assert_eq!(ce.snap().code, json_error_invalid_utf8);
        }
        {
            // Two halves of "é" concatenated ARE valid; each alone is not.
            let f = cs("s#+#");
            let h1: [u8; 2] = [0xC3, 0];
            let h2: [u8; 2] = [0xA9, 0];
            let mut ce = json_error_t::default();
            let mut re = json_error_t::default();
            let cj = cpk(
                &mut ce, 0, f.as_ptr(), h1.as_ptr() as *const c_char, 1i32,
                h2.as_ptr() as *const c_char, 1i32,
            );
            let rj = rpk(
                &mut re, 0, f.as_ptr(), h1.as_ptr() as *const c_char, 1i32,
                h2.as_ptr() as *const c_char, 1i32,
            );
            assert_eq!(cj.is_null(), rj.is_null(), "pack(s#+#, split é)");
            assert_eq!(ce.snap(), re.snap());
            assert!(!cj.is_null(), "C must accept the JOINED valid sequence");
            p.c.json_decref(cj);
            p.r.json_decref(rj);
        }
        // N6: a NULL piece inside the `+` loop
        {
            let f = cs("s+");
            let a = cs("x");
            let mut ce = json_error_t::default();
            let mut re = json_error_t::default();
            let cj = cpk(&mut ce, 0, f.as_ptr(), a.as_ptr(), nilp);
            let rj = rpk(&mut re, 0, f.as_ptr(), a.as_ptr(), nilp);
            assert!(cj.is_null() && rj.is_null(), "pack(s+, x, NULL)");
            assert_eq!(ce.snap(), re.snap());
            assert_eq!(ce.snap().code, json_error_null_value);
            // NULL as the FIRST piece too.
            let mut ce = json_error_t::default();
            let mut re = json_error_t::default();
            let cj = cpk(&mut ce, 0, f.as_ptr(), nilp, a.as_ptr());
            let rj = rpk(&mut re, 0, f.as_ptr(), nilp, a.as_ptr());
            assert!(cj.is_null() && rj.is_null(), "pack(s+, NULL, x)");
            assert_eq!(ce.snap(), re.snap());
        }

        // N11/N17: NULL json for `o`/`O` without `?`/`*`
        for fmt in ["o", "O", "[o]", "[O]", "{s:o}", "{s:O}"] {
            let f = cs(fmt);
            let kk = cs("k");
            let nilj: json_ptr = std::ptr::null_mut();
            let mut ce = json_error_t::default();
            let mut re = json_error_t::default();
            let (cj, rj) = if fmt.starts_with('{') {
                (
                    cpk(&mut ce, 0, f.as_ptr(), kk.as_ptr(), nilj),
                    rpk(&mut re, 0, f.as_ptr(), kk.as_ptr(), nilj),
                )
            } else {
                (
                    cpk(&mut ce, 0, f.as_ptr(), nilj),
                    rpk(&mut re, 0, f.as_ptr(), nilj),
                )
            };
            assert!(cj.is_null() && rj.is_null(), "pack({fmt:?}, NULL json)");
            assert_eq!(ce.snap(), re.snap(), "pack({fmt:?}, NULL json) error");
            assert_eq!(ce.snap().code, json_error_null_value);
        }
        // N2/N18: with `?` the NULL becomes json null; with `*` the item is omitted.
        for (fmt, ok) in [
            ("o?", true), ("O?", true), ("[o?]", true), ("[O*]", true),
            ("{s:o?}", true), ("{s:O*}", true), ("s?", true), ("[s*]", true),
        ] {
            let f = cs(fmt);
            let kk = cs("k");
            let nilj: json_ptr = std::ptr::null_mut();
            let mut ce = json_error_t::default();
            let mut re = json_error_t::default();
            let (cj, rj) = if fmt.starts_with('{') {
                (
                    cpk(&mut ce, 0, f.as_ptr(), kk.as_ptr(), nilj),
                    rpk(&mut re, 0, f.as_ptr(), kk.as_ptr(), nilj),
                )
            } else {
                (
                    cpk(&mut ce, 0, f.as_ptr(), nilj),
                    rpk(&mut re, 0, f.as_ptr(), nilj),
                )
            };
            assert_eq!(ce.snap(), re.snap(), "pack({fmt:?}, NULL) error");
            assert_eq!(cj.is_null(), rj.is_null(), "pack({fmt:?}, NULL) NULL-ness");
            if ok && !fmt.starts_with('o') && !fmt.starts_with('O') && !fmt.starts_with('s') {
                assert!(!cj.is_null(), "C: {fmt:?} with NULL must still build");
            }
            if !cj.is_null() {
                assert_eq!(p.c.dumps(cj, R_ANY), p.r.dumps(rj, R_ANY), "pack({fmt:?})");
            }
            p.c.json_decref(cj);
            p.r.json_decref(rj);
        }

        // N21: `f` with a non-finite double
        for v in [f64::NAN, -f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            for fmt in ["f", "[f]", "{s:f}"] {
                let f = cs(fmt);
                let kk = cs("k");
                let mut ce = json_error_t::default();
                let mut re = json_error_t::default();
                let (cj, rj) = if fmt.starts_with('{') {
                    (
                        cpk(&mut ce, 0, f.as_ptr(), kk.as_ptr(), v),
                        rpk(&mut re, 0, f.as_ptr(), kk.as_ptr(), v),
                    )
                } else {
                    (cpk(&mut ce, 0, f.as_ptr(), v), rpk(&mut re, 0, f.as_ptr(), v))
                };
                assert!(cj.is_null() && rj.is_null(), "pack({fmt:?}, {v:?})");
                assert_eq!(ce.snap(), re.snap(), "pack({fmt:?}, {v:?}) error");
                assert_eq!(ce.snap().code, json_error_numeric_overflow);
            }
        }
    }
}

/// O2..O7, O9..O14, O16..O26, O28, O29, O31: unpack rejections, by exact code.
#[test]
fn o_unpack_rejections() {
    unsafe {
        let p = pair();
        let cun = p.c.json_unpack_ex_sym();
        let run = p.r.json_unpack_ex_sym();

        // O28: NULL root
        for fmt in ["n", "i", "[i]", "{s:i}", ""] {
            let f = cs(fmt);
            let mut cv: i64 = 0;
            let mut rv: i64 = 0;
            let kk = cs("k");
            let mut ce = json_error_t::default();
            let mut re = json_error_t::default();
            let cr = cun(
                std::ptr::null_mut(), &mut ce, 0, f.as_ptr(), kk.as_ptr(),
                &mut cv as *mut i64,
            );
            let rr = run(
                std::ptr::null_mut(), &mut re, 0, f.as_ptr(), kk.as_ptr(),
                &mut rv as *mut i64,
            );
            assert_eq!(cr, rr, "unpack(NULL root, {fmt:?})");
            assert_eq!(cr, -1, "C must reject a NULL root");
            assert_eq!(ce.snap(), re.snap(), "unpack(NULL root, {fmt:?}) error");
            assert_eq!(ce.snap().code, json_error_null_value);
            assert_eq!(ce.snap().source, b"<root>".to_vec());
        }

        // O29: NULL / empty format
        let ci = p.c.json_integer(1);
        let ri = p.r.json_integer(1);
        for fp in [std::ptr::null::<c_char>(), cs("").as_ptr()] {
            let mut ce = json_error_t::default();
            let mut re = json_error_t::default();
            let cr = cun(ci, &mut ce, 0, fp);
            let rr = run(ri, &mut re, 0, fp);
            assert_eq!(cr, rr, "unpack(empty/NULL fmt)");
            assert_eq!(cr, -1);
            assert_eq!(ce.snap(), re.snap());
            assert_eq!(ce.snap().code, json_error_invalid_argument);
        }
        p.c.json_decref(ci);
        p.r.json_decref(ri);

        // Format errors and wrong-type errors against a chosen root.
        // (root node, format, expected code)
        let cases: Vec<(Node, &str, u8)> = vec![
            // O2: `{` on a non-object
            (Node::Arr(vec![]), "{s:i}", json_error_wrong_type),
            (Node::Int(1), "{s:i}", json_error_wrong_type),
            (Node::Str(b"s".to_vec()), "{s:i}", json_error_wrong_type),
            (Node::Null, "{s:i}", json_error_wrong_type),
            (Node::True, "{s:i}", json_error_wrong_type),
            // O10: `[` on a non-array
            (Node::Obj(vec![]), "[i]", json_error_wrong_type),
            (Node::Int(1), "[i]", json_error_wrong_type),
            (Node::Real(1.0), "[]", json_error_wrong_type),
            // O3: tokens after `!` / `*` inside an object. The root must
            // CONTAIN key "k", otherwise json_error_item_not_found is raised
            // first and the format error is never reached.
            (
                Node::Obj(vec![(b"k".to_vec(), Node::Int(1))]),
                "{s:i!s:i}",
                json_error_invalid_format,
            ),
            (
                Node::Obj(vec![(b"k".to_vec(), Node::Int(1))]),
                "{s:i*s:i}",
                json_error_invalid_format,
            ),
            (Node::Obj(vec![]), "{!s:i}", json_error_invalid_format),
            // O4: format ends before `}`
            (Node::Obj(vec![]), "{", json_error_invalid_format),
            // key must be present, else json_error_item_not_found fires first
            (
                Node::Obj(vec![(b"k".to_vec(), Node::Int(1))]),
                "{s:i",
                json_error_invalid_format,
            ),
            // O5: key position not `s`
            (Node::Obj(vec![]), "{i:i}", json_error_invalid_format),
            (Node::Obj(vec![]), "{n:i}", json_error_invalid_format),
            (Node::Obj(vec![]), "{[]:i}", json_error_invalid_format),
            // O11: tokens after `!` / `*` inside an array. The array must have
            // an element, otherwise json_error_index_out_of_range fires first.
            (Node::Arr(vec![Node::Int(1)]), "[i!i]", json_error_invalid_format),
            (Node::Arr(vec![Node::Int(1)]), "[i*i]", json_error_invalid_format),
            (Node::Arr(vec![]), "[!i]", json_error_invalid_format),
            // O12: format ends before `]`
            (Node::Arr(vec![]), "[", json_error_invalid_format),
            // must have an element, else json_error_index_out_of_range fires first
            (Node::Arr(vec![Node::Int(1)]), "[i", json_error_invalid_format),
            // O13: element char not in "{[siIbfFOon"
            (Node::Arr(vec![Node::Int(1)]), "[x]", json_error_invalid_format),
            (Node::Arr(vec![Node::Int(1)]), "[#]", json_error_invalid_format),
            (Node::Arr(vec![Node::Int(1)]), "[%]", json_error_invalid_format),
            (Node::Arr(vec![Node::Int(1)]), "[+]", json_error_invalid_format),
            (Node::Arr(vec![Node::Int(1)]), "[?]", json_error_invalid_format),
            // O26: unknown char at value position
            (Node::Int(1), "x", json_error_invalid_format),
            (Node::Int(1), "}", json_error_invalid_format),
            (Node::Int(1), "]", json_error_invalid_format),
            (Node::Int(1), "#", json_error_invalid_format),
            (Node::Int(1), "+", json_error_invalid_format),
            (
                Node::Obj(vec![(b"k".to_vec(), Node::Int(1))]),
                "{s:x}",
                json_error_invalid_format,
            ),
            (
                Node::Obj(vec![(b"k".to_vec(), Node::Int(1))]),
                "{s:#}",
                json_error_invalid_format,
            ),
            // O31: garbage after the top-level value
            (Node::Arr(vec![Node::Int(1)]), "[i]x", json_error_invalid_format),
            (Node::Int(1), "ii", json_error_invalid_format),
            (Node::Arr(vec![]), "[]]", json_error_invalid_format),
            // O17/O20..O25: wrong type per scalar spec
            (Node::Int(1), "s", json_error_wrong_type),
            (Node::Null, "s", json_error_wrong_type),
            (Node::Arr(vec![]), "s", json_error_wrong_type),
            (Node::Str(b"s".to_vec()), "i", json_error_wrong_type),
            (Node::Real(1.0), "i", json_error_wrong_type),
            (Node::True, "i", json_error_wrong_type),
            (Node::Real(1.0), "I", json_error_wrong_type),
            (Node::Int(1), "b", json_error_wrong_type),
            (Node::Null, "b", json_error_wrong_type),
            // O23: `f` REJECTS an integer
            (Node::Int(1), "f", json_error_wrong_type),
            (Node::Str(b"1".to_vec()), "f", json_error_wrong_type),
            // O24: `F` rejects a non-number
            (Node::Str(b"1".to_vec()), "F", json_error_wrong_type),
            (Node::True, "F", json_error_wrong_type),
            (Node::Null, "F", json_error_wrong_type),
            // O25: `n` rejects a non-null
            (Node::Int(1), "n", json_error_wrong_type),
            (Node::False, "n", json_error_wrong_type),
            (Node::Arr(vec![]), "n", json_error_wrong_type),
            // O7: required key absent
            (Node::Obj(vec![]), "{s:i}", json_error_item_not_found),
            (
                Node::Obj(vec![(b"other".to_vec(), Node::Int(1))]),
                "{s:i}",
                json_error_item_not_found,
            ),
            // O14: more format items than array elements
            (Node::Arr(vec![]), "[i]", json_error_index_out_of_range),
            (Node::Arr(vec![Node::Int(1)]), "[i,i]", json_error_index_out_of_range),
            (Node::Arr(vec![Node::Int(1)]), "[i,i,i]", json_error_index_out_of_range),
        ];

        for (node, fmt, code) in cases {
            let cj = node.build(&p.c);
            let rj = node.build(&p.r);
            let f = cs(fmt);
            let kk = cs("k");
            for &fl in &[0usize, JSON_VALIDATE_ONLY, JSON_STRICT, 3] {
                let validate = fl & JSON_VALIDATE_ONLY != 0;
                let mut cv: i64 = 0;
                let mut rv: i64 = 0;
                let mut ce = json_error_t::default();
                let mut re = json_error_t::default();
                let (cr, rr) = if validate {
                    (
                        cun(cj, &mut ce, fl, f.as_ptr(), kk.as_ptr()),
                        run(rj, &mut re, fl, f.as_ptr(), kk.as_ptr()),
                    )
                } else {
                    (
                        cun(cj, &mut ce, fl, f.as_ptr(), kk.as_ptr(), &mut cv as *mut i64),
                        run(rj, &mut re, fl, f.as_ptr(), kk.as_ptr(), &mut rv as *mut i64),
                    )
                };
                let ctx = format!("unpack({fmt:?}, {node:?}, flags 0x{fl:x})");
                assert_eq!(cr, rr, "{ctx}");
                assert_eq!(cr, -1, "{ctx}: C should have rejected this");
                assert_eq!(ce.snap(), re.snap(), "{ctx}: json_error_t");
                assert_eq!(ce.snap().code, code, "{ctx}: expected code {code}");
            }
            p.c.json_decref(cj);
            p.r.json_decref(rj);
        }

        // O18/O19: NULL string / length target
        {
            let cj = p.c.json_string(cs("v").as_ptr());
            let rj = p.r.json_string(cs("v").as_ptr());
            let f = cs("s");
            let mut ce = json_error_t::default();
            let mut re = json_error_t::default();
            let cr = cun(cj, &mut ce, 0, f.as_ptr(), std::ptr::null_mut::<*const c_char>());
            let rr = run(rj, &mut re, 0, f.as_ptr(), std::ptr::null_mut::<*const c_char>());
            assert_eq!(cr, rr, "unpack('s', NULL target)");
            assert_eq!(cr, -1);
            assert_eq!(ce.snap(), re.snap());
            assert_eq!(ce.snap().code, json_error_null_value);
            // O19: NULL length target for 's%'
            let f = cs("s%");
            let mut cp_: *const c_char = std::ptr::null();
            let mut rp_: *const c_char = std::ptr::null();
            let mut ce = json_error_t::default();
            let mut re = json_error_t::default();
            let cr = cun(
                cj, &mut ce, 0, f.as_ptr(), &mut cp_ as *mut *const c_char,
                std::ptr::null_mut::<usize>(),
            );
            let rr = run(
                rj, &mut re, 0, f.as_ptr(), &mut rp_ as *mut *const c_char,
                std::ptr::null_mut::<usize>(),
            );
            assert_eq!(cr, rr, "unpack('s%', NULL length target)");
            assert_eq!(cr, -1);
            assert_eq!(ce.snap(), re.snap());
            assert_eq!(ce.snap().code, json_error_null_value);
            p.c.json_decref(cj);
            p.r.json_decref(rj);
        }
        // O6: NULL object key argument
        {
            let cj = p.c.json_object();
            let rj = p.r.json_object();
            let f = cs("{s:i}");
            let mut cv: i64 = 0;
            let mut rv: i64 = 0;
            let mut ce = json_error_t::default();
            let mut re = json_error_t::default();
            let cr = cun(
                cj, &mut ce, 0, f.as_ptr(), std::ptr::null::<c_char>(),
                &mut cv as *mut i64,
            );
            let rr = run(
                rj, &mut re, 0, f.as_ptr(), std::ptr::null::<c_char>(),
                &mut rv as *mut i64,
            );
            assert_eq!(cr, rr, "unpack({{s:i}}, NULL key)");
            assert_eq!(cr, -1);
            assert_eq!(ce.snap(), re.snap());
            assert_eq!(ce.snap().code, json_error_null_value);
            p.c.json_decref(cj);
            p.r.json_decref(rj);
        }

        // O9: strict object with leftover keys; O16: strict array too long.
        {
            let node = Node::Obj(vec![
                (b"a".to_vec(), Node::Int(1)),
                (b"b".to_vec(), Node::Int(2)),
                (b"c".to_vec(), Node::Int(3)),
            ]);
            let cj = node.build(&p.c);
            let rj = node.build(&p.r);
            for (fmt, fl) in [
                ("{s:i!}", 0usize),
                ("{s:i}", JSON_STRICT),
                ("{s?i!}", 0),
                ("{s?i}", JSON_STRICT),
            ] {
                let f = cs(fmt);
                let kk = cs("a");
                let mut cv: i64 = 0;
                let mut rv: i64 = 0;
                let mut ce = json_error_t::default();
                let mut re = json_error_t::default();
                let cr = cun(cj, &mut ce, fl, f.as_ptr(), kk.as_ptr(), &mut cv as *mut i64);
                let rr = run(rj, &mut re, fl, f.as_ptr(), kk.as_ptr(), &mut rv as *mut i64);
                assert_eq!(cr, rr, "strict unpack({fmt:?}, flags 0x{fl:x})");
                assert_eq!(cr, -1, "C must report leftover keys");
                assert_eq!(ce.snap(), re.snap(), "strict unpack({fmt:?}) error");
                assert_eq!(ce.snap().code, json_error_end_of_input_expected);
            }
            p.c.json_decref(cj);
            p.r.json_decref(rj);

            let node = Node::Arr(vec![Node::Int(1), Node::Int(2), Node::Int(3)]);
            let cj = node.build(&p.c);
            let rj = node.build(&p.r);
            for (fmt, fl) in [("[i!]", 0usize), ("[i]", JSON_STRICT), ("[!]", 0), ("[]", JSON_STRICT)] {
                let f = cs(fmt);
                let mut cv: i64 = 0;
                let mut rv: i64 = 0;
                let mut ce = json_error_t::default();
                let mut re = json_error_t::default();
                let n = fmt.matches('i').count();
                let (cr, rr) = if n == 0 {
                    (
                        cun(cj, &mut ce, fl, f.as_ptr()),
                        run(rj, &mut re, fl, f.as_ptr()),
                    )
                } else {
                    (
                        cun(cj, &mut ce, fl, f.as_ptr(), &mut cv as *mut i64),
                        run(rj, &mut re, fl, f.as_ptr(), &mut rv as *mut i64),
                    )
                };
                assert_eq!(cr, rr, "strict unpack({fmt:?}, flags 0x{fl:x})");
                assert_eq!(cr, -1, "C must report leftover array items");
                assert_eq!(ce.snap(), re.snap(), "strict unpack({fmt:?}) error");
                assert_eq!(ce.snap().code, json_error_end_of_input_expected);
            }
            p.c.json_decref(cj);
            p.r.json_decref(rj);
        }
        // O9: the leftover-key LIST in the message must match, so use enough keys
        // to force multiple names (and a rehash-sized object).
        for n in [2usize, 5, 9, 17] {
            let node = Node::Obj(
                (0..n)
                    .map(|i| (format!("k{i:03}").into_bytes(), Node::Int(i as i64)))
                    .collect(),
            );
            let cj = node.build(&p.c);
            let rj = node.build(&p.r);
            let f = cs("{s:i!}");
            let kk = cs("k000");
            let mut cv: i64 = 0;
            let mut rv: i64 = 0;
            let mut ce = json_error_t::default();
            let mut re = json_error_t::default();
            let cr = cun(cj, &mut ce, 0, f.as_ptr(), kk.as_ptr(), &mut cv as *mut i64);
            let rr = run(rj, &mut re, 0, f.as_ptr(), kk.as_ptr(), &mut rv as *mut i64);
            assert_eq!(cr, rr, "strict unpack with {n} keys");
            assert_eq!(
                ce.snap(),
                re.snap(),
                "strict unpack with {n} keys — the leftover-key list must match"
            );
            p.c.json_decref(cj);
            p.r.json_decref(rj);
        }
        // A NULL error out-param must be tolerated.
        let cj = p.c.json_integer(1);
        let rj = p.r.json_integer(1);
        for fmt in ["s", "x", "{s:i}", ""] {
            let f = cs(fmt);
            let kk = cs("k");
            let mut cv: i64 = 0;
            let mut rv: i64 = 0;
            assert_eq!(
                cun(cj, std::ptr::null_mut(), 0, f.as_ptr(), kk.as_ptr(), &mut cv as *mut i64),
                run(rj, std::ptr::null_mut(), 0, f.as_ptr(), kk.as_ptr(), &mut rv as *mut i64),
                "unpack({fmt:?}, NULL error)"
            );
        }
        p.c.json_decref(cj);
        p.r.json_decref(rj);
    }
}

// ===========================================================================
// P — hashtable_seed.c
// ===========================================================================

/// P5, P6, Q11: `json_object_seed` is one-shot; a `SIZE_MAX` seed is truncated
/// to `uint32_t`.
#[test]
fn p5_p6_q11_object_seed_is_one_shot() {
    unsafe {
        let p = pair();
        let cg: Symbol<*mut u32> = p.c.lib.get(b"hashtable_seed\0").unwrap();
        let rg: Symbol<*mut u32> = p.r.lib.get(b"hashtable_seed\0").unwrap();
        let before = **cg;
        assert_eq!(before, **rg, "seeds agree before");
        assert_ne!(before, 0, "the seed is already set by the harness");
        for seed in [0usize, 1, 0x1234_5678, usize::MAX, usize::MAX - 1, 0x1_0000_0000] {
            p.c.json_object_seed(seed);
            p.r.json_object_seed(seed);
            assert_eq!(**cg, before, "C: json_object_seed({seed}) must be a no-op");
            assert_eq!(**cg, **rg, "seeds agree after json_object_seed({seed})");
        }
    }
}

// ===========================================================================
// Q — generic FFI-boundary boundaries
// ===========================================================================

/// Q2, Q3: flag words with every bit set, and out-of-range indent / precision
/// that wrap to 0.
#[test]
fn q2_q3_extreme_flag_words() {
    unsafe {
        let node = Node::Obj(vec![
            (b"a".to_vec(), Node::Arr(vec![Node::Int(1), Node::Real(1.5)])),
            (b"b".to_vec(), Node::Str("é/\"\\\n".as_bytes().to_vec())),
        ]);
        with_both(&node, |p, cj, rj| {
            let mut flags: Vec<usize> = vec![usize::MAX, usize::MAX - 1, !0usize >> 1, 0];
            // Q3: indent and precision one past the mask
            for n in [31usize, 32, 33, 63, 64] {
                flags.push(n & JSON_MAX_INDENT);
                flags.push(n); // raw, so the extra bits land on other flags
                flags.push(JSON_REAL_PRECISION(n));
                flags.push((n & 0x1F) << 11);
                flags.push(n << 11);
            }
            for f in flags {
                assert_dumps_eq(p, cj, rj, f, "extreme flag word");
                assert_eq!(
                    p.c.json_dumpb(cj, std::ptr::null_mut(), 0, f),
                    p.r.json_dumpb(rj, std::ptr::null_mut(), 0, f),
                    "json_dumpb(flags 0x{f:x})"
                );
            }
        });
        // Q2: decode flags with every bit set.
        let p = pair();
        for doc in [
            &b"[1,2]"[..],
            &b"{\"a\":1}"[..],
            &b"42"[..],
            &b"[\"\\u0000\"]"[..],
            &b"[1] x"[..],
            &b"{\"a\":1,\"a\":2}"[..],
            &b"[99999999999999999999]"[..],
        ] {
            for f in [usize::MAX, usize::MAX - 1, 0x1F, 0xFFFF, !0x1Fusize] {
                let mut ce = json_error_t::default();
                let mut re = json_error_t::default();
                let cj = p.c.json_loadb(doc.as_ptr() as *const c_char, doc.len(), f, &mut ce);
                let rj = p.r.json_loadb(doc.as_ptr() as *const c_char, doc.len(), f, &mut re);
                assert_eq!(
                    cj.is_null(),
                    rj.is_null(),
                    "json_loadb({:?}, flags 0x{f:x})",
                    String::from_utf8_lossy(doc)
                );
                assert_eq!(ce.snap(), re.snap(), "error struct for flags 0x{f:x}");
                if !cj.is_null() {
                    assert_eq!(p.c.dumps(cj, R_ANY), p.r.dumps(rj, R_ANY));
                }
                p.c.json_decref(cj);
                p.r.json_decref(rj);
            }
        }
    }
}

/// Q12: zero-length strings everywhere.
#[test]
fn q12_zero_length_strings() {
    unsafe {
        let p = pair();
        let empty: [u8; 1] = [0];
        let ep = empty.as_ptr() as *const c_char;
        for cj_rj in [
            (p.c.json_stringn(ep, 0), p.r.json_stringn(ep, 0)),
            (p.c.json_stringn_nocheck(ep, 0), p.r.json_stringn_nocheck(ep, 0)),
            (p.c.json_string(ep), p.r.json_string(ep)),
            (p.c.json_string_nocheck(ep), p.r.json_string_nocheck(ep)),
        ] {
            let (cj, rj) = cj_rj;
            assert_eq!(cj.is_null(), rj.is_null(), "empty string constructor");
            assert!(!cj.is_null(), "C must accept an empty string");
            assert_eq!(p.c.json_string_length(cj), p.r.json_string_length(rj));
            assert_eq!(p.c.json_string_length(cj), 0);
            assert_eq!(p.c.dumps(cj, R_ANY), p.r.dumps(rj, R_ANY));
            // Setters to length 0.
            assert_eq!(
                p.c.json_string_setn(cj, ep, 0),
                p.r.json_string_setn(rj, ep, 0)
            );
            assert_eq!(
                p.c.json_string_setn_nocheck(cj, ep, 0),
                p.r.json_string_setn_nocheck(rj, ep, 0)
            );
            p.c.json_decref(cj);
            p.r.json_decref(rj);
        }
        // Zero-length object key.
        let co = p.c.json_object();
        let ro = p.r.json_object();
        assert_eq!(
            p.c.json_object_setn_new_nocheck(co, ep, 0, p.c.json_integer(1)),
            p.r.json_object_setn_new_nocheck(ro, ep, 0, p.r.json_integer(1)),
            "empty object key"
        );
        assert_eq!(
            p.c.json_object_getn(co, ep, 0).is_null(),
            p.r.json_object_getn(ro, ep, 0).is_null()
        );
        assert_eq!(p.c.dumps(co, R_ANY), p.r.dumps(ro, R_ANY));
        p.c.json_decref(co);
        p.r.json_decref(ro);
    }
}
