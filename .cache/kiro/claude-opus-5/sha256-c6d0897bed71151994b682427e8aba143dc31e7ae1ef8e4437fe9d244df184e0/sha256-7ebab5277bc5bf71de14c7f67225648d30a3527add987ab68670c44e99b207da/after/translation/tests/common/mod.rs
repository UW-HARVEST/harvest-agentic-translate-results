//! Shared harness: loads BOTH the C `.so` and the Rust `.so` through
//! `libloading` and exposes thin wrappers around the exported symbols.
//!
//! Nothing in here calls the Rust crate directly - every call goes through the
//! dynamic-library boundary so that the `#[no_mangle]` wrappers are exercised
//! exactly the way an external C caller would exercise them.

#![allow(dead_code)]

use std::ffi::{c_char, c_int, c_void};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use libloading::{Library, Symbol};

// ---------------------------------------------------------------------------
// Mirrors of the C structures (needed to inspect the produced state)
// ---------------------------------------------------------------------------

pub const BUCKET_LENGTH: usize = 8;

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct ArrHeader {
    pub length: usize,
    pub capacity: usize,
    pub hash_table: *mut c_void,
    pub temp: isize,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct StringArena {
    pub storage: *mut c_void,
    pub remaining: usize,
    pub block: u8,
    pub mode: u8,
}

impl StringArena {
    pub fn zeroed() -> Self {
        StringArena {
            storage: std::ptr::null_mut(),
            remaining: 0,
            block: 0,
            mode: 0,
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct HashBucket {
    pub hash: [usize; BUCKET_LENGTH],
    pub index: [isize; BUCKET_LENGTH],
}

#[repr(C)]
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
    pub string: StringArena,
    pub storage: *mut HashBucket,
}

pub const HM_BINARY: c_int = 0;
pub const HM_STRING: c_int = 1;

pub const SH_NONE: c_int = 0;
pub const SH_DEFAULT: c_int = 1;
pub const SH_STRDUP: c_int = 2;
pub const SH_ARENA: c_int = 3;

pub const HEADER_SIZE: usize = std::mem::size_of::<ArrHeader>();

// ---------------------------------------------------------------------------
// Locating / loading the two shared objects
// ---------------------------------------------------------------------------

fn workspace_root() -> PathBuf {
    // CARGO_MANIFEST_DIR == <root>/translation
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation/ has a parent")
        .to_path_buf()
}

fn c_so_path() -> PathBuf {
    // Allows pointing the comparison tests at an instrumented build of the same
    // C source (e.g. a gcov build) without touching c_src/.
    if let Ok(p) = std::env::var("C_SO_PATH") {
        return PathBuf::from(p);
    }
    c_so_path_canonical()
}

/// The .so produced by `c_src/CMakeLists.txt`, ignoring any `C_SO_PATH`
/// override - symbol parity is always judged against the canonical build.
fn c_so_path_canonical() -> PathBuf {
    let build = workspace_root().join("c_src/build");
    let mut found = None;
    if let Ok(rd) = std::fs::read_dir(&build) {
        for e in rd.flatten() {
            let p = e.path();
            if p.extension().and_then(|s| s.to_str()) == Some("so") {
                found = Some(p);
                break;
            }
        }
    }
    found.unwrap_or_else(|| {
        panic!(
            "no .so found in {}: build the C library first (cmake .. && cmake --build .)",
            build.display()
        )
    })
}

/// The Rust cdylib lives next to the integration-test executable's parent
/// directory (`target/<profile>/libarr_push_lib.so`). `cargo test` does not
/// build the cdylib on its own, so build it here if it is missing/stale.
fn rust_so_path() -> PathBuf {
    let exe = std::env::current_exe().expect("current_exe");
    // .../target/<profile>/deps/<test>-<hash>
    let profile_dir = exe
        .parent()
        .and_then(|p| p.parent())
        .expect("target/<profile>")
        .to_path_buf();
    let so = profile_dir.join("libarr_push_lib.so");

    let profile = profile_dir
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("debug")
        .to_string();

    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/lib.rs");
    let needs_build = match (std::fs::metadata(&so), std::fs::metadata(&src)) {
        (Ok(a), Ok(b)) => match (a.modified(), b.modified()) {
            (Ok(am), Ok(bm)) => am < bm,
            _ => false,
        },
        _ => true,
    };

    if needs_build {
        let mut cmd = std::process::Command::new(std::env::var("CARGO").unwrap_or("cargo".into()));
        cmd.current_dir(env!("CARGO_MANIFEST_DIR"));
        cmd.arg("build").arg("--lib");
        if profile == "release" {
            cmd.arg("--release");
        }
        // Propagate the feature selection this test binary was compiled with.
        for extra in FEATURE_ARGS {
            cmd.arg(extra);
        }
        cmd.env_remove("RUSTFLAGS");
        let status = cmd.status().expect("spawn cargo build --lib");
        assert!(status.success(), "cargo build --lib failed");
    }

    assert!(
        so.exists(),
        "rust cdylib not found at {} - run `cargo build` first",
        so.display()
    );
    so
}

/// The crate currently declares no `[features]`, so there is exactly one
/// build configuration; keep the hook so extra flags can be threaded through.
const FEATURE_ARGS: &[&str] = &[];

/// Public accessors so the symbol-parity test can `nm` the same two files
/// that the comparison tests load.
pub fn c_so_path_pub() -> PathBuf {
    c_so_path_canonical()
}

pub fn rust_so_path_pub() -> PathBuf {
    rust_so_path()
}

pub struct Lib {
    lib: Library,
    pub name: &'static str,
}

impl Lib {
    fn sym<T>(&self, name: &[u8]) -> Symbol<'_, T> {
        unsafe {
            self.lib
                .get(name)
                .unwrap_or_else(|e| panic!("{}: missing symbol {:?}: {e}", self.name, name))
        }
    }

    // ---- stbds_arrgrowf / stbds_arrfreef ----------------------------------
    pub unsafe fn arrgrowf(
        &self,
        a: *mut c_void,
        elemsize: usize,
        addlen: usize,
        min_cap: usize,
    ) -> *mut u8 {
        let f: Symbol<
            unsafe extern "C" fn(*mut c_void, usize, usize, usize) -> *mut c_void,
        > = self.sym(b"stbds_arrgrowf\0");
        f(a, elemsize, addlen, min_cap) as *mut u8
    }

    pub unsafe fn arrfreef(&self, a: *mut c_void) {
        let f: Symbol<unsafe extern "C" fn(*mut c_void)> = self.sym(b"stbds_arrfreef\0");
        f(a)
    }

    // ---- seeding / hashing -------------------------------------------------
    pub unsafe fn rand_seed(&self, seed: usize) {
        let f: Symbol<unsafe extern "C" fn(usize)> = self.sym(b"stbds_rand_seed\0");
        f(seed)
    }

    pub unsafe fn hash_string(&self, s: *mut c_char, seed: usize) -> usize {
        let f: Symbol<unsafe extern "C" fn(*mut c_char, usize) -> usize> =
            self.sym(b"stbds_hash_string\0");
        f(s, seed)
    }

    pub unsafe fn hash_bytes(&self, p: *mut c_void, len: usize, seed: usize) -> usize {
        let f: Symbol<unsafe extern "C" fn(*mut c_void, usize, usize) -> usize> =
            self.sym(b"stbds_hash_bytes\0");
        f(p, len, seed)
    }

    // ---- hash map ---------------------------------------------------------
    pub unsafe fn hmfree_func(&self, p: *mut c_void, elemsize: usize) {
        let f: Symbol<unsafe extern "C" fn(*mut c_void, usize)> = self.sym(b"stbds_hmfree_func\0");
        f(p, elemsize)
    }

    pub unsafe fn hmget_key(
        &self,
        a: *mut c_void,
        elemsize: usize,
        key: *mut c_void,
        keysize: usize,
        mode: c_int,
    ) -> *mut u8 {
        let f: Symbol<
            unsafe extern "C" fn(*mut c_void, usize, *mut c_void, usize, c_int) -> *mut c_void,
        > = self.sym(b"stbds_hmget_key\0");
        f(a, elemsize, key, keysize, mode) as *mut u8
    }

    pub unsafe fn hmget_key_ts(
        &self,
        a: *mut c_void,
        elemsize: usize,
        key: *mut c_void,
        keysize: usize,
        temp: *mut isize,
        mode: c_int,
    ) -> *mut u8 {
        let f: Symbol<
            unsafe extern "C" fn(
                *mut c_void,
                usize,
                *mut c_void,
                usize,
                *mut isize,
                c_int,
            ) -> *mut c_void,
        > = self.sym(b"stbds_hmget_key_ts\0");
        f(a, elemsize, key, keysize, temp, mode) as *mut u8
    }

    pub unsafe fn hmput_default(&self, a: *mut c_void, elemsize: usize) -> *mut u8 {
        let f: Symbol<unsafe extern "C" fn(*mut c_void, usize) -> *mut c_void> =
            self.sym(b"stbds_hmput_default\0");
        f(a, elemsize) as *mut u8
    }

    pub unsafe fn hmput_key(
        &self,
        a: *mut c_void,
        elemsize: usize,
        key: *mut c_void,
        keysize: usize,
        mode: c_int,
    ) -> *mut u8 {
        let f: Symbol<
            unsafe extern "C" fn(*mut c_void, usize, *mut c_void, usize, c_int) -> *mut c_void,
        > = self.sym(b"stbds_hmput_key\0");
        f(a, elemsize, key, keysize, mode) as *mut u8
    }

    pub unsafe fn hmdel_key(
        &self,
        a: *mut c_void,
        elemsize: usize,
        key: *mut c_void,
        keysize: usize,
        keyoffset: usize,
        mode: c_int,
    ) -> *mut u8 {
        let f: Symbol<
            unsafe extern "C" fn(
                *mut c_void,
                usize,
                *mut c_void,
                usize,
                usize,
                c_int,
            ) -> *mut c_void,
        > = self.sym(b"stbds_hmdel_key\0");
        f(a, elemsize, key, keysize, keyoffset, mode) as *mut u8
    }

    pub unsafe fn shmode_func(&self, elemsize: usize, mode: c_int) -> *mut u8 {
        let f: Symbol<unsafe extern "C" fn(usize, c_int) -> *mut c_void> =
            self.sym(b"stbds_shmode_func\0");
        f(elemsize, mode) as *mut u8
    }

    // ---- string arena ----------------------------------------------------
    pub unsafe fn stralloc(&self, a: *mut StringArena, s: *mut c_char) -> *mut c_char {
        let f: Symbol<unsafe extern "C" fn(*mut StringArena, *mut c_char) -> *mut c_char> =
            self.sym(b"stbds_stralloc\0");
        f(a, s)
    }

    pub unsafe fn strreset(&self, a: *mut StringArena) {
        let f: Symbol<unsafe extern "C" fn(*mut StringArena)> = self.sym(b"stbds_strreset\0");
        f(a)
    }

    // ---- driver helpers --------------------------------------------------
    pub unsafe fn strkey(&self, n: c_int) -> *mut c_char {
        let f: Symbol<unsafe extern "C" fn(c_int) -> *mut c_char> = self.sym(b"strkey\0");
        f(n)
    }

    pub unsafe fn arr_push(&self, n: c_int) {
        let f: Symbol<unsafe extern "C" fn(c_int)> = self.sym(b"arr_push\0");
        f(n)
    }
}

pub struct Pair {
    pub c: Lib,
    pub rs: Lib,
}

static PAIR: OnceLock<Pair> = OnceLock::new();

pub fn libs() -> &'static Pair {
    PAIR.get_or_init(|| {
        let c = unsafe { Library::new(c_so_path()) }.expect("load C .so");
        let rs = unsafe { Library::new(rust_so_path()) }.expect("load Rust .so");
        Pair {
            c: Lib { lib: c, name: "C" },
            rs: Lib {
                lib: rs,
                name: "Rust",
            },
        }
    })
}

// ---------------------------------------------------------------------------
// Snapshotting: turn library-produced state into a comparable value
// ---------------------------------------------------------------------------

/// How to decode a hash-map element for comparison. Raw pointer values and
/// malloc addresses necessarily differ between the two libraries, so keys that
/// are `char *` are compared by the string they point at.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Fmt {
    /// `struct { int key; int value; }`
    BinaryKV,
    /// `struct { char *key; int value; }`
    StrKV,
    /// `struct { int key[2]; int b, c, d; }`
    Binary2KV,
    /// Raw element bytes.
    Raw,
}

impl Fmt {
    pub fn elemsize(self) -> usize {
        match self {
            Fmt::BinaryKV => 8,
            Fmt::StrKV => 16,
            Fmt::Binary2KV => 20,
            Fmt::Raw => 8,
        }
    }
    pub fn keysize(self) -> usize {
        match self {
            Fmt::BinaryKV => 4,
            Fmt::StrKV => 8,
            Fmt::Binary2KV => 8,
            Fmt::Raw => 8,
        }
    }
}

#[derive(PartialEq, Eq, Debug)]
pub enum Elem {
    BinaryKV { key: i32, value: i32 },
    StrKV { key: Option<Vec<u8>>, value: i32 },
    Binary2KV { key: [i32; 2], b: i32, c: i32, d: i32 },
    Raw(Vec<u8>),
}

#[derive(PartialEq, Eq, Debug)]
pub struct TableSnap {
    pub slot_count: usize,
    pub used_count: usize,
    pub used_count_threshold: usize,
    pub used_count_shrink_threshold: usize,
    pub tombstone_count: usize,
    pub tombstone_count_threshold: usize,
    pub seed: usize,
    pub slot_count_log2: usize,
    pub string_remaining: usize,
    pub string_block: u8,
    pub string_mode: u8,
    pub string_storage_null: bool,
    pub buckets: Vec<(Vec<usize>, Vec<isize>)>,
}

#[derive(PartialEq, Eq, Debug)]
pub struct Snap {
    pub null: bool,
    pub length: usize,
    pub capacity: usize,
    pub temp: isize,
    pub table: Option<TableSnap>,
    pub elems: Vec<Elem>,
}

pub unsafe fn read_cstr(p: *const c_char) -> Option<Vec<u8>> {
    if p.is_null() {
        return None;
    }
    let mut v = Vec::new();
    let mut i = 0isize;
    loop {
        let b = *p.offset(i) as u8;
        if b == 0 {
            break;
        }
        v.push(b);
        i += 1;
        assert!(i < 1 << 20, "unterminated string");
    }
    Some(v)
}

unsafe fn read_elem(p: *const u8, fmt: Fmt) -> Elem {
    match fmt {
        Fmt::BinaryKV => Elem::BinaryKV {
            key: (p as *const i32).read_unaligned(),
            value: (p.add(4) as *const i32).read_unaligned(),
        },
        Fmt::StrKV => {
            let kp = (p as *const *const c_char).read_unaligned();
            Elem::StrKV {
                key: read_cstr(kp),
                value: (p.add(8) as *const i32).read_unaligned(),
            }
        }
        Fmt::Binary2KV => Elem::Binary2KV {
            key: [
                (p as *const i32).read_unaligned(),
                (p.add(4) as *const i32).read_unaligned(),
            ],
            b: (p.add(8) as *const i32).read_unaligned(),
            c: (p.add(12) as *const i32).read_unaligned(),
            d: (p.add(16) as *const i32).read_unaligned(),
        },
        Fmt::Raw => Elem::Raw(std::slice::from_raw_parts(p, fmt.elemsize()).to_vec()),
    }
}

unsafe fn header_of(arr: *mut u8) -> *mut ArrHeader {
    (arr as *mut ArrHeader).offset(-1)
}

unsafe fn snap_table(t: *mut HashIndex) -> Option<TableSnap> {
    if t.is_null() {
        return None;
    }
    let nbuckets = (*t).slot_count >> 3;
    let mut buckets = Vec::with_capacity(nbuckets);
    for i in 0..nbuckets {
        let b = (*t).storage.add(i);
        buckets.push(((*b).hash.to_vec(), (*b).index.to_vec()));
    }
    Some(TableSnap {
        slot_count: (*t).slot_count,
        used_count: (*t).used_count,
        used_count_threshold: (*t).used_count_threshold,
        used_count_shrink_threshold: (*t).used_count_shrink_threshold,
        tombstone_count: (*t).tombstone_count,
        tombstone_count_threshold: (*t).tombstone_count_threshold,
        seed: (*t).seed,
        slot_count_log2: (*t).slot_count_log2,
        string_remaining: (*t).string.remaining,
        string_block: (*t).string.block,
        string_mode: (*t).string.mode,
        string_storage_null: (*t).string.storage.is_null(),
        buckets,
    })
}

/// Snapshot a *plain array* pointer (as returned by `stbds_arrgrowf`).
pub unsafe fn snap_arr(a: *mut u8, fmt: Fmt, read_elems: usize) -> Snap {
    if a.is_null() {
        return Snap {
            null: true,
            length: 0,
            capacity: 0,
            temp: 0,
            table: None,
            elems: Vec::new(),
        };
    }
    let h = header_of(a);
    let mut elems = Vec::new();
    for i in 0..read_elems {
        elems.push(read_elem(a.add(i * fmt.elemsize()), fmt));
    }
    Snap {
        null: false,
        length: (*h).length,
        capacity: (*h).capacity,
        temp: (*h).temp,
        table: snap_table((*h).hash_table as *mut HashIndex),
        elems,
    }
}

/// Snapshot a *hash map* pointer `t` (one element past the raw array base).
pub unsafe fn snap_hm(t: *mut u8, fmt: Fmt) -> Snap {
    if t.is_null() {
        return Snap {
            null: true,
            length: 0,
            capacity: 0,
            temp: 0,
            table: None,
            elems: Vec::new(),
        };
    }
    let raw = t.sub(fmt.elemsize());
    let h = header_of(raw);
    let len = (*h).length;
    let mut elems = Vec::new();
    for i in 0..len {
        elems.push(read_elem(raw.add(i * fmt.elemsize()), fmt));
    }
    Snap {
        null: false,
        length: len,
        capacity: (*h).capacity,
        temp: (*h).temp,
        table: snap_table((*h).hash_table as *mut HashIndex),
        elems,
    }
}

// ---------------------------------------------------------------------------
// Macro-level helpers (the `hmput` / `hmget` / `hmdel` wrappers from lib.c)
// ---------------------------------------------------------------------------

pub unsafe fn temp_of(t: *mut u8, elemsize: usize) -> isize {
    (*header_of(t.sub(elemsize))).temp
}

/// `stbds_hmput(t, k, v)` for `struct { int key; int value; }`
pub unsafe fn hmput_i32(lib: &Lib, t: *mut u8, k: i32, v: i32) -> *mut u8 {
    let fmt = Fmt::BinaryKV;
    let mut key = k;
    let t = lib.hmput_key(
        t as *mut c_void,
        fmt.elemsize(),
        &mut key as *mut i32 as *mut c_void,
        fmt.keysize(),
        HM_BINARY,
    );
    let temp = temp_of(t, fmt.elemsize());
    let slot = t.offset(temp * fmt.elemsize() as isize);
    (slot as *mut i32).write_unaligned(k);
    (slot.add(4) as *mut i32).write_unaligned(v);
    t
}

/// `stbds_hmgeti(t, k)` for `struct { int key; int value; }` -> (t, index)
pub unsafe fn hmgeti_i32(lib: &Lib, t: *mut u8, k: i32) -> (*mut u8, isize) {
    let fmt = Fmt::BinaryKV;
    let mut key = k;
    let t = lib.hmget_key(
        t as *mut c_void,
        fmt.elemsize(),
        &mut key as *mut i32 as *mut c_void,
        fmt.keysize(),
        HM_BINARY,
    );
    (t, temp_of(t, fmt.elemsize()))
}

/// `stbds_hmdel(t, k)` for `struct { int key; int value; }` -> (t, result)
pub unsafe fn hmdel_i32(lib: &Lib, t: *mut u8, k: i32) -> (*mut u8, isize) {
    let fmt = Fmt::BinaryKV;
    let mut key = k;
    let t = lib.hmdel_key(
        t as *mut c_void,
        fmt.elemsize(),
        &mut key as *mut i32 as *mut c_void,
        fmt.keysize(),
        0,
        HM_BINARY,
    );
    let r = if t.is_null() {
        0
    } else {
        temp_of(t, fmt.elemsize())
    };
    (t, r)
}

/// `stbds_shput(t, k, v)` for `struct { char *key; int value; }`
pub unsafe fn shput(lib: &Lib, t: *mut u8, k: *mut c_char, v: i32) -> *mut u8 {
    let fmt = Fmt::StrKV;
    let t = lib.hmput_key(
        t as *mut c_void,
        fmt.elemsize(),
        k as *mut c_void,
        fmt.keysize(),
        HM_STRING,
    );
    let temp = temp_of(t, fmt.elemsize());
    let slot = t.offset(temp * fmt.elemsize() as isize);
    (slot.add(8) as *mut i32).write_unaligned(v);
    t
}

/// `stbds_shgeti(t, k)` -> (t, index)
pub unsafe fn shgeti(lib: &Lib, t: *mut u8, k: *mut c_char) -> (*mut u8, isize) {
    let fmt = Fmt::StrKV;
    let t = lib.hmget_key(
        t as *mut c_void,
        fmt.elemsize(),
        k as *mut c_void,
        fmt.keysize(),
        HM_STRING,
    );
    (t, temp_of(t, fmt.elemsize()))
}

/// `stbds_shdel(t, k)` -> (t, result)
pub unsafe fn shdel(lib: &Lib, t: *mut u8, k: *mut c_char) -> (*mut u8, isize) {
    let fmt = Fmt::StrKV;
    let t = lib.hmdel_key(
        t as *mut c_void,
        fmt.elemsize(),
        k as *mut c_void,
        fmt.keysize(),
        0,
        HM_STRING,
    );
    let r = if t.is_null() {
        0
    } else {
        temp_of(t, fmt.elemsize())
    };
    (t, r)
}

/// Owned NUL-terminated byte buffer usable as a `char *` argument.
pub struct CStrBuf(pub Vec<u8>);

impl CStrBuf {
    pub fn new(s: &str) -> Self {
        let mut v = s.as_bytes().to_vec();
        v.push(0);
        CStrBuf(v)
    }
    pub fn from_bytes(b: &[u8]) -> Self {
        let mut v = b.to_vec();
        v.push(0);
        CStrBuf(v)
    }
    pub fn as_ptr(&mut self) -> *mut c_char {
        self.0.as_mut_ptr() as *mut c_char
    }
}

/// Both shared objects carry process-global mutable state (`stbds_hash_seed`
/// and `strkey`'s static buffer). Cargo runs integration tests in parallel
/// threads inside one process, so every test must serialise on this lock.
static GLOBAL: std::sync::Mutex<()> = std::sync::Mutex::new(());

pub fn guard() -> std::sync::MutexGuard<'static, ()> {
    GLOBAL.lock().unwrap_or_else(|e| e.into_inner())
}

/// Deterministic PRNG so both libraries see identical input sequences.
pub struct Rng(pub u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Rng(seed | 1)
    }
    pub fn next_u64(&mut self) -> u64 {
        // xorshift64*
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
    pub fn next_i32(&mut self) -> i32 {
        self.next_u64() as i32
    }
    pub fn below(&mut self, n: u64) -> u64 {
        self.next_u64() % n
    }
}
