//! Shared differential-test harness.
//!
//! Loads BOTH the C shared object and the Rust `cdylib` through `libloading`
//! and calls every entry point exclusively through its exported C symbol, so
//! the `#[no_mangle] extern "C"` wrappers are part of what is under test.

#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::ffi::c_void;
use std::os::raw::{c_char, c_int};
use std::path::PathBuf;

// ---------------------------------------------------------------------------
// C-side constants (mirrors of the ones in c_src/src/lib.c)
// ---------------------------------------------------------------------------

pub const STBDS_HM_BINARY: c_int = 0;
pub const STBDS_HM_STRING: c_int = 1;
pub const STBDS_HM_PTR_TO_STRING: c_int = 2;

pub const STBDS_SH_NONE: c_int = 0;
pub const STBDS_SH_DEFAULT: c_int = 1;
pub const STBDS_SH_STRDUP: c_int = 2;
pub const STBDS_SH_ARENA: c_int = 3;

pub const STBDS_BUCKET_LENGTH: usize = 8;
pub const STBDS_BUCKET_SHIFT: usize = 3;
pub const STBDS_BUCKET_MASK: usize = 7;

pub const DEFAULT_SEED: usize = 0x3141_5926;

// ---------------------------------------------------------------------------
// ABI-identical mirrors of the C structs, used only to *read* state back out
// ---------------------------------------------------------------------------

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ArrayHeader {
    pub length: usize,
    pub capacity: usize,
    pub hash_table: *mut c_void,
    pub temp: isize,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
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

#[repr(C)]
#[derive(Clone, Copy)]
pub struct StringBlock {
    pub next: *mut StringBlock,
    pub storage: [c_char; 8],
}

// ---------------------------------------------------------------------------
// Function-pointer types
// ---------------------------------------------------------------------------

pub type FnArrGrowF =
    unsafe extern "C" fn(*mut c_void, usize, usize, usize) -> *mut c_void;
pub type FnArrFreeF = unsafe extern "C" fn(*mut c_void);
pub type FnRandSeed = unsafe extern "C" fn(usize);
pub type FnHashString = unsafe extern "C" fn(*mut c_char, usize) -> usize;
pub type FnHashBytes = unsafe extern "C" fn(*mut c_void, usize, usize) -> usize;
pub type FnHmFreeFunc = unsafe extern "C" fn(*mut c_void, usize);
pub type FnHmGetKeyTs = unsafe extern "C" fn(
    *mut c_void,
    usize,
    *mut c_void,
    usize,
    *mut isize,
    c_int,
) -> *mut c_void;
pub type FnHmGetKey =
    unsafe extern "C" fn(*mut c_void, usize, *mut c_void, usize, c_int) -> *mut c_void;
pub type FnHmPutDefault = unsafe extern "C" fn(*mut c_void, usize) -> *mut c_void;
pub type FnHmPutKey =
    unsafe extern "C" fn(*mut c_void, usize, *mut c_void, usize, c_int) -> *mut c_void;
pub type FnShModeFunc = unsafe extern "C" fn(usize, c_int) -> *mut c_void;
pub type FnHmDelKey = unsafe extern "C" fn(
    *mut c_void,
    usize,
    *mut c_void,
    usize,
    usize,
    c_int,
) -> *mut c_void;
pub type FnStrAlloc = unsafe extern "C" fn(*mut StringArena, *mut c_char) -> *mut c_char;
pub type FnStrReset = unsafe extern "C" fn(*mut StringArena);
pub type FnStrKey = unsafe extern "C" fn(c_int) -> *mut c_char;
pub type FnStrDups = unsafe extern "C" fn(c_int);

/// All 16 exported entry points of one implementation, resolved by symbol name.
pub struct Api {
    pub name: &'static str,
    pub path: PathBuf,
    _lib: Library,
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
    pub str_dups: FnStrDups,
}

unsafe fn sym<T: Copy>(lib: &Library, name: &[u8]) -> T {
    let s: Symbol<T> = lib
        .get(name)
        .unwrap_or_else(|e| panic!("missing symbol {}: {e}", String::from_utf8_lossy(name)));
    *s
}

impl Api {
    pub fn load(name: &'static str, path: PathBuf) -> Api {
        unsafe {
            let lib = Library::new(&path)
                .unwrap_or_else(|e| panic!("cannot dlopen {}: {e}", path.display()));
            let api = Api {
                name,
                path: path.clone(),
                arrgrowf: sym(&lib, b"stbds_arrgrowf\0"),
                arrfreef: sym(&lib, b"stbds_arrfreef\0"),
                rand_seed: sym(&lib, b"stbds_rand_seed\0"),
                hash_string: sym(&lib, b"stbds_hash_string\0"),
                hash_bytes: sym(&lib, b"stbds_hash_bytes\0"),
                hmfree_func: sym(&lib, b"stbds_hmfree_func\0"),
                hmget_key_ts: sym(&lib, b"stbds_hmget_key_ts\0"),
                hmget_key: sym(&lib, b"stbds_hmget_key\0"),
                hmput_default: sym(&lib, b"stbds_hmput_default\0"),
                hmput_key: sym(&lib, b"stbds_hmput_key\0"),
                shmode_func: sym(&lib, b"stbds_shmode_func\0"),
                hmdel_key: sym(&lib, b"stbds_hmdel_key\0"),
                stralloc: sym(&lib, b"stbds_stralloc\0"),
                strreset: sym(&lib, b"stbds_strreset\0"),
                strkey: sym(&lib, b"strkey\0"),
                str_dups: sym(&lib, b"str_dups\0"),
                _lib: lib,
            };
            api
        }
    }
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn find_c_so() -> PathBuf {
    let build = manifest_dir().join("..").join("c_src").join("build");
    let mut found: Option<PathBuf> = None;
    let rd = std::fs::read_dir(&build).unwrap_or_else(|e| {
        panic!(
            "c_src/build not found ({e}); run:\n  cd c_src && mkdir -p build && cd build && \
             cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build ."
        )
    });
    for e in rd.flatten() {
        let p = e.path();
        if p.extension().and_then(|s| s.to_str()) == Some("so") {
            found = Some(p);
            break;
        }
    }
    found.unwrap_or_else(|| panic!("no .so in {}", build.display()))
}

/// The Rust `cdylib` produced by the *same* profile as this test binary
/// (`target/<profile>/libstr_dups_lib.so`).
fn find_rust_so() -> PathBuf {
    let exe = std::env::current_exe().expect("current_exe");
    // .../target/<profile>/deps/<test>  ->  .../target/<profile>
    let profile_dir = exe
        .parent()
        .and_then(|p| p.parent())
        .expect("test binary layout")
        .to_path_buf();
    let candidates = [
        profile_dir.join("libstr_dups_lib.so"),
        manifest_dir().join("target/release/libstr_dups_lib.so"),
        manifest_dir().join("target/debug/libstr_dups_lib.so"),
    ];
    for c in candidates.iter() {
        if c.exists() {
            return c.clone();
        }
    }
    panic!(
        "Rust cdylib not found. Build it first, e.g.\n  cargo build\n  cargo build --release\n\
         looked in: {:?}",
        candidates
    );
}

pub struct Pair {
    pub c: Api,
    pub rs: Api,
}

/// Both implementations, loaded once per test process.
pub fn pair() -> &'static Pair {
    use std::sync::OnceLock;
    static P: OnceLock<Pair> = OnceLock::new();
    P.get_or_init(|| Pair {
        c: Api::load("C", find_c_so()),
        rs: Api::load("Rust", find_rust_so()),
    })
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (xorshift64*) — fixed seed for reproducibility
// ---------------------------------------------------------------------------

pub const SEED: u64 = 0x243F_6A88_85A3_08D3;

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng(if seed == 0 { 0xdead_beef } else { seed })
    }
    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
    pub fn next_usize(&mut self) -> usize {
        self.next_u64() as usize
    }
    /// Uniform-ish in `[0, n)`; `n == 0` yields 0.
    pub fn below(&mut self, n: usize) -> usize {
        if n == 0 {
            0
        } else {
            (self.next_u64() % (n as u64)) as usize
        }
    }
    pub fn byte(&mut self) -> u8 {
        (self.next_u64() >> 33) as u8
    }
    pub fn bytes(&mut self, n: usize) -> Vec<u8> {
        (0..n).map(|_| self.byte()).collect()
    }
    /// A NUL-terminated printable-ASCII string of `n` content bytes.
    pub fn cstring(&mut self, n: usize) -> Vec<u8> {
        let mut v: Vec<u8> = (0..n).map(|_| 0x21 + (self.byte() % 0x5e)).collect();
        v.push(0);
        v
    }
    /// `cstring` with a length drawn from `[lo, lo+span)`.
    pub fn cstring_range(&mut self, lo: usize, span: usize) -> Vec<u8> {
        let n = lo + self.below(span);
        self.cstring(n)
    }
    /// A NUL-terminated string of arbitrary non-zero bytes (incl. >= 0x80).
    pub fn cstring_highbit(&mut self, n: usize) -> Vec<u8> {
        let mut v: Vec<u8> = (0..n)
            .map(|_| {
                let b = self.byte();
                if b == 0 {
                    0x80
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
// State canonicalisation: turn a live hash-map / array into comparable bytes
// ---------------------------------------------------------------------------

pub unsafe fn header(arr: *mut c_void) -> *mut ArrayHeader {
    (arr as *mut ArrayHeader).sub(1)
}

pub unsafe fn hash_to_arr(h: *mut c_void, elemsize: usize) -> *mut c_void {
    (h as *mut u8).wrapping_sub(elemsize) as *mut c_void
}

pub unsafe fn arr_to_hash(a: *mut c_void, elemsize: usize) -> *mut c_void {
    (a as *mut u8).wrapping_add(elemsize) as *mut c_void
}

fn ptr_tag(p: *const c_void) -> &'static str {
    if p.is_null() {
        "NULL"
    } else {
        "PTR"
    }
}

/// Dump the raw array (header + `length` elements) in a pointer-independent way.
///
/// `string_key`: the first 8 bytes of each element are a `char *`; dump the
/// pointed-to string content instead of the (allocation-dependent) address.
pub unsafe fn dump_array(arr: *mut c_void, elemsize: usize, string_key: bool) -> String {
    if arr.is_null() {
        return "arr=NULL".to_string();
    }
    let h = &*header(arr);
    let mut s = format!(
        "arr{{len={},cap={},temp={},table={}}}",
        h.length,
        h.capacity,
        h.temp,
        ptr_tag(h.hash_table)
    );
    for i in 0..h.length {
        let base = (arr as *mut u8).add(elemsize * i);
        s.push_str(&format!("\n  e[{i}]="));
        if string_key && elemsize >= 8 {
            let kp = *(base as *const *const c_char);
            if kp.is_null() {
                s.push_str("key=NULL");
            } else {
                let cs = std::ffi::CStr::from_ptr(kp);
                s.push_str(&format!("key={:?}", cs.to_bytes()));
            }
            let rest = std::slice::from_raw_parts(base.add(8), elemsize - 8);
            s.push_str(&format!(" rest={rest:?}"));
        } else {
            let all = std::slice::from_raw_parts(base, elemsize);
            s.push_str(&format!("{all:?}"));
        }
    }
    s
}

/// Dump the hash index (all scalar fields + every bucket) minus raw addresses.
pub unsafe fn dump_table(arr: *mut c_void) -> String {
    if arr.is_null() {
        return "table=NULL(arr)".to_string();
    }
    let h = &*header(arr);
    if h.hash_table.is_null() {
        return "table=NULL".to_string();
    }
    let t = &*(h.hash_table as *const HashIndex);
    let mut s = format!(
        "table{{slots={},used={},used_thr={},shrink_thr={},tomb={},tomb_thr={},seed={:#x},log2={},\
         arena{{storage={},remaining={},block={},mode={}}}}}",
        t.slot_count,
        t.used_count,
        t.used_count_threshold,
        t.used_count_shrink_threshold,
        t.tombstone_count,
        t.tombstone_count_threshold,
        t.seed,
        t.slot_count_log2,
        ptr_tag(t.string.storage),
        t.string.remaining,
        t.string.block,
        t.string.mode,
    );
    let nb = t.slot_count >> STBDS_BUCKET_SHIFT;
    for i in 0..nb {
        let b = &*t.storage.add(i);
        s.push_str(&format!("\n  b[{i}] h={:?} i={:?}", b.hash, b.index));
    }
    s
}

/// Full canonical state of a hash-map: array + index.
pub unsafe fn dump_map(hashptr: *mut c_void, elemsize: usize, string_key: bool) -> String {
    if hashptr.is_null() {
        return "map=NULL".to_string();
    }
    let arr = hash_to_arr(hashptr, elemsize);
    format!(
        "{}\n{}",
        dump_array(arr, elemsize, string_key),
        dump_table(arr)
    )
}

/// `stbds_temp_key(arr)` == `*(char **) stbds_header(arr)->hash_table`, i.e. the
/// `temp_key` field. It is left *uninitialised* by `stbds_make_hash_index`, so
/// it may only be inspected after a string-mode put has written it.
pub unsafe fn temp_key_str(arr: *mut c_void) -> String {
    if arr.is_null() {
        return "NULL(arr)".into();
    }
    let h = &*header(arr);
    if h.hash_table.is_null() {
        return "NULL(table)".into();
    }
    let kp = *(h.hash_table as *const *const c_char);
    if kp.is_null() {
        "NULL".into()
    } else {
        format!("{:?}", std::ffi::CStr::from_ptr(kp).to_bytes())
    }
}

/// Canonical state of a string arena: block chain lengths + scalar fields.
pub unsafe fn dump_arena(a: *const StringArena) -> String {
    let a = &*a;
    let mut n = 0usize;
    let mut x = a.storage as *const StringBlock;
    while !x.is_null() && n < 100_000 {
        n += 1;
        x = (*x).next;
    }
    format!(
        "arena{{blocks={},remaining={},block={},mode={},storage={}}}",
        n,
        a.remaining,
        a.block,
        a.mode,
        ptr_tag(a.storage)
    )
}

// ---------------------------------------------------------------------------
// stdout capture (for str_dups, which printf()s)
// ---------------------------------------------------------------------------

extern "C" {
    fn dup(fd: c_int) -> c_int;
    fn dup2(old: c_int, new: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
    fn open(path: *const c_char, flags: c_int, ...) -> c_int;
}

const O_RDWR: c_int = 2;
const O_CREAT: c_int = 64;
const O_TRUNC: c_int = 512;

/// Run `f` with fd 1 redirected to a fresh temp file; return what was written.
pub fn capture_stdout<F: FnOnce()>(tag: &str, f: F) -> Vec<u8> {
    let path = std::env::temp_dir().join(format!(
        "strdups_cap_{}_{}_{}.txt",
        std::process::id(),
        tag,
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let cpath = std::ffi::CString::new(path.to_str().unwrap()).unwrap();
    unsafe {
        fflush(std::ptr::null_mut());
        let saved = dup(1);
        assert!(saved >= 0, "dup(1) failed");
        let fd = open(cpath.as_ptr(), O_RDWR | O_CREAT | O_TRUNC, 0o600 as c_int);
        assert!(fd >= 0, "open temp failed");
        dup2(fd, 1);
        f();
        fflush(std::ptr::null_mut());
        dup2(saved, 1);
        close(saved);
        close(fd);
    }
    let out = std::fs::read(&path).unwrap_or_default();
    let _ = std::fs::remove_file(&path);
    out
}

// ---------------------------------------------------------------------------
// Convenience: a hash-map handle driven through one Api
// ---------------------------------------------------------------------------

/// Owns a `void *` hash-map pointer (the "user visible" `t`, i.e. `arr + elemsize`).
pub struct Map<'a> {
    pub api: &'a Api,
    pub p: *mut c_void,
    pub elemsize: usize,
}

impl<'a> Map<'a> {
    /// `t = NULL` — nothing allocated yet.
    pub fn empty(api: &'a Api, elemsize: usize) -> Map<'a> {
        Map { api, p: std::ptr::null_mut(), elemsize }
    }
    /// `sh_new_strdup` / `sh_new_arena` / `shmode_func(elemsize, mode)`.
    pub fn shmode(api: &'a Api, elemsize: usize, mode: c_int) -> Map<'a> {
        let p = unsafe { (api.shmode_func)(elemsize, mode) };
        Map { api, p, elemsize }
    }

    pub unsafe fn put(&mut self, key: *mut c_void, keysize: usize, mode: c_int) -> isize {
        self.p = (self.api.hmput_key)(self.p, self.elemsize, key, keysize, mode);
        (*header(hash_to_arr(self.p, self.elemsize))).temp
    }

    /// `hmput` semantics: put the key, then write the value bytes at
    /// `t[temp]` offset `voff..voff+vlen`.
    pub unsafe fn put_kv(
        &mut self,
        key: *mut c_void,
        keysize: usize,
        mode: c_int,
        voff: usize,
        value: &[u8],
    ) -> isize {
        let temp = self.put(key, keysize, mode);
        let slot = (self.p as *mut u8)
            .wrapping_add(self.elemsize.wrapping_mul(temp as usize))
            .add(voff);
        std::ptr::copy_nonoverlapping(value.as_ptr(), slot, value.len());
        temp
    }

    pub unsafe fn get(&mut self, key: *mut c_void, keysize: usize, mode: c_int) -> isize {
        self.p = (self.api.hmget_key)(self.p, self.elemsize, key, keysize, mode);
        (*header(hash_to_arr(self.p, self.elemsize))).temp
    }

    pub unsafe fn get_ts(&mut self, key: *mut c_void, keysize: usize, mode: c_int) -> isize {
        let mut temp: isize = 0x5555_5555;
        self.p = (self.api.hmget_key_ts)(
            self.p,
            self.elemsize,
            key,
            keysize,
            &mut temp,
            mode,
        );
        temp
    }

    pub unsafe fn del(
        &mut self,
        key: *mut c_void,
        keysize: usize,
        keyoffset: usize,
        mode: c_int,
    ) -> isize {
        self.p = (self.api.hmdel_key)(self.p, self.elemsize, key, keysize, keyoffset, mode);
        if self.p.is_null() {
            0
        } else {
            (*header(hash_to_arr(self.p, self.elemsize))).temp
        }
    }

    pub unsafe fn put_default(&mut self) {
        self.p = (self.api.hmput_default)(self.p, self.elemsize);
    }

    pub unsafe fn dump(&self, string_key: bool) -> String {
        dump_map(self.p, self.elemsize, string_key)
    }

    pub unsafe fn free(&mut self) {
        if !self.p.is_null() {
            (self.api.hmfree_func)(hash_to_arr(self.p, self.elemsize), self.elemsize);
            self.p = std::ptr::null_mut();
        }
    }
}

/// Reset the global hash seed on BOTH libraries so their `make_hash_index`
/// LCG streams stay in lockstep.
pub fn sync_seed(p: &Pair, seed: usize) {
    unsafe {
        (p.c.rand_seed)(seed);
        (p.rs.rand_seed)(seed);
    }
}

/// Both `.so`s keep a *global* `stbds_hash_seed` that `make_hash_index()`
/// advances, and `strkey()` writes to a *global* 256-byte buffer. Tests must
/// therefore not run concurrently; every test takes this lock first.
pub fn lock() -> std::sync::MutexGuard<'static, ()> {
    use std::sync::{Mutex, OnceLock};
    static M: OnceLock<Mutex<()>> = OnceLock::new();
    M.get_or_init(|| Mutex::new(()))
        .lock()
        .unwrap_or_else(|e| e.into_inner())
}

/// Take the lock and put both libraries in a known seed state.
pub fn begin(seed: usize) -> (&'static Pair, std::sync::MutexGuard<'static, ()>) {
    let g = lock();
    let p = pair();
    sync_seed(p, seed);
    (p, g)
}
