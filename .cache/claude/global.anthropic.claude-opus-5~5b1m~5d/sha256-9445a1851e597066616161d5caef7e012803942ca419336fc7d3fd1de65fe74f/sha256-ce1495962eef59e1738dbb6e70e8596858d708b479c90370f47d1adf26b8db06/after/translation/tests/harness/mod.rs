//! Differential harness: loads BOTH the C `.so` and the Rust `.so` with
//! `libloading` and drives them exclusively through their exported symbols.
//!
//! The reference C library is built without `NDEBUG`, so a failing `assert()`
//! kills the process with `SIGABRT`. Every call is therefore executed in a
//! forked child, and a batch of cases is replayed from the case after the one
//! that killed the child. That way "both abort" and "both return error code X"
//! are distinguished from each other.

#![allow(dead_code)]

pub mod make;

use libloading::{Library, Symbol};
use std::ffi::{CStr, c_char, c_int, c_void};
use std::path::PathBuf;

// ---------------------------------------------------------------------------
// libc bits we need for the fork/pipe plumbing and for allocating the buffers
// the two libraries see (identical alignment + identical padding contents, so
// that the C's deliberate out-of-bounds *reads* are deterministic).
// ---------------------------------------------------------------------------

unsafe extern "C" {
    fn fork() -> i32;
    fn pipe(fds: *mut c_int) -> c_int;
    fn read(fd: c_int, buf: *mut c_void, n: usize) -> isize;
    fn write(fd: c_int, buf: *const c_void, n: usize) -> isize;
    fn close(fd: c_int) -> c_int;
    fn waitpid(pid: i32, status: *mut c_int, opts: c_int) -> i32;
    fn _exit(code: c_int) -> !;
    fn malloc(n: usize) -> *mut c_void;
    fn free(p: *mut c_void);
    fn dup2(old: c_int, new: c_int) -> c_int;
    fn alarm(seconds: u32) -> u32;
    fn setrlimit(resource: c_int, rlim: *const RLimit) -> c_int;
    #[link_name = "open"]
    fn libc_open(path: *const c_char, flags: c_int, ...) -> c_int;
}

#[repr(C)]
struct RLimit {
    cur: u64,
    max: u64,
}

/// `RLIMIT_CORE` on Linux. Every failing `assert()` in the C library aborts, and
/// with the default `core_pattern` piping to `systemd-coredump` each abort costs
/// ~150 ms. Turning core dumps off in the child makes the error-path tests ~30x
/// faster and changes nothing observable.
const RLIMIT_CORE: c_int = 4;

/// Per-case watchdog, in seconds (`SIGALRM` = 14 if it fires).
pub const CASE_TIMEOUT_SECS: u32 = 3;

unsafe fn disable_core_dumps() {
    unsafe {
        let z = RLimit { cur: 0, max: 0 };
        setrlimit(RLIMIT_CORE, &z);
    }
}

unsafe fn open_devnull() -> c_int {
    unsafe { libc_open(c"/dev/null".as_ptr(), 1 /* O_WRONLY */) }
}

// ---------------------------------------------------------------------------
// Public ABI of the library under test
// ---------------------------------------------------------------------------

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct CpImage {
    pub w: c_int,
    pub h: c_int,
    pub pix: *mut u8,
}

pub type FnLoadPng = unsafe extern "C" fn(*const u8, c_int) -> CpImage;
pub type FnInflate = unsafe extern "C" fn(*mut c_void, c_int, *mut c_void, c_int) -> c_int;

/// The six mutable lookup tables the public API exposes.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Table {
    FixedTable,
    PermutationOrder,
    LenExtraBits,
    LenBase,
    DistExtraBits,
    DistBase,
}

impl Table {
    pub const ALL: [Table; 6] = [
        Table::FixedTable,
        Table::PermutationOrder,
        Table::LenExtraBits,
        Table::LenBase,
        Table::DistExtraBits,
        Table::DistBase,
    ];
    /// `nm -D` sizes of the corresponding objects.
    pub fn byte_len(self) -> usize {
        match self {
            Table::FixedTable => 320,
            Table::PermutationOrder => 19,
            Table::LenExtraBits => 31,
            Table::LenBase => 124,
            Table::DistExtraBits => 32,
            Table::DistBase => 128,
        }
    }
    pub fn symbol(self) -> &'static [u8] {
        match self {
            Table::FixedTable => b"cp_fixed_table\0",
            Table::PermutationOrder => b"cp_permutation_order\0",
            Table::LenExtraBits => b"cp_len_extra_bits\0",
            Table::LenBase => b"cp_len_base\0",
            Table::DistExtraBits => b"cp_dist_extra_bits\0",
            Table::DistBase => b"cp_dist_base\0",
        }
    }
}

pub struct Lib {
    _lib: Library,
    pub name: &'static str,
    pub load_png: FnLoadPng,
    pub inflate: FnInflate,
    pub err: *mut *const c_char,
    tables: [*mut u8; 6],
}

impl Lib {
    pub fn table_ptr(&self, t: Table) -> *mut u8 {
        self.tables[Table::ALL.iter().position(|x| *x == t).unwrap()]
    }
}

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf()
}

pub fn c_lib_path() -> PathBuf {
    let dir = workspace_root().join("c_src/build");
    let mut found: Vec<PathBuf> = std::fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}. Build the C library first.", dir.display()))
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().map(|x| x == "so").unwrap_or(false))
        .collect();
    found.sort();
    assert_eq!(found.len(), 1, "expected exactly one .so in {}", dir.display());
    found.pop().unwrap()
}

pub fn rust_lib_path() -> PathBuf {
    let p = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/release/libload_png_mem_lib.so");
    assert!(
        p.exists(),
        "{} missing -- run `cargo build --release` first",
        p.display()
    );
    p
}

unsafe fn open(path: &PathBuf, name: &'static str) -> Lib {
    unsafe {
        let lib = Library::new(path).unwrap_or_else(|e| panic!("dlopen {}: {e}", path.display()));
        let load_png: Symbol<FnLoadPng> = lib.get(b"load_png_mem\0").expect("load_png_mem");
        let inflate: Symbol<FnInflate> = lib.get(b"cp_inflate\0").expect("cp_inflate");
        let err: Symbol<*mut *const c_char> =
            lib.get(b"cp_error_reason\0").expect("cp_error_reason");
        let load_png = *load_png;
        let inflate = *inflate;
        let err = err.into_raw().into_raw() as *mut *const c_char;
        let mut tables = [std::ptr::null_mut(); 6];
        for (i, t) in Table::ALL.iter().enumerate() {
            let s: Symbol<*mut u8> = lib
                .get(t.symbol())
                .unwrap_or_else(|e| panic!("{name}: {:?}: {e}", t));
            tables[i] = s.into_raw().into_raw() as *mut u8;
        }
        Lib {
            _lib: lib,
            name,
            load_png,
            inflate,
            err,
            tables,
        }
    }
}

pub struct Pair {
    pub c: Lib,
    pub rust: Lib,
}

pub fn load_pair() -> Pair {
    unsafe {
        Pair {
            c: open(&c_lib_path(), "C"),
            rust: open(&rust_lib_path(), "Rust"),
        }
    }
}

// ---------------------------------------------------------------------------
// Cases
// ---------------------------------------------------------------------------

#[derive(Clone, Debug)]
pub struct Mutation {
    pub table: Table,
    /// byte offset inside the table object (may be past its end -- the C has no
    /// bounds check either, but the harness never does that)
    pub off: usize,
    pub val: u8,
}

#[derive(Clone, Debug)]
pub enum Call {
    /// `load_png_mem(buf, len)`. `buf` holds `data` followed by `pad` bytes of a
    /// deterministic filler, so the C's reads past `len` are reproducible.
    LoadPng {
        data: Vec<u8>,
        len: c_int,
        pad: usize,
        /// How many of the `w*h*4` output bytes to compare (`None` = all).
        compare_pixels: Option<usize>,
    },
    /// `cp_inflate(in + align, in_bytes, out, out_bytes)`.
    Inflate {
        input: Vec<u8>,
        in_bytes: c_int,
        /// `in % 4`, which drives `first_bytes` / the `cp_ptr` maths
        align: usize,
        out_bytes: c_int,
        /// extra bytes allocated (and compared) past `out_bytes`, so that
        /// out-of-bounds *writes* are compared too
        out_slack: usize,
    },
    /// `load_png_mem(NULL, len)` -- separate variant so no buffer is allocated.
    LoadPngNull { len: c_int },
}

#[derive(Clone, Debug)]
pub struct Case {
    pub label: String,
    pub mutations: Vec<Mutation>,
    pub call: Call,
    /// Replace the (potentially huge) output blob with a 64-bit FNV-1a digest.
    pub digest: bool,
}

impl Case {
    pub fn png(label: impl Into<String>, data: Vec<u8>) -> Case {
        let len = data.len() as c_int;
        Case {
            label: label.into(),
            mutations: Vec::new(),
            digest: false,
            call: Call::LoadPng {
                data,
                len,
                pad: 1024,
                compare_pixels: None,
            },
        }
    }
    pub fn png_len(label: impl Into<String>, data: Vec<u8>, len: c_int) -> Case {
        Case {
            label: label.into(),
            mutations: Vec::new(),
            digest: false,
            call: Call::LoadPng {
                data,
                len,
                pad: 1024,
                compare_pixels: None,
            },
        }
    }
    pub fn inflate(
        label: impl Into<String>,
        input: Vec<u8>,
        align: usize,
        out_bytes: c_int,
    ) -> Case {
        let in_bytes = input.len() as c_int;
        Case {
            label: label.into(),
            mutations: Vec::new(),
            digest: false,
            call: Call::Inflate {
                input,
                in_bytes,
                align,
                out_bytes,
                out_slack: 32,
            },
        }
    }
    pub fn with_mutations(mut self, m: Vec<Mutation>) -> Case {
        self.mutations = m;
        self
    }
    pub fn compare_pixels(mut self, n: usize) -> Case {
        if let Call::LoadPng {
            ref mut compare_pixels,
            ..
        } = self.call
        {
            *compare_pixels = Some(n);
        }
        self
    }
}

/// Deterministic filler for the bytes the C reads out of bounds.
fn filler(i: usize) -> u8 {
    (i.wrapping_mul(97).wrapping_add(29) & 0xFF) as u8
}

/// 16-byte-aligned allocation with deterministic content, returned as
/// `(free_me, aligned_ptr)`. 32 bytes on either side of `[aligned, aligned+n)`
/// are filled too, because `cp_ptr` can legitimately compute a pointer *before*
/// the input buffer and `cp_stored`/`cp_peak_bits` read past its end; filling
/// them makes those out-of-bounds reads identical in both libraries.
unsafe fn alloc_filled(n: usize, seed: usize) -> (*mut c_void, *mut u8) {
    unsafe {
        let raw = malloc(n + 160);
        assert!(!raw.is_null());
        let aligned = (((raw as usize) + 48) & !15usize) as *mut u8;
        for i in -48i64..(n as i64 + 48) {
            *aligned.offset(i as isize) = filler(seed.wrapping_add((i + 64) as usize));
        }
        (raw, aligned)
    }
}

/// FNV-1a, used to keep fuzz payloads small.
pub fn fnv1a(data: &[u8]) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for &b in data {
        h ^= b as u64;
        h = h.wrapping_mul(0x100_0000_01b3);
    }
    h
}

fn push_blob(v: &mut Vec<u8>, blob: &[u8], digest: bool) {
    if digest {
        v.extend_from_slice(&(blob.len() as u64).to_le_bytes());
        v.extend_from_slice(&fnv1a(blob).to_le_bytes());
    } else {
        v.extend_from_slice(blob);
    }
}

unsafe fn err_bytes(lib: &Lib) -> Vec<u8> {
    unsafe {
        let p = *lib.err;
        if p.is_null() {
            b"<null>".to_vec()
        } else {
            CStr::from_ptr(p).to_bytes().to_vec()
        }
    }
}

/// Applies the mutations and returns the previous bytes so they can be put
/// back. Restoring matters: a child runs many cases in a row, and the C and
/// Rust children restart at different points (whenever one of them aborts), so
/// leaked mutations would make the two disagree for reasons that have nothing
/// to do with the translation.
unsafe fn apply(lib: &Lib, ms: &[Mutation]) -> Vec<(*mut u8, u8)> {
    unsafe {
        let mut saved = Vec::with_capacity(ms.len());
        for m in ms {
            assert!(m.off < m.table.byte_len(), "mutation out of table bounds");
            let p = lib.table_ptr(m.table).add(m.off);
            saved.push((p, *p));
            *p = m.val;
        }
        saved
    }
}

unsafe fn restore(saved: &[(*mut u8, u8)]) {
    unsafe {
        for (p, v) in saved.iter().rev() {
            **p = *v;
        }
    }
}

/// Runs one case against one library and returns the serialized observable
/// result. Executed inside the forked child.
pub unsafe fn exec_case(lib: &Lib, case: &Case) -> Vec<u8> {
    unsafe {
        let saved = apply(lib, &case.mutations);
        let mut v = Vec::new();
        match &case.call {
            Call::LoadPngNull { len } => {
                *lib.err = std::ptr::null();
                let img = (lib.load_png)(std::ptr::null(), *len);
                v.extend_from_slice(&img.w.to_le_bytes());
                v.extend_from_slice(&img.h.to_le_bytes());
                v.push(u8::from(!img.pix.is_null()));
                v.extend_from_slice(&err_bytes(lib));
            }
            Call::LoadPng {
                data,
                len,
                pad,
                compare_pixels,
            } => {
                let total = data.len() + pad;
                let (raw, buf) = alloc_filled(total, 0x1000);
                std::ptr::copy_nonoverlapping(data.as_ptr(), buf, data.len());
                *lib.err = std::ptr::null();
                let img = (lib.load_png)(buf, *len);
                v.extend_from_slice(&img.w.to_le_bytes());
                v.extend_from_slice(&img.h.to_le_bytes());
                v.push(u8::from(!img.pix.is_null()));
                if !img.pix.is_null() {
                    let full = (img.w as i64 * img.h as i64 * 4).max(0) as usize;
                    let n = compare_pixels.map(|c| c.min(full)).unwrap_or(full);
                    push_blob(
                        &mut v,
                        std::slice::from_raw_parts(img.pix, n),
                        case.digest,
                    );
                    free(img.pix as *mut c_void);
                }
                v.extend_from_slice(&err_bytes(lib));
                free(raw);
            }
            Call::Inflate {
                input,
                in_bytes,
                align,
                out_bytes,
                out_slack,
            } => {
                let (iraw, ibase) = alloc_filled(align + input.len() + 64, 0x2000);
                let inp = ibase.add(*align);
                std::ptr::copy_nonoverlapping(input.as_ptr(), inp, input.len());
                let osize = (*out_bytes).max(0) as usize + out_slack;
                let (oraw, obuf) = alloc_filled(osize, 0x3000);
                *lib.err = std::ptr::null();
                let rc = (lib.inflate)(
                    inp as *mut c_void,
                    *in_bytes,
                    obuf as *mut c_void,
                    *out_bytes,
                );
                v.extend_from_slice(&rc.to_le_bytes());
                push_blob(
                    &mut v,
                    std::slice::from_raw_parts(obuf, osize),
                    case.digest,
                );
                v.extend_from_slice(&err_bytes(lib));
                free(iraw);
                free(oraw);
            }
        }
        restore(&saved);
        v
    }
}

// ---------------------------------------------------------------------------
// Fork plumbing
// ---------------------------------------------------------------------------

#[derive(Clone, PartialEq, Eq)]
pub enum Outcome {
    /// The call returned; payload is the serialized observable result.
    Ret(Vec<u8>),
    /// The process died from a signal (6 = SIGABRT from a failed `assert`).
    Signal(i32),
    /// The process exited with a non-zero status without producing a result.
    Exit(i32),
}

impl std::fmt::Debug for Outcome {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Outcome::Ret(v) => {
                write!(f, "Ret(len={}, ", v.len())?;
                let head: Vec<String> = v.iter().take(24).map(|b| format!("{b:02x}")).collect();
                write!(f, "{}{})", head.join(""), if v.len() > 24 { ".." } else { "" })
            }
            Outcome::Signal(s) => write!(f, "Signal({s})"),
            Outcome::Exit(c) => write!(f, "Exit({c})"),
        }
    }
}

unsafe fn write_all(fd: c_int, buf: &[u8]) {
    unsafe {
        let mut off = 0usize;
        while off < buf.len() {
            let n = write(fd, buf.as_ptr().add(off) as *const c_void, buf.len() - off);
            if n <= 0 {
                _exit(3);
            }
            off += n as usize;
        }
    }
}

unsafe fn read_exact(fd: c_int, buf: &mut [u8]) -> bool {
    unsafe {
        let mut off = 0usize;
        while off < buf.len() {
            let n = read(fd, buf.as_mut_ptr().add(off) as *mut c_void, buf.len() - off);
            if n <= 0 {
                return false;
            }
            off += n as usize;
        }
        true
    }
}

/// Runs `cases[start..]` inside one forked child and collects whatever results
/// it managed to produce plus how it terminated.
unsafe fn fork_run(lib: &Lib, cases: &[Case], start: usize) -> (Vec<Vec<u8>>, Outcome) {
    unsafe {
        let mut fds = [0i32; 2];
        assert_eq!(pipe(fds.as_mut_ptr()), 0, "pipe");
        let pid = fork();
        assert!(pid >= 0, "fork");
        if pid == 0 {
            close(fds[0]);
            disable_core_dumps();
            // `assert()` prints to stderr before aborting; the harness compares
            // exit status, not the message, so keep the test output readable.
            if std::env::var_os("KEEP_ASSERT_OUTPUT").is_none() {
                let devnull = open_devnull();
                if devnull >= 0 {
                    dup2(devnull, 2);
                }
            }
            for c in &cases[start..] {
                // A per-case watchdog. Some malformed inputs make the C spin
                // for a very long time (e.g. a retuned `cp_len_base` yielding a
                // negative `int length`, whose `while (length--)` writes
                // gigabytes before faulting). Both libraries get the same
                // budget, so a timeout is just another comparable outcome
                // (`Signal(14)`).
                alarm(CASE_TIMEOUT_SECS);
                let bytes = exec_case(lib, c);
                alarm(0);
                write_all(fds[1], &(bytes.len() as u32).to_le_bytes());
                write_all(fds[1], &bytes);
            }
            close(fds[1]);
            _exit(0);
        }
        close(fds[1]);
        let mut out: Vec<Vec<u8>> = Vec::new();
        loop {
            let mut hdr = [0u8; 4];
            if !read_exact(fds[0], &mut hdr) {
                break;
            }
            let len = u32::from_le_bytes(hdr) as usize;
            let mut body = vec![0u8; len];
            if len > 0 && !read_exact(fds[0], &mut body) {
                break;
            }
            out.push(body);
        }
        close(fds[0]);
        let mut status: c_int = 0;
        assert!(waitpid(pid, &mut status, 0) == pid, "waitpid");
        let term = if status & 0x7f != 0 {
            Outcome::Signal(status & 0x7f)
        } else {
            Outcome::Exit((status >> 8) & 0xff)
        };
        (out, term)
    }
}

/// Runs every case against `lib`, isolating aborts.
pub fn run_all(lib: &Lib, cases: &[Case]) -> Vec<Outcome> {
    let mut res: Vec<Outcome> = Vec::with_capacity(cases.len());
    let mut start = 0usize;
    while start < cases.len() {
        let (produced, term) = unsafe { fork_run(lib, cases, start) };
        let got = produced.len();
        for p in produced {
            res.push(Outcome::Ret(p));
        }
        let idx = start + got;
        if idx >= cases.len() {
            // the child finished the batch; `term` should be Exit(0)
            assert_eq!(term, Outcome::Exit(0), "{}: child exit after full batch", lib.name);
            break;
        }
        res.push(term);
        start = idx + 1;
    }
    assert_eq!(res.len(), cases.len());
    res
}

/// Runs every case against both libraries and asserts identical outcomes.
pub fn assert_same(pair: &Pair, cases: &[Case]) {
    assert!(!cases.is_empty(), "no cases");
    let a = run_all(&pair.c, cases);
    let b = run_all(&pair.rust, cases);
    let mut failures = Vec::new();
    for (i, c) in cases.iter().enumerate() {
        if a[i] != b[i] {
            failures.push(format!(
                "  [{i}] {}\n      C    = {:?}\n      Rust = {:?}",
                c.label, a[i], b[i]
            ));
        }
    }
    if !failures.is_empty() {
        panic!(
            "{} of {} cases diverged:\n{}",
            failures.len(),
            cases.len(),
            failures
                .iter()
                .take(20)
                .cloned()
                .collect::<Vec<_>>()
                .join("\n")
        );
    }
}

/// Same, but also returns the outcomes so a test can assert *what* happened
/// (e.g. "both aborted", "both returned this error string").
pub fn run_same(pair: &Pair, cases: &[Case]) -> Vec<Outcome> {
    assert_same(pair, cases);
    run_all(&pair.c, cases)
}

pub fn outcome_err(o: &Outcome) -> Option<String> {
    match o {
        Outcome::Ret(v) => {
            // the error string is the tail of the payload
            let s = String::from_utf8_lossy(v);
            Some(s.to_string())
        }
        _ => None,
    }
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (xorshift64*) so every row is driven by many inputs
// ---------------------------------------------------------------------------

pub struct Rng(pub u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng(seed | 1)
    }
    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545F4914F6CDD1D)
    }
    pub fn u32(&mut self) -> u32 {
        (self.next_u64() >> 32) as u32
    }
    pub fn below(&mut self, n: u32) -> u32 {
        if n == 0 { 0 } else { self.u32() % n }
    }
    pub fn range(&mut self, lo: u32, hi: u32) -> u32 {
        lo + self.below(hi - lo + 1)
    }
    pub fn byte(&mut self) -> u8 {
        (self.next_u64() >> 40) as u8
    }
    pub fn bytes(&mut self, n: usize) -> Vec<u8> {
        (0..n).map(|_| self.byte()).collect()
    }
    pub fn bool(&mut self) -> bool {
        self.next_u64() & 1 == 1
    }
}


// ---------------------------------------------------------------------------
// Fuzzing: only compare inputs the C library itself is deterministic on.
//
// A few of the C's behaviours are undefined *and* layout-dependent -- most
// notably `cp_stored`'s `memcpy` ignores `out_end`, so a malformed stream can
// smash the heap, and whether glibc notices depends on the heap layout rather
// than on anything the translation controls. Running the C corpus twice (from
// two differently-aged parent heaps) filters those out.
// ---------------------------------------------------------------------------

pub struct FuzzReport {
    pub compared: usize,
    pub dropped: usize,
}

pub fn fuzz_same(pair: &Pair, cases: &[Case]) -> FuzzReport {
    let c1 = run_all(&pair.c, cases);
    let c2 = run_all(&pair.c, cases);
    let r = run_all(&pair.rust, cases);
    let mut compared = 0usize;
    let mut dropped = 0usize;
    let mut failures = Vec::new();
    for (i, case) in cases.iter().enumerate() {
        if c1[i] != c2[i] {
            dropped += 1;
            continue;
        }
        compared += 1;
        if c1[i] != r[i] {
            failures.push(format!(
                "  [{i}] {} muts={:?} call={}\n      C    = {:?}\n      Rust = {:?}",
                case.label,
                case.mutations,
                match &case.call {
                    Call::Inflate { input, in_bytes, align, out_bytes, .. } => format!(
                        "inflate(align={align}, in_bytes={in_bytes}, out_bytes={out_bytes}, input={:02x?})",
                        input
                    ),
                    Call::LoadPng { data, len, .. } => format!("load_png(len={len}, data={:02x?})", data),
                    Call::LoadPngNull { len } => format!("load_png(NULL, {len})"),
                },
                c1[i],
                r[i]
            ));
        }
    }
    if !failures.is_empty() {
        panic!(
            "{} of {compared} deterministic cases diverged:\n{}",
            failures.len(),
            failures
                .iter()
                .take(20)
                .cloned()
                .collect::<Vec<_>>()
                .join("\n")
        );
    }
    FuzzReport { compared, dropped }
}
