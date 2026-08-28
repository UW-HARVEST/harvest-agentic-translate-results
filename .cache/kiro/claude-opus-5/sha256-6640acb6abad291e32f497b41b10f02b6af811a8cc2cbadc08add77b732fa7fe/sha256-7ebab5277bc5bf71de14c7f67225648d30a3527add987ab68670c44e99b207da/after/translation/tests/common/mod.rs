//! Shared harness: loads the C and Rust shared objects via `libloading` and
//! drives `hex2bin` through the FFI boundary on both sides.

use libloading::{Library, Symbol};
use std::ffi::c_char;
use std::ffi::c_int;
use std::path::PathBuf;

pub type Hex2BinFn = unsafe extern "C" fn(
    *mut u8,
    usize,
    *const c_char,
    usize,
    *const c_char,
    *mut *const c_char,
) -> c_int;

fn workspace_root() -> PathBuf {
    // CARGO_MANIFEST_DIR == <root>/translation
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation/ has a parent")
        .to_path_buf()
}

fn find_c_so() -> PathBuf {
    let build_dir = workspace_root().join("c_src").join("build");
    let mut candidates: Vec<PathBuf> = Vec::new();
    if let Ok(entries) = std::fs::read_dir(&build_dir) {
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
            "no C .so found in {}; build it with: \
             cd c_src && mkdir -p build && cd build && \
             cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
            build_dir.display()
        )
    })
}

/// Locate the Rust cdylib. Prefer the profile the test binary itself lives in
/// so `cargo test` and `cargo test --release` both work.
fn find_rust_so() -> PathBuf {
    let name = "libhex2bin_lib.so";
    let mut dirs: Vec<PathBuf> = Vec::new();

    // .../target/<profile>/deps/<test-bin>
    if let Ok(exe) = std::env::current_exe() {
        if let Some(deps) = exe.parent() {
            dirs.push(deps.to_path_buf());
            if let Some(profile) = deps.parent() {
                dirs.push(profile.to_path_buf());
            }
        }
    }
    let target = workspace_root().join("translation").join("target");
    dirs.push(target.join("debug"));
    dirs.push(target.join("release"));

    for d in &dirs {
        let p = d.join(name);
        if p.is_file() {
            return p;
        }
    }
    panic!(
        "could not find {name}; searched: {:?}. Run `cargo build` first.",
        dirs
    );
}

pub struct Impls {
    _c_lib: Library,
    _rust_lib: Library,
    pub c: Hex2BinFn,
    pub rust: Hex2BinFn,
}

impl Impls {
    pub fn load() -> Impls {
        unsafe {
            let c_lib = Library::new(find_c_so()).expect("load C .so");
            let rust_lib = Library::new(find_rust_so()).expect("load Rust .so");
            let c: Symbol<Hex2BinFn> = c_lib.get(b"hex2bin\0").expect("C hex2bin symbol");
            let rust: Symbol<Hex2BinFn> =
                rust_lib.get(b"hex2bin\0").expect("Rust hex2bin symbol");
            let c = *c;
            let rust = *rust;
            Impls {
                _c_lib: c_lib,
                _rust_lib: rust_lib,
                c,
                rust,
            }
        }
    }
}

/// Everything observable about one `hex2bin` call.
#[derive(Debug, PartialEq, Eq)]
pub struct Outcome {
    pub ret: c_int,
    /// Full output buffer (canary-filled before the call) so stray writes show up.
    pub bin: Vec<u8>,
    /// `Some(offset)` of `*hex_end_p` relative to `hex`, when requested.
    pub hex_end: Option<isize>,
}

const CANARY: u8 = 0xA5;
/// Extra slack past `bin_maxlen` to detect out-of-bounds writes.
const SLACK: usize = 8;

/// Invoke one implementation. `hex` is passed as raw bytes (may contain NULs).
/// `ignore` is passed as a NUL-terminated byte slice, or `None` for a NULL ptr.
pub fn call(
    f: Hex2BinFn,
    bin_maxlen: usize,
    hex: &[u8],
    hex_len: usize,
    ignore: Option<&[u8]>,
    want_hex_end: bool,
) -> Outcome {
    let mut bin = vec![CANARY; bin_maxlen + SLACK];
    // Keep the hex bytes in their own allocation; the callee only reads them.
    let hex_buf: Vec<u8> = hex.to_vec();

    let ignore_buf: Option<Vec<u8>> = ignore.map(|s| {
        let mut v = s.to_vec();
        if v.last() != Some(&0) {
            v.push(0);
        }
        v
    });
    let ignore_ptr: *const c_char = match &ignore_buf {
        Some(v) => v.as_ptr() as *const c_char,
        None => std::ptr::null(),
    };

    let hex_ptr = hex_buf.as_ptr() as *const c_char;
    let mut hex_end: *const c_char = std::ptr::null();
    let hex_end_ptr: *mut *const c_char = if want_hex_end {
        &mut hex_end
    } else {
        std::ptr::null_mut()
    };

    let ret = unsafe {
        f(
            bin.as_mut_ptr(),
            bin_maxlen,
            hex_ptr,
            hex_len,
            ignore_ptr,
            hex_end_ptr,
        )
    };

    let hex_end = if want_hex_end {
        Some(unsafe { hex_end.offset_from(hex_ptr) })
    } else {
        None
    };

    drop(ignore_buf);
    drop(hex_buf);

    Outcome { ret, bin, hex_end }
}

/// Assert both implementations agree byte-for-byte for one input.
#[track_caller]
pub fn assert_same(
    impls: &Impls,
    label: &str,
    bin_maxlen: usize,
    hex: &[u8],
    hex_len: usize,
    ignore: Option<&[u8]>,
    want_hex_end: bool,
) {
    let c = call(impls.c, bin_maxlen, hex, hex_len, ignore, want_hex_end);
    let r = call(impls.rust, bin_maxlen, hex, hex_len, ignore, want_hex_end);
    assert_eq!(
        c, r,
        "mismatch [{label}] bin_maxlen={bin_maxlen} hex={hex:?} hex_len={hex_len} \
         ignore={ignore:?} want_hex_end={want_hex_end}\n  C   = {c:?}\n  rust= {r:?}"
    );
}

/// Tiny deterministic PRNG (xorshift64*) so failures are reproducible.
pub struct Rng(pub u64);

impl Rng {
    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
    pub fn below(&mut self, n: usize) -> usize {
        if n == 0 {
            0
        } else {
            (self.next_u64() % n as u64) as usize
        }
    }
    pub fn byte(&mut self) -> u8 {
        (self.next_u64() >> 33) as u8
    }
}
