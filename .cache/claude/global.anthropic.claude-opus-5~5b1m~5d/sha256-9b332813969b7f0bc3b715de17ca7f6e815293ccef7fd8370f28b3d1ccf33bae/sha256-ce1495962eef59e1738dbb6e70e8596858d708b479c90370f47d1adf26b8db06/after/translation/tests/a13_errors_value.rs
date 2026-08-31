//! Phase C — error-path differential tests for `src/value.c`.
//!
//! Complement to `a04_value.rs` (which covers the happy paths): every test here
//! feeds an INVALID input to both libraries and proves they reject it the *same
//! way* — same return sentinel, same container state afterwards, and, for the
//! reference-counting rows, the same number of decrefs on the passed-in value.
//!
//! ERRORS.md rows covered:
//!   1-11, 13-57, 59-71, 74-82, 84-86, 91-94, 96-100, 105-120, 122-125,
//!   336, 337.
//!
//! Row -> test map (a row may be proven by more than one test):
//!   1,2,5,18,20,21,22,24-29,33,36,38,44,48,49,52,56,60,64,66,67,68,78,79,81,
//!   91,92,96,97,100        -> wrong_type_* tests (table-driven, 8 wrong types
//!                             + NULL + 7 out-of-range type tags for EVERY
//!                             `json_is_*`-guarded entry point)
//!   3,4,7,8,9,13,14,16,17,35,39,41,42,43,45,46,47,51,55,59,70,71,74,75,77,80,
//!   82,84,85               -> null_pointer_arguments*, null_key_*
//!   6,19,37                -> missing_key_lookups
//!   34,40                  -> iteration_end_of_object
//!   10,11,23,53,57,61      -> self_insertion_rejected
//!   15,76,86               -> invalid_utf8_checked_vs_nocheck
//!   50,54,62,65            -> array_index_out_of_range
//!   30,31                  -> update_recursive_cycles_and_inner_failure
//!   32,63,69,118,119,125   -> the OOM tests (budgeted allocator)
//!   93,94,98,99            -> real_rejects_nan_and_inf
//!   105,106                -> json_delete_null_and_singletons
//!   107-116                -> json_equal_rejections
//!   117,120                -> copy_and_deep_copy_null
//!   122,123,124            -> deep_copy_indirect_cycles, jsonp_loop_check_*
//!   336,337                -> seed_fallback_without_urandom (subprocess)
//!
//! Two rows outside the assignment fall out for free and are asserted anyway:
//! 83 (`jsonp_strndup` failure — reachable without an allocator hook via the
//! `len + 1` wrap in `strndup_length_overflow_is_rejected`) and 121
//! (`hashtable_init` failure inside `json_deep_copy` — the `budget == 0` step of
//! `oom_budget_sweep_copy_and_deep_copy`).
//!
//! Two techniques recur:
//!
//! * **Refcount witness.** A value handed to a failing mutator must be decref'd
//!   exactly once. Every such test creates the value, increfs it to 2, calls the
//!   function, and asserts the refcount came back as 1 in BOTH libraries — which
//!   proves "decref'd exactly once" without freeing it (so no use-after-free
//!   inside the test) and would catch both a missing and a double decref.
//!
//! * **Out-of-range type tags.** `json_type` is a C enum, so any `int` fits. A
//!   stack `json_t { type_: <invalid>, refcount: (size_t)-1 }` is a legal thing
//!   for a caller to pass: the `(size_t)-1` refcount makes it a never-freed
//!   pseudo-singleton, so it can even be embedded in a real container. Both
//!   libraries must take the same `default:`/`else` arm for it.

mod common;
use common::*;
use std::ffi::{c_char, c_int, c_void};
use std::sync::atomic::{AtomicIsize, AtomicUsize, Ordering as O};

// ---------------------------------------------------------------------------
// Byte strings that print readably in divergence messages
// ---------------------------------------------------------------------------

#[derive(PartialEq, Eq, Clone)]
struct B(Vec<u8>);

impl std::fmt::Debug for B {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "b\"")?;
        for &x in &self.0 {
            match x {
                b'"' => write!(f, "\\\"")?,
                b'\\' => write!(f, "\\\\")?,
                0x20..=0x7e => write!(f, "{}", x as char)?,
                _ => write!(f, "\\x{x:02x}")?,
            }
        }
        write!(f, "\"")
    }
}

const CANON: size_t = JSON_SORT_KEYS | JSON_ENCODE_ANY;
const RAWD: size_t = JSON_ENCODE_ANY;

unsafe fn dumpb(api: &Api, j: *const json_t, flags: size_t) -> Option<B> {
    if j.is_null() {
        return None;
    }
    let p = (api.json_dumps)(j, flags);
    let b = cbytes(p).map(B);
    jfree(api, p as *mut c_void);
    b
}

/// One child of a container, as observed through the public API.
#[derive(Debug, PartialEq, Clone)]
struct Kid {
    key: Option<B>,
    key_len: size_t,
    ty: c_int,
    refcount: size_t,
    dump: Option<B>,
}

/// Everything observable about a value, one level deep. Cyclic graphs are safe:
/// `json_dumps` itself detects loops and returns NULL in both libraries.
#[derive(Debug, PartialEq, Clone)]
struct St {
    ty: c_int,
    refcount: size_t,
    osize: size_t,
    asize: size_t,
    slen: size_t,
    sbytes: Option<B>,
    ival: i64,
    rbits: u64,
    nbits: u64,
    sorted: Option<B>,
    raw: Option<B>,
    kids: Vec<Kid>,
}

unsafe fn state(api: &Api, j: *mut json_t) -> St {
    let ty = if j.is_null() { -100 } else { (*j).type_ };
    let refcount = if j.is_null() { 0 } else { (*j).refcount };
    let slen = (api.json_string_length)(j);
    let sp = (api.json_string_value)(j);
    let sbytes = if sp.is_null() {
        None
    } else {
        Some(B((0..slen).map(|i| *(sp as *const u8).add(i)).collect()))
    };

    let mut kids: Vec<Kid> = Vec::new();
    if ty == JSON_OBJECT {
        let mut it = (api.json_object_iter)(j);
        while !it.is_null() {
            let kp = (api.json_object_iter_key)(it);
            let kl = (api.json_object_iter_key_len)(it);
            let key = if kp.is_null() {
                None
            } else {
                Some(B((0..kl).map(|i| *(kp as *const u8).add(i)).collect()))
            };
            let v = (api.json_object_iter_value)(it);
            kids.push(Kid {
                key,
                key_len: kl,
                ty: if v.is_null() { -100 } else { (*v).type_ },
                refcount: if v.is_null() { 0 } else { (*v).refcount },
                dump: dumpb(api, v, CANON),
            });
            it = (api.json_object_iter_next)(j, it);
        }
    } else if ty == JSON_ARRAY {
        let n = (api.json_array_size)(j);
        for i in 0..n {
            let v = (api.json_array_get)(j, i);
            kids.push(Kid {
                key: None,
                key_len: i,
                ty: if v.is_null() { -100 } else { (*v).type_ },
                refcount: if v.is_null() { 0 } else { (*v).refcount },
                dump: dumpb(api, v, CANON),
            });
        }
    }

    St {
        ty,
        refcount,
        osize: (api.json_object_size)(j),
        asize: (api.json_array_size)(j),
        slen,
        sbytes,
        ival: (api.json_integer_value)(j),
        rbits: (api.json_real_value)(j).to_bits(),
        nbits: (api.json_number_value)(j).to_bits(),
        sorted: dumpb(api, j, CANON),
        raw: dumpb(api, j, RAWD),
        kids,
    }
}

// ---------------------------------------------------------------------------
// The "zoo": one entry per candidate argument, built in BOTH libraries so that
// index `i` denotes the same logical value on each side.
// ---------------------------------------------------------------------------

/// Type tags with no valid `json_type` variant. `256` is deliberate: its low
/// byte is `JSON_OBJECT`, so an implementation that truncated the tag to a byte
/// would treat it as an object and crash instead of returning a sentinel.
const BAD_TAGS: &[c_int] = &[-1, 8, 9, 99, 256, i32::MAX, i32::MIN];

/// NULL is denoted by this pseudo type tag inside the zoo.
const TY_NULLPTR: c_int = -100;

fn make_bads() -> (Vec<Box<json_t>>, Vec<(String, c_int, *mut json_t)>) {
    let mut boxes: Vec<Box<json_t>> = BAD_TAGS
        .iter()
        .map(|&t| {
            Box::new(json_t {
                type_: t,
                // (size_t)-1: the singleton refcount, so json_incref/json_decref
                // are no-ops and nothing can ever free this stack value.
                refcount: usize::MAX,
            })
        })
        .collect();
    let mut out = Vec::new();
    for b in boxes.iter_mut() {
        let ty = b.type_;
        let p: *mut json_t = &mut **b;
        out.push((format!("badtype({ty})"), ty, p));
    }
    (boxes, out)
}

/// All eight real types (plus a couple of interesting shapes), created by `api`.
unsafe fn real_values(api: &Api) -> Vec<(&'static str, c_int, *mut json_t)> {
    let s = cs("s");
    let k = cs("a");
    let o1 = (api.json_object)();
    (api.json_object_set_new)(o1, k.as_ptr(), (api.json_integer)(1));
    let a2 = (api.json_array)();
    (api.json_array_append_new)(a2, (api.json_integer)(1));
    (api.json_array_append_new)(a2, (api.json_true)());
    vec![
        ("object{}", JSON_OBJECT, (api.json_object)()),
        ("object{a:1}", JSON_OBJECT, o1),
        ("array[]", JSON_ARRAY, (api.json_array)()),
        ("array[1,true]", JSON_ARRAY, a2),
        ("string(s)", JSON_STRING, (api.json_string)(s.as_ptr())),
        (
            "string()",
            JSON_STRING,
            (api.json_stringn_nocheck)(s.as_ptr(), 0),
        ),
        ("integer", JSON_INTEGER, (api.json_integer)(7)),
        ("real", JSON_REAL, (api.json_real)(1.5)),
        ("true", JSON_TRUE, (api.json_true)()),
        ("false", JSON_FALSE, (api.json_false)()),
        ("null", JSON_NULL, (api.json_null)()),
    ]
}

struct Zoo {
    names: Vec<String>,
    tys: Vec<c_int>,
    cp: Vec<*mut json_t>,
    rp: Vec<*mut json_t>,
    owned: usize,
}

impl Zoo {
    fn len(&self) -> usize {
        self.names.len()
    }
    unsafe fn release(&self, c: &Api, r: &Api) {
        for i in 0..self.owned {
            decref(c, self.cp[i]);
            decref(r, self.rp[i]);
        }
    }
}

/// Real values (owned, per library) + the shared out-of-range-tag pseudo-values
/// + NULL. `bads` must outlive the returned `Zoo`.
unsafe fn build_zoo(c: &Api, r: &Api, bads: &[(String, c_int, *mut json_t)]) -> Zoo {
    let cv = real_values(c);
    let rv = real_values(r);
    assert_eq!(cv.len(), rv.len());
    let mut z = Zoo {
        names: Vec::new(),
        tys: Vec::new(),
        cp: Vec::new(),
        rp: Vec::new(),
        owned: cv.len(),
    };
    for i in 0..cv.len() {
        assert_eq!(cv[i].0, rv[i].0);
        assert!(!cv[i].2.is_null() && !rv[i].2.is_null(), "zoo build failed");
        z.names.push(cv[i].0.to_string());
        z.tys.push(cv[i].1);
        z.cp.push(cv[i].2);
        z.rp.push(rv[i].2);
    }
    // The bad-tag pseudo-values are plain memory owned by the test, so the very
    // same pointer is handed to both libraries.
    for (name, ty, p) in bads {
        z.names.push(name.clone());
        z.tys.push(*ty);
        z.cp.push(*p);
        z.rp.push(*p);
    }
    z.names.push("NULL".to_string());
    z.tys.push(TY_NULLPTR);
    z.cp.push(std::ptr::null_mut());
    z.rp.push(std::ptr::null_mut());
    z
}

/// A fresh value with refcount 2, so that exactly one decref by the callee is
/// observable as `refcount == 1` without the value being freed.
unsafe fn witness(api: &Api) -> *mut json_t {
    let v = (api.json_integer)(0x5AFE);
    assert!(!v.is_null());
    incref(v);
    assert_eq!((*v).refcount, 2);
    v
}

/// Assert that both libraries decref'd their witness the same number of times,
/// and that the C did exactly `expect` decrefs. Then release the witnesses.
unsafe fn check_witness(
    c: &Api,
    r: &Api,
    cv: *mut json_t,
    rv: *mut json_t,
    expect_refcount: size_t,
    ctx: &str,
) {
    diff_eq!((*cv).refcount, (*rv).refcount, "{}: witness refcount", ctx);
    assert_eq!(
        (*cv).refcount, expect_refcount,
        "C ground truth: {ctx}: witness refcount"
    );
    // Drop the extra reference the test added, then the original one.
    while (*cv).refcount > 1 {
        decref(c, cv);
    }
    while (*rv).refcount > 1 {
        decref(r, rv);
    }
    decref(c, cv);
    decref(r, rv);
}

// ===========================================================================
// Wrong-type read accessors
// ERRORS.md 1, 2, 5, 48, 49, 78, 79, 91, 96, 100, 33, 36
// ===========================================================================

#[test]
fn wrong_type_read_accessors_return_the_same_sentinel() {
    let _g = global_state_lock();
    let (c, r) = both();
    unsafe {
        let (_boxes, bads) = make_bads();
        let z = build_zoo(c, r, &bads);
        let key = cs("a");
        let nokey = cs("zzz");

        for i in 0..z.len() {
            let (n, ty, cv, rv) = (&z.names[i], z.tys[i], z.cp[i], z.rp[i]);

            // --- row 1/2: json_object_size => 0 for anything but an object
            let (cs_, rs_) = ((c.json_object_size)(cv), (r.json_object_size)(rv));
            diff_eq!(cs_, rs_, "json_object_size({n})");
            if ty != JSON_OBJECT {
                assert_eq!(cs_, 0, "C ground truth: json_object_size({n}) must be 0");
            }

            // --- row 48: json_array_size => 0 for anything but an array
            let (ca, ra) = ((c.json_array_size)(cv), (r.json_array_size)(rv));
            diff_eq!(ca, ra, "json_array_size({n})");
            if ty != JSON_ARRAY {
                assert_eq!(ca, 0, "C ground truth: json_array_size({n}) must be 0");
            }

            // --- row 5: json_object_get / json_object_getn => NULL
            for kp in [key.as_ptr(), nokey.as_ptr()] {
                let cg = (c.json_object_get)(cv, kp);
                let rg = (r.json_object_get)(rv, kp);
                diff_eq!(cg.is_null(), rg.is_null(), "json_object_get({n})");
                let cg = (c.json_object_getn)(cv, kp, 1);
                let rg = (r.json_object_getn)(rv, kp, 1);
                diff_eq!(cg.is_null(), rg.is_null(), "json_object_getn({n})");
                if ty != JSON_OBJECT {
                    assert!(cg.is_null(), "C ground truth: json_object_getn({n}) NULL");
                }
            }

            // --- row 49: json_array_get => NULL for a non-array, at any index
            for idx in [0usize, 1, 7, 8, usize::MAX] {
                let cg = (c.json_array_get)(cv, idx);
                let rg = (r.json_array_get)(rv, idx);
                diff_eq!(cg.is_null(), rg.is_null(), "json_array_get({n},{idx})");
                if ty != JSON_ARRAY {
                    assert!(cg.is_null(), "C ground truth: json_array_get({n}) NULL");
                }
            }

            // --- row 78/79: json_string_value => NULL, json_string_length => 0
            let (cvv, rvv) = ((c.json_string_value)(cv), (r.json_string_value)(rv));
            diff_eq!(cbytes(cvv), cbytes(rvv), "json_string_value({n})");
            let (cl, rl) = ((c.json_string_length)(cv), (r.json_string_length)(rv));
            diff_eq!(cl, rl, "json_string_length({n})");
            if ty != JSON_STRING {
                assert!(cvv.is_null(), "C ground truth: json_string_value({n}) NULL");
                assert_eq!(cl, 0, "C ground truth: json_string_length({n}) 0");
            }

            // --- row 91: json_integer_value => 0
            let (ci, ri) = ((c.json_integer_value)(cv), (r.json_integer_value)(rv));
            diff_eq!(ci, ri, "json_integer_value({n})");
            if ty != JSON_INTEGER {
                assert_eq!(ci, 0, "C ground truth: json_integer_value({n}) 0");
            }

            // --- row 96: json_real_value => 0.0 (compared as bits, so a -0.0
            //     divergence could not hide)
            let (cr_, rr_) = ((c.json_real_value)(cv), (r.json_real_value)(rv));
            diff_eq!(cr_.to_bits(), rr_.to_bits(), "json_real_value({n})");
            if ty != JSON_REAL {
                assert_eq!(cr_.to_bits(), 0.0f64.to_bits(), "C: json_real_value({n})");
            }

            // --- row 100: json_number_value => 0.0 for non-numbers
            let (cn, rn) = ((c.json_number_value)(cv), (r.json_number_value)(rv));
            diff_eq!(cn.to_bits(), rn.to_bits(), "json_number_value({n})");
            if ty != JSON_INTEGER && ty != JSON_REAL {
                assert_eq!(cn.to_bits(), 0.0f64.to_bits(), "C: json_number_value({n})");
            }

            // --- row 33/36: json_object_iter / json_object_iter_at => NULL
            let (cit, rit) = ((c.json_object_iter)(cv), (r.json_object_iter)(rv));
            diff_eq!(cit.is_null(), rit.is_null(), "json_object_iter({n})");
            if ty != JSON_OBJECT {
                assert!(cit.is_null(), "C ground truth: json_object_iter({n}) NULL");
            }
            let cia = (c.json_object_iter_at)(cv, key.as_ptr());
            let ria = (r.json_object_iter_at)(rv, key.as_ptr());
            diff_eq!(cia.is_null(), ria.is_null(), "json_object_iter_at({n})");
            if ty != JSON_OBJECT {
                assert!(cia.is_null(), "C: json_object_iter_at({n}) NULL");
            }

            // --- row 38: json_object_iter_next on a non-object => NULL (with a
            //     genuine iterator taken from a real object, so only the type
            //     guard can be what rejects it)
            diff_eq!(
                (c.json_object_iter_next)(cv, (c.json_object_iter)(z.cp[1])).is_null(),
                (r.json_object_iter_next)(rv, (r.json_object_iter)(z.rp[1])).is_null(),
                "json_object_iter_next({n}, iter-of-a-real-object)"
            );
        }
        z.release(c, r);
    }
}

// ===========================================================================
// Wrong-type mutators: same -1, same container state, same decref
// ERRORS.md 10, 18, 20, 44, 52, 56, 60, 64, 66, 81, 92, 97
// ===========================================================================

#[test]
fn wrong_type_mutators_return_minus_one_and_decref_the_value() {
    let _g = global_state_lock();
    let (c, r) = both();
    unsafe {
        let (_boxes, bads) = make_bads();
        let z = build_zoo(c, r, &bads);
        let key = cs("k");
        let val = cs("v");

        for i in 0..z.len() {
            let (n, ty, cv, rv) = (&z.names[i], z.tys[i], z.cp[i], z.rp[i]);
            let before_c = state(c, cv);
            let before_r = state(r, rv);
            diff_eq!(before_c.clone(), before_r, "zoo entry {n} must start identical");

            // ---- row 10: the four json_object_set*_new entry points
            if ty != JSON_OBJECT {
                for which in 0..4 {
                    let (cw, rw) = (witness(c), witness(r));
                    let (cret, rret) = match which {
                        0 => (
                            (c.json_object_set_new)(cv, key.as_ptr(), cw),
                            (r.json_object_set_new)(rv, key.as_ptr(), rw),
                        ),
                        1 => (
                            (c.json_object_setn_new)(cv, key.as_ptr(), 1, cw),
                            (r.json_object_setn_new)(rv, key.as_ptr(), 1, rw),
                        ),
                        2 => (
                            (c.json_object_set_new_nocheck)(cv, key.as_ptr(), cw),
                            (r.json_object_set_new_nocheck)(rv, key.as_ptr(), rw),
                        ),
                        _ => (
                            (c.json_object_setn_new_nocheck)(cv, key.as_ptr(), 1, cw),
                            (r.json_object_setn_new_nocheck)(rv, key.as_ptr(), 1, rw),
                        ),
                    };
                    diff_eq!(cret, rret, "object_set variant {which} on {n} (row 10)");
                    assert_eq!(cret, -1, "C ground truth: set on non-object {n}");
                    check_witness(c, r, cw, rw, 1, &format!("set{which} on {n}"));
                }
            }

            // ---- row 18: json_object_del / deln
            if ty != JSON_OBJECT {
                diff_eq!(
                    (c.json_object_del)(cv, key.as_ptr()),
                    (r.json_object_del)(rv, key.as_ptr()),
                    "json_object_del({n}) (row 18)"
                );
                assert_eq!(
                    (c.json_object_del)(cv, key.as_ptr()),
                    -1,
                    "C ground truth: json_object_del on non-object {n}"
                );
                diff_eq!(
                    (c.json_object_deln)(cv, key.as_ptr(), 1),
                    (r.json_object_deln)(rv, key.as_ptr(), 1),
                    "json_object_deln({n}) (row 18)"
                );
            }

            // ---- row 20: json_object_clear
            if ty != JSON_OBJECT {
                let (cc, rc) = ((c.json_object_clear)(cv), (r.json_object_clear)(rv));
                diff_eq!(cc, rc, "json_object_clear({n}) (row 20)");
                assert_eq!(cc, -1, "C ground truth: clear on non-object {n}");
            }

            // ---- row 44: json_object_iter_set_new with a non-object
            if ty != JSON_OBJECT {
                let cit = (c.json_object_iter)(z.cp[1]);
                let rit = (r.json_object_iter)(z.rp[1]);
                assert!(!cit.is_null() && !rit.is_null());
                let (cw, rw) = (witness(c), witness(r));
                let cret = (c.json_object_iter_set_new)(cv, cit, cw);
                let rret = (r.json_object_iter_set_new)(rv, rit, rw);
                diff_eq!(cret, rret, "json_object_iter_set_new({n}) (row 44)");
                assert_eq!(cret, -1, "C ground truth: iter_set_new on non-object {n}");
                check_witness(c, r, cw, rw, 1, &format!("iter_set_new on {n}"));
            }

            // ---- rows 52/56/60: array set/append/insert on a non-array
            if ty != JSON_ARRAY {
                for which in 0..3 {
                    let (cw, rw) = (witness(c), witness(r));
                    let (cret, rret) = match which {
                        0 => (
                            (c.json_array_set_new)(cv, 0, cw),
                            (r.json_array_set_new)(rv, 0, rw),
                        ),
                        1 => (
                            (c.json_array_append_new)(cv, cw),
                            (r.json_array_append_new)(rv, rw),
                        ),
                        _ => (
                            (c.json_array_insert_new)(cv, 0, cw),
                            (r.json_array_insert_new)(rv, 0, rw),
                        ),
                    };
                    diff_eq!(cret, rret, "array mutator {which} on {n} (rows 52/56/60)");
                    assert_eq!(cret, -1, "C ground truth: array mutator on non-array {n}");
                    check_witness(c, r, cw, rw, 1, &format!("array mut {which} on {n}"));
                }
            }

            // ---- rows 64/66: json_array_remove / json_array_clear
            if ty != JSON_ARRAY {
                for idx in [0usize, 1, usize::MAX] {
                    diff_eq!(
                        (c.json_array_remove)(cv, idx),
                        (r.json_array_remove)(rv, idx),
                        "json_array_remove({n},{idx}) (row 64)"
                    );
                }
                let (cc, rc) = ((c.json_array_clear)(cv), (r.json_array_clear)(rv));
                diff_eq!(cc, rc, "json_array_clear({n}) (row 66)");
                assert_eq!(cc, -1, "C ground truth: json_array_clear on non-array {n}");
            }

            // ---- row 81/86: json_string_set* on a non-string
            if ty != JSON_STRING {
                let (a, b2) = (
                    (c.json_string_set)(cv, val.as_ptr()),
                    (r.json_string_set)(rv, val.as_ptr()),
                );
                diff_eq!(a, b2, "json_string_set({n}) (row 81)");
                assert_eq!(a, -1, "C ground truth: json_string_set on non-string {n}");
                diff_eq!(
                    (c.json_string_setn)(cv, val.as_ptr(), 1),
                    (r.json_string_setn)(rv, val.as_ptr(), 1),
                    "json_string_setn({n})"
                );
                diff_eq!(
                    (c.json_string_set_nocheck)(cv, val.as_ptr()),
                    (r.json_string_set_nocheck)(rv, val.as_ptr()),
                    "json_string_set_nocheck({n})"
                );
                diff_eq!(
                    (c.json_string_setn_nocheck)(cv, val.as_ptr(), 1),
                    (r.json_string_setn_nocheck)(rv, val.as_ptr(), 1),
                    "json_string_setn_nocheck({n}) (row 81)"
                );
            }

            // ---- row 92: json_integer_set on a non-integer
            if ty != JSON_INTEGER {
                let (a, b2) = ((c.json_integer_set)(cv, 9), (r.json_integer_set)(rv, 9));
                diff_eq!(a, b2, "json_integer_set({n}) (row 92)");
                assert_eq!(a, -1, "C ground truth: json_integer_set on {n}");
            }

            // ---- row 97: json_real_set on a non-real
            if ty != JSON_REAL {
                let (a, b2) = ((c.json_real_set)(cv, 2.5), (r.json_real_set)(rv, 2.5));
                diff_eq!(a, b2, "json_real_set({n}) (row 97)");
                assert_eq!(a, -1, "C ground truth: json_real_set on {n}");
            }

            // A rejected mutation must leave the value COMPLETELY unchanged.
            let after_c = state(c, cv);
            let after_r = state(r, rv);
            diff_eq!(after_c.clone(), after_r, "state of {n} after rejected mutations");
            assert_eq!(
                before_c, after_c,
                "C ground truth: rejected mutations must not change {n}"
            );
        }
        z.release(c, r);
    }
}

// ===========================================================================
// Wrong-type binary operations: the update family and json_array_extend
// ERRORS.md 21, 22, 24, 25, 26, 27, 28, 29, 67, 68
// ===========================================================================

#[test]
fn wrong_type_binary_operations_return_minus_one() {
    let _g = global_state_lock();
    let (c, r) = both();
    unsafe {
        let (_boxes, bads) = make_bads();
        let z = build_zoo(c, r, &bads);

        for i in 0..z.len() {
            for j in 0..z.len() {
                let (ni, ti, ci, ri) = (&z.names[i], z.tys[i], z.cp[i], z.rp[i]);
                let (nj, tj, cj, rj) = (&z.names[j], z.tys[j], z.cp[j], z.rp[j]);

                // Rows 21/22/24/25/26/27/28/29 — every update variant needs BOTH
                // arguments to be objects; skip the pair that would succeed.
                if !(ti == JSON_OBJECT && tj == JSON_OBJECT) {
                    let fns: [(&str, usize); 4] = [
                        ("update", 0),
                        ("update_existing", 1),
                        ("update_missing", 2),
                        ("update_recursive", 3),
                    ];
                    for (fname, which) in fns {
                        let (cret, rret) = match which {
                            0 => (
                                (c.json_object_update)(ci, cj),
                                (r.json_object_update)(ri, rj),
                            ),
                            1 => (
                                (c.json_object_update_existing)(ci, cj),
                                (r.json_object_update_existing)(ri, rj),
                            ),
                            2 => (
                                (c.json_object_update_missing)(ci, cj),
                                (r.json_object_update_missing)(ri, rj),
                            ),
                            _ => (
                                (c.json_object_update_recursive)(ci, cj),
                                (r.json_object_update_recursive)(ri, rj),
                            ),
                        };
                        diff_eq!(cret, rret, "json_object_{fname}({ni}, {nj})");
                        assert_eq!(
                            cret, -1,
                            "C ground truth: json_object_{fname}({ni},{nj}) must fail"
                        );
                    }
                }

                // Rows 67/68 — json_array_extend needs both to be arrays.
                if !(ti == JSON_ARRAY && tj == JSON_ARRAY) {
                    let cret = (c.json_array_extend)(ci, cj);
                    let rret = (r.json_array_extend)(ri, rj);
                    diff_eq!(cret, rret, "json_array_extend({ni}, {nj})");
                    assert_eq!(cret, -1, "C: json_array_extend({ni},{nj}) must fail");
                }
            }
        }

        // Nothing may have changed anywhere.
        for i in 0..z.len() {
            diff_eq!(
                state(c, z.cp[i]),
                state(r, z.rp[i]),
                "state of {} after rejected binary ops",
                z.names[i]
            );
        }
        z.release(c, r);
    }
}

// ===========================================================================
// NULL pointer arguments
// ERRORS.md 3, 4, 7, 8, 9, 13, 14, 16, 17, 35, 39, 41, 42, 43, 45, 46, 47,
//           51, 55, 59, 70, 71, 74, 75, 77, 80, 82, 84, 85
// ===========================================================================

#[test]
fn null_key_arguments_are_rejected_and_the_value_is_decrefd() {
    let _g = global_state_lock();
    let (c, r) = both();
    unsafe {
        let nul: *const c_char = std::ptr::null();

        // A real object, plus a non-object, so the NULL-key guard is exercised
        // both before and behind the type guard.
        let co = (c.json_object)();
        let ro = (r.json_object)();

        // --- row 3/4: json_object_get / getn with key == NULL
        for &(cv, rv, what) in &[(co, ro, "object"), (std::ptr::null_mut(), std::ptr::null_mut(), "NULL")] {
            diff_eq!(
                (c.json_object_get)(cv, nul).is_null(),
                (r.json_object_get)(rv, nul).is_null(),
                "json_object_get({what}, NULL key) (row 3)"
            );
            assert!(
                (c.json_object_get)(cv, nul).is_null(),
                "C ground truth: NULL key => NULL"
            );
            for kl in [0usize, 1, usize::MAX] {
                diff_eq!(
                    (c.json_object_getn)(cv, nul, kl).is_null(),
                    (r.json_object_getn)(rv, nul, kl).is_null(),
                    "json_object_getn({what}, NULL key, {kl}) (row 4)"
                );
            }
        }

        // --- rows 7/9/13/14: NULL key in every setter, value must be decref'd
        for which in 0..4 {
            for kl in [0usize, 1, usize::MAX] {
                let (cw, rw) = (witness(c), witness(r));
                let (cret, rret) = match which {
                    0 => (
                        (c.json_object_set_new)(co, nul, cw),
                        (r.json_object_set_new)(ro, nul, rw),
                    ),
                    1 => (
                        (c.json_object_setn_new)(co, nul, kl, cw),
                        (r.json_object_setn_new)(ro, nul, kl, rw),
                    ),
                    2 => (
                        (c.json_object_set_new_nocheck)(co, nul, cw),
                        (r.json_object_set_new_nocheck)(ro, nul, rw),
                    ),
                    _ => (
                        (c.json_object_setn_new_nocheck)(co, nul, kl, cw),
                        (r.json_object_setn_new_nocheck)(ro, nul, kl, rw),
                    ),
                };
                diff_eq!(cret, rret, "setter {which} with NULL key, key_len={kl}");
                assert_eq!(cret, -1, "C ground truth: NULL key must be rejected");
                check_witness(c, r, cw, rw, 1, &format!("NULL-key setter {which}"));
            }
        }

        // --- row 8: value == NULL is checked BEFORE key and type, so a NULL key
        //     AND a NULL value still gives -1 and no crash.
        for kl in [0usize, 5] {
            diff_eq!(
                (c.json_object_setn_new_nocheck)(co, nul, kl, std::ptr::null_mut()),
                (r.json_object_setn_new_nocheck)(ro, nul, kl, std::ptr::null_mut()),
                "setn_new_nocheck(NULL key, NULL value, {kl}) (row 8)"
            );
        }
        let k = cs("k");
        diff_eq!(
            (c.json_object_setn_new_nocheck)(co, k.as_ptr(), 1, std::ptr::null_mut()),
            (r.json_object_setn_new_nocheck)(ro, k.as_ptr(), 1, std::ptr::null_mut()),
            "setn_new_nocheck(valid key, NULL value) (row 8)"
        );
        assert_eq!(
            (c.json_object_setn_new_nocheck)(co, k.as_ptr(), 1, std::ptr::null_mut()),
            -1,
            "C ground truth: NULL value => -1"
        );
        // json_object_set_new / setn_new with a valid key and a NULL value: the
        // UTF-8 / strlen path runs first, then the value check.
        diff_eq!(
            (c.json_object_set_new)(co, k.as_ptr(), std::ptr::null_mut()),
            (r.json_object_set_new)(ro, k.as_ptr(), std::ptr::null_mut()),
            "json_object_set_new(valid key, NULL value)"
        );
        diff_eq!(
            (c.json_object_setn_new)(co, k.as_ptr(), 1, std::ptr::null_mut()),
            (r.json_object_setn_new)(ro, k.as_ptr(), 1, std::ptr::null_mut()),
            "json_object_setn_new(valid key, NULL value)"
        );

        // --- rows 16/17: json_object_del / deln with key == NULL
        diff_eq!(
            (c.json_object_del)(co, nul),
            (r.json_object_del)(ro, nul),
            "json_object_del(NULL key) (row 16)"
        );
        assert_eq!(
            (c.json_object_del)(co, nul),
            -1,
            "C ground truth: del(NULL) => -1"
        );
        for kl in [0usize, 3, usize::MAX] {
            diff_eq!(
                (c.json_object_deln)(co, nul, kl),
                (r.json_object_deln)(ro, nul, kl),
                "json_object_deln(NULL key, {kl}) (row 17)"
            );
        }

        // --- row 35: json_object_iter_at with key == NULL
        diff_eq!(
            (c.json_object_iter_at)(co, nul).is_null(),
            (r.json_object_iter_at)(ro, nul).is_null(),
            "json_object_iter_at(NULL key) (row 35)"
        );
        assert!(
            (c.json_object_iter_at)(co, nul).is_null(),
            "C ground truth: iter_at(NULL) => NULL"
        );

        // --- row 47: json_object_key_to_iter(NULL)
        diff_eq!(
            (c.json_object_key_to_iter)(nul).is_null(),
            (r.json_object_key_to_iter)(nul).is_null(),
            "json_object_key_to_iter(NULL) (row 47)"
        );
        assert!(
            (c.json_object_key_to_iter)(nul).is_null(),
            "C ground truth: key_to_iter(NULL) => NULL"
        );

        // The object must still be empty and identical on both sides.
        diff_eq!(state(c, co), state(r, ro), "object after NULL-key rejections");
        assert_eq!(
            (c.json_object_size)(co),
            0,
            "C ground truth: nothing was inserted"
        );
        decref(c, co);
        decref(r, ro);
    }
}

#[test]
fn null_iter_and_null_value_arguments() {
    let _g = global_state_lock();
    let (c, r) = both();
    unsafe {
        let nulit: *mut c_void = std::ptr::null_mut();

        // --- rows 41/42/43: the iterator accessors with iter == NULL
        diff_eq!(
            cbytes((c.json_object_iter_key)(nulit)),
            cbytes((r.json_object_iter_key)(nulit)),
            "json_object_iter_key(NULL) (row 41)"
        );
        assert!(
            (c.json_object_iter_key)(nulit).is_null(),
            "C ground truth: iter_key(NULL) => NULL"
        );
        let (cl, rl) = (
            (c.json_object_iter_key_len)(nulit),
            (r.json_object_iter_key_len)(nulit),
        );
        diff_eq!(cl, rl, "json_object_iter_key_len(NULL) (row 42)");
        assert_eq!(cl, 0, "C ground truth: iter_key_len(NULL) => 0");
        diff_eq!(
            (c.json_object_iter_value)(nulit).is_null(),
            (r.json_object_iter_value)(nulit).is_null(),
            "json_object_iter_value(NULL) (row 43)"
        );
        assert!(
            (c.json_object_iter_value)(nulit).is_null(),
            "C ground truth: iter_value(NULL) => NULL"
        );

        // --- row 39: json_object_iter_next with iter == NULL on a real object
        let co = (c.json_object)();
        let ro = (r.json_object)();
        let k = cs("a");
        (c.json_object_set_new)(co, k.as_ptr(), (c.json_integer)(1));
        (r.json_object_set_new)(ro, k.as_ptr(), (r.json_integer)(1));
        diff_eq!(
            (c.json_object_iter_next)(co, nulit).is_null(),
            (r.json_object_iter_next)(ro, nulit).is_null(),
            "json_object_iter_next(object, NULL iter) (row 39)"
        );
        assert!(
            (c.json_object_iter_next)(co, nulit).is_null(),
            "C ground truth: iter_next(_, NULL) => NULL"
        );

        // --- row 45: json_object_iter_set_new with iter == NULL, value decref'd
        let (cw, rw) = (witness(c), witness(r));
        let cret = (c.json_object_iter_set_new)(co, nulit, cw);
        let rret = (r.json_object_iter_set_new)(ro, nulit, rw);
        diff_eq!(cret, rret, "json_object_iter_set_new(NULL iter) (row 45)");
        assert_eq!(cret, -1, "C ground truth: NULL iter => -1");
        check_witness(c, r, cw, rw, 1, "iter_set_new with NULL iter");

        // --- row 46: value == NULL (with a perfectly good object and iterator)
        let cit = (c.json_object_iter)(co);
        let rit = (r.json_object_iter)(ro);
        assert!(!cit.is_null() && !rit.is_null());
        let cret = (c.json_object_iter_set_new)(co, cit, std::ptr::null_mut());
        let rret = (r.json_object_iter_set_new)(ro, rit, std::ptr::null_mut());
        diff_eq!(cret, rret, "json_object_iter_set_new(NULL value) (row 46)");
        assert_eq!(cret, -1, "C ground truth: NULL value => -1");
        // ... and the existing entry must survive untouched.
        diff_eq!(state(c, co), state(r, ro), "object after iter_set_new(NULL)");
        assert_eq!((c.json_object_size)(co), 1, "C: entry still there");

        // --- rows 51/55/59: NULL value in the array mutators. `value == NULL` is
        //     checked FIRST, so even a non-array `json` gives -1 with no decref.
        let ca = (c.json_array)();
        let ra = (r.json_array)();
        (c.json_array_append_new)(ca, (c.json_integer)(1));
        (r.json_array_append_new)(ra, (r.json_integer)(1));
        for &(cj, rj, what) in &[
            (ca, ra, "array"),
            (co, ro, "object"),
            (std::ptr::null_mut(), std::ptr::null_mut(), "NULL"),
        ] {
            for idx in [0usize, 1, usize::MAX] {
                diff_eq!(
                    (c.json_array_set_new)(cj, idx, std::ptr::null_mut()),
                    (r.json_array_set_new)(rj, idx, std::ptr::null_mut()),
                    "json_array_set_new({what},{idx},NULL) (row 51)"
                );
                diff_eq!(
                    (c.json_array_insert_new)(cj, idx, std::ptr::null_mut()),
                    (r.json_array_insert_new)(rj, idx, std::ptr::null_mut()),
                    "json_array_insert_new({what},{idx},NULL) (row 59)"
                );
            }
            let cret = (c.json_array_append_new)(cj, std::ptr::null_mut());
            let rret = (r.json_array_append_new)(rj, std::ptr::null_mut());
            diff_eq!(cret, rret, "json_array_append_new({what},NULL) (row 55)");
            assert_eq!(cret, -1, "C ground truth: NULL value => -1");
        }
        diff_eq!(state(c, ca), state(r, ra), "array after NULL-value rejections");
        assert_eq!((c.json_array_size)(ca), 1, "C: array unchanged");

        decref(c, co);
        decref(r, ro);
        decref(c, ca);
        decref(r, ra);
    }
}

#[test]
fn null_string_arguments_are_rejected() {
    let _g = global_state_lock();
    let (c, r) = both();
    unsafe {
        let nul: *const c_char = std::ptr::null();

        // --- rows 70/71/74/75/77: the constructors
        diff_eq!(
            (c.json_string_nocheck)(nul).is_null(),
            (r.json_string_nocheck)(nul).is_null(),
            "json_string_nocheck(NULL) (row 70)"
        );
        diff_eq!(
            (c.json_string)(nul).is_null(),
            (r.json_string)(nul).is_null(),
            "json_string(NULL) (row 74)"
        );
        for len in [0usize, 1, 5, usize::MAX] {
            diff_eq!(
                (c.json_stringn_nocheck)(nul, len).is_null(),
                (r.json_stringn_nocheck)(nul, len).is_null(),
                "json_stringn_nocheck(NULL,{len}) (row 71)"
            );
            diff_eq!(
                (c.json_stringn)(nul, len).is_null(),
                (r.json_stringn)(nul, len).is_null(),
                "json_stringn(NULL,{len}) (row 75)"
            );
            diff_eq!(
                (c.jsonp_stringn_nocheck_own)(nul, len).is_null(),
                (r.jsonp_stringn_nocheck_own)(nul, len).is_null(),
                "jsonp_stringn_nocheck_own(NULL,{len}) (row 77)"
            );
            assert!(
                (c.json_stringn_nocheck)(nul, len).is_null(),
                "C ground truth: NULL value => NULL"
            );
        }

        // --- rows 80/82/84/85: the setters, on a real string so only the NULL
        //     value can be the reason for the rejection. The string must be
        //     left completely untouched.
        let sv = cs("original");
        let cs_ = (c.json_string)(sv.as_ptr());
        let rs_ = (r.json_string)(sv.as_ptr());
        assert!(!cs_.is_null() && !rs_.is_null());
        let before = state(c, cs_);
        for len in [0usize, 1, 8, usize::MAX] {
            let a = (c.json_string_setn_nocheck)(cs_, nul, len);
            let b2 = (r.json_string_setn_nocheck)(rs_, nul, len);
            diff_eq!(a, b2, "json_string_setn_nocheck(NULL,{len}) (row 82)");
            assert_eq!(a, -1, "C ground truth: NULL value => -1");
            diff_eq!(
                (c.json_string_setn)(cs_, nul, len),
                (r.json_string_setn)(rs_, nul, len),
                "json_string_setn(NULL,{len}) (row 85)"
            );
        }
        diff_eq!(
            (c.json_string_set_nocheck)(cs_, nul),
            (r.json_string_set_nocheck)(rs_, nul),
            "json_string_set_nocheck(NULL) (row 80)"
        );
        diff_eq!(
            (c.json_string_set)(cs_, nul),
            (r.json_string_set)(rs_, nul),
            "json_string_set(NULL) (row 84)"
        );
        assert_eq!(
            (c.json_string_set)(cs_, nul),
            -1,
            "C ground truth: set(NULL) => -1"
        );
        diff_eq!(state(c, cs_), state(r, rs_), "string after NULL-value sets");
        assert_eq!(before, state(c, cs_), "C: the string must be unchanged");

        decref(c, cs_);
        decref(r, rs_);
    }
}

#[test]
fn strndup_length_overflow_is_rejected() {
    let _g = global_state_lock();
    // `jsonp_strndup(str, len)` allocates `len + 1`, which WRAPS TO 0 for
    // `len == SIZE_MAX`; `jsonp_malloc(0)` then returns NULL, so the caller sees
    // an allocation failure and never copies anything. This is the only way to
    // reach the `jsonp_strndup` failure path (rows 72/83) without an allocator
    // hook, and a port that used a checked add would abort instead of returning.
    let (c, r) = both();
    unsafe {
        let v = cs("payload");
        for len in [usize::MAX, usize::MAX - 1] {
            // json_stringn_nocheck: string_create -> jsonp_strndup -> NULL
            let cj = (c.json_stringn_nocheck)(v.as_ptr(), len);
            let rj = (r.json_stringn_nocheck)(v.as_ptr(), len);
            diff_eq!(cj.is_null(), rj.is_null(), "json_stringn_nocheck(len={len})");
            if len == usize::MAX {
                assert!(
                    cj.is_null(),
                    "C ground truth: len+1 wraps to 0, so the allocation fails"
                );
            } else {
                // SIZE_MAX-1 asks the allocator for SIZE_MAX bytes, which no
                // allocator can satisfy — also NULL, but by a different route.
                assert!(cj.is_null(), "C ground truth: a SIZE_MAX allocation fails");
            }

            // json_string_setn_nocheck: the target must be left untouched.
            let orig = cs("orig");
            let cstr = (c.json_string)(orig.as_ptr());
            let rstr = (r.json_string)(orig.as_ptr());
            let before = state(c, cstr);
            let a = (c.json_string_setn_nocheck)(cstr, v.as_ptr(), len);
            let b2 = (r.json_string_setn_nocheck)(rstr, v.as_ptr(), len);
            diff_eq!(a, b2, "json_string_setn_nocheck(len={len})");
            assert_eq!(a, -1, "C ground truth: strndup failure => -1 (row 83)");
            diff_eq!(state(c, cstr), state(r, rstr), "string after strndup failure");
            assert_eq!(before, state(c, cstr), "C: the string must be unchanged");
            decref(c, cstr);
            decref(r, rstr);
        }
    }
}

// ===========================================================================
// Lookups that find nothing, and the end of an iteration
// ERRORS.md 6, 19, 37, 34, 40
// ===========================================================================

#[test]
fn missing_key_lookups_and_iteration_ends() {
    let _g = global_state_lock();
    let (c, r) = both();
    unsafe {
        let co = (c.json_object)();
        let ro = (r.json_object)();

        // --- row 34: json_object_iter on an EMPTY object => NULL
        diff_eq!(
            (c.json_object_iter)(co).is_null(),
            (r.json_object_iter)(ro).is_null(),
            "json_object_iter(empty object) (row 34)"
        );
        assert!(
            (c.json_object_iter)(co).is_null(),
            "C ground truth: empty object iter => NULL"
        );

        // Keys that are absent, including tricky ones: empty, embedded NUL, a
        // prefix and an extension of a present key.
        let present = cs("key");
        (c.json_object_set_new)(co, present.as_ptr(), (c.json_integer)(1));
        (r.json_object_set_new)(ro, present.as_ptr(), (r.json_integer)(1));

        let missing: Vec<Vec<u8>> = vec![
            b"".to_vec(),
            b"k".to_vec(),
            b"ke".to_vec(),
            b"keys".to_vec(),
            b"KEY".to_vec(),
            b"\0key".to_vec(),
            b"\xff".to_vec(),
            b"\x00".to_vec(),
        ];
        for m in &missing {
            let buf = cs_bytes(m);
            // row 6: json_object_getn / get
            for kl in [m.len(), 0] {
                if kl == 3 && m.starts_with(b"key") {
                    continue; // would actually find it
                }
                let cg = (c.json_object_getn)(co, buf.as_ptr(), kl);
                let rg = (r.json_object_getn)(ro, buf.as_ptr(), kl);
                diff_eq!(cg.is_null(), rg.is_null(), "getn({m:?},{kl}) (row 6)");
                assert!(cg.is_null(), "C ground truth: getn({m:?},{kl}) => NULL");
            }
            let cg = (c.json_object_get)(co, buf.as_ptr());
            let rg = (r.json_object_get)(ro, buf.as_ptr());
            diff_eq!(cg.is_null(), rg.is_null(), "get({m:?}) (row 6)");

            // row 19: json_object_del / deln on an absent key
            let cd = (c.json_object_del)(co, buf.as_ptr());
            let rd = (r.json_object_del)(ro, buf.as_ptr());
            diff_eq!(cd, rd, "json_object_del({m:?}) (row 19)");
            let cd = (c.json_object_deln)(co, buf.as_ptr(), m.len());
            let rd = (r.json_object_deln)(ro, buf.as_ptr(), m.len());
            diff_eq!(cd, rd, "json_object_deln({m:?}) (row 19)");
            assert_eq!(cd, -1, "C ground truth: deln of a missing key => -1");

            // row 37: json_object_iter_at on an absent key
            let ci = (c.json_object_iter_at)(co, buf.as_ptr());
            let ri = (r.json_object_iter_at)(ro, buf.as_ptr());
            diff_eq!(ci.is_null(), ri.is_null(), "iter_at({m:?}) (row 37)");
        }
        // The present key must have survived every failed lookup and delete.
        diff_eq!(state(c, co), state(r, ro), "object after missing-key lookups");
        assert_eq!((c.json_object_size)(co), 1, "C: 'key' must still be there");

        // The mirror image of row 6: a probe whose bytes BEFORE the NUL spell a
        // present key. The strlen-based entry points truncate and therefore DO
        // find it; the explicit-length ones must not.
        let trunc = cs_bytes(b"key\0extra");
        diff_eq!(
            (c.json_object_get)(co, trunc.as_ptr()).is_null(),
            (r.json_object_get)(ro, trunc.as_ptr()).is_null(),
            "json_object_get(\"key\\0extra\") truncates at the NUL"
        );
        assert!(
            !(c.json_object_get)(co, trunc.as_ptr()).is_null(),
            "C ground truth: get() truncates at the NUL and FINDS 'key'"
        );
        for kl in [4usize, 9] {
            diff_eq!(
                (c.json_object_getn)(co, trunc.as_ptr(), kl).is_null(),
                (r.json_object_getn)(ro, trunc.as_ptr(), kl).is_null(),
                "json_object_getn(\"key\\0extra\", {kl})"
            );
            assert!(
                (c.json_object_getn)(co, trunc.as_ptr(), kl).is_null(),
                "C ground truth: getn({kl}) must not match the 3-byte key"
            );
            diff_eq!(
                (c.json_object_deln)(co, trunc.as_ptr(), kl),
                (r.json_object_deln)(ro, trunc.as_ptr(), kl),
                "json_object_deln(\"key\\0extra\", {kl})"
            );
        }
        assert_eq!((c.json_object_size)(co), 1, "C: still there after deln");
        // ... and the strlen-based delete DOES remove it.
        diff_eq!(
            (c.json_object_del)(co, trunc.as_ptr()),
            (r.json_object_del)(ro, trunc.as_ptr()),
            "json_object_del(\"key\\0extra\") truncates and deletes"
        );
        diff_eq!(state(c, co), state(r, ro), "object after the truncating del");
        assert_eq!((c.json_object_size)(co), 0, "C: del() truncated and removed 'key'");

        // --- row 40: iter_next at the LAST pair => NULL, for sizes 1..6
        for n in 1..7usize {
            let co2 = (c.json_object)();
            let ro2 = (r.json_object)();
            for i in 0..n {
                let k = cs(&format!("k{i}"));
                (c.json_object_set_new)(co2, k.as_ptr(), (c.json_integer)(i as i64));
                (r.json_object_set_new)(ro2, k.as_ptr(), (r.json_integer)(i as i64));
            }
            let mut cit = (c.json_object_iter)(co2);
            let mut rit = (r.json_object_iter)(ro2);
            let mut steps = 0;
            while !cit.is_null() || !rit.is_null() {
                diff_eq!(
                    cit.is_null(),
                    rit.is_null(),
                    "iteration length mismatch at step {steps} (n={n})"
                );
                // Same key at every step: iteration ORDER is part of the contract.
                diff_eq!(
                    cbytes((c.json_object_iter_key)(cit)),
                    cbytes((r.json_object_iter_key)(rit)),
                    "iter key at step {steps} (n={n})"
                );
                cit = (c.json_object_iter_next)(co2, cit);
                rit = (r.json_object_iter_next)(ro2, rit);
                steps += 1;
            }
            assert_eq!(steps, n, "C ground truth: exactly {n} pairs (row 40)");
            // Past the end stays NULL, not garbage.
            diff_eq!(
                (c.json_object_iter_next)(co2, cit).is_null(),
                (r.json_object_iter_next)(ro2, rit).is_null(),
                "iter_next past the end (n={n}) (row 40)"
            );
            decref(c, co2);
            decref(r, ro2);
        }

        decref(c, co);
        decref(r, ro);
    }
}

// ===========================================================================
// Self insertion
// ERRORS.md 11, 23, 53, 57, 61
// ===========================================================================

#[test]
fn self_insertion_rejected_identically() {
    let _g = global_state_lock();
    let (c, r) = both();
    unsafe {
        let k = cs("self");

        // --- row 11: every object setter refuses `json == value`.
        for which in 0..4 {
            let co = (c.json_object)();
            let ro = (r.json_object)();
            // Give the object a refcount of 2 so the decref the C performs is
            // observable rather than fatal.
            incref(co);
            incref(ro);
            let (cret, rret) = match which {
                0 => (
                    (c.json_object_set_new)(co, k.as_ptr(), co),
                    (r.json_object_set_new)(ro, k.as_ptr(), ro),
                ),
                1 => (
                    (c.json_object_setn_new)(co, k.as_ptr(), 4, co),
                    (r.json_object_setn_new)(ro, k.as_ptr(), 4, ro),
                ),
                2 => (
                    (c.json_object_set_new_nocheck)(co, k.as_ptr(), co),
                    (r.json_object_set_new_nocheck)(ro, k.as_ptr(), ro),
                ),
                _ => (
                    (c.json_object_setn_new_nocheck)(co, k.as_ptr(), 4, co),
                    (r.json_object_setn_new_nocheck)(ro, k.as_ptr(), 4, ro),
                ),
            };
            diff_eq!(cret, rret, "self-insertion via setter {which} (row 11)");
            assert_eq!(cret, -1, "C ground truth: self insertion => -1");
            // The self-decref must have happened exactly once on both sides.
            diff_eq!(
                (*co).refcount,
                (*ro).refcount,
                "refcount after self-insertion (setter {which})"
            );
            assert_eq!(
                (*co).refcount, 1,
                "C ground truth: the value was decref'd once"
            );
            diff_eq!(state(c, co), state(r, ro), "object after self-insertion");
            assert_eq!((c.json_object_size)(co), 0, "C: nothing was inserted");
            decref(c, co);
            decref(r, ro);
        }

        // --- rows 53/57/61: the array mutators refuse `json == value`.
        for which in 0..3 {
            let ca = (c.json_array)();
            let ra = (r.json_array)();
            (c.json_array_append_new)(ca, (c.json_integer)(1));
            (r.json_array_append_new)(ra, (r.json_integer)(1));
            incref(ca);
            incref(ra);
            let (cret, rret) = match which {
                0 => (
                    (c.json_array_set_new)(ca, 0, ca),
                    (r.json_array_set_new)(ra, 0, ra),
                ),
                1 => (
                    (c.json_array_append_new)(ca, ca),
                    (r.json_array_append_new)(ra, ra),
                ),
                _ => (
                    (c.json_array_insert_new)(ca, 0, ca),
                    (r.json_array_insert_new)(ra, 0, ra),
                ),
            };
            diff_eq!(cret, rret, "array self-insertion variant {which}");
            assert_eq!(cret, -1, "C ground truth: array self insertion => -1");
            diff_eq!(
                (*ca).refcount,
                (*ra).refcount,
                "refcount after array self-insertion {which}"
            );
            assert_eq!((*ca).refcount, 1, "C: the value was decref'd once");
            diff_eq!(state(c, ca), state(r, ra), "array after self-insertion");
            assert_eq!((c.json_array_size)(ca), 1, "C: array unchanged");
            decref(c, ca);
            decref(r, ra);
        }

        // --- row 23: json_object_update where `other` holds `object` as a
        //     value. The inner json_object_setn_nocheck hits `json == value`,
        //     so the update returns -1 having applied only the keys it reached
        //     before that one. The PARTIAL result is the interesting observable,
        //     and it depends on the iteration order, so it is compared in full.
        for self_at in 0..3usize {
            let co = (c.json_object)();
            let ro = (r.json_object)();
            let cp = (c.json_object)();
            let rp = (r.json_object)();
            for i in 0..3usize {
                let key = cs(&format!("k{i}"));
                if i == self_at {
                    (c.json_object_setn_new_nocheck)(cp, key.as_ptr(), 2, incref(co));
                    (r.json_object_setn_new_nocheck)(rp, key.as_ptr(), 2, incref(ro));
                } else {
                    (c.json_object_setn_new_nocheck)(cp, key.as_ptr(), 2, (c.json_integer)(i as i64));
                    (r.json_object_setn_new_nocheck)(rp, key.as_ptr(), 2, (r.json_integer)(i as i64));
                }
            }
            let cret = (c.json_object_update)(co, cp);
            let rret = (r.json_object_update)(ro, rp);
            diff_eq!(cret, rret, "json_object_update with self at {self_at} (row 23)");
            assert_eq!(cret, -1, "C ground truth: update must fail (row 23)");
            diff_eq!(
                state(c, co),
                state(r, ro),
                "partial update result (self at {self_at}) (row 23)"
            );
            diff_eq!(
                state(c, cp),
                state(r, rp),
                "source object after failed update (self at {self_at})"
            );
            // Break the co <-> cp reference so both can be freed.
            let key = cs(&format!("k{self_at}"));
            (c.json_object_deln)(cp, key.as_ptr(), 2);
            (r.json_object_deln)(rp, key.as_ptr(), 2);
            decref(c, cp);
            decref(r, rp);
            decref(c, co);
            decref(r, ro);
        }
    }
}

// ===========================================================================
// Invalid UTF-8: the checking entry points reject, the `_nocheck` ones accept
// ERRORS.md 15, 76, 86
// ===========================================================================

/// Every interesting class of invalid UTF-8.
fn bad_utf8() -> Vec<(&'static str, Vec<u8>)> {
    vec![
        ("lone continuation 0x80", b"\x80".to_vec()),
        ("lone continuation 0xbf", b"\xbf".to_vec()),
        ("overlong ASCII lead 0xc0", b"\xc0\x80".to_vec()),
        ("overlong ASCII lead 0xc1", b"\xc1\xbf".to_vec()),
        ("truncated 2-byte", b"\xc2".to_vec()),
        ("truncated 3-byte", b"\xe0\xa0".to_vec()),
        ("truncated 4-byte", b"\xf0\x9f\x92".to_vec()),
        ("bad continuation", b"\xc2\x41".to_vec()),
        ("overlong 3-byte", b"\xe0\x80\x80".to_vec()),
        ("overlong 4-byte", b"\xf0\x80\x80\x80".to_vec()),
        ("surrogate D800", b"\xed\xa0\x80".to_vec()),
        ("surrogate DFFF", b"\xed\xbf\xbf".to_vec()),
        ("above U+10FFFF", b"\xf4\x90\x80\x80".to_vec()),
        ("5-byte form 0xf8", b"\xf8\x88\x80\x80\x80".to_vec()),
        ("6-byte form 0xfc", b"\xfc\x84\x80\x80\x80\x80".to_vec()),
        ("0xfe", b"\xfe".to_vec()),
        ("0xff", b"\xff".to_vec()),
        ("valid prefix then 0xff", b"ok\xff".to_vec()),
        ("0xff then valid", b"\xffok".to_vec()),
    ]
}

#[test]
fn invalid_utf8_rejected_by_checking_variants_only() {
    let _g = global_state_lock();
    let (c, r) = both();
    unsafe {
        for (label, bytes) in bad_utf8() {
            let buf = cs_bytes(&bytes);
            let p = buf.as_ptr();
            let n = bytes.len();

            // --- row 76: json_string / json_stringn reject; nocheck accepts.
            let cj = (c.json_string)(p);
            let rj = (r.json_string)(p);
            diff_eq!(cj.is_null(), rj.is_null(), "json_string({label}) (row 76)");
            assert!(cj.is_null(), "C ground truth: json_string({label}) => NULL");
            let cj = (c.json_stringn)(p, n);
            let rj = (r.json_stringn)(p, n);
            diff_eq!(cj.is_null(), rj.is_null(), "json_stringn({label}) (row 76)");
            assert!(cj.is_null(), "C: json_stringn({label}) => NULL");

            let cj = (c.json_string_nocheck)(p);
            let rj = (r.json_string_nocheck)(p);
            diff_eq!(
                cj.is_null(),
                rj.is_null(),
                "json_string_nocheck({label}) must ACCEPT"
            );
            assert!(!cj.is_null(), "C: nocheck must accept {label}");
            diff_eq!(state(c, cj), state(r, rj), "nocheck string({label})");
            decref(c, cj);
            decref(r, rj);
            let cj = (c.json_stringn_nocheck)(p, n);
            let rj = (r.json_stringn_nocheck)(p, n);
            diff_eq!(
                cj.is_null(),
                rj.is_null(),
                "json_stringn_nocheck({label}) must ACCEPT"
            );
            diff_eq!(state(c, cj), state(r, rj), "nocheck stringn({label})");
            decref(c, cj);
            decref(r, rj);

            // --- row 86: json_string_set / setn reject and leave the value
            //     untouched; the nocheck variants replace it.
            let orig = cs("orig");
            let cs_ = (c.json_string)(orig.as_ptr());
            let rs_ = (r.json_string)(orig.as_ptr());
            let before = state(c, cs_);
            let a = (c.json_string_set)(cs_, p);
            let b2 = (r.json_string_set)(rs_, p);
            diff_eq!(a, b2, "json_string_set({label}) (row 86)");
            assert_eq!(a, -1, "C ground truth: json_string_set({label}) => -1");
            let a = (c.json_string_setn)(cs_, p, n);
            let b2 = (r.json_string_setn)(rs_, p, n);
            diff_eq!(a, b2, "json_string_setn({label}) (row 86)");
            diff_eq!(state(c, cs_), state(r, rs_), "string after rejected set");
            assert_eq!(before, state(c, cs_), "C: value unchanged by a rejected set");
            // ... and now the nocheck variant, which must succeed.
            let a = (c.json_string_setn_nocheck)(cs_, p, n);
            let b2 = (r.json_string_setn_nocheck)(rs_, p, n);
            diff_eq!(a, b2, "json_string_setn_nocheck({label}) must ACCEPT");
            assert_eq!(a, 0, "C: setn_nocheck({label}) => 0");
            diff_eq!(state(c, cs_), state(r, rs_), "string after nocheck set");
            decref(c, cs_);
            decref(r, rs_);

            // --- row 15: an invalid-UTF-8 KEY. setn_new / set_new reject (and
            //     decref the value); the nocheck variants accept it.
            let co = (c.json_object)();
            let ro = (r.json_object)();
            for which in 0..2 {
                let (cw, rw) = (witness(c), witness(r));
                let (cret, rret) = if which == 0 {
                    (
                        (c.json_object_set_new)(co, p, cw),
                        (r.json_object_set_new)(ro, p, rw),
                    )
                } else {
                    (
                        (c.json_object_setn_new)(co, p, n, cw),
                        (r.json_object_setn_new)(ro, p, n, rw),
                    )
                };
                diff_eq!(cret, rret, "set_new variant {which} bad key {label} (row 15)");
                assert_eq!(cret, -1, "C ground truth: bad UTF-8 key => -1");
                check_witness(c, r, cw, rw, 1, &format!("bad key {label} v{which}"));
            }
            diff_eq!(
                (c.json_object_size)(co),
                (r.json_object_size)(ro),
                "object size after bad-key rejection ({label})"
            );
            assert_eq!((c.json_object_size)(co), 0, "C: nothing inserted");
            // The nocheck variant MUST accept the same key (row 15's note).
            let cret = (c.json_object_setn_new_nocheck)(co, p, n, (c.json_integer)(1));
            let rret = (r.json_object_setn_new_nocheck)(ro, p, n, (r.json_integer)(1));
            diff_eq!(cret, rret, "setn_new_nocheck with bad key {label}");
            assert_eq!(cret, 0, "C: nocheck accepts the bad key {label}");
            diff_eq!(state(c, co), state(r, ro), "object with a bad-UTF-8 key");
            decref(c, co);
            decref(r, ro);
        }
    }
}

// ===========================================================================
// Embedded NUL: the plain entry points use strlen, the `n` ones do not
// (supporting detail for rows 3/4/6/15/16/17)
// ===========================================================================

#[test]
fn embedded_nul_in_keys_and_values() {
    let _g = global_state_lock();
    let (c, r) = both();
    unsafe {
        let raw = b"a\0b";
        let buf = cs_bytes(raw);
        let p = buf.as_ptr();

        let co = (c.json_object)();
        let ro = (r.json_object)();

        // A 3-byte key containing a NUL. utf8_check_string accepts NUL (it is a
        // 1-byte sequence), so even the CHECKING setter takes it.
        let cret = (c.json_object_setn_new)(co, p, 3, (c.json_integer)(1));
        let rret = (r.json_object_setn_new)(ro, p, 3, (r.json_integer)(1));
        diff_eq!(cret, rret, "setn_new with an embedded-NUL key");
        assert_eq!(cret, 0, "C ground truth: an embedded NUL is valid UTF-8");

        // The strlen-based getter can only see "a", so it must NOT find it.
        diff_eq!(
            (c.json_object_get)(co, p).is_null(),
            (r.json_object_get)(ro, p).is_null(),
            "json_object_get sees only the bytes before the NUL"
        );
        assert!(
            (c.json_object_get)(co, p).is_null(),
            "C ground truth: get() truncates at the NUL and misses"
        );
        // The explicit-length getter finds it.
        diff_eq!(
            (c.json_object_getn)(co, p, 3).is_null(),
            (r.json_object_getn)(ro, p, 3).is_null(),
            "json_object_getn(len=3) finds the embedded-NUL key"
        );
        assert!(
            !(c.json_object_getn)(co, p, 3).is_null(),
            "C ground truth: getn(3) finds it"
        );
        // ... and the strlen-based delete cannot remove it.
        diff_eq!(
            (c.json_object_del)(co, p),
            (r.json_object_del)(ro, p),
            "json_object_del truncates at the NUL"
        );
        assert_eq!(
            (c.json_object_del)(co, p),
            -1,
            "C ground truth: del() cannot see the 3-byte key"
        );
        diff_eq!(state(c, co), state(r, ro), "object with an embedded-NUL key");

        // The 2-byte prefix "a\0" must also miss (partial match).
        for kl in [0usize, 1, 2, 4] {
            diff_eq!(
                (c.json_object_getn)(co, p, kl).is_null(),
                (r.json_object_getn)(ro, p, kl).is_null(),
                "getn(embedded NUL, key_len={kl})"
            );
        }
        // Only the exact length matches.
        assert!(
            (c.json_object_getn)(co, p, 2).is_null(),
            "C ground truth: a 2-byte prefix must not match a 3-byte key"
        );
        diff_eq!(
            (c.json_object_deln)(co, p, 3),
            (r.json_object_deln)(ro, p, 3),
            "deln(3) removes the embedded-NUL key"
        );
        diff_eq!(state(c, co), state(r, ro), "object after deln(3)");
        assert_eq!((c.json_object_size)(co), 0, "C: the key is gone");

        decref(c, co);
        decref(r, ro);

        // A string value containing a NUL: json_string stops at it,
        // json_stringn keeps all 3 bytes. Both must agree byte for byte.
        let c1 = (c.json_string)(p);
        let r1 = (r.json_string)(p);
        diff_eq!(state(c, c1), state(r, r1), "json_string with an embedded NUL");
        assert_eq!((c.json_string_length)(c1), 1, "C: strlen stops at the NUL");
        let c2 = (c.json_stringn)(p, 3);
        let r2 = (r.json_stringn)(p, 3);
        diff_eq!(state(c, c2), state(r, r2), "json_stringn(3) with a NUL");
        assert_eq!((c.json_string_length)(c2), 3, "C: all 3 bytes are kept");
        decref(c, c1);
        decref(r, r1);
        decref(c, c2);
        decref(r, r2);
    }
}

// ===========================================================================
// Out-of-range array indices
// ERRORS.md 50, 54, 62, 65
// ===========================================================================

#[test]
fn array_index_out_of_range() {
    let _g = global_state_lock();
    let (c, r) = both();
    unsafe {
        for n in [0usize, 1, 2, 8, 9] {
            let ca = (c.json_array)();
            let ra = (r.json_array)();
            for i in 0..n {
                (c.json_array_append_new)(ca, (c.json_integer)(i as i64));
                (r.json_array_append_new)(ra, (r.json_integer)(i as i64));
            }
            let before = state(c, ca);
            diff_eq!(before.clone(), state(r, ra), "array of {n} built identically");

            // Indices at, just past, and at the extreme end of the range.
            let idxs: Vec<usize> = vec![n, n + 1, n + 2, usize::MAX, usize::MAX - 1, isize::MAX as usize];
            for &idx in &idxs {
                // row 50: json_array_get out of range => NULL
                let cg = (c.json_array_get)(ca, idx);
                let rg = (r.json_array_get)(ra, idx);
                diff_eq!(cg.is_null(), rg.is_null(), "get(n={n},idx={idx}) (row 50)");
                assert!(cg.is_null(), "C ground truth: get({idx}) out of range => NULL");

                // row 54: json_array_set_new out of range => -1, value decref'd
                let (cw, rw) = (witness(c), witness(r));
                let cret = (c.json_array_set_new)(ca, idx, cw);
                let rret = (r.json_array_set_new)(ra, idx, rw);
                diff_eq!(cret, rret, "set_new(n={n},idx={idx}) (row 54)");
                assert_eq!(cret, -1, "C ground truth: set_new({idx}) => -1");
                check_witness(c, r, cw, rw, 1, &format!("set_new n={n} idx={idx}"));

                // row 65: json_array_remove out of range => -1
                diff_eq!(
                    (c.json_array_remove)(ca, idx),
                    (r.json_array_remove)(ra, idx),
                    "remove(n={n},idx={idx}) (row 65)"
                );
                assert_eq!(
                    (c.json_array_remove)(ca, idx),
                    -1,
                    "C ground truth: remove({idx}) => -1"
                );

                // row 62: insert_new rejects index > entries. index == entries
                // is LEGAL, so it must NOT be rejected — assert that too, then
                // undo it.
                let (cw, rw) = (witness(c), witness(r));
                let cret = (c.json_array_insert_new)(ca, idx, cw);
                let rret = (r.json_array_insert_new)(ra, idx, rw);
                diff_eq!(cret, rret, "insert_new(n={n},idx={idx}) (row 62)");
                if idx == n {
                    assert_eq!(cret, 0, "C ground truth: insert at index==entries is legal");
                    assert_eq!((c.json_array_remove)(ca, idx), 0);
                    assert_eq!((r.json_array_remove)(ra, idx), 0);
                    // The witness was consumed by the array and then released
                    // by the remove, so it is back to the test's single ref.
                    check_witness(c, r, cw, rw, 1, &format!("insert@end n={n}"));
                } else {
                    assert_eq!(cret, -1, "C ground truth: insert({idx}) out of range => -1");
                    check_witness(c, r, cw, rw, 1, &format!("insert n={n} idx={idx}"));
                }
            }
            diff_eq!(state(c, ca), state(r, ra), "array of {n} after bad indices");
            assert_eq!(before, state(c, ca), "C: the array must be unchanged");
            decref(c, ca);
            decref(r, ra);
        }
    }
}

// ===========================================================================
// json_real / json_real_set reject NaN and Inf
// ERRORS.md 93, 94, 98, 99
// ===========================================================================

#[test]
fn real_rejects_nan_and_inf() {
    let _g = global_state_lock();
    let (c, r) = both();
    unsafe {
        // Several distinct NaN encodings (quiet, signalling, negative, payload)
        // and both infinities: `isnan`/`isinf` must reject all of them.
        let bad: Vec<(&str, f64)> = vec![
            ("NaN", f64::NAN),
            ("-NaN", -f64::NAN),
            ("qNaN bits", f64::from_bits(0x7ff8_0000_0000_0000)),
            ("sNaN bits", f64::from_bits(0x7ff0_0000_0000_0001)),
            ("neg qNaN bits", f64::from_bits(0xfff8_0000_0000_0000)),
            ("NaN payload", f64::from_bits(0x7ff8_dead_beef_cafe)),
            ("+inf", f64::INFINITY),
            ("-inf", f64::NEG_INFINITY),
        ];

        for (label, v) in &bad {
            // rows 93/94
            let cj = (c.json_real)(*v);
            let rj = (r.json_real)(*v);
            diff_eq!(cj.is_null(), rj.is_null(), "json_real({label}) (rows 93/94)");
            assert!(cj.is_null(), "C ground truth: json_real({label}) => NULL");

            // rows 98/99 — on a real, the value must be left alone
            let cr_ = (c.json_real)(1.25);
            let rr_ = (r.json_real)(1.25);
            let before = state(c, cr_);
            let a = (c.json_real_set)(cr_, *v);
            let b2 = (r.json_real_set)(rr_, *v);
            diff_eq!(a, b2, "json_real_set({label}) (rows 98/99)");
            assert_eq!(a, -1, "C ground truth: json_real_set({label}) => -1");
            diff_eq!(state(c, cr_), state(r, rr_), "real after rejected set");
            assert_eq!(before, state(c, cr_), "C: 1.25 must be unchanged");
            decref(c, cr_);
            decref(r, rr_);
        }

        // The boundary: the largest finite double and the smallest subnormal
        // must still be ACCEPTED, so the guard is not over-eager.
        for (label, v) in [
            ("MAX", f64::MAX),
            ("-MAX", f64::MIN),
            ("min subnormal", 5e-324),
            ("-0.0", -0.0f64),
        ] {
            let cj = (c.json_real)(v);
            let rj = (r.json_real)(v);
            diff_eq!(cj.is_null(), rj.is_null(), "json_real({label}) must succeed");
            assert!(!cj.is_null(), "C: json_real({label}) must succeed");
            diff_eq!(state(c, cj), state(r, rj), "json_real({label})");
            decref(c, cj);
            decref(r, rj);
        }
    }
}

// ===========================================================================
// json_delete(NULL) and the singletons
// ERRORS.md 105, 106
// ===========================================================================

#[test]
fn json_delete_null_and_singletons_never_free() {
    let _g = global_state_lock();
    let (c, r) = both();
    unsafe {
        // --- row 105: json_delete(NULL) returns immediately.
        for _ in 0..3 {
            (c.json_delete)(std::ptr::null_mut());
            (r.json_delete)(std::ptr::null_mut());
        }

        // --- row 106: json_delete on true/false/null takes the `default:` arm
        //     and frees nothing. Calling it repeatedly must be harmless and the
        //     singletons must stay fully usable afterwards.
        let trip: [(&str, unsafe extern "C" fn() -> *mut json_t, unsafe extern "C" fn() -> *mut json_t, c_int); 3] = [
            ("true", c.json_true, r.json_true, JSON_TRUE),
            ("false", c.json_false, r.json_false, JSON_FALSE),
            ("null", c.json_null, r.json_null, JSON_NULL),
        ];
        for (label, cf, rf, want_ty) in trip {
            let cj = cf();
            let rj = rf();
            assert_eq!((*cj).refcount, usize::MAX, "C: {label} refcount is (size_t)-1");
            diff_eq!(
                ((*cj).type_, (*cj).refcount),
                ((*rj).type_, (*rj).refcount),
                "singleton {label} header"
            );
            assert_eq!((*cj).type_, want_ty, "C: {label} type tag");

            for _ in 0..5 {
                (c.json_delete)(cj);
                (r.json_delete)(rj);
                // incref/decref of a (size_t)-1 refcount must be no-ops.
                incref(cj);
                incref(rj);
                decref(c, cj);
                decref(r, rj);
                diff_eq!(
                    ((*cj).type_, (*cj).refcount),
                    ((*rj).type_, (*rj).refcount),
                    "singleton {label} after delete/incref/decref"
                );
                assert_eq!(
                    ((*cj).type_, (*cj).refcount),
                    (want_ty, usize::MAX),
                    "C: {label} must be untouched"
                );
            }
            // Still the same address, and still usable.
            assert_eq!(cf(), cj, "C: {label} address is stable");
            assert_eq!(rf(), rj, "Rust: {label} address is stable");
            diff_eq!(
                dumpb(c, cj, JSON_ENCODE_ANY),
                dumpb(r, rj, JSON_ENCODE_ANY),
                "singleton {label} still dumps"
            );

            // json_copy of a singleton returns the singleton itself, not a copy.
            let cc = (c.json_copy)(cj);
            let rc = (r.json_copy)(rj);
            assert_eq!(cc, cj, "C: json_copy({label}) returns the singleton");
            assert_eq!(rc, rj, "Rust: json_copy({label}) returns the singleton");
            let cd = (c.json_deep_copy)(cj);
            let rd = (r.json_deep_copy)(rj);
            assert_eq!(cd, cj, "C: json_deep_copy({label}) returns the singleton");
            assert_eq!(rd, rj, "Rust: json_deep_copy({label}) returns the singleton");
            diff_eq!(
                ((*cj).type_, (*cj).refcount),
                ((*rj).type_, (*rj).refcount),
                "singleton {label} after copies"
            );
        }
    }
}

// ===========================================================================
// json_equal rejections
// ERRORS.md 107-116
// ===========================================================================

#[test]
fn json_equal_rejections() {
    let _g = global_state_lock();
    let (c, r) = both();
    unsafe {
        let (_boxes, bads) = make_bads();
        let z = build_zoo(c, r, &bads);

        // --- rows 107/108/109: NULL either side, and every mismatched pair of
        //     types (including the out-of-range tags).
        for i in 0..z.len() {
            for j in 0..z.len() {
                let cret = (c.json_equal)(z.cp[i], z.cp[j]);
                let rret = (r.json_equal)(z.rp[i], z.rp[j]);
                diff_eq!(
                    cret,
                    rret,
                    "json_equal({}, {})",
                    z.names[i],
                    z.names[j]
                );
                let (ti, tj) = (z.tys[i], z.tys[j]);
                if ti == TY_NULLPTR || tj == TY_NULLPTR {
                    assert_eq!(cret, 0, "C ground truth: NULL operand => 0 (rows 107/108)");
                } else if ti != tj {
                    assert_eq!(cret, 0, "C ground truth: type mismatch => 0 (row 109)");
                } else if BAD_TAGS.contains(&ti) && z.cp[i] != z.cp[j] {
                    // Same invalid tag, different pointers: the switch falls to
                    // `default:` and returns 0.
                    assert_eq!(cret, 0, "C ground truth: invalid tag => default arm => 0");
                }
            }
        }

        // --- rows 110/111: objects of different sizes, and same size with a
        //     key that is missing on the other side (the inner
        //     json_equal(value, NULL) is what returns 0).
        let cases: &[(&str, &[(&str, i64)], &[(&str, i64)], c_int)] = &[
            ("size differs", &[("a", 1)], &[("a", 1), ("b", 2)], 0),
            ("size differs (0 vs 1)", &[], &[("a", 1)], 0),
            ("key missing", &[("a", 1), ("b", 2)], &[("a", 1), ("c", 2)], 0),
            ("value differs", &[("a", 1)], &[("a", 2)], 0),
            ("equal", &[("a", 1), ("b", 2)], &[("b", 2), ("a", 1)], 1),
        ];
        for (label, l, rr, want) in cases {
            let mk = |api: &Api, kv: &[(&str, i64)]| -> *mut json_t {
                let o = (api.json_object)();
                for (k, v) in kv {
                    let key = cs(k);
                    (api.json_object_set_new)(o, key.as_ptr(), (api.json_integer)(*v));
                }
                o
            };
            let (c1, c2) = (mk(c, l), mk(c, rr));
            let (r1, r2) = (mk(r, l), mk(r, rr));
            let cret = (c.json_equal)(c1, c2);
            let rret = (r.json_equal)(r1, r2);
            diff_eq!(cret, rret, "json_equal objects [{label}] (rows 110/111)");
            assert_eq!(cret, *want, "C ground truth: objects [{label}]");
            // Symmetry must hold too.
            diff_eq!(
                (c.json_equal)(c2, c1),
                (r.json_equal)(r2, r1),
                "json_equal objects reversed [{label}]"
            );
            decref(c, c1);
            decref(c, c2);
            decref(r, r1);
            decref(r, r2);
        }

        // --- rows 112/113: arrays of different sizes, and equal sizes with one
        //     differing element at every position.
        for n in 0..5usize {
            for m in 0..5usize {
                let mk = |api: &Api, len: usize, diff_at: usize| -> *mut json_t {
                    let a = (api.json_array)();
                    for i in 0..len {
                        let v = if i == diff_at { 999 } else { i as i64 };
                        (api.json_array_append_new)(a, (api.json_integer)(v));
                    }
                    a
                };
                let (c1, r1) = (mk(c, n, usize::MAX), mk(r, n, usize::MAX));
                let (c2, r2) = (mk(c, m, usize::MAX), mk(r, m, usize::MAX));
                let cret = (c.json_equal)(c1, c2);
                let rret = (r.json_equal)(r1, r2);
                diff_eq!(cret, rret, "json_equal arrays {n} vs {m} (row 112)");
                assert_eq!(
                    cret,
                    (n == m) as c_int,
                    "C ground truth: arrays {n} vs {m}"
                );
                decref(c, c2);
                decref(r, r2);
                if n == m {
                    for d in 0..n {
                        let (c3, r3) = (mk(c, n, d), mk(r, n, d));
                        let cret = (c.json_equal)(c1, c3);
                        let rret = (r.json_equal)(r1, r3);
                        diff_eq!(cret, rret, "json_equal arrays differing at {d} (row 113)");
                        assert_eq!(cret, 0, "C ground truth: element {d} differs => 0");
                        decref(c, c3);
                        decref(r, r3);
                    }
                }
                decref(c, c1);
                decref(r, r1);
            }
        }

        // --- row 114: strings differing in length or content, including
        //     embedded NULs and invalid UTF-8 (via the nocheck constructor).
        let strs: &[&[u8]] = &[
            b"", b"a", b"ab", b"b", b"A", b"a\0", b"a\0b", b"\0", b"\xff", b"\xff\xfe",
        ];
        for x in strs {
            for y in strs {
                let bx = cs_bytes(x);
                let by = cs_bytes(y);
                let c1 = (c.json_stringn_nocheck)(bx.as_ptr(), x.len());
                let c2 = (c.json_stringn_nocheck)(by.as_ptr(), y.len());
                let r1 = (r.json_stringn_nocheck)(bx.as_ptr(), x.len());
                let r2 = (r.json_stringn_nocheck)(by.as_ptr(), y.len());
                let cret = (c.json_equal)(c1, c2);
                let rret = (r.json_equal)(r1, r2);
                diff_eq!(cret, rret, "json_equal strings {x:?} vs {y:?} (row 114)");
                assert_eq!(cret, (x == y) as c_int, "C: strings {x:?} vs {y:?}");
                decref(c, c1);
                decref(c, c2);
                decref(r, r1);
                decref(r, r2);
            }
        }

        // --- row 115: integers
        let ints: &[i64] = &[0, 1, -1, i64::MIN, i64::MAX, i32::MAX as i64, 42];
        for &x in ints {
            for &y in ints {
                let c1 = (c.json_integer)(x);
                let c2 = (c.json_integer)(y);
                let r1 = (r.json_integer)(x);
                let r2 = (r.json_integer)(y);
                let cret = (c.json_equal)(c1, c2);
                diff_eq!(cret, (r.json_equal)(r1, r2), "json_equal ints {x} vs {y}");
                assert_eq!(cret, (x == y) as c_int, "C: ints {x} vs {y} (row 115)");
                decref(c, c1);
                decref(c, c2);
                decref(r, r1);
                decref(r, r2);
            }
        }

        // --- row 116: reals. `0.0 == -0.0` in C, so json_equal says EQUAL for
        //     that pair even though the bit patterns differ — a port using bit
        //     comparison would diverge here.
        let reals: &[f64] = &[0.0, -0.0, 1.0, -1.0, 0.1, 1e308, 5e-324, f64::MAX];
        for &x in reals {
            for &y in reals {
                let c1 = (c.json_real)(x);
                let c2 = (c.json_real)(y);
                let r1 = (r.json_real)(x);
                let r2 = (r.json_real)(y);
                let cret = (c.json_equal)(c1, c2);
                diff_eq!(cret, (r.json_equal)(r1, r2), "json_equal reals {x:e}/{y:e}");
                assert_eq!(
                    cret,
                    (x == y) as c_int,
                    "C ground truth: reals {x:e} vs {y:e} (row 116)"
                );
                decref(c, c1);
                decref(c, c2);
                decref(r, r1);
                decref(r, r2);
            }
        }
        assert_eq!(0.0f64, -0.0f64, "sanity: 0.0 == -0.0");

        z.release(c, r);
    }
}

// ===========================================================================
// json_copy / json_deep_copy with NULL and with invalid type tags
// ERRORS.md 117, 120 (+ the `default:` arms)
// ===========================================================================

#[test]
fn copy_and_deep_copy_null_and_invalid_tags() {
    let _g = global_state_lock();
    let (c, r) = both();
    unsafe {
        // --- rows 117/120
        diff_eq!(
            (c.json_copy)(std::ptr::null_mut()).is_null(),
            (r.json_copy)(std::ptr::null_mut()).is_null(),
            "json_copy(NULL) (row 117)"
        );
        assert!(
            (c.json_copy)(std::ptr::null_mut()).is_null(),
            "C ground truth: json_copy(NULL) => NULL"
        );
        diff_eq!(
            (c.json_deep_copy)(std::ptr::null()).is_null(),
            (r.json_deep_copy)(std::ptr::null()).is_null(),
            "json_deep_copy(NULL) (row 120)"
        );
        assert!(
            (c.json_deep_copy)(std::ptr::null()).is_null(),
            "C ground truth: json_deep_copy(NULL) => NULL"
        );

        // do_deep_copy(NULL, parents) — the same guard, reached directly.
        let mut cht = Box::new(hashtable_t::zeroed());
        let mut rht = Box::new(hashtable_t::zeroed());
        assert_eq!((c.hashtable_init)(&mut *cht), 0);
        assert_eq!((r.hashtable_init)(&mut *rht), 0);
        diff_eq!(
            (c.do_deep_copy)(std::ptr::null(), &mut *cht).is_null(),
            (r.do_deep_copy)(std::ptr::null(), &mut *rht).is_null(),
            "do_deep_copy(NULL, parents)"
        );
        (c.hashtable_close)(&mut *cht);
        (r.hashtable_close)(&mut *rht);
    }
}

// ===========================================================================
// Out-of-range type tags crossing the FFI boundary
// (the `default:` arm of every json_typeof switch)
// ===========================================================================

#[test]
fn out_of_range_type_tags_take_the_same_default_arm() {
    let _g = global_state_lock();
    let (c, r) = both();
    unsafe {
        let (_boxes, bads) = make_bads();

        for (name, ty, p) in &bads {
            // json_copy / json_deep_copy / do_deep_copy => NULL
            let cc = (c.json_copy)(*p);
            let rc = (r.json_copy)(*p);
            diff_eq!(cc.is_null(), rc.is_null(), "json_copy({name})");
            assert!(cc.is_null(), "C ground truth: json_copy({name}) => NULL");
            let cd = (c.json_deep_copy)(*p);
            let rd = (r.json_deep_copy)(*p);
            diff_eq!(cd.is_null(), rd.is_null(), "json_deep_copy({name})");
            assert!(cd.is_null(), "C ground truth: json_deep_copy({name}) => NULL");

            // json_dumps => NULL (do_dump's `default: /* not reached */`)
            for flags in [0usize, JSON_ENCODE_ANY, JSON_ENCODE_ANY | JSON_COMPACT] {
                diff_eq!(
                    dumpb(c, *p, flags),
                    dumpb(r, *p, flags),
                    "json_dumps({name}, flags={flags:#x})"
                );
                assert!(
                    dumpb(c, *p, flags).is_none(),
                    "C ground truth: dumping {name} must fail"
                );
            }

            // json_delete must free nothing (the `default:` arm). If either
            // implementation tried to free this stack value the test would
            // crash, which is itself the assertion.
            (c.json_delete)(*p);
            (r.json_delete)(*p);
            assert_eq!(
                ((**p).type_, (**p).refcount),
                (*ty, usize::MAX),
                "the pseudo-value must be untouched by json_delete"
            );

            // Embedded in a REAL array: the container operations must reject it
            // the same way. (refcount == (size_t)-1, so the array's decref on
            // release is a no-op and this is safe.)
            let ca = (c.json_array)();
            let ra = (r.json_array)();
            assert_eq!((c.json_array_append_new)(ca, *p), 0);
            assert_eq!((r.json_array_append_new)(ra, *p), 0);
            diff_eq!(
                (c.json_array_size)(ca),
                (r.json_array_size)(ra),
                "array holding {name}"
            );
            // dump of the container fails
            diff_eq!(
                dumpb(c, ca, JSON_ENCODE_ANY),
                dumpb(r, ra, JSON_ENCODE_ANY),
                "dump of array holding {name}"
            );
            // deep copy of the container fails: do_deep_copy returns NULL, then
            // json_array_append_new(result, NULL) returns -1, so the partially
            // built copy is decref'd and NULL comes back.
            let cdc = (c.json_deep_copy)(ca);
            let rdc = (r.json_deep_copy)(ra);
            diff_eq!(cdc.is_null(), rdc.is_null(), "deep_copy of array+{name}");
            assert!(cdc.is_null(), "C ground truth: deep copy must fail");
            // shallow copy SUCCEEDS (it never inspects the type)
            let csc = (c.json_copy)(ca);
            let rsc = (r.json_copy)(ra);
            diff_eq!(csc.is_null(), rsc.is_null(), "shallow copy of array+{name}");
            assert!(!csc.is_null(), "C ground truth: shallow copy succeeds");
            diff_eq!(
                (c.json_array_size)(csc),
                (r.json_array_size)(rsc),
                "shallow copy size"
            );
            diff_eq!(
                (c.json_equal)(ca, csc),
                (r.json_equal)(ra, rsc),
                "json_equal(array+{name}, its shallow copy)"
            );
            decref(c, csc);
            decref(r, rsc);
            decref(c, ca);
            decref(r, ra);

            // Same in an object.
            let co = (c.json_object)();
            let ro = (r.json_object)();
            let k = cs("bad");
            assert_eq!((c.json_object_set_new)(co, k.as_ptr(), *p), 0);
            assert_eq!((r.json_object_set_new)(ro, k.as_ptr(), *p), 0);
            diff_eq!(
                dumpb(c, co, JSON_ENCODE_ANY),
                dumpb(r, ro, JSON_ENCODE_ANY),
                "dump of object holding {name}"
            );
            let cdc = (c.json_deep_copy)(co);
            let rdc = (r.json_deep_copy)(ro);
            diff_eq!(cdc.is_null(), rdc.is_null(), "deep_copy of object+{name}");
            assert!(cdc.is_null(), "C ground truth: deep copy must fail");
            decref(c, co);
            decref(r, ro);

            // json_object_update from an object holding the bad value: the
            // setter never looks at the value's type, so this SUCCEEDS.
            let co = (c.json_object)();
            let ro = (r.json_object)();
            (c.json_object_set_new)(co, k.as_ptr(), *p);
            (r.json_object_set_new)(ro, k.as_ptr(), *p);
            let cdst = (c.json_object)();
            let rdst = (r.json_object)();
            diff_eq!(
                (c.json_object_update)(cdst, co),
                (r.json_object_update)(rdst, ro),
                "json_object_update from an object holding {name}"
            );
            diff_eq!(
                (c.json_object_size)(cdst),
                (r.json_object_size)(rdst),
                "size after update with {name}"
            );
            // ... but update_recursive must treat it as a non-object leaf.
            let cdst2 = (c.json_object)();
            let rdst2 = (r.json_object)();
            diff_eq!(
                (c.json_object_update_recursive)(cdst2, co),
                (r.json_object_update_recursive)(rdst2, ro),
                "json_object_update_recursive with {name}"
            );
            diff_eq!(
                (c.json_object_size)(cdst2),
                (r.json_object_size)(rdst2),
                "size after recursive update with {name}"
            );
            decref(c, cdst);
            decref(r, rdst);
            decref(c, cdst2);
            decref(r, rdst2);
            decref(c, co);
            decref(r, ro);

            // json_object_size / json_array_size / json_string_* on the bad tag
            diff_eq!(
                (
                    (c.json_object_size)(*p),
                    (c.json_array_size)(*p),
                    (c.json_string_length)(*p),
                    (c.json_integer_value)(*p),
                    (c.json_real_value)(*p).to_bits(),
                    (c.json_number_value)(*p).to_bits(),
                    (c.json_string_value)(*p).is_null(),
                ),
                (
                    (r.json_object_size)(*p),
                    (r.json_array_size)(*p),
                    (r.json_string_length)(*p),
                    (r.json_integer_value)(*p),
                    (r.json_real_value)(*p).to_bits(),
                    (r.json_number_value)(*p).to_bits(),
                    (r.json_string_value)(*p).is_null(),
                ),
                "all typed accessors on {name}"
            );
        }
    }
}

// ===========================================================================
// json_object_update_recursive: cycles and inner failures
// ERRORS.md 30, 31
// ===========================================================================

/// `x = {}`, `y = {"a": x}`, then `x["a"] = y` — an indirect cycle. Both
/// references are owned, so `break_obj_cycle` must be called before releasing.
unsafe fn obj_cycle(api: &Api) -> (*mut json_t, *mut json_t) {
    let x = (api.json_object)();
    let y = (api.json_object)();
    let a = cs("a");
    (api.json_object_setn_new_nocheck)(y, a.as_ptr(), 1, incref(x));
    (api.json_object_setn_new_nocheck)(x, a.as_ptr(), 1, incref(y));
    (x, y)
}

unsafe fn break_obj_cycle(api: &Api, x: *mut json_t, y: *mut json_t) {
    let a = cs("a");
    (api.json_object_deln)(x, a.as_ptr(), 1);
    (api.json_object_deln)(y, a.as_ptr(), 1);
    decref(api, x);
    decref(api, y);
}

#[test]
fn update_recursive_cycle_and_inner_failure() {
    let _g = global_state_lock();
    let (c, r) = both();
    unsafe {
        // --- row 30: `other` reachable from itself along keys that are objects
        //     in `object` too, so jsonp_loop_check finds the pointer again.
        let mk_target = |api: &Api| -> *mut json_t {
            // {"a": {"a": {"a": 1}}}
            let inner = (api.json_object)();
            let a = cs("a");
            (api.json_object_setn_new_nocheck)(inner, a.as_ptr(), 1, (api.json_integer)(1));
            let mid = (api.json_object)();
            (api.json_object_setn_new_nocheck)(mid, a.as_ptr(), 1, inner);
            let top = (api.json_object)();
            (api.json_object_setn_new_nocheck)(top, a.as_ptr(), 1, mid);
            top
        };
        let (cx, cy) = obj_cycle(c);
        let (rx, ry) = obj_cycle(r);
        let cobj = mk_target(c);
        let robj = mk_target(r);
        let cret = (c.json_object_update_recursive)(cobj, cx);
        let rret = (r.json_object_update_recursive)(robj, rx);
        diff_eq!(cret, rret, "update_recursive with a cyclic other (row 30)");
        assert_eq!(cret, -1, "C ground truth: cycle => -1 (row 30)");
        // Whatever partial update happened must be identical.
        diff_eq!(
            (c.json_object_size)(cobj),
            (r.json_object_size)(robj),
            "target size after the cyclic update"
        );
        diff_eq!(
            dumpb(c, cobj, CANON),
            dumpb(r, robj, CANON),
            "target after the cyclic update (row 30)"
        );
        // Running it again must give the same answer (no leftover state in the
        // parents set).
        diff_eq!(
            (c.json_object_update_recursive)(cobj, cx),
            (r.json_object_update_recursive)(robj, rx),
            "update_recursive with a cyclic other, second call"
        );
        // The cyclic pair itself must be unchanged.
        diff_eq!(
            (c.json_object_size)(cx),
            (r.json_object_size)(rx),
            "cyclic x size"
        );
        break_obj_cycle(c, cx, cy);
        break_obj_cycle(r, rx, ry);
        // cobj may now hold a reference into the (broken) cycle; release it.
        decref(c, cobj);
        decref(r, robj);

        // --- row 31: the inner json_object_setn_nocheck fails because `other`
        //     holds `object` itself as a value (self insertion, one level down).
        let co = (c.json_object)();
        let ro = (r.json_object)();
        let cp = (c.json_object)();
        let rp = (r.json_object)();
        let k = cs("k");
        (c.json_object_setn_new_nocheck)(cp, k.as_ptr(), 1, incref(co));
        (r.json_object_setn_new_nocheck)(rp, k.as_ptr(), 1, incref(ro));
        let cret = (c.json_object_update_recursive)(co, cp);
        let rret = (r.json_object_update_recursive)(ro, rp);
        diff_eq!(cret, rret, "update_recursive inner self-insertion (row 31)");
        assert_eq!(cret, -1, "C ground truth: inner failure => -1 (row 31)");
        diff_eq!(state(c, co), state(r, ro), "target after inner failure");
        assert_eq!((c.json_object_size)(co), 0, "C: nothing was inserted");
        (c.json_object_deln)(cp, k.as_ptr(), 1);
        (r.json_object_deln)(rp, k.as_ptr(), 1);
        decref(c, cp);
        decref(r, rp);
        decref(c, co);
        decref(r, ro);
    }
}

// ===========================================================================
// json_deep_copy: INDIRECT cycles are rejected, shared subtrees are not
// ERRORS.md 122, 123
// ===========================================================================

#[test]
fn deep_copy_rejects_indirect_cycles() {
    let _g = global_state_lock();
    let (c, r) = both();
    unsafe {
        // Direct self-insertion is refused by the setters (rows 11/57), so a
        // cycle can only be built INDIRECTLY. Four shapes:
        //   1. a = [b], b = [a]
        //   2. a = {"b": b}, b = {"a": a}
        //   3. a = [b], b = [d], d = [a]           (3-cycle)
        //   4. a = [o], o = {"a": a}               (object <-> array)
        // In each case json_deep_copy must return NULL from BOTH entry points
        // (a and the other node), and the graph must be left intact.

        struct Cyc {
            nodes: Vec<*mut json_t>,
        }

        // shape 1
        let mk1 = |api: &Api| -> Cyc {
            let a = (api.json_array)();
            let b = (api.json_array)();
            (api.json_array_append_new)(a, incref(b));
            (api.json_array_append_new)(b, incref(a));
            Cyc { nodes: vec![a, b] }
        };
        // shape 2
        let mk2 = |api: &Api| -> Cyc {
            let a = (api.json_object)();
            let b = (api.json_object)();
            let ka = cs("a");
            let kb = cs("b");
            (api.json_object_setn_new_nocheck)(a, kb.as_ptr(), 1, incref(b));
            (api.json_object_setn_new_nocheck)(b, ka.as_ptr(), 1, incref(a));
            Cyc { nodes: vec![a, b] }
        };
        // shape 3
        let mk3 = |api: &Api| -> Cyc {
            let a = (api.json_array)();
            let b = (api.json_array)();
            let d = (api.json_array)();
            (api.json_array_append_new)(a, incref(b));
            (api.json_array_append_new)(b, incref(d));
            (api.json_array_append_new)(d, incref(a));
            Cyc {
                nodes: vec![a, b, d],
            }
        };
        // shape 4
        let mk4 = |api: &Api| -> Cyc {
            let a = (api.json_array)();
            let o = (api.json_object)();
            let ka = cs("a");
            (api.json_array_append_new)(a, incref(o));
            (api.json_object_setn_new_nocheck)(o, ka.as_ptr(), 1, incref(a));
            Cyc { nodes: vec![a, o] }
        };

        let makers: Vec<(&str, &dyn Fn(&Api) -> Cyc)> = vec![
            ("a=[b], b=[a]", &mk1),
            ("a={b:b}, b={a:a}", &mk2),
            ("3-cycle of arrays", &mk3),
            ("array <-> object", &mk4),
        ];

        for (label, mk) in makers {
            let cc = mk(c);
            let rc = mk(r);
            assert_eq!(cc.nodes.len(), rc.nodes.len());
            for i in 0..cc.nodes.len() {
                let cd = (c.json_deep_copy)(cc.nodes[i]);
                let rd = (r.json_deep_copy)(rc.nodes[i]);
                diff_eq!(
                    cd.is_null(),
                    rd.is_null(),
                    "json_deep_copy from node {i} of [{label}] (rows 122/123)"
                );
                assert!(
                    cd.is_null(),
                    "C ground truth: a cycle must make deep_copy fail [{label}] node {i}"
                );
                // The refcounts of the whole cycle must be unchanged, i.e. the
                // aborted copy leaked nothing and released nothing extra.
                for j in 0..cc.nodes.len() {
                    diff_eq!(
                        (*cc.nodes[j]).refcount,
                        (*rc.nodes[j]).refcount,
                        "refcount of node {j} after a failed deep copy [{label}]"
                    );
                }
                // Dumping must also fail, and identically.
                diff_eq!(
                    dumpb(c, cc.nodes[i], CANON),
                    dumpb(r, rc.nodes[i], CANON),
                    "dump of a cyclic graph [{label}] node {i}"
                );
                // A second attempt must behave the same (the parents hashtable
                // is per-call, so no state may leak between calls).
                diff_eq!(
                    (c.json_deep_copy)(cc.nodes[i]).is_null(),
                    (r.json_deep_copy)(rc.nodes[i]).is_null(),
                    "second json_deep_copy [{label}] node {i}"
                );
            }
            // Break every edge, then release.
            for i in 0..cc.nodes.len() {
                (c.json_array_clear)(cc.nodes[i]);
                (r.json_array_clear)(rc.nodes[i]);
                (c.json_object_clear)(cc.nodes[i]);
                (r.json_object_clear)(rc.nodes[i]);
            }
            for i in 0..cc.nodes.len() {
                decref(c, cc.nodes[i]);
                decref(r, rc.nodes[i]);
            }
        }

        // --- The negative control: a shared subtree (a DAG, not a cycle) must
        //     still deep-copy SUCCESSFULLY, because jsonp_loop_check removes the
        //     key again on the way out. Without that, a port would wrongly
        //     reject this.
        let mk_dag = |api: &Api| -> *mut json_t {
            let shared = (api.json_array)();
            (api.json_array_append_new)(shared, (api.json_integer)(7));
            let top = (api.json_array)();
            (api.json_array_append_new)(top, incref(shared));
            (api.json_array_append_new)(top, incref(shared));
            let obj = (api.json_object)();
            let k1 = cs("x");
            let k2 = cs("y");
            (api.json_object_setn_new_nocheck)(obj, k1.as_ptr(), 1, incref(shared));
            (api.json_object_setn_new_nocheck)(obj, k2.as_ptr(), 1, incref(shared));
            (api.json_array_append_new)(top, obj);
            decref(api, shared);
            top
        };
        let cdag = mk_dag(c);
        let rdag = mk_dag(r);
        let cd = (c.json_deep_copy)(cdag);
        let rd = (r.json_deep_copy)(rdag);
        diff_eq!(cd.is_null(), rd.is_null(), "deep copy of a shared-subtree DAG");
        assert!(
            !cd.is_null(),
            "C ground truth: a DAG (no cycle) must deep-copy successfully"
        );
        diff_eq!(state(c, cd), state(r, rd), "deep copy of a DAG");
        diff_eq!(
            (c.json_equal)(cdag, cd),
            (r.json_equal)(rdag, rd),
            "the DAG copy must be equal to the original"
        );
        assert_eq!((c.json_equal)(cdag, cd), 1, "C: DAG copy equals the original");
        // The copy must NOT share the sub-array (deep copy duplicates it).
        assert_ne!(
            (c.json_array_get)(cd, 0),
            (c.json_array_get)(cdag, 0),
            "C: deep copy must not share children"
        );
        assert_ne!(
            (r.json_array_get)(rd, 0),
            (r.json_array_get)(rdag, 0),
            "Rust: deep copy must not share children"
        );
        decref(c, cd);
        decref(r, rd);
        decref(c, cdag);
        decref(r, rdag);
    }
}

// ===========================================================================
// jsonp_loop_check
// ERRORS.md 124
// ===========================================================================

#[test]
fn jsonp_loop_check_rejects_a_pointer_already_present() {
    let _g = global_state_lock();
    let (c, r) = both();
    unsafe {
        const KLEN: usize = 2 + std::mem::size_of::<*mut json_t>() * 2 + 1;
        let mut cht = Box::new(hashtable_t::zeroed());
        let mut rht = Box::new(hashtable_t::zeroed());
        assert_eq!((c.hashtable_init)(&mut *cht), 0);
        assert_eq!((r.hashtable_init)(&mut *rht), 0);

        // A pointer value each library formats with "%p". Both libraries use the
        // platform snprintf, so the key bytes must match exactly. NULL is
        // included on purpose: glibc prints it as "(nil)".
        let co = (c.json_object)();
        let ro = (r.json_object)();
        let probes: [(&str, *const json_t, *const json_t); 3] = [
            ("real object", co as *const json_t, ro as *const json_t),
            ("NULL", std::ptr::null(), std::ptr::null()),
            (
                "singleton null",
                (c.json_null)() as *const json_t,
                (c.json_null)() as *const json_t,
            ),
        ];

        for (label, cp, rp) in probes {
            let mut ck = [0xAAu8; 64];
            let mut rk = [0xAAu8; 64];
            let mut clen = usize::MAX;
            let mut rlen = usize::MAX;
            // First call: inserts, returns 0.
            let cret = (c.jsonp_loop_check)(
                &mut *cht,
                cp,
                ck.as_mut_ptr() as *mut c_char,
                KLEN,
                &mut clen,
            );
            let rret = (r.jsonp_loop_check)(
                &mut *rht,
                rp,
                rk.as_mut_ptr() as *mut c_char,
                KLEN,
                &mut rlen,
            );
            diff_eq!(cret, rret, "jsonp_loop_check first call [{label}]");
            diff_eq!(clen, rlen, "jsonp_loop_check *key_len_out [{label}]");
            assert_eq!(cret, 0, "C ground truth: the first insert succeeds");
            if cp == rp {
                // Same pointer on both sides => byte-identical key.
                diff_eq!(B(ck.to_vec()), B(rk.to_vec()), "loop key bytes [{label}]");
            }

            // Second call with the SAME pointer: row 124 => -1.
            let mut clen2 = usize::MAX;
            let mut rlen2 = usize::MAX;
            for attempt in 0..3 {
                let cret = (c.jsonp_loop_check)(
                    &mut *cht,
                    cp,
                    ck.as_mut_ptr() as *mut c_char,
                    KLEN,
                    &mut clen2,
                );
                let rret = (r.jsonp_loop_check)(
                    &mut *rht,
                    rp,
                    rk.as_mut_ptr() as *mut c_char,
                    KLEN,
                    &mut rlen2,
                );
                diff_eq!(cret, rret, "jsonp_loop_check repeat {attempt} [{label}] (row 124)");
                assert_eq!(cret, -1, "C ground truth: already present => -1 (row 124)");
                diff_eq!(clen2, rlen2, "*key_len_out on the reject path [{label}]");
                assert_eq!(clen2, clen, "C: *key_len_out is written before the lookup");
            }

            // key_len_out == NULL must be tolerated on both paths.
            let cret = (c.jsonp_loop_check)(
                &mut *cht,
                cp,
                ck.as_mut_ptr() as *mut c_char,
                KLEN,
                std::ptr::null_mut(),
            );
            let rret = (r.jsonp_loop_check)(
                &mut *rht,
                rp,
                rk.as_mut_ptr() as *mut c_char,
                KLEN,
                std::ptr::null_mut(),
            );
            diff_eq!(cret, rret, "jsonp_loop_check with NULL key_len_out [{label}]");
        }

        (c.hashtable_close)(&mut *cht);
        (r.hashtable_close)(&mut *rht);
        decref(c, co);
        decref(r, ro);
    }
}

// ===========================================================================
// OOM rows: a budgeted allocator installed on BOTH libraries
// ERRORS.md 32, 63, 69, 118, 119, 125
// ===========================================================================

extern "C" {
    fn malloc(n: size_t) -> *mut c_void;
    fn realloc(p: *mut c_void, n: size_t) -> *mut c_void;
    fn free(p: *mut c_void);
}

/// Remaining allocations before failure; `-1` means unlimited. Separate counters
/// per library so that a budget of N means "the Nth allocation of THIS library
/// fails", which is what makes "both fail at the same allocation index" a
/// meaningful assertion.
static C_LEFT: AtomicIsize = AtomicIsize::new(-1);
static R_LEFT: AtomicIsize = AtomicIsize::new(-1);
static C_USED: AtomicUsize = AtomicUsize::new(0);
static R_USED: AtomicUsize = AtomicUsize::new(0);

fn take(left: &AtomicIsize) -> bool {
    let v = left.load(O::Relaxed);
    if v < 0 {
        return true;
    }
    if v == 0 {
        return false;
    }
    left.store(v - 1, O::Relaxed);
    true
}

unsafe extern "C" fn c_malloc(n: size_t) -> *mut c_void {
    C_USED.fetch_add(1, O::Relaxed);
    if !take(&C_LEFT) {
        return std::ptr::null_mut();
    }
    malloc(n)
}
unsafe extern "C" fn c_realloc(p: *mut c_void, n: size_t) -> *mut c_void {
    C_USED.fetch_add(1, O::Relaxed);
    if !take(&C_LEFT) {
        return std::ptr::null_mut();
    }
    realloc(p, n)
}
unsafe extern "C" fn c_free(p: *mut c_void) {
    free(p)
}
unsafe extern "C" fn r_malloc(n: size_t) -> *mut c_void {
    R_USED.fetch_add(1, O::Relaxed);
    if !take(&R_LEFT) {
        return std::ptr::null_mut();
    }
    malloc(n)
}
unsafe extern "C" fn r_realloc(p: *mut c_void, n: size_t) -> *mut c_void {
    R_USED.fetch_add(1, O::Relaxed);
    if !take(&R_LEFT) {
        return std::ptr::null_mut();
    }
    realloc(p, n)
}
unsafe extern "C" fn r_free(p: *mut c_void) {
    free(p)
}

/// Installs the counting/budgeted allocators and restores the originals on drop
/// (including on a panic, so one failing assertion cannot break every later
/// test in the file).
struct Alloc<'a> {
    c: &'a Api,
    r: &'a Api,
    saved: (
        json_malloc_t,
        json_realloc_t,
        json_free_t,
        json_malloc_t,
        json_realloc_t,
        json_free_t,
    ),
}

impl<'a> Alloc<'a> {
    /// `budget < 0` means unlimited (pure counting).
    unsafe fn install(c: &'a Api, r: &'a Api, budget: isize) -> Alloc<'a> {
        let (mut cm, mut crl, mut cf) = (None, None, None);
        let (mut rm, mut rrl, mut rf) = (None, None, None);
        (c.json_get_alloc_funcs2)(&mut cm, &mut crl, &mut cf);
        (r.json_get_alloc_funcs2)(&mut rm, &mut rrl, &mut rf);
        C_LEFT.store(budget, O::Relaxed);
        R_LEFT.store(budget, O::Relaxed);
        C_USED.store(0, O::Relaxed);
        R_USED.store(0, O::Relaxed);
        (c.json_set_alloc_funcs2)(Some(c_malloc), Some(c_realloc), Some(c_free));
        (r.json_set_alloc_funcs2)(Some(r_malloc), Some(r_realloc), Some(r_free));
        Alloc {
            c,
            r,
            saved: (cm, crl, cf, rm, rrl, rf),
        }
    }
    fn used(&self) -> (usize, usize) {
        (C_USED.load(O::Relaxed), R_USED.load(O::Relaxed))
    }
    /// Stop failing allocations (so that results can be inspected/freed) while
    /// keeping the counters.
    fn unlimit(&self) {
        C_LEFT.store(-1, O::Relaxed);
        R_LEFT.store(-1, O::Relaxed);
    }
}

impl Drop for Alloc<'_> {
    fn drop(&mut self) {
        C_LEFT.store(-1, O::Relaxed);
        R_LEFT.store(-1, O::Relaxed);
        unsafe {
            (self.c.json_set_alloc_funcs2)(self.saved.0, self.saved.1, self.saved.2);
            (self.r.json_set_alloc_funcs2)(self.saved.3, self.saved.4, self.saved.5);
        }
    }
}

#[test]
fn oom_budget_sweep_copy_and_deep_copy() {
    let _g = global_state_lock();
    // ERRORS.md 118 (json_object() inside json_object_copy fails) and 119
    // (json_array() inside json_array_copy fails), swept across EVERY
    // allocation the operation makes: for each budget both libraries must fail
    // (or succeed) at the same allocation index, and consume the same number of
    // allocations. That is a much stronger statement than "both return NULL
    // when nothing can be allocated".
    let (c, r) = both();
    unsafe {
        // Trees built with the real allocator first.
        let mk = |api: &Api| -> Vec<(&'static str, *mut json_t)> {
            let o = (api.json_object)();
            for i in 0..4 {
                let k = cs(&format!("k{i}"));
                (api.json_object_set_new)(o, k.as_ptr(), (api.json_integer)(i));
            }
            let a = (api.json_array)();
            for i in 0..4 {
                (api.json_array_append_new)(a, (api.json_integer)(i));
            }
            let nested = (api.json_object)();
            let kk = cs("arr");
            (api.json_object_set_new)(nested, kk.as_ptr(), incref(a));
            let ks = cs("str");
            let sv = cs("hello");
            (api.json_object_set_new)(nested, ks.as_ptr(), (api.json_string)(sv.as_ptr()));
            vec![("object", o), ("array", a), ("nested", nested)]
        };
        let ctrees = mk(c);
        let rtrees = mk(r);

        for t in 0..ctrees.len() {
            let label = ctrees[t].0;
            let (ct, rt) = (ctrees[t].1, rtrees[t].1);

            for &deep in &[false, true] {
                // First, count the allocations an unconstrained run makes.
                let total = {
                    let a = Alloc::install(c, r, -1);
                    let cres = if deep {
                        (c.json_deep_copy)(ct)
                    } else {
                        (c.json_copy)(ct)
                    };
                    let rres = if deep {
                        (r.json_deep_copy)(rt)
                    } else {
                        (r.json_copy)(rt)
                    };
                    assert!(!cres.is_null() && !rres.is_null());
                    let (cu, ru) = a.used();
                    diff_eq!(
                        cu,
                        ru,
                        "allocation COUNT for {} copy of {label}",
                        if deep { "deep" } else { "shallow" }
                    );
                    decref(c, cres);
                    decref(r, rres);
                    cu
                };
                assert!(total > 0, "the copy must allocate something");

                for budget in 0..=(total as isize) {
                    let a = Alloc::install(c, r, budget);
                    let cres = if deep {
                        (c.json_deep_copy)(ct)
                    } else {
                        (c.json_copy)(ct)
                    };
                    let rres = if deep {
                        (r.json_deep_copy)(rt)
                    } else {
                        (r.json_copy)(rt)
                    };
                    let (cu, ru) = a.used();
                    a.unlimit();
                    let ctx = format!(
                        "{} copy of {label} with budget {budget}/{total}",
                        if deep { "deep" } else { "shallow" }
                    );
                    diff_eq!(cres.is_null(), rres.is_null(), "{ctx}: NULL-ness");
                    diff_eq!(cu, ru, "{ctx}: allocations consumed");
                    if budget == 0 {
                        assert!(
                            cres.is_null(),
                            "C ground truth: with no allocations the copy must fail ({ctx})"
                        );
                    }
                    drop(a);
                    // Inspect and release only once the real allocator is back.
                    diff_eq!(state(c, cres), state(r, rres), "{ctx}: result");
                    decref(c, cres);
                    decref(r, rres);
                    // The source must be untouched by a failed copy.
                    diff_eq!(state(c, ct), state(r, rt), "{ctx}: source after");
                }
            }
        }

        for (_, p) in ctrees {
            decref(c, p);
        }
        for (_, p) in rtrees {
            decref(r, p);
        }
    }
}

#[test]
fn oom_grow_failures_in_insert_and_extend() {
    let _g = global_state_lock();
    // ERRORS.md 63 (json_array_insert_new's json_array_grow fails => -1 and the
    // value is decref'd) and 69 (json_array_extend's grow fails => -1 with NO
    // refcount touched anywhere).
    let (c, r) = both();
    unsafe {
        // --- row 63
        for n in [8usize, 16] {
            let ca = (c.json_array)();
            let ra = (r.json_array)();
            for i in 0..n {
                (c.json_array_append_new)(ca, (c.json_integer)(i as i64));
                (r.json_array_append_new)(ra, (r.json_integer)(i as i64));
            }
            for idx in [0usize, 1, n / 2, n] {
                let (cw, rw) = (witness(c), witness(r));
                let before_c = state(c, ca);
                let before_r = state(r, ra);
                diff_eq!(before_c.clone(), before_r, "array of {n} before the failing insert");
                {
                    let _a = Alloc::install(c, r, 0);
                    let cret = (c.json_array_insert_new)(ca, idx, cw);
                    let rret = (r.json_array_insert_new)(ra, idx, rw);
                    diff_eq!(cret, rret, "insert_new(n={n},idx={idx}) with OOM grow (row 63)");
                    assert_eq!(cret, -1, "C ground truth: grow failure => -1 (row 63)");
                }
                check_witness(c, r, cw, rw, 1, &format!("row 63 n={n} idx={idx}"));
                diff_eq!(state(c, ca), state(r, ra), "array after the failed insert");
                assert_eq!(before_c, state(c, ca), "C: array unchanged (row 63)");
            }
            decref(c, ca);
            decref(r, ra);
        }

        // --- row 58 companion / row 63 with append is already covered in a09,
        //     so here only extend (row 69).
        // `n` must fill the array to its capacity exactly (8, then 16 after one
        // doubling), so that extending by 3 genuinely needs a grow — otherwise
        // the OOM would never be reached and the test would prove nothing.
        for n in [8usize, 16] {
            let ca = (c.json_array)();
            let ra = (r.json_array)();
            let cb = (c.json_array)();
            let rb = (r.json_array)();
            for i in 0..n {
                (c.json_array_append_new)(ca, (c.json_integer)(i as i64));
                (r.json_array_append_new)(ra, (r.json_integer)(i as i64));
            }
            for i in 0..3 {
                (c.json_array_append_new)(cb, (c.json_integer)(100 + i));
                (r.json_array_append_new)(rb, (r.json_integer)(100 + i));
            }
            let before_c = (state(c, ca), state(c, cb));
            let before_r = (state(r, ra), state(r, rb));
            diff_eq!(before_c.clone(), before_r, "arrays before the failing extend");
            {
                let _a = Alloc::install(c, r, 0);
                let cret = (c.json_array_extend)(ca, cb);
                let rret = (r.json_array_extend)(ra, rb);
                diff_eq!(cret, rret, "json_array_extend with OOM grow (row 69)");
                assert_eq!(cret, -1, "C ground truth: extend grow failure => -1 (row 69)");
            }
            // Row 69's "no refcounts touched": the elements of `other` must not
            // have been increfd, which the full state comparison covers because
            // it records every child's refcount.
            diff_eq!(
                (state(c, ca), state(c, cb)),
                (state(r, ra), state(r, rb)),
                "arrays after the failed extend (row 69)"
            );
            assert_eq!(
                before_c,
                (state(c, ca), state(c, cb)),
                "C: nothing changed by a failed extend (row 69)"
            );
            decref(c, ca);
            decref(r, ra);
            decref(c, cb);
            decref(r, rb);
        }
    }
}

#[test]
fn oom_in_update_recursive_and_loop_check() {
    let _g = global_state_lock();
    // ERRORS.md 32 (hashtable_init(&parents_set) fails => -1) and 125
    // (jsonp_loop_check's hashtable_set fails => -1).
    let (c, r) = both();
    unsafe {
        let mk = |api: &Api| -> (*mut json_t, *mut json_t) {
            let a = (api.json_object)();
            let b = (api.json_object)();
            let k = cs("k");
            (api.json_object_setn_new_nocheck)(b, k.as_ptr(), 1, (api.json_integer)(1));
            (a, b)
        };
        let (ca, cb) = mk(c);
        let (ra, rb) = mk(r);

        // --- row 32: with a budget of 0 the parents_set cannot be created.
        {
            let _a = Alloc::install(c, r, 0);
            let cret = (c.json_object_update_recursive)(ca, cb);
            let rret = (r.json_object_update_recursive)(ra, rb);
            diff_eq!(cret, rret, "update_recursive with OOM parents_set (row 32)");
            assert_eq!(cret, -1, "C ground truth: hashtable_init failure => -1");
        }
        diff_eq!(state(c, ca), state(r, ra), "target after row-32 failure");
        assert_eq!((c.json_object_size)(ca), 0, "C: no partial update");

        // Sweep the budget: the whole operation must fail or succeed at the same
        // allocation index in both libraries.
        let total = {
            let a = Alloc::install(c, r, -1);
            let (cx, cy) = mk(c);
            let (rx, ry) = mk(r);
            assert_eq!((c.json_object_update_recursive)(cx, cy), 0);
            assert_eq!((r.json_object_update_recursive)(rx, ry), 0);
            let (cu, ru) = a.used();
            drop(a);
            decref(c, cx);
            decref(c, cy);
            decref(r, rx);
            decref(r, ry);
            // `mk` itself allocates, so this is an upper bound, which is all the
            // sweep needs.
            diff_eq!(cu, ru, "allocation count for a full update_recursive");
            cu
        };
        for budget in 0..=(total as isize) {
            let (cx, cy) = mk(c);
            let (rx, ry) = mk(r);
            let a = Alloc::install(c, r, budget);
            let cret = (c.json_object_update_recursive)(cx, cy);
            let rret = (r.json_object_update_recursive)(rx, ry);
            let (cu, ru) = a.used();
            drop(a);
            diff_eq!(cret, rret, "update_recursive at budget {budget}/{total}");
            diff_eq!(cu, ru, "allocations used at budget {budget}");
            diff_eq!(
                (state(c, cx), state(c, cy)),
                (state(r, rx), state(r, ry)),
                "state after update_recursive at budget {budget}"
            );
            decref(c, cx);
            decref(c, cy);
            decref(r, rx);
            decref(r, ry);
        }

        // --- row 125: jsonp_loop_check's hashtable_set fails.
        const KLEN: usize = 2 + std::mem::size_of::<*mut json_t>() * 2 + 1;
        let mut cht = Box::new(hashtable_t::zeroed());
        let mut rht = Box::new(hashtable_t::zeroed());
        assert_eq!((c.hashtable_init)(&mut *cht), 0);
        assert_eq!((r.hashtable_init)(&mut *rht), 0);
        let mut ck = [0u8; 64];
        let mut rk = [0u8; 64];
        let mut clen = usize::MAX;
        let mut rlen = usize::MAX;
        {
            let _a = Alloc::install(c, r, 0);
            let cret = (c.jsonp_loop_check)(
                &mut *cht,
                ca as *const json_t,
                ck.as_mut_ptr() as *mut c_char,
                KLEN,
                &mut clen,
            );
            let rret = (r.jsonp_loop_check)(
                &mut *rht,
                ra as *const json_t,
                rk.as_mut_ptr() as *mut c_char,
                KLEN,
                &mut rlen,
            );
            diff_eq!(cret, rret, "jsonp_loop_check with OOM hashtable_set (row 125)");
            assert_eq!(cret, -1, "C ground truth: hashtable_set failure => -1");
        }
        // *key_len_out is written before the failing insert.
        diff_eq!(clen, rlen, "*key_len_out on the OOM path (row 125)");
        assert_ne!(clen, usize::MAX, "C: *key_len_out was written");
        (c.hashtable_close)(&mut *cht);
        (r.hashtable_close)(&mut *rht);

        decref(c, ca);
        decref(c, cb);
        decref(r, ra);
        decref(r, rb);
    }
}

// ===========================================================================
// ERRORS.md 336 / 337 — json_object_seed's seed generation
// ===========================================================================

#[repr(C)]
struct rlimit {
    rlim_cur: u64,
    rlim_max: u64,
}

#[repr(C)]
struct timeval_ {
    tv_sec: i64,
    tv_usec: i64,
}

extern "C" {
    fn getrlimit(resource: c_int, rlim: *mut rlimit) -> c_int;
    fn setrlimit(resource: c_int, rlim: *const rlimit) -> c_int;
    fn getpid() -> c_int;
    fn gettimeofday(tv: *mut timeval_, tz: *mut c_void) -> c_int;
}
const RLIMIT_NOFILE: c_int = 7;

/// Row 337: `seed_from_urandom` returns 1 when `open("/dev/urandom")` fails, so
/// `generate_seed` falls back to `seed_from_timestamp_and_pid`.
///
/// `json_object_seed` is ONE-SHOT (`if (hashtable_seed == 0)`) and `both()` has
/// already installed the fixed seed in this process, so the seeding code cannot
/// be re-entered here — and the C's `seed_initialized` flag is a file-local
/// static, so it cannot be reset through the FFI either. The test therefore
/// re-executes itself as a CHILD process (where nothing has seeded yet), drops
/// `RLIMIT_NOFILE` so that `open("/dev/urandom", O_RDONLY)` must return `-1`,
/// and then seeds both libraries with `json_object_seed(0)`.
///
/// The fallback is identifiable: `seed = ((uint32_t)tv_sec ^ (uint32_t)tv_usec)
/// ^ (uint32_t)getpid()`. The child brackets the call with `gettimeofday`, so it
/// can assert that each library's resulting seed is exactly that expression for
/// some timestamp inside the bracket — which the /dev/urandom path would only
/// match by a ~1-in-2^32 coincidence.
///
/// Row 336 (`generate_seed()` computing 0, forced to 1) is NOT forceable: its
/// inputs are 4 bytes of /dev/urandom, or gettimeofday XOR getpid, and none of
/// those can be steered to make the XOR come out as zero. The child does assert
/// the property that line exists to guarantee — the installed seed is never 0 —
/// which is the only observable consequence.
#[test]
fn seed_fallback_without_urandom_subprocess() {
    if std::env::var("A13_SEED_CHILD").is_ok() {
        seed_child_body();
        return;
    }
    let _g = global_state_lock();
    let exe = std::env::current_exe().expect("current_exe");
    let out = std::process::Command::new(exe)
        .args([
            "--exact",
            "seed_fallback_without_urandom_subprocess",
            "--nocapture",
            "--test-threads=1",
        ])
        .env("A13_SEED_CHILD", "1")
        .output()
        .expect("failed to spawn the child test process");
    let so = String::from_utf8_lossy(&out.stdout);
    let se = String::from_utf8_lossy(&out.stderr);
    assert!(
        out.status.success(),
        "child process failed (rows 336/337)\n--- stdout ---\n{so}\n--- stderr ---\n{se}"
    );
    assert!(
        so.contains("A13_SEED_OK"),
        "child did not report success\n--- stdout ---\n{so}\n--- stderr ---\n{se}"
    );
}

fn seed_child_body() {
    // NOTE: deliberately NOT `both()` — that would seed both libraries.
    let (c, r) = (capi(), rapi());
    unsafe {
        assert_eq!(c.hashtable_seed(), 0, "C: nothing may have seeded yet");
        assert_eq!(r.hashtable_seed(), 0, "Rust: nothing may have seeded yet");

        // Make open() impossible: the libraries are already dlopen'd, so no
        // further file descriptor is needed by anything except the urandom read.
        let mut saved = rlimit {
            rlim_cur: 0,
            rlim_max: 0,
        };
        assert_eq!(getrlimit(RLIMIT_NOFILE, &mut saved), 0, "getrlimit failed");
        let low = rlimit {
            rlim_cur: 3,
            rlim_max: saved.rlim_max,
        };
        assert_eq!(setrlimit(RLIMIT_NOFILE, &low), 0, "setrlimit failed");

        let pid = getpid() as u32;
        let mut t0 = timeval_ {
            tv_sec: 0,
            tv_usec: 0,
        };
        let mut t1 = timeval_ {
            tv_sec: 0,
            tv_usec: 0,
        };
        gettimeofday(&mut t0, std::ptr::null_mut());
        (c.json_object_seed)(0);
        (r.json_object_seed)(0);
        gettimeofday(&mut t1, std::ptr::null_mut());

        // Restore the limit before doing anything that might need a descriptor.
        setrlimit(RLIMIT_NOFILE, &saved);

        let cseed = c.hashtable_seed();
        let rseed = r.hashtable_seed();
        println!("C seed = {cseed:#010x}, Rust seed = {rseed:#010x}, pid = {pid}");

        // Row 336's guarantee: the installed seed is never zero (otherwise the
        // library would re-enter auto-seeding forever).
        assert_ne!(cseed, 0, "C: the seed must never be 0 (row 336)");
        assert_ne!(rseed, 0, "Rust: the seed must never be 0 (row 336)");

        // Row 337: both must have used the timestamp+pid fallback.
        let mut candidates: Vec<u32> = Vec::new();
        for sec in [t0.tv_sec, t1.tv_sec] {
            let lo = if sec == t0.tv_sec { t0.tv_usec } else { 0 };
            let hi = if sec == t1.tv_sec {
                t1.tv_usec
            } else {
                999_999
            };
            for usec in lo..=hi {
                candidates.push(((sec as u32) ^ (usec as u32)) ^ pid);
            }
        }
        assert!(
            candidates.len() < 4_000_000,
            "the bracket is implausibly wide: {} candidates",
            candidates.len()
        );
        assert!(
            candidates.contains(&cseed),
            "C seed {cseed:#x} is not a timestamp^pid value from the bracket \
             [{}.{:06}, {}.{:06}] — /dev/urandom must have been readable, so \
             row 337's fallback was not exercised",
            t0.tv_sec,
            t0.tv_usec,
            t1.tv_sec,
            t1.tv_usec
        );
        assert!(
            candidates.contains(&rseed),
            "Rust seed {rseed:#x} is not a timestamp^pid value from the bracket \
             — the Rust library did not take the /dev/urandom fallback (row 337)"
        );

        // And the one-shot guard: a second call cannot change the seed.
        (c.json_object_seed)(0x1234_5678);
        (r.json_object_seed)(0x1234_5678);
        assert_eq!(c.hashtable_seed(), cseed, "C: json_object_seed is one-shot");
        assert_eq!(r.hashtable_seed(), rseed, "Rust: json_object_seed is one-shot");

        println!("A13_SEED_OK");
    }
}
