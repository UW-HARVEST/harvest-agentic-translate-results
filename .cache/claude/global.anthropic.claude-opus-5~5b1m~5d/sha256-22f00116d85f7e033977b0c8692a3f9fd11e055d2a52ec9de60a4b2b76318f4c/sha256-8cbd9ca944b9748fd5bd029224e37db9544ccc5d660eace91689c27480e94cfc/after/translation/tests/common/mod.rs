//! Shared differential-test harness.
//!
//! BOTH implementations are loaded as shared objects through `libloading`; no
//! Rust function is ever called directly, so the `#[no_mangle]` export wrappers
//! are exercised exactly like an external C consumer would.

#![allow(non_snake_case, non_upper_case_globals, dead_code)]

use std::ffi::{c_char, c_int, c_void, CStr, CString};
use std::path::PathBuf;

// ---------------------------------------------------------------- cJSON types

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
#[derive(Debug)]
pub struct CJson {
    pub next: *mut CJson,
    pub prev: *mut CJson,
    pub child: *mut CJson,
    pub type_: c_int,
    pub valuestring: *mut c_char,
    pub valueint: c_int,
    pub valuedouble: f64,
    pub string: *mut c_char,
}

#[repr(C)]
pub struct CJsonHooks {
    pub malloc_fn: Option<unsafe extern "C" fn(usize) -> *mut c_void>,
    pub free_fn: Option<unsafe extern "C" fn(*mut c_void)>,
}

// ------------------------------------------------------------- symbol loading

macro_rules! api {
    ( $( $name:ident : $t:ty , )* ) => {
        pub struct Api {
            pub tag: &'static str,
            _lib: libloading::Library,
            $( pub $name : $t , )*
        }

        impl Api {
            pub fn load(tag: &'static str, path: &std::path::Path) -> Api {
                unsafe {
                    let lib = libloading::Library::new(path)
                        .unwrap_or_else(|e| panic!("dlopen {:?}: {}", path, e));
                    $(
                        let $name : $t = *lib
                            .get::<$t>(concat!(stringify!($name), "\0").as_bytes())
                            .unwrap_or_else(|e| panic!(
                                "{}: missing symbol {}: {}", tag, stringify!($name), e));
                    )*
                    Api { tag, _lib: lib, $( $name , )* }
                }
            }
        }
    };
}

pub type Item = *mut CJson;

api! {
    cJSON_Version: unsafe extern "C" fn() -> *const c_char,
    cJSON_InitHooks: unsafe extern "C" fn(*mut CJsonHooks),

    cJSON_Parse: unsafe extern "C" fn(*const c_char) -> Item,
    cJSON_ParseWithLength: unsafe extern "C" fn(*const c_char, usize) -> Item,
    cJSON_ParseWithOpts: unsafe extern "C" fn(*const c_char, *mut *const c_char, c_int) -> Item,
    cJSON_ParseWithLengthOpts:
        unsafe extern "C" fn(*const c_char, usize, *mut *const c_char, c_int) -> Item,

    cJSON_Print: unsafe extern "C" fn(*const CJson) -> *mut c_char,
    cJSON_PrintUnformatted: unsafe extern "C" fn(*const CJson) -> *mut c_char,
    cJSON_PrintBuffered: unsafe extern "C" fn(*const CJson, c_int, c_int) -> *mut c_char,
    cJSON_PrintPreallocated: unsafe extern "C" fn(Item, *mut c_char, c_int, c_int) -> c_int,
    cJSON_Delete: unsafe extern "C" fn(Item),

    cJSON_GetArraySize: unsafe extern "C" fn(*const CJson) -> c_int,
    cJSON_GetArrayItem: unsafe extern "C" fn(*const CJson, c_int) -> Item,
    cJSON_GetObjectItem: unsafe extern "C" fn(*const CJson, *const c_char) -> Item,
    cJSON_GetObjectItemCaseSensitive: unsafe extern "C" fn(*const CJson, *const c_char) -> Item,
    cJSON_HasObjectItem: unsafe extern "C" fn(*const CJson, *const c_char) -> c_int,
    cJSON_GetErrorPtr: unsafe extern "C" fn() -> *const c_char,

    cJSON_GetStringValue: unsafe extern "C" fn(*const CJson) -> *mut c_char,
    cJSON_GetNumberValue: unsafe extern "C" fn(*const CJson) -> f64,

    cJSON_IsInvalid: unsafe extern "C" fn(*const CJson) -> c_int,
    cJSON_IsFalse: unsafe extern "C" fn(*const CJson) -> c_int,
    cJSON_IsTrue: unsafe extern "C" fn(*const CJson) -> c_int,
    cJSON_IsBool: unsafe extern "C" fn(*const CJson) -> c_int,
    cJSON_IsNull: unsafe extern "C" fn(*const CJson) -> c_int,
    cJSON_IsNumber: unsafe extern "C" fn(*const CJson) -> c_int,
    cJSON_IsString: unsafe extern "C" fn(*const CJson) -> c_int,
    cJSON_IsArray: unsafe extern "C" fn(*const CJson) -> c_int,
    cJSON_IsObject: unsafe extern "C" fn(*const CJson) -> c_int,
    cJSON_IsRaw: unsafe extern "C" fn(*const CJson) -> c_int,

    cJSON_CreateNull: unsafe extern "C" fn() -> Item,
    cJSON_CreateTrue: unsafe extern "C" fn() -> Item,
    cJSON_CreateFalse: unsafe extern "C" fn() -> Item,
    cJSON_CreateBool: unsafe extern "C" fn(c_int) -> Item,
    cJSON_CreateNumber: unsafe extern "C" fn(f64) -> Item,
    cJSON_CreateString: unsafe extern "C" fn(*const c_char) -> Item,
    cJSON_CreateRaw: unsafe extern "C" fn(*const c_char) -> Item,
    cJSON_CreateArray: unsafe extern "C" fn() -> Item,
    cJSON_CreateObject: unsafe extern "C" fn() -> Item,
    cJSON_CreateStringReference: unsafe extern "C" fn(*const c_char) -> Item,
    cJSON_CreateObjectReference: unsafe extern "C" fn(*const CJson) -> Item,
    cJSON_CreateArrayReference: unsafe extern "C" fn(*const CJson) -> Item,

    cJSON_CreateIntArray: unsafe extern "C" fn(*const c_int, c_int) -> Item,
    cJSON_CreateFloatArray: unsafe extern "C" fn(*const f32, c_int) -> Item,
    cJSON_CreateDoubleArray: unsafe extern "C" fn(*const f64, c_int) -> Item,
    cJSON_CreateStringArray: unsafe extern "C" fn(*const *const c_char, c_int) -> Item,

    cJSON_AddItemToArray: unsafe extern "C" fn(Item, Item) -> c_int,
    cJSON_AddItemToObject: unsafe extern "C" fn(Item, *const c_char, Item) -> c_int,
    cJSON_AddItemToObjectCS: unsafe extern "C" fn(Item, *const c_char, Item) -> c_int,
    cJSON_AddItemReferenceToArray: unsafe extern "C" fn(Item, Item) -> c_int,
    cJSON_AddItemReferenceToObject: unsafe extern "C" fn(Item, *const c_char, Item) -> c_int,

    cJSON_DetachItemViaPointer: unsafe extern "C" fn(Item, Item) -> Item,
    cJSON_DetachItemFromArray: unsafe extern "C" fn(Item, c_int) -> Item,
    cJSON_DeleteItemFromArray: unsafe extern "C" fn(Item, c_int),
    cJSON_DetachItemFromObject: unsafe extern "C" fn(Item, *const c_char) -> Item,
    cJSON_DetachItemFromObjectCaseSensitive: unsafe extern "C" fn(Item, *const c_char) -> Item,
    cJSON_DeleteItemFromObject: unsafe extern "C" fn(Item, *const c_char),
    cJSON_DeleteItemFromObjectCaseSensitive: unsafe extern "C" fn(Item, *const c_char),

    cJSON_InsertItemInArray: unsafe extern "C" fn(Item, c_int, Item) -> c_int,
    cJSON_ReplaceItemViaPointer: unsafe extern "C" fn(Item, Item, Item) -> c_int,
    cJSON_ReplaceItemInArray: unsafe extern "C" fn(Item, c_int, Item) -> c_int,
    cJSON_ReplaceItemInObject: unsafe extern "C" fn(Item, *const c_char, Item) -> c_int,
    cJSON_ReplaceItemInObjectCaseSensitive: unsafe extern "C" fn(Item, *const c_char, Item) -> c_int,

    cJSON_Duplicate: unsafe extern "C" fn(*const CJson, c_int) -> Item,
    cJSON_Compare: unsafe extern "C" fn(*const CJson, *const CJson, c_int) -> c_int,
    cJSON_Minify: unsafe extern "C" fn(*mut c_char),

    cJSON_AddNullToObject: unsafe extern "C" fn(Item, *const c_char) -> Item,
    cJSON_AddTrueToObject: unsafe extern "C" fn(Item, *const c_char) -> Item,
    cJSON_AddFalseToObject: unsafe extern "C" fn(Item, *const c_char) -> Item,
    cJSON_AddBoolToObject: unsafe extern "C" fn(Item, *const c_char, c_int) -> Item,
    cJSON_AddNumberToObject: unsafe extern "C" fn(Item, *const c_char, f64) -> Item,
    cJSON_AddStringToObject: unsafe extern "C" fn(Item, *const c_char, *const c_char) -> Item,
    cJSON_AddRawToObject: unsafe extern "C" fn(Item, *const c_char, *const c_char) -> Item,
    cJSON_AddObjectToObject: unsafe extern "C" fn(Item, *const c_char) -> Item,
    cJSON_AddArrayToObject: unsafe extern "C" fn(Item, *const c_char) -> Item,

    cJSON_SetNumberHelper: unsafe extern "C" fn(Item, f64) -> f64,
    cJSON_SetValuestring: unsafe extern "C" fn(Item, *const c_char) -> *mut c_char,

    cJSON_malloc: unsafe extern "C" fn(usize) -> *mut c_void,
    cJSON_free: unsafe extern "C" fn(*mut c_void),
}

// -------------------------------------------------------------- library paths

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

pub fn c_lib_path() -> PathBuf {
    let base = manifest_dir().parent().unwrap().join("c_src/build");
    for name in ["libcjson.so.1.7.19", "libcjson.so.1", "libcjson.so"] {
        let p = base.join(name);
        if p.exists() {
            return p;
        }
    }
    panic!("C library not built; run cmake in c_src/build");
}

pub fn c_driver_path() -> PathBuf {
    manifest_dir()
        .parent()
        .unwrap()
        .join("c_src/build/libcJSON_test.so")
}

/// Path of the Rust `cdylib` under test.
///
/// `cargo test` does NOT build the `cdylib` (no test target depends on it), so a
/// stale `.so` from an earlier `cargo build` would silently be tested instead of
/// the current sources.  To make that impossible the crate's library target is
/// (re)built here, into a dedicated target directory so that it cannot deadlock
/// against the outer `cargo test` invocation's lock on `target/`.
pub fn rust_lib_path() -> PathBuf {
    static P: std::sync::OnceLock<PathBuf> = std::sync::OnceLock::new();
    P.get_or_init(|| {
        let target_dir = manifest_dir().join("target/so");
        let profile = if cfg!(debug_assertions) {
            "debug"
        } else {
            "release"
        };
        let mut cmd = std::process::Command::new(
            std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_string()),
        );
        cmd.current_dir(manifest_dir())
            .arg("build")
            .arg("--offline")
            .arg("--lib")
            .arg("--target-dir")
            .arg(&target_dir);
        if profile == "release" {
            cmd.arg("--release");
        }
        let status = cmd
            .status()
            .unwrap_or_else(|e| panic!("failed to spawn cargo to build the cdylib: {}", e));
        assert!(status.success(), "building the Rust cdylib failed");

        let p = target_dir.join(profile).join("libcJSON_test.so");
        assert!(p.exists(), "Rust cdylib not found at {:?}", p);

        // Staleness guard: the .so must be newer than every Rust source file.
        let so_mtime = std::fs::metadata(&p).unwrap().modified().unwrap();
        for entry in std::fs::read_dir(manifest_dir().join("src")).unwrap() {
            let entry = entry.unwrap();
            let m = entry.metadata().unwrap().modified().unwrap();
            assert!(
                m <= so_mtime,
                "{:?} is newer than {:?}: the Rust .so is stale",
                entry.path(),
                p
            );
        }
        p
    })
    .clone()
}

/// The two implementations under test: `.0` is C (ground truth), `.1` is Rust.
pub fn both() -> (Api, Api) {
    (
        Api::load("C", &c_lib_path()),
        Api::load("RUST", &rust_lib_path()),
    )
}

// -------------------------------------------------------------------- helpers

pub fn cs(s: &str) -> CString {
    CString::new(s).unwrap()
}

/// Owns a NUL-terminated byte buffer (allows arbitrary non-UTF8 content).
pub fn cbytes(b: &[u8]) -> Vec<u8> {
    let mut v = b.to_vec();
    v.push(0);
    v
}

/// Read a C string, or `None` for NULL.
pub unsafe fn read_cstr(p: *const c_char) -> Option<Vec<u8>> {
    if p.is_null() {
        None
    } else {
        Some(CStr::from_ptr(p).to_bytes().to_vec())
    }
}

/// `cJSON_Print` + read + free through the SAME library. Returns `None` on NULL.
pub unsafe fn print_and_free(api: &Api, item: *const CJson) -> Option<Vec<u8>> {
    let p = (api.cJSON_Print)(item);
    let r = read_cstr(p);
    if !p.is_null() {
        (api.cJSON_free)(p as *mut c_void);
    }
    r
}

pub unsafe fn print_unformatted_and_free(api: &Api, item: *const CJson) -> Option<Vec<u8>> {
    let p = (api.cJSON_PrintUnformatted)(item);
    let r = read_cstr(p);
    if !p.is_null() {
        (api.cJSON_free)(p as *mut c_void);
    }
    r
}

pub unsafe fn print_buffered_and_free(
    api: &Api,
    item: *const CJson,
    prebuffer: c_int,
    fmt: c_int,
) -> Option<Vec<u8>> {
    let p = (api.cJSON_PrintBuffered)(item, prebuffer, fmt);
    let r = read_cstr(p);
    if !p.is_null() {
        (api.cJSON_free)(p as *mut c_void);
    }
    r
}

pub fn show(v: &Option<Vec<u8>>) -> String {
    match v {
        None => "<NULL>".to_string(),
        Some(b) => String::from_utf8_lossy(b).into_owned(),
    }
}

/// Structural fingerprint of an item that does not depend on addresses.
pub unsafe fn fingerprint(api: &Api, item: *const CJson) -> String {
    if item.is_null() {
        return "NULL".to_string();
    }
    let it = &*item;
    let mut s = format!(
        "{{type={} valueint={} valuedouble={:?} valuestring={:?} string={:?}",
        it.type_,
        it.valueint,
        it.valuedouble,
        read_cstr(it.valuestring).map(|b| String::from_utf8_lossy(&b).into_owned()),
        read_cstr(it.string).map(|b| String::from_utf8_lossy(&b).into_owned()),
    );
    s.push_str(" children=[");
    let mut child = it.child;
    let mut n = 0;
    while !child.is_null() && n < 20000 {
        s.push_str(&fingerprint(api, child));
        s.push(',');
        child = (*child).next;
        n += 1;
    }
    s.push_str("]}");
    s
}

/// Compare bookkeeping links (`prev` of first child etc.) in an addressing
/// independent way: reports, for every level, the index that `child->prev`
/// resolves to (or -1) plus the forward/backward walk lengths.
pub unsafe fn link_shape(item: *const CJson) -> String {
    if item.is_null() {
        return "N".to_string();
    }
    let it = &*item;
    let mut kids: Vec<*mut CJson> = Vec::new();
    let mut c = it.child;
    while !c.is_null() && kids.len() < 20000 {
        kids.push(c);
        c = (*c).next;
    }
    let idx = |p: *mut CJson| -> i64 {
        if p.is_null() {
            return -1;
        }
        kids.iter().position(|&k| k == p).map(|i| i as i64).unwrap_or(-2)
    };
    let mut s = format!("(n={}", kids.len());
    if !kids.is_empty() {
        s.push_str(&format!(
            " firstprev={} lastnext={}",
            idx((*kids[0]).prev),
            idx((*kids[kids.len() - 1]).next)
        ));
        for (i, &k) in kids.iter().enumerate() {
            if i > 0 {
                s.push_str(&format!(" p{}={}", i, idx((*k).prev)));
            }
        }
    }
    for &k in &kids {
        s.push_str(&link_shape(k));
    }
    s.push(')');
    s
}

// ------------------------------------------------------------------------ rng

/// Deterministic splitmix64 — fixed seed for reproducibility.
pub struct Rng(pub u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng(seed)
    }
    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E3779B97F4A7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D049BB133111EB);
        z ^ (z >> 31)
    }
    pub fn below(&mut self, n: u64) -> u64 {
        if n == 0 {
            0
        } else {
            self.next_u64() % n
        }
    }
    pub fn i32(&mut self) -> i32 {
        self.next_u64() as i32
    }
    /// Random double drawn from a distribution rich in interesting bit patterns.
    pub fn f64(&mut self) -> f64 {
        match self.below(10) {
            0 => f64::from_bits(self.next_u64()), // any bit pattern (incl. NaN/Inf)
            1 => self.i32() as f64,
            2 => (self.i32() as f64) / 1000.0,
            3 => self.next_u64() as f64,
            4 => -(self.next_u64() as f64),
            5 => (self.next_u64() as f64) * 1e-300,
            6 => (self.below(1000) as f64) * 1e300,
            7 => (self.i32() as f64) + 0.5,
            8 => 1.0 / ((self.i32() as f64) + 0.25),
            _ => (self.next_u64() >> 11) as f64 / (1u64 << 53) as f64,
        }
    }
    pub fn f32(&mut self) -> f32 {
        match self.below(6) {
            0 => f32::from_bits(self.next_u64() as u32),
            1 => self.i32() as f32,
            2 => (self.i32() as f32) / 32.0,
            3 => (self.below(1000) as f32) * 1e30,
            4 => (self.below(1000) as f32) * 1e-30,
            _ => (self.next_u64() >> 40) as f32 / 16777216.0,
        }
    }
    /// Random printable-ish string, sometimes containing escapes / UTF-8.
    pub fn string(&mut self, maxlen: usize) -> String {
        let n = self.below(maxlen as u64 + 1) as usize;
        let mut s = String::new();
        for _ in 0..n {
            match self.below(12) {
                0 => s.push('"'),
                1 => s.push('\\'),
                2 => s.push('\n'),
                3 => s.push('\t'),
                4 => s.push('\u{1}'),
                5 => s.push('é'),
                6 => s.push('€'),
                7 => s.push('😀'),
                8 => s.push('/'),
                _ => s.push((b'a' + self.below(26) as u8) as char),
            }
        }
        s
    }
}

// ----------------------------------------------------------- tree description

/// A library-agnostic description of a cJSON tree, so that the *same* tree can
/// be built twice: once with the C exports and once with the Rust exports.
#[derive(Clone, Debug)]
pub enum Node {
    Null,
    True,
    False,
    Bool(c_int),
    Number(f64),
    Str(String),
    Raw(String),
    StrRef(String),
    IntArray(Vec<c_int>),
    FloatArray(Vec<f32>),
    DoubleArray(Vec<f64>),
    StringArray(Vec<String>),
    /// Built with `cJSON_CreateArray` + `cJSON_AddItemToArray`.
    Array(Vec<Node>),
    /// Built with `cJSON_CreateObject` + `cJSON_AddItemToObject`.
    Object(Vec<(String, Node)>),
    /// Built with `cJSON_AddItemToObjectCS` (constant key).
    ObjectCS(Vec<(String, Node)>),
    /// Item whose `type` field is overwritten after construction.
    Retyped(Box<Node>, c_int),
}

/// Leaked NUL-terminated key storage (needed for `cJSON_AddItemToObjectCS`,
/// which stores the pointer without copying).
fn leak_cstr(s: &str) -> *const c_char {
    let b = Box::leak(cbytes(s.as_bytes()).into_boxed_slice());
    b.as_ptr() as *const c_char
}

pub unsafe fn build(api: &Api, node: &Node) -> Item {
    match node {
        Node::Null => (api.cJSON_CreateNull)(),
        Node::True => (api.cJSON_CreateTrue)(),
        Node::False => (api.cJSON_CreateFalse)(),
        Node::Bool(b) => (api.cJSON_CreateBool)(*b),
        Node::Number(d) => (api.cJSON_CreateNumber)(*d),
        Node::Str(s) => {
            let c = cbytes(s.as_bytes());
            (api.cJSON_CreateString)(c.as_ptr() as *const c_char)
        }
        Node::Raw(s) => {
            let c = cbytes(s.as_bytes());
            (api.cJSON_CreateRaw)(c.as_ptr() as *const c_char)
        }
        Node::StrRef(s) => (api.cJSON_CreateStringReference)(leak_cstr(s)),
        Node::IntArray(v) => (api.cJSON_CreateIntArray)(v.as_ptr(), v.len() as c_int),
        Node::FloatArray(v) => (api.cJSON_CreateFloatArray)(v.as_ptr(), v.len() as c_int),
        Node::DoubleArray(v) => (api.cJSON_CreateDoubleArray)(v.as_ptr(), v.len() as c_int),
        Node::StringArray(v) => {
            let owned: Vec<Vec<u8>> = v.iter().map(|s| cbytes(s.as_bytes())).collect();
            let ptrs: Vec<*const c_char> =
                owned.iter().map(|b| b.as_ptr() as *const c_char).collect();
            (api.cJSON_CreateStringArray)(ptrs.as_ptr(), ptrs.len() as c_int)
        }
        Node::Array(items) => {
            let a = (api.cJSON_CreateArray)();
            for it in items {
                let child = build(api, it);
                (api.cJSON_AddItemToArray)(a, child);
            }
            a
        }
        Node::Object(kv) => {
            let o = (api.cJSON_CreateObject)();
            for (k, v) in kv {
                let child = build(api, v);
                let key = cbytes(k.as_bytes());
                (api.cJSON_AddItemToObject)(o, key.as_ptr() as *const c_char, child);
            }
            o
        }
        Node::ObjectCS(kv) => {
            let o = (api.cJSON_CreateObject)();
            for (k, v) in kv {
                let child = build(api, v);
                (api.cJSON_AddItemToObjectCS)(o, leak_cstr(k), child);
            }
            o
        }
        Node::Retyped(inner, t) => {
            let it = build(api, inner);
            if !it.is_null() {
                (*it).type_ = *t;
            }
            it
        }
    }
}

/// Random tree; `depth` bounds nesting.
pub fn random_node(rng: &mut Rng, depth: u32) -> Node {
    let leafy = depth == 0;
    let k = if leafy { rng.below(12) } else { rng.below(15) };
    match k {
        0 => Node::Null,
        1 => Node::True,
        2 => Node::False,
        3 => Node::Bool(rng.i32()),
        4 => Node::Number(rng.f64()),
        5 => Node::Str(rng.string(12)),
        6 => Node::StrRef(rng.string(8)),
        7 => {
            let n = rng.below(5) as usize;
            Node::IntArray((0..n).map(|_| rng.i32()).collect())
        }
        8 => {
            let n = rng.below(5) as usize;
            Node::FloatArray((0..n).map(|_| rng.f32()).collect())
        }
        9 => {
            let n = rng.below(5) as usize;
            Node::DoubleArray((0..n).map(|_| rng.f64()).collect())
        }
        10 => {
            let n = rng.below(5) as usize;
            Node::StringArray((0..n).map(|_| rng.string(6)).collect())
        }
        11 => Node::Number(rng.f64()),
        12 => {
            let n = rng.below(5) as usize;
            Node::Array((0..n).map(|_| random_node(rng, depth - 1)).collect())
        }
        13 => {
            let n = rng.below(5) as usize;
            Node::Object(
                (0..n)
                    .map(|_| (rng.string(6), random_node(rng, depth - 1)))
                    .collect(),
            )
        }
        _ => {
            let n = rng.below(4) as usize;
            Node::ObjectCS(
                (0..n)
                    .map(|_| (rng.string(6), random_node(rng, depth - 1)))
                    .collect(),
            )
        }
    }
}

/// Random tree restricted to nodes that round-trip through JSON text
/// (no `Raw`, no retyped items).
pub fn random_json_node(rng: &mut Rng, depth: u32) -> Node {
    let n = random_node(rng, depth);
    n
}

/// Run `f` for both libraries and assert the two results are equal.
pub fn diff<T: PartialEq + std::fmt::Debug>(
    label: &str,
    c: &Api,
    r: &Api,
    f: impl Fn(&Api) -> T,
) -> T {
    let a = f(c);
    let b = f(r);
    assert_eq!(a, b, "divergence in {}: C={:?} RUST={:?}", label, a, b);
    a
}

// ------------------------------------------------------- global-state serializer

/// `global_error` and `global_hooks` are per-library process globals, so any
/// test that observes them must not run concurrently with another such test.
pub fn global_lock() -> std::sync::MutexGuard<'static, ()> {
    static M: std::sync::OnceLock<std::sync::Mutex<()>> = std::sync::OnceLock::new();
    M.get_or_init(|| std::sync::Mutex::new(()))
        .lock()
        .unwrap_or_else(|e| e.into_inner())
}
