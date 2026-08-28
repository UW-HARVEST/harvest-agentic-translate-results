//! Shared harness: loads the C `.so` and the Rust `.so` via `libloading` and
//! exposes the exported stb_ds symbols as plain function pointers so that both
//! implementations are exercised strictly through their FFI boundary.

#![allow(dead_code)]
#![allow(non_snake_case)]

use libloading::{Library, Symbol};
use std::ffi::{c_char, c_int, c_void};
use std::path::PathBuf;

pub type FnArrGrowf = unsafe extern "C" fn(*mut c_void, usize, usize, usize) -> *mut c_void;
pub type FnArrFreef = unsafe extern "C" fn(*mut c_void);
pub type FnRandSeed = unsafe extern "C" fn(usize);
pub type FnHashBytes = unsafe extern "C" fn(*mut c_void, usize, usize) -> usize;
pub type FnHashString = unsafe extern "C" fn(*mut c_char, usize) -> usize;
pub type FnHmFreeFunc = unsafe extern "C" fn(*mut c_void, usize);
pub type FnHmGetKey = unsafe extern "C" fn(*mut c_void, usize, *mut c_void, usize, c_int) -> *mut c_void;
pub type FnHmGetKeyTs =
    unsafe extern "C" fn(*mut c_void, usize, *mut c_void, usize, *mut isize, c_int) -> *mut c_void;
pub type FnHmPutDefault = unsafe extern "C" fn(*mut c_void, usize) -> *mut c_void;
pub type FnHmPutKey = unsafe extern "C" fn(*mut c_void, usize, *mut c_void, usize, c_int) -> *mut c_void;
pub type FnHmDelKey =
    unsafe extern "C" fn(*mut c_void, usize, *mut c_void, usize, usize, c_int) -> *mut c_void;
pub type FnShmodeFunc = unsafe extern "C" fn(usize, c_int) -> *mut c_void;
pub type FnStralloc = unsafe extern "C" fn(*mut c_void, *mut c_char) -> *mut c_char;
pub type FnStrreset = unsafe extern "C" fn(*mut c_void);
pub type FnStrkey = unsafe extern "C" fn(c_int) -> *mut c_char;
pub type FnArrDel = unsafe extern "C" fn(c_int);

pub struct Lib {
    pub name: &'static str,
    _lib: &'static Library,
    pub arrgrowf: FnArrGrowf,
    pub arrfreef: FnArrFreef,
    pub rand_seed: FnRandSeed,
    pub hash_bytes: FnHashBytes,
    pub hash_string: FnHashString,
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

unsafe fn sym<T: Copy>(lib: &'static Library, name: &[u8]) -> T {
    let s: Symbol<T> = lib
        .get(name)
        .unwrap_or_else(|e| panic!("missing symbol {}: {e}", String::from_utf8_lossy(name)));
    *s
}

impl Lib {
    pub fn open(name: &'static str, path: &PathBuf) -> Lib {
        let lib: &'static Library = Box::leak(Box::new(unsafe {
            Library::new(path).unwrap_or_else(|e| panic!("cannot load {}: {e}", path.display()))
        }));
        unsafe {
            Lib {
                name,
                _lib: lib,
                arrgrowf: sym(lib, b"stbds_arrgrowf\0"),
                arrfreef: sym(lib, b"stbds_arrfreef\0"),
                rand_seed: sym(lib, b"stbds_rand_seed\0"),
                hash_bytes: sym(lib, b"stbds_hash_bytes\0"),
                hash_string: sym(lib, b"stbds_hash_string\0"),
                hmfree_func: sym(lib, b"stbds_hmfree_func\0"),
                hmget_key: sym(lib, b"stbds_hmget_key\0"),
                hmget_key_ts: sym(lib, b"stbds_hmget_key_ts\0"),
                hmput_default: sym(lib, b"stbds_hmput_default\0"),
                hmput_key: sym(lib, b"stbds_hmput_key\0"),
                hmdel_key: sym(lib, b"stbds_hmdel_key\0"),
                shmode_func: sym(lib, b"stbds_shmode_func\0"),
                stralloc: sym(lib, b"stbds_stralloc\0"),
                strreset: sym(lib, b"stbds_strreset\0"),
                strkey: sym(lib, b"strkey\0"),
                arr_del: sym(lib, b"arr_del\0"),
            }
        }
    }
}

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("manifest dir has parent")
        .to_path_buf()
}

/// `c_src/build/lib<projectname>.so` – the project name is derived from the
/// working-directory name by CMakeLists.txt, so discover it by globbing.
pub fn c_so_path() -> PathBuf {
    let dir = repo_root().join("c_src").join("build");
    let mut found: Vec<PathBuf> = std::fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("{} not built: {e}", dir.display()))
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().map(|x| x == "so").unwrap_or(false))
        .collect();
    found.sort();
    assert_eq!(found.len(), 1, "expected exactly one .so in {}", dir.display());
    found.pop().unwrap()
}

/// The Rust cdylib. `cargo test` only builds the `rlib` for the test harness,
/// so build the `cdylib` on demand (once per test binary) and load that.
pub fn rust_so_path() -> PathBuf {
    static SO: std::sync::OnceLock<PathBuf> = std::sync::OnceLock::new();
    SO.get_or_init(|| {
        let exe = std::env::current_exe().expect("current_exe");
        let profile_dir = exe
            .parent()
            .and_then(|p| p.parent())
            .expect("target/<profile>")
            .to_path_buf();
        let release = profile_dir
            .file_name()
            .map(|n| n == "release")
            .unwrap_or(false);

        let mut cmd = std::process::Command::new(env!("CARGO"));
        cmd.current_dir(env!("CARGO_MANIFEST_DIR")).arg("build").arg("--lib");
        if release {
            cmd.arg("--release");
        }
        // Cargo passes its own build config through the environment; drop the
        // variables that would confuse a nested invocation.
        for k in ["RUSTC", "RUSTDOC", "RUSTC_WORKSPACE_WRAPPER", "CARGO_MAKEFLAGS"] {
            cmd.env_remove(k);
        }
        let out = cmd.output().expect("failed to spawn cargo build --lib");
        assert!(
            out.status.success(),
            "cargo build --lib failed:\n{}",
            String::from_utf8_lossy(&out.stderr)
        );

        let p = profile_dir.join("libarr_del_lib.so");
        assert!(p.exists(), "rust cdylib not found at {}", p.display());
        p
    })
    .clone()
}

pub fn both() -> (Lib, Lib) {
    (
        Lib::open("C", &c_so_path()),
        Lib::open("Rust", &rust_so_path()),
    )
}

/// Both shared objects keep a process-global `stbds_hash_seed` that every fresh
/// hash index consumes and advances. `dlopen` hands every test thread the same
/// handle, so any test that creates a hash index must hold this lock to keep the
/// two libraries' seed sequences in lock-step.
pub fn serial() -> std::sync::MutexGuard<'static, ()> {
    static LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
    LOCK.lock().unwrap_or_else(|e| e.into_inner())
}

// ---------------------------------------------------------------------------
// mirrors of the C internal structures, used only for state comparison
// ---------------------------------------------------------------------------

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct ArrayHeader {
    pub length: usize,
    pub capacity: usize,
    pub hash_table: *mut c_void,
    pub temp: isize,
}

pub const BUCKET_LENGTH: usize = 8;
pub const BUCKET_SHIFT: usize = 3;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct HashBucket {
    pub hash: [usize; BUCKET_LENGTH],
    pub index: [isize; BUCKET_LENGTH],
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

/// `stbds_header(t)` – uses wrapping arithmetic so that passing a null pointer
/// merely yields a bogus pointer (as in C) instead of tripping Rust's
/// `ptr::offset` precondition check.
pub unsafe fn header(t: *mut u8) -> *mut ArrayHeader {
    t.wrapping_sub(std::mem::size_of::<ArrayHeader>()) as *mut ArrayHeader
}

/// A pointer-free, comparable snapshot of an stb_ds hash map / array.
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Snapshot {
    pub null: bool,
    pub length: usize,
    pub capacity: usize,
    pub temp: isize,
    pub elems: Vec<u8>,
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
    pub arena_storage_null: bool,
    pub buckets: Vec<(usize, isize)>,
    /// Key strings, only populated for string-keyed maps.
    pub keys: Vec<Vec<u8>>,
}

pub unsafe fn read_cstr(p: *const c_char) -> Vec<u8> {
    let mut v = Vec::new();
    let mut i = 0isize;
    while *p.offset(i) != 0 {
        v.push(*p.offset(i) as u8);
        i += 1;
    }
    v
}

/// Snapshot the map addressed by the "hash side" pointer `t` (as returned by
/// `stbds_hmput_key` & friends). `string_keys` selects whether element slot 0
/// of each entry is a `char *` that should be dereferenced for comparison.
pub unsafe fn snapshot(t: *mut u8, elemsize: usize, string_keys: bool) -> Snapshot {
    let mut s = Snapshot {
        null: t.is_null(),
        length: 0,
        capacity: 0,
        temp: 0,
        elems: Vec::new(),
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
        arena_storage_null: true,
        buckets: Vec::new(),
        keys: Vec::new(),
    };
    if t.is_null() {
        return s;
    }
    let raw = t.wrapping_sub(elemsize);
    let h = header(raw);
    s.length = (*h).length;
    s.capacity = (*h).capacity;
    s.temp = (*h).temp;

    // element bytes; for string keys the first 8 bytes are a pointer, so read
    // the payload after the key and the key text separately.
    for i in 0..s.length {
        let e = raw.add(elemsize * i);
        if string_keys {
            let kp = *(e as *mut *mut c_char);
            if i == 0 || kp.is_null() {
                s.keys.push(Vec::new());
            } else {
                s.keys.push(read_cstr(kp));
            }
            s.elems
                .extend_from_slice(std::slice::from_raw_parts(e.add(8), elemsize - 8));
        } else {
            s.elems
                .extend_from_slice(std::slice::from_raw_parts(e, elemsize));
        }
    }

    let table = (*h).hash_table as *mut HashIndex;
    if table.is_null() {
        return s;
    }
    s.has_table = true;
    s.slot_count = (*table).slot_count;
    s.used_count = (*table).used_count;
    s.used_count_threshold = (*table).used_count_threshold;
    s.used_count_shrink_threshold = (*table).used_count_shrink_threshold;
    s.tombstone_count = (*table).tombstone_count;
    s.tombstone_count_threshold = (*table).tombstone_count_threshold;
    s.seed = (*table).seed;
    s.slot_count_log2 = (*table).slot_count_log2;
    s.arena_remaining = (*table).string.remaining;
    s.arena_block = (*table).string.block;
    s.arena_mode = (*table).string.mode;
    s.arena_storage_null = (*table).string.storage.is_null();
    for i in 0..(s.slot_count >> BUCKET_SHIFT) {
        let b = (*table).storage.add(i);
        for j in 0..BUCKET_LENGTH {
            s.buckets.push(((*b).hash[j], (*b).index[j]));
        }
    }
    s
}

// ---------------------------------------------------------------------------
// re-implementations of the stb_ds macros, driving the loaded .so exports
// ---------------------------------------------------------------------------

pub const HM_BINARY: c_int = 0;
pub const HM_STRING: c_int = 1;
pub const SH_NONE: c_int = 0;
pub const SH_DEFAULT: c_int = 1;
pub const SH_STRDUP: c_int = 2;
pub const SH_ARENA: c_int = 3;

/// `stbds_temp_key((t)-1)` – the `temp_key` field of the attached hash index.
///
/// NOTE: `stbds_make_hash_index` never initialises this field, so it only holds
/// a defined value right after a string-mode `stbds_hmput_key` that reached the
/// `found_empty_slot` path. Callers must only compare it there.
pub unsafe fn temp_key_str(t: *mut u8, elemsize: usize) -> Option<Vec<u8>> {
    if t.is_null() {
        return None;
    }
    let table = (*header(t.wrapping_sub(elemsize))).hash_table as *mut HashIndex;
    if table.is_null() || (*table).temp_key.is_null() {
        return None;
    }
    Some(read_cstr((*table).temp_key))
}

/// `stbds_temp((t)-1)`
pub unsafe fn temp_of(t: *mut u8, elemsize: usize) -> isize {
    (*header(t.wrapping_sub(elemsize))).temp
}

/// `hmput(t, k, v)` for an element layout of `{ KEY key; PAYLOAD value; }`
/// where the key occupies `keysize` leading bytes.
pub unsafe fn hmput_bytes(
    lib: &Lib,
    t: *mut u8,
    elemsize: usize,
    key: &[u8],
    value: &[u8],
) -> *mut u8 {
    let mut k = key.to_vec();
    let t = (lib.hmput_key)(
        t as *mut c_void,
        elemsize,
        k.as_mut_ptr() as *mut c_void,
        key.len(),
        HM_BINARY,
    ) as *mut u8;
    let idx = temp_of(t, elemsize);
    let e = t.offset(idx * elemsize as isize);
    std::ptr::copy_nonoverlapping(key.as_ptr(), e, key.len());
    std::ptr::copy_nonoverlapping(value.as_ptr(), e.add(key.len()), value.len());
    t
}

/// `hmgeti(t, k)` -> (new t, index)
pub unsafe fn hmgeti_bytes(lib: &Lib, t: *mut u8, elemsize: usize, key: &[u8]) -> (*mut u8, isize) {
    let mut k = key.to_vec();
    let t = (lib.hmget_key)(
        t as *mut c_void,
        elemsize,
        k.as_mut_ptr() as *mut c_void,
        key.len(),
        HM_BINARY,
    ) as *mut u8;
    (t, temp_of(t, elemsize))
}

/// `hmdel(t, k)` -> (new t, result)
pub unsafe fn hmdel_bytes(lib: &Lib, t: *mut u8, elemsize: usize, key: &[u8]) -> (*mut u8, isize) {
    let mut k = key.to_vec();
    let t = (lib.hmdel_key)(
        t as *mut c_void,
        elemsize,
        k.as_mut_ptr() as *mut c_void,
        key.len(),
        0,
        HM_BINARY,
    ) as *mut u8;
    let r = if t.is_null() { 0 } else { temp_of(t, elemsize) };
    (t, r)
}
