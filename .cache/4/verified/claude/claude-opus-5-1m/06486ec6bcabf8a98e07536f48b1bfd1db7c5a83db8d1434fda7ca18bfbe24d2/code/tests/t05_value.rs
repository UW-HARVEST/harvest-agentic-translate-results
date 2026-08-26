//! Differential tests for `value.c` — CONFIGS.md rows 36-70, ERRORS.md rows 1-95.
mod common;
use common::*;

/// `dtoa.c` is compiled WITHOUT `MULTIPLE_THREADS`, so `Balloc`'s `freelist`,
/// `p5s` and `dtoa_result` are unsynchronised mutable statics in BOTH libraries.
/// Any test that formats a real number must therefore run exclusively.
fn lock() -> std::sync::MutexGuard<'static, ()> {
    static L: std::sync::Mutex<()> = std::sync::Mutex::new(());
    match L.lock() {
        Ok(g) => g,
        Err(e) => e.into_inner(),
    }
}
use std::ffi::{c_char, c_int, c_void};
use std::ptr;

// ---------------------------------------------------------------------------
// A library-independent description of a value tree, so the *same* tree can be
// built in the C `.so` and in the Rust `.so`.
// ---------------------------------------------------------------------------

#[derive(Clone, Debug)]
pub enum V {
    Obj(Vec<(Vec<u8>, V)>),
    Arr(Vec<V>),
    /// built with `json_stringn` (UTF-8 checked)
    Str(Vec<u8>),
    /// built with `json_stringn_nocheck` (no UTF-8 check, may hold NULs)
    StrRaw(Vec<u8>),
    Int(i64),
    Real(f64),
    True,
    False,
    Null,
}

pub fn build(l: &Lib, v: &V) -> *mut json_t {
    unsafe {
        match v {
            V::Obj(kv) => {
                let o = (l.json_object)();
                assert!(!o.is_null());
                for (k, val) in kv {
                    let child = build(l, val);
                    let r = (l.json_object_setn_new_nocheck)(
                        o,
                        k.as_ptr() as *const c_char,
                        k.len(),
                        child,
                    );
                    assert_eq!(r, 0, "{}: setn_new_nocheck failed", l.which);
                }
                o
            }
            V::Arr(items) => {
                let a = (l.json_array)();
                assert!(!a.is_null());
                for it in items {
                    let child = build(l, it);
                    let r = (l.json_array_append_new)(a, child);
                    assert_eq!(r, 0, "{}: append_new failed", l.which);
                }
                a
            }
            V::Str(b) => (l.json_stringn)(b.as_ptr() as *const c_char, b.len()),
            V::StrRaw(b) => (l.json_stringn_nocheck)(b.as_ptr() as *const c_char, b.len()),
            V::Int(i) => (l.json_integer)(*i),
            V::Real(f) => (l.json_real)(*f),
            V::True => (l.json_true)(),
            V::False => (l.json_false)(),
            V::Null => (l.json_null)(),
        }
    }
}

/// Build the same tree in both libraries.
pub fn build2(d: &Duo, v: &V) -> (*mut json_t, *mut json_t) {
    (build(&d.c, v), build(&d.rs, v))
}

pub fn free2(d: &Duo, cj: *mut json_t, rj: *mut json_t) {
    decref(&d.c, cj);
    decref(&d.rs, rj);
}

/// Compare two trees through the public getters only.
#[track_caller]
pub fn same(d: &Duo, what: &str, cj: *const json_t, rj: *const json_t) {
    eq(what, describe(&d.c, cj), describe(&d.rs, rj));
}

// ---------------------------------------------------------------------------
// Random corpus
// ---------------------------------------------------------------------------

pub fn rand_value(rng: &mut Rng, depth: usize) -> V {
    let leaf = depth == 0 || rng.below(100) < 40;
    if leaf {
        match rng.below(8) {
            0 => { let n = rng.below(8); V::Str(rng.utf8_string(n)) }
            1 => { let n = rng.below(12); V::Str(rng.ascii_string(n)) }
            2 => V::Int(match rng.below(4) {
                0 => 0,
                1 => rng.next_u64() as i64,
                2 => rng.range_i64(-1000, 1000),
                _ => {
                    if rng.bool() {
                        i64::MAX
                    } else {
                        i64::MIN
                    }
                }
            }),
            3 => V::Real(rng.tame_f64()),
            4 => V::Real(rng.finite_f64()),
            5 => V::True,
            6 => V::False,
            _ => V::Null,
        }
    } else if rng.bool() {
        let n = rng.below(6);
        V::Arr((0..n).map(|_| rand_value(rng, depth - 1)).collect())
    } else {
        let n = rng.below(6);
        let mut kv = Vec::new();
        for i in 0..n {
            let kn = 1 + rng.below(4);
            let mut k = rng.utf8_string(kn);
            if rng.below(10) == 0 {
                k = format!("dup{}", i % 2).into_bytes();
            }
            kv.push((k, rand_value(rng, depth - 1)));
        }
        V::Obj(kv)
    }
}

pub fn corpus(n: usize, seed: u64, depth: usize) -> Vec<V> {
    let mut rng = Rng::new(seed);
    (0..n).map(|_| rand_value(&mut rng, depth)).collect()
}

/// A fabricated `json_t` with an out-of-range `json_type`, to test the C's
/// `default:` branches across the FFI boundary.
pub fn bogus(ty: c_int) -> Box<json_t> {
    Box::new(json_t {
        type_: ty,
        refcount: usize::MAX,
    })
}

// ===========================================================================
// CONFIGS 36-47, ERRORS 1-39 — objects
// ===========================================================================

#[test]
fn object_empty_and_delete() {
    let d = duo();
    let _g = lock();
    unsafe {
        let (c, r) = (( d.c.json_object)(), (d.rs.json_object)());
        assert!(!c.is_null() && !r.is_null());
        eq("type", (*c).type_, (*r).type_);
        eq("type is JSON_OBJECT", (*c).type_, JSON_OBJECT);
        eq("refcount", (*c).refcount, (*r).refcount);
        eq(
            "size",
            (d.c.json_object_size)(c),
            (d.rs.json_object_size)(r),
        );
        same(d, "empty object", c, r);
        free2(d, c, r);
    }
}

#[test]
fn object_set_get_del_counts() {
    let d = duo();
    let _g = lock();
    unsafe {
        for n in [1usize, 7, 8, 9, 64, 200] {
            let (c, r) = ((d.c.json_object)(), (d.rs.json_object)());
            for i in 0..n {
                let k = cs(&format!("k{}", i));
                let cv = (d.c.json_integer)(i as i64);
                let rv = (d.rs.json_integer)(i as i64);
                eq(
                    &format!("set_new n={} i={}", n, i),
                    (d.c.json_object_set_new)(c, k.as_ptr(), cv),
                    (d.rs.json_object_set_new)(r, k.as_ptr(), rv),
                );
                same(d, &format!("object after set n={} i={}", n, i), c, r);
            }
            // get every key, present and absent
            for i in 0..n + 5 {
                let k = cs(&format!("k{}", i));
                let cg = (d.c.json_object_get)(c, k.as_ptr());
                let rg = (d.rs.json_object_get)(r, k.as_ptr());
                eq(&format!("get null n={} i={}", n, i), cg.is_null(), rg.is_null());
                if !cg.is_null() {
                    same(d, &format!("get value n={} i={}", n, i), cg, rg);
                }
            }
            // delete all in a shuffled order
            let mut order: Vec<usize> = (0..n).collect();
            let mut rng = Rng::new(0xD3_1000 + n as u64);
            for i in (1..order.len()).rev() {
                let j = rng.below(i + 1);
                order.swap(i, j);
            }
            for &i in &order {
                let k = cs(&format!("k{}", i));
                eq(
                    &format!("del n={} i={}", n, i),
                    (d.c.json_object_del)(c, k.as_ptr()),
                    (d.rs.json_object_del)(r, k.as_ptr()),
                );
                // second delete: must be -1 in both
                eq(
                    &format!("del again n={} i={}", n, i),
                    (d.c.json_object_del)(c, k.as_ptr()),
                    (d.rs.json_object_del)(r, k.as_ptr()),
                );
                same(d, &format!("object after del n={} i={}", n, i), c, r);
            }
            free2(d, c, r);
        }
    }
}

#[test]
fn object_setn_key_shapes() {
    let d = duo();
    let _g = lock();
    unsafe {
        let (c, r) = ((d.c.json_object)(), (d.rs.json_object)());
        // key_len shorter than strlen, empty key, embedded NUL, long key
        let cases: Vec<(Vec<u8>, usize)> = vec![
            (b"".to_vec(), 0),
            (b"a".to_vec(), 1),
            (b"abcdef".to_vec(), 3),
            (b"ab\0cd".to_vec(), 5),
            (b"\0".to_vec(), 1),
            (vec![b'x'; 255], 255),
            (vec![b'y'; 1000], 1000),
            ("ünïcødé".as_bytes().to_vec(), "ünïcødé".len()),
            ("𝄞𝄢".as_bytes().to_vec(), "𝄞𝄢".len()),
        ];
        for (i, (k, kl)) in cases.iter().enumerate() {
            // Pad every key buffer so that `key_len + 1` stays in bounds (the C
            // hashes exactly `key_len` bytes) and so an EMPTY key still has a
            // real, dereferenceable pointer rather than Vec's dangling one.
            let mut pad = k.clone();
            pad.extend_from_slice(&[0xEEu8; 8]);
            let kp = pad.as_ptr() as *const c_char;

            let cv = (d.c.json_integer)(i as i64);
            let rv = (d.rs.json_integer)(i as i64);
            eq(
                &format!("setn_new key={:?}", k),
                (d.c.json_object_setn_new)(c, kp, *kl, cv),
                (d.rs.json_object_setn_new)(r, kp, *kl, rv),
            );
            same(d, &format!("object after setn key={:?}", k), c, r);
            // getn with the exact len, one shorter and one longer
            for dl in [0isize, -1, 1] {
                let l2 = (*kl as isize + dl).max(0) as usize;
                let cg = (d.c.json_object_getn)(c, kp, l2);
                let rg = (d.rs.json_object_getn)(r, kp, l2);
                eq(
                    &format!("getn key={:?} len={}", k, l2),
                    cg.is_null(),
                    rg.is_null(),
                );
                if !cg.is_null() {
                    same(d, "getn value", cg, rg);
                }
            }
            // and the nocheck variant with the same shapes
            let cv = (d.c.json_integer)(1000 + i as i64);
            let rv = (d.rs.json_integer)(1000 + i as i64);
            eq(
                &format!("setn_new_nocheck key={:?}", k),
                (d.c.json_object_setn_new_nocheck)(c, kp, *kl, cv),
                (d.rs.json_object_setn_new_nocheck)(r, kp, *kl, rv),
            );
            same(d, &format!("object after setn_nocheck key={:?}", k), c, r);
            // deln with the exact len and one shorter
            for dl in [1isize, 0] {
                let l2 = (*kl as isize + dl).max(0) as usize;
                eq(
                    &format!("deln key={:?} len={}", k, l2),
                    (d.c.json_object_deln)(c, kp, l2),
                    (d.rs.json_object_deln)(r, kp, l2),
                );
                same(d, &format!("object after deln key={:?} len={}", k, l2), c, r);
            }
        }
        free2(d, c, r);
    }
}

#[test]
fn object_setn_new_rejects_invalid_utf8_but_nocheck_accepts() {
    let d = duo();
    let _g = lock();
    unsafe {
        let bad: Vec<Vec<u8>> = vec![
            vec![0x80],
            vec![0xC0, 0x80],
            vec![0xC1, 0xBF],
            vec![0xE0, 0x80, 0x80],
            vec![0xED, 0xA0, 0x80],
            vec![0xF5, 0x80, 0x80, 0x80],
            vec![0xC2],
            vec![b'a', 0xE2, 0x82],
        ];
        for k in &bad {
            let (c, r) = ((d.c.json_object)(), (d.rs.json_object)());
            // checked: rejected, value decref'd
            let cv = (d.c.json_integer)(1);
            let rv = (d.rs.json_integer)(1);
            eq(
                &format!("setn_new invalid utf8 {:?}", k),
                (d.c.json_object_setn_new)(c, k.as_ptr() as *const c_char, k.len(), cv),
                (d.rs.json_object_setn_new)(r, k.as_ptr() as *const c_char, k.len(), rv),
            );
            same(d, "object unchanged after reject", c, r);
            // nocheck: accepted
            let cv = (d.c.json_integer)(2);
            let rv = (d.rs.json_integer)(2);
            eq(
                &format!("setn_new_nocheck invalid utf8 {:?}", k),
                (d.c.json_object_setn_new_nocheck)(c, k.as_ptr() as *const c_char, k.len(), cv),
                (d.rs.json_object_setn_new_nocheck)(r, k.as_ptr() as *const c_char, k.len(), rv),
            );
            same(d, "object after nocheck accept", c, r);
            free2(d, c, r);
        }
    }
}

#[test]
fn object_clear_and_reuse() {
    let d = duo();
    let _g = lock();
    unsafe {
        for n in [0usize, 1, 64] {
            let (c, r) = ((d.c.json_object)(), (d.rs.json_object)());
            for i in 0..n {
                let k = cs(&format!("k{}", i));
                (d.c.json_object_set_new)(c, k.as_ptr(), (d.c.json_integer)(i as i64));
                (d.rs.json_object_set_new)(r, k.as_ptr(), (d.rs.json_integer)(i as i64));
            }
            eq(
                &format!("clear n={}", n),
                (d.c.json_object_clear)(c),
                (d.rs.json_object_clear)(r),
            );
            same(d, &format!("after clear n={}", n), c, r);
            for i in 0..20 {
                let k = cs(&format!("z{}", i));
                (d.c.json_object_set_new)(c, k.as_ptr(), (d.c.json_integer)(i as i64));
                (d.rs.json_object_set_new)(r, k.as_ptr(), (d.rs.json_integer)(i as i64));
                same(d, &format!("after reuse n={} i={}", n, i), c, r);
            }
            free2(d, c, r);
        }
    }
}

#[test]
fn object_update_variants() {
    let d = duo();
    let _g = lock();
    let base = V::Obj(vec![
        (b"a".to_vec(), V::Int(1)),
        (b"b".to_vec(), V::Int(2)),
        (b"c".to_vec(), V::Obj(vec![(b"x".to_vec(), V::Int(9))])),
    ]);
    let others = vec![
        V::Obj(vec![]),
        V::Obj(vec![(b"d".to_vec(), V::Int(4))]),
        V::Obj(vec![(b"a".to_vec(), V::Int(100))]),
        V::Obj(vec![
            (b"a".to_vec(), V::Int(100)),
            (b"d".to_vec(), V::Int(4)),
        ]),
        V::Obj(vec![(
            b"c".to_vec(),
            V::Obj(vec![(b"y".to_vec(), V::Int(8))]),
        )]),
        V::Obj(vec![(b"c".to_vec(), V::Int(0))]),
    ];
    unsafe {
        let fns: [(&str, unsafe extern "C" fn(*mut json_t, *mut json_t) -> c_int, unsafe extern "C" fn(*mut json_t, *mut json_t) -> c_int); 4] = [
            ("update", d.c.json_object_update, d.rs.json_object_update),
            (
                "update_existing",
                d.c.json_object_update_existing,
                d.rs.json_object_update_existing,
            ),
            (
                "update_missing",
                d.c.json_object_update_missing,
                d.rs.json_object_update_missing,
            ),
            (
                "update_recursive",
                d.c.json_object_update_recursive,
                d.rs.json_object_update_recursive,
            ),
        ];
        for (name, cf, rf) in fns {
            for (i, other) in others.iter().enumerate() {
                let (c, r) = build2(d, &base);
                let (co, ro) = build2(d, other);
                eq(&format!("{} #{}", name, i), cf(c, co), rf(r, ro));
                same(d, &format!("{} #{} result", name, i), c, r);
                same(d, &format!("{} #{} other", name, i), co, ro);
                free2(d, co, ro);
                free2(d, c, r);
            }
            // other == object (self-update)
            let (c, r) = build2(d, &base);
            eq(&format!("{} self", name), cf(c, c), rf(r, r));
            same(d, &format!("{} self result", name), c, r);
            free2(d, c, r);
        }
    }
}

#[test]
fn object_update_recursive_deep_and_cycle() {
    let d = duo();
    let _g = lock();
    unsafe {
        let deep = V::Obj(vec![(
            b"l1".to_vec(),
            V::Obj(vec![(
                b"l2".to_vec(),
                V::Obj(vec![(b"l3".to_vec(), V::Int(1))]),
            )]),
        )]);
        let patch = V::Obj(vec![(
            b"l1".to_vec(),
            V::Obj(vec![(
                b"l2".to_vec(),
                V::Obj(vec![(b"l4".to_vec(), V::Int(2))]),
            )]),
        )]);
        let (c, r) = build2(d, &deep);
        let (cp, rp) = build2(d, &patch);
        eq(
            "update_recursive deep",
            (d.c.json_object_update_recursive)(c, cp),
            (d.rs.json_object_update_recursive)(r, rp),
        );
        same(d, "update_recursive deep result", c, r);
        free2(d, cp, rp);
        free2(d, c, r);

        // ERRORS 24: `jsonp_loop_check` hit — a cycle in `other`.
        //
        // Note `json_object_setn_new_nocheck` refuses `json == value`, so a direct
        // self-reference is impossible; the cycle must go through a second object.
        // `do_object_update_recursive` only recurses when BOTH the value in `other`
        // and the value already in `object` are objects, so `object` needs a
        // matching 3-level shape for the recursion to revisit `other`:
        //
        //     other = { a: inner }, inner = { b: other }
        //     target = { a: t1 },   t1 = { b: t2 },   t2 = {}
        //
        //   do_recursive(target, other) : parents={other}
        //     do_recursive(t1,   inner) : parents={other,inner}
        //       do_recursive(t2,  other) : `other` already in parents -> -1
        let mut rcs = Vec::new();
        for l in d.both() {
            let other = (l.json_object)();
            let inner = (l.json_object)();
            (l.json_object_set_new)(other, cs("a").as_ptr(), incref(inner));
            (l.json_object_set_new)(inner, cs("b").as_ptr(), incref(other));

            let target = (l.json_object)();
            let t1 = (l.json_object)();
            let t2 = (l.json_object)();
            (l.json_object_set_new)(t1, cs("b").as_ptr(), t2);
            (l.json_object_set_new)(target, cs("a").as_ptr(), t1);

            let rc = (l.json_object_update_recursive)(target, other);
            rcs.push(rc);
            assert_eq!(rc, -1, "{}: expected -1 for a cyclic `other`", l.which);

            // `target` now also holds a reference to `other` (t2.b was set before
            // the cycle was detected); break every cycle before dropping.
            (l.json_object_clear)(t2);
            (l.json_object_del)(inner, cs("b").as_ptr());
            decref(l, inner);
            decref(l, other);
            decref(l, target);
        }
        eq("update_recursive cycle return", rcs[0], rcs[1]);
    }
}

#[test]
fn object_iteration_and_iter_set() {
    let d = duo();
    let _g = lock();
    unsafe {
        for n in [0usize, 1, 8, 64] {
            let (c, r) = ((d.c.json_object)(), (d.rs.json_object)());
            for i in 0..n {
                let k = cs(&format!("key-{}", i));
                (d.c.json_object_set_new)(c, k.as_ptr(), (d.c.json_integer)(i as i64));
                (d.rs.json_object_set_new)(r, k.as_ptr(), (d.rs.json_integer)(i as i64));
            }
            // full traversal, comparing key / key_len / value at every step
            let mut ci = (d.c.json_object_iter)(c);
            let mut ri = (d.rs.json_object_iter)(r);
            let mut steps = 0;
            loop {
                eq(&format!("iter null n={}", n), ci.is_null(), ri.is_null());
                if ci.is_null() {
                    break;
                }
                let ck = (d.c.json_object_iter_key)(ci);
                let rk = (d.rs.json_object_iter_key)(ri);
                let ckl = (d.c.json_object_iter_key_len)(ci);
                let rkl = (d.rs.json_object_iter_key_len)(ri);
                eq(&format!("iter_key_len n={} s={}", n, steps), ckl, rkl);
                eq_bytes(
                    &format!("iter_key n={} s={}", n, steps),
                    std::slice::from_raw_parts(ck as *const u8, ckl),
                    std::slice::from_raw_parts(rk as *const u8, rkl),
                );
                same(
                    d,
                    &format!("iter_value n={} s={}", n, steps),
                    (d.c.json_object_iter_value)(ci),
                    (d.rs.json_object_iter_value)(ri),
                );
                // key_to_iter must round-trip
                let ci2 = (d.c.json_object_key_to_iter)(ck);
                let ri2 = (d.rs.json_object_key_to_iter)(rk);
                eq(
                    &format!("key_to_iter key_len n={} s={}", n, steps),
                    (d.c.json_object_iter_key_len)(ci2),
                    (d.rs.json_object_iter_key_len)(ri2),
                );
                ci = (d.c.json_object_iter_next)(c, ci);
                ri = (d.rs.json_object_iter_next)(r, ri);
                steps += 1;
                assert!(steps <= n + 1);
            }
            eq(&format!("iteration length n={}", n), steps, n);

            // iter_at at each key, then traverse to the end
            for i in 0..n {
                let k = cs(&format!("key-{}", i));
                let mut ci = (d.c.json_object_iter_at)(c, k.as_ptr());
                let mut ri = (d.rs.json_object_iter_at)(r, k.as_ptr());
                let mut cseq: Vec<Vec<u8>> = Vec::new();
                let mut rseq: Vec<Vec<u8>> = Vec::new();
                while !ci.is_null() {
                    let kl = (d.c.json_object_iter_key_len)(ci);
                    cseq.push(
                        std::slice::from_raw_parts(
                            (d.c.json_object_iter_key)(ci) as *const u8,
                            kl,
                        )
                        .to_vec(),
                    );
                    ci = (d.c.json_object_iter_next)(c, ci);
                }
                while !ri.is_null() {
                    let kl = (d.rs.json_object_iter_key_len)(ri);
                    rseq.push(
                        std::slice::from_raw_parts(
                            (d.rs.json_object_iter_key)(ri) as *const u8,
                            kl,
                        )
                        .to_vec(),
                    );
                    ri = (d.rs.json_object_iter_next)(r, ri);
                }
                eq(&format!("iter_at seq n={} i={}", n, i), cseq, rseq);
            }

            // iter_set_new at every position
            let mut ci = (d.c.json_object_iter)(c);
            let mut ri = (d.rs.json_object_iter)(r);
            let mut k = 0i64;
            while !ci.is_null() {
                eq(
                    &format!("iter_set_new n={} k={}", n, k),
                    (d.c.json_object_iter_set_new)(c, ci, (d.c.json_integer)(1000 + k)),
                    (d.rs.json_object_iter_set_new)(r, ri, (d.rs.json_integer)(1000 + k)),
                );
                ci = (d.c.json_object_iter_next)(c, ci);
                ri = (d.rs.json_object_iter_next)(r, ri);
                k += 1;
            }
            same(d, &format!("after iter_set_new n={}", n), c, r);
            free2(d, c, r);
        }
    }
}

// ===========================================================================
// CONFIGS 48-55, ERRORS 40-58 — arrays
// ===========================================================================

#[test]
fn array_append_grow() {
    let d = duo();
    let _g = lock();
    unsafe {
        for n in [0usize, 1, 8, 9, 100, 1000] {
            let (c, r) = ((d.c.json_array)(), (d.rs.json_array)());
            for i in 0..n {
                eq(
                    &format!("append n={} i={}", n, i),
                    (d.c.json_array_append_new)(c, (d.c.json_integer)(i as i64)),
                    (d.rs.json_array_append_new)(r, (d.rs.json_integer)(i as i64)),
                );
            }
            eq(
                &format!("size n={}", n),
                (d.c.json_array_size)(c),
                (d.rs.json_array_size)(r),
            );
            same(d, &format!("array n={}", n), c, r);
            // every index plus two past the end
            for i in 0..n + 2 {
                let cg = (d.c.json_array_get)(c, i);
                let rg = (d.rs.json_array_get)(r, i);
                eq(&format!("get n={} i={}", n, i), cg.is_null(), rg.is_null());
                if !cg.is_null() {
                    same(d, "get value", cg, rg);
                }
            }
            for i in [usize::MAX, usize::MAX - 1, n.wrapping_sub(1)] {
                let cg = (d.c.json_array_get)(c, i);
                let rg = (d.rs.json_array_get)(r, i);
                eq(&format!("get oob n={} i={}", n, i), cg.is_null(), rg.is_null());
            }
            free2(d, c, r);
        }
    }
}

#[test]
fn array_insert_set_remove_positions() {
    let d = duo();
    let _g = lock();
    unsafe {
        for n in 0usize..11 {
            for idx in 0..n + 3 {
                // insert
                let (c, r) = ((d.c.json_array)(), (d.rs.json_array)());
                for i in 0..n {
                    (d.c.json_array_append_new)(c, (d.c.json_integer)(i as i64));
                    (d.rs.json_array_append_new)(r, (d.rs.json_integer)(i as i64));
                }
                eq(
                    &format!("insert n={} idx={}", n, idx),
                    (d.c.json_array_insert_new)(c, idx, (d.c.json_integer)(-1)),
                    (d.rs.json_array_insert_new)(r, idx, (d.rs.json_integer)(-1)),
                );
                same(d, &format!("after insert n={} idx={}", n, idx), c, r);
                // set
                eq(
                    &format!("set n={} idx={}", n, idx),
                    (d.c.json_array_set_new)(c, idx, (d.c.json_integer)(-2)),
                    (d.rs.json_array_set_new)(r, idx, (d.rs.json_integer)(-2)),
                );
                same(d, &format!("after set n={} idx={}", n, idx), c, r);
                // remove
                eq(
                    &format!("remove n={} idx={}", n, idx),
                    (d.c.json_array_remove)(c, idx),
                    (d.rs.json_array_remove)(r, idx),
                );
                same(d, &format!("after remove n={} idx={}", n, idx), c, r);
                free2(d, c, r);
            }
        }
        // out-of-range indices
        for idx in [usize::MAX, usize::MAX - 1, usize::MAX / 2] {
            let (c, r) = ((d.c.json_array)(), (d.rs.json_array)());
            (d.c.json_array_append_new)(c, (d.c.json_integer)(0));
            (d.rs.json_array_append_new)(r, (d.rs.json_integer)(0));
            eq(
                &format!("set oob idx={}", idx),
                (d.c.json_array_set_new)(c, idx, (d.c.json_integer)(1)),
                (d.rs.json_array_set_new)(r, idx, (d.rs.json_integer)(1)),
            );
            eq(
                &format!("insert oob idx={}", idx),
                (d.c.json_array_insert_new)(c, idx, (d.c.json_integer)(1)),
                (d.rs.json_array_insert_new)(r, idx, (d.rs.json_integer)(1)),
            );
            eq(
                &format!("remove oob idx={}", idx),
                (d.c.json_array_remove)(c, idx),
                (d.rs.json_array_remove)(r, idx),
            );
            same(d, "array unchanged after oob", c, r);
            free2(d, c, r);
        }
    }
}

#[test]
fn array_clear_and_extend() {
    let d = duo();
    let _g = lock();
    unsafe {
        for n in [0usize, 1, 100] {
            let (c, r) = ((d.c.json_array)(), (d.rs.json_array)());
            for i in 0..n {
                (d.c.json_array_append_new)(c, (d.c.json_integer)(i as i64));
                (d.rs.json_array_append_new)(r, (d.rs.json_integer)(i as i64));
            }
            eq(
                &format!("clear n={}", n),
                (d.c.json_array_clear)(c),
                (d.rs.json_array_clear)(r),
            );
            same(d, &format!("after clear n={}", n), c, r);
            for i in 0..5 {
                (d.c.json_array_append_new)(c, (d.c.json_integer)(i));
                (d.rs.json_array_append_new)(r, (d.rs.json_integer)(i));
            }
            same(d, &format!("after clear+reuse n={}", n), c, r);
            free2(d, c, r);
        }
        // extend: empty+empty, empty+N, N+empty, N+M
        for (n, m) in [(0usize, 0usize), (0, 5), (5, 0), (3, 3), (8, 8), (5, 100)] {
            let (c, r) = ((d.c.json_array)(), (d.rs.json_array)());
            let (co, ro) = ((d.c.json_array)(), (d.rs.json_array)());
            for i in 0..n {
                (d.c.json_array_append_new)(c, (d.c.json_integer)(i as i64));
                (d.rs.json_array_append_new)(r, (d.rs.json_integer)(i as i64));
            }
            for i in 0..m {
                (d.c.json_array_append_new)(co, (d.c.json_integer)(1000 + i as i64));
                (d.rs.json_array_append_new)(ro, (d.rs.json_integer)(1000 + i as i64));
            }
            eq(
                &format!("extend {}+{}", n, m),
                (d.c.json_array_extend)(c, co),
                (d.rs.json_array_extend)(r, ro),
            );
            same(d, &format!("extend {}+{} result", n, m), c, r);
            same(d, &format!("extend {}+{} other", n, m), co, ro);
            free2(d, co, ro);
            free2(d, c, r);
        }
        // other == array (self-extend)
        let (c, r) = ((d.c.json_array)(), (d.rs.json_array)());
        for i in 0..3 {
            (d.c.json_array_append_new)(c, (d.c.json_integer)(i));
            (d.rs.json_array_append_new)(r, (d.rs.json_integer)(i));
        }
        eq(
            "extend self",
            (d.c.json_array_extend)(c, c),
            (d.rs.json_array_extend)(r, r),
        );
        same(d, "extend self result", c, r);
        free2(d, c, r);
    }
}

// ===========================================================================
// CONFIGS 56-64, ERRORS 59-85 — scalars
// ===========================================================================

#[test]
fn string_construction_shapes() {
    let d = duo();
    let _g = lock();
    let mut rng = Rng::new(0x57_1234);
    unsafe {
        let mut cases: Vec<Vec<u8>> = vec![
            vec![],
            b"a".to_vec(),
            b"hello world".to_vec(),
            "ü".as_bytes().to_vec(),
            "€".as_bytes().to_vec(),
            "𝄞".as_bytes().to_vec(),
            vec![0x7F],
            vec![0x00],
            b"a\0b".to_vec(),
            vec![0x80],
            vec![0xC0, 0x80],
            vec![0xED, 0xA0, 0x80],
            vec![0xF5, 0x80, 0x80, 0x80],
            vec![b'x'; 1000],
        ];
        for _ in 0..400 {
            let a = rng.below(20); cases.push(rng.utf8_string(a));
            let b = rng.below(20); cases.push(rng.random_bytes(b));
        }
        for b in &cases {
            let z = cbuf(b);
            // json_string (NUL-terminated, checked)
            let cj = (d.c.json_string)(z.as_ptr() as *const c_char);
            let rj = (d.rs.json_string)(z.as_ptr() as *const c_char);
            eq(&format!("json_string null {:?}", b), cj.is_null(), rj.is_null());
            if !cj.is_null() {
                same(d, "json_string", cj, rj);
            }
            free2(d, cj, rj);
            // json_string_nocheck
            let cj = (d.c.json_string_nocheck)(z.as_ptr() as *const c_char);
            let rj = (d.rs.json_string_nocheck)(z.as_ptr() as *const c_char);
            eq("json_string_nocheck null", cj.is_null(), rj.is_null());
            if !cj.is_null() {
                same(d, "json_string_nocheck", cj, rj);
            }
            free2(d, cj, rj);
            // json_stringn / json_stringn_nocheck with the exact len, len-1 and len+1
            for l in [b.len(), b.len().saturating_sub(1), b.len() + 1] {
                if l > z.len() {
                    continue;
                }
                let cj = (d.c.json_stringn)(z.as_ptr() as *const c_char, l);
                let rj = (d.rs.json_stringn)(z.as_ptr() as *const c_char, l);
                eq(
                    &format!("json_stringn null {:?} l={}", b, l),
                    cj.is_null(),
                    rj.is_null(),
                );
                if !cj.is_null() {
                    same(d, "json_stringn", cj, rj);
                }
                free2(d, cj, rj);

                let cj = (d.c.json_stringn_nocheck)(z.as_ptr() as *const c_char, l);
                let rj = (d.rs.json_stringn_nocheck)(z.as_ptr() as *const c_char, l);
                eq("json_stringn_nocheck null", cj.is_null(), rj.is_null());
                if !cj.is_null() {
                    same(d, "json_stringn_nocheck", cj, rj);
                }
                free2(d, cj, rj);
            }
        }
    }
}

#[test]
fn string_setters() {
    let d = duo();
    let _g = lock();
    let mut rng = Rng::new(0x57_5555);
    unsafe {
        let mut cases: Vec<Vec<u8>> = vec![
            vec![],
            b"x".to_vec(),
            b"longer string value".to_vec(),
            "ü€𝄞".as_bytes().to_vec(),
            vec![0x80],
            vec![0xC0, 0x80],
            b"a\0b".to_vec(),
            vec![b'q'; 500],
        ];
        for _ in 0..200 {
            let a = rng.below(12); cases.push(rng.utf8_string(a));
            let b = rng.below(12); cases.push(rng.random_bytes(b));
        }
        for b in &cases {
            let z = cbuf(b);
            for which in 0..4 {
                let (c, r) = (
                    (d.c.json_stringn_nocheck)(b"seed\0".as_ptr() as *const c_char, 4),
                    (d.rs.json_stringn_nocheck)(b"seed\0".as_ptr() as *const c_char, 4),
                );
                let (cv, rv) = match which {
                    0 => (
                        (d.c.json_string_set)(c, z.as_ptr() as *const c_char),
                        (d.rs.json_string_set)(r, z.as_ptr() as *const c_char),
                    ),
                    1 => (
                        (d.c.json_string_setn)(c, z.as_ptr() as *const c_char, b.len()),
                        (d.rs.json_string_setn)(r, z.as_ptr() as *const c_char, b.len()),
                    ),
                    2 => (
                        (d.c.json_string_set_nocheck)(c, z.as_ptr() as *const c_char),
                        (d.rs.json_string_set_nocheck)(r, z.as_ptr() as *const c_char),
                    ),
                    _ => (
                        (d.c.json_string_setn_nocheck)(c, z.as_ptr() as *const c_char, b.len()),
                        (d.rs.json_string_setn_nocheck)(r, z.as_ptr() as *const c_char, b.len()),
                    ),
                };
                eq(&format!("string set#{} {:?}", which, b), cv, rv);
                same(d, &format!("string after set#{}", which), c, r);
                free2(d, c, r);
            }
        }
    }
}

#[test]
fn integer_and_real_values() {
    let d = duo();
    let _g = lock();
    let mut rng = Rng::new(0x1E_4444);
    unsafe {
        let mut ints: Vec<i64> = vec![0, 1, -1, i64::MIN, i64::MAX, i64::MIN + 1, i64::MAX - 1];
        for _ in 0..2000 {
            ints.push(rng.next_u64() as i64);
        }
        for &i in &ints {
            let (c, r) = ((d.c.json_integer)(i), (d.rs.json_integer)(i));
            same(d, &format!("json_integer {}", i), c, r);
            eq(
                "integer_value",
                (d.c.json_integer_value)(c),
                (d.rs.json_integer_value)(r),
            );
            eq(
                "number_value",
                (d.c.json_number_value)(c).to_bits(),
                (d.rs.json_number_value)(r).to_bits(),
            );
            for &j in &[0i64, -7, i64::MIN, i64::MAX] {
                eq(
                    "integer_set",
                    (d.c.json_integer_set)(c, j),
                    (d.rs.json_integer_set)(r, j),
                );
                same(d, "after integer_set", c, r);
            }
            free2(d, c, r);
        }

        let mut reals: Vec<f64> = vec![
            0.0,
            -0.0,
            1.0,
            -1.0,
            0.5,
            f64::MIN,
            f64::MAX,
            f64::MIN_POSITIVE,
            1e-323,
            f64::NAN,
            -f64::NAN,
            f64::INFINITY,
            f64::NEG_INFINITY,
        ];
        for _ in 0..2000 {
            reals.push(rng.tame_f64());
            reals.push(rng.finite_f64());
        }
        for &f in &reals {
            let (c, r) = ((d.c.json_real)(f), (d.rs.json_real)(f));
            eq(
                &format!("json_real null {:#018x}", f.to_bits()),
                c.is_null(),
                r.is_null(),
            );
            if c.is_null() {
                continue;
            }
            same(d, "json_real", c, r);
            eq(
                "real_value",
                (d.c.json_real_value)(c).to_bits(),
                (d.rs.json_real_value)(r).to_bits(),
            );
            eq(
                "number_value",
                (d.c.json_number_value)(c).to_bits(),
                (d.rs.json_number_value)(r).to_bits(),
            );
            for &g in &[0.0f64, -0.0, 1.5, f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
                eq(
                    &format!("real_set {:#018x}", g.to_bits()),
                    (d.c.json_real_set)(c, g),
                    (d.rs.json_real_set)(r, g),
                );
                same(d, "after real_set", c, r);
            }
            free2(d, c, r);
        }
    }
}

#[test]
fn singletons() {
    let d = duo();
    let _g = lock();
    unsafe {
        for (name, cf, rf) in [
            ("json_true", d.c.json_true, d.rs.json_true),
            ("json_false", d.c.json_false, d.rs.json_false),
            ("json_null", d.c.json_null, d.rs.json_null),
        ] {
            let c = cf();
            let r = rf();
            eq(&format!("{} type", name), (*c).type_, (*r).type_);
            eq(&format!("{} refcount", name), (*c).refcount, (*r).refcount);
            eq(
                &format!("{} refcount is (size_t)-1", name),
                (*c).refcount,
                usize::MAX,
            );
            // repeated calls must return the same pointer within one library
            eq(&format!("{} identity C", name), cf() as usize, c as usize);
            eq(&format!("{} identity RUST", name), rf() as usize, r as usize);
            // json_delete on a singleton is a no-op in both
            (d.c.json_delete)(c);
            (d.rs.json_delete)(r);
            eq(&format!("{} type after delete", name), (*c).type_, (*r).type_);
        }
    }
}

// ===========================================================================
// CONFIGS 65-69, ERRORS 86-95 — equality / copying / loop check
// ===========================================================================

#[test]
fn equality_all_type_pairs_and_randomized() {
    let d = duo();
    let _g = lock();
    unsafe {
        let vals = vec![
            V::Obj(vec![]),
            V::Obj(vec![(b"a".to_vec(), V::Int(1))]),
            V::Obj(vec![(b"a".to_vec(), V::Int(2))]),
            V::Obj(vec![
                (b"a".to_vec(), V::Int(1)),
                (b"b".to_vec(), V::Int(2)),
            ]),
            // same content, opposite insertion order -> must still be equal
            V::Obj(vec![
                (b"b".to_vec(), V::Int(2)),
                (b"a".to_vec(), V::Int(1)),
            ]),
            V::Arr(vec![]),
            V::Arr(vec![V::Int(1)]),
            V::Arr(vec![V::Int(1), V::Int(2)]),
            V::Arr(vec![V::Int(2), V::Int(1)]),
            V::Str(b"".to_vec()),
            V::Str(b"a".to_vec()),
            V::StrRaw(b"a\0b".to_vec()),
            V::StrRaw(b"a\0c".to_vec()),
            V::Int(0),
            V::Int(1),
            V::Real(0.0),
            V::Real(-0.0),
            V::Real(1.0),
            V::True,
            V::False,
            V::Null,
        ];
        let cs_: Vec<*mut json_t> = vals.iter().map(|v| build(&d.c, v)).collect();
        let rs_: Vec<*mut json_t> = vals.iter().map(|v| build(&d.rs, v)).collect();
        for i in 0..vals.len() {
            for j in 0..vals.len() {
                eq(
                    &format!("json_equal {} vs {}", i, j),
                    (d.c.json_equal)(cs_[i], cs_[j]),
                    (d.rs.json_equal)(rs_[i], rs_[j]),
                );
            }
            // NULL operands (ERRORS 86, 87)
            eq(
                &format!("json_equal NULL,{}", i),
                (d.c.json_equal)(ptr::null(), cs_[i]),
                (d.rs.json_equal)(ptr::null(), rs_[i]),
            );
            eq(
                &format!("json_equal {},NULL", i),
                (d.c.json_equal)(cs_[i], ptr::null()),
                (d.rs.json_equal)(rs_[i], ptr::null()),
            );
        }
        eq(
            "json_equal NULL,NULL",
            (d.c.json_equal)(ptr::null(), ptr::null()),
            (d.rs.json_equal)(ptr::null(), ptr::null()),
        );
        for (c, r) in cs_.iter().zip(rs_.iter()) {
            decref(&d.c, *c);
            decref(&d.rs, *r);
        }

        // randomized trees
        for (i, v) in corpus(400, 0xE0_1177u64, 4).iter().enumerate() {
            let (c1, r1) = build2(d, v);
            let (c2, r2) = build2(d, v);
            eq(
                &format!("json_equal same-content #{}", i),
                (d.c.json_equal)(c1, c2),
                (d.rs.json_equal)(r1, r2),
            );
            free2(d, c1, r1);
            free2(d, c2, r2);
        }
    }
}

#[test]
fn copy_and_deep_copy() {
    let d = duo();
    let _g = lock();
    unsafe {
        let mut vals = vec![
            V::Obj(vec![]),
            V::Arr(vec![]),
            V::Str(b"s".to_vec()),
            V::StrRaw(b"a\0b".to_vec()),
            V::Int(7),
            V::Real(1.25),
            V::True,
            V::False,
            V::Null,
            V::Obj(vec![(
                b"deep".to_vec(),
                V::Arr(vec![
                    V::Obj(vec![(b"x".to_vec(), V::Arr(vec![V::Int(1), V::Null]))]),
                    V::Real(-0.5),
                ]),
            )]),
        ];
        vals.extend(corpus(300, 0xC0_9911, 4));
        for (i, v) in vals.iter().enumerate() {
            let (c, r) = build2(d, v);

            let cc = (d.c.json_copy)(c);
            let rc = (d.rs.json_copy)(r);
            eq(&format!("json_copy null #{}", i), cc.is_null(), rc.is_null());
            if !cc.is_null() {
                same(d, &format!("json_copy #{}", i), cc, rc);
                // shallow: for containers the children are the SAME objects
                eq(
                    &format!("json_copy shallow-identity #{}", i),
                    (*c).type_ == JSON_ARRAY
                        && (d.c.json_array_size)(c) > 0
                        && (d.c.json_array_get)(cc, 0) == (d.c.json_array_get)(c, 0),
                    (*r).type_ == JSON_ARRAY
                        && (d.rs.json_array_size)(r) > 0
                        && (d.rs.json_array_get)(rc, 0) == (d.rs.json_array_get)(r, 0),
                );
            }
            free2(d, cc, rc);

            let cd = (d.c.json_deep_copy)(c);
            let rd = (d.rs.json_deep_copy)(r);
            eq(
                &format!("json_deep_copy null #{}", i),
                cd.is_null(),
                rd.is_null(),
            );
            if !cd.is_null() {
                same(d, &format!("json_deep_copy #{}", i), cd, rd);
                eq(
                    &format!("deep copy is equal to source #{}", i),
                    (d.c.json_equal)(c, cd),
                    (d.rs.json_equal)(r, rd),
                );
            }
            free2(d, cd, rd);
            free2(d, c, r);
        }
        // NULL input (ERRORS 90, 92)
        eq(
            "json_copy(NULL)",
            (d.c.json_copy)(ptr::null_mut()).is_null(),
            (d.rs.json_copy)(ptr::null_mut()).is_null(),
        );
        eq(
            "json_deep_copy(NULL)",
            (d.c.json_deep_copy)(ptr::null()).is_null(),
            (d.rs.json_deep_copy)(ptr::null()).is_null(),
        );
    }
}

#[test]
fn deep_copy_detects_cycles() {
    let d = duo();
    let _g = lock();
    unsafe {
        // `json_array_append_new` / `json_object_setn_new_nocheck` reject
        // `json == value`, so every cycle has to go through a second container.

        // array cycle: a = [b], b = [a]   (ERRORS 95)
        let mut rets = Vec::new();
        for l in d.both() {
            let a = (l.json_array)();
            let b = (l.json_array)();
            eq_i(
                "append b into a",
                (l.json_array_append_new)(a, incref(b)),
                0,
            );
            eq_i(
                "append a into b",
                (l.json_array_append_new)(b, incref(a)),
                0,
            );
            let cp = (l.json_deep_copy)(a);
            rets.push(cp.is_null());
            assert!(
                cp.is_null(),
                "{}: deep_copy of a cyclic array must be NULL",
                l.which
            );
            // also from the other end
            assert!((l.json_deep_copy)(b).is_null(), "{}", l.which);
            (l.json_array_clear)(a);
            (l.json_array_clear)(b);
            decref(l, a);
            decref(l, b);
        }
        eq("array-cycle deep_copy NULL-ness", rets[0], rets[1]);

        // object cycle: o1.x = o2, o2.y = o1   (ERRORS 94)
        let mut rets = Vec::new();
        for l in d.both() {
            let o1 = (l.json_object)();
            let o2 = (l.json_object)();
            eq_i(
                "set o2 into o1",
                (l.json_object_set_new)(o1, cs("x").as_ptr(), incref(o2)),
                0,
            );
            eq_i(
                "set o1 into o2",
                (l.json_object_set_new)(o2, cs("y").as_ptr(), incref(o1)),
                0,
            );
            let cp = (l.json_deep_copy)(o1);
            rets.push(cp.is_null());
            assert!(
                cp.is_null(),
                "{}: deep_copy of a cyclic object must be NULL",
                l.which
            );
            assert!((l.json_deep_copy)(o2).is_null(), "{}", l.which);
            (l.json_object_clear)(o1);
            (l.json_object_clear)(o2);
            decref(l, o1);
            decref(l, o2);
        }
        eq("object-cycle deep_copy NULL-ness", rets[0], rets[1]);

        // mixed 3-hop cycle: arr -> obj -> arr
        let mut rets = Vec::new();
        for l in d.both() {
            let a = (l.json_array)();
            let o = (l.json_object)();
            let a2 = (l.json_array)();
            (l.json_array_append_new)(a, incref(o));
            (l.json_object_set_new)(o, cs("k").as_ptr(), incref(a2));
            (l.json_array_append_new)(a2, incref(a));
            let cp = (l.json_deep_copy)(a);
            rets.push(cp.is_null());
            assert!(cp.is_null(), "{}: 3-hop cycle must be NULL", l.which);
            (l.json_array_clear)(a);
            (l.json_object_clear)(o);
            (l.json_array_clear)(a2);
            decref(l, a);
            decref(l, o);
            decref(l, a2);
        }
        eq("3-hop-cycle deep_copy NULL-ness", rets[0], rets[1]);

        // A DAG (same node reachable twice, but no cycle) must SUCCEED.
        let mut dumps = Vec::new();
        for l in d.both() {
            let shared = (l.json_array)();
            (l.json_array_append_new)(shared, (l.json_integer)(7));
            let a = (l.json_array)();
            (l.json_array_append_new)(a, incref(shared));
            (l.json_array_append_new)(a, incref(shared));
            let cp = (l.json_deep_copy)(a);
            assert!(!cp.is_null(), "{}: DAG deep_copy must succeed", l.which);
            let s = (l.json_dumps)(cp, 0);
            dumps.push(cstr_bytes(s));
            (l.jsonp_free)(s as *mut c_void);
            decref(l, cp);
            decref(l, a);
            decref(l, shared);
        }
        eq_bytes("DAG deep_copy dump", &dumps[0], &dumps[1]);
    }
}

#[track_caller]
fn eq_i(what: &str, got: c_int, want: c_int) {
    assert_eq!(got, want, "{}", what);
}

#[test]
fn do_deep_copy_and_loop_check_low_level() {
    let d = duo();
    let _g = lock();
    unsafe {
        // CONFIGS 68: jsonp_loop_check with a caller-supplied hashtable
        for l in d.both() {
            let mut ht = hashtable_t::zeroed();
            assert_eq!((l.hashtable_init)(&mut ht), 0);
            let v = (l.json_integer)(1);
            let mut key = [0u8; 32];
            let mut klen: usize = usize::MAX;
            let r1 = (l.jsonp_loop_check)(
                &mut ht,
                v,
                key.as_mut_ptr() as *mut c_char,
                key.len(),
                &mut klen,
            );
            assert_eq!(r1, 0, "{}: first loop_check must succeed", l.which);
            assert_eq!(klen, cstr_bytes(key.as_ptr() as *const c_char).len());
            let r2 = (l.jsonp_loop_check)(
                &mut ht,
                v,
                key.as_mut_ptr() as *mut c_char,
                key.len(),
                ptr::null_mut(),
            );
            assert_eq!(r2, -1, "{}: second loop_check must return -1", l.which);
            (l.hashtable_close)(&mut ht);
            decref(l, v);
        }
        // both libraries must agree on the key length written for the same pointer
        // shape (the pointer value itself differs, so compare only the length range)
        // CONFIGS 69: do_deep_copy / do_object_update_recursive called directly
        let v = V::Obj(vec![
            (b"a".to_vec(), V::Arr(vec![V::Int(1), V::Str(b"s".to_vec())])),
            (b"b".to_vec(), V::Real(2.5)),
        ]);
        let (c, r) = build2(d, &v);
        let mut cht = hashtable_t::zeroed();
        let mut rht = hashtable_t::zeroed();
        assert_eq!((d.c.hashtable_init)(&mut cht), 0);
        assert_eq!((d.rs.hashtable_init)(&mut rht), 0);
        let cc = (d.c.do_deep_copy)(c, &mut cht);
        let rc = (d.rs.do_deep_copy)(r, &mut rht);
        eq("do_deep_copy null", cc.is_null(), rc.is_null());
        same(d, "do_deep_copy", cc, rc);
        eq("parents table drained C", cht.size, 0);
        eq("parents table drained RUST", rht.size, 0);
        free2(d, cc, rc);
        (d.c.hashtable_close)(&mut cht);
        (d.rs.hashtable_close)(&mut rht);

        // do_object_update_recursive with an external table
        let patch = V::Obj(vec![(b"b".to_vec(), V::Int(9))]);
        let (cp, rp) = build2(d, &patch);
        let mut cht = hashtable_t::zeroed();
        let mut rht = hashtable_t::zeroed();
        assert_eq!((d.c.hashtable_init)(&mut cht), 0);
        assert_eq!((d.rs.hashtable_init)(&mut rht), 0);
        eq(
            "do_object_update_recursive",
            (d.c.do_object_update_recursive)(c, cp, &mut cht),
            (d.rs.do_object_update_recursive)(r, rp, &mut rht),
        );
        same(d, "do_object_update_recursive result", c, r);
        (d.c.hashtable_close)(&mut cht);
        (d.rs.hashtable_close)(&mut rht);
        free2(d, cp, rp);
        free2(d, c, r);
    }
}

// ===========================================================================
// CONFIGS 70 — json_sprintf / json_vsprintf
// ===========================================================================

#[test]
fn sprintf_variants() {
    let d = duo();
    let _g = lock();
    unsafe {
        // json_vsprintf with a hand-built va_list, and json_sprintf variadically
        let long = "x".repeat(2000);
        let cases: Vec<(&str, Vec<u64>)> = vec![
            ("", vec![]),
            ("plain", vec![]),
            ("%d", vec![42u32 as u64]),
            ("%d-%d", vec![7u32 as u64, (-7i32) as u32 as u64]),
            ("%%", vec![]),
            ("%5d|", vec![3u32 as u64]),
            ("%c", vec![b'A' as u64]),
            ("%x", vec![0xdeadbeefu32 as u64]),
        ];
        for (fmt, words) in &cases {
            let f = cs(fmt);
            let mut cv = VaArgs::new();
            for w in words {
                cv = cv.i64(*w as i64);
            }
            let ap = cv.build();
            let cj = (d.c.json_vsprintf)(f.as_ptr(), ap);
            let mut rv = VaArgs::new();
            for w in words {
                rv = rv.i64(*w as i64);
            }
            let ap = rv.build();
            let rj = (d.rs.json_vsprintf)(f.as_ptr(), ap);
            eq(
                &format!("json_vsprintf null {:?}", fmt),
                cj.is_null(),
                rj.is_null(),
            );
            if !cj.is_null() {
                same(d, &format!("json_vsprintf {:?}", fmt), cj, rj);
            }
            free2(d, cj, rj);
        }
        // %s and %f need pointer / double slots
        let s = cs("héllo");
        let f = cs("%s");
        for l in d.both() {
            let mut va = VaArgs::new().ptr(s.as_ptr());
            let ap = va.build();
            let j = (l.json_vsprintf)(f.as_ptr(), ap);
            assert!(!j.is_null());
            let p = (l.json_string_value)(j);
            eq_bytes(&format!("{} vsprintf %s", l.which), "héllo".as_bytes(), &cstr_bytes(p));
            decref(l, j);
        }
        let f = cs("%.3f");
        let cj;
        let rj;
        {
            let mut va = VaArgs::new().f64(1.0 / 3.0);
            cj = (d.c.json_vsprintf)(f.as_ptr(), va.build());
        }
        {
            let mut va = VaArgs::new().f64(1.0 / 3.0);
            rj = (d.rs.json_vsprintf)(f.as_ptr(), va.build());
        }
        same(d, "json_vsprintf %.3f", cj, rj);
        free2(d, cj, rj);

        // long (> 1 KiB) result
        let f = cs("%s");
        let ls = cs(&long);
        let cj;
        let rj;
        {
            let mut va = VaArgs::new().ptr(ls.as_ptr());
            cj = (d.c.json_vsprintf)(f.as_ptr(), va.build());
        }
        {
            let mut va = VaArgs::new().ptr(ls.as_ptr());
            rj = (d.rs.json_vsprintf)(f.as_ptr(), va.build());
        }
        same(d, "json_vsprintf long", cj, rj);
        free2(d, cj, rj);

        // invalid UTF-8 in the formatted result -> NULL (ERRORS 73)
        let bad = [0x80u8, 0x00];
        let f = cs("%s");
        let cj;
        let rj;
        {
            let mut va = VaArgs::new().ptr(bad.as_ptr());
            cj = (d.c.json_vsprintf)(f.as_ptr(), va.build());
        }
        {
            let mut va = VaArgs::new().ptr(bad.as_ptr());
            rj = (d.rs.json_vsprintf)(f.as_ptr(), va.build());
        }
        eq("json_vsprintf invalid utf8", cj.is_null(), rj.is_null());
        assert!(cj.is_null(), "C must reject invalid UTF-8");
        free2(d, cj, rj);

        // empty result -> json_string("") (ERRORS 74)
        let f = cs("");
        let cj;
        let rj;
        {
            let mut va = VaArgs::new();
            cj = (d.c.json_vsprintf)(f.as_ptr(), va.build());
        }
        {
            let mut va = VaArgs::new();
            rj = (d.rs.json_vsprintf)(f.as_ptr(), va.build());
        }
        same(d, "json_vsprintf empty", cj, rj);
        eq("empty is a string", (*cj).type_, JSON_STRING);
        free2(d, cj, rj);

        // the variadic wrapper (exercises the naked-asm export)
        let f = cs("%d/%s/%d");
        let s = cs("mid");
        let cj = (d.c.json_sprintf)(f.as_ptr(), 11i32, s.as_ptr(), 22i32);
        let rj = (d.rs.json_sprintf)(f.as_ptr(), 11i32, s.as_ptr(), 22i32);
        same(d, "json_sprintf variadic", cj, rj);
        free2(d, cj, rj);
    }
}

// ===========================================================================
// ERRORS: NULL / wrong-type / out-of-range-enum rejection matrix
// ===========================================================================

#[test]
fn wrong_type_and_null_rejection_matrix() {
    let d = duo();
    let _g = lock();
    unsafe {
        // one value of every type in each library, plus NULL and a fabricated
        // json_t with an out-of-range json_type (ERRORS 85, 89, 91, 93, 256)
        let mut b42 = bogus(42);
        let mut bneg = bogus(-1);
        let mut b8 = bogus(8);
        let types = vec![
            V::Obj(vec![(b"k".to_vec(), V::Int(1))]),
            V::Arr(vec![V::Int(1)]),
            V::Str(b"s".to_vec()),
            V::Int(1),
            V::Real(1.0),
            V::True,
            V::False,
            V::Null,
        ];
        let mut cvals: Vec<*mut json_t> = types.iter().map(|v| build(&d.c, v)).collect();
        let mut rvals: Vec<*mut json_t> = types.iter().map(|v| build(&d.rs, v)).collect();
        // NULL and the bogus values are shared between the libraries (they are
        // plain memory, not owned by either allocator)
        for extra in [
            ptr::null_mut(),
            &mut *b42 as *mut json_t,
            &mut *bneg as *mut json_t,
            &mut *b8 as *mut json_t,
        ] {
            cvals.push(extra);
            rvals.push(extra);
        }

        let key = cs("k");
        let absent = cs("nope");
        for i in 0..cvals.len() {
            let (c, r) = (cvals[i], rvals[i]);
            let tag = format!("#{}", i);

            eq(
                &format!("object_size {}", tag),
                (d.c.json_object_size)(c),
                (d.rs.json_object_size)(r),
            );
            eq(
                &format!("object_get {}", tag),
                (d.c.json_object_get)(c, key.as_ptr()).is_null(),
                (d.rs.json_object_get)(r, key.as_ptr()).is_null(),
            );
            eq(
                &format!("object_get NULL key {}", tag),
                (d.c.json_object_get)(c, ptr::null()).is_null(),
                (d.rs.json_object_get)(r, ptr::null()).is_null(),
            );
            eq(
                &format!("object_getn NULL key {}", tag),
                (d.c.json_object_getn)(c, ptr::null(), 0).is_null(),
                (d.rs.json_object_getn)(r, ptr::null(), 0).is_null(),
            );
            eq(
                &format!("object_del {}", tag),
                (d.c.json_object_del)(c, absent.as_ptr()),
                (d.rs.json_object_del)(r, absent.as_ptr()),
            );
            eq(
                &format!("object_del NULL key {}", tag),
                (d.c.json_object_del)(c, ptr::null()),
                (d.rs.json_object_del)(r, ptr::null()),
            );
            eq(
                &format!("object_deln NULL key {}", tag),
                (d.c.json_object_deln)(c, ptr::null(), 0),
                (d.rs.json_object_deln)(r, ptr::null(), 0),
            );
            eq(
                &format!("object_clear {}", tag),
                (d.c.json_object_clear)(c),
                (d.rs.json_object_clear)(r),
            );
            eq(
                &format!("object_iter {}", tag),
                (d.c.json_object_iter)(c).is_null(),
                (d.rs.json_object_iter)(r).is_null(),
            );
            eq(
                &format!("object_iter_at {}", tag),
                (d.c.json_object_iter_at)(c, absent.as_ptr()).is_null(),
                (d.rs.json_object_iter_at)(r, absent.as_ptr()).is_null(),
            );
            eq(
                &format!("object_iter_at NULL key {}", tag),
                (d.c.json_object_iter_at)(c, ptr::null()).is_null(),
                (d.rs.json_object_iter_at)(r, ptr::null()).is_null(),
            );
            eq(
                &format!("object_iter_next NULL iter {}", tag),
                (d.c.json_object_iter_next)(c, ptr::null_mut()).is_null(),
                (d.rs.json_object_iter_next)(r, ptr::null_mut()).is_null(),
            );
            eq(
                &format!("object_iter_set_new NULL iter {}", tag),
                (d.c.json_object_iter_set_new)(c, ptr::null_mut(), (d.c.json_integer)(1)),
                (d.rs.json_object_iter_set_new)(r, ptr::null_mut(), (d.rs.json_integer)(1)),
            );
            eq(
                &format!("object_iter_set_new NULL value {}", tag),
                (d.c.json_object_iter_set_new)(c, 1usize as *mut c_void, ptr::null_mut()),
                (d.rs.json_object_iter_set_new)(r, 1usize as *mut c_void, ptr::null_mut()),
            );

            eq(
                &format!("array_size {}", tag),
                (d.c.json_array_size)(c),
                (d.rs.json_array_size)(r),
            );
            for idx in [0usize, 1, usize::MAX] {
                eq(
                    &format!("array_get {} idx={}", tag, idx),
                    (d.c.json_array_get)(c, idx).is_null(),
                    (d.rs.json_array_get)(r, idx).is_null(),
                );
                eq(
                    &format!("array_remove {} idx={}", tag, idx),
                    (d.c.json_array_remove)(c, idx),
                    (d.rs.json_array_remove)(r, idx),
                );
                eq(
                    &format!("array_set_new NULL value {} idx={}", tag, idx),
                    (d.c.json_array_set_new)(c, idx, ptr::null_mut()),
                    (d.rs.json_array_set_new)(r, idx, ptr::null_mut()),
                );
                eq(
                    &format!("array_insert_new NULL value {} idx={}", tag, idx),
                    (d.c.json_array_insert_new)(c, idx, ptr::null_mut()),
                    (d.rs.json_array_insert_new)(r, idx, ptr::null_mut()),
                );
            }
            eq(
                &format!("array_clear {}", tag),
                (d.c.json_array_clear)(c),
                (d.rs.json_array_clear)(r),
            );
            eq(
                &format!("array_append_new NULL value {}", tag),
                (d.c.json_array_append_new)(c, ptr::null_mut()),
                (d.rs.json_array_append_new)(r, ptr::null_mut()),
            );

            eq(
                &format!("string_value {}", tag),
                (d.c.json_string_value)(c).is_null(),
                (d.rs.json_string_value)(r).is_null(),
            );
            eq(
                &format!("string_length {}", tag),
                (d.c.json_string_length)(c),
                (d.rs.json_string_length)(r),
            );
            eq(
                &format!("string_set NULL {}", tag),
                (d.c.json_string_set)(c, ptr::null()),
                (d.rs.json_string_set)(r, ptr::null()),
            );
            eq(
                &format!("string_setn NULL {}", tag),
                (d.c.json_string_setn)(c, ptr::null(), 0),
                (d.rs.json_string_setn)(r, ptr::null(), 0),
            );
            eq(
                &format!("string_set_nocheck NULL {}", tag),
                (d.c.json_string_set_nocheck)(c, ptr::null()),
                (d.rs.json_string_set_nocheck)(r, ptr::null()),
            );
            eq(
                &format!("string_setn_nocheck NULL {}", tag),
                (d.c.json_string_setn_nocheck)(c, ptr::null(), 0),
                (d.rs.json_string_setn_nocheck)(r, ptr::null(), 0),
            );
            eq(
                &format!("integer_value {}", tag),
                (d.c.json_integer_value)(c),
                (d.rs.json_integer_value)(r),
            );
            eq(
                &format!("integer_set {}", tag),
                (d.c.json_integer_set)(c, 5),
                (d.rs.json_integer_set)(r, 5),
            );
            eq(
                &format!("real_value {}", tag),
                (d.c.json_real_value)(c).to_bits(),
                (d.rs.json_real_value)(r).to_bits(),
            );
            eq(
                &format!("real_set {}", tag),
                (d.c.json_real_set)(c, 1.0),
                (d.rs.json_real_set)(r, 1.0),
            );
            eq(
                &format!("real_set NaN {}", tag),
                (d.c.json_real_set)(c, f64::NAN),
                (d.rs.json_real_set)(r, f64::NAN),
            );
            eq(
                &format!("real_set Inf {}", tag),
                (d.c.json_real_set)(c, f64::INFINITY),
                (d.rs.json_real_set)(r, f64::INFINITY),
            );
            eq(
                &format!("number_value {}", tag),
                (d.c.json_number_value)(c).to_bits(),
                (d.rs.json_number_value)(r).to_bits(),
            );

            // set_new on a non-object rejects and takes ownership of `value`
            eq(
                &format!("object_set_new NULL key {}", tag),
                (d.c.json_object_set_new)(c, ptr::null(), (d.c.json_integer)(1)),
                (d.rs.json_object_set_new)(r, ptr::null(), (d.rs.json_integer)(1)),
            );
            eq(
                &format!("object_setn_new NULL key {}", tag),
                (d.c.json_object_setn_new)(c, ptr::null(), 0, (d.c.json_integer)(1)),
                (d.rs.json_object_setn_new)(r, ptr::null(), 0, (d.rs.json_integer)(1)),
            );
            eq(
                &format!("object_set_new_nocheck NULL key {}", tag),
                (d.c.json_object_set_new_nocheck)(c, ptr::null(), (d.c.json_integer)(1)),
                (d.rs.json_object_set_new_nocheck)(r, ptr::null(), (d.rs.json_integer)(1)),
            );
            eq(
                &format!("object_setn_new_nocheck NULL value {}", tag),
                (d.c.json_object_setn_new_nocheck)(c, key.as_ptr(), 1, ptr::null_mut()),
                (d.rs.json_object_setn_new_nocheck)(r, key.as_ptr(), 1, ptr::null_mut()),
            );
            // json == value self-insert (ERRORS 11, 45, 49, 52)
            if !c.is_null() {
                eq(
                    &format!("object_setn_new_nocheck self {}", tag),
                    (d.c.json_object_setn_new_nocheck)(c, key.as_ptr(), 1, incref(c)),
                    (d.rs.json_object_setn_new_nocheck)(r, key.as_ptr(), 1, incref(r)),
                );
                eq(
                    &format!("array_append_new self {}", tag),
                    (d.c.json_array_append_new)(c, incref(c)),
                    (d.rs.json_array_append_new)(r, incref(r)),
                );
                eq(
                    &format!("array_set_new self {}", tag),
                    (d.c.json_array_set_new)(c, 0, incref(c)),
                    (d.rs.json_array_set_new)(r, 0, incref(r)),
                );
                eq(
                    &format!("array_insert_new self {}", tag),
                    (d.c.json_array_insert_new)(c, 0, incref(c)),
                    (d.rs.json_array_insert_new)(r, 0, incref(r)),
                );
            }

            // update / extend against every other value
            for j in 0..cvals.len() {
                eq(
                    &format!("object_update {} <- #{}", tag, j),
                    (d.c.json_object_update)(c, cvals[j]),
                    (d.rs.json_object_update)(r, rvals[j]),
                );
                eq(
                    &format!("object_update_existing {} <- #{}", tag, j),
                    (d.c.json_object_update_existing)(c, cvals[j]),
                    (d.rs.json_object_update_existing)(r, rvals[j]),
                );
                eq(
                    &format!("object_update_missing {} <- #{}", tag, j),
                    (d.c.json_object_update_missing)(c, cvals[j]),
                    (d.rs.json_object_update_missing)(r, rvals[j]),
                );
                eq(
                    &format!("object_update_recursive {} <- #{}", tag, j),
                    (d.c.json_object_update_recursive)(c, cvals[j]),
                    (d.rs.json_object_update_recursive)(r, rvals[j]),
                );
                eq(
                    &format!("array_extend {} <- #{}", tag, j),
                    (d.c.json_array_extend)(c, cvals[j]),
                    (d.rs.json_array_extend)(r, rvals[j]),
                );
            }

            eq(
                &format!("json_copy {}", tag),
                (d.c.json_copy)(c).is_null(),
                (d.rs.json_copy)(r).is_null(),
            );
            eq(
                &format!("json_deep_copy {}", tag),
                (d.c.json_deep_copy)(c).is_null(),
                (d.rs.json_deep_copy)(r).is_null(),
            );
        }
        // iterator accessors with a NULL iterator (ERRORS 33, 34, 35, 39)
        eq(
            "object_iter_key(NULL)",
            (d.c.json_object_iter_key)(ptr::null_mut()).is_null(),
            (d.rs.json_object_iter_key)(ptr::null_mut()).is_null(),
        );
        eq(
            "object_iter_key_len(NULL)",
            (d.c.json_object_iter_key_len)(ptr::null_mut()),
            (d.rs.json_object_iter_key_len)(ptr::null_mut()),
        );
        eq(
            "object_iter_value(NULL)",
            (d.c.json_object_iter_value)(ptr::null_mut()).is_null(),
            (d.rs.json_object_iter_value)(ptr::null_mut()).is_null(),
        );
        eq(
            "object_key_to_iter(NULL)",
            (d.c.json_object_key_to_iter)(ptr::null()).is_null(),
            (d.rs.json_object_key_to_iter)(ptr::null()).is_null(),
        );
        // json_delete(NULL) is a no-op (ERRORS 84)
        (d.c.json_delete)(ptr::null_mut());
        (d.rs.json_delete)(ptr::null_mut());
        // json_delete on an out-of-range type hits `default: return` (ERRORS 85)
        (d.c.json_delete)(&mut *b42);
        (d.rs.json_delete)(&mut *b42);
        eq("bogus type survived json_delete", b42.type_, 42);

        // string constructors with NULL (ERRORS 59-63)
        eq(
            "json_string(NULL)",
            (d.c.json_string)(ptr::null()).is_null(),
            (d.rs.json_string)(ptr::null()).is_null(),
        );
        eq(
            "json_stringn(NULL)",
            (d.c.json_stringn)(ptr::null(), 0).is_null(),
            (d.rs.json_stringn)(ptr::null(), 0).is_null(),
        );
        eq(
            "json_string_nocheck(NULL)",
            (d.c.json_string_nocheck)(ptr::null()).is_null(),
            (d.rs.json_string_nocheck)(ptr::null()).is_null(),
        );
        eq(
            "json_stringn_nocheck(NULL)",
            (d.c.json_stringn_nocheck)(ptr::null(), 0).is_null(),
            (d.rs.json_stringn_nocheck)(ptr::null(), 0).is_null(),
        );
        eq(
            "jsonp_stringn_nocheck_own(NULL)",
            (d.c.jsonp_stringn_nocheck_own)(ptr::null(), 0).is_null(),
            (d.rs.jsonp_stringn_nocheck_own)(ptr::null(), 0).is_null(),
        );
        // json_real with NaN / Inf (ERRORS 77, 78)
        for v in [f64::NAN, -f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            eq(
                &format!("json_real({:#018x})", v.to_bits()),
                (d.c.json_real)(v).is_null(),
                (d.rs.json_real)(v).is_null(),
            );
        }
        for (i, (c, r)) in cvals.iter().zip(rvals.iter()).enumerate() {
            if i < types.len() {
                decref(&d.c, *c);
                decref(&d.rs, *r);
            }
        }
        let _ = (&b42, &bneg, &b8);
    }
}

#[test]
fn jsonp_stringn_nocheck_own_takes_ownership() {
    let d = duo();
    let _g = lock();
    unsafe {
        for l in d.both() {
            for content in [&b""[..], &b"abc"[..], &b"a\0b"[..], &[0x80u8][..]] {
                let buf = (l.jsonp_malloc)(content.len() + 1) as *mut u8;
                assert!(!buf.is_null());
                std::ptr::copy_nonoverlapping(content.as_ptr(), buf, content.len());
                *buf.add(content.len()) = 0;
                let j = (l.jsonp_stringn_nocheck_own)(buf as *const c_char, content.len());
                assert!(!j.is_null());
                let p = (l.json_string_value)(j);
                eq(
                    &format!("{} own value ptr is the given buffer", l.which),
                    p as usize,
                    buf as usize,
                );
                eq(
                    &format!("{} own length", l.which),
                    (l.json_string_length)(j),
                    content.len(),
                );
                decref(l, j); // frees `buf` too
            }
        }
    }
}

// ===========================================================================
// Randomized end-to-end value manipulation
// ===========================================================================

#[test]
fn randomized_value_operation_sequences() {
    let d = duo();
    let _g = lock();
    let mut rng = Rng::new(0x9A11_0F2E);
    unsafe {
        for round in 0..300 {
            let v = rand_value(&mut rng, 3);
            let (c, r) = build2(d, &v);
            for step in 0..40 {
                let op = rng.below(12);
                let ki = rng.below(6);
                let k = cs(&format!("k{}", ki));
                let idx = rng.below(6);
                let tag = format!("round={} step={} op={}", round, step, op);
                match op {
                    0 => eq(
                        &tag,
                        (d.c.json_object_set_new)(c, k.as_ptr(), (d.c.json_integer)(step)),
                        (d.rs.json_object_set_new)(r, k.as_ptr(), (d.rs.json_integer)(step)),
                    ),
                    1 => eq(
                        &tag,
                        (d.c.json_object_del)(c, k.as_ptr()),
                        (d.rs.json_object_del)(r, k.as_ptr()),
                    ),
                    2 => eq(
                        &tag,
                        (d.c.json_array_append_new)(c, (d.c.json_integer)(step)),
                        (d.rs.json_array_append_new)(r, (d.rs.json_integer)(step)),
                    ),
                    3 => eq(
                        &tag,
                        (d.c.json_array_insert_new)(c, idx, (d.c.json_integer)(step)),
                        (d.rs.json_array_insert_new)(r, idx, (d.rs.json_integer)(step)),
                    ),
                    4 => eq(
                        &tag,
                        (d.c.json_array_set_new)(c, idx, (d.c.json_integer)(step)),
                        (d.rs.json_array_set_new)(r, idx, (d.rs.json_integer)(step)),
                    ),
                    5 => eq(
                        &tag,
                        (d.c.json_array_remove)(c, idx),
                        (d.rs.json_array_remove)(r, idx),
                    ),
                    6 => {
                        let sub = rand_value(&mut rng, 2);
                        let (cs_, rs_) = build2(d, &sub);
                        eq(
                            &tag,
                            (d.c.json_object_setn_new_nocheck)(c, k.as_ptr(), ki, cs_),
                            (d.rs.json_object_setn_new_nocheck)(r, k.as_ptr(), ki, rs_),
                        );
                    }
                    7 => {
                        let sub = rand_value(&mut rng, 2);
                        let (co, ro) = build2(d, &sub);
                        eq(
                            &tag,
                            (d.c.json_object_update)(c, co),
                            (d.rs.json_object_update)(r, ro),
                        );
                        free2(d, co, ro);
                    }
                    8 => {
                        let sub = rand_value(&mut rng, 2);
                        let (co, ro) = build2(d, &sub);
                        eq(
                            &tag,
                            (d.c.json_object_update_recursive)(c, co),
                            (d.rs.json_object_update_recursive)(r, ro),
                        );
                        free2(d, co, ro);
                    }
                    9 => {
                        let cc = (d.c.json_deep_copy)(c);
                        let rc = (d.rs.json_deep_copy)(r);
                        eq(&tag, cc.is_null(), rc.is_null());
                        if !cc.is_null() {
                            same(d, &tag, cc, rc);
                        }
                        free2(d, cc, rc);
                    }
                    10 => {
                        let cc = (d.c.json_copy)(c);
                        let rc = (d.rs.json_copy)(r);
                        eq(&tag, cc.is_null(), rc.is_null());
                        if !cc.is_null() {
                            same(d, &tag, cc, rc);
                        }
                        free2(d, cc, rc);
                    }
                    _ => eq(
                        &tag,
                        (d.c.json_array_clear)(c),
                        (d.rs.json_array_clear)(r),
                    ),
                }
                same(d, &format!("state {}", tag), c, r);
                // dumps must agree too
                let (cd, rd) = dumps_both(d, c, r, JSON_ENCODE_ANY | JSON_SORT_KEYS);
                eq(&format!("dumps null {}", tag), cd.is_none(), rd.is_none());
                if let (Some(a), Some(b)) = (&cd, &rd) {
                    eq_bytes(&format!("dumps {}", tag), a, b);
                }
            }
            free2(d, c, r);
        }
    }
}
