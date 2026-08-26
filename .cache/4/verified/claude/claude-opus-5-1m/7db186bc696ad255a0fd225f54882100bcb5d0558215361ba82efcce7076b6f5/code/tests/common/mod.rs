//! Differential-test harness.
//!
//! Both the C shared object (built by `c_src/CMakeLists.txt`) and the Rust
//! shared object (`cdylib`) are loaded with `libloading` and driven **only**
//! through their exported C symbols, exactly as an external consumer would.
//! Nothing in here calls a Rust function of the crate directly, so the
//! `#[no_mangle]`/`extern "C"` wrappers are part of what is under test.

#![allow(dead_code)]

use libloading::Library;
use std::ffi::c_void;
use std::os::raw::{c_char, c_int};
use std::path::PathBuf;
use std::sync::OnceLock;

// ---------------------------------------------------------------------------
// ABI mirrors
// ---------------------------------------------------------------------------

/// POSIX `regmatch_t` (glibc: two `int`s — verified 8 bytes on this platform).
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct RegMatch {
    pub rm_so: c_int,
    pub rm_eo: c_int,
}

impl RegMatch {
    pub const fn sentinel() -> Self {
        // Distinctive value so "written by regexec" vs "left alone" is visible.
        RegMatch {
            rm_so: 0x5A5A_5A5A,
            rm_eo: 0x5A5A_5A5A,
        }
    }
}

/// `os_data` from `include/lib.h` — nine `char *` in declaration order.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct OsData {
    pub fields: [*mut c_char; 9],
}

pub const OS_DATA_FIELD_NAMES: [&str; 9] = [
    "os_name",
    "os_version",
    "os_major",
    "os_minor",
    "os_codename",
    "os_platform",
    "os_build",
    "os_uname",
    "os_arch",
];

impl OsData {
    /// All nine members filled with the repeated byte `poison`.
    /// `0x00` gives the usual all-`NULL` struct; anything else proves that the
    /// implementation leaves non-participating members strictly alone.
    pub fn poisoned(poison: u8) -> Self {
        let word = usize::from_ne_bytes([poison; 8]);
        OsData {
            fields: [word as *mut c_char; 9],
        }
    }

    pub fn poison_ptr(poison: u8) -> *mut c_char {
        usize::from_ne_bytes([poison; 8]) as *mut c_char
    }
}

// ---------------------------------------------------------------------------
// libc bits the harness itself needs
// ---------------------------------------------------------------------------

extern "C" {
    fn free(ptr: *mut c_void);
    fn strlen(s: *const c_char) -> usize;
}

/// Copy a NUL-terminated C string into an owned byte vector (without the NUL).
pub unsafe fn cstr_bytes(p: *const c_char) -> Vec<u8> {
    let n = strlen(p);
    std::slice::from_raw_parts(p as *const u8, n).to_vec()
}

/// `Some(bytes)` for a real pointer, `None` for the poison/NULL value.
pub unsafe fn field_value(p: *mut c_char, poison: u8) -> Option<Vec<u8>> {
    if p == OsData::poison_ptr(poison) {
        None
    } else {
        Some(cstr_bytes(p))
    }
}

pub unsafe fn free_if_owned(p: *mut c_char, poison: u8) {
    if p != OsData::poison_ptr(poison) && !p.is_null() {
        free(p as *mut c_void);
    }
}

// ---------------------------------------------------------------------------
// The two implementations
// ---------------------------------------------------------------------------

pub type FnGetOsArch = unsafe extern "C" fn(*mut c_char) -> *mut c_char;
pub type FnWRegexec =
    unsafe extern "C" fn(*const c_char, *const c_char, usize, *mut RegMatch) -> c_int;
pub type FnParseUname = unsafe extern "C" fn(*mut c_char, *mut OsData);

pub struct Impl {
    pub name: &'static str,
    pub path: PathBuf,
    pub get_os_arch: FnGetOsArch,
    pub w_regexec: FnWRegexec,
    pub parse_uname_string: FnParseUname,
}

impl Impl {
    fn load(name: &'static str, path: PathBuf) -> Impl {
        assert!(
            path.exists(),
            "{name} shared object not found at {}\n\
             build the C side with:\n  \
             cd c_src && mkdir -p build && cd build && \
             cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .\n\
             build the Rust side with `cargo build` (same profile as the test).",
            path.display()
        );
        // Leaked on purpose: the resolved function pointers must stay valid for
        // the whole process lifetime.
        let lib: &'static Library = Box::leak(Box::new(unsafe {
            Library::new(&path).unwrap_or_else(|e| panic!("dlopen({}) failed: {e}", path.display()))
        }));
        unsafe {
            let get_os_arch = *lib
                .get::<FnGetOsArch>(b"get_os_arch\0")
                .unwrap_or_else(|e| panic!("{name}: missing symbol get_os_arch: {e}"));
            let w_regexec = *lib
                .get::<FnWRegexec>(b"w_regexec\0")
                .unwrap_or_else(|e| panic!("{name}: missing symbol w_regexec: {e}"));
            let parse_uname_string = *lib
                .get::<FnParseUname>(b"parse_uname_string\0")
                .unwrap_or_else(|e| panic!("{name}: missing symbol parse_uname_string: {e}"));
            Impl {
                name,
                path,
                get_os_arch,
                w_regexec,
                parse_uname_string,
            }
        }
    }
}

pub struct Pair {
    pub c: Impl,
    pub rs: Impl,
}

pub fn c_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("HARVEST_C_SO") {
        return PathBuf::from(p);
    }
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("c_src")
        .join("build")
        .join("libdriver.so")
}

/// `target/<profile>/libdriver.so`, derived from the test executable's own
/// location so that `cargo test` and `cargo test --release` both work.
pub fn rust_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("HARVEST_RUST_SO") {
        return PathBuf::from(p);
    }
    let exe = std::env::current_exe().expect("current_exe");
    // .../target/<profile>/deps/<test-bin>
    let profile_dir = exe
        .parent()
        .and_then(|p| p.parent())
        .expect("test exe layout");
    profile_dir.join("libdriver.so")
}

/// `cargo test --test <name>` does not necessarily materialise the `cdylib`
/// artifact (the integration test does not depend on it at link time), so build
/// it on demand for the profile the test itself was built with. Set
/// `HARVEST_NO_AUTOBUILD=1` to disable.
fn ensure_rust_so(path: &PathBuf) {
    if path.exists() || std::env::var_os("HARVEST_NO_AUTOBUILD").is_some() {
        return;
    }
    let release = path
        .parent()
        .and_then(|p| p.file_name())
        .map(|n| n == "release")
        .unwrap_or(false);
    let mut cmd = std::process::Command::new(std::env::var("CARGO").unwrap_or("cargo".into()));
    cmd.current_dir(env!("CARGO_MANIFEST_DIR"))
        .arg("build")
        .arg("--offline");
    if release {
        cmd.arg("--release");
    }
    let _ = cmd.status();
}

/// `c_src/build/libdriver.so`, built on demand with cmake.
fn ensure_c_so(path: &PathBuf) {
    if path.exists() || std::env::var_os("HARVEST_NO_AUTOBUILD").is_some() {
        return;
    }
    let c_src = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("c_src");
    let build = c_src.join("build");
    let _ = std::fs::create_dir_all(&build);
    let _ = std::process::Command::new("cmake")
        .current_dir(&build)
        .args(["..", "-DCMAKE_POSITION_INDEPENDENT_CODE=ON"])
        .status();
    let _ = std::process::Command::new("cmake")
        .current_dir(&build)
        .args(["--build", "."])
        .status();
}

pub fn pair() -> &'static Pair {
    static PAIR: OnceLock<Pair> = OnceLock::new();
    PAIR.get_or_init(|| {
        let c_path = c_so_path();
        let rs_path = rust_so_path();
        ensure_c_so(&c_path);
        ensure_rust_so(&rs_path);
        Pair {
            c: Impl::load("C", c_path),
            rs: Impl::load("Rust", rs_path),
        }
    })
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (splitmix64) — property-style testing with a fixed seed
// ---------------------------------------------------------------------------

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng(seed ^ 0x9E37_79B9_7F4A_7C15)
    }
    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    /// Uniform in `0..n` (`n > 0`).
    pub fn below(&mut self, n: usize) -> usize {
        (self.next_u64() % n as u64) as usize
    }
    pub fn range(&mut self, lo: usize, hi_inclusive: usize) -> usize {
        lo + self.below(hi_inclusive - lo + 1)
    }
    pub fn pick<'a, T>(&mut self, xs: &'a [T]) -> &'a T {
        &xs[self.below(xs.len())]
    }
    pub fn chance(&mut self, one_in: usize) -> bool {
        self.below(one_in) == 0
    }
}

// ---------------------------------------------------------------------------
// Guard-padded, NUL-terminated buffer
// ---------------------------------------------------------------------------

pub const GUARD: u8 = 0xCC;
pub const GUARD_LEN: usize = 32;

/// A NUL-terminated copy of `payload` surrounded by `GUARD_LEN` guard bytes on
/// both sides. The pointer handed to the library points at the payload, so the
/// C code's `*(str_tmp + strlen(str_tmp) - 1) = '\0'` write that lands *before*
/// the start of a zero-length substring is captured by the leading guard and
/// compared byte-for-byte between the two implementations.
pub struct Buf {
    pub raw: Vec<u8>,
}

impl Buf {
    pub fn new(payload: &[u8]) -> Buf {
        assert!(
            !payload.contains(&0),
            "payload must not contain an interior NUL"
        );
        let mut raw = vec![GUARD; GUARD_LEN];
        raw.extend_from_slice(payload);
        raw.push(0);
        raw.extend(std::iter::repeat(GUARD).take(GUARD_LEN));
        Buf { raw }
    }
    pub fn ptr(&mut self) -> *mut c_char {
        unsafe { self.raw.as_mut_ptr().add(GUARD_LEN) as *mut c_char }
    }
}

fn hex(bytes: &[u8]) -> String {
    let mut s = String::new();
    for b in bytes {
        s.push_str(&format!("{b:02x}"));
    }
    s
}

fn show(bytes: &[u8]) -> String {
    let mut s = String::from("\"");
    for &b in bytes {
        match b {
            b'"' => s.push_str("\\\""),
            b'\\' => s.push_str("\\\\"),
            0x20..=0x7e => s.push(b as char),
            0 => s.push_str("\\0"),
            _ => s.push_str(&format!("\\x{b:02x}")),
        }
    }
    s.push('"');
    s
}

fn show_opt(v: &Option<Vec<u8>>) -> String {
    match v {
        None => "<untouched>".to_string(),
        Some(b) => show(b),
    }
}

// ---------------------------------------------------------------------------
// Differential drivers — one per exported entry point
// ---------------------------------------------------------------------------

/// `get_os_arch`: compare the returned string (or the NULL sentinel) and the
/// caller's buffer (which must not be modified).
pub fn diff_get_os_arch(ctx: &str, header: &[u8]) {
    let p = pair();
    let mut cb = Buf::new(header);
    let mut rb = Buf::new(header);

    let (c_res, r_res) = unsafe {
        let cp = p.c.get_os_arch.clone()(cb.ptr());
        let rp = p.rs.get_os_arch.clone()(rb.ptr());
        let c_res = if cp.is_null() {
            None
        } else {
            Some(cstr_bytes(cp))
        };
        let r_res = if rp.is_null() {
            None
        } else {
            Some(cstr_bytes(rp))
        };
        if !cp.is_null() {
            free(cp as *mut c_void);
        }
        if !rp.is_null() {
            free(rp as *mut c_void);
        }
        (c_res, r_res)
    };

    assert_eq!(
        c_res,
        r_res,
        "get_os_arch return mismatch [{ctx}]\n  input: {}\n  C   : {}\n  Rust: {}",
        show(header),
        show_opt(&c_res),
        show_opt(&r_res)
    );
    assert_eq!(
        hex(&cb.raw),
        hex(&rb.raw),
        "get_os_arch buffer mismatch [{ctx}] input {}",
        show(header)
    );
}

/// `w_regexec`: compare the return value and the *entire* `pmatch` buffer,
/// including the entries glibc may or may not overwrite.
pub fn diff_w_regexec(
    ctx: &str,
    pattern: Option<&[u8]>,
    subject: Option<&[u8]>,
    nmatch: usize,
    pmatch_slots: Option<usize>,
) {
    let p = pair();

    let mut cpat = pattern.map(Buf::new);
    let mut rpat = pattern.map(Buf::new);
    let mut csub = subject.map(Buf::new);
    let mut rsub = subject.map(Buf::new);

    let slots = pmatch_slots.unwrap_or(0);
    let mut c_m = vec![RegMatch::sentinel(); slots];
    let mut r_m = vec![RegMatch::sentinel(); slots];

    let (c_ret, r_ret) = unsafe {
        let cpp = cpat
            .as_mut()
            .map(|b| b.ptr() as *const c_char)
            .unwrap_or(std::ptr::null());
        let rpp = rpat
            .as_mut()
            .map(|b| b.ptr() as *const c_char)
            .unwrap_or(std::ptr::null());
        let csp = csub
            .as_mut()
            .map(|b| b.ptr() as *const c_char)
            .unwrap_or(std::ptr::null());
        let rsp = rsub
            .as_mut()
            .map(|b| b.ptr() as *const c_char)
            .unwrap_or(std::ptr::null());
        let cmp = if pmatch_slots.is_none() {
            std::ptr::null_mut()
        } else {
            c_m.as_mut_ptr()
        };
        let rmp = if pmatch_slots.is_none() {
            std::ptr::null_mut()
        } else {
            r_m.as_mut_ptr()
        };
        (
            p.c.w_regexec.clone()(cpp, csp, nmatch, cmp),
            p.rs.w_regexec.clone()(rpp, rsp, nmatch, rmp),
        )
    };

    let desc = format!(
        "[{ctx}] pattern={} subject={} nmatch={nmatch} slots={pmatch_slots:?}",
        pattern.map(show).unwrap_or_else(|| "NULL".into()),
        subject.map(show).unwrap_or_else(|| "NULL".into())
    );
    assert_eq!(
        c_ret, r_ret,
        "w_regexec return mismatch {desc}\n  C={c_ret} Rust={r_ret}"
    );
    assert_eq!(
        c_m, r_m,
        "w_regexec pmatch mismatch {desc}\n  C   ={c_m:?}\n  Rust={r_m:?}"
    );
    // The subject/pattern buffers are read-only for this function.
    if let (Some(a), Some(b)) = (&cpat, &rpat) {
        assert_eq!(hex(&a.raw), hex(&b.raw), "w_regexec pattern buffer {desc}");
    }
    if let (Some(a), Some(b)) = (&csub, &rsub) {
        assert_eq!(hex(&a.raw), hex(&b.raw), "w_regexec subject buffer {desc}");
    }
}

/// `parse_uname_string`: compare all nine `os_data` members (untouched-vs-set
/// *and* contents) plus every byte of the guard-padded `uname` buffer.
pub fn diff_parse_uname(ctx: &str, uname: &[u8], poison: u8) {
    let p = pair();
    let mut cb = Buf::new(uname);
    let mut rb = Buf::new(uname);
    let mut c_osd = OsData::poisoned(poison);
    let mut r_osd = OsData::poisoned(poison);

    unsafe {
        p.c.parse_uname_string.clone()(cb.ptr(), &mut c_osd);
        p.rs.parse_uname_string.clone()(rb.ptr(), &mut r_osd);
    }

    let mut mismatches = Vec::new();
    for i in 0..9 {
        let (cv, rv) = unsafe {
            (
                field_value(c_osd.fields[i], poison),
                field_value(r_osd.fields[i], poison),
            )
        };
        if cv != rv {
            mismatches.push(format!(
                "    {}: C={} Rust={}",
                OS_DATA_FIELD_NAMES[i],
                show_opt(&cv),
                show_opt(&rv)
            ));
        }
    }
    let buf_ok = cb.raw == rb.raw;

    unsafe {
        for i in 0..9 {
            free_if_owned(c_osd.fields[i], poison);
            free_if_owned(r_osd.fields[i], poison);
        }
    }

    assert!(
        mismatches.is_empty(),
        "parse_uname_string os_data mismatch [{ctx}] poison=0x{poison:02x}\n  \
         input: {}\n{}",
        show(uname),
        mismatches.join("\n")
    );
    assert!(
        buf_ok,
        "parse_uname_string mutated the uname buffer differently [{ctx}]\n  \
         input: {}\n  C   : {}\n  Rust: {}",
        show(uname),
        hex(&cb.raw),
        hex(&rb.raw)
    );
}

/// Same as [`diff_parse_uname`] but with a NULL `os_data *`.
pub fn diff_parse_uname_null_osd(ctx: &str, uname: Option<&[u8]>) {
    let p = pair();
    let mut cb = uname.map(Buf::new);
    let mut rb = uname.map(Buf::new);
    unsafe {
        let cp = cb
            .as_mut()
            .map(|b| b.ptr())
            .unwrap_or(std::ptr::null_mut());
        let rp = rb
            .as_mut()
            .map(|b| b.ptr())
            .unwrap_or(std::ptr::null_mut());
        p.c.parse_uname_string.clone()(cp, std::ptr::null_mut());
        p.rs.parse_uname_string.clone()(rp, std::ptr::null_mut());
    }
    if let (Some(a), Some(b)) = (&cb, &rb) {
        assert_eq!(
            hex(&a.raw),
            hex(&b.raw),
            "parse_uname_string(NULL osd) buffer mismatch [{ctx}]"
        );
        // The C returns before touching anything.
        let expected = Buf::new(uname.unwrap());
        assert_eq!(
            hex(&a.raw),
            hex(&expected.raw),
            "parse_uname_string(NULL osd) must not modify the buffer [{ctx}]"
        );
    }
}

/// Composed pipeline: run `parse_uname_string`, then feed the *already
/// mutated* buffer to `get_os_arch` and the produced `os_version` to
/// `w_regexec`, comparing every intermediate result. This exercises state
/// carried across entry points, which per-function tests cannot see.
pub fn diff_pipeline(ctx: &str, uname: &[u8], poison: u8) {
    let p = pair();
    let mut cb = Buf::new(uname);
    let mut rb = Buf::new(uname);
    let mut c_osd = OsData::poisoned(poison);
    let mut r_osd = OsData::poisoned(poison);

    let mut problems: Vec<String> = Vec::new();

    unsafe {
        p.c.parse_uname_string.clone()(cb.ptr(), &mut c_osd);
        p.rs.parse_uname_string.clone()(rb.ptr(), &mut r_osd);

        for i in 0..9 {
            let cv = field_value(c_osd.fields[i], poison);
            let rv = field_value(r_osd.fields[i], poison);
            if cv != rv {
                problems.push(format!(
                    "    stage1 {}: C={} Rust={}",
                    OS_DATA_FIELD_NAMES[i],
                    show_opt(&cv),
                    show_opt(&rv)
                ));
            }
        }
        if cb.raw != rb.raw {
            problems.push("    stage1 uname buffer differs".to_string());
        }

        // stage 2: get_os_arch over the mutated buffer
        let ca = p.c.get_os_arch.clone()(cb.ptr());
        let ra = p.rs.get_os_arch.clone()(rb.ptr());
        let cav = if ca.is_null() {
            None
        } else {
            Some(cstr_bytes(ca))
        };
        let rav = if ra.is_null() {
            None
        } else {
            Some(cstr_bytes(ra))
        };
        if cav != rav {
            problems.push(format!(
                "    stage2 get_os_arch(mutated): C={} Rust={}",
                show_opt(&cav),
                show_opt(&rav)
            ));
        }
        if !ca.is_null() {
            free(ca as *mut c_void);
        }
        if !ra.is_null() {
            free(ra as *mut c_void);
        }

        // stage 3: w_regexec over the produced os_version (index 1)
        let cver = c_osd.fields[1];
        let rver = r_osd.fields[1];
        if cver != OsData::poison_ptr(poison) && rver != OsData::poison_ptr(poison) {
            let pat = b"^([0-9]+)\\.([0-9]+)\\.*\0";
            let mut cm = vec![RegMatch::sentinel(); 4];
            let mut rm = vec![RegMatch::sentinel(); 4];
            let cr = p.c.w_regexec.clone()(pat.as_ptr() as *const c_char, cver, 3, cm.as_mut_ptr());
            let rr =
                p.rs.w_regexec.clone()(pat.as_ptr() as *const c_char, rver, 3, rm.as_mut_ptr());
            if cr != rr || cm != rm {
                problems.push(format!(
                    "    stage3 w_regexec(os_version): C=({cr},{cm:?}) Rust=({rr},{rm:?})"
                ));
            }
        }

        for i in 0..9 {
            free_if_owned(c_osd.fields[i], poison);
            free_if_owned(r_osd.fields[i], poison);
        }
    }

    assert!(
        problems.is_empty(),
        "pipeline mismatch [{ctx}]\n  input: {}\n{}",
        show(uname),
        problems.join("\n")
    );
}

// ---------------------------------------------------------------------------
// stderr capture (for the `regcomp` failure diagnostic, ERRORS.md row 4-8)
// ---------------------------------------------------------------------------

extern "C" {
    fn dup(oldfd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn open(path: *const c_char, flags: c_int, ...) -> c_int;
}

const O_RDWR: c_int = 0o2;
const O_CREAT: c_int = 0o100;
const O_TRUNC: c_int = 0o1000;

/// Serialises [`capture_stderr`]: fd 2 is process-global.
static STDERR_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// Run `f` with fd 2 redirected to a temporary file and return what was
/// written. `stderr` is unbuffered in C, so no explicit flush is needed.
///
/// fd 2 is process-global, so callers must not run concurrently with anything
/// else that writes to stderr — keep such tests in their own test binary. The
/// mutex above serialises concurrent callers *within* that binary.
pub fn capture_stderr<F: FnOnce()>(tag: &str, f: F) -> Vec<u8> {
    let _guard = STDERR_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let dir = std::env::var("TMPDIR").unwrap_or_else(|_| "/tmp".to_string());
    let path = format!("{dir}/harvest-stderr-{}-{}.txt", std::process::id(), tag);
    let cpath = std::ffi::CString::new(path.clone()).unwrap();
    unsafe {
        let saved = dup(2);
        assert!(saved >= 0, "dup(2) failed");
        let fd = open(cpath.as_ptr(), O_RDWR | O_CREAT | O_TRUNC, 0o600 as c_int);
        assert!(fd >= 0, "open({path}) failed");
        assert!(dup2(fd, 2) >= 0, "dup2 failed");
        f();
        dup2(saved, 2);
        close(fd);
        close(saved);
    }
    let out = std::fs::read(&path).unwrap_or_default();
    let _ = std::fs::remove_file(&path);
    out
}

// ---------------------------------------------------------------------------
// crash parity (ERRORS.md rows 38-39): run the call in a forked child and
// compare how the child died.
// ---------------------------------------------------------------------------

extern "C" {
    fn fork() -> c_int;
    fn waitpid(pid: c_int, status: *mut c_int, options: c_int) -> c_int;
    fn _exit(code: c_int) -> !;
}

/// How a forked child terminated.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum Death {
    Exited(c_int),
    Signalled(c_int),
    Other(c_int),
}

fn decode(status: c_int) -> Death {
    // WIFEXITED / WEXITSTATUS / WIFSIGNALED / WTERMSIG
    if status & 0x7f == 0x7f {
        Death::Other(status)
    } else if status & 0x7f == 0 {
        Death::Exited((status >> 8) & 0xff)
    } else {
        Death::Signalled(status & 0x7f)
    }
}

/// Serialises [`run_in_child`]: `fork()` from a multi-threaded process is only
/// safe if the child does the bare minimum, so keep one at a time.
static FORK_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// Run `f` in a forked child; the child `_exit(0)`s if `f` returns normally.
/// Returns how the child terminated.
pub fn run_in_child<F: FnOnce()>(f: F) -> Death {
    let _guard = FORK_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    unsafe {
        let pid = fork();
        assert!(pid >= 0, "fork failed");
        if pid == 0 {
            f();
            _exit(0);
        }
        let mut status: c_int = 0;
        let r = waitpid(pid, &mut status, 0);
        assert_eq!(r, pid, "waitpid failed");
        decode(status)
    }
}

// ---------------------------------------------------------------------------
// Shared data
// ---------------------------------------------------------------------------

/// The `ARCHS` table from `c_src/src/lib.c:18`, in source order.
pub const ARCHS: [&str; 12] = [
    "x86_64", "i386", "i686", "sparc", "amd64", "i86pc", "ia64", "AIX", "armv6", "armv7",
    "aarch64", "arm64",
];

/// The five regular expressions `parse_uname_string` actually compiles.
pub const PATTERNS: [&str; 5] = [
    r"^([0-9]+)\.*",
    r"^[0-9]+\.([0-9]+)\.*",
    r"^[0-9]+\.[0-9]+\.([0-9]+(\.[0-9]+)*)\.*",
    r"^([0-9]+)\.*",
    r"^[0-9]+\.([0-9]+)\.*",
];
