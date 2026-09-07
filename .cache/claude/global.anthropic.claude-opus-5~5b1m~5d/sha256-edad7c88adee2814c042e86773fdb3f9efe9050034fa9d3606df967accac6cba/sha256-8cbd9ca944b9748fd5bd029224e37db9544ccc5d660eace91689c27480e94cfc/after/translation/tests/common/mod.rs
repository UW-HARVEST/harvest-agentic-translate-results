//! Shared differential-test harness.
//!
//! Loads BOTH the C `.so` and the Rust `.so` through `libloading` and exposes
//! their exported symbols as raw `extern "C"` function pointers, so every call
//! crosses the real FFI boundary (exercising the `#[no_mangle]` wrappers).

#![allow(dead_code)]
#![allow(non_snake_case)]

use libloading::{Library, Symbol};
use std::ffi::{c_char, c_int, c_void};
use std::sync::{Mutex, MutexGuard, OnceLock};

// ---------------------------------------------------------------------------
// Mirrored C layouts (used only to *inspect* state; never to call Rust directly)
// ---------------------------------------------------------------------------

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct Header {
    pub length: usize,
    pub capacity: usize,
    pub hash_table: *mut c_void,
    pub temp: isize,
}

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct Bucket {
    pub hash: [usize; 8],
    pub index: [isize; 8],
}

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct Arena {
    pub storage: *mut c_void,
    pub remaining: usize,
    pub block: u8,
    pub mode: u8,
}

impl Arena {
    pub fn new() -> Arena {
        Arena {
            storage: std::ptr::null_mut(),
            remaining: 0,
            block: 0,
            mode: 0,
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct HashIndex {
    pub temp_key: *mut c_char,
    pub slot_count: usize,
    pub used_count: usize,
    pub used_count_threshold: usize,
    pub used_count_shrink_threshold: usize,
    pub tombstone_count: usize,
    pub tombstone_count_threshold: usize,
    pub seed: usize,
    pub slot_count_log2: usize,
    pub string: Arena,
    pub storage: *mut Bucket,
}

pub const HDR: usize = std::mem::size_of::<Header>(); // 32

pub const HM_BINARY: c_int = 0;
pub const HM_STRING: c_int = 1;
pub const SH_NONE: c_int = 0;
pub const SH_DEFAULT: c_int = 1;
pub const SH_STRDUP: c_int = 2;
pub const SH_ARENA: c_int = 3;

// ---------------------------------------------------------------------------
// Symbol table
// ---------------------------------------------------------------------------

pub struct Lib {
    pub name: &'static str,
    pub arrgrowf: unsafe extern "C" fn(*mut c_void, usize, usize, usize) -> *mut c_void,
    pub arrfreef: unsafe extern "C" fn(*mut c_void),
    pub rand_seed: unsafe extern "C" fn(usize),
    pub hash_string: unsafe extern "C" fn(*mut c_char, usize) -> usize,
    pub hash_bytes: unsafe extern "C" fn(*mut c_void, usize, usize) -> usize,
    pub hmfree_func: unsafe extern "C" fn(*mut c_void, usize),
    pub hmget_key_ts:
        unsafe extern "C" fn(*mut c_void, usize, *mut c_void, usize, *mut isize, c_int) -> *mut c_void,
    pub hmget_key: unsafe extern "C" fn(*mut c_void, usize, *mut c_void, usize, c_int) -> *mut c_void,
    pub hmput_default: unsafe extern "C" fn(*mut c_void, usize) -> *mut c_void,
    pub hmput_key: unsafe extern "C" fn(*mut c_void, usize, *mut c_void, usize, c_int) -> *mut c_void,
    pub shmode_func: unsafe extern "C" fn(usize, c_int) -> *mut c_void,
    pub hmdel_key:
        unsafe extern "C" fn(*mut c_void, usize, *mut c_void, usize, usize, c_int) -> *mut c_void,
    pub stralloc: unsafe extern "C" fn(*mut Arena, *mut c_char) -> *mut c_char,
    pub strreset: unsafe extern "C" fn(*mut Arena),
    pub strkey: unsafe extern "C" fn(c_int) -> *mut c_char,
    pub str_dups: unsafe extern "C" fn(c_int),
    _lib: &'static Library,
}

unsafe fn sym<T: Copy>(lib: &Library, name: &[u8]) -> T {
    let s: Symbol<T> = lib
        .get(name)
        .unwrap_or_else(|e| panic!("missing symbol {}: {e}", String::from_utf8_lossy(name)));
    *s
}

impl Lib {
    unsafe fn load(name: &'static str, path: &str) -> Lib {
        let lib: &'static Library = Box::leak(Box::new(
            Library::new(path).unwrap_or_else(|e| panic!("dlopen {path}: {e}")),
        ));
        Lib {
            name,
            arrgrowf: sym(lib, b"stbds_arrgrowf\0"),
            arrfreef: sym(lib, b"stbds_arrfreef\0"),
            rand_seed: sym(lib, b"stbds_rand_seed\0"),
            hash_string: sym(lib, b"stbds_hash_string\0"),
            hash_bytes: sym(lib, b"stbds_hash_bytes\0"),
            hmfree_func: sym(lib, b"stbds_hmfree_func\0"),
            hmget_key_ts: sym(lib, b"stbds_hmget_key_ts\0"),
            hmget_key: sym(lib, b"stbds_hmget_key\0"),
            hmput_default: sym(lib, b"stbds_hmput_default\0"),
            hmput_key: sym(lib, b"stbds_hmput_key\0"),
            shmode_func: sym(lib, b"stbds_shmode_func\0"),
            hmdel_key: sym(lib, b"stbds_hmdel_key\0"),
            stralloc: sym(lib, b"stbds_stralloc\0"),
            strreset: sym(lib, b"stbds_strreset\0"),
            strkey: sym(lib, b"strkey\0"),
            str_dups: sym(lib, b"str_dups\0"),
            _lib: lib,
        }
    }
}

fn manifest() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_so_path() -> String {
    let dir = manifest().parent().unwrap().join("c_src").join("build");
    let mut found = None;
    for e in std::fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("c_src/build not built ({}): {e}", dir.display()))
    {
        let p = e.unwrap().path();
        let n = p.file_name().unwrap().to_string_lossy().to_string();
        if n.starts_with("lib") && n.ends_with(".so") {
            found = Some(p.to_string_lossy().to_string());
        }
    }
    found.unwrap_or_else(|| panic!("no lib*.so in {}", dir.display()))
}

fn rust_so_path() -> String {
    let p = manifest()
        .join("target")
        .join("release")
        .join("libstr_dups_lib.so");
    assert!(
        p.exists(),
        "run `cargo build --release` first: {} missing",
        p.display()
    );
    p.to_string_lossy().to_string()
}

pub struct Libs {
    pub c: Lib,
    pub rust: Lib,
}

static LIBS: OnceLock<Mutex<Libs>> = OnceLock::new();

/// Serialised access to both libraries. Serialisation matters because the
/// libraries carry *global* mutable state (`stbds_hash_seed`), so concurrent
/// tests would interleave the seed LCG.
pub fn libs() -> MutexGuard<'static, Libs> {
    let m = LIBS.get_or_init(|| unsafe {
        Mutex::new(Libs {
            c: Lib::load("C", &c_so_path()),
            rust: Lib::load("Rust", &rust_so_path()),
        })
    });
    match m.lock() {
        Ok(g) => g,
        Err(p) => p.into_inner(),
    }
}

// ---------------------------------------------------------------------------
// Trace
// ---------------------------------------------------------------------------

#[derive(Default)]
pub struct Trace {
    pub lines: Vec<String>,
}

impl Trace {
    pub fn rec(&mut self, tag: &str, v: impl std::fmt::Debug) {
        self.lines.push(format!("{tag}={v:?}"));
    }
    pub fn rec_str(&mut self, tag: &str, v: String) {
        self.lines.push(format!("{tag}={v}"));
    }
}

/// Run the same closure against the C lib and the Rust lib and require the
/// resulting traces to be byte-identical.
pub fn differential(label: &str, f: impl Fn(&Lib, &mut Trace)) {
    let g = libs();
    let mut tc = Trace::default();
    let mut tr = Trace::default();
    f(&g.c, &mut tc);
    f(&g.rust, &mut tr);
    compare(label, &tc, &tr);
}

pub fn compare(label: &str, tc: &Trace, tr: &Trace) {
    if tc.lines == tr.lines {
        return;
    }
    let n = tc.lines.len().max(tr.lines.len());
    let mut msg = format!("DIVERGENCE in `{label}`:\n");
    let mut shown = 0;
    for i in 0..n {
        let a = tc.lines.get(i).map(|s| s.as_str()).unwrap_or("<missing>");
        let b = tr.lines.get(i).map(|s| s.as_str()).unwrap_or("<missing>");
        if a != b {
            msg.push_str(&format!("  [{i}] C   : {a}\n  [{i}] RUST: {b}\n"));
            shown += 1;
            if shown >= 12 {
                msg.push_str("  ... (truncated)\n");
                break;
            }
        }
    }
    panic!("{msg}");
}

// ---------------------------------------------------------------------------
// State inspection
// ---------------------------------------------------------------------------

#[derive(Copy, Clone, PartialEq, Eq)]
pub enum KeyKind {
    /// element payload is raw bytes
    Raw,
    /// element payload starts with a `char *` that must be compared by content
    StrPtr,
    /// like `StrPtr`, but `temp_key` is not dereferenced (it may legitimately
    /// dangle after an `SH_STRDUP` delete, in which case the freed heap bytes
    /// are allocator-internal and not part of the observable contract)
    StrPtrNoTk,
}

pub unsafe fn hex(p: *const u8, n: usize) -> String {
    let mut s = String::with_capacity(n * 2);
    for i in 0..n {
        s.push_str(&format!("{:02x}", *p.add(i)));
    }
    s
}

pub unsafe fn cstr(p: *const c_char) -> String {
    if p.is_null() {
        return "<null>".to_string();
    }
    let mut v = Vec::new();
    let mut i = 0isize;
    loop {
        let b = *(p.offset(i) as *const u8);
        if b == 0 {
            break;
        }
        v.push(b);
        i += 1;
        if i > 1 << 20 {
            break;
        }
    }
    format!("\"{}\"", String::from_utf8_lossy(&v))
}

/// Snapshot everything observable about a stb_ds hash map given its "hash
/// pointer" (`t`, i.e. raw array base + elemsize).
pub unsafe fn snap_map(t: &mut Trace, tag: &str, map: *mut c_void, elemsize: usize, kk: KeyKind) {
    if map.is_null() {
        t.rec_str(tag, "NULL".to_string());
        return;
    }
    let raw = (map as *mut u8).sub(elemsize) as *mut c_void;
    let h = (raw as *mut u8).sub(HDR) as *const Header;
    t.rec(&format!("{tag}.length"), (*h).length);
    t.rec(&format!("{tag}.capacity"), (*h).capacity);
    t.rec(&format!("{tag}.temp"), (*h).temp);
    let ht = (*h).hash_table as *const HashIndex;
    if ht.is_null() {
        t.rec_str(&format!("{tag}.table"), "NULL".to_string());
    } else {
        let ti = &*ht;
        t.rec(&format!("{tag}.slot_count"), ti.slot_count);
        t.rec(&format!("{tag}.used_count"), ti.used_count);
        t.rec(&format!("{tag}.uc_thresh"), ti.used_count_threshold);
        t.rec(&format!("{tag}.uc_shrink"), ti.used_count_shrink_threshold);
        t.rec(&format!("{tag}.tomb"), ti.tombstone_count);
        t.rec(&format!("{tag}.tomb_thresh"), ti.tombstone_count_threshold);
        t.rec(&format!("{tag}.seed"), ti.seed);
        t.rec(&format!("{tag}.log2"), ti.slot_count_log2);
        t.rec(&format!("{tag}.str.remaining"), ti.string.remaining);
        t.rec(&format!("{tag}.str.block"), ti.string.block);
        t.rec(&format!("{tag}.str.mode"), ti.string.mode);
        // `temp_key` is *never* initialised by stbds_make_hash_index (and is
        // dropped on every table grow/shrink/rebuild), so on a
        // binary-mode table (and on a string table on which nothing has been
        // inserted yet) it holds indeterminate malloc bytes.  Only compare it
        // where the C code has definitely written it.
        if kk == KeyKind::StrPtr {
            t.rec_str(&format!("{tag}.temp_key"), cstr(ti.temp_key));
        }
        let nb = ti.slot_count >> 3;
        for i in 0..nb {
            let b = &*ti.storage.add(i);
            t.rec_str(
                &format!("{tag}.bucket[{i}]"),
                format!("{:?} {:?}", b.hash, b.index),
            );
        }
    }
    // element payloads, raw indices 0..length
    for i in 0..(*h).length {
        let e = (raw as *mut u8).add(elemsize * i);
        match kk {
            KeyKind::Raw => t.rec_str(&format!("{tag}.elem[{i}]"), hex(e, elemsize)),
            KeyKind::StrPtr | KeyKind::StrPtrNoTk => {
                let kp = *(e as *const *const c_char);
                let rest = if elemsize > 8 {
                    hex(e.add(8), elemsize - 8)
                } else {
                    String::new()
                };
                t.rec_str(
                    &format!("{tag}.elem[{i}]"),
                    format!("{} {}", cstr(kp), rest),
                );
            }
        }
    }
}

pub unsafe fn snap_arr(t: &mut Trace, tag: &str, a: *mut c_void, elemsize: usize, dump: bool) {
    if a.is_null() {
        t.rec_str(tag, "NULL".to_string());
        return;
    }
    let h = (a as *mut u8).sub(HDR) as *const Header;
    t.rec(&format!("{tag}.length"), (*h).length);
    t.rec(&format!("{tag}.capacity"), (*h).capacity);
    t.rec(&format!("{tag}.temp"), (*h).temp);
    t.rec(&format!("{tag}.has_table"), !(*h).hash_table.is_null());
    if dump {
        for i in 0..(*h).length {
            t.rec_str(
                &format!("{tag}.elem[{i}]"),
                hex((a as *mut u8).add(elemsize * i), elemsize),
            );
        }
    }
}

pub unsafe fn snap_arena(t: &mut Trace, tag: &str, a: &Arena) {
    t.rec(&format!("{tag}.remaining"), a.remaining);
    t.rec(&format!("{tag}.block"), a.block);
    t.rec(&format!("{tag}.mode"), a.mode);
    t.rec(&format!("{tag}.storage_null"), a.storage.is_null());
    // walk the block chain length (pointers differ, count does not)
    let mut n = 0usize;
    let mut x = a.storage as *const *const c_void;
    while !x.is_null() && n < 100000 {
        n += 1;
        x = *x as *const *const c_void;
    }
    t.rec(&format!("{tag}.blocks"), n);
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (xorshift64*)
// ---------------------------------------------------------------------------

pub struct Rng(pub u64);

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
    pub fn below(&mut self, n: usize) -> usize {
        if n == 0 {
            0
        } else {
            (self.next_u64() % (n as u64)) as usize
        }
    }
    pub fn bytes(&mut self, n: usize) -> Vec<u8> {
        (0..n).map(|_| (self.next_u64() >> 24) as u8).collect()
    }
    /// printable ASCII, never contains NUL
    pub fn ascii(&mut self, n: usize) -> Vec<u8> {
        (0..n)
            .map(|_| 0x21u8 + (self.next_u64() % 94) as u8)
            .collect()
    }
    /// arbitrary non-NUL bytes (includes >= 0x80)
    pub fn nonzero(&mut self, n: usize) -> Vec<u8> {
        (0..n)
            .map(|_| {
                let b = (self.next_u64() >> 32) as u8;
                if b == 0 {
                    1
                } else {
                    b
                }
            })
            .collect()
    }
}

pub fn cstring(bytes: &[u8]) -> Vec<u8> {
    let mut v = bytes.to_vec();
    v.push(0);
    v
}

// ---------------------------------------------------------------------------
// stdout capture (printf writes to fd 1 of the process, bypassing Rust)
// ---------------------------------------------------------------------------

extern "C" {
    fn dup(fd: c_int) -> c_int;
    fn dup2(old: c_int, new: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn fflush(f: *mut c_void) -> c_int;
    fn open(path: *const c_char, flags: c_int, ...) -> c_int;
    fn lseek(fd: c_int, off: i64, whence: c_int) -> i64;
    fn read(fd: c_int, buf: *mut c_void, n: usize) -> isize;
}

const O_RDWR: c_int = 2;
const O_CREAT: c_int = 64;
const O_TRUNC: c_int = 512;

static CAP_N: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

/// Redirect fd 1 to a scratch file, run `f`, and return everything written.
pub fn capture_stdout(f: impl FnOnce()) -> Vec<u8> {
    let n = CAP_N.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    let dir = std::env::var("TMPDIR").unwrap_or_else(|_| "/tmp".to_string());
    let path = format!("{dir}/difftest_cap_{}_{n}.txt", std::process::id());
    let cpath = cstring(path.as_bytes());
    unsafe {
        let fd = open(cpath.as_ptr() as *const c_char, O_RDWR | O_CREAT | O_TRUNC, 0o600i32);
        assert!(fd >= 0, "open {path} failed");
        fflush(std::ptr::null_mut());
        let saved = dup(1);
        assert!(saved >= 0);
        dup2(fd, 1);
        f();
        fflush(std::ptr::null_mut());
        dup2(saved, 1);
        close(saved);
        lseek(fd, 0, 0);
        let mut out = Vec::new();
        let mut buf = [0u8; 65536];
        loop {
            let r = read(fd, buf.as_mut_ptr() as *mut c_void, buf.len());
            if r <= 0 {
                break;
            }
            out.extend_from_slice(&buf[..r as usize]);
        }
        close(fd);
        let _ = std::fs::remove_file(&path);
        out
    }
}
