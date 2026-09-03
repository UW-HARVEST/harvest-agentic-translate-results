//! Phase B — valid-path differential tests, part 2: the construction,
//! mutation and query API. Covers CONFIGS.md rows 37-81 and 91-92.

mod common;

use common::*;
use std::ffi::{c_char, c_int};

/* ------------------------------------------------------------------ */
/* helpers                                                            */
/* ------------------------------------------------------------------ */

unsafe fn both_prints(p: &Pair, c: *mut CJson, r: *mut CJson, what: &str) {
    let cf = take_printed(p.c, (p.c.cJSON_Print)(c));
    let rf = take_printed(p.r, (p.r.cJSON_Print)(r));
    assert!(cf == rf, "{what}: Print differs\n C: {}\n R: {}", show(&cf), show(&rf));
    let cu = take_printed(p.c, (p.c.cJSON_PrintUnformatted)(c));
    let ru = take_printed(p.r, (p.r.cJSON_PrintUnformatted)(r));
    assert!(cu == ru, "{what}: PrintUnformatted differs\n C: {}\n R: {}", show(&cu), show(&ru));
    assert!(
        snapshot(c) == snapshot(r),
        "{what}: tree snapshot differs\n C: {:?}\n R: {:?}",
        snapshot(c),
        snapshot(r)
    );
    assert_eq!(
        (p.c.cJSON_GetArraySize)(c),
        (p.r.cJSON_GetArraySize)(r),
        "{what}: GetArraySize differs"
    );
}

/// Build the identical object in both libraries using all nine
/// `cJSON_Add*ToObject` helpers.
unsafe fn build_kitchen_sink(api: &Api, boolean: c_int) -> *mut CJson {
    let o = (api.cJSON_CreateObject)();
    let (n, t, f, b, num, s, raw, obj, arr) = (
        cs("n"), cs("t"), cs("f"), cs("b"), cs("num"), cs("s"), cs("raw"), cs("obj"), cs("arr"),
    );
    (api.cJSON_AddNullToObject)(o, n.as_ptr());
    (api.cJSON_AddTrueToObject)(o, t.as_ptr());
    (api.cJSON_AddFalseToObject)(o, f.as_ptr());
    (api.cJSON_AddBoolToObject)(o, b.as_ptr(), boolean);
    (api.cJSON_AddNumberToObject)(o, num.as_ptr(), 1.5);
    let sv = cs("str\tval");
    (api.cJSON_AddStringToObject)(o, s.as_ptr(), sv.as_ptr());
    let rv = cs("[1,2]");
    (api.cJSON_AddRawToObject)(o, raw.as_ptr(), rv.as_ptr());
    let inner = (api.cJSON_AddObjectToObject)(o, obj.as_ptr());
    let k = cs("deep");
    (api.cJSON_AddNumberToObject)(inner, k.as_ptr(), 2.0);
    let a = (api.cJSON_AddArrayToObject)(o, arr.as_ptr());
    for i in 0..3 {
        (api.cJSON_AddItemToArray)(a, (api.cJSON_CreateNumber)(i as f64));
    }
    o
}

/* ================================================================== */
/* rows 37-44: creation entry points                                   */
/* ================================================================== */

#[test]
fn row_37_52_create_bool_and_add_bool() {
    let _g = lock();
    let p = pair();
    unsafe {
        for &b in &[0i32, 1, 2, -1, i32::MAX, i32::MIN, 256, 512] {
            let c = (p.c.cJSON_CreateBool)(b);
            let r = (p.r.cJSON_CreateBool)(b);
            assert_eq!((*c).type_, (*r).type_, "CreateBool({b}) type");
            both_prints(p, c, r, &format!("CreateBool({b})"));
            (p.c.cJSON_Delete)(c);
            (p.r.cJSON_Delete)(r);

            let co = build_kitchen_sink(p.c, b);
            let ro = build_kitchen_sink(p.r, b);
            both_prints(p, co, ro, &format!("kitchen_sink(bool={b})"));
            (p.c.cJSON_Delete)(co);
            (p.r.cJSON_Delete)(ro);
        }
    }
}

#[test]
fn row_38_create_string_and_raw() {
    let _g = lock();
    let p = pair();
    let payloads: &[&[u8]] = &[
        b"",
        b"a",
        b"hello",
        b"with \"quotes\"",
        b"tab\there",
        b"\x01\x02\x1f",
        b"\x7f",
        b"\x80\xc3\xa9\xff",
        b"very long string ---------------------------------------------------------------",
        b"{\"looks\":\"like json\"}",
    ];
    unsafe {
        for pl in payloads {
            let buf = cbytes(pl);
            for which in 0..3 {
                let (c, r) = match which {
                    0 => (
                        (p.c.cJSON_CreateString)(buf.as_ptr()),
                        (p.r.cJSON_CreateString)(buf.as_ptr()),
                    ),
                    1 => (
                        (p.c.cJSON_CreateRaw)(buf.as_ptr()),
                        (p.r.cJSON_CreateRaw)(buf.as_ptr()),
                    ),
                    _ => (
                        (p.c.cJSON_CreateStringReference)(buf.as_ptr()),
                        (p.r.cJSON_CreateStringReference)(buf.as_ptr()),
                    ),
                };
                assert_eq!((*c).type_, (*r).type_, "which={which} type");
                both_prints(p, c, r, &format!("create which={which} {:?}", String::from_utf8_lossy(pl)));
                assert_eq!(
                    take_cstr((p.c.cJSON_GetStringValue)(c)),
                    take_cstr((p.r.cJSON_GetStringValue)(r)),
                    "which={which} GetStringValue"
                );
                (p.c.cJSON_Delete)(c);
                (p.r.cJSON_Delete)(r);
            }
        }
    }
}

#[test]
fn row_39_40_reference_items() {
    let _g = lock();
    let p = pair();
    unsafe {
        // Object/Array references share the child list of a live node.
        for as_object in [false, true] {
            let cbase = if as_object {
                (p.c.cJSON_CreateObject)()
            } else {
                (p.c.cJSON_CreateArray)()
            };
            let rbase = if as_object {
                (p.r.cJSON_CreateObject)()
            } else {
                (p.r.cJSON_CreateArray)()
            };
            for i in 0..4 {
                let k = cs(&format!("k{i}"));
                if as_object {
                    (p.c.cJSON_AddNumberToObject)(cbase, k.as_ptr(), i as f64);
                    (p.r.cJSON_AddNumberToObject)(rbase, k.as_ptr(), i as f64);
                } else {
                    (p.c.cJSON_AddItemToArray)(cbase, (p.c.cJSON_CreateNumber)(i as f64));
                    (p.r.cJSON_AddItemToArray)(rbase, (p.r.cJSON_CreateNumber)(i as f64));
                }
            }
            let cref = if as_object {
                (p.c.cJSON_CreateObjectReference)((*cbase).child)
            } else {
                (p.c.cJSON_CreateArrayReference)((*cbase).child)
            };
            let rref = if as_object {
                (p.r.cJSON_CreateObjectReference)((*rbase).child)
            } else {
                (p.r.cJSON_CreateArrayReference)((*rbase).child)
            };
            assert_eq!((*cref).type_, (*rref).type_, "reference type");
            both_prints(p, cref, rref, "reference item");
            // Deleting the reference must not free the shared children.
            (p.c.cJSON_Delete)(cref);
            (p.r.cJSON_Delete)(rref);
            both_prints(p, cbase, rbase, "base after reference deleted");
            (p.c.cJSON_Delete)(cbase);
            (p.r.cJSON_Delete)(rbase);
        }
    }
}

#[test]
fn rows_41_44_typed_array_constructors() {
    let _g = lock();
    let p = pair();
    let mut rng = Rng::new(0x5EED_0011);
    unsafe {
        for &count in &[0i32, 1, 2, 3, 7, 16, 64] {
            for trial in 0..12 {
                let ints: Vec<c_int> = (0..count.max(1))
                    .map(|i| match (trial + i) % 6 {
                        0 => 0,
                        1 => i32::MAX,
                        2 => i32::MIN,
                        3 => -1,
                        4 => 1234567,
                        _ => rng.i32(),
                    })
                    .collect();
                let floats: Vec<f32> = (0..count.max(1))
                    .map(|i| match (trial + i) % 7 {
                        0 => 0.0f32,
                        1 => -0.0f32,
                        2 => f32::NAN,
                        3 => f32::INFINITY,
                        4 => f32::NEG_INFINITY,
                        5 => f32::MIN_POSITIVE,
                        _ => f32::from_bits(rng.next_u64() as u32),
                    })
                    .collect();
                let doubles: Vec<f64> = (0..count.max(1))
                    .map(|i| match (trial + i) % 8 {
                        0 => 0.0,
                        1 => -0.0,
                        2 => f64::NAN,
                        3 => f64::INFINITY,
                        4 => f64::NEG_INFINITY,
                        5 => i32::MAX as f64 + 1.0,
                        6 => i32::MIN as f64 - 1.0,
                        _ => rng.f64(),
                    })
                    .collect();
                let owned: Vec<Vec<c_char>> = (0..count.max(1))
                    .map(|i| cbytes(format!("s{}-{}", trial, i).as_bytes()))
                    .collect();
                let strs: Vec<*const c_char> = owned.iter().map(|v| v.as_ptr()).collect();

                let ci = (p.c.cJSON_CreateIntArray)(ints.as_ptr(), count);
                let ri = (p.r.cJSON_CreateIntArray)(ints.as_ptr(), count);
                both_prints(p, ci, ri, &format!("IntArray(count={count},trial={trial})"));
                (p.c.cJSON_Delete)(ci);
                (p.r.cJSON_Delete)(ri);

                let cf = (p.c.cJSON_CreateFloatArray)(floats.as_ptr(), count);
                let rf = (p.r.cJSON_CreateFloatArray)(floats.as_ptr(), count);
                both_prints(p, cf, rf, &format!("FloatArray(count={count},trial={trial})"));
                (p.c.cJSON_Delete)(cf);
                (p.r.cJSON_Delete)(rf);

                let cd = (p.c.cJSON_CreateDoubleArray)(doubles.as_ptr(), count);
                let rd = (p.r.cJSON_CreateDoubleArray)(doubles.as_ptr(), count);
                both_prints(p, cd, rd, &format!("DoubleArray(count={count},trial={trial})"));
                (p.c.cJSON_Delete)(cd);
                (p.r.cJSON_Delete)(rd);

                let cst = (p.c.cJSON_CreateStringArray)(strs.as_ptr(), count);
                let rst = (p.r.cJSON_CreateStringArray)(strs.as_ptr(), count);
                both_prints(p, cst, rst, &format!("StringArray(count={count},trial={trial})"));
                (p.c.cJSON_Delete)(cst);
                (p.r.cJSON_Delete)(rst);
            }
        }
    }
}

/* ================================================================== */
/* rows 45-50: append paths, const keys, references                    */
/* ================================================================== */

#[test]
fn rows_45_48_add_item_to_object_const_vs_copied() {
    let _g = lock();
    let p = pair();
    unsafe {
        for use_cs in [false, true] {
            let co = (p.c.cJSON_CreateObject)();
            let ro = (p.r.cJSON_CreateObject)();
            let keys: Vec<_> = (0..5).map(|i| cs(&format!("key{i}"))).collect();
            for (i, k) in keys.iter().enumerate() {
                let cn = (p.c.cJSON_CreateNumber)(i as f64);
                let rn = (p.r.cJSON_CreateNumber)(i as f64);
                let (ca, ra) = if use_cs {
                    (
                        (p.c.cJSON_AddItemToObjectCS)(co, k.as_ptr(), cn),
                        (p.r.cJSON_AddItemToObjectCS)(ro, k.as_ptr(), rn),
                    )
                } else {
                    (
                        (p.c.cJSON_AddItemToObject)(co, k.as_ptr(), cn),
                        (p.r.cJSON_AddItemToObject)(ro, k.as_ptr(), rn),
                    )
                };
                assert_eq!(ca, ra, "add (cs={use_cs}) return");
                assert_eq!((*cn).type_, (*rn).type_, "add (cs={use_cs}) type bits");
            }
            both_prints(p, co, ro, &format!("object(cs={use_cs})"));

            // Re-key an existing item: exercises the `StringIsConst` branch of
            // add_item_to_object that frees (or keeps) the previous key.
            let cchild = (*co).child;
            let rchild = (*ro).child;
            let detached_c = (p.c.cJSON_DetachItemViaPointer)(co, cchild);
            let detached_r = (p.r.cJSON_DetachItemViaPointer)(ro, rchild);
            let newkey = cs("rekeyed");
            let cr = (p.c.cJSON_AddItemToObject)(co, newkey.as_ptr(), detached_c);
            let rr = (p.r.cJSON_AddItemToObject)(ro, newkey.as_ptr(), detached_r);
            assert_eq!(cr, rr, "rekey return (cs={use_cs})");
            assert_eq!((*detached_c).type_, (*detached_r).type_, "rekey type bits");
            both_prints(p, co, ro, &format!("object rekeyed (cs={use_cs})"));

            (p.c.cJSON_Delete)(co);
            (p.r.cJSON_Delete)(ro);
        }
    }
}

#[test]
fn rows_49_50_add_item_reference() {
    let _g = lock();
    let p = pair();
    unsafe {
        for keyed in [false, true] {
            // shared subtree
            let csub = (p.c.cJSON_CreateArray)();
            let rsub = (p.r.cJSON_CreateArray)();
            for i in 0..3 {
                (p.c.cJSON_AddItemToArray)(csub, (p.c.cJSON_CreateNumber)(i as f64));
                (p.r.cJSON_AddItemToArray)(rsub, (p.r.cJSON_CreateNumber)(i as f64));
            }
            let (cp_, rp_) = if keyed {
                ((p.c.cJSON_CreateObject)(), (p.r.cJSON_CreateObject)())
            } else {
                ((p.c.cJSON_CreateArray)(), (p.r.cJSON_CreateArray)())
            };
            let key = cs("ref");
            let (ca, ra) = if keyed {
                (
                    (p.c.cJSON_AddItemReferenceToObject)(cp_, key.as_ptr(), csub),
                    (p.r.cJSON_AddItemReferenceToObject)(rp_, key.as_ptr(), rsub),
                )
            } else {
                (
                    (p.c.cJSON_AddItemReferenceToArray)(cp_, csub),
                    (p.r.cJSON_AddItemReferenceToArray)(rp_, rsub),
                )
            };
            assert_eq!(ca, ra, "AddItemReference (keyed={keyed}) return");
            both_prints(p, cp_, rp_, &format!("parent with reference (keyed={keyed})"));
            // deleting the parent must leave the referenced subtree intact
            (p.c.cJSON_Delete)(cp_);
            (p.r.cJSON_Delete)(rp_);
            both_prints(p, csub, rsub, &format!("subtree survives (keyed={keyed})"));
            (p.c.cJSON_Delete)(csub);
            (p.r.cJSON_Delete)(rsub);
        }
    }
}

/* ================================================================== */
/* rows 53-58: queries over every array size / key variant             */
/* ================================================================== */

#[test]
fn rows_53_58_queries() {
    let _g = lock();
    let p = pair();
    unsafe {
        for size in 0..8i32 {
            let ca = (p.c.cJSON_CreateArray)();
            let ra = (p.r.cJSON_CreateArray)();
            for i in 0..size {
                (p.c.cJSON_AddItemToArray)(ca, (p.c.cJSON_CreateNumber)(i as f64));
                (p.r.cJSON_AddItemToArray)(ra, (p.r.cJSON_CreateNumber)(i as f64));
            }
            assert_eq!((p.c.cJSON_GetArraySize)(ca), (p.r.cJSON_GetArraySize)(ra));
            for idx in [-3i32, -1, 0, 1, 2, size - 1, size, size + 1, i32::MAX, i32::MIN] {
                let c = (p.c.cJSON_GetArrayItem)(ca, idx);
                let r = (p.r.cJSON_GetArrayItem)(ra, idx);
                assert_eq!(c.is_null(), r.is_null(), "GetArrayItem(size={size},idx={idx})");
                assert!(
                    snapshot(c) == snapshot(r),
                    "GetArrayItem(size={size},idx={idx}) value"
                );
            }
            (p.c.cJSON_Delete)(ca);
            (p.r.cJSON_Delete)(ra);
        }

        // object key lookups
        let co = (p.c.cJSON_CreateObject)();
        let ro = (p.r.cJSON_CreateObject)();
        for k in ["a", "A", "b", "", "MiXeD", "dup", "dup"] {
            let key = cs(k);
            (p.c.cJSON_AddNumberToObject)(co, key.as_ptr(), 1.0);
            (p.r.cJSON_AddNumberToObject)(ro, key.as_ptr(), 1.0);
        }
        for probe in ["a", "A", "b", "B", "", "mixed", "MIXED", "MiXeD", "dup", "missing", "\u{1}"] {
            let key = cs(probe);
            let c1 = (p.c.cJSON_GetObjectItem)(co, key.as_ptr());
            let r1 = (p.r.cJSON_GetObjectItem)(ro, key.as_ptr());
            assert_eq!(c1.is_null(), r1.is_null(), "GetObjectItem({probe:?})");
            assert!(snapshot(c1) == snapshot(r1), "GetObjectItem({probe:?}) value");
            let c2 = (p.c.cJSON_GetObjectItemCaseSensitive)(co, key.as_ptr());
            let r2 = (p.r.cJSON_GetObjectItemCaseSensitive)(ro, key.as_ptr());
            assert_eq!(c2.is_null(), r2.is_null(), "GetObjectItemCS({probe:?})");
            assert!(snapshot(c2) == snapshot(r2), "GetObjectItemCS({probe:?}) value");
            assert_eq!(
                (p.c.cJSON_HasObjectItem)(co, key.as_ptr()),
                (p.r.cJSON_HasObjectItem)(ro, key.as_ptr()),
                "HasObjectItem({probe:?})"
            );
        }
        (p.c.cJSON_Delete)(co);
        (p.r.cJSON_Delete)(ro);

        // GetStringValue / GetNumberValue over all item shapes
        let sv = cs("s");
        let makers: [(&str, fn(&Api, *const c_char) -> *mut CJson); 8] = [
            ("null", |a, _| unsafe { (a.cJSON_CreateNull)() }),
            ("true", |a, _| unsafe { (a.cJSON_CreateTrue)() }),
            ("false", |a, _| unsafe { (a.cJSON_CreateFalse)() }),
            ("number", |a, _| unsafe { (a.cJSON_CreateNumber)(2.5) }),
            ("string", |a, s| unsafe { (a.cJSON_CreateString)(s) }),
            ("raw", |a, s| unsafe { (a.cJSON_CreateRaw)(s) }),
            ("array", |a, _| unsafe { (a.cJSON_CreateArray)() }),
            ("object", |a, _| unsafe { (a.cJSON_CreateObject)() }),
        ];
        for (name, mk) in makers {
            let c = mk(p.c, sv.as_ptr());
            let r = mk(p.r, sv.as_ptr());
            assert_eq!(
                take_cstr((p.c.cJSON_GetStringValue)(c)),
                take_cstr((p.r.cJSON_GetStringValue)(r)),
                "{name} GetStringValue"
            );
            assert_eq!(
                (p.c.cJSON_GetNumberValue)(c).to_bits(),
                (p.r.cJSON_GetNumberValue)(r).to_bits(),
                "{name} GetNumberValue"
            );
            (p.c.cJSON_Delete)(c);
            (p.r.cJSON_Delete)(r);
        }
    }
}

/* ================================================================== */
/* rows 59-68, 92: detach / delete / insert / replace                   */
/* ================================================================== */

unsafe fn fresh_array(api: &Api, size: i32) -> *mut CJson {
    let a = (api.cJSON_CreateArray)();
    for i in 0..size {
        (api.cJSON_AddItemToArray)(a, (api.cJSON_CreateNumber)(i as f64));
    }
    a
}

unsafe fn fresh_object(api: &Api, keys: &[&str]) -> *mut CJson {
    let o = (api.cJSON_CreateObject)();
    for (i, k) in keys.iter().enumerate() {
        let key = cs(k);
        (api.cJSON_AddNumberToObject)(o, key.as_ptr(), i as f64);
    }
    o
}

#[test]
fn rows_59_60_62_detach_and_delete_array() {
    let _g = lock();
    let p = pair();
    unsafe {
        for size in 0..7i32 {
            for which in [-1i32, 0, 1, size / 2, size - 1, size, size + 1, i32::MAX] {
                // DetachItemFromArray
                let ca = fresh_array(p.c, size);
                let ra = fresh_array(p.r, size);
                let cd = (p.c.cJSON_DetachItemFromArray)(ca, which);
                let rd = (p.r.cJSON_DetachItemFromArray)(ra, which);
                assert_eq!(cd.is_null(), rd.is_null(), "Detach(size={size},which={which})");
                assert!(snapshot(cd) == snapshot(rd), "Detach(size={size},which={which}) item");
                both_prints(p, ca, ra, &format!("after detach(size={size},which={which})"));
                // re-append the detached item and print again (link bookkeeping)
                if !cd.is_null() {
                    assert_eq!(
                        (p.c.cJSON_AddItemToArray)(ca, cd),
                        (p.r.cJSON_AddItemToArray)(ra, rd),
                        "re-append return"
                    );
                    both_prints(p, ca, ra, &format!("after re-append(size={size},which={which})"));
                }
                (p.c.cJSON_Delete)(ca);
                (p.r.cJSON_Delete)(ra);

                // DeleteItemFromArray
                let ca = fresh_array(p.c, size);
                let ra = fresh_array(p.r, size);
                (p.c.cJSON_DeleteItemFromArray)(ca, which);
                (p.r.cJSON_DeleteItemFromArray)(ra, which);
                both_prints(p, ca, ra, &format!("after DeleteItemFromArray(size={size},which={which})"));
                (p.c.cJSON_Delete)(ca);
                (p.r.cJSON_Delete)(ra);
            }

            // DetachItemViaPointer for every position
            for pos in 0..size {
                let ca = fresh_array(p.c, size);
                let ra = fresh_array(p.r, size);
                let ci = (p.c.cJSON_GetArrayItem)(ca, pos);
                let ri = (p.r.cJSON_GetArrayItem)(ra, pos);
                let cd = (p.c.cJSON_DetachItemViaPointer)(ca, ci);
                let rd = (p.r.cJSON_DetachItemViaPointer)(ra, ri);
                assert_eq!(cd.is_null(), rd.is_null(), "ViaPointer(size={size},pos={pos})");
                both_prints(p, ca, ra, &format!("after ViaPointer(size={size},pos={pos})"));
                (p.c.cJSON_Delete)(cd);
                (p.r.cJSON_Delete)(rd);
                (p.c.cJSON_Delete)(ca);
                (p.r.cJSON_Delete)(ra);
            }
        }
    }
}

#[test]
fn rows_61_63_detach_and_delete_object() {
    let _g = lock();
    let p = pair();
    let keys = ["a", "B", "c", "", "D"];
    unsafe {
        for probe in ["a", "A", "b", "B", "c", "C", "", "D", "d", "missing"] {
            let key = cs(probe);
            for cse in [false, true] {
                let co = fresh_object(p.c, &keys);
                let ro = fresh_object(p.r, &keys);
                let (cd, rd) = if cse {
                    (
                        (p.c.cJSON_DetachItemFromObjectCaseSensitive)(co, key.as_ptr()),
                        (p.r.cJSON_DetachItemFromObjectCaseSensitive)(ro, key.as_ptr()),
                    )
                } else {
                    (
                        (p.c.cJSON_DetachItemFromObject)(co, key.as_ptr()),
                        (p.r.cJSON_DetachItemFromObject)(ro, key.as_ptr()),
                    )
                };
                assert_eq!(cd.is_null(), rd.is_null(), "Detach({probe:?},cs={cse})");
                assert!(snapshot(cd) == snapshot(rd), "Detach({probe:?},cs={cse}) item");
                both_prints(p, co, ro, &format!("after detach({probe:?},cs={cse})"));
                (p.c.cJSON_Delete)(cd);
                (p.r.cJSON_Delete)(rd);
                (p.c.cJSON_Delete)(co);
                (p.r.cJSON_Delete)(ro);

                let co = fresh_object(p.c, &keys);
                let ro = fresh_object(p.r, &keys);
                if cse {
                    (p.c.cJSON_DeleteItemFromObjectCaseSensitive)(co, key.as_ptr());
                    (p.r.cJSON_DeleteItemFromObjectCaseSensitive)(ro, key.as_ptr());
                } else {
                    (p.c.cJSON_DeleteItemFromObject)(co, key.as_ptr());
                    (p.r.cJSON_DeleteItemFromObject)(ro, key.as_ptr());
                }
                both_prints(p, co, ro, &format!("after DeleteItemFromObject({probe:?},cs={cse})"));
                (p.c.cJSON_Delete)(co);
                (p.r.cJSON_Delete)(ro);
            }
        }
    }
}

#[test]
fn row_64_insert_item_in_array() {
    let _g = lock();
    let p = pair();
    unsafe {
        for size in 0..7i32 {
            for which in [-1i32, 0, 1, size / 2, size - 1, size, size + 1, i32::MAX] {
                let ca = fresh_array(p.c, size);
                let ra = fresh_array(p.r, size);
                let cn = (p.c.cJSON_CreateNumber)(99.0);
                let rn = (p.r.cJSON_CreateNumber)(99.0);
                let cr = (p.c.cJSON_InsertItemInArray)(ca, which, cn);
                let rr = (p.r.cJSON_InsertItemInArray)(ra, which, rn);
                assert_eq!(cr, rr, "Insert(size={size},which={which}) return");
                both_prints(p, ca, ra, &format!("after insert(size={size},which={which})"));
                // walk both directions to validate prev/next bookkeeping
                for idx in 0..=(size + 1) {
                    let c = (p.c.cJSON_GetArrayItem)(ca, idx);
                    let r = (p.r.cJSON_GetArrayItem)(ra, idx);
                    assert_eq!(c.is_null(), r.is_null(), "post-insert idx {idx}");
                    assert!(snapshot(c) == snapshot(r), "post-insert idx {idx} value");
                }
                if cr == 0 {
                    (p.c.cJSON_Delete)(cn);
                    (p.r.cJSON_Delete)(rn);
                }
                (p.c.cJSON_Delete)(ca);
                (p.r.cJSON_Delete)(ra);
            }
        }
    }
}

#[test]
fn rows_65_66_replace_in_array() {
    let _g = lock();
    let p = pair();
    unsafe {
        for size in 0..7i32 {
            for which in [-1i32, 0, 1, size / 2, size - 1, size, size + 1] {
                let ca = fresh_array(p.c, size);
                let ra = fresh_array(p.r, size);
                let cn = (p.c.cJSON_CreateString)(cs("repl").as_ptr());
                let rn = (p.r.cJSON_CreateString)(cs("repl").as_ptr());
                let cr = (p.c.cJSON_ReplaceItemInArray)(ca, which, cn);
                let rr = (p.r.cJSON_ReplaceItemInArray)(ra, which, rn);
                assert_eq!(cr, rr, "ReplaceInArray(size={size},which={which}) return");
                both_prints(p, ca, ra, &format!("after replace(size={size},which={which})"));
                if cr == 0 {
                    (p.c.cJSON_Delete)(cn);
                    (p.r.cJSON_Delete)(rn);
                }
                (p.c.cJSON_Delete)(ca);
                (p.r.cJSON_Delete)(ra);
            }

            // ReplaceItemViaPointer, including replacement == item
            for pos in 0..size {
                let ca = fresh_array(p.c, size);
                let ra = fresh_array(p.r, size);
                let ci = (p.c.cJSON_GetArrayItem)(ca, pos);
                let ri = (p.r.cJSON_GetArrayItem)(ra, pos);
                let cn = (p.c.cJSON_CreateNumber)(-1.0);
                let rn = (p.r.cJSON_CreateNumber)(-1.0);
                assert_eq!(
                    (p.c.cJSON_ReplaceItemViaPointer)(ca, ci, cn),
                    (p.r.cJSON_ReplaceItemViaPointer)(ra, ri, rn),
                    "ViaPointer(size={size},pos={pos})"
                );
                both_prints(p, ca, ra, &format!("after ReplaceViaPointer(size={size},pos={pos})"));
                // self-replacement
                let ci2 = (p.c.cJSON_GetArrayItem)(ca, pos);
                let ri2 = (p.r.cJSON_GetArrayItem)(ra, pos);
                assert_eq!(
                    (p.c.cJSON_ReplaceItemViaPointer)(ca, ci2, ci2),
                    (p.r.cJSON_ReplaceItemViaPointer)(ra, ri2, ri2),
                    "self-replace(size={size},pos={pos})"
                );
                both_prints(p, ca, ra, &format!("after self-replace(size={size},pos={pos})"));
                (p.c.cJSON_Delete)(ca);
                (p.r.cJSON_Delete)(ra);
            }
        }
    }
}

#[test]
fn rows_67_68_replace_in_object() {
    let _g = lock();
    let p = pair();
    let keys = ["a", "B", "c", ""];
    unsafe {
        for probe in ["a", "A", "b", "B", "c", "C", "", "missing"] {
            for cse in [false, true] {
                let co = fresh_object(p.c, &keys);
                let ro = fresh_object(p.r, &keys);
                let key = cs(probe);
                let cn = (p.c.cJSON_CreateString)(cs("R").as_ptr());
                let rn = (p.r.cJSON_CreateString)(cs("R").as_ptr());
                let (cr, rr) = if cse {
                    (
                        (p.c.cJSON_ReplaceItemInObjectCaseSensitive)(co, key.as_ptr(), cn),
                        (p.r.cJSON_ReplaceItemInObjectCaseSensitive)(ro, key.as_ptr(), rn),
                    )
                } else {
                    (
                        (p.c.cJSON_ReplaceItemInObject)(co, key.as_ptr(), cn),
                        (p.r.cJSON_ReplaceItemInObject)(ro, key.as_ptr(), rn),
                    )
                };
                assert_eq!(cr, rr, "ReplaceInObject({probe:?},cs={cse}) return");
                both_prints(p, co, ro, &format!("after replace({probe:?},cs={cse})"));
                assert!(
                    snapshot(cn) == snapshot(rn),
                    "replacement item state ({probe:?},cs={cse})"
                );
                if cr == 0 {
                    (p.c.cJSON_Delete)(cn);
                    (p.r.cJSON_Delete)(rn);
                }
                (p.c.cJSON_Delete)(co);
                (p.r.cJSON_Delete)(ro);
            }
        }
    }
}

/* ================================================================== */
/* rows 69-74: Duplicate                                               */
/* ================================================================== */

#[test]
fn rows_69_73_duplicate() {
    let _g = lock();
    let p = pair();
    let mut rng = Rng::new(0x5EED_0012);
    unsafe {
        for i in 0..200 {
            let json = random_json(&mut rng, 5);
            let buf = cbytes(json.as_bytes());
            let ci = (p.c.cJSON_Parse)(buf.as_ptr());
            let ri = (p.r.cJSON_Parse)(buf.as_ptr());
            if ci.is_null() {
                (p.c.cJSON_Delete)(ci);
                (p.r.cJSON_Delete)(ri);
                continue;
            }
            for &rec in &[0i32, 1, 2, -1] {
                let cd = (p.c.cJSON_Duplicate)(ci, rec);
                let rd = (p.r.cJSON_Duplicate)(ri, rec);
                assert_eq!(cd.is_null(), rd.is_null(), "dup#{i} rec={rec}");
                assert!(snapshot(cd) == snapshot(rd), "dup#{i} rec={rec} tree");
                both_prints(p, cd, rd, &format!("dup#{i} rec={rec}"));
                (p.c.cJSON_Delete)(cd);
                (p.r.cJSON_Delete)(rd);
            }
            (p.c.cJSON_Delete)(ci);
            (p.r.cJSON_Delete)(ri);
        }

        // reference + const-key items: the IsReference bit must be cleared and
        // a const key must be aliased rather than copied.
        for kind in 0..3 {
            let cparent = (p.c.cJSON_CreateObject)();
            let rparent = (p.r.cJSON_CreateObject)();
            let key = cs("k");
            match kind {
                0 => {
                    let sref = cs("shared");
                    (p.c.cJSON_AddItemToObject)(
                        cparent, key.as_ptr(), (p.c.cJSON_CreateStringReference)(sref.as_ptr()));
                    (p.r.cJSON_AddItemToObject)(
                        rparent, key.as_ptr(), (p.r.cJSON_CreateStringReference)(sref.as_ptr()));
                }
                1 => {
                    (p.c.cJSON_AddItemToObjectCS)(
                        cparent, key.as_ptr(), (p.c.cJSON_CreateNumber)(1.0));
                    (p.r.cJSON_AddItemToObjectCS)(
                        rparent, key.as_ptr(), (p.r.cJSON_CreateNumber)(1.0));
                }
                _ => {
                    let csub = fresh_array(p.c, 3);
                    let rsub = fresh_array(p.r, 3);
                    (p.c.cJSON_AddItemReferenceToObject)(cparent, key.as_ptr(), csub);
                    (p.r.cJSON_AddItemReferenceToObject)(rparent, key.as_ptr(), rsub);
                    // keep subtrees alive: they are owned separately
                    let _keep = (csub, rsub);
                }
            }
            for &rec in &[0i32, 1] {
                let cd = (p.c.cJSON_Duplicate)(cparent, rec);
                let rd = (p.r.cJSON_Duplicate)(rparent, rec);
                assert!(snapshot(cd) == snapshot(rd), "dup kind={kind} rec={rec}");
                both_prints(p, cd, rd, &format!("dup kind={kind} rec={rec}"));
                (p.c.cJSON_Delete)(cd);
                (p.r.cJSON_Delete)(rd);
            }
            (p.c.cJSON_Delete)(cparent);
            (p.r.cJSON_Delete)(rparent);
        }
    }
}

#[test]
fn row_74_duplicate_circular_limit() {
    // CJSON_CIRCULAR_LIMIT is 10000; duplicating / deleting a tree that deep
    // recurses 10000 frames in both libraries, so run on a big stack.
    std::thread::Builder::new()
        .stack_size(512 * 1024 * 1024)
        .spawn(row_74_body)
        .unwrap()
        .join()
        .unwrap();
}

fn row_74_body() {
    let _g = lock();
    let p = pair();
    unsafe {
        // cJSON_Duplicate_rec's `depth` counter increments once per nesting
        // level, so a tree nested >= CJSON_CIRCULAR_LIMIT (10000) deep must fail.
        for &depth in &[10usize, 9998, 9999, 10000, 10001, 10002] {
            let cbuild = |api: &Api| -> *mut CJson {
                let root = (api.cJSON_CreateArray)();
                let mut cur = root;
                for _ in 1..depth {
                    let next = (api.cJSON_CreateArray)();
                    (api.cJSON_AddItemToArray)(cur, next);
                    cur = next;
                }
                root
            };
            let cr = cbuild(p.c);
            let rr = cbuild(p.r);
            let cd = (p.c.cJSON_Duplicate)(cr, 1);
            let rd = (p.r.cJSON_Duplicate)(rr, 1);
            assert_eq!(
                cd.is_null(),
                rd.is_null(),
                "Duplicate(recurse=1) at nesting depth {depth}: C null={}, R null={}",
                cd.is_null(),
                rd.is_null()
            );
            (p.c.cJSON_Delete)(cd);
            (p.r.cJSON_Delete)(rd);
            (p.c.cJSON_Delete)(cr);
            (p.r.cJSON_Delete)(rr);
        }
    }
}

/* ================================================================== */
/* rows 75-78: Compare                                                 */
/* ================================================================== */

#[test]
fn rows_75_78_compare() {
    let _g = lock();
    let p = pair();
    let docs: &[&str] = &[
        "null", "true", "false", "0", "1", "1.0", "1.0000000000000002", "-0",
        "\"\"", "\"a\"", "\"A\"", "[]", "[1]", "[1,2]", "[2,1]", "[1,2,3]",
        "{}", "{\"a\":1}", "{\"A\":1}", "{\"a\":2}", "{\"a\":1,\"b\":2}",
        "{\"b\":2,\"a\":1}", "{\"a\":{\"b\":[1,2]}}", "{\"a\":{\"b\":[1,3]}}",
        "1e308", "1e-308", "5e-324",
    ];
    unsafe {
        let cn: Vec<*mut CJson> = docs
            .iter()
            .map(|d| (p.c.cJSON_Parse)(cbytes(d.as_bytes()).as_ptr()))
            .collect();
        let rn: Vec<*mut CJson> = docs
            .iter()
            .map(|d| (p.r.cJSON_Parse)(cbytes(d.as_bytes()).as_ptr()))
            .collect();
        for i in 0..docs.len() {
            for j in 0..docs.len() {
                for &cse in &[0i32, 1, 2, -1] {
                    assert_eq!(
                        (p.c.cJSON_Compare)(cn[i], cn[j], cse),
                        (p.r.cJSON_Compare)(rn[i], rn[j], cse),
                        "Compare({:?},{:?},{cse})",
                        docs[i],
                        docs[j]
                    );
                }
            }
        }
        // numbers straddling the DBL_EPSILON threshold used by compare_double
        for &(a, b) in &[
            (1.0f64, 1.0 + f64::EPSILON),
            (1.0, 1.0 + 2.0 * f64::EPSILON),
            (1.0, 1.0 - f64::EPSILON / 2.0),
            (0.0, 0.0),
            (0.0, -0.0),
            (0.0, f64::MIN_POSITIVE),
            (1e300, 1e300 * (1.0 + f64::EPSILON)),
            (f64::NAN, f64::NAN),
            (f64::INFINITY, f64::INFINITY),
            (f64::INFINITY, f64::NEG_INFINITY),
        ] {
            let ca = (p.c.cJSON_CreateNumber)(a);
            let cb = (p.c.cJSON_CreateNumber)(b);
            let ra = (p.r.cJSON_CreateNumber)(a);
            let rb = (p.r.cJSON_CreateNumber)(b);
            for &cse in &[0i32, 1] {
                assert_eq!(
                    (p.c.cJSON_Compare)(ca, cb, cse),
                    (p.r.cJSON_Compare)(ra, rb, cse),
                    "Compare({a:?},{b:?},{cse})"
                );
            }
            (p.c.cJSON_Delete)(ca);
            (p.c.cJSON_Delete)(cb);
            (p.r.cJSON_Delete)(ra);
            (p.r.cJSON_Delete)(rb);
        }
        for i in 0..docs.len() {
            (p.c.cJSON_Delete)(cn[i]);
            (p.r.cJSON_Delete)(rn[i]);
        }
    }
}

/* ================================================================== */
/* rows 79-80: Minify                                                 */
/* ================================================================== */

unsafe fn minify_both(p: &Pair, input: &[u8], label: &str) {
    let mut cbuf = cbytes(input);
    let mut rbuf = cbytes(input);
    (p.c.cJSON_Minify)(cbuf.as_mut_ptr());
    (p.r.cJSON_Minify)(rbuf.as_mut_ptr());
    // compare the whole scratch buffer, not just up to the NUL: cJSON_Minify
    // rewrites in place and the trailing bytes are part of the observable state.
    assert!(
        cbuf == rbuf,
        "[{label}] cJSON_Minify differs for {:?}\n C: {:?}\n R: {:?}",
        String::from_utf8_lossy(input),
        String::from_utf8_lossy(&cbuf.iter().map(|&x| x as u8).collect::<Vec<_>>()),
        String::from_utf8_lossy(&rbuf.iter().map(|&x| x as u8).collect::<Vec<_>>())
    );
}

#[test]
fn row_79_minify_edge_cases() {
    let _g = lock();
    let p = pair();
    let cases: &[&str] = &[
        "",
        " ",
        "\t\r\n ",
        "{}",
        "{ }",
        "{\n\t\"a\" : 1\n}",
        "[1, 2,\t3]",
        "// comment\n1",
        "// comment without newline",
        "1 // trailing\n2",
        "/* block */1",
        "/* unterminated",
        "/*/",
        "/**/",
        "/* a */ /* b */ 1",
        "/",
        "//",
        "1/2",
        "\"string with // not a comment\"",
        "\"string with /* not a comment */\"",
        "\"escaped \\\" quote\"",
        "\"escaped backslash \\\\\"",
        "\"trailing backslash \\",
        "\"unterminated",
        "\"\"",
        "[\"a\", \"b\"]",
        "{\"a\":\"//\"}",
        "{\"a\":\"/*\"}",
        "\t{\t\"k\"\t:\t[\t1\t,\t2\t]\t}\t",
        "[\"\\\\\\\"\"]",
        "\"\\\\\"",
        "\"\\\"\"",
        "a\\\"b",
    ];
    unsafe {
        for (i, c) in cases.iter().enumerate() {
            minify_both(p, c.as_bytes(), &format!("minify#{i}"));
        }
        // raw byte patterns, incl. lone control characters
        for b in [0u8 + 1, 2, 11, 12, 31, 32, 127, 128, 255] {
            minify_both(p, &[b'[', b, b']'], &format!("minify-byte#{b}"));
        }
    }
}

#[test]
fn row_80_minify_randomized() {
    let _g = lock();
    let p = pair();
    let mut rng = Rng::new(0x5EED_0013);
    let comments = ["", "// c\n", "/*c*/", "/**/", "/* multi\nline */", "//\n"];
    unsafe {
        for i in 0..400 {
            let json = random_json(&mut rng, 4);
            let ws = sprinkle_ws(&mut rng, &json);
            let mut with_comments = String::new();
            for (n, ch) in ws.chars().enumerate() {
                if n % 7 == 0 {
                    with_comments.push_str(comments[rng.below(comments.len())]);
                }
                with_comments.push(ch);
            }
            minify_both(p, ws.as_bytes(), &format!("minify-rand#{i}"));
            minify_both(p, with_comments.as_bytes(), &format!("minify-cmt#{i}"));
        }
    }
}

/* ================================================================== */
/* row 81: SetValuestring                                             */
/* ================================================================== */

#[test]
fn row_81_set_valuestring() {
    let _g = lock();
    let p = pair();
    unsafe {
        let originals = ["", "a", "abc", "abcdefghij", "long original value here"];
        let news = ["", "a", "ab", "abc", "abcdefghij", "much longer replacement value"];
        for orig in originals {
            for new in news {
                for kind in 0..3 {
                    let ov = cs(orig);
                    let (c, r) = match kind {
                        0 => (
                            (p.c.cJSON_CreateString)(ov.as_ptr()),
                            (p.r.cJSON_CreateString)(ov.as_ptr()),
                        ),
                        1 => (
                            (p.c.cJSON_CreateRaw)(ov.as_ptr()),
                            (p.r.cJSON_CreateRaw)(ov.as_ptr()),
                        ),
                        _ => (
                            (p.c.cJSON_CreateStringReference)(ov.as_ptr()),
                            (p.r.cJSON_CreateStringReference)(ov.as_ptr()),
                        ),
                    };
                    let nv = cs(new);
                    let cres = (p.c.cJSON_SetValuestring)(c, nv.as_ptr());
                    let rres = (p.r.cJSON_SetValuestring)(r, nv.as_ptr());
                    assert_eq!(
                        cres.is_null(),
                        rres.is_null(),
                        "SetValuestring kind={kind} {orig:?}->{new:?} nullness"
                    );
                    assert_eq!(
                        take_cstr(cres),
                        take_cstr(rres),
                        "SetValuestring kind={kind} {orig:?}->{new:?} returned text"
                    );
                    assert!(
                        snapshot(c) == snapshot(r),
                        "SetValuestring kind={kind} {orig:?}->{new:?} item state"
                    );
                    if kind != 2 {
                        both_prints(p, c, r, &format!("SetValuestring kind={kind}"));
                    }
                    if kind == 2 {
                        // reference item: don't let Delete free the borrowed CString
                        (*c).type_ = cJSON_String | cJSON_IsReference;
                        (*r).type_ = cJSON_String | cJSON_IsReference;
                    }
                    (p.c.cJSON_Delete)(c);
                    (p.r.cJSON_Delete)(r);
                }
            }
        }
        // self-assignment / overlapping buffers
        for orig in ["abcdef", "x"] {
            let ov = cs(orig);
            let c = (p.c.cJSON_CreateString)(ov.as_ptr());
            let r = (p.r.cJSON_CreateString)(ov.as_ptr());
            let cres = (p.c.cJSON_SetValuestring)(c, (*c).valuestring);
            let rres = (p.r.cJSON_SetValuestring)(r, (*r).valuestring);
            assert_eq!(cres.is_null(), rres.is_null(), "self SetValuestring {orig:?}");
            assert!(snapshot(c) == snapshot(r), "self SetValuestring {orig:?} state");
            // overlapping: point at the middle of our own buffer (shorter suffix)
            let cres2 = (p.c.cJSON_SetValuestring)(c, (*c).valuestring.add(1));
            let rres2 = (p.r.cJSON_SetValuestring)(r, (*r).valuestring.add(1));
            assert_eq!(cres2.is_null(), rres2.is_null(), "overlap SetValuestring {orig:?}");
            assert!(snapshot(c) == snapshot(r), "overlap SetValuestring {orig:?} state");
            (p.c.cJSON_Delete)(c);
            (p.r.cJSON_Delete)(r);
        }
    }
}

/* ================================================================== */
/* rows 91-92: randomized composed pipeline                            */
/* ================================================================== */

#[test]
fn rows_91_92_composed_pipeline() {
    let _g = lock();
    let p = pair();
    let mut rng = Rng::new(0x5EED_0014);
    unsafe {
        for iter in 0..250 {
            let json = random_json(&mut rng, 4);
            let buf = cbytes(json.as_bytes());
            let croot = (p.c.cJSON_Parse)(buf.as_ptr());
            let rroot = (p.r.cJSON_Parse)(buf.as_ptr());
            if croot.is_null() {
                (p.c.cJSON_Delete)(croot);
                (p.r.cJSON_Delete)(rroot);
                continue;
            }
            for step in 0..12 {
                let op = rng.below(9);
                let idx = rng.i32() % 6;
                let keyname = cs(&format!("k{}", rng.below(4)));
                match op {
                    0 => {
                        let cn = (p.c.cJSON_CreateNumber)(step as f64);
                        let rn = (p.r.cJSON_CreateNumber)(step as f64);
                        assert_eq!(
                            (p.c.cJSON_AddItemToArray)(croot, cn),
                            (p.r.cJSON_AddItemToArray)(rroot, rn),
                            "iter{iter} step{step} AddItemToArray"
                        );
                    }
                    1 => {
                        let cn = (p.c.cJSON_CreateString)(cs("s").as_ptr());
                        let rn = (p.r.cJSON_CreateString)(cs("s").as_ptr());
                        assert_eq!(
                            (p.c.cJSON_AddItemToObject)(croot, keyname.as_ptr(), cn),
                            (p.r.cJSON_AddItemToObject)(rroot, keyname.as_ptr(), rn),
                            "iter{iter} step{step} AddItemToObject"
                        );
                    }
                    2 => {
                        let cn = (p.c.cJSON_CreateNumber)(-(step as f64));
                        let rn = (p.r.cJSON_CreateNumber)(-(step as f64));
                        assert_eq!(
                            (p.c.cJSON_InsertItemInArray)(croot, idx, cn),
                            (p.r.cJSON_InsertItemInArray)(rroot, idx, rn),
                            "iter{iter} step{step} Insert"
                        );
                    }
                    3 => {
                        let cd = (p.c.cJSON_DetachItemFromArray)(croot, idx);
                        let rd = (p.r.cJSON_DetachItemFromArray)(rroot, idx);
                        assert_eq!(cd.is_null(), rd.is_null(), "iter{iter} step{step} Detach");
                        assert!(snapshot(cd) == snapshot(rd), "iter{iter} step{step} Detach item");
                        (p.c.cJSON_Delete)(cd);
                        (p.r.cJSON_Delete)(rd);
                    }
                    4 => {
                        (p.c.cJSON_DeleteItemFromArray)(croot, idx);
                        (p.r.cJSON_DeleteItemFromArray)(rroot, idx);
                    }
                    5 => {
                        let cn = (p.c.cJSON_CreateBool)(step as c_int);
                        let rn = (p.r.cJSON_CreateBool)(step as c_int);
                        let cr = (p.c.cJSON_ReplaceItemInArray)(croot, idx, cn);
                        let rr = (p.r.cJSON_ReplaceItemInArray)(rroot, idx, rn);
                        assert_eq!(cr, rr, "iter{iter} step{step} ReplaceInArray");
                        if cr == 0 {
                            (p.c.cJSON_Delete)(cn);
                            (p.r.cJSON_Delete)(rn);
                        }
                    }
                    6 => {
                        let cn = (p.c.cJSON_CreateNull)();
                        let rn = (p.r.cJSON_CreateNull)();
                        let cr = (p.c.cJSON_ReplaceItemInObject)(croot, keyname.as_ptr(), cn);
                        let rr = (p.r.cJSON_ReplaceItemInObject)(rroot, keyname.as_ptr(), rn);
                        assert_eq!(cr, rr, "iter{iter} step{step} ReplaceInObject");
                        if cr == 0 {
                            (p.c.cJSON_Delete)(cn);
                            (p.r.cJSON_Delete)(rn);
                        }
                    }
                    7 => {
                        (p.c.cJSON_DeleteItemFromObjectCaseSensitive)(croot, keyname.as_ptr());
                        (p.r.cJSON_DeleteItemFromObjectCaseSensitive)(rroot, keyname.as_ptr());
                    }
                    _ => {
                        let cd = (p.c.cJSON_Duplicate)(croot, 1);
                        let rd = (p.r.cJSON_Duplicate)(rroot, 1);
                        assert!(snapshot(cd) == snapshot(rd), "iter{iter} step{step} Duplicate");
                        for &cse in &[0i32, 1] {
                            assert_eq!(
                                (p.c.cJSON_Compare)(croot, cd, cse),
                                (p.r.cJSON_Compare)(rroot, rd, cse),
                                "iter{iter} step{step} Compare"
                            );
                        }
                        (p.c.cJSON_Delete)(cd);
                        (p.r.cJSON_Delete)(rd);
                    }
                }
                both_prints(p, croot, rroot, &format!("iter{iter} step{step} op{op}"));
            }
            (p.c.cJSON_Delete)(croot);
            (p.r.cJSON_Delete)(rroot);
        }
    }
}
