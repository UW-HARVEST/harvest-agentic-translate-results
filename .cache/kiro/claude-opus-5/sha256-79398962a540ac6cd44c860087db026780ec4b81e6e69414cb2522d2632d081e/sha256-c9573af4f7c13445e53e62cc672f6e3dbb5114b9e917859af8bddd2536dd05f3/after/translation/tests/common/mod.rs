//! Shared differential-test harness.
//!
//! BOTH implementations are loaded as shared objects through `libloading` and
//! driven only through their exported C symbols — the Rust crate's functions are
//! never called directly, so the `#[no_mangle] extern "C"` wrappers are part of
//! what is under test.
#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::ffi::{c_char, c_int, c_void};
use std::path::PathBuf;
use std::sync::{Mutex, MutexGuard, OnceLock};

// ---------------------------------------------------------------------------
// Layout mirrors of the internal C structs (all `#[repr(C)]`, byte-identical)
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
#[derive(Copy, Clone)]
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

pub const HM_BINARY: c_int = 0;
pub const HM_STRING: c_int = 1;

pub const SH_NONE: c_int = 0;
pub const SH_DEFAULT: c_int = 1;
pub const SH_STRDUP: c_int = 2;
pub const SH_ARENA: c_int = 3;

// ---------------------------------------------------------------------------
// The loaded API
// ---------------------------------------------------------------------------

pub struct Api {
    pub name: &'static str,
    _lib: Library,
    pub arrgrowf: unsafe extern "C" fn(*mut c_void, usize, usize, usize) -> *mut c_void,
    pub arrfreef: unsafe extern "C" fn(*mut c_void),
    pub rand_seed: unsafe extern "C" fn(usize),
    pub hash_bytes: unsafe extern "C" fn(*mut c_void, usize, usize) -> usize,
    pub hash_string: unsafe extern "C" fn(*mut c_char, usize) -> usize,
    pub hmfree_func: unsafe extern "C" fn(*mut c_void, usize),
    pub hmget_key: unsafe extern "C" fn(*mut c_void, usize, *mut c_void, usize, c_int) -> *mut c_void,
    pub hmget_key_ts:
        unsafe extern "C" fn(*mut c_void, usize, *mut c_void, usize, *mut isize, c_int) -> *mut c_void,
    pub hmput_default: unsafe extern "C" fn(*mut c_void, usize) -> *mut c_void,
    pub hmput_key: unsafe extern "C" fn(*mut c_void, usize, *mut c_void, usize, c_int) -> *mut c_void,
    pub hmdel_key:
        unsafe extern "C" fn(*mut c_void, usize, *mut c_void, usize, usize, c_int) -> *mut c_void,
    pub shmode_func: unsafe extern "C" fn(usize, c_int) -> *mut c_void,
    pub stralloc: unsafe extern "C" fn(*mut StringArena, *mut c_char) -> *mut c_char,
    pub strreset: unsafe extern "C" fn(*mut StringArena),
    pub sh_puts: unsafe extern "C" fn(c_int),
    pub strkey: unsafe extern "C" fn(c_int) -> *mut c_char,
}

unsafe fn sym<T: Copy>(lib: &Library, n: &str) -> T {
    let s: Symbol<T> = lib
        .get(n.as_bytes())
        .unwrap_or_else(|e| panic!("symbol `{n}` not found: {e}"));
    *s
}

impl Api {
    unsafe fn load(name: &'static str, path: &PathBuf) -> Api {
        let lib = Library::new(path).unwrap_or_else(|e| panic!("dlopen {path:?}: {e}"));
        Api {
            name,
            arrgrowf: sym(&lib, "stbds_arrgrowf"),
            arrfreef: sym(&lib, "stbds_arrfreef"),
            rand_seed: sym(&lib, "stbds_rand_seed"),
            hash_bytes: sym(&lib, "stbds_hash_bytes"),
            hash_string: sym(&lib, "stbds_hash_string"),
            hmfree_func: sym(&lib, "stbds_hmfree_func"),
            hmget_key: sym(&lib, "stbds_hmget_key"),
            hmget_key_ts: sym(&lib, "stbds_hmget_key_ts"),
            hmput_default: sym(&lib, "stbds_hmput_default"),
            hmput_key: sym(&lib, "stbds_hmput_key"),
            hmdel_key: sym(&lib, "stbds_hmdel_key"),
            shmode_func: sym(&lib, "stbds_shmode_func"),
            stralloc: sym(&lib, "stbds_stralloc"),
            strreset: sym(&lib, "stbds_strreset"),
            sh_puts: sym(&lib, "sh_puts"),
            strkey: sym(&lib, "strkey"),
            _lib: lib,
        }
    }
}

fn find_c_so() -> PathBuf {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("c_src")
        .join("build");
    let mut found: Vec<PathBuf> = Vec::new();
    for e in std::fs::read_dir(&root)
        .unwrap_or_else(|e| panic!("{root:?} not built ({e}); run cmake first"))
    {
        let p = e.unwrap().path();
        if p.extension().map(|x| x == "so").unwrap_or(false) {
            found.push(p);
        }
    }
    assert_eq!(found.len(), 1, "expected exactly one C .so, got {found:?}");
    found.pop().unwrap()
}

fn find_rust_so() -> PathBuf {
    // Allow the harness to be pointed at a specific build (used to verify the
    // debug profile as well as the release one).
    if let Ok(p) = std::env::var("SHPUTS_RUST_SO") {
        let p = PathBuf::from(p);
        assert!(p.exists(), "SHPUTS_RUST_SO={p:?} does not exist");
        return p;
    }
    let base = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target");
    for profile in ["release", "debug"] {
        let p = base.join(profile).join("libsh_puts_lib.so");
        if p.exists() {
            return p;
        }
    }
    panic!("libsh_puts_lib.so not found; run `cargo build --release` first");
}

pub struct Both {
    pub c: Api,
    pub r: Api,
}

static BOTH: OnceLock<Both> = OnceLock::new();
static LOCK: Mutex<()> = Mutex::new(());

/// Acquire exclusive access to both libraries.
///
/// Both `.so`s carry a mutable `stbds_hash_seed` global, so scenarios must not
/// interleave. The guard also re-seeds both sides so every scenario starts from
/// an identical, reproducible state.
pub fn session(seed: usize) -> Session {
    let g = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let both = BOTH.get_or_init(|| unsafe {
        Both {
            c: Api::load("C", &find_c_so()),
            r: Api::load("Rust", &find_rust_so()),
        }
    });
    unsafe {
        (both.c.rand_seed)(seed);
        (both.r.rand_seed)(seed);
    }
    Session { both, _g: g }
}

pub struct Session {
    both: &'static Both,
    _g: MutexGuard<'static, ()>,
}

impl Session {
    pub fn c(&self) -> &Api {
        &self.both.c
    }
    pub fn r(&self) -> &Api {
        &self.both.r
    }
    /// `[c, r]` — for loops that must do the identical thing to both sides.
    pub fn each(&self) -> [&Api; 2] {
        [&self.both.c, &self.both.r]
    }
    pub fn reseed(&self, seed: usize) {
        unsafe {
            (self.both.c.rand_seed)(seed);
            (self.both.r.rand_seed)(seed);
        }
    }
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (xorshift64*), fixed seed per test
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
        x.wrapping_mul(0x2545F4914F6CDD1D)
    }
    pub fn below(&mut self, n: usize) -> usize {
        if n == 0 {
            0
        } else {
            (self.next_u64() % n as u64) as usize
        }
    }
    pub fn range(&mut self, lo: usize, hi_inclusive: usize) -> usize {
        lo + self.below(hi_inclusive - lo + 1)
    }
    pub fn byte(&mut self) -> u8 {
        self.next_u64() as u8
    }
    pub fn bytes(&mut self, n: usize) -> Vec<u8> {
        (0..n).map(|_| self.byte()).collect()
    }
    /// NUL-terminated C string of `len` non-zero bytes (full 1..=255 range).
    pub fn cstring(&mut self, len: usize) -> Vec<u8> {
        let mut v: Vec<u8> = (0..len).map(|_| 1 + (self.next_u64() % 255) as u8).collect();
        v.push(0);
        v
    }
}

// ---------------------------------------------------------------------------
// Map handle: `t` is the "hash pointer" (raw array + one element)
// ---------------------------------------------------------------------------

#[derive(Copy, Clone)]
pub struct Map {
    pub t: *mut c_void,
    pub elemsize: usize,
}

impl Map {
    pub fn null(elemsize: usize) -> Map {
        Map {
            t: std::ptr::null_mut(),
            elemsize,
        }
    }
    pub fn raw(&self) -> *mut c_void {
        (self.t as *mut u8).wrapping_sub(self.elemsize) as *mut c_void
    }
    pub unsafe fn header(&self) -> *mut ArrayHeader {
        (self.raw() as *mut ArrayHeader).wrapping_sub(1)
    }
    /// `stbds_temp(t-1)` — the index the last put/get/del reported.
    pub unsafe fn temp(&self) -> isize {
        (*self.header()).temp
    }
    /// `stbds_hmlen(t)`
    pub unsafe fn len(&self) -> isize {
        if self.t.is_null() {
            0
        } else {
            (*self.header()).length as isize - 1
        }
    }
    pub unsafe fn table(&self) -> *mut HashIndex {
        if self.t.is_null() {
            std::ptr::null_mut()
        } else {
            (*self.header()).hash_table as *mut HashIndex
        }
    }
    /// raw element `i` (i == 0 is the `t[-1]` default slot)
    pub fn elem(&self, i: usize) -> *mut u8 {
        (self.raw() as *mut u8).wrapping_add(self.elemsize * i)
    }
}

// ---------------------------------------------------------------------------
// Comparable snapshot of everything the two libraries must agree on
// ---------------------------------------------------------------------------

/// How the key field of an element is stored, which decides whether the first
/// 8 bytes of each element are a (necessarily different) pointer or raw data.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum KeyRepr {
    /// key bytes live inline in the element (binary / `default:` switch arm)
    Inline(usize),
    /// key is a `char *` at offset 0 whose *target* string is compared
    Pointer,
}

#[derive(PartialEq, Eq, Debug)]
pub struct Snapshot {
    pub is_null: bool,
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
    pub str_remaining: usize,
    pub str_block: u8,
    pub str_mode: u8,
    pub bucket_hash: Vec<[usize; 8]>,
    pub bucket_index: Vec<[isize; 8]>,
    /// per raw element: the comparable bytes
    pub elem_bytes: Vec<Vec<u8>>,
    /// per raw element: the key string when `KeyRepr::Pointer`
    pub key_strings: Vec<Option<Vec<u8>>>,
}

/// Snapshot a map. `value_range` is the byte range of each element that the
/// caller has explicitly initialised (so uninitialised realloc padding is never
/// compared).
pub unsafe fn snapshot(m: &Map, key: KeyRepr, value_range: std::ops::Range<usize>) -> Snapshot {
    if m.t.is_null() {
        return Snapshot {
            is_null: true,
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
            str_remaining: 0,
            str_block: 0,
            str_mode: 0,
            bucket_hash: vec![],
            bucket_index: vec![],
            elem_bytes: vec![],
            key_strings: vec![],
        };
    }
    let h = &*m.header();
    let tbl = m.table();

    let mut s = Snapshot {
        is_null: false,
        length: h.length,
        capacity: h.capacity,
        temp: h.temp,
        has_table: !tbl.is_null(),
        slot_count: 0,
        used_count: 0,
        used_count_threshold: 0,
        used_count_shrink_threshold: 0,
        tombstone_count: 0,
        tombstone_count_threshold: 0,
        seed: 0,
        slot_count_log2: 0,
        str_remaining: 0,
        str_block: 0,
        str_mode: 0,
        bucket_hash: vec![],
        bucket_index: vec![],
        elem_bytes: vec![],
        key_strings: vec![],
    };

    if !tbl.is_null() {
        let t = &*tbl;
        s.slot_count = t.slot_count;
        s.used_count = t.used_count;
        s.used_count_threshold = t.used_count_threshold;
        s.used_count_shrink_threshold = t.used_count_shrink_threshold;
        s.tombstone_count = t.tombstone_count;
        s.tombstone_count_threshold = t.tombstone_count_threshold;
        s.seed = t.seed;
        s.slot_count_log2 = t.slot_count_log2;
        s.str_remaining = t.string.remaining;
        s.str_block = t.string.block;
        s.str_mode = t.string.mode;
        for i in 0..(t.slot_count >> 3) {
            let b = &*t.storage.wrapping_add(i);
            s.bucket_hash.push(b.hash);
            s.bucket_index.push(b.index);
        }
    }

    for i in 0..h.length {
        let e = m.elem(i);
        match key {
            KeyRepr::Inline(ks) => {
                let mut v: Vec<u8> = std::slice::from_raw_parts(e, ks.min(m.elemsize)).to_vec();
                for off in value_range.clone() {
                    if off < m.elemsize {
                        v.push(*e.add(off));
                    }
                }
                s.elem_bytes.push(v);
                s.key_strings.push(None);
            }
            KeyRepr::Pointer => {
                let mut v: Vec<u8> = Vec::new();
                for off in value_range.clone() {
                    if off < m.elemsize {
                        v.push(*e.add(off));
                    }
                }
                s.elem_bytes.push(v);
                let p = *(e as *mut *mut c_char);
                if p.is_null() {
                    s.key_strings.push(None);
                } else {
                    s.key_strings.push(Some(cstr_bytes(p)));
                }
            }
        }
    }
    s
}

pub unsafe fn cstr_bytes(p: *const c_char) -> Vec<u8> {
    let mut v = Vec::new();
    let mut q = p as *const u8;
    while *q != 0 {
        v.push(*q);
        q = q.add(1);
    }
    v
}

/// Assert two snapshots agree, with a readable field-level diff.
pub fn assert_snap_eq(c: &Snapshot, r: &Snapshot, ctx: &str) {
    macro_rules! f {
        ($($n:ident),*) => {$(
            assert_eq!(c.$n, r.$n, "{}: field `{}` differs (C={:?} Rust={:?})",
                       ctx, stringify!($n), c.$n, r.$n);
        )*};
    }
    f!(
        is_null,
        length,
        capacity,
        temp,
        has_table,
        slot_count,
        used_count,
        used_count_threshold,
        used_count_shrink_threshold,
        tombstone_count,
        tombstone_count_threshold,
        seed,
        slot_count_log2,
        str_remaining,
        str_block,
        str_mode
    );
    assert_eq!(
        c.bucket_hash.len(),
        r.bucket_hash.len(),
        "{ctx}: bucket count differs"
    );
    for i in 0..c.bucket_hash.len() {
        assert_eq!(
            c.bucket_hash[i], r.bucket_hash[i],
            "{ctx}: bucket[{i}].hash differs"
        );
        assert_eq!(
            c.bucket_index[i], r.bucket_index[i],
            "{ctx}: bucket[{i}].index differs"
        );
    }
    assert_eq!(
        c.elem_bytes.len(),
        r.elem_bytes.len(),
        "{ctx}: element count differs"
    );
    for i in 0..c.elem_bytes.len() {
        assert_eq!(
            c.elem_bytes[i], r.elem_bytes[i],
            "{ctx}: element[{i}] bytes differ"
        );
        assert_eq!(
            c.key_strings[i], r.key_strings[i],
            "{ctx}: element[{i}] key string differs"
        );
    }
}

// ---------------------------------------------------------------------------
// Lockstep driver: runs the same op on both maps and diffs the result
// ---------------------------------------------------------------------------

/// A pair of maps (one per library) driven in lockstep.
pub struct Pair<'s> {
    pub s: &'s Session,
    pub cm: Map,
    pub rm: Map,
    pub keyrepr: KeyRepr,
    /// the element byte range this harness writes after each put
    pub value_range: std::ops::Range<usize>,
    pub keysize: usize,
    pub ops: usize,
}

impl<'s> Pair<'s> {
    pub fn new(
        s: &'s Session,
        elemsize: usize,
        keysize: usize,
        keyrepr: KeyRepr,
        value_range: std::ops::Range<usize>,
    ) -> Pair<'s> {
        Pair {
            s,
            cm: Map::null(elemsize),
            rm: Map::null(elemsize),
            keyrepr,
            value_range,
            keysize,
            ops: 0,
        }
    }

    /// `sh_new_arena` / `sh_new_strdup` / any `stbds_shmode_func` mode.
    pub unsafe fn shmode(&mut self, mode: c_int) {
        let es = self.cm.elemsize;
        self.cm.t = (self.s.c().shmode_func)(es, mode);
        self.rm.t = (self.s.r().shmode_func)(es, mode);
        self.check("shmode_func");
    }

    pub unsafe fn hmput_default(&mut self) {
        let es = self.cm.elemsize;
        self.cm.t = (self.s.c().hmput_default)(self.cm.t, es);
        self.rm.t = (self.s.r().hmput_default)(self.rm.t, es);
        self.check("hmput_default");
    }

    /// `stbds_hmput_key` + the value write the `hmput`/`shput` macro performs.
    /// `key` must be a buffer the caller keeps alive (the C library may store
    /// the pointer itself in `SH_DEFAULT` mode).
    pub unsafe fn put(&mut self, key: *mut u8, mode: c_int, value: &[u8]) -> (isize, isize) {
        let es = self.cm.elemsize;
        let ks = self.keysize;
        self.cm.t = (self.s.c().hmput_key)(self.cm.t, es, key as *mut c_void, ks, mode);
        self.rm.t = (self.s.r().hmput_key)(self.rm.t, es, key as *mut c_void, ks, mode);
        let ct = self.cm.temp();
        let rt = self.rm.temp();
        assert_eq!(ct, rt, "put: reported index differs (C={ct} Rust={rt})");
        // Emulate `(t)[stbds_temp((t)-1)].value = v` and fully initialise the
        // element tail so no uninitialised realloc padding is ever compared.
        for (m, _) in [(&self.cm, 0), (&self.rm, 1)] {
            let e = m.elem((ct + 1) as usize);
            for (j, off) in self.value_range.clone().enumerate() {
                if off < es {
                    *e.add(off) = value[j % value.len().max(1)];
                }
            }
        }
        self.ops += 1;
        self.check("hmput_key");
        (ct, rt)
    }

    pub unsafe fn get(&mut self, key: *mut u8, mode: c_int) -> isize {
        let es = self.cm.elemsize;
        let ks = self.keysize;
        self.cm.t = (self.s.c().hmget_key)(self.cm.t, es, key as *mut c_void, ks, mode);
        self.rm.t = (self.s.r().hmget_key)(self.rm.t, es, key as *mut c_void, ks, mode);
        let ct = self.cm.temp();
        let rt = self.rm.temp();
        assert_eq!(ct, rt, "hmget_key: index differs (C={ct} Rust={rt})");
        self.check("hmget_key");
        ct
    }

    pub unsafe fn get_ts(&mut self, key: *mut u8, mode: c_int) -> isize {
        let es = self.cm.elemsize;
        let ks = self.keysize;
        let mut ctmp: isize = 0x5A5A_5A5A;
        let mut rtmp: isize = 0x5A5A_5A5A;
        self.cm.t = (self.s.c().hmget_key_ts)(self.cm.t, es, key as *mut c_void, ks, &mut ctmp, mode);
        self.rm.t = (self.s.r().hmget_key_ts)(self.rm.t, es, key as *mut c_void, ks, &mut rtmp, mode);
        assert_eq!(ctmp, rtmp, "hmget_key_ts: *temp differs");
        self.check("hmget_key_ts");
        ctmp
    }

    pub unsafe fn del(&mut self, key: *mut u8, mode: c_int, keyoffset: usize) -> isize {
        let es = self.cm.elemsize;
        let ks = self.keysize;
        let cprev = self.cm.t;
        let rprev = self.rm.t;
        self.cm.t = (self.s.c().hmdel_key)(self.cm.t, es, key as *mut c_void, ks, keyoffset, mode);
        self.rm.t = (self.s.r().hmdel_key)(self.rm.t, es, key as *mut c_void, ks, keyoffset, mode);
        assert_eq!(
            self.cm.t.is_null(),
            self.rm.t.is_null(),
            "hmdel_key: NULL-ness of the returned pointer differs"
        );
        assert_eq!(
            self.cm.t == cprev,
            self.rm.t == rprev,
            "hmdel_key: identity of the returned pointer differs"
        );
        if self.cm.t.is_null() {
            return 0;
        }
        let ct = self.cm.temp();
        let rt = self.rm.temp();
        assert_eq!(ct, rt, "hmdel_key: reported flag differs (C={ct} Rust={rt})");
        self.check("hmdel_key");
        ct
    }

    pub unsafe fn free(&mut self) {
        let es = self.cm.elemsize;
        if !self.cm.t.is_null() {
            (self.s.c().hmfree_func)(self.cm.raw(), es);
            (self.s.r().hmfree_func)(self.rm.raw(), es);
        }
        self.cm.t = std::ptr::null_mut();
        self.rm.t = std::ptr::null_mut();
    }

    pub unsafe fn snap(&self) -> (Snapshot, Snapshot) {
        (
            snapshot(&self.cm, self.keyrepr, self.value_range.clone()),
            snapshot(&self.rm, self.keyrepr, self.value_range.clone()),
        )
    }

    pub unsafe fn check(&self, what: &str) {
        let (c, r) = self.snap();
        assert_snap_eq(&c, &r, &format!("{what} (after {} ops)", self.ops));
    }
}

// ---------------------------------------------------------------------------
// stdout capture (for `sh_puts`, which printf()s)
//
// Capturing fd 1 in-process is unreliable: libtest writes its own progress lines
// to fd 1 from other threads, and they land in the capture.  So `sh_puts` is
// always run in a child process whose fd 1 starts at /dev/null and is redirected
// onto a file only for the duration of the call.
// ---------------------------------------------------------------------------

extern "C" {
    fn fflush(f: *mut c_void) -> c_int;
    fn dup(fd: c_int) -> c_int;
    fn dup2(old: c_int, new: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
}

/// Redirect libc `stdout` to `path` for the duration of `f`.
unsafe fn capture_stdout_to<F: FnOnce()>(path: &str, f: F) {
    use std::io::Write;
    use std::os::unix::io::AsRawFd;

    let _ = std::io::stdout().flush();
    let file = std::fs::File::create(path).unwrap();
    fflush(std::ptr::null_mut());
    let saved = dup(1);
    assert!(saved >= 0, "dup(1) failed");
    assert!(dup2(file.as_raw_fd(), 1) >= 0, "dup2 failed");

    f();

    fflush(std::ptr::null_mut());
    dup2(saved, 1);
    close(saved);
    drop(file);
}

/// Child-process body. Returns immediately unless `SHPUTS_NUM` is set, so it is
/// inert during a normal `cargo test` run.
pub fn sh_puts_child_main() {
    let num: c_int = match std::env::var("SHPUTS_NUM") {
        Ok(v) => v.parse().expect("SHPUTS_NUM"),
        Err(_) => return,
    };
    let which = std::env::var("SHPUTS_LIB").expect("SHPUTS_LIB");
    let out = std::env::var("SHPUTS_OUT").expect("SHPUTS_OUT");
    let s = session(0x31415926);
    let api = if which == "c" { s.c() } else { s.r() };
    unsafe {
        capture_stdout_to(&out, || (api.sh_puts)(num));
    }
    std::process::exit(0);
}

/// Run `sh_puts(num)` against `which` ("c" | "rust") in a child process and
/// return exactly the bytes the library printed.
pub fn sh_puts_stdout(num: c_int, which: &str, runner: &str) -> Vec<u8> {
    use std::process::{Command, Stdio};
    let tmp = std::env::temp_dir().join(format!(
        "shputs-out-{}-{}-{}-{}.txt",
        std::process::id(),
        which,
        num,
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let exe = std::env::current_exe().expect("current_exe");
    let st = Command::new(exe)
        .args(["--exact", runner, "--ignored", "--nocapture"])
        .env("SHPUTS_NUM", num.to_string())
        .env("SHPUTS_LIB", which)
        .env("SHPUTS_OUT", tmp.to_str().unwrap())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .output()
        .expect("spawn sh_puts child");
    assert!(
        st.status.success(),
        "sh_puts({num}) child for {which} exited with {:?}\nstderr: {}",
        st.status,
        String::from_utf8_lossy(&st.stderr)
    );
    let bytes = std::fs::read(&tmp).unwrap_or_default();
    let _ = std::fs::remove_file(&tmp);
    bytes
}
