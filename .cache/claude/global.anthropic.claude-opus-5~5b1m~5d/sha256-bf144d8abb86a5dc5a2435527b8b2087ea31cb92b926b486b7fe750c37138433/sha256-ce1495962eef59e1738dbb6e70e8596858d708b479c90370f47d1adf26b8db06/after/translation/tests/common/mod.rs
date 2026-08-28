//! Shared differential-test harness.
//!
//! Both implementations are loaded as shared objects through `libloading` and
//! called across the FFI boundary. The Rust function is *never* called
//! directly from the test crate — it is resolved by symbol name out of
//! `libdriver.so`, exactly as an external C consumer would, so the
//! `#[no_mangle] extern "C"` export wrapper is part of what is under test.

#![allow(dead_code)]

use std::ffi::c_char;
use std::path::PathBuf;
use std::sync::OnceLock;

use libloading::{Library, Symbol};

/// ABI of the one public entry point: `char *tool_basename(char *path)`.
pub type ToolBasenameFn = unsafe extern "C" fn(*mut c_char) -> *mut c_char;

/// Which implementation a result came from (for assertion messages).
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Impl {
    C,
    Rust,
}

/// A loaded implementation: the `Library` is leaked into a `'static` so the
/// resolved function pointer stays valid for the whole test process.
pub struct Driver {
    pub which: Impl,
    pub path: PathBuf,
    pub tool_basename: ToolBasenameFn,
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// Locate the C shared object built by CMake.
fn c_so_path() -> PathBuf {
    if let Some(p) = std::env::var_os("DRIVER_C_SO") {
        return PathBuf::from(p);
    }
    let root = manifest_dir().parent().expect("crate has a parent dir").to_path_buf();
    let candidates = [
        root.join("c_src/build/libdriver.so"),
        root.join("c_src/build/lib/libdriver.so"),
    ];
    for c in &candidates {
        if c.is_file() {
            return c.clone();
        }
    }
    panic!(
        "C shared object not found. Build it with:\n  \
         cd c_src && mkdir -p build && cd build && \
         cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .\n\
         Looked in: {candidates:?}\n\
         (or set DRIVER_C_SO)"
    );
}

/// Locate the Rust `cdylib`. Honours `DRIVER_RUST_SO` so the very same test
/// suite can be re-run against the release artifact.
fn rust_so_path() -> PathBuf {
    if let Some(p) = std::env::var_os("DRIVER_RUST_SO") {
        return PathBuf::from(p);
    }
    let base = manifest_dir().join("target");
    // Prefer the profile the tests themselves were built with.
    let candidates = if cfg!(debug_assertions) {
        [base.join("debug/libdriver.so"), base.join("release/libdriver.so")]
    } else {
        [base.join("release/libdriver.so"), base.join("debug/libdriver.so")]
    };
    for c in &candidates {
        if c.is_file() {
            return c.clone();
        }
    }
    panic!(
        "Rust cdylib not found. Build it with `cargo build` (and/or \
         `cargo build --release`).\nLooked in: {candidates:?}\n(or set DRIVER_RUST_SO)"
    );
}

/// Guard against the single most dangerous failure mode of this harness:
/// `cargo test` does **not** rebuild a `cdylib`-only lib target, so without this
/// check the whole suite could pass against a stale `libdriver.so` that no
/// longer corresponds to `src/lib.rs`.
///
/// Fails loudly (rather than silently testing the wrong artifact) if any source
/// input is newer than the shared object.
fn assert_not_stale(so: &PathBuf) {
    let so_mtime = match std::fs::metadata(so).and_then(|m| m.modified()) {
        Ok(t) => t,
        Err(e) => panic!("cannot stat {}: {e}", so.display()),
    };

    let mut inputs = vec![manifest_dir().join("Cargo.toml")];
    let src = manifest_dir().join("src");
    if let Ok(rd) = std::fs::read_dir(&src) {
        for entry in rd.flatten() {
            if entry.path().extension().is_some_and(|e| e == "rs") {
                inputs.push(entry.path());
            }
        }
    }

    for input in inputs {
        if let Ok(t) = std::fs::metadata(&input).and_then(|m| m.modified()) {
            if t > so_mtime {
                panic!(
                    "STALE ARTIFACT: {} is newer than {}.\n\
                     `cargo test` does not rebuild a cdylib-only lib target, so these \
                     tests would have run against an out-of-date shared object.\n\
                     Run `cargo build && cargo build --release` (or ./verify.sh) first.",
                    input.display(),
                    so.display()
                );
            }
        }
    }
}

unsafe fn load(which: Impl, path: PathBuf) -> Driver {
    let lib = unsafe { Library::new(&path) }
        .unwrap_or_else(|e| panic!("failed to dlopen {}: {e}", path.display()));
    let lib: &'static Library = Box::leak(Box::new(lib));
    let sym: Symbol<'static, ToolBasenameFn> = unsafe { lib.get(b"tool_basename\0") }
        .unwrap_or_else(|e| {
            panic!("symbol `tool_basename` missing from {}: {e}", path.display())
        });
    Driver { which, path, tool_basename: *sym }
}

/// The C implementation (`c_src/build/libdriver.so`).
pub fn c_driver() -> &'static Driver {
    static C: OnceLock<Driver> = OnceLock::new();
    C.get_or_init(|| unsafe { load(Impl::C, c_so_path()) })
}

/// The Rust implementation (`target/{debug,release}/libdriver.so`).
pub fn rust_driver() -> &'static Driver {
    static R: OnceLock<Driver> = OnceLock::new();
    R.get_or_init(|| {
        let p = rust_so_path();
        assert_not_stale(&p);
        unsafe { load(Impl::Rust, p) }
    })
}

/// The observable result of one `tool_basename` call.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Outcome {
    /// Byte offset of the returned pointer from the buffer base. This is the
    /// complete information content of the return value: the result always
    /// aliases the caller's buffer, so comparing offsets *is* the byte-exact
    /// comparison of the returned pointer.
    pub offset: isize,
    /// The returned C string's bytes (excluding the NUL).
    pub result: Vec<u8>,
    /// The whole buffer after the call, to prove nothing was mutated.
    pub buffer_after: Vec<u8>,
    /// Whether the implementation returned NULL (the C never does).
    pub was_null: bool,
}

/// Call one implementation on a private copy of `input` (which must not contain
/// an interior NUL) and record everything observable.
///
/// `input` is the string *content*; a NUL terminator is appended here.
pub fn call(driver: &Driver, input: &[u8]) -> Outcome {
    call_raw(driver, input, 0)
}

/// Like [`call`], but appends `slack` extra bytes of garbage *after* the NUL
/// terminator, to prove the implementation stops at the terminator.
pub fn call_raw(driver: &Driver, input: &[u8], slack: usize) -> Outcome {
    let mut buf: Vec<u8> = Vec::with_capacity(input.len() + 1 + slack);
    buf.extend_from_slice(input);
    buf.push(0);
    // Recognisable garbage past the terminator, including both separators.
    for i in 0..slack {
        buf.push(match i % 4 {
            0 => b'/',
            1 => b'\\',
            2 => 0xFF,
            _ => b'X',
        });
    }

    let base = buf.as_mut_ptr() as *mut c_char;
    let ret = unsafe { (driver.tool_basename)(base) };

    let was_null = ret.is_null();
    let (offset, result) = if was_null {
        (isize::MIN, Vec::new())
    } else {
        let off = unsafe { ret.offset_from(base) };
        let bytes = unsafe { std::ffi::CStr::from_ptr(ret) }.to_bytes().to_vec();
        (off, bytes)
    };

    Outcome { offset, result, buffer_after: buf, was_null }
}

/// Core differential assertion: run `input` through BOTH shared objects and
/// require byte-identical observable behaviour.
///
/// Each implementation gets its own buffer copy, so neither can mask a
/// difference by mutating shared state.
pub fn assert_same(input: &[u8]) -> Outcome {
    assert_same_with_slack(input, 0)
}

pub fn assert_same_with_slack(input: &[u8], slack: usize) -> Outcome {
    let c = call_raw(c_driver(), input, slack);
    let r = call_raw(rust_driver(), input, slack);

    let show = Pretty(input);

    assert!(!c.was_null, "C returned NULL for {show} (it never should)");
    assert_eq!(
        c.was_null, r.was_null,
        "NULL-ness differs for {show}: C null={} Rust null={}",
        c.was_null, r.was_null
    );
    assert_eq!(
        c.offset, r.offset,
        "returned pointer OFFSET differs for {show}: C=+{} Rust=+{}\n  C result  = {}\n  Rust result = {}",
        c.offset,
        r.offset,
        Pretty(&c.result),
        Pretty(&r.result)
    );
    assert_eq!(
        c.result,
        r.result,
        "returned STRING differs for {show}: C={} Rust={}",
        Pretty(&c.result),
        Pretty(&r.result)
    );
    // Neither implementation may write through the pointer.
    let mut pristine = input.to_vec();
    pristine.push(0);
    assert_eq!(
        &c.buffer_after[..pristine.len()],
        &pristine[..],
        "C mutated the input buffer for {show}"
    );
    assert_eq!(
        &r.buffer_after[..pristine.len()],
        &pristine[..],
        "Rust mutated the input buffer for {show}"
    );
    assert_eq!(
        c.buffer_after, r.buffer_after,
        "buffers diverged after the call for {show}"
    );
    c
}

/// Byte-string formatter that stays readable for binary input.
pub struct Pretty<'a>(pub &'a [u8]);

impl std::fmt::Display for Pretty<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "b\"")?;
        for &b in self.0.iter().take(160) {
            match b {
                b'\\' => write!(f, "\\\\")?,
                b'"' => write!(f, "\\\"")?,
                0x20..=0x7E => write!(f, "{}", b as char)?,
                _ => write!(f, "\\x{b:02x}")?,
            }
        }
        if self.0.len() > 160 {
            write!(f, "\"...(len={})", self.0.len())
        } else {
            write!(f, "\" (len={})", self.0.len())
        }
    }
}

/// Deterministic PCG32 so every "randomized" row is reproducible from a seed.
pub struct Rng {
    state: u64,
    inc: u64,
}

impl Rng {
    pub fn new(seed: u64) -> Self {
        let mut r = Rng { state: 0, inc: (seed << 1) | 1 };
        r.next_u32();
        r.state = r.state.wrapping_add(0x853c_49e6_748f_ea9b ^ seed);
        r.next_u32();
        r
    }

    pub fn next_u32(&mut self) -> u32 {
        let old = self.state;
        self.state = old
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(self.inc);
        let xorshifted = (((old >> 18) ^ old) >> 27) as u32;
        let rot = (old >> 59) as u32;
        xorshifted.rotate_right(rot)
    }

    /// Uniform in `[0, n)`; returns 0 when `n == 0`.
    pub fn below(&mut self, n: usize) -> usize {
        if n == 0 {
            return 0;
        }
        (self.next_u32() as usize) % n
    }

    /// Uniform in `[lo, hi]`.
    pub fn range(&mut self, lo: usize, hi: usize) -> usize {
        debug_assert!(lo <= hi);
        lo + self.below(hi - lo + 1)
    }

    pub fn byte(&mut self) -> u8 {
        (self.next_u32() >> 7) as u8
    }

    /// A byte that is never NUL (interior NUL would truncate the C string) and
    /// never a separator, so tests can place separators deliberately.
    pub fn plain_byte(&mut self) -> u8 {
        loop {
            let b = self.byte();
            if b != 0 && b != b'/' && b != b'\\' {
                return b;
            }
        }
    }

    pub fn pick<T: Copy>(&mut self, xs: &[T]) -> T {
        xs[self.below(xs.len())]
    }

    pub fn bool(&mut self) -> bool {
        self.next_u32() & 1 == 1
    }
}

/// Reference model of the C, used only as a cross-check on the *test* logic
/// (the real oracle is always the C `.so`).
pub fn model(input: &[u8]) -> isize {
    let s1 = input.iter().rposition(|&b| b == b'/');
    let s2 = input.iter().rposition(|&b| b == b'\\');
    match (s1, s2) {
        (Some(a), Some(b)) => (a.max(b) + 1) as isize,
        (Some(a), None) => (a + 1) as isize,
        (None, Some(b)) => (b + 1) as isize,
        (None, None) => 0,
    }
}
