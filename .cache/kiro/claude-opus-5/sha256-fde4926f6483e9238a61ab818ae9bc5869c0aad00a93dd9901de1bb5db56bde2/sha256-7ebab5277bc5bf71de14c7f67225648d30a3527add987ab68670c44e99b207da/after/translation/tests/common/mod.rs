//! Shared harness: loads BOTH the C shared library and the Rust shared library
//! through `libloading` and exposes their exported symbols as plain C function
//! pointers.  Nothing in the crate under test is ever called directly – every
//! call goes through the `.so` export table, exactly like an external consumer.

#![allow(dead_code)]

use libloading::Library;
use std::ffi::{c_char, c_int, c_void};
use std::path::{Path, PathBuf};

// ---------------------------------------------------------------------------
// Mirrored C data structures (used only to *inspect* results)
// ---------------------------------------------------------------------------

pub const BUCKET_LENGTH: usize = 8;
pub const BUCKET_SHIFT: usize = 3;
pub const BUCKET_MASK: usize = BUCKET_LENGTH - 1;

pub const HM_BINARY: c_int = 0;
pub const HM_STRING: c_int = 1;

pub const SH_NONE: c_int = 0;
pub const SH_DEFAULT: c_int = 1;
pub const SH_STRDUP: c_int = 2;
pub const SH_ARENA: c_int = 3;

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct StringArena {
    pub storage: *mut StringBlock,
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
pub struct StringBlock {
    pub next: *mut StringBlock,
    pub storage: [c_char; 8],
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct ArrayHeader {
    pub length: usize,
    pub capacity: usize,
    pub hash_table: *mut c_void,
    pub temp: isize,
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

// ---------------------------------------------------------------------------
// Exported function signatures
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
pub type FnStrAlloc = unsafe extern "C" fn(*mut StringArena, *mut c_char) -> *mut c_char;
pub type FnStrReset = unsafe extern "C" fn(*mut StringArena);
pub type FnStrKey = unsafe extern "C" fn(c_int) -> *mut c_char;
pub type FnStrPut = unsafe extern "C" fn(c_int);

/// Every symbol the library exports, resolved once.
pub struct Api {
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
    pub str_put: FnStrPut,
}

macro_rules! sym {
    ($lib:expr, $t:ty, $n:literal) => {{
        let s: libloading::Symbol<$t> = $lib
            .get($n)
            .unwrap_or_else(|e| panic!("missing symbol {}: {e}", stringify!($n)));
        *s
    }};
}

impl Api {
    pub fn load(name: &'static str, path: &Path) -> Api {
        unsafe {
            // Leaked on purpose: the resolved function pointers must stay valid
            // for the whole test process.
            let lib: &'static Library = Box::leak(Box::new(
                Library::new(path).unwrap_or_else(|e| panic!("dlopen {}: {e}", path.display())),
            ));
            Api {
                name,
                arrgrowf: sym!(lib, FnArrGrowF, b"stbds_arrgrowf\0"),
                arrfreef: sym!(lib, FnArrFreeF, b"stbds_arrfreef\0"),
                rand_seed: sym!(lib, FnRandSeed, b"stbds_rand_seed\0"),
                hash_bytes: sym!(lib, FnHashBytes, b"stbds_hash_bytes\0"),
                hash_string: sym!(lib, FnHashString, b"stbds_hash_string\0"),
                hmfree_func: sym!(lib, FnHmFreeFunc, b"stbds_hmfree_func\0"),
                hmget_key_ts: sym!(lib, FnHmGetKeyTs, b"stbds_hmget_key_ts\0"),
                hmget_key: sym!(lib, FnHmGetKey, b"stbds_hmget_key\0"),
                hmput_default: sym!(lib, FnHmPutDefault, b"stbds_hmput_default\0"),
                hmput_key: sym!(lib, FnHmPutKey, b"stbds_hmput_key\0"),
                shmode_func: sym!(lib, FnShmodeFunc, b"stbds_shmode_func\0"),
                hmdel_key: sym!(lib, FnHmDelKey, b"stbds_hmdel_key\0"),
                stralloc: sym!(lib, FnStrAlloc, b"stbds_stralloc\0"),
                strreset: sym!(lib, FnStrReset, b"stbds_strreset\0"),
                strkey: sym!(lib, FnStrKey, b"strkey\0"),
                str_put: sym!(lib, FnStrPut, b"str_put\0"),
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Locating the two shared objects
// ---------------------------------------------------------------------------

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("manifest dir has a parent")
        .to_path_buf()
}

fn find_so(dir: &Path) -> Option<PathBuf> {
    let mut best: Option<PathBuf> = None;
    for e in std::fs::read_dir(dir).ok()? {
        let p = e.ok()?.path();
        if p.extension().map(|x| x == "so").unwrap_or(false) {
            best = Some(p);
        }
    }
    best
}

pub fn c_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("C_SO") {
        return PathBuf::from(p);
    }
    let build = workspace_root().join("c_src/build");
    find_so(&build).unwrap_or_else(|| {
        panic!(
            "no .so found in {} – build the C library first",
            build.display()
        )
    })
}

pub fn rust_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("RUST_SO") {
        return PathBuf::from(p);
    }
    let base = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target");
    let rel = base.join("release/libstr_put_lib.so");
    if rel.exists() {
        return rel;
    }
    let dbg = base.join("debug/libstr_put_lib.so");
    if dbg.exists() {
        return dbg;
    }
    panic!("libstr_put_lib.so not found under {}", base.display());
}

/// The pair of implementations under comparison.
pub struct Pair {
    pub c: Api,
    pub r: Api,
}

pub fn load_pair() -> Pair {
    Pair {
        c: Api::load("C", &c_so_path()),
        r: Api::load("Rust", &rust_so_path()),
    }
}

// ---------------------------------------------------------------------------
// Inspection helpers
// ---------------------------------------------------------------------------

pub unsafe fn header_of(raw: *mut c_void) -> *mut ArrayHeader {
    (raw as *mut ArrayHeader).offset(-1)
}

/// `t` is a "hash pointer" (raw array + elemsize), as returned by the
/// `stbds_hm*` functions.
pub unsafe fn raw_of(t: *mut c_void, elemsize: usize) -> *mut c_void {
    (t as *mut u8).offset(-(elemsize as isize)) as *mut c_void
}

pub unsafe fn cstr(p: *const c_char) -> String {
    if p.is_null() {
        return "(null)".to_string();
    }
    let mut v = Vec::new();
    let mut i = 0isize;
    while *p.offset(i) != 0 {
        v.push(*p.offset(i) as u8);
        i += 1;
    }
    String::from_utf8_lossy(&v).into_owned()
}

/// Textual dump of an array header (pointer values deliberately excluded).
pub unsafe fn dump_header(raw: *mut c_void) -> String {
    let h = &*header_of(raw);
    format!(
        "len={} cap={} temp={} ht={}",
        h.length,
        h.capacity,
        h.temp,
        if h.hash_table.is_null() { "nil" } else { "set" }
    )
}

pub unsafe fn dump_table(raw: *mut c_void) -> String {
    let h = &*header_of(raw);
    if h.hash_table.is_null() {
        return "table: nil".to_string();
    }
    let t = &*(h.hash_table as *mut HashIndex);
    // NOTE: `temp_key` is deliberately excluded -- `stbds_make_hash_index` never
    // initialises it, so it holds uninitialised heap bytes until a string-mode
    // `hmput_key` writes it. It is compared explicitly at the points where the C
    // code defines it (see `temp_key_str`).
    let mut s = format!(
        "table: slots={} used={} used_thr={} shrink_thr={} tomb={} tomb_thr={} seed={:#x} log2={} \
         str{{remaining={} block={} mode={} storage={}}}\n",
        t.slot_count,
        t.used_count,
        t.used_count_threshold,
        t.used_count_shrink_threshold,
        t.tombstone_count,
        t.tombstone_count_threshold,
        t.seed,
        t.slot_count_log2,
        t.string.remaining,
        t.string.block,
        t.string.mode,
        if t.string.storage.is_null() {
            "nil"
        } else {
            "set"
        },
    );
    // storage must be 64-byte aligned and inside the same allocation
    s.push_str(&format!(
        "storage_aligned={}\n",
        (t.storage as usize) % 64 == 0
    ));
    let buckets = t.slot_count >> BUCKET_SHIFT;
    for i in 0..buckets {
        let b = &*t.storage.add(i);
        s.push_str(&format!("  bucket[{i}] hash={:?} index={:?}\n", b.hash, b.index));
    }
    s
}

/// Raw element bytes for elements `0..length` of the underlying array.
pub unsafe fn dump_elems_bytes(raw: *mut c_void, elemsize: usize) -> String {
    let h = &*header_of(raw);
    let mut s = String::new();
    for i in 0..h.length {
        let p = (raw as *mut u8).add(elemsize * i);
        let bytes = std::slice::from_raw_parts(p, elemsize);
        s.push_str(&format!("  elem[{i}]={bytes:02x?}\n"));
    }
    s
}

/// Elements of a string-keyed map: key string + the `int` value that follows.
pub unsafe fn dump_elems_strmap(raw: *mut c_void, elemsize: usize) -> String {
    let h = &*header_of(raw);
    let mut s = String::new();
    for i in 0..h.length {
        let p = (raw as *mut u8).add(elemsize * i);
        let key = *(p as *mut *mut c_char);
        let val = *(p.add(std::mem::size_of::<*mut c_char>()) as *mut c_int);
        s.push_str(&format!("  elem[{i}] key=\"{}\" value={}\n", cstr(key), val));
    }
    s
}

pub unsafe fn dump_map(raw: *mut c_void, elemsize: usize, string_keys: bool) -> String {
    let mut s = String::new();
    s.push_str(&dump_header(raw));
    s.push('\n');
    s.push_str(&dump_table(raw));
    if string_keys {
        s.push_str(&dump_elems_strmap(raw, elemsize));
    } else {
        s.push_str(&dump_elems_bytes(raw, elemsize));
    }
    s
}

/// `stbds_temp_key(t)` as a string. Only defined immediately after a
/// string-mode `stbds_hmput_key` call.
pub unsafe fn temp_key_str(raw: *mut c_void) -> String {
    let h = &*header_of(raw);
    if h.hash_table.is_null() {
        return "(no table)".to_string();
    }
    cstr((*(h.hash_table as *mut HashIndex)).temp_key)
}

// ---------------------------------------------------------------------------
// Cheap structural fingerprint (used for the very frequent equality checks;
// the textual dump above is only produced when a fingerprint mismatch is found)
// ---------------------------------------------------------------------------

struct Fnv(u64);

impl Fnv {
    fn new() -> Fnv {
        Fnv(0xcbf2_9ce4_8422_2325)
    }
    fn w(&mut self, v: u64) {
        self.0 ^= v;
        self.0 = self.0.wrapping_mul(0x1000_0000_01b3);
    }
    unsafe fn wstr(&mut self, p: *const c_char) {
        if p.is_null() {
            self.w(0x0F0F_0F0F);
            return;
        }
        self.w(0x5555_5555);
        let mut i = 0isize;
        while *p.offset(i) != 0 {
            self.w(*p.offset(i) as u8 as u64);
            i += 1;
        }
        self.w(0xAAAA_AAAA);
    }
}

pub unsafe fn fingerprint(raw: *mut c_void, elemsize: usize, string_keys: bool) -> u64 {
    let mut f = Fnv::new();
    let h = &*header_of(raw);
    f.w(h.length as u64);
    f.w(h.capacity as u64);
    f.w(h.temp as u64);
    f.w(h.hash_table.is_null() as u64);

    if !h.hash_table.is_null() {
        let t = &*(h.hash_table as *mut HashIndex);
        for v in [
            t.slot_count,
            t.used_count,
            t.used_count_threshold,
            t.used_count_shrink_threshold,
            t.tombstone_count,
            t.tombstone_count_threshold,
            t.seed,
            t.slot_count_log2,
            t.string.remaining,
        ] {
            f.w(v as u64);
        }
        f.w(t.string.block as u64);
        f.w(t.string.mode as u64);
        f.w(t.string.storage.is_null() as u64);
        f.w(((t.storage as usize) % 64) as u64);
        for i in 0..(t.slot_count >> BUCKET_SHIFT) {
            let b = &*t.storage.add(i);
            for j in 0..BUCKET_LENGTH {
                f.w(b.hash[j] as u64);
                f.w(b.index[j] as u64);
            }
        }
    }

    for i in 0..h.length {
        let p = (raw as *mut u8).add(elemsize * i);
        if string_keys {
            f.wstr(*(p as *mut *mut c_char));
            f.w(*(p.add(std::mem::size_of::<*mut c_char>()) as *mut c_int) as i64 as u64);
        } else {
            for k in 0..elemsize {
                f.w(*p.add(k) as u64);
            }
        }
    }
    f.0
}

// ---------------------------------------------------------------------------
// Deterministic pseudo-random data (identical for both sides)
// ---------------------------------------------------------------------------

pub struct Rng(pub u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
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
    pub fn next_u32(&mut self) -> u32 {
        (self.next_u64() >> 32) as u32
    }
    pub fn bytes(&mut self, n: usize) -> Vec<u8> {
        (0..n).map(|_| (self.next_u64() >> 24) as u8).collect()
    }
}

/// NUL-terminated C string buffer.
pub fn cbuf(s: &str) -> Vec<c_char> {
    let mut v: Vec<c_char> = s.bytes().map(|b| b as c_char).collect();
    v.push(0);
    v
}

pub fn cbuf_bytes(s: &[u8]) -> Vec<c_char> {
    let mut v: Vec<c_char> = s.iter().map(|&b| b as c_char).collect();
    v.push(0);
    v
}
