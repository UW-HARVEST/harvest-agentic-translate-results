//! Shared differential-test harness.
//!
//! Both the C `.so` and the Rust `.so` are loaded with `libloading`; every call
//! goes through the dynamic symbols, so the `#[no_mangle]` export wrappers are
//! part of what is under test. Rust functions are never called directly.

#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::os::raw::{c_char, c_int};
use std::path::PathBuf;
use std::sync::OnceLock;

// ---------------------------------------------------------------------------
// ABI mirrors
// ---------------------------------------------------------------------------

#[repr(C)]
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct RegMatch {
    pub rm_so: c_int,
    pub rm_eo: c_int,
}

/// Mirror of `os_data` from `c_src/include/lib.h`.
#[repr(C)]
#[derive(Copy, Clone)]
pub struct OsData {
    pub os_name: *mut c_char,
    pub os_version: *mut c_char,
    pub os_major: *mut c_char,
    pub os_minor: *mut c_char,
    pub os_codename: *mut c_char,
    pub os_platform: *mut c_char,
    pub os_build: *mut c_char,
    pub os_uname: *mut c_char,
    pub os_arch: *mut c_char,
}

pub const OS_DATA_FIELDS: [&str; 9] = [
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
    pub fn zeroed() -> Self {
        OsData {
            os_name: std::ptr::null_mut(),
            os_version: std::ptr::null_mut(),
            os_major: std::ptr::null_mut(),
            os_minor: std::ptr::null_mut(),
            os_codename: std::ptr::null_mut(),
            os_platform: std::ptr::null_mut(),
            os_build: std::ptr::null_mut(),
            os_uname: std::ptr::null_mut(),
            os_arch: std::ptr::null_mut(),
        }
    }

    /// Fill every field with a distinct non-NULL sentinel so that "field left
    /// untouched" can be told apart from "field set to NULL".
    pub fn poisoned(sentinels: &[*mut c_char; 9]) -> Self {
        OsData {
            os_name: sentinels[0],
            os_version: sentinels[1],
            os_major: sentinels[2],
            os_minor: sentinels[3],
            os_codename: sentinels[4],
            os_platform: sentinels[5],
            os_build: sentinels[6],
            os_uname: sentinels[7],
            os_arch: sentinels[8],
        }
    }

    pub fn as_array(&self) -> [*mut c_char; 9] {
        [
            self.os_name,
            self.os_version,
            self.os_major,
            self.os_minor,
            self.os_codename,
            self.os_platform,
            self.os_build,
            self.os_uname,
            self.os_arch,
        ]
    }
}

/// A field observation that can be compared between the two libraries:
/// either "still the sentinel we put there", "NULL", or an owned byte string.
#[derive(PartialEq, Eq, Debug, Clone)]
pub enum Field {
    Null,
    Untouched,
    Bytes(Vec<u8>),
}

// ---------------------------------------------------------------------------
// Library loading
// ---------------------------------------------------------------------------

pub struct Lib {
    _lib: Library,
    pub parse_uname_string: unsafe extern "C" fn(*mut c_char, *mut OsData),
    pub get_os_arch: unsafe extern "C" fn(*mut c_char) -> *mut c_char,
    pub w_regexec:
        unsafe extern "C" fn(*const c_char, *const c_char, usize, *mut RegMatch) -> c_int,
}

impl Lib {
    unsafe fn open(path: &PathBuf) -> Lib {
        let lib = Library::new(path).unwrap_or_else(|e| panic!("dlopen {:?}: {}", path, e));
        let parse: Symbol<unsafe extern "C" fn(*mut c_char, *mut OsData)> = lib
            .get(b"parse_uname_string\0")
            .unwrap_or_else(|e| panic!("parse_uname_string in {:?}: {}", path, e));
        let arch: Symbol<unsafe extern "C" fn(*mut c_char) -> *mut c_char> = lib
            .get(b"get_os_arch\0")
            .unwrap_or_else(|e| panic!("get_os_arch in {:?}: {}", path, e));
        let rex: Symbol<
            unsafe extern "C" fn(*const c_char, *const c_char, usize, *mut RegMatch) -> c_int,
        > = lib
            .get(b"w_regexec\0")
            .unwrap_or_else(|e| panic!("w_regexec in {:?}: {}", path, e));
        let out = Lib {
            parse_uname_string: *parse,
            get_os_arch: *arch,
            w_regexec: *rex,
            _lib: lib,
        };
        out
    }
}

pub struct Pair {
    pub c: Lib,
    pub rs: Lib,
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// `target/<profile>/libdriver.so` — derived from the running test binary
/// (`target/<profile>/deps/<test>-<hash>`), so it works for debug and release.
fn rust_so() -> PathBuf {
    let exe = std::env::current_exe().expect("current_exe");
    let profile_dir = exe
        .parent()
        .and_then(|p| p.parent())
        .expect("target/<profile>")
        .to_path_buf();
    let candidate = profile_dir.join("libdriver.so");
    if candidate.exists() {
        return candidate;
    }
    for p in ["release", "debug"] {
        let alt = manifest_dir().join("target").join(p).join("libdriver.so");
        if alt.exists() {
            return alt;
        }
    }
    panic!("could not locate the Rust libdriver.so (looked in {:?})", profile_dir);
}

fn c_so() -> PathBuf {
    let root = manifest_dir().parent().expect("workspace root").to_path_buf();
    let p = root.join("c_src/build/libdriver.so");
    assert!(
        p.exists(),
        "C shared library not built: {:?}\nrun: cd c_src && mkdir -p build && cd build \
         && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
        p
    );
    p
}

/// Guard against silently testing a stale `.so`: the artifact must be at least
/// as new as the source it was built from.
fn assert_fresh(so: &PathBuf, src: &PathBuf) {
    let m = |p: &PathBuf| {
        std::fs::metadata(p)
            .and_then(|md| md.modified())
            .unwrap_or_else(|e| panic!("stat {:?}: {}", p, e))
    };
    assert!(
        m(so) >= m(src),
        "{:?} is older than {:?} — rebuild before testing",
        so,
        src
    );
}

static PAIR: OnceLock<Pair> = OnceLock::new();

pub fn libs() -> &'static Pair {
    PAIR.get_or_init(|| unsafe {
        let cp = c_so();
        let rp = rust_so();
        let root = manifest_dir().parent().unwrap().to_path_buf();
        assert_fresh(&cp, &root.join("c_src/src/lib.c"));
        assert_fresh(&rp, &manifest_dir().join("src/lib.rs"));
        eprintln!("[harness] C  .so: {:?}", cp);
        eprintln!("[harness] Rust .so: {:?}", rp);
        Pair {
            c: Lib::open(&cp),
            rs: Lib::open(&rp),
        }
    })
}

// ---------------------------------------------------------------------------
// libc bits used by the harness itself
// ---------------------------------------------------------------------------

extern "C" {
    fn malloc(n: usize) -> *mut std::os::raw::c_void;
    fn free(p: *mut std::os::raw::c_void);
    fn strlen(s: *const c_char) -> usize;
    fn memcpy(
        d: *mut std::os::raw::c_void,
        s: *const std::os::raw::c_void,
        n: usize,
    ) -> *mut std::os::raw::c_void;
}

/// A NUL-terminated, `malloc`-backed, mutable copy of `bytes` with 32 bytes of
/// slack on both sides.
///
/// The slack matters: `parse_uname_string` reproduces the C code's
/// `*(p + strlen(p) - 1) = '\0'` which writes one byte *before* the string when
/// the string is empty. Keeping that write inside our own allocation makes the
/// tests deterministic instead of scribbling on a malloc chunk header.
pub struct CBuf {
    base: *mut c_char,
    off: usize,
    len: usize,
}

const SLACK: usize = 32;

impl CBuf {
    pub fn new(bytes: &[u8]) -> CBuf {
        let total = bytes.len() + 1 + 2 * SLACK;
        unsafe {
            let base = malloc(total) as *mut c_char;
            assert!(!base.is_null());
            std::ptr::write_bytes(base as *mut u8, 0xAA, total);
            let p = base.add(SLACK);
            if !bytes.is_empty() {
                memcpy(
                    p as *mut std::os::raw::c_void,
                    bytes.as_ptr() as *const std::os::raw::c_void,
                    bytes.len(),
                );
            }
            *p.add(bytes.len()) = 0;
            CBuf {
                base,
                off: SLACK,
                len: bytes.len(),
            }
        }
    }

    pub fn ptr(&self) -> *mut c_char {
        unsafe { self.base.add(self.off) }
    }

    /// The whole allocation including the slack, so out-of-bounds writes are
    /// part of the compared output.
    pub fn raw(&self) -> Vec<u8> {
        unsafe {
            std::slice::from_raw_parts(self.base as *const u8, self.len + 1 + 2 * SLACK).to_vec()
        }
    }
}

impl Drop for CBuf {
    fn drop(&mut self) {
        unsafe { free(self.base as *mut std::os::raw::c_void) }
    }
}

pub unsafe fn cstr_bytes(p: *const c_char) -> Vec<u8> {
    let n = strlen(p);
    std::slice::from_raw_parts(p as *const u8, n).to_vec()
}

// ---------------------------------------------------------------------------
// Differential drivers
// ---------------------------------------------------------------------------

#[derive(PartialEq, Eq, Debug, Clone)]
pub struct ParseOutcome {
    pub fields: Vec<Field>,
    pub buffer: Vec<u8>,
}

/// Nine distinct sentinel pointers, one per `os_data` field. They are never
/// dereferenced; only compared by identity so we can detect untouched fields.
fn sentinels() -> &'static [*mut c_char; 9] {
    // Values chosen to be obviously invalid but aligned; the library only ever
    // overwrites them, never reads them.
    static S: OnceLock<[usize; 9]> = OnceLock::new();
    let raw = S.get_or_init(|| {
        let mut a = [0usize; 9];
        for (i, slot) in a.iter_mut().enumerate() {
            *slot = 0xDEAD_0000_0000_0000usize | ((i as usize + 1) << 8);
        }
        a
    });
    // SAFETY: layout-compatible reinterpretation of 9 usizes as 9 raw pointers.
    unsafe { &*(raw as *const [usize; 9] as *const [*mut c_char; 9]) }
}

pub fn run_parse(lib: &Lib, input: &[u8], poison: bool) -> ParseOutcome {
    let buf = CBuf::new(input);
    let sent = sentinels();
    let mut osd = if poison {
        OsData::poisoned(sent)
    } else {
        OsData::zeroed()
    };
    unsafe {
        (lib.parse_uname_string)(buf.ptr(), &mut osd);
    }
    let arr = osd.as_array();
    let mut fields = Vec::with_capacity(9);
    for (i, p) in arr.iter().enumerate() {
        if p.is_null() {
            fields.push(Field::Null);
        } else if *p == sent[i] {
            fields.push(Field::Untouched);
        } else {
            let b = unsafe { cstr_bytes(*p) };
            fields.push(Field::Bytes(b));
            // Intentionally leaked: several C paths perform the documented
            // `*(p-1) = 0` write on these allocations, so handing them back to
            // free() would be reasoning about corrupted chunk headers. Test
            // inputs are small and bounded.
        }
    }
    ParseOutcome {
        fields,
        buffer: buf.raw(),
    }
}

/// Assert the two `.so`s agree on `parse_uname_string` for `input`.
pub fn diff_parse(input: &[u8], poison: bool, ctx: &str) {
    let l = libs();
    let a = run_parse(&l.c, input, poison);
    let b = run_parse(&l.rs, input, poison);
    if a != b {
        let mut msg = format!(
            "parse_uname_string divergence [{}]\n  input  = {:?}\n  poison = {}\n",
            ctx,
            String::from_utf8_lossy(input),
            poison
        );
        for (i, name) in OS_DATA_FIELDS.iter().enumerate() {
            if a.fields[i] != b.fields[i] {
                msg += &format!(
                    "  {:<12} C={:?}\n  {:<12} R={:?}\n",
                    name, a.fields[i], "", b.fields[i]
                );
            }
        }
        if a.buffer != b.buffer {
            msg += &format!(
                "  buffer C={:?}\n  buffer R={:?}\n",
                String::from_utf8_lossy(&a.buffer),
                String::from_utf8_lossy(&b.buffer)
            );
        }
        panic!("{}", msg);
    }
}

/// Assert the two `.so`s agree on `get_os_arch` for `input`.
pub fn diff_arch(input: &[u8], ctx: &str) {
    let l = libs();
    let run = |lib: &Lib| -> Option<Vec<u8>> {
        let buf = CBuf::new(input);
        unsafe {
            let p = (lib.get_os_arch)(buf.ptr());
            if p.is_null() {
                None
            } else {
                let v = cstr_bytes(p);
                free(p as *mut std::os::raw::c_void);
                Some(v)
            }
        }
    };
    let a = run(&l.c);
    let b = run(&l.rs);
    assert_eq!(
        a,
        b,
        "get_os_arch divergence [{}] input={:?}",
        ctx,
        String::from_utf8_lossy(input)
    );
}

/// Assert the two `.so`s agree on `w_regexec`, including the `pmatch` slots.
///
/// `pattern`/`subject` are `Some(bytes)` (NUL appended by the harness) or
/// `None` for a genuine NULL pointer. `slots` is the size of the caller's
/// `pmatch` buffer; `nmatch` is what is passed to the function, so
/// `nmatch > slots` is expressible only via `nmatch_override`.
pub fn diff_regexec(
    pattern: Option<&[u8]>,
    subject: Option<&[u8]>,
    nmatch: usize,
    slots: usize,
    pmatch_null: bool,
    ctx: &str,
) {
    assert!(
        !pmatch_null || nmatch == 0,
        "regexec(nmatch>0, pmatch=NULL) is a caller bug, not a library behaviour"
    );
    assert!(
        pmatch_null || nmatch <= slots,
        "nmatch must not exceed the caller's pmatch buffer (glibc would write OOB)"
    );
    let l = libs();
    const POISON: RegMatch = RegMatch {
        rm_so: -424242,
        rm_eo: -434343,
    };
    let run = |lib: &Lib| -> (c_int, Vec<RegMatch>) {
        let pbuf = pattern.map(CBuf::new);
        let sbuf = subject.map(CBuf::new);
        let mut m = vec![POISON; slots.max(1)];
        let mp = if pmatch_null {
            std::ptr::null_mut()
        } else {
            m.as_mut_ptr()
        };
        let r = unsafe {
            (lib.w_regexec)(
                pbuf.as_ref().map_or(std::ptr::null(), |b| b.ptr()),
                sbuf.as_ref().map_or(std::ptr::null(), |b| b.ptr()),
                nmatch,
                mp,
            )
        };
        m.truncate(slots);
        (r, m)
    };
    let a = run(&l.c);
    let b = run(&l.rs);
    assert_eq!(
        a.0,
        b.0,
        "w_regexec return divergence [{}] pat={:?} subj={:?} nmatch={}",
        ctx,
        pattern.map(String::from_utf8_lossy),
        subject.map(String::from_utf8_lossy),
        nmatch
    );
    assert_eq!(
        a.1,
        b.1,
        "w_regexec pmatch divergence [{}] pat={:?} subj={:?} nmatch={} slots={}",
        ctx,
        pattern.map(String::from_utf8_lossy),
        subject.map(String::from_utf8_lossy),
        nmatch,
        slots
    );
}

// ---------------------------------------------------------------------------
// Deterministic RNG (splitmix64) — fixed seeds, no external crates
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
    pub fn below(&mut self, n: usize) -> usize {
        if n == 0 {
            0
        } else {
            (self.next_u64() % n as u64) as usize
        }
    }
    pub fn range(&mut self, lo: usize, hi: usize) -> usize {
        lo + self.below(hi.saturating_sub(lo) + 1)
    }
    pub fn pick<'a, T>(&mut self, xs: &'a [T]) -> &'a T {
        &xs[self.below(xs.len())]
    }
    pub fn boolean(&mut self) -> bool {
        self.next_u64() & 1 == 1
    }
}

// ---------------------------------------------------------------------------
// Corpus building blocks
// ---------------------------------------------------------------------------

/// The 12 architecture literals, in the exact order of the C `ARCHS[]` array.
pub const ARCHS: [&str; 12] = [
    "x86_64", "i386", "i686", "sparc", "amd64", "i86pc", "ia64", "AIX", "armv6", "armv7",
    "aarch64", "arm64",
];

/// The five regex patterns `parse_uname_string` passes to `w_regexec`.
pub const LIB_PATTERNS: [&str; 3] = [
    r"^([0-9]+)\.*",
    r"^[0-9]+\.([0-9]+)\.*",
    r"^[0-9]+\.[0-9]+\.([0-9]+(\.[0-9]+)*)\.*",
];

/// Random token that deliberately avoids every marker the parser looks for
/// (`" ["`, `": "`, `" ("`, `"|"`) unless asked for.
pub fn plain_token_n(rng: &mut Rng, len: usize) -> String {
    const CH: &[u8] = b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789-_./+~";
    (0..len).map(|_| *rng.pick(CH) as char).collect()
}

/// `plain_token_n` with the length drawn uniformly from `[lo, hi]`.
pub fn plain_token(rng: &mut Rng, lo: usize, hi: usize) -> String {
    let n = rng.range(lo, hi);
    plain_token_n(rng, n)
}

/// Random token drawn from a marker-rich alphabet, to hit the parser's
/// `strstr` boundaries by accident as well as on purpose.
pub fn marker_token_n(rng: &mut Rng, len: usize) -> String {
    const CH: &[u8] = b"ab01 [](): |.-x86_64AIXarmv7";
    (0..len).map(|_| *rng.pick(CH) as char).collect()
}

pub fn marker_token(rng: &mut Rng, lo: usize, hi: usize) -> String {
    let n = rng.range(lo, hi);
    marker_token_n(rng, n)
}

pub fn digits_n(rng: &mut Rng, n: usize) -> String {
    (0..n.max(1))
        .map(|_| (b'0' + rng.below(10) as u8) as char)
        .collect()
}

pub fn digits(rng: &mut Rng, lo: usize, hi: usize) -> String {
    let n = rng.range(lo, hi);
    digits_n(rng, n)
}

/// A dotted version number with exactly `parts` components.
pub fn version_n(rng: &mut Rng, parts: usize) -> String {
    (0..parts.max(1))
        .map(|_| digits(rng, 1, 5))
        .collect::<Vec<_>>()
        .join(".")
}

/// `version_n` with the component count drawn uniformly from `[lo, hi]`.
pub fn version(rng: &mut Rng, lo: usize, hi: usize) -> String {
    let n = rng.range(lo, hi);
    version_n(rng, n)
}

/// Random count in `[lo, hi]`, as a standalone statement so it can be bound to
/// a local before being handed to a generator.
pub fn hi_bytes(rng: &mut Rng, lo: usize, hi: usize) -> Vec<u8> {
    let n = rng.range(lo, hi);
    (0..n).map(|_| (0x80 + rng.below(0x80)) as u8).collect()
}

// ---------------------------------------------------------------------------
// Extra drivers for the error-path phase
// ---------------------------------------------------------------------------

/// `parse_uname_string(uname, NULL)` — the C returns before touching `uname`,
/// so the whole buffer (including the slack) must come back unchanged.
pub fn diff_parse_null_osd(input: &[u8], ctx: &str) {
    let l = libs();
    let run = |lib: &Lib| -> Vec<u8> {
        let buf = CBuf::new(input);
        let before = buf.raw();
        unsafe { (lib.parse_uname_string)(buf.ptr(), std::ptr::null_mut()) };
        let after = buf.raw();
        assert_eq!(
            before, after,
            "osd==NULL must leave the input buffer untouched [{}]",
            ctx
        );
        after
    };
    let a = run(&l.c);
    let b = run(&l.rs);
    assert_eq!(a, b, "diff_parse_null_osd divergence [{}]", ctx);
}

/// One `w_regexec` invocation in a sequence sharing a single `pmatch` buffer.
pub struct RegCall<'a> {
    pub pattern: Option<&'a [u8]>,
    pub subject: Option<&'a [u8]>,
    pub nmatch: usize,
}

/// Run a sequence of `w_regexec` calls against ONE shared `pmatch` buffer (the
/// way `parse_uname_string` does) and compare every return value plus the final
/// buffer contents. This is what makes stale-offset behaviour observable.
pub fn diff_regexec_seq(calls: &[RegCall<'_>], slots: usize, ctx: &str) {
    let l = libs();
    const POISON: RegMatch = RegMatch {
        rm_so: -515151,
        rm_eo: -525252,
    };
    let run = |lib: &Lib| -> (Vec<c_int>, Vec<RegMatch>) {
        let mut m = vec![POISON; slots.max(1)];
        let mut rets = Vec::with_capacity(calls.len());
        for c in calls {
            assert!(c.nmatch <= slots, "nmatch must fit the pmatch buffer");
            let pbuf = c.pattern.map(CBuf::new);
            let sbuf = c.subject.map(CBuf::new);
            let r = unsafe {
                (lib.w_regexec)(
                    pbuf.as_ref().map_or(std::ptr::null(), |b| b.ptr()),
                    sbuf.as_ref().map_or(std::ptr::null(), |b| b.ptr()),
                    c.nmatch,
                    m.as_mut_ptr(),
                )
            };
            rets.push(r);
        }
        (rets, m)
    };
    let a = run(&l.c);
    let b = run(&l.rs);
    assert_eq!(a.0, b.0, "w_regexec sequence returns diverge [{}]", ctx);
    assert_eq!(a.1, b.1, "w_regexec sequence pmatch diverges [{}]", ctx);
}

// ---------------------------------------------------------------------------
// stderr capture (for the regcomp-failure diagnostic)
// ---------------------------------------------------------------------------

extern "C" {
    fn dup(oldfd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
}

use std::sync::Mutex;
pub static STDERR_LOCK: Mutex<()> = Mutex::new(());

/// Redirect fd 2 to a temporary file for the duration of `f` and return what
/// was written. `stderr` is unbuffered in glibc, so no flush is required.
pub fn capture_stderr<F: FnOnce()>(f: F) -> Vec<u8> {
    use std::io::{Read, Seek, SeekFrom};
    use std::os::unix::io::AsRawFd;

    let _g = STDERR_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let path = std::env::temp_dir().join(format!(
        "driver-stderr-{}-{:?}.txt",
        std::process::id(),
        std::thread::current().id()
    ));
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .read(true)
        .write(true)
        .truncate(true)
        .open(&path)
        .expect("open capture file");
    let fd = file.as_raw_fd();
    let saved = unsafe { dup(2) };
    assert!(saved >= 0, "dup(2) failed");
    unsafe { dup2(fd, 2) };
    f();
    unsafe {
        dup2(saved, 2);
        close(saved);
    }
    file.seek(SeekFrom::Start(0)).expect("seek");
    let mut out = Vec::new();
    file.read_to_end(&mut out).expect("read");
    drop(file);
    let _ = std::fs::remove_file(&path);
    out
}

/// Differential `w_regexec` that also compares the bytes written to `stderr`.
pub fn diff_regexec_with_stderr(
    pattern: Option<&[u8]>,
    subject: Option<&[u8]>,
    nmatch: usize,
    slots: usize,
    ctx: &str,
) -> (c_int, Vec<u8>) {
    let l = libs();
    const POISON: RegMatch = RegMatch {
        rm_so: -616161,
        rm_eo: -626262,
    };
    let run = |lib: &Lib| -> (c_int, Vec<RegMatch>, Vec<u8>) {
        let pbuf = pattern.map(CBuf::new);
        let sbuf = subject.map(CBuf::new);
        let mut m = vec![POISON; slots.max(1)];
        let mut r: c_int = 0;
        let err = capture_stderr(|| {
            r = unsafe {
                (lib.w_regexec)(
                    pbuf.as_ref().map_or(std::ptr::null(), |b| b.ptr()),
                    sbuf.as_ref().map_or(std::ptr::null(), |b| b.ptr()),
                    nmatch,
                    m.as_mut_ptr(),
                )
            };
        });
        m.truncate(slots);
        (r, m, err)
    };
    let a = run(&l.c);
    let b = run(&l.rs);
    assert_eq!(a.0, b.0, "w_regexec return diverges [{}]", ctx);
    assert_eq!(a.1, b.1, "w_regexec pmatch diverges [{}]", ctx);
    assert_eq!(
        String::from_utf8_lossy(&a.2),
        String::from_utf8_lossy(&b.2),
        "w_regexec stderr diagnostic diverges [{}]",
        ctx
    );
    (a.0, a.2)
}
