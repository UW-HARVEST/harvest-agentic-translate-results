//! Phase B — CONFIGS.md section B: the value API, driven the way a real
//! consumer does (build state up, mutate it, observe it) and compared
//! step-by-step between the two shared objects.
mod common;

use common::*;
use std::os::raw::{c_char, c_int, c_void};

/// A textual, order-preserving description of a value, produced by the same
/// library that owns it.
unsafe fn desc(api: &Api, j: Jt) -> String {
    unsafe {
        match dumps(api, j, JSON_ENCODE_ANY) {
            Some(b) => String::from_utf8_lossy(&b).into_owned(),
            None => "<dump-failed>".to_string(),
        }
    }
}

unsafe fn ty(j: Jt) -> i64 {
    unsafe {
        if j.is_null() {
            -1
        } else {
            (*j).type_ as i64
        }
    }
}

unsafe fn rc(j: Jt) -> u64 {
    unsafe {
        if j.is_null() {
            u64::MAX
        } else {
            (*j).refcount as u64
        }
    }
}

/* ===================== B1..B6 object basics ===================== */

#[test]
fn b1_b6_object_set_get_del_clear() {
    let _g = lock();
    let script = |api: &'static Api| -> Vec<String> {
        let mut out = Vec::new();
        let mut rng = Rng::new(0xB001);
        unsafe {
            // B1
            let o = (api.json_object)();
            out.push(format!("empty size={} desc={}", (api.json_object_size)(o), desc(api, o)));
            decref(api, o);

            // B2: every setter x every size across the rehash boundary
            for setter in 0..4 {
                for n in [1usize, 7, 8, 9, 64] {
                    let o = (api.json_object)();
                    for i in 0..n {
                        let k = format!("k{i:03}");
                        let z = cstr(&k);
                        let v = (api.json_integer)(i as i64);
                        let r = match setter {
                            0 => (api.json_object_set_new)(o, z.as_ptr(), v),
                            1 => (api.json_object_setn_new)(o, z.as_ptr(), k.len(), v),
                            2 => (api.json_object_set_new_nocheck)(o, z.as_ptr(), v),
                            _ => (api.json_object_setn_new_nocheck)(o, z.as_ptr(), k.len(), v),
                        };
                        assert_eq!(r, 0);
                    }
                    out.push(format!(
                        "setter{setter} n={n} size={} {}",
                        (api.json_object_size)(o),
                        desc(api, o)
                    ));
                    // B3 overwrite existing keys
                    for i in (0..n).step_by(3) {
                        let k = format!("k{i:03}");
                        let z = cstr(&k);
                        let v = (api.json_string)(cstr(&format!("v{i}")).as_ptr());
                        assert_eq!((api.json_object_set_new)(o, z.as_ptr(), v), 0);
                    }
                    out.push(format!("after-overwrite size={} {}", (api.json_object_size)(o), desc(api, o)));

                    // B4 getn with shorter / longer / exact key_len
                    for i in 0..n.min(8) {
                        let k = format!("k{i:03}");
                        let z = cstr(&k);
                        for kl in [0usize, 1, k.len() - 1, k.len(), k.len() + 1] {
                            let g = (api.json_object_getn)(o, z.as_ptr(), kl);
                            out.push(format!("getn {k} kl={kl} -> ty={}", ty(g)));
                        }
                    }
                    out.push(format!(
                        "get absent -> {}",
                        (api.json_object_get)(o, cstr("nope").as_ptr()).is_null()
                    ));

                    // B5 del first / middle / last / absent
                    if n > 0 {
                        for idx in [0usize, n / 2, n - 1] {
                            let k = format!("k{idx:03}");
                            let z = cstr(&k);
                            out.push(format!(
                                "del {idx} -> {} size={}",
                                (api.json_object_del)(o, z.as_ptr()),
                                (api.json_object_size)(o)
                            ));
                        }
                        out.push(format!(
                            "deln absent -> {}",
                            (api.json_object_deln)(o, cstr("zzz").as_ptr(), 3)
                        ));
                        out.push(format!("after-del {}", desc(api, o)));
                    }

                    // B6 clear then repopulate
                    out.push(format!("clear -> {}", (api.json_object_clear)(o)));
                    out.push(format!("cleared size={} {}", (api.json_object_size)(o), desc(api, o)));
                    for i in 0..3 {
                        let z = cstr(&format!("r{i}"));
                        (api.json_object_set_new)(o, z.as_ptr(), (api.json_integer)(i));
                    }
                    out.push(format!("repopulated {}", desc(api, o)));
                    decref(api, o);
                }
            }

            // B2/B4 randomized keys (bytes, arbitrary lengths, via setn)
            for _ in 0..50 {
                let o = (api.json_object)();
                let mut keys: Vec<Vec<u8>> = Vec::new();
                for _ in 0..rng.below(40) {
                    let n = rng.below(10);
                    let k = rng.bytes(n).iter().map(|b| b'a' + (b % 26)).collect::<Vec<u8>>();
                    keys.push(k.clone());
                    let v = (api.json_integer)(rng.i64());
                    (api.json_object_setn_new_nocheck)(o, k.as_ptr() as *const c_char, k.len(), v);
                }
                out.push(format!("rand size={} {}", (api.json_object_size)(o), desc(api, o)));
                for k in &keys {
                    let g = (api.json_object_getn)(o, k.as_ptr() as *const c_char, k.len());
                    out.push(format!("rget ty={}", ty(g)));
                }
                decref(api, o);
            }
        }
        out
    };
    let (a, b) = both(script);
    assert_eq!(a.len(), b.len());
    for (i, (x, y)) in a.iter().zip(b.iter()).enumerate() {
        assert_eq!(x, y, "object step {i}");
    }
}

/* ===================== B7..B11 object update family ===================== */

#[test]
fn b7_b11_object_updates() {
    let _g = lock();
    let script = |api: &'static Api| -> Vec<String> {
        let mut out = Vec::new();
        unsafe {
            let mk = |spec: &[(&str, i64)]| -> Jt {
                let o = (api.json_object)();
                for (k, v) in spec {
                    (api.json_object_set_new)(o, cstr(k).as_ptr(), (api.json_integer)(*v));
                }
                o
            };
            for (name, f) in [
                ("update", api.json_object_update),
                ("existing", api.json_object_update_existing),
                ("missing", api.json_object_update_missing),
                ("recursive", api.json_object_update_recursive),
            ] {
                // disjoint
                let a = mk(&[("a", 1), ("b", 2)]);
                let b = mk(&[("c", 3), ("d", 4)]);
                out.push(format!("{name} disjoint -> {} {}", f(a, b), desc(api, a)));
                decref(api, a);
                decref(api, b);
                // overlapping
                let a = mk(&[("a", 1), ("b", 2), ("c", 3)]);
                let b = mk(&[("b", 20), ("d", 40)]);
                out.push(format!("{name} overlap -> {} {}", f(a, b), desc(api, a)));
                decref(api, a);
                decref(api, b);
                // empty other
                let a = mk(&[("a", 1)]);
                let b = (api.json_object)();
                out.push(format!("{name} empty -> {} {}", f(a, b), desc(api, a)));
                decref(api, a);
                decref(api, b);
                // self
                let a = mk(&[("a", 1), ("b", 2)]);
                out.push(format!("{name} self -> {} {}", f(a, a), desc(api, a)));
                decref(api, a);
            }

            // B10 nested three levels deep
            let deep = |api: &Api, leaf: i64| -> Jt {
                let l3 = (api.json_object)();
                (api.json_object_set_new)(l3, cstr("x").as_ptr(), (api.json_integer)(leaf));
                let l2 = (api.json_object)();
                (api.json_object_set_new)(l2, cstr("l3").as_ptr(), l3);
                let l1 = (api.json_object)();
                (api.json_object_set_new)(l1, cstr("l2").as_ptr(), l2);
                l1
            };
            let a = deep(api, 1);
            let b = deep(api, 2);
            (api.json_object_set_new)(b, cstr("extra").as_ptr(), (api.json_integer)(9));
            out.push(format!(
                "recursive deep -> {} {}",
                (api.json_object_update_recursive)(a, b),
                desc(api, a)
            ));
            decref(api, a);
            decref(api, b);

            // scalar over object and object over scalar
            let a = deep(api, 1);
            let b = (api.json_object)();
            (api.json_object_set_new)(b, cstr("l2").as_ptr(), (api.json_integer)(5));
            out.push(format!(
                "scalar-over-object -> {} {}",
                (api.json_object_update_recursive)(a, b),
                desc(api, a)
            ));
            decref(api, a);
            decref(api, b);

            let a = (api.json_object)();
            (api.json_object_set_new)(a, cstr("l2").as_ptr(), (api.json_integer)(5));
            let b = deep(api, 7);
            out.push(format!(
                "object-over-scalar -> {} {}",
                (api.json_object_update_recursive)(a, b),
                desc(api, a)
            ));
            decref(api, a);
            decref(api, b);

            // B11: do_object_update_recursive called directly with our own parents set
            let a = deep(api, 1);
            let b = deep(api, 2);
            let mut ht = HashtableT::zeroed();
            assert_eq!((api.hashtable_init)(&mut ht), 0);
            out.push(format!(
                "do_object_update_recursive -> {} {} parents={}",
                (api.do_object_update_recursive)(a, b, &mut ht),
                desc(api, a),
                ht.size
            ));
            (api.hashtable_close)(&mut ht);
            decref(api, a);
            decref(api, b);
        }
        out
    };
    let (a, b) = both(script);
    assert_eq!(a.len(), b.len());
    for (i, (x, y)) in a.iter().zip(b.iter()).enumerate() {
        assert_eq!(x, y, "update step {i}");
    }
}

/* ===================== B12..B14 object iteration ===================== */

#[test]
fn b12_b14_object_iteration() {
    let _g = lock();
    let script = |api: &'static Api| -> Vec<String> {
        let mut out = Vec::new();
        unsafe {
            for n in [0usize, 1, 8, 64] {
                let o = (api.json_object)();
                for i in 0..n {
                    (api.json_object_set_new)(
                        o,
                        cstr(&format!("k{i:03}")).as_ptr(),
                        (api.json_integer)(i as i64),
                    );
                }
                // B12 full traversal
                let mut it = (api.json_object_iter)(o);
                let mut seen = Vec::new();
                while !it.is_null() {
                    let k = (api.json_object_iter_key)(it);
                    let kl = (api.json_object_iter_key_len)(it);
                    let kb = std::slice::from_raw_parts(k as *const u8, kl).to_vec();
                    let v = (api.json_object_iter_value)(it);
                    seen.push(format!(
                        "{}={}",
                        String::from_utf8_lossy(&kb),
                        (api.json_integer_value)(v)
                    ));
                    // B12 key_to_iter round-trip
                    let it2 = (api.json_object_key_to_iter)(k);
                    assert_eq!(it2, it, "key_to_iter round-trip");
                    it = (api.json_object_iter_next)(o, it);
                }
                out.push(format!("n={n} iter {seen:?}"));

                if n > 0 {
                    // B13 iter_at + resume
                    let mid = cstr(&format!("k{:03}", n / 2));
                    let it = (api.json_object_iter_at)(o, mid.as_ptr());
                    let mut rest = Vec::new();
                    let mut cur = it;
                    while !cur.is_null() {
                        let k = (api.json_object_iter_key)(cur);
                        let kl = (api.json_object_iter_key_len)(cur);
                        rest.push(
                            String::from_utf8_lossy(std::slice::from_raw_parts(
                                k as *const u8,
                                kl,
                            ))
                            .into_owned(),
                        );
                        cur = (api.json_object_iter_next)(o, cur);
                    }
                    out.push(format!("resume {rest:?}"));

                    // B14 iter_set_new
                    let it = (api.json_object_iter_at)(o, mid.as_ptr());
                    out.push(format!(
                        "iter_set_new -> {}",
                        (api.json_object_iter_set_new)(o, it, (api.json_string)(cstr("SET").as_ptr()))
                    ));
                    out.push(format!("after-iter-set {}", desc(api, o)));
                }
                decref(api, o);
            }
        }
        out
    };
    let (a, b) = both(script);
    assert_eq!(a, b);
}

/* ===================== B15..B22 arrays ===================== */

#[test]
fn b15_b22_arrays() {
    let _g = lock();
    let script = |api: &'static Api| -> Vec<String> {
        let mut out = Vec::new();
        let mut rng = Rng::new(0xB015);
        unsafe {
            // B15
            let a = (api.json_array)();
            out.push(format!("empty size={} {}", (api.json_array_size)(a), desc(api, a)));
            decref(api, a);

            // B16 append across the growth boundaries
            for n in [1usize, 7, 8, 9, 16, 17, 100] {
                let a = (api.json_array)();
                for i in 0..n {
                    assert_eq!((api.json_array_append_new)(a, (api.json_integer)(i as i64)), 0);
                }
                out.push(format!("append n={n} size={} {}", (api.json_array_size)(a), desc(api, a)));
                // B22 get every index
                for i in 0..n {
                    out.push(format!("get {i} = {}", (api.json_integer_value)((api.json_array_get)(a, i))));
                }
                // B18 set every valid index
                for i in 0..n {
                    assert_eq!(
                        (api.json_array_set_new)(a, i, (api.json_integer)((i as i64) * 10)),
                        0
                    );
                }
                out.push(format!("after-set {}", desc(api, a)));
                decref(api, a);
            }

            // B17 insert at 0 / middle / entries
            for n in 0..18usize {
                for pos in [0usize, n / 2, n] {
                    let a = (api.json_array)();
                    for i in 0..n {
                        (api.json_array_append_new)(a, (api.json_integer)(i as i64));
                    }
                    out.push(format!(
                        "insert n={n} pos={pos} -> {} {}",
                        (api.json_array_insert_new)(a, pos, (api.json_integer)(-1)),
                        desc(api, a)
                    ));
                    decref(api, a);
                }
            }

            // B19 remove 0 / middle / last
            for n in 1..18usize {
                for pos in [0usize, n / 2, n - 1] {
                    let a = (api.json_array)();
                    for i in 0..n {
                        (api.json_array_append_new)(a, (api.json_integer)(i as i64));
                    }
                    out.push(format!(
                        "remove n={n} pos={pos} -> {} {}",
                        (api.json_array_remove)(a, pos),
                        desc(api, a)
                    ));
                    decref(api, a);
                }
            }

            // B20 clear then re-append
            for n in [0usize, 1, 100] {
                let a = (api.json_array)();
                for i in 0..n {
                    (api.json_array_append_new)(a, (api.json_integer)(i as i64));
                }
                out.push(format!("clear n={n} -> {}", (api.json_array_clear)(a)));
                out.push(format!("cleared size={} {}", (api.json_array_size)(a), desc(api, a)));
                for i in 0..3 {
                    (api.json_array_append_new)(a, (api.json_integer)(100 + i));
                }
                out.push(format!("re-appended {}", desc(api, a)));
                decref(api, a);
            }

            // B21 extend
            for m in [0usize, 1, 100] {
                for n in [0usize, 1, 100] {
                    let a = (api.json_array)();
                    for i in 0..n {
                        (api.json_array_append_new)(a, (api.json_integer)(i as i64));
                    }
                    let b = (api.json_array)();
                    for i in 0..m {
                        (api.json_array_append_new)(b, (api.json_integer)(1000 + i as i64));
                    }
                    out.push(format!(
                        "extend n={n} m={m} -> {} size={}",
                        (api.json_array_extend)(a, b),
                        (api.json_array_size)(a)
                    ));
                    out.push(desc(api, a));
                    decref(api, a);
                    decref(api, b);
                }
            }
            // extend with itself
            let a = (api.json_array)();
            for i in 0..5 {
                (api.json_array_append_new)(a, (api.json_integer)(i));
            }
            out.push(format!(
                "extend self -> {} size={} {}",
                (api.json_array_extend)(a, a),
                (api.json_array_size)(a),
                desc(api, a)
            ));
            decref(api, a);

            // randomized mixed operation sequences
            for _ in 0..80 {
                let a = (api.json_array)();
                for _ in 0..rng.below(60) {
                    let sz = (api.json_array_size)(a);
                    match rng.below(5) {
                        0 => {
                            (api.json_array_append_new)(a, (api.json_integer)(rng.i64()));
                        }
                        1 => {
                            let i = rng.below(sz + 2);
                            (api.json_array_insert_new)(a, i, (api.json_integer)(rng.i64()));
                        }
                        2 => {
                            let i = rng.below(sz + 2);
                            (api.json_array_set_new)(a, i, (api.json_integer)(rng.i64()));
                        }
                        3 => {
                            let i = rng.below(sz + 2);
                            (api.json_array_remove)(a, i);
                        }
                        _ => {
                            (api.json_array_append_new)(a, (api.json_array)());
                        }
                    }
                }
                out.push(format!("rand {} {}", (api.json_array_size)(a), desc(api, a)));
                decref(api, a);
            }
        }
        out
    };
    let (a, b) = both(script);
    assert_eq!(a.len(), b.len());
    for (i, (x, y)) in a.iter().zip(b.iter()).enumerate() {
        assert_eq!(x, y, "array step {i}");
    }
}

/* ===================== B23..B29 scalars ===================== */

#[test]
fn b23_b25_strings() {
    let _g = lock();
    let script = |api: &'static Api| -> Vec<String> {
        let mut out = Vec::new();
        let mut rng = Rng::new(0xB023);
        let mut inputs: Vec<Vec<u8>> = vec![
            b"".to_vec(),
            b"a".to_vec(),
            b"hello".to_vec(),
            "é".as_bytes().to_vec(),
            "€".as_bytes().to_vec(),
            "𝄞".as_bytes().to_vec(),
            "a\u{7f}\u{80}".as_bytes().to_vec(),
            b"a\0b".to_vec(),
            b"\xff\xfe".to_vec(),
            b"\xc2".to_vec(),
            b"\xe2\x82".to_vec(),
            b"\xed\xa0\x80".to_vec(),
        ];
        for _ in 0..200 {
            inputs.push(rng.spicy_string(10).into_bytes());
        }
        for _ in 0..200 {
            let n = rng.below(12);
            inputs.push(rng.bytes(n));
        }
        unsafe {
            for src in &inputs {
                let z = nul_terminated(src);
                for ctor in 0..4 {
                    let s = match ctor {
                        0 => (api.json_string)(z.as_ptr()),
                        1 => (api.json_stringn)(z.as_ptr(), src.len()),
                        2 => (api.json_string_nocheck)(z.as_ptr()),
                        _ => (api.json_stringn_nocheck)(z.as_ptr(), src.len()),
                    };
                    if s.is_null() {
                        out.push(format!("ctor{ctor} {src:?} -> NULL"));
                        continue;
                    }
                    let len = (api.json_string_length)(s);
                    let val = std::slice::from_raw_parts(
                        (api.json_string_value)(s) as *const u8,
                        len,
                    )
                    .to_vec();
                    out.push(format!(
                        "ctor{ctor} {src:?} len={len} val={val:?} dump={:?}",
                        dumps(api, s, JSON_ENCODE_ANY)
                    ));
                    // B24: all four setters, shorter / longer / empty
                    for repl in [&b""[..], &b"x"[..], &b"much longer replacement"[..]] {
                        let rz = nul_terminated(repl);
                        for setter in 0..4 {
                            let r = match setter {
                                0 => (api.json_string_set)(s, rz.as_ptr()),
                                1 => (api.json_string_setn)(s, rz.as_ptr(), repl.len()),
                                2 => (api.json_string_set_nocheck)(s, rz.as_ptr()),
                                _ => (api.json_string_setn_nocheck)(s, rz.as_ptr(), repl.len()),
                            };
                            let l = (api.json_string_length)(s);
                            let v = std::slice::from_raw_parts(
                                (api.json_string_value)(s) as *const u8,
                                l,
                            )
                            .to_vec();
                            out.push(format!("set{setter} {repl:?} -> {r} len={l} val={v:?}"));
                        }
                    }
                    decref(api, s);
                }
            }
        }
        out
    };
    let (a, b) = both(script);
    assert_eq!(a.len(), b.len());
    for (i, (x, y)) in a.iter().zip(b.iter()).enumerate() {
        assert_eq!(x, y, "string step {i}");
    }
}

#[test]
fn b26_b29_numbers_and_singletons() {
    let _g = lock();
    let script = |api: &'static Api| -> Vec<String> {
        let mut out = Vec::new();
        let mut rng = Rng::new(0xB026);
        unsafe {
            // B26 integers
            let mut ints: Vec<i64> = vec![
                0,
                1,
                -1,
                i32::MAX as i64,
                i32::MIN as i64,
                i64::MAX,
                i64::MIN,
                9007199254740993,
            ];
            for _ in 0..500 {
                ints.push(rng.i64());
            }
            for v in ints {
                let j = (api.json_integer)(v);
                out.push(format!(
                    "int {v} ty={} val={} num={} dump={:?}",
                    ty(j),
                    (api.json_integer_value)(j),
                    (api.json_number_value)(j),
                    dumps(api, j, JSON_ENCODE_ANY)
                ));
                out.push(format!("set -> {}", (api.json_integer_set)(j, v.wrapping_neg())));
                out.push(format!("after {}", (api.json_integer_value)(j)));
                decref(api, j);
            }

            // B27 reals
            let mut reals: Vec<f64> = vec![
                0.0,
                -0.0,
                1.0,
                -1.0,
                f64::MIN_POSITIVE,
                5e-324,
                f64::MAX,
                f64::MIN,
                3.141592653589793,
                1e-5,
                1e16,
                1e17,
            ];
            for _ in 0..300 {
                reals.push(rng.tame_f64());
            }
            for _ in 0..300 {
                reals.push(rng.finite_f64());
            }
            for v in reals {
                let j = (api.json_real)(v);
                if j.is_null() {
                    out.push(format!("real {v:?} -> NULL"));
                    continue;
                }
                out.push(format!(
                    "real {:?} ty={} val={:?} num={:?} dump={:?}",
                    v.to_bits(),
                    ty(j),
                    (api.json_real_value)(j).to_bits(),
                    (api.json_number_value)(j).to_bits(),
                    dumps(api, j, JSON_ENCODE_ANY)
                ));
                out.push(format!("set -> {}", (api.json_real_set)(j, -v)));
                out.push(format!("after {:?}", (api.json_real_value)(j).to_bits()));
                decref(api, j);
            }

            // B28 json_number_value on every type
            let samples: Vec<Jt> = vec![
                (api.json_object)(),
                (api.json_array)(),
                (api.json_string)(cstr("s").as_ptr()),
                (api.json_integer)(42),
                (api.json_real)(1.5),
                (api.json_true)(),
                (api.json_false)(),
                (api.json_null)(),
            ];
            for s in &samples {
                out.push(format!(
                    "num_value ty={} -> {:?} int={} real={:?} strlen={} strval_null={}",
                    ty(*s),
                    (api.json_number_value)(*s).to_bits(),
                    (api.json_integer_value)(*s),
                    (api.json_real_value)(*s).to_bits(),
                    (api.json_string_length)(*s),
                    (api.json_string_value)(*s).is_null()
                ));
            }
            for s in &samples {
                decref(api, *s);
            }

            // B29 singletons
            let t1 = (api.json_true)();
            let t2 = (api.json_true)();
            out.push(format!("true same={} rc={}", t1 == t2, rc(t1)));
            let f1 = (api.json_false)();
            out.push(format!("false rc={} ty={}", rc(f1), ty(f1)));
            let n1 = (api.json_null)();
            out.push(format!("null rc={} ty={}", rc(n1), ty(n1)));
            incref(api, t1);
            decref(api, t1);
            out.push(format!("true rc after incref/decref = {}", rc(t1)));
        }
        out
    };
    let (a, b) = both(script);
    assert_eq!(a.len(), b.len());
    for (i, (x, y)) in a.iter().zip(b.iter()).enumerate() {
        assert_eq!(x, y, "number step {i}");
    }
}

/* ===================== B30..B33 equality / copying ===================== */

/// Build a value from a JSON text using the library's own parser.
unsafe fn build(api: &Api, text: &str) -> Jt {
    unsafe { (api.json_loads)(cstr(text).as_ptr(), JSON_DECODE_ANY, std::ptr::null_mut()) }
}

#[test]
fn b30_json_equal() {
    let _g = lock();
    let texts = [
        "null", "true", "false", "0", "1", "-1", "1.0", "1.5", "\"\"", "\"a\"", "\"b\"",
        "[]", "[1]", "[1,2]", "[2,1]", "{}", "{\"a\":1}", "{\"a\":2}", "{\"b\":1}",
        "{\"a\":1,\"b\":2}", "{\"b\":2,\"a\":1}", "[[1],[2]]", "[{\"a\":[1,2]}]",
        "{\"a\":{\"b\":{\"c\":[1,2,3]}}}",
    ];
    let script = |api: &'static Api| -> Vec<c_int> {
        let mut out = Vec::new();
        unsafe {
            let vals: Vec<Jt> = texts.iter().map(|t| build(api, t)).collect();
            for a in &vals {
                for b in &vals {
                    out.push((api.json_equal)(*a, *b));
                }
            }
            // equal content but distinct pointers via deep copy
            for a in &vals {
                let c = (api.json_deep_copy)(*a);
                out.push((api.json_equal)(*a, c));
                decref(api, c);
            }
            for v in vals {
                decref(api, v);
            }
        }
        out
    };
    let (a, b) = both(script);
    assert_eq!(a, b);
}

#[test]
fn b31_b33_copy_deep_copy_delete() {
    let _g = lock();
    let texts = [
        "null", "true", "false", "42", "1.5", "\"str\"", "[]", "{}", "[1,[2,[3,[4]]]]",
        "{\"a\":{\"b\":{\"c\":{\"d\":[1,2,{\"e\":null}]}}}}",
        "[{\"k\":\"v\"},[1,2,3],\"x\",1,2.5,true,false,null]",
    ];
    let script = |api: &'static Api| -> Vec<String> {
        let mut out = Vec::new();
        unsafe {
            for t in texts {
                let v = build(api, t);
                // B31 shallow copy: children shared
                let c = (api.json_copy)(v);
                out.push(format!("copy {t} -> {} same_ptr={}", desc(api, c), c == v));
                if ty(v) == JSON_ARRAY as i64 && (api.json_array_size)(v) > 0 {
                    out.push(format!(
                        "shared child = {}",
                        (api.json_array_get)(v, 0) == (api.json_array_get)(c, 0)
                    ));
                }
                decref(api, c);
                // B32 deep copy: children distinct
                let d = (api.json_deep_copy)(v);
                out.push(format!("deep {t} -> {}", desc(api, d)));
                if ty(v) == JSON_ARRAY as i64 && (api.json_array_size)(v) > 0 {
                    let c0 = (api.json_array_get)(v, 0);
                    let d0 = (api.json_array_get)(d, 0);
                    out.push(format!(
                        "deep child distinct = {}",
                        (c0 != d0) || ty(c0) >= JSON_TRUE as i64
                    ));
                }
                decref(api, d);
                // B32 do_deep_copy driven directly with our own parents table
                let mut ht = HashtableT::zeroed();
                assert_eq!((api.hashtable_init)(&mut ht), 0);
                let e = (api.do_deep_copy)(v, &mut ht);
                out.push(format!("do_deep_copy {t} -> {} parents={}", desc(api, e), ht.size));
                (api.hashtable_close)(&mut ht);
                decref(api, e);
                // B33 refcount down to zero
                out.push(format!("rc before delete = {}", rc(v)));
                decref(api, v);
            }
        }
        out
    };
    let (a, b) = both(script);
    assert_eq!(a.len(), b.len());
    for (i, (x, y)) in a.iter().zip(b.iter()).enumerate() {
        assert_eq!(x, y, "copy step {i}");
    }
}

/* Silence unused-import warnings when a helper is not used in some builds. */
#[allow(unused)]
fn _unused(_: *mut c_void) {}
