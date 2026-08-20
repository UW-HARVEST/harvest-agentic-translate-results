//! Phase B — rows C58..C84: query, mutation, duplicate and compare APIs.
#![allow(non_snake_case)]

mod common;
use common::*;

use std::ffi::{c_char, c_int};
use std::fmt::Write as _;
use std::ptr::null_mut;

unsafe fn make_array(api: &Api, n: usize) -> *mut CJson {
    let a = (api.cJSON_CreateArray)();
    for i in 0..n {
        (api.cJSON_AddItemToArray)(a, (api.cJSON_CreateNumber)(i as f64));
    }
    a
}

unsafe fn make_object(api: &Api, keys: &[&str]) -> *mut CJson {
    let o = (api.cJSON_CreateObject)();
    for (i, k) in keys.iter().enumerate() {
        let kb = cs(k);
        (api.cJSON_AddItemToObject)(o, kb.as_ptr(), (api.cJSON_CreateNumber)(i as f64));
    }
    o
}

/// One item of every type, built without the parser.
unsafe fn typed_items(api: &Api) -> Vec<(String, *mut CJson)> {
    let s = Box::leak(Box::new(cs("value")));
    let r = Box::leak(Box::new(cs("raw")));
    let mut v = vec![
        ("null".to_string(), (api.cJSON_CreateNull)()),
        ("true".to_string(), (api.cJSON_CreateTrue)()),
        ("false".to_string(), (api.cJSON_CreateFalse)()),
        ("number".to_string(), (api.cJSON_CreateNumber)(2.5)),
        ("nan".to_string(), (api.cJSON_CreateNumber)(f64::NAN)),
        ("string".to_string(), (api.cJSON_CreateString)(s.as_ptr())),
        (
            "string_ref".to_string(),
            (api.cJSON_CreateStringReference)(s.as_ptr()),
        ),
        ("raw".to_string(), (api.cJSON_CreateRaw)(r.as_ptr())),
        ("array".to_string(), make_array(api, 3)),
        ("object".to_string(), make_object(api, &["a", "b"])),
        ("empty_array".to_string(), (api.cJSON_CreateArray)()),
        ("empty_object".to_string(), (api.cJSON_CreateObject)()),
    ];
    for t in [CJSON_INVALID, 3, 0x0F, 0xFF, 0x1FF, c_int::MIN, CJSON_TRUE | CJSON_FALSE] {
        let it = (api.cJSON_CreateNumber)(1.0);
        (*it).type_ = t;
        v.push((format!("type_0x{t:x}"), it));
    }
    v
}

#[test]
fn c58_get_array_size() {
    diff("C58 cJSON_GetArraySize", |api| unsafe {
        let mut log = String::new();
        for (name, it) in typed_items(api) {
            let _ = writeln!(log, "{name}: size={}", (api.cJSON_GetArraySize)(it));
            (api.cJSON_Delete)(it);
        }
        for n in [0usize, 1, 2, 10, 100] {
            let a = make_array(api, n);
            let _ = writeln!(log, "array({n}) size={}", (api.cJSON_GetArraySize)(a));
            (api.cJSON_Delete)(a);
        }
        let _ = writeln!(log, "NULL size={}", (api.cJSON_GetArraySize)(null_mut()));
        log
    });
}

#[test]
fn c59_get_array_item() {
    diff("C59 cJSON_GetArrayItem", |api| unsafe {
        let mut log = String::new();
        for n in [0usize, 1, 2, 5] {
            let a = make_array(api, n);
            for idx in [-2i32, -1, 0, 1, 2, 4, 5, 6, 1000, c_int::MAX, c_int::MIN] {
                let it = (api.cJSON_GetArrayItem)(a, idx);
                let _ = writeln!(
                    log,
                    "array({n})[{idx}] = {:?}",
                    (!it.is_null()).then(|| (*it).valueint)
                );
            }
            (api.cJSON_Delete)(a);
        }
        // objects behave like arrays for index access
        let o = make_object(api, &["x", "y", "z"]);
        for idx in [0i32, 1, 2, 3] {
            let it = (api.cJSON_GetArrayItem)(o, idx);
            let _ = writeln!(
                log,
                "object[{idx}] key={:?}",
                (!it.is_null()).then(|| read_cstr((*it).string).map(|v| show(&v)))
            );
        }
        (api.cJSON_Delete)(o);
        let _ = writeln!(log, "NULL[0] null={}", (api.cJSON_GetArrayItem)(null_mut(), 0).is_null());
        log
    });
}

#[test]
fn c60_c61_c62_object_lookup() {
    diff("C60/C61/C62 object lookup (case (in)sensitive)", |api| unsafe {
        let mut log = String::new();
        let o = (api.cJSON_CreateObject)();
        for (k, v) in [
            ("alpha", 1.0),
            ("Beta", 2.0),
            ("GAMMA", 3.0),
            ("", 4.0),
            ("dup", 5.0),
            ("dup", 6.0),
            ("mIxEd", 7.0),
        ] {
            let kb = cs(k);
            (api.cJSON_AddItemToObject)(o, kb.as_ptr(), (api.cJSON_CreateNumber)(v));
        }
        // an array element (no `string`) mixed into the child chain
        (api.cJSON_AddItemToArray)(o, (api.cJSON_CreateNumber)(99.0));
        let last = cs("last");
        (api.cJSON_AddItemToObject)(o, last.as_ptr(), (api.cJSON_CreateNumber)(8.0));

        let _ = write!(log, "graph: {}", dump(o));
        for k in [
            "alpha", "ALPHA", "Alpha", "beta", "Beta", "gamma", "GAMMA", "", "dup", "DUP",
            "mixed", "MIXED", "mIxEd", "missing", "last", "alpha ", " alpha",
        ] {
            let kb = cs(k);
            let ins = (api.cJSON_GetObjectItem)(o, kb.as_ptr());
            let sens = (api.cJSON_GetObjectItemCaseSensitive)(o, kb.as_ptr());
            let _ = writeln!(
                log,
                "{k:?}: insens={:?} sens={:?} has={}",
                (!ins.is_null()).then(|| (*ins).valuedouble),
                (!sens.is_null()).then(|| (*sens).valuedouble),
                (api.cJSON_HasObjectItem)(o, kb.as_ptr())
            );
        }
        (api.cJSON_Delete)(o);
        log
    });
}

#[test]
fn c63_c64_value_accessors() {
    diff("C63/C64 cJSON_GetStringValue / cJSON_GetNumberValue", |api| unsafe {
        let mut log = String::new();
        for (name, it) in typed_items(api) {
            let sv = read_cstr((api.cJSON_GetStringValue)(it));
            let nv = (api.cJSON_GetNumberValue)(it);
            let _ = writeln!(
                log,
                "{name}: str={:?} num=0x{:016x}",
                sv.map(|v| show(&v)),
                nv.to_bits()
            );
            (api.cJSON_Delete)(it);
        }
        let _ = writeln!(
            log,
            "NULL: str_null={} num=0x{:016x}",
            (api.cJSON_GetStringValue)(null_mut()).is_null(),
            (api.cJSON_GetNumberValue)(null_mut()).to_bits()
        );
        // numbers with special payloads
        for v in [0.0, -0.0, f64::NAN, f64::INFINITY, f64::NEG_INFINITY, 1e308] {
            let it = (api.cJSON_CreateNumber)(v);
            let _ = writeln!(
                log,
                "num 0x{:016x} -> 0x{:016x}",
                v.to_bits(),
                (api.cJSON_GetNumberValue)(it).to_bits()
            );
            (api.cJSON_Delete)(it);
        }
        log
    });
}

#[test]
fn c65_type_predicates() {
    diff("C65 cJSON_Is* over every type", |api| unsafe {
        let mut log = String::new();
        let preds: [(&str, unsafe extern "C" fn(*const CJson) -> c_int); 10] = [
            ("IsInvalid", api.cJSON_IsInvalid),
            ("IsFalse", api.cJSON_IsFalse),
            ("IsTrue", api.cJSON_IsTrue),
            ("IsBool", api.cJSON_IsBool),
            ("IsNull", api.cJSON_IsNull),
            ("IsNumber", api.cJSON_IsNumber),
            ("IsString", api.cJSON_IsString),
            ("IsArray", api.cJSON_IsArray),
            ("IsObject", api.cJSON_IsObject),
            ("IsRaw", api.cJSON_IsRaw),
        ];
        // every possible low byte, plus the flag bits
        let it = (api.cJSON_CreateNumber)(1.0);
        for hi in [0, CJSON_IS_REFERENCE, CJSON_STRING_IS_CONST, CJSON_IS_REFERENCE | CJSON_STRING_IS_CONST] {
            for low in 0..=0xFFi32 {
                (*it).type_ = hi | low;
                let mut row = format!("type=0x{:x}:", (*it).type_);
                for (n, f) in preds {
                    let _ = write!(row, " {n}={}", f(it));
                }
                let _ = writeln!(log, "{row}");
            }
        }
        (*it).type_ = CJSON_NUMBER;
        (api.cJSON_Delete)(it);
        // NULL for every predicate
        for (n, f) in preds {
            let _ = writeln!(log, "{n}(NULL)={}", f(null_mut()));
        }
        // and the real items
        for (name, item) in typed_items(api) {
            let mut row = format!("{name}:");
            for (n, f) in preds {
                let _ = write!(row, " {n}={}", f(item));
            }
            let _ = writeln!(log, "{row}");
            (api.cJSON_Delete)(item);
        }
        log
    });
}

#[test]
fn c66_set_number_helper() {
    diff("C66 cJSON_SetNumberHelper", |api| unsafe {
        let mut log = String::new();
        let mut rng = Rng::new(0xAAAA_BBBB_CCCC_DDDD);
        let mut values: Vec<f64> = vec![
            0.0, -0.0, 1.0, -1.0, 2147483647.0, 2147483648.0, -2147483648.0, -2147483649.0,
            f64::NAN, f64::INFINITY, f64::NEG_INFINITY, 1e300, -1e300, 0.5, -0.5,
            2147483646.9, -2147483647.9,
        ];
        for _ in 0..300 {
            values.push(rng.nice_f64());
        }
        for _ in 0..200 {
            values.push(rng.any_f64());
        }
        for v in values {
            let it = (api.cJSON_CreateNumber)(0.0);
            let ret = (api.cJSON_SetNumberHelper)(it, v);
            let p = take_print(api, (api.cJSON_PrintUnformatted)(it));
            let _ = writeln!(
                log,
                "set 0x{:016x}: ret=0x{:016x} int={} dbl=0x{:016x} print={}",
                v.to_bits(),
                ret.to_bits(),
                (*it).valueint,
                (*it).valuedouble.to_bits(),
                p.map(|x| show(&x)).unwrap_or("NULL".into())
            );
            (api.cJSON_Delete)(it);
        }
        log
    });
}

#[test]
fn c67_set_valuestring() {
    diff("C67 cJSON_SetValuestring", |api| unsafe {
        let mut log = String::new();
        let cases: [(&str, &str); 9] = [
            ("original", "short"),      // shorter
            ("original", "originaL"),   // same length
            ("original", "much longer than before"),
            ("", ""),
            ("", "grown"),
            ("abc", ""),
            ("abc", "ab"),
            ("abc", "abc"),
            ("with \"escape\"\n", "other \\ escape\t"),
        ];
        for (old, new) in cases {
            let ob = cs(old);
            let nb = cs(new);
            let it = (api.cJSON_CreateString)(ob.as_ptr());
            let before = read_cstr((*it).valuestring).map(|v| show(&v));
            let ret = (api.cJSON_SetValuestring)(it, nb.as_ptr());
            let after = read_cstr((*it).valuestring).map(|v| show(&v));
            let _ = writeln!(
                log,
                "{old:?} -> {new:?}: before={:?} ret_null={} ret_is_valuestring={} after={:?}",
                before,
                ret.is_null(),
                ret == (*it).valuestring,
                after
            );
            let p = take_print(api, (api.cJSON_PrintUnformatted)(it));
            let _ = writeln!(log, "  print={}", p.map(|v| show(&v)).unwrap_or("NULL".into()));
            (api.cJSON_Delete)(it);
        }
        log
    });
}

#[test]
fn c68_detach_via_pointer() {
    diff("C68 cJSON_DetachItemViaPointer", |api| unsafe {
        let mut log = String::new();
        for n in [1usize, 2, 3, 5] {
            for pos in 0..n {
                let a = make_array(api, n);
                let target = (api.cJSON_GetArrayItem)(a, pos as c_int);
                let det = (api.cJSON_DetachItemViaPointer)(a, target);
                let _ = write!(log, "array n={n} pos={pos} detached={}\n{}", !det.is_null(), dump(a));
                let _ = write!(log, "  det: {}", dump(det));
                let p = take_print(api, (api.cJSON_PrintUnformatted)(a));
                let _ = writeln!(log, "  print={}", p.map(|v| show(&v)).unwrap_or("NULL".into()));
                (api.cJSON_Delete)(det);
                (api.cJSON_Delete)(a);
            }
        }
        for keys in [&["a"][..], &["a", "b"], &["a", "b", "c"]] {
            for pos in 0..keys.len() {
                let o = make_object(api, keys);
                let target = (api.cJSON_GetArrayItem)(o, pos as c_int);
                let det = (api.cJSON_DetachItemViaPointer)(o, target);
                let _ = write!(log, "object {keys:?} pos={pos}\n{}", dump(o));
                let _ = write!(log, "  det: {}", dump(det));
                (api.cJSON_Delete)(det);
                (api.cJSON_Delete)(o);
            }
        }
        log
    });
}

#[test]
fn c69_c71_detach_delete_from_array() {
    diff("C69/C71 cJSON_DetachItemFromArray / DeleteItemFromArray", |api| unsafe {
        let mut log = String::new();
        for n in [0usize, 1, 2, 5] {
            for which in [-1i32, 0, 1, 2, 4, 5, 100] {
                let a = make_array(api, n);
                let det = (api.cJSON_DetachItemFromArray)(a, which);
                let _ = write!(
                    log,
                    "detach n={n} which={which} null={}\n{}",
                    det.is_null(),
                    dump(a)
                );
                let _ = write!(log, "  det: {}", dump(det));
                (api.cJSON_Delete)(det);
                (api.cJSON_Delete)(a);

                let a = make_array(api, n);
                (api.cJSON_DeleteItemFromArray)(a, which);
                let _ = write!(log, "delete n={n} which={which}\n{}", dump(a));
                let p = take_print(api, (api.cJSON_PrintUnformatted)(a));
                let _ = writeln!(log, "  print={}", p.map(|v| show(&v)).unwrap_or("NULL".into()));
                (api.cJSON_Delete)(a);
            }
        }
        log
    });
}

#[test]
fn c70_c72_detach_delete_from_object() {
    diff("C70/C72 detach/delete from object", |api| unsafe {
        let mut log = String::new();
        let keys = ["alpha", "Beta", "GAMMA"];
        for probe in ["alpha", "ALPHA", "beta", "Beta", "gamma", "GAMMA", "missing", ""] {
            for sensitive in [false, true] {
                let o = make_object(api, &keys);
                let pb = cs(probe);
                let det = if sensitive {
                    (api.cJSON_DetachItemFromObjectCaseSensitive)(o, pb.as_ptr())
                } else {
                    (api.cJSON_DetachItemFromObject)(o, pb.as_ptr())
                };
                let _ = write!(
                    log,
                    "detach {probe:?} sensitive={sensitive} null={}\n{}",
                    det.is_null(),
                    dump(o)
                );
                let _ = write!(log, "  det: {}", dump(det));
                (api.cJSON_Delete)(det);
                (api.cJSON_Delete)(o);

                let o = make_object(api, &keys);
                if sensitive {
                    (api.cJSON_DeleteItemFromObjectCaseSensitive)(o, pb.as_ptr());
                } else {
                    (api.cJSON_DeleteItemFromObject)(o, pb.as_ptr());
                }
                let p = take_print(api, (api.cJSON_PrintUnformatted)(o));
                let _ = writeln!(
                    log,
                    "delete {probe:?} sensitive={sensitive} print={}",
                    p.map(|v| show(&v)).unwrap_or("NULL".into())
                );
                let _ = write!(log, "{}", dump(o));
                (api.cJSON_Delete)(o);
            }
        }
        log
    });
}

#[test]
fn c73_insert_item_in_array() {
    diff("C73 cJSON_InsertItemInArray", |api| unsafe {
        let mut log = String::new();
        for n in [0usize, 1, 2, 5] {
            for which in [-1i32, 0, 1, 2, 4, 5, 6, 100] {
                let a = make_array(api, n);
                let item = (api.cJSON_CreateNumber)(999.0);
                let rc = (api.cJSON_InsertItemInArray)(a, which, item);
                let _ = write!(log, "insert n={n} which={which} rc={rc}\n{}", dump(a));
                let p = take_print(api, (api.cJSON_PrintUnformatted)(a));
                let _ = writeln!(log, "  print={}", p.map(|v| show(&v)).unwrap_or("NULL".into()));
                if rc == 0 {
                    (api.cJSON_Delete)(item);
                }
                (api.cJSON_Delete)(a);
            }
        }
        // insert into an object
        let o = make_object(api, &["a", "b"]);
        let item = (api.cJSON_CreateNumber)(7.0);
        let _ = writeln!(log, "object insert rc={}", (api.cJSON_InsertItemInArray)(o, 1, item));
        let _ = write!(log, "{}", dump(o));
        (api.cJSON_Delete)(o);
        log
    });
}

#[test]
fn c74_c75_replace() {
    diff("C74/C75 cJSON_ReplaceItemViaPointer / InArray", |api| unsafe {
        let mut log = String::new();
        for n in [1usize, 2, 3, 5] {
            for pos in 0..n {
                let a = make_array(api, n);
                let target = (api.cJSON_GetArrayItem)(a, pos as c_int);
                let rep = (api.cJSON_CreateString)(cs("replacement").as_ptr());
                let rc = (api.cJSON_ReplaceItemViaPointer)(a, target, rep);
                let _ = write!(log, "via ptr n={n} pos={pos} rc={rc}\n{}", dump(a));
                let p = take_print(api, (api.cJSON_PrintUnformatted)(a));
                let _ = writeln!(log, "  print={}", p.map(|v| show(&v)).unwrap_or("NULL".into()));
                (api.cJSON_Delete)(a);
            }
        }
        for n in [0usize, 1, 2, 5] {
            for which in [-1i32, 0, 1, 4, 5, 100] {
                let a = make_array(api, n);
                let rep = (api.cJSON_CreateNumber)(-1.0);
                let rc = (api.cJSON_ReplaceItemInArray)(a, which, rep);
                let _ = write!(log, "in array n={n} which={which} rc={rc}\n{}", dump(a));
                if rc == 0 {
                    (api.cJSON_Delete)(rep);
                }
                (api.cJSON_Delete)(a);
            }
        }
        // replacement == item
        let a = make_array(api, 3);
        let target = (api.cJSON_GetArrayItem)(a, 1);
        let _ = writeln!(
            log,
            "self replace rc={}",
            (api.cJSON_ReplaceItemViaPointer)(a, target, target)
        );
        let _ = write!(log, "{}", dump(a));
        (api.cJSON_Delete)(a);
        // replace inside an object
        for keys in [&["a"][..], &["a", "b", "c"]] {
            for pos in 0..keys.len() {
                let o = make_object(api, keys);
                let target = (api.cJSON_GetArrayItem)(o, pos as c_int);
                let rep = (api.cJSON_CreateNumber)(42.0);
                let rc = (api.cJSON_ReplaceItemViaPointer)(o, target, rep);
                let _ = write!(log, "object via ptr {keys:?} pos={pos} rc={rc}\n{}", dump(o));
                let p = take_print(api, (api.cJSON_PrintUnformatted)(o));
                let _ = writeln!(log, "  print={}", p.map(|v| show(&v)).unwrap_or("NULL".into()));
                (api.cJSON_Delete)(o);
            }
        }
        log
    });
}

#[test]
fn c76_replace_in_object() {
    diff("C76 cJSON_ReplaceItemInObject(CaseSensitive)", |api| unsafe {
        let mut log = String::new();
        let keys = ["alpha", "Beta", "GAMMA"];
        for probe in ["alpha", "ALPHA", "beta", "Beta", "GAMMA", "missing", ""] {
            for sensitive in [false, true] {
                for rep_kind in 0..3 {
                    let o = make_object(api, &keys);
                    let pb = cs(probe);
                    let rep = (api.cJSON_CreateNumber)(77.0);
                    match rep_kind {
                        1 => {
                            // replacement already carries a heap key
                            let tmp = (api.cJSON_CreateObject)();
                            let k = cs("old_key");
                            (api.cJSON_AddItemToObject)(tmp, k.as_ptr(), rep);
                            (api.cJSON_DetachItemViaPointer)(tmp, rep);
                            (api.cJSON_Delete)(tmp);
                        }
                        2 => {
                            // replacement carries a const key
                            let tmp = (api.cJSON_CreateObject)();
                            let k = Box::leak(Box::new(cs("const_key")));
                            (api.cJSON_AddItemToObjectCS)(tmp, k.as_ptr(), rep);
                            (api.cJSON_DetachItemViaPointer)(tmp, rep);
                            (api.cJSON_Delete)(tmp);
                        }
                        _ => {}
                    }
                    let rc = if sensitive {
                        (api.cJSON_ReplaceItemInObjectCaseSensitive)(o, pb.as_ptr(), rep)
                    } else {
                        (api.cJSON_ReplaceItemInObject)(o, pb.as_ptr(), rep)
                    };
                    let _ = write!(
                        log,
                        "replace {probe:?} sensitive={sensitive} rep_kind={rep_kind} rc={rc}\n{}",
                        dump(o)
                    );
                    let _ = writeln!(
                        log,
                        "  rep type=0x{:x} key={:?}",
                        (*rep).type_,
                        read_cstr((*rep).string).map(|v| show(&v))
                    );
                    let p = take_print(api, (api.cJSON_PrintUnformatted)(o));
                    let _ = writeln!(log, "  print={}", p.map(|v| show(&v)).unwrap_or("NULL".into()));
                    if rc == 0 {
                        (api.cJSON_Delete)(rep);
                    }
                    (api.cJSON_Delete)(o);
                }
            }
        }
        log
    });
}

#[test]
fn c77_c78_duplicate() {
    diff("C77/C78 cJSON_Duplicate recurse 0/1", |api| unsafe {
        let mut log = String::new();
        let mut rng = Rng::new(0x1357_9BDF_2468_ACE0);

        for recurse in [0i32, 1, 2, -1, c_int::MIN] {
            for (name, it) in typed_items(api) {
                let d = (api.cJSON_Duplicate)(it, recurse);
                let _ = write!(log, "{name} recurse={recurse}\n  src: {}", dump(it));
                let _ = write!(log, "  dup: {}", dump(d));
                let p = take_print(api, (api.cJSON_PrintUnformatted)(d));
                let _ = writeln!(
                    log,
                    "  print={} compare={}",
                    p.map(|v| show(&v)).unwrap_or("NULL".into()),
                    (api.cJSON_Compare)(it, d, 1)
                );
                (api.cJSON_Delete)(d);
                (api.cJSON_Delete)(it);
            }
        }
        // random documents, both recurse modes
        for round in 0..80 {
            let text = gen_json(&mut rng);
            let buf = CBuf::new(&text);
            let root = (api.cJSON_Parse)(buf.ptr());
            for recurse in [0i32, 1] {
                let d = (api.cJSON_Duplicate)(root, recurse);
                let _ = write!(
                    log,
                    "round {round} recurse={recurse} src={}\n  dup: {}",
                    show(&text),
                    dump(d)
                );
                let p = take_print(api, (api.cJSON_PrintUnformatted)(d));
                let _ = writeln!(
                    log,
                    "  print={} cmp_sens={} cmp_insens={}",
                    p.map(|v| show(&v)).unwrap_or("NULL".into()),
                    (api.cJSON_Compare)(root, d, 1),
                    (api.cJSON_Compare)(root, d, 0)
                );
                (api.cJSON_Delete)(d);
            }
            (api.cJSON_Delete)(root);
        }
        // const keys and references survive duplication with the flag cleared
        let o = (api.cJSON_CreateObject)();
        let k = Box::leak(Box::new(cs("const")));
        (api.cJSON_AddItemToObjectCS)(o, k.as_ptr(), (api.cJSON_CreateNumber)(1.0));
        let inner = make_array(api, 2);
        (api.cJSON_AddItemReferenceToArray)(o, inner);
        for recurse in [0i32, 1] {
            let d = (api.cJSON_Duplicate)(o, recurse);
            let _ = write!(log, "const/ref recurse={recurse}\n  src: {}", dump(o));
            let _ = write!(log, "  dup: {}", dump(d));
            (api.cJSON_Delete)(d);
        }
        (api.cJSON_Delete)(o);
        (api.cJSON_Delete)(inner);
        log
    });
}

/// Row C79 — the `depth` parameter of `cJSON_Duplicate_rec`.
///
/// The reference C build hides `cJSON_Duplicate_rec` (`-fvisibility=hidden`), so
/// it cannot be `dlsym`'d from the C `.so`. The only depth reachable through the
/// public C API is 0 (`cJSON_Duplicate`), which is what is compared here: the
/// Rust `.so`'s exported `cJSON_Duplicate_rec(item, 0, recurse)` must be
/// byte-identical to the C `cJSON_Duplicate(item, recurse)`. The depth *limit*
/// itself is covered differentially by `c79b_duplicate_circular_limit`.
#[test]
fn c79a_duplicate_rec_depth0() {
    let (c, r) = libs();
    let rust_lib = unsafe {
        libloading::Library::new(rust_driver_so_path()).expect("rust .so")
    };
    let dup_rec: libloading::Symbol<
        unsafe extern "C" fn(*const CJson, usize, c_int) -> *mut CJson,
    > = unsafe { rust_lib.get(b"cJSON_Duplicate_rec\0").expect("cJSON_Duplicate_rec") };

    let mut rng = Rng::new(0x2468_ACE0_1357_9BDF);
    for round in 0..60 {
        let text = gen_json(&mut rng);
        for recurse in [0i32, 1] {
            unsafe {
                let buf = CBuf::new(&text);
                let c_root = (c.cJSON_Parse)(buf.ptr());
                let c_dup = (c.cJSON_Duplicate)(c_root, recurse);
                let c_log = format!(
                    "{}{}",
                    dump(c_dup),
                    take_print(c, (c.cJSON_PrintUnformatted)(c_dup))
                        .map(|v| show(&v))
                        .unwrap_or("NULL".into())
                );
                (c.cJSON_Delete)(c_dup);
                (c.cJSON_Delete)(c_root);

                let r_root = (r.cJSON_Parse)(buf.ptr());
                let r_dup = dup_rec(r_root, 0, recurse);
                let r_log = format!(
                    "{}{}",
                    dump(r_dup),
                    take_print(r, (r.cJSON_PrintUnformatted)(r_dup))
                        .map(|v| show(&v))
                        .unwrap_or("NULL".into())
                );
                (r.cJSON_Delete)(r_dup);
                (r.cJSON_Delete)(r_root);

                assert_eq!(
                    c_log, r_log,
                    "C79 round {round} recurse={recurse} src={}",
                    show(&text)
                );
            }
        }
    }
}

/// Row C79 / ERRORS 158 — `depth >= CJSON_CIRCULAR_LIMIT`.
#[test]
fn c79b_duplicate_circular_limit() {
    // deep recursion in both implementations: run on a thread with a big stack
    std::thread::Builder::new()
        .stack_size(256 * 1024 * 1024)
        .spawn(|| {
            diff("C79b CJSON_CIRCULAR_LIMIT boundary", |api| unsafe {
                let mut log = String::new();
                for depth in [1usize, 2, 9998, 9999, 10000, 10001, 10002] {
                    // chain of `depth` nested single-element arrays
                    let mut root = (api.cJSON_CreateArray)();
                    let innermost = root;
                    for _ in 1..depth {
                        let outer = (api.cJSON_CreateArray)();
                        (api.cJSON_AddItemToArray)(outer, root);
                        root = outer;
                    }
                    let _ = (api.cJSON_AddItemToArray)(innermost, (api.cJSON_CreateNumber)(1.0));
                    for recurse in [0i32, 1] {
                        let d = (api.cJSON_Duplicate)(root, recurse);
                        let mut measured = 0usize;
                        if !d.is_null() {
                            let mut p = d;
                            while !(*p).child.is_null() {
                                p = (*p).child;
                                measured += 1;
                            }
                        }
                        let _ = writeln!(
                            log,
                            "depth={depth} recurse={recurse} null={} measured={measured}",
                            d.is_null()
                        );
                        (api.cJSON_Delete)(d);
                    }
                    (api.cJSON_Delete)(root);
                }
                log
            });
        })
        .unwrap()
        .join()
        .unwrap();
}

#[test]
fn c80_c84_compare() {
    diff("C80..C84 cJSON_Compare", |api| unsafe {
        let mut log = String::new();

        // every type against every type, both case modes
        let a_items = typed_items(api);
        let b_items = typed_items(api);
        for (na, ia) in &a_items {
            for (nb, ib) in &b_items {
                for cs_ in [0i32, 1, 2, -1] {
                    let _ = writeln!(
                        log,
                        "{na} vs {nb} cs={cs_} -> {}",
                        (api.cJSON_Compare)(*ia, *ib, cs_)
                    );
                }
            }
            let _ = writeln!(log, "{na} vs NULL -> {}", (api.cJSON_Compare)(*ia, null_mut(), 1));
            let _ = writeln!(log, "NULL vs {na} -> {}", (api.cJSON_Compare)(null_mut(), *ia, 1));
            let _ = writeln!(log, "{na} vs self -> {}", (api.cJSON_Compare)(*ia, *ia, 1));
        }
        let _ = writeln!(
            log,
            "NULL vs NULL -> {}",
            (api.cJSON_Compare)(null_mut(), null_mut(), 1)
        );
        for (_, it) in a_items {
            (api.cJSON_Delete)(it);
        }
        for (_, it) in b_items {
            (api.cJSON_Delete)(it);
        }

        // numeric tolerance
        let pairs: [(f64, f64); 14] = [
            (0.0, 0.0),
            (0.0, -0.0),
            (1.0, 1.0),
            (1.0, 1.0 + f64::EPSILON),
            (1.0, 1.0 + 4.0 * f64::EPSILON),
            (1e300, 1e300),
            (1e300, 1.0000000000000002e300),
            (f64::NAN, f64::NAN),
            (f64::INFINITY, f64::INFINITY),
            (f64::INFINITY, f64::NEG_INFINITY),
            (f64::NAN, 0.0),
            (1e-300, 1.0000000000000002e-300),
            (0.1, 0.1),
            (0.1, 0.10000000000000002),
        ];
        for (x, y) in pairs {
            let a = (api.cJSON_CreateNumber)(x);
            let b = (api.cJSON_CreateNumber)(y);
            let _ = writeln!(
                log,
                "num 0x{:016x} vs 0x{:016x} -> {}",
                x.to_bits(),
                y.to_bits(),
                (api.cJSON_Compare)(a, b, 1)
            );
            (api.cJSON_Delete)(a);
            (api.cJSON_Delete)(b);
        }

        // structural pairs
        let pairs: [(&str, &str); 22] = [
            ("[]", "[]"),
            ("[1]", "[1]"),
            ("[1]", "[1,2]"),
            ("[1,2]", "[1]"),
            ("[1,2]", "[2,1]"),
            ("[[1]]", "[[1]]"),
            ("[[1]]", "[[2]]"),
            ("{}", "{}"),
            ("{\"a\":1}", "{\"a\":1}"),
            ("{\"a\":1}", "{\"A\":1}"),
            ("{\"a\":1,\"b\":2}", "{\"b\":2,\"a\":1}"),
            ("{\"a\":1}", "{\"a\":1,\"b\":2}"),
            ("{\"a\":1,\"b\":2}", "{\"a\":1}"),
            ("{\"a\":{\"b\":[1,2]}}", "{\"a\":{\"b\":[1,2]}}"),
            ("{\"a\":{\"b\":[1,2]}}", "{\"a\":{\"b\":[1,3]}}"),
            ("\"x\"", "\"x\""),
            ("\"x\"", "\"X\""),
            ("null", "null"),
            ("true", "true"),
            ("true", "false"),
            ("1", "1.0"),
            ("{\"a\":1,\"A\":2}", "{\"A\":2,\"a\":1}"),
        ];
        for (x, y) in pairs {
            let xb = cs(x);
            let yb = cs(y);
            let a = (api.cJSON_Parse)(xb.as_ptr());
            let b = (api.cJSON_Parse)(yb.as_ptr());
            for cs_ in [0i32, 1] {
                let _ = writeln!(log, "{x} vs {y} cs={cs_} -> {}", (api.cJSON_Compare)(a, b, cs_));
            }
            (api.cJSON_Delete)(a);
            (api.cJSON_Delete)(b);
        }

        // raw items compare by valuestring
        let r1 = (api.cJSON_CreateRaw)(cs("abc").as_ptr());
        let r2 = (api.cJSON_CreateRaw)(cs("abc").as_ptr());
        let r3 = (api.cJSON_CreateRaw)(cs("abd").as_ptr());
        let _ = writeln!(log, "raw eq -> {}", (api.cJSON_Compare)(r1, r2, 1));
        let _ = writeln!(log, "raw ne -> {}", (api.cJSON_Compare)(r1, r3, 1));
        (api.cJSON_Delete)(r1);
        (api.cJSON_Delete)(r2);
        (api.cJSON_Delete)(r3);

        // random documents against themselves and against a mutated copy
        let mut rng = Rng::new(0x9999_8888_7777_6666);
        for round in 0..100 {
            let t1 = gen_json(&mut rng);
            let t2 = gen_json(&mut rng);
            let b1 = CBuf::new(&t1);
            let b2 = CBuf::new(&t2);
            let a = (api.cJSON_Parse)(b1.ptr());
            let b = (api.cJSON_Parse)(b2.ptr());
            let a2 = (api.cJSON_Duplicate)(a, 1);
            for cs_ in [0i32, 1] {
                let _ = writeln!(
                    log,
                    "round {round} cs={cs_}: self={} dup={} other={}",
                    (api.cJSON_Compare)(a, a, cs_),
                    (api.cJSON_Compare)(a, a2, cs_),
                    (api.cJSON_Compare)(a, b, cs_)
                );
            }
            (api.cJSON_Delete)(a);
            (api.cJSON_Delete)(a2);
            (api.cJSON_Delete)(b);
        }
        log
    });
}

#[allow(unused)]
fn _keep(_: *const c_char) {}
