//! Shared differential-test harness.
//!
//! Both the C `.so` and the Rust `.so` are loaded with `libloading`; every call
//! in every test goes through `dlsym`, so the `#[no_mangle]` export wrappers are
//! part of what is under test.

#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::ffi::{c_char, c_int, c_void};
use std::path::PathBuf;

// ---------------------------------------------------------------------------
// Layout-compatible mirrors of the C structs (used to inspect shared memory)
// ---------------------------------------------------------------------------

#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct ArrayHeader {
    pub length: usize,
    pub capacity: usize,
    pub hash_table: *mut c_void,
    pub temp: isize,
}

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct StringBlock {
    pub next: *mut StringBlock,
    pub storage: [c_char; 8],
}

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct StringArena {
    pub storage: *mut StringBlock,
    pub remaining: usize,
    pub block: u8,
    pub mode: u8,
}

impl StringArena {
    pub const fn zeroed() -> Self {
        StringArena { storage: std::ptr::null_mut(), remaining: 0, block: 0, mode: 0 }
    }
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct HashBucket {
    pub hash: [usize; 8],
    pub index: [isize; 8],
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

// Compile-time layout parity with the C (measured with gcc: 32/16/24/128/104).
const _: () = {
    assert!(std::mem::size_of::<ArrayHeader>() == 32);
    assert!(std::mem::size_of::<StringBlock>() == 16);
    assert!(std::mem::size_of::<StringArena>() == 24);
    assert!(std::mem::offset_of!(StringArena, block) == 16);
    assert!(std::mem::offset_of!(StringArena, mode) == 17);
    assert!(std::mem::size_of::<HashBucket>() == 128);
    assert!(std::mem::size_of::<HashIndex>() == 104);
    assert!(std::mem::offset_of!(HashIndex, string) == 72);
    assert!(std::mem::offset_of!(HashIndex, storage) == 96);
};

pub const HDR: usize = std::mem::size_of::<ArrayHeader>();

pub const HM_BINARY: c_int = 0;
pub const HM_STRING: c_int = 1;
pub const SH_NONE: c_int = 0;
pub const SH_DEFAULT: c_int = 1;
pub const SH_STRDUP: c_int = 2;
pub const SH_ARENA: c_int = 3;

// ---------------------------------------------------------------------------
// A normalized, comparable snapshot of a whole hash-map allocation
// ---------------------------------------------------------------------------

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Snapshot {
    pub null: bool,
    /// header length / capacity / temp (the `hash_table` *pointer* is excluded:
    /// the two libraries legitimately get different malloc addresses)
    pub length: usize,
    pub capacity: usize,
    pub temp: isize,
    pub has_table: bool,
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
    pub arena_block_count: usize,
    /// every bucket slot: (hash, index)
    pub slots: Vec<(usize, isize)>,
}

/// Snapshot the array header + hash index reachable from a `hash`-biased pointer.
///
/// `hash_ptr` is what `stbds_hmput_key` & friends return (i.e. `arr + elemsize`).
pub unsafe fn snapshot(hash_ptr: *mut c_void, elemsize: usize) -> Snapshot {
    let mut s = Snapshot {
        null: hash_ptr.is_null(),
        length: 0,
        capacity: 0,
        temp: 0,
        has_table: false,
        slot_count: 0,
        used_count: 0,
        used_count_threshold: 0,
        used_count_shrink_threshold: 0,
        tombstone_count: 0,
        tombstone_count_threshold: 0,
        seed: 0,
        slot_count_log2: 0,
        arena_remaining: 0,
        arena_block: 0,
        arena_mode: 0,
        arena_block_count: 0,
        slots: Vec::new(),
    };
    if hash_ptr.is_null() {
        return s;
    }
    let arr = (hash_ptr as *mut u8).sub(elemsize);
    snapshot_arr(arr as *mut c_void, &mut s);
    s
}

/// Snapshot from a raw (unbiased) array pointer.
pub unsafe fn snapshot_raw(arr: *mut c_void) -> Snapshot {
    let mut s = snapshot(std::ptr::null_mut(), 0);
    s.null = arr.is_null();
    if arr.is_null() {
        return s;
    }
    snapshot_arr(arr, &mut s);
    s
}

unsafe fn snapshot_arr(arr: *mut c_void, s: &mut Snapshot) {
    let h = (arr as *mut ArrayHeader).offset(-1);
    s.length = (*h).length;
    s.capacity = (*h).capacity;
    s.temp = (*h).temp;
    let t = (*h).hash_table as *mut HashIndex;
    if t.is_null() {
        return;
    }
    s.has_table = true;
    s.slot_count = (*t).slot_count;
    s.used_count = (*t).used_count;
    s.used_count_threshold = (*t).used_count_threshold;
    s.used_count_shrink_threshold = (*t).used_count_shrink_threshold;
    s.tombstone_count = (*t).tombstone_count;
    s.tombstone_count_threshold = (*t).tombstone_count_threshold;
    s.seed = (*t).seed;
    s.slot_count_log2 = (*t).slot_count_log2;
    s.arena_remaining = (*t).string.remaining;
    s.arena_block = (*t).string.block;
    s.arena_mode = (*t).string.mode;
    let mut b = (*t).string.storage;
    while !b.is_null() {
        s.arena_block_count += 1;
        b = (*b).next;
        if s.arena_block_count > 100_000 {
            break;
        }
    }
    let nbuckets = (*t).slot_count >> 3;
    s.slots.reserve(nbuckets * 8);
    for i in 0..nbuckets {
        let bk = (*t).storage.add(i);
        for j in 0..8 {
            s.slots.push(((*bk).hash[j], (*bk).index[j]));
        }
    }
}

/// The payload bytes of elements `1..length` (element 0 is the "default" slot,
/// which the C leaves zeroed for binary tables but writes key pointers into for
/// string tables — pointers differ between libraries so callers select what to
/// compare).
pub unsafe fn elem_bytes(hash_ptr: *mut c_void, elemsize: usize, len: usize) -> Vec<u8> {
    if hash_ptr.is_null() {
        return Vec::new();
    }
    let arr = (hash_ptr as *mut u8).sub(elemsize);
    std::slice::from_raw_parts(arr, elemsize * len).to_vec()
}

/// The NUL-terminated key strings referenced by elements `1..length`
/// (for string-mode tables where element `i` starts with a `char *`).
pub unsafe fn key_strings(hash_ptr: *mut c_void, elemsize: usize, len: usize) -> Vec<Vec<u8>> {
    let mut out = Vec::new();
    if hash_ptr.is_null() {
        return out;
    }
    let arr = (hash_ptr as *mut u8).sub(elemsize);
    for i in 1..len {
        let p = *(arr.add(elemsize * i) as *const *const c_char);
        if p.is_null() {
            out.push(Vec::new());
        } else {
            out.push(std::ffi::CStr::from_ptr(p).to_bytes().to_vec());
        }
    }
    out
}

// ---------------------------------------------------------------------------
// Deterministic RNG (xorshift64*) — fixed seed, reproducible
// ---------------------------------------------------------------------------

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng(if seed == 0 { 0x9E3779B97F4A7C15 } else { seed })
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
        if n == 0 { 0 } else { (self.next_u64() % (n as u64)) as usize }
    }
    pub fn byte(&mut self) -> u8 {
        (self.next_u64() >> 24) as u8
    }
    pub fn bytes(&mut self, n: usize) -> Vec<u8> {
        (0..n).map(|_| self.byte()).collect()
    }
    /// Random NUL-terminated C string of `len` payload bytes drawn from `alphabet`.
    pub fn cstring(&mut self, len: usize, alphabet: &[u8]) -> Vec<u8> {
        let mut v: Vec<u8> = (0..len).map(|_| alphabet[self.below(alphabet.len())]).collect();
        v.push(0);
        v
    }
}

pub const ASCII: &[u8] = b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789_-";
pub const HIGHBIT: &[u8] = &[
    0x80, 0x81, 0x9f, 0xa0, 0xbf, 0xc0, 0xdf, 0xe0, 0xfe, 0xff, 0x41, 0x7f,
];

// ---------------------------------------------------------------------------
// Library binding
// ---------------------------------------------------------------------------

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
pub type FnShModeFunc = unsafe extern "C" fn(usize, c_int) -> *mut c_void;
pub type FnStrAlloc = unsafe extern "C" fn(*mut StringArena, *mut c_char) -> *mut c_char;
pub type FnStrReset = unsafe extern "C" fn(*mut StringArena);
pub type FnStrKey = unsafe extern "C" fn(c_int) -> *mut c_char;
pub type FnHelxo = unsafe extern "C" fn(c_char);

pub struct Lib {
    pub name: &'static str,
    _lib: Library,
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
    pub shmode_func: FnShModeFunc,
    pub stralloc: FnStrAlloc,
    pub strreset: FnStrReset,
    pub strkey: FnStrKey,
    pub helxo: FnHelxo,
}

unsafe fn sym<T: Copy>(lib: &Library, name: &[u8]) -> T {
    let s: Symbol<T> = lib
        .get(name)
        .unwrap_or_else(|e| panic!("missing symbol {}: {e}", String::from_utf8_lossy(name)));
    *s
}

impl Lib {
    pub unsafe fn open(name: &'static str, path: &PathBuf) -> Lib {
        let lib = Library::new(path).unwrap_or_else(|e| panic!("dlopen {path:?}: {e}"));
        Lib {
            name,
            rand_seed: sym(&lib, b"stbds_rand_seed"),
            hash_bytes: sym(&lib, b"stbds_hash_bytes"),
            hash_string: sym(&lib, b"stbds_hash_string"),
            arrgrowf: sym(&lib, b"stbds_arrgrowf"),
            arrfreef: sym(&lib, b"stbds_arrfreef"),
            hmfree_func: sym(&lib, b"stbds_hmfree_func"),
            hmget_key: sym(&lib, b"stbds_hmget_key"),
            hmget_key_ts: sym(&lib, b"stbds_hmget_key_ts"),
            hmput_default: sym(&lib, b"stbds_hmput_default"),
            hmput_key: sym(&lib, b"stbds_hmput_key"),
            hmdel_key: sym(&lib, b"stbds_hmdel_key"),
            shmode_func: sym(&lib, b"stbds_shmode_func"),
            stralloc: sym(&lib, b"stbds_stralloc"),
            strreset: sym(&lib, b"stbds_strreset"),
            strkey: sym(&lib, b"strkey"),
            helxo: sym(&lib, b"helxo"),
            _lib: lib,
        }
    }
}

fn workspace_root() -> PathBuf {
    // CARGO_MANIFEST_DIR == <root>/translation
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).parent().unwrap().to_path_buf()
}

pub fn c_so_path() -> PathBuf {
    let root = workspace_root();
    let build = root.join("c_src/build");
    let mut found = None;
    if let Ok(rd) = std::fs::read_dir(&build) {
        for e in rd.flatten() {
            let p = e.path();
            let n = p.file_name().unwrap().to_string_lossy().to_string();
            if n.starts_with("lib") && n.ends_with(".so") {
                found = Some(p);
            }
        }
    }
    found.unwrap_or_else(|| {
        panic!(
            "C shared library not found in {build:?}. Build it with:\n  \
             cd c_src && mkdir -p build && cd build && \
             cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build ."
        )
    })
}

pub fn rust_so_path() -> PathBuf {
    // The test binary lives in target/<profile>/deps/ ; the cdylib is one level up.
    let exe = std::env::current_exe().expect("current_exe");
    let profile_dir = exe.parent().unwrap().parent().unwrap();
    let p = profile_dir.join("libhelxo_lib.so");
    if p.exists() {
        return p;
    }
    // Fall back to whichever profile has been built.
    for prof in ["release", "debug"] {
        let q = workspace_root().join("translation/target").join(prof).join("libhelxo_lib.so");
        if q.exists() {
            return q;
        }
    }
    panic!("Rust cdylib libhelxo_lib.so not found (looked in {p:?})");
}

/// The (C, Rust) pair, opened once per test process.
pub fn libs() -> &'static (Lib, Lib) {
    use std::sync::OnceLock;
    static LIBS: OnceLock<(Lib, Lib)> = OnceLock::new();
    LIBS.get_or_init(|| unsafe {
        (Lib::open("C", &c_so_path()), Lib::open("RUST", &rust_so_path()))
    })
}

// ---------------------------------------------------------------------------
// stdout capture (for `helxo`, which prints via printf)
// ---------------------------------------------------------------------------

unsafe extern "C" {
    fn dup(fd: c_int) -> c_int;
    fn dup2(old: c_int, new: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
}

/// Run `f` with fd 1 redirected into a temporary file, return the bytes written.
pub fn capture_stdout<F: FnOnce()>(tag: &str, f: F) -> Vec<u8> {
    use std::io::{Read, Seek, SeekFrom};
    use std::os::unix::io::AsRawFd;

    let path = std::env::temp_dir().join(format!(
        "helxo_capture_{}_{}_{}.txt",
        std::process::id(),
        tag,
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .read(true)
        .write(true)
        .truncate(true)
        .open(&path)
        .expect("temp file");

    let out = unsafe {
        fflush(std::ptr::null_mut()); // flush all streams before swapping fd 1
        let saved = dup(1);
        assert!(saved >= 0);
        assert!(dup2(file.as_raw_fd(), 1) >= 0);
        f();
        fflush(std::ptr::null_mut());
        assert!(dup2(saved, 1) >= 0);
        close(saved);

        let mut buf = Vec::new();
        file.seek(SeekFrom::Start(0)).unwrap();
        file.read_to_end(&mut buf).unwrap();
        buf
    };
    let _ = std::fs::remove_file(&path);
    out
}

// ---------------------------------------------------------------------------
// Serialization + seed control
// ---------------------------------------------------------------------------

/// `stbds_hash_seed` is a file-static in each library, consumed at
/// table-creation time and advanced by an LCG. Any test that creates a hash
/// table must therefore hold this lock so that the C and Rust seed streams stay
/// in lock-step (tests inside one binary run on several threads).
pub fn serial() -> std::sync::MutexGuard<'static, ()> {
    use std::sync::{Mutex, OnceLock};
    static M: OnceLock<Mutex<()>> = OnceLock::new();
    match M.get_or_init(|| Mutex::new(())).lock() {
        Ok(g) => g,
        Err(p) => p.into_inner(),
    }
}

/// Reset both libraries' global hash seed so a test is reproducible in isolation.
pub fn set_seed(s: usize) {
    let (c, r) = libs();
    unsafe {
        (c.rand_seed)(s);
        (r.rand_seed)(s);
    }
}

pub const DEFAULT_SEED: usize = 0x31415926;

// ---------------------------------------------------------------------------
// Dual map driver: every operation is applied to both `.so`s and compared
// ---------------------------------------------------------------------------

/// How the element's key field is stored, which decides what can be compared
/// byte-for-byte across the two libraries.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum KeyRepr {
    /// key bytes are `memcpy`'d into the element (binary / `SH_NONE`)
    Bytes,
    /// the caller's pointer is stored verbatim (`SH_DEFAULT`) — comparable,
    /// because both libraries are handed the *same* key buffer
    SharedPtr,
    /// the library allocates its own copy (`SH_STRDUP` / `SH_ARENA`) — compare
    /// the pointed-to strings instead of the pointers
    OwnedStr,
}

pub struct Dual {
    pub elemsize: usize,
    pub keysize: usize,
    pub mode: c_int,
    pub repr: KeyRepr,
    pub cp: *mut c_void,
    pub rp: *mut c_void,
    /// payload byte written into `[key_end .. elemsize)` after each insert, so
    /// that `hmdel_key`'s element `memmove` is observable
    pub tag: u8,
    /// `stbds_make_hash_index` never initialises `stbds_hash_index::temp_key`
    /// (in C or in Rust), so it only becomes comparable once a string-mode
    /// `hmput_key` has written it.
    pub temp_key_valid: bool,
}

impl Dual {
    pub fn new(elemsize: usize, keysize: usize, mode: c_int, repr: KeyRepr) -> Dual {
        Dual {
            elemsize,
            keysize,
            mode,
            repr,
            cp: std::ptr::null_mut(),
            rp: std::ptr::null_mut(),
            tag: 0,
            temp_key_valid: false,
        }
    }

    /// Start from `stbds_shmode_func(elemsize, sh_mode)` on both libraries.
    pub fn with_shmode(elemsize: usize, keysize: usize, mode: c_int, sh: c_int, repr: KeyRepr) -> Dual {
        let (c, r) = libs();
        let mut d = Dual::new(elemsize, keysize, mode, repr);
        unsafe {
            d.cp = (c.shmode_func)(elemsize, sh);
            d.rp = (r.shmode_func)(elemsize, sh);
        }
        d.check("with_shmode");
        d
    }

    fn key_end(&self) -> usize {
        match self.repr {
            KeyRepr::Bytes => self.keysize,
            _ => 8,
        }
    }

    pub fn snap_c(&self) -> Snapshot {
        unsafe { snapshot(self.cp, self.elemsize) }
    }
    pub fn snap_r(&self) -> Snapshot {
        unsafe { snapshot(self.rp, self.elemsize) }
    }

    /// Compare everything comparable: header, hash index, buckets, key material,
    /// element payload tags.
    pub fn check(&self, ctx: &str) {
        let sc = self.snap_c();
        let sr = self.snap_r();
        assert_eq!(sc.null, sr.null, "{ctx}: null-ness differs");
        assert_eq!(sc, sr, "{ctx}: array header / hash index / buckets differ");
        if sc.null {
            return;
        }
        let len = sc.length;
        match self.repr {
            KeyRepr::Bytes | KeyRepr::SharedPtr => {
                // element 0 is the zeroed default slot; keys of 1..len are
                // either copied bytes or the shared caller pointer
                let kc = unsafe { self.key_region(self.cp, len) };
                let kr = unsafe { self.key_region(self.rp, len) };
                assert_eq!(kc, kr, "{ctx}: element key bytes differ");
            }
            KeyRepr::OwnedStr => {
                let kc = unsafe { key_strings(self.cp, self.elemsize, len) };
                let kr = unsafe { key_strings(self.rp, self.elemsize, len) };
                assert_eq!(kc, kr, "{ctx}: element key strings differ");
            }
        }
        // payload tags in [key_end, elemsize)
        let ke = self.key_end();
        if ke < self.elemsize {
            let pc = unsafe { self.payload_region(self.cp, len) };
            let pr = unsafe { self.payload_region(self.rp, len) };
            assert_eq!(pc, pr, "{ctx}: element payload bytes differ");
        }
        // temp_key is only meaningful (and comparable) for pointer-keyed tables
        if !self.temp_key_valid {
            return;
        }
        if self.repr == KeyRepr::SharedPtr {
            let tc = unsafe { self.temp_key(self.cp) };
            let tr = unsafe { self.temp_key(self.rp) };
            assert_eq!(tc, tr, "{ctx}: table->temp_key pointer differs");
        } else if self.repr == KeyRepr::OwnedStr {
            let tc = unsafe { self.temp_key_str(self.cp) };
            let tr = unsafe { self.temp_key_str(self.rp) };
            assert_eq!(tc, tr, "{ctx}: table->temp_key string differs");
        }
    }

    unsafe fn key_region(&self, hp: *mut c_void, len: usize) -> Vec<u8> {
        let arr = (hp as *mut u8).sub(self.elemsize);
        let ke = self.key_end();
        let mut v = Vec::new();
        for i in 0..len {
            v.extend_from_slice(std::slice::from_raw_parts(arr.add(self.elemsize * i), ke.min(self.elemsize)));
        }
        v
    }

    unsafe fn payload_region(&self, hp: *mut c_void, len: usize) -> Vec<u8> {
        let arr = (hp as *mut u8).sub(self.elemsize);
        let ke = self.key_end();
        let mut v = Vec::new();
        for i in 1..len {
            v.extend_from_slice(std::slice::from_raw_parts(
                arr.add(self.elemsize * i + ke),
                self.elemsize - ke,
            ));
        }
        v
    }

    unsafe fn temp_key(&self, hp: *mut c_void) -> usize {
        let arr = (hp as *mut u8).sub(self.elemsize);
        let h = (arr as *mut ArrayHeader).offset(-1);
        let t = (*h).hash_table as *mut HashIndex;
        if t.is_null() { 0 } else { (*t).temp_key as usize }
    }

    unsafe fn temp_key_str(&self, hp: *mut c_void) -> Vec<u8> {
        let p = self.temp_key(hp) as *const c_char;
        if p.is_null() { Vec::new() } else { std::ffi::CStr::from_ptr(p).to_bytes().to_vec() }
    }

    /// `stbds_hmput_key` on both. Returns the resulting `header->temp` (they are
    /// asserted equal). Also stamps the element payload so later `memmove`s are
    /// observable.
    pub fn put(&mut self, key: *mut c_void, ctx: &str) -> isize {
        let (c, r) = libs();
        unsafe {
            self.cp = (c.hmput_key)(self.cp, self.elemsize, key, self.keysize, self.mode);
            self.rp = (r.hmput_key)(self.rp, self.elemsize, key, self.keysize, self.mode);
        }
        // stamp payload before comparing so both sides are defined
        let ke = self.key_end();
        if ke < self.elemsize {
            self.tag = self.tag.wrapping_add(1);
            let t = self.tag;
            let idx = unsafe { self.temp_of(self.cp) };
            let idxr = unsafe { self.temp_of(self.rp) };
            assert_eq!(idx, idxr, "{ctx}: put temp differs");
            if idx >= 0 {
                unsafe {
                    for (hp, _) in [(self.cp, 0), (self.rp, 1)] {
                        let e = (hp as *mut u8).offset((self.elemsize as isize) * idx);
                        std::ptr::write_bytes(e.add(ke), t, self.elemsize - ke);
                    }
                }
            }
        }
        if matches!(self.repr, KeyRepr::SharedPtr | KeyRepr::OwnedStr) {
            self.temp_key_valid = true;
        }
        self.check(ctx);
        unsafe { self.temp_of(self.cp) }
    }

    unsafe fn temp_of(&self, hp: *mut c_void) -> isize {
        let arr = (hp as *mut u8).sub(self.elemsize);
        (*(arr as *mut ArrayHeader).offset(-1)).temp
    }

    pub fn get(&mut self, key: *mut c_void, ctx: &str) -> isize {
        let (c, r) = libs();
        unsafe {
            self.cp = (c.hmget_key)(self.cp, self.elemsize, key, self.keysize, self.mode);
            self.rp = (r.hmget_key)(self.rp, self.elemsize, key, self.keysize, self.mode);
            let tc = self.temp_of(self.cp);
            let tr = self.temp_of(self.rp);
            assert_eq!(tc, tr, "{ctx}: hmget_key temp differs");
            self.check(ctx);
            tc
        }
    }

    pub fn get_ts(&mut self, key: *mut c_void, ctx: &str) -> isize {
        let (c, r) = libs();
        unsafe {
            let mut tc: isize = 0x5555_5555;
            let mut tr: isize = 0x5555_5555;
            self.cp = (c.hmget_key_ts)(self.cp, self.elemsize, key, self.keysize, &mut tc, self.mode);
            self.rp = (r.hmget_key_ts)(self.rp, self.elemsize, key, self.keysize, &mut tr, self.mode);
            assert_eq!(tc, tr, "{ctx}: hmget_key_ts *temp differs");
            self.check(ctx);
            tc
        }
    }

    /// `stbds_hmdel_key` on both. Returns `header->temp` (0 = not deleted).
    pub fn del(&mut self, key: *mut c_void, keyoffset: usize, ctx: &str) -> isize {
        let (c, r) = libs();
        unsafe {
            // `stbds_hmdel_key` never writes `table->temp_key`, and a shrink /
            // rebuild swaps in a freshly `realloc`'d (hence uninitialised)
            // `stbds_hash_index`, so temp_key stops being comparable here.
            self.temp_key_valid = false;
            let nc = (c.hmdel_key)(self.cp, self.elemsize, key, self.keysize, keyoffset, self.mode);
            let nr = (r.hmdel_key)(self.rp, self.elemsize, key, self.keysize, keyoffset, self.mode);
            assert_eq!(nc.is_null(), nr.is_null(), "{ctx}: hmdel_key null-ness differs");
            self.cp = nc;
            self.rp = nr;
            if nc.is_null() {
                return 0;
            }
            let tc = self.temp_of(self.cp);
            let tr = self.temp_of(self.rp);
            assert_eq!(tc, tr, "{ctx}: hmdel_key temp differs");
            self.check(ctx);
            tc
        }
    }

    pub fn free(&mut self) {
        let (c, r) = libs();
        unsafe {
            if !self.cp.is_null() {
                (c.hmfree_func)((self.cp as *mut u8).sub(self.elemsize) as *mut c_void, self.elemsize);
            }
            if !self.rp.is_null() {
                (r.hmfree_func)((self.rp as *mut u8).sub(self.elemsize) as *mut c_void, self.elemsize);
            }
        }
        self.cp = std::ptr::null_mut();
        self.rp = std::ptr::null_mut();
    }
}

/// A stable set of key buffers. The *same* buffer is handed to both libraries,
/// so `SH_DEFAULT` tables end up storing identical pointers.
pub struct Keys {
    pub bufs: Vec<Box<[u8]>>,
}

impl Keys {
    pub fn binary(rng: &mut Rng, n: usize, keysize: usize) -> Keys {
        let mut bufs = Vec::with_capacity(n);
        let mut seen = std::collections::HashSet::new();
        while bufs.len() < n {
            let b: Vec<u8> = if keysize == 0 { Vec::new() } else { rng.bytes(keysize) };
            if keysize != 0 && !seen.insert(b.clone()) {
                continue;
            }
            // always allocate at least one byte so the pointer is valid
            let mut v = b;
            if v.is_empty() {
                v.push(0);
            }
            bufs.push(v.into_boxed_slice());
            if keysize == 0 {
                break;
            }
        }
        Keys { bufs }
    }

    pub fn strings(rng: &mut Rng, n: usize, maxlen: usize, alphabet: &[u8]) -> Keys {
        let mut bufs = Vec::with_capacity(n);
        let mut seen = std::collections::HashSet::new();
        let mut guard = 0;
        while bufs.len() < n && guard < n * 1000 {
            guard += 1;
            let len = rng.below(maxlen + 1);
            let s = rng.cstring(len, alphabet);
            if !seen.insert(s.clone()) {
                continue;
            }
            bufs.push(s.into_boxed_slice());
        }
        Keys { bufs }
    }

    pub fn ptr(&mut self, i: usize) -> *mut c_void {
        self.bufs[i].as_mut_ptr() as *mut c_void
    }
    pub fn len(&self) -> usize {
        self.bufs.len()
    }
}
