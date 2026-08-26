//! Phase B — rows C1..C24: hooks, constructors, and the item-building API.
#![allow(non_snake_case)]

mod common;
use common::*;

use std::ffi::{c_char, c_int};
use std::fmt::Write as _;
use std::ptr::{null, null_mut};

#[test]
fn c1_version() {
    diff("C1 cJSON_Version", |api| unsafe {
        let v = (api.cJSON_Version)();
        let a = read_cstr(v).unwrap();
        let v2 = (api.cJSON_Version)();
        let b = read_cstr(v2).unwrap();
        format!(
            "version={} again={} same_ptr={}\n",
            show(&a),
            show(&b),
            v == v2
        )
    });
}

#[test]
fn c2_malloc_free() {
    diff("C2 cJSON_malloc/cJSON_free", |api| unsafe {
        let mut log = String::new();
        for size in [0usize, 1, 8, 64, 4096] {
            let p = (api.cJSON_malloc)(size);
            let _ = writeln!(log, "malloc({size}) null={}", p.is_null());
            if !p.is_null() && size > 0 {
                std::ptr::write_bytes(p as *mut u8, 0xAB, size);
                let _ = writeln!(log, "  first byte = {}", *(p as *const u8));
            }
            (api.cJSON_free)(p);
        }
        (api.cJSON_free)(null_mut());
        let _ = writeln!(log, "free(NULL) survived");
        log
    });
}

#[test]
fn c7_create_scalars() {
    diff("C7 cJSON_Create{Null,True,False,Array,Object}", |api| unsafe {
        let mut log = String::new();
        let items: [(&str, *mut CJson); 5] = [
            ("null", (api.cJSON_CreateNull)()),
            ("true", (api.cJSON_CreateTrue)()),
            ("false", (api.cJSON_CreateFalse)()),
            ("array", (api.cJSON_CreateArray)()),
            ("object", (api.cJSON_CreateObject)()),
        ];
        for (name, it) in items {
            let _ = writeln!(log, "{name}:\n{}", dump(it));
            let pf = take_print(api, (api.cJSON_Print)(it));
            let pu = take_print(api, (api.cJSON_PrintUnformatted)(it));
            let _ = writeln!(
                log,
                "  fmt={} unfmt={}",
                pf.map(|v| show(&v)).unwrap_or("NULL".into()),
                pu.map(|v| show(&v)).unwrap_or("NULL".into())
            );
            (api.cJSON_Delete)(it);
        }
        log
    });
}

#[test]
fn c8_create_bool_out_of_range() {
    diff("C8 cJSON_CreateBool out-of-range cJSON_bool", |api| unsafe {
        let mut log = String::new();
        for b in [0, 1, 2, -1, 256, 0x1_0000, c_int::MIN, c_int::MAX] {
            let it = (api.cJSON_CreateBool)(b);
            let _ = writeln!(log, "bool({b}):\n{}", dump(it));
            let p = take_print(api, (api.cJSON_PrintUnformatted)(it));
            let _ = writeln!(log, "  print={}", p.map(|v| show(&v)).unwrap_or("NULL".into()));
            (api.cJSON_Delete)(it);
        }
        log
    });
}

fn number_classes() -> Vec<f64> {
    vec![
        0.0,
        -0.0,
        1.0,
        -1.0,
        0.5,
        -0.5,
        1.0 / 3.0,
        2147483647.0,
        2147483646.5,
        2147483648.0,
        -2147483648.0,
        -2147483649.0,
        -2147483647.5,
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::NAN,
        -f64::NAN,
        f64::MIN_POSITIVE,
        5e-324,
        f64::MAX,
        f64::MIN,
        1e300,
        1e-300,
        1e15,
        1e16,
        1e17,
        123456789012345.6,
        0.1,
        0.2,
        0.3,
        1e21,
        1e-7,
        3.141592653589793,
        2.718281828459045,
        9007199254740993.0,
        1.7976931348623157e308,
    ]
}

#[test]
fn c9_create_number() {
    diff("C9 cJSON_CreateNumber", |api| unsafe {
        let mut log = String::new();
        let mut rng = Rng::new(0x9E37_79B9_7F4A_7C15);
        let mut values = number_classes();
        for _ in 0..400 {
            values.push(rng.nice_f64());
        }
        for _ in 0..200 {
            values.push(rng.any_f64());
        }
        for v in values {
            let it = (api.cJSON_CreateNumber)(v);
            let _ = write!(log, "num 0x{:016x}: {}", v.to_bits(), dump(it));
            let pf = take_print(api, (api.cJSON_Print)(it));
            let pu = take_print(api, (api.cJSON_PrintUnformatted)(it));
            let gn = (api.cJSON_GetNumberValue)(it);
            let _ = writeln!(
                log,
                "  fmt={} unfmt={} getnum=0x{:016x}",
                pf.map(|v| show(&v)).unwrap_or("NULL".into()),
                pu.map(|v| show(&v)).unwrap_or("NULL".into()),
                gn.to_bits()
            );
            (api.cJSON_Delete)(it);
        }
        log
    });
}

fn string_classes() -> Vec<Vec<u8>> {
    let mut v: Vec<Vec<u8>> = vec![
        b"".to_vec(),
        b"a".to_vec(),
        b"hello world".to_vec(),
        b"\"quoted\"".to_vec(),
        b"back\\slash".to_vec(),
        b"tab\there".to_vec(),
        b"nl\nhere".to_vec(),
        b"cr\rhere".to_vec(),
        vec![b'b', 0x08, b'e'],
        vec![b'f', 0x0c, b'e'],
        b"slash/es".to_vec(),
        b"\x7f".to_vec(),
        "é€😀".as_bytes().to_vec(),
        vec![0x80, 0xff, 0xfe, 0xc3],
    ];
    // every control byte on its own
    for b in 1u8..=31 {
        v.push(vec![b'x', b, b'y']);
    }
    // long string forcing buffer growth
    v.push(vec![b'L'; 1000]);
    v
}

#[test]
fn c10_create_string() {
    diff("C10 cJSON_CreateString", |api| unsafe {
        let mut log = String::new();
        let mut rng = Rng::new(0xDEAD_BEEF_1234_5678);
        let mut values = string_classes();
        for _ in 0..300 {
            values.push(rng.ascii(24));
        }
        for s in values {
            let buf = CBuf::new(&s);
            let it = (api.cJSON_CreateString)(buf.ptr());
            let _ = write!(log, "str[{}]: {}", show(&s), dump(it));
            let pf = take_print(api, (api.cJSON_Print)(it));
            let pu = take_print(api, (api.cJSON_PrintUnformatted)(it));
            let gs = read_cstr((api.cJSON_GetStringValue)(it));
            let _ = writeln!(
                log,
                "  fmt={} unfmt={} getstr={:?}",
                pf.map(|v| show(&v)).unwrap_or("NULL".into()),
                pu.map(|v| show(&v)).unwrap_or("NULL".into()),
                gs.map(|v| show(&v))
            );
            (api.cJSON_Delete)(it);
        }
        log
    });
}

#[test]
fn c11_create_string_reference() {
    diff("C11 cJSON_CreateStringReference", |api| unsafe {
        let mut log = String::new();
        for s in [&b""[..], b"referenced", b"with \"escapes\"\n"] {
            let buf = CBuf::new(s);
            let it = (api.cJSON_CreateStringReference)(buf.ptr());
            let _ = write!(log, "ref[{}]: {}", show(s), dump(it));
            let _ = writeln!(
                log,
                "  aliases_input={} is_string={} is_ref={}",
                (*it).valuestring as *const c_char == buf.ptr(),
                (api.cJSON_IsString)(it),
                (*it).type_ & CJSON_IS_REFERENCE
            );
            let p = take_print(api, (api.cJSON_PrintUnformatted)(it));
            let _ = writeln!(log, "  print={}", p.map(|v| show(&v)).unwrap_or("NULL".into()));
            // must not free the referenced buffer
            (api.cJSON_Delete)(it);
            let _ = writeln!(log, "  buffer intact after delete: {}", show(&buf.0[..buf.len()]));
        }
        log
    });
}

#[test]
fn c12_create_raw() {
    diff("C12 cJSON_CreateRaw", |api| unsafe {
        let mut log = String::new();
        for s in [&b""[..], b"[1,2]", b"garbage", b"{\"not\":\"escaped\\n\"}"] {
            let buf = CBuf::new(s);
            let it = (api.cJSON_CreateRaw)(buf.ptr());
            let _ = write!(log, "raw[{}]: {}", show(s), dump(it));
            let pf = take_print(api, (api.cJSON_Print)(it));
            let pu = take_print(api, (api.cJSON_PrintUnformatted)(it));
            let _ = writeln!(
                log,
                "  fmt={} unfmt={} is_raw={}",
                pf.map(|v| show(&v)).unwrap_or("NULL".into()),
                pu.map(|v| show(&v)).unwrap_or("NULL".into()),
                (api.cJSON_IsRaw)(it)
            );
            (api.cJSON_Delete)(it);
        }
        log
    });
}

#[test]
fn c13_container_references() {
    diff("C13 cJSON_Create{Object,Array}Reference", |api| unsafe {
        let mut log = String::new();
        let doc = cs("{\"a\":1,\"b\":[2,3]}");
        let obj = (api.cJSON_Parse)(doc.as_ptr());
        let arr_doc = cs("[10,20,30]");
        let arr = (api.cJSON_Parse)(arr_doc.as_ptr());

        let oref = (api.cJSON_CreateObjectReference)((*obj).child);
        let aref = (api.cJSON_CreateArrayReference)((*arr).child);
        let _ = write!(log, "oref: {}", dump(oref));
        let _ = write!(log, "aref: {}", dump(aref));
        for (name, it) in [("oref", oref), ("aref", aref)] {
            let pf = take_print(api, (api.cJSON_Print)(it));
            let pu = take_print(api, (api.cJSON_PrintUnformatted)(it));
            let _ = writeln!(
                log,
                "{name}: fmt={} unfmt={}",
                pf.map(|v| show(&v)).unwrap_or("NULL".into()),
                pu.map(|v| show(&v)).unwrap_or("NULL".into())
            );
        }
        // deleting the references must not free the referenced children
        (api.cJSON_Delete)(oref);
        (api.cJSON_Delete)(aref);
        let _ = write!(log, "obj after: {}", dump(obj));
        let _ = write!(log, "arr after: {}", dump(arr));
        (api.cJSON_Delete)(obj);
        (api.cJSON_Delete)(arr);
        log
    });
}

#[test]
fn c14_create_int_array() {
    diff("C14 cJSON_CreateIntArray", |api| unsafe {
        let mut log = String::new();
        let mut rng = Rng::new(0x1111_2222_3333_4444);
        for count in [0i32, 1, 2, 7, 64] {
            for round in 0..8 {
                let mut nums: Vec<c_int> = Vec::new();
                for i in 0..count.max(0) as usize {
                    nums.push(match (round, i % 4) {
                        (0, _) => i as c_int,
                        (1, 0) => c_int::MAX,
                        (1, 1) => c_int::MIN,
                        (1, 2) => 0,
                        (1, _) => -1,
                        _ => rng.range_i32(c_int::MIN, c_int::MAX),
                    });
                }
                let ptr = if nums.is_empty() {
                    // still a valid, non-NULL pointer
                    nums.reserve(1);
                    nums.as_ptr()
                } else {
                    nums.as_ptr()
                };
                let a = (api.cJSON_CreateIntArray)(ptr, count);
                let _ = write!(log, "int count={count} round={round}: {}", dump(a));
                let p = take_print(api, (api.cJSON_PrintUnformatted)(a));
                let _ = writeln!(
                    log,
                    "  size={} print={}",
                    (api.cJSON_GetArraySize)(a),
                    p.map(|v| show(&v)).unwrap_or("NULL".into())
                );
                (api.cJSON_Delete)(a);
            }
        }
        log
    });
}

#[test]
fn c15_create_float_array() {
    diff("C15 cJSON_CreateFloatArray", |api| unsafe {
        let mut log = String::new();
        let mut rng = Rng::new(0x5555_6666_7777_8888);
        let specials: Vec<f32> = vec![
            0.0,
            -0.0,
            1.0,
            -1.0,
            0.1,
            f32::INFINITY,
            f32::NEG_INFINITY,
            f32::NAN,
            f32::MAX,
            f32::MIN,
            f32::MIN_POSITIVE,
            1e-45,
            16777217.0,
        ];
        for count in [0usize, 1, 3, 13, 32] {
            for round in 0..4 {
                let mut nums: Vec<f32> = Vec::with_capacity(count.max(1));
                for i in 0..count {
                    nums.push(if round == 0 {
                        specials[i % specials.len()]
                    } else {
                        f32::from_bits(rng.next_u64() as u32)
                    });
                }
                nums.reserve(1);
                let a = (api.cJSON_CreateFloatArray)(nums.as_ptr(), count as c_int);
                let _ = write!(log, "float count={count} round={round}: {}", dump(a));
                let p = take_print(api, (api.cJSON_PrintUnformatted)(a));
                let _ = writeln!(log, "  print={}", p.map(|v| show(&v)).unwrap_or("NULL".into()));
                (api.cJSON_Delete)(a);
            }
        }
        log
    });
}

#[test]
fn c16_create_double_array() {
    diff("C16 cJSON_CreateDoubleArray", |api| unsafe {
        let mut log = String::new();
        let mut rng = Rng::new(0x99AA_BBCC_DDEE_FF00);
        let specials = number_classes();
        for count in [0usize, 1, 3, 13, 32] {
            for round in 0..4 {
                let mut nums: Vec<f64> = Vec::with_capacity(count.max(1));
                for i in 0..count {
                    nums.push(if round == 0 {
                        specials[i % specials.len()]
                    } else if round == 1 {
                        rng.nice_f64()
                    } else {
                        rng.any_f64()
                    });
                }
                nums.reserve(1);
                let a = (api.cJSON_CreateDoubleArray)(nums.as_ptr(), count as c_int);
                let _ = write!(log, "double count={count} round={round}: {}", dump(a));
                let p = take_print(api, (api.cJSON_PrintUnformatted)(a));
                let _ = writeln!(log, "  print={}", p.map(|v| show(&v)).unwrap_or("NULL".into()));
                (api.cJSON_Delete)(a);
            }
        }
        log
    });
}

#[test]
fn c17_create_string_array() {
    diff("C17 cJSON_CreateStringArray", |api| unsafe {
        let mut log = String::new();
        let mut rng = Rng::new(0x0F0F_0F0F_F0F0_F0F0);
        let classes = string_classes();
        for count in [0usize, 1, 7, 32] {
            for round in 0..4 {
                let mut owned: Vec<CBuf> = Vec::new();
                for i in 0..count {
                    owned.push(CBuf::new(&if round == 0 {
                        classes[i % classes.len()].clone()
                    } else {
                        rng.ascii(16)
                    }));
                }
                let ptrs: Vec<*const c_char> = owned.iter().map(|b| b.ptr()).collect();
                let mut ptrs = ptrs;
                ptrs.reserve(1);
                let a = (api.cJSON_CreateStringArray)(ptrs.as_ptr(), count as c_int);
                let _ = write!(log, "strarr count={count} round={round}: {}", dump(a));
                let p = take_print(api, (api.cJSON_PrintUnformatted)(a));
                let _ = writeln!(log, "  print={}", p.map(|v| show(&v)).unwrap_or("NULL".into()));
                (api.cJSON_Delete)(a);
            }
        }
        log
    });
}

#[test]
fn c18_add_item_to_array() {
    diff("C18 cJSON_AddItemToArray", |api| unsafe {
        let mut log = String::new();
        let arr = (api.cJSON_CreateArray)();
        for i in 0..6 {
            let it = (api.cJSON_CreateNumber)(i as f64);
            let rc = (api.cJSON_AddItemToArray)(arr, it);
            let _ = write!(log, "after add {i} (rc={rc}), size={}: {}",
                (api.cJSON_GetArraySize)(arr), dump(arr));
            let p = take_print(api, (api.cJSON_PrintUnformatted)(arr));
            let _ = writeln!(log, "  print={}", p.map(|v| show(&v)).unwrap_or("NULL".into()));
        }
        // append an array and an object as elements
        let inner = (api.cJSON_CreateArray)();
        let _ = writeln!(log, "add inner array rc={}", (api.cJSON_AddItemToArray)(arr, inner));
        let obj = (api.cJSON_CreateObject)();
        let _ = writeln!(log, "add object rc={}", (api.cJSON_AddItemToArray)(arr, obj));
        let _ = write!(log, "final: {}", dump(arr));
        let pf = take_print(api, (api.cJSON_Print)(arr));
        let _ = writeln!(log, "fmt={}", pf.map(|v| show(&v)).unwrap_or("NULL".into()));
        (api.cJSON_Delete)(arr);
        log
    });
}

#[test]
fn c19_add_item_to_object() {
    diff("C19 cJSON_AddItemToObject", |api| unsafe {
        let mut log = String::new();
        let obj = (api.cJSON_CreateObject)();
        let keys: Vec<Vec<u8>> = vec![
            b"a".to_vec(),
            b"".to_vec(),
            b"with \"quote\"".to_vec(),
            b"tab\there".to_vec(),
            b"a".to_vec(), // duplicate key
            vec![b'K'; 300],
            vec![1u8, 2, 3],
        ];
        for (i, k) in keys.iter().enumerate() {
            let kb = CBuf::new(k);
            let it = (api.cJSON_CreateNumber)(i as f64);
            let rc = (api.cJSON_AddItemToObject)(obj, kb.ptr(), it);
            let _ = writeln!(log, "add key[{}] rc={rc}", show(k));
        }
        let _ = write!(log, "graph: {}", dump(obj));
        let pf = take_print(api, (api.cJSON_Print)(obj));
        let pu = take_print(api, (api.cJSON_PrintUnformatted)(obj));
        let _ = writeln!(
            log,
            "fmt={}\nunfmt={}",
            pf.map(|v| show(&v)).unwrap_or("NULL".into()),
            pu.map(|v| show(&v)).unwrap_or("NULL".into())
        );
        // lookups
        for k in ["a", "A", "", "missing"] {
            let kb = cs(k);
            let ci = (api.cJSON_GetObjectItem)(obj, kb.as_ptr());
            let cse = (api.cJSON_GetObjectItemCaseSensitive)(obj, kb.as_ptr());
            let _ = writeln!(
                log,
                "lookup {k:?}: insens_int={:?} sens_int={:?} has={}",
                (!ci.is_null()).then(|| (*ci).valueint),
                (!cse.is_null()).then(|| (*cse).valueint),
                (api.cJSON_HasObjectItem)(obj, kb.as_ptr())
            );
        }
        (api.cJSON_Delete)(obj);
        log
    });
}

#[test]
fn c20_c21_add_item_to_object_cs() {
    diff("C20/C21 cJSON_AddItemToObjectCS", |api| unsafe {
        let mut log = String::new();
        let obj = (api.cJSON_CreateObject)();
        let k1 = cs("const_key");
        let k2 = cs("heap_key");

        // 1. const key on a fresh item
        let a = (api.cJSON_CreateNumber)(1.0);
        let _ = writeln!(log, "CS add rc={}", (api.cJSON_AddItemToObjectCS)(obj, k1.as_ptr(), a));
        let _ = writeln!(log, "  type=0x{:x} const_flag={}", (*a).type_, (*a).type_ & CJSON_STRING_IS_CONST);

        // 2. item that already has a heap key gets a const key (old key freed)
        let b = (api.cJSON_CreateNumber)(2.0);
        let _ = writeln!(log, "heap add rc={}", (api.cJSON_AddItemToObject)(obj, k2.as_ptr(), b));
        let detached = (api.cJSON_DetachItemViaPointer)(obj, b);
        let _ = writeln!(log, "  detached={}", !detached.is_null());
        let _ = writeln!(log, "  CS re-add rc={}", (api.cJSON_AddItemToObjectCS)(obj, k1.as_ptr(), b));
        let _ = writeln!(log, "  type=0x{:x}", (*b).type_);

        // 3. item with a const key gets a heap key (old key must NOT be freed)
        let c = (api.cJSON_CreateNumber)(3.0);
        let _ = writeln!(log, "CS add c rc={}", (api.cJSON_AddItemToObjectCS)(obj, k1.as_ptr(), c));
        let d = (api.cJSON_DetachItemViaPointer)(obj, c);
        let _ = writeln!(log, "  detached={}", !d.is_null());
        let _ = writeln!(log, "  heap re-add rc={}", (api.cJSON_AddItemToObject)(obj, k2.as_ptr(), c));
        let _ = writeln!(log, "  type=0x{:x} const_flag={}", (*c).type_, (*c).type_ & CJSON_STRING_IS_CONST);

        let _ = write!(log, "graph: {}", dump(obj));
        let p = take_print(api, (api.cJSON_Print)(obj));
        let _ = writeln!(log, "print={}", p.map(|v| show(&v)).unwrap_or("NULL".into()));
        (api.cJSON_Delete)(obj);
        let _ = writeln!(log, "const key still readable: {}", show(k1.as_bytes()));
        log
    });
}

#[test]
fn c22_c23_item_references() {
    diff("C22/C23 cJSON_AddItemReferenceTo{Array,Object}", |api| unsafe {
        let mut log = String::new();
        let doc = cs("{\"orig\":[1,2,3]}");
        let owner = (api.cJSON_Parse)(doc.as_ptr());
        let key = cs("orig");
        let target = (api.cJSON_GetObjectItem)(owner, key.as_ptr());

        let arr = (api.cJSON_CreateArray)();
        let _ = writeln!(log, "ref->array rc={}", (api.cJSON_AddItemReferenceToArray)(arr, target));
        let obj = (api.cJSON_CreateObject)();
        let rkey = cs("ref");
        let _ = writeln!(
            log,
            "ref->object rc={}",
            (api.cJSON_AddItemReferenceToObject)(obj, rkey.as_ptr(), target)
        );
        let _ = write!(log, "arr: {}", dump(arr));
        let _ = write!(log, "obj: {}", dump(obj));
        for (n, it) in [("arr", arr), ("obj", obj)] {
            let p = take_print(api, (api.cJSON_Print)(it));
            let _ = writeln!(log, "{n} print={}", p.map(|v| show(&v)).unwrap_or("NULL".into()));
        }
        (api.cJSON_Delete)(arr);
        (api.cJSON_Delete)(obj);
        let _ = write!(log, "owner survived: {}", dump(owner));
        (api.cJSON_Delete)(owner);
        log
    });
}

#[test]
fn c24_add_helpers() {
    diff("C24 cJSON_Add*ToObject helpers", |api| unsafe {
        let mut log = String::new();
        let obj = (api.cJSON_CreateObject)();
        let n = cs("n");
        let t = cs("t");
        let f = cs("f");
        let b0 = cs("b0");
        let b1 = cs("b1");
        let b2 = cs("b2");
        let num = cs("num");
        let st = cs("st");
        let raw = cs("raw");
        let o = cs("o");
        let a = cs("a");
        let sval = cs("string \"value\"\n");
        let rval = cs("[1,2]");

        let _ = writeln!(log, "null   = {}", !(api.cJSON_AddNullToObject)(obj, n.as_ptr()).is_null());
        let _ = writeln!(log, "true   = {}", !(api.cJSON_AddTrueToObject)(obj, t.as_ptr()).is_null());
        let _ = writeln!(log, "false  = {}", !(api.cJSON_AddFalseToObject)(obj, f.as_ptr()).is_null());
        let _ = writeln!(log, "bool0  = {}", !(api.cJSON_AddBoolToObject)(obj, b0.as_ptr(), 0).is_null());
        let _ = writeln!(log, "bool1  = {}", !(api.cJSON_AddBoolToObject)(obj, b1.as_ptr(), 1).is_null());
        let _ = writeln!(log, "bool2  = {}", !(api.cJSON_AddBoolToObject)(obj, b2.as_ptr(), 2).is_null());
        let _ = writeln!(log, "number = {}", !(api.cJSON_AddNumberToObject)(obj, num.as_ptr(), 1.25).is_null());
        let _ = writeln!(log, "string = {}", !(api.cJSON_AddStringToObject)(obj, st.as_ptr(), sval.as_ptr()).is_null());
        let _ = writeln!(log, "raw    = {}", !(api.cJSON_AddRawToObject)(obj, raw.as_ptr(), rval.as_ptr()).is_null());
        let sub = (api.cJSON_AddObjectToObject)(obj, o.as_ptr());
        let _ = writeln!(log, "object = {}", !sub.is_null());
        let subarr = (api.cJSON_AddArrayToObject)(obj, a.as_ptr());
        let _ = writeln!(log, "array  = {}", !subarr.is_null());
        // and nest into the freshly created containers
        let inner = cs("inner");
        let _ = writeln!(log, "nested = {}", !(api.cJSON_AddNumberToObject)(sub, inner.as_ptr(), 42.0).is_null());
        let _ = writeln!(log, "nested arr rc={}", (api.cJSON_AddItemToArray)(subarr, (api.cJSON_CreateNull)()));

        let _ = write!(log, "graph: {}", dump(obj));
        let pf = take_print(api, (api.cJSON_Print)(obj));
        let pu = take_print(api, (api.cJSON_PrintUnformatted)(obj));
        let _ = writeln!(
            log,
            "fmt={}\nunfmt={}",
            pf.map(|v| show(&v)).unwrap_or("NULL".into()),
            pu.map(|v| show(&v)).unwrap_or("NULL".into())
        );
        (api.cJSON_Delete)(obj);
        log
    });
}

/* keep `null` import used even if a scenario changes */
#[allow(unused)]
fn _unused() -> *const c_char {
    null()
}
