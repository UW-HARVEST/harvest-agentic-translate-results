//! Phase B/C — json_equal / json_copy / json_deep_copy / do_deep_copy.
//! CONFIGS rows 49-51 · ERRORS rows 83-90.
mod common;
use common::*;

/// Builds the same random tree in `api` for a given RNG seed.
unsafe fn tree(api: &'static Api, rng: &mut Rng, depth: u32) -> *mut JsonT {
    unsafe {
        let pick = if depth == 0 { rng.below(6) } else { rng.below(8) };
        match pick {
            0 => (api.json_null)(),
            1 => {
                if rng.bool() {
                    (api.json_true)()
                } else {
                    (api.json_false)()
                }
            }
            2 => (api.json_integer)(rng.range_i64(-1000, 1000)),
            3 => (api.json_real)(rng.range_i64(-1000, 1000) as f64 / 8.0),
            4 => (api.json_string)(cs(&rng.key(8)).as_ptr()),
            5 => (api.json_string)(cs(&rng.utf8(6)).as_ptr()),
            6 => {
                let a = (api.json_array)();
                let n = rng.below(5);
                for _ in 0..n {
                    let v = tree(api, rng, depth - 1);
                    (api.json_array_append_new)(a, v);
                }
                a
            }
            _ => {
                let o = (api.json_object)();
                let n = rng.below(5);
                for i in 0..n {
                    let k = cs(&format!("{}{}", rng.key(5), i));
                    let v = tree(api, rng, depth - 1);
                    (api.json_object_set_new_nocheck)(o, k.as_ptr(), v);
                }
                o
            }
        }
    }
}

/// Builds the SAME tree in both libraries by replaying one RNG seed twice.
unsafe fn twin(seed: u64, depth: u32) -> (*mut JsonT, *mut JsonT) {
    unsafe {
        let mut r1 = Rng::new(seed);
        let mut r2 = Rng::new(seed);
        let a = tree(c(), &mut r1, depth);
        let b = tree(r(), &mut r2, depth);
        assert_eq!(shape(c(), a), shape(r(), b), "twin setup (seed {seed:#x})");
        (a, b)
    }
}

/* ---------------- CONFIGS 49 · ERRORS 83-85 ---------------- */

#[test]
fn json_equal_differential() {
    let _g = dtoa_guard();
    unsafe {
        let mut rng = Rng::new(0xE0_0001);
        for trial in 0..1500 {
            let s1 = rng.next_u64();
            // same seed => equal trees; different seed => usually unequal
            let s2 = if rng.bool() { s1 } else { rng.next_u64() };
            let (ca, ra) = twin(s1, 4);
            let (cb, rb) = twin(s2, 4);
            let cv = (c().json_equal)(ca, cb);
            let rv = (r().json_equal)(ra, rb);
            assert_eq!(cv, rv, "trial {trial} json_equal (s1={s1:#x}, s2={s2:#x})");
            if s1 == s2 {
                assert_eq!(cv, 1, "identical trees must compare equal");
            }
            // symmetry and reflexivity must match too
            assert_eq!((c().json_equal)(cb, ca), (r().json_equal)(rb, ra));
            assert_eq!((c().json_equal)(ca, ca), (r().json_equal)(ra, ra));
            decref(c(), ca);
            decref(r(), ra);
            decref(c(), cb);
            decref(r(), rb);
        }

        // ERRORS 83: NULL operands
        for api in both() {
            let x = (api.json_integer)(1);
            assert_eq!((api.json_equal)(std::ptr::null(), x), 0);
            assert_eq!((api.json_equal)(x, std::ptr::null()), 0);
            assert_eq!(
                (api.json_equal)(std::ptr::null(), std::ptr::null()),
                0
            );
            decref(api, x);
        }

        // ERRORS 84: cross-type comparisons; every ordered pair of types
        for api in both() {
            let vals: Vec<*mut JsonT> = vec![
                (api.json_object)(),
                (api.json_array)(),
                (api.json_string)(cs("s").as_ptr()),
                (api.json_integer)(0),
                (api.json_real)(0.0),
                (api.json_true)(),
                (api.json_false)(),
                (api.json_null)(),
            ];
            let mut m = Vec::new();
            for &a in &vals {
                for &b in &vals {
                    m.push((api.json_equal)(a, b));
                }
            }
            if api.tag == "C" {
                C_EQ_MATRIX.set(m).ok();
            } else {
                assert_eq!(
                    C_EQ_MATRIX.get().unwrap(),
                    &m,
                    "cross-type json_equal matrix"
                );
            }
            for v in vals {
                decref(api, v);
            }
        }

        // 0 == -0.0 for reals; integer 1 != real 1.0 (different types)
        for api in both() {
            let z = (api.json_real)(0.0);
            let nz = (api.json_real)(-0.0);
            assert_eq!((api.json_equal)(z, nz), 1, "{}: 0.0 == -0.0", api.tag);
            let i1 = (api.json_integer)(1);
            let r1 = (api.json_real)(1.0);
            assert_eq!((api.json_equal)(i1, r1), 0);
            decref(api, z);
            decref(api, nz);
            decref(api, i1);
            decref(api, r1);
        }

        // ERRORS 85: out-of-range type on BOTH sides hits the `default:` arm
        for api in both() {
            let a = (api.json_integer)(1);
            let b = (api.json_integer)(1);
            (*a).type_ = 42;
            (*b).type_ = 42;
            assert_eq!((api.json_equal)(a, b), 0, "{}: ERRORS 85", api.tag);
            (*a).type_ = JSON_INTEGER;
            (*b).type_ = JSON_INTEGER;
            decref(api, a);
            decref(api, b);
        }
    }
}

static C_EQ_MATRIX: std::sync::OnceLock<Vec<i32>> = std::sync::OnceLock::new();

/* ---------------- CONFIGS 50 · ERRORS 86, 87 ---------------- */

#[test]
fn json_copy_differential() {
    let _g = dtoa_guard();
    unsafe {
        let mut rng = Rng::new(0xE0_0002);
        for trial in 0..1200 {
            let s = rng.next_u64();
            let (ca, ra) = twin(s, 4);
            let cc = (c().json_copy)(ca);
            let rc = (r().json_copy)(ra);
            assert_eq!(cc.is_null(), rc.is_null(), "trial {trial} json_copy null-ness");
            if !cc.is_null() {
                assert_eq!(shape(c(), cc), shape(r(), rc), "trial {trial} json_copy shape");
                // shallow copy: equal but (for containers) a different object
                assert_eq!(
                    (c().json_equal)(ca, cc),
                    (r().json_equal)(ra, rc),
                    "trial {trial} copy equality"
                );
                let cshared = cc == ca;
                let rshared = rc == ra;
                assert_eq!(cshared, rshared, "trial {trial} copy identity");
                if !cshared {
                    decref(c(), cc);
                    decref(r(), rc);
                }
            }
            decref(c(), ca);
            decref(r(), ra);
        }

        // singletons: json_copy returns the SAME pointer, without increfing
        for api in both() {
            for mk in [api.json_true, api.json_false, api.json_null] {
                let v = mk();
                let cp = (api.json_copy)(v);
                assert_eq!(cp, v, "{}: json_copy(singleton) is identity", api.tag);
                assert_eq!((*v).refcount, usize::MAX);
            }
        }

        // ERRORS 86: NULL
        for api in both() {
            assert!((api.json_copy)(std::ptr::null_mut()).is_null());
            // ERRORS 87: out-of-range type
            let p = (api.json_integer)(3);
            (*p).type_ = 200;
            assert!((api.json_copy)(p).is_null(), "{}: ERRORS 87", api.tag);
            (*p).type_ = JSON_INTEGER;
            decref(api, p);
        }
    }
}

/* ---------------- CONFIGS 51 · ERRORS 88-90 ---------------- */

#[test]
fn json_deep_copy_differential() {
    let _g = dtoa_guard();
    unsafe {
        let mut rng = Rng::new(0xE0_0003);
        for trial in 0..1200 {
            let s = rng.next_u64();
            let (ca, ra) = twin(s, 5);
            let cc = (c().json_deep_copy)(ca);
            let rc = (r().json_deep_copy)(ra);
            assert_eq!(cc.is_null(), rc.is_null(), "trial {trial} deep_copy null-ness");
            if !cc.is_null() {
                assert_eq!(shape(c(), cc), shape(r(), rc), "trial {trial} deep_copy shape");
                assert_eq!(
                    (c().json_equal)(ca, cc),
                    (r().json_equal)(ra, rc),
                    "trial {trial} deep_copy equality"
                );
                assert_eq!((c().json_equal)(ca, cc), 1);
                // mutating the copy must not change the original in either
                if (*cc).type_ == JSON_ARRAY && (c().json_array_size)(cc) > 0 {
                    (c().json_array_set_new)(cc, 0, (c().json_string)(cs("MUT").as_ptr()));
                    (r().json_array_set_new)(rc, 0, (r().json_string)(cs("MUT").as_ptr()));
                    assert_eq!(shape(c(), ca), shape(r(), ra), "original unchanged");
                    assert_eq!(shape(c(), cc), shape(r(), rc), "copy mutated");
                }
                let cshared = cc == ca;
                let rshared = rc == ra;
                assert_eq!(cshared, rshared);
                if !cshared {
                    decref(c(), cc);
                    decref(r(), rc);
                }
            }
            decref(c(), ca);
            decref(r(), ra);
        }

        // ERRORS 88: NULL
        for api in both() {
            assert!((api.json_deep_copy)(std::ptr::null()).is_null());
            // ERRORS 89: out-of-range type
            let p = (api.json_integer)(3);
            (*p).type_ = 8; // one past JSON_NULL
            assert!((api.json_deep_copy)(p).is_null(), "{}: ERRORS 89", api.tag);
            (*p).type_ = -1;
            assert!((api.json_deep_copy)(p).is_null());
            (*p).type_ = i32::MAX;
            assert!((api.json_deep_copy)(p).is_null());
            (*p).type_ = JSON_INTEGER;
            decref(api, p);
        }

        // ERRORS 90: cycles => NULL
        let mut cyc = Vec::new();
        for api in both() {
            // array cycle
            let a = (api.json_array)();
            let b = (api.json_array)();
            (api.json_array_append_new)(a, incref(b));
            (api.json_array_append_new)(b, incref(a));
            let r1 = (api.json_deep_copy)(a).is_null();
            // object cycle
            let o1 = (api.json_object)();
            let o2 = (api.json_object)();
            (api.json_object_set_new_nocheck)(o1, cs("o2").as_ptr(), incref(o2));
            (api.json_object_set_new_nocheck)(o2, cs("o1").as_ptr(), incref(o1));
            let r2 = (api.json_deep_copy)(o1).is_null();
            // diamond (shared but acyclic) must SUCCEED
            let leaf = (api.json_array)();
            (api.json_array_append_new)(leaf, (api.json_integer)(9));
            let d = (api.json_array)();
            (api.json_array_append_new)(d, incref(leaf));
            (api.json_array_append_new)(d, incref(leaf));
            let dc = (api.json_deep_copy)(d);
            let r3 = dc.is_null();
            let s3 = if dc.is_null() {
                "<NULL>".to_string()
            } else {
                shape(api, dc)
            };
            cyc.push((r1, r2, r3, s3));
        }
        assert_eq!(cyc[0], cyc[1], "ERRORS 90: cycle / diamond handling");
        assert!(cyc[0].0, "array cycle must yield NULL");
        assert!(cyc[0].1, "object cycle must yield NULL");

        // do_deep_copy with a caller-supplied parents hashtable
        let mut cht = Box::new(Hashtable::default());
        let mut rht = Box::new(Hashtable::default());
        assert_eq!((c().hashtable_init)(&mut *cht), 0);
        assert_eq!((r().hashtable_init)(&mut *rht), 0);
        let (ca, ra) = twin(0x1234_5678, 4);
        let cc = (c().do_deep_copy)(ca, &mut *cht);
        let rc = (r().do_deep_copy)(ra, &mut *rht);
        assert_eq!(cc.is_null(), rc.is_null(), "do_deep_copy null-ness");
        if !cc.is_null() {
            assert_eq!(shape(c(), cc), shape(r(), rc), "do_deep_copy shape");
        }
        assert_eq!(cht.size, rht.size, "parents set left clean");
        (c().hashtable_close)(&mut *cht);
        (r().hashtable_close)(&mut *rht);
        decref(c(), ca);
        decref(r(), ra);
    }
}
