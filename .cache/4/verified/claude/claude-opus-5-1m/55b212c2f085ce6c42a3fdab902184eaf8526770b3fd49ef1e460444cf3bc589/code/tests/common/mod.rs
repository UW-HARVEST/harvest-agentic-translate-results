//! Differential-test harness.
//!
//! Loads BOTH the original C `libjansson.so` and the translated Rust
//! `libjansson.so` through `libloading` and exposes every exported symbol as a
//! typed function pointer.  Tests never call Rust functions directly: they only
//! ever go through the `.so` exports, exactly like an external C consumer.

#![allow(dead_code)]
#![allow(non_snake_case)]
#![allow(non_camel_case_types)]

use std::ffi::{CStr, CString};
use std::fmt::Write as _;
use std::os::raw::{c_char, c_int, c_longlong, c_void};
use std::path::PathBuf;

/* ------------------------------------------------------------------ types */

pub const JSON_OBJECT: c_int = 0;
pub const JSON_ARRAY: c_int = 1;
pub const JSON_STRING: c_int = 2;
pub const JSON_INTEGER: c_int = 3;
pub const JSON_REAL: c_int = 4;
pub const JSON_TRUE: c_int = 5;
pub const JSON_FALSE: c_int = 6;
pub const JSON_NULL: c_int = 7;

/* decoding flags */
pub const JSON_REJECT_DUPLICATES: usize = 0x1;
pub const JSON_DISABLE_EOF_CHECK: usize = 0x2;
pub const JSON_DECODE_ANY: usize = 0x4;
pub const JSON_DECODE_INT_AS_REAL: usize = 0x8;
pub const JSON_ALLOW_NUL: usize = 0x10;

/* encoding flags */
pub const JSON_MAX_INDENT: usize = 0x1F;
pub const JSON_COMPACT: usize = 0x20;
pub const JSON_ENSURE_ASCII: usize = 0x40;
pub const JSON_SORT_KEYS: usize = 0x80;
pub const JSON_PRESERVE_ORDER: usize = 0x100;
pub const JSON_ENCODE_ANY: usize = 0x200;
pub const JSON_ESCAPE_SLASH: usize = 0x400;
pub const JSON_EMBED: usize = 0x10000;
pub fn json_indent(n: usize) -> usize {
    n & JSON_MAX_INDENT
}
pub fn json_real_precision(n: usize) -> usize {
    (n & 0x1F) << 11
}

/* pack/unpack flags */
pub const JSON_VALIDATE_ONLY: usize = 0x1;
pub const JSON_STRICT: usize = 0x2;

pub const JSON_ERROR_TEXT_LENGTH: usize = 160;
pub const JSON_ERROR_SOURCE_LENGTH: usize = 80;

#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Json {
    pub type_: c_int,
    pub refcount: usize,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct JsonError {
    pub line: c_int,
    pub column: c_int,
    pub position: c_int,
    pub source: [u8; JSON_ERROR_SOURCE_LENGTH],
    pub text: [u8; JSON_ERROR_TEXT_LENGTH],
}

impl JsonError {
    /// Fill with a recognisable pattern so that *every* byte the library does
    /// (or does not) write becomes observable.
    pub fn patterned() -> JsonError {
        JsonError {
            line: 0x5555_5555,
            column: 0x5555_5555,
            position: 0x5555_5555,
            source: [0x55; JSON_ERROR_SOURCE_LENGTH],
            text: [0x55; JSON_ERROR_TEXT_LENGTH],
        }
    }
    pub fn as_bytes(&self) -> &[u8] {
        unsafe {
            std::slice::from_raw_parts(
                self as *const JsonError as *const u8,
                std::mem::size_of::<JsonError>(),
            )
        }
    }
    pub fn code(&self) -> u8 {
        self.text[JSON_ERROR_TEXT_LENGTH - 1]
    }
    pub fn text_str(&self) -> String {
        cstr_lossy(&self.text)
    }
    pub fn source_str(&self) -> String {
        cstr_lossy(&self.source)
    }
}

fn cstr_lossy(b: &[u8]) -> String {
    let n = b.iter().position(|&c| c == 0).unwrap_or(b.len());
    String::from_utf8_lossy(&b[..n]).into_owned()
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct HList {
    pub prev: *mut HList,
    pub next: *mut HList,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct Hashtable {
    pub size: usize,
    pub buckets: *mut c_void,
    pub order: usize,
    pub list: HList,
    pub ordered_list: HList,
}

impl Hashtable {
    pub fn zeroed() -> Hashtable {
        unsafe { std::mem::zeroed() }
    }
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct Strbuffer {
    pub value: *mut c_char,
    pub length: usize,
    pub size: usize,
}

impl Strbuffer {
    pub fn zeroed() -> Strbuffer {
        unsafe { std::mem::zeroed() }
    }
}

/// `typedef union { double d; ULong L[2]; } U;` from dtoa.c
#[repr(C)]
#[derive(Copy, Clone)]
pub union U {
    pub d: f64,
    pub L: [u32; 2],
}

pub type JsonMalloc = Option<unsafe extern "C" fn(usize) -> *mut c_void>;
pub type JsonRealloc = Option<unsafe extern "C" fn(*mut c_void, usize) -> *mut c_void>;
pub type JsonFree = Option<unsafe extern "C" fn(*mut c_void)>;
pub type DumpCallback = Option<unsafe extern "C" fn(*const c_char, usize, *mut c_void) -> c_int>;
pub type LoadCallback = Option<unsafe extern "C" fn(*mut c_void, usize, *mut c_void) -> usize>;

/* ------------------------------------------------------------------- loader */

macro_rules! api {
    ( $( $name:ident : $ty:ty , )* ) => {
        /// Every dynamic symbol of libjansson, resolved through `dlsym`.
        pub struct Api {
            pub tag: &'static str,
            $( pub $name : $ty , )*
            _lib: libloading::Library,
        }

        impl Api {
            pub fn load(path: &std::path::Path, tag: &'static str) -> Api {
                unsafe {
                    let lib = libloading::Library::new(path)
                        .unwrap_or_else(|e| panic!("dlopen {:?}: {}", path, e));
                    $(
                        let $name : $ty = *lib
                            .get(concat!(stringify!($name), "\0").as_bytes())
                            .unwrap_or_else(|e| panic!("dlsym {} in {:?}: {}",
                                                       stringify!($name), path, e));
                    )*
                    Api { tag, $( $name , )* _lib: lib }
                }
            }
        }
    }
}

api! {
    /* ---- version.c ---- */
    jansson_version_str: unsafe extern "C" fn() -> *const c_char,
    jansson_version_cmp: unsafe extern "C" fn(c_int, c_int, c_int) -> c_int,

    /* ---- memory.c ---- */
    jsonp_malloc: unsafe extern "C" fn(usize) -> *mut c_void,
    jsonp_realloc: unsafe extern "C" fn(*mut c_void, usize, usize) -> *mut c_void,
    jsonp_free: unsafe extern "C" fn(*mut c_void),
    jsonp_strndup: unsafe extern "C" fn(*const c_char, usize) -> *mut c_char,
    json_set_alloc_funcs: unsafe extern "C" fn(JsonMalloc, JsonFree),
    json_get_alloc_funcs: unsafe extern "C" fn(*mut JsonMalloc, *mut JsonFree),
    json_set_alloc_funcs2: unsafe extern "C" fn(JsonMalloc, JsonRealloc, JsonFree),
    json_get_alloc_funcs2: unsafe extern "C" fn(*mut JsonMalloc, *mut JsonRealloc, *mut JsonFree),

    /* ---- error.c ---- */
    jsonp_error_init: unsafe extern "C" fn(*mut JsonError, *const c_char),
    jsonp_error_set_source: unsafe extern "C" fn(*mut JsonError, *const c_char),
    jsonp_error_set: unsafe extern "C" fn(*mut JsonError, c_int, c_int, usize, c_int, *const c_char, ...),

    /* ---- strbuffer.c ---- */
    strbuffer_init: unsafe extern "C" fn(*mut Strbuffer) -> c_int,
    strbuffer_close: unsafe extern "C" fn(*mut Strbuffer),
    strbuffer_clear: unsafe extern "C" fn(*mut Strbuffer),
    strbuffer_value: unsafe extern "C" fn(*const Strbuffer) -> *const c_char,
    strbuffer_steal_value: unsafe extern "C" fn(*mut Strbuffer) -> *mut c_char,
    strbuffer_append_byte: unsafe extern "C" fn(*mut Strbuffer, c_char) -> c_int,
    strbuffer_append_bytes: unsafe extern "C" fn(*mut Strbuffer, *const c_char, usize) -> c_int,
    strbuffer_pop: unsafe extern "C" fn(*mut Strbuffer) -> c_char,

    /* ---- utf.c ---- */
    utf8_encode: unsafe extern "C" fn(i32, *mut c_char, *mut usize) -> c_int,
    utf8_check_first: unsafe extern "C" fn(c_char) -> usize,
    utf8_check_full: unsafe extern "C" fn(*const c_char, usize, *mut i32) -> usize,
    utf8_iterate: unsafe extern "C" fn(*const c_char, usize, *mut i32) -> *const c_char,
    utf8_check_string: unsafe extern "C" fn(*const c_char, usize) -> c_int,

    /* ---- hashtable.c / hashtable_seed.c ---- */
    hashtable_seed: *mut u32,
    json_object_seed: unsafe extern "C" fn(usize),
    hashtable_init: unsafe extern "C" fn(*mut Hashtable) -> c_int,
    hashtable_close: unsafe extern "C" fn(*mut Hashtable),
    hashtable_set: unsafe extern "C" fn(*mut Hashtable, *const c_char, usize, *mut Json) -> c_int,
    hashtable_get: unsafe extern "C" fn(*mut Hashtable, *const c_char, usize) -> *mut c_void,
    hashtable_del: unsafe extern "C" fn(*mut Hashtable, *const c_char, usize) -> c_int,
    hashtable_clear: unsafe extern "C" fn(*mut Hashtable),
    hashtable_iter: unsafe extern "C" fn(*mut Hashtable) -> *mut c_void,
    hashtable_iter_at: unsafe extern "C" fn(*mut Hashtable, *const c_char, usize) -> *mut c_void,
    hashtable_iter_next: unsafe extern "C" fn(*mut Hashtable, *mut c_void) -> *mut c_void,
    hashtable_iter_key: unsafe extern "C" fn(*mut c_void) -> *mut c_char,
    hashtable_iter_key_len: unsafe extern "C" fn(*mut c_void) -> usize,
    hashtable_iter_value: unsafe extern "C" fn(*mut c_void) -> *mut Json,
    hashtable_iter_set: unsafe extern "C" fn(*mut c_void, *mut Json),

    /* ---- strconv.c ---- */
    jsonp_strtod: unsafe extern "C" fn(*mut Strbuffer, *mut f64) -> c_int,
    jsonp_dtostr: unsafe extern "C" fn(*mut c_char, usize, f64, c_int) -> c_int,

    /* ---- dtoa.c ---- */
    dtoa_divmax: *mut c_int,
    dtoa: unsafe extern "C" fn(f64, c_int, c_int, *mut c_int, *mut c_int, *mut *mut c_char) -> *mut c_char,
    dtoa_r: unsafe extern "C" fn(f64, c_int, c_int, *mut c_int, *mut c_int, *mut *mut c_char, *mut c_char, usize) -> *mut c_char,
    freedtoa: unsafe extern "C" fn(*mut c_char),
    gethex: unsafe extern "C" fn(*mut *const c_char, *mut U, c_int, c_int),
    strtod__unused: unsafe extern "C" fn(*const c_char, *mut *mut c_char) -> f64,

    /* ---- value.c ---- */
    json_object: unsafe extern "C" fn() -> *mut Json,
    json_array: unsafe extern "C" fn() -> *mut Json,
    json_string: unsafe extern "C" fn(*const c_char) -> *mut Json,
    json_stringn: unsafe extern "C" fn(*const c_char, usize) -> *mut Json,
    json_string_nocheck: unsafe extern "C" fn(*const c_char) -> *mut Json,
    json_stringn_nocheck: unsafe extern "C" fn(*const c_char, usize) -> *mut Json,
    jsonp_stringn_nocheck_own: unsafe extern "C" fn(*const c_char, usize) -> *mut Json,
    json_integer: unsafe extern "C" fn(c_longlong) -> *mut Json,
    json_real: unsafe extern "C" fn(f64) -> *mut Json,
    json_true: unsafe extern "C" fn() -> *mut Json,
    json_false: unsafe extern "C" fn() -> *mut Json,
    json_null: unsafe extern "C" fn() -> *mut Json,
    json_delete: unsafe extern "C" fn(*mut Json),
    json_object_size: unsafe extern "C" fn(*const Json) -> usize,
    json_object_get: unsafe extern "C" fn(*const Json, *const c_char) -> *mut Json,
    json_object_getn: unsafe extern "C" fn(*const Json, *const c_char, usize) -> *mut Json,
    json_object_set_new: unsafe extern "C" fn(*mut Json, *const c_char, *mut Json) -> c_int,
    json_object_setn_new: unsafe extern "C" fn(*mut Json, *const c_char, usize, *mut Json) -> c_int,
    json_object_set_new_nocheck: unsafe extern "C" fn(*mut Json, *const c_char, *mut Json) -> c_int,
    json_object_setn_new_nocheck: unsafe extern "C" fn(*mut Json, *const c_char, usize, *mut Json) -> c_int,
    json_object_del: unsafe extern "C" fn(*mut Json, *const c_char) -> c_int,
    json_object_deln: unsafe extern "C" fn(*mut Json, *const c_char, usize) -> c_int,
    json_object_clear: unsafe extern "C" fn(*mut Json) -> c_int,
    json_object_update: unsafe extern "C" fn(*mut Json, *mut Json) -> c_int,
    json_object_update_existing: unsafe extern "C" fn(*mut Json, *mut Json) -> c_int,
    json_object_update_missing: unsafe extern "C" fn(*mut Json, *mut Json) -> c_int,
    json_object_update_recursive: unsafe extern "C" fn(*mut Json, *mut Json) -> c_int,
    do_object_update_recursive: unsafe extern "C" fn(*mut Json, *mut Json, *mut Hashtable) -> c_int,
    json_object_iter: unsafe extern "C" fn(*mut Json) -> *mut c_void,
    json_object_iter_at: unsafe extern "C" fn(*mut Json, *const c_char) -> *mut c_void,
    json_object_key_to_iter: unsafe extern "C" fn(*const c_char) -> *mut c_void,
    json_object_iter_next: unsafe extern "C" fn(*mut Json, *mut c_void) -> *mut c_void,
    json_object_iter_key: unsafe extern "C" fn(*mut c_void) -> *const c_char,
    json_object_iter_key_len: unsafe extern "C" fn(*mut c_void) -> usize,
    json_object_iter_value: unsafe extern "C" fn(*mut c_void) -> *mut Json,
    json_object_iter_set_new: unsafe extern "C" fn(*mut Json, *mut c_void, *mut Json) -> c_int,
    json_array_size: unsafe extern "C" fn(*const Json) -> usize,
    json_array_get: unsafe extern "C" fn(*const Json, usize) -> *mut Json,
    json_array_set_new: unsafe extern "C" fn(*mut Json, usize, *mut Json) -> c_int,
    json_array_append_new: unsafe extern "C" fn(*mut Json, *mut Json) -> c_int,
    json_array_insert_new: unsafe extern "C" fn(*mut Json, usize, *mut Json) -> c_int,
    json_array_remove: unsafe extern "C" fn(*mut Json, usize) -> c_int,
    json_array_clear: unsafe extern "C" fn(*mut Json) -> c_int,
    json_array_extend: unsafe extern "C" fn(*mut Json, *mut Json) -> c_int,
    json_string_value: unsafe extern "C" fn(*const Json) -> *const c_char,
    json_string_length: unsafe extern "C" fn(*const Json) -> usize,
    json_integer_value: unsafe extern "C" fn(*const Json) -> c_longlong,
    json_real_value: unsafe extern "C" fn(*const Json) -> f64,
    json_number_value: unsafe extern "C" fn(*const Json) -> f64,
    json_string_set: unsafe extern "C" fn(*mut Json, *const c_char) -> c_int,
    json_string_setn: unsafe extern "C" fn(*mut Json, *const c_char, usize) -> c_int,
    json_string_set_nocheck: unsafe extern "C" fn(*mut Json, *const c_char) -> c_int,
    json_string_setn_nocheck: unsafe extern "C" fn(*mut Json, *const c_char, usize) -> c_int,
    json_integer_set: unsafe extern "C" fn(*mut Json, c_longlong) -> c_int,
    json_real_set: unsafe extern "C" fn(*mut Json, f64) -> c_int,
    json_equal: unsafe extern "C" fn(*const Json, *const Json) -> c_int,
    json_copy: unsafe extern "C" fn(*mut Json) -> *mut Json,
    json_deep_copy: unsafe extern "C" fn(*const Json) -> *mut Json,
    do_deep_copy: unsafe extern "C" fn(*const Json, *mut Hashtable) -> *mut Json,
    jsonp_loop_check: unsafe extern "C" fn(*mut Hashtable, *const Json, *mut c_char, usize, *mut usize) -> c_int,
    json_sprintf: unsafe extern "C" fn(*const c_char, ...) -> *mut Json,

    /* ---- dump.c ---- */
    json_dumps: unsafe extern "C" fn(*const Json, usize) -> *mut c_char,
    json_dumpb: unsafe extern "C" fn(*const Json, *mut c_char, usize, usize) -> usize,
    json_dumpf: unsafe extern "C" fn(*const Json, *mut c_void, usize) -> c_int,
    json_dumpfd: unsafe extern "C" fn(*const Json, c_int, usize) -> c_int,
    json_dump_file: unsafe extern "C" fn(*const Json, *const c_char, usize) -> c_int,
    json_dump_callback: unsafe extern "C" fn(*const Json, DumpCallback, *mut c_void, usize) -> c_int,

    /* ---- load.c ---- */
    json_loads: unsafe extern "C" fn(*const c_char, usize, *mut JsonError) -> *mut Json,
    json_loadb: unsafe extern "C" fn(*const c_char, usize, usize, *mut JsonError) -> *mut Json,
    json_loadf: unsafe extern "C" fn(*mut c_void, usize, *mut JsonError) -> *mut Json,
    json_loadfd: unsafe extern "C" fn(c_int, usize, *mut JsonError) -> *mut Json,
    json_load_file: unsafe extern "C" fn(*const c_char, usize, *mut JsonError) -> *mut Json,
    json_load_callback: unsafe extern "C" fn(LoadCallback, *mut c_void, usize, *mut JsonError) -> *mut Json,

    /* ---- pack_unpack.c ---- */
    json_pack: unsafe extern "C" fn(*const c_char, ...) -> *mut Json,
    json_pack_ex: unsafe extern "C" fn(*mut JsonError, usize, *const c_char, ...) -> *mut Json,
    json_unpack: unsafe extern "C" fn(*mut Json, *const c_char, ...) -> c_int,
    json_unpack_ex: unsafe extern "C" fn(*mut Json, *mut JsonError, usize, *const c_char, ...) -> c_int,
}

/* --------------------------------------------------------------- .so paths */

/// Fixed hash seed installed in both libraries so that no behaviour can depend
/// on the random per-process seed.
pub const FIXED_SEED: usize = 0x5EED_1234;

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

pub fn c_so_path() -> PathBuf {
    let p = manifest_dir().join("c_src/build/libjansson.so");
    assert!(
        p.exists(),
        "C shared library not built: {:?}\n\
         build it with: cd c_src && mkdir -p build && cd build && \
         cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
        p
    );
    p
}

pub fn rust_so_path() -> PathBuf {
    // An explicit override makes it possible to re-run the whole suite against
    // the debug artefact (which has integer-overflow checks enabled).
    if let Some(p) = std::env::var_os("JANSSON_RUST_SO") {
        let p = PathBuf::from(p);
        assert!(p.exists(), "JANSSON_RUST_SO does not exist: {:?}", p);
        return p;
    }
    // Prefer the release artefact (that is what ships; `panic = \"abort\"` and no
    // debug overflow checks), fall back to debug.
    let rel = manifest_dir().join("target/release/libjansson.so");
    if rel.exists() {
        return rel;
    }
    let dbg = manifest_dir().join("target/debug/libjansson.so");
    assert!(dbg.exists(), "Rust shared library not built: {:?}", rel);
    dbg
}

pub struct Pair {
    pub c: Api,
    pub rust: Api,
}

/// Load both libraries.  Each call performs a fresh `dlopen`; because glibc
/// reference-counts by path the same mapping is reused, so global state
/// (allocator hooks, hash seed) persists for the whole test binary — the same
/// situation a real consumer sees.
pub fn load_pair() -> Pair {
    let c = Api::load(&c_so_path(), "C");
    let rust = Api::load(&rust_so_path(), "RUST");
    unsafe {
        // Deterministic hash seed on both sides.
        (c.json_object_seed)(FIXED_SEED);
        (rust.json_object_seed)(FIXED_SEED);
    }
    Pair { c, rust }
}

// The two `.so`s carry *process-global* state (allocator hooks, hash seed,
// dtoa's static result buffer), and glibc reference-counts `dlopen` by path, so
// every thread shares the same mapping.  Serialise all access.
unsafe impl Send for Api {}
unsafe impl Sync for Api {}

static PAIR: std::sync::OnceLock<std::sync::Mutex<Pair>> = std::sync::OnceLock::new();

pub fn with_pair<R>(f: impl FnOnce(&Pair) -> R) -> R {
    let m = PAIR.get_or_init(|| std::sync::Mutex::new(load_pair()));
    let g = m.lock().unwrap_or_else(|e| e.into_inner());
    f(&g)
}

/// Run `f` against the C library and against the Rust library and assert the
/// recorded observations are byte-identical.
pub fn diff<F>(what: &str, f: F)
where
    F: Fn(&Api, &mut Rec),
{
    with_pair(|p| {
        let mut rc = Rec::new();
        let mut rr = Rec::new();
        f(&p.c, &mut rc);
        f(&p.rust, &mut rr);
        if std::env::var_os("JANSSON_DIFF_STATS").is_some() {
            eprintln!(
                "[diff] {what}: {} observations, {} bytes",
                rc.out.lines().count(),
                rc.out.len()
            );
        }
        if rc.out != rr.out {
            let diff = first_diff(&rc.out, &rr.out);
            panic!(
                "DIVERGENCE in {what}\n--- first differing line ---\n{diff}\n\
                 --- C ({} lines) ---\n{}\n--- RUST ({} lines) ---\n{}",
                rc.out.lines().count(),
                clip(&rc.out),
                rr.out.lines().count(),
                clip(&rr.out),
            );
        }
    });
}

/// Same as [`diff`] but gives the closure a mutable seed so property-style
/// randomised runs stay reproducible.
pub fn diff_seeded<F>(what: &str, iters: u32, f: F)
where
    F: Fn(&Api, &mut Rec, &mut Rng),
{
    diff(what, |api, rec| {
        let mut rng = Rng::new(0x243F_6A88_85A3_08D3);
        for i in 0..iters {
            rec.tag_i("iter", i as i64);
            f(api, rec, &mut rng);
        }
    });
}

fn clip(s: &str) -> String {
    const MAX: usize = 6000;
    if s.len() <= MAX {
        s.to_string()
    } else {
        format!("{}\n...[{} more bytes]", &s[..MAX], s.len() - MAX)
    }
}

fn first_diff(a: &str, b: &str) -> String {
    let av: Vec<&str> = a.lines().collect();
    let bv: Vec<&str> = b.lines().collect();
    for i in 0..av.len().max(bv.len()) {
        let x = av.get(i).copied().unwrap_or("<missing>");
        let y = bv.get(i).copied().unwrap_or("<missing>");
        if x != y {
            let ctx = i.saturating_sub(3);
            let mut s = String::new();
            for j in ctx..i {
                let _ = writeln!(s, "  ctx[{j}] {}", av[j]);
            }
            let _ = writeln!(s, "line {i}:\n    C: {x}\n RUST: {y}");
            return s;
        }
    }
    "identical line content but different trailing bytes".into()
}

/* ------------------------------------------------------------- recorder */

/// Accumulates a canonical textual transcript of everything a test observes.
pub struct Rec {
    pub out: String,
}

impl Rec {
    pub fn new() -> Rec {
        Rec { out: String::new() }
    }
    pub fn line(&mut self, s: &str) {
        self.out.push_str(s);
        self.out.push('\n');
    }
    pub fn tag_i(&mut self, tag: &str, v: i64) {
        let _ = writeln!(self.out, "{tag}={v}");
    }
    pub fn tag_u(&mut self, tag: &str, v: usize) {
        let _ = writeln!(self.out, "{tag}={v}");
    }
    pub fn tag_f(&mut self, tag: &str, v: f64) {
        // bit pattern -> exact, NaN-safe comparison
        let _ = writeln!(self.out, "{tag}=0x{:016x}", v.to_bits());
    }
    pub fn tag_s(&mut self, tag: &str, v: &str) {
        let _ = writeln!(self.out, "{tag}={v:?}");
    }
    pub fn tag_bytes(&mut self, tag: &str, v: &[u8]) {
        let _ = writeln!(self.out, "{tag}=[{}]", hex(v));
    }
    pub fn tag_ptr_null(&mut self, tag: &str, p: *const c_void) {
        let _ = writeln!(self.out, "{tag}={}", if p.is_null() { "NULL" } else { "ptr" });
    }
    /// Record a json_t: null-ness, type tag and refcount.
    pub fn json(&mut self, tag: &str, p: *const Json) {
        if p.is_null() {
            let _ = writeln!(self.out, "{tag}=NULL");
        } else {
            let v = unsafe { *p };
            let _ = writeln!(self.out, "{tag}=type:{} rc:{}", v.type_, v.refcount as isize);
        }
    }
    /// Record a NUL-terminated C string (or NULL).
    pub fn cstring(&mut self, tag: &str, p: *const c_char) {
        if p.is_null() {
            let _ = writeln!(self.out, "{tag}=NULL");
        } else {
            let s = unsafe { CStr::from_ptr(p) };
            let _ = writeln!(self.out, "{tag}={:?}", s.to_bytes());
        }
    }
    pub fn error(&mut self, tag: &str, e: &JsonError) {
        let _ = writeln!(
            self.out,
            "{tag}=line:{} col:{} pos:{} code:{} src:{:?} text:{:?}",
            e.line,
            e.column,
            e.position,
            e.code(),
            e.source_str(),
            e.text_str()
        );
        // and the whole struct, byte for byte
        let _ = writeln!(self.out, "{tag}.raw=[{}]", hex(e.as_bytes()));
    }
}

pub fn hex(b: &[u8]) -> String {
    let mut s = String::with_capacity(b.len() * 2);
    for x in b {
        let _ = write!(s, "{x:02x}");
    }
    s
}

/* ------------------------------------------------------------------- rng */

/// SplitMix64 — deterministic, reproducible, no external crates.
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng(seed)
    }
    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    pub fn next_u32(&mut self) -> u32 {
        (self.next_u64() >> 32) as u32
    }
    pub fn below(&mut self, n: usize) -> usize {
        if n == 0 {
            0
        } else {
            (self.next_u64() % n as u64) as usize
        }
    }
    pub fn range_i64(&mut self, lo: i64, hi: i64) -> i64 {
        if hi <= lo {
            return lo;
        }
        lo + (self.next_u64() % ((hi - lo) as u64)) as i64
    }
    pub fn f64_any(&mut self) -> f64 {
        loop {
            let v = f64::from_bits(self.next_u64());
            if v.is_finite() {
                return v;
            }
        }
    }
    /// "Interesting" finite double: mixes small integers, fractions and extremes.
    pub fn f64_interesting(&mut self) -> f64 {
        match self.below(8) {
            0 => self.range_i64(-1000, 1000) as f64,
            1 => self.range_i64(-1_000_000_000, 1_000_000_000) as f64,
            2 => (self.next_u32() as f64) / 7.0,
            3 => (self.next_u32() as f64) * 1e-30,
            4 => (self.next_u32() as f64) * 1e30,
            5 => self.f64_any(),
            6 => {
                let v = [
                    0.0,
                    -0.0,
                    1.0,
                    -1.0,
                    0.5,
                    1e16,
                    1e17,
                    1e-4,
                    1e-5,
                    f64::MIN_POSITIVE,
                    f64::MAX,
                    -f64::MAX,
                    5e-324,
                    1.7976931348623157e308,
                    2.2250738585072014e-308,
                    123456789012345678.0,
                    0.1,
                    1.0 / 3.0,
                ];
                v[self.below(v.len())]
            }
            _ => f64::from_bits(self.next_u64() & 0x7FEF_FFFF_FFFF_FFFF),
        }
    }
    /// Random byte string, `ascii_only` restricts to printable ASCII.
    pub fn bytes(&mut self, len: usize, ascii_only: bool) -> Vec<u8> {
        (0..len)
            .map(|_| {
                if ascii_only {
                    b' ' + (self.below(95)) as u8
                } else {
                    (self.next_u32() & 0xFF) as u8
                }
            })
            .collect()
    }
    /// Random *valid* UTF-8 string.
    pub fn utf8(&mut self, chars: usize) -> String {
        let mut s = String::new();
        for _ in 0..chars {
            let c = match self.below(5) {
                // never U+0000: many call sites pass these strings through
                // CString / strlen, where an interior NUL is not expressible.
                0 => 1 + self.below(0x7F) as u32,
                1 => 0x80 + self.below(0x780) as u32,
                2 => 0x800 + self.below(0xF800) as u32,
                3 => 0x10000 + self.below(0x100000) as u32,
                _ => *[0x22u32, 0x5C, 0x2F, 0x08, 0x0C, 0x0A, 0x0D, 0x09, 0x01, 0x1F]
                    .get(self.below(10))
                    .unwrap(),
            };
            if let Some(ch) = char::from_u32(c) {
                s.push(ch);
            } else {
                s.push('?');
            }
        }
        s
    }
}

/* --------------------------------------------------------------- utilities */

pub fn cs(s: &str) -> CString {
    CString::new(s).unwrap()
}

/// A NUL-terminated buffer that may itself contain embedded NUL bytes.
pub fn cbuf(b: &[u8]) -> Vec<u8> {
    let mut v = b.to_vec();
    v.push(0);
    v
}

/// Dump `json` with `flags` through the given library and return the bytes.
pub unsafe fn dumps(api: &Api, json: *const Json, flags: usize) -> Option<Vec<u8>> {
    let p = (api.json_dumps)(json, flags);
    if p.is_null() {
        return None;
    }
    let v = CStr::from_ptr(p).to_bytes().to_vec();
    (api.jsonp_free)(p as *mut c_void);
    Some(v)
}

/// Record the canonical dump of a value under several flag sets — catches
/// structural differences that a single dump could hide.
pub unsafe fn rec_dump_all(api: &Api, rec: &mut Rec, tag: &str, json: *const Json) {
    for (i, f) in [
        JSON_ENCODE_ANY,
        JSON_ENCODE_ANY | JSON_COMPACT,
        JSON_ENCODE_ANY | JSON_SORT_KEYS,
        JSON_ENCODE_ANY | JSON_ENSURE_ASCII,
        JSON_ENCODE_ANY | json_indent(2),
    ]
    .iter()
    .enumerate()
    {
        match dumps(api, json, *f) {
            None => rec.line(&format!("{tag}.dump{i}=NULL")),
            Some(v) => rec.tag_bytes(&format!("{tag}.dump{i}"), &v),
        }
    }
}

/* ------------------------------------------------------- libc bits we need */

extern "C" {
    pub fn malloc(n: usize) -> *mut c_void;
    pub fn realloc(p: *mut c_void, n: usize) -> *mut c_void;
    pub fn free(p: *mut c_void);
    pub fn fopen(path: *const c_char, mode: *const c_char) -> *mut c_void;
    pub fn fclose(f: *mut c_void) -> c_int;
    pub fn fflush(f: *mut c_void) -> c_int;
    pub fn fileno(f: *mut c_void) -> c_int;
    pub fn strtod(s: *const c_char, end: *mut *mut c_char) -> f64;
}

/* -------------------------------------------------- instrumented allocator */

use std::sync::atomic::{AtomicI64, Ordering::SeqCst};

/// Number of `jsonp_malloc`/`jsonp_realloc` calls since the last reset.
pub static ALLOC_COUNT: AtomicI64 = AtomicI64::new(0);
/// Fail exactly this allocation index (-1 = never).
pub static ALLOC_FAIL_NTH: AtomicI64 = AtomicI64::new(-1);
/// Fail this allocation index and every later one (-1 = never).
pub static ALLOC_FAIL_FROM: AtomicI64 = AtomicI64::new(-1);

fn should_fail() -> bool {
    let idx = ALLOC_COUNT.fetch_add(1, SeqCst);
    let nth = ALLOC_FAIL_NTH.load(SeqCst);
    let from = ALLOC_FAIL_FROM.load(SeqCst);
    (nth >= 0 && idx == nth) || (from >= 0 && idx >= from)
}

pub unsafe extern "C" fn hook_malloc(n: usize) -> *mut c_void {
    if should_fail() {
        std::ptr::null_mut()
    } else {
        malloc(n)
    }
}

pub unsafe extern "C" fn hook_realloc(p: *mut c_void, n: usize) -> *mut c_void {
    if should_fail() {
        std::ptr::null_mut()
    } else {
        realloc(p, n)
    }
}

pub unsafe extern "C" fn hook_free(p: *mut c_void) {
    free(p)
}

/// Reset the allocation counter and clear all failure injection.
pub fn alloc_reset() {
    ALLOC_COUNT.store(0, SeqCst);
    ALLOC_FAIL_NTH.store(-1, SeqCst);
    ALLOC_FAIL_FROM.store(-1, SeqCst);
}

pub fn alloc_fail_nth(n: i64) {
    ALLOC_COUNT.store(0, SeqCst);
    ALLOC_FAIL_NTH.store(n, SeqCst);
    ALLOC_FAIL_FROM.store(-1, SeqCst);
}

pub fn alloc_fail_from(n: i64) {
    ALLOC_COUNT.store(0, SeqCst);
    ALLOC_FAIL_NTH.store(-1, SeqCst);
    ALLOC_FAIL_FROM.store(n, SeqCst);
}

pub fn alloc_count() -> i64 {
    ALLOC_COUNT.load(SeqCst)
}

/// Install `malloc`/`realloc`/`free` forwarding hooks (`do_realloc != NULL`).
/// Because they forward to the very same libc heap, pointers stay compatible
/// with the default allocator, so the hooks can be installed permanently.
pub unsafe fn install_hooks2(api: &Api) {
    (api.json_set_alloc_funcs2)(Some(hook_malloc), Some(hook_realloc), Some(hook_free));
}

/// Install only malloc/free (`do_realloc == NULL`) — selects the realloc
/// *emulation* branch inside `jsonp_realloc`.
pub unsafe fn install_hooks1(api: &Api) {
    (api.json_set_alloc_funcs)(Some(hook_malloc), Some(hook_free));
}

/// Restore the pristine libc allocator triple.
pub unsafe fn restore_alloc(api: &Api) {
    (api.json_set_alloc_funcs2)(Some(real_malloc), Some(real_realloc), Some(real_free));
}

pub unsafe extern "C" fn real_malloc(n: usize) -> *mut c_void {
    malloc(n)
}
pub unsafe extern "C" fn real_realloc(p: *mut c_void, n: usize) -> *mut c_void {
    realloc(p, n)
}
pub unsafe extern "C" fn real_free(p: *mut c_void) {
    free(p)
}

/* ------------------------------------------------------------ temp files */

pub fn tmp_file(name: &str) -> PathBuf {
    let mut d = std::env::temp_dir();
    d.push(format!("jansson_difftest_{}_{}", std::process::id(), name));
    d
}

pub mod tree;

extern "C" {
    pub static stdin: *mut c_void;
    pub fn freopen(path: *const c_char, mode: *const c_char, stream: *mut c_void) -> *mut c_void;
}

/// Serialise a [`tree::Spec`] to JSON text *in Rust*, so both libraries get a
/// byte-identical input document.
pub fn spec_to_text(s: &tree::Spec, out: &mut String) {
    use tree::Spec;
    match s {
        Spec::Null => out.push_str("null"),
        Spec::True => out.push_str("true"),
        Spec::False => out.push_str("false"),
        Spec::Int(v) => {
            let _ = write!(out, "{v}");
        }
        Spec::Real(v) => {
            // 17 significant digits round-trips every double
            let mut t = format!("{:.17e}", v);
            if !t.contains('e') {
                t.push_str("e0");
            }
            out.push_str(&t);
        }
        Spec::Str(t) => escape_json_str(t.as_bytes(), out),
        Spec::StrRaw(b) => escape_json_str(b, out),
        Spec::Arr(items) => {
            out.push('[');
            for (i, it) in items.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                spec_to_text(it, out);
            }
            out.push(']');
        }
        Spec::Obj(pairs) => {
            out.push('{');
            for (i, (k, v)) in pairs.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                escape_json_str(k, out);
                out.push(':');
                spec_to_text(v, out);
            }
            out.push('}');
        }
    }
}

/// Escape bytes as a JSON string literal; non-UTF-8 bytes become `\uXXXX` of
/// their latin-1 value so the result is always valid UTF-8 JSON text.
pub fn escape_json_str(b: &[u8], out: &mut String) {
    out.push('"');
    match std::str::from_utf8(b) {
        Ok(s) => {
            for c in s.chars() {
                match c {
                    '"' => out.push_str("\\\""),
                    '\\' => out.push_str("\\\\"),
                    '\n' => out.push_str("\\n"),
                    '\r' => out.push_str("\\r"),
                    '\t' => out.push_str("\\t"),
                    '\u{8}' => out.push_str("\\b"),
                    '\u{c}' => out.push_str("\\f"),
                    c if (c as u32) < 0x20 => {
                        let _ = write!(out, "\\u{:04X}", c as u32);
                    }
                    c => out.push(c),
                }
            }
        }
        Err(_) => {
            for &x in b {
                let _ = write!(out, "\\u{:04X}", x as u32);
            }
        }
    }
    out.push('"');
}

/// Reimplementation of the `json_decref` static inline from `jansson.h`
/// (it is not an exported symbol, so an external consumer inlines it too).
pub unsafe fn decref(api: &Api, json: *mut Json) {
    if json.is_null() {
        return;
    }
    if (*json).refcount == usize::MAX {
        return;
    }
    (*json).refcount -= 1;
    if (*json).refcount == 0 {
        (api.json_delete)(json);
    }
}

/// Reimplementation of the `json_incref` static inline from `jansson.h`.
pub unsafe fn incref(api: &Api, json: *mut Json) -> *mut Json {
    let _ = api;
    if !json.is_null() && (*json).refcount != usize::MAX {
        (*json).refcount += 1;
    }
    json
}

/* --------------------------------------------- enum json_error_code ----- */

pub const E_UNKNOWN: u8 = 0;
pub const E_OUT_OF_MEMORY: u8 = 1;
pub const E_STACK_OVERFLOW: u8 = 2;
pub const E_CANNOT_OPEN_FILE: u8 = 3;
pub const E_INVALID_ARGUMENT: u8 = 4;
pub const E_INVALID_UTF8: u8 = 5;
pub const E_PREMATURE_END_OF_INPUT: u8 = 6;
pub const E_END_OF_INPUT_EXPECTED: u8 = 7;
pub const E_INVALID_SYNTAX: u8 = 8;
pub const E_INVALID_FORMAT: u8 = 9;
pub const E_WRONG_TYPE: u8 = 10;
pub const E_NULL_CHARACTER: u8 = 11;
pub const E_NULL_VALUE: u8 = 12;
pub const E_NULL_BYTE_IN_KEY: u8 = 13;
pub const E_DUPLICATE_KEY: u8 = 14;
pub const E_NUMERIC_OVERFLOW: u8 = 15;
pub const E_ITEM_NOT_FOUND: u8 = 16;
pub const E_INDEX_OUT_OF_RANGE: u8 = 17;

/// Assert that the library actually took the branch the ERRORS.md row claims.
/// This guards against a test that "passes" because both libraries silently
/// succeeded instead of rejecting the input.
pub fn expect_code(api: &Api, row: u32, e: &JsonError, want: u8) {
    assert_eq!(
        e.code(),
        want,
        "[{}] ERRORS.md row {row}: expected error code {want}, got {} (text {:?})",
        api.tag,
        e.code(),
        e.text_str()
    );
}

/// Forge a `json_t` header with an out-of-range `json_type` tag, exactly as a
/// hostile / buggy C caller could hand one across the FFI boundary.
pub unsafe fn forge_json(api: &Api, type_tag: c_int, refcount: usize) -> *mut Json {
    let p = (api.jsonp_malloc)(std::mem::size_of::<Json>()) as *mut Json;
    assert!(!p.is_null());
    (*p).type_ = type_tag;
    (*p).refcount = refcount;
    p
}

/// Run `op` once to count how many allocations it performs, then re-run it with
/// each allocation index made to fail in turn.  This mechanically covers every
/// out-of-memory branch reachable from `op`.
pub unsafe fn oom_sweep<F>(api: &Api, rec: &mut Rec, tag: &str, limit: i64, op: F)
where
    F: Fn(&Api, &mut Rec),
{
    install_hooks2(api);
    alloc_reset();
    let mut probe = Rec::new();
    op(api, &mut probe);
    let total = alloc_count().min(limit);
    rec.tag_i(&format!("{tag}.alloc_total"), alloc_count());
    for k in 0..total {
        alloc_fail_nth(k);
        rec.tag_i(&format!("{tag}.fail{k}"), k);
        op(api, rec);
        alloc_reset();
    }
    // and "everything from here on fails"
    for k in 0..total.min(8) {
        alloc_fail_from(k);
        rec.tag_i(&format!("{tag}.from{k}"), k);
        op(api, rec);
        alloc_reset();
    }
    restore_alloc(api);
    alloc_reset();
}
