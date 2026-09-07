//! Differential-test harness: loads BOTH shared objects (the C one built by
//! cmake and the Rust `cdylib`) with `libloading` and exposes their exported
//! symbols behind identical function-pointer tables.  Nothing in the crate is
//! ever called directly, so the `#[no_mangle]` wrappers are under test too.

#![allow(dead_code)]

use std::ffi::{c_char, c_int, c_void, CStr};
use std::path::PathBuf;
use std::sync::OnceLock;

// ---------------------------------------------------------------------------
// C data layout (mirrors `c_src/src/lib.c`)
// ---------------------------------------------------------------------------

pub const STBDS_BUCKET_LENGTH: usize = 8;
pub const STBDS_HM_BINARY: c_int = 0;
pub const STBDS_HM_STRING: c_int = 1;
pub const STBDS_SH_NONE: c_int = 0;
pub const STBDS_SH_DEFAULT: c_int = 1;
pub const STBDS_SH_STRDUP: c_int = 2;
pub const STBDS_SH_ARENA: c_int = 3;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct ArrayHeader {
    pub length: usize,
    pub capacity: usize,
    pub hash_table: *mut c_void,
    pub temp: isize,
}

#[repr(C)]
pub struct StringBlock {
    pub next: *mut StringBlock,
    pub storage: [c_char; 8],
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StringArena {
    pub storage: *mut StringBlock,
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
    pub hash: [usize; STBDS_BUCKET_LENGTH],
    pub index: [isize; STBDS_BUCKET_LENGTH],
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

// ---------------------------------------------------------------------------
// Symbol table
// ---------------------------------------------------------------------------

pub type FnArrGrowf = unsafe extern "C" fn(*mut c_void, usize, usize, usize) -> *mut c_void;
pub type FnArrFreef = unsafe extern "C" fn(*mut c_void);
pub type FnHashBytes = unsafe extern "C" fn(*mut c_void, usize, usize) -> usize;
pub type FnHashString = unsafe extern "C" fn(*mut c_char, usize) -> usize;
pub type FnRandSeed = unsafe extern "C" fn(usize);
pub type FnStralloc = unsafe extern "C" fn(*mut StringArena, *mut c_char) -> *mut c_char;
pub type FnStrreset = unsafe extern "C" fn(*mut StringArena);
pub type FnHmFree = unsafe extern "C" fn(*mut c_void, usize);
pub type FnHmGetKey = unsafe extern "C" fn(*mut c_void, usize, *mut c_void, usize, c_int) -> *mut c_void;
pub type FnHmGetKeyTs =
    unsafe extern "C" fn(*mut c_void, usize, *mut c_void, usize, *mut isize, c_int) -> *mut c_void;
pub type FnHmPutDefault = unsafe extern "C" fn(*mut c_void, usize) -> *mut c_void;
pub type FnHmPutKey = unsafe extern "C" fn(*mut c_void, usize, *mut c_void, usize, c_int) -> *mut c_void;
pub type FnHmDelKey =
    unsafe extern "C" fn(*mut c_void, usize, *mut c_void, usize, usize, c_int) -> *mut c_void;
pub type FnShmodeFunc = unsafe extern "C" fn(usize, c_int) -> *mut c_void;
pub type FnStrPut = unsafe extern "C" fn(c_int);
pub type FnStrkey = unsafe extern "C" fn(c_int) -> *mut c_char;

pub struct Lib {
    pub name: &'static str,
    _lib: &'static libloading::Library,
    pub arrgrowf: FnArrGrowf,
    pub arrfreef: FnArrFreef,
    pub hash_bytes: FnHashBytes,
    pub hash_string: FnHashString,
    pub rand_seed: FnRandSeed,
    pub stralloc: FnStralloc,
    pub strreset: FnStrreset,
    pub hmfree_func: FnHmFree,
    pub hmget_key: FnHmGetKey,
    pub hmget_key_ts: FnHmGetKeyTs,
    pub hmput_default: FnHmPutDefault,
    pub hmput_key: FnHmPutKey,
    pub hmdel_key: FnHmDelKey,
    pub shmode_func: FnShmodeFunc,
    pub str_put: FnStrPut,
    pub strkey: FnStrkey,
}

unsafe impl Send for Lib {}
unsafe impl Sync for Lib {}

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_so() -> PathBuf {
    let base = root().parent().unwrap().join("c_src").join("build");
    for e in std::fs::read_dir(&base).unwrap_or_else(|e| {
        panic!(
            "cannot read {}: {e}. Build the C library first:\n  cd c_src && mkdir -p build && cd build && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
            base.display()
        )
    }) {
        let p = e.unwrap().path();
        if p.extension().map(|x| x == "so").unwrap_or(false) {
            return p;
        }
    }
    panic!("no .so found in {}", base.display());
}

fn rust_so() -> PathBuf {
    let rel = root().join("target/release/libstr_put_lib.so");
    if rel.exists() {
        return rel;
    }
    let dbg = root().join("target/debug/libstr_put_lib.so");
    if dbg.exists() {
        return dbg;
    }
    panic!("Rust cdylib not built; run `cargo build --release` in translation/");
}

unsafe fn load(name: &'static str, path: &std::path::Path) -> Lib {
    let lib: &'static libloading::Library =
        Box::leak(Box::new(libloading::Library::new(path).unwrap_or_else(|e| {
            panic!("dlopen {} failed: {e}", path.display())
        })));
    macro_rules! sym {
        ($n:literal, $t:ty) => {{
            let s: libloading::Symbol<$t> = lib
                .get($n)
                .unwrap_or_else(|e| panic!("{} missing symbol {}: {e}", name, stringify!($n)));
            *s
        }};
    }
    Lib {
        name,
        _lib: lib,
        arrgrowf: sym!(b"stbds_arrgrowf\0", FnArrGrowf),
        arrfreef: sym!(b"stbds_arrfreef\0", FnArrFreef),
        hash_bytes: sym!(b"stbds_hash_bytes\0", FnHashBytes),
        hash_string: sym!(b"stbds_hash_string\0", FnHashString),
        rand_seed: sym!(b"stbds_rand_seed\0", FnRandSeed),
        stralloc: sym!(b"stbds_stralloc\0", FnStralloc),
        strreset: sym!(b"stbds_strreset\0", FnStrreset),
        hmfree_func: sym!(b"stbds_hmfree_func\0", FnHmFree),
        hmget_key: sym!(b"stbds_hmget_key\0", FnHmGetKey),
        hmget_key_ts: sym!(b"stbds_hmget_key_ts\0", FnHmGetKeyTs),
        hmput_default: sym!(b"stbds_hmput_default\0", FnHmPutDefault),
        hmput_key: sym!(b"stbds_hmput_key\0", FnHmPutKey),
        hmdel_key: sym!(b"stbds_hmdel_key\0", FnHmDelKey),
        shmode_func: sym!(b"stbds_shmode_func\0", FnShmodeFunc),
        str_put: sym!(b"str_put\0", FnStrPut),
        strkey: sym!(b"strkey\0", FnStrkey),
    }
}

static LIBS: OnceLock<(Lib, Lib)> = OnceLock::new();

/// `(c, rust)` — both loaded once per test process.
pub fn libs() -> &'static (Lib, Lib) {
    LIBS.get_or_init(|| unsafe { (load("C", &c_so()), load("RUST", &rust_so())) })
}

/// Puts both libraries' `stbds_hash_seed` global into the same state.
pub fn reseed(seed: usize) {
    let (c, r) = libs();
    unsafe {
        (c.rand_seed)(seed);
        (r.rand_seed)(seed);
    }
}

pub const DEFAULT_SEED: usize = 0x31415926;

// ---------------------------------------------------------------------------
// Deterministic RNG (xorshift64*) — fixed seeds keep every test reproducible.
// ---------------------------------------------------------------------------

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Rng(if seed == 0 { 0x9E3779B97F4A7C15 } else { seed })
    }
    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }
    pub fn next_u32(&mut self) -> u32 {
        (self.next_u64() >> 32) as u32
    }
    /// Uniform-ish in `[0, n)`.
    pub fn below(&mut self, n: usize) -> usize {
        if n == 0 {
            0
        } else {
            (self.next_u64() % (n as u64)) as usize
        }
    }
    pub fn bytes(&mut self, n: usize) -> Vec<u8> {
        (0..n).map(|_| self.next_u32() as u8).collect()
    }
    /// NUL-free ASCII-ish string body of `n` bytes (no interior zeros).
    pub fn ascii(&mut self, n: usize) -> Vec<u8> {
        (0..n)
            .map(|_| 0x21u8 + (self.next_u32() % 94) as u8)
            .collect()
    }
    /// NUL-free bytes over the *whole* 1..=255 range (exercises the
    /// `(unsigned char)` widening in `stbds_hash_string`).
    pub fn nonzero(&mut self, n: usize) -> Vec<u8> {
        (0..n)
            .map(|_| 1u8 + (self.next_u32() % 255) as u8)
            .collect()
    }
}

/// `Vec<u8>` -> NUL-terminated buffer usable as `char *`.
pub fn cstr(body: &[u8]) -> Vec<u8> {
    let mut v = body.to_vec();
    v.push(0);
    v
}

// ---------------------------------------------------------------------------
// State snapshots
// ---------------------------------------------------------------------------

#[derive(Debug, PartialEq, Eq, Clone)]
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
    pub buckets: Vec<([usize; 8], [isize; 8])>,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct MapSnap {
    pub is_null: bool,
    pub length: usize,
    pub capacity: usize,
    pub temp: isize,
    pub table: Option<TableSnap>,
    /// Per raw-array element (index 0 is the reserved default slot):
    /// either the raw bytes (binary keys) or `(key string, value bytes)`.
    pub elements: Vec<ElemSnap>,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum ElemSnap {
    Raw(Vec<u8>),
    Str { key: Option<Vec<u8>>, value: Vec<u8> },
}

/// How the elements of a map are laid out, so the snapshot can be taken in a
/// pointer-value-independent way.
///
/// Only the byte ranges the library / the caller actually *writes* are
/// compared: `stbds_arrgrowf` never zeroes the payload, so the padding between
/// the key and the value (and anything past the value) holds `realloc` garbage
/// that legitimately differs between two independent allocators.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum KeyKind {
    /// keys stored inline (binary maps): compare `key[0..keysize]` and
    /// `elem[value_offset .. value_offset+value_size]`.
    Binary {
        keysize: usize,
        value_offset: usize,
        value_size: usize,
    },
    /// keys stored as `char *`: compare the pointed-to string plus the value.
    StringPtr {
        value_offset: usize,
        value_size: usize,
    },
    /// compare the first `n` raw bytes of the element and nothing else (used
    /// for the `STBDS_SH_NONE` + `mode = STBDS_HM_STRING` corner, where the
    /// stored "key" is a memcpy of the string body, not a pointer).
    RawPrefix { n: usize },
}

pub unsafe fn header_of(hash_view: *mut c_void, elemsize: usize) -> *mut ArrayHeader {
    let raw = (hash_view as *mut u8).wrapping_sub(elemsize);
    (raw as *mut ArrayHeader).wrapping_sub(1)
}

pub unsafe fn table_snap(t: *mut HashIndex) -> TableSnap {
    let nb = (*t).slot_count >> 3;
    let mut buckets = Vec::with_capacity(nb);
    for i in 0..nb {
        let b = (*t).storage.wrapping_add(i);
        buckets.push(((*b).hash, (*b).index));
    }
    TableSnap {
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
    }
}

pub unsafe fn map_snap(hash_view: *mut c_void, elemsize: usize, kind: KeyKind) -> MapSnap {
    if hash_view.is_null() {
        return MapSnap {
            is_null: true,
            length: 0,
            capacity: 0,
            temp: 0,
            table: None,
            elements: Vec::new(),
        };
    }
    let raw = (hash_view as *mut u8).wrapping_sub(elemsize) as *mut c_void;
    let h = header_of(hash_view, elemsize);
    let table = (*h).hash_table as *mut HashIndex;
    let length = (*h).length;
    let mut elements = Vec::with_capacity(length);
    for i in 0..length {
        let e = (raw as *mut u8).wrapping_add(elemsize * i);
        match kind {
            KeyKind::Binary {
                keysize,
                value_offset,
                value_size,
            } => {
                let mut v = std::slice::from_raw_parts(e, keysize).to_vec();
                v.extend_from_slice(std::slice::from_raw_parts(
                    e.add(value_offset),
                    value_size,
                ));
                elements.push(ElemSnap::Raw(v));
            }
            KeyKind::StringPtr {
                value_offset,
                value_size,
            } => {
                let kp = *(e as *mut *mut c_char);
                // element 0 is the zeroed "default" slot: its key is NULL.
                let key = if kp.is_null() {
                    None
                } else {
                    Some(CStr::from_ptr(kp).to_bytes().to_vec())
                };
                let value =
                    std::slice::from_raw_parts(e.add(value_offset), value_size).to_vec();
                elements.push(ElemSnap::Str { key, value });
            }
            KeyKind::RawPrefix { n } => {
                elements.push(ElemSnap::Raw(
                    std::slice::from_raw_parts(e, n).to_vec(),
                ));
            }
        }
    }
    MapSnap {
        is_null: false,
        length,
        capacity: (*h).capacity,
        temp: (*h).temp,
        table: if table.is_null() {
            None
        } else {
            Some(table_snap(table))
        },
        elements,
    }
}

/// Snapshot of a raw (headered) dynamic array as produced by `stbds_arrgrowf`.
#[derive(Debug, PartialEq, Eq)]
pub struct ArrSnap {
    pub is_null: bool,
    pub length: usize,
    pub capacity: usize,
    pub temp: isize,
    pub hash_table_null: bool,
    pub payload: Vec<u8>,
}

pub unsafe fn arr_snap(a: *mut c_void, bytes: usize) -> ArrSnap {
    if a.is_null() {
        return ArrSnap {
            is_null: true,
            length: 0,
            capacity: 0,
            temp: 0,
            hash_table_null: true,
            payload: Vec::new(),
        };
    }
    let h = (a as *mut ArrayHeader).wrapping_sub(1);
    ArrSnap {
        is_null: false,
        length: (*h).length,
        capacity: (*h).capacity,
        temp: (*h).temp,
        hash_table_null: (*h).hash_table.is_null(),
        payload: std::slice::from_raw_parts(a as *mut u8, bytes).to_vec(),
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct ArenaSnap {
    pub remaining: usize,
    pub block: u8,
    pub mode: u8,
    pub storage_null: bool,
    /// Number of blocks reachable through `next`.
    pub block_count: usize,
}

pub unsafe fn arena_snap(a: *const StringArena) -> ArenaSnap {
    let mut n = 0usize;
    let mut x = (*a).storage;
    while !x.is_null() && n < 1_000_000 {
        n += 1;
        x = (*x).next;
    }
    ArenaSnap {
        remaining: (*a).remaining,
        block: (*a).block,
        mode: (*a).mode,
        storage_null: (*a).storage.is_null(),
        block_count: n,
    }
}

// ---------------------------------------------------------------------------
// stdout capture (for `str_put`, which prints through libc `printf`)
// ---------------------------------------------------------------------------

extern "C" {
    fn dup(oldfd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn open(path: *const c_char, flags: c_int, mode: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
}

pub static STDOUT_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// Runs `f` with fd 1 redirected into a temp file and returns everything the
/// C `printf` inside the shared object wrote.
pub fn capture_stdout<F: FnOnce()>(tag: &str, f: F) -> Vec<u8> {
    let _g = STDOUT_LOCK.lock().unwrap();
    let path = std::env::temp_dir().join(format!(
        "stbds_cap_{}_{}_{}.txt",
        std::process::id(),
        tag,
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let cpath = std::ffi::CString::new(path.to_str().unwrap()).unwrap();
    unsafe {
        fflush(std::ptr::null_mut());
        let saved = dup(1);
        assert!(saved >= 0, "dup(1) failed");
        // O_WRONLY | O_CREAT | O_TRUNC on Linux
        let fd = open(cpath.as_ptr(), 1 | 64 | 512, 0o644);
        assert!(fd >= 0, "open({}) failed", path.display());
        assert!(dup2(fd, 1) >= 0, "dup2 failed");
        close(fd);

        f();

        fflush(std::ptr::null_mut());
        dup2(saved, 1);
        close(saved);
    }
    let out = std::fs::read(&path).unwrap_or_default();
    let _ = std::fs::remove_file(&path);
    out
}

// ---------------------------------------------------------------------------
// Lock-step map driver
//
// Reproduces what the `stbds_hm*` / `stbds_sh*` macros expand to, driving the C
// and the Rust `.so` through the *same* call sequence and comparing the whole
// observable state (array header, every element, the entire hash index and all
// buckets) after every single operation.
// ---------------------------------------------------------------------------

pub struct MapPair {
    pub c: *mut c_void,
    pub r: *mut c_void,
    pub elemsize: usize,
    pub keysize: usize,
    pub voff: usize,
    pub mode: c_int,
    pub kind: KeyKind,
    /// Keeps key storage alive: `STBDS_SH_DEFAULT` stores the caller's pointer.
    keep: Vec<Box<[u8]>>,
}

impl MapPair {
    /// Empty (NULL) map — the table is created lazily by `hmput_key`.
    pub fn new(elemsize: usize, keysize: usize, voff: usize, mode: c_int, kind: KeyKind) -> Self {
        MapPair {
            c: std::ptr::null_mut(),
            r: std::ptr::null_mut(),
            elemsize,
            keysize,
            voff,
            mode,
            kind,
            keep: Vec::new(),
        }
    }

    /// `stbds_sh_new_arena` / `stbds_sh_new_strdup` style construction:
    /// `t = stbds_shmode_func(sizeof *t, sh_mode)`.
    pub fn with_shmode(
        elemsize: usize,
        keysize: usize,
        voff: usize,
        mode: c_int,
        kind: KeyKind,
        sh_mode: c_int,
    ) -> Self {
        let (c, r) = libs();
        let mut m = Self::new(elemsize, keysize, voff, mode, kind);
        unsafe {
            m.c = (c.shmode_func)(elemsize, sh_mode);
            m.r = (r.shmode_func)(elemsize, sh_mode);
        }
        m
    }

    fn hold(&mut self, bytes: &[u8]) -> *mut c_void {
        self.keep.push(bytes.to_vec().into_boxed_slice());
        self.keep.last_mut().unwrap().as_mut_ptr() as *mut c_void
    }

    pub unsafe fn temp(&self, p: *mut c_void) -> isize {
        (*header_of(p, self.elemsize)).temp
    }

    /// `stbds_hmput(t,k,v)` for binary keys / `stbds_shput(t,k,v)` for string
    /// keys.  `key` must already be NUL terminated for string modes.
    pub fn put(&mut self, key: &[u8], value: &[u8]) {
        let (lc, lr) = libs();
        let kp = self.hold(key);
        unsafe {
            self.c = (lc.hmput_key)(self.c, self.elemsize, kp, self.keysize, self.mode);
            self.r = (lr.hmput_key)(self.r, self.elemsize, kp, self.keysize, self.mode);
            for p in [self.c, self.r] {
                let t = self.temp(p);
                let e = (p as *mut u8).wrapping_add(self.elemsize * (t as usize));
                if let KeyKind::Binary { .. } = self.kind {
                    // `(t)[temp].key = (k)` — the macro re-assigns the key too.
                    std::ptr::copy_nonoverlapping(key.as_ptr(), e, self.keysize);
                }
                let n = value.len().min(self.elemsize - self.voff);
                std::ptr::copy_nonoverlapping(value.as_ptr(), e.add(self.voff), n);
            }
        }
    }

    /// `stbds_shputs(t,s)` — additionally writes back `stbds_temp_key`.
    pub fn puts_string(&mut self, key: &[u8], value: &[u8]) {
        let (lc, lr) = libs();
        let kp = self.hold(key);
        unsafe {
            self.c = (lc.hmput_key)(self.c, self.elemsize, kp, self.keysize, self.mode);
            self.r = (lr.hmput_key)(self.r, self.elemsize, kp, self.keysize, self.mode);
            for p in [self.c, self.r] {
                let raw = (p as *mut u8).wrapping_sub(self.elemsize) as *mut c_void;
                let t = self.temp(p);
                let e = (p as *mut u8).wrapping_add(self.elemsize * (t as usize));
                let n = value.len().min(self.elemsize - self.voff);
                std::ptr::copy_nonoverlapping(value.as_ptr(), e.add(self.voff), n);
                // (t)[temp].key = stbds_temp_key((t)-1)
                let tk = (*((raw as *mut ArrayHeader).wrapping_sub(1))).hash_table
                    as *mut *mut c_char;
                *(e as *mut *mut c_char) = *tk;
            }
        }
    }

    /// `stbds_hmgeti` / `stbds_shgeti` — returns `(c_temp, rust_temp)`.
    pub fn geti(&mut self, key: &[u8]) -> (isize, isize) {
        let (lc, lr) = libs();
        let kp = key.as_ptr() as *mut c_void;
        unsafe {
            self.c = (lc.hmget_key)(self.c, self.elemsize, kp, self.keysize, self.mode);
            self.r = (lr.hmget_key)(self.r, self.elemsize, kp, self.keysize, self.mode);
            (self.temp(self.c), self.temp(self.r))
        }
    }

    /// `stbds_hmgeti_ts` — returns `(c_temp, rust_temp)` from the out-param.
    pub fn geti_ts(&mut self, key: &[u8]) -> (isize, isize) {
        let (lc, lr) = libs();
        let kp = key.as_ptr() as *mut c_void;
        let mut tc: isize = 0x5a5a;
        let mut tr: isize = 0x5a5a;
        unsafe {
            self.c = (lc.hmget_key_ts)(self.c, self.elemsize, kp, self.keysize, &mut tc, self.mode);
            self.r = (lr.hmget_key_ts)(self.r, self.elemsize, kp, self.keysize, &mut tr, self.mode);
        }
        (tc, tr)
    }

    /// `stbds_hmdel` / `stbds_shdel` — returns `(c_result, rust_result)`.
    pub fn del(&mut self, key: &[u8]) -> (isize, isize) {
        self.del_mode(key, self.mode)
    }

    /// Same but with an explicit `mode` (the `hmdel_key` `mode ==
    /// STBDS_HM_STRING` branch differs from `mode >= STBDS_HM_STRING`).
    pub fn del_mode(&mut self, key: &[u8], mode: c_int) -> (isize, isize) {
        let (lc, lr) = libs();
        let kp = key.as_ptr() as *mut c_void;
        unsafe {
            self.c = (lc.hmdel_key)(self.c, self.elemsize, kp, self.keysize, 0, mode);
            self.r = (lr.hmdel_key)(self.r, self.elemsize, kp, self.keysize, 0, mode);
            let a = if self.c.is_null() { 0 } else { self.temp(self.c) };
            let b = if self.r.is_null() { 0 } else { self.temp(self.r) };
            (a, b)
        }
    }

    /// `stbds_hmdefault` bootstrap: `t = stbds_hmput_default(t, sizeof *t)`.
    pub fn put_default(&mut self) {
        let (lc, lr) = libs();
        unsafe {
            self.c = (lc.hmput_default)(self.c, self.elemsize);
            self.r = (lr.hmput_default)(self.r, self.elemsize);
        }
    }

    pub fn snap_pair(&self) -> (MapSnap, MapSnap) {
        unsafe {
            (
                map_snap(self.c, self.elemsize, self.kind),
                map_snap(self.r, self.elemsize, self.kind),
            )
        }
    }

    pub fn assert_same(&self, ctx: &str) {
        let (a, b) = self.snap_pair();
        if a != b {
            panic!("state divergence [{ctx}]\n  C   : {a:?}\n  RUST: {b:?}");
        }
    }

    /// `stbds_hmlen` / `stbds_shlen`.
    pub fn len(&self) -> (isize, isize) {
        unsafe {
            let f = |p: *mut c_void| {
                if p.is_null() {
                    0isize
                } else {
                    (*header_of(p, self.elemsize)).length as isize - 1
                }
            };
            (f(self.c), f(self.r))
        }
    }

    /// `stbds_hmfree` / `stbds_shfree`.
    pub fn free(&mut self) {
        let (lc, lr) = libs();
        unsafe {
            if !self.c.is_null() {
                (lc.hmfree_func)(
                    (self.c as *mut u8).wrapping_sub(self.elemsize) as *mut c_void,
                    self.elemsize,
                );
            }
            if !self.r.is_null() {
                (lr.hmfree_func)(
                    (self.r as *mut u8).wrapping_sub(self.elemsize) as *mut c_void,
                    self.elemsize,
                );
            }
        }
        self.c = std::ptr::null_mut();
        self.r = std::ptr::null_mut();
        self.keep.clear();
    }
}

// ---------------------------------------------------------------------------
// Serialisation
//
// `stbds_hash_seed` is a process-wide mutable global *inside each `.so`*, and
// `stbds_make_hash_index` advances it on every fresh table.  Tests that pin it
// with `reseed()` must therefore not run concurrently with each other.
// ---------------------------------------------------------------------------

static SERIAL: std::sync::Mutex<()> = std::sync::Mutex::new(());

pub fn serial() -> std::sync::MutexGuard<'static, ()> {
    SERIAL.lock().unwrap_or_else(|e| e.into_inner())
}

// ---------------------------------------------------------------------------
// Crash / abort comparison
//
// Several of the rows in `ERRORS.md` are `assert()`s or genuinely undefined
// operations (`stbds_arrfreef(NULL)` frees `(header*)NULL - 1`).  They are
// compared by running the call in a forked child and comparing the raw wait
// status (exit code / fatal signal) plus whatever the child wrote to stderr.
// ---------------------------------------------------------------------------

extern "C" {
    fn fork() -> c_int;
    fn waitpid(pid: c_int, status: *mut c_int, options: c_int) -> c_int;
    fn _exit(code: c_int) -> !;
}

#[derive(Debug, PartialEq, Eq)]
pub struct ChildOutcome {
    /// 0 for a normal `_exit(0)`, otherwise the fatal signal number.
    pub signal: i32,
    pub exit_code: i32,
}

pub fn child_run<F: FnOnce()>(tag: &str, f: F) -> (ChildOutcome, Vec<u8>) {
    let path = std::env::temp_dir().join(format!(
        "stbds_child_{}_{}_{}.err",
        std::process::id(),
        tag,
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let cpath = std::ffi::CString::new(path.to_str().unwrap()).unwrap();
    let status;
    unsafe {
        fflush(std::ptr::null_mut());
        let pid = fork();
        assert!(pid >= 0, "fork failed");
        if pid == 0 {
            let fd = open(cpath.as_ptr(), 1 | 64 | 512, 0o644);
            if fd >= 0 {
                dup2(fd, 2);
                close(fd);
            }
            // keep the child's own stdout out of the test harness's output
            let devnull = std::ffi::CString::new("/dev/null").unwrap();
            let nfd = open(devnull.as_ptr(), 1, 0);
            if nfd >= 0 {
                dup2(nfd, 1);
                close(nfd);
            }
            f();
            fflush(std::ptr::null_mut());
            _exit(0);
        }
        let mut st: c_int = 0;
        waitpid(pid, &mut st, 0);
        status = st;
    }
    let signal = status & 0x7f;
    let exit_code = (status >> 8) & 0xff;
    let err = std::fs::read(&path).unwrap_or_default();
    let _ = std::fs::remove_file(&path);
    (
        ChildOutcome {
            signal,
            exit_code: if signal == 0 { exit_code } else { 0 },
        },
        err,
    )
}
