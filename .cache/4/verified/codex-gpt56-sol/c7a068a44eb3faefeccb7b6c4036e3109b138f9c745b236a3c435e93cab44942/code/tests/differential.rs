#![allow(unsafe_op_in_unsafe_fn)]

use libloading::Library;
use std::ffi::{CStr, CString, c_char, c_int, c_void};
use std::fs;
use std::mem::MaybeUninit;
use std::os::unix::process::ExitStatusExt;
use std::path::{Path, PathBuf};
use std::ptr;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

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
const JSON_VALIDATE_ONLY: usize = 0x1;
const JSON_STRICT: usize = 0x2;

static FAIL_ALLOCATIONS: AtomicBool = AtomicBool::new(false);
static FREE_CALLS: AtomicUsize = AtomicUsize::new(0);
static FAIL_AT_ALLOCATION: AtomicUsize = AtomicUsize::new(0);
static ALLOCATION_CALLS: AtomicUsize = AtomicUsize::new(0);

fn allocation_should_fail() -> bool {
    if FAIL_ALLOCATIONS.load(Ordering::SeqCst) {
        return true;
    }
    let fail_at = FAIL_AT_ALLOCATION.load(Ordering::SeqCst);
    fail_at != 0 && ALLOCATION_CALLS.fetch_add(1, Ordering::SeqCst) + 1 == fail_at
}

unsafe extern "C" fn test_malloc(size: usize) -> *mut c_void {
    if allocation_should_fail() {
        ptr::null_mut()
    } else {
        libc::malloc(size)
    }
}

unsafe extern "C" fn test_realloc(value: *mut c_void, size: usize) -> *mut c_void {
    if allocation_should_fail() {
        ptr::null_mut()
    } else {
        libc::realloc(value, size)
    }
}

unsafe extern "C" fn test_free(value: *mut c_void) {
    FREE_CALLS.fetch_add(1, Ordering::SeqCst);
    libc::free(value);
}

type Json = c_void;

#[repr(C)]
#[derive(Clone, Copy)]
struct JsonError {
    line: c_int,
    column: c_int,
    position: c_int,
    source: [c_char; 80],
    text: [c_char; 160],
}

impl Default for JsonError {
    fn default() -> Self {
        unsafe { std::mem::zeroed() }
    }
}

impl JsonError {
    fn code(&self) -> u8 {
        self.text[159] as u8
    }
}

#[repr(C)]
#[derive(Clone, Copy)]
struct StrBuffer {
    value: *mut c_char,
    length: usize,
    size: usize,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct List {
    prev: *mut List,
    next: *mut List,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct Bucket {
    first: *mut List,
    last: *mut List,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct HashTable {
    size: usize,
    buckets: *mut Bucket,
    order: usize,
    list: List,
    ordered_list: List,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct TestVaList {
    gp_offset: u32,
    fp_offset: u32,
    overflow_arg_area: *mut u8,
    reg_save_area: *mut u8,
}

struct Api {
    name: &'static str,
    library: Library,
}

impl Api {
    unsafe fn open(name: &'static str, path: &Path) -> Self {
        assert!(path.exists(), "{name} library missing: {}", path.display());
        let unix = unsafe {
            libloading::os::unix::Library::open(
                Some(path),
                libloading::os::unix::RTLD_NOW
                    | libloading::os::unix::RTLD_LOCAL
                    | libc::RTLD_DEEPBIND,
            )
        }
        .unwrap();
        Self {
            name,
            library: unix.into(),
        }
    }

    unsafe fn sym<T: Copy>(&self, name: &[u8]) -> T {
        *unsafe { self.library.get::<T>(name) }
            .unwrap_or_else(|error| panic!("{} missing {:?}: {error}", self.name, name))
    }

    unsafe fn delete(&self, value: *mut Json) {
        let function: unsafe extern "C" fn(*mut Json) = unsafe { self.sym(b"json_delete") };
        unsafe { function(value) };
    }

    unsafe fn dump(&self, value: *const Json, flags: usize) -> Option<Vec<u8>> {
        let dumps: unsafe extern "C" fn(*const Json, usize) -> *mut c_char =
            unsafe { self.sym(b"json_dumps") };
        let free: unsafe extern "C" fn(*mut c_void) = unsafe { self.sym(b"jsonp_free") };
        let output = unsafe { dumps(value, flags) };
        if output.is_null() {
            return None;
        }
        let bytes = unsafe { CStr::from_ptr(output) }.to_bytes().to_vec();
        unsafe { free(output.cast()) };
        Some(bytes)
    }

    unsafe fn loadb(&self, bytes: &[u8], flags: usize) -> (*mut Json, JsonError) {
        let function: unsafe extern "C" fn(
            *const c_char,
            usize,
            usize,
            *mut JsonError,
        ) -> *mut Json = unsafe { self.sym(b"json_loadb") };
        let mut error = JsonError::default();
        let value = unsafe { function(bytes.as_ptr().cast(), bytes.len(), flags, &mut error) };
        (value, error)
    }
}

struct Libraries {
    c: Api,
    rust: Api,
}

impl Libraries {
    unsafe fn open() -> Self {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        Self {
            c: unsafe { Api::open("C", &root.join("c_src/build/libjansson.so")) },
            rust: unsafe { Api::open("Rust", &root.join("target/release/libjansson.so")) },
        }
    }
}

fn compare_load(libs: &Libraries, bytes: &[u8], flags: usize, dump_flags: &[usize]) {
    unsafe {
        let (c_value, c_error) = libs.c.loadb(bytes, flags);
        let (rust_value, rust_error) = libs.rust.loadb(bytes, flags);
        assert_eq!(
            c_value.is_null(),
            rust_value.is_null(),
            "load result differs for input {:?}, flags {flags:#x}",
            String::from_utf8_lossy(bytes)
        );
        if c_value.is_null() {
            assert_eq!(
                c_error.code(),
                rust_error.code(),
                "error code differs for input {:?}, flags {flags:#x}",
                String::from_utf8_lossy(bytes)
            );
        } else {
            for &dump_flag in dump_flags {
                assert_eq!(
                    libs.c.dump(c_value, dump_flag),
                    libs.rust.dump(rust_value, dump_flag),
                    "dump differs for input {:?}, load {flags:#x}, dump {dump_flag:#x}",
                    String::from_utf8_lossy(bytes)
                );
            }
            libs.c.delete(c_value);
            libs.rust.delete(rust_value);
        }
    }
}

fn xorshift(state: &mut u64) -> u64 {
    *state ^= *state << 13;
    *state ^= *state >> 7;
    *state ^= *state << 17;
    *state
}

fn random_string(state: &mut u64) -> String {
    let atoms = ["", "a", "Z", "/", "\"", "\\", "\n", "é", "😀", "\u{1f}"];
    let mut value = String::new();
    for _ in 0..(xorshift(state) % 7) {
        value.push_str(atoms[(xorshift(state) as usize) % atoms.len()]);
    }
    let mut escaped = String::new();
    for character in value.chars() {
        match character {
            '"' => escaped.push_str("\\\""),
            '\\' => escaped.push_str("\\\\"),
            '/' if xorshift(state) & 1 != 0 => escaped.push_str("\\/"),
            '\n' => escaped.push_str("\\n"),
            c if c < ' ' => escaped.push_str(&format!("\\u{:04X}", c as u32)),
            c => escaped.push(c),
        }
    }
    escaped
}

fn random_json(state: &mut u64, depth: usize) -> String {
    if depth == 0 {
        match xorshift(state) % 6 {
            0 => "null".into(),
            1 => (xorshift(state) & 1 != 0).to_string(),
            2 => (xorshift(state) as i64).to_string(),
            3 => format!(
                "{}.{}e{}",
                xorshift(state) % 1000,
                xorshift(state) % 1000,
                (xorshift(state) % 20) as i32 - 10
            ),
            _ => format!("\"{}\"", random_string(state)),
        }
    } else if xorshift(state) & 1 == 0 {
        let mut parts = Vec::new();
        for _ in 0..(xorshift(state) % 5) {
            parts.push(random_json(state, depth - 1));
        }
        format!("[{}]", parts.join(","))
    } else {
        let mut parts = Vec::new();
        for index in 0..(xorshift(state) % 5) {
            parts.push(format!(
                "\"k{index}_{}\":{}",
                random_string(state),
                random_json(state, depth - 1)
            ));
        }
        format!("{{{}}}", parts.join(","))
    }
}

#[test]
fn phase_a_and_d_symbol_parity() {
    unsafe {
        let libs = Libraries::open();
        let documented: Vec<_> = include_str!("../SYMBOLS.md")
            .lines()
            .filter_map(|line| {
                let line = line.strip_prefix("| ")?;
                let (_, rest) = line.split_once(" | `")?;
                let (symbol, _) = rest.split_once('`')?;
                Some(symbol.to_owned())
            })
            .collect();
        assert_eq!(documented.len(), 130);
        for symbol in documented {
            let mut nul = symbol.as_bytes().to_vec();
            nul.push(0);
            let _: *mut c_void = libs.c.sym(&nul);
            let _: *mut c_void = libs.rust.sym(&nul);
        }
    }
}

#[test]
fn configs_17_through_36_randomized_load_dump_matrix() {
    unsafe {
        let libs = Libraries::open();
        let dump_flags = [
            JSON_COMPACT,
            JSON_SORT_KEYS,
            1,
            2 | JSON_SORT_KEYS,
            31 | JSON_COMPACT,
            JSON_ENSURE_ASCII | JSON_COMPACT,
            JSON_ESCAPE_SLASH | JSON_COMPACT,
            JSON_PRESERVE_ORDER | JSON_COMPACT,
            JSON_EMBED | JSON_COMPACT,
        ];
        let fixed = [
            "{}",
            "[]",
            "{\"\":0}",
            "[0,-0,1.0,1e2,1E-2]",
            "{\"emoji\":\"\\uD83D\\uDE00\",\"nul\":\"\\u0000\"}",
            "{\"esc\":\"\\\\\\\"\\/\\b\\f\\n\\r\\t\"}",
            "{\"b\":2,\"a\":1,\"aa\":3,\"A\":4}",
            "[[[[[null]]]]]",
            "9223372036854775807",
            "-9223372036854775808",
        ];
        for input in fixed {
            for flags in [
                0,
                JSON_DECODE_ANY,
                JSON_ALLOW_NUL | JSON_DECODE_ANY,
                JSON_REJECT_DUPLICATES | JSON_DECODE_ANY,
                JSON_DISABLE_EOF_CHECK | JSON_DECODE_ANY,
                JSON_DECODE_INT_AS_REAL | JSON_DECODE_ANY,
                JSON_REJECT_DUPLICATES
                    | JSON_DISABLE_EOF_CHECK
                    | JSON_DECODE_ANY
                    | JSON_DECODE_INT_AS_REAL
                    | JSON_ALLOW_NUL,
            ] {
                compare_load(&libs, input.as_bytes(), flags, &dump_flags);
            }
        }
        let mut seed = 0x4a41_4e53_534f_4e15;
        for _ in 0..400 {
            let input = random_json(&mut seed, 3);
            compare_load(&libs, input.as_bytes(), 0, &dump_flags);
        }
    }
}

unsafe fn value_transcript(api: &Api, seed: u64) -> Vec<Vec<u8>> {
    let object: unsafe extern "C" fn() -> *mut Json = unsafe { api.sym(b"json_object") };
    let array: unsafe extern "C" fn() -> *mut Json = unsafe { api.sym(b"json_array") };
    let integer: unsafe extern "C" fn(i64) -> *mut Json = unsafe { api.sym(b"json_integer") };
    let real: unsafe extern "C" fn(f64) -> *mut Json = unsafe { api.sym(b"json_real") };
    let stringn: unsafe extern "C" fn(*const c_char, usize) -> *mut Json =
        unsafe { api.sym(b"json_stringn") };
    let setn: unsafe extern "C" fn(*mut Json, *const c_char, usize, *mut Json) -> c_int =
        unsafe { api.sym(b"json_object_setn_new") };
    let setn_nc: unsafe extern "C" fn(*mut Json, *const c_char, usize, *mut Json) -> c_int =
        unsafe { api.sym(b"json_object_setn_new_nocheck") };
    let append: unsafe extern "C" fn(*mut Json, *mut Json) -> c_int =
        unsafe { api.sym(b"json_array_append_new") };
    let insert: unsafe extern "C" fn(*mut Json, usize, *mut Json) -> c_int =
        unsafe { api.sym(b"json_array_insert_new") };
    let set_array: unsafe extern "C" fn(*mut Json, usize, *mut Json) -> c_int =
        unsafe { api.sym(b"json_array_set_new") };
    let remove: unsafe extern "C" fn(*mut Json, usize) -> c_int =
        unsafe { api.sym(b"json_array_remove") };
    let update: unsafe extern "C" fn(*mut Json, *mut Json) -> c_int =
        unsafe { api.sym(b"json_object_update") };
    let update_existing: unsafe extern "C" fn(*mut Json, *mut Json) -> c_int =
        unsafe { api.sym(b"json_object_update_existing") };
    let update_missing: unsafe extern "C" fn(*mut Json, *mut Json) -> c_int =
        unsafe { api.sym(b"json_object_update_missing") };
    let update_recursive: unsafe extern "C" fn(*mut Json, *mut Json) -> c_int =
        unsafe { api.sym(b"json_object_update_recursive") };
    let copy: unsafe extern "C" fn(*mut Json) -> *mut Json = unsafe { api.sym(b"json_copy") };
    let deep_copy: unsafe extern "C" fn(*const Json) -> *mut Json =
        unsafe { api.sym(b"json_deep_copy") };
    let equal: unsafe extern "C" fn(*const Json, *const Json) -> c_int =
        unsafe { api.sym(b"json_equal") };
    let iter_first: unsafe extern "C" fn(*mut Json) -> *mut c_void =
        unsafe { api.sym(b"json_object_iter") };
    let iter_next: unsafe extern "C" fn(*mut Json, *mut c_void) -> *mut c_void =
        unsafe { api.sym(b"json_object_iter_next") };
    let iter_key: unsafe extern "C" fn(*mut c_void) -> *const c_char =
        unsafe { api.sym(b"json_object_iter_key") };
    let iter_key_len: unsafe extern "C" fn(*mut c_void) -> usize =
        unsafe { api.sym(b"json_object_iter_key_len") };
    let iter_value: unsafe extern "C" fn(*mut c_void) -> *mut Json =
        unsafe { api.sym(b"json_object_iter_value") };
    let iter_set: unsafe extern "C" fn(*mut Json, *mut c_void, *mut Json) -> c_int =
        unsafe { api.sym(b"json_object_iter_set_new") };

    let root = unsafe { object() };
    let numbers = unsafe { array() };
    let mut state = seed;
    for _ in 0..24 {
        unsafe { append(numbers, integer(xorshift(&mut state) as i64)) };
    }
    unsafe {
        insert(numbers, 0, real(-0.0));
        set_array(numbers, 3, integer(i64::MIN));
        remove(numbers, 7);
        setn(root, c"numbers".as_ptr(), 7, numbers);
    }
    let text = b"a\0b";
    unsafe {
        setn_nc(
            root,
            b"k\0x".as_ptr().cast(),
            3,
            stringn(text.as_ptr().cast(), text.len()),
        );
    }

    let mut transcript = vec![unsafe { api.dump(root, JSON_COMPACT | JSON_SORT_KEYS).unwrap() }];
    let mut iter = unsafe { iter_first(root) };
    while !iter.is_null() {
        let key = unsafe {
            std::slice::from_raw_parts(iter_key(iter).cast(), iter_key_len(iter)).to_vec()
        };
        transcript.push(key);
        assert!(!unsafe { iter_value(iter) }.is_null());
        iter = unsafe { iter_next(root, iter) };
    }
    iter = unsafe { iter_first(root) };
    unsafe { iter_set(root, iter, integer(77)) };
    transcript.push(unsafe { api.dump(root, JSON_COMPACT).unwrap() });

    for which in 0..4 {
        let other = unsafe { object() };
        unsafe {
            setn(other, c"numbers".as_ptr(), 7, array());
            setn(other, c"new".as_ptr(), 3, integer(which));
        }
        let result = match which {
            0 => unsafe { update(root, other) },
            1 => unsafe { update_existing(root, other) },
            2 => unsafe { update_missing(root, other) },
            _ => unsafe { update_recursive(root, other) },
        };
        transcript.push(result.to_string().into_bytes());
        transcript.push(unsafe { api.dump(root, JSON_COMPACT | JSON_SORT_KEYS).unwrap() });
        unsafe { api.delete(other) };
    }

    let shallow = unsafe { copy(root) };
    let deep = unsafe { deep_copy(root) };
    transcript.push(
        format!(
            "{},{},{}",
            unsafe { equal(root, shallow) },
            unsafe { equal(root, deep) },
            unsafe { equal(shallow, deep) }
        )
        .into_bytes(),
    );
    unsafe {
        api.delete(shallow);
        api.delete(deep);
        api.delete(root);
    }
    transcript
}

#[test]
fn configs_2_through_16_randomized_value_api() {
    unsafe {
        let libs = Libraries::open();
        for seed in 0..64 {
            assert_eq!(
                value_transcript(&libs.c, seed),
                value_transcript(&libs.rust, seed),
                "value API differs for seed {seed}"
            );
        }
    }
}

unsafe fn variadic_transcript(api: &Api) -> Vec<Vec<u8>> {
    type Pack = unsafe extern "C" fn(*const c_char, ...) -> *mut Json;
    type PackEx = unsafe extern "C" fn(*mut JsonError, usize, *const c_char, ...) -> *mut Json;
    type Unpack = unsafe extern "C" fn(*mut Json, *const c_char, ...) -> c_int;
    type UnpackEx =
        unsafe extern "C" fn(*mut Json, *mut JsonError, usize, *const c_char, ...) -> c_int;
    type Sprintf = unsafe extern "C" fn(*const c_char, ...) -> *mut Json;
    type VPack =
        unsafe extern "C" fn(*mut JsonError, usize, *const c_char, *mut TestVaList) -> *mut Json;
    type VUnpack = unsafe extern "C" fn(
        *mut Json,
        *mut JsonError,
        usize,
        *const c_char,
        *mut TestVaList,
    ) -> c_int;
    type VSprintf = unsafe extern "C" fn(*const c_char, *mut TestVaList) -> *mut Json;
    let pack: Pack = unsafe { api.sym(b"json_pack") };
    let pack_ex: PackEx = unsafe { api.sym(b"json_pack_ex") };
    let unpack: Unpack = unsafe { api.sym(b"json_unpack") };
    let unpack_ex: UnpackEx = unsafe { api.sym(b"json_unpack_ex") };
    let sprintf: Sprintf = unsafe { api.sym(b"json_sprintf") };
    let vpack: VPack = unsafe { api.sym(b"json_vpack_ex") };
    let vunpack: VUnpack = unsafe { api.sym(b"json_vunpack_ex") };
    let vsprintf: VSprintf = unsafe { api.sym(b"json_vsprintf") };

    let mut output = Vec::new();
    let mut register_save = [0u64; 22];
    register_save[0] = (-123_i32 as u32) as u64;
    let mut overflow = [0u64; 2];
    let mut va = TestVaList {
        gp_offset: 0,
        fp_offset: 48,
        overflow_arg_area: overflow.as_mut_ptr().cast(),
        reg_save_area: register_save.as_mut_ptr().cast(),
    };
    let mut direct_error = JsonError::default();
    let direct = vpack(&mut direct_error, 0, c"i".as_ptr(), &mut va);
    output.push(api.dump(direct, JSON_ENCODE_ANY).unwrap());
    let mut direct_integer = 0;
    register_save[0] = (&mut direct_integer as *mut c_int) as usize as u64;
    va.gp_offset = 0;
    output.push(
        format!(
            "{},{}",
            vunpack(direct, &mut direct_error, 0, c"i".as_ptr(), &mut va),
            direct_integer
        )
        .into_bytes(),
    );
    ptr::write_volatile(register_save.as_mut_ptr(), 77);
    va.gp_offset = 0;
    let direct_string = vsprintf(c"%d".as_ptr(), &mut va);
    output.push(api.dump(direct_string, JSON_ENCODE_ANY).unwrap());
    api.delete(direct_string);
    api.delete(direct);

    let packed = unsafe {
        pack(
            c"[s,i,I,f,b,n,{s:s#}]".as_ptr(),
            c"word".as_ptr(),
            -4 as c_int,
            1_234_567_890_123_i64,
            2.5_f64,
            1 as c_int,
            c"key".as_ptr(),
            c"abcdef".as_ptr(),
            3 as c_int,
        )
    };
    output.push(unsafe { api.dump(packed, JSON_COMPACT).unwrap() });
    let mut word: *const c_char = ptr::null();
    let mut small = 0;
    let mut large = 0_i64;
    let mut number = 0_f64;
    let mut boolean = 0;
    let mut object: *mut Json = ptr::null_mut();
    let result = unsafe {
        unpack(
            packed,
            c"[s,i,I,F,b,n,o]".as_ptr(),
            &mut word,
            &mut small,
            &mut large,
            &mut number,
            &mut boolean,
            &mut object,
        )
    };
    output.push(
        format!(
            "{result},{},{small},{large},{number:.17},{boolean},{}",
            unsafe { CStr::from_ptr(word) }.to_string_lossy(),
            !object.is_null()
        )
        .into_bytes(),
    );

    let mut error = JsonError::default();
    let lengths = unsafe {
        pack(
            c"[s#,s%]".as_ptr(),
            c"abcdef".as_ptr(),
            3 as c_int,
            c"uvwxyz".as_ptr(),
            4_usize,
        )
    };
    output.push(unsafe { api.dump(lengths, JSON_COMPACT).unwrap() });
    let concatenated = unsafe { pack(c"[s+]".as_ptr(), c"ab".as_ptr(), c"cdef".as_ptr()) };
    output.push(unsafe { api.dump(concatenated, JSON_COMPACT).unwrap() });
    let optional = unsafe {
        pack_ex(
            &mut error,
            0,
            c"[s?,s*,O?,o*]".as_ptr(),
            ptr::null::<c_char>(),
            ptr::null::<c_char>(),
            ptr::null_mut::<Json>(),
            ptr::null_mut::<Json>(),
        )
    };
    output.push(unsafe { api.dump(optional, JSON_COMPACT).unwrap() });

    let root = unsafe {
        pack(
            c"{s:[i,s,{s:b}],s:f}".as_ptr(),
            c"items".as_ptr(),
            7 as c_int,
            c"word".as_ptr(),
            c"yes".as_ptr(),
            1 as c_int,
            c"real".as_ptr(),
            2.5_f64,
        )
    };
    let mut integer = 0;
    let mut string: *const c_char = ptr::null();
    let mut truth = 0;
    let mut real = 0.0;
    error = JsonError::default();
    let result = unsafe {
        unpack_ex(
            root,
            &mut error,
            JSON_STRICT,
            c"{s:[i,s,{s:b}],s:F}".as_ptr(),
            c"items".as_ptr(),
            &mut integer,
            &mut string,
            c"yes".as_ptr(),
            &mut truth,
            c"real".as_ptr(),
            &mut real,
        )
    };
    output.push(
        format!(
            "{result},{integer},{},{truth},{real:.17},{}",
            unsafe { CStr::from_ptr(string) }.to_string_lossy(),
            error.code()
        )
        .into_bytes(),
    );
    error = JsonError::default();
    let validate = unsafe {
        unpack_ex(
            root,
            &mut error,
            JSON_VALIDATE_ONLY,
            c"{s:[i,s,{s:b}],s:F}".as_ptr(),
            c"items".as_ptr(),
            c"yes".as_ptr(),
            c"real".as_ptr(),
        )
    };
    output.push(format!("{validate},{}", error.code()).into_bytes());

    for (format, string_arg, integer_arg, real_arg) in [
        (c"%s:%d:%.2f", c"fmt", 17, 1.5),
        (c"[%10s]/%+06d/%g", c"é", -9, 0.00001),
        (c"", c"", 0, 0.0),
    ] {
        let value = unsafe {
            sprintf(
                format.as_ptr(),
                string_arg.as_ptr(),
                integer_arg as c_int,
                real_arg,
            )
        };
        output.push(unsafe { api.dump(value, JSON_ENCODE_ANY).unwrap() });
        unsafe { api.delete(value) };
    }
    let invalid_format_string = [0xc0_u8, 0];
    output.push(
        sprintf(
            c"%s".as_ptr(),
            invalid_format_string.as_ptr().cast::<c_char>(),
        )
        .is_null()
        .to_string()
        .into_bytes(),
    );
    let invalid_format = sprintf(c"%".as_ptr());
    output.push(invalid_format.is_null().to_string().into_bytes());
    if !invalid_format.is_null() {
        api.delete(invalid_format);
    }
    unsafe {
        api.delete(root);
        api.delete(optional);
        api.delete(concatenated);
        api.delete(lengths);
        api.delete(packed);
    }
    output
}

#[test]
fn configs_37_through_45_variadic_abi() {
    unsafe {
        let libs = Libraries::open();
        assert_eq!(
            variadic_transcript(&libs.c),
            variadic_transcript(&libs.rust)
        );
    }
}

unsafe fn variadic_error_transcript(api: &Api) -> Vec<(c_int, u8)> {
    type PackEx = unsafe extern "C" fn(*mut JsonError, usize, *const c_char, ...) -> *mut Json;
    type UnpackEx =
        unsafe extern "C" fn(*mut Json, *mut JsonError, usize, *const c_char, ...) -> c_int;
    let pack: PackEx = api.sym(b"json_pack_ex");
    let unpack: UnpackEx = api.sym(b"json_unpack_ex");
    let mut output = Vec::new();

    macro_rules! pack_case {
        ($format:expr $(, $arg:expr)* $(,)?) => {{
            let mut error = JsonError::default();
            let value = pack(&mut error, 0, $format $(, $arg)*);
            output.push((value.is_null() as c_int, error.code()));
            if !value.is_null() {
                api.delete(value);
            }
        }};
    }

    pack_case!(ptr::null::<c_char>());
    pack_case!(c"".as_ptr());
    pack_case!(c"s".as_ptr(), ptr::null::<c_char>());
    pack_case!(c"{s:i}".as_ptr(), ptr::null::<c_char>(), 1 as c_int);
    let invalid_utf8 = [0xc0_u8, 0x80, 0];
    pack_case!(c"s".as_ptr(), invalid_utf8.as_ptr().cast::<c_char>());
    pack_case!(c"s?#".as_ptr(), c"x".as_ptr(), 1 as c_int);
    pack_case!(c"{".as_ptr());
    pack_case!(c"[".as_ptr());
    pack_case!(c"{i}".as_ptr());
    pack_case!(c"{s:s}".as_ptr(), c"k".as_ptr(), ptr::null::<c_char>());
    pack_case!(c"o".as_ptr(), ptr::null_mut::<Json>());
    pack_case!(c"O".as_ptr(), ptr::null_mut::<Json>());
    pack_case!(c"f".as_ptr(), f64::NAN);
    pack_case!(c"q".as_ptr());
    pack_case!(c"n n".as_ptr());

    let object = api.loadb(b"{\"a\":1,\"b\":2}", 0).0;
    let array = api.loadb(b"[1,2]", 0).0;
    let integer = api.loadb(b"1", JSON_DECODE_ANY).0;
    let string = api.loadb(b"\"x\"", JSON_DECODE_ANY).0;
    let real = api.loadb(b"1.5", JSON_DECODE_ANY).0;
    let boolean = api.loadb(b"true", JSON_DECODE_ANY).0;
    let null = api.loadb(b"null", JSON_DECODE_ANY).0;

    macro_rules! unpack_case {
        ($root:expr, $flags:expr, $format:expr $(, $arg:expr)* $(,)?) => {{
            let mut error = JsonError::default();
            let result = unpack($root, &mut error, $flags, $format $(, $arg)*);
            output.push((result, error.code()));
        }};
    }

    unpack_case!(ptr::null_mut(), 0, c"i".as_ptr(), ptr::null_mut::<c_int>());
    unpack_case!(integer, 0, ptr::null::<c_char>());
    unpack_case!(integer, 0, c"".as_ptr());
    unpack_case!(integer, 0, c"{}".as_ptr());
    unpack_case!(integer, 0, c"[]".as_ptr());
    let mut integer_out = 0;
    unpack_case!(
        object,
        0,
        c"{!s:i}".as_ptr(),
        c"a".as_ptr(),
        &mut integer_out
    );
    unpack_case!(object, 0, c"{".as_ptr());
    unpack_case!(array, 0, c"[".as_ptr());
    unpack_case!(object, 0, c"{i}".as_ptr(), &mut integer_out);
    unpack_case!(
        object,
        0,
        c"{s:i}".as_ptr(),
        ptr::null::<c_char>(),
        &mut integer_out
    );
    unpack_case!(
        object,
        0,
        c"{s:i}".as_ptr(),
        c"missing".as_ptr(),
        &mut integer_out
    );
    unpack_case!(
        object,
        0,
        c"{s:i!}".as_ptr(),
        c"a".as_ptr(),
        &mut integer_out
    );
    unpack_case!(array, 0, c"[q]".as_ptr());
    let mut integer_out_2 = 0;
    let mut integer_out_3 = 0;
    unpack_case!(
        array,
        0,
        c"[iii]".as_ptr(),
        &mut integer_out,
        &mut integer_out_2,
        &mut integer_out_3
    );
    unpack_case!(array, 0, c"[i!]".as_ptr(), &mut integer_out);

    let mut string_out: *const c_char = ptr::null();
    let mut int64_out = 0_i64;
    let mut bool_out = 0;
    let mut real_out = 0.0_f64;
    for (root, format) in [
        (integer, c"s".as_ptr()),
        (string, c"i".as_ptr()),
        (string, c"I".as_ptr()),
        (integer, c"b".as_ptr()),
        (integer, c"f".as_ptr()),
        (string, c"F".as_ptr()),
        (integer, c"n".as_ptr()),
    ] {
        let mut error = JsonError::default();
        let result = match CStr::from_ptr(format).to_bytes()[0] {
            b's' => unpack(root, &mut error, 0, format, &mut string_out),
            b'i' => unpack(root, &mut error, 0, format, &mut integer_out),
            b'I' => unpack(root, &mut error, 0, format, &mut int64_out),
            b'b' => unpack(root, &mut error, 0, format, &mut bool_out),
            b'f' | b'F' => unpack(root, &mut error, 0, format, &mut real_out),
            b'n' => unpack(root, &mut error, 0, format),
            _ => unreachable!(),
        };
        output.push((result, error.code()));
    }
    unpack_case!(string, 0, c"s".as_ptr(), ptr::null_mut::<*const c_char>());
    unpack_case!(
        string,
        0,
        c"s%".as_ptr(),
        &mut string_out,
        ptr::null_mut::<usize>()
    );
    unpack_case!(integer, 0, c"q".as_ptr());
    let mut trailing_out = 0;
    unpack_case!(
        integer,
        0,
        c"i i".as_ptr(),
        &mut integer_out,
        &mut trailing_out
    );

    for value in [null, boolean, real, string, integer, array, object] {
        api.delete(value);
    }
    output
}

#[test]
fn errors_109_through_134_variadic_rejections() {
    unsafe {
        let libs = Libraries::open();
        let c = variadic_error_transcript(&libs.c);
        let rust = variadic_error_transcript(&libs.rust);
        assert_eq!(c, rust);
        assert!(
            c.iter()
                .all(|&(result, code)| result == -1 || result == 1 && code != 0)
        );
    }
}

#[repr(C)]
struct CallbackInput {
    bytes: Vec<u8>,
    offset: usize,
    chunk: usize,
}

unsafe extern "C" fn load_callback(buffer: *mut c_void, length: usize, data: *mut c_void) -> usize {
    let input = unsafe { &mut *data.cast::<CallbackInput>() };
    let count = input
        .chunk
        .min(length)
        .min(input.bytes.len().saturating_sub(input.offset));
    if count != 0 {
        unsafe {
            ptr::copy_nonoverlapping(input.bytes.as_ptr().add(input.offset), buffer.cast(), count)
        };
        input.offset += count;
    }
    count
}

unsafe extern "C" fn dump_callback(
    buffer: *const c_char,
    length: usize,
    data: *mut c_void,
) -> c_int {
    let output = unsafe { &mut *data.cast::<Vec<u8>>() };
    output.extend_from_slice(unsafe { std::slice::from_raw_parts(buffer.cast::<u8>(), length) });
    0
}

unsafe extern "C" fn rejecting_dump_callback(
    _buffer: *const c_char,
    _length: usize,
    _data: *mut c_void,
) -> c_int {
    -1
}

unsafe extern "C" fn failing_load_callback(
    _buffer: *mut c_void,
    _length: usize,
    _data: *mut c_void,
) -> usize {
    usize::MAX
}

unsafe fn transport_transcript(api: &Api, base: &Path) -> Vec<Vec<u8>> {
    type LoadFile = unsafe extern "C" fn(*const c_char, usize, *mut JsonError) -> *mut Json;
    type LoadFd = unsafe extern "C" fn(c_int, usize, *mut JsonError) -> *mut Json;
    type LoadF = unsafe extern "C" fn(*mut libc::FILE, usize, *mut JsonError) -> *mut Json;
    type LoadCallback = unsafe extern "C" fn(
        Option<unsafe extern "C" fn(*mut c_void, usize, *mut c_void) -> usize>,
        *mut c_void,
        usize,
        *mut JsonError,
    ) -> *mut Json;
    type DumpFile = unsafe extern "C" fn(*const Json, *const c_char, usize) -> c_int;
    type DumpFd = unsafe extern "C" fn(*const Json, c_int, usize) -> c_int;
    type DumpF = unsafe extern "C" fn(*const Json, *mut libc::FILE, usize) -> c_int;
    type DumpCallback = unsafe extern "C" fn(
        *const Json,
        Option<unsafe extern "C" fn(*const c_char, usize, *mut c_void) -> c_int>,
        *mut c_void,
        usize,
    ) -> c_int;

    let load_file: LoadFile = unsafe { api.sym(b"json_load_file") };
    let load_fd: LoadFd = unsafe { api.sym(b"json_loadfd") };
    let load_f: LoadF = unsafe { api.sym(b"json_loadf") };
    let load_cb: LoadCallback = unsafe { api.sym(b"json_load_callback") };
    let dump_file: DumpFile = unsafe { api.sym(b"json_dump_file") };
    let dump_fd: DumpFd = unsafe { api.sym(b"json_dumpfd") };
    let dump_f: DumpF = unsafe { api.sym(b"json_dumpf") };
    let dump_cb: DumpCallback = unsafe { api.sym(b"json_dump_callback") };

    let source = b"{\"z\":1,\"a\":[true,\"\xc3\xa9\",-2.5]}";
    fs::write(base, source).unwrap();
    let path = CString::new(base.as_os_str().as_encoded_bytes()).unwrap();
    let mut transcript = Vec::new();
    let mut error = JsonError::default();
    let from_file = unsafe { load_file(path.as_ptr(), 0, &mut error) };
    transcript.push(unsafe { api.dump(from_file, JSON_COMPACT | JSON_SORT_KEYS).unwrap() });

    let file = fs::File::open(base).unwrap();
    let from_fd = unsafe { load_fd(std::os::fd::AsRawFd::as_raw_fd(&file), 0, &mut error) };
    transcript.push(unsafe { api.dump(from_fd, JSON_COMPACT | JSON_SORT_KEYS).unwrap() });

    let mode = c"rb";
    let c_file = unsafe { libc::fopen(path.as_ptr(), mode.as_ptr()) };
    let from_stream = unsafe { load_f(c_file, 0, &mut error) };
    unsafe { libc::fclose(c_file) };
    transcript.push(unsafe {
        api.dump(from_stream, JSON_COMPACT | JSON_SORT_KEYS)
            .unwrap()
    });

    for chunk in [1, 2, 7, 1024] {
        let mut input = CallbackInput {
            bytes: source.to_vec(),
            offset: 0,
            chunk,
        };
        let value = unsafe {
            load_cb(
                Some(load_callback),
                (&mut input as *mut CallbackInput).cast(),
                0,
                &mut error,
            )
        };
        transcript.push(unsafe { api.dump(value, JSON_COMPACT | JSON_SORT_KEYS).unwrap() });
        unsafe { api.delete(value) };
    }

    let output_path = base.with_extension(format!("{}.json", api.name));
    let output_c = CString::new(output_path.as_os_str().as_encoded_bytes()).unwrap();
    transcript.push(
        unsafe { dump_file(from_file, output_c.as_ptr(), JSON_COMPACT) }
            .to_string()
            .into_bytes(),
    );
    transcript.push(fs::read(&output_path).unwrap());

    let (read_fd, write_fd) = {
        let mut fds = [0; 2];
        assert_eq!(unsafe { libc::pipe(fds.as_mut_ptr()) }, 0);
        (fds[0], fds[1])
    };
    transcript.push(
        unsafe { dump_fd(from_file, write_fd, JSON_COMPACT) }
            .to_string()
            .into_bytes(),
    );
    unsafe { libc::close(write_fd) };
    let mut fd_bytes = Vec::new();
    let mut buffer = [0u8; 256];
    loop {
        let count = unsafe { libc::read(read_fd, buffer.as_mut_ptr().cast(), buffer.len()) };
        if count <= 0 {
            break;
        }
        fd_bytes.extend_from_slice(&buffer[..count as usize]);
    }
    unsafe { libc::close(read_fd) };
    transcript.push(fd_bytes);

    let stream_path = base.with_extension(format!("{}.stream", api.name));
    let stream_c = CString::new(stream_path.as_os_str().as_encoded_bytes()).unwrap();
    let stream = unsafe { libc::fopen(stream_c.as_ptr(), c"wb".as_ptr()) };
    transcript.push(
        unsafe { dump_f(from_file, stream, JSON_COMPACT) }
            .to_string()
            .into_bytes(),
    );
    unsafe { libc::fclose(stream) };
    transcript.push(fs::read(stream_path).unwrap());

    let mut callback_bytes = Vec::new();
    transcript.push(
        unsafe {
            dump_cb(
                from_file,
                Some(dump_callback),
                (&mut callback_bytes as *mut Vec<u8>).cast(),
                JSON_COMPACT,
            )
        }
        .to_string()
        .into_bytes(),
    );
    transcript.push(callback_bytes);
    unsafe {
        api.delete(from_stream);
        api.delete(from_fd);
        api.delete(from_file);
    }
    transcript
}

#[test]
fn configs_17_and_27_all_io_transports() {
    unsafe {
        let libs = Libraries::open();
        let base = std::env::temp_dir().join(format!(
            "jansson-differential-{}-{}.json",
            std::process::id(),
            17
        ));
        assert_eq!(
            transport_transcript(&libs.c, &base),
            transport_transcript(&libs.rust, &base)
        );
        let _ = fs::remove_file(base);
    }
}

unsafe fn private_transcript(api: &Api) -> Vec<Vec<u8>> {
    type UtfFirst = unsafe extern "C" fn(c_char) -> usize;
    type UtfFull = unsafe extern "C" fn(*const c_char, usize, *mut i32) -> usize;
    type UtfCheck = unsafe extern "C" fn(*const c_char, usize) -> c_int;
    type UtfIter = unsafe extern "C" fn(*const c_char, usize, *mut i32) -> *const c_char;
    type UtfEncode = unsafe extern "C" fn(i32, *mut c_char, *mut usize) -> c_int;
    let first: UtfFirst = unsafe { api.sym(b"utf8_check_first") };
    let full: UtfFull = unsafe { api.sym(b"utf8_check_full") };
    let check: UtfCheck = unsafe { api.sym(b"utf8_check_string") };
    let iterate: UtfIter = unsafe { api.sym(b"utf8_iterate") };
    let encode: UtfEncode = unsafe { api.sym(b"utf8_encode") };
    let mut output = Vec::new();
    let mut first_bytes = Vec::new();
    for byte in 0..=255 {
        first_bytes.push(unsafe { first(byte as u8 as c_char) } as u8);
    }
    output.push(first_bytes);
    for bytes in [
        b"A".as_slice(),
        &[0xc2, 0x80],
        &[0xdf, 0xbf],
        &[0xe0, 0xa0, 0x80],
        &[0xef, 0xbf, 0xbf],
        &[0xf0, 0x90, 0x80, 0x80],
        &[0xf4, 0x8f, 0xbf, 0xbf],
        &[0xc0, 0x80],
        &[0xed, 0xa0, 0x80],
        &[0xf5, 0x80, 0x80, 0x80],
        &[0xe2, 0x82],
    ] {
        let mut codepoint = -1;
        let full_result = unsafe { full(bytes.as_ptr().cast(), bytes.len(), &mut codepoint) };
        let check_result = unsafe { check(bytes.as_ptr().cast(), bytes.len()) };
        let next = unsafe { iterate(bytes.as_ptr().cast(), bytes.len(), &mut codepoint) };
        let offset = if next.is_null() {
            -1
        } else {
            unsafe { next.offset_from(bytes.as_ptr().cast()) as isize }
        };
        output.push(format!("{full_result},{check_result},{codepoint},{offset}").into_bytes());
    }
    for codepoint in [
        -1, 0, 0x7f, 0x80, 0x7ff, 0x800, 0xd7ff, 0xd800, 0xdfff, 0xe000, 0xffff, 0x10000, 0x10ffff,
        0x110000,
    ] {
        let mut buffer = [0i8; 8];
        let mut length = 99;
        let result = unsafe { encode(codepoint, buffer.as_mut_ptr(), &mut length) };
        output.push(
            format!("{result},{length},")
                .bytes()
                .chain(buffer[..length.min(8)].iter().map(|byte| *byte as u8))
                .collect(),
        );
    }

    type SbInit = unsafe extern "C" fn(*mut StrBuffer) -> c_int;
    type SbAppend = unsafe extern "C" fn(*mut StrBuffer, *const c_char, usize) -> c_int;
    type SbByte = unsafe extern "C" fn(*mut StrBuffer, c_char) -> c_int;
    type SbPop = unsafe extern "C" fn(*mut StrBuffer) -> c_char;
    type SbClear = unsafe extern "C" fn(*mut StrBuffer);
    type SbSteal = unsafe extern "C" fn(*mut StrBuffer) -> *mut c_char;
    type SbClose = unsafe extern "C" fn(*mut StrBuffer);
    let sb_init: SbInit = unsafe { api.sym(b"strbuffer_init") };
    let sb_append: SbAppend = unsafe { api.sym(b"strbuffer_append_bytes") };
    let sb_byte: SbByte = unsafe { api.sym(b"strbuffer_append_byte") };
    let sb_pop: SbPop = unsafe { api.sym(b"strbuffer_pop") };
    let sb_clear: SbClear = unsafe { api.sym(b"strbuffer_clear") };
    let sb_steal: SbSteal = unsafe { api.sym(b"strbuffer_steal_value") };
    let sb_close: SbClose = unsafe { api.sym(b"strbuffer_close") };
    let free: unsafe extern "C" fn(*mut c_void) = unsafe { api.sym(b"jsonp_free") };
    let mut sb = MaybeUninit::<StrBuffer>::uninit();
    output.push(unsafe { sb_init(sb.as_mut_ptr()) }.to_string().into_bytes());
    let mut sb = unsafe { sb.assume_init() };
    for size in [0, 1, 15, 16, 17, 63, 129] {
        let bytes: Vec<_> = (0..size).map(|index| b'a' + (index % 26) as u8).collect();
        output.push(
            unsafe { sb_append(&mut sb, bytes.as_ptr().cast(), bytes.len()) }
                .to_string()
                .into_bytes(),
        );
        unsafe { sb_byte(&mut sb, b'!' as c_char) };
        let popped = unsafe { sb_pop(&mut sb) };
        output.push(format!("{},{},{popped}", sb.length, sb.size).into_bytes());
    }
    unsafe { sb_clear(&mut sb) };
    let popped = unsafe { sb_pop(&mut sb) };
    output.push(format!("{},{popped}", sb.length).into_bytes());
    let stolen = unsafe { sb_steal(&mut sb) };
    output.push((!stolen.is_null()).to_string().into_bytes());
    unsafe {
        free(stolen.cast());
        sb_close(&mut sb);
    }

    type HInit = unsafe extern "C" fn(*mut HashTable) -> c_int;
    type HSet = unsafe extern "C" fn(*mut HashTable, *const c_char, usize, *mut Json) -> c_int;
    type HGet = unsafe extern "C" fn(*mut HashTable, *const c_char, usize) -> *mut Json;
    type HDel = unsafe extern "C" fn(*mut HashTable, *const c_char, usize) -> c_int;
    type HIter = unsafe extern "C" fn(*mut HashTable) -> *mut c_void;
    type HNext = unsafe extern "C" fn(*mut HashTable, *mut c_void) -> *mut c_void;
    type HKey = unsafe extern "C" fn(*mut c_void) -> *const c_char;
    type HKeyLen = unsafe extern "C" fn(*mut c_void) -> usize;
    type HValue = unsafe extern "C" fn(*mut c_void) -> *mut Json;
    type HIterSet = unsafe extern "C" fn(*mut c_void, *mut Json);
    type HClose = unsafe extern "C" fn(*mut HashTable);
    let h_init: HInit = unsafe { api.sym(b"hashtable_init") };
    let h_set: HSet = unsafe { api.sym(b"hashtable_set") };
    let h_get: HGet = unsafe { api.sym(b"hashtable_get") };
    let h_del: HDel = unsafe { api.sym(b"hashtable_del") };
    let h_iter: HIter = unsafe { api.sym(b"hashtable_iter") };
    let h_next: HNext = unsafe { api.sym(b"hashtable_iter_next") };
    let h_key: HKey = unsafe { api.sym(b"hashtable_iter_key") };
    let h_key_len: HKeyLen = unsafe { api.sym(b"hashtable_iter_key_len") };
    let h_value: HValue = unsafe { api.sym(b"hashtable_iter_value") };
    let h_iter_set: HIterSet = unsafe { api.sym(b"hashtable_iter_set") };
    let h_close: HClose = unsafe { api.sym(b"hashtable_close") };
    let integer: unsafe extern "C" fn(i64) -> *mut Json = unsafe { api.sym(b"json_integer") };
    let integer_value: unsafe extern "C" fn(*const Json) -> i64 =
        unsafe { api.sym(b"json_integer_value") };
    let mut table: Box<HashTable> = Box::new(unsafe { std::mem::zeroed() });
    output.push(unsafe { h_init(&mut *table) }.to_string().into_bytes());
    for index in 0..20 {
        let key = format!("k\0{index}");
        unsafe { h_set(&mut *table, key.as_ptr().cast(), key.len(), integer(index)) };
    }
    unsafe { h_set(&mut *table, b"k\00".as_ptr().cast(), 3, integer(999)) };
    let table_size = table.size;
    let replaced = unsafe { integer_value(h_get(&mut *table, b"k\00".as_ptr().cast(), 3)) };
    output.push(format!("{table_size},{replaced}").into_bytes());
    let mut iter = unsafe { h_iter(&mut *table) };
    while !iter.is_null() {
        let key_len = unsafe { h_key_len(iter) };
        assert!(
            key_len < 1024,
            "{} invalid hashtable key length {key_len}",
            api.name
        );
        let key = unsafe { std::slice::from_raw_parts(h_key(iter).cast(), key_len) };
        output.push(
            key.iter()
                .copied()
                .chain(format!("={}", unsafe { integer_value(h_value(iter)) }).bytes())
                .collect(),
        );
        iter = unsafe { h_next(&mut *table, iter) };
    }
    iter = unsafe { h_iter(&mut *table) };
    unsafe { h_iter_set(iter, integer(-7)) };
    output.push(
        unsafe { integer_value(h_value(iter)) }
            .to_string()
            .into_bytes(),
    );
    output.push(
        unsafe { h_del(&mut *table, c"missing".as_ptr(), 7) }
            .to_string()
            .into_bytes(),
    );
    unsafe { h_close(&mut *table) };

    type LoopCheck =
        unsafe extern "C" fn(*mut HashTable, *const Json, *mut c_char, usize, *mut usize) -> c_int;
    let loop_check: LoopCheck = unsafe { api.sym(b"jsonp_loop_check") };
    let null: unsafe extern "C" fn() -> *mut Json = unsafe { api.sym(b"json_null") };
    let mut loop_table: HashTable = unsafe { std::mem::zeroed() };
    output.push(unsafe { h_init(&mut loop_table) }.to_string().into_bytes());
    let mut loop_key = [0i8; 75];
    let mut loop_key_len = 0;
    output.push(
        unsafe {
            loop_check(
                &mut loop_table,
                null(),
                loop_key.as_mut_ptr(),
                loop_key.len(),
                &mut loop_key_len,
            )
        }
        .to_string()
        .into_bytes(),
    );
    output.push(
        unsafe {
            loop_check(
                &mut loop_table,
                null(),
                loop_key.as_mut_ptr(),
                loop_key.len(),
                ptr::null_mut(),
            )
        }
        .to_string()
        .into_bytes(),
    );
    unsafe { h_close(&mut loop_table) };
    output
}

#[test]
fn configs_46_through_56_private_structures_and_utf8() {
    unsafe {
        let libs = Libraries::open();
        let c = private_transcript(&libs.c);
        let rust = private_transcript(&libs.rust);
        assert_eq!(c.len(), rust.len());
        for (index, (c, rust)) in c.iter().zip(&rust).enumerate() {
            assert_eq!(
                c,
                rust,
                "private mismatch at {index}: C={:?}, Rust={:?}",
                String::from_utf8_lossy(c),
                String::from_utf8_lossy(rust)
            );
        }
    }
}

unsafe fn float_transcript(api: &Api) -> Vec<Vec<u8>> {
    type DtoaR = unsafe extern "C" fn(
        f64,
        c_int,
        c_int,
        *mut c_int,
        *mut c_int,
        *mut *mut c_char,
        *mut c_char,
        usize,
    ) -> *mut c_char;
    type DtoStr = unsafe extern "C" fn(*mut c_char, usize, f64, c_int) -> c_int;
    type StrToD = unsafe extern "C" fn(*mut StrBuffer, *mut f64) -> c_int;
    let dtoa_r: DtoaR = unsafe { api.sym(b"dtoa_r") };
    let dtostr: DtoStr = unsafe { api.sym(b"jsonp_dtostr") };
    let strtod: StrToD = unsafe { api.sym(b"jsonp_strtod") };
    let mut output = Vec::new();
    let mut values = vec![
        0.0,
        -0.0,
        0.1,
        1e-7,
        1e16,
        1.2345678901234567,
        f64::MIN_POSITIVE,
        f64::from_bits(1),
        f64::MAX,
        9_007_199_254_740_991.0,
    ];
    let mut state = 0xd70a_2150_0000_0001;
    while values.len() < 300 {
        let value = f64::from_bits(xorshift(&mut state));
        if value.is_finite() {
            values.push(value);
        }
    }
    for (value_index, value) in values.into_iter().enumerate() {
        for precision in [0, 1, 2, 6, 10, 17, 31] {
            let mut buffer = [0i8; 128];
            let length = unsafe { dtostr(buffer.as_mut_ptr(), buffer.len(), value, precision) };
            let bytes = if length >= 0 {
                unsafe { CStr::from_ptr(buffer.as_ptr()) }
                    .to_bytes()
                    .to_vec()
            } else {
                Vec::new()
            };
            output.push(
                format!("{:016x},{precision},{length},", value.to_bits())
                    .bytes()
                    .chain(bytes)
                    .collect(),
            );
        }
        let mode_end = if value_index < 10 { 9 } else { 3 };
        for mode in 0..=mode_end {
            let digit_cases: &[c_int] = if value_index >= 10 && mode == 3 {
                &[0, 1, 6, 17]
            } else {
                &[-2, 0, 1, 6, 17]
            };
            for &digits in digit_cases {
                let mut buffer = [0i8; 1024];
                let mut decpt = 0;
                let mut sign = 0;
                let mut end = ptr::null_mut();
                let result = unsafe {
                    dtoa_r(
                        value,
                        mode,
                        digits,
                        &mut decpt,
                        &mut sign,
                        &mut end,
                        buffer.as_mut_ptr(),
                        buffer.len(),
                    )
                };
                let bytes = if result.is_null() {
                    Vec::new()
                } else {
                    unsafe { CStr::from_ptr(result) }.to_bytes().to_vec()
                };
                output.push(
                    format!(
                        "{:016x},{mode},{digits},{decpt},{sign},{},",
                        value.to_bits(),
                        if result.is_null() {
                            -1
                        } else {
                            unsafe { end.offset_from(result) }
                        }
                    )
                    .bytes()
                    .chain(bytes)
                    .collect(),
                );
            }
        }
    }
    for text in ["0", "-0", "1.25e3", "1e-300", "1.7976931348623157e308"] {
        let mut bytes = CString::new(text).unwrap().into_bytes_with_nul();
        let mut buffer = StrBuffer {
            value: bytes.as_mut_ptr().cast(),
            length: bytes.len() - 1,
            size: bytes.len(),
        };
        let mut value = 0.0;
        let result = unsafe { strtod(&mut buffer, &mut value) };
        output.push(format!("{text},{result},{:016x}", value.to_bits()).into_bytes());
    }
    output
}

#[test]
fn configs_57_through_60_randomized_numeric_conversions() {
    unsafe {
        let libs = Libraries::open();
        let c = float_transcript(&libs.c);
        let rust = float_transcript(&libs.rust);
        assert_eq!(c.len(), rust.len());
        for (index, (c, rust)) in c.iter().zip(&rust).enumerate() {
            assert_eq!(
                c,
                rust,
                "numeric mismatch at {index}: C={}, Rust={}",
                String::from_utf8_lossy(c),
                String::from_utf8_lossy(rust)
            );
        }
    }
}

#[test]
fn config_1_version_surface() {
    unsafe {
        let libs = Libraries::open();
        type Version = unsafe extern "C" fn() -> *const c_char;
        type Compare = unsafe extern "C" fn(c_int, c_int, c_int) -> c_int;
        let c_version: Version = libs.c.sym(b"jansson_version_str");
        let r_version: Version = libs.rust.sym(b"jansson_version_str");
        assert_eq!(
            CStr::from_ptr(c_version()).to_bytes(),
            CStr::from_ptr(r_version()).to_bytes()
        );
        let c_compare: Compare = libs.c.sym(b"jansson_version_cmp");
        let r_compare: Compare = libs.rust.sym(b"jansson_version_cmp");
        for major in 0..=4 {
            for minor in [0, 14, 15, 16, 99] {
                for micro in [0, 1, 99] {
                    assert_eq!(
                        c_compare(major, minor, micro),
                        r_compare(major, minor, micro)
                    );
                }
            }
        }
    }
}

#[test]
fn errors_78_through_102_parser_rejections() {
    unsafe {
        let libs = Libraries::open();
        let mut cases: Vec<(Vec<u8>, usize)> = vec![
            (b"".to_vec(), 0),
            (b"\"".to_vec(), JSON_DECODE_ANY),
            (b"\"abc\\".to_vec(), JSON_DECODE_ANY),
            (b"\"\n\"".to_vec(), JSON_DECODE_ANY),
            (b"\"\\u12xz\"".to_vec(), JSON_DECODE_ANY),
            (b"\"\\q\"".to_vec(), JSON_DECODE_ANY),
            (b"\"\\uD800\"".to_vec(), JSON_DECODE_ANY),
            (b"\"\\uD800\\u0041\"".to_vec(), JSON_DECODE_ANY),
            (b"\"\\uDC00\"".to_vec(), JSON_DECODE_ANY),
            (vec![b'"', 0xc0, 0x80, b'"'], JSON_DECODE_ANY),
            (vec![b'"', 0xe2, 0x82, b'"'], JSON_DECODE_ANY),
            (b"01".to_vec(), JSON_DECODE_ANY),
            (b"-".to_vec(), JSON_DECODE_ANY),
            (b"1.".to_vec(), JSON_DECODE_ANY),
            (b"1e".to_vec(), JSON_DECODE_ANY),
            (b"1e+".to_vec(), JSON_DECODE_ANY),
            (b"9223372036854775808".to_vec(), JSON_DECODE_ANY),
            (b"-9223372036854775809".to_vec(), JSON_DECODE_ANY),
            (b"1e9999".to_vec(), JSON_DECODE_ANY),
            (b"{x:1}".to_vec(), 0),
            (b"{\"a\" 1}".to_vec(), 0),
            (b"{\"a\":1".to_vec(), 0),
            (b"{\"a\":}".to_vec(), 0),
            (b"{\"a\":1,}".to_vec(), 0),
            (b"{\"\\u0000\":1}".to_vec(), JSON_ALLOW_NUL),
            (b"{\"a\":1,\"a\":2}".to_vec(), JSON_REJECT_DUPLICATES),
            (b"[1".to_vec(), 0),
            (b"[1,]".to_vec(), 0),
            (b"[1 2]".to_vec(), 0),
            (b"true".to_vec(), 0),
            (b"{}".to_vec(), JSON_DECODE_ANY | JSON_ALLOW_NUL),
            (b"{} trailing".to_vec(), 0),
            (b"\"\\u0000\"".to_vec(), JSON_DECODE_ANY),
            (b"?".to_vec(), JSON_DECODE_ANY),
            (vec![0], JSON_DECODE_ANY),
        ];
        let mut deep = vec![b'['; 2050];
        deep.extend(std::iter::repeat_n(b']', 2050));
        cases.push((deep, 0));
        for (bytes, flags) in cases {
            compare_load(&libs, &bytes, flags, &[JSON_COMPACT | JSON_ENCODE_ANY]);
        }

        type LoadS = unsafe extern "C" fn(*const c_char, usize, *mut JsonError) -> *mut Json;
        type LoadB = unsafe extern "C" fn(*const c_char, usize, usize, *mut JsonError) -> *mut Json;
        type LoadF = unsafe extern "C" fn(*mut libc::FILE, usize, *mut JsonError) -> *mut Json;
        type LoadFd = unsafe extern "C" fn(c_int, usize, *mut JsonError) -> *mut Json;
        type LoadFile = unsafe extern "C" fn(*const c_char, usize, *mut JsonError) -> *mut Json;
        for api in [&libs.c, &libs.rust] {
            let mut error = JsonError::default();
            assert!((api.sym::<LoadS>(b"json_loads"))(ptr::null(), 0, &mut error).is_null());
            assert_eq!(error.code(), 4);
            error = JsonError::default();
            assert!((api.sym::<LoadB>(b"json_loadb"))(ptr::null(), 0, 0, &mut error).is_null());
            assert_eq!(error.code(), 4);
            error = JsonError::default();
            assert!((api.sym::<LoadF>(b"json_loadf"))(ptr::null_mut(), 0, &mut error).is_null());
            assert_eq!(error.code(), 4);
            error = JsonError::default();
            assert!((api.sym::<LoadFd>(b"json_loadfd"))(-1, 0, &mut error).is_null());
            assert_eq!(error.code(), 4);
            error = JsonError::default();
            assert!(
                (api.sym::<LoadFile>(b"json_load_file"))(
                    c"/definitely/not/a/jansson/file".as_ptr(),
                    0,
                    &mut error
                )
                .is_null()
            );
            assert_eq!(error.code(), 3);
        }
    }
}

unsafe fn allocation_error_transcript(api: &Api) -> Vec<i128> {
    type MallocFn = Option<unsafe extern "C" fn(usize) -> *mut c_void>;
    type ReallocFn = Option<unsafe extern "C" fn(*mut c_void, usize) -> *mut c_void>;
    type FreeFn = Option<unsafe extern "C" fn(*mut c_void)>;
    type SetAlloc = unsafe extern "C" fn(MallocFn, FreeFn);
    type SetAlloc2 = unsafe extern "C" fn(MallocFn, ReallocFn, FreeFn);
    type Malloc = unsafe extern "C" fn(usize) -> *mut c_void;
    type Realloc = unsafe extern "C" fn(*mut c_void, usize, usize) -> *mut c_void;
    type StrDup = unsafe extern "C" fn(*const c_char, usize) -> *mut c_char;
    type SbInit = unsafe extern "C" fn(*mut StrBuffer) -> c_int;
    type SbAppend = unsafe extern "C" fn(*mut StrBuffer, *const c_char, usize) -> c_int;
    type SbClose = unsafe extern "C" fn(*mut StrBuffer);
    type Constructor = unsafe extern "C" fn() -> *mut Json;
    type Integer = unsafe extern "C" fn(i64) -> *mut Json;
    type Real = unsafe extern "C" fn(f64) -> *mut Json;
    type StringFn = unsafe extern "C" fn(*const c_char) -> *mut Json;
    type StringSet = unsafe extern "C" fn(*mut Json, *const c_char) -> c_int;
    type ObjectSet = unsafe extern "C" fn(*mut Json, *const c_char, *mut Json) -> c_int;
    type ArrayAppend = unsafe extern "C" fn(*mut Json, *mut Json) -> c_int;
    type ArrayInsert = unsafe extern "C" fn(*mut Json, usize, *mut Json) -> c_int;
    type ArrayExtend = unsafe extern "C" fn(*mut Json, *mut Json) -> c_int;
    type DeepCopy = unsafe extern "C" fn(*const Json) -> *mut Json;
    type HInit = unsafe extern "C" fn(*mut HashTable) -> c_int;
    type HSet = unsafe extern "C" fn(*mut HashTable, *const c_char, usize, *mut Json) -> c_int;
    type HClose = unsafe extern "C" fn(*mut HashTable);
    type PackEx = unsafe extern "C" fn(*mut JsonError, usize, *const c_char, ...) -> *mut Json;
    type UnpackEx =
        unsafe extern "C" fn(*mut Json, *mut JsonError, usize, *const c_char, ...) -> c_int;
    type Sprintf = unsafe extern "C" fn(*const c_char, ...) -> *mut Json;

    let set_alloc: SetAlloc = api.sym(b"json_set_alloc_funcs");
    let set_alloc2: SetAlloc2 = api.sym(b"json_set_alloc_funcs2");
    let malloc: Malloc = api.sym(b"jsonp_malloc");
    let realloc: Realloc = api.sym(b"jsonp_realloc");
    let strdup: StrDup = api.sym(b"jsonp_strndup");
    let sb_init: SbInit = api.sym(b"strbuffer_init");
    let sb_append: SbAppend = api.sym(b"strbuffer_append_bytes");
    let sb_close: SbClose = api.sym(b"strbuffer_close");
    let object: Constructor = api.sym(b"json_object");
    let array: Constructor = api.sym(b"json_array");
    let integer: Integer = api.sym(b"json_integer");
    let real: Real = api.sym(b"json_real");
    let string: StringFn = api.sym(b"json_string");
    let string_set: StringSet = api.sym(b"json_string_set");
    let object_set: ObjectSet = api.sym(b"json_object_set_new");
    let array_append: ArrayAppend = api.sym(b"json_array_append_new");
    let array_insert: ArrayInsert = api.sym(b"json_array_insert_new");
    let array_extend: ArrayExtend = api.sym(b"json_array_extend");
    let deep_copy: DeepCopy = api.sym(b"json_deep_copy");
    let h_init: HInit = api.sym(b"hashtable_init");
    let h_set: HSet = api.sym(b"hashtable_set");
    let h_close: HClose = api.sym(b"hashtable_close");
    let null: Constructor = api.sym(b"json_null");
    let pack: PackEx = api.sym(b"json_pack_ex");
    let unpack: UnpackEx = api.sym(b"json_unpack_ex");
    let sprintf: Sprintf = api.sym(b"json_sprintf");

    let mut out = Vec::new();
    FAIL_ALLOCATIONS.store(false, Ordering::SeqCst);
    set_alloc2(Some(test_malloc), Some(test_realloc), Some(test_free));
    let unpack_root = object();

    set_alloc(Some(test_malloc), Some(test_free));
    let allocation = malloc(32);
    FREE_CALLS.store(0, Ordering::SeqCst);
    out.push(realloc(allocation, 32, 0).is_null() as i128);
    out.push(FREE_CALLS.load(Ordering::SeqCst) as i128);
    set_alloc2(Some(test_malloc), Some(test_realloc), Some(test_free));

    let mut byte = 0i8;
    let mut overflow_a = StrBuffer {
        value: &mut byte,
        length: 0,
        size: 1,
    };
    out.push(sb_append(&mut overflow_a, &byte, usize::MAX) as i128);
    let mut overflow_b = StrBuffer {
        value: &mut byte,
        length: 1,
        size: 1,
    };
    out.push(sb_append(&mut overflow_b, &byte, usize::MAX - 1) as i128);

    FAIL_ALLOCATIONS.store(true, Ordering::SeqCst);
    out.push(strdup(c"x".as_ptr(), 1).is_null() as i128);
    let mut failed_buffer = MaybeUninit::<StrBuffer>::uninit();
    out.push(sb_init(failed_buffer.as_mut_ptr()) as i128);
    out.push(object().is_null() as i128);
    out.push(array().is_null() as i128);
    out.push(string(c"x".as_ptr()).is_null() as i128);
    out.push(integer(1).is_null() as i128);
    out.push(real(1.0).is_null() as i128);
    let mut failed_table = MaybeUninit::<HashTable>::zeroed();
    out.push(h_init(failed_table.as_mut_ptr()) as i128);
    let (loaded, load_error) = api.loadb(b"{}", 0);
    out.push(loaded.is_null() as i128);
    out.push(load_error.code() as i128);
    let mut pack_error = JsonError::default();
    let packed = pack(&mut pack_error, 0, c"i".as_ptr(), 1 as c_int);
    out.push(packed.is_null() as i128);
    out.push(pack_error.code() as i128);
    pack_error = JsonError::default();
    let packed = pack(&mut pack_error, 0, c"f".as_ptr(), 1.0_f64);
    out.push(packed.is_null() as i128);
    out.push(pack_error.code() as i128);
    pack_error = JsonError::default();
    let packed = pack(
        &mut pack_error,
        0,
        c"s#".as_ptr(),
        c"x".as_ptr(),
        1 as c_int,
    );
    out.push(packed.is_null() as i128);
    out.push(pack_error.code() as i128);
    let mut unpack_error = JsonError::default();
    out.push(unpack(unpack_root, &mut unpack_error, 0, c"{}".as_ptr()) as i128);
    out.push(unpack_error.code() as i128);
    out.push(sprintf(c"%s".as_ptr(), c"x".as_ptr()).is_null() as i128);

    FAIL_ALLOCATIONS.store(false, Ordering::SeqCst);
    pack_error = JsonError::default();
    ALLOCATION_CALLS.store(0, Ordering::SeqCst);
    FAIL_AT_ALLOCATION.store(4, Ordering::SeqCst);
    let packed = pack(
        &mut pack_error,
        0,
        c"{s:i}".as_ptr(),
        c"k".as_ptr(),
        1 as c_int,
    );
    FAIL_AT_ALLOCATION.store(0, Ordering::SeqCst);
    out.push(packed.is_null() as i128);
    out.push(pack_error.code() as i128);

    pack_error = JsonError::default();
    ALLOCATION_CALLS.store(0, Ordering::SeqCst);
    FAIL_AT_ALLOCATION.store(12, Ordering::SeqCst);
    let packed = pack(
        &mut pack_error,
        0,
        c"[iiiiiiiii]".as_ptr(),
        1 as c_int,
        2 as c_int,
        3 as c_int,
        4 as c_int,
        5 as c_int,
        6 as c_int,
        7 as c_int,
        8 as c_int,
        9 as c_int,
    );
    FAIL_AT_ALLOCATION.store(0, Ordering::SeqCst);
    out.push(packed.is_null() as i128);
    out.push(pack_error.code() as i128);

    let mut growing_buffer = MaybeUninit::<StrBuffer>::uninit();
    out.push(sb_init(growing_buffer.as_mut_ptr()) as i128);
    let mut growing_buffer = growing_buffer.assume_init();
    FAIL_ALLOCATIONS.store(true, Ordering::SeqCst);
    out.push(sb_append(&mut growing_buffer, b"0123456789abcdef".as_ptr().cast(), 16) as i128);
    FAIL_ALLOCATIONS.store(false, Ordering::SeqCst);
    sb_close(&mut growing_buffer);

    let object_value = object();
    let rejected_value = integer(7);
    FAIL_ALLOCATIONS.store(true, Ordering::SeqCst);
    out.push(object_set(object_value, c"k".as_ptr(), rejected_value) as i128);
    out.push(api.dump(object_value, JSON_COMPACT).is_none() as i128);
    out.push(deep_copy(object_value).is_null() as i128);
    FAIL_ALLOCATIONS.store(false, Ordering::SeqCst);
    api.delete(object_value);

    let append_array = array();
    for value in 0..8 {
        out.push(array_append(append_array, integer(value)) as i128);
    }
    let rejected_value = integer(8);
    FAIL_ALLOCATIONS.store(true, Ordering::SeqCst);
    out.push(array_append(append_array, rejected_value) as i128);
    FAIL_ALLOCATIONS.store(false, Ordering::SeqCst);
    api.delete(append_array);

    let insert_array = array();
    for value in 0..8 {
        out.push(array_append(insert_array, integer(value)) as i128);
    }
    let rejected_value = integer(8);
    FAIL_ALLOCATIONS.store(true, Ordering::SeqCst);
    out.push(array_insert(insert_array, 0, rejected_value) as i128);
    FAIL_ALLOCATIONS.store(false, Ordering::SeqCst);
    api.delete(insert_array);

    let extend_array = array();
    for value in 0..8 {
        out.push(array_append(extend_array, integer(value)) as i128);
    }
    let extension = array();
    out.push(array_append(extension, integer(9)) as i128);
    FAIL_ALLOCATIONS.store(true, Ordering::SeqCst);
    out.push(array_extend(extend_array, extension) as i128);
    FAIL_ALLOCATIONS.store(false, Ordering::SeqCst);
    api.delete(extension);
    api.delete(extend_array);

    let string_value = string(c"old".as_ptr());
    FAIL_ALLOCATIONS.store(true, Ordering::SeqCst);
    out.push(string_set(string_value, c"replacement".as_ptr()) as i128);
    FAIL_ALLOCATIONS.store(false, Ordering::SeqCst);
    api.delete(string_value);

    let mut table: HashTable = std::mem::zeroed();
    out.push(h_init(&mut table) as i128);
    FAIL_ALLOCATIONS.store(true, Ordering::SeqCst);
    out.push(h_set(&mut table, c"k".as_ptr(), 1, integer(0)) as i128);
    FAIL_ALLOCATIONS.store(false, Ordering::SeqCst);
    h_close(&mut table);

    let mut rehash_table: HashTable = std::mem::zeroed();
    out.push(h_init(&mut rehash_table) as i128);
    for index in 0..8 {
        let key = [b'a' + index as u8];
        out.push(h_set(&mut rehash_table, key.as_ptr().cast(), 1, null()) as i128);
    }
    FAIL_ALLOCATIONS.store(true, Ordering::SeqCst);
    out.push(h_set(&mut rehash_table, c"x".as_ptr(), usize::MAX, null()) as i128);
    FAIL_ALLOCATIONS.store(false, Ordering::SeqCst);
    h_close(&mut rehash_table);
    api.delete(unpack_root);

    set_alloc2(Some(test_malloc), Some(test_realloc), Some(test_free));
    FAIL_ALLOCATIONS.store(false, Ordering::SeqCst);
    FAIL_AT_ALLOCATION.store(0, Ordering::SeqCst);
    out
}

#[test]
fn allocation_and_growth_error_paths() {
    unsafe {
        let libs = Libraries::open();
        assert_eq!(
            allocation_error_transcript(&libs.c),
            allocation_error_transcript(&libs.rust)
        );
    }
}

unsafe fn low_level_export_transcript(api: &Api) -> Vec<Vec<u8>> {
    type MallocFn = Option<unsafe extern "C" fn(usize) -> *mut c_void>;
    type ReallocFn = Option<unsafe extern "C" fn(*mut c_void, usize) -> *mut c_void>;
    type FreeFn = Option<unsafe extern "C" fn(*mut c_void)>;
    type SetAlloc = unsafe extern "C" fn(MallocFn, FreeFn);
    type SetAlloc2 = unsafe extern "C" fn(MallocFn, ReallocFn, FreeFn);
    type GetAlloc = unsafe extern "C" fn(*mut MallocFn, *mut FreeFn);
    type GetAlloc2 = unsafe extern "C" fn(*mut MallocFn, *mut ReallocFn, *mut FreeFn);
    type Malloc = unsafe extern "C" fn(usize) -> *mut c_void;
    type Realloc = unsafe extern "C" fn(*mut c_void, usize, usize) -> *mut c_void;
    type Free = unsafe extern "C" fn(*mut c_void);
    type OwnedString = unsafe extern "C" fn(*const c_char, usize) -> *mut Json;
    type ErrorInit = unsafe extern "C" fn(*mut JsonError, *const c_char);
    type ErrorSource = unsafe extern "C" fn(*mut JsonError, *const c_char);
    type ErrorSet =
        unsafe extern "C" fn(*mut JsonError, c_int, c_int, usize, c_int, *const c_char, ...);
    type ErrorVSet = unsafe extern "C" fn(
        *mut JsonError,
        c_int,
        c_int,
        usize,
        c_int,
        *const c_char,
        *mut TestVaList,
    );
    type SbInit = unsafe extern "C" fn(*mut StrBuffer) -> c_int;
    type SbAppend = unsafe extern "C" fn(*mut StrBuffer, *const c_char, usize) -> c_int;
    type SbValue = unsafe extern "C" fn(*const StrBuffer) -> *const c_char;
    type SbClose = unsafe extern "C" fn(*mut StrBuffer);
    type StrtodUnused = unsafe extern "C" fn(*const c_char, *mut *mut c_char) -> f64;
    type Dtoa = unsafe extern "C" fn(
        f64,
        c_int,
        c_int,
        *mut c_int,
        *mut c_int,
        *mut *mut c_char,
    ) -> *mut c_char;
    type FreeDtoa = unsafe extern "C" fn(*mut c_char);
    type GetHex = unsafe extern "C" fn(*mut *const c_char, *mut c_void, c_int, c_int);
    type Constructor = unsafe extern "C" fn() -> *mut Json;
    type Integer = unsafe extern "C" fn(i64) -> *mut Json;
    type DeepCopy = unsafe extern "C" fn(*const Json, *mut c_void) -> *mut Json;
    type RecursiveUpdate = unsafe extern "C" fn(*mut Json, *mut Json, *mut HashTable) -> c_int;
    type HInit = unsafe extern "C" fn(*mut HashTable) -> c_int;
    type HClose = unsafe extern "C" fn(*mut HashTable);
    type Seed = unsafe extern "C" fn(usize);

    let set_alloc: SetAlloc = api.sym(b"json_set_alloc_funcs");
    let set_alloc2: SetAlloc2 = api.sym(b"json_set_alloc_funcs2");
    let get_alloc: GetAlloc = api.sym(b"json_get_alloc_funcs");
    let get_alloc2: GetAlloc2 = api.sym(b"json_get_alloc_funcs2");
    let malloc: Malloc = api.sym(b"jsonp_malloc");
    let realloc: Realloc = api.sym(b"jsonp_realloc");
    let free: Free = api.sym(b"jsonp_free");
    let seed: Seed = api.sym(b"json_object_seed");
    let seed_value: *mut u32 = api.sym(b"hashtable_seed");
    let mut output = Vec::new();

    seed(0x1234_5678);
    output.push(format!("{:08x}", *seed_value).into_bytes());
    FAIL_ALLOCATIONS.store(false, Ordering::SeqCst);
    FAIL_AT_ALLOCATION.store(0, Ordering::SeqCst);
    set_alloc2(Some(test_malloc), Some(test_realloc), Some(test_free));
    let mut malloc_fn = None;
    let mut realloc_fn = None;
    let mut free_fn = None;
    get_alloc2(&mut malloc_fn, &mut realloc_fn, &mut free_fn);
    output.push(
        format!(
            "{},{},{}",
            malloc_fn.map(|f| f as usize) == Some(test_malloc as *const () as usize),
            realloc_fn.map(|f| f as usize) == Some(test_realloc as *const () as usize),
            free_fn.map(|f| f as usize) == Some(test_free as *const () as usize),
        )
        .into_bytes(),
    );
    malloc_fn = None;
    free_fn = None;
    get_alloc(&mut malloc_fn, &mut free_fn);
    output.push(
        format!(
            "{},{}",
            malloc_fn.map(|f| f as usize) == Some(test_malloc as *const () as usize),
            free_fn.map(|f| f as usize) == Some(test_free as *const () as usize),
        )
        .into_bytes(),
    );
    let allocation = malloc(8);
    let allocation = realloc(allocation, 8, 24);
    output.push((!allocation.is_null()).to_string().into_bytes());
    free(allocation);
    set_alloc(Some(test_malloc), Some(test_free));
    realloc_fn = Some(test_realloc);
    get_alloc2(ptr::null_mut(), &mut realloc_fn, ptr::null_mut());
    output.push(realloc_fn.is_none().to_string().into_bytes());
    set_alloc2(Some(test_malloc), Some(test_realloc), Some(test_free));

    let owned = malloc(4).cast::<u8>();
    ptr::copy_nonoverlapping(b"a\0b\0".as_ptr(), owned, 4);
    let owned_string: OwnedString = api.sym(b"jsonp_stringn_nocheck_own");
    let value = owned_string(owned.cast(), 3);
    output.push(api.dump(value, JSON_ENCODE_ANY).unwrap());
    api.delete(value);

    let error_init: ErrorInit = api.sym(b"jsonp_error_init");
    let error_source: ErrorSource = api.sym(b"jsonp_error_set_source");
    let error_set: ErrorSet = api.sym(b"jsonp_error_set");
    let error_vset: ErrorVSet = api.sym(b"jsonp_error_vset");
    let mut error = JsonError::default();
    error_init(&mut error, c"source".as_ptr());
    error_set(&mut error, 2, 3, 4, 9, c"value=%d".as_ptr(), 17 as c_int);
    output.push(
        format!(
            "{},{},{},{},{},{}",
            error.line,
            error.column,
            error.position,
            error.code(),
            CStr::from_ptr(error.source.as_ptr()).to_string_lossy(),
            CStr::from_ptr(error.text.as_ptr()).to_string_lossy(),
        )
        .into_bytes(),
    );
    let long_source = CString::new("x".repeat(100)).unwrap();
    error_source(&mut error, long_source.as_ptr());
    output.push(CStr::from_ptr(error.source.as_ptr()).to_bytes().to_vec());

    for code in 0..=17 {
        error_init(&mut error, ptr::null());
        error_set(&mut error, -1, -1, usize::MAX, code, c"code".as_ptr());
        output.push(vec![error.code()]);
    }
    error_set(ptr::null_mut(), 0, 0, 0, 0, c"ignored".as_ptr());

    let mut register_save = [0u64; 22];
    ptr::write_volatile(register_save.as_mut_ptr(), 23);
    let mut overflow = [0u64; 1];
    let mut va = TestVaList {
        gp_offset: 0,
        fp_offset: 48,
        overflow_arg_area: overflow.as_mut_ptr().cast(),
        reg_save_area: register_save.as_mut_ptr().cast(),
    };
    error_init(&mut error, c"vset".as_ptr());
    error_vset(&mut error, 5, 6, 7, 8, c"v=%d".as_ptr(), &mut va);
    output.push(CStr::from_ptr(error.text.as_ptr()).to_bytes().to_vec());

    let sb_init: SbInit = api.sym(b"strbuffer_init");
    let sb_append: SbAppend = api.sym(b"strbuffer_append_bytes");
    let sb_value: SbValue = api.sym(b"strbuffer_value");
    let sb_close: SbClose = api.sym(b"strbuffer_close");
    let mut buffer = MaybeUninit::<StrBuffer>::uninit();
    assert_eq!(sb_init(buffer.as_mut_ptr()), 0);
    let mut buffer = buffer.assume_init();
    assert_eq!(sb_append(&mut buffer, c"buffer".as_ptr(), 6), 0);
    output.push(CStr::from_ptr(sb_value(&buffer)).to_bytes().to_vec());
    sb_close(&mut buffer);

    let strtod_unused: StrtodUnused = api.sym(b"strtod__unused");
    let number = c"-1.25e2x";
    let mut number_end = ptr::null_mut();
    let parsed = strtod_unused(number.as_ptr(), &mut number_end);
    output.push(
        format!(
            "{:016x},{}",
            parsed.to_bits(),
            number_end.offset_from(number.as_ptr())
        )
        .into_bytes(),
    );

    let dtoa: Dtoa = api.sym(b"dtoa");
    let freedtoa: FreeDtoa = api.sym(b"freedtoa");
    let mut decpt = 0;
    let mut sign = 0;
    let mut end = ptr::null_mut();
    let digits = dtoa(-12.5, 0, 0, &mut decpt, &mut sign, &mut end);
    output.push(
        format!(
            "{},{},{}",
            CStr::from_ptr(digits).to_string_lossy(),
            decpt,
            sign
        )
        .into_bytes(),
    );
    freedtoa(digits);

    let gethex: GetHex = api.sym(b"gethex");
    let hex = c"0x1.8p+1!";
    let mut hex_input = hex.as_ptr();
    let mut hex_bits = 0_u64;
    gethex(&mut hex_input, (&mut hex_bits as *mut u64).cast(), 1, 0);
    output.push(format!("{hex_bits:016x},{}", hex_input.offset_from(hex.as_ptr())).into_bytes());

    let integer: Integer = api.sym(b"json_integer");
    let do_deep_copy: DeepCopy = api.sym(b"do_deep_copy");
    let scalar = integer(42);
    let scalar_copy = do_deep_copy(scalar, ptr::null_mut());
    output.push(api.dump(scalar_copy, JSON_ENCODE_ANY).unwrap());
    api.delete(scalar_copy);
    api.delete(scalar);

    let object: Constructor = api.sym(b"json_object");
    let recursive_update: RecursiveUpdate = api.sym(b"do_object_update_recursive");
    let target = object();
    let source = object();
    let h_init: HInit = api.sym(b"hashtable_init");
    let h_close: HClose = api.sym(b"hashtable_close");
    let mut parents: HashTable = std::mem::zeroed();
    assert_eq!(h_init(&mut parents), 0);
    output.push(
        recursive_update(target, source, &mut parents)
            .to_string()
            .into_bytes(),
    );
    h_close(&mut parents);
    api.delete(source);
    api.delete(target);

    let divmax: *mut c_int = api.sym(b"dtoa_divmax");
    output.push((*divmax).to_string().into_bytes());
    let old_seed = *seed_value;
    seed(0x8765_4321);
    output.push((*seed_value == old_seed).to_string().into_bytes());
    output
}

#[test]
fn configs_46_49_57_60_direct_low_level_exports() {
    unsafe {
        let libs = Libraries::open();
        assert_eq!(
            low_level_export_transcript(&libs.c),
            low_level_export_transcript(&libs.rust)
        );
    }
}

#[test]
fn jsonp_strtod_assertion_probe() {
    let Ok(which) = std::env::var("JANSSON_ASSERTION_PROBE") else {
        return;
    };
    unsafe {
        let libs = Libraries::open();
        let api = if which == "C" { &libs.c } else { &libs.rust };
        type StrToD = unsafe extern "C" fn(*mut StrBuffer, *mut f64) -> c_int;
        let strtod: StrToD = api.sym(b"jsonp_strtod");
        let mut text = *b"1x\0";
        let mut buffer = StrBuffer {
            value: text.as_mut_ptr().cast(),
            length: 2,
            size: text.len(),
        };
        let mut output = 0.0;
        let _ = strtod(&mut buffer, &mut output);
    }
}

#[test]
fn jsonp_strtod_assertion_matches() {
    let executable = std::env::current_exe().unwrap();
    let mut statuses = Vec::new();
    for implementation in ["C", "Rust"] {
        statuses.push(
            std::process::Command::new(&executable)
                .arg("jsonp_strtod_assertion_probe")
                .arg("--exact")
                .arg("--test-threads=1")
                .env("JANSSON_ASSERTION_PROBE", implementation)
                .status()
                .unwrap(),
        );
    }
    assert!(!statuses[0].success() && !statuses[1].success());
    assert_eq!(statuses[0].signal(), statuses[1].signal());
}

unsafe fn invalid_api_transcript(api: &Api) -> Vec<i128> {
    let mut out = Vec::new();
    macro_rules! function {
        ($name:expr, $type:ty) => {
            unsafe { api.sym::<$type>($name) }
        };
    }
    let object = function!(b"json_object", unsafe extern "C" fn() -> *mut Json)();
    let array = function!(b"json_array", unsafe extern "C" fn() -> *mut Json)();
    let integer = function!(b"json_integer", unsafe extern "C" fn(i64) -> *mut Json)(1);
    let string = function!(
        b"json_string",
        unsafe extern "C" fn(*const c_char) -> *mut Json
    )(c"x".as_ptr());

    type Get = unsafe extern "C" fn(*const Json, *const c_char) -> *mut Json;
    out.push(function!(b"json_object_get", Get)(object, ptr::null()).is_null() as i128);
    out.push(function!(b"json_object_get", Get)(array, c"x".as_ptr()).is_null() as i128);
    type Set = unsafe extern "C" fn(*mut Json, *const c_char, *mut Json) -> c_int;
    out.push(
        function!(b"json_object_set_new", Set)(object, c"null".as_ptr(), ptr::null_mut()) as i128,
    );
    let self_object = function!(b"json_object", unsafe extern "C" fn() -> *mut Json)();
    out.push(
        function!(b"json_object_set_new", Set)(self_object, c"self".as_ptr(), self_object) as i128,
    );
    out.push(function!(b"json_object_set_new", Set)(
        array,
        c"x".as_ptr(),
        function!(b"json_integer", unsafe extern "C" fn(i64) -> *mut Json)(2),
    ) as i128);
    out.push(function!(b"json_object_set_new", Set)(
        object,
        ptr::null(),
        function!(b"json_integer", unsafe extern "C" fn(i64) -> *mut Json)(3),
    ) as i128);
    let invalid_utf8 = [0xc0_u8, 0x80];
    type SetN = unsafe extern "C" fn(*mut Json, *const c_char, usize, *mut Json) -> c_int;
    out.push(function!(b"json_object_setn_new", SetN)(
        object,
        invalid_utf8.as_ptr().cast(),
        invalid_utf8.len(),
        function!(b"json_integer", unsafe extern "C" fn(i64) -> *mut Json)(4),
    ) as i128);
    type Del = unsafe extern "C" fn(*mut Json, *const c_char) -> c_int;
    out.push(function!(b"json_object_del", Del)(object, ptr::null()) as i128);
    out.push(function!(b"json_object_del", Del)(object, c"missing".as_ptr()) as i128);
    type Clear = unsafe extern "C" fn(*mut Json) -> c_int;
    out.push(function!(b"json_object_clear", Clear)(array) as i128);
    type Update = unsafe extern "C" fn(*mut Json, *mut Json) -> c_int;
    for name in [
        b"json_object_update".as_slice(),
        b"json_object_update_existing".as_slice(),
        b"json_object_update_missing".as_slice(),
        b"json_object_update_recursive".as_slice(),
    ] {
        out.push(function!(name, Update)(array, object) as i128);
        out.push(function!(name, Update)(object, array) as i128);
    }

    let iterator_value = function!(b"json_integer", unsafe extern "C" fn(i64) -> *mut Json)(10);
    assert_eq!(
        function!(b"json_object_set_new", Set)(object, c"iter".as_ptr(), iterator_value,),
        0
    );
    type Iter = unsafe extern "C" fn(*mut Json) -> *mut c_void;
    type IterAt = unsafe extern "C" fn(*mut Json, *const c_char) -> *mut c_void;
    type IterNext = unsafe extern "C" fn(*mut Json, *mut c_void) -> *mut c_void;
    type IterKey = unsafe extern "C" fn(*mut c_void) -> *const c_char;
    type IterKeyLen = unsafe extern "C" fn(*mut c_void) -> usize;
    type IterValue = unsafe extern "C" fn(*mut c_void) -> *mut Json;
    type IterSet = unsafe extern "C" fn(*mut Json, *mut c_void, *mut Json) -> c_int;
    type KeyToIter = unsafe extern "C" fn(*const c_char) -> *mut c_void;
    let iter = function!(b"json_object_iter", Iter)(object);
    out.push(function!(b"json_object_iter", Iter)(array).is_null() as i128);
    out.push(function!(b"json_object_iter_at", IterAt)(object, ptr::null()).is_null() as i128);
    out.push(function!(b"json_object_iter_at", IterAt)(array, c"x".as_ptr()).is_null() as i128);
    out.push(
        function!(b"json_object_iter_at", IterAt)(object, c"missing".as_ptr()).is_null() as i128,
    );
    out.push(function!(b"json_object_iter_next", IterNext)(array, iter).is_null() as i128);
    out.push(
        function!(b"json_object_iter_next", IterNext)(object, ptr::null_mut()).is_null() as i128,
    );
    out.push(function!(b"json_object_iter_key", IterKey)(ptr::null_mut()).is_null() as i128);
    out.push(function!(b"json_object_iter_key_len", IterKeyLen)(ptr::null_mut()) as i128);
    out.push(function!(b"json_object_iter_value", IterValue)(ptr::null_mut()).is_null() as i128);
    out.push(function!(b"json_object_iter_set_new", IterSet)(
        object,
        ptr::null_mut(),
        ptr::null_mut(),
    ) as i128);
    out.push(function!(b"json_object_iter_set_new", IterSet)(
        array,
        iter,
        function!(b"json_integer", unsafe extern "C" fn(i64) -> *mut Json)(11),
    ) as i128);
    out.push(function!(b"json_object_key_to_iter", KeyToIter)(ptr::null()).is_null() as i128);

    type ArrayGet = unsafe extern "C" fn(*const Json, usize) -> *mut Json;
    out.push(function!(b"json_array_get", ArrayGet)(object, 0).is_null() as i128);
    out.push(function!(b"json_array_get", ArrayGet)(array, usize::MAX).is_null() as i128);
    type ArraySet = unsafe extern "C" fn(*mut Json, usize, *mut Json) -> c_int;
    out.push(function!(b"json_array_set_new", ArraySet)(array, 0, ptr::null_mut()) as i128);
    out.push(function!(b"json_array_set_new", ArraySet)(
        array,
        0,
        function!(b"json_integer", unsafe extern "C" fn(i64) -> *mut Json)(5),
    ) as i128);
    type Append = unsafe extern "C" fn(*mut Json, *mut Json) -> c_int;
    out.push(function!(b"json_array_append_new", Append)(array, ptr::null_mut()) as i128);
    let self_array = function!(b"json_array", unsafe extern "C" fn() -> *mut Json)();
    out.push(function!(b"json_array_append_new", Append)(self_array, self_array) as i128);
    type Insert = unsafe extern "C" fn(*mut Json, usize, *mut Json) -> c_int;
    out.push(function!(b"json_array_insert_new", Insert)(array, 0, ptr::null_mut()) as i128);
    out.push(function!(b"json_array_insert_new", Insert)(
        array,
        1,
        function!(b"json_integer", unsafe extern "C" fn(i64) -> *mut Json)(6),
    ) as i128);
    type Remove = unsafe extern "C" fn(*mut Json, usize) -> c_int;
    out.push(function!(b"json_array_remove", Remove)(array, 0) as i128);
    out.push(function!(b"json_array_clear", Clear)(object) as i128);
    out.push(function!(b"json_array_extend", Update)(array, object) as i128);
    out.push(function!(b"json_array_extend", Update)(object, array) as i128);

    type String = unsafe extern "C" fn(*const c_char) -> *mut Json;
    out.push(function!(b"json_string", String)(ptr::null()).is_null() as i128);
    type StringN = unsafe extern "C" fn(*const c_char, usize) -> *mut Json;
    out.push(
        function!(b"json_stringn", StringN)(invalid_utf8.as_ptr().cast(), invalid_utf8.len())
            .is_null() as i128,
    );
    type StringSet = unsafe extern "C" fn(*mut Json, *const c_char) -> c_int;
    out.push(function!(b"json_string_set", StringSet)(string, ptr::null()) as i128);
    out.push(function!(b"json_string_set", StringSet)(integer, c"x".as_ptr()) as i128);
    type StringSetN = unsafe extern "C" fn(*mut Json, *const c_char, usize) -> c_int;
    out.push(function!(b"json_string_setn", StringSetN)(
        string,
        invalid_utf8.as_ptr().cast(),
        invalid_utf8.len(),
    ) as i128);
    type StringValue = unsafe extern "C" fn(*const Json) -> *const c_char;
    type StringLength = unsafe extern "C" fn(*const Json) -> usize;
    out.push(function!(b"json_string_value", StringValue)(integer).is_null() as i128);
    out.push(function!(b"json_string_length", StringLength)(integer) as i128);

    type Real = unsafe extern "C" fn(f64) -> *mut Json;
    for value in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        out.push(function!(b"json_real", Real)(value).is_null() as i128);
    }
    type RealSet = unsafe extern "C" fn(*mut Json, f64) -> c_int;
    type RealValue = unsafe extern "C" fn(*const Json) -> f64;
    out.push(function!(b"json_real_value", RealValue)(integer).to_bits() as i128);
    out.push(function!(b"json_real_set", RealSet)(integer, 1.0) as i128);
    out.push(
        function!(b"json_real_set", RealSet)(function!(b"json_real", Real)(1.0), f64::NAN) as i128,
    );
    type IntSet = unsafe extern "C" fn(*mut Json, i64) -> c_int;
    out.push(function!(b"json_integer_set", IntSet)(string, 1) as i128);
    type IntValue = unsafe extern "C" fn(*const Json) -> i64;
    out.push(function!(b"json_integer_value", IntValue)(string) as i128);

    type Equal = unsafe extern "C" fn(*const Json, *const Json) -> c_int;
    out.push(function!(b"json_equal", Equal)(ptr::null(), integer) as i128);
    out.push(function!(b"json_equal", Equal)(integer, string) as i128);
    #[repr(C)]
    struct Header {
        type_: c_int,
        refcount: usize,
    }
    let invalid = Header {
        type_: 99,
        refcount: 1,
    };
    type Copy = unsafe extern "C" fn(*mut Json) -> *mut Json;
    out.push(
        function!(b"json_copy", Copy)((&invalid as *const Header).cast_mut().cast()).is_null()
            as i128,
    );
    out.push(function!(b"json_copy", Copy)(ptr::null_mut()).is_null() as i128);
    out.push(api.dump(integer, 0).is_none() as i128);
    out.push(api.dump(ptr::null(), JSON_ENCODE_ANY).is_none() as i128);
    out.push(
        api.dump((&invalid as *const Header).cast(), JSON_ENCODE_ANY)
            .is_none() as i128,
    );
    type DumpB = unsafe extern "C" fn(*const Json, *mut c_char, usize, usize) -> usize;
    let mut dump_buffer = [0i8; 8];
    out.push(function!(b"json_dumpb", DumpB)(
        ptr::null(),
        dump_buffer.as_mut_ptr(),
        dump_buffer.len(),
        JSON_ENCODE_ANY,
    ) as i128);
    out.push(function!(b"json_dumpb", DumpB)(
        (&invalid as *const Header).cast(),
        dump_buffer.as_mut_ptr(),
        dump_buffer.len(),
        JSON_ENCODE_ANY,
    ) as i128);
    type DumpCallback = unsafe extern "C" fn(
        *const Json,
        Option<unsafe extern "C" fn(*const c_char, usize, *mut c_void) -> c_int>,
        *mut c_void,
        usize,
    ) -> c_int;
    out.push(function!(b"json_dump_callback", DumpCallback)(
        object,
        Some(rejecting_dump_callback),
        ptr::null_mut(),
        JSON_COMPACT,
    ) as i128);
    type DumpFile = unsafe extern "C" fn(*const Json, *const c_char, usize) -> c_int;
    out.push(function!(b"json_dump_file", DumpFile)(
        object,
        c"/definitely/not/a/jansson/output/file".as_ptr(),
        JSON_COMPACT,
    ) as i128);

    type LoadCallback = unsafe extern "C" fn(
        Option<unsafe extern "C" fn(*mut c_void, usize, *mut c_void) -> usize>,
        *mut c_void,
        usize,
        *mut JsonError,
    ) -> *mut Json;
    let load_callback: LoadCallback = function!(b"json_load_callback", LoadCallback);
    let mut callback_error = JsonError::default();
    let callback_value = load_callback(None, ptr::null_mut(), 0, &mut callback_error);
    out.push(callback_value.is_null() as i128);
    out.push(callback_error.code() as i128);
    callback_error = JsonError::default();
    let callback_value = load_callback(
        Some(failing_load_callback),
        ptr::null_mut(),
        0,
        &mut callback_error,
    );
    out.push(callback_value.is_null() as i128);
    out.push(callback_error.code() as i128);

    type StrToD = unsafe extern "C" fn(*mut StrBuffer, *mut f64) -> c_int;
    let mut overflow_text = *b"1e9999\0";
    let mut overflow_buffer = StrBuffer {
        value: overflow_text.as_mut_ptr().cast(),
        length: 6,
        size: overflow_text.len(),
    };
    let mut parsed = 0.0;
    out.push(function!(b"jsonp_strtod", StrToD)(&mut overflow_buffer, &mut parsed) as i128);

    let cycle_a = function!(b"json_object", unsafe extern "C" fn() -> *mut Json)();
    let cycle_b = function!(b"json_object", unsafe extern "C" fn() -> *mut Json)();
    *cycle_a.cast::<usize>().add(1) += 1;
    *cycle_b.cast::<usize>().add(1) += 1;
    assert_eq!(
        function!(b"json_object_set_new", Set)(cycle_a, c"b".as_ptr(), cycle_b),
        0
    );
    assert_eq!(
        function!(b"json_object_set_new", Set)(cycle_b, c"a".as_ptr(), cycle_a),
        0
    );
    type DeepCopy = unsafe extern "C" fn(*const Json) -> *mut Json;
    out.push(function!(b"json_deep_copy", DeepCopy)(cycle_a).is_null() as i128);
    out.push(api.dump(cycle_a, JSON_COMPACT).is_none() as i128);

    let recursive_target = function!(b"json_object", unsafe extern "C" fn() -> *mut Json)();
    let recursive_b = function!(b"json_object", unsafe extern "C" fn() -> *mut Json)();
    let recursive_a = function!(b"json_object", unsafe extern "C" fn() -> *mut Json)();
    assert_eq!(
        function!(b"json_object_set_new", Set)(recursive_target, c"b".as_ptr(), recursive_b,),
        0
    );
    assert_eq!(
        function!(b"json_object_set_new", Set)(recursive_b, c"a".as_ptr(), recursive_a,),
        0
    );
    out.push(function!(b"json_object_update_recursive", Update)(recursive_target, cycle_a) as i128);
    api.delete(recursive_target);

    assert_eq!(
        function!(b"json_object_del", Del)(cycle_a, c"b".as_ptr()),
        0
    );
    assert_eq!(
        function!(b"json_object_del", Del)(cycle_b, c"a".as_ptr()),
        0
    );
    api.delete(cycle_b);
    api.delete(cycle_a);

    type Malloc = unsafe extern "C" fn(usize) -> *mut c_void;
    out.push(function!(b"jsonp_malloc", Malloc)(0).is_null() as i128);
    type DtoStr = unsafe extern "C" fn(*mut c_char, usize, f64, c_int) -> c_int;
    let mut tiny = [0i8; 2];
    out.push(function!(b"jsonp_dtostr", DtoStr)(tiny.as_mut_ptr(), 2, 1.25, 0) as i128);
    type DtoaR = unsafe extern "C" fn(
        f64,
        c_int,
        c_int,
        *mut c_int,
        *mut c_int,
        *mut *mut c_char,
        *mut c_char,
        usize,
    ) -> *mut c_char;
    let mut one = [0i8; 1];
    let mut decpt = 0;
    let mut sign = 0;
    let mut end = ptr::null_mut();
    out.push(
        function!(b"dtoa_r", DtoaR)(
            1.25,
            0,
            0,
            &mut decpt,
            &mut sign,
            &mut end,
            one.as_mut_ptr(),
            1,
        )
        .is_null() as i128,
    );

    type SbAppend = unsafe extern "C" fn(*mut StrBuffer, *const c_char, usize) -> c_int;
    let mut sb = StrBuffer {
        value: one.as_mut_ptr(),
        length: 0,
        size: 1,
    };
    out.push(
        function!(b"strbuffer_append_bytes", SbAppend)(&mut sb, one.as_ptr(), usize::MAX) as i128,
    );

    api.delete(string);
    api.delete(integer);
    api.delete(array);
    api.delete(object);
    out
}

#[test]
fn errors_1_through_77_and_103_through_108_nonvariadic_boundaries() {
    unsafe {
        let libs = Libraries::open();
        assert_eq!(
            invalid_api_transcript(&libs.c),
            invalid_api_transcript(&libs.rust)
        );
    }
}
