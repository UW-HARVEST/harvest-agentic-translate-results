//! Shared differential-test harness.
//!
//! Loads BOTH shared objects with `libloading` and calls `hex2bin` through the
//! FFI boundary in each of them. The Rust implementation is never called
//! directly — always through `libhex2bin_lib.so`'s exported symbol, so the
//! `#[no_mangle] extern "C"` wrapper is exercised too.

#![allow(dead_code)]

use std::ffi::{c_char, c_int};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

/// `int hex2bin(uint8_t *bin, size_t bin_maxlen, const char *hex,
///              size_t hex_len, const char *ignore, const char **hex_end_p);`
pub type Hex2Bin = unsafe extern "C" fn(
    *mut u8,
    usize,
    *const c_char,
    usize,
    *const c_char,
    *mut *const c_char,
) -> c_int;

pub struct Libs {
    // Kept alive for the whole process so the fn pointers stay valid.
    _c: libloading::Library,
    _rust: libloading::Library,
    pub c: Hex2Bin,
    pub rust: Hex2Bin,
}

// The raw fn pointers are plain code addresses; the libraries are never unloaded.
unsafe impl Send for Libs {}
unsafe impl Sync for Libs {}

fn workspace_root() -> PathBuf {
    // CARGO_MANIFEST_DIR = <root>/translation
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("manifest dir has a parent")
        .to_path_buf()
}

/// Find the C `.so`. Its file name is derived from the parent directory name by
/// `c_src/CMakeLists.txt`, so glob for it rather than hard-coding.
fn c_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("C_SO_PATH") {
        return PathBuf::from(p);
    }
    let build_dir = workspace_root().join("c_src/build");
    let mut found: Vec<PathBuf> = std::fs::read_dir(&build_dir)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}. Build the C library first.", build_dir.display()))
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| {
            p.extension().map(|x| x == "so").unwrap_or(false)
                && p.file_name()
                    .and_then(|n| n.to_str())
                    .map(|n| n.starts_with("lib"))
                    .unwrap_or(false)
        })
        .collect();
    found.sort();
    assert_eq!(
        found.len(),
        1,
        "expected exactly one lib*.so in {}, found {:?}",
        build_dir.display(),
        found
    );
    found.pop().unwrap()
}

/// Build and locate the Rust `cdylib`.
///
/// `cargo test` does not emit a `cdylib` artifact, so the harness builds one
/// itself. The nested build uses a SEPARATE `CARGO_TARGET_DIR` so it cannot
/// deadlock on the target-directory lock held by the outer `cargo test`, and it
/// is rebuilt on every run so a stale `.so` can never be tested.
///
/// `DIFF_CARGO_ARGS` (space separated) is forwarded to the nested build so that
/// feature-combination runs build the matching `.so`.
fn rust_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("RUST_SO_PATH") {
        return PathBuf::from(p);
    }
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    let out_dir = manifest.join("target/differential");
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());

    let mut cmd = std::process::Command::new(&cargo);
    cmd.current_dir(manifest)
        .env("CARGO_TARGET_DIR", &out_dir)
        // Do not inherit the outer test run's rustflags/profile settings.
        .env_remove("RUSTFLAGS")
        .arg("build")
        .arg("--release")
        .arg("--lib");
    if let Ok(extra) = std::env::var("DIFF_CARGO_ARGS") {
        for a in extra.split_whitespace() {
            cmd.arg(a);
        }
    }
    let status = cmd
        .status()
        .unwrap_or_else(|e| panic!("failed to spawn `{cargo} build`: {e}"));
    assert!(status.success(), "nested `cargo build --release --lib` failed");

    let p = out_dir.join("release/libhex2bin_lib.so");
    assert!(p.exists(), "{} not produced by the nested build", p.display());
    p
}

pub fn libs() -> &'static Libs {
    static LIBS: OnceLock<Libs> = OnceLock::new();
    LIBS.get_or_init(|| unsafe {
        let cp = c_so_path();
        let rp = rust_so_path();
        let clib = libloading::Library::new(&cp)
            .unwrap_or_else(|e| panic!("loading C so {}: {e}", cp.display()));
        let rlib = libloading::Library::new(&rp)
            .unwrap_or_else(|e| panic!("loading Rust so {}: {e}", rp.display()));
        let csym: libloading::Symbol<Hex2Bin> = clib
            .get(b"hex2bin\0")
            .expect("C .so does not export hex2bin");
        let rsym: libloading::Symbol<Hex2Bin> = rlib
            .get(b"hex2bin\0")
            .expect("Rust .so does not export hex2bin");
        let c = *csym;
        let rust = *rsym;
        Libs {
            _c: clib,
            _rust: rlib,
            c,
            rust,
        }
    })
}

/// Guard bytes appended past `bin_maxlen` to detect out-of-bounds writes.
const GUARD: usize = 8;
const FILL: u8 = 0xA5;
/// Sentinel stored in the `hex_end_p` out-slot to detect "not written".
const HEX_END_SENTINEL: usize = 0xDEAD_BEEF_1234_5678;

/// One observed call outcome — everything the C call can possibly affect.
#[derive(PartialEq, Eq)]
pub struct Out {
    pub ret: c_int,
    /// The full output buffer, `bin_maxlen` bytes plus the guard region.
    pub bin: Vec<u8>,
    /// `*hex_end_p` normalised to a byte offset relative to the `hex` base
    /// pointer, or `None` when `hex_end_p` was NULL (nothing written).
    pub hex_end: Option<isize>,
    /// Raw value of the out-slot when it was *not* overwritten (sentinel check).
    pub hex_end_written: bool,
}

impl std::fmt::Debug for Out {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Out")
            .field("ret", &self.ret)
            .field("bin", &format_args!("{:02x?}", self.bin))
            .field("hex_end", &self.hex_end)
            .field("hex_end_written", &self.hex_end_written)
            .finish()
    }
}

/// How the `bin` argument should be passed.
#[derive(Clone, Copy, Debug)]
pub enum BinArg {
    /// Allocate a real buffer of `bin_maxlen + GUARD` bytes.
    Buffer,
    /// Pass a NULL `bin` pointer.
    Null,
    /// In-place decoding: `bin` and `hex` are THE SAME buffer. The C writes
    /// `bin[bin_pos]` while reading `hex[hex_pos]` with `bin_pos < hex_pos`, so
    /// this is legitimate usage and the write/read interleaving must match.
    AliasHex,
}

/// How the `hex` argument should be passed.
#[derive(Clone, Copy, Debug)]
pub enum HexArg {
    Buffer,
    Null,
}

/// A fully specified call to `hex2bin`.
#[derive(Clone, Debug)]
pub struct Case {
    pub bin_arg: BinArg,
    pub bin_maxlen: usize,
    pub hex: Vec<u8>,
    /// `hex_len` passed to the callee. Defaults to `hex.len()`.
    pub hex_len: usize,
    pub hex_arg: HexArg,
    /// `ignore` set, NUL terminator added automatically. `None` => NULL.
    pub ignore: Option<Vec<u8>>,
    pub want_hex_end: bool,
}

impl Case {
    pub fn new(hex: &[u8], bin_maxlen: usize) -> Self {
        Case {
            bin_arg: BinArg::Buffer,
            bin_maxlen,
            hex: hex.to_vec(),
            hex_len: hex.len(),
            hex_arg: HexArg::Buffer,
            ignore: None,
            want_hex_end: true,
        }
    }
    /// `bin_maxlen` exactly large enough for a fully valid even-length input.
    pub fn exact(hex: &[u8]) -> Self {
        Case::new(hex, hex.len() / 2)
    }
    pub fn ignore(mut self, set: Option<&[u8]>) -> Self {
        self.ignore = set.map(|s| s.to_vec());
        self
    }
    pub fn hex_end(mut self, want: bool) -> Self {
        self.want_hex_end = want;
        self
    }
    pub fn bin_maxlen(mut self, n: usize) -> Self {
        self.bin_maxlen = n;
        self
    }
    pub fn hex_len(mut self, n: usize) -> Self {
        self.hex_len = n;
        self
    }
    pub fn bin_null(mut self) -> Self {
        self.bin_arg = BinArg::Null;
        self
    }
    /// Decode in place: pass the same address as both `bin` and `hex`.
    pub fn in_place(mut self) -> Self {
        self.bin_arg = BinArg::AliasHex;
        self
    }
    pub fn hex_null(mut self) -> Self {
        self.hex_arg = HexArg::Null;
        self
    }
}

/// Invoke one implementation with the given case and capture everything.
fn run_one(f: Hex2Bin, case: &Case) -> Out {
    // A `bin_maxlen` of SIZE_MAX obviously cannot be allocated; the C code can
    // never write past the digits actually supplied, so a buffer that covers
    // every possible written byte is enough.
    let alloc = match case.bin_arg {
        BinArg::Null => 0,
        BinArg::AliasHex => case.hex.len().saturating_add(GUARD),
        BinArg::Buffer => case
            .bin_maxlen
            .min(case.hex.len().max(case.hex_len) + 1)
            .saturating_add(GUARD),
    };
    let mut buf = vec![FILL; alloc];
    if matches!(case.bin_arg, BinArg::AliasHex) {
        buf[..case.hex.len()].copy_from_slice(&case.hex);
    }

    let bin_ptr = match case.bin_arg {
        BinArg::Null => std::ptr::null_mut(),
        BinArg::Buffer | BinArg::AliasHex => buf.as_mut_ptr(),
    };

    let hex_ptr: *const c_char = match (case.bin_arg, case.hex_arg) {
        (_, HexArg::Null) => std::ptr::null(),
        // Same address for both arguments -> true in-place decoding.
        (BinArg::AliasHex, HexArg::Buffer) => buf.as_ptr() as *const c_char,
        (_, HexArg::Buffer) => case.hex.as_ptr() as *const c_char,
    };

    let ignore_buf: Option<Vec<u8>> = case.ignore.as_ref().map(|s| {
        let mut v = s.clone();
        v.push(0);
        v
    });
    let ignore_ptr: *const c_char = match &ignore_buf {
        None => std::ptr::null(),
        Some(v) => v.as_ptr() as *const c_char,
    };

    let mut hex_end_slot: *const c_char = HEX_END_SENTINEL as *const c_char;
    let hex_end_ptr: *mut *const c_char = if case.want_hex_end {
        &mut hex_end_slot
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

    let written = hex_end_slot as usize != HEX_END_SENTINEL;
    let hex_end = if case.want_hex_end && written {
        Some((hex_end_slot as isize) - (hex_ptr as isize))
    } else {
        None
    };

    Out {
        ret,
        bin: buf,
        hex_end,
        hex_end_written: written,
    }
}

/// Run one case through BOTH `.so` files and assert byte-identical results.
pub fn assert_same(case: &Case) {
    let l = libs();
    let c = run_one(l.c, case);
    let r = run_one(l.rust, case);
    if c != r {
        panic!(
            "DIVERGENCE\n case: bin_maxlen={} hex_len={} hex={:02x?} ({:?}) ignore={:02x?} hex_end_p={} bin={:?}\n    C: {:?}\n rust: {:?}",
            case.bin_maxlen,
            case.hex_len,
            case.hex,
            String::from_utf8_lossy(&case.hex),
            case.ignore,
            case.want_hex_end,
            case.bin_arg,
            c,
            r
        );
    }
}

/// Deterministic PRNG (SplitMix64) — fixed seed keeps every run reproducible.
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
    /// Uniform in `0..n` (n > 0).
    pub fn below(&mut self, n: usize) -> usize {
        (self.next_u64() % n as u64) as usize
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
}

pub const DIGITS: &[u8] = b"0123456789";
pub const UPPER: &[u8] = b"0123456789ABCDEF";
pub const LOWER: &[u8] = b"0123456789abcdef";
pub const MIXED: &[u8] = b"0123456789abcdefABCDEF";
pub const LETTERS: &[u8] = b"abcdefABCDEF";
/// Bytes one step outside each end of the accepted classes.
pub const BOUNDARY: &[u8] = b"/:@G`g";

pub fn random_from(rng: &mut Rng, alphabet: &[u8], len: usize) -> Vec<u8> {
    (0..len).map(|_| *rng.pick(alphabet)).collect()
}
