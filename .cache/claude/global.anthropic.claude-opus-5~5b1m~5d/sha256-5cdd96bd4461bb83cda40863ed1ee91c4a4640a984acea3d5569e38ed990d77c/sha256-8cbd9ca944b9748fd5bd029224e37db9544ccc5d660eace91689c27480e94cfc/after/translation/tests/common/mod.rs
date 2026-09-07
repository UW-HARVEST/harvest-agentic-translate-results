//! Shared harness for the C-vs-Rust differential tests.
//!
//! BOTH libraries are loaded as shared objects through `libloading`; no Rust
//! function is ever called directly, so the `#[no_mangle]` export wrappers are
//! part of what is under test.

#![allow(dead_code)]
#![allow(non_camel_case_types)]

use libloading::{Library, Symbol};
use std::ffi::{c_char, c_int, c_void};
use std::path::{Path, PathBuf};

// ---------------------------------------------------------------------------
// layout mirrors (identical in C and Rust)
// ---------------------------------------------------------------------------

pub const HEADER_SIZE: usize = 32;
pub const BUCKET_LENGTH: usize = 8;
pub const BUCKET_SHIFT: usize = 3;

pub const HM_BINARY: c_int = 0;
pub const HM_STRING: c_int = 1;
pub const HM_PTR_TO_STRING: c_int = 2;

pub const SH_NONE: c_int = 0;
pub const SH_DEFAULT: c_int = 1;
pub const SH_STRDUP: c_int = 2;
pub const SH_ARENA: c_int = 3;

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct ArrayHeader {
    pub length: usize,
    pub capacity: usize,
    pub hash_table: *mut c_void,
    pub temp: isize,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct StringArena {
    pub storage: *mut StringBlock,
    pub remaining: usize,
    pub block: u8,
    pub mode: u8,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct StringBlock {
    pub next: *mut StringBlock,
    pub storage: [c_char; 8],
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct HashBucket {
    pub hash: [usize; BUCKET_LENGTH],
    pub index: [isize; BUCKET_LENGTH],
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

impl Default for StringBlock {
    fn default() -> Self {
        StringBlock {
            next: std::ptr::null_mut(),
            storage: [0; 8],
        }
    }
}

// ---------------------------------------------------------------------------
// FFI signature aliases
// ---------------------------------------------------------------------------

type FnArrGrowf = unsafe extern "C" fn(*mut c_void, usize, usize, usize) -> *mut c_void;
type FnArrFreef = unsafe extern "C" fn(*mut c_void);
type FnRandSeed = unsafe extern "C" fn(usize);
type FnHashBytes = unsafe extern "C" fn(*mut c_void, usize, usize) -> usize;
type FnHashString = unsafe extern "C" fn(*mut c_char, usize) -> usize;
type FnHmFree = unsafe extern "C" fn(*mut c_void, usize);
type FnHmGetTs =
    unsafe extern "C" fn(*mut c_void, usize, *mut c_void, usize, *mut isize, c_int) -> *mut c_void;
type FnHmGet = unsafe extern "C" fn(*mut c_void, usize, *mut c_void, usize, c_int) -> *mut c_void;
type FnHmPutDefault = unsafe extern "C" fn(*mut c_void, usize) -> *mut c_void;
type FnHmPutKey = unsafe extern "C" fn(*mut c_void, usize, *mut c_void, usize, c_int) -> *mut c_void;
type FnShmode = unsafe extern "C" fn(usize, c_int) -> *mut c_void;
type FnHmDel =
    unsafe extern "C" fn(*mut c_void, usize, *mut c_void, usize, usize, c_int) -> *mut c_void;
type FnStrAlloc = unsafe extern "C" fn(*mut StringArena, *mut c_char) -> *mut c_char;
type FnStrReset = unsafe extern "C" fn(*mut StringArena);
type FnStrKey = unsafe extern "C" fn(c_int) -> *mut c_char;
type FnArrIns = unsafe extern "C" fn(c_int);

/// All 16 exported entry points of one shared object.
pub struct Api {
    pub name: &'static str,
    pub arrgrowf: FnArrGrowf,
    pub arrfreef: FnArrFreef,
    pub rand_seed: FnRandSeed,
    pub hash_bytes: FnHashBytes,
    pub hash_string: FnHashString,
    pub hmfree_func: FnHmFree,
    pub hmget_key_ts: FnHmGetTs,
    pub hmget_key: FnHmGet,
    pub hmput_default: FnHmPutDefault,
    pub hmput_key: FnHmPutKey,
    pub shmode_func: FnShmode,
    pub hmdel_key: FnHmDel,
    pub stralloc: FnStrAlloc,
    pub strreset: FnStrReset,
    pub strkey: FnStrKey,
    pub arr_ins: FnArrIns,
}

unsafe fn sym<T: Copy>(lib: &'static Library, name: &[u8]) -> T {
    let s: Symbol<T> = lib
        .get(name)
        .unwrap_or_else(|e| panic!("missing symbol {}: {e}", String::from_utf8_lossy(name)));
    *s
}

impl Api {
    fn load(path: &Path, name: &'static str) -> Api {
        let lib: &'static Library = Box::leak(Box::new(unsafe {
            Library::new(path).unwrap_or_else(|e| panic!("cannot dlopen {}: {e}", path.display()))
        }));
        unsafe {
            Api {
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
    }
}

pub fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf()
}

pub fn c_so_path() -> PathBuf {
    let dir = repo_root().join("c_src/build");
    let mut found: Option<PathBuf> = None;
    if let Ok(rd) = std::fs::read_dir(&dir) {
        for e in rd.flatten() {
            let p = e.path();
            if p.extension().map(|x| x == "so").unwrap_or(false) {
                found = Some(p);
            }
        }
    }
    found.unwrap_or_else(|| panic!("no .so found in {} - build the C library first", dir.display()))
}

pub fn rust_so_path() -> PathBuf {
    let base = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target");
    for prof in ["release", "debug"] {
        let p = base.join(prof).join("libarr_ins_lib.so");
        if p.exists() {
            return p;
        }
    }
    panic!("libarr_ins_lib.so not built");
}

/// The two APIs under comparison.  Loaded once per test binary.
pub fn apis() -> &'static (Api, Api) {
    use std::sync::OnceLock;
    static APIS: OnceLock<(Api, Api)> = OnceLock::new();
    APIS.get_or_init(|| {
        (
            Api::load(&c_so_path(), "C"),
            Api::load(&rust_so_path(), "Rust"),
        )
    })
}

/// Serialises tests that depend on the per-library global `stbds_hash_seed`.
pub fn seed_lock() -> std::sync::MutexGuard<'static, ()> {
    static M: std::sync::Mutex<()> = std::sync::Mutex::new(());
    M.lock().unwrap_or_else(|e| e.into_inner())
}

// ---------------------------------------------------------------------------
// deterministic PRNG (xorshift64*) - no external crates
// ---------------------------------------------------------------------------

pub struct Rng(u64);

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
        (0..n).map(|_| self.next_u32() as u8).collect()
    }
    /// Random NUL-terminated ASCII/high-bit string of `len` characters.
    pub fn cstring(&mut self, len: usize) -> Vec<u8> {
        let mut v: Vec<u8> = (0..len)
            .map(|_| {
                let b = self.next_u32() as u8;
                if b == 0 {
                    1
                } else {
                    b
                }
            })
            .collect();
        v.push(0);
        v
    }
}

// ---------------------------------------------------------------------------
// snapshotting: turn an opaque stb_ds block into a comparable value
// ---------------------------------------------------------------------------

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct IndexSnap {
    pub scalars: Vec<u64>,
    pub buckets: Vec<(Vec<u64>, Vec<i64>)>,
    pub arena_remaining: usize,
    pub arena_block: u8,
    pub arena_mode: u8,
    pub arena_blocks: usize,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct MapSnap {
    pub length: usize,
    pub capacity: usize,
    pub temp: isize,
    pub has_table: bool,
    /// raw element bytes, with the key pointer word blanked when keys are `char*`
    pub payload: Vec<u8>,
    /// key strings when keys are `char*`
    pub keys: Vec<Option<Vec<u8>>>,
    pub index: Option<IndexSnap>,
}

pub unsafe fn read_cstr(p: *const c_char) -> Vec<u8> {
    let mut out = Vec::new();
    let mut q = p as *const u8;
    loop {
        let b = *q;
        if b == 0 {
            break;
        }
        out.push(b);
        q = q.add(1);
    }
    out
}

unsafe fn count_blocks(mut b: *mut StringBlock) -> usize {
    let mut n = 0;
    while !b.is_null() {
        n += 1;
        b = (*b).next;
        assert!(n < 100_000, "cyclic arena block chain");
    }
    n
}

/// Snapshot the hash index of a *raw* (array-side) pointer.
///
/// NOTE: `temp_key` is deliberately **not** part of the snapshot.
/// `stbds_make_hash_index` never initialises it (only `t->string` is memset), so
/// it holds uninitialised heap bytes until the first `mode >= STBDS_HM_STRING`
/// put; it is compared explicitly by `Map::temp_key` where it is meaningful.
pub unsafe fn snap_index(raw_a: *mut c_void) -> Option<IndexSnap> {
    let hdr = (raw_a as *mut u8).sub(HEADER_SIZE) as *mut ArrayHeader;
    let t = (*hdr).hash_table as *mut HashIndex;
    if t.is_null() {
        return None;
    }
    let ti = &*t;
    let nbuckets = ti.slot_count >> BUCKET_SHIFT;
    let mut buckets = Vec::with_capacity(nbuckets);
    for i in 0..nbuckets {
        let b = &*ti.storage.add(i);
        buckets.push((
            b.hash.iter().map(|&h| h as u64).collect::<Vec<u64>>(),
            b.index.iter().map(|&x| x as i64).collect::<Vec<i64>>(),
        ));
    }
    Some(IndexSnap {
        scalars: vec![
            ti.slot_count as u64,
            ti.used_count as u64,
            ti.used_count_threshold as u64,
            ti.used_count_shrink_threshold as u64,
            ti.tombstone_count as u64,
            ti.tombstone_count_threshold as u64,
            ti.seed as u64,
            ti.slot_count_log2 as u64,
        ],
        buckets,
        arena_remaining: ti.string.remaining,
        arena_block: ti.string.block,
        arena_mode: ti.string.mode,
        arena_blocks: count_blocks(ti.string.storage),
    })
}

/// Snapshot a map given its *hash-side* pointer `a` (what `hmput_key` returns).
///
/// `pointer_keys` selects whether the `keysize` bytes at `keyoffset` are a
/// `char*` (string modes) or plain bytes.
pub unsafe fn snap_map(
    a: *mut c_void,
    elemsize: usize,
    keyoffset: usize,
    pointer_keys: bool,
) -> Option<MapSnap> {
    if a.is_null() {
        return None;
    }
    let raw_a = (a as *mut u8).sub(elemsize) as *mut c_void;
    let hdr = &*((raw_a as *mut u8).sub(HEADER_SIZE) as *mut ArrayHeader);
    let len = hdr.length;
    let mut payload = vec![0u8; len * elemsize];
    std::ptr::copy_nonoverlapping(raw_a as *const u8, payload.as_mut_ptr(), len * elemsize);
    let mut keys = Vec::new();
    if pointer_keys {
        // element 0 is the "default" slot and holds no key
        for i in 0..len {
            let base = i * elemsize + keyoffset;
            let pw = *((raw_a as *const u8).add(base) as *const *const c_char);
            if i == 0 || pw.is_null() {
                keys.push(None);
            } else {
                keys.push(Some(read_cstr(pw)));
            }
            // blank the pointer word so raw addresses never enter the comparison
            for b in payload[base..base + 8].iter_mut() {
                *b = 0;
            }
        }
    }
    Some(MapSnap {
        length: len,
        capacity: hdr.capacity,
        temp: hdr.temp,
        has_table: !hdr.hash_table.is_null(),
        payload,
        keys,
        index: snap_index(raw_a),
    })
}

/// Snapshot a plain dynamic array (`arrgrowf` result).
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct ArrSnap {
    pub length: usize,
    pub capacity: usize,
    pub temp: isize,
    pub has_table: bool,
    pub payload: Vec<u8>,
}

pub unsafe fn snap_arr(a: *mut c_void, elemsize: usize, nelems: usize) -> ArrSnap {
    let hdr = &*((a as *mut u8).sub(HEADER_SIZE) as *mut ArrayHeader);
    let mut payload = vec![0u8; nelems * elemsize];
    if nelems > 0 {
        std::ptr::copy_nonoverlapping(a as *const u8, payload.as_mut_ptr(), nelems * elemsize);
    }
    ArrSnap {
        length: hdr.length,
        capacity: hdr.capacity,
        temp: hdr.temp,
        has_table: !hdr.hash_table.is_null(),
        payload,
    }
}

// ---------------------------------------------------------------------------
// map driver: reproduces what the stb_ds macros do around the raw functions
// ---------------------------------------------------------------------------

pub struct Map<'a> {
    pub api: &'a Api,
    /// hash-side pointer (`t` in the macros); NULL until the first operation
    pub a: *mut c_void,
    pub elemsize: usize,
    pub keysize: usize,
    pub keyoffset: usize,
    pub pointer_keys: bool,
}

impl<'a> Map<'a> {
    pub fn new(
        api: &'a Api,
        elemsize: usize,
        keysize: usize,
        keyoffset: usize,
        pointer_keys: bool,
    ) -> Map<'a> {
        Map {
            api,
            a: std::ptr::null_mut(),
            elemsize,
            keysize,
            keyoffset,
            pointer_keys,
        }
    }

    /// `stbds_sh_new_arena` / `stbds_sh_new_strdup` style construction.
    pub fn new_shmode(
        api: &'a Api,
        elemsize: usize,
        keysize: usize,
        keyoffset: usize,
        mode: c_int,
    ) -> Map<'a> {
        let a = unsafe { (api.shmode_func)(elemsize, mode) };
        Map {
            api,
            a,
            elemsize,
            keysize,
            keyoffset,
            pointer_keys: true,
        }
    }

    pub fn raw(&self) -> *mut c_void {
        ((self.a as usize).wrapping_sub(self.elemsize)) as *mut c_void
    }

    pub unsafe fn header(&self) -> *mut ArrayHeader {
        (self.raw() as *mut u8).sub(HEADER_SIZE) as *mut ArrayHeader
    }

    pub unsafe fn temp(&self) -> isize {
        (*self.header()).temp
    }

    /// `stbds_hmput(t,k,v)` with an explicit `mode`: put the key, then write the
    /// value bytes at the slot the library selected.
    pub unsafe fn put(&mut self, key: &mut [u8], value: &[u8], value_off: usize, mode: c_int) {
        self.a = (self.api.hmput_key)(
            self.a,
            self.elemsize,
            key.as_mut_ptr() as *mut c_void,
            self.keysize,
            mode,
        );
        let t = self.temp();
        let slot = (self.a as *mut u8).add((t as usize).wrapping_mul(self.elemsize) + value_off);
        std::ptr::copy_nonoverlapping(value.as_ptr(), slot, value.len());
    }

    /// `stbds_hmgeti(t,k)` -> index (or -1)
    pub unsafe fn get(&mut self, key: &mut [u8], mode: c_int) -> isize {
        self.a = (self.api.hmget_key)(
            self.a,
            self.elemsize,
            key.as_mut_ptr() as *mut c_void,
            self.keysize,
            mode,
        );
        self.temp()
    }

    /// `stbds_hmgeti_ts(t,k,temp)` -> index (or -1)
    pub unsafe fn get_ts(&mut self, key: &mut [u8], mode: c_int) -> isize {
        let mut tmp: isize = 0x5a5a_5a5a;
        self.a = (self.api.hmget_key_ts)(
            self.a,
            self.elemsize,
            key.as_mut_ptr() as *mut c_void,
            self.keysize,
            &mut tmp,
            mode,
        );
        tmp
    }

    /// `stbds_hmdel(t,k)` -> 0/1
    pub unsafe fn del(&mut self, key: &mut [u8], mode: c_int) -> isize {
        self.a = (self.api.hmdel_key)(
            self.a,
            self.elemsize,
            key.as_mut_ptr() as *mut c_void,
            self.keysize,
            self.keyoffset,
            mode,
        );
        if self.a.is_null() {
            0
        } else {
            self.temp()
        }
    }

    pub unsafe fn put_default(&mut self) {
        self.a = (self.api.hmput_default)(self.a, self.elemsize);
    }

    /// `stbds_temp_key(t-1)` == `*(char**)header(raw)->hash_table`
    pub unsafe fn temp_key(&self) -> Option<Vec<u8>> {
        let t = (*self.header()).hash_table as *mut *mut c_char;
        if t.is_null() {
            return None;
        }
        let p = *t;
        if p.is_null() {
            None
        } else {
            Some(read_cstr(p))
        }
    }

    pub unsafe fn snap(&self) -> Option<MapSnap> {
        snap_map(self.a, self.elemsize, self.keyoffset, self.pointer_keys)
    }

    pub unsafe fn free(&mut self) {
        if !self.a.is_null() {
            (self.api.hmfree_func)(self.raw(), self.elemsize);
            self.a = std::ptr::null_mut();
        }
    }
}

/// Panic with a readable message when two snapshots diverge.
pub fn assert_same<T: PartialEq + std::fmt::Debug>(ctx: &str, c: &T, r: &T) {
    if c != r {
        panic!("DIVERGENCE [{ctx}]\n  C    = {c:?}\n  Rust = {r:?}");
    }
}
