//! Phase B/C — value.c arrays.
//! CONFIGS rows 30-36 · ERRORS rows 41-59.
mod common;
use common::*;

unsafe fn mk(api: &Api, i: usize) -> *mut JsonT {
    unsafe {
        match i % 6 {
            0 => (api.json_integer)(i as i64),
            1 => (api.json_string)(cs(&format!("s{i}")).as_ptr()),
            2 => (api.json_true)(),
            3 => (api.json_false)(),
            4 => (api.json_null)(),
            _ => {
                let a = (api.json_array)();
                (api.json_array_append_new)(a, (api.json_integer)(i as i64));
                a
            }
        }
    }
}

unsafe fn build(api: &'static Api, n: usize) -> *mut JsonT {
    unsafe {
        let a = (api.json_array)();
        for i in 0..n {
            assert_eq!((api.json_array_append_new)(a, mk(api, i)), 0);
        }
        a
    }
}

/* -------- CONFIGS 30/36: append across the growth boundary -------- */

#[test]
fn json_array_append_growth() {
    unsafe {
        for &n in &[0usize, 1, 7, 8, 9, 16, 17, 100, 1000] {
            let ca = build(c(), n);
            let ra = build(r(), n);
            assert_eq!(
                (c().json_array_size)(ca),
                (r().json_array_size)(ra),
                "size n={n}"
            );
            assert_eq!((c().json_array_size)(ca), n);
            assert_eq!(shape(c(), ca), shape(r(), ra), "shape n={n}");
            // ERRORS 43: index sweep incl. one past the end and SIZE_MAX
            for i in 0..=(n + 2) {
                let cg = (c().json_array_get)(ca, i);
                let rg = (r().json_array_get)(ra, i);
                assert_eq!(cg.is_null(), rg.is_null(), "get({i}) n={n}");
            }
            assert!((c().json_array_get)(ca, usize::MAX).is_null());
            assert!((r().json_array_get)(ra, usize::MAX).is_null());
            assert!((c().json_array_get)(ca, usize::MAX - 1).is_null());
            assert!((r().json_array_get)(ra, usize::MAX - 1).is_null());
            decref(c(), ca);
            decref(r(), ra);
        }
    }
}

/* -------- CONFIGS 31 / ERRORS 51-54: insert -------- */

#[test]
fn json_array_insert_new_all_positions() {
    unsafe {
        for &n in &[0usize, 1, 2, 7, 8, 9, 17] {
            for idx in 0..=(n + 2) {
                let ca = build(c(), n);
                let ra = build(r(), n);
                let cv = (c().json_array_insert_new)(ca, idx, (c().json_integer)(-1));
                let rv = (r().json_array_insert_new)(ra, idx, (r().json_integer)(-1));
                assert_eq!(cv, rv, "insert(n={n}, idx={idx}) ret");
                assert_eq!(shape(c(), ca), shape(r(), ra), "insert(n={n}, idx={idx})");
                if idx <= n {
                    assert_eq!(cv, 0);
                } else {
                    // ERRORS 54: index > entries
                    assert_eq!(cv, -1);
                }
                decref(c(), ca);
                decref(r(), ra);
            }
            // ERRORS 51: NULL value
            let ca = build(c(), n);
            let ra = build(r(), n);
            assert_eq!(
                (c().json_array_insert_new)(ca, 0, std::ptr::null_mut()),
                (r().json_array_insert_new)(ra, 0, std::ptr::null_mut())
            );
            assert_eq!(
                (c().json_array_insert_new)(ca, 0, std::ptr::null_mut()),
                -1
            );
            // ERRORS 53: json == value
            assert_eq!(
                (c().json_array_insert_new)(ca, 0, incref(ca)),
                (r().json_array_insert_new)(ra, 0, incref(ra))
            );
            assert_eq!((c().json_array_insert_new)(ca, 0, incref(ca)), -1);
            assert_eq!(shape(c(), ca), shape(r(), ra));
            decref(c(), ca);
            decref(r(), ra);
        }
        // ERRORS 52: not an array
        for api in both() {
            let o = (api.json_object)();
            assert_eq!((api.json_array_insert_new)(o, 0, (api.json_integer)(1)), -1);
            assert_eq!(
                (api.json_array_insert_new)(std::ptr::null_mut(), 0, (api.json_integer)(1)),
                -1
            );
            decref(api, o);
        }
    }
}

/* -------- CONFIGS 32 / ERRORS 44-47: set -------- */

#[test]
fn json_array_set_new_all_positions() {
    unsafe {
        for &n in &[0usize, 1, 8, 9, 17] {
            for idx in 0..=(n + 1) {
                let ca = build(c(), n);
                let ra = build(r(), n);
                let cv = (c().json_array_set_new)(ca, idx, (c().json_string)(cs("SET").as_ptr()));
                let rv = (r().json_array_set_new)(ra, idx, (r().json_string)(cs("SET").as_ptr()));
                assert_eq!(cv, rv, "set(n={n}, idx={idx}) ret");
                assert_eq!(shape(c(), ca), shape(r(), ra), "set(n={n}, idx={idx})");
                if idx < n {
                    assert_eq!(cv, 0);
                } else {
                    assert_eq!(cv, -1); // ERRORS 47
                }
                decref(c(), ca);
                decref(r(), ra);
            }
        }
        for api in both() {
            let a = build(api, 3);
            // ERRORS 44: NULL value
            assert_eq!((api.json_array_set_new)(a, 0, std::ptr::null_mut()), -1);
            // ERRORS 46: json == value
            assert_eq!((api.json_array_set_new)(a, 0, incref(a)), -1);
            // ERRORS 45: not an array
            let o = (api.json_object)();
            assert_eq!((api.json_array_set_new)(o, 0, (api.json_integer)(1)), -1);
            assert_eq!(
                (api.json_array_set_new)(std::ptr::null_mut(), 0, (api.json_integer)(1)),
                -1
            );
            decref(api, o);
            decref(api, a);
        }
    }
}

/* -------- CONFIGS 33 / ERRORS 55, 56: remove -------- */

#[test]
fn json_array_remove_all_positions() {
    unsafe {
        for &n in &[0usize, 1, 2, 8, 9, 17] {
            for idx in 0..=(n + 1) {
                let ca = build(c(), n);
                let ra = build(r(), n);
                let cv = (c().json_array_remove)(ca, idx);
                let rv = (r().json_array_remove)(ra, idx);
                assert_eq!(cv, rv, "remove(n={n}, idx={idx}) ret");
                assert_eq!(shape(c(), ca), shape(r(), ra), "remove(n={n}, idx={idx})");
                assert_eq!(cv, if idx < n { 0 } else { -1 });
                decref(c(), ca);
                decref(r(), ra);
            }
            // remove everything, front to back and back to front
            for reverse in [false, true] {
                let ca = build(c(), n);
                let ra = build(r(), n);
                for k in 0..n {
                    let idx = if reverse { n - 1 - k } else { 0 };
                    assert_eq!(
                        (c().json_array_remove)(ca, idx),
                        (r().json_array_remove)(ra, idx)
                    );
                    assert_eq!(shape(c(), ca), shape(r(), ra));
                }
                assert_eq!((c().json_array_size)(ca), 0);
                assert_eq!((r().json_array_size)(ra), 0);
                decref(c(), ca);
                decref(r(), ra);
            }
        }
        for api in both() {
            let o = (api.json_object)();
            assert_eq!((api.json_array_remove)(o, 0), -1); // ERRORS 55
            assert_eq!((api.json_array_remove)(std::ptr::null_mut(), 0), -1);
            decref(api, o);
        }
    }
}

/* -------- CONFIGS 34 / ERRORS 57: clear -------- */

#[test]
fn json_array_clear_and_reuse() {
    unsafe {
        for &n in &[0usize, 1, 9, 100] {
            let ca = build(c(), n);
            let ra = build(r(), n);
            assert_eq!((c().json_array_clear)(ca), (r().json_array_clear)(ra));
            assert_eq!((c().json_array_clear)(ca), 0);
            assert_eq!(shape(c(), ca), shape(r(), ra), "after clear n={n}");
            assert_eq!((c().json_array_size)(ca), 0);
            // reuse
            for i in 0..5 {
                assert_eq!(
                    (c().json_array_append_new)(ca, mk(c(), i)),
                    (r().json_array_append_new)(ra, mk(r(), i))
                );
            }
            assert_eq!(shape(c(), ca), shape(r(), ra), "after reuse n={n}");
            decref(c(), ca);
            decref(r(), ra);
        }
        for api in both() {
            let o = (api.json_object)();
            assert_eq!((api.json_array_clear)(o), -1); // ERRORS 57
            assert_eq!((api.json_array_clear)(std::ptr::null_mut()), -1);
            decref(api, o);
        }
    }
}

/* -------- CONFIGS 35 / ERRORS 58, 59: extend -------- */

#[test]
fn json_array_extend_matrix() {
    unsafe {
        for &n in &[0usize, 1, 5, 8, 9] {
            for &m in &[0usize, 1, 5, 8, 9, 20] {
                let ca = build(c(), n);
                let ra = build(r(), n);
                let cb = build(c(), m);
                let rb = build(r(), m);
                let cv = (c().json_array_extend)(ca, cb);
                let rv = (r().json_array_extend)(ra, rb);
                assert_eq!(cv, rv, "extend({n}, {m}) ret");
                assert_eq!(cv, 0);
                assert_eq!(shape(c(), ca), shape(r(), ra), "extend({n}, {m})");
                assert_eq!((c().json_array_size)(ca), n + m);
                // `other` is unchanged
                assert_eq!(shape(c(), cb), shape(r(), rb), "extend other unchanged");
                decref(c(), cb);
                decref(r(), rb);
                decref(c(), ca);
                decref(r(), ra);
            }
        }
        // self-extend (valid in this C: grows then copies)
        for &n in &[1usize, 3, 8] {
            let ca = build(c(), n);
            let ra = build(r(), n);
            assert_eq!(
                (c().json_array_extend)(ca, ca),
                (r().json_array_extend)(ra, ra),
                "self-extend n={n}"
            );
            assert_eq!(shape(c(), ca), shape(r(), ra), "self-extend n={n}");
            decref(c(), ca);
            decref(r(), ra);
        }
        // ERRORS 58/59: non-array args
        for api in both() {
            let a = build(api, 3);
            let o = (api.json_object)();
            assert_eq!((api.json_array_extend)(o, a), -1);
            assert_eq!((api.json_array_extend)(a, o), -1);
            assert_eq!((api.json_array_extend)(std::ptr::null_mut(), a), -1);
            assert_eq!((api.json_array_extend)(a, std::ptr::null_mut()), -1);
            decref(api, o);
            decref(api, a);
        }
    }
}

/* -------- ERRORS 41, 42, 48-50 + randomized mutation sequences -------- */

#[test]
fn json_array_size_get_wrong_type() {
    unsafe {
        for api in both() {
            for &p in &[
                (api.json_object)(),
                (api.json_string)(cs("x").as_ptr()),
                (api.json_integer)(1),
                (api.json_true)(),
                (api.json_null)(),
            ] {
                assert_eq!((api.json_array_size)(p), 0, "{}: ERRORS 41", api.tag);
                assert!((api.json_array_get)(p, 0).is_null(), "ERRORS 42");
                decref(api, p);
            }
            assert_eq!((api.json_array_size)(std::ptr::null()), 0);
            assert!((api.json_array_get)(std::ptr::null(), 0).is_null());
            // ERRORS 48/49/50 for append
            let a = (api.json_array)();
            assert_eq!((api.json_array_append_new)(a, std::ptr::null_mut()), -1);
            assert_eq!((api.json_array_append_new)(a, incref(a)), -1);
            let o = (api.json_object)();
            assert_eq!((api.json_array_append_new)(o, (api.json_integer)(1)), -1);
            assert_eq!(
                (api.json_array_append_new)(std::ptr::null_mut(), (api.json_integer)(1)),
                -1
            );
            decref(api, o);
            decref(api, a);
        }
    }
}

#[test]
fn json_array_randomized_mutation_sequences() {
    unsafe {
        let mut rng = Rng::new(0xA55A_0001);
        for trial in 0..800 {
            let ca = (c().json_array)();
            let ra = (r().json_array)();
            let nops = 1 + rng.below(60);
            for op in 0..nops {
                let n = (c().json_array_size)(ca);
                assert_eq!(n, (r().json_array_size)(ra));
                // index deliberately allowed to go out of range
                let idx = if rng.below(4) == 0 {
                    rng.below(n + 3)
                } else if n == 0 {
                    0
                } else {
                    rng.below(n)
                };
                let which = rng.below(6);
                let seed = rng.below(6);
                let (cv, rv) = match which {
                    0 => (
                        (c().json_array_append_new)(ca, mk(c(), seed)),
                        (r().json_array_append_new)(ra, mk(r(), seed)),
                    ),
                    1 => (
                        (c().json_array_insert_new)(ca, idx, mk(c(), seed)),
                        (r().json_array_insert_new)(ra, idx, mk(r(), seed)),
                    ),
                    2 => (
                        (c().json_array_set_new)(ca, idx, mk(c(), seed)),
                        (r().json_array_set_new)(ra, idx, mk(r(), seed)),
                    ),
                    3 => (
                        (c().json_array_remove)(ca, idx),
                        (r().json_array_remove)(ra, idx),
                    ),
                    4 => {
                        let m = rng.below(6);
                        let cb = build(c(), m);
                        let rb = build(r(), m);
                        let x = (
                            (c().json_array_extend)(ca, cb),
                            (r().json_array_extend)(ra, rb),
                        );
                        decref(c(), cb);
                        decref(r(), rb);
                        x
                    }
                    _ => (
                        (c().json_array_clear)(ca),
                        (r().json_array_clear)(ra),
                    ),
                };
                assert_eq!(cv, rv, "trial {trial} op {op} which={which} ret");
                assert_eq!(
                    shape(c(), ca),
                    shape(r(), ra),
                    "trial {trial} op {op} which={which} shape"
                );
            }
            // and the dumped form must be byte-identical
            for flags in [0usize, JSON_COMPACT, json_indent(3), JSON_SORT_KEYS] {
                let _g = dtoa_guard();
                assert_bytes_eq(
                    &format!("trial {trial} dumps flags={flags:#x}"),
                    &dumps(c(), ca, flags),
                    &dumps(r(), ra, flags),
                );
            }
            decref(c(), ca);
            decref(r(), ra);
        }
    }
}
