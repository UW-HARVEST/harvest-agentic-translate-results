//! Shared differential-test harness.
//!
//! BOTH implementations are loaded as shared objects through `libloading` and
//! called only through their exported C symbols — the Rust crate is never
//! linked or called directly, so the `#[no_mangle]` / `extern "C"` wrappers are
//! part of what is under test.

#![allow(dead_code)]
#![allow(non_camel_case_types)]

use libloading::{Library, Symbol};
use std::ffi::{c_char, c_int, c_void, CStr};
use std::path::PathBuf;
use std::sync::OnceLock;

// ---------------------------------------------------------------------------
// ABI types (must mirror c_src/include/lib.h and <regex.h>)
// ---------------------------------------------------------------------------

#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct RegMatch {
    pub rm_so: c_int,
    pub rm_eo: c_int,
}

impl std::fmt::Debug for RegMatch {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{{{},{}}}", self.rm_so, self.rm_eo)
    }
}

pub const SENTINEL: RegMatch = RegMatch {
    rm_so: -0x4243_4445,
    rm_eo: -0x4647_4849,
};

/// `typedef struct os_data { char *…; } os_data;` — 9 pointers.
#[repr(C)]
#[derive(Clone, Copy)]
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

    /// Pre-fill every field with a distinct non-null sentinel pointer so that
    /// "field left untouched by the C" is distinguishable from "field set to
    /// NULL by the C". The C code never zeroes `osd`.
    pub fn prefilled(sentinels: &[*mut c_char; 9]) -> Self {
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

    pub fn fields(&self) -> [*mut c_char; 9] {
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

pub const FIELD_NAMES: [&str; 9] = [
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

/// Observable result of one `parse_uname_string` call: the 9 fields, each as
/// `None` (NULL) / `Some(bytes)`, plus the caller's mutated `uname` buffer.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct ParseOutcome {
    pub fields: Vec<Option<Vec<u8>>>,
    /// Raw pointer identity classification, used for the pre-filled test:
    /// `true` when the field pointer is bit-identical to the sentinel that was
    /// there before the call (i.e. the C did not write the field).
    pub untouched: Vec<bool>,
    /// Full byte image of the caller's `uname` buffer after the call.
    pub buffer: Vec<u8>,
    /// `malloc_usable_size` of each newly-allocated field.
    ///
    /// The C documents that the caller must free these buffers, so the size
    /// asked of `malloc` (`lib.c:77,84,91` use `match_size + 1`; every other
    /// field uses `strdup`, i.e. `strlen + 1`) is part of the observable
    /// contract. It is NOT compared by `assert_parse_eq`: glibc may hand back a
    /// chunk larger than the request when it reuses a free chunk, so the value
    /// depends on heap history and the C/Rust calls cannot share one.
    /// `phase_d_alloc_sizes.rs` compares it in heap-lockstep subprocesses.
    pub sizes: Vec<Option<usize>>,
}

// ---------------------------------------------------------------------------
// libc bindings used by the harness itself (freeing, buffer setup)
// ---------------------------------------------------------------------------

extern "C" {
    fn free(p: *mut c_void);
    fn strdup(s: *const c_char) -> *mut c_char;
    /// glibc: derived purely from the original request size, so it is a stable
    /// witness for "did both sides ask `malloc` for the same number of bytes?".
    fn malloc_usable_size(p: *mut c_void) -> usize;
}

// ---------------------------------------------------------------------------
// Library loading
// ---------------------------------------------------------------------------

pub type FnGetOsArch = unsafe extern "C" fn(*mut c_char) -> *mut c_char;
pub type FnWRegexec =
    unsafe extern "C" fn(*const c_char, *const c_char, usize, *mut RegMatch) -> c_int;
pub type FnParseUname = unsafe extern "C" fn(*mut c_char, *mut OsData) -> ();

pub struct Impl {
    pub name: &'static str,
    _lib: Library,
    pub get_os_arch: FnGetOsArch,
    pub w_regexec: FnWRegexec,
    pub parse_uname_string: FnParseUname,
}

impl Impl {
    unsafe fn load(name: &'static str, path: &PathBuf) -> Impl {
        let lib = Library::new(path)
            .unwrap_or_else(|e| panic!("cannot dlopen {} ({}): {e}", name, path.display()));
        let get_os_arch: Symbol<FnGetOsArch> = lib
            .get(b"get_os_arch\0")
            .unwrap_or_else(|e| panic!("{name}: missing symbol get_os_arch: {e}"));
        let w_regexec: Symbol<FnWRegexec> = lib
            .get(b"w_regexec\0")
            .unwrap_or_else(|e| panic!("{name}: missing symbol w_regexec: {e}"));
        let parse_uname_string: Symbol<FnParseUname> = lib
            .get(b"parse_uname_string\0")
            .unwrap_or_else(|e| panic!("{name}: missing symbol parse_uname_string: {e}"));
        let (g, w, p) = (*get_os_arch, *w_regexec, *parse_uname_string);
        Impl {
            name,
            _lib: lib,
            get_os_arch: g,
            w_regexec: w,
            parse_uname_string: p,
        }
    }
}

pub struct Both {
    pub c: Impl,
    pub rs: Impl,
}

fn crate_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// Locate the Rust cdylib built for the current test profile. The test binary
/// lives at `<target>/<profile>/deps/<name>-<hash>`, so the cdylib is two
/// directories up.
fn rust_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("RUST_DRIVER_SO") {
        return PathBuf::from(p);
    }
    let exe = std::env::current_exe().expect("current_exe");
    let profile_dir = exe
        .parent()
        .and_then(|p| p.parent())
        .expect("target/<profile>");
    let candidate = profile_dir.join("libdriver.so");
    if candidate.exists() {
        return candidate;
    }
    for p in ["release", "debug"] {
        let c = crate_root().join("target").join(p).join("libdriver.so");
        if c.exists() {
            return c;
        }
    }
    panic!(
        "Rust libdriver.so not found (looked in {}). Run `cargo build`.",
        profile_dir.display()
    );
}

fn c_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("C_DRIVER_SO") {
        return PathBuf::from(p);
    }
    let p = crate_root()
        .parent()
        .expect("workdir")
        .join("c_src/build/libdriver.so");
    assert!(
        p.exists(),
        "C libdriver.so not found at {}. Build it with cmake first.",
        p.display()
    );
    p
}

static BOTH: OnceLock<Both> = OnceLock::new();

pub fn both() -> &'static Both {
    BOTH.get_or_init(|| unsafe {
        Both {
            c: Impl::load("C", &c_so_path()),
            rs: Impl::load("Rust", &rust_so_path()),
        }
    })
}

// ---------------------------------------------------------------------------
// Differential drivers
// ---------------------------------------------------------------------------

unsafe fn cstr_bytes(p: *const c_char) -> Option<Vec<u8>> {
    if p.is_null() {
        None
    } else {
        Some(CStr::from_ptr(p).to_bytes().to_vec())
    }
}

/// A NUL-terminated, heap-allocated, writable copy of `bytes` with a guard
/// region in front so that the C code's documented `p[-1] = '\0'` underflow on
/// empty strings is captured instead of corrupting unrelated memory.
pub struct Buf {
    raw: Vec<u8>,
    /// offset of the logical string start inside `raw`
    start: usize,
    len: usize,
}

const GUARD: usize = 16;
const GUARD_BYTE: u8 = 0xA5;

impl Buf {
    pub fn new(bytes: &[u8]) -> Buf {
        assert!(
            !bytes.contains(&0),
            "test inputs must not contain interior NUL"
        );
        let mut raw = vec![GUARD_BYTE; GUARD];
        raw.extend_from_slice(bytes);
        raw.push(0);
        raw.extend_from_slice(&[GUARD_BYTE; GUARD]);
        Buf {
            raw,
            start: GUARD,
            len: bytes.len(),
        }
    }
    pub fn ptr(&mut self) -> *mut c_char {
        unsafe { self.raw.as_mut_ptr().add(self.start) as *mut c_char }
    }
    pub fn cptr(&self) -> *const c_char {
        unsafe { self.raw.as_ptr().add(self.start) as *const c_char }
    }
    /// Whole image including guards, so any out-of-bounds write is compared too.
    pub fn image(&self) -> Vec<u8> {
        self.raw.clone()
    }
    pub fn logical_len(&self) -> usize {
        self.len
    }
}

/// Distinct non-null sentinel pointers for the pre-filled `os_data` test.
pub struct Sentinels {
    pub ptrs: [*mut c_char; 9],
}

impl Sentinels {
    pub fn new(tag: &str) -> Sentinels {
        let mut ptrs = [std::ptr::null_mut(); 9];
        for (i, slot) in ptrs.iter_mut().enumerate() {
            let s = std::ffi::CString::new(format!("<{tag}-sentinel-{i}>")).unwrap();
            *slot = unsafe { strdup(s.as_ptr()) };
            assert!(!slot.is_null());
        }
        Sentinels { ptrs }
    }
}

impl Drop for Sentinels {
    fn drop(&mut self) {
        for p in self.ptrs {
            unsafe { free(p as *mut c_void) }
        }
    }
}

/// Run `parse_uname_string` on one implementation with its own private copy of
/// the input buffer and collect everything observable.
///
/// `pre` supplies the initial `os_data` field values; fields whose pointer is
/// still bit-identical to `pre` afterwards are reported as "untouched".
/// Newly-allocated fields are `free`d before returning (same libc allocator on
/// both sides, so this is legal for either `.so`).
pub fn run_parse(f: FnParseUname, input: &[u8], pre: &[*mut c_char; 9]) -> ParseOutcome {
    let mut buf = Buf::new(input);
    let mut osd = OsData::prefilled(pre);
    unsafe { f(buf.ptr(), &mut osd) };

    let after = osd.fields();
    let mut fields = Vec::with_capacity(9);
    let mut untouched = Vec::with_capacity(9);
    let mut sizes = Vec::with_capacity(9);
    for i in 0..9 {
        let same = after[i] == pre[i];
        untouched.push(same);
        fields.push(unsafe { cstr_bytes(after[i]) });
        sizes.push(if same || after[i].is_null() {
            None
        } else {
            Some(unsafe { malloc_usable_size(after[i] as *mut c_void) })
        });
    }
    // Free only what the library allocated.
    for i in 0..9 {
        if !untouched[i] && !after[i].is_null() {
            unsafe { free(after[i] as *mut c_void) }
        }
    }
    ParseOutcome {
        fields,
        untouched,
        buffer: buf.image(),
        sizes,
    }
}

/// `parse_uname_string` with an all-NULL `os_data` (the ordinary caller shape).
pub fn run_parse_zeroed(f: FnParseUname, input: &[u8]) -> ParseOutcome {
    run_parse(f, input, &[std::ptr::null_mut(); 9])
}

fn describe(input: &[u8]) -> String {
    format!("{:?} (len {})", String::from_utf8_lossy(input), input.len())
}

/// Differentially compare `parse_uname_string` between C and Rust.
pub fn diff_parse(input: &[u8], ctx: &str) {
    let b = both();
    let null9 = [std::ptr::null_mut(); 9];
    let c = run_parse(b.c.parse_uname_string, input, &null9);
    let r = run_parse(b.rs.parse_uname_string, input, &null9);
    assert_parse_eq(&c, &r, input, ctx);
}

/// Same, but starting from a pre-filled (non-null) `os_data`.
pub fn diff_parse_prefilled(input: &[u8], ctx: &str) {
    let b = both();
    let sc = Sentinels::new("c");
    let sr = Sentinels::new("rs");
    let c = run_parse(b.c.parse_uname_string, input, &sc.ptrs);
    let r = run_parse(b.rs.parse_uname_string, input, &sr.ptrs);
    // The sentinel *contents* differ only in their tag, so compare the
    // untouched-mask plus the contents of touched fields.
    assert_eq!(
        c.untouched,
        r.untouched,
        "\n[{ctx}] untouched-field mask differs for input {}\n  C   : {:?}\n  Rust: {:?}\n  names: {:?}",
        describe(input),
        c.untouched,
        r.untouched,
        FIELD_NAMES
    );
    for i in 0..9 {
        if c.untouched[i] {
            continue;
        }
        assert_eq!(
            c.fields[i], r.fields[i],
            "\n[{ctx}] field {} differs for input {}\n  C   : {:?}\n  Rust: {:?}",
            FIELD_NAMES[i],
            describe(input),
            c.fields[i].as_ref().map(|v| String::from_utf8_lossy(v).into_owned()),
            r.fields[i].as_ref().map(|v| String::from_utf8_lossy(v).into_owned()),
        );
    }
    assert_eq!(
        c.buffer,
        r.buffer,
        "\n[{ctx}] mutated uname buffer differs for input {}\n  C   : {:?}\n  Rust: {:?}",
        describe(input),
        String::from_utf8_lossy(&c.buffer),
        String::from_utf8_lossy(&r.buffer),
    );
}

pub fn assert_parse_eq(c: &ParseOutcome, r: &ParseOutcome, input: &[u8], ctx: &str) {
    for i in 0..9 {
        assert_eq!(
            c.fields[i], r.fields[i],
            "\n[{ctx}] field {} differs for input {}\n  C   : {:?}\n  Rust: {:?}",
            FIELD_NAMES[i],
            describe(input),
            c.fields[i].as_ref().map(|v| String::from_utf8_lossy(v).into_owned()),
            r.fields[i].as_ref().map(|v| String::from_utf8_lossy(v).into_owned()),
        );
        assert_eq!(
            c.untouched[i], r.untouched[i],
            "\n[{ctx}] field {} NULL-ness/write differs for input {}",
            FIELD_NAMES[i],
            describe(input),
        );
    }
    assert_eq!(
        c.buffer,
        r.buffer,
        "\n[{ctx}] mutated uname buffer (incl. guard bytes) differs for input {}\n  C   : {:02x?}\n  Rust: {:02x?}",
        describe(input),
        c.buffer,
        r.buffer,
    );
}

/// Differentially compare `get_os_arch`.
pub fn diff_arch(input: &[u8], ctx: &str) {
    let b = both();
    let mut bc = Buf::new(input);
    let mut br = Buf::new(input);
    let (rc, rr) = unsafe {
        let pc = (b.c.get_os_arch)(bc.ptr());
        let pr = (b.rs.get_os_arch)(br.ptr());
        let vc = cstr_bytes(pc);
        let vr = cstr_bytes(pr);
        if !pc.is_null() {
            free(pc as *mut c_void)
        }
        if !pr.is_null() {
            free(pr as *mut c_void)
        }
        (vc, vr)
    };
    assert_eq!(
        rc,
        rr,
        "\n[{ctx}] get_os_arch differs for input {}\n  C   : {:?}\n  Rust: {:?}",
        describe(input),
        rc.as_ref().map(|v| String::from_utf8_lossy(v).into_owned()),
        rr.as_ref().map(|v| String::from_utf8_lossy(v).into_owned()),
    );
    assert_eq!(
        bc.image(),
        br.image(),
        "\n[{ctx}] get_os_arch mutated its input differently for {}",
        describe(input)
    );
}

/// Differentially compare `w_regexec`, including the full `regmatch_t` array.
pub fn diff_regexec(pattern: Option<&[u8]>, subject: Option<&[u8]>, nmatch: usize, slots: usize, ctx: &str) {
    let b = both();
    let mut pbuf = pattern.map(Buf::new);
    let mut sbuf = subject.map(Buf::new);
    let pp = pbuf.as_mut().map_or(std::ptr::null(), |x| x.cptr());
    let sp = sbuf.as_mut().map_or(std::ptr::null(), |x| x.cptr());

    let mut mc = vec![SENTINEL; slots];
    let mut mr = vec![SENTINEL; slots];
    let (vc, vr) = unsafe {
        (
            (b.c.w_regexec)(pp, sp, nmatch, mc.as_mut_ptr()),
            (b.rs.w_regexec)(pp, sp, nmatch, mr.as_mut_ptr()),
        )
    };
    let pd = pattern.map(String::from_utf8_lossy);
    let sd = subject.map(String::from_utf8_lossy);
    assert_eq!(
        vc, vr,
        "\n[{ctx}] w_regexec return differs\n  pattern={pd:?} subject={sd:?} nmatch={nmatch}\n  C={vc} Rust={vr}"
    );
    assert_eq!(
        mc, mr,
        "\n[{ctx}] w_regexec pmatch differs\n  pattern={pd:?} subject={sd:?} nmatch={nmatch}\n  C   ={mc:?}\n  Rust={mr:?}"
    );
    if let (Some(a), Some(c)) = (pbuf.as_ref(), sbuf.as_ref()) {
        let _ = (a.logical_len(), c.logical_len());
    }
    // Inputs are const; neither side may mutate them.
    if let Some(x) = pbuf.as_ref() {
        assert_eq!(
            x.image()[..],
            Buf::new(pattern.unwrap()).image()[..],
            "[{ctx}] pattern buffer was mutated"
        );
    }
    if let Some(x) = sbuf.as_ref() {
        assert_eq!(
            x.image()[..],
            Buf::new(subject.unwrap()).image()[..],
            "[{ctx}] subject buffer was mutated"
        );
    }
}

/// Convenience: `nmatch == slots`.
pub fn diff_regexec_n(pattern: &[u8], subject: &[u8], nmatch: usize, ctx: &str) {
    diff_regexec(Some(pattern), Some(subject), nmatch, nmatch.max(1), ctx);
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (xorshift64*) — fixed seed for reproducibility
// ---------------------------------------------------------------------------

/// xorshift64* with interior mutability so that nested calls like
/// `rng.bytes_from(a, rng.below(8))` are expressible.
pub struct Rng(std::cell::Cell<u64>);

pub const SEED: u64 = 0x5EED_1234_ABCD_9876;

impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng(std::cell::Cell::new(if seed == 0 {
            0x9E37_79B9_7F4A_7C15
        } else {
            seed
        }))
    }
    pub fn next_u64(&self) -> u64 {
        let mut x = self.0.get();
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0.set(x);
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
    pub fn below(&self, n: usize) -> usize {
        if n == 0 {
            0
        } else {
            (self.next_u64() % n as u64) as usize
        }
    }
    pub fn range(&self, lo: usize, hi_inclusive: usize) -> usize {
        lo + self.below(hi_inclusive - lo + 1)
    }
    pub fn pick<'a, T>(&self, xs: &'a [T]) -> &'a T {
        &xs[self.below(xs.len())]
    }
    pub fn bool(&self) -> bool {
        self.next_u64() & 1 == 1
    }
    /// Random bytes drawn from `alphabet`, never containing NUL.
    pub fn bytes_from(&self, alphabet: &[u8], len: usize) -> Vec<u8> {
        (0..len).map(|_| *self.pick(alphabet)).collect()
    }
    /// A decimal number string with interesting shapes: 0, small, leading
    /// zeros, long runs of digits, values past u32/i32/u64 range.
    pub fn number(&self) -> Vec<u8> {
        match self.below(8) {
            0 => b"0".to_vec(),
            1 => format!("{}", self.below(10)).into_bytes(),
            2 => format!("{}", self.below(100000)).into_bytes(),
            3 => format!("{:07}", self.below(1000)).into_bytes(), // leading zeros
            4 => format!("{}", u32::MAX as u64 + self.below(5) as u64).into_bytes(),
            5 => format!("{}", i32::MAX as u64 + self.below(5) as u64).into_bytes(),
            6 => format!("{}", u64::MAX - self.below(5) as u64).into_bytes(),
            _ => {
                let n = self.range(1, 40);
                (0..n).map(|_| b'0' + self.below(10) as u8).collect()
            }
        }
    }
    /// One of the 12 ARCHS tokens.
    pub fn arch(&self) -> &'static str {
        ARCHS[self.below(ARCHS.len())]
    }
}

/// Filler text that deliberately never contains any of the parser's separator
/// sequences unless explicitly asked for.
pub const SAFE_ALPHA: &[u8] = b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789-_/~+@#";

/// Alphabet enriched with every byte the C code special-cases.
pub const HOSTILE_ALPHA: &[u8] =
    b"  [[]]::(())||..VVeerr0123456789abcXzz\t\x7f\x80\xff\xfe\xc3\xa9/-_";

pub const ARCHS: [&str; 12] = [
    "x86_64", "i386", "i686", "sparc", "amd64", "i86pc", "ia64", "AIX", "armv6", "armv7",
    "aarch64", "arm64",
];

pub const PARSER_PATTERNS: [&str; 3] = [
    r"^([0-9]+)\.*",
    r"^[0-9]+\.([0-9]+)\.*",
    r"^[0-9]+\.[0-9]+\.([0-9]+(\.[0-9]+)*)\.*",
];

// ---------------------------------------------------------------------------
// stderr silencing
// ---------------------------------------------------------------------------

extern "C" {
    fn dup(fd: c_int) -> c_int;
    fn dup2(old: c_int, new: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn open(path: *const c_char, flags: c_int, ...) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
}

/// fd 2 is process-global, so every redirection must be serialised against the
/// other `cargo test` threads or their `regcomp` diagnostics leak into each
/// other's capture buffers.
static STDERR_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

fn stderr_lock() -> std::sync::MutexGuard<'static, ()> {
    match STDERR_LOCK.lock() {
        Ok(g) => g,
        Err(poisoned) => poisoned.into_inner(),
    }
}

/// Run `f` with fd 2 pointed at /dev/null. Both `.so`s call `fprintf(stderr, …)`
/// on `regcomp` failure (`lib.c:41`); the message text is identical by
/// construction (same format string, same libc), and the fuzz rows would
/// otherwise emit tens of thousands of lines.
pub fn with_stderr_silenced<R>(f: impl FnOnce() -> R) -> R {
    let _g = stderr_lock();
    unsafe {
        fflush(std::ptr::null_mut());
        let saved = dup(2);
        let devnull = open(c"/dev/null".as_ptr(), 1 /* O_WRONLY */);
        if devnull >= 0 {
            dup2(devnull, 2);
            close(devnull);
        }
        let r = f();
        fflush(std::ptr::null_mut());
        if saved >= 0 {
            dup2(saved, 2);
            close(saved);
        }
        r
    }
}

/// Run `f` with fd 2 redirected into a temp file and return everything written.
/// Used to prove the `regcomp`-failure diagnostic (`lib.c:41`) is byte-identical.
pub fn capture_stderr<R>(tag: &str, f: impl FnOnce() -> R) -> (R, Vec<u8>) {
    use std::io::Read;
    use std::os::unix::io::AsRawFd;
    let dir = std::env::temp_dir();
    let path = dir.join(format!(
        "diffstderr-{}-{}-{}.txt",
        std::process::id(),
        tag,
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let file = std::fs::File::create(&path).expect("create temp stderr file");
    let _g = stderr_lock();
    let r = unsafe {
        fflush(std::ptr::null_mut());
        let saved = dup(2);
        dup2(file.as_raw_fd(), 2);
        let r = f();
        fflush(std::ptr::null_mut());
        dup2(saved, 2);
        close(saved);
        r
    };
    drop(file);
    let mut out = Vec::new();
    std::fs::File::open(&path)
        .expect("reopen temp stderr file")
        .read_to_end(&mut out)
        .expect("read temp stderr file");
    let _ = std::fs::remove_file(&path);
    (r, out)
}

/// Raw single-implementation `w_regexec` call, for tests that need to inspect
/// each side separately (e.g. stderr capture).
pub fn call_regexec(
    f: FnWRegexec,
    pattern: Option<&[u8]>,
    subject: Option<&[u8]>,
    nmatch: usize,
    slots: usize,
) -> (c_int, Vec<RegMatch>) {
    let mut pbuf = pattern.map(Buf::new);
    let mut sbuf = subject.map(Buf::new);
    let pp = pbuf.as_mut().map_or(std::ptr::null(), |x| x.cptr());
    let sp = sbuf.as_mut().map_or(std::ptr::null(), |x| x.cptr());
    let mut m = vec![SENTINEL; slots];
    let rv = unsafe { f(pp, sp, nmatch, m.as_mut_ptr()) };
    (rv, m)
}

/// Raw single-implementation `get_os_arch` call returning owned bytes.
pub fn call_arch(f: FnGetOsArch, input: &[u8]) -> Option<Vec<u8>> {
    let mut b = Buf::new(input);
    unsafe {
        let p = f(b.ptr());
        let v = cstr_bytes(p);
        if !p.is_null() {
            free(p as *mut c_void);
        }
        v
    }
}

/// `malloc_usable_size` of the buffer `get_os_arch` returned (`lib.c:24`).
pub fn arch_alloc_size(f: FnGetOsArch, input: &[u8]) -> Option<usize> {
    let mut b = Buf::new(input);
    unsafe {
        let p = f(b.ptr());
        if p.is_null() {
            return None;
        }
        let n = malloc_usable_size(p as *mut c_void);
        free(p as *mut c_void);
        Some(n)
    }
}
