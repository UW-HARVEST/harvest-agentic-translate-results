#![allow(non_camel_case_types, non_snake_case)]
use libloading::{Library, Symbol};
use std::ffi::{c_char, c_double, c_float, c_int, c_void, CStr, CString};
use std::ptr;

#[repr(C)]
pub struct cJSON {
    pub next: *mut cJSON,
    pub prev: *mut cJSON,
    pub child: *mut cJSON,
    pub type_: c_int,
    pub valuestring: *mut c_char,
    pub valueint: c_int,
    pub valuedouble: c_double,
    pub string: *mut c_char,
}

#[repr(C)]
pub struct cJSON_Hooks {
    pub malloc_fn: Option<unsafe extern "C" fn(usize) -> *mut c_void>,
    pub free_fn: Option<unsafe extern "C" fn(*mut c_void)>,
}

struct Lib {
    _lib: Library,
}

macro_rules! sym {
    ($lib:expr, $name:literal, $ty:ty) => {
        *$lib._lib.get::<$ty>($name).unwrap()
    };
}

fn load_c() -> Lib {
    let lib = unsafe {
        Library::new("/tmp/harvest-work-0AAhlP/translated_rust/c_src/build/libcjson.so").unwrap()
    };
    Lib { _lib: lib }
}

fn load_rust() -> Lib {
    let lib = unsafe {
        Library::new("/tmp/harvest-work-0AAhlP/translated_rust/target/debug/libcJSON_test.so")
            .unwrap()
    };
    Lib { _lib: lib }
}

unsafe fn lib_parse_print(lib: &Lib, json: &CStr) -> String {
    let parse: Symbol<unsafe extern "C" fn(*const c_char) -> *mut cJSON> =
        lib._lib.get(b"cJSON_Parse").unwrap();
    let print: Symbol<unsafe extern "C" fn(*const cJSON) -> *mut c_char> =
        lib._lib.get(b"cJSON_Print").unwrap();
    let delete: Symbol<unsafe extern "C" fn(*mut cJSON)> =
        lib._lib.get(b"cJSON_Delete").unwrap();
    let free_fn: Symbol<unsafe extern "C" fn(*mut c_void)> =
        lib._lib.get(b"cJSON_free").unwrap();

    let item = parse(json.as_ptr());
    assert!(!item.is_null(), "parse returned null");
    let s = print(item);
    assert!(!s.is_null(), "print returned null");
    let result = CStr::from_ptr(s).to_string_lossy().into_owned();
    free_fn(s as *mut c_void);
    delete(item);
    result
}

unsafe fn lib_parse_print_unformatted(lib: &Lib, json: &CStr) -> String {
    let parse: Symbol<unsafe extern "C" fn(*const c_char) -> *mut cJSON> =
        lib._lib.get(b"cJSON_Parse").unwrap();
    let print: Symbol<unsafe extern "C" fn(*const cJSON) -> *mut c_char> =
        lib._lib.get(b"cJSON_PrintUnformatted").unwrap();
    let delete: Symbol<unsafe extern "C" fn(*mut cJSON)> =
        lib._lib.get(b"cJSON_Delete").unwrap();
    let free_fn: Symbol<unsafe extern "C" fn(*mut c_void)> =
        lib._lib.get(b"cJSON_free").unwrap();

    let item = parse(json.as_ptr());
    assert!(!item.is_null());
    let s = print(item);
    assert!(!s.is_null());
    let result = CStr::from_ptr(s).to_string_lossy().into_owned();
    free_fn(s as *mut c_void);
    delete(item);
    result
}

fn cstr(s: &str) -> CString {
    CString::new(s).unwrap()
}

/// Compare a function's output on both libs for the same JSON input
fn compare_parse_print(json: &str) {
    let input = cstr(json);
    unsafe {
        let c = load_c();
        let r = load_rust();
        let c_out = lib_parse_print(&c, &input);
        let r_out = lib_parse_print(&r, &input);
        assert_eq!(c_out, r_out, "Mismatch for input: {json}");
    }
}

fn compare_parse_print_unformatted(json: &str) {
    let input = cstr(json);
    unsafe {
        let c = load_c();
        let r = load_rust();
        let c_out = lib_parse_print_unformatted(&c, &input);
        let r_out = lib_parse_print_unformatted(&r, &input);
        assert_eq!(c_out, r_out, "Unformatted mismatch for input: {json}");
    }
}

// ============ Tests ============

#[test]
fn test_version() {
    unsafe {
        let c = load_c();
        let r = load_rust();
        let cv: Symbol<unsafe extern "C" fn() -> *const c_char> =
            c._lib.get(b"cJSON_Version").unwrap();
        let rv: Symbol<unsafe extern "C" fn() -> *const c_char> =
            r._lib.get(b"cJSON_Version").unwrap();
        let cs = CStr::from_ptr(cv());
        let rs = CStr::from_ptr(rv());
        assert_eq!(cs, rs);
    }
}

#[test]
fn test_parse_print_simple_objects() {
    let cases = [
        r#"{"key":"value"}"#,
        r#"{"a":1,"b":2.5,"c":true,"d":false,"e":null}"#,
        r#"{"name":"Jack (\"Bee\") Nimble"}"#,
        r#"[1,2,3]"#,
        r#"[]"#,
        r#"{}"#,
        r#""hello""#,
        r#"42"#,
        r#"true"#,
        r#"false"#,
        r#"null"#,
        r#"{"nested":{"a":{"b":{"c":1}}}}"#,
        r#"[1,[2,[3,[4]]]]"#,
    ];
    for case in &cases {
        compare_parse_print(case);
        compare_parse_print_unformatted(case);
    }
}

#[test]
fn test_parse_print_numbers() {
    let cases = [
        "0", "1", "-1", "1.5", "-1.5", "1e10", "1e-10", "1.23e4",
        "1.0e+2", "0.0", "-0.0",
        "1234567890", "-1234567890",
        "0.00001", "1e20", "1e-20",
    ];
    for case in &cases {
        let json = format!("[{case}]");
        compare_parse_print(&json);
        compare_parse_print_unformatted(&json);
    }
}

#[test]
fn test_parse_print_strings_with_escapes() {
    let cases = [
        r#"["hello\nworld"]"#,
        r#"["tab\there"]"#,
        r#"["quote\"inside"]"#,
        r#"["backslash\\here"]"#,
        r#"["slash\/here"]"#,
        r#"["null\u0000char"]"#,
        r#"["\u0041\u0042\u0043"]"#,
        r#"["\ud83d\ude00"]"#,
    ];
    for case in &cases {
        compare_parse_print(case);
    }
}

#[test]
fn test_parse_with_length() {
    unsafe {
        let c = load_c();
        let r = load_rust();

        let json = cstr(r#"{"a":1}GARBAGE"#);
        let len = 7usize; // just {"a":1}

        let c_parse: Symbol<unsafe extern "C" fn(*const c_char, usize) -> *mut cJSON> =
            c._lib.get(b"cJSON_ParseWithLength").unwrap();
        let r_parse: Symbol<unsafe extern "C" fn(*const c_char, usize) -> *mut cJSON> =
            r._lib.get(b"cJSON_ParseWithLength").unwrap();
        let c_print: Symbol<unsafe extern "C" fn(*const cJSON) -> *mut c_char> =
            c._lib.get(b"cJSON_Print").unwrap();
        let r_print: Symbol<unsafe extern "C" fn(*const cJSON) -> *mut c_char> =
            r._lib.get(b"cJSON_Print").unwrap();
        let c_del: Symbol<unsafe extern "C" fn(*mut cJSON)> =
            c._lib.get(b"cJSON_Delete").unwrap();
        let r_del: Symbol<unsafe extern "C" fn(*mut cJSON)> =
            r._lib.get(b"cJSON_Delete").unwrap();
        let c_free: Symbol<unsafe extern "C" fn(*mut c_void)> =
            c._lib.get(b"cJSON_free").unwrap();
        let r_free: Symbol<unsafe extern "C" fn(*mut c_void)> =
            r._lib.get(b"cJSON_free").unwrap();

        let ci = c_parse(json.as_ptr(), len);
        let ri = r_parse(json.as_ptr(), len);
        assert!(!ci.is_null());
        assert!(!ri.is_null());

        let cs = CStr::from_ptr(c_print(ci)).to_string_lossy().into_owned();
        let rs = CStr::from_ptr(r_print(ri)).to_string_lossy().into_owned();
        assert_eq!(cs, rs);

        c_free(c_print(ci) as *mut c_void);
        r_free(r_print(ri) as *mut c_void);
        c_del(ci);
        r_del(ri);
    }
}

#[test]
fn test_parse_with_opts() {
    unsafe {
        let c = load_c();
        let r = load_rust();

        let json = cstr(r#"{"a":1}"#);
        let mut c_end: *const c_char = ptr::null();
        let mut r_end: *const c_char = ptr::null();

        let c_parse: Symbol<
            unsafe extern "C" fn(*const c_char, *mut *const c_char, c_int) -> *mut cJSON,
        > = c._lib.get(b"cJSON_ParseWithOpts").unwrap();
        let r_parse: Symbol<
            unsafe extern "C" fn(*const c_char, *mut *const c_char, c_int) -> *mut cJSON,
        > = r._lib.get(b"cJSON_ParseWithOpts").unwrap();
        let c_del: Symbol<unsafe extern "C" fn(*mut cJSON)> =
            c._lib.get(b"cJSON_Delete").unwrap();
        let r_del: Symbol<unsafe extern "C" fn(*mut cJSON)> =
            r._lib.get(b"cJSON_Delete").unwrap();

        let ci = c_parse(json.as_ptr(), &mut c_end, 1);
        let ri = r_parse(json.as_ptr(), &mut r_end, 1);
        assert!(!ci.is_null());
        assert!(!ri.is_null());

        // Both should have consumed the same amount
        let c_offset = c_end.offset_from(json.as_ptr());
        let r_offset = r_end.offset_from(json.as_ptr());
        assert_eq!(c_offset, r_offset);

        c_del(ci);
        r_del(ri);
    }
}

#[test]
fn test_print_buffered() {
    unsafe {
        let c = load_c();
        let r = load_rust();
        let json = cstr(r#"{"a":1,"b":[2,3]}"#);

        let c_parse: Symbol<unsafe extern "C" fn(*const c_char) -> *mut cJSON> =
            c._lib.get(b"cJSON_Parse").unwrap();
        let r_parse: Symbol<unsafe extern "C" fn(*const c_char) -> *mut cJSON> =
            r._lib.get(b"cJSON_Parse").unwrap();
        let c_pb: Symbol<unsafe extern "C" fn(*const cJSON, c_int, c_int) -> *mut c_char> =
            c._lib.get(b"cJSON_PrintBuffered").unwrap();
        let r_pb: Symbol<unsafe extern "C" fn(*const cJSON, c_int, c_int) -> *mut c_char> =
            r._lib.get(b"cJSON_PrintBuffered").unwrap();
        let c_del: Symbol<unsafe extern "C" fn(*mut cJSON)> =
            c._lib.get(b"cJSON_Delete").unwrap();
        let r_del: Symbol<unsafe extern "C" fn(*mut cJSON)> =
            r._lib.get(b"cJSON_Delete").unwrap();
        let c_free: Symbol<unsafe extern "C" fn(*mut c_void)> =
            c._lib.get(b"cJSON_free").unwrap();
        let r_free: Symbol<unsafe extern "C" fn(*mut c_void)> =
            r._lib.get(b"cJSON_free").unwrap();

        let ci = c_parse(json.as_ptr());
        let ri = r_parse(json.as_ptr());

        for fmt in [0, 1] {
            let cs = c_pb(ci, 256, fmt);
            let rs = r_pb(ri, 256, fmt);
            let c_str = CStr::from_ptr(cs).to_string_lossy().into_owned();
            let r_str = CStr::from_ptr(rs).to_string_lossy().into_owned();
            assert_eq!(c_str, r_str, "PrintBuffered mismatch fmt={fmt}");
            c_free(cs as *mut c_void);
            r_free(rs as *mut c_void);
        }

        c_del(ci);
        r_del(ri);
    }
}

#[test]
fn test_print_preallocated() {
    unsafe {
        let c = load_c();
        let r = load_rust();
        let json = cstr(r#"{"x":42}"#);

        let c_parse: Symbol<unsafe extern "C" fn(*const c_char) -> *mut cJSON> =
            c._lib.get(b"cJSON_Parse").unwrap();
        let r_parse: Symbol<unsafe extern "C" fn(*const c_char) -> *mut cJSON> =
            r._lib.get(b"cJSON_Parse").unwrap();
        let c_pp: Symbol<unsafe extern "C" fn(*mut cJSON, *mut c_char, c_int, c_int) -> c_int> =
            c._lib.get(b"cJSON_PrintPreallocated").unwrap();
        let r_pp: Symbol<unsafe extern "C" fn(*mut cJSON, *mut c_char, c_int, c_int) -> c_int> =
            r._lib.get(b"cJSON_PrintPreallocated").unwrap();
        let c_del: Symbol<unsafe extern "C" fn(*mut cJSON)> =
            c._lib.get(b"cJSON_Delete").unwrap();
        let r_del: Symbol<unsafe extern "C" fn(*mut cJSON)> =
            r._lib.get(b"cJSON_Delete").unwrap();

        let ci = c_parse(json.as_ptr());
        let ri = r_parse(json.as_ptr());

        let mut c_buf = vec![0u8; 256];
        let mut r_buf = vec![0u8; 256];

        for fmt in [0, 1] {
            c_buf.fill(0);
            r_buf.fill(0);
            let c_ret = c_pp(ci, c_buf.as_mut_ptr() as *mut c_char, 256, fmt);
            let r_ret = r_pp(ri, r_buf.as_mut_ptr() as *mut c_char, 256, fmt);
            assert_eq!(c_ret, r_ret, "PrintPreallocated return mismatch fmt={fmt}");
            assert_eq!(c_buf, r_buf, "PrintPreallocated output mismatch fmt={fmt}");
        }

        // Test with too-small buffer
        let mut c_small = vec![0u8; 5];
        let mut r_small = vec![0u8; 5];
        let c_ret = c_pp(ci, c_small.as_mut_ptr() as *mut c_char, 5, 1);
        let r_ret = r_pp(ri, r_small.as_mut_ptr() as *mut c_char, 5, 1);
        assert_eq!(c_ret, r_ret, "PrintPreallocated small buffer return mismatch");

        c_del(ci);
        r_del(ri);
    }
}

#[test]
fn test_create_and_type_checks() {
    unsafe {
        let c = load_c();
        let r = load_rust();

        // Test all Create* + Is* functions
        macro_rules! test_create_is {
            ($lib:expr, $create:literal, $check:literal) => {{
                let create: Symbol<unsafe extern "C" fn() -> *mut cJSON> =
                    $lib._lib.get($create).unwrap();
                let check: Symbol<unsafe extern "C" fn(*const cJSON) -> c_int> =
                    $lib._lib.get($check).unwrap();
                let del: Symbol<unsafe extern "C" fn(*mut cJSON)> =
                    $lib._lib.get(b"cJSON_Delete").unwrap();
                let item = create();
                let result = check(item);
                del(item);
                result
            }};
        }

        let pairs = [
            (b"cJSON_CreateNull\0" as &[u8], b"cJSON_IsNull\0" as &[u8]),
            (b"cJSON_CreateTrue\0", b"cJSON_IsTrue\0"),
            (b"cJSON_CreateFalse\0", b"cJSON_IsFalse\0"),
            (b"cJSON_CreateArray\0", b"cJSON_IsArray\0"),
            (b"cJSON_CreateObject\0", b"cJSON_IsObject\0"),
        ];

        for (create, check) in &pairs {
            let c_res = test_create_is!(c, create, check);
            let r_res = test_create_is!(r, create, check);
            assert_eq!(c_res, r_res, "Type check mismatch for {:?}", std::str::from_utf8(create));
        }

        // cJSON_CreateBool
        for b in [0, 1] {
            let c_create: Symbol<unsafe extern "C" fn(c_int) -> *mut cJSON> =
                c._lib.get(b"cJSON_CreateBool").unwrap();
            let r_create: Symbol<unsafe extern "C" fn(c_int) -> *mut cJSON> =
                r._lib.get(b"cJSON_CreateBool").unwrap();
            let c_is_bool: Symbol<unsafe extern "C" fn(*const cJSON) -> c_int> =
                c._lib.get(b"cJSON_IsBool").unwrap();
            let r_is_bool: Symbol<unsafe extern "C" fn(*const cJSON) -> c_int> =
                r._lib.get(b"cJSON_IsBool").unwrap();
            let c_del: Symbol<unsafe extern "C" fn(*mut cJSON)> =
                c._lib.get(b"cJSON_Delete").unwrap();
            let r_del: Symbol<unsafe extern "C" fn(*mut cJSON)> =
                r._lib.get(b"cJSON_Delete").unwrap();

            let ci = c_create(b);
            let ri = r_create(b);
            assert_eq!(c_is_bool(ci), r_is_bool(ri));
            assert_eq!((*ci).type_, (*ri).type_);
            c_del(ci);
            r_del(ri);
        }

        // cJSON_IsInvalid on null pointer
        let c_inv: Symbol<unsafe extern "C" fn(*const cJSON) -> c_int> =
            c._lib.get(b"cJSON_IsInvalid").unwrap();
        let r_inv: Symbol<unsafe extern "C" fn(*const cJSON) -> c_int> =
            r._lib.get(b"cJSON_IsInvalid").unwrap();
        assert_eq!(c_inv(ptr::null()), r_inv(ptr::null()));
    }
}

#[test]
fn test_create_number() {
    unsafe {
        let c = load_c();
        let r = load_rust();

        let c_create: Symbol<unsafe extern "C" fn(c_double) -> *mut cJSON> =
            c._lib.get(b"cJSON_CreateNumber").unwrap();
        let r_create: Symbol<unsafe extern "C" fn(c_double) -> *mut cJSON> =
            r._lib.get(b"cJSON_CreateNumber").unwrap();
        let c_print: Symbol<unsafe extern "C" fn(*const cJSON) -> *mut c_char> =
            c._lib.get(b"cJSON_Print").unwrap();
        let r_print: Symbol<unsafe extern "C" fn(*const cJSON) -> *mut c_char> =
            r._lib.get(b"cJSON_Print").unwrap();
        let c_del: Symbol<unsafe extern "C" fn(*mut cJSON)> =
            c._lib.get(b"cJSON_Delete").unwrap();
        let r_del: Symbol<unsafe extern "C" fn(*mut cJSON)> =
            r._lib.get(b"cJSON_Delete").unwrap();
        let c_free: Symbol<unsafe extern "C" fn(*mut c_void)> =
            c._lib.get(b"cJSON_free").unwrap();
        let r_free: Symbol<unsafe extern "C" fn(*mut c_void)> =
            r._lib.get(b"cJSON_free").unwrap();
        let c_is_num: Symbol<unsafe extern "C" fn(*const cJSON) -> c_int> =
            c._lib.get(b"cJSON_IsNumber").unwrap();
        let r_is_num: Symbol<unsafe extern "C" fn(*const cJSON) -> c_int> =
            r._lib.get(b"cJSON_IsNumber").unwrap();
        let c_get_num: Symbol<unsafe extern "C" fn(*const cJSON) -> c_double> =
            c._lib.get(b"cJSON_GetNumberValue").unwrap();
        let r_get_num: Symbol<unsafe extern "C" fn(*const cJSON) -> c_double> =
            r._lib.get(b"cJSON_GetNumberValue").unwrap();

        let nums = [0.0, 1.0, -1.0, 3.14, 1e10, 1e-10, f64::MAX, f64::MIN_POSITIVE];
        for n in nums {
            let ci = c_create(n);
            let ri = r_create(n);
            assert_eq!(c_is_num(ci), r_is_num(ri));
            assert_eq!(c_get_num(ci), r_get_num(ri));
            assert_eq!((*ci).valueint, (*ri).valueint);
            assert_eq!((*ci).valuedouble, (*ri).valuedouble);

            let cs = c_print(ci);
            let rs = r_print(ri);
            let c_str = CStr::from_ptr(cs).to_string_lossy().into_owned();
            let r_str = CStr::from_ptr(rs).to_string_lossy().into_owned();
            assert_eq!(c_str, r_str, "CreateNumber print mismatch for {n}");
            c_free(cs as *mut c_void);
            r_free(rs as *mut c_void);
            c_del(ci);
            r_del(ri);
        }
    }
}

#[test]
fn test_create_string() {
    unsafe {
        let c = load_c();
        let r = load_rust();

        let c_create: Symbol<unsafe extern "C" fn(*const c_char) -> *mut cJSON> =
            c._lib.get(b"cJSON_CreateString").unwrap();
        let r_create: Symbol<unsafe extern "C" fn(*const c_char) -> *mut cJSON> =
            r._lib.get(b"cJSON_CreateString").unwrap();
        let c_print: Symbol<unsafe extern "C" fn(*const cJSON) -> *mut c_char> =
            c._lib.get(b"cJSON_Print").unwrap();
        let r_print: Symbol<unsafe extern "C" fn(*const cJSON) -> *mut c_char> =
            r._lib.get(b"cJSON_Print").unwrap();
        let c_del: Symbol<unsafe extern "C" fn(*mut cJSON)> =
            c._lib.get(b"cJSON_Delete").unwrap();
        let r_del: Symbol<unsafe extern "C" fn(*mut cJSON)> =
            r._lib.get(b"cJSON_Delete").unwrap();
        let c_free: Symbol<unsafe extern "C" fn(*mut c_void)> =
            c._lib.get(b"cJSON_free").unwrap();
        let r_free: Symbol<unsafe extern "C" fn(*mut c_void)> =
            r._lib.get(b"cJSON_free").unwrap();
        let c_is_str: Symbol<unsafe extern "C" fn(*const cJSON) -> c_int> =
            c._lib.get(b"cJSON_IsString").unwrap();
        let r_is_str: Symbol<unsafe extern "C" fn(*const cJSON) -> c_int> =
            r._lib.get(b"cJSON_IsString").unwrap();
        let c_get_str: Symbol<unsafe extern "C" fn(*const cJSON) -> *mut c_char> =
            c._lib.get(b"cJSON_GetStringValue").unwrap();
        let r_get_str: Symbol<unsafe extern "C" fn(*const cJSON) -> *mut c_char> =
            r._lib.get(b"cJSON_GetStringValue").unwrap();

        let strings = ["hello", "world", "", "with spaces", "with\ttab"];
        for s in strings {
            let cs_input = cstr(s);
            let ci = c_create(cs_input.as_ptr());
            let ri = r_create(cs_input.as_ptr());
            assert_eq!(c_is_str(ci), r_is_str(ri));

            let c_val = CStr::from_ptr(c_get_str(ci)).to_string_lossy().into_owned();
            let r_val = CStr::from_ptr(r_get_str(ri)).to_string_lossy().into_owned();
            assert_eq!(c_val, r_val);

            let cs = c_print(ci);
            let rs = r_print(ri);
            let c_str = CStr::from_ptr(cs).to_string_lossy().into_owned();
            let r_str = CStr::from_ptr(rs).to_string_lossy().into_owned();
            assert_eq!(c_str, r_str, "CreateString print mismatch for {s}");
            c_free(cs as *mut c_void);
            r_free(rs as *mut c_void);
            c_del(ci);
            r_del(ri);
        }
    }
}

#[test]
fn test_create_raw() {
    unsafe {
        let c = load_c();
        let r = load_rust();

        let c_create: Symbol<unsafe extern "C" fn(*const c_char) -> *mut cJSON> =
            c._lib.get(b"cJSON_CreateRaw").unwrap();
        let r_create: Symbol<unsafe extern "C" fn(*const c_char) -> *mut cJSON> =
            r._lib.get(b"cJSON_CreateRaw").unwrap();
        let c_print: Symbol<unsafe extern "C" fn(*const cJSON) -> *mut c_char> =
            c._lib.get(b"cJSON_Print").unwrap();
        let r_print: Symbol<unsafe extern "C" fn(*const cJSON) -> *mut c_char> =
            r._lib.get(b"cJSON_Print").unwrap();
        let c_del: Symbol<unsafe extern "C" fn(*mut cJSON)> =
            c._lib.get(b"cJSON_Delete").unwrap();
        let r_del: Symbol<unsafe extern "C" fn(*mut cJSON)> =
            r._lib.get(b"cJSON_Delete").unwrap();
        let c_free: Symbol<unsafe extern "C" fn(*mut c_void)> =
            c._lib.get(b"cJSON_free").unwrap();
        let r_free: Symbol<unsafe extern "C" fn(*mut c_void)> =
            r._lib.get(b"cJSON_free").unwrap();
        let c_is_raw: Symbol<unsafe extern "C" fn(*const cJSON) -> c_int> =
            c._lib.get(b"cJSON_IsRaw").unwrap();
        let r_is_raw: Symbol<unsafe extern "C" fn(*const cJSON) -> c_int> =
            r._lib.get(b"cJSON_IsRaw").unwrap();

        let raw = cstr(r#"{"raw": true}"#);
        let ci = c_create(raw.as_ptr());
        let ri = r_create(raw.as_ptr());
        assert_eq!(c_is_raw(ci), r_is_raw(ri));

        let cs = c_print(ci);
        let rs = r_print(ri);
        let c_str = CStr::from_ptr(cs).to_string_lossy().into_owned();
        let r_str = CStr::from_ptr(rs).to_string_lossy().into_owned();
        assert_eq!(c_str, r_str);
        c_free(cs as *mut c_void);
        r_free(rs as *mut c_void);
        c_del(ci);
        r_del(ri);
    }
}

#[test]
fn test_create_int_array() {
    unsafe {
        let c = load_c();
        let r = load_rust();
        let nums: [c_int; 4] = [1, -2, 3, 0];

        let c_create: Symbol<unsafe extern "C" fn(*const c_int, c_int) -> *mut cJSON> =
            c._lib.get(b"cJSON_CreateIntArray").unwrap();
        let r_create: Symbol<unsafe extern "C" fn(*const c_int, c_int) -> *mut cJSON> =
            r._lib.get(b"cJSON_CreateIntArray").unwrap();
        let c_print: Symbol<unsafe extern "C" fn(*const cJSON) -> *mut c_char> =
            c._lib.get(b"cJSON_Print").unwrap();
        let r_print: Symbol<unsafe extern "C" fn(*const cJSON) -> *mut c_char> =
            r._lib.get(b"cJSON_Print").unwrap();
        let c_del: Symbol<unsafe extern "C" fn(*mut cJSON)> = c._lib.get(b"cJSON_Delete").unwrap();
        let r_del: Symbol<unsafe extern "C" fn(*mut cJSON)> = r._lib.get(b"cJSON_Delete").unwrap();
        let c_free: Symbol<unsafe extern "C" fn(*mut c_void)> = c._lib.get(b"cJSON_free").unwrap();
        let r_free: Symbol<unsafe extern "C" fn(*mut c_void)> = r._lib.get(b"cJSON_free").unwrap();

        let ci = c_create(nums.as_ptr(), 4);
        let ri = r_create(nums.as_ptr(), 4);
        let cs = c_print(ci);
        let rs = r_print(ri);
        assert_eq!(
            CStr::from_ptr(cs).to_string_lossy(),
            CStr::from_ptr(rs).to_string_lossy()
        );
        c_free(cs as *mut c_void);
        r_free(rs as *mut c_void);
        c_del(ci);
        r_del(ri);
    }
}

#[test]
fn test_create_float_array() {
    unsafe {
        let c = load_c();
        let r = load_rust();
        let nums: [c_float; 3] = [1.5, -2.5, 0.0];

        let c_create: Symbol<unsafe extern "C" fn(*const c_float, c_int) -> *mut cJSON> =
            c._lib.get(b"cJSON_CreateFloatArray").unwrap();
        let r_create: Symbol<unsafe extern "C" fn(*const c_float, c_int) -> *mut cJSON> =
            r._lib.get(b"cJSON_CreateFloatArray").unwrap();
        let c_print: Symbol<unsafe extern "C" fn(*const cJSON) -> *mut c_char> =
            c._lib.get(b"cJSON_Print").unwrap();
        let r_print: Symbol<unsafe extern "C" fn(*const cJSON) -> *mut c_char> =
            r._lib.get(b"cJSON_Print").unwrap();
        let c_del: Symbol<unsafe extern "C" fn(*mut cJSON)> = c._lib.get(b"cJSON_Delete").unwrap();
        let r_del: Symbol<unsafe extern "C" fn(*mut cJSON)> = r._lib.get(b"cJSON_Delete").unwrap();
        let c_free: Symbol<unsafe extern "C" fn(*mut c_void)> = c._lib.get(b"cJSON_free").unwrap();
        let r_free: Symbol<unsafe extern "C" fn(*mut c_void)> = r._lib.get(b"cJSON_free").unwrap();

        let ci = c_create(nums.as_ptr(), 3);
        let ri = r_create(nums.as_ptr(), 3);
        let cs = c_print(ci);
        let rs = r_print(ri);
        assert_eq!(
            CStr::from_ptr(cs).to_string_lossy(),
            CStr::from_ptr(rs).to_string_lossy()
        );
        c_free(cs as *mut c_void);
        r_free(rs as *mut c_void);
        c_del(ci);
        r_del(ri);
    }
}

#[test]
fn test_create_double_array() {
    unsafe {
        let c = load_c();
        let r = load_rust();
        let nums: [c_double; 3] = [1.5, -2.5, 3.14159];

        let c_create: Symbol<unsafe extern "C" fn(*const c_double, c_int) -> *mut cJSON> =
            c._lib.get(b"cJSON_CreateDoubleArray").unwrap();
        let r_create: Symbol<unsafe extern "C" fn(*const c_double, c_int) -> *mut cJSON> =
            r._lib.get(b"cJSON_CreateDoubleArray").unwrap();
        let c_print: Symbol<unsafe extern "C" fn(*const cJSON) -> *mut c_char> =
            c._lib.get(b"cJSON_Print").unwrap();
        let r_print: Symbol<unsafe extern "C" fn(*const cJSON) -> *mut c_char> =
            r._lib.get(b"cJSON_Print").unwrap();
        let c_del: Symbol<unsafe extern "C" fn(*mut cJSON)> = c._lib.get(b"cJSON_Delete").unwrap();
        let r_del: Symbol<unsafe extern "C" fn(*mut cJSON)> = r._lib.get(b"cJSON_Delete").unwrap();
        let c_free: Symbol<unsafe extern "C" fn(*mut c_void)> = c._lib.get(b"cJSON_free").unwrap();
        let r_free: Symbol<unsafe extern "C" fn(*mut c_void)> = r._lib.get(b"cJSON_free").unwrap();

        let ci = c_create(nums.as_ptr(), 3);
        let ri = r_create(nums.as_ptr(), 3);
        let cs = c_print(ci);
        let rs = r_print(ri);
        assert_eq!(
            CStr::from_ptr(cs).to_string_lossy(),
            CStr::from_ptr(rs).to_string_lossy()
        );
        c_free(cs as *mut c_void);
        r_free(rs as *mut c_void);
        c_del(ci);
        r_del(ri);
    }
}

#[test]
fn test_create_string_array() {
    unsafe {
        let c = load_c();
        let r = load_rust();
        let s1 = cstr("Mon");
        let s2 = cstr("Tue");
        let s3 = cstr("Wed");
        let ptrs: [*const c_char; 3] = [s1.as_ptr(), s2.as_ptr(), s3.as_ptr()];

        let c_create: Symbol<unsafe extern "C" fn(*const *const c_char, c_int) -> *mut cJSON> =
            c._lib.get(b"cJSON_CreateStringArray").unwrap();
        let r_create: Symbol<unsafe extern "C" fn(*const *const c_char, c_int) -> *mut cJSON> =
            r._lib.get(b"cJSON_CreateStringArray").unwrap();
        let c_print: Symbol<unsafe extern "C" fn(*const cJSON) -> *mut c_char> =
            c._lib.get(b"cJSON_Print").unwrap();
        let r_print: Symbol<unsafe extern "C" fn(*const cJSON) -> *mut c_char> =
            r._lib.get(b"cJSON_Print").unwrap();
        let c_del: Symbol<unsafe extern "C" fn(*mut cJSON)> = c._lib.get(b"cJSON_Delete").unwrap();
        let r_del: Symbol<unsafe extern "C" fn(*mut cJSON)> = r._lib.get(b"cJSON_Delete").unwrap();
        let c_free: Symbol<unsafe extern "C" fn(*mut c_void)> = c._lib.get(b"cJSON_free").unwrap();
        let r_free: Symbol<unsafe extern "C" fn(*mut c_void)> = r._lib.get(b"cJSON_free").unwrap();

        let ci = c_create(ptrs.as_ptr(), 3);
        let ri = r_create(ptrs.as_ptr(), 3);
        let cs = c_print(ci);
        let rs = r_print(ri);
        assert_eq!(
            CStr::from_ptr(cs).to_string_lossy(),
            CStr::from_ptr(rs).to_string_lossy()
        );
        c_free(cs as *mut c_void);
        r_free(rs as *mut c_void);
        c_del(ci);
        r_del(ri);
    }
}

#[test]
fn test_array_operations() {
    unsafe {
        let c = load_c();
        let r = load_rust();

        let json = cstr(r#"[1,2,3,4,5]"#);
        let c_parse: Symbol<unsafe extern "C" fn(*const c_char) -> *mut cJSON> =
            c._lib.get(b"cJSON_Parse").unwrap();
        let r_parse: Symbol<unsafe extern "C" fn(*const c_char) -> *mut cJSON> =
            r._lib.get(b"cJSON_Parse").unwrap();
        let c_size: Symbol<unsafe extern "C" fn(*const cJSON) -> c_int> =
            c._lib.get(b"cJSON_GetArraySize").unwrap();
        let r_size: Symbol<unsafe extern "C" fn(*const cJSON) -> c_int> =
            r._lib.get(b"cJSON_GetArraySize").unwrap();
        let c_item: Symbol<unsafe extern "C" fn(*const cJSON, c_int) -> *mut cJSON> =
            c._lib.get(b"cJSON_GetArrayItem").unwrap();
        let r_item: Symbol<unsafe extern "C" fn(*const cJSON, c_int) -> *mut cJSON> =
            r._lib.get(b"cJSON_GetArrayItem").unwrap();
        let c_del: Symbol<unsafe extern "C" fn(*mut cJSON)> = c._lib.get(b"cJSON_Delete").unwrap();
        let r_del: Symbol<unsafe extern "C" fn(*mut cJSON)> = r._lib.get(b"cJSON_Delete").unwrap();

        let ci = c_parse(json.as_ptr());
        let ri = r_parse(json.as_ptr());

        assert_eq!(c_size(ci), r_size(ri));
        assert_eq!(c_size(ci), 5);

        for i in 0..5 {
            let c_el = c_item(ci, i);
            let r_el = r_item(ri, i);
            assert_eq!((*c_el).valuedouble, (*r_el).valuedouble);
            assert_eq!((*c_el).valueint, (*r_el).valueint);
        }

        // Out of bounds
        assert!(c_item(ci, 10).is_null());
        assert!(r_item(ri, 10).is_null());

        // Null array
        assert_eq!(c_size(ptr::null()), r_size(ptr::null()));

        c_del(ci);
        r_del(ri);
    }
}

#[test]
fn test_object_operations() {
    unsafe {
        let c = load_c();
        let r = load_rust();

        let json = cstr(r#"{"Name":"Jack","Age":30,"City":"NYC"}"#);
        let c_parse: Symbol<unsafe extern "C" fn(*const c_char) -> *mut cJSON> =
            c._lib.get(b"cJSON_Parse").unwrap();
        let r_parse: Symbol<unsafe extern "C" fn(*const c_char) -> *mut cJSON> =
            r._lib.get(b"cJSON_Parse").unwrap();
        let c_get: Symbol<unsafe extern "C" fn(*const cJSON, *const c_char) -> *mut cJSON> =
            c._lib.get(b"cJSON_GetObjectItem").unwrap();
        let r_get: Symbol<unsafe extern "C" fn(*const cJSON, *const c_char) -> *mut cJSON> =
            r._lib.get(b"cJSON_GetObjectItem").unwrap();
        let c_get_cs: Symbol<unsafe extern "C" fn(*const cJSON, *const c_char) -> *mut cJSON> =
            c._lib.get(b"cJSON_GetObjectItemCaseSensitive").unwrap();
        let r_get_cs: Symbol<unsafe extern "C" fn(*const cJSON, *const c_char) -> *mut cJSON> =
            r._lib.get(b"cJSON_GetObjectItemCaseSensitive").unwrap();
        let c_has: Symbol<unsafe extern "C" fn(*const cJSON, *const c_char) -> c_int> =
            c._lib.get(b"cJSON_HasObjectItem").unwrap();
        let r_has: Symbol<unsafe extern "C" fn(*const cJSON, *const c_char) -> c_int> =
            r._lib.get(b"cJSON_HasObjectItem").unwrap();
        let c_del: Symbol<unsafe extern "C" fn(*mut cJSON)> = c._lib.get(b"cJSON_Delete").unwrap();
        let r_del: Symbol<unsafe extern "C" fn(*mut cJSON)> = r._lib.get(b"cJSON_Delete").unwrap();

        let ci = c_parse(json.as_ptr());
        let ri = r_parse(json.as_ptr());

        // Case insensitive get
        let name_key = cstr("name");
        let c_name = c_get(ci, name_key.as_ptr());
        let r_name = r_get(ri, name_key.as_ptr());
        assert!(!c_name.is_null());
        assert!(!r_name.is_null());
        assert_eq!(
            CStr::from_ptr((*c_name).valuestring),
            CStr::from_ptr((*r_name).valuestring)
        );

        // Case sensitive get - "name" should NOT match "Name"
        let c_name_cs = c_get_cs(ci, name_key.as_ptr());
        let r_name_cs = r_get_cs(ri, name_key.as_ptr());
        assert_eq!(c_name_cs.is_null(), r_name_cs.is_null());

        // Case sensitive get - "Name" should match
        let name_key2 = cstr("Name");
        let c_name_cs2 = c_get_cs(ci, name_key2.as_ptr());
        let r_name_cs2 = r_get_cs(ri, name_key2.as_ptr());
        assert!(!c_name_cs2.is_null());
        assert!(!r_name_cs2.is_null());

        // HasObjectItem
        assert_eq!(c_has(ci, name_key2.as_ptr()), r_has(ri, name_key2.as_ptr()));
        let missing = cstr("missing");
        assert_eq!(c_has(ci, missing.as_ptr()), r_has(ri, missing.as_ptr()));

        c_del(ci);
        r_del(ri);
    }
}

#[test]
fn test_add_item_to_object() {
    unsafe {
        let c = load_c();
        let r = load_rust();

        let c_obj: Symbol<unsafe extern "C" fn() -> *mut cJSON> =
            c._lib.get(b"cJSON_CreateObject").unwrap();
        let r_obj: Symbol<unsafe extern "C" fn() -> *mut cJSON> =
            r._lib.get(b"cJSON_CreateObject").unwrap();
        let c_str: Symbol<unsafe extern "C" fn(*const c_char) -> *mut cJSON> =
            c._lib.get(b"cJSON_CreateString").unwrap();
        let r_str: Symbol<unsafe extern "C" fn(*const c_char) -> *mut cJSON> =
            r._lib.get(b"cJSON_CreateString").unwrap();
        let c_num: Symbol<unsafe extern "C" fn(c_double) -> *mut cJSON> =
            c._lib.get(b"cJSON_CreateNumber").unwrap();
        let r_num: Symbol<unsafe extern "C" fn(c_double) -> *mut cJSON> =
            r._lib.get(b"cJSON_CreateNumber").unwrap();
        let c_add: Symbol<unsafe extern "C" fn(*mut cJSON, *const c_char, *mut cJSON) -> c_int> =
            c._lib.get(b"cJSON_AddItemToObject").unwrap();
        let r_add: Symbol<unsafe extern "C" fn(*mut cJSON, *const c_char, *mut cJSON) -> c_int> =
            r._lib.get(b"cJSON_AddItemToObject").unwrap();
        let c_print: Symbol<unsafe extern "C" fn(*const cJSON) -> *mut c_char> =
            c._lib.get(b"cJSON_Print").unwrap();
        let r_print: Symbol<unsafe extern "C" fn(*const cJSON) -> *mut c_char> =
            r._lib.get(b"cJSON_Print").unwrap();
        let c_del: Symbol<unsafe extern "C" fn(*mut cJSON)> = c._lib.get(b"cJSON_Delete").unwrap();
        let r_del: Symbol<unsafe extern "C" fn(*mut cJSON)> = r._lib.get(b"cJSON_Delete").unwrap();
        let c_free: Symbol<unsafe extern "C" fn(*mut c_void)> = c._lib.get(b"cJSON_free").unwrap();
        let r_free: Symbol<unsafe extern "C" fn(*mut c_void)> = r._lib.get(b"cJSON_free").unwrap();

        let co = c_obj();
        let ro = r_obj();
        let key = cstr("key");
        let val = cstr("value");

        c_add(co, key.as_ptr(), c_str(val.as_ptr()));
        r_add(ro, key.as_ptr(), r_str(val.as_ptr()));

        let num_key = cstr("num");
        c_add(co, num_key.as_ptr(), c_num(42.0));
        r_add(ro, num_key.as_ptr(), r_num(42.0));

        let cs = c_print(co);
        let rs = r_print(ro);
        assert_eq!(
            CStr::from_ptr(cs).to_string_lossy(),
            CStr::from_ptr(rs).to_string_lossy()
        );
        c_free(cs as *mut c_void);
        r_free(rs as *mut c_void);
        c_del(co);
        r_del(ro);
    }
}

#[test]
fn test_add_helpers_to_object() {
    unsafe {
        let c = load_c();
        let r = load_rust();

        let c_obj: Symbol<unsafe extern "C" fn() -> *mut cJSON> =
            c._lib.get(b"cJSON_CreateObject").unwrap();
        let r_obj: Symbol<unsafe extern "C" fn() -> *mut cJSON> =
            r._lib.get(b"cJSON_CreateObject").unwrap();
        let c_print: Symbol<unsafe extern "C" fn(*const cJSON) -> *mut c_char> =
            c._lib.get(b"cJSON_Print").unwrap();
        let r_print: Symbol<unsafe extern "C" fn(*const cJSON) -> *mut c_char> =
            r._lib.get(b"cJSON_Print").unwrap();
        let c_del: Symbol<unsafe extern "C" fn(*mut cJSON)> = c._lib.get(b"cJSON_Delete").unwrap();
        let r_del: Symbol<unsafe extern "C" fn(*mut cJSON)> = r._lib.get(b"cJSON_Delete").unwrap();
        let c_free: Symbol<unsafe extern "C" fn(*mut c_void)> = c._lib.get(b"cJSON_free").unwrap();
        let r_free: Symbol<unsafe extern "C" fn(*mut c_void)> = r._lib.get(b"cJSON_free").unwrap();

        let co = c_obj();
        let ro = r_obj();

        macro_rules! add_helper {
            ($name:literal, $c_lib:expr, $r_lib:expr, $co:expr, $ro:expr, $key:expr $(, $arg:expr)*) => {{
                let c_fn: Symbol<unsafe extern "C" fn(*mut cJSON, *const c_char $(, ${ignore($arg)} _)*) -> *mut cJSON> =
                    $c_lib._lib.get($name).unwrap();
                let r_fn: Symbol<unsafe extern "C" fn(*mut cJSON, *const c_char $(, ${ignore($arg)} _)*) -> *mut cJSON> =
                    $r_lib._lib.get($name).unwrap();
                let k = cstr($key);
                c_fn($co, k.as_ptr() $(, $arg)*);
                r_fn($ro, k.as_ptr() $(, $arg)*);
            }};
        }

        // AddNullToObject
        let k = cstr("n");
        let c_add_null: Symbol<unsafe extern "C" fn(*mut cJSON, *const c_char) -> *mut cJSON> =
            c._lib.get(b"cJSON_AddNullToObject").unwrap();
        let r_add_null: Symbol<unsafe extern "C" fn(*mut cJSON, *const c_char) -> *mut cJSON> =
            r._lib.get(b"cJSON_AddNullToObject").unwrap();
        c_add_null(co, k.as_ptr());
        r_add_null(ro, k.as_ptr());

        let k = cstr("t");
        let c_add_true: Symbol<unsafe extern "C" fn(*mut cJSON, *const c_char) -> *mut cJSON> =
            c._lib.get(b"cJSON_AddTrueToObject").unwrap();
        let r_add_true: Symbol<unsafe extern "C" fn(*mut cJSON, *const c_char) -> *mut cJSON> =
            r._lib.get(b"cJSON_AddTrueToObject").unwrap();
        c_add_true(co, k.as_ptr());
        r_add_true(ro, k.as_ptr());

        let k = cstr("f");
        let c_add_false: Symbol<unsafe extern "C" fn(*mut cJSON, *const c_char) -> *mut cJSON> =
            c._lib.get(b"cJSON_AddFalseToObject").unwrap();
        let r_add_false: Symbol<unsafe extern "C" fn(*mut cJSON, *const c_char) -> *mut cJSON> =
            r._lib.get(b"cJSON_AddFalseToObject").unwrap();
        c_add_false(co, k.as_ptr());
        r_add_false(ro, k.as_ptr());

        let k = cstr("b");
        let c_add_bool: Symbol<unsafe extern "C" fn(*mut cJSON, *const c_char, c_int) -> *mut cJSON> =
            c._lib.get(b"cJSON_AddBoolToObject").unwrap();
        let r_add_bool: Symbol<unsafe extern "C" fn(*mut cJSON, *const c_char, c_int) -> *mut cJSON> =
            r._lib.get(b"cJSON_AddBoolToObject").unwrap();
        c_add_bool(co, k.as_ptr(), 1);
        r_add_bool(ro, k.as_ptr(), 1);

        let k = cstr("num");
        let c_add_num: Symbol<unsafe extern "C" fn(*mut cJSON, *const c_char, c_double) -> *mut cJSON> =
            c._lib.get(b"cJSON_AddNumberToObject").unwrap();
        let r_add_num: Symbol<unsafe extern "C" fn(*mut cJSON, *const c_char, c_double) -> *mut cJSON> =
            r._lib.get(b"cJSON_AddNumberToObject").unwrap();
        c_add_num(co, k.as_ptr(), 3.14);
        r_add_num(ro, k.as_ptr(), 3.14);

        let k = cstr("str");
        let v = cstr("hello");
        let c_add_str: Symbol<unsafe extern "C" fn(*mut cJSON, *const c_char, *const c_char) -> *mut cJSON> =
            c._lib.get(b"cJSON_AddStringToObject").unwrap();
        let r_add_str: Symbol<unsafe extern "C" fn(*mut cJSON, *const c_char, *const c_char) -> *mut cJSON> =
            r._lib.get(b"cJSON_AddStringToObject").unwrap();
        c_add_str(co, k.as_ptr(), v.as_ptr());
        r_add_str(ro, k.as_ptr(), v.as_ptr());

        let k = cstr("raw");
        let v = cstr("{\"x\":1}");
        let c_add_raw: Symbol<unsafe extern "C" fn(*mut cJSON, *const c_char, *const c_char) -> *mut cJSON> =
            c._lib.get(b"cJSON_AddRawToObject").unwrap();
        let r_add_raw: Symbol<unsafe extern "C" fn(*mut cJSON, *const c_char, *const c_char) -> *mut cJSON> =
            r._lib.get(b"cJSON_AddRawToObject").unwrap();
        c_add_raw(co, k.as_ptr(), v.as_ptr());
        r_add_raw(ro, k.as_ptr(), v.as_ptr());

        let k = cstr("obj");
        let c_add_obj: Symbol<unsafe extern "C" fn(*mut cJSON, *const c_char) -> *mut cJSON> =
            c._lib.get(b"cJSON_AddObjectToObject").unwrap();
        let r_add_obj: Symbol<unsafe extern "C" fn(*mut cJSON, *const c_char) -> *mut cJSON> =
            r._lib.get(b"cJSON_AddObjectToObject").unwrap();
        c_add_obj(co, k.as_ptr());
        r_add_obj(ro, k.as_ptr());

        let k = cstr("arr");
        let c_add_arr: Symbol<unsafe extern "C" fn(*mut cJSON, *const c_char) -> *mut cJSON> =
            c._lib.get(b"cJSON_AddArrayToObject").unwrap();
        let r_add_arr: Symbol<unsafe extern "C" fn(*mut cJSON, *const c_char) -> *mut cJSON> =
            r._lib.get(b"cJSON_AddArrayToObject").unwrap();
        c_add_arr(co, k.as_ptr());
        r_add_arr(ro, k.as_ptr());

        let cs = c_print(co);
        let rs = r_print(ro);
        assert_eq!(
            CStr::from_ptr(cs).to_string_lossy(),
            CStr::from_ptr(rs).to_string_lossy(),
            "AddHelpers mismatch"
        );
        c_free(cs as *mut c_void);
        r_free(rs as *mut c_void);
        c_del(co);
        r_del(ro);
    }
}

#[test]
fn test_minify() {
    unsafe {
        let c = load_c();
        let r = load_rust();
        let c_minify: Symbol<unsafe extern "C" fn(*mut c_char)> =
            c._lib.get(b"cJSON_Minify").unwrap();
        let r_minify: Symbol<unsafe extern "C" fn(*mut c_char)> =
            r._lib.get(b"cJSON_Minify").unwrap();

        let cases = [
            r#"{ "a" : 1 , "b" : [ 2 , 3 ] }"#,
            "  [  1  ,  2  ,  3  ]  ",
            "\"hello world\"",
            "{ }",
            "[ ]",
            "// comment\n{\"a\":1}",
            "/* block */\n{\"a\":1}",
        ];

        for case in &cases {
            let mut c_buf: Vec<u8> = case.bytes().chain(std::iter::once(0)).collect();
            let mut r_buf = c_buf.clone();
            c_minify(c_buf.as_mut_ptr() as *mut c_char);
            r_minify(r_buf.as_mut_ptr() as *mut c_char);
            assert_eq!(c_buf, r_buf, "Minify mismatch for: {case}");
        }
    }
}

#[test]
fn test_duplicate() {
    unsafe {
        let c = load_c();
        let r = load_rust();

        let json = cstr(r#"{"a":1,"b":[2,3],"c":{"d":"e"}}"#);
        let c_parse: Symbol<unsafe extern "C" fn(*const c_char) -> *mut cJSON> =
            c._lib.get(b"cJSON_Parse").unwrap();
        let r_parse: Symbol<unsafe extern "C" fn(*const c_char) -> *mut cJSON> =
            r._lib.get(b"cJSON_Parse").unwrap();
        let c_dup: Symbol<unsafe extern "C" fn(*const cJSON, c_int) -> *mut cJSON> =
            c._lib.get(b"cJSON_Duplicate").unwrap();
        let r_dup: Symbol<unsafe extern "C" fn(*const cJSON, c_int) -> *mut cJSON> =
            r._lib.get(b"cJSON_Duplicate").unwrap();
        let c_print: Symbol<unsafe extern "C" fn(*const cJSON) -> *mut c_char> =
            c._lib.get(b"cJSON_Print").unwrap();
        let r_print: Symbol<unsafe extern "C" fn(*const cJSON) -> *mut c_char> =
            r._lib.get(b"cJSON_Print").unwrap();
        let c_del: Symbol<unsafe extern "C" fn(*mut cJSON)> = c._lib.get(b"cJSON_Delete").unwrap();
        let r_del: Symbol<unsafe extern "C" fn(*mut cJSON)> = r._lib.get(b"cJSON_Delete").unwrap();
        let c_free: Symbol<unsafe extern "C" fn(*mut c_void)> = c._lib.get(b"cJSON_free").unwrap();
        let r_free: Symbol<unsafe extern "C" fn(*mut c_void)> = r._lib.get(b"cJSON_free").unwrap();

        let ci = c_parse(json.as_ptr());
        let ri = r_parse(json.as_ptr());

        // Recursive duplicate
        let c_d = c_dup(ci, 1);
        let r_d = r_dup(ri, 1);
        let cs = c_print(c_d);
        let rs = r_print(r_d);
        assert_eq!(
            CStr::from_ptr(cs).to_string_lossy(),
            CStr::from_ptr(rs).to_string_lossy(),
            "Duplicate recursive mismatch"
        );
        c_free(cs as *mut c_void);
        r_free(rs as *mut c_void);

        // Non-recursive duplicate
        let c_d2 = c_dup(ci, 0);
        let r_d2 = r_dup(ri, 0);
        let cs2 = c_print(c_d2);
        let rs2 = r_print(r_d2);
        assert_eq!(
            CStr::from_ptr(cs2).to_string_lossy(),
            CStr::from_ptr(rs2).to_string_lossy(),
            "Duplicate non-recursive mismatch"
        );
        c_free(cs2 as *mut c_void);
        r_free(rs2 as *mut c_void);

        c_del(c_d);
        r_del(r_d);
        c_del(c_d2);
        r_del(r_d2);
        c_del(ci);
        r_del(ri);
    }
}

#[test]
fn test_compare() {
    unsafe {
        let c = load_c();
        let r = load_rust();

        let c_parse: Symbol<unsafe extern "C" fn(*const c_char) -> *mut cJSON> =
            c._lib.get(b"cJSON_Parse").unwrap();
        let r_parse: Symbol<unsafe extern "C" fn(*const c_char) -> *mut cJSON> =
            r._lib.get(b"cJSON_Parse").unwrap();
        let c_cmp: Symbol<unsafe extern "C" fn(*const cJSON, *const cJSON, c_int) -> c_int> =
            c._lib.get(b"cJSON_Compare").unwrap();
        let r_cmp: Symbol<unsafe extern "C" fn(*const cJSON, *const cJSON, c_int) -> c_int> =
            r._lib.get(b"cJSON_Compare").unwrap();
        let c_del: Symbol<unsafe extern "C" fn(*mut cJSON)> = c._lib.get(b"cJSON_Delete").unwrap();
        let r_del: Symbol<unsafe extern "C" fn(*mut cJSON)> = r._lib.get(b"cJSON_Delete").unwrap();

        let cases: &[(&str, &str, c_int)] = &[
            (r#"{"a":1}"#, r#"{"a":1}"#, 1),
            (r#"{"a":1}"#, r#"{"a":2}"#, 1),
            (r#"{"a":1}"#, r#"{"A":1}"#, 1),
            (r#"[1,2,3]"#, r#"[1,2,3]"#, 1),
            (r#"[1,2,3]"#, r#"[1,2,4]"#, 1),
            (r#""hello""#, r#""hello""#, 1),
            (r#""hello""#, r#""world""#, 1),
            (r#"true"#, r#"true"#, 1),
            (r#"true"#, r#"false"#, 1),
            (r#"null"#, r#"null"#, 1),
        ];

        for (a, b, case_sensitive) in cases {
            let a_cs = cstr(a);
            let b_cs = cstr(b);
            let c_a = c_parse(a_cs.as_ptr());
            let c_b = c_parse(b_cs.as_ptr());
            let r_a = r_parse(a_cs.as_ptr());
            let r_b = r_parse(b_cs.as_ptr());

            let c_res = c_cmp(c_a, c_b, *case_sensitive);
            let r_res = r_cmp(r_a, r_b, *case_sensitive);
            assert_eq!(c_res, r_res, "Compare mismatch for ({a}, {b}, cs={case_sensitive})");

            c_del(c_a);
            c_del(c_b);
            r_del(r_a);
            r_del(r_b);
        }

        // Compare with null
        let c_null_res = c_cmp(ptr::null(), ptr::null(), 1);
        let r_null_res = r_cmp(ptr::null(), ptr::null(), 1);
        assert_eq!(c_null_res, r_null_res);
    }
}

#[test]
fn test_detach_and_delete_from_array() {
    unsafe {
        let c = load_c();
        let r = load_rust();

        let json = cstr(r#"[1,2,3,4,5]"#);
        let c_parse: Symbol<unsafe extern "C" fn(*const c_char) -> *mut cJSON> =
            c._lib.get(b"cJSON_Parse").unwrap();
        let r_parse: Symbol<unsafe extern "C" fn(*const c_char) -> *mut cJSON> =
            r._lib.get(b"cJSON_Parse").unwrap();
        let c_detach: Symbol<unsafe extern "C" fn(*mut cJSON, c_int) -> *mut cJSON> =
            c._lib.get(b"cJSON_DetachItemFromArray").unwrap();
        let r_detach: Symbol<unsafe extern "C" fn(*mut cJSON, c_int) -> *mut cJSON> =
            r._lib.get(b"cJSON_DetachItemFromArray").unwrap();
        let c_print: Symbol<unsafe extern "C" fn(*const cJSON) -> *mut c_char> =
            c._lib.get(b"cJSON_Print").unwrap();
        let r_print: Symbol<unsafe extern "C" fn(*const cJSON) -> *mut c_char> =
            r._lib.get(b"cJSON_Print").unwrap();
        let c_del: Symbol<unsafe extern "C" fn(*mut cJSON)> = c._lib.get(b"cJSON_Delete").unwrap();
        let r_del: Symbol<unsafe extern "C" fn(*mut cJSON)> = r._lib.get(b"cJSON_Delete").unwrap();
        let c_free: Symbol<unsafe extern "C" fn(*mut c_void)> = c._lib.get(b"cJSON_free").unwrap();
        let r_free: Symbol<unsafe extern "C" fn(*mut c_void)> = r._lib.get(b"cJSON_free").unwrap();

        let ci = c_parse(json.as_ptr());
        let ri = r_parse(json.as_ptr());

        // Detach item at index 2 (value 3)
        let c_detached = c_detach(ci, 2);
        let r_detached = r_detach(ri, 2);
        assert_eq!((*c_detached).valuedouble, (*r_detached).valuedouble);

        // Print remaining array
        let cs = c_print(ci);
        let rs = r_print(ri);
        assert_eq!(
            CStr::from_ptr(cs).to_string_lossy(),
            CStr::from_ptr(rs).to_string_lossy(),
            "After detach mismatch"
        );
        c_free(cs as *mut c_void);
        r_free(rs as *mut c_void);

        c_del(c_detached);
        r_del(r_detached);

        // Now delete item at index 0
        let c_del_arr: Symbol<unsafe extern "C" fn(*mut cJSON, c_int)> =
            c._lib.get(b"cJSON_DeleteItemFromArray").unwrap();
        let r_del_arr: Symbol<unsafe extern "C" fn(*mut cJSON, c_int)> =
            r._lib.get(b"cJSON_DeleteItemFromArray").unwrap();
        c_del_arr(ci, 0);
        r_del_arr(ri, 0);

        let cs = c_print(ci);
        let rs = r_print(ri);
        assert_eq!(
            CStr::from_ptr(cs).to_string_lossy(),
            CStr::from_ptr(rs).to_string_lossy(),
            "After delete mismatch"
        );
        c_free(cs as *mut c_void);
        r_free(rs as *mut c_void);

        c_del(ci);
        r_del(ri);
    }
}

#[test]
fn test_detach_and_delete_from_object() {
    unsafe {
        let c = load_c();
        let r = load_rust();

        let json = cstr(r#"{"a":1,"b":2,"c":3}"#);
        let c_parse: Symbol<unsafe extern "C" fn(*const c_char) -> *mut cJSON> =
            c._lib.get(b"cJSON_Parse").unwrap();
        let r_parse: Symbol<unsafe extern "C" fn(*const c_char) -> *mut cJSON> =
            r._lib.get(b"cJSON_Parse").unwrap();
        let c_detach: Symbol<unsafe extern "C" fn(*mut cJSON, *const c_char) -> *mut cJSON> =
            c._lib.get(b"cJSON_DetachItemFromObject").unwrap();
        let r_detach: Symbol<unsafe extern "C" fn(*mut cJSON, *const c_char) -> *mut cJSON> =
            r._lib.get(b"cJSON_DetachItemFromObject").unwrap();
        let c_detach_cs: Symbol<unsafe extern "C" fn(*mut cJSON, *const c_char) -> *mut cJSON> =
            c._lib.get(b"cJSON_DetachItemFromObjectCaseSensitive").unwrap();
        let r_detach_cs: Symbol<unsafe extern "C" fn(*mut cJSON, *const c_char) -> *mut cJSON> =
            r._lib.get(b"cJSON_DetachItemFromObjectCaseSensitive").unwrap();
        let c_print: Symbol<unsafe extern "C" fn(*const cJSON) -> *mut c_char> =
            c._lib.get(b"cJSON_Print").unwrap();
        let r_print: Symbol<unsafe extern "C" fn(*const cJSON) -> *mut c_char> =
            r._lib.get(b"cJSON_Print").unwrap();
        let c_del: Symbol<unsafe extern "C" fn(*mut cJSON)> = c._lib.get(b"cJSON_Delete").unwrap();
        let r_del: Symbol<unsafe extern "C" fn(*mut cJSON)> = r._lib.get(b"cJSON_Delete").unwrap();
        let c_free: Symbol<unsafe extern "C" fn(*mut c_void)> = c._lib.get(b"cJSON_free").unwrap();
        let r_free: Symbol<unsafe extern "C" fn(*mut c_void)> = r._lib.get(b"cJSON_free").unwrap();

        let ci = c_parse(json.as_ptr());
        let ri = r_parse(json.as_ptr());

        // Detach "b" (case insensitive)
        let key = cstr("b");
        let c_d = c_detach(ci, key.as_ptr());
        let r_d = r_detach(ri, key.as_ptr());
        assert!(!c_d.is_null());
        assert!(!r_d.is_null());
        c_del(c_d);
        r_del(r_d);

        let cs = c_print(ci);
        let rs = r_print(ri);
        assert_eq!(
            CStr::from_ptr(cs).to_string_lossy(),
            CStr::from_ptr(rs).to_string_lossy()
        );
        c_free(cs as *mut c_void);
        r_free(rs as *mut c_void);

        // Delete "a" from object (case insensitive)
        let c_del_obj: Symbol<unsafe extern "C" fn(*mut cJSON, *const c_char)> =
            c._lib.get(b"cJSON_DeleteItemFromObject").unwrap();
        let r_del_obj: Symbol<unsafe extern "C" fn(*mut cJSON, *const c_char)> =
            r._lib.get(b"cJSON_DeleteItemFromObject").unwrap();
        let key_a = cstr("a");
        c_del_obj(ci, key_a.as_ptr());
        r_del_obj(ri, key_a.as_ptr());

        let cs = c_print(ci);
        let rs = r_print(ri);
        assert_eq!(
            CStr::from_ptr(cs).to_string_lossy(),
            CStr::from_ptr(rs).to_string_lossy()
        );
        c_free(cs as *mut c_void);
        r_free(rs as *mut c_void);

        c_del(ci);
        r_del(ri);
    }
}
