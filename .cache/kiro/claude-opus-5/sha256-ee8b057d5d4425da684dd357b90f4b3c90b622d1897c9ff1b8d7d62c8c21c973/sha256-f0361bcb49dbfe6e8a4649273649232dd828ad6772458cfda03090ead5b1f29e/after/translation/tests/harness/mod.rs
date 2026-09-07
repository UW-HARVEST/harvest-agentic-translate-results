//! Shared differential-test harness.
//!
//! Loads BOTH the C `.so` and the Rust `.so` with `libloading` and exposes the
//! 16 exported symbols through identical function-pointer tables. No Rust
//! function is ever called directly — every call goes through the `.so`
//! exports, exactly as an external C consumer would.

#![allow(dead_code)]
#![allow(non_snake_case)]

use libloading::{Library, Symbol};
use std::ffi::{c_char, c_int, c_void};
use std::path::PathBuf;
use std::sync::{Mutex, MutexGuard, OnceLock};

// ---------------------------------------------------------------------------
// C type mirrors (layouts must match `c_src/src/lib.c` exactly)
// ---------------------------------------------------------------------------

pub const STBDS_BUCKET_LENGTH: usize = 8;
pub const STBDS_BUCKET_SHIFT: usize = 3;
pub const STBDS_BUCKET_MASK: usize = STBDS_BUCKET_LENGTH - 1;

pub const STBDS_HM_BINARY: c_int = 0;
pub const STBDS_HM_STRING: c_int = 1;

pub const STBDS_SH_NONE: c_int = 0;
pub const STBDS_SH_DEFAULT: c_int = 1;
pub const STBDS_SH_STRDUP: c_int = 2;
pub const STBDS_SH_ARENA: c_int = 3;

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct ArrHeader {
    pub length: usize,
    pub capacity: usize,
    pub hash_table: *mut c_void,
    pub temp: isize,
}

pub const HEADER_SIZE: usize = std::mem::size_of::<ArrHeader>();

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
// Function pointer types
// ---------------------------------------------------------------------------

type FnRandSeed = unsafe extern "C" fn(usize);
type FnHashBytes = unsafe extern "C" fn(*mut c_void, usize, usize) -> usize;
type FnHashString = unsafe extern "C" fn(*mut c_char, usize) -> usize;
type FnArrGrowf = unsafe extern "C" fn(*mut c_void, usize, usize, usize) -> *mut c_void;
type FnArrFreef = unsafe extern "C" fn(*mut c_void);
type FnHmFreeFunc = unsafe extern "C" fn(*mut c_void, usize);
type FnHmGetKey = unsafe extern "C" fn(*mut c_void, usize, *mut c_void, usize, c_int) -> *mut c_void;
type FnHmGetKeyTs =
    unsafe extern "C" fn(*mut c_void, usize, *mut c_void, usize, *mut isize, c_int) -> *mut c_void;
type FnHmPutDefault = unsafe extern "C" fn(*mut c_void, usize) -> *mut c_void;
type FnHmPutKey = unsafe extern "C" fn(*mut c_void, usize, *mut c_void, usize, c_int) -> *mut c_void;
type FnHmDelKey =
    unsafe extern "C" fn(*mut c_void, usize, *mut c_void, usize, usize, c_int) -> *mut c_void;
type FnShmodeFunc = unsafe extern "C" fn(usize, c_int) -> *mut c_void;
type FnStralloc = unsafe extern "C" fn(*mut StringArena, *mut c_char) -> *mut c_char;
type FnStrreset = unsafe extern "C" fn(*mut StringArena);
type FnStrkey = unsafe extern "C" fn(c_int) -> *mut c_char;
type FnArrDel = unsafe extern "C" fn(c_int);

/// The complete export surface of one library.
pub struct Api {
    pub name: &'static str,
    pub rand_seed: FnRandSeed,
    pub hash_bytes: FnHashBytes,
    pub hash_string: FnHashString,
    pub arrgrowf: FnArrGrowf,
    pub arrfreef: FnArrFreef,
    pub hmfree_func: FnHmFreeFunc,
    pub hmget_key: FnHmGetKey,
    pub hmget_key_ts: FnHmGetKeyTs,
    pub hmput_default: FnHmPutDefault,
    pub hmput_key: FnHmPutKey,
    pub hmdel_key: FnHmDelKey,
    pub shmode_func: FnShmodeFunc,
    pub stralloc: FnStralloc,
    pub strreset: FnStrreset,
    pub strkey: FnStrkey,
    pub arr_del: FnArrDel,
}

unsafe fn get<T: Copy>(lib: &Library, name: &[u8]) -> T {
    let s: Symbol<T> = unsafe {
        lib.get(name)
            .unwrap_or_else(|e| panic!("missing symbol {}: {e}", String::from_utf8_lossy(name)))
    };
    *s
}

unsafe fn build(name: &'static str, lib: &Library) -> Api {
    unsafe {
        Api {
            name,
            rand_seed: get(lib, b"stbds_rand_seed\0"),
            hash_bytes: get(lib, b"stbds_hash_bytes\0"),
            hash_string: get(lib, b"stbds_hash_string\0"),
            arrgrowf: get(lib, b"stbds_arrgrowf\0"),
            arrfreef: get(lib, b"stbds_arrfreef\0"),
            hmfree_func: get(lib, b"stbds_hmfree_func\0"),
            hmget_key: get(lib, b"stbds_hmget_key\0"),
            hmget_key_ts: get(lib, b"stbds_hmget_key_ts\0"),
            hmput_default: get(lib, b"stbds_hmput_default\0"),
            hmput_key: get(lib, b"stbds_hmput_key\0"),
            hmdel_key: get(lib, b"stbds_hmdel_key\0"),
            shmode_func: get(lib, b"stbds_shmode_func\0"),
            stralloc: get(lib, b"stbds_stralloc\0"),
            strreset: get(lib, b"stbds_strreset\0"),
            strkey: get(lib, b"strkey\0"),
            arr_del: get(lib, b"arr_del\0"),
        }
    }
}

pub struct Both {
    pub c: Api,
    pub r: Api,
    _libs: (Library, Library),
}

fn workspace_root() -> PathBuf {
    // .../<root>/translation
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("crate has a parent dir")
        .to_path_buf()
}

fn find_c_so() -> PathBuf {
    let dir = workspace_root().join("c_src/build");
    let mut found: Vec<PathBuf> = std::fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("cannot read {dir:?}: {e} — build the C library first"))
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| {
            p.extension().map(|x| x == "so").unwrap_or(false)
                && p.file_name()
                    .map(|n| n.to_string_lossy().starts_with("lib"))
                    .unwrap_or(false)
        })
        .collect();
    found.sort();
    found
        .pop()
        .unwrap_or_else(|| panic!("no lib*.so in {dir:?} — build the C library first"))
}

fn find_rust_so() -> PathBuf {
    // Allow pointing the harness at a specific build (e.g. the debug profile,
    // where Rust's arithmetic overflow checks are ON while the C always wraps).
    if let Ok(p) = std::env::var("RUST_SO_PATH") {
        let p = PathBuf::from(p);
        assert!(p.exists(), "RUST_SO_PATH does not exist: {p:?}");
        return p;
    }
    let base = workspace_root().join("translation/target");
    for profile in ["release", "debug"] {
        let p = base.join(profile).join("libarr_del_lib.so");
        if p.exists() {
            return p;
        }
    }
    panic!("libarr_del_lib.so not found under {base:?} — run `cargo build --release`")
}

static BOTH: OnceLock<Mutex<Both>> = OnceLock::new();

/// Acquire exclusive access to both libraries.
///
/// Both `.so`s carry mutable global state (`stbds_hash_seed`, `buffer`), so
/// every test must hold this lock, and each test resets the seed on both sides
/// so that the two libraries always start a scenario in identical states.
pub fn libs() -> MutexGuard<'static, Both> {
    let m = BOTH.get_or_init(|| {
        let cp = find_c_so();
        let rp = find_rust_so();
        unsafe {
            let cl = Library::new(&cp).unwrap_or_else(|e| panic!("load {cp:?}: {e}"));
            let rl = Library::new(&rp).unwrap_or_else(|e| panic!("load {rp:?}: {e}"));
            let c = build("C", &cl);
            let r = build("RUST", &rl);
            Mutex::new(Both {
                c,
                r,
                _libs: (cl, rl),
            })
        }
    });
    let g = m.lock().unwrap_or_else(|e| e.into_inner());
    g
}

/// Reset the mutable global seed in both libraries to the same value.
pub fn reset_seed(b: &Both, seed: usize) {
    unsafe {
        (b.c.rand_seed)(seed);
        (b.r.rand_seed)(seed);
    }
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (xorshift64*) — fixed seed for reproducibility
// ---------------------------------------------------------------------------

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Rng(if seed == 0 { 0xdead_beef_cafe_babe } else { seed })
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
    /// Printable, NUL-free ASCII string of length `n`.
    pub fn ascii(&mut self, n: usize) -> Vec<u8> {
        let mut v: Vec<u8> = (0..n).map(|_| 0x21 + (self.next_u8() % 0x5e)).collect();
        v.push(0);
        v
    }
    /// NUL-free string that may contain high-bit bytes.
    pub fn latin1(&mut self, n: usize) -> Vec<u8> {
        let mut v: Vec<u8> = (0..n)
            .map(|_| {
                let b = self.next_u8();
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

pub const TEST_SEED: u64 = 0x00C0_FFEE;

// ---------------------------------------------------------------------------
// State snapshots — canonical, pointer-free descriptions of library state
// ---------------------------------------------------------------------------

#[derive(PartialEq, Eq, Debug, Clone)]
pub struct HeaderSnap {
    pub length: usize,
    pub capacity: usize,
    pub has_hash_table: bool,
    pub temp: isize,
}

#[derive(PartialEq, Eq, Debug, Clone)]
pub struct IndexSnap {
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
    pub arena_chain_len: usize,
    pub buckets: Vec<(usize, isize)>,
}

#[derive(PartialEq, Eq, Debug, Clone)]
pub struct MapSnap {
    pub is_null: bool,
    pub header: Option<HeaderSnap>,
    pub index: Option<IndexSnap>,
    /// Raw `keysize + valsize` prefix of every element `0..length`
    /// (`KeyRepr::Bytes` only).
    pub elems: Option<Vec<u8>>,
    /// Key strings for elements `1..length` (`KeyRepr::Ptr` only).
    pub keys: Option<Vec<Vec<u8>>>,
    /// Value bytes for elements `1..length` (`KeyRepr::Ptr` only).
    pub vals: Option<Vec<Vec<u8>>>,
}

pub unsafe fn read_cstr(p: *const c_char) -> Vec<u8> {
    if p.is_null() {
        return b"<null>".to_vec();
    }
    let mut out = Vec::new();
    let mut q = p as *const u8;
    // Bound the walk so a corrupted pointer cannot hang the test forever.
    for _ in 0..1 << 20 {
        let b = unsafe { *q };
        if b == 0 {
            break;
        }
        out.push(b);
        q = unsafe { q.add(1) };
    }
    out
}

unsafe fn arena_chain_len(a: &StringArena) -> usize {
    let mut n = 0usize;
    let mut p = a.storage;
    while !p.is_null() && n < 1_000_000 {
        n += 1;
        p = unsafe { (*p).next };
    }
    n
}

/// How the element's key should be interpreted when snapshotting.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum KeyRepr {
    /// Keys are stored inline: compare raw element bytes.
    Bytes,
    /// Keys are `char *`: compare the pointed-to strings.
    Ptr,
}

/// Snapshot the state reachable from a hash-map pointer (`a` = `arr + elemsize`).
///
/// Only the `keysize + valsize` prefix of each element is compared: the rest of
/// an element is uninitialised padding in both libraries and must not be
/// compared. For `KeyRepr::Ptr` the key is an 8-byte `char *` whose *contents*
/// are compared instead of its (necessarily different) address.
pub unsafe fn snap_map(
    a: *mut c_void,
    elemsize: usize,
    repr: KeyRepr,
    keysize: usize,
    valsize: usize,
) -> MapSnap {
    unsafe {
        if a.is_null() {
            return MapSnap {
                is_null: true,
                header: None,
                index: None,
                elems: None,
                keys: None,
                vals: None,
            };
        }
        let raw = (a as *mut u8).wrapping_sub(elemsize) as *mut c_void;
        let h = (raw as *mut u8).wrapping_sub(HEADER_SIZE) as *mut ArrHeader;
        let hdr = *h;
        let header = HeaderSnap {
            length: hdr.length,
            capacity: hdr.capacity,
            has_hash_table: !hdr.hash_table.is_null(),
            temp: hdr.temp,
        };

        let index = if hdr.hash_table.is_null() {
            None
        } else {
            let t = hdr.hash_table as *mut HashIndex;
            let ti = *t;
            let nbuckets = ti.slot_count >> STBDS_BUCKET_SHIFT;
            let mut buckets = Vec::with_capacity(ti.slot_count);
            for bi in 0..nbuckets {
                let b = &*ti.storage.wrapping_add(bi);
                for j in 0..STBDS_BUCKET_LENGTH {
                    buckets.push((b.hash[j], b.index[j]));
                }
            }
            // `temp_key` is deliberately NOT part of this snapshot: the C never
            // initialises it in `stbds_make_hash_index`, so it holds allocator
            // garbage until a string-mode `stbds_hmput_key` writes it. Tests
            // that care read it explicitly via `read_temp_key` right after a put.
            Some(IndexSnap {
                slot_count: ti.slot_count,
                used_count: ti.used_count,
                used_count_threshold: ti.used_count_threshold,
                used_count_shrink_threshold: ti.used_count_shrink_threshold,
                tombstone_count: ti.tombstone_count,
                tombstone_count_threshold: ti.tombstone_count_threshold,
                seed: ti.seed,
                slot_count_log2: ti.slot_count_log2,
                arena_remaining: ti.string.remaining,
                arena_block: ti.string.block,
                arena_mode: ti.string.mode,
                arena_chain_len: arena_chain_len(&ti.string),
                buckets,
            })
        };

        let (elems, keys, vals) = match repr {
            KeyRepr::Bytes => {
                let n = (keysize + valsize).min(elemsize);
                let mut v = Vec::new();
                for i in 0..hdr.length {
                    let p = (raw as *mut u8).wrapping_add(elemsize * i);
                    for k in 0..n {
                        v.push(*p.wrapping_add(k));
                    }
                }
                (Some(v), None, None)
            }
            KeyRepr::Ptr => {
                let mut ks = Vec::new();
                let mut vs = Vec::new();
                // Element 0 is the zeroed "default" slot and holds no key.
                for i in 1..hdr.length {
                    let p = (raw as *mut u8).wrapping_add(elemsize * i);
                    ks.push(read_cstr(*(p as *mut *mut c_char)));
                    let mut vb = Vec::new();
                    for k in 0..valsize.min(elemsize.saturating_sub(8)) {
                        vb.push(*p.wrapping_add(8 + k));
                    }
                    vs.push(vb);
                }
                (None, Some(ks), Some(vs))
            }
        };

        MapSnap {
            is_null: false,
            header: Some(header),
            index,
            elems,
            keys,
            vals,
        }
    }
}

/// Read `stbds_temp_key(raw_a)` == `*(char **) header(raw_a)->hash_table`,
/// i.e. the `temp_key` field of the hash index, resolved to its string bytes.
/// Only meaningful right after a string-mode `stbds_hmput_key`.
pub unsafe fn read_temp_key(a: *mut c_void, elemsize: usize) -> Option<Vec<u8>> {
    unsafe {
        if a.is_null() {
            return None;
        }
        let raw = (a as *mut u8).wrapping_sub(elemsize) as *mut c_void;
        let h = (raw as *mut u8).wrapping_sub(HEADER_SIZE) as *mut ArrHeader;
        let ht = (*h).hash_table;
        if ht.is_null() {
            return None;
        }
        let tk = *(ht as *mut *mut c_char);
        if tk.is_null() {
            return Some(b"<null>".to_vec());
        }
        Some(read_cstr(tk))
    }
}

/// Snapshot a plain dynamic array pointer produced by `stbds_arrgrowf`.
#[derive(PartialEq, Eq, Debug, Clone)]
pub struct ArrSnap {
    pub offset_from_null: Option<usize>,
    pub header: Option<HeaderSnap>,
}

/// `stbds_arrgrowf` can legitimately return `HEADER_SIZE` (NULL + 32) without
/// allocating; in that case only the numeric value is comparable.
pub unsafe fn snap_arr(a: *mut c_void) -> ArrSnap {
    unsafe {
        let val = a as usize;
        if val < 4096 {
            return ArrSnap {
                offset_from_null: Some(val),
                header: None,
            };
        }
        let h = (a as *mut u8).wrapping_sub(HEADER_SIZE) as *mut ArrHeader;
        let hdr = *h;
        ArrSnap {
            offset_from_null: None,
            header: Some(HeaderSnap {
                length: hdr.length,
                capacity: hdr.capacity,
                has_hash_table: !hdr.hash_table.is_null(),
                temp: hdr.temp,
            }),
        }
    }
}

#[derive(PartialEq, Eq, Debug, Clone)]
pub struct ArenaSnap {
    pub remaining: usize,
    pub block: u8,
    pub mode: u8,
    pub chain_len: usize,
    /// Contents pointed at by each returned pointer, in call order.
    pub returned: Vec<Vec<u8>>,
    /// Byte offset of each returned pointer inside its own block's storage.
    pub offsets: Vec<isize>,
}

pub unsafe fn snap_arena(a: &StringArena) -> (usize, u8, u8, usize) {
    unsafe { (a.remaining, a.block, a.mode, arena_chain_len(a)) }
}

/// Assert two snapshots are identical, with a descriptive context message.
#[macro_export]
macro_rules! diff_eq {
    ($ctx:expr, $c:expr, $r:expr) => {{
        let cv = $c;
        let rv = $r;
        if cv != rv {
            panic!(
                "DIVERGENCE [{}]\n  C    = {:?}\n  RUST = {:?}",
                $ctx, cv, rv
            );
        }
    }};
}

// ---------------------------------------------------------------------------
// Lockstep hash-map driver
// ---------------------------------------------------------------------------

/// Drives one logical hash map in BOTH libraries at once, through the `.so`
/// exports only, and compares the complete resulting state after every call.
///
/// This deliberately reproduces what the `stbds_hmput`/`stbds_hmget`/
/// `stbds_hmdel` macros in `c_src/src/lib.c` expand to, so the low-level
/// `stbds_*_key` entry points are exercised the way a real consumer drives them
/// (including writing the value into `t[stbds_temp(t-1)]` after a put).
pub struct MapPair<'a> {
    pub b: &'a Both,
    pub c: *mut c_void,
    pub r: *mut c_void,
    pub elemsize: usize,
    pub keysize: usize,
    pub valsize: usize,
    pub repr: KeyRepr,
    pub mode: c_int,
    /// Byte offset of the value inside an element.
    pub valoff: usize,
}

impl<'a> MapPair<'a> {
    pub fn new(
        b: &'a Both,
        elemsize: usize,
        keysize: usize,
        valsize: usize,
        repr: KeyRepr,
        mode: c_int,
    ) -> Self {
        let valoff = match repr {
            KeyRepr::Bytes => keysize,
            KeyRepr::Ptr => 8,
        };
        assert!(valoff + valsize <= elemsize, "value does not fit in element");
        MapPair {
            b,
            c: std::ptr::null_mut(),
            r: std::ptr::null_mut(),
            elemsize,
            keysize,
            valsize,
            repr,
            mode,
            valoff,
        }
    }

    /// Start from an explicitly-moded table (`stbds_shmode_func`), as
    /// `sh_new_strdup` / `sh_new_arena` do.
    pub fn with_shmode(mut self, sh_mode: c_int) -> Self {
        unsafe {
            self.c = (self.b.c.shmode_func)(self.elemsize, sh_mode);
            self.r = (self.b.r.shmode_func)(self.elemsize, sh_mode);
        }
        self
    }

    pub fn snap_c(&self) -> MapSnap {
        unsafe { snap_map(self.c, self.elemsize, self.repr, self.keysize, self.valsize) }
    }
    pub fn snap_r(&self) -> MapSnap {
        unsafe { snap_map(self.r, self.elemsize, self.repr, self.keysize, self.valsize) }
    }

    pub fn check(&self, ctx: &str) {
        diff_eq!(ctx, self.snap_c(), self.snap_r());
    }

    unsafe fn temp_of(&self, a: *mut c_void) -> isize {
        unsafe {
            let raw = (a as *mut u8).wrapping_sub(self.elemsize) as *mut c_void;
            (*((raw as *mut u8).wrapping_sub(HEADER_SIZE) as *mut ArrHeader)).temp
        }
    }

    unsafe fn length_of(&self, a: *mut c_void) -> usize {
        unsafe {
            if a.is_null() {
                return 0;
            }
            let raw = (a as *mut u8).wrapping_sub(self.elemsize) as *mut c_void;
            (*((raw as *mut u8).wrapping_sub(HEADER_SIZE) as *mut ArrHeader)).length
        }
    }

    /// `stbds_hmput(t, k, v)` — put, then write the value at `t[temp]`.
    pub fn put(&mut self, key: &mut [u8], val: &[u8], ctx: &str) {
        assert_eq!(val.len(), self.valsize);
        unsafe {
            let len_before_c = self.length_of(self.c);
            let len_before_r = self.length_of(self.r);
            diff_eq!(format!("{ctx} pre-put length"), len_before_c, len_before_r);
            let kp = key.as_mut_ptr() as *mut c_void;
            self.c = (self.b.c.hmput_key)(self.c, self.elemsize, kp, self.keysize, self.mode);
            self.r = (self.b.r.hmput_key)(self.r, self.elemsize, kp, self.keysize, self.mode);
            let ct = self.temp_of(self.c);
            let rt = self.temp_of(self.r);
            diff_eq!(format!("{ctx} put temp"), ct, rt);
            let inserted = self.length_of(self.c) != len_before_c;
            diff_eq!(
                format!("{ctx} put inserted?"),
                inserted,
                self.length_of(self.r) != len_before_r
            );
            // `stbds_temp_key` is only compared when the C definitely WROTE it
            // on this call, i.e. on a fresh insertion in a pointer key mode.
            //
            // On a duplicate hit the C writes `temp_key` only in the first,
            // non-wrapped bucket scan — the wrapped `0..limit` scan omits it —
            // so `temp_key` can retain a pointer to a block that has since been
            // freed (`STBDS_SH_STRDUP` frees keys on delete). That stale value
            // is indeterminate memory, not an observable, so it is not compared.
            if self.repr == KeyRepr::Ptr && inserted {
                diff_eq!(
                    format!("{ctx} put temp_key"),
                    read_temp_key(self.c, self.elemsize),
                    read_temp_key(self.r, self.elemsize)
                );
            }
            // t[temp].value = v
            let cdst = (self.c as *mut u8)
                .wrapping_offset(self.elemsize as isize * ct)
                .wrapping_add(self.valoff);
            let rdst = (self.r as *mut u8)
                .wrapping_offset(self.elemsize as isize * rt)
                .wrapping_add(self.valoff);
            std::ptr::copy_nonoverlapping(val.as_ptr(), cdst, self.valsize);
            std::ptr::copy_nonoverlapping(val.as_ptr(), rdst, self.valsize);
            self.check(&format!("{ctx} put state"));
        }
    }

    /// `stbds_hmgeti(t, k)` — returns the index both libraries reported.
    pub fn geti(&mut self, key: &mut [u8], ctx: &str) -> isize {
        unsafe {
            let kp = key.as_mut_ptr() as *mut c_void;
            self.c = (self.b.c.hmget_key)(self.c, self.elemsize, kp, self.keysize, self.mode);
            self.r = (self.b.r.hmget_key)(self.r, self.elemsize, kp, self.keysize, self.mode);
            let ct = self.temp_of(self.c);
            let rt = self.temp_of(self.r);
            diff_eq!(format!("{ctx} geti temp"), ct, rt);
            self.check(&format!("{ctx} geti state"));
            ct
        }
    }

    /// `stbds_hmgeti_ts(t, k, temp)` — returns the out-param both reported.
    pub fn geti_ts(&mut self, key: &mut [u8], ctx: &str) -> isize {
        unsafe {
            let kp = key.as_mut_ptr() as *mut c_void;
            let mut ctemp: isize = 0x5a5a_5a5a;
            let mut rtemp: isize = 0x5a5a_5a5a;
            self.c = (self.b.c.hmget_key_ts)(
                self.c,
                self.elemsize,
                kp,
                self.keysize,
                &mut ctemp,
                self.mode,
            );
            self.r = (self.b.r.hmget_key_ts)(
                self.r,
                self.elemsize,
                kp,
                self.keysize,
                &mut rtemp,
                self.mode,
            );
            diff_eq!(format!("{ctx} geti_ts out"), ctemp, rtemp);
            self.check(&format!("{ctx} geti_ts state"));
            ctemp
        }
    }

    /// Read the value bytes at index `idx` (as `t[idx].value` would).
    pub fn value_at(&self, idx: isize) -> (Vec<u8>, Vec<u8>) {
        unsafe {
            let rd = |a: *mut c_void| -> Vec<u8> {
                let p = (a as *mut u8)
                    .wrapping_offset(self.elemsize as isize * idx)
                    .wrapping_add(self.valoff);
                (0..self.valsize).map(|k| *p.wrapping_add(k)).collect()
            };
            (rd(self.c), rd(self.r))
        }
    }

    /// `stbds_hmdel(t, k)` — returns the `temp` flag both libraries reported.
    pub fn del(&mut self, key: &mut [u8], keyoffset: usize, ctx: &str) -> isize {
        unsafe {
            let kp = key.as_mut_ptr() as *mut c_void;
            self.c = (self.b.c.hmdel_key)(
                self.c,
                self.elemsize,
                kp,
                self.keysize,
                keyoffset,
                self.mode,
            );
            self.r = (self.b.r.hmdel_key)(
                self.r,
                self.elemsize,
                kp,
                self.keysize,
                keyoffset,
                self.mode,
            );
            diff_eq!(
                format!("{ctx} del nullness"),
                self.c.is_null(),
                self.r.is_null()
            );
            let ct = if self.c.is_null() {
                0
            } else {
                self.temp_of(self.c)
            };
            let rt = if self.r.is_null() {
                0
            } else {
                self.temp_of(self.r)
            };
            diff_eq!(format!("{ctx} del temp"), ct, rt);
            self.check(&format!("{ctx} del state"));
            ct
        }
    }

    pub fn free(&mut self) {
        unsafe {
            if !self.c.is_null() {
                (self.b.c.hmfree_func)(
                    (self.c as *mut u8).wrapping_sub(self.elemsize) as *mut c_void,
                    self.elemsize,
                );
                self.c = std::ptr::null_mut();
            }
            if !self.r.is_null() {
                (self.b.r.hmfree_func)(
                    (self.r as *mut u8).wrapping_sub(self.elemsize) as *mut c_void,
                    self.elemsize,
                );
                self.r = std::ptr::null_mut();
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Fork helper — for comparing process-level outcomes (C `assert` -> SIGABRT)
// ---------------------------------------------------------------------------

unsafe extern "C" {
    fn fork() -> i32;
    fn waitpid(pid: i32, status: *mut i32, options: i32) -> i32;
    fn _exit(code: i32) -> !;
    fn setrlimit(resource: i32, rlim: *const RLimit) -> i32;
}

#[repr(C)]
struct RLimit {
    cur: u64,
    max: u64,
}

/// `RLIMIT_CORE` on Linux.
const RLIMIT_CORE: i32 = 4;

#[derive(PartialEq, Eq, Debug, Clone, Copy)]
pub enum Outcome {
    Exited(i32),
    Signaled(i32),
    Unknown(i32),
}

/// Run `f` in a forked child and report how the child terminated.
///
/// Needed because several `STBDS_ASSERT`s in `c_src/src/lib.c` are live (the
/// CMake build defines no `NDEBUG`), so hitting one aborts the whole process,
/// and because a few documented inputs corrupt the heap or dereference a wild
/// pointer identically in both libraries. The Rust translation must terminate
/// the same way in the same situations.
pub fn run_in_child<F: FnOnce()>(f: F) -> Outcome {
    unsafe {
        let pid = fork();
        assert!(pid >= 0, "fork failed");
        if pid == 0 {
            // Suppress core dumps: several scenarios abort or segfault on
            // purpose and dumping cores makes the suite an order of magnitude
            // slower.
            let rl = RLimit { cur: 0, max: 0 };
            setrlimit(RLIMIT_CORE, &rl);
            f();
            _exit(0);
        }
        let mut status: i32 = 0;
        let w = waitpid(pid, &mut status, 0);
        assert_eq!(w, pid, "waitpid failed");
        // WIFEXITED / WEXITSTATUS / WIFSIGNALED / WTERMSIG
        if status & 0x7f == 0x7f {
            Outcome::Unknown(status)
        } else if status & 0x7f == 0 {
            Outcome::Exited((status >> 8) & 0xff)
        } else {
            Outcome::Signaled(status & 0x7f)
        }
    }
}
