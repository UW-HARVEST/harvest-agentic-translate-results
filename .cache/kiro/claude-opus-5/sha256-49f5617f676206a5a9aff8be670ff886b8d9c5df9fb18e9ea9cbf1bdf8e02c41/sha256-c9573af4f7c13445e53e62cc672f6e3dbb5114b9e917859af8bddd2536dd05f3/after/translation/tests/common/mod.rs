//! Shared differential-test harness.
//!
//! Loads BOTH shared objects through `libloading` and calls `hex2bin` only via
//! its exported symbol — never through the Rust crate directly — so the
//! `#[no_mangle] extern "C"` wrapper is part of what is under test.

#![allow(dead_code)]

use std::ffi::c_char;
use std::ffi::c_int;
use std::path::PathBuf;
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

/// Outcome of one `hex2bin` invocation, in a form that is comparable between
/// the two libraries (the raw `hex_end_p` pointer is normalised to an offset
/// relative to the `hex` base pointer, using wrapping arithmetic so the C
/// `hex_pos--` underflow case would still compare equal).
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Outcome {
    pub ret: c_int,
    /// Full `bin` buffer after the call, including bytes the callee should not
    /// have touched (they are pre-filled with a sentinel pattern).
    pub bin: Vec<u8>,
    /// `None` when `hex_end_p` was passed as NULL.
    pub hex_end_off: Option<usize>,
}

pub struct Libs {
    _c_lib: Library,
    _rs_lib: Library,
    pub c: Hex2BinFn,
    pub rs: Hex2BinFn,
}

fn workspace_root() -> PathBuf {
    // CARGO_MANIFEST_DIR = <root>/translation
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.pop();
    p
}

pub fn find_c_so() -> PathBuf {
    let build = workspace_root().join("c_src").join("build");
    let mut candidates: Vec<PathBuf> = Vec::new();
    if let Ok(entries) = std::fs::read_dir(&build) {
        for e in entries.flatten() {
            let p = e.path();
            if p.extension().and_then(|s| s.to_str()) == Some("so") {
                candidates.push(p);
            }
        }
    }
    candidates.sort();
    candidates.into_iter().next().unwrap_or_else(|| {
        panic!(
            "no C .so found in {}; build it with:\n  cd c_src && mkdir -p build && cd build && \
             cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
            build.display()
        )
    })
}

/// Locate the Rust cdylib to load.
///
/// IMPORTANT: `cargo test` does **not** build a `crate-type = ["cdylib"]`
/// artifact, so a naive lookup can silently load a *stale* `.so` and report a
/// vacuous pass. Therefore:
///
/// * `HEX2BIN_RUST_SO` (set by `scripts/run_all.sh`) wins if present;
/// * otherwise the newest `libhex2bin_lib.so` under `target/` is used, and its
///   mtime is checked against `src/lib.rs` / `Cargo.toml`. A stale artifact is a
///   hard failure, never a silent pass.
pub fn find_rust_so() -> PathBuf {
    let name = "libhex2bin_lib.so";

    if let Ok(p) = std::env::var("HEX2BIN_RUST_SO") {
        let p = PathBuf::from(p);
        assert!(p.is_file(), "HEX2BIN_RUST_SO points at a missing file: {}", p.display());
        return p;
    }

    let exe = std::env::current_exe().expect("current_exe");
    let profile_dir = exe
        .parent()
        .and_then(|p| p.parent())
        .expect("target/<profile>")
        .to_path_buf();
    let target = profile_dir.parent().expect("target dir").to_path_buf();

    let mut found: Vec<PathBuf> = Vec::new();
    for cand in [
        profile_dir.join(name),
        target.join("debug").join(name),
        target.join("release").join(name),
    ] {
        if cand.is_file() && !found.contains(&cand) {
            found.push(cand);
        }
    }
    assert!(
        !found.is_empty(),
        "no {name} anywhere under {} — the cdylib is not built.\n\
         `cargo test` alone does NOT build a cdylib; run `scripts/run_all.sh` \
         (or `cargo build --release` first).",
        target.display()
    );

    let mtime = |p: &PathBuf| {
        std::fs::metadata(p)
            .and_then(|m| m.modified())
            .unwrap_or(std::time::SystemTime::UNIX_EPOCH)
    };
    found.sort_by_key(mtime);
    let newest = found.pop().unwrap();

    // Freshness gate.
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let so_time = mtime(&newest);
    for src in ["src/lib.rs", "Cargo.toml"] {
        let sp = manifest.join(src);
        if sp.is_file() && mtime(&sp) > so_time {
            panic!(
                "STALE ARTIFACT: {} is older than {}.\n\
                 `cargo test` does not rebuild a cdylib, so this run would test \
                 outdated machine code.\nRun `scripts/run_all.sh` (it rebuilds \
                 the cdylib and exports HEX2BIN_RUST_SO) instead.",
                newest.display(),
                sp.display()
            );
        }
    }
    newest
}

static LIBS: OnceLock<Libs> = OnceLock::new();

pub fn libs() -> &'static Libs {
    LIBS.get_or_init(|| {
        let c_path = find_c_so();
        let rs_path = find_rust_so();
        unsafe {
            let c_lib = Library::new(&c_path)
                .unwrap_or_else(|e| panic!("dlopen {}: {e}", c_path.display()));
            let rs_lib = Library::new(&rs_path)
                .unwrap_or_else(|e| panic!("dlopen {}: {e}", rs_path.display()));
            let c_sym: Symbol<Hex2BinFn> = c_lib
                .get(b"hex2bin\0")
                .unwrap_or_else(|e| panic!("hex2bin missing from C .so: {e}"));
            let rs_sym: Symbol<Hex2BinFn> = rs_lib
                .get(b"hex2bin\0")
                .unwrap_or_else(|e| panic!("hex2bin missing from Rust .so: {e}"));
            let c = *c_sym;
            let rs = *rs_sym;
            Libs {
                _c_lib: c_lib,
                _rs_lib: rs_lib,
                c,
                rs,
            }
        }
    })
}

/// Sentinel used to pre-fill the `bin` buffer so that stray writes past the
/// reported length are caught.
pub const SENTINEL: u8 = 0xA5;

/// How `bin` should be passed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinArg {
    /// Allocate a buffer of this many bytes (pre-filled with `SENTINEL`).
    Buf(usize),
    /// Pass a NULL `bin` pointer.
    Null,
}

/// How `hex` should be passed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HexArg<'a> {
    Bytes(&'a [u8]),
    /// Pass a NULL `hex` pointer.
    Null,
}

/// One fully specified call.
#[derive(Debug, Clone)]
pub struct Call<'a> {
    pub bin: BinArg,
    pub bin_maxlen: usize,
    pub hex: HexArg<'a>,
    pub hex_len: usize,
    /// `None` => NULL `ignore`. Contents must not contain an interior NUL.
    pub ignore: Option<&'a [u8]>,
    pub want_hex_end: bool,
}

fn run_one(f: Hex2BinFn, call: &Call<'_>) -> Outcome {
    let mut bin_buf: Vec<u8> = match call.bin {
        BinArg::Buf(n) => vec![SENTINEL; n],
        BinArg::Null => Vec::new(),
    };
    let bin_ptr: *mut u8 = match call.bin {
        BinArg::Buf(_) => bin_buf.as_mut_ptr(),
        BinArg::Null => std::ptr::null_mut(),
    };

    // Keep the hex buffer alive for the duration of the call.
    let hex_owned: Vec<u8>;
    let hex_ptr: *const c_char = match call.hex {
        HexArg::Bytes(b) => {
            hex_owned = b.to_vec();
            hex_owned.as_ptr() as *const c_char
        }
        HexArg::Null => {
            hex_owned = Vec::new();
            std::ptr::null()
        }
    };

    let ignore_owned: Vec<u8>;
    let ignore_ptr: *const c_char = match call.ignore {
        Some(s) => {
            let mut v = s.to_vec();
            v.push(0);
            ignore_owned = v;
            ignore_owned.as_ptr() as *const c_char
        }
        None => {
            ignore_owned = Vec::new();
            std::ptr::null()
        }
    };

    let mut hex_end: *const c_char = std::ptr::null();
    let hex_end_ptr: *mut *const c_char = if call.want_hex_end {
        &mut hex_end
    } else {
        std::ptr::null_mut()
    };

    let ret = unsafe {
        f(
            bin_ptr,
            call.bin_maxlen,
            hex_ptr,
            call.hex_len,
            ignore_ptr,
            hex_end_ptr,
        )
    };

    let hex_end_off = if call.want_hex_end {
        Some((hex_end as usize).wrapping_sub(hex_ptr as usize))
    } else {
        None
    };

    // Silence "unused" for the keep-alive buffers.
    let _ = (&hex_owned, &ignore_owned);

    Outcome {
        ret,
        bin: bin_buf,
        hex_end_off,
    }
}

// Per-thread count of C-vs-Rust comparisons performed. The libtest harness
// runs each `#[test]` on its own thread, so a thread-local counter attributes
// work to the right test even when tests run in parallel. Used to assert that
// loops actually executed — a loop that silently iterates zero times would
// otherwise "pass".
thread_local! {
    static COMPARISONS: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };
}

pub fn comparisons() -> u64 {
    COMPARISONS.with(|c| c.get())
}

/// Run the same call against both libraries and assert byte-identical results.
#[track_caller]
pub fn assert_same(label: &str, call: &Call<'_>) {
    COMPARISONS.with(|c| c.set(c.get() + 1));
    let l = libs();
    let c = run_one(l.c, call);
    let r = run_one(l.rs, call);
    if c != r {
        panic!(
            "DIVERGENCE [{label}]\n  call        = {call:?}\n  hex bytes   = {:?}\n  ignore      = {:?}\n\
             \n  C   ret={} hex_end_off={:?}\n  C   bin={:02x?}\n  RS  ret={} hex_end_off={:?}\n  RS  bin={:02x?}",
            match call.hex { HexArg::Bytes(b) => format!("{b:02x?}"), HexArg::Null => "NULL".into() },
            call.ignore.map(|s| format!("{s:02x?}")).unwrap_or_else(|| "NULL".into()),
            c.ret, c.hex_end_off, c.bin,
            r.ret, r.hex_end_off, r.bin,
        );
    }
}

/// Convenience: run a call in all four (`ignore` NULL-ness × `hex_end_p`
/// NULL-ness) modes is *not* automatic — `ignore` semantics are part of the
/// configuration, so only `hex_end_p` is swept here.
#[track_caller]
pub fn assert_same_both_end_modes(label: &str, call: &Call<'_>) {
    let mut a = call.clone();
    a.want_hex_end = true;
    assert_same(&format!("{label}/hex_end=set"), &a);
    let mut b = call.clone();
    b.want_hex_end = false;
    assert_same(&format!("{label}/hex_end=NULL"), &b);
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (xorshift64*) — fixed seed for reproducibility.
// ---------------------------------------------------------------------------

pub struct Rng(u64);

pub const SEED: u64 = 0x5EED_1234_ABCD_0001;

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
    pub fn next_u8(&mut self) -> u8 {
        (self.next_u64() >> 33) as u8
    }
    /// Uniform-ish value in `0..n` (n > 0).
    pub fn below(&mut self, n: usize) -> usize {
        (self.next_u64() % (n as u64)) as usize
    }
    /// Value in `lo..=hi`.
    pub fn range(&mut self, lo: usize, hi: usize) -> usize {
        debug_assert!(lo <= hi);
        lo + self.below(hi - lo + 1)
    }
    pub fn bool(&mut self) -> bool {
        self.next_u64() & 1 == 1
    }
    pub fn pick<'a, T>(&mut self, xs: &'a [T]) -> &'a T {
        &xs[self.below(xs.len())]
    }
}

// ---------------------------------------------------------------------------
// Character-class helpers
// ---------------------------------------------------------------------------

pub const LOWER: &[u8] = b"0123456789abcdef";
pub const UPPER: &[u8] = b"0123456789ABCDEF";
pub const DEC: &[u8] = b"0123456789";
pub const ALPHA_LOWER: &[u8] = b"abcdef";
pub const ALPHA_UPPER: &[u8] = b"ABCDEF";
pub const MIXED: &[u8] = b"0123456789abcdefABCDEF";

/// Bytes immediately adjacent to the three hex-digit ranges plus assorted junk.
pub const ADJACENT: &[u8] = &[
    b'/', b':', b'@', b'G', b'`', b'g', b'.', b' ', b'\t', b'\n', b'z', b'Z', b'~', 0x7f,
];

pub fn is_hex_digit(b: u8) -> bool {
    b.is_ascii_digit() || (b'a'..=b'f').contains(&b) || (b'A'..=b'F').contains(&b)
}

/// Random hex string of `n` nibbles drawn from `alphabet`.
pub fn rand_hex(rng: &mut Rng, n: usize, alphabet: &[u8]) -> Vec<u8> {
    (0..n).map(|_| *rng.pick(alphabet)).collect()
}

/// Assert that a block performed at least `min` C-vs-Rust comparisons.
/// Guards against loops that silently iterate zero times.
#[track_caller]
pub fn assert_did_work(label: &str, before: u64, min: u64) {
    let done = comparisons() - before;
    assert!(
        done >= min,
        "[{label}] only {done} C-vs-Rust comparisons were performed, expected >= {min} \
         — the test loop did not run"
    );
    eprintln!("[{label}] {done} C-vs-Rust comparisons");
}
