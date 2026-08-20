//! Shared differential-test harness.
//!
//! Both implementations are loaded as shared objects through `libloading` and
//! called only through their exported C ABI symbol `hex2bin`. The Rust
//! implementation is *never* called directly as a Rust function, so the
//! `#[no_mangle] extern "C"` export wrapper is under test as well.

#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::ffi::{c_char, c_int};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

pub type Hex2BinFn = unsafe extern "C" fn(
    *mut u8,     // bin
    usize,       // bin_maxlen
    *const c_char, // hex
    usize,       // hex_len
    *const c_char, // ignore
    *mut *const c_char, // hex_end_p
) -> c_int;

struct Loaded {
    c: Hex2BinFn,
    rust: Hex2BinFn,
    _libs: Vec<Library>,
}

// The function pointers point into `dlopen`ed images that are kept alive for
// the whole process lifetime by `_libs`.
unsafe impl Send for Loaded {}
unsafe impl Sync for Loaded {}

static LOADED: OnceLock<Loaded> = OnceLock::new();

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn find_so(dir: &Path, stem: &str) -> Option<PathBuf> {
    let entries = std::fs::read_dir(dir).ok()?;
    for e in entries.flatten() {
        let p = e.path();
        let name = p.file_name()?.to_string_lossy().to_string();
        if name.starts_with(stem) && name.ends_with(".so") {
            return Some(p);
        }
    }
    None
}

fn c_so_path() -> PathBuf {
    if let Some(p) = std::env::var_os("HEX2BIN_C_SO") {
        return PathBuf::from(p);
    }
    let build = manifest_dir().join("c_src/build");
    find_so(&build, "lib").unwrap_or_else(|| {
        panic!(
            "no C shared object found in {:?}; build it with:\n  cd c_src && mkdir -p build && \
             cd build && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
            build
        )
    })
}

fn rust_so_path() -> PathBuf {
    if let Some(p) = std::env::var_os("HEX2BIN_RUST_SO") {
        return PathBuf::from(p);
    }
    let mut candidates: Vec<PathBuf> = Vec::new();
    if let Ok(exe) = std::env::current_exe() {
        // target/<profile>/deps/<test-bin>
        if let Some(deps) = exe.parent() {
            candidates.push(deps.to_path_buf());
            if let Some(profile) = deps.parent() {
                candidates.push(profile.to_path_buf());
            }
        }
    }
    candidates.push(manifest_dir().join("target/debug"));
    candidates.push(manifest_dir().join("target/release"));

    for dir in &candidates {
        if let Some(p) = find_so(dir, "libhex2bin_lib") {
            return p;
        }
    }
    panic!(
        "no Rust cdylib (libhex2bin_lib*.so) found; looked in {:?}. Run `cargo build` first.",
        candidates
    );
}

/// Guard against a stale shared object: if a `.so` is older than any of its
/// sources, the differential test would compare an outdated binary and could
/// report a false PASS.
fn assert_fresh(so: &Path, src_dirs: &[PathBuf]) {
    let so_m = match std::fs::metadata(so).and_then(|m| m.modified()) {
        Ok(m) => m,
        Err(_) => return,
    };
    for dir in src_dirs {
        let rd = match std::fs::read_dir(dir) {
            Ok(rd) => rd,
            Err(_) => continue,
        };
        for e in rd.flatten() {
            let p = e.path();
            if !p.is_file() {
                continue;
            }
            if let Ok(m) = e.metadata().and_then(|m| m.modified()) {
                assert!(
                    m <= so_m,
                    "STALE SHARED OBJECT: {:?} is older than its source {:?}. \
                     Rebuild before testing (cargo build / cmake --build).",
                    so,
                    p
                );
            }
        }
    }
}

fn load_one(path: &Path) -> (Library, Hex2BinFn) {
    unsafe {
        let lib = Library::new(path).unwrap_or_else(|e| panic!("dlopen({:?}) failed: {e}", path));
        let f = {
            let sym: Symbol<Hex2BinFn> = lib
                .get(b"hex2bin\0")
                .unwrap_or_else(|e| panic!("dlsym(hex2bin) in {:?} failed: {e}", path));
            *sym
        };
        (lib, f)
    }
}

fn load() -> Loaded {
    let c_path = c_so_path();
    let rust_path = rust_so_path();
    assert_fresh(
        &c_path,
        &[
            manifest_dir().join("c_src/src"),
            manifest_dir().join("c_src/include"),
        ],
    );
    assert_fresh(&rust_path, &[manifest_dir().join("src")]);
    if std::env::var_os("HEX2BIN_DIFF_VERBOSE").is_some() {
        eprintln!("C   .so: {:?}", c_path);
        eprintln!("RS  .so: {:?}", rust_path);
    }
    let (clib, c) = load_one(&c_path);
    let (rlib, rust) = load_one(&rust_path);
    assert_ne!(
        c as usize, rust as usize,
        "both handles resolved to the same `hex2bin` implementation"
    );
    Loaded {
        c,
        rust,
        _libs: vec![clib, rlib],
    }
}

/// Paths of the two shared objects under comparison (for reporting).
pub fn so_paths() -> (PathBuf, PathBuf) {
    (c_so_path(), rust_so_path())
}

/// `(c_impl, rust_impl)`
pub fn impls() -> (Hex2BinFn, Hex2BinFn) {
    let l = LOADED.get_or_init(load);
    (l.c, l.rust)
}

// ---------------------------------------------------------------------------
// Test case description
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Case {
    /// Backing bytes for the `hex` argument.
    pub hex: Vec<u8>,
    /// `hex_len` argument. Normally `hex.len()`; may be smaller (partial view)
    /// or, for the "oversized length" error rows, larger (only safe when the
    /// parse is guaranteed to stop on the first byte).
    pub hex_len: usize,
    /// `bin_maxlen` argument.
    pub bin_maxlen: usize,
    /// Bytes actually allocated for `bin` (must be >= min(bin_maxlen, hex_len/2)).
    pub bin_cap: usize,
    /// `ignore` argument; a terminating NUL is appended automatically.
    /// `None` == NULL pointer.
    pub ignore: Option<Vec<u8>>,
    /// Pass a non-NULL `hex_end_p`?
    pub want_end: bool,
    /// Alias `bin` onto the `hex` buffer (in-place decoding).
    pub in_place: bool,
    /// Pass NULL for `bin` (only meaningful when nothing can be written).
    pub null_bin: bool,
    /// Pass NULL for `hex` (only meaningful when `hex_len == 0`).
    pub null_hex: bool,
}

impl Case {
    pub fn new(hex: impl Into<Vec<u8>>) -> Case {
        let hex = hex.into();
        let hex_len = hex.len();
        let bin_maxlen = hex_len / 2;
        Case {
            hex,
            hex_len,
            bin_maxlen,
            bin_cap: bin_maxlen + 8,
            ignore: None,
            want_end: true,
            in_place: false,
            null_bin: false,
            null_hex: false,
        }
    }

    pub fn hex_len(mut self, n: usize) -> Case {
        self.hex_len = n;
        self.fix_cap();
        self
    }
    pub fn bin_maxlen(mut self, n: usize) -> Case {
        self.bin_maxlen = n;
        self.fix_cap();
        self
    }
    pub fn bin_cap(mut self, n: usize) -> Case {
        self.bin_cap = n;
        self
    }
    pub fn ignore(mut self, s: impl Into<Vec<u8>>) -> Case {
        let v: Vec<u8> = s.into();
        assert!(!v.contains(&0), "ignore set must not contain interior NUL");
        self.ignore = Some(v);
        self
    }
    pub fn no_ignore(mut self) -> Case {
        self.ignore = None;
        self
    }
    pub fn want_end(mut self, b: bool) -> Case {
        self.want_end = b;
        self
    }
    pub fn in_place(mut self, b: bool) -> Case {
        self.in_place = b;
        self
    }
    pub fn null_bin(mut self, b: bool) -> Case {
        self.null_bin = b;
        self
    }
    pub fn null_hex(mut self, b: bool) -> Case {
        self.null_hex = b;
        self
    }

    /// Keep `bin_cap` big enough that a conforming implementation can never
    /// write out of bounds: at most `min(bin_maxlen, hex_len/2)` bytes are
    /// stored, and we add slack so that stray writes past the limit are caught.
    fn fix_cap(&mut self) {
        let max_written = std::cmp::min(self.bin_maxlen, self.hex_len / 2);
        self.bin_cap = max_written.saturating_add(8);
    }
}

/// Everything an external caller can observe from one call.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Outcome {
    pub ret: c_int,
    /// Full `bin` allocation afterwards (including the slack past `bin_maxlen`).
    pub bin: Vec<u8>,
    /// `*hex_end_p - hex` when `hex_end_p != NULL`.
    pub hex_end: Option<isize>,
    /// The `hex` buffer afterwards (detects unexpected writes / in-place result).
    pub hex_after: Vec<u8>,
}

fn bin_fill(i: usize) -> u8 {
    // Deterministic non-trivial pattern so misplaced writes are visible.
    (i as u8).wrapping_mul(31).wrapping_add(0xA5)
}

pub fn run(f: Hex2BinFn, case: &Case) -> Outcome {
    let mut hex_buf: Vec<u8> = case.hex.clone();
    let mut bin_buf: Vec<u8> = (0..case.bin_cap).map(bin_fill).collect();

    // Sentinel that no conforming implementation can produce, so "was it
    // written?" is observable.
    let mut end_ptr: *const c_char = usize::MAX as *const c_char;

    let hex_ptr: *const c_char = if case.null_hex {
        std::ptr::null()
    } else {
        hex_buf.as_ptr() as *const c_char
    };

    let ignore_cstr: Option<Vec<u8>> = case.ignore.as_ref().map(|v| {
        let mut v2 = v.clone();
        v2.push(0);
        v2
    });
    let ignore_ptr: *const c_char = match &ignore_cstr {
        Some(v) => v.as_ptr() as *const c_char,
        None => std::ptr::null(),
    };

    let bin_ptr: *mut u8 = if case.null_bin {
        std::ptr::null_mut()
    } else if case.in_place {
        hex_buf.as_mut_ptr()
    } else {
        bin_buf.as_mut_ptr()
    };

    let end_arg: *mut *const c_char = if case.want_end {
        &mut end_ptr as *mut *const c_char
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
            end_arg,
        )
    };

    let hex_end = if case.want_end {
        Some((end_ptr as isize).wrapping_sub(hex_ptr as isize))
    } else {
        // Must not have been touched.
        assert_eq!(
            end_ptr,
            usize::MAX as *const c_char,
            "hex_end_p was NULL but the sentinel changed"
        );
        None
    };

    Outcome {
        ret,
        bin: bin_buf,
        hex_end,
        hex_after: hex_buf,
    }
}

/// Run both implementations on the same case and assert identical observable
/// behaviour.
#[track_caller]
pub fn check(case: &Case) {
    let (c, rust) = impls();
    let oc = run(c, case);
    let or = run(rust, case);
    if oc != or {
        panic!(
            "DIVERGENCE\n case      : {:?}\n hex (esc) : {}\n ignore    : {:?}\n C   ret={} hex_end={:?}\n     bin={:02x?}\n     hex_after={:02x?}\n RS  ret={} hex_end={:?}\n     bin={:02x?}\n     hex_after={:02x?}\n",
            case,
            escape(&case.hex),
            case.ignore.as_ref().map(|v| escape(v)),
            oc.ret,
            oc.hex_end,
            oc.bin,
            oc.hex_after,
            or.ret,
            or.hex_end,
            or.bin,
            or.hex_after,
        );
    }
}

/// Like [`check`], but also returns the (identical) outcome for extra
/// assertions about the C reference behaviour itself.
#[track_caller]
pub fn check_and_get(case: &Case) -> Outcome {
    check(case);
    let (c, _) = impls();
    run(c, case)
}

pub fn escape(b: &[u8]) -> String {
    let mut s = String::from("\"");
    for &x in b {
        if x.is_ascii_graphic() || x == b' ' {
            s.push(x as char);
        } else {
            s.push_str(&format!("\\x{:02x}", x));
        }
    }
    s.push('"');
    s
}

// ---------------------------------------------------------------------------
// Deterministic RNG (splitmix64) — property-style testing with a fixed seed
// ---------------------------------------------------------------------------

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
    /// Uniform-ish in `0..n`.
    pub fn below(&mut self, n: usize) -> usize {
        assert!(n > 0);
        (self.next_u64() % (n as u64)) as usize
    }
    pub fn range(&mut self, lo: usize, hi_inclusive: usize) -> usize {
        lo + self.below(hi_inclusive - lo + 1)
    }
    pub fn byte(&mut self) -> u8 {
        (self.next_u64() >> 24) as u8
    }
    pub fn bool(&mut self) -> bool {
        self.next_u64() & 1 == 1
    }
    pub fn pick<'a, T>(&mut self, xs: &'a [T]) -> &'a T {
        &xs[self.below(xs.len())]
    }
}

pub const LOWER: &[u8] = b"0123456789abcdef";
pub const UPPER: &[u8] = b"0123456789ABCDEF";
pub const DIGITS_ONLY: &[u8] = b"0123456789";
pub const LETTERS_LOWER: &[u8] = b"abcdef";
pub const LETTERS_UPPER: &[u8] = b"ABCDEF";
pub const MIXED: &[u8] = b"0123456789abcdefABCDEF";
/// A typical `ignore` set (separators).
pub const SEPS: &[u8] = b" \t\r\n:-";

/// All bytes that `hex2bin` accepts as a nibble, per the C classifier.
pub fn is_hex_digit(c: u8) -> bool {
    c.is_ascii_digit() || (b'a'..=b'f').contains(&c) || (b'A'..=b'F').contains(&c)
}

pub fn random_stream(rng: &mut Rng, n: usize, alphabet: &[u8]) -> Vec<u8> {
    (0..n).map(|_| *rng.pick(alphabet)).collect()
}
