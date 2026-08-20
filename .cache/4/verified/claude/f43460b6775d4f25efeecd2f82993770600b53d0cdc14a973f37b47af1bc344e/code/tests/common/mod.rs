//! Shared plumbing for the C-vs-Rust differential tests.
//!
//! Both implementations are loaded as shared objects through `libloading` and
//! called only through their exported `process_decisions` symbol, so the
//! `#[no_mangle] extern "C"` wrapper is under test as well.  No Rust function
//! is ever called directly.

#![allow(dead_code)]

use std::ffi::{c_char, c_int};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

/// The one and only symbol `c_src/src/lib.c` exports.
pub type ProcessDecisionsFn =
    unsafe extern "C" fn(*mut c_char, usize, c_int, c_int) -> c_int;

pub struct Libs {
    pub c: ProcessDecisionsFn,
    pub rust: ProcessDecisionsFn,
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// Builds `c_build/libcdecisions.so` from the untouched C sources (idempotent).
///
/// `DRIVER_C_SO` overrides the path, which is used to re-run the whole suite
/// against C builds at other optimisation levels.
pub fn c_library_path() -> PathBuf {
    if let Some(p) = std::env::var_os("DRIVER_C_SO") {
        let p = PathBuf::from(p);
        assert!(p.is_file(), "DRIVER_C_SO={} is not a file", p.display());
        return p;
    }

    let root = manifest_dir();
    let so = root.join("c_build/libcdecisions.so");
    let script = root.join("build_c_so.sh");

    let needs_build = match (so.metadata(), root.join("c_src/src/lib.c").metadata()) {
        (Ok(a), Ok(b)) => match (a.modified(), b.modified()) {
            (Ok(a), Ok(b)) => a < b,
            _ => false,
        },
        _ => true,
    };

    if needs_build {
        let out = Command::new("sh")
            .arg(&script)
            .output()
            .expect("failed to spawn build_c_so.sh");
        assert!(
            out.status.success(),
            "building the C shared library failed:\nstdout: {}\nstderr: {}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        );
    }

    assert!(so.is_file(), "missing C shared library at {}", so.display());
    so
}

/// Locates the `cdylib` cargo built for this crate.  Integration-test binaries
/// live in `<target>/<profile>/deps/`, so the library sits two levels up.
///
/// `DRIVER_RUST_SO` overrides the search, which lets the (necessarily
/// `panic = "unwind"`) test harness drive the **release** cdylib as well — the
/// crate's `[profile.release] panic = "abort"` makes `cargo test --release`
/// impossible, so the release configuration is verified this way instead.
pub fn rust_library_path() -> PathBuf {
    if let Some(p) = std::env::var_os("DRIVER_RUST_SO") {
        let p = PathBuf::from(p);
        assert!(p.is_file(), "DRIVER_RUST_SO={} is not a file", p.display());
        return p;
    }

    let exe = std::env::current_exe().expect("current_exe");
    let mut dir: Option<&Path> = exe.parent();

    while let Some(d) = dir {
        let candidate = d.join("libdriver.so");
        if candidate.is_file() {
            return candidate;
        }
        dir = d.parent();
    }

    panic!(
        "could not find libdriver.so near {} — run `cargo build` first so the \
         cdylib target exists",
        exe.display()
    );
}

/// Loads both shared objects exactly once per test binary.
pub fn libs() -> &'static Libs {
    static LIBS: OnceLock<Libs> = OnceLock::new();

    LIBS.get_or_init(|| {
        let load = |path: PathBuf| -> ProcessDecisionsFn {
            // Leaked on purpose: the resolved function pointers have to stay
            // valid for the whole process lifetime.
            let lib: &'static libloading::Library = Box::leak(Box::new(unsafe {
                libloading::Library::new(&path)
                    .unwrap_or_else(|e| panic!("dlopen({}) failed: {e}", path.display()))
            }));
            let sym: libloading::Symbol<'static, ProcessDecisionsFn> = unsafe {
                lib.get(b"process_decisions\0").unwrap_or_else(|e| {
                    panic!("process_decisions missing from {}: {e}", path.display())
                })
            };
            *sym
        };

        Libs {
            c: load(c_library_path()),
            rust: load(rust_library_path()),
        }
    })
}

/// Sentinel bytes appended after the region the C code may touch, so that an
/// out-of-bounds write by either implementation is caught by the comparison.
const GUARD: [u8; 16] = [0xAA; 16];

pub struct Outcome {
    pub ret: c_int,
    /// The buffer contents (including the guard bytes) after the call.
    pub buffer: Vec<u8>,
}

/// Calls `process_decisions` in `lib` on a fresh copy of `bytes`.
fn call_one(f: ProcessDecisionsFn, bytes: &[u8], length: usize, op: c_int, param: c_int) -> Outcome {
    let mut buf = Vec::with_capacity(bytes.len() + GUARD.len());
    buf.extend_from_slice(bytes);
    buf.extend_from_slice(&GUARD);

    let ret = unsafe { f(buf.as_mut_ptr() as *mut c_char, length, op, param) };

    Outcome { ret, buffer: buf }
}

/// Runs the same call against C and Rust and asserts the return value *and* the
/// post-call buffer bytes are identical.  Returns the (shared) result.
#[track_caller]
pub fn assert_same(bytes: &[u8], length: usize, op: c_int, param: c_int, ctx: &str) -> c_int {
    assert!(
        length <= bytes.len(),
        "test bug ({ctx}): length {length} exceeds the {} byte buffer, which \
         would make the C code read out of bounds",
        bytes.len()
    );

    let l = libs();
    let c = call_one(l.c, bytes, length, op, param);
    let r = call_one(l.rust, bytes, length, op, param);

    assert_eq!(
        c.ret,
        r.ret,
        "return value mismatch [{ctx}]: op={op} param={param} length={length} \
         input={:?} ({:02x?}) -> C={} Rust={}",
        String::from_utf8_lossy(bytes),
        bytes,
        c.ret,
        r.ret
    );

    assert_eq!(
        c.buffer,
        r.buffer,
        "post-call buffer mismatch [{ctx}]: op={op} param={param} length={length} \
         input={:02x?}\n  C   -> {:02x?}\n  Rust-> {:02x?}",
        bytes,
        c.buffer,
        r.buffer
    );

    assert_eq!(
        &c.buffer[bytes.len()..],
        &GUARD[..],
        "guard bytes clobbered by the C implementation [{ctx}]: op={op} length={length}"
    );

    c.ret
}

/// Like [`assert_same`] but WITHOUT the `length <= bytes.len()` guard.
///
/// This is for the legitimate case where the C code provably touches fewer bytes
/// than `length` claims — e.g. operation 0/1 only ever read indices 0..2, so a
/// caller may pass a gigantic `length` with a 3-byte buffer and the C behaves
/// perfectly well.  A naive Rust translation that did
/// `slice::from_raw_parts(ptr, length)` up front would break here.
///
/// # Safety
///
/// The caller must guarantee `bytes` covers every byte the C code will touch for
/// this `operation`.
#[track_caller]
pub unsafe fn assert_same_overclaimed_length(
    bytes: &[u8],
    length: usize,
    op: c_int,
    param: c_int,
    ctx: &str,
) -> c_int {
    let l = libs();
    let c = call_one(l.c, bytes, length, op, param);
    let r = call_one(l.rust, bytes, length, op, param);

    assert_eq!(
        c.ret, r.ret,
        "return value mismatch [{ctx}]: op={op} param={param} length={length} \
         buffer={bytes:02x?} -> C={} Rust={}",
        c.ret, r.ret
    );
    assert_eq!(
        c.buffer, r.buffer,
        "post-call buffer mismatch [{ctx}]: op={op} param={param} length={length}\n  \
         C   -> {:02x?}\n  Rust-> {:02x?}",
        c.buffer, r.buffer
    );
    assert_eq!(
        &c.buffer[bytes.len()..],
        &GUARD[..],
        "guard bytes clobbered [{ctx}]: op={op} length={length}"
    );
    c.ret
}

/// Same as [`assert_same`] but with `length == bytes.len()`.
#[track_caller]
pub fn assert_same_full(bytes: &[u8], op: c_int, param: c_int, ctx: &str) -> c_int {
    assert_same(bytes, bytes.len(), op, param, ctx)
}

/// Raw pointer variant, used for the NULL-pointer error rows.
#[track_caller]
pub fn assert_same_null(length: usize, op: c_int, param: c_int, ctx: &str) -> c_int {
    let l = libs();
    let cres = unsafe { (l.c)(std::ptr::null_mut(), length, op, param) };
    let rres = unsafe { (l.rust)(std::ptr::null_mut(), length, op, param) };
    assert_eq!(
        cres, rres,
        "NULL-pointer mismatch [{ctx}]: op={op} param={param} length={length} \
         -> C={cres} Rust={rres}"
    );
    cres
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (splitmix64) — property-style testing with a fixed seed.
// ---------------------------------------------------------------------------

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
        (self.next_u64() % n as u64) as usize
    }

    /// Uniform in `[lo, hi]`.
    pub fn range(&mut self, lo: i64, hi: i64) -> i64 {
        lo + (self.next_u64() % ((hi - lo + 1) as u64)) as i64
    }

    pub fn byte(&mut self) -> u8 {
        (self.next_u64() >> 24) as u8
    }

    pub fn bool(&mut self) -> bool {
        self.next_u64() & 1 == 1
    }

    /// A byte drawn from the four characters `parse_bool` treats specially.
    pub fn yn_byte(&mut self) -> u8 {
        b"yYnN"[self.below(4)]
    }
}

/// All the bytes `parse_bool` gives a dedicated branch to.
pub const YN_BYTES: [u8; 4] = [b'y', b'Y', b'n', b'N'];

/// Interesting `int` values to push across the FFI boundary for
/// `operation` / `param`, including out-of-range "enum" values.
pub const INT_EDGE_VALUES: [c_int; 15] = [
    c_int::MIN,
    c_int::MIN + 1,
    -1000,
    -4,
    -3,
    -2,
    -1,
    0,
    1,
    2,
    3,
    4,
    5,
    1000,
    c_int::MAX,
];

/// Renders a bit pattern as a `y`/`n` decision string of the given length.
pub fn pattern_to_bytes(bits: u32, len: usize) -> Vec<u8> {
    (0..len)
        .map(|i| if (bits >> i) & 1 == 1 { b'y' } else { b'n' })
        .collect()
}
