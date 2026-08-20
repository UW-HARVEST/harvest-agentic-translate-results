//! Shared differential-test harness.
//!
//! BOTH implementations are loaded as shared objects with `libloading` and
//! driven exclusively through their exported `parse_number` symbol — the Rust
//! functions are never called directly, so the `#[unsafe(no_mangle)]`/
//! `extern "C"` wrapper and the C ABI layout are part of what is under test.

#![allow(dead_code)]

use libloading::Library;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

// ---------------------------------------------------------------------------
// ABI mirrors of the C types (c_src/include/lib.h)
// ---------------------------------------------------------------------------

/// ```c
/// typedef struct {
///     const unsigned char *content;
///     size_t length;
///     size_t offset;
///     size_t depth;
/// } parse_buffer;
/// ```
#[repr(C)]
#[derive(Clone, Copy)]
pub struct ParseBuffer {
    pub content: *const u8,
    pub length: usize,
    pub offset: usize,
    pub depth: usize,
}

/// ```c
/// typedef struct { int type; int valueint; double valuedouble; } cJSON;
/// ```
#[repr(C)]
#[derive(Clone, Copy)]
pub struct CJson {
    pub type_: i32,
    pub valueint: i32,
    pub valuedouble: f64,
}

/// `cJSON_bool parse_number(cJSON * const item, parse_buffer * const input_buffer);`
pub type ParseNumberFn = unsafe extern "C" fn(*mut CJson, *mut ParseBuffer) -> i32;

pub const CJSON_NUMBER: i32 = 1 << 3;
pub const C_TRUE: i32 = 1;
pub const C_FALSE: i32 = 0;

// ---------------------------------------------------------------------------
// Loading the two shared objects
// ---------------------------------------------------------------------------

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn first_existing(candidates: &[PathBuf], what: &str) -> PathBuf {
    for c in candidates {
        if c.is_file() {
            return c.clone();
        }
    }
    panic!(
        "could not locate the {what} shared object; tried:\n{}",
        candidates
            .iter()
            .map(|p| format!("  {}", p.display()))
            .collect::<Vec<_>>()
            .join("\n")
    );
}

fn c_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("C_DRIVER_SO") {
        return PathBuf::from(p);
    }
    let m = manifest_dir();
    first_existing(
        &[
            m.join("c_src/build/libdriver.so"),
            m.join("c_src/build/lib/libdriver.so"),
            m.join("c_src/build/Release/libdriver.so"),
        ],
        "C",
    )
}

/// The Rust `cdylib`. `cargo test` builds it next to the integration-test
/// binary's directory (`target/<profile>/`), which we derive from
/// `std::env::current_exe()` so that it works for any profile / target dir.
fn rust_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("RUST_DRIVER_SO") {
        return PathBuf::from(p);
    }
    let mut candidates = Vec::new();
    if let Ok(exe) = std::env::current_exe() {
        // target/<profile>/deps/<test>-<hash>  ->  target/<profile>/
        let mut d: Option<&Path> = exe.parent();
        for _ in 0..3 {
            if let Some(dir) = d {
                candidates.push(dir.join("libdriver.so"));
                d = dir.parent();
            }
        }
    }
    let m = manifest_dir();
    for profile in ["debug", "release"] {
        candidates.push(m.join("target").join(profile).join("libdriver.so"));
    }
    first_existing(&candidates, "Rust")
}

struct Both {
    c: ParseNumberFn,
    rust: ParseNumberFn,
}

// Raw `extern "C"` function pointers are `Send + Sync`.
unsafe impl Send for Both {}
unsafe impl Sync for Both {}

static BOTH: OnceLock<Both> = OnceLock::new();

fn load(path: &Path) -> ParseNumberFn {
    unsafe {
        let lib = Library::new(path)
            .unwrap_or_else(|e| panic!("dlopen({}) failed: {e}", path.display()));
        // Leak the handle: the function pointer must stay valid for the whole
        // process lifetime.
        let lib: &'static Library = Box::leak(Box::new(lib));
        let sym = lib
            .get::<ParseNumberFn>(b"parse_number\0")
            .unwrap_or_else(|e| panic!("dlsym(parse_number) in {} failed: {e}", path.display()));
        *sym
    }
}

fn both() -> &'static Both {
    BOTH.get_or_init(|| Both {
        c: load(&c_so_path()),
        rust: load(&rust_so_path()),
    })
}

pub fn c_parse_number() -> ParseNumberFn {
    both().c
}

pub fn rust_parse_number() -> ParseNumberFn {
    both().rust
}

// ---------------------------------------------------------------------------
// Test case description + execution
// ---------------------------------------------------------------------------

/// Poison values pre-written into the out-parameters so that "field not
/// written" is observable.
pub const POISON_TYPE: i32 = -0x5A5A_5A5A;
pub const POISON_VALUEINT: i32 = 0x1234_5678;
pub const POISON_DOUBLE_BITS: u64 = 0xDEAD_BEEF_CAFE_F00D;
pub const POISON_DEPTH: usize = 0xA5A5_A5A5_A5A5_A5A5;

#[derive(Clone, Debug)]
pub struct Case {
    /// Backing bytes the `parse_buffer::content` pointer refers to.
    pub content: Vec<u8>,
    /// `parse_buffer::length`; `None` means `content.len()`.
    pub length: Option<usize>,
    pub offset: usize,
    pub depth: usize,
    /// Pass `content == NULL`.
    pub content_null: bool,
    /// Pass `input_buffer == NULL`.
    pub buffer_null: bool,
    /// Pass `item == NULL` (only legal on paths that return before touching it).
    pub item_null: bool,
    pub item_type: i32,
    pub item_valueint: i32,
    pub item_double_bits: u64,
}

impl Case {
    pub fn new(content: impl AsRef<[u8]>) -> Self {
        Case {
            content: content.as_ref().to_vec(),
            length: None,
            offset: 0,
            depth: POISON_DEPTH,
            content_null: false,
            buffer_null: false,
            item_null: false,
            item_type: POISON_TYPE,
            item_valueint: POISON_VALUEINT,
            item_double_bits: POISON_DOUBLE_BITS,
        }
    }
    pub fn length(mut self, l: usize) -> Self {
        self.length = Some(l);
        self
    }
    pub fn offset(mut self, o: usize) -> Self {
        self.offset = o;
        self
    }
    pub fn depth(mut self, d: usize) -> Self {
        self.depth = d;
        self
    }
    pub fn content_null(mut self) -> Self {
        self.content_null = true;
        self
    }
    pub fn buffer_null(mut self) -> Self {
        self.buffer_null = true;
        self
    }
    pub fn item_null(mut self) -> Self {
        self.item_null = true;
        self
    }
    pub fn item_state(mut self, t: i32, vi: i32, bits: u64) -> Self {
        self.item_type = t;
        self.item_valueint = vi;
        self.item_double_bits = bits;
        self
    }

    /// Human-readable label used in assertion messages.
    pub fn label(&self) -> String {
        format!(
            "content={:?} (escaped={}) length={:?} offset={} depth={:#x} \
             content_null={} buffer_null={} item_null={}",
            String::from_utf8_lossy(&self.content),
            escape(&self.content),
            self.length,
            self.offset,
            self.depth,
            self.content_null,
            self.buffer_null,
            self.item_null,
        )
    }
}

pub fn escape(bytes: &[u8]) -> String {
    let mut s = String::new();
    for &b in bytes {
        if b.is_ascii_graphic() || b == b' ' {
            s.push(b as char);
        } else {
            s.push_str(&format!("\\x{b:02x}"));
        }
    }
    s
}

/// Everything observable after one `parse_number` call.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Outcome {
    pub ret: i32,
    pub item_type: i32,
    pub item_valueint: i32,
    /// Raw bits, so `-0.0` vs `+0.0`, `±inf` and NaN payloads all compare exactly.
    pub item_double_bits: u64,
    pub buf_content_is_null: bool,
    pub buf_length: usize,
    pub buf_offset: usize,
    pub buf_depth: usize,
}

/// Bytes appended after the visible region of every content buffer.
///
/// They are all in the C's accepted charset (`'8'`), so an implementation that
/// reads even ONE byte past `length` (e.g. `offset + index <= length` instead of
/// `<`) deterministically produces a different number and is caught — rather
/// than happening to read a harmless heap byte. The guard also guarantees the
/// content pointer is a real, non-dangling allocation for empty inputs.
pub const GUARD: &[u8] = b"88888888";

/// Run one case against one implementation, through the FFI boundary.
pub fn run(f: ParseNumberFn, case: &Case) -> Outcome {
    // A fresh copy of the content bytes per run so that a (buggy) write by one
    // implementation cannot leak into the other's run, plus the read guard.
    let mut content = Vec::with_capacity(case.content.len() + GUARD.len());
    content.extend_from_slice(&case.content);
    content.extend_from_slice(GUARD);
    let content_ptr: *const u8 = if case.content_null {
        std::ptr::null()
    } else {
        content.as_mut_ptr()
    };
    // NB: derived from the CASE's content length, never from the guarded buffer.
    let length = case.length.unwrap_or(case.content.len());

    let mut item = CJson {
        type_: case.item_type,
        valueint: case.item_valueint,
        valuedouble: f64::from_bits(case.item_double_bits),
    };
    let mut buf = ParseBuffer {
        content: content_ptr,
        length,
        offset: case.offset,
        depth: case.depth,
    };

    let item_ptr: *mut CJson = if case.item_null {
        std::ptr::null_mut()
    } else {
        &mut item
    };
    let buf_ptr: *mut ParseBuffer = if case.buffer_null {
        std::ptr::null_mut()
    } else {
        &mut buf
    };

    let ret = unsafe { f(item_ptr, buf_ptr) };

    // Keep `content` alive across the call.
    std::hint::black_box(&content);

    Outcome {
        ret,
        item_type: item.type_,
        item_valueint: item.valueint,
        item_double_bits: item.valuedouble.to_bits(),
        buf_content_is_null: buf.content.is_null(),
        buf_length: buf.length,
        buf_offset: buf.offset,
        buf_depth: buf.depth,
    }
}

/// Run one case against BOTH `.so`s and assert byte-identical observables.
#[track_caller]
pub fn assert_same(case: &Case) -> Outcome {
    let c = run(c_parse_number(), case);
    let r = run(rust_parse_number(), case);
    if c != r {
        panic!(
            "DIVERGENCE\n  case : {}\n  C    : {c:?}\n  Rust : {r:?}\n  \
             (valuedouble C={} Rust={})",
            case.label(),
            f64::from_bits(c.item_double_bits),
            f64::from_bits(r.item_double_bits),
        );
    }
    c
}

/// Convenience: build a `Case` from a byte string and compare.
#[track_caller]
pub fn assert_same_str(s: impl AsRef<[u8]>) -> Outcome {
    assert_same(&Case::new(s))
}

/// Drive ONE implementation with `calls` successive `parse_number` invocations
/// that share a single `parse_buffer` / `cJSON` pair — i.e. the way a real
/// consumer walks a document, so the cumulative `offset` arithmetic of the
/// composed pipeline is under test, not just one isolated call.
///
/// Between numbers the caller-supplied `skip` closure is applied to the shared
/// buffer (mimicking cJSON's separator skipping).
pub fn run_sequence(
    f: ParseNumberFn,
    content: &[u8],
    calls: usize,
    skip: impl Fn(&mut ParseBuffer, &[u8]),
) -> Vec<Outcome> {
    let visible_len = content.len();
    let mut owned = Vec::with_capacity(visible_len + GUARD.len());
    owned.extend_from_slice(content);
    owned.extend_from_slice(GUARD);
    let mut item = CJson {
        type_: POISON_TYPE,
        valueint: POISON_VALUEINT,
        valuedouble: f64::from_bits(POISON_DOUBLE_BITS),
    };
    let mut buf = ParseBuffer {
        content: owned.as_mut_ptr(),
        length: visible_len, // guard bytes are OUTSIDE the visible region
        offset: 0,
        depth: POISON_DEPTH,
    };
    let snapshot = content.to_vec();
    let mut out = Vec::with_capacity(calls);
    for _ in 0..calls {
        let ret = unsafe { f(&mut item, &mut buf) };
        out.push(Outcome {
            ret,
            item_type: item.type_,
            item_valueint: item.valueint,
            item_double_bits: item.valuedouble.to_bits(),
            buf_content_is_null: buf.content.is_null(),
            buf_length: buf.length,
            buf_offset: buf.offset,
            buf_depth: buf.depth,
        });
        skip(&mut buf, &snapshot);
    }
    std::hint::black_box(&owned);
    out
}

#[track_caller]
pub fn assert_same_sequence(
    content: &[u8],
    calls: usize,
    skip: impl Fn(&mut ParseBuffer, &[u8]) + Copy,
) {
    let c = run_sequence(c_parse_number(), content, calls, skip);
    let r = run_sequence(rust_parse_number(), content, calls, skip);
    assert_eq!(
        c,
        r,
        "DIVERGENCE in sequence over {:?}",
        escape(content)
    );
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (SplitMix64) — property-style testing with a fixed seed
// ---------------------------------------------------------------------------

pub struct Rng(u64);

pub const SEED: u64 = 0x5EED_1234_ABCD_0001;

impl Rng {
    pub fn new(seed: u64) -> Self {
        Rng(seed)
    }
    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    /// Uniform in `[0, n)` (`n > 0`).
    pub fn below(&mut self, n: usize) -> usize {
        (self.next_u64() % (n as u64)) as usize
    }
    /// Uniform in `[lo, hi]`.
    pub fn range(&mut self, lo: usize, hi: usize) -> usize {
        lo + self.below(hi - lo + 1)
    }
    pub fn bool(&mut self) -> bool {
        self.next_u64() & 1 == 1
    }
    pub fn pick<'a, T>(&mut self, xs: &'a [T]) -> &'a T {
        &xs[self.below(xs.len())]
    }
    pub fn digit(&mut self) -> u8 {
        b'0' + (self.below(10) as u8)
    }
    pub fn digits(&mut self, n: usize) -> Vec<u8> {
        (0..n).map(|_| self.digit()).collect()
    }
    /// `digits(range(lo, hi))` — separate method to dodge the double-borrow.
    pub fn digits_range(&mut self, lo: usize, hi: usize) -> Vec<u8> {
        let n = self.range(lo, hi);
        self.digits(n)
    }
    pub fn byte(&mut self) -> u8 {
        (self.next_u64() & 0xFF) as u8
    }
}
