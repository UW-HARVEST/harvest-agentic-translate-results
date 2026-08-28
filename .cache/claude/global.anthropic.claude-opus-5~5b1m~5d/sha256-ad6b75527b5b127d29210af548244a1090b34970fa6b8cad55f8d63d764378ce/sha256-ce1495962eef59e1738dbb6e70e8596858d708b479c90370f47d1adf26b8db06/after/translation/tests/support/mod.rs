//! Differential-test infrastructure.
//!
//! Both the reference C library and the Rust translation are loaded as shared
//! objects with `libloading` and driven exclusively through their exported
//! symbols, exactly like an external C consumer would.
//!
//! Several inputs make `c_src/src/lib.c` fail an `assert()` (the reference `.so`
//! is built with asserts live) or read wildly out of range, which aborts the
//! process.  To observe those outcomes the driver re-executes **itself** as a
//! worker process (`PINFLATE_WORKER=c|rust`), so a dying library only takes the
//! worker down.  The worker runs the cases of one test in order, appending one
//! record per finished case to a results file; when it dies, the driver knows
//! exactly which case killed it, records the signal plus the child's stderr, and
//! restarts the worker at the following case.
//!
//! Cases are *never* serialised: worker and driver both rebuild them from the
//! same deterministic (fixed-seed) generator, keyed by the test id.

#![allow(dead_code)]

pub mod cases;
pub mod deflate;

use std::ffi::{c_char, c_int, c_void, CStr};
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

// ---------------------------------------------------------------------------
// small deterministic RNG (SplitMix64) — fixed seeds everywhere
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
    pub fn next_u32(&mut self) -> u32 {
        (self.next_u64() >> 32) as u32
    }
    /// Uniform in `0..n` (n > 0).
    pub fn below(&mut self, n: usize) -> usize {
        (self.next_u64() % (n as u64)) as usize
    }
    pub fn range(&mut self, lo: usize, hi_inclusive: usize) -> usize {
        lo + self.below(hi_inclusive - lo + 1)
    }
    pub fn byte(&mut self) -> u8 {
        (self.next_u64() >> 24) as u8
    }
    pub fn bool(&mut self) -> bool {
        self.next_u64() & 1 == 1
    }
}

pub fn fnv64(bytes: &[u8]) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for &b in bytes {
        h ^= b as u64;
        h = h.wrapping_mul(0x100_0000_01b3);
    }
    h
}

// ---------------------------------------------------------------------------
// library paths
// ---------------------------------------------------------------------------

pub fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

pub fn repo_root() -> PathBuf {
    manifest_dir().parent().map(PathBuf::from).unwrap_or_else(manifest_dir)
}

/// The C reference `.so` produced by `c_src/CMakeLists.txt` (the library name is
/// derived from the parent directory name, so glob for it).
pub fn c_so_path() -> PathBuf {
    let dir = repo_root().join("c_src").join("build");
    let mut best: Option<PathBuf> = None;
    if let Ok(rd) = fs::read_dir(&dir) {
        for e in rd.flatten() {
            let p = e.path();
            let name = p.file_name().and_then(|s| s.to_str()).unwrap_or("");
            if name.starts_with("lib") && name.ends_with(".so") {
                best = Some(p);
            }
        }
    }
    best.unwrap_or_else(|| {
        panic!(
            "C shared library not found in {}.\nBuild it with:\n  cd c_src && mkdir -p build && \
             cd build && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
            dir.display()
        )
    })
}

/// The Rust `cdylib`.  `cargo test` does not emit it, so `run_verification.sh`
/// runs `cargo build` first; look in both profiles and prefer the newest.
pub fn rust_so_path() -> PathBuf {
    // explicit override, so the whole suite can be run against the debug and the
    // release cdylib in turn (see run_verification.sh)
    if let Ok(p) = std::env::var("PINFLATE_RUST_SO") {
        let p = PathBuf::from(p);
        assert!(p.exists(), "PINFLATE_RUST_SO={} does not exist", p.display());
        return p;
    }
    let mut cands: Vec<PathBuf> = Vec::new();
    for profile in ["debug", "release"] {
        for sub in ["", "deps"] {
            let mut p = manifest_dir().join("target").join(profile);
            if !sub.is_empty() {
                p = p.join(sub);
            }
            cands.push(p.join("libpinflate_lib.so"));
        }
    }
    // Also handle CARGO_TARGET_DIR.
    if let Ok(td) = std::env::var("CARGO_TARGET_DIR") {
        for profile in ["debug", "release"] {
            cands.push(PathBuf::from(&td).join(profile).join("libpinflate_lib.so"));
        }
    }
    let mut best: Option<(std::time::SystemTime, PathBuf)> = None;
    for c in cands {
        if let Ok(md) = fs::metadata(&c) {
            let t = md.modified().unwrap_or(std::time::UNIX_EPOCH);
            if best.as_ref().map(|(bt, _)| t > *bt).unwrap_or(true) {
                best = Some((t, c));
            }
        }
    }
    best.map(|(_, p)| p).unwrap_or_else(|| {
        panic!(
            "Rust cdylib libpinflate_lib.so not found under {}/target.\nBuild it with: cargo build",
            manifest_dir().display()
        )
    })
}

pub fn tmp_dir() -> PathBuf {
    let d = manifest_dir().join("target").join("diffwork");
    let _ = fs::create_dir_all(&d);
    d
}

// ---------------------------------------------------------------------------
// the loaded library
// ---------------------------------------------------------------------------

pub type PinflateFn = unsafe extern "C" fn(*mut c_void, c_int, *mut c_void, c_int) -> c_int;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Tbl {
    Fixed,
    Perm,
    LenExtra,
    LenBase,
    DistExtra,
    DistBase,
}

impl Tbl {
    pub fn name(self) -> &'static str {
        match self {
            Tbl::Fixed => "cp_fixed_table",
            Tbl::Perm => "cp_permutation_order",
            Tbl::LenExtra => "cp_len_extra_bits",
            Tbl::LenBase => "cp_len_base",
            Tbl::DistExtra => "cp_dist_extra_bits",
            Tbl::DistBase => "cp_dist_base",
        }
    }
    /// element size in bytes, element count
    pub fn shape(self) -> (usize, usize) {
        match self {
            Tbl::Fixed => (1, 288 + 32),
            Tbl::Perm => (1, 19),
            Tbl::LenExtra => (1, 29 + 2),
            Tbl::LenBase => (4, 29 + 2),
            Tbl::DistExtra => (1, 30 + 2),
            Tbl::DistBase => (4, 30 + 2),
        }
    }
    pub fn all() -> [Tbl; 6] {
        [Tbl::Fixed, Tbl::Perm, Tbl::LenExtra, Tbl::LenBase, Tbl::DistExtra, Tbl::DistBase]
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Patch {
    pub tbl: Tbl,
    pub idx: usize,
    pub val: u32,
}

pub struct Api {
    pub which: &'static str,
    pub path: PathBuf,
    _lib: libloading::Library,
    pinflate: libloading::os::unix::Symbol<PinflateFn>,
    pub error_reason: *mut *const c_char,
    tables: [*mut u8; 6],
    snapshot: Vec<Vec<u8>>,
}

impl Api {
    pub fn load(which: &'static str, path: &Path) -> Api {
        unsafe {
            let lib = libloading::Library::new(path)
                .unwrap_or_else(|e| panic!("dlopen({}) failed: {e}", path.display()));
            let f: libloading::Symbol<PinflateFn> = lib
                .get(b"pinflate\0")
                .unwrap_or_else(|e| panic!("{}: no `pinflate`: {e}", path.display()));
            let pinflate = f.into_raw();

            let er: libloading::Symbol<*mut *const c_char> = lib
                .get(b"cp_error_reason\0")
                .unwrap_or_else(|e| panic!("{}: no `cp_error_reason`: {e}", path.display()));
            let error_reason: *mut *const c_char = *er;

            let mut tables = [std::ptr::null_mut(); 6];
            let mut snapshot = Vec::new();
            for (i, t) in Tbl::all().iter().enumerate() {
                let mut nm = t.name().as_bytes().to_vec();
                nm.push(0);
                let s: libloading::Symbol<*mut u8> = lib
                    .get(&nm)
                    .unwrap_or_else(|e| panic!("{}: no `{}`: {e}", path.display(), t.name()));
                tables[i] = *s;
                let (esz, n) = t.shape();
                snapshot.push(std::slice::from_raw_parts(tables[i], esz * n).to_vec());
            }

            Api {
                which,
                path: path.to_path_buf(),
                _lib: lib,
                pinflate,
                error_reason,
                tables,
                snapshot,
            }
        }
    }

    pub fn table_ptr(&self, t: Tbl) -> *mut u8 {
        self.tables[Tbl::all().iter().position(|x| *x == t).unwrap()]
    }

    pub fn table_bytes(&self, t: Tbl) -> Vec<u8> {
        let (esz, n) = t.shape();
        unsafe { std::slice::from_raw_parts(self.table_ptr(t), esz * n).to_vec() }
    }

    pub fn apply(&self, p: &Patch) {
        let (esz, n) = p.tbl.shape();
        assert!(p.idx < n, "patch index out of range for {}", p.tbl.name());
        unsafe {
            let base = self.table_ptr(p.tbl);
            if esz == 1 {
                *base.add(p.idx) = p.val as u8;
            } else {
                *(base as *mut u32).add(p.idx) = p.val;
            }
        }
    }

    pub fn restore_tables(&self) {
        for (i, t) in Tbl::all().iter().enumerate() {
            let (esz, n) = t.shape();
            unsafe {
                std::ptr::copy_nonoverlapping(self.snapshot[i].as_ptr(), self.tables[i], esz * n);
            }
        }
    }

    pub fn set_reason(&self, p: *const c_char) {
        unsafe { *self.error_reason = p };
    }

    pub fn get_reason(&self) -> Option<Vec<u8>> {
        unsafe {
            let p = *self.error_reason;
            if p.is_null() {
                None
            } else {
                Some(CStr::from_ptr(p).to_bytes().to_vec())
            }
        }
    }

    pub unsafe fn call(
        &self,
        input: *mut c_void,
        in_bytes: c_int,
        out: *mut c_void,
        out_bytes: c_int,
    ) -> c_int {
        (self.pinflate)(input, in_bytes, out, out_bytes)
    }
}

// ---------------------------------------------------------------------------
// padded buffers with controlled alignment
// ---------------------------------------------------------------------------

/// A heap buffer whose *interior* pointer has a chosen value mod 4 and which is
/// surrounded by `pad` deterministically filled bytes, so that the C code's
/// out-of-range reads/writes stay inside a real allocation and are reproducible
/// across processes.
pub struct PaddedBuf {
    mem: Vec<u8>,
    off: usize,
    len: usize,
}

impl PaddedBuf {
    pub fn new(content: &[u8], pad: usize, align_mod4: usize, fill_seed: u64) -> PaddedBuf {
        let total = pad * 2 + content.len() + 8;
        let mut mem = vec![0u8; total];
        // deterministic filler, 8 bytes per RNG step (the padding can be large,
        // and it is regenerated for every case in every worker)
        let mut r = Rng::new(fill_seed);
        let mut it = mem.chunks_exact_mut(8);
        for c in &mut it {
            c.copy_from_slice(&r.next_u64().to_le_bytes());
        }
        let rest = it.into_remainder();
        let last = r.next_u64().to_le_bytes();
        for (i, b) in rest.iter_mut().enumerate() {
            *b = last[i];
        }
        let base = mem.as_ptr() as usize;
        let mut off = pad;
        while (base + off) % 4 != align_mod4 % 4 {
            off += 1;
        }
        mem[off..off + content.len()].copy_from_slice(content);
        PaddedBuf { mem, off, len: content.len() }
    }

    pub fn ptr(&mut self) -> *mut c_void {
        unsafe { self.mem.as_mut_ptr().add(self.off) as *mut c_void }
    }
    pub fn all(&self) -> &[u8] {
        &self.mem
    }
    pub fn content(&self) -> &[u8] {
        &self.mem[self.off..self.off + self.len]
    }
    pub fn offset(&self) -> usize {
        self.off
    }
    pub fn align_mod4(&self) -> usize {
        (self.mem.as_ptr() as usize + self.off) % 4
    }
}

// ---------------------------------------------------------------------------
// cases and outcomes
// ---------------------------------------------------------------------------

/// What the C library is expected to do — checked against the *C* outcome so
/// that a test cannot silently stop exercising the row it was written for.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Expect {
    /// no expectation beyond "C and Rust agree"
    Any,
    /// returns `ret`; `reason` is the `cp_error_reason` C string afterwards
    Ret { ret: i32, reason: Option<&'static str> },
    /// returns `ret` and produces exactly these output bytes
    Out { ret: i32, out: Vec<u8> },
    /// `assert()` failure: SIGABRT + this exact assertion message
    Assert { line: u32, func: &'static str, expr: &'static str },
    /// died from this signal (6 = SIGABRT, 11 = SIGSEGV)
    Signal(i32),
}

#[derive(Clone, Debug)]
pub struct Case {
    pub label: String,
    /// bytes placed in the padded input buffer
    pub input: Vec<u8>,
    /// value passed as `in_bytes` (defaults to `input.len()`)
    pub in_bytes: i32,
    pub in_align: usize,
    pub in_pad: usize,
    /// size of the *real* output allocation (padding included)
    pub out_pad: usize,
    /// value passed as `out_bytes`
    pub out_bytes: i32,
    pub out_align: usize,
    /// pass a null pointer instead of the input buffer
    pub in_null: bool,
    pub out_null: bool,
    pub patches: Vec<Patch>,
    /// pre-set `cp_error_reason` to a non-null sentinel before the call
    pub preset_reason: bool,
    /// call `pinflate` this many times in a row on the same buffers (checks that
    /// nothing leaks between calls); the reported outcome is the last call's
    pub calls: usize,
    pub expect: Expect,
}

impl Case {
    pub fn new(label: impl Into<String>, input: Vec<u8>, out_bytes: i32) -> Case {
        let n = input.len() as i32;
        Case {
            label: label.into(),
            input,
            in_bytes: n,
            in_align: 0,
            in_pad: 64,
            out_pad: (out_bytes.max(0) as usize) + 4096,
            out_bytes,
            out_align: 0,
            in_null: false,
            out_null: false,
            patches: Vec::new(),
            preset_reason: false,
            calls: 1,
            expect: Expect::Any,
        }
    }
    pub fn in_align(mut self, a: usize) -> Case {
        self.in_align = a;
        self
    }
    pub fn out_align(mut self, a: usize) -> Case {
        self.out_align = a;
        self
    }
    pub fn in_bytes(mut self, n: i32) -> Case {
        self.in_bytes = n;
        self
    }
    pub fn in_pad(mut self, n: usize) -> Case {
        self.in_pad = n;
        self
    }
    pub fn out_pad(mut self, n: usize) -> Case {
        self.out_pad = n;
        self
    }
    pub fn in_null(mut self) -> Case {
        self.in_null = true;
        self
    }
    pub fn out_null(mut self) -> Case {
        self.out_null = true;
        self
    }
    pub fn patch(mut self, tbl: Tbl, idx: usize, val: u32) -> Case {
        self.patches.push(Patch { tbl, idx, val });
        self
    }
    pub fn preset_reason(mut self) -> Case {
        self.preset_reason = true;
        self
    }
    pub fn calls(mut self, n: usize) -> Case {
        self.calls = n;
        self
    }
    pub fn expect(mut self, e: Expect) -> Case {
        self.expect = e;
        self
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Outcome {
    Ok {
        ret: i32,
        reason: Option<Vec<u8>>,
        /// hash of the whole padded output allocation (catches out-of-range writes)
        out_hash: u64,
        /// the declared output region, if small enough to keep around
        out: Option<Vec<u8>>,
        /// the tables afterwards (hash) — catches a library writing its own globals
        tables_hash: u64,
    },
    Died {
        signal: i32,
        stderr: Vec<u8>,
    },
    Exited {
        code: i32,
        stderr: Vec<u8>,
    },
    /// the library did not return within the worker timeout (e.g. `cp_dynamic`'s
    /// unbounded repeat loop)
    TimedOut,
}

const KEEP_OUT_LIMIT: usize = 1 << 18;
pub const PRESET_REASON_TEXT: &[u8] = b"<<untouched sentinel>>\0";

/// Runs one case in-process; only ever called inside a worker process.
fn run_case(api: &Api, c: &Case) -> Outcome {
    let mut inbuf = PaddedBuf::new(&c.input, c.in_pad, c.in_align, 0x5EED_1234_ABCD_0001);
    let out_content = vec![0u8; 0];
    let mut outbuf = PaddedBuf::new(&out_content, c.out_pad, c.out_align, 0x5EED_1234_ABCD_0002);

    for p in &c.patches {
        api.apply(p);
    }
    if c.preset_reason {
        api.set_reason(PRESET_REASON_TEXT.as_ptr() as *const c_char);
    } else {
        api.set_reason(std::ptr::null());
    }

    let inp = if c.in_null { std::ptr::null_mut() } else { inbuf.ptr() };
    let outp = if c.out_null { std::ptr::null_mut() } else { outbuf.ptr() };

    let mut ret = unsafe { api.call(inp, c.in_bytes, outp, c.out_bytes) };
    for _ in 1..c.calls {
        // no clearing of cp_error_reason and no re-initialisation of the output
        // buffer between calls: exactly what a real consumer that decompresses
        // several streams in a row sees
        ret = unsafe { api.call(inp, c.in_bytes, outp, c.out_bytes) };
    }
    let reason = api.get_reason();

    let out_hash = fnv64(outbuf.all());
    let declared = if c.out_null {
        Some(Vec::new())
    } else if c.out_bytes >= 0 && (c.out_bytes as usize) <= KEEP_OUT_LIMIT {
        let n = c.out_bytes as usize;
        let off = outbuf.offset();
        let all = outbuf.all();
        Some(all[off..off + n.min(all.len() - off)].to_vec())
    } else {
        None
    };
    let mut tb = Vec::new();
    for t in Tbl::all() {
        tb.extend_from_slice(&api.table_bytes(t));
    }
    let tables_hash = fnv64(&tb);

    api.restore_tables();
    api.set_reason(std::ptr::null());

    Outcome::Ok { ret, reason, out_hash, out: declared, tables_hash }
}

// ---------------------------------------------------------------------------
// worker protocol
// ---------------------------------------------------------------------------

fn put32(buf: &mut Vec<u8>, v: u32) {
    buf.extend_from_slice(&v.to_le_bytes());
}
fn put_blob(buf: &mut Vec<u8>, b: Option<&[u8]>) {
    match b {
        None => put32(buf, u32::MAX),
        Some(b) => {
            put32(buf, b.len() as u32);
            buf.extend_from_slice(b);
        }
    }
}

/// One length-prefixed, tagged record per case.
fn write_record(f: &mut fs::File, idx: usize, o: &Outcome) {
    let mut buf: Vec<u8> = Vec::new();
    put32(&mut buf, idx as u32);
    match o {
        Outcome::Ok { ret, reason, out_hash, out, tables_hash } => {
            buf.push(0);
            put32(&mut buf, *ret as u32);
            buf.extend_from_slice(&out_hash.to_le_bytes());
            buf.extend_from_slice(&tables_hash.to_le_bytes());
            put_blob(&mut buf, reason.as_deref());
            put_blob(&mut buf, out.as_deref());
        }
        Outcome::Died { signal, stderr } => {
            buf.push(1);
            put32(&mut buf, *signal as u32);
            put_blob(&mut buf, Some(stderr));
        }
        Outcome::TimedOut => buf.push(2),
        Outcome::Exited { code, stderr } => {
            buf.push(3);
            put32(&mut buf, *code as u32);
            put_blob(&mut buf, Some(stderr));
        }
    }
    let mut rec = (buf.len() as u32).to_le_bytes().to_vec();
    rec.extend_from_slice(&buf);
    f.write_all(&rec).expect("write record");
    f.flush().expect("flush record");
}

fn parse_records(path: &Path) -> Vec<(usize, Outcome)> {
    let mut v = Vec::new();
    let mut data = Vec::new();
    if let Ok(mut f) = fs::File::open(path) {
        let _ = f.read_to_end(&mut data);
    }
    let mut i = 0usize;
    while i + 4 <= data.len() {
        let len = u32::from_le_bytes([data[i], data[i + 1], data[i + 2], data[i + 3]]) as usize;
        i += 4;
        if i + len > data.len() || len < 5 {
            break; // truncated by a crash mid-write
        }
        let b = &data[i..i + len];
        i += len;
        let mut j = 0usize;
        let rd32 = |b: &[u8], j: &mut usize| -> u32 {
            let v = u32::from_le_bytes([b[*j], b[*j + 1], b[*j + 2], b[*j + 3]]);
            *j += 4;
            v
        };
        let rd64 = |b: &[u8], j: &mut usize| -> u64 {
            let mut a = [0u8; 8];
            a.copy_from_slice(&b[*j..*j + 8]);
            *j += 8;
            u64::from_le_bytes(a)
        };
        let rd_blob = |b: &[u8], j: &mut usize| -> Option<Vec<u8>> {
            let l = u32::from_le_bytes([b[*j], b[*j + 1], b[*j + 2], b[*j + 3]]);
            *j += 4;
            if l == u32::MAX {
                None
            } else {
                let r = b[*j..*j + l as usize].to_vec();
                *j += l as usize;
                Some(r)
            }
        };
        let idx = rd32(b, &mut j) as usize;
        let tag = b[j];
        j += 1;
        let o = match tag {
            0 => {
                let ret = rd32(b, &mut j) as i32;
                let out_hash = rd64(b, &mut j);
                let tables_hash = rd64(b, &mut j);
                let reason = rd_blob(b, &mut j);
                let out = rd_blob(b, &mut j);
                Outcome::Ok { ret, reason, out_hash, out, tables_hash }
            }
            1 => {
                let signal = rd32(b, &mut j) as i32;
                let stderr = rd_blob(b, &mut j).unwrap_or_default();
                Outcome::Died { signal, stderr }
            }
            2 => Outcome::TimedOut,
            _ => {
                let code = rd32(b, &mut j) as i32;
                let stderr = rd_blob(b, &mut j).unwrap_or_default();
                Outcome::Exited { code, stderr }
            }
        };
        v.push((idx, o));
    }
    v
}

/// `struct rlimit`
#[repr(C)]
struct RLimit {
    rlim_cur: u64,
    rlim_max: u64,
}
extern "C" {
    fn setrlimit(resource: c_int, rlim: *const RLimit) -> c_int;
    fn prctl(option: c_int, a: u64, b: u64, c: u64, d: u64) -> c_int;
}
const RLIMIT_CORE: c_int = 4;
const PR_SET_DUMPABLE: c_int = 4;

/// Many of the differential cases make the library under test `abort()` or fault
/// on purpose.  Unless core dumps are suppressed, every one of them is handed to
/// `systemd-coredump` (see `/proc/sys/kernel/core_pattern`), which costs ~75 ms
/// per case; `PR_SET_DUMPABLE` is what actually stops the kernel from invoking
/// the core_pattern pipe.  Inherited across `fork()`.
pub fn disable_core_dumps() {
    unsafe {
        let l = RLimit { rlim_cur: 0, rlim_max: 0 };
        setrlimit(RLIMIT_CORE, &l);
        prctl(PR_SET_DUMPABLE, 0, 0, 0, 0);
    }
}

extern "C" {
    fn fork() -> c_int;
    fn waitpid(pid: c_int, status: *mut c_int, options: c_int) -> c_int;
    fn kill(pid: c_int, sig: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn _exit(code: c_int) -> !;
}
const WNOHANG: c_int = 1;

/// Seconds one *case* may run before it is considered hung (`cp_dynamic`'s
/// repeat codes can walk `n` backwards and loop forever on corrupt input).
/// Override with `PINFLATE_CASE_TIMEOUT`.
fn case_timeout() -> u64 {
    std::env::var("PINFLATE_CASE_TIMEOUT")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(5)
}

/// Worker process: runs the cases of one test against one library.
///
/// Each case runs in a `fork()`ed child, so an `assert()` abort or a fault only
/// takes the child down and the worker can keep going.  The worker is
/// single-threaded (it is a fresh process spawned by the driver), which is what
/// makes `fork()` without `exec()` safe here.
pub fn worker_main(which: String) {
    disable_core_dumps();
    let test = std::env::var("PINFLATE_TEST").expect("PINFLATE_TEST");
    let start: usize = std::env::var("PINFLATE_START").unwrap().parse().unwrap();
    let results = PathBuf::from(std::env::var("PINFLATE_RESULTS").unwrap());
    let case_err = PathBuf::from(std::env::var("PINFLATE_CASE_ERR").unwrap());

    let path = if which == "c" { c_so_path() } else { rust_so_path() };
    let api = Api::load(if which == "c" { "C" } else { "Rust" }, &path);

    let cases = cases::build(&test);
    let mut f = fs::OpenOptions::new().create(true).append(true).open(&results).unwrap();

    for i in start..cases.len() {
        use std::os::unix::io::AsRawFd;
        let ef = fs::File::create(&case_err).expect("create per-case stderr file");
        f.flush().expect("flush before fork");
        let pid = unsafe { fork() };
        if pid == 0 {
            // child: everything the library does happens here
            unsafe { dup2(ef.as_raw_fd(), 2) };
            let o = run_case(&api, &cases[i]);
            let mut cf =
                fs::OpenOptions::new().create(true).append(true).open(&results).unwrap();
            write_record(&mut cf, i, &o);
            drop(cf);
            unsafe { _exit(0) };
        }
        assert!(pid > 0, "fork failed");
        drop(ef);

        let mut status: c_int = 0;
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(case_timeout());
        let mut timed_out = false;
        loop {
            let r = unsafe { waitpid(pid, &mut status, WNOHANG) };
            if r == pid {
                break;
            }
            if r < 0 {
                break;
            }
            if std::time::Instant::now() > deadline {
                unsafe { kill(pid, 9) };
                unsafe { waitpid(pid, &mut status, 0) };
                timed_out = true;
                break;
            }
            std::thread::sleep(std::time::Duration::from_micros(100));
        }

        if timed_out {
            write_record(&mut f, i, &Outcome::TimedOut);
        } else if (status & 0x7f) != 0 && (status & 0x7f) != 0x7f {
            let signal = status & 0x7f;
            let stderr = fs::read(&case_err).unwrap_or_default();
            write_record(&mut f, i, &Outcome::Died { signal, stderr });
        } else {
            let code = (status >> 8) & 0xff;
            if code != 0 {
                let stderr = fs::read(&case_err).unwrap_or_default();
                write_record(&mut f, i, &Outcome::Exited { code, stderr });
            }
            // exit code 0: the child already appended its own Ok record
        }
    }
    std::process::exit(0);
}

/// Seconds a whole worker *process* may run before the driver gives up on it.
/// Individual cases are already bounded by `CASE_TIMEOUT`, so this only guards
/// against a hang in the case *generator*.
const WORKER_TIMEOUT: u64 = 900;

fn run_pass(which: &str, test: &str, ncases: usize) -> Vec<Outcome> {
    let exe = std::env::current_exe().expect("current_exe");
    // include the driver's pid so that a stale worker from an earlier, killed
    // run can never append to the file we are reading
    let pid = std::process::id();
    let results = tmp_dir().join(format!("res-{pid}-{test}-{which}.bin"));
    let errpath = tmp_dir().join(format!("err-{pid}-{test}-{which}.txt"));
    let caseerr = tmp_dir().join(format!("cerr-{pid}-{test}-{which}.txt"));
    let mut out: Vec<Outcome> = Vec::new();

    while out.len() < ncases {
        let _ = fs::remove_file(&results);
        let _ = fs::remove_file(&errpath);
        let errf = fs::File::create(&errpath).unwrap();
        let mut child = Command::new(&exe)
            .env("PINFLATE_WORKER", which)
            .env("PINFLATE_TEST", test)
            .env("PINFLATE_START", out.len().to_string())
            .env("PINFLATE_RESULTS", &results)
            .env("PINFLATE_CASE_ERR", &caseerr)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(errf)
            .spawn()
            .expect("spawn worker");

        // wait with a timeout, so an unbounded loop in the library under test
        // does not hang the whole suite
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(WORKER_TIMEOUT);
        let mut timed_out = false;
        let status = loop {
            match child.try_wait().expect("try_wait") {
                Some(s) => break s,
                None => {
                    if std::time::Instant::now() > deadline {
                        let _ = child.kill();
                        let s = child.wait().expect("wait after kill");
                        timed_out = true;
                        break s;
                    }
                    std::thread::sleep(std::time::Duration::from_micros(200));
                }
            }
        };

        let recs = parse_records(&results);
        if timed_out {
            let expect_first = out.len();
            for (k, (idx, o)) in recs.into_iter().enumerate() {
                assert_eq!(idx, expect_first + k, "worker wrote out-of-order records");
                out.push(o);
            }
            eprintln!("note: worker timed out on case {} of `{test}` ({which})", out.len());
            out.push(Outcome::TimedOut);
            continue;
        }
        let expect_first = out.len();
        for (k, (idx, o)) in recs.into_iter().enumerate() {
            assert_eq!(idx, expect_first + k, "worker wrote out-of-order records");
            out.push(o);
        }
        if out.len() >= ncases {
            break;
        }
        // the worker stopped before finishing: the next case killed it
        let stderr = fs::read(&errpath).unwrap_or_default();
        use std::os::unix::process::ExitStatusExt;
        if let Some(sig) = status.signal() {
            out.push(Outcome::Died { signal: sig, stderr });
        } else {
            out.push(Outcome::Exited { code: status.code().unwrap_or(-1), stderr });
        }
    }
    out.truncate(ncases);
    out
}

// ---------------------------------------------------------------------------
// driver
// ---------------------------------------------------------------------------

pub struct Harness {
    pub failures: Vec<String>,
    pub run: usize,
    pub cases: usize,
    filter: Option<String>,
}

impl Harness {
    pub fn new() -> Harness {
        let filter = std::env::args().skip(1).find(|a| !a.starts_with('-'));
        Harness { failures: Vec::new(), run: 0, cases: 0, filter }
    }

    pub fn wanted(&self, id: &str) -> bool {
        match &self.filter {
            None => true,
            Some(f) => id.contains(f.as_str()),
        }
    }

    /// Runs one registered test id: builds its cases, runs them against both
    /// libraries in worker processes, and compares.
    pub fn run_test(&mut self, id: &str) {
        if !self.wanted(id) {
            return;
        }
        self.run += 1;
        let cases = cases::build(id);
        let n = cases.len();
        assert!(n > 0, "test `{id}` produced no cases");
        self.cases += n;
        let c_out = run_pass("c", id, n);
        let r_out = run_pass("rust", id, n);

        let mut local: Vec<String> = Vec::new();
        for i in 0..n {
            let case = &cases[i];
            let co = &c_out[i];
            let ro = &r_out[i];
            if let Err(e) = compare(co, ro) {
                local.push(format!("  [{i}] {}: C vs Rust: {e}", case.label));
                if local.len() > 6 {
                    local.push(format!("  … {} more cases not shown", n - i - 1));
                    break;
                }
                continue;
            }
            if let Err(e) = check_expect(&case.expect, co) {
                local.push(format!("  [{i}] {}: C did not match Expect: {e}", case.label));
                if local.len() > 6 {
                    break;
                }
            }
        }
        let n_timeout = c_out.iter().filter(|o| matches!(o, Outcome::TimedOut)).count();
        let n_died = c_out.iter().filter(|o| matches!(o, Outcome::Died { .. })).count();
        if local.is_empty() {
            let extra = if n_timeout + n_died > 0 {
                let idxs: Vec<usize> = (0..n)
                    .filter(|&i| matches!(c_out[i], Outcome::TimedOut))
                    .collect();
                if n_timeout > 0 {
                    format!(", {n_died} aborted identically, {n_timeout} hung identically {idxs:?}")
                } else {
                    format!(", {n_died} aborted identically")
                }
            } else {
                String::new()
            };
            println!("ok   {id}  ({n} cases{extra})");
        } else {
            println!("FAIL {id}  ({n} cases)");
            for l in &local {
                println!("{l}");
            }
            self.failures.push(format!("{id}:\n{}", local.join("\n")));
        }
    }

    pub fn check(&mut self, id: &str, res: Result<(), String>) {
        if !self.wanted(id) {
            return;
        }
        self.run += 1;
        match res {
            Ok(()) => println!("ok   {id}"),
            Err(e) => {
                println!("FAIL {id}\n  {e}");
                self.failures.push(format!("{id}: {e}"));
            }
        }
    }

    pub fn finish(self) -> ! {
        println!("\n{} tests, {} cases, {} failures", self.run, self.cases, self.failures.len());
        if self.failures.is_empty() {
            println!("ALL DIFFERENTIAL TESTS PASSED");
            std::process::exit(0);
        } else {
            println!("\n=== FAILURES ===");
            for f in &self.failures {
                println!("{f}");
            }
            std::process::exit(1);
        }
    }
}

fn show(b: &[u8]) -> String {
    String::from_utf8_lossy(b).replace('\n', "\\n")
}

pub fn compare(c: &Outcome, r: &Outcome) -> Result<(), String> {
    match (c, r) {
        (
            Outcome::Ok { ret: r1, reason: s1, out_hash: h1, out: o1, tables_hash: t1 },
            Outcome::Ok { ret: r2, reason: s2, out_hash: h2, out: o2, tables_hash: t2 },
        ) => {
            if r1 != r2 {
                return Err(format!("return value {r1} != {r2}"));
            }
            if s1 != s2 {
                return Err(format!(
                    "cp_error_reason {:?} != {:?}",
                    s1.as_deref().map(show),
                    s2.as_deref().map(show)
                ));
            }
            if t1 != t2 {
                return Err("exported tables differ after the call".to_string());
            }
            if h1 != h2 {
                let detail = match (o1, o2) {
                    (Some(a), Some(b)) => {
                        let k = a.iter().zip(b.iter()).position(|(x, y)| x != y);
                        match k {
                            Some(k) => format!(
                                " first difference at out[{k}]: C=0x{:02x} Rust=0x{:02x}",
                                a[k], b[k]
                            ),
                            None => " (declared regions equal; difference is outside out_bytes \
                                     — one library wrote out of range)"
                                .to_string(),
                        }
                    }
                    _ => String::new(),
                };
                return Err(format!("output buffers differ.{detail}"));
            }
            Ok(())
        }
        (Outcome::Died { signal: s1, stderr: e1 }, Outcome::Died { signal: s2, stderr: e2 }) => {
            if s1 != s2 {
                return Err(format!("died from different signals: C={s1} Rust={s2}"));
            }
            if e1 != e2 {
                return Err(format!(
                    "same signal {s1} but different stderr:\n    C   : {}\n    Rust: {}",
                    show(e1),
                    show(e2)
                ));
            }
            Ok(())
        }
        (Outcome::TimedOut, Outcome::TimedOut) => Ok(()),
        (Outcome::Exited { code: c1, stderr: e1 }, Outcome::Exited { code: c2, stderr: e2 }) => {
            if c1 != c2 {
                Err(format!("different exit codes: C={c1} Rust={c2}"))
            } else if e1 != e2 {
                Err(format!(
                    "same exit code {c1} but different stderr:\n    C   : {}\n    Rust: {}",
                    show(e1),
                    show(e2)
                ))
            } else {
                Ok(())
            }
        }
        (a, b) => Err(format!("different outcome kinds:\n    C   : {a:?}\n    Rust: {b:?}")),
    }
}

fn check_expect(e: &Expect, c: &Outcome) -> Result<(), String> {
    match (e, c) {
        (Expect::Any, _) => Ok(()),
        (Expect::Ret { ret, reason }, Outcome::Ok { ret: r, reason: s, .. }) => {
            if ret != r {
                return Err(format!("expected ret {ret}, C returned {r}"));
            }
            let want = reason.map(|x| x.as_bytes().to_vec());
            if want != *s {
                return Err(format!(
                    "expected cp_error_reason {:?}, got {:?}",
                    reason,
                    s.as_deref().map(show)
                ));
            }
            Ok(())
        }
        (Expect::Out { ret, out }, Outcome::Ok { ret: r, out: Some(o), .. }) => {
            if ret != r {
                return Err(format!("expected ret {ret}, C returned {r}"));
            }
            if out != o {
                let k = out.iter().zip(o.iter()).position(|(x, y)| x != y);
                return Err(format!(
                    "output != expected (len {} vs {}), first diff at {:?}",
                    out.len(),
                    o.len(),
                    k
                ));
            }
            Ok(())
        }
        (Expect::Out { .. }, Outcome::Ok { out: None, .. }) => {
            Err("output not captured (too large)".to_string())
        }
        (Expect::Assert { line, func, expr }, Outcome::Died { signal, stderr }) => {
            if *signal != 6 {
                return Err(format!("expected SIGABRT(6), got signal {signal}"));
            }
            let want = format!(":{line}: {func}: Assertion `{expr}' failed.");
            if !String::from_utf8_lossy(stderr).contains(&want) {
                return Err(format!("stderr {:?} does not contain {:?}", show(stderr), want));
            }
            Ok(())
        }
        (Expect::Signal(s), Outcome::Died { signal, .. }) => {
            if s != signal {
                Err(format!("expected signal {s}, got {signal}"))
            } else {
                Ok(())
            }
        }
        (e, o) => Err(format!("expected {e:?}, got {o:?}")),
    }
}
