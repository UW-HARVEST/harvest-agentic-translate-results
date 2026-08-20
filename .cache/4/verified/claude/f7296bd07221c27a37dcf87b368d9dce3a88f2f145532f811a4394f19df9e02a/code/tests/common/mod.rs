//! Differential-test harness: loads BOTH the C `.so` and the Rust `.so` with
//! `libloading` and calls every function through the FFI boundary only.
//!
//! Nothing in here ever links against the Rust crate directly — every call goes
//! through `dlsym`, exactly like an external consumer, so the `#[no_mangle]`
//! export wrappers are part of what is being tested.
#![allow(dead_code)]
#![allow(non_snake_case)]

use libloading::Library;
use std::collections::HashMap;
use std::ffi::{c_char, c_int, c_void, CStr, CString};
use std::fmt::Write as _;

/* ------------------------------------------------------------------ */
/* mirrored public types                                              */
/* ------------------------------------------------------------------ */

pub const CJSON_INVALID: c_int = 0;
pub const CJSON_FALSE: c_int = 1 << 0;
pub const CJSON_TRUE: c_int = 1 << 1;
pub const CJSON_NULL: c_int = 1 << 2;
pub const CJSON_NUMBER: c_int = 1 << 3;
pub const CJSON_STRING: c_int = 1 << 4;
pub const CJSON_ARRAY: c_int = 1 << 5;
pub const CJSON_OBJECT: c_int = 1 << 6;
pub const CJSON_RAW: c_int = 1 << 7;
pub const CJSON_IS_REFERENCE: c_int = 256;
pub const CJSON_STRING_IS_CONST: c_int = 512;

#[repr(C)]
#[derive(Clone, Copy)]
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

pub type MallocFn = unsafe extern "C" fn(usize) -> *mut c_void;
pub type FreeFn = unsafe extern "C" fn(*mut c_void);

#[repr(C)]
pub struct CJsonHooks {
    pub malloc_fn: Option<MallocFn>,
    pub free_fn: Option<FreeFn>,
}

/* ------------------------------------------------------------------ */
/* the loaded API                                                     */
/* ------------------------------------------------------------------ */

macro_rules! api_struct {
    ( $( $name:ident : $t:ty , )* ) => {
        pub struct Api {
            pub tag: &'static str,
            $( pub $name: $t, )*
        }

        impl Api {
            fn load_from(path: &str, tag: &'static str) -> Api {
                let lib: &'static Library = Box::leak(Box::new(
                    unsafe { Library::new(path) }
                        .unwrap_or_else(|e| panic!("cannot dlopen {path}: {e}")),
                ));
                unsafe {
                    Api {
                        tag,
                        $( $name: *lib
                            .get::<$t>(concat!(stringify!($name), "\0").as_bytes())
                            .unwrap_or_else(|e| panic!("{}: missing symbol {}: {}", path, stringify!($name), e)), )*
                    }
                }
            }
        }
    };
}

api_struct! {
    cJSON_Version: unsafe extern "C" fn() -> *const c_char,
    cJSON_InitHooks: unsafe extern "C" fn(*mut CJsonHooks),
    cJSON_Parse: unsafe extern "C" fn(*const c_char) -> *mut CJson,
    cJSON_ParseWithLength: unsafe extern "C" fn(*const c_char, usize) -> *mut CJson,
    cJSON_ParseWithOpts: unsafe extern "C" fn(*const c_char, *mut *const c_char, c_int) -> *mut CJson,
    cJSON_ParseWithLengthOpts: unsafe extern "C" fn(*const c_char, usize, *mut *const c_char, c_int) -> *mut CJson,
    cJSON_Print: unsafe extern "C" fn(*const CJson) -> *mut c_char,
    cJSON_PrintUnformatted: unsafe extern "C" fn(*const CJson) -> *mut c_char,
    cJSON_PrintBuffered: unsafe extern "C" fn(*const CJson, c_int, c_int) -> *mut c_char,
    cJSON_PrintPreallocated: unsafe extern "C" fn(*mut CJson, *mut c_char, c_int, c_int) -> c_int,
    cJSON_Delete: unsafe extern "C" fn(*mut CJson),
    cJSON_GetArraySize: unsafe extern "C" fn(*const CJson) -> c_int,
    cJSON_GetArrayItem: unsafe extern "C" fn(*const CJson, c_int) -> *mut CJson,
    cJSON_GetObjectItem: unsafe extern "C" fn(*const CJson, *const c_char) -> *mut CJson,
    cJSON_GetObjectItemCaseSensitive: unsafe extern "C" fn(*const CJson, *const c_char) -> *mut CJson,
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
    cJSON_CreateNull: unsafe extern "C" fn() -> *mut CJson,
    cJSON_CreateTrue: unsafe extern "C" fn() -> *mut CJson,
    cJSON_CreateFalse: unsafe extern "C" fn() -> *mut CJson,
    cJSON_CreateBool: unsafe extern "C" fn(c_int) -> *mut CJson,
    cJSON_CreateNumber: unsafe extern "C" fn(f64) -> *mut CJson,
    cJSON_CreateString: unsafe extern "C" fn(*const c_char) -> *mut CJson,
    cJSON_CreateRaw: unsafe extern "C" fn(*const c_char) -> *mut CJson,
    cJSON_CreateArray: unsafe extern "C" fn() -> *mut CJson,
    cJSON_CreateObject: unsafe extern "C" fn() -> *mut CJson,
    cJSON_CreateStringReference: unsafe extern "C" fn(*const c_char) -> *mut CJson,
    cJSON_CreateObjectReference: unsafe extern "C" fn(*const CJson) -> *mut CJson,
    cJSON_CreateArrayReference: unsafe extern "C" fn(*const CJson) -> *mut CJson,
    cJSON_CreateIntArray: unsafe extern "C" fn(*const c_int, c_int) -> *mut CJson,
    cJSON_CreateFloatArray: unsafe extern "C" fn(*const f32, c_int) -> *mut CJson,
    cJSON_CreateDoubleArray: unsafe extern "C" fn(*const f64, c_int) -> *mut CJson,
    cJSON_CreateStringArray: unsafe extern "C" fn(*const *const c_char, c_int) -> *mut CJson,
    cJSON_AddItemToArray: unsafe extern "C" fn(*mut CJson, *mut CJson) -> c_int,
    cJSON_AddItemToObject: unsafe extern "C" fn(*mut CJson, *const c_char, *mut CJson) -> c_int,
    cJSON_AddItemToObjectCS: unsafe extern "C" fn(*mut CJson, *const c_char, *mut CJson) -> c_int,
    cJSON_AddItemReferenceToArray: unsafe extern "C" fn(*mut CJson, *mut CJson) -> c_int,
    cJSON_AddItemReferenceToObject: unsafe extern "C" fn(*mut CJson, *const c_char, *mut CJson) -> c_int,
    cJSON_DetachItemViaPointer: unsafe extern "C" fn(*mut CJson, *mut CJson) -> *mut CJson,
    cJSON_DetachItemFromArray: unsafe extern "C" fn(*mut CJson, c_int) -> *mut CJson,
    cJSON_DeleteItemFromArray: unsafe extern "C" fn(*mut CJson, c_int),
    cJSON_DetachItemFromObject: unsafe extern "C" fn(*mut CJson, *const c_char) -> *mut CJson,
    cJSON_DetachItemFromObjectCaseSensitive: unsafe extern "C" fn(*mut CJson, *const c_char) -> *mut CJson,
    cJSON_DeleteItemFromObject: unsafe extern "C" fn(*mut CJson, *const c_char),
    cJSON_DeleteItemFromObjectCaseSensitive: unsafe extern "C" fn(*mut CJson, *const c_char),
    cJSON_InsertItemInArray: unsafe extern "C" fn(*mut CJson, c_int, *mut CJson) -> c_int,
    cJSON_ReplaceItemViaPointer: unsafe extern "C" fn(*mut CJson, *mut CJson, *mut CJson) -> c_int,
    cJSON_ReplaceItemInArray: unsafe extern "C" fn(*mut CJson, c_int, *mut CJson) -> c_int,
    cJSON_ReplaceItemInObject: unsafe extern "C" fn(*mut CJson, *const c_char, *mut CJson) -> c_int,
    cJSON_ReplaceItemInObjectCaseSensitive: unsafe extern "C" fn(*mut CJson, *const c_char, *mut CJson) -> c_int,
    cJSON_Duplicate: unsafe extern "C" fn(*const CJson, c_int) -> *mut CJson,
    cJSON_Compare: unsafe extern "C" fn(*const CJson, *const CJson, c_int) -> c_int,
    cJSON_Minify: unsafe extern "C" fn(*mut c_char),
    cJSON_AddNullToObject: unsafe extern "C" fn(*mut CJson, *const c_char) -> *mut CJson,
    cJSON_AddTrueToObject: unsafe extern "C" fn(*mut CJson, *const c_char) -> *mut CJson,
    cJSON_AddFalseToObject: unsafe extern "C" fn(*mut CJson, *const c_char) -> *mut CJson,
    cJSON_AddBoolToObject: unsafe extern "C" fn(*mut CJson, *const c_char, c_int) -> *mut CJson,
    cJSON_AddNumberToObject: unsafe extern "C" fn(*mut CJson, *const c_char, f64) -> *mut CJson,
    cJSON_AddStringToObject: unsafe extern "C" fn(*mut CJson, *const c_char, *const c_char) -> *mut CJson,
    cJSON_AddRawToObject: unsafe extern "C" fn(*mut CJson, *const c_char, *const c_char) -> *mut CJson,
    cJSON_AddObjectToObject: unsafe extern "C" fn(*mut CJson, *const c_char) -> *mut CJson,
    cJSON_AddArrayToObject: unsafe extern "C" fn(*mut CJson, *const c_char) -> *mut CJson,
    cJSON_SetNumberHelper: unsafe extern "C" fn(*mut CJson, f64) -> f64,
    cJSON_SetValuestring: unsafe extern "C" fn(*mut CJson, *const c_char) -> *mut c_char,
    cJSON_malloc: unsafe extern "C" fn(usize) -> *mut c_void,
    cJSON_free: unsafe extern "C" fn(*mut c_void),
}

fn manifest_dir() -> String {
    env!("CARGO_MANIFEST_DIR").to_string()
}

fn c_so_path() -> String {
    if let Ok(p) = std::env::var("CJSON_C_SO") {
        return p;
    }
    format!("{}/c_src/build/libcjson.so.1.7.19", manifest_dir())
}

fn rust_so_path() -> String {
    if let Ok(p) = std::env::var("CJSON_RUST_SO") {
        return p;
    }
    format!("{}/target/release/libcJSON_test.so", manifest_dir())
}

pub fn c_driver_so_path() -> String {
    if let Ok(p) = std::env::var("CJSON_C_DRIVER_SO") {
        return p;
    }
    format!("{}/c_src/build/libcJSON_test.so", manifest_dir())
}

pub fn rust_driver_so_path() -> String {
    rust_so_path()
}

/// The two loaded libraries. Loaded once per test process (each integration
/// test file is its own process).
pub fn libs() -> (&'static Api, &'static Api) {
    use std::sync::OnceLock;
    static PAIR: OnceLock<(Api, Api)> = OnceLock::new();
    let p = PAIR.get_or_init(|| {
        for path in [c_so_path(), rust_so_path()] {
            assert!(
                std::path::Path::new(&path).exists(),
                "missing shared object {path}\n\
                 build the C side with:  (cd c_src && mkdir -p build && cd build && \
                 cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .)\n\
                 build the Rust side with: cargo build --release"
            );
        }
        (
            Api::load_from(&c_so_path(), "C"),
            Api::load_from(&rust_so_path(), "RUST"),
        )
    });
    (&p.0, &p.1)
}

/* ------------------------------------------------------------------ */
/* string helpers                                                     */
/* ------------------------------------------------------------------ */

/// NUL-terminated byte buffer that we own, so both libraries see the very same
/// address (which makes `cJSON_GetErrorPtr()` offsets directly comparable).
pub struct CBuf(pub Vec<u8>);

impl CBuf {
    pub fn new(bytes: &[u8]) -> CBuf {
        let mut v = bytes.to_vec();
        v.push(0);
        CBuf(v)
    }
    pub fn ptr(&self) -> *const c_char {
        self.0.as_ptr() as *const c_char
    }
    pub fn ptr_mut(&mut self) -> *mut c_char {
        self.0.as_mut_ptr() as *mut c_char
    }
    /// length without the trailing NUL
    pub fn len(&self) -> usize {
        self.0.len() - 1
    }
}

pub fn cs(s: &str) -> CString {
    CString::new(s).unwrap()
}

/// Render a byte slice deterministically (printable ASCII kept, everything else
/// escaped) so assertion messages stay readable.
pub fn show(bytes: &[u8]) -> String {
    let mut out = String::new();
    for &b in bytes {
        if b == b'\\' {
            out.push_str("\\\\");
        } else if (0x20..0x7f).contains(&b) {
            out.push(b as char);
        } else {
            let _ = write!(out, "\\x{b:02x}");
        }
    }
    out
}

pub unsafe fn read_cstr(p: *const c_char) -> Option<Vec<u8>> {
    if p.is_null() {
        None
    } else {
        Some(CStr::from_ptr(p).to_bytes().to_vec())
    }
}

/* ------------------------------------------------------------------ */
/* graph dumping (address independent)                                */
/* ------------------------------------------------------------------ */

unsafe fn walk(node: *mut CJson, ids: &mut HashMap<usize, usize>, order: &mut Vec<*mut CJson>) {
    let mut cur = node;
    while !cur.is_null() {
        if ids.contains_key(&(cur as usize)) {
            break;
        }
        ids.insert(cur as usize, order.len());
        order.push(cur);
        walk((*cur).child, ids, order);
        cur = (*cur).next;
    }
}

/// Canonical, address-independent textual representation of a cJSON graph:
/// every node numbered in a deterministic traversal order, all scalar fields,
/// and the `next`/`prev`/`child` topology expressed with those numbers.
pub unsafe fn dump(root: *const CJson) -> String {
    let mut ids = HashMap::new();
    let mut order = Vec::new();
    walk(root as *mut CJson, &mut ids, &mut order);

    let name = |p: *mut CJson| -> String {
        if p.is_null() {
            "-".to_string()
        } else {
            match ids.get(&(p as usize)) {
                Some(i) => format!("#{i}"),
                None => "EXT".to_string(),
            }
        }
    };

    if order.is_empty() {
        return "NULL".to_string();
    }

    let mut out = String::new();
    for (i, &n) in order.iter().enumerate() {
        let vs = match read_cstr((*n).valuestring) {
            None => "-".to_string(),
            Some(b) => format!("\"{}\"", show(&b)),
        };
        let st = match read_cstr((*n).string) {
            None => "-".to_string(),
            Some(b) => format!("\"{}\"", show(&b)),
        };
        let _ = writeln!(
            out,
            "#{i} type=0x{:x} int={} dbl=0x{:016x} vs={vs} str={st} next={} prev={} child={}",
            (*n).type_,
            (*n).valueint,
            (*n).valuedouble.to_bits(),
            name((*n).next),
            name((*n).prev),
            name((*n).child),
        );
    }
    out
}

/* ------------------------------------------------------------------ */
/* print helpers                                                      */
/* ------------------------------------------------------------------ */

/// Call a printing function on both libraries and return the produced bytes.
pub unsafe fn take_print(api: &Api, p: *mut c_char) -> Option<Vec<u8>> {
    if p.is_null() {
        return None;
    }
    let bytes = CStr::from_ptr(p).to_bytes().to_vec();
    (api.cJSON_free)(p as *mut c_void);
    Some(bytes)
}

pub fn assert_bytes_eq(what: &str, c: &Option<Vec<u8>>, r: &Option<Vec<u8>>) {
    match (c, r) {
        (None, None) => {}
        (Some(a), Some(b)) => assert!(
            a == b,
            "{what}: C and Rust printed different bytes\n  C   : {}\n  RUST: {}",
            show(a),
            show(b)
        ),
        _ => panic!(
            "{what}: NULL mismatch (C={:?} RUST={:?})",
            c.as_ref().map(|v| show(v)),
            r.as_ref().map(|v| show(v))
        ),
    }
}

/* ------------------------------------------------------------------ */
/* the differential driver                                            */
/* ------------------------------------------------------------------ */

/// Run the same scenario against the C `.so` and the Rust `.so` and require the
/// produced observation logs to be identical, line by line.
///
/// The closure gets one `Api` and returns everything it observed (return
/// values, printed bytes, error-pointer offsets, complete item graphs ...).
pub fn diff<F>(what: &str, scenario: F)
where
    F: Fn(&Api) -> String,
{
    // Both libraries keep process-global state (`global_error`, `global_hooks`,
    // the `cJSON_Version` buffer), so scenarios must never run concurrently.
    static SERIAL: std::sync::Mutex<()> = std::sync::Mutex::new(());
    let _serial = SERIAL.lock().unwrap_or_else(|e| e.into_inner());

    let (c, r) = libs();
    let a = scenario(c);
    let b = scenario(r);
    if a == b {
        return;
    }
    // dump both logs so they can be inspected with a real diff tool
    let slug: String = what
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '_' })
        .collect();
    let dir = std::env::temp_dir();
    let pc = dir.join(format!("cjson_diff_{slug}.C.txt"));
    let pr = dir.join(format!("cjson_diff_{slug}.RUST.txt"));
    let _ = std::fs::write(&pc, &a);
    let _ = std::fs::write(&pr, &b);

    let al: Vec<&str> = a.lines().collect();
    let bl: Vec<&str> = b.lines().collect();
    let mut msg = format!(
        "{what}: C and Rust behaviour differ\n  logs: {} vs {}\n",
        pc.display(),
        pr.display()
    );
    let n = al.len().max(bl.len());
    let mut shown = 0;
    for i in 0..n {
        let x = al.get(i).copied().unwrap_or("<missing>");
        let y = bl.get(i).copied().unwrap_or("<missing>");
        if x != y {
            let _ = write!(msg, "  line {i}:\n    C   : {x}\n    RUST: {y}\n");
            shown += 1;
            if shown >= 12 {
                let _ = write!(msg, "  ... (more differences suppressed)\n");
                break;
            }
        }
    }
    panic!("{msg}");
}

/* ------------------------------------------------------------------ */
/* deterministic RNG (xorshift64*)                                    */
/* ------------------------------------------------------------------ */

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng(if seed == 0 { 0x2545_F491_4F6C_DD1D } else { seed })
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
    pub fn range_i32(&mut self, lo: i32, hi: i32) -> i32 {
        let span = (hi as i64 - lo as i64 + 1) as u64;
        (lo as i64 + (self.next_u64() % span) as i64) as i32
    }
    pub fn bool(&mut self) -> bool {
        self.next_u64() & 1 == 1
    }
    /// Uniformly random bit pattern: deliberately produces NaNs, infinities and
    /// denormals as well as ordinary values.
    pub fn any_f64(&mut self) -> f64 {
        f64::from_bits(self.next_u64())
    }
    /// "Interesting but finite" doubles.
    pub fn nice_f64(&mut self) -> f64 {
        match self.below(8) {
            0 => self.range_i32(-1000, 1000) as f64,
            1 => self.range_i32(i32::MIN, i32::MAX) as f64,
            2 => (self.next_u64() as f64) / 7.0,
            3 => (self.range_i32(-1000, 1000) as f64) / 3.0,
            4 => 1.0e300 * (self.next_u64() as f64 / u64::MAX as f64),
            5 => 1.0e-300 * (self.next_u64() as f64 / u64::MAX as f64),
            6 => self.range_i32(-1000, 1000) as f64 * 0.5,
            _ => f64::from_bits(self.next_u64() & 0x7FEF_FFFF_FFFF_FFFF),
        }
    }
    /// Random byte string (never containing a NUL) of length `0..=max`.
    pub fn ascii(&mut self, max: usize) -> Vec<u8> {
        let n = self.below(max + 1);
        (0..n)
            .map(|_| match self.below(16) {
                0 => b'"',
                1 => b'\\',
                2 => b'\n',
                3 => b'\t',
                4 => b'\r',
                5 => 0x08,
                6 => 0x0c,
                7 => (self.below(31) + 1) as u8,
                8 => (self.below(0x80) + 0x80) as u8,
                9 => b'/',
                _ => (b' ' + self.below(0x5f) as u8) as u8,
            })
            .collect()
    }
}

/* ------------------------------------------------------------------ */
/* random JSON text generator                                         */
/* ------------------------------------------------------------------ */

fn json_escape(bytes: &[u8], out: &mut Vec<u8>) {
    out.push(b'"');
    for &b in bytes {
        match b {
            b'"' => out.extend_from_slice(b"\\\""),
            b'\\' => out.extend_from_slice(b"\\\\"),
            b'\n' => out.extend_from_slice(b"\\n"),
            b'\r' => out.extend_from_slice(b"\\r"),
            b'\t' => out.extend_from_slice(b"\\t"),
            0x08 => out.extend_from_slice(b"\\b"),
            0x0c => out.extend_from_slice(b"\\f"),
            0..=0x1f => out.extend_from_slice(format!("\\u{:04x}", b).as_bytes()),
            _ => out.push(b),
        }
    }
    out.push(b'"');
}

fn gen_value(rng: &mut Rng, depth: usize, out: &mut Vec<u8>) {
    let choice = if depth >= 4 { rng.below(5) } else { rng.below(8) };
    match choice {
        0 => out.extend_from_slice(b"null"),
        1 => out.extend_from_slice(b"true"),
        2 => out.extend_from_slice(b"false"),
        3 => {
            let n = rng.nice_f64();
            if n.is_finite() {
                out.extend_from_slice(format!("{:?}", n).as_bytes());
            } else {
                out.extend_from_slice(b"0");
            }
        }
        4 => {
            let s = rng.ascii(12);
            json_escape(&s, out);
        }
        5 | 6 => {
            let n = rng.below(5);
            out.push(b'[');
            for i in 0..n {
                if i > 0 {
                    out.push(b',');
                }
                if rng.bool() {
                    out.push(b' ');
                }
                gen_value(rng, depth + 1, out);
            }
            out.push(b']');
        }
        _ => {
            let n = rng.below(5);
            out.push(b'{');
            for i in 0..n {
                if i > 0 {
                    out.push(b',');
                }
                if rng.bool() {
                    out.extend_from_slice(b" \n\t");
                }
                let k = rng.ascii(8);
                json_escape(&k, out);
                out.push(b':');
                gen_value(rng, depth + 1, out);
            }
            out.push(b'}');
        }
    }
}

/// Random *syntactically valid* JSON document (no interior NUL bytes).
pub fn gen_json(rng: &mut Rng) -> Vec<u8> {
    let mut out = Vec::new();
    gen_value(rng, 0, &mut out);
    out
}

/// Random document with lots of whitespace and comments (for `cJSON_Minify`).
pub fn gen_minify_input(rng: &mut Rng) -> Vec<u8> {
    let mut out = Vec::new();
    let n = rng.below(6) + 1;
    for _ in 0..n {
        match rng.below(6) {
            0 => out.extend_from_slice(b"   \t\r\n "),
            1 => out.extend_from_slice(b"// line comment\n"),
            2 => out.extend_from_slice(b"/* block \n comment */"),
            3 => out.extend_from_slice(b"\"a string with // and /* inside \\\" \""),
            4 => out.push(b'/'),
            _ => out.extend_from_slice(&gen_json(rng)),
        }
    }
    out.retain(|&b| b != 0);
    out
}
