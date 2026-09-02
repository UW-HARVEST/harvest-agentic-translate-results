// Differential tests: C `.so` vs Rust `.so`, both loaded with `libloading`.
//
// Nothing in this file calls a Rust translation function directly; every call
// goes through a symbol resolved out of the compiled shared object, exactly as
// an external C consumer would do. That also exercises the `#[no_mangle]`
// export wrappers.
//
// Every comparison checks THREE things:
//   * the return value,
//   * every out-parameter / heap buffer the call produces,
//   * the exact bytes the call writes to stdout (fd 1 is redirected around it).

#![allow(clippy::missing_safety_doc)]

use libloading::{Library, Symbol};
use std::ffi::{c_char, c_int, c_void};
use std::io::Write;
use std::os::unix::io::AsRawFd;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

// ---------------------------------------------------------------------------
// libc bits the harness itself needs (the harness is allowed to use libc; the
// libraries under test are only ever reached through libloading).
// ---------------------------------------------------------------------------
unsafe extern "C" {
    fn fflush(stream: *mut c_void) -> c_int;
    fn dup(fd: c_int) -> c_int;
    fn dup2(old: c_int, new: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn free(p: *mut c_void);
    fn strlen(s: *const c_char) -> usize;
}

// ---------------------------------------------------------------------------
// Locating the two shared objects
// ---------------------------------------------------------------------------

/// `translation/` — the crate root.
fn crate_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// The C `.so`. Its name is derived from the parent directory name by
/// `CMakeLists.txt`, so glob for it instead of hard-coding.
fn c_so_path() -> PathBuf {
    let build = crate_root().parent().unwrap().join("c_src/build");
    let mut found: Vec<PathBuf> = std::fs::read_dir(&build)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}. Build the C library first.", build.display()))
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| {
            let n = p.file_name().unwrap_or_default().to_string_lossy().to_string();
            n.starts_with("lib") && n.ends_with(".so")
        })
        .collect();
    found.sort();
    assert_eq!(
        found.len(),
        1,
        "expected exactly one lib*.so in {}, found {:?}",
        build.display(),
        found
    );
    found.pop().unwrap()
}

/// The Rust `.so`, taken from the same profile directory the test binary itself
/// was built into, so `cargo test` and `cargo test --release` each check their
/// own artifact.
fn rust_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("RUST_SO_UNDER_TEST") {
        return PathBuf::from(p);
    }
    let exe = std::env::current_exe().expect("current_exe");
    // .../target/<profile>/deps/<testbin>
    let profile_dir = exe.parent().unwrap().parent().unwrap();
    let candidate = profile_dir.join("libcomplexmode_lib.so");
    assert!(
        candidate.exists(),
        "Rust cdylib not found at {}",
        candidate.display()
    );
    candidate
}

// ---------------------------------------------------------------------------
// The exported ABI of both libraries
// ---------------------------------------------------------------------------

type FnCreateResultString = unsafe extern "C" fn(*const c_char, c_int) -> *mut c_char;
type FnCheckPermissions = unsafe extern "C" fn(c_int, c_int) -> c_int;
type FnSafeAdd = unsafe extern "C" fn(c_int, c_int, c_int) -> c_int;
type FnMultiplyWithLog = unsafe extern "C" fn(c_int, c_int, *mut *mut c_char) -> c_int;
type FnCopyAndSum = unsafe extern "C" fn(*mut c_int, c_int) -> c_int;
type FnCompareOperations = unsafe extern "C" fn(*const c_char, *const c_char) -> c_int;
type FnComplexmode = unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int;

pub struct Api {
    pub tag: &'static str,
    pub path: PathBuf,
    pub create_result_string: FnCreateResultString,
    pub check_permissions: FnCheckPermissions,
    pub safe_add: FnSafeAdd,
    pub multiply_with_log: FnMultiplyWithLog,
    pub copy_and_sum: FnCopyAndSum,
    pub compare_operations: FnCompareOperations,
    pub complexmode: FnComplexmode,
}

/// Resolve every symbol out of `path`. The `Library` is leaked on purpose: the
/// function pointers must stay valid for the whole process lifetime.
fn load(tag: &'static str, path: &Path) -> Api {
    let lib = unsafe { Library::new(path) }
        .unwrap_or_else(|e| panic!("dlopen({}) failed: {e}", path.display()));
    let lib: &'static Library = Box::leak(Box::new(lib));

    macro_rules! sym {
        ($name:literal, $t:ty) => {{
            let s: Symbol<$t> = unsafe { lib.get($name) }.unwrap_or_else(|e| {
                panic!("{} does not export `{}`: {e}", path.display(), {
                    let b: &[u8] = $name;
                    String::from_utf8_lossy(&b[..b.len() - 1]).to_string()
                })
            });
            unsafe { *s.into_raw() }
        }};
    }

    Api {
        tag,
        path: path.to_path_buf(),
        create_result_string: sym!(b"create_result_string\0", FnCreateResultString),
        check_permissions: sym!(b"check_permissions\0", FnCheckPermissions),
        safe_add: sym!(b"safe_add\0", FnSafeAdd),
        multiply_with_log: sym!(b"multiply_with_log\0", FnMultiplyWithLog),
        copy_and_sum: sym!(b"copy_and_sum\0", FnCopyAndSum),
        compare_operations: sym!(b"compare_operations\0", FnCompareOperations),
        complexmode: sym!(b"complexmode\0", FnComplexmode),
    }
}

pub fn c() -> &'static Api {
    static C: OnceLock<Api> = OnceLock::new();
    C.get_or_init(|| load("C", &c_so_path()))
}

pub fn r() -> &'static Api {
    static R: OnceLock<Api> = OnceLock::new();
    R.get_or_init(|| load("RUST", &rust_so_path()))
}

// ---------------------------------------------------------------------------
// stdout capture
//
// Both libraries write through the *process* `stdout` FILE*, so fd 1 is
// swapped for a temp file around each call and the FILE* is flushed on both
// sides of the swap. A global mutex serialises this because fd 1 is global.
// ---------------------------------------------------------------------------

fn capture_lock() -> &'static Mutex<CapState> {
    static M: OnceLock<Mutex<CapState>> = OnceLock::new();
    M.get_or_init(|| {
        let mut tmp = std::env::temp_dir();
        tmp.push(format!("cmode-capture-{}.txt", std::process::id()));
        let file = std::fs::File::options()
            .read(true)
            .write(true)
            .create(true)
            .truncate(true)
            .open(&tmp)
            .expect("scratch capture file");
        Mutex::new(CapState { file, path: tmp })
    })
}

pub struct CapState {
    file: std::fs::File,
    #[allow(dead_code)]
    path: PathBuf,
}

/// Run `f` with fd 1 pointed at a scratch file; return `f`'s value and the
/// bytes written to stdout. The scratch file is reused across calls (creating
/// and deleting a file per call dominates the runtime of the randomized rows).
pub fn capture<R>(f: impl FnOnce() -> R) -> (R, Vec<u8>) {
    use std::io::{Read, Seek, SeekFrom};

    let mut st = capture_lock().lock().unwrap_or_else(|e| e.into_inner());

    st.file.set_len(0).expect("truncate scratch");
    st.file.seek(SeekFrom::Start(0)).expect("rewind scratch");

    let ret = unsafe {
        // Drain whatever is already buffered so it is not attributed to `f`.
        fflush(std::ptr::null_mut());
        let saved = dup(1);
        assert!(saved >= 0, "dup(1) failed");
        assert!(dup2(st.file.as_raw_fd(), 1) >= 0, "dup2 failed");

        let ret = f();

        fflush(std::ptr::null_mut());
        assert!(dup2(saved, 1) >= 0, "dup2 restore failed");
        close(saved);
        ret
    };

    st.file.seek(SeekFrom::Start(0)).expect("rewind scratch");
    let mut bytes = Vec::new();
    st.file.read_to_end(&mut bytes).expect("read capture");
    (ret, bytes)
}

// ---------------------------------------------------------------------------
// Comparison helper
// ---------------------------------------------------------------------------

/// One differential observation: return value + out-params + stdout bytes.
#[derive(PartialEq, Eq)]
pub struct Obs {
    pub ret: i64,
    pub aux: Vec<u8>,
    pub out: Vec<u8>,
}

impl std::fmt::Debug for Obs {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Obs {{ ret: {}, aux: {:?}, stdout: {:?} }}",
            self.ret,
            String::from_utf8_lossy(&self.aux),
            String::from_utf8_lossy(&self.out)
        )
    }
}

/// Accumulates failures so a whole randomized row is reported at once instead
/// of dying on the first mismatch.
#[derive(Default)]
pub struct Diffs {
    pub row: String,
    pub cases: usize,
    pub failures: Vec<String>,
}

impl Diffs {
    pub fn new(row: &str) -> Self {
        Diffs {
            row: row.to_string(),
            cases: 0,
            failures: Vec::new(),
        }
    }

    pub fn check(&mut self, label: impl std::fmt::Display, cobs: &Obs, robs: &Obs) {
        self.cases += 1;
        if cobs != robs {
            if self.failures.len() < 25 {
                self.failures
                    .push(format!("  {label}\n    C   : {cobs:?}\n    RUST: {robs:?}"));
            }
        }
    }

    pub fn finish(self) {
        assert!(
            self.cases > 0,
            "row `{}` executed zero cases — the test is vacuous",
            self.row
        );
        if !self.failures.is_empty() {
            let mut msg = format!(
                "row `{}`: {} of {} cases diverged\n",
                self.row,
                self.failures.len(),
                self.cases
            );
            for f in &self.failures {
                msg.push_str(f);
                msg.push('\n');
            }
            let _ = std::io::stderr().write_all(msg.as_bytes());
            panic!("{}", msg);
        }
    }
}

// ---------------------------------------------------------------------------
// Deterministic RNG (SplitMix64) — fixed seed, reproducible.
// ---------------------------------------------------------------------------
pub const SEED: u64 = 0x5EED_1234_ABCD_F00D;

pub struct Rng(u64);

impl Rng {
    pub fn new(salt: u64) -> Self {
        Rng(SEED ^ salt.wrapping_mul(0x9E37_79B9_7F4A_7C15))
    }
    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    pub fn i32(&mut self) -> i32 {
        self.next_u64() as u32 as i32
    }
    /// Biased toward interesting values: small magnitudes and the extremes,
    /// mixed with full-range noise. A purely uniform i32 almost never hits
    /// 0, ±1, INT_MAX or INT_MIN.
    pub fn i32_interesting(&mut self) -> i32 {
        let v = self.next_u64();
        match v % 8 {
            0 => 0,
            1 => 1,
            2 => -1,
            3 => i32::MAX,
            4 => i32::MIN,
            5 => ((v >> 8) % 512) as i32 - 256,
            _ => self.i32(),
        }
    }
    pub fn range(&mut self, lo: u64, hi_inclusive: u64) -> u64 {
        lo + self.next_u64() % (hi_inclusive - lo + 1)
    }
    /// A random NUL-free byte string of length `len`, drawn from `alphabet`.
    pub fn bytes(&mut self, len: usize, alphabet: &[u8]) -> Vec<u8> {
        (0..len)
            .map(|_| alphabet[(self.next_u64() % alphabet.len() as u64) as usize])
            .collect()
    }
}

/// NUL-free alphabets. Byte 0 is excluded everywhere because it would
/// terminate the C string.
pub const ASCII: &[u8] = b"abcXYZ019 _-%d.";
pub const FULL_BYTES: &[u8] = &{
    let mut a = [0u8; 255];
    let mut i = 0;
    while i < 255 {
        a[i] = (i + 1) as u8;
        i += 1;
    }
    a
};

/// Turn bytes into a NUL-terminated C buffer.
pub fn cstr(bytes: &[u8]) -> Vec<u8> {
    let mut v = bytes.to_vec();
    v.push(0);
    v
}

/// Read a NUL-terminated string out of a heap pointer and `free` it.
/// Bytes past the NUL are left uninitialized by `snprintf`, so only the
/// string itself (plus its terminator) is comparable.
unsafe fn take_cstring(p: *mut c_char) -> Vec<u8> {
    if p.is_null() {
        return b"<NULL>".to_vec();
    }
    let n = unsafe { strlen(p) };
    let bytes = unsafe { std::slice::from_raw_parts(p as *const u8, n + 1) }.to_vec();
    unsafe { free(p as *mut c_void) };
    bytes
}

// ===========================================================================
// Per-function observation wrappers. Each one drives ONE library through its
// `.so` export and packages everything observable into an `Obs`.
// ===========================================================================

fn obs_check_permissions(a: &Api, perms: i32, required: i32) -> Obs {
    let f = a.check_permissions;
    let (ret, out) = capture(|| unsafe { f(perms, required) });
    Obs { ret: ret as i64, aux: Vec::new(), out }
}

fn obs_safe_add(a: &Api, x: i32, y: i32, perms: i32) -> Obs {
    let f = a.safe_add;
    let (ret, out) = capture(|| unsafe { f(x, y, perms) });
    Obs { ret: ret as i64, aux: Vec::new(), out }
}

/// `op` is passed as a raw NUL-terminated buffer; `None` means a NULL pointer.
fn obs_create_result_string(a: &Api, op: Option<&[u8]>, val: i32) -> Obs {
    let f = a.create_result_string;
    let buf = op.map(cstr);
    let p = match &buf {
        Some(b) => b.as_ptr() as *const c_char,
        None => std::ptr::null(),
    };
    let (ret, out) = capture(|| unsafe { f(p, val) });
    let aux = unsafe { take_cstring(ret) };
    Obs {
        // The pointer value itself is not comparable across libraries; only
        // whether it was NULL is.
        ret: if ret.is_null() { 0 } else { 1 },
        aux,
        out,
    }
}

fn obs_multiply_with_log(a: &Api, x: i32, y: i32) -> Obs {
    let f = a.multiply_with_log;
    let mut log: *mut c_char = std::ptr::null_mut();
    let logp: *mut *mut c_char = &mut log;
    let (ret, out) = capture(|| unsafe { f(x, y, logp) });
    let aux = unsafe { take_cstring(log) };
    Obs { ret: ret as i64, aux, out }
}

/// `src_len` may exceed `count` (to prove no over-read) or be `None` for NULL.
fn obs_copy_and_sum(a: &Api, src: Option<&[i32]>, count: i32) -> Obs {
    let f = a.copy_and_sum;
    let mut buf = src.map(|s| s.to_vec());
    let p = match &mut buf {
        Some(b) => b.as_mut_ptr(),
        None => std::ptr::null_mut(),
    };
    let (ret, out) = capture(|| unsafe { f(p, count) });
    // The source buffer must come back untouched.
    let aux = buf
        .map(|b| b.iter().flat_map(|v| v.to_le_bytes()).collect::<Vec<u8>>())
        .unwrap_or_default();
    Obs { ret: ret as i64, aux, out }
}

fn obs_compare_operations(a: &Api, op1: Option<&[u8]>, op2: Option<&[u8]>) -> Obs {
    let f = a.compare_operations;
    let b1 = op1.map(cstr);
    let b2 = op2.map(cstr);
    let p1 = b1.as_ref().map_or(std::ptr::null(), |b| b.as_ptr() as *const c_char);
    let p2 = b2.as_ref().map_or(std::ptr::null(), |b| b.as_ptr() as *const c_char);
    let (ret, out) = capture(|| unsafe { f(p1, p2) });
    Obs { ret: ret as i64, aux: Vec::new(), out }
}

fn obs_complexmode(a: &Api, mode: i32, v1: i32, v2: i32, v3: i32) -> Obs {
    let f = a.complexmode;
    let (ret, out) = capture(|| unsafe { f(mode, v1, v2, v3) });
    Obs { ret: ret as i64, aux: Vec::new(), out }
}

// ===========================================================================
// PHASE A / PHASE D — symbol parity
// ===========================================================================

#[test]
fn phase_a_symbol_parity() {
    fn defined(path: &Path) -> Vec<String> {
        let o = std::process::Command::new("nm")
            .args(["-D", "--defined-only", "--format=posix"])
            .arg(path)
            .output()
            .expect("nm must be available");
        assert!(o.status.success(), "nm failed on {}", path.display());
        String::from_utf8_lossy(&o.stdout)
            .lines()
            .filter_map(|l| l.split_whitespace().next().map(|s| s.to_string()))
            .collect()
    }

    let cs = defined(&c_so_path());
    let rs = defined(&rust_so_path());
    assert!(!cs.is_empty(), "C .so exported nothing — bad build?");

    let missing: Vec<&String> = cs.iter().filter(|s| !rs.contains(s)).collect();
    assert!(
        missing.is_empty(),
        "symbols exported by the C .so but MISSING from the Rust .so: {missing:?}"
    );

    // Sanity: the seven documented entry points really are there.
    for want in [
        "create_result_string",
        "check_permissions",
        "safe_add",
        "multiply_with_log",
        "copy_and_sum",
        "compare_operations",
        "complexmode",
    ] {
        assert!(cs.iter().any(|s| s == want), "C .so lost `{want}`");
        assert!(rs.iter().any(|s| s == want), "Rust .so lost `{want}`");
    }

    // And that both libraries resolve completely at load time.
    for p in [c_so_path(), rust_so_path()] {
        let o = std::process::Command::new("ldd").arg("-r").arg(&p).output();
        if let Ok(o) = o {
            let txt = format!(
                "{}{}",
                String::from_utf8_lossy(&o.stdout),
                String::from_utf8_lossy(&o.stderr)
            );
            assert!(
                !txt.contains("undefined symbol"),
                "{} has unresolved symbols:\n{txt}",
                p.display()
            );
        }
    }
}

#[test]
fn phase_a_both_libraries_load() {
    // Forces the dlopen + all seven symbol lookups on both sides.
    assert_eq!(c().tag, "C");
    assert_eq!(r().tag, "RUST");
    assert_ne!(c().path, r().path);
}

// ===========================================================================
// PHASE B — valid-path differential tests, one test per CONFIGS.md row group.
// ===========================================================================

/// CONFIGS rows 1–5 — `check_permissions` across every permission shape.
#[test]
fn cfg_check_permissions() {
    // row 1: required == 0 accepts everything
    let mut d = Diffs::new("CONFIGS#1 check_permissions required==0");
    let mut rng = Rng::new(1);
    for _ in 0..512 {
        let p = rng.i32_interesting();
        d.check(
            format!("perms={p} required=0"),
            &obs_check_permissions(c(), p, 0),
            &obs_check_permissions(r(), p, 0),
        );
    }
    d.finish();

    // row 2: required is an exact subset of perms -> accept
    let mut d = Diffs::new("CONFIGS#2 check_permissions required subset");
    let mut rng = Rng::new(2);
    for _ in 0..512 {
        let p = rng.i32();
        let req = p & rng.i32(); // guaranteed subset of p
        d.check(
            format!("perms={p} required={req}"),
            &obs_check_permissions(c(), p, req),
            &obs_check_permissions(r(), p, req),
        );
    }
    d.finish();

    // row 3: required shares only some bits -> reject
    let mut d = Diffs::new("CONFIGS#3 check_permissions partial overlap");
    let mut rng = Rng::new(3);
    for _ in 0..512 {
        let p = rng.i32();
        // force at least one required bit that p lacks
        let missing_bit = 1i32 << (rng.range(0, 31) as u32);
        let req = (p & rng.i32()) | missing_bit;
        let p = p & !missing_bit;
        d.check(
            format!("perms={p} required={req}"),
            &obs_check_permissions(c(), p, req),
            &obs_check_permissions(r(), p, req),
        );
    }
    d.finish();

    // row 4: fully random, incl. negatives / sign bit / extremes
    let mut d = Diffs::new("CONFIGS#4 check_permissions random");
    let mut rng = Rng::new(4);
    for _ in 0..4096 {
        let p = rng.i32_interesting();
        let req = rng.i32_interesting();
        d.check(
            format!("perms={p} required={req}"),
            &obs_check_permissions(c(), p, req),
            &obs_check_permissions(r(), p, req),
        );
    }
    d.finish();

    // row 5: the library's own constants
    let mut d = Diffs::new("CONFIGS#5 check_permissions library constants");
    for p in [0o644, 0o600, 0o400, 0o200, 0o100, 0o777, 0, -1] {
        for req in [0o400, 0o200, 0o100, 0o600, 0o644, 0o777, 0] {
            d.check(
                format!("perms={p:o} required={req:o}"),
                &obs_check_permissions(c(), p, req),
                &obs_check_permissions(r(), p, req),
            );
        }
    }
    d.finish();
}

/// CONFIGS rows 6–8 — `safe_add`, both the accept and the reject branch.
#[test]
fn cfg_safe_add() {
    const RW: i32 = 0o600;

    // row 6: perms superset of 0600 -> accept
    let mut d = Diffs::new("CONFIGS#6 safe_add accept");
    let mut rng = Rng::new(6);
    for _ in 0..4096 {
        let x = rng.i32_interesting();
        let y = rng.i32_interesting();
        let perms = (rng.i32() | RW) & 0x7FFF_FFFF;
        d.check(
            format!("a={x} b={y} perms={perms:o}"),
            &obs_safe_add(c(), x, y, perms),
            &obs_safe_add(r(), x, y, perms),
        );
    }
    d.finish();

    // row 7: overflow / underflow on the accept path
    let mut d = Diffs::new("CONFIGS#7 safe_add overflow");
    for (x, y) in [
        (i32::MAX, 1),
        (i32::MAX, i32::MAX),
        (i32::MIN, -1),
        (i32::MIN, i32::MIN),
        (i32::MAX, -1),
        (i32::MIN, 1),
        (1 << 30, 1 << 30),
        (-(1 << 30), -(1 << 30)),
    ] {
        d.check(
            format!("a={x} b={y}"),
            &obs_safe_add(c(), x, y, 0o644),
            &obs_safe_add(r(), x, y, 0o644),
        );
    }
    let mut rng = Rng::new(7);
    for _ in 0..512 {
        // pick values whose sum is very likely to overflow
        let x = i32::MAX - (rng.range(0, 64) as i32);
        let y = i32::MAX - (rng.range(0, 64) as i32);
        d.check(
            format!("a={x} b={y}"),
            &obs_safe_add(c(), x, y, 0o644),
            &obs_safe_add(r(), x, y, 0o644),
        );
        let x = i32::MIN + (rng.range(0, 64) as i32);
        let y = i32::MIN + (rng.range(0, 64) as i32);
        d.check(
            format!("a={x} b={y}"),
            &obs_safe_add(c(), x, y, 0o644),
            &obs_safe_add(r(), x, y, 0o644),
        );
    }
    d.finish();

    // row 8: reject path (also ERRORS rows 5–7)
    let mut d = Diffs::new("CONFIGS#8 safe_add reject");
    let mut rng = Rng::new(8);
    for perms in [0, 0o400, 0o200, 0o100, 0o444, 0o111, 0o500, 0o300] {
        for _ in 0..16 {
            let x = rng.i32_interesting();
            let y = rng.i32_interesting();
            d.check(
                format!("a={x} b={y} perms={perms:o}"),
                &obs_safe_add(c(), x, y, perms),
                &obs_safe_add(r(), x, y, perms),
            );
        }
    }
    for _ in 0..1900 {
        // random perms with at least one of the two required bits cleared
        let drop_bit = if rng.next_u64() % 2 == 0 { 0o400 } else { 0o200 };
        let perms = rng.i32() & !drop_bit;
        let x = rng.i32_interesting();
        let y = rng.i32_interesting();
        d.check(
            format!("a={x} b={y} perms={perms:o}"),
            &obs_safe_add(c(), x, y, perms),
            &obs_safe_add(r(), x, y, perms),
        );
    }
    d.finish();
}

/// CONFIGS rows 9–12 — `create_result_string` across string and value shapes.
#[test]
fn cfg_create_result_string() {
    // row 9: empty op
    let mut d = Diffs::new("CONFIGS#9 create_result_string empty op");
    let mut rng = Rng::new(9);
    for _ in 0..256 {
        let v = rng.i32_interesting();
        d.check(
            format!("op=\"\" val={v}"),
            &obs_create_result_string(c(), Some(b""), v),
            &obs_create_result_string(r(), Some(b""), v),
        );
    }
    d.finish();

    // row 10: short ASCII ops, values incl. extremes
    let mut d = Diffs::new("CONFIGS#10 create_result_string short ascii");
    let mut rng = Rng::new(10);
    for _ in 0..2048 {
        let len = rng.range(1, 12) as usize;
        let op = rng.bytes(len, ASCII);
        let v = rng.i32_interesting();
        d.check(
            format!("op={:?} val={v}", String::from_utf8_lossy(&op)),
            &obs_create_result_string(c(), Some(&op), v),
            &obs_create_result_string(r(), Some(&op), v),
        );
    }
    d.finish();

    // row 11: exhaustive length sweep across the 64-byte snprintf boundary
    let mut d = Diffs::new("CONFIGS#11 create_result_string truncation sweep");
    let mut rng = Rng::new(11);
    for len in 0..=80usize {
        for v in [0, 7, -7, i32::MAX, i32::MIN, 1234567890] {
            let op = rng.bytes(len, ASCII);
            d.check(
                format!("len={len} val={v}"),
                &obs_create_result_string(c(), Some(&op), v),
                &obs_create_result_string(r(), Some(&op), v),
            );
        }
    }
    d.finish();

    // row 12: high-bit / format-looking bytes
    let mut d = Diffs::new("CONFIGS#12 create_result_string arbitrary bytes");
    let mut rng = Rng::new(12);
    for _ in 0..512 {
        let len = rng.range(1, 70) as usize;
        let op = rng.bytes(len, FULL_BYTES);
        let v = rng.i32_interesting();
        d.check(
            format!("len={len} val={v}"),
            &obs_create_result_string(c(), Some(&op), v),
            &obs_create_result_string(r(), Some(&op), v),
        );
    }
    for op in [
        &b"%s"[..],
        b"%d",
        b"%n",
        b"%%",
        b"100%",
        b"\xff\xfe\xfd",
        b"tab\there",
        b"nl-is-absent",
    ] {
        d.check(
            format!("op={:?}", String::from_utf8_lossy(op)),
            &obs_create_result_string(c(), Some(op), -42),
            &obs_create_result_string(r(), Some(op), -42),
        );
    }
    d.finish();
}

/// CONFIGS rows 13–15 — `multiply_with_log`, return value plus heap log string.
#[test]
fn cfg_multiply_with_log() {
    // row 13: random
    let mut d = Diffs::new("CONFIGS#13 multiply_with_log random");
    let mut rng = Rng::new(13);
    for _ in 0..4096 {
        let x = rng.i32_interesting();
        let y = rng.i32_interesting();
        d.check(
            format!("a={x} b={y}"),
            &obs_multiply_with_log(c(), x, y),
            &obs_multiply_with_log(r(), x, y),
        );
    }
    d.finish();

    // row 14: overflowing products
    let mut d = Diffs::new("CONFIGS#14 multiply_with_log overflow");
    for (x, y) in [
        (i32::MIN, -1),
        (i32::MIN, -2),
        (i32::MAX, 2),
        (i32::MAX, i32::MAX),
        (i32::MIN, i32::MIN),
        (1 << 16, 1 << 16),
        (-(1 << 16), 1 << 16),
        (65535, 65537),
    ] {
        d.check(
            format!("a={x} b={y}"),
            &obs_multiply_with_log(c(), x, y),
            &obs_multiply_with_log(r(), x, y),
        );
    }
    let mut rng = Rng::new(14);
    for _ in 0..512 {
        let x = rng.i32() | 0x4000_0000;
        let y = rng.i32() | 0x4000_0000;
        d.check(
            format!("a={x} b={y}"),
            &obs_multiply_with_log(c(), x, y),
            &obs_multiply_with_log(r(), x, y),
        );
    }
    d.finish();

    // row 15: product == 0, colliding with the failure sentinel
    let mut d = Diffs::new("CONFIGS#15 multiply_with_log zero product");
    for (x, y) in [(0, 0), (0, 1), (1, 0), (0, i32::MAX), (i32::MIN, 0), (0, -1)] {
        d.check(
            format!("a={x} b={y}"),
            &obs_multiply_with_log(c(), x, y),
            &obs_multiply_with_log(r(), x, y),
        );
    }
    d.finish();
}

/// CONFIGS rows 16–22 — `copy_and_sum` across every count shape.
#[test]
fn cfg_copy_and_sum() {
    // row 16: count == 0
    let mut d = Diffs::new("CONFIGS#16 copy_and_sum count=0");
    for src in [&[][..], &[1][..], &[1, 2, 3][..]] {
        d.check(
            format!("src_len={}", src.len()),
            &obs_copy_and_sum(c(), Some(src), 0),
            &obs_copy_and_sum(r(), Some(src), 0),
        );
    }
    d.finish();

    // rows 17–19: count 1, 2, 3
    for (row, n) in [(17, 1usize), (18, 2), (19, 3)] {
        let iters = if n == 3 { 2048 } else { 512 };
        let mut d = Diffs::new(&format!("CONFIGS#{row} copy_and_sum count={n}"));
        let mut rng = Rng::new(100 + n as u64);
        for _ in 0..iters {
            let v: Vec<i32> = (0..n).map(|_| rng.i32_interesting()).collect();
            d.check(
                format!("src={v:?}"),
                &obs_copy_and_sum(c(), Some(&v), n as i32),
                &obs_copy_and_sum(r(), Some(&v), n as i32),
            );
        }
        d.finish();
    }

    // row 20: "many"
    let mut d = Diffs::new("CONFIGS#20 copy_and_sum many");
    let mut rng = Rng::new(20);
    for _ in 0..256 {
        let n = rng.range(4, 4096) as usize;
        let v: Vec<i32> = (0..n).map(|_| rng.i32_interesting()).collect();
        d.check(
            format!("count={n}"),
            &obs_copy_and_sum(c(), Some(&v), n as i32),
            &obs_copy_and_sum(r(), Some(&v), n as i32),
        );
    }
    d.finish();

    // row 21: accumulator overflow
    let mut d = Diffs::new("CONFIGS#21 copy_and_sum sum overflow");
    let mut rng = Rng::new(21);
    for _ in 0..512 {
        let n = rng.range(2, 64) as usize;
        let v: Vec<i32> = (0..n)
            .map(|_| {
                if rng.next_u64() % 2 == 0 {
                    i32::MAX - (rng.range(0, 8) as i32)
                } else {
                    i32::MIN + (rng.range(0, 8) as i32)
                }
            })
            .collect();
        d.check(
            format!("count={n} src={v:?}"),
            &obs_copy_and_sum(c(), Some(&v), n as i32),
            &obs_copy_and_sum(r(), Some(&v), n as i32),
        );
    }
    d.finish();

    // row 22: count < buffer length, and count == buffer length
    let mut d = Diffs::new("CONFIGS#22 copy_and_sum partial buffer");
    let mut rng = Rng::new(22);
    for _ in 0..256 {
        let n = rng.range(1, 128) as usize;
        let v: Vec<i32> = (0..n).map(|_| rng.i32_interesting()).collect();
        let count = rng.range(0, n as u64) as i32;
        d.check(
            format!("buflen={n} count={count}"),
            &obs_copy_and_sum(c(), Some(&v), count),
            &obs_copy_and_sum(r(), Some(&v), count),
        );
        d.check(
            format!("buflen={n} count={n}"),
            &obs_copy_and_sum(c(), Some(&v), n as i32),
            &obs_copy_and_sum(r(), Some(&v), n as i32),
        );
    }
    d.finish();
}

/// CONFIGS rows 23–26 — `compare_operations`, exact `strcmp` value.
#[test]
fn cfg_compare_operations() {
    // row 23: equal strings
    let mut d = Diffs::new("CONFIGS#23 compare_operations equal");
    let mut rng = Rng::new(23);
    for _ in 0..1024 {
        let len = rng.range(0, 40) as usize;
        let s = rng.bytes(len, FULL_BYTES);
        d.check(
            format!("len={len}"),
            &obs_compare_operations(c(), Some(&s), Some(&s)),
            &obs_compare_operations(r(), Some(&s), Some(&s)),
        );
    }
    d.finish();

    // row 24: differ at a random position — magnitude matters
    let mut d = Diffs::new("CONFIGS#24 compare_operations differing");
    let mut rng = Rng::new(24);
    for _ in 0..4096 {
        let len = rng.range(1, 40) as usize;
        let a = rng.bytes(len, FULL_BYTES);
        let mut b = a.clone();
        let pos = rng.range(0, len as u64 - 1) as usize;
        b[pos] = FULL_BYTES[(rng.next_u64() % FULL_BYTES.len() as u64) as usize];
        d.check(
            format!("len={len} pos={pos} a[pos]={} b[pos]={}", a[pos], b[pos]),
            &obs_compare_operations(c(), Some(&a), Some(&b)),
            &obs_compare_operations(r(), Some(&a), Some(&b)),
        );
    }
    d.finish();

    // row 25: strict prefix
    let mut d = Diffs::new("CONFIGS#25 compare_operations prefix");
    let mut rng = Rng::new(25);
    for _ in 0..1024 {
        let len = rng.range(1, 40) as usize;
        let a = rng.bytes(len, FULL_BYTES);
        let cut = rng.range(0, len as u64 - 1) as usize;
        let b = &a[..cut];
        d.check(
            format!("len={len} cut={cut}"),
            &obs_compare_operations(c(), Some(&a), Some(b)),
            &obs_compare_operations(r(), Some(&a), Some(b)),
        );
        d.check(
            format!("len={len} cut={cut} swapped"),
            &obs_compare_operations(c(), Some(b), Some(&a)),
            &obs_compare_operations(r(), Some(b), Some(&a)),
        );
    }
    d.finish();

    // row 26: high-bit bytes — signed vs unsigned char comparison
    let mut d = Diffs::new("CONFIGS#26 compare_operations high-bit bytes");
    let mut d2 = Diffs::new("CONFIGS#26b compare_operations high-bit exhaustive");
    for lo in [0x01u8, 0x41, 0x7e, 0x7f] {
        for hi in [0x80u8, 0x81, 0xfe, 0xff] {
            for (a, b) in [(vec![lo], vec![hi]), (vec![hi], vec![lo]), (vec![hi], vec![hi])] {
                d2.check(
                    format!("a={a:?} b={b:?}"),
                    &obs_compare_operations(c(), Some(&a), Some(&b)),
                    &obs_compare_operations(r(), Some(&a), Some(&b)),
                );
            }
        }
    }
    d2.finish();
    let mut rng = Rng::new(26);
    let highbit: Vec<u8> = (0x80..=0xffu8).collect();
    for _ in 0..2048 {
        let len = rng.range(1, 16) as usize;
        let a = rng.bytes(len, &highbit);
        let b = rng.bytes(len, &highbit);
        d.check(
            format!("a={a:?} b={b:?}"),
            &obs_compare_operations(c(), Some(&a), Some(&b)),
            &obs_compare_operations(r(), Some(&a), Some(&b)),
        );
    }
    d.finish();
}

/// CONFIGS rows 27–37 — `complexmode`, the composed entry point, every mode.
#[test]
fn cfg_complexmode_modes() {
    // rows 27/29/31/33: each valid mode with randomized values
    for (row, mode) in [(27, 1i32), (29, 2), (31, 3), (33, 4)] {
        let mut d = Diffs::new(&format!("CONFIGS#{row} complexmode mode={mode}"));
        let mut rng = Rng::new(200 + mode as u64);
        for _ in 0..4096 {
            let (a, b, cc) = (
                rng.i32_interesting(),
                rng.i32_interesting(),
                rng.i32_interesting(),
            );
            d.check(
                format!("mode={mode} v=({a},{b},{cc})"),
                &obs_complexmode(c(), mode, a, b, cc),
                &obs_complexmode(r(), mode, a, b, cc),
            );
        }
        d.finish();
    }

    // rows 28/30/32/34: the same modes driven into arithmetic overflow
    for (row, mode) in [(28, 1i32), (30, 2), (32, 3), (34, 4)] {
        let mut d = Diffs::new(&format!("CONFIGS#{row} complexmode mode={mode} overflow"));
        let mut rng = Rng::new(300 + mode as u64);
        for _ in 0..512 {
            let big = |rng: &mut Rng| {
                if rng.next_u64() % 2 == 0 {
                    i32::MAX - (rng.range(0, 16) as i32)
                } else {
                    i32::MIN + (rng.range(0, 16) as i32)
                }
            };
            let (a, b, cc) = (big(&mut rng), big(&mut rng), big(&mut rng));
            d.check(
                format!("mode={mode} v=({a},{b},{cc})"),
                &obs_complexmode(c(), mode, a, b, cc),
                &obs_complexmode(r(), mode, a, b, cc),
            );
        }
        d.finish();
    }

    // row 35: exhaustive cross-product of extremes
    let mut d = Diffs::new("CONFIGS#35 complexmode extremes cross-product");
    const EXTREMES: [i32; 5] = [0, 1, -1, i32::MAX, i32::MIN];
    for mode in 1..=4i32 {
        for a in EXTREMES {
            for b in EXTREMES {
                for cc in EXTREMES {
                    d.check(
                        format!("mode={mode} v=({a},{b},{cc})"),
                        &obs_complexmode(c(), mode, a, b, cc),
                        &obs_complexmode(r(), mode, a, b, cc),
                    );
                }
            }
        }
    }
    d.finish();

    // row 37: fully unconstrained cross-product (mixes valid and invalid modes)
    let mut d = Diffs::new("CONFIGS#37 complexmode fully random");
    let mut rng = Rng::new(37);
    for _ in 0..8192 {
        let mode = match rng.next_u64() % 3 {
            0 => rng.range(0, 6) as i32 - 1, // clusters around the switch range
            1 => rng.i32_interesting(),
            _ => rng.i32(),
        };
        let (a, b, cc) = (
            rng.i32_interesting(),
            rng.i32_interesting(),
            rng.i32_interesting(),
        );
        d.check(
            format!("mode={mode} v=({a},{b},{cc})"),
            &obs_complexmode(c(), mode, a, b, cc),
            &obs_complexmode(r(), mode, a, b, cc),
        );
    }
    d.finish();
}

/// ERRORS row 25 — mode 1's `safe_add` rejection branch is dead because
/// `complexmode` hard-codes `permissions = 0644`. Assert the message never
/// appears, in both libraries.
#[test]
fn cfg_complexmode_mode1_permission_branch_is_dead() {
    let mut rng = Rng::new(2501);
    let mut d = Diffs::new("ERRORS#25 complexmode mode1 perms branch");
    for _ in 0..512 {
        let (a, b) = (rng.i32_interesting(), rng.i32_interesting());
        let co = obs_complexmode(c(), 1, a, b, rng.i32());
        let ro = obs_complexmode(r(), 1, a, b, 0);
        // stdout of the two calls above uses different v3, but v3 is unused in
        // mode 1, so they must still agree.
        d.check(format!("a={a} b={b}"), &co, &ro);
        let s = String::from_utf8_lossy(&co.out).to_string();
        assert!(
            !s.contains("Insufficient permissions"),
            "mode 1 unexpectedly hit the safe_add rejection branch: {s:?}"
        );
        assert!(s.contains("Mode 1: Addition"), "missing mode 1 banner: {s:?}");
        assert!(
            s.contains("Operation performed: addition"),
            "missing trailer: {s:?}"
        );
    }
    d.finish();
}

/// ERRORS row 26 — mode 4 always takes the `else` branch because
/// `0644 & 0100 == 0`, so the result is `v1+v2+v3`, never `v1*v2+v3`.
#[test]
fn cfg_complexmode_mode4_takes_else_branch() {
    let mut rng = Rng::new(2601);
    let mut d = Diffs::new("ERRORS#26 complexmode mode4 else branch");
    for _ in 0..1024 {
        let (a, b, cc) = (rng.i32_interesting(), rng.i32_interesting(), rng.i32_interesting());
        let co = obs_complexmode(c(), 4, a, b, cc);
        d.check(
            format!("v=({a},{b},{cc})"),
            &co,
            &obs_complexmode(r(), 4, a, b, cc),
        );
        let expect_else = a.wrapping_add(b).wrapping_add(cc);
        assert_eq!(
            co.ret as i32, expect_else,
            "mode 4 must use the additive else branch for ({a},{b},{cc})"
        );
    }
    d.finish();
}

/// CONFIGS rows 38–40 — composed pipelines and cross-library buffer exchange.
/// These feed one export's output into another's input, which per-call tests
/// cannot reach.
#[test]
fn cfg_composed_pipelines() {
    // row 38: create_result_string -> compare_operations -> copy_and_sum
    let mut d = Diffs::new("CONFIGS#38 composed pipeline");
    let mut rng = Rng::new(38);
    for _ in 0..1024 {
        let v1 = rng.i32_interesting();
        let v2 = rng.i32_interesting();
        let la = rng.range(0, 20) as usize;
        let opa = rng.bytes(la, ASCII);
        let opb = if rng.next_u64() % 2 == 0 {
            opa.clone()
        } else {
            let lb = rng.range(0, 20) as usize;
            rng.bytes(lb, ASCII)
        };

        // Run the whole chain inside ONE library, then compare the chains.
        let run = |a: &Api| -> Obs {
            let (ret, out) = capture(|| unsafe {
                let ba = cstr(&opa);
                let bb = cstr(&opb);
                let s1 = (a.create_result_string)(ba.as_ptr() as *const c_char, v1);
                let s2 = (a.create_result_string)(bb.as_ptr() as *const c_char, v2);
                let cmp = (a.compare_operations)(s1 as *const c_char, s2 as *const c_char);
                let mut vals = [cmp, v1, v2];
                let sum = (a.copy_and_sum)(vals.as_mut_ptr(), 3);
                let s1b = take_cstring(s1);
                let s2b = take_cstring(s2);
                (cmp, sum, s1b, s2b)
            });
            let (cmp, sum, s1b, s2b) = ret;
            let mut aux = s1b;
            aux.extend_from_slice(&s2b);
            Obs {
                ret: ((cmp as i64) << 32) ^ (sum as u32 as i64),
                aux,
                out,
            }
        };
        d.check(format!("v1={v1} v2={v2}"), &run(c()), &run(r()));
    }
    d.finish();

    // row 39: multiply_with_log's string must equal
    // create_result_string("multiply", a*b) within the same library.
    let mut d = Diffs::new("CONFIGS#39 multiply_with_log vs create_result_string");
    let mut rng = Rng::new(39);
    for _ in 0..1024 {
        let x = rng.i32_interesting();
        let y = rng.i32_interesting();
        let run = |a: &Api| -> Obs {
            let (ret, out) = capture(|| unsafe {
                let mut log: *mut c_char = std::ptr::null_mut();
                let prod = (a.multiply_with_log)(x, y, &mut log);
                let expect =
                    (a.create_result_string)(b"multiply\0".as_ptr() as *const c_char, prod);
                let cmp = (a.compare_operations)(log as *const c_char, expect as *const c_char);
                let lb = take_cstring(log);
                let eb = take_cstring(expect);
                (prod, cmp, lb, eb)
            });
            let (prod, cmp, lb, eb) = ret;
            assert_eq!(cmp, 0, "log string diverged from create_result_string");
            let mut aux = lb;
            aux.extend_from_slice(&eb);
            Obs {
                ret: ((prod as i64) << 32) ^ (cmp as u32 as i64),
                aux,
                out,
            }
        };
        d.check(format!("a={x} b={y}"), &run(c()), &run(r()));
    }
    d.finish();

    // row 40: cross-library buffer exchange — a heap string minted by one
    // library must be byte-identical and usable by the other's comparator.
    let mut d = Diffs::new("CONFIGS#40 cross-library buffer exchange");
    let mut rng = Rng::new(40);
    for _ in 0..1024 {
        let val = rng.i32_interesting();
        let lop = rng.range(0, 30) as usize;
        let op = rng.bytes(lop, ASCII);
        let ob = cstr(&op);

        let (cmp_via_c, cmp_via_r, cs, rs) = unsafe {
            let sc = (c().create_result_string)(ob.as_ptr() as *const c_char, val);
            let sr = (r().create_result_string)(ob.as_ptr() as *const c_char, val);
            let via_c = (c().compare_operations)(sc as *const c_char, sr as *const c_char);
            let via_r = (r().compare_operations)(sr as *const c_char, sc as *const c_char);
            let csb = take_cstring(sc);
            let rsb = take_cstring(sr);
            (via_c, via_r, csb, rsb)
        };
        assert_eq!(cs, rs, "heap strings differ for op={op:?} val={val}");
        assert_eq!(cmp_via_c, 0, "C comparator rejects the Rust buffer");
        assert_eq!(cmp_via_r, 0, "Rust comparator rejects the C buffer");
        d.check(
            format!("val={val}"),
            &Obs { ret: cmp_via_c as i64, aux: cs, out: Vec::new() },
            &Obs { ret: cmp_via_r as i64, aux: rs, out: Vec::new() },
        );
    }
    d.finish();
}

// ===========================================================================
// Subprocess isolation for cases that may legitimately crash or make a
// multi-gigabyte allocation. The parent runs the SAME case once per library in
// a child process and compares exit status plus every byte the child wrote.
// ===========================================================================

const CHILD_CASE_ENV: &str = "DIFF_CHILD_CASE";
const CHILD_LIB_ENV: &str = "DIFF_CHILD_LIB";
const CHILD_OUT_ENV: &str = "DIFF_CHILD_OUT";

/// Result of one isolated run.
#[derive(PartialEq, Eq)]
struct ChildOutcome {
    code: Option<i32>,
    signal: Option<i32>,
    bytes: Vec<u8>,
}

impl std::fmt::Debug for ChildOutcome {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "ChildOutcome {{ code: {:?}, signal: {:?}, out: {:?} }}",
            self.code,
            self.signal,
            String::from_utf8_lossy(&self.bytes)
        )
    }
}

fn run_isolated(case: &str, lib: &str) -> ChildOutcome {
    use std::os::unix::process::ExitStatusExt;

    let mut out = std::env::temp_dir();
    out.push(format!(
        "cmode-child-{}-{}-{}.txt",
        std::process::id(),
        case,
        lib
    ));
    let _ = std::fs::remove_file(&out);

    let exe = std::env::current_exe().expect("current_exe");
    let status = std::process::Command::new(exe)
        .args(["isolated_child_runner", "--exact", "--nocapture"])
        .env(CHILD_CASE_ENV, case)
        .env(CHILD_LIB_ENV, lib)
        .env(CHILD_OUT_ENV, &out)
        .env("RUST_BACKTRACE", "0")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .expect("spawn child");

    let bytes = std::fs::read(&out).unwrap_or_default();
    let _ = std::fs::remove_file(&out);
    ChildOutcome {
        code: status.code(),
        signal: status.signal(),
        bytes,
    }
}

/// Not a real assertion test: this is the body the isolated children execute.
/// Without `DIFF_CHILD_CASE` set it does nothing, so a normal `cargo test`
/// run just sees it pass.
#[test]
fn isolated_child_runner() {
    let Ok(case) = std::env::var(CHILD_CASE_ENV) else {
        return;
    };
    let which = std::env::var(CHILD_LIB_ENV).expect(CHILD_LIB_ENV);
    let outpath = std::env::var(CHILD_OUT_ENV).expect(CHILD_OUT_ENV);

    let api = match which.as_str() {
        "C" => c(),
        "RUST" => r(),
        other => panic!("bad {CHILD_LIB_ENV}: {other}"),
    };

    // Point fd 1 at the report file for the rest of this process's life, so the
    // library's printf output and our own markers land in the same stream.
    let file = std::fs::File::create(&outpath).expect("child report file");
    unsafe {
        fflush(std::ptr::null_mut());
        assert!(dup2(file.as_raw_fd(), 1) >= 0);
    }

    let report = |s: String| {
        let _ = std::io::stdout().write_all(s.as_bytes());
        let _ = std::io::stdout().flush();
        unsafe { fflush(std::ptr::null_mut()) };
    };

    match case.as_str() {
        // ERRORS row 9: log_msg is NULL and the C dereferences it unguarded.
        "multiply_with_log_null_out" => {
            unsafe { fflush(std::ptr::null_mut()) };
            let v = unsafe { (api.multiply_with_log)(6, 7, std::ptr::null_mut()) };
            report(format!("RESULT:{v}\n"));
        }
        // ERRORS row 13: positive count so large the allocation may fail or
        // may succeed and then over-read. Whatever happens must happen the
        // same way in both libraries.
        "copy_and_sum_huge_count" => {
            let mut src = [1i32, 2, 3, 4];
            unsafe { fflush(std::ptr::null_mut()) };
            let v = unsafe { (api.copy_and_sum)(src.as_mut_ptr(), i32::MAX) };
            report(format!("RESULT:{v}\n"));
        }
        "copy_and_sum_1g_count" => {
            let mut src = [1i32, 2, 3, 4];
            unsafe { fflush(std::ptr::null_mut()) };
            let v = unsafe { (api.copy_and_sum)(src.as_mut_ptr(), 1 << 30) };
            report(format!("RESULT:{v}\n"));
        }
        other => panic!("unknown isolated case `{other}`"),
    }

    unsafe { fflush(std::ptr::null_mut()) };
    let _ = std::io::stdout().flush();
    std::process::exit(0);
}

// ===========================================================================
// PHASE C — error-path differential tests, one per ERRORS.md row.
// ===========================================================================

/// ERRORS row 2 — NULL `op` reaches `snprintf`'s `%s` with no NULL check.
#[test]
fn err_create_result_string_null_op() {
    let mut d = Diffs::new("ERRORS#2 create_result_string NULL op");
    for v in [0, 1, -1, 42, i32::MAX, i32::MIN] {
        let co = obs_create_result_string(c(), None, v);
        let ro = obs_create_result_string(r(), None, v);
        d.check(format!("op=NULL val={v}"), &co, &ro);
        // The buffer is still returned (non-NULL) and glibc rendered "(null)".
        assert_eq!(co.ret, 1, "C returned NULL for a NULL op");
        assert!(
            String::from_utf8_lossy(&co.aux).contains("(null)"),
            "unexpected C rendering: {:?}",
            String::from_utf8_lossy(&co.aux)
        );
    }
    d.finish();
}

/// ERRORS row 3 — `op` longer than the 64-byte budget is truncated, not
/// rejected. Checks the exact truncation point.
#[test]
fn err_create_result_string_truncation() {
    let mut d = Diffs::new("ERRORS#3 create_result_string truncation");
    for len in [40usize, 41, 42, 43, 44, 45, 46, 63, 64, 65, 127, 128, 1000] {
        let op = vec![b'A'; len];
        let co = obs_create_result_string(c(), Some(&op), 123456);
        d.check(
            format!("len={len}"),
            &co,
            &obs_create_result_string(r(), Some(&op), 123456),
        );
        // snprintf(buf, 64, ...) writes at most 63 chars + NUL.
        assert!(
            co.aux.len() <= 64,
            "C wrote {} bytes into a 64-byte buffer",
            co.aux.len()
        );
    }
    d.finish();
}

/// ERRORS row 4 — `check_permissions` rejects when any required bit is absent.
#[test]
fn err_check_permissions_missing_bits() {
    let mut d = Diffs::new("ERRORS#4 check_permissions rejection");
    let mut rng = Rng::new(404);
    for _ in 0..2048 {
        let bit = 1i32 << (rng.range(0, 31) as u32);
        let perms = rng.i32() & !bit;
        let required = bit | (perms & rng.i32());
        let co = obs_check_permissions(c(), perms, required);
        d.check(
            format!("perms={perms:#x} required={required:#x}"),
            &co,
            &obs_check_permissions(r(), perms, required),
        );
        assert_eq!(co.ret, 0, "expected rejection for {perms:#x}/{required:#x}");
        assert!(co.out.is_empty(), "check_permissions must print nothing");
    }
    d.finish();
}

/// ERRORS rows 5–7 — `safe_add` rejects and returns `0` (not `-1`), and prints
/// the exact message.
#[test]
fn err_safe_add_insufficient_perms() {
    let mut d = Diffs::new("ERRORS#5-7 safe_add insufficient permissions");
    const MSG: &str = "Insufficient permissions for addition\n";
    // row 6: perms == 0; row 7: exactly one of the two bits; plus assorted.
    for perms in [
        0, 0o400, 0o200, 0o100, 0o500, 0o300, 0o644 & !0o400, 0o644 & !0o200, -1 & !0o400,
    ] {
        for (x, y) in [(1, 2), (i32::MAX, i32::MAX), (i32::MIN, -1), (0, 0)] {
            let co = obs_safe_add(c(), x, y, perms);
            d.check(
                format!("a={x} b={y} perms={perms:o}"),
                &co,
                &obs_safe_add(r(), x, y, perms),
            );
            assert_eq!(co.ret, 0, "rejection must return 0, got {}", co.ret);
            assert_eq!(String::from_utf8_lossy(&co.out), MSG, "wrong C message");
        }
    }
    // Positive control: `perms` with both bits must NOT print the message.
    let co = obs_safe_add(c(), 1, 2, 0o600);
    assert!(co.out.is_empty(), "accept path must be silent");
    assert_eq!(co.ret, 3);
    d.finish();
}

/// ERRORS row 10 — NULL `src`, checked before `count`.
#[test]
fn err_copy_and_sum_null_src() {
    let mut d = Diffs::new("ERRORS#10 copy_and_sum NULL src");
    const MSG: &str = "Source pointer is NULL\n";
    for count in [0, 1, 3, -1, i32::MAX, i32::MIN, 1 << 30] {
        let co = obs_copy_and_sum(c(), None, count);
        d.check(
            format!("src=NULL count={count}"),
            &co,
            &obs_copy_and_sum(r(), None, count),
        );
        assert_eq!(co.ret, -1, "NULL src must return -1");
        assert_eq!(String::from_utf8_lossy(&co.out), MSG);
    }
    d.finish();
}

/// ERRORS rows 11–12 — negative `count` becomes a huge `size_t`, so `malloc`
/// fails and the function reports an allocation failure.
#[test]
fn err_copy_and_sum_negative_count() {
    let mut d = Diffs::new("ERRORS#11-12 copy_and_sum negative count");
    const MSG: &str = "Memory allocation failed\n";
    let mut counts = vec![-1i32, -2, -3, -4, -1000, i32::MIN, i32::MIN + 1, -(1 << 30)];
    let mut rng = Rng::new(1112);
    for _ in 0..64 {
        counts.push(-(rng.range(1, 1_000_000) as i32));
    }
    for count in counts {
        let src = [7i32, 8, 9, 10];
        let co = obs_copy_and_sum(c(), Some(&src), count);
        d.check(
            format!("count={count}"),
            &co,
            &obs_copy_and_sum(r(), Some(&src), count),
        );
        assert_eq!(co.ret, -1, "negative count must return -1 (count={count})");
        assert_eq!(
            String::from_utf8_lossy(&co.out),
            MSG,
            "expected the allocation-failure message for count={count}"
        );
    }
    d.finish();
}

/// ERRORS row 14 — `count == 0` is NOT an error: `malloc(0)` succeeds and the
/// sum is 0.
#[test]
fn err_copy_and_sum_zero_count() {
    let mut d = Diffs::new("ERRORS#14 copy_and_sum count==0");
    for src in [&[][..], &[1, 2, 3][..], &[i32::MIN][..]] {
        let co = obs_copy_and_sum(c(), Some(src), 0);
        d.check(
            format!("src_len={}", src.len()),
            &co,
            &obs_copy_and_sum(r(), Some(src), 0),
        );
        assert_eq!(co.ret, 0, "count==0 must return 0, not an error");
        assert!(co.out.is_empty(), "count==0 must print nothing: {:?}", co.out);
    }
    d.finish();
}

/// Compare one isolated case across both libraries.
///
/// The large-allocation cases depend on how much memory the kernel is willing
/// to hand out, which can genuinely differ between two consecutive child runs
/// under memory pressure. A real translation divergence reproduces every time,
/// so retry a few times and only fail on a persistent mismatch.
fn assert_isolated_matches(case: &str) {
    let mut last = None;
    for attempt in 0..3 {
        let co = run_isolated(case, "C");
        let ro = run_isolated(case, "RUST");
        if co == ro {
            return;
        }
        last = Some((attempt, co, ro));
    }
    let (attempt, co, ro) = last.unwrap();
    panic!(
        "isolated case `{case}` diverged on every attempt (last = {attempt})\n  C   : {co:?}\n  RUST: {ro:?}"
    );
}

/// ERRORS row 13 — very large positive `count`. Run isolated: depending on
/// overcommit the allocation may fail (returning -1) or succeed and then
/// over-read. Either way both libraries must do the SAME thing.
///
/// Observed on this machine: `count == INT_MAX` (8 GiB) fails `malloc` and
/// returns -1 with `Memory allocation failed`, while `count == 1 << 30` (4 GiB)
/// succeeds and then faults in `memcpy` — identically in both libraries.
#[test]
fn err_copy_and_sum_huge_count() {
    assert_isolated_matches("copy_and_sum_huge_count");
    assert_isolated_matches("copy_and_sum_1g_count");
}

/// ERRORS rows 15–17 — either or both string pointers NULL.
#[test]
fn err_compare_operations_nulls() {
    let mut d = Diffs::new("ERRORS#15-17 compare_operations NULLs");
    const MSG: &str = "One or both operation strings are NULL\n";
    let some: &[u8] = b"addition";
    for (label, a, b) in [
        ("op1=NULL op2=valid", None, Some(some)),
        ("op1=valid op2=NULL", Some(some), None),
        ("both NULL", None, None),
        ("op1=NULL op2=empty", None, Some(&b""[..])),
        ("op1=empty op2=NULL", Some(&b""[..]), None),
    ] {
        let co = obs_compare_operations(c(), a, b);
        d.check(label, &co, &obs_compare_operations(r(), a, b));
        assert_eq!(co.ret, -1, "{label} must return -1");
        assert_eq!(String::from_utf8_lossy(&co.out), MSG, "{label} message");
    }
    // Positive control: two valid strings must not print.
    let co = obs_compare_operations(c(), Some(b"a"), Some(b"a"));
    assert!(co.out.is_empty());
    assert_eq!(co.ret, 0);
    d.finish();
}

/// ERRORS row 18 — the exact `strcmp` return value is observable, including
/// its magnitude, and `-1` is ambiguous with the NULL-rejection sentinel.
#[test]
fn err_compare_operations_nonzero_magnitude() {
    let mut d = Diffs::new("ERRORS#18 compare_operations strcmp magnitude");
    // Pairs engineered so strcmp's byte difference is exactly -1 / +1, which
    // collides with the error sentinel.
    for (a, b) in [
        (&b"a"[..], &b"b"[..]),
        (&b"b"[..], &b"a"[..]),
        (&b""[..], &b"\x01"[..]),
        (&b"\x01"[..], &b""[..]),
        (&b"\xfe"[..], &b"\xff"[..]),
        (&b"\xff"[..], &b"\xfe"[..]),
        (&b"A"[..], &b"a"[..]),
        (&b"zzz"[..], &b"zzy"[..]),
    ] {
        let co = obs_compare_operations(c(), Some(a), Some(b));
        d.check(
            format!("a={:?} b={:?}", String::from_utf8_lossy(a), String::from_utf8_lossy(b)),
            &co,
            &obs_compare_operations(r(), Some(a), Some(b)),
        );
        assert!(co.out.is_empty(), "valid strings must not print");
        assert_ne!(co.ret, 0, "these pairs must compare unequal");
    }
    d.finish();
}

/// ERRORS rows 20–23 — every out-of-range `mode`, including values that no
/// `case` label covers, negative values, and both int extremes. C `switch`
/// accepts any int, so this is the out-of-range-enum class of input.
#[test]
fn err_complexmode_invalid_mode() {
    let mut d = Diffs::new("ERRORS#20-23 complexmode invalid mode");
    const MSG: &str = "Invalid mode\n";

    let mut modes: Vec<i32> = vec![
        0,
        5,
        6,
        7,
        -1,
        -2,
        -4,
        i32::MAX,
        i32::MIN,
        i32::MAX - 1,
        i32::MIN + 1,
        1 << 30,
        -(1 << 30),
        // values whose low bits look like a valid mode but are not
        0x1_0001,
        0x7FFF_FFF4,
    ];
    let mut rng = Rng::new(2023);
    while modes.len() < 300 {
        let m = rng.i32();
        if !(1..=4).contains(&m) {
            modes.push(m);
        }
    }

    for mode in modes {
        for (a, b, cc) in [
            (0, 0, 0),
            (1, 2, 3),
            (i32::MAX, i32::MIN, -1),
            (rng.i32(), rng.i32(), rng.i32()),
        ] {
            let co = obs_complexmode(c(), mode, a, b, cc);
            d.check(
                format!("mode={mode} v=({a},{b},{cc})"),
                &co,
                &obs_complexmode(r(), mode, a, b, cc),
            );
            assert_eq!(co.ret, -1, "invalid mode {mode} must return -1");
            // `default:` never touches `operation`, so it stays "none" and the
            // trailing "Operation performed:" line is suppressed.
            assert_eq!(
                String::from_utf8_lossy(&co.out),
                MSG,
                "invalid mode {mode} must print exactly {MSG:?}"
            );
        }
    }
    d.finish();
}

/// ERRORS row 9 — `multiply_with_log` dereferences `log_msg` with no NULL
/// check. Both libraries must fail the same way; run isolated so the harness
/// survives.
#[test]
fn err_multiply_with_log_null_out() {
    assert_isolated_matches("multiply_with_log_null_out");
    // Non-vacuity: this case must actually fail (both sides), not quietly
    // return. If it ever starts exiting 0 the test has stopped testing.
    let co = run_isolated("multiply_with_log_null_out", "C");
    assert!(
        co.signal.is_some(),
        "expected the C library to fault on a NULL out-pointer, got {co:?}"
    );
}

/// ERRORS row 9 (companion) — a MISALIGNED out-pointer. The C does an unaligned
/// 8-byte store, which succeeds on x86-64; the Rust must not turn this into a
/// panic. Not isolated, because neither library is expected to fault.
#[test]
fn err_multiply_with_log_misaligned_out() {
    let mut d = Diffs::new("ERRORS#9b multiply_with_log misaligned out-pointer");
    for offset in [1usize, 2, 3, 5, 7] {
        for (x, y) in [(6, 7), (0, 0), (i32::MIN, -1)] {
            let run = |a: &Api| -> Obs {
                // 16 bytes of scratch so a pointer fits at any offset in 0..=8.
                let mut scratch = [0u8; 16];
                let slot = unsafe { scratch.as_mut_ptr().add(offset) } as *mut *mut c_char;
                let (ret, out) = capture(|| unsafe { (a.multiply_with_log)(x, y, slot) });
                let stored = unsafe { std::ptr::read_unaligned(slot) };
                let aux = unsafe { take_cstring(stored) };
                Obs { ret: ret as i64, aux, out }
            };
            d.check(
                format!("offset={offset} a={x} b={y}"),
                &run(c()),
                &run(r()),
            );
        }
    }
    d.finish();
}

// ===========================================================================
// Harness self-checks — prove the comparison machinery is not vacuous.
// ===========================================================================

/// If stdout capture silently returned empty buffers, every stdout comparison
/// above would pass for the wrong reason. Pin the exact expected bytes.
#[test]
fn harness_stdout_capture_is_real() {
    let (_, out) = capture(|| unsafe { (c().safe_add)(1, 2, 0) });
    assert_eq!(
        String::from_utf8_lossy(&out),
        "Insufficient permissions for addition\n",
        "stdout capture is not working"
    );

    let (_, out) = capture(|| unsafe { (c().complexmode)(3, 1, 2, 3) });
    assert_eq!(
        String::from_utf8_lossy(&out),
        "Mode 3: Array Sum\nResult: 6\nOperation performed: array_sum\n"
    );

    // Two successive captures must not leak into one another.
    let (_, out) = capture(|| unsafe { (c().check_permissions)(0o644, 0o600) });
    assert!(out.is_empty(), "capture leaked previous output: {out:?}");
}

/// The differential comparison must actually be able to fail. Feed `Diffs` a
/// known-divergent pair and confirm it reports it.
#[test]
fn harness_detects_divergence() {
    let mut d = Diffs::new("self-check");
    d.check(
        "deliberate",
        &Obs { ret: 1, aux: b"a".to_vec(), out: b"x".to_vec() },
        &Obs { ret: 2, aux: b"a".to_vec(), out: b"x".to_vec() },
    );
    assert_eq!(d.failures.len(), 1, "Diffs failed to notice a divergence");

    let mut d = Diffs::new("self-check-stdout");
    d.check(
        "deliberate",
        &Obs { ret: 1, aux: Vec::new(), out: b"x".to_vec() },
        &Obs { ret: 1, aux: Vec::new(), out: b"y".to_vec() },
    );
    assert_eq!(d.failures.len(), 1, "Diffs ignores stdout differences");

    // An empty row must be rejected as vacuous.
    let empty = Diffs::new("vacuous");
    assert!(
        std::panic::catch_unwind(move || empty.finish()).is_err(),
        "Diffs::finish accepted a zero-case row"
    );
}
