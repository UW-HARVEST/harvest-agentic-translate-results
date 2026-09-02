//! Shared differential-test harness.
//!
//! Loads BOTH shared objects through `libloading` and calls only exported
//! symbols — the Rust crate is never linked directly, so the `#[no_mangle]`
//! wrappers are part of what is under test.

#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::ffi::{c_char, c_int, c_void};
use std::path::PathBuf;

// ---------------------------------------------------------------------------
// Exported-symbol signatures
// ---------------------------------------------------------------------------

pub type FnArrgrowf = unsafe extern "C" fn(*mut c_void, usize, usize, usize) -> *mut c_void;
pub type FnArrfreef = unsafe extern "C" fn(*mut c_void);
pub type FnRandSeed = unsafe extern "C" fn(usize);
pub type FnHashBytes = unsafe extern "C" fn(*mut c_void, usize, usize) -> usize;
pub type FnHashString = unsafe extern "C" fn(*mut c_char, usize) -> usize;
pub type FnHmfreeFunc = unsafe extern "C" fn(*mut c_void, usize);
pub type FnHmgetKey = unsafe extern "C" fn(*mut c_void, usize, *mut c_void, usize, c_int) -> *mut c_void;
pub type FnHmgetKeyTs =
    unsafe extern "C" fn(*mut c_void, usize, *mut c_void, usize, *mut isize, c_int) -> *mut c_void;
pub type FnHmputDefault = unsafe extern "C" fn(*mut c_void, usize) -> *mut c_void;
pub type FnHmputKey = unsafe extern "C" fn(*mut c_void, usize, *mut c_void, usize, c_int) -> *mut c_void;
pub type FnHmdelKey =
    unsafe extern "C" fn(*mut c_void, usize, *mut c_void, usize, usize, c_int) -> *mut c_void;
pub type FnShmodeFunc = unsafe extern "C" fn(usize, c_int) -> *mut c_void;
pub type FnStralloc = unsafe extern "C" fn(*mut c_void, *mut c_char) -> *mut c_char;
pub type FnStrreset = unsafe extern "C" fn(*mut c_void);
pub type FnStrkey = unsafe extern "C" fn(c_int) -> *mut c_char;
pub type FnIntput = unsafe extern "C" fn(c_int);

/// One loaded implementation (the C `.so` or the Rust `.so`).
pub struct Impl {
    _lib: Library,
    pub name: &'static str,
    pub arrgrowf: FnArrgrowf,
    pub arrfreef: FnArrfreef,
    pub rand_seed: FnRandSeed,
    pub hash_bytes: FnHashBytes,
    pub hash_string: FnHashString,
    pub hmfree_func: FnHmfreeFunc,
    pub hmget_key: FnHmgetKey,
    pub hmget_key_ts: FnHmgetKeyTs,
    pub hmput_default: FnHmputDefault,
    pub hmput_key: FnHmputKey,
    pub hmdel_key: FnHmdelKey,
    pub shmode_func: FnShmodeFunc,
    pub stralloc: FnStralloc,
    pub strreset: FnStrreset,
    pub strkey: FnStrkey,
    pub intput: FnIntput,
}

unsafe fn sym<T: Copy>(lib: &Library, n: &[u8]) -> T {
    unsafe {
        let s: Symbol<T> = lib
            .get(n)
            .unwrap_or_else(|e| panic!("symbol {:?} missing: {e}", String::from_utf8_lossy(n)));
        *s
    }
}

impl Impl {
    pub fn load(path: &PathBuf, name: &'static str) -> Impl {
        unsafe {
            let lib = Library::new(path).unwrap_or_else(|e| panic!("cannot load {path:?}: {e}"));
            Impl {
                name,
                arrgrowf: sym(&lib, b"stbds_arrgrowf\0"),
                arrfreef: sym(&lib, b"stbds_arrfreef\0"),
                rand_seed: sym(&lib, b"stbds_rand_seed\0"),
                hash_bytes: sym(&lib, b"stbds_hash_bytes\0"),
                hash_string: sym(&lib, b"stbds_hash_string\0"),
                hmfree_func: sym(&lib, b"stbds_hmfree_func\0"),
                hmget_key: sym(&lib, b"stbds_hmget_key\0"),
                hmget_key_ts: sym(&lib, b"stbds_hmget_key_ts\0"),
                hmput_default: sym(&lib, b"stbds_hmput_default\0"),
                hmput_key: sym(&lib, b"stbds_hmput_key\0"),
                hmdel_key: sym(&lib, b"stbds_hmdel_key\0"),
                shmode_func: sym(&lib, b"stbds_shmode_func\0"),
                stralloc: sym(&lib, b"stbds_stralloc\0"),
                strreset: sym(&lib, b"stbds_strreset\0"),
                strkey: sym(&lib, b"strkey\0"),
                intput: sym(&lib, b"intput\0"),
                _lib: lib,
            }
        }
    }
}

pub struct Libs {
    pub c: Impl,
    pub r: Impl,
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

pub fn c_so_path() -> PathBuf {
    let build = manifest_dir().parent().unwrap().join("c_src/build");
    let mut found: Vec<PathBuf> = std::fs::read_dir(&build)
        .unwrap_or_else(|e| panic!("{build:?} not built ({e}); run cmake first"))
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().map(|x| x == "so").unwrap_or(false))
        .collect();
    found.sort();
    assert_eq!(found.len(), 1, "expected exactly one .so in {build:?}, got {found:?}");
    found.pop().unwrap()
}

pub fn rust_so_path() -> PathBuf {
    let p = manifest_dir().join("target/release/libintput_lib.so");
    assert!(p.exists(), "{p:?} missing — run `cargo build --release` first");
    p
}

pub fn libs() -> Libs {
    Libs {
        c: Impl::load(&c_so_path(), "C"),
        r: Impl::load(&rust_so_path(), "Rust"),
    }
}

/// `dlopen` returns the *same* library instance for every caller in the
/// process, so the two `.so`s' `stbds_hash_seed` globals are shared across all
/// `#[test]` threads. Every scenario must therefore run under this lock, or
/// concurrent tests perturb each other's seed sequence.
pub static SERIAL: std::sync::Mutex<()> = std::sync::Mutex::new(());

pub fn with_libs<R>(f: impl FnOnce(&Libs) -> R) -> R {
    let guard = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let l = libs();
    let r = f(&l);
    drop(guard);
    r
}

// ---------------------------------------------------------------------------
// Deterministic RNG (SplitMix64) — fixed seed for reproducibility
// ---------------------------------------------------------------------------

pub struct Rng(pub u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
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
    /// Uniform-ish in `0..n` (n > 0).
    pub fn below(&mut self, n: usize) -> usize {
        (self.next_u64() % n as u64) as usize
    }
    pub fn range(&mut self, lo: usize, hi_incl: usize) -> usize {
        lo + self.below(hi_incl - lo + 1)
    }
    pub fn bytes(&mut self, n: usize) -> Vec<u8> {
        (0..n).map(|_| self.next_u64() as u8).collect()
    }
    pub fn i32(&mut self) -> i32 {
        self.next_u32() as i32
    }
}

// ---------------------------------------------------------------------------
// Structural layout mirrors (must match both implementations)
// ---------------------------------------------------------------------------

pub const HEADER_SIZE: usize = 32; // length, capacity, hash_table, temp
pub const BUCKET_LENGTH: usize = 8;
pub const BUCKET_SHIFT: usize = 3;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct ArrayHeader {
    pub length: usize,
    pub capacity: usize,
    pub hash_table: *mut c_void,
    pub temp: isize,
}

#[repr(C)]
pub struct StringArena {
    pub storage: *mut c_void,
    pub remaining: usize,
    pub block: u8,
    pub mode: u8,
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

#[repr(C)]
pub struct HashBucket {
    pub hash: [usize; BUCKET_LENGTH],
    pub index: [isize; BUCKET_LENGTH],
}

/// `stbds_header(t)` for a raw array pointer.
pub unsafe fn header(raw: *mut c_void) -> *mut ArrayHeader {
    unsafe { (raw as *mut u8).sub(HEADER_SIZE) as *mut ArrayHeader }
}

/// `STBDS_HASH_TO_ARR`
pub unsafe fn hash_to_arr(t: *mut c_void, elemsize: usize) -> *mut c_void {
    unsafe { (t as *mut u8).sub(elemsize) as *mut c_void }
}

// ---------------------------------------------------------------------------
// Digests: canonical, address-free snapshots for byte-for-byte comparison
// ---------------------------------------------------------------------------

/// How the key of a map element is stored, so the digest can compare contents
/// rather than addresses.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum KeyRepr {
    /// The first `keysize` bytes of the element are the key.
    Inline(usize),
    /// The first 8 bytes of the element are a `char *`.
    Pointer,
}

#[derive(Default)]
pub struct Digest(pub Vec<u8>);

impl Digest {
    pub fn u64(&mut self, v: u64) {
        self.0.extend_from_slice(&v.to_le_bytes());
    }
    pub fn i64(&mut self, v: i64) {
        self.0.extend_from_slice(&v.to_le_bytes());
    }
    pub fn usize(&mut self, v: usize) {
        self.u64(v as u64);
    }
    pub fn isize(&mut self, v: isize) {
        self.i64(v as i64);
    }
    pub fn u8(&mut self, v: u8) {
        self.0.push(v);
    }
    pub fn tag(&mut self, s: &str) {
        self.0.extend_from_slice(s.as_bytes());
        self.0.push(0);
    }
    pub fn bytes(&mut self, b: &[u8]) {
        self.usize(b.len());
        self.0.extend_from_slice(b);
    }
}

pub unsafe fn cstr_bytes(p: *const c_char) -> Vec<u8> {
    unsafe {
        if p.is_null() {
            return b"<null>".to_vec();
        }
        let mut v = Vec::new();
        let mut q = p as *const u8;
        while *q != 0 {
            v.push(*q);
            q = q.add(1);
        }
        v
    }
}

/// Digest of an `stbds_string_arena` (addresses excluded; `storage` reduced to
/// null / non-null because the block chain is heap-address dependent).
pub unsafe fn digest_arena(d: &mut Digest, a: *const StringArena) {
    unsafe {
        d.tag("arena");
        d.u8((!(*a).storage.is_null()) as u8);
        d.usize((*a).remaining);
        d.u8((*a).block);
        d.u8((*a).mode);
    }
}

/// Digest of the whole hash index reachable from a *raw array* pointer.
pub unsafe fn digest_table(
    d: &mut Digest,
    raw: *mut c_void,
    key_repr: KeyRepr,
    include_temp_key: bool,
) {
    unsafe {
        let h = header(raw);
        let ht = (*h).hash_table as *const HashIndex;
        if ht.is_null() {
            d.tag("no-table");
            return;
        }
        d.tag("table");
        d.usize((*ht).slot_count);
        d.usize((*ht).used_count);
        d.usize((*ht).used_count_threshold);
        d.usize((*ht).used_count_shrink_threshold);
        d.usize((*ht).tombstone_count);
        d.usize((*ht).tombstone_count_threshold);
        d.usize((*ht).seed);
        d.usize((*ht).slot_count_log2);
        digest_arena(d, &raw const (*ht).string);
        let _ = (key_repr, include_temp_key);
        let nbuckets = (*ht).slot_count >> BUCKET_SHIFT;
        for i in 0..nbuckets {
            let b = (*ht).storage.add(i);
            for j in 0..BUCKET_LENGTH {
                d.usize((*b).hash[j]);
            }
            for j in 0..BUCKET_LENGTH {
                d.isize((*b).index[j]);
            }
        }
    }
}

/// Digest of the live elements of a raw array (`[0, length)`).
///
/// Element 0 is the zero-initialised "default" slot. For `KeyRepr::Pointer` the
/// key is compared by string content and the remaining `elemsize - 8` bytes are
/// compared verbatim.
pub unsafe fn digest_elements(
    d: &mut Digest,
    raw: *mut c_void,
    elemsize: usize,
    length: usize,
    key_repr: KeyRepr,
) {
    unsafe {
        d.tag("elems");
        d.usize(length);
        for i in 0..length {
            let e = (raw as *mut u8).add(elemsize * i);
            match key_repr {
                KeyRepr::Inline(_) => {
                    d.0.extend_from_slice(std::slice::from_raw_parts(e, elemsize));
                }
                KeyRepr::Pointer => {
                    let kp = *(e as *const *const c_char);
                    if i == 0 {
                        // default slot: key pointer is NULL in both impls
                        d.u8((!kp.is_null()) as u8);
                    } else {
                        d.bytes(&cstr_bytes(kp));
                    }
                    d.0
                        .extend_from_slice(std::slice::from_raw_parts(e.add(8), elemsize - 8));
                }
            }
        }
    }
}

/// Index of the live element whose `char *key` field aliases the hash index's
/// `temp_key`, or `-1`. Never dereferences either pointer.
pub unsafe fn temp_key_index(raw: *mut c_void, elemsize: usize, length: usize) -> isize {
    unsafe {
        let ht = (*header(raw)).hash_table as *const HashIndex;
        if ht.is_null() {
            return -2;
        }
        let tk = (*ht).temp_key;
        for i in 0..length {
            let e = (raw as *mut u8).add(elemsize * i) as *const *mut c_char;
            if *e == tk {
                return i as isize;
            }
        }
        -1
    }
}

/// Full digest of a map, given the *hash* pointer returned by `hmput_key` etc.
pub unsafe fn digest_map(
    t: *mut c_void,
    elemsize: usize,
    key_repr: KeyRepr,
    include_temp_key: bool,
) -> Vec<u8> {
    unsafe {
        let mut d = Digest::default();
        if t.is_null() {
            d.tag("null-map");
            return d.0;
        }
        let raw = hash_to_arr(t, elemsize);
        let h = header(raw);
        d.tag("hdr");
        d.usize((*h).length);
        d.usize((*h).capacity);
        d.isize((*h).temp);
        digest_table(&mut d, raw, key_repr, include_temp_key);
        if key_repr == KeyRepr::Pointer && include_temp_key {
            // `temp_key` is a raw address, and after a table rebuild it is
            // uninitialised `realloc` memory, so it must never be dereferenced.
            // Compare it *positionally* instead: which live element's key
            // pointer does it alias? That is address-independent and identical
            // between the two implementations.
            d.tag("temp_key@");
            d.isize(temp_key_index(raw, elemsize, (*h).length));
        }
        digest_elements(&mut d, raw, elemsize, (*h).length, key_repr);
        d.0
    }
}

/// Full digest of a plain array (no hash table semantics), given the raw pointer.
pub unsafe fn digest_array(raw: *mut c_void, elemsize: usize, live: usize) -> Vec<u8> {
    unsafe {
        let mut d = Digest::default();
        if raw.is_null() {
            d.tag("null-arr");
            return d.0;
        }
        let h = header(raw);
        d.tag("arr");
        d.usize((*h).length);
        d.usize((*h).capacity);
        d.isize((*h).temp);
        d.u8((!(*h).hash_table.is_null()) as u8);
        d.usize(elemsize);
        d.0.extend_from_slice(std::slice::from_raw_parts(raw as *const u8, elemsize * live));
        d.0
    }
}

pub fn assert_same(what: &str, c: &[u8], r: &[u8]) {
    if c != r {
        let n = c.len().min(r.len());
        let mut first = n;
        for i in 0..n {
            if c[i] != r[i] {
                first = i;
                break;
            }
        }
        panic!(
            "{what}: C and Rust digests differ (C len {}, Rust len {}, first difference at byte {})\n\
             C   : {:02x?}\nRust: {:02x?}",
            c.len(),
            r.len(),
            first,
            &c[first.saturating_sub(16)..(first + 48).min(c.len())],
            &r[first.saturating_sub(16)..(first + 48).min(r.len())],
        );
    }
}

// ---------------------------------------------------------------------------
// Small conveniences
// ---------------------------------------------------------------------------

/// A NUL-terminated, mutable byte buffer usable as `char *`.
pub fn cstring(bytes: &[u8]) -> Vec<u8> {
    let mut v = bytes.to_vec();
    v.push(0);
    v
}

/// Random NUL-free string of `len` bytes drawn from `alphabet`.
pub fn rand_cstring(rng: &mut Rng, len: usize, alphabet: &[u8]) -> Vec<u8> {
    let mut v: Vec<u8> = (0..len).map(|_| alphabet[rng.below(alphabet.len())]).collect();
    v.push(0);
    v
}

pub const ASCII: &[u8] = b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789_-";
pub static HIGH_BIT: [u8; 128] = {
    let mut a = [0u8; 128];
    let mut i = 0;
    while i < 128 {
        a[i] = (i + 128) as u8;
        i += 1;
    }
    a
};

/// Reset both implementations' global hash seed so their LCG state is in
/// lockstep at the start of a scenario.
pub fn sync_seed(l: &Libs, seed: usize) {
    unsafe {
        (l.c.rand_seed)(seed);
        (l.r.rand_seed)(seed);
    }
}
