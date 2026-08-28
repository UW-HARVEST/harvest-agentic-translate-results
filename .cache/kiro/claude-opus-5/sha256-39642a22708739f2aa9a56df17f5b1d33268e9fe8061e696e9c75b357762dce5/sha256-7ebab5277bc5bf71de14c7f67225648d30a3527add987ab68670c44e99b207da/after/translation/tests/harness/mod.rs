//! Shared test harness: loads the C reference `.so` and the Rust `.so` through
//! `libloading` and exposes both behind an identical `Lib` facade so tests can
//! drive them side by side.
#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::ffi::{c_char, c_int, c_void};
use std::path::PathBuf;

pub mod map;
pub mod snap;

pub const HEADER_SIZE: usize = 32;

pub const STBDS_HM_BINARY: c_int = 0;
pub const STBDS_HM_STRING: c_int = 1;

pub const STBDS_SH_NONE: c_int = 0;
pub const STBDS_SH_DEFAULT: c_int = 1;
pub const STBDS_SH_STRDUP: c_int = 2;
pub const STBDS_SH_ARENA: c_int = 3;

// --- function pointer types -------------------------------------------------

pub type ArrGrowF = unsafe extern "C" fn(*mut c_void, usize, usize, usize) -> *mut c_void;
pub type ArrFreeF = unsafe extern "C" fn(*mut c_void);
pub type RandSeed = unsafe extern "C" fn(usize);
pub type HashString = unsafe extern "C" fn(*mut c_char, usize) -> usize;
pub type HashBytes = unsafe extern "C" fn(*mut c_void, usize, usize) -> usize;
pub type HmFreeFunc = unsafe extern "C" fn(*mut c_void, usize);
pub type HmGetKeyTs =
    unsafe extern "C" fn(*mut c_void, usize, *mut c_void, usize, *mut isize, c_int) -> *mut c_void;
pub type HmGetKey = unsafe extern "C" fn(*mut c_void, usize, *mut c_void, usize, c_int) -> *mut c_void;
pub type HmPutDefault = unsafe extern "C" fn(*mut c_void, usize) -> *mut c_void;
pub type HmPutKey = unsafe extern "C" fn(*mut c_void, usize, *mut c_void, usize, c_int) -> *mut c_void;
pub type ShmodeFunc = unsafe extern "C" fn(usize, c_int) -> *mut c_void;
pub type HmDelKey =
    unsafe extern "C" fn(*mut c_void, usize, *mut c_void, usize, usize, c_int) -> *mut c_void;
pub type StrAlloc = unsafe extern "C" fn(*mut c_void, *mut c_char) -> *mut c_char;
pub type StrReset = unsafe extern "C" fn(*mut c_void);
pub type StrKey = unsafe extern "C" fn(c_int) -> *mut c_char;
pub type Intput = unsafe extern "C" fn(c_int);

/// All exported entry points of one shared object.
pub struct Lib {
    pub name: &'static str,
    _lib: Library,
    pub arrgrowf: ArrGrowF,
    pub arrfreef: ArrFreeF,
    pub rand_seed: RandSeed,
    pub hash_string: HashString,
    pub hash_bytes: HashBytes,
    pub hmfree_func: HmFreeFunc,
    pub hmget_key_ts: HmGetKeyTs,
    pub hmget_key: HmGetKey,
    pub hmput_default: HmPutDefault,
    pub hmput_key: HmPutKey,
    pub shmode_func: ShmodeFunc,
    pub hmdel_key: HmDelKey,
    pub stralloc: StrAlloc,
    pub strreset: StrReset,
    pub strkey: StrKey,
    pub intput: Intput,
}

unsafe fn sym<T: Copy>(lib: &Library, name: &[u8]) -> T {
    unsafe {
        let s: Symbol<T> = lib
            .get(name)
            .unwrap_or_else(|e| panic!("missing symbol {}: {e}", String::from_utf8_lossy(name)));
        *s
    }
}

impl Lib {
    pub fn open(name: &'static str, path: &PathBuf) -> Lib {
        unsafe {
            let lib = Library::new(path).unwrap_or_else(|e| panic!("cannot load {path:?}: {e}"));
            Lib {
                name,
                arrgrowf: sym(&lib, b"stbds_arrgrowf"),
                arrfreef: sym(&lib, b"stbds_arrfreef"),
                rand_seed: sym(&lib, b"stbds_rand_seed"),
                hash_string: sym(&lib, b"stbds_hash_string"),
                hash_bytes: sym(&lib, b"stbds_hash_bytes"),
                hmfree_func: sym(&lib, b"stbds_hmfree_func"),
                hmget_key_ts: sym(&lib, b"stbds_hmget_key_ts"),
                hmget_key: sym(&lib, b"stbds_hmget_key"),
                hmput_default: sym(&lib, b"stbds_hmput_default"),
                hmput_key: sym(&lib, b"stbds_hmput_key"),
                shmode_func: sym(&lib, b"stbds_shmode_func"),
                hmdel_key: sym(&lib, b"stbds_hmdel_key"),
                stralloc: sym(&lib, b"stbds_stralloc"),
                strreset: sym(&lib, b"stbds_strreset"),
                strkey: sym(&lib, b"strkey"),
                intput: sym(&lib, b"intput"),
                _lib: lib,
            }
        }
    }
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

pub fn c_so_path() -> PathBuf {
    let dir = manifest_dir().join("../c_src/build");
    let mut found: Option<PathBuf> = None;
    let entries = std::fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("cannot read {dir:?} (build the C library first): {e}"));
    for e in entries.flatten() {
        let p = e.path();
        let n = p.file_name().unwrap().to_string_lossy().to_string();
        if n.starts_with("lib") && n.ends_with(".so") {
            found = Some(p);
        }
    }
    found.unwrap_or_else(|| panic!("no lib*.so in {dir:?}"))
}

pub fn rust_so_path() -> PathBuf {
    // test executable lives in <profile-dir>/deps/
    let exe = std::env::current_exe().expect("current_exe");
    let profile_dir = exe.parent().unwrap().parent().unwrap();
    let p = profile_dir.join("libintput_lib.so");
    assert!(p.exists(), "rust cdylib not found at {p:?}");
    p
}

/// The pair of libraries under comparison.
///
/// Both `.so`s carry a mutable global (`stbds_hash_seed`) and `dlopen` returns
/// the *same* mapping for every caller in the process, so the two libraries
/// must be driven by one test at a time.  Holding this struct holds that lock.
pub struct Pair {
    pub c: Lib,
    pub r: Lib,
    _guard: std::sync::MutexGuard<'static, ()>,
}

static SERIALIZE: std::sync::Mutex<()> = std::sync::Mutex::new(());

pub fn pair() -> Pair {
    let guard = SERIALIZE.lock().unwrap_or_else(|e| e.into_inner());
    let p = Pair {
        c: Lib::open("C", &c_so_path()),
        r: Lib::open("Rust", &rust_so_path()),
        _guard: guard,
    };
    // Start every test from the same `stbds_hash_seed` so that the seed each
    // hash index picks up is reproducible and identical on both sides.
    unsafe {
        (p.c.rand_seed)(0x31415926);
        (p.r.rand_seed)(0x31415926);
    }
    p
}
