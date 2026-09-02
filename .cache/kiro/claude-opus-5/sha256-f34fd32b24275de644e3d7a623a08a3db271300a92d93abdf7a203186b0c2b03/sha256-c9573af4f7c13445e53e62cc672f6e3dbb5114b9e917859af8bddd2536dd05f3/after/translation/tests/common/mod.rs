//! Differential-test harness.
//!
//! Loads BOTH shared objects through `libloading` and calls every function only
//! through its exported `extern "C"` symbol — never through the Rust crate
//! directly — so the `#[no_mangle]` wrappers are part of what is under test.

#![allow(dead_code)]

use std::ffi::{c_char, c_int, c_void, CStr, CString};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard, OnceLock};

use libloading::{Library, Symbol};

// ---------------------------------------------------------------------------
// C ABI layout constants (must mirror `c_src/src/lib.c`)
// ---------------------------------------------------------------------------

/// `sizeof(stbds_array_header)` = 2*size_t + void* + ptrdiff_t
pub const HDR_SIZE: usize = 32;
/// `sizeof(stbds_hash_bucket)` = 8*size_t + 8*ptrdiff_t
pub const BUCKET_SIZE: usize = 128;
pub const BUCKET_LENGTH: usize = 8;
pub const BUCKET_SHIFT: usize = 3;
pub const BUCKET_MASK: usize = 7;
pub const STBDS_CACHE_LINE: usize = 64;

pub const HM_BINARY: c_int = 0;
pub const HM_STRING: c_int = 1;

pub const SH_NONE: c_int = 0;
pub const SH_DEFAULT: c_int = 1;
pub const SH_STRDUP: c_int = 2;
pub const SH_ARENA: c_int = 3;

pub const INDEX_EMPTY: isize = -1;
pub const INDEX_DELETED: isize = -2;

/// `struct stbds_string_arena` — 24 bytes on LP64.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct StringArena {
    pub storage: *mut c_void,
    pub remaining: usize,
    pub block: u8,
    pub mode: u8,
    pub _pad: [u8; 6],
}

impl StringArena {
    pub fn zeroed() -> Self {
        StringArena {
            storage: std::ptr::null_mut(),
            remaining: 0,
            block: 0,
            mode: 0,
            _pad: [0; 6],
        }
    }
}

const _: () = assert!(std::mem::size_of::<StringArena>() == 24);

/// Field offsets inside `stbds_hash_index`.
pub mod hi {
    pub const TEMP_KEY: usize = 0;
    pub const SLOT_COUNT: usize = 8;
    pub const USED_COUNT: usize = 16;
    pub const USED_COUNT_THRESHOLD: usize = 24;
    pub const USED_COUNT_SHRINK_THRESHOLD: usize = 32;
    pub const TOMBSTONE_COUNT: usize = 40;
    pub const TOMBSTONE_COUNT_THRESHOLD: usize = 48;
    pub const SEED: usize = 56;
    pub const SLOT_COUNT_LOG2: usize = 64;
    pub const STRING: usize = 72; // stbds_string_arena, 24 bytes
    pub const STORAGE: usize = 96;
    pub const SIZE: usize = 104;
}

// ---------------------------------------------------------------------------
// libc bits the harness itself needs
// ---------------------------------------------------------------------------

unsafe extern "C" {
    fn fflush(stream: *mut c_void) -> c_int;
    fn dup2(old: c_int, new: c_int) -> c_int;
    pub fn fork() -> c_int;
    pub fn waitpid(pid: c_int, status: *mut c_int, options: c_int) -> c_int;
    pub fn _exit(code: c_int) -> !;
}

// ---------------------------------------------------------------------------
// The loaded API
// ---------------------------------------------------------------------------

pub type FnArrGrowf = unsafe extern "C" fn(*mut c_void, usize, usize, usize) -> *mut c_void;
pub type FnArrFreef = unsafe extern "C" fn(*mut c_void);
pub type FnRandSeed = unsafe extern "C" fn(usize);
pub type FnHashString = unsafe extern "C" fn(*mut c_char, usize) -> usize;
pub type FnHashBytes = unsafe extern "C" fn(*mut c_void, usize, usize) -> usize;
pub type FnHmFree = unsafe extern "C" fn(*mut c_void, usize);
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
pub type FnStrPut = unsafe extern "C" fn(c_int);

pub struct Api {
    pub name: &'static str,
    _lib: Library,
    pub arrgrowf: FnArrGrowf,
    pub arrfreef: FnArrFreef,
    pub rand_seed: FnRandSeed,
    pub hash_string: FnHashString,
    pub hash_bytes: FnHashBytes,
    pub hmfree_func: FnHmFree,
    pub hmget_key_ts: FnHmGetKeyTs,
    pub hmget_key: FnHmGetKey,
    pub hmput_default: FnHmPutDefault,
    pub hmput_key: FnHmPutKey,
    pub shmode_func: FnShModeFunc,
    pub hmdel_key: FnHmDelKey,
    pub stralloc: FnStrAlloc,
    pub strreset: FnStrReset,
    pub strkey: FnStrKey,
    pub str_put: FnStrPut,
}

unsafe fn sym<T: Copy>(lib: &Library, n: &[u8]) -> T {
    let s: Symbol<T> = lib
        .get(n)
        .unwrap_or_else(|e| panic!("missing symbol {}: {e}", String::from_utf8_lossy(n)));
    *s
}

impl Api {
    unsafe fn load(name: &'static str, path: &Path) -> Api {
        let lib = Library::new(path).unwrap_or_else(|e| panic!("dlopen {path:?}: {e}"));
        Api {
            name,
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
            str_put: sym(&lib, b"str_put\0"),
            _lib: lib,
        }
    }
}

fn root() -> PathBuf {
    // CARGO_MANIFEST_DIR == <work>/translation
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf()
}

fn find_c_so() -> PathBuf {
    let dir = root().join("c_src/build");
    let mut best: Option<PathBuf> = None;
    if let Ok(rd) = std::fs::read_dir(&dir) {
        for e in rd.flatten() {
            let p = e.path();
            let n = p.file_name().unwrap().to_string_lossy().to_string();
            if n.starts_with("lib") && n.ends_with(".so") {
                best = Some(p);
            }
        }
    }
    best.unwrap_or_else(|| {
        panic!(
            "no C .so found in {dir:?}; build it with:\n  cd c_src && mkdir -p build && cd build \
             && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build ."
        )
    })
}

fn find_rust_so() -> PathBuf {
    let base = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target");
    for prof in ["release", "debug"] {
        let p = base.join(prof).join("libstr_put_lib.so");
        if p.exists() {
            RUST_SO_PROFILE.set(prof).ok();
            return p;
        }
    }
    panic!("no Rust .so found; run `cargo build --release` first");
}

static RUST_SO_PROFILE: OnceLock<&'static str> = OnceLock::new();

/// Which cargo profile produced the Rust `.so` that was loaded.
///
/// `Cargo.toml` declares `[profile.release] panic = "abort"` and the crate's
/// only artifact is a `cdylib`, so `release` is the shipped configuration.  The
/// intentionally-UB rows of `ERRORS.md` (null deref, allocation failure) abort
/// through a different mechanism in a `dev` build, so the crash-equivalence
/// tests are only meaningful against the release artifact.
pub fn rust_so_profile() -> &'static str {
    both();
    RUST_SO_PROFILE.get().copied().unwrap_or("unknown")
}

pub struct Pair {
    pub c: Api,
    pub r: Api,
}

static PAIR: OnceLock<Pair> = OnceLock::new();
static LOCK: Mutex<()> = Mutex::new(());

/// Both libraries plus the process-wide lock.  The libraries carry mutable
/// global state (`stbds_hash_seed`, `strkey`'s `buffer`) and the tests redirect
/// fd 1, so every test body must be serialised.
pub fn both() -> (&'static Api, &'static Api, MutexGuard<'static, ()>) {
    let g = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let p = PAIR.get_or_init(|| unsafe {
        Pair {
            c: Api::load("C", &find_c_so()),
            r: Api::load("RUST", &find_rust_so()),
        }
    });
    (&p.c, &p.r, g)
}

// ---------------------------------------------------------------------------
// Deterministic RNG (xorshift64*), fixed seed for reproducibility
// ---------------------------------------------------------------------------

pub const SEED: u64 = 0x5EED_1234_ABCD_0001;

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
        x.wrapping_mul(0x2545F4914F6CDD1D)
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
    /// Printable, NUL-free string of length `n`.
    pub fn ascii(&mut self, n: usize) -> CString {
        let v: Vec<u8> = (0..n)
            .map(|_| 33 + ((self.next_u64() >> 20) % 94) as u8)
            .collect();
        CString::new(v).unwrap()
    }
    /// Printable, NUL-free string with length in `lo..lo+span`.
    pub fn ascii_len(&mut self, lo: usize, span: usize) -> CString {
        let n = lo + self.below(span);
        self.ascii(n)
    }
    /// Arbitrary NUL-free bytes (including >= 0x80) of length `n`.
    pub fn highbytes(&mut self, n: usize) -> CString {        let v: Vec<u8> = (0..n)
            .map(|_| {
                let b = (self.next_u64() >> 24) as u8;
                if b == 0 {
                    1
                } else {
                    b
                }
            })
            .collect();
        CString::new(v).unwrap()
    }
}

// ---------------------------------------------------------------------------
// Structural dumping
// ---------------------------------------------------------------------------

/// How the key region of an element must be interpreted when dumping.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KeyKind {
    /// `keysize` raw bytes at `keyoffset`.
    Bytes,
    /// A `char *` at `keyoffset`; dump the pointed-to C string instead of the
    /// (allocator-dependent) pointer value.
    CStrPtr,
}

#[derive(Clone, Copy, Debug)]
pub struct MapShape {
    pub elemsize: usize,
    pub keyoffset: usize,
    pub keysize: usize,
    pub kind: KeyKind,
}

impl MapShape {
    pub fn bytes(elemsize: usize, keysize: usize) -> MapShape {
        MapShape {
            elemsize,
            keyoffset: 0,
            keysize,
            kind: KeyKind::Bytes,
        }
    }
    pub fn strp(elemsize: usize) -> MapShape {
        MapShape {
            elemsize,
            keyoffset: 0,
            keysize: 8,
            kind: KeyKind::CStrPtr,
        }
    }
    /// The byte range of the element that the *library* owns.
    pub fn key_region(&self) -> (usize, usize) {
        match self.kind {
            KeyKind::Bytes => (self.keyoffset, self.keysize),
            KeyKind::CStrPtr => (self.keyoffset, 8),
        }
    }
}

fn push_usize(out: &mut Vec<u8>, v: usize) {
    out.extend_from_slice(&(v as u64).to_le_bytes());
}
fn push_isize(out: &mut Vec<u8>, v: isize) {
    out.extend_from_slice(&(v as i64).to_le_bytes());
}

unsafe fn rd_usize(p: *const u8) -> usize {
    std::ptr::read_unaligned(p as *const usize)
}
unsafe fn rd_isize(p: *const u8) -> isize {
    std::ptr::read_unaligned(p as *const isize)
}

/// Canonical byte image of an `stbds_array_header` + payload, as returned by
/// `stbds_arrgrowf` (i.e. `arr` points at element 0).
///
/// `payload_len` bytes of payload are included verbatim.
pub unsafe fn dump_arr(arr: *mut c_void, payload_len: usize) -> Vec<u8> {
    let mut out = Vec::new();
    if arr.is_null() {
        out.extend_from_slice(b"NULLARR");
        return out;
    }
    let h = (arr as *mut u8).sub(HDR_SIZE);
    push_usize(&mut out, rd_usize(h)); // length
    push_usize(&mut out, rd_usize(h.add(8))); // capacity
    out.push((rd_usize(h.add(16)) != 0) as u8); // hash_table != NULL
    push_isize(&mut out, rd_isize(h.add(24))); // temp
    out.extend_from_slice(std::slice::from_raw_parts(arr as *const u8, payload_len));
    out
}

/// Canonical byte image of a whole hash map, given the "hash pointer" that the
/// `stbds_hm*` functions return (element `-1` is the default element).
pub unsafe fn dump_map(hash_ptr: *mut c_void, shape: MapShape) -> Vec<u8> {
    let mut out = Vec::new();
    if hash_ptr.is_null() {
        out.extend_from_slice(b"NULLMAP");
        return out;
    }
    let arr = (hash_ptr as *mut u8).sub(shape.elemsize);
    let h = arr.sub(HDR_SIZE);
    let length = rd_usize(h);
    let capacity = rd_usize(h.add(8));
    let table = rd_usize(h.add(16)) as *mut u8;
    let temp = rd_isize(h.add(24));

    out.extend_from_slice(b"HDR");
    push_usize(&mut out, length);
    push_usize(&mut out, capacity);
    push_isize(&mut out, temp);

    out.extend_from_slice(b"ELEMS");
    push_usize(&mut out, length);
    let (koff, klen) = shape.key_region();
    for i in 0..length {
        let e = arr.add(shape.elemsize * i);
        // bytes before the key region
        out.extend_from_slice(std::slice::from_raw_parts(e, koff));
        match shape.kind {
            KeyKind::Bytes => {
                out.extend_from_slice(std::slice::from_raw_parts(e.add(koff), klen));
            }
            KeyKind::CStrPtr => {
                let p = rd_usize(e.add(koff)) as *const c_char;
                if p.is_null() {
                    out.extend_from_slice(b"<null>");
                } else {
                    out.extend_from_slice(b"<s>");
                    out.extend_from_slice(CStr::from_ptr(p).to_bytes());
                }
                out.push(0);
            }
        }
        // bytes after the key region
        out.extend_from_slice(std::slice::from_raw_parts(
            e.add(koff + klen),
            shape.elemsize - koff - klen,
        ));
        out.push(b'|');
    }

    out.extend_from_slice(b"TABLE");
    if table.is_null() {
        out.extend_from_slice(b"<none>");
        return out;
    }
    let slot_count = rd_usize(table.add(hi::SLOT_COUNT));
    push_usize(&mut out, slot_count);
    push_usize(&mut out, rd_usize(table.add(hi::USED_COUNT)));
    push_usize(&mut out, rd_usize(table.add(hi::USED_COUNT_THRESHOLD)));
    push_usize(&mut out, rd_usize(table.add(hi::USED_COUNT_SHRINK_THRESHOLD)));
    push_usize(&mut out, rd_usize(table.add(hi::TOMBSTONE_COUNT)));
    push_usize(&mut out, rd_usize(table.add(hi::TOMBSTONE_COUNT_THRESHOLD)));
    push_usize(&mut out, rd_usize(table.add(hi::SEED)));
    push_usize(&mut out, rd_usize(table.add(hi::SLOT_COUNT_LOG2)));
    // string arena: pointer value is allocator dependent, the rest is state
    let sa = table.add(hi::STRING);
    out.push((rd_usize(sa) != 0) as u8);
    push_usize(&mut out, rd_usize(sa.add(8)));
    out.push(*sa.add(16)); // block
    out.push(*sa.add(17)); // mode

    out.extend_from_slice(b"BUCKETS");
    let storage = rd_usize(table.add(hi::STORAGE)) as *mut u8;
    // The absolute address of `storage` is allocator dependent, but the
    // `STBDS_ALIGN_FWD((size_t)(t+1), 64)` computation itself is observable:
    // it must be 64-byte aligned and sit within the 64 padding bytes that
    // `stbds_make_hash_index` over-allocates.
    out.push(((storage as usize) % STBDS_CACHE_LINE == 0) as u8);
    out.push(((storage as usize) >= table as usize + hi::SIZE) as u8);
    out.push(((storage as usize) < table as usize + hi::SIZE + STBDS_CACHE_LINE) as u8);
    for b in 0..(slot_count >> BUCKET_SHIFT) {
        let bk = storage.add(b * BUCKET_SIZE);
        for j in 0..BUCKET_LENGTH {
            push_usize(&mut out, rd_usize(bk.add(8 * j)));
        }
        for j in 0..BUCKET_LENGTH {
            push_isize(&mut out, rd_isize(bk.add(64 + 8 * j)));
        }
    }
    out
}

/// Dump the `temp_key` field (`stbds_temp_key`) as the pointed-to string.
pub unsafe fn dump_temp_key(hash_ptr: *mut c_void, elemsize: usize) -> Vec<u8> {
    let arr = (hash_ptr as *mut u8).sub(elemsize);
    let table = rd_usize(arr.sub(HDR_SIZE).add(16)) as *mut u8;
    if table.is_null() {
        return b"<no-table>".to_vec();
    }
    let p = rd_usize(table.add(hi::TEMP_KEY)) as *const c_char;
    if p.is_null() {
        return b"<null>".to_vec();
    }
    let mut v = b"<s>".to_vec();
    v.extend_from_slice(CStr::from_ptr(p).to_bytes());
    v
}

/// `stbds_header(x)->temp`
pub unsafe fn arr_temp(arr: *mut c_void) -> isize {
    rd_isize((arr as *mut u8).sub(HDR_SIZE).add(24))
}
/// `stbds_header(x)->length`
pub unsafe fn arr_length(arr: *mut c_void) -> usize {
    rd_usize((arr as *mut u8).sub(HDR_SIZE))
}
/// `stbds_header(x)->capacity`
pub unsafe fn arr_capacity(arr: *mut c_void) -> usize {
    rd_usize((arr as *mut u8).sub(HDR_SIZE).add(8))
}
/// `stbds_header(x)->hash_table`
pub unsafe fn arr_table(arr: *mut c_void) -> *mut u8 {
    rd_usize((arr as *mut u8).sub(HDR_SIZE).add(16)) as *mut u8
}

/// `stbds_header(x)->hash_table` of a map, given its hash pointer.
pub unsafe fn map_table(hash_ptr: *mut c_void, elemsize: usize) -> *mut u8 {
    arr_table((hash_ptr as *mut u8).sub(elemsize) as *mut c_void)
}

/// Overwrite `stbds_hash_index::temp_key`.
///
/// `stbds_make_hash_index` never initialises this field, so after a rehash it
/// holds uninitialised heap bytes that legitimately differ between the two
/// libraries.  Tests prime it to a known value so that "did the library write
/// `temp_key`?" becomes an observable, comparable fact.
pub unsafe fn set_temp_key(hash_ptr: *mut c_void, elemsize: usize, v: usize) {
    let t = map_table(hash_ptr, elemsize);
    if !t.is_null() {
        std::ptr::write_unaligned(t as *mut usize, v);
    }
}

/// `stbds_hmlen`: `stbds_header(t-1)->length - 1`
pub unsafe fn hmlen(hash_ptr: *mut c_void, elemsize: usize) -> isize {
    if hash_ptr.is_null() {
        return 0;
    }
    arr_length((hash_ptr as *mut u8).sub(elemsize) as *mut c_void) as isize - 1
}

/// `stbds_hmfree(p)` — the macro passes `(p)-1`, i.e. the *array* pointer.
pub unsafe fn hmfree(api: &Api, hash_ptr: *mut c_void, elemsize: usize) {
    if hash_ptr.is_null() {
        return;
    }
    (api.hmfree_func)((hash_ptr as *mut u8).sub(elemsize) as *mut c_void, elemsize);
}

/// Fill everything in element `i` that the *library* does not own with a
/// deterministic pattern, so the whole element becomes comparable.
pub unsafe fn fill_value(hash_ptr: *mut c_void, shape: MapShape, i: usize, tag: u64) {
    let arr = (hash_ptr as *mut u8).sub(shape.elemsize);
    let e = arr.add(shape.elemsize * i);
    let (koff, klen) = shape.key_region();
    for b in 0..shape.elemsize {
        if b >= koff && b < koff + klen {
            continue;
        }
        *e.add(b) = (tag.wrapping_mul(31).wrapping_add(b as u64 * 7) & 0xff) as u8;
    }
}

/// Dump of a `stbds_string_arena` (pointer values excluded, block chain walked
/// only for its length because block sizes are implied by `remaining`).
pub unsafe fn dump_arena(a: *const StringArena) -> Vec<u8> {
    let mut out = Vec::new();
    push_usize(&mut out, (*a).remaining);
    out.push((*a).block);
    out.push((*a).mode);
    let mut n = 0usize;
    let mut x = (*a).storage as *mut *mut c_void;
    while !x.is_null() {
        n += 1;
        if n > 100_000 {
            break;
        }
        x = *x as *mut *mut c_void;
    }
    push_usize(&mut out, n);
    out
}

// ---------------------------------------------------------------------------
// stdout capture (for `str_put`)
// ---------------------------------------------------------------------------

/// Capture everything `f` writes to fd 1.
///
/// The call runs in a **forked child** so that the redirection cannot race with
/// the test harness's own writes to fd 1 from other threads (which would inject
/// `test ... ok` lines into the capture and produce spurious divergences).
pub fn capture_stdout<F: FnOnce()>(tag: &str, f: F) -> Vec<u8> {
    use std::os::unix::io::AsRawFd;
    let path = std::env::temp_dir().join(format!(
        "strput_cap_{}_{}.txt",
        std::process::id(),
        tag.replace('/', "_")
    ));
    let file = std::fs::File::create(&path).unwrap();
    let fd = file.as_raw_fd();
    unsafe {
        // don't let already-buffered parent output be duplicated into the child
        fflush(std::ptr::null_mut());
        let pid = fork();
        assert!(pid >= 0, "fork failed");
        if pid == 0 {
            if dup2(fd, 1) < 0 {
                _exit(101);
            }
            f();
            fflush(std::ptr::null_mut());
            _exit(0);
        }
        let mut st: c_int = 0;
        assert!(waitpid(pid, &mut st, 0) == pid, "waitpid failed");
        assert_eq!(
            st, 0,
            "capture_stdout({tag}): child terminated abnormally (raw status {st:#x})"
        );
    }
    drop(file);
    let data = std::fs::read(&path).unwrap_or_default();
    let _ = std::fs::remove_file(&path);
    data
}

// ---------------------------------------------------------------------------
// Assertion helper
// ---------------------------------------------------------------------------

pub fn same(what: &str, c: &[u8], r: &[u8]) {
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
            "DIVERGENCE in {what}\n  C  len={} \n  R  len={}\n  first differing byte index {}\n  \
             C[..]={:02x?}\n  R[..]={:02x?}",
            c.len(),
            r.len(),
            first,
            &c[first.saturating_sub(8)..(first + 24).min(c.len())],
            &r[first.saturating_sub(8)..(first + 24).min(r.len())],
        );
    }
}
