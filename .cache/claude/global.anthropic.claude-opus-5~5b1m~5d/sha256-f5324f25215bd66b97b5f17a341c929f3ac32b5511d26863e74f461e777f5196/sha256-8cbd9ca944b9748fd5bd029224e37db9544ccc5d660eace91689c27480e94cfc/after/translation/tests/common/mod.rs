//! Shared differential-test harness.
//!
//! BOTH implementations are loaded as shared objects via `libloading` and
//! called only through their exported C symbols — the Rust crate is never
//! called directly, so the `#[no_mangle]` wrappers are under test too.

#![allow(dead_code)]
#![allow(non_camel_case_types)]

use libloading::{Library, Symbol};
use std::ffi::{c_char, c_int, CStr, CString};
use std::path::PathBuf;
use std::sync::OnceLock;

// ---------------------------------------------------------------------------
// ABI mirrors (must match c_src/include/lib.h and glibc <regex.h>)
// ---------------------------------------------------------------------------

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct regmatch_t {
    pub rm_so: c_int,
    pub rm_eo: c_int,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct os_data {
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

impl os_data {
    pub fn zeroed() -> Self {
        // SAFETY: an all-zero os_data is a valid struct of 9 NULL pointers,
        // which is exactly the state a real C caller uses (`os_data osd = {0}`).
        unsafe { std::mem::zeroed() }
    }

    /// Pre-populate every field with a *distinct, readable* sentinel pointer so
    /// that "field left untouched" is observable and safely readable.
    pub fn sentinelled(sent: &Sentinels) -> Self {
        os_data {
            os_name: sent.p(0),
            os_version: sent.p(1),
            os_major: sent.p(2),
            os_minor: sent.p(3),
            os_codename: sent.p(4),
            os_platform: sent.p(5),
            os_build: sent.p(6),
            os_uname: sent.p(7),
            os_arch: sent.p(8),
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

/// Nine leaked C strings used as readable sentinel values.
pub struct Sentinels {
    ptrs: Vec<*mut c_char>,
}

impl Sentinels {
    pub fn new() -> Self {
        let ptrs = (0..9)
            .map(|i| {
                let s = CString::new(format!("<<untouched-{i}>>")).unwrap();
                s.into_raw()
            })
            .collect();
        Sentinels { ptrs }
    }
    pub fn p(&self, i: usize) -> *mut c_char {
        self.ptrs[i]
    }
}

/// Snapshot of an `os_data` as owned byte strings, `None` == NULL pointer.
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct OsDataSnapshot {
    pub fields: Vec<Option<Vec<u8>>>,
}

impl OsDataSnapshot {
    /// # Safety
    /// Every non-NULL field must point to a valid NUL-terminated string.
    pub unsafe fn capture(osd: &os_data) -> Self {
        let fields = osd
            .fields()
            .iter()
            .map(|&p| {
                if p.is_null() {
                    None
                } else {
                    Some(CStr::from_ptr(p).to_bytes().to_vec())
                }
            })
            .collect();
        OsDataSnapshot { fields }
    }

    pub fn describe(&self) -> String {
        let mut out = String::new();
        for (n, f) in FIELD_NAMES.iter().zip(self.fields.iter()) {
            match f {
                None => out.push_str(&format!("  {n:12} = NULL\n")),
                Some(b) => out.push_str(&format!("  {n:12} = {:?}\n", String::from_utf8_lossy(b))),
            }
        }
        out
    }
}

// ---------------------------------------------------------------------------
// Result of one `parse_uname_string` invocation
// ---------------------------------------------------------------------------

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct ParseOutcome {
    pub snapshot: OsDataSnapshot,
    /// The caller's `uname` buffer after in-place mutation (full byte contents,
    /// including everything past the first NUL, plus the guard bytes).
    pub buffer: Vec<u8>,
}

// ---------------------------------------------------------------------------
// Loaded implementation
// ---------------------------------------------------------------------------

type FnGetOsArch = unsafe extern "C" fn(*mut c_char) -> *mut c_char;
type FnWRegexec =
    unsafe extern "C" fn(*const c_char, *const c_char, usize, *mut regmatch_t) -> c_int;
type FnParseUname = unsafe extern "C" fn(*mut c_char, *mut os_data);

pub struct Impl {
    pub name: &'static str,
    _lib: Library,
    pub get_os_arch: FnGetOsArch,
    pub w_regexec: FnWRegexec,
    pub parse_uname_string: FnParseUname,
}

impl Impl {
    fn load(name: &'static str, path: &PathBuf) -> Impl {
        // SAFETY: loading a trusted, freshly built shared object.
        unsafe {
            let lib = Library::new(path)
                .unwrap_or_else(|e| panic!("failed to dlopen {}: {e}", path.display()));
            let get_os_arch: Symbol<FnGetOsArch> = lib
                .get(b"get_os_arch\0")
                .expect("missing symbol get_os_arch");
            let w_regexec: Symbol<FnWRegexec> =
                lib.get(b"w_regexec\0").expect("missing symbol w_regexec");
            let parse_uname_string: Symbol<FnParseUname> = lib
                .get(b"parse_uname_string\0")
                .expect("missing symbol parse_uname_string");
            let g = *get_os_arch;
            let w = *w_regexec;
            let p = *parse_uname_string;
            Impl {
                name,
                _lib: lib,
                get_os_arch: g,
                w_regexec: w,
                parse_uname_string: p,
            }
        }
    }

    // -- typed wrappers -----------------------------------------------------

    /// Calls `get_os_arch` on a private mutable copy of `input`.
    /// Returns the resulting string (`None` for NULL) — the returned block is
    /// intentionally leaked (see note in `parse`).
    pub fn get_arch(&self, input: &[u8]) -> Option<Vec<u8>> {
        let mut buf: Vec<u8> = input.to_vec();
        buf.push(0);
        unsafe {
            let r = (self.get_os_arch)(buf.as_mut_ptr() as *mut c_char);
            if r.is_null() {
                None
            } else {
                Some(CStr::from_ptr(r).to_bytes().to_vec())
            }
        }
    }

    /// Raw `get_os_arch` with an explicit pointer (for NULL tests).
    pub unsafe fn get_arch_raw(&self, p: *mut c_char) -> *mut c_char {
        (self.get_os_arch)(p)
    }

    /// Calls `w_regexec`. `pmatch_init` seeds the match array so that slots the
    /// C left untouched are still comparable. Returns `(ret, pmatch_after)`.
    pub fn regexec(
        &self,
        pattern: Option<&[u8]>,
        string: Option<&[u8]>,
        nmatch: usize,
        pmatch_init: &[regmatch_t],
    ) -> (c_int, Vec<regmatch_t>) {
        let pat_buf = pattern.map(|p| {
            let mut v = p.to_vec();
            v.push(0);
            v
        });
        let str_buf = string.map(|s| {
            let mut v = s.to_vec();
            v.push(0);
            v
        });
        let mut pm: Vec<regmatch_t> = pmatch_init.to_vec();
        let pat_ptr = pat_buf
            .as_ref()
            .map(|v| v.as_ptr() as *const c_char)
            .unwrap_or(std::ptr::null());
        let str_ptr = str_buf
            .as_ref()
            .map(|v| v.as_ptr() as *const c_char)
            .unwrap_or(std::ptr::null());
        let ret = unsafe { (self.w_regexec)(pat_ptr, str_ptr, nmatch, pm.as_mut_ptr()) };
        (ret, pm)
    }

    /// Raw `w_regexec` (for NULL-`pmatch` tests).
    pub unsafe fn regexec_raw(
        &self,
        pattern: *const c_char,
        string: *const c_char,
        nmatch: usize,
        pmatch: *mut regmatch_t,
    ) -> c_int {
        (self.w_regexec)(pattern, string, nmatch, pmatch)
    }

    /// Runs `parse_uname_string` on a private, guard-padded copy of `uname`
    /// with a freshly zeroed `os_data`.
    ///
    /// Note: every `strdup`/`malloc` block the library returns is deliberately
    /// **leaked**. Some C code paths write one byte before a heap block
    /// (ERRORS.md rows 22–24), which corrupts that block's metadata; calling
    /// `free` on it would abort. Leaking keeps the comparison faithful.
    pub fn parse(&self, uname: &[u8]) -> ParseOutcome {
        self.parse_with(uname, &mut os_data::zeroed())
    }

    pub fn parse_sent(&self, uname: &[u8], sent: &Sentinels) -> ParseOutcome {
        self.parse_with(uname, &mut os_data::sentinelled(sent))
    }

    pub fn parse_with(&self, uname: &[u8], osd: &mut os_data) -> ParseOutcome {
        // 16 guard bytes on each side so that any stray write next to the
        // buffer is captured rather than corrupting the test harness.
        const GUARD: usize = 16;
        let mut buf = vec![0xAAu8; GUARD];
        buf.extend_from_slice(uname);
        buf.push(0);
        buf.extend_from_slice(&[0xAAu8; GUARD]);

        unsafe {
            let p = buf.as_mut_ptr().add(GUARD) as *mut c_char;
            (self.parse_uname_string)(p, osd);
            ParseOutcome {
                snapshot: OsDataSnapshot::capture(osd),
                buffer: buf,
            }
        }
    }

    /// `parse_uname_string` with a NULL `osd`.
    pub fn parse_null_osd(&self, uname: &[u8]) -> Vec<u8> {
        let mut buf: Vec<u8> = uname.to_vec();
        buf.push(0);
        unsafe {
            (self.parse_uname_string)(buf.as_mut_ptr() as *mut c_char, std::ptr::null_mut());
        }
        buf
    }
}

// ---------------------------------------------------------------------------
// Global handles
// ---------------------------------------------------------------------------

fn crate_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_so_path() -> PathBuf {
    crate_root()
        .parent()
        .expect("crate has a parent dir")
        .join("c_src/build/libdriver.so")
}

fn rust_so_path() -> PathBuf {
    // `DIFF_RUST_SO` lets the driver script point the tests at a specific build
    // profile (release vs. debug, i.e. overflow-checks off vs. on).
    if let Ok(p) = std::env::var("DIFF_RUST_SO") {
        return PathBuf::from(p);
    }
    // Prefer the release artifact (the shipped one); fall back to debug.
    let rel = crate_root().join("target/release/libdriver.so");
    if rel.exists() {
        return rel;
    }
    crate_root().join("target/debug/libdriver.so")
}

static C_IMPL: OnceLock<Impl> = OnceLock::new();
static RUST_IMPL: OnceLock<Impl> = OnceLock::new();

pub fn c_impl() -> &'static Impl {
    C_IMPL.get_or_init(|| {
        let p = c_so_path();
        assert!(
            p.exists(),
            "C shared library not built: {}\nRun:\n  cd c_src && mkdir -p build && cd build && \
             cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
            p.display()
        );
        Impl::load("C", &p)
    })
}

pub fn rust_impl() -> &'static Impl {
    RUST_IMPL.get_or_init(|| {
        let p = rust_so_path();
        assert!(
            p.exists(),
            "Rust shared library not built: {}\nRun: cargo build --release",
            p.display()
        );
        Impl::load("Rust", &p)
    })
}

/// `(c, rust)` pair.
pub fn both() -> (&'static Impl, &'static Impl) {
    (c_impl(), rust_impl())
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (xoshiro256**) — fixed seed for reproducibility
// ---------------------------------------------------------------------------

pub struct Rng {
    s: [u64; 4],
}

impl Rng {
    pub fn new(seed: u64) -> Rng {
        // SplitMix64 to expand the seed.
        let mut x = seed.wrapping_add(0x9E3779B97F4A7C15);
        let mut next = || {
            x = x.wrapping_add(0x9E3779B97F4A7C15);
            let mut z = x;
            z = (z ^ (z >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
            z = (z ^ (z >> 27)).wrapping_mul(0x94D049BB133111EB);
            z ^ (z >> 31)
        };
        Rng {
            s: [next(), next(), next(), next()],
        }
    }

    pub fn next_u64(&mut self) -> u64 {
        let result = self.s[1]
            .wrapping_mul(5)
            .rotate_left(7)
            .wrapping_mul(9);
        let t = self.s[1] << 17;
        self.s[2] ^= self.s[0];
        self.s[3] ^= self.s[1];
        self.s[1] ^= self.s[2];
        self.s[0] ^= self.s[3];
        self.s[2] ^= t;
        self.s[3] = self.s[3].rotate_left(45);
        result
    }

    /// Uniform in `[0, n)`; `n == 0` yields 0.
    pub fn below(&mut self, n: usize) -> usize {
        if n == 0 {
            0
        } else {
            (self.next_u64() % n as u64) as usize
        }
    }

    pub fn range(&mut self, lo: usize, hi_inclusive: usize) -> usize {
        lo + self.below(hi_inclusive - lo + 1)
    }

    pub fn bool(&mut self) -> bool {
        self.next_u64() & 1 == 1
    }

    pub fn pick<'a, T>(&mut self, xs: &'a [T]) -> &'a T {
        &xs[self.below(xs.len())]
    }

    /// Random byte string from a printable, regex-inert alphabet.
    pub fn word(&mut self, min: usize, max: usize) -> Vec<u8> {
        const AL: &[u8] = b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789-_.~/ ";
        let n = self.range(min, max);
        (0..n).map(|_| *self.pick(AL)).collect()
    }

    /// Random word guaranteed to contain no ARCHS substring and no separator.
    pub fn safe_word(&mut self, min: usize, max: usize) -> Vec<u8> {
        loop {
            let w = self.word(min, max);
            if !contains_any_arch(&w)
                && !find_sub(&w, b" [").is_some()
                && !find_sub(&w, b": ").is_some()
                && !find_sub(&w, b" (").is_some()
                && !w.contains(&b'|')
            {
                return w;
            }
        }
    }

    pub fn digits(&mut self, min: usize, max: usize) -> Vec<u8> {
        let n = self.range(min, max);
        (0..n).map(|_| b'0' + (self.below(10) as u8)).collect()
    }
}

// ---------------------------------------------------------------------------
// Reference data straight out of the C source
// ---------------------------------------------------------------------------

/// `ARCHS[]` from `c_src/src/lib.c:18`, in the exact source order.
pub const ARCHS: [&[u8]; 12] = [
    b"x86_64", b"i386", b"i686", b"sparc", b"amd64", b"i86pc", b"ia64", b"AIX", b"armv6", b"armv7",
    b"aarch64", b"arm64",
];

/// The three regex patterns `parse_uname_string` uses.
pub const PAT_MAJOR: &[u8] = br"^([0-9]+)\.*";
pub const PAT_MINOR: &[u8] = br"^[0-9]+\.([0-9]+)\.*";
pub const PAT_BUILD: &[u8] = br"^[0-9]+\.[0-9]+\.([0-9]+(\.[0-9]+)*)\.*";

pub fn find_sub(hay: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.is_empty() {
        return Some(0);
    }
    hay.windows(needle.len()).position(|w| w == needle)
}

pub fn contains_any_arch(s: &[u8]) -> bool {
    ARCHS.iter().any(|a| find_sub(s, a).is_some())
}

// ---------------------------------------------------------------------------
// Assertions
// ---------------------------------------------------------------------------

/// Differential assertion for `parse_uname_string`.
pub fn assert_parse_eq(row: &str, uname: &[u8]) {
    let (c, r) = both();
    let a = c.parse(uname);
    let b = r.parse(uname);
    if a != b {
        panic!(
            "[{row}] parse_uname_string DIVERGED\n uname = {:?}\n\n--- C fields ---\n{}\
             --- Rust fields ---\n{}\n C buffer    = {:?}\n Rust buffer = {:?}\n",
            String::from_utf8_lossy(uname),
            a.snapshot.describe(),
            b.snapshot.describe(),
            a.buffer,
            b.buffer,
        );
    }
}

/// Differential assertion using a sentinel-filled `os_data`, which also proves
/// exactly which fields each branch writes.
pub fn assert_parse_sent_eq(row: &str, uname: &[u8]) {
    let (c, r) = both();
    let sc = Sentinels::new();
    let sr = Sentinels::new();
    let a = c.parse_sent(uname, &sc);
    let b = r.parse_sent(uname, &sr);
    if a != b {
        panic!(
            "[{row}] parse_uname_string (sentinelled) DIVERGED\n uname = {:?}\n\n\
             --- C fields ---\n{}--- Rust fields ---\n{}\n C buffer    = {:?}\n Rust buffer = {:?}\n",
            String::from_utf8_lossy(uname),
            a.snapshot.describe(),
            b.snapshot.describe(),
            a.buffer,
            b.buffer,
        );
    }
}

/// Both flavours (zeroed + sentinelled) in one call.
pub fn assert_parse_both_inits(row: &str, uname: &[u8]) {
    assert_parse_eq(row, uname);
    assert_parse_sent_eq(row, uname);
}

pub fn assert_arch_eq(row: &str, input: &[u8]) {
    let (c, r) = both();
    let a = c.get_arch(input);
    let b = r.get_arch(input);
    assert_eq!(
        a,
        b,
        "[{row}] get_os_arch DIVERGED on {:?}: C={:?} Rust={:?}",
        String::from_utf8_lossy(input),
        a.as_ref().map(|v| String::from_utf8_lossy(v).to_string()),
        b.as_ref().map(|v| String::from_utf8_lossy(v).to_string()),
    );
}

pub fn assert_regexec_eq(
    row: &str,
    pattern: Option<&[u8]>,
    string: Option<&[u8]>,
    nmatch: usize,
    init: &[regmatch_t],
) {
    let (c, r) = both();
    let (ra, pa) = c.regexec(pattern, string, nmatch, init);
    let (rb, pb) = r.regexec(pattern, string, nmatch, init);
    assert_eq!(
        ra,
        rb,
        "[{row}] w_regexec RETURN DIVERGED: pat={:?} str={:?} nmatch={} C={} Rust={}",
        pattern.map(String::from_utf8_lossy),
        string.map(String::from_utf8_lossy),
        nmatch,
        ra,
        rb
    );
    assert_eq!(
        pa, pb,
        "[{row}] w_regexec PMATCH DIVERGED: pat={:?} str={:?} nmatch={}\n C   ={:?}\n Rust={:?}",
        pattern.map(String::from_utf8_lossy),
        string.map(String::from_utf8_lossy),
        nmatch,
        pa,
        pb
    );
}

pub const PM_INIT: regmatch_t = regmatch_t {
    rm_so: 0x5A5A5A5A,
    rm_eo: 0x3C3C3C3C,
};

pub fn pm_init(n: usize) -> Vec<regmatch_t> {
    vec![PM_INIT; n]
}
