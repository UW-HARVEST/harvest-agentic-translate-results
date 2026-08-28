//! Shared harness: loads both the C reference `.so` and the Rust `.so` through
//! `libloading` and exposes every exported symbol as a raw function pointer, so
//! that *both* implementations are always exercised through the FFI boundary.

#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::ffi::{c_char, c_int, c_void};
use std::path::{Path, PathBuf};

// ---------------------------------------------------------------------------
// Layout-compatible mirrors of the C structures (for introspection only)
// ---------------------------------------------------------------------------

pub const HEADER_SIZE: usize = 32;
pub const BUCKET_LENGTH: usize = 8;
pub const BUCKET_SHIFT: usize = 3;

pub const HM_BINARY: c_int = 0;
pub const HM_STRING: c_int = 1;

pub const SH_NONE: c_int = 0;
pub const SH_DEFAULT: c_int = 1;
pub const SH_STRDUP: c_int = 2;
pub const SH_ARENA: c_int = 3;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct ArrayHeader {
    pub length: usize,
    pub capacity: usize,
    pub hash_table: *mut c_void,
    pub temp: isize,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Default)]
pub struct StringArena {
    pub storage: *mut c_void,
    pub remaining: usize,
    pub block: u8,
    pub mode: u8,
    pub _pad: [u8; 6],
}

impl StringArena {
    pub fn new() -> Self {
        StringArena {
            storage: std::ptr::null_mut(),
            remaining: 0,
            block: 0,
            mode: 0,
            _pad: [0; 6],
        }
    }
}

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

// Compile-time layout guards: these must match the C `sizeof`s.
const _: () = assert!(std::mem::size_of::<ArrayHeader>() == 32);
const _: () = assert!(std::mem::size_of::<StringArena>() == 24);
const _: () = assert!(std::mem::size_of::<HashBucket>() == 128);
const _: () = assert!(std::mem::size_of::<HashIndex>() == 104);

// ---------------------------------------------------------------------------
// Function pointer types
// ---------------------------------------------------------------------------

pub type FnArrgrowf = unsafe extern "C" fn(*mut c_void, usize, usize, usize) -> *mut c_void;
pub type FnArrfreef = unsafe extern "C" fn(*mut c_void);
pub type FnRandSeed = unsafe extern "C" fn(usize);
pub type FnHashBytes = unsafe extern "C" fn(*mut c_void, usize, usize) -> usize;
pub type FnHashString = unsafe extern "C" fn(*mut c_char, usize) -> usize;
pub type FnHmfreeFunc = unsafe extern "C" fn(*mut c_void, usize);
pub type FnHmgetKey = unsafe extern "C" fn(*mut c_void, usize, *mut c_void, usize, c_int) -> *mut c_void;
pub type FnHmgetKeyTs =
    unsafe extern "C" fn(*mut c_void, usize, *mut c_void, usize, *mut isize, c_int) -> *mut c_void;
pub type FnHmputDefault = unsafe extern "C" fn(*mut c_void, usize) -> *mut c_void;
pub type FnHmputKey = unsafe extern "C" fn(*mut c_void, usize, *mut c_void, usize, c_int) -> *mut c_void;
pub type FnHmdelKey =
    unsafe extern "C" fn(*mut c_void, usize, *mut c_void, usize, usize, c_int) -> *mut c_void;
pub type FnShmodeFunc = unsafe extern "C" fn(usize, c_int) -> *mut c_void;
pub type FnStralloc = unsafe extern "C" fn(*mut StringArena, *mut c_char) -> *mut c_char;
pub type FnStrreset = unsafe extern "C" fn(*mut StringArena);
pub type FnStrkey = unsafe extern "C" fn(c_int) -> *mut c_char;
pub type FnHelxo = unsafe extern "C" fn(c_char);

/// One loaded implementation (either the C reference or the Rust translation).
pub struct Impl {
    pub name: &'static str,
    #[allow(unused)]
    lib: Library,
    pub arrgrowf: FnArrgrowf,
    pub arrfreef: FnArrfreef,
    pub rand_seed: FnRandSeed,
    pub hash_bytes: FnHashBytes,
    pub hash_string: FnHashString,
    pub hmfree_func: FnHmfreeFunc,
    pub hmget_key: FnHmgetKey,
    pub hmget_key_ts: FnHmgetKeyTs,
    pub hmput_default: FnHmputDefault,
    pub hmput_key: FnHmputKey,
    pub hmdel_key: FnHmdelKey,
    pub shmode_func: FnShmodeFunc,
    pub stralloc: FnStralloc,
    pub strreset: FnStrreset,
    pub strkey: FnStrkey,
    pub helxo: FnHelxo,
}

unsafe fn sym<T: Copy>(lib: &Library, name: &[u8]) -> T {
    let s: Symbol<T> = unsafe {
        lib.get(name)
            .unwrap_or_else(|e| panic!("missing symbol {}: {e}", String::from_utf8_lossy(name)))
    };
    *s
}

impl Impl {
    pub fn load(name: &'static str, path: &Path) -> Impl {
        let lib = unsafe { Library::new(path) }
            .unwrap_or_else(|e| panic!("failed to dlopen {}: {e}", path.display()));
        unsafe {
            Impl {
                name,
                arrgrowf: sym(&lib, b"stbds_arrgrowf\0"),
                arrfreef: sym(&lib, b"stbds_arrfreef\0"),
                rand_seed: sym(&lib, b"stbds_rand_seed\0"),
                hash_bytes: sym(&lib, b"stbds_hash_bytes\0"),
                hash_string: sym(&lib, b"stbds_hash_string\0"),
                hmfree_func: sym(&lib, b"stbds_hmfree_func\0"),
                hmget_key: sym(&lib, b"stbds_hmget_key\0"),
                hmget_key_ts: sym(&lib, b"stbds_hmget_key_ts\0"),
                hmput_default: sym(&lib, b"stbds_hmput_default\0"),
                hmput_key: sym(&lib, b"stbds_hmput_key\0"),
                hmdel_key: sym(&lib, b"stbds_hmdel_key\0"),
                shmode_func: sym(&lib, b"stbds_shmode_func\0"),
                stralloc: sym(&lib, b"stbds_stralloc\0"),
                strreset: sym(&lib, b"stbds_strreset\0"),
                strkey: sym(&lib, b"strkey\0"),
                helxo: sym(&lib, b"helxo\0"),
                lib,
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Locating the two shared objects
// ---------------------------------------------------------------------------

fn workspace_root() -> PathBuf {
    // translation/ -> parent
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("manifest dir has a parent")
        .to_path_buf()
}

fn find_c_so() -> PathBuf {
    // Allow pointing the suite at an alternative C build (e.g. an optimised
    // one) without touching c_src/.
    if let Ok(p) = std::env::var("C_SO_PATH") {
        let p = PathBuf::from(p);
        assert!(p.exists(), "C_SO_PATH does not exist: {}", p.display());
        return p;
    }
    let dir = workspace_root().join("c_src").join("build");
    let mut found: Vec<PathBuf> = std::fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("cannot read {}: {e} (did you build the C library?)", dir.display()))
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().map(|x| x == "so").unwrap_or(false))
        .collect();
    found.sort();
    found
        .pop()
        .unwrap_or_else(|| panic!("no .so found in {}", dir.display()))
}

fn find_rust_so() -> PathBuf {
    // current_exe: target/<profile>/deps/<test>-<hash>
    let exe = std::env::current_exe().expect("current_exe");
    let profile_dir = exe
        .parent()
        .and_then(|p| p.parent())
        .expect("target/<profile>")
        .to_path_buf();
    let mut candidates = vec![profile_dir.join("libhelxo_lib.so")];
    // `cargo test` does not build the cdylib target, so fall back to whichever
    // profile directory has it (see run_all.sh, which builds it first).
    if let Some(target_dir) = profile_dir.parent() {
        for p in ["debug", "release"] {
            candidates.push(target_dir.join(p).join("libhelxo_lib.so"));
        }
    }
    for c in &candidates {
        if c.exists() {
            return c.clone();
        }
    }
    panic!(
        "Rust cdylib not found (looked in {:?}) - run `cargo build` first",
        candidates
    );
}

/// Loads the C reference implementation and the Rust translation.
pub fn load_pair() -> (Impl, Impl) {
    let c = Impl::load("C", &find_c_so());
    let r = Impl::load("Rust", &find_rust_so());
    (c, r)
}

/// `dlopen` of the same path returns the same mapping, so the `stbds_hash_seed`
/// global of each library is shared by every test in this binary. Tests that
/// create hash indices must therefore serialise and re-seed.
pub static SEED_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

pub struct Guard(#[allow(unused)] std::sync::MutexGuard<'static, ()>);

/// Serialises access to the libraries' global hash seed and forces both
/// implementations to the same starting seed.
pub fn seeded(c: &Impl, r: &Impl, seed: usize) -> Guard {
    let g = SEED_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    unsafe {
        (c.rand_seed)(seed);
        (r.rand_seed)(seed);
    }
    Guard(g)
}

// ---------------------------------------------------------------------------
// Introspection helpers
// ---------------------------------------------------------------------------

/// Header of a raw stb_ds array (`a` is the element-0 pointer).
pub unsafe fn header(a: *mut c_void) -> ArrayHeader {
    unsafe { *((a as *mut u8).sub(HEADER_SIZE) as *const ArrayHeader) }
}

/// Header of a hash map (`t` is the "hash side" pointer, i.e. `raw + elemsize`).
pub unsafe fn map_header(t: *mut c_void, elemsize: usize) -> ArrayHeader {
    unsafe { header((t as *mut u8).sub(elemsize) as *mut c_void) }
}

pub unsafe fn map_temp(t: *mut c_void, elemsize: usize) -> isize {
    unsafe { map_header(t, elemsize).temp }
}

pub unsafe fn cstr_bytes(p: *const c_char) -> Vec<u8> {
    if p.is_null() {
        return b"<null>".to_vec();
    }
    let mut out = Vec::new();
    let mut i = 0isize;
    unsafe {
        while *p.offset(i) != 0 {
            out.push(*p.offset(i) as u8);
            i += 1;
        }
    }
    out
}

/// How the key of a map element should be interpreted when snapshotting.
#[derive(Copy, Clone, PartialEq, Eq)]
pub enum KeyKind {
    /// Key bytes live inline in the element (binary hash map).
    Inline,
    /// Key is a `char *`; compare the pointed-to string instead of the pointer.
    StringPtr,
}

/// Serialises everything about a map that must be identical between the two
/// implementations: array header, hash-index bookkeeping, every bucket's
/// hash/index arrays, and the live element payloads.
///
/// Raw pointer values (which necessarily differ) are reduced to
/// null / non-null, and `char *` keys are replaced by their contents.
pub unsafe fn snapshot_map(
    t: *mut c_void,
    elemsize: usize,
    keysize: usize,
    kind: KeyKind,
) -> Vec<u8> {
    let mut out = Vec::new();
    if t.is_null() {
        out.extend_from_slice(b"map=NULL;");
        return out;
    }
    let raw = unsafe { (t as *mut u8).sub(elemsize) };
    let h = unsafe { header(raw as *mut c_void) };
    out.extend_from_slice(format!("len={} cap={} temp={};", h.length, h.capacity, h.temp).as_bytes());

    if h.hash_table.is_null() {
        out.extend_from_slice(b"table=NULL;");
    } else {
        let ti = unsafe { *(h.hash_table as *const HashIndex) };
        out.extend_from_slice(
            format!(
                "slots={} used={} uct={} ucst={} tomb={} tct={} seed={:#x} log2={};",
                ti.slot_count,
                ti.used_count,
                ti.used_count_threshold,
                ti.used_count_shrink_threshold,
                ti.tombstone_count,
                ti.tombstone_count_threshold,
                ti.seed,
                ti.slot_count_log2
            )
            .as_bytes(),
        );
        out.extend_from_slice(
            format!(
                "arena(remaining={} block={} mode={} storage_null={});",
                ti.string.remaining,
                ti.string.block,
                ti.string.mode,
                ti.string.storage.is_null()
            )
            .as_bytes(),
        );
        // NOTE: `temp_key` is deliberately *not* compared here. C's
        // `stbds_make_hash_index` never initialises it, and only the string
        // modes ever assign it, so in general it holds indeterminate memory.
        // It is checked separately right after insertions that define it.
        // Bucket contents are pure values -> compare exactly.
        let nbuckets = ti.slot_count >> BUCKET_SHIFT;
        for b in 0..nbuckets {
            let bucket = unsafe { *ti.storage.add(b) };
            out.extend_from_slice(format!("b{b}:").as_bytes());
            for i in 0..BUCKET_LENGTH {
                out.extend_from_slice(
                    format!("({:#x},{})", bucket.hash[i], bucket.index[i]).as_bytes(),
                );
            }
            out.push(b';');
        }
    }

    // Element payloads. Element 0 of the raw array is the "default" slot.
    for i in 0..h.length {
        let elem = unsafe { raw.add(elemsize * i) };
        out.extend_from_slice(format!("e{i}:").as_bytes());
        match kind {
            KeyKind::Inline => {
                let bytes = unsafe { std::slice::from_raw_parts(elem, elemsize) };
                out.extend_from_slice(format!("{bytes:?}").as_bytes());
            }
            KeyKind::StringPtr => {
                let kp = unsafe { *(elem as *const *const c_char) };
                out.extend_from_slice(format!("key={:?}", unsafe { cstr_bytes(kp) }).as_bytes());
                // Remaining payload after the key pointer.
                if elemsize > keysize {
                    let tail =
                        unsafe { std::slice::from_raw_parts(elem.add(keysize), elemsize - keysize) };
                    out.extend_from_slice(format!(" tail={tail:?}").as_bytes());
                }
            }
        }
        out.push(b';');
    }
    out
}

pub fn assert_same(what: &str, c: &[u8], r: &[u8]) {
    if c != r {
        panic!(
            "MISMATCH in {what}\n  C   : {}\n  Rust: {}",
            String::from_utf8_lossy(c),
            String::from_utf8_lossy(r)
        );
    }
}

// ---------------------------------------------------------------------------
// stdout capture (for comparing `printf` output byte-for-byte)
// ---------------------------------------------------------------------------

unsafe extern "C" {
    fn dup(fd: c_int) -> c_int;
    fn dup2(old: c_int, new: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn open(path: *const c_char, flags: c_int, ...) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
}

/// Redirects fd 1 to a temporary file, runs `f`, and returns everything that
/// was written. Both libraries print through the same libc `stdout`, so the
/// stream is flushed before and after the redirection.
pub fn capture_stdout<F: FnOnce()>(tag: &str, f: F) -> Vec<u8> {
    let path = std::env::temp_dir().join(format!(
        "helxo_cmp_{}_{}_{tag}.out",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let cpath = std::ffi::CString::new(path.to_str().unwrap()).unwrap();
    // O_WRONLY | O_CREAT | O_TRUNC on Linux
    const FLAGS: c_int = 1 | 64 | 512;
    unsafe {
        fflush(std::ptr::null_mut());
        let saved = dup(1);
        assert!(saved >= 0, "dup(1) failed");
        let fd = open(cpath.as_ptr(), FLAGS, 0o644 as c_int);
        assert!(fd >= 0, "open({}) failed", path.display());
        assert!(dup2(fd, 1) >= 0, "dup2 failed");
        close(fd);

        f();

        fflush(std::ptr::null_mut());
        assert!(dup2(saved, 1) >= 0, "dup2 restore failed");
        close(saved);
    }
    let data = std::fs::read(&path).unwrap_or_default();
    let _ = std::fs::remove_file(&path);
    data
}
