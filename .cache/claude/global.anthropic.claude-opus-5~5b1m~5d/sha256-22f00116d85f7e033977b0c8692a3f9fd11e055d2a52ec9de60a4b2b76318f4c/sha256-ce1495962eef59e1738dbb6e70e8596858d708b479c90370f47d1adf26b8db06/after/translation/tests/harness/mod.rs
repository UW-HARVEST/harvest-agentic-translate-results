//! Shared differential-test harness.
//!
//! Loads BOTH the C `.so` (built by CMake in `c_src/build`) and the Rust
//! `.so` (`target/<profile>/libcJSON_test.so`) through `libloading` and exposes
//! every exported symbol behind an identical Rust-side signature.  No Rust
//! function is ever called directly — everything goes through the dynamic
//! symbol table, exactly like an external C consumer.
#![allow(dead_code)]
#![allow(non_snake_case)]
#![allow(non_camel_case_types)]
#![allow(non_upper_case_globals)]

use libloading::Library;
use std::ffi::{c_char, c_int, c_void, CString};
use std::path::{Path, PathBuf};

// ---------------------------------------------------------------------------
// C ABI types
// ---------------------------------------------------------------------------

pub type cJSON_bool = c_int;

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

/// `struct record` from `c_src/test.c`.
#[repr(C)]
pub struct Record {
    pub precision: *const c_char,
    pub lat: f64,
    pub lon: f64,
    pub address: *const c_char,
    pub city: *const c_char,
    pub state: *const c_char,
    pub zip: *const c_char,
    pub country: *const c_char,
}

// ---------------------------------------------------------------------------
// symbol table
// ---------------------------------------------------------------------------

macro_rules! declare_api {
    ( $( $name:ident : fn( $($arg:ty),* ) $(-> $ret:ty)? ; )* ) => {
        pub struct Api {
            /// Which side this is ("C" or "Rust"); used in assertion messages.
            pub tag: &'static str,
            #[allow(dead_code)]
            libs: Vec<Library>,
            $( pub $name: unsafe extern "C" fn( $($arg),* ) $(-> $ret)? , )*
        }

        impl Api {
            fn from_libs(tag: &'static str, libs: Vec<Library>) -> Api {
                unsafe {
                    Api {
                        tag,
                        $( $name: lookup(&libs, concat!(stringify!($name), "\0").as_bytes()), )*
                        libs,
                    }
                }
            }
        }
    };
}

unsafe fn lookup<T: Copy>(libs: &[Library], name: &[u8]) -> T {
    for l in libs {
        if let Ok(s) = l.get::<T>(name) {
            return *s;
        }
    }
    panic!(
        "symbol not found in any loaded library: {}",
        String::from_utf8_lossy(&name[..name.len() - 1])
    );
}

declare_api! {
    cJSON_Version: fn() -> *const c_char;
    cJSON_InitHooks: fn(*mut CJsonHooks);

    cJSON_Parse: fn(*const c_char) -> *mut CJson;
    cJSON_ParseWithLength: fn(*const c_char, usize) -> *mut CJson;
    cJSON_ParseWithOpts: fn(*const c_char, *mut *const c_char, cJSON_bool) -> *mut CJson;
    cJSON_ParseWithLengthOpts: fn(*const c_char, usize, *mut *const c_char, cJSON_bool) -> *mut CJson;

    cJSON_Print: fn(*const CJson) -> *mut c_char;
    cJSON_PrintUnformatted: fn(*const CJson) -> *mut c_char;
    cJSON_PrintBuffered: fn(*const CJson, c_int, cJSON_bool) -> *mut c_char;
    cJSON_PrintPreallocated: fn(*mut CJson, *mut c_char, c_int, cJSON_bool) -> cJSON_bool;
    cJSON_Delete: fn(*mut CJson);

    cJSON_GetArraySize: fn(*const CJson) -> c_int;
    cJSON_GetArrayItem: fn(*const CJson, c_int) -> *mut CJson;
    cJSON_GetObjectItem: fn(*const CJson, *const c_char) -> *mut CJson;
    cJSON_GetObjectItemCaseSensitive: fn(*const CJson, *const c_char) -> *mut CJson;
    cJSON_HasObjectItem: fn(*const CJson, *const c_char) -> cJSON_bool;
    cJSON_GetErrorPtr: fn() -> *const c_char;

    cJSON_GetStringValue: fn(*const CJson) -> *mut c_char;
    cJSON_GetNumberValue: fn(*const CJson) -> f64;

    cJSON_IsInvalid: fn(*const CJson) -> cJSON_bool;
    cJSON_IsFalse: fn(*const CJson) -> cJSON_bool;
    cJSON_IsTrue: fn(*const CJson) -> cJSON_bool;
    cJSON_IsBool: fn(*const CJson) -> cJSON_bool;
    cJSON_IsNull: fn(*const CJson) -> cJSON_bool;
    cJSON_IsNumber: fn(*const CJson) -> cJSON_bool;
    cJSON_IsString: fn(*const CJson) -> cJSON_bool;
    cJSON_IsArray: fn(*const CJson) -> cJSON_bool;
    cJSON_IsObject: fn(*const CJson) -> cJSON_bool;
    cJSON_IsRaw: fn(*const CJson) -> cJSON_bool;

    cJSON_CreateNull: fn() -> *mut CJson;
    cJSON_CreateTrue: fn() -> *mut CJson;
    cJSON_CreateFalse: fn() -> *mut CJson;
    cJSON_CreateBool: fn(cJSON_bool) -> *mut CJson;
    cJSON_CreateNumber: fn(f64) -> *mut CJson;
    cJSON_CreateString: fn(*const c_char) -> *mut CJson;
    cJSON_CreateRaw: fn(*const c_char) -> *mut CJson;
    cJSON_CreateArray: fn() -> *mut CJson;
    cJSON_CreateObject: fn() -> *mut CJson;
    cJSON_CreateStringReference: fn(*const c_char) -> *mut CJson;
    cJSON_CreateObjectReference: fn(*const CJson) -> *mut CJson;
    cJSON_CreateArrayReference: fn(*const CJson) -> *mut CJson;
    cJSON_CreateIntArray: fn(*const c_int, c_int) -> *mut CJson;
    cJSON_CreateFloatArray: fn(*const f32, c_int) -> *mut CJson;
    cJSON_CreateDoubleArray: fn(*const f64, c_int) -> *mut CJson;
    cJSON_CreateStringArray: fn(*const *const c_char, c_int) -> *mut CJson;

    cJSON_AddItemToArray: fn(*mut CJson, *mut CJson) -> cJSON_bool;
    cJSON_AddItemToObject: fn(*mut CJson, *const c_char, *mut CJson) -> cJSON_bool;
    cJSON_AddItemToObjectCS: fn(*mut CJson, *const c_char, *mut CJson) -> cJSON_bool;
    cJSON_AddItemReferenceToArray: fn(*mut CJson, *mut CJson) -> cJSON_bool;
    cJSON_AddItemReferenceToObject: fn(*mut CJson, *const c_char, *mut CJson) -> cJSON_bool;

    cJSON_DetachItemViaPointer: fn(*mut CJson, *mut CJson) -> *mut CJson;
    cJSON_DetachItemFromArray: fn(*mut CJson, c_int) -> *mut CJson;
    cJSON_DeleteItemFromArray: fn(*mut CJson, c_int);
    cJSON_DetachItemFromObject: fn(*mut CJson, *const c_char) -> *mut CJson;
    cJSON_DetachItemFromObjectCaseSensitive: fn(*mut CJson, *const c_char) -> *mut CJson;
    cJSON_DeleteItemFromObject: fn(*mut CJson, *const c_char);
    cJSON_DeleteItemFromObjectCaseSensitive: fn(*mut CJson, *const c_char);

    cJSON_InsertItemInArray: fn(*mut CJson, c_int, *mut CJson) -> cJSON_bool;
    cJSON_ReplaceItemViaPointer: fn(*mut CJson, *mut CJson, *mut CJson) -> cJSON_bool;
    cJSON_ReplaceItemInArray: fn(*mut CJson, c_int, *mut CJson) -> cJSON_bool;
    cJSON_ReplaceItemInObject: fn(*mut CJson, *const c_char, *mut CJson) -> cJSON_bool;
    cJSON_ReplaceItemInObjectCaseSensitive: fn(*mut CJson, *const c_char, *mut CJson) -> cJSON_bool;

    cJSON_Duplicate: fn(*const CJson, cJSON_bool) -> *mut CJson;
    cJSON_Compare: fn(*const CJson, *const CJson, cJSON_bool) -> cJSON_bool;
    cJSON_Minify: fn(*mut c_char);

    cJSON_AddNullToObject: fn(*mut CJson, *const c_char) -> *mut CJson;
    cJSON_AddTrueToObject: fn(*mut CJson, *const c_char) -> *mut CJson;
    cJSON_AddFalseToObject: fn(*mut CJson, *const c_char) -> *mut CJson;
    cJSON_AddBoolToObject: fn(*mut CJson, *const c_char, cJSON_bool) -> *mut CJson;
    cJSON_AddNumberToObject: fn(*mut CJson, *const c_char, f64) -> *mut CJson;
    cJSON_AddStringToObject: fn(*mut CJson, *const c_char, *const c_char) -> *mut CJson;
    cJSON_AddRawToObject: fn(*mut CJson, *const c_char, *const c_char) -> *mut CJson;
    cJSON_AddObjectToObject: fn(*mut CJson, *const c_char) -> *mut CJson;
    cJSON_AddArrayToObject: fn(*mut CJson, *const c_char) -> *mut CJson;

    cJSON_SetNumberHelper: fn(*mut CJson, f64) -> f64;
    cJSON_SetValuestring: fn(*mut CJson, *const c_char) -> *mut c_char;

    cJSON_malloc: fn(usize) -> *mut c_void;
    cJSON_free: fn(*mut c_void);

    driver: fn(*const *const c_char, *const [c_int; 3], *const c_int, *const Record) -> c_int;
}

// ---------------------------------------------------------------------------
// library discovery
// ---------------------------------------------------------------------------

fn workspace_root() -> PathBuf {
    // CARGO_MANIFEST_DIR == <root>/translation
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest.parent().unwrap().to_path_buf()
}

fn must_exist(p: PathBuf) -> PathBuf {
    assert!(
        p.exists(),
        "required shared library is missing: {}\n\
         Build the C side with:\n\
           cd c_src && mkdir -p build && cd build && \\\n\
           cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .\n\
         Build the Rust side with: cd translation && cargo build --release",
        p.display()
    );
    p
}

fn rust_so_path() -> PathBuf {
    let root = workspace_root().join("translation").join("target");
    // Prefer the profile the test binary itself was built with, then fall back.
    let mut candidates: Vec<PathBuf> = Vec::new();
    if let Ok(exe) = std::env::current_exe() {
        // .../target/<profile>/deps/<test>
        if let Some(deps) = exe.parent() {
            if let Some(profile_dir) = deps.parent() {
                candidates.push(profile_dir.join("libcJSON_test.so"));
            }
        }
    }
    candidates.push(root.join("release").join("libcJSON_test.so"));
    candidates.push(root.join("debug").join("libcJSON_test.so"));
    for c in &candidates {
        if c.exists() {
            return c.clone();
        }
    }
    must_exist(candidates.pop().unwrap())
}

unsafe fn open(p: &Path) -> Library {
    Library::new(p).unwrap_or_else(|e| panic!("failed to dlopen {}: {e}", p.display()))
}

/// Loads the C reference implementation (`libcjson.so` + `libcJSON_test.so`).
pub fn load_c() -> Api {
    let build = workspace_root().join("c_src").join("build");
    let cjson = must_exist(build.join("libcjson.so"));
    let test = must_exist(build.join("libcJSON_test.so"));
    unsafe { Api::from_libs("C", vec![open(&cjson), open(&test)]) }
}

/// Loads the Rust translation (`libcJSON_test.so`).
pub fn load_rust() -> Api {
    let so = rust_so_path();
    unsafe { Api::from_libs("Rust", vec![open(&so)]) }
}

/// Convenience: both sides at once.
pub fn both() -> (Api, Api) {
    (load_c(), load_rust())
}

// ---------------------------------------------------------------------------
// small helpers
// ---------------------------------------------------------------------------

/// Reads a NUL-terminated C string into a `Vec<u8>` (no UTF-8 validation), or
/// `None` for a null pointer.
pub unsafe fn cstr(p: *const c_char) -> Option<Vec<u8>> {
    if p.is_null() {
        return None;
    }
    let mut out = Vec::new();
    let mut q = p as *const u8;
    while *q != 0 {
        out.push(*q);
        q = q.add(1);
    }
    Some(out)
}

pub fn show(v: &Option<Vec<u8>>) -> String {
    match v {
        None => "<NULL>".to_string(),
        Some(b) => format!("{:?}", String::from_utf8_lossy(b)),
    }
}

/// Owns a NUL-terminated byte buffer usable as `*const c_char`.
pub fn cs(s: &str) -> CString {
    CString::new(s).unwrap()
}

/// Owns raw bytes plus a trailing NUL (allows embedded non-UTF8/control bytes,
/// but not embedded NULs).
pub struct Bytes(pub Vec<u8>);

impl Bytes {
    pub fn new(b: &[u8]) -> Bytes {
        let mut v = b.to_vec();
        v.push(0);
        Bytes(v)
    }
    pub fn as_ptr(&self) -> *const c_char {
        self.0.as_ptr() as *const c_char
    }
    pub fn as_mut_ptr(&mut self) -> *mut c_char {
        self.0.as_mut_ptr() as *mut c_char
    }
}

/// Snapshot of the observable scalar fields of a `cJSON` node, plus the
/// contents (not addresses) of its two strings and the shape of its children.
#[derive(Debug, PartialEq)]
pub struct NodeSnap {
    pub type_: c_int,
    pub valueint: c_int,
    pub valuedouble_bits: u64,
    pub valuestring: Option<Vec<u8>>,
    pub string: Option<Vec<u8>>,
    pub children: Vec<NodeSnap>,
    /// `prev` of the first child equals the last child (cJSON's circular
    /// back-pointer trick) — recorded as a bool so addresses never leak in.
    pub child_prev_is_last: Option<bool>,
    pub has_child: bool,
}

pub unsafe fn snap(item: *const CJson) -> Option<NodeSnap> {
    snap_depth(item, 0)
}

unsafe fn snap_depth(item: *const CJson, depth: usize) -> Option<NodeSnap> {
    if item.is_null() {
        return None;
    }
    let it = &*item;
    let mut children = Vec::new();
    let mut child_prev_is_last = None;
    // Guard against cJSON_*Reference cycles and absurd depth.
    if depth < 64 && (it.type_ & cJSON_IsReference) == 0 {
        let mut c = it.child;
        let mut last: *mut CJson = std::ptr::null_mut();
        let mut n = 0;
        while !c.is_null() && n < 100_000 {
            children.push(snap_depth(c, depth + 1).unwrap());
            last = c;
            c = (*c).next;
            n += 1;
        }
        if !it.child.is_null() {
            child_prev_is_last = Some((*it.child).prev == last);
        }
    }
    Some(NodeSnap {
        type_: it.type_,
        valueint: it.valueint,
        valuedouble_bits: it.valuedouble.to_bits(),
        valuestring: cstr(it.valuestring),
        string: cstr(it.string),
        children,
        child_prev_is_last,
        has_child: !it.child.is_null(),
    })
}

/// Prints with `cJSON_Print` and returns the bytes, freeing the buffer with the
/// same library's `cJSON_free`.
pub unsafe fn print_and_take(api: &Api, item: *const CJson) -> Option<Vec<u8>> {
    let p = (api.cJSON_Print)(item);
    let r = cstr(p);
    if !p.is_null() {
        (api.cJSON_free)(p as *mut c_void);
    }
    r
}

pub unsafe fn print_unformatted_and_take(api: &Api, item: *const CJson) -> Option<Vec<u8>> {
    let p = (api.cJSON_PrintUnformatted)(item);
    let r = cstr(p);
    if !p.is_null() {
        (api.cJSON_free)(p as *mut c_void);
    }
    r
}

pub unsafe fn print_buffered_and_take(
    api: &Api,
    item: *const CJson,
    prebuffer: c_int,
    fmt: cJSON_bool,
) -> Option<Vec<u8>> {
    let p = (api.cJSON_PrintBuffered)(item, prebuffer, fmt);
    let r = cstr(p);
    if !p.is_null() {
        (api.cJSON_free)(p as *mut c_void);
    }
    r
}

// ---------------------------------------------------------------------------
// deterministic PRNG (xorshift64*) — fixed seed, reproducible
// ---------------------------------------------------------------------------

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng(if seed == 0 { 0x9E3779B97F4A7C15 } else { seed })
    }
    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545F4914F6CDD1D)
    }
    pub fn below(&mut self, n: usize) -> usize {
        assert!(n > 0);
        (self.next_u64() % (n as u64)) as usize
    }
    pub fn range_i32(&mut self, lo: i32, hi: i32) -> i32 {
        // inclusive lo, exclusive hi
        let span = (hi as i64 - lo as i64) as u64;
        (lo as i64 + (self.next_u64() % span) as i64) as i32
    }
    pub fn bool(&mut self) -> bool {
        self.next_u64() & 1 == 1
    }
    /// Uniform bit-pattern double, including NaNs/infinities/denormals.
    pub fn any_f64(&mut self) -> f64 {
        f64::from_bits(self.next_u64())
    }
    /// "Realistic" doubles biased towards values JSON actually contains.
    pub fn json_f64(&mut self) -> f64 {
        match self.below(10) {
            0 => 0.0,
            1 => -0.0,
            2 => self.range_i32(-1000, 1000) as f64,
            3 => self.range_i32(i32::MIN, i32::MAX) as f64,
            4 => self.next_u64() as f64,
            5 => (self.next_u64() as f64) / (1u64 << 32) as f64,
            6 => f64::from_bits(self.next_u64() & 0x7FEF_FFFF_FFFF_FFFF),
            7 => {
                let m = (self.next_u64() % 1_000_000_000) as f64;
                m / 1e9
            }
            8 => {
                let e = self.range_i32(-320, 320);
                (self.range_i32(1, 1000) as f64) * 10f64.powi(e)
            }
            _ => self.any_f64(),
        }
    }
}

// ---------------------------------------------------------------------------
// stdout capture (for the `driver` entry point, which uses printf)
// ---------------------------------------------------------------------------

extern "C" {
    fn dup(oldfd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
    #[link_name = "open"]
    fn libc_open(path: *const c_char, flags: c_int, ...) -> c_int;
    fn lseek(fd: c_int, off: i64, whence: c_int) -> i64;
    fn read(fd: c_int, buf: *mut c_void, count: usize) -> isize;
}

/// Runs `f` with fd 1 redirected into a temporary file and returns everything
/// written to it.  Both implementations print through the platform `printf`, so
/// this captures the C and Rust output identically.
pub unsafe fn capture_stdout<F: FnOnce()>(f: F) -> Vec<u8> {
    const O_RDWR: c_int = 2;
    const O_CREAT: c_int = 64;
    const O_TRUNC: c_int = 512;

    // fd 1 is process-wide: serialise so parallel tests cannot interleave.
    let _guard = STDOUT_LOCK.lock().unwrap_or_else(|e| e.into_inner());

    let dir = std::env::var("TMPDIR").unwrap_or_else(|_| "/tmp".to_string());
    let path = format!(
        "{}/cjson_capture_{}_{}.txt",
        dir,
        std::process::id(),
        COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst)
    );
    let cpath = CString::new(path.clone()).unwrap();

    fflush(std::ptr::null_mut());
    let saved = dup(1);
    assert!(saved >= 0, "dup(1) failed");
    let tmp = libc_open(cpath.as_ptr(), O_RDWR | O_CREAT | O_TRUNC, 0o600 as c_int);
    assert!(tmp >= 0, "open({path}) failed");
    assert!(dup2(tmp, 1) >= 0, "dup2 failed");

    f();

    fflush(std::ptr::null_mut());
    dup2(saved, 1);
    close(saved);

    lseek(tmp, 0, 0);
    let mut out = Vec::new();
    let mut buf = [0u8; 65536];
    loop {
        let n = read(tmp, buf.as_mut_ptr() as *mut c_void, buf.len());
        if n <= 0 {
            break;
        }
        out.extend_from_slice(&buf[..n as usize]);
    }
    close(tmp);
    let _ = std::fs::remove_file(&path);
    out
}

static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
static STDOUT_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

// ---------------------------------------------------------------------------
// declarative tree specs — the same spec is materialised through BOTH `.so`s
// using only their exported constructors, so the two trees are built by the
// exact same sequence of public calls.
// ---------------------------------------------------------------------------

#[derive(Clone, Debug)]
pub enum Spec {
    Null,
    True,
    False,
    Bool(c_int),
    Num(f64),
    Str(Vec<u8>),
    Raw(Vec<u8>),
    /// `cJSON_CreateStringReference` — `valuestring` aliases the caller's buffer.
    StrRef(Vec<u8>),
    /// `cJSON_CreateArray` + `cJSON_AddItemToArray` for each element.
    Arr(Vec<Spec>),
    /// `cJSON_CreateObject` + `cJSON_AddItemToObject`.
    Obj(Vec<(Vec<u8>, Spec)>),
    /// `cJSON_CreateObject` + `cJSON_AddItemToObjectCS` (constant keys).
    ObjCS(Vec<(Vec<u8>, Spec)>),
    IntArr(Vec<c_int>),
    FloatArr(Vec<f32>),
    DoubleArr(Vec<f64>),
    StrArr(Vec<Vec<u8>>),
    /// `cJSON_CreateObject` then `cJSON_AddArrayToObject`/`AddObjectToObject`/
    /// `AddNullToObject`/… i.e. the convenience helpers rather than
    /// `cJSON_AddItemToObject`.
    ObjViaHelpers(Vec<(Vec<u8>, Helper)>),
    /// An array holding `cJSON_AddItemReferenceToArray` references to the
    /// elements of an inner array (also kept alive as element 0).
    ArrWithRefs(Box<Spec>),
    /// Object holding `cJSON_AddItemReferenceToObject` references.
    ObjWithRefs(Box<Spec>),
    /// `cJSON_CreateArrayReference` / `cJSON_CreateObjectReference` over a
    /// container that is itself element 0 of the surrounding array.
    ContainerRef(Box<Spec>, bool),
}

#[derive(Clone, Debug)]
pub enum Helper {
    Null,
    True,
    False,
    Bool(c_int),
    Num(f64),
    Str(Vec<u8>),
    Raw(Vec<u8>),
    Object,
    Array,
}

/// A materialised tree plus the byte buffers it borrows.
pub struct Built<'a> {
    pub root: *mut CJson,
    pub arena: Vec<Bytes>,
    api: &'a Api,
    pub deletable: bool,
}

impl<'a> Built<'a> {
    pub fn delete(mut self) {
        if self.deletable && !self.root.is_null() {
            unsafe { (self.api.cJSON_Delete)(self.root) };
        }
        self.root = std::ptr::null_mut();
        self.deletable = false;
    }
}

impl<'a> Drop for Built<'a> {
    fn drop(&mut self) {
        if self.deletable && !self.root.is_null() {
            unsafe { (self.api.cJSON_Delete)(self.root) };
        }
    }
}

pub unsafe fn build<'a>(api: &'a Api, spec: &Spec) -> Built<'a> {
    let mut arena: Vec<Bytes> = Vec::new();
    let root = build_into(api, spec, &mut arena);
    Built {
        root,
        arena,
        api,
        deletable: true,
    }
}

unsafe fn build_into(api: &Api, spec: &Spec, arena: &mut Vec<Bytes>) -> *mut CJson {
    macro_rules! bytes {
        ($v:expr) => {{
            arena.push(Bytes::new($v));
            arena.last().unwrap().as_ptr()
        }};
    }
    match spec {
        Spec::Null => (api.cJSON_CreateNull)(),
        Spec::True => (api.cJSON_CreateTrue)(),
        Spec::False => (api.cJSON_CreateFalse)(),
        Spec::Bool(b) => (api.cJSON_CreateBool)(*b),
        Spec::Num(d) => (api.cJSON_CreateNumber)(*d),
        Spec::Str(s) => (api.cJSON_CreateString)(bytes!(s)),
        Spec::Raw(s) => (api.cJSON_CreateRaw)(bytes!(s)),
        Spec::StrRef(s) => (api.cJSON_CreateStringReference)(bytes!(s)),
        Spec::Arr(items) => {
            let a = (api.cJSON_CreateArray)();
            for it in items {
                let c = build_into(api, it, arena);
                (api.cJSON_AddItemToArray)(a, c);
            }
            a
        }
        Spec::Obj(kv) => {
            let o = (api.cJSON_CreateObject)();
            for (k, v) in kv {
                let c = build_into(api, v, arena);
                (api.cJSON_AddItemToObject)(o, bytes!(k), c);
            }
            o
        }
        Spec::ObjCS(kv) => {
            let o = (api.cJSON_CreateObject)();
            for (k, v) in kv {
                let c = build_into(api, v, arena);
                (api.cJSON_AddItemToObjectCS)(o, bytes!(k), c);
            }
            o
        }
        Spec::IntArr(v) => (api.cJSON_CreateIntArray)(v.as_ptr(), v.len() as c_int),
        Spec::FloatArr(v) => (api.cJSON_CreateFloatArray)(v.as_ptr(), v.len() as c_int),
        Spec::DoubleArr(v) => (api.cJSON_CreateDoubleArray)(v.as_ptr(), v.len() as c_int),
        Spec::StrArr(v) => {
            let mut ptrs: Vec<*const c_char> = Vec::new();
            for s in v {
                ptrs.push(bytes!(s));
            }
            (api.cJSON_CreateStringArray)(ptrs.as_ptr(), v.len() as c_int)
        }
        Spec::ObjViaHelpers(kv) => {
            let o = (api.cJSON_CreateObject)();
            for (k, h) in kv {
                let kp = bytes!(k);
                match h {
                    Helper::Null => (api.cJSON_AddNullToObject)(o, kp),
                    Helper::True => (api.cJSON_AddTrueToObject)(o, kp),
                    Helper::False => (api.cJSON_AddFalseToObject)(o, kp),
                    Helper::Bool(b) => (api.cJSON_AddBoolToObject)(o, kp, *b),
                    Helper::Num(d) => (api.cJSON_AddNumberToObject)(o, kp, *d),
                    Helper::Str(s) => {
                        let sp = bytes!(s);
                        (api.cJSON_AddStringToObject)(o, kp, sp)
                    }
                    Helper::Raw(s) => {
                        let sp = bytes!(s);
                        (api.cJSON_AddRawToObject)(o, kp, sp)
                    }
                    Helper::Object => (api.cJSON_AddObjectToObject)(o, kp),
                    Helper::Array => (api.cJSON_AddArrayToObject)(o, kp),
                };
            }
            o
        }
        Spec::ArrWithRefs(inner) => {
            let outer = (api.cJSON_CreateArray)();
            let src = build_into(api, inner, arena);
            (api.cJSON_AddItemToArray)(outer, src);
            let mut c = (*src).child;
            while !c.is_null() {
                (api.cJSON_AddItemReferenceToArray)(outer, c);
                c = (*c).next;
            }
            outer
        }
        Spec::ObjWithRefs(inner) => {
            let outer = (api.cJSON_CreateObject)();
            let src = build_into(api, inner, arena);
            (api.cJSON_AddItemToObject)(outer, bytes!(b"src"), src);
            let mut c = (*src).child;
            let mut i = 0;
            while !c.is_null() {
                let k = format!("ref{i}").into_bytes();
                (api.cJSON_AddItemReferenceToObject)(outer, bytes!(&k), c);
                c = (*c).next;
                i += 1;
            }
            outer
        }
        Spec::ContainerRef(inner, as_object) => {
            let outer = (api.cJSON_CreateArray)();
            let src = build_into(api, inner, arena);
            (api.cJSON_AddItemToArray)(outer, src);
            let r = if *as_object {
                (api.cJSON_CreateObjectReference)((*src).child)
            } else {
                (api.cJSON_CreateArrayReference)((*src).child)
            };
            (api.cJSON_AddItemToArray)(outer, r);
            outer
        }
    }
}

// ---------------------------------------------------------------------------
// random spec generation
// ---------------------------------------------------------------------------

/// Byte-string pool covering every branch of `print_string_ptr` / `parse_string`.
pub fn string_pool() -> Vec<Vec<u8>> {
    let mut v: Vec<Vec<u8>> = vec![
        b"".to_vec(),
        b"a".to_vec(),
        b"hello world".to_vec(),
        b"\"".to_vec(),
        b"\\".to_vec(),
        b"/".to_vec(),
        b"\x08".to_vec(),
        b"\x0c".to_vec(),
        b"\n".to_vec(),
        b"\r".to_vec(),
        b"\t".to_vec(),
        b"\x01\x02\x03\x1e\x1f".to_vec(),
        b"\x7f".to_vec(),
        b"\x80\xff".to_vec(),
        "\u{e9}\u{4e2d}\u{1F600}".as_bytes().to_vec(),
        b"mixed\t\"quote\\and\nnewline\x01".to_vec(),
        b"0123456789012345678901234567890123456789012345678901234567890123".to_vec(),
        b"    ".to_vec(),
        b"{}[],:".to_vec(),
        b"null".to_vec(),
        b"true".to_vec(),
        b"1e5".to_vec(),
    ];
    // every single byte 1..=255 (0 cannot appear in a C string)
    for b in 1u16..=255 {
        v.push(vec![b as u8]);
    }
    v
}

/// Numbers hitting every `print_number` branch.
pub fn number_pool() -> Vec<f64> {
    vec![
        0.0,
        -0.0,
        1.0,
        -1.0,
        24.0,
        1920.0,
        1e15,
        1e16,
        1e17,
        0.1,
        1.0 / 3.0,
        2.0 / 3.0,
        1.7976931348623157e308,
        5e-324,
        2.2250738585072014e-308,
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::NAN,
        -f64::NAN,
        i32::MAX as f64,
        i32::MAX as f64 - 1.0,
        i32::MAX as f64 + 1.0,
        i32::MIN as f64,
        i32::MIN as f64 + 1.0,
        i32::MIN as f64 - 1.0,
        2147483646.5,
        -2147483648.5,
        123456789012345.0,
        1234567890123456.0,
        0.30000000000000004,
        3.141592653589793,
        1e-7,
        1e21,
        1e22,
        9007199254740993.0,
        4.9406564584124654e-324,
    ]
}

pub fn rand_string(rng: &mut Rng, pool: &[Vec<u8>]) -> Vec<u8> {
    if rng.below(4) == 0 {
        // synthesise a random byte string (no NULs)
        let n = rng.below(24);
        (0..n).map(|_| (rng.below(255) + 1) as u8).collect()
    } else {
        pool[rng.below(pool.len())].clone()
    }
}

pub fn rand_spec(rng: &mut Rng, depth: usize) -> Spec {
    let pool = string_pool();
    rand_spec_with(rng, depth, &pool)
}

pub fn rand_spec_with(rng: &mut Rng, depth: usize, pool: &[Vec<u8>]) -> Spec {
    let leaf_only = depth == 0;
    let n = if leaf_only { 12 } else { 20 };
    match rng.below(n) {
        0 => Spec::Null,
        1 => Spec::True,
        2 => Spec::False,
        3 => Spec::Bool(rng.range_i32(-3, 4)),
        4 | 5 => {
            let np = number_pool();
            if rng.bool() {
                Spec::Num(np[rng.below(np.len())])
            } else {
                Spec::Num(rng.json_f64())
            }
        }
        6 | 7 => Spec::Str(rand_string(rng, pool)),
        8 => Spec::Raw(rand_string(rng, pool)),
        9 => Spec::StrRef(rand_string(rng, pool)),
        10 => {
            let k = rng.below(6);
            Spec::IntArr((0..k).map(|_| rng.range_i32(i32::MIN, i32::MAX)).collect())
        }
        11 => {
            let k = rng.below(6);
            Spec::DoubleArr((0..k).map(|_| rng.json_f64()).collect())
        }
        12 => {
            let k = rng.below(6);
            Spec::FloatArr(
                (0..k)
                    .map(|_| f32::from_bits(rng.next_u64() as u32))
                    .collect(),
            )
        }
        13 => {
            let k = rng.below(5);
            Spec::StrArr((0..k).map(|_| rand_string(rng, pool)).collect())
        }
        14 | 15 => {
            let k = rng.below(5);
            Spec::Arr((0..k).map(|_| rand_spec_with(rng, depth - 1, pool)).collect())
        }
        16 | 17 => {
            let k = rng.below(5);
            Spec::Obj(
                (0..k)
                    .map(|_| (rand_string(rng, pool), rand_spec_with(rng, depth - 1, pool)))
                    .collect(),
            )
        }
        18 => {
            let k = rng.below(5);
            Spec::ObjCS(
                (0..k)
                    .map(|_| (rand_string(rng, pool), rand_spec_with(rng, depth - 1, pool)))
                    .collect(),
            )
        }
        _ => {
            let k = rng.below(4);
            Spec::ObjViaHelpers(
                (0..k)
                    .map(|_| {
                        let h = match rng.below(9) {
                            0 => Helper::Null,
                            1 => Helper::True,
                            2 => Helper::False,
                            3 => Helper::Bool(rng.range_i32(-2, 3)),
                            4 => Helper::Num(rng.json_f64()),
                            5 => Helper::Str(rand_string(rng, pool)),
                            6 => Helper::Raw(rand_string(rng, pool)),
                            7 => Helper::Object,
                            _ => Helper::Array,
                        };
                        (rand_string(rng, pool), h)
                    })
                    .collect(),
            )
        }
    }
}

// ---------------------------------------------------------------------------
// full differential comparison of a materialised tree
// ---------------------------------------------------------------------------

/// The complete externally-observable state of a tree, gathered through the
/// public API only.
#[derive(Debug, PartialEq)]
pub struct TreeObs {
    pub snapshot: Option<NodeSnap>,
    pub printed: Option<Vec<u8>>,
    pub unformatted: Option<Vec<u8>>,
    pub buffered: Vec<Option<Vec<u8>>>,
    /// (return value, bytes of the buffer after the call) for each length probed
    pub preallocated: Vec<(cJSON_bool, Vec<u8>)>,
    pub array_size: c_int,
    pub predicates: [cJSON_bool; 10],
    pub string_value: Option<Vec<u8>>,
    pub number_value_bits: u64,
}

pub const PREBUFFERS: [c_int; 10] = [0, 1, 2, 3, 8, 64, 255, 256, 257, 4096];

pub unsafe fn observe(api: &Api, root: *mut CJson) -> TreeObs {
    let printed = print_and_take(api, root);
    let unformatted = print_unformatted_and_take(api, root);

    let mut buffered = Vec::new();
    for pb in PREBUFFERS {
        buffered.push(print_buffered_and_take(api, root, pb, 1));
        buffered.push(print_buffered_and_take(api, root, pb, 0));
    }

    // Probe cJSON_PrintPreallocated around the exact required size for both
    // formats.  The buffer is pre-filled with a marker so that partial writes
    // are visible too.
    let mut preallocated = Vec::new();
    for (fmt, want) in [(1, printed.as_ref()), (0, unformatted.as_ref())] {
        let exact = want.map(|v| v.len()).unwrap_or(0);
        for len in [
            0usize,
            1,
            2,
            exact.saturating_sub(2),
            exact.saturating_sub(1),
            exact,
            exact + 1,
            exact + 2,
            exact + 6,
        ] {
            let cap = len.max(1) + 64;
            let mut buf = vec![0xAAu8; cap];
            let rc = (api.cJSON_PrintPreallocated)(
                root,
                buf.as_mut_ptr() as *mut c_char,
                len as c_int,
                fmt,
            );
            buf.truncate(len.min(cap));
            preallocated.push((rc, buf));
        }
    }

    let predicates = [
        (api.cJSON_IsInvalid)(root),
        (api.cJSON_IsFalse)(root),
        (api.cJSON_IsTrue)(root),
        (api.cJSON_IsBool)(root),
        (api.cJSON_IsNull)(root),
        (api.cJSON_IsNumber)(root),
        (api.cJSON_IsString)(root),
        (api.cJSON_IsArray)(root),
        (api.cJSON_IsObject)(root),
        (api.cJSON_IsRaw)(root),
    ];

    TreeObs {
        snapshot: snap(root),
        printed,
        unformatted,
        buffered,
        preallocated,
        array_size: (api.cJSON_GetArraySize)(root),
        predicates,
        string_value: cstr((api.cJSON_GetStringValue)(root)),
        number_value_bits: (api.cJSON_GetNumberValue)(root).to_bits(),
    }
}

/// Builds `spec` on both sides, compares every observable, then deletes both.
pub fn assert_spec_matches(c: &Api, r: &Api, spec: &Spec, ctx: &str) {
    unsafe {
        let bc = build(c, spec);
        let br = build(r, spec);
        let oc = observe(c, bc.root);
        let or = observe(r, br.root);
        assert_obs_eq(&oc, &or, ctx, spec);
        bc.delete();
        br.delete();
    }
}

pub fn assert_obs_eq(oc: &TreeObs, or: &TreeObs, ctx: &str, spec: &Spec) {
    if oc.snapshot != or.snapshot {
        panic!("{ctx}: node snapshot differs\nspec = {spec:?}\nC    = {:#?}\nRust = {:#?}", oc.snapshot, or.snapshot);
    }
    if oc.printed != or.printed {
        panic!(
            "{ctx}: cJSON_Print differs\nspec = {spec:?}\nC    = {}\nRust = {}",
            show(&oc.printed),
            show(&or.printed)
        );
    }
    if oc.unformatted != or.unformatted {
        panic!(
            "{ctx}: cJSON_PrintUnformatted differs\nspec = {spec:?}\nC    = {}\nRust = {}",
            show(&oc.unformatted),
            show(&or.unformatted)
        );
    }
    for (i, (a, b)) in oc.buffered.iter().zip(or.buffered.iter()).enumerate() {
        if a != b {
            let pb = PREBUFFERS[i / 2];
            let fmt = if i % 2 == 0 { 1 } else { 0 };
            panic!(
                "{ctx}: cJSON_PrintBuffered(prebuffer={pb}, fmt={fmt}) differs\nspec = {spec:?}\nC    = {}\nRust = {}",
                show(a),
                show(b)
            );
        }
    }
    for (i, (a, b)) in oc
        .preallocated
        .iter()
        .zip(or.preallocated.iter())
        .enumerate()
    {
        if a != b {
            panic!(
                "{ctx}: cJSON_PrintPreallocated probe #{i} differs\nspec = {spec:?}\n\
                 C    = rc={} buf={:?}\nRust = rc={} buf={:?}",
                a.0,
                String::from_utf8_lossy(&a.1),
                b.0,
                String::from_utf8_lossy(&b.1)
            );
        }
    }
    assert_eq!(
        oc.array_size, or.array_size,
        "{ctx}: cJSON_GetArraySize differs (spec = {spec:?})"
    );
    assert_eq!(
        oc.predicates, or.predicates,
        "{ctx}: cJSON_Is* predicates differ (spec = {spec:?})"
    );
    assert_eq!(
        oc.string_value, or.string_value,
        "{ctx}: cJSON_GetStringValue differs (spec = {spec:?})"
    );
    assert_eq!(
        oc.number_value_bits, or.number_value_bits,
        "{ctx}: cJSON_GetNumberValue differs (spec = {spec:?})"
    );
}

// ---------------------------------------------------------------------------
// process-wide library state
// ---------------------------------------------------------------------------

/// `cJSON.c` keeps two pieces of mutable global state: `global_error` (read
/// back by `cJSON_GetErrorPtr`) and `global_hooks` (set by `cJSON_InitHooks`).
/// Tests that observe either must hold this lock for the whole
/// call-C-then-call-Rust sequence, otherwise parallel tests clobber each other.
pub static GLOBAL_STATE: std::sync::Mutex<()> = std::sync::Mutex::new(());

pub fn lock_global_state() -> std::sync::MutexGuard<'static, ()> {
    GLOBAL_STATE.lock().unwrap_or_else(|e| e.into_inner())
}
