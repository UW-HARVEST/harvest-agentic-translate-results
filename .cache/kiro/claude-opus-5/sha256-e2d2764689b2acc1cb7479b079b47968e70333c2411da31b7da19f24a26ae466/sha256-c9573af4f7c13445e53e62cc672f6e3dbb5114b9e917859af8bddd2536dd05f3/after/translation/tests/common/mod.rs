//! Shared harness: loads BOTH the C `.so` and the Rust `.so` with `libloading`
//! and calls every function through its exported symbol only.

#![allow(dead_code)]

use std::ffi::{c_char, c_int, c_void};
use std::path::PathBuf;

// ---------------------------------------------------------------------------
// C data layouts (must match c_src/src/lib.c)
// ---------------------------------------------------------------------------

#[repr(C)]
#[derive(Clone, Copy)]
pub struct ArrHeader {
    pub length: usize,
    pub capacity: usize,
    pub hash_table: *mut c_void,
    pub temp: isize,
}

pub const HEADER_SIZE: usize = std::mem::size_of::<ArrHeader>();

#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct HashBucket {
    pub hash: [usize; 8],
    pub index: [isize; 8],
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct StringArena {
    pub storage: *mut c_void,
    pub remaining: usize,
    pub block: u8,
    pub mode: u8,
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

pub const STBDS_HM_BINARY: c_int = 0;
pub const STBDS_HM_STRING: c_int = 1;
pub const STBDS_SH_NONE: c_int = 0;
pub const STBDS_SH_DEFAULT: c_int = 1;
pub const STBDS_SH_STRDUP: c_int = 2;
pub const STBDS_SH_ARENA: c_int = 3;

// ---------------------------------------------------------------------------
// FFI signatures
// ---------------------------------------------------------------------------

pub type FnArrGrowf = unsafe extern "C" fn(*mut c_void, usize, usize, usize) -> *mut c_void;
pub type FnArrFreef = unsafe extern "C" fn(*mut c_void);
pub type FnHashBytes = unsafe extern "C" fn(*mut c_void, usize, usize) -> usize;
pub type FnHashString = unsafe extern "C" fn(*mut c_char, usize) -> usize;
pub type FnRandSeed = unsafe extern "C" fn(usize);
pub type FnHmFreeFunc = unsafe extern "C" fn(*mut c_void, usize);
pub type FnHmGetKey = unsafe extern "C" fn(*mut c_void, usize, *mut c_void, usize, c_int) -> *mut c_void;
pub type FnHmGetKeyTs =
    unsafe extern "C" fn(*mut c_void, usize, *mut c_void, usize, *mut isize, c_int) -> *mut c_void;
pub type FnHmPutDefault = unsafe extern "C" fn(*mut c_void, usize) -> *mut c_void;
pub type FnHmPutKey = unsafe extern "C" fn(*mut c_void, usize, *mut c_void, usize, c_int) -> *mut c_void;
pub type FnHmDelKey =
    unsafe extern "C" fn(*mut c_void, usize, *mut c_void, usize, usize, c_int) -> *mut c_void;
pub type FnShModeFunc = unsafe extern "C" fn(usize, c_int) -> *mut c_void;
pub type FnStrAlloc = unsafe extern "C" fn(*mut c_void, *mut c_char) -> *mut c_char;
pub type FnStrReset = unsafe extern "C" fn(*mut c_void);
pub type FnStrKey = unsafe extern "C" fn(c_int) -> *mut c_char;
pub type FnArrPush = unsafe extern "C" fn(c_int);

pub struct Lib {
    pub name: &'static str,
    _lib: &'static libloading::Library,
    pub arrgrowf: FnArrGrowf,
    pub arrfreef: FnArrFreef,
    pub hash_bytes: FnHashBytes,
    pub hash_string: FnHashString,
    pub rand_seed: FnRandSeed,
    pub hmfree_func: FnHmFreeFunc,
    pub hmget_key: FnHmGetKey,
    pub hmget_key_ts: FnHmGetKeyTs,
    pub hmput_default: FnHmPutDefault,
    pub hmput_key: FnHmPutKey,
    pub hmdel_key: FnHmDelKey,
    pub shmode_func: FnShModeFunc,
    pub stralloc: FnStrAlloc,
    pub strreset: FnStrReset,
    pub strkey: FnStrKey,
    pub arr_push: FnArrPush,
}

macro_rules! sym {
    ($lib:expr, $name:literal, $t:ty) => {{
        let s: libloading::Symbol<$t> = unsafe {
            $lib.get(concat!($name, "\0").as_bytes())
                .unwrap_or_else(|e| panic!("missing symbol {}: {}", $name, e))
        };
        unsafe { *s.into_raw() }
    }};
}

impl Lib {
    fn open(name: &'static str, path: &PathBuf) -> Lib {
        let lib: &'static libloading::Library = Box::leak(Box::new(unsafe {
            libloading::Library::new(path).unwrap_or_else(|e| panic!("dlopen {:?}: {}", path, e))
        }));
        Lib {
            name,
            arrgrowf: sym!(lib, "stbds_arrgrowf", FnArrGrowf),
            arrfreef: sym!(lib, "stbds_arrfreef", FnArrFreef),
            hash_bytes: sym!(lib, "stbds_hash_bytes", FnHashBytes),
            hash_string: sym!(lib, "stbds_hash_string", FnHashString),
            rand_seed: sym!(lib, "stbds_rand_seed", FnRandSeed),
            hmfree_func: sym!(lib, "stbds_hmfree_func", FnHmFreeFunc),
            hmget_key: sym!(lib, "stbds_hmget_key", FnHmGetKey),
            hmget_key_ts: sym!(lib, "stbds_hmget_key_ts", FnHmGetKeyTs),
            hmput_default: sym!(lib, "stbds_hmput_default", FnHmPutDefault),
            hmput_key: sym!(lib, "stbds_hmput_key", FnHmPutKey),
            hmdel_key: sym!(lib, "stbds_hmdel_key", FnHmDelKey),
            shmode_func: sym!(lib, "stbds_shmode_func", FnShModeFunc),
            stralloc: sym!(lib, "stbds_stralloc", FnStrAlloc),
            strreset: sym!(lib, "stbds_strreset", FnStrReset),
            strkey: sym!(lib, "strkey", FnStrKey),
            arr_push: sym!(lib, "arr_push", FnArrPush),
            _lib: lib,
        }
    }
}

fn manifest() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn find_c_so() -> PathBuf {
    let build = manifest().parent().unwrap().join("c_src/build");
    let mut found = None;
    for e in std::fs::read_dir(&build)
        .unwrap_or_else(|e| panic!("c_src/build not built ({e}); run cmake first"))
    {
        let p = e.unwrap().path();
        if p.extension().map(|x| x == "so").unwrap_or(false) {
            found = Some(p);
        }
    }
    found.unwrap_or_else(|| panic!("no .so in {:?}", build))
}

fn find_rust_so() -> PathBuf {
    if let Ok(p) = std::env::var("RUST_SO") {
        return PathBuf::from(p);
    }
    // The test binary lives in target/<profile>/deps/.  IMPORTANT: `cargo test`
    // rebuilds the cdylib into deps/ but only `cargo build` refreshes the
    // "uplifted" copy at target/<profile>/, so that copy can be stale.  Collect
    // every candidate and take the most recently modified one, otherwise the
    // differential tests can silently compare against an old .so.
    let exe = std::env::current_exe().expect("current_exe");
    let cands = [
        exe.parent().map(|p| p.join("libarr_push_lib.so")),
        exe.parent().and_then(|p| p.parent()).map(|p| p.join("libarr_push_lib.so")),
        Some(manifest().join("target/debug/deps/libarr_push_lib.so")),
        Some(manifest().join("target/debug/libarr_push_lib.so")),
        Some(manifest().join("target/release/deps/libarr_push_lib.so")),
        Some(manifest().join("target/release/libarr_push_lib.so")),
    ];
    let mut best: Option<(std::time::SystemTime, PathBuf)> = None;
    for c in cands.into_iter().flatten() {
        if let Ok(md) = std::fs::metadata(&c) {
            let t = md.modified().unwrap_or(std::time::SystemTime::UNIX_EPOCH);
            if best.as_ref().map(|(bt, _)| t > *bt).unwrap_or(true) {
                best = Some((t, c));
            }
        }
    }
    match best {
        Some((_, p)) => p,
        None => panic!("libarr_push_lib.so not found; run `cargo build` first"),
    }
}

/// Both libraries, opened once per test process.
pub struct Pair {
    pub c: Lib,
    pub r: Lib,
}

pub fn libs() -> &'static Pair {
    use std::sync::OnceLock;
    static P: OnceLock<Pair> = OnceLock::new();
    P.get_or_init(|| Pair {
        c: Lib::open("C", &find_c_so()),
        r: Lib::open("Rust", &find_rust_so()),
    })
}

/// Reset the global `stbds_hash_seed` in BOTH libraries so their per-table
/// seed LCGs stay in lock-step.
pub fn reset_seeds(p: &Pair, seed: usize) {
    unsafe {
        (p.c.rand_seed)(seed);
        (p.r.rand_seed)(seed);
    }
}

// ---------------------------------------------------------------------------
// Deterministic RNG (splitmix64) - fixed seed, reproducible
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
        if n == 0 { 0 } else { (self.next_u64() % n as u64) as usize }
    }
    pub fn range(&mut self, lo: usize, hi: usize) -> usize {
        lo + self.below(hi - lo + 1)
    }
    pub fn byte(&mut self) -> u8 {
        (self.next_u64() >> 17) as u8
    }
    pub fn bytes(&mut self, n: usize) -> Vec<u8> {
        (0..n).map(|_| self.byte()).collect()
    }
    pub fn bool(&mut self) -> bool {
        self.next_u64() & 1 == 1
    }
}

// ---------------------------------------------------------------------------
// Raw pointer helpers replicating the C macros on the *caller* side
// ---------------------------------------------------------------------------

/// `stbds_header(t)`
pub unsafe fn header(t: *mut u8) -> *mut ArrHeader {
    unsafe { (t as *mut ArrHeader).offset(-1) }
}

/// `stbds_temp((t)-1)` for a hash-view pointer `t` with element size `elemsize`
pub unsafe fn hm_temp(t: *mut u8, elemsize: usize) -> isize {
    unsafe { (*header(t.sub(elemsize))).temp }
}

/// `stbds_header((t)-1)->length` for a hash-view pointer
pub unsafe fn hm_raw_len(t: *mut u8, elemsize: usize) -> usize {
    unsafe { (*header(t.sub(elemsize))).length }
}

/// `stbds_hmlen(t)` = length-1
pub unsafe fn hm_len(t: *mut u8, elemsize: usize) -> isize {
    if t.is_null() {
        0
    } else {
        unsafe { (*header(t.sub(elemsize))).length as isize - 1 }
    }
}

pub unsafe fn hm_table(t: *mut u8, elemsize: usize) -> *mut HashIndex {
    unsafe { (*header(t.sub(elemsize))).hash_table as *mut HashIndex }
}

/// `stbds_temp_key((t)-1)` — only meaningful right after a string-mode insert
/// or a string-mode "found existing in first scan" hit.
pub unsafe fn table_temp_key(t: *mut u8, elemsize: usize) -> Option<Vec<u8>> {
    unsafe {
        let tbl = hm_table(t, elemsize);
        if tbl.is_null() || (*tbl).temp_key.is_null() {
            None
        } else {
            Some(cstr((*tbl).temp_key))
        }
    }
}

// ---------------------------------------------------------------------------
// Snapshots: structural, pointer-value-independent representations
// ---------------------------------------------------------------------------

#[derive(PartialEq, Eq, Debug, Clone, Copy)]
pub enum KeyRepr {
    /// Element bytes compared verbatim (binary maps, and string maps whose keys
    /// are caller-owned pointers we pass identically to both libraries).
    Raw,
    /// A `char *` lives at `off`; compare the pointed-to string, not the pointer.
    PtrString { off: usize },
}

#[derive(PartialEq, Eq, Debug)]
pub struct ArenaSnap {
    pub storage_null: bool,
    pub remaining: usize,
    pub block: u8,
    pub mode: u8,
}

#[derive(PartialEq, Eq, Debug)]
pub struct TableSnap {
    // NOTE: `temp_key` is deliberately NOT part of the snapshot.
    // `stbds_make_hash_index` never initialises it, so it holds indeterminate
    // heap bytes until `stbds_temp_key()` writes it.  It is compared
    // explicitly, via `table_temp_key`, only at points where the C source
    // provably wrote it.
    pub slot_count: usize,
    pub used_count: usize,
    pub used_count_threshold: usize,
    pub used_count_shrink_threshold: usize,
    pub tombstone_count: usize,
    pub tombstone_count_threshold: usize,
    pub seed: usize,
    pub slot_count_log2: usize,
    pub string: ArenaSnap,
    pub buckets: Vec<HashBucket>,
}

#[derive(PartialEq, Eq, Debug)]
pub struct MapSnap {
    pub null: bool,
    pub length: usize,
    pub capacity: usize,
    pub temp: isize,
    /// One entry per element; keys normalised per `KeyRepr`.
    pub elems: Vec<Vec<u8>>,
    pub table: Option<TableSnap>,
}

pub unsafe fn cstr(p: *const c_char) -> Vec<u8> {
    unsafe {
        let mut v = Vec::new();
        let mut i = 0;
        loop {
            let b = *(p.add(i) as *const u8);
            if b == 0 {
                break;
            }
            v.push(b);
            i += 1;
        }
        v
    }
}

unsafe fn arena_snap(a: &StringArena) -> ArenaSnap {
    ArenaSnap {
        storage_null: a.storage.is_null(),
        remaining: a.remaining,
        block: a.block,
        mode: a.mode,
    }
}

/// Snapshot a hash-map pointer as returned by `stbds_hmput_key` & friends.
pub unsafe fn snap_map(t: *mut u8, elemsize: usize, kr: KeyRepr) -> MapSnap {
    unsafe {
        if t.is_null() {
            return MapSnap {
                null: true,
                length: 0,
                capacity: 0,
                temp: 0,
                elems: Vec::new(),
                table: None,
            };
        }
        let raw = t.sub(elemsize);
        let h = *header(raw);
        let mut elems = Vec::with_capacity(h.length);
        for i in 0..h.length {
            let e = raw.add(i * elemsize);
            let mut bytes = std::slice::from_raw_parts(e, elemsize).to_vec();
            if let KeyRepr::PtrString { off } = kr {
                let pp = *(e.add(off) as *const *const c_char);
                // Replace the pointer bytes with a normalised marker + the
                // string it points at.
                for b in &mut bytes[off..off + 8] {
                    *b = 0;
                }
                let mut norm = bytes;
                if pp.is_null() {
                    norm.push(0xAA);
                } else {
                    norm.push(0xBB);
                    norm.extend_from_slice(&cstr(pp));
                }
                elems.push(norm);
            } else {
                elems.push(bytes);
            }
        }
        let tp = h.hash_table as *mut HashIndex;
        let table = if tp.is_null() {
            None
        } else {
            let ti = &*tp;
            let nb = ti.slot_count >> 3;
            let mut buckets = Vec::with_capacity(nb);
            for i in 0..nb {
                buckets.push(*ti.storage.add(i));
            }
            Some(TableSnap {
                slot_count: ti.slot_count,
                used_count: ti.used_count,
                used_count_threshold: ti.used_count_threshold,
                used_count_shrink_threshold: ti.used_count_shrink_threshold,
                tombstone_count: ti.tombstone_count,
                tombstone_count_threshold: ti.tombstone_count_threshold,
                seed: ti.seed,
                slot_count_log2: ti.slot_count_log2,
                string: arena_snap(&ti.string),
                buckets,
            })
        };
        MapSnap {
            null: false,
            length: h.length,
            capacity: h.capacity,
            temp: h.temp,
            elems,
            table,
        }
    }
}

/// Snapshot a plain dynamic array (`stbds_arrgrowf` result).
#[derive(PartialEq, Eq, Debug)]
pub struct ArrSnap {
    pub null: bool,
    pub length: usize,
    pub capacity: usize,
    pub temp: isize,
    pub hash_table_null: bool,
    pub elems: Vec<u8>,
}

pub unsafe fn snap_arr(a: *mut u8, elemsize: usize) -> ArrSnap {
    unsafe {
        if a.is_null() {
            return ArrSnap {
                null: true,
                length: 0,
                capacity: 0,
                temp: 0,
                hash_table_null: true,
                elems: Vec::new(),
            };
        }
        let h = *header(a);
        ArrSnap {
            null: false,
            length: h.length,
            capacity: h.capacity,
            temp: h.temp,
            hash_table_null: h.hash_table.is_null(),
            elems: std::slice::from_raw_parts(a, h.length * elemsize).to_vec(),
        }
    }
}

// ---------------------------------------------------------------------------
// Macro-level pipelines (the way a real consumer drives the library)
// ---------------------------------------------------------------------------

/// One hash map, held for a single library.
pub struct Map<'a> {
    pub lib: &'a Lib,
    pub t: *mut u8,
    pub elemsize: usize,
    pub keysize: usize,
    pub value_off: usize,
}

impl<'a> Map<'a> {
    pub fn new(lib: &'a Lib, elemsize: usize, keysize: usize, value_off: usize) -> Map<'a> {
        Map { lib, t: std::ptr::null_mut(), elemsize, keysize, value_off }
    }

    /// `sh_new_strdup` / `sh_new_arena`: `t = stbds_shmode_func(sizeof *t, mode)`
    pub unsafe fn sh_new(&mut self, mode: c_int) {
        unsafe {
            self.t = (self.lib.shmode_func)(self.elemsize, mode) as *mut u8;
        }
    }

    /// `stbds_hmputs(t, s)` in spirit: after `stbds_hmput_key` the *whole*
    /// element is assigned, so no byte of the element is left indeterminate.
    /// `key` occupies `[0, keysize)`, `value` occupies `[value_off, elemsize)`,
    /// and any gap in between is zeroed on both sides.
    pub unsafe fn hmput(&mut self, key: &[u8], value: &[u8], mode: c_int) -> isize {
        unsafe {
            assert_eq!(key.len(), self.keysize, "key must be exactly keysize bytes");
            assert_eq!(
                value.len(),
                self.elemsize - self.value_off,
                "value must fill the element"
            );
            let mut elem = vec![0u8; self.elemsize];
            elem[..self.keysize].copy_from_slice(key);
            elem[self.value_off..].copy_from_slice(value);

            let mut k = key.to_vec();
            self.t = (self.lib.hmput_key)(
                self.t as *mut c_void,
                self.elemsize,
                k.as_mut_ptr() as *mut c_void,
                self.keysize,
                mode,
            ) as *mut u8;
            let idx = hm_temp(self.t, self.elemsize);
            let e = self.t.offset(idx * self.elemsize as isize);
            std::ptr::copy_nonoverlapping(elem.as_ptr(), e, self.elemsize);
            idx
        }
    }

    /// `stbds_shput(t, k, v)`: the key pointer itself is the lookup key.
    pub unsafe fn shput(&mut self, key: *mut c_char, value: &[u8], mode: c_int) -> isize {
        unsafe {
            self.t = (self.lib.hmput_key)(
                self.t as *mut c_void,
                self.elemsize,
                key as *mut c_void,
                self.keysize,
                mode,
            ) as *mut u8;
            let idx = hm_temp(self.t, self.elemsize);
            let e = self.t.offset(idx * self.elemsize as isize);
            std::ptr::copy_nonoverlapping(value.as_ptr(), e.add(self.value_off), value.len());
            idx
        }
    }

    /// `stbds_hmgeti(t, k)` -> index or -1
    pub unsafe fn hmgeti(&mut self, key: &[u8], mode: c_int) -> isize {
        unsafe {
            let mut k = key.to_vec();
            self.t = (self.lib.hmget_key)(
                self.t as *mut c_void,
                self.elemsize,
                k.as_mut_ptr() as *mut c_void,
                self.keysize,
                mode,
            ) as *mut u8;
            hm_temp(self.t, self.elemsize)
        }
    }

    /// `stbds_shgeti(t, k)` -> index or -1
    pub unsafe fn shgeti(&mut self, key: *mut c_char, mode: c_int) -> isize {
        unsafe {
            self.t = (self.lib.hmget_key)(
                self.t as *mut c_void,
                self.elemsize,
                key as *mut c_void,
                self.keysize,
                mode,
            ) as *mut u8;
            hm_temp(self.t, self.elemsize)
        }
    }

    /// `stbds_hmgeti_ts(t, k, temp)` -> temp
    pub unsafe fn hmgeti_ts(&mut self, key: &[u8], mode: c_int) -> isize {
        unsafe {
            let mut k = key.to_vec();
            let mut temp: isize = 0x5555;
            self.t = (self.lib.hmget_key_ts)(
                self.t as *mut c_void,
                self.elemsize,
                k.as_mut_ptr() as *mut c_void,
                self.keysize,
                &mut temp,
                mode,
            ) as *mut u8;
            temp
        }
    }

    /// `stbds_hmdel(t, k)` -> 0/1
    pub unsafe fn hmdel(&mut self, key: &[u8], mode: c_int, keyoffset: usize) -> isize {
        unsafe {
            let mut k = key.to_vec();
            self.t = (self.lib.hmdel_key)(
                self.t as *mut c_void,
                self.elemsize,
                k.as_mut_ptr() as *mut c_void,
                self.keysize,
                keyoffset,
                mode,
            ) as *mut u8;
            if self.t.is_null() { 0 } else { hm_temp(self.t, self.elemsize) }
        }
    }

    /// `stbds_shdel(t, k)` -> 0/1
    pub unsafe fn shdel(&mut self, key: *mut c_char, mode: c_int, keyoffset: usize) -> isize {
        unsafe {
            self.t = (self.lib.hmdel_key)(
                self.t as *mut c_void,
                self.elemsize,
                key as *mut c_void,
                self.keysize,
                keyoffset,
                mode,
            ) as *mut u8;
            if self.t.is_null() { 0 } else { hm_temp(self.t, self.elemsize) }
        }
    }

    pub unsafe fn hmput_default(&mut self) {
        unsafe {
            self.t = (self.lib.hmput_default)(self.t as *mut c_void, self.elemsize) as *mut u8;
        }
    }

    pub unsafe fn snap(&self, kr: KeyRepr) -> MapSnap {
        unsafe { snap_map(self.t, self.elemsize, kr) }
    }

    pub unsafe fn free(&mut self) {
        unsafe {
            if !self.t.is_null() {
                (self.lib.hmfree_func)(self.t.sub(self.elemsize) as *mut c_void, self.elemsize);
                self.t = std::ptr::null_mut();
            }
        }
    }
}

/// Run the same closure against both libraries and compare the results.
pub fn both<T: PartialEq + std::fmt::Debug>(
    p: &Pair,
    seed: usize,
    label: &str,
    mut f: impl FnMut(&Lib) -> T,
) {
    unsafe {
        (p.c.rand_seed)(seed);
    }
    let a = f(&p.c);
    unsafe {
        (p.r.rand_seed)(seed);
    }
    let b = f(&p.r);
    assert_eq!(a, b, "divergence in {label}");
}
