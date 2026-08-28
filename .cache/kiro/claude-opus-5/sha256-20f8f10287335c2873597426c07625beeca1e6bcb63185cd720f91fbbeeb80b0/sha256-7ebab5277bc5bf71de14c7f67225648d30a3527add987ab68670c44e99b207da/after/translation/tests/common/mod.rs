//! Shared harness: loads both the C and the Rust shared objects via
//! `libloading` and exposes the stb_ds public API through function pointers.
//!
//! Nothing here calls into the Rust crate directly -- every call goes through
//! `dlsym` on the freshly built `cdylib`, exactly like an external C caller.

#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::ffi::{c_char, c_int, c_void};
use std::path::PathBuf;

// ---------------------------------------------------------------------------
// C ABI signatures
// ---------------------------------------------------------------------------

pub type ArrGrowf = unsafe extern "C" fn(*mut c_void, usize, usize, usize) -> *mut c_void;
pub type ArrFreef = unsafe extern "C" fn(*mut c_void);
pub type RandSeed = unsafe extern "C" fn(usize);
pub type HashBytes = unsafe extern "C" fn(*mut c_void, usize, usize) -> usize;
pub type HashString = unsafe extern "C" fn(*mut c_char, usize) -> usize;
pub type HmfreeFunc = unsafe extern "C" fn(*mut c_void, usize);
pub type HmgetKey = unsafe extern "C" fn(*mut c_void, usize, *mut c_void, usize, c_int) -> *mut c_void;
pub type HmgetKeyTs =
    unsafe extern "C" fn(*mut c_void, usize, *mut c_void, usize, *mut isize, c_int) -> *mut c_void;
pub type HmputDefault = unsafe extern "C" fn(*mut c_void, usize) -> *mut c_void;
pub type HmputKey = unsafe extern "C" fn(*mut c_void, usize, *mut c_void, usize, c_int) -> *mut c_void;
pub type HmdelKey =
    unsafe extern "C" fn(*mut c_void, usize, *mut c_void, usize, usize, c_int) -> *mut c_void;
pub type ShmodeFunc = unsafe extern "C" fn(usize, c_int) -> *mut c_void;
pub type Stralloc = unsafe extern "C" fn(*mut c_void, *mut c_char) -> *mut c_char;
pub type Strreset = unsafe extern "C" fn(*mut c_void);
pub type Strkey = unsafe extern "C" fn(c_int) -> *mut c_char;
pub type ArrIns = unsafe extern "C" fn(c_int);

// ---------------------------------------------------------------------------
// Mirrors of the C layouts (used only to inspect results)
// ---------------------------------------------------------------------------

pub const HEADER_SIZE: usize = 32; // sizeof(stbds_array_header) on LP64
pub const BUCKET_LENGTH: usize = 8;
pub const BUCKET_SHIFT: usize = 3;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct ArrayHeader {
    pub length: usize,
    pub capacity: usize,
    pub hash_table: *mut c_void,
    pub temp: isize,
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
#[derive(Clone, Copy)]
pub struct HashBucket {
    pub hash: [usize; BUCKET_LENGTH],
    pub index: [isize; BUCKET_LENGTH],
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

pub const HM_BINARY: c_int = 0;
pub const HM_STRING: c_int = 1;

pub const SH_NONE: c_int = 0;
pub const SH_DEFAULT: c_int = 1;
pub const SH_STRDUP: c_int = 2;
pub const SH_ARENA: c_int = 3;

// ---------------------------------------------------------------------------
// Library wrapper
// ---------------------------------------------------------------------------

pub struct Api {
    pub name: &'static str,
    _lib: Library,
    pub arrgrowf: ArrGrowf,
    pub arrfreef: ArrFreef,
    pub rand_seed: RandSeed,
    pub hash_bytes: HashBytes,
    pub hash_string: HashString,
    pub hmfree_func: HmfreeFunc,
    pub hmget_key: HmgetKey,
    pub hmget_key_ts: HmgetKeyTs,
    pub hmput_default: HmputDefault,
    pub hmput_key: HmputKey,
    pub hmdel_key: HmdelKey,
    pub shmode_func: ShmodeFunc,
    pub stralloc: Stralloc,
    pub strreset: Strreset,
    pub strkey: Strkey,
    pub arr_ins: ArrIns,
}

unsafe fn sym<T: Copy>(lib: &Library, name: &str) -> T {
    unsafe {
        let s: Symbol<T> = lib
            .get(format!("{name}\0").as_bytes())
            .unwrap_or_else(|e| panic!("missing symbol {name}: {e}"));
        *s
    }
}

impl Api {
    pub fn load(name: &'static str, path: &PathBuf) -> Api {
        unsafe {
            let lib = Library::new(path)
                .unwrap_or_else(|e| panic!("cannot dlopen {}: {e}", path.display()));
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
                strkey: sym(&lib, "strkey"),
                arr_ins: sym(&lib, "arr_ins"),
                _lib: lib,
            }
        }
    }
}

fn workspace_root() -> PathBuf {
    // CARGO_MANIFEST_DIR == <root>/translation
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.pop();
    p
}

fn find_c_so() -> PathBuf {
    // Allows re-running the whole suite against a differently configured C
    // build (e.g. -O2) without touching c_src.
    if let Ok(p) = std::env::var("HARVEST_C_SO") {
        return PathBuf::from(p);
    }
    let dir = workspace_root().join("c_src/build");
    let mut cands: Vec<PathBuf> = std::fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("c_src/build not built ({e}); run cmake first"))
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().map(|x| x == "so").unwrap_or(false))
        .collect();
    cands.sort();
    cands
        .pop()
        .unwrap_or_else(|| panic!("no .so found in {}", dir.display()))
}

fn find_rust_so() -> PathBuf {
    // The integration-test executable lives in target/<profile>/deps/, so the
    // cdylib sits one directory up.
    let exe = std::env::current_exe().expect("current_exe");
    let deps = exe.parent().expect("deps dir");
    let profile = deps.parent().expect("profile dir");
    for d in [profile, deps] {
        let p = d.join("libarr_ins_lib.so");
        if p.exists() {
            return p;
        }
    }
    panic!("libarr_ins_lib.so not found near {}", profile.display());
}

/// Load both libraries. The C build is the reference.
pub fn both() -> (Api, Api) {
    (
        Api::load("C", &find_c_so()),
        Api::load("Rust", &find_rust_so()),
    )
}

/// Both shared objects keep a process-global hash seed (`stbds_hash_seed`),
/// which `stbds_rand_seed` sets and every new hash table advances. Tests in one
/// integration binary run on multiple threads, so any test that depends on that
/// seed must hold this lock for the whole replay.
pub fn global_lock() -> std::sync::MutexGuard<'static, ()> {
    static M: std::sync::Mutex<()> = std::sync::Mutex::new(());
    M.lock().unwrap_or_else(|e| e.into_inner())
}

// ---------------------------------------------------------------------------
// Snapshotting helpers -- turn opaque results into comparable values
// ---------------------------------------------------------------------------

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct HeaderSnap {
    pub length: usize,
    pub capacity: usize,
    pub has_hash_table: bool,
    pub temp: isize,
}

/// Reads the `stbds_array_header` that sits immediately before `a`.
pub unsafe fn header_snap(a: *mut c_void) -> HeaderSnap {
    unsafe {
        let h = (a as *mut ArrayHeader).sub(1);
        HeaderSnap {
            length: (*h).length,
            capacity: (*h).capacity,
            has_hash_table: !(*h).hash_table.is_null(),
            temp: (*h).temp,
        }
    }
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
    pub arena_remaining: usize,
    pub arena_block: u8,
    pub arena_mode: u8,
    pub arena_has_storage: bool,
    pub buckets: Vec<([usize; BUCKET_LENGTH], [isize; BUCKET_LENGTH])>,
}

pub unsafe fn table_snap(t: *mut HashIndex) -> Option<TableSnap> {
    unsafe {
        if t.is_null() {
            return None;
        }
        let n = (*t).slot_count >> BUCKET_SHIFT;
        let mut buckets = Vec::with_capacity(n);
        for i in 0..n {
            let b = (*t).storage.add(i);
            buckets.push(((*b).hash, (*b).index));
        }
        Some(TableSnap {
            slot_count: (*t).slot_count,
            used_count: (*t).used_count,
            used_count_threshold: (*t).used_count_threshold,
            used_count_shrink_threshold: (*t).used_count_shrink_threshold,
            tombstone_count: (*t).tombstone_count,
            tombstone_count_threshold: (*t).tombstone_count_threshold,
            seed: (*t).seed,
            slot_count_log2: (*t).slot_count_log2,
            arena_remaining: (*t).string.remaining,
            arena_block: (*t).string.block,
            arena_mode: (*t).string.mode,
            arena_has_storage: !(*t).string.storage.is_null(),
            buckets,
        })
    }
}

/// Full snapshot of a hash-map handle (`t`, i.e. the pointer users hold, which
/// is one element past the array base).
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct MapSnap {
    pub is_null: bool,
    pub header: Option<HeaderSnap>,
    pub table: Option<TableSnap>,
    /// Element bytes for slots `0..length`, restricted to the ranges the caller
    /// declared as defined (padding is skipped).
    pub elems: Vec<Vec<u8>>,
    /// Key strings for string-keyed maps.
    pub keys: Vec<Option<String>>,
    /// Whether `table->temp_key` equals the key pointer of the element that
    /// `header->temp` selects. `stbds_make_hash_index` leaves `temp_key`
    /// uninitialised, so it is never dereferenced here -- only compared.
    pub temp_key_is_elem_key: Option<bool>,
}

pub unsafe fn c_string(p: *const c_char) -> Option<String> {
    unsafe {
        if p.is_null() {
            None
        } else {
            Some(std::ffi::CStr::from_ptr(p).to_string_lossy().into_owned())
        }
    }
}

/// `string_key`: when true, the first pointer-sized field of each element is a
/// `char *` and is compared by pointee rather than by value.
/// `ranges`: byte ranges within an element that hold defined (non-padding)
/// data and are therefore safe to compare verbatim.
pub unsafe fn map_snap(
    t: *mut c_void,
    elemsize: usize,
    string_key: bool,
    ranges: &[(usize, usize)],
) -> MapSnap {
    unsafe {
        if t.is_null() {
            return MapSnap {
                is_null: true,
                header: None,
                table: None,
                elems: Vec::new(),
                keys: Vec::new(),
                temp_key_is_elem_key: None,
            };
        }
        let raw_a = (t as *mut u8).sub(elemsize) as *mut c_void;
        let header = header_snap(raw_a);
        let table = (raw_a as *mut ArrayHeader).sub(1);
        let tbl = (*table).hash_table as *mut HashIndex;
        let tsnap = table_snap(tbl);

        let mut elems = Vec::new();
        let mut keys = Vec::new();
        for i in 0..header.length {
            let e = (raw_a as *mut u8).add(elemsize * i);
            let mut bytes = Vec::new();
            for &(off, len) in ranges {
                bytes.extend_from_slice(std::slice::from_raw_parts(e.add(off), len));
            }
            elems.push(bytes);
            if string_key && i != 0 {
                // slot 0 is the default element; its key is NULL
                keys.push(c_string(*(e as *mut *mut c_char)));
            } else {
                keys.push(None);
            }
        }

        // `temp` indexes `t`, i.e. element `temp + 1` of the raw array.
        let temp_key_is_elem_key = if string_key
            && !tbl.is_null()
            && header.temp >= 0
            && (header.temp as usize) + 1 < header.length
        {
            let e = (t as *mut u8).offset(elemsize as isize * header.temp);
            Some((*tbl).temp_key == *(e as *mut *mut c_char))
        } else {
            None
        };

        MapSnap {
            is_null: false,
            header: Some(header),
            table: tsnap,
            elems,
            keys,
            temp_key_is_elem_key,
        }
    }
}
