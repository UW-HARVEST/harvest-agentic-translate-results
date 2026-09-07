//! Differential-test harness.
//!
//! Loads BOTH shared objects (the C reference and the Rust translation) with
//! `libloading` and exposes their exported symbols as raw `extern "C"` function
//! pointers.  Nothing in this crate is ever called directly — every call goes
//! through `dlsym`, exactly like an external consumer, so the `#[no_mangle]`
//! wrappers are part of what is under test.

#![allow(dead_code)]

use std::ffi::{c_char, c_int, c_void, CStr};
use std::path::PathBuf;
use std::sync::{Mutex, MutexGuard, OnceLock};

// ---------------------------------------------------------------------------
// Layout mirrors of the C structs (verified identical in both libraries)
// ---------------------------------------------------------------------------

#[repr(C)]
#[derive(Clone, Copy)]
pub struct ArrHeader {
    pub length: usize,
    pub capacity: usize,
    pub hash_table: *mut c_void,
    pub temp: isize,
}

pub const HEADER_SIZE: usize = std::mem::size_of::<ArrHeader>();

#[repr(C)]
#[derive(Clone, Copy)]
pub struct StringArena {
    pub storage: *mut c_void,
    pub remaining: usize,
    pub block: u8,
    pub mode: u8,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct HashBucket {
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
    pub string: StringArena,
    pub storage: *mut HashBucket,
}

// ---------------------------------------------------------------------------
// Exported-symbol signatures
// ---------------------------------------------------------------------------

pub type FnArrGrowF = unsafe extern "C" fn(*mut c_void, usize, usize, usize) -> *mut c_void;
pub type FnArrFreeF = unsafe extern "C" fn(*mut c_void);
pub type FnRandSeed = unsafe extern "C" fn(usize);
pub type FnHashBytes = unsafe extern "C" fn(*mut c_void, usize, usize) -> usize;
pub type FnHashString = unsafe extern "C" fn(*mut c_char, usize) -> usize;
pub type FnHmFreeFunc = unsafe extern "C" fn(*mut c_void, usize);
pub type FnHmGetKeyTs =
    unsafe extern "C" fn(*mut c_void, usize, *mut c_void, usize, *mut isize, c_int) -> *mut c_void;
pub type FnHmGetKey =
    unsafe extern "C" fn(*mut c_void, usize, *mut c_void, usize, c_int) -> *mut c_void;
pub type FnHmPutDefault = unsafe extern "C" fn(*mut c_void, usize) -> *mut c_void;
pub type FnHmPutKey =
    unsafe extern "C" fn(*mut c_void, usize, *mut c_void, usize, c_int) -> *mut c_void;
pub type FnShmodeFunc = unsafe extern "C" fn(usize, c_int) -> *mut c_void;
pub type FnHmDelKey =
    unsafe extern "C" fn(*mut c_void, usize, *mut c_void, usize, usize, c_int) -> *mut c_void;
pub type FnStrAlloc = unsafe extern "C" fn(*mut c_void, *mut c_char) -> *mut c_char;
pub type FnStrReset = unsafe extern "C" fn(*mut c_void);
pub type FnStrKey = unsafe extern "C" fn(c_int) -> *mut c_char;
pub type FnArrIns = unsafe extern "C" fn(c_int);

/// All 16 exported symbols of one shared object.
pub struct Lib {
    pub name: &'static str,
    pub arrgrowf: FnArrGrowF,
    pub arrfreef: FnArrFreeF,
    pub rand_seed: FnRandSeed,
    pub hash_bytes: FnHashBytes,
    pub hash_string: FnHashString,
    pub hmfree_func: FnHmFreeFunc,
    pub hmget_key_ts: FnHmGetKeyTs,
    pub hmget_key: FnHmGetKey,
    pub hmput_default: FnHmPutDefault,
    pub hmput_key: FnHmPutKey,
    pub shmode_func: FnShmodeFunc,
    pub hmdel_key: FnHmDelKey,
    pub stralloc: FnStrAlloc,
    pub strreset: FnStrReset,
    pub strkey: FnStrKey,
    pub arr_ins: FnArrIns,
}

unsafe fn sym<T: Copy>(lib: &libloading::Library, name: &[u8]) -> T {
    let s: libloading::Symbol<T> = lib
        .get(name)
        .unwrap_or_else(|e| panic!("dlsym {:?} failed: {e}", String::from_utf8_lossy(name)));
    *s
}

fn crate_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_so_path() -> PathBuf {
    let build = crate_root().join("../c_src/build");
    let mut found: Option<PathBuf> = None;
    for e in std::fs::read_dir(&build)
        .unwrap_or_else(|e| panic!("{} not readable ({e}); build the C library first", build.display()))
    {
        let p = e.unwrap().path();
        let n = p.file_name().unwrap().to_string_lossy().to_string();
        if n.starts_with("lib") && n.ends_with(".so") {
            found = Some(p);
        }
    }
    found.expect("no lib*.so in c_src/build")
}

fn rust_so_path() -> PathBuf {
    let p = crate_root().join("target/release/libarr_ins_lib.so");
    assert!(
        p.exists(),
        "{} missing; run `cargo build --release` first",
        p.display()
    );
    p
}

unsafe fn load(path: &PathBuf, name: &'static str) -> Lib {
    // RTLD_NOW | RTLD_LOCAL: RTLD_LOCAL is essential, otherwise the two
    // libraries would interpose each other's identically named symbols.
    let raw = libloading::os::unix::Library::open(
        Some(path),
        libloading::os::unix::RTLD_NOW | libloading::os::unix::RTLD_LOCAL,
    )
    .unwrap_or_else(|e| panic!("dlopen {} failed: {e}", path.display()));
    let lib: &'static libloading::Library = Box::leak(Box::new(libloading::Library::from(raw)));
    Lib {
        name,
        arrgrowf: sym(lib, b"stbds_arrgrowf\0"),
        arrfreef: sym(lib, b"stbds_arrfreef\0"),
        rand_seed: sym(lib, b"stbds_rand_seed\0"),
        hash_bytes: sym(lib, b"stbds_hash_bytes\0"),
        hash_string: sym(lib, b"stbds_hash_string\0"),
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
        arr_ins: sym(lib, b"arr_ins\0"),
    }
}

pub struct Pair {
    pub c: Lib,
    pub rs: Lib,
}

static PAIR: OnceLock<Pair> = OnceLock::new();
static LOCK: Mutex<()> = Mutex::new(());

/// Both libraries keep mutable process-global state (`stbds_hash_seed`, the
/// `strkey` buffer), so tests must not run concurrently against them.
pub fn libs() -> (&'static Pair, MutexGuard<'static, ()>) {
    let g = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let p = PAIR.get_or_init(|| unsafe {
        Pair {
            c: load(&c_so_path(), "C"),
            rs: load(&rust_so_path(), "RUST"),
        }
    });
    (p, g)
}

/// Put both libraries' global hash seed into a known state.
pub fn reset_seed(p: &Pair, seed: usize) {
    unsafe {
        (p.c.rand_seed)(seed);
        (p.rs.rand_seed)(seed);
    }
}

pub const DEFAULT_SEED: usize = 0x31415926;

// ---------------------------------------------------------------------------
// Deterministic RNG (splitmix64) — fixed seed for reproducibility
// ---------------------------------------------------------------------------

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Rng(seed)
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
    pub fn next_i32(&mut self) -> i32 {
        self.next_u32() as i32
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
    /// Bytes with the high bit always set (maximises the C int-shift
    /// sign-extension quirks in `stbds_hash_bytes`).
    pub fn high_bytes(&mut self, n: usize) -> Vec<u8> {
        (0..n)
            .map(|_| ((self.next_u64() >> 24) as u8) | 0x80)
            .collect()
    }
    /// Printable, NUL-free string bytes (not NUL-terminated).
    pub fn ascii(&mut self, n: usize) -> Vec<u8> {
        (0..n)
            .map(|_| b'!' + ((self.next_u64() >> 24) as u8 % 93))
            .collect()
    }
    /// Non-NUL bytes including values >= 0x80.
    pub fn nonnul(&mut self, n: usize) -> Vec<u8> {
        (0..n)
            .map(|_| {
                let b = (self.next_u64() >> 24) as u8;
                if b == 0 {
                    1
                } else {
                    b
                }
            })
            .collect()
    }
}

// ---------------------------------------------------------------------------
// State snapshots (pointer-free, therefore directly comparable)
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
    pub arena_remaining: usize,
    pub arena_block: u8,
    pub arena_mode: u8,
    pub arena_block_count: usize,
    pub temp_key: Option<Vec<u8>>,
    /// storage pointer is 64-byte aligned and lands inside the allocation
    pub storage_aligned: bool,
    pub buckets: Vec<([usize; 8], [isize; 8])>,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Snap {
    pub null: bool,
    pub length: usize,
    pub capacity: usize,
    pub temp: isize,
    pub has_table: bool,
    pub table: Option<TableSnap>,
    /// element bytes for indices `1..length` (index 0 is the default slot)
    pub elems: Vec<Vec<u8>>,
    /// dereferenced key strings when the element's first 8 bytes are a `char*`
    pub keys: Option<Vec<Option<Vec<u8>>>>,
    /// element 0 (the "default value" slot)
    pub elem0: Vec<u8>,
}

/// How to interpret an element when snapshotting.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ElemFmt {
    /// Whole element is plain data: compare raw bytes.
    Raw,
    /// First 8 bytes are a `char *` key: compare the pointee string plus the
    /// remaining bytes verbatim.
    PtrKey,
}

unsafe fn cstr_bytes(p: *const c_char) -> Option<Vec<u8>> {
    if p.is_null() {
        None
    } else {
        Some(CStr::from_ptr(p).to_bytes().to_vec())
    }
}

unsafe fn arena_block_count(a: &StringArena) -> usize {
    // block list nodes: { next; char storage[] }
    let mut n = 0usize;
    let mut x = a.storage as *const *const c_void;
    while !x.is_null() {
        n += 1;
        if n > 100_000 {
            break;
        }
        x = *x as *const *const c_void;
    }
    n
}

/// Snapshot a `stbds` *hash-map* pointer (i.e. the value the `hm*` functions
/// return, which points one element past the raw array base).
pub unsafe fn snap_hash(h: *mut c_void, elemsize: usize, fmt: ElemFmt) -> Snap {
    if h.is_null() {
        return Snap {
            null: true,
            length: 0,
            capacity: 0,
            temp: 0,
            has_table: false,
            table: None,
            elems: vec![],
            keys: None,
            elem0: vec![],
        };
    }
    let raw = (h as *mut u8).sub(elemsize);
    snap_arr(raw as *mut c_void, elemsize, fmt, true)
}

/// Snapshot a raw `stbds` *array* pointer (what `stbds_arrgrowf` returns).
pub unsafe fn snap_arr(a: *mut c_void, elemsize: usize, fmt: ElemFmt, is_map: bool) -> Snap {
    if a.is_null() {
        return Snap {
            null: true,
            length: 0,
            capacity: 0,
            temp: 0,
            has_table: false,
            table: None,
            elems: vec![],
            keys: None,
            elem0: vec![],
        };
    }
    let base = a as *mut u8;
    let hdr = (base as *mut ArrHeader).sub(1);
    let length = (*hdr).length;
    let capacity = (*hdr).capacity;
    let temp = (*hdr).temp;
    let table = (*hdr).hash_table as *mut HashIndex;

    let mut elems = Vec::new();
    let mut keys: Option<Vec<Option<Vec<u8>>>> = if fmt == ElemFmt::PtrKey {
        Some(Vec::new())
    } else {
        None
    };
    let start = if is_map { 1 } else { 0 };
    let mut elem0 = Vec::new();
    if is_map && length >= 1 && elemsize > 0 {
        elem0 = std::slice::from_raw_parts(base, elemsize).to_vec();
        if fmt == ElemFmt::PtrKey {
            // element 0's key slot is the zeroed default; don't deref it
            for b in elem0[..8.min(elemsize)].iter_mut() {
                *b = 0;
            }
        }
    }
    if elemsize > 0 {
        for i in start..length {
            let ep = base.add(elemsize * i);
            match fmt {
                ElemFmt::Raw => elems.push(std::slice::from_raw_parts(ep, elemsize).to_vec()),
                ElemFmt::PtrKey => {
                    let kp = *(ep as *const *const c_char);
                    keys.as_mut().unwrap().push(cstr_bytes(kp));
                    elems.push(std::slice::from_raw_parts(ep.add(8), elemsize - 8).to_vec());
                }
            }
        }
    }

    let tsnap = if table.is_null() {
        None
    } else {
        let t = &*table;
        let nbuckets = t.slot_count >> 3;
        let mut buckets = Vec::with_capacity(nbuckets);
        for i in 0..nbuckets {
            let b = &*t.storage.add(i);
            buckets.push((b.hash, b.index));
        }
        Some(TableSnap {
            slot_count: t.slot_count,
            used_count: t.used_count,
            used_count_threshold: t.used_count_threshold,
            used_count_shrink_threshold: t.used_count_shrink_threshold,
            tombstone_count: t.tombstone_count,
            tombstone_count_threshold: t.tombstone_count_threshold,
            seed: t.seed,
            slot_count_log2: t.slot_count_log2,
            arena_remaining: t.string.remaining,
            arena_block: t.string.block,
            arena_mode: t.string.mode,
            arena_block_count: arena_block_count(&t.string),
            // NOTE: `stbds_make_hash_index` never initialises `temp_key`, so
            // it is genuine uninitialised heap garbage until a STRING-mode
            // `hmput_key` writes it.  Never dereference it here; string-mode
            // tests use `temp_key_of()` explicitly at points where the C code
            // has definitely assigned it.
            temp_key: None,
            storage_aligned: (t.storage as usize) % 64 == 0
                && (t.storage as usize) >= (table as usize + std::mem::size_of::<HashIndex>())
                && (t.storage as usize) < (table as usize + std::mem::size_of::<HashIndex>() + 64),
            buckets,
        })
    };

    Snap {
        null: false,
        length,
        capacity,
        temp,
        has_table: !table.is_null(),
        table: tsnap,
        elems,
        keys,
        elem0,
    }
}

/// `stbds_temp_key(t)` == `*(char **) stbds_header(t)->hash_table`, i.e. the
/// `temp_key` field of the hash index.  Only valid after a STRING-mode
/// `hmput_key`, which is the only writer.
pub unsafe fn temp_key_of(h: *mut c_void, elemsize: usize) -> Option<Vec<u8>> {
    if h.is_null() {
        return None;
    }
    let raw = (h as *mut u8).sub(elemsize);
    let hdr = (raw as *mut ArrHeader).sub(1);
    let t = (*hdr).hash_table as *mut HashIndex;
    if t.is_null() {
        return None;
    }
    cstr_bytes((*t).temp_key)
}

/// Raw `temp_key` pointer value (for identity checks in `SH_DEFAULT` mode).
pub unsafe fn temp_key_ptr(h: *mut c_void, elemsize: usize) -> *mut c_char {
    if h.is_null() {
        return std::ptr::null_mut();
    }
    let raw = (h as *mut u8).sub(elemsize);
    let hdr = (raw as *mut ArrHeader).sub(1);
    let t = (*hdr).hash_table as *mut HashIndex;
    if t.is_null() {
        return std::ptr::null_mut();
    }
    (*t).temp_key
}

/// Overwrite `temp_key` so that "did this call write temp_key?" becomes
/// observable.  `stbds_make_hash_index` leaves the field uninitialised and the
/// wrapped half of `hmput_key`'s probe loop deliberately does NOT refresh it,
/// so poisoning is the only way to compare that asymmetry reliably.
pub unsafe fn set_temp_key_ptr(h: *mut c_void, elemsize: usize, v: *mut c_char) -> bool {
    if h.is_null() {
        return false;
    }
    let raw = (h as *mut u8).sub(elemsize);
    let hdr = (raw as *mut ArrHeader).sub(1);
    let t = (*hdr).hash_table as *mut HashIndex;
    if t.is_null() {
        return false;
    }
    (*t).temp_key = v;
    true
}

/// A process-wide poison string, shared by both libraries so that pointer
/// identity comparisons are meaningful across them.
pub fn poison_ptr() -> *mut c_char {
    static POISON: OnceLock<usize> = OnceLock::new();
    let p = *POISON.get_or_init(|| {
        let b: Box<[u8]> = b"<<temp_key-not-written>>\0".to_vec().into_boxed_slice();
        Box::leak(b).as_ptr() as usize
    });
    p as *mut c_char
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct ArenaSnap {    pub remaining: usize,
    pub block: u8,
    pub mode: u8,
    pub block_count: usize,
}

pub unsafe fn snap_arena(a: *const StringArena) -> ArenaSnap {
    let a = &*a;
    ArenaSnap {
        remaining: a.remaining,
        block: a.block,
        mode: a.mode,
        block_count: arena_block_count(a),
    }
}

/// A NUL-terminated, heap-owned key buffer that stays alive while the map
/// references it (needed for `STBDS_SH_DEFAULT` maps, which store the caller's
/// pointer verbatim).
pub struct CKey(pub Box<[u8]>);

impl CKey {
    pub fn new(bytes: &[u8]) -> Self {
        let mut v = bytes.to_vec();
        v.push(0);
        CKey(v.into_boxed_slice())
    }
    pub fn ptr(&self) -> *mut c_char {
        self.0.as_ptr() as *mut c_char
    }
}

/// Assert two snapshots are identical, with a descriptive context on failure.
#[track_caller]
pub fn eq_snap(ctx: &str, c: &Snap, r: &Snap) {
    if c != r {
        // narrow the report down to the first differing field
        let mut diffs = Vec::new();
        macro_rules! f {
            ($n:ident) => {
                if c.$n != r.$n {
                    diffs.push(format!("{}: C={:?} RUST={:?}", stringify!($n), c.$n, r.$n));
                }
            };
        }
        f!(null);
        f!(length);
        f!(capacity);
        f!(temp);
        f!(has_table);
        f!(elem0);
        f!(keys);
        if c.elems != r.elems {
            for (i, (a, b)) in c.elems.iter().zip(r.elems.iter()).enumerate() {
                if a != b {
                    diffs.push(format!("elems[{i}]: C={a:02x?} RUST={b:02x?}"));
                }
            }
            if c.elems.len() != r.elems.len() {
                diffs.push(format!(
                    "elems.len: C={} RUST={}",
                    c.elems.len(),
                    r.elems.len()
                ));
            }
        }
        match (&c.table, &r.table) {
            (Some(a), Some(b)) => {
                macro_rules! t {
                    ($n:ident) => {
                        if a.$n != b.$n {
                            diffs.push(format!(
                                "table.{}: C={:?} RUST={:?}",
                                stringify!($n),
                                a.$n,
                                b.$n
                            ));
                        }
                    };
                }
                t!(slot_count);
                t!(used_count);
                t!(used_count_threshold);
                t!(used_count_shrink_threshold);
                t!(tombstone_count);
                t!(tombstone_count_threshold);
                t!(seed);
                t!(slot_count_log2);
                t!(arena_remaining);
                t!(arena_block);
                t!(arena_mode);
                t!(arena_block_count);
                t!(temp_key);
                t!(storage_aligned);
                if a.buckets != b.buckets {
                    for (i, (x, y)) in a.buckets.iter().zip(b.buckets.iter()).enumerate() {
                        if x != y {
                            diffs.push(format!("table.buckets[{i}]: C={x:?} RUST={y:?}"));
                        }
                    }
                }
            }
            (x, y) => {
                if x.is_some() != y.is_some() {
                    diffs.push(format!(
                        "table presence: C={} RUST={}",
                        x.is_some(),
                        y.is_some()
                    ));
                }
            }
        }
        panic!("[{ctx}] C/Rust state divergence:\n  {}", diffs.join("\n  "));
    }
}

pub const HM_BINARY: c_int = 0;
pub const HM_STRING: c_int = 1;
pub const SH_NONE: c_int = 0;
pub const SH_DEFAULT: c_int = 1;
pub const SH_STRDUP: c_int = 2;
pub const SH_ARENA: c_int = 3;

// ---------------------------------------------------------------------------
// Differential hash-map driver
//
// Drives BOTH libraries through the same sequence of low-level calls exactly
// the way the `stbds_hmput` / `stbds_hmgeti` / `stbds_hmdel` macros in the C
// header do (grow, read `header->temp`, write the payload at `h[temp]`), and
// compares the complete observable state after every single call.
// ---------------------------------------------------------------------------

#[derive(Clone, Copy)]
pub struct MapCfg {
    pub elemsize: usize,
    pub keysize: usize,
    pub keyoffset: usize,
    /// `mode` passed to `hmput_key` / `hmget_key*`
    pub put_mode: c_int,
    /// `mode` passed to `hmdel_key`
    pub del_mode: c_int,
    pub fmt: ElemFmt,
    /// byte range of the "value" part of the element written by the caller
    pub value_off: usize,
    pub value_len: usize,
}

pub struct Maps<'a> {
    p: &'a Pair,
    pub ch: *mut c_void,
    pub rh: *mut c_void,
    pub cfg: MapCfg,
    /// keys kept alive because `STBDS_SH_DEFAULT` maps store the caller pointer
    keep: Vec<CKey>,
    pub ops: usize,
}

impl<'a> Maps<'a> {
    pub fn empty(p: &'a Pair, cfg: MapCfg) -> Self {
        Maps {
            p,
            ch: std::ptr::null_mut(),
            rh: std::ptr::null_mut(),
            cfg,
            keep: Vec::new(),
            ops: 0,
        }
    }

    /// Create both maps via `stbds_shmode_func` (the explicit `sh_new_*` path).
    pub fn shmode(p: &'a Pair, cfg: MapCfg, shmode: c_int) -> Self {
        let mut m = Maps::empty(p, cfg);
        unsafe {
            m.ch = (p.c.shmode_func)(cfg.elemsize, shmode);
            m.rh = (p.rs.shmode_func)(cfg.elemsize, shmode);
        }
        m.compare("shmode_func");
        m
    }

    pub fn snap_c(&self) -> Snap {
        unsafe { snap_hash(self.ch, self.cfg.elemsize, self.cfg.fmt) }
    }
    pub fn snap_r(&self) -> Snap {
        unsafe { snap_hash(self.rh, self.cfg.elemsize, self.cfg.fmt) }
    }

    #[track_caller]
    pub fn compare(&self, what: &str) {
        assert_eq!(
            self.ch.is_null(),
            self.rh.is_null(),
            "[{what} op#{}] returned-pointer nullness differs",
            self.ops
        );
        if self.ch.is_null() {
            return;
        }
        eq_snap(
            &format!("{what} op#{}", self.ops),
            &self.snap_c(),
            &self.snap_r(),
        );
    }

    fn temp_c(&self) -> isize {
        unsafe { (*((self.ch as *mut u8).sub(self.cfg.elemsize) as *mut ArrHeader).sub(1)).temp }
    }
    fn temp_r(&self) -> isize {
        unsafe { (*((self.rh as *mut u8).sub(self.cfg.elemsize) as *mut ArrHeader).sub(1)).temp }
    }

    /// `stbds_hmput(t, k, v)` for a BINARY map: grow, then write key+value at
    /// `t[temp]`.
    #[track_caller]
    pub fn put_binary(&mut self, key: &[u8], value: &[u8]) -> isize {
        self.ops += 1;
        let cfg = self.cfg;
        assert_eq!(key.len(), cfg.keysize);
        assert_eq!(value.len(), cfg.value_len);
        let mut k = key.to_vec();
        if k.is_empty() {
            k.push(0);
        }
        unsafe {
            self.ch = (self.p.c.hmput_key)(
                self.ch,
                cfg.elemsize,
                k.as_mut_ptr() as *mut c_void,
                cfg.keysize,
                cfg.put_mode,
            );
            self.rh = (self.p.rs.hmput_key)(
                self.rh,
                cfg.elemsize,
                k.as_mut_ptr() as *mut c_void,
                cfg.keysize,
                cfg.put_mode,
            );
            let ct = self.temp_c();
            let rt = self.temp_r();
            assert_eq!(
                ct, rt,
                "[put op#{}] header->temp differs: C={ct} RUST={rt}",
                self.ops
            );
            // emulate `(t)[temp].key = k; (t)[temp].value = v;`
            // (value first so that overlapping test layouts stay deterministic)
            for lib_h in [self.ch, self.rh] {
                let e = (lib_h as *mut u8).add(cfg.elemsize * ct as usize);
                if cfg.value_len > 0 {
                    std::ptr::copy_nonoverlapping(
                        value.as_ptr(),
                        e.add(cfg.value_off),
                        cfg.value_len,
                    );
                }
                if cfg.keysize > 0 {
                    std::ptr::copy_nonoverlapping(key.as_ptr(), e.add(cfg.keyoffset), cfg.keysize);
                }
            }
            self.compare("put_binary");
            ct
        }
    }

    /// `stbds_shput(t, k, v)` for a STRING map: grow (the library stores the
    /// key itself according to `string.mode`), then write only the value.
    #[track_caller]
    pub fn put_string(&mut self, key: &[u8], value: &[u8]) -> isize {
        self.ops += 1;
        let cfg = self.cfg;
        assert_eq!(value.len(), cfg.value_len);
        let ck = CKey::new(key);
        let kp = ck.ptr();
        self.keep.push(ck);
        unsafe {
            // `temp_key` is genuinely indeterminate unless the C actually
            // assigns it:
            //   * `make_hash_index` never initialises the field, so any table
            //     growth/shrink/rebuild resets it to heap garbage;
            //   * the wrapped half of `hmput_key`'s probe loop deliberately
            //     does NOT refresh it when it finds an existing key;
            //   * the `default:` storage mode does not write it either.
            // Poisoning makes "did this call write temp_key?" observable
            // whenever the table object survives the call.
            let poison = poison_ptr();
            let had_table = set_temp_key_ptr(self.ch, cfg.elemsize, poison);
            let had_table_r = set_temp_key_ptr(self.rh, cfg.elemsize, poison);
            assert_eq!(
                had_table, had_table_r,
                "[put_string op#{}] table presence differs",
                self.ops
            );
            let before = if self.ch.is_null() {
                None
            } else {
                let s = self.snap_c();
                Some((s.length, s.table.map(|t| t.slot_count)))
            };

            self.ch = (self.p.c.hmput_key)(
                self.ch,
                cfg.elemsize,
                kp as *mut c_void,
                cfg.keysize,
                cfg.put_mode,
            );
            self.rh = (self.p.rs.hmput_key)(
                self.rh,
                cfg.elemsize,
                kp as *mut c_void,
                cfg.keysize,
                cfg.put_mode,
            );
            let ct = self.temp_c();
            let rt = self.temp_r();
            assert_eq!(
                ct, rt,
                "[put_string op#{}] header->temp differs: C={ct} RUST={rt}",
                self.ops
            );

            let after = self.snap_c();
            let new_entry = before.map(|(l, _)| after.length > l).unwrap_or(true);
            let table_kept = match (before.and_then(|(_, s)| s), after.table.as_ref()) {
                (Some(a), Some(b)) => a == b.slot_count,
                _ => false,
            };
            let stores_ptr = matches!(
                after.table.as_ref().map(|t| t.arena_mode),
                Some(1) | Some(2) | Some(3)
            );
            if new_entry && stores_ptr {
                // the insert path always assigns temp_key from the stored key
                let c = temp_key_of(self.ch, cfg.elemsize);
                let r = temp_key_of(self.rh, cfg.elemsize);
                assert_eq!(
                    c, r,
                    "[put_string op#{}] temp_key contents differ after insert",
                    self.ops
                );
                assert_eq!(
                    c.as_deref(),
                    Some(key),
                    "[put_string op#{}] temp_key must point at the stored key",
                    self.ops
                );
            } else if !new_entry && had_table && table_kept {
                // existing key found: the poison survives iff the C's wrapped
                // probe half took the branch that skips the temp_key refresh
                let cw = temp_key_ptr(self.ch, cfg.elemsize) != poison;
                let rw = temp_key_ptr(self.rh, cfg.elemsize) != poison;
                assert_eq!(
                    cw, rw,
                    "[put_string op#{}] temp_key refresh differs (C wrote={cw} RUST wrote={rw})",
                    self.ops
                );
                if cw {
                    assert_eq!(
                        temp_key_of(self.ch, cfg.elemsize),
                        temp_key_of(self.rh, cfg.elemsize),
                        "[put_string op#{}] refreshed temp_key contents differ",
                        self.ops
                    );
                }
            }

            for lib_h in [self.ch, self.rh] {
                let e = (lib_h as *mut u8).add(cfg.elemsize * ct as usize);
                if cfg.value_len > 0 {
                    std::ptr::copy_nonoverlapping(
                        value.as_ptr(),
                        e.add(cfg.value_off),
                        cfg.value_len,
                    );
                }
            }
            self.compare("put_string");
            ct
        }
    }

    /// `stbds_hmgeti(t, k)` — returns `header->temp` (`-1` when absent).
    #[track_caller]
    pub fn get(&mut self, key: &[u8]) -> isize {
        self.ops += 1;
        let cfg = self.cfg;
        let mut k = key.to_vec();
        k.push(0);
        unsafe {
            self.ch = (self.p.c.hmget_key)(
                self.ch,
                cfg.elemsize,
                k.as_mut_ptr() as *mut c_void,
                cfg.keysize,
                cfg.put_mode,
            );
            self.rh = (self.p.rs.hmget_key)(
                self.rh,
                cfg.elemsize,
                k.as_mut_ptr() as *mut c_void,
                cfg.keysize,
                cfg.put_mode,
            );
            let ct = self.temp_c();
            let rt = self.temp_r();
            assert_eq!(
                ct, rt,
                "[get op#{}] index differs: C={ct} RUST={rt} key={key:02x?}",
                self.ops
            );
            self.compare("get");
            ct
        }
    }

    /// `stbds_hmget_key_ts` — the thread-safe variant with an explicit out-param.
    #[track_caller]
    pub fn get_ts(&mut self, key: &[u8]) -> isize {
        self.ops += 1;
        let cfg = self.cfg;
        let mut k = key.to_vec();
        k.push(0);
        unsafe {
            let mut ct: isize = 0x7f7f;
            let mut rt: isize = 0x7f7f;
            self.ch = (self.p.c.hmget_key_ts)(
                self.ch,
                cfg.elemsize,
                k.as_mut_ptr() as *mut c_void,
                cfg.keysize,
                &mut ct,
                cfg.put_mode,
            );
            self.rh = (self.p.rs.hmget_key_ts)(
                self.rh,
                cfg.elemsize,
                k.as_mut_ptr() as *mut c_void,
                cfg.keysize,
                &mut rt,
                cfg.put_mode,
            );
            assert_eq!(
                ct, rt,
                "[get_ts op#{}] *temp differs: C={ct} RUST={rt}",
                self.ops
            );
            self.compare("get_ts");
            ct
        }
    }

    /// `stbds_hmdel(t, k)` — returns `t ? header->temp : 0` (1 = deleted).
    #[track_caller]
    pub fn del(&mut self, key: &[u8]) -> isize {
        self.ops += 1;
        let cfg = self.cfg;
        let mut k = key.to_vec();
        k.push(0);
        unsafe {
            self.ch = (self.p.c.hmdel_key)(
                self.ch,
                cfg.elemsize,
                k.as_mut_ptr() as *mut c_void,
                cfg.keysize,
                cfg.keyoffset,
                cfg.del_mode,
            );
            self.rh = (self.p.rs.hmdel_key)(
                self.rh,
                cfg.elemsize,
                k.as_mut_ptr() as *mut c_void,
                cfg.keysize,
                cfg.keyoffset,
                cfg.del_mode,
            );
            assert_eq!(
                self.ch.is_null(),
                self.rh.is_null(),
                "[del op#{}] nullness differs",
                self.ops
            );
            if self.ch.is_null() {
                return 0;
            }
            let ct = self.temp_c();
            let rt = self.temp_r();
            assert_eq!(
                ct, rt,
                "[del op#{}] result differs: C={ct} RUST={rt} key={key:02x?}",
                self.ops
            );
            self.compare("del");
            ct
        }
    }

    pub fn free(&mut self) {
        unsafe {
            if !self.ch.is_null() {
                (self.p.c.hmfree_func)(
                    (self.ch as *mut u8).sub(self.cfg.elemsize) as *mut c_void,
                    self.cfg.elemsize,
                );
            }
            if !self.rh.is_null() {
                (self.p.rs.hmfree_func)(
                    (self.rh as *mut u8).sub(self.cfg.elemsize) as *mut c_void,
                    self.cfg.elemsize,
                );
            }
        }
        self.ch = std::ptr::null_mut();
        self.rh = std::ptr::null_mut();
        self.keep.clear();
    }
}

/// A 16-byte `{int key,b,c,d}` BINARY map (`stbds_struct` from the C source).
pub fn cfg_struct() -> MapCfg {
    MapCfg {
        elemsize: 16,
        keysize: 4,
        keyoffset: 0,
        put_mode: HM_BINARY,
        del_mode: HM_BINARY,
        fmt: ElemFmt::Raw,
        value_off: 4,
        value_len: 12,
    }
}

/// A 20-byte `{int key[2],b,c,d}` BINARY map (`stbds_struct2`).
pub fn cfg_struct2() -> MapCfg {
    MapCfg {
        elemsize: 20,
        keysize: 8,
        keyoffset: 0,
        put_mode: HM_BINARY,
        del_mode: HM_BINARY,
        fmt: ElemFmt::Raw,
        value_off: 8,
        value_len: 12,
    }
}

/// A 16-byte `{char *key; int value; pad}` STRING map.  `value_len` covers the
/// whole 8 bytes after the key pointer so no element byte is left undefined.
pub fn cfg_string() -> MapCfg {
    MapCfg {
        elemsize: 16,
        keysize: 8,
        keyoffset: 0,
        put_mode: HM_STRING,
        del_mode: HM_STRING,
        fmt: ElemFmt::PtrKey,
        value_off: 8,
        value_len: 8,
    }
}
