// Shared harness for the differential tests.
//
// Both the C shared object and the Rust shared object are loaded with
// `libloading` (RTLD_NOW | RTLD_LOCAL, so the two copies of the identically
// named symbols cannot interfere with each other) and every call goes through
// the dynamic symbol table — the Rust crate is never linked directly, so the
// `#[no_mangle] extern "C"` wrappers are exercised exactly as an external C
// consumer would exercise them.
#![allow(dead_code)]

use std::ffi::{c_char, c_int, c_void, OsStr};
use std::os::unix::io::AsRawFd;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};

pub type OpFn = unsafe extern "C" fn(c_int) -> c_int;
pub type OpFnOpt = Option<OpFn>;

type FnCharInBuf = unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int;
type FnIntToInt = unsafe extern "C" fn(c_int) -> c_int;
type FnStrToInt = unsafe extern "C" fn(*const c_char) -> c_int;
type FnFind = unsafe extern "C" fn(*const c_char, usize, c_char) -> *mut c_char;
type FnCreate = unsafe extern "C" fn(*const c_char) -> *mut c_char;
type FnApply = unsafe extern "C" fn(OpFnOpt, c_int) -> c_int;

extern "C" {
    fn dup(oldfd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
    fn free(p: *mut c_void);
}

/// All ten exported entry points of one implementation.
pub struct Api {
    pub name: &'static str,
    pub path: PathBuf,
    pub charinbuf: FnCharInBuf,
    pub increment_counter: FnIntToInt,
    pub decrement_counter: FnIntToInt,
    pub multiply_counter: FnIntToInt,
    pub reset_counter: FnIntToInt,
    pub is_string_empty: FnStrToInt,
    pub find_char_in_buffer: FnFind,
    pub create_buffer: FnCreate,
    pub validate_uint16_range: FnIntToInt,
    pub apply_operation: FnApply,
    // Keep the library alive for the whole process lifetime.
    _lib: &'static libloading::os::unix::Library,
}

impl Api {
    fn open(name: &'static str, path: &Path) -> Api {
        // RTLD_NOW so that any missing symbol shows up immediately, RTLD_LOCAL
        // (the default) so that the two libraries keep separate symbol scopes.
        let lib = unsafe {
            libloading::os::unix::Library::open(
                Some(OsStr::new(path)),
                libloading::os::unix::RTLD_NOW | libloading::os::unix::RTLD_LOCAL,
            )
        }
        .unwrap_or_else(|e| panic!("failed to dlopen {} ({}): {e}", name, path.display()));
        let lib: &'static libloading::os::unix::Library = Box::leak(Box::new(lib));

        macro_rules! sym {
            ($t:ty, $n:expr) => {{
                let s: libloading::os::unix::Symbol<$t> = unsafe { lib.get($n) }
                    .unwrap_or_else(|e| panic!("{}: missing symbol {:?}: {e}", name, $n));
                *s
            }};
        }

        Api {
            name,
            path: path.to_path_buf(),
            charinbuf: sym!(FnCharInBuf, b"charinbuf\0"),
            increment_counter: sym!(FnIntToInt, b"increment_counter\0"),
            decrement_counter: sym!(FnIntToInt, b"decrement_counter\0"),
            multiply_counter: sym!(FnIntToInt, b"multiply_counter\0"),
            reset_counter: sym!(FnIntToInt, b"reset_counter\0"),
            is_string_empty: sym!(FnStrToInt, b"is_string_empty\0"),
            find_char_in_buffer: sym!(FnFind, b"find_char_in_buffer\0"),
            create_buffer: sym!(FnCreate, b"create_buffer\0"),
            validate_uint16_range: sym!(FnIntToInt, b"validate_uint16_range\0"),
            apply_operation: sym!(FnApply, b"apply_operation\0"),
            _lib: lib,
        }
    }

    /// The four counter operations in the order used by `charinbuf` mode 3.
    pub fn ops(&self) -> [OpFn; 4] {
        [
            self.reset_counter,
            self.increment_counter,
            self.multiply_counter,
            self.decrement_counter,
        ]
    }
}

// ---------------------------------------------------------------------------
// Library discovery
// ---------------------------------------------------------------------------

fn workspace_root() -> PathBuf {
    // .../harvest-work-XXXX/translation
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest.parent().expect("manifest parent").to_path_buf()
}

/// `c_src/build/lib<project>.so` — the project name is derived from the parent
/// directory name by CMakeLists.txt, so the file is located by extension.
pub fn c_lib_path() -> PathBuf {
    let dir = workspace_root().join("c_src").join("build");
    let mut found: Vec<PathBuf> = std::fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("cannot read {}: {e} — build the C library first", dir.display()))
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().map(|x| x == "so").unwrap_or(false))
        .collect();
    found.sort();
    assert_eq!(
        found.len(),
        1,
        "expected exactly one .so in {}, found {:?}",
        dir.display(),
        found
    );
    let so = found.pop().unwrap();

    // Guard against comparing against a stale ground truth: the .so must be at
    // least as new as every C source/header it is built from.
    let so_mtime = std::fs::metadata(&so)
        .and_then(|m| m.modified())
        .expect("stat C .so");
    let c_root = workspace_root().join("c_src");
    for sub in ["src", "include"] {
        if let Ok(rd) = std::fs::read_dir(c_root.join(sub)) {
            for e in rd.flatten() {
                let p = e.path();
                if let Ok(m) = std::fs::metadata(&p).and_then(|m| m.modified()) {
                    assert!(
                        m <= so_mtime,
                        "{} is newer than {} — rebuild the C library:\n  \
                         cd c_src/build && cmake --build .",
                        p.display(),
                        so.display()
                    );
                }
            }
        }
    }
    so
}

/// Newest mtime among the crate's own build inputs (`src/**.rs`, `Cargo.toml`).
fn newest_source_mtime() -> std::time::SystemTime {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let mut newest = std::time::SystemTime::UNIX_EPOCH;
    let mut stack = vec![root.join("src")];
    let consider = |p: &Path, newest: &mut std::time::SystemTime| {
        if let Ok(m) = std::fs::metadata(p).and_then(|m| m.modified()) {
            if m > *newest {
                *newest = m;
            }
        }
    };
    consider(&root.join("Cargo.toml"), &mut newest);
    while let Some(dir) = stack.pop() {
        if let Ok(rd) = std::fs::read_dir(&dir) {
            for e in rd.flatten() {
                let p = e.path();
                if p.is_dir() {
                    stack.push(p);
                } else {
                    consider(&p, &mut newest);
                }
            }
        }
    }
    newest
}

/// `target/<profile>/libcharinbuf_lib.so`, resolved relative to the running
/// test executable so that the .so from the *same* cargo profile is used.
///
/// `cargo test` does **not** build the `cdylib` artifact (a cdylib-only crate is
/// not a dependency of an integration test), so the file on disk can easily be
/// stale — which would silently validate an old translation. The path is
/// therefore rebuilt on demand and its freshness is asserted.
pub fn rust_lib_path() -> PathBuf {
    let exe = std::env::current_exe().expect("current_exe");
    // target/<profile>/deps/<test-bin>
    let profile_dir = exe
        .parent()
        .and_then(|p| p.parent())
        .expect("profile dir")
        .to_path_buf();
    let p = profile_dir.join("libcharinbuf_lib.so");
    let profile_name = profile_dir
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("debug")
        .to_string();

    let fresh = |p: &Path| -> bool {
        match std::fs::metadata(p).and_then(|m| m.modified()) {
            Ok(m) => m >= newest_source_mtime(),
            Err(_) => false,
        }
    };

    if fresh(&p) {
        return p;
    }

    // Build (or refresh) the cdylib for this exact profile.
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());
    let mut cmd = std::process::Command::new(cargo);
    cmd.current_dir(env!("CARGO_MANIFEST_DIR"))
        .arg("build")
        .arg("--lib");
    if profile_name != "debug" {
        cmd.arg("--profile").arg(&profile_name);
    }
    // Avoid inheriting the jobserver/lock of the outer cargo invocation.
    cmd.env_remove("CARGO_MAKEFLAGS");
    cmd.env_remove("RUSTC_WRAPPER");
    let status = cmd.status().expect("failed to spawn `cargo build --lib`");
    assert!(
        status.success(),
        "`cargo build --lib` for the cdylib failed (profile {profile_name})"
    );
    assert!(
        p.exists(),
        "Rust cdylib not found at {} even after `cargo build --lib`",
        p.display()
    );
    assert!(
        fresh(&p),
        "Rust cdylib at {} is OLDER than the crate sources even after rebuilding — \
         refusing to test a stale library",
        p.display()
    );
    p
}

static C_API: OnceLock<Api> = OnceLock::new();
static RUST_API: OnceLock<Api> = OnceLock::new();

pub fn c_api() -> &'static Api {
    C_API.get_or_init(|| Api::open("C", &c_lib_path()))
}

pub fn rust_api() -> &'static Api {
    RUST_API.get_or_init(|| Api::open("Rust", &rust_lib_path()))
}

/// `(c, rust)` — always returned as a pair so tests cannot forget one side.
pub fn both() -> (&'static Api, &'static Api) {
    (c_api(), rust_api())
}

// ---------------------------------------------------------------------------
// stdout capture (fd level — the libraries print with libc `printf`)
// ---------------------------------------------------------------------------

fn capture_lock() -> &'static Mutex<()> {
    static L: OnceLock<Mutex<()>> = OnceLock::new();
    L.get_or_init(|| Mutex::new(()))
}

static CAPTURE_SEQ: AtomicU64 = AtomicU64::new(0);

/// Runs `f` with file descriptor 1 redirected to a temporary file and returns
/// `(result, captured stdout bytes)`.
pub fn capture<R>(f: impl FnOnce() -> R) -> (R, Vec<u8>) {
    let _guard = capture_lock().lock().unwrap_or_else(|e| e.into_inner());

    let seq = CAPTURE_SEQ.fetch_add(1, Ordering::SeqCst);
    let path = std::env::temp_dir().join(format!(
        "charinbuf_capture_{}_{}_{}.txt",
        std::process::id(),
        seq,
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.subsec_nanos())
            .unwrap_or(0)
    ));
    let file = std::fs::File::create(&path).expect("create capture file");
    let tmp_fd = file.as_raw_fd();

    let result;
    unsafe {
        fflush(std::ptr::null_mut());
        let saved = dup(1);
        assert!(saved >= 0, "dup(1) failed");
        assert!(dup2(tmp_fd, 1) >= 0, "dup2 failed");

        result = f();

        fflush(std::ptr::null_mut());
        assert!(dup2(saved, 1) >= 0, "dup2 restore failed");
        close(saved);
    }
    drop(file);

    let bytes = std::fs::read(&path).expect("read capture file");
    let _ = std::fs::remove_file(&path);
    (result, bytes)
}

// ---------------------------------------------------------------------------
// Differential helpers
// ---------------------------------------------------------------------------

fn show(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).escape_debug().to_string()
}

/// Calls `charinbuf` on both libraries and asserts the return value *and* the
/// stdout bytes are identical.
pub fn diff_charinbuf(mode: c_int, value: c_int, opt1: c_int, opt2: c_int) {
    let (c, r) = both();
    let (rc, oc) = capture(|| unsafe { (c.charinbuf)(mode, value, opt1, opt2) });
    let (rr, or) = capture(|| unsafe { (r.charinbuf)(mode, value, opt1, opt2) });
    assert_eq!(
        rc, rr,
        "charinbuf({mode}, {value}, {opt1}, {opt2}) return mismatch\n  C   : {rc}\n  Rust: {rr}\n  C stdout   : {}\n  Rust stdout: {}",
        show(&oc),
        show(&or)
    );
    assert_eq!(
        oc,
        or,
        "charinbuf({mode}, {value}, {opt1}, {opt2}) stdout mismatch\n  C   : {}\n  Rust: {}",
        show(&oc),
        show(&or)
    );
}

/// Same as [`diff_charinbuf`] but also reports the captured C stdout so callers
/// can make extra assertions on it.
pub fn diff_charinbuf_capture(mode: c_int, value: c_int, opt1: c_int, opt2: c_int) -> (c_int, Vec<u8>) {
    let (c, r) = both();
    let (rc, oc) = capture(|| unsafe { (c.charinbuf)(mode, value, opt1, opt2) });
    let (rr, or) = capture(|| unsafe { (r.charinbuf)(mode, value, opt1, opt2) });
    assert_eq!(rc, rr, "charinbuf({mode}, {value}, {opt1}, {opt2}) return mismatch");
    assert_eq!(
        oc,
        or,
        "charinbuf({mode}, {value}, {opt1}, {opt2}) stdout mismatch\n  C   : {}\n  Rust: {}",
        show(&oc),
        show(&or)
    );
    (rc, oc)
}

/// High-volume differential: runs the whole batch of `charinbuf` calls against
/// one library with fd 1 redirected exactly once, then the same batch against
/// the other, and compares the complete return-value vector and the complete
/// stdout byte stream. Much cheaper per input than [`diff_charinbuf`], which
/// makes very large sweeps practical.
pub fn bulk_charinbuf(cases: &[(c_int, c_int, c_int, c_int)]) {
    let (c, r) = both();
    let run = |api: &Api| -> (Vec<c_int>, Vec<u8>) {
        capture(|| {
            let mut out = Vec::with_capacity(cases.len());
            for &(m, v, o1, o2) in cases {
                out.push(unsafe { (api.charinbuf)(m, v, o1, o2) });
            }
            out
        })
    };
    let (rets_c, out_c) = run(c);
    let (rets_r, out_r) = run(r);

    if rets_c != rets_r {
        let idx = rets_c
            .iter()
            .zip(&rets_r)
            .position(|(a, b)| a != b)
            .expect("vectors differ");
        let (m, v, o1, o2) = cases[idx];
        panic!(
            "bulk charinbuf return mismatch at case #{idx} charinbuf({m}, {v}, {o1}, {o2}): C {} vs Rust {}",
            rets_c[idx], rets_r[idx]
        );
    }
    if out_c != out_r {
        // Report the first differing line together with its input.
        let lines_c: Vec<&[u8]> = out_c.split(|&b| b == b'\n').collect();
        let lines_r: Vec<&[u8]> = out_r.split(|&b| b == b'\n').collect();
        let first = lines_c
            .iter()
            .zip(&lines_r)
            .position(|(a, b)| a != b)
            .unwrap_or(0);
        panic!(
            "bulk charinbuf stdout mismatch (first differing line #{first}):\n  C   : {}\n  Rust: {}",
            show(lines_c.get(first).copied().unwrap_or(b"<eof>")),
            show(lines_r.get(first).copied().unwrap_or(b"<eof>"))
        );
    }
}

/// Differential `create_buffer`: compares NULL-ness and the copied bytes, then
/// releases both allocations with libc `free` (the C contract).
pub fn diff_create_buffer(input: &[u8]) {
    assert_eq!(input.last(), Some(&0u8), "input must be NUL terminated");
    let (c, r) = both();
    unsafe {
        let pc = (c.create_buffer)(input.as_ptr() as *const c_char);
        let pr = (r.create_buffer)(input.as_ptr() as *const c_char);
        assert_eq!(
            pc.is_null(),
            pr.is_null(),
            "create_buffer NULL-ness mismatch for {:?}",
            show(input)
        );
        if !pc.is_null() {
            let sc = std::ffi::CStr::from_ptr(pc).to_bytes().to_vec();
            let sr = std::ffi::CStr::from_ptr(pr).to_bytes().to_vec();
            assert_eq!(
                sc,
                sr,
                "create_buffer content mismatch for {:?}: C {:?} vs Rust {:?}",
                show(input),
                show(&sc),
                show(&sr)
            );
            let expect = &input[..input.iter().position(|&b| b == 0).unwrap()];
            assert_eq!(sc, expect, "C create_buffer did not copy the input prefix");
            free(pc as *mut c_void);
            free(pr as *mut c_void);
        }
    }
}

/// Differential `find_char_in_buffer`: compares the *offset* of the returned
/// pointer (or `None` for NULL).
pub fn diff_find(buf: &[u8], size: usize, target: u8) {
    let (c, r) = both();
    let base = buf.as_ptr() as *const c_char;
    unsafe {
        let pc = (c.find_char_in_buffer)(base, size, target as c_char);
        let pr = (r.find_char_in_buffer)(base, size, target as c_char);
        let oc = if pc.is_null() {
            None
        } else {
            Some(pc as usize - base as usize)
        };
        let or = if pr.is_null() {
            None
        } else {
            Some(pr as usize - base as usize)
        };
        assert_eq!(
            oc, or,
            "find_char_in_buffer(len={}, size={size}, target={target:#04x}) mismatch: C {oc:?} vs Rust {or:?} (buffer {:?})",
            buf.len(),
            show(buf)
        );
    }
}

pub fn diff_find_null(size: usize, target: u8) {
    let (c, r) = both();
    unsafe {
        let pc = (c.find_char_in_buffer)(std::ptr::null(), size, target as c_char);
        let pr = (r.find_char_in_buffer)(std::ptr::null(), size, target as c_char);
        assert!(pc.is_null(), "C find_char_in_buffer(NULL) should be NULL");
        assert!(pr.is_null(), "Rust find_char_in_buffer(NULL) should be NULL");
    }
}

pub fn diff_is_string_empty(s: &[u8]) {
    assert_eq!(s.last(), Some(&0u8), "input must be NUL terminated");
    let (c, r) = both();
    unsafe {
        let rc = (c.is_string_empty)(s.as_ptr() as *const c_char);
        let rr = (r.is_string_empty)(s.as_ptr() as *const c_char);
        assert_eq!(rc, rr, "is_string_empty({:?}) mismatch: C {rc} vs Rust {rr}", show(s));
    }
}

pub fn diff_validate(value: c_int) {
    let (c, r) = both();
    unsafe {
        let rc = (c.validate_uint16_range)(value);
        let rr = (r.validate_uint16_range)(value);
        assert_eq!(
            rc, rr,
            "validate_uint16_range({value}) mismatch: C {rc} vs Rust {rr}"
        );
    }
}

/// Applies the same op/value to both libraries' counters and compares.
/// `op` selects one of `reset, increment, multiply, decrement` (0..=3).
pub fn diff_op(op: usize, value: c_int) {
    let (c, r) = both();
    unsafe {
        let rc = (c.ops()[op])(value);
        let rr = (r.ops()[op])(value);
        assert_eq!(
            rc, rr,
            "counter op #{op}({value}) mismatch: C {rc} vs Rust {rr}"
        );
    }
}

/// Same as [`diff_op`] but routed through `apply_operation` with the library's
/// own function pointer.
pub fn diff_apply(op: usize, value: c_int) {
    let (c, r) = both();
    unsafe {
        let rc = (c.apply_operation)(Some(c.ops()[op]), value);
        let rr = (r.apply_operation)(Some(r.ops()[op]), value);
        assert_eq!(
            rc, rr,
            "apply_operation(op #{op}, {value}) mismatch: C {rc} vs Rust {rr}"
        );
    }
}

/// Puts both counters into the same known state.
pub fn seed_counters(value: c_int) {
    let (c, r) = both();
    unsafe {
        let rc = (c.reset_counter)(value);
        let rr = (r.reset_counter)(value);
        assert_eq!(rc, rr, "reset_counter({value}) mismatch");
    }
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (xorshift64*) — fixed seed for reproducibility
// ---------------------------------------------------------------------------

pub const SEED: u64 = 0x2026_0827_C0FF_EE01;

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
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
    pub fn next_u32(&mut self) -> u32 {
        (self.next_u64() >> 32) as u32
    }
    pub fn next_i32(&mut self) -> i32 {
        self.next_u32() as i32
    }
    /// Uniform in `0..n` (n > 0).
    pub fn below(&mut self, n: usize) -> usize {
        (self.next_u64() % (n as u64)) as usize
    }
    pub fn range_i32(&mut self, lo: i32, hi: i32) -> i32 {
        let span = (hi as i64 - lo as i64 + 1) as u64;
        (lo as i64 + (self.next_u64() % span) as i64) as i32
    }
    pub fn byte(&mut self) -> u8 {
        (self.next_u64() >> 24) as u8
    }
    /// Random NUL-terminated string of `len` non-zero bytes.
    pub fn cstring(&mut self, len: usize) -> Vec<u8> {
        let mut v = Vec::with_capacity(len + 1);
        for _ in 0..len {
            let mut b = self.byte();
            if b == 0 {
                b = 1;
            }
            v.push(b);
        }
        v.push(0);
        v
    }
    /// Random `i32` biased towards interesting values.
    pub fn interesting_i32(&mut self) -> i32 {
        match self.below(8) {
            0 => 0,
            1 => 1,
            2 => -1,
            3 => i32::MAX,
            4 => i32::MIN,
            5 => 65535,
            6 => 65536,
            _ => self.next_i32(),
        }
    }
}

pub const EXTREMES: [i32; 11] = [
    i32::MIN,
    i32::MIN + 1,
    -65536,
    -1,
    0,
    1,
    255,
    65535,
    65536,
    i32::MAX - 1,
    i32::MAX,
];
