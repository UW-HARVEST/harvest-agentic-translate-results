#![allow(non_camel_case_types)]

use libloading::Library;
use std::ffi::{CStr, CString, c_char, c_double, c_float, c_int, c_void};
use std::path::PathBuf;
use std::ptr;
use std::sync::Mutex;
use std::sync::atomic::{AtomicIsize, Ordering};

#[repr(C)]
#[derive(Debug)]
struct cJSON {
    next: *mut cJSON,
    prev: *mut cJSON,
    child: *mut cJSON,
    r#type: c_int,
    valuestring: *mut c_char,
    valueint: c_int,
    valuedouble: c_double,
    string: *mut c_char,
}

type Alloc = unsafe extern "C" fn(usize) -> *mut c_void;
type Dealloc = unsafe extern "C" fn(*mut c_void);

#[repr(C)]
struct Hooks {
    malloc_fn: Option<Alloc>,
    free_fn: Option<Dealloc>,
}

const CJSON_INVALID: c_int = 0;
const CJSON_FALSE: c_int = 1;
const CJSON_TRUE: c_int = 2;
const CJSON_NULL: c_int = 4;
const CJSON_NUMBER: c_int = 8;
const CJSON_STRING: c_int = 16;
const CJSON_ARRAY: c_int = 32;
const CJSON_OBJECT: c_int = 64;
const CJSON_RAW: c_int = 128;
const CJSON_IS_REFERENCE: c_int = 256;
const CJSON_STRING_IS_CONST: c_int = 512;

static TEST_LOCK: Mutex<()> = Mutex::new(());
static ALLOC_BUDGET: AtomicIsize = AtomicIsize::new(0);

unsafe extern "C" {
    fn malloc(size: usize) -> *mut c_void;
    fn free(pointer: *mut c_void);
}

unsafe extern "C" fn test_alloc(size: usize) -> *mut c_void {
    unsafe { malloc(size) }
}

unsafe extern "C" fn test_free(pointer: *mut c_void) {
    unsafe { free(pointer) }
}

unsafe extern "C" fn fail_alloc(_: usize) -> *mut c_void {
    ptr::null_mut()
}

unsafe extern "C" fn budget_alloc(size: usize) -> *mut c_void {
    if ALLOC_BUDGET.fetch_sub(1, Ordering::SeqCst) <= 0 {
        ptr::null_mut()
    } else {
        unsafe { malloc(size) }
    }
}

struct Libs {
    c: Library,
    r: Library,
}

impl Libs {
    fn new() -> Self {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let c_path = root.join("../c_src/build/libcjson.so");
        let r_path = root.join("target/release/libcJSON_test.so");
        assert!(
            c_path.exists(),
            "missing C shared library: {}",
            c_path.display()
        );
        assert!(
            r_path.exists(),
            "missing Rust shared library: {}",
            r_path.display()
        );
        unsafe {
            Self {
                c: Library::new(c_path).expect("load C shared library"),
                r: Library::new(r_path).expect("load Rust shared library"),
            }
        }
    }

    unsafe fn both<T: Copy>(&self, name: &[u8]) -> (T, T) {
        unsafe {
            (
                *self.c.get::<T>(name).unwrap(),
                *self.r.get::<T>(name).unwrap(),
            )
        }
    }
}

type Delete = unsafe extern "C" fn(*mut cJSON);
type Print = unsafe extern "C" fn(*const cJSON) -> *mut c_char;
type Free = unsafe extern "C" fn(*mut c_void);

unsafe fn rendered(lib: &Library, item: *const cJSON, formatted: bool) -> Option<Vec<u8>> {
    let symbol = if formatted {
        b"cJSON_Print\0".as_slice()
    } else {
        b"cJSON_PrintUnformatted\0".as_slice()
    };
    let print = unsafe { *lib.get::<Print>(symbol).unwrap() };
    let free_fn = unsafe { *lib.get::<Free>(b"cJSON_free\0").unwrap() };
    let output = unsafe { print(item) };
    if output.is_null() {
        return None;
    }
    let bytes = unsafe { CStr::from_ptr(output).to_bytes().to_vec() };
    unsafe { free_fn(output.cast()) };
    Some(bytes)
}

unsafe fn assert_same_tree(libs: &Libs, c_item: *mut cJSON, r_item: *mut cJSON) {
    assert_eq!(c_item.is_null(), r_item.is_null());
    if c_item.is_null() {
        return;
    }
    assert_eq!(unsafe { rendered(&libs.c, c_item, false) }, unsafe {
        rendered(&libs.r, r_item, false)
    });
    assert_eq!(unsafe { rendered(&libs.c, c_item, true) }, unsafe {
        rendered(&libs.r, r_item, true)
    });
}

unsafe fn delete_pair(libs: &Libs, c_item: *mut cJSON, r_item: *mut cJSON) {
    let (c_delete, r_delete): (Delete, Delete) = unsafe { libs.both(b"cJSON_Delete\0") };
    unsafe {
        c_delete(c_item);
        r_delete(r_item);
    }
}

unsafe fn build_number_array(lib: &Library, values: &[i32]) -> *mut cJSON {
    type Create0 = unsafe extern "C" fn() -> *mut cJSON;
    type CreateNumber = unsafe extern "C" fn(c_double) -> *mut cJSON;
    type Add = unsafe extern "C" fn(*mut cJSON, *mut cJSON) -> c_int;
    let create = unsafe { *lib.get::<Create0>(b"cJSON_CreateArray\0").unwrap() };
    let number = unsafe { *lib.get::<CreateNumber>(b"cJSON_CreateNumber\0").unwrap() };
    let add = unsafe { *lib.get::<Add>(b"cJSON_AddItemToArray\0").unwrap() };
    let array = unsafe { create() };
    for value in values {
        assert_eq!(unsafe { add(array, number(*value as f64)) }, 1);
    }
    array
}

unsafe fn build_object(lib: &Library, entries: &[(&CString, i32)]) -> *mut cJSON {
    type Create0 = unsafe extern "C" fn() -> *mut cJSON;
    type CreateNumber = unsafe extern "C" fn(c_double) -> *mut cJSON;
    type Add = unsafe extern "C" fn(*mut cJSON, *const c_char, *mut cJSON) -> c_int;
    let create = unsafe { *lib.get::<Create0>(b"cJSON_CreateObject\0").unwrap() };
    let number = unsafe { *lib.get::<CreateNumber>(b"cJSON_CreateNumber\0").unwrap() };
    let add = unsafe { *lib.get::<Add>(b"cJSON_AddItemToObject\0").unwrap() };
    let object = unsafe { create() };
    for (key, value) in entries {
        assert_eq!(
            unsafe { add(object, key.as_ptr(), number(*value as f64)) },
            1
        );
    }
    object
}

fn cstring(bytes: &[u8]) -> CString {
    CString::new(bytes).unwrap()
}

#[derive(Clone)]
struct Rng(u64);

impl Rng {
    fn new() -> Self {
        Self(0x6a09_e667_f3bc_c909)
    }

    fn next_u64(&mut self) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        self.0
    }

    fn next_i32(&mut self) -> i32 {
        (self.next_u64() >> 32) as i32
    }

    fn next_f64(&mut self) -> f64 {
        let sign = if self.next_u64() & 1 == 0 { 1.0 } else { -1.0 };
        let whole = (self.next_u64() % 1_000_000) as f64;
        let frac = (self.next_u64() % 1_000_000) as f64 / 1_000_003.0;
        sign * (whole + frac)
    }

    fn ascii(&mut self, max_len: usize) -> CString {
        let len = (self.next_u64() as usize) % (max_len + 1);
        let mut bytes = Vec::with_capacity(len);
        for _ in 0..len {
            let choices = b"abCD09 _-/\\\"\n\t";
            bytes.push(choices[(self.next_u64() as usize) % choices.len()]);
        }
        CString::new(bytes).unwrap()
    }
}

const C_SYMBOLS: &[&[u8]] = &[
    b"cJSON_AddArrayToObject\0",
    b"cJSON_AddBoolToObject\0",
    b"cJSON_AddFalseToObject\0",
    b"cJSON_AddItemReferenceToArray\0",
    b"cJSON_AddItemReferenceToObject\0",
    b"cJSON_AddItemToArray\0",
    b"cJSON_AddItemToObject\0",
    b"cJSON_AddItemToObjectCS\0",
    b"cJSON_AddNullToObject\0",
    b"cJSON_AddNumberToObject\0",
    b"cJSON_AddObjectToObject\0",
    b"cJSON_AddRawToObject\0",
    b"cJSON_AddStringToObject\0",
    b"cJSON_AddTrueToObject\0",
    b"cJSON_Compare\0",
    b"cJSON_CreateArray\0",
    b"cJSON_CreateArrayReference\0",
    b"cJSON_CreateBool\0",
    b"cJSON_CreateDoubleArray\0",
    b"cJSON_CreateFalse\0",
    b"cJSON_CreateFloatArray\0",
    b"cJSON_CreateIntArray\0",
    b"cJSON_CreateNull\0",
    b"cJSON_CreateNumber\0",
    b"cJSON_CreateObject\0",
    b"cJSON_CreateObjectReference\0",
    b"cJSON_CreateRaw\0",
    b"cJSON_CreateString\0",
    b"cJSON_CreateStringArray\0",
    b"cJSON_CreateStringReference\0",
    b"cJSON_CreateTrue\0",
    b"cJSON_Delete\0",
    b"cJSON_DeleteItemFromArray\0",
    b"cJSON_DeleteItemFromObject\0",
    b"cJSON_DeleteItemFromObjectCaseSensitive\0",
    b"cJSON_DetachItemFromArray\0",
    b"cJSON_DetachItemFromObject\0",
    b"cJSON_DetachItemFromObjectCaseSensitive\0",
    b"cJSON_DetachItemViaPointer\0",
    b"cJSON_Duplicate\0",
    b"cJSON_GetArrayItem\0",
    b"cJSON_GetArraySize\0",
    b"cJSON_GetErrorPtr\0",
    b"cJSON_GetNumberValue\0",
    b"cJSON_GetObjectItem\0",
    b"cJSON_GetObjectItemCaseSensitive\0",
    b"cJSON_GetStringValue\0",
    b"cJSON_HasObjectItem\0",
    b"cJSON_InitHooks\0",
    b"cJSON_InsertItemInArray\0",
    b"cJSON_IsArray\0",
    b"cJSON_IsBool\0",
    b"cJSON_IsFalse\0",
    b"cJSON_IsInvalid\0",
    b"cJSON_IsNull\0",
    b"cJSON_IsNumber\0",
    b"cJSON_IsObject\0",
    b"cJSON_IsRaw\0",
    b"cJSON_IsString\0",
    b"cJSON_IsTrue\0",
    b"cJSON_Minify\0",
    b"cJSON_Parse\0",
    b"cJSON_ParseWithLength\0",
    b"cJSON_ParseWithLengthOpts\0",
    b"cJSON_ParseWithOpts\0",
    b"cJSON_Print\0",
    b"cJSON_PrintBuffered\0",
    b"cJSON_PrintPreallocated\0",
    b"cJSON_PrintUnformatted\0",
    b"cJSON_ReplaceItemInArray\0",
    b"cJSON_ReplaceItemInObject\0",
    b"cJSON_ReplaceItemInObjectCaseSensitive\0",
    b"cJSON_ReplaceItemViaPointer\0",
    b"cJSON_SetNumberHelper\0",
    b"cJSON_SetValuestring\0",
    b"cJSON_Version\0",
    b"cJSON_free\0",
    b"cJSON_malloc\0",
];

#[test]
fn dynamic_symbol_surface_matches() {
    let _guard = TEST_LOCK.lock().unwrap();
    let libs = Libs::new();
    assert_eq!(C_SYMBOLS.len(), 78);
    for name in C_SYMBOLS {
        unsafe {
            libs.c
                .get::<*mut c_void>(name)
                .unwrap_or_else(|_| panic!("C missing {:?}", CStr::from_bytes_with_nul(name)));
            libs.r
                .get::<*mut c_void>(name)
                .unwrap_or_else(|_| panic!("Rust missing {:?}", CStr::from_bytes_with_nul(name)));
        }
    }
}

#[test]
fn scalar_constructors_getters_setters_and_predicates_match() {
    let _guard = TEST_LOCK.lock().unwrap();
    let libs = Libs::new();
    unsafe {
        type Version = unsafe extern "C" fn() -> *const c_char;
        type InitHooks = unsafe extern "C" fn(*mut Hooks);
        type Create0 = unsafe extern "C" fn() -> *mut cJSON;
        type CreateBool = unsafe extern "C" fn(c_int) -> *mut cJSON;
        type CreateNumber = unsafe extern "C" fn(c_double) -> *mut cJSON;
        type CreateString = unsafe extern "C" fn(*const c_char) -> *mut cJSON;
        type Predicate = unsafe extern "C" fn(*const cJSON) -> c_int;
        type GetString = unsafe extern "C" fn(*const cJSON) -> *mut c_char;
        type GetNumber = unsafe extern "C" fn(*const cJSON) -> c_double;
        type SetNumber = unsafe extern "C" fn(*mut cJSON, c_double) -> c_double;
        type SetString = unsafe extern "C" fn(*mut cJSON, *const c_char) -> *mut c_char;

        let (c_version, r_version): (Version, Version) = libs.both(b"cJSON_Version\0");
        for _ in 0..3 {
            assert_eq!(
                CStr::from_ptr(c_version()).to_bytes(),
                CStr::from_ptr(r_version()).to_bytes()
            );
        }

        let (c_init, r_init): (InitHooks, InitHooks) = libs.both(b"cJSON_InitHooks\0");
        c_init(ptr::null_mut());
        r_init(ptr::null_mut());

        let zero_constructors = [
            (b"cJSON_CreateNull\0".as_slice(), CJSON_NULL),
            (b"cJSON_CreateTrue\0".as_slice(), CJSON_TRUE),
            (b"cJSON_CreateFalse\0".as_slice(), CJSON_FALSE),
            (b"cJSON_CreateArray\0".as_slice(), CJSON_ARRAY),
            (b"cJSON_CreateObject\0".as_slice(), CJSON_OBJECT),
        ];
        for (name, expected_type) in zero_constructors {
            let (c_create, r_create): (Create0, Create0) = libs.both(name);
            let (c_item, r_item) = (c_create(), r_create());
            assert!(!c_item.is_null() && !r_item.is_null());
            assert_eq!((*c_item).r#type, expected_type);
            assert_eq!((*c_item).r#type, (*r_item).r#type);
            assert_same_tree(&libs, c_item, r_item);
            delete_pair(&libs, c_item, r_item);
        }

        let (c_bool, r_bool): (CreateBool, CreateBool) = libs.both(b"cJSON_CreateBool\0");
        for value in [0, 1, -1, 2, i32::MIN, i32::MAX] {
            let (c_item, r_item) = (c_bool(value), r_bool(value));
            assert_eq!((*c_item).r#type, (*r_item).r#type);
            assert_same_tree(&libs, c_item, r_item);
            delete_pair(&libs, c_item, r_item);
        }

        let predicate_names = [
            b"cJSON_IsInvalid\0".as_slice(),
            b"cJSON_IsFalse\0".as_slice(),
            b"cJSON_IsTrue\0".as_slice(),
            b"cJSON_IsBool\0".as_slice(),
            b"cJSON_IsNull\0".as_slice(),
            b"cJSON_IsNumber\0".as_slice(),
            b"cJSON_IsString\0".as_slice(),
            b"cJSON_IsArray\0".as_slice(),
            b"cJSON_IsObject\0".as_slice(),
            b"cJSON_IsRaw\0".as_slice(),
        ];
        let mut manual_items = Vec::new();
        for kind in [
            CJSON_INVALID,
            CJSON_FALSE,
            CJSON_TRUE,
            CJSON_NULL,
            CJSON_NUMBER,
            CJSON_STRING,
            CJSON_ARRAY,
            CJSON_OBJECT,
            CJSON_RAW,
        ] {
            for flags in [0, CJSON_IS_REFERENCE, CJSON_STRING_IS_CONST] {
                manual_items.push(cJSON {
                    next: ptr::null_mut(),
                    prev: ptr::null_mut(),
                    child: ptr::null_mut(),
                    r#type: kind | flags,
                    valuestring: ptr::null_mut(),
                    valueint: 0,
                    valuedouble: 0.0,
                    string: ptr::null_mut(),
                });
            }
        }
        for item in &manual_items {
            for name in predicate_names {
                let (c_predicate, r_predicate): (Predicate, Predicate) = libs.both(name);
                assert_eq!(c_predicate(item), r_predicate(item), "{name:?}");
            }
        }

        let (c_create_number, r_create_number): (CreateNumber, CreateNumber) =
            libs.both(b"cJSON_CreateNumber\0");
        let (c_get_number, r_get_number): (GetNumber, GetNumber) =
            libs.both(b"cJSON_GetNumberValue\0");
        let (c_set_number, r_set_number): (SetNumber, SetNumber) =
            libs.both(b"cJSON_SetNumberHelper\0");
        let mut rng = Rng::new();
        let mut numbers = vec![
            0.0,
            -0.0,
            1.0,
            -1.0,
            i32::MIN as f64,
            i32::MAX as f64,
            i32::MIN as f64 - 1.0,
            i32::MAX as f64 + 1.0,
            f64::MIN_POSITIVE,
            f64::MAX,
            f64::NAN,
            f64::INFINITY,
            f64::NEG_INFINITY,
            0.1,
            1.2345678901234567,
        ];
        numbers.extend((0..100).map(|_| rng.next_f64()));
        for value in numbers {
            let (c_item, r_item) = (c_create_number(value), r_create_number(value));
            assert_eq!((*c_item).valueint, (*r_item).valueint);
            assert_eq!(
                (*c_item).valuedouble.to_bits(),
                (*r_item).valuedouble.to_bits()
            );
            let (cv, rv) = (c_get_number(c_item), r_get_number(r_item));
            assert_eq!(cv.to_bits(), rv.to_bits());
            assert_same_tree(&libs, c_item, r_item);

            let replacement = rng.next_f64();
            assert_eq!(
                c_set_number(c_item, replacement).to_bits(),
                r_set_number(r_item, replacement).to_bits()
            );
            assert_eq!((*c_item).valueint, (*r_item).valueint);
            assert_same_tree(&libs, c_item, r_item);
            delete_pair(&libs, c_item, r_item);
        }

        let (c_create_string, r_create_string): (CreateString, CreateString) =
            libs.both(b"cJSON_CreateString\0");
        let (c_create_raw, r_create_raw): (CreateString, CreateString) =
            libs.both(b"cJSON_CreateRaw\0");
        let (c_get_string, r_get_string): (GetString, GetString) =
            libs.both(b"cJSON_GetStringValue\0");
        let (c_set_string, r_set_string): (SetString, SetString) =
            libs.both(b"cJSON_SetValuestring\0");
        for _ in 0..100 {
            let original = rng.ascii(40);
            let (c_item, r_item) = (
                c_create_string(original.as_ptr()),
                r_create_string(original.as_ptr()),
            );
            assert_eq!(
                CStr::from_ptr(c_get_string(c_item)).to_bytes(),
                CStr::from_ptr(r_get_string(r_item)).to_bytes()
            );
            assert_same_tree(&libs, c_item, r_item);
            for replacement in [rng.ascii(4), rng.ascii(40), rng.ascii(80)] {
                let c_result = c_set_string(c_item, replacement.as_ptr());
                let r_result = r_set_string(r_item, replacement.as_ptr());
                assert_eq!(c_result.is_null(), r_result.is_null());
                if !c_result.is_null() {
                    assert_eq!(
                        CStr::from_ptr(c_result).to_bytes(),
                        CStr::from_ptr(r_result).to_bytes()
                    );
                }
                assert_same_tree(&libs, c_item, r_item);
            }
            delete_pair(&libs, c_item, r_item);

            let raw = rng.ascii(50);
            let (c_raw, r_raw) = (c_create_raw(raw.as_ptr()), r_create_raw(raw.as_ptr()));
            assert_same_tree(&libs, c_raw, r_raw);
            delete_pair(&libs, c_raw, r_raw);
        }
    }
}

#[test]
fn array_constructors_and_reference_constructors_match() {
    let _guard = TEST_LOCK.lock().unwrap();
    let libs = Libs::new();
    unsafe {
        type CreateIntArray = unsafe extern "C" fn(*const c_int, c_int) -> *mut cJSON;
        type CreateFloatArray = unsafe extern "C" fn(*const c_float, c_int) -> *mut cJSON;
        type CreateDoubleArray = unsafe extern "C" fn(*const c_double, c_int) -> *mut cJSON;
        type CreateStringArray = unsafe extern "C" fn(*const *const c_char, c_int) -> *mut cJSON;
        type CreateString = unsafe extern "C" fn(*const c_char) -> *mut cJSON;
        type CreateRef = unsafe extern "C" fn(*const cJSON) -> *mut cJSON;

        let (c_ints, r_ints): (CreateIntArray, CreateIntArray) =
            libs.both(b"cJSON_CreateIntArray\0");
        let (c_floats, r_floats): (CreateFloatArray, CreateFloatArray) =
            libs.both(b"cJSON_CreateFloatArray\0");
        let (c_doubles, r_doubles): (CreateDoubleArray, CreateDoubleArray) =
            libs.both(b"cJSON_CreateDoubleArray\0");
        let (c_strings, r_strings): (CreateStringArray, CreateStringArray) =
            libs.both(b"cJSON_CreateStringArray\0");
        let mut rng = Rng::new();
        for count in [0usize, 1, 2, 9, 32] {
            let ints: Vec<i32> = (0..count).map(|_| rng.next_i32()).collect();
            let floats: Vec<f32> = (0..count)
                .map(|_| match rng.next_u64() % 17 {
                    0 => f32::NAN,
                    1 => f32::INFINITY,
                    2 => f32::NEG_INFINITY,
                    _ => rng.next_f64() as f32,
                })
                .collect();
            let doubles: Vec<f64> = (0..count)
                .map(|_| match rng.next_u64() % 17 {
                    0 => f64::NAN,
                    1 => f64::INFINITY,
                    2 => f64::NEG_INFINITY,
                    _ => rng.next_f64(),
                })
                .collect();
            let strings: Vec<CString> = (0..count).map(|_| rng.ascii(30)).collect();
            let string_ptrs: Vec<*const c_char> = strings.iter().map(|s| s.as_ptr()).collect();

            for (c_item, r_item) in [
                (
                    c_ints(ints.as_ptr(), count as c_int),
                    r_ints(ints.as_ptr(), count as c_int),
                ),
                (
                    c_floats(floats.as_ptr(), count as c_int),
                    r_floats(floats.as_ptr(), count as c_int),
                ),
                (
                    c_doubles(doubles.as_ptr(), count as c_int),
                    r_doubles(doubles.as_ptr(), count as c_int),
                ),
                (
                    c_strings(string_ptrs.as_ptr(), count as c_int),
                    r_strings(string_ptrs.as_ptr(), count as c_int),
                ),
            ] {
                assert_same_tree(&libs, c_item, r_item);
                delete_pair(&libs, c_item, r_item);
            }
        }

        let text = cstring(b"borrowed");
        let (c_string_ref, r_string_ref): (CreateString, CreateString) =
            libs.both(b"cJSON_CreateStringReference\0");
        let (c_ref, r_ref) = (c_string_ref(text.as_ptr()), r_string_ref(text.as_ptr()));
        assert_eq!((*c_ref).r#type, (*r_ref).r#type);
        assert_same_tree(&libs, c_ref, r_ref);
        delete_pair(&libs, c_ref, r_ref);

        type Create0 = unsafe extern "C" fn() -> *mut cJSON;
        let (c_create_array, r_create_array): (Create0, Create0) =
            libs.both(b"cJSON_CreateArray\0");
        let (c_array_ref, r_array_ref): (CreateRef, CreateRef) =
            libs.both(b"cJSON_CreateArrayReference\0");
        let (c_source, r_source) = (c_create_array(), r_create_array());
        let (c_reference, r_reference) = (c_array_ref(c_source), r_array_ref(r_source));
        assert_eq!((*c_reference).r#type, (*r_reference).r#type);
        assert_same_tree(&libs, c_reference, r_reference);
        delete_pair(&libs, c_reference, r_reference);
        delete_pair(&libs, c_source, r_source);

        let (c_create_object, r_create_object): (Create0, Create0) =
            libs.both(b"cJSON_CreateObject\0");
        let (c_object_ref, r_object_ref): (CreateRef, CreateRef) =
            libs.both(b"cJSON_CreateObjectReference\0");
        let (c_source, r_source) = (c_create_object(), r_create_object());
        let (c_reference, r_reference) = (c_object_ref(c_source), r_object_ref(r_source));
        assert_same_tree(&libs, c_reference, r_reference);
        delete_pair(&libs, c_reference, r_reference);
        delete_pair(&libs, c_source, r_source);
    }
}

#[test]
fn parsing_and_printing_valid_configurations_match() {
    let _guard = TEST_LOCK.lock().unwrap();
    let libs = Libs::new();
    unsafe {
        type Parse = unsafe extern "C" fn(*const c_char) -> *mut cJSON;
        type ParseLength = unsafe extern "C" fn(*const c_char, usize) -> *mut cJSON;
        type ParseOpts =
            unsafe extern "C" fn(*const c_char, *mut *const c_char, c_int) -> *mut cJSON;
        type ParseLengthOpts =
            unsafe extern "C" fn(*const c_char, usize, *mut *const c_char, c_int) -> *mut cJSON;
        type PrintBuffered = unsafe extern "C" fn(*const cJSON, c_int, c_int) -> *mut c_char;
        type PrintPreallocated =
            unsafe extern "C" fn(*mut cJSON, *mut c_char, c_int, c_int) -> c_int;

        let (c_parse, r_parse): (Parse, Parse) = libs.both(b"cJSON_Parse\0");
        let (c_parse_len, r_parse_len): (ParseLength, ParseLength) =
            libs.both(b"cJSON_ParseWithLength\0");
        let (c_parse_opts, r_parse_opts): (ParseOpts, ParseOpts) =
            libs.both(b"cJSON_ParseWithOpts\0");
        let (c_parse_len_opts, r_parse_len_opts): (ParseLengthOpts, ParseLengthOpts) =
            libs.both(b"cJSON_ParseWithLengthOpts\0");
        let (c_buffered, r_buffered): (PrintBuffered, PrintBuffered) =
            libs.both(b"cJSON_PrintBuffered\0");
        let (c_preallocated, r_preallocated): (PrintPreallocated, PrintPreallocated) =
            libs.both(b"cJSON_PrintPreallocated\0");
        let (c_free, r_free): (Free, Free) = libs.both(b"cJSON_free\0");

        let mut corpus: Vec<Vec<u8>> = vec![
            b"null".to_vec(),
            b"false".to_vec(),
            b"true".to_vec(),
            b"0".to_vec(),
            b"-0".to_vec(),
            b"2147483647".to_vec(),
            b"-2147483648".to_vec(),
            b"2147483648".to_vec(),
            b"-2147483649".to_vec(),
            b"1.25".to_vec(),
            b"-2.5e+30".to_vec(),
            b"6.02214076E23".to_vec(),
            br#""""#.to_vec(),
            br#""plain""#.to_vec(),
            br#""quote:\" slash:\/ backslash:\\""#.to_vec(),
            br#""\b\f\n\r\t""#.to_vec(),
            br#""\u0000\u001f\u007f\u07ff\u20ac""#.to_vec(),
            br#""\ud83d\ude03""#.to_vec(),
            b"[]".to_vec(),
            b"[null]".to_vec(),
            br#"[true,false,0,-1,1.5,"x",[],{}]"#.to_vec(),
            b"{}".to_vec(),
            br#"{"a":1}"#.to_vec(),
            br#"{"A":1,"a":2,"escaped\nkey":"v","nested":{"x":[1,2,3]}}"#.to_vec(),
            b" \t\r\n [ 1 , 2 , 3 ] \n".to_vec(),
            b"\xef\xbb\xbf{\"bom\":true}".to_vec(),
            b"\"\x80\xfe\"".to_vec(),
        ];
        let mut rng = Rng::new();
        for _ in 0..100 {
            let a = rng.next_i32();
            let b = rng.next_f64();
            let s = rng.ascii(24);
            let escaped = serde_free_json_string(s.to_bytes());
            corpus.push(
                format!("{{\"n\":{a},\"f\":{b:.17},\"s\":{escaped},\"a\":[{a},null,true,false]}}")
                    .into_bytes(),
            );
        }

        for bytes in corpus {
            let value = cstring(&bytes);

            let (c_item, r_item) = (c_parse(value.as_ptr()), r_parse(value.as_ptr()));
            assert_same_tree(&libs, c_item, r_item);

            for format in [0, 1, -1, 2] {
                for prebuffer in [0, 1, 8, 256, 1024] {
                    let (co, ro) = (
                        c_buffered(c_item, prebuffer, format),
                        r_buffered(r_item, prebuffer, format),
                    );
                    assert_eq!(co.is_null(), ro.is_null());
                    if !co.is_null() {
                        assert_eq!(CStr::from_ptr(co).to_bytes(), CStr::from_ptr(ro).to_bytes());
                        c_free(co.cast());
                        r_free(ro.cast());
                    }
                }
            }

            for format in [0, 1, -1, 2] {
                let expected = rendered(&libs.c, c_item, format != 0).unwrap();
                for extra in [0usize, 1, 5, 32] {
                    let length = expected.len() + 1 + extra;
                    let mut cb = vec![0x55u8; length];
                    let mut rb = vec![0x55u8; length];
                    let (cr, rr) = (
                        c_preallocated(c_item, cb.as_mut_ptr().cast(), length as c_int, format),
                        r_preallocated(r_item, rb.as_mut_ptr().cast(), length as c_int, format),
                    );
                    assert_eq!(cr, rr);
                    assert_eq!(cb, rb);
                }
                let mut cb = vec![0x55u8; expected.len()];
                let mut rb = vec![0x55u8; expected.len()];
                assert_eq!(
                    c_preallocated(c_item, cb.as_mut_ptr().cast(), cb.len() as c_int, format),
                    r_preallocated(r_item, rb.as_mut_ptr().cast(), rb.len() as c_int, format)
                );
                assert_eq!(cb, rb);
            }

            let (c_len_item, r_len_item) = (
                c_parse_len(value.as_ptr(), bytes.len()),
                r_parse_len(value.as_ptr(), bytes.len()),
            );
            assert_same_tree(&libs, c_len_item, r_len_item);
            delete_pair(&libs, c_len_item, r_len_item);

            let (c_len_nul, r_len_nul) = (
                c_parse_len(value.as_ptr(), bytes.len() + 1),
                r_parse_len(value.as_ptr(), bytes.len() + 1),
            );
            assert_same_tree(&libs, c_len_nul, r_len_nul);
            delete_pair(&libs, c_len_nul, r_len_nul);

            for require_nul in [0, 1, -1, 2] {
                let mut c_end = ptr::null();
                let mut r_end = ptr::null();
                let (co, ro) = (
                    c_parse_opts(value.as_ptr(), &mut c_end, require_nul),
                    r_parse_opts(value.as_ptr(), &mut r_end, require_nul),
                );
                assert_same_tree(&libs, co, ro);
                assert_eq!(
                    c_end.offset_from(value.as_ptr()),
                    r_end.offset_from(value.as_ptr())
                );
                delete_pair(&libs, co, ro);

                for length in [bytes.len(), bytes.len() + 1] {
                    for with_end in [false, true] {
                        let mut c_end = ptr::null();
                        let mut r_end = ptr::null();
                        let c_end_ptr = if with_end {
                            &mut c_end
                        } else {
                            ptr::null_mut()
                        };
                        let r_end_ptr = if with_end {
                            &mut r_end
                        } else {
                            ptr::null_mut()
                        };
                        let (co, ro) = (
                            c_parse_len_opts(value.as_ptr(), length, c_end_ptr, require_nul),
                            r_parse_len_opts(value.as_ptr(), length, r_end_ptr, require_nul),
                        );
                        assert_same_tree(&libs, co, ro);
                        if with_end {
                            assert_eq!(
                                c_end.offset_from(value.as_ptr()),
                                r_end.offset_from(value.as_ptr())
                            );
                        }
                        delete_pair(&libs, co, ro);
                    }
                }
            }
            delete_pair(&libs, c_item, r_item);
        }
    }
}

fn serde_free_json_string(bytes: &[u8]) -> String {
    let mut output = String::from("\"");
    for &byte in bytes {
        match byte {
            b'"' => output.push_str("\\\""),
            b'\\' => output.push_str("\\\\"),
            b'\n' => output.push_str("\\n"),
            b'\r' => output.push_str("\\r"),
            b'\t' => output.push_str("\\t"),
            0x08 => output.push_str("\\b"),
            0x0c => output.push_str("\\f"),
            0x00..=0x1f => output.push_str(&format!("\\u{byte:04x}")),
            _ => output.push(byte as char),
        }
    }
    output.push('"');
    output
}

#[test]
fn parsing_error_paths_and_offsets_match() {
    let _guard = TEST_LOCK.lock().unwrap();
    let libs = Libs::new();
    unsafe {
        type ParseLengthOpts =
            unsafe extern "C" fn(*const c_char, usize, *mut *const c_char, c_int) -> *mut cJSON;
        type GetError = unsafe extern "C" fn() -> *const c_char;

        let (c_parse, r_parse): (ParseLengthOpts, ParseLengthOpts) =
            libs.both(b"cJSON_ParseWithLengthOpts\0");
        let (c_error, r_error): (GetError, GetError) = libs.both(b"cJSON_GetErrorPtr\0");

        let invalid: Vec<Vec<u8>> = vec![
            b"".to_vec(),
            b" ".to_vec(),
            b"x".to_vec(),
            b"+1".to_vec(),
            b"\"".to_vec(),
            b"\"abc".to_vec(),
            b"\"abc\\".to_vec(),
            br#""\q""#.to_vec(),
            br#""\u1""#.to_vec(),
            br#""\u12x4""#.to_vec(),
            br#""\udc00""#.to_vec(),
            br#""\ud800""#.to_vec(),
            br#""\ud800x""#.to_vec(),
            br#""\ud800\u0041""#.to_vec(),
            b"[".to_vec(),
            b"[ ".to_vec(),
            b"[,]".to_vec(),
            b"[1,]".to_vec(),
            b"[1".to_vec(),
            b"{".to_vec(),
            b"{ ".to_vec(),
            b"{,}".to_vec(),
            b"{x:1}".to_vec(),
            br#"{"a" 1}"#.to_vec(),
            br#"{"a":}"#.to_vec(),
            br#"{"a":1"#.to_vec(),
            br#"{"a":1,}"#.to_vec(),
            b"tru".to_vec(),
            b"nul".to_vec(),
        ];
        for bytes in invalid {
            let value = cstring(&bytes);
            for require_nul in [0, 1, -1, 2] {
                for length in [0, bytes.len(), bytes.len() + 1] {
                    let mut c_end = ptr::null();
                    let mut r_end = ptr::null();
                    let (co, ro) = (
                        c_parse(value.as_ptr(), length, &mut c_end, require_nul),
                        r_parse(value.as_ptr(), length, &mut r_end, require_nul),
                    );
                    assert_eq!(
                        co.is_null(),
                        ro.is_null(),
                        "{bytes:?}/{length}/{require_nul}"
                    );
                    if !co.is_null() {
                        assert_same_tree(&libs, co, ro);
                        delete_pair(&libs, co, ro);
                    }
                    assert_eq!(
                        c_end.offset_from(value.as_ptr()),
                        r_end.offset_from(value.as_ptr()),
                        "{bytes:?}/{length}/{require_nul}"
                    );
                    let ce = c_error();
                    let re = r_error();
                    assert_eq!(ce.is_null(), re.is_null());
                    if !ce.is_null() {
                        assert_eq!(
                            ce.offset_from(value.as_ptr()),
                            re.offset_from(value.as_ptr()),
                            "global error {bytes:?}/{length}/{require_nul}"
                        );
                    }
                }
            }
        }

        let trailing = cstring(b"true trailing");
        for require_nul in [1, -1, 2] {
            let mut c_end = ptr::null();
            let mut r_end = ptr::null();
            assert!(
                c_parse(
                    trailing.as_ptr(),
                    trailing.as_bytes_with_nul().len(),
                    &mut c_end,
                    require_nul
                )
                .is_null()
            );
            assert!(
                r_parse(
                    trailing.as_ptr(),
                    trailing.as_bytes_with_nul().len(),
                    &mut r_end,
                    require_nul
                )
                .is_null()
            );
            assert_eq!(
                c_end.offset_from(trailing.as_ptr()),
                r_end.offset_from(trailing.as_ptr())
            );
        }

        let nested = format!("{}0{}", "[".repeat(1001), "]".repeat(1001));
        let value = CString::new(nested).unwrap();
        let mut c_end = ptr::null();
        let mut r_end = ptr::null();
        let (co, ro) = (
            c_parse(
                value.as_ptr(),
                value.as_bytes_with_nul().len(),
                &mut c_end,
                1,
            ),
            r_parse(
                value.as_ptr(),
                value.as_bytes_with_nul().len(),
                &mut r_end,
                1,
            ),
        );
        assert!(co.is_null() && ro.is_null());
        assert_eq!(
            c_end.offset_from(value.as_ptr()),
            r_end.offset_from(value.as_ptr())
        );

        let mut c_end = ptr::null();
        let mut r_end = ptr::null();
        assert!(c_parse(ptr::null(), usize::MAX, &mut c_end, 0).is_null());
        assert!(r_parse(ptr::null(), usize::MAX, &mut r_end, 0).is_null());
        assert_eq!(c_end, r_end);
    }
}

#[test]
fn array_and_object_mutation_entry_points_match() {
    let _guard = TEST_LOCK.lock().unwrap();
    let libs = Libs::new();
    unsafe {
        type Create0 = unsafe extern "C" fn() -> *mut cJSON;
        type CreateNumber = unsafe extern "C" fn(c_double) -> *mut cJSON;
        type CreateString = unsafe extern "C" fn(*const c_char) -> *mut cJSON;
        type AddArray = unsafe extern "C" fn(*mut cJSON, *mut cJSON) -> c_int;
        type AddObject = unsafe extern "C" fn(*mut cJSON, *const c_char, *mut cJSON) -> c_int;
        type GetSize = unsafe extern "C" fn(*const cJSON) -> c_int;
        type GetArray = unsafe extern "C" fn(*const cJSON, c_int) -> *mut cJSON;
        type GetObject = unsafe extern "C" fn(*const cJSON, *const c_char) -> *mut cJSON;
        type HasObject = unsafe extern "C" fn(*const cJSON, *const c_char) -> c_int;
        type DetachPointer = unsafe extern "C" fn(*mut cJSON, *mut cJSON) -> *mut cJSON;
        type DetachArray = unsafe extern "C" fn(*mut cJSON, c_int) -> *mut cJSON;
        type DeleteArray = unsafe extern "C" fn(*mut cJSON, c_int);
        type DetachObject = unsafe extern "C" fn(*mut cJSON, *const c_char) -> *mut cJSON;
        type DeleteObject = unsafe extern "C" fn(*mut cJSON, *const c_char);
        type Insert = unsafe extern "C" fn(*mut cJSON, c_int, *mut cJSON) -> c_int;
        type ReplacePointer = unsafe extern "C" fn(*mut cJSON, *mut cJSON, *mut cJSON) -> c_int;
        type ReplaceArray = unsafe extern "C" fn(*mut cJSON, c_int, *mut cJSON) -> c_int;
        type ReplaceObject = unsafe extern "C" fn(*mut cJSON, *const c_char, *mut cJSON) -> c_int;

        let (c_create_array, r_create_array): (Create0, Create0) =
            libs.both(b"cJSON_CreateArray\0");
        let (c_create_object, r_create_object): (Create0, Create0) =
            libs.both(b"cJSON_CreateObject\0");
        let (c_number, r_number): (CreateNumber, CreateNumber) = libs.both(b"cJSON_CreateNumber\0");
        let (c_string, r_string): (CreateString, CreateString) = libs.both(b"cJSON_CreateString\0");
        let (c_add_array, r_add_array): (AddArray, AddArray) = libs.both(b"cJSON_AddItemToArray\0");
        let (c_add_object, r_add_object): (AddObject, AddObject) =
            libs.both(b"cJSON_AddItemToObject\0");
        let (c_add_object_cs, r_add_object_cs): (AddObject, AddObject) =
            libs.both(b"cJSON_AddItemToObjectCS\0");
        let (c_size, r_size): (GetSize, GetSize) = libs.both(b"cJSON_GetArraySize\0");
        let (c_get_array, r_get_array): (GetArray, GetArray) = libs.both(b"cJSON_GetArrayItem\0");
        let (c_get_object, r_get_object): (GetObject, GetObject) =
            libs.both(b"cJSON_GetObjectItem\0");
        let (c_get_object_cs, r_get_object_cs): (GetObject, GetObject) =
            libs.both(b"cJSON_GetObjectItemCaseSensitive\0");
        let (c_has, r_has): (HasObject, HasObject) = libs.both(b"cJSON_HasObjectItem\0");

        let (c_array, r_array) = (c_create_array(), r_create_array());
        let mut rng = Rng::new();
        for index in 0..100 {
            let value = rng.next_i32();
            assert_eq!(
                c_add_array(c_array, c_number(value as f64)),
                r_add_array(r_array, r_number(value as f64))
            );
            assert_eq!(c_size(c_array), r_size(r_array));
            for probe in [0, index / 2, index, index + 1] {
                let (ci, ri) = (
                    c_get_array(c_array, probe as c_int),
                    r_get_array(r_array, probe as c_int),
                );
                assert_eq!(ci.is_null(), ri.is_null());
                if !ci.is_null() {
                    assert_same_tree(&libs, ci, ri);
                }
            }
        }
        assert_same_tree(&libs, c_array, r_array);
        delete_pair(&libs, c_array, r_array);

        let keys = [
            cstring(b"Alpha"),
            cstring(b"beta"),
            cstring(b"ALPHA"),
            cstring(b""),
        ];
        let (c_object, r_object) = (c_create_object(), r_create_object());
        for (index, key) in keys.iter().enumerate() {
            assert_eq!(
                c_add_object(c_object, key.as_ptr(), c_number(index as f64)),
                r_add_object(r_object, key.as_ptr(), r_number(index as f64))
            );
        }
        for probe in [
            cstring(b"alpha"),
            cstring(b"ALPHA"),
            cstring(b"Beta"),
            cstring(b""),
            cstring(b"missing"),
        ] {
            let (ci, ri) = (
                c_get_object(c_object, probe.as_ptr()),
                r_get_object(r_object, probe.as_ptr()),
            );
            assert_eq!(ci.is_null(), ri.is_null());
            assert_eq!(
                c_has(c_object, probe.as_ptr()),
                r_has(r_object, probe.as_ptr())
            );
            if !ci.is_null() {
                assert_same_tree(&libs, ci, ri);
            }
            let (ci, ri) = (
                c_get_object_cs(c_object, probe.as_ptr()),
                r_get_object_cs(r_object, probe.as_ptr()),
            );
            assert_eq!(ci.is_null(), ri.is_null());
            if !ci.is_null() {
                assert_same_tree(&libs, ci, ri);
            }
        }
        assert_same_tree(&libs, c_object, r_object);
        delete_pair(&libs, c_object, r_object);

        let const_key = cstring(b"ConstKey");
        let text = cstring(b"value");
        let (c_object, r_object) = (c_create_object(), r_create_object());
        assert_eq!(
            c_add_object_cs(c_object, const_key.as_ptr(), c_string(text.as_ptr())),
            r_add_object_cs(r_object, const_key.as_ptr(), r_string(text.as_ptr()))
        );
        assert_eq!(
            (*c_get_object_cs(c_object, const_key.as_ptr())).r#type,
            (*r_get_object_cs(r_object, const_key.as_ptr())).r#type
        );
        assert_same_tree(&libs, c_object, r_object);
        delete_pair(&libs, c_object, r_object);

        let (c_detach_pointer, r_detach_pointer): (DetachPointer, DetachPointer) =
            libs.both(b"cJSON_DetachItemViaPointer\0");
        let (c_detach_array, r_detach_array): (DetachArray, DetachArray) =
            libs.both(b"cJSON_DetachItemFromArray\0");
        let (c_delete_array, r_delete_array): (DeleteArray, DeleteArray) =
            libs.both(b"cJSON_DeleteItemFromArray\0");
        for position in [0, 2, 4] {
            let c_array = build_number_array(&libs.c, &[0, 1, 2, 3, 4]);
            let r_array = build_number_array(&libs.r, &[0, 1, 2, 3, 4]);
            let (ci, ri) = (
                c_get_array(c_array, position),
                r_get_array(r_array, position),
            );
            let (cd, rd) = (c_detach_pointer(c_array, ci), r_detach_pointer(r_array, ri));
            assert_same_tree(&libs, cd, rd);
            assert_same_tree(&libs, c_array, r_array);
            delete_pair(&libs, cd, rd);
            delete_pair(&libs, c_array, r_array);

            let c_array = build_number_array(&libs.c, &[0, 1, 2, 3, 4]);
            let r_array = build_number_array(&libs.r, &[0, 1, 2, 3, 4]);
            let (cd, rd) = (
                c_detach_array(c_array, position),
                r_detach_array(r_array, position),
            );
            assert_same_tree(&libs, cd, rd);
            assert_same_tree(&libs, c_array, r_array);
            delete_pair(&libs, cd, rd);
            delete_pair(&libs, c_array, r_array);

            let c_array = build_number_array(&libs.c, &[0, 1, 2, 3, 4]);
            let r_array = build_number_array(&libs.r, &[0, 1, 2, 3, 4]);
            c_delete_array(c_array, position);
            r_delete_array(r_array, position);
            assert_same_tree(&libs, c_array, r_array);
            delete_pair(&libs, c_array, r_array);
        }

        let (c_detach_object, r_detach_object): (DetachObject, DetachObject) =
            libs.both(b"cJSON_DetachItemFromObject\0");
        let (c_detach_object_cs, r_detach_object_cs): (DetachObject, DetachObject) =
            libs.both(b"cJSON_DetachItemFromObjectCaseSensitive\0");
        let (c_delete_object, r_delete_object): (DeleteObject, DeleteObject) =
            libs.both(b"cJSON_DeleteItemFromObject\0");
        let (c_delete_object_cs, r_delete_object_cs): (DeleteObject, DeleteObject) =
            libs.both(b"cJSON_DeleteItemFromObjectCaseSensitive\0");
        let ka = cstring(b"A");
        let kb = cstring(b"b");
        let kc = cstring(b"C");
        let entries = [(&ka, 1), (&kb, 2), (&kc, 3)];
        for case_sensitive in [false, true] {
            let probe = if case_sensitive {
                cstring(b"b")
            } else {
                cstring(b"B")
            };
            let c_object = build_object(&libs.c, &entries);
            let r_object = build_object(&libs.r, &entries);
            let (cd, rd) = if case_sensitive {
                (
                    c_detach_object_cs(c_object, probe.as_ptr()),
                    r_detach_object_cs(r_object, probe.as_ptr()),
                )
            } else {
                (
                    c_detach_object(c_object, probe.as_ptr()),
                    r_detach_object(r_object, probe.as_ptr()),
                )
            };
            assert_same_tree(&libs, cd, rd);
            assert_same_tree(&libs, c_object, r_object);
            delete_pair(&libs, cd, rd);
            delete_pair(&libs, c_object, r_object);

            let c_object = build_object(&libs.c, &entries);
            let r_object = build_object(&libs.r, &entries);
            if case_sensitive {
                c_delete_object_cs(c_object, probe.as_ptr());
                r_delete_object_cs(r_object, probe.as_ptr());
            } else {
                c_delete_object(c_object, probe.as_ptr());
                r_delete_object(r_object, probe.as_ptr());
            }
            assert_same_tree(&libs, c_object, r_object);
            delete_pair(&libs, c_object, r_object);
        }

        let (c_insert, r_insert): (Insert, Insert) = libs.both(b"cJSON_InsertItemInArray\0");
        for position in [0, 2, 99] {
            let c_array = build_number_array(&libs.c, &[0, 1, 2, 3]);
            let r_array = build_number_array(&libs.r, &[0, 1, 2, 3]);
            assert_eq!(
                c_insert(c_array, position, c_number(99.0)),
                r_insert(r_array, position, r_number(99.0))
            );
            assert_same_tree(&libs, c_array, r_array);
            delete_pair(&libs, c_array, r_array);
        }

        let (c_replace_pointer, r_replace_pointer): (ReplacePointer, ReplacePointer) =
            libs.both(b"cJSON_ReplaceItemViaPointer\0");
        let (c_replace_array, r_replace_array): (ReplaceArray, ReplaceArray) =
            libs.both(b"cJSON_ReplaceItemInArray\0");
        for position in [0, 2, 4] {
            let c_array = build_number_array(&libs.c, &[0, 1, 2, 3, 4]);
            let r_array = build_number_array(&libs.r, &[0, 1, 2, 3, 4]);
            let (ci, ri) = (
                c_get_array(c_array, position),
                r_get_array(r_array, position),
            );
            assert_eq!(
                c_replace_pointer(c_array, ci, c_number(77.0)),
                r_replace_pointer(r_array, ri, r_number(77.0))
            );
            assert_same_tree(&libs, c_array, r_array);
            delete_pair(&libs, c_array, r_array);

            let c_array = build_number_array(&libs.c, &[0, 1, 2, 3, 4]);
            let r_array = build_number_array(&libs.r, &[0, 1, 2, 3, 4]);
            assert_eq!(
                c_replace_array(c_array, position, c_number(88.0)),
                r_replace_array(r_array, position, r_number(88.0))
            );
            assert_same_tree(&libs, c_array, r_array);
            delete_pair(&libs, c_array, r_array);
        }

        let (c_replace_object, r_replace_object): (ReplaceObject, ReplaceObject) =
            libs.both(b"cJSON_ReplaceItemInObject\0");
        let (c_replace_object_cs, r_replace_object_cs): (ReplaceObject, ReplaceObject) =
            libs.both(b"cJSON_ReplaceItemInObjectCaseSensitive\0");
        for case_sensitive in [false, true] {
            let c_object = build_object(&libs.c, &entries);
            let r_object = build_object(&libs.r, &entries);
            let probe = if case_sensitive {
                cstring(b"b")
            } else {
                cstring(b"B")
            };
            let result = if case_sensitive {
                (
                    c_replace_object_cs(c_object, probe.as_ptr(), c_number(55.0)),
                    r_replace_object_cs(r_object, probe.as_ptr(), r_number(55.0)),
                )
            } else {
                (
                    c_replace_object(c_object, probe.as_ptr(), c_number(55.0)),
                    r_replace_object(r_object, probe.as_ptr(), r_number(55.0)),
                )
            };
            assert_eq!(result.0, result.1);
            assert_same_tree(&libs, c_object, r_object);
            delete_pair(&libs, c_object, r_object);
        }
    }
}

#[test]
fn convenience_reference_duplicate_and_compare_paths_match() {
    let _guard = TEST_LOCK.lock().unwrap();
    let libs = Libs::new();
    unsafe {
        type Create0 = unsafe extern "C" fn() -> *mut cJSON;
        type CreateNumber = unsafe extern "C" fn(c_double) -> *mut cJSON;
        type AddReference = unsafe extern "C" fn(*mut cJSON, *mut cJSON) -> c_int;
        type AddReferenceObject =
            unsafe extern "C" fn(*mut cJSON, *const c_char, *mut cJSON) -> c_int;
        type AddSimple = unsafe extern "C" fn(*mut cJSON, *const c_char) -> *mut cJSON;
        type AddBool = unsafe extern "C" fn(*mut cJSON, *const c_char, c_int) -> *mut cJSON;
        type AddNumber = unsafe extern "C" fn(*mut cJSON, *const c_char, c_double) -> *mut cJSON;
        type AddString =
            unsafe extern "C" fn(*mut cJSON, *const c_char, *const c_char) -> *mut cJSON;
        type Duplicate = unsafe extern "C" fn(*const cJSON, c_int) -> *mut cJSON;
        type Compare = unsafe extern "C" fn(*const cJSON, *const cJSON, c_int) -> c_int;
        type Parse = unsafe extern "C" fn(*const c_char) -> *mut cJSON;

        let (c_array, r_array): (Create0, Create0) = libs.both(b"cJSON_CreateArray\0");
        let (c_object, r_object): (Create0, Create0) = libs.both(b"cJSON_CreateObject\0");
        let (c_number, r_number): (CreateNumber, CreateNumber) = libs.both(b"cJSON_CreateNumber\0");
        let (c_add_ref_array, r_add_ref_array): (AddReference, AddReference) =
            libs.both(b"cJSON_AddItemReferenceToArray\0");
        let (c_add_ref_object, r_add_ref_object): (AddReferenceObject, AddReferenceObject) =
            libs.both(b"cJSON_AddItemReferenceToObject\0");
        let key = cstring(b"ref");

        let (c_source, r_source) = (c_number(123.5), r_number(123.5));
        let (ca, ra) = (c_array(), r_array());
        assert_eq!(c_add_ref_array(ca, c_source), r_add_ref_array(ra, r_source));
        assert_same_tree(&libs, ca, ra);
        delete_pair(&libs, ca, ra);
        assert_same_tree(&libs, c_source, r_source);
        delete_pair(&libs, c_source, r_source);

        let (c_source, r_source) = (c_number(-44.0), r_number(-44.0));
        let (co, ro) = (c_object(), r_object());
        assert_eq!(
            c_add_ref_object(co, key.as_ptr(), c_source),
            r_add_ref_object(ro, key.as_ptr(), r_source)
        );
        assert_same_tree(&libs, co, ro);
        delete_pair(&libs, co, ro);
        assert_same_tree(&libs, c_source, r_source);
        delete_pair(&libs, c_source, r_source);

        let simple_names = [
            b"cJSON_AddNullToObject\0".as_slice(),
            b"cJSON_AddTrueToObject\0".as_slice(),
            b"cJSON_AddFalseToObject\0".as_slice(),
            b"cJSON_AddObjectToObject\0".as_slice(),
            b"cJSON_AddArrayToObject\0".as_slice(),
        ];
        for name in simple_names {
            let (cf, rf): (AddSimple, AddSimple) = libs.both(name);
            let (co, ro) = (c_object(), r_object());
            let (ci, ri) = (cf(co, key.as_ptr()), rf(ro, key.as_ptr()));
            assert_eq!(ci.is_null(), ri.is_null());
            assert_same_tree(&libs, co, ro);
            delete_pair(&libs, co, ro);
        }

        let (c_add_bool, r_add_bool): (AddBool, AddBool) = libs.both(b"cJSON_AddBoolToObject\0");
        for boolean in [0, 1, -1, 2] {
            let (co, ro) = (c_object(), r_object());
            assert_eq!(
                c_add_bool(co, key.as_ptr(), boolean).is_null(),
                r_add_bool(ro, key.as_ptr(), boolean).is_null()
            );
            assert_same_tree(&libs, co, ro);
            delete_pair(&libs, co, ro);
        }

        let (c_add_number, r_add_number): (AddNumber, AddNumber) =
            libs.both(b"cJSON_AddNumberToObject\0");
        let (c_add_string, r_add_string): (AddString, AddString) =
            libs.both(b"cJSON_AddStringToObject\0");
        let (c_add_raw, r_add_raw): (AddString, AddString) = libs.both(b"cJSON_AddRawToObject\0");
        let mut rng = Rng::new();
        for _ in 0..100 {
            let value = rng.next_f64();
            let text = rng.ascii(40);
            let raw = cstring(b"[1,true,null]");
            let (co, ro) = (c_object(), r_object());
            assert_eq!(
                c_add_number(co, c"number".as_ptr(), value).is_null(),
                r_add_number(ro, c"number".as_ptr(), value).is_null()
            );
            assert_eq!(
                c_add_string(co, c"string".as_ptr(), text.as_ptr()).is_null(),
                r_add_string(ro, c"string".as_ptr(), text.as_ptr()).is_null()
            );
            assert_eq!(
                c_add_raw(co, c"raw".as_ptr(), raw.as_ptr()).is_null(),
                r_add_raw(ro, c"raw".as_ptr(), raw.as_ptr()).is_null()
            );
            assert_same_tree(&libs, co, ro);
            delete_pair(&libs, co, ro);
        }

        let (c_parse, r_parse): (Parse, Parse) = libs.both(b"cJSON_Parse\0");
        let (c_duplicate, r_duplicate): (Duplicate, Duplicate) = libs.both(b"cJSON_Duplicate\0");
        let (c_compare, r_compare): (Compare, Compare) = libs.both(b"cJSON_Compare\0");
        let documents = [
            cstring(b"null"),
            cstring(b"true"),
            cstring(b"123.5"),
            cstring(br#""text""#),
            cstring(b"[]"),
            cstring(b"[1,2,{\"x\":3}]"),
            cstring(b"{}"),
            cstring(b"{\"A\":1,\"b\":[2,3]}"),
        ];
        for document in &documents {
            let (ci, ri) = (c_parse(document.as_ptr()), r_parse(document.as_ptr()));
            for recurse in [0, 1, -1, 2] {
                let (cd, rd) = (c_duplicate(ci, recurse), r_duplicate(ri, recurse));
                assert_same_tree(&libs, cd, rd);
                for case_sensitive in [0, 1, -1, 2] {
                    assert_eq!(
                        c_compare(ci, cd, case_sensitive),
                        r_compare(ri, rd, case_sensitive)
                    );
                }
                delete_pair(&libs, cd, rd);
            }
            delete_pair(&libs, ci, ri);
        }

        let comparison_pairs = [
            (b"1".as_slice(), b"1.0000000000000002".as_slice()),
            (br#""a""#.as_slice(), br#""b""#.as_slice()),
            (b"[1,2]".as_slice(), b"[1,2,3]".as_slice()),
            (b"[1,2]".as_slice(), b"[2,1]".as_slice()),
            (br#"{"A":1}"#.as_slice(), br#"{"a":1}"#.as_slice()),
            (br#"{"a":1}"#.as_slice(), br#"{"a":1,"b":2}"#.as_slice()),
            (
                br#"{"a":1,"b":2}"#.as_slice(),
                br#"{"b":2,"a":1}"#.as_slice(),
            ),
        ];
        for (left, right) in comparison_pairs {
            let left = cstring(left);
            let right = cstring(right);
            let (cl, rl) = (c_parse(left.as_ptr()), r_parse(left.as_ptr()));
            let (cr, rr) = (c_parse(right.as_ptr()), r_parse(right.as_ptr()));
            for case_sensitive in [0, 1, -1, 2] {
                assert_eq!(
                    c_compare(cl, cr, case_sensitive),
                    r_compare(rl, rr, case_sensitive)
                );
            }
            delete_pair(&libs, cl, rl);
            delete_pair(&libs, cr, rr);
        }
    }
}

#[test]
fn minify_matches_for_comments_whitespace_and_strings() {
    let _guard = TEST_LOCK.lock().unwrap();
    let libs = Libs::new();
    unsafe {
        type Minify = unsafe extern "C" fn(*mut c_char);
        let (c_minify, r_minify): (Minify, Minify) = libs.both(b"cJSON_Minify\0");
        let corpus: &[&[u8]] = &[
            b"",
            b" ",
            b" \t\r\n { \"a\" : 1 } ",
            b"// comment\n{\"a\":1}",
            b"/* comment */ [1, 2]",
            b"/ not-comment /",
            br#"{"s":" spaces // /* */ stay ","q":"a\"b"}"#,
            b"/* unterminated",
            br#""unterminated"#,
            b"{\n// one\n\"a\":1,/*two*/\"b\":2\n}",
        ];
        for input in corpus {
            let mut cb = input.to_vec();
            let mut rb = input.to_vec();
            cb.push(0);
            rb.push(0);
            c_minify(cb.as_mut_ptr().cast());
            r_minify(rb.as_mut_ptr().cast());
            assert_eq!(
                CStr::from_ptr(cb.as_ptr().cast()).to_bytes(),
                CStr::from_ptr(rb.as_ptr().cast()).to_bytes(),
                "{input:?}"
            );
        }
        c_minify(ptr::null_mut());
        r_minify(ptr::null_mut());
    }
}

#[test]
fn public_error_and_boundary_surface_matches() {
    let _guard = TEST_LOCK.lock().unwrap();
    let libs = Libs::new();
    unsafe {
        type Create0 = unsafe extern "C" fn() -> *mut cJSON;
        type CreateNumber = unsafe extern "C" fn(c_double) -> *mut cJSON;
        type CreateString = unsafe extern "C" fn(*const c_char) -> *mut cJSON;
        type GetString = unsafe extern "C" fn(*const cJSON) -> *mut c_char;
        type GetNumber = unsafe extern "C" fn(*const cJSON) -> c_double;
        type SetString = unsafe extern "C" fn(*mut cJSON, *const c_char) -> *mut c_char;
        type Predicate = unsafe extern "C" fn(*const cJSON) -> c_int;
        type GetSize = unsafe extern "C" fn(*const cJSON) -> c_int;
        type GetArray = unsafe extern "C" fn(*const cJSON, c_int) -> *mut cJSON;
        type GetObject = unsafe extern "C" fn(*const cJSON, *const c_char) -> *mut cJSON;
        type AddArray = unsafe extern "C" fn(*mut cJSON, *mut cJSON) -> c_int;
        type AddObject = unsafe extern "C" fn(*mut cJSON, *const c_char, *mut cJSON) -> c_int;
        type AddSimple = unsafe extern "C" fn(*mut cJSON, *const c_char) -> *mut cJSON;
        type AddReferenceObject =
            unsafe extern "C" fn(*mut cJSON, *const c_char, *mut cJSON) -> c_int;
        type DetachPointer = unsafe extern "C" fn(*mut cJSON, *mut cJSON) -> *mut cJSON;
        type DetachArray = unsafe extern "C" fn(*mut cJSON, c_int) -> *mut cJSON;
        type DetachObject = unsafe extern "C" fn(*mut cJSON, *const c_char) -> *mut cJSON;
        type DeleteArray = unsafe extern "C" fn(*mut cJSON, c_int);
        type DeleteObject = unsafe extern "C" fn(*mut cJSON, *const c_char);
        type Insert = unsafe extern "C" fn(*mut cJSON, c_int, *mut cJSON) -> c_int;
        type ReplacePointer = unsafe extern "C" fn(*mut cJSON, *mut cJSON, *mut cJSON) -> c_int;
        type ReplaceArray = unsafe extern "C" fn(*mut cJSON, c_int, *mut cJSON) -> c_int;
        type ReplaceObject = unsafe extern "C" fn(*mut cJSON, *const c_char, *mut cJSON) -> c_int;
        type CreateIntArray = unsafe extern "C" fn(*const c_int, c_int) -> *mut cJSON;
        type CreateFloatArray = unsafe extern "C" fn(*const c_float, c_int) -> *mut cJSON;
        type CreateDoubleArray = unsafe extern "C" fn(*const c_double, c_int) -> *mut cJSON;
        type CreateStringArray = unsafe extern "C" fn(*const *const c_char, c_int) -> *mut cJSON;
        type Duplicate = unsafe extern "C" fn(*const cJSON, c_int) -> *mut cJSON;
        type Compare = unsafe extern "C" fn(*const cJSON, *const cJSON, c_int) -> c_int;
        type PrintBuffered = unsafe extern "C" fn(*const cJSON, c_int, c_int) -> *mut c_char;
        type PrintPreallocated =
            unsafe extern "C" fn(*mut cJSON, *mut c_char, c_int, c_int) -> c_int;

        let (c_get_string, r_get_string): (GetString, GetString) =
            libs.both(b"cJSON_GetStringValue\0");
        let (c_get_number, r_get_number): (GetNumber, GetNumber) =
            libs.both(b"cJSON_GetNumberValue\0");
        assert!(c_get_string(ptr::null()).is_null());
        assert!(r_get_string(ptr::null()).is_null());
        assert!(c_get_number(ptr::null()).is_nan());
        assert!(r_get_number(ptr::null()).is_nan());

        let invalid_item = cJSON {
            next: ptr::null_mut(),
            prev: ptr::null_mut(),
            child: ptr::null_mut(),
            r#type: 255,
            valuestring: ptr::null_mut(),
            valueint: 0,
            valuedouble: 0.0,
            string: ptr::null_mut(),
        };
        assert!(c_get_string(&invalid_item).is_null());
        assert!(r_get_string(&invalid_item).is_null());
        assert!(c_get_number(&invalid_item).is_nan());
        assert!(r_get_number(&invalid_item).is_nan());

        let predicate_names = [
            b"cJSON_IsInvalid\0".as_slice(),
            b"cJSON_IsFalse\0".as_slice(),
            b"cJSON_IsTrue\0".as_slice(),
            b"cJSON_IsBool\0".as_slice(),
            b"cJSON_IsNull\0".as_slice(),
            b"cJSON_IsNumber\0".as_slice(),
            b"cJSON_IsString\0".as_slice(),
            b"cJSON_IsArray\0".as_slice(),
            b"cJSON_IsObject\0".as_slice(),
            b"cJSON_IsRaw\0".as_slice(),
        ];
        for name in predicate_names {
            let (cp, rp): (Predicate, Predicate) = libs.both(name);
            assert_eq!(cp(ptr::null()), rp(ptr::null()));
            assert_eq!(cp(&invalid_item), rp(&invalid_item));
        }

        let (c_create_string, r_create_string): (CreateString, CreateString) =
            libs.both(b"cJSON_CreateString\0");
        let (c_create_raw, r_create_raw): (CreateString, CreateString) =
            libs.both(b"cJSON_CreateRaw\0");
        assert!(c_create_string(ptr::null()).is_null());
        assert!(r_create_string(ptr::null()).is_null());
        assert!(c_create_raw(ptr::null()).is_null());
        assert!(r_create_raw(ptr::null()).is_null());

        let (c_set_string, r_set_string): (SetString, SetString) =
            libs.both(b"cJSON_SetValuestring\0");
        assert!(c_set_string(ptr::null_mut(), c"x".as_ptr()).is_null());
        assert!(r_set_string(ptr::null_mut(), c"x".as_ptr()).is_null());
        let (c_number, r_number): (CreateNumber, CreateNumber) = libs.both(b"cJSON_CreateNumber\0");
        let (cn, rn) = (c_number(1.0), r_number(1.0));
        assert!(c_set_string(cn, c"x".as_ptr()).is_null());
        assert!(r_set_string(rn, c"x".as_ptr()).is_null());
        delete_pair(&libs, cn, rn);
        let original = cstring(b"abcdef");
        let (cs, rs) = (
            c_create_string(original.as_ptr()),
            r_create_string(original.as_ptr()),
        );
        assert!(c_set_string(cs, ptr::null()).is_null());
        assert!(r_set_string(rs, ptr::null()).is_null());
        assert!(c_set_string(cs, (*cs).valuestring).is_null());
        assert!(r_set_string(rs, (*rs).valuestring).is_null());
        assert_same_tree(&libs, cs, rs);
        delete_pair(&libs, cs, rs);
        let mut corrupt_c = cJSON {
            next: ptr::null_mut(),
            prev: ptr::null_mut(),
            child: ptr::null_mut(),
            r#type: CJSON_STRING,
            valuestring: ptr::null_mut(),
            valueint: 0,
            valuedouble: 0.0,
            string: ptr::null_mut(),
        };
        let mut corrupt_r = cJSON { ..corrupt_c };
        assert!(c_set_string(&mut corrupt_c, c"x".as_ptr()).is_null());
        assert!(r_set_string(&mut corrupt_r, c"x".as_ptr()).is_null());

        let (c_print, r_print): (Print, Print) = libs.both(b"cJSON_Print\0");
        let (c_print_u, r_print_u): (Print, Print) = libs.both(b"cJSON_PrintUnformatted\0");
        assert!(c_print(ptr::null()).is_null());
        assert!(r_print(ptr::null()).is_null());
        assert!(c_print_u(&invalid_item).is_null());
        assert!(r_print_u(&invalid_item).is_null());
        let raw_null = cJSON {
            r#type: CJSON_RAW,
            ..invalid_item
        };
        assert!(c_print_u(&raw_null).is_null());
        assert!(r_print_u(&raw_null).is_null());

        let (c_buffered, r_buffered): (PrintBuffered, PrintBuffered) =
            libs.both(b"cJSON_PrintBuffered\0");
        assert!(c_buffered(&invalid_item, -1, 0).is_null());
        assert!(r_buffered(&invalid_item, -1, 0).is_null());
        let (c_pre, r_pre): (PrintPreallocated, PrintPreallocated) =
            libs.both(b"cJSON_PrintPreallocated\0");
        let mut cb = [0u8; 4];
        let mut rb = [0u8; 4];
        assert_eq!(
            c_pre(
                &invalid_item as *const _ as *mut _,
                cb.as_mut_ptr().cast(),
                -1,
                0
            ),
            r_pre(
                &invalid_item as *const _ as *mut _,
                rb.as_mut_ptr().cast(),
                -1,
                0
            )
        );
        assert_eq!(
            c_pre(&invalid_item as *const _ as *mut _, ptr::null_mut(), 4, 0),
            r_pre(&invalid_item as *const _ as *mut _, ptr::null_mut(), 4, 0)
        );
        assert_eq!(
            c_pre(
                &invalid_item as *const _ as *mut _,
                cb.as_mut_ptr().cast(),
                0,
                0
            ),
            r_pre(
                &invalid_item as *const _ as *mut _,
                rb.as_mut_ptr().cast(),
                0,
                0
            )
        );

        let (c_size, r_size): (GetSize, GetSize) = libs.both(b"cJSON_GetArraySize\0");
        let (c_get_array, r_get_array): (GetArray, GetArray) = libs.both(b"cJSON_GetArrayItem\0");
        assert_eq!(c_size(ptr::null()), r_size(ptr::null()));
        for index in [-1, 0, i32::MAX] {
            assert_eq!(
                c_get_array(ptr::null(), index).is_null(),
                r_get_array(ptr::null(), index).is_null()
            );
        }
        let ca = build_number_array(&libs.c, &[1, 2]);
        let ra = build_number_array(&libs.r, &[1, 2]);
        for index in [-1, 2, i32::MAX] {
            assert_eq!(
                c_get_array(ca, index).is_null(),
                r_get_array(ra, index).is_null()
            );
        }
        delete_pair(&libs, ca, ra);

        let (c_get_object, r_get_object): (GetObject, GetObject) =
            libs.both(b"cJSON_GetObjectItem\0");
        let (c_get_object_cs, r_get_object_cs): (GetObject, GetObject) =
            libs.both(b"cJSON_GetObjectItemCaseSensitive\0");
        for (co, ro, key) in [
            (ptr::null(), ptr::null(), c"x".as_ptr()),
            (&invalid_item, &invalid_item, ptr::null()),
        ] {
            assert_eq!(
                c_get_object(co, key).is_null(),
                r_get_object(ro, key).is_null()
            );
            assert_eq!(
                c_get_object_cs(co, key).is_null(),
                r_get_object_cs(ro, key).is_null()
            );
        }

        let (c_create_array, r_create_array): (Create0, Create0) =
            libs.both(b"cJSON_CreateArray\0");
        let (c_create_object, r_create_object): (Create0, Create0) =
            libs.both(b"cJSON_CreateObject\0");
        let (c_add_array, r_add_array): (AddArray, AddArray) = libs.both(b"cJSON_AddItemToArray\0");
        assert_eq!(
            c_add_array(ptr::null_mut(), ptr::null_mut()),
            r_add_array(ptr::null_mut(), ptr::null_mut())
        );
        let (ca, ra) = (c_create_array(), r_create_array());
        assert_eq!(c_add_array(ca, ca), r_add_array(ra, ra));
        delete_pair(&libs, ca, ra);

        let (c_add_object, r_add_object): (AddObject, AddObject) =
            libs.both(b"cJSON_AddItemToObject\0");
        let (c_add_object_cs, r_add_object_cs): (AddObject, AddObject) =
            libs.both(b"cJSON_AddItemToObjectCS\0");
        for (co, ro, key, ci, ri) in [
            (
                ptr::null_mut(),
                ptr::null_mut(),
                c"x".as_ptr(),
                ptr::null_mut(),
                ptr::null_mut(),
            ),
            (
                ptr::null_mut(),
                ptr::null_mut(),
                ptr::null(),
                ptr::null_mut(),
                ptr::null_mut(),
            ),
        ] {
            assert_eq!(c_add_object(co, key, ci), r_add_object(ro, key, ri));
            assert_eq!(c_add_object_cs(co, key, ci), r_add_object_cs(ro, key, ri));
        }
        let (co, ro) = (c_create_object(), r_create_object());
        assert_eq!(
            c_add_object(co, c"x".as_ptr(), co),
            r_add_object(ro, c"x".as_ptr(), ro)
        );
        delete_pair(&libs, co, ro);

        let (c_add_ref_array, r_add_ref_array): (AddArray, AddArray) =
            libs.both(b"cJSON_AddItemReferenceToArray\0");
        let (c_add_ref_object, r_add_ref_object): (AddReferenceObject, AddReferenceObject) =
            libs.both(b"cJSON_AddItemReferenceToObject\0");
        assert_eq!(
            c_add_ref_array(ptr::null_mut(), ptr::null_mut()),
            r_add_ref_array(ptr::null_mut(), ptr::null_mut())
        );
        let (ca, ra) = (c_create_array(), r_create_array());
        assert_eq!(
            c_add_ref_array(ca, ptr::null_mut()),
            r_add_ref_array(ra, ptr::null_mut())
        );
        delete_pair(&libs, ca, ra);
        assert_eq!(
            c_add_ref_object(ptr::null_mut(), ptr::null(), ptr::null_mut()),
            r_add_ref_object(ptr::null_mut(), ptr::null(), ptr::null_mut())
        );

        let convenience = [
            b"cJSON_AddNullToObject\0".as_slice(),
            b"cJSON_AddTrueToObject\0".as_slice(),
            b"cJSON_AddFalseToObject\0".as_slice(),
            b"cJSON_AddObjectToObject\0".as_slice(),
            b"cJSON_AddArrayToObject\0".as_slice(),
        ];
        for name in convenience {
            let (cf, rf): (AddSimple, AddSimple) = libs.both(name);
            assert_eq!(
                cf(ptr::null_mut(), ptr::null()).is_null(),
                rf(ptr::null_mut(), ptr::null()).is_null()
            );
        }

        let (c_detach_pointer, r_detach_pointer): (DetachPointer, DetachPointer) =
            libs.both(b"cJSON_DetachItemViaPointer\0");
        let mut foreign_c = cJSON { ..invalid_item };
        let mut foreign_r = cJSON { ..invalid_item };
        assert_eq!(
            c_detach_pointer(ptr::null_mut(), &mut foreign_c).is_null(),
            r_detach_pointer(ptr::null_mut(), &mut foreign_r).is_null()
        );
        let (ca, ra) = (c_create_array(), r_create_array());
        assert_eq!(
            c_detach_pointer(ca, &mut foreign_c).is_null(),
            r_detach_pointer(ra, &mut foreign_r).is_null()
        );
        delete_pair(&libs, ca, ra);

        let (c_detach_array, r_detach_array): (DetachArray, DetachArray) =
            libs.both(b"cJSON_DetachItemFromArray\0");
        let (c_delete_array, r_delete_array): (DeleteArray, DeleteArray) =
            libs.both(b"cJSON_DeleteItemFromArray\0");
        assert_eq!(
            c_detach_array(ptr::null_mut(), -1).is_null(),
            r_detach_array(ptr::null_mut(), -1).is_null()
        );
        c_delete_array(ptr::null_mut(), i32::MAX);
        r_delete_array(ptr::null_mut(), i32::MAX);
        let (c_detach_object, r_detach_object): (DetachObject, DetachObject) =
            libs.both(b"cJSON_DetachItemFromObject\0");
        let (c_delete_object, r_delete_object): (DeleteObject, DeleteObject) =
            libs.both(b"cJSON_DeleteItemFromObject\0");
        assert_eq!(
            c_detach_object(ptr::null_mut(), ptr::null()).is_null(),
            r_detach_object(ptr::null_mut(), ptr::null()).is_null()
        );
        c_delete_object(ptr::null_mut(), ptr::null());
        r_delete_object(ptr::null_mut(), ptr::null());

        let (c_insert, r_insert): (Insert, Insert) = libs.both(b"cJSON_InsertItemInArray\0");
        assert_eq!(
            c_insert(ptr::null_mut(), -1, ptr::null_mut()),
            r_insert(ptr::null_mut(), -1, ptr::null_mut())
        );
        let (ci, ri) = (c_number(1.0), r_number(1.0));
        assert_eq!(
            c_insert(ptr::null_mut(), 0, ci),
            r_insert(ptr::null_mut(), 0, ri)
        );
        delete_pair(&libs, ci, ri);
        let ca = build_number_array(&libs.c, &[1, 2]);
        let ra = build_number_array(&libs.r, &[1, 2]);
        (*c_get_array(ca, 1)).prev = ptr::null_mut();
        (*r_get_array(ra, 1)).prev = ptr::null_mut();
        let (ci, ri) = (c_number(3.0), r_number(3.0));
        assert_eq!(c_insert(ca, 1, ci), r_insert(ra, 1, ri));
        delete_pair(&libs, ci, ri);
        delete_pair(&libs, ca, ra);

        let (c_replace_pointer, r_replace_pointer): (ReplacePointer, ReplacePointer) =
            libs.both(b"cJSON_ReplaceItemViaPointer\0");
        assert_eq!(
            c_replace_pointer(ptr::null_mut(), ptr::null_mut(), ptr::null_mut()),
            r_replace_pointer(ptr::null_mut(), ptr::null_mut(), ptr::null_mut())
        );
        let ca = build_number_array(&libs.c, &[1]);
        let ra = build_number_array(&libs.r, &[1]);
        let (ci, ri) = (c_get_array(ca, 0), r_get_array(ra, 0));
        assert_eq!(c_replace_pointer(ca, ci, ci), r_replace_pointer(ra, ri, ri));
        delete_pair(&libs, ca, ra);

        let (c_replace_array, r_replace_array): (ReplaceArray, ReplaceArray) =
            libs.both(b"cJSON_ReplaceItemInArray\0");
        let ca = build_number_array(&libs.c, &[1]);
        let ra = build_number_array(&libs.r, &[1]);
        let (ci, ri) = (c_number(2.0), r_number(2.0));
        assert_eq!(c_replace_array(ca, -1, ci), r_replace_array(ra, -1, ri));
        delete_pair(&libs, ci, ri);
        let (ci, ri) = (c_number(2.0), r_number(2.0));
        assert_eq!(
            c_replace_array(ca, i32::MAX, ci),
            r_replace_array(ra, i32::MAX, ri)
        );
        delete_pair(&libs, ci, ri);
        delete_pair(&libs, ca, ra);

        let (c_replace_object, r_replace_object): (ReplaceObject, ReplaceObject) =
            libs.both(b"cJSON_ReplaceItemInObject\0");
        let (ci, ri) = (c_number(2.0), r_number(2.0));
        assert_eq!(
            c_replace_object(ptr::null_mut(), ptr::null(), ci),
            r_replace_object(ptr::null_mut(), ptr::null(), ri)
        );
        delete_pair(&libs, ci, ri);
        let (co, ro) = (c_create_object(), r_create_object());
        let (ci, ri) = (c_number(2.0), r_number(2.0));
        assert_eq!(
            c_replace_object(co, c"absent".as_ptr(), ci),
            r_replace_object(ro, c"absent".as_ptr(), ri)
        );
        delete_pair(&libs, ci, ri);
        delete_pair(&libs, co, ro);

        let (c_ints, r_ints): (CreateIntArray, CreateIntArray) =
            libs.both(b"cJSON_CreateIntArray\0");
        let (c_floats, r_floats): (CreateFloatArray, CreateFloatArray) =
            libs.both(b"cJSON_CreateFloatArray\0");
        let (c_doubles, r_doubles): (CreateDoubleArray, CreateDoubleArray) =
            libs.both(b"cJSON_CreateDoubleArray\0");
        let (c_strings, r_strings): (CreateStringArray, CreateStringArray) =
            libs.both(b"cJSON_CreateStringArray\0");
        let one_i = [1];
        let one_f = [1.0f32];
        let one_d = [1.0f64];
        let one_s = [c"x".as_ptr()];
        for (cp, rp) in [
            (c_ints(ptr::null(), 0), r_ints(ptr::null(), 0)),
            (c_ints(one_i.as_ptr(), -1), r_ints(one_i.as_ptr(), -1)),
            (c_floats(ptr::null(), 0), r_floats(ptr::null(), 0)),
            (c_floats(one_f.as_ptr(), -1), r_floats(one_f.as_ptr(), -1)),
            (c_doubles(ptr::null(), 0), r_doubles(ptr::null(), 0)),
            (c_doubles(one_d.as_ptr(), -1), r_doubles(one_d.as_ptr(), -1)),
            (c_strings(ptr::null(), 0), r_strings(ptr::null(), 0)),
            (c_strings(one_s.as_ptr(), -1), r_strings(one_s.as_ptr(), -1)),
        ] {
            assert_eq!(cp.is_null(), rp.is_null());
        }
        let null_string = [ptr::null()];
        assert_eq!(
            c_strings(null_string.as_ptr(), 1).is_null(),
            r_strings(null_string.as_ptr(), 1).is_null()
        );

        let (c_duplicate, r_duplicate): (Duplicate, Duplicate) = libs.both(b"cJSON_Duplicate\0");
        assert_eq!(
            c_duplicate(ptr::null(), 1).is_null(),
            r_duplicate(ptr::null(), 1).is_null()
        );

        let (c_compare, r_compare): (Compare, Compare) = libs.both(b"cJSON_Compare\0");
        assert_eq!(
            c_compare(ptr::null(), &invalid_item, 0),
            r_compare(ptr::null(), &invalid_item, 0)
        );
        assert_eq!(
            c_compare(&invalid_item, &invalid_item, 0),
            r_compare(&invalid_item, &invalid_item, 0)
        );
        let null_string_item = cJSON {
            r#type: CJSON_STRING,
            ..invalid_item
        };
        assert_eq!(
            c_compare(&null_string_item, &null_string_item, 0),
            r_compare(&null_string_item, &null_string_item, 0)
        );

        let (c_delete, r_delete): (Delete, Delete) = libs.both(b"cJSON_Delete\0");
        c_delete(ptr::null_mut());
        r_delete(ptr::null_mut());
        let (c_free, r_free): (Free, Free) = libs.both(b"cJSON_free\0");
        c_free(ptr::null_mut());
        r_free(ptr::null_mut());
    }
}

#[test]
fn allocator_hook_success_and_failure_paths_match() {
    let _guard = TEST_LOCK.lock().unwrap();
    let libs = Libs::new();
    unsafe {
        type InitHooks = unsafe extern "C" fn(*mut Hooks);
        type Parse = unsafe extern "C" fn(*const c_char) -> *mut cJSON;
        type Create0 = unsafe extern "C" fn() -> *mut cJSON;
        type CreateString = unsafe extern "C" fn(*const c_char) -> *mut cJSON;
        type AddObject = unsafe extern "C" fn(*mut cJSON, *const c_char, *mut cJSON) -> c_int;
        type Duplicate = unsafe extern "C" fn(*const cJSON, c_int) -> *mut cJSON;
        type Malloc = unsafe extern "C" fn(usize) -> *mut c_void;

        let (c_init, r_init): (InitHooks, InitHooks) = libs.both(b"cJSON_InitHooks\0");
        let mut custom = Hooks {
            malloc_fn: Some(test_alloc),
            free_fn: Some(test_free),
        };
        c_init(&mut custom);
        r_init(&mut custom);
        let document = CString::new(format!(
            "{{\"long\":[1,2,3,4,5,6,7,8,9],\"text\":\"{}\"}}",
            "abcdefghijklmnopqrstuvwxyz".repeat(30)
        ))
        .unwrap();
        let (c_parse, r_parse): (Parse, Parse) = libs.both(b"cJSON_Parse\0");
        let (ci, ri) = (c_parse(document.as_ptr()), r_parse(document.as_ptr()));
        assert_same_tree(&libs, ci, ri);
        delete_pair(&libs, ci, ri);

        let (c_malloc, r_malloc): (Malloc, Malloc) = libs.both(b"cJSON_malloc\0");
        let (c_free, r_free): (Free, Free) = libs.both(b"cJSON_free\0");
        for size in [0usize, 1, 8, 255, 4096] {
            let (cp, rp) = (c_malloc(size), r_malloc(size));
            assert_eq!(cp.is_null(), rp.is_null());
            c_free(cp);
            r_free(rp);
        }

        c_init(ptr::null_mut());
        r_init(ptr::null_mut());

        let (c_create_object, r_create_object): (Create0, Create0) =
            libs.both(b"cJSON_CreateObject\0");
        let (c_create_array, r_create_array): (Create0, Create0) =
            libs.both(b"cJSON_CreateArray\0");
        let (c_create_string, r_create_string): (CreateString, CreateString) =
            libs.both(b"cJSON_CreateString\0");
        let (c_add_object, r_add_object): (AddObject, AddObject) =
            libs.both(b"cJSON_AddItemToObject\0");
        let (c_duplicate, r_duplicate): (Duplicate, Duplicate) = libs.both(b"cJSON_Duplicate\0");

        let (co, ro) = (c_create_object(), r_create_object());
        let (ca, ra) = (c_create_array(), r_create_array());
        let (c_source, r_source) = (
            c_create_string(c"source".as_ptr()),
            r_create_string(c"source".as_ptr()),
        );
        let text = cstring(b"a value longer than the existing storage");
        let (cs, rs) = (
            c_create_string(c"short".as_ptr()),
            r_create_string(c"short".as_ptr()),
        );
        let mut failing = Hooks {
            malloc_fn: Some(fail_alloc),
            free_fn: Some(test_free),
        };
        c_init(&mut failing);
        r_init(&mut failing);

        for name in [
            b"cJSON_CreateNull\0".as_slice(),
            b"cJSON_CreateTrue\0".as_slice(),
            b"cJSON_CreateFalse\0".as_slice(),
            b"cJSON_CreateArray\0".as_slice(),
            b"cJSON_CreateObject\0".as_slice(),
        ] {
            let (cc, rc): (Create0, Create0) = libs.both(name);
            assert!(cc().is_null());
            assert!(rc().is_null());
        }
        type CreateBool = unsafe extern "C" fn(c_int) -> *mut cJSON;
        type CreateNumber = unsafe extern "C" fn(c_double) -> *mut cJSON;
        let (c_bool, r_bool): (CreateBool, CreateBool) = libs.both(b"cJSON_CreateBool\0");
        let (c_number, r_number): (CreateNumber, CreateNumber) = libs.both(b"cJSON_CreateNumber\0");
        assert!(c_bool(1).is_null());
        assert!(r_bool(1).is_null());
        assert!(c_number(1.0).is_null());
        assert!(r_number(1.0).is_null());
        assert!(c_create_string(c"x".as_ptr()).is_null());
        assert!(r_create_string(c"x".as_ptr()).is_null());
        let (c_raw, r_raw): (CreateString, CreateString) = libs.both(b"cJSON_CreateRaw\0");
        assert!(c_raw(c"1".as_ptr()).is_null());
        assert!(r_raw(c"1".as_ptr()).is_null());
        assert!(c_parse(document.as_ptr()).is_null());
        assert!(r_parse(document.as_ptr()).is_null());
        assert!(c_print_failure(&libs.c, &invalid_printable_number()).is_none());
        assert!(c_print_failure(&libs.r, &invalid_printable_number()).is_none());
        assert_eq!(
            c_add_object(co, c"k".as_ptr(), cs),
            r_add_object(ro, c"k".as_ptr(), rs)
        );
        assert!(c_duplicate(co, 1).is_null());
        assert!(r_duplicate(ro, 1).is_null());
        type SetString = unsafe extern "C" fn(*mut cJSON, *const c_char) -> *mut c_char;
        let (c_set, r_set): (SetString, SetString) = libs.both(b"cJSON_SetValuestring\0");
        assert!(c_set(cs, text.as_ptr()).is_null());
        assert!(r_set(rs, text.as_ptr()).is_null());
        assert!(c_malloc(usize::MAX).is_null());
        assert!(r_malloc(usize::MAX).is_null());
        type AddReference = unsafe extern "C" fn(*mut cJSON, *mut cJSON) -> c_int;
        let (c_add_ref, r_add_ref): (AddReference, AddReference) =
            libs.both(b"cJSON_AddItemReferenceToArray\0");
        assert_eq!(c_add_ref(ca, c_source), r_add_ref(ra, r_source));
        type AddReferenceObject =
            unsafe extern "C" fn(*mut cJSON, *const c_char, *mut cJSON) -> c_int;
        let (c_add_ref_object, r_add_ref_object): (AddReferenceObject, AddReferenceObject) =
            libs.both(b"cJSON_AddItemReferenceToObject\0");
        assert_eq!(
            c_add_ref_object(co, c"ref".as_ptr(), c_source),
            r_add_ref_object(ro, c"ref".as_ptr(), r_source)
        );
        type AddSimple = unsafe extern "C" fn(*mut cJSON, *const c_char) -> *mut cJSON;
        for name in [
            b"cJSON_AddNullToObject\0".as_slice(),
            b"cJSON_AddTrueToObject\0".as_slice(),
            b"cJSON_AddFalseToObject\0".as_slice(),
            b"cJSON_AddObjectToObject\0".as_slice(),
            b"cJSON_AddArrayToObject\0".as_slice(),
        ] {
            let (cf, rf): (AddSimple, AddSimple) = libs.both(name);
            assert_eq!(
                cf(co, c"simple".as_ptr()).is_null(),
                rf(ro, c"simple".as_ptr()).is_null()
            );
        }
        type AddBool = unsafe extern "C" fn(*mut cJSON, *const c_char, c_int) -> *mut cJSON;
        type AddNumber = unsafe extern "C" fn(*mut cJSON, *const c_char, c_double) -> *mut cJSON;
        type AddString =
            unsafe extern "C" fn(*mut cJSON, *const c_char, *const c_char) -> *mut cJSON;
        let (c_add_bool, r_add_bool): (AddBool, AddBool) = libs.both(b"cJSON_AddBoolToObject\0");
        let (c_add_number, r_add_number): (AddNumber, AddNumber) =
            libs.both(b"cJSON_AddNumberToObject\0");
        let (c_add_string, r_add_string): (AddString, AddString) =
            libs.both(b"cJSON_AddStringToObject\0");
        let (c_add_raw, r_add_raw): (AddString, AddString) = libs.both(b"cJSON_AddRawToObject\0");
        assert_eq!(
            c_add_bool(co, c"bool".as_ptr(), 1).is_null(),
            r_add_bool(ro, c"bool".as_ptr(), 1).is_null()
        );
        assert_eq!(
            c_add_number(co, c"number".as_ptr(), 1.0).is_null(),
            r_add_number(ro, c"number".as_ptr(), 1.0).is_null()
        );
        assert_eq!(
            c_add_string(co, c"string".as_ptr(), c"x".as_ptr()).is_null(),
            r_add_string(ro, c"string".as_ptr(), c"x".as_ptr()).is_null()
        );
        assert_eq!(
            c_add_raw(co, c"raw".as_ptr(), c"1".as_ptr()).is_null(),
            r_add_raw(ro, c"raw".as_ptr(), c"1".as_ptr()).is_null()
        );
        type CreateIntArray = unsafe extern "C" fn(*const c_int, c_int) -> *mut cJSON;
        type CreateFloatArray = unsafe extern "C" fn(*const c_float, c_int) -> *mut cJSON;
        type CreateDoubleArray = unsafe extern "C" fn(*const c_double, c_int) -> *mut cJSON;
        type CreateStringArray = unsafe extern "C" fn(*const *const c_char, c_int) -> *mut cJSON;
        let (c_ints, r_ints): (CreateIntArray, CreateIntArray) =
            libs.both(b"cJSON_CreateIntArray\0");
        let (c_floats, r_floats): (CreateFloatArray, CreateFloatArray) =
            libs.both(b"cJSON_CreateFloatArray\0");
        let (c_doubles, r_doubles): (CreateDoubleArray, CreateDoubleArray) =
            libs.both(b"cJSON_CreateDoubleArray\0");
        let (c_strings, r_strings): (CreateStringArray, CreateStringArray) =
            libs.both(b"cJSON_CreateStringArray\0");
        let iv = [1];
        let fv = [1.0f32];
        let dv = [1.0f64];
        let sv = [c"x".as_ptr()];
        assert!(c_ints(iv.as_ptr(), 1).is_null());
        assert!(r_ints(iv.as_ptr(), 1).is_null());
        assert!(c_floats(fv.as_ptr(), 1).is_null());
        assert!(r_floats(fv.as_ptr(), 1).is_null());
        assert!(c_doubles(dv.as_ptr(), 1).is_null());
        assert!(r_doubles(dv.as_ptr(), 1).is_null());
        assert!(c_strings(sv.as_ptr(), 1).is_null());
        assert!(r_strings(sv.as_ptr(), 1).is_null());
        type PrintBuffered = unsafe extern "C" fn(*const cJSON, c_int, c_int) -> *mut c_char;
        let (c_buffered, r_buffered): (PrintBuffered, PrintBuffered) =
            libs.both(b"cJSON_PrintBuffered\0");
        assert_eq!(
            c_buffered(c_source, 8, 0).is_null(),
            r_buffered(r_source, 8, 0).is_null()
        );

        c_init(ptr::null_mut());
        r_init(ptr::null_mut());
        delete_pair(&libs, cs, rs);
        delete_pair(&libs, co, ro);
        delete_pair(&libs, ca, ra);
        delete_pair(&libs, c_source, r_source);

        let (c_budget_source, r_budget_source) = (
            c_create_string(c"budget source".as_ptr()),
            r_create_string(c"budget source".as_ptr()),
        );
        let key = cstring(b"key");
        let budget_entries = [(&key, 1)];
        let c_budget_object = build_object(&libs.c, &budget_entries);
        let r_budget_object = build_object(&libs.r, &budget_entries);
        type GetArray = unsafe extern "C" fn(*const cJSON, c_int) -> *mut cJSON;
        let (c_get_array, r_get_array): (GetArray, GetArray) = libs.both(b"cJSON_GetArrayItem\0");
        let (c_budget_child, r_budget_child) = (
            c_get_array(c_budget_object, 0),
            r_get_array(r_budget_object, 0),
        );
        let mut budgeted = Hooks {
            malloc_fn: Some(budget_alloc),
            free_fn: Some(test_free),
        };
        c_init(&mut budgeted);
        r_init(&mut budgeted);

        ALLOC_BUDGET.store(1, Ordering::SeqCst);
        let c_dup = c_duplicate(c_budget_source, 1);
        ALLOC_BUDGET.store(1, Ordering::SeqCst);
        let r_dup = r_duplicate(r_budget_source, 1);
        assert!(c_dup.is_null() && r_dup.is_null());

        ALLOC_BUDGET.store(1, Ordering::SeqCst);
        let c_key_dup = c_duplicate(c_budget_child, 1);
        ALLOC_BUDGET.store(1, Ordering::SeqCst);
        let r_key_dup = r_duplicate(r_budget_child, 1);
        assert!(c_key_dup.is_null() && r_key_dup.is_null());

        let values = [1, 2, 3];
        ALLOC_BUDGET.store(2, Ordering::SeqCst);
        let c_partial = c_ints(values.as_ptr(), values.len() as c_int);
        ALLOC_BUDGET.store(2, Ordering::SeqCst);
        let r_partial = r_ints(values.as_ptr(), values.len() as c_int);
        assert!(c_partial.is_null() && r_partial.is_null());

        let strings = [c"a".as_ptr(), c"b".as_ptr()];
        ALLOC_BUDGET.store(4, Ordering::SeqCst);
        let c_string_partial = c_strings(strings.as_ptr(), strings.len() as c_int);
        ALLOC_BUDGET.store(4, Ordering::SeqCst);
        let r_string_partial = r_strings(strings.as_ptr(), strings.len() as c_int);
        assert!(c_string_partial.is_null() && r_string_partial.is_null());

        let one = cstring(b"1");
        ALLOC_BUDGET.store(1, Ordering::SeqCst);
        let c_parse_fail = c_parse(one.as_ptr());
        ALLOC_BUDGET.store(1, Ordering::SeqCst);
        let r_parse_fail = r_parse(one.as_ptr());
        assert!(c_parse_fail.is_null() && r_parse_fail.is_null());

        c_init(ptr::null_mut());
        r_init(ptr::null_mut());
        delete_pair(&libs, c_budget_source, r_budget_source);
        delete_pair(&libs, c_budget_object, r_budget_object);
    }
}

fn invalid_printable_number() -> cJSON {
    cJSON {
        next: ptr::null_mut(),
        prev: ptr::null_mut(),
        child: ptr::null_mut(),
        r#type: CJSON_NUMBER,
        valuestring: ptr::null_mut(),
        valueint: 1,
        valuedouble: 1.0,
        string: ptr::null_mut(),
    }
}

unsafe fn c_print_failure(lib: &Library, item: *const cJSON) -> Option<Vec<u8>> {
    unsafe { rendered(lib, item, false) }
}

#[test]
fn duplicate_circular_limit_matches() {
    let _guard = TEST_LOCK.lock().unwrap();
    std::thread::Builder::new()
        .name("duplicate-circular-limit".into())
        .stack_size(64 * 1024 * 1024)
        .spawn(|| unsafe {
            type Duplicate = unsafe extern "C" fn(*const cJSON, c_int) -> *mut cJSON;
            let libs = Libs::new();
            let (c_duplicate, r_duplicate): (Duplicate, Duplicate) =
                libs.both(b"cJSON_Duplicate\0");
            let mut nodes: Vec<Box<cJSON>> = (0..10002)
                .map(|index| {
                    Box::new(cJSON {
                        next: ptr::null_mut(),
                        prev: ptr::null_mut(),
                        child: ptr::null_mut(),
                        r#type: if index == 10001 {
                            CJSON_NULL
                        } else {
                            CJSON_ARRAY
                        },
                        valuestring: ptr::null_mut(),
                        valueint: 0,
                        valuedouble: 0.0,
                        string: ptr::null_mut(),
                    })
                })
                .collect();
            for index in 0..10001 {
                nodes[index].child = (&mut *nodes[index + 1]) as *mut cJSON;
            }
            let root = (&*nodes[0]) as *const cJSON;
            let c_result = c_duplicate(root, 1);
            let r_result = r_duplicate(root, 1);
            assert!(c_result.is_null());
            assert!(r_result.is_null());
        })
        .unwrap()
        .join()
        .unwrap();
}

#[test]
fn remaining_reference_compare_and_composed_shapes_match() {
    let _guard = TEST_LOCK.lock().unwrap();
    let libs = Libs::new();
    unsafe {
        type CreateRef = unsafe extern "C" fn(*const cJSON) -> *mut cJSON;
        type CreateString = unsafe extern "C" fn(*const c_char) -> *mut cJSON;
        type SetString = unsafe extern "C" fn(*mut cJSON, *const c_char) -> *mut c_char;
        type Compare = unsafe extern "C" fn(*const cJSON, *const cJSON, c_int) -> c_int;
        type GetObject = unsafe extern "C" fn(*const cJSON, *const c_char) -> *mut cJSON;
        type Parse = unsafe extern "C" fn(*const c_char) -> *mut cJSON;
        type GetArray = unsafe extern "C" fn(*const cJSON, c_int) -> *mut cJSON;
        type ReplaceArray = unsafe extern "C" fn(*mut cJSON, c_int, *mut cJSON) -> c_int;
        type CreateNumber = unsafe extern "C" fn(c_double) -> *mut cJSON;
        type ReplaceObject = unsafe extern "C" fn(*mut cJSON, *const c_char, *mut cJSON) -> c_int;

        let (c_array_ref, r_array_ref): (CreateRef, CreateRef) =
            libs.both(b"cJSON_CreateArrayReference\0");
        let c_source = build_number_array(&libs.c, &[1, 2, 3]);
        let r_source = build_number_array(&libs.r, &[1, 2, 3]);
        let (cr, rr) = (c_array_ref(c_source), r_array_ref(r_source));
        assert_same_tree(&libs, cr, rr);
        delete_pair(&libs, cr, rr);
        assert_same_tree(&libs, c_source, r_source);
        delete_pair(&libs, c_source, r_source);

        let ka = cstring(b"a");
        let kb = cstring(b"b");
        let entries = [(&ka, 1), (&kb, 2)];
        let c_source = build_object(&libs.c, &entries);
        let r_source = build_object(&libs.r, &entries);
        let (c_object_ref, r_object_ref): (CreateRef, CreateRef) =
            libs.both(b"cJSON_CreateObjectReference\0");
        let (cr, rr) = (c_object_ref(c_source), r_object_ref(r_source));
        assert_same_tree(&libs, cr, rr);
        delete_pair(&libs, cr, rr);
        assert_same_tree(&libs, c_source, r_source);
        delete_pair(&libs, c_source, r_source);

        let (c_string_ref, r_string_ref): (CreateString, CreateString) =
            libs.both(b"cJSON_CreateStringReference\0");
        let text = cstring(b"borrowed");
        let (cs, rs) = (c_string_ref(text.as_ptr()), r_string_ref(text.as_ptr()));
        let (c_set, r_set): (SetString, SetString) = libs.both(b"cJSON_SetValuestring\0");
        assert_eq!(
            c_set(cs, c"new".as_ptr()).is_null(),
            r_set(rs, c"new".as_ptr()).is_null()
        );
        delete_pair(&libs, cs, rs);
        let (cs, rs) = (c_string_ref(ptr::null()), r_string_ref(ptr::null()));
        assert_same_tree(&libs, cs, rs);
        delete_pair(&libs, cs, rs);

        let (c_parse, r_parse): (Parse, Parse) = libs.both(b"cJSON_Parse\0");
        let (c_compare, r_compare): (Compare, Compare) = libs.both(b"cJSON_Compare\0");
        let doc = cstring(b"{\"a\":[1,2,3],\"b\":true}");
        let (ci, ri) = (c_parse(doc.as_ptr()), r_parse(doc.as_ptr()));
        for case_sensitive in [0, 1, -1, 2] {
            assert_eq!(
                c_compare(ci, ci, case_sensitive),
                r_compare(ri, ri, case_sensitive)
            );
        }

        let (c_get, r_get): (GetObject, GetObject) =
            libs.both(b"cJSON_GetObjectItemCaseSensitive\0");
        let (c_get_array, r_get_array): (GetArray, GetArray) = libs.both(b"cJSON_GetArrayItem\0");
        let (c_replace_array, r_replace_array): (ReplaceArray, ReplaceArray) =
            libs.both(b"cJSON_ReplaceItemInArray\0");
        let (c_number, r_number): (CreateNumber, CreateNumber) = libs.both(b"cJSON_CreateNumber\0");
        let (ca, ra) = (c_get(ci, c"a".as_ptr()), r_get(ri, c"a".as_ptr()));
        assert_same_tree(&libs, c_get_array(ca, 1), r_get_array(ra, 1));
        assert_eq!(
            c_replace_array(ca, 1, c_number(99.0)),
            r_replace_array(ra, 1, r_number(99.0))
        );
        assert_same_tree(&libs, ci, ri);
        delete_pair(&libs, ci, ri);

        let mut child_c = cJSON {
            next: ptr::null_mut(),
            prev: ptr::null_mut(),
            child: ptr::null_mut(),
            r#type: CJSON_NUMBER,
            valuestring: ptr::null_mut(),
            valueint: 1,
            valuedouble: 1.0,
            string: ptr::null_mut(),
        };
        let mut child_r = cJSON { ..child_c };
        let object_c = cJSON {
            child: &mut child_c,
            r#type: CJSON_OBJECT,
            ..child_c
        };
        let object_r = cJSON {
            child: &mut child_r,
            r#type: CJSON_OBJECT,
            ..child_r
        };
        let (c_get_ci, r_get_ci): (GetObject, GetObject) = libs.both(b"cJSON_GetObjectItem\0");
        assert_eq!(
            c_get_ci(&object_c, c"x".as_ptr()).is_null(),
            r_get_ci(&object_r, c"x".as_ptr()).is_null()
        );

        let null_string_c1 = cJSON {
            r#type: CJSON_STRING,
            ..child_c
        };
        let null_string_c2 = cJSON {
            r#type: CJSON_STRING,
            ..child_c
        };
        let null_string_r1 = cJSON {
            r#type: CJSON_STRING,
            ..child_r
        };
        let null_string_r2 = cJSON {
            r#type: CJSON_STRING,
            ..child_r
        };
        assert_eq!(
            c_compare(&null_string_c1, &null_string_c2, 0),
            r_compare(&null_string_r1, &null_string_r2, 0)
        );

        let (c_raw, r_raw): (CreateString, CreateString) = libs.both(b"cJSON_CreateRaw\0");
        let (craw1, rraw1) = (c_raw(c"1".as_ptr()), r_raw(c"1".as_ptr()));
        let (craw2, rraw2) = (c_raw(c"2".as_ptr()), r_raw(c"2".as_ptr()));
        assert_eq!(c_compare(craw1, craw2, 0), r_compare(rraw1, rraw2, 0));
        delete_pair(&libs, craw1, rraw1);
        delete_pair(&libs, craw2, rraw2);

        let (c_replace_cs, r_replace_cs): (ReplaceObject, ReplaceObject) =
            libs.both(b"cJSON_ReplaceItemInObjectCaseSensitive\0");
        let co = build_object(&libs.c, &entries);
        let ro = build_object(&libs.r, &entries);
        let (cn, rn) = (c_number(3.0), r_number(3.0));
        assert_eq!(
            c_replace_cs(co, c"A".as_ptr(), cn),
            r_replace_cs(ro, c"A".as_ptr(), rn)
        );
        delete_pair(&libs, cn, rn);
        delete_pair(&libs, co, ro);
    }
}
