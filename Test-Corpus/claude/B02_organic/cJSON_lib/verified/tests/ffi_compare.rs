//! Integration tests comparing the C implementation against the Rust port.
//!
//! Both libraries are loaded via `libloading`, ensuring we exercise the
//! `#[no_mangle]` exports rather than calling Rust functions directly.

use libloading::{Library, Symbol};
use std::ffi::{c_char, c_double, c_int, c_void, CStr, CString};
use std::os::raw::c_uchar;
use std::path::PathBuf;
use std::ptr;
use std::sync::OnceLock;

#[repr(C)]
#[derive(Debug)]
pub struct cJSON {
    pub next: *mut cJSON,
    pub prev: *mut cJSON,
    pub child: *mut cJSON,
    pub r#type: c_int,
    pub valuestring: *mut c_char,
    pub valueint: c_int,
    pub valuedouble: c_double,
    pub string: *mut c_char,
}

pub const cJSON_Invalid: c_int = 0;
pub const cJSON_False: c_int = 1 << 0;
pub const cJSON_True: c_int = 1 << 1;
pub const cJSON_NULL: c_int = 1 << 2;
pub const cJSON_Number: c_int = 1 << 3;
pub const cJSON_String: c_int = 1 << 4;
pub const cJSON_Array: c_int = 1 << 5;
pub const cJSON_Object: c_int = 1 << 6;
pub const cJSON_Raw: c_int = 1 << 7;

fn workspace_root() -> PathBuf {
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.push("");
    p
}

fn c_so_path() -> PathBuf {
    let mut p = workspace_root();
    p.push("c_src");
    p.push("build");
    p.push("libcjson.so");
    p
}

fn rust_so_path() -> PathBuf {
    // The crate type is cdylib named "cJSON_test", so target file is libcJSON_test.so
    let target = std::env::var("CARGO_TARGET_DIR").unwrap_or_else(|_| {
        let mut p = workspace_root();
        p.push("target");
        p.to_string_lossy().to_string()
    });
    let mut p = PathBuf::from(target);
    p.push("debug");
    p.push("libcJSON_test.so");
    p
}

fn ensure_rust_so_built() {
    use std::process::Command;
    let so = rust_so_path();
    if !so.exists() {
        let status = Command::new(env!("CARGO"))
            .args(["build"])
            .current_dir(workspace_root())
            .status()
            .expect("cargo build to run");
        assert!(status.success(), "cargo build failed");
    }
}

static C_LIB: OnceLock<Library> = OnceLock::new();
static RUST_LIB: OnceLock<Library> = OnceLock::new();

fn c_lib() -> &'static Library {
    C_LIB.get_or_init(|| unsafe {
        Library::new(c_so_path()).expect("failed to load C cJSON library")
    })
}

fn rust_lib() -> &'static Library {
    RUST_LIB.get_or_init(|| {
        ensure_rust_so_built();
        unsafe { Library::new(rust_so_path()).expect("failed to load Rust cJSON library") }
    })
}

unsafe fn sym<'a, T>(lib: &'a Library, name: &str) -> Symbol<'a, T> {
    lib.get(name.as_bytes()).unwrap_or_else(|e| {
        panic!("symbol {} not found: {}", name, e);
    })
}

// ==========================================================================
// Function signature types
// ==========================================================================
type Fn_Version = unsafe extern "C" fn() -> *const c_char;
type Fn_Parse = unsafe extern "C" fn(*const c_char) -> *mut cJSON;
type Fn_ParseWithLength = unsafe extern "C" fn(*const c_char, usize) -> *mut cJSON;
type Fn_ParseWithOpts = unsafe extern "C" fn(*const c_char, *mut *const c_char, c_int) -> *mut cJSON;
type Fn_Print = unsafe extern "C" fn(*const cJSON) -> *mut c_char;
type Fn_PrintUnformatted = unsafe extern "C" fn(*const cJSON) -> *mut c_char;
type Fn_PrintBuffered = unsafe extern "C" fn(*const cJSON, c_int, c_int) -> *mut c_char;
type Fn_PrintPreallocated = unsafe extern "C" fn(*mut cJSON, *mut c_char, c_int, c_int) -> c_int;
type Fn_Delete = unsafe extern "C" fn(*mut cJSON);
type Fn_GetArraySize = unsafe extern "C" fn(*const cJSON) -> c_int;
type Fn_GetArrayItem = unsafe extern "C" fn(*const cJSON, c_int) -> *mut cJSON;
type Fn_GetObjectItem = unsafe extern "C" fn(*const cJSON, *const c_char) -> *mut cJSON;
type Fn_GetObjectItemCaseSensitive = unsafe extern "C" fn(*const cJSON, *const c_char) -> *mut cJSON;
type Fn_HasObjectItem = unsafe extern "C" fn(*const cJSON, *const c_char) -> c_int;
type Fn_GetErrorPtr = unsafe extern "C" fn() -> *const c_char;
type Fn_GetStringValue = unsafe extern "C" fn(*const cJSON) -> *mut c_char;
type Fn_GetNumberValue = unsafe extern "C" fn(*const cJSON) -> c_double;
type Fn_IsX = unsafe extern "C" fn(*const cJSON) -> c_int;
type Fn_CreateNull = unsafe extern "C" fn() -> *mut cJSON;
type Fn_CreateBool = unsafe extern "C" fn(c_int) -> *mut cJSON;
type Fn_CreateNumber = unsafe extern "C" fn(c_double) -> *mut cJSON;
type Fn_CreateString = unsafe extern "C" fn(*const c_char) -> *mut cJSON;
type Fn_CreateRaw = unsafe extern "C" fn(*const c_char) -> *mut cJSON;
type Fn_CreateArray = unsafe extern "C" fn() -> *mut cJSON;
type Fn_CreateObject = unsafe extern "C" fn() -> *mut cJSON;
type Fn_CreateStringReference = unsafe extern "C" fn(*const c_char) -> *mut cJSON;
type Fn_CreateObjectReference = unsafe extern "C" fn(*const cJSON) -> *mut cJSON;
type Fn_CreateArrayReference = unsafe extern "C" fn(*const cJSON) -> *mut cJSON;
type Fn_CreateIntArray = unsafe extern "C" fn(*const c_int, c_int) -> *mut cJSON;
type Fn_CreateFloatArray = unsafe extern "C" fn(*const f32, c_int) -> *mut cJSON;
type Fn_CreateDoubleArray = unsafe extern "C" fn(*const c_double, c_int) -> *mut cJSON;
type Fn_CreateStringArray = unsafe extern "C" fn(*const *const c_char, c_int) -> *mut cJSON;
type Fn_AddItemToArray = unsafe extern "C" fn(*mut cJSON, *mut cJSON) -> c_int;
type Fn_AddItemToObject = unsafe extern "C" fn(*mut cJSON, *const c_char, *mut cJSON) -> c_int;
type Fn_AddItemToObjectCS = unsafe extern "C" fn(*mut cJSON, *const c_char, *mut cJSON) -> c_int;
type Fn_AddItemReferenceToArray = unsafe extern "C" fn(*mut cJSON, *mut cJSON) -> c_int;
type Fn_AddItemReferenceToObject = unsafe extern "C" fn(*mut cJSON, *const c_char, *mut cJSON) -> c_int;
type Fn_DetachItemViaPointer = unsafe extern "C" fn(*mut cJSON, *mut cJSON) -> *mut cJSON;
type Fn_DetachItemFromArray = unsafe extern "C" fn(*mut cJSON, c_int) -> *mut cJSON;
type Fn_DeleteItemFromArray = unsafe extern "C" fn(*mut cJSON, c_int);
type Fn_DetachItemFromObject = unsafe extern "C" fn(*mut cJSON, *const c_char) -> *mut cJSON;
type Fn_DetachItemFromObjectCaseSensitive = unsafe extern "C" fn(*mut cJSON, *const c_char) -> *mut cJSON;
type Fn_DeleteItemFromObject = unsafe extern "C" fn(*mut cJSON, *const c_char);
type Fn_DeleteItemFromObjectCaseSensitive = unsafe extern "C" fn(*mut cJSON, *const c_char);
type Fn_InsertItemInArray = unsafe extern "C" fn(*mut cJSON, c_int, *mut cJSON) -> c_int;
type Fn_ReplaceItemViaPointer = unsafe extern "C" fn(*mut cJSON, *mut cJSON, *mut cJSON) -> c_int;
type Fn_ReplaceItemInArray = unsafe extern "C" fn(*mut cJSON, c_int, *mut cJSON) -> c_int;
type Fn_ReplaceItemInObject = unsafe extern "C" fn(*mut cJSON, *const c_char, *mut cJSON) -> c_int;
type Fn_ReplaceItemInObjectCaseSensitive = unsafe extern "C" fn(*mut cJSON, *const c_char, *mut cJSON) -> c_int;
type Fn_Duplicate = unsafe extern "C" fn(*const cJSON, c_int) -> *mut cJSON;
type Fn_Compare = unsafe extern "C" fn(*const cJSON, *const cJSON, c_int) -> c_int;
type Fn_Minify = unsafe extern "C" fn(*mut c_char);
type Fn_AddNullToObject = unsafe extern "C" fn(*mut cJSON, *const c_char) -> *mut cJSON;
type Fn_AddTrueToObject = unsafe extern "C" fn(*mut cJSON, *const c_char) -> *mut cJSON;
type Fn_AddFalseToObject = unsafe extern "C" fn(*mut cJSON, *const c_char) -> *mut cJSON;
type Fn_AddBoolToObject = unsafe extern "C" fn(*mut cJSON, *const c_char, c_int) -> *mut cJSON;
type Fn_AddNumberToObject = unsafe extern "C" fn(*mut cJSON, *const c_char, c_double) -> *mut cJSON;
type Fn_AddStringToObject = unsafe extern "C" fn(*mut cJSON, *const c_char, *const c_char) -> *mut cJSON;
type Fn_AddRawToObject = unsafe extern "C" fn(*mut cJSON, *const c_char, *const c_char) -> *mut cJSON;
type Fn_AddObjectToObject = unsafe extern "C" fn(*mut cJSON, *const c_char) -> *mut cJSON;
type Fn_AddArrayToObject = unsafe extern "C" fn(*mut cJSON, *const c_char) -> *mut cJSON;
type Fn_SetNumberHelper = unsafe extern "C" fn(*mut cJSON, c_double) -> c_double;
type Fn_SetValuestring = unsafe extern "C" fn(*mut cJSON, *const c_char) -> *mut c_char;
type Fn_malloc = unsafe extern "C" fn(usize) -> *mut c_void;
type Fn_free = unsafe extern "C" fn(*mut c_void);

extern "C" {
    fn strcmp(a: *const c_char, b: *const c_char) -> c_int;
    fn strlen(s: *const c_char) -> usize;
    fn free(p: *mut c_void);
}

unsafe fn cstr_eq(a: *const c_char, b: *const c_char) -> bool {
    if a.is_null() && b.is_null() {
        return true;
    }
    if a.is_null() || b.is_null() {
        return false;
    }
    strcmp(a, b) == 0
}

fn rust_str(p: *const c_char) -> Option<String> {
    if p.is_null() {
        return None;
    }
    Some(unsafe { CStr::from_ptr(p).to_string_lossy().into_owned() })
}

// ==========================================================================
// Tests
// ==========================================================================

#[test]
fn version_matches() {
    unsafe {
        let cv: Symbol<Fn_Version> = sym(c_lib(), "cJSON_Version");
        let rv: Symbol<Fn_Version> = sym(rust_lib(), "cJSON_Version");
        let cs = CStr::from_ptr(cv()).to_string_lossy().into_owned();
        let rs = CStr::from_ptr(rv()).to_string_lossy().into_owned();
        assert_eq!(cs, rs);
        assert_eq!(cs, "1.7.19");
    }
}

#[test]
fn create_and_print_null() {
    unsafe {
        let c_create: Symbol<Fn_CreateNull> = sym(c_lib(), "cJSON_CreateNull");
        let r_create: Symbol<Fn_CreateNull> = sym(rust_lib(), "cJSON_CreateNull");
        let c_print: Symbol<Fn_PrintUnformatted> = sym(c_lib(), "cJSON_PrintUnformatted");
        let r_print: Symbol<Fn_PrintUnformatted> = sym(rust_lib(), "cJSON_PrintUnformatted");
        let c_del: Symbol<Fn_Delete> = sym(c_lib(), "cJSON_Delete");
        let r_del: Symbol<Fn_Delete> = sym(rust_lib(), "cJSON_Delete");

        let c_obj = c_create();
        let r_obj = r_create();
        let cp = c_print(c_obj);
        let rp = r_print(r_obj);
        assert!(cstr_eq(cp, rp), "C: {:?}, Rust: {:?}", rust_str(cp), rust_str(rp));
        free(cp as *mut c_void);
        free(rp as *mut c_void);
        c_del(c_obj);
        r_del(r_obj);
    }
}

#[test]
fn create_and_print_bool() {
    unsafe {
        let c_create: Symbol<Fn_CreateBool> = sym(c_lib(), "cJSON_CreateBool");
        let r_create: Symbol<Fn_CreateBool> = sym(rust_lib(), "cJSON_CreateBool");
        let c_print: Symbol<Fn_PrintUnformatted> = sym(c_lib(), "cJSON_PrintUnformatted");
        let r_print: Symbol<Fn_PrintUnformatted> = sym(rust_lib(), "cJSON_PrintUnformatted");
        let c_del: Symbol<Fn_Delete> = sym(c_lib(), "cJSON_Delete");
        let r_del: Symbol<Fn_Delete> = sym(rust_lib(), "cJSON_Delete");

        for v in &[0, 1, 5] {
            let c_obj = c_create(*v);
            let r_obj = r_create(*v);
            let cp = c_print(c_obj);
            let rp = r_print(r_obj);
            assert!(cstr_eq(cp, rp), "v={}, C: {:?}, Rust: {:?}", v, rust_str(cp), rust_str(rp));
            free(cp as *mut c_void);
            free(rp as *mut c_void);
            c_del(c_obj);
            r_del(r_obj);
        }
    }
}

#[test]
fn create_and_print_number() {
    unsafe {
        let c_create: Symbol<Fn_CreateNumber> = sym(c_lib(), "cJSON_CreateNumber");
        let r_create: Symbol<Fn_CreateNumber> = sym(rust_lib(), "cJSON_CreateNumber");
        let c_print: Symbol<Fn_PrintUnformatted> = sym(c_lib(), "cJSON_PrintUnformatted");
        let r_print: Symbol<Fn_PrintUnformatted> = sym(rust_lib(), "cJSON_PrintUnformatted");
        let c_del: Symbol<Fn_Delete> = sym(c_lib(), "cJSON_Delete");
        let r_del: Symbol<Fn_Delete> = sym(rust_lib(), "cJSON_Delete");

        let nums = [0.0, 1.0, -1.0, 3.14159265358979, 1e10, 1e-10, 1234567890.5,
                    -0.0, f64::INFINITY, f64::NEG_INFINITY, f64::NAN, 1.5e308, 1e-300, 100000000000000.0];
        for &v in nums.iter() {
            let c_obj = c_create(v);
            let r_obj = r_create(v);
            let cp = c_print(c_obj);
            let rp = r_print(r_obj);
            assert!(cstr_eq(cp, rp),
                "n={}, C: {:?}, Rust: {:?}", v, rust_str(cp), rust_str(rp));
            free(cp as *mut c_void);
            free(rp as *mut c_void);
            c_del(c_obj);
            r_del(r_obj);
        }
    }
}

#[test]
fn create_and_print_string() {
    unsafe {
        let c_create: Symbol<Fn_CreateString> = sym(c_lib(), "cJSON_CreateString");
        let r_create: Symbol<Fn_CreateString> = sym(rust_lib(), "cJSON_CreateString");
        let c_print: Symbol<Fn_PrintUnformatted> = sym(c_lib(), "cJSON_PrintUnformatted");
        let r_print: Symbol<Fn_PrintUnformatted> = sym(rust_lib(), "cJSON_PrintUnformatted");
        let c_del: Symbol<Fn_Delete> = sym(c_lib(), "cJSON_Delete");
        let r_del: Symbol<Fn_Delete> = sym(rust_lib(), "cJSON_Delete");

        let strs: &[&[u8]] = &[
            b"hello\0",
            b"\0",
            b"\\\"\n\t\\\\\0",
            b"unicode \xe2\x98\x83\0",
            b"control \x01\x02\x1f\0",
            b"emoji \xf0\x9f\x98\x80\0",
        ];
        for &s in strs {
            let p = s.as_ptr() as *const c_char;
            let c_obj = c_create(p);
            let r_obj = r_create(p);
            let cp = c_print(c_obj);
            let rp = r_print(r_obj);
            assert!(cstr_eq(cp, rp),
                "s={:?}, C: {:?}, Rust: {:?}", std::str::from_utf8(s), rust_str(cp), rust_str(rp));
            free(cp as *mut c_void);
            free(rp as *mut c_void);
            c_del(c_obj);
            r_del(r_obj);
        }
    }
}

#[test]
fn parse_and_reprint() {
    unsafe {
        let c_parse: Symbol<Fn_Parse> = sym(c_lib(), "cJSON_Parse");
        let r_parse: Symbol<Fn_Parse> = sym(rust_lib(), "cJSON_Parse");
        let c_print: Symbol<Fn_Print> = sym(c_lib(), "cJSON_Print");
        let r_print: Symbol<Fn_Print> = sym(rust_lib(), "cJSON_Print");
        let c_print_u: Symbol<Fn_PrintUnformatted> = sym(c_lib(), "cJSON_PrintUnformatted");
        let r_print_u: Symbol<Fn_PrintUnformatted> = sym(rust_lib(), "cJSON_PrintUnformatted");
        let c_del: Symbol<Fn_Delete> = sym(c_lib(), "cJSON_Delete");
        let r_del: Symbol<Fn_Delete> = sym(rust_lib(), "cJSON_Delete");

        let inputs: &[&[u8]] = &[
            b"null\0",
            b"true\0",
            b"false\0",
            b"123\0",
            b"-3.14\0",
            b"\"hello\"\0",
            b"[1,2,3]\0",
            b"{\"a\":1,\"b\":[true,false,null]}\0",
            b"   { \"x\" : [1.5, -2.5, 3.5e10] , \"y\" : \"str with \\\"quotes\\\"\" } \0",
            b"[]\0",
            b"{}\0",
            b"[[[1,2,3],[4,5]],[]]\0",
            b"{\"key\\u00e9\":\"value\"}\0",
        ];
        for &input in inputs {
            let p = input.as_ptr() as *const c_char;
            let c_obj = c_parse(p);
            let r_obj = r_parse(p);
            assert!(!c_obj.is_null(), "C parse failed: {:?}", std::str::from_utf8(input));
            assert!(!r_obj.is_null(), "Rust parse failed: {:?}", std::str::from_utf8(input));

            let cp = c_print(c_obj);
            let rp = r_print(r_obj);
            assert!(cstr_eq(cp, rp), "PRINT: {:?}\nC:   {:?}\nRust:{:?}",
                std::str::from_utf8(input), rust_str(cp), rust_str(rp));
            free(cp as *mut c_void);
            free(rp as *mut c_void);

            let cp = c_print_u(c_obj);
            let rp = r_print_u(r_obj);
            assert!(cstr_eq(cp, rp), "UNFORMATTED: {:?}\nC:   {:?}\nRust:{:?}",
                std::str::from_utf8(input), rust_str(cp), rust_str(rp));
            free(cp as *mut c_void);
            free(rp as *mut c_void);

            c_del(c_obj);
            r_del(r_obj);
        }
    }
}

#[test]
fn parse_invalid_returns_null() {
    unsafe {
        let c_parse: Symbol<Fn_Parse> = sym(c_lib(), "cJSON_Parse");
        let r_parse: Symbol<Fn_Parse> = sym(rust_lib(), "cJSON_Parse");
        let inputs: &[&[u8]] = &[
            b"\0",
            b"   \0",
            b"abc\0",
            b"{\0",
            b"[1,]\0",
            b"\"unterminated\0",
        ];
        for &input in inputs {
            let p = input.as_ptr() as *const c_char;
            let c_obj = c_parse(p);
            let r_obj = r_parse(p);
            assert_eq!(c_obj.is_null(), r_obj.is_null(),
                "input={:?} C null={} Rust null={}",
                std::str::from_utf8(input), c_obj.is_null(), r_obj.is_null());
        }
    }
}

#[test]
fn array_operations() {
    unsafe {
        let c_create_arr: Symbol<Fn_CreateArray> = sym(c_lib(), "cJSON_CreateArray");
        let r_create_arr: Symbol<Fn_CreateArray> = sym(rust_lib(), "cJSON_CreateArray");
        let c_create_num: Symbol<Fn_CreateNumber> = sym(c_lib(), "cJSON_CreateNumber");
        let r_create_num: Symbol<Fn_CreateNumber> = sym(rust_lib(), "cJSON_CreateNumber");
        let c_add: Symbol<Fn_AddItemToArray> = sym(c_lib(), "cJSON_AddItemToArray");
        let r_add: Symbol<Fn_AddItemToArray> = sym(rust_lib(), "cJSON_AddItemToArray");
        let c_size: Symbol<Fn_GetArraySize> = sym(c_lib(), "cJSON_GetArraySize");
        let r_size: Symbol<Fn_GetArraySize> = sym(rust_lib(), "cJSON_GetArraySize");
        let c_at: Symbol<Fn_GetArrayItem> = sym(c_lib(), "cJSON_GetArrayItem");
        let r_at: Symbol<Fn_GetArrayItem> = sym(rust_lib(), "cJSON_GetArrayItem");
        let c_print: Symbol<Fn_PrintUnformatted> = sym(c_lib(), "cJSON_PrintUnformatted");
        let r_print: Symbol<Fn_PrintUnformatted> = sym(rust_lib(), "cJSON_PrintUnformatted");
        let c_del: Symbol<Fn_Delete> = sym(c_lib(), "cJSON_Delete");
        let r_del: Symbol<Fn_Delete> = sym(rust_lib(), "cJSON_Delete");

        let c_arr = c_create_arr();
        let r_arr = r_create_arr();
        for i in 0..5 {
            c_add(c_arr, c_create_num(i as c_double));
            r_add(r_arr, r_create_num(i as c_double));
        }
        assert_eq!(c_size(c_arr), r_size(r_arr));
        for i in 0..5 {
            let ca = c_at(c_arr, i);
            let ra = r_at(r_arr, i);
            assert!(!ca.is_null());
            assert!(!ra.is_null());
            assert_eq!((*ca).valuedouble, (*ra).valuedouble);
            assert_eq!((*ca).valueint, (*ra).valueint);
        }
        let cp = c_print(c_arr);
        let rp = r_print(r_arr);
        assert!(cstr_eq(cp, rp), "C={:?} R={:?}", rust_str(cp), rust_str(rp));
        free(cp as *mut c_void);
        free(rp as *mut c_void);
        c_del(c_arr);
        r_del(r_arr);
    }
}

#[test]
fn object_operations() {
    unsafe {
        let c_create: Symbol<Fn_CreateObject> = sym(c_lib(), "cJSON_CreateObject");
        let r_create: Symbol<Fn_CreateObject> = sym(rust_lib(), "cJSON_CreateObject");
        let c_add_str: Symbol<Fn_AddStringToObject> = sym(c_lib(), "cJSON_AddStringToObject");
        let r_add_str: Symbol<Fn_AddStringToObject> = sym(rust_lib(), "cJSON_AddStringToObject");
        let c_add_num: Symbol<Fn_AddNumberToObject> = sym(c_lib(), "cJSON_AddNumberToObject");
        let r_add_num: Symbol<Fn_AddNumberToObject> = sym(rust_lib(), "cJSON_AddNumberToObject");
        let c_get: Symbol<Fn_GetObjectItem> = sym(c_lib(), "cJSON_GetObjectItem");
        let r_get: Symbol<Fn_GetObjectItem> = sym(rust_lib(), "cJSON_GetObjectItem");
        let c_get_cs: Symbol<Fn_GetObjectItemCaseSensitive> = sym(c_lib(), "cJSON_GetObjectItemCaseSensitive");
        let r_get_cs: Symbol<Fn_GetObjectItemCaseSensitive> = sym(rust_lib(), "cJSON_GetObjectItemCaseSensitive");
        let c_has: Symbol<Fn_HasObjectItem> = sym(c_lib(), "cJSON_HasObjectItem");
        let r_has: Symbol<Fn_HasObjectItem> = sym(rust_lib(), "cJSON_HasObjectItem");
        let c_print: Symbol<Fn_PrintUnformatted> = sym(c_lib(), "cJSON_PrintUnformatted");
        let r_print: Symbol<Fn_PrintUnformatted> = sym(rust_lib(), "cJSON_PrintUnformatted");
        let c_del: Symbol<Fn_Delete> = sym(c_lib(), "cJSON_Delete");
        let r_del: Symbol<Fn_Delete> = sym(rust_lib(), "cJSON_Delete");

        let c_obj = c_create();
        let r_obj = r_create();
        c_add_str(c_obj, b"name\0".as_ptr() as *const c_char, b"alice\0".as_ptr() as *const c_char);
        r_add_str(r_obj, b"name\0".as_ptr() as *const c_char, b"alice\0".as_ptr() as *const c_char);
        c_add_num(c_obj, b"age\0".as_ptr() as *const c_char, 30.0);
        r_add_num(r_obj, b"age\0".as_ptr() as *const c_char, 30.0);

        // case-insensitive
        let key = b"NAME\0".as_ptr() as *const c_char;
        let cg = c_get(c_obj, key);
        let rg = r_get(r_obj, key);
        assert!(!cg.is_null() && !rg.is_null());

        let cgcs = c_get_cs(c_obj, key);
        let rgcs = r_get_cs(r_obj, key);
        assert_eq!(cgcs.is_null(), rgcs.is_null());

        assert_eq!(c_has(c_obj, b"age\0".as_ptr() as *const c_char),
                   r_has(r_obj, b"age\0".as_ptr() as *const c_char));
        assert_eq!(c_has(c_obj, b"missing\0".as_ptr() as *const c_char),
                   r_has(r_obj, b"missing\0".as_ptr() as *const c_char));

        let cp = c_print(c_obj);
        let rp = r_print(r_obj);
        assert!(cstr_eq(cp, rp), "C={:?} R={:?}", rust_str(cp), rust_str(rp));
        free(cp as *mut c_void);
        free(rp as *mut c_void);
        c_del(c_obj);
        r_del(r_obj);
    }
}

#[test]
fn type_predicates() {
    unsafe {
        let names = [
            ("cJSON_IsInvalid", false),
            ("cJSON_IsFalse", false),
            ("cJSON_IsTrue", true),
            ("cJSON_IsBool", true),
            ("cJSON_IsNull", false),
            ("cJSON_IsNumber", false),
            ("cJSON_IsString", false),
            ("cJSON_IsArray", false),
            ("cJSON_IsObject", false),
            ("cJSON_IsRaw", false),
        ];
        let r_create_true: Symbol<Fn_CreateBool> = sym(rust_lib(), "cJSON_CreateBool");
        let c_create_true: Symbol<Fn_CreateBool> = sym(c_lib(), "cJSON_CreateBool");
        let c_obj = c_create_true(1);
        let r_obj = r_create_true(1);
        for (n, _expected) in &names {
            let c_fn: Symbol<Fn_IsX> = sym(c_lib(), n);
            let r_fn: Symbol<Fn_IsX> = sym(rust_lib(), n);
            let cv = c_fn(c_obj);
            let rv = r_fn(r_obj);
            assert_eq!(cv, rv, "{}", n);
        }
        let c_del: Symbol<Fn_Delete> = sym(c_lib(), "cJSON_Delete");
        let r_del: Symbol<Fn_Delete> = sym(rust_lib(), "cJSON_Delete");
        c_del(c_obj);
        r_del(r_obj);
    }
}

#[test]
fn duplicate_then_compare() {
    unsafe {
        let c_parse: Symbol<Fn_Parse> = sym(c_lib(), "cJSON_Parse");
        let r_parse: Symbol<Fn_Parse> = sym(rust_lib(), "cJSON_Parse");
        let c_dup: Symbol<Fn_Duplicate> = sym(c_lib(), "cJSON_Duplicate");
        let r_dup: Symbol<Fn_Duplicate> = sym(rust_lib(), "cJSON_Duplicate");
        let c_cmp: Symbol<Fn_Compare> = sym(c_lib(), "cJSON_Compare");
        let r_cmp: Symbol<Fn_Compare> = sym(rust_lib(), "cJSON_Compare");
        let c_print: Symbol<Fn_PrintUnformatted> = sym(c_lib(), "cJSON_PrintUnformatted");
        let r_print: Symbol<Fn_PrintUnformatted> = sym(rust_lib(), "cJSON_PrintUnformatted");
        let c_del: Symbol<Fn_Delete> = sym(c_lib(), "cJSON_Delete");
        let r_del: Symbol<Fn_Delete> = sym(rust_lib(), "cJSON_Delete");

        let input = b"{\"a\":[1,2,3],\"b\":\"hello\",\"c\":{\"d\":true}}\0";
        let p = input.as_ptr() as *const c_char;

        let c1 = c_parse(p);
        let r1 = r_parse(p);
        let c2 = c_dup(c1, 1);
        let r2 = r_dup(r1, 1);

        let cp = c_print(c2);
        let rp = r_print(r2);
        assert!(cstr_eq(cp, rp));
        free(cp as *mut c_void);
        free(rp as *mut c_void);

        assert_eq!(c_cmp(c1, c2, 1), r_cmp(r1, r2, 1));
        assert_eq!(c_cmp(c1, c1, 1), r_cmp(r1, r1, 1));

        c_del(c1);
        c_del(c2);
        r_del(r1);
        r_del(r2);
    }
}

#[test]
fn minify_matches() {
    unsafe {
        let c_min: Symbol<Fn_Minify> = sym(c_lib(), "cJSON_Minify");
        let r_min: Symbol<Fn_Minify> = sym(rust_lib(), "cJSON_Minify");

        let input = b"{ \"a\" : 1 ,\n\t\"b\" : [ 1, 2, 3 ] // comment here\n}\0";
        let mut c_buf = input.to_vec();
        let mut r_buf = input.to_vec();
        c_min(c_buf.as_mut_ptr() as *mut c_char);
        r_min(r_buf.as_mut_ptr() as *mut c_char);
        assert_eq!(c_buf, r_buf);
    }
}

#[test]
fn create_int_array() {
    unsafe {
        let c_create: Symbol<Fn_CreateIntArray> = sym(c_lib(), "cJSON_CreateIntArray");
        let r_create: Symbol<Fn_CreateIntArray> = sym(rust_lib(), "cJSON_CreateIntArray");
        let c_print: Symbol<Fn_PrintUnformatted> = sym(c_lib(), "cJSON_PrintUnformatted");
        let r_print: Symbol<Fn_PrintUnformatted> = sym(rust_lib(), "cJSON_PrintUnformatted");
        let c_del: Symbol<Fn_Delete> = sym(c_lib(), "cJSON_Delete");
        let r_del: Symbol<Fn_Delete> = sym(rust_lib(), "cJSON_Delete");

        let nums: [c_int; 5] = [1, 2, 3, -100, 0];
        let c_arr = c_create(nums.as_ptr(), nums.len() as c_int);
        let r_arr = r_create(nums.as_ptr(), nums.len() as c_int);
        let cp = c_print(c_arr);
        let rp = r_print(r_arr);
        assert!(cstr_eq(cp, rp), "C={:?} R={:?}", rust_str(cp), rust_str(rp));
        free(cp as *mut c_void);
        free(rp as *mut c_void);
        c_del(c_arr);
        r_del(r_arr);
    }
}

#[test]
fn create_double_array() {
    unsafe {
        let c_create: Symbol<Fn_CreateDoubleArray> = sym(c_lib(), "cJSON_CreateDoubleArray");
        let r_create: Symbol<Fn_CreateDoubleArray> = sym(rust_lib(), "cJSON_CreateDoubleArray");
        let c_print: Symbol<Fn_PrintUnformatted> = sym(c_lib(), "cJSON_PrintUnformatted");
        let r_print: Symbol<Fn_PrintUnformatted> = sym(rust_lib(), "cJSON_PrintUnformatted");
        let c_del: Symbol<Fn_Delete> = sym(c_lib(), "cJSON_Delete");
        let r_del: Symbol<Fn_Delete> = sym(rust_lib(), "cJSON_Delete");

        let nums: [c_double; 4] = [1.5, -3.25, 1e10, 1e-5];
        let c_arr = c_create(nums.as_ptr(), nums.len() as c_int);
        let r_arr = r_create(nums.as_ptr(), nums.len() as c_int);
        let cp = c_print(c_arr);
        let rp = r_print(r_arr);
        assert!(cstr_eq(cp, rp), "C={:?} R={:?}", rust_str(cp), rust_str(rp));
        free(cp as *mut c_void);
        free(rp as *mut c_void);
        c_del(c_arr);
        r_del(r_arr);
    }
}

#[test]
fn create_string_array() {
    unsafe {
        let c_create: Symbol<Fn_CreateStringArray> = sym(c_lib(), "cJSON_CreateStringArray");
        let r_create: Symbol<Fn_CreateStringArray> = sym(rust_lib(), "cJSON_CreateStringArray");
        let c_print: Symbol<Fn_PrintUnformatted> = sym(c_lib(), "cJSON_PrintUnformatted");
        let r_print: Symbol<Fn_PrintUnformatted> = sym(rust_lib(), "cJSON_PrintUnformatted");
        let c_del: Symbol<Fn_Delete> = sym(c_lib(), "cJSON_Delete");
        let r_del: Symbol<Fn_Delete> = sym(rust_lib(), "cJSON_Delete");

        let s1 = CString::new("alpha").unwrap();
        let s2 = CString::new("beta").unwrap();
        let s3 = CString::new("gamma").unwrap();
        let arr: [*const c_char; 3] = [s1.as_ptr(), s2.as_ptr(), s3.as_ptr()];

        let c_arr = c_create(arr.as_ptr(), arr.len() as c_int);
        let r_arr = r_create(arr.as_ptr(), arr.len() as c_int);
        let cp = c_print(c_arr);
        let rp = r_print(r_arr);
        assert!(cstr_eq(cp, rp), "C={:?} R={:?}", rust_str(cp), rust_str(rp));
        free(cp as *mut c_void);
        free(rp as *mut c_void);
        c_del(c_arr);
        r_del(r_arr);
    }
}

#[test]
fn add_helper_functions() {
    unsafe {
        let c_create: Symbol<Fn_CreateObject> = sym(c_lib(), "cJSON_CreateObject");
        let r_create: Symbol<Fn_CreateObject> = sym(rust_lib(), "cJSON_CreateObject");
        let c_print: Symbol<Fn_PrintUnformatted> = sym(c_lib(), "cJSON_PrintUnformatted");
        let r_print: Symbol<Fn_PrintUnformatted> = sym(rust_lib(), "cJSON_PrintUnformatted");
        let c_del: Symbol<Fn_Delete> = sym(c_lib(), "cJSON_Delete");
        let r_del: Symbol<Fn_Delete> = sym(rust_lib(), "cJSON_Delete");

        let helpers = [
            "cJSON_AddNullToObject",
            "cJSON_AddTrueToObject",
            "cJSON_AddFalseToObject",
            "cJSON_AddObjectToObject",
            "cJSON_AddArrayToObject",
        ];
        let c_obj = c_create();
        let r_obj = r_create();
        for h in &helpers {
            let c_fn: Symbol<Fn_AddNullToObject> = sym(c_lib(), h);
            let r_fn: Symbol<Fn_AddNullToObject> = sym(rust_lib(), h);
            let key = CString::new(format!("k_{}", h)).unwrap();
            c_fn(c_obj, key.as_ptr());
            r_fn(r_obj, key.as_ptr());
        }
        let c_addbool: Symbol<Fn_AddBoolToObject> = sym(c_lib(), "cJSON_AddBoolToObject");
        let r_addbool: Symbol<Fn_AddBoolToObject> = sym(rust_lib(), "cJSON_AddBoolToObject");
        c_addbool(c_obj, b"truthy\0".as_ptr() as *const c_char, 1);
        r_addbool(r_obj, b"truthy\0".as_ptr() as *const c_char, 1);

        let c_addnum: Symbol<Fn_AddNumberToObject> = sym(c_lib(), "cJSON_AddNumberToObject");
        let r_addnum: Symbol<Fn_AddNumberToObject> = sym(rust_lib(), "cJSON_AddNumberToObject");
        c_addnum(c_obj, b"num\0".as_ptr() as *const c_char, 42.5);
        r_addnum(r_obj, b"num\0".as_ptr() as *const c_char, 42.5);

        let c_addstr: Symbol<Fn_AddStringToObject> = sym(c_lib(), "cJSON_AddStringToObject");
        let r_addstr: Symbol<Fn_AddStringToObject> = sym(rust_lib(), "cJSON_AddStringToObject");
        c_addstr(c_obj, b"str\0".as_ptr() as *const c_char, b"hello\0".as_ptr() as *const c_char);
        r_addstr(r_obj, b"str\0".as_ptr() as *const c_char, b"hello\0".as_ptr() as *const c_char);

        let c_addraw: Symbol<Fn_AddRawToObject> = sym(c_lib(), "cJSON_AddRawToObject");
        let r_addraw: Symbol<Fn_AddRawToObject> = sym(rust_lib(), "cJSON_AddRawToObject");
        c_addraw(c_obj, b"raw\0".as_ptr() as *const c_char, b"\"raw_value\"\0".as_ptr() as *const c_char);
        r_addraw(r_obj, b"raw\0".as_ptr() as *const c_char, b"\"raw_value\"\0".as_ptr() as *const c_char);

        let cp = c_print(c_obj);
        let rp = r_print(r_obj);
        assert!(cstr_eq(cp, rp), "C={:?} R={:?}", rust_str(cp), rust_str(rp));
        free(cp as *mut c_void);
        free(rp as *mut c_void);
        c_del(c_obj);
        r_del(r_obj);
    }
}

#[test]
fn detach_and_delete_from_array() {
    unsafe {
        let c_parse: Symbol<Fn_Parse> = sym(c_lib(), "cJSON_Parse");
        let r_parse: Symbol<Fn_Parse> = sym(rust_lib(), "cJSON_Parse");
        let c_det: Symbol<Fn_DetachItemFromArray> = sym(c_lib(), "cJSON_DetachItemFromArray");
        let r_det: Symbol<Fn_DetachItemFromArray> = sym(rust_lib(), "cJSON_DetachItemFromArray");
        let c_dela: Symbol<Fn_DeleteItemFromArray> = sym(c_lib(), "cJSON_DeleteItemFromArray");
        let r_dela: Symbol<Fn_DeleteItemFromArray> = sym(rust_lib(), "cJSON_DeleteItemFromArray");
        let c_print: Symbol<Fn_PrintUnformatted> = sym(c_lib(), "cJSON_PrintUnformatted");
        let r_print: Symbol<Fn_PrintUnformatted> = sym(rust_lib(), "cJSON_PrintUnformatted");
        let c_del: Symbol<Fn_Delete> = sym(c_lib(), "cJSON_Delete");
        let r_del: Symbol<Fn_Delete> = sym(rust_lib(), "cJSON_Delete");

        let p = b"[1,2,3,4,5]\0".as_ptr() as *const c_char;
        let c_arr = c_parse(p);
        let r_arr = r_parse(p);

        let c_d = c_det(c_arr, 1);
        let r_d = r_det(r_arr, 1);
        assert!(!c_d.is_null() && !r_d.is_null());
        assert_eq!((*c_d).valueint, (*r_d).valueint);

        c_dela(c_arr, 1);
        r_dela(r_arr, 1);

        let cp = c_print(c_arr);
        let rp = r_print(r_arr);
        assert!(cstr_eq(cp, rp), "C={:?} R={:?}", rust_str(cp), rust_str(rp));
        free(cp as *mut c_void);
        free(rp as *mut c_void);

        c_del(c_arr);
        r_del(r_arr);
        c_del(c_d);
        r_del(r_d);
    }
}

#[test]
fn replace_and_insert() {
    unsafe {
        let c_parse: Symbol<Fn_Parse> = sym(c_lib(), "cJSON_Parse");
        let r_parse: Symbol<Fn_Parse> = sym(rust_lib(), "cJSON_Parse");
        let c_create: Symbol<Fn_CreateNumber> = sym(c_lib(), "cJSON_CreateNumber");
        let r_create: Symbol<Fn_CreateNumber> = sym(rust_lib(), "cJSON_CreateNumber");
        let c_rep: Symbol<Fn_ReplaceItemInArray> = sym(c_lib(), "cJSON_ReplaceItemInArray");
        let r_rep: Symbol<Fn_ReplaceItemInArray> = sym(rust_lib(), "cJSON_ReplaceItemInArray");
        let c_ins: Symbol<Fn_InsertItemInArray> = sym(c_lib(), "cJSON_InsertItemInArray");
        let r_ins: Symbol<Fn_InsertItemInArray> = sym(rust_lib(), "cJSON_InsertItemInArray");
        let c_print: Symbol<Fn_PrintUnformatted> = sym(c_lib(), "cJSON_PrintUnformatted");
        let r_print: Symbol<Fn_PrintUnformatted> = sym(rust_lib(), "cJSON_PrintUnformatted");
        let c_del: Symbol<Fn_Delete> = sym(c_lib(), "cJSON_Delete");
        let r_del: Symbol<Fn_Delete> = sym(rust_lib(), "cJSON_Delete");

        let p = b"[1,2,3]\0".as_ptr() as *const c_char;
        let c_arr = c_parse(p);
        let r_arr = r_parse(p);

        c_rep(c_arr, 1, c_create(99.0));
        r_rep(r_arr, 1, r_create(99.0));
        c_ins(c_arr, 0, c_create(0.0));
        r_ins(r_arr, 0, r_create(0.0));

        let cp = c_print(c_arr);
        let rp = r_print(r_arr);
        assert!(cstr_eq(cp, rp), "C={:?} R={:?}", rust_str(cp), rust_str(rp));
        free(cp as *mut c_void);
        free(rp as *mut c_void);

        c_del(c_arr);
        r_del(r_arr);
    }
}

#[test]
fn print_buffered_matches() {
    unsafe {
        let c_parse: Symbol<Fn_Parse> = sym(c_lib(), "cJSON_Parse");
        let r_parse: Symbol<Fn_Parse> = sym(rust_lib(), "cJSON_Parse");
        let c_pb: Symbol<Fn_PrintBuffered> = sym(c_lib(), "cJSON_PrintBuffered");
        let r_pb: Symbol<Fn_PrintBuffered> = sym(rust_lib(), "cJSON_PrintBuffered");
        let c_del: Symbol<Fn_Delete> = sym(c_lib(), "cJSON_Delete");
        let r_del: Symbol<Fn_Delete> = sym(rust_lib(), "cJSON_Delete");

        let p = b"{\"a\":1,\"b\":[2,3]}\0".as_ptr() as *const c_char;
        let c_obj = c_parse(p);
        let r_obj = r_parse(p);
        for &fmt in &[0, 1] {
            for &pre in &[1, 16, 256] {
                let cp = c_pb(c_obj, pre, fmt);
                let rp = r_pb(r_obj, pre, fmt);
                assert!(cstr_eq(cp, rp),
                    "fmt={} pre={} C={:?} R={:?}", fmt, pre, rust_str(cp), rust_str(rp));
                free(cp as *mut c_void);
                free(rp as *mut c_void);
            }
        }
        c_del(c_obj);
        r_del(r_obj);
    }
}

#[test]
fn print_preallocated_matches() {
    unsafe {
        let c_parse: Symbol<Fn_Parse> = sym(c_lib(), "cJSON_Parse");
        let r_parse: Symbol<Fn_Parse> = sym(rust_lib(), "cJSON_Parse");
        let c_pp: Symbol<Fn_PrintPreallocated> = sym(c_lib(), "cJSON_PrintPreallocated");
        let r_pp: Symbol<Fn_PrintPreallocated> = sym(rust_lib(), "cJSON_PrintPreallocated");
        let c_del: Symbol<Fn_Delete> = sym(c_lib(), "cJSON_Delete");
        let r_del: Symbol<Fn_Delete> = sym(rust_lib(), "cJSON_Delete");

        let p = b"{\"a\":1,\"b\":[2,3]}\0".as_ptr() as *const c_char;
        let c_obj = c_parse(p);
        let r_obj = r_parse(p);

        let mut c_buf = vec![0u8; 256];
        let mut r_buf = vec![0u8; 256];
        let c_ok = c_pp(c_obj, c_buf.as_mut_ptr() as *mut c_char, 256, 1);
        let r_ok = r_pp(r_obj, r_buf.as_mut_ptr() as *mut c_char, 256, 1);
        assert_eq!(c_ok, r_ok);
        assert_eq!(c_buf, r_buf);

        // tiny buffer should fail same way
        let mut c_buf = vec![0u8; 4];
        let mut r_buf = vec![0u8; 4];
        let c_ok = c_pp(c_obj, c_buf.as_mut_ptr() as *mut c_char, 4, 0);
        let r_ok = r_pp(r_obj, r_buf.as_mut_ptr() as *mut c_char, 4, 0);
        assert_eq!(c_ok, r_ok);

        c_del(c_obj);
        r_del(r_obj);
    }
}

#[test]
fn malloc_free_match_glibc() {
    unsafe {
        let c_m: Symbol<Fn_malloc> = sym(c_lib(), "cJSON_malloc");
        let r_m: Symbol<Fn_malloc> = sym(rust_lib(), "cJSON_malloc");
        let c_f: Symbol<Fn_free> = sym(c_lib(), "cJSON_free");
        let r_f: Symbol<Fn_free> = sym(rust_lib(), "cJSON_free");
        let cp = c_m(64);
        let rp = r_m(64);
        assert!(!cp.is_null() && !rp.is_null());
        c_f(cp);
        r_f(rp);
    }
}

#[test]
fn set_number_helper_and_valuestring() {
    unsafe {
        let c_create_num: Symbol<Fn_CreateNumber> = sym(c_lib(), "cJSON_CreateNumber");
        let r_create_num: Symbol<Fn_CreateNumber> = sym(rust_lib(), "cJSON_CreateNumber");
        let c_set: Symbol<Fn_SetNumberHelper> = sym(c_lib(), "cJSON_SetNumberHelper");
        let r_set: Symbol<Fn_SetNumberHelper> = sym(rust_lib(), "cJSON_SetNumberHelper");
        let c_del: Symbol<Fn_Delete> = sym(c_lib(), "cJSON_Delete");
        let r_del: Symbol<Fn_Delete> = sym(rust_lib(), "cJSON_Delete");

        let c_obj = c_create_num(0.0);
        let r_obj = r_create_num(0.0);
        for &v in &[1.0, -1.0, 1e30, -1e30, 1.5, 0.0] {
            let cv = c_set(c_obj, v);
            let rv = r_set(r_obj, v);
            assert_eq!(cv.to_bits(), rv.to_bits());
            assert_eq!((*c_obj).valueint, (*r_obj).valueint);
            assert_eq!((*c_obj).valuedouble.to_bits(), (*r_obj).valuedouble.to_bits());
        }
        c_del(c_obj);
        r_del(r_obj);

        // SetValuestring
        let c_create_str: Symbol<Fn_CreateString> = sym(c_lib(), "cJSON_CreateString");
        let r_create_str: Symbol<Fn_CreateString> = sym(rust_lib(), "cJSON_CreateString");
        let c_setvs: Symbol<Fn_SetValuestring> = sym(c_lib(), "cJSON_SetValuestring");
        let r_setvs: Symbol<Fn_SetValuestring> = sym(rust_lib(), "cJSON_SetValuestring");

        let c_obj = c_create_str(b"original_long_string\0".as_ptr() as *const c_char);
        let r_obj = r_create_str(b"original_long_string\0".as_ptr() as *const c_char);
        let new_short = b"hi\0".as_ptr() as *const c_char;
        c_setvs(c_obj, new_short);
        r_setvs(r_obj, new_short);
        assert!(cstr_eq((*c_obj).valuestring, (*r_obj).valuestring));
        c_del(c_obj);
        r_del(r_obj);
    }
}

#[test]
fn parse_with_opts_and_length() {
    unsafe {
        let c_p1: Symbol<Fn_ParseWithLength> = sym(c_lib(), "cJSON_ParseWithLength");
        let r_p1: Symbol<Fn_ParseWithLength> = sym(rust_lib(), "cJSON_ParseWithLength");
        let c_print: Symbol<Fn_PrintUnformatted> = sym(c_lib(), "cJSON_PrintUnformatted");
        let r_print: Symbol<Fn_PrintUnformatted> = sym(rust_lib(), "cJSON_PrintUnformatted");
        let c_del: Symbol<Fn_Delete> = sym(c_lib(), "cJSON_Delete");
        let r_del: Symbol<Fn_Delete> = sym(rust_lib(), "cJSON_Delete");

        let s = b"[1,2,3]xxxxx";
        let c_obj = c_p1(s.as_ptr() as *const c_char, 7);
        let r_obj = r_p1(s.as_ptr() as *const c_char, 7);
        assert!(!c_obj.is_null() && !r_obj.is_null());
        let cp = c_print(c_obj);
        let rp = r_print(r_obj);
        assert!(cstr_eq(cp, rp));
        free(cp as *mut c_void);
        free(rp as *mut c_void);
        c_del(c_obj);
        r_del(r_obj);

        let c_p2: Symbol<Fn_ParseWithOpts> = sym(c_lib(), "cJSON_ParseWithOpts");
        let r_p2: Symbol<Fn_ParseWithOpts> = sym(rust_lib(), "cJSON_ParseWithOpts");
        let mut c_end: *const c_char = ptr::null();
        let mut r_end: *const c_char = ptr::null();
        let json = b"{\"a\":1}\0".as_ptr() as *const c_char;
        let c_obj = c_p2(json, &mut c_end, 1);
        let r_obj = r_p2(json, &mut r_end, 1);
        assert!(!c_obj.is_null() && !r_obj.is_null());
        let c_offset = c_end as usize - json as usize;
        let r_offset = r_end as usize - json as usize;
        assert_eq!(c_offset, r_offset);
        c_del(c_obj);
        r_del(r_obj);
    }
}
