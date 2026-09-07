//! Shared differential-test harness.
//!
//! Loads BOTH the C `.so` and the Rust `.so` with `libloading` and exposes the
//! 16 exported symbols of each behind identical function-pointer types, so every
//! call in every test crosses a real FFI boundary (exercising the
//! `#[no_mangle]` wrappers) for both implementations.

#![allow(dead_code)]

use std::ffi::{c_char, c_int, c_void};
use std::path::PathBuf;
use std::sync::{Mutex, MutexGuard, OnceLock};

// ---------------------------------------------------------------------------
// Layout-compatible mirrors of the C structs (for white-box state comparison)
// ---------------------------------------------------------------------------

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct ArrayHeader {
    pub length: usize,
    pub capacity: usize,
    pub hash_table: *mut c_void,
    pub temp: isize,
}

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct StringArena {
    pub storage: *mut c_void,
    pub remaining: usize,
    pub block: u8,
    pub mode: u8,
}

impl StringArena {
    pub fn new() -> Self {
        StringArena {
            storage: std::ptr::null_mut(),
            remaining: 0,
            block: 0,
            mode: 0,
        }
    }
}

pub const BUCKET_LENGTH: usize = 8;

#[repr(C)]
#[derive(Copy, Clone)]
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

pub const HM_BINARY: c_int = 0;
pub const HM_STRING: c_int = 1;
pub const SH_NONE: c_int = 0;
pub const SH_DEFAULT: c_int = 1;
pub const SH_STRDUP: c_int = 2;
pub const SH_ARENA: c_int = 3;

pub const INDEX_EMPTY: isize = -1;
pub const INDEX_DELETED: isize = -2;

// ---------------------------------------------------------------------------
// Exported-symbol signatures
// ---------------------------------------------------------------------------

pub type FnArrGrowF = unsafe extern "C" fn(*mut c_void, usize, usize, usize) -> *mut c_void;
pub type FnArrFreeF = unsafe extern "C" fn(*mut c_void);
pub type FnRandSeed = unsafe extern "C" fn(usize);
pub type FnHashString = unsafe extern "C" fn(*mut c_char, usize) -> usize;
pub type FnHashBytes = unsafe extern "C" fn(*mut c_void, usize, usize) -> usize;
pub type FnHmFreeFunc = unsafe extern "C" fn(*mut c_void, usize);
pub type FnHmGetKeyTs =
    unsafe extern "C" fn(*mut c_void, usize, *mut c_void, usize, *mut isize, c_int) -> *mut c_void;
pub type FnHmGetKey =
    unsafe extern "C" fn(*mut c_void, usize, *mut c_void, usize, c_int) -> *mut c_void;
pub type FnHmPutDefault = unsafe extern "C" fn(*mut c_void, usize) -> *mut c_void;
pub type FnHmPutKey =
    unsafe extern "C" fn(*mut c_void, usize, *mut c_void, usize, c_int) -> *mut c_void;
pub type FnShModeFunc = unsafe extern "C" fn(usize, c_int) -> *mut c_void;
pub type FnHmDelKey =
    unsafe extern "C" fn(*mut c_void, usize, *mut c_void, usize, usize, c_int) -> *mut c_void;
pub type FnStrAlloc = unsafe extern "C" fn(*mut StringArena, *mut c_char) -> *mut c_char;
pub type FnStrReset = unsafe extern "C" fn(*mut StringArena);
pub type FnStrKey = unsafe extern "C" fn(c_int) -> *mut c_char;
pub type FnShPuts = unsafe extern "C" fn(c_int);

/// One loaded implementation. `#[allow(dead_code)] _lib` keeps the handle alive.
pub struct Impl {
    pub name: &'static str,
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
    pub sh_puts: FnShPuts,
}

pub struct Libs {
    pub c: Impl,
    pub rs: Impl,
}

unsafe impl Send for Libs {}
unsafe impl Sync for Libs {}

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf()
}

fn find_c_so() -> PathBuf {
    let build = workspace_root().join("c_src/build");
    let mut candidates: Vec<PathBuf> = Vec::new();
    if let Ok(rd) = std::fs::read_dir(&build) {
        for e in rd.flatten() {
            let p = e.path();
            let name = p.file_name().unwrap().to_string_lossy().to_string();
            if name.starts_with("lib") && name.ends_with(".so") {
                candidates.push(p);
            }
        }
    }
    candidates.sort();
    assert!(
        !candidates.is_empty(),
        "no C .so found in {}; build it with: cd c_src && mkdir -p build && cd build && \
         cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
        build.display()
    );
    candidates.remove(0)
}

fn find_rust_so() -> PathBuf {
    // The integration-test binary lives in target/<profile>/deps/, so walk up.
    let exe = std::env::current_exe().expect("current_exe");
    let profile_dir = exe.parent().unwrap().parent().unwrap();
    let direct = profile_dir.join("libsh_puts_lib.so");
    if direct.exists() {
        return direct;
    }
    for p in ["target/debug", "target/release"] {
        let c = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join(p)
            .join("libsh_puts_lib.so");
        if c.exists() {
            return c;
        }
    }
    panic!("libsh_puts_lib.so not found (looked in {})", direct.display());
}

macro_rules! load {
    ($lib:expr, $t:ty, $name:literal) => {{
        let sym: libloading::Symbol<$t> = $lib
            .get($name.as_bytes())
            .unwrap_or_else(|e| panic!("symbol {} missing: {}", $name, e));
        *sym
    }};
}

unsafe fn load_impl(name: &'static str, path: &PathBuf) -> Impl {
    // Leak the Library so the function pointers stay valid for 'static.
    let lib: &'static libloading::Library = Box::leak(Box::new(
        libloading::Library::new(path).unwrap_or_else(|e| panic!("dlopen {:?}: {}", path, e)),
    ));
    Impl {
        name,
        arrgrowf: load!(lib, FnArrGrowF, "stbds_arrgrowf"),
        arrfreef: load!(lib, FnArrFreeF, "stbds_arrfreef"),
        rand_seed: load!(lib, FnRandSeed, "stbds_rand_seed"),
        hash_string: load!(lib, FnHashString, "stbds_hash_string"),
        hash_bytes: load!(lib, FnHashBytes, "stbds_hash_bytes"),
        hmfree_func: load!(lib, FnHmFreeFunc, "stbds_hmfree_func"),
        hmget_key_ts: load!(lib, FnHmGetKeyTs, "stbds_hmget_key_ts"),
        hmget_key: load!(lib, FnHmGetKey, "stbds_hmget_key"),
        hmput_default: load!(lib, FnHmPutDefault, "stbds_hmput_default"),
        hmput_key: load!(lib, FnHmPutKey, "stbds_hmput_key"),
        shmode_func: load!(lib, FnShModeFunc, "stbds_shmode_func"),
        hmdel_key: load!(lib, FnHmDelKey, "stbds_hmdel_key"),
        stralloc: load!(lib, FnStrAlloc, "stbds_stralloc"),
        strreset: load!(lib, FnStrReset, "stbds_strreset"),
        strkey: load!(lib, FnStrKey, "strkey"),
        sh_puts: load!(lib, FnShPuts, "sh_puts"),
    }
}

static LIBS: OnceLock<Libs> = OnceLock::new();
static GATE: Mutex<()> = Mutex::new(());

pub fn libs() -> &'static Libs {
    LIBS.get_or_init(|| unsafe {
        Libs {
            c: load_impl("C", &find_c_so()),
            rs: load_impl("Rust", &find_rust_so()),
        }
    })
}

/// Serialise every test (both libraries carry mutable file-static state:
/// `stbds_hash_seed` and `buffer`) and reset both global seeds to `seed`
/// so the two implementations start from an identical state.
pub fn run<F: FnOnce(&'static Impl, &'static Impl)>(seed: usize, f: F) {
    let libs = libs();
    let _g: MutexGuard<()> = GATE.lock().unwrap_or_else(|e| e.into_inner());
    unsafe {
        (libs.c.rand_seed)(seed);
        (libs.rs.rand_seed)(seed);
    }
    f(&libs.c, &libs.rs);
}

/// Same as [`run`] but also hands the guard-protected stdout capture facility.
pub fn run_locked<T, F: FnOnce(&'static Impl, &'static Impl) -> T>(seed: usize, f: F) -> T {
    let libs = libs();
    let _g: MutexGuard<()> = GATE.lock().unwrap_or_else(|e| e.into_inner());
    unsafe {
        (libs.c.rand_seed)(seed);
        (libs.rs.rand_seed)(seed);
    }
    f(&libs.c, &libs.rs)
}

// ---------------------------------------------------------------------------
// Deterministic RNG (SplitMix64) — fixed seeds for reproducibility
// ---------------------------------------------------------------------------

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
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
    pub fn below(&mut self, n: usize) -> usize {
        if n == 0 {
            0
        } else {
            (self.next_u64() % n as u64) as usize
        }
    }
    pub fn bytes(&mut self, n: usize) -> Vec<u8> {
        (0..n).map(|_| (self.next_u64() >> 24) as u8).collect()
    }
    /// Printable, NUL-free ASCII string of length `n`.
    pub fn ascii(&mut self, n: usize) -> Vec<u8> {
        let mut v: Vec<u8> = (0..n)
            .map(|_| b'a' + (self.next_u64() % 26) as u8)
            .collect();
        v.push(0);
        v
    }
}

// ---------------------------------------------------------------------------
// White-box state snapshots (identical strings <=> byte-identical behaviour)
// ---------------------------------------------------------------------------

#[derive(Copy, Clone, PartialEq, Eq)]
pub enum ElemFmt {
    /// Dump all `elemsize` bytes of each element verbatim.
    Raw,
    /// Element starts with a `char *` key at offset 0: render the pointed-to
    /// C string (pointers themselves differ between the two libraries) and dump
    /// the remaining `elemsize - 8` bytes verbatim.
    KeyPtr,
}

pub unsafe fn header_of(hash_ptr: *mut c_void, elemsize: usize) -> *mut ArrayHeader {
    let raw = (hash_ptr as *mut u8).wrapping_sub(elemsize);
    (raw as *mut ArrayHeader).wrapping_sub(1)
}

unsafe fn cstr(p: *const c_char) -> String {
    if p.is_null() {
        return "<null>".to_string();
    }
    let mut out = Vec::new();
    let mut q = p as *const u8;
    let mut n = 0usize;
    while *q != 0 && n < 8192 {
        out.push(*q);
        q = q.add(1);
        n += 1;
    }
    format!("{:?}", String::from_utf8_lossy(&out))
}

/// Full observable state of an array allocated by `stbds_arrgrowf`.
pub unsafe fn snap_array(a: *mut c_void, elemsize: usize, dump_len: usize) -> String {
    if a.is_null() {
        return "array<null>".to_string();
    }
    let h = (a as *mut ArrayHeader).wrapping_sub(1);
    let mut s = format!(
        "array len={} cap={} temp={} table={}",
        (*h).length,
        (*h).capacity,
        (*h).temp,
        if (*h).hash_table.is_null() { "null" } else { "set" }
    );
    if dump_len > 0 {
        let bytes = std::slice::from_raw_parts(a as *const u8, elemsize * dump_len);
        s.push_str(&format!(" bytes={:02x?}", bytes));
    }
    s
}

/// Full observable state of a hash map (element pointer, i.e. `raw + elemsize`).
///
/// `show_temp_key` must be `false` unless a string-mode insert has already run:
/// `stbds_make_hash_index` never initialises `temp_key`, so before the first
/// `stbds_temp_key(a) = …` write it is uninitialised `malloc` memory in both
/// implementations and comparing it would compare garbage.
pub unsafe fn snap_map(
    hash_ptr: *mut c_void,
    elemsize: usize,
    fmt: ElemFmt,
    show_temp_key: bool,
) -> String {
    if hash_ptr.is_null() {
        return "map<null>".to_string();
    }
    let raw = (hash_ptr as *mut u8).wrapping_sub(elemsize) as *mut c_void;
    let h = (raw as *mut ArrayHeader).wrapping_sub(1);
    let mut s = String::new();
    s.push_str(&format!(
        "map len={} cap={} temp={}\n",
        (*h).length,
        (*h).capacity,
        (*h).temp
    ));

    // elements 0..length (index 0 is the zeroed default slot)
    for i in 0..(*h).length {
        let e = (raw as *mut u8).wrapping_add(elemsize * i);
        match fmt {
            ElemFmt::Raw => {
                let b = std::slice::from_raw_parts(e as *const u8, elemsize);
                s.push_str(&format!("  e[{}]={:02x?}\n", i, b));
            }
            ElemFmt::KeyPtr => {
                let kp = *(e as *mut *mut c_char);
                let tail = if elemsize > 8 {
                    let b = std::slice::from_raw_parts(e.add(8) as *const u8, elemsize - 8);
                    format!("{:02x?}", b)
                } else {
                    "[]".to_string()
                };
                s.push_str(&format!("  e[{}] key={} tail={}\n", i, cstr(kp), tail));
            }
        }
    }

    let t = (*h).hash_table as *mut HashIndex;
    if t.is_null() {
        s.push_str("  table=<null>\n");
        return s;
    }
    s.push_str(&format!(
        "  table slots={} log2={} used={} used_thr={} shrink_thr={} tomb={} tomb_thr={} seed={:#x}\n",
        (*t).slot_count,
        (*t).slot_count_log2,
        (*t).used_count,
        (*t).used_count_threshold,
        (*t).used_count_shrink_threshold,
        (*t).tombstone_count,
        (*t).tombstone_count_threshold,
        (*t).seed
    ));
    s.push_str(&format!(
        "  arena remaining={} block={} mode={} storage={}\n",
        (*t).string.remaining,
        (*t).string.block,
        (*t).string.mode,
        if (*t).string.storage.is_null() {
            "null"
        } else {
            "set"
        }
    ));
    s.push_str(&format!(
        "  temp_key={}\n",
        if show_temp_key {
            cstr((*t).temp_key)
        } else {
            "<skipped:uninit>".to_string()
        }
    ));
    for b in 0..((*t).slot_count / BUCKET_LENGTH) {
        let bk = (*t).storage.wrapping_add(b);
        s.push_str(&format!(
            "  bucket[{}] hash={:#x?} index={:?}\n",
            b,
            (*bk).hash,
            (*bk).index
        ));
    }
    s
}

pub fn snap_arena(a: &StringArena) -> String {
    format!(
        "arena remaining={} block={} mode={} storage={}",
        a.remaining,
        a.block,
        a.mode,
        if a.storage.is_null() { "null" } else { "set" }
    )
}

// ---------------------------------------------------------------------------
// stdout capture (for `sh_puts`, which prints through libc `printf`)
// ---------------------------------------------------------------------------

extern "C" {
    fn dup(fd: c_int) -> c_int;
    fn dup2(old: c_int, new: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
    fn open(path: *const c_char, flags: c_int, ...) -> c_int;
}

const O_WRONLY: c_int = 1;
const O_CREAT: c_int = 64;
const O_TRUNC: c_int = 512;

/// Redirect fd 1 to a temp file, run `f`, restore, and return the raw bytes.
/// Both libraries share glibc's `stdout`, so `fflush(NULL)` is used to drain it.
pub unsafe fn capture_stdout<F: FnOnce()>(tag: &str, f: F) -> Vec<u8> {
    let path = std::env::temp_dir().join(format!(
        "shputs_cap_{}_{}_{}.txt",
        tag,
        std::process::id(),
        COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst)
    ));
    let cpath = std::ffi::CString::new(path.to_string_lossy().as_bytes()).unwrap();

    fflush(std::ptr::null_mut());
    let saved = dup(1);
    assert!(saved >= 0, "dup(1) failed");
    let fd = open(cpath.as_ptr(), O_WRONLY | O_CREAT | O_TRUNC, 0o644 as c_int);
    assert!(fd >= 0, "open({:?}) failed", path);
    assert!(dup2(fd, 1) >= 0, "dup2 failed");
    close(fd);

    f();

    fflush(std::ptr::null_mut());
    dup2(saved, 1);
    close(saved);

    let out = std::fs::read(&path).unwrap_or_default();
    let _ = std::fs::remove_file(&path);
    out
}

static COUNTER: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

// ---------------------------------------------------------------------------
// Snapshot diffing
// ---------------------------------------------------------------------------

/// Compare two snapshots, panicking with only the differing lines.
pub fn assert_snap_eq(sc: &str, sr: &str, ctx: &str) {
    if sc == sr {
        return;
    }
    let cl: Vec<&str> = sc.lines().collect();
    let rl: Vec<&str> = sr.lines().collect();
    let mut msg = format!("{}\nsnapshot divergence (C vs Rust):\n", ctx);
    for i in 0..cl.len().max(rl.len()) {
        let a = cl.get(i).copied().unwrap_or("<missing>");
        let b = rl.get(i).copied().unwrap_or("<missing>");
        if a != b {
            msg.push_str(&format!("  line {}:\n    C   : {}\n    Rust: {}\n", i, a, b));
        }
    }
    panic!("{}", msg);
}


/// `stbds_hmput(t,k,v)` for an element laid out as
/// `{ key: [u8; keysize], payload: [u8; elemsize-keysize] }`:
///
/// ```c
/// (t) = stbds_hmput_key((t), sizeof *(t), &k, sizeof (t)->key, MODE);
/// (t)[stbds_temp((t)-1)].key   = (k);
/// (t)[stbds_temp((t)-1)].value = (v);
/// ```
pub unsafe fn hm_put(
    imp: &Impl,
    map: *mut c_void,
    elemsize: usize,
    key: &[u8],
    keysize: usize,
    payload: &[u8],
    mode: c_int,
) -> *mut c_void {
    let mut k = key.to_vec();
    k.resize(keysize.max(key.len()), 0);
    let m = (imp.hmput_key)(map, elemsize, k.as_mut_ptr() as *mut c_void, keysize, mode);
    let t = (*header_of(m, elemsize)).temp;
    let e = (m as *mut u8).offset(t * elemsize as isize);
    std::ptr::copy_nonoverlapping(k.as_ptr(), e, keysize);
    if elemsize > keysize {
        let n = (elemsize - keysize).min(payload.len());
        std::ptr::copy_nonoverlapping(payload.as_ptr(), e.add(keysize), n);
    }
    m
}

/// `stbds_shput(t,k,v)` — string map insert that writes **only** the value:
///
/// ```c
/// (t) = stbds_hmput_key((t), sizeof *(t), (void*)(k), sizeof (t)->key, STBDS_HM_STRING);
/// (t)[stbds_temp((t)-1)].value = (v);
/// ```
///
/// Unlike `shputs` this never reads `stbds_temp_key`, so it is also valid for
/// `string.mode` values that take the `switch` `default:` (memcpy) branch.
pub unsafe fn sh_put_v(
    imp: &Impl,
    map: *mut c_void,
    elemsize: usize,
    key: *mut c_char,
    payload: &[u8],
    mode: c_int,
) -> *mut c_void {
    let keysize = std::mem::size_of::<*mut c_char>();
    let m = (imp.hmput_key)(map, elemsize, key as *mut c_void, keysize, mode);
    let t = (*header_of(m, elemsize)).temp;
    let e = (m as *mut u8).offset(t * elemsize as isize);
    if elemsize > keysize {
        let n = (elemsize - keysize).min(payload.len());
        std::ptr::copy_nonoverlapping(payload.as_ptr(), e.add(keysize), n);
    }
    m
}

/// `stbds_shputs(t,s)` — string map insert.
///
/// ```c
/// (t) = stbds_hmput_key((t), sizeof *(t), (void*)(s).key, sizeof (s).key, STBDS_HM_STRING);
/// (t)[stbds_temp((t)-1)]     = (s);
/// (t)[stbds_temp((t)-1)].key = stbds_temp_key((t)-1);
/// ```
///
/// The element is `{ char *key; u8 payload[elemsize-8]; }`.
pub unsafe fn sh_put(
    imp: &Impl,
    map: *mut c_void,
    elemsize: usize,
    key: *mut c_char,
    payload: &[u8],
    mode: c_int,
) -> *mut c_void {
    let keysize = std::mem::size_of::<*mut c_char>();
    let m = (imp.hmput_key)(map, elemsize, key as *mut c_void, keysize, mode);
    let raw = (m as *mut u8).wrapping_sub(elemsize) as *mut c_void;
    let t = (*header_of(m, elemsize)).temp;
    let e = (m as *mut u8).offset(t * elemsize as isize);
    // (t)[temp] = s  — write the whole struct (key pointer + payload) ...
    *(e as *mut *mut c_char) = key;
    if elemsize > keysize {
        let n = (elemsize - keysize).min(payload.len());
        std::ptr::copy_nonoverlapping(payload.as_ptr(), e.add(keysize), n);
    }
    // ... then overwrite .key with the library-owned copy.
    let tk = *((*((raw as *mut ArrayHeader).wrapping_sub(1))).hash_table as *mut *mut c_char);
    *(e as *mut *mut c_char) = tk;
    m
}

/// `stbds_hmgeti(t,k)` / `stbds_shgeti(t,k)`: returns `(new_map, temp)`.
pub unsafe fn hm_get(
    imp: &Impl,
    map: *mut c_void,
    elemsize: usize,
    key: &[u8],
    keysize: usize,
    mode: c_int,
) -> (*mut c_void, isize) {
    let mut k = key.to_vec();
    k.resize(keysize.max(key.len()), 0);
    let m = (imp.hmget_key)(map, elemsize, k.as_mut_ptr() as *mut c_void, keysize, mode);
    (m, (*header_of(m, elemsize)).temp)
}

/// String-key variant: the key is passed as a `char *` directly.
pub unsafe fn sh_get(
    imp: &Impl,
    map: *mut c_void,
    elemsize: usize,
    key: *mut c_char,
    mode: c_int,
) -> (*mut c_void, isize) {
    let keysize = std::mem::size_of::<*mut c_char>();
    let m = (imp.hmget_key)(map, elemsize, key as *mut c_void, keysize, mode);
    (m, (*header_of(m, elemsize)).temp)
}

/// `stbds_hmgeti_ts(t,k,temp)`: returns `(new_map, temp)` via the out-param.
pub unsafe fn hm_get_ts(
    imp: &Impl,
    map: *mut c_void,
    elemsize: usize,
    key: &[u8],
    keysize: usize,
    mode: c_int,
) -> (*mut c_void, isize) {
    let mut k = key.to_vec();
    k.resize(keysize.max(key.len()), 0);
    let mut temp: isize = 0xBAAD;
    let m = (imp.hmget_key_ts)(
        map,
        elemsize,
        k.as_mut_ptr() as *mut c_void,
        keysize,
        &mut temp,
        mode,
    );
    (m, temp)
}

/// `stbds_hmdel(t,k)`: returns `(new_map, temp)` (`temp` is 1 on delete, 0 on reject).
pub unsafe fn hm_del(
    imp: &Impl,
    map: *mut c_void,
    elemsize: usize,
    key: &[u8],
    keysize: usize,
    keyoffset: usize,
    mode: c_int,
) -> (*mut c_void, isize) {
    let mut k = key.to_vec();
    k.resize(keysize.max(key.len()), 0);
    let m = (imp.hmdel_key)(
        map,
        elemsize,
        k.as_mut_ptr() as *mut c_void,
        keysize,
        keyoffset,
        mode,
    );
    if m.is_null() {
        (m, 0)
    } else {
        (m, (*header_of(m, elemsize)).temp)
    }
}

pub unsafe fn sh_del(
    imp: &Impl,
    map: *mut c_void,
    elemsize: usize,
    key: *mut c_char,
    keyoffset: usize,
    mode: c_int,
) -> (*mut c_void, isize) {
    let keysize = std::mem::size_of::<*mut c_char>();
    let m = (imp.hmdel_key)(
        map,
        elemsize,
        key as *mut c_void,
        keysize,
        keyoffset,
        mode,
    );
    if m.is_null() {
        (m, 0)
    } else {
        (m, (*header_of(m, elemsize)).temp)
    }
}

/// `stbds_hmfree(t)` / `stbds_shfree(t)`.
pub unsafe fn hm_free(imp: &Impl, map: *mut c_void, elemsize: usize) {
    if !map.is_null() {
        (imp.hmfree_func)((map as *mut u8).wrapping_sub(elemsize) as *mut c_void, elemsize);
    }
}

/// `stbds_hmlen(t)`.
pub unsafe fn hm_len(map: *mut c_void, elemsize: usize) -> isize {
    if map.is_null() {
        0
    } else {
        (*header_of(map, elemsize)).length as isize - 1
    }
}

// ---------------------------------------------------------------------------
// Map drivers — faithful expansions of the stbds_* macros in lib.c
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// Fork helper — for inputs where the C `assert()`s fire (SIGABRT)
// ---------------------------------------------------------------------------

extern "C" {
    fn fork() -> c_int;
    fn waitpid(pid: c_int, status: *mut c_int, options: c_int) -> c_int;
    fn _exit(code: c_int) -> !;
}

#[derive(Debug, PartialEq, Eq)]
pub enum Outcome {
    Exited(i32),
    Signalled(i32),
}

/// Run `f` in a forked child and report how the child terminated.  Used for the
/// error rows where the C library's live `assert()` aborts the process — the
/// only way to compare that behaviour without killing the test harness.
pub unsafe fn fork_outcome<F: FnOnce()>(f: F) -> Outcome {
    let pid = fork();
    assert!(pid >= 0, "fork failed");
    if pid == 0 {
        // Child: silence stdout/stderr noise is not needed; just run and exit.
        f();
        fflush(std::ptr::null_mut());
        _exit(0);
    }
    let mut status: c_int = 0;
    let w = waitpid(pid, &mut status, 0);
    assert_eq!(w, pid, "waitpid failed");
    // WIFSIGNALED / WTERMSIG / WEXITSTATUS
    if status & 0x7f != 0 && status & 0x7f != 0x7f {
        Outcome::Signalled(status & 0x7f)
    } else {
        Outcome::Exited((status >> 8) & 0xff)
    }
}
