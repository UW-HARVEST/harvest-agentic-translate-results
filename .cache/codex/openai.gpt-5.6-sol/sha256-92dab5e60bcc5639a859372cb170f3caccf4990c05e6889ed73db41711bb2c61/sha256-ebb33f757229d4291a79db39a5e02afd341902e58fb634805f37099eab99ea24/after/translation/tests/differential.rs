#![allow(unsafe_op_in_unsafe_fn)]

use libloading::Library;
use std::ffi::{CStr, CString, c_char, c_double, c_int, c_void};
use std::fs;
use std::os::fd::AsRawFd;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::ptr;

unsafe extern "C" {
    fn fopen(path: *const c_char, mode: *const c_char) -> *mut c_void;
    fn fclose(file: *mut c_void) -> c_int;
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct JsonT {
    type_: c_int,
    refcount: usize,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct JsonError {
    line: c_int,
    column: c_int,
    position: c_int,
    source: [c_char; 80],
    text: [c_char; 160],
}

impl Default for JsonError {
    fn default() -> Self {
        Self {
            line: 0,
            column: 0,
            position: 0,
            source: [0; 80],
            text: [0; 160],
        }
    }
}

#[repr(C)]
#[derive(Debug)]
struct StrBuffer {
    value: *mut c_char,
    length: usize,
    size: usize,
}

#[repr(C)]
#[derive(Debug)]
struct List {
    prev: *mut List,
    next: *mut List,
}

#[repr(C)]
#[derive(Debug)]
struct Bucket {
    first: *mut List,
    last: *mut List,
}

#[repr(C)]
#[derive(Debug)]
struct Hashtable {
    size: usize,
    buckets: *mut Bucket,
    order: usize,
    list: List,
    ordered_list: List,
}

const JSON_REJECT_DUPLICATES: usize = 0x1;
const JSON_DISABLE_EOF_CHECK: usize = 0x2;
const JSON_DECODE_ANY: usize = 0x4;
const JSON_DECODE_INT_AS_REAL: usize = 0x8;
const JSON_ALLOW_NUL: usize = 0x10;
const JSON_COMPACT: usize = 0x20;
const JSON_ENSURE_ASCII: usize = 0x40;
const JSON_SORT_KEYS: usize = 0x80;
const JSON_PRESERVE_ORDER: usize = 0x100;
const JSON_ENCODE_ANY: usize = 0x200;
const JSON_ESCAPE_SLASH: usize = 0x400;
const JSON_EMBED: usize = 0x10000;

fn paths() -> (PathBuf, PathBuf) {
    let crate_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    (
        crate_dir.join("../c_src/build/libjansson.so"),
        crate_dir.join("target/release/libjansson.so"),
    )
}

unsafe fn load_pair() -> (Library, Library) {
    let (c, rust) = paths();
    assert!(c.is_file(), "missing C library: {}", c.display());
    assert!(rust.is_file(), "missing Rust library: {}", rust.display());
    (unsafe { Library::new(c).unwrap() }, unsafe {
        Library::new(rust).unwrap()
    })
}

unsafe fn symbol<T: Copy>(lib: &Library, name: &[u8]) -> T {
    *unsafe { lib.get::<T>(name).unwrap() }
}

fn error_bytes(error: &JsonError) -> Vec<u8> {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&error.line.to_ne_bytes());
    bytes.extend_from_slice(&error.column.to_ne_bytes());
    bytes.extend_from_slice(&error.position.to_ne_bytes());
    bytes.extend(error.source.iter().map(|v| *v as u8));
    bytes.extend(error.text.iter().map(|v| *v as u8));
    bytes
}

unsafe fn dumped(lib: &Library, value: *const JsonT, flags: usize) -> Option<Vec<u8>> {
    type Dumps = unsafe extern "C" fn(*const JsonT, usize) -> *mut c_char;
    type Free = unsafe extern "C" fn(*mut c_void);
    let dumps: Dumps = unsafe { symbol(lib, b"json_dumps\0") };
    let free: Free = unsafe { symbol(lib, b"jsonp_free\0") };
    let output = unsafe { dumps(value, flags) };
    if output.is_null() {
        None
    } else {
        let result = unsafe { CStr::from_ptr(output) }.to_bytes().to_vec();
        unsafe { free(output.cast()) };
        Some(result)
    }
}

unsafe fn delete(lib: &Library, value: *mut JsonT) {
    type Delete = unsafe extern "C" fn(*mut JsonT);
    let function: Delete = unsafe { symbol(lib, b"json_delete\0") };
    unsafe { function(value) };
}

unsafe fn load_bytes(lib: &Library, bytes: &[u8], flags: usize) -> (Option<Vec<u8>>, Vec<u8>) {
    type Loadb = unsafe extern "C" fn(*const c_char, usize, usize, *mut JsonError) -> *mut JsonT;
    let loadb: Loadb = unsafe { symbol(lib, b"json_loadb\0") };
    let mut error = JsonError::default();
    let value = unsafe {
        loadb(
            bytes.as_ptr().cast(),
            bytes.len(),
            flags,
            &mut error as *mut JsonError,
        )
    };
    let result = if value.is_null() {
        None
    } else {
        let output = unsafe { dumped(lib, value, JSON_SORT_KEYS | JSON_ENCODE_ANY) };
        unsafe { delete(lib, value) };
        output
    };
    (result, error_bytes(&error))
}

fn lcg(state: &mut u64) -> u64 {
    *state = state
        .wrapping_mul(6364136223846793005)
        .wrapping_add(1442695040888963407);
    *state
}

#[test]
fn dynamic_symbol_surface_matches() {
    let (c, rust) = paths();
    fn names(path: &Path) -> Vec<String> {
        let output = Command::new("nm")
            .args(["-D", "--defined-only"])
            .arg(path)
            .output()
            .unwrap();
        assert!(output.status.success());
        let mut names: Vec<_> = String::from_utf8(output.stdout)
            .unwrap()
            .lines()
            .filter_map(|line| line.split_whitespace().last())
            .map(str::to_owned)
            .collect();
        names.sort();
        names.dedup();
        names
    }
    assert_eq!(names(&c), names(&rust));
}

#[test]
fn rust_cdylib_embeds_every_c_translation_unit() {
    let crate_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let cmake = fs::read_to_string(crate_dir.join("../c_src/CMakeLists.txt")).unwrap();
    let build_rs = fs::read_to_string(crate_dir.join("build.rs")).unwrap();
    for source in [
        "dtoa.c",
        "dump.c",
        "error.c",
        "hashtable.c",
        "hashtable_seed.c",
        "load.c",
        "memory.c",
        "pack_unpack.c",
        "strbuffer.c",
        "strconv.c",
        "utf.c",
        "value.c",
        "version.c",
    ] {
        assert!(cmake.contains(source), "CMake omits {source}");
        assert!(build_rs.contains(source), "Rust cdylib omits {source}");
    }
    assert!(build_rs.contains("-include"));
    assert!(build_rs.contains("rename.h"));
}

#[test]
fn valid_load_dump_matrix_randomized() {
    unsafe {
        let (c, rust) = load_pair();
        let fixed = [
            b"{}".as_slice(),
            b"[]",
            br#"{"b":2,"a":[true,false,null,"x/y","\xE2\x82\xAC"]}"#,
            br#"[0,-1,1.5,1e100,"",{"nested":{"z":0}}]"#,
            br#""scalar""#,
            b"9223372036854775807",
        ];
        let load_flags = [
            0,
            JSON_DECODE_ANY,
            JSON_DECODE_ANY | JSON_DECODE_INT_AS_REAL,
            JSON_DISABLE_EOF_CHECK | JSON_DECODE_ANY,
            JSON_REJECT_DUPLICATES | JSON_DECODE_ANY,
        ];
        for input in fixed {
            for flags in load_flags {
                let left = load_bytes(&c, input, flags);
                let right = load_bytes(&rust, input, flags);
                assert_eq!(left, right, "input={input:?} flags={flags:#x}");
            }
        }

        let mut state = 0x5eed_cafe_d00d_beefu64;
        for _ in 0..300 {
            let a = lcg(&mut state) as i64;
            let b = lcg(&mut state) as i64;
            let text = format!(
                r#"{{"b":{},"a":[{},{}],"s":"v{:016x}/\u20ac"}}"#,
                a,
                b,
                (a as f64 / 37.0),
                lcg(&mut state)
            );
            for flags in [0, JSON_REJECT_DUPLICATES] {
                assert_eq!(
                    load_bytes(&c, text.as_bytes(), flags),
                    load_bytes(&rust, text.as_bytes(), flags)
                );
            }
        }
    }
}

#[test]
fn constructors_containers_and_dump_options_match() {
    unsafe fn exercise(lib: &Library, seed: u64) -> Vec<Vec<u8>> {
        type Object = unsafe extern "C" fn() -> *mut JsonT;
        type Array = unsafe extern "C" fn() -> *mut JsonT;
        type Integer = unsafe extern "C" fn(i64) -> *mut JsonT;
        type Real = unsafe extern "C" fn(c_double) -> *mut JsonT;
        type StringN = unsafe extern "C" fn(*const c_char, usize) -> *mut JsonT;
        type ObjectSet =
            unsafe extern "C" fn(*mut JsonT, *const c_char, usize, *mut JsonT) -> c_int;
        type ArrayAppend = unsafe extern "C" fn(*mut JsonT, *mut JsonT) -> c_int;
        type ArrayInsert = unsafe extern "C" fn(*mut JsonT, usize, *mut JsonT) -> c_int;
        type ArraySet = unsafe extern "C" fn(*mut JsonT, usize, *mut JsonT) -> c_int;
        type ArrayRemove = unsafe extern "C" fn(*mut JsonT, usize) -> c_int;
        type Copy = unsafe extern "C" fn(*mut JsonT) -> *mut JsonT;
        type DeepCopy = unsafe extern "C" fn(*const JsonT) -> *mut JsonT;
        type Equal = unsafe extern "C" fn(*const JsonT, *const JsonT) -> c_int;
        let object: Object = symbol(lib, b"json_object\0");
        let array: Array = symbol(lib, b"json_array\0");
        let integer: Integer = symbol(lib, b"json_integer\0");
        let real: Real = symbol(lib, b"json_real\0");
        let stringn: StringN = symbol(lib, b"json_stringn\0");
        let object_set: ObjectSet = symbol(lib, b"json_object_setn_new\0");
        let append: ArrayAppend = symbol(lib, b"json_array_append_new\0");
        let insert: ArrayInsert = symbol(lib, b"json_array_insert_new\0");
        let set: ArraySet = symbol(lib, b"json_array_set_new\0");
        let remove: ArrayRemove = symbol(lib, b"json_array_remove\0");
        let copy: Copy = symbol(lib, b"json_copy\0");
        let deep_copy: DeepCopy = symbol(lib, b"json_deep_copy\0");
        let equal: Equal = symbol(lib, b"json_equal\0");

        let root = object();
        let values = array();
        assert_eq!(append(values, integer(seed as i64)), 0);
        assert_eq!(append(values, real((seed as i64) as f64 / 17.0)), 0);
        let bytes = b"a\0b\xe2\x82\xac";
        assert_eq!(
            append(values, stringn(bytes.as_ptr().cast(), bytes.len())),
            0
        );
        assert_eq!(insert(values, 1, integer(-7)), 0);
        assert_eq!(set(values, 0, integer(42)), 0);
        assert_eq!(remove(values, 2), 0);
        assert_eq!(object_set(root, b"k\0x".as_ptr().cast(), 3, values), 0);
        assert_eq!(
            object_set(
                root,
                b"slash".as_ptr().cast(),
                5,
                stringn(b"/".as_ptr().cast(), 1)
            ),
            0
        );

        let shallow = copy(root);
        let deep = deep_copy(root);
        assert_eq!(equal(root, shallow), 1);
        assert_eq!(equal(root, deep), 1);
        let mut outputs = Vec::new();
        for flags in [
            0,
            JSON_COMPACT,
            1,
            31,
            JSON_SORT_KEYS,
            JSON_SORT_KEYS | JSON_COMPACT,
            JSON_PRESERVE_ORDER,
            JSON_ENSURE_ASCII,
            JSON_ESCAPE_SLASH,
            JSON_EMBED,
        ] {
            outputs.push(dumped(lib, root, flags).unwrap());
        }
        delete(lib, shallow);
        delete(lib, deep);
        delete(lib, root);
        outputs
    }

    unsafe {
        let (c, rust) = load_pair();
        let mut state = 0x1234_5678_9abc_def0;
        for _ in 0..100 {
            let seed = lcg(&mut state);
            assert_eq!(exercise(&c, seed), exercise(&rust, seed));
        }
    }
}

#[test]
fn low_level_utf_strbuffer_and_numbers_match() {
    unsafe fn exercise(lib: &Library) -> Vec<Vec<u8>> {
        type CheckFirst = unsafe extern "C" fn(c_char) -> usize;
        type CheckFull = unsafe extern "C" fn(*const c_char, usize, *mut i32) -> usize;
        type CheckString = unsafe extern "C" fn(*const c_char, usize) -> c_int;
        type Encode = unsafe extern "C" fn(i32, *mut c_char, *mut usize) -> c_int;
        type Dtostr = unsafe extern "C" fn(*mut c_char, usize, c_double, c_int) -> c_int;
        type StrInit = unsafe extern "C" fn(*mut StrBuffer) -> c_int;
        type StrAppend = unsafe extern "C" fn(*mut StrBuffer, *const c_char, usize) -> c_int;
        type StrPop = unsafe extern "C" fn(*mut StrBuffer) -> c_char;
        type StrClear = unsafe extern "C" fn(*mut StrBuffer);
        type StrClose = unsafe extern "C" fn(*mut StrBuffer);
        let first: CheckFirst = symbol(lib, b"utf8_check_first\0");
        let full: CheckFull = symbol(lib, b"utf8_check_full\0");
        let check: CheckString = symbol(lib, b"utf8_check_string\0");
        let encode: Encode = symbol(lib, b"utf8_encode\0");
        let dtostr: Dtostr = symbol(lib, b"jsonp_dtostr\0");
        let init: StrInit = symbol(lib, b"strbuffer_init\0");
        let append: StrAppend = symbol(lib, b"strbuffer_append_bytes\0");
        let pop: StrPop = symbol(lib, b"strbuffer_pop\0");
        let clear: StrClear = symbol(lib, b"strbuffer_clear\0");
        let close: StrClose = symbol(lib, b"strbuffer_close\0");
        let mut out = Vec::new();

        out.push((0u8..=255).map(|b| first(b as c_char) as u8).collect());
        for bytes in [
            b"a".as_slice(),
            b"\xc2\xa2",
            b"\xe2\x82\xac",
            b"\xf0\x9f\x92\xa9",
            b"\xc0\x80",
            b"\xed\xa0\x80",
            b"\xf4\x90\x80\x80",
            b"\xe2\x82",
        ] {
            let mut cp = -1;
            let full_result = full(bytes.as_ptr().cast(), bytes.len(), &mut cp);
            let string_result = check(bytes.as_ptr().cast(), bytes.len());
            let mut row = Vec::new();
            row.extend_from_slice(&full_result.to_ne_bytes());
            row.extend_from_slice(&string_result.to_ne_bytes());
            row.extend_from_slice(&cp.to_ne_bytes());
            out.push(row);
        }
        for cp in [
            -1, 0, 0x7f, 0x80, 0x7ff, 0x800, 0xffff, 0x10000, 0x10ffff, 0x110000,
        ] {
            let mut buffer = [0i8; 8];
            let mut size = 99usize;
            let result = encode(cp, buffer.as_mut_ptr(), &mut size);
            let mut row = result.to_ne_bytes().to_vec();
            row.extend_from_slice(&size.to_ne_bytes());
            row.extend(buffer.iter().map(|v| *v as u8));
            out.push(row);
        }
        for value in [
            -0.0,
            0.0,
            1.0,
            -1.5,
            f64::MIN_POSITIVE,
            f64::from_bits(1),
            f64::MAX,
            1e-5,
            1e20,
        ] {
            for precision in [0, 1, 17, 31] {
                let mut buffer = [0i8; 128];
                let length = dtostr(buffer.as_mut_ptr(), buffer.len(), value, precision);
                let mut row = length.to_ne_bytes().to_vec();
                if length >= 0 {
                    row.extend(buffer[..length as usize].iter().map(|v| *v as u8));
                }
                out.push(row);
            }
        }

        let mut buffer = StrBuffer {
            value: ptr::null_mut(),
            length: 0,
            size: 0,
        };
        assert_eq!(init(&mut buffer), 0);
        let payload: Vec<u8> = (0..80).map(|v| (v * 17) as u8).collect();
        assert_eq!(
            append(&mut buffer, payload.as_ptr().cast(), payload.len()),
            0
        );
        out.push(std::slice::from_raw_parts(buffer.value.cast::<u8>(), buffer.length).to_vec());
        out.push(vec![pop(&mut buffer) as u8]);
        clear(&mut buffer);
        out.push(buffer.length.to_ne_bytes().to_vec());
        close(&mut buffer);
        out
    }

    unsafe {
        let (c, rust) = load_pair();
        assert_eq!(exercise(&c), exercise(&rust));
    }
}

#[test]
fn hashtable_low_level_matches() {
    unsafe fn exercise(lib: &Library) -> Vec<Vec<u8>> {
        type Init = unsafe extern "C" fn(*mut Hashtable) -> c_int;
        type Close = unsafe extern "C" fn(*mut Hashtable);
        type Set = unsafe extern "C" fn(*mut Hashtable, *const c_char, usize, *mut JsonT) -> c_int;
        type Get = unsafe extern "C" fn(*mut Hashtable, *const c_char, usize) -> *mut c_void;
        type Del = unsafe extern "C" fn(*mut Hashtable, *const c_char, usize) -> c_int;
        type Iter = unsafe extern "C" fn(*mut Hashtable) -> *mut c_void;
        type IterNext = unsafe extern "C" fn(*mut Hashtable, *mut c_void) -> *mut c_void;
        type IterKey = unsafe extern "C" fn(*mut c_void) -> *const c_char;
        type IterKeyLen = unsafe extern "C" fn(*mut c_void) -> usize;
        type IterValue = unsafe extern "C" fn(*mut c_void) -> *mut JsonT;
        type Integer = unsafe extern "C" fn(i64) -> *mut JsonT;
        type IntegerValue = unsafe extern "C" fn(*const JsonT) -> i64;
        let init: Init = symbol(lib, b"hashtable_init\0");
        let close: Close = symbol(lib, b"hashtable_close\0");
        let set: Set = symbol(lib, b"hashtable_set\0");
        let get: Get = symbol(lib, b"hashtable_get\0");
        let del: Del = symbol(lib, b"hashtable_del\0");
        let iter: Iter = symbol(lib, b"hashtable_iter\0");
        let next: IterNext = symbol(lib, b"hashtable_iter_next\0");
        let key: IterKey = symbol(lib, b"hashtable_iter_key\0");
        let key_len: IterKeyLen = symbol(lib, b"hashtable_iter_key_len\0");
        let iter_value: IterValue = symbol(lib, b"hashtable_iter_value\0");
        let integer: Integer = symbol(lib, b"json_integer\0");
        let integer_value: IntegerValue = symbol(lib, b"json_integer_value\0");
        let mut table: Hashtable = std::mem::zeroed();
        assert_eq!(init(&mut table), 0);
        let keys: [&[u8]; 4] = [b"", b"a", b"a\0b", b"a-longer-key"];
        for (index, bytes) in keys.iter().enumerate() {
            assert_eq!(
                set(
                    &mut table,
                    bytes.as_ptr().cast(),
                    bytes.len(),
                    integer(index as i64)
                ),
                0
            );
        }
        let mut rows = vec![table.size.to_ne_bytes().to_vec()];
        for bytes in keys {
            let value = get(&mut table, bytes.as_ptr().cast(), bytes.len()).cast::<JsonT>();
            rows.push(integer_value(value).to_ne_bytes().to_vec());
        }
        let mut found = Vec::new();
        let mut current = iter(&mut table);
        while !current.is_null() {
            let len = key_len(current);
            let bytes = std::slice::from_raw_parts(key(current).cast::<u8>(), len);
            found.push((bytes.to_vec(), integer_value(iter_value(current))));
            current = next(&mut table, current);
        }
        found.sort();
        for (bytes, value) in found {
            let mut row = bytes;
            row.extend_from_slice(&value.to_ne_bytes());
            rows.push(row);
        }
        rows.push(
            del(&mut table, b"a".as_ptr().cast(), 1)
                .to_ne_bytes()
                .to_vec(),
        );
        rows.push(
            del(&mut table, b"a".as_ptr().cast(), 1)
                .to_ne_bytes()
                .to_vec(),
        );
        close(&mut table);
        rows
    }
    unsafe {
        let (c, rust) = load_pair();
        assert_eq!(exercise(&c), exercise(&rust));
    }
}

#[test]
fn exact_error_paths_match() {
    unsafe fn exercise(lib: &Library) -> Vec<Vec<u8>> {
        type Real = unsafe extern "C" fn(c_double) -> *mut JsonT;
        type Integer = unsafe extern "C" fn(i64) -> *mut JsonT;
        type Array = unsafe extern "C" fn() -> *mut JsonT;
        type ArrayGet = unsafe extern "C" fn(*const JsonT, usize) -> *mut JsonT;
        type ArraySet = unsafe extern "C" fn(*mut JsonT, usize, *mut JsonT) -> c_int;
        type ObjectSet =
            unsafe extern "C" fn(*mut JsonT, *const c_char, usize, *mut JsonT) -> c_int;
        type StringN = unsafe extern "C" fn(*const c_char, usize) -> *mut JsonT;
        type Dumpb = unsafe extern "C" fn(*const JsonT, *mut c_char, usize, usize) -> usize;
        type Dtostr = unsafe extern "C" fn(*mut c_char, usize, c_double, c_int) -> c_int;
        let real: Real = symbol(lib, b"json_real\0");
        let integer: Integer = symbol(lib, b"json_integer\0");
        let array: Array = symbol(lib, b"json_array\0");
        let get: ArrayGet = symbol(lib, b"json_array_get\0");
        let set: ArraySet = symbol(lib, b"json_array_set_new\0");
        let object_set: ObjectSet = symbol(lib, b"json_object_setn_new\0");
        let stringn: StringN = symbol(lib, b"json_stringn\0");
        let dumpb: Dumpb = symbol(lib, b"json_dumpb\0");
        let dtostr: Dtostr = symbol(lib, b"jsonp_dtostr\0");
        let mut rows = Vec::new();

        rows.push(vec![real(f64::NAN).is_null() as u8]);
        rows.push(vec![real(f64::INFINITY).is_null() as u8]);
        rows.push(vec![get(ptr::null(), 0).is_null() as u8]);
        let a = array();
        rows.push(vec![get(a, 0).is_null() as u8]);
        rows.push(set(a, usize::MAX, integer(1)).to_ne_bytes().to_vec());
        rows.push(
            object_set(a, ptr::null(), 0, integer(1))
                .to_ne_bytes()
                .to_vec(),
        );
        let malformed = b"\xc0\x80";
        rows.push(vec![
            stringn(malformed.as_ptr().cast(), malformed.len()).is_null() as u8,
        ]);
        let scalar = integer(1);
        rows.push(dumpb(scalar, ptr::null_mut(), 0, 0).to_ne_bytes().to_vec());
        let mut tiny = [0i8; 1];
        rows.push(
            dtostr(tiny.as_mut_ptr(), tiny.len(), 1.5, 17)
                .to_ne_bytes()
                .to_vec(),
        );
        delete(lib, scalar);
        delete(lib, a);

        let invalid_inputs: &[(&[u8], usize)] = &[
            (b"", 0),
            (b"{", 0),
            (b"[1,]", 0),
            (b"{\"a\":1,\"a\":2}", JSON_REJECT_DUPLICATES),
            (b"1 trailing", JSON_DECODE_ANY),
            (b"1 trailing", JSON_DECODE_ANY | JSON_DISABLE_EOF_CHECK),
            (b"\"\\uD800\"", JSON_DECODE_ANY),
            (b"1e99999", JSON_DECODE_ANY),
            (b"\"a\\u0000b\"", JSON_DECODE_ANY),
            (b"\"a\\u0000b\"", JSON_DECODE_ANY | JSON_ALLOW_NUL),
        ];
        for (input, flags) in invalid_inputs {
            let (value, error) = load_bytes(lib, input, *flags);
            rows.push(value.unwrap_or_else(|| b"<null>".to_vec()));
            rows.push(error);
        }
        rows
    }
    unsafe {
        let (c, rust) = load_pair();
        assert_eq!(exercise(&c), exercise(&rust));
    }
}

#[test]
fn file_and_callback_entry_points_match() {
    unsafe extern "C" fn collect(buffer: *const c_char, size: usize, data: *mut c_void) -> c_int {
        let output = unsafe { &mut *(data as *mut Vec<u8>) };
        output.extend_from_slice(unsafe { std::slice::from_raw_parts(buffer.cast(), size) });
        0
    }
    unsafe fn exercise(lib: &Library, input: &Path, output: &Path) -> Vec<u8> {
        type LoadFile = unsafe extern "C" fn(*const c_char, usize, *mut JsonError) -> *mut JsonT;
        type DumpFile = unsafe extern "C" fn(*const JsonT, *const c_char, usize) -> c_int;
        type LoadFd = unsafe extern "C" fn(c_int, usize, *mut JsonError) -> *mut JsonT;
        type DumpFd = unsafe extern "C" fn(*const JsonT, c_int, usize) -> c_int;
        type LoadF = unsafe extern "C" fn(*mut c_void, usize, *mut JsonError) -> *mut JsonT;
        type DumpF = unsafe extern "C" fn(*const JsonT, *mut c_void, usize) -> c_int;
        type DumpCallback = unsafe extern "C" fn(
            *const JsonT,
            Option<unsafe extern "C" fn(*const c_char, usize, *mut c_void) -> c_int>,
            *mut c_void,
            usize,
        ) -> c_int;
        let load: LoadFile = symbol(lib, b"json_load_file\0");
        let dump_file: DumpFile = symbol(lib, b"json_dump_file\0");
        let load_fd: LoadFd = symbol(lib, b"json_loadfd\0");
        let dump_fd: DumpFd = symbol(lib, b"json_dumpfd\0");
        let load_f: LoadF = symbol(lib, b"json_loadf\0");
        let dump_f: DumpF = symbol(lib, b"json_dumpf\0");
        let dump_callback: DumpCallback = symbol(lib, b"json_dump_callback\0");
        let input_c = CString::new(input.as_os_str().as_encoded_bytes()).unwrap();
        let output_c = CString::new(output.as_os_str().as_encoded_bytes()).unwrap();
        let mut error = JsonError::default();
        let value = load(input_c.as_ptr(), 0, &mut error);
        assert!(!value.is_null());
        assert_eq!(
            dump_file(value, output_c.as_ptr(), JSON_SORT_KEYS | JSON_COMPACT),
            0
        );
        let mut callback_output = Vec::new();
        assert_eq!(
            dump_callback(
                value,
                Some(collect),
                (&mut callback_output as *mut Vec<u8>).cast(),
                JSON_SORT_KEYS | JSON_COMPACT,
            ),
            0
        );
        delete(lib, value);
        let mut result = fs::read(output).unwrap();
        result.push(0xff);
        result.extend(callback_output);

        let input_file = fs::File::open(input).unwrap();
        let fd_value = load_fd(input_file.as_raw_fd(), 0, &mut error);
        assert!(!fd_value.is_null());
        let fd_path = output.with_extension("fd");
        let fd_file = fs::File::create(&fd_path).unwrap();
        assert_eq!(
            dump_fd(fd_value, fd_file.as_raw_fd(), JSON_SORT_KEYS | JSON_COMPACT,),
            0
        );
        delete(lib, fd_value);
        drop(fd_file);
        result.push(0xfe);
        result.extend(fs::read(fd_path).unwrap());

        let mode_r = CString::new("r").unwrap();
        let mode_w = CString::new("w").unwrap();
        let c_input = fopen(input_c.as_ptr(), mode_r.as_ptr());
        assert!(!c_input.is_null());
        let f_value = load_f(c_input, 0, &mut error);
        assert_eq!(fclose(c_input), 0);
        assert!(!f_value.is_null());
        let f_path = output.with_extension("file");
        let f_path_c = CString::new(f_path.as_os_str().as_encoded_bytes()).unwrap();
        let c_output = fopen(f_path_c.as_ptr(), mode_w.as_ptr());
        assert!(!c_output.is_null());
        assert_eq!(dump_f(f_value, c_output, JSON_SORT_KEYS | JSON_COMPACT), 0);
        assert_eq!(fclose(c_output), 0);
        delete(lib, f_value);
        result.push(0xfd);
        result.extend(fs::read(f_path).unwrap());
        result
    }

    let base = std::env::temp_dir().join(format!("jansson-diff-{}", std::process::id()));
    fs::create_dir_all(&base).unwrap();
    let input = base.join("input.json");
    let c_out = base.join("c.json");
    let rust_out = base.join("rust.json");
    fs::write(&input, br#"{"z":[3,2,1],"a":"x"}"#).unwrap();
    unsafe {
        let (c, rust) = load_pair();
        assert_eq!(
            exercise(&c, &input, &c_out),
            exercise(&rust, &input, &rust_out)
        );
    }
    let _ = fs::remove_dir_all(base);
}

#[test]
fn version_globals_allocators_and_dtoa_match() {
    unsafe fn exercise(lib: &Library) -> Vec<Vec<u8>> {
        type Version = unsafe extern "C" fn() -> *const c_char;
        type VersionCmp = unsafe extern "C" fn(c_int, c_int, c_int) -> c_int;
        type GetAlloc = unsafe extern "C" fn(*mut *mut c_void, *mut *mut c_void);
        type GetAlloc2 = unsafe extern "C" fn(*mut *mut c_void, *mut *mut c_void, *mut *mut c_void);
        type DtoaR = unsafe extern "C" fn(
            c_double,
            c_int,
            c_int,
            *mut c_int,
            *mut c_int,
            *mut *mut c_char,
            *mut c_char,
            usize,
        ) -> *mut c_char;
        let version: Version = symbol(lib, b"jansson_version_str\0");
        let compare: VersionCmp = symbol(lib, b"jansson_version_cmp\0");
        let get_alloc: GetAlloc = symbol(lib, b"json_get_alloc_funcs\0");
        let get_alloc2: GetAlloc2 = symbol(lib, b"json_get_alloc_funcs2\0");
        let dtoa: DtoaR = symbol(lib, b"dtoa_r\0");
        let mut rows = vec![CStr::from_ptr(version()).to_bytes().to_vec()];
        for triple in [(2, 14, 99), (2, 15, 0), (2, 15, 1), (3, 0, 0)] {
            rows.push(compare(triple.0, triple.1, triple.2).to_ne_bytes().to_vec());
        }
        let mut malloc = ptr::null_mut();
        let mut realloc = ptr::null_mut();
        let mut free = ptr::null_mut();
        get_alloc(&mut malloc, &mut free);
        let mut allocator_shape = vec![(!malloc.is_null()) as u8, (!free.is_null()) as u8];
        get_alloc2(&mut malloc, &mut realloc, &mut free);
        allocator_shape.extend([
            (!malloc.is_null()) as u8,
            (!realloc.is_null()) as u8,
            (!free.is_null()) as u8,
        ]);
        rows.push(allocator_shape);

        let mut state = 0x9e37_79b9_7f4a_7c15u64;
        let special = [
            -0.0,
            0.0,
            f64::from_bits(1),
            f64::MIN_POSITIVE,
            f64::MAX,
            f64::INFINITY,
            f64::NEG_INFINITY,
            f64::NAN,
        ];
        for value in special
            .into_iter()
            .chain((0..200).map(|_| f64::from_bits(lcg(&mut state))))
        {
            for mode in 0..=9 {
                for digits in [-3, 0, 1, 17, 31] {
                    let mut buffer = [0i8; 768];
                    let mut decpt = 0;
                    let mut sign = 0;
                    let mut end = ptr::null_mut();
                    let result = dtoa(
                        value,
                        mode,
                        digits,
                        &mut decpt,
                        &mut sign,
                        &mut end,
                        buffer.as_mut_ptr(),
                        buffer.len(),
                    );
                    let mut row = vec![result.is_null() as u8];
                    row.extend_from_slice(&decpt.to_ne_bytes());
                    row.extend_from_slice(&sign.to_ne_bytes());
                    if !result.is_null() {
                        let length = end.offset_from(result) as usize;
                        row.extend(buffer[..length].iter().map(|v| *v as u8));
                    }
                    rows.push(row);
                }
            }
        }
        rows
    }
    unsafe {
        let (c, rust) = load_pair();
        assert_eq!(exercise(&c), exercise(&rust));
    }
}

#[test]
fn updates_extensions_and_variadic_apis_match() {
    unsafe fn exercise(lib: &Library) -> Vec<Vec<u8>> {
        type Loads = unsafe extern "C" fn(*const c_char, usize, *mut JsonError) -> *mut JsonT;
        type Update = unsafe extern "C" fn(*mut JsonT, *mut JsonT) -> c_int;
        type Extend = unsafe extern "C" fn(*mut JsonT, *mut JsonT) -> c_int;
        type Pack = unsafe extern "C" fn(*const c_char, ...) -> *mut JsonT;
        type PackEx = unsafe extern "C" fn(*mut JsonError, usize, *const c_char, ...) -> *mut JsonT;
        type Unpack = unsafe extern "C" fn(*mut JsonT, *const c_char, ...) -> c_int;
        type UnpackEx =
            unsafe extern "C" fn(*mut JsonT, *mut JsonError, usize, *const c_char, ...) -> c_int;
        type Sprintf = unsafe extern "C" fn(*const c_char, ...) -> *mut JsonT;
        let loads: Loads = symbol(lib, b"json_loads\0");
        let update: Update = symbol(lib, b"json_object_update\0");
        let existing: Update = symbol(lib, b"json_object_update_existing\0");
        let missing: Update = symbol(lib, b"json_object_update_missing\0");
        let recursive: Update = symbol(lib, b"json_object_update_recursive\0");
        let extend: Extend = symbol(lib, b"json_array_extend\0");
        let pack: Pack = symbol(lib, b"json_pack\0");
        let pack_ex: PackEx = symbol(lib, b"json_pack_ex\0");
        let unpack: Unpack = symbol(lib, b"json_unpack\0");
        let unpack_ex: UnpackEx = symbol(lib, b"json_unpack_ex\0");
        let sprintf: Sprintf = symbol(lib, b"json_sprintf\0");
        let mut rows = Vec::new();

        for function in [update, existing, missing, recursive] {
            let left_text = CString::new(r#"{"a":1,"nested":{"x":1}}"#).unwrap();
            let right_text = CString::new(r#"{"a":2,"b":3,"nested":{"y":2}}"#).unwrap();
            let left = loads(left_text.as_ptr(), 0, ptr::null_mut());
            let right = loads(right_text.as_ptr(), 0, ptr::null_mut());
            rows.push(function(left, right).to_ne_bytes().to_vec());
            rows.push(dumped(lib, left, JSON_SORT_KEYS | JSON_COMPACT).unwrap());
            delete(lib, right);
            delete(lib, left);
        }

        let array_text = CString::new("[1,2,3]").unwrap();
        let array = loads(array_text.as_ptr(), 0, ptr::null_mut());
        rows.push(extend(array, array).to_ne_bytes().to_vec());
        rows.push(dumped(lib, array, JSON_COMPACT).unwrap());
        delete(lib, array);

        let format = CString::new("[i,I,s,b,n]").unwrap();
        let text = CString::new("hello").unwrap();
        let packed = pack(
            format.as_ptr(),
            7 as c_int,
            -9i64,
            text.as_ptr(),
            1 as c_int,
        );
        rows.push(dumped(lib, packed, JSON_COMPACT).unwrap());
        let unpack_format = CString::new("[i,I,s,b,n]").unwrap();
        let mut small = 0 as c_int;
        let mut wide = 0i64;
        let mut string = ptr::null::<c_char>();
        let mut boolean = 0 as c_int;
        rows.push(
            unpack(
                packed,
                unpack_format.as_ptr(),
                &mut small,
                &mut wide,
                &mut string,
                &mut boolean,
            )
            .to_ne_bytes()
            .to_vec(),
        );
        let mut values = Vec::new();
        values.extend_from_slice(&small.to_ne_bytes());
        values.extend_from_slice(&wide.to_ne_bytes());
        values.extend_from_slice(&boolean.to_ne_bytes());
        values.extend_from_slice(CStr::from_ptr(string).to_bytes());
        rows.push(values);
        delete(lib, packed);

        let mut error = JsonError::default();
        let bad_format = CString::new("{s:i").unwrap();
        let bad = pack_ex(
            &mut error,
            0,
            bad_format.as_ptr(),
            text.as_ptr(),
            1 as c_int,
        );
        rows.push(vec![bad.is_null() as u8]);
        rows.push(error_bytes(&error));

        let strict_input = CString::new("[1,2]").unwrap();
        let strict_root = loads(strict_input.as_ptr(), 0, ptr::null_mut());
        let one_item = CString::new("[i]").unwrap();
        let mut value = 0;
        error = JsonError::default();
        rows.push(
            unpack_ex(strict_root, &mut error, 0x2, one_item.as_ptr(), &mut value)
                .to_ne_bytes()
                .to_vec(),
        );
        rows.push(error_bytes(&error));
        delete(lib, strict_root);

        let sprintf_format = CString::new("%d-%s").unwrap();
        let rendered = sprintf(sprintf_format.as_ptr(), 12 as c_int, text.as_ptr());
        rows.push(dumped(lib, rendered, JSON_ENCODE_ANY).unwrap());
        delete(lib, rendered);
        rows
    }
    unsafe {
        let (c, rust) = load_pair();
        assert_eq!(exercise(&c), exercise(&rust));
    }
}

#[test]
fn callback_loader_matches() {
    struct Reader {
        bytes: Vec<u8>,
        offset: usize,
        chunk: usize,
    }
    unsafe extern "C" fn reader(buffer: *mut c_void, size: usize, data: *mut c_void) -> usize {
        let reader = unsafe { &mut *(data as *mut Reader) };
        let count = reader
            .chunk
            .min(size)
            .min(reader.bytes.len().saturating_sub(reader.offset));
        if count != 0 {
            unsafe {
                ptr::copy_nonoverlapping(
                    reader.bytes.as_ptr().add(reader.offset),
                    buffer.cast::<u8>(),
                    count,
                )
            };
            reader.offset += count;
        }
        count
    }
    unsafe fn exercise(lib: &Library, chunk: usize) -> (Option<Vec<u8>>, Vec<u8>) {
        type LoadCallback = unsafe extern "C" fn(
            Option<unsafe extern "C" fn(*mut c_void, usize, *mut c_void) -> usize>,
            *mut c_void,
            usize,
            *mut JsonError,
        ) -> *mut JsonT;
        let load: LoadCallback = symbol(lib, b"json_load_callback\0");
        let mut reader_state = Reader {
            bytes: br#"{"z":[1,2,3],"a":"callback"}"#.to_vec(),
            offset: 0,
            chunk,
        };
        let mut error = JsonError::default();
        let value = load(
            Some(reader),
            (&mut reader_state as *mut Reader).cast(),
            0,
            &mut error,
        );
        let output = if value.is_null() {
            None
        } else {
            let result = dumped(lib, value, JSON_SORT_KEYS | JSON_COMPACT);
            delete(lib, value);
            result
        };
        (output, error_bytes(&error))
    }
    unsafe {
        let (c, rust) = load_pair();
        for chunk in [1, 2, 7, 4096] {
            assert_eq!(exercise(&c, chunk), exercise(&rust, chunk));
        }
    }
}
