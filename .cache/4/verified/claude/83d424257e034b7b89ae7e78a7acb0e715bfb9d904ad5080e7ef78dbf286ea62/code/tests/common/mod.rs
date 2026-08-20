//! Shared differential-test harness.
//!
//! Loads **both** shared objects (the C reference and the Rust translation)
//! through `libloading` and calls every entry point across the FFI boundary, so
//! the `#[no_mangle]` export wrappers are exercised exactly as an external
//! consumer would use them.  Nothing in the Rust crate is ever called directly.

#![allow(dead_code)]
#![allow(clippy::missing_safety_doc)]

use std::ffi::{c_char, c_int, c_void, CStr};
use std::fmt::Write as _;
use std::path::PathBuf;
use std::ptr;

// ---------------------------------------------------------------------------
// Constants mirrored from c_src/src/lib.c
// ---------------------------------------------------------------------------

pub const STBDS_HM_BINARY: c_int = 0;
pub const STBDS_HM_STRING: c_int = 1;

pub const STBDS_SH_NONE: c_int = 0;
pub const STBDS_SH_DEFAULT: c_int = 1;
pub const STBDS_SH_STRDUP: c_int = 2;
pub const STBDS_SH_ARENA: c_int = 3;

pub const STBDS_BUCKET_LENGTH: usize = 8;
pub const DEFAULT_HASH_SEED: usize = 0x3141_5926;

// ---------------------------------------------------------------------------
// Layout-compatible views of the C structures (read-only inspection)
// ---------------------------------------------------------------------------

#[repr(C)]
#[derive(Clone, Copy)]
pub struct ArrayHeader {
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
    pub fn zeroed() -> Self {
        StringArena { storage: ptr::null_mut(), remaining: 0, block: 0, mode: 0 }
    }
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct HashBucket {
    pub hash: [usize; STBDS_BUCKET_LENGTH],
    pub index: [isize; STBDS_BUCKET_LENGTH],
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

const _: () = {
    assert!(std::mem::size_of::<ArrayHeader>() == 32);
    assert!(std::mem::size_of::<StringArena>() == 24);
    assert!(std::mem::size_of::<HashBucket>() == 128);
    assert!(std::mem::size_of::<HashIndex>() == 104);
};

// ---------------------------------------------------------------------------
// Library loading
// ---------------------------------------------------------------------------

pub type FnRandSeed = unsafe extern "C" fn(usize);
pub type FnHashBytes = unsafe extern "C" fn(*mut c_void, usize, usize) -> usize;
pub type FnHashString = unsafe extern "C" fn(*mut c_char, usize) -> usize;
pub type FnArrGrowf = unsafe extern "C" fn(*mut c_void, usize, usize, usize) -> *mut c_void;
pub type FnArrFreef = unsafe extern "C" fn(*mut c_void);
pub type FnHmFreeFunc = unsafe extern "C" fn(*mut c_void, usize);
pub type FnHmGetKey = unsafe extern "C" fn(*mut c_void, usize, *mut c_void, usize, c_int) -> *mut c_void;
pub type FnHmGetKeyTs =
    unsafe extern "C" fn(*mut c_void, usize, *mut c_void, usize, *mut isize, c_int) -> *mut c_void;
pub type FnHmPutDefault = unsafe extern "C" fn(*mut c_void, usize) -> *mut c_void;
pub type FnHmPutKey = unsafe extern "C" fn(*mut c_void, usize, *mut c_void, usize, c_int) -> *mut c_void;
pub type FnHmDelKey =
    unsafe extern "C" fn(*mut c_void, usize, *mut c_void, usize, usize, c_int) -> *mut c_void;
pub type FnShmodeFunc = unsafe extern "C" fn(usize, c_int) -> *mut c_void;
pub type FnStrAlloc = unsafe extern "C" fn(*mut StringArena, *mut c_char) -> *mut c_char;
pub type FnStrReset = unsafe extern "C" fn(*mut StringArena);
pub type FnStrKey = unsafe extern "C" fn(c_int) -> *mut c_char;
pub type FnHmGeti = unsafe extern "C" fn(c_int);

pub struct Api {
    pub name: &'static str,
    pub rand_seed: FnRandSeed,
    pub hash_bytes: FnHashBytes,
    pub hash_string: FnHashString,
    pub arrgrowf: FnArrGrowf,
    pub arrfreef: FnArrFreef,
    pub hmfree_func: FnHmFreeFunc,
    pub hmget_key: FnHmGetKey,
    pub hmget_key_ts: FnHmGetKeyTs,
    pub hmput_default: FnHmPutDefault,
    pub hmput_key: FnHmPutKey,
    pub hmdel_key: FnHmDelKey,
    pub shmode_func: FnShmodeFunc,
    pub stralloc: FnStrAlloc,
    pub strreset: FnStrReset,
    pub strkey: FnStrKey,
    pub hm_geti: FnHmGeti,
}

fn target_profile_dir() -> PathBuf {
    // .../target/<profile>/deps/<test-bin>
    let exe = std::env::current_exe().expect("current_exe");
    exe.parent().unwrap().parent().unwrap().to_path_buf()
}

pub fn rust_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("DIFF_RUST_SO") {
        return PathBuf::from(p);
    }
    let p = target_profile_dir().join("libhm_geti_lib.so");
    // IMPORTANT: `cargo test` does NOT rebuild a `cdylib`-only lib target
    // (integration tests cannot link one), so the `.so` on disk can easily be
    // stale.  Testing a stale artifact silently invalidates the whole suite, so
    // refuse to run instead.
    assert_so_not_stale(&p);
    p
}

fn assert_so_not_stale(so: &PathBuf) {
    let src = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/lib.rs");
    let (so_m, src_m) = match (
        std::fs::metadata(so).and_then(|m| m.modified()),
        std::fs::metadata(&src).and_then(|m| m.modified()),
    ) {
        (Ok(a), Ok(b)) => (a, b),
        _ => panic!(
            "cannot stat {} / {} — run `cargo build` first",
            so.display(),
            src.display()
        ),
    };
    if so_m < src_m {
        panic!(
            "STALE ARTIFACT: {} is older than {}.\n\
             `cargo test` does not rebuild a cdylib-only lib target — run\n\
             `cargo build` (same profile) before `cargo test`, or use\n\
             ./check_all_configs.sh which does it for you.",
            so.display(),
            src.display()
        );
    }
}

pub fn c_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("DIFF_C_SO") {
        return PathBuf::from(p);
    }
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("c_src/build/libtranslated_rust.so")
}

unsafe fn get<T: Copy>(lib: &'static libloading::Library, name: &[u8]) -> T {
    let s: libloading::Symbol<T> = lib
        .get(name)
        .unwrap_or_else(|e| panic!("missing symbol {}: {e}", String::from_utf8_lossy(name)));
    *s
}

fn load(path: &PathBuf, name: &'static str) -> Api {
    unsafe {
        let lib = libloading::Library::new(path)
            .unwrap_or_else(|e| panic!("cannot load {}: {e}", path.display()));
        // Leak so all obtained function pointers stay valid for the whole run.
        let lib: &'static libloading::Library = Box::leak(Box::new(lib));
        Api {
            name,
            rand_seed: get(lib, b"stbds_rand_seed\0"),
            hash_bytes: get(lib, b"stbds_hash_bytes\0"),
            hash_string: get(lib, b"stbds_hash_string\0"),
            arrgrowf: get(lib, b"stbds_arrgrowf\0"),
            arrfreef: get(lib, b"stbds_arrfreef\0"),
            hmfree_func: get(lib, b"stbds_hmfree_func\0"),
            hmget_key: get(lib, b"stbds_hmget_key\0"),
            hmget_key_ts: get(lib, b"stbds_hmget_key_ts\0"),
            hmput_default: get(lib, b"stbds_hmput_default\0"),
            hmput_key: get(lib, b"stbds_hmput_key\0"),
            hmdel_key: get(lib, b"stbds_hmdel_key\0"),
            shmode_func: get(lib, b"stbds_shmode_func\0"),
            stralloc: get(lib, b"stbds_stralloc\0"),
            strreset: get(lib, b"stbds_strreset\0"),
            strkey: get(lib, b"strkey\0"),
            hm_geti: get(lib, b"hm_geti\0"),
        }
    }
}

/// Loads the C reference `.so` and the Rust `.so`.  Returns `(c, rust)`.
pub fn load_both() -> (Api, Api) {
    (load(&c_so_path(), "C"), load(&rust_so_path(), "RUST"))
}

// ---------------------------------------------------------------------------
// Global-state serialisation
// ---------------------------------------------------------------------------

/// Both libraries keep *process-global* mutable state:
///   * `static size_t stbds_hash_seed` (consumed and advanced by every
///     `stbds_make_hash_index`, settable via `stbds_rand_seed`), and
///   * `static char buffer[256]` (returned by `strkey`).
///
/// libtest runs `#[test]`s in parallel threads inside one process, and a `.so`
/// dlopen'ed twice shares one copy of its globals, so any test that depends on
/// that state must hold this lock for the whole scenario — otherwise the C and
/// the Rust library would consume *differently interleaved* seed sequences and
/// the comparison would be meaningless.
pub fn global_lock() -> std::sync::MutexGuard<'static, ()> {
    static LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
    LOCK.lock().unwrap_or_else(|e| e.into_inner())
}

/// Pins the global hash seed of *both* libraries to `seed`.
pub unsafe fn pin_seed(c: &Api, r: &Api, seed: usize) {
    (c.rand_seed)(seed);
    (r.rand_seed)(seed);
}

// ---------------------------------------------------------------------------
// Deterministic RNG (SplitMix64) — reproducible property testing
// ---------------------------------------------------------------------------

pub struct Rng(pub u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Rng(seed ^ 0x9E37_79B9_7F4A_7C15)
    }
    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    pub fn next_usize(&mut self) -> usize {
        self.next_u64() as usize
    }
    pub fn below(&mut self, n: u64) -> u64 {
        if n == 0 {
            0
        } else {
            self.next_u64() % n
        }
    }
    pub fn byte(&mut self) -> u8 {
        (self.next_u64() >> 33) as u8
    }
    pub fn bytes(&mut self, n: usize) -> Vec<u8> {
        (0..n).map(|_| self.byte()).collect()
    }
    pub fn i32(&mut self) -> i32 {
        self.next_u64() as i32
    }
}

// ---------------------------------------------------------------------------
// Pointer helpers (mirror the stb_ds macros)
// ---------------------------------------------------------------------------

/// `STBDS_HASH_TO_ARR`
pub fn hash_to_arr(t: *mut c_void, elemsize: usize) -> *mut c_void {
    (t as *mut u8).wrapping_sub(elemsize) as *mut c_void
}

/// `STBDS_ARR_TO_HASH`
pub fn arr_to_hash(t: *mut c_void, elemsize: usize) -> *mut c_void {
    (t as *mut u8).wrapping_add(elemsize) as *mut c_void
}

/// `stbds_header(t)`
pub unsafe fn header(t: *mut c_void) -> *mut ArrayHeader {
    (t as *mut ArrayHeader).wrapping_sub(1)
}

/// `stbds_temp((t)-1)` for a hash-map pointer
pub unsafe fn hm_temp(t: *mut c_void, elemsize: usize) -> isize {
    (*header(hash_to_arr(t, elemsize))).temp
}

/// `stbds_hmlen(t)`
pub unsafe fn hm_len(t: *mut c_void, elemsize: usize) -> isize {
    if t.is_null() {
        0
    } else {
        (*header(hash_to_arr(t, elemsize))).length as isize - 1
    }
}

/// `stbds_arrlen(a)` on a raw array pointer
pub unsafe fn arr_len(a: *mut c_void) -> isize {
    if a.is_null() {
        0
    } else {
        (*header(a)).length as isize
    }
}

/// `stbds_arrcap(a)` on a raw array pointer
pub unsafe fn arr_cap(a: *mut c_void) -> usize {
    if a.is_null() {
        0
    } else {
        (*header(a)).capacity
    }
}

// ---------------------------------------------------------------------------
// Snapshots — canonical, address-free renderings used for byte-exact compares
// ---------------------------------------------------------------------------

/// How the key is stored inside an element, i.e. how to render it.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum KeyKind {
    /// Key bytes stored inline (`memcpy` / binary mode): compare raw bytes.
    Raw,
    /// Key is a `char *` at offset 0 (string modes): compare the pointed-to
    /// C string, never the address.
    Pointer,
}

fn hexdump(out: &mut String, bytes: &[u8]) {
    for b in bytes {
        let _ = write!(out, "{:02x}", b);
    }
}

extern "C" {
    fn malloc_usable_size(p: *mut c_void) -> usize;
}

/// `#define STBDS_ALIGN_FWD(n,a) (((n) + (a) - 1) & ~((a)-1))`
fn align_fwd(n: usize, a: usize) -> usize {
    n.wrapping_add(a - 1) & !(a - 1)
}

/// Whitebox invariants that the address-free snapshots cannot express.
///
/// A wrong `STBDS_ALIGN_FWD` or a wrong `realloc` size would leave every
/// *comparable* field identical while quietly corrupting the heap, so both are
/// asserted directly, for each library, on every snapshot.
unsafe fn check_allocation_invariants(raw: *mut c_void, elemsize: usize, who: &str) {
    let h = &*header(raw);
    // `stbds_arrgrowf`: realloc(elemsize * capacity + sizeof(stbds_array_header))
    let need = elemsize
        .wrapping_mul(h.capacity)
        .wrapping_add(std::mem::size_of::<ArrayHeader>());
    let got = malloc_usable_size(header(raw) as *mut c_void);
    assert!(
        got >= need,
        "{who}: array allocation too small: usable {got} < required {need} \
         (elemsize={elemsize}, capacity={})",
        h.capacity
    );
    if h.hash_table.is_null() {
        return;
    }
    let ti = &*(h.hash_table as *const HashIndex);
    // `stbds_make_hash_index`: realloc((slot_count>>3)*sizeof(bucket)
    //                                 + sizeof(hash_index) + CACHE_LINE-1)
    let need = (ti.slot_count >> 3)
        .wrapping_mul(std::mem::size_of::<HashBucket>())
        .wrapping_add(std::mem::size_of::<HashIndex>())
        .wrapping_add(64 - 1);
    let got = malloc_usable_size(h.hash_table);
    assert!(
        got >= need,
        "{who}: hash-index allocation too small: usable {got} < required {need} \
         (slot_count={})",
        ti.slot_count
    );
    // `t->storage = (stbds_hash_bucket *) STBDS_ALIGN_FWD((size_t)(t+1), 64)`
    let expect = align_fwd(
        (h.hash_table as usize).wrapping_add(std::mem::size_of::<HashIndex>()),
        64,
    );
    assert_eq!(
        ti.storage as usize, expect,
        "{who}: hash bucket storage is not STBDS_ALIGN_FWD(t+1, 64)"
    );
    assert_eq!(ti.storage as usize % 64, 0, "{who}: storage not 64-byte aligned");
    // and the buckets must fit inside the allocation
    let end = (ti.storage as usize) + (ti.slot_count >> 3) * std::mem::size_of::<HashBucket>();
    assert!(
        end <= (h.hash_table as usize) + got,
        "{who}: buckets run past the end of the hash-index allocation"
    );
}

/// Canonical rendering of a raw array (the pointer returned by `arrgrowf`).
///
/// `payload` is the number of *bytes per element* to include; pass `0` to skip
/// the payload (useful when the elements are uninitialised).
pub unsafe fn snapshot_array(a: *mut c_void, elemsize: usize, payload_elems: usize) -> String {
    let mut s = String::new();
    if a.is_null() {
        return "arr=NULL".into();
    }
    check_allocation_invariants(a, elemsize, "array");
    let h = &*header(a);
    let _ = write!(
        s,
        "arr len={} cap={} temp={} table={}\n",
        h.length,
        h.capacity,
        h.temp,
        if h.hash_table.is_null() { "NULL" } else { "SET" }
    );
    for i in 0..payload_elems {
        let p = (a as *const u8).wrapping_add(i * elemsize);
        let _ = write!(s, "  e[{}]=", i);
        hexdump(&mut s, std::slice::from_raw_parts(p, elemsize));
        s.push('\n');
    }
    s
}

/// Canonical rendering of a hash map (the pointer the `hm*`/`sh*` functions
/// return, i.e. `raw + elemsize`).
///
/// Deliberately excludes every machine address (`hash_table`, `storage`,
/// `temp_key`, key pointers) because those legitimately differ between two
/// independently allocated libraries; everything else — including all bucket
/// hashes and indices, all thresholds and the table seed — is fully
/// deterministic and therefore compared byte-for-byte.
pub unsafe fn snapshot_map(t: *mut c_void, elemsize: usize, kind: KeyKind) -> String {
    let mut s = String::new();
    if t.is_null() {
        return "map=NULL".into();
    }
    let raw = hash_to_arr(t, elemsize);
    check_allocation_invariants(raw, elemsize, "map");
    let h = &*header(raw);
    let _ = write!(
        s,
        "map len={} cap={} temp={} table={}\n",
        h.length,
        h.capacity,
        h.temp,
        if h.hash_table.is_null() { "NULL" } else { "SET" }
    );

    if !h.hash_table.is_null() {
        let ti = &*(h.hash_table as *const HashIndex);
        let _ = write!(
            s,
            "  idx slots={} used={} used_thr={} shrink_thr={} tomb={} tomb_thr={} seed={:#x} log2={}\n",
            ti.slot_count,
            ti.used_count,
            ti.used_count_threshold,
            ti.used_count_shrink_threshold,
            ti.tombstone_count,
            ti.tombstone_count_threshold,
            ti.seed,
            ti.slot_count_log2
        );
        // NOTE: `stbds_hash_index::temp_key` is deliberately NOT part of the
        // snapshot: `stbds_make_hash_index` never initialises it (the struct
        // comes straight out of `realloc`), so its value is indeterminate until
        // a string-mode `hmput_key` writes it.  It is compared explicitly, right
        // after a put, in `tests/strmap.rs`.
        let _ = write!(
            s,
            "  arena remaining={} block={} mode={} storage={}\n",
            ti.string.remaining,
            ti.string.block,
            ti.string.mode,
            if ti.string.storage.is_null() { "NULL" } else { "SET" },
        );
        let nbuckets = ti.slot_count >> 3;
        for b in 0..nbuckets {
            let bk = &*ti.storage.wrapping_add(b);
            let _ = write!(s, "  b[{}] h=", b);
            for j in 0..STBDS_BUCKET_LENGTH {
                let _ = write!(s, "{:016x},", bk.hash[j]);
            }
            let _ = write!(s, " i=");
            for j in 0..STBDS_BUCKET_LENGTH {
                let _ = write!(s, "{},", bk.index[j]);
            }
            s.push('\n');
        }
    }

    for i in 0..h.length {
        let p = (raw as *const u8).wrapping_add(i * elemsize);
        let _ = write!(s, "  e[{}]=", i);
        match kind {
            KeyKind::Raw => hexdump(&mut s, std::slice::from_raw_parts(p, elemsize)),
            KeyKind::Pointer => {
                let kp = *(p as *const *const c_char);
                if kp.is_null() {
                    s.push_str("key=NULL");
                } else {
                    let _ = write!(s, "key={:?}", CStr::from_ptr(kp));
                }
                s.push('|');
                if elemsize > 8 {
                    hexdump(&mut s, std::slice::from_raw_parts(p.wrapping_add(8), elemsize - 8));
                }
            }
        }
        s.push('\n');
    }
    s
}

/// Copy of the map's `stbds_hash_index` (for coverage assertions), or `None`
/// when the map has no hash table yet.
pub unsafe fn map_table(t: *mut c_void, elemsize: usize) -> Option<HashIndex> {
    if t.is_null() {
        return None;
    }
    let h = &*header(hash_to_arr(t, elemsize));
    if h.hash_table.is_null() {
        None
    } else {
        Some(*(h.hash_table as *const HashIndex))
    }
}

/// Canonical rendering of a `stbds_string_arena` plus its whole block chain.
pub unsafe fn snapshot_arena(a: *const StringArena, blocks_to_dump: usize) -> String {
    let mut s = String::new();
    let ar = &*a;
    let _ = write!(
        s,
        "arena remaining={} block={} mode={} storage={}\n",
        ar.remaining,
        ar.block,
        ar.mode,
        if ar.storage.is_null() { "NULL" } else { "SET" }
    );
    // Walk the block list; only the *shape* (chain length) is address-free.
    let mut n = 0usize;
    let mut p = ar.storage as *const *const c_void;
    while !p.is_null() && n < blocks_to_dump {
        n += 1;
        p = *p as *const *const c_void;
    }
    let _ = write!(s, "blocks={}\n", n);
    s
}

/// Reads back the C string at `p` (or `<null>`).
pub unsafe fn cstr(p: *const c_char) -> String {
    if p.is_null() {
        "<null>".into()
    } else {
        CStr::from_ptr(p).to_string_lossy().into_owned()
    }
}

// ---------------------------------------------------------------------------
// Map driver — replays the stb_ds macros faithfully for a given element layout
// ---------------------------------------------------------------------------

#[derive(Clone, Copy)]
pub struct MapCfg {
    pub elemsize: usize,
    pub keysize: usize,
    pub mode: c_int,
    /// `keyoffset` handed to `stbds_hmdel_key` (the macros always pass 0).
    pub del_keyoffset: usize,
    pub kind: KeyKind,
}

impl MapCfg {
    pub fn binary(elemsize: usize, keysize: usize) -> Self {
        MapCfg {
            elemsize,
            keysize,
            mode: STBDS_HM_BINARY,
            del_keyoffset: 0,
            kind: KeyKind::Raw,
        }
    }
    pub fn string(elemsize: usize, mode: c_int) -> Self {
        MapCfg {
            elemsize,
            keysize: 8,
            mode,
            del_keyoffset: 0,
            kind: KeyKind::Pointer,
        }
    }
}

/// `hmput(t, key, value)` for a binary map: calls `stbds_hmput_key` and then
/// writes the *whole* element (key bytes are already copied by the library; the
/// caller fills everything after `keysize`) so that no byte is ever
/// uninitialised and the snapshots stay deterministic.
pub unsafe fn map_put_binary(
    api: &Api,
    t: *mut c_void,
    cfg: &MapCfg,
    key: &[u8],
    payload: &[u8],
) -> *mut c_void {
    assert_eq!(key.len(), cfg.keysize);
    let mut k = key.to_vec();
    let t = (api.hmput_key)(
        t,
        cfg.elemsize,
        k.as_mut_ptr() as *mut c_void,
        cfg.keysize,
        cfg.mode,
    );
    let idx = hm_temp(t, cfg.elemsize);
    let e = (t as *mut u8).wrapping_offset(idx * cfg.elemsize as isize);
    // key (idempotent — mirrors `(t)[temp].key = k`)
    ptr::copy_nonoverlapping(key.as_ptr(), e, cfg.keysize);
    // value / padding (mirrors `(t)[temp].value = v`)
    if cfg.elemsize > cfg.keysize {
        let n = cfg.elemsize - cfg.keysize;
        let mut v = vec![0u8; n];
        for (i, b) in v.iter_mut().enumerate() {
            *b = payload.get(i).copied().unwrap_or(0);
        }
        ptr::copy_nonoverlapping(v.as_ptr(), e.wrapping_add(cfg.keysize), n);
    }
    t
}

/// `hmput(t, key, value)` for a string map: the library stores the key itself
/// (pointer / strdup / arena), the caller only writes the trailing payload.
pub unsafe fn map_put_string(
    api: &Api,
    t: *mut c_void,
    cfg: &MapCfg,
    key: *mut c_char,
    payload: &[u8],
) -> *mut c_void {
    let t = (api.hmput_key)(t, cfg.elemsize, key as *mut c_void, cfg.keysize, cfg.mode);
    let idx = hm_temp(t, cfg.elemsize);
    let e = (t as *mut u8).wrapping_offset(idx * cfg.elemsize as isize);
    if cfg.elemsize > 8 {
        let n = cfg.elemsize - 8;
        let mut v = vec![0u8; n];
        for (i, b) in v.iter_mut().enumerate() {
            *b = payload.get(i).copied().unwrap_or(0);
        }
        ptr::copy_nonoverlapping(v.as_ptr(), e.wrapping_add(8), n);
    }
    t
}

/// `hmgeti(t,k)` — returns `(new_t, temp)`
pub unsafe fn map_geti(api: &Api, t: *mut c_void, cfg: &MapCfg, key: &mut [u8]) -> (*mut c_void, isize) {
    let t = (api.hmget_key)(
        t,
        cfg.elemsize,
        key.as_mut_ptr() as *mut c_void,
        cfg.keysize,
        cfg.mode,
    );
    (t, hm_temp(t, cfg.elemsize))
}

/// `hmgeti_ts(t,k,temp)` — returns `(new_t, temp)`
pub unsafe fn map_geti_ts(
    api: &Api,
    t: *mut c_void,
    cfg: &MapCfg,
    key: &mut [u8],
) -> (*mut c_void, isize) {
    let mut temp: isize = 0x5555_5555;
    let t = (api.hmget_key_ts)(
        t,
        cfg.elemsize,
        key.as_mut_ptr() as *mut c_void,
        cfg.keysize,
        &mut temp,
        cfg.mode,
    );
    (t, temp)
}

/// `hmdel(t,k)` — returns `(new_t, temp_or_0)`
pub unsafe fn map_del(api: &Api, t: *mut c_void, cfg: &MapCfg, key: &mut [u8]) -> (*mut c_void, isize) {
    let t = (api.hmdel_key)(
        t,
        cfg.elemsize,
        key.as_mut_ptr() as *mut c_void,
        cfg.keysize,
        cfg.del_keyoffset,
        cfg.mode,
    );
    let r = if t.is_null() { 0 } else { hm_temp(t, cfg.elemsize) };
    (t, r)
}

/// `hmfree(t)`
pub unsafe fn map_free(api: &Api, t: *mut c_void, elemsize: usize) {
    if !t.is_null() {
        (api.hmfree_func)(hash_to_arr(t, elemsize), elemsize);
    }
}

// ---------------------------------------------------------------------------
// Differential assertion helper
// ---------------------------------------------------------------------------

#[track_caller]
pub fn diff_eq(ctx: &str, c: &str, r: &str) {
    if c != r {
        let mut msg = format!("DIVERGENCE in {ctx}\n");
        let cl: Vec<&str> = c.lines().collect();
        let rl: Vec<&str> = r.lines().collect();
        for i in 0..cl.len().max(rl.len()) {
            let a = cl.get(i).copied().unwrap_or("<missing>");
            let b = rl.get(i).copied().unwrap_or("<missing>");
            if a != b {
                let _ = writeln!(msg, "  line {i}:\n    C   : {a}\n    RUST: {b}");
            }
        }
        panic!("{msg}");
    }
}

#[track_caller]
pub fn diff_eq_val<T: PartialEq + std::fmt::Debug>(ctx: &str, c: T, r: T) {
    assert_eq!(c, r, "DIVERGENCE in {ctx} (C vs RUST)");
}
