//! Shared differential-test harness.
//!
//! Loads BOTH the C `.so` and the Rust `.so` with `libloading` and drives them
//! through their exported symbols only.  Nothing in this file calls a Rust
//! function of the crate directly.

#![allow(dead_code)]
#![allow(non_camel_case_types)]

use core::ffi::{c_char, c_int, c_void};
use std::path::PathBuf;
use std::sync::{Mutex, MutexGuard, OnceLock};

// ---------------------------------------------------------------------------
// C data-structure mirrors (used only to *read* state back out of each lib)
// ---------------------------------------------------------------------------

pub const HDR_SIZE: usize = 32;
pub const BUCKET_LENGTH: usize = 8;
pub const BUCKET_SHIFT: usize = 3;
pub const BUCKET_MASK: usize = 7;

pub const STBDS_HM_BINARY: c_int = 0;
pub const STBDS_HM_STRING: c_int = 1;
pub const STBDS_HM_PTR_TO_STRING: c_int = 2;

pub const STBDS_SH_NONE: c_int = 0;
pub const STBDS_SH_DEFAULT: c_int = 1;
pub const STBDS_SH_STRDUP: c_int = 2;
pub const STBDS_SH_ARENA: c_int = 3;

pub const STBDS_INDEX_EMPTY: isize = -1;
pub const STBDS_INDEX_DELETED: isize = -2;
pub const STBDS_HASH_EMPTY: usize = 0;
pub const STBDS_HASH_DELETED: usize = 1;

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct CArrayHeader {
    pub length: usize,
    pub capacity: usize,
    pub hash_table: *mut c_void,
    pub temp: isize,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct CStringBlock {
    pub next: *mut CStringBlock,
    pub storage: [c_char; 8],
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct CStringArena {
    pub storage: *mut CStringBlock,
    pub remaining: usize,
    pub block: u8,
    pub mode: u8,
}

impl CStringArena {
    pub fn zeroed() -> Self {
        CStringArena {
            storage: std::ptr::null_mut(),
            remaining: 0,
            block: 0,
            mode: 0,
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct CHashBucket {
    pub hash: [usize; BUCKET_LENGTH],
    pub index: [isize; BUCKET_LENGTH],
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
    pub string: CStringArena,
    pub storage: *mut CHashBucket,
}

// Layout must be what the C compiler produced (see SYMBOLS.md).
const _: () = assert!(core::mem::size_of::<CArrayHeader>() == 32);
const _: () = assert!(core::mem::size_of::<CStringBlock>() == 16);
const _: () = assert!(core::mem::size_of::<CStringArena>() == 24);
const _: () = assert!(core::mem::size_of::<CHashBucket>() == 128);
const _: () = assert!(core::mem::size_of::<CHashIndex>() == 104);
const _: () = assert!(core::mem::offset_of!(CHashIndex, string) == 72);
const _: () = assert!(core::mem::offset_of!(CHashIndex, storage) == 96);

// ---------------------------------------------------------------------------
// Library wrapper
// ---------------------------------------------------------------------------

pub type FnArrgrowf = unsafe extern "C" fn(*mut c_void, usize, usize, usize) -> *mut c_void;
pub type FnArrfreef = unsafe extern "C" fn(*mut c_void);
pub type FnRandSeed = unsafe extern "C" fn(usize);
pub type FnHashString = unsafe extern "C" fn(*mut c_char, usize) -> usize;
pub type FnHashBytes = unsafe extern "C" fn(*mut c_void, usize, usize) -> usize;
pub type FnHmfreeFunc = unsafe extern "C" fn(*mut c_void, usize);
pub type FnHmgetKeyTs =
    unsafe extern "C" fn(*mut c_void, usize, *mut c_void, usize, *mut isize, c_int) -> *mut c_void;
pub type FnHmgetKey =
    unsafe extern "C" fn(*mut c_void, usize, *mut c_void, usize, c_int) -> *mut c_void;
pub type FnHmputDefault = unsafe extern "C" fn(*mut c_void, usize) -> *mut c_void;
pub type FnHmputKey =
    unsafe extern "C" fn(*mut c_void, usize, *mut c_void, usize, c_int) -> *mut c_void;
pub type FnShmodeFunc = unsafe extern "C" fn(usize, c_int) -> *mut c_void;
pub type FnHmdelKey =
    unsafe extern "C" fn(*mut c_void, usize, *mut c_void, usize, usize, c_int) -> *mut c_void;
pub type FnStralloc = unsafe extern "C" fn(*mut CStringArena, *mut c_char) -> *mut c_char;
pub type FnStrreset = unsafe extern "C" fn(*mut CStringArena);
pub type FnStrkey = unsafe extern "C" fn(c_int) -> *mut c_char;
pub type FnArrPush = unsafe extern "C" fn(c_int);

pub struct Lib {
    pub name: &'static str,
    pub path: PathBuf,
    pub arrgrowf: FnArrgrowf,
    pub arrfreef: FnArrfreef,
    pub rand_seed: FnRandSeed,
    pub hash_string: FnHashString,
    pub hash_bytes: FnHashBytes,
    pub hmfree_func: FnHmfreeFunc,
    pub hmget_key_ts: FnHmgetKeyTs,
    pub hmget_key: FnHmgetKey,
    pub hmput_default: FnHmputDefault,
    pub hmput_key: FnHmputKey,
    pub shmode_func: FnShmodeFunc,
    pub hmdel_key: FnHmdelKey,
    pub stralloc: FnStralloc,
    pub strreset: FnStrreset,
    pub strkey: FnStrkey,
    pub arr_push: FnArrPush,
}

/// All 16 exported symbols, in `SYMBOLS.md` order.
pub const EXPORTS: [&str; 16] = [
    "stbds_arrgrowf",
    "stbds_arrfreef",
    "stbds_rand_seed",
    "stbds_hash_string",
    "stbds_hash_bytes",
    "stbds_hmfree_func",
    "stbds_hmget_key_ts",
    "stbds_hmget_key",
    "stbds_hmput_default",
    "stbds_hmput_key",
    "stbds_shmode_func",
    "stbds_hmdel_key",
    "stbds_stralloc",
    "stbds_strreset",
    "strkey",
    "arr_push",
];

unsafe fn sym<T: Copy + 'static>(lib: &'static libloading::Library, name: &str) -> T {
    let raw: libloading::Symbol<'static, T> = lib
        .get(format!("{name}\0").as_bytes())
        .unwrap_or_else(|e| panic!("missing symbol `{name}`: {e}"));
    *raw
}

impl Lib {
    fn load(name: &'static str, path: PathBuf) -> Lib {
        let lib: &'static libloading::Library = Box::leak(Box::new(unsafe {
            libloading::Library::new(&path)
                .unwrap_or_else(|e| panic!("dlopen {} failed: {e}", path.display()))
        }));
        unsafe {
            Lib {
                name,
                path,
                arrgrowf: sym(lib, "stbds_arrgrowf"),
                arrfreef: sym(lib, "stbds_arrfreef"),
                rand_seed: sym(lib, "stbds_rand_seed"),
                hash_string: sym(lib, "stbds_hash_string"),
                hash_bytes: sym(lib, "stbds_hash_bytes"),
                hmfree_func: sym(lib, "stbds_hmfree_func"),
                hmget_key_ts: sym(lib, "stbds_hmget_key_ts"),
                hmget_key: sym(lib, "stbds_hmget_key"),
                hmput_default: sym(lib, "stbds_hmput_default"),
                hmput_key: sym(lib, "stbds_hmput_key"),
                shmode_func: sym(lib, "stbds_shmode_func"),
                hmdel_key: sym(lib, "stbds_hmdel_key"),
                stralloc: sym(lib, "stbds_stralloc"),
                strreset: sym(lib, "stbds_strreset"),
                strkey: sym(lib, "strkey"),
                arr_push: sym(lib, "arr_push"),
            }
        }
    }
}

pub struct Pair {
    pub c: Lib,
    pub r: Lib,
}

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("crate has a parent dir")
        .to_path_buf()
}

fn find_c_so() -> PathBuf {
    if let Ok(p) = std::env::var("C_SO") {
        return PathBuf::from(p);
    }
    let dir = workspace_root().join("c_src").join("build");
    let mut found: Vec<PathBuf> = Vec::new();
    if let Ok(rd) = std::fs::read_dir(&dir) {
        for e in rd.flatten() {
            let p = e.path();
            let n = p.file_name().unwrap_or_default().to_string_lossy().to_string();
            if n.starts_with("lib") && n.ends_with(".so") {
                found.push(p);
            }
        }
    }
    found.sort();
    found.pop().unwrap_or_else(|| {
        panic!(
            "no C .so under {}; build it with:\n  cd c_src && mkdir -p build && cd build && \
             cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
            dir.display()
        )
    })
}

fn find_rust_so() -> PathBuf {
    if let Ok(p) = std::env::var("RUST_SO") {
        return PathBuf::from(p);
    }
    let base = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target");
    // The crate's [profile.release] disables overflow-checks, which is what the
    // C build effectively does, so prefer the release artifact.
    for prof in ["release", "debug"] {
        let p = base.join(prof).join("libarr_push_lib.so");
        if p.exists() {
            return p;
        }
    }
    panic!(
        "no Rust .so under {}; build it with `cargo build --release`",
        base.display()
    )
}

static PAIR: OnceLock<Mutex<Pair>> = OnceLock::new();

fn pair_cell() -> &'static Mutex<Pair> {
    PAIR.get_or_init(|| {
        Mutex::new(Pair {
            c: Lib::load("C", find_c_so()),
            r: Lib::load("RUST", find_rust_so()),
        })
    })
}

/// Serialised access to both libraries.  Serialisation is required because both
/// libraries keep process-global mutable state (`stbds_hash_seed`, the `strkey`
/// static buffer).
pub fn libs() -> MutexGuard<'static, Pair> {
    match pair_cell().lock() {
        Ok(g) => g,
        Err(poisoned) => poisoned.into_inner(),
    }
}

/// Reset the global hash seed in BOTH libraries so that a scenario is
/// reproducible and the two libraries start from identical state.
pub fn reset_seed(p: &Pair, seed: usize) {
    unsafe {
        (p.c.rand_seed)(seed);
        (p.r.rand_seed)(seed);
    }
}

pub const DEFAULT_SEED: usize = 0x3141_5926;

// ---------------------------------------------------------------------------
// Deterministic RNG (no external crates)
// ---------------------------------------------------------------------------

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng(seed ^ 0x9E37_79B9_7F4A_7C15)
    }
    pub fn next_u64(&mut self) -> u64 {
        // splitmix64
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
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
    pub fn range(&mut self, lo: usize, hi_inclusive: usize) -> usize {
        lo + self.below(hi_inclusive - lo + 1)
    }
    pub fn bytes(&mut self, n: usize) -> Vec<u8> {
        (0..n).map(|_| self.next_u32() as u8).collect()
    }
    /// Random NUL-free bytes (valid C string body).
    pub fn cstr_body(&mut self, n: usize, full_byte_range: bool) -> Vec<u8> {
        (0..n)
            .map(|_| {
                if full_byte_range {
                    let b = self.next_u32() as u8;
                    if b == 0 {
                        1
                    } else {
                        b
                    }
                } else {
                    b'a' + (self.next_u32() % 26) as u8
                }
            })
            .collect()
    }
}

// ---------------------------------------------------------------------------
// State snapshots — everything comparable between the two libraries
// ---------------------------------------------------------------------------

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct HdrSnap {
    pub null: bool,
    pub length: usize,
    pub capacity: usize,
    pub temp: isize,
    pub has_table: bool,
}

pub unsafe fn hdr_of(a: *mut c_void) -> *mut CArrayHeader {
    (a as *mut u8).sub(HDR_SIZE) as *mut CArrayHeader
}

pub unsafe fn snap_hdr(a: *mut c_void) -> HdrSnap {
    if a.is_null() {
        return HdrSnap {
            null: true,
            length: 0,
            capacity: 0,
            temp: 0,
            has_table: false,
        };
    }
    let h = *hdr_of(a);
    HdrSnap {
        null: false,
        length: h.length,
        capacity: h.capacity,
        temp: h.temp,
        has_table: !h.hash_table.is_null(),
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct ArenaSnap {
    pub has_storage: bool,
    pub remaining: usize,
    pub block: u8,
    pub mode: u8,
    /// Number of blocks reachable through the `next` chain.
    pub chain_len: usize,
}

pub unsafe fn snap_arena(a: *const CStringArena) -> ArenaSnap {
    let v = *a;
    let mut chain_len = 0usize;
    let mut p = v.storage;
    while !p.is_null() && chain_len < 1_000_000 {
        chain_len += 1;
        p = (*p).next;
    }
    ArenaSnap {
        has_storage: !v.storage.is_null(),
        remaining: v.remaining,
        block: v.block,
        mode: v.mode,
        chain_len,
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct IdxSnap {
    pub present: bool,
    pub slot_count: usize,
    pub used_count: usize,
    pub used_count_threshold: usize,
    pub used_count_shrink_threshold: usize,
    pub tombstone_count: usize,
    pub tombstone_count_threshold: usize,
    pub seed: usize,
    pub slot_count_log2: usize,
    pub arena: ArenaSnap,
    pub storage_aligned_64: bool,
    pub storage_in_alloc: bool,
    /// `(hash, index)` for every one of the `slot_count` slots.
    pub slots: Vec<(usize, isize)>,
}

impl IdxSnap {
    pub fn absent() -> IdxSnap {
        IdxSnap {
            present: false,
            slot_count: 0,
            used_count: 0,
            used_count_threshold: 0,
            used_count_shrink_threshold: 0,
            tombstone_count: 0,
            tombstone_count_threshold: 0,
            seed: 0,
            slot_count_log2: 0,
            arena: ArenaSnap {
                has_storage: false,
                remaining: 0,
                block: 0,
                mode: 0,
                chain_len: 0,
            },
            storage_aligned_64: false,
            storage_in_alloc: false,
            slots: Vec::new(),
        }
    }
}

/// Snapshot the `stbds_hash_index` hanging off an array header.
/// `a` must be the *array* pointer (not the "hash pointer").
pub unsafe fn snap_idx(a: *mut c_void) -> IdxSnap {
    if a.is_null() {
        return IdxSnap::absent();
    }
    let t = (*hdr_of(a)).hash_table as *mut CHashIndex;
    if t.is_null() {
        return IdxSnap::absent();
    }
    let v = *t;
    let n_buckets = v.slot_count >> BUCKET_SHIFT;
    let mut slots = Vec::with_capacity(v.slot_count);
    for b in 0..n_buckets {
        let bk = &*v.storage.add(b);
        for j in 0..BUCKET_LENGTH {
            slots.push((bk.hash[j], bk.index[j]));
        }
    }
    let base = t as usize + core::mem::size_of::<CHashIndex>();
    let st = v.storage as usize;
    IdxSnap {
        present: true,
        slot_count: v.slot_count,
        used_count: v.used_count,
        used_count_threshold: v.used_count_threshold,
        used_count_shrink_threshold: v.used_count_shrink_threshold,
        tombstone_count: v.tombstone_count,
        tombstone_count_threshold: v.tombstone_count_threshold,
        seed: v.seed,
        slot_count_log2: v.slot_count_log2,
        arena: snap_arena(&raw const (*t).string),
        storage_aligned_64: st % 64 == 0,
        storage_in_alloc: st >= base && st < base + 64,
        slots,
    }
}

/// Read the `temp_key` field's *contents* (only valid once a string-mode
/// put/dup-hit has written it; `make_hash_index` leaves it uninitialised).
pub unsafe fn snap_temp_key(a: *mut c_void) -> Option<Vec<u8>> {
    if a.is_null() {
        return None;
    }
    let t = (*hdr_of(a)).hash_table as *mut CHashIndex;
    if t.is_null() {
        return None;
    }
    let k = (*t).temp_key;
    if k.is_null() {
        None
    } else {
        Some(read_cstr(k))
    }
}

/// The raw `temp_key` pointer value (comparable only when both libraries were
/// handed the *same* caller key pointer, i.e. `STBDS_SH_DEFAULT`).
pub unsafe fn raw_temp_key(a: *mut c_void) -> *mut c_char {
    if a.is_null() {
        return std::ptr::null_mut();
    }
    let t = (*hdr_of(a)).hash_table as *mut CHashIndex;
    if t.is_null() {
        std::ptr::null_mut()
    } else {
        (*t).temp_key
    }
}

pub unsafe fn read_cstr(p: *const c_char) -> Vec<u8> {
    let mut out = Vec::new();
    let mut q = p as *const u8;
    while *q != 0 {
        out.push(*q);
        q = q.add(1);
    }
    out
}

// ---------------------------------------------------------------------------
// Element-buffer snapshots
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KeyKind {
    /// The key field holds inline bytes; compare them verbatim.
    Bytes,
    /// The key field holds a `char *`; compare the pointed-to string contents
    /// (the pointer values themselves are allocator-dependent).
    PtrStr,
}

#[derive(Clone, Copy, Debug)]
pub struct Spec {
    pub elemsize: usize,
    pub keysize: usize,
    pub key_kind: KeyKind,
}

impl Spec {
    pub const fn bytes(elemsize: usize, keysize: usize) -> Spec {
        Spec {
            elemsize,
            keysize,
            key_kind: KeyKind::Bytes,
        }
    }
    pub const fn ptr(elemsize: usize) -> Spec {
        Spec {
            elemsize,
            keysize: core::mem::size_of::<usize>(),
            key_kind: KeyKind::PtrStr,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum KeySnap {
    Raw(Vec<u8>),
    Str(Option<Vec<u8>>),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ElemSnap {
    pub key: KeySnap,
    pub rest: Vec<u8>,
}

/// Snapshot `length` elements starting at the array base `a`.
pub unsafe fn snap_elems(a: *mut c_void, length: usize, spec: Spec) -> Vec<ElemSnap> {
    let mut out = Vec::with_capacity(length);
    if a.is_null() {
        return out;
    }
    for i in 0..length {
        let e = (a as *mut u8).add(spec.elemsize * i);
        let key = match spec.key_kind {
            KeyKind::Bytes => {
                KeySnap::Raw(core::slice::from_raw_parts(e, spec.keysize).to_vec())
            }
            KeyKind::PtrStr => {
                let p = *(e as *const *const c_char);
                KeySnap::Str(if p.is_null() {
                    None
                } else {
                    Some(read_cstr(p))
                })
            }
        };
        let rest_off = spec.keysize;
        let rest = if spec.elemsize > rest_off {
            core::slice::from_raw_parts(e.add(rest_off), spec.elemsize - rest_off).to_vec()
        } else {
            Vec::new()
        };
        out.push(ElemSnap { key, rest });
    }
    out
}

/// Everything observable about a live hash-map / array, in one comparable value.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FullSnap {
    pub hdr: HdrSnap,
    pub idx: IdxSnap,
    pub elems: Vec<ElemSnap>,
}

/// `a` is the *array* pointer.
pub unsafe fn snap_full(a: *mut c_void, spec: Spec) -> FullSnap {
    let hdr = snap_hdr(a);
    FullSnap {
        idx: snap_idx(a),
        elems: snap_elems(a, hdr.length, spec),
        hdr,
    }
}

// ---------------------------------------------------------------------------
// Macro protocol drivers (reproduce the header macros of `lib.c` exactly)
// ---------------------------------------------------------------------------

/// A live hash map, driven through one library's exported functions.
/// `t` is the "hash pointer" the `hm*`/`sh*` macros keep (element index 1).
pub struct Map<'a> {
    pub lib: &'a Lib,
    pub t: *mut c_void,
    pub spec: Spec,
    pub mode: c_int,
}

impl<'a> Map<'a> {
    pub fn new(lib: &'a Lib, spec: Spec, mode: c_int) -> Map<'a> {
        Map {
            lib,
            t: std::ptr::null_mut(),
            spec,
            mode,
        }
    }

    /// `sh_new_arena(t)` / `sh_new_strdup(t)` / any explicit `shmode_func`.
    pub fn new_shmode(lib: &'a Lib, spec: Spec, mode: c_int, sh_mode: c_int) -> Map<'a> {
        let t = unsafe { (lib.shmode_func)(spec.elemsize, sh_mode) };
        Map {
            lib,
            t,
            spec,
            mode,
        }
    }

    /// The array pointer, i.e. `(t) - 1`.
    pub fn raw(&self) -> *mut c_void {
        if self.t.is_null() {
            std::ptr::null_mut()
        } else {
            (self.t as *mut u8).wrapping_sub(self.spec.elemsize) as *mut c_void
        }
    }

    /// `stbds_temp((t)-1)`
    pub fn temp(&self) -> isize {
        unsafe { (*hdr_of(self.raw())).temp }
    }

    /// `stbds_hmlen(t)`
    pub fn hmlen(&self) -> isize {
        if self.t.is_null() {
            0
        } else {
            unsafe { (*hdr_of(self.raw())).length as isize - 1 }
        }
    }

    fn elem(&self, index: isize) -> *mut u8 {
        (self.t as *mut u8).wrapping_add(self.spec.elemsize.wrapping_mul(index as usize))
    }

    /// `stbds_hmput(t, k, v)` — BINARY protocol: the library memcpy's the key,
    /// then the macro re-assigns `.key` and `.value`.
    pub fn hmput(&mut self, key: &[u8], value: &[u8]) -> isize {
        assert_eq!(key.len(), self.spec.keysize);
        unsafe {
            self.t = (self.lib.hmput_key)(
                self.t,
                self.spec.elemsize,
                key.as_ptr() as *mut c_void,
                self.spec.keysize,
                self.mode,
            );
            let temp = self.temp();
            let e = self.elem(temp);
            core::ptr::copy_nonoverlapping(key.as_ptr(), e, self.spec.keysize);
            let n = value.len().min(self.spec.elemsize - self.spec.keysize);
            if n != 0 {
                core::ptr::copy_nonoverlapping(value.as_ptr(), e.add(self.spec.keysize), n);
            }
            temp
        }
    }

    /// `stbds_shput(t, k, v)` — STRING protocol: the library owns the key slot,
    /// the macro only writes `.value`.  `key` must be a live NUL-terminated
    /// buffer owned by the caller.
    pub fn shput(&mut self, key: *mut c_char, value: &[u8]) -> isize {
        unsafe {
            self.t = (self.lib.hmput_key)(
                self.t,
                self.spec.elemsize,
                key as *mut c_void,
                self.spec.keysize,
                self.mode,
            );
            let temp = self.temp();
            let e = self.elem(temp);
            let n = value.len().min(self.spec.elemsize - self.spec.keysize);
            if n != 0 {
                core::ptr::copy_nonoverlapping(value.as_ptr(), e.add(self.spec.keysize), n);
            }
            temp
        }
    }

    /// `stbds_shputs(t, s)`: copy the whole struct, then re-point `.key` at
    /// `stbds_temp_key((t)-1)`.
    pub fn shputs(&mut self, key: *mut c_char, whole_elem: &[u8]) -> isize {
        assert_eq!(whole_elem.len(), self.spec.elemsize);
        unsafe {
            self.t = (self.lib.hmput_key)(
                self.t,
                self.spec.elemsize,
                key as *mut c_void,
                self.spec.keysize,
                self.mode,
            );
            let temp = self.temp();
            let e = self.elem(temp);
            core::ptr::copy_nonoverlapping(whole_elem.as_ptr(), e, self.spec.elemsize);
            // (t)[temp].key = stbds_temp_key((t)-1)
            let tk = *((*hdr_of(self.raw())).hash_table as *mut *mut c_char);
            *(e as *mut *mut c_char) = tk;
            temp
        }
    }

    /// `stbds_hmgeti(t, k)` (BINARY) — uses `stbds_hmget_key`.
    pub fn hmgeti(&mut self, key: &[u8]) -> isize {
        unsafe {
            self.t = (self.lib.hmget_key)(
                self.t,
                self.spec.elemsize,
                key.as_ptr() as *mut c_void,
                self.spec.keysize,
                self.mode,
            );
            self.temp()
        }
    }

    /// `stbds_shgeti(t, k)` (STRING) — uses `stbds_hmget_key`.
    pub fn shgeti(&mut self, key: *mut c_char) -> isize {
        unsafe {
            self.t = (self.lib.hmget_key)(
                self.t,
                self.spec.elemsize,
                key as *mut c_void,
                self.spec.keysize,
                self.mode,
            );
            self.temp()
        }
    }

    /// `stbds_hmgeti_ts(t, k, temp)` — uses `stbds_hmget_key_ts`.
    pub fn hmgeti_ts(&mut self, key: *mut c_void) -> isize {
        unsafe {
            let mut temp: isize = 0x5A5A_5A5A;
            self.t = (self.lib.hmget_key_ts)(
                self.t,
                self.spec.elemsize,
                key,
                self.spec.keysize,
                &mut temp,
                self.mode,
            );
            temp
        }
    }

    /// `stbds_hmdel(t, k)` / `stbds_shdel(t, k)`.
    pub fn hmdel(&mut self, key: *mut c_void, keyoffset: usize) -> isize {
        unsafe {
            self.t = (self.lib.hmdel_key)(
                self.t,
                self.spec.elemsize,
                key,
                self.spec.keysize,
                keyoffset,
                self.mode,
            );
            if self.t.is_null() {
                0
            } else {
                self.temp()
            }
        }
    }

    /// `stbds_hmdefault(t, v)`: `(t)[-1].value = v`.
    pub fn hmdefault(&mut self, value: &[u8]) {
        unsafe {
            self.t = (self.lib.hmput_default)(self.t, self.spec.elemsize);
            let e = self.elem(-1);
            let n = value.len().min(self.spec.elemsize - self.spec.keysize);
            if n != 0 {
                core::ptr::copy_nonoverlapping(value.as_ptr(), e.add(self.spec.keysize), n);
            }
        }
    }

    /// `stbds_hmdefaults(t, s)`: `(t)[-1] = s`.
    pub fn hmdefaults(&mut self, whole_elem: &[u8]) {
        assert_eq!(whole_elem.len(), self.spec.elemsize);
        unsafe {
            self.t = (self.lib.hmput_default)(self.t, self.spec.elemsize);
            core::ptr::copy_nonoverlapping(whole_elem.as_ptr(), self.elem(-1), self.spec.elemsize);
        }
    }

    /// `stbds_hmfree(t)`
    pub fn hmfree(&mut self) {
        unsafe {
            if !self.t.is_null() {
                (self.lib.hmfree_func)(self.raw(), self.spec.elemsize);
            }
            self.t = std::ptr::null_mut();
        }
    }

    pub fn snap(&self) -> FullSnap {
        unsafe { snap_full(self.raw(), self.spec) }
    }

    pub fn snap_temp_key(&self) -> Option<Vec<u8>> {
        unsafe { snap_temp_key(self.raw()) }
    }
}

/// A live plain dynamic array driven only through `stbds_arrgrowf`/`stbds_arrfreef`.
pub struct Arr<'a> {
    pub lib: &'a Lib,
    pub a: *mut c_void,
    pub elemsize: usize,
}

impl<'a> Arr<'a> {
    pub fn new(lib: &'a Lib, elemsize: usize) -> Arr<'a> {
        Arr {
            lib,
            a: std::ptr::null_mut(),
            elemsize,
        }
    }

    pub fn len(&self) -> isize {
        if self.a.is_null() {
            0
        } else {
            unsafe { (*hdr_of(self.a)).length as isize }
        }
    }
    pub fn cap(&self) -> usize {
        if self.a.is_null() {
            0
        } else {
            unsafe { (*hdr_of(self.a)).capacity }
        }
    }

    /// `stbds_arrgrow(a,b,c)`
    pub fn grow(&mut self, addlen: usize, min_cap: usize) {
        unsafe { self.a = (self.lib.arrgrowf)(self.a, self.elemsize, addlen, min_cap) }
    }

    /// `stbds_arrmaybegrow(a,n)`
    pub fn maybe_grow(&mut self, n: usize) {
        unsafe {
            if self.a.is_null()
                || (*hdr_of(self.a)).length.wrapping_add(n) > (*hdr_of(self.a)).capacity
            {
                self.grow(n, 0);
            }
        }
    }

    /// `stbds_arrput(a, v)`
    pub fn put(&mut self, v: &[u8]) {
        assert_eq!(v.len(), self.elemsize);
        self.maybe_grow(1);
        unsafe {
            let h = hdr_of(self.a);
            let idx = (*h).length;
            (*h).length = idx + 1;
            core::ptr::copy_nonoverlapping(
                v.as_ptr(),
                (self.a as *mut u8).add(idx * self.elemsize),
                self.elemsize,
            );
        }
    }

    /// `stbds_arrpop(a)`
    pub fn pop(&mut self) -> Vec<u8> {
        unsafe {
            let h = hdr_of(self.a);
            (*h).length -= 1;
            let idx = (*h).length;
            core::slice::from_raw_parts(
                (self.a as *const u8).add(idx * self.elemsize),
                self.elemsize,
            )
            .to_vec()
        }
    }

    /// `stbds_arrsetcap(a,n)`
    pub fn setcap(&mut self, n: usize) {
        self.grow(0, n);
    }

    /// `stbds_arrsetlen(a,n)`
    pub fn setlen(&mut self, n: usize) {
        if self.cap() < n {
            self.setcap(n);
        }
        if !self.a.is_null() {
            unsafe { (*hdr_of(self.a)).length = n }
        }
    }

    /// `stbds_arraddnindex(a,n)`
    pub fn addn_index(&mut self, n: usize) -> isize {
        self.maybe_grow(n);
        unsafe {
            if n != 0 {
                let h = hdr_of(self.a);
                (*h).length += n;
                ((*h).length - n) as isize
            } else {
                self.len()
            }
        }
    }

    /// `stbds_arrdeln(a,i,n)`
    pub fn deln(&mut self, i: usize, n: usize) {
        unsafe {
            let h = hdr_of(self.a);
            let cnt = (*h).length - n - i;
            let base = self.a as *mut u8;
            core::ptr::copy(
                base.add((i + n) * self.elemsize),
                base.add(i * self.elemsize),
                cnt * self.elemsize,
            );
            (*h).length -= n;
        }
    }

    /// `stbds_arrdelswap(a,i)`
    pub fn delswap(&mut self, i: usize) {
        unsafe {
            let h = hdr_of(self.a);
            let last = (*h).length - 1;
            let base = self.a as *mut u8;
            core::ptr::copy_nonoverlapping(
                base.add(last * self.elemsize),
                base.add(i * self.elemsize),
                self.elemsize,
            );
            (*h).length -= 1;
        }
    }

    /// `stbds_arrinsn(a,i,n)`
    pub fn insn(&mut self, i: usize, n: usize) {
        self.addn_index(n);
        unsafe {
            let h = hdr_of(self.a);
            let cnt = (*h).length - n - i;
            let base = self.a as *mut u8;
            core::ptr::copy(
                base.add(i * self.elemsize),
                base.add((i + n) * self.elemsize),
                cnt * self.elemsize,
            );
        }
    }

    /// `stbds_arrins(a,i,v)`
    pub fn ins(&mut self, i: usize, v: &[u8]) {
        self.insn(i, 1);
        unsafe {
            core::ptr::copy_nonoverlapping(
                v.as_ptr(),
                (self.a as *mut u8).add(i * self.elemsize),
                self.elemsize,
            );
        }
    }

    /// `stbds_arrfree(a)`
    pub fn free(&mut self) {
        unsafe {
            if !self.a.is_null() {
                (self.lib.arrfreef)(self.a);
            }
            self.a = std::ptr::null_mut();
        }
    }

    pub fn bytes(&self) -> Vec<u8> {
        if self.a.is_null() {
            return Vec::new();
        }
        unsafe {
            core::slice::from_raw_parts(
                self.a as *const u8,
                (*hdr_of(self.a)).length * self.elemsize,
            )
            .to_vec()
        }
    }

    pub fn snap(&self) -> (HdrSnap, Vec<u8>) {
        (unsafe { snap_hdr(self.a) }, self.bytes())
    }
}

// ---------------------------------------------------------------------------
// C-string scratch buffers (stable addresses shared by both libraries)
// ---------------------------------------------------------------------------

/// Owns NUL-terminated key buffers so that both libraries receive the *same*
/// key pointer (important for `STBDS_SH_DEFAULT`, which stores the pointer).
pub struct Keys {
    bufs: Vec<Box<[u8]>>,
}

impl Keys {
    pub fn new() -> Keys {
        Keys { bufs: Vec::new() }
    }
    pub fn add(&mut self, body: &[u8]) -> *mut c_char {
        let mut v = body.to_vec();
        v.push(0);
        let b: Box<[u8]> = v.into_boxed_slice();
        self.bufs.push(b);
        self.bufs.last_mut().unwrap().as_mut_ptr() as *mut c_char
    }
    pub fn add_str(&mut self, s: &str) -> *mut c_char {
        self.add(s.as_bytes())
    }
    pub fn len(&self) -> usize {
        self.bufs.len()
    }
}

// ---------------------------------------------------------------------------
// Assertion helpers
// ---------------------------------------------------------------------------

#[macro_export]
macro_rules! diff_eq {
    ($c:expr, $r:expr, $($ctx:tt)*) => {{
        let cv = $c;
        let rv = $r;
        if cv != rv {
            panic!(
                "C/Rust divergence [{}]\n  C    = {:?}\n  RUST = {:?}",
                format!($($ctx)*), cv, rv
            );
        }
    }};
}
