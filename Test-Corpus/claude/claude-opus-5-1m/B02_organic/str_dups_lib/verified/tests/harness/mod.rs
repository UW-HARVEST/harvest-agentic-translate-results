//! Differential-test harness.
//!
//! Loads BOTH shared libraries with `libloading` and calls everything through
//! their exported `extern "C"` symbols — the Rust crate is never linked or
//! called directly, so the `#[no_mangle]` wrappers are under test too.
//!
//! * C   : `c_src/build/libtranslated_rust.so`
//! * Rust: `target/release/libstr_dups_lib.so`
//!
//! Because the two libraries live in the same process but own separate heaps'
//! worth of allocations, raw pointer *values* can never be compared.  Instead
//! every operation is followed by a `Snapshot` that captures all observable
//! state (array header, hash index, every bucket slot, every element byte) with
//! library-owned key pointers normalised to their string contents.

#![allow(dead_code)]

use std::ffi::{c_char, c_int, c_void, CStr, CString};
use std::path::PathBuf;
use std::sync::OnceLock;

// ---------------------------------------------------------------------------
// C data layout (mirrors c_src/src/lib.c; verified by `abi_layout_matches_c`)
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
pub struct StringBlock {
    pub next: *mut StringBlock,
    pub storage: [c_char; 8],
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
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

pub const HEADER_SIZE: usize = std::mem::size_of::<ArrayHeader>();

pub const STBDS_HM_BINARY: c_int = 0;
pub const STBDS_HM_STRING: c_int = 1;
pub const STBDS_SH_NONE: c_int = 0;
pub const STBDS_SH_DEFAULT: c_int = 1;
pub const STBDS_SH_STRDUP: c_int = 2;
pub const STBDS_SH_ARENA: c_int = 3;

pub const DEFAULT_SEED: usize = 0x3141_5926;

/// `stbds_header(t)`
#[inline]
pub unsafe fn header(t: *mut c_void) -> *mut ArrayHeader {
    (t as *mut ArrayHeader).wrapping_sub(1)
}

// ---------------------------------------------------------------------------
// libc bits we need for output capture
// ---------------------------------------------------------------------------

extern "C" {
    fn dup(fd: c_int) -> c_int;
    fn dup2(old: c_int, new: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn open(path: *const c_char, flags: c_int, mode: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
    fn lseek(fd: c_int, off: i64, whence: c_int) -> i64;
    fn read(fd: c_int, buf: *mut c_void, n: usize) -> isize;
    fn unlink(path: *const c_char) -> c_int;
    fn free(p: *mut c_void);
    fn malloc(n: usize) -> *mut c_void;
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
pub type FnHmGetTs =
    unsafe extern "C" fn(*mut c_void, usize, *mut c_void, usize, *mut isize, c_int) -> *mut c_void;
pub type FnHmGet = unsafe extern "C" fn(*mut c_void, usize, *mut c_void, usize, c_int) -> *mut c_void;
pub type FnHmPutDefault = unsafe extern "C" fn(*mut c_void, usize) -> *mut c_void;
pub type FnHmPut = unsafe extern "C" fn(*mut c_void, usize, *mut c_void, usize, c_int) -> *mut c_void;
pub type FnShMode = unsafe extern "C" fn(usize, c_int) -> *mut c_void;
pub type FnHmDel =
    unsafe extern "C" fn(*mut c_void, usize, *mut c_void, usize, usize, c_int) -> *mut c_void;
pub type FnStrAlloc = unsafe extern "C" fn(*mut StringArena, *mut c_char) -> *mut c_char;
pub type FnStrReset = unsafe extern "C" fn(*mut StringArena);
pub type FnStrKey = unsafe extern "C" fn(c_int) -> *mut c_char;
pub type FnStrDups = unsafe extern "C" fn(c_int);

pub struct Lib {
    pub name: &'static str,
    pub path: PathBuf,
    pub arrgrowf: FnArrGrowf,
    pub arrfreef: FnArrFreef,
    pub rand_seed: FnRandSeed,
    pub hash_string: FnHashString,
    pub hash_bytes: FnHashBytes,
    pub hmfree_func: FnHmFree,
    pub hmget_key_ts: FnHmGetTs,
    pub hmget_key: FnHmGet,
    pub hmput_default: FnHmPutDefault,
    pub hmput_key: FnHmPut,
    pub shmode_func: FnShMode,
    pub hmdel_key: FnHmDel,
    pub stralloc: FnStrAlloc,
    pub strreset: FnStrReset,
    pub strkey: FnStrKey,
    pub str_dups: FnStrDups,
}

unsafe impl Sync for Lib {}
unsafe impl Send for Lib {}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("C_SO") {
        return PathBuf::from(p);
    }
    manifest_dir().join("c_src/build/libtranslated_rust.so")
}

fn rust_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("RUST_SO") {
        return PathBuf::from(p);
    }
    let rel = manifest_dir().join("target/release/libstr_dups_lib.so");
    if rel.exists() {
        return rel;
    }
    manifest_dir().join("target/debug/libstr_dups_lib.so")
}

unsafe fn load(name: &'static str, path: PathBuf) -> Lib {
    assert!(
        path.exists(),
        "{} shared library not found at {}\n\
         Build it first:\n  \
         C:    cd c_src && mkdir -p build && cd build && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .\n  \
         Rust: cargo build --release",
        name,
        path.display()
    );
    let lib = libloading::Library::new(&path)
        .unwrap_or_else(|e| panic!("dlopen({}) failed: {e}", path.display()));

    macro_rules! sym {
        ($n:literal, $t:ty) => {{
            let s: libloading::Symbol<$t> = lib
                .get($n)
                .unwrap_or_else(|e| panic!("{} missing symbol {:?}: {e}", name, $n));
            *s
        }};
    }

    let l = Lib {
        name,
        arrgrowf: sym!(b"stbds_arrgrowf\0", FnArrGrowf),
        arrfreef: sym!(b"stbds_arrfreef\0", FnArrFreef),
        rand_seed: sym!(b"stbds_rand_seed\0", FnRandSeed),
        hash_string: sym!(b"stbds_hash_string\0", FnHashString),
        hash_bytes: sym!(b"stbds_hash_bytes\0", FnHashBytes),
        hmfree_func: sym!(b"stbds_hmfree_func\0", FnHmFree),
        hmget_key_ts: sym!(b"stbds_hmget_key_ts\0", FnHmGetTs),
        hmget_key: sym!(b"stbds_hmget_key\0", FnHmGet),
        hmput_default: sym!(b"stbds_hmput_default\0", FnHmPutDefault),
        hmput_key: sym!(b"stbds_hmput_key\0", FnHmPut),
        shmode_func: sym!(b"stbds_shmode_func\0", FnShMode),
        hmdel_key: sym!(b"stbds_hmdel_key\0", FnHmDel),
        stralloc: sym!(b"stbds_stralloc\0", FnStrAlloc),
        strreset: sym!(b"stbds_strreset\0", FnStrReset),
        strkey: sym!(b"strkey\0", FnStrKey),
        str_dups: sym!(b"str_dups\0", FnStrDups),
        path,
    };
    // Never unload: the function pointers must stay valid for the whole run.
    std::mem::forget(lib);
    l
}

pub struct Libs {
    pub c: Lib,
    pub rust: Lib,
}

static LIBS: OnceLock<Libs> = OnceLock::new();

pub fn libs() -> &'static Libs {
    LIBS.get_or_init(|| unsafe {
        Libs {
            c: load("C", c_so_path()),
            rust: load("Rust", rust_so_path()),
        }
    })
}

/// Both libraries, in a fixed order, for loops that must do the *same* thing to
/// each of them.
pub fn both() -> [&'static Lib; 2] {
    let l = libs();
    [&l.c, &l.rust]
}

/// Reset the per-library global `stbds_hash_seed` so the two libraries stay in
/// lockstep no matter what ran before.
pub fn seed_both(seed: usize) {
    for l in both() {
        unsafe { (l.rand_seed)(seed) };
    }
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (xorshift64*) — fixed seed per test for reproducibility
// ---------------------------------------------------------------------------

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Rng(seed | 1)
    }
    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
    pub fn next_u32(&mut self) -> u32 {
        (self.next_u64() >> 32) as u32
    }
    pub fn next_usize(&mut self) -> usize {
        self.next_u64() as usize
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
    pub fn bytes(&mut self, n: usize) -> Vec<u8> {
        (0..n).map(|_| (self.next_u64() >> 24) as u8).collect()
    }
    /// NUL-free bytes, suitable for a C string body.
    pub fn nz_bytes(&mut self, n: usize) -> Vec<u8> {
        (0..n)
            .map(|_| {
                let b = (self.next_u64() >> 24) as u8;
                if b == 0 {
                    1
                } else {
                    b
                }
            })
            .collect()
    }
    /// Printable-ASCII bytes.
    pub fn ascii(&mut self, n: usize) -> Vec<u8> {
        (0..n)
            .map(|_| b'a' + (self.next_u64() % 26) as u8)
            .collect()
    }
}

// ---------------------------------------------------------------------------
// Snapshots
// ---------------------------------------------------------------------------

/// How a `char *` seen inside library memory is normalised for comparison.
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum KeyRepr {
    Null,
    /// Pointer into a buffer the *test* owns — the identical value is handed to
    /// both libraries, so the raw address is comparable.
    External(usize),
    /// Pointer into library-owned memory (`strdup`/arena): compared by the NUL
    /// terminated bytes it points at.
    Owned(Vec<u8>),
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum ElemSnap {
    /// Raw element bytes (binary keys, or string tables in the `memcpy` mode).
    Bytes(Vec<u8>),
    /// `char *` key followed by the raw value bytes.
    Ptr(KeyRepr, Vec<u8>),
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct TableSnap {
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
    pub string_block_count: usize,
    pub temp_key: Option<KeyRepr>,
    pub slots: Vec<(usize, isize)>,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Snapshot {
    pub is_null: bool,
    pub length: usize,
    pub capacity: usize,
    pub temp: isize,
    pub has_table: bool,
    pub table: Option<TableSnap>,
    pub elems: Vec<ElemSnap>,
}

/// Registry of test-owned key buffers, used to classify `KeyRepr`.
///
/// A library only ever *stores* the exact base pointer the test handed it
/// (`STBDS_SH_DEFAULT`), so exact-address membership is the right test — and it
/// keeps snapshotting O(1) per element for the multi-thousand-key rows.
#[derive(Default, Clone)]
pub struct ExternRanges(pub std::collections::HashSet<usize>);

impl ExternRanges {
    pub fn add(&mut self, p: *const u8, _len: usize) {
        self.0.insert(p as usize);
    }
    pub fn contains(&self, addr: usize) -> bool {
        self.0.contains(&addr)
    }
}

pub unsafe fn key_repr(p: *const c_char, ext: &ExternRanges) -> KeyRepr {
    if p.is_null() {
        KeyRepr::Null
    } else if ext.contains(p as usize) {
        KeyRepr::External(p as usize)
    } else {
        KeyRepr::Owned(CStr::from_ptr(p).to_bytes().to_vec())
    }
}

pub unsafe fn count_blocks(mut b: *mut StringBlock) -> usize {
    let mut n = 0;
    while !b.is_null() && n < 100_000 {
        n += 1;
        b = (*b).next;
    }
    n
}

/// Snapshot the observable state reachable from a hash-map pointer `t`.
///
/// * `elemsize` / `keysize` describe the element layout.
/// * `key_is_ptr` selects `ElemSnap::Ptr` (the first 8 bytes are a `char *`).
/// * `include_temp_key` must be `false` whenever `table->temp_key` has not been
///   written since the index was (re)built — `stbds_make_hash_index` leaves it
///   uninitialised, so it would otherwise compare malloc garbage.
pub unsafe fn snapshot(
    t: *mut c_void,
    elemsize: usize,
    keysize: usize,
    key_is_ptr: bool,
    include_temp_key: bool,
    ext: &ExternRanges,
) -> Snapshot {
    if t.is_null() {
        return Snapshot {
            is_null: true,
            length: 0,
            capacity: 0,
            temp: 0,
            has_table: false,
            table: None,
            elems: Vec::new(),
        };
    }
    // `t` is the "hash pointer": the raw array starts one element earlier.
    let raw = (t as *mut u8).wrapping_sub(elemsize) as *mut c_void;
    let h = header(raw);
    let length = (*h).length;
    let capacity = (*h).capacity;
    let temp = (*h).temp;
    let ti = (*h).hash_table as *mut HashIndex;

    let table = if ti.is_null() {
        None
    } else {
        let slot_count = (*ti).slot_count;
        let mut slots = Vec::with_capacity(slot_count);
        for i in 0..slot_count {
            let b = (*ti).storage.wrapping_add(i >> 3);
            slots.push(((*b).hash[i & 7], (*b).index[i & 7]));
        }
        Some(TableSnap {
            slot_count,
            used_count: (*ti).used_count,
            used_count_threshold: (*ti).used_count_threshold,
            used_count_shrink_threshold: (*ti).used_count_shrink_threshold,
            tombstone_count: (*ti).tombstone_count,
            tombstone_count_threshold: (*ti).tombstone_count_threshold,
            seed: (*ti).seed,
            slot_count_log2: (*ti).slot_count_log2,
            string_mode: (*ti).string.mode,
            string_block: (*ti).string.block,
            string_remaining: (*ti).string.remaining,
            string_block_count: count_blocks((*ti).string.storage),
            temp_key: if include_temp_key {
                Some(key_repr((*ti).temp_key, ext))
            } else {
                None
            },
            slots,
        })
    };

    // Elements: raw index 0 is the sentinel written by the bootstrap memset,
    // raw indices 1..length are the user elements.
    let mut elems = Vec::with_capacity(length);
    if elemsize > 0 {
        for i in 0..length {
            let e = (raw as *mut u8).wrapping_add(elemsize * i);
            if key_is_ptr {
                assert!(elemsize >= 8);
                let kp = *(e as *mut *mut c_char);
                let value = std::slice::from_raw_parts(e.add(8), elemsize - 8).to_vec();
                elems.push(ElemSnap::Ptr(key_repr(kp, ext), value));
            } else {
                let _ = keysize;
                elems.push(ElemSnap::Bytes(
                    std::slice::from_raw_parts(e, elemsize).to_vec(),
                ));
            }
        }
    }

    Snapshot {
        is_null: false,
        length,
        capacity,
        temp,
        has_table: !ti.is_null(),
        table,
        elems,
    }
}

// ---------------------------------------------------------------------------
// Rich mismatch reporting
// ---------------------------------------------------------------------------

pub fn diff_snapshots(ctx: &str, c: &Snapshot, r: &Snapshot) {
    if c == r {
        return;
    }
    let mut msg = format!("DIVERGENCE at {ctx}\n");
    macro_rules! f {
        ($field:ident) => {
            if c.$field != r.$field {
                msg += &format!("  {}: C={:?} Rust={:?}\n", stringify!($field), c.$field, r.$field);
            }
        };
    }
    f!(is_null);
    f!(length);
    f!(capacity);
    f!(temp);
    f!(has_table);
    match (&c.table, &r.table) {
        (Some(a), Some(b)) if a != b => {
            macro_rules! g {
                ($field:ident) => {
                    if a.$field != b.$field {
                        msg += &format!(
                            "  table.{}: C={:?} Rust={:?}\n",
                            stringify!($field),
                            a.$field,
                            b.$field
                        );
                    }
                };
            }
            g!(slot_count);
            g!(used_count);
            g!(used_count_threshold);
            g!(used_count_shrink_threshold);
            g!(tombstone_count);
            g!(tombstone_count_threshold);
            g!(seed);
            g!(slot_count_log2);
            g!(string_mode);
            g!(string_block);
            g!(string_remaining);
            g!(string_block_count);
            g!(temp_key);
            if a.slots != b.slots {
                msg += "  table.slots differ:\n";
                for (i, (x, y)) in a.slots.iter().zip(b.slots.iter()).enumerate() {
                    if x != y {
                        msg += &format!("    slot {i}: C={x:?} Rust={y:?}\n");
                    }
                }
                if a.slots.len() != b.slots.len() {
                    msg += &format!(
                        "    slot count C={} Rust={}\n",
                        a.slots.len(),
                        b.slots.len()
                    );
                }
            }
        }
        (x, y) if x != y => {
            msg += &format!("  table presence: C={:?} Rust={:?}\n", x.is_some(), y.is_some());
        }
        _ => {}
    }
    if c.elems != r.elems {
        msg += "  elems differ:\n";
        for i in 0..c.elems.len().max(r.elems.len()) {
            let a = c.elems.get(i);
            let b = r.elems.get(i);
            if a != b {
                msg += &format!("    raw elem {i}: C={a:?} Rust={b:?}\n");
            }
        }
    }
    panic!("{msg}");
}

// ---------------------------------------------------------------------------
// One map per library, driven in lockstep
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, Debug)]
pub struct MapCfg {
    pub elemsize: usize,
    pub keysize: usize,
    /// `true` when the first 8 bytes of an element are a `char *` written by the
    /// library (string.mode 1/2/3).  `false` for binary keys **and** for string
    /// tables whose `string.mode` falls into the `memcpy` default branch.
    pub key_is_ptr: bool,
}

pub struct Pair {
    pub cfg: MapCfg,
    /// hash pointers, index 0 = C, index 1 = Rust
    pub t: [*mut c_void; 2],
    table_ptr: [*mut c_void; 2],
    temp_key_valid: bool,
    pub ext: ExternRanges,
    /// Keeps test-owned key buffers alive for the lifetime of the pair.
    keep: Vec<Box<[u8]>>,
    pub label: String,
}

impl Pair {
    pub fn new(label: impl Into<String>, cfg: MapCfg) -> Self {
        Pair {
            cfg,
            t: [std::ptr::null_mut(); 2],
            table_ptr: [std::ptr::null_mut(); 2],
            temp_key_valid: false,
            ext: ExternRanges::default(),
            keep: Vec::new(),
            label: label.into(),
        }
    }

    fn lib(i: usize) -> &'static Lib {
        both()[i]
    }

    /// Register a test-owned buffer (so `KeyRepr::External` can recognise it)
    /// and return a stable pointer that is passed to *both* libraries.
    pub fn intern(&mut self, bytes: &[u8]) -> *mut u8 {
        let b: Box<[u8]> = bytes.to_vec().into_boxed_slice();
        let p = b.as_ptr() as *mut u8;
        self.ext.add(p, b.len());
        self.keep.push(b);
        p
    }

    pub fn intern_cstr(&mut self, s: &[u8]) -> *mut c_char {
        let mut v = s.to_vec();
        v.push(0);
        self.intern(&v) as *mut c_char
    }

    unsafe fn cur_table(&self, i: usize) -> *mut c_void {
        if self.t[i].is_null() {
            return std::ptr::null_mut();
        }
        let raw = (self.t[i] as *mut u8).wrapping_sub(self.cfg.elemsize) as *mut c_void;
        (*header(raw)).hash_table
    }

    unsafe fn len_of(&self, i: usize) -> usize {
        if self.t[i].is_null() {
            return 0;
        }
        let raw = (self.t[i] as *mut u8).wrapping_sub(self.cfg.elemsize) as *mut c_void;
        (*header(raw)).length
    }

    /// `table->string.mode`, or `None` when there is no index yet.
    pub fn string_mode(&self, i: usize) -> Option<u8> {
        unsafe {
            let tp = self.cur_table(i) as *mut HashIndex;
            if tp.is_null() {
                None
            } else {
                Some((*tp).string.mode)
            }
        }
    }

    fn note_rehash_and_growth(&mut self, len_before: [usize; 2], _mode: c_int) {
        // `stbds_temp_key` is written by the `switch (table->string.mode)` at
        // c_src/src/lib.c L785-790 — i.e. exactly for string.mode 1/2/3, and
        // *independently* of the `mode` argument.  The `default:` (memcpy)
        // branch leaves it as malloc garbage.
        let string_write = matches!(self.string_mode(0), Some(1) | Some(2) | Some(3));
        let mut rehashed = false;
        let mut grew = true;
        for i in 0..2 {
            let tp = unsafe { self.cur_table(i) };
            if tp != self.table_ptr[i] {
                rehashed = true;
                self.table_ptr[i] = tp;
            }
            // A bootstrap from NULL first creates the sentinel element, so the
            // baseline length is 1 even though it was 0 before the call.
            let base = if len_before[i] == 0 { 1 } else { len_before[i] };
            if unsafe { self.len_of(i) } != base + 1 {
                grew = false;
            }
        }
        if rehashed {
            self.temp_key_valid = false;
        }
        if string_write && grew {
            // A brand new element always writes `stbds_temp_key`.
            self.temp_key_valid = true;
        }
    }

    pub fn snapshot(&self, i: usize) -> Snapshot {
        unsafe {
            snapshot(
                self.t[i],
                self.cfg.elemsize,
                self.cfg.keysize,
                self.cfg.key_is_ptr,
                self.temp_key_valid,
                &self.ext,
            )
        }
    }

    pub fn check(&self, ctx: &str) {
        let c = self.snapshot(0);
        let r = self.snapshot(1);
        diff_snapshots(&format!("{} / {}", self.label, ctx), &c, &r);
    }

    /// Write the deterministic "value" region of user element `idx`, mirroring
    /// what the `hmput` / `shput` macros do after the library call.
    pub fn write_value(&self, idx: isize, tag: u64) {
        if idx < 0 {
            return;
        }
        let voff = if self.cfg.key_is_ptr {
            8
        } else {
            self.cfg.keysize
        };
        if voff >= self.cfg.elemsize {
            return;
        }
        let n = self.cfg.elemsize - voff;
        let mut bytes = Vec::with_capacity(n);
        let mut x = tag | 1;
        for _ in 0..n {
            x = x.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
            bytes.push((x >> 33) as u8);
        }
        for i in 0..2 {
            if self.t[i].is_null() {
                continue;
            }
            unsafe {
                let e = (self.t[i] as *mut u8)
                    .wrapping_add(self.cfg.elemsize * idx as usize)
                    .wrapping_add(voff);
                std::ptr::copy_nonoverlapping(bytes.as_ptr(), e, n);
            }
        }
    }

    // --- operations -------------------------------------------------------

    pub fn shmode(&mut self, mode: c_int) {
        for i in 0..2 {
            self.t[i] = unsafe { (Self::lib(i).shmode_func)(self.cfg.elemsize, mode) };
            self.table_ptr[i] = unsafe { self.cur_table(i) };
        }
        self.temp_key_valid = false;
    }

    pub fn put_default(&mut self) {
        for i in 0..2 {
            self.t[i] = unsafe { (Self::lib(i).hmput_default)(self.t[i], self.cfg.elemsize) };
            self.table_ptr[i] = unsafe { self.cur_table(i) };
        }
    }

    /// `stbds_hmput_key`; `key` must be a pointer valid in *both* libraries
    /// (i.e. test-owned).  Returns the agreed `temp`.
    pub fn put(&mut self, key: *mut c_void, mode: c_int, tag: u64) -> isize {
        let len_before = [unsafe { self.len_of(0) }, unsafe { self.len_of(1) }];
        let mut temps = [0isize; 2];
        for i in 0..2 {
            self.t[i] = unsafe {
                (Self::lib(i).hmput_key)(self.t[i], self.cfg.elemsize, key, self.cfg.keysize, mode)
            };
            let raw = (self.t[i] as *mut u8).wrapping_sub(self.cfg.elemsize) as *mut c_void;
            temps[i] = unsafe { (*header(raw)).temp };
        }
        assert_eq!(
            temps[0], temps[1],
            "{}: hmput_key temp mismatch (C={} Rust={})",
            self.label, temps[0], temps[1]
        );
        self.note_rehash_and_growth(len_before, mode);
        self.write_value(temps[0], tag);
        temps[0]
    }

    pub fn get(&mut self, key: *mut c_void, mode: c_int) -> isize {
        let mut temps = [0isize; 2];
        for i in 0..2 {
            self.t[i] = unsafe {
                (Self::lib(i).hmget_key)(self.t[i], self.cfg.elemsize, key, self.cfg.keysize, mode)
            };
            let raw = (self.t[i] as *mut u8).wrapping_sub(self.cfg.elemsize) as *mut c_void;
            temps[i] = unsafe { (*header(raw)).temp };
            self.table_ptr[i] = unsafe { self.cur_table(i) };
        }
        assert_eq!(
            temps[0], temps[1],
            "{}: hmget_key temp mismatch (C={} Rust={})",
            self.label, temps[0], temps[1]
        );
        temps[0]
    }

    pub fn get_ts(&mut self, key: *mut c_void, mode: c_int) -> isize {
        let mut temps = [0isize; 2];
        for i in 0..2 {
            let mut tmp: isize = 0x5a5a_5a5a;
            self.t[i] = unsafe {
                (Self::lib(i).hmget_key_ts)(
                    self.t[i],
                    self.cfg.elemsize,
                    key,
                    self.cfg.keysize,
                    &mut tmp,
                    mode,
                )
            };
            temps[i] = tmp;
            self.table_ptr[i] = unsafe { self.cur_table(i) };
        }
        assert_eq!(
            temps[0], temps[1],
            "{}: hmget_key_ts *temp mismatch (C={} Rust={})",
            self.label, temps[0], temps[1]
        );
        temps[0]
    }

    pub fn del(&mut self, key: *mut c_void, keyoffset: usize, mode: c_int) -> isize {
        // `stbds_hmdel_key` never writes `stbds_temp_key`, but on a SH_STRDUP
        // table it `free()`s the key the pointer may still refer to (C L836-837)
        // — so afterwards `table->temp_key` can dangle and must not be compared.
        self.temp_key_valid = false;
        let mut temps = [0isize; 2];
        for i in 0..2 {
            self.t[i] = unsafe {
                (Self::lib(i).hmdel_key)(
                    self.t[i],
                    self.cfg.elemsize,
                    key,
                    self.cfg.keysize,
                    keyoffset,
                    mode,
                )
            };
            temps[i] = if self.t[i].is_null() {
                0
            } else {
                let raw = (self.t[i] as *mut u8).wrapping_sub(self.cfg.elemsize) as *mut c_void;
                unsafe { (*header(raw)).temp }
            };
            let tp = unsafe { self.cur_table(i) };
            if tp != self.table_ptr[i] {
                self.temp_key_valid = false;
                self.table_ptr[i] = tp;
            }
        }
        assert_eq!(
            temps[0], temps[1],
            "{}: hmdel_key temp mismatch (C={} Rust={})",
            self.label, temps[0], temps[1]
        );
        temps[0]
    }

    pub fn free(&mut self) {
        for i in 0..2 {
            if !self.t[i].is_null() {
                let raw = (self.t[i] as *mut u8).wrapping_sub(self.cfg.elemsize) as *mut c_void;
                unsafe { (Self::lib(i).hmfree_func)(raw, self.cfg.elemsize) };
            }
            self.t[i] = std::ptr::null_mut();
            self.table_ptr[i] = std::ptr::null_mut();
        }
    }
}

// ---------------------------------------------------------------------------
// stdout capture (for `str_dups`, which printf()s)
// ---------------------------------------------------------------------------

const O_RDWR: c_int = 2;
const O_CREAT: c_int = 64;
const O_TRUNC: c_int = 512;

/// Redirect fd 1 to a temporary file, run `f`, restore, return what was written.
///
/// Both libraries `printf` through the *process* libc, so this observes the C
/// and the Rust output identically.  Tests must run single threaded
/// (`--test-threads=1`, see `run_tests.sh`).
pub fn capture_stdout<F: FnOnce()>(tag: &str, f: F) -> Vec<u8> {
    let dir = std::env::var("TMPDIR").unwrap_or_else(|_| "/tmp".into());
    let path = format!("{dir}/difftest-stdout-{}-{}.tmp", std::process::id(), tag);
    let cpath = CString::new(path.clone()).unwrap();
    unsafe {
        fflush(std::ptr::null_mut());
        let saved = dup(1);
        assert!(saved >= 0, "dup(1) failed");
        let fd = open(cpath.as_ptr(), O_RDWR | O_CREAT | O_TRUNC, 0o600);
        assert!(fd >= 0, "open({path}) failed");
        assert!(dup2(fd, 1) >= 0, "dup2 failed");

        f();

        fflush(std::ptr::null_mut());
        dup2(saved, 1);
        close(saved);

        lseek(fd, 0, 0);
        let mut out = Vec::new();
        let mut buf = [0u8; 8192];
        loop {
            let n = read(fd, buf.as_mut_ptr() as *mut c_void, buf.len());
            if n <= 0 {
                break;
            }
            out.extend_from_slice(&buf[..n as usize]);
        }
        close(fd);
        unlink(cpath.as_ptr());
        out
    }
}

// ---------------------------------------------------------------------------
// Crash / abort equivalence: re-exec this very test binary for one #[ignore]d
// case, once per library, and compare the way the process dies.
// ---------------------------------------------------------------------------

#[derive(Debug, PartialEq, Eq)]
pub struct ChildOutcome {
    pub signal: Option<i32>,
    pub code: Option<i32>,
    /// The `assert()` diagnostic, normalised: the directory part of `__FILE__`
    /// is dropped because it records where the library was *compiled*.
    pub assert_line: Option<String>,
}

pub const CHILD_LIB_ENV: &str = "DIFFTEST_CHILD_LIB";

/// Which library a `#[ignore]`d child case should drive.
pub fn child_lib() -> &'static Lib {
    match std::env::var(CHILD_LIB_ENV).as_deref() {
        Ok("c") => &libs().c,
        Ok("rust") => &libs().rust,
        other => panic!("{CHILD_LIB_ENV} must be 'c' or 'rust' (got {other:?})"),
    }
}

fn normalise_assert(stderr: &[u8]) -> Option<String> {
    let s = String::from_utf8_lossy(stderr);
    for line in s.lines() {
        if let Some(pos) = line.find(": Assertion `") {
            // line looks like: "<prog>: <file>:<line>: <func>: Assertion `x' failed."
            let head = &line[..pos];
            // Keep everything from the basename of __FILE__ onwards.
            let start = head.rfind('/').map(|i| i + 1).unwrap_or(0);
            return Some(format!("{}{}", &head[start..], &line[pos..]));
        }
    }
    None
}

pub fn run_child_case(case: &str, which: &str) -> ChildOutcome {
    use std::os::unix::process::ExitStatusExt;
    let exe = std::env::current_exe().expect("current_exe");
    let out = std::process::Command::new(exe)
        .arg(case)
        .arg("--exact")
        .arg("--ignored")
        .arg("--nocapture")
        .arg("--test-threads=1")
        .env(CHILD_LIB_ENV, which)
        .env("RUST_BACKTRACE", "0")
        .output()
        .expect("spawning child test case");
    ChildOutcome {
        signal: out.status.signal(),
        code: out.status.code(),
        assert_line: normalise_assert(&out.stderr),
    }
}

/// `true` when the Rust `.so` under test is an unoptimized build.
///
/// The crate's `[profile.release]` pins `debug-assertions = false` and
/// `panic = "abort"`, which is the configuration that mirrors the C.  A `dev`
/// build additionally enables Rust's *runtime null-pointer-dereference check*,
/// which converts the C's `SIGSEGV` into a non-unwinding panic (`SIGABRT`).
/// That is a build-profile artifact, not a behavioural difference, so the
/// crash-equivalence tests bucket every fatal signal together in that case
/// (the `assert()` diagnostics are still compared byte for byte).
pub fn rust_so_is_debug() -> bool {
    rust_so_path().to_string_lossy().contains("/debug/")
}

fn fatal_class(o: &ChildOutcome) -> Option<&'static str> {
    match o.signal {
        Some(6) | Some(4) | Some(11) => Some("fatal"),
        Some(_) => Some("other-signal"),
        None => None,
    }
}

/// Assert that a deliberately-fatal case kills the process in exactly the same
/// way for both libraries.
pub fn assert_same_crash(case: &str) {
    let c = run_child_case(case, "c");
    let r = run_child_case(case, "rust");
    assert!(
        c.signal.is_some(),
        "{case}: the C library was expected to die from a signal, got {c:?}"
    );
    assert!(
        r.signal.is_some(),
        "{case}: the Rust library was expected to die from a signal, got {r:?}"
    );
    assert_eq!(
        c.assert_line, r.assert_line,
        "crash diagnostic of {case} diverges:\n  C   = {c:?}\n  Rust= {r:?}"
    );
    if rust_so_is_debug() {
        assert_eq!(
            fatal_class(&c),
            fatal_class(&r),
            "crash class of {case} diverges (debug .so):\n  C   = {c:?}\n  Rust= {r:?}"
        );
        if c.signal != r.signal {
            eprintln!(
                "note: {case}: C signal {:?} vs Rust signal {:?} — expected with a \
                 dev-profile .so (debug_assertions null check); re-run against \
                 target/release/libstr_dups_lib.so for exact signal parity",
                c.signal, r.signal
            );
        }
        return;
    }
    assert_eq!(
        c, r,
        "crash behaviour of {case} diverges:\n  C   = {c:?}\n  Rust= {r:?}"
    );
}

/// Assert that a case is *not* fatal for either library (used for the
/// "unreachable assert" rows of ERRORS.md).
pub fn assert_no_crash(case: &str) {
    let c = run_child_case(case, "c");
    let r = run_child_case(case, "rust");
    assert_eq!(
        c, r,
        "outcome of {case} diverges:\n  C   = {c:?}\n  Rust= {r:?}"
    );
    assert_eq!(c.signal, None, "{case} unexpectedly died: {c:?}");
    assert_eq!(c.code, Some(0), "{case} did not exit cleanly: {c:?}");
}

// ---------------------------------------------------------------------------
// Small helpers
// ---------------------------------------------------------------------------

pub fn cstring(s: &[u8]) -> CString {
    CString::new(s.to_vec()).unwrap()
}

pub unsafe fn cstr_bytes(p: *const c_char) -> Vec<u8> {
    CStr::from_ptr(p).to_bytes().to_vec()
}

/// Heap-allocate with the *process* allocator so both libraries can `free` it.
pub unsafe fn libc_alloc(n: usize) -> *mut u8 {
    let p = malloc(n) as *mut u8;
    assert!(!p.is_null());
    p
}

pub unsafe fn libc_free(p: *mut u8) {
    free(p as *mut c_void)
}
