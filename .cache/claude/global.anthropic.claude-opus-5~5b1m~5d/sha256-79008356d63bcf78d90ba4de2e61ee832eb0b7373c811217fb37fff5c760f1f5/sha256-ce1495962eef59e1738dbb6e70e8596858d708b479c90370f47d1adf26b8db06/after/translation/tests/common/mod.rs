//! Shared differential-test harness.
//!
//! Loads BOTH shared objects through `libloading` and calls `parse_number`
//! only through the dynamic-symbol table, so the `#[no_mangle]`/`extern "C"`
//! export wrappers are exercised exactly as an external C consumer would.
//!
//! * C   : `c_src/build/libdriver.so`
//! * Rust: `translation/target/<profile>/libdriver.so`

#![allow(dead_code)]

use std::ffi::{c_double, c_int, c_uchar};
use std::path::PathBuf;
use std::sync::OnceLock;

/* ------------------------------------------------------------------ ABI ---- */

/// Mirror of the C `parse_buffer` from `c_src/include/lib.h`.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ParseBuffer {
    pub content: *const c_uchar,
    pub length: usize,
    pub offset: usize,
    pub depth: usize,
}

/// Mirror of the C `cJSON` from `c_src/include/lib.h`.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct CJson {
    pub type_: c_int,
    pub valueint: c_int,
    pub valuedouble: c_double,
}

/// Snapshot of everything a call can observably touch.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Observed {
    pub ret: c_int,
    pub type_: c_int,
    pub valueint: c_int,
    /// Raw bits so that `NaN` / `-0.0` compare byte-for-byte.
    pub valuedouble_bits: u64,
    /// `parse_buffer` state after the call (content pointer excluded: the two
    /// runs use two distinct copies of the bytes, but we assert it is unchanged
    /// relative to its own input).
    pub buf_content_unchanged: bool,
    pub buf_length: usize,
    pub buf_offset: usize,
    pub buf_depth: usize,
}

pub type ParseNumberFn = unsafe extern "C" fn(*mut CJson, *mut ParseBuffer) -> c_int;

/* -------------------------------------------------------------- loading ---- */

struct Libs {
    _c: libloading::Library,
    _rust: libloading::Library,
    c_parse_number: ParseNumberFn,
    rust_parse_number: ParseNumberFn,
}

// The two `Library` handles are only ever read after initialisation.
unsafe impl Send for Libs {}
unsafe impl Sync for Libs {}

static LIBS: OnceLock<Libs> = OnceLock::new();

fn c_so_path() -> PathBuf {
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.pop(); // -> working directory
    p.push("c_src");
    p.push("build");
    p.push("libdriver.so");
    p
}

fn rust_so_path() -> PathBuf {
    // Explicit override wins, so a driver script can pin an exact artifact.
    if let Ok(p) = std::env::var("RUST_DRIVER_SO") {
        if !p.is_empty() {
            return PathBuf::from(p);
        }
    }
    // .../target/<profile>/deps/<test-exe>  ->  .../target/<profile>/libdriver.so
    let exe = std::env::current_exe().expect("current_exe");
    let profile_dir = exe
        .parent()
        .and_then(|p| p.parent())
        .expect("target/<profile>");
    let candidate = profile_dir.join("libdriver.so");
    if candidate.exists() {
        return candidate;
    }
    // Fallbacks, in case the harness is invoked from an unusual layout.
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target");
    for prof in ["release", "debug"] {
        let c = root.join(prof).join("libdriver.so");
        if c.exists() {
            return c;
        }
    }
    candidate
}

fn libs() -> &'static Libs {
    LIBS.get_or_init(|| {
        let c_path = c_so_path();
        let rust_path = rust_so_path();
        assert!(
            c_path.exists(),
            "C shared object not found at {c_path:?}. Build it with:\n  \
             cd c_src && mkdir -p build && cd build && \
             cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build ."
        );
        assert!(
            rust_path.exists(),
            "Rust shared object not found at {rust_path:?}. Build it with `cargo build`."
        );
        unsafe {
            let c = libloading::Library::new(&c_path)
                .unwrap_or_else(|e| panic!("dlopen {c_path:?}: {e}"));
            let rust = libloading::Library::new(&rust_path)
                .unwrap_or_else(|e| panic!("dlopen {rust_path:?}: {e}"));
            let c_sym: libloading::Symbol<ParseNumberFn> = c
                .get(b"parse_number\0")
                .expect("C .so does not export parse_number");
            let rust_sym: libloading::Symbol<ParseNumberFn> = rust
                .get(b"parse_number\0")
                .expect("Rust .so does not export parse_number");
            let c_parse_number = *c_sym;
            let rust_parse_number = *rust_sym;
            Libs {
                _c: c,
                _rust: rust,
                c_parse_number,
                rust_parse_number,
            }
        }
    })
}

pub fn c_parse_number() -> ParseNumberFn {
    libs().c_parse_number
}

pub fn rust_parse_number() -> ParseNumberFn {
    libs().rust_parse_number
}

/* ------------------------------------------------------------- the case ---- */

/// A single differential test case: the exact bytes plus the exact struct state
/// handed to `parse_number`.
#[derive(Clone, Debug)]
pub struct Case {
    /// Backing allocation handed to `content` (may be longer than `length`, so
    /// that reads past `length` are detectable but still in-bounds).
    pub bytes: Vec<u8>,
    /// `parse_buffer.length`
    pub length: usize,
    /// `parse_buffer.offset`
    pub offset: usize,
    /// `parse_buffer.depth`
    pub depth: usize,
    /// pass `content == NULL`
    pub content_null: bool,
    /// pass `input_buffer == NULL`
    pub buffer_null: bool,
    /// pre-existing `item` contents (must be preserved on every failure path)
    pub item_type: c_int,
    pub item_valueint: c_int,
    pub item_valuedouble_bits: u64,
}

impl Case {
    /// Case over `s`, with `length == s.len()`, `offset == 0`.
    pub fn from_str(s: &str) -> Case {
        Case::from_bytes(s.as_bytes())
    }

    pub fn from_bytes(b: &[u8]) -> Case {
        Case {
            bytes: b.to_vec(),
            length: b.len(),
            offset: 0,
            depth: 0,
            content_null: false,
            buffer_null: false,
            // distinctive sentinels: must be overwritten on success and
            // preserved on failure
            item_type: -559038737,           // 0xDEADBEEF
            item_valueint: -889275714,       // 0xCAFEBABE
            item_valuedouble_bits: 0x7ff8_0000_dead_beef, // a signalling-ish NaN
        }
    }

    pub fn length(mut self, n: usize) -> Case {
        self.length = n;
        self
    }
    pub fn offset(mut self, n: usize) -> Case {
        self.offset = n;
        self
    }
    pub fn depth(mut self, n: usize) -> Case {
        self.depth = n;
        self
    }
    /// Append bytes past `length` that must never be read.
    pub fn with_guard(mut self, guard: &[u8]) -> Case {
        self.bytes.extend_from_slice(guard);
        self
    }
    pub fn content_null(mut self) -> Case {
        self.content_null = true;
        self
    }
    pub fn buffer_null(mut self) -> Case {
        self.buffer_null = true;
        self
    }

    fn describe(&self) -> String {
        let visible_end = self.bytes.len().min(96);
        format!(
            "bytes={:?}{} len={} length={} offset={} depth={} content_null={} buffer_null={}",
            String::from_utf8_lossy(&self.bytes[..visible_end]),
            if self.bytes.len() > visible_end { "..." } else { "" },
            self.bytes.len(),
            self.length,
            self.offset,
            self.depth,
            self.content_null,
            self.buffer_null,
        )
    }
}

/// Deterministic filler placed after `case.bytes` in every backing allocation.
///
/// `0xAB` is *not* in `ACCEPTED`, so it terminates the C scanner. Its purpose is
/// to make any read past `case.bytes` deterministic and identical for both runs,
/// so a `length` that overshoots the logical content can never produce a
/// spurious "divergence" from uninitialised heap garbage.
const PAD: u8 = 0xAB;
const PAD_LEN: usize = 128;

/// Run one implementation over a case, returning everything observable.
fn run_one(f: ParseNumberFn, case: &Case) -> Observed {
    // Private copy of the bytes, so neither run can influence the other and so
    // a stray write is visible.
    let mut bytes = case.bytes.clone();
    bytes.resize(case.bytes.len() + PAD_LEN, PAD);
    let expected = bytes.clone();
    let base: *const c_uchar = if case.content_null {
        std::ptr::null()
    } else {
        // Always non-null and uniquely owned: `bytes` has at least PAD_LEN
        // elements, so `as_mut_ptr` never returns a dangling pointer.
        bytes.as_mut_ptr()
    };

    let mut item = CJson {
        type_: case.item_type,
        valueint: case.item_valueint,
        valuedouble: f64::from_bits(case.item_valuedouble_bits),
    };

    let mut buf = ParseBuffer {
        content: base,
        length: case.length,
        offset: case.offset,
        depth: case.depth,
    };
    let buf_in = buf;

    let buf_ptr: *mut ParseBuffer = if case.buffer_null {
        std::ptr::null_mut()
    } else {
        &mut buf
    };

    let ret = unsafe { f(&mut item, buf_ptr) };

    // Guard against any implementation writing through `content`.
    assert!(
        bytes == expected,
        "implementation mutated the input buffer contents"
    );

    if case.buffer_null {
        Observed {
            ret,
            type_: item.type_,
            valueint: item.valueint,
            valuedouble_bits: item.valuedouble.to_bits(),
            buf_content_unchanged: true,
            buf_length: 0,
            buf_offset: 0,
            buf_depth: 0,
        }
    } else {
        Observed {
            ret,
            type_: item.type_,
            valueint: item.valueint,
            valuedouble_bits: item.valuedouble.to_bits(),
            buf_content_unchanged: buf.content == buf_in.content,
            buf_length: buf.length,
            buf_offset: buf.offset,
            buf_depth: buf.depth,
        }
    }
}

pub fn observe_c(case: &Case) -> Observed {
    run_one(c_parse_number(), case)
}

pub fn observe_rust(case: &Case) -> Observed {
    run_one(rust_parse_number(), case)
}

/// Differential check for one case. Panics with a full diff on divergence.
#[track_caller]
pub fn diff(case: &Case) -> Observed {
    let c = observe_c(case);
    let r = observe_rust(case);
    if c != r {
        panic!(
            "DIVERGENCE\n  case: {}\n  C   : {:?}\n  Rust: {:?}\n  \
             C.valuedouble={} Rust.valuedouble={}",
            case.describe(),
            c,
            r,
            f64::from_bits(c.valuedouble_bits),
            f64::from_bits(r.valuedouble_bits),
        );
    }
    c
}

#[track_caller]
pub fn diff_str(s: &str) -> Observed {
    diff(&Case::from_str(s))
}

/* ----------------------------------------------------------------- PRNG ---- */

/// xorshift64* — fixed seed, fully reproducible.
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
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
    pub fn below(&mut self, n: u64) -> u64 {
        if n == 0 { 0 } else { self.next_u64() % n }
    }
    pub fn range(&mut self, lo: u64, hi_inclusive: u64) -> u64 {
        lo + self.below(hi_inclusive - lo + 1)
    }
    pub fn bool(&mut self) -> bool {
        self.next_u64() & 1 == 1
    }
    pub fn pick<'a, T>(&mut self, xs: &'a [T]) -> &'a T {
        &xs[self.below(xs.len() as u64) as usize]
    }
    pub fn digit(&mut self) -> u8 {
        b'0' + self.below(10) as u8
    }
    pub fn digits(&mut self, n: usize) -> String {
        (0..n).map(|_| self.digit() as char).collect()
    }
    /// `digits` with a randomly chosen length in `lo..=hi` (avoids nested
    /// `&mut self` borrows at call sites).
    pub fn digits_between(&mut self, lo: u64, hi_inclusive: u64) -> String {
        let n = self.range(lo, hi_inclusive) as usize;
        self.digits(n)
    }
    /// `pick` returning an owned copy.
    pub fn choose<T: Copy>(&mut self, xs: &[T]) -> T {
        xs[self.below(xs.len() as u64) as usize]
    }
}

/// Bytes the C `switch` accepts (everything else hits `default:`).
pub const ACCEPTED: &[u8] = b"0123456789+-eE.";
