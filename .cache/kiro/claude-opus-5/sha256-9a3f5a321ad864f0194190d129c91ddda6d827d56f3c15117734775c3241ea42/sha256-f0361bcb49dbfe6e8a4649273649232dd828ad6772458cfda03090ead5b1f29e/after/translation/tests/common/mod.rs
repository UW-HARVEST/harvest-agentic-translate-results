//! Shared harness: loads BOTH shared objects (the C reference and the Rust
//! translation) via `libloading` and exposes them behind identical FFI
//! signatures. No Rust function is ever called directly — every call goes
//! through `dlsym` on the built `.so`, exactly as an external consumer would,
//! which also exercises the `#[no_mangle]` export wrappers.

#![allow(dead_code)]

use std::ffi::c_void;
use std::os::raw::{c_char, c_int};
use std::path::PathBuf;

use libloading::{Library, Symbol};

pub type ExtractFilenameFn = unsafe extern "C" fn(*const c_char, c_char) -> *const c_char;
/// Same symbol, but with the `separator` parameter typed as `int`. C `char`
/// parameters are passed in a full register and truncated by the callee, so an
/// FFI caller can legally hand over any `int`. This alias lets us drive that
/// out-of-range case.
pub type ExtractFilenameIntFn = unsafe extern "C" fn(*const c_char, c_int) -> *const c_char;
pub type CreateFilenameFn =
    unsafe extern "C" fn(*const c_char, *const c_char, usize) -> *mut c_char;

unsafe extern "C" {
    pub fn free(p: *mut c_void);
}

pub struct Lib {
    pub name: &'static str,
    _lib: Library,
    pub extract_filename: ExtractFilenameFn,
    pub extract_filename_int: ExtractFilenameIntFn,
    pub create_filename: CreateFilenameFn,
}

impl Lib {
    fn open(name: &'static str, path: &PathBuf) -> Lib {
        let lib = unsafe { Library::new(path) }
            .unwrap_or_else(|e| panic!("failed to dlopen {} ({}): {e}", path.display(), name));
        unsafe {
            let ef: Symbol<ExtractFilenameFn> = lib
                .get(b"extractFilename\0")
                .unwrap_or_else(|e| panic!("{name}: missing symbol extractFilename: {e}"));
            let efi: Symbol<ExtractFilenameIntFn> = lib.get(b"extractFilename\0").unwrap();
            let cf: Symbol<CreateFilenameFn> = lib
                .get(b"FIO_createFilename_fromOutDir\0")
                .unwrap_or_else(|e| {
                    panic!("{name}: missing symbol FIO_createFilename_fromOutDir: {e}")
                });
            let extract_filename = *ef;
            let extract_filename_int = *efi;
            let create_filename = *cf;
            Lib {
                name,
                _lib: lib,
                extract_filename,
                extract_filename_int,
                create_filename,
            }
        }
    }
}

fn workspace_root() -> PathBuf {
    // CARGO_MANIFEST_DIR == <root>/translation
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

pub fn c_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("C_SO") {
        return PathBuf::from(p);
    }
    let root = workspace_root().parent().unwrap().to_path_buf();
    root.join("c_src/build/libdriver.so")
}

pub fn rust_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("RUST_SO") {
        return PathBuf::from(p);
    }
    let base = workspace_root();
    for profile in ["release", "debug"] {
        let p = base.join("target").join(profile).join("libdriver.so");
        if p.exists() {
            return p;
        }
    }
    panic!("no Rust libdriver.so found; run `cargo build --release` first");
}

pub struct Pair {
    pub c: Lib,
    pub rs: Lib,
}

/// Both libraries, loaded once per test binary.
pub fn pair() -> &'static Pair {
    use std::sync::OnceLock;
    static P: OnceLock<Pair> = OnceLock::new();
    P.get_or_init(|| {
        let c = c_so_path();
        let r = rust_so_path();
        assert!(c.exists(), "C .so not built: {}", c.display());
        assert!(r.exists(), "Rust .so not built: {}", r.display());
        eprintln!("[harness] C   = {}", c.display());
        eprintln!("[harness] Rust= {}", r.display());
        Pair {
            c: Lib::open("C", &c),
            rs: Lib::open("Rust", &r),
        }
    })
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (xorshift64*), fixed seed => reproducible property tests.
// ---------------------------------------------------------------------------

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng(if seed == 0 { 0x9E3779B97F4A7C15 } else { seed })
    }
    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545F4914F6CDD1D)
    }
    /// Uniform-ish in `0..n` (n > 0).
    pub fn below(&mut self, n: usize) -> usize {
        (self.next_u64() % (n as u64)) as usize
    }
    pub fn range(&mut self, lo: usize, hi_inclusive: usize) -> usize {
        lo + self.below(hi_inclusive - lo + 1)
    }
    pub fn byte(&mut self) -> u8 {
        (self.next_u64() >> 33) as u8
    }
    /// A random non-NUL byte (NUL would terminate the C string).
    pub fn nonnul_byte(&mut self) -> u8 {
        let b = self.byte();
        if b == 0 { 1 } else { b }
    }
    pub fn bool(&mut self) -> bool {
        self.next_u64() & 1 == 1
    }
}

/// Build a NUL-terminated byte buffer from `bytes` (which must not contain NUL).
pub fn cstr(bytes: &[u8]) -> Vec<u8> {
    assert!(!bytes.contains(&0), "input must not contain interior NUL");
    let mut v = bytes.to_vec();
    v.push(0);
    v
}

/// Random string of `len` bytes drawn from `alphabet`.
pub fn rand_from(rng: &mut Rng, len: usize, alphabet: &[u8]) -> Vec<u8> {
    (0..len).map(|_| alphabet[rng.below(alphabet.len())]).collect()
}

/// Random string of `len` arbitrary non-NUL bytes.
pub fn rand_raw(rng: &mut Rng, len: usize) -> Vec<u8> {
    (0..len).map(|_| rng.nonnul_byte()).collect()
}

// ---------------------------------------------------------------------------
// Differential assertions
// ---------------------------------------------------------------------------

fn render(b: &[u8]) -> String {
    b.iter()
        .map(|&c| {
            if (0x20..0x7f).contains(&c) {
                (c as char).to_string()
            } else {
                format!("\\x{c:02x}")
            }
        })
        .collect()
}

/// Call `extractFilename` on both libraries with the *same* input buffer and
/// assert they return the identical pointer, expressed as an offset from the
/// start of `path` (the C contract is "the same pointer, or a pointer into the
/// same string", so offsets are the only meaningful comparison).
pub fn diff_extract(path_buf: &[u8], separator: u8) -> isize {
    let p = pair();
    let base = path_buf.as_ptr() as *const c_char;
    let (oc, or) = unsafe {
        let rc = (p.c.extract_filename)(base, separator as i8 as c_char);
        let rr = (p.rs.extract_filename)(base, separator as i8 as c_char);
        (
            (rc as isize) - (base as isize),
            (rr as isize) - (base as isize),
        )
    };
    assert_eq!(
        oc,
        or,
        "extractFilename divergence: path={:?} separator=0x{:02x}: C offset {} vs Rust offset {}",
        render(path_buf),
        separator,
        oc,
        or
    );
    oc
}

/// Same, but with `separator` supplied as a raw `int` (possibly outside the
/// `char` range).
pub fn diff_extract_int(path_buf: &[u8], separator: c_int) -> isize {
    let p = pair();
    let base = path_buf.as_ptr() as *const c_char;
    let (oc, or) = unsafe {
        let rc = (p.c.extract_filename_int)(base, separator);
        let rr = (p.rs.extract_filename_int)(base, separator);
        (
            (rc as isize) - (base as isize),
            (rr as isize) - (base as isize),
        )
    };
    assert_eq!(
        oc, or,
        "extractFilename(int) divergence: path={:?} separator={} (0x{:x}): C {} vs Rust {}",
        render(path_buf), separator, separator, oc, or
    );
    oc
}

/// Number of bytes `FIO_createFilename_fromOutDir` allocates, replicating the C
/// `size_t` arithmetic (wrapping) exactly.
pub fn expected_alloc_len(out_dir_len: usize, filename_len: usize, suffix_len: usize) -> usize {
    out_dir_len
        .wrapping_add(1)
        .wrapping_add(filename_len)
        .wrapping_add(suffix_len)
        .wrapping_add(1)
}

fn c_strlen(p: *const c_char) -> usize {
    let mut n = 0usize;
    unsafe {
        while *p.add(n) != 0 {
            n += 1;
        }
    }
    n
}

/// Drive `FIO_createFilename_fromOutDir` on both libraries and compare the FULL
/// returned allocation byte-for-byte (not just up to the NUL — the `calloc`
/// zero-fill of the `suffixLen` tail is part of the observable output).
///
/// `out_dir` and `path` must be NUL-terminated buffers. Returns the compared
/// bytes.
pub fn diff_create(path_buf: &[u8], out_dir_buf: &[u8], suffix_len: usize) -> Vec<u8> {
    diff_create_ptr(
        path_buf.as_ptr() as *const c_char,
        out_dir_buf.as_ptr() as *const c_char,
        suffix_len,
        &render(path_buf),
        &render(out_dir_buf),
    )
}

pub fn diff_create_ptr(
    path: *const c_char,
    out_dir: *const c_char,
    suffix_len: usize,
    path_dbg: &str,
    out_dir_dbg: &str,
) -> Vec<u8> {
    let p = pair();

    // Compute the allocation size the C code will request, so we know how many
    // bytes are legitimately readable in the result.
    let out_dir_len = c_strlen(out_dir);
    // filenameStart is derived with the platform separator '/'.
    let filename_start = unsafe { (p.c.extract_filename)(path, b'/' as i8 as c_char) };
    let filename_len = c_strlen(filename_start);
    let n = expected_alloc_len(out_dir_len, filename_len, suffix_len);

    unsafe {
        let rc = (p.c.create_filename)(path, out_dir, suffix_len);
        let rr = (p.rs.create_filename)(path, out_dir, suffix_len);
        assert!(!rc.is_null(), "C returned NULL (it never should)");
        assert!(!rr.is_null(), "Rust returned NULL (it never should)");

        let bc = std::slice::from_raw_parts(rc as *const u8, n).to_vec();
        let br = std::slice::from_raw_parts(rr as *const u8, n).to_vec();
        free(rc as *mut c_void);
        free(rr as *mut c_void);

        assert_eq!(
            bc,
            br,
            "FIO_createFilename_fromOutDir divergence\n  path      = {path_dbg}\n  outDirName= {out_dir_dbg}\n  suffixLen = {suffix_len}\n  alloc     = {n}\n  C   = {}\n  Rust= {}",
            render(&bc),
            render(&br)
        );
        bc
    }
}

/// `rand_from` with the length drawn from the same rng (avoids two-phase borrow
/// problems at the call sites).
pub fn rand_from_range(rng: &mut Rng, lo: usize, hi: usize, alphabet: &[u8]) -> Vec<u8> {
    let len = rng.range(lo, hi);
    rand_from(rng, len, alphabet)
}

/// `rand_raw` with the length drawn from the same rng.
pub fn rand_raw_range(rng: &mut Rng, lo: usize, hi: usize) -> Vec<u8> {
    let len = rng.range(lo, hi);
    rand_raw(rng, len)
}

/// Open exactly ONE of the two libraries (used by the subprocess helpers, where
/// loading the other side would pollute the compared stderr/allocator state).
pub fn open_one(which: &str) -> Lib {
    match which {
        "c" => Lib::open("C", &c_so_path()),
        "rust" => Lib::open("Rust", &rust_so_path()),
        other => panic!("unknown library selector {other:?}"),
    }
}
