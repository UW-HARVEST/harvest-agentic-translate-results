//! Shared differential-test harness.
//!
//! Both the C `.so` and the Rust `.so` are loaded with `libloading`; every call
//! goes through the dynamic-symbol table, exactly as an external C consumer
//! would.  Rust functions are never called directly.

#![allow(dead_code)]

use libloading::Library;
use std::ffi::c_void;
use std::os::raw::{c_char, c_int};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard, OnceLock};

// ---------------------------------------------------------------------------
// FFI signatures
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
pub type FnStralloc = unsafe extern "C" fn(*mut c_void, *mut c_char) -> *mut c_char;
pub type FnStrreset = unsafe extern "C" fn(*mut c_void);
pub type FnStrkey = unsafe extern "C" fn(c_int) -> *mut c_char;
pub type FnIntput = unsafe extern "C" fn(c_int);

pub struct Api {
    pub name: &'static str,
    pub path: PathBuf,
    _lib: Library,
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
    pub intput: FnIntput,
}

unsafe impl Send for Api {}
unsafe impl Sync for Api {}

macro_rules! sym {
    ($lib:expr, $t:ty, $n:expr) => {{
        let s: libloading::Symbol<$t> = $lib
            .get($n)
            .unwrap_or_else(|e| panic!("missing symbol {:?}: {e}", $n));
        *s
    }};
}

impl Api {
    pub fn load(name: &'static str, path: &Path) -> Api {
        unsafe {
            let lib = Library::new(path)
                .unwrap_or_else(|e| panic!("dlopen {} failed: {e}", path.display()));
            let api = Api {
                name,
                path: path.to_path_buf(),
                arrgrowf: sym!(lib, FnArrgrowf, b"stbds_arrgrowf\0"),
                arrfreef: sym!(lib, FnArrfreef, b"stbds_arrfreef\0"),
                rand_seed: sym!(lib, FnRandSeed, b"stbds_rand_seed\0"),
                hash_string: sym!(lib, FnHashString, b"stbds_hash_string\0"),
                hash_bytes: sym!(lib, FnHashBytes, b"stbds_hash_bytes\0"),
                hmfree_func: sym!(lib, FnHmfreeFunc, b"stbds_hmfree_func\0"),
                hmget_key_ts: sym!(lib, FnHmgetKeyTs, b"stbds_hmget_key_ts\0"),
                hmget_key: sym!(lib, FnHmgetKey, b"stbds_hmget_key\0"),
                hmput_default: sym!(lib, FnHmputDefault, b"stbds_hmput_default\0"),
                hmput_key: sym!(lib, FnHmputKey, b"stbds_hmput_key\0"),
                shmode_func: sym!(lib, FnShmodeFunc, b"stbds_shmode_func\0"),
                hmdel_key: sym!(lib, FnHmdelKey, b"stbds_hmdel_key\0"),
                stralloc: sym!(lib, FnStralloc, b"stbds_stralloc\0"),
                strreset: sym!(lib, FnStrreset, b"stbds_strreset\0"),
                strkey: sym!(lib, FnStrkey, b"strkey\0"),
                intput: sym!(lib, FnIntput, b"intput\0"),
                _lib: lib,
            };
            api
        }
    }
}

// ---------------------------------------------------------------------------
// library discovery
// ---------------------------------------------------------------------------

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

pub fn c_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("C_DS_SO") {
        return PathBuf::from(p);
    }
    let build = manifest_dir().join("../c_src/build");
    let mut best: Option<PathBuf> = None;
    if let Ok(rd) = std::fs::read_dir(&build) {
        for e in rd.flatten() {
            let p = e.path();
            if p.extension().and_then(|s| s.to_str()) == Some("so") {
                best = Some(p);
                break;
            }
        }
    }
    best.unwrap_or_else(|| {
        panic!(
            "no C .so found in {} — build it with cmake first",
            build.display()
        )
    })
}

pub fn rust_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("RUST_DS_SO") {
        return PathBuf::from(p);
    }
    // Prefer the profile the test binary itself was built with.
    let exe = std::env::current_exe().expect("current_exe");
    // .../target/<profile>/deps/<test>-hash
    let mut d = exe.clone();
    d.pop(); // deps
    d.pop(); // profile
    let cand = d.join("libintput_lib.so");
    if cand.exists() {
        return cand;
    }
    for prof in ["debug", "release"] {
        let c = manifest_dir().join("target").join(prof).join("libintput_lib.so");
        if c.exists() {
            return c;
        }
    }
    panic!("no Rust libintput_lib.so found (build with cargo build)");
}

struct Pair2 {
    c: Api,
    r: Api,
}

static LIBS: OnceLock<Pair2> = OnceLock::new();
static LOCK: Mutex<()> = Mutex::new(());

fn libs() -> &'static Pair2 {
    LIBS.get_or_init(|| Pair2 {
        c: Api::load("C", &c_so_path()),
        r: Api::load("RUST", &rust_so_path()),
    })
}

/// Every test body runs under a process-wide lock, because both libraries own
/// mutable globals (`stbds_hash_seed`, `buffer`) that are shared by all callers
/// of the same `.so` within a process.  The seed is reset first so each test is
/// deterministic and reproducible.
pub fn with_libs<R>(seed: usize, f: impl FnOnce(&'static Api, &'static Api) -> R) -> R {
    let _g: MutexGuard<'_, ()> = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let l = libs();
    unsafe {
        (l.c.rand_seed)(seed);
        (l.r.rand_seed)(seed);
    }
    f(&l.c, &l.r)
}

pub const DEFAULT_SEED: usize = 0x3141_5926;

// ---------------------------------------------------------------------------
// deterministic RNG (xoshiro-ish, fixed seed per test)
// ---------------------------------------------------------------------------

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng(seed ^ 0x9E3779B97F4A7C15)
    }
    pub fn next_u64(&mut self) -> u64 {
        // splitmix64
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
    pub fn range(&mut self, lo: usize, hi_incl: usize) -> usize {
        lo + self.below(hi_incl - lo + 1)
    }
    pub fn i32(&mut self) -> i32 {
        self.next_u32() as i32
    }
    pub fn bytes(&mut self, n: usize) -> Vec<u8> {
        (0..n).map(|_| (self.next_u32() & 0xff) as u8).collect()
    }
    pub fn ascii(&mut self, n: usize) -> Vec<u8> {
        (0..n)
            .map(|_| b'a' + (self.next_u32() % 26) as u8)
            .collect()
    }
    /// non-zero bytes only (valid C string body)
    pub fn cstr_bytes(&mut self, n: usize) -> Vec<u8> {
        (0..n)
            .map(|_| {
                let b = (self.next_u32() & 0xff) as u8;
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
// C struct mirrors (for reading state out of both libraries)
// ---------------------------------------------------------------------------

pub const HDR_SIZE: usize = 32; // length, capacity, hash_table, temp
pub const BUCKET_LENGTH: usize = 8;
pub const BUCKET_SHIFT: usize = 3;
pub const BUCKET_MASK: usize = 7;

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ArrHeader {
    pub length: usize,
    pub capacity: usize,
    pub hash_table: *mut c_void,
    pub temp: isize,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct StringArena {
    pub storage: *mut c_void,
    pub remaining: usize,
    pub block: u8,
    pub mode: u8,
}

impl StringArena {
    pub fn zeroed() -> StringArena {
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
    pub storage: *mut c_void,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct HashBucket {
    pub hash: [usize; 8],
    pub index: [isize; 8],
}

// Sanity: layout must match the C definitions.
const _: () = assert!(std::mem::size_of::<ArrHeader>() == 32);
const _: () = assert!(std::mem::size_of::<StringArena>() == 24);
const _: () = assert!(std::mem::size_of::<HashIndex>() == 104);
const _: () = assert!(std::mem::size_of::<HashBucket>() == 128);

// ---------------------------------------------------------------------------
// Snapshots (structural, pointer-independent)
// ---------------------------------------------------------------------------

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
    pub arena_storage_null: bool,
    pub temp_key: Option<Vec<u8>>,
    pub hashes: Vec<usize>,
    pub indices: Vec<isize>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MapSnap {
    pub t_null: bool,
    pub length: usize,
    pub capacity: usize,
    pub temp: isize,
    pub table: Option<TableSnap>,
    /// element bytes (index 0 == the `t[-1]` default slot); key field blanked
    /// out when the key is a `char *`
    pub elems: Vec<Vec<u8>>,
    /// key string contents when the key is a `char *`
    pub keys: Vec<Option<Vec<u8>>>,
}

pub unsafe fn read_cstr(p: *const c_char) -> Vec<u8> {
    let mut v = Vec::new();
    let mut q = p as *const u8;
    while *q != 0 {
        v.push(*q);
        q = q.add(1);
    }
    v
}

// ---------------------------------------------------------------------------
// Map driver — reimplements the stb_ds macros over the exported functions
// ---------------------------------------------------------------------------

pub const HM_BINARY: c_int = 0;
pub const HM_STRING: c_int = 1;
pub const SH_NONE: c_int = 0;
pub const SH_DEFAULT: c_int = 1;
pub const SH_STRDUP: c_int = 2;
pub const SH_ARENA: c_int = 3;

#[derive(Clone, Copy, Debug)]
pub struct Shape {
    pub elemsize: usize,
    pub keysize: usize,
    pub keyoffset: usize,
    /// key field is a `char *` (string modes) → compare contents, not bytes
    pub string_key: bool,
    /// `stbds_hash_index::temp_key` is only ever written by the string branches
    /// of `stbds_hmput_key`; `stbds_make_hash_index` leaves it *uninitialised*
    /// (see lib.c:388-413 — every field but `temp_key` is assigned).  Reading it
    /// otherwise compares heap garbage, so it is opt-in.
    pub cmp_temp_key: bool,
}

impl Shape {
    pub fn binary(elemsize: usize, keysize: usize) -> Shape {
        Shape {
            elemsize,
            keysize,
            keyoffset: 0,
            string_key: false,
            cmp_temp_key: false,
        }
    }
    /// `cmp_temp_key` is deliberately `false`: `stbds_make_hash_index` never
    /// initialises `temp_key`, and every grow / shrink / rebuild installs a
    /// brand-new (uninitialised) `stbds_hash_index`.  `temp_key` is therefore
    /// only well defined *immediately* after a string-mode `hmput_key`, which is
    /// checked explicitly via `Pair::assert_temp_key`.
    pub fn string(elemsize: usize) -> Shape {
        Shape {
            elemsize,
            keysize: 8,
            keyoffset: 0,
            string_key: true,
            cmp_temp_key: false,
        }
    }
    pub fn with_keyoffset(mut self, k: usize) -> Shape {
        self.keyoffset = k;
        self
    }
    pub fn with_keysize(mut self, k: usize) -> Shape {
        self.keysize = k;
        self
    }
    pub fn raw_keys(mut self) -> Shape {
        self.string_key = false;
        self.cmp_temp_key = false;
        self
    }
}

pub struct Map {
    pub api: &'static Api,
    pub t: *mut u8,
    pub shape: Shape,
}

impl Map {
    pub fn new(api: &'static Api, shape: Shape) -> Map {
        Map {
            api,
            t: std::ptr::null_mut(),
            shape,
        }
    }

    pub fn raw(&self) -> *mut u8 {
        self.t.wrapping_sub(self.shape.elemsize)
    }
    pub unsafe fn header(&self) -> *mut ArrHeader {
        self.raw().wrapping_sub(HDR_SIZE) as *mut ArrHeader
    }
    pub unsafe fn temp(&self) -> isize {
        (*self.header()).temp
    }
    pub unsafe fn table(&self) -> *mut HashIndex {
        (*self.header()).hash_table as *mut HashIndex
    }
    /// `(t)[j]` — j may be -1 (the default element)
    pub fn elem(&self, j: isize) -> *mut u8 {
        (self.t as isize).wrapping_add((self.shape.elemsize as isize).wrapping_mul(j)) as *mut u8
    }
    /// `stbds_hmlen(t)`
    pub unsafe fn hmlen(&self) -> isize {
        if self.t.is_null() {
            0
        } else {
            (*self.header()).length as isize - 1
        }
    }
    /// `stbds_temp_key(t-1)` == `*(char **) header->hash_table`
    pub unsafe fn temp_key(&self) -> *mut c_char {
        let ht = (*self.header()).hash_table as *mut *mut c_char;
        if ht.is_null() {
            std::ptr::null_mut()
        } else {
            *ht
        }
    }

    // --- raw entry points ------------------------------------------------

    /// `stbds_hmput_key` + the caller-side element write (`hmputs` semantics)
    pub unsafe fn put_struct(&mut self, key: *mut c_void, elem: &[u8], mode: c_int) -> isize {
        assert_eq!(elem.len(), self.shape.elemsize);
        self.t = (self.api.hmput_key)(
            self.t as *mut c_void,
            self.shape.elemsize,
            key,
            self.shape.keysize,
            mode,
        ) as *mut u8;
        let i = self.temp();
        std::ptr::copy_nonoverlapping(elem.as_ptr(), self.elem(i), self.shape.elemsize);
        i
    }

    /// `stbds_shput` — key already written by `hmput_key`, only the payload
    /// after the key pointer is written by the macro.
    pub unsafe fn shput_value(
        &mut self,
        key: *mut c_char,
        value_off: usize,
        value: &[u8],
        mode: c_int,
    ) -> isize {
        self.t = (self.api.hmput_key)(
            self.t as *mut c_void,
            self.shape.elemsize,
            key as *mut c_void,
            self.shape.keysize,
            mode,
        ) as *mut u8;
        let i = self.temp();
        std::ptr::copy_nonoverlapping(
            value.as_ptr(),
            self.elem(i).wrapping_add(value_off),
            value.len(),
        );
        i
    }

    /// `stbds_shputs` — whole struct then `.key = stbds_temp_key(t-1)`
    pub unsafe fn shputs(&mut self, key: *mut c_char, elem: &[u8], mode: c_int) -> isize {
        assert_eq!(elem.len(), self.shape.elemsize);
        self.t = (self.api.hmput_key)(
            self.t as *mut c_void,
            self.shape.elemsize,
            key as *mut c_void,
            self.shape.keysize,
            mode,
        ) as *mut u8;
        let i = self.temp();
        let tk = self.temp_key();
        std::ptr::copy_nonoverlapping(elem.as_ptr(), self.elem(i), self.shape.elemsize);
        std::ptr::write_unaligned(
            self.elem(i).wrapping_add(self.shape.keyoffset) as *mut *mut c_char,
            tk,
        );
        i
    }

    /// `stbds_hmgeti` / `stbds_shgeti`
    pub unsafe fn geti(&mut self, key: *mut c_void, mode: c_int) -> isize {
        self.t = (self.api.hmget_key)(
            self.t as *mut c_void,
            self.shape.elemsize,
            key,
            self.shape.keysize,
            mode,
        ) as *mut u8;
        self.temp()
    }

    /// `stbds_hmgeti_ts` → (`*temp`, `header->temp`)
    pub unsafe fn geti_ts(&mut self, key: *mut c_void, mode: c_int) -> (isize, isize) {
        let mut out: isize = 0x5A5A_5A5A;
        self.t = (self.api.hmget_key_ts)(
            self.t as *mut c_void,
            self.shape.elemsize,
            key,
            self.shape.keysize,
            &mut out,
            mode,
        ) as *mut u8;
        (out, self.temp())
    }

    /// `stbds_hmdel` / `stbds_shdel`
    pub unsafe fn del(&mut self, key: *mut c_void, mode: c_int) -> isize {
        self.t = (self.api.hmdel_key)(
            self.t as *mut c_void,
            self.shape.elemsize,
            key,
            self.shape.keysize,
            self.shape.keyoffset,
            mode,
        ) as *mut u8;
        if self.t.is_null() {
            0
        } else {
            self.temp()
        }
    }

    /// `stbds_hmdefaults` (write the whole default element)
    pub unsafe fn defaults(&mut self, elem: &[u8]) {
        assert_eq!(elem.len(), self.shape.elemsize);
        self.t =
            (self.api.hmput_default)(self.t as *mut c_void, self.shape.elemsize) as *mut u8;
        std::ptr::copy_nonoverlapping(elem.as_ptr(), self.elem(-1), self.shape.elemsize);
    }

    /// `stbds_hmput_default` alone
    pub unsafe fn put_default(&mut self) {
        self.t =
            (self.api.hmput_default)(self.t as *mut c_void, self.shape.elemsize) as *mut u8;
    }

    /// `stbds_sh_new_strdup` / `stbds_sh_new_arena`
    pub unsafe fn shmode(&mut self, mode: c_int) {
        self.t = (self.api.shmode_func)(self.shape.elemsize, mode) as *mut u8;
    }

    /// `stbds_hmfree`
    pub unsafe fn free(&mut self) {
        if !self.t.is_null() {
            (self.api.hmfree_func)(self.raw() as *mut c_void, self.shape.elemsize);
        }
        self.t = std::ptr::null_mut();
    }

    // --- snapshot --------------------------------------------------------

    pub unsafe fn snapshot(&self) -> MapSnap {
        if self.t.is_null() {
            return MapSnap {
                t_null: true,
                length: 0,
                capacity: 0,
                temp: 0,
                table: None,
                elems: Vec::new(),
                keys: Vec::new(),
            };
        }
        let h = *self.header();
        let mut elems = Vec::new();
        let mut keys = Vec::new();
        for i in 0..h.length {
            let e = self.raw().wrapping_add(self.shape.elemsize * i);
            let mut bytes = std::slice::from_raw_parts(e, self.shape.elemsize).to_vec();
            if self.shape.string_key {
                let kp = std::ptr::read_unaligned(
                    e.wrapping_add(self.shape.keyoffset) as *const *mut c_char,
                );
                keys.push(if kp.is_null() {
                    None
                } else {
                    Some(read_cstr(kp))
                });
                for b in bytes
                    [self.shape.keyoffset..self.shape.keyoffset + 8]
                    .iter_mut()
                {
                    *b = 0;
                }
            } else {
                keys.push(None);
            }
            elems.push(bytes);
        }

        let tp = self.table();
        let table = if tp.is_null() {
            None
        } else {
            let ti = *tp;
            let nbuckets = ti.slot_count >> BUCKET_SHIFT;
            let mut hashes = Vec::with_capacity(ti.slot_count);
            let mut indices = Vec::with_capacity(ti.slot_count);
            for b in 0..nbuckets {
                let bk = (ti.storage as *const HashBucket).add(b);
                for s in 0..BUCKET_LENGTH {
                    hashes.push((*bk).hash[s]);
                    indices.push((*bk).index[s]);
                }
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
                arena_storage_null: ti.string.storage.is_null(),
                temp_key: if !self.shape.cmp_temp_key || ti.temp_key.is_null() {
                    None
                } else {
                    Some(read_cstr(ti.temp_key))
                },
                hashes,
                indices,
            })
        };

        MapSnap {
            t_null: false,
            length: h.length,
            capacity: h.capacity,
            temp: h.temp,
            table,
            elems,
            keys,
        }
    }
}

// ---------------------------------------------------------------------------
// Paired driver
// ---------------------------------------------------------------------------

pub struct Pair {
    pub c: Map,
    pub r: Map,
}

impl Pair {
    pub fn new(c: &'static Api, r: &'static Api, shape: Shape) -> Pair {
        Pair {
            c: Map::new(c, shape),
            r: Map::new(r, shape),
        }
    }

    pub fn assert_same(&self, ctx: &str) {
        unsafe {
            let a = self.c.snapshot();
            let b = self.r.snapshot();
            if a != b {
                panic!("{ctx}: state diverged\n C   = {a:#?}\n RUST= {b:#?}");
            }
        }
    }

    pub unsafe fn put_struct(&mut self, key: &[u8], elem: &[u8], mode: c_int, ctx: &str) -> isize {
        let mut kc = key.to_vec();
        let mut kr = key.to_vec();
        let ic = self
            .c
            .put_struct(kc.as_mut_ptr() as *mut c_void, elem, mode);
        let ir = self
            .r
            .put_struct(kr.as_mut_ptr() as *mut c_void, elem, mode);
        assert_eq!(ic, ir, "{ctx}: put index C={ic} RUST={ir}");
        self.assert_same(ctx);
        ic
    }

    pub unsafe fn geti(&mut self, key: &[u8], mode: c_int, ctx: &str) -> isize {
        let mut kc = key.to_vec();
        let mut kr = key.to_vec();
        let ic = self.c.geti(kc.as_mut_ptr() as *mut c_void, mode);
        let ir = self.r.geti(kr.as_mut_ptr() as *mut c_void, mode);
        assert_eq!(ic, ir, "{ctx}: geti C={ic} RUST={ir}");
        self.assert_same(ctx);
        ic
    }

    pub unsafe fn geti_ts(&mut self, key: &[u8], mode: c_int, ctx: &str) -> isize {
        let mut kc = key.to_vec();
        let mut kr = key.to_vec();
        let (oc, hc) = self.c.geti_ts(kc.as_mut_ptr() as *mut c_void, mode);
        let (or_, hr) = self.r.geti_ts(kr.as_mut_ptr() as *mut c_void, mode);
        assert_eq!(oc, or_, "{ctx}: geti_ts *temp C={oc} RUST={or_}");
        assert_eq!(hc, hr, "{ctx}: geti_ts header.temp C={hc} RUST={hr}");
        self.assert_same(ctx);
        oc
    }

    pub unsafe fn del(&mut self, key: &[u8], mode: c_int, ctx: &str) -> isize {
        let mut kc = key.to_vec();
        let mut kr = key.to_vec();
        let ic = self.c.del(kc.as_mut_ptr() as *mut c_void, mode);
        let ir = self.r.del(kr.as_mut_ptr() as *mut c_void, mode);
        assert_eq!(ic, ir, "{ctx}: del C={ic} RUST={ir}");
        self.assert_same(ctx);
        ic
    }

    pub unsafe fn defaults(&mut self, elem: &[u8], ctx: &str) {
        self.c.defaults(elem);
        self.r.defaults(elem);
        self.assert_same(ctx);
    }

    pub unsafe fn put_default(&mut self, ctx: &str) {
        self.c.put_default();
        self.r.put_default();
        self.assert_same(ctx);
    }

    pub unsafe fn shmode(&mut self, mode: c_int, ctx: &str) {
        self.c.shmode(mode);
        self.r.shmode(mode);
        self.assert_same(ctx);
    }

    /// string-mode put: the key pointer handed to both libraries is the *same*
    /// buffer (as a real C caller would); it must stay alive for the whole test.
    pub unsafe fn shput_value(
        &mut self,
        key: *mut c_char,
        value_off: usize,
        value: &[u8],
        mode: c_int,
        ctx: &str,
    ) -> isize {
        let ic = self.c.shput_value(key, value_off, value, mode);
        let ir = self.r.shput_value(key, value_off, value, mode);
        assert_eq!(ic, ir, "{ctx}: shput index C={ic} RUST={ir}");
        self.assert_same(ctx);
        ic
    }

    pub unsafe fn shputs(&mut self, key: *mut c_char, elem: &[u8], mode: c_int, ctx: &str) -> isize {
        let ic = self.c.shputs(key, elem, mode);
        let ir = self.r.shputs(key, elem, mode);
        assert_eq!(ic, ir, "{ctx}: shputs index C={ic} RUST={ir}");
        self.assert_same(ctx);
        ic
    }

    pub unsafe fn sgeti(&mut self, key: *mut c_char, mode: c_int, ctx: &str) -> isize {
        let ic = self.c.geti(key as *mut c_void, mode);
        let ir = self.r.geti(key as *mut c_void, mode);
        assert_eq!(ic, ir, "{ctx}: sgeti C={ic} RUST={ir}");
        self.assert_same(ctx);
        ic
    }

    pub unsafe fn sdel(&mut self, key: *mut c_char, mode: c_int, ctx: &str) -> isize {
        let ic = self.c.del(key as *mut c_void, mode);
        let ir = self.r.del(key as *mut c_void, mode);
        assert_eq!(ic, ir, "{ctx}: sdel C={ic} RUST={ir}");
        self.assert_same(ctx);
        ic
    }

    pub unsafe fn free(&mut self, ctx: &str) {
        self.c.free();
        self.r.free();
        self.assert_same(ctx);
    }

    /// `stbds_temp_key(t-1)` contents, valid only right after a string-mode
    /// `hmput_key` on a table whose `string.mode` is 1/2/3.
    pub unsafe fn assert_temp_key(&self, expect: &[u8], ctx: &str) {
        let tc = read_cstr(self.c.temp_key());
        let tr = read_cstr(self.r.temp_key());
        assert_eq!(tc, expect.to_vec(), "{ctx}: C temp_key");
        assert_eq!(tr, expect.to_vec(), "{ctx}: RUST temp_key");
    }

    /// Compare `temp_key` contents between the two libraries without asserting a
    /// particular value (used where the C code legitimately leaves a stale key).
    pub unsafe fn assert_temp_key_same(&self, ctx: &str) {
        let tc = read_cstr(self.c.temp_key());
        let tr = read_cstr(self.r.temp_key());
        assert_eq!(tc, tr, "{ctx}: temp_key differs");
    }
}

// ---------------------------------------------------------------------------
// raw-array (arrgrowf) paired driver
// ---------------------------------------------------------------------------

pub struct ArrPair {
    pub c: *mut u8,
    pub r: *mut u8,
    pub capi: &'static Api,
    pub rapi: &'static Api,
    pub elemsize: usize,
}

#[derive(Debug, PartialEq, Eq)]
pub struct ArrSnap {
    pub null: bool,
    pub length: usize,
    pub capacity: usize,
    pub temp: isize,
    pub hash_table_null: bool,
    pub payload: Vec<u8>,
}

impl ArrPair {
    pub fn new(capi: &'static Api, rapi: &'static Api, elemsize: usize) -> ArrPair {
        ArrPair {
            c: std::ptr::null_mut(),
            r: std::ptr::null_mut(),
            capi,
            rapi,
            elemsize,
        }
    }

    pub unsafe fn grow(&mut self, addlen: usize, min_cap: usize) -> (bool, bool) {
        let oc = self.c;
        let or_ = self.r;
        self.c = (self.capi.arrgrowf)(self.c as *mut c_void, self.elemsize, addlen, min_cap)
            as *mut u8;
        self.r = (self.rapi.arrgrowf)(self.r as *mut c_void, self.elemsize, addlen, min_cap)
            as *mut u8;
        (self.c == oc, self.r == or_)
    }

    unsafe fn snap_one(p: *mut u8, elemsize: usize) -> ArrSnap {
        if p.is_null() {
            return ArrSnap {
                null: true,
                length: 0,
                capacity: 0,
                temp: 0,
                hash_table_null: true,
                payload: Vec::new(),
            };
        }
        let h = *(p.wrapping_sub(HDR_SIZE) as *const ArrHeader);
        ArrSnap {
            null: false,
            length: h.length,
            capacity: h.capacity,
            temp: h.temp,
            hash_table_null: h.hash_table.is_null(),
            payload: std::slice::from_raw_parts(p, elemsize * h.length).to_vec(),
        }
    }

    pub unsafe fn assert_same(&self, ctx: &str) {
        let a = Self::snap_one(self.c, self.elemsize);
        let b = Self::snap_one(self.r, self.elemsize);
        if a != b {
            panic!("{ctx}: array state diverged\n C   = {a:#?}\n RUST= {b:#?}");
        }
    }

    pub unsafe fn set_length(&mut self, n: usize) {
        (*(self.c.wrapping_sub(HDR_SIZE) as *mut ArrHeader)).length = n;
        (*(self.r.wrapping_sub(HDR_SIZE) as *mut ArrHeader)).length = n;
    }

    pub unsafe fn length(&self) -> usize {
        (*(self.c.wrapping_sub(HDR_SIZE) as *const ArrHeader)).length
    }

    pub unsafe fn capacity(&self) -> usize {
        (*(self.c.wrapping_sub(HDR_SIZE) as *const ArrHeader)).capacity
    }

    /// `stbds_arrput(a, v)` — maybegrow then append
    pub unsafe fn put(&mut self, elem: &[u8]) {
        assert_eq!(elem.len(), self.elemsize);
        let len = if self.c.is_null() { 0 } else { self.length() };
        let cap = if self.c.is_null() { 0 } else { self.capacity() };
        if self.c.is_null() || len + 1 > cap {
            self.grow(1, 0);
        }
        let n = self.length();
        std::ptr::copy_nonoverlapping(
            elem.as_ptr(),
            self.c.wrapping_add(self.elemsize * n),
            self.elemsize,
        );
        std::ptr::copy_nonoverlapping(
            elem.as_ptr(),
            self.r.wrapping_add(self.elemsize * n),
            self.elemsize,
        );
        self.set_length(n + 1);
    }

    pub unsafe fn free(&mut self) {
        if !self.c.is_null() {
            (self.capi.arrfreef)(self.c as *mut c_void);
        }
        if !self.r.is_null() {
            (self.rapi.arrfreef)(self.r as *mut c_void);
        }
        self.c = std::ptr::null_mut();
        self.r = std::ptr::null_mut();
    }
}
