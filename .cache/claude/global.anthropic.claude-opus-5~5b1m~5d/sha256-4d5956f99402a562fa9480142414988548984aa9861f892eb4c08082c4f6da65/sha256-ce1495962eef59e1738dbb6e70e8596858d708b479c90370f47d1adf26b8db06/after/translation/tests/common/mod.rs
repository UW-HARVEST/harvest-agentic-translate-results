//! Shared differential-test harness.
//!
//! BOTH implementations are loaded as shared objects through `libloading` and
//! driven exclusively through their exported C symbols — the Rust crate is never
//! called directly, so the `#[no_mangle]` wrappers are part of what is tested.

#![allow(dead_code)]
#![allow(non_snake_case)]

use libloading::Library;
use std::ffi::{c_char, c_int, c_void};
use std::path::{Path, PathBuf};

// ---------------------------------------------------------------------------
// Mirrored C layouts (private in lib.c, reconstructed here so the tests can
// compare the *entire* internal state, not just return values)
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
    pub fn zeroed() -> StringArena {
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
#[derive(Copy, Clone)]
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

pub const HEADER_SIZE: usize = std::mem::size_of::<ArrayHeader>();

pub const SH_NONE: u8 = 0;
pub const SH_DEFAULT: u8 = 1;
pub const SH_STRDUP: u8 = 2;
pub const SH_ARENA: u8 = 3;

pub const HM_BINARY: c_int = 0;
pub const HM_STRING: c_int = 1;

pub const DEFAULT_SEED: usize = 0x3141_5926;

// ---------------------------------------------------------------------------
// FFI signatures of every exported symbol
// ---------------------------------------------------------------------------

pub type FnArrgrowf = unsafe extern "C" fn(*mut c_void, usize, usize, usize) -> *mut c_void;
pub type FnArrfreef = unsafe extern "C" fn(*mut c_void);
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
pub type FnStralloc = unsafe extern "C" fn(*mut StringArena, *mut c_char) -> *mut c_char;
pub type FnStrreset = unsafe extern "C" fn(*mut StringArena);
pub type FnStrkey = unsafe extern "C" fn(c_int) -> *mut c_char;
pub type FnShGeti = unsafe extern "C" fn(c_int);

pub struct Api {
    pub name: &'static str,
    pub path: PathBuf,
    pub arrgrowf: FnArrgrowf,
    pub arrfreef: FnArrfreef,
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
    pub sh_geti: FnShGeti,
}

macro_rules! sym {
    ($lib:expr, $ty:ty, $name:literal) => {
        *$lib
            .get::<$ty>(concat!($name, "\0").as_bytes())
            .unwrap_or_else(|e| panic!("missing symbol {}: {}", $name, e))
    };
}

impl Api {
    fn load(path: &Path, name: &'static str) -> Api {
        let lib: &'static Library = Box::leak(Box::new(unsafe {
            Library::new(path).unwrap_or_else(|e| panic!("dlopen({}) failed: {}", path.display(), e))
        }));
        unsafe {
            Api {
                name,
                path: path.to_path_buf(),
                arrgrowf: sym!(lib, FnArrgrowf, "stbds_arrgrowf"),
                arrfreef: sym!(lib, FnArrfreef, "stbds_arrfreef"),
                rand_seed: sym!(lib, FnRandSeed, "stbds_rand_seed"),
                hash_string: sym!(lib, FnHashString, "stbds_hash_string"),
                hash_bytes: sym!(lib, FnHashBytes, "stbds_hash_bytes"),
                hmfree_func: sym!(lib, FnHmfreeFunc, "stbds_hmfree_func"),
                hmget_key_ts: sym!(lib, FnHmgetKeyTs, "stbds_hmget_key_ts"),
                hmget_key: sym!(lib, FnHmgetKey, "stbds_hmget_key"),
                hmput_default: sym!(lib, FnHmputDefault, "stbds_hmput_default"),
                hmput_key: sym!(lib, FnHmputKey, "stbds_hmput_key"),
                shmode_func: sym!(lib, FnShmodeFunc, "stbds_shmode_func"),
                hmdel_key: sym!(lib, FnHmdelKey, "stbds_hmdel_key"),
                stralloc: sym!(lib, FnStralloc, "stbds_stralloc"),
                strreset: sym!(lib, FnStrreset, "stbds_strreset"),
                strkey: sym!(lib, FnStrkey, "strkey"),
                sh_geti: sym!(lib, FnShGeti, "sh_geti"),
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Locating the two .so files
// ---------------------------------------------------------------------------

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn newest(mut cands: Vec<PathBuf>) -> Option<PathBuf> {
    cands.retain(|p| p.is_file());
    cands.sort_by_key(|p| {
        std::fs::metadata(p)
            .and_then(|m| m.modified())
            .unwrap_or(std::time::SystemTime::UNIX_EPOCH)
    });
    cands.pop()
}

fn c_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("HARVEST_C_SO") {
        return PathBuf::from(p);
    }
    let build = manifest_dir().join("../c_src/build");
    let mut cands = Vec::new();
    if let Ok(rd) = std::fs::read_dir(&build) {
        for e in rd.flatten() {
            let p = e.path();
            let n = p.file_name().unwrap_or_default().to_string_lossy().to_string();
            if n.starts_with("lib") && n.ends_with(".so") {
                cands.push(p);
            }
        }
    }
    newest(cands).unwrap_or_else(|| {
        panic!(
            "no C .so found in {} — build it with:\n  cd c_src && mkdir -p build && cd build && \
             cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
            build.display()
        )
    })
}

fn mtime(p: &Path) -> std::time::SystemTime {
    std::fs::metadata(p)
        .and_then(|m| m.modified())
        .unwrap_or(std::time::SystemTime::UNIX_EPOCH)
}

fn rust_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("HARVEST_RUST_SO") {
        return PathBuf::from(p);
    }
    let md = manifest_dir();
    let cands = vec![
        md.join("target/release/libsh_geti_lib.so"),
        md.join("target/debug/libsh_geti_lib.so"),
    ];
    let so = newest(cands).unwrap_or_else(|| {
        panic!(
            "no Rust .so found under {}/target — build it with `cargo build --release --offline`",
            md.display()
        )
    });
    // `cargo test` does not rebuild the cdylib, so guard against silently
    // testing a stale .so after `src/lib.rs` changed.
    let src = md.join("src/lib.rs");
    if src.is_file() && mtime(&src) > mtime(&so) {
        panic!(
            "the Rust .so ({}) is OLDER than src/lib.rs — run `cargo build --release --offline`              (or `cargo build --offline`) before `cargo test`",
            so.display()
        );
    }
    so
}

pub struct Pair {
    pub c: Api,
    pub r: Api,
}

static PAIR: std::sync::OnceLock<Pair> = std::sync::OnceLock::new();

pub fn pair() -> &'static Pair {
    PAIR.get_or_init(|| Pair {
        c: Api::load(&c_so_path(), "C"),
        r: Api::load(&rust_so_path(), "Rust"),
    })
}

/// Put both libraries' global `stbds_hash_seed` into the same known state.
pub fn reseed(seed: usize) {
    let p = pair();
    unsafe {
        (p.c.rand_seed)(seed);
        (p.r.rand_seed)(seed);
    }
}

/// `stbds_hash_seed` is a *global* in each `.so` and every fresh hash index
/// advances it.  Because the two libraries are advanced one after the other,
/// concurrent test threads could interleave the two sequences and desynchronise
/// them.  Any test that creates hash indices must therefore hold this guard.
/// It also protects the other per-`.so` global: the `static char buffer[256]`
/// that `strkey` (and therefore `sh_geti`) writes into.
static SEED_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// Serialise access to the libraries' mutable globals without touching the seed.
pub fn globals_guard() -> std::sync::MutexGuard<'static, ()> {
    SEED_LOCK.lock().unwrap_or_else(|e| e.into_inner())
}

pub fn seed_guard(seed: usize) -> std::sync::MutexGuard<'static, ()> {
    let g = globals_guard();
    reseed(seed);
    g
}

/// `stbds_load_32_or_64(var, temp, v32, v64_hi, v64_lo)` transcribed from
/// `lib.c:360-363` (all sub-expressions are 32-bit `unsigned int` in C).
fn load_32_or_64(v32: u32, v64_hi: u32, v64_lo: u32) -> usize {
    let temp = (v64_lo ^ v32) as usize; // zero-extended unsigned int
    let mut var = (v64_hi as usize) << 16 << 16;
    var ^= temp ^ (v32 as usize);
    var
}

/// The LCG the C uses to advance `stbds_hash_seed` (`lib.c:410-412`).
pub fn next_hash_seed(seed: usize) -> usize {
    let a = load_32_or_64(2147001325, 0x27bb_2ee6, 0x87b0_b0fd);
    let b = load_32_or_64(715136305, 0, 0xb504_f32d);
    seed.wrapping_mul(a).wrapping_add(b)
}

// ---------------------------------------------------------------------------
// Deterministic RNG (splitmix64) — reproducible across runs
// ---------------------------------------------------------------------------

pub struct Rng(u64);

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
    pub fn next_u8(&mut self) -> u8 {
        (self.next_u64() >> 56) as u8
    }
    /// Uniform-ish in `0..n` (n > 0).
    pub fn below(&mut self, n: usize) -> usize {
        (self.next_u64() % (n as u64)) as usize
    }
    pub fn bytes(&mut self, n: usize) -> Vec<u8> {
        (0..n).map(|_| self.next_u8()).collect()
    }
    pub fn pick<'a, T>(&mut self, s: &'a [T]) -> &'a T {
        &s[self.below(s.len())]
    }
    /// A NUL-terminated C string of `len` payload bytes drawn from `alphabet`.
    pub fn cstring(&mut self, len: usize, alphabet: &[u8]) -> Vec<u8> {
        let mut v: Vec<u8> = (0..len).map(|_| *self.pick(alphabet)).collect();
        v.push(0);
        v
    }
}

pub const ASCII: &[u8] = b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789_-";
pub const HIGHBYTES: &[u8] = &[
    0x80, 0x81, 0x9f, 0xa0, 0xc3, 0xe9, 0xfe, 0xff, 0x7f, 0x01, 0x41, 0x5a,
];

// ---------------------------------------------------------------------------
// State snapshots
// ---------------------------------------------------------------------------

/// How to interpret the first 8 bytes of an element.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum KeyRepr {
    /// element starts with a `char *` that must be dereferenced
    Pointer,
    /// element bytes are compared verbatim
    Raw,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TableSnap {
    pub slot_count: usize,
    pub used_count: usize,
    pub used_count_threshold: usize,
    pub used_count_shrink_threshold: usize,
    pub tombstone_count: usize,
    pub tombstone_count_threshold: usize,
    pub seed: usize,
    pub slot_count_log2: usize,
    pub str_mode: u8,
    pub str_block: u8,
    pub str_remaining: usize,
    pub str_has_storage: bool,
    pub buckets: Vec<(Vec<usize>, Vec<isize>)>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MapSnap {
    pub is_null: bool,
    pub length: usize,
    pub capacity: usize,
    pub temp: isize,
    pub has_table: bool,
    pub elems: Vec<Vec<u8>>,
    pub table: Option<TableSnap>,
}

/// Read a NUL-terminated string with a hard cap so a wild pointer cannot hang
/// the test forever.
pub unsafe fn read_cstr(p: *const c_char) -> Vec<u8> {
    if p.is_null() {
        return b"<null>".to_vec();
    }
    let mut out = Vec::new();
    let mut q = p as *const u8;
    for _ in 0..(1usize << 25) {
        let b = *q;
        if b == 0 {
            break;
        }
        out.push(b);
        q = q.add(1);
    }
    out
}

/// Canonical representation of one element.
unsafe fn elem_snap(base: *const u8, elemsize: usize, idx: usize, repr: KeyRepr) -> Vec<u8> {
    let e = base.add(elemsize * idx);
    match repr {
        KeyRepr::Raw => std::slice::from_raw_parts(e, elemsize).to_vec(),
        KeyRepr::Pointer => {
            // length-prefixed so that 0xFF bytes inside a key stay unambiguous
            let mut out = Vec::new();
            let kp = *(e as *const *const c_char);
            if kp.is_null() {
                out.push(0u8);
                out.extend_from_slice(&0u32.to_le_bytes());
            } else {
                out.push(1u8);
                let s = read_cstr(kp);
                out.extend_from_slice(&(s.len() as u32).to_le_bytes());
                out.extend_from_slice(&s);
            }
            if elemsize > 8 {
                out.extend_from_slice(std::slice::from_raw_parts(e.add(8), elemsize - 8));
            }
            out
        }
    }
}

/// Snapshot the raw array base pointer `a` (i.e. `t - elemsize`).
pub unsafe fn snap_raw(a: *mut c_void, elemsize: usize, repr: KeyRepr) -> MapSnap {
    if a.is_null() {
        return MapSnap {
            is_null: true,
            length: 0,
            capacity: 0,
            temp: 0,
            has_table: false,
            elems: Vec::new(),
            table: None,
        };
    }
    let h = (a as *mut u8).sub(HEADER_SIZE) as *mut ArrayHeader;
    let length = (*h).length;
    let capacity = (*h).capacity;
    let temp = (*h).temp;
    let ht = (*h).hash_table as *mut HashIndex;

    let mut elems = Vec::with_capacity(length);
    if elemsize > 0 {
        for i in 0..length {
            elems.push(elem_snap(a as *const u8, elemsize, i, repr));
        }
    }

    let table = if ht.is_null() {
        None
    } else {
        let t = &*ht;
        let nbuckets = t.slot_count >> 3;
        let mut buckets = Vec::with_capacity(nbuckets);
        for i in 0..nbuckets {
            let b = &*t.storage.add(i);
            buckets.push((b.hash.to_vec(), b.index.to_vec()));
        }
        Some(TableSnap {
            slot_count: t.slot_count,
            used_count: t.used_count,
            used_count_threshold: t.used_count_threshold,
            used_count_shrink_threshold: t.used_count_shrink_threshold,
            tombstone_count: t.tombstone_count,
            tombstone_count_threshold: t.tombstone_count_threshold,
            seed: t.seed,
            slot_count_log2: t.slot_count_log2,
            str_mode: t.string.mode,
            str_block: t.string.block,
            str_remaining: t.string.remaining,
            str_has_storage: !t.string.storage.is_null(),
            buckets,
        })
    };

    MapSnap {
        is_null: false,
        length,
        capacity,
        temp,
        has_table: !ht.is_null(),
        elems,
        table,
    }
}

/// Snapshot from the *hash-map* pointer `t` (== raw base + elemsize).
pub unsafe fn snap_map(t: *mut c_void, elemsize: usize, repr: KeyRepr) -> MapSnap {
    if t.is_null() {
        return snap_raw(std::ptr::null_mut(), elemsize, repr);
    }
    snap_raw((t as *mut u8).sub(elemsize) as *mut c_void, elemsize, repr)
}

/// `table->temp_key`, canonicalised as string content.
pub unsafe fn temp_key_of(t: *mut c_void, elemsize: usize) -> Option<Vec<u8>> {
    if t.is_null() {
        return None;
    }
    let a = (t as *mut u8).sub(elemsize);
    let h = a.sub(HEADER_SIZE) as *mut ArrayHeader;
    let ht = (*h).hash_table as *mut HashIndex;
    if ht.is_null() {
        return None;
    }
    Some(read_cstr((*ht).temp_key))
}

// ---------------------------------------------------------------------------
// A pair of parallel maps, one per implementation
// ---------------------------------------------------------------------------

pub struct MapPair {
    pub tc: *mut c_void,
    pub tr: *mut c_void,
    pub elemsize: usize,
    pub repr: KeyRepr,
    pub ctx: String,
    pub step: usize,
    /// Offset of the "value" area inside an element.  `stbds_hmput_key` only
    /// initialises the *key* bytes, so the value area of a brand-new element is
    /// uninitialised `malloc` memory.  The harness therefore writes a
    /// deterministic, key-derived pattern there right after every insert (that
    /// is exactly what the `stbds_hmput`/`shput` macros do with the user's
    /// value) so that the two implementations can be compared byte-for-byte.
    pub value_offset: usize,
}

impl MapPair {
    pub fn new(elemsize: usize, repr: KeyRepr, ctx: impl Into<String>) -> MapPair {
        MapPair {
            tc: std::ptr::null_mut(),
            tr: std::ptr::null_mut(),
            elemsize,
            repr,
            ctx: ctx.into(),
            step: 0,
            value_offset: elemsize,
        }
    }

    pub fn with_value_offset(mut self, off: usize) -> MapPair {
        self.value_offset = off;
        self
    }

    /// Deterministic value bytes for a key.
    fn value_for(&self, key: &[u8]) -> Vec<u8> {
        if self.elemsize <= self.value_offset {
            return Vec::new();
        }
        let mut h: u64 = 0xcbf2_9ce4_8422_2325;
        for b in key {
            h ^= *b as u64;
            h = h.wrapping_mul(0x100_0000_01b3);
        }
        (0..(self.elemsize - self.value_offset))
            .map(|j| ((h >> ((j % 8) * 8)) as u8) ^ (j as u8))
            .collect()
    }

    pub fn check(&mut self, what: &str) {
        self.step += 1;
        let sc = unsafe { snap_map(self.tc, self.elemsize, self.repr) };
        let sr = unsafe { snap_map(self.tr, self.elemsize, self.repr) };
        if sc != sr {
            panic!(
                "DIVERGENCE [{}] step {} after `{}`\n  C    = {:#?}\n  Rust = {:#?}",
                self.ctx, self.step, what, sc, sr
            );
        }
    }

    /// `stbds_make_hash_index` never initialises `temp_key`, and
    /// `stbds_hmput_key` only writes it on the *insert* path and on the
    /// "duplicate found in the upper half of the bucket" path.  Zeroing it in
    /// both libraries before an operation turns it into a well-defined
    /// observable that can then be compared.
    pub unsafe fn zero_temp_key(&mut self) {
        for t in [self.tc, self.tr] {
            if t.is_null() {
                continue;
            }
            let a = (t as *mut u8).sub(self.elemsize);
            let h = a.sub(HEADER_SIZE) as *mut ArrayHeader;
            let ht = (*h).hash_table as *mut HashIndex;
            if !ht.is_null() {
                (*ht).temp_key = std::ptr::null_mut();
            }
        }
    }

    /// SAFETY: only call this when the preceding `hmput_key` is known to have
    /// written `temp_key` (a fresh insert, or after `zero_temp_key`).
    pub fn check_temp_key(&mut self, what: &str) {
        let kc = unsafe { temp_key_of(self.tc, self.elemsize) };
        let kr = unsafe { temp_key_of(self.tr, self.elemsize) };
        assert_eq!(
            kc, kr,
            "DIVERGENCE [{}] step {} temp_key after `{}`",
            self.ctx, self.step, what
        );
    }

    pub unsafe fn shmode(&mut self, mode: c_int) {
        let p = pair();
        self.tc = (p.c.shmode_func)(self.elemsize, mode);
        self.tr = (p.r.shmode_func)(self.elemsize, mode);
        self.check("shmode_func");
    }

    pub unsafe fn put_default(&mut self) {
        let p = pair();
        self.tc = (p.c.hmput_default)(self.tc, self.elemsize);
        self.tr = (p.r.hmput_default)(self.tr, self.elemsize);
        self.check("hmput_default");
    }

    /// Write `bytes` at `offset` inside element index `idx` (raw-array indexing)
    /// in both maps.  Used to fill in "values" the way the stb_ds macros do.
    pub unsafe fn write_elem(&mut self, idx: isize, offset: usize, bytes: &[u8]) {
        for (t, _) in [(self.tc, 0), (self.tr, 1)] {
            let e = (t as *mut u8).offset(idx * self.elemsize as isize).add(offset);
            std::ptr::copy_nonoverlapping(bytes.as_ptr(), e, bytes.len());
        }
    }

    pub unsafe fn put(&mut self, key: &mut [u8], keysize: usize, mode: c_int) -> (isize, isize) {
        let p = pair();
        self.tc = (p.c.hmput_key)(
            self.tc,
            self.elemsize,
            key.as_mut_ptr() as *mut c_void,
            keysize,
            mode,
        );
        self.tr = (p.r.hmput_key)(
            self.tr,
            self.elemsize,
            key.as_mut_ptr() as *mut c_void,
            keysize,
            mode,
        );
        let a = self.temps();
        assert_eq!(
            a.0, a.1,
            "[{}] step {}: hmput_key index divergence",
            self.ctx, self.step
        );
        // initialise the value area exactly like the stb_ds macros do, so that
        // the byte-for-byte comparison below never looks at malloc garbage
        let v = self.value_for(key);
        if !v.is_empty() {
            self.write_elem(a.0, self.value_offset, &v);
        }
        self.check("hmput_key");
        a
    }

    pub unsafe fn get(&mut self, key: &mut [u8], keysize: usize, mode: c_int) -> (isize, isize) {
        let p = pair();
        self.tc = (p.c.hmget_key)(
            self.tc,
            self.elemsize,
            key.as_mut_ptr() as *mut c_void,
            keysize,
            mode,
        );
        self.tr = (p.r.hmget_key)(
            self.tr,
            self.elemsize,
            key.as_mut_ptr() as *mut c_void,
            keysize,
            mode,
        );
        let a = self.temps();
        self.check("hmget_key");
        a
    }

    pub unsafe fn get_ts(&mut self, key: &mut [u8], keysize: usize, mode: c_int) -> (isize, isize) {
        let p = pair();
        let mut tc_temp: isize = 0x5A5A;
        let mut tr_temp: isize = 0x5A5A;
        self.tc = (p.c.hmget_key_ts)(
            self.tc,
            self.elemsize,
            key.as_mut_ptr() as *mut c_void,
            keysize,
            &mut tc_temp,
            mode,
        );
        self.tr = (p.r.hmget_key_ts)(
            self.tr,
            self.elemsize,
            key.as_mut_ptr() as *mut c_void,
            keysize,
            &mut tr_temp,
            mode,
        );
        self.check("hmget_key_ts");
        (tc_temp, tr_temp)
    }

    pub unsafe fn del(
        &mut self,
        key: &mut [u8],
        keysize: usize,
        keyoffset: usize,
        mode: c_int,
    ) -> (isize, isize) {
        let p = pair();
        self.tc = (p.c.hmdel_key)(
            self.tc,
            self.elemsize,
            key.as_mut_ptr() as *mut c_void,
            keysize,
            keyoffset,
            mode,
        );
        self.tr = (p.r.hmdel_key)(
            self.tr,
            self.elemsize,
            key.as_mut_ptr() as *mut c_void,
            keysize,
            keyoffset,
            mode,
        );
        let a = self.temps();
        self.check("hmdel_key");
        a
    }

    pub unsafe fn temps(&self) -> (isize, isize) {
        let tc = if self.tc.is_null() {
            0
        } else {
            (*(((self.tc as *mut u8).sub(self.elemsize)).sub(HEADER_SIZE) as *mut ArrayHeader)).temp
        };
        let tr = if self.tr.is_null() {
            0
        } else {
            (*(((self.tr as *mut u8).sub(self.elemsize)).sub(HEADER_SIZE) as *mut ArrayHeader)).temp
        };
        (tc, tr)
    }

    pub unsafe fn free(&mut self) {
        let p = pair();
        if !self.tc.is_null() {
            (p.c.hmfree_func)((self.tc as *mut u8).sub(self.elemsize) as *mut c_void, self.elemsize);
        }
        if !self.tr.is_null() {
            (p.r.hmfree_func)((self.tr as *mut u8).sub(self.elemsize) as *mut c_void, self.elemsize);
        }
        self.tc = std::ptr::null_mut();
        self.tr = std::ptr::null_mut();
    }
}

/// Assert both implementations returned the same "index" sentinel.
#[track_caller]
pub fn same_idx(ctx: &str, got: (isize, isize)) -> isize {
    assert_eq!(got.0, got.1, "index divergence in {}", ctx);
    got.0
}

// ---------------------------------------------------------------------------
// stdout capture (sh_geti prints through libc printf)
// ---------------------------------------------------------------------------

extern "C" {
    fn dup(fd: c_int) -> c_int;
    fn dup2(old: c_int, new: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
}

static CAP_COUNTER: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
static CAP_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// Run `f` with fd 1 redirected to a temp file and return everything written.
pub fn capture_stdout<F: FnOnce()>(f: F) -> Vec<u8> {
    use std::io::{Read, Seek, SeekFrom, Write};
    use std::os::fd::AsRawFd;

    let _guard = CAP_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let n = CAP_COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    let path = std::env::temp_dir().join(format!("harvest_cap_{}_{}.bin", std::process::id(), n));

    let mut file = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(true)
        .open(&path)
        .expect("temp file");

    let _ = std::io::stdout().flush();
    unsafe {
        fflush(std::ptr::null_mut());
    }
    let saved = unsafe { dup(1) };
    assert!(saved >= 0, "dup(1) failed");
    unsafe {
        assert!(dup2(file.as_raw_fd(), 1) >= 0, "dup2 failed");
    }

    f();

    unsafe {
        fflush(std::ptr::null_mut());
        dup2(saved, 1);
        close(saved);
    }

    file.seek(SeekFrom::Start(0)).unwrap();
    let mut out = Vec::new();
    file.read_to_end(&mut out).unwrap();
    drop(file);
    let _ = std::fs::remove_file(&path);
    out
}

pub fn show(b: &[u8]) -> String {
    String::from_utf8_lossy(b).into_owned()
}

// ---------------------------------------------------------------------------
// sh_geti stdout capture, isolated in a subprocess
// ---------------------------------------------------------------------------
//
// `capture_stdout` redirects the *process-wide* fd 1, but libtest also writes
// its own progress lines to fd 1 from the main thread while other tests run.
// Doing the capture in a child process that runs exactly one (single-threaded)
// test removes that interference completely, so a plain parallel `cargo test`
// is reliable.

/// libtest names a `#[test]` inside a module `<module>::<fn>`.
pub const SHGETI_WORKER: &str = "common::zzz_shgeti_worker";

fn parse_nums(s: &str) -> Vec<c_int> {
    if s.is_empty() {
        Vec::new()
    } else {
        s.split(',').map(|x| x.parse::<c_int>().unwrap()).collect()
    }
}

/// Worker: present in every test binary, a no-op unless `HARVEST_SHGETI` is set.
///
/// `HARVEST_SHGETI = "<c|r>:<seed>:<each|seq>:<num,num,...>"`
/// Writes a length-prefixed record per captured output to `HARVEST_SHGETI_OUT`.
#[test]
fn zzz_shgeti_worker() {
    let Ok(spec) = std::env::var("HARVEST_SHGETI") else {
        return;
    };
    let out_path = std::env::var("HARVEST_SHGETI_OUT").expect("HARVEST_SHGETI_OUT");
    let parts: Vec<&str> = spec.splitn(4, ':').collect();
    assert_eq!(parts.len(), 4, "bad HARVEST_SHGETI spec: {}", spec);
    let which = parts[0];
    let seed: usize = parts[1].parse().unwrap();
    let seq = parts[2] == "seq";
    let nums = parse_nums(parts[3]);

    let p = pair();
    let api = if which == "c" { &p.c } else { &p.r };

    let mut framed: Vec<u8> = Vec::new();
    if seq {
        let bytes = capture_stdout(|| unsafe {
            (api.rand_seed)(seed);
            for &n in &nums {
                (api.sh_geti)(n);
            }
        });
        framed.extend_from_slice(&(bytes.len() as u64).to_le_bytes());
        framed.extend_from_slice(&bytes);
    } else {
        for &n in &nums {
            let bytes = capture_stdout(|| unsafe {
                (api.rand_seed)(seed);
                (api.sh_geti)(n);
            });
            framed.extend_from_slice(&(bytes.len() as u64).to_le_bytes());
            framed.extend_from_slice(&bytes);
        }
    }
    std::fs::write(&out_path, &framed).expect("write worker output");
}

static SHGETI_COUNTER: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

fn spawn_shgeti(which: &str, seed: usize, nums: &[c_int], seq: bool) -> Vec<Vec<u8>> {
    let n = SHGETI_COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    let out_path = std::env::temp_dir().join(format!(
        "harvest_shgeti_{}_{}_{}.bin",
        std::process::id(),
        which,
        n
    ));
    let exe = std::env::current_exe().expect("current_exe");
    let list = nums
        .iter()
        .map(|x| x.to_string())
        .collect::<Vec<_>>()
        .join(",");
    let out = std::process::Command::new(exe)
        .args([SHGETI_WORKER, "--exact", "--test-threads=1"])
        .env(
            "HARVEST_SHGETI",
            format!(
                "{}:{}:{}:{}",
                which,
                seed,
                if seq { "seq" } else { "each" },
                list
            ),
        )
        .env("HARVEST_SHGETI_OUT", &out_path)
        .env_remove("HARVEST_SCENARIO")
        .output()
        .expect("spawn sh_geti worker");
    assert!(
        out.status.success(),
        "sh_geti worker ({}) failed: status={:?}\nstdout: {}\nstderr: {}",
        which,
        out.status,
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    let framed = std::fs::read(&out_path).unwrap_or_else(|e| {
        panic!(
            "worker ({}) produced no output file {}: {}\nstdout: {}",
            which,
            out_path.display(),
            e,
            String::from_utf8_lossy(&out.stdout)
        )
    });
    let _ = std::fs::remove_file(&out_path);

    let mut res = Vec::new();
    let mut i = 0usize;
    while i + 8 <= framed.len() {
        let len = u64::from_le_bytes(framed[i..i + 8].try_into().unwrap()) as usize;
        i += 8;
        assert!(i + len <= framed.len(), "truncated worker record");
        res.push(framed[i..i + len].to_vec());
        i += len;
    }
    assert_eq!(i, framed.len(), "trailing bytes in worker output");
    res
}

/// Run `sh_geti` for each `num` in BOTH libraries (each in its own subprocess,
/// each starting from `seed`) and assert the captured stdout matches
/// byte-for-byte.  Returns the (identical) outputs.
#[track_caller]
pub fn sh_geti_diff(seed: usize, nums: &[c_int]) -> Vec<Vec<u8>> {
    let c = spawn_shgeti("c", seed, nums, false);
    let r = spawn_shgeti("r", seed, nums, false);
    assert_eq!(c.len(), nums.len(), "C worker returned {} records", c.len());
    assert_eq!(r.len(), nums.len(), "Rust worker returned {} records", r.len());
    for (i, &num) in nums.iter().enumerate() {
        if c[i] != r[i] {
            let cl: Vec<String> = c[i].split(|b| *b == b'\n').map(|l| show(l)).collect();
            let rl: Vec<String> = r[i].split(|b| *b == b'\n').map(|l| show(l)).collect();
            for j in 0..cl.len().max(rl.len()) {
                let a = cl.get(j).cloned().unwrap_or_else(|| "<eof>".into());
                let b = rl.get(j).cloned().unwrap_or_else(|| "<eof>".into());
                if a != b {
                    panic!(
                        "sh_geti({}) seed={:#x}: first difference at line {}\n  C    = {:?}\n  Rust = {:?}\n  \
                         (C {} lines, Rust {} lines)",
                        num, seed, j, a, b, cl.len(), rl.len()
                    );
                }
            }
            panic!("sh_geti({}) seed={:#x}: outputs differ in length only", num, seed);
        }
    }
    c
}

/// Same, but the whole `nums` sequence runs in ONE process without reseeding
/// between calls (so the global `stbds_hash_seed` and the static `buffer`
/// carry over), and the concatenated output is compared.
#[track_caller]
pub fn sh_geti_diff_sequence(seed: usize, nums: &[c_int]) -> Vec<u8> {
    let c = spawn_shgeti("c", seed, nums, true);
    let r = spawn_shgeti("r", seed, nums, true);
    assert_eq!(c.len(), 1);
    assert_eq!(r.len(), 1);
    assert_eq!(
        show(&c[0]),
        show(&r[0]),
        "sh_geti sequence {:?} (seed {:#x}) diverged",
        nums,
        seed
    );
    c.into_iter().next().unwrap()
}
