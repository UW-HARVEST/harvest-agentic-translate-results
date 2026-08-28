//! Lowest level: version string, allocator wrappers, type predicates,
//! and the `cJSON_Create*` constructors.
mod common;

use common::*;
use std::os::raw::{c_char, c_double, c_float, c_int};

#[test]
fn version_string() {
    let _guard = serial();
    let a = apis();
    unsafe {
        let c = cstr_bytes(a.c.cJSON_Version()).unwrap();
        let r = cstr_bytes(a.rust.cJSON_Version()).unwrap();
        assert_eq!(c, r, "cJSON_Version mismatch");
        assert_eq!(c, b"1.7.19".to_vec());
    }
}

#[test]
fn malloc_free_roundtrip() {
    let _guard = serial();
    let a = apis();
    unsafe {
        for size in [0usize, 1, 7, 64, 1024, 65536] {
            let pc = a.c.cJSON_malloc(size);
            let pr = a.rust.cJSON_malloc(size);
            assert_eq!(pc.is_null(), pr.is_null(), "cJSON_malloc({size}) nullness");
            if !pc.is_null() {
                std::ptr::write_bytes(pc as *mut u8, 0xAB, size);
            }
            if !pr.is_null() {
                std::ptr::write_bytes(pr as *mut u8, 0xAB, size);
            }
            a.c.cJSON_free(pc);
            a.rust.cJSON_free(pr);
        }
        // free(NULL) must be safe in both
        a.c.cJSON_free(std::ptr::null_mut());
        a.rust.cJSON_free(std::ptr::null_mut());
    }
}

/// Every predicate applied to every kind of item, plus NULL.
#[test]
fn type_predicates() {
    let _guard = serial();
    let a = apis();
    unsafe {
        let preds: [(&str, fn(&Api, *const cJSON) -> cJSON_bool); 10] = [
            ("IsInvalid", |p, i| p.cJSON_IsInvalid(i)),
            ("IsFalse", |p, i| p.cJSON_IsFalse(i)),
            ("IsTrue", |p, i| p.cJSON_IsTrue(i)),
            ("IsBool", |p, i| p.cJSON_IsBool(i)),
            ("IsNull", |p, i| p.cJSON_IsNull(i)),
            ("IsNumber", |p, i| p.cJSON_IsNumber(i)),
            ("IsString", |p, i| p.cJSON_IsString(i)),
            ("IsArray", |p, i| p.cJSON_IsArray(i)),
            ("IsObject", |p, i| p.cJSON_IsObject(i)),
            ("IsRaw", |p, i| p.cJSON_IsRaw(i)),
        ];

        // synthesise items with every possible `type` value, including the
        // reference / const-string flag bits.
        for base in [
            cJSON_Invalid,
            cJSON_False,
            cJSON_True,
            cJSON_NULL,
            cJSON_Number,
            cJSON_String,
            cJSON_Array,
            cJSON_Object,
            cJSON_Raw,
            cJSON_False | cJSON_True,
            0xFF,
            -1,
        ] {
            for extra in [0, cJSON_IsReference, cJSON_StringIsConst] {
                let mut item = cJSON {
                    next: std::ptr::null_mut(),
                    prev: std::ptr::null_mut(),
                    child: std::ptr::null_mut(),
                    type_: base | extra,
                    valuestring: std::ptr::null_mut(),
                    valueint: 0,
                    valuedouble: 0.0,
                    string: std::ptr::null_mut(),
                };
                let p: *const cJSON = &mut item;
                for (name, f) in preds.iter() {
                    let cv = f(&a.c, p);
                    let rv = f(&a.rust, p);
                    assert_eq!(cv, rv, "{name}(type={}) mismatch", base | extra);
                }
                // Getters
                assert_eq!(
                    a.c.cJSON_GetStringValue(p).is_null(),
                    a.rust.cJSON_GetStringValue(p).is_null(),
                    "GetStringValue(type={})",
                    base | extra
                );
                let cn = a.c.cJSON_GetNumberValue(p);
                let rn = a.rust.cJSON_GetNumberValue(p);
                assert_eq!(
                    cn.is_nan() && rn.is_nan() || cn == rn,
                    true,
                    "GetNumberValue(type={}) {cn} vs {rn}",
                    base | extra
                );
            }
        }

        // NULL handling
        for (name, f) in preds.iter() {
            assert_eq!(
                f(&a.c, std::ptr::null()),
                f(&a.rust, std::ptr::null()),
                "{name}(NULL) mismatch"
            );
        }
        assert_eq!(
            a.c.cJSON_GetStringValue(std::ptr::null()).is_null(),
            a.rust.cJSON_GetStringValue(std::ptr::null()).is_null()
        );
        let cn = a.c.cJSON_GetNumberValue(std::ptr::null());
        let rn = a.rust.cJSON_GetNumberValue(std::ptr::null());
        assert!(cn.is_nan() && rn.is_nan(), "GetNumberValue(NULL): {cn} {rn}");
    }
}

#[test]
fn create_simple_items() {
    let _guard = serial();
    let a = apis();
    unsafe {
        macro_rules! both {
            ($ctx:expr, $call:expr) => {{
                let cp = { let api = &a.c; $call(api) };
                let rp = { let api = &a.rust; $call(api) };
                assert_tree_eq($ctx, cp, rp);
                a.c.cJSON_Delete(cp);
                a.rust.cJSON_Delete(rp);
            }};
        }

        both!("CreateNull", |p: &Api| p.cJSON_CreateNull());
        both!("CreateTrue", |p: &Api| p.cJSON_CreateTrue());
        both!("CreateFalse", |p: &Api| p.cJSON_CreateFalse());
        both!("CreateBool(0)", |p: &Api| p.cJSON_CreateBool(0));
        both!("CreateBool(1)", |p: &Api| p.cJSON_CreateBool(1));
        both!("CreateBool(42)", |p: &Api| p.cJSON_CreateBool(42));
        both!("CreateBool(-1)", |p: &Api| p.cJSON_CreateBool(-1));
        both!("CreateArray", |p: &Api| p.cJSON_CreateArray());
        both!("CreateObject", |p: &Api| p.cJSON_CreateObject());
    }
}

/// `cJSON_CreateNumber` has interesting clamping behaviour for `valueint`.
#[test]
fn create_number_edge_cases() {
    let _guard = serial();
    let a = apis();
    let numbers: Vec<c_double> = vec![
        0.0,
        -0.0,
        1.0,
        -1.0,
        0.5,
        -0.5,
        1e-7,
        1e17,
        1e300,
        -1e300,
        1.0 / 3.0,
        2.0 / 3.0,
        123456789.0,
        1234567890123456789.0,
        2147483647.0,
        2147483648.0,
        -2147483648.0,
        -2147483649.0,
        4294967296.0,
        f64::MAX,
        f64::MIN,
        f64::MIN_POSITIVE,
        f64::EPSILON,
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::NAN,
        60.0,
        1e21,
        1e-21,
        3.141592653589793,
        2.718281828459045,
        1e15,
        1e16,
        -1e16,
        0.1,
        0.2,
        0.3,
        1e9 + 0.5,
        9007199254740993.0,
        5e-324,
        1.7976931348623157e308,
    ];
    unsafe {
        for n in numbers {
            let cp = a.c.cJSON_CreateNumber(n);
            let rp = a.rust.cJSON_CreateNumber(n);
            assert_tree_eq(&format!("CreateNumber({n:?})"), cp, rp);
            a.c.cJSON_Delete(cp);
            a.rust.cJSON_Delete(rp);
        }
    }
}

pub const STRINGS: &[&str] = &[
    "",
    "a",
    "hello world",
    "with \"quotes\"",
    "back\\slash",
    "tab\there",
    "newline\nhere",
    "carriage\rreturn",
    "form\u{000c}feed",
    "back\u{0008}space",
    "bell\u{0007}",
    "\u{0001}\u{0002}\u{001f}",
    "unicode: \u{00e9}\u{00e8}\u{00ea}",
    "emoji: \u{1f600}",
    "cjk: \u{4f60}\u{597d}",
    "del\u{007f}",
    "slash/forward",
    "mixed \"a\\b\nc\td\u{0000}",
    "0123456789012345678901234567890123456789012345678901234567890123456789",
];

#[test]
fn create_string_and_raw() {
    let _guard = serial();
    let a = apis();
    unsafe {
        for s in STRINGS {
            // NUL inside a Rust &str would truncate the C string; skip those
            if s.contains('\0') {
                continue;
            }
            let cstr = cs(s);
            let cp = a.c.cJSON_CreateString(cstr.as_ptr());
            let rp = a.rust.cJSON_CreateString(cstr.as_ptr());
            assert_tree_eq(&format!("CreateString({s:?})"), cp, rp);
            a.c.cJSON_Delete(cp);
            a.rust.cJSON_Delete(rp);

            let cp = a.c.cJSON_CreateStringReference(cstr.as_ptr());
            let rp = a.rust.cJSON_CreateStringReference(cstr.as_ptr());
            assert_tree_eq(&format!("CreateStringReference({s:?})"), cp, rp);
            a.c.cJSON_Delete(cp);
            a.rust.cJSON_Delete(rp);
        }

        for s in ["null", "1234", "{\"a\":1}", "[1,2,3]", "", "not json at all"] {
            let cstr = cs(s);
            let cp = a.c.cJSON_CreateRaw(cstr.as_ptr());
            let rp = a.rust.cJSON_CreateRaw(cstr.as_ptr());
            assert_tree_eq(&format!("CreateRaw({s:?})"), cp, rp);
            a.c.cJSON_Delete(cp);
            a.rust.cJSON_Delete(rp);
        }

        // NULL inputs
        let cp = a.c.cJSON_CreateString(std::ptr::null());
        let rp = a.rust.cJSON_CreateString(std::ptr::null());
        assert_eq!(cp.is_null(), rp.is_null(), "CreateString(NULL)");
        a.c.cJSON_Delete(cp);
        a.rust.cJSON_Delete(rp);

        let cp = a.c.cJSON_CreateRaw(std::ptr::null());
        let rp = a.rust.cJSON_CreateRaw(std::ptr::null());
        assert_eq!(cp.is_null(), rp.is_null(), "CreateRaw(NULL)");
        a.c.cJSON_Delete(cp);
        a.rust.cJSON_Delete(rp);

        let cp = a.c.cJSON_CreateStringReference(std::ptr::null());
        let rp = a.rust.cJSON_CreateStringReference(std::ptr::null());
        assert_tree_eq("CreateStringReference(NULL)", cp, rp);
        a.c.cJSON_Delete(cp);
        a.rust.cJSON_Delete(rp);
    }
}

#[test]
fn create_typed_arrays() {
    let _guard = serial();
    let a = apis();
    unsafe {
        let ints: Vec<c_int> = vec![0, 1, -1, i32::MAX, i32::MIN, 42, -999999];
        let floats: Vec<c_float> = vec![
            0.0,
            -0.0,
            1.5,
            -1.5,
            f32::MAX,
            f32::MIN,
            f32::EPSILON,
            f32::INFINITY,
            f32::NEG_INFINITY,
            f32::NAN,
            1.0 / 3.0,
        ];
        let doubles: Vec<c_double> = vec![
            0.0,
            -0.0,
            1.0 / 3.0,
            1e300,
            -1e-300,
            f64::INFINITY,
            f64::NAN,
            2147483648.0,
        ];

        for count in 0..=(ints.len() as c_int) {
            let cp = a.c.cJSON_CreateIntArray(ints.as_ptr(), count);
            let rp = a.rust.cJSON_CreateIntArray(ints.as_ptr(), count);
            assert_tree_eq(&format!("CreateIntArray(count={count})"), cp, rp);
            a.c.cJSON_Delete(cp);
            a.rust.cJSON_Delete(rp);
        }
        for count in 0..=(floats.len() as c_int) {
            let cp = a.c.cJSON_CreateFloatArray(floats.as_ptr(), count);
            let rp = a.rust.cJSON_CreateFloatArray(floats.as_ptr(), count);
            assert_tree_eq(&format!("CreateFloatArray(count={count})"), cp, rp);
            a.c.cJSON_Delete(cp);
            a.rust.cJSON_Delete(rp);
        }
        for count in 0..=(doubles.len() as c_int) {
            let cp = a.c.cJSON_CreateDoubleArray(doubles.as_ptr(), count);
            let rp = a.rust.cJSON_CreateDoubleArray(doubles.as_ptr(), count);
            assert_tree_eq(&format!("CreateDoubleArray(count={count})"), cp, rp);
            a.c.cJSON_Delete(cp);
            a.rust.cJSON_Delete(rp);
        }

        // negative counts must be rejected identically
        for count in [-1, -5, i32::MIN] {
            let cp = a.c.cJSON_CreateIntArray(ints.as_ptr(), count);
            let rp = a.rust.cJSON_CreateIntArray(ints.as_ptr(), count);
            assert_eq!(cp.is_null(), rp.is_null(), "CreateIntArray({count})");
            a.c.cJSON_Delete(cp);
            a.rust.cJSON_Delete(rp);

            let cp = a.c.cJSON_CreateFloatArray(floats.as_ptr(), count);
            let rp = a.rust.cJSON_CreateFloatArray(floats.as_ptr(), count);
            assert_eq!(cp.is_null(), rp.is_null(), "CreateFloatArray({count})");
            a.c.cJSON_Delete(cp);
            a.rust.cJSON_Delete(rp);

            let cp = a.c.cJSON_CreateDoubleArray(doubles.as_ptr(), count);
            let rp = a.rust.cJSON_CreateDoubleArray(doubles.as_ptr(), count);
            assert_eq!(cp.is_null(), rp.is_null(), "CreateDoubleArray({count})");
            a.c.cJSON_Delete(cp);
            a.rust.cJSON_Delete(rp);
        }

        // NULL array pointer
        let cp = a.c.cJSON_CreateIntArray(std::ptr::null(), 3);
        let rp = a.rust.cJSON_CreateIntArray(std::ptr::null(), 3);
        assert_eq!(cp.is_null(), rp.is_null(), "CreateIntArray(NULL,3)");
        a.c.cJSON_Delete(cp);
        a.rust.cJSON_Delete(rp);
    }
}

#[test]
fn create_string_array() {
    let _guard = serial();
    let a = apis();
    unsafe {
        let owned: Vec<std::ffi::CString> = ["mon", "tue", "wed", "thu", "fri", "sat", "sun"]
            .iter()
            .map(|s| cs(s))
            .collect();
        let ptrs: Vec<*const c_char> = owned.iter().map(|s| s.as_ptr()).collect();

        for count in 0..=(ptrs.len() as c_int) {
            let cp = a.c.cJSON_CreateStringArray(ptrs.as_ptr(), count);
            let rp = a.rust.cJSON_CreateStringArray(ptrs.as_ptr(), count);
            assert_tree_eq(&format!("CreateStringArray(count={count})"), cp, rp);
            a.c.cJSON_Delete(cp);
            a.rust.cJSON_Delete(rp);
        }
        for count in [-1, i32::MIN] {
            let cp = a.c.cJSON_CreateStringArray(ptrs.as_ptr(), count);
            let rp = a.rust.cJSON_CreateStringArray(ptrs.as_ptr(), count);
            assert_eq!(cp.is_null(), rp.is_null(), "CreateStringArray({count})");
            a.c.cJSON_Delete(cp);
            a.rust.cJSON_Delete(rp);
        }
        let cp = a.c.cJSON_CreateStringArray(std::ptr::null(), 2);
        let rp = a.rust.cJSON_CreateStringArray(std::ptr::null(), 2);
        assert_eq!(cp.is_null(), rp.is_null(), "CreateStringArray(NULL,2)");
        a.c.cJSON_Delete(cp);
        a.rust.cJSON_Delete(rp);
    }
}

#[test]
fn create_references() {
    let _guard = serial();
    let a = apis();
    unsafe {
        // object/array references
        let cchild = a.c.cJSON_CreateObject();
        let rchild = a.rust.cJSON_CreateObject();
        let k = cs("k");
        a.c.cJSON_AddNumberToObject(cchild, k.as_ptr(), 1.0);
        a.rust.cJSON_AddNumberToObject(rchild, k.as_ptr(), 1.0);

        let cref = a.c.cJSON_CreateObjectReference(cchild);
        let rref = a.rust.cJSON_CreateObjectReference(rchild);
        assert_tree_eq("CreateObjectReference", cref, rref);
        a.c.cJSON_Delete(cref);
        a.rust.cJSON_Delete(rref);

        let cref = a.c.cJSON_CreateArrayReference(cchild);
        let rref = a.rust.cJSON_CreateArrayReference(rchild);
        assert_tree_eq("CreateArrayReference", cref, rref);
        a.c.cJSON_Delete(cref);
        a.rust.cJSON_Delete(rref);

        a.c.cJSON_Delete(cchild);
        a.rust.cJSON_Delete(rchild);

        // NULL child
        let cref = a.c.cJSON_CreateObjectReference(std::ptr::null());
        let rref = a.rust.cJSON_CreateObjectReference(std::ptr::null());
        assert_tree_eq("CreateObjectReference(NULL)", cref, rref);
        a.c.cJSON_Delete(cref);
        a.rust.cJSON_Delete(rref);

        let cref = a.c.cJSON_CreateArrayReference(std::ptr::null());
        let rref = a.rust.cJSON_CreateArrayReference(std::ptr::null());
        assert_tree_eq("CreateArrayReference(NULL)", cref, rref);
        a.c.cJSON_Delete(cref);
        a.rust.cJSON_Delete(rref);
    }
}

