//! Shared differential-test harness.
//!
//! Both the C `.so` and the Rust `.so` are loaded with `libloading` and driven
//! *only* through their exported C symbols, exactly as an external consumer
//! would. Nothing in the crate is called directly.
#![allow(dead_code)]

use core::ffi::{c_char, c_int, c_void};
use std::path::PathBuf;

// ---------------------------------------------------------------------------
// Deterministic PRNG (xorshift64*)
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
    pub fn next_u8(&mut self) -> u8 {
        (self.next_u64() >> 33) as u8
    }
    pub fn below(&mut self, n: usize) -> usize {
        if n == 0 {
            0
        } else {
            (self.next_u64() >> 1) as usize % n
        }
    }
    pub fn range(&mut self, lo: usize, hi_inclusive: usize) -> usize {
        lo + self.below(hi_inclusive - lo + 1)
    }
    pub fn bytes(&mut self, n: usize) -> Vec<u8> {
        (0..n).map(|_| self.next_u8()).collect()
    }
    /// Non-zero bytes (so the buffer can also be used as a NUL-terminated C string).
    pub fn nz_bytes(&mut self, n: usize) -> Vec<u8> {
        (0..n)
            .map(|_| {
                let b = self.next_u8();
                if b == 0 {
                    1
                } else {
                    b
                }
            })
            .collect()
    }
}

// ---------------------------------------------------------------------------
// C ABI mirror of the private types in c_src/src/lib.c
// ---------------------------------------------------------------------------

pub const BUCKET_LENGTH: usize = 8;
pub const BUCKET_SHIFT: usize = 3;
pub const BUCKET_MASK: usize = BUCKET_LENGTH - 1;
pub const HEADER_SIZE: usize = core::mem::size_of::<ArrayHeader>();

pub const HM_BINARY: c_int = 0;
pub const HM_STRING: c_int = 1;

pub const SH_NONE: c_int = 0;
pub const SH_DEFAULT: c_int = 1;
pub const SH_STRDUP: c_int = 2;
pub const SH_ARENA: c_int = 3;

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
pub struct HashBucket {
    pub hash: [usize; BUCKET_LENGTH],
    pub index: [isize; BUCKET_LENGTH],
}

/// `struct stbds_string_block { struct stbds_string_block *next; char storage[8]; }`
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct StringBlock {
    pub next: *mut StringBlock,
    pub storage: [c_char; 8],
}

/// `sizeof(stbds_string_block) - 8`, i.e. the offset of the flexible `storage`.
pub const STRING_BLOCK_HDR: usize = core::mem::size_of::<StringBlock>() - 8;

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct StringArena {
    pub storage: *mut StringBlock,
    pub remaining: usize,
    pub block: u8,
    pub mode: u8,
}

impl StringArena {
    pub fn zeroed() -> StringArena {
        StringArena {
            storage: core::ptr::null_mut(),
            remaining: 0,
            block: 0,
            mode: 0,
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
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
// Library loading
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
pub type FnShGeti = unsafe extern "C" fn(c_int);

pub struct Lib {
    pub name: &'static str,
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
    pub sh_geti: FnShGeti,
}

pub const ALL_SYMBOLS: &[&str] = &[
    "stbds_arrgrowf",
    "stbds_arrfreef",
    "stbds_rand_seed",
    "stbds_hash_string",
    "stbds_hash_bytes",
    "stbds_hmfree_func",
    "stbds_hmget_key_ts",
    "stbds_hmget_key",
    "stbds_hmput_default",
    "stbds_hmput_key",
    "stbds_shmode_func",
    "stbds_hmdel_key",
    "stbds_stralloc",
    "stbds_strreset",
    "strkey",
    "sh_geti",
];

pub fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

pub fn c_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("DIFF_C_SO") {
        return PathBuf::from(p);
    }
    manifest_dir().join("c_src/build/libtranslated_rust.so")
}

pub fn rust_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("DIFF_RUST_SO") {
        return PathBuf::from(p);
    }
    // current_exe() == <target>/<profile>/deps/<testbin>
    if let Ok(exe) = std::env::current_exe() {
        if let Some(profile_dir) = exe.parent().and_then(|d| d.parent()) {
            let p = profile_dir.join("libsh_geti_lib.so");
            if p.exists() {
                return p;
            }
        }
    }
    for prof in ["release", "debug"] {
        let p = manifest_dir().join("target").join(prof).join("libsh_geti_lib.so");
        if p.exists() {
            return p;
        }
    }
    manifest_dir().join("target/release/libsh_geti_lib.so")
}

macro_rules! sym {
    ($lib:expr, $name:literal, $ty:ty) => {{
        let s: libloading::Symbol<'static, $ty> = $lib
            .get(concat!($name, "\0").as_bytes())
            .unwrap_or_else(|e| panic!("missing symbol {}: {}", $name, e));
        *s
    }};
}

impl Lib {
    unsafe fn load(name: &'static str, path: &std::path::Path) -> Lib {
        let lib: &'static libloading::Library = Box::leak(Box::new(
            libloading::Library::new(path)
                .unwrap_or_else(|e| panic!("failed to dlopen {}: {}", path.display(), e)),
        ));
        Lib {
            name,
            arrgrowf: sym!(lib, "stbds_arrgrowf", FnArrGrowf),
            arrfreef: sym!(lib, "stbds_arrfreef", FnArrFreef),
            rand_seed: sym!(lib, "stbds_rand_seed", FnRandSeed),
            hash_string: sym!(lib, "stbds_hash_string", FnHashString),
            hash_bytes: sym!(lib, "stbds_hash_bytes", FnHashBytes),
            hmfree_func: sym!(lib, "stbds_hmfree_func", FnHmFree),
            hmget_key_ts: sym!(lib, "stbds_hmget_key_ts", FnHmGetKeyTs),
            hmget_key: sym!(lib, "stbds_hmget_key", FnHmGetKey),
            hmput_default: sym!(lib, "stbds_hmput_default", FnHmPutDefault),
            hmput_key: sym!(lib, "stbds_hmput_key", FnHmPutKey),
            shmode_func: sym!(lib, "stbds_shmode_func", FnShModeFunc),
            hmdel_key: sym!(lib, "stbds_hmdel_key", FnHmDelKey),
            stralloc: sym!(lib, "stbds_stralloc", FnStrAlloc),
            strreset: sym!(lib, "stbds_strreset", FnStrReset),
            strkey: sym!(lib, "strkey", FnStrKey),
            sh_geti: sym!(lib, "sh_geti", FnShGeti),
        }
    }

    pub fn c() -> Lib {
        unsafe { Lib::load("C", &c_so_path()) }
    }

    pub fn rust() -> Lib {
        unsafe { Lib::load("Rust", &rust_so_path()) }
    }

    pub fn pick(which: &str) -> Lib {
        match which {
            "c" => Lib::c(),
            "rust" => Lib::rust(),
            other => panic!("unknown lib selector {other:?}"),
        }
    }
}

/// Both libraries, loaded once per test process.
pub fn both() -> (Lib, Lib) {
    (Lib::c(), Lib::rust())
}

// ---------------------------------------------------------------------------
// Serialisation.
//
// `dlopen` of the same path returns the same handle, so every `Lib::c()` in a
// process shares ONE copy of the C library's mutable globals (`stbds_hash_seed`,
// `buffer`). Likewise for the Rust `.so`. libtest runs #[test] fns on parallel
// threads, so every test that touches that global state must take this lock.
// ---------------------------------------------------------------------------

static SERIAL: std::sync::Mutex<()> = std::sync::Mutex::new(());

pub fn serial() -> std::sync::MutexGuard<'static, ()> {
    SERIAL.lock().unwrap_or_else(|e| e.into_inner())
}

// ---------------------------------------------------------------------------
// Stable key storage: the SAME pointer is handed to both libraries, so that
// `STBDS_SH_DEFAULT` / `memcpy`-of-pointer element bytes are comparable.
// ---------------------------------------------------------------------------

pub struct Keys {
    bufs: Vec<Box<[u8]>>,
}

impl Keys {
    pub fn new() -> Keys {
        Keys { bufs: Vec::new() }
    }
    /// Store `bytes` verbatim (caller supplies any NUL terminator).
    pub fn raw(&mut self, bytes: &[u8]) -> *mut c_void {
        let mut b: Box<[u8]> = bytes.to_vec().into_boxed_slice();
        let p = b.as_mut_ptr() as *mut c_void;
        self.bufs.push(b);
        p
    }
    /// Store `bytes` plus a NUL terminator; the returned pointer is a valid `char*`.
    pub fn cstr(&mut self, bytes: &[u8]) -> *mut c_void {
        let mut v = bytes.to_vec();
        v.push(0);
        self.raw(&v)
    }
    pub fn len(&self) -> usize {
        self.bufs.len()
    }
}

// ---------------------------------------------------------------------------
// Header / element accessors (hash pointer == raw array base + elemsize)
// ---------------------------------------------------------------------------

/// `stbds_header(t)` for a raw array pointer.
pub unsafe fn hdr_of_arr(arr: *mut c_void) -> *mut ArrayHeader {
    (arr as *mut u8).wrapping_sub(HEADER_SIZE) as *mut ArrayHeader
}

/// `stbds_header((t)-1)` for a hash pointer.
pub unsafe fn hdr_of_map(t: *mut c_void, elemsize: usize) -> *mut ArrayHeader {
    hdr_of_arr((t as *mut u8).wrapping_sub(elemsize) as *mut c_void)
}

pub unsafe fn map_temp(t: *mut c_void, elemsize: usize) -> isize {
    (*hdr_of_map(t, elemsize)).temp
}

pub unsafe fn map_table(t: *mut c_void, elemsize: usize) -> *mut HashIndex {
    (*hdr_of_map(t, elemsize)).hash_table as *mut HashIndex
}

/// Address of hash-space element `idx` (`idx == -1` is the default entry).
pub unsafe fn elem(t: *mut c_void, elemsize: usize, idx: isize) -> *mut u8 {
    (t as *mut u8).wrapping_offset((elemsize as isize).wrapping_mul(idx))
}

pub unsafe fn cstr_bytes(p: *const c_char) -> Option<Vec<u8>> {
    if p.is_null() {
        return None;
    }
    let mut v = Vec::new();
    let mut i = 0usize;
    loop {
        let b = *(p.add(i) as *const u8);
        if b == 0 {
            break;
        }
        v.push(b);
        i += 1;
        if i > 1 << 20 {
            panic!("unterminated string");
        }
    }
    Some(v)
}

// ---------------------------------------------------------------------------
// Observable snapshot of a map
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ElemCmp {
    /// Element bytes are directly comparable (binary keys, or `SH_DEFAULT` /
    /// `SH_NONE` where the stored pointer is the caller's own).
    Raw,
    /// The first 8 bytes of each element are a library-owned `char*`
    /// (`SH_STRDUP` / `SH_ARENA`): compare the pointed-to string instead.
    StrKey,
}

#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct MapSnap {
    pub null: bool,
    pub length: usize,
    pub capacity: usize,
    pub temp: isize,
    pub has_table: bool,
    pub slot_count: usize,
    pub used_count: usize,
    pub uc_threshold: usize,
    pub uc_shrink_threshold: usize,
    pub tomb_count: usize,
    pub tomb_threshold: usize,
    pub seed: usize,
    pub slot_count_log2: usize,
    pub str_remaining: usize,
    pub str_block: u8,
    pub str_mode: u8,
    pub str_has_storage: bool,
    pub buckets: Vec<([usize; BUCKET_LENGTH], [isize; BUCKET_LENGTH])>,
    /// Raw bytes of each element of the *raw* array (index 0 == default entry).
    pub elems: Vec<Vec<u8>>,
    /// For `ElemCmp::StrKey`, the dereferenced key of each raw element.
    pub keys: Vec<Option<Vec<u8>>>,
}

pub unsafe fn snapshot(t: *mut c_void, elemsize: usize, cmp: ElemCmp) -> MapSnap {
    let mut s = MapSnap::default();
    if t.is_null() {
        s.null = true;
        return s;
    }
    let h = hdr_of_map(t, elemsize);
    s.length = (*h).length;
    s.capacity = (*h).capacity;
    s.temp = (*h).temp;
    let table = (*h).hash_table as *mut HashIndex;
    s.has_table = !table.is_null();
    if s.has_table {
        s.slot_count = (*table).slot_count;
        s.used_count = (*table).used_count;
        s.uc_threshold = (*table).used_count_threshold;
        s.uc_shrink_threshold = (*table).used_count_shrink_threshold;
        s.tomb_count = (*table).tombstone_count;
        s.tomb_threshold = (*table).tombstone_count_threshold;
        s.seed = (*table).seed;
        s.slot_count_log2 = (*table).slot_count_log2;
        s.str_remaining = (*table).string.remaining;
        s.str_block = (*table).string.block;
        s.str_mode = (*table).string.mode;
        s.str_has_storage = !(*table).string.storage.is_null();
        // NOTE: `temp_key` (the first field of stbds_hash_index) is deliberately
        // NOT snapshotted. `stbds_make_hash_index` never initialises it, so in
        // binary mode it is whatever `realloc` handed back — uninitialised, and
        // therefore different between the two independent heaps. It is only
        // meaningful right after a `mode >= STBDS_HM_STRING` put, where it is
        // compared explicitly by `Pair::temp_key`.
        let nb = (*table).slot_count >> BUCKET_SHIFT;
        for i in 0..nb {
            let b = (*table).storage.add(i);
            s.buckets.push(((*b).hash, (*b).index));
        }
    }
    // Raw-array elements 0 .. length
    let arr = (t as *mut u8).wrapping_sub(elemsize);
    for i in 0..s.length {
        let e = arr.wrapping_add(elemsize.wrapping_mul(i));
        match cmp {
            ElemCmp::Raw => {
                s.elems
                    .push(core::slice::from_raw_parts(e, elemsize).to_vec());
                s.keys.push(None);
            }
            ElemCmp::StrKey => {
                assert!(elemsize >= 8, "StrKey needs elemsize >= 8");
                let kp = *(e as *mut *mut c_char);
                s.keys.push(cstr_bytes(kp));
                s.elems
                    .push(core::slice::from_raw_parts(e.add(8), elemsize - 8).to_vec());
            }
        }
    }
    s
}

/// Compact field-by-field diff of two snapshots (the full `{:#?}` dump of a
/// 64-slot table is unreadable).
pub fn diff_snaps(c: &MapSnap, r: &MapSnap) -> String {
    let mut out = String::new();
    macro_rules! f {
        ($($n:ident),*) => {$(
            if c.$n != r.$n {
                out.push_str(&format!("  {:<24} C={:?}  Rust={:?}\n", stringify!($n), c.$n, r.$n));
            }
        )*};
    }
    f!(
        null, length, capacity, temp, has_table, slot_count, used_count, uc_threshold,
        uc_shrink_threshold, tomb_count, tomb_threshold, seed, slot_count_log2, str_remaining,
        str_block, str_mode, str_has_storage
    );
    if c.buckets.len() != r.buckets.len() {
        out.push_str(&format!(
            "  buckets.len               C={}  Rust={}\n",
            c.buckets.len(),
            r.buckets.len()
        ));
    }
    for (i, (cb, rb)) in c.buckets.iter().zip(r.buckets.iter()).enumerate() {
        for j in 0..BUCKET_LENGTH {
            if cb.0[j] != rb.0[j] {
                out.push_str(&format!(
                    "  bucket[{i}].hash[{j}]        C={:#018x}  Rust={:#018x}\n",
                    cb.0[j], rb.0[j]
                ));
            }
            if cb.1[j] != rb.1[j] {
                out.push_str(&format!(
                    "  bucket[{i}].index[{j}]       C={}  Rust={}\n",
                    cb.1[j], rb.1[j]
                ));
            }
        }
    }
    if c.elems.len() != r.elems.len() {
        out.push_str(&format!(
            "  elems.len                 C={}  Rust={}\n",
            c.elems.len(),
            r.elems.len()
        ));
    }
    for (i, (ce, re)) in c.elems.iter().zip(r.elems.iter()).enumerate() {
        if ce != re {
            out.push_str(&format!(
                "  elems[{i}]                 C={:02x?}\n                            Rust={:02x?}\n",
                ce, re
            ));
        }
    }
    for (i, (ck, rk)) in c.keys.iter().zip(r.keys.iter()).enumerate() {
        if ck != rk {
            out.push_str(&format!(
                "  keys[{i}]                  C={:?}  Rust={:?}\n",
                ck.as_ref().map(|v| String::from_utf8_lossy(v).into_owned()),
                rk.as_ref().map(|v| String::from_utf8_lossy(v).into_owned())
            ));
        }
    }
    if out.is_empty() {
        out.push_str("  (no field-level difference found?!)\n");
    }
    out
}

// ---------------------------------------------------------------------------
// Macro-equivalent driver operations (mirroring the stbds_* macros in lib.c)
// ---------------------------------------------------------------------------

/// One map under test, on one library.
pub struct Map<'l> {
    pub lib: &'l Lib,
    pub t: *mut c_void,
    pub elemsize: usize,
    pub keysize: usize,
    /// bytes `[value_off .. elemsize)` are the "value" region the driver writes
    pub value_off: usize,
}

impl<'l> Map<'l> {
    pub fn new(lib: &'l Lib, elemsize: usize, keysize: usize) -> Map<'l> {
        let value_off = core::cmp::min(keysize, elemsize);
        Map {
            lib,
            t: core::ptr::null_mut(),
            elemsize,
            keysize,
            value_off,
        }
    }

    /// `stbds_sh_new_arena` / `stbds_sh_new_strdup` / any `string.mode`
    pub unsafe fn shmode(&mut self, mode: c_int) {
        self.t = (self.lib.shmode_func)(self.elemsize, mode);
    }

    /// `stbds_hmdefault(t, v)` — bootstrap + write the default value
    pub unsafe fn set_default(&mut self, tag: u64) {
        self.t = (self.lib.hmput_default)(self.t, self.elemsize);
        self.write_value(-1, tag);
    }

    pub unsafe fn write_value(&self, idx: isize, tag: u64) {
        if self.value_off >= self.elemsize {
            return;
        }
        let e = elem(self.t, self.elemsize, idx);
        let mut r = Rng::new(tag ^ 0xA5A5_1234_5678_9ABC);
        for k in self.value_off..self.elemsize {
            *e.add(k) = r.next_u8();
        }
    }

    /// `stbds_hmput_key` + the macro's value store. Returns `stbds_temp`.
    pub unsafe fn put(&mut self, key: *mut c_void, mode: c_int, tag: u64) -> isize {
        self.t = (self.lib.hmput_key)(self.t, self.elemsize, key, self.keysize, mode);
        let idx = map_temp(self.t, self.elemsize);
        self.write_value(idx, tag);
        idx
    }

    /// `stbds_hmgeti` / `stbds_shgeti`
    pub unsafe fn geti(&mut self, key: *mut c_void, mode: c_int) -> isize {
        self.t = (self.lib.hmget_key)(self.t, self.elemsize, key, self.keysize, mode);
        map_temp(self.t, self.elemsize)
    }

    /// `stbds_hmgeti_ts`
    pub unsafe fn geti_ts(&mut self, key: *mut c_void, mode: c_int) -> isize {
        let mut temp: isize = 0x5555_5555;
        self.t = (self.lib.hmget_key_ts)(
            self.t,
            self.elemsize,
            key,
            self.keysize,
            &mut temp,
            mode,
        );
        temp
    }

    /// `stbds_hmdel` / `stbds_shdel`
    pub unsafe fn del(&mut self, key: *mut c_void, keyoffset: usize, mode: c_int) -> isize {
        self.t = (self.lib.hmdel_key)(
            self.t,
            self.elemsize,
            key,
            self.keysize,
            keyoffset,
            mode,
        );
        if self.t.is_null() {
            0
        } else {
            map_temp(self.t, self.elemsize)
        }
    }

    /// `stbds_hmlen`
    pub unsafe fn len(&self) -> isize {
        if self.t.is_null() {
            0
        } else {
            (*hdr_of_map(self.t, self.elemsize)).length as isize - 1
        }
    }

    /// `stbds_hmfree`
    pub unsafe fn free(&mut self) {
        if !self.t.is_null() {
            (self.lib.hmfree_func)(
                (self.t as *mut u8).wrapping_sub(self.elemsize) as *mut c_void,
                self.elemsize,
            );
        }
        self.t = core::ptr::null_mut();
    }

    pub unsafe fn snap(&self, cmp: ElemCmp) -> MapSnap {
        snapshot(self.t, self.elemsize, cmp)
    }
}

/// Run the identical script on the C map and the Rust map, comparing the full
/// snapshot after every single operation.
pub struct Pair<'l> {
    pub c: Map<'l>,
    pub r: Map<'l>,
    pub cmp: ElemCmp,
    pub label: String,
    pub step: usize,
}

impl<'l> Pair<'l> {
    pub fn new(
        libc_: &'l Lib,
        libr: &'l Lib,
        elemsize: usize,
        keysize: usize,
        cmp: ElemCmp,
        label: impl Into<String>,
    ) -> Pair<'l> {
        Pair {
            c: Map::new(libc_, elemsize, keysize),
            r: Map::new(libr, elemsize, keysize),
            cmp,
            label: label.into(),
            step: 0,
        }
    }

    pub unsafe fn seed(&mut self, s: usize) {
        (self.c.lib.rand_seed)(s);
        (self.r.lib.rand_seed)(s);
    }

    fn check(&mut self, what: &str, cv: impl std::fmt::Debug + PartialEq, rv: impl std::fmt::Debug) {
        // compared via strings so the two generic params can differ in type
        let cs = format!("{cv:?}");
        let rs = format!("{rv:?}");
        assert_eq!(
            cs, rs,
            "[{}] step {} ({}): C != Rust",
            self.label, self.step, what
        );
    }

    unsafe fn compare_state(&mut self, what: &str) {
        let cs = self.c.snap(self.cmp);
        let rs = self.r.snap(self.cmp);
        if cs != rs {
            panic!(
                "[{}] step {} ({}): map state diverged\n{}",
                self.label,
                self.step,
                what,
                diff_snaps(&cs, &rs)
            );
        }
        self.step += 1;
    }

    pub unsafe fn shmode(&mut self, mode: c_int) {
        self.c.shmode(mode);
        self.r.shmode(mode);
        self.compare_state("shmode");
    }

    pub unsafe fn set_default(&mut self, tag: u64) {
        self.c.set_default(tag);
        self.r.set_default(tag);
        self.compare_state("hmput_default");
    }

    pub unsafe fn put(&mut self, key: *mut c_void, mode: c_int, tag: u64) -> isize {
        let a = self.c.put(key, mode, tag);
        let b = self.r.put(key, mode, tag);
        self.check("put temp", a, b);
        self.compare_state("hmput_key");
        a
    }

    pub unsafe fn geti(&mut self, key: *mut c_void, mode: c_int) -> isize {
        let a = self.c.geti(key, mode);
        let b = self.r.geti(key, mode);
        self.check("geti", a, b);
        self.compare_state("hmget_key");
        a
    }

    pub unsafe fn geti_ts(&mut self, key: *mut c_void, mode: c_int) -> isize {
        let a = self.c.geti_ts(key, mode);
        let b = self.r.geti_ts(key, mode);
        self.check("geti_ts", a, b);
        self.compare_state("hmget_key_ts");
        a
    }

    pub unsafe fn del(&mut self, key: *mut c_void, keyoffset: usize, mode: c_int) -> isize {
        let a = self.c.del(key, keyoffset, mode);
        let b = self.r.del(key, keyoffset, mode);
        self.check("del", a, b);
        self.compare_state("hmdel_key");
        a
    }

    pub unsafe fn len(&mut self) -> isize {
        let a = self.c.len();
        let b = self.r.len();
        self.check("len", a, b);
        a
    }

    /// Read the value region of hash-space element `idx` from both and compare.
    pub unsafe fn value_of(&mut self, idx: isize) -> Vec<u8> {
        let n = self.c.elemsize - self.c.value_off;
        let ce = core::slice::from_raw_parts(
            elem(self.c.t, self.c.elemsize, idx).add(self.c.value_off),
            n,
        )
        .to_vec();
        let re = core::slice::from_raw_parts(
            elem(self.r.t, self.r.elemsize, idx).add(self.r.value_off),
            n,
        )
        .to_vec();
        self.check("value", &ce, &re);
        ce
    }

    /// `stbds_temp_key(t)` == `*(char **) stbds_header(t)->hash_table`, i.e.
    /// the `temp_key` field. Only defined after a `mode >= STBDS_HM_STRING`
    /// put; compares the pointed-to string, since the pointers themselves are
    /// library-owned under STRDUP/ARENA.
    pub unsafe fn temp_key(&mut self) -> Option<Vec<u8>> {
        let tc = map_table(self.c.t, self.c.elemsize);
        let tr = map_table(self.r.t, self.r.elemsize);
        assert!(!tc.is_null() && !tr.is_null(), "temp_key needs a live table");
        let a = cstr_bytes((*tc).temp_key);
        let b = cstr_bytes((*tr).temp_key);
        assert_eq!(
            a.as_ref().map(|v| String::from_utf8_lossy(v).into_owned()),
            b.as_ref().map(|v| String::from_utf8_lossy(v).into_owned()),
            "[{}] step {}: stbds_temp_key diverged",
            self.label,
            self.step
        );
        a
    }

    pub unsafe fn free(&mut self) {
        self.c.free();
        self.r.free();
        self.compare_state("hmfree_func");
    }
}

// ---------------------------------------------------------------------------
// Subprocess scenario runner.
//
// Rows whose expected C behaviour is process death (assert -> SIGABRT, or a
// SIGSEGV from a NULL dereference) cannot be observed in-process, and `sh_geti`
// writes to stdout. Both are handled by re-executing this very test binary with
// DIFF_SCENARIO / DIFF_LIB set, then comparing the child's stdout + exit
// status between the C run and the Rust run.
// ---------------------------------------------------------------------------

use std::os::unix::process::ExitStatusExt;

#[derive(Debug, PartialEq, Eq)]
pub struct Outcome {
    pub code: Option<i32>,
    pub signal: Option<i32>,
    pub stdout: Vec<u8>,
}

impl Outcome {
    pub fn describe(&self) -> String {
        format!(
            "code={:?} signal={:?} stdout={:?}",
            self.code,
            self.signal,
            String::from_utf8_lossy(&self.stdout)
        )
    }
}

pub fn spawn_scenario(scenario: &str, which: &str) -> Outcome {
    let exe = std::env::current_exe().expect("current_exe");
    let out = std::process::Command::new(exe)
        .args(["--exact", "scenario_runner", "--nocapture", "--test-threads=1"])
        .env("DIFF_SCENARIO", scenario)
        .env("DIFF_LIB", which)
        .env("DIFF_C_SO", c_so_path())
        .env("DIFF_RUST_SO", rust_so_path())
        .stderr(std::process::Stdio::null())
        .output()
        .expect("spawn scenario child");
    // strip everything up to and including the marker (libtest's preamble)
    let mark = format!("{SCENARIO_MARK}\n").into_bytes();
    let stdout = match out
        .stdout
        .windows(mark.len())
        .position(|w| w == mark.as_slice())
    {
        Some(i) => out.stdout[i + mark.len()..].to_vec(),
        None => {
            // the child died before it even got to the marker (e.g. dlopen
            // failure); keep the raw bytes so the mismatch is visible
            out.stdout.clone()
        }
    };
    Outcome {
        code: out.status.code(),
        signal: out.status.signal(),
        stdout,
    }
}

/// Run `scenario` on both libraries in separate child processes and require
/// identical stdout and identical termination status.
#[track_caller]
pub fn assert_scenario_matches(scenario: &str) -> Outcome {
    let c = spawn_scenario(scenario, "c");
    let r = spawn_scenario(scenario, "rust");
    assert_eq!(
        (c.code, c.signal),
        (r.code, r.signal),
        "scenario {scenario:?}: termination differs\n  C   : {}\n  Rust: {}",
        c.describe(),
        r.describe()
    );
    assert_eq!(
        c.stdout,
        r.stdout,
        "scenario {scenario:?}: stdout differs\n  C   : {:?}\n  Rust: {:?}",
        String::from_utf8_lossy(&c.stdout),
        String::from_utf8_lossy(&r.stdout)
    );
    c
}

/// True when the Rust `.so` under test carries Rust's debug UB checks
/// (`debug_assertions`). Those checks deliberately convert C-style undefined
/// behaviour — a NULL dereference above all — into a controlled
/// `panic!("null pointer dereference occurred")` + `abort()`, i.e. **SIGABRT
/// instead of the SIGSEGV the C code raises**.
///
/// That is a property of the debug *sanitizer*, not of the translation: the
/// shipping artifact is the release `cdylib` (`[profile.release] panic =
/// "abort"`), where the fatal rows match the C signal exactly. `cargo test`
/// picks the `.so` from the same profile as the test binary, so the test
/// crate's own `debug_assertions` is a faithful proxy.
pub fn rust_so_has_ub_checks() -> bool {
    cfg!(debug_assertions)
}

pub const SIGSEGV: i32 = 11;
pub const SIGABRT: i32 = 6;

/// Fatal-row variant of [`assert_scenario_matches`].
///
/// * stdout must be byte-identical;
/// * both must die by a signal;
/// * the C signal must be exactly `expect_c_signal`;
/// * the Rust signal must equal the C signal, except that a `debug_assertions`
///   build is additionally allowed to report `SIGABRT` where C reports
///   `SIGSEGV` (see [`rust_so_has_ub_checks`]).
#[track_caller]
pub fn assert_fatal_scenario_matches(scenario: &str, expect_c_signal: i32) -> Outcome {
    let c = spawn_scenario(scenario, "c");
    let r = spawn_scenario(scenario, "rust");
    assert_eq!(
        c.stdout,
        r.stdout,
        "scenario {scenario:?}: stdout differs\n  C   : {:?}\n  Rust: {:?}",
        String::from_utf8_lossy(&c.stdout),
        String::from_utf8_lossy(&r.stdout)
    );
    assert_eq!(
        c.signal,
        Some(expect_c_signal),
        "scenario {scenario:?}: the C library was expected to die on signal \
         {expect_c_signal}, got {}",
        c.describe()
    );
    let rust_ok = r.signal == c.signal
        || (rust_so_has_ub_checks()
            && c.signal == Some(SIGSEGV)
            && r.signal == Some(SIGABRT));
    assert!(
        rust_ok,
        "scenario {scenario:?}: termination differs\n  C   : {}\n  Rust: {}",
        c.describe(),
        r.describe()
    );
    c
}

fn nums(s: &str) -> Vec<i64> {
    s.split(':')
        .skip(1)
        .filter(|p| !p.is_empty())
        .map(|p| p.parse::<i64>().unwrap_or_else(|e| panic!("bad param {p:?}: {e}")))
        .collect()
}

/// Written to Rust's stdout (and flushed) before the scenario runs, so the
/// parent can strip libtest's own preamble and compare only the scenario's
/// output. The flush guarantees ordering against the library's libc `printf`.
pub const SCENARIO_MARK: &str = "@@SCENARIO-OUTPUT-BEGINS@@";

/// Executed inside the child process. May abort / segfault on purpose.
pub unsafe fn run_scenario(scenario: &str, lib: &Lib) {
    {
        use std::io::Write;
        let mut so = std::io::stdout();
        let _ = so.write_all(SCENARIO_MARK.as_bytes());
        let _ = so.write_all(b"\n");
        let _ = so.flush();
    }
    let head = scenario.split(':').next().unwrap();
    let p = nums(scenario);
    match head {
        // ---------------- sh_geti (stdout comparison) ----------------
        "sh_geti" => {
            if p.len() >= 2 {
                (lib.rand_seed)(p[1] as usize);
            }
            (lib.sh_geti)(p[0] as c_int);
            flush_stdout();
        }
        "sh_geti_twice" => {
            (lib.sh_geti)(p[0] as c_int);
            (lib.sh_geti)(p[0] as c_int);
            flush_stdout();
        }
        "sh_geti_seed_max" => {
            (lib.rand_seed)(usize::MAX);
            (lib.sh_geti)(p[0] as c_int);
            flush_stdout();
        }

        // ---------------- fatal rows ----------------
        // ERRORS.md row 5
        "arrfreef_null" => {
            (lib.arrfreef)(core::ptr::null_mut());
        }
        // row 6
        "hash_string_null" => {
            let h = (lib.hash_string)(core::ptr::null_mut(), 1);
            println!("{h}");
            flush_stdout();
        }
        // row 8
        "hash_bytes_null_len1" => {
            let h = (lib.hash_bytes)(core::ptr::null_mut(), 1, 1);
            println!("{h}");
            flush_stdout();
        }
        // row 9
        "hash_bytes_huge_len" => {
            let buf = [0u8; 64];
            let h = (lib.hash_bytes)(buf.as_ptr() as *mut c_void, usize::MAX, 1);
            println!("{h}");
            flush_stdout();
        }
        // row 15
        "hmget_ts_null_temp" => {
            let r = (lib.hmget_key_ts)(
                core::ptr::null_mut(),
                16,
                core::ptr::null_mut(),
                8,
                core::ptr::null_mut(),
                HM_BINARY,
            );
            println!("{}", !r.is_null());
            flush_stdout();
        }
        // row 19: string map deleted with mode == 2
        "hmdel_mode2_string_map" => {
            (lib.rand_seed)(7);
            let elemsize = 16usize;
            let mut t = (lib.shmode_func)(elemsize, SH_STRDUP);
            let mut keys: Vec<Vec<u8>> = Vec::new();
            for i in 0..6u32 {
                keys.push(format!("key_{i}\0").into_bytes());
            }
            for k in keys.iter_mut() {
                t = (lib.hmput_key)(t, elemsize, k.as_mut_ptr() as *mut c_void, 8, HM_STRING);
            }
            // delete an interior entry with mode 2 -> string hashing but binary
            // re-find => STBDS_ASSERT(slot >= 0) must fire
            t = (lib.hmdel_key)(
                t,
                elemsize,
                keys[0].as_mut_ptr() as *mut c_void,
                8,
                0,
                2,
            );
            println!("survived {}", !t.is_null());
            flush_stdout();
        }
        // row 23
        "hmput_key_null_string_key" => {
            let t = (lib.hmput_key)(core::ptr::null_mut(), 16, core::ptr::null_mut(), 8, HM_STRING);
            println!("{}", !t.is_null());
            flush_stdout();
        }
        // row 25
        "hmput_key_null_binary_key" => {
            let t = (lib.hmput_key)(core::ptr::null_mut(), 16, core::ptr::null_mut(), 8, HM_BINARY);
            println!("{}", !t.is_null());
            flush_stdout();
        }
        // row 36
        "hmdel_corrupted_index" => {
            (lib.rand_seed)(11);
            let elemsize = 16usize;
            let mut t = (lib.shmode_func)(elemsize, SH_DEFAULT);
            let mut ka: Vec<u8> = b"alpha\0".to_vec();
            let mut kb: Vec<u8> = b"bravo\0".to_vec();
            let mut kc: Vec<u8> = b"charlie\0".to_vec();
            for k in [&mut ka, &mut kb, &mut kc] {
                t = (lib.hmput_key)(t, elemsize, k.as_mut_ptr() as *mut c_void, 8, HM_STRING);
            }
            // Forge: make the tail element's key duplicate the first key, so the
            // post-memmove re-find lands on the *first* slot, whose stored index
            // is not final_index => STBDS_ASSERT(b->index[i] == final_index).
            let tail = (t as *mut u8).add(elemsize * 2) as *mut *mut c_char;
            *tail = ka.as_mut_ptr() as *mut c_char;
            t = (lib.hmdel_key)(t, elemsize, kb.as_mut_ptr() as *mut c_void, 8, 0, HM_STRING);
            println!("survived {}", !t.is_null());
            flush_stdout();
        }
        // row 40
        "stralloc_null_arena" => {
            let mut s: Vec<u8> = b"hello\0".to_vec();
            let p = (lib.stralloc)(core::ptr::null_mut(), s.as_mut_ptr() as *mut c_char);
            println!("{}", !p.is_null());
            flush_stdout();
        }
        // row 41
        "stralloc_null_str" => {
            let mut a = StringArena::zeroed();
            let p = (lib.stralloc)(&mut a, core::ptr::null_mut());
            println!("{}", !p.is_null());
            flush_stdout();
        }
        // row 42
        "stralloc_forged_remaining" => {
            let mut a = StringArena::zeroed();
            a.remaining = 1 << 30; // len <= remaining, so the alloc path is skipped
            let mut s: Vec<u8> = b"hello\0".to_vec();
            let p = (lib.stralloc)(&mut a, s.as_mut_ptr() as *mut c_char);
            println!("{}", !p.is_null());
            flush_stdout();
        }
        // row 49
        "strreset_null_arena" => {
            (lib.strreset)(core::ptr::null_mut());
            println!("survived");
            flush_stdout();
        }
        // row 26: keysize > elemsize
        "keysize_gt_elemsize" => {
            (lib.rand_seed)(3);
            let elemsize = 8usize;
            let keysize = 64usize;
            let key = [0xABu8; 64];
            let t = (lib.hmput_key)(
                core::ptr::null_mut(),
                elemsize,
                key.as_ptr() as *mut c_void,
                keysize,
                HM_BINARY,
            );
            println!("temp={}", map_temp(t, elemsize));
            flush_stdout();
        }
        // row 4: elemsize * min_cap overflow
        "arrgrowf_size_overflow" => {
            let r = (lib.arrgrowf)(core::ptr::null_mut(), usize::MAX / 2, 0, 4);
            println!("{}", r.is_null());
            flush_stdout();
        }
        // ERRORS.md row 43b: `a->block` chosen so `512 << (block>>1)` is 2^63,
        // i.e. the block allocation must fail and `sb->next = a->storage`
        // dereferences NULL.
        "stralloc_huge_block" => {
            let mut a = StringArena::zeroed();
            a.block = p.first().copied().unwrap_or(108) as u8;
            let mut sv: Vec<u8> = b"probe\0".to_vec();
            let q = (lib.stralloc)(&mut a, sv.as_mut_ptr() as *mut c_char);
            println!("survived {} remaining={}", !q.is_null(), a.remaining);
            flush_stdout();
        }
        // ERRORS.md row 44b: SH_NONE + mode >= HM_STRING => the element holds
        // the key's TEXT, which a later lookup reinterprets as a char*.
        "sh_none_string_lookup" => {
            (lib.rand_seed)(5);
            let elemsize = 16usize;
            let mut t = (lib.shmode_func)(elemsize, SH_NONE);
            let mut key: Vec<u8> = b"a_long_enough_key\0".to_vec();
            t = (lib.hmput_key)(t, elemsize, key.as_mut_ptr() as *mut c_void, 8, HM_STRING);
            // the very same key now hashes to the same slot, so
            // stbds_is_key_equal runs and dereferences the copied text
            t = (lib.hmget_key)(t, elemsize, key.as_mut_ptr() as *mut c_void, 8, HM_STRING);
            println!("temp={}", map_temp(t, elemsize));
            flush_stdout();
        }
        // ERRORS.md row 27/33/37/39: the STBDS_ASSERTs that must never fire.
        // A long randomised put/delete corpus; if any assert fired the child
        // would die on SIGABRT instead of exiting 0.
        "assert_soak" => {
            (lib.rand_seed)(p.first().copied().unwrap_or(1) as usize);
            let elemsize = 16usize;
            let mut rng = Rng::new(0xA55E27);
            let mut t: *mut c_void = core::ptr::null_mut();
            let mut keys: Vec<Box<[u8]>> = Vec::new();
            for _ in 0..256 {
                let mut b: Box<[u8]> = vec![0u8; 8].into_boxed_slice();
                for x in b.iter_mut() {
                    *x = rng.next_u8();
                }
                keys.push(b);
            }
            let mut live = vec![false; keys.len()];
            for _ in 0..20_000 {
                let i = rng.below(keys.len());
                let kp = keys[i].as_ptr() as *mut c_void;
                if rng.below(3) == 0 && live[i] {
                    t = (lib.hmdel_key)(t, elemsize, kp, 8, 0, HM_BINARY);
                    live[i] = false;
                } else {
                    t = (lib.hmput_key)(t, elemsize, kp, 8, HM_BINARY);
                    live[i] = true;
                }
            }
            let n = if t.is_null() {
                0
            } else {
                (*hdr_of_map(t, elemsize)).length
            };
            println!("survived length={n}");
            flush_stdout();
        }
        other => panic!("unknown scenario {other:?}"),
    }
}

fn flush_stdout() {
    use std::io::Write;
    let _ = std::io::stdout().flush();
    // The C library writes via libc `printf`, i.e. glibc's own stdout buffer,
    // which is flushed by exit(). Use libc exit semantics by returning normally
    // and letting the caller `std::process::exit(0)` (which runs atexit).
}
