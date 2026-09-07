//! Phase B — CONFIGS.md rows 36..52: mutation of arrays and objects.

mod common;
use common::*;
use std::ffi::{c_char, c_int};

unsafe fn cmp_state(label: &str, c: &Api, r: &Api, ci: Item, ri: Item) {
    assert_eq!(
        fingerprint(c, ci),
        fingerprint(r, ri),
        "{}: structure",
        label
    );
    assert_eq!(link_shape(ci), link_shape(ri), "{}: links", label);
    assert_eq!(
        show(&print_and_free(c, ci)),
        show(&print_and_free(r, ri)),
        "{}: Print",
        label
    );
    assert_eq!(
        show(&print_unformatted_and_free(c, ci)),
        show(&print_unformatted_and_free(r, ri)),
        "{}: PrintUnformatted",
        label
    );
    assert_eq!(
        (c.cJSON_GetArraySize)(ci),
        (r.cJSON_GetArraySize)(ri),
        "{}: GetArraySize",
        label
    );
}

fn arr(n: usize) -> Node {
    Node::Array((0..n).map(|i| Node::Number(i as f64)).collect())
}

fn obj(keys: &[&str]) -> Node {
    Node::Object(
        keys.iter()
            .enumerate()
            .map(|(i, k)| (k.to_string(), Node::Number(i as f64)))
            .collect(),
    )
}

/// Row 36
#[test]
fn row36_add_item_to_array() {
    unsafe {
        let (c, r) = both();
        let mut rng = Rng::new(36);
        for base in [0usize, 1, 2, 5] {
            for _ in 0..40 {
                let n = arr(base);
                let ci = build(&c, &n);
                let ri = build(&r, &n);
                for step in 0..4 {
                    let item = random_node(&mut rng, 2);
                    let a = build(&c, &item);
                    let b = build(&r, &item);
                    assert_eq!(
                        (c.cJSON_AddItemToArray)(ci, a),
                        (r.cJSON_AddItemToArray)(ri, b),
                        "row36 base={} step={} return",
                        base,
                        step
                    );
                    cmp_state(
                        &format!("row36 base={} step={}", base, step),
                        &c,
                        &r,
                        ci,
                        ri,
                    );
                }
                (c.cJSON_Delete)(ci);
                (r.cJSON_Delete)(ri);
            }
        }
    }
}

/// Rows 37 & 38
#[test]
fn row37_38_add_item_to_object() {
    unsafe {
        let (c, r) = both();
        let mut rng = Rng::new(37);
        for cs_variant in [false, true] {
            for base in [0usize, 1, 3] {
                for _ in 0..40 {
                    let n = obj(&["a", "b", "c"][..base]);
                    let ci = build(&c, &n);
                    let ri = build(&r, &n);
                    for step in 0..4 {
                        let key = if step == 0 {
                            "a".to_string()
                        } else {
                            rng.string(6)
                        };
                        let item = random_node(&mut rng, 2);
                        let a = build(&c, &item);
                        let b = build(&r, &item);
                        let kb = cbytes(key.as_bytes());
                        let kp = kb.as_ptr() as *const c_char;
                        let (rc, rr) = if cs_variant {
                            // CS stores the pointer, so it must be a stable key
                            let leaked: &'static [u8] =
                                Box::leak(cbytes(key.as_bytes()).into_boxed_slice());
                            let lp = leaked.as_ptr() as *const c_char;
                            (
                                (c.cJSON_AddItemToObjectCS)(ci, lp, a),
                                (r.cJSON_AddItemToObjectCS)(ri, lp, b),
                            )
                        } else {
                            (
                                (c.cJSON_AddItemToObject)(ci, kp, a),
                                (r.cJSON_AddItemToObject)(ri, kp, b),
                            )
                        };
                        assert_eq!(rc, rr, "row37/38 cs={} return", cs_variant);
                        cmp_state(
                            &format!("row37/38 cs={} base={} step={}", cs_variant, base, step),
                            &c,
                            &r,
                            ci,
                            ri,
                        );
                    }
                    (c.cJSON_Delete)(ci);
                    (r.cJSON_Delete)(ri);
                }
            }
        }
    }
}

/// Row 38b — re-keying an item that already carries a key (the branch in
/// `add_item_to_object` that frees the previous non-const key).  The item is
/// detached from its first parent first, because re-adding an item that is
/// still linked builds a cyclic list (and both implementations then loop
/// forever, which is the C behaviour but not testable).
#[test]
fn row38b_rekey_item() {
    unsafe {
        let (c, r) = both();
        for (first_cs, second_cs) in [(false, false), (false, true), (true, false), (true, true)] {
            let co1 = (c.cJSON_CreateObject)();
            let ro1 = (r.cJSON_CreateObject)();
            let co2 = (c.cJSON_CreateObject)();
            let ro2 = (r.cJSON_CreateObject)();
            let ca = (c.cJSON_CreateNumber)(1.0);
            let ra = (r.cJSON_CreateNumber)(1.0);
            let k1: &'static [u8] = Box::leak(cbytes(b"first").into_boxed_slice());
            let k2: &'static [u8] = Box::leak(cbytes(b"second").into_boxed_slice());
            let k1p = k1.as_ptr() as *const c_char;
            let k2p = k2.as_ptr() as *const c_char;

            let rc1 = if first_cs {
                (c.cJSON_AddItemToObjectCS)(co1, k1p, ca)
            } else {
                (c.cJSON_AddItemToObject)(co1, k1p, ca)
            };
            let rr1 = if first_cs {
                (r.cJSON_AddItemToObjectCS)(ro1, k1p, ra)
            } else {
                (r.cJSON_AddItemToObject)(ro1, k1p, ra)
            };
            assert_eq!(rc1, rr1, "row38b first add");
            assert_eq!((*ca).type_, (*ra).type_, "row38b type after first add");
            assert_eq!(
                read_cstr((*ca).string),
                read_cstr((*ra).string),
                "row38b key after first add"
            );

            let dc = (c.cJSON_DetachItemViaPointer)(co1, ca);
            let dr = (r.cJSON_DetachItemViaPointer)(ro1, ra);
            assert_eq!(dc.is_null(), dr.is_null(), "row38b detach");

            let rc2 = if second_cs {
                (c.cJSON_AddItemToObjectCS)(co2, k2p, ca)
            } else {
                (c.cJSON_AddItemToObject)(co2, k2p, ca)
            };
            let rr2 = if second_cs {
                (r.cJSON_AddItemToObjectCS)(ro2, k2p, ra)
            } else {
                (r.cJSON_AddItemToObject)(ro2, k2p, ra)
            };
            assert_eq!(rc2, rr2, "row38b second add");
            cmp_state(
                &format!("row38b old parent {} {}", first_cs, second_cs),
                &c,
                &r,
                co1,
                ro1,
            );
            cmp_state(
                &format!("row38b new parent {} {}", first_cs, second_cs),
                &c,
                &r,
                co2,
                ro2,
            );
            (c.cJSON_Delete)(co1);
            (r.cJSON_Delete)(ro1);
            (c.cJSON_Delete)(co2);
            (r.cJSON_Delete)(ro2);
        }
    }
}

/// Row 39
#[test]
fn row39_add_item_reference() {
    unsafe {
        let (c, r) = both();
        let mut rng = Rng::new(39);
        for _ in 0..80 {
            let shared = random_node(&mut rng, 3);
            let cs_ = build(&c, &shared);
            let rs = build(&r, &shared);

            let ca = (c.cJSON_CreateArray)();
            let ra = (r.cJSON_CreateArray)();
            let co = (c.cJSON_CreateObject)();
            let ro = (r.cJSON_CreateObject)();

            assert_eq!(
                (c.cJSON_AddItemReferenceToArray)(ca, cs_),
                (r.cJSON_AddItemReferenceToArray)(ra, rs),
                "row39 ref to array"
            );
            let kb = cbytes(b"ref");
            let kp = kb.as_ptr() as *const c_char;
            assert_eq!(
                (c.cJSON_AddItemReferenceToObject)(co, kp, cs_),
                (r.cJSON_AddItemReferenceToObject)(ro, kp, rs),
                "row39 ref to object"
            );
            cmp_state("row39 array parent", &c, &r, ca, ra);
            cmp_state("row39 object parent", &c, &r, co, ro);
            (c.cJSON_Delete)(ca);
            (r.cJSON_Delete)(ra);
            (c.cJSON_Delete)(co);
            (r.cJSON_Delete)(ro);
            // shared subtree must have survived
            cmp_state("row39 shared survives", &c, &r, cs_, rs);
            (c.cJSON_Delete)(cs_);
            (r.cJSON_Delete)(rs);
        }
    }
}

/// Rows 40, 41, 42, 43 — the Add*ToObject convenience helpers.
#[test]
fn row40_43_add_helpers() {
    unsafe {
        let (c, r) = both();
        let mut rng = Rng::new(40);
        for _ in 0..120 {
            let co = (c.cJSON_CreateObject)();
            let ro = (r.cJSON_CreateObject)();
            macro_rules! keyed {
                ($k:expr) => {{
                    let kb = cbytes($k);
                    (kb, ())
                }};
            }
            let (k1, _) = keyed!(b"n");
            assert_eq!(
                (c.cJSON_AddNullToObject)(co, k1.as_ptr() as *const c_char).is_null(),
                (r.cJSON_AddNullToObject)(ro, k1.as_ptr() as *const c_char).is_null()
            );
            let (k2, _) = keyed!(b"t");
            assert_eq!(
                (c.cJSON_AddTrueToObject)(co, k2.as_ptr() as *const c_char).is_null(),
                (r.cJSON_AddTrueToObject)(ro, k2.as_ptr() as *const c_char).is_null()
            );
            let (k3, _) = keyed!(b"f");
            assert_eq!(
                (c.cJSON_AddFalseToObject)(co, k3.as_ptr() as *const c_char).is_null(),
                (r.cJSON_AddFalseToObject)(ro, k3.as_ptr() as *const c_char).is_null()
            );
            for (i, bv) in [0i32, 1, 2, -5, c_int::MAX].iter().enumerate() {
                let kb = cbytes(format!("b{}", i).as_bytes());
                assert_eq!(
                    (c.cJSON_AddBoolToObject)(co, kb.as_ptr() as *const c_char, *bv).is_null(),
                    (r.cJSON_AddBoolToObject)(ro, kb.as_ptr() as *const c_char, *bv).is_null()
                );
            }
            for i in 0..6 {
                let d = rng.f64();
                let kb = cbytes(format!("d{}", i).as_bytes());
                assert_eq!(
                    (c.cJSON_AddNumberToObject)(co, kb.as_ptr() as *const c_char, d).is_null(),
                    (r.cJSON_AddNumberToObject)(ro, kb.as_ptr() as *const c_char, d).is_null()
                );
            }
            for i in 0..4 {
                let s = rng.string(16);
                let sb = cbytes(s.as_bytes());
                let kb = cbytes(format!("s{}", i).as_bytes());
                assert_eq!(
                    (c.cJSON_AddStringToObject)(
                        co,
                        kb.as_ptr() as *const c_char,
                        sb.as_ptr() as *const c_char
                    )
                    .is_null(),
                    (r.cJSON_AddStringToObject)(
                        ro,
                        kb.as_ptr() as *const c_char,
                        sb.as_ptr() as *const c_char
                    )
                    .is_null()
                );
                let kb = cbytes(format!("w{}", i).as_bytes());
                assert_eq!(
                    (c.cJSON_AddRawToObject)(
                        co,
                        kb.as_ptr() as *const c_char,
                        sb.as_ptr() as *const c_char
                    )
                    .is_null(),
                    (r.cJSON_AddRawToObject)(
                        ro,
                        kb.as_ptr() as *const c_char,
                        sb.as_ptr() as *const c_char
                    )
                    .is_null()
                );
            }
            // Add nested object and array, then populate them.
            let ko = cbytes(b"obj");
            let cno = (c.cJSON_AddObjectToObject)(co, ko.as_ptr() as *const c_char);
            let rno = (r.cJSON_AddObjectToObject)(ro, ko.as_ptr() as *const c_char);
            assert_eq!(cno.is_null(), rno.is_null());
            let ka = cbytes(b"arr");
            let cna = (c.cJSON_AddArrayToObject)(co, ka.as_ptr() as *const c_char);
            let rna = (r.cJSON_AddArrayToObject)(ro, ka.as_ptr() as *const c_char);
            assert_eq!(cna.is_null(), rna.is_null());
            if !cno.is_null() {
                let kx = cbytes(b"x");
                (c.cJSON_AddNumberToObject)(cno, kx.as_ptr() as *const c_char, 1.0);
                (r.cJSON_AddNumberToObject)(rno, kx.as_ptr() as *const c_char, 1.0);
            }
            if !cna.is_null() {
                (c.cJSON_AddItemToArray)(cna, (c.cJSON_CreateNumber)(2.0));
                (r.cJSON_AddItemToArray)(rna, (r.cJSON_CreateNumber)(2.0));
            }
            cmp_state("row40_43", &c, &r, co, ro);
            (c.cJSON_Delete)(co);
            (r.cJSON_Delete)(ro);
        }
    }
}

/// Row 44
#[test]
fn row44_detach_via_pointer() {
    unsafe {
        let (c, r) = both();
        for size in [1usize, 2, 3, 6] {
            for pos in 0..size {
                for is_object in [false, true] {
                    let n = if is_object {
                        Node::Object(
                            (0..size)
                                .map(|i| (format!("k{}", i), Node::Number(i as f64)))
                                .collect(),
                        )
                    } else {
                        arr(size)
                    };
                    let ci = build(&c, &n);
                    let ri = build(&r, &n);
                    let ca = (c.cJSON_GetArrayItem)(ci, pos as c_int);
                    let ra = (r.cJSON_GetArrayItem)(ri, pos as c_int);
                    let dc = (c.cJSON_DetachItemViaPointer)(ci, ca);
                    let dr = (r.cJSON_DetachItemViaPointer)(ri, ra);
                    assert_eq!(
                        dc.is_null(),
                        dr.is_null(),
                        "row44 size={} pos={} obj={} NULL-ness",
                        size,
                        pos,
                        is_object
                    );
                    cmp_state(
                        &format!("row44 parent size={} pos={} obj={}", size, pos, is_object),
                        &c,
                        &r,
                        ci,
                        ri,
                    );
                    if !dc.is_null() {
                        cmp_state("row44 detached", &c, &r, dc, dr);
                        (c.cJSON_Delete)(dc);
                        (r.cJSON_Delete)(dr);
                    }
                    (c.cJSON_Delete)(ci);
                    (r.cJSON_Delete)(ri);
                }
            }
        }
    }
}

/// Row 45
#[test]
fn row45_detach_from_array() {
    unsafe {
        let (c, r) = both();
        for size in [1usize, 2, 5] {
            for which in 0..(size + 1) {
                let n = arr(size);
                let ci = build(&c, &n);
                let ri = build(&r, &n);
                let dc = (c.cJSON_DetachItemFromArray)(ci, which as c_int);
                let dr = (r.cJSON_DetachItemFromArray)(ri, which as c_int);
                assert_eq!(dc.is_null(), dr.is_null(), "row45 size={} which={}", size, which);
                cmp_state(
                    &format!("row45 size={} which={}", size, which),
                    &c,
                    &r,
                    ci,
                    ri,
                );
                if !dc.is_null() {
                    cmp_state("row45 detached", &c, &r, dc, dr);
                    (c.cJSON_Delete)(dc);
                    (r.cJSON_Delete)(dr);
                }
                (c.cJSON_Delete)(ci);
                (r.cJSON_Delete)(ri);
            }
        }
    }
}

/// Row 46
#[test]
fn row46_detach_from_object() {
    unsafe {
        let (c, r) = both();
        let keys = ["Alpha", "beta", "Gamma", "delta"];
        for probe in ["Alpha", "alpha", "ALPHA", "beta", "Gamma", "delta", "missing", ""] {
            for case_sensitive in [false, true] {
                let n = obj(&keys);
                let ci = build(&c, &n);
                let ri = build(&r, &n);
                let kb = cbytes(probe.as_bytes());
                let kp = kb.as_ptr() as *const c_char;
                let (dc, dr) = if case_sensitive {
                    (
                        (c.cJSON_DetachItemFromObjectCaseSensitive)(ci, kp),
                        (r.cJSON_DetachItemFromObjectCaseSensitive)(ri, kp),
                    )
                } else {
                    (
                        (c.cJSON_DetachItemFromObject)(ci, kp),
                        (r.cJSON_DetachItemFromObject)(ri, kp),
                    )
                };
                assert_eq!(
                    dc.is_null(),
                    dr.is_null(),
                    "row46 probe={:?} cs={}",
                    probe,
                    case_sensitive
                );
                cmp_state(
                    &format!("row46 probe={:?} cs={}", probe, case_sensitive),
                    &c,
                    &r,
                    ci,
                    ri,
                );
                if !dc.is_null() {
                    cmp_state("row46 detached", &c, &r, dc, dr);
                    (c.cJSON_Delete)(dc);
                    (r.cJSON_Delete)(dr);
                }
                (c.cJSON_Delete)(ci);
                (r.cJSON_Delete)(ri);
            }
        }
    }
}

/// Rows 47 & 48
#[test]
fn row47_48_delete_item() {
    unsafe {
        let (c, r) = both();
        for size in [0usize, 1, 2, 5] {
            for which in [-1i32, 0, 1, size as i32 - 1, size as i32, size as i32 + 1] {
                let n = arr(size);
                let ci = build(&c, &n);
                let ri = build(&r, &n);
                (c.cJSON_DeleteItemFromArray)(ci, which);
                (r.cJSON_DeleteItemFromArray)(ri, which);
                cmp_state(&format!("row47 size={} which={}", size, which), &c, &r, ci, ri);
                (c.cJSON_Delete)(ci);
                (r.cJSON_Delete)(ri);
            }
        }
        let keys = ["Alpha", "beta", "Gamma"];
        for probe in ["Alpha", "alpha", "beta", "BETA", "missing", ""] {
            for case_sensitive in [false, true] {
                let n = obj(&keys);
                let ci = build(&c, &n);
                let ri = build(&r, &n);
                let kb = cbytes(probe.as_bytes());
                let kp = kb.as_ptr() as *const c_char;
                if case_sensitive {
                    (c.cJSON_DeleteItemFromObjectCaseSensitive)(ci, kp);
                    (r.cJSON_DeleteItemFromObjectCaseSensitive)(ri, kp);
                } else {
                    (c.cJSON_DeleteItemFromObject)(ci, kp);
                    (r.cJSON_DeleteItemFromObject)(ri, kp);
                }
                cmp_state(
                    &format!("row48 probe={:?} cs={}", probe, case_sensitive),
                    &c,
                    &r,
                    ci,
                    ri,
                );
                (c.cJSON_Delete)(ci);
                (r.cJSON_Delete)(ri);
            }
        }
    }
}

/// Row 49
#[test]
fn row49_insert_item_in_array() {
    unsafe {
        let (c, r) = both();
        let mut rng = Rng::new(49);
        for size in [0usize, 1, 2, 5] {
            for which in 0..(size + 2) {
                for _ in 0..10 {
                    let n = arr(size);
                    let ci = build(&c, &n);
                    let ri = build(&r, &n);
                    let item = random_node(&mut rng, 2);
                    let a = build(&c, &item);
                    let b = build(&r, &item);
                    assert_eq!(
                        (c.cJSON_InsertItemInArray)(ci, which as c_int, a),
                        (r.cJSON_InsertItemInArray)(ri, which as c_int, b),
                        "row49 size={} which={} return",
                        size,
                        which
                    );
                    cmp_state(
                        &format!("row49 size={} which={}", size, which),
                        &c,
                        &r,
                        ci,
                        ri,
                    );
                    // add one more afterwards to prove prev/next bookkeeping is intact
                    let extra = Node::Str("tail".into());
                    let ea = build(&c, &extra);
                    let eb = build(&r, &extra);
                    (c.cJSON_AddItemToArray)(ci, ea);
                    (r.cJSON_AddItemToArray)(ri, eb);
                    cmp_state(
                        &format!("row49 after append size={} which={}", size, which),
                        &c,
                        &r,
                        ci,
                        ri,
                    );
                    (c.cJSON_Delete)(ci);
                    (r.cJSON_Delete)(ri);
                }
            }
        }
    }
}

/// Row 50
#[test]
fn row50_replace_via_pointer() {
    unsafe {
        let (c, r) = both();
        let mut rng = Rng::new(50);
        for size in [1usize, 2, 3, 6] {
            for pos in 0..size {
                for _ in 0..10 {
                    let n = arr(size);
                    let ci = build(&c, &n);
                    let ri = build(&r, &n);
                    let ca = (c.cJSON_GetArrayItem)(ci, pos as c_int);
                    let ra = (r.cJSON_GetArrayItem)(ri, pos as c_int);
                    let item = random_node(&mut rng, 2);
                    let a = build(&c, &item);
                    let b = build(&r, &item);
                    assert_eq!(
                        (c.cJSON_ReplaceItemViaPointer)(ci, ca, a),
                        (r.cJSON_ReplaceItemViaPointer)(ri, ra, b),
                        "row50 size={} pos={} return",
                        size,
                        pos
                    );
                    cmp_state(&format!("row50 size={} pos={}", size, pos), &c, &r, ci, ri);
                    // append to verify list tail bookkeeping
                    (c.cJSON_AddItemToArray)(ci, (c.cJSON_CreateNumber)(99.0));
                    (r.cJSON_AddItemToArray)(ri, (r.cJSON_CreateNumber)(99.0));
                    cmp_state(
                        &format!("row50 append size={} pos={}", size, pos),
                        &c,
                        &r,
                        ci,
                        ri,
                    );
                    (c.cJSON_Delete)(ci);
                    (r.cJSON_Delete)(ri);
                }
            }
        }
        // replacement == item
        for size in [1usize, 3] {
            let n = arr(size);
            let ci = build(&c, &n);
            let ri = build(&r, &n);
            let ca = (c.cJSON_GetArrayItem)(ci, 0);
            let ra = (r.cJSON_GetArrayItem)(ri, 0);
            assert_eq!(
                (c.cJSON_ReplaceItemViaPointer)(ci, ca, ca),
                (r.cJSON_ReplaceItemViaPointer)(ri, ra, ra),
                "row50 self-replace"
            );
            cmp_state("row50 self-replace", &c, &r, ci, ri);
            (c.cJSON_Delete)(ci);
            (r.cJSON_Delete)(ri);
        }
    }
}

/// Row 51
#[test]
fn row51_replace_item_in_array() {
    unsafe {
        let (c, r) = both();
        let mut rng = Rng::new(51);
        for size in [0usize, 1, 2, 5] {
            for which in [-1i32, 0, 1, size as i32 - 1, size as i32, size as i32 + 3] {
                for _ in 0..5 {
                    let n = arr(size);
                    let ci = build(&c, &n);
                    let ri = build(&r, &n);
                    let item = random_node(&mut rng, 2);
                    let a = build(&c, &item);
                    let b = build(&r, &item);
                    let rc = (c.cJSON_ReplaceItemInArray)(ci, which, a);
                    let rr = (r.cJSON_ReplaceItemInArray)(ri, which, b);
                    assert_eq!(rc, rr, "row51 size={} which={} return", size, which);
                    cmp_state(&format!("row51 size={} which={}", size, which), &c, &r, ci, ri);
                    if rc == 0 {
                        // replacement was not adopted; free it
                        (c.cJSON_Delete)(a);
                        (r.cJSON_Delete)(b);
                    }
                    (c.cJSON_Delete)(ci);
                    (r.cJSON_Delete)(ri);
                }
            }
        }
    }
}

/// Row 52
#[test]
fn row52_replace_item_in_object() {
    unsafe {
        let (c, r) = both();
        let mut rng = Rng::new(52);
        let keys = ["Alpha", "beta", "Gamma", "delta"];
        for probe in ["Alpha", "alpha", "ALPHA", "beta", "delta", "missing", ""] {
            for case_sensitive in [false, true] {
                for _ in 0..5 {
                    let n = obj(&keys);
                    let ci = build(&c, &n);
                    let ri = build(&r, &n);
                    let item = random_node(&mut rng, 2);
                    let a = build(&c, &item);
                    let b = build(&r, &item);
                    let kb = cbytes(probe.as_bytes());
                    let kp = kb.as_ptr() as *const c_char;
                    let (rc, rr) = if case_sensitive {
                        (
                            (c.cJSON_ReplaceItemInObjectCaseSensitive)(ci, kp, a),
                            (r.cJSON_ReplaceItemInObjectCaseSensitive)(ri, kp, b),
                        )
                    } else {
                        (
                            (c.cJSON_ReplaceItemInObject)(ci, kp, a),
                            (r.cJSON_ReplaceItemInObject)(ri, kp, b),
                        )
                    };
                    assert_eq!(
                        rc, rr,
                        "row52 probe={:?} cs={} return",
                        probe, case_sensitive
                    );
                    cmp_state(
                        &format!("row52 probe={:?} cs={}", probe, case_sensitive),
                        &c,
                        &r,
                        ci,
                        ri,
                    );
                    // The replacement's own key must match too (it is rewritten
                    // even when the replace itself fails).
                    assert_eq!(
                        read_cstr((*a).string),
                        read_cstr((*b).string),
                        "row52 probe={:?} cs={} replacement key",
                        probe,
                        case_sensitive
                    );
                    assert_eq!(
                        (*a).type_,
                        (*b).type_,
                        "row52 probe={:?} replacement type",
                        probe
                    );
                    if rc == 0 {
                        (c.cJSON_Delete)(a);
                        (r.cJSON_Delete)(b);
                    }
                    (c.cJSON_Delete)(ci);
                    (r.cJSON_Delete)(ri);
                }
            }
        }
    }
}
