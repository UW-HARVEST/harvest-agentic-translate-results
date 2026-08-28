//! Phase B — CONFIGS.md rows 1–25: constructors, accessors and the
//! object/array building API, driven through the exported symbols of both
//! `.so`s.
#![allow(non_snake_case)]

mod harness;

use harness::*;
use std::ffi::{c_int, c_void};

// ---------------------------------------------------------------------------
// row 1 — cJSON_Version
// ---------------------------------------------------------------------------
#[test]
fn cfg01_version() {
    let (c, r) = both();
    unsafe {
        // Called repeatedly: the C version re-sprintf()s into a static buffer
        // every time, so the returned pointer must stay stable and the contents
        // identical.
        for _ in 0..5 {
            let a = (c.cJSON_Version)();
            let b = (r.cJSON_Version)();
            assert_eq!(cstr(a), cstr(b));
        }
        assert_eq!((c.cJSON_Version)(), (c.cJSON_Version)());
        assert_eq!((r.cJSON_Version)(), (r.cJSON_Version)());
    }
}

// ---------------------------------------------------------------------------
// row 2 — cJSON_malloc / cJSON_free
// ---------------------------------------------------------------------------
#[test]
fn cfg02_malloc_free() {
    let (c, r) = both();
    unsafe {
        for size in [0usize, 1, 8, 64, 4096, 1 << 20] {
            let pc = (c.cJSON_malloc)(size);
            let pr = (r.cJSON_malloc)(size);
            assert_eq!(
                pc.is_null(),
                pr.is_null(),
                "cJSON_malloc({size}) nullness differs"
            );
            if !pc.is_null() {
                // writable for `size` bytes
                std::ptr::write_bytes(pc as *mut u8, 0x5A, size);
                std::ptr::write_bytes(pr as *mut u8, 0x5A, size);
            }
            (c.cJSON_free)(pc);
            (r.cJSON_free)(pr);
        }
        // cJSON_free(NULL) must be a no-op on both sides
        (c.cJSON_free)(std::ptr::null_mut::<c_void>());
        (r.cJSON_free)(std::ptr::null_mut::<c_void>());
    }
}

// ---------------------------------------------------------------------------
// rows 3, 4, 9 — trivial constructors
// ---------------------------------------------------------------------------
#[test]
fn cfg03_04_09_trivial_constructors() {
    let (c, r) = both();
    for spec in [Spec::Null, Spec::True, Spec::False] {
        assert_spec_matches(&c, &r, &spec, "trivial constructor");
    }
    for spec in [
        Spec::Arr(vec![]),
        Spec::Obj(vec![]),
        Spec::ObjCS(vec![]),
        Spec::ObjViaHelpers(vec![]),
    ] {
        assert_spec_matches(&c, &r, &spec, "empty container");
    }
    // row 4: cJSON_CreateBool with in-range AND out-of-range ints
    for b in [0, 1, 2, -1, 3, i32::MIN, i32::MAX, 0x10000, -0x10000] {
        assert_spec_matches(&c, &r, &Spec::Bool(b), &format!("CreateBool({b})"));
    }
}

// ---------------------------------------------------------------------------
// row 5 — cJSON_CreateNumber over the whole double space
// ---------------------------------------------------------------------------
#[test]
fn cfg05_create_number() {
    let (c, r) = both();
    for d in number_pool() {
        assert_spec_matches(&c, &r, &Spec::Num(d), &format!("CreateNumber({d:?})"));
    }
    let mut rng = Rng::new(0x0505_0505);
    for i in 0..4000 {
        let d = if i % 3 == 0 { rng.any_f64() } else { rng.json_f64() };
        assert_spec_matches(
            &c,
            &r,
            &Spec::Num(d),
            &format!("CreateNumber(random #{i} = {:#018x})", d.to_bits()),
        );
    }
}

// ---------------------------------------------------------------------------
// rows 6, 7, 8 — string-ish constructors
// ---------------------------------------------------------------------------
#[test]
fn cfg06_07_08_string_constructors() {
    let (c, r) = both();
    for s in string_pool() {
        for spec in [
            Spec::Str(s.clone()),
            Spec::Raw(s.clone()),
            Spec::StrRef(s.clone()),
        ] {
            assert_spec_matches(
                &c,
                &r,
                &spec,
                &format!("string constructor {:?}", String::from_utf8_lossy(&s)),
            );
        }
    }
    let mut rng = Rng::new(0x0607_0800);
    let pool = string_pool();
    for i in 0..600 {
        let s = rand_string(&mut rng, &pool);
        let spec = match i % 3 {
            0 => Spec::Str(s),
            1 => Spec::Raw(s),
            _ => Spec::StrRef(s),
        };
        assert_spec_matches(&c, &r, &spec, &format!("random string #{i}"));
    }
}

// ---------------------------------------------------------------------------
// row 10 — cJSON_CreateObjectReference / cJSON_CreateArrayReference
// ---------------------------------------------------------------------------
#[test]
fn cfg10_container_references() {
    let (c, r) = both();
    let inner_specs = [
        Spec::Arr(vec![Spec::Arr(vec![Spec::Num(1.0), Spec::Num(2.0)])]),
        Spec::Arr(vec![Spec::Obj(vec![
            (b"a".to_vec(), Spec::True),
            (b"b".to_vec(), Spec::Str(b"x".to_vec())),
        ])]),
        Spec::Arr(vec![Spec::Arr(vec![])]),
    ];
    for inner in inner_specs {
        for as_object in [false, true] {
            assert_spec_matches(
                &c,
                &r,
                &Spec::ContainerRef(Box::new(inner.clone()), as_object),
                &format!("ContainerRef(as_object={as_object})"),
            );
        }
    }
}

// ---------------------------------------------------------------------------
// rows 11, 12, 13, 14 — cJSON_Create{Int,Float,Double,String}Array
// ---------------------------------------------------------------------------
#[test]
fn cfg11_14_typed_arrays() {
    let (c, r) = both();
    let mut rng = Rng::new(0x1114_1114);
    let pool = string_pool();

    for count in [0usize, 1, 2, 3, 4, 17, 64, 257] {
        // ints, incl. the saturation boundaries
        let mut ints: Vec<c_int> = (0..count).map(|_| rng.range_i32(i32::MIN, i32::MAX)).collect();
        if count >= 4 {
            ints[0] = i32::MIN;
            ints[1] = i32::MAX;
            ints[2] = 0;
            ints[3] = -1;
        }
        assert_spec_matches(&c, &r, &Spec::IntArr(ints), &format!("IntArr({count})"));

        // floats — note the float→double widening in cJSON_CreateFloatArray
        let mut floats: Vec<f32> = (0..count)
            .map(|_| f32::from_bits(rng.next_u64() as u32))
            .collect();
        if count >= 6 {
            floats[0] = 0.0;
            floats[1] = -0.0;
            floats[2] = f32::INFINITY;
            floats[3] = f32::NEG_INFINITY;
            floats[4] = f32::NAN;
            floats[5] = f32::MIN_POSITIVE / 3.0; // denormal
        }
        assert_spec_matches(&c, &r, &Spec::FloatArr(floats), &format!("FloatArr({count})"));

        let doubles: Vec<f64> = (0..count).map(|_| rng.json_f64()).collect();
        assert_spec_matches(
            &c,
            &r,
            &Spec::DoubleArr(doubles),
            &format!("DoubleArr({count})"),
        );

        let strings: Vec<Vec<u8>> = (0..count).map(|_| rand_string(&mut rng, &pool)).collect();
        assert_spec_matches(&c, &r, &Spec::StrArr(strings), &format!("StrArr({count})"));
    }
}

// ---------------------------------------------------------------------------
// row 15 — cJSON_GetArraySize / cJSON_GetArrayItem
// ---------------------------------------------------------------------------
#[test]
fn cfg15_get_array_size_and_item() {
    let (c, r) = both();
    let mut rng = Rng::new(0x1515_1515);

    for size in [0usize, 1, 2, 5, 64] {
        let spec = Spec::Arr(
            (0..size)
                .map(|i| Spec::Num(i as f64 * 1.5))
                .collect::<Vec<_>>(),
        );
        unsafe {
            let bc = build(&c, &spec);
            let br = build(&r, &spec);
            assert_eq!(
                (c.cJSON_GetArraySize)(bc.root),
                (r.cJSON_GetArraySize)(br.root),
                "GetArraySize(size={size})"
            );
            let mut indices: Vec<c_int> = vec![
                0,
                (size as c_int) / 2,
                size as c_int - 1,
                size as c_int,
                size as c_int + 1,
                -1,
                i32::MIN,
                i32::MAX,
            ];
            for _ in 0..10 {
                indices.push(rng.range_i32(-4, size as i32 + 4));
            }
            for idx in indices {
                let ic = (c.cJSON_GetArrayItem)(bc.root, idx);
                let ir = (r.cJSON_GetArrayItem)(br.root, idx);
                assert_eq!(
                    ic.is_null(),
                    ir.is_null(),
                    "GetArrayItem(size={size}, index={idx}) nullness differs"
                );
                assert_eq!(
                    snap(ic),
                    snap(ir),
                    "GetArrayItem(size={size}, index={idx}) content differs"
                );
            }
            bc.delete();
            br.delete();
        }
    }

    // non-container roots: index lookup must return NULL identically
    for spec in [
        Spec::Null,
        Spec::True,
        Spec::Num(1.0),
        Spec::Str(b"abc".to_vec()),
        Spec::Raw(b"1".to_vec()),
    ] {
        unsafe {
            let bc = build(&c, &spec);
            let br = build(&r, &spec);
            assert_eq!(
                (c.cJSON_GetArraySize)(bc.root),
                (r.cJSON_GetArraySize)(br.root)
            );
            for idx in [-1, 0, 1, 100] {
                assert_eq!(
                    (c.cJSON_GetArrayItem)(bc.root, idx).is_null(),
                    (r.cJSON_GetArrayItem)(br.root, idx).is_null(),
                    "GetArrayItem on non-container, index={idx}"
                );
            }
            bc.delete();
            br.delete();
        }
    }
}

// ---------------------------------------------------------------------------
// rows 16, 17, 18 — object lookup, case sensitive and insensitive
// ---------------------------------------------------------------------------
#[test]
fn cfg16_18_object_lookup() {
    let (c, r) = both();

    let objects = [
        Spec::Obj(vec![]),
        Spec::Obj(vec![(b"key".to_vec(), Spec::Num(1.0))]),
        Spec::Obj(vec![
            (b"Key".to_vec(), Spec::Num(1.0)),
            (b"KEY".to_vec(), Spec::Num(2.0)),
            (b"key".to_vec(), Spec::Num(3.0)),
            (b"kEy".to_vec(), Spec::Num(4.0)),
        ]),
        Spec::Obj(vec![
            (b"a".to_vec(), Spec::Null),
            (b"a".to_vec(), Spec::True),
            (b"b".to_vec(), Spec::False),
            (b"".to_vec(), Spec::Str(b"empty key".to_vec())),
        ]),
        Spec::Obj(vec![
            (b"\xc3\xa9".to_vec(), Spec::Num(1.0)),
            (b"\xc3\x89".to_vec(), Spec::Num(2.0)),
            (b"tab\there".to_vec(), Spec::Num(3.0)),
        ]),
        // arrays: children have string == NULL, which the C walk treats specially
        Spec::Arr(vec![Spec::Num(1.0), Spec::Num(2.0)]),
        Spec::ObjCS(vec![
            (b"cs".to_vec(), Spec::Num(9.0)),
            (b"CS".to_vec(), Spec::Num(8.0)),
        ]),
    ];
    let keys: Vec<Vec<u8>> = vec![
        b"key".to_vec(),
        b"Key".to_vec(),
        b"KEY".to_vec(),
        b"kEy".to_vec(),
        b"a".to_vec(),
        b"A".to_vec(),
        b"b".to_vec(),
        b"".to_vec(),
        b"missing".to_vec(),
        b"\xc3\xa9".to_vec(),
        b"\xc3\x89".to_vec(),
        b"tab\there".to_vec(),
        b"cs".to_vec(),
        b"CS".to_vec(),
        b"\x80".to_vec(),
        b"\xff".to_vec(),
        b"KEY\x00extra".to_vec(), // CString would reject NUL; keep as literal bytes
    ];

    for (oi, spec) in objects.iter().enumerate() {
        unsafe {
            let bc = build(&c, spec);
            let br = build(&r, spec);
            for key in &keys {
                if key.contains(&0) {
                    continue;
                }
                let kb = Bytes::new(key);
                let label = format!("obj#{oi} key {:?}", String::from_utf8_lossy(key));
                let a = (c.cJSON_GetObjectItem)(bc.root, kb.as_ptr());
                let b = (r.cJSON_GetObjectItem)(br.root, kb.as_ptr());
                assert_eq!(snap(a), snap(b), "GetObjectItem {label}");
                assert_eq!(a.is_null(), b.is_null(), "GetObjectItem nullness {label}");

                let a = (c.cJSON_GetObjectItemCaseSensitive)(bc.root, kb.as_ptr());
                let b = (r.cJSON_GetObjectItemCaseSensitive)(br.root, kb.as_ptr());
                assert_eq!(snap(a), snap(b), "GetObjectItemCaseSensitive {label}");
                assert_eq!(a.is_null(), b.is_null(), "…CaseSensitive nullness {label}");

                assert_eq!(
                    (c.cJSON_HasObjectItem)(bc.root, kb.as_ptr()),
                    (r.cJSON_HasObjectItem)(br.root, kb.as_ptr()),
                    "HasObjectItem {label}"
                );
            }
            bc.delete();
            br.delete();
        }
    }
}

// ---------------------------------------------------------------------------
// rows 19, 20 — value accessors and the 10 type predicates over every type
// ---------------------------------------------------------------------------
#[test]
fn cfg19_20_accessors_and_predicates() {
    let (c, r) = both();
    let specs = [
        Spec::Null,
        Spec::True,
        Spec::False,
        Spec::Num(42.5),
        Spec::Num(f64::NAN),
        Spec::Str(b"str".to_vec()),
        Spec::StrRef(b"ref".to_vec()),
        Spec::Raw(b"[1,2]".to_vec()),
        Spec::Arr(vec![Spec::Num(1.0)]),
        Spec::Obj(vec![(b"k".to_vec(), Spec::Num(1.0))]),
        Spec::ObjCS(vec![(b"k".to_vec(), Spec::Num(1.0))]),
    ];
    for spec in &specs {
        // assert_spec_matches already compares all 10 predicates,
        // GetStringValue and GetNumberValue for the root.
        assert_spec_matches(&c, &r, spec, "accessors/predicates");
    }

    // Predicates on caller-fabricated `type` values (a C `int` can hold
    // anything, including combinations no constructor produces).
    unsafe {
        for t in [
            0i32, 1, 2, 3, 4, 5, 8, 16, 32, 64, 128, 255, 256, 512, 768, 0x0A, 0x18, 0x88, 0xFF,
            0x100 | 0x10, 0x200 | 0x40, -1, i32::MIN, i32::MAX,
        ] {
            let nc = (c.cJSON_CreateNull)();
            let nr = (r.cJSON_CreateNull)();
            (*nc).type_ = t;
            (*nr).type_ = t;
            (*nc).valuedouble = 1.25;
            (*nr).valuedouble = 1.25;
            (*nc).valueint = 7;
            (*nr).valueint = 7;
            let pc = [
                (c.cJSON_IsInvalid)(nc),
                (c.cJSON_IsFalse)(nc),
                (c.cJSON_IsTrue)(nc),
                (c.cJSON_IsBool)(nc),
                (c.cJSON_IsNull)(nc),
                (c.cJSON_IsNumber)(nc),
                (c.cJSON_IsString)(nc),
                (c.cJSON_IsArray)(nc),
                (c.cJSON_IsObject)(nc),
                (c.cJSON_IsRaw)(nc),
            ];
            let pr = [
                (r.cJSON_IsInvalid)(nr),
                (r.cJSON_IsFalse)(nr),
                (r.cJSON_IsTrue)(nr),
                (r.cJSON_IsBool)(nr),
                (r.cJSON_IsNull)(nr),
                (r.cJSON_IsNumber)(nr),
                (r.cJSON_IsString)(nr),
                (r.cJSON_IsArray)(nr),
                (r.cJSON_IsObject)(nr),
                (r.cJSON_IsRaw)(nr),
            ];
            assert_eq!(pc, pr, "predicates for fabricated type = {t:#x}");
            assert_eq!(
                (c.cJSON_GetNumberValue)(nc).to_bits(),
                (r.cJSON_GetNumberValue)(nr).to_bits(),
                "GetNumberValue for fabricated type = {t:#x}"
            );
            assert_eq!(
                cstr((c.cJSON_GetStringValue)(nc)),
                cstr((r.cJSON_GetStringValue)(nr)),
                "GetStringValue for fabricated type = {t:#x}"
            );
            // restore a deletable type before freeing
            (*nc).type_ = cJSON_NULL;
            (*nr).type_ = cJSON_NULL;
            (c.cJSON_Delete)(nc);
            (r.cJSON_Delete)(nr);
        }
    }
}

// ---------------------------------------------------------------------------
// rows 21, 22, 23 — AddItemToArray / AddItemToObject / AddItemToObjectCS
// ---------------------------------------------------------------------------
#[test]
fn cfg21_23_add_item() {
    let (c, r) = both();
    let mut rng = Rng::new(0x2123_2123);
    let pool = string_pool();

    // append into arrays of every size
    for size in [0usize, 1, 2, 3, 7] {
        let mut items: Vec<Spec> = (0..size).map(|i| Spec::Num(i as f64)).collect();
        items.push(Spec::Str(b"appended".to_vec()));
        assert_spec_matches(&c, &r, &Spec::Arr(items), &format!("append into {size}"));
    }

    // self-append and double-append return values
    unsafe {
        let ac = (c.cJSON_CreateArray)();
        let ar = (r.cJSON_CreateArray)();
        assert_eq!(
            (c.cJSON_AddItemToArray)(ac, ac),
            (r.cJSON_AddItemToArray)(ar, ar),
            "self-append"
        );
        let ic = (c.cJSON_CreateNumber)(1.0);
        let ir = (r.cJSON_CreateNumber)(1.0);
        assert_eq!(
            (c.cJSON_AddItemToArray)(ac, ic),
            (r.cJSON_AddItemToArray)(ar, ir)
        );
        // adding the SAME item a second time (cJSON allows it, the list gets a
        // self-loop) — only the return value is compared, the tree is then
        // abandoned without printing to avoid an infinite walk.
        assert_eq!(
            (c.cJSON_AddItemToArray)(ac, ic),
            (r.cJSON_AddItemToArray)(ar, ir),
            "double-append return value"
        );
        // break the loop again so Delete terminates
        (*ic).next = std::ptr::null_mut();
        (*ir).next = std::ptr::null_mut();
        (*ic).prev = ic;
        (*ir).prev = ir;
        (*ac).child = ic;
        (*ar).child = ir;
        (c.cJSON_Delete)(ac);
        (r.cJSON_Delete)(ar);
    }

    // keys of every shape, incl. duplicates, through AddItemToObject and …CS
    for i in 0..250 {
        let n = 1 + rng.below(5);
        let kv: Vec<(Vec<u8>, Spec)> = (0..n)
            .map(|_| (rand_string(&mut rng, &pool), rand_spec_with(&mut rng, 1, &pool)))
            .collect();
        assert_spec_matches(&c, &r, &Spec::Obj(kv.clone()), &format!("Obj #{i}"));
        assert_spec_matches(&c, &r, &Spec::ObjCS(kv), &format!("ObjCS #{i}"));
    }

    // An item that already carries a heap-allocated `string` gets it freed and
    // replaced by `add_item_to_object` (cJSON.c:2060).  The item is first
    // detached (which keeps `->string`) so that re-adding cannot create the
    // self-referential list that re-keying an already-linked child would.
    unsafe {
        let oc1 = (c.cJSON_CreateObject)();
        let or1 = (r.cJSON_CreateObject)();
        let oc2 = (c.cJSON_CreateObject)();
        let or2 = (r.cJSON_CreateObject)();
        let nc = (c.cJSON_CreateNumber)(5.0);
        let nr = (r.cJSON_CreateNumber)(5.0);
        let k1 = Bytes::new(b"a_short_key");
        let k2 = Bytes::new(b"a_much_longer_second_key");
        assert_eq!(
            (c.cJSON_AddItemToObject)(oc1, k1.as_ptr(), nc),
            (r.cJSON_AddItemToObject)(or1, k1.as_ptr(), nr)
        );
        assert_eq!(snap(oc1), snap(or1), "object after first key");
        let dc = (c.cJSON_DetachItemViaPointer)(oc1, nc);
        let dr = (r.cJSON_DetachItemViaPointer)(or1, nr);
        assert_eq!(snap(dc), snap(dr), "detached item keeps its key");
        // re-key: the old heap key must be freed and replaced
        assert_eq!(
            (c.cJSON_AddItemToObject)(oc2, k2.as_ptr(), dc),
            (r.cJSON_AddItemToObject)(or2, k2.as_ptr(), dr)
        );
        assert_eq!(snap(oc2), snap(or2), "object after re-key");
        assert_eq!(print_and_take(&c, oc2), print_and_take(&r, or2));
        for (api, o) in [(&c, oc1), (&c, oc2)] {
            (api.cJSON_Delete)(o);
        }
        for (api, o) in [(&r, or1), (&r, or2)] {
            (api.cJSON_Delete)(o);
        }
    }

    // AddItemToObjectCS marks the key constant; a later re-key with a heap key
    // must NOT free the constant one (cJSON.c:2060 checks cJSON_StringIsConst).
    unsafe {
        let oc1 = (c.cJSON_CreateObject)();
        let or1 = (r.cJSON_CreateObject)();
        let oc2 = (c.cJSON_CreateObject)();
        let or2 = (r.cJSON_CreateObject)();
        let nc = (c.cJSON_CreateNumber)(1.0);
        let nr = (r.cJSON_CreateNumber)(1.0);
        let kc = Bytes::new(b"const_key");
        let kh = Bytes::new(b"heap_key");
        (c.cJSON_AddItemToObjectCS)(oc1, kc.as_ptr(), nc);
        (r.cJSON_AddItemToObjectCS)(or1, kc.as_ptr(), nr);
        assert_eq!(snap(oc1), snap(or1), "after AddItemToObjectCS");
        assert_eq!(print_and_take(&c, oc1), print_and_take(&r, or1));
        let dc = (c.cJSON_DetachItemViaPointer)(oc1, nc);
        let dr = (r.cJSON_DetachItemViaPointer)(or1, nr);
        (c.cJSON_AddItemToObject)(oc2, kh.as_ptr(), dc);
        (r.cJSON_AddItemToObject)(or2, kh.as_ptr(), dr);
        assert_eq!(snap(oc2), snap(or2), "CS key replaced by heap key");
        assert_eq!(print_and_take(&c, oc2), print_and_take(&r, or2));
        (c.cJSON_Delete)(oc1);
        (c.cJSON_Delete)(oc2);
        (r.cJSON_Delete)(or1);
        (r.cJSON_Delete)(or2);
    }
}

// ---------------------------------------------------------------------------
// row 24 — item references inside arrays and objects
// ---------------------------------------------------------------------------
#[test]
fn cfg24_item_references() {
    let (c, r) = both();
    let inners = [
        Spec::Arr(vec![Spec::Num(1.0), Spec::Str(b"two".to_vec()), Spec::Null]),
        Spec::Obj(vec![
            (b"x".to_vec(), Spec::Num(1.0)),
            (b"y".to_vec(), Spec::Arr(vec![Spec::True, Spec::False])),
        ]),
        Spec::Arr(vec![]),
    ];
    for (i, inner) in inners.iter().enumerate() {
        assert_spec_matches(
            &c,
            &r,
            &Spec::ArrWithRefs(Box::new(inner.clone())),
            &format!("ArrWithRefs #{i}"),
        );
        assert_spec_matches(
            &c,
            &r,
            &Spec::ObjWithRefs(Box::new(inner.clone())),
            &format!("ObjWithRefs #{i}"),
        );
    }
}

// ---------------------------------------------------------------------------
// row 25 — all nine cJSON_Add*ToObject helpers
// ---------------------------------------------------------------------------
#[test]
fn cfg25_add_helpers() {
    let (c, r) = both();
    let pool = string_pool();

    // one explicit object exercising all nine helpers in order
    let all = Spec::ObjViaHelpers(vec![
        (b"n".to_vec(), Helper::Null),
        (b"t".to_vec(), Helper::True),
        (b"f".to_vec(), Helper::False),
        (b"b0".to_vec(), Helper::Bool(0)),
        (b"b1".to_vec(), Helper::Bool(1)),
        (b"b2".to_vec(), Helper::Bool(2)),
        (b"num".to_vec(), Helper::Num(1.5)),
        (b"nan".to_vec(), Helper::Num(f64::NAN)),
        (b"inf".to_vec(), Helper::Num(f64::INFINITY)),
        (b"str".to_vec(), Helper::Str(b"hi\t\"there\"".to_vec())),
        (b"raw".to_vec(), Helper::Raw(b"{\"raw\":true}".to_vec())),
        (b"obj".to_vec(), Helper::Object),
        (b"arr".to_vec(), Helper::Array),
    ]);
    assert_spec_matches(&c, &r, &all, "all nine Add*ToObject helpers");

    let mut rng = Rng::new(0x2525_2525);
    for i in 0..200 {
        let spec = {
            let n = 1 + rng.below(6);
            Spec::ObjViaHelpers(
                (0..n)
                    .map(|_| {
                        let h = match rng.below(9) {
                            0 => Helper::Null,
                            1 => Helper::True,
                            2 => Helper::False,
                            3 => Helper::Bool(rng.range_i32(-2, 3)),
                            4 => Helper::Num(rng.json_f64()),
                            5 => Helper::Str(rand_string(&mut rng, &pool)),
                            6 => Helper::Raw(rand_string(&mut rng, &pool)),
                            7 => Helper::Object,
                            _ => Helper::Array,
                        };
                        (rand_string(&mut rng, &pool), h)
                    })
                    .collect(),
            )
        };
        assert_spec_matches(&c, &r, &spec, &format!("random helpers #{i}"));
    }
}
