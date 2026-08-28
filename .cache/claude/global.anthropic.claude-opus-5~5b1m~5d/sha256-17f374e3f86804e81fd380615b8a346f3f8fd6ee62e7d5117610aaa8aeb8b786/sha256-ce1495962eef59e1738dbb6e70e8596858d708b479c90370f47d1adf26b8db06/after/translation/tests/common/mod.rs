//! Shared differential-test harness.
//!
//! Loads BOTH the original C `.so` and the translated Rust `.so` through
//! `libloading` and exposes their exported symbols behind identical function
//! pointers, so every assertion goes through the real FFI boundary.

#![allow(dead_code)]
#![allow(non_snake_case)]
#![allow(non_camel_case_types)]
#![allow(non_upper_case_globals)]

use std::ffi::{c_char, c_int, c_void};
use std::path::PathBuf;

// ---------------------------------------------------------------------------
// C struct mirrors (must match c_src/src/lib.c exactly)
// ---------------------------------------------------------------------------

#[repr(C)]
#[derive(Clone, Copy)]
pub struct ArrHeader {
    pub length: usize,
    pub capacity: usize,
    pub hash_table: *mut c_void,
    pub temp: isize,
}

pub const HDR_SIZE: usize = std::mem::size_of::<ArrHeader>();

#[repr(C)]
#[derive(Clone, Copy)]
pub struct StringBlock {
    pub next: *mut StringBlock,
    pub storage: [c_char; 8],
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct StringArena {
    pub storage: *mut StringBlock,
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

pub const BUCKET_LEN: usize = 8;
pub const BUCKET_SHIFT: usize = 3;
pub const BUCKET_MASK: usize = BUCKET_LEN - 1;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct HashBucket {
    pub hash: [usize; BUCKET_LEN],
    pub index: [isize; BUCKET_LEN],
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

pub const STBDS_HM_BINARY: c_int = 0;
pub const STBDS_HM_STRING: c_int = 1;

pub const STBDS_SH_NONE: c_int = 0;
pub const STBDS_SH_DEFAULT: c_int = 1;
pub const STBDS_SH_STRDUP: c_int = 2;
pub const STBDS_SH_ARENA: c_int = 3;

// ---------------------------------------------------------------------------
// Function-pointer types
// ---------------------------------------------------------------------------

pub type FnArrGrowF = unsafe extern "C" fn(*mut c_void, usize, usize, usize) -> *mut c_void;
pub type FnArrFreeF = unsafe extern "C" fn(*mut c_void);
pub type FnRandSeed = unsafe extern "C" fn(usize);
pub type FnHashBytes = unsafe extern "C" fn(*mut c_void, usize, usize) -> usize;
pub type FnHashString = unsafe extern "C" fn(*mut c_char, usize) -> usize;
pub type FnHmFree = unsafe extern "C" fn(*mut c_void, usize);
pub type FnHmGetKeyTs =
    unsafe extern "C" fn(*mut c_void, usize, *mut c_void, usize, *mut isize, c_int) -> *mut c_void;
pub type FnHmGetKey =
    unsafe extern "C" fn(*mut c_void, usize, *mut c_void, usize, c_int) -> *mut c_void;
pub type FnHmPutDefault = unsafe extern "C" fn(*mut c_void, usize) -> *mut c_void;
pub type FnHmPutKey =
    unsafe extern "C" fn(*mut c_void, usize, *mut c_void, usize, c_int) -> *mut c_void;
pub type FnShMode = unsafe extern "C" fn(usize, c_int) -> *mut c_void;
pub type FnHmDelKey =
    unsafe extern "C" fn(*mut c_void, usize, *mut c_void, usize, usize, c_int) -> *mut c_void;
pub type FnStrAlloc = unsafe extern "C" fn(*mut StringArena, *mut c_char) -> *mut c_char;
pub type FnStrReset = unsafe extern "C" fn(*mut StringArena);
pub type FnStrKey = unsafe extern "C" fn(c_int) -> *mut c_char;
pub type FnArrDel = unsafe extern "C" fn(c_int);

pub struct Lib {
    pub name: &'static str,
    pub path: PathBuf,
    pub arrgrowf: FnArrGrowF,
    pub arrfreef: FnArrFreeF,
    pub rand_seed: FnRandSeed,
    pub hash_bytes: FnHashBytes,
    pub hash_string: FnHashString,
    pub hmfree_func: FnHmFree,
    pub hmget_key_ts: FnHmGetKeyTs,
    pub hmget_key: FnHmGetKey,
    pub hmput_default: FnHmPutDefault,
    pub hmput_key: FnHmPutKey,
    pub shmode_func: FnShMode,
    pub hmdel_key: FnHmDelKey,
    pub stralloc: FnStrAlloc,
    pub strreset: FnStrReset,
    pub strkey: FnStrKey,
    pub arr_del: FnArrDel,
}

// ---------------------------------------------------------------------------
// .so discovery
// ---------------------------------------------------------------------------

/// `.../target/<profile>/deps/<testbin>` -> `.../target/<profile>`
fn profile_dir() -> PathBuf {
    let exe = std::env::current_exe().expect("current_exe");
    exe.parent()
        .and_then(|p| p.parent())
        .expect("profile dir")
        .to_path_buf()
}

pub fn rust_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("RUST_DYLIB") {
        return PathBuf::from(p);
    }
    let p = profile_dir().join("libarr_del_lib.so");
    assert!(
        p.exists(),
        "Rust cdylib not found at {p:?} — run `cargo build --release` first"
    );
    // `cargo test` does NOT rebuild a cdylib-only lib target, so the .so can be
    // stale while the test binaries are fresh.  Refuse to run in that case
    // instead of silently verifying an old library.
    let so_mtime = std::fs::metadata(&p).and_then(|m| m.modified()).ok();
    for src in ["src/lib.rs", "Cargo.toml"] {
        if let (Some(so), Ok(m)) = (
            so_mtime,
            std::fs::metadata(src).and_then(|m| m.modified()),
        ) {
            assert!(
                so >= m,
                "STALE Rust cdylib: {p:?} is older than {src}.\n\
                 `cargo test` does not rebuild a cdylib-only lib target — run\n\
                 `cargo build --release && cargo test --release` (or ./run_all.sh)."
            );
        }
    }
    p
}

pub fn c_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("C_DYLIB") {
        return PathBuf::from(p);
    }
    // tests run with CWD = the crate root (translation/)
    let dir = PathBuf::from("../c_src/build");
    let mut found: Option<PathBuf> = None;
    if let Ok(rd) = std::fs::read_dir(&dir) {
        for e in rd.flatten() {
            let p = e.path();
            if p.extension().map(|x| x == "so").unwrap_or(false) {
                found = Some(p);
                break;
            }
        }
    }
    found.unwrap_or_else(|| {
        panic!(
            "no C .so found in {dir:?} — build it with:\n  \
             cd c_src && mkdir -p build && cd build && \
             cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build ."
        )
    })
}

macro_rules! sym {
    ($lib:expr, $ty:ty, $name:literal) => {{
        let s: libloading::Symbol<$ty> = $lib
            .get(concat!($name, "\0").as_bytes())
            .unwrap_or_else(|e| panic!("missing symbol {}: {e}", $name));
        *s
    }};
}

fn load(name: &'static str, path: PathBuf) -> Lib {
    unsafe {
        let lib: &'static libloading::Library = Box::leak(Box::new(
            libloading::Library::new(&path)
                .unwrap_or_else(|e| panic!("dlopen {path:?} failed: {e}")),
        ));
        Lib {
            name,
            path: path.clone(),
            arrgrowf: sym!(lib, FnArrGrowF, "stbds_arrgrowf"),
            arrfreef: sym!(lib, FnArrFreeF, "stbds_arrfreef"),
            rand_seed: sym!(lib, FnRandSeed, "stbds_rand_seed"),
            hash_bytes: sym!(lib, FnHashBytes, "stbds_hash_bytes"),
            hash_string: sym!(lib, FnHashString, "stbds_hash_string"),
            hmfree_func: sym!(lib, FnHmFree, "stbds_hmfree_func"),
            hmget_key_ts: sym!(lib, FnHmGetKeyTs, "stbds_hmget_key_ts"),
            hmget_key: sym!(lib, FnHmGetKey, "stbds_hmget_key"),
            hmput_default: sym!(lib, FnHmPutDefault, "stbds_hmput_default"),
            hmput_key: sym!(lib, FnHmPutKey, "stbds_hmput_key"),
            shmode_func: sym!(lib, FnShMode, "stbds_shmode_func"),
            hmdel_key: sym!(lib, FnHmDelKey, "stbds_hmdel_key"),
            stralloc: sym!(lib, FnStrAlloc, "stbds_stralloc"),
            strreset: sym!(lib, FnStrReset, "stbds_strreset"),
            strkey: sym!(lib, FnStrKey, "strkey"),
            arr_del: sym!(lib, FnArrDel, "arr_del"),
        }
    }
}

pub struct Pair {
    pub c: Lib,
    pub r: Lib,
}

pub fn libs() -> &'static Pair {
    use std::sync::OnceLock;
    static P: OnceLock<Pair> = OnceLock::new();
    P.get_or_init(|| Pair {
        c: load("C", c_so_path()),
        r: load("RUST", rust_so_path()),
    })
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (splitmix64) — fixed seed => reproducible runs
// ---------------------------------------------------------------------------

pub struct Rng(pub u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Rng(seed)
    }
    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E3779B97F4A7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D049BB133111EB);
        z ^ (z >> 31)
    }
    pub fn next_u32(&mut self) -> u32 {
        (self.next_u64() >> 32) as u32
    }
    pub fn below(&mut self, n: usize) -> usize {
        if n == 0 {
            0
        } else {
            (self.next_u64() % (n as u64)) as usize
        }
    }
    pub fn bytes(&mut self, n: usize) -> Vec<u8> {
        (0..n).map(|_| (self.next_u64() >> 24) as u8).collect()
    }
    /// NUL-terminated printable-ASCII C string of `n` payload bytes.
    pub fn cstring(&mut self, n: usize) -> Vec<u8> {
        let mut v: Vec<u8> = (0..n)
            .map(|_| 0x21u8 + ((self.next_u64() >> 24) as u8 % 0x5E))
            .collect();
        v.push(0);
        v
    }
    /// NUL-terminated string that may contain bytes 0x80..=0xFF.
    pub fn cstring_high(&mut self, n: usize) -> Vec<u8> {
        let mut v: Vec<u8> = (0..n)
            .map(|_| {
                let b = (self.next_u64() >> 24) as u8;
                if b == 0 { 0x80 } else { b }
            })
            .collect();
        v.push(0);
        v
    }
}

// ---------------------------------------------------------------------------
// Snapshots
// ---------------------------------------------------------------------------

#[derive(PartialEq, Eq, Debug, Clone)]
pub struct ArrSnap {
    pub null: bool,
    pub length: usize,
    pub capacity: usize,
    pub temp: isize,
    pub has_table: bool,
    pub payload: Vec<u8>,
}

/// Snapshot of a plain `stbds_arr*` array given the *element* pointer.
pub unsafe fn arr_snap(a: *mut c_void, elemsize: usize, payload_elems: usize) -> ArrSnap {
    unsafe {
        if a.is_null() {
            return ArrSnap {
                null: true,
                length: 0,
                capacity: 0,
                temp: 0,
                has_table: false,
                payload: Vec::new(),
            };
        }
        let h = (a as *mut u8).sub(HDR_SIZE) as *mut ArrHeader;
        let n = payload_elems * elemsize;
        let mut payload = vec![0u8; n];
        std::ptr::copy_nonoverlapping(a as *const u8, payload.as_mut_ptr(), n);
        ArrSnap {
            null: false,
            length: (*h).length,
            capacity: (*h).capacity,
            temp: (*h).temp,
            has_table: !(*h).hash_table.is_null(),
            payload,
        }
    }
}

/// How the key of an element should be rendered for comparison.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum KeyRepr {
    /// Raw `keysize` bytes.
    Raw,
    /// First 8 bytes are a `char*`; render the pointed-to C string instead.
    CStr,
}

#[derive(PartialEq, Eq, Debug, Clone)]
pub struct MapSnap {
    pub null: bool,
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
    pub arena_chain: usize,
    pub buckets: Vec<(usize, isize)>,
    pub elems: Vec<Vec<u8>>,
}

pub unsafe fn cstr_bytes(p: *const c_char) -> Option<Vec<u8>> {
    unsafe {
        if p.is_null() {
            None
        } else {
            let mut v = Vec::new();
            let mut q = p as *const u8;
            let mut guard = 0usize;
            while *q != 0 {
                v.push(*q);
                q = q.add(1);
                guard += 1;
                assert!(guard < 1 << 20, "unterminated C string");
            }
            Some(v)
        }
    }
}

unsafe fn elem_repr(
    base: *mut c_void,
    elemsize: usize,
    keysize: usize,
    i: usize,
    kr: KeyRepr,
) -> Vec<u8> {
    unsafe {
        let e = (base as *mut u8).add(elemsize * i);
        let mut out = Vec::new();
        match kr {
            KeyRepr::Raw => {
                out.extend_from_slice(std::slice::from_raw_parts(e, keysize.min(elemsize)));
            }
            KeyRepr::CStr => {
                let p = *(e as *const *const c_char);
                match cstr_bytes(p) {
                    None => out.push(0xAA),
                    Some(s) => {
                        out.push(0x55);
                        out.extend_from_slice(&s);
                        out.push(0);
                    }
                }
            }
        }
        // the rest of the element (the "value" region) — the tests always
        // initialise it explicitly, so every byte is defined.
        if elemsize > keysize {
            out.extend_from_slice(std::slice::from_raw_parts(e.add(keysize), elemsize - keysize));
        }
        out
    }
}

/// Snapshot of a hash map given the *hash pointer* `t` (what the `hm*` calls
/// return / what `t` is inside the `stbds_hm*` macros).
pub unsafe fn map_snap(t: *mut c_void, elemsize: usize, keysize: usize, kr: KeyRepr) -> MapSnap {
    unsafe {
        if t.is_null() {
            return MapSnap {
                null: true,
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
                arena_chain: 0,
                buckets: Vec::new(),
                elems: Vec::new(),
            };
        }
        let base = (t as *mut u8).sub(elemsize) as *mut c_void;
        let h = (base as *mut u8).sub(HDR_SIZE) as *mut ArrHeader;
        let mut s = MapSnap {
            null: false,
            length: (*h).length,
            capacity: (*h).capacity,
            temp: (*h).temp,
            has_table: !(*h).hash_table.is_null(),
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
            arena_chain: 0,
            buckets: Vec::new(),
            elems: Vec::new(),
        };
        if s.has_table {
            let ti = (*h).hash_table as *mut HashIndex;
            s.slot_count = (*ti).slot_count;
            s.used_count = (*ti).used_count;
            s.used_count_threshold = (*ti).used_count_threshold;
            s.used_count_shrink_threshold = (*ti).used_count_shrink_threshold;
            s.tombstone_count = (*ti).tombstone_count;
            s.tombstone_count_threshold = (*ti).tombstone_count_threshold;
            s.seed = (*ti).seed;
            s.slot_count_log2 = (*ti).slot_count_log2;
            s.arena_remaining = (*ti).string.remaining;
            s.arena_block = (*ti).string.block;
            s.arena_mode = (*ti).string.mode;
            let mut b = (*ti).string.storage;
            while !b.is_null() {
                s.arena_chain += 1;
                b = (*b).next;
                assert!(s.arena_chain < 1 << 20);
            }
            let nb = s.slot_count >> BUCKET_SHIFT;
            for i in 0..nb {
                let bk = (*ti).storage.add(i);
                for j in 0..BUCKET_LEN {
                    s.buckets.push(((*bk).hash[j], (*bk).index[j]));
                }
            }
        }
        for i in 0..s.length {
            s.elems.push(elem_repr(base, elemsize, keysize, i, kr));
        }
        s
    }
}

/// `stbds_temp_key(t-1)` — `table->temp_key`, rendered as its C-string
/// contents.  NOTE: `stbds_make_hash_index` leaves this field **uninitialised**,
/// so it may only be read right after a `stbds_hmput_key` on a string-mode
/// table (which is the only thing that writes it).  It is therefore *not* part
/// of `map_snap`.
pub unsafe fn map_temp_key(t: *mut c_void, elemsize: usize) -> Option<Vec<u8>> {
    unsafe {
        let base = (t as *mut u8).sub(elemsize);
        let h = base.sub(HDR_SIZE) as *mut ArrHeader;
        let ti = (*h).hash_table as *mut HashIndex;
        if ti.is_null() {
            return None;
        }
        cstr_bytes((*ti).temp_key)
    }
}

/// Raw `table->temp_key` pointer (for identity comparisons against a key
/// buffer the caller owns).
pub unsafe fn map_temp_key_ptr(t: *mut c_void, elemsize: usize) -> *mut c_char {
    unsafe {
        let base = (t as *mut u8).sub(elemsize);
        let h = base.sub(HDR_SIZE) as *mut ArrHeader;
        let ti = (*h).hash_table as *mut HashIndex;
        if ti.is_null() {
            return std::ptr::null_mut();
        }
        (*ti).temp_key
    }
}

/// `stbds_temp(t-1)` — the index the `hm*` macros use after a put/get.
pub unsafe fn map_temp(t: *mut c_void, elemsize: usize) -> isize {
    unsafe {
        let base = (t as *mut u8).sub(elemsize);
        let h = base.sub(HDR_SIZE) as *mut ArrHeader;
        (*h).temp
    }
}

pub unsafe fn map_len(t: *mut c_void, elemsize: usize) -> isize {
    unsafe {
        if t.is_null() {
            return 0;
        }
        let base = (t as *mut u8).sub(elemsize);
        let h = base.sub(HDR_SIZE) as *mut ArrHeader;
        (*h).length as isize - 1
    }
}

/// Fill the non-key part of `t[idx]` (i.e. of the element the `hm*` macros
/// address as `(t)[stbds_temp((t)-1)]`) with deterministic bytes, exactly like
/// `hmput(t,k,v)` writes `t[i].value = v`.
pub unsafe fn write_value(t: *mut c_void, elemsize: usize, keysize: usize, idx: isize, tag: u64) {
    unsafe {
        assert!(idx >= -1, "element index out of range");
        let e = (t as *mut u8).offset(elemsize as isize * idx);
        let mut x = tag;
        for k in keysize..elemsize {
            x = x
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            *e.add(k) = (x >> 33) as u8;
        }
    }
}

/// `t[stbds_temp((t)-1)].value = tag` — the exact write the `stbds_hmput`
/// macro performs after `stbds_hmput_key` returns.
pub unsafe fn put_value(t: *mut c_void, elemsize: usize, keysize: usize, tag: u64) -> isize {
    unsafe {
        let idx = map_temp(t, elemsize);
        write_value(t, elemsize, keysize, idx, tag);
        idx
    }
}

#[derive(PartialEq, Eq, Debug, Clone)]
pub struct ArenaSnap {
    pub remaining: usize,
    pub block: u8,
    pub mode: u8,
    pub chain: usize,
    pub storage_null: bool,
}

pub unsafe fn arena_snap(a: *const StringArena) -> ArenaSnap {
    unsafe {
        let mut chain = 0usize;
        let mut b = (*a).storage;
        while !b.is_null() {
            chain += 1;
            b = (*b).next;
            assert!(chain < 1 << 20);
        }
        ArenaSnap {
            remaining: (*a).remaining,
            block: (*a).block,
            mode: (*a).mode,
            chain,
            storage_null: (*a).storage.is_null(),
        }
    }
}

// ---------------------------------------------------------------------------
// DualMap — drives the C and the Rust hash map in lockstep and compares the
// complete observable state after every single operation.
// ---------------------------------------------------------------------------

/// Stable storage for key buffers (string-mode maps with `STBDS_SH_DEFAULT`
/// store the *caller's* pointer, so the buffers must never move).
pub struct KeyArena {
    store: Vec<Box<[u8]>>,
}

impl KeyArena {
    pub fn new() -> Self {
        KeyArena { store: Vec::new() }
    }
    /// Interns `bytes` and returns a stable pointer to it.
    pub fn add(&mut self, bytes: &[u8]) -> *mut c_void {
        let b: Box<[u8]> = bytes.to_vec().into_boxed_slice();
        let p = b.as_ptr() as *mut c_void;
        self.store.push(b);
        p
    }
    pub fn len(&self) -> usize {
        self.store.len()
    }
}

pub struct DualMap {
    pub tc: *mut c_void,
    pub tr: *mut c_void,
    pub elemsize: usize,
    pub keysize: usize,
    pub kr: KeyRepr,
    pub ops: usize,
}

impl DualMap {
    pub fn null(elemsize: usize, keysize: usize, kr: KeyRepr) -> Self {
        DualMap {
            tc: std::ptr::null_mut(),
            tr: std::ptr::null_mut(),
            elemsize,
            keysize,
            kr,
            ops: 0,
        }
    }

    /// `stbds_sh_new_arena` / `stbds_sh_new_strdup` and friends.
    pub unsafe fn shmode(elemsize: usize, keysize: usize, kr: KeyRepr, mode: c_int) -> Self {
        unsafe {
            let p = libs();
            let m = DualMap {
                tc: (p.c.shmode_func)(elemsize, mode),
                tr: (p.r.shmode_func)(elemsize, mode),
                elemsize,
                keysize,
                kr,
                ops: 0,
            };
            m.check("after shmode_func");
            m
        }
    }

    /// `stbds_hmdefault` — `stbds_hmput_default`.
    pub unsafe fn put_default(&mut self) {
        unsafe {
            let p = libs();
            self.tc = (p.c.hmput_default)(self.tc, self.elemsize);
            self.tr = (p.r.hmput_default)(self.tr, self.elemsize);
            self.ops += 1;
            self.check("after hmput_default");
        }
    }

    pub unsafe fn snap_c(&self) -> MapSnap {
        unsafe { map_snap(self.tc, self.elemsize, self.keysize, self.kr) }
    }
    pub unsafe fn snap_r(&self) -> MapSnap {
        unsafe { map_snap(self.tr, self.elemsize, self.keysize, self.kr) }
    }

    pub fn check(&self, what: &str) {
        let sc = unsafe { self.snap_c() };
        let sr = unsafe { self.snap_r() };
        assert_eq!(
            sc, sr,
            "map state diverged {what} (op #{}, elemsize={}, keysize={})",
            self.ops, self.elemsize, self.keysize
        );
    }

    /// `stbds_hmput(t,k,v)` / `stbds_shput(t,k,v)`
    pub unsafe fn put(&mut self, key: *mut c_void, mode: c_int, tag: u64) -> isize {
        unsafe {
            let p = libs();
            self.ops += 1;
            self.tc = (p.c.hmput_key)(self.tc, self.elemsize, key, self.keysize, mode);
            self.tr = (p.r.hmput_key)(self.tr, self.elemsize, key, self.keysize, mode);
            let ic = map_temp(self.tc, self.elemsize);
            let ir = map_temp(self.tr, self.elemsize);
            assert_eq!(
                ic, ir,
                "hmput_key temp diverged (mode={mode}, op #{})",
                self.ops
            );
            write_value(self.tc, self.elemsize, self.keysize, ic, tag);
            write_value(self.tr, self.elemsize, self.keysize, ir, tag);
            self.check("after hmput_key");
            ic
        }
    }

    /// `stbds_hmgeti(t,k)` / `stbds_shgeti(t,k)`
    pub unsafe fn get(&mut self, key: *mut c_void, mode: c_int) -> isize {
        unsafe {
            let p = libs();
            self.ops += 1;
            self.tc = (p.c.hmget_key)(self.tc, self.elemsize, key, self.keysize, mode);
            self.tr = (p.r.hmget_key)(self.tr, self.elemsize, key, self.keysize, mode);
            let ic = map_temp(self.tc, self.elemsize);
            let ir = map_temp(self.tr, self.elemsize);
            assert_eq!(
                ic, ir,
                "hmget_key temp diverged (mode={mode}, op #{})",
                self.ops
            );
            self.check("after hmget_key");
            ic
        }
    }

    /// `stbds_hmgeti_ts(t,k,temp)`
    pub unsafe fn get_ts(&mut self, key: *mut c_void, mode: c_int) -> isize {
        unsafe {
            let p = libs();
            self.ops += 1;
            let mut oc: isize = 0x5555_5555;
            let mut or_: isize = 0x5555_5555;
            self.tc = (p.c.hmget_key_ts)(
                self.tc,
                self.elemsize,
                key,
                self.keysize,
                &raw mut oc,
                mode,
            );
            self.tr = (p.r.hmget_key_ts)(
                self.tr,
                self.elemsize,
                key,
                self.keysize,
                &raw mut or_,
                mode,
            );
            assert_eq!(
                oc, or_,
                "hmget_key_ts *temp diverged (mode={mode}, op #{})",
                self.ops
            );
            self.check("after hmget_key_ts");
            oc
        }
    }

    /// `stbds_hmdel(t,k)` / `stbds_shdel(t,k)`; returns the macro's result
    /// (`t ? stbds_temp(t-1) : 0`).
    pub unsafe fn del(&mut self, key: *mut c_void, mode: c_int) -> isize {
        unsafe { self.del_off(key, 0, mode) }
    }

    pub unsafe fn del_off(&mut self, key: *mut c_void, keyoffset: usize, mode: c_int) -> isize {
        unsafe {
            let p = libs();
            self.ops += 1;
            self.tc = (p.c.hmdel_key)(self.tc, self.elemsize, key, self.keysize, keyoffset, mode);
            self.tr = (p.r.hmdel_key)(self.tr, self.elemsize, key, self.keysize, keyoffset, mode);
            assert_eq!(
                self.tc.is_null(),
                self.tr.is_null(),
                "hmdel_key null-ness diverged (op #{})",
                self.ops
            );
            let ic = if self.tc.is_null() {
                0
            } else {
                map_temp(self.tc, self.elemsize)
            };
            let ir = if self.tr.is_null() {
                0
            } else {
                map_temp(self.tr, self.elemsize)
            };
            assert_eq!(
                ic, ir,
                "hmdel_key temp diverged (mode={mode}, op #{})",
                self.ops
            );
            self.check("after hmdel_key");
            ic
        }
    }

    /// `stbds_hmfree(t)` / `stbds_shfree(t)`
    pub unsafe fn free(self) {
        unsafe {
            let p = libs();
            if !self.tc.is_null() {
                (p.c.hmfree_func)(
                    (self.tc as *mut u8).sub(self.elemsize) as *mut c_void,
                    self.elemsize,
                );
            }
            if !self.tr.is_null() {
                (p.r.hmfree_func)(
                    (self.tr as *mut u8).sub(self.elemsize) as *mut c_void,
                    self.elemsize,
                );
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Global library state is shared by all threads of a test binary
// (`stbds_hash_seed` in both .so's, and `strkey`'s static buffer), so every
// test that depends on it must serialise.
// ---------------------------------------------------------------------------

static LIB_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

pub struct LibGuard(#[allow(dead_code)] std::sync::MutexGuard<'static, ()>);

pub fn lock_libs() -> LibGuard {
    LibGuard(LIB_LOCK.lock().unwrap_or_else(|e| e.into_inner()))
}

/// Put both libraries' global hash seed into a known state so that the
/// `table->seed` values (and therefore every bucket layout) are reproducible,
/// and hold the library lock for the duration of the returned guard.
#[must_use = "hold the guard for as long as the library globals are in use"]
pub fn reset_seeds(seed: usize) -> LibGuard {
    let g = lock_libs();
    let p = libs();
    unsafe {
        (p.c.rand_seed)(seed);
        (p.r.rand_seed)(seed);
    }
    g
}
