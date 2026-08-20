//! Code shared between the differential tests (`tests/*.rs`) and the isolated
//! worker process (`examples/diffworker.rs`).
//!
//! Everything here talks to a `pinflate` implementation **only** through a
//! `dlopen`ed shared object: the C reference `.so` and the Rust `.so` are
//! loaded the exact same way, through `libloading`, so the Rust `#[no_mangle]`
//! export wrappers are on the critical path just like an external C caller
//! would see them.

#![allow(dead_code)]

use std::alloc::{alloc, dealloc, Layout};
use std::ffi::{c_char, c_int, c_void};

// ---------------------------------------------------------------------------
// The exported ABI, as declared in c_src/include/lib.h and c_src/src/lib.c
// ---------------------------------------------------------------------------

pub type PinflateFn = unsafe extern "C" fn(*mut c_void, c_int, *mut c_void, c_int) -> c_int;

/// One writable exported table. `elem` is the element size in bytes so a u32
/// table round-trips through hex as little-endian bytes, exactly as it sits in
/// memory.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct TableSpec {
    pub key: &'static str,
    pub symbol: &'static str,
    pub len_bytes: usize,
}

pub const TABLES: &[TableSpec] = &[
    TableSpec { key: "ft", symbol: "cp_fixed_table", len_bytes: 288 + 32 },
    TableSpec { key: "po", symbol: "cp_permutation_order", len_bytes: 19 },
    TableSpec { key: "le", symbol: "cp_len_extra_bits", len_bytes: 29 + 2 },
    TableSpec { key: "lb", symbol: "cp_len_base", len_bytes: (29 + 2) * 4 },
    TableSpec { key: "de", symbol: "cp_dist_extra_bits", len_bytes: 30 + 2 },
    TableSpec { key: "db", symbol: "cp_dist_base", len_bytes: (30 + 2) * 4 },
];

pub fn table_by_key(key: &str) -> &'static TableSpec {
    TABLES.iter().find(|t| t.key == key).expect("unknown table key")
}

/// A sentinel string used to prove that a *successful* `pinflate` does not
/// touch `cp_error_reason` (the C only ever assigns it, never clears it).
pub static SENTINEL: [u8; 21] = *b"SENTINEL-UNTOUCHED-X\0";

// ---------------------------------------------------------------------------
// Loaded library
// ---------------------------------------------------------------------------

pub struct Lib {
    pub path: String,
    _lib: libloading::Library,
    pinflate: PinflateFn,
    err: *mut *const c_char,
    tables: Vec<(&'static TableSpec, *mut u8, Vec<u8>)>,
}

impl Lib {
    pub fn open(path: &str) -> Lib {
        unsafe {
            let lib = libloading::Library::new(path)
                .unwrap_or_else(|e| panic!("dlopen({path}) failed: {e}"));
            let pinflate: libloading::Symbol<PinflateFn> = lib
                .get(b"pinflate\0")
                .unwrap_or_else(|e| panic!("dlsym(pinflate) in {path} failed: {e}"));
            let pinflate = *pinflate;
            let err: libloading::Symbol<*mut *const c_char> = lib
                .get(b"cp_error_reason\0")
                .unwrap_or_else(|e| panic!("dlsym(cp_error_reason) in {path} failed: {e}"));
            let err = *err;
            let mut tables = Vec::new();
            for spec in TABLES {
                let mut name = spec.symbol.as_bytes().to_vec();
                name.push(0);
                let sym: libloading::Symbol<*mut u8> = lib
                    .get(&name)
                    .unwrap_or_else(|e| panic!("dlsym({}) in {path} failed: {e}", spec.symbol));
                let ptr = *sym;
                let pristine = std::slice::from_raw_parts(ptr, spec.len_bytes).to_vec();
                tables.push((spec, ptr, pristine));
            }
            Lib { path: path.to_string(), _lib: lib, pinflate, err, tables }
        }
    }

    /// Byte-exact snapshot of every exported table (Phase D data check).
    pub fn table_bytes(&self, key: &str) -> Vec<u8> {
        let (spec, ptr, _) = self
            .tables
            .iter()
            .find(|(s, _, _)| s.key == key)
            .expect("unknown table");
        unsafe { std::slice::from_raw_parts(*ptr, spec.len_bytes).to_vec() }
    }

    fn restore_tables(&self) {
        unsafe {
            for (spec, ptr, pristine) in &self.tables {
                std::ptr::copy_nonoverlapping(pristine.as_ptr(), *ptr, spec.len_bytes);
            }
        }
    }

    fn set_table(&self, key: &str, bytes: &[u8]) {
        let (spec, ptr, _) = self
            .tables
            .iter()
            .find(|(s, _, _)| s.key == key)
            .expect("unknown table");
        assert_eq!(bytes.len(), spec.len_bytes, "table {} wrong length", spec.symbol);
        unsafe { std::ptr::copy_nonoverlapping(bytes.as_ptr(), *ptr, spec.len_bytes) };
    }

    /// Runs one case in *this* process. Returns `Outcome::Ret`.
    pub fn run(&self, case: &Case) -> Outcome {
        unsafe {
            self.restore_tables();
            for (key, bytes) in &case.tables {
                self.set_table(key, bytes);
            }

            // Deterministic input buffer: `in_off` bytes of filler, the stream,
            // then `in_pad` bytes of a fixed pattern so that the C code's
            // deliberate reads past `in_bytes` (see ERRORS.md E21/E23) land on
            // known bytes in both libraries.
            let in_total = case.in_off + case.data.len() + case.in_pad;
            let in_buf = Buf::new(in_total.max(1));
            for i in 0..in_total {
                *in_buf.at(i) = INPUT_FILL[i % INPUT_FILL.len()];
            }
            for (i, b) in case.data.iter().enumerate() {
                *in_buf.at(case.in_off + i) = *b;
            }

            // Output buffer: `out_off` bytes of filler, `out_size` usable
            // bytes, then `out_pad` bytes of filler. The whole buffer is
            // compared, so writes past `out_bytes` (E24) are caught.
            let out_usable = if case.out_size > 0 { case.out_size as usize } else { 0 };
            let out_total = case.out_off + out_usable + case.out_pad;
            let out_buf = Buf::new(out_total.max(1));
            for i in 0..out_total {
                *out_buf.at(i) = 0xCD;
            }

            *self.err = if case.err_preset {
                SENTINEL.as_ptr() as *const c_char
            } else {
                std::ptr::null()
            };

            let in_ptr = if case.null_in {
                std::ptr::null_mut()
            } else {
                in_buf.at(case.in_off) as *mut c_void
            };
            let out_ptr = if case.null_out {
                std::ptr::null_mut()
            } else {
                out_buf.at(case.out_off) as *mut c_void
            };

            let ret = (self.pinflate)(in_ptr, case.in_len, out_ptr, case.out_size);

            let err_ptr = *self.err;
            let err = if err_ptr.is_null() {
                None
            } else {
                let mut v = Vec::new();
                let mut p = err_ptr as *const u8;
                // bounded so a garbage pointer cannot run away
                for _ in 0..4096 {
                    let b = *p;
                    if b == 0 {
                        break;
                    }
                    v.push(b);
                    p = p.add(1);
                }
                Some(v)
            };

            let out = (0..out_total).map(|i| *out_buf.at(i)).collect::<Vec<u8>>();
            self.restore_tables();
            Outcome::Ret { ret, err, out }
        }
    }
}

/// 16-byte-aligned raw buffer, so that `ptr + off` has a *deterministic*
/// alignment mod 4. `pinflate` branches on `(size_t)in & 3` (`first_bytes`), so
/// both libraries must see the same alignment for the comparison to mean
/// anything.
struct Buf {
    ptr: *mut u8,
    layout: Layout,
}

impl Buf {
    fn new(len: usize) -> Buf {
        let layout = Layout::from_size_align(len, 16).unwrap();
        let ptr = unsafe { alloc(layout) };
        assert!(!ptr.is_null());
        Buf { ptr, layout }
    }
    #[inline]
    fn at(&self, i: usize) -> *mut u8 {
        unsafe { self.ptr.add(i) }
    }
}

impl Drop for Buf {
    fn drop(&mut self) {
        unsafe { dealloc(self.ptr, self.layout) }
    }
}

pub const INPUT_FILL: [u8; 16] =
    [0x00, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88, 0x99, 0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF];

// ---------------------------------------------------------------------------
// Case / Outcome
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Case {
    pub label: String,
    pub data: Vec<u8>,
    /// the `in_bytes` argument; normally `data.len()`
    pub in_len: c_int,
    /// the `out_bytes` argument
    pub out_size: c_int,
    pub in_off: usize,
    pub out_off: usize,
    pub in_pad: usize,
    pub out_pad: usize,
    pub null_in: bool,
    pub null_out: bool,
    pub err_preset: bool,
    pub tables: Vec<(String, Vec<u8>)>,
}

impl Case {
    pub fn new(label: &str, data: &[u8], out_size: i32) -> Case {
        Case {
            label: label.to_string(),
            data: data.to_vec(),
            in_len: data.len() as c_int,
            out_size,
            in_off: 0,
            out_off: 0,
            in_pad: 512,
            out_pad: 512,
            null_in: false,
            null_out: false,
            err_preset: false,
            tables: Vec::new(),
        }
    }
    pub fn in_off(mut self, v: usize) -> Case {
        self.in_off = v;
        self
    }
    pub fn out_off(mut self, v: usize) -> Case {
        self.out_off = v;
        self
    }
    pub fn in_len(mut self, v: i32) -> Case {
        self.in_len = v;
        self
    }
    pub fn in_pad(mut self, v: usize) -> Case {
        self.in_pad = v;
        self
    }
    pub fn out_pad(mut self, v: usize) -> Case {
        self.out_pad = v;
        self
    }
    pub fn null_in(mut self) -> Case {
        self.null_in = true;
        self
    }
    pub fn null_out(mut self) -> Case {
        self.null_out = true;
        self
    }
    pub fn err_preset(mut self) -> Case {
        self.err_preset = true;
        self
    }
    pub fn table(mut self, key: &str, bytes: Vec<u8>) -> Case {
        self.tables.push((key.to_string(), bytes));
        self
    }

    pub fn encode(&self) -> String {
        let mut s = String::new();
        s.push_str(&hex(&self.data));
        s.push(' ');
        s.push_str(&format!(
            "{} {} {} {} {} {} {} {} {}",
            self.in_len,
            self.out_size,
            self.in_off,
            self.out_off,
            self.in_pad,
            self.out_pad,
            self.null_in as u8,
            self.null_out as u8,
            self.err_preset as u8
        ));
        s.push(' ');
        if self.tables.is_empty() {
            s.push('-');
        } else {
            let parts: Vec<String> = self
                .tables
                .iter()
                .map(|(k, v)| format!("{}:{}", k, hex(v)))
                .collect();
            s.push_str(&parts.join(";"));
        }
        s
    }

    pub fn decode(line: &str) -> Case {
        let f: Vec<&str> = line.split(' ').collect();
        assert_eq!(f.len(), 11, "bad case encoding: {line}");
        let tables = if f[10] == "-" {
            Vec::new()
        } else {
            f[10]
                .split(';')
                .map(|p| {
                    let (k, v) = p.split_once(':').expect("bad table spec");
                    (k.to_string(), unhex(v))
                })
                .collect()
        };
        Case {
            label: String::new(),
            data: unhex(f[0]),
            in_len: f[1].parse().unwrap(),
            out_size: f[2].parse().unwrap(),
            in_off: f[3].parse().unwrap(),
            out_off: f[4].parse().unwrap(),
            in_pad: f[5].parse().unwrap(),
            out_pad: f[6].parse().unwrap(),
            null_in: f[7] == "1",
            null_out: f[8] == "1",
            err_preset: f[9] == "1",
            tables,
        }
    }
}

#[derive(Clone, PartialEq, Eq)]
pub enum Outcome {
    /// `pinflate` returned normally.
    Ret {
        ret: c_int,
        err: Option<Vec<u8>>,
        out: Vec<u8>,
    },
    /// The process died from `signal` (6 = SIGABRT from a failed `assert()`,
    /// 11 = SIGSEGV, 14 = SIGALRM i.e. the call did not terminate).
    ///
    /// `diag` is the normalised `assert()` diagnostic scraped from the worker's
    /// stderr, e.g. `lib.c:95: cp_ptr: Assertion `!(s->bits_left & 7)' failed.`
    /// Comparing it -- not just the signal number -- is what makes the error
    /// rows precise: two libraries that both `abort()` for *different* reasons
    /// are not equivalent, and `SIGABRT == SIGABRT` alone would hide that.
    Signal { sig: i32, diag: Option<String> },
}

/// Extracts `lib.c:<line>: <func>: Assertion `<expr>' failed.` from a stderr
/// blob, dropping the leading program name (which necessarily differs between
/// the C worker and the Rust worker).
pub fn extract_assertion(stderr: &str) -> Option<String> {
    let mut best = None;
    for line in stderr.lines() {
        if !line.contains("Assertion") {
            continue;
        }
        // glibc: "<prog>: <path>/lib.c:95: cp_ptr: Assertion `...' failed."
        // Rust:  "pinflate: c_src/src/lib.c:95: cp_ptr: Assertion `...' failed."
        if let Some(i) = line.find("lib.c:") {
            best = Some(line[i..].trim().to_string());
        }
    }
    best
}

impl std::fmt::Debug for Outcome {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Outcome::Ret { ret, err, out } => {
                let e = err
                    .as_ref()
                    .map(|v| String::from_utf8_lossy(v).to_string())
                    .unwrap_or_else(|| "<null>".to_string());
                write!(
                    f,
                    "Ret{{ret={ret}, err={e:?}, out.len={}, out_head={}}}",
                    out.len(),
                    hex(&out[..out.len().min(48)])
                )
            }
            Outcome::Signal { sig, diag } => match diag {
                Some(d) => write!(f, "Signal({sig}{}) {d}", sig_name(*sig)),
                None => write!(f, "Signal({sig}{})", sig_name(*sig)),
            },
        }
    }
}

fn sig_name(s: i32) -> &'static str {
    match s {
        6 => " SIGABRT/assert",
        11 => " SIGSEGV",
        14 => " SIGALRM/hang",
        8 => " SIGFPE",
        4 => " SIGILL",
        _ => "",
    }
}

impl Outcome {
    pub fn encode(&self) -> String {
        match self {
            Outcome::Ret { ret, err, out } => format!(
                "R {} {} {}",
                ret,
                match err {
                    None => "-".to_string(),
                    Some(v) => hex(v),
                },
                hex(out)
            ),
            Outcome::Signal { sig, diag } => format!(
                "S {sig} {}",
                match diag {
                    None => "-".to_string(),
                    Some(d) => hex(d.as_bytes()),
                }
            ),
        }
    }
    pub fn decode(line: &str) -> Outcome {
        let f: Vec<&str> = line.split(' ').collect();
        match f[0] {
            "R" => Outcome::Ret {
                ret: f[1].parse().unwrap(),
                err: if f[2] == "-" { None } else { Some(unhex(f[2])) },
                out: unhex(f[3]),
            },
            "S" => Outcome::Signal {
                sig: f[1].parse().unwrap(),
                diag: if f.len() < 3 || f[2] == "-" {
                    None
                } else {
                    Some(String::from_utf8(unhex(f[2])).unwrap())
                },
            },
            _ => panic!("bad outcome encoding: {line}"),
        }
    }
}

// ---------------------------------------------------------------------------
// hex
// ---------------------------------------------------------------------------

pub fn hex(b: &[u8]) -> String {
    if b.is_empty() {
        return "-".to_string();
    }
    let mut s = String::with_capacity(b.len() * 2);
    for x in b {
        s.push_str(&format!("{x:02x}"));
    }
    s
}

pub fn unhex(s: &str) -> Vec<u8> {
    if s == "-" {
        return Vec::new();
    }
    assert!(s.len() % 2 == 0, "odd hex string");
    (0..s.len() / 2)
        .map(|i| u8::from_str_radix(&s[i * 2..i * 2 + 2], 16).expect("bad hex"))
        .collect()
}
