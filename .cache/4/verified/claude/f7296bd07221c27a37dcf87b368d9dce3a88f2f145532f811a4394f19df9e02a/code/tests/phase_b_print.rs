//! Phase B — rows C25..C37: every printing entry point × format × sink.
#![allow(non_snake_case)]

mod common;
use common::*;

use std::ffi::{c_char, c_int};
use std::fmt::Write as _;

/// Build one item of every kind that can be handed to `print_value`, including
/// the ones that no parser can produce (raw, references, invalid types).
unsafe fn build_zoo(api: &Api, keep: &mut Vec<*mut CJson>) -> Vec<(String, *mut CJson)> {
    let mut v: Vec<(String, *mut CJson)> = Vec::new();

    v.push(("null".into(), (api.cJSON_CreateNull)()));
    v.push(("true".into(), (api.cJSON_CreateTrue)()));
    v.push(("false".into(), (api.cJSON_CreateFalse)()));
    v.push(("number_int".into(), (api.cJSON_CreateNumber)(42.0)));
    v.push(("number_frac".into(), (api.cJSON_CreateNumber)(1.0 / 3.0)));
    v.push(("number_nan".into(), (api.cJSON_CreateNumber)(f64::NAN)));
    v.push(("number_inf".into(), (api.cJSON_CreateNumber)(f64::INFINITY)));

    let s = Box::leak(Box::new(cs("a \"string\"\twith\nescapes\x01")));
    v.push(("string".into(), (api.cJSON_CreateString)(s.as_ptr())));
    let sref = Box::leak(Box::new(cs("string reference")));
    v.push((
        "string_ref".into(),
        (api.cJSON_CreateStringReference)(sref.as_ptr()),
    ));
    let r = Box::leak(Box::new(cs("[raw,json]")));
    v.push(("raw".into(), (api.cJSON_CreateRaw)(r.as_ptr())));

    v.push(("empty_array".into(), (api.cJSON_CreateArray)()));
    v.push(("empty_object".into(), (api.cJSON_CreateObject)()));

    // array with one of everything
    let arr = (api.cJSON_CreateArray)();
    (api.cJSON_AddItemToArray)(arr, (api.cJSON_CreateNull)());
    (api.cJSON_AddItemToArray)(arr, (api.cJSON_CreateTrue)());
    (api.cJSON_AddItemToArray)(arr, (api.cJSON_CreateFalse)());
    (api.cJSON_AddItemToArray)(arr, (api.cJSON_CreateNumber)(-0.5));
    (api.cJSON_AddItemToArray)(arr, (api.cJSON_CreateString)(s.as_ptr()));
    (api.cJSON_AddItemToArray)(arr, (api.cJSON_CreateRaw)(r.as_ptr()));
    (api.cJSON_AddItemToArray)(arr, (api.cJSON_CreateArray)());
    (api.cJSON_AddItemToArray)(arr, (api.cJSON_CreateObject)());
    v.push(("array_mixed".into(), arr));

    // object with one of everything, incl. a key that needs escaping
    let obj = (api.cJSON_CreateObject)();
    let k1 = Box::leak(Box::new(cs("plain")));
    let k2 = Box::leak(Box::new(cs("needs \"escaping\"\n")));
    let k3 = Box::leak(Box::new(cs("")));
    (api.cJSON_AddItemToObject)(obj, k1.as_ptr(), (api.cJSON_CreateNumber)(1.0));
    (api.cJSON_AddItemToObject)(obj, k2.as_ptr(), (api.cJSON_CreateString)(s.as_ptr()));
    (api.cJSON_AddItemToObjectCS)(obj, k3.as_ptr(), (api.cJSON_CreateNull)());
    let nested = (api.cJSON_CreateObject)();
    (api.cJSON_AddItemToObject)(nested, k1.as_ptr(), (api.cJSON_CreateArray)());
    (api.cJSON_AddItemToObject)(obj, k1.as_ptr(), nested);
    v.push(("object_mixed".into(), obj));

    // deeply nested object (exercises the `depth` indentation logic)
    let mut deep = (api.cJSON_CreateObject)();
    for _ in 0..8 {
        let outer = (api.cJSON_CreateObject)();
        (api.cJSON_AddItemToObject)(outer, k1.as_ptr(), deep);
        deep = outer;
    }
    v.push(("object_deep".into(), deep));

    let mut deep_arr = (api.cJSON_CreateArray)();
    for _ in 0..8 {
        let outer = (api.cJSON_CreateArray)();
        (api.cJSON_AddItemToArray)(outer, deep_arr);
        deep_arr = outer;
    }
    v.push(("array_deep".into(), deep_arr));

    // invalid / unusual types
    for t in [
        CJSON_INVALID,
        3,
        0x0F,
        0xFF,
        0x1FF,
        c_int::MIN,
        CJSON_NUMBER | CJSON_IS_REFERENCE,
    ] {
        let it = (api.cJSON_CreateNumber)(7.0);
        (*it).type_ = t;
        v.push((format!("type_0x{t:x}"), it));
    }
    // raw with a NULL valuestring
    let it = (api.cJSON_CreateNull)();
    (*it).type_ = CJSON_RAW;
    v.push(("raw_null_string".into(), it));
    // string with a NULL valuestring
    let it = (api.cJSON_CreateNull)();
    (*it).type_ = CJSON_STRING;
    v.push(("string_null_string".into(), it));
    // object with a NULL key
    let obj = (api.cJSON_CreateObject)();
    let child = (api.cJSON_CreateNumber)(3.0);
    (api.cJSON_AddItemToArray)(obj, child);
    v.push(("object_null_key".into(), obj));

    for (_, p) in &v {
        keep.push(*p);
    }
    v
}

unsafe fn free_zoo(api: &Api, keep: &[*mut CJson]) {
    for &p in keep {
        // put every hand-modified type back to something Delete can walk
        (api.cJSON_Delete)(p);
    }
}

#[test]
fn c25_c26_print_every_type() {
    diff("C25/C26 cJSON_Print / cJSON_PrintUnformatted per type", |api| unsafe {
        let mut log = String::new();
        let mut keep = Vec::new();
        for (name, it) in build_zoo(api, &mut keep) {
            let pf = take_print(api, (api.cJSON_Print)(it));
            let pu = take_print(api, (api.cJSON_PrintUnformatted)(it));
            let _ = writeln!(
                log,
                "{name}: fmt={} unfmt={}",
                pf.map(|v| show(&v)).unwrap_or("NULL".into()),
                pu.map(|v| show(&v)).unwrap_or("NULL".into())
            );
        }
        free_zoo(api, &keep);
        log
    });
}

#[test]
fn c27_c28_print_random_graphs() {
    diff("C27/C28 print random nested graphs", |api| unsafe {
        let mut log = String::new();
        let mut rng = Rng::new(0x0123_4567_89AB_CDEF);
        for round in 0..250 {
            let text = gen_json(&mut rng);
            let buf = CBuf::new(&text);
            let root = (api.cJSON_Parse)(buf.ptr());
            let pf = take_print(api, (api.cJSON_Print)(root));
            let pu = take_print(api, (api.cJSON_PrintUnformatted)(root));
            let _ = writeln!(
                log,
                "{round}: src={} fmt={} unfmt={}",
                show(&text),
                pf.map(|v| show(&v)).unwrap_or("NULL".into()),
                pu.map(|v| show(&v)).unwrap_or("NULL".into())
            );
            let _ = write!(log, "{}", dump(root));
            (api.cJSON_Delete)(root);
        }
        log
    });
}

#[test]
fn c29_print_number_magnitudes() {
    diff("C29 print numbers of every magnitude class", |api| unsafe {
        let mut log = String::new();
        let mut rng = Rng::new(0xFEED_FACE_CAFE_BEEF);
        let mut values: Vec<f64> = vec![
            0.0, -0.0, 1.0, -1.0, 1e-5, 1e15, 1e16, 1e17, 1e21, 1e22, 0.1, 0.3,
            1.0 / 3.0, 2.0 / 3.0, 123456789.123456789, 2147483647.0, 2147483648.0,
            -2147483648.0, -2147483649.0, 4294967296.0, 9007199254740992.0,
            9007199254740993.0, 1.7976931348623157e308, 5e-324, 2.2250738585072014e-308,
            f64::NAN, f64::INFINITY, f64::NEG_INFINITY, 1e308, -1e308,
            0.000001, 0.0000001, 1234567890123456.7,
        ];
        for _ in 0..500 {
            values.push(rng.nice_f64());
        }
        for _ in 0..300 {
            values.push(rng.any_f64());
        }
        // print them individually and as one big array (so `ensure` also grows)
        let arr = (api.cJSON_CreateArray)();
        for v in &values {
            let it = (api.cJSON_CreateNumber)(*v);
            let pu = take_print(api, (api.cJSON_PrintUnformatted)(it));
            let _ = writeln!(
                log,
                "0x{:016x} -> {} (valueint={})",
                v.to_bits(),
                pu.map(|x| show(&x)).unwrap_or("NULL".into()),
                (*it).valueint
            );
            (api.cJSON_AddItemToArray)(arr, it);
        }
        let pf = take_print(api, (api.cJSON_Print)(arr));
        let pu = take_print(api, (api.cJSON_PrintUnformatted)(arr));
        let _ = writeln!(log, "array_fmt={}", pf.map(|v| show(&v)).unwrap_or("NULL".into()));
        let _ = writeln!(log, "array_unfmt={}", pu.map(|v| show(&v)).unwrap_or("NULL".into()));
        (api.cJSON_Delete)(arr);
        log
    });
}

#[test]
fn c30_print_string_escapes() {
    diff("C30 print strings of every escape class", |api| unsafe {
        let mut log = String::new();
        let mut rng = Rng::new(0x7777_8888_9999_AAAA);
        let mut cases: Vec<Vec<u8>> = Vec::new();
        for b in 1u8..=255 {
            cases.push(vec![b]);
            cases.push(vec![b'a', b, b'z']);
        }
        cases.push(b"".to_vec());
        cases.push(b"no escapes at all".to_vec());
        cases.push(b"\"\\\x08\x0c\n\r\t".to_vec());
        for _ in 0..300 {
            cases.push(rng.ascii(20));
        }
        let obj = (api.cJSON_CreateObject)();
        for c in &cases {
            let b = CBuf::new(c);
            let it = (api.cJSON_CreateString)(b.ptr());
            let pu = take_print(api, (api.cJSON_PrintUnformatted)(it));
            let pf = take_print(api, (api.cJSON_Print)(it));
            let _ = writeln!(
                log,
                "[{}] unfmt={} fmt={}",
                show(c),
                pu.map(|v| show(&v)).unwrap_or("NULL".into()),
                pf.map(|v| show(&v)).unwrap_or("NULL".into())
            );
            (api.cJSON_Delete)(it);
            // and the same bytes used as an object key
            let it = (api.cJSON_CreateNumber)(1.0);
            (api.cJSON_AddItemToObject)(obj, b.ptr(), it);
        }
        let pu = take_print(api, (api.cJSON_PrintUnformatted)(obj));
        let _ = writeln!(log, "keys_unfmt={}", pu.map(|v| show(&v)).unwrap_or("NULL".into()));
        let pf = take_print(api, (api.cJSON_Print)(obj));
        let _ = writeln!(log, "keys_fmt={}", pf.map(|v| show(&v)).unwrap_or("NULL".into()));
        (api.cJSON_Delete)(obj);
        log
    });
}

#[test]
fn c31_print_empties() {
    diff("C31 print empty containers", |api| unsafe {
        let mut log = String::new();
        for src in [
            "[]", "{}", "[[]]", "[{}]", "{\"a\":{}}", "{\"a\":[]}", "[[],[]]",
            "{\"\":\"\"}", "[[[[[]]]]]", "{\"a\":{\"b\":{\"c\":{}}}}",
        ] {
            let doc = cs(src);
            let root = (api.cJSON_Parse)(doc.as_ptr());
            let pf = take_print(api, (api.cJSON_Print)(root));
            let pu = take_print(api, (api.cJSON_PrintUnformatted)(root));
            let _ = writeln!(
                log,
                "{src}: fmt={} unfmt={}",
                pf.map(|v| show(&v)).unwrap_or("NULL".into()),
                pu.map(|v| show(&v)).unwrap_or("NULL".into())
            );
            (api.cJSON_Delete)(root);
        }
        log
    });
}

#[test]
fn c32_print_large_documents() {
    diff("C32 print documents forcing repeated ensure() growth", |api| unsafe {
        let mut log = String::new();
        for n in [1usize, 8, 30, 200, 2000, 20000] {
            let arr = (api.cJSON_CreateArray)();
            for i in 0..n {
                (api.cJSON_AddItemToArray)(arr, (api.cJSON_CreateNumber)(i as f64 + 0.25));
            }
            let pf = take_print(api, (api.cJSON_Print)(arr));
            let pu = take_print(api, (api.cJSON_PrintUnformatted)(arr));
            let _ = writeln!(
                log,
                "n={n} fmt_len={:?} unfmt_len={:?} fmt_hash={:?} unfmt_hash={:?}",
                pf.as_ref().map(|v| v.len()),
                pu.as_ref().map(|v| v.len()),
                pf.as_ref().map(|v| v.iter().fold(0u64, |a, &b| a
                    .wrapping_mul(1000003)
                    .wrapping_add(b as u64))),
                pu.as_ref().map(|v| v.iter().fold(0u64, |a, &b| a
                    .wrapping_mul(1000003)
                    .wrapping_add(b as u64))),
            );
            if n <= 30 {
                let _ = writeln!(log, "  fmt={}", pf.map(|v| show(&v)).unwrap_or_default());
                let _ = writeln!(log, "  unfmt={}", pu.map(|v| show(&v)).unwrap_or_default());
            }
            (api.cJSON_Delete)(arr);
        }
        // long strings, too
        for n in [255usize, 256, 257, 1000, 65536] {
            let s = CBuf::new(&vec![b'x'; n]);
            let it = (api.cJSON_CreateString)(s.ptr());
            let pu = take_print(api, (api.cJSON_PrintUnformatted)(it));
            let _ = writeln!(log, "string n={n} len={:?}", pu.as_ref().map(|v| v.len()));
            (api.cJSON_Delete)(it);
        }
        log
    });
}

#[test]
fn c33_c34_print_buffered() {
    diff("C33/C34 cJSON_PrintBuffered", |api| unsafe {
        let mut log = String::new();
        let mut keep = Vec::new();
        let zoo = build_zoo(api, &mut keep);
        for (name, it) in &zoo {
            for prebuffer in [0i32, 1, 2, 3, 16, 255, 256, 257, 4096] {
                for fmt in [0i32, 1, 2, -1, c_int::MIN] {
                    let p = take_print(api, (api.cJSON_PrintBuffered)(*it, prebuffer, fmt));
                    let _ = writeln!(
                        log,
                        "{name} prebuffer={prebuffer} fmt={fmt} -> {}",
                        p.map(|v| show(&v)).unwrap_or("NULL".into())
                    );
                }
            }
        }
        free_zoo(api, &keep);
        log
    });
}

#[test]
fn c35_c36_c37_print_preallocated() {
    diff("C35/C36/C37 cJSON_PrintPreallocated", |api| unsafe {
        let mut log = String::new();
        let mut keep = Vec::new();
        let zoo = build_zoo(api, &mut keep);
        for (name, it) in &zoo {
            // reference length from cJSON_Print
            let reference = take_print(api, (api.cJSON_PrintUnformatted)(*it));
            let base = reference.as_ref().map(|v| v.len()).unwrap_or(0) as i32;
            for format in [0i32, 1, 2] {
                for delta in [-base - 1, -base, -base / 2, -2, -1, 0, 1, 5, 64] {
                    let len = base + delta;
                    if len < 0 {
                        continue;
                    }
                    let mut buf = vec![0xAAu8; (len as usize) + 8];
                    let rc = (api.cJSON_PrintPreallocated)(
                        *it,
                        buf.as_mut_ptr() as *mut c_char,
                        len,
                        format,
                    );
                    let _ = writeln!(
                        log,
                        "{name} format={format} len={len} rc={rc} buffer={}",
                        show(&buf)
                    );
                }
            }
        }
        free_zoo(api, &keep);
        log
    });
}

#[test]
fn c35_c36_preallocated_random() {
    diff("C35/C36 preallocated with random documents", |api| unsafe {
        let mut log = String::new();
        let mut rng = Rng::new(0x2222_3333_4444_5555);
        for round in 0..120 {
            let text = gen_json(&mut rng);
            let cbuf = CBuf::new(&text);
            let root = (api.cJSON_Parse)(cbuf.ptr());
            let reference = take_print(api, (api.cJSON_Print)(root));
            let base = reference.as_ref().map(|v| v.len()).unwrap_or(0) as i32;
            for format in [0i32, 1] {
                for len in [0, 1, base / 2, base - 1, base, base + 1, base + 5, base + 100] {
                    if len < 0 {
                        continue;
                    }
                    let mut buf = vec![0x5Au8; (len as usize) + 4];
                    let rc = (api.cJSON_PrintPreallocated)(
                        root,
                        buf.as_mut_ptr() as *mut c_char,
                        len,
                        format,
                    );
                    let _ = writeln!(
                        log,
                        "{round} format={format} len={len} rc={rc} buf={}",
                        show(&buf)
                    );
                }
            }
            (api.cJSON_Delete)(root);
        }
        log
    });
}
