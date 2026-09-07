//! Shared differential-test harness.
//!
//! Both the C `.so` and the Rust `.so` are loaded with `libloading` and driven
//! exclusively through their exported symbols, so the `#[no_mangle]` wrappers
//! are part of what is under test.

#![allow(dead_code)]
#![allow(nonstandard_style)]

use libloading::{Library, Symbol};
use std::ffi::{c_char, c_int, c_void};
use std::path::PathBuf;

// ---------------------------------------------------------------------------
// libc bits we need for stdout capture
// ---------------------------------------------------------------------------
extern "C" {
    fn dup(oldfd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
    fn free(p: *mut c_void);
}

// ---------------------------------------------------------------------------
// Mirror of the C types (for state inspection only)
// ---------------------------------------------------------------------------

pub const STBDS_BUCKET_LENGTH: usize = 8;
pub const STBDS_BUCKET_SHIFT: usize = 3;

pub const STBDS_HM_BINARY: c_int = 0;
pub const STBDS_HM_STRING: c_int = 1;
pub const STBDS_HM_PTR_TO_STRING: c_int = 2;

pub const STBDS_SH_NONE: c_int = 0;
pub const STBDS_SH_DEFAULT: c_int = 1;
pub const STBDS_SH_STRDUP: c_int = 2;
pub const STBDS_SH_ARENA: c_int = 3;

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ArrayHeader {
    pub length: usize,
    pub capacity: usize,
    pub hash_table: *mut c_void,
    pub temp: isize,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
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

// ---------------------------------------------------------------------------
// Function-pointer table
// ---------------------------------------------------------------------------

type FnArrGrowf = unsafe extern "C" fn(*mut c_void, usize, usize, usize) -> *mut c_void;
type FnArrFreef = unsafe extern "C" fn(*mut c_void);
type FnRandSeed = unsafe extern "C" fn(usize);
type FnHashString = unsafe extern "C" fn(*mut c_char, usize) -> usize;
type FnHashBytes = unsafe extern "C" fn(*mut c_void, usize, usize) -> usize;
type FnHmFree = unsafe extern "C" fn(*mut c_void, usize);
type FnHmGetKey = unsafe extern "C" fn(*mut c_void, usize, *mut c_void, usize, c_int) -> *mut c_void;
type FnHmGetKeyTs =
    unsafe extern "C" fn(*mut c_void, usize, *mut c_void, usize, *mut isize, c_int) -> *mut c_void;
type FnHmPutDefault = unsafe extern "C" fn(*mut c_void, usize) -> *mut c_void;
type FnHmPutKey = unsafe extern "C" fn(*mut c_void, usize, *mut c_void, usize, c_int) -> *mut c_void;
type FnHmDelKey =
    unsafe extern "C" fn(*mut c_void, usize, *mut c_void, usize, usize, c_int) -> *mut c_void;
type FnShModeFunc = unsafe extern "C" fn(usize, c_int) -> *mut c_void;
type FnStrAlloc = unsafe extern "C" fn(*mut StringArena, *mut c_char) -> *mut c_char;
type FnStrReset = unsafe extern "C" fn(*mut StringArena);
type FnStrKey = unsafe extern "C" fn(c_int) -> *mut c_char;
type FnStrDups = unsafe extern "C" fn(c_int);

pub struct Lib {
    pub name: &'static str,
    _lib: Library,
    pub arrgrowf: FnArrGrowf,
    pub arrfreef: FnArrFreef,
    pub rand_seed: FnRandSeed,
    pub hash_string: FnHashString,
    pub hash_bytes: FnHashBytes,
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
    pub str_dups: FnStrDups,
}

macro_rules! sym {
    ($lib:expr, $t:ty, $n:literal) => {{
        let s: Symbol<$t> = unsafe { $lib.get(concat!($n, "\0").as_bytes()) }
            .unwrap_or_else(|e| panic!("missing symbol {}: {}", $n, e));
        unsafe { *s.into_raw() }
    }};
}

impl Lib {
    pub fn open(name: &'static str, path: &PathBuf) -> Lib {
        let lib = unsafe { Library::new(path) }
            .unwrap_or_else(|e| panic!("cannot dlopen {}: {}", path.display(), e));
        Lib {
            name,
            arrgrowf: sym!(lib, FnArrGrowf, "stbds_arrgrowf"),
            arrfreef: sym!(lib, FnArrFreef, "stbds_arrfreef"),
            rand_seed: sym!(lib, FnRandSeed, "stbds_rand_seed"),
            hash_string: sym!(lib, FnHashString, "stbds_hash_string"),
            hash_bytes: sym!(lib, FnHashBytes, "stbds_hash_bytes"),
            hmfree_func: sym!(lib, FnHmFree, "stbds_hmfree_func"),
            hmget_key: sym!(lib, FnHmGetKey, "stbds_hmget_key"),
            hmget_key_ts: sym!(lib, FnHmGetKeyTs, "stbds_hmget_key_ts"),
            hmput_default: sym!(lib, FnHmPutDefault, "stbds_hmput_default"),
            hmput_key: sym!(lib, FnHmPutKey, "stbds_hmput_key"),
            hmdel_key: sym!(lib, FnHmDelKey, "stbds_hmdel_key"),
            shmode_func: sym!(lib, FnShModeFunc, "stbds_shmode_func"),
            stralloc: sym!(lib, FnStrAlloc, "stbds_stralloc"),
            strreset: sym!(lib, FnStrReset, "stbds_strreset"),
            strkey: sym!(lib, FnStrKey, "strkey"),
            str_dups: sym!(lib, FnStrDups, "str_dups"),
            _lib: lib,
        }
    }
}

// ---------------------------------------------------------------------------
// Locating the two shared objects
// ---------------------------------------------------------------------------

fn workspace_root() -> PathBuf {
    // translation/ -> parent
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf()
}

fn c_so_path() -> PathBuf {
    let root = workspace_root();
    let build = root.join("c_src/build");
    let mut found = None;
    if let Ok(rd) = std::fs::read_dir(&build) {
        for e in rd.flatten() {
            let p = e.path();
            let name = p.file_name().unwrap().to_string_lossy().to_string();
            if name.starts_with("lib") && name.ends_with(".so") {
                found = Some(p);
            }
        }
    }
    found.unwrap_or_else(|| {
        panic!(
            "C shared library not found in {}; run the cmake build first",
            build.display()
        )
    })
}

fn rust_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("RUST_SO") {
        return PathBuf::from(p);
    }
    // current_exe is <...>/target/<profile>/deps/<test-bin>
    let exe = std::env::current_exe().expect("current_exe");
    let profile_dir = exe.parent().unwrap().parent().unwrap();
    let p = profile_dir.join("libstr_dups_lib.so");
    if p.exists() {
        return p;
    }
    // Fall back to the sibling profile dir (cargo does not rebuild a
    // cdylib-only lib for integration tests, so it may live elsewhere).
    let target = profile_dir.parent().unwrap();
    for prof in ["debug", "release"] {
        let q = target.join(prof).join("libstr_dups_lib.so");
        if q.exists() {
            return q;
        }
    }
    panic!(
        "Rust cdylib not found at {} (build it with `cargo build`)",
        p.display()
    );
}

/// Both libraries plus the process-wide lock that must be held while they are
/// driven: `stbds_hash_seed` is a mutable global inside each `.so`, so two test
/// threads sharing the same `dlopen`ed image would corrupt each other's table
/// layouts.
pub struct Both {
    pub c: Lib,
    pub r: Lib,
    _guard: std::sync::MutexGuard<'static, ()>,
}

static LIB_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// Open both libraries. The C one first, so its `dlopen` ordering never
/// perturbs the Rust one.
pub fn both() -> Both {
    let guard = LIB_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let c = Lib::open("C", &c_so_path());
    let r = Lib::open("RUST", &rust_so_path());
    Both {
        c,
        r,
        _guard: guard,
    }
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (SplitMix64) so every property test is reproducible
// ---------------------------------------------------------------------------

pub struct Rng(pub u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
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
    /// Random NUL-free bytes so they can be used as C strings.
    pub fn cstr_bytes(&mut self, n: usize) -> Vec<u8> {
        let mut v: Vec<u8> = (0..n)
            .map(|_| {
                let b = (self.next_u64() >> 24) as u8;
                if b == 0 {
                    1
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
// State snapshots
// ---------------------------------------------------------------------------

pub unsafe fn header_of(arr: *mut c_void) -> ArrayHeader {
    *(arr as *mut ArrayHeader).sub(1)
}

/// How the key stored in each element should be interpreted when comparing
/// element bytes across the two libraries.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum KeyRepr {
    /// Key bytes are inline data; compare the raw element bytes.
    Inline,
    /// The first 8 bytes of each element are a `char *`; compare the pointee
    /// C string instead of the (necessarily different) pointer value.
    Pointer,
}

/// Complete, comparable image of a hash-map array as produced by the library.
#[derive(Debug, PartialEq, Eq)]
pub struct MapSnapshot {
    pub length: usize,
    pub capacity_is_ge_length: bool,
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
    pub string_mode: u8,
    pub string_block: u8,
    pub string_remaining: usize,
    pub buckets: Vec<(Vec<usize>, Vec<isize>)>,
    pub elements: Vec<Vec<u8>>,
}

/// `hm` points at the *hash* pointer (i.e. `arr + elemsize`).
///
/// Only the bytes the C code actually defines are compared: `[0, keysize)` for
/// the key and `[value_offset, value_offset+value_size)` for the value. Padding
/// the library never writes holds indeterminate `realloc` leftovers and would
/// otherwise produce spurious differences.
pub unsafe fn snapshot_map_lay(hm: *mut c_void, lay: Layout, key: KeyRepr) -> MapSnapshot {
    let elemsize = lay.elemsize;
    let raw = (hm as *mut u8).wrapping_sub(elemsize) as *mut c_void;
    let h = header_of(raw);
    let mut snap = MapSnapshot {
        length: h.length,
        capacity_is_ge_length: h.capacity >= h.length,
        temp: h.temp,
        has_table: !h.hash_table.is_null(),
        slot_count: 0,
        used_count: 0,
        used_count_threshold: 0,
        used_count_shrink_threshold: 0,
        tombstone_count: 0,
        tombstone_count_threshold: 0,
        seed: 0,
        slot_count_log2: 0,
        string_mode: 0,
        string_block: 0,
        string_remaining: 0,
        buckets: Vec::new(),
        elements: Vec::new(),
    };
    if !h.hash_table.is_null() {
        let t = &*(h.hash_table as *mut HashIndex);
        snap.slot_count = t.slot_count;
        snap.used_count = t.used_count;
        snap.used_count_threshold = t.used_count_threshold;
        snap.used_count_shrink_threshold = t.used_count_shrink_threshold;
        snap.tombstone_count = t.tombstone_count;
        snap.tombstone_count_threshold = t.tombstone_count_threshold;
        snap.seed = t.seed;
        snap.slot_count_log2 = t.slot_count_log2;
        snap.string_mode = t.string.mode;
        snap.string_block = t.string.block;
        snap.string_remaining = t.string.remaining;
        for i in 0..(t.slot_count >> STBDS_BUCKET_SHIFT) {
            let b = &*t.storage.add(i);
            snap.buckets.push((b.hash.to_vec(), b.index.to_vec()));
        }
    }
    for i in 0..h.length {
        let base = (raw as *mut u8).add(elemsize * i);
        let mut v = Vec::new();
        match key {
            KeyRepr::Inline => {
                v.extend_from_slice(std::slice::from_raw_parts(base, lay.keysize));
            }
            KeyRepr::Pointer => {
                let p = *(base as *mut *mut c_char);
                if p.is_null() {
                    v.push(0xFFu8);
                } else {
                    v.push(0x01u8);
                    let mut q = p as *const u8;
                    while *q != 0 {
                        v.push(*q);
                        q = q.add(1);
                    }
                    v.push(0);
                }
            }
        }
        v.push(0xAA); // separator so key/value regions can't alias
        if lay.value_size > 0 {
            v.extend_from_slice(std::slice::from_raw_parts(
                base.add(lay.value_offset),
                lay.value_size,
            ));
        }
        snap.elements.push(v);
    }
    snap
}

/// `hm` points at the *hash* pointer (i.e. `arr + elemsize`).
pub unsafe fn snapshot_map(hm: *mut c_void, elemsize: usize, key: KeyRepr) -> MapSnapshot {
    let raw = (hm as *mut u8).wrapping_sub(elemsize) as *mut c_void;
    let h = header_of(raw);
    let mut snap = MapSnapshot {
        length: h.length,
        capacity_is_ge_length: h.capacity >= h.length,
        temp: h.temp,
        has_table: !h.hash_table.is_null(),
        slot_count: 0,
        used_count: 0,
        used_count_threshold: 0,
        used_count_shrink_threshold: 0,
        tombstone_count: 0,
        tombstone_count_threshold: 0,
        seed: 0,
        slot_count_log2: 0,
        string_mode: 0,
        string_block: 0,
        string_remaining: 0,
        buckets: Vec::new(),
        elements: Vec::new(),
    };
    if !h.hash_table.is_null() {
        let t = &*(h.hash_table as *mut HashIndex);
        snap.slot_count = t.slot_count;
        snap.used_count = t.used_count;
        snap.used_count_threshold = t.used_count_threshold;
        snap.used_count_shrink_threshold = t.used_count_shrink_threshold;
        snap.tombstone_count = t.tombstone_count;
        snap.tombstone_count_threshold = t.tombstone_count_threshold;
        snap.seed = t.seed;
        snap.slot_count_log2 = t.slot_count_log2;
        snap.string_mode = t.string.mode;
        snap.string_block = t.string.block;
        snap.string_remaining = t.string.remaining;
        for i in 0..(t.slot_count >> STBDS_BUCKET_SHIFT) {
            let b = &*t.storage.add(i);
            snap.buckets.push((b.hash.to_vec(), b.index.to_vec()));
        }
    }
    // element 0 of the raw array is the "default" slot; include it too.
    for i in 0..h.length {
        let base = (raw as *mut u8).add(elemsize * i);
        match key {
            KeyRepr::Inline => {
                snap.elements
                    .push(std::slice::from_raw_parts(base, elemsize).to_vec());
            }
            KeyRepr::Pointer => {
                let mut v = Vec::new();
                let p = *(base as *mut *mut c_char);
                if p.is_null() {
                    v.push(0xFFu8);
                } else {
                    v.push(0x01u8);
                    let mut q = p as *const u8;
                    while *q != 0 {
                        v.push(*q);
                        q = q.add(1);
                    }
                    v.push(0);
                }
                // rest of the element after the pointer is plain data
                if elemsize > 8 {
                    v.extend_from_slice(std::slice::from_raw_parts(base.add(8), elemsize - 8));
                }
                snap.elements.push(v);
            }
        }
    }
    snap
}

pub unsafe fn cstr(p: *const c_char) -> Vec<u8> {
    let mut v = Vec::new();
    let mut q = p as *const u8;
    while *q != 0 {
        v.push(*q);
        q = q.add(1);
    }
    v
}

/// Snapshot of a `stbds_string_arena`: field values plus the chain length.
#[derive(Debug, PartialEq, Eq)]
pub struct ArenaSnapshot {
    pub remaining: usize,
    pub block: u8,
    pub mode: u8,
    pub chain_len: usize,
    pub storage_is_null: bool,
}

pub unsafe fn snapshot_arena(a: *const StringArena) -> ArenaSnapshot {
    let a = &*a;
    let mut n = 0usize;
    let mut p = a.storage as *const *const c_void; // block->next is first field
    while !p.is_null() {
        n += 1;
        if n > 100_000 {
            break;
        }
        p = *p as *const *const c_void;
    }
    ArenaSnapshot {
        remaining: a.remaining,
        block: a.block,
        mode: a.mode,
        chain_len: n,
        storage_is_null: a.storage.is_null(),
    }
}

// ---------------------------------------------------------------------------
// stdout capture (for `str_dups`, which printf()s)
// ---------------------------------------------------------------------------

/// Runs `f`, capturing everything written to fd 1 (including from libc
/// `printf` inside the loaded `.so`s).
pub fn capture_stdout<F: FnOnce()>(f: F) -> Vec<u8> {
    unsafe {
        let dir = std::env::temp_dir();
        let path = dir.join(format!(
            "strdups_cap_{}_{}.txt",
            std::process::id(),
            CAPTURE_SEQ.fetch_add(1, std::sync::atomic::Ordering::SeqCst)
        ));
        {
            // create/truncate
            let _ = std::fs::File::create(&path).expect("create capture file");
        }
        use std::os::unix::io::AsRawFd;
        let file = std::fs::OpenOptions::new()
            .write(true)
            .open(&path)
            .expect("open capture file");
        fflush(std::ptr::null_mut());
        let saved = dup(1);
        assert!(saved >= 0, "dup(1) failed");
        assert!(dup2(file.as_raw_fd(), 1) >= 0, "dup2 failed");

        f();

        fflush(std::ptr::null_mut());
        assert!(dup2(saved, 1) >= 0, "dup2 restore failed");
        close(saved);
        drop(file);
        let out = std::fs::read(&path).expect("read capture file");
        let _ = std::fs::remove_file(&path);
        out
    }
}

static CAPTURE_SEQ: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

// ---------------------------------------------------------------------------
// Assert-abort probing: run a closure in a forked child and report how it died
// ---------------------------------------------------------------------------

extern "C" {
    fn fork() -> c_int;
    fn waitpid(pid: c_int, status: *mut c_int, options: c_int) -> c_int;
    fn _exit(code: c_int) -> !;
}

#[derive(Debug, PartialEq, Eq)]
pub enum ChildOutcome {
    Exited(i32),
    Signalled(i32),
}

/// Fork and run `f` in the child. Returns how the child terminated, so an
/// `assert()` abort (SIGABRT = 6) can be compared between C and Rust.
pub fn probe_abort<F: FnOnce()>(f: F) -> ChildOutcome {
    unsafe {
        let _ = fflush(std::ptr::null_mut());
        let pid = fork();
        assert!(pid >= 0, "fork failed");
        if pid == 0 {
            // Silence the assert message so test output stays readable.
            let devnull = std::fs::OpenOptions::new()
                .write(true)
                .open("/dev/null")
                .unwrap();
            use std::os::unix::io::AsRawFd;
            dup2(devnull.as_raw_fd(), 2);
            dup2(devnull.as_raw_fd(), 1);
            f();
            let _ = fflush(std::ptr::null_mut());
            _exit(0);
        }
        let mut status: c_int = 0;
        let r = waitpid(pid, &mut status, 0);
        assert!(r == pid, "waitpid failed");
        if status & 0x7f == 0x7f {
            ChildOutcome::Exited(-1)
        } else if status & 0x7f != 0 {
            ChildOutcome::Signalled(status & 0x7f)
        } else {
            ChildOutcome::Exited((status >> 8) & 0xff)
        }
    }
}

pub unsafe fn cfree(p: *mut c_void) {
    free(p)
}

// ---------------------------------------------------------------------------
// Macro-level drivers
//
// The C macros (`stbds_hmput`, `stbds_shput`, `stbds_hmgeti`, `stbds_hmdel`, …)
// live only in the header, so a real consumer's behaviour has to be reproduced
// here: call the exported function, then perform the same follow-up writes the
// macro performs.
// ---------------------------------------------------------------------------

/// Element layout used by the tests: `key` at offset 0 (`keysize` bytes),
/// `value` at `value_offset` (`value_size` bytes), total `elemsize`.
#[derive(Clone, Copy, Debug)]
pub struct Layout {
    pub elemsize: usize,
    pub keysize: usize,
    pub value_offset: usize,
    pub value_size: usize,
}

impl Layout {
    pub const fn new(elemsize: usize, keysize: usize, value_offset: usize, value_size: usize) -> Self {
        Layout {
            elemsize,
            keysize,
            value_offset,
            value_size,
        }
    }
}

pub unsafe fn raw_of(hm: *mut c_void, elemsize: usize) -> *mut c_void {
    (hm as *mut u8).wrapping_sub(elemsize) as *mut c_void
}

impl Lib {
    /// `stbds_hmput(t,k,v)` for a binary-mode map.
    pub unsafe fn hmput(
        &self,
        hm: *mut c_void,
        lay: Layout,
        key: &mut [u8],
        value: &[u8],
        mode: c_int,
    ) -> (*mut c_void, isize) {
        let t = (self.hmput_key)(
            hm,
            lay.elemsize,
            key.as_mut_ptr() as *mut c_void,
            lay.keysize,
            mode,
        );
        let temp = header_of(raw_of(t, lay.elemsize)).temp;
        let elem = (t as *mut u8).offset(lay.elemsize as isize * temp);
        // macro: (t)[temp].key = k
        std::ptr::copy_nonoverlapping(key.as_ptr(), elem, lay.keysize);
        // macro: (t)[temp].value = v
        if lay.value_size > 0 {
            std::ptr::copy_nonoverlapping(
                value.as_ptr(),
                elem.add(lay.value_offset),
                lay.value_size,
            );
        }
        (t, temp)
    }

    /// `stbds_shput(t,k,v)` — string mode; the macro writes ONLY the value, the
    /// key pointer having been installed by `stbds_hmput_key` itself.
    pub unsafe fn shput(
        &self,
        hm: *mut c_void,
        lay: Layout,
        key: *mut c_char,
        value: &[u8],
        mode: c_int,
    ) -> (*mut c_void, isize) {
        let t = (self.hmput_key)(hm, lay.elemsize, key as *mut c_void, lay.keysize, mode);
        let temp = header_of(raw_of(t, lay.elemsize)).temp;
        let elem = (t as *mut u8).offset(lay.elemsize as isize * temp);
        if lay.value_size > 0 {
            std::ptr::copy_nonoverlapping(
                value.as_ptr(),
                elem.add(lay.value_offset),
                lay.value_size,
            );
        }
        (t, temp)
    }

    /// `stbds_shputs(t,s)` — writes the whole struct, then re-reads the key
    /// pointer out of `stbds_temp_key`.
    pub unsafe fn shputs(
        &self,
        hm: *mut c_void,
        lay: Layout,
        key: *mut c_char,
        value: &[u8],
        mode: c_int,
    ) -> (*mut c_void, isize) {
        let t = (self.hmput_key)(hm, lay.elemsize, key as *mut c_void, lay.keysize, mode);
        let raw = raw_of(t, lay.elemsize);
        let temp = header_of(raw).temp;
        let elem = (t as *mut u8).offset(lay.elemsize as isize * temp);
        // (t)[temp] = s   (key + value)
        *(elem as *mut *mut c_char) = key;
        if lay.value_size > 0 {
            std::ptr::copy_nonoverlapping(
                value.as_ptr(),
                elem.add(lay.value_offset),
                lay.value_size,
            );
        }
        // (t)[temp].key = stbds_temp_key((t)-1)
        let temp_key = *(header_of(raw).hash_table as *mut *mut c_char);
        *(elem as *mut *mut c_char) = temp_key;
        (t, temp)
    }

    /// `stbds_hmgeti(t,k)` / `stbds_shgeti(t,k)`.
    pub unsafe fn hmgeti(
        &self,
        hm: *mut c_void,
        lay: Layout,
        key: *mut c_void,
        mode: c_int,
    ) -> (*mut c_void, isize) {
        let t = (self.hmget_key)(hm, lay.elemsize, key, lay.keysize, mode);
        let temp = header_of(raw_of(t, lay.elemsize)).temp;
        (t, temp)
    }

    /// `stbds_hmgeti_ts(t,k,temp)`.
    pub unsafe fn hmgeti_ts(
        &self,
        hm: *mut c_void,
        lay: Layout,
        key: *mut c_void,
        mode: c_int,
    ) -> (*mut c_void, isize) {
        let mut temp: isize = 0xDEAD;
        let t = (self.hmget_key_ts)(hm, lay.elemsize, key, lay.keysize, &mut temp, mode);
        (t, temp)
    }

    /// `stbds_hmdel(t,k)` / `stbds_shdel(t,k)`.
    pub unsafe fn hmdel(
        &self,
        hm: *mut c_void,
        lay: Layout,
        key: *mut c_void,
        keyoffset: usize,
        mode: c_int,
    ) -> (*mut c_void, isize) {
        let t = (self.hmdel_key)(hm, lay.elemsize, key, lay.keysize, keyoffset, mode);
        let temp = if t.is_null() {
            0
        } else {
            header_of(raw_of(t, lay.elemsize)).temp
        };
        (t, temp)
    }

    /// `stbds_hmfree(t)`.
    pub unsafe fn hmfree(&self, hm: *mut c_void, elemsize: usize) {
        if !hm.is_null() {
            (self.hmfree_func)(raw_of(hm, elemsize), elemsize);
        }
    }
}
