//! Shared differential-testing harness.
//!
//! Loads BOTH the C `.so` and the Rust `.so` through `libloading` and exposes
//! every exported symbol as a raw function pointer, so the Rust code under test
//! is always reached through its `#[no_mangle] extern "C"` wrappers exactly as
//! an external consumer would reach it.

#![allow(dead_code)]
#![allow(non_snake_case)]

use libloading::{Library, Symbol};
use std::ffi::{c_char, c_int, c_void};
use std::path::PathBuf;

// ---------------------------------------------------------------------------
// Layout mirrors of the C internal structs (for state comparison only)
// ---------------------------------------------------------------------------

pub const STBDS_BUCKET_LENGTH: usize = 8;
pub const STBDS_BUCKET_SHIFT: usize = 3;
pub const STBDS_BUCKET_MASK: usize = 7;

pub const STBDS_HM_BINARY: c_int = 0;
pub const STBDS_HM_STRING: c_int = 1;

pub const STBDS_SH_NONE: c_int = 0;
pub const STBDS_SH_DEFAULT: c_int = 1;
pub const STBDS_SH_STRDUP: c_int = 2;
pub const STBDS_SH_ARENA: c_int = 3;

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct ArrayHeader {
    pub length: usize,
    pub capacity: usize,
    pub hash_table: *mut c_void,
    pub temp: isize,
}

pub const HEADER_SIZE: usize = std::mem::size_of::<ArrayHeader>();

#[repr(C)]
#[derive(Clone, Copy)]
pub struct StringArena {
    pub storage: *mut c_void,
    pub remaining: usize,
    pub block: u8,
    pub mode: u8,
}

#[repr(C)]
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
// The loaded library
// ---------------------------------------------------------------------------

pub type FnArrIns = unsafe extern "C" fn(c_int);
pub type FnStrkey = unsafe extern "C" fn(c_int) -> *mut c_char;
pub type FnRandSeed = unsafe extern "C" fn(usize);
pub type FnHashBytes = unsafe extern "C" fn(*mut c_void, usize, usize) -> usize;
pub type FnHashString = unsafe extern "C" fn(*mut c_char, usize) -> usize;
pub type FnArrGrowf = unsafe extern "C" fn(*mut c_void, usize, usize, usize) -> *mut c_void;
pub type FnArrFreef = unsafe extern "C" fn(*mut c_void);
pub type FnHmFree = unsafe extern "C" fn(*mut c_void, usize);
pub type FnHmGetKey = unsafe extern "C" fn(*mut c_void, usize, *mut c_void, usize, c_int) -> *mut c_void;
pub type FnHmGetKeyTs =
    unsafe extern "C" fn(*mut c_void, usize, *mut c_void, usize, *mut isize, c_int) -> *mut c_void;
pub type FnHmPutDefault = unsafe extern "C" fn(*mut c_void, usize) -> *mut c_void;
pub type FnHmPutKey = unsafe extern "C" fn(*mut c_void, usize, *mut c_void, usize, c_int) -> *mut c_void;
pub type FnHmDelKey =
    unsafe extern "C" fn(*mut c_void, usize, *mut c_void, usize, usize, c_int) -> *mut c_void;
pub type FnShmodeFunc = unsafe extern "C" fn(usize, c_int) -> *mut c_void;
pub type FnStralloc = unsafe extern "C" fn(*mut StringArena, *mut c_char) -> *mut c_char;
pub type FnStrreset = unsafe extern "C" fn(*mut StringArena);

pub struct Lib {
    pub name: &'static str,
    _lib: Library,
    pub arr_ins: FnArrIns,
    pub strkey: FnStrkey,
    pub rand_seed: FnRandSeed,
    pub hash_bytes: FnHashBytes,
    pub hash_string: FnHashString,
    pub arrgrowf: FnArrGrowf,
    pub arrfreef: FnArrFreef,
    pub hmfree_func: FnHmFree,
    pub hmget_key: FnHmGetKey,
    pub hmget_key_ts: FnHmGetKeyTs,
    pub hmput_default: FnHmPutDefault,
    pub hmput_key: FnHmPutKey,
    pub hmdel_key: FnHmDelKey,
    pub shmode_func: FnShmodeFunc,
    pub stralloc: FnStralloc,
    pub strreset: FnStrreset,
}

macro_rules! sym {
    ($lib:expr, $name:literal, $ty:ty) => {{
        let s: Symbol<$ty> = $lib
            .get(concat!($name, "\0").as_bytes())
            .unwrap_or_else(|e| panic!("missing symbol {}: {}", $name, e));
        *s
    }};
}

impl Lib {
    pub unsafe fn load(name: &'static str, path: &PathBuf) -> Lib {
        let lib = Library::new(path).unwrap_or_else(|e| panic!("load {:?}: {}", path, e));
        let l = Lib {
            name,
            arr_ins: sym!(lib, "arr_ins", FnArrIns),
            strkey: sym!(lib, "strkey", FnStrkey),
            rand_seed: sym!(lib, "stbds_rand_seed", FnRandSeed),
            hash_bytes: sym!(lib, "stbds_hash_bytes", FnHashBytes),
            hash_string: sym!(lib, "stbds_hash_string", FnHashString),
            arrgrowf: sym!(lib, "stbds_arrgrowf", FnArrGrowf),
            arrfreef: sym!(lib, "stbds_arrfreef", FnArrFreef),
            hmfree_func: sym!(lib, "stbds_hmfree_func", FnHmFree),
            hmget_key: sym!(lib, "stbds_hmget_key", FnHmGetKey),
            hmget_key_ts: sym!(lib, "stbds_hmget_key_ts", FnHmGetKeyTs),
            hmput_default: sym!(lib, "stbds_hmput_default", FnHmPutDefault),
            hmput_key: sym!(lib, "stbds_hmput_key", FnHmPutKey),
            hmdel_key: sym!(lib, "stbds_hmdel_key", FnHmDelKey),
            shmode_func: sym!(lib, "stbds_shmode_func", FnShmodeFunc),
            stralloc: sym!(lib, "stbds_stralloc", FnStralloc),
            strreset: sym!(lib, "stbds_strreset", FnStrreset),
            _lib: lib,
        };
        l
    }
}

fn workspace_root() -> PathBuf {
    // CARGO_MANIFEST_DIR == <root>/translation
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.pop();
    p
}

pub fn c_so_path() -> PathBuf {
    let root = workspace_root();
    let dir = root.join("c_src/build");
    let mut found = None;
    if let Ok(rd) = std::fs::read_dir(&dir) {
        for e in rd.flatten() {
            let n = e.file_name().to_string_lossy().to_string();
            if n.starts_with("lib") && n.ends_with(".so") {
                found = Some(e.path());
            }
        }
    }
    found.unwrap_or_else(|| panic!("no C .so found in {:?}; build it first", dir))
}

pub fn rust_so_path() -> PathBuf {
    let root = workspace_root();
    // DIFF_RUST_PROFILE=debug lets the whole suite be re-run against the
    // overflow-checked debug cdylib, which proves no path relies on Rust
    // arithmetic that would panic where the C wraps.
    let prefer = std::env::var("DIFF_RUST_PROFILE").unwrap_or_else(|_| "release".to_string());
    let mut order = vec![prefer.clone()];
    for p in ["release", "debug"] {
        if p != prefer {
            order.push(p.to_string());
        }
    }
    for prof in order {
        let p = root.join(format!("translation/target/{}/libarr_ins_lib.so", prof));
        if p.exists() {
            return p;
        }
    }
    panic!("no Rust .so found; run `cargo build --release`");
}

/// The two libraries under comparison.
pub struct Pair {
    pub c: Lib,
    pub r: Lib,
}

pub fn pair() -> Pair {
    unsafe {
        Pair {
            c: Lib::load("C", &c_so_path()),
            r: Lib::load("Rust", &rust_so_path()),
        }
    }
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (SplitMix64) — fixed seeds for reproducibility
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
    /// A random NUL-terminated C string of `len` payload bytes (no interior NUL).
    pub fn cstring(&mut self, len: usize) -> Vec<u8> {
        let mut v: Vec<u8> = (0..len)
            .map(|_| {
                let b = self.byte();
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
    /// A random printable-ASCII NUL-terminated C string.
    pub fn ascii_cstring(&mut self, len: usize) -> Vec<u8> {
        let mut v: Vec<u8> = (0..len).map(|_| b'a' + (self.byte() % 26)).collect();
        v.push(0);
        v
    }
}

// ---------------------------------------------------------------------------
// Observable-state snapshots
// ---------------------------------------------------------------------------

/// Everything about an stb_ds dynamic array that is a *value* (not an address).
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct ArrSnapshot {
    pub is_null: bool,
    pub length: usize,
    pub capacity: usize,
    pub temp: isize,
    pub has_table: bool,
    pub elems: Vec<u8>,
}

/// `a` is the *raw array* pointer (element 0), i.e. `t` for arrays and
/// `t - elemsize` for hash maps.
pub unsafe fn snap_arr(a: *mut c_void, elemsize: usize) -> ArrSnapshot {
    if a.is_null() {
        return ArrSnapshot {
            is_null: true,
            length: 0,
            capacity: 0,
            temp: 0,
            has_table: false,
            elems: Vec::new(),
        };
    }
    let h = (a as *mut u8).sub(HEADER_SIZE) as *mut ArrayHeader;
    let len = (*h).length;
    let n = len.saturating_mul(elemsize);
    let elems = std::slice::from_raw_parts(a as *const u8, n).to_vec();
    ArrSnapshot {
        is_null: false,
        length: len,
        capacity: (*h).capacity,
        temp: (*h).temp,
        has_table: !(*h).hash_table.is_null(),
        elems,
    }
}

/// Address-free snapshot of the whole `stbds_hash_index` (scalars + all buckets).
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct TableSnapshot {
    pub present: bool,
    pub slot_count: usize,
    pub used_count: usize,
    pub used_count_threshold: usize,
    pub used_count_shrink_threshold: usize,
    pub tombstone_count: usize,
    pub tombstone_count_threshold: usize,
    pub seed: usize,
    pub slot_count_log2: usize,
    pub str_remaining: usize,
    pub str_block: u8,
    pub str_mode: u8,
    pub str_has_storage: bool,
    pub hashes: Vec<usize>,
    pub indices: Vec<isize>,
}

pub unsafe fn snap_table(a: *mut c_void) -> TableSnapshot {
    let empty = TableSnapshot {
        present: false,
        slot_count: 0,
        used_count: 0,
        used_count_threshold: 0,
        used_count_shrink_threshold: 0,
        tombstone_count: 0,
        tombstone_count_threshold: 0,
        seed: 0,
        slot_count_log2: 0,
        str_remaining: 0,
        str_block: 0,
        str_mode: 0,
        str_has_storage: false,
        hashes: Vec::new(),
        indices: Vec::new(),
    };
    if a.is_null() {
        return empty;
    }
    let h = (a as *mut u8).sub(HEADER_SIZE) as *mut ArrayHeader;
    let t = (*h).hash_table as *mut HashIndex;
    if t.is_null() {
        return empty;
    }
    let nbuckets = (*t).slot_count >> STBDS_BUCKET_SHIFT;
    let mut hashes = Vec::with_capacity((*t).slot_count);
    let mut indices = Vec::with_capacity((*t).slot_count);
    for i in 0..nbuckets {
        let b = (*t).storage.add(i);
        for j in 0..STBDS_BUCKET_LENGTH {
            hashes.push((*b).hash[j]);
            indices.push((*b).index[j]);
        }
    }
    TableSnapshot {
        present: true,
        slot_count: (*t).slot_count,
        used_count: (*t).used_count,
        used_count_threshold: (*t).used_count_threshold,
        used_count_shrink_threshold: (*t).used_count_shrink_threshold,
        tombstone_count: (*t).tombstone_count,
        tombstone_count_threshold: (*t).tombstone_count_threshold,
        seed: (*t).seed,
        slot_count_log2: (*t).slot_count_log2,
        str_remaining: (*t).string.remaining,
        str_block: (*t).string.block,
        str_mode: (*t).string.mode,
        str_has_storage: !(*t).string.storage.is_null(),
        hashes,
        indices,
    }
}

// ---------------------------------------------------------------------------
// Hash-map driver: reproduces the stb_ds *macros* on top of the low-level API
// ---------------------------------------------------------------------------

/// `t` is the hash pointer (what the macros keep in the user's variable);
/// the raw array pointer is `t - elemsize`.
pub struct Hm<'a> {
    pub lib: &'a Lib,
    pub t: *mut c_void,
    pub elemsize: usize,
    pub keysize: usize,
    pub keyoffset: usize,
    /// where the driver writes the "value" half of the element
    pub valoffset: usize,
    pub valsize: usize,
}

impl<'a> Hm<'a> {
    pub fn new(lib: &'a Lib, elemsize: usize, keysize: usize, keyoffset: usize) -> Hm<'a> {
        Hm {
            lib,
            t: std::ptr::null_mut(),
            elemsize,
            keysize,
            keyoffset,
            valoffset: keysize,
            valsize: elemsize - keysize,
        }
    }

    pub unsafe fn from_shmode(lib: &'a Lib, elemsize: usize, keysize: usize, mode: c_int) -> Hm<'a> {
        let t = (lib.shmode_func)(elemsize, mode);
        Hm {
            lib,
            t,
            elemsize,
            keysize,
            keyoffset: 0,
            valoffset: keysize,
            valsize: elemsize - keysize,
        }
    }

    /// raw array pointer (`(t)-1` in macro terms)
    pub unsafe fn raw(&self) -> *mut c_void {
        if self.t.is_null() {
            std::ptr::null_mut()
        } else {
            (self.t as *mut u8).sub(self.elemsize) as *mut c_void
        }
    }

    pub unsafe fn header(&self) -> *mut ArrayHeader {
        (self.raw() as *mut u8).sub(HEADER_SIZE) as *mut ArrayHeader
    }

    pub unsafe fn temp(&self) -> isize {
        if self.t.is_null() {
            0
        } else {
            (*self.header()).temp
        }
    }

    pub unsafe fn temp_key(&self) -> *mut c_char {
        let ti = (*self.header()).hash_table as *mut HashIndex;
        if ti.is_null() {
            std::ptr::null_mut()
        } else {
            (*ti).temp_key
        }
    }

    /// element slot `i` of the hash array (`t[i]`)
    pub unsafe fn slot(&self, i: usize) -> *mut u8 {
        (self.t as *mut u8).add(self.elemsize * i)
    }

    /// `stbds_hmput_key` + the macro's key/value store.
    /// `key` is `keysize` bytes; `value` is written at `valoffset`.
    pub unsafe fn put(&mut self, key: &[u8], mode: c_int) -> isize {
        self.t = (self.lib.hmput_key)(
            self.t,
            self.elemsize,
            key.as_ptr() as *mut c_void,
            self.keysize,
            mode,
        );
        self.temp()
    }

    /// `stbds_hmput(t,k,v)`: put + write BOTH halves of the element, so that the
    /// whole `elemsize` bytes are deterministic and byte-comparable.
    pub unsafe fn put_kv(&mut self, key: &[u8], value: &[u8], mode: c_int) -> isize {
        let idx = self.put(key, mode);
        assert!(idx >= 0, "hmput_key returned a negative index");
        let s = self.slot(idx as usize);
        std::ptr::copy_nonoverlapping(key.as_ptr(), s.add(self.keyoffset), self.keysize);
        assert_eq!(value.len(), self.valsize);
        std::ptr::copy_nonoverlapping(value.as_ptr(), s.add(self.valoffset), self.valsize);
        idx
    }

    /// `stbds_shput(t,k,v)`: the key argument is the `char*` itself and the macro
    /// writes ONLY the value half (the key slot is owned by `hmput_key`).
    pub unsafe fn put_str(&mut self, key: *mut c_char, value: &[u8], mode: c_int) -> isize {
        self.t = (self.lib.hmput_key)(
            self.t,
            self.elemsize,
            key as *mut c_void,
            self.keysize,
            mode,
        );
        let idx = self.temp();
        assert!(idx >= 0);
        let s = self.slot(idx as usize);
        assert_eq!(value.len(), self.valsize);
        std::ptr::copy_nonoverlapping(value.as_ptr(), s.add(self.valoffset), self.valsize);
        idx
    }

    pub unsafe fn get_str(&mut self, key: *mut c_char, mode: c_int) -> isize {
        self.t = (self.lib.hmget_key)(
            self.t,
            self.elemsize,
            key as *mut c_void,
            self.keysize,
            mode,
        );
        self.temp()
    }

    pub unsafe fn del_str(&mut self, key: *mut c_char, mode: c_int) -> isize {
        self.t = (self.lib.hmdel_key)(
            self.t,
            self.elemsize,
            key as *mut c_void,
            self.keysize,
            self.keyoffset,
            mode,
        );
        if self.t.is_null() {
            0
        } else {
            self.temp()
        }
    }

    pub unsafe fn get(&mut self, key: &[u8], mode: c_int) -> isize {
        self.t = (self.lib.hmget_key)(
            self.t,
            self.elemsize,
            key.as_ptr() as *mut c_void,
            self.keysize,
            mode,
        );
        self.temp()
    }

    pub unsafe fn get_ts(&mut self, key: &[u8], mode: c_int) -> isize {
        let mut temp: isize = 0x5555_5555;
        self.t = (self.lib.hmget_key_ts)(
            self.t,
            self.elemsize,
            key.as_ptr() as *mut c_void,
            self.keysize,
            &mut temp,
            mode,
        );
        temp
    }

    pub unsafe fn del(&mut self, key: &[u8], mode: c_int) -> isize {
        self.t = (self.lib.hmdel_key)(
            self.t,
            self.elemsize,
            key.as_ptr() as *mut c_void,
            self.keysize,
            self.keyoffset,
            mode,
        );
        if self.t.is_null() {
            0
        } else {
            self.temp()
        }
    }

    pub unsafe fn put_default(&mut self) {
        self.t = (self.lib.hmput_default)(self.t, self.elemsize);
    }

    pub unsafe fn snap(&self) -> (ArrSnapshot, TableSnapshot) {
        let raw = self.raw();
        (snap_arr(raw, self.elemsize), snap_table(raw))
    }

    pub unsafe fn free(&mut self) {
        if !self.t.is_null() {
            (self.lib.hmfree_func)(self.raw(), self.elemsize);
            self.t = std::ptr::null_mut();
        }
    }
}

/// Read a NUL-terminated string from a pointer, as bytes (without the NUL).
pub unsafe fn cstr_bytes(p: *const c_char) -> Vec<u8> {
    if p.is_null() {
        return Vec::new();
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
    }
    v
}

/// Address-free snapshot of a *string* hash map: for every element the key's
/// string CONTENT (the pointer itself is not comparable across the two `.so`s)
/// plus the value bytes.
pub unsafe fn snap_str_elems(hm: &Hm) -> Vec<(Vec<u8>, Vec<u8>)> {
    if hm.t.is_null() {
        return Vec::new();
    }
    let len = (*hm.header()).length;
    let mut out = Vec::new();
    for i in 0..len {
        // element i of the raw array == hm.t[i-1]; iterate over raw indices
        let e = (hm.raw() as *mut u8).add(hm.elemsize * i);
        let kp = *(e.add(hm.keyoffset) as *mut *mut c_char);
        let k = cstr_bytes(kp);
        let v = std::slice::from_raw_parts(e.add(hm.valoffset), hm.valsize).to_vec();
        out.push((k, v));
    }
    out
}

// ---------------------------------------------------------------------------
// Trace-based differential comparison
// ---------------------------------------------------------------------------

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum Ev {
    Tag(&'static str),
    I(isize),
    U(usize),
    Bool(bool),
    Bytes(Vec<u8>),
    Arr(ArrSnapshot),
    Tbl(TableSnapshot),
    Str(Vec<(Vec<u8>, Vec<u8>)>),
}

pub type Trace = Vec<Ev>;

pub fn compare_traces(what: &str, c: &Trace, r: &Trace) {
    let n = c.len().min(r.len());
    for i in 0..n {
        if c[i] != r[i] {
            panic!(
                "\n=== DIVERGENCE in `{}` at trace event #{} ===\n  C    : {:?}\n  Rust : {:?}\n  (context: {:?})\n",
                what,
                i,
                &c[i],
                &r[i],
                &c[i.saturating_sub(4)..i]
            );
        }
    }
    assert_eq!(
        c.len(),
        r.len(),
        "`{}`: trace length differs (C={} Rust={})",
        what,
        c.len(),
        r.len()
    );
}

/// Run the same closure against the C lib and the Rust lib and compare traces.
pub fn diff<F>(what: &str, f: F)
where
    F: Fn(&Lib, &mut Trace),
{
    let p = pair();
    let mut tc: Trace = Vec::new();
    let mut tr: Trace = Vec::new();
    f(&p.c, &mut tc);
    f(&p.r, &mut tr);
    compare_traces(what, &tc, &tr);
}
