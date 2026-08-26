//! Differential-test harness.
//!
//! BOTH implementations are loaded as shared objects with `libloading` and
//! driven only through their exported `extern "C"` symbols — the Rust crate is
//! never linked or called directly, so the `#[unsafe(no_mangle)]` wrappers are
//! under test as well.

#![allow(dead_code, non_snake_case, non_camel_case_types)]

use libloading::{Library, Symbol};
use std::ffi::{c_char, c_int, c_void};
use std::path::PathBuf;
use std::sync::{Mutex, MutexGuard, OnceLock};

// ---------------------------------------------------------------------------
// C-layout mirrors of the private structs in c_src/src/lib.c
// ---------------------------------------------------------------------------

#[repr(C)]
#[derive(Debug)]
pub struct ArrayHeader {
    pub length: usize,
    pub capacity: usize,
    pub hash_table: *mut c_void,
    pub temp: isize,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct StringBlock {
    pub next: *mut StringBlock,
    pub storage: [c_char; 8],
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct Arena {
    pub storage: *mut StringBlock,
    pub remaining: usize,
    pub block: u8,
    pub mode: u8,
}

impl Arena {
    pub const fn zeroed() -> Arena {
        Arena { storage: std::ptr::null_mut(), remaining: 0, block: 0, mode: 0 }
    }
}

#[repr(C)]
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
    pub string: Arena,
    pub storage: *mut HashBucket,
}

pub const HEADER_SIZE: usize = std::mem::size_of::<ArrayHeader>(); // 32

pub const STBDS_HM_BINARY: c_int = 0;
pub const STBDS_HM_STRING: c_int = 1;
pub const STBDS_HM_PTR_TO_STRING: c_int = 2;

pub const SH_NONE: c_int = 0;
pub const SH_DEFAULT: c_int = 1;
pub const SH_STRDUP: c_int = 2;
pub const SH_ARENA: c_int = 3;

// ---------------------------------------------------------------------------
// The 16 exported symbols
// ---------------------------------------------------------------------------

pub struct Api {
    pub name: &'static str,
    pub arrgrowf: unsafe extern "C" fn(*mut c_void, usize, usize, usize) -> *mut c_void,
    pub arrfreef: unsafe extern "C" fn(*mut c_void),
    pub rand_seed: unsafe extern "C" fn(usize),
    pub hash_string: unsafe extern "C" fn(*mut c_char, usize) -> usize,
    pub hash_bytes: unsafe extern "C" fn(*mut c_void, usize, usize) -> usize,
    pub hmfree_func: unsafe extern "C" fn(*mut c_void, usize),
    pub hmget_key_ts:
        unsafe extern "C" fn(*mut c_void, usize, *mut c_void, usize, *mut isize, c_int) -> *mut c_void,
    pub hmget_key: unsafe extern "C" fn(*mut c_void, usize, *mut c_void, usize, c_int) -> *mut c_void,
    pub hmput_default: unsafe extern "C" fn(*mut c_void, usize) -> *mut c_void,
    pub hmput_key: unsafe extern "C" fn(*mut c_void, usize, *mut c_void, usize, c_int) -> *mut c_void,
    pub shmode_func: unsafe extern "C" fn(usize, c_int) -> *mut c_void,
    pub hmdel_key:
        unsafe extern "C" fn(*mut c_void, usize, *mut c_void, usize, usize, c_int) -> *mut c_void,
    pub stralloc: unsafe extern "C" fn(*mut Arena, *mut c_char) -> *mut c_char,
    pub strreset: unsafe extern "C" fn(*mut Arena),
    pub strkey: unsafe extern "C" fn(c_int) -> *mut c_char,
    pub arr_del: unsafe extern "C" fn(c_int),
}

unsafe fn sym<T: Copy>(lib: &'static Library, n: &str) -> T {
    let s: Symbol<T> = lib.get(n.as_bytes()).unwrap_or_else(|e| {
        panic!("symbol `{n}` missing from shared object: {e}");
    });
    *s
}

unsafe fn load(path: &PathBuf, name: &'static str) -> Api {
    let lib: &'static Library = Box::leak(Box::new(
        Library::new(path).unwrap_or_else(|e| panic!("cannot dlopen {}: {e}", path.display())),
    ));
    Api {
        name,
        arrgrowf: sym(lib, "stbds_arrgrowf"),
        arrfreef: sym(lib, "stbds_arrfreef"),
        rand_seed: sym(lib, "stbds_rand_seed"),
        hash_string: sym(lib, "stbds_hash_string"),
        hash_bytes: sym(lib, "stbds_hash_bytes"),
        hmfree_func: sym(lib, "stbds_hmfree_func"),
        hmget_key_ts: sym(lib, "stbds_hmget_key_ts"),
        hmget_key: sym(lib, "stbds_hmget_key"),
        hmput_default: sym(lib, "stbds_hmput_default"),
        hmput_key: sym(lib, "stbds_hmput_key"),
        shmode_func: sym(lib, "stbds_shmode_func"),
        hmdel_key: sym(lib, "stbds_hmdel_key"),
        stralloc: sym(lib, "stbds_stralloc"),
        strreset: sym(lib, "stbds_strreset"),
        strkey: sym(lib, "strkey"),
        arr_del: sym(lib, "arr_del"),
    }
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

pub fn c_so_path() -> PathBuf {
    manifest_dir().join("c_src/build/libtranslated_rust.so")
}

pub fn rust_so_path() -> PathBuf {
    // .../target/<profile>/deps/<testbin>  ->  .../target/<profile>/libarr_del_lib.so
    let exe = std::env::current_exe().expect("current_exe");
    let dir = exe.parent().unwrap().parent().unwrap();
    dir.join("libarr_del_lib.so")
}

static APIS: OnceLock<(Api, Api)> = OnceLock::new();

/// `(c, rust)`
pub fn apis() -> (&'static Api, &'static Api) {
    let p = APIS.get_or_init(|| unsafe {
        let c = load(&c_so_path(), "C");
        let r = load(&rust_so_path(), "RUST");
        (c, r)
    });
    (&p.0, &p.1)
}

/// Both shared objects carry process-global mutable state (`stbds_hash_seed`,
/// `buffer`). Tests therefore serialise on this lock and re-seed both libraries
/// at the start of every scenario so the two stay in lock-step.
static BIG_LOCK: Mutex<()> = Mutex::new(());

pub fn lock() -> MutexGuard<'static, ()> {
    match BIG_LOCK.lock() {
        Ok(g) => g,
        Err(p) => p.into_inner(),
    }
}

/// Take the lock, fetch both APIs and reset both global seeds to `seed`.
pub fn scenario(seed: usize) -> (MutexGuard<'static, ()>, &'static Api, &'static Api) {
    let g = lock();
    let (c, r) = apis();
    unsafe {
        (c.rand_seed)(seed);
        (r.rand_seed)(seed);
    }
    (g, c, r)
}

// ---------------------------------------------------------------------------
// Deterministic RNG (splitmix64) — fixed seed for reproducibility
// ---------------------------------------------------------------------------

pub struct Rng(pub u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng(seed ^ 0x243F_6A88_85A3_08D3)
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
    /// uniform-ish in `0..n`
    pub fn below(&mut self, n: usize) -> usize {
        if n == 0 {
            0
        } else {
            (self.next_u64() % (n as u64)) as usize
        }
    }
    pub fn bytes(&mut self, n: usize) -> Vec<u8> {
        (0..n).map(|_| self.next_u8()).collect()
    }
    /// NUL-terminated printable-ASCII string of `n` characters
    pub fn cstring(&mut self, n: usize) -> Vec<u8> {
        let mut v: Vec<u8> = (0..n).map(|_| b'a' + (self.next_u8() % 26)).collect();
        v.push(0);
        v
    }
}

// ---------------------------------------------------------------------------
// Canonical dumps: byte serialisations that are independent of the absolute
// addresses handed out by malloc, but capture every observable field.
// ---------------------------------------------------------------------------

fn push_u64(o: &mut Vec<u8>, v: u64) {
    o.extend_from_slice(&v.to_le_bytes());
}
fn push_usize(o: &mut Vec<u8>, v: usize) {
    push_u64(o, v as u64);
}
fn push_isize(o: &mut Vec<u8>, v: isize) {
    push_u64(o, v as u64);
}

/// `NULL` -> `[0]`, otherwise `[1] + bytes + [0]`
pub unsafe fn push_cstr(o: &mut Vec<u8>, p: *const c_char) {
    if p.is_null() {
        o.push(0);
    } else {
        o.push(1);
        let mut q = p as *const u8;
        while *q != 0 {
            o.push(*q);
            q = q.add(1);
        }
        o.push(0);
    }
}

pub unsafe fn cstr_to_vec(p: *const c_char) -> Vec<u8> {
    let mut v = Vec::new();
    if p.is_null() {
        return v;
    }
    let mut q = p as *const u8;
    while *q != 0 {
        v.push(*q);
        q = q.add(1);
    }
    v
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum KeyKind {
    /// element bytes are compared verbatim (binary keys)
    Raw,
    /// the first `sizeof(char*)` bytes of every element are a `char *`; the
    /// pointed-to string is compared instead of the (allocation-dependent)
    /// pointer value
    StrPtr,
}

/// Serialise the array header + live elements reachable from an array base
/// pointer `arr` (i.e. `t - elemsize` for a map, or the raw `arrgrowf` result).
pub unsafe fn dump_array(arr: *mut u8, elemsize: usize, kk: KeyKind) -> Vec<u8> {
    let mut o = Vec::new();
    if arr.is_null() {
        o.push(0);
        return o;
    }
    o.push(1);
    let h = arr.sub(HEADER_SIZE) as *const ArrayHeader;
    push_usize(&mut o, (*h).length);
    push_usize(&mut o, (*h).capacity);
    push_isize(&mut o, (*h).temp);
    for i in 0..(*h).length {
        let p = arr.add(i * elemsize);
        match kk {
            KeyKind::Raw => o.extend_from_slice(std::slice::from_raw_parts(p, elemsize)),
            KeyKind::StrPtr => {
                push_cstr(&mut o, *(p as *const *const c_char));
                if elemsize > 8 {
                    o.extend_from_slice(std::slice::from_raw_parts(p.add(8), elemsize - 8));
                }
            }
        }
    }
    o
}

/// Serialise everything reachable from a map's "hash pointer" `t`
/// (`t == arr + elemsize`): header, live elements and the whole hash index
/// including every bucket.
pub unsafe fn dump_map(t: *mut u8, elemsize: usize, kk: KeyKind) -> Vec<u8> {
    let mut o = Vec::new();
    if t.is_null() {
        o.push(0);
        return o;
    }
    let arr = t.sub(elemsize);
    o.extend_from_slice(&dump_array(arr, elemsize, kk));

    let h = arr.sub(HEADER_SIZE) as *const ArrayHeader;
    let ht = (*h).hash_table as *const HashIndex;
    if ht.is_null() {
        o.push(0);
        return o;
    }
    o.push(1);
    // NB: `temp_key` is deliberately NOT dumped here. `stbds_make_hash_index`
    // never initialises it (lib.c:386-472), so it holds indeterminate heap
    // bytes until a `mode >= STBDS_HM_STRING` put writes it. Tests that have
    // just done such a put compare it explicitly via `temp_key()`.
    push_usize(&mut o, (*ht).slot_count);
    push_usize(&mut o, (*ht).used_count);
    push_usize(&mut o, (*ht).used_count_threshold);
    push_usize(&mut o, (*ht).used_count_shrink_threshold);
    push_usize(&mut o, (*ht).tombstone_count);
    push_usize(&mut o, (*ht).tombstone_count_threshold);
    push_usize(&mut o, (*ht).seed);
    push_usize(&mut o, (*ht).slot_count_log2);
    // embedded arena: pointer identity is not comparable, its shape is
    o.push((*ht).string.storage.is_null() as u8);
    push_usize(&mut o, (*ht).string.remaining);
    o.push((*ht).string.block);
    o.push((*ht).string.mode);
    push_usize(&mut o, arena_chain_len(&(*ht).string));
    // buckets
    let nbuckets = (*ht).slot_count >> 3;
    for b in 0..nbuckets {
        let bk = (*ht).storage.add(b);
        for j in 0..8 {
            push_usize(&mut o, (*bk).hash[j]);
        }
        for j in 0..8 {
            push_isize(&mut o, (*bk).index[j]);
        }
    }
    o
}

/// Overwrite the element bytes the library deliberately leaves indeterminate
/// (everything past the key: `stbds_hmput_key` only `memcpy`s `keysize` bytes /
/// writes a `char *`, the rest comes straight from `realloc`) with a
/// deterministic pattern, so that raw element dumps are comparable at all.
/// `keep` = number of leading bytes per element to preserve.
pub unsafe fn canon_elements(arr: *mut u8, elemsize: usize, keep: usize, pat: u8) {
    if arr.is_null() || elemsize <= keep {
        return;
    }
    let h = arr.sub(HEADER_SIZE) as *const ArrayHeader;
    for i in 0..(*h).length {
        let p = arr.add(i * elemsize);
        for j in keep..elemsize {
            *p.add(j) = pat.wrapping_add(j as u8).wrapping_add((i as u8).wrapping_mul(7));
        }
    }
}

/// `canon_elements` for a C/Rust pair of map "hash pointers".
pub unsafe fn canon_pair(ct: *mut c_void, rt: *mut c_void, elemsize: usize, keep: usize) {
    if !ct.is_null() {
        canon_elements((ct as *mut u8).sub(elemsize), elemsize, keep, 0xC3);
    }
    if !rt.is_null() {
        canon_elements((rt as *mut u8).sub(elemsize), elemsize, keep, 0xC3);
    }
}

/// `stbds_temp_key(t-1)` == `*(char **) stbds_header(arr)->hash_table`
/// (the `temp_key` field of the hash index). Only meaningful right after a
/// `mode >= STBDS_HM_STRING` put.
pub unsafe fn temp_key(t: *mut u8, elemsize: usize) -> *mut c_char {
    let arr = t.sub(elemsize);
    let h = arr.sub(HEADER_SIZE) as *const ArrayHeader;
    let ht = (*h).hash_table as *const HashIndex;
    if ht.is_null() {
        std::ptr::null_mut()
    } else {
        (*ht).temp_key
    }
}

pub unsafe fn arena_chain_len(a: *const Arena) -> usize {
    let mut n = 0usize;
    let mut x = (*a).storage;
    while !x.is_null() {
        n += 1;
        x = (*x).next;
        assert!(n < 100_000, "arena chain does not terminate");
    }
    n
}

/// Scalar shape of a `stbds_string_arena` (pointer values excluded).
pub unsafe fn dump_arena(a: *const Arena) -> Vec<u8> {
    let mut o = Vec::new();
    o.push((*a).storage.is_null() as u8);
    push_usize(&mut o, (*a).remaining);
    o.push((*a).block);
    o.push((*a).mode);
    push_usize(&mut o, arena_chain_len(a));
    o
}

/// Where did `stbds_stralloc` put the string? Fully determined by the C code,
/// and independent of the actual addresses.
///  0 = carved out of the head block (`head->storage + remaining`)
///  1 = the head block's base (over-sized allocation on an empty arena)
///  2 = the block spliced in behind the head (over-sized on a non-empty arena)
///  3 = anything else (would be a bug)
pub unsafe fn stralloc_class(a: *const Arena, p: *const c_char) -> u8 {
    if p.is_null() {
        return 255;
    }
    let head = (*a).storage;
    if head.is_null() {
        return 3;
    }
    let base = (&(*head).storage) as *const c_char;
    if p == base.add((*a).remaining) {
        return 0;
    }
    if p == base {
        return 1;
    }
    let nxt = (*head).next;
    if !nxt.is_null() && p == (&(*nxt).storage) as *const c_char {
        return 2;
    }
    3
}

// ---------------------------------------------------------------------------
// A map driven through one Api, mirroring what the stb_ds macros do
// ---------------------------------------------------------------------------

pub struct Map<'a> {
    pub api: &'a Api,
    /// the "hash pointer" the macros keep (`arr + elemsize`), or NULL
    pub t: *mut u8,
    pub elemsize: usize,
    pub kk: KeyKind,
}

impl<'a> Map<'a> {
    pub fn new(api: &'a Api, elemsize: usize, kk: KeyKind) -> Map<'a> {
        Map { api, t: std::ptr::null_mut(), elemsize, kk }
    }

    /// map created up-front by `stbds_shmode_func` (the `sh_new_arena` /
    /// `sh_new_strdup` idiom)
    pub fn new_shmode(api: &'a Api, elemsize: usize, kk: KeyKind, mode: c_int) -> Map<'a> {
        let t = unsafe { (api.shmode_func)(elemsize, mode) } as *mut u8;
        Map { api, t, elemsize, kk }
    }

    pub fn arr(&self) -> *mut u8 {
        if self.t.is_null() {
            std::ptr::null_mut()
        } else {
            unsafe { self.t.sub(self.elemsize) }
        }
    }

    pub unsafe fn header(&self) -> *mut ArrayHeader {
        self.arr().sub(HEADER_SIZE) as *mut ArrayHeader
    }

    pub unsafe fn temp(&self) -> isize {
        (*self.header()).temp
    }

    pub unsafe fn len(&self) -> isize {
        if self.t.is_null() {
            0
        } else {
            (*self.header()).length as isize - 1
        }
    }

    /// element `i` of the map (`t[i]`)
    pub unsafe fn elem(&self, i: isize) -> *mut u8 {
        self.t.offset(i * self.elemsize as isize)
    }

    /// Write the caller-owned part of the element the macros would write
    /// (`.value = v`, plus `.key = k` for binary maps is already done by the
    /// library). Every byte the library leaves uninitialised is filled
    /// deterministically so that raw comparisons are meaningful.
    pub unsafe fn fill_value(&self, i: isize, keysize: usize, pat: u8) {
        let p = self.elem(i);
        let start = match self.kk {
            KeyKind::Raw => keysize,
            KeyKind::StrPtr => 8,
        };
        for j in start..self.elemsize {
            *p.add(j) = pat.wrapping_add(j as u8);
        }
    }

    pub unsafe fn put(&mut self, key: *mut u8, keysize: usize, mode: c_int, pat: u8) -> isize {
        self.t = (self.api.hmput_key)(
            self.t as *mut c_void,
            self.elemsize,
            key as *mut c_void,
            keysize,
            mode,
        ) as *mut u8;
        let i = self.temp();
        self.fill_value(i, keysize, pat);
        i
    }

    pub unsafe fn get(&mut self, key: *mut u8, keysize: usize, mode: c_int) -> isize {
        self.t = (self.api.hmget_key)(
            self.t as *mut c_void,
            self.elemsize,
            key as *mut c_void,
            keysize,
            mode,
        ) as *mut u8;
        self.temp()
    }

    pub unsafe fn get_ts(&mut self, key: *mut u8, keysize: usize, mode: c_int) -> isize {
        let mut tmp: isize = 0x5A5A_5A5A;
        self.t = (self.api.hmget_key_ts)(
            self.t as *mut c_void,
            self.elemsize,
            key as *mut c_void,
            keysize,
            &mut tmp,
            mode,
        ) as *mut u8;
        tmp
    }

    /// returns the value the `stbds_hmdel` macro would yield
    /// (`(t) ? stbds_temp(t-1) : 0`)
    pub unsafe fn del(&mut self, key: *mut u8, keysize: usize, keyoffset: usize, mode: c_int) -> isize {
        self.t = (self.api.hmdel_key)(
            self.t as *mut c_void,
            self.elemsize,
            key as *mut c_void,
            keysize,
            keyoffset,
            mode,
        ) as *mut u8;
        if self.t.is_null() {
            0
        } else {
            self.temp()
        }
    }

    pub unsafe fn put_default(&mut self) {
        self.t = (self.api.hmput_default)(self.t as *mut c_void, self.elemsize) as *mut u8;
    }

    pub unsafe fn dump(&self) -> Vec<u8> {
        dump_map(self.t, self.elemsize, self.kk)
    }

    pub unsafe fn free(&mut self) {
        if !self.t.is_null() {
            (self.api.hmfree_func)(self.arr() as *mut c_void, self.elemsize);
            self.t = std::ptr::null_mut();
        }
    }
}

/// Assert the two dumps are identical, with a readable diff.
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
            "{what}: C and Rust differ\n  C   len={} \n  RUST len={}\n  first differing byte at {}\n  C   [{}..]: {:02x?}\n  RUST[{}..]: {:02x?}",
            c.len(),
            r.len(),
            first,
            first,
            &c[first..(first + 48).min(c.len())],
            first,
            &r[first..(first + 48).min(r.len())],
        );
    }
}
