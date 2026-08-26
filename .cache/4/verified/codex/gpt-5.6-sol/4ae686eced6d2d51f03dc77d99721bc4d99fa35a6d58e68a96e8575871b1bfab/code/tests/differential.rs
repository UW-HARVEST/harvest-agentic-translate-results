#![allow(unsafe_op_in_unsafe_fn)]

use libloading::Library;
use std::ffi::{CStr, CString, c_char, c_double, c_float, c_int, c_void};
use std::path::{Path, PathBuf};
use std::ptr;
use std::sync::Mutex;
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
struct CJson {
    next: *mut CJson,
    prev: *mut CJson,
    child: *mut CJson,
    type_: c_int,
    valuestring: *mut c_char,
    valueint: c_int,
    valuedouble: c_double,
    string: *mut c_char,
}

impl Default for CJson {
    fn default() -> Self {
        Self {
            next: ptr::null_mut(),
            prev: ptr::null_mut(),
            child: ptr::null_mut(),
            type_: INVALID,
            valuestring: ptr::null_mut(),
            valueint: 0,
            valuedouble: 0.0,
            string: ptr::null_mut(),
        }
    }
}

type AllocFn = unsafe extern "C" fn(usize) -> *mut c_void;
type FreeFn = unsafe extern "C" fn(*mut c_void);

#[repr(C)]
struct Hooks {
    malloc_fn: Option<AllocFn>,
    free_fn: Option<FreeFn>,
}

unsafe extern "C" {
    #[link_name = "malloc"]
    fn libc_malloc(size: usize) -> *mut c_void;
    #[link_name = "free"]
    fn libc_free(pointer: *mut c_void);
    fn close(fd: c_int) -> c_int;
    fn dup(fd: c_int) -> c_int;
    fn dup2(old_fd: c_int, new_fd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
    fn pipe(fds: *mut c_int) -> c_int;
    fn read(fd: c_int, buffer: *mut c_void, count: usize) -> isize;
}

static FAIL_AFTER: AtomicIsize = AtomicIsize::new(-1);
static TEST_LOCK: Mutex<()> = Mutex::new(());

unsafe extern "C" fn passthrough_malloc(size: usize) -> *mut c_void {
    libc_malloc(size)
}

unsafe extern "C" fn passthrough_free(pointer: *mut c_void) {
    libc_free(pointer)
}

unsafe extern "C" fn fail_malloc(size: usize) -> *mut c_void {
    let remaining = FAIL_AFTER.fetch_sub(1, Ordering::SeqCst);
    if remaining == 0 {
        ptr::null_mut()
    } else {
        libc_malloc(size)
    }
}

struct Api {
    _library: Library,
    label: &'static str,
}

impl Api {
    unsafe fn open(path: &Path, label: &'static str) -> Self {
        Self {
            _library: Library::new(path)
                .unwrap_or_else(|error| panic!("load {label} {}: {error}", path.display())),
            label,
        }
    }

    unsafe fn symbol<T: Copy>(&self, name: &[u8]) -> T {
        *self
            ._library
            .get::<T>(name)
            .unwrap_or_else(|error| panic!("{} lacks {:?}: {error}", self.label, name))
    }

    unsafe fn init_hooks(&self, hooks: *mut Hooks) {
        let function: unsafe extern "C" fn(*mut Hooks) = self.symbol(b"cJSON_InitHooks\0");
        function(hooks);
    }

    unsafe fn parse(&self, input: &CString) -> *mut CJson {
        let function: unsafe extern "C" fn(*const c_char) -> *mut CJson =
            self.symbol(b"cJSON_Parse\0");
        function(input.as_ptr())
    }

    unsafe fn delete(&self, item: *mut CJson) {
        let function: unsafe extern "C" fn(*mut CJson) = self.symbol(b"cJSON_Delete\0");
        function(item);
    }

    unsafe fn free(&self, item: *mut c_void) {
        let function: unsafe extern "C" fn(*mut c_void) = self.symbol(b"cJSON_free\0");
        function(item);
    }

    unsafe fn print_with(&self, item: *const CJson, symbol: &[u8]) -> Option<Vec<u8>> {
        let function: unsafe extern "C" fn(*const CJson) -> *mut c_char = self.symbol(symbol);
        let output = function(item);
        if output.is_null() {
            return None;
        }
        let bytes = CStr::from_ptr(output).to_bytes().to_vec();
        self.free(output.cast());
        Some(bytes)
    }

    unsafe fn print(&self, item: *const CJson) -> Option<Vec<u8>> {
        self.print_with(item, b"cJSON_Print\0")
    }

    unsafe fn print_unformatted(&self, item: *const CJson) -> Option<Vec<u8>> {
        self.print_with(item, b"cJSON_PrintUnformatted\0")
    }
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_library() -> PathBuf {
    manifest_dir().join("c_src/build/libcjson.so")
}

fn rust_library() -> PathBuf {
    manifest_dir().join("target/debug/libcJSON_test.so")
}

fn c_string(value: &str) -> CString {
    CString::new(value).unwrap()
}

fn text(bytes: Option<Vec<u8>>) -> String {
    match bytes {
        Some(bytes) => String::from_utf8_lossy(&bytes).into_owned(),
        None => "<NULL>".to_owned(),
    }
}

unsafe fn c_string_at(pointer: *const c_char) -> String {
    if pointer.is_null() {
        "<NULL>".to_owned()
    } else {
        String::from_utf8_lossy(CStr::from_ptr(pointer).to_bytes()).into_owned()
    }
}

fn push(result: &mut Vec<String>, id: &str, value: impl std::fmt::Display) {
    result.push(format!("{id}:{value}"));
}

fn generated_json() -> Vec<String> {
    let mut state = 0x5eed_cafe_1234_5678u64;
    let mut next = || {
        state = state
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        state
    };
    let mut values = vec![
        "null".to_owned(),
        "false".to_owned(),
        "true".to_owned(),
        "0".to_owned(),
        "-0".to_owned(),
        "1.5".to_owned(),
        "-2.75e+12".to_owned(),
        "2147483647".to_owned(),
        "2147483648".to_owned(),
        "-2147483649".to_owned(),
        "1e309".to_owned(),
        "1e-400".to_owned(),
        "\"\"".to_owned(),
        r#""quote:\" slash:\\ controls:\b\f\n\r\t""#.to_owned(),
        r#""\u0041\u00df\u6771\ud834\udd1e""#.to_owned(),
        "[]".to_owned(),
        "{}".to_owned(),
        "[null,true,false,0,\"x\",[],{}]".to_owned(),
        "{\"a\":1,\"A\":2,\"a\":3}".to_owned(),
        "\u{feff} {\"bom\":true}".to_owned(),
    ];

    for index in 0..128 {
        let a = (next() as i64) >> 9;
        let b = next() % 100_000;
        let letters: String = (0..(next() % 12))
            .map(|_| (b'a' + (next() % 26) as u8) as char)
            .collect();
        match index % 5 {
            0 => values.push(format!("{a}")),
            1 => values.push(format!("{}.{}e{}", a % 10_000, b, (a % 40) - 20)),
            2 => values.push(format!(r#"["{letters}",{},{}]"#, a, b)),
            3 => values.push(format!(
                r#"{{"k{index}":"{letters}","n":{},"b":{}}}"#,
                a,
                if next() & 1 == 0 { "true" } else { "false" }
            )),
            _ => values.push(format!(
                r#"{{"outer":[{{"x":{}}},[],["{}"]],"tail":null}}"#,
                b, letters
            )),
        }
    }
    values
}

unsafe fn set_failure_hook(api: &Api, fail_after: isize) {
    FAIL_AFTER.store(fail_after, Ordering::SeqCst);
    let mut hooks = Hooks {
        malloc_fn: Some(fail_malloc),
        free_fn: Some(passthrough_free),
    };
    api.init_hooks(&mut hooks);
}

unsafe fn reset_hooks(api: &Api) {
    api.init_hooks(ptr::null_mut());
    FAIL_AFTER.store(-1, Ordering::SeqCst);
}

unsafe fn run_valid(api: &Api) -> Vec<String> {
    let mut result = Vec::new();

    let version: unsafe extern "C" fn() -> *const c_char = api.symbol(b"cJSON_Version\0");
    push(&mut result, "C001", c_string_at(version()));

    let malloc_fn: unsafe extern "C" fn(usize) -> *mut c_void = api.symbol(b"cJSON_malloc\0");
    for size in [0usize, 1, 17, 4096] {
        let allocation = malloc_fn(size);
        push(
            &mut result,
            "C002",
            format!("{size}:{}", allocation.is_null()),
        );
        if !allocation.is_null() {
            api.free(allocation);
        }
    }
    let hook_variants = [
        Hooks {
            malloc_fn: Some(passthrough_malloc),
            free_fn: Some(passthrough_free),
        },
        Hooks {
            malloc_fn: Some(passthrough_malloc),
            free_fn: None,
        },
        Hooks {
            malloc_fn: None,
            free_fn: Some(passthrough_free),
        },
    ];
    for mut hooks in hook_variants {
        api.init_hooks(&mut hooks);
        let item = api.parse(&c_string(r#"{"hook":[1,2,3],"s":"value"}"#));
        push(&mut result, "C003", text(api.print_unformatted(item)));
        api.delete(item);
        reset_hooks(api);
    }

    let get_size: unsafe extern "C" fn(*const CJson) -> c_int = api.symbol(b"cJSON_GetArraySize\0");
    let get_string: unsafe extern "C" fn(*const CJson) -> *mut c_char =
        api.symbol(b"cJSON_GetStringValue\0");
    let get_number: unsafe extern "C" fn(*const CJson) -> c_double =
        api.symbol(b"cJSON_GetNumberValue\0");
    let duplicate: unsafe extern "C" fn(*const CJson, c_int) -> *mut CJson =
        api.symbol(b"cJSON_Duplicate\0");
    let compare: unsafe extern "C" fn(*const CJson, *const CJson, c_int) -> c_int =
        api.symbol(b"cJSON_Compare\0");
    let print_buffered: unsafe extern "C" fn(*const CJson, c_int, c_int) -> *mut c_char =
        api.symbol(b"cJSON_PrintBuffered\0");
    let print_preallocated: unsafe extern "C" fn(*mut CJson, *mut c_char, c_int, c_int) -> c_int =
        api.symbol(b"cJSON_PrintPreallocated\0");

    for (index, input) in generated_json().iter().enumerate() {
        let item = api.parse(&c_string(input));
        push(
            &mut result,
            "C005,C006,C007,C008,C009,C014",
            format!("{index}:{}", item.is_null()),
        );
        if item.is_null() {
            continue;
        }
        push(
            &mut result,
            "C005,C006,C007,C008,C009,C014",
            format!(
                "{index}:{}:{}:{:016x}",
                (*item).type_,
                (*item).valueint,
                (*item).valuedouble.to_bits()
            ),
        );
        push(
            &mut result,
            "C015,C016,C017,C018,C019",
            format!("{index}:{}", text(api.print(item))),
        );
        let compact = api.print_unformatted(item);
        push(
            &mut result,
            "C015,C016,C017,C018,C019",
            format!("{index}:{}", text(compact.clone())),
        );
        push(
            &mut result,
            "C023-C024",
            format!(
                "{index}:size={}:str={}:num={:016x}",
                get_size(item),
                c_string_at(get_string(item)),
                get_number(item).to_bits()
            ),
        );

        if index < 32 {
            for (prebuffer, format) in [(0, 0), (1, 1), (3, -7), (256, 0), (1024, 1)] {
                let output = print_buffered(item, prebuffer, format);
                let value = if output.is_null() {
                    "<NULL>".to_owned()
                } else {
                    let value = c_string_at(output);
                    api.free(output.cast());
                    value
                };
                push(
                    &mut result,
                    "C020",
                    format!("{index}:{prebuffer}:{format}:{value}"),
                );
            }

            let expected = compact.unwrap();
            for extra in [0usize, 5, 31] {
                let mut buffer = vec![0x55u8; expected.len() + 1 + extra];
                let success =
                    print_preallocated(item, buffer.as_mut_ptr().cast(), buffer.len() as c_int, 0);
                let output = if success != 0 {
                    c_string_at(buffer.as_ptr().cast())
                } else {
                    "<FAILED>".to_owned()
                };
                push(
                    &mut result,
                    "C021",
                    format!("{index}:{extra}:{success}:{output}"),
                );
            }
            if !expected.is_empty() {
                let mut buffer = vec![0x55u8; expected.len()];
                let success =
                    print_preallocated(item, buffer.as_mut_ptr().cast(), buffer.len() as c_int, 0);
                push(&mut result, "C022", format!("{index}:{success}"));
            }
        }

        for recurse in [0, 1, -11] {
            let copy = duplicate(item, recurse);
            push(
                &mut result,
                "C066",
                format!(
                    "{index}:{recurse}:{}:{}",
                    compare(item, copy, 1),
                    text(api.print_unformatted(copy))
                ),
            );
            api.delete(copy);
        }
        api.delete(item);
    }

    let parse_opts: unsafe extern "C" fn(*const c_char, *mut *const c_char, c_int) -> *mut CJson =
        api.symbol(b"cJSON_ParseWithOpts\0");
    for require in [0, 1, -4] {
        for source in ["  [1,2]  ", "true trailing"] {
            let input = c_string(source);
            let mut end = ptr::null();
            let item = parse_opts(input.as_ptr(), &mut end, require);
            let offset = if end.is_null() {
                -1
            } else {
                end.offset_from(input.as_ptr()) as isize
            };
            push(
                &mut result,
                "C010-C011",
                format!(
                    "{require}:{source}:{offset}:{}",
                    text(api.print_unformatted(item))
                ),
            );
            api.delete(item);
        }
    }

    let parse_length: unsafe extern "C" fn(*const c_char, usize) -> *mut CJson =
        api.symbol(b"cJSON_ParseWithLength\0");
    let parse_length_opts: unsafe extern "C" fn(
        *const c_char,
        usize,
        *mut *const c_char,
        c_int,
    ) -> *mut CJson = api.symbol(b"cJSON_ParseWithLengthOpts\0");
    let mut bytes = b"[1,2]\0garbage\0".to_vec();
    for length in [0usize, 3, 5, 6, 7, bytes.len()] {
        let item = parse_length(bytes.as_ptr().cast(), length);
        push(
            &mut result,
            "C012",
            format!("{length}:{}", text(api.print_unformatted(item))),
        );
        api.delete(item);
        for require in [0, 1, 19] {
            let mut end = ptr::null();
            let item = parse_length_opts(bytes.as_ptr().cast(), length, &mut end, require);
            let offset = if end.is_null() {
                -1
            } else {
                end.offset_from(bytes.as_ptr().cast()) as isize
            };
            push(
                &mut result,
                "C013",
                format!(
                    "{length}:{require}:{offset}:{}",
                    text(api.print_unformatted(item))
                ),
            );
            api.delete(item);
        }
    }
    bytes.clear();

    let no_arg_constructors = [
        ("null", b"cJSON_CreateNull\0".as_slice()),
        ("true", b"cJSON_CreateTrue\0".as_slice()),
        ("false", b"cJSON_CreateFalse\0".as_slice()),
        ("array", b"cJSON_CreateArray\0".as_slice()),
        ("object", b"cJSON_CreateObject\0".as_slice()),
    ];
    for (name, symbol) in no_arg_constructors {
        let function: unsafe extern "C" fn() -> *mut CJson = api.symbol(symbol);
        let item = function();
        push(
            &mut result,
            "C025-C030",
            format!(
                "{name}:{}:{}",
                (*item).type_,
                text(api.print_unformatted(item))
            ),
        );
        api.delete(item);
    }

    let create_bool: unsafe extern "C" fn(c_int) -> *mut CJson = api.symbol(b"cJSON_CreateBool\0");
    for value in [0, 1, -1, 17] {
        let item = create_bool(value);
        push(
            &mut result,
            "C026",
            format!(
                "{value}:{}:{}",
                (*item).type_,
                text(api.print_unformatted(item))
            ),
        );
        api.delete(item);
    }

    let create_number: unsafe extern "C" fn(c_double) -> *mut CJson =
        api.symbol(b"cJSON_CreateNumber\0");
    let set_number: unsafe extern "C" fn(*mut CJson, c_double) -> c_double =
        api.symbol(b"cJSON_SetNumberHelper\0");
    for value in [
        0.0,
        -0.0,
        1.5,
        c_int::MAX as f64,
        c_int::MAX as f64 + 1.0,
        c_int::MIN as f64 - 1.0,
        f64::NAN,
        f64::INFINITY,
        f64::NEG_INFINITY,
    ] {
        let item = create_number(value);
        let changed = set_number(item, value);
        push(
            &mut result,
            "C027",
            format!(
                "{:016x}:{}:{:016x}:{}",
                value.to_bits(),
                (*item).valueint,
                changed.to_bits(),
                text(api.print_unformatted(item))
            ),
        );
        api.delete(item);
    }

    let create_string: unsafe extern "C" fn(*const c_char) -> *mut CJson =
        api.symbol(b"cJSON_CreateString\0");
    let create_raw: unsafe extern "C" fn(*const c_char) -> *mut CJson =
        api.symbol(b"cJSON_CreateRaw\0");
    let create_string_reference: unsafe extern "C" fn(*const c_char) -> *mut CJson =
        api.symbol(b"cJSON_CreateStringReference\0");
    for value in ["", "plain", "quote\"slash\\", "utf8-\u{6771}"] {
        let value = c_string(value);
        for (name, function) in [("string", create_string), ("raw", create_raw)] {
            let item = function(value.as_ptr());
            push(
                &mut result,
                "C028",
                format!(
                    "{name}:{}:{}",
                    (*item).type_,
                    text(api.print_unformatted(item))
                ),
            );
            api.delete(item);
        }
        let item = create_string_reference(value.as_ptr());
        push(
            &mut result,
            "C029",
            format!("{}:{}", (*item).type_, text(api.print_unformatted(item))),
        );
        api.delete(item);
    }
    let null_reference = create_string_reference(ptr::null());
    push(
        &mut result,
        "C029",
        format!(
            "null:{}:{}",
            (*null_reference).type_,
            text(api.print_unformatted(null_reference))
        ),
    );
    api.delete(null_reference);

    let create_int_array: unsafe extern "C" fn(*const c_int, c_int) -> *mut CJson =
        api.symbol(b"cJSON_CreateIntArray\0");
    let create_float_array: unsafe extern "C" fn(*const c_float, c_int) -> *mut CJson =
        api.symbol(b"cJSON_CreateFloatArray\0");
    let create_double_array: unsafe extern "C" fn(*const c_double, c_int) -> *mut CJson =
        api.symbol(b"cJSON_CreateDoubleArray\0");
    let create_string_array: unsafe extern "C" fn(*const *const c_char, c_int) -> *mut CJson =
        api.symbol(b"cJSON_CreateStringArray\0");
    let integers = [c_int::MIN, -1, 0, 1, c_int::MAX];
    let floats = [f32::NEG_INFINITY, -1.25, 0.0, f32::NAN, f32::INFINITY];
    let doubles = [f64::NEG_INFINITY, -1.25, 0.0, f64::NAN, f64::INFINITY];
    let strings = [c_string(""), c_string("a"), c_string("b\"c")];
    let string_pointers: Vec<_> = strings.iter().map(|value| value.as_ptr()).collect();
    for count in [0, 1, 3, 5] {
        for (id, item) in [
            ("C032", create_int_array(integers.as_ptr(), count)),
            ("C033", create_float_array(floats.as_ptr(), count)),
            ("C034", create_double_array(doubles.as_ptr(), count)),
        ] {
            push(
                &mut result,
                id,
                format!("{count}:{}", text(api.print_unformatted(item))),
            );
            api.delete(item);
        }
    }
    for count in [0, 1, 3] {
        let item = create_string_array(string_pointers.as_ptr(), count);
        push(
            &mut result,
            "C035",
            format!("{count}:{}", text(api.print_unformatted(item))),
        );
        api.delete(item);
    }

    run_valid_mutations(api, &mut result);
    run_valid_minify(api, &mut result);
    push(&mut result, "C004", "delete paths completed");
    push(&mut result, "C074", "parse error pointer paths covered");
    result
}

unsafe fn run_valid_mutations(api: &Api, result: &mut Vec<String>) {
    let create_array: unsafe extern "C" fn() -> *mut CJson = api.symbol(b"cJSON_CreateArray\0");
    let create_object: unsafe extern "C" fn() -> *mut CJson = api.symbol(b"cJSON_CreateObject\0");
    let create_number: unsafe extern "C" fn(c_double) -> *mut CJson =
        api.symbol(b"cJSON_CreateNumber\0");
    let create_string: unsafe extern "C" fn(*const c_char) -> *mut CJson =
        api.symbol(b"cJSON_CreateString\0");
    let add_array: unsafe extern "C" fn(*mut CJson, *mut CJson) -> c_int =
        api.symbol(b"cJSON_AddItemToArray\0");
    let add_object: unsafe extern "C" fn(*mut CJson, *const c_char, *mut CJson) -> c_int =
        api.symbol(b"cJSON_AddItemToObject\0");
    let add_object_cs: unsafe extern "C" fn(*mut CJson, *const c_char, *mut CJson) -> c_int =
        api.symbol(b"cJSON_AddItemToObjectCS\0");
    let get_item: unsafe extern "C" fn(*const CJson, c_int) -> *mut CJson =
        api.symbol(b"cJSON_GetArrayItem\0");
    let get_size: unsafe extern "C" fn(*const CJson) -> c_int = api.symbol(b"cJSON_GetArraySize\0");
    let get_object: unsafe extern "C" fn(*const CJson, *const c_char) -> *mut CJson =
        api.symbol(b"cJSON_GetObjectItem\0");
    let get_object_cs: unsafe extern "C" fn(*const CJson, *const c_char) -> *mut CJson =
        api.symbol(b"cJSON_GetObjectItemCaseSensitive\0");
    let has_object: unsafe extern "C" fn(*const CJson, *const c_char) -> c_int =
        api.symbol(b"cJSON_HasObjectItem\0");

    let array = create_array();
    for value in 0..7 {
        push(
            result,
            "C036",
            add_array(array, create_number(value as f64)),
        );
    }
    push(
        result,
        "C050",
        format!(
            "{}:{}:{}:{}",
            get_size(array),
            (*get_item(array, 0)).valueint,
            (*get_item(array, 3)).valueint,
            (*get_item(array, 6)).valueint
        ),
    );
    push(result, "C036", text(api.print_unformatted(array)));
    api.delete(array);

    let object = create_object();
    let owned_key = c_string("OwnedKey");
    let constant_key = c_string("ConstantKey");
    let reused = create_number(1.0);
    (*reused).string = {
        let old = c_string("old");
        let allocation: unsafe extern "C" fn(usize) -> *mut c_void = api.symbol(b"cJSON_malloc\0");
        let memory = allocation(old.as_bytes_with_nul().len()).cast::<c_char>();
        ptr::copy_nonoverlapping(old.as_ptr(), memory, old.as_bytes_with_nul().len());
        memory
    };
    push(
        result,
        "C037",
        add_object(object, owned_key.as_ptr(), reused),
    );
    push(
        result,
        "C038",
        add_object_cs(object, constant_key.as_ptr(), create_number(2.0)),
    );
    for query in [
        "ownedkey",
        "OWNEDKEY",
        "ConstantKey",
        "constantkey",
        "missing",
    ] {
        let query = c_string(query);
        push(
            result,
            "C051,C052,C053",
            format!(
                "{}:{}:{}",
                !get_object(object, query.as_ptr()).is_null(),
                !get_object_cs(object, query.as_ptr()).is_null(),
                has_object(object, query.as_ptr())
            ),
        );
    }
    push(result, "C037-C038", text(api.print_unformatted(object)));
    api.delete(object);

    let add_ref_array: unsafe extern "C" fn(*mut CJson, *mut CJson) -> c_int =
        api.symbol(b"cJSON_AddItemReferenceToArray\0");
    let add_ref_object: unsafe extern "C" fn(*mut CJson, *const c_char, *mut CJson) -> c_int =
        api.symbol(b"cJSON_AddItemReferenceToObject\0");
    let source = create_string(c_string("borrowed").as_ptr());
    let array = create_array();
    let object = create_object();
    let key = c_string("ref");
    push(result, "C039", add_ref_array(array, source));
    push(result, "C040", add_ref_object(object, key.as_ptr(), source));
    push(
        result,
        "C039-C040",
        format!(
            "{}:{}",
            text(api.print_unformatted(array)),
            text(api.print_unformatted(object))
        ),
    );
    api.delete(array);
    api.delete(object);
    push(result, "C039-C040", text(api.print_unformatted(source)));
    api.delete(source);

    let create_array_ref: unsafe extern "C" fn(*const CJson) -> *mut CJson =
        api.symbol(b"cJSON_CreateArrayReference\0");
    let create_object_ref: unsafe extern "C" fn(*const CJson) -> *mut CJson =
        api.symbol(b"cJSON_CreateObjectReference\0");
    let source_array = api.parse(&c_string("[1,2]"));
    let source_object = api.parse(&c_string(r#"{"x":1}"#));
    for (id, reference) in [
        ("array", create_array_ref((*source_array).child)),
        ("object", create_object_ref((*source_object).child)),
        ("array-null", create_array_ref(ptr::null())),
        ("object-null", create_object_ref(ptr::null())),
    ] {
        push(
            result,
            "C031",
            format!(
                "{id}:{}:{}",
                (*reference).type_,
                text(api.print_unformatted(reference))
            ),
        );
        api.delete(reference);
    }
    api.delete(source_array);
    api.delete(source_object);

    let helper_symbols = [
        ("C041", b"cJSON_AddNullToObject\0".as_slice()),
        ("C042", b"cJSON_AddTrueToObject\0".as_slice()),
        ("C043", b"cJSON_AddFalseToObject\0".as_slice()),
        ("C048", b"cJSON_AddObjectToObject\0".as_slice()),
        ("C049", b"cJSON_AddArrayToObject\0".as_slice()),
    ];
    let object = create_object();
    for (index, (id, symbol)) in helper_symbols.into_iter().enumerate() {
        let function: unsafe extern "C" fn(*mut CJson, *const c_char) -> *mut CJson =
            api.symbol(symbol);
        let key = c_string(&format!("k{index}"));
        let child = function(object, key.as_ptr());
        push(result, id, !child.is_null());
        if id == "C048" {
            add_object(child, c_string("inner").as_ptr(), create_number(1.0));
        } else if id == "C049" {
            add_array(child, create_number(2.0));
        }
    }
    let add_bool: unsafe extern "C" fn(*mut CJson, *const c_char, c_int) -> *mut CJson =
        api.symbol(b"cJSON_AddBoolToObject\0");
    let add_number: unsafe extern "C" fn(*mut CJson, *const c_char, c_double) -> *mut CJson =
        api.symbol(b"cJSON_AddNumberToObject\0");
    let add_string: unsafe extern "C" fn(*mut CJson, *const c_char, *const c_char) -> *mut CJson =
        api.symbol(b"cJSON_AddStringToObject\0");
    let add_raw: unsafe extern "C" fn(*mut CJson, *const c_char, *const c_char) -> *mut CJson =
        api.symbol(b"cJSON_AddRawToObject\0");
    for boolean in [0, 1, -9] {
        push(
            result,
            "C044",
            !add_bool(
                object,
                c_string(&format!("bool{boolean}")).as_ptr(),
                boolean,
            )
            .is_null(),
        );
    }
    for number in [1.25, f64::NAN, f64::INFINITY] {
        push(
            result,
            "C045",
            !add_number(object, c_string("number").as_ptr(), number).is_null(),
        );
    }
    for value in ["", "a\"b", "value"] {
        push(
            result,
            "C046",
            !add_string(
                object,
                c_string("string").as_ptr(),
                c_string(value).as_ptr(),
            )
            .is_null(),
        );
        push(
            result,
            "C047",
            !add_raw(object, c_string("raw").as_ptr(), c_string(value).as_ptr()).is_null(),
        );
    }
    push(result, "C041-C049", text(api.print_unformatted(object)));
    api.delete(object);

    let detach_pointer: unsafe extern "C" fn(*mut CJson, *mut CJson) -> *mut CJson =
        api.symbol(b"cJSON_DetachItemViaPointer\0");
    let detach_array: unsafe extern "C" fn(*mut CJson, c_int) -> *mut CJson =
        api.symbol(b"cJSON_DetachItemFromArray\0");
    let delete_array: unsafe extern "C" fn(*mut CJson, c_int) =
        api.symbol(b"cJSON_DeleteItemFromArray\0");
    for index in [0, 2, 4] {
        let array = api.parse(&c_string("[0,1,2,3,4]"));
        let detached = detach_pointer(array, get_item(array, index));
        push(
            result,
            "C054",
            format!(
                "{index}:{}:{}",
                text(api.print_unformatted(detached)),
                text(api.print_unformatted(array))
            ),
        );
        api.delete(detached);
        api.delete(array);

        let array = api.parse(&c_string("[0,1,2,3,4]"));
        let detached = detach_array(array, index);
        push(
            result,
            "C055",
            format!(
                "{index}:{}:{}",
                text(api.print_unformatted(detached)),
                text(api.print_unformatted(array))
            ),
        );
        api.delete(detached);
        delete_array(array, 0);
        push(result, "C055", text(api.print_unformatted(array)));
        api.delete(array);
    }

    let detach_object: unsafe extern "C" fn(*mut CJson, *const c_char) -> *mut CJson =
        api.symbol(b"cJSON_DetachItemFromObject\0");
    let detach_object_cs: unsafe extern "C" fn(*mut CJson, *const c_char) -> *mut CJson =
        api.symbol(b"cJSON_DetachItemFromObjectCaseSensitive\0");
    let delete_object: unsafe extern "C" fn(*mut CJson, *const c_char) =
        api.symbol(b"cJSON_DeleteItemFromObject\0");
    let delete_object_cs: unsafe extern "C" fn(*mut CJson, *const c_char) =
        api.symbol(b"cJSON_DeleteItemFromObjectCaseSensitive\0");
    for case_sensitive in [false, true] {
        let object = api.parse(&c_string(r#"{"Alpha":1,"Beta":2}"#));
        let query = c_string(if case_sensitive { "Alpha" } else { "alpha" });
        let detached = if case_sensitive {
            detach_object_cs(object, query.as_ptr())
        } else {
            detach_object(object, query.as_ptr())
        };
        push(
            result,
            if case_sensitive { "C057" } else { "C056" },
            format!(
                "{}:{}",
                text(api.print_unformatted(detached)),
                text(api.print_unformatted(object))
            ),
        );
        api.delete(detached);
        if case_sensitive {
            delete_object_cs(object, c_string("beta").as_ptr());
            delete_object_cs(object, c_string("Beta").as_ptr());
        } else {
            delete_object(object, c_string("BETA").as_ptr());
        }
        push(
            result,
            if case_sensitive { "C057" } else { "C056" },
            text(api.print_unformatted(object)),
        );
        api.delete(object);
    }

    let insert: unsafe extern "C" fn(*mut CJson, c_int, *mut CJson) -> c_int =
        api.symbol(b"cJSON_InsertItemInArray\0");
    for index in [0, 2, 5, 99] {
        let array = api.parse(&c_string("[0,1,2,3,4]"));
        let status = insert(array, index, create_number(9.0));
        push(
            result,
            "C058-C059",
            format!("{index}:{status}:{}", text(api.print_unformatted(array))),
        );
        api.delete(array);
    }
    let empty = create_array();
    push(
        result,
        "C058",
        format!(
            "{}:{}",
            insert(empty, 0, create_number(7.0)),
            text(api.print_unformatted(empty))
        ),
    );
    api.delete(empty);

    let replace_pointer: unsafe extern "C" fn(*mut CJson, *mut CJson, *mut CJson) -> c_int =
        api.symbol(b"cJSON_ReplaceItemViaPointer\0");
    let replace_array: unsafe extern "C" fn(*mut CJson, c_int, *mut CJson) -> c_int =
        api.symbol(b"cJSON_ReplaceItemInArray\0");
    for index in [0, 2, 4] {
        let array = api.parse(&c_string("[0,1,2,3,4]"));
        let old = get_item(array, index);
        push(result, "C060", replace_pointer(array, old, old));
        let status = replace_pointer(array, old, create_number(8.0));
        push(
            result,
            "C060",
            format!("{index}:{status}:{}", text(api.print_unformatted(array))),
        );
        api.delete(array);

        let array = api.parse(&c_string("[0,1,2,3,4]"));
        let status = replace_array(array, index, create_number(6.0));
        push(
            result,
            "C061",
            format!("{index}:{status}:{}", text(api.print_unformatted(array))),
        );
        api.delete(array);
    }

    let replace_object: unsafe extern "C" fn(*mut CJson, *const c_char, *mut CJson) -> c_int =
        api.symbol(b"cJSON_ReplaceItemInObject\0");
    let replace_object_cs: unsafe extern "C" fn(*mut CJson, *const c_char, *mut CJson) -> c_int =
        api.symbol(b"cJSON_ReplaceItemInObjectCaseSensitive\0");
    for case_sensitive in [false, true] {
        let object = api.parse(&c_string(r#"{"Alpha":1,"Beta":2}"#));
        let key = c_string(if case_sensitive { "Alpha" } else { "alpha" });
        let status = if case_sensitive {
            replace_object_cs(object, key.as_ptr(), create_number(9.0))
        } else {
            replace_object(object, key.as_ptr(), create_number(9.0))
        };
        push(
            result,
            if case_sensitive { "C063" } else { "C062" },
            format!("{status}:{}", text(api.print_unformatted(object))),
        );
        api.delete(object);
    }

    let set_string: unsafe extern "C" fn(*mut CJson, *const c_char) -> *mut c_char =
        api.symbol(b"cJSON_SetValuestring\0");
    let item = create_string(c_string("original").as_ptr());
    for (id, value) in [
        ("C064", "tiny"),
        ("C064", "12345678"),
        ("C065", "a much longer replacement"),
    ] {
        let value = c_string(value);
        let output = set_string(item, value.as_ptr());
        push(
            result,
            id,
            format!("{}:{}", output == (*item).valuestring, c_string_at(output)),
        );
    }
    api.delete(item);

    let duplicate: unsafe extern "C" fn(*const CJson, c_int) -> *mut CJson =
        api.symbol(b"cJSON_Duplicate\0");
    let modified = create_object();
    let constant = c_string("constant");
    add_object_cs(
        modified,
        constant.as_ptr(),
        create_string(c_string("v").as_ptr()),
    );
    let copy = duplicate(modified, 1);
    push(
        result,
        "C067",
        format!("{}:{}", (*(*modified).child).type_, (*(*copy).child).type_),
    );
    api.delete(copy);
    api.delete(modified);

    let compare: unsafe extern "C" fn(*const CJson, *const CJson, c_int) -> c_int =
        api.symbol(b"cJSON_Compare\0");
    let pairs = [
        ("null", "null"),
        ("true", "true"),
        ("1.25", "1.25"),
        (r#""x""#, r#""x""#),
        ("[1,2,3]", "[1,2,3]"),
        (r#"{"a":1,"b":2}"#, r#"{"b":2,"a":1}"#),
    ];
    for (left, right) in pairs {
        let a = api.parse(&c_string(left));
        let b = api.parse(&c_string(right));
        for case in [0, 1, -2] {
            push(
                result,
                "C068,C069,C070",
                format!(
                    "{left}:{case}:{}:{}",
                    compare(a, a, case),
                    compare(a, b, case)
                ),
            );
        }
        api.delete(a);
        api.delete(b);
    }
    let lower = api.parse(&c_string(r#"{"key":1}"#));
    let upper = api.parse(&c_string(r#"{"KEY":1}"#));
    for case in [0, 1, -1] {
        push(
            result,
            "C070",
            format!("case:{case}:{}", compare(lower, upper, case)),
        );
    }
    api.delete(lower);
    api.delete(upper);

    let type_checks = [
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
    for type_ in [
        INVALID, FALSE, TRUE, NULL, NUMBER, STRING, ARRAY, OBJECT, RAW,
    ] {
        let item = CJson {
            type_: type_ | IS_REFERENCE | STRING_IS_CONST,
            ..CJson::default()
        };
        let values: Vec<_> = type_checks
            .iter()
            .map(|name| {
                let function: unsafe extern "C" fn(*const CJson) -> c_int = api.symbol(name);
                function(&item)
            })
            .collect();
        push(result, "C024", format!("{type_}:{values:?}"));
    }
}

unsafe fn run_valid_minify(api: &Api, result: &mut Vec<String>) {
    let minify: unsafe extern "C" fn(*mut c_char) = api.symbol(b"cJSON_Minify\0");
    for (id, source) in [
        ("C071", " { \n \"a b\" : [ 1, 2 ] } \t"),
        ("C072", "{// line\n\"a\":1,/* block */\"b\":2/3}"),
        ("C073", r#" { "s" : "quote: \" slash: \\ keep space" } "#),
    ] {
        let mut bytes = source.as_bytes().to_vec();
        bytes.push(0);
        minify(bytes.as_mut_ptr().cast());
        push(result, id, c_string_at(bytes.as_ptr().cast()));
    }
}

unsafe fn parse_error_case(
    api: &Api,
    id: &str,
    source: &str,
    require_null: c_int,
    result: &mut Vec<String>,
) {
    let parse: unsafe extern "C" fn(*const c_char, usize, *mut *const c_char, c_int) -> *mut CJson =
        api.symbol(b"cJSON_ParseWithLengthOpts\0");
    let get_error: unsafe extern "C" fn() -> *const c_char = api.symbol(b"cJSON_GetErrorPtr\0");
    let input = c_string(source);
    let mut end = ptr::null();
    let item = parse(
        input.as_ptr(),
        input.as_bytes_with_nul().len(),
        &mut end,
        require_null,
    );
    let end_offset = if end.is_null() {
        -1
    } else {
        end.offset_from(input.as_ptr()) as isize
    };
    let error = get_error();
    let error_offset = if error.is_null() {
        -1
    } else {
        error.offset_from(input.as_ptr()) as isize
    };
    push(
        result,
        id,
        format!("{}:{end_offset}:{error_offset}", item.is_null()),
    );
    api.delete(item);
}

unsafe fn run_errors(api: &Api) -> Vec<String> {
    let mut result = Vec::new();

    let get_string: unsafe extern "C" fn(*const CJson) -> *mut c_char =
        api.symbol(b"cJSON_GetStringValue\0");
    let get_number: unsafe extern "C" fn(*const CJson) -> c_double =
        api.symbol(b"cJSON_GetNumberValue\0");
    let number = CJson {
        type_: NUMBER,
        valuedouble: 3.5,
        ..CJson::default()
    };
    let string = CJson {
        type_: STRING,
        ..CJson::default()
    };
    push(&mut result, "E001", get_string(ptr::null()).is_null());
    push(&mut result, "E002", get_string(&number).is_null());
    push(&mut result, "E003", get_number(ptr::null()).is_nan());
    push(&mut result, "E004", get_number(&string).is_nan());

    let parse_opts: unsafe extern "C" fn(*const c_char, *mut *const c_char, c_int) -> *mut CJson =
        api.symbol(b"cJSON_ParseWithOpts\0");
    let parse_length_opts: unsafe extern "C" fn(
        *const c_char,
        usize,
        *mut *const c_char,
        c_int,
    ) -> *mut CJson = api.symbol(b"cJSON_ParseWithLengthOpts\0");
    push(
        &mut result,
        "E005",
        parse_opts(ptr::null(), ptr::null_mut(), 0).is_null(),
    );
    let mut end = 1usize as *const c_char;
    push(
        &mut result,
        "E006",
        format!(
            "{}:{}",
            parse_length_opts(ptr::null(), 1, &mut end, 0).is_null(),
            end as usize
        ),
    );
    let empty = c_string("");
    end = ptr::null();
    push(
        &mut result,
        "E007",
        format!(
            "{}:{}",
            parse_length_opts(empty.as_ptr(), 0, &mut end, 0).is_null(),
            end.offset_from(empty.as_ptr())
        ),
    );

    for (id, source) in [
        ("E008", "?"),
        ("E009", "-"),
        ("E010", "\"unterminated"),
        ("E011", "\"slash\\"),
        ("E012", "\"\\x\""),
        ("E013", "\"\\u12\""),
        ("E014", "\"\\udc00\""),
        ("E015", "\"\\ud800\""),
        ("E016", "\"\\ud800\\u0041\""),
        ("E018", "["),
        ("E019", "[?]"),
        ("E020", "[1,]"),
        ("E021", "[1"),
        ("E023", "{"),
        ("E024", "{\"a\":1,"),
        ("E025", "{a:1}"),
        ("E026", "{\"a\" 1}"),
        ("E027", "{\"a\":?}"),
        ("E028", "{\"a\":1"),
    ] {
        parse_error_case(api, id, source, 0, &mut result);
    }
    parse_error_case(api, "E029", "true trailing", 1, &mut result);
    let deep_array = format!("{}0{}", "[".repeat(1001), "]".repeat(1001));
    parse_error_case(api, "E017", &deep_array, 0, &mut result);
    let deep_object = format!("{}0{}", "{\"x\":".repeat(1001), "}".repeat(1001));
    parse_error_case(api, "E022", &deep_object, 0, &mut result);

    for fail_at in 0..16 {
        set_failure_hook(api, fail_at);
        let item = api.parse(&c_string(
            r#"{"number":1.25,"string":"value","array":[1,2,3],"object":{"x":true}}"#,
        ));
        let output = if item.is_null() {
            "<NULL>".to_owned()
        } else {
            text(api.print_unformatted(item))
        };
        push(
            &mut result,
            "E030,E031,E032,E033,E034",
            format!("{fail_at}:{}:{output}", item.is_null()),
        );
        api.delete(item);
        reset_hooks(api);
    }

    push(
        &mut result,
        "E035",
        format!(
            "{}:{}",
            api.print(ptr::null()).is_none(),
            api.print_unformatted(ptr::null()).is_none()
        ),
    );
    let invalid = CJson {
        type_: 3,
        ..CJson::default()
    };
    let raw_null = CJson {
        type_: RAW,
        ..CJson::default()
    };
    push(
        &mut result,
        "E036",
        api.print_unformatted(&invalid).is_none(),
    );
    push(
        &mut result,
        "E037",
        api.print_unformatted(&raw_null).is_none(),
    );

    for fail_at in 0..10 {
        let item = api.parse(&c_string(
            r#"{"long":"abcdefghijklmnopqrstuvwxyz0123456789","array":[1,2,3,4,5]}"#,
        ));
        set_failure_hook(api, fail_at);
        let output = api.print(item);
        push(
            &mut result,
            "E038,E039,E040",
            format!("{fail_at}:{}", text(output)),
        );
        reset_hooks(api);
        api.delete(item);
    }

    let print_buffered: unsafe extern "C" fn(*const CJson, c_int, c_int) -> *mut c_char =
        api.symbol(b"cJSON_PrintBuffered\0");
    let item = api.parse(&c_string("true"));
    push(&mut result, "E041", print_buffered(item, -1, 0).is_null());
    set_failure_hook(api, 0);
    push(&mut result, "E042", print_buffered(item, 10, 0).is_null());
    reset_hooks(api);

    let print_preallocated: unsafe extern "C" fn(*mut CJson, *mut c_char, c_int, c_int) -> c_int =
        api.symbol(b"cJSON_PrintPreallocated\0");
    let mut output = [0u8; 16];
    push(
        &mut result,
        "E043",
        print_preallocated(item, output.as_mut_ptr().cast(), -1, 0),
    );
    push(
        &mut result,
        "E044",
        print_preallocated(item, ptr::null_mut(), 16, 0),
    );
    push(
        &mut result,
        "E045",
        print_preallocated(item, output.as_mut_ptr().cast(), 4, 0),
    );
    api.delete(item);

    let create_string: unsafe extern "C" fn(*const c_char) -> *mut CJson =
        api.symbol(b"cJSON_CreateString\0");
    let create_raw: unsafe extern "C" fn(*const c_char) -> *mut CJson =
        api.symbol(b"cJSON_CreateRaw\0");
    let create_number: unsafe extern "C" fn(c_double) -> *mut CJson =
        api.symbol(b"cJSON_CreateNumber\0");
    let create_object: unsafe extern "C" fn() -> *mut CJson = api.symbol(b"cJSON_CreateObject\0");
    let set_string: unsafe extern "C" fn(*mut CJson, *const c_char) -> *mut c_char =
        api.symbol(b"cJSON_SetValuestring\0");
    let valid_string = create_string(c_string("abcdef").as_ptr());
    let nonstring = create_number(1.0);
    let mut corrupt_string = CJson {
        type_: STRING,
        ..CJson::default()
    };
    push(
        &mut result,
        "E046",
        set_string(ptr::null_mut(), c_string("x").as_ptr()).is_null(),
    );
    push(
        &mut result,
        "E047",
        set_string(nonstring, c_string("x").as_ptr()).is_null(),
    );
    (*valid_string).type_ |= IS_REFERENCE;
    push(
        &mut result,
        "E048",
        set_string(valid_string, c_string("x").as_ptr()).is_null(),
    );
    (*valid_string).type_ &= !IS_REFERENCE;
    push(
        &mut result,
        "E049",
        set_string(&mut corrupt_string, c_string("x").as_ptr()).is_null(),
    );
    push(
        &mut result,
        "E050",
        set_string(valid_string, ptr::null()).is_null(),
    );
    push(
        &mut result,
        "E051",
        set_string(valid_string, (*valid_string).valuestring.add(1)).is_null(),
    );
    set_failure_hook(api, 0);
    push(
        &mut result,
        "E052",
        set_string(valid_string, c_string("a much longer value").as_ptr()).is_null(),
    );
    reset_hooks(api);
    api.delete(valid_string);
    api.delete(nonstring);

    let get_size: unsafe extern "C" fn(*const CJson) -> c_int = api.symbol(b"cJSON_GetArraySize\0");
    let get_item: unsafe extern "C" fn(*const CJson, c_int) -> *mut CJson =
        api.symbol(b"cJSON_GetArrayItem\0");
    push(&mut result, "E053", get_size(ptr::null()));
    push(&mut result, "E054", get_item(ptr::null(), 0).is_null());
    let array = api.parse(&c_string("[1,2]"));
    push(&mut result, "E055", get_item(array, -1).is_null());
    push(&mut result, "E056", get_item(array, 2).is_null());

    let get_object: unsafe extern "C" fn(*const CJson, *const c_char) -> *mut CJson =
        api.symbol(b"cJSON_GetObjectItem\0");
    let get_object_cs: unsafe extern "C" fn(*const CJson, *const c_char) -> *mut CJson =
        api.symbol(b"cJSON_GetObjectItemCaseSensitive\0");
    let has_object: unsafe extern "C" fn(*const CJson, *const c_char) -> c_int =
        api.symbol(b"cJSON_HasObjectItem\0");
    let object = api.parse(&c_string(r#"{"a":1}"#));
    let absent = c_string("absent");
    push(
        &mut result,
        "E057",
        format!(
            "{}:{}",
            get_object(ptr::null(), absent.as_ptr()).is_null(),
            has_object(ptr::null(), absent.as_ptr())
        ),
    );
    push(
        &mut result,
        "E058",
        format!(
            "{}:{}",
            get_object(object, ptr::null()).is_null(),
            has_object(object, ptr::null())
        ),
    );
    push(
        &mut result,
        "E059",
        format!(
            "{}:{}",
            get_object_cs(object, absent.as_ptr()).is_null(),
            has_object(object, absent.as_ptr())
        ),
    );
    let nameless = create_number(1.0);
    let nameless_object = create_object();
    let add_array: unsafe extern "C" fn(*mut CJson, *mut CJson) -> c_int =
        api.symbol(b"cJSON_AddItemToArray\0");
    add_array(nameless_object, nameless);
    push(
        &mut result,
        "E060",
        get_object(nameless_object, c_string("x").as_ptr()).is_null(),
    );

    let detached_item = create_number(5.0);
    push(
        &mut result,
        "E061",
        add_array(ptr::null_mut(), detached_item),
    );
    push(&mut result, "E062", add_array(array, ptr::null_mut()));
    push(&mut result, "E063", add_array(array, array));

    let add_object: unsafe extern "C" fn(*mut CJson, *const c_char, *mut CJson) -> c_int =
        api.symbol(b"cJSON_AddItemToObject\0");
    let add_object_cs: unsafe extern "C" fn(*mut CJson, *const c_char, *mut CJson) -> c_int =
        api.symbol(b"cJSON_AddItemToObjectCS\0");
    let spare = create_number(2.0);
    push(
        &mut result,
        "E064",
        format!(
            "{}:{}:{}",
            add_object(ptr::null_mut(), c_string("x").as_ptr(), spare),
            add_object(object, ptr::null(), spare),
            add_object_cs(object, c_string("x").as_ptr(), ptr::null_mut())
        ),
    );
    push(
        &mut result,
        "E065",
        add_object(object, c_string("self").as_ptr(), object),
    );
    let fail_key_item = create_number(3.0);
    set_failure_hook(api, 0);
    let fail_key_status = add_object(object, c_string("key").as_ptr(), fail_key_item);
    push(&mut result, "E066", fail_key_status);
    reset_hooks(api);
    api.delete(fail_key_item);

    let add_ref_array: unsafe extern "C" fn(*mut CJson, *mut CJson) -> c_int =
        api.symbol(b"cJSON_AddItemReferenceToArray\0");
    let add_ref_object: unsafe extern "C" fn(*mut CJson, *const c_char, *mut CJson) -> c_int =
        api.symbol(b"cJSON_AddItemReferenceToObject\0");
    push(&mut result, "E067", add_ref_array(ptr::null_mut(), spare));
    push(&mut result, "E068", add_ref_array(array, ptr::null_mut()));
    push(
        &mut result,
        "E069",
        format!(
            "{}:{}",
            add_ref_object(ptr::null_mut(), c_string("x").as_ptr(), spare),
            add_ref_object(object, ptr::null(), spare)
        ),
    );
    push(
        &mut result,
        "E070",
        add_ref_object(object, c_string("x").as_ptr(), ptr::null_mut()),
    );

    let helper_symbols = [
        ("E071", b"cJSON_AddNullToObject\0".as_slice()),
        ("E072", b"cJSON_AddTrueToObject\0".as_slice()),
        ("E073", b"cJSON_AddFalseToObject\0".as_slice()),
        ("E078", b"cJSON_AddObjectToObject\0".as_slice()),
        ("E079", b"cJSON_AddArrayToObject\0".as_slice()),
    ];
    for (id, symbol) in helper_symbols {
        let function: unsafe extern "C" fn(*mut CJson, *const c_char) -> *mut CJson =
            api.symbol(symbol);
        push(
            &mut result,
            id,
            function(ptr::null_mut(), c_string("x").as_ptr()).is_null(),
        );
    }
    let add_bool_helper: unsafe extern "C" fn(*mut CJson, *const c_char, c_int) -> *mut CJson =
        api.symbol(b"cJSON_AddBoolToObject\0");
    let add_number_helper: unsafe extern "C" fn(*mut CJson, *const c_char, c_double) -> *mut CJson =
        api.symbol(b"cJSON_AddNumberToObject\0");
    let add_string_helper: unsafe extern "C" fn(
        *mut CJson,
        *const c_char,
        *const c_char,
    ) -> *mut CJson = api.symbol(b"cJSON_AddStringToObject\0");
    let add_raw_helper: unsafe extern "C" fn(
        *mut CJson,
        *const c_char,
        *const c_char,
    ) -> *mut CJson = api.symbol(b"cJSON_AddRawToObject\0");
    push(
        &mut result,
        "E074",
        add_bool_helper(ptr::null_mut(), c_string("x").as_ptr(), 1).is_null(),
    );
    push(
        &mut result,
        "E075",
        add_number_helper(ptr::null_mut(), c_string("x").as_ptr(), 1.0).is_null(),
    );
    push(
        &mut result,
        "E076",
        add_string_helper(object, c_string("x").as_ptr(), ptr::null()).is_null(),
    );
    push(
        &mut result,
        "E077",
        add_raw_helper(object, c_string("x").as_ptr(), ptr::null()).is_null(),
    );

    run_mutation_errors(api, &mut result, array, object);

    api.delete(detached_item);
    api.delete(spare);
    api.delete(nameless_object);
    api.delete(array);
    api.delete(object);

    push(&mut result, "E095", create_string(ptr::null()).is_null());
    push(&mut result, "E097", create_raw(ptr::null()).is_null());
    for (id, constructor) in [("E096", create_string), ("E098", create_raw)] {
        set_failure_hook(api, 0);
        let item = constructor(c_string("value").as_ptr());
        push(&mut result, id, item.is_null());
        reset_hooks(api);
        api.delete(item);
    }

    run_array_creation_errors(api, &mut result);
    run_compare_errors(api, &mut result);

    let duplicate: unsafe extern "C" fn(*const CJson, c_int) -> *mut CJson =
        api.symbol(b"cJSON_Duplicate\0");
    push(&mut result, "E106", duplicate(ptr::null(), 1).is_null());
    let duplicate_source = api.parse(&c_string(r#"{"x":[1,2],"s":"value"}"#));
    for fail_at in 0..8 {
        set_failure_hook(api, fail_at);
        let copy = duplicate(duplicate_source, 1);
        push(&mut result, "E107", format!("{fail_at}:{}", copy.is_null()));
        api.delete(copy);
        reset_hooks(api);
    }
    api.delete(duplicate_source);

    let checks = [
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
    let null_results: Vec<_> = checks
        .iter()
        .map(|name| {
            let function: unsafe extern "C" fn(*const CJson) -> c_int = api.symbol(name);
            function(ptr::null())
        })
        .collect();
    push(&mut result, "E109", format!("{null_results:?}"));

    set_failure_hook(api, 0);
    let allocation: unsafe extern "C" fn(usize) -> *mut c_void = api.symbol(b"cJSON_malloc\0");
    push(&mut result, "E118", allocation(128).is_null());
    reset_hooks(api);
    let minify: unsafe extern "C" fn(*mut c_char) = api.symbol(b"cJSON_Minify\0");
    minify(ptr::null_mut());
    push(&mut result, "E119", "no-op");
    result
}

unsafe fn run_mutation_errors(
    api: &Api,
    result: &mut Vec<String>,
    array: *mut CJson,
    object: *mut CJson,
) {
    let create_number: unsafe extern "C" fn(c_double) -> *mut CJson =
        api.symbol(b"cJSON_CreateNumber\0");
    let get_item: unsafe extern "C" fn(*const CJson, c_int) -> *mut CJson =
        api.symbol(b"cJSON_GetArrayItem\0");
    let detach_pointer: unsafe extern "C" fn(*mut CJson, *mut CJson) -> *mut CJson =
        api.symbol(b"cJSON_DetachItemViaPointer\0");
    let detach_array: unsafe extern "C" fn(*mut CJson, c_int) -> *mut CJson =
        api.symbol(b"cJSON_DetachItemFromArray\0");
    let detach_object: unsafe extern "C" fn(*mut CJson, *const c_char) -> *mut CJson =
        api.symbol(b"cJSON_DetachItemFromObject\0");
    let detach_object_cs: unsafe extern "C" fn(*mut CJson, *const c_char) -> *mut CJson =
        api.symbol(b"cJSON_DetachItemFromObjectCaseSensitive\0");
    let delete_object: unsafe extern "C" fn(*mut CJson, *const c_char) =
        api.symbol(b"cJSON_DeleteItemFromObject\0");
    let delete_object_cs: unsafe extern "C" fn(*mut CJson, *const c_char) =
        api.symbol(b"cJSON_DeleteItemFromObjectCaseSensitive\0");

    let unlinked = create_number(7.0);
    push(
        result,
        "E080",
        format!(
            "{}:{}",
            detach_pointer(ptr::null_mut(), unlinked).is_null(),
            detach_pointer(array, ptr::null_mut()).is_null()
        ),
    );
    push(result, "E081", detach_pointer(array, unlinked).is_null());
    push(result, "E082", detach_array(array, -1).is_null());
    push(result, "E083", detach_array(array, 99).is_null());
    let absent = c_string("absent");
    push(
        result,
        "E084",
        format!(
            "{}:{}",
            detach_object(object, absent.as_ptr()).is_null(),
            detach_object_cs(object, ptr::null()).is_null()
        ),
    );
    delete_object(object, absent.as_ptr());
    delete_object_cs(ptr::null_mut(), absent.as_ptr());
    api.delete(unlinked);

    let insert: unsafe extern "C" fn(*mut CJson, c_int, *mut CJson) -> c_int =
        api.symbol(b"cJSON_InsertItemInArray\0");
    let item = create_number(8.0);
    push(result, "E085", insert(array, -1, item));
    push(result, "E086", insert(array, 0, ptr::null_mut()));
    push(result, "E087", insert(ptr::null_mut(), 0, item));
    api.delete(item);
    let corrupt = api.parse(&c_string("[1,2]"));
    let target = get_item(corrupt, 1);
    let saved_prev = (*target).prev;
    (*target).prev = ptr::null_mut();
    let item = create_number(9.0);
    push(result, "E088", insert(corrupt, 1, item));
    (*target).prev = saved_prev;
    api.delete(item);
    api.delete(corrupt);

    let replace_pointer: unsafe extern "C" fn(*mut CJson, *mut CJson, *mut CJson) -> c_int =
        api.symbol(b"cJSON_ReplaceItemViaPointer\0");
    let replacement = create_number(10.0);
    let empty = api.parse(&c_string("[]"));
    push(
        result,
        "E089",
        format!(
            "{}:{}:{}:{}",
            replace_pointer(ptr::null_mut(), get_item(array, 0), replacement),
            replace_pointer(empty, get_item(array, 0), replacement),
            replace_pointer(array, ptr::null_mut(), replacement),
            replace_pointer(array, get_item(array, 0), ptr::null_mut())
        ),
    );
    api.delete(replacement);
    api.delete(empty);

    let replace_array: unsafe extern "C" fn(*mut CJson, c_int, *mut CJson) -> c_int =
        api.symbol(b"cJSON_ReplaceItemInArray\0");
    let replacement = create_number(11.0);
    push(result, "E090", replace_array(array, -1, replacement));
    push(result, "E091", replace_array(array, 99, replacement));
    api.delete(replacement);

    let replace_object: unsafe extern "C" fn(*mut CJson, *const c_char, *mut CJson) -> c_int =
        api.symbol(b"cJSON_ReplaceItemInObject\0");
    let replace_object_cs: unsafe extern "C" fn(*mut CJson, *const c_char, *mut CJson) -> c_int =
        api.symbol(b"cJSON_ReplaceItemInObjectCaseSensitive\0");
    let replacement = create_number(12.0);
    push(
        result,
        "E092",
        format!(
            "{}:{}",
            replace_object(object, ptr::null(), replacement),
            replace_object_cs(object, c_string("x").as_ptr(), ptr::null_mut())
        ),
    );
    api.delete(replacement);

    let replacement = create_number(13.0);
    set_failure_hook(api, 0);
    push(
        result,
        "E093",
        replace_object(object, c_string("a").as_ptr(), replacement),
    );
    reset_hooks(api);
    api.delete(replacement);

    let replacement = create_number(14.0);
    push(
        result,
        "E094",
        format!(
            "{}:{}",
            replace_object(object, absent.as_ptr(), replacement),
            replace_object_cs(ptr::null_mut(), absent.as_ptr(), replacement)
        ),
    );
    api.delete(replacement);
}

unsafe fn run_array_creation_errors(api: &Api, result: &mut Vec<String>) {
    let create_int: unsafe extern "C" fn(*const c_int, c_int) -> *mut CJson =
        api.symbol(b"cJSON_CreateIntArray\0");
    let create_float: unsafe extern "C" fn(*const c_float, c_int) -> *mut CJson =
        api.symbol(b"cJSON_CreateFloatArray\0");
    let create_double: unsafe extern "C" fn(*const c_double, c_int) -> *mut CJson =
        api.symbol(b"cJSON_CreateDoubleArray\0");
    let create_strings: unsafe extern "C" fn(*const *const c_char, c_int) -> *mut CJson =
        api.symbol(b"cJSON_CreateStringArray\0");
    let integers = [1, 2];
    let floats = [1.0f32, 2.0];
    let doubles = [1.0f64, 2.0];
    push(
        result,
        "E099",
        format!(
            "{}:{}:{}",
            create_int(integers.as_ptr(), -1).is_null(),
            create_float(floats.as_ptr(), -1).is_null(),
            create_double(doubles.as_ptr(), -1).is_null()
        ),
    );
    push(
        result,
        "E100",
        format!(
            "{}:{}:{}",
            create_int(ptr::null(), 0).is_null(),
            create_float(ptr::null(), 0).is_null(),
            create_double(ptr::null(), 0).is_null()
        ),
    );
    for fail_at in 0..4 {
        set_failure_hook(api, fail_at);
        let item = create_int(integers.as_ptr(), 2);
        push(result, "E101", format!("{fail_at}:{}", item.is_null()));
        api.delete(item);
        reset_hooks(api);
    }
    let string = c_string("x");
    let strings = [string.as_ptr()];
    push(
        result,
        "E102",
        create_strings(strings.as_ptr(), -1).is_null(),
    );
    push(result, "E103", create_strings(ptr::null(), 0).is_null());
    let null_strings = [ptr::null()];
    push(
        result,
        "E104",
        create_strings(null_strings.as_ptr(), 1).is_null(),
    );
    for fail_at in 0..5 {
        set_failure_hook(api, fail_at);
        let item = create_strings(strings.as_ptr(), 1);
        push(result, "E105", format!("{fail_at}:{}", item.is_null()));
        api.delete(item);
        reset_hooks(api);
    }
}

unsafe fn run_compare_errors(api: &Api, result: &mut Vec<String>) {
    let compare: unsafe extern "C" fn(*const CJson, *const CJson, c_int) -> c_int =
        api.symbol(b"cJSON_Compare\0");
    let number = api.parse(&c_string("1"));
    let string = api.parse(&c_string(r#""1""#));
    push(
        result,
        "E110",
        format!(
            "{}:{}",
            compare(ptr::null(), number, 0),
            compare(number, string, 0)
        ),
    );
    let invalid_a = CJson {
        type_: 3,
        ..CJson::default()
    };
    let invalid_b = CJson {
        type_: 3,
        ..CJson::default()
    };
    push(result, "E111", compare(&invalid_a, &invalid_b, 0));

    let number_two = api.parse(&c_string("2"));
    push(result, "E112", compare(number, number_two, 0));
    let null_string_a = CJson {
        type_: STRING,
        ..CJson::default()
    };
    let null_string_b = CJson {
        type_: STRING,
        ..CJson::default()
    };
    push(
        result,
        "E113",
        format!(
            "{}:{}",
            compare(&null_string_a, &null_string_b, 0),
            compare(
                api.parse(&c_string(r#""a""#)),
                api.parse(&c_string(r#""b""#)),
                0
            )
        ),
    );

    for (id, left, right) in [
        ("E114", "[1,2]", "[1,3]"),
        ("E115", "[1,2]", "[1,2,3]"),
        ("E116", r#"{"a":1}"#, r#"{"b":1}"#),
        ("E117", r#"{"a":1}"#, r#"{"a":2}"#),
    ] {
        let left = api.parse(&c_string(left));
        let right = api.parse(&c_string(right));
        push(result, id, compare(left, right, 1));
        api.delete(left);
        api.delete(right);
    }
    api.delete(number);
    api.delete(number_two);
    api.delete(string);
}

fn assert_results_equal(kind: &str, c: &[String], rust: &[String]) {
    assert_eq!(c.len(), rust.len(), "{kind} result count differs");
    for (index, (left, right)) in c.iter().zip(rust).enumerate() {
        assert_eq!(left, right, "{kind} divergence at result {index}");
    }
}

const CORE_SYMBOLS: &[&str] = &[
    "cJSON_AddArrayToObject",
    "cJSON_AddBoolToObject",
    "cJSON_AddFalseToObject",
    "cJSON_AddItemReferenceToArray",
    "cJSON_AddItemReferenceToObject",
    "cJSON_AddItemToArray",
    "cJSON_AddItemToObject",
    "cJSON_AddItemToObjectCS",
    "cJSON_AddNullToObject",
    "cJSON_AddNumberToObject",
    "cJSON_AddObjectToObject",
    "cJSON_AddRawToObject",
    "cJSON_AddStringToObject",
    "cJSON_AddTrueToObject",
    "cJSON_Compare",
    "cJSON_CreateArray",
    "cJSON_CreateArrayReference",
    "cJSON_CreateBool",
    "cJSON_CreateDoubleArray",
    "cJSON_CreateFalse",
    "cJSON_CreateFloatArray",
    "cJSON_CreateIntArray",
    "cJSON_CreateNull",
    "cJSON_CreateNumber",
    "cJSON_CreateObject",
    "cJSON_CreateObjectReference",
    "cJSON_CreateRaw",
    "cJSON_CreateString",
    "cJSON_CreateStringArray",
    "cJSON_CreateStringReference",
    "cJSON_CreateTrue",
    "cJSON_Delete",
    "cJSON_DeleteItemFromArray",
    "cJSON_DeleteItemFromObject",
    "cJSON_DeleteItemFromObjectCaseSensitive",
    "cJSON_DetachItemFromArray",
    "cJSON_DetachItemFromObject",
    "cJSON_DetachItemFromObjectCaseSensitive",
    "cJSON_DetachItemViaPointer",
    "cJSON_Duplicate",
    "cJSON_GetArrayItem",
    "cJSON_GetArraySize",
    "cJSON_GetErrorPtr",
    "cJSON_GetNumberValue",
    "cJSON_GetObjectItem",
    "cJSON_GetObjectItemCaseSensitive",
    "cJSON_GetStringValue",
    "cJSON_HasObjectItem",
    "cJSON_InitHooks",
    "cJSON_InsertItemInArray",
    "cJSON_IsArray",
    "cJSON_IsBool",
    "cJSON_IsFalse",
    "cJSON_IsInvalid",
    "cJSON_IsNull",
    "cJSON_IsNumber",
    "cJSON_IsObject",
    "cJSON_IsRaw",
    "cJSON_IsString",
    "cJSON_IsTrue",
    "cJSON_Minify",
    "cJSON_Parse",
    "cJSON_ParseWithLength",
    "cJSON_ParseWithLengthOpts",
    "cJSON_ParseWithOpts",
    "cJSON_Print",
    "cJSON_PrintBuffered",
    "cJSON_PrintPreallocated",
    "cJSON_PrintUnformatted",
    "cJSON_ReplaceItemInArray",
    "cJSON_ReplaceItemInObject",
    "cJSON_ReplaceItemInObjectCaseSensitive",
    "cJSON_ReplaceItemViaPointer",
    "cJSON_SetNumberHelper",
    "cJSON_SetValuestring",
    "cJSON_Version",
    "cJSON_free",
    "cJSON_malloc",
];

unsafe fn check_symbols(c: &Api, rust: &Api) {
    for symbol in CORE_SYMBOLS {
        let mut name = symbol.as_bytes().to_vec();
        name.push(0);
        let _: *mut c_void = c.symbol(&name);
        let _: *mut c_void = rust.symbol(&name);
    }
    let _: *mut c_void = rust.symbol(b"driver\0");
}

unsafe fn deep_duplicate_rejected(path: &Path, label: &'static str) -> bool {
    let api = Api::open(path, label);
    let duplicate: unsafe extern "C" fn(*const CJson, c_int) -> *mut CJson =
        api.symbol(b"cJSON_Duplicate\0");
    let mut nodes: Vec<CJson> = (0..10_002).map(|_| CJson::default()).collect();
    let base = nodes.as_mut_ptr();
    for index in 0..10_001 {
        (*base.add(index)).type_ = ARRAY;
        (*base.add(index)).child = base.add(index + 1);
    }
    (*base.add(10_001)).type_ = NULL;
    let copy = duplicate(base, 1);
    let rejected = copy.is_null();
    api.delete(copy);
    rejected
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

unsafe fn run_driver(library: &Library) -> c_int {
    let driver: unsafe extern "C" fn(
        *const *const c_char,
        *mut [c_int; 3],
        *mut c_int,
        *mut Record,
    ) -> c_int = *library.get(b"driver\0").unwrap();
    let days: Vec<CString> = [
        "Sunday",
        "Monday",
        "Tuesday",
        "Wednesday",
        "Thursday",
        "Friday",
        "Saturday",
    ]
    .iter()
    .map(|value| c_string(value))
    .collect();
    let pointers: Vec<_> = days.iter().map(|value| value.as_ptr()).collect();
    let mut numbers = [[0, -1, 2], [3, 4, 5], [6, 7, c_int::MAX]];
    let mut ids = [116, 943, 234, 38793];
    let values: Vec<CString> = [
        "zip",
        "",
        "SAN FRANCISCO",
        "CA",
        "94107",
        "US",
        "exact",
        "1 Main St",
        "Boston",
        "MA",
        "02110",
        "US",
    ]
    .iter()
    .map(|value| c_string(value))
    .collect();
    let mut records = [
        Record {
            precision: values[0].as_ptr(),
            lat: 37.7668,
            lon: -122.3959,
            address: values[1].as_ptr(),
            city: values[2].as_ptr(),
            state: values[3].as_ptr(),
            zip: values[4].as_ptr(),
            country: values[5].as_ptr(),
        },
        Record {
            precision: values[6].as_ptr(),
            lat: 42.3584,
            lon: -71.0598,
            address: values[7].as_ptr(),
            city: values[8].as_ptr(),
            state: values[9].as_ptr(),
            zip: values[10].as_ptr(),
            country: values[11].as_ptr(),
        },
    ];
    driver(
        pointers.as_ptr(),
        numbers.as_mut_ptr(),
        ids.as_mut_ptr(),
        records.as_mut_ptr(),
    )
}

unsafe fn capture_stdout(function: impl FnOnce() -> c_int) -> (c_int, Vec<u8>) {
    let mut fds = [0; 2];
    assert_eq!(fflush(ptr::null_mut()), 0);
    assert_eq!(pipe(fds.as_mut_ptr()), 0);
    let saved_stdout = dup(1);
    assert!(saved_stdout >= 0);
    assert_eq!(dup2(fds[1], 1), 1);
    close(fds[1]);

    let status = function();
    assert_eq!(fflush(ptr::null_mut()), 0);
    assert_eq!(dup2(saved_stdout, 1), 1);
    close(saved_stdout);

    let mut output = Vec::new();
    let mut buffer = [0u8; 4096];
    loop {
        let count = read(fds[0], buffer.as_mut_ptr().cast(), buffer.len());
        assert!(count >= 0);
        if count == 0 {
            break;
        }
        output.extend_from_slice(&buffer[..count as usize]);
    }
    close(fds[0]);
    (status, output)
}

#[test]
fn differential_core_surface() {
    let _guard = TEST_LOCK.lock().unwrap();
    unsafe {
        assert!(c_library().is_file(), "C shared library was not built");
        assert!(
            rust_library().is_file(),
            "Rust shared library was not built"
        );
        let c = Api::open(&c_library(), "C");
        let rust = Api::open(&rust_library(), "Rust");
        check_symbols(&c, &rust);

        let c_valid = run_valid(&c);
        let rust_valid = run_valid(&rust);
        assert_results_equal("valid", &c_valid, &rust_valid);

        let c_errors = run_errors(&c);
        let rust_errors = run_errors(&rust);
        assert_results_equal("error", &c_errors, &rust_errors);
    }
}

#[test]
fn circular_limit_and_driver_match() {
    let _guard = TEST_LOCK.lock().unwrap();
    let c_path = c_library();
    let rust_path = rust_library();
    let c_rejected = std::thread::Builder::new()
        .name("c-circular-limit".to_owned())
        .stack_size(64 * 1024 * 1024)
        .spawn(move || unsafe { deep_duplicate_rejected(&c_path, "C") })
        .unwrap()
        .join()
        .unwrap();
    let rust_rejected = std::thread::Builder::new()
        .name("rust-circular-limit".to_owned())
        .stack_size(64 * 1024 * 1024)
        .spawn(move || unsafe { deep_duplicate_rejected(&rust_path, "Rust") })
        .unwrap()
        .join()
        .unwrap();
    assert_eq!(c_rejected, rust_rejected, "E108 divergence");
    assert!(
        c_rejected,
        "C did not reject at its documented circular limit"
    );

    unsafe {
        let c_driver = Library::new(manifest_dir().join("c_src/build/libcJSON_test.so")).unwrap();
        let rust_driver = Library::new(rust_library()).unwrap();
        let c_output = capture_stdout(|| run_driver(&c_driver));
        let rust_output = capture_stdout(|| run_driver(&rust_driver));
        assert_eq!(c_output, rust_output, "C075");
    }
}
