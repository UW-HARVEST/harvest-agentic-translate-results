//! Shared differential-test harness.
//!
//! Loads BOTH the C `.so` and the Rust `.so` through `libloading` and exposes
//! the 16 exported symbols as typed function pointers, plus helpers to take a
//! deterministic "snapshot" of the observable state behind a returned pointer.
//!
//! No Rust function is ever called directly — everything goes through
//! `dlsym` on the respective shared object, exactly like an external consumer.

#![allow(dead_code)]

use libloading::Library;
use std::ffi::{c_char, c_int, c_void};
use std::path::PathBuf;

// ---------------------------------------------------------------------------
// Mirrored C layouts (used only to *read* state the libraries produced)
// ---------------------------------------------------------------------------

pub const HEADER_SIZE: usize = std::mem::size_of::<RawArrayHeader>();
pub const BUCKET_LENGTH: usize = 8;
pub const BUCKET_SHIFT: usize = 3;

#[repr(C)]
#[derive(Copy, Clone)]
pub struct RawArrayHeader {
    pub length: usize,
    pub capacity: usize,
    pub hash_table: *mut c_void,
    pub temp: isize,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct RawStringArena {
    pub storage: *mut c_void,
    pub remaining: usize,
    pub block: u8,
    pub mode: u8,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct RawHashBucket {
    pub hash: [usize; BUCKET_LENGTH],
    pub index: [isize; BUCKET_LENGTH],
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct RawHashIndex {
    pub temp_key: *mut c_char,
    pub slot_count: usize,
    pub used_count: usize,
    pub used_count_threshold: usize,
    pub used_count_shrink_threshold: usize,
    pub tombstone_count: usize,
    pub tombstone_count_threshold: usize,
    pub seed: usize,
    pub slot_count_log2: usize,
    pub string: RawStringArena,
    pub storage: *mut RawHashBucket,
}

// ---------------------------------------------------------------------------
// Function pointer types
// ---------------------------------------------------------------------------

pub type FnArrGrowf = unsafe extern "C" fn(*mut c_void, usize, usize, usize) -> *mut c_void;
pub type FnArrFreef = unsafe extern "C" fn(*mut c_void);
pub type FnRandSeed = unsafe extern "C" fn(usize);
pub type FnHashBytes = unsafe extern "C" fn(*mut c_void, usize, usize) -> usize;
pub type FnHashString = unsafe extern "C" fn(*mut c_char, usize) -> usize;
pub type FnHmfreeFunc = unsafe extern "C" fn(*mut c_void, usize);
pub type FnHmgetKey = unsafe extern "C" fn(*mut c_void, usize, *mut c_void, usize, c_int) -> *mut c_void;
pub type FnHmgetKeyTs =
    unsafe extern "C" fn(*mut c_void, usize, *mut c_void, usize, *mut isize, c_int) -> *mut c_void;
pub type FnHmputDefault = unsafe extern "C" fn(*mut c_void, usize) -> *mut c_void;
pub type FnHmputKey = unsafe extern "C" fn(*mut c_void, usize, *mut c_void, usize, c_int) -> *mut c_void;
pub type FnHmdelKey =
    unsafe extern "C" fn(*mut c_void, usize, *mut c_void, usize, usize, c_int) -> *mut c_void;
pub type FnShmodeFunc = unsafe extern "C" fn(usize, c_int) -> *mut c_void;
pub type FnStralloc = unsafe extern "C" fn(*mut c_void, *mut c_char) -> *mut c_char;
pub type FnStrreset = unsafe extern "C" fn(*mut c_void);
pub type FnStrkey = unsafe extern "C" fn(c_int) -> *mut c_char;
pub type FnIntput = unsafe extern "C" fn(c_int);

pub struct Lib {
    pub name: &'static str,
    _lib: &'static Library,
    pub arrgrowf: FnArrGrowf,
    pub arrfreef: FnArrFreef,
    pub rand_seed: FnRandSeed,
    pub hash_bytes: FnHashBytes,
    pub hash_string: FnHashString,
    pub hmfree_func: FnHmfreeFunc,
    pub hmget_key: FnHmgetKey,
    pub hmget_key_ts: FnHmgetKeyTs,
    pub hmput_default: FnHmputDefault,
    pub hmput_key: FnHmputKey,
    pub hmdel_key: FnHmdelKey,
    pub shmode_func: FnShmodeFunc,
    pub stralloc: FnStralloc,
    pub strreset: FnStrreset,
    pub strkey: FnStrkey,
    pub intput: FnIntput,
}

macro_rules! sym {
    ($lib:expr, $name:literal, $ty:ty) => {{
        let s: libloading::Symbol<$ty> = unsafe {
            $lib.get(concat!($name, "\0").as_bytes())
                .unwrap_or_else(|e| panic!("missing symbol {}: {e}", $name))
        };
        *s
    }};
}

impl Lib {
    fn open(name: &'static str, path: &PathBuf) -> Lib {
        let lib: &'static Library = Box::leak(Box::new(unsafe {
            Library::new(path).unwrap_or_else(|e| panic!("cannot load {}: {e}", path.display()))
        }));
        Lib {
            name,
            arrgrowf: sym!(lib, "stbds_arrgrowf", FnArrGrowf),
            arrfreef: sym!(lib, "stbds_arrfreef", FnArrFreef),
            rand_seed: sym!(lib, "stbds_rand_seed", FnRandSeed),
            hash_bytes: sym!(lib, "stbds_hash_bytes", FnHashBytes),
            hash_string: sym!(lib, "stbds_hash_string", FnHashString),
            hmfree_func: sym!(lib, "stbds_hmfree_func", FnHmfreeFunc),
            hmget_key: sym!(lib, "stbds_hmget_key", FnHmgetKey),
            hmget_key_ts: sym!(lib, "stbds_hmget_key_ts", FnHmgetKeyTs),
            hmput_default: sym!(lib, "stbds_hmput_default", FnHmputDefault),
            hmput_key: sym!(lib, "stbds_hmput_key", FnHmputKey),
            hmdel_key: sym!(lib, "stbds_hmdel_key", FnHmdelKey),
            shmode_func: sym!(lib, "stbds_shmode_func", FnShmodeFunc),
            stralloc: sym!(lib, "stbds_stralloc", FnStralloc),
            strreset: sym!(lib, "stbds_strreset", FnStrreset),
            strkey: sym!(lib, "strkey", FnStrkey),
            intput: sym!(lib, "intput", FnIntput),
            _lib: lib,
        }
    }
}

fn target_dir() -> PathBuf {
    // <target>/<profile>/deps/<test-exe>
    let exe = std::env::current_exe().expect("current_exe");
    exe.parent().unwrap().parent().unwrap().to_path_buf()
}

fn crate_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

pub fn rust_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("DIFFTEST_RUST_SO") {
        let p = PathBuf::from(p);
        assert!(p.exists(), "DIFFTEST_RUST_SO does not exist: {}", p.display());
        return p;
    }
    let root = crate_root().join("target");
    let candidates = [
        target_dir().join("libintput_lib.so"),
        root.join("debug").join("libintput_lib.so"),
        root.join("release").join("libintput_lib.so"),
    ];
    for p in &candidates {
        if p.exists() {
            return p.clone();
        }
    }
    panic!(
        "rust cdylib not found; run `cargo build` first. tried: {:?}",
        candidates
    );
}

pub fn c_so_path() -> PathBuf {
    let dir = crate_root().parent().unwrap().join("c_src").join("build");
    let mut found = None;
    for e in std::fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("cannot read {}: {e} (did you build the C library?)", dir.display()))
    {
        let p = e.unwrap().path();
        let n = p.file_name().unwrap().to_string_lossy().to_string();
        if n.starts_with("lib") && n.ends_with(".so") {
            found = Some(p);
        }
    }
    found.unwrap_or_else(|| panic!("no lib*.so in {}", dir.display()))
}

/// Both libraries, freshly loaded (leaked, so the fn pointers stay valid).
pub fn libs() -> (&'static Lib, &'static Lib) {
    use std::sync::OnceLock;
    static PAIR: OnceLock<(Lib, Lib)> = OnceLock::new();
    let p = PAIR.get_or_init(|| {
        (
            Lib::open("C", &c_so_path()),
            Lib::open("RUST", &rust_so_path()),
        )
    });
    (&p.0, &p.1)
}

/// Both libraries keep a *global* `stbds_hash_seed` that every
/// `stbds_make_hash_index(_, NULL)` advances. Two tests running concurrently
/// would interleave their seed consumption differently in the C `.so` than in
/// the Rust `.so`, so all library interaction is serialised through this lock.
pub fn lock() -> std::sync::MutexGuard<'static, ()> {
    use std::sync::{Mutex, OnceLock};
    static M: OnceLock<Mutex<()>> = OnceLock::new();
    match M.get_or_init(|| Mutex::new(())).lock() {
        Ok(g) => g,
        Err(p) => p.into_inner(),
    }
}

// ---------------------------------------------------------------------------
// Deterministic RNG (xorshift64*) so both sides see identical input streams
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
    pub fn next_u32(&mut self) -> u32 {
        (self.next_u64() >> 32) as u32
    }
    /// Uniform-ish in `0..n`.
    pub fn below(&mut self, n: usize) -> usize {
        (self.next_u64() % (n as u64)) as usize
    }
    pub fn bytes(&mut self, n: usize) -> Vec<u8> {
        (0..n).map(|_| (self.next_u64() >> 24) as u8).collect()
    }
    /// A NUL-terminated ASCII string of `n` visible chars.
    pub fn cstring(&mut self, n: usize) -> Vec<u8> {
        let mut v: Vec<u8> = (0..n)
            .map(|_| b'a' + (self.next_u64() % 26) as u8)
            .collect();
        v.push(0);
        v
    }
    /// A NUL-terminated string of `n` arbitrary non-zero bytes (incl. >= 0x80).
    pub fn cstring_raw(&mut self, n: usize) -> Vec<u8> {
        let mut v: Vec<u8> = (0..n)
            .map(|_| {
                let b = (self.next_u64() >> 20) as u8;
                if b == 0 { 1 } else { b }
            })
            .collect();
        v.push(0);
        v
    }
}

// ---------------------------------------------------------------------------
// State snapshots
// ---------------------------------------------------------------------------

/// Everything observable about a hash-map pointer, with all raw addresses
/// normalised away (pointers are reduced to "null / non-null", and
/// pointed-to strings are captured by value).
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct MapSnapshot {
    pub ptr_null: bool,
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
    pub string_storage_null: bool,
    pub string_remaining: usize,
    pub string_block: u8,
    pub string_mode: u8,
    /// `hash[]` for every bucket, in slot order.
    pub bucket_hash: Vec<usize>,
    /// `index[]` for every bucket, in slot order.
    pub bucket_index: Vec<isize>,
    /// Payload of elements `0..length`, with pointer-typed key fields replaced
    /// by the string they point at.
    pub elements: Vec<Vec<u8>>,
}

/// How the key field of an element should be read back.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum KeyKind {
    /// Key is `keysize` inline bytes at `keyoffset`.
    Bytes,
    /// Key is a `char *` at `keyoffset`; compare the string it points at.
    StrPtr,
}

/// `t` is the value returned by `hmput_key` / `hmget_key` / … (the "hash"
/// pointer, i.e. `raw_array + elemsize`).
pub unsafe fn snapshot_map(
    t: *mut c_void,
    elemsize: usize,
    keyoffset: usize,
    kind: KeyKind,
) -> MapSnapshot {
    unsafe {
        if t.is_null() {
            return MapSnapshot {
                ptr_null: true,
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
                string_storage_null: true,
                string_remaining: 0,
                string_block: 0,
                string_mode: 0,
                bucket_hash: vec![],
                bucket_index: vec![],
                elements: vec![],
            };
        }
        let raw_a = (t as *mut u8).sub(elemsize);
        let h = raw_a.sub(HEADER_SIZE) as *const RawArrayHeader;
        let hdr = *h;
        let table = hdr.hash_table as *const RawHashIndex;

        let mut snap = MapSnapshot {
            ptr_null: false,
            length: hdr.length,
            capacity: hdr.capacity,
            temp: hdr.temp,
            has_table: !table.is_null(),
            slot_count: 0,
            used_count: 0,
            used_count_threshold: 0,
            used_count_shrink_threshold: 0,
            tombstone_count: 0,
            tombstone_count_threshold: 0,
            seed: 0,
            slot_count_log2: 0,
            string_storage_null: true,
            string_remaining: 0,
            string_block: 0,
            string_mode: 0,
            bucket_hash: vec![],
            bucket_index: vec![],
            elements: vec![],
        };

        if !table.is_null() {
            let ti = *table;
            snap.slot_count = ti.slot_count;
            snap.used_count = ti.used_count;
            snap.used_count_threshold = ti.used_count_threshold;
            snap.used_count_shrink_threshold = ti.used_count_shrink_threshold;
            snap.tombstone_count = ti.tombstone_count;
            snap.tombstone_count_threshold = ti.tombstone_count_threshold;
            snap.seed = ti.seed;
            snap.slot_count_log2 = ti.slot_count_log2;
            snap.string_storage_null = ti.string.storage.is_null();
            snap.string_remaining = ti.string.remaining;
            snap.string_block = ti.string.block;
            snap.string_mode = ti.string.mode;
            let nbuckets = ti.slot_count >> BUCKET_SHIFT;
            for b in 0..nbuckets {
                let bucket = &*ti.storage.add(b);
                snap.bucket_hash.extend_from_slice(&bucket.hash);
                snap.bucket_index.extend_from_slice(&bucket.index);
            }
        }

        for i in 0..hdr.length {
            let e = raw_a.add(elemsize * i);
            let mut bytes = std::slice::from_raw_parts(e, elemsize).to_vec();
            if kind == KeyKind::StrPtr {
                // Replace the 8 pointer bytes with the pointed-to string (or a
                // null marker), so addresses never leak into the comparison.
                let pp = *(e.add(keyoffset) as *const *const c_char);
                let mut repl = Vec::new();
                if pp.is_null() {
                    repl.push(0xFEu8);
                } else {
                    repl.push(0xFDu8);
                    let mut q = pp as *const u8;
                    loop {
                        let c = *q;
                        if c == 0 {
                            break;
                        }
                        repl.push(c);
                        q = q.add(1);
                    }
                }
                bytes.splice(keyoffset..keyoffset + 8, repl);
            }
            snap.elements.push(bytes);
        }

        snap
    }
}

/// Snapshot of a plain array (`arrgrowf` result): header only, plus the first
/// `payload_len` bytes (caller guarantees they were written).
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct ArrSnapshot {
    pub null: bool,
    pub length: usize,
    pub capacity: usize,
    pub temp: isize,
    pub hash_table_null: bool,
    pub payload: Vec<u8>,
}

pub unsafe fn snapshot_arr(a: *mut c_void, payload_len: usize) -> ArrSnapshot {
    unsafe {
        if a.is_null() {
            return ArrSnapshot {
                null: true,
                length: 0,
                capacity: 0,
                temp: 0,
                hash_table_null: true,
                payload: vec![],
            };
        }
        let h = *((a as *mut u8).sub(HEADER_SIZE) as *const RawArrayHeader);
        ArrSnapshot {
            null: false,
            length: h.length,
            capacity: h.capacity,
            temp: h.temp,
            hash_table_null: h.hash_table.is_null(),
            payload: std::slice::from_raw_parts(a as *const u8, payload_len).to_vec(),
        }
    }
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct ArenaSnapshot {
    pub storage_null: bool,
    pub remaining: usize,
    pub block: u8,
    pub mode: u8,
    /// NUL-terminated strings reachable from each block, in list order,
    /// captured as the block's raw bytes are not comparable (addresses).
    pub block_count: usize,
}

pub unsafe fn snapshot_arena(a: *const RawStringArena) -> ArenaSnapshot {
    unsafe {
        let s = *a;
        let mut n = 0usize;
        let mut x = s.storage as *const *const c_void; // first field is `next`
        while !x.is_null() {
            n += 1;
            if n > 100000 {
                panic!("arena block list cycle");
            }
            x = *x as *const *const c_void;
        }
        ArenaSnapshot {
            storage_null: s.storage.is_null(),
            remaining: s.remaining,
            block: s.block,
            mode: s.mode,
            block_count: n,
        }
    }
}

pub unsafe fn cstr_bytes(p: *const c_char) -> Vec<u8> {
    unsafe {
        let mut v = Vec::new();
        let mut q = p as *const u8;
        loop {
            let c = *q;
            if c == 0 {
                break;
            }
            v.push(c);
            q = q.add(1);
        }
        v
    }
}

// ---------------------------------------------------------------------------
// Abort-parity helper: re-exec this test binary to run one scenario in a child
// ---------------------------------------------------------------------------

pub const CASE_ENV: &str = "DIFFTEST_CASE";
pub const WHICH_ENV: &str = "DIFFTEST_WHICH";

/// Runs the `abort_child` test in a subprocess with `case`/`which` set and
/// returns `(exit_code, signal)`.
pub fn run_child(case: &str, which: &str) -> (Option<i32>, Option<i32>) {
    use std::os::unix::process::ExitStatusExt;
    let exe = std::env::current_exe().unwrap();
    let out = std::process::Command::new(exe)
        .args(["--exact", "abort_child", "--nocapture", "--test-threads=1"])
        .env(CASE_ENV, case)
        .env(WHICH_ENV, which)
        .output()
        .expect("spawn child");
    (out.status.code(), out.status.signal())
}

/// Asserts C and Rust terminate identically for the given scenario name.
pub fn assert_abort_parity(case: &str) {
    let c = run_child(case, "c");
    let r = run_child(case, "rust");
    assert_eq!(c, r, "termination mismatch for case `{case}`: C={c:?} RUST={r:?}");
}

/// Selects the library the child should drive.
pub fn child_lib() -> &'static Lib {
    let (c, r) = libs();
    match std::env::var(WHICH_ENV).unwrap_or_default().as_str() {
        "c" => c,
        "rust" => r,
        other => panic!("bad {WHICH_ENV}: {other}"),
    }
}

// ---------------------------------------------------------------------------
// MapPair — drives the C and Rust hash map in lockstep
// ---------------------------------------------------------------------------

/// A pair of hash maps (one per library) kept in lockstep. Every operation is
/// applied to both through their `.so` exports and the resulting state is
/// compared byte-for-byte.
pub struct MapPair {
    pub c: *mut c_void,
    pub r: *mut c_void,
    pub elemsize: usize,
    pub keysize: usize,
    pub keyoffset: usize,
    pub kind: KeyKind,
    /// Keeps `SH_DEFAULT`-mode key buffers alive for the map's lifetime.
    keep: Vec<Box<[u8]>>,
    pub cl: &'static Lib,
    pub rl: &'static Lib,
}

impl MapPair {
    /// Empty (NULL) pair; the first `put` creates the table.
    pub fn new(elemsize: usize, keysize: usize, kind: KeyKind) -> MapPair {
        let (cl, rl) = libs();
        MapPair {
            c: std::ptr::null_mut(),
            r: std::ptr::null_mut(),
            elemsize,
            keysize,
            keyoffset: 0,
            kind,
            keep: Vec::new(),
            cl,
            rl,
        }
    }

    /// Pair created via `stbds_shmode_func(elemsize, mode)` with the global
    /// seed pinned to `seed` on both sides first.
    pub fn with_shmode(elemsize: usize, keysize: usize, kind: KeyKind, mode: c_int, seed: usize) -> MapPair {
        let mut p = MapPair::new(elemsize, keysize, kind);
        unsafe {
            (p.cl.rand_seed)(seed);
            (p.rl.rand_seed)(seed);
            p.c = (p.cl.shmode_func)(elemsize, mode);
            p.r = (p.rl.shmode_func)(elemsize, mode);
        }
        p
    }

    /// Pin the global hash seed on both libraries (call before an op that
    /// creates a fresh table).
    pub fn pin_seed(&self, seed: usize) {
        unsafe {
            (self.cl.rand_seed)(seed);
            (self.rl.rand_seed)(seed);
        }
    }

    pub fn snap(&self) -> (MapSnapshot, MapSnapshot) {
        unsafe {
            (
                snapshot_map(self.c, self.elemsize, self.keyoffset, self.kind),
                snapshot_map(self.r, self.elemsize, self.keyoffset, self.kind),
            )
        }
    }

    pub fn assert_same(&self, what: &str) {
        let (a, b) = self.snap();
        assert_eq!(a.ptr_null, b.ptr_null, "{what}: null-ness differs");
        assert_eq!(a.length, b.length, "{what}: length C={} RUST={}", a.length, b.length);
        assert_eq!(a.capacity, b.capacity, "{what}: capacity");
        assert_eq!(a.temp, b.temp, "{what}: temp C={} RUST={}", a.temp, b.temp);
        assert_eq!(a.has_table, b.has_table, "{what}: has_table");
        assert_eq!(a.slot_count, b.slot_count, "{what}: slot_count");
        assert_eq!(a.used_count, b.used_count, "{what}: used_count");
        assert_eq!(a.used_count_threshold, b.used_count_threshold, "{what}: used_count_threshold");
        assert_eq!(a.used_count_shrink_threshold, b.used_count_shrink_threshold, "{what}: shrink_threshold");
        assert_eq!(a.tombstone_count, b.tombstone_count, "{what}: tombstone_count");
        assert_eq!(a.tombstone_count_threshold, b.tombstone_count_threshold, "{what}: tombstone_threshold");
        assert_eq!(a.seed, b.seed, "{what}: table seed C={:#x} RUST={:#x}", a.seed, b.seed);
        assert_eq!(a.slot_count_log2, b.slot_count_log2, "{what}: slot_count_log2");
        assert_eq!(a.string_storage_null, b.string_storage_null, "{what}: string.storage null-ness");
        assert_eq!(a.string_remaining, b.string_remaining, "{what}: string.remaining");
        assert_eq!(a.string_block, b.string_block, "{what}: string.block");
        assert_eq!(a.string_mode, b.string_mode, "{what}: string.mode");
        assert_eq!(a.bucket_hash, b.bucket_hash, "{what}: bucket hash[] differs");
        assert_eq!(a.bucket_index, b.bucket_index, "{what}: bucket index[] differs");
        assert_eq!(a.elements, b.elements, "{what}: element payload differs");
    }

    fn header(p: *mut c_void, elemsize: usize) -> *mut RawArrayHeader {
        unsafe { (p as *mut u8).sub(elemsize).sub(HEADER_SIZE) as *mut RawArrayHeader }
    }

    /// Write the non-key part of element `t[temp]` with a deterministic fill so
    /// no uninitialised realloc bytes ever enter the comparison.
    unsafe fn write_value(&self, p: *mut c_void, temp: isize, fill: u8) {
        unsafe {
            let e = (p as *mut u8).offset(self.elemsize as isize * temp);
            match self.kind {
                KeyKind::Bytes => {
                    // fill everything, then restore the key bytes the library stored
                    let mut key = vec![0u8; self.keysize];
                    std::ptr::copy_nonoverlapping(e.add(self.keyoffset), key.as_mut_ptr(), self.keysize);
                    std::ptr::write_bytes(e, fill, self.elemsize);
                    std::ptr::copy_nonoverlapping(key.as_ptr(), e.add(self.keyoffset), self.keysize);
                }
                KeyKind::StrPtr => {
                    // preserve the 8-byte char* at keyoffset
                    if self.keyoffset > 0 {
                        std::ptr::write_bytes(e, fill, self.keyoffset);
                    }
                    let after = self.keyoffset + 8;
                    if self.elemsize > after {
                        std::ptr::write_bytes(e.add(after), fill, self.elemsize - after);
                    }
                }
            }
        }
    }

    /// `stbds_hmput_key(t, elemsize, key, keysize, mode)` on both, then write
    /// the value part, then compare. Returns the `temp` index.
    pub fn put(&mut self, key: &[u8], mode: c_int, fill: u8) -> isize {
        // Each library gets its own key buffer: SH_DEFAULT stores the pointer
        // verbatim, so the buffers must be distinct and must outlive the map.
        let ckey: Box<[u8]> = key.to_vec().into_boxed_slice();
        let rkey: Box<[u8]> = key.to_vec().into_boxed_slice();
        let cp = ckey.as_ptr() as *mut c_void;
        let rp = rkey.as_ptr() as *mut c_void;
        self.keep.push(ckey);
        self.keep.push(rkey);
        unsafe {
            self.c = (self.cl.hmput_key)(self.c, self.elemsize, cp, self.keysize, mode);
            self.r = (self.rl.hmput_key)(self.r, self.elemsize, rp, self.keysize, mode);
            let ct = (*Self::header(self.c, self.elemsize)).temp;
            let rt = (*Self::header(self.r, self.elemsize)).temp;
            assert_eq!(ct, rt, "put temp mismatch (key={key:?}, mode={mode})");
            self.write_value(self.c, ct, fill);
            self.write_value(self.r, rt, fill);
            self.assert_same(&format!("after put(key={key:?}, mode={mode})"));
            ct
        }
    }

    /// `stbds_hmget_key`; returns `temp` (the element index, or -1).
    pub fn get(&mut self, key: &[u8], mode: c_int) -> isize {
        let mut ck = key.to_vec();
        let mut rk = key.to_vec();
        unsafe {
            self.c = (self.cl.hmget_key)(
                self.c, self.elemsize, ck.as_mut_ptr() as *mut c_void, self.keysize, mode);
            self.r = (self.rl.hmget_key)(
                self.r, self.elemsize, rk.as_mut_ptr() as *mut c_void, self.keysize, mode);
            let ct = (*Self::header(self.c, self.elemsize)).temp;
            let rt = (*Self::header(self.r, self.elemsize)).temp;
            assert_eq!(ct, rt, "get temp mismatch (key={key:?}, mode={mode})");
            self.assert_same(&format!("after get(key={key:?}, mode={mode})"));
            ct
        }
    }

    /// `stbds_hmget_key_ts`; returns the `*temp` out-param value.
    pub fn get_ts(&mut self, key: &[u8], mode: c_int) -> isize {
        let mut ck = key.to_vec();
        let mut rk = key.to_vec();
        let mut ctemp: isize = 0x5A5A;
        let mut rtemp: isize = 0x5A5A;
        unsafe {
            self.c = (self.cl.hmget_key_ts)(
                self.c, self.elemsize, ck.as_mut_ptr() as *mut c_void, self.keysize, &mut ctemp, mode);
            self.r = (self.rl.hmget_key_ts)(
                self.r, self.elemsize, rk.as_mut_ptr() as *mut c_void, self.keysize, &mut rtemp, mode);
            assert_eq!(ctemp, rtemp, "get_ts *temp mismatch (key={key:?}, mode={mode})");
            self.assert_same(&format!("after get_ts(key={key:?}, mode={mode})"));
            ctemp
        }
    }

    /// `stbds_hmdel_key`; returns `temp` (1 if deleted, 0 otherwise).
    pub fn del(&mut self, key: &[u8], mode: c_int) -> isize {
        let mut ck = key.to_vec();
        let mut rk = key.to_vec();
        unsafe {
            let cn = (self.cl.hmdel_key)(
                self.c, self.elemsize, ck.as_mut_ptr() as *mut c_void, self.keysize, self.keyoffset, mode);
            let rn = (self.rl.hmdel_key)(
                self.r, self.elemsize, rk.as_mut_ptr() as *mut c_void, self.keysize, self.keyoffset, mode);
            assert_eq!(cn.is_null(), rn.is_null(), "del null-ness mismatch (key={key:?})");
            self.c = cn;
            self.r = rn;
            if cn.is_null() {
                return 0;
            }
            let ct = (*Self::header(self.c, self.elemsize)).temp;
            let rt = (*Self::header(self.r, self.elemsize)).temp;
            assert_eq!(ct, rt, "del temp mismatch (key={key:?}, mode={mode})");
            self.assert_same(&format!("after del(key={key:?}, mode={mode})"));
            ct
        }
    }

    pub fn put_default(&mut self) {
        unsafe {
            self.c = (self.cl.hmput_default)(self.c, self.elemsize);
            self.r = (self.rl.hmput_default)(self.r, self.elemsize);
            self.assert_same("after hmput_default");
        }
    }

    /// `stbds_hmfree_func((t)-1, elemsize)` — the `hmfree` macro's argument.
    pub fn free(&mut self) {
        unsafe {
            if !self.c.is_null() {
                (self.cl.hmfree_func)((self.c as *mut u8).sub(self.elemsize) as *mut c_void, self.elemsize);
            }
            if !self.r.is_null() {
                (self.rl.hmfree_func)((self.r as *mut u8).sub(self.elemsize) as *mut c_void, self.elemsize);
            }
            self.c = std::ptr::null_mut();
            self.r = std::ptr::null_mut();
            self.keep.clear();
        }
    }
}

/// `stbds_temp_key(t)` == `*(char **) stbds_header(t)->hash_table`, i.e. the
/// `temp_key` field of the hash index. Returns the string it points at.
///
/// Only safe to call when the current table generation has already had at least
/// one string-mode put (the field is uninitialised realloc memory otherwise).
pub unsafe fn temp_key_string(t: *mut c_void, elemsize: usize) -> Option<Vec<u8>> {
    unsafe {
        if t.is_null() {
            return None;
        }
        let raw_a = (t as *mut u8).sub(elemsize);
        let hdr = *(raw_a.sub(HEADER_SIZE) as *const RawArrayHeader);
        if hdr.hash_table.is_null() {
            return None;
        }
        let p = *(hdr.hash_table as *const *const c_char);
        if p.is_null() {
            return None;
        }
        Some(cstr_bytes(p))
    }
}

/// Raw `stbds_header(t-1)->hash_table` pointer — used only to detect that a
/// table was re-created (grown / shrunk / rebuilt), which leaves the
/// uninitialised `temp_key` field indeterminate.
pub unsafe fn table_ptr(t: *mut c_void, elemsize: usize) -> *mut c_void {
    unsafe {
        if t.is_null() {
            return std::ptr::null_mut();
        }
        let raw_a = (t as *mut u8).sub(elemsize);
        (*(raw_a.sub(HEADER_SIZE) as *const RawArrayHeader)).hash_table
    }
}
