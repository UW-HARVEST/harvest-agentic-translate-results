//! Differential-test harness.
//!
//! Loads BOTH the C shared object (built by `c_src/CMakeLists.txt`) and the
//! Rust `cdylib` through `libloading` and calls every function only through
//! its exported symbol, exactly as an external consumer would.
//!
//! Nothing in `src/lib.rs` is ever called directly.

#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::ffi::{c_char, c_int, c_void};
use std::path::PathBuf;
use std::sync::{Mutex, MutexGuard, OnceLock};

// ---------------------------------------------------------------------------
// Constants mirrored from c_src/src/lib.c
// ---------------------------------------------------------------------------

pub const HM_BINARY: c_int = 0;
pub const HM_STRING: c_int = 1;

pub const SH_NONE: c_int = 0;
pub const SH_DEFAULT: c_int = 1;
pub const SH_STRDUP: c_int = 2;
pub const SH_ARENA: c_int = 3;

pub const INDEX_EMPTY: isize = -1;
pub const INDEX_DELETED: isize = -2;
pub const HASH_EMPTY: usize = 0;
pub const HASH_DELETED: usize = 1;

pub const HDR_SIZE: usize = 32;
pub const BUCKET_LEN: usize = 8;
pub const BUCKET_SHIFT: usize = 3;
pub const BUCKET_MASK: usize = 7;

pub const DEFAULT_SEED: usize = 0x31415926;

pub const ARENA_BLOCKSIZE_MIN: usize = 512;
pub const ARENA_BLOCKSIZE_MAX: usize = 1 << 20;

// ---------------------------------------------------------------------------
// Layout mirrors (sizes verified below: 32 / 128 / 24 / 104 bytes)
// ---------------------------------------------------------------------------

#[repr(C)]
#[derive(Debug)]
pub struct ArrayHeader {
    pub length: usize,
    pub capacity: usize,
    pub hash_table: *mut c_void,
    pub temp: isize,
}

#[repr(C)]
pub struct HashBucket {
    pub hash: [usize; BUCKET_LEN],
    pub index: [isize; BUCKET_LEN],
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
        StringArena { storage: std::ptr::null_mut(), remaining: 0, block: 0, mode: 0 }
    }
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

const _: () = assert!(std::mem::size_of::<ArrayHeader>() == 32);
const _: () = assert!(std::mem::size_of::<HashBucket>() == 128);
const _: () = assert!(std::mem::size_of::<StringArena>() == 24);
const _: () = assert!(std::mem::size_of::<HashIndex>() == 104);

// ---------------------------------------------------------------------------
// The exported API, resolved by symbol name
// ---------------------------------------------------------------------------

pub type FnArrGrowf = unsafe extern "C" fn(*mut c_void, usize, usize, usize) -> *mut c_void;
pub type FnArrFreef = unsafe extern "C" fn(*mut c_void);
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
pub type FnHmGeti = unsafe extern "C" fn(c_int);

pub struct Api {
    pub tag: &'static str,
    pub path: String,
    pub arrgrowf: FnArrGrowf,
    pub arrfreef: FnArrFreef,
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
    pub hm_geti: FnHmGeti,
}

unsafe fn sym<T: Copy>(lib: &'static Library, name: &[u8]) -> T {
    unsafe {
        let s: Symbol<T> = lib
            .get(name)
            .unwrap_or_else(|e| panic!("missing symbol {}: {e}", String::from_utf8_lossy(name)));
        *s
    }
}

fn load(tag: &'static str, path: PathBuf) -> Api {
    let lib: &'static Library = Box::leak(Box::new(unsafe {
        Library::new(&path).unwrap_or_else(|e| panic!("cannot load {}: {e}", path.display()))
    }));
    unsafe {
        Api {
            tag,
            path: path.display().to_string(),
            arrgrowf: sym(lib, b"stbds_arrgrowf\0"),
            arrfreef: sym(lib, b"stbds_arrfreef\0"),
            rand_seed: sym(lib, b"stbds_rand_seed\0"),
            hash_string: sym(lib, b"stbds_hash_string\0"),
            hash_bytes: sym(lib, b"stbds_hash_bytes\0"),
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
            hm_geti: sym(lib, b"hm_geti\0"),
        }
    }
}

fn c_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("C_SO") {
        return PathBuf::from(p);
    }
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("workspace parent")
        .to_path_buf();
    let build = root.join("c_src").join("build");
    let mut found: Vec<PathBuf> = Vec::new();
    if let Ok(rd) = std::fs::read_dir(&build) {
        for e in rd.flatten() {
            let p = e.path();
            let name = p.file_name().unwrap().to_string_lossy().to_string();
            if name.starts_with("lib") && name.ends_with(".so") {
                found.push(p);
            }
        }
    }
    found.sort();
    found.pop().unwrap_or_else(|| {
        panic!(
            "no C .so under {} — build it with:\n  cd c_src && mkdir -p build && cd build && \
             cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
            build.display()
        )
    })
}

fn rust_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("RUST_SO") {
        return PathBuf::from(p);
    }
    // <target>/<profile>/deps/<testbin> -> <target>/<profile>/libhm_geti_lib.so
    let exe = std::env::current_exe().expect("current_exe");
    let mut cands = Vec::new();
    if let Some(profile_dir) = exe.parent().and_then(|d| d.parent()) {
        cands.push(profile_dir.join("libhm_geti_lib.so"));
    }
    let target = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target");
    cands.push(target.join("release").join("libhm_geti_lib.so"));
    cands.push(target.join("debug").join("libhm_geti_lib.so"));
    for c in &cands {
        if c.exists() {
            return c.clone();
        }
    }
    panic!("no Rust cdylib found; tried {cands:?}");
}

static LIBS: OnceLock<(Api, Api)> = OnceLock::new();
static LOCK: Mutex<()> = Mutex::new(());

/// (C api, Rust api)
pub fn apis() -> (&'static Api, &'static Api) {
    let (c, r) = LIBS.get_or_init(|| (load("C", c_so_path()), load("RUST", rust_so_path())));
    (c, r)
}

/// Serialises tests: both libraries carry a mutable global hash seed, so every
/// scenario runs one at a time with an explicitly reset seed.
pub struct Pair {
    pub c: &'static Api,
    pub r: &'static Api,
    _g: MutexGuard<'static, ()>,
}

/// Take the global lock and force both libraries to the same hash seed.
pub fn seeded(seed: usize) -> Pair {
    let g = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let (c, r) = apis();
    unsafe {
        (c.rand_seed)(seed);
        (r.rand_seed)(seed);
    }
    Pair { c, r, _g: g }
}

impl Pair {
    pub fn both(&self) -> [&'static Api; 2] {
        [self.c, self.r]
    }
    /// Reset both libraries' global seed mid-test.
    pub fn reseed(&self, seed: usize) {
        unsafe {
            (self.c.rand_seed)(seed);
            (self.r.rand_seed)(seed);
        }
    }
}

// ---------------------------------------------------------------------------
// Pointer helpers (identical to the C macros)
// ---------------------------------------------------------------------------

/// `stbds_header(raw)`
pub unsafe fn header(raw: *mut c_void) -> *mut ArrayHeader {
    unsafe { (raw as *mut u8).sub(HDR_SIZE) as *mut ArrayHeader }
}

/// header of a *hash-map* pointer `h` (`h - elemsize` is the raw array)
pub unsafe fn map_header(h: *mut c_void, elemsize: usize) -> *mut ArrayHeader {
    unsafe { header((h as *mut u8).sub(elemsize) as *mut c_void) }
}

/// `stbds_temp((t)-1)` for a hash-map pointer
pub unsafe fn map_temp(h: *mut c_void, elemsize: usize) -> isize {
    unsafe { (*map_header(h, elemsize)).temp }
}

pub unsafe fn map_len(h: *mut c_void, elemsize: usize) -> usize {
    unsafe { (*map_header(h, elemsize)).length }
}

pub unsafe fn map_table(h: *mut c_void, elemsize: usize) -> *mut HashIndex {
    unsafe { (*map_header(h, elemsize)).hash_table as *mut HashIndex }
}

/// raw array base for a hash-map pointer
pub unsafe fn map_raw(h: *mut c_void, elemsize: usize) -> *mut u8 {
    unsafe { (h as *mut u8).sub(elemsize) }
}

pub unsafe fn map_cap(h: *mut c_void, elemsize: usize) -> usize {
    unsafe {
        if h.is_null() { 0 } else { (*map_header(h, elemsize)).capacity }
    }
}

/// Pointer-identity report that is only assertive where the C *guarantees* the
/// pointer is unchanged.  When the array was reallocated, `realloc` is free to
/// move the block or not, so identity must not be compared across libraries.
fn ident(prev: *mut c_void, now: *mut c_void, cap_before: usize, cap_after: usize) -> String {
    if prev.is_null() {
        "ptr=fresh".into()
    } else if cap_before == cap_after {
        format!("ptr_same={}", prev == now)
    } else {
        "ptr=grown".into()
    }
}

pub unsafe fn map_string_mode(h: *mut c_void, elemsize: usize) -> u8 {
    unsafe {
        if h.is_null() {
            return 0;
        }
        let t = map_table(h, elemsize);
        if t.is_null() { 0 } else { (*t).string.mode }
    }
}

// ---------------------------------------------------------------------------
// Structural snapshots
// ---------------------------------------------------------------------------

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum KeyKind {
    /// keys are raw bytes inside the element
    Binary,
    /// the element holds a `char *` at `keyoffset`; compare the pointee string
    StrPtr { keyoffset: usize },
}

pub fn hex(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        s.push_str(&format!("{b:02x}"));
    }
    s
}

pub unsafe fn cstr_bytes(p: *const c_char) -> Option<Vec<u8>> {
    unsafe {
        if p.is_null() {
            None
        } else {
            Some(std::ffi::CStr::from_ptr(p).to_bytes().to_vec())
        }
    }
}

pub unsafe fn cstr_repr(p: *const c_char) -> String {
    unsafe {
        match cstr_bytes(p) {
            None => "NULL".to_string(),
            Some(b) => format!("s<{}>", hex(&b)),
        }
    }
}

/// One element rendered so that only *content* (never addresses) is compared.
pub unsafe fn elem_repr(e: *const u8, elemsize: usize, kind: KeyKind) -> String {
    unsafe {
        match kind {
            KeyKind::Binary => hex(std::slice::from_raw_parts(e, elemsize)),
            KeyKind::StrPtr { keyoffset } => {
                assert!(elemsize >= keyoffset + 8, "StrPtr needs elemsize >= keyoffset+8");
                let kp = *(e.add(keyoffset) as *const *const c_char);
                let mut rest = Vec::new();
                rest.extend_from_slice(std::slice::from_raw_parts(e, keyoffset));
                rest.extend_from_slice(std::slice::from_raw_parts(
                    e.add(keyoffset + 8),
                    elemsize - keyoffset - 8,
                ));
                format!("key={} rest={}", cstr_repr(kp), hex(&rest))
            }
        }
    }
}

/// Full structural dump of a map: header, every element (keys compared by
/// pointee string when `kind == StrPtr`), and the whole `stbds_hash_index`
/// including every bucket.  Raw pointer values are deliberately excluded (the
/// two libraries allocate at different addresses); everything else is compared
/// exactly.
pub unsafe fn snap_map(h: *mut c_void, elemsize: usize, kind: KeyKind) -> Vec<String> {
    unsafe {
        let mut out = Vec::new();
        if h.is_null() {
            out.push("map=NULL".into());
            return out;
        }
        let raw = map_raw(h, elemsize);
        let hdr = &*header(raw as *mut c_void);
        out.push(format!("hdr.length={}", hdr.length));
        out.push(format!("hdr.capacity={}", hdr.capacity));
        out.push(format!("hdr.temp={}", hdr.temp));
        out.push(format!("hdr.has_table={}", !hdr.hash_table.is_null()));
        for i in 0..hdr.length {
            out.push(format!("elem[{i}] {}", elem_repr(raw.add(i * elemsize), elemsize, kind)));
        }
        let t = hdr.hash_table as *const HashIndex;
        if t.is_null() {
            out.push("table=NULL".into());
            return out;
        }
        let t = &*t;
        out.push(format!("t.slot_count={}", t.slot_count));
        out.push(format!("t.used_count={}", t.used_count));
        out.push(format!("t.used_count_threshold={}", t.used_count_threshold));
        out.push(format!("t.used_count_shrink_threshold={}", t.used_count_shrink_threshold));
        out.push(format!("t.tombstone_count={}", t.tombstone_count));
        out.push(format!("t.tombstone_count_threshold={}", t.tombstone_count_threshold));
        out.push(format!("t.seed={:#x}", t.seed));
        out.push(format!("t.slot_count_log2={}", t.slot_count_log2));
        out.push(format!("t.string.remaining={}", t.string.remaining));
        out.push(format!("t.string.block={}", t.string.block));
        out.push(format!("t.string.mode={}", t.string.mode));
        out.push(format!("t.string.has_storage={}", !t.string.storage.is_null()));
        // the invariant the C asserts on construction
        out.push(format!(
            "t.invariant_ok={}",
            t.used_count_threshold + t.tombstone_count_threshold < t.slot_count
        ));
        let nbuckets = t.slot_count >> BUCKET_SHIFT;
        for b in 0..nbuckets {
            let bk = &*t.storage.add(b);
            let mut line = format!("bucket[{b}] h=[");
            for i in 0..BUCKET_LEN {
                if i > 0 {
                    line.push(',');
                }
                line.push_str(&format!("{:#x}", bk.hash[i]));
            }
            line.push_str("] i=[");
            for i in 0..BUCKET_LEN {
                if i > 0 {
                    line.push(',');
                }
                line.push_str(&format!("{}", bk.index[i]));
            }
            line.push(']');
            out.push(line);
        }
        out
    }
}

/// Fast FNV-1a digest over exactly the same information `snap_map` prints, for
/// long scripts / big maps where keeping every line would be too slow.
pub unsafe fn digest_map(h: *mut c_void, elemsize: usize, kind: KeyKind) -> u64 {
    unsafe {
        let mut acc: u64 = 0xcbf29ce484222325;
        let mut feed = |bytes: &[u8]| {
            for b in bytes {
                acc ^= *b as u64;
                acc = acc.wrapping_mul(0x100000001b3);
            }
        };
        if h.is_null() {
            feed(b"NULL");
            return acc;
        }
        let raw = map_raw(h, elemsize);
        let hdr = &*header(raw as *mut c_void);
        feed(&hdr.length.to_le_bytes());
        feed(&hdr.capacity.to_le_bytes());
        feed(&hdr.temp.to_le_bytes());
        feed(&[hdr.hash_table.is_null() as u8]);
        for i in 0..hdr.length {
            let e = raw.add(i * elemsize);
            match kind {
                KeyKind::Binary => feed(std::slice::from_raw_parts(e, elemsize)),
                KeyKind::StrPtr { keyoffset } => {
                    let kp = *(e.add(keyoffset) as *const *const c_char);
                    match cstr_bytes(kp) {
                        None => feed(b"<null-key>"),
                        Some(b) => {
                            feed(&b);
                            feed(&[0]);
                        }
                    }
                    feed(std::slice::from_raw_parts(e, keyoffset));
                    feed(std::slice::from_raw_parts(
                        e.add(keyoffset + 8),
                        elemsize - keyoffset - 8,
                    ));
                }
            }
        }
        let t = hdr.hash_table as *const HashIndex;
        if t.is_null() {
            feed(b"<no-table>");
            return acc;
        }
        let t = &*t;
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
            feed(&v.to_le_bytes());
        }
        feed(&[t.string.block, t.string.mode, t.string.storage.is_null() as u8]);
        for b in 0..(t.slot_count >> BUCKET_SHIFT) {
            let bk = &*t.storage.add(b);
            for i in 0..BUCKET_LEN {
                feed(&bk.hash[i].to_le_bytes());
                feed(&bk.index[i].to_le_bytes());
            }
        }
        acc
    }
}

/// `stbds_temp_key(a)` == `*(char**)hash_table` == `table->temp_key`,
/// rendered as its pointee string.  Only ever read at points where the C
/// guarantees it was just written (a brand new insert with
/// `string.mode in {DEFAULT, STRDUP, ARENA}`); `stbds_make_hash_index` leaves
/// the field uninitialised, so reading it elsewhere is not meaningful.
pub unsafe fn snap_temp_key(h: *mut c_void, elemsize: usize) -> String {
    unsafe {
        let t = map_table(h, elemsize);
        if t.is_null() {
            return "temp_key=<no table>".into();
        }
        format!("temp_key={}", cstr_repr((*t).temp_key))
    }
}

pub unsafe fn snap_arena(a: *const StringArena) -> Vec<String> {
    unsafe {
        let a = &*a;
        vec![
            format!("arena.remaining={}", a.remaining),
            format!("arena.block={}", a.block),
            format!("arena.mode={}", a.mode),
            format!("arena.has_storage={}", !a.storage.is_null()),
        ]
    }
}

// ---------------------------------------------------------------------------
// Trace comparison
// ---------------------------------------------------------------------------

pub fn assert_traces_eq(ctx: &str, c: &[String], r: &[String]) {
    if c == r {
        return;
    }
    let n = c.len().min(r.len());
    let mut first = n;
    for i in 0..n {
        if c[i] != r[i] {
            first = i;
            break;
        }
    }
    let lo = first.saturating_sub(12);
    let mut msg = format!(
        "\n=== DIVERGENCE ({ctx}) ===\nfirst differing line: {first} (C has {} lines, RUST has {})\n",
        c.len(),
        r.len()
    );
    for i in lo..first {
        msg.push_str(&format!("  [{i}] both: {}\n", c[i]));
    }
    msg.push_str(&format!(
        "> [{first}] C   : {}\n",
        c.get(first).map(|s| s.as_str()).unwrap_or("<end of trace>")
    ));
    msg.push_str(&format!(
        "> [{first}] RUST: {}\n",
        r.get(first).map(|s| s.as_str()).unwrap_or("<end of trace>")
    ));
    for i in (first + 1)..(first + 6).min(n) {
        msg.push_str(&format!("  [{i}] C   : {}\n", c[i]));
        msg.push_str(&format!("  [{i}] RUST: {}\n", r[i]));
    }
    panic!("{msg}");
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (SplitMix64) — fixed seeds everywhere
// ---------------------------------------------------------------------------

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Rng(seed.wrapping_add(0x9E3779B97F4A7C15))
    }
    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E3779B97F4A7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D049BB133111EB);
        z ^ (z >> 31)
    }
    pub fn below(&mut self, n: u64) -> u64 {
        assert!(n > 0);
        self.next_u64() % n
    }
    pub fn bytes(&mut self, n: usize) -> Vec<u8> {
        (0..n).map(|_| (self.next_u64() >> 13) as u8).collect()
    }
    pub fn u8(&mut self) -> u8 {
        (self.next_u64() >> 11) as u8
    }
    /// NUL-free byte
    pub fn str_byte(&mut self) -> u8 {
        let b = self.u8();
        if b == 0 { 0x41 } else { b }
    }
    /// NUL-terminated random string of `n` payload bytes
    pub fn nul_free(&mut self, n: usize) -> Vec<u8> {
        let mut v: Vec<u8> = (0..n).map(|_| self.str_byte()).collect();
        v.push(0);
        v
    }
}

// ---------------------------------------------------------------------------
// Map script runner
// ---------------------------------------------------------------------------

#[derive(Clone, Debug)]
pub enum Op {
    /// `hmput_key(h, elemsize, key, keysize, mode)`, then the `hmput` macro's
    /// `(t)[temp].key = k; (t)[temp].value = v;`
    Put { key: Vec<u8>, payload: Vec<u8> },
    /// `hmget_key(...)`
    Get { key: Vec<u8> },
    /// `hmget_key_ts(...)`
    GetTs { key: Vec<u8> },
    /// `hmdel_key(..., keyoffset, mode)`
    Del { key: Vec<u8>, keyoffset: usize },
    /// `hmput_default(...)`, then `(t)[-1].value = v`
    PutDefault { payload: Vec<u8> },
    /// `hmfree_func(h - elemsize, elemsize)`
    Free,
    /// `shmode_func(elemsize, sh_mode)` — like the `sh_new_arena`/`sh_new_strdup`
    /// macros this replaces the (empty) map, so it may only run first
    ShMode { sh_mode: c_int },
}

pub fn put(key: &[u8], payload: &[u8]) -> Op {
    Op::Put { key: key.to_vec(), payload: payload.to_vec() }
}
pub fn get(key: &[u8]) -> Op {
    Op::Get { key: key.to_vec() }
}
pub fn get_ts(key: &[u8]) -> Op {
    Op::GetTs { key: key.to_vec() }
}
pub fn del(key: &[u8]) -> Op {
    Op::Del { key: key.to_vec(), keyoffset: 0 }
}
pub fn del_off(key: &[u8], keyoffset: usize) -> Op {
    Op::Del { key: key.to_vec(), keyoffset }
}

#[derive(Clone, Copy, Debug)]
pub struct MapCfg {
    pub elemsize: usize,
    pub keysize: usize,
    /// `STBDS_HM_*` (possibly an out-of-range value, on purpose)
    pub mode: c_int,
    /// where the payload is written inside an element (`keysize` for binary
    /// maps, 8 for `char *`-keyed maps)
    pub payload_off: usize,
    pub kind: KeyKind,
    /// digest per op instead of a full line-by-line snapshot (for big scripts)
    pub digest: bool,
    /// also compare `table->temp_key` after *every* put, not just after a new
    /// insert.  Only safe when the script guarantees the table is never rebuilt
    /// (`stbds_make_hash_index` leaves `temp_key` uninitialised).
    pub temp_key_always: bool,
}

impl MapCfg {
    pub fn binary(elemsize: usize, keysize: usize) -> Self {
        MapCfg {
            elemsize,
            keysize,
            mode: HM_BINARY,
            payload_off: keysize.min(elemsize),
            kind: KeyKind::Binary,
            digest: false,
            temp_key_always: false,
        }
    }
    pub fn string(elemsize: usize, mode: c_int) -> Self {
        MapCfg {
            elemsize,
            keysize: 8,
            mode,
            payload_off: 8,
            kind: KeyKind::StrPtr { keyoffset: 0 },
            digest: false,
            temp_key_always: false,
        }
    }
    pub fn with_mode(mut self, mode: c_int) -> Self {
        self.mode = mode;
        self
    }
    pub fn digested(mut self) -> Self {
        self.digest = true;
        self
    }
    pub fn always_temp_key(mut self) -> Self {
        self.temp_key_always = true;
        self
    }
}

/// Writes the element's payload region *in full* (cycling `payload`, zeros if
/// empty).  Leaving any byte of a live element uninitialised would compare
/// uninitialised heap between the two libraries.
unsafe fn write_payload(elem: *mut u8, elemsize: usize, payload_off: usize, payload: &[u8]) {
    unsafe {
        if elemsize <= payload_off {
            return;
        }
        let want = elemsize - payload_off;
        let mut buf = vec![0u8; want];
        if !payload.is_empty() {
            for i in 0..want {
                buf[i] = payload[i % payload.len()];
            }
        }
        std::ptr::copy_nonoverlapping(buf.as_ptr(), elem.add(payload_off), want);
    }
}

/// Runs the op script against one library, returning a full trace.
///
/// Key buffers are heap-allocated and kept alive until the script finishes,
/// because `STBDS_SH_DEFAULT` stores the caller's pointer inside the map.
pub fn run_script(api: &Api, cfg: MapCfg, ops: &[Op]) -> Vec<String> {
    let mut trace: Vec<String> = Vec::new();
    let mut keep: Vec<Box<[u8]>> = Vec::new();
    let mut h: *mut c_void = std::ptr::null_mut();
    let es = cfg.elemsize;

    unsafe {
        for (n, op) in ops.iter().enumerate() {
            let prev = h;
            match op {
                Op::ShMode { sh_mode } => {
                    assert!(h.is_null(), "ShMode only on a fresh map");
                    h = (api.shmode_func)(es, *sh_mode);
                    trace.push(format!("[{n}] shmode({sh_mode}) null={}", h.is_null()));
                }
                Op::Put { key, payload } => {
                    let mut kb: Box<[u8]> = key.clone().into_boxed_slice();
                    let kp = kb.as_mut_ptr() as *mut c_void;
                    keep.push(kb);
                    let len_before = if h.is_null() { 0 } else { map_len(h, es) };
                    let cap_before = map_cap(h, es);
                    h = (api.hmput_key)(h, es, kp, cfg.keysize, cfg.mode);
                    let idx = map_temp(h, es);
                    trace.push(format!(
                        "[{n}] put temp={idx} {}",
                        ident(prev, h, cap_before, map_cap(h, es))
                    ));
                    // (t)[temp].key = k; (t)[temp].value = v;
                    let raw = map_raw(h, es);
                    let elem = raw.offset((idx + 1) * es as isize);
                    if cfg.kind == KeyKind::Binary && cfg.keysize > 0 {
                        std::ptr::copy_nonoverlapping(key.as_ptr(), elem, cfg.keysize.min(es));
                    }
                    write_payload(elem, es, cfg.payload_off, payload);
                    let smode = map_string_mode(h, es);
                    if (map_len(h, es) > len_before || cfg.temp_key_always)
                        && (1..=3).contains(&smode)
                    {
                        trace.push(snap_temp_key(h, es));
                    }
                }
                Op::Get { key } => {
                    let mut kb: Box<[u8]> = key.clone().into_boxed_slice();
                    let kp = kb.as_mut_ptr() as *mut c_void;
                    keep.push(kb);
                    let cap_before = map_cap(h, es);
                    h = (api.hmget_key)(h, es, kp, cfg.keysize, cfg.mode);
                    let idx = map_temp(h, es);
                    trace.push(format!(
                        "[{n}] get temp={idx} {}",
                        ident(prev, h, cap_before, map_cap(h, es))
                    ));
                    if idx >= 0 {
                        let elem = map_raw(h, es).offset((idx + 1) * es as isize);
                        trace.push(format!("     hit {}", elem_repr(elem, es, cfg.kind)));
                    }
                }
                Op::GetTs { key } => {
                    let mut kb: Box<[u8]> = key.clone().into_boxed_slice();
                    let kp = kb.as_mut_ptr() as *mut c_void;
                    keep.push(kb);
                    let mut t: isize = 0x7f7f_7f7f;
                    let cap_before = map_cap(h, es);
                    h = (api.hmget_key_ts)(h, es, kp, cfg.keysize, &mut t, cfg.mode);
                    trace.push(format!(
                        "[{n}] get_ts out={t} hdr_temp={} {}",
                        map_temp(h, es),
                        ident(prev, h, cap_before, map_cap(h, es))
                    ));
                    if t >= 0 {
                        let elem = map_raw(h, es).offset((t + 1) * es as isize);
                        trace.push(format!("     hit {}", elem_repr(elem, es, cfg.kind)));
                    }
                }
                Op::Del { key, keyoffset } => {
                    let mut kb: Box<[u8]> = key.clone().into_boxed_slice();
                    let kp = kb.as_mut_ptr() as *mut c_void;
                    keep.push(kb);
                    let cap_before = map_cap(h, es);
                    h = (api.hmdel_key)(h, es, kp, cfg.keysize, *keyoffset, cfg.mode);
                    if h.is_null() {
                        trace.push(format!("[{n}] del ret=NULL"));
                    } else {
                        // hmdel_key never reallocates the element array
                        trace.push(format!(
                            "[{n}] del temp={} {}",
                            map_temp(h, es),
                            ident(prev, h, cap_before, map_cap(h, es))
                        ));
                    }
                }
                Op::PutDefault { payload } => {
                    let cap_before = map_cap(h, es);
                    h = (api.hmput_default)(h, es);
                    trace.push(format!(
                        "[{n}] put_default {}",
                        ident(prev, h, cap_before, map_cap(h, es))
                    ));
                    write_payload(map_raw(h, es), es, cfg.payload_off, payload);
                }
                Op::Free => {
                    if !h.is_null() {
                        (api.hmfree_func)(map_raw(h, es) as *mut c_void, es);
                    }
                    h = std::ptr::null_mut();
                    trace.push(format!("[{n}] free"));
                    keep.clear();
                }
            }
            if cfg.digest {
                trace.push(format!("  d={:#018x}", digest_map(h, es, cfg.kind)));
            } else {
                trace.extend(snap_map(h, es, cfg.kind));
            }
        }
        // final full snapshot even in digest mode, then release
        if cfg.digest {
            trace.extend(snap_map(h, es, cfg.kind));
        }
        if !h.is_null() {
            (api.hmfree_func)(map_raw(h, es) as *mut c_void, es);
        }
    }
    drop(keep);
    trace
}

/// Run one script against both libraries (each starting from `seed`) and
/// assert the traces are identical.
pub fn diff_script(ctx: &str, seed: usize, cfg: MapCfg, ops: &[Op]) {
    let p = seeded(seed);
    let tc = run_script(p.c, cfg, ops);
    p.reseed(seed);
    let tr = run_script(p.r, cfg, ops);
    assert_traces_eq(&format!("{ctx} seed={seed:#x} cfg={cfg:?}"), &tc, &tr);
}
