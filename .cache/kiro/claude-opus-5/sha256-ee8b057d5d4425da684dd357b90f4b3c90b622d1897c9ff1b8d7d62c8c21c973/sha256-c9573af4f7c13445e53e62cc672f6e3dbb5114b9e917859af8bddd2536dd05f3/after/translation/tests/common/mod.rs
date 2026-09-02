//! Shared harness: loads BOTH the C `.so` and the Rust `.so` with `libloading`
//! and calls everything through the FFI boundary, so the `#[no_mangle]` export
//! wrappers are under test too.  No Rust function is ever called directly.

#![allow(dead_code)]

use libloading::Library;
use std::ffi::{c_char, c_int, c_void};
use std::path::PathBuf;
use std::sync::{Mutex, MutexGuard, OnceLock};

// ---------------------------------------------------------------------------
// Layouts mirrored from c_src/src/lib.c (used only to *read* state back)
// ---------------------------------------------------------------------------

#[repr(C)]
#[derive(Clone, Copy)]
pub struct Header {
    pub length: usize,
    pub capacity: usize,
    pub hash_table: *mut c_void,
    pub temp: isize,
}
pub const HEADER_SIZE: usize = std::mem::size_of::<Header>();

#[repr(C)]
#[derive(Clone, Copy)]
pub struct Arena {
    pub storage: *mut c_void,
    pub remaining: usize,
    pub block: u8,
    pub mode: u8,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct Bucket {
    pub hash: [usize; 8],
    pub index: [isize; 8],
}

#[repr(C)]
#[derive(Clone, Copy)]
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

pub const HM_BINARY: c_int = 0;
pub const HM_STRING: c_int = 1;

pub const SH_NONE: c_int = 0;
pub const SH_DEFAULT: c_int = 1;
pub const SH_STRDUP: c_int = 2;
pub const SH_ARENA: c_int = 3;

// ---------------------------------------------------------------------------
// The 16 exported symbols
// ---------------------------------------------------------------------------

pub type FnArrGrowf = unsafe extern "C" fn(*mut c_void, usize, usize, usize) -> *mut c_void;
pub type FnArrFreef = unsafe extern "C" fn(*mut c_void);
pub type FnHashBytes = unsafe extern "C" fn(*mut c_void, usize, usize) -> usize;
pub type FnHashString = unsafe extern "C" fn(*mut c_char, usize) -> usize;
pub type FnRandSeed = unsafe extern "C" fn(usize);
pub type FnHmFree = unsafe extern "C" fn(*mut c_void, usize);
pub type FnHmGetKey = unsafe extern "C" fn(*mut c_void, usize, *mut c_void, usize, c_int) -> *mut c_void;
pub type FnHmGetKeyTs =
    unsafe extern "C" fn(*mut c_void, usize, *mut c_void, usize, *mut isize, c_int) -> *mut c_void;
pub type FnHmPutDefault = unsafe extern "C" fn(*mut c_void, usize) -> *mut c_void;
pub type FnHmPutKey = unsafe extern "C" fn(*mut c_void, usize, *mut c_void, usize, c_int) -> *mut c_void;
pub type FnHmDelKey =
    unsafe extern "C" fn(*mut c_void, usize, *mut c_void, usize, usize, c_int) -> *mut c_void;
pub type FnShmodeFunc = unsafe extern "C" fn(usize, c_int) -> *mut c_void;
pub type FnStralloc = unsafe extern "C" fn(*mut Arena, *mut c_char) -> *mut c_char;
pub type FnStrreset = unsafe extern "C" fn(*mut Arena);
pub type FnStrkey = unsafe extern "C" fn(c_int) -> *mut c_char;
pub type FnArrDel = unsafe extern "C" fn(c_int);

pub struct Lib {
    pub name: &'static str,
    _lib: Library,
    pub arrgrowf: FnArrGrowf,
    pub arrfreef: FnArrFreef,
    pub hash_bytes: FnHashBytes,
    pub hash_string: FnHashString,
    pub rand_seed: FnRandSeed,
    pub hmfree_func: FnHmFree,
    pub hmget_key: FnHmGetKey,
    pub hmget_key_ts: FnHmGetKeyTs,
    pub hmput_default: FnHmPutDefault,
    pub hmput_key: FnHmPutKey,
    pub hmdel_key: FnHmDelKey,
    pub shmode_func: FnShmodeFunc,
    pub stralloc: FnStralloc,
    pub strreset: FnStrreset,
    pub strkey: FnStrkey,
    pub arr_del: FnArrDel,
}

macro_rules! sym {
    ($lib:expr, $name:literal, $ty:ty) => {{
        let s: libloading::Symbol<$ty> = unsafe { $lib.get(concat!($name, "\0").as_bytes()) }
            .unwrap_or_else(|e| panic!("missing symbol {}: {e}", $name));
        *s
    }};
}

impl Lib {
    fn open(name: &'static str, path: &PathBuf) -> Lib {
        let lib = unsafe { Library::new(path) }
            .unwrap_or_else(|e| panic!("failed to dlopen {}: {e}", path.display()));
        Lib {
            name,
            arrgrowf: sym!(lib, "stbds_arrgrowf", FnArrGrowf),
            arrfreef: sym!(lib, "stbds_arrfreef", FnArrFreef),
            hash_bytes: sym!(lib, "stbds_hash_bytes", FnHashBytes),
            hash_string: sym!(lib, "stbds_hash_string", FnHashString),
            rand_seed: sym!(lib, "stbds_rand_seed", FnRandSeed),
            hmfree_func: sym!(lib, "stbds_hmfree_func", FnHmFree),
            hmget_key: sym!(lib, "stbds_hmget_key", FnHmGetKey),
            hmget_key_ts: sym!(lib, "stbds_hmget_key_ts", FnHmGetKeyTs),
            hmput_default: sym!(lib, "stbds_hmput_default", FnHmPutDefault),
            hmput_key: sym!(lib, "stbds_hmput_key", FnHmPutKey),
            hmdel_key: sym!(lib, "stbds_hmdel_key", FnHmDelKey),
            shmode_func: sym!(lib, "stbds_shmode_func", FnShmodeFunc),
            stralloc: sym!(lib, "stbds_stralloc", FnStralloc),
            strreset: sym!(lib, "stbds_strreset", FnStrreset),
            strkey: sym!(lib, "strkey", FnStrkey),
            arr_del: sym!(lib, "arr_del", FnArrDel),
            _lib: lib,
        }
    }
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn find_c_so() -> PathBuf {
    if let Ok(p) = std::env::var("C_SO") {
        return PathBuf::from(p);
    }
    let build = manifest_dir().join("../c_src/build");
    let mut found: Vec<PathBuf> = std::fs::read_dir(&build)
        .unwrap_or_else(|e| {
            panic!(
                "cannot read {} ({e}); build the C library first:\n  cd c_src && mkdir -p build && cd build && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
                build.display()
            )
        })
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| {
            p.extension().map(|x| x == "so").unwrap_or(false)
                && p.file_name()
                    .map(|n| n.to_string_lossy().starts_with("lib"))
                    .unwrap_or(false)
        })
        .collect();
    found.sort();
    assert_eq!(
        found.len(),
        1,
        "expected exactly one lib*.so in {}, found {found:?}",
        build.display()
    );
    found.pop().unwrap()
}

fn find_rust_so() -> PathBuf {
    if let Ok(p) = std::env::var("RUST_SO") {
        return PathBuf::from(p);
    }
    let rel = manifest_dir().join("target/release/libarr_del_lib.so");
    if rel.exists() {
        return rel;
    }
    let dbg = manifest_dir().join("target/debug/libarr_del_lib.so");
    assert!(
        dbg.exists(),
        "no Rust .so found; run `cargo build --release` in translation/ first"
    );
    dbg
}

pub struct Pair {
    pub c: Lib,
    pub r: Lib,
}

// The two libraries carry mutable process-global state (`stbds_hash_seed`, the
// `strkey` buffer), so scenarios must not run concurrently.
struct Shared(Pair);
unsafe impl Sync for Shared {}
unsafe impl Send for Shared {}

static PAIR: OnceLock<Shared> = OnceLock::new();
static LOCK: Mutex<()> = Mutex::new(());

pub fn libs() -> (&'static Pair, MutexGuard<'static, ()>) {
    let g = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let s = PAIR.get_or_init(|| {
        Shared(Pair {
            c: Lib::open("C", &find_c_so()),
            r: Lib::open("Rust", &find_rust_so()),
        })
    });
    (&s.0, g)
}

// ---------------------------------------------------------------------------
// Deterministic RNG (xorshift64*), fixed seeds for reproducibility
// ---------------------------------------------------------------------------

pub struct Rng(u64);

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
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
    pub fn next_u32(&mut self) -> u32 {
        (self.next_u64() >> 32) as u32
    }
    pub fn below(&mut self, n: usize) -> usize {
        if n == 0 {
            0
        } else {
            (self.next_u64() % n as u64) as usize
        }
    }
    pub fn bytes(&mut self, n: usize) -> Vec<u8> {
        (0..n).map(|_| (self.next_u64() >> 24) as u8).collect()
    }
    /// NUL-terminated random printable string of `n` payload bytes.
    pub fn cstring(&mut self, n: usize) -> Vec<u8> {
        let mut v: Vec<u8> = (0..n).map(|_| b'a' + (self.below(26) as u8)).collect();
        v.push(0);
        v
    }
}

// ---------------------------------------------------------------------------
// State readers / snapshots
// ---------------------------------------------------------------------------

pub unsafe fn header(raw: *mut c_void) -> Header {
    *((raw as *mut u8).wrapping_sub(HEADER_SIZE) as *const Header)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TableSnap {
    pub slot_count: usize,
    pub used_count: usize,
    pub used_count_threshold: usize,
    pub used_count_shrink_threshold: usize,
    pub tombstone_count: usize,
    pub tombstone_count_threshold: usize,
    pub seed: usize,
    pub slot_count_log2: usize,
    pub arena_remaining: usize,
    pub arena_block: u8,
    pub arena_mode: u8,
    pub arena_has_storage: bool,
    pub buckets: Vec<([usize; 8], [isize; 8])>,
}

pub unsafe fn table_snap(t: *const HashIndex) -> Option<TableSnap> {
    if t.is_null() {
        return None;
    }
    let ti = &*t;
    let nbuckets = ti.slot_count >> 3;
    let mut buckets = Vec::with_capacity(nbuckets);
    for i in 0..nbuckets {
        let b = &*ti.storage.wrapping_add(i);
        buckets.push((b.hash, b.index));
    }
    Some(TableSnap {
        slot_count: ti.slot_count,
        used_count: ti.used_count,
        used_count_threshold: ti.used_count_threshold,
        used_count_shrink_threshold: ti.used_count_shrink_threshold,
        tombstone_count: ti.tombstone_count,
        tombstone_count_threshold: ti.tombstone_count_threshold,
        seed: ti.seed,
        slot_count_log2: ti.slot_count_log2,
        arena_remaining: ti.string.remaining,
        arena_block: ti.string.block,
        arena_mode: ti.string.mode,
        arena_has_storage: !ti.string.storage.is_null(),
        buckets,
    })
}

/// `stbds_temp_key(t)` == `*(char **) header(t)->hash_table` == `table->temp_key`.
/// Only defined right after a string-mode `hmput_key`; `stbds_make_hash_index`
/// leaves it uninitialised, so it is NOT part of `TableSnap`.
pub unsafe fn temp_key_str(raw: *mut c_void) -> Option<Vec<u8>> {
    let ht = header(raw).hash_table as *const HashIndex;
    if ht.is_null() {
        return None;
    }
    cstr_bytes((*ht).temp_key)
}

/// How to read the element payload back for comparison.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Pay {
    /// Every byte of every element is caller-initialised -> compare verbatim.
    Raw,
    /// Element starts with a `char *` key; compare the pointed-to string plus
    /// the remaining `elemsize - 8` bytes verbatim.
    StrPtr,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Elem {
    Raw(Vec<u8>),
    Str(Option<Vec<u8>>, Vec<u8>),
}

/// Full observable state of a hash-map array, given the RAW array pointer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Snap {
    pub null: bool,
    pub length: usize,
    pub capacity: usize,
    pub temp: isize,
    pub has_table: bool,
    pub table: Option<TableSnap>,
    pub elems: Vec<Elem>,
}

unsafe fn cstr_bytes(p: *const c_char) -> Option<Vec<u8>> {
    if p.is_null() {
        None
    } else {
        Some(std::ffi::CStr::from_ptr(p).to_bytes().to_vec())
    }
}

pub unsafe fn snap_raw(raw: *mut c_void, elemsize: usize, pay: Pay) -> Snap {
    if raw.is_null() {
        return Snap {
            null: true,
            length: 0,
            capacity: 0,
            temp: 0,
            has_table: false,
            table: None,
            elems: vec![],
        };
    }
    let h = header(raw);
    let mut elems = Vec::with_capacity(h.length);
    for i in 0..h.length {
        let base = (raw as *mut u8).wrapping_add(elemsize * i);
        match pay {
            Pay::Raw => {
                elems.push(Elem::Raw(std::slice::from_raw_parts(base, elemsize).to_vec()));
            }
            Pay::StrPtr => {
                let kp = *(base as *const *const c_char);
                let rest = if elemsize > 8 {
                    std::slice::from_raw_parts(base.wrapping_add(8), elemsize - 8).to_vec()
                } else {
                    vec![]
                };
                elems.push(Elem::Str(cstr_bytes(kp), rest));
            }
        }
    }
    Snap {
        null: false,
        length: h.length,
        capacity: h.capacity,
        temp: h.temp,
        has_table: !h.hash_table.is_null(),
        table: table_snap(h.hash_table as *const HashIndex),
        elems,
    }
}

/// Snapshot from a HASH pointer (what `hmput_key` etc. return).
pub unsafe fn snap_hash(t: *mut c_void, elemsize: usize, pay: Pay) -> Snap {
    if t.is_null() {
        return snap_raw(std::ptr::null_mut(), elemsize, pay);
    }
    snap_raw((t as *mut u8).wrapping_sub(elemsize) as *mut c_void, elemsize, pay)
}

/// Snapshot of a plain `arrgrowf` array: header + the first `used` elements.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArrSnap {
    pub length: usize,
    pub capacity: usize,
    pub temp: isize,
    pub table_null: bool,
    pub bytes: Vec<u8>,
}

pub unsafe fn snap_arr(raw: *mut c_void, nbytes: usize) -> ArrSnap {
    let h = header(raw);
    ArrSnap {
        length: h.length,
        capacity: h.capacity,
        temp: h.temp,
        table_null: h.hash_table.is_null(),
        bytes: std::slice::from_raw_parts(raw as *const u8, nbytes).to_vec(),
    }
}

// ---------------------------------------------------------------------------
// Arena snapshot
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArenaSnap {
    pub remaining: usize,
    pub block: u8,
    pub mode: u8,
    pub has_storage: bool,
    /// number of blocks in the chain
    pub chain_len: usize,
}

pub unsafe fn snap_arena(a: *const Arena) -> ArenaSnap {
    let ar = *a;
    let mut n = 0usize;
    let mut p = ar.storage as *const *const c_void; // block.next is the first field
    while !p.is_null() {
        n += 1;
        if n > 100_000 {
            break;
        }
        p = *p as *const *const c_void;
    }
    ArenaSnap {
        remaining: ar.remaining,
        block: ar.block,
        mode: ar.mode,
        has_storage: !ar.storage.is_null(),
        chain_len: n,
    }
}

// ---------------------------------------------------------------------------
// Mirrored hash-map driver: runs the same op on both libraries.
// ---------------------------------------------------------------------------

/// Emulates what the `stbds_hmput` macro does around `stbds_hmput_key`:
/// call the raw entry point, then write the value into `t[temp]`.
pub struct Map<'a> {
    pub c: *mut c_void,
    pub r: *mut c_void,
    pub elemsize: usize,
    pub keysize: usize,
    pub mode: c_int,
    pub pay: Pay,
    pub lib: &'a Pair,
}

impl<'a> Map<'a> {
    pub fn new(lib: &'a Pair, elemsize: usize, keysize: usize, mode: c_int, pay: Pay) -> Map<'a> {
        Map {
            c: std::ptr::null_mut(),
            r: std::ptr::null_mut(),
            elemsize,
            keysize,
            mode,
            pay,
            lib,
        }
    }

    pub fn from_shmode(
        lib: &'a Pair,
        elemsize: usize,
        keysize: usize,
        mode: c_int,
        shmode: c_int,
        pay: Pay,
    ) -> Map<'a> {
        let c = unsafe { (lib.c.shmode_func)(elemsize, shmode) };
        let r = unsafe { (lib.r.shmode_func)(elemsize, shmode) };
        Map {
            c,
            r,
            elemsize,
            keysize,
            mode,
            pay,
            lib,
        }
    }

    pub fn raw_c(&self) -> *mut c_void {
        if self.c.is_null() {
            std::ptr::null_mut()
        } else {
            (self.c as *mut u8).wrapping_sub(self.elemsize) as *mut c_void
        }
    }
    pub fn raw_r(&self) -> *mut c_void {
        if self.r.is_null() {
            std::ptr::null_mut()
        } else {
            (self.r as *mut u8).wrapping_sub(self.elemsize) as *mut c_void
        }
    }

    /// `hmput_key` on both, then write `value` at `t[temp]` + `voff`.
    /// `key_c`/`key_r` are separate buffers so the two libraries never share
    /// caller memory (important for SH_DEFAULT, which stores the pointer).
    pub unsafe fn put(&mut self, key_c: *mut c_void, key_r: *mut c_void, value: &[u8], voff: usize) {
        self.c = (self.lib.c.hmput_key)(self.c, self.elemsize, key_c, self.keysize, self.mode);
        self.r = (self.lib.r.hmput_key)(self.r, self.elemsize, key_r, self.keysize, self.mode);
        if !value.is_empty() {
            for (t, raw) in [(self.c, self.raw_c()), (self.r, self.raw_r())] {
                let _ = t;
                let temp = header(raw).temp;
                let dst = (raw as *mut u8).wrapping_add(self.elemsize * ((temp + 1) as usize) + voff);
                std::ptr::copy_nonoverlapping(value.as_ptr(), dst, value.len());
            }
        }
    }

    pub unsafe fn get_ts(&mut self, key_c: *mut c_void, key_r: *mut c_void) -> (isize, isize) {
        let mut tc: isize = 0x5A5A;
        let mut tr: isize = 0x5A5A;
        self.c = (self.lib.c.hmget_key_ts)(self.c, self.elemsize, key_c, self.keysize, &mut tc, self.mode);
        self.r = (self.lib.r.hmget_key_ts)(self.r, self.elemsize, key_r, self.keysize, &mut tr, self.mode);
        (tc, tr)
    }

    pub unsafe fn get(&mut self, key_c: *mut c_void, key_r: *mut c_void) -> (isize, isize) {
        self.c = (self.lib.c.hmget_key)(self.c, self.elemsize, key_c, self.keysize, self.mode);
        self.r = (self.lib.r.hmget_key)(self.r, self.elemsize, key_r, self.keysize, self.mode);
        (header(self.raw_c()).temp, header(self.raw_r()).temp)
    }

    pub unsafe fn del(&mut self, key_c: *mut c_void, key_r: *mut c_void, keyoffset: usize) {
        self.c = (self.lib.c.hmdel_key)(
            self.c,
            self.elemsize,
            key_c,
            self.keysize,
            keyoffset,
            self.mode,
        );
        self.r = (self.lib.r.hmdel_key)(
            self.r,
            self.elemsize,
            key_r,
            self.keysize,
            keyoffset,
            self.mode,
        );
    }

    pub unsafe fn snaps(&self) -> (Snap, Snap) {
        (
            snap_raw(self.raw_c(), self.elemsize, self.pay),
            snap_raw(self.raw_r(), self.elemsize, self.pay),
        )
    }

    pub unsafe fn assert_eq(&self, what: &str) {
        let (a, b) = self.snaps();
        assert_snap(&a, &b, what);
    }

    pub unsafe fn free(&mut self) {
        if !self.c.is_null() {
            (self.lib.c.hmfree_func)(self.raw_c(), self.elemsize);
        }
        if !self.r.is_null() {
            (self.lib.r.hmfree_func)(self.raw_r(), self.elemsize);
        }
        self.c = std::ptr::null_mut();
        self.r = std::ptr::null_mut();
    }
}

pub fn assert_snap(c: &Snap, r: &Snap, what: &str) {
    if c != r {
        // Produce a focused diff rather than dumping two huge structs.
        let mut diffs: Vec<String> = vec![];
        if c.null != r.null {
            diffs.push(format!("null: C={} Rust={}", c.null, r.null));
        }
        if c.length != r.length {
            diffs.push(format!("length: C={} Rust={}", c.length, r.length));
        }
        if c.capacity != r.capacity {
            diffs.push(format!("capacity: C={} Rust={}", c.capacity, r.capacity));
        }
        if c.temp != r.temp {
            diffs.push(format!("temp: C={} Rust={}", c.temp, r.temp));
        }
        if c.has_table != r.has_table {
            diffs.push(format!("has_table: C={} Rust={}", c.has_table, r.has_table));
        }
        match (&c.table, &r.table) {
            (Some(a), Some(b)) if a != b => {
                macro_rules! f {
                    ($n:ident) => {
                        if a.$n != b.$n {
                            diffs.push(format!("table.{}: C={:?} Rust={:?}", stringify!($n), a.$n, b.$n));
                        }
                    };
                }
                f!(slot_count);
                f!(used_count);
                f!(used_count_threshold);
                f!(used_count_shrink_threshold);
                f!(tombstone_count);
                f!(tombstone_count_threshold);
                f!(seed);
                f!(slot_count_log2);
                f!(arena_remaining);
                f!(arena_block);
                f!(arena_mode);
                f!(arena_has_storage);
                if a.buckets != b.buckets {
                    for (i, (x, y)) in a.buckets.iter().zip(b.buckets.iter()).enumerate() {
                        if x != y {
                            diffs.push(format!("bucket[{i}]: C={x:?} Rust={y:?}"));
                        }
                    }
                    if a.buckets.len() != b.buckets.len() {
                        diffs.push(format!(
                            "bucket count: C={} Rust={}",
                            a.buckets.len(),
                            b.buckets.len()
                        ));
                    }
                }
            }
            (x, y) if x.is_some() != y.is_some() => {
                diffs.push(format!("table presence: C={} Rust={}", x.is_some(), y.is_some()));
            }
            _ => {}
        }
        for (i, (x, y)) in c.elems.iter().zip(r.elems.iter()).enumerate() {
            if x != y {
                diffs.push(format!("elem[{i}]: C={x:?} Rust={y:?}"));
            }
        }
        if c.elems.len() != r.elems.len() {
            diffs.push(format!("elem count: C={} Rust={}", c.elems.len(), r.elems.len()));
        }
        panic!("DIVERGENCE [{what}]:\n  {}", diffs.join("\n  "));
    }
}

// ---------------------------------------------------------------------------
// misc helpers
// ---------------------------------------------------------------------------

pub fn seed_both(l: &Pair, seed: usize) {
    unsafe {
        (l.c.rand_seed)(seed);
        (l.r.rand_seed)(seed);
    }
}

/// Heap buffer that stays alive for the whole scenario (needed by SH_DEFAULT,
/// which stores the caller's pointer inside the table).
pub struct Keep(pub Vec<Box<[u8]>>);

impl Keep {
    pub fn new() -> Keep {
        Keep(vec![])
    }
    pub fn add(&mut self, bytes: &[u8]) -> *mut c_void {
        let b: Box<[u8]> = bytes.to_vec().into_boxed_slice();
        let p = b.as_ptr() as *mut c_void;
        self.0.push(b);
        p
    }
}
