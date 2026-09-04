#![allow(non_camel_case_types, non_snake_case, unsafe_op_in_unsafe_fn)]

use libloading::Library;
use std::ffi::{CStr, CString, c_char, c_double, c_float, c_int, c_void};
use std::fs::File;
use std::io::Read;
use std::os::fd::FromRawFd;
use std::path::{Path, PathBuf};
use std::ptr;
use std::sync::atomic::{AtomicIsize, AtomicUsize, Ordering};

const CJSON_RAW: c_int = 128;
const CJSON_IS_REFERENCE: c_int = 256;
const CJSON_STRING_IS_CONST: c_int = 512;

#[repr(C)]
struct cJSON {
    next: *mut cJSON,
    prev: *mut cJSON,
    child: *mut cJSON,
    kind: c_int,
    valuestring: *mut c_char,
    valueint: c_int,
    valuedouble: c_double,
    string: *mut c_char,
}

type Allocate = unsafe extern "C" fn(usize) -> *mut c_void;
type Deallocate = unsafe extern "C" fn(*mut c_void);

#[repr(C)]
struct cJSON_Hooks {
    malloc_fn: Option<Allocate>,
    free_fn: Option<Deallocate>,
}

#[repr(C)]
struct Record {
    precision: *const c_char,
    lat: c_double,
    lon: c_double,
    address: *const c_char,
    city: *const c_char,
    state: *const c_char,
    zip: *const c_char,
    country: *const c_char,
}

unsafe extern "C" {
    fn malloc(size: usize) -> *mut c_void;
    fn free(pointer: *mut c_void);
    fn fflush(stream: *mut c_void) -> c_int;
    fn pipe(fds: *mut c_int) -> c_int;
    fn dup(fd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
}

struct Api {
    name: &'static str,
    lib: Library,
    driver_lib: Option<Library>,
}

impl Api {
    unsafe fn load(name: &'static str, library: &Path, driver_library: Option<&Path>) -> Self {
        let lib = Library::new(library)
            .unwrap_or_else(|error| panic!("load {name} {}: {error}", library.display()));
        let driver_lib = driver_library.map(|path| {
            Library::new(path)
                .unwrap_or_else(|error| panic!("load {name} driver {}: {error}", path.display()))
        });
        Self {
            name,
            lib,
            driver_lib,
        }
    }

    unsafe fn symbol<T: Copy>(&self, name: &[u8]) -> T {
        *self
            .lib
            .get::<T>(name)
            .unwrap_or_else(|error| panic!("{} missing {:?}: {error}", self.name, name))
    }

    unsafe fn driver_symbol<T: Copy>(&self, name: &[u8]) -> T {
        let lib = self.driver_lib.as_ref().unwrap_or(&self.lib);
        *lib.get::<T>(name)
            .unwrap_or_else(|error| panic!("{} missing driver: {error}", self.name))
    }
}

fn library_paths() -> (PathBuf, PathBuf, PathBuf) {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let c_root = manifest.parent().unwrap().join("c_src").join("build");
    (
        c_root.join("libcjson.so"),
        c_root.join("libcJSON_test.so"),
        manifest.join("target/release/libcJSON_test.so"),
    )
}

unsafe fn delete(api: &Api, item: *mut cJSON) {
    let function: unsafe extern "C" fn(*mut cJSON) = api.symbol(b"cJSON_Delete\0");
    function(item);
}

unsafe fn cjson_free(api: &Api, value: *mut c_void) {
    let function: unsafe extern "C" fn(*mut c_void) = api.symbol(b"cJSON_free\0");
    function(value);
}

unsafe fn printed(api: &Api, item: *const cJSON, formatted: bool) -> Option<Vec<u8>> {
    let name = if formatted {
        b"cJSON_Print\0".as_slice()
    } else {
        b"cJSON_PrintUnformatted\0".as_slice()
    };
    let function: unsafe extern "C" fn(*const cJSON) -> *mut c_char = api.symbol(name);
    let output = function(item);
    if output.is_null() {
        return None;
    }
    let bytes = CStr::from_ptr(output).to_bytes().to_vec();
    cjson_free(api, output.cast());
    Some(bytes)
}

unsafe fn parse(
    api: &Api,
    input: &[u8],
    variant: usize,
    explicit_length: usize,
    require_nul: c_int,
) -> ParseResult {
    let mut storage = input.to_vec();
    if storage.last().copied() != Some(0) {
        storage.push(0);
    }
    let base = storage.as_ptr().cast::<c_char>();
    let mut end: *const c_char = ptr::null();
    let item = match variant {
        0 => {
            let function: unsafe extern "C" fn(*const c_char) -> *mut cJSON =
                api.symbol(b"cJSON_Parse\0");
            function(base)
        }
        1 => {
            let function: unsafe extern "C" fn(*const c_char, usize) -> *mut cJSON =
                api.symbol(b"cJSON_ParseWithLength\0");
            function(base, explicit_length)
        }
        2 => {
            let function: unsafe extern "C" fn(
                *const c_char,
                *mut *const c_char,
                c_int,
            ) -> *mut cJSON = api.symbol(b"cJSON_ParseWithOpts\0");
            function(base, &mut end, require_nul)
        }
        _ => {
            let function: unsafe extern "C" fn(
                *const c_char,
                usize,
                *mut *const c_char,
                c_int,
            ) -> *mut cJSON = api.symbol(b"cJSON_ParseWithLengthOpts\0");
            function(base, explicit_length, &mut end, require_nul)
        }
    };
    let end_offset = (!end.is_null()).then(|| end.offset_from(base));
    let error_function: unsafe extern "C" fn() -> *const c_char =
        api.symbol(b"cJSON_GetErrorPtr\0");
    let error = error_function();
    let error_offset = (!error.is_null()).then(|| error.offset_from(base));
    let compact = if item.is_null() {
        None
    } else {
        printed(api, item, false)
    };
    let formatted = if item.is_null() {
        None
    } else {
        printed(api, item, true)
    };
    delete(api, item);
    ParseResult {
        ok: !item.is_null(),
        end_offset,
        error_offset,
        compact,
        formatted,
    }
}

#[derive(Debug, PartialEq, Eq)]
struct ParseResult {
    ok: bool,
    end_offset: Option<isize>,
    error_offset: Option<isize>,
    compact: Option<Vec<u8>>,
    formatted: Option<Vec<u8>>,
}

struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Self(seed)
    }

    fn next(&mut self) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        self.0
    }

    fn range(&mut self, upper: usize) -> usize {
        (self.next() as usize) % upper
    }
}

fn random_json_string(rng: &mut Rng) -> String {
    let mut output = String::from("\"");
    for _ in 0..rng.range(18) {
        match rng.range(14) {
            0 => output.push_str("\\\""),
            1 => output.push_str("\\\\"),
            2 => output.push_str("\\n"),
            3 => output.push_str("\\t"),
            4 => output.push_str("\\r"),
            5 => output.push_str("\\b"),
            6 => output.push_str("\\f"),
            7 => output.push_str("\\/"),
            8 => output.push_str("\\u00df"),
            9 => output.push_str("\\u6771"),
            10 => output.push_str("\\ud834\\udd1e"),
            _ => output.push((b'a' + rng.range(26) as u8) as char),
        }
    }
    output.push('"');
    output
}

fn random_json(rng: &mut Rng, depth: usize) -> String {
    if depth == 0 {
        return match rng.range(6) {
            0 => "null".into(),
            1 => "true".into(),
            2 => "false".into(),
            3 => format!("{}", rng.next() as i64),
            4 => format!(
                "{}.{}e{}",
                rng.range(10000),
                rng.range(100000),
                rng.range(30)
            ),
            _ => random_json_string(rng),
        };
    }
    match rng.range(8) {
        0..=4 => random_json(rng, 0),
        5 | 6 => {
            let mut parts = Vec::new();
            for _ in 0..rng.range(6) {
                parts.push(random_json(rng, depth - 1));
            }
            format!("[{}]", parts.join(","))
        }
        _ => {
            let mut parts = Vec::new();
            for index in 0..rng.range(6) {
                parts.push(format!(
                    "\"k{}{}\":{}",
                    index,
                    rng.range(10),
                    random_json(rng, depth - 1)
                ));
            }
            format!("{{{}}}", parts.join(","))
        }
    }
}

unsafe fn compare_parse_surfaces(c: &Api, rust: &Api) {
    let fixed = [
        "null",
        "true",
        "false",
        "0",
        "-0",
        "2147483647",
        "-2147483648",
        "1.5",
        "1e+20",
        "1e-20",
        "4.9406564584124654e-324",
        "\"\"",
        "\"a\\\\b\\n\\t\\\"c\\/d\\b\\f\\r\"",
        "\"\\u0041\\u00df\\u6771\\ud834\\udd1e\"",
        "[]",
        "[1,true,null,\"x\",{\"k\":2}]",
        "{}",
        "{\"a\":1,\"B\":[2,3],\"a\":4}",
        " \t\r\n [ 1 , 2 ] ",
        "\u{feff}{\"bom\":true}",
    ];
    for input in fixed {
        let bytes = input.as_bytes();
        for variant in 0..4 {
            for require in [0, 1, 7, -3] {
                if variant < 2 && require != 0 {
                    continue;
                }
                let length = bytes.len() + 1;
                assert_eq!(
                    parse(c, bytes, variant, length, require),
                    parse(rust, bytes, variant, length, require),
                    "parse fixed {input:?}, variant={variant}, require={require}"
                );
            }
        }
    }

    let mut rng = Rng::new(0xc0ffee_17519);
    for case in 0..500 {
        let json = random_json(&mut rng, 3);
        let mut decorated = String::new();
        if case % 3 == 0 {
            decorated.push_str(" \t\n");
        }
        decorated.push_str(&json);
        if case % 5 == 0 {
            decorated.push_str(" \r\n");
        }
        let bytes = decorated.as_bytes();
        for &(variant, require) in &[(0, 0), (1, 0), (2, 0), (2, 1), (3, 0), (3, 1)] {
            assert_eq!(
                parse(c, bytes, variant, bytes.len() + 1, require),
                parse(rust, bytes, variant, bytes.len() + 1, require),
                "random parse case={case}, json={decorated:?}, variant={variant}, require={require}"
            );
        }
    }

    let bounded = b"[1,2]\0garbage";
    for length in [1, 2, 5, 6, 7, 8, bounded.len()] {
        for require in [0, 1] {
            assert_eq!(
                parse(c, bounded, 3, length, require),
                parse(rust, bounded, 3, length, require),
                "bounded length={length}, require={require}"
            );
        }
    }
    assert_eq!(
        parse(c, b"null", 3, usize::MAX, 0),
        parse(rust, b"null", 3, usize::MAX, 0),
        "oversized parse length"
    );
}

unsafe fn create0(api: &Api, name: &[u8]) -> *mut cJSON {
    let function: unsafe extern "C" fn() -> *mut cJSON = api.symbol(name);
    function()
}

unsafe fn create_number(api: &Api, value: f64) -> *mut cJSON {
    let function: unsafe extern "C" fn(f64) -> *mut cJSON = api.symbol(b"cJSON_CreateNumber\0");
    function(value)
}

unsafe fn create_string(api: &Api, value: *const c_char) -> *mut cJSON {
    let function: unsafe extern "C" fn(*const c_char) -> *mut cJSON =
        api.symbol(b"cJSON_CreateString\0");
    function(value)
}

unsafe fn add_array(api: &Api, array: *mut cJSON, item: *mut cJSON) -> c_int {
    let function: unsafe extern "C" fn(*mut cJSON, *mut cJSON) -> c_int =
        api.symbol(b"cJSON_AddItemToArray\0");
    function(array, item)
}

unsafe fn add_object(
    api: &Api,
    object: *mut cJSON,
    key: *const c_char,
    item: *mut cJSON,
    constant: bool,
) -> c_int {
    let name = if constant {
        b"cJSON_AddItemToObjectCS\0".as_slice()
    } else {
        b"cJSON_AddItemToObject\0".as_slice()
    };
    let function: unsafe extern "C" fn(*mut cJSON, *const c_char, *mut cJSON) -> c_int =
        api.symbol(name);
    function(object, key, item)
}

unsafe fn construction_trace(api: &Api, seed: u64) -> Vec<String> {
    let mut trace = Vec::new();
    let root = create0(api, b"cJSON_CreateObject\0");
    let array = create0(api, b"cJSON_CreateArray\0");
    assert_ne!(add_object(api, root, c"array".as_ptr(), array, false), 0);

    let mut rng = Rng::new(seed);
    for index in 0..80 {
        let item = match index % 7 {
            0 => create0(api, b"cJSON_CreateNull\0"),
            1 => {
                let function: unsafe extern "C" fn(c_int) -> *mut cJSON =
                    api.symbol(b"cJSON_CreateBool\0");
                function(if rng.next() & 1 == 0 { 0 } else { 9 })
            }
            2 => create_number(api, (rng.next() as i64) as f64 / 17.0),
            3 => {
                let value = CString::new(format!("s{}-\\\"\n", rng.next())).unwrap();
                create_string(api, value.as_ptr())
            }
            4 => {
                let function: unsafe extern "C" fn(*const c_char) -> *mut cJSON =
                    api.symbol(b"cJSON_CreateRaw\0");
                function(c"[9,8]".as_ptr())
            }
            5 => create0(api, b"cJSON_CreateArray\0"),
            _ => create0(api, b"cJSON_CreateObject\0"),
        };
        assert_ne!(add_array(api, array, item), 0);
    }
    trace.push(String::from_utf8(printed(api, root, false).unwrap()).unwrap());

    let insert: unsafe extern "C" fn(*mut cJSON, c_int, *mut cJSON) -> c_int =
        api.symbol(b"cJSON_InsertItemInArray\0");
    for &(index, value) in &[(0, -1.0), (20, -2.0), (80, -3.0), (999, -4.0)] {
        trace.push(insert(array, index, create_number(api, value)).to_string());
    }
    trace.push(String::from_utf8(printed(api, root, false).unwrap()).unwrap());

    let replace: unsafe extern "C" fn(*mut cJSON, c_int, *mut cJSON) -> c_int =
        api.symbol(b"cJSON_ReplaceItemInArray\0");
    for &(index, value) in &[(0, 100.0), (23, 200.0), (82, 300.0)] {
        trace.push(replace(array, index, create_number(api, value)).to_string());
    }

    let detach: unsafe extern "C" fn(*mut cJSON, c_int) -> *mut cJSON =
        api.symbol(b"cJSON_DetachItemFromArray\0");
    for index in [0, 20, 80] {
        let item = detach(array, index);
        trace.push(String::from_utf8(printed(api, item, false).unwrap()).unwrap());
        delete(api, item);
    }

    let add_number: unsafe extern "C" fn(*mut cJSON, *const c_char, f64) -> *mut cJSON =
        api.symbol(b"cJSON_AddNumberToObject\0");
    let add_string: unsafe extern "C" fn(*mut cJSON, *const c_char, *const c_char) -> *mut cJSON =
        api.symbol(b"cJSON_AddStringToObject\0");
    let add_bool: unsafe extern "C" fn(*mut cJSON, *const c_char, c_int) -> *mut cJSON =
        api.symbol(b"cJSON_AddBoolToObject\0");
    let add_simple: [(&[u8], &[u8]); 5] = [
        (b"cJSON_AddNullToObject\0", b"null\0"),
        (b"cJSON_AddTrueToObject\0", b"true\0"),
        (b"cJSON_AddFalseToObject\0", b"false\0"),
        (b"cJSON_AddObjectToObject\0", b"object\0"),
        (b"cJSON_AddArrayToObject\0", b"nested_array\0"),
    ];
    for (function_name, key) in add_simple {
        let function: unsafe extern "C" fn(*mut cJSON, *const c_char) -> *mut cJSON =
            api.symbol(function_name);
        trace.push((!function(root, key.as_ptr().cast()).is_null()).to_string());
    }
    trace.push((!add_number(root, c"number".as_ptr(), 1.0 / 3.0).is_null()).to_string());
    trace.push(
        (!add_string(root, c"string".as_ptr(), c"long value".as_ptr()).is_null()).to_string(),
    );
    trace.push((!add_bool(root, c"bool".as_ptr(), -7).is_null()).to_string());
    let add_raw: unsafe extern "C" fn(*mut cJSON, *const c_char, *const c_char) -> *mut cJSON =
        api.symbol(b"cJSON_AddRawToObject\0");
    trace.push((!add_raw(root, c"raw".as_ptr(), c"{\"x\":1}".as_ptr()).is_null()).to_string());

    let lookup: unsafe extern "C" fn(*const cJSON, *const c_char) -> *mut cJSON =
        api.symbol(b"cJSON_GetObjectItem\0");
    let lookup_cs: unsafe extern "C" fn(*const cJSON, *const c_char) -> *mut cJSON =
        api.symbol(b"cJSON_GetObjectItemCaseSensitive\0");
    let has: unsafe extern "C" fn(*const cJSON, *const c_char) -> c_int =
        api.symbol(b"cJSON_HasObjectItem\0");
    trace.push((!lookup(root, c"NuMbEr".as_ptr()).is_null()).to_string());
    trace.push((!lookup_cs(root, c"NuMbEr".as_ptr()).is_null()).to_string());
    trace.push(has(root, c"NUMBER".as_ptr()).to_string());

    let string_item = lookup(root, c"string".as_ptr());
    let set_string: unsafe extern "C" fn(*mut cJSON, *const c_char) -> *mut c_char =
        api.symbol(b"cJSON_SetValuestring\0");
    for value in [
        c"short".as_ptr(),
        c"short".as_ptr(),
        c"a much longer replacement".as_ptr(),
    ] {
        trace.push((!set_string(string_item, value).is_null()).to_string());
    }

    let duplicate: unsafe extern "C" fn(*const cJSON, c_int) -> *mut cJSON =
        api.symbol(b"cJSON_Duplicate\0");
    let shallow = duplicate(root, 0);
    let deep = duplicate(root, 1);
    trace.push(String::from_utf8(printed(api, shallow, false).unwrap()).unwrap());
    trace.push(String::from_utf8(printed(api, deep, false).unwrap()).unwrap());

    let compare: unsafe extern "C" fn(*const cJSON, *const cJSON, c_int) -> c_int =
        api.symbol(b"cJSON_Compare\0");
    trace.push(compare(root, root, 1).to_string());
    trace.push(compare(root, deep, 1).to_string());
    trace.push(compare(root, deep, 0).to_string());

    let replace_object: unsafe extern "C" fn(*mut cJSON, *const c_char, *mut cJSON) -> c_int =
        api.symbol(b"cJSON_ReplaceItemInObjectCaseSensitive\0");
    trace.push(replace_object(deep, c"number".as_ptr(), create_number(api, 4.0)).to_string());
    trace.push(compare(root, deep, 1).to_string());

    trace.push(String::from_utf8(printed(api, root, true).unwrap()).unwrap());
    trace.push(String::from_utf8(printed(api, root, false).unwrap()).unwrap());

    delete(api, shallow);
    delete(api, deep);
    delete(api, root);
    trace
}

unsafe fn compare_construction(c: &Api, rust: &Api) {
    for seed in 0..40 {
        assert_eq!(
            construction_trace(c, 0x1234_5678 + seed),
            construction_trace(rust, 0x1234_5678 + seed),
            "construction seed={seed}"
        );
    }
}

unsafe fn parse_c_string(api: &Api, value: *const c_char) -> *mut cJSON {
    let function: unsafe extern "C" fn(*const c_char) -> *mut cJSON = api.symbol(b"cJSON_Parse\0");
    function(value)
}

unsafe fn direct_low_level_trace(api: &Api) -> Vec<String> {
    let mut trace = Vec::new();
    let get_item: unsafe extern "C" fn(*const cJSON, c_int) -> *mut cJSON =
        api.symbol(b"cJSON_GetArrayItem\0");
    let get_object: unsafe extern "C" fn(*const cJSON, *const c_char) -> *mut cJSON =
        api.symbol(b"cJSON_GetObjectItem\0");
    let get_object_cs: unsafe extern "C" fn(*const cJSON, *const c_char) -> *mut cJSON =
        api.symbol(b"cJSON_GetObjectItemCaseSensitive\0");

    let constant_key = CString::new("ConstantKey").unwrap();
    let object = create0(api, b"cJSON_CreateObject\0");
    let constant_item = create_number(api, 1.0);
    trace.push(add_object(api, object, constant_key.as_ptr(), constant_item, true).to_string());
    trace.push(((*constant_item).kind & CJSON_STRING_IS_CONST != 0).to_string());
    trace.push(((*constant_item).string == constant_key.as_ptr().cast_mut()).to_string());
    trace.push(String::from_utf8(printed(api, object, false).unwrap()).unwrap());

    let original = create_number(api, 5.0);
    let reference_array = create0(api, b"cJSON_CreateArray\0");
    let reference_object = create0(api, b"cJSON_CreateObject\0");
    let add_reference_array: unsafe extern "C" fn(*mut cJSON, *mut cJSON) -> c_int =
        api.symbol(b"cJSON_AddItemReferenceToArray\0");
    let add_reference_object: unsafe extern "C" fn(*mut cJSON, *const c_char, *mut cJSON) -> c_int =
        api.symbol(b"cJSON_AddItemReferenceToObject\0");
    trace.push(add_reference_array(reference_array, original).to_string());
    trace.push(add_reference_object(reference_object, c"ref".as_ptr(), original).to_string());
    let set_number: unsafe extern "C" fn(*mut cJSON, c_double) -> c_double =
        api.symbol(b"cJSON_SetNumberHelper\0");
    set_number(original, 9.25);
    trace.push(String::from_utf8(printed(api, reference_array, false).unwrap()).unwrap());
    trace.push(String::from_utf8(printed(api, reference_object, false).unwrap()).unwrap());
    delete(api, reference_array);
    delete(api, reference_object);
    delete(api, original);

    let detach_pointer: unsafe extern "C" fn(*mut cJSON, *mut cJSON) -> *mut cJSON =
        api.symbol(b"cJSON_DetachItemViaPointer\0");
    for target in [0, 1, 3] {
        let array = parse_c_string(api, c"[0,1,2,3]".as_ptr());
        let detached = detach_pointer(array, get_item(array, target));
        trace.push(String::from_utf8(printed(api, detached, false).unwrap()).unwrap());
        trace.push(String::from_utf8(printed(api, array, false).unwrap()).unwrap());
        delete(api, detached);
        delete(api, array);
    }
    let sole_array = parse_c_string(api, c"[7]".as_ptr());
    let sole = detach_pointer(sole_array, get_item(sole_array, 0));
    trace.push(String::from_utf8(printed(api, sole_array, false).unwrap()).unwrap());
    delete(api, sole);
    delete(api, sole_array);

    let replace_pointer: unsafe extern "C" fn(*mut cJSON, *mut cJSON, *mut cJSON) -> c_int =
        api.symbol(b"cJSON_ReplaceItemViaPointer\0");
    for target in [0, 1, 3] {
        let array = parse_c_string(api, c"[0,1,2,3]".as_ptr());
        let old = get_item(array, target);
        trace.push(replace_pointer(array, old, old).to_string());
        trace.push(replace_pointer(array, old, create_number(api, 99.0)).to_string());
        trace.push(String::from_utf8(printed(api, array, false).unwrap()).unwrap());
        delete(api, array);
    }

    let delete_array: unsafe extern "C" fn(*mut cJSON, c_int) =
        api.symbol(b"cJSON_DeleteItemFromArray\0");
    let array = parse_c_string(api, c"[0,1,2]".as_ptr());
    delete_array(array, 1);
    trace.push(String::from_utf8(printed(api, array, false).unwrap()).unwrap());
    delete_array(array, -1);
    delete_array(array, 99);
    delete(api, array);

    let detach_object: unsafe extern "C" fn(*mut cJSON, *const c_char) -> *mut cJSON =
        api.symbol(b"cJSON_DetachItemFromObject\0");
    let detach_object_cs: unsafe extern "C" fn(*mut cJSON, *const c_char) -> *mut cJSON =
        api.symbol(b"cJSON_DetachItemFromObjectCaseSensitive\0");
    let object_detach = parse_c_string(api, c"{\"Alpha\":1,\"beta\":2,\"Gamma\":3}".as_ptr());
    let detached = detach_object(object_detach, c"ALPHA".as_ptr());
    trace.push(String::from_utf8(printed(api, detached, false).unwrap()).unwrap());
    delete(api, detached);
    let detached = detach_object_cs(object_detach, c"Gamma".as_ptr());
    trace.push(String::from_utf8(printed(api, detached, false).unwrap()).unwrap());
    delete(api, detached);
    trace.push(String::from_utf8(printed(api, object_detach, false).unwrap()).unwrap());
    delete(api, object_detach);

    let delete_object: unsafe extern "C" fn(*mut cJSON, *const c_char) =
        api.symbol(b"cJSON_DeleteItemFromObject\0");
    let delete_object_cs: unsafe extern "C" fn(*mut cJSON, *const c_char) =
        api.symbol(b"cJSON_DeleteItemFromObjectCaseSensitive\0");
    let object_delete = parse_c_string(api, c"{\"Alpha\":1,\"beta\":2,\"Gamma\":3}".as_ptr());
    delete_object(object_delete, c"ALPHA".as_ptr());
    delete_object_cs(object_delete, c"gamma".as_ptr());
    trace.push(String::from_utf8(printed(api, object_delete, false).unwrap()).unwrap());
    delete_object_cs(object_delete, c"Gamma".as_ptr());
    trace.push(String::from_utf8(printed(api, object_delete, false).unwrap()).unwrap());
    delete(api, object_delete);

    let replace_object: unsafe extern "C" fn(*mut cJSON, *const c_char, *mut cJSON) -> c_int =
        api.symbol(b"cJSON_ReplaceItemInObject\0");
    let replace_object_cs: unsafe extern "C" fn(*mut cJSON, *const c_char, *mut cJSON) -> c_int =
        api.symbol(b"cJSON_ReplaceItemInObjectCaseSensitive\0");
    let object_replace = parse_c_string(api, c"{\"Alpha\":1,\"beta\":2}".as_ptr());
    trace.push(
        replace_object(object_replace, c"ALPHA".as_ptr(), create_number(api, 10.0)).to_string(),
    );
    trace.push(
        replace_object_cs(object_replace, c"beta".as_ptr(), create_number(api, 20.0)).to_string(),
    );
    trace.push(String::from_utf8(printed(api, object_replace, false).unwrap()).unwrap());
    trace.push((!get_object(object_replace, c"alpha".as_ptr()).is_null()).to_string());
    trace.push((!get_object_cs(object_replace, c"Alpha".as_ptr()).is_null()).to_string());
    delete(api, object_replace);

    let compare: unsafe extern "C" fn(*const cJSON, *const cJSON, c_int) -> c_int =
        api.symbol(b"cJSON_Compare\0");
    let compare_cases = [
        (
            c"{\"A\":1,\"b\":[2,3]}".as_ptr(),
            c"{\"b\":[2,3],\"A\":1}".as_ptr(),
        ),
        (c"{\"A\":1}".as_ptr(), c"{\"a\":1}".as_ptr()),
        (c"[1,2]".as_ptr(), c"[1,2,3]".as_ptr()),
        (c"[1,2]".as_ptr(), c"[1,9]".as_ptr()),
        (c"\"x\"".as_ptr(), c"\"y\"".as_ptr()),
    ];
    for (left_text, right_text) in compare_cases {
        let left = parse_c_string(api, left_text);
        let right = parse_c_string(api, right_text);
        trace.push(compare(left, right, 0).to_string());
        trace.push(compare(left, right, 1).to_string());
        delete(api, left);
        delete(api, right);
    }

    delete(api, object);
    trace
}

unsafe fn compare_direct_low_level(c: &Api, rust: &Api) {
    assert_eq!(direct_low_level_trace(c), direct_low_level_trace(rust));
}

unsafe fn array_and_type_trace(api: &Api, seed: u64) -> Vec<String> {
    let mut trace = Vec::new();
    let mut rng = Rng::new(seed);
    let mut ints = [0i32; 32];
    let mut floats = [0f32; 32];
    let mut doubles = [0f64; 32];
    for index in 0..32 {
        ints[index] = rng.next() as i32;
        floats[index] = (rng.next() as i32) as f32 / 31.0;
        doubles[index] = (rng.next() as i64) as f64 / 127.0;
    }
    ints[0] = i32::MIN;
    ints[1] = i32::MAX;
    floats[0] = -0.0;
    floats[1] = f32::INFINITY;
    doubles[0] = f64::NAN;
    doubles[1] = f64::NEG_INFINITY;

    for count in [0, 1, 2, 17, 32] {
        let int_function: unsafe extern "C" fn(*const c_int, c_int) -> *mut cJSON =
            api.symbol(b"cJSON_CreateIntArray\0");
        let float_function: unsafe extern "C" fn(*const c_float, c_int) -> *mut cJSON =
            api.symbol(b"cJSON_CreateFloatArray\0");
        let double_function: unsafe extern "C" fn(*const c_double, c_int) -> *mut cJSON =
            api.symbol(b"cJSON_CreateDoubleArray\0");
        for item in [
            int_function(ints.as_ptr(), count),
            float_function(floats.as_ptr(), count),
            double_function(doubles.as_ptr(), count),
        ] {
            trace.push(String::from_utf8(printed(api, item, false).unwrap()).unwrap());
            delete(api, item);
        }
    }

    let strings: Vec<CString> = (0..24)
        .map(|index| CString::new(format!("value-{index}-{}", rng.next())).unwrap())
        .collect();
    let pointers: Vec<*const c_char> = strings.iter().map(|value| value.as_ptr()).collect();
    let string_array: unsafe extern "C" fn(*const *const c_char, c_int) -> *mut cJSON =
        api.symbol(b"cJSON_CreateStringArray\0");
    for count in [0, 1, 13, 24] {
        let item = string_array(pointers.as_ptr(), count);
        trace.push(String::from_utf8(printed(api, item, false).unwrap()).unwrap());
        delete(api, item);
    }

    let scalar_items = [
        create0(api, b"cJSON_CreateNull\0"),
        create0(api, b"cJSON_CreateFalse\0"),
        create0(api, b"cJSON_CreateTrue\0"),
        create_number(api, 7.5),
        create_string(api, c"x".as_ptr()),
        {
            let function: unsafe extern "C" fn(*const c_char) -> *mut cJSON =
                api.symbol(b"cJSON_CreateRaw\0");
            function(c"[1]".as_ptr())
        },
        create0(api, b"cJSON_CreateArray\0"),
        create0(api, b"cJSON_CreateObject\0"),
    ];
    let predicates = [
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
    for &item in &scalar_items {
        let mut row = String::new();
        for predicate_name in predicates {
            let predicate: unsafe extern "C" fn(*const cJSON) -> c_int = api.symbol(predicate_name);
            row.push(char::from(b'0' + predicate(item) as u8));
        }
        trace.push(row);
    }

    let number_value: unsafe extern "C" fn(*const cJSON) -> c_double =
        api.symbol(b"cJSON_GetNumberValue\0");
    let string_value: unsafe extern "C" fn(*const cJSON) -> *mut c_char =
        api.symbol(b"cJSON_GetStringValue\0");
    trace.push(format!("{:.17}", number_value(scalar_items[3])));
    trace.push(
        CStr::from_ptr(string_value(scalar_items[4]))
            .to_string_lossy()
            .into_owned(),
    );

    let set_number: unsafe extern "C" fn(*mut cJSON, c_double) -> c_double =
        api.symbol(b"cJSON_SetNumberHelper\0");
    for value in [
        f64::NEG_INFINITY,
        i32::MIN as f64 - 1.0,
        i32::MIN as f64,
        -0.0,
        i32::MAX as f64,
        i32::MAX as f64 + 1.0,
        f64::INFINITY,
        f64::NAN,
    ] {
        let returned = set_number(scalar_items[3], value);
        trace.push(format!(
            "{}:{}:{}",
            returned.to_bits(),
            (*scalar_items[3]).valueint,
            (*scalar_items[3]).valuedouble.to_bits()
        ));
    }

    let string_ref_function: unsafe extern "C" fn(*const c_char) -> *mut cJSON =
        api.symbol(b"cJSON_CreateStringReference\0");
    let string_ref = string_ref_function(c"borrowed".as_ptr());
    let array_ref_function: unsafe extern "C" fn(*const cJSON) -> *mut cJSON =
        api.symbol(b"cJSON_CreateArrayReference\0");
    let object_ref_function: unsafe extern "C" fn(*const cJSON) -> *mut cJSON =
        api.symbol(b"cJSON_CreateObjectReference\0");
    let array_ref = array_ref_function((*scalar_items[6]).child);
    let object_ref = object_ref_function((*scalar_items[7]).child);
    trace.push(format!(
        "{}:{}:{}",
        (*string_ref).kind,
        (*array_ref).kind,
        (*object_ref).kind
    ));
    trace.push(String::from_utf8(printed(api, string_ref, false).unwrap()).unwrap());

    delete(api, string_ref);
    delete(api, array_ref);
    delete(api, object_ref);
    for item in scalar_items {
        delete(api, item);
    }
    trace
}

unsafe fn compare_arrays_and_types(c: &Api, rust: &Api) {
    for seed in 0..80 {
        assert_eq!(
            array_and_type_trace(c, 0xa11a_0000 + seed),
            array_and_type_trace(rust, 0xa11a_0000 + seed),
            "array/type seed={seed}"
        );
    }
}

unsafe fn print_mode_trace(api: &Api) -> Vec<String> {
    let mut trace = Vec::new();
    let item = {
        let parse: unsafe extern "C" fn(*const c_char) -> *mut cJSON = api.symbol(b"cJSON_Parse\0");
        parse(c"{\"s\":\"a\\nb\\t\\\"c\",\"a\":[1,2.5,true,null],\"o\":{\"x\":3}}".as_ptr())
    };
    let expected_formatted = printed(api, item, true).unwrap();
    let expected_compact = printed(api, item, false).unwrap();
    trace.push(String::from_utf8(expected_formatted.clone()).unwrap());
    trace.push(String::from_utf8(expected_compact.clone()).unwrap());

    let buffered: unsafe extern "C" fn(*const cJSON, c_int, c_int) -> *mut c_char =
        api.symbol(b"cJSON_PrintBuffered\0");
    for prebuffer in [0, 1, 8, 64, 256, 1024] {
        for format in [0, 1, -7] {
            let output = buffered(item, prebuffer, format);
            if output.is_null() {
                trace.push("<null>".into());
            } else {
                trace.push(CStr::from_ptr(output).to_string_lossy().into_owned());
                cjson_free(api, output.cast());
            }
        }
    }

    let preallocated: unsafe extern "C" fn(*mut cJSON, *mut c_char, c_int, c_int) -> c_int =
        api.symbol(b"cJSON_PrintPreallocated\0");
    for format in [0, 1, 9] {
        for extra in [0usize, 1, 5, 64] {
            let expected = if format == 0 {
                &expected_compact
            } else {
                &expected_formatted
            };
            let length = expected.len() + 1 + extra;
            let mut output = vec![0x5au8; length];
            let ok = preallocated(item, output.as_mut_ptr().cast(), length as c_int, format);
            let bytes = if ok == 0 {
                Vec::new()
            } else {
                CStr::from_ptr(output.as_ptr().cast()).to_bytes().to_vec()
            };
            trace.push(format!("{ok}:{}", String::from_utf8_lossy(&bytes)));
        }
    }

    for too_short in [0usize, 1, expected_formatted.len().saturating_sub(1)] {
        let mut output = vec![0u8; too_short.max(1)];
        trace.push(
            preallocated(item, output.as_mut_ptr().cast(), too_short as c_int, 1).to_string(),
        );
    }
    let mut oversized_claim = vec![0u8; expected_compact.len() + 1];
    trace.push(preallocated(item, oversized_claim.as_mut_ptr().cast(), c_int::MAX, 0).to_string());
    trace.push(
        CStr::from_ptr(oversized_claim.as_ptr().cast())
            .to_string_lossy()
            .into_owned(),
    );
    delete(api, item);
    trace
}

unsafe fn minify_trace(api: &Api) -> Vec<String> {
    let function: unsafe extern "C" fn(*mut c_char) = api.symbol(b"cJSON_Minify\0");
    let cases = [
        " \t\r\n { \"x\" : [ 1, 2 ] } ",
        "/* leading */{\"x\":1}// trailing\n",
        "{\"s\":\"a b // c /* d */\",\"q\":\"a\\\\\\\" b\"}",
        "/not-comment / * x",
        "/* unterminated",
        "// unterminated",
    ];
    let mut trace = Vec::new();
    for case in cases {
        let mut bytes = case.as_bytes().to_vec();
        bytes.push(0);
        function(bytes.as_mut_ptr().cast());
        trace.push(
            CStr::from_ptr(bytes.as_ptr().cast())
                .to_string_lossy()
                .into_owned(),
        );
    }
    function(ptr::null_mut());
    trace
}

unsafe fn compare_print_and_minify(c: &Api, rust: &Api) {
    assert_eq!(print_mode_trace(c), print_mode_trace(rust));
    assert_eq!(minify_trace(c), minify_trace(rust));
}

static ALLOCATIONS: AtomicUsize = AtomicUsize::new(0);
static FREES: AtomicUsize = AtomicUsize::new(0);
static FAIL_AT: AtomicIsize = AtomicIsize::new(-1);

unsafe extern "C" fn controlled_malloc(size: usize) -> *mut c_void {
    let allocation = ALLOCATIONS.fetch_add(1, Ordering::SeqCst) + 1;
    if FAIL_AT.load(Ordering::SeqCst) == allocation as isize {
        ptr::null_mut()
    } else {
        malloc(size)
    }
}

unsafe extern "C" fn controlled_free(pointer: *mut c_void) {
    FREES.fetch_add(1, Ordering::SeqCst);
    free(pointer);
}

unsafe fn set_hooks(api: &Api, fail_at: isize) {
    ALLOCATIONS.store(0, Ordering::SeqCst);
    FREES.store(0, Ordering::SeqCst);
    FAIL_AT.store(fail_at, Ordering::SeqCst);
    let function: unsafe extern "C" fn(*mut cJSON_Hooks) = api.symbol(b"cJSON_InitHooks\0");
    let mut hooks = cJSON_Hooks {
        malloc_fn: Some(controlled_malloc),
        free_fn: Some(controlled_free),
    };
    function(&mut hooks);
}

unsafe fn reset_hooks(api: &Api) {
    let function: unsafe extern "C" fn(*mut cJSON_Hooks) = api.symbol(b"cJSON_InitHooks\0");
    function(ptr::null_mut());
    FAIL_AT.store(-1, Ordering::SeqCst);
}

unsafe fn allocation_failure_trace(api: &Api, fail_at: isize) -> Vec<String> {
    set_hooks(api, fail_at);
    let mut trace = Vec::new();
    let parse: unsafe extern "C" fn(*const c_char) -> *mut cJSON = api.symbol(b"cJSON_Parse\0");
    let item = parse(c"{\"a\":[1,2,3],\"s\":\"long string\"}".as_ptr());
    trace.push((!item.is_null()).to_string());
    if !item.is_null() {
        let output = printed(api, item, false);
        trace.push(
            output
                .map(|bytes| String::from_utf8(bytes).unwrap())
                .unwrap_or_else(|| "<null>".into()),
        );
        let duplicate: unsafe extern "C" fn(*const cJSON, c_int) -> *mut cJSON =
            api.symbol(b"cJSON_Duplicate\0");
        let copy = duplicate(item, 1);
        trace.push((!copy.is_null()).to_string());
        delete(api, copy);
    }
    delete(api, item);
    trace.push(ALLOCATIONS.load(Ordering::SeqCst).to_string());
    trace.push(FREES.load(Ordering::SeqCst).to_string());
    reset_hooks(api);
    trace
}

unsafe fn compare_allocation_failures(c: &Api, rust: &Api) {
    for fail_at in -1..40 {
        assert_eq!(
            allocation_failure_trace(c, fail_at),
            allocation_failure_trace(rust, fail_at),
            "allocation fail_at={fail_at}"
        );
    }
}

unsafe fn targeted_failure_trace(api: &Api) -> Vec<String> {
    let mut trace = Vec::new();

    let string = create_string(api, c"old".as_ptr());
    set_hooks(api, 1);
    let set_string: unsafe extern "C" fn(*mut cJSON, *const c_char) -> *mut c_char =
        api.symbol(b"cJSON_SetValuestring\0");
    trace.push(
        set_string(string, c"this replacement must allocate".as_ptr())
            .is_null()
            .to_string(),
    );
    trace.push(
        CStr::from_ptr((*string).valuestring)
            .to_string_lossy()
            .into_owned(),
    );
    reset_hooks(api);
    delete(api, string);

    let object = create0(api, b"cJSON_CreateObject\0");
    let item = create_number(api, 1.0);
    set_hooks(api, 1);
    trace.push(add_object(api, object, c"copied".as_ptr(), item, false).to_string());
    reset_hooks(api);
    delete(api, item);
    delete(api, object);

    for constructor in [
        b"cJSON_CreateNull\0".as_slice(),
        b"cJSON_CreateTrue\0".as_slice(),
        b"cJSON_CreateFalse\0".as_slice(),
        b"cJSON_CreateArray\0".as_slice(),
        b"cJSON_CreateObject\0".as_slice(),
    ] {
        set_hooks(api, 1);
        trace.push(create0(api, constructor).is_null().to_string());
        reset_hooks(api);
    }
    set_hooks(api, 1);
    trace.push(create_number(api, 1.0).is_null().to_string());
    reset_hooks(api);
    set_hooks(api, 1);
    trace.push(create_string(api, c"x".as_ptr()).is_null().to_string());
    reset_hooks(api);
    let create_raw: unsafe extern "C" fn(*const c_char) -> *mut cJSON =
        api.symbol(b"cJSON_CreateRaw\0");
    set_hooks(api, 1);
    trace.push(create_raw(c"x".as_ptr()).is_null().to_string());
    reset_hooks(api);

    let ints = [1, 2, 3];
    let int_array: unsafe extern "C" fn(*const c_int, c_int) -> *mut cJSON =
        api.symbol(b"cJSON_CreateIntArray\0");
    for fail_at in 1..=5 {
        set_hooks(api, fail_at);
        let array = int_array(ints.as_ptr(), ints.len() as c_int);
        trace.push((!array.is_null()).to_string());
        delete(api, array);
        reset_hooks(api);
    }
    let strings = [c"a".as_ptr(), c"b".as_ptr(), c"c".as_ptr()];
    let string_array: unsafe extern "C" fn(*const *const c_char, c_int) -> *mut cJSON =
        api.symbol(b"cJSON_CreateStringArray\0");
    for fail_at in 1..=8 {
        set_hooks(api, fail_at);
        let array = string_array(strings.as_ptr(), strings.len() as c_int);
        trace.push((!array.is_null()).to_string());
        delete(api, array);
        reset_hooks(api);
    }

    let printable = parse_c_string(api, c"{\"long\":\"value\",\"array\":[1,2,3]}".as_ptr());
    let print: unsafe extern "C" fn(*const cJSON) -> *mut c_char = api.symbol(b"cJSON_Print\0");
    let print_buffered: unsafe extern "C" fn(*const cJSON, c_int, c_int) -> *mut c_char =
        api.symbol(b"cJSON_PrintBuffered\0");
    set_hooks(api, 1);
    trace.push(print(printable).is_null().to_string());
    reset_hooks(api);
    set_hooks(api, 1);
    trace.push(print_buffered(printable, 0, 0).is_null().to_string());
    reset_hooks(api);
    set_hooks(api, 2);
    let output = print_buffered(printable, 1, 0);
    trace.push(output.is_null().to_string());
    cjson_free(api, output.cast());
    reset_hooks(api);
    delete(api, printable);

    let corrupt_array = parse_c_string(api, c"[1,2]".as_ptr());
    let first = (*corrupt_array).child;
    let second = (*first).next;
    (*second).prev = ptr::null_mut();
    let insert: unsafe extern "C" fn(*mut cJSON, c_int, *mut cJSON) -> c_int =
        api.symbol(b"cJSON_InsertItemInArray\0");
    let new_item = create_number(api, 3.0);
    trace.push(insert(corrupt_array, 1, new_item).to_string());
    let detach: unsafe extern "C" fn(*mut cJSON, *mut cJSON) -> *mut cJSON =
        api.symbol(b"cJSON_DetachItemViaPointer\0");
    trace.push(detach(corrupt_array, second).is_null().to_string());
    (*second).prev = first;
    delete(api, new_item);
    delete(api, corrupt_array);

    trace
}

unsafe fn compare_targeted_failures(c: &Api, rust: &Api) {
    assert_eq!(targeted_failure_trace(c), targeted_failure_trace(rust));
}

unsafe fn null_and_error_trace(api: &Api) -> Vec<String> {
    let mut trace = Vec::new();

    let parse_null: unsafe extern "C" fn(*const c_char) -> *mut cJSON =
        api.symbol(b"cJSON_Parse\0");
    let parse_opts: unsafe extern "C" fn(*const c_char, *mut *const c_char, c_int) -> *mut cJSON =
        api.symbol(b"cJSON_ParseWithOpts\0");
    let parse_length: unsafe extern "C" fn(*const c_char, usize) -> *mut cJSON =
        api.symbol(b"cJSON_ParseWithLength\0");
    let parse_length_opts: unsafe extern "C" fn(
        *const c_char,
        usize,
        *mut *const c_char,
        c_int,
    ) -> *mut cJSON = api.symbol(b"cJSON_ParseWithLengthOpts\0");
    let mut end = 1usize as *const c_char;
    trace.push(parse_null(ptr::null()).is_null().to_string());
    trace.push(parse_opts(ptr::null(), &mut end, 1).is_null().to_string());
    trace.push((end as usize).to_string());
    trace.push(parse_length(ptr::null(), 10).is_null().to_string());
    end = 1usize as *const c_char;
    trace.push(
        parse_length_opts(ptr::null(), 10, &mut end, 1)
            .is_null()
            .to_string(),
    );
    trace.push((end as usize).to_string());
    end = ptr::null();
    trace.push(
        parse_length_opts(c"null".as_ptr(), 0, &mut end, 0)
            .is_null()
            .to_string(),
    );
    trace.push(
        if end.is_null() {
            -1
        } else {
            end.offset_from(c"null".as_ptr())
        }
        .to_string(),
    );

    let invalid = [
        "",
        "x",
        "-",
        "\"",
        "\"abc\\",
        "\"abc",
        "\"\\q\"",
        "\"\\u\"",
        "\"\\udc00\"",
        "\"\\ud800\"",
        "\"\\ud800x0000\"",
        "\"\\ud800\\u0041\"",
        "[",
        "[ ",
        "[1,",
        "[1",
        "[1}",
        "{",
        "{ ",
        "{\"a\"",
        "{\"a\":",
        "{\"a\":1",
        "{\"a\":1]",
        "tru",
        "nul",
        "/*x*/1",
        "1 trailing",
    ];
    for input in invalid {
        let bytes = input.as_bytes();
        for &(variant, require) in &[(0, 0), (2, 1), (3, 0), (3, 1)] {
            let result = parse(api, bytes, variant, bytes.len() + 1, require);
            trace.push(format!(
                "{input:?}:{variant}:{require}:{:?}:{:?}:{:?}",
                result.ok, result.end_offset, result.error_offset
            ));
        }
    }

    let deep_array = format!("{}0{}", "[".repeat(1001), "]".repeat(1001));
    let deep_object = format!("{}0{}", "{\"x\":".repeat(1001), "}".repeat(1001));
    for deep in [deep_array, deep_object] {
        let result = parse(api, deep.as_bytes(), 3, deep.len() + 1, 1);
        trace.push(format!("deep:{:?}:{:?}", result.ok, result.error_offset));
    }

    let print: unsafe extern "C" fn(*const cJSON) -> *mut c_char = api.symbol(b"cJSON_Print\0");
    let print_unformatted: unsafe extern "C" fn(*const cJSON) -> *mut c_char =
        api.symbol(b"cJSON_PrintUnformatted\0");
    let print_buffered: unsafe extern "C" fn(*const cJSON, c_int, c_int) -> *mut c_char =
        api.symbol(b"cJSON_PrintBuffered\0");
    let print_preallocated: unsafe extern "C" fn(*mut cJSON, *mut c_char, c_int, c_int) -> c_int =
        api.symbol(b"cJSON_PrintPreallocated\0");
    trace.push(print(ptr::null()).is_null().to_string());
    trace.push(print_unformatted(ptr::null()).is_null().to_string());
    trace.push(print_buffered(ptr::null(), 32, 0).is_null().to_string());
    trace.push(print_buffered(ptr::null(), -1, 0).is_null().to_string());
    let mut tiny = [0i8; 2];
    trace.push(print_preallocated(ptr::null_mut(), tiny.as_mut_ptr(), 2, 0).to_string());
    trace.push(print_preallocated(ptr::null_mut(), ptr::null_mut(), 2, 0).to_string());
    trace.push(print_preallocated(ptr::null_mut(), tiny.as_mut_ptr(), -1, 0).to_string());

    let invalid_item = create0(api, b"cJSON_CreateNull\0");
    (*invalid_item).kind = 0x7f;
    trace.push(print(invalid_item).is_null().to_string());
    trace.push(print_buffered(invalid_item, 16, 0).is_null().to_string());
    trace.push(print_preallocated(invalid_item, tiny.as_mut_ptr(), 2, 0).to_string());
    let raw = create0(api, b"cJSON_CreateNull\0");
    (*raw).kind = CJSON_RAW;
    trace.push(print(raw).is_null().to_string());

    let get_size: unsafe extern "C" fn(*const cJSON) -> c_int = api.symbol(b"cJSON_GetArraySize\0");
    let get_item: unsafe extern "C" fn(*const cJSON, c_int) -> *mut cJSON =
        api.symbol(b"cJSON_GetArrayItem\0");
    trace.push(get_size(ptr::null()).to_string());
    trace.push(get_item(ptr::null(), 0).is_null().to_string());
    let empty_array = create0(api, b"cJSON_CreateArray\0");
    trace.push(get_item(empty_array, -1).is_null().to_string());
    trace.push(get_item(empty_array, 0).is_null().to_string());
    trace.push(get_item(empty_array, c_int::MAX).is_null().to_string());

    let get_object: unsafe extern "C" fn(*const cJSON, *const c_char) -> *mut cJSON =
        api.symbol(b"cJSON_GetObjectItem\0");
    let get_object_cs: unsafe extern "C" fn(*const cJSON, *const c_char) -> *mut cJSON =
        api.symbol(b"cJSON_GetObjectItemCaseSensitive\0");
    let has: unsafe extern "C" fn(*const cJSON, *const c_char) -> c_int =
        api.symbol(b"cJSON_HasObjectItem\0");
    trace.push(get_object(ptr::null(), c"x".as_ptr()).is_null().to_string());
    trace.push(get_object(empty_array, ptr::null()).is_null().to_string());
    trace.push(
        get_object_cs(empty_array, c"x".as_ptr())
            .is_null()
            .to_string(),
    );
    trace.push(has(ptr::null(), ptr::null()).to_string());

    let get_string: unsafe extern "C" fn(*const cJSON) -> *mut c_char =
        api.symbol(b"cJSON_GetStringValue\0");
    let get_number: unsafe extern "C" fn(*const cJSON) -> c_double =
        api.symbol(b"cJSON_GetNumberValue\0");
    trace.push(get_string(ptr::null()).is_null().to_string());
    trace.push(get_string(empty_array).is_null().to_string());
    trace.push(get_number(ptr::null()).to_bits().to_string());
    trace.push(get_number(empty_array).to_bits().to_string());

    for predicate_name in [
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
    ] {
        let predicate: unsafe extern "C" fn(*const cJSON) -> c_int = api.symbol(predicate_name);
        trace.push(predicate(ptr::null()).to_string());
    }

    let string = create_string(api, c"old value".as_ptr());
    let set_string: unsafe extern "C" fn(*mut cJSON, *const c_char) -> *mut c_char =
        api.symbol(b"cJSON_SetValuestring\0");
    trace.push(
        set_string(ptr::null_mut(), c"x".as_ptr())
            .is_null()
            .to_string(),
    );
    trace.push(set_string(empty_array, c"x".as_ptr()).is_null().to_string());
    (*string).kind |= CJSON_IS_REFERENCE;
    trace.push(set_string(string, c"x".as_ptr()).is_null().to_string());
    (*string).kind &= !CJSON_IS_REFERENCE;
    let old_value = (*string).valuestring;
    (*string).valuestring = ptr::null_mut();
    trace.push(set_string(string, c"x".as_ptr()).is_null().to_string());
    (*string).valuestring = old_value;
    trace.push(set_string(string, ptr::null()).is_null().to_string());
    trace.push(set_string(string, old_value).is_null().to_string());

    let add_to_array: unsafe extern "C" fn(*mut cJSON, *mut cJSON) -> c_int =
        api.symbol(b"cJSON_AddItemToArray\0");
    trace.push(add_to_array(ptr::null_mut(), ptr::null_mut()).to_string());
    trace.push(add_to_array(empty_array, ptr::null_mut()).to_string());
    trace.push(add_to_array(empty_array, empty_array).to_string());

    for add_name in [
        b"cJSON_AddItemToObject\0".as_slice(),
        b"cJSON_AddItemToObjectCS\0".as_slice(),
    ] {
        let add: unsafe extern "C" fn(*mut cJSON, *const c_char, *mut cJSON) -> c_int =
            api.symbol(add_name);
        trace.push(add(ptr::null_mut(), c"x".as_ptr(), empty_array).to_string());
        trace.push(add(empty_array, ptr::null(), invalid_item).to_string());
        trace.push(add(empty_array, c"x".as_ptr(), ptr::null_mut()).to_string());
        trace.push(add(empty_array, c"x".as_ptr(), empty_array).to_string());
    }

    let add_ref_array: unsafe extern "C" fn(*mut cJSON, *mut cJSON) -> c_int =
        api.symbol(b"cJSON_AddItemReferenceToArray\0");
    let add_ref_object: unsafe extern "C" fn(*mut cJSON, *const c_char, *mut cJSON) -> c_int =
        api.symbol(b"cJSON_AddItemReferenceToObject\0");
    trace.push(add_ref_array(ptr::null_mut(), invalid_item).to_string());
    trace.push(add_ref_array(empty_array, ptr::null_mut()).to_string());
    trace.push(add_ref_object(ptr::null_mut(), c"x".as_ptr(), invalid_item).to_string());
    trace.push(add_ref_object(empty_array, ptr::null(), invalid_item).to_string());
    trace.push(add_ref_object(empty_array, c"x".as_ptr(), ptr::null_mut()).to_string());

    let object = create0(api, b"cJSON_CreateObject\0");
    for add_name in [
        b"cJSON_AddNullToObject\0".as_slice(),
        b"cJSON_AddTrueToObject\0".as_slice(),
        b"cJSON_AddFalseToObject\0".as_slice(),
        b"cJSON_AddObjectToObject\0".as_slice(),
        b"cJSON_AddArrayToObject\0".as_slice(),
    ] {
        let add: unsafe extern "C" fn(*mut cJSON, *const c_char) -> *mut cJSON =
            api.symbol(add_name);
        trace.push(add(ptr::null_mut(), c"x".as_ptr()).is_null().to_string());
        trace.push(add(object, ptr::null()).is_null().to_string());
    }
    let add_bool: unsafe extern "C" fn(*mut cJSON, *const c_char, c_int) -> *mut cJSON =
        api.symbol(b"cJSON_AddBoolToObject\0");
    let add_number: unsafe extern "C" fn(*mut cJSON, *const c_char, f64) -> *mut cJSON =
        api.symbol(b"cJSON_AddNumberToObject\0");
    let add_text_names = [
        b"cJSON_AddStringToObject\0".as_slice(),
        b"cJSON_AddRawToObject\0".as_slice(),
    ];
    trace.push(
        add_bool(ptr::null_mut(), c"x".as_ptr(), 1)
            .is_null()
            .to_string(),
    );
    trace.push(add_bool(object, ptr::null(), 1).is_null().to_string());
    trace.push(
        add_number(ptr::null_mut(), c"x".as_ptr(), 1.0)
            .is_null()
            .to_string(),
    );
    trace.push(add_number(object, ptr::null(), 1.0).is_null().to_string());
    for add_name in add_text_names {
        let add: unsafe extern "C" fn(*mut cJSON, *const c_char, *const c_char) -> *mut cJSON =
            api.symbol(add_name);
        trace.push(
            add(ptr::null_mut(), c"x".as_ptr(), c"v".as_ptr())
                .is_null()
                .to_string(),
        );
        trace.push(
            add(object, ptr::null(), c"v".as_ptr())
                .is_null()
                .to_string(),
        );
        trace.push(
            add(object, c"x".as_ptr(), ptr::null())
                .is_null()
                .to_string(),
        );
    }

    let detach_pointer: unsafe extern "C" fn(*mut cJSON, *mut cJSON) -> *mut cJSON =
        api.symbol(b"cJSON_DetachItemViaPointer\0");
    let detach_array: unsafe extern "C" fn(*mut cJSON, c_int) -> *mut cJSON =
        api.symbol(b"cJSON_DetachItemFromArray\0");
    trace.push(
        detach_pointer(ptr::null_mut(), invalid_item)
            .is_null()
            .to_string(),
    );
    trace.push(
        detach_pointer(empty_array, ptr::null_mut())
            .is_null()
            .to_string(),
    );
    trace.push(
        detach_pointer(empty_array, invalid_item)
            .is_null()
            .to_string(),
    );
    trace.push(detach_array(empty_array, -1).is_null().to_string());
    trace.push(detach_array(empty_array, 0).is_null().to_string());

    let detach_object: unsafe extern "C" fn(*mut cJSON, *const c_char) -> *mut cJSON =
        api.symbol(b"cJSON_DetachItemFromObject\0");
    let detach_object_cs: unsafe extern "C" fn(*mut cJSON, *const c_char) -> *mut cJSON =
        api.symbol(b"cJSON_DetachItemFromObjectCaseSensitive\0");
    trace.push(detach_object(object, ptr::null()).is_null().to_string());
    trace.push(
        detach_object(object, c"absent".as_ptr())
            .is_null()
            .to_string(),
    );
    trace.push(
        detach_object_cs(object, c"absent".as_ptr())
            .is_null()
            .to_string(),
    );

    let insert: unsafe extern "C" fn(*mut cJSON, c_int, *mut cJSON) -> c_int =
        api.symbol(b"cJSON_InsertItemInArray\0");
    trace.push(insert(empty_array, -1, invalid_item).to_string());
    trace.push(insert(empty_array, 0, ptr::null_mut()).to_string());

    let replace_pointer: unsafe extern "C" fn(*mut cJSON, *mut cJSON, *mut cJSON) -> c_int =
        api.symbol(b"cJSON_ReplaceItemViaPointer\0");
    let replacement = create_number(api, 1.0);
    trace.push(replace_pointer(ptr::null_mut(), invalid_item, replacement).to_string());
    trace.push(replace_pointer(empty_array, invalid_item, replacement).to_string());
    trace.push(replace_pointer(empty_array, ptr::null_mut(), replacement).to_string());
    trace.push(replace_pointer(empty_array, invalid_item, ptr::null_mut()).to_string());

    let replace_array: unsafe extern "C" fn(*mut cJSON, c_int, *mut cJSON) -> c_int =
        api.symbol(b"cJSON_ReplaceItemInArray\0");
    trace.push(replace_array(empty_array, -1, replacement).to_string());
    trace.push(replace_array(empty_array, 0, replacement).to_string());

    for replace_name in [
        b"cJSON_ReplaceItemInObject\0".as_slice(),
        b"cJSON_ReplaceItemInObjectCaseSensitive\0".as_slice(),
    ] {
        let replace: unsafe extern "C" fn(*mut cJSON, *const c_char, *mut cJSON) -> c_int =
            api.symbol(replace_name);
        trace.push(replace(object, ptr::null(), replacement).to_string());
        trace.push(replace(object, c"x".as_ptr(), ptr::null_mut()).to_string());
        let fresh = create_number(api, 2.0);
        trace.push(replace(object, c"absent".as_ptr(), fresh).to_string());
        delete(api, fresh);
    }

    let create_text_names = [
        b"cJSON_CreateString\0".as_slice(),
        b"cJSON_CreateRaw\0".as_slice(),
    ];
    for create_name in create_text_names {
        let create: unsafe extern "C" fn(*const c_char) -> *mut cJSON = api.symbol(create_name);
        trace.push(create(ptr::null()).is_null().to_string());
    }
    for create_name in [
        b"cJSON_CreateIntArray\0".as_slice(),
        b"cJSON_CreateFloatArray\0".as_slice(),
        b"cJSON_CreateDoubleArray\0".as_slice(),
    ] {
        let create: unsafe extern "C" fn(*const c_void, c_int) -> *mut cJSON =
            api.symbol(create_name);
        trace.push(create(ptr::null(), 0).is_null().to_string());
        trace.push(create(ptr::null(), -1).is_null().to_string());
        let one = 0u64;
        trace.push(
            create((&one as *const u64).cast(), -1)
                .is_null()
                .to_string(),
        );
    }
    let create_strings: unsafe extern "C" fn(*const *const c_char, c_int) -> *mut cJSON =
        api.symbol(b"cJSON_CreateStringArray\0");
    trace.push(create_strings(ptr::null(), 0).is_null().to_string());
    trace.push(create_strings(ptr::null(), -1).is_null().to_string());

    let duplicate: unsafe extern "C" fn(*const cJSON, c_int) -> *mut cJSON =
        api.symbol(b"cJSON_Duplicate\0");
    trace.push(duplicate(ptr::null(), 0).is_null().to_string());
    let compare: unsafe extern "C" fn(*const cJSON, *const cJSON, c_int) -> c_int =
        api.symbol(b"cJSON_Compare\0");
    trace.push(compare(ptr::null(), invalid_item, 0).to_string());
    trace.push(compare(invalid_item, raw, 0).to_string());
    trace.push(compare(invalid_item, invalid_item, 0).to_string());

    let malloc_fn: unsafe extern "C" fn(usize) -> *mut c_void = api.symbol(b"cJSON_malloc\0");
    let free_fn: unsafe extern "C" fn(*mut c_void) = api.symbol(b"cJSON_free\0");
    let memory = malloc_fn(37);
    trace.push((!memory.is_null()).to_string());
    free_fn(memory);
    free_fn(ptr::null_mut());

    delete(api, replacement);
    delete(api, string);
    delete(api, raw);
    delete(api, invalid_item);
    delete(api, empty_array);
    delete(api, object);
    trace
}

unsafe fn compare_errors(c: &Api, rust: &Api) {
    let c_trace = null_and_error_trace(c);
    let rust_trace = null_and_error_trace(rust);
    assert_eq!(c_trace.len(), rust_trace.len());
    for (index, (c_value, rust_value)) in c_trace.iter().zip(&rust_trace).enumerate() {
        assert_eq!(c_value, rust_value, "error trace index {index}");
    }
}

unsafe fn deep_duplicate_result(api: &Api) -> bool {
    let depth = 10002usize;
    let root = create0(api, b"cJSON_CreateArray\0");
    let mut current = root;
    for _ in 0..depth {
        let child = create0(api, b"cJSON_CreateArray\0");
        (*current).child = child;
        (*child).prev = child;
        current = child;
    }
    let duplicate: unsafe extern "C" fn(*const cJSON, c_int) -> *mut cJSON =
        api.symbol(b"cJSON_Duplicate\0");
    let copy = duplicate(root, 1);
    let rejected = copy.is_null();
    delete(api, copy);

    current = root;
    while !current.is_null() {
        let child = (*current).child;
        (*current).child = ptr::null_mut();
        delete(api, current);
        current = child;
    }
    rejected
}

unsafe fn symbol_surface(api: &Api) {
    const SYMBOLS: &[&[u8]] = &[
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
    for symbol in SYMBOLS {
        let _: *mut c_void = api.symbol(symbol);
    }
    let _: *mut c_void = api.driver_symbol(b"driver\0");
}

unsafe fn capture_stdout<F: FnOnce() -> c_int>(function: F) -> (c_int, Vec<u8>) {
    let mut descriptors = [0; 2];
    assert_eq!(pipe(descriptors.as_mut_ptr()), 0);
    let saved_stdout = dup(1);
    assert!(saved_stdout >= 0);
    fflush(ptr::null_mut());
    assert_eq!(dup2(descriptors[1], 1), 1);
    close(descriptors[1]);

    let result = function();

    fflush(ptr::null_mut());
    assert_eq!(dup2(saved_stdout, 1), 1);
    close(saved_stdout);
    let mut output = Vec::new();
    let mut reader = File::from_raw_fd(descriptors[0]);
    reader.read_to_end(&mut output).unwrap();
    (result, output)
}

unsafe fn driver_output(api: &Api) -> (c_int, Vec<u8>) {
    let day_values: Vec<CString> = [
        "Sunday",
        "Monday",
        "Tuesday",
        "Wednesday",
        "Thursday",
        "Friday",
        "Saturday",
    ]
    .into_iter()
    .map(|value| CString::new(value).unwrap())
    .collect();
    let day_pointers: Vec<*const c_char> = day_values.iter().map(|value| value.as_ptr()).collect();
    let mut numbers = [[0, -1, 2], [3, 4, 5], [6, 7, 8]];
    let mut ids = [116, 943, 234, 38793];

    let strings: Vec<CString> = [
        "zip",
        "",
        "SAN FRANCISCO",
        "CA",
        "94107",
        "US",
        "SUNNYVALE",
        "94085",
    ]
    .into_iter()
    .map(|value| CString::new(value).unwrap())
    .collect();
    let mut fields = [
        Record {
            precision: strings[0].as_ptr(),
            lat: 37.7668,
            lon: -122.3959,
            address: strings[1].as_ptr(),
            city: strings[2].as_ptr(),
            state: strings[3].as_ptr(),
            zip: strings[4].as_ptr(),
            country: strings[5].as_ptr(),
        },
        Record {
            precision: strings[0].as_ptr(),
            lat: 37.371991,
            lon: -122.026020,
            address: strings[1].as_ptr(),
            city: strings[6].as_ptr(),
            state: strings[3].as_ptr(),
            zip: strings[7].as_ptr(),
            country: strings[5].as_ptr(),
        },
    ];
    let driver: unsafe extern "C" fn(
        *const *const c_char,
        *mut [c_int; 3],
        *mut c_int,
        *mut Record,
    ) -> c_int = api.driver_symbol(b"driver\0");
    capture_stdout(|| {
        driver(
            day_pointers.as_ptr(),
            numbers.as_mut_ptr(),
            ids.as_mut_ptr(),
            fields.as_mut_ptr(),
        )
    })
}

unsafe fn version(api: &Api) -> Vec<u8> {
    let function: unsafe extern "C" fn() -> *const c_char = api.symbol(b"cJSON_Version\0");
    CStr::from_ptr(function()).to_bytes().to_vec()
}

unsafe fn run_all() {
    let (c_library, c_driver_library, rust_library) = library_paths();
    assert!(c_library.exists(), "missing {}", c_library.display());
    assert!(
        c_driver_library.exists(),
        "missing {}",
        c_driver_library.display()
    );
    assert!(rust_library.exists(), "missing {}", rust_library.display());

    let c = Api::load("C", &c_library, Some(&c_driver_library));
    let rust = Api::load("Rust", &rust_library, None);

    symbol_surface(&c);
    symbol_surface(&rust);
    assert_eq!(version(&c), version(&rust));
    assert_eq!(version(&c), b"1.7.19");

    compare_parse_surfaces(&c, &rust);
    compare_construction(&c, &rust);
    compare_direct_low_level(&c, &rust);
    compare_arrays_and_types(&c, &rust);
    compare_print_and_minify(&c, &rust);
    compare_errors(&c, &rust);
    compare_allocation_failures(&c, &rust);
    compare_targeted_failures(&c, &rust);
    assert_eq!(deep_duplicate_result(&c), deep_duplicate_result(&rust));
    assert!(deep_duplicate_result(&c));
    assert_eq!(driver_output(&c), driver_output(&rust));

    delete(&c, ptr::null_mut());
    delete(&rust, ptr::null_mut());
}

#[test]
fn differential_surface_matches_c() {
    std::thread::Builder::new()
        .name("cjson-differential".into())
        .stack_size(64 * 1024 * 1024)
        .spawn(|| unsafe { run_all() })
        .unwrap()
        .join()
        .unwrap();
}
