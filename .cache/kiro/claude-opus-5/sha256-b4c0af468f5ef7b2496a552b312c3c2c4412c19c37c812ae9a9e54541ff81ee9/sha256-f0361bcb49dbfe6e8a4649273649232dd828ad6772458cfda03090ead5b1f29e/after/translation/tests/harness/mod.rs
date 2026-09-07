//! Shared harness: loads BOTH the C `.so` and the Rust `.so` through
//! `libloading` and exposes their exported symbols behind an identical
//! interface, so every test is a true differential test across the FFI
//! boundary.
#![allow(
    non_snake_case,
    dead_code,
    non_camel_case_types,
    non_upper_case_globals
)]

use libloading::{Library, Symbol};
use std::ffi::{c_char, c_double, c_float, c_int, c_void, CStr};
use std::path::PathBuf;

/* ------------------------------------------------------------------ */
/* cJSON ABI types                                                     */
/* ------------------------------------------------------------------ */

pub const cJSON_Invalid: c_int = 0;
pub const cJSON_False: c_int = 1 << 0;
pub const cJSON_True: c_int = 1 << 1;
pub const cJSON_NULL: c_int = 1 << 2;
pub const cJSON_Number: c_int = 1 << 3;
pub const cJSON_String: c_int = 1 << 4;
pub const cJSON_Array: c_int = 1 << 5;
pub const cJSON_Object: c_int = 1 << 6;
pub const cJSON_Raw: c_int = 1 << 7;
pub const cJSON_IsReference: c_int = 256;
pub const cJSON_StringIsConst: c_int = 512;

#[repr(C)]
#[derive(Debug, Clone, Copy)]
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
#[derive(Clone, Copy)]
pub struct cJSON_Hooks {
    pub malloc_fn: Option<unsafe extern "C" fn(usize) -> *mut c_void>,
    pub free_fn: Option<unsafe extern "C" fn(*mut c_void)>,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct record {
    pub precision: *const c_char,
    pub lat: c_double,
    pub lon: c_double,
    pub address: *const c_char,
    pub city: *const c_char,
    pub state: *const c_char,
    pub zip: *const c_char,
    pub country: *const c_char,
}

/* A plain, comparable snapshot of a cJSON node (pointers replaced by
 * structural information) so C and Rust trees can be compared for equality
 * even though their addresses differ. */
#[derive(Debug, PartialEq, Clone)]
pub enum Snap {
    Null,
    Node {
        type_: c_int,
        valuestring: Option<Vec<u8>>,
        valueint: c_int,
        valuedouble_bits: u64,
        string: Option<Vec<u8>>,
        children: Vec<Snap>,
    },
}

/* ------------------------------------------------------------------ */
/* Function-pointer table                                              */
/* ------------------------------------------------------------------ */

macro_rules! api {
    ( $( $name:ident : $ty:ty ),* $(,)? ) => {
        pub struct Api {
            _lib: Library,
            pub tag: &'static str,
            $( pub $name: $ty, )*
        }
        impl Api {
            unsafe fn load(path: &std::path::Path, tag: &'static str) -> Api {
                let lib = Library::new(path)
                    .unwrap_or_else(|e| panic!("cannot load {}: {e}", path.display()));
                $(
                    let $name: $ty = {
                        let s: Symbol<$ty> = lib
                            .get(concat!(stringify!($name), "\0").as_bytes())
                            .unwrap_or_else(|e| panic!("{tag}: missing symbol {}: {e}", stringify!($name)));
                        *s
                    };
                )*
                Api { _lib: lib, tag, $( $name, )* }
            }
        }
    };
}

api! {
    cJSON_Version: unsafe extern "C" fn() -> *const c_char,
    cJSON_InitHooks: unsafe extern "C" fn(*mut cJSON_Hooks),
    cJSON_Parse: unsafe extern "C" fn(*const c_char) -> *mut cJSON,
    cJSON_ParseWithLength: unsafe extern "C" fn(*const c_char, usize) -> *mut cJSON,
    cJSON_ParseWithOpts:
        unsafe extern "C" fn(*const c_char, *mut *const c_char, c_int) -> *mut cJSON,
    cJSON_ParseWithLengthOpts:
        unsafe extern "C" fn(*const c_char, usize, *mut *const c_char, c_int) -> *mut cJSON,
    cJSON_Print: unsafe extern "C" fn(*const cJSON) -> *mut c_char,
    cJSON_PrintUnformatted: unsafe extern "C" fn(*const cJSON) -> *mut c_char,
    cJSON_PrintBuffered: unsafe extern "C" fn(*const cJSON, c_int, c_int) -> *mut c_char,
    cJSON_PrintPreallocated:
        unsafe extern "C" fn(*mut cJSON, *mut c_char, c_int, c_int) -> c_int,
    cJSON_Delete: unsafe extern "C" fn(*mut cJSON),
    cJSON_GetArraySize: unsafe extern "C" fn(*const cJSON) -> c_int,
    cJSON_GetArrayItem: unsafe extern "C" fn(*const cJSON, c_int) -> *mut cJSON,
    cJSON_GetObjectItem: unsafe extern "C" fn(*const cJSON, *const c_char) -> *mut cJSON,
    cJSON_GetObjectItemCaseSensitive:
        unsafe extern "C" fn(*const cJSON, *const c_char) -> *mut cJSON,
    cJSON_HasObjectItem: unsafe extern "C" fn(*const cJSON, *const c_char) -> c_int,
    cJSON_GetErrorPtr: unsafe extern "C" fn() -> *const c_char,
    cJSON_GetStringValue: unsafe extern "C" fn(*const cJSON) -> *mut c_char,
    cJSON_GetNumberValue: unsafe extern "C" fn(*const cJSON) -> c_double,
    cJSON_IsInvalid: unsafe extern "C" fn(*const cJSON) -> c_int,
    cJSON_IsFalse: unsafe extern "C" fn(*const cJSON) -> c_int,
    cJSON_IsTrue: unsafe extern "C" fn(*const cJSON) -> c_int,
    cJSON_IsBool: unsafe extern "C" fn(*const cJSON) -> c_int,
    cJSON_IsNull: unsafe extern "C" fn(*const cJSON) -> c_int,
    cJSON_IsNumber: unsafe extern "C" fn(*const cJSON) -> c_int,
    cJSON_IsString: unsafe extern "C" fn(*const cJSON) -> c_int,
    cJSON_IsArray: unsafe extern "C" fn(*const cJSON) -> c_int,
    cJSON_IsObject: unsafe extern "C" fn(*const cJSON) -> c_int,
    cJSON_IsRaw: unsafe extern "C" fn(*const cJSON) -> c_int,
    cJSON_CreateNull: unsafe extern "C" fn() -> *mut cJSON,
    cJSON_CreateTrue: unsafe extern "C" fn() -> *mut cJSON,
    cJSON_CreateFalse: unsafe extern "C" fn() -> *mut cJSON,
    cJSON_CreateBool: unsafe extern "C" fn(c_int) -> *mut cJSON,
    cJSON_CreateNumber: unsafe extern "C" fn(c_double) -> *mut cJSON,
    cJSON_CreateString: unsafe extern "C" fn(*const c_char) -> *mut cJSON,
    cJSON_CreateRaw: unsafe extern "C" fn(*const c_char) -> *mut cJSON,
    cJSON_CreateArray: unsafe extern "C" fn() -> *mut cJSON,
    cJSON_CreateObject: unsafe extern "C" fn() -> *mut cJSON,
    cJSON_CreateStringReference: unsafe extern "C" fn(*const c_char) -> *mut cJSON,
    cJSON_CreateObjectReference: unsafe extern "C" fn(*const cJSON) -> *mut cJSON,
    cJSON_CreateArrayReference: unsafe extern "C" fn(*const cJSON) -> *mut cJSON,
    cJSON_CreateIntArray: unsafe extern "C" fn(*const c_int, c_int) -> *mut cJSON,
    cJSON_CreateFloatArray: unsafe extern "C" fn(*const c_float, c_int) -> *mut cJSON,
    cJSON_CreateDoubleArray: unsafe extern "C" fn(*const c_double, c_int) -> *mut cJSON,
    cJSON_CreateStringArray: unsafe extern "C" fn(*const *const c_char, c_int) -> *mut cJSON,
    cJSON_AddItemToArray: unsafe extern "C" fn(*mut cJSON, *mut cJSON) -> c_int,
    cJSON_AddItemToObject:
        unsafe extern "C" fn(*mut cJSON, *const c_char, *mut cJSON) -> c_int,
    cJSON_AddItemToObjectCS:
        unsafe extern "C" fn(*mut cJSON, *const c_char, *mut cJSON) -> c_int,
    cJSON_AddItemReferenceToArray: unsafe extern "C" fn(*mut cJSON, *mut cJSON) -> c_int,
    cJSON_AddItemReferenceToObject:
        unsafe extern "C" fn(*mut cJSON, *const c_char, *mut cJSON) -> c_int,
    cJSON_AddNullToObject: unsafe extern "C" fn(*mut cJSON, *const c_char) -> *mut cJSON,
    cJSON_AddTrueToObject: unsafe extern "C" fn(*mut cJSON, *const c_char) -> *mut cJSON,
    cJSON_AddFalseToObject: unsafe extern "C" fn(*mut cJSON, *const c_char) -> *mut cJSON,
    cJSON_AddBoolToObject:
        unsafe extern "C" fn(*mut cJSON, *const c_char, c_int) -> *mut cJSON,
    cJSON_AddNumberToObject:
        unsafe extern "C" fn(*mut cJSON, *const c_char, c_double) -> *mut cJSON,
    cJSON_AddStringToObject:
        unsafe extern "C" fn(*mut cJSON, *const c_char, *const c_char) -> *mut cJSON,
    cJSON_AddRawToObject:
        unsafe extern "C" fn(*mut cJSON, *const c_char, *const c_char) -> *mut cJSON,
    cJSON_AddObjectToObject: unsafe extern "C" fn(*mut cJSON, *const c_char) -> *mut cJSON,
    cJSON_AddArrayToObject: unsafe extern "C" fn(*mut cJSON, *const c_char) -> *mut cJSON,
    cJSON_DetachItemViaPointer: unsafe extern "C" fn(*mut cJSON, *mut cJSON) -> *mut cJSON,
    cJSON_DetachItemFromArray: unsafe extern "C" fn(*mut cJSON, c_int) -> *mut cJSON,
    cJSON_DeleteItemFromArray: unsafe extern "C" fn(*mut cJSON, c_int),
    cJSON_DetachItemFromObject:
        unsafe extern "C" fn(*mut cJSON, *const c_char) -> *mut cJSON,
    cJSON_DetachItemFromObjectCaseSensitive:
        unsafe extern "C" fn(*mut cJSON, *const c_char) -> *mut cJSON,
    cJSON_DeleteItemFromObject: unsafe extern "C" fn(*mut cJSON, *const c_char),
    cJSON_DeleteItemFromObjectCaseSensitive:
        unsafe extern "C" fn(*mut cJSON, *const c_char),
    cJSON_InsertItemInArray: unsafe extern "C" fn(*mut cJSON, c_int, *mut cJSON) -> c_int,
    cJSON_ReplaceItemViaPointer:
        unsafe extern "C" fn(*mut cJSON, *mut cJSON, *mut cJSON) -> c_int,
    cJSON_ReplaceItemInArray: unsafe extern "C" fn(*mut cJSON, c_int, *mut cJSON) -> c_int,
    cJSON_ReplaceItemInObject:
        unsafe extern "C" fn(*mut cJSON, *const c_char, *mut cJSON) -> c_int,
    cJSON_ReplaceItemInObjectCaseSensitive:
        unsafe extern "C" fn(*mut cJSON, *const c_char, *mut cJSON) -> c_int,
    cJSON_Duplicate: unsafe extern "C" fn(*const cJSON, c_int) -> *mut cJSON,
    cJSON_Compare: unsafe extern "C" fn(*const cJSON, *const cJSON, c_int) -> c_int,
    cJSON_Minify: unsafe extern "C" fn(*mut c_char),
    cJSON_SetNumberHelper: unsafe extern "C" fn(*mut cJSON, c_double) -> c_double,
    cJSON_SetValuestring: unsafe extern "C" fn(*mut cJSON, *const c_char) -> *mut c_char,
    cJSON_malloc: unsafe extern "C" fn(usize) -> *mut c_void,
    cJSON_free: unsafe extern "C" fn(*mut c_void),
}

fn root() -> PathBuf {
    // tests/ -> crate root -> workspace root
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf()
}

fn c_so() -> PathBuf {
    let p = root().join("c_src/build/libcjson.so.1.7.19");
    assert!(p.exists(), "C .so not built: {}", p.display());
    p
}

fn c_test_so() -> PathBuf {
    let p = root().join("c_src/build/libcJSON_test.so");
    assert!(p.exists(), "C test .so not built: {}", p.display());
    p
}

pub fn rust_so_path() -> PathBuf {
    rust_so()
}

fn rust_so() -> PathBuf {
    /* Allow the suite to be pointed at a specific build (the release cdylib is
     * the one whose symbol table is compared in Phase D, so it must pass the
     * same differential tests as the debug one). */
    if let Ok(p) = std::env::var("CJSON_RUST_SO") {
        let p = PathBuf::from(p);
        assert!(p.exists(), "CJSON_RUST_SO={} does not exist", p.display());
        assert_not_stale(&p);
        return p;
    }
    let base = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target");
    for prof in ["debug", "release"] {
        let p = base.join(prof).join("libcJSON_test.so");
        if p.exists() {
            assert_not_stale(&p);
            return p;
        }
    }
    panic!(
        "Rust cdylib not found under {} — run `cargo build` first \
         (`cargo test` does not build a cdylib that nothing links against)",
        base.display()
    );
}

/// `cargo test` builds the integration-test binaries but NOT the `cdylib`,
/// because no Rust target links against it.  A stale `.so` would silently make
/// the whole differential suite meaningless, so refuse to run against one.
fn assert_not_stale(so: &std::path::Path) {
    let so_time = std::fs::metadata(so).and_then(|m| m.modified()).unwrap();
    let src = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut newest = None::<(std::time::SystemTime, PathBuf)>;
    let mut stack = vec![src];
    while let Some(dir) = stack.pop() {
        if let Ok(rd) = std::fs::read_dir(&dir) {
            for e in rd.flatten() {
                let p = e.path();
                if p.is_dir() {
                    stack.push(p);
                } else if p.extension().map(|x| x == "rs").unwrap_or(false) {
                    if let Ok(t) = e.metadata().and_then(|m| m.modified()) {
                        if newest.as_ref().map(|(n, _)| t > *n).unwrap_or(true) {
                            newest = Some((t, p));
                        }
                    }
                }
            }
        }
    }
    if let Some((t, p)) = newest {
        assert!(
            so_time >= t,
            "STALE Rust .so: {} is older than {}.\n\
             Run `cargo build --all-targets` (or `./run_verification.sh`) before `cargo test`.",
            so.display(),
            p.display()
        );
    }
}

pub struct Pair {
    pub c: Api,
    pub r: Api,
}

/// Both libraries, loaded once per process.
pub fn pair() -> &'static Pair {
    use std::sync::OnceLock;
    static P: OnceLock<Pair> = OnceLock::new();
    P.get_or_init(|| unsafe {
        Pair {
            c: Api::load(&c_so(), "C"),
            r: Api::load(&rust_so(), "RUST"),
        }
    })
}

pub fn c_driver_so_path() -> PathBuf {
    c_test_so()
}

/* ------------------------------------------------------------------ */
/* Helpers                                                             */
/* ------------------------------------------------------------------ */

/// NUL-terminated byte buffer usable as `*const c_char`.
pub fn cstr(s: &str) -> Vec<u8> {
    let mut v = s.as_bytes().to_vec();
    v.push(0);
    v
}

pub fn cbytes(s: &[u8]) -> Vec<u8> {
    let mut v = s.to_vec();
    v.push(0);
    v
}

pub fn sp(v: &[u8]) -> *const c_char {
    v.as_ptr() as *const c_char
}

pub unsafe fn read_cstr(ptr: *const c_char) -> Option<Vec<u8>> {
    if ptr.is_null() {
        None
    } else {
        Some(CStr::from_ptr(ptr).to_bytes().to_vec())
    }
}

/// Take a printed string from `api`, snapshot the bytes, then free it with the
/// SAME library's `cJSON_free` (allocator ownership must not cross libraries).
pub unsafe fn take_print(api: &Api, ptr: *mut c_char) -> Option<Vec<u8>> {
    if ptr.is_null() {
        return None;
    }
    let out = CStr::from_ptr(ptr).to_bytes().to_vec();
    (api.cJSON_free)(ptr as *mut c_void);
    Some(out)
}

/// Structural snapshot of a tree (recursion depth bounded to avoid blowing the
/// stack on adversarial inputs).
pub unsafe fn snap(item: *const cJSON) -> Snap {
    snap_d(item, 0)
}

unsafe fn snap_d(item: *const cJSON, depth: u32) -> Snap {
    if item.is_null() || depth > 2000 {
        return Snap::Null;
    }
    let it = &*item;
    let mut children = Vec::new();
    let mut cur = it.child;
    let mut guard = 0;
    while !cur.is_null() && guard < 100_000 {
        children.push(snap_d(cur, depth + 1));
        cur = (*cur).next;
        guard += 1;
    }
    Snap::Node {
        type_: it.type_,
        valuestring: read_cstr(it.valuestring),
        valueint: it.valueint,
        valuedouble_bits: it.valuedouble.to_bits(),
        string: read_cstr(it.string),
        children,
    }
}

/// Assert two nodes snapshot identically.
pub unsafe fn assert_snap_eq(c: *const cJSON, r: *const cJSON, what: &str) {
    let sc = snap(c);
    let sr = snap(r);
    assert_eq!(sc, sr, "tree mismatch for {what}");
}

/// Print an item through all four print entry points and return the results,
/// so a single call site covers CONFIGS rows 50-56.
pub unsafe fn print_all(api: &Api, item: *mut cJSON) -> Vec<Option<Vec<u8>>> {
    let mut out = Vec::new();
    out.push(take_print(api, (api.cJSON_Print)(item)));
    out.push(take_print(api, (api.cJSON_PrintUnformatted)(item)));
    for pre in [0i32, 1, 2, 16, 256, 4096] {
        for fmt in [0i32, 1] {
            out.push(take_print(api, (api.cJSON_PrintBuffered)(item, pre, fmt)));
        }
    }
    // PrintPreallocated: exact, generous and one-byte-short buffers.
    for fmt in [0i32, 1] {
        let reference = take_print(
            api,
            if fmt == 1 {
                (api.cJSON_Print)(item)
            } else {
                (api.cJSON_PrintUnformatted)(item)
            },
        );
        let need = reference.as_ref().map(|v| v.len() + 1).unwrap_or(1);
        for len in [need, need + 5, need.saturating_sub(1), 0] {
            let mut buf = vec![0u8; len.max(1) + 8];
            let ok = (api.cJSON_PrintPreallocated)(
                item,
                buf.as_mut_ptr() as *mut c_char,
                len as c_int,
                fmt,
            );
            out.push(Some({
                let mut v = vec![if ok != 0 { 1u8 } else { 0u8 }];
                v.extend_from_slice(&buf);
                v
            }));
        }
    }
    out
}

/* ------------------------------------------------------------------ */
/* Deterministic PRNG (xorshift64*) — fixed seed, reproducible          */
/* ------------------------------------------------------------------ */

pub struct Rng(pub u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng(seed | 1)
    }
    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
    pub fn below(&mut self, n: usize) -> usize {
        if n == 0 {
            0
        } else {
            (self.next_u64() % n as u64) as usize
        }
    }
    pub fn i32(&mut self) -> i32 {
        self.next_u64() as i32
    }
    pub fn f64(&mut self) -> f64 {
        f64::from_bits(self.next_u64())
    }
    /// "Interesting" finite-ish double.
    pub fn nice_f64(&mut self) -> f64 {
        match self.below(10) {
            0 => 0.0,
            1 => -0.0,
            2 => f64::NAN,
            3 => f64::INFINITY,
            4 => f64::NEG_INFINITY,
            5 => self.i32() as f64,
            6 => (self.i32() as f64) / 7.0,
            7 => self.f64(),
            8 => {
                let m = (self.next_u64() % 1_000_000) as f64;
                let e = (self.below(60) as i32) - 30;
                m * 10f64.powi(e)
            }
            _ => (self.next_u64() as f64) * 1e-9,
        }
    }
    /// Random byte string with no interior NUL (C strings can't carry one).
    pub fn cstring(&mut self, maxlen: usize) -> Vec<u8> {
        let n = self.below(maxlen + 1);
        let mut v = Vec::with_capacity(n);
        for _ in 0..n {
            let b = match self.below(8) {
                0 => (self.below(0x1f) + 1) as u8,     // control chars
                1 => b'"',
                2 => b'\\',
                3 => *b"\x08\x0c\n\r\t".get(self.below(5)).unwrap(),
                4 => (0x80 + self.below(0x7f)) as u8, // high bytes
                _ => (0x20 + self.below(0x5f)) as u8, // printable ASCII
            };
            v.push(if b == 0 { b'x' } else { b });
        }
        v
    }
}

/* ------------------------------------------------------------------ */
/* Random JSON document generator (valid documents)                     */
/* ------------------------------------------------------------------ */

pub fn gen_json(rng: &mut Rng, depth: u32) -> String {
    let leaf = depth >= 4 || rng.below(100) < 45;
    if leaf {
        match rng.below(9) {
            0 => "null".into(),
            1 => "true".into(),
            2 => "false".into(),
            3 => format!("{}", rng.i32()),
            4 => format!("{:?}", (rng.i32() as f64) / 3.0),
            5 => format!("{}e{}", rng.below(1000), (rng.below(40) as i32) - 20),
            6 => format!("-{}.{}", rng.below(1000), rng.below(1_000_000)),
            7 => format!("\"{}\"", gen_json_string_body(rng)),
            _ => format!("\"{}\"", gen_json_string_body(rng)),
        }
    } else if rng.below(2) == 0 {
        let n = rng.below(6);
        let items: Vec<String> = (0..n).map(|_| gen_json(rng, depth + 1)).collect();
        format!("[{}]", items.join(","))
    } else {
        let n = rng.below(6);
        let items: Vec<String> = (0..n)
            .map(|i| {
                format!(
                    "\"k{}{}\":{}",
                    i,
                    gen_json_string_body(rng),
                    gen_json(rng, depth + 1)
                )
            })
            .collect();
        format!("{{{}}}", items.join(","))
    }
}

/// Body of a JSON string literal: exercises every escape production.
pub fn gen_json_string_body(rng: &mut Rng) -> String {
    let n = rng.below(8);
    let mut s = String::new();
    for _ in 0..n {
        match rng.below(14) {
            0 => s.push_str("\\\""),
            1 => s.push_str("\\\\"),
            2 => s.push_str("\\/"),
            3 => s.push_str("\\b"),
            4 => s.push_str("\\f"),
            5 => s.push_str("\\n"),
            6 => s.push_str("\\r"),
            7 => s.push_str("\\t"),
            8 => s.push_str(&format!("\\u{:04x}", rng.below(0xd7ff) + 1)),
            9 => s.push_str("\\ud83d\\ude00"), // valid surrogate pair
            10 => s.push_str("\\u00e9"),
            11 => s.push('é'),
            12 => s.push('€'),
            _ => s.push((0x20 + rng.below(0x5f)) as u8 as char),
        }
    }
    // never emit a bare '"' or '\'
    s
}

/* Whitespace sprinkling (all bytes <= 0x20 are whitespace to cJSON). */
pub fn sprinkle_ws(rng: &mut Rng, json: &str) -> String {
    let ws = [' ', '\t', '\n', '\r'];
    let mut out = String::new();
    for ch in json.chars() {
        if rng.below(6) == 0 {
            out.push(ws[rng.below(4)]);
        }
        out.push(ch);
    }
    out
}

/* ------------------------------------------------------------------ */
/* Global-state serialisation                                          */
/* ------------------------------------------------------------------ */

/* cJSON keeps process-wide mutable state (`global_error` used by
 * `cJSON_GetErrorPtr`, and `global_hooks` set by `cJSON_InitHooks`).  Cargo
 * runs #[test] functions on parallel threads, so any test that observes or
 * mutates that state must hold this lock for its whole body — otherwise
 * unrelated tests interleave and produce spurious differences. */
pub fn lock() -> std::sync::MutexGuard<'static, ()> {
    use std::sync::{Mutex, OnceLock};
    static M: OnceLock<Mutex<()>> = OnceLock::new();
    let m = M.get_or_init(|| Mutex::new(()));
    match m.lock() {
        Ok(g) => g,
        Err(e) => e.into_inner(), // ignore poisoning from an earlier failure
    }
}
