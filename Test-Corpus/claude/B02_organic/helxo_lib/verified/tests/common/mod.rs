//! Differential-test harness.
//!
//! Loads BOTH shared libraries with `libloading` and calls them only through
//! their exported C symbols:
//!   * `c_src/build/libtranslated_rust.so` (the C ground truth)
//!   * `target/<profile>/libhelxo_lib.so`  (the Rust translation)
//!
//! Nothing in this crate is ever called directly — every call goes through
//! `dlsym`, exactly as an external consumer would, so the `#[no_mangle]`
//! export wrappers are under test as well.
#![allow(dead_code)]

use libloading::Library;
use std::ffi::{c_char, c_int, c_void};
use std::path::PathBuf;

// ---------------------------------------------------------------------------
// C structures (must match c_src/src/lib.c byte for byte)
// ---------------------------------------------------------------------------

#[repr(C)]
#[derive(Clone, Copy, Debug)]
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

pub const HM_BINARY: c_int = 0;
pub const HM_STRING: c_int = 1;

pub const SH_NONE: c_int = 0;
pub const SH_DEFAULT: c_int = 1;
pub const SH_STRDUP: c_int = 2;
pub const SH_ARENA: c_int = 3;

pub const INDEX_EMPTY: isize = -1;
pub const INDEX_DELETED: isize = -2;

// ---------------------------------------------------------------------------
// The loaded library
// ---------------------------------------------------------------------------

type FnArrGrowf = unsafe extern "C" fn(*mut c_void, usize, usize, usize) -> *mut c_void;
type FnArrFreef = unsafe extern "C" fn(*mut c_void);
type FnRandSeed = unsafe extern "C" fn(usize);
type FnHashString = unsafe extern "C" fn(*mut c_char, usize) -> usize;
type FnHashBytes = unsafe extern "C" fn(*mut c_void, usize, usize) -> usize;
type FnHmFree = unsafe extern "C" fn(*mut c_void, usize);
type FnHmGetKeyTs =
    unsafe extern "C" fn(*mut c_void, usize, *mut c_void, usize, *mut isize, c_int) -> *mut c_void;
type FnHmGetKey = unsafe extern "C" fn(*mut c_void, usize, *mut c_void, usize, c_int) -> *mut c_void;
type FnHmPutDefault = unsafe extern "C" fn(*mut c_void, usize) -> *mut c_void;
type FnHmPutKey = unsafe extern "C" fn(*mut c_void, usize, *mut c_void, usize, c_int) -> *mut c_void;
type FnShModeFunc = unsafe extern "C" fn(usize, c_int) -> *mut c_void;
type FnHmDelKey =
    unsafe extern "C" fn(*mut c_void, usize, *mut c_void, usize, usize, c_int) -> *mut c_void;
type FnStrAlloc = unsafe extern "C" fn(*mut StringArena, *mut c_char) -> *mut c_char;
type FnStrReset = unsafe extern "C" fn(*mut StringArena);
type FnStrKey = unsafe extern "C" fn(c_int) -> *mut c_char;
type FnHelxo = unsafe extern "C" fn(c_char);

pub struct Lib {
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
    pub helxo: FnHelxo,
}

unsafe fn sym<T: Copy>(lib: &Library, name: &[u8]) -> T {
    let s: libloading::Symbol<T> = lib
        .get(name)
        .unwrap_or_else(|e| panic!("missing symbol {}: {e}", String::from_utf8_lossy(name)));
    *s
}

impl Lib {
    fn open(name: &'static str, path: &PathBuf) -> Lib {
        unsafe {
            let lib = Library::new(path).unwrap_or_else(|e| panic!("dlopen {path:?}: {e}"));
            Lib {
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
                helxo: sym(&lib, b"helxo\0"),
                _lib: lib,
            }
        }
    }
}

/// The C `.so` and the Rust `.so`, both loaded.
pub struct Pair {
    pub c: Lib,
    pub r: Lib,
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// `target/<profile>/` — derived from the test executable's own location so it
/// works for both `cargo test` and `cargo test --release`.
fn profile_dir() -> PathBuf {
    let exe = std::env::current_exe().expect("current_exe");
    // .../target/<profile>/deps/<test-bin>
    exe.parent()
        .and_then(|p| p.parent())
        .expect("profile dir")
        .to_path_buf()
}

pub fn libs() -> &'static Pair {
    use std::sync::OnceLock;
    static LIBS: OnceLock<Pair> = OnceLock::new();
    LIBS.get_or_init(|| {
        let c_path = manifest_dir().join("c_src/build/libtranslated_rust.so");
        let r_path = profile_dir().join("libhelxo_lib.so");
        assert!(
            c_path.exists(),
            "C shared library not built: {c_path:?}\n\
             build it with: cd c_src && mkdir -p build && cd build && \
             cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build ."
        );
        assert!(r_path.exists(), "Rust cdylib not built: {r_path:?}");
        assert_fresh(&r_path);
        Pair {
            c: Lib::open("C", &c_path),
            r: Lib::open("RUST", &r_path),
        }
    })
}

/// `cargo test --test <name>` does **not** rebuild the `cdylib` target (an
/// integration test has no dependency edge to a `crate-type = ["cdylib"]`
/// library), so the `.so` under test can silently be stale.  Refuse to run in
/// that case instead of verifying an old binary.
fn assert_fresh(so: &PathBuf) {
    let so_time = std::fs::metadata(so)
        .and_then(|m| m.modified())
        .expect("cdylib mtime");
    let mut newest: Option<(std::time::SystemTime, PathBuf)> = None;
    let mut roots = vec![manifest_dir().join("src")];
    let mut files = vec![manifest_dir().join("Cargo.toml")];
    while let Some(dir) = roots.pop() {
        for e in std::fs::read_dir(&dir).expect("read_dir src") {
            let e = e.expect("dir entry");
            let p = e.path();
            if p.is_dir() {
                roots.push(p);
            } else {
                files.push(p);
            }
        }
    }
    for f in files {
        if let Ok(t) = std::fs::metadata(&f).and_then(|m| m.modified()) {
            if newest.as_ref().map(|(nt, _)| t > *nt).unwrap_or(true) {
                newest = Some((t, f));
            }
        }
    }
    if let Some((t, f)) = newest {
        assert!(
            so_time >= t,
            "STALE cdylib: {so:?} is older than {f:?}.\n\
             `cargo test --test <name>` does not rebuild a cdylib target -- run\n\
             `cargo build --offline && cargo test --offline` (or ./scripts/verify.sh)."
        );
    }
}

/// Both libraries keep a *private, mutable* `stbds_hash_seed` global that every
/// `stbds_make_hash_index(.., NULL)` advances.  `cargo test` runs the tests of
/// one binary on parallel threads, so any test that creates a hash index (or
/// calls `stbds_rand_seed`) must hold this lock for its whole body, otherwise
/// the two libraries could pick up *different* seeds and diverge spuriously.
pub fn seed_lock() -> std::sync::MutexGuard<'static, ()> {
    use std::sync::{Mutex, OnceLock};
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    match LOCK.get_or_init(|| Mutex::new(())).lock() {
        Ok(g) => g,
        Err(p) => p.into_inner(),
    }
}

/// Put both libraries' private `stbds_hash_seed` into the same known state.
pub fn seed_both(seed: usize) {
    let l = libs();
    unsafe {
        (l.c.rand_seed)(seed);
        (l.r.rand_seed)(seed);
    }
}

// ---------------------------------------------------------------------------
// Deterministic RNG (SplitMix64) so every property test is reproducible
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
    pub fn below(&mut self, n: usize) -> usize {
        if n == 0 {
            0
        } else {
            (self.next_u64() % n as u64) as usize
        }
    }
    pub fn byte(&mut self) -> u8 {
        (self.next_u64() >> 24) as u8
    }
    pub fn bytes(&mut self, n: usize) -> Vec<u8> {
        (0..n).map(|_| self.byte()).collect()
    }
    /// Printable, NUL-free C string (with the trailing NUL included).
    pub fn cstring(&mut self, len: usize) -> Vec<u8> {
        let mut v: Vec<u8> = (0..len).map(|_| 1 + (self.byte() % 255)).collect();
        v.push(0);
        v
    }
}

// ---------------------------------------------------------------------------
// Map shape + fingerprinting
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum KeyRepr {
    /// Element key field holds `keysize` raw bytes (binary maps, and string
    /// maps whose `string.mode` falls through to the `memcpy` `default:` case).
    Bytes,
    /// Element key field holds a `char *` (SH_DEFAULT / SH_STRDUP / SH_ARENA).
    CStr,
}

#[derive(Clone, Copy, Debug)]
pub struct Shape {
    pub elemsize: usize,
    pub keysize: usize,
    pub key: KeyRepr,
}

impl Shape {
    pub fn bin(elemsize: usize, keysize: usize) -> Shape {
        Shape {
            elemsize,
            keysize,
            key: KeyRepr::Bytes,
        }
    }
    pub fn str_ptr(elemsize: usize) -> Shape {
        Shape {
            elemsize,
            keysize: std::mem::size_of::<*mut c_char>(),
            key: KeyRepr::CStr,
        }
    }
    /// Offset of the "value" region inside an element (what the `stbds_hm*`
    /// macros write after the call returns).
    pub fn value_off(&self) -> usize {
        match self.key {
            KeyRepr::Bytes => self.keysize.min(self.elemsize),
            KeyRepr::CStr => std::mem::size_of::<*mut c_char>().min(self.elemsize),
        }
    }
    pub fn value_len(&self) -> usize {
        self.elemsize - self.value_off()
    }
}

/// Fill `len` bytes at `dst` by repeating `src` — the test values must cover
/// the *whole* value region of an element, otherwise the fingerprint would
/// compare bytes that neither library ever wrote (i.e. whatever `malloc`
/// happened to hand out).
pub unsafe fn fill(dst: *mut u8, len: usize, src: &[u8]) {
    assert!(!src.is_empty(), "value pattern must not be empty");
    for i in 0..len {
        *dst.add(i) = src[i % src.len()];
    }
}

pub unsafe fn header_of(hash_ptr: *mut c_void, elemsize: usize) -> *mut ArrayHeader {
    let raw = (hash_ptr as *mut u8).wrapping_sub(elemsize);
    (raw as *mut ArrayHeader).wrapping_sub(1)
}

pub unsafe fn cstr(p: *const c_char) -> String {
    if p.is_null() {
        return "<null>".to_string();
    }
    let mut v = Vec::new();
    let mut q = p as *const u8;
    while *q != 0 {
        v.push(*q);
        q = q.add(1);
    }
    format!("{:02x?}", v)
}

/// A canonical, address-independent dump of the whole map state.
///
/// Deliberately excluded: the `hash_table` / `storage` / `string.storage` /
/// `temp_key` raw pointer *values* (heap addresses differ between the two
/// libraries), and `temp_key` (never initialised by `stbds_make_hash_index`,
/// so it is uninitialised memory for binary maps).  Everything else — headers,
/// every scalar of the hash index, the complete bucket array and every
/// element's key and value bytes — is compared.
pub unsafe fn fingerprint(hash_ptr: *mut c_void, shape: Shape) -> Vec<String> {
    let mut out = Vec::new();
    if hash_ptr.is_null() {
        out.push("map=NULL".into());
        return out;
    }
    let hdr = header_of(hash_ptr, shape.elemsize);
    let h = *hdr;
    out.push(format!("hdr.length={}", h.length));
    out.push(format!("hdr.capacity={}", h.capacity));
    out.push(format!("hdr.temp={}", h.temp));
    out.push(format!("hdr.has_table={}", !h.hash_table.is_null()));

    if !h.hash_table.is_null() {
        let t = *(h.hash_table as *mut HashIndex);
        out.push(format!("t.slot_count={}", t.slot_count));
        out.push(format!("t.used_count={}", t.used_count));
        out.push(format!("t.used_count_threshold={}", t.used_count_threshold));
        out.push(format!(
            "t.used_count_shrink_threshold={}",
            t.used_count_shrink_threshold
        ));
        out.push(format!("t.tombstone_count={}", t.tombstone_count));
        out.push(format!(
            "t.tombstone_count_threshold={}",
            t.tombstone_count_threshold
        ));
        out.push(format!("t.seed={:#018x}", t.seed));
        out.push(format!("t.slot_count_log2={}", t.slot_count_log2));
        out.push(format!("t.string.remaining={}", t.string.remaining));
        out.push(format!("t.string.block={}", t.string.block));
        out.push(format!("t.string.mode={}", t.string.mode));
        out.push(format!(
            "t.string.has_storage={}",
            !t.string.storage.is_null()
        ));
        // the storage must be 64-byte aligned (STBDS_ALIGN_FWD)
        out.push(format!(
            "t.storage_aligned={}",
            (t.storage as usize) % 64 == 0
        ));
        let nbuckets = t.slot_count >> 3;
        for b in 0..nbuckets {
            let bucket = *t.storage.add(b);
            for s in 0..BUCKET_LENGTH {
                out.push(format!(
                    "slot[{}].hash={:#018x} .index={}",
                    b * BUCKET_LENGTH + s,
                    bucket.hash[s],
                    bucket.index[s]
                ));
            }
        }
    }

    // The raw array holds `hdr.length` elements: raw[0] is the "default"
    // element (`(t)[-1]`, zeroed at creation) and raw[1..] are the entries,
    // i.e. `hash_ptr[idx] == raw[idx+1]`.
    let raw = (hash_ptr as *mut u8).wrapping_sub(shape.elemsize);
    for i in 0..h.length {
        let elem = raw.add(shape.elemsize * i);
        match shape.key {
            KeyRepr::Bytes => {
                let kb = std::slice::from_raw_parts(elem, shape.keysize.min(shape.elemsize));
                out.push(format!("elem[{i}].key={kb:02x?}"));
            }
            KeyRepr::CStr => {
                let p = *(elem as *mut *mut c_char);
                out.push(format!("elem[{i}].key={}", cstr(p)));
            }
        }
        let vo = shape.value_off();
        let vb = std::slice::from_raw_parts(elem.add(vo), shape.value_len());
        out.push(format!("elem[{i}].value={vb:02x?}"));
    }
    out
}

pub fn assert_same(ctx: &str, c: &[String], r: &[String]) {
    if c == r {
        return;
    }
    let mut msg = format!("DIVERGENCE ({ctx})\n");
    let n = c.len().max(r.len());
    for i in 0..n {
        let a = c.get(i).map(|s| s.as_str()).unwrap_or("<missing>");
        let b = r.get(i).map(|s| s.as_str()).unwrap_or("<missing>");
        if a != b {
            msg.push_str(&format!("  [{i}] C   : {a}\n       RUST: {b}\n"));
        }
    }
    panic!("{msg}");
}

// ---------------------------------------------------------------------------
// A map instance living in one of the two libraries
// ---------------------------------------------------------------------------

pub struct Map<'a> {
    pub lib: &'a Lib,
    pub p: *mut c_void,
    pub shape: Shape,
    /// stable storage for keys handed to the library (SH_DEFAULT stores the
    /// pointer, so the buffer must outlive the map, and each library gets its
    /// *own* copy so that a mixed-up pointer shows up as different content)
    keys: Vec<Box<[u8]>>,
}

impl<'a> Map<'a> {
    pub fn empty(lib: &'a Lib, shape: Shape) -> Map<'a> {
        Map {
            lib,
            p: std::ptr::null_mut(),
            shape,
            keys: Vec::new(),
        }
    }

    pub fn shmode(lib: &'a Lib, shape: Shape, mode: c_int) -> Map<'a> {
        let p = unsafe { (lib.shmode_func)(shape.elemsize, mode) };
        Map {
            lib,
            p,
            shape,
            keys: Vec::new(),
        }
    }

    fn stash(&mut self, key: &[u8]) -> *mut c_void {
        self.keys.push(key.to_vec().into_boxed_slice());
        self.keys.last_mut().unwrap().as_mut_ptr() as *mut c_void
    }

    pub unsafe fn temp(&self) -> isize {
        (*header_of(self.p, self.shape.elemsize)).temp
    }

    pub unsafe fn len(&self) -> usize {
        if self.p.is_null() {
            0
        } else {
            (*header_of(self.p, self.shape.elemsize)).length
        }
    }

    /// `stbds_shlen`/`stbds_hmlen`
    pub unsafe fn hmlen(&self) -> isize {
        if self.p.is_null() {
            0
        } else {
            (*header_of(self.p, self.shape.elemsize)).length as isize - 1
        }
    }

    pub unsafe fn table(&self) -> *mut HashIndex {
        if self.p.is_null() {
            return std::ptr::null_mut();
        }
        (*header_of(self.p, self.shape.elemsize)).hash_table as *mut HashIndex
    }

    /// Full `hmput`: call `stbds_hmput_key`, then write the value at the
    /// returned index exactly like the `stbds_hmput`/`stbds_shput` macros do.
    pub unsafe fn put(&mut self, key: &[u8], mode: c_int, value: &[u8]) -> isize {
        let kp = self.stash(key);
        self.p = (self.lib.hmput_key)(self.p, self.shape.elemsize, kp, self.shape.keysize, mode);
        let idx = self.temp();
        if self.shape.value_len() > 0 && idx >= 0 {
            let dst = (self.p as *mut u8)
                .add(self.shape.elemsize * idx as usize)
                .add(self.shape.value_off());
            fill(dst, self.shape.value_len(), value);
        }
        idx
    }

    /// `stbds_hmput_key` only (no value write) — for the shapes where the value
    /// region must stay untouched.
    pub unsafe fn put_raw(&mut self, key: &[u8], mode: c_int) -> isize {
        let kp = self.stash(key);
        self.p = (self.lib.hmput_key)(self.p, self.shape.elemsize, kp, self.shape.keysize, mode);
        self.temp()
    }

    /// `stbds_hmgeti` — returns `stbds_temp`
    pub unsafe fn get(&mut self, key: &[u8], mode: c_int) -> isize {
        let kp = self.stash(key);
        self.p = (self.lib.hmget_key)(self.p, self.shape.elemsize, kp, self.shape.keysize, mode);
        self.temp()
    }

    /// `stbds_hmgeti_ts` — returns the caller-supplied `temp`
    pub unsafe fn get_ts(&mut self, key: &[u8], mode: c_int) -> isize {
        let kp = self.stash(key);
        let mut temp: isize = 0x5555_5555;
        self.p = (self.lib.hmget_key_ts)(
            self.p,
            self.shape.elemsize,
            kp,
            self.shape.keysize,
            &mut temp,
            mode,
        );
        temp
    }

    /// `stbds_hmdel` — returns `stbds_temp` (1 = deleted, 0 = not found)
    pub unsafe fn del(&mut self, key: &[u8], mode: c_int) -> isize {
        let kp = self.stash(key);
        self.p = (self.lib.hmdel_key)(
            self.p,
            self.shape.elemsize,
            kp,
            self.shape.keysize,
            0,
            mode,
        );
        if self.p.is_null() {
            0
        } else {
            self.temp()
        }
    }

    pub unsafe fn hmput_default(&mut self, value: &[u8]) {
        self.p = (self.lib.hmput_default)(self.p, self.shape.elemsize);
        // (t)[-1].value = v
        let dst = (self.p as *mut u8)
            .sub(self.shape.elemsize)
            .add(self.shape.value_off());
        fill(dst, self.shape.value_len(), value);
    }

    pub unsafe fn free(&mut self) {
        if !self.p.is_null() {
            (self.lib.hmfree_func)(
                (self.p as *mut u8).sub(self.shape.elemsize) as *mut c_void,
                self.shape.elemsize,
            );
            self.p = std::ptr::null_mut();
        }
    }

    pub unsafe fn fingerprint(&self) -> Vec<String> {
        fingerprint(self.p, self.shape)
    }

    /// The `stbds_temp_key` slot (`*(char**)hdr->hash_table`) rendered as
    /// content, valid to read only right after a string-mode put.
    pub unsafe fn temp_key_str(&self) -> String {
        let t = self.table();
        if t.is_null() {
            return "<no table>".into();
        }
        cstr((*t).temp_key)
    }
}

/// Two maps — one per library — driven in lockstep.
pub struct MapPair<'a> {
    pub c: Map<'a>,
    pub r: Map<'a>,
    pub ctx: String,
}

impl<'a> MapPair<'a> {
    pub fn empty(shape: Shape) -> MapPair<'static> {
        let l = libs();
        MapPair {
            c: Map::empty(&l.c, shape),
            r: Map::empty(&l.r, shape),
            ctx: String::new(),
        }
    }

    pub fn shmode(shape: Shape, mode: c_int) -> MapPair<'static> {
        let l = libs();
        MapPair {
            c: Map::shmode(&l.c, shape, mode),
            r: Map::shmode(&l.r, shape, mode),
            ctx: format!("shmode({mode})"),
        }
    }

    pub fn check(&self, what: &str) {
        unsafe {
            assert_same(
                &format!("{} / {}", self.ctx, what),
                &self.c.fingerprint(),
                &self.r.fingerprint(),
            );
        }
    }

    pub unsafe fn put(&mut self, key: &[u8], mode: c_int, value: &[u8], what: &str) -> isize {
        let a = self.c.put(key, mode, value);
        let b = self.r.put(key, mode, value);
        assert_eq!(a, b, "{} / {what}: hmput_key index", self.ctx);
        self.check(&format!("after put {what}"));
        a
    }

    pub unsafe fn put_raw(&mut self, key: &[u8], mode: c_int, what: &str) -> isize {
        let a = self.c.put_raw(key, mode);
        let b = self.r.put_raw(key, mode);
        assert_eq!(a, b, "{} / {what}: hmput_key index", self.ctx);
        a
    }

    pub unsafe fn put_check_temp_key(
        &mut self,
        key: &[u8],
        mode: c_int,
        value: &[u8],
        what: &str,
    ) -> isize {
        let idx = self.put(key, mode, value, what);
        assert_eq!(
            self.c.temp_key_str(),
            self.r.temp_key_str(),
            "{} / {what}: temp_key",
            self.ctx
        );
        idx
    }

    pub unsafe fn get(&mut self, key: &[u8], mode: c_int, what: &str) -> isize {
        let a = self.c.get(key, mode);
        let b = self.r.get(key, mode);
        assert_eq!(a, b, "{} / {what}: hmget_key temp", self.ctx);
        self.check(&format!("after get {what}"));
        a
    }

    pub unsafe fn get_ts(&mut self, key: &[u8], mode: c_int, what: &str) -> isize {
        let a = self.c.get_ts(key, mode);
        let b = self.r.get_ts(key, mode);
        assert_eq!(a, b, "{} / {what}: hmget_key_ts temp", self.ctx);
        self.check(&format!("after get_ts {what}"));
        a
    }

    pub unsafe fn del(&mut self, key: &[u8], mode: c_int, what: &str) -> isize {
        let a = self.c.del(key, mode);
        let b = self.r.del(key, mode);
        assert_eq!(a, b, "{} / {what}: hmdel_key temp", self.ctx);
        assert_eq!(
            self.c.p.is_null(),
            self.r.p.is_null(),
            "{} / {what}: hmdel_key NULL-ness",
            self.ctx
        );
        self.check(&format!("after del {what}"));
        a
    }

    pub unsafe fn hmput_default(&mut self, value: &[u8], what: &str) {
        self.c.hmput_default(value);
        self.r.hmput_default(value);
        self.check(&format!("after hmput_default {what}"));
    }

    pub unsafe fn free(&mut self) {
        self.c.free();
        self.r.free();
    }
}

// ---------------------------------------------------------------------------
// stdout capture (for `helxo`)
// ---------------------------------------------------------------------------

extern "C" {
    fn dup(fd: c_int) -> c_int;
    fn dup2(old: c_int, new: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn fflush(f: *mut c_void) -> c_int;
    fn open(path: *const c_char, flags: c_int, ...) -> c_int;
    fn lseek(fd: c_int, off: i64, whence: c_int) -> i64;
    fn read(fd: c_int, buf: *mut c_void, n: usize) -> isize;
}

const O_RDWR: c_int = 2;
const O_CREAT: c_int = 64;
const O_TRUNC: c_int = 512;

/// Run `f` with fd 1 redirected into a temp file and return everything written.
pub fn capture_stdout<F: FnOnce()>(tag: &str, f: F) -> Vec<u8> {
    unsafe {
        let mut path = std::env::temp_dir();
        path.push(format!("helxo_capture_{}_{}.txt", std::process::id(), tag));
        let cpath = std::ffi::CString::new(path.to_str().unwrap()).unwrap();

        // Flush BOTH buffering layers before stealing fd 1: libc's `FILE*`
        // buffers (what the two libraries' `printf` writes into) and Rust's
        // own `std::io::Stdout` LineWriter (what the libtest harness writes
        // into) -- otherwise their pending bytes would land in the capture.
        use std::io::Write;
        let _ = std::io::stdout().flush();
        let _ = std::io::stderr().flush();
        fflush(std::ptr::null_mut());
        let saved = dup(1);
        assert!(saved >= 0, "dup(1) failed");
        let fd = open(cpath.as_ptr(), O_RDWR | O_CREAT | O_TRUNC, 0o600 as c_int);
        assert!(fd >= 0, "open temp file failed");
        assert!(dup2(fd, 1) >= 0, "dup2 failed");

        f();

        let _ = std::io::stdout().flush();
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
        let _ = std::fs::remove_file(&path);
        out
    }
}
