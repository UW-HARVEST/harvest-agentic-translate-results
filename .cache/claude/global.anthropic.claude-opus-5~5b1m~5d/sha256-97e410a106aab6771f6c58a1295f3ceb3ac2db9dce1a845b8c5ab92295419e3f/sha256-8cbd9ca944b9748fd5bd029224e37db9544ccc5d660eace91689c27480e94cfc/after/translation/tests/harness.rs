//! Shared differential-testing harness.
//!
//! BOTH implementations are loaded as shared objects via `libloading` and
//! called through their exported `hex2bin` C symbol. The Rust function is
//! never called directly, so the `#[no_mangle] extern "C"` wrapper is under
//! test too.

#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::ffi::c_char;
use std::path::PathBuf;
use std::sync::OnceLock;

pub type Hex2BinFn = unsafe extern "C" fn(
    *mut u8,       // bin
    usize,         // bin_maxlen
    *const c_char, // hex
    usize,         // hex_len
    *const c_char, // ignore
    *mut *const c_char,
) -> i32;

/// The observable result of one `hex2bin` call, captured completely so that
/// C and Rust can be compared byte-for-byte.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Outcome {
    /// The `int` return value.
    pub ret: i32,
    /// The FULL output buffer after the call, including bytes past `bin_pos`
    /// and the guard bytes, so out-of-bounds writes are caught.
    pub bin: Vec<u8>,
    /// `*hex_end_p` expressed as an offset from `hex`, or `None` if
    /// `hex_end_p` was NULL. `isize` because line 44 can rewind.
    pub hex_end_off: Option<isize>,
}

fn workspace_root() -> PathBuf {
    // CARGO_MANIFEST_DIR = <root>/translation
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation dir has a parent")
        .to_path_buf()
}

fn find_c_so() -> PathBuf {
    let build = workspace_root().join("c_src").join("build");
    let mut candidates: Vec<PathBuf> = Vec::new();
    if let Ok(rd) = std::fs::read_dir(&build) {
        for e in rd.flatten() {
            let p = e.path();
            let name = p.file_name().unwrap_or_default().to_string_lossy().to_string();
            if name.starts_with("lib") && name.ends_with(".so") {
                candidates.push(p);
            }
        }
    }
    candidates.sort();
    candidates.into_iter().next().unwrap_or_else(|| {
        panic!(
            "no C .so found in {}. Build it with:\n  cd c_src && mkdir -p build && cd build && \
             cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
            build.display()
        )
    })
}

fn find_rust_so() -> PathBuf {
    // IMPORTANT: `cargo test` does NOT rebuild a cdylib-only lib target,
    // because an integration test cannot link against a cdylib and therefore
    // does not depend on it. Picking up a stale `.so` would make these
    // differential tests vacuous, so the path is taken from
    // HEX2BIN_RUST_SO when set and the artifact's mtime is checked against
    // src/lib.rs. Use `./run-tests.sh`, which builds first.
    let p = if let Ok(env_path) = std::env::var("HEX2BIN_RUST_SO") {
        PathBuf::from(env_path)
    } else {
        // The integration-test binary lives in target/<profile>/deps/.
        let exe = std::env::current_exe().expect("current_exe");
        let profile_dir = exe
            .parent()
            .and_then(|p| p.parent())
            .expect("target/<profile>")
            .to_path_buf();
        let mut found = None;
        for dir in [
            profile_dir.clone(),
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/release"),
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/debug"),
        ] {
            for n in ["libhex2bin_lib.so", "libtranslation.so"] {
                let c = dir.join(n);
                if c.exists() {
                    found = Some(c);
                    break;
                }
            }
            if found.is_some() {
                break;
            }
        }
        found.unwrap_or_else(|| {
            panic!(
                "no Rust cdylib found near {}. Run ./run-tests.sh (it builds the \
                 cdylib first) or set HEX2BIN_RUST_SO.",
                profile_dir.display()
            )
        })
    };

    if !p.exists() {
        panic!("Rust cdylib {} does not exist", p.display());
    }

    // Staleness guard: the .so must be at least as new as the source.
    let src = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/lib.rs");
    if let (Ok(so_m), Ok(src_m)) = (
        std::fs::metadata(&p).and_then(|m| m.modified()),
        std::fs::metadata(&src).and_then(|m| m.modified()),
    ) {
        if so_m < src_m {
            panic!(
                "STALE Rust cdylib: {} is older than {}. Rebuild with \
                 `cargo build --release` (or run ./run-tests.sh) — testing a \
                 stale artifact would make these differential tests vacuous.",
                p.display(),
                src.display()
            );
        }
    }
    p
}

struct Impls {
    _c_lib: Library,
    _rs_lib: Library,
    c: Hex2BinFn,
    rs: Hex2BinFn,
}

// Safety: the libraries are leaked for the process lifetime and the function
// pointers are plain code addresses; `hex2bin` holds no global state.
unsafe impl Send for Impls {}
unsafe impl Sync for Impls {}

fn impls() -> &'static Impls {
    static IMPLS: OnceLock<Impls> = OnceLock::new();
    IMPLS.get_or_init(|| unsafe {
        let c_path = find_c_so();
        let rs_path = find_rust_so();
        let c_lib = Library::new(&c_path)
            .unwrap_or_else(|e| panic!("dlopen {}: {e}", c_path.display()));
        let rs_lib = Library::new(&rs_path)
            .unwrap_or_else(|e| panic!("dlopen {}: {e}", rs_path.display()));
        let c: Symbol<Hex2BinFn> = c_lib
            .get(b"hex2bin\0")
            .unwrap_or_else(|e| panic!("C hex2bin: {e}"));
        let rs: Symbol<Hex2BinFn> = rs_lib
            .get(b"hex2bin\0")
            .unwrap_or_else(|e| panic!("Rust hex2bin: {e}"));
        let c = *c;
        let rs = *rs;
        Impls {
            _c_lib: c_lib,
            _rs_lib: rs_lib,
            c,
            rs,
        }
    })
}

/// One fully-specified `hex2bin` invocation.
#[derive(Debug, Clone)]
pub struct Call {
    /// Size of the caller-allocated output buffer.
    pub bin_cap: usize,
    /// Byte the buffer is pre-filled with, so untouched bytes are verified.
    pub poison: u8,
    /// `bin_maxlen` argument (may differ from `bin_cap`; must be <= bin_cap
    /// unless the C is expected to reject before writing).
    pub bin_maxlen: usize,
    /// The `hex` bytes. Passed with an explicit length; NOT NUL-terminated.
    pub hex: Vec<u8>,
    /// `hex_len` argument (defaults to `hex.len()`).
    pub hex_len: usize,
    /// `ignore` set; `None` => NULL pointer. NUL-terminated when `Some`.
    pub ignore: Option<Vec<u8>>,
    /// Whether to pass a non-NULL `hex_end_p`.
    pub want_hex_end: bool,
    /// Pass `bin = NULL`.
    pub bin_null: bool,
    /// Pass `hex = NULL`.
    pub hex_null: bool,
}

impl Call {
    pub fn new(hex: impl Into<Vec<u8>>) -> Self {
        let hex = hex.into();
        let hex_len = hex.len();
        Call {
            bin_cap: hex_len / 2 + 8,
            poison: 0xA5,
            bin_maxlen: hex_len / 2,
            hex,
            hex_len,
            ignore: None,
            want_hex_end: true,
            bin_null: false,
            hex_null: false,
        }
    }
    pub fn bin_maxlen(mut self, n: usize) -> Self {
        self.bin_maxlen = n;
        self
    }
    pub fn bin_cap(mut self, n: usize) -> Self {
        self.bin_cap = n;
        self
    }
    pub fn poison(mut self, b: u8) -> Self {
        self.poison = b;
        self
    }
    pub fn hex_len(mut self, n: usize) -> Self {
        self.hex_len = n;
        self
    }
    pub fn ignore(mut self, s: &[u8]) -> Self {
        let mut v = s.to_vec();
        v.push(0);
        self.ignore = Some(v);
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
    pub fn bin_null(mut self) -> Self {
        self.bin_null = true;
        self
    }
    pub fn hex_null(mut self) -> Self {
        self.hex_null = true;
        self
    }

    fn invoke(&self, f: Hex2BinFn) -> Outcome {
        let mut buf = vec![self.poison; self.bin_cap];
        let mut hex = self.hex.clone();
        // Give the reader some slack so an over-read of the C version does not
        // segfault before we can observe the divergence.
        hex.extend_from_slice(&[0xEE; 16]);

        let bin_ptr = if self.bin_null {
            std::ptr::null_mut()
        } else {
            buf.as_mut_ptr()
        };
        let hex_ptr = if self.hex_null {
            std::ptr::null()
        } else {
            hex.as_ptr() as *const c_char
        };

        let mut end: *const c_char = std::ptr::null();
        let end_ptr = if self.want_hex_end {
            &mut end as *mut *const c_char
        } else {
            std::ptr::null_mut()
        };

        let ret = unsafe {
            f(
                bin_ptr,
                self.bin_maxlen,
                hex_ptr,
                self.hex_len,
                self.ignore
                    .as_ref()
                    .map(|v| v.as_ptr() as *const c_char)
                    .unwrap_or(std::ptr::null()),
                end_ptr,
            )
        };

        let hex_end_off = if self.want_hex_end {
            // Offset relative to the hex base pointer, even when hex is NULL
            // (NULL + 0 == NULL => offset 0).
            let base = if self.hex_null {
                std::ptr::null::<c_char>()
            } else {
                hex.as_ptr() as *const c_char
            };
            Some((end as isize).wrapping_sub(base as isize))
        } else {
            None
        };

        Outcome {
            ret,
            bin: buf,
            hex_end_off,
        }
    }

    /// Run the C implementation via its `.so`.
    pub fn run_c(&self) -> Outcome {
        self.invoke(impls().c)
    }
    /// Run the Rust implementation via its `.so`.
    pub fn run_rust(&self) -> Outcome {
        self.invoke(impls().rs)
    }
}

/// Assert C and Rust agree completely for this call.
#[track_caller]
pub fn assert_same(call: &Call, label: &str) {
    let c = call.run_c();
    let r = call.run_rust();
    if c != r {
        panic!(
            "DIVERGENCE [{label}]\n  call: bin_cap={} bin_maxlen={} hex_len={} \
             want_hex_end={} bin_null={} hex_null={}\n  ignore={:?}\n  hex={:02x?}\n\
             \n  C   : ret={} hex_end_off={:?}\n        bin={:02x?}\n\
             \n  RUST: ret={} hex_end_off={:?}\n        bin={:02x?}\n",
            call.bin_cap,
            call.bin_maxlen,
            call.hex_len,
            call.want_hex_end,
            call.bin_null,
            call.hex_null,
            call.ignore.as_ref().map(|v| String::from_utf8_lossy(v).to_string()),
            call.hex,
            c.ret,
            c.hex_end_off,
            c.bin,
            r.ret,
            r.hex_end_off,
            r.bin,
        );
    }
}

/// Assert agreement and additionally that the shared return value is `expect`
/// (used by the Phase C error-path tests so they check the *same specific*
/// sentinel, not merely "both failed").
#[track_caller]
pub fn assert_same_ret(call: &Call, expect: i32, label: &str) {
    assert_same(call, label);
    let c = call.run_c();
    assert_eq!(
        c.ret, expect,
        "[{label}] expected C return {expect}, got {} (hex={:02x?})",
        c.ret, call.hex
    );
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (xorshift64*) so every property test is reproducible.
// ---------------------------------------------------------------------------

pub const SEED: u64 = 0x5EED_1234_DEAD_BEEF;

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Rng(if seed == 0 { 1 } else { seed })
    }
    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
    pub fn u8(&mut self) -> u8 {
        (self.next_u64() >> 33) as u8
    }
    /// Uniform in `0..n` (n > 0).
    pub fn below(&mut self, n: usize) -> usize {
        (self.next_u64() % n as u64) as usize
    }
    /// Uniform in `lo..=hi`.
    pub fn range(&mut self, lo: usize, hi: usize) -> usize {
        lo + self.below(hi - lo + 1)
    }
    pub fn bool(&mut self) -> bool {
        self.next_u64() & 1 == 1
    }
    pub fn pick<'a, T>(&mut self, xs: &'a [T]) -> &'a T {
        &xs[self.below(xs.len())]
    }
    /// A random valid hex digit, random case for a..f.
    pub fn hex_digit(&mut self) -> u8 {
        const LOWER: &[u8; 16] = b"0123456789abcdef";
        const UPPER: &[u8; 16] = b"0123456789ABCDEF";
        let i = self.below(16);
        if self.bool() {
            LOWER[i]
        } else {
            UPPER[i]
        }
    }
    pub fn hex_digits(&mut self, n: usize) -> Vec<u8> {
        (0..n).map(|_| self.hex_digit()).collect()
    }
}

/// Number of randomized iterations per `CONFIGS.md` row.
pub const ITERS: usize = 400;
