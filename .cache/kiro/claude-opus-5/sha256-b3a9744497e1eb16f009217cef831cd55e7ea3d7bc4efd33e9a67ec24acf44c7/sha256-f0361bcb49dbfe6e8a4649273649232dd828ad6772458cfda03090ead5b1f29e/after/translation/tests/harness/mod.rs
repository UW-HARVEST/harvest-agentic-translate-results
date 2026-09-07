//! Shared differential-test harness.
//!
//! Loads BOTH shared objects through `libloading` and calls `parse_number`
//! only through their exported `extern "C"` symbols — the Rust function is
//! never called directly, so the `#[no_mangle]` wrapper is under test too.

#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::ffi::c_int;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

/* ---------------- ABI types (must mirror c_src/include/lib.h) ------------- */

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct ParseBuffer {
    pub content: *const u8,
    pub length: usize,
    pub offset: usize,
    pub depth: usize,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct CJson {
    pub type_: c_int,
    pub valueint: c_int,
    pub valuedouble: f64,
}

/// A recognisable pre-state so that "the C never wrote this field" is
/// observable. `type_` is deliberately NOT `cJSON_Number` (8).
pub const SENTINEL_ITEM: CJson = CJson {
    type_: 0x7f7f_7f7f,
    valueint: -0x1234_5678,
    valuedouble: -0.0,
};

pub type ParseNumberFn = unsafe extern "C" fn(*mut CJson, *mut ParseBuffer) -> c_int;

/* ------------------------------ library loading --------------------------- */

struct Libs {
    c: Library,
    rust: Library,
    c_path: PathBuf,
    rust_path: PathBuf,
}

// Safety: we only ever hand out raw `extern "C"` fn pointers, and the two
// libraries are stateless (no globals, no TLS).
unsafe impl Send for Libs {}
unsafe impl Sync for Libs {}

static LIBS: OnceLock<Libs> = OnceLock::new();

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn first_existing(cands: &[PathBuf]) -> Option<PathBuf> {
    cands.iter().find(|p| p.is_file()).cloned()
}

fn locate_c_so() -> PathBuf {
    if let Ok(p) = std::env::var("C_SO_PATH") {
        let p = PathBuf::from(p);
        assert!(p.is_file(), "C_SO_PATH does not exist: {}", p.display());
        return p;
    }
    let root = manifest_dir().parent().unwrap().to_path_buf();
    let cands = vec![
        root.join("c_src/build/libdriver.so"),
        root.join("c_src/build/lib/libdriver.so"),
    ];
    first_existing(&cands).unwrap_or_else(|| {
        panic!(
            "could not find the C shared library; build it with:\n  \
             cd c_src && mkdir -p build && cd build && \
             cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .\n\
             looked in: {:?}",
            cands
        )
    })
}

fn locate_rust_so() -> PathBuf {
    if let Ok(p) = std::env::var("RUST_SO_PATH") {
        let p = PathBuf::from(p);
        assert!(p.is_file(), "RUST_SO_PATH does not exist: {}", p.display());
        return p;
    }
    let mut cands: Vec<PathBuf> = Vec::new();
    // target/<profile>/deps/<test-bin>  ->  target/<profile>/libdriver.so
    if let Ok(exe) = std::env::current_exe() {
        if let Some(deps) = exe.parent() {
            if let Some(profile_dir) = deps.parent() {
                cands.push(profile_dir.join("libdriver.so"));
            }
        }
    }
    let md = manifest_dir();
    cands.push(md.join("target/release/libdriver.so"));
    cands.push(md.join("target/debug/libdriver.so"));
    first_existing(&cands).unwrap_or_else(|| {
        panic!(
            "could not find the Rust cdylib; build it with `cargo build --release`.\n\
             looked in: {:?}",
            cands
        )
    })
}

fn libs() -> &'static Libs {
    LIBS.get_or_init(|| {
        let c_path = locate_c_so();
        let rust_path = locate_rust_so();
        // Safety: both paths point at our own freshly built libraries.
        let c = unsafe { Library::new(&c_path) }
            .unwrap_or_else(|e| panic!("dlopen({}) failed: {e}", c_path.display()));
        let rust = unsafe { Library::new(&rust_path) }
            .unwrap_or_else(|e| panic!("dlopen({}) failed: {e}", rust_path.display()));
        eprintln!("[harness]   C .so: {}", c_path.display());
        eprintln!("[harness] Rust .so: {}", rust_path.display());
        Libs {
            c,
            rust,
            c_path,
            rust_path,
        }
    })
}

pub fn c_so_path() -> &'static Path {
    &libs().c_path
}
pub fn rust_so_path() -> &'static Path {
    &libs().rust_path
}

fn sym(lib: &'static Library, which: &str) -> ParseNumberFn {
    // Safety: the symbol has exactly this signature in both libraries.
    let s: Symbol<ParseNumberFn> = unsafe { lib.get(b"parse_number\0") }
        .unwrap_or_else(|e| panic!("{which} .so does not export `parse_number`: {e}"));
    // Safety: the library is leaked for the whole process lifetime (OnceLock).
    unsafe { *s.into_raw() }
}

pub fn c_parse_number() -> ParseNumberFn {
    static F: OnceLock<ParseNumberFn> = OnceLock::new();
    *F.get_or_init(|| sym(&libs().c, "C"))
}

pub fn rust_parse_number() -> ParseNumberFn {
    static F: OnceLock<ParseNumberFn> = OnceLock::new();
    *F.get_or_init(|| sym(&libs().rust, "Rust"))
}

/* ------------------------------ byte imaging ------------------------------ */

fn item_bytes(i: &CJson) -> [u8; 16] {
    let mut out = [0u8; 16];
    out[0..4].copy_from_slice(&i.type_.to_ne_bytes());
    out[4..8].copy_from_slice(&i.valueint.to_ne_bytes());
    out[8..16].copy_from_slice(&i.valuedouble.to_bits().to_ne_bytes());
    out
}

fn buf_bytes(b: &ParseBuffer) -> [u8; 32] {
    let mut out = [0u8; 32];
    out[0..8].copy_from_slice(&(b.content as usize).to_ne_bytes());
    out[8..16].copy_from_slice(&b.length.to_ne_bytes());
    out[16..24].copy_from_slice(&b.offset.to_ne_bytes());
    out[24..32].copy_from_slice(&b.depth.to_ne_bytes());
    out
}

fn show(content: &[u8], length: usize, offset: usize) -> String {
    let mut s = String::new();
    for &b in content.iter().take(64) {
        if b.is_ascii_graphic() || b == b' ' {
            s.push(b as char);
        } else {
            s.push_str(&format!("\\x{b:02x}"));
        }
    }
    if content.len() > 64 {
        s.push_str("...");
    }
    format!(
        "content(len={} alloc)={:?} length={} offset={}",
        content.len(),
        s,
        length,
        offset
    )
}

/* ------------------------------ the comparison ---------------------------- */

/// One differential invocation. Runs both `.so`s on identical inputs and
/// asserts the return value, the whole `cJSON` image and the whole
/// `parse_buffer` image are byte-identical.
///
/// `content` is the real backing allocation; `length` is the value written into
/// `parse_buffer.length` (may deliberately differ from `content.len()`).
pub fn diff_full(
    desc: &str,
    content: &[u8],
    length: usize,
    offset: usize,
    depth: usize,
    item_pre: CJson,
) {
    let mut item_c = item_pre;
    let mut item_r = item_pre;
    let mut buf_c = ParseBuffer {
        content: content.as_ptr(),
        length,
        offset,
        depth,
    };
    let mut buf_r = buf_c;

    // Safety: both callees are the C ABI `parse_number`; pointers are valid.
    let ret_c = unsafe { (c_parse_number())(&mut item_c, &mut buf_c) };
    let ret_r = unsafe { (rust_parse_number())(&mut item_r, &mut buf_r) };

    let ctx = || format!("[{desc}] {}", show(content, length, offset));

    assert_eq!(
        ret_c,
        ret_r,
        "return value differs: C={ret_c} Rust={ret_r}\n  {}",
        ctx()
    );
    assert_eq!(
        item_bytes(&item_c),
        item_bytes(&item_r),
        "cJSON image differs:\n  C   : type={} valueint={} valuedouble={:?} bits={:#018x}\n  \
         Rust: type={} valueint={} valuedouble={:?} bits={:#018x}\n  {}",
        item_c.type_,
        item_c.valueint,
        item_c.valuedouble,
        item_c.valuedouble.to_bits(),
        item_r.type_,
        item_r.valueint,
        item_r.valuedouble,
        item_r.valuedouble.to_bits(),
        ctx()
    );
    assert_eq!(
        buf_bytes(&buf_c),
        buf_bytes(&buf_r),
        "parse_buffer image differs:\n  C   : length={} offset={} depth={}\n  \
         Rust: length={} offset={} depth={}\n  {}",
        buf_c.length,
        buf_c.offset,
        buf_c.depth,
        buf_r.length,
        buf_r.offset,
        buf_r.depth,
        ctx()
    );
}

/// `content.len()` as length, offset 0, depth 0, sentinel item.
pub fn diff(desc: &str, content: &[u8]) {
    diff_full(desc, content, content.len(), 0, 0, SENTINEL_ITEM);
}

/// Same but with an explicit offset (length = `content.len()`).
pub fn diff_at(desc: &str, content: &[u8], offset: usize) {
    diff_full(desc, content, content.len(), offset, 0, SENTINEL_ITEM);
}

/// `input_buffer == NULL`.
pub fn diff_null_buffer(desc: &str, item_pre: CJson) {
    let mut item_c = item_pre;
    let mut item_r = item_pre;
    // Safety: the C checks for NULL before dereferencing; so must the Rust.
    let ret_c = unsafe { (c_parse_number())(&mut item_c, std::ptr::null_mut()) };
    let ret_r = unsafe { (rust_parse_number())(&mut item_r, std::ptr::null_mut()) };
    assert_eq!(ret_c, ret_r, "[{desc}] return value differs (NULL buffer)");
    assert_eq!(
        item_bytes(&item_c),
        item_bytes(&item_r),
        "[{desc}] cJSON image differs (NULL buffer)"
    );
}

/// `input_buffer != NULL` but `input_buffer->content == NULL`.
pub fn diff_null_content(desc: &str, length: usize, offset: usize, depth: usize, item_pre: CJson) {
    let mut item_c = item_pre;
    let mut item_r = item_pre;
    let mut buf_c = ParseBuffer {
        content: std::ptr::null(),
        length,
        offset,
        depth,
    };
    let mut buf_r = buf_c;
    // Safety: the C checks `content == NULL` before any read.
    let ret_c = unsafe { (c_parse_number())(&mut item_c, &mut buf_c) };
    let ret_r = unsafe { (rust_parse_number())(&mut item_r, &mut buf_r) };
    assert_eq!(ret_c, ret_r, "[{desc}] return value differs (NULL content)");
    assert_eq!(
        item_bytes(&item_c),
        item_bytes(&item_r),
        "[{desc}] cJSON image differs (NULL content)"
    );
    assert_eq!(
        buf_bytes(&buf_c),
        buf_bytes(&buf_r),
        "[{desc}] parse_buffer image differs (NULL content)"
    );
}

/* --------------------------------- the RNG -------------------------------- */

/// xorshift64* — deterministic, dependency-free, fixed seed per test.
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Rng(if seed == 0 { 0x9E37_79B9_7F4A_7C15 } else { seed })
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
        assert!(n > 0);
        (self.next_u64() % (n as u64)) as usize
    }
    /// Inclusive range.
    pub fn range(&mut self, lo: usize, hi: usize) -> usize {
        assert!(lo <= hi);
        lo + self.below(hi - lo + 1)
    }
    pub fn byte(&mut self) -> u8 {
        (self.next_u64() >> 33) as u8
    }
    pub fn pick<'a, T>(&mut self, xs: &'a [T]) -> &'a T {
        &xs[self.below(xs.len())]
    }
    pub fn bool(&mut self) -> bool {
        self.next_u64() & 1 == 1
    }
    pub fn digits(&mut self, n: usize) -> Vec<u8> {
        (0..n).map(|_| b'0' + self.below(10) as u8).collect()
    }
    /// `digits(range(lo, hi))` — avoids a double mutable borrow at call sites.
    pub fn digits_range(&mut self, lo: usize, hi: usize) -> Vec<u8> {
        let n = self.range(lo, hi);
        self.digits(n)
    }
}

/// The 15 bytes the C scan loop accepts.
pub const ACCEPTED: &[u8] = b"0123456789+-eE.";

/// A byte that the scan loop does NOT accept (terminates the run).
pub fn non_accepted_byte(rng: &mut Rng) -> u8 {
    loop {
        let b = rng.byte();
        if !ACCEPTED.contains(&b) {
            return b;
        }
    }
}

/// The shared seed for all property-style rows.
pub const SEED: u64 = 0x5EED_C0FF_EE00_0001;

/// How many randomized inputs each `CONFIGS.md` row uses.
pub const N: usize = 600;
