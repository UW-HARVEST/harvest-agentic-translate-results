//! Shared differential-test harness.
//!
//! Loads BOTH the C `.so` and the Rust `.so` via `libloading` and calls the
//! exported `hex2bin` symbol through the FFI boundary in each, so the
//! `#[no_mangle]` export wrapper is exercised exactly as an external consumer
//! would exercise it. No Rust function is ever called directly.

#![allow(dead_code)]

use std::ffi::{c_char, c_int};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use libloading::{Library, Symbol};

pub type Hex2BinFn = unsafe extern "C" fn(
    *mut u8,
    usize,
    *const c_char,
    usize,
    *const c_char,
    *mut *const c_char,
) -> c_int;

/// The observable result of one `hex2bin` call, in a form that can be compared
/// byte-for-byte between the two implementations.
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Outcome {
    /// Return value (`-1` sentinel, or the number of decoded bytes).
    pub ret: c_int,
    /// The FULL output window as the callee left it, including any poison bytes
    /// it did not touch. Catches "wrote too much" / "rolled back" divergences.
    pub bin: Vec<u8>,
    /// `Some(offset_from_hex_base)` when `hex_end_p` was non-NULL, else `None`.
    /// Signed, because the C does `hex_pos--` and may point *before* `hex`.
    pub hex_end: Option<isize>,
}

struct Loaded {
    _lib: Library,
    f: Hex2BinFn,
}

// SAFETY: the loaded function is a pure computation over caller-provided
// buffers; the `Library` is leaked for the process lifetime via `OnceLock`.
unsafe impl Send for Loaded {}
unsafe impl Sync for Loaded {}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn find_first(dir: &Path, pred: impl Fn(&str) -> bool) -> Option<PathBuf> {
    let entries = std::fs::read_dir(dir).ok()?;
    let mut hits: Vec<PathBuf> = entries
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .map(&pred)
                .unwrap_or(false)
        })
        .collect();
    hits.sort();
    hits.into_iter().next()
}

fn c_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("HARVEST_C_SO") {
        return PathBuf::from(p);
    }
    let build = manifest_dir().join("../c_src/build");
    find_first(&build, |n| n.starts_with("lib") && n.ends_with(".so")).unwrap_or_else(|| {
        panic!(
            "no C .so found in {}; build it with:\n  cd c_src && mkdir -p build && cd build && \
             cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
            build.display()
        )
    })
}

fn rust_so_path() -> PathBuf {
    // Deterministic: honour an explicit override, else prefer target/debug
    // (which `cargo build` produces), else fall back to target/release.
    if let Ok(p) = std::env::var("HARVEST_RUST_SO") {
        return PathBuf::from(p);
    }
    let root = manifest_dir().join("target");
    for profile in ["debug", "release"] {
        let p = root.join(profile).join("libhex2bin_lib.so");
        if p.exists() {
            return p;
        }
    }
    panic!(
        "no Rust libhex2bin_lib.so under {}; run `cargo build` first",
        root.display()
    )
}

/// `cargo test` does NOT rebuild a `crate-type = ["cdylib"]` artifact, so a
/// differential suite that just `dlopen`s the `.so` can silently test a STALE
/// library and pass no matter what the source says. Refuse to run in that case.
fn assert_so_fresh(so: &Path) {
    let so_time = std::fs::metadata(so)
        .and_then(|m| m.modified())
        .unwrap_or(std::time::UNIX_EPOCH);
    let mut newest_src = std::time::UNIX_EPOCH;
    let mut newest_name = String::new();
    let src = manifest_dir().join("src");
    let mut stack = vec![src];
    while let Some(dir) = stack.pop() {
        let Ok(rd) = std::fs::read_dir(&dir) else { continue };
        for e in rd.flatten() {
            let p = e.path();
            if p.is_dir() {
                stack.push(p);
            } else if p.extension().and_then(|x| x.to_str()) == Some("rs") {
                if let Ok(t) = e.metadata().and_then(|m| m.modified()) {
                    if t > newest_src {
                        newest_src = t;
                        newest_name = p.display().to_string();
                    }
                }
            }
        }
    }
    assert!(
        so_time >= newest_src,
        "STALE Rust .so: {} is older than {}.\n\
         `cargo test` does not rebuild a cdylib -- run `cargo build` (or \
         ./run_all.sh) before testing, otherwise this suite would validate a \
         library that no longer matches src/.",
        so.display(),
        newest_name
    );
}

fn load(path: &Path) -> Loaded {
    unsafe {
        let lib = Library::new(path)
            .unwrap_or_else(|e| panic!("dlopen({}) failed: {e}", path.display()));
        let sym: Symbol<Hex2BinFn> = lib
            .get(b"hex2bin\0")
            .unwrap_or_else(|e| panic!("dlsym(hex2bin) in {} failed: {e}", path.display()));
        let f = *sym;
        Loaded { _lib: lib, f }
    }
}

fn c_lib() -> &'static Loaded {
    static L: OnceLock<Loaded> = OnceLock::new();
    L.get_or_init(|| load(&c_so_path()))
}

fn rust_lib() -> &'static Loaded {
    static L: OnceLock<Loaded> = OnceLock::new();
    L.get_or_init(|| {
        let p = rust_so_path();
        assert_so_fresh(&p);
        load(&p)
    })
}

pub const POISON: u8 = 0xA5;

/// One test case: exactly the six parameters of `hex2bin`, in a form that is
/// safe to materialise twice (once per implementation).
#[derive(Debug, Clone)]
pub struct Case {
    /// Bytes handed to `hex` (may contain NULs and high-bit bytes).
    pub hex: Vec<u8>,
    /// Value passed as `hex_len`. Normally `hex.len()`; may be shorter to prove
    /// the scan stops at the length rather than at a NUL.
    pub hex_len: usize,
    /// Real capacity allocated for `bin` (always >= what can be written).
    pub bin_cap: usize,
    /// Value passed as `bin_maxlen` (may be larger than `bin_cap`, e.g.
    /// `usize::MAX`, as long as the decode cannot actually reach that far).
    pub bin_maxlen: usize,
    /// `None` => pass `ignore = NULL`. `Some(s)` => pass a NUL-terminated copy.
    pub ignore: Option<Vec<u8>>,
    /// Whether to pass a non-NULL `hex_end_p`.
    pub want_hex_end: bool,
    /// Pass `hex = NULL` (only legal together with `hex_len == 0`).
    pub null_hex: bool,
    /// Pass `bin = NULL` (only legal when nothing can be written).
    pub null_bin: bool,
}

impl Case {
    pub fn new(hex: &[u8]) -> Self {
        Case {
            hex: hex.to_vec(),
            hex_len: hex.len(),
            bin_cap: hex.len() / 2 + 1,
            bin_maxlen: hex.len() / 2,
            ignore: None,
            want_hex_end: true,
            null_hex: false,
            null_bin: false,
        }
    }
    pub fn hex_len(mut self, n: usize) -> Self {
        self.hex_len = n;
        self
    }
    pub fn bin_maxlen(mut self, n: usize) -> Self {
        self.bin_maxlen = n;
        self
    }
    pub fn bin_cap(mut self, n: usize) -> Self {
        self.bin_cap = n;
        self
    }
    pub fn ignore(mut self, s: &[u8]) -> Self {
        self.ignore = Some(s.to_vec());
        self
    }
    pub fn no_ignore(mut self) -> Self {
        self.ignore = None;
        self
    }
    pub fn hex_end(mut self, yes: bool) -> Self {
        self.want_hex_end = yes;
        self
    }
    pub fn null_hex(mut self) -> Self {
        self.null_hex = true;
        self
    }
    pub fn null_bin(mut self) -> Self {
        self.null_bin = true;
        self
    }
}

fn invoke(f: Hex2BinFn, case: &Case) -> Outcome {
    // Fresh, poisoned output window per invocation.
    let mut bin = vec![POISON; case.bin_cap];
    // Fresh input copy per invocation (the C must not mutate it, and using a
    // distinct allocation means a stale-pointer bug cannot accidentally pass).
    let hex_buf = case.hex.clone();
    let ignore_buf: Option<Vec<u8>> = case.ignore.as_ref().map(|s| {
        let mut v = s.clone();
        v.push(0);
        v
    });

    let bin_ptr = if case.null_bin {
        std::ptr::null_mut()
    } else {
        bin.as_mut_ptr()
    };
    let hex_ptr: *const c_char = if case.null_hex {
        std::ptr::null()
    } else {
        hex_buf.as_ptr() as *const c_char
    };
    let ignore_ptr: *const c_char = match &ignore_buf {
        None => std::ptr::null(),
        Some(v) => v.as_ptr() as *const c_char,
    };

    let mut hex_end: *const c_char = std::ptr::null();
    let hex_end_ptr: *mut *const c_char = if case.want_hex_end {
        &mut hex_end
    } else {
        std::ptr::null_mut()
    };

    let ret = unsafe {
        f(
            bin_ptr,
            case.bin_maxlen,
            hex_ptr,
            case.hex_len,
            ignore_ptr,
            hex_end_ptr,
        )
    };

    let hex_end_off = if case.want_hex_end {
        Some((hex_end as isize).wrapping_sub(hex_ptr as isize))
    } else {
        None
    };

    Outcome {
        ret,
        bin,
        hex_end: hex_end_off,
    }
}

pub fn call_c(case: &Case) -> Outcome {
    invoke(c_lib().f, case)
}

pub fn call_rust(case: &Case) -> Outcome {
    invoke(rust_lib().f, case)
}

/// Run the case through both `.so`s and assert byte-for-byte equality of every
/// observable: return value, the whole output window, and the end pointer.
#[track_caller]
pub fn assert_same(label: &str, case: &Case) -> Outcome {
    let c = call_c(case);
    let r = call_rust(case);
    if c != r {
        panic!(
            "DIVERGENCE [{label}]\n  case: hex={:02x?} hex_len={} bin_cap={} bin_maxlen={} \
             ignore={:02x?} hex_end_p={} null_hex={} null_bin={}\n  C   : ret={} hex_end={:?} bin={:02x?}\n  \
             Rust: ret={} hex_end={:?} bin={:02x?}",
            case.hex,
            case.hex_len,
            case.bin_cap,
            case.bin_maxlen,
            case.ignore,
            case.want_hex_end,
            case.null_hex,
            case.null_bin,
            c.ret,
            c.hex_end,
            c.bin,
            r.ret,
            r.hex_end,
            r.bin,
        );
    }
    c
}

/// Deterministic PRNG (SplitMix64) so every "randomized" row is reproducible.
pub struct Rng(u64);

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
    /// Uniform in `[0, n)`.
    pub fn below(&mut self, n: usize) -> usize {
        if n == 0 {
            0
        } else {
            (self.next_u64() % n as u64) as usize
        }
    }
    pub fn byte(&mut self) -> u8 {
        (self.next_u64() >> 24) as u8
    }
    pub fn pick<'a, T>(&mut self, xs: &'a [T]) -> &'a T {
        &xs[self.below(xs.len())]
    }
}

pub const HEX_LOWER: &[u8] = b"0123456789abcdef";
pub const HEX_UPPER: &[u8] = b"0123456789ABCDEF";
pub const DIGITS: &[u8] = b"0123456789";
pub const ALPHA_LOWER: &[u8] = b"abcdef";
pub const ALPHA_UPPER: &[u8] = b"ABCDEF";
pub const MIXED: &[u8] = b"0123456789abcdefABCDEF";

/// `true` for the bytes the C accepts as hex digits (`c_num0 | c_alpha0 != 0`).
pub fn is_c_hex(b: u8) -> bool {
    b.is_ascii_digit() || (b'a'..=b'f').contains(&b) || (b'A'..=b'F').contains(&b)
}
