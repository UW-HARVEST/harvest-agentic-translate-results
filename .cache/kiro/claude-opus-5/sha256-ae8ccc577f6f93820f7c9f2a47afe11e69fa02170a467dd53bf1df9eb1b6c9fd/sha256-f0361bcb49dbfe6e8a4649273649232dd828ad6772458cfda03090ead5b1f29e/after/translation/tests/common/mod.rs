//! Shared differential-test harness.
//!
//! Loads BOTH the C `.so` and the Rust `.so` through `libloading` and exposes
//! every symbol only through `dlsym`.  Rust functions are never called
//! directly, so the `#[no_mangle]` export wrappers are under test too.
#![allow(dead_code, unused_unsafe, unsafe_op_in_unsafe_fn)]

use libloading::{Library, Symbol};
use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_double, c_int, c_longlong, c_void};
use std::path::PathBuf;
use std::sync::OnceLock;

pub const JSON_OBJECT: c_int = 0;
pub const JSON_ARRAY: c_int = 1;
pub const JSON_STRING: c_int = 2;
pub const JSON_INTEGER: c_int = 3;
pub const JSON_REAL: c_int = 4;
pub const JSON_TRUE: c_int = 5;
pub const JSON_FALSE: c_int = 6;
pub const JSON_NULL: c_int = 7;

// decoder flags
pub const JSON_REJECT_DUPLICATES: usize = 0x1;
pub const JSON_DISABLE_EOF_CHECK: usize = 0x2;
pub const JSON_DECODE_ANY: usize = 0x4;
pub const JSON_DECODE_INT_AS_REAL: usize = 0x8;
pub const JSON_ALLOW_NUL: usize = 0x10;

// encoder flags
pub const JSON_COMPACT: usize = 0x20;
pub const JSON_ENSURE_ASCII: usize = 0x40;
pub const JSON_SORT_KEYS: usize = 0x80;
pub const JSON_PRESERVE_ORDER: usize = 0x100;
pub const JSON_ENCODE_ANY: usize = 0x200;
pub const JSON_ESCAPE_SLASH: usize = 0x400;
pub const JSON_EMBED: usize = 0x10000;
pub const fn json_indent(n: usize) -> usize {
    n & 0x1F
}
pub const fn json_real_precision(n: usize) -> usize {
    (n & 0x1F) << 11
}

// pack/unpack flags
pub const JSON_VALIDATE_ONLY: usize = 0x1;
pub const JSON_STRICT: usize = 0x2;

/// Fixed hashtable seed so object iteration order is identical in both libs.
pub const FIXED_SEED: usize = 0x5EED_1234;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct JsonT {
    pub typ: c_int,
    pub refcount: usize,
}

pub const ERR_TEXT_LEN: usize = 160;
pub const ERR_SRC_LEN: usize = 80;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct JsonError {
    pub line: c_int,
    pub column: c_int,
    pub position: c_int,
    pub source: [c_char; ERR_SRC_LEN],
    pub text: [c_char; ERR_TEXT_LEN],
}

impl JsonError {
    pub fn new() -> Self {
        JsonError {
            line: 0,
            column: 0,
            position: 0,
            source: [0; ERR_SRC_LEN],
            text: [0; ERR_TEXT_LEN],
        }
    }
    /// Full raw bytes, so the trailing error-code byte is compared too.
    pub fn raw(&self) -> (c_int, c_int, c_int, Vec<u8>, Vec<u8>) {
        (
            self.line,
            self.column,
            self.position,
            self.source.iter().map(|c| *c as u8).collect(),
            self.text.iter().map(|c| *c as u8).collect(),
        )
    }
    pub fn text_str(&self) -> String {
        let bytes: Vec<u8> = self
            .text
            .iter()
            .take(ERR_TEXT_LEN - 1)
            .map(|c| *c as u8)
            .take_while(|b| *b != 0)
            .collect();
        String::from_utf8_lossy(&bytes).into_owned()
    }
    pub fn code(&self) -> u8 {
        self.text[ERR_TEXT_LEN - 1] as u8
    }
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct StrBuffer {
    pub value: *mut c_char,
    pub length: usize,
    pub size: usize,
}

impl StrBuffer {
    pub fn zeroed() -> Self {
        StrBuffer {
            value: std::ptr::null_mut(),
            length: 0,
            size: 0,
        }
    }
}

/// `hashtable_t` from `hashtable.h`: size, buckets*, order, list{prev,next},
/// ordered_list{prev,next} => 7 pointer-sized words.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct HashTable {
    pub words: [usize; 7],
}
impl HashTable {
    pub fn zeroed() -> Self {
        HashTable { words: [0; 7] }
    }
}

pub struct Lib {
    pub lib: Library,
    pub tag: &'static str,
}

fn workspace_root() -> PathBuf {
    // CARGO_MANIFEST_DIR == <root>/translation
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf()
}

/// Copy each `.so` to a unique file name so `dlopen` cannot dedupe them.
fn staged(src: PathBuf, name: &str) -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join("difftest");
    std::fs::create_dir_all(&dir).unwrap();
    let dst = dir.join(name);
    assert!(src.exists(), "missing shared library: {}", src.display());
    // Only copy when changed, so concurrently-running test binaries don't
    // truncate a library another process has mapped.
    let need = match (std::fs::metadata(&src), std::fs::metadata(&dst)) {
        (Ok(a), Ok(b)) => a.len() != b.len() || a.modified().ok() > b.modified().ok(),
        _ => true,
    };
    if need {
        let tmp = dir.join(format!("{}.{}.tmp", name, std::process::id()));
        std::fs::copy(&src, &tmp).unwrap();
        std::fs::rename(&tmp, &dst).unwrap();
    }
    dst
}

static LIBS: OnceLock<(Lib, Lib)> = OnceLock::new();

/// The two libraries under test: `(c, rust)`.
pub fn libs() -> &'static (Lib, Lib) {
    LIBS.get_or_init(|| {
        let root = workspace_root();
        let cpath = staged(root.join("c_src/build/libjansson.so"), "libjansson_c.so");
        let rpath = staged(
            root.join("translation/target/release/libjansson.so"),
            "libjansson_rs.so",
        );
        let c = Lib {
            lib: unsafe { Library::new(&cpath) }.expect("dlopen C lib"),
            tag: "C",
        };
        let r = Lib {
            lib: unsafe { Library::new(&rpath) }.expect("dlopen Rust lib"),
            tag: "RUST",
        };
        // Deterministic hash seed in both, before any object exists.
        for l in [&c, &r] {
            let f: Symbol<unsafe extern "C" fn(usize)> =
                unsafe { l.lib.get(b"json_object_seed\0") }.unwrap();
            unsafe { f(FIXED_SEED) };
        }
        (c, r)
    })
}

pub fn both() -> (&'static Lib, &'static Lib) {
    let (c, r) = libs();
    (c, r)
}

#[macro_export]
macro_rules! sym {
    ($lib:expr, $name:literal, ($($at:ty),*) -> $rt:ty) => {{
        let f: libloading::Symbol<unsafe extern "C" fn($($at),*) -> $rt> = unsafe {
            $lib.lib
                .get(concat!($name, "\0").as_bytes())
                .expect(concat!("dlsym failed: ", $name))
        };
        f
    }};
    ($lib:expr, $name:literal, ($($at:ty),*)) => {{
        let f: libloading::Symbol<unsafe extern "C" fn($($at),*)> = unsafe {
            $lib.lib
                .get(concat!($name, "\0").as_bytes())
                .expect(concat!("dlsym failed: ", $name))
        };
        f
    }};
}

// ---------------------------------------------------------------- convenience

pub fn cstr(s: &str) -> CString {
    CString::new(s).unwrap()
}

/// Read a NUL-terminated string produced by the library and free it with that
/// library's own `jsonp_free`.
pub unsafe fn take_cstring(l: &Lib, p: *mut c_char) -> Option<Vec<u8>> {
    if p.is_null() {
        return None;
    }
    let out = CStr::from_ptr(p).to_bytes().to_vec();
    let free = sym!(l, "jsonp_free", (*mut c_void));
    free(p as *mut c_void);
    Some(out)
}

/// `json_dumps` on one library, returning raw bytes.
pub unsafe fn dumps(l: &Lib, v: *mut JsonT, flags: usize) -> Option<Vec<u8>> {
    let f = sym!(l, "json_dumps", (*const JsonT, usize) -> *mut c_char);
    let p = f(v, flags);
    take_cstring(l, p)
}

pub unsafe fn decref(l: &Lib, v: *mut JsonT) {
    if v.is_null() {
        return;
    }
    let jt = &mut *v;
    if jt.refcount == usize::MAX {
        return;
    }
    jt.refcount -= 1;
    if jt.refcount == 0 {
        let f = sym!(l, "json_delete", (*mut JsonT));
        f(v);
    }
}

pub unsafe fn incref(l: &Lib, v: *mut JsonT) -> *mut JsonT {
    let _ = l;
    if !v.is_null() {
        let jt = &mut *v;
        if jt.refcount != usize::MAX {
            jt.refcount += 1;
        }
    }
    v
}

pub unsafe fn loads(l: &Lib, text: &[u8], flags: usize) -> (*mut JsonT, JsonError) {
    let f = sym!(l, "json_loadb", (*const c_char, usize, usize, *mut JsonError) -> *mut JsonT);
    let mut err = JsonError::new();
    let v = f(
        text.as_ptr() as *const c_char,
        text.len(),
        flags,
        &mut err as *mut JsonError,
    );
    (v, err)
}

// ------------------------------------------------------------------- PRNG

pub struct Rng(pub u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Rng(seed ^ 0x9E3779B97F4A7C15)
    }
    pub fn next_u64(&mut self) -> u64 {
        // splitmix64
        self.0 = self.0.wrapping_add(0x9E3779B97F4A7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D049BB133111EB);
        z ^ (z >> 31)
    }
    pub fn next_u32(&mut self) -> u32 {
        (self.next_u64() >> 32) as u32
    }
    /// uniform in `0..n`
    pub fn below(&mut self, n: usize) -> usize {
        if n == 0 {
            0
        } else {
            (self.next_u64() % n as u64) as usize
        }
    }
    pub fn f64(&mut self) -> f64 {
        loop {
            let b = f64::from_bits(self.next_u64());
            if b.is_finite() {
                return b;
            }
        }
    }
    pub fn i64(&mut self) -> i64 {
        self.next_u64() as i64
    }
    pub fn bool(&mut self) -> bool {
        self.next_u64() & 1 == 1
    }
}

// ------------------------------------------------------- JSON text corpus

/// A JSON value we can serialize deterministically ourselves, so the *input*
/// text handed to both libraries is byte-identical.
#[derive(Clone, Debug)]
pub enum V {
    Null,
    Bool(bool),
    Int(i64),
    Real(f64),
    Str(String),
    Arr(Vec<V>),
    Obj(Vec<(String, V)>),
}

impl V {
    pub fn to_text(&self) -> String {
        let mut s = String::new();
        self.write(&mut s);
        s
    }
    fn write(&self, out: &mut String) {
        match self {
            V::Null => out.push_str("null"),
            V::Bool(true) => out.push_str("true"),
            V::Bool(false) => out.push_str("false"),
            V::Int(i) => out.push_str(&i.to_string()),
            V::Real(f) => {
                let mut t = format!("{:?}", f);
                if !t.contains('.') && !t.contains('e') && !t.contains('E') {
                    t.push_str(".0");
                }
                out.push_str(&t);
            }
            V::Str(s) => write_json_string(s, out),
            V::Arr(a) => {
                out.push('[');
                for (i, v) in a.iter().enumerate() {
                    if i > 0 {
                        out.push(',');
                    }
                    v.write(out);
                }
                out.push(']');
            }
            V::Obj(m) => {
                out.push('{');
                for (i, (k, v)) in m.iter().enumerate() {
                    if i > 0 {
                        out.push(',');
                    }
                    write_json_string(k, out);
                    out.push(':');
                    v.write(out);
                }
                out.push('}');
            }
        }
    }
}

fn write_json_string(s: &str, out: &mut String) {
    out.push('"');
    for ch in s.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\u{8}' => out.push_str("\\b"),
            '\u{c}' => out.push_str("\\f"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
}

const INTERESTING_STRINGS: &[&str] = &[
    "",
    "a",
    "hello world",
    "with \"quotes\" and \\backslash\\",
    "tab\there\nnewline\rcr",
    "ctrl\u{1}\u{1f}",
    "slash / and //",
    "caf\u{e9}",
    "\u{4e2d}\u{6587}\u{6d4b}\u{8bd5}",
    "emoji \u{1f600}\u{1f4a9}",
    "\u{7f}\u{80}\u{7ff}\u{800}\u{ffff}\u{10000}\u{10ffff}",
    "\u{fffd}",
    "0123456789012345678901234567890123456789",
    "\u{2028}\u{2029}",
];

const INTERESTING_INTS: &[i64] = &[
    0,
    1,
    -1,
    2,
    -2,
    9,
    10,
    99,
    100,
    127,
    128,
    -128,
    255,
    256,
    32767,
    -32768,
    65535,
    2147483647,
    -2147483648,
    4294967295,
    i64::MAX,
    i64::MIN,
    i64::MAX - 1,
    i64::MIN + 1,
];

const INTERESTING_REALS: &[f64] = &[
    0.0,
    -0.0,
    1.0,
    -1.0,
    0.5,
    -0.5,
    0.1,
    1.0 / 3.0,
    2.0 / 3.0,
    1e-5,
    1e-4,
    1e-3,
    1e16,
    1e17,
    1e-300,
    1e300,
    1.7976931348623157e308,
    2.2250738585072014e-308,
    5e-324,
    3.141592653589793,
    2.718281828459045,
    1234567890.123456,
    -9007199254740993.0,
    123456789012345678.0,
    1e21,
    1e-21,
    99999999999999999.0,
];

pub fn interesting_strings() -> &'static [&'static str] {
    INTERESTING_STRINGS
}
pub fn interesting_ints() -> &'static [i64] {
    INTERESTING_INTS
}
pub fn interesting_reals() -> &'static [f64] {
    INTERESTING_REALS
}

fn rand_string(rng: &mut Rng) -> String {
    if rng.below(3) == 0 {
        return INTERESTING_STRINGS[rng.below(INTERESTING_STRINGS.len())].to_string();
    }
    let n = rng.below(12);
    let mut s = String::new();
    for _ in 0..n {
        let pick = rng.below(10);
        let c = match pick {
            0..=4 => char::from(b'a' + rng.below(26) as u8),
            5 => char::from(b'0' + rng.below(10) as u8),
            6 => ['"', '\\', '/', '\n', '\t', '\r', '\u{8}', '\u{c}'][rng.below(8)],
            7 => char::from_u32(0x80 + rng.below(0x700) as u32).unwrap_or('x'),
            8 => char::from_u32(0x1000 + rng.below(0xC000) as u32).unwrap_or('x'),
            _ => char::from_u32(0x10000 + rng.below(0xFFFF) as u32).unwrap_or('x'),
        };
        s.push(c);
    }
    s
}

pub fn rand_value(rng: &mut Rng, depth: usize) -> V {
    let leaf = depth == 0;
    let pick = if leaf { rng.below(5) } else { rng.below(7) };
    match pick {
        0 => V::Null,
        1 => V::Bool(rng.bool()),
        2 => {
            if rng.below(2) == 0 {
                V::Int(INTERESTING_INTS[rng.below(INTERESTING_INTS.len())])
            } else {
                V::Int(rng.i64())
            }
        }
        3 => {
            if rng.below(2) == 0 {
                V::Real(INTERESTING_REALS[rng.below(INTERESTING_REALS.len())])
            } else {
                V::Real(rng.f64())
            }
        }
        4 => V::Str(rand_string(rng)),
        5 => {
            let n = rng.below(5);
            V::Arr((0..n).map(|_| rand_value(rng, depth - 1)).collect())
        }
        _ => {
            let n = rng.below(5);
            V::Obj(
                (0..n)
                    .map(|i| (format!("k{}{}", i, rand_string(rng)), rand_value(rng, depth - 1)))
                    .collect(),
            )
        }
    }
}

/// Curated + randomized corpus of JSON texts (all top-level-any tolerant).
pub fn corpus(n_random: usize, seed: u64) -> Vec<String> {
    let mut out: Vec<String> = vec![
        "{}".into(),
        "[]".into(),
        "[[]]".into(),
        "{\"a\":{}}".into(),
        "[1,2,3]".into(),
        "[1, 2 , 3]".into(),
        " \t\r\n[1]\r\n ".into(),
        "{\"a\":1,\"b\":2,\"c\":3}".into(),
        "{\"z\":1,\"y\":2,\"x\":3,\"a\":4,\"m\":5}".into(),
        "{\"a\":1,\"a\":2}".into(),
        "[null,true,false]".into(),
        "[0,-0,1,-1]".into(),
        "[1e0,1E0,1e+0,1e-0,1.0e1]".into(),
        "[0.0,-0.0,1.5,-1.5]".into(),
        "[9223372036854775807,-9223372036854775808]".into(),
        "[9223372036854775808]".into(),
        "[-9223372036854775809]".into(),
        "[1e400]".into(),
        "[1e-400]".into(),
        "[1e308,1e-308,5e-324]".into(),
        "[\"\"]".into(),
        "[\"\\u0041\\u00e9\\u4e2d\\ud83d\\ude00\"]".into(),
        "[\"\\\"\\\\\\/\\b\\f\\n\\r\\t\"]".into(),
        "[\"caf\u{e9} \u{4e2d}\u{6587} \u{1f600}\"]".into(),
        "[\"\\u0000\"]".into(),
        "[\"a\\u0000b\"]".into(),
        "[\"/\",\"//\",\"a/b\"]".into(),
        "{\"\":1}".into(),
        "{\"\u{4e2d}\":\"\u{6587}\"}".into(),
        "[[[[[[[[[[1]]]]]]]]]]".into(),
        "{\"a\":{\"b\":{\"c\":{\"d\":[1,2,{\"e\":null}]}}}}".into(),
        "[{\"a\":[1,{\"b\":[2]}]}]".into(),
        "1".into(),
        "-1".into(),
        "1.5".into(),
        "\"str\"".into(),
        "true".into(),
        "false".into(),
        "null".into(),
        "[1] trailing".into(),
        "[1]{}".into(),
        "[0.1,0.2,0.3,1e-5,1e-4,1e16,1e17,123456789012345678]".into(),
        "[1.0,2.0,100.0,1e21,1e-21]".into(),
    ];
    // wide/large shapes
    {
        let items: Vec<String> = (0..200).map(|i| i.to_string()).collect();
        out.push(format!("[{}]", items.join(",")));
        let pairs: Vec<String> = (0..200).map(|i| format!("\"k{:03}\":{}", i, i)).collect();
        out.push(format!("{{{}}}", pairs.join(",")));
        let pairs: Vec<String> = (0..200)
            .map(|i| format!("\"k{:03}\":{}", 199 - i, i))
            .collect();
        out.push(format!("{{{}}}", pairs.join(",")));
    }
    // deep nesting around the 2048 parser depth limit
    for d in [1usize, 2, 10, 100, 1000, 2046, 2047, 2048, 2049, 2100] {
        out.push(format!("{}1{}", "[".repeat(d), "]".repeat(d)));
        out.push(format!(
            "{}1{}",
            "{\"a\":".repeat(d),
            "}".repeat(d)
        ));
    }
    let mut rng = Rng::new(seed);
    for _ in 0..n_random {
        out.push(rand_value(&mut rng, 4).to_text());
    }
    out
}

/// Inputs that are outright invalid, for the error-path phase.
pub fn invalid_corpus() -> Vec<String> {
    vec![
        "".into(),
        " ".into(),
        "\n\t ".into(),
        "{".into(),
        "}".into(),
        "[".into(),
        "]".into(),
        "[,]".into(),
        "[1,]".into(),
        "[1 2]".into(),
        "{,}".into(),
        "{\"a\"}".into(),
        "{\"a\":}".into(),
        "{\"a\":1,}".into(),
        "{\"a\" 1}".into(),
        "{1:2}".into(),
        "{\"a\":1 \"b\":2}".into(),
        "\"unterminated".into(),
        "\"line\nbreak\"".into(),
        "\"ctrl\u{1}\"".into(),
        "\"\\q\"".into(),
        "\"\\u00\"".into(),
        "\"\\uZZZZ\"".into(),
        "\"\\ud834\"".into(),
        "\"\\ud834x\"".into(),
        "\"\\udd1e\"".into(),
        "\"\\ud834\\u0041\"".into(),
        "\"\\u0000\"".into(),
        "01".into(),
        "-".into(),
        "-x".into(),
        "1.".into(),
        "1e".into(),
        "1e+".into(),
        ".1".into(),
        "+1".into(),
        "tru".into(),
        "fals".into(),
        "nul".into(),
        "TRUE".into(),
        "NaN".into(),
        "Infinity".into(),
        "-Infinity".into(),
        "'single'".into(),
        "[1] x".into(),
        "{} {}".into(),
        "[1,2".into(),
        "{\"a\":1".into(),
        "\u{feff}[1]".into(),
        "[\"\\ud800\\ud800\"]".into(),
        "1 2".into(),
        "//comment\n[1]".into(),
        "/*c*/[1]".into(),
        "[00]".into(),
        "[0x1]".into(),
        "[1..2]".into(),
        "[--1]".into(),
        "[1e1e1]".into(),
        "9223372036854775808".into(),
        "-9223372036854775809".into(),
        "1e999999".into(),
        "{\"a\":1,\"a\":2}".into(),
        format!("{}1{}", "[".repeat(2049), "]".repeat(2049)),
        format!("{}1{}", "[".repeat(5000), "]".repeat(5000)),
    ]
}

/// Byte sequences that are not valid UTF-8 (or contain NUL), for load tests.
pub fn invalid_utf8_corpus() -> Vec<Vec<u8>> {
    vec![
        b"[\"\x80\"]".to_vec(),
        b"[\"\xC0\x80\"]".to_vec(),
        b"[\"\xC1\xBF\"]".to_vec(),
        b"[\"\xE0\x80\x80\"]".to_vec(),
        b"[\"\xF0\x80\x80\x80\"]".to_vec(),
        b"[\"\xF5\x80\x80\x80\"]".to_vec(),
        b"[\"\xFF\"]".to_vec(),
        b"[\"\xFE\xFE\"]".to_vec(),
        b"[\"\xED\xA0\x80\"]".to_vec(), // surrogate
        b"[\"\xC2\"]".to_vec(),         // truncated
        b"[\"\xE2\x82\"]".to_vec(),
        b"[\"a\x00b\"]".to_vec(),
        b"[\x00]".to_vec(),
        b"\x00".to_vec(),
        vec![0xEF, 0xBB, 0xBF, b'[', b'1', b']'],
    ]
}

pub fn all_decode_flag_combos() -> Vec<usize> {
    (0..32usize).collect()
}

pub fn assert_bytes_eq(what: &str, c: &Option<Vec<u8>>, r: &Option<Vec<u8>>) {
    if c != r {
        panic!(
            "{}\n  C   : {:?}\n  RUST: {:?}",
            what,
            c.as_ref().map(|v| String::from_utf8_lossy(v).into_owned()),
            r.as_ref().map(|v| String::from_utf8_lossy(v).into_owned()),
        );
    }
}

pub fn assert_err_eq(what: &str, c: &JsonError, r: &JsonError) {
    if c.raw() != r.raw() {
        // Raw bytes matter: the C library can leave an embedded NUL (and
        // content after it) inside `text`, so compare/report the full buffers.
        eprintln!(
            "RAW C   text: {:02x?}\nRAW RUST text: {:02x?}\nRAW C   src: {:02x?}\nRAW RUST src: {:02x?}",
            c.text.iter().map(|x| *x as u8).collect::<Vec<u8>>(),
            r.text.iter().map(|x| *x as u8).collect::<Vec<u8>>(),
            c.source.iter().map(|x| *x as u8).collect::<Vec<u8>>(),
            r.source.iter().map(|x| *x as u8).collect::<Vec<u8>>(),
        );
        panic!(
            "{}\n  C   : line={} col={} pos={} code={} src={:?} text={:?}\n\
             \x20 RUST: line={} col={} pos={} code={} src={:?} text={:?}",
            what,
            c.line,
            c.column,
            c.position,
            c.code(),
            src_str(c),
            c.text_str(),
            r.line,
            r.column,
            r.position,
            r.code(),
            src_str(r),
            r.text_str(),
        );
    }
}

pub fn src_str(e: &JsonError) -> String {
    let bytes: Vec<u8> = e
        .source
        .iter()
        .map(|c| *c as u8)
        .take_while(|b| *b != 0)
        .collect();
    String::from_utf8_lossy(&bytes).into_owned()
}

pub type Pfn = *const c_void;
pub type CInt = c_int;
pub type CLL = c_longlong;
pub type CDouble = c_double;
