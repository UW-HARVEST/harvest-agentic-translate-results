use libloading::Library;
use std::ffi::{CStr, CString, c_char, c_double, c_float, c_int, c_void};
use std::fs::File;
use std::io::Read;
use std::os::fd::FromRawFd;
use std::path::{Path, PathBuf};
use std::ptr;
use std::sync::atomic::{AtomicIsize, Ordering};

const INVALID: c_int = 0;
const FALSE: c_int = 1;
const TRUE: c_int = 2;
const NULL: c_int = 4;
const NUMBER: c_int = 8;
const STRING: c_int = 16;
const ARRAY: c_int = 32;
const OBJECT: c_int = 64;
const RAW: c_int = 128;
const IS_REFERENCE: c_int = 256;
const STRING_IS_CONST: c_int = 512;

#[repr(C)]
#[derive(Debug)]
struct Json {
    next: *mut Json,
    prev: *mut Json,
    child: *mut Json,
    kind: c_int,
    valuestring: *mut c_char,
    valueint: c_int,
    valuedouble: c_double,
    string: *mut c_char,
}

type Allocate = unsafe extern "C" fn(usize) -> *mut c_void;
type Deallocate = unsafe extern "C" fn(*mut c_void);

#[repr(C)]
struct Hooks {
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
    fn pipe(fds: *mut c_int) -> c_int;
    fn dup(fd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
}

struct Api {
    library: Library,
    label: &'static str,
}

impl Api {
    unsafe fn open(path: &Path, label: &'static str) -> Self {
        Self {
            library: unsafe { Library::new(path) }
                .unwrap_or_else(|error| panic!("load {label} at {}: {error}", path.display())),
            label,
        }
    }

    unsafe fn symbol<T: Copy>(&self, name: &[u8]) -> T {
        *unsafe { self.library.get::<T>(name) }.unwrap_or_else(|error| {
            panic!(
                "{} missing {}: {error}",
                self.label,
                String::from_utf8_lossy(name)
            )
        })
    }

    unsafe fn delete(&self, item: *mut Json) {
        let function: unsafe extern "C" fn(*mut Json) = unsafe { self.symbol(b"cJSON_Delete") };
        unsafe { function(item) };
    }

    unsafe fn free(&self, memory: *mut c_void) {
        let function: unsafe extern "C" fn(*mut c_void) = unsafe { self.symbol(b"cJSON_free") };
        unsafe { function(memory) };
    }

    unsafe fn parse_bytes(
        &self,
        bytes: &[u8],
        length: usize,
        parse_end: Option<&mut *const c_char>,
        require_nul: c_int,
    ) -> *mut Json {
        let function: unsafe extern "C" fn(
            *const c_char,
            usize,
            *mut *const c_char,
            c_int,
        ) -> *mut Json = unsafe { self.symbol(b"cJSON_ParseWithLengthOpts") };
        unsafe {
            function(
                bytes.as_ptr().cast(),
                length,
                parse_end.map_or(ptr::null_mut(), |end| end),
                require_nul,
            )
        }
    }

    unsafe fn render(&self, item: *const Json, formatted: bool) -> Option<Vec<u8>> {
        let name = if formatted {
            b"cJSON_Print\0".as_slice()
        } else {
            b"cJSON_PrintUnformatted\0".as_slice()
        };
        let function: unsafe extern "C" fn(*const Json) -> *mut c_char =
            unsafe { self.symbol(name) };
        let output = unsafe { function(item) };
        if output.is_null() {
            return None;
        }
        let bytes = unsafe { CStr::from_ptr(output) }.to_bytes().to_vec();
        unsafe { self.free(output.cast()) };
        Some(bytes)
    }

    unsafe fn parse_and_render(&self, json: &[u8], formatted: bool) -> Option<Vec<u8>> {
        let mut input = json.to_vec();
        input.push(0);
        let parse: unsafe extern "C" fn(*const c_char) -> *mut Json =
            unsafe { self.symbol(b"cJSON_Parse") };
        let item = unsafe { parse(input.as_ptr().cast()) };
        if item.is_null() {
            return None;
        }
        let output = unsafe { self.render(item, formatted) };
        unsafe { self.delete(item) };
        output
    }
}

struct Pair {
    c: Api,
    rust: Api,
    c_driver: Api,
}

impl Pair {
    unsafe fn load() -> Self {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let c_build = root.join("../c_src/build");
        let c = c_build.join("libcjson.so");
        let c_driver = c_build.join("libcJSON_test.so");
        let rust = root.join("target/release/libcJSON_test.so");
        assert!(
            c.is_file(),
            "missing C library {}; build C first",
            c.display()
        );
        assert!(
            c_driver.is_file(),
            "missing C driver library {}; build C first",
            c_driver.display()
        );
        assert!(
            rust.is_file(),
            "missing Rust library {}; run cargo build --release first",
            rust.display()
        );
        Self {
            c: unsafe { Api::open(&c, "C") },
            rust: unsafe { Api::open(&rust, "Rust") },
            c_driver: unsafe { Api::open(&c_driver, "C driver") },
        }
    }
}

struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Self(seed)
    }

    fn next_u64(&mut self) -> u64 {
        let mut value = self.0;
        value ^= value << 13;
        value ^= value >> 7;
        value ^= value << 17;
        self.0 = value;
        value
    }

    fn range(&mut self, upper: usize) -> usize {
        (self.next_u64() as usize) % upper
    }

    fn json_string(&mut self) -> String {
        const PARTS: &[&str] = &[
            "alpha",
            "BETA",
            "",
            "quote\\\"",
            "slash\\\\",
            "\\n",
            "\\t",
            "\\u00e9",
            "\\u20ac",
            "\\ud83d\\ude00",
        ];
        format!("{}{}", PARTS[self.range(PARTS.len())], self.range(1000))
    }

    fn json_value(&mut self, depth: usize) -> String {
        if depth == 0 {
            return match self.range(6) {
                0 => "null".into(),
                1 => "true".into(),
                2 => "false".into(),
                3 => format!("{}", self.next_u64() as i64),
                4 => format!("{}.{:06}", self.next_u64() as i32, self.range(1_000_000)),
                _ => format!("\"{}\"", self.json_string()),
            };
        }
        match self.range(8) {
            0..=4 => self.json_value(0),
            5 => {
                let count = self.range(5);
                let values = (0..count)
                    .map(|_| self.json_value(depth - 1))
                    .collect::<Vec<_>>();
                format!("[{}]", values.join(","))
            }
            _ => {
                let count = self.range(5);
                let values = (0..count)
                    .map(|index| {
                        format!(
                            "\"k{index}{}\":{}",
                            self.range(10),
                            self.json_value(depth - 1)
                        )
                    })
                    .collect::<Vec<_>>();
                format!("{{{}}}", values.join(","))
            }
        }
    }
}

fn assert_same_parse(pair: &Pair, input: &[u8]) {
    let c = unsafe { pair.c.parse_and_render(input, false) };
    let rust = unsafe { pair.rust.parse_and_render(input, false) };
    assert_eq!(rust, c, "parse/render mismatch for {:?}", input);
}

unsafe fn create_number(api: &Api, value: f64) -> *mut Json {
    let function: unsafe extern "C" fn(f64) -> *mut Json =
        unsafe { api.symbol(b"cJSON_CreateNumber") };
    unsafe { function(value) }
}

unsafe fn create_container(api: &Api, object: bool) -> *mut Json {
    let name = if object {
        b"cJSON_CreateObject\0".as_slice()
    } else {
        b"cJSON_CreateArray\0".as_slice()
    };
    let function: unsafe extern "C" fn() -> *mut Json = unsafe { api.symbol(name) };
    unsafe { function() }
}

unsafe fn add_array(api: &Api, array: *mut Json, item: *mut Json) -> c_int {
    let function: unsafe extern "C" fn(*mut Json, *mut Json) -> c_int =
        unsafe { api.symbol(b"cJSON_AddItemToArray") };
    unsafe { function(array, item) }
}

unsafe fn build_helper_object(api: &Api, seed: u64) -> Vec<u8> {
    let object = unsafe { create_container(api, true) };
    assert!(!object.is_null());
    let key_null = CString::new("null").unwrap();
    let key_true = CString::new("true").unwrap();
    let key_false = CString::new("false").unwrap();
    let key_bool = CString::new("bool").unwrap();
    let key_number = CString::new("number").unwrap();
    let key_string = CString::new("string").unwrap();
    let key_raw = CString::new("raw").unwrap();
    let key_object = CString::new("object").unwrap();
    let key_array = CString::new("array").unwrap();
    let value = CString::new(format!("value-{seed}")).unwrap();
    let raw = CString::new(format!("{{\"seed\":{seed}}}")).unwrap();

    macro_rules! call {
        ($symbol:literal, $type:ty, $($argument:expr),*) => {{
            let function: $type = unsafe { api.symbol($symbol) };
            let result = unsafe { function($($argument),*) };
            assert!(!result.is_null(), "{} helper failed", api.label);
        }};
    }
    call!(
        b"cJSON_AddNullToObject",
        unsafe extern "C" fn(*mut Json, *const c_char) -> *mut Json,
        object,
        key_null.as_ptr()
    );
    call!(
        b"cJSON_AddTrueToObject",
        unsafe extern "C" fn(*mut Json, *const c_char) -> *mut Json,
        object,
        key_true.as_ptr()
    );
    call!(
        b"cJSON_AddFalseToObject",
        unsafe extern "C" fn(*mut Json, *const c_char) -> *mut Json,
        object,
        key_false.as_ptr()
    );
    call!(
        b"cJSON_AddBoolToObject",
        unsafe extern "C" fn(*mut Json, *const c_char, c_int) -> *mut Json,
        object,
        key_bool.as_ptr(),
        -7
    );
    call!(
        b"cJSON_AddNumberToObject",
        unsafe extern "C" fn(*mut Json, *const c_char, f64) -> *mut Json,
        object,
        key_number.as_ptr(),
        seed as f64 + 0.25
    );
    call!(
        b"cJSON_AddStringToObject",
        unsafe extern "C" fn(*mut Json, *const c_char, *const c_char) -> *mut Json,
        object,
        key_string.as_ptr(),
        value.as_ptr()
    );
    call!(
        b"cJSON_AddRawToObject",
        unsafe extern "C" fn(*mut Json, *const c_char, *const c_char) -> *mut Json,
        object,
        key_raw.as_ptr(),
        raw.as_ptr()
    );
    call!(
        b"cJSON_AddObjectToObject",
        unsafe extern "C" fn(*mut Json, *const c_char) -> *mut Json,
        object,
        key_object.as_ptr()
    );
    call!(
        b"cJSON_AddArrayToObject",
        unsafe extern "C" fn(*mut Json, *const c_char) -> *mut Json,
        object,
        key_array.as_ptr()
    );

    let rendered = unsafe { api.render(object, false) }.unwrap();
    unsafe { api.delete(object) };
    rendered
}

unsafe fn build_low_level_tree(api: &Api) -> (*mut Json, Vec<CString>) {
    let object = unsafe { create_container(api, true) };
    let mut keepalive = Vec::new();
    for (index, key) in ["Alpha", "beta", "Gamma"].into_iter().enumerate() {
        let key = CString::new(key).unwrap();
        let item = unsafe { create_number(api, (index * 10) as f64) };
        let add: unsafe extern "C" fn(*mut Json, *const c_char, *mut Json) -> c_int =
            unsafe { api.symbol(b"cJSON_AddItemToObject") };
        assert_eq!(unsafe { add(object, key.as_ptr(), item) }, 1);
        keepalive.push(key);
    }
    let const_key = CString::new("Const").unwrap();
    let const_item = unsafe { create_number(api, 99.0) };
    let add_const: unsafe extern "C" fn(*mut Json, *const c_char, *mut Json) -> c_int =
        unsafe { api.symbol(b"cJSON_AddItemToObjectCS") };
    assert_eq!(
        unsafe { add_const(object, const_key.as_ptr(), const_item) },
        1
    );
    keepalive.push(const_key);
    (object, keepalive)
}

unsafe fn mutation_result(api: &Api) -> Vec<Vec<u8>> {
    let mut snapshots = Vec::new();
    let array = unsafe { create_container(api, false) };
    for value in [1.0, 2.0, 3.0] {
        let item = unsafe { create_number(api, value) };
        assert_eq!(unsafe { add_array(api, array, item) }, 1);
    }
    snapshots.push(unsafe { api.render(array, false) }.unwrap());

    let inserted = unsafe { create_number(api, 8.0) };
    let insert: unsafe extern "C" fn(*mut Json, c_int, *mut Json) -> c_int =
        unsafe { api.symbol(b"cJSON_InsertItemInArray") };
    assert_eq!(unsafe { insert(array, 1, inserted) }, 1);
    snapshots.push(unsafe { api.render(array, false) }.unwrap());

    let replacement = unsafe { create_number(api, 9.0) };
    let replace: unsafe extern "C" fn(*mut Json, c_int, *mut Json) -> c_int =
        unsafe { api.symbol(b"cJSON_ReplaceItemInArray") };
    assert_eq!(unsafe { replace(array, 2, replacement) }, 1);
    snapshots.push(unsafe { api.render(array, false) }.unwrap());

    let get: unsafe extern "C" fn(*const Json, c_int) -> *mut Json =
        unsafe { api.symbol(b"cJSON_GetArrayItem") };
    let middle = unsafe { get(array, 1) };
    let detach_pointer: unsafe extern "C" fn(*mut Json, *mut Json) -> *mut Json =
        unsafe { api.symbol(b"cJSON_DetachItemViaPointer") };
    let detached = unsafe { detach_pointer(array, middle) };
    assert_eq!(detached, middle);
    snapshots.push(unsafe { api.render(array, false) }.unwrap());
    unsafe { api.delete(detached) };

    let detach_index: unsafe extern "C" fn(*mut Json, c_int) -> *mut Json =
        unsafe { api.symbol(b"cJSON_DetachItemFromArray") };
    let detached = unsafe { detach_index(array, 0) };
    snapshots.push(unsafe { api.render(array, false) }.unwrap());
    unsafe { api.delete(detached) };

    let delete_index: unsafe extern "C" fn(*mut Json, c_int) =
        unsafe { api.symbol(b"cJSON_DeleteItemFromArray") };
    unsafe { delete_index(array, 0) };
    snapshots.push(unsafe { api.render(array, false) }.unwrap());
    unsafe { api.delete(array) };
    snapshots
}

unsafe fn object_mutation_result(api: &Api) -> Vec<Vec<u8>> {
    let mut snapshots = Vec::new();
    let (object, _keys) = unsafe { build_low_level_tree(api) };
    let lower = CString::new("alpha").unwrap();
    let exact = CString::new("Gamma").unwrap();

    let get_ci: unsafe extern "C" fn(*const Json, *const c_char) -> *mut Json =
        unsafe { api.symbol(b"cJSON_GetObjectItem") };
    let get_cs: unsafe extern "C" fn(*const Json, *const c_char) -> *mut Json =
        unsafe { api.symbol(b"cJSON_GetObjectItemCaseSensitive") };
    let has: unsafe extern "C" fn(*const Json, *const c_char) -> c_int =
        unsafe { api.symbol(b"cJSON_HasObjectItem") };
    assert!(!unsafe { get_ci(object, lower.as_ptr()) }.is_null());
    assert!(unsafe { get_cs(object, lower.as_ptr()) }.is_null());
    assert_eq!(unsafe { has(object, lower.as_ptr()) }, 1);

    let replacement = unsafe { create_number(api, 40.0) };
    let replace_ci: unsafe extern "C" fn(*mut Json, *const c_char, *mut Json) -> c_int =
        unsafe { api.symbol(b"cJSON_ReplaceItemInObject") };
    assert_eq!(
        unsafe { replace_ci(object, lower.as_ptr(), replacement) },
        1
    );
    snapshots.push(unsafe { api.render(object, false) }.unwrap());

    let replacement = unsafe { create_number(api, 50.0) };
    let replace_cs: unsafe extern "C" fn(*mut Json, *const c_char, *mut Json) -> c_int =
        unsafe { api.symbol(b"cJSON_ReplaceItemInObjectCaseSensitive") };
    assert_eq!(
        unsafe { replace_cs(object, exact.as_ptr(), replacement) },
        1
    );
    snapshots.push(unsafe { api.render(object, false) }.unwrap());

    let detach_ci: unsafe extern "C" fn(*mut Json, *const c_char) -> *mut Json =
        unsafe { api.symbol(b"cJSON_DetachItemFromObject") };
    let detached = unsafe { detach_ci(object, lower.as_ptr()) };
    snapshots.push(unsafe { api.render(object, false) }.unwrap());
    unsafe { api.delete(detached) };

    let delete_cs: unsafe extern "C" fn(*mut Json, *const c_char) =
        unsafe { api.symbol(b"cJSON_DeleteItemFromObjectCaseSensitive") };
    let beta = CString::new("beta").unwrap();
    unsafe { delete_cs(object, beta.as_ptr()) };
    snapshots.push(unsafe { api.render(object, false) }.unwrap());

    let detach_cs: unsafe extern "C" fn(*mut Json, *const c_char) -> *mut Json =
        unsafe { api.symbol(b"cJSON_DetachItemFromObjectCaseSensitive") };
    let detached = unsafe { detach_cs(object, exact.as_ptr()) };
    snapshots.push(unsafe { api.render(object, false) }.unwrap());
    unsafe { api.delete(detached) };

    let delete_ci: unsafe extern "C" fn(*mut Json, *const c_char) =
        unsafe { api.symbol(b"cJSON_DeleteItemFromObject") };
    let constant = CString::new("const").unwrap();
    unsafe { delete_ci(object, constant.as_ptr()) };
    snapshots.push(unsafe { api.render(object, false) }.unwrap());
    unsafe { api.delete(object) };
    snapshots
}

static FAIL_AFTER: AtomicIsize = AtomicIsize::new(-1);

unsafe extern "C" fn controlled_malloc(size: usize) -> *mut c_void {
    let remaining = FAIL_AFTER.fetch_sub(1, Ordering::SeqCst);
    if remaining == 0 {
        ptr::null_mut()
    } else {
        unsafe { malloc(size) }
    }
}

unsafe extern "C" fn controlled_free(pointer: *mut c_void) {
    unsafe { free(pointer) };
}

unsafe fn set_hooks(api: &Api, enabled: bool) {
    let init: unsafe extern "C" fn(*mut Hooks) = unsafe { api.symbol(b"cJSON_InitHooks") };
    if enabled {
        let mut hooks = Hooks {
            malloc_fn: Some(controlled_malloc),
            free_fn: Some(controlled_free),
        };
        unsafe { init(&mut hooks) };
    } else {
        unsafe { init(ptr::null_mut()) };
    }
}

unsafe fn allocation_outcomes(api: &Api) -> Vec<(bool, bool, bool)> {
    let parse: unsafe extern "C" fn(*const c_char) -> *mut Json =
        unsafe { api.symbol(b"cJSON_Parse") };
    let create: unsafe extern "C" fn(*const c_char) -> *mut Json =
        unsafe { api.symbol(b"cJSON_CreateString") };
    let input =
        CString::new(r#"{"long":[1,2,3,4,5],"text":"abcdefghijklmnopqrstuvwxyz"}"#).unwrap();
    let value = CString::new("allocation-value").unwrap();
    let mut outcomes = Vec::new();
    unsafe { set_hooks(api, true) };
    for fail_at in 0..24 {
        FAIL_AFTER.store(fail_at, Ordering::SeqCst);
        let parsed = unsafe { parse(input.as_ptr()) };
        let parse_ok = !parsed.is_null();
        if parse_ok {
            unsafe { api.delete(parsed) };
        }

        FAIL_AFTER.store(fail_at, Ordering::SeqCst);
        let string = unsafe { create(value.as_ptr()) };
        let create_ok = !string.is_null();
        if create_ok {
            unsafe { api.delete(string) };
        }

        FAIL_AFTER.store(fail_at, Ordering::SeqCst);
        let item = unsafe { create_number(api, 12.5) };
        let print_ok = if item.is_null() {
            false
        } else {
            let output = unsafe { api.render(item, false) };
            unsafe { api.delete(item) };
            output.is_some()
        };
        outcomes.push((parse_ok, create_ok, print_ok));
    }
    FAIL_AFTER.store(-1, Ordering::SeqCst);
    unsafe { set_hooks(api, false) };
    outcomes
}

unsafe fn custom_hook_growth(api: &Api) -> Vec<u8> {
    unsafe { set_hooks(api, true) };
    FAIL_AFTER.store(-1, Ordering::SeqCst);
    let value = CString::new("x".repeat(4096)).unwrap();
    let create: unsafe extern "C" fn(*const c_char) -> *mut Json =
        unsafe { api.symbol(b"cJSON_CreateString") };
    let item = unsafe { create(value.as_ptr()) };
    let output = unsafe { api.render(item, false) }.unwrap();
    unsafe { api.delete(item) };
    unsafe { set_hooks(api, false) };
    output
}

fn test_duplicate_limit(pair: &Pair) {
    let run = |api: &Api| {
        let mut nodes = (0..10_002)
            .map(|_| Json {
                next: ptr::null_mut(),
                prev: ptr::null_mut(),
                child: ptr::null_mut(),
                kind: ARRAY,
                valuestring: ptr::null_mut(),
                valueint: 0,
                valuedouble: 0.0,
                string: ptr::null_mut(),
            })
            .collect::<Vec<_>>();
        for index in 0..nodes.len() - 1 {
            nodes[index].child = &mut nodes[index + 1];
        }
        let duplicate: unsafe extern "C" fn(*const Json, c_int) -> *mut Json =
            unsafe { api.symbol(b"cJSON_Duplicate") };
        let copy = unsafe { duplicate(nodes.as_ptr(), 1) };
        let rejected = copy.is_null();
        unsafe { api.delete(copy) };
        rejected
    };
    let c = run(&pair.c);
    let rust = run(&pair.rust);
    assert_eq!(rust, c);
    assert!(c);
}

unsafe fn capture_stdout<F: FnOnce() -> c_int>(call: F) -> (c_int, Vec<u8>) {
    let mut fds = [-1; 2];
    assert_eq!(unsafe { pipe(fds.as_mut_ptr()) }, 0);
    let saved = unsafe { dup(1) };
    assert!(saved >= 0);
    unsafe { fflush(ptr::null_mut()) };
    assert_eq!(unsafe { dup2(fds[1], 1) }, 1);
    unsafe { close(fds[1]) };
    let result = call();
    unsafe { fflush(ptr::null_mut()) };
    assert_eq!(unsafe { dup2(saved, 1) }, 1);
    unsafe { close(saved) };
    let mut output = Vec::new();
    let mut reader = unsafe { File::from_raw_fd(fds[0]) };
    reader.read_to_end(&mut output).unwrap();
    (result, output)
}

#[test]
fn differential_surface() {
    let pair = unsafe { Pair::load() };

    const SYMBOLS: &[&[u8]] = &[
        b"cJSON_AddArrayToObject",
        b"cJSON_AddBoolToObject",
        b"cJSON_AddFalseToObject",
        b"cJSON_AddItemReferenceToArray",
        b"cJSON_AddItemReferenceToObject",
        b"cJSON_AddItemToArray",
        b"cJSON_AddItemToObject",
        b"cJSON_AddItemToObjectCS",
        b"cJSON_AddNullToObject",
        b"cJSON_AddNumberToObject",
        b"cJSON_AddObjectToObject",
        b"cJSON_AddRawToObject",
        b"cJSON_AddStringToObject",
        b"cJSON_AddTrueToObject",
        b"cJSON_Compare",
        b"cJSON_CreateArray",
        b"cJSON_CreateArrayReference",
        b"cJSON_CreateBool",
        b"cJSON_CreateDoubleArray",
        b"cJSON_CreateFalse",
        b"cJSON_CreateFloatArray",
        b"cJSON_CreateIntArray",
        b"cJSON_CreateNull",
        b"cJSON_CreateNumber",
        b"cJSON_CreateObject",
        b"cJSON_CreateObjectReference",
        b"cJSON_CreateRaw",
        b"cJSON_CreateString",
        b"cJSON_CreateStringArray",
        b"cJSON_CreateStringReference",
        b"cJSON_CreateTrue",
        b"cJSON_Delete",
        b"cJSON_DeleteItemFromArray",
        b"cJSON_DeleteItemFromObject",
        b"cJSON_DeleteItemFromObjectCaseSensitive",
        b"cJSON_DetachItemFromArray",
        b"cJSON_DetachItemFromObject",
        b"cJSON_DetachItemFromObjectCaseSensitive",
        b"cJSON_DetachItemViaPointer",
        b"cJSON_Duplicate",
        b"cJSON_GetArrayItem",
        b"cJSON_GetArraySize",
        b"cJSON_GetErrorPtr",
        b"cJSON_GetNumberValue",
        b"cJSON_GetObjectItem",
        b"cJSON_GetObjectItemCaseSensitive",
        b"cJSON_GetStringValue",
        b"cJSON_HasObjectItem",
        b"cJSON_InitHooks",
        b"cJSON_InsertItemInArray",
        b"cJSON_IsArray",
        b"cJSON_IsBool",
        b"cJSON_IsFalse",
        b"cJSON_IsInvalid",
        b"cJSON_IsNull",
        b"cJSON_IsNumber",
        b"cJSON_IsObject",
        b"cJSON_IsRaw",
        b"cJSON_IsString",
        b"cJSON_IsTrue",
        b"cJSON_Minify",
        b"cJSON_Parse",
        b"cJSON_ParseWithLength",
        b"cJSON_ParseWithLengthOpts",
        b"cJSON_ParseWithOpts",
        b"cJSON_Print",
        b"cJSON_PrintBuffered",
        b"cJSON_PrintPreallocated",
        b"cJSON_PrintUnformatted",
        b"cJSON_ReplaceItemInArray",
        b"cJSON_ReplaceItemInObject",
        b"cJSON_ReplaceItemInObjectCaseSensitive",
        b"cJSON_ReplaceItemViaPointer",
        b"cJSON_SetNumberHelper",
        b"cJSON_SetValuestring",
        b"cJSON_Version",
        b"cJSON_free",
        b"cJSON_malloc",
    ];
    for symbol in SYMBOLS {
        let _: *const c_void = unsafe { pair.c.symbol(symbol) };
        let _: *const c_void = unsafe { pair.rust.symbol(symbol) };
    }
    let _: *const c_void = unsafe { pair.c_driver.symbol(b"driver") };
    let _: *const c_void = unsafe { pair.rust.symbol(b"driver") };

    let version: unsafe extern "C" fn() -> *const c_char =
        unsafe { pair.c.symbol(b"cJSON_Version") };
    let rust_version: unsafe extern "C" fn() -> *const c_char =
        unsafe { pair.rust.symbol(b"cJSON_Version") };
    assert_eq!(
        unsafe { CStr::from_ptr(rust_version()) }.to_bytes(),
        unsafe { CStr::from_ptr(version()) }.to_bytes()
    );

    let mut rng = Rng::new(0x7f4a_7c15_9e37_79b9);
    let fixed = [
        b"null".as_slice(),
        b" false ",
        b"true trailing",
        br#""""#,
        br#""a\\b\"c\/d\b\f\n\r\t""#,
        br#""\u0000\u007f\u0080\u07ff\u0800\uffff\ud83d\ude00""#,
        b"0",
        b"-0",
        b"2147483647",
        b"2147483648",
        b"-2147483649",
        b"1.2345678901234567",
        b"1e308",
        b"1e-308",
        b"[]",
        b"[1]",
        b"[null, true, false, 1.5, \"x\"]",
        b"{}",
        br#"{"a":1,"A":2,"a":3}"#,
        br#"{"nested":[{"x":"y"},[],{}]}"#,
    ];
    for input in fixed {
        assert_same_parse(&pair, input);
    }
    for _ in 0..256 {
        let value = rng.json_value(4);
        assert_same_parse(&pair, value.as_bytes());
    }

    for value in [
        0.0,
        -0.0,
        1.0,
        -1.0,
        c_int::MAX as f64,
        c_int::MAX as f64 + 1.0,
        c_int::MIN as f64,
        c_int::MIN as f64 - 1.0,
        f64::MIN_POSITIVE,
        f64::MAX,
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::NAN,
    ] {
        let c_item = unsafe { create_number(&pair.c, value) };
        let rust_item = unsafe { create_number(&pair.rust, value) };
        assert_eq!(
            unsafe { pair.rust.render(rust_item, false) },
            unsafe { pair.c.render(c_item, false) },
            "number {value:?}"
        );
        assert_eq!(unsafe { (*rust_item).valueint }, unsafe {
            (*c_item).valueint
        });
        unsafe {
            pair.c.delete(c_item);
            pair.rust.delete(rust_item);
        }
    }
    for _ in 0..256 {
        let bits = rng.next_u64();
        let value = f64::from_bits(bits);
        let c_item = unsafe { create_number(&pair.c, value) };
        let rust_item = unsafe { create_number(&pair.rust, value) };
        assert_eq!(
            unsafe { pair.rust.render(rust_item, false) },
            unsafe { pair.c.render(c_item, false) },
            "number bits {bits:016x}"
        );
        unsafe {
            pair.c.delete(c_item);
            pair.rust.delete(rust_item);
        }
    }

    assert_eq!(unsafe { build_helper_object(&pair.rust, 12345) }, unsafe {
        build_helper_object(&pair.c, 12345)
    });
    assert_eq!(unsafe { mutation_result(&pair.rust) }, unsafe {
        mutation_result(&pair.c)
    });
    assert_eq!(unsafe { object_mutation_result(&pair.rust) }, unsafe {
        object_mutation_result(&pair.c)
    });

    test_arrays(&pair, &mut rng);
    test_predicates_values_and_references(&pair);
    assert_eq!(unsafe { direct_api_results(&pair.rust) }, unsafe {
        direct_api_results(&pair.c)
    });
    test_parsers_and_errors(&pair);
    test_print_modes(&pair);
    test_duplicate_compare_minify(&pair);
    test_rejections(&pair);

    assert_eq!(unsafe { allocation_outcomes(&pair.rust) }, unsafe {
        allocation_outcomes(&pair.c)
    });
    assert_eq!(unsafe { custom_hook_growth(&pair.rust) }, unsafe {
        custom_hook_growth(&pair.c)
    });
    test_duplicate_limit(&pair);

    test_driver(&pair);
}

unsafe fn direct_api_results(api: &Api) -> Vec<Vec<u8>> {
    let mut results = Vec::new();

    let allocate: unsafe extern "C" fn(usize) -> *mut c_void =
        unsafe { api.symbol(b"cJSON_malloc") };
    for size in [0, 1, 31, 4096] {
        let memory = unsafe { allocate(size) };
        results.push(vec![(!memory.is_null()) as u8]);
        unsafe { api.free(memory) };
    }

    let create_string_fn: unsafe extern "C" fn(*const c_char) -> *mut Json =
        unsafe { api.symbol(b"cJSON_CreateString") };
    let get_string: unsafe extern "C" fn(*const Json) -> *mut c_char =
        unsafe { api.symbol(b"cJSON_GetStringValue") };
    let set_string: unsafe extern "C" fn(*mut Json, *const c_char) -> *mut c_char =
        unsafe { api.symbol(b"cJSON_SetValuestring") };
    let initial = CString::new("initial-long-value").unwrap();
    let item = unsafe { create_string_fn(initial.as_ptr()) };
    results.push(
        unsafe { CStr::from_ptr(get_string(item)) }
            .to_bytes()
            .to_vec(),
    );
    for replacement in [
        "x",
        "same-size-value!!!",
        "a substantially longer replacement value",
    ] {
        let replacement = CString::new(replacement).unwrap();
        let changed = unsafe { set_string(item, replacement.as_ptr()) };
        results.push(vec![(!changed.is_null()) as u8]);
        results.push(unsafe { api.render(item, false) }.unwrap());
    }
    let overlap = unsafe { set_string(item, (*item).valuestring) };
    results.push(vec![overlap.is_null() as u8]);
    unsafe { api.delete(item) };

    let raw_text = CString::new(r#"{"raw":[1,true]}"#).unwrap();
    let create_raw: unsafe extern "C" fn(*const c_char) -> *mut Json =
        unsafe { api.symbol(b"cJSON_CreateRaw") };
    let raw = unsafe { create_raw(raw_text.as_ptr()) };
    results.push(unsafe { api.render(raw, false) }.unwrap());
    unsafe { api.delete(raw) };

    let set_number: unsafe extern "C" fn(*mut Json, f64) -> f64 =
        unsafe { api.symbol(b"cJSON_SetNumberHelper") };
    let get_number: unsafe extern "C" fn(*const Json) -> f64 =
        unsafe { api.symbol(b"cJSON_GetNumberValue") };
    let number = unsafe { create_number(api, 0.0) };
    for value in [12.0, -12.75, c_int::MAX as f64 + 1.0, f64::NAN] {
        let returned = unsafe { set_number(number, value) };
        let fetched = unsafe { get_number(number) };
        results.push(returned.to_bits().to_ne_bytes().to_vec());
        results.push(fetched.to_bits().to_ne_bytes().to_vec());
        results.push(unsafe { (*number).valueint }.to_ne_bytes().to_vec());
    }
    unsafe { api.delete(number) };

    let parse_opts: unsafe extern "C" fn(*const c_char, *mut *const c_char, c_int) -> *mut Json =
        unsafe { api.symbol(b"cJSON_ParseWithOpts") };
    let parse_length: unsafe extern "C" fn(*const c_char, usize) -> *mut Json =
        unsafe { api.symbol(b"cJSON_ParseWithLength") };
    let parse_length_opts: unsafe extern "C" fn(
        *const c_char,
        usize,
        *mut *const c_char,
        c_int,
    ) -> *mut Json = unsafe { api.symbol(b"cJSON_ParseWithLengthOpts") };
    let input = b"true trailing\0";
    let mut end = ptr::null();
    let parsed = unsafe { parse_opts(input.as_ptr().cast(), &mut end, 0) };
    results.push(
        (unsafe { end.offset_from(input.as_ptr().cast()) } as i64)
            .to_ne_bytes()
            .to_vec(),
    );
    results.push(unsafe { api.render(parsed, false) }.unwrap());
    unsafe { api.delete(parsed) };
    let parsed = unsafe { parse_length(input.as_ptr().cast(), 4) };
    results.push(unsafe { api.render(parsed, false) }.unwrap());
    unsafe { api.delete(parsed) };
    let exact = b"false\0";
    let parsed =
        unsafe { parse_length_opts(exact.as_ptr().cast(), exact.len(), ptr::null_mut(), -1) };
    results.push(unsafe { api.render(parsed, false) }.unwrap());
    unsafe { api.delete(parsed) };

    let malformed = b"{\"x\":]\0";
    let failed =
        unsafe { parse_length_opts(malformed.as_ptr().cast(), malformed.len(), &mut end, 1) };
    assert!(failed.is_null());
    let get_error: unsafe extern "C" fn() -> *const c_char =
        unsafe { api.symbol(b"cJSON_GetErrorPtr") };
    let error = unsafe { get_error() };
    results.push(
        (unsafe { error.offset_from(malformed.as_ptr().cast()) } as i64)
            .to_ne_bytes()
            .to_vec(),
    );

    let source_text = CString::new("referenced").unwrap();
    let source = unsafe { create_string_fn(source_text.as_ptr()) };
    let array = unsafe { create_container(api, false) };
    let add_reference: unsafe extern "C" fn(*mut Json, *mut Json) -> c_int =
        unsafe { api.symbol(b"cJSON_AddItemReferenceToArray") };
    results.push(vec![unsafe { add_reference(array, source) } as u8]);
    results.push(unsafe { api.render(array, false) }.unwrap());
    unsafe { api.delete(array) };
    results.push(unsafe { api.render(source, false) }.unwrap());

    let object = unsafe { create_container(api, true) };
    let key = CString::new("reference").unwrap();
    let add_reference_object: unsafe extern "C" fn(*mut Json, *const c_char, *mut Json) -> c_int =
        unsafe { api.symbol(b"cJSON_AddItemReferenceToObject") };
    results.push(vec![
        unsafe { add_reference_object(object, key.as_ptr(), source) } as u8,
    ]);
    results.push(unsafe { api.render(object, false) }.unwrap());
    unsafe {
        api.delete(object);
        api.delete(source);
    }

    let (object, _keys) = unsafe { build_low_level_tree(api) };
    let size: unsafe extern "C" fn(*const Json) -> c_int =
        unsafe { api.symbol(b"cJSON_GetArraySize") };
    results.push(unsafe { size(object) }.to_ne_bytes().to_vec());
    let first = unsafe { (*object).child };
    let replace_pointer: unsafe extern "C" fn(*mut Json, *mut Json, *mut Json) -> c_int =
        unsafe { api.symbol(b"cJSON_ReplaceItemViaPointer") };
    results.push(vec![unsafe { replace_pointer(object, first, first) } as u8]);
    results.push(unsafe { api.render(object, false) }.unwrap());
    unsafe { api.delete(object) };

    results
}

fn test_arrays(pair: &Pair, rng: &mut Rng) {
    unsafe fn one_numeric<T: Copy>(api: &Api, symbol: &[u8], values: &[T]) -> Vec<u8> {
        let create: unsafe extern "C" fn(*const T, c_int) -> *mut Json =
            unsafe { api.symbol(symbol) };
        let item = unsafe { create(values.as_ptr(), values.len() as c_int) };
        let output = unsafe { api.render(item, false) }.unwrap();
        unsafe { api.delete(item) };
        output
    }

    for count in [0, 1, 2, 17, 64] {
        let ints = (0..count)
            .map(|_| rng.next_u64() as c_int)
            .collect::<Vec<_>>();
        assert_eq!(
            unsafe { one_numeric(&pair.rust, b"cJSON_CreateIntArray", &ints) },
            unsafe { one_numeric(&pair.c, b"cJSON_CreateIntArray", &ints) }
        );
        let floats = (0..count)
            .map(|_| f32::from_bits(rng.next_u64() as u32))
            .collect::<Vec<c_float>>();
        assert_eq!(
            unsafe { one_numeric(&pair.rust, b"cJSON_CreateFloatArray", &floats) },
            unsafe { one_numeric(&pair.c, b"cJSON_CreateFloatArray", &floats) }
        );
        let doubles = (0..count)
            .map(|_| f64::from_bits(rng.next_u64()))
            .collect::<Vec<_>>();
        assert_eq!(
            unsafe { one_numeric(&pair.rust, b"cJSON_CreateDoubleArray", &doubles) },
            unsafe { one_numeric(&pair.c, b"cJSON_CreateDoubleArray", &doubles) }
        );

        let strings = (0..count)
            .map(|index| CString::new(format!("s{index}-{}", rng.json_string())).unwrap())
            .collect::<Vec<_>>();
        let pointers = strings
            .iter()
            .map(|value| value.as_ptr())
            .collect::<Vec<_>>();
        let run = |api: &Api| unsafe {
            let create: unsafe extern "C" fn(*const *const c_char, c_int) -> *mut Json =
                api.symbol(b"cJSON_CreateStringArray");
            let item = create(pointers.as_ptr(), pointers.len() as c_int);
            let output = api.render(item, false).unwrap();
            api.delete(item);
            output
        };
        assert_eq!(run(&pair.rust), run(&pair.c));
    }
}

fn test_predicates_values_and_references(pair: &Pair) {
    const CONSTRUCTORS: &[(&[u8], c_int)] = &[
        (b"cJSON_CreateNull", NULL),
        (b"cJSON_CreateTrue", TRUE),
        (b"cJSON_CreateFalse", FALSE),
        (b"cJSON_CreateArray", ARRAY),
        (b"cJSON_CreateObject", OBJECT),
    ];
    const PREDICATES: &[(&[u8], c_int)] = &[
        (b"cJSON_IsInvalid", INVALID),
        (b"cJSON_IsFalse", FALSE),
        (b"cJSON_IsTrue", TRUE),
        (b"cJSON_IsNull", NULL),
        (b"cJSON_IsNumber", NUMBER),
        (b"cJSON_IsString", STRING),
        (b"cJSON_IsArray", ARRAY),
        (b"cJSON_IsObject", OBJECT),
        (b"cJSON_IsRaw", RAW),
    ];
    for api in [&pair.c, &pair.rust] {
        for &(constructor_name, kind) in CONSTRUCTORS {
            let constructor: unsafe extern "C" fn() -> *mut Json =
                unsafe { api.symbol(constructor_name) };
            let item = unsafe { constructor() };
            for &(predicate_name, expected_kind) in PREDICATES {
                let predicate: unsafe extern "C" fn(*const Json) -> c_int =
                    unsafe { api.symbol(predicate_name) };
                assert_eq!(unsafe { predicate(item) }, (kind == expected_kind) as c_int);
            }
            unsafe {
                (*item).kind |= IS_REFERENCE | STRING_IS_CONST;
            }
            let matching: unsafe extern "C" fn(*const Json) -> c_int =
                unsafe { api.symbol(PREDICATES.iter().find(|entry| entry.1 == kind).unwrap().0) };
            assert_eq!(unsafe { matching(item) }, 1);
            unsafe { api.delete(item) };
        }

        let bool_create: unsafe extern "C" fn(c_int) -> *mut Json =
            unsafe { api.symbol(b"cJSON_CreateBool") };
        let is_bool: unsafe extern "C" fn(*const Json) -> c_int =
            unsafe { api.symbol(b"cJSON_IsBool") };
        for value in [0, 1, -1, 2, c_int::MAX] {
            let item = unsafe { bool_create(value) };
            assert_eq!(unsafe { is_bool(item) }, 1);
            assert_eq!(
                unsafe { (*item).kind & 0xff },
                if value == 0 { FALSE } else { TRUE }
            );
            unsafe { api.delete(item) };
        }

        let text = CString::new("reference").unwrap();
        let create_reference: unsafe extern "C" fn(*const c_char) -> *mut Json =
            unsafe { api.symbol(b"cJSON_CreateStringReference") };
        let reference = unsafe { create_reference(text.as_ptr()) };
        assert_eq!(
            unsafe { (*reference).valuestring },
            text.as_ptr().cast_mut()
        );
        unsafe { api.delete(reference) };

        let child = unsafe { create_number(api, 7.0) };
        for name in [
            b"cJSON_CreateArrayReference".as_slice(),
            b"cJSON_CreateObjectReference",
        ] {
            let create_reference: unsafe extern "C" fn(*const Json) -> *mut Json =
                unsafe { api.symbol(name) };
            let reference = unsafe { create_reference(child) };
            assert_eq!(unsafe { (*reference).child }, child);
            unsafe { api.delete(reference) };
        }
        unsafe { api.delete(child) };
    }
}

fn test_parsers_and_errors(pair: &Pair) {
    let malformed: &[&[u8]] = &[
        b"",
        b" ",
        b"x",
        b"-",
        b"\"",
        b"\"abc",
        b"\"abc\\",
        b"\"\\q\"",
        b"\"\\u12\"",
        b"\"\\udc00\"",
        b"\"\\ud800\"",
        b"\"\\ud800xxxxxx\"",
        b"\"\\ud800\\u0041\"",
        b"[",
        b"[,",
        b"[1,]",
        b"[1 2]",
        b"{",
        b"{,",
        b"{a:1}",
        b"{\"a\" 1}",
        b"{\"a\":}",
        b"{\"a\":1,}",
        b"{\"a\":1 \"b\":2}",
    ];
    for &input in malformed {
        let mut bytes = input.to_vec();
        bytes.push(0);
        let mut c_end = ptr::null();
        let mut rust_end = ptr::null();
        let c = unsafe { pair.c.parse_bytes(&bytes, bytes.len(), Some(&mut c_end), 1) };
        let rust = unsafe {
            pair.rust
                .parse_bytes(&bytes, bytes.len(), Some(&mut rust_end), 1)
        };
        assert_eq!(rust.is_null(), c.is_null(), "malformed {:?}", input);
        let c_offset = unsafe { c_end.offset_from(bytes.as_ptr().cast()) };
        let rust_offset = unsafe { rust_end.offset_from(bytes.as_ptr().cast()) };
        assert_eq!(rust_offset, c_offset, "error offset for {:?}", input);
        unsafe {
            pair.c.delete(c);
            pair.rust.delete(rust);
        }
    }

    for require in [0, 1, -1, 2] {
        for text in [b"true\0".as_slice(), b"true \0", b"true x\0"] {
            let mut c_end = ptr::null();
            let mut rust_end = ptr::null();
            let c = unsafe {
                pair.c
                    .parse_bytes(text, text.len(), Some(&mut c_end), require)
            };
            let rust = unsafe {
                pair.rust
                    .parse_bytes(text, text.len(), Some(&mut rust_end), require)
            };
            assert_eq!(rust.is_null(), c.is_null(), "require={require}, {text:?}");
            assert_eq!(
                unsafe { rust_end.offset_from(text.as_ptr().cast()) },
                unsafe { c_end.offset_from(text.as_ptr().cast()) }
            );
            unsafe {
                pair.c.delete(c);
                pair.rust.delete(rust);
            }
        }
    }

    let mut deep_array = vec![b'['; 1001];
    deep_array.extend(std::iter::repeat_n(b']', 1001));
    deep_array.push(0);
    let mut deep_object = Vec::new();
    for _ in 0..1001 {
        deep_object.extend_from_slice(b"{\"x\":");
    }
    deep_object.extend(std::iter::repeat_n(b'}', 1001));
    deep_object.push(0);
    for input in [&deep_array, &deep_object] {
        let c = unsafe { pair.c.parse_bytes(input, input.len(), None, 1) };
        let rust = unsafe { pair.rust.parse_bytes(input, input.len(), None, 1) };
        assert_eq!(rust.is_null(), c.is_null());
        unsafe {
            pair.c.delete(c);
            pair.rust.delete(rust);
        }
    }

    for api in [&pair.c, &pair.rust] {
        let parse: unsafe extern "C" fn(*const c_char) -> *mut Json =
            unsafe { api.symbol(b"cJSON_Parse") };
        let parse_opts: unsafe extern "C" fn(
            *const c_char,
            *mut *const c_char,
            c_int,
        ) -> *mut Json = unsafe { api.symbol(b"cJSON_ParseWithOpts") };
        let parse_length: unsafe extern "C" fn(*const c_char, usize) -> *mut Json =
            unsafe { api.symbol(b"cJSON_ParseWithLength") };
        assert!(unsafe { parse(ptr::null()) }.is_null());
        assert!(unsafe { parse_opts(ptr::null(), ptr::null_mut(), 0) }.is_null());
        assert!(unsafe { parse_length(ptr::null(), 5) }.is_null());
        let value = b"true\0";
        assert!(unsafe { parse_length(value.as_ptr().cast(), 0) }.is_null());
    }

    for input in [
        b"\xef\xbb\xbftrue\0".as_slice(),
        b"\xef\xbb\xbftrue",
        b" \xef\xbb\xbftrue\0",
    ] {
        let c = unsafe { pair.c.parse_bytes(input, input.len(), None, 0) };
        let rust = unsafe { pair.rust.parse_bytes(input, input.len(), None, 0) };
        assert_eq!(rust.is_null(), c.is_null(), "BOM case {input:?}");
        unsafe {
            pair.c.delete(c);
            pair.rust.delete(rust);
        }
    }
}

fn test_print_modes(pair: &Pair) {
    let input = br#"{"a":[1,2,{"text":"x\n\t\u0001"}],"raw":true}"#;
    for api in [&pair.c, &pair.rust] {
        let mut bytes = input.to_vec();
        bytes.push(0);
        let parse: unsafe extern "C" fn(*const c_char) -> *mut Json =
            unsafe { api.symbol(b"cJSON_Parse") };
        let item = unsafe { parse(bytes.as_ptr().cast()) };
        assert!(!item.is_null());

        for format in [0, 1, -1, 2] {
            for prebuffer in [0, 1, 8, 64, 256, 1024] {
                let buffered: unsafe extern "C" fn(*const Json, c_int, c_int) -> *mut c_char =
                    unsafe { api.symbol(b"cJSON_PrintBuffered") };
                let output = unsafe { buffered(item, prebuffer, format) };
                assert!(!output.is_null());
                let actual = unsafe { CStr::from_ptr(output) }.to_bytes().to_vec();
                let expected = unsafe { api.render(item, format != 0) }.unwrap();
                assert_eq!(actual, expected);
                unsafe { api.free(output.cast()) };
            }
        }
        unsafe { api.delete(item) };
    }

    let run_preallocated = |api: &Api, format: c_int, length: usize| unsafe {
        let mut bytes = input.to_vec();
        bytes.push(0);
        let parse: unsafe extern "C" fn(*const c_char) -> *mut Json = api.symbol(b"cJSON_Parse");
        let item = parse(bytes.as_ptr().cast());
        let mut output = vec![0x55u8; length.max(1)];
        let print: unsafe extern "C" fn(*mut Json, *mut c_char, c_int, c_int) -> c_int =
            api.symbol(b"cJSON_PrintPreallocated");
        let result = print(item, output.as_mut_ptr().cast(), length as c_int, format);
        let value = if result != 0 {
            Some(CStr::from_ptr(output.as_ptr().cast()).to_bytes().to_vec())
        } else {
            None
        };
        api.delete(item);
        (result, value)
    };
    for format in [0, 1, -1, 2] {
        for length in 0..180 {
            assert_eq!(
                run_preallocated(&pair.rust, format, length),
                run_preallocated(&pair.c, format, length),
                "preallocated format={format} length={length}"
            );
        }
    }
}

fn test_duplicate_compare_minify(pair: &Pair) {
    let input = CString::new(r#"{"A":[1,2,{"x":"y"}],"ref":"value"}"#).unwrap();
    for api in [&pair.c, &pair.rust] {
        let parse: unsafe extern "C" fn(*const c_char) -> *mut Json =
            unsafe { api.symbol(b"cJSON_Parse") };
        let duplicate: unsafe extern "C" fn(*const Json, c_int) -> *mut Json =
            unsafe { api.symbol(b"cJSON_Duplicate") };
        let compare: unsafe extern "C" fn(*const Json, *const Json, c_int) -> c_int =
            unsafe { api.symbol(b"cJSON_Compare") };
        let item = unsafe { parse(input.as_ptr()) };
        for recurse in [0, 1, -1, 2] {
            let copy = unsafe { duplicate(item, recurse) };
            assert!(!copy.is_null());
            let expected = if recurse == 0 { 0 } else { 1 };
            assert_eq!(unsafe { compare(item, copy, 1) }, expected);
            unsafe { api.delete(copy) };
        }
        assert_eq!(unsafe { compare(item, item, 1) }, 1);
        assert_eq!(unsafe { compare(item, ptr::null(), 1) }, 0);
        unsafe { api.delete(item) };

        let upper = CString::new(r#"{"Key":1}"#).unwrap();
        let lower = CString::new(r#"{"key":1}"#).unwrap();
        let a = unsafe { parse(upper.as_ptr()) };
        let b = unsafe { parse(lower.as_ptr()) };
        assert_eq!(unsafe { compare(a, b, 0) }, 1);
        for mode in [1, -1, 2] {
            assert_eq!(unsafe { compare(a, b, mode) }, 0);
        }
        unsafe {
            api.delete(a);
            api.delete(b);
        }
    }

    let minify_cases = [
        " { \"a\" : 1, \"s\" : \" // not comment \" } ",
        "// head\n[ 1, 2 ]",
        "/* block */ {\"x\":/* middle */true}",
        "{\"quote\":\"a\\\\\\\" b\", \"slash\":\"/*\"}",
        "/* unterminated",
    ];
    for input in minify_cases {
        let run = |api: &Api| unsafe {
            let minify: unsafe extern "C" fn(*mut c_char) = api.symbol(b"cJSON_Minify");
            let mut value = CString::new(input).unwrap().into_bytes_with_nul();
            minify(value.as_mut_ptr().cast());
            CStr::from_ptr(value.as_ptr().cast()).to_bytes().to_vec()
        };
        assert_eq!(run(&pair.rust), run(&pair.c), "minify {input:?}");
    }
}

fn test_rejections(pair: &Pair) {
    for api in [&pair.c, &pair.rust] {
        let print: unsafe extern "C" fn(*const Json) -> *mut c_char =
            unsafe { api.symbol(b"cJSON_Print") };
        let print_unformatted: unsafe extern "C" fn(*const Json) -> *mut c_char =
            unsafe { api.symbol(b"cJSON_PrintUnformatted") };
        let print_buffered: unsafe extern "C" fn(*const Json, c_int, c_int) -> *mut c_char =
            unsafe { api.symbol(b"cJSON_PrintBuffered") };
        let print_preallocated: unsafe extern "C" fn(
            *mut Json,
            *mut c_char,
            c_int,
            c_int,
        ) -> c_int = unsafe { api.symbol(b"cJSON_PrintPreallocated") };
        assert!(unsafe { print(ptr::null()) }.is_null());
        assert!(unsafe { print_unformatted(ptr::null()) }.is_null());
        assert!(unsafe { print_buffered(ptr::null(), -1, 0) }.is_null());
        assert_eq!(
            unsafe { print_preallocated(ptr::null_mut(), ptr::null_mut(), 1, 0) },
            0
        );
        let mut tiny = [0u8; 1];
        assert_eq!(
            unsafe { print_preallocated(ptr::null_mut(), tiny.as_mut_ptr().cast(), -1, 0) },
            0
        );

        let get_size: unsafe extern "C" fn(*const Json) -> c_int =
            unsafe { api.symbol(b"cJSON_GetArraySize") };
        let get_item: unsafe extern "C" fn(*const Json, c_int) -> *mut Json =
            unsafe { api.symbol(b"cJSON_GetArrayItem") };
        assert_eq!(unsafe { get_size(ptr::null()) }, 0);
        assert!(unsafe { get_item(ptr::null(), 0) }.is_null());
        let array = unsafe { create_container(api, false) };
        assert!(unsafe { get_item(array, -1) }.is_null());
        assert!(unsafe { get_item(array, 0) }.is_null());

        let add: unsafe extern "C" fn(*mut Json, *mut Json) -> c_int =
            unsafe { api.symbol(b"cJSON_AddItemToArray") };
        assert_eq!(unsafe { add(ptr::null_mut(), ptr::null_mut()) }, 0);
        assert_eq!(unsafe { add(array, array) }, 0);
        let insert: unsafe extern "C" fn(*mut Json, c_int, *mut Json) -> c_int =
            unsafe { api.symbol(b"cJSON_InsertItemInArray") };
        assert_eq!(unsafe { insert(array, -1, ptr::null_mut()) }, 0);
        assert_eq!(unsafe { insert(ptr::null_mut(), 0, ptr::null_mut()) }, 0);
        let replace: unsafe extern "C" fn(*mut Json, *mut Json, *mut Json) -> c_int =
            unsafe { api.symbol(b"cJSON_ReplaceItemViaPointer") };
        assert_eq!(
            unsafe { replace(ptr::null_mut(), ptr::null_mut(), ptr::null_mut()) },
            0
        );

        let first = unsafe { create_number(api, 1.0) };
        let second = unsafe { create_number(api, 2.0) };
        let third = unsafe { create_number(api, 3.0) };
        assert_eq!(unsafe { add(array, first) }, 1);
        assert_eq!(unsafe { add(array, second) }, 1);
        assert_eq!(unsafe { add(array, third) }, 1);
        unsafe { (*second).prev = ptr::null_mut() };
        let extra = unsafe { create_number(api, 4.0) };
        assert_eq!(unsafe { insert(array, 1, extra) }, 0);
        unsafe { api.delete(extra) };
        unsafe { (*second).prev = first };

        let detach: unsafe extern "C" fn(*mut Json, *mut Json) -> *mut Json =
            unsafe { api.symbol(b"cJSON_DetachItemViaPointer") };
        assert!(unsafe { detach(ptr::null_mut(), first) }.is_null());
        assert!(unsafe { detach(array, ptr::null_mut()) }.is_null());
        unsafe { api.delete(array) };

        for name in [
            b"cJSON_CreateIntArray".as_slice(),
            b"cJSON_CreateFloatArray",
            b"cJSON_CreateDoubleArray",
            b"cJSON_CreateStringArray",
        ] {
            let create: unsafe extern "C" fn(*const c_void, c_int) -> *mut Json =
                unsafe { api.symbol(name) };
            assert!(unsafe { create(ptr::null(), 0) }.is_null());
            assert!(unsafe { create(ptr::null(), -1) }.is_null());
        }

        let create_string: unsafe extern "C" fn(*const c_char) -> *mut Json =
            unsafe { api.symbol(b"cJSON_CreateString") };
        let create_raw: unsafe extern "C" fn(*const c_char) -> *mut Json =
            unsafe { api.symbol(b"cJSON_CreateRaw") };
        assert!(unsafe { create_string(ptr::null()) }.is_null());
        assert!(unsafe { create_raw(ptr::null()) }.is_null());

        let set_string: unsafe extern "C" fn(*mut Json, *const c_char) -> *mut c_char =
            unsafe { api.symbol(b"cJSON_SetValuestring") };
        let replacement = CString::new("replacement").unwrap();
        assert!(unsafe { set_string(ptr::null_mut(), replacement.as_ptr()) }.is_null());
        let number = unsafe { create_number(api, 1.0) };
        assert!(unsafe { set_string(number, replacement.as_ptr()) }.is_null());
        unsafe { api.delete(number) };
        let source = CString::new("source").unwrap();
        let create_reference: unsafe extern "C" fn(*const c_char) -> *mut Json =
            unsafe { api.symbol(b"cJSON_CreateStringReference") };
        let reference = unsafe { create_reference(source.as_ptr()) };
        assert!(unsafe { set_string(reference, replacement.as_ptr()) }.is_null());
        unsafe { api.delete(reference) };
        let string = unsafe { create_string(source.as_ptr()) };
        assert!(unsafe { set_string(string, ptr::null()) }.is_null());
        let owned = unsafe { (*string).valuestring };
        unsafe {
            api.free(owned.cast());
            (*string).valuestring = ptr::null_mut();
        }
        assert!(unsafe { set_string(string, replacement.as_ptr()) }.is_null());
        unsafe { api.delete(string) };

        let object = unsafe { create_container(api, true) };
        let key = CString::new("key").unwrap();
        let item = unsafe { create_number(api, 1.0) };
        for name in [
            b"cJSON_AddItemToObject".as_slice(),
            b"cJSON_AddItemToObjectCS",
        ] {
            let add_object: unsafe extern "C" fn(*mut Json, *const c_char, *mut Json) -> c_int =
                unsafe { api.symbol(name) };
            assert_eq!(
                unsafe { add_object(ptr::null_mut(), key.as_ptr(), item) },
                0
            );
            assert_eq!(unsafe { add_object(object, ptr::null(), item) }, 0);
            assert_eq!(
                unsafe { add_object(object, key.as_ptr(), ptr::null_mut()) },
                0
            );
            assert_eq!(unsafe { add_object(object, key.as_ptr(), object) }, 0);
        }
        let add_reference: unsafe extern "C" fn(*mut Json, *const c_char, *mut Json) -> c_int =
            unsafe { api.symbol(b"cJSON_AddItemReferenceToObject") };
        assert_eq!(
            unsafe { add_reference(ptr::null_mut(), key.as_ptr(), item) },
            0
        );
        assert_eq!(unsafe { add_reference(object, ptr::null(), item) }, 0);
        assert_eq!(
            unsafe { add_reference(object, key.as_ptr(), ptr::null_mut()) },
            0
        );
        let get_object: unsafe extern "C" fn(*const Json, *const c_char) -> *mut Json =
            unsafe { api.symbol(b"cJSON_GetObjectItem") };
        assert!(unsafe { get_object(ptr::null(), key.as_ptr()) }.is_null());
        assert!(unsafe { get_object(object, ptr::null()) }.is_null());
        let replace_object: unsafe extern "C" fn(*mut Json, *const c_char, *mut Json) -> c_int =
            unsafe { api.symbol(b"cJSON_ReplaceItemInObject") };
        assert_eq!(unsafe { replace_object(object, ptr::null(), item) }, 0);
        assert_eq!(
            unsafe { replace_object(object, key.as_ptr(), ptr::null_mut()) },
            0
        );
        assert_eq!(unsafe { replace_object(object, key.as_ptr(), item) }, 0);
        unsafe {
            api.delete(item);
            api.delete(object);
        }

        let get_string: unsafe extern "C" fn(*const Json) -> *mut c_char =
            unsafe { api.symbol(b"cJSON_GetStringValue") };
        let get_number: unsafe extern "C" fn(*const Json) -> f64 =
            unsafe { api.symbol(b"cJSON_GetNumberValue") };
        assert!(unsafe { get_string(ptr::null()) }.is_null());
        assert!(unsafe { get_number(ptr::null()) }.is_nan());

        let duplicate: unsafe extern "C" fn(*const Json, c_int) -> *mut Json =
            unsafe { api.symbol(b"cJSON_Duplicate") };
        let compare: unsafe extern "C" fn(*const Json, *const Json, c_int) -> c_int =
            unsafe { api.symbol(b"cJSON_Compare") };
        assert!(unsafe { duplicate(ptr::null(), 1) }.is_null());
        assert_eq!(unsafe { compare(ptr::null(), ptr::null(), 0) }, 0);

        let mut invalid = Json {
            next: ptr::null_mut(),
            prev: ptr::null_mut(),
            child: ptr::null_mut(),
            kind: 0x7f,
            valuestring: ptr::null_mut(),
            valueint: 0,
            valuedouble: 0.0,
            string: ptr::null_mut(),
        };
        let mut buffer = [0u8; 32];
        assert!(unsafe { print(&invalid) }.is_null());
        assert_eq!(
            unsafe { print_preallocated(&mut invalid, buffer.as_mut_ptr().cast(), 32, 0) },
            0
        );
        invalid.kind = RAW;
        assert!(unsafe { print(&invalid) }.is_null());

        for name in [
            b"cJSON_IsInvalid".as_slice(),
            b"cJSON_IsFalse",
            b"cJSON_IsTrue",
            b"cJSON_IsBool",
            b"cJSON_IsNull",
            b"cJSON_IsNumber",
            b"cJSON_IsString",
            b"cJSON_IsArray",
            b"cJSON_IsObject",
            b"cJSON_IsRaw",
        ] {
            let predicate: unsafe extern "C" fn(*const Json) -> c_int = unsafe { api.symbol(name) };
            assert_eq!(unsafe { predicate(ptr::null()) }, 0);
        }

        let minify: unsafe extern "C" fn(*mut c_char) = unsafe { api.symbol(b"cJSON_Minify") };
        unsafe { minify(ptr::null_mut()) };
        unsafe {
            api.delete(ptr::null_mut());
            api.free(ptr::null_mut());
        }
    }
}

fn test_driver(pair: &Pair) {
    let strings = [
        "Sunday",
        "Monday",
        "Tuesday",
        "Wednesday",
        "Thursday",
        "Friday",
        "Saturday",
    ]
    .map(|value| CString::new(value).unwrap());
    let string_pointers = strings.map(|value| value.as_ptr());
    let mut numbers = [1, 2, 3, 4, 5, 6, 7, 8, 9];
    let mut ids = [116, 943, 234, 38793];
    let text = [
        CString::new("zip").unwrap(),
        CString::new("float").unwrap(),
        CString::new("500").unwrap(),
        CString::new("SAN FRANCISCO").unwrap(),
        CString::new("CA").unwrap(),
        CString::new("94107").unwrap(),
        CString::new("US").unwrap(),
        CString::new("double").unwrap(),
        CString::new("Avenue").unwrap(),
        CString::new("LOS ANGELES").unwrap(),
        CString::new("90001").unwrap(),
    ];
    let mut records = [
        Record {
            precision: text[1].as_ptr(),
            lat: 37.7668,
            lon: -122.3959,
            address: text[2].as_ptr(),
            city: text[3].as_ptr(),
            state: text[4].as_ptr(),
            zip: text[5].as_ptr(),
            country: text[6].as_ptr(),
        },
        Record {
            precision: text[7].as_ptr(),
            lat: 34.0522,
            lon: -118.2437,
            address: text[8].as_ptr(),
            city: text[9].as_ptr(),
            state: text[4].as_ptr(),
            zip: text[10].as_ptr(),
            country: text[6].as_ptr(),
        },
    ];
    type Driver =
        unsafe extern "C" fn(*const *const c_char, *mut c_int, *mut c_int, *mut Record) -> c_int;
    let c_driver: Driver = unsafe { pair.c_driver.symbol(b"driver") };
    let rust_driver: Driver = unsafe { pair.rust.symbol(b"driver") };
    let c = unsafe {
        capture_stdout(|| {
            c_driver(
                string_pointers.as_ptr(),
                numbers.as_mut_ptr(),
                ids.as_mut_ptr(),
                records.as_mut_ptr(),
            )
        })
    };
    let rust = unsafe {
        capture_stdout(|| {
            rust_driver(
                string_pointers.as_ptr(),
                numbers.as_mut_ptr(),
                ids.as_mut_ptr(),
                records.as_mut_ptr(),
            )
        })
    };
    assert_eq!(rust, c, "driver return/stdout mismatch");
}
