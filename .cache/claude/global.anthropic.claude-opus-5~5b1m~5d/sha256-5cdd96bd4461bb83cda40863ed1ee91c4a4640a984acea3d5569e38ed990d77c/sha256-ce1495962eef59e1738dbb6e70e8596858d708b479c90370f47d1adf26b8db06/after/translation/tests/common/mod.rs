//! Shared differential-test harness.
//!
//! Loads BOTH the C `.so` and the Rust `.so` through `libloading` and calls
//! every entry point purely through its exported C symbol — the Rust functions
//! are never called directly, so the `#[no_mangle]`/`extern "C"` wrappers are
//! part of what gets tested.
#![allow(dead_code)]

use std::ffi::c_void;
use std::os::raw::{c_char, c_int};
use std::path::{Path, PathBuf};

// ---------------------------------------------------------------------------
// C ABI signatures (from c_src/src/lib.c)
// ---------------------------------------------------------------------------

pub type FnArrGrowF = unsafe extern "C" fn(*mut c_void, usize, usize, usize) -> *mut c_void;
pub type FnArrFreeF = unsafe extern "C" fn(*mut c_void);
pub type FnRandSeed = unsafe extern "C" fn(usize);
pub type FnHashString = unsafe extern "C" fn(*mut c_char, usize) -> usize;
pub type FnHashBytes = unsafe extern "C" fn(*mut c_void, usize, usize) -> usize;
pub type FnHmFreeFunc = unsafe extern "C" fn(*mut c_void, usize);
pub type FnHmGetKeyTs =
    unsafe extern "C" fn(*mut c_void, usize, *mut c_void, usize, *mut isize, c_int) -> *mut c_void;
pub type FnHmGetKey =
    unsafe extern "C" fn(*mut c_void, usize, *mut c_void, usize, c_int) -> *mut c_void;
pub type FnHmPutDefault = unsafe extern "C" fn(*mut c_void, usize) -> *mut c_void;
pub type FnHmPutKey =
    unsafe extern "C" fn(*mut c_void, usize, *mut c_void, usize, c_int) -> *mut c_void;
pub type FnShModeFunc = unsafe extern "C" fn(usize, c_int) -> *mut c_void;
pub type FnHmDelKey =
    unsafe extern "C" fn(*mut c_void, usize, *mut c_void, usize, usize, c_int) -> *mut c_void;
pub type FnStrAlloc = unsafe extern "C" fn(*mut CArena, *mut c_char) -> *mut c_char;
pub type FnStrReset = unsafe extern "C" fn(*mut CArena);
pub type FnStrKey = unsafe extern "C" fn(c_int) -> *mut c_char;
pub type FnArrIns = unsafe extern "C" fn(c_int);

// ---------------------------------------------------------------------------
// Mirror of the C data layout (used only to *read* results back)
// ---------------------------------------------------------------------------

pub const HEADER_SIZE: usize = 32;
pub const BUCKET_LENGTH: usize = 8;
pub const BUCKET_SHIFT: usize = 3;
pub const CACHE_LINE_SIZE: usize = 64;

pub const HM_BINARY: c_int = 0;
pub const HM_STRING: c_int = 1;

pub const SH_NONE: c_int = 0;
pub const SH_DEFAULT: c_int = 1;
pub const SH_STRDUP: c_int = 2;
pub const SH_ARENA: c_int = 3;

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CHeader {
    pub length: usize,
    pub capacity: usize,
    pub hash_table: *mut c_void,
    pub temp: isize,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct CArena {
    pub storage: *mut c_void,
    pub remaining: usize,
    pub block: u8,
    pub mode: u8,
}

impl CArena {
    pub fn zeroed() -> CArena {
        CArena {
            storage: std::ptr::null_mut(),
            remaining: 0,
            block: 0,
            mode: 0,
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct CHashIndex {
    pub temp_key: *mut c_char,
    pub slot_count: usize,
    pub used_count: usize,
    pub used_count_threshold: usize,
    pub used_count_shrink_threshold: usize,
    pub tombstone_count: usize,
    pub tombstone_count_threshold: usize,
    pub seed: usize,
    pub slot_count_log2: usize,
    pub string: CArena,
    pub storage: *mut CBucket,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct CBucket {
    pub hash: [usize; BUCKET_LENGTH],
    pub index: [isize; BUCKET_LENGTH],
}

// ---------------------------------------------------------------------------
// Library loading
// ---------------------------------------------------------------------------

pub struct Lib {
    pub name: &'static str,
    _lib: libloading::Library,
    pub arrgrowf: FnArrGrowF,
    pub arrfreef: FnArrFreeF,
    pub rand_seed: FnRandSeed,
    pub hash_string: FnHashString,
    pub hash_bytes: FnHashBytes,
    pub hmfree_func: FnHmFreeFunc,
    pub hmget_key_ts: FnHmGetKeyTs,
    pub hmget_key: FnHmGetKey,
    pub hmput_default: FnHmPutDefault,
    pub hmput_key: FnHmPutKey,
    pub shmode_func: FnShModeFunc,
    pub hmdel_key: FnHmDelKey,
    pub stralloc: FnStrAlloc,
    pub strreset: FnStrReset,
    pub strkey: FnStrKey,
    pub arr_ins: FnArrIns,
}

macro_rules! sym {
    ($lib:expr, $name:literal, $ty:ty) => {{
        let s: libloading::Symbol<$ty> = unsafe { $lib.get(concat!($name, "\0").as_bytes()) }
            .unwrap_or_else(|e| panic!("symbol `{}` missing: {e}", $name));
        *s
    }};
}

impl Lib {
    pub fn open(name: &'static str, path: &Path) -> Lib {
        let lib = unsafe { libloading::Library::new(path) }
            .unwrap_or_else(|e| panic!("cannot load {}: {e}", path.display()));
        let l = Lib {
            name,
            arrgrowf: sym!(lib, "stbds_arrgrowf", FnArrGrowF),
            arrfreef: sym!(lib, "stbds_arrfreef", FnArrFreeF),
            rand_seed: sym!(lib, "stbds_rand_seed", FnRandSeed),
            hash_string: sym!(lib, "stbds_hash_string", FnHashString),
            hash_bytes: sym!(lib, "stbds_hash_bytes", FnHashBytes),
            hmfree_func: sym!(lib, "stbds_hmfree_func", FnHmFreeFunc),
            hmget_key_ts: sym!(lib, "stbds_hmget_key_ts", FnHmGetKeyTs),
            hmget_key: sym!(lib, "stbds_hmget_key", FnHmGetKey),
            hmput_default: sym!(lib, "stbds_hmput_default", FnHmPutDefault),
            hmput_key: sym!(lib, "stbds_hmput_key", FnHmPutKey),
            shmode_func: sym!(lib, "stbds_shmode_func", FnShModeFunc),
            hmdel_key: sym!(lib, "stbds_hmdel_key", FnHmDelKey),
            stralloc: sym!(lib, "stbds_stralloc", FnStrAlloc),
            strreset: sym!(lib, "stbds_strreset", FnStrReset),
            strkey: sym!(lib, "strkey", FnStrKey),
            arr_ins: sym!(lib, "arr_ins", FnArrIns),
            _lib: lib,
        };
        l
    }
}

fn crate_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

pub fn c_so_path() -> PathBuf {
    let build = crate_root().parent().unwrap().join("c_src/build");
    let mut found = None;
    if let Ok(rd) = std::fs::read_dir(&build) {
        for e in rd.flatten() {
            let p = e.path();
            if p.extension().map(|x| x == "so").unwrap_or(false) {
                found = Some(p);
                break;
            }
        }
    }
    found.unwrap_or_else(|| {
        panic!(
            "no .so in {} — build the C library first",
            build.display()
        )
    })
}

pub fn rust_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("RUST_SO") {
        return PathBuf::from(p);
    }
    let root = crate_root();
    for profile in ["release", "debug"] {
        let p = root.join("target").join(profile).join("libarr_ins_lib.so");
        if p.exists() {
            return p;
        }
    }
    panic!("libarr_ins_lib.so not found; run `cargo build --release`");
}

/// `dlopen` returns the *same* handle for the same path, so every test in a
/// binary shares the libraries' mutable globals (`stbds_hash_seed` and the
/// `static char buffer[256]` behind `strkey`). Holding this lock for the whole
/// lifetime of a `Pair` serialises the tests inside one binary, which is what
/// the C library's global state requires.
static LIB_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// The two libraries under test.
pub struct Pair {
    pub c: Lib,
    pub r: Lib,
    _guard: std::sync::MutexGuard<'static, ()>,
}

pub fn pair() -> Pair {
    let guard = LIB_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    Pair {
        c: Lib::open("C", &c_so_path()),
        r: Lib::open("Rust", &rust_so_path()),
        _guard: guard,
    }
}

/// Fresh, independent copies of both libraries. `dlopen` of the same path
/// returns the same handle, so global state (`stbds_hash_seed`) persists for the
/// whole process; every test therefore calls `rand_seed` explicitly to sync.
pub fn fresh_pair(seed: usize) -> Pair {
    let p = pair();
    unsafe {
        (p.c.rand_seed)(seed);
        (p.r.rand_seed)(seed);
    }
    p
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (SplitMix64) so every property test is reproducible
// ---------------------------------------------------------------------------

pub struct Rng(pub u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng(seed)
    }
    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E3779B97F4A7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D049BB133111EB);
        z ^ (z >> 31)
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
    pub fn byte(&mut self) -> u8 {
        (self.next_u64() >> 56) as u8
    }
    pub fn bytes(&mut self, n: usize) -> Vec<u8> {
        (0..n).map(|_| self.byte()).collect()
    }
    /// NUL-terminated random string of `lo..hi` bytes drawn from `alphabet`.
    pub fn cstring_range(&mut self, lo: usize, hi: usize, alphabet: &[u8]) -> Vec<u8> {
        let n = lo + self.below(hi.saturating_sub(lo).max(1));
        self.cstring(n, alphabet)
    }
    /// NUL-terminated random string of `n` bytes drawn from `alphabet`.
    pub fn cstring(&mut self, n: usize, alphabet: &[u8]) -> Vec<u8> {
        let mut v: Vec<u8> = (0..n)
            .map(|_| alphabet[self.below(alphabet.len())])
            .collect();
        v.push(0);
        v
    }
}

pub const ASCII: &[u8] = b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789_";
pub const HIGHBYTES: &[u8] = &[
    0x01, 0x02, 0x7f, 0x80, 0x81, 0xa0, 0xc3, 0xfe, 0xff, 0x41, 0x5a,
];

// ---------------------------------------------------------------------------
// State snapshotting: turns an opaque C data structure into comparable bytes
// ---------------------------------------------------------------------------

/// How the element's key field must be interpreted when snapshotting.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum KeyRepr {
    /// key bytes live inline in the element
    Inline,
    /// element offset 0 holds a `char *`; compare the pointed-to string
    CharPtr,
    /// pick per `table->string.mode`: `SH_STRDUP`/`SH_ARENA` store a
    /// library-private pointer (compare the target), everything else stores
    /// bytes that are bit-identical between the two libraries.
    Auto,
    /// Only the first `n` bytes of each live element are defined. Used when the
    /// raw `stbds_hmput_key` is called directly, without the macro's
    /// `t[temp].value = v` follow-up: the C only `memcpy`s `keysize` bytes, so
    /// everything past that is uninitialised `realloc` memory in BOTH libraries.
    InlineKeyOnly(usize),
}

#[derive(Default, PartialEq, Eq)]
pub struct Snap(pub Vec<String>);

impl Snap {
    fn push<T: std::fmt::Debug>(&mut self, k: &str, v: T) {
        self.0.push(format!("{k}={v:?}"));
    }
}

impl std::fmt::Debug for Snap {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f)?;
        for l in &self.0 {
            writeln!(f, "  {l}")?;
        }
        Ok(())
    }
}

/// Snapshot a raw dynamic array (`a` is the *element* base pointer as returned
/// by `stbds_arrgrowf`).
pub unsafe fn snap_array(a: *mut c_void, elemsize: usize) -> Snap {
    let mut s = Snap::default();
    if a.is_null() {
        s.push("array", "NULL");
        return s;
    }
    let h = &*((a as usize - HEADER_SIZE) as *const CHeader);
    s.push("len", h.length);
    s.push("cap", h.capacity);
    s.push("temp", h.temp);
    s.push("has_table", !h.hash_table.is_null());
    let n = elemsize.saturating_mul(h.length);
    let bytes = std::slice::from_raw_parts(a as *const u8, n);
    s.push("elems", hex(bytes));
    s
}

/// Snapshot a hash-map "user" pointer `t` (== element base + elemsize).
///
/// `keyrepr` tells the snapshotter whether the first 8 bytes of each element are
/// raw key bytes or a `char *` that must be dereferenced (pointers differ
/// between the two libraries, their targets must not).
///
/// NOTE: `stbds_make_hash_index` never initialises `stbds_hash_index::temp_key`,
/// so it holds uninitialised `realloc` bytes unless `string.mode` is one of
/// `SH_DEFAULT`/`SH_STRDUP`/`SH_ARENA` (the only cases where `stbds_hmput_key`
/// assigns `stbds_temp_key`). It is therefore only snapshotted in those cases.
pub unsafe fn snap_map(t: *mut c_void, elemsize: usize, keyrepr: KeyRepr) -> Snap {
    let mut s = Snap::default();
    if t.is_null() {
        s.push("map", "NULL");
        return s;
    }
    let raw = (t as usize - elemsize) as *mut c_void;
    let h = &*((raw as usize - HEADER_SIZE) as *const CHeader);
    s.push("len", h.length);
    s.push("cap", h.capacity);
    s.push("temp", h.temp);

    let keyrepr = if let KeyRepr::Auto = keyrepr {
        let m = if h.hash_table.is_null() {
            0u8
        } else {
            (*(h.hash_table as *const CHashIndex)).string.mode
        };
        if matches!(m, 2 | 3) {
            KeyRepr::CharPtr
        } else {
            KeyRepr::Inline
        }
    } else {
        keyrepr
    };

    // elements: element 0 is the "default" row; elements 1.. are the live rows
    for i in 0..h.length {
        let ep = (raw as usize + elemsize * i) as *const u8;
        match keyrepr {
            KeyRepr::Inline => {
                s.push(
                    &format!("e{i}"),
                    hex(std::slice::from_raw_parts(ep, elemsize)),
                );
            }
            KeyRepr::Auto => unreachable!(),
            KeyRepr::InlineKeyOnly(n) => {
                let take = if i == 0 { elemsize } else { n.min(elemsize) };
                s.push(
                    &format!("e{i}"),
                    hex(std::slice::from_raw_parts(ep, take)),
                );
            }
            KeyRepr::CharPtr => {
                let key = if i == 0 || elemsize < 8 {
                    // row 0 is memset to zero by the library
                    hex(std::slice::from_raw_parts(ep, elemsize.min(8)))
                } else {
                    let kp = *(ep as *const *const c_char);
                    if kp.is_null() {
                        "<null>".to_string()
                    } else {
                        cstr(kp)
                    }
                };
                let tail = if elemsize > 8 {
                    hex(std::slice::from_raw_parts(ep.add(8), elemsize - 8))
                } else {
                    String::new()
                };
                s.push(&format!("e{i}"), format!("key={key} rest={tail}"));
            }
        }
    }

    if h.hash_table.is_null() {
        s.push("table", "NULL");
        return s;
    }
    let ti = &*(h.hash_table as *const CHashIndex);
    s.push("slot_count", ti.slot_count);
    s.push("used_count", ti.used_count);
    s.push("used_thr", ti.used_count_threshold);
    s.push("shrink_thr", ti.used_count_shrink_threshold);
    s.push("tomb_count", ti.tombstone_count);
    s.push("tomb_thr", ti.tombstone_count_threshold);
    s.push("seed", ti.seed);
    s.push("log2", ti.slot_count_log2);
    s.push("arena_remaining", ti.string.remaining);
    s.push("arena_block", ti.string.block);
    s.push("arena_mode", ti.string.mode);
    s.push("arena_has_storage", !ti.string.storage.is_null());
    // NOTE: `stbds_hash_index::temp_key` is deliberately NOT snapshotted here.
    // `stbds_make_hash_index` never initialises it and never carries it over on
    // rehash / shrink / rebuild, so it holds uninitialised `realloc` bytes at
    // every point except immediately after an insert. `DiffMap::put` compares it
    // exactly at those points instead (see `check_temp_key_after_insert`).
    // full bucket contents
    let nbuckets = ti.slot_count >> BUCKET_SHIFT;
    for b in 0..nbuckets {
        let bk = &*ti.storage.add(b);
        s.push(&format!("b{b}.hash"), bk.hash);
        s.push(&format!("b{b}.index"), bk.index);
    }
    s
}

/// `temp_key` for string maps points into library-private storage; when the map
/// stores the caller's pointer verbatim (`SH_DEFAULT`) both libraries must
/// return the *identical* pointer, which this checks.
pub unsafe fn temp_key_ptr(t: *mut c_void, elemsize: usize) -> *mut c_char {
    let raw = (t as usize - elemsize) as *mut c_void;
    let h = &*((raw as usize - HEADER_SIZE) as *const CHeader);
    if h.hash_table.is_null() {
        return std::ptr::null_mut();
    }
    (*(h.hash_table as *const CHashIndex)).temp_key
}

pub unsafe fn header_of(a: *mut c_void) -> CHeader {
    *((a as usize - HEADER_SIZE) as *const CHeader)
}

pub unsafe fn map_header(t: *mut c_void, elemsize: usize) -> CHeader {
    header_of((t as usize - elemsize) as *mut c_void)
}

pub unsafe fn cstr(p: *const c_char) -> String {
    let mut v = Vec::new();
    let mut q = p as *const u8;
    while *q != 0 {
        v.push(*q);
        q = q.add(1);
    }
    v.iter().map(|b| format!("{b:02x}")).collect()
}

pub fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}

/// Snapshot an arena struct (owned by the caller, so pointers are private).
pub unsafe fn snap_arena(a: &CArena) -> Snap {
    let mut s = Snap::default();
    s.push("remaining", a.remaining);
    s.push("block", a.block);
    s.push("mode", a.mode);
    s.push("has_storage", !a.storage.is_null());
    // walk the block chain and record its length (contents are addresses, so
    // only the shape is comparable)
    let mut n = 0usize;
    let mut p = a.storage as *const *const c_void;
    while !p.is_null() && n < 4096 {
        n += 1;
        p = *p as *const *const c_void;
    }
    s.push("chain_len", n);
    s
}

/// Assert two snapshots are identical, with a helpful diff.
#[track_caller]
pub fn same(ctx: &str, c: &Snap, r: &Snap) {
    if c != r {
        let mut msg = format!("DIVERGENCE at {ctx}\n");
        let n = c.0.len().max(r.0.len());
        for i in 0..n {
            let cl = c.0.get(i).map(|s| s.as_str()).unwrap_or("<missing>");
            let rl = r.0.get(i).map(|s| s.as_str()).unwrap_or("<missing>");
            if cl != rl {
                msg += &format!("  C   : {cl}\n  Rust: {rl}\n");
            }
        }
        panic!("{msg}");
    }
}

#[track_caller]
pub fn same_val<T: PartialEq + std::fmt::Debug>(ctx: &str, c: T, r: T) {
    assert_eq!(c, r, "DIVERGENCE at {ctx}: C={c:?} Rust={r:?}");
}

// ---------------------------------------------------------------------------
// DiffMap — drives BOTH libraries through the *macro-level* protocol
// ---------------------------------------------------------------------------
//
// The public stb_ds macros do more than call the `stbds_hm*_key` functions;
// they also write the key/value into the returned slot:
//
//   stbds_hmput(t,k,v):  t = hmput_key(...,BINARY); t[temp].key = k; t[temp].value = v;
//   stbds_shput(t,k,v):  t = hmput_key(...,STRING); t[temp].value = v;
//
// `DiffMap` reproduces that exactly, so every byte of every live element is
// deterministic and can be compared. Driving only the raw functions would leave
// the value half of each element uninitialised.

/// Owns the key buffers so that `SH_DEFAULT` maps (which store the caller's
/// pointer verbatim) keep pointing at live, stable memory.
pub struct KeyArena(Vec<Vec<u8>>);

impl KeyArena {
    pub fn new() -> KeyArena {
        KeyArena(Vec::new())
    }
    /// Stores `bytes` and returns a stable pointer to it.
    pub fn add(&mut self, bytes: &[u8]) -> *mut u8 {
        self.0.push(bytes.to_vec());
        self.0.last_mut().unwrap().as_mut_ptr()
    }
    pub fn len(&self) -> usize {
        self.0.len()
    }
}

pub struct DiffMap<'a> {
    pub p: &'a Pair,
    pub elemsize: usize,
    pub keysize: usize,
    pub mode: c_int,
    pub keyrepr: KeyRepr,
    pub ct: *mut c_void,
    pub rt: *mut c_void,
    /// number of ops applied (for error context)
    pub op: usize,
}

impl<'a> DiffMap<'a> {
    /// Table created lazily by `stbds_hmput_key` (implicit `string.mode`).
    pub fn lazy(p: &'a Pair, elemsize: usize, keysize: usize, mode: c_int, keyrepr: KeyRepr) -> Self {
        DiffMap {
            p,
            elemsize,
            keysize,
            mode,
            keyrepr,
            ct: std::ptr::null_mut(),
            rt: std::ptr::null_mut(),
            op: 0,
        }
    }

    /// Table created eagerly by `stbds_shmode_func(elemsize, shmode)`
    /// (== `stbds_sh_new_strdup` / `stbds_sh_new_arena`).
    pub fn shmode(
        p: &'a Pair,
        elemsize: usize,
        keysize: usize,
        mode: c_int,
        shmode: c_int,
        keyrepr: KeyRepr,
    ) -> Self {
        let (ct, rt) = unsafe {
            (
                (p.c.shmode_func)(elemsize, shmode),
                (p.r.shmode_func)(elemsize, shmode),
            )
        };
        DiffMap {
            p,
            elemsize,
            keysize,
            mode,
            keyrepr,
            ct,
            rt,
            op: 0,
        }
    }

    /// Where the "value" half of an element begins, mirroring the layouts the
    /// stb_ds macros produce (`struct { K key; V value; }`).
    fn value_offset(&self) -> usize {
        match self.keyrepr {
            KeyRepr::Inline => self.keysize.min(self.elemsize),
            KeyRepr::CharPtr | KeyRepr::Auto => 8usize.min(self.elemsize),
            KeyRepr::InlineKeyOnly(n) => n.min(self.elemsize),
        }
    }

    unsafe fn write_slot(&self, t: *mut c_void, value: &[u8]) {
        if self.elemsize == 0 || t.is_null() {
            return;
        }
        let raw = (t as usize - self.elemsize) as *mut c_void;
        let temp = (*((raw as usize - HEADER_SIZE) as *const CHeader)).temp;
        if temp < 0 {
            return;
        }
        // t[temp] == raw + elemsize*(temp+1)
        let ep = (t as usize).wrapping_add(self.elemsize.wrapping_mul(temp as usize)) as *mut u8;
        let voff = self.value_offset();
        let n = self.elemsize - voff;
        for i in 0..n.min(value.len()) {
            *ep.add(voff + i) = value[i];
        }
        for i in value.len()..n {
            *ep.add(voff + i) = 0;
        }
    }

    /// `stbds_hmput` / `stbds_shput`
    pub fn put(&mut self, key: *mut u8, value: &[u8]) -> (isize, isize) {
        self.op += 1;
        unsafe {
            let before = self.lens();
            self.ct = (self.p.c.hmput_key)(
                self.ct,
                self.elemsize,
                key as *mut c_void,
                self.keysize,
                self.mode,
            );
            self.rt = (self.p.r.hmput_key)(
                self.rt,
                self.elemsize,
                key as *mut c_void,
                self.keysize,
                self.mode,
            );
            self.write_slot(self.ct, value);
            self.write_slot(self.rt, value);
            let after = self.lens();
            if after.0 > before.0 || after.1 > before.1 {
                same_val("put length step", after.0 - before.0, after.1 - before.1);
                self.check_temp_key_after_insert();
            }
            (self.temp_c(), self.temp_r())
        }
    }

    unsafe fn lens(&self) -> (usize, usize) {
        (
            if self.ct.is_null() { 0 } else { map_header(self.ct, self.elemsize).length },
            if self.rt.is_null() { 0 } else { map_header(self.rt, self.elemsize).length },
        )
    }

    /// `stbds_temp_key` is assigned by `stbds_hmput_key` only for
    /// `string.mode` in {SH_DEFAULT, SH_STRDUP, SH_ARENA}. Right after an insert
    /// it is therefore well-defined and must agree between the libraries.
    #[track_caller]
    unsafe fn check_temp_key_after_insert(&self) {
        let (cti, rti) = (
            table_of(self.ct, self.elemsize),
            table_of(self.rt, self.elemsize),
        );
        let (cti, rti) = match (cti, rti) {
            (Some(a), Some(b)) => (a, b),
            _ => return,
        };
        same_val("temp_key: string.mode", cti.string.mode, rti.string.mode);
        if !matches!(cti.string.mode, 1 | 2 | 3) {
            return;
        }
        let ck = if cti.temp_key.is_null() { "<null>".into() } else { cstr(cti.temp_key) };
        let rk = if rti.temp_key.is_null() { "<null>".into() } else { cstr(rti.temp_key) };
        same_val(
            &format!("temp_key content [op {}]", self.op),
            ck,
            rk,
        );
        if cti.string.mode == 1 {
            // SH_DEFAULT stores the caller's pointer verbatim
            same_val(
                &format!("temp_key pointer [op {}]", self.op),
                cti.temp_key as usize,
                rti.temp_key as usize,
            );
        }
    }

    /// `stbds_hmgeti` / `stbds_shgeti`
    pub fn get(&mut self, key: *mut u8) -> (isize, isize) {
        self.op += 1;
        unsafe {
            self.ct = (self.p.c.hmget_key)(
                self.ct,
                self.elemsize,
                key as *mut c_void,
                self.keysize,
                self.mode,
            );
            self.rt = (self.p.r.hmget_key)(
                self.rt,
                self.elemsize,
                key as *mut c_void,
                self.keysize,
                self.mode,
            );
            (self.temp_c(), self.temp_r())
        }
    }

    /// `stbds_hmgeti_ts`
    pub fn get_ts(&mut self, key: *mut u8) -> (isize, isize) {
        self.op += 1;
        unsafe {
            let mut tc: isize = 0x5555_5555;
            let mut tr: isize = 0x5555_5555;
            self.ct = (self.p.c.hmget_key_ts)(
                self.ct,
                self.elemsize,
                key as *mut c_void,
                self.keysize,
                &mut tc,
                self.mode,
            );
            self.rt = (self.p.r.hmget_key_ts)(
                self.rt,
                self.elemsize,
                key as *mut c_void,
                self.keysize,
                &mut tr,
                self.mode,
            );
            (tc, tr)
        }
    }

    /// `stbds_hmdel` / `stbds_shdel`
    pub fn del(&mut self, key: *mut u8, keyoffset: usize) -> (isize, isize) {
        self.op += 1;
        unsafe {
            self.ct = (self.p.c.hmdel_key)(
                self.ct,
                self.elemsize,
                key as *mut c_void,
                self.keysize,
                keyoffset,
                self.mode,
            );
            self.rt = (self.p.r.hmdel_key)(
                self.rt,
                self.elemsize,
                key as *mut c_void,
                self.keysize,
                keyoffset,
                self.mode,
            );
            // the macro yields `(t) ? stbds_temp((t)-1) : 0`
            let tc = if self.ct.is_null() { 0 } else { self.temp_c() };
            let tr = if self.rt.is_null() { 0 } else { self.temp_r() };
            (tc, tr)
        }
    }

    /// `stbds_hmdefault` (writes the default value into row 0)
    pub fn put_default(&mut self, value: &[u8]) {
        self.op += 1;
        unsafe {
            self.ct = (self.p.c.hmput_default)(self.ct, self.elemsize);
            self.rt = (self.p.r.hmput_default)(self.rt, self.elemsize);
            // (t)[-1].value = v  ==  raw[0].value = v
            for (t, _) in [(self.ct, 0u8), (self.rt, 0u8)] {
                if t.is_null() || self.elemsize == 0 {
                    continue;
                }
                let raw = (t as usize - self.elemsize) as *mut u8;
                let voff = self.value_offset();
                let n = self.elemsize - voff;
                for i in 0..n.min(value.len()) {
                    *raw.add(voff + i) = value[i];
                }
            }
        }
    }

    pub unsafe fn temp_c(&self) -> isize {
        if self.ct.is_null() {
            return 0;
        }
        map_header(self.ct, self.elemsize).temp
    }
    pub unsafe fn temp_r(&self) -> isize {
        if self.rt.is_null() {
            return 0;
        }
        map_header(self.rt, self.elemsize).temp
    }

    pub fn snaps(&self) -> (Snap, Snap) {
        unsafe {
            (
                snap_map(self.ct, self.elemsize, self.keyrepr),
                snap_map(self.rt, self.elemsize, self.keyrepr),
            )
        }
    }

    #[track_caller]
    pub fn check(&self, ctx: &str) {
        let (c, r) = self.snaps();
        same(&format!("{ctx} [op {}]", self.op), &c, &r);
        same_val(
            &format!("{ctx} [op {}] null-ness", self.op),
            self.ct.is_null(),
            self.rt.is_null(),
        );
    }

    /// `stbds_hmfree` / `stbds_shfree`
    pub fn free(&mut self) {
        unsafe {
            if !self.ct.is_null() {
                (self.p.c.hmfree_func)((self.ct as usize - self.elemsize) as *mut c_void, self.elemsize);
            }
            if !self.rt.is_null() {
                (self.p.r.hmfree_func)((self.rt as usize - self.elemsize) as *mut c_void, self.elemsize);
            }
            self.ct = std::ptr::null_mut();
            self.rt = std::ptr::null_mut();
        }
    }
}

// ---------------------------------------------------------------------------
// DiffArr — drives BOTH libraries through the stb_ds *array* macro protocol
// ---------------------------------------------------------------------------
//
// Faithful expansions of lib.c:108-131 over byte-sized elements, so that the
// low-level `stbds_arrgrowf` is exercised exactly as the public macros do.

pub struct DiffArr<'a> {
    pub p: &'a Pair,
    pub elemsize: usize,
    pub ca: *mut c_void,
    pub ra: *mut c_void,
    pub op: usize,
}

impl<'a> DiffArr<'a> {
    pub fn new(p: &'a Pair, elemsize: usize) -> Self {
        DiffArr {
            p,
            elemsize,
            ca: std::ptr::null_mut(),
            ra: std::ptr::null_mut(),
            op: 0,
        }
    }

    unsafe fn hdr(a: *mut c_void) -> *mut CHeader {
        (a as usize - HEADER_SIZE) as *mut CHeader
    }

    pub unsafe fn len(a: *mut c_void) -> usize {
        if a.is_null() {
            0
        } else {
            (*Self::hdr(a)).length
        }
    }
    pub unsafe fn cap(a: *mut c_void) -> usize {
        if a.is_null() {
            0
        } else {
            (*Self::hdr(a)).capacity
        }
    }

    /// `stbds_arrmaybegrow(a,n)` + `stbds_arrgrow(a,n,0)`
    unsafe fn maybegrow(&self, growf: FnArrGrowF, a: *mut c_void, n: usize) -> *mut c_void {
        if a.is_null()
            || (*Self::hdr(a)).length.wrapping_add(n) > (*Self::hdr(a)).capacity
        {
            growf(a, self.elemsize, n, 0)
        } else {
            a
        }
    }

    /// `stbds_arrput(a, v)`
    pub fn put(&mut self, value: &[u8]) {
        self.op += 1;
        unsafe {
            for side in 0..2 {
                let (growf, a) = if side == 0 {
                    (self.p.c.arrgrowf, self.ca)
                } else {
                    (self.p.r.arrgrowf, self.ra)
                };
                let a = self.maybegrow(growf, a, 1);
                let h = Self::hdr(a);
                let ep = (a as usize + self.elemsize * (*h).length) as *mut u8;
                for i in 0..self.elemsize {
                    *ep.add(i) = value.get(i).copied().unwrap_or(0);
                }
                (*h).length = (*h).length.wrapping_add(1);
                if side == 0 {
                    self.ca = a
                } else {
                    self.ra = a
                }
            }
        }
    }

    /// `stbds_arrpop(a)` — returns the popped element bytes from both libs
    pub fn pop(&mut self) -> (Vec<u8>, Vec<u8>) {
        self.op += 1;
        unsafe {
            let mut out = Vec::new();
            for a in [self.ca, self.ra] {
                let h = Self::hdr(a);
                (*h).length = (*h).length.wrapping_sub(1);
                let ep = (a as usize + self.elemsize * (*h).length) as *const u8;
                out.push(std::slice::from_raw_parts(ep, self.elemsize).to_vec());
            }
            (out[0].clone(), out[1].clone())
        }
    }

    /// `stbds_arraddnindex(a, n)` (== `arraddn` / `arraddnptr` index form)
    pub fn addn(&mut self, n: usize) -> (isize, isize) {
        self.op += 1;
        unsafe {
            let mut out = [0isize; 2];
            for side in 0..2 {
                let (growf, a) = if side == 0 {
                    (self.p.c.arrgrowf, self.ca)
                } else {
                    (self.p.r.arrgrowf, self.ra)
                };
                let a = self.maybegrow(growf, a, n);
                out[side] = if n != 0 {
                    let h = Self::hdr(a);
                    let old = (*h).length;
                    (*h).length = (*h).length.wrapping_add(n);
                    // arraddn/arraddnptr reserve slots without initialising them;
                    // a real consumer fills them, and so must we (identical
                    // pattern on both sides) or we would compare heap garbage.
                    let base = a as *mut u8;
                    for b in (old * self.elemsize)..((old + n) * self.elemsize) {
                        *base.add(b) = 0xCD;
                    }
                    old as isize
                } else {
                    Self::len(a) as isize
                };
                if side == 0 {
                    self.ca = a
                } else {
                    self.ra = a
                }
            }
            (out[0], out[1])
        }
    }

    /// `stbds_arrins(a, i, v)` (via `stbds_arrinsn`)
    pub fn ins(&mut self, i: usize, value: &[u8]) {
        self.op += 1;
        self.insn(i, 1);
        unsafe {
            for a in [self.ca, self.ra] {
                let ep = (a as usize + self.elemsize * i) as *mut u8;
                for k in 0..self.elemsize {
                    *ep.add(k) = value.get(k).copied().unwrap_or(0);
                }
            }
        }
    }

    /// `stbds_arrinsn(a, i, n)`
    pub fn insn(&mut self, i: usize, n: usize) {
        unsafe {
            self.addn(n);
            for a in [self.ca, self.ra] {
                let h = Self::hdr(a);
                let count = (*h).length.wrapping_sub(n).wrapping_sub(i);
                std::ptr::copy(
                    (a as usize + self.elemsize * i) as *const u8,
                    (a as usize + self.elemsize * (i + n)) as *mut u8,
                    self.elemsize.wrapping_mul(count),
                );
            }
        }
    }

    /// `stbds_arrdeln(a, i, n)`
    pub fn deln(&mut self, i: usize, n: usize) {
        self.op += 1;
        unsafe {
            for a in [self.ca, self.ra] {
                let h = Self::hdr(a);
                let count = (*h).length.wrapping_sub(n).wrapping_sub(i);
                std::ptr::copy(
                    (a as usize + self.elemsize * (i + n)) as *const u8,
                    (a as usize + self.elemsize * i) as *mut u8,
                    self.elemsize.wrapping_mul(count),
                );
                (*h).length = (*h).length.wrapping_sub(n);
            }
        }
    }

    /// `stbds_arrdelswap(a, i)`
    pub fn delswap(&mut self, i: usize) {
        self.op += 1;
        unsafe {
            for a in [self.ca, self.ra] {
                let h = Self::hdr(a);
                let last = (a as usize + self.elemsize * ((*h).length - 1)) as *const u8;
                std::ptr::copy(
                    last,
                    (a as usize + self.elemsize * i) as *mut u8,
                    self.elemsize,
                );
                (*h).length = (*h).length.wrapping_sub(1);
            }
        }
    }

    /// `stbds_arrsetcap(a, n)`
    pub fn setcap(&mut self, n: usize) {
        self.op += 1;
        unsafe {
            self.ca = (self.p.c.arrgrowf)(self.ca, self.elemsize, 0, n);
            self.ra = (self.p.r.arrgrowf)(self.ra, self.elemsize, 0, n);
        }
    }

    /// `stbds_arrsetlen(a, n)`
    pub fn setlen(&mut self, n: usize) {
        self.op += 1;
        unsafe {
            for side in 0..2 {
                let (growf, mut a) = if side == 0 {
                    (self.p.c.arrgrowf, self.ca)
                } else {
                    (self.p.r.arrgrowf, self.ra)
                };
                if Self::cap(a) < n {
                    a = growf(a, self.elemsize, 0, n);
                }
                if !a.is_null() {
                    (*Self::hdr(a)).length = n;
                }
                if side == 0 {
                    self.ca = a
                } else {
                    self.ra = a
                }
            }
        }
    }

    /// `stbds_arrfree(a)`
    pub fn free(&mut self) {
        self.op += 1;
        unsafe {
            if !self.ca.is_null() {
                (self.p.c.arrfreef)(self.ca);
            }
            if !self.ra.is_null() {
                (self.p.r.arrfreef)(self.ra);
            }
            self.ca = std::ptr::null_mut();
            self.ra = std::ptr::null_mut();
        }
    }

    pub fn snaps(&self) -> (Snap, Snap) {
        unsafe {
            (
                snap_array(self.ca, self.elemsize),
                snap_array(self.ra, self.elemsize),
            )
        }
    }

    #[track_caller]
    pub fn check(&self, ctx: &str) {
        let (c, r) = self.snaps();
        same(&format!("{ctx} [op {}]"  , self.op), &c, &r);
    }
}

// ---------------------------------------------------------------------------
// Collision engineering: find keys that probe into a chosen bucket
// ---------------------------------------------------------------------------

/// The effective hash the library probes with (`if (hash < 2) hash += 2`).
pub unsafe fn effective_hash_bytes(l: &Lib, key: &mut [u8], seed: usize) -> usize {
    let h = (l.hash_bytes)(key.as_mut_ptr() as *mut c_void, key.len(), seed);
    if h < 2 { h + 2 } else { h }
}

pub unsafe fn effective_hash_string(l: &Lib, key: &mut [u8], seed: usize) -> usize {
    let h = (l.hash_string)(key.as_mut_ptr() as *mut c_char, seed);
    if h < 2 { h + 2 } else { h }
}

/// Brute-force a 4-byte binary key whose probe position lands in `bucket`.
pub unsafe fn key_in_bucket_bin(
    l: &Lib,
    seed: usize,
    slot_count: usize,
    bucket: usize,
    counter: &mut u32,
) -> [u8; 4] {
    loop {
        let mut k = counter.to_le_bytes();
        *counter = counter.wrapping_add(1);
        let h = effective_hash_bytes(l, &mut k, seed);
        if ((h & (slot_count - 1)) >> BUCKET_SHIFT) == bucket {
            return k;
        }
    }
}

/// Brute-force a printable string key whose probe position lands in `bucket`.
pub unsafe fn key_in_bucket_str(
    l: &Lib,
    seed: usize,
    slot_count: usize,
    bucket: usize,
    counter: &mut u32,
) -> Vec<u8> {
    loop {
        let mut k: Vec<u8> = format!("k{}", *counter).into_bytes();
        k.push(0);
        *counter = counter.wrapping_add(1);
        let h = effective_hash_string(l, &mut k, seed);
        if ((h & (slot_count - 1)) >> BUCKET_SHIFT) == bucket {
            return k;
        }
    }
}

/// Read the live hash-index scalars of a map (`t` = user pointer).
pub unsafe fn table_of(t: *mut c_void, elemsize: usize) -> Option<CHashIndex> {
    if t.is_null() {
        return None;
    }
    let h = map_header(t, elemsize);
    if h.hash_table.is_null() {
        None
    } else {
        Some(*(h.hash_table as *const CHashIndex))
    }
}

/// Count non-empty slots in bucket `b`.
pub unsafe fn bucket_fill(t: *mut c_void, elemsize: usize, b: usize) -> usize {
    match table_of(t, elemsize) {
        None => 0,
        Some(ti) => {
            let bk = &*ti.storage.add(b);
            bk.hash.iter().filter(|&&h| h != 0).count()
        }
    }
}

// ---------------------------------------------------------------------------
// Self-referential keys for the mixed-mode quirk (CONFIGS C31 / ERRORS E64)
// ---------------------------------------------------------------------------
//
// `stbds_shmode_func(elemsize, STBDS_SH_NONE)` produces a table whose
// `string.mode` is 0, so `stbds_hmput_key(..., STBDS_HM_STRING)` hashes/compares
// the key as a *string* but stores it with the `default:` label
// `memcpy(a + elemsize*i, key, keysize)` — i.e. raw bytes. `stbds_is_key_equal`
// then reads those bytes back as a `char *` and dereferences them.
//
// The only way to drive that path without an invalid dereference is with keys
// whose first 8 bytes ARE their own address. Such a key round-trips: the stored
// 8 bytes equal the key pointer, so `strcmp(key, *(char**)elem)` compares the
// buffer with itself.
pub struct SelfKeys {
    _backing: Vec<u8>,
    pub keys: Vec<*mut u8>,
}

impl SelfKeys {
    /// Build `n` distinct self-referential keys (n <= 200).
    pub fn new(n: usize) -> SelfKeys {
        assert!(n <= 200);
        let mut backing = vec![0u8; 8192];
        let base = backing.as_mut_ptr() as usize;
        let mut keys = Vec::new();
        let mut used_low = std::collections::HashSet::new();
        let mut off = 0usize;
        while keys.len() < n && off + 16 < backing.len() {
            let addr = base + off;
            let low = (addr & 0xff) as u8;
            // byte 0 of the "string" must be non-NUL and unique, so that two
            // distinct keys never strcmp() as equal
            if low != 0 && used_low.insert(low) {
                let bytes = (addr as u64).to_le_bytes();
                backing[off..off + 8].copy_from_slice(&bytes);
                keys.push(addr as *mut u8);
                off += 16;
            } else {
                off += 1;
            }
        }
        assert_eq!(keys.len(), n, "could not build enough self-referential keys");
        SelfKeys {
            _backing: backing,
            keys,
        }
    }
}

/// Brute-force a 4-byte binary key whose probe position is exactly `slot`.
pub unsafe fn key_at_slot_bin(
    l: &Lib,
    seed: usize,
    slot_count: usize,
    slot: usize,
    counter: &mut u32,
) -> [u8; 4] {
    for _ in 0..100_000_000u64 {
        let mut k = counter.to_le_bytes();
        *counter = counter.wrapping_add(1);
        let h = effective_hash_bytes(l, &mut k, seed);
        if (h & (slot_count - 1)) == slot {
            return k;
        }
    }
    panic!("no key found for slot {slot}");
}

/// Brute-force a printable string key whose probe position is exactly `slot`.
pub unsafe fn key_at_slot_str(
    l: &Lib,
    seed: usize,
    slot_count: usize,
    slot: usize,
    counter: &mut u32,
) -> Vec<u8> {
    for _ in 0..100_000_000u64 {
        let mut k: Vec<u8> = format!("s{}", *counter).into_bytes();
        k.push(0);
        *counter = counter.wrapping_add(1);
        let h = effective_hash_string(l, &mut k, seed);
        if (h & (slot_count - 1)) == slot {
            return k;
        }
    }
    panic!("no string key found for slot {slot}");
}

/// Raw bucket read-out for a map (`t` = user pointer).
pub unsafe fn bucket(t: *mut c_void, elemsize: usize, b: usize) -> CBucket {
    let ti = table_of(t, elemsize).expect("no table");
    *ti.storage.add(b)
}
