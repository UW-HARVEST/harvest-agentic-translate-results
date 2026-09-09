//! Phase B — CONFIGS.md sections D (objects), E (arrays), F (scalars, equality,
//! copying).
//!
//! Each test drives an identical *operation script* against both libraries and
//! compares the return value of every call plus the resulting structure.

mod common;
use common::*;
use std::os::raw::{c_char, c_void};

/// Rendering used to compare a whole value; insertion order is preserved so an
/// ordering divergence shows up.
const R_ORD: usize = JSON_ENCODE_ANY;
const R_SORT: usize = JSON_ENCODE_ANY | JSON_SORT_KEYS;

fn assert_same(p: &Pair, cj: json_ptr, rj: json_ptr, ctx: &str) {
    unsafe {
        assert_eq!(cj.is_null(), rj.is_null(), "{ctx}: NULL-ness");
        if cj.is_null() {
            return;
        }
        assert_eq!(typeof_json(cj), typeof_json(rj), "{ctx}: json_type");
        for f in [R_ORD, R_SORT, R_ORD | JSON_ENSURE_ASCII] {
            let cd = p.c.dumps(cj, f);
            let rd = p.r.dumps(rj, f);
            assert_eq!(
                cd,
                rd,
                "{ctx}: dumps(0x{f:x}):\n  C   = {}\n  RUST= {}",
                show(&cd),
                show(&rd)
            );
        }
    }
}

// ===========================================================================
// D — objects
// ===========================================================================

#[derive(Debug, Clone)]
enum ObjOp {
    /// `json_object_setn_new_nocheck` — the low-level, length-aware setter.
    SetnNoCheck(Vec<u8>, Node),
    /// `json_object_setn_new` — the UTF-8-checked, length-aware setter.
    Setn(Vec<u8>, Node),
    /// `json_object_set_new` — NUL-terminated wrapper.
    Set(Vec<u8>, Node),
    /// `json_object_set_new_nocheck` — NUL-terminated nocheck wrapper.
    SetNoCheck(Vec<u8>, Node),
    Getn(Vec<u8>),
    Get(Vec<u8>),
    Deln(Vec<u8>),
    Del(Vec<u8>),
    Clear,
    Size,
    Traverse,
    IterAt(Vec<u8>),
    /// `json_object_iter_set_new` on the iterator at the given key.
    IterSetAt(Vec<u8>, Node),
    /// `json_object_key_to_iter` on the key returned by `iter_at`.
    KeyToIter(Vec<u8>),
}

fn object_script(ops: &[ObjOp]) {
    unsafe {
        let p = pair();
        let co = p.c.json_object();
        let ro = p.r.json_object();
        assert!(!co.is_null() && !ro.is_null());

        for (n, op) in ops.iter().enumerate() {
            let ctx = format!("op {n}: {op:?}");
            match op {
                ObjOp::SetnNoCheck(k, v) => {
                    let cv = v.build(&p.c);
                    let rv = v.build(&p.r);
                    let cr = p
                        .c
                        .json_object_setn_new_nocheck(co, k.as_ptr() as *const c_char, k.len(), cv);
                    let rr = p
                        .r
                        .json_object_setn_new_nocheck(ro, k.as_ptr() as *const c_char, k.len(), rv);
                    assert_eq!(cr, rr, "{ctx}");
                }
                ObjOp::Setn(k, v) => {
                    let cv = v.build(&p.c);
                    let rv = v.build(&p.r);
                    let cr =
                        p.c.json_object_setn_new(co, k.as_ptr() as *const c_char, k.len(), cv);
                    let rr =
                        p.r.json_object_setn_new(ro, k.as_ptr() as *const c_char, k.len(), rv);
                    assert_eq!(cr, rr, "{ctx}");
                }
                ObjOp::Set(k, v) => {
                    let z = raw_z(k);
                    let cv = v.build(&p.c);
                    let rv = v.build(&p.r);
                    let cr = p.c.json_object_set_new(co, z.as_ptr() as *const c_char, cv);
                    let rr = p.r.json_object_set_new(ro, z.as_ptr() as *const c_char, rv);
                    assert_eq!(cr, rr, "{ctx}");
                }
                ObjOp::SetNoCheck(k, v) => {
                    let z = raw_z(k);
                    let cv = v.build(&p.c);
                    let rv = v.build(&p.r);
                    let cr = p
                        .c
                        .json_object_set_new_nocheck(co, z.as_ptr() as *const c_char, cv);
                    let rr = p
                        .r
                        .json_object_set_new_nocheck(ro, z.as_ptr() as *const c_char, rv);
                    assert_eq!(cr, rr, "{ctx}");
                }
                ObjOp::Getn(k) => {
                    let cv = p.c.json_object_getn(co, k.as_ptr() as *const c_char, k.len());
                    let rv = p.r.json_object_getn(ro, k.as_ptr() as *const c_char, k.len());
                    assert_same(p, cv, rv, &ctx);
                }
                ObjOp::Get(k) => {
                    let z = raw_z(k);
                    let cv = p.c.json_object_get(co, z.as_ptr() as *const c_char);
                    let rv = p.r.json_object_get(ro, z.as_ptr() as *const c_char);
                    assert_same(p, cv, rv, &ctx);
                }
                ObjOp::Deln(k) => {
                    let cr = p.c.json_object_deln(co, k.as_ptr() as *const c_char, k.len());
                    let rr = p.r.json_object_deln(ro, k.as_ptr() as *const c_char, k.len());
                    assert_eq!(cr, rr, "{ctx}");
                }
                ObjOp::Del(k) => {
                    let z = raw_z(k);
                    let cr = p.c.json_object_del(co, z.as_ptr() as *const c_char);
                    let rr = p.r.json_object_del(ro, z.as_ptr() as *const c_char);
                    assert_eq!(cr, rr, "{ctx}");
                }
                ObjOp::Clear => {
                    let cr = p.c.json_object_clear(co);
                    let rr = p.r.json_object_clear(ro);
                    assert_eq!(cr, rr, "{ctx}");
                }
                ObjOp::Size => {
                    assert_eq!(
                        p.c.json_object_size(co),
                        p.r.json_object_size(ro),
                        "{ctx}"
                    );
                }
                ObjOp::Traverse => {
                    assert_eq!(
                        object_entries(&p.c, co, 0),
                        object_entries(&p.r, ro, 0),
                        "{ctx}"
                    );
                }
                ObjOp::IterAt(k) => {
                    let z = raw_z(k);
                    let ci = p.c.json_object_iter_at(co, z.as_ptr() as *const c_char);
                    let ri = p.r.json_object_iter_at(ro, z.as_ptr() as *const c_char);
                    assert_eq!(ci.is_null(), ri.is_null(), "{ctx}: iter_at presence");
                    if !ci.is_null() {
                        let ckl = p.c.json_object_iter_key_len(ci);
                        let rkl = p.r.json_object_iter_key_len(ri);
                        assert_eq!(ckl, rkl, "{ctx}: iter key_len");
                        let ck = p.c.json_object_iter_key(ci);
                        let rk = p.r.json_object_iter_key(ri);
                        assert_eq!(
                            std::slice::from_raw_parts(ck as *const u8, ckl),
                            std::slice::from_raw_parts(rk as *const u8, rkl),
                            "{ctx}: iter key"
                        );
                        assert_same(
                            p,
                            p.c.json_object_iter_value(ci),
                            p.r.json_object_iter_value(ri),
                            &format!("{ctx}: iter value"),
                        );
                    }
                }
                ObjOp::IterSetAt(k, v) => {
                    let z = raw_z(k);
                    let ci = p.c.json_object_iter_at(co, z.as_ptr() as *const c_char);
                    let ri = p.r.json_object_iter_at(ro, z.as_ptr() as *const c_char);
                    assert_eq!(ci.is_null(), ri.is_null(), "{ctx}: iter_at presence");
                    let cv = v.build(&p.c);
                    let rv = v.build(&p.r);
                    let cr = p.c.json_object_iter_set_new(co, ci, cv);
                    let rr = p.r.json_object_iter_set_new(ro, ri, rv);
                    assert_eq!(cr, rr, "{ctx}");
                }
                ObjOp::KeyToIter(k) => {
                    let z = raw_z(k);
                    let ci = p.c.json_object_iter_at(co, z.as_ptr() as *const c_char);
                    let ri = p.r.json_object_iter_at(ro, z.as_ptr() as *const c_char);
                    assert_eq!(ci.is_null(), ri.is_null(), "{ctx}");
                    if ci.is_null() {
                        continue;
                    }
                    let ck = p.c.json_object_iter_key(ci);
                    let rk = p.r.json_object_iter_key(ri);
                    let ci2 = p.c.json_object_key_to_iter(ck);
                    let ri2 = p.r.json_object_key_to_iter(rk);
                    assert_eq!(ci2 == ci, ri2 == ri, "{ctx}: key_to_iter round-trip");
                    // And that iter_next from there agrees.
                    let cn = p.c.json_object_iter_next(co, ci2);
                    let rn = p.r.json_object_iter_next(ro, ri2);
                    assert_eq!(cn.is_null(), rn.is_null(), "{ctx}: iter_next after key_to_iter");
                    if !cn.is_null() {
                        let ckl = p.c.json_object_iter_key_len(cn);
                        let rkl = p.r.json_object_iter_key_len(rn);
                        assert_eq!(ckl, rkl);
                        assert_eq!(
                            std::slice::from_raw_parts(
                                p.c.json_object_iter_key(cn) as *const u8,
                                ckl
                            ),
                            std::slice::from_raw_parts(
                                p.r.json_object_iter_key(rn) as *const u8,
                                rkl
                            ),
                            "{ctx}: next key"
                        );
                    }
                }
            }
            // Invariants after every single op.
            assert_eq!(
                p.c.json_object_size(co),
                p.r.json_object_size(ro),
                "{ctx}: size after op"
            );
            assert_same(p, co, ro, &format!("{ctx}: whole object"));
        }
        p.c.json_decref(co);
        p.r.json_decref(ro);
    }
}

fn k(s: &str) -> Vec<u8> {
    s.as_bytes().to_vec()
}

/// D1, D6: low-level setter/getter across every rehash threshold, plus key
/// shapes (empty, embedded NUL, 4 KiB).
#[test]
fn d1_d6_object_rehash_thresholds_and_key_shapes() {
    for n in [0usize, 1, 7, 8, 9, 16, 17, 32, 33, 64, 65, 128, 129, 257] {
        let mut ops = Vec::new();
        for i in 0..n {
            ops.push(ObjOp::SetnNoCheck(
                format!("k{i:05}").into_bytes(),
                Node::Int(i as i64),
            ));
        }
        ops.push(ObjOp::Size);
        ops.push(ObjOp::Traverse);
        for i in 0..n {
            ops.push(ObjOp::Getn(format!("k{i:05}").into_bytes()));
        }
        ops.push(ObjOp::Getn(k("absent")));
        object_script(&ops);
    }
    // Key shapes.
    let shapes: Vec<Vec<u8>> = vec![
        vec![],
        k("a"),
        b"a\0b".to_vec(),
        b"a\0c".to_vec(),
        b"\0".to_vec(),
        vec![b'x'; 4096],
        vec![b'x'; 4095],
        "é€😀".as_bytes().to_vec(),
        vec![0xFF, 0xFE], // invalid UTF-8 (nocheck only)
    ];
    let mut ops = Vec::new();
    for (i, s) in shapes.iter().enumerate() {
        ops.push(ObjOp::SetnNoCheck(s.clone(), Node::Int(i as i64)));
    }
    ops.push(ObjOp::Traverse);
    for s in &shapes {
        ops.push(ObjOp::Getn(s.clone()));
        ops.push(ObjOp::IterAt(s.clone()));
    }
    ops.push(ObjOp::Traverse);
    object_script(&ops);
}

/// D2: the checked vs nocheck setter, and the NUL-terminated wrappers, on both
/// valid and invalid UTF-8 keys.
#[test]
fn d2_checked_vs_nocheck_setters() {
    let keys: Vec<Vec<u8>> = vec![
        k("ascii"),
        "é".as_bytes().to_vec(),
        "€".as_bytes().to_vec(),
        "😀".as_bytes().to_vec(),
        vec![0x80],
        vec![0xC0, 0x80],
        vec![0xFF],
        vec![0xED, 0xA0, 0x80], // surrogate
        vec![0xC3],             // truncated
        vec![],
    ];
    for key in &keys {
        for v in [Node::Int(1), Node::Null, Node::Arr(vec![])] {
            object_script(&[
                ObjOp::Setn(key.clone(), v.clone()),
                ObjOp::Traverse,
                ObjOp::Getn(key.clone()),
                ObjOp::SetnNoCheck(key.clone(), v.clone()),
                ObjOp::Traverse,
                ObjOp::Set(key.clone(), v.clone()),
                ObjOp::Traverse,
                ObjOp::SetNoCheck(key.clone(), v.clone()),
                ObjOp::Traverse,
                ObjOp::Get(key.clone()),
                ObjOp::Getn(key.clone()),
            ]);
        }
    }
}

/// D3: get with a key that is present / absent / a prefix of a present key /
/// present but queried with a different length.
#[test]
fn d3_object_get_key_variants() {
    let mut ops = vec![
        ObjOp::SetnNoCheck(k("abc"), Node::Int(1)),
        ObjOp::SetnNoCheck(k("ab"), Node::Int(2)),
        ObjOp::SetnNoCheck(k("a"), Node::Int(3)),
        ObjOp::SetnNoCheck(k(""), Node::Int(4)),
        ObjOp::SetnNoCheck(b"a\0b".to_vec(), Node::Int(5)),
        ObjOp::Traverse,
    ];
    for probe in ["", "a", "ab", "abc", "abcd", "b", "A"] {
        ops.push(ObjOp::Getn(k(probe)));
        ops.push(ObjOp::Get(k(probe)));
        ops.push(ObjOp::IterAt(k(probe)));
    }
    // Same bytes, wrong length.
    for len in 0..=3usize {
        ops.push(ObjOp::Getn(k("abc")[..len].to_vec()));
    }
    ops.push(ObjOp::Getn(b"a\0b".to_vec()));
    ops.push(ObjOp::Get(b"a\0b".to_vec())); // strlen stops at the NUL
    object_script(&ops);
}

/// D4: delete then re-insert; ordered-list position after each; deletion in
/// insertion, reverse and randomized order.
#[test]
fn d4_object_delete_and_reinsert_orders() {
    let n = 40usize;
    // forward
    let mut ops: Vec<ObjOp> = (0..n)
        .map(|i| ObjOp::SetnNoCheck(format!("k{i:03}").into_bytes(), Node::Int(i as i64)))
        .collect();
    for i in 0..n {
        ops.push(ObjOp::Deln(format!("k{i:03}").into_bytes()));
        ops.push(ObjOp::Traverse);
    }
    ops.push(ObjOp::Deln(k("k000"))); // now absent
    object_script(&ops);

    // reverse
    let mut ops: Vec<ObjOp> = (0..n)
        .map(|i| ObjOp::SetnNoCheck(format!("k{i:03}").into_bytes(), Node::Int(i as i64)))
        .collect();
    for i in (0..n).rev() {
        ops.push(ObjOp::Deln(format!("k{i:03}").into_bytes()));
        ops.push(ObjOp::Traverse);
    }
    object_script(&ops);

    // delete-then-reinsert: the key must move to the END of the ordered list
    let mut ops: Vec<ObjOp> = (0..8)
        .map(|i| ObjOp::SetnNoCheck(format!("k{i}").into_bytes(), Node::Int(i as i64)))
        .collect();
    for i in 0..8 {
        ops.push(ObjOp::Deln(format!("k{i}").into_bytes()));
        ops.push(ObjOp::Traverse);
        ops.push(ObjOp::SetnNoCheck(
            format!("k{i}").into_bytes(),
            Node::Int(100 + i as i64),
        ));
        ops.push(ObjOp::Traverse);
    }
    object_script(&ops);

    // Randomized interleaving of every object op.
    let mut rng = Rng::new(0xD4_D4_D4);
    for _ in 0..250 {
        let mut ops = Vec::new();
        for _ in 0..rng.below(80) {
            let key = format!("k{}", rng.below(20)).into_bytes();
            ops.push(match rng.below(11) {
                0 | 1 | 2 => ObjOp::SetnNoCheck(key, rng.node(2)),
                3 => ObjOp::Setn(key, rng.node(1)),
                4 => ObjOp::Set(key, rng.node(1)),
                5 => ObjOp::Deln(key),
                6 => ObjOp::Del(key),
                7 => ObjOp::Getn(key),
                8 => ObjOp::IterAt(key),
                9 => ObjOp::KeyToIter(key),
                _ => ObjOp::Traverse,
            });
        }
        ops.push(ObjOp::Traverse);
        ops.push(ObjOp::Size);
        object_script(&ops);
    }
}

/// D5: clear then refill — the bucket array is not shrunk, so the later rehash
/// thresholds shift.
#[test]
fn d5_object_clear_retains_capacity() {
    let mut ops: Vec<ObjOp> = (0..100)
        .map(|i| ObjOp::SetnNoCheck(format!("k{i:03}").into_bytes(), Node::Int(i as i64)))
        .collect();
    ops.push(ObjOp::Clear);
    ops.push(ObjOp::Traverse);
    ops.push(ObjOp::Size);
    for i in 0..20 {
        ops.push(ObjOp::SetnNoCheck(
            format!("n{i}").into_bytes(),
            Node::Int(i as i64),
        ));
    }
    ops.push(ObjOp::Traverse);
    ops.push(ObjOp::Clear);
    ops.push(ObjOp::Clear);
    ops.push(ObjOp::Traverse);
    object_script(&ops);
}

/// D7, D8: full iterator traversal at every size, and `iter_set_new` replacing
/// values without moving them.
#[test]
fn d7_d8_object_iteration() {
    for n in [0usize, 1, 2, 8, 9, 17, 33, 65] {
        let mut ops: Vec<ObjOp> = (0..n)
            .map(|i| ObjOp::SetnNoCheck(format!("k{i:03}").into_bytes(), Node::Int(i as i64)))
            .collect();
        ops.push(ObjOp::Traverse);
        for i in 0..n {
            ops.push(ObjOp::IterAt(format!("k{i:03}").into_bytes()));
            ops.push(ObjOp::KeyToIter(format!("k{i:03}").into_bytes()));
        }
        // D8: replace every value in place.
        for i in 0..n {
            ops.push(ObjOp::IterSetAt(
                format!("k{i:03}").into_bytes(),
                Node::Str(format!("v{i}").into_bytes()),
            ));
            ops.push(ObjOp::Traverse);
        }
        object_script(&ops);
    }
}

/// D9..D12: the four update variants over disjoint / overlapping / nested key
/// sets, at sizes that cross rehash thresholds.
#[test]
fn d9_d12_object_updates() {
    unsafe {
        let p = pair();
        #[derive(Debug, Clone, Copy)]
        enum Kind {
            Plain,
            Existing,
            Missing,
            Recursive,
        }
        let kinds = [Kind::Plain, Kind::Existing, Kind::Missing, Kind::Recursive];

        let mut cases: Vec<(Node, Node)> = vec![
            // disjoint
            (
                Node::Obj(vec![(k("a"), Node::Int(1))]),
                Node::Obj(vec![(k("b"), Node::Int(2))]),
            ),
            // fully overlapping
            (
                Node::Obj(vec![(k("a"), Node::Int(1))]),
                Node::Obj(vec![(k("a"), Node::Int(9))]),
            ),
            // partially overlapping
            (
                Node::Obj(vec![(k("a"), Node::Int(1)), (k("b"), Node::Int(2))]),
                Node::Obj(vec![(k("b"), Node::Int(9)), (k("c"), Node::Int(3))]),
            ),
            // empty other
            (Node::Obj(vec![(k("a"), Node::Int(1))]), Node::Obj(vec![])),
            // empty target
            (Node::Obj(vec![]), Node::Obj(vec![(k("a"), Node::Int(1))])),
            (Node::Obj(vec![]), Node::Obj(vec![])),
            // D12: nested objects — recursive merges, the others replace
            (
                Node::Obj(vec![(
                    k("n"),
                    Node::Obj(vec![(k("x"), Node::Int(1)), (k("y"), Node::Int(2))]),
                )]),
                Node::Obj(vec![(
                    k("n"),
                    Node::Obj(vec![(k("y"), Node::Int(9)), (k("z"), Node::Int(3))]),
                )]),
            ),
            // nested but the target value is NOT an object -> replaced wholesale
            (
                Node::Obj(vec![(k("n"), Node::Int(0))]),
                Node::Obj(vec![(k("n"), Node::Obj(vec![(k("x"), Node::Int(1))]))]),
            ),
            (
                Node::Obj(vec![(k("n"), Node::Obj(vec![(k("x"), Node::Int(1))]))]),
                Node::Obj(vec![(k("n"), Node::Int(0))]),
            ),
            // three levels deep
            (
                Node::Obj(vec![(
                    k("a"),
                    Node::Obj(vec![(k("b"), Node::Obj(vec![(k("c"), Node::Int(1))]))]),
                )]),
                Node::Obj(vec![(
                    k("a"),
                    Node::Obj(vec![(k("b"), Node::Obj(vec![(k("d"), Node::Int(2))]))]),
                )]),
            ),
        ];
        // Sizes crossing rehash thresholds.
        for n in [8usize, 9, 16, 17, 33] {
            cases.push((
                Node::Obj(
                    (0..n)
                        .map(|i| (format!("k{i:03}").into_bytes(), Node::Int(i as i64)))
                        .collect(),
                ),
                Node::Obj(
                    (0..n)
                        .map(|i| (format!("k{:03}", i + n / 2).into_bytes(), Node::Int(-(i as i64))))
                        .collect(),
                ),
            ));
        }
        // Randomized.
        let mut rng = Rng::new(0xD9_D9);
        for _ in 0..300 {
            let mk = |rng: &mut Rng| {
                let n = rng.below(12);
                Node::Obj(
                    (0..n)
                        .map(|_| (format!("k{}", rng.below(10)).into_bytes(), rng.node(3)))
                        .collect(),
                )
            };
            let a = mk(&mut rng);
            let b = mk(&mut rng);
            cases.push((a, b));
        }

        for (a, b) in &cases {
            for kind in kinds {
                let ca = a.build(&p.c);
                let ra = a.build(&p.r);
                let cb = b.build(&p.c);
                let rb = b.build(&p.r);
                let (cr, rr) = match kind {
                    Kind::Plain => (
                        p.c.json_object_update(ca, cb),
                        p.r.json_object_update(ra, rb),
                    ),
                    Kind::Existing => (
                        p.c.json_object_update_existing(ca, cb),
                        p.r.json_object_update_existing(ra, rb),
                    ),
                    Kind::Missing => (
                        p.c.json_object_update_missing(ca, cb),
                        p.r.json_object_update_missing(ra, rb),
                    ),
                    Kind::Recursive => (
                        p.c.json_object_update_recursive(ca, cb),
                        p.r.json_object_update_recursive(ra, rb),
                    ),
                };
                assert_eq!(cr, rr, "{kind:?} return for {a:?} <- {b:?}");
                assert_same(p, ca, ra, &format!("{kind:?} target after {a:?} <- {b:?}"));
                assert_same(p, cb, rb, &format!("{kind:?} source after {a:?} <- {b:?}"));
                p.c.json_decref(ca);
                p.r.json_decref(ra);
                p.c.json_decref(cb);
                p.r.json_decref(rb);
            }
        }
    }
}

/// D13: `do_object_update_recursive` called directly with a caller-supplied
/// `parents` hashtable — the exported internal entry point.
#[test]
fn d13_do_object_update_recursive_direct() {
    unsafe {
        let p = pair();
        type Fn3 = unsafe extern "C" fn(json_ptr, json_ptr, *mut c_void) -> std::os::raw::c_int;
        let cf: libloading::Symbol<Fn3> = p.c.lib.get(b"do_object_update_recursive\0").unwrap();
        let rf: libloading::Symbol<Fn3> = p.r.lib.get(b"do_object_update_recursive\0").unwrap();

        let cases: Vec<(Node, Node)> = vec![
            (
                Node::Obj(vec![(k("a"), Node::Int(1))]),
                Node::Obj(vec![(k("b"), Node::Int(2))]),
            ),
            (
                Node::Obj(vec![(k("n"), Node::Obj(vec![(k("x"), Node::Int(1))]))]),
                Node::Obj(vec![(k("n"), Node::Obj(vec![(k("y"), Node::Int(2))]))]),
            ),
            (Node::Obj(vec![]), Node::Obj(vec![])),
            (Node::Arr(vec![]), Node::Obj(vec![])), // non-object target -> -1
            (Node::Obj(vec![]), Node::Arr(vec![])), // non-object source -> -1
        ];
        for (a, b) in &cases {
            let mut cslab = [0u8; 256];
            let mut rslab = [0u8; 256];
            let ch = cslab.as_mut_ptr() as *mut c_void;
            let rh = rslab.as_mut_ptr() as *mut c_void;
            assert_eq!(p.c.hashtable_init(ch), p.r.hashtable_init(rh));
            let ca = a.build(&p.c);
            let ra = a.build(&p.r);
            let cb = b.build(&p.c);
            let rb = b.build(&p.r);
            let cr = cf(ca, cb, ch);
            let rr = rf(ra, rb, rh);
            assert_eq!(cr, rr, "do_object_update_recursive({a:?}, {b:?})");
            assert_same(p, ca, ra, &format!("target after {a:?} <- {b:?}"));
            p.c.hashtable_close(ch);
            p.r.hashtable_close(rh);
            p.c.json_decref(ca);
            p.r.json_decref(ra);
            p.c.json_decref(cb);
            p.r.json_decref(rb);
        }
    }
}

// ===========================================================================
// E — arrays
// ===========================================================================

#[derive(Debug, Clone)]
enum ArrOp {
    Append(Node),
    Insert(usize, Node),
    Set(usize, Node),
    Get(usize),
    Remove(usize),
    Clear,
    Size,
    Extend(Node),
    /// Read every index 0..size+2, so out-of-range reads are compared too.
    ScanAll,
}

fn array_script(ops: &[ArrOp]) {
    unsafe {
        let p = pair();
        let ca = p.c.json_array();
        let ra = p.r.json_array();
        assert!(!ca.is_null() && !ra.is_null());
        for (n, op) in ops.iter().enumerate() {
            let ctx = format!("op {n}: {op:?}");
            match op {
                ArrOp::Append(v) => {
                    let cr = p.c.json_array_append_new(ca, v.build(&p.c));
                    let rr = p.r.json_array_append_new(ra, v.build(&p.r));
                    assert_eq!(cr, rr, "{ctx}");
                }
                ArrOp::Insert(i, v) => {
                    let cr = p.c.json_array_insert_new(ca, *i, v.build(&p.c));
                    let rr = p.r.json_array_insert_new(ra, *i, v.build(&p.r));
                    assert_eq!(cr, rr, "{ctx}");
                }
                ArrOp::Set(i, v) => {
                    let cr = p.c.json_array_set_new(ca, *i, v.build(&p.c));
                    let rr = p.r.json_array_set_new(ra, *i, v.build(&p.r));
                    assert_eq!(cr, rr, "{ctx}");
                }
                ArrOp::Get(i) => {
                    assert_same(p, p.c.json_array_get(ca, *i), p.r.json_array_get(ra, *i), &ctx);
                }
                ArrOp::Remove(i) => {
                    let cr = p.c.json_array_remove(ca, *i);
                    let rr = p.r.json_array_remove(ra, *i);
                    assert_eq!(cr, rr, "{ctx}");
                }
                ArrOp::Clear => {
                    assert_eq!(p.c.json_array_clear(ca), p.r.json_array_clear(ra), "{ctx}");
                }
                ArrOp::Size => {
                    assert_eq!(p.c.json_array_size(ca), p.r.json_array_size(ra), "{ctx}");
                }
                ArrOp::Extend(other) => {
                    let co = other.build(&p.c);
                    let ro = other.build(&p.r);
                    let cr = p.c.json_array_extend(ca, co);
                    let rr = p.r.json_array_extend(ra, ro);
                    assert_eq!(cr, rr, "{ctx}");
                    assert_same(p, co, ro, &format!("{ctx}: source unchanged"));
                    p.c.json_decref(co);
                    p.r.json_decref(ro);
                }
                ArrOp::ScanAll => {
                    let n = p.c.json_array_size(ca);
                    assert_eq!(n, p.r.json_array_size(ra), "{ctx}");
                    for i in 0..n + 3 {
                        assert_same(
                            p,
                            p.c.json_array_get(ca, i),
                            p.r.json_array_get(ra, i),
                            &format!("{ctx}: index {i}"),
                        );
                    }
                    for i in [usize::MAX, usize::MAX - 1, n, n + 1] {
                        assert_eq!(
                            p.c.json_array_get(ca, i).is_null(),
                            p.r.json_array_get(ra, i).is_null(),
                            "{ctx}: extreme index {i}"
                        );
                    }
                }
            }
            assert_eq!(
                p.c.json_array_size(ca),
                p.r.json_array_size(ra),
                "{ctx}: size after op"
            );
            assert_same(p, ca, ra, &format!("{ctx}: whole array"));
        }
        p.c.json_decref(ca);
        p.r.json_decref(ra);
    }
}

/// E1, E7: capacity growth 8 -> x2 at every boundary; scan every index.
#[test]
fn e1_e7_array_growth() {
    for n in [0usize, 1, 7, 8, 9, 15, 16, 17, 33, 100, 1000] {
        let mut ops: Vec<ArrOp> = (0..n).map(|i| ArrOp::Append(Node::Int(i as i64))).collect();
        ops.push(ArrOp::Size);
        ops.push(ArrOp::ScanAll);
        array_script(&ops);
    }
}

/// E2: insert at the front, the middle, exactly `entries` (no memmove) and one
/// past (rejected), across a growth boundary.
#[test]
fn e2_array_insert_positions() {
    for n in [0usize, 1, 7, 8, 9, 16, 17] {
        let base: Vec<ArrOp> = (0..n).map(|i| ArrOp::Append(Node::Int(i as i64))).collect();
        for idx in 0..n + 3 {
            let mut ops = base.clone();
            ops.push(ArrOp::Insert(idx, Node::Str(b"ins".to_vec())));
            ops.push(ArrOp::ScanAll);
            array_script(&ops);
        }
        let mut ops = base.clone();
        ops.push(ArrOp::Insert(usize::MAX, Node::Int(0)));
        ops.push(ArrOp::ScanAll);
        array_script(&ops);
    }
}

/// E3: set at every valid index (and past the end).
#[test]
fn e3_array_set_positions() {
    for n in [0usize, 1, 8, 9, 17] {
        let base: Vec<ArrOp> = (0..n).map(|i| ArrOp::Append(Node::Int(i as i64))).collect();
        for idx in 0..n + 3 {
            let mut ops = base.clone();
            ops.push(ArrOp::Set(idx, Node::Str(b"set".to_vec())));
            ops.push(ArrOp::ScanAll);
            array_script(&ops);
        }
        let mut ops = base;
        ops.push(ArrOp::Set(usize::MAX, Node::Int(0)));
        ops.push(ArrOp::ScanAll);
        array_script(&ops);
    }
}

/// E4: remove index 0 / middle / last (no memmove); remove-all in forward,
/// reverse and randomized order.
#[test]
fn e4_array_remove_orders() {
    for n in [1usize, 2, 8, 9, 17, 40] {
        let base: Vec<ArrOp> = (0..n).map(|i| ArrOp::Append(Node::Int(i as i64))).collect();
        for idx in [0usize, n / 2, n - 1, n, n + 1, usize::MAX] {
            let mut ops = base.clone();
            ops.push(ArrOp::Remove(idx));
            ops.push(ArrOp::ScanAll);
            array_script(&ops);
        }
        // remove-all forward
        let mut ops = base.clone();
        for _ in 0..n + 1 {
            ops.push(ArrOp::Remove(0));
            ops.push(ArrOp::ScanAll);
        }
        array_script(&ops);
        // remove-all from the back
        let mut ops = base.clone();
        for i in (0..n).rev() {
            ops.push(ArrOp::Remove(i));
            ops.push(ArrOp::ScanAll);
        }
        array_script(&ops);
    }
    // randomized
    let mut rng = Rng::new(0xE4_E4);
    for _ in 0..300 {
        let mut ops = Vec::new();
        for _ in 0..rng.below(60) {
            let i = rng.below(20);
            ops.push(match rng.below(8) {
                0 | 1 | 2 => ArrOp::Append(rng.node(2)),
                3 => ArrOp::Insert(i, rng.node(2)),
                4 => ArrOp::Set(i, rng.node(2)),
                5 => ArrOp::Remove(i),
                6 => ArrOp::Get(i),
                _ => ArrOp::ScanAll,
            });
        }
        ops.push(ArrOp::ScanAll);
        array_script(&ops);
    }
}

/// E5: clear then refill — the capacity is retained.
#[test]
fn e5_array_clear_retains_capacity() {
    let mut ops: Vec<ArrOp> = (0..100).map(|i| ArrOp::Append(Node::Int(i as i64))).collect();
    ops.push(ArrOp::Clear);
    ops.push(ArrOp::ScanAll);
    for i in 0..10 {
        ops.push(ArrOp::Append(Node::Int(1000 + i)));
    }
    ops.push(ArrOp::ScanAll);
    ops.push(ArrOp::Clear);
    ops.push(ArrOp::Clear);
    ops.push(ArrOp::ScanAll);
    array_script(&ops);
}

/// E6: extend with an `other` that is empty / smaller / larger than the target —
/// a large `other` jumps capacity straight to `size + amount`.
#[test]
fn e6_array_extend() {
    for n in [0usize, 1, 8, 9] {
        for m in [0usize, 1, 5, 8, 9, 50, 200] {
            let mut ops: Vec<ArrOp> =
                (0..n).map(|i| ArrOp::Append(Node::Int(i as i64))).collect();
            ops.push(ArrOp::Extend(Node::Arr(
                (0..m).map(|i| Node::Int(1000 + i as i64)).collect(),
            )));
            ops.push(ArrOp::ScanAll);
            // Extend a second time to exercise growth from the new capacity.
            ops.push(ArrOp::Extend(Node::Arr(
                (0..m).map(|i| Node::Int(2000 + i as i64)).collect(),
            )));
            ops.push(ArrOp::ScanAll);
            array_script(&ops);
        }
    }
    // Non-array `other` -> -1
    array_script(&[
        ArrOp::Append(Node::Int(1)),
        ArrOp::Extend(Node::Obj(vec![])),
        ArrOp::Extend(Node::Int(5)),
        ArrOp::Extend(Node::Null),
        ArrOp::ScanAll,
    ]);
}

// ===========================================================================
// F — scalars, equality, copying
// ===========================================================================

/// F1..F4: string constructors, setters and length-aware accessors.
#[test]
fn f1_f4_strings() {
    unsafe {
        let p = pair();
        let payloads: Vec<Vec<u8>> = vec![
            vec![],
            b"a".to_vec(),
            vec![b'x'; 15],
            vec![b'x'; 16],
            vec![b'x'; 17],
            vec![b'x'; 1000],
            "é".as_bytes().to_vec(),
            "€".as_bytes().to_vec(),
            "😀".as_bytes().to_vec(),
            "aé€😀z".as_bytes().to_vec(),
            b"a\0b".to_vec(),
            vec![0u8],
            vec![0x80],
            vec![0xC0, 0x80],
            vec![0xFF, 0xFE],
            vec![0xED, 0xA0, 0x80],
            vec![0xC3],
        ];
        for pl in &payloads {
            for &nocheck in &[false, true] {
                for &use_len in &[false, true] {
                    let ptr = pl.as_ptr() as *const c_char;
                    let z = raw_z(pl);
                    let zp = z.as_ptr() as *const c_char;
                    let (cj, rj) = match (nocheck, use_len) {
                        (false, true) => (
                            p.c.json_stringn(ptr, pl.len()),
                            p.r.json_stringn(ptr, pl.len()),
                        ),
                        (true, true) => (
                            p.c.json_stringn_nocheck(ptr, pl.len()),
                            p.r.json_stringn_nocheck(ptr, pl.len()),
                        ),
                        (false, false) => (p.c.json_string(zp), p.r.json_string(zp)),
                        (true, false) => {
                            (p.c.json_string_nocheck(zp), p.r.json_string_nocheck(zp))
                        }
                    };
                    let ctx =
                        format!("string({pl:02x?}, nocheck={nocheck}, use_len={use_len})");
                    assert_eq!(cj.is_null(), rj.is_null(), "{ctx}: NULL-ness");
                    if cj.is_null() {
                        continue;
                    }
                    let cl = p.c.json_string_length(cj);
                    let rl = p.r.json_string_length(rj);
                    assert_eq!(cl, rl, "{ctx}: json_string_length");
                    let cv = p.c.json_string_value(cj);
                    let rv = p.r.json_string_value(rj);
                    assert_eq!(
                        std::slice::from_raw_parts(cv as *const u8, cl),
                        std::slice::from_raw_parts(rv as *const u8, rl),
                        "{ctx}: json_string_value bytes"
                    );
                    // F3: every setter variant, shrinking and growing.
                    for repl in &payloads {
                        let rp = repl.as_ptr() as *const c_char;
                        let rz = raw_z(repl);
                        let rzp = rz.as_ptr() as *const c_char;
                        for variant in 0..4 {
                            let (cr, rr) = match variant {
                                0 => (
                                    p.c.json_string_setn(cj, rp, repl.len()),
                                    p.r.json_string_setn(rj, rp, repl.len()),
                                ),
                                1 => (
                                    p.c.json_string_setn_nocheck(cj, rp, repl.len()),
                                    p.r.json_string_setn_nocheck(rj, rp, repl.len()),
                                ),
                                2 => (
                                    p.c.json_string_set(cj, rzp),
                                    p.r.json_string_set(rj, rzp),
                                ),
                                _ => (
                                    p.c.json_string_set_nocheck(cj, rzp),
                                    p.r.json_string_set_nocheck(rj, rzp),
                                ),
                            };
                            assert_eq!(cr, rr, "{ctx}: set variant {variant} -> {repl:02x?}");
                            let cl = p.c.json_string_length(cj);
                            let rl = p.r.json_string_length(rj);
                            assert_eq!(cl, rl, "{ctx}: length after set {variant}");
                            assert_eq!(
                                std::slice::from_raw_parts(
                                    p.c.json_string_value(cj) as *const u8,
                                    cl
                                ),
                                std::slice::from_raw_parts(
                                    p.r.json_string_value(rj) as *const u8,
                                    rl
                                ),
                                "{ctx}: bytes after set {variant}"
                            );
                        }
                    }
                    p.c.json_decref(cj);
                    p.r.json_decref(rj);
                }
            }
        }
    }
}

/// F5..F7: integers, reals, and `json_number_value` on every type.
#[test]
fn f5_f7_numbers() {
    unsafe {
        let p = pair();
        let mut rng = Rng::new(0xF5_F5);
        let mut ints: Vec<i64> = vec![0, 1, -1, i64::MAX, i64::MIN, i32::MAX as i64, i32::MIN as i64];
        for _ in 0..2000 {
            ints.push(rng.next_u64() as i64);
        }
        for v in ints {
            let cj = p.c.json_integer(v);
            let rj = p.r.json_integer(v);
            assert_eq!(cj.is_null(), rj.is_null());
            assert_eq!(
                p.c.json_integer_value(cj),
                p.r.json_integer_value(rj),
                "json_integer_value({v})"
            );
            assert_eq!(
                p.c.json_number_value(cj).to_bits(),
                p.r.json_number_value(rj).to_bits(),
                "json_number_value(int {v})"
            );
            // real_value on an integer must be 0.0
            assert_eq!(
                p.c.json_real_value(cj).to_bits(),
                p.r.json_real_value(rj).to_bits(),
                "json_real_value(int {v})"
            );
            for w in [0i64, -1, i64::MAX, i64::MIN, 12345] {
                assert_eq!(
                    p.c.json_integer_set(cj, w),
                    p.r.json_integer_set(rj, w),
                    "json_integer_set({w})"
                );
                assert_eq!(p.c.json_integer_value(cj), p.r.json_integer_value(rj));
            }
            p.c.json_decref(cj);
            p.r.json_decref(rj);
        }

        let mut reals: Vec<f64> = vec![
            0.0, -0.0, 1.0, -1.0, 0.5, f64::MAX, f64::MIN, f64::MIN_POSITIVE, 5e-324,
            f64::EPSILON, 1e300, 1e-300, 1.0 / 3.0,
        ];
        for _ in 0..2000 {
            reals.push(rng.finite_f64());
        }
        // Non-finite values are rejected by json_real / json_real_set.
        let bad = [f64::NAN, f64::INFINITY, f64::NEG_INFINITY, -f64::NAN];
        for v in reals.iter().copied().chain(bad) {
            let cj = p.c.json_real(v);
            let rj = p.r.json_real(v);
            assert_eq!(cj.is_null(), rj.is_null(), "json_real({v:?}) NULL-ness");
            if cj.is_null() {
                continue;
            }
            assert_eq!(
                p.c.json_real_value(cj).to_bits(),
                p.r.json_real_value(rj).to_bits(),
                "json_real_value({v:?})"
            );
            assert_eq!(
                p.c.json_number_value(cj).to_bits(),
                p.r.json_number_value(rj).to_bits(),
                "json_number_value(real {v:?})"
            );
            assert_eq!(
                p.c.json_integer_value(cj),
                p.r.json_integer_value(rj),
                "json_integer_value(real {v:?})"
            );
            for w in reals.iter().copied().take(6).chain(bad) {
                assert_eq!(
                    p.c.json_real_set(cj, w),
                    p.r.json_real_set(rj, w),
                    "json_real_set({w:?})"
                );
                assert_eq!(
                    p.c.json_real_value(cj).to_bits(),
                    p.r.json_real_value(rj).to_bits()
                );
            }
            p.c.json_decref(cj);
            p.r.json_decref(rj);
        }

        // F7: json_number_value / integer_value / real_value on every type.
        for node in [
            Node::Null,
            Node::True,
            Node::False,
            Node::Str(b"1".to_vec()),
            Node::Arr(vec![]),
            Node::Obj(vec![]),
        ] {
            with_both(&node, |p2, cj, rj| {
                assert_eq!(
                    p2.c.json_number_value(cj).to_bits(),
                    p2.r.json_number_value(rj).to_bits(),
                    "json_number_value({node:?})"
                );
                assert_eq!(
                    p2.c.json_integer_value(cj),
                    p2.r.json_integer_value(rj),
                    "json_integer_value({node:?})"
                );
                assert_eq!(
                    p2.c.json_real_value(cj).to_bits(),
                    p2.r.json_real_value(rj).to_bits(),
                    "json_real_value({node:?})"
                );
                assert_eq!(
                    p2.c.json_string_length(cj),
                    p2.r.json_string_length(rj),
                    "json_string_length({node:?})"
                );
                assert_eq!(
                    p2.c.json_string_value(cj).is_null(),
                    p2.r.json_string_value(rj).is_null(),
                    "json_string_value({node:?})"
                );
            });
        }
    }
}

/// F8: the immortal singletons — refcount `(size_t)-1`, never freed.
#[test]
fn f8_singletons() {
    unsafe {
        let p = pair();
        for which in 0..3 {
            let (cj, rj) = match which {
                0 => (p.c.json_true(), p.r.json_true()),
                1 => (p.c.json_false(), p.r.json_false()),
                _ => (p.c.json_null(), p.r.json_null()),
            };
            assert_eq!(typeof_json(cj), typeof_json(rj), "singleton {which} type");
            assert_eq!(
                refcount_json(cj),
                refcount_json(rj),
                "singleton {which} refcount"
            );
            assert_eq!(refcount_json(cj), usize::MAX, "singleton {which} is immortal");
            // Repeated calls return the same pointer.
            let (cj2, rj2) = match which {
                0 => (p.c.json_true(), p.r.json_true()),
                1 => (p.c.json_false(), p.r.json_false()),
                _ => (p.c.json_null(), p.r.json_null()),
            };
            assert_eq!(cj == cj2, rj == rj2, "singleton {which} identity");
            // json_delete must not free them.
            p.c.json_delete(cj);
            p.r.json_delete(rj);
            assert_eq!(typeof_json(cj), typeof_json(rj), "singleton {which} survived");
            assert_eq!(refcount_json(cj), usize::MAX);
            assert_eq!(refcount_json(rj), usize::MAX);
            // json_copy returns the same pointer for these.
            let cc = p.c.json_copy(cj);
            let rc = p.r.json_copy(rj);
            assert_eq!(cc == cj, rc == rj, "singleton {which} json_copy identity");
        }
    }
}

/// F9: json_equal across every type pair, nested containers, order-independent
/// object equality, and strings differing only past an embedded NUL.
#[test]
fn f9_json_equal() {
    unsafe {
        let p = pair();
        let mut nodes: Vec<Node> = vec![
            Node::Null,
            Node::True,
            Node::False,
            Node::Int(0),
            Node::Int(1),
            Node::Int(-1),
            Node::Int(i64::MAX),
            Node::Real(0.0),
            Node::Real(-0.0),
            Node::Real(1.0),
            Node::Real(1.5),
            Node::Str(vec![]),
            Node::Str(b"a".to_vec()),
            Node::Str(b"b".to_vec()),
            Node::StrNoCheck(b"a\0b".to_vec()),
            Node::StrNoCheck(b"a\0c".to_vec()),
            Node::StrNoCheck(b"a".to_vec()),
            Node::Arr(vec![]),
            Node::Arr(vec![Node::Int(1)]),
            Node::Arr(vec![Node::Int(1), Node::Int(2)]),
            Node::Arr(vec![Node::Int(2), Node::Int(1)]),
            Node::Obj(vec![]),
            Node::Obj(vec![(k("a"), Node::Int(1))]),
            // same content, different insertion order -> must still be equal
            Node::Obj(vec![(k("a"), Node::Int(1)), (k("b"), Node::Int(2))]),
            Node::Obj(vec![(k("b"), Node::Int(2)), (k("a"), Node::Int(1))]),
            Node::Obj(vec![(k("a"), Node::Int(1)), (k("b"), Node::Int(3))]),
            Node::Obj(vec![(k("a"), Node::Obj(vec![(k("x"), Node::Int(1))]))]),
        ];
        let mut rng = Rng::new(0xF9_F9);
        for _ in 0..60 {
            nodes.push(rng.node(3));
        }

        for a in &nodes {
            for b in &nodes {
                let ca = a.build(&p.c);
                let ra = a.build(&p.r);
                let cb = b.build(&p.c);
                let rb = b.build(&p.r);
                if ca.is_null() || cb.is_null() {
                    p.c.json_decref(ca);
                    p.r.json_decref(ra);
                    p.c.json_decref(cb);
                    p.r.json_decref(rb);
                    continue;
                }
                assert_eq!(
                    p.c.json_equal(ca, cb),
                    p.r.json_equal(ra, rb),
                    "json_equal({a:?}, {b:?})"
                );
                // NULL operands
                assert_eq!(
                    p.c.json_equal(ca, std::ptr::null_mut()),
                    p.r.json_equal(ra, std::ptr::null_mut()),
                    "json_equal({a:?}, NULL)"
                );
                assert_eq!(
                    p.c.json_equal(std::ptr::null_mut(), cb),
                    p.r.json_equal(std::ptr::null_mut(), rb),
                    "json_equal(NULL, {b:?})"
                );
                p.c.json_decref(ca);
                p.r.json_decref(ra);
                p.c.json_decref(cb);
                p.r.json_decref(rb);
            }
        }
        assert_eq!(
            p.c.json_equal(std::ptr::null_mut(), std::ptr::null_mut()),
            p.r.json_equal(std::ptr::null_mut(), std::ptr::null_mut())
        );
    }
}

/// F10, F11: json_copy (shallow) and json_deep_copy, plus `do_deep_copy` called
/// directly with a caller-supplied `parents` hashtable.
#[test]
fn f10_f11_copying() {
    unsafe {
        let p = pair();
        let mut nodes: Vec<Node> = vec![
            Node::Null,
            Node::True,
            Node::False,
            Node::Int(7),
            Node::Real(1.25),
            Node::Str(b"s".to_vec()),
            Node::StrNoCheck(b"a\0b".to_vec()),
            Node::Arr(vec![]),
            Node::Obj(vec![]),
            Node::Arr(vec![Node::Int(1), Node::Arr(vec![Node::Int(2)])]),
            Node::Obj(vec![
                (k("a"), Node::Obj(vec![(k("b"), Node::Int(1))])),
                (k("c"), Node::Arr(vec![Node::Null, Node::True])),
            ]),
        ];
        let mut rng = Rng::new(0xF10_F11);
        for _ in 0..300 {
            nodes.push(rng.node(4));
        }
        for node in &nodes {
            with_both(node, |p2, cj, rj| {
                let cc = p2.c.json_copy(cj);
                let rc = p2.r.json_copy(rj);
                assert_same(p2, cc, rc, &format!("json_copy({node:?})"));
                if !cc.is_null() {
                    assert_eq!(
                        p2.c.json_equal(cj, cc),
                        p2.r.json_equal(rj, rc),
                        "json_equal after json_copy({node:?})"
                    );
                    p2.c.json_decref(cc);
                    p2.r.json_decref(rc);
                }
                let cd = p2.c.json_deep_copy(cj);
                let rd = p2.r.json_deep_copy(rj);
                assert_same(p2, cd, rd, &format!("json_deep_copy({node:?})"));
                if !cd.is_null() {
                    assert_eq!(
                        p2.c.json_equal(cj, cd),
                        p2.r.json_equal(rj, rd),
                        "json_equal after deep_copy({node:?})"
                    );
                    p2.c.json_decref(cd);
                    p2.r.json_decref(rd);
                }
            });
        }
        // NULL input
        assert_eq!(
            p.c.json_copy(std::ptr::null_mut()).is_null(),
            p.r.json_copy(std::ptr::null_mut()).is_null()
        );
        assert_eq!(
            p.c.json_deep_copy(std::ptr::null_mut()).is_null(),
            p.r.json_deep_copy(std::ptr::null_mut()).is_null()
        );

        // F11: do_deep_copy called directly.
        type Fn2 = unsafe extern "C" fn(json_ptr, *mut c_void) -> json_ptr;
        let cf: libloading::Symbol<Fn2> = p.c.lib.get(b"do_deep_copy\0").unwrap();
        let rf: libloading::Symbol<Fn2> = p.r.lib.get(b"do_deep_copy\0").unwrap();
        for node in nodes.iter().take(60) {
            let mut cslab = [0u8; 256];
            let mut rslab = [0u8; 256];
            let ch = cslab.as_mut_ptr() as *mut c_void;
            let rh = rslab.as_mut_ptr() as *mut c_void;
            assert_eq!(p.c.hashtable_init(ch), p.r.hashtable_init(rh));
            let cj = node.build(&p.c);
            let rj = node.build(&p.r);
            let cd = cf(cj, ch);
            let rd = rf(rj, rh);
            assert_same(p, cd, rd, &format!("do_deep_copy({node:?})"));
            // NULL input
            assert_eq!(
                cf(std::ptr::null_mut(), ch).is_null(),
                rf(std::ptr::null_mut(), rh).is_null(),
                "do_deep_copy(NULL)"
            );
            p.c.json_decref(cd);
            p.r.json_decref(rd);
            p.c.json_decref(cj);
            p.r.json_decref(rj);
            p.c.hashtable_close(ch);
            p.r.hashtable_close(rh);
        }
    }
}

/// F12: json_sprintf / json_vsprintf.
#[test]
fn f12_json_sprintf() {
    unsafe {
        let p = pair();
        let csp = p.c.json_sprintf_sym();
        let rsp = p.r.json_sprintf_sym();

        // No-argument formats, including the empty result.
        for fmt in ["", "x", "hello", "100%%", "a\tb\nc", "é€😀"] {
            let f = cs(fmt);
            let cj = csp(f.as_ptr());
            let rj = rsp(f.as_ptr());
            assert_same(p, cj, rj, &format!("json_sprintf({fmt:?})"));
            p.c.json_decref(cj);
            p.r.json_decref(rj);
        }
        // Integer arguments, incl. width/precision.
        for fmt in ["%d", "%5d", "%-5d|", "%05d", "%+d", "%x", "%X", "%o", "%u", "%i"] {
            for v in [0i32, 1, -1, 42, i32::MAX, i32::MIN] {
                let f = cs(fmt);
                let cj = csp(f.as_ptr(), v);
                let rj = rsp(f.as_ptr(), v);
                assert_same(p, cj, rj, &format!("json_sprintf({fmt:?}, {v})"));
                p.c.json_decref(cj);
                p.r.json_decref(rj);
            }
        }
        // String arguments, incl. non-ASCII and invalid UTF-8 (must be rejected).
        for fmt in ["%s", "[%s]", "%10s", "%-10s|", "%.3s"] {
            for s in ["", "a", "hello", "é€😀", "\u{10FFFF}"] {
                let f = cs(fmt);
                let a = cs(s);
                let cj = csp(f.as_ptr(), a.as_ptr());
                let rj = rsp(f.as_ptr(), a.as_ptr());
                assert_same(p, cj, rj, &format!("json_sprintf({fmt:?}, {s:?})"));
                p.c.json_decref(cj);
                p.r.json_decref(rj);
            }
            // Raw invalid UTF-8 argument -> utf8_check_string fails -> NULL
            for bad in [vec![0x80u8, 0], vec![0xFFu8, 0], vec![0xC3u8, 0]] {
                let f = cs(fmt);
                let cj = csp(f.as_ptr(), bad.as_ptr() as *const c_char);
                let rj = rsp(f.as_ptr(), bad.as_ptr() as *const c_char);
                assert_eq!(
                    cj.is_null(),
                    rj.is_null(),
                    "json_sprintf({fmt:?}, {bad:02x?}) NULL-ness"
                );
                assert_same(p, cj, rj, &format!("json_sprintf({fmt:?}, {bad:02x?})"));
                p.c.json_decref(cj);
                p.r.json_decref(rj);
            }
        }
        // Double arguments.
        for fmt in ["%f", "%.2f", "%e", "%g", "%20.10f"] {
            for v in [0.0f64, -0.0, 1.5, -1.5, 1e300, 1e-300, 1.0 / 3.0] {
                let f = cs(fmt);
                let cj = csp(f.as_ptr(), v);
                let rj = rsp(f.as_ptr(), v);
                assert_same(p, cj, rj, &format!("json_sprintf({fmt:?}, {v:?})"));
                p.c.json_decref(cj);
                p.r.json_decref(rj);
            }
        }
        // A result well over 1 KiB.
        let f = cs("%s%s%s%s");
        let big = cs(&"z".repeat(600));
        let cj = csp(f.as_ptr(), big.as_ptr(), big.as_ptr(), big.as_ptr(), big.as_ptr());
        let rj = rsp(f.as_ptr(), big.as_ptr(), big.as_ptr(), big.as_ptr(), big.as_ptr());
        assert_eq!(
            p.c.json_string_length(cj),
            p.r.json_string_length(rj),
            "json_sprintf large result length"
        );
        assert_same(p, cj, rj, "json_sprintf large result");
        p.c.json_decref(cj);
        p.r.json_decref(rj);
    }
}

/// E7 / ERRORS B5, B8, B18, Q4 (hardened): the one-past-the-end index.
///
/// A naive `index > entries` bug is normally INVISIBLE, because the slot at
/// `table[entries]` in a freshly-`malloc`ed array is usually already zero, so an
/// out-of-range read returns NULL anyway and looks correct. This test first
/// plants a *live, non-NULL* `json_t*` in that slot (append, keep a reference,
/// then remove — `json_array_remove` shrinks `entries` but leaves the stale
/// pointer in the table), so a boundary that is off by one becomes observable.
#[test]
fn e7_one_past_end_index_with_live_stale_slot() {
    unsafe {
        let p = pair();
        for n in [1usize, 2, 3, 8, 9, 16, 17] {
            let ca = p.c.json_array();
            let ra = p.r.json_array();
            for i in 0..n {
                p.c.json_array_append_new(ca, p.c.json_integer(1000 + i as i64));
                p.r.json_array_append_new(ra, p.r.json_integer(1000 + i as i64));
            }
            // Keep the last element alive across the removal so the stale slot
            // points at valid memory (we only ever compare NULL-ness, but this
            // keeps the test free of dangling reads either way).
            let ckeep = p.c.json_incref(p.c.json_array_get(ca, n - 1));
            let rkeep = p.r.json_incref(p.r.json_array_get(ra, n - 1));
            assert!(!ckeep.is_null() && !rkeep.is_null());

            assert_eq!(
                p.c.json_array_remove(ca, n - 1),
                p.r.json_array_remove(ra, n - 1),
                "remove(last) at n={n}"
            );
            assert_eq!(p.c.json_array_size(ca), n - 1);
            assert_eq!(p.r.json_array_size(ra), n - 1);

            // `table[entries]` now holds a live, non-NULL pointer. Every
            // one-past-the-end read must STILL be NULL.
            for idx in [n - 1, n, n + 1] {
                let cg = p.c.json_array_get(ca, idx);
                let rg = p.r.json_array_get(ra, idx);
                assert_eq!(
                    cg.is_null(),
                    rg.is_null(),
                    "json_array_get({idx}) after removing the last of {n}"
                );
                assert!(
                    cg.is_null(),
                    "C: json_array_get({idx}) on a {}-element array must be NULL",
                    n - 1
                );
            }
            // The same boundary on the mutating entry points.
            for idx in [n - 1, n, n + 1] {
                let cv = p.c.json_integer(7);
                let rv = p.r.json_integer(7);
                assert_eq!(
                    p.c.json_array_set_new(ca, idx, cv),
                    p.r.json_array_set_new(ra, idx, rv),
                    "json_array_set_new({idx}) at entries={}",
                    n - 1
                );
                assert_eq!(
                    p.c.json_array_remove(ca, idx),
                    p.r.json_array_remove(ra, idx),
                    "json_array_remove({idx}) at entries={}",
                    n - 1
                );
            }
            assert_same(p, ca, ra, &format!("array after boundary probing at n={n}"));
            p.c.json_decref(ckeep);
            p.r.json_decref(rkeep);
            p.c.json_decref(ca);
            p.r.json_decref(ra);
        }
    }
}

/// The same hardening for objects: after deleting a key, a stale `pair_t` may
/// still be reachable from a bucket chain, so a missing-key lookup that walks one
/// node too far becomes observable.
#[test]
fn d3_object_lookup_after_delete_with_live_values() {
    unsafe {
        let p = pair();
        for n in [1usize, 8, 9, 17, 33] {
            let co = p.c.json_object();
            let ro = p.r.json_object();
            let mut keys = Vec::new();
            for i in 0..n {
                let k = format!("k{i:04}");
                p.c.json_object_setn_new_nocheck(
                    co,
                    k.as_ptr() as *const c_char,
                    k.len(),
                    p.c.json_integer(i as i64),
                );
                p.r.json_object_setn_new_nocheck(
                    ro,
                    k.as_ptr() as *const c_char,
                    k.len(),
                    p.r.json_integer(i as i64),
                );
                keys.push(k);
            }
            // Delete every key one at a time; after each delete, probe EVERY key
            // (deleted and remaining) through all three lookup entry points.
            for d in 0..n {
                let dk = &keys[d];
                assert_eq!(
                    p.c.json_object_deln(co, dk.as_ptr() as *const c_char, dk.len()),
                    p.r.json_object_deln(ro, dk.as_ptr() as *const c_char, dk.len()),
                    "deln({dk}) at n={n}"
                );
                for k in &keys {
                    let cg = p.c.json_object_getn(co, k.as_ptr() as *const c_char, k.len());
                    let rg = p.r.json_object_getn(ro, k.as_ptr() as *const c_char, k.len());
                    assert_eq!(
                        cg.is_null(),
                        rg.is_null(),
                        "getn({k}) after deleting {dk} (n={n})"
                    );
                    if !cg.is_null() {
                        assert_eq!(
                            p.c.json_integer_value(cg),
                            p.r.json_integer_value(rg),
                            "getn({k}) value after deleting {dk}"
                        );
                    }
                    let z = raw_z(k.as_bytes());
                    assert_eq!(
                        p.c.json_object_iter_at(co, z.as_ptr() as *const c_char).is_null(),
                        p.r.json_object_iter_at(ro, z.as_ptr() as *const c_char).is_null(),
                        "iter_at({k}) after deleting {dk}"
                    );
                    assert_eq!(
                        p.c.json_object_deln(co, k.as_ptr() as *const c_char, k.len()) == 0,
                        p.r.json_object_deln(ro, k.as_ptr() as *const c_char, k.len()) == 0,
                        "second deln({k}) after deleting {dk}"
                    );
                    // Put it back if the delete above removed it, so the loop
                    // keeps a meaningful population.
                    if p.c.json_object_getn(co, k.as_ptr() as *const c_char, k.len()).is_null()
                        && k != dk
                    {
                        p.c.json_object_setn_new_nocheck(
                            co,
                            k.as_ptr() as *const c_char,
                            k.len(),
                            p.c.json_integer(-1),
                        );
                        p.r.json_object_setn_new_nocheck(
                            ro,
                            k.as_ptr() as *const c_char,
                            k.len(),
                            p.r.json_integer(-1),
                        );
                    }
                }
                assert_eq!(
                    object_entries(&p.c, co, 0),
                    object_entries(&p.r, ro, 0),
                    "traversal after deleting {dk} (n={n})"
                );
            }
            p.c.json_decref(co);
            p.r.json_decref(ro);
        }
    }
}
