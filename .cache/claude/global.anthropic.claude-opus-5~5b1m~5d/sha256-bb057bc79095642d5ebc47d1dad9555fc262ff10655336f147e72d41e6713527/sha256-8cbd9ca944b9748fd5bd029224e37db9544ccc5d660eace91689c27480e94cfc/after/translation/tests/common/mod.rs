//! Differential-test harness: loads BOTH the C `.so` and the Rust `.so` via
//! `libloading` and drives them through their exported C ABI only.
#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::ffi::{c_char, c_int, c_void};
use std::path::PathBuf;
use std::sync::OnceLock;

// ---------------------------------------------------------------------------
// mirrored C types
// ---------------------------------------------------------------------------

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
#[derive(Clone, Copy, Debug, Default)]
pub struct StringArena {
    pub storage: *mut StringBlock,
    pub remaining: usize,
    pub block: u8,
    pub mode: u8,
}
impl StringArena {
    pub fn zeroed() -> Self {
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

pub const BUCKET_LENGTH: usize = 8;
pub const BUCKET_SHIFT: usize = 3;

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

pub const SH_NONE: c_int = 0;
pub const SH_DEFAULT: c_int = 1;
pub const SH_STRDUP: c_int = 2;
pub const SH_ARENA: c_int = 3;
pub const HM_BINARY: c_int = 0;
pub const HM_STRING: c_int = 1;

// ---------------------------------------------------------------------------
// function signatures
// ---------------------------------------------------------------------------

type FnArrGrowf = unsafe extern "C" fn(*mut c_void, usize, usize, usize) -> *mut c_void;
type FnArrFreef = unsafe extern "C" fn(*mut c_void);
type FnRandSeed = unsafe extern "C" fn(usize);
type FnHashString = unsafe extern "C" fn(*mut c_char, usize) -> usize;
type FnHashBytes = unsafe extern "C" fn(*mut c_void, usize, usize) -> usize;
type FnHmFree = unsafe extern "C" fn(*mut c_void, usize);
type FnHmGetTs =
    unsafe extern "C" fn(*mut c_void, usize, *mut c_void, usize, *mut isize, c_int) -> *mut c_void;
type FnHmGet = unsafe extern "C" fn(*mut c_void, usize, *mut c_void, usize, c_int) -> *mut c_void;
type FnHmPutDefault = unsafe extern "C" fn(*mut c_void, usize) -> *mut c_void;
type FnHmPutKey = unsafe extern "C" fn(*mut c_void, usize, *mut c_void, usize, c_int) -> *mut c_void;
type FnShMode = unsafe extern "C" fn(usize, c_int) -> *mut c_void;
type FnHmDelKey =
    unsafe extern "C" fn(*mut c_void, usize, *mut c_void, usize, usize, c_int) -> *mut c_void;
type FnStrAlloc = unsafe extern "C" fn(*mut StringArena, *mut c_char) -> *mut c_char;
type FnStrReset = unsafe extern "C" fn(*mut StringArena);
type FnStrKey = unsafe extern "C" fn(c_int) -> *mut c_char;
type FnStrPut = unsafe extern "C" fn(c_int);

/// One loaded implementation (either the C `.so` or the Rust `.so`).
pub struct Impl {
    _lib: Library,
    pub name: &'static str,
    pub arrgrowf: FnArrGrowf,
    pub arrfreef: FnArrFreef,
    pub rand_seed: FnRandSeed,
    pub hash_string: FnHashString,
    pub hash_bytes: FnHashBytes,
    pub hmfree_func: FnHmFree,
    pub hmget_key_ts: FnHmGetTs,
    pub hmget_key: FnHmGet,
    pub hmput_default: FnHmPutDefault,
    pub hmput_key: FnHmPutKey,
    pub shmode_func: FnShMode,
    pub hmdel_key: FnHmDelKey,
    pub stralloc: FnStrAlloc,
    pub strreset: FnStrReset,
    pub strkey: FnStrKey,
    pub str_put: FnStrPut,
}

unsafe fn sym<T: Copy>(lib: &Library, n: &str) -> T {
    let s: Symbol<T> = lib
        .get(n.as_bytes())
        .unwrap_or_else(|e| panic!("missing symbol {n}: {e}"));
    *s
}

impl Impl {
    unsafe fn load(path: &PathBuf, name: &'static str) -> Impl {
        let lib = Library::new(path).unwrap_or_else(|e| panic!("dlopen {path:?}: {e}"));
        Impl {
            name,
            arrgrowf: sym(&lib, "stbds_arrgrowf"),
            arrfreef: sym(&lib, "stbds_arrfreef"),
            rand_seed: sym(&lib, "stbds_rand_seed"),
            hash_string: sym(&lib, "stbds_hash_string"),
            hash_bytes: sym(&lib, "stbds_hash_bytes"),
            hmfree_func: sym(&lib, "stbds_hmfree_func"),
            hmget_key_ts: sym(&lib, "stbds_hmget_key_ts"),
            hmget_key: sym(&lib, "stbds_hmget_key"),
            hmput_default: sym(&lib, "stbds_hmput_default"),
            hmput_key: sym(&lib, "stbds_hmput_key"),
            shmode_func: sym(&lib, "stbds_shmode_func"),
            hmdel_key: sym(&lib, "stbds_hmdel_key"),
            stralloc: sym(&lib, "stbds_stralloc"),
            strreset: sym(&lib, "stbds_strreset"),
            strkey: sym(&lib, "strkey"),
            str_put: sym(&lib, "str_put"),
            _lib: lib,
        }
    }
}

pub struct Both {
    pub c: Impl,
    pub r: Impl,
}

fn repo_root() -> PathBuf {
    // CARGO_MANIFEST_DIR == <root>/translation
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf()
}

fn find_c_so() -> PathBuf {
    let dir = repo_root().join("c_src/build");
    let mut cands: Vec<PathBuf> = std::fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("c_src/build not built ({e}); run cmake first"))
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| {
            p.extension().map(|x| x == "so").unwrap_or(false)
                && p.file_name().unwrap().to_string_lossy().starts_with("lib")
        })
        .collect();
    cands.sort();
    cands.pop().expect("no .so in c_src/build")
}

fn find_rust_so() -> PathBuf {
    let p = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/release/libstr_put_lib.so");
    assert!(p.exists(), "run `cargo build --release` first: {p:?}");
    p
}

static BOTH: OnceLock<Both> = OnceLock::new();

pub fn libs() -> &'static Both {
    BOTH.get_or_init(|| unsafe {
        Both {
            c: Impl::load(&find_c_so(), "C"),
            r: Impl::load(&find_rust_so(), "RUST"),
        }
    })
}

/// Reset the (per-library) global hash seed on both impls so their PRNG chains
/// stay in lockstep.
pub fn seed_both(seed: usize) {
    let l = libs();
    unsafe {
        (l.c.rand_seed)(seed);
        (l.r.rand_seed)(seed);
    }
}

// ---------------------------------------------------------------------------
// deterministic PRNG (splitmix64) -- fixed seed for reproducibility
// ---------------------------------------------------------------------------

pub struct Rng(pub u64);
impl Rng {
    pub fn new() -> Rng {
        Rng(0x243F_6A88_85A3_08D3)
    }
    pub fn with(s: u64) -> Rng {
        Rng(s)
    }
    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    pub fn below(&mut self, n: u64) -> u64 {
        if n == 0 {
            0
        } else {
            self.next_u64() % n
        }
    }
    pub fn bytes(&mut self, n: usize) -> Vec<u8> {
        (0..n).map(|_| (self.next_u64() & 0xff) as u8).collect()
    }
    /// NUL-terminated random printable-ASCII C string of `n` chars.
    pub fn cstring(&mut self, n: usize) -> Vec<u8> {
        let mut v: Vec<u8> = (0..n)
            .map(|_| b'a' + (self.next_u64() % 26) as u8)
            .collect();
        v.push(0);
        v
    }
}

// ---------------------------------------------------------------------------
// snapshots (normalised, pointer-free descriptions of library state)
// ---------------------------------------------------------------------------

#[derive(PartialEq, Eq, Debug, Clone)]
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
    pub arena_blocks: usize,
    pub temp_key: Option<Vec<u8>>,
    pub buckets: Vec<(usize, isize)>,
}

#[derive(PartialEq, Eq, Debug, Clone)]
pub struct MapSnap {
    pub null: bool,
    pub length: usize,
    pub capacity: usize,
    pub temp: isize,
    /// raw element bytes with the key slot blanked when it holds a pointer
    pub elems: Vec<Vec<u8>>,
    /// C-string content of the key slot, when the key slot holds a pointer
    pub keys: Vec<Option<Vec<u8>>>,
    pub table: Option<TableSnap>,
}

unsafe fn read_cstr(p: *const c_char) -> Option<Vec<u8>> {
    if p.is_null() {
        return None;
    }
    let mut v = Vec::new();
    let mut q = p as *const u8;
    // hard cap so a corrupt pointer cannot hang the test
    for _ in 0..1 << 20 {
        let b = *q;
        if b == 0 {
            break;
        }
        v.push(b);
        q = q.add(1);
    }
    Some(v)
}

unsafe fn count_blocks(mut b: *mut StringBlock) -> usize {
    let mut n = 0usize;
    while !b.is_null() && n < 1 << 20 {
        n += 1;
        b = (*b).next;
    }
    n
}

/// Snapshot a hash-map pointer (the value returned by `stbds_hmput_key` &c.,
/// i.e. `arr + elemsize`).
pub unsafe fn snap_map(p: *mut c_void, elemsize: usize) -> MapSnap {
    if p.is_null() {
        return MapSnap {
            null: true,
            length: 0,
            capacity: 0,
            temp: 0,
            elems: vec![],
            keys: vec![],
            table: None,
        };
    }
    let arr = (p as *mut u8).sub(elemsize);
    snap_arr(arr as *mut c_void, elemsize)
}

/// Snapshot a raw array pointer (as returned by `stbds_arrgrowf`).
pub unsafe fn snap_arr(arr: *mut c_void, elemsize: usize) -> MapSnap {
    if arr.is_null() {
        return MapSnap {
            null: true,
            length: 0,
            capacity: 0,
            temp: 0,
            elems: vec![],
            keys: vec![],
            table: None,
        };
    }
    let h = (arr as *mut u8).sub(HEADER_SIZE) as *mut ArrayHeader;
    let length = (*h).length;
    let capacity = (*h).capacity;
    let temp = (*h).temp;

    let ti = (*h).hash_table as *mut HashIndex;
    let key_is_ptr = if ti.is_null() {
        false
    } else {
        matches!((*ti).string.mode as c_int, SH_DEFAULT | SH_STRDUP | SH_ARENA)
    };

    let mut elems = Vec::new();
    let mut keys = Vec::new();
    let n = length.min(1 << 20);
    for i in 0..n {
        let e = (arr as *mut u8).add(i * elemsize);
        let mut raw = std::slice::from_raw_parts(e, elemsize).to_vec();
        if key_is_ptr && i > 0 && elemsize >= 8 {
            let kp = *(e as *mut *mut c_char);
            keys.push(read_cstr(kp));
            for b in raw[..8].iter_mut() {
                *b = 0;
            }
        } else {
            keys.push(None);
        }
        elems.push(raw);
    }

    let table = if ti.is_null() {
        None
    } else {
        let nb = ((*ti).slot_count >> BUCKET_SHIFT).min(1 << 16);
        let mut buckets = Vec::with_capacity(nb * BUCKET_LENGTH);
        for i in 0..nb {
            let b = (*ti).storage.add(i);
            for j in 0..BUCKET_LENGTH {
                buckets.push(((*b).hash[j], (*b).index[j]));
            }
        }
        Some(TableSnap {
            slot_count: (*ti).slot_count,
            used_count: (*ti).used_count,
            used_count_threshold: (*ti).used_count_threshold,
            used_count_shrink_threshold: (*ti).used_count_shrink_threshold,
            tombstone_count: (*ti).tombstone_count,
            tombstone_count_threshold: (*ti).tombstone_count_threshold,
            seed: (*ti).seed,
            slot_count_log2: (*ti).slot_count_log2,
            arena_remaining: (*ti).string.remaining,
            arena_block: (*ti).string.block,
            arena_mode: (*ti).string.mode,
            arena_blocks: count_blocks((*ti).string.storage),
            // NOTE: `temp_key` is *uninitialised* in a freshly built table
            // (`stbds_make_hash_index` never writes it) and only becomes
            // meaningful right after a string-mode put. Reading it here would
            // dereference garbage, so it is checked explicitly instead
            // (see `temp_key_of`).
            temp_key: None,
            buckets,
        })
    };

    MapSnap {
        null: false,
        length,
        capacity,
        temp,
        elems,
        keys,
        table,
    }
}

#[derive(PartialEq, Eq, Debug, Clone)]
pub struct ArenaSnap {
    pub remaining: usize,
    pub block: u8,
    pub mode: u8,
    pub blocks: usize,
    /// contents handed back by each stralloc call (as C strings)
    pub strings: Vec<Option<Vec<u8>>>,
}

pub unsafe fn snap_arena(a: &StringArena, strings: &[*mut c_char]) -> ArenaSnap {
    ArenaSnap {
        remaining: a.remaining,
        block: a.block,
        mode: a.mode,
        blocks: count_blocks(a.storage),
        strings: strings.iter().map(|p| read_cstr(*p)).collect(),
    }
}

// ---------------------------------------------------------------------------
// stdout capture (for `str_put`)
// ---------------------------------------------------------------------------

extern "C" {
    fn dup(fd: c_int) -> c_int;
    fn dup2(old: c_int, new: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn open(path: *const c_char, flags: c_int, ...) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
    fn lseek(fd: c_int, off: i64, whence: c_int) -> i64;
    fn read(fd: c_int, buf: *mut c_void, n: usize) -> isize;
    fn unlink(path: *const c_char) -> c_int;
}
const O_RDWR: c_int = 2;
const O_CREAT: c_int = 64;
const O_TRUNC: c_int = 512;

/// Run `f` with fd 1 redirected to a temp file and return everything written.
pub unsafe fn capture_stdout<F: FnOnce()>(tag: &str, f: F) -> Vec<u8> {
    let dir = std::env::var("TMPDIR").unwrap_or_else(|_| "/tmp".into());
    let path = format!("{dir}/diffcap_{}_{}.txt\0", tag, std::process::id());
    // Flush BOTH the libc streams and Rust's own buffered stdout: anything left
    // in either buffer would otherwise be written into the redirected fd.
    {
        use std::io::Write;
        let _ = std::io::stdout().flush();
    }
    fflush(std::ptr::null_mut());
    let saved = dup(1);
    let fd = open(path.as_ptr() as *const c_char, O_RDWR | O_CREAT | O_TRUNC, 0o600);
    assert!(fd >= 0, "open temp file failed");
    dup2(fd, 1);
    f();
    {
        use std::io::Write;
        let _ = std::io::stdout().flush();
    }
    fflush(std::ptr::null_mut());
    dup2(saved, 1);
    close(saved);
    lseek(fd, 0, 0);
    let mut out = Vec::new();
    let mut buf = [0u8; 4096];
    loop {
        let n = read(fd, buf.as_mut_ptr() as *mut c_void, buf.len());
        if n <= 0 {
            break;
        }
        out.extend_from_slice(&buf[..n as usize]);
    }
    close(fd);
    unlink(path.as_ptr() as *const c_char);
    out
}

// ---------------------------------------------------------------------------
// paired map driver: every operation is applied to the C map and the Rust map
// and the resulting state is compared.
// ---------------------------------------------------------------------------

pub struct MapPair {
    pub c: *mut c_void,
    pub r: *mut c_void,
    pub elemsize: usize,
    /// keeps caller-owned key buffers alive (SH_DEFAULT stores the pointer)
    pub keep: Vec<Box<[u8]>>,
    pub label: String,
}

impl MapPair {
    /// Start from a NULL map (`hmput_key`/`hmget_key` will create it).
    pub fn null(elemsize: usize, label: &str) -> MapPair {
        MapPair {
            c: std::ptr::null_mut(),
            r: std::ptr::null_mut(),
            elemsize,
            keep: Vec::new(),
            label: label.to_string(),
        }
    }

    /// Start from `stbds_shmode_func(elemsize, mode)` on both libraries.
    pub fn shmode(elemsize: usize, mode: c_int, label: &str) -> MapPair {
        let l = libs();
        unsafe {
            MapPair {
                c: (l.c.shmode_func)(elemsize, mode),
                r: (l.r.shmode_func)(elemsize, mode),
                elemsize,
                keep: Vec::new(),
                label: label.to_string(),
            }
        }
    }

    pub fn snap_c(&self) -> MapSnap {
        unsafe { snap_map(self.c, self.elemsize) }
    }
    pub fn snap_r(&self) -> MapSnap {
        unsafe { snap_map(self.r, self.elemsize) }
    }

    #[track_caller]
    pub fn assert_same(&self, what: &str) {
        assert_eq!(self.c.is_null(), self.r.is_null(), "[{}] {}: NULL-ness differs", self.label, what);
        let cs = self.snap_c();
        let rs = self.snap_r();
        if cs != rs {
            panic!(
                "[{}] {}: state diverged\n{}",
                self.label,
                what,
                compact_diff(&cs, &rs)
            );
        }
    }

    /// Register a key buffer (stable address) and hand back its pointer.
    pub fn own(&mut self, bytes: &[u8]) -> *mut c_void {
        let b: Box<[u8]> = bytes.to_vec().into_boxed_slice();
        let p = b.as_ptr() as *mut c_void;
        self.keep.push(b);
        p
    }

    pub fn put(&mut self, key: *mut c_void, keysize: usize, mode: c_int) {
        let l = libs();
        unsafe {
            self.c = (l.c.hmput_key)(self.c, self.elemsize, key, keysize, mode);
            self.r = (l.r.hmput_key)(self.r, self.elemsize, key, keysize, mode);
        }
    }

    /// Write `value` into the freshly-put/updated element (mimics what the
    /// `stbds_hmput` macro does with `stbds_temp`).
    pub unsafe fn write_value(&mut self, value_off: usize, value: &[u8]) {
        for (m, _) in [(self.c, 0), (self.r, 1)] {
            let arr = (m as *mut u8).sub(self.elemsize);
            let h = arr.sub(HEADER_SIZE) as *mut ArrayHeader;
            let t = (*h).temp;
            let e = (m as *mut u8).offset(t * self.elemsize as isize);
            std::ptr::copy_nonoverlapping(value.as_ptr(), e.add(value_off), value.len());
        }
    }

    /// Fill every byte of the current element from `from` to `elemsize` with a
    /// deterministic pattern. Necessary because `stbds_hmput_key` only writes
    /// `keysize` key bytes, leaving the rest of the element as raw `realloc`
    /// memory that legitimately differs between the two allocations.
    pub unsafe fn fill_tail(&mut self, from: usize, tag: u64) {
        for m in [self.c, self.r] {
            let arr = (m as *mut u8).sub(self.elemsize);
            let h = arr.sub(HEADER_SIZE) as *mut ArrayHeader;
            let t = (*h).temp;
            let e = (m as *mut u8).offset(t * self.elemsize as isize);
            for k in from..self.elemsize {
                *e.add(k) = (tag.wrapping_mul(31).wrapping_add(k as u64 * 7) & 0xff) as u8;
            }
        }
    }

    pub fn get(&mut self, key: *mut c_void, keysize: usize, mode: c_int) -> (isize, isize) {
        let l = libs();
        unsafe {
            self.c = (l.c.hmget_key)(self.c, self.elemsize, key, keysize, mode);
            self.r = (l.r.hmget_key)(self.r, self.elemsize, key, keysize, mode);
            (self.temp_c(), self.temp_r())
        }
    }

    pub fn get_ts(&mut self, key: *mut c_void, keysize: usize, mode: c_int) -> (isize, isize) {
        let l = libs();
        let mut tc: isize = 0x5555;
        let mut tr: isize = 0x5555;
        unsafe {
            self.c = (l.c.hmget_key_ts)(self.c, self.elemsize, key, keysize, &mut tc, mode);
            self.r = (l.r.hmget_key_ts)(self.r, self.elemsize, key, keysize, &mut tr, mode);
        }
        (tc, tr)
    }

    pub fn del(&mut self, key: *mut c_void, keysize: usize, keyoffset: usize, mode: c_int) {
        let l = libs();
        unsafe {
            self.c = (l.c.hmdel_key)(self.c, self.elemsize, key, keysize, keyoffset, mode);
            self.r = (l.r.hmdel_key)(self.r, self.elemsize, key, keysize, keyoffset, mode);
        }
    }

    pub fn put_default(&mut self) {
        let l = libs();
        unsafe {
            self.c = (l.c.hmput_default)(self.c, self.elemsize);
            self.r = (l.r.hmput_default)(self.r, self.elemsize);
        }
    }

    pub unsafe fn temp_c(&self) -> isize {
        if self.c.is_null() {
            return 0;
        }
        (*((self.c as *mut u8).sub(self.elemsize + HEADER_SIZE) as *mut ArrayHeader)).temp
    }
    pub unsafe fn temp_r(&self) -> isize {
        if self.r.is_null() {
            return 0;
        }
        (*((self.r as *mut u8).sub(self.elemsize + HEADER_SIZE) as *mut ArrayHeader)).temp
    }

    pub fn free(&mut self) {
        let l = libs();
        unsafe {
            if !self.c.is_null() {
                (l.c.hmfree_func)((self.c as *mut u8).sub(self.elemsize) as *mut c_void, self.elemsize);
            }
            if !self.r.is_null() {
                (l.r.hmfree_func)((self.r as *mut u8).sub(self.elemsize) as *mut c_void, self.elemsize);
            }
        }
        self.c = std::ptr::null_mut();
        self.r = std::ptr::null_mut();
    }
}

// ---------------------------------------------------------------------------
// serialisation: both libraries carry a *global* `stbds_hash_seed`, so tests
// must not interleave. Every test takes this guard first.
// ---------------------------------------------------------------------------

static TEST_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

pub struct Session(Option<std::sync::MutexGuard<'static, ()>>);

/// Serialise against every other test and reset both libraries' global seed.
pub fn session(seed: usize) -> Session {
    let g = match TEST_LOCK.lock() {
        Ok(g) => g,
        Err(p) => p.into_inner(),
    };
    seed_both(seed);
    Session(Some(g))
}

/// Compact, human-readable first-difference report between two map snapshots.
pub fn compact_diff(c: &MapSnap, r: &MapSnap) -> String {
    let mut out = String::new();
    macro_rules! f {
        ($n:expr, $a:expr, $b:expr) => {
            if $a != $b {
                out.push_str(&format!("  {:<32} C={:?}  RUST={:?}\n", $n, $a, $b));
            }
        };
    }
    f!("null", c.null, r.null);
    f!("header.length", c.length, r.length);
    f!("header.capacity", c.capacity, r.capacity);
    f!("header.temp", c.temp, r.temp);
    if c.elems != r.elems {
        let n = c.elems.len().max(r.elems.len());
        for i in 0..n {
            let a = c.elems.get(i);
            let b = r.elems.get(i);
            if a != b {
                out.push_str(&format!(
                    "  elem[{i}] (raw)                   C={:02x?}\n                                   RUST={:02x?}\n",
                    a, b
                ));
                if out.len() > 4000 {
                    out.push_str("  ...\n");
                    break;
                }
            }
        }
    }
    if c.keys != r.keys {
        let n = c.keys.len().max(r.keys.len());
        for i in 0..n {
            let a = c.keys.get(i);
            let b = r.keys.get(i);
            if a != b {
                out.push_str(&format!(
                    "  key[{i}]                          C={:?}  RUST={:?}\n",
                    a.map(|x| x.as_ref().map(|v| String::from_utf8_lossy(v).to_string())),
                    b.map(|x| x.as_ref().map(|v| String::from_utf8_lossy(v).to_string()))
                ));
                if out.len() > 6000 {
                    out.push_str("  ...\n");
                    break;
                }
            }
        }
    }
    match (&c.table, &r.table) {
        (None, None) => {}
        (a, b) if a.is_none() != b.is_none() => {
            out.push_str(&format!("  table present                    C={}  RUST={}\n", a.is_some(), b.is_some()));
        }
        (Some(a), Some(b)) => {
            f!("table.slot_count", a.slot_count, b.slot_count);
            f!("table.used_count", a.used_count, b.used_count);
            f!("table.used_count_threshold", a.used_count_threshold, b.used_count_threshold);
            f!("table.used_count_shrink_thr", a.used_count_shrink_threshold, b.used_count_shrink_threshold);
            f!("table.tombstone_count", a.tombstone_count, b.tombstone_count);
            f!("table.tombstone_count_thr", a.tombstone_count_threshold, b.tombstone_count_threshold);
            f!("table.seed", a.seed, b.seed);
            f!("table.slot_count_log2", a.slot_count_log2, b.slot_count_log2);
            f!("table.arena_remaining", a.arena_remaining, b.arena_remaining);
            f!("table.arena_block", a.arena_block, b.arena_block);
            f!("table.arena_mode", a.arena_mode, b.arena_mode);
            f!("table.arena_blocks", a.arena_blocks, b.arena_blocks);
            f!("table.temp_key", a.temp_key.as_ref().map(|v| String::from_utf8_lossy(v).to_string()),
                                b.temp_key.as_ref().map(|v| String::from_utf8_lossy(v).to_string()));
            if a.buckets != b.buckets {
                let n = a.buckets.len().max(b.buckets.len());
                let mut shown = 0;
                for i in 0..n {
                    if a.buckets.get(i) != b.buckets.get(i) {
                        out.push_str(&format!(
                            "  slot[{i}]                         C={:?}  RUST={:?}\n",
                            a.buckets.get(i), b.buckets.get(i)
                        ));
                        shown += 1;
                        if shown > 24 {
                            out.push_str("  ...\n");
                            break;
                        }
                    }
                }
            }
        }
        _ => {}
    }
    if out.is_empty() {
        out.push_str("  (snapshots differ but no field diff found?)\n");
    }
    out
}

/// Read `table->temp_key` as a C string. Only valid straight after a
/// string-mode `stbds_hmput_key`, which is the only place the C sets it.
pub unsafe fn temp_key_of(map: *mut c_void, elemsize: usize) -> Option<Vec<u8>> {
    if map.is_null() {
        return None;
    }
    let arr = (map as *mut u8).sub(elemsize);
    let h = arr.sub(HEADER_SIZE) as *mut ArrayHeader;
    let ti = (*h).hash_table as *mut HashIndex;
    if ti.is_null() {
        return None;
    }
    read_cstr((*ti).temp_key)
}

impl MapPair {
    /// After a string-mode put, both libraries must report the same key text in
    /// `table->temp_key` (the pointer itself differs for SH_STRDUP/SH_ARENA).
    #[track_caller]
    pub fn assert_temp_key(&self, expect: &[u8], what: &str) {
        let want: Vec<u8> = expect.iter().copied().take_while(|&b| b != 0).collect();
        unsafe {
            let c = temp_key_of(self.c, self.elemsize);
            let r = temp_key_of(self.r, self.elemsize);
            assert_eq!(c, r, "[{}] {}: temp_key text differs", self.label, what);
            assert_eq!(
                c.as_deref(),
                Some(&want[..]),
                "[{}] {}: temp_key content",
                self.label,
                what
            );
        }
    }
}
