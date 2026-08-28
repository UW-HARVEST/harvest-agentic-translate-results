//! Shared harness: locates/builds both shared objects and exposes a typed
//! view over every symbol the C `.so` exports.
//!
//! Both libraries are loaded with `libloading` (i.e. `dlopen` with
//! `RTLD_LOCAL`), so each handle resolves the symbols of its own object and
//! internal calls stay inside the object they came from. The Rust side is
//! never called directly - only through the `#[no_mangle]` exports.

#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::ffi::{c_char, c_int, c_void};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

// ---------------------------------------------------------------------------
// Layout mirrors of the C types (used to inspect state produced by *both*
// libraries; the layout is part of the ABI contract, not of the translation).
// ---------------------------------------------------------------------------

pub const BUCKET_LENGTH: usize = 8;
pub const BUCKET_SHIFT: usize = 3;
pub const HM_BINARY: c_int = 0;
pub const HM_STRING: c_int = 1;
pub const SH_NONE: c_int = 0;
pub const SH_DEFAULT: c_int = 1;
pub const SH_STRDUP: c_int = 2;
pub const SH_ARENA: c_int = 3;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct ArrayHeader {
    pub length: usize,
    pub capacity: usize,
    pub hash_table: *mut c_void,
    pub temp: isize,
}

pub const HEADER_SIZE: usize = std::mem::size_of::<ArrayHeader>();

#[repr(C)]
#[derive(Clone, Copy)]
pub struct StringBlock {
    pub next: *mut StringBlock,
    pub storage: [c_char; 8],
}

#[repr(C)]
#[derive(Clone, Copy)]
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

// ---------------------------------------------------------------------------
// Locating / building the two shared objects
// ---------------------------------------------------------------------------

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation/ has a parent")
        .to_path_buf()
}

fn newer_than(a: &Path, b: &Path) -> bool {
    let ta = std::fs::metadata(a).and_then(|m| m.modified()).ok();
    let tb = std::fs::metadata(b).and_then(|m| m.modified()).ok();
    match (ta, tb) {
        (Some(ta), Some(tb)) => ta >= tb,
        _ => false,
    }
}

/// Path to the C shared library, building it with cmake when absent.
pub fn c_lib_path() -> PathBuf {
    let root = workspace_root();
    let c_src = root.join("c_src");
    let build = c_src.join("build");

    let find = || -> Option<PathBuf> {
        let entries = std::fs::read_dir(&build).ok()?;
        for e in entries.flatten() {
            let p = e.path();
            if p.extension().map(|x| x == "so").unwrap_or(false) {
                return Some(p);
            }
        }
        None
    };

    if let Some(p) = find() {
        return p;
    }

    std::fs::create_dir_all(&build).expect("create c_src/build");
    let ok = Command::new("cmake")
        .current_dir(&build)
        .args(["..", "-DCMAKE_POSITION_INDEPENDENT_CODE=ON"])
        .status()
        .expect("run cmake")
        .success();
    assert!(ok, "cmake configure failed");
    let ok = Command::new("cmake")
        .current_dir(&build)
        .args(["--build", "."])
        .status()
        .expect("run cmake --build")
        .success();
    assert!(ok, "cmake build failed");

    find().expect("C .so produced by cmake")
}

/// Path to the Rust `cdylib`. Prefers a cargo-produced artifact that is at
/// least as new as `src/lib.rs`; otherwise compiles one with `rustc` using the
/// same crate type and name as `[lib]` in Cargo.toml.
pub fn rust_lib_path() -> PathBuf {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let src = manifest.join("src/lib.rs");

    for profile in ["release", "debug"] {
        let p = manifest
            .join("target")
            .join(profile)
            .join("libsh_puts_lib.so");
        if p.exists() && newer_than(&p, &src) {
            return p;
        }
    }

    let out_dir = manifest.join("target/difftest");
    std::fs::create_dir_all(&out_dir).expect("create target/difftest");
    let out = out_dir.join("libsh_puts_lib.so");
    let status = Command::new("rustc")
        .args([
            "--edition",
            "2021",
            "--crate-type",
            "cdylib",
            "--crate-name",
            "sh_puts_lib",
            "-C",
            "opt-level=2",
        ])
        .arg(&src)
        .arg("-o")
        .arg(&out)
        .status()
        .expect("run rustc");
    assert!(status.success(), "rustc failed to build the cdylib");
    out
}

// ---------------------------------------------------------------------------
// Typed façade over the exported API
// ---------------------------------------------------------------------------

pub struct Api {
    pub name: &'static str,
    _lib: Library,
    // ordered lowest-level first
    f_rand_seed: unsafe extern "C" fn(usize),
    f_hash_string: unsafe extern "C" fn(*mut c_char, usize) -> usize,
    f_hash_bytes: unsafe extern "C" fn(*mut c_void, usize, usize) -> usize,
    f_arrgrowf: unsafe extern "C" fn(*mut c_void, usize, usize, usize) -> *mut c_void,
    f_arrfreef: unsafe extern "C" fn(*mut c_void),
    f_stralloc: unsafe extern "C" fn(*mut StringArena, *mut c_char) -> *mut c_char,
    f_strreset: unsafe extern "C" fn(*mut StringArena),
    f_shmode_func: unsafe extern "C" fn(usize, c_int) -> *mut c_void,
    f_hmput_default: unsafe extern "C" fn(*mut c_void, usize) -> *mut c_void,
    f_hmput_key: unsafe extern "C" fn(*mut c_void, usize, *mut c_void, usize, c_int) -> *mut c_void,
    f_hmget_key: unsafe extern "C" fn(*mut c_void, usize, *mut c_void, usize, c_int) -> *mut c_void,
    f_hmget_key_ts: unsafe extern "C" fn(
        *mut c_void,
        usize,
        *mut c_void,
        usize,
        *mut isize,
        c_int,
    ) -> *mut c_void,
    f_hmdel_key:
        unsafe extern "C" fn(*mut c_void, usize, *mut c_void, usize, usize, c_int) -> *mut c_void,
    f_hmfree_func: unsafe extern "C" fn(*mut c_void, usize),
    f_strkey: unsafe extern "C" fn(c_int) -> *mut c_char,
    f_sh_puts: unsafe extern "C" fn(c_int),
}

macro_rules! sym {
    ($lib:expr, $name:literal, $t:ty) => {{
        let s: Symbol<$t> = $lib
            .get(concat!($name, "\0").as_bytes())
            .unwrap_or_else(|e| panic!("missing symbol {}: {}", $name, e));
        *s
    }};
}

impl Api {
    unsafe fn open(name: &'static str, path: &Path) -> Api {
        let lib = Library::new(path).unwrap_or_else(|e| panic!("dlopen {:?}: {}", path, e));
        let api = Api {
            name,
            f_rand_seed: sym!(lib, "stbds_rand_seed", unsafe extern "C" fn(usize)),
            f_hash_string: sym!(
                lib,
                "stbds_hash_string",
                unsafe extern "C" fn(*mut c_char, usize) -> usize
            ),
            f_hash_bytes: sym!(
                lib,
                "stbds_hash_bytes",
                unsafe extern "C" fn(*mut c_void, usize, usize) -> usize
            ),
            f_arrgrowf: sym!(
                lib,
                "stbds_arrgrowf",
                unsafe extern "C" fn(*mut c_void, usize, usize, usize) -> *mut c_void
            ),
            f_arrfreef: sym!(lib, "stbds_arrfreef", unsafe extern "C" fn(*mut c_void)),
            f_stralloc: sym!(
                lib,
                "stbds_stralloc",
                unsafe extern "C" fn(*mut StringArena, *mut c_char) -> *mut c_char
            ),
            f_strreset: sym!(lib, "stbds_strreset", unsafe extern "C" fn(*mut StringArena)),
            f_shmode_func: sym!(
                lib,
                "stbds_shmode_func",
                unsafe extern "C" fn(usize, c_int) -> *mut c_void
            ),
            f_hmput_default: sym!(
                lib,
                "stbds_hmput_default",
                unsafe extern "C" fn(*mut c_void, usize) -> *mut c_void
            ),
            f_hmput_key: sym!(
                lib,
                "stbds_hmput_key",
                unsafe extern "C" fn(*mut c_void, usize, *mut c_void, usize, c_int) -> *mut c_void
            ),
            f_hmget_key: sym!(
                lib,
                "stbds_hmget_key",
                unsafe extern "C" fn(*mut c_void, usize, *mut c_void, usize, c_int) -> *mut c_void
            ),
            f_hmget_key_ts: sym!(
                lib,
                "stbds_hmget_key_ts",
                unsafe extern "C" fn(
                    *mut c_void,
                    usize,
                    *mut c_void,
                    usize,
                    *mut isize,
                    c_int,
                ) -> *mut c_void
            ),
            f_hmdel_key: sym!(
                lib,
                "stbds_hmdel_key",
                unsafe extern "C" fn(
                    *mut c_void,
                    usize,
                    *mut c_void,
                    usize,
                    usize,
                    c_int,
                ) -> *mut c_void
            ),
            f_hmfree_func: sym!(
                lib,
                "stbds_hmfree_func",
                unsafe extern "C" fn(*mut c_void, usize)
            ),
            f_strkey: sym!(lib, "strkey", unsafe extern "C" fn(c_int) -> *mut c_char),
            f_sh_puts: sym!(lib, "sh_puts", unsafe extern "C" fn(c_int)),
            _lib: lib,
        };
        api
    }

    // -- thin wrappers ----------------------------------------------------
    pub unsafe fn rand_seed(&self, s: usize) {
        (self.f_rand_seed)(s)
    }
    pub unsafe fn hash_string(&self, s: *mut c_char, seed: usize) -> usize {
        (self.f_hash_string)(s, seed)
    }
    pub unsafe fn hash_bytes(&self, p: *mut c_void, len: usize, seed: usize) -> usize {
        (self.f_hash_bytes)(p, len, seed)
    }
    pub unsafe fn arrgrowf(
        &self,
        a: *mut c_void,
        elemsize: usize,
        addlen: usize,
        min_cap: usize,
    ) -> *mut c_void {
        (self.f_arrgrowf)(a, elemsize, addlen, min_cap)
    }
    pub unsafe fn arrfreef(&self, a: *mut c_void) {
        (self.f_arrfreef)(a)
    }
    pub unsafe fn stralloc(&self, a: *mut StringArena, s: *mut c_char) -> *mut c_char {
        (self.f_stralloc)(a, s)
    }
    pub unsafe fn strreset(&self, a: *mut StringArena) {
        (self.f_strreset)(a)
    }
    pub unsafe fn shmode_func(&self, elemsize: usize, mode: c_int) -> *mut c_void {
        (self.f_shmode_func)(elemsize, mode)
    }
    pub unsafe fn hmput_default(&self, a: *mut c_void, elemsize: usize) -> *mut c_void {
        (self.f_hmput_default)(a, elemsize)
    }
    pub unsafe fn hmput_key(
        &self,
        a: *mut c_void,
        elemsize: usize,
        key: *mut c_void,
        keysize: usize,
        mode: c_int,
    ) -> *mut c_void {
        (self.f_hmput_key)(a, elemsize, key, keysize, mode)
    }
    pub unsafe fn hmget_key(
        &self,
        a: *mut c_void,
        elemsize: usize,
        key: *mut c_void,
        keysize: usize,
        mode: c_int,
    ) -> *mut c_void {
        (self.f_hmget_key)(a, elemsize, key, keysize, mode)
    }
    pub unsafe fn hmget_key_ts(
        &self,
        a: *mut c_void,
        elemsize: usize,
        key: *mut c_void,
        keysize: usize,
        temp: *mut isize,
        mode: c_int,
    ) -> *mut c_void {
        (self.f_hmget_key_ts)(a, elemsize, key, keysize, temp, mode)
    }
    pub unsafe fn hmdel_key(
        &self,
        a: *mut c_void,
        elemsize: usize,
        key: *mut c_void,
        keysize: usize,
        keyoffset: usize,
        mode: c_int,
    ) -> *mut c_void {
        (self.f_hmdel_key)(a, elemsize, key, keysize, keyoffset, mode)
    }
    pub unsafe fn hmfree_func(&self, a: *mut c_void, elemsize: usize) {
        (self.f_hmfree_func)(a, elemsize)
    }
    pub unsafe fn strkey(&self, n: c_int) -> *mut c_char {
        (self.f_strkey)(n)
    }
    pub unsafe fn sh_puts(&self, n: c_int) {
        (self.f_sh_puts)(n)
    }
}

pub struct Pair {
    pub c: Api,
    pub rs: Api,
}

static PAIR: OnceLock<Pair> = OnceLock::new();

/// The two loaded libraries. `dlopen`ed once per test process.
pub fn pair() -> &'static Pair {
    PAIR.get_or_init(|| {
        let cp = c_lib_path();
        let rp = rust_lib_path();
        unsafe {
            Pair {
                c: Api::open("C", &cp),
                rs: Api::open("Rust", &rp),
            }
        }
    })
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

pub fn cstring(s: &str) -> Vec<u8> {
    let mut v = s.as_bytes().to_vec();
    v.push(0);
    v
}

pub unsafe fn read_cstr(p: *const c_char) -> Vec<u8> {
    assert!(!p.is_null(), "null C string");
    let mut out = Vec::new();
    let mut q = p as *const u8;
    while *q != 0 {
        out.push(*q);
        q = q.add(1);
    }
    out
}

pub unsafe fn header(arr: *mut c_void) -> ArrayHeader {
    *(arr as *mut ArrayHeader).sub(1)
}

pub unsafe fn set_length(arr: *mut c_void, len: usize) {
    (*(arr as *mut ArrayHeader).sub(1)).length = len;
}

/// Fully comparable snapshot of an stb_ds hash-map (`t` is the *user* pointer,
/// i.e. one element past the raw array base).
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct MapSnapshot {
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
    pub string_mode: u8,
    pub string_block: u8,
    pub string_remaining: usize,
    pub string_has_storage: bool,
    pub storage_aligned: bool,
    pub buckets: Vec<(Vec<usize>, Vec<isize>)>,
    /// Raw element bytes for binary maps, or `None` for string maps.
    pub elem_bytes: Option<Vec<u8>>,
    /// Key strings (string maps only), element order.
    pub keys: Option<Vec<Vec<u8>>>,
    /// Values read as `int` at `value_offset` (string maps only).
    pub values: Option<Vec<i32>>,
    pub temp_key: Option<Vec<u8>>,
}

/// `defined` lists the `(offset, len)` byte ranges inside an element that the
/// caller actually initialises. Everything else in an element is `realloc`
/// garbage in *both* libraries (the C code only ever `memcpy`s `keysize` bytes
/// and the caller-side macro writes the key and value fields), so comparing it
/// would compare uninitialised memory.
pub unsafe fn snapshot_binary(
    t: *mut c_void,
    elemsize: usize,
    defined: &[(usize, usize)],
) -> MapSnapshot {
    let mut s = snapshot_common(t, elemsize);
    let raw = (t as *mut u8).sub(elemsize);
    let h = header(raw as *mut c_void);
    let mut bytes = Vec::new();
    for i in 0..h.length {
        if i == 0 {
            // the default slot is memset to zero by the library, so all of it
            // is meaningful
            for k in 0..elemsize {
                bytes.push(*raw.add(k));
            }
            continue;
        }
        for &(off, len) in defined {
            for k in 0..len {
                bytes.push(*raw.add(i * elemsize + off + k));
            }
        }
    }
    s.elem_bytes = Some(bytes);
    s
}

/// `read_temp_key` must only be set when the caller knows `table->temp_key` has
/// been written since the table was last (re)allocated: `stbds_make_hash_index`
/// leaves that field uninitialised, and `stbds_hmput_key` is the only writer.
pub unsafe fn snapshot_string(
    t: *mut c_void,
    elemsize: usize,
    value_offset: usize,
    read_temp_key: bool,
) -> MapSnapshot {
    let mut s = snapshot_common(t, elemsize);
    let raw = (t as *mut u8).sub(elemsize);
    let h = header(raw as *mut c_void);
    let mut keys = Vec::new();
    let mut values = Vec::new();
    // element 0 is the all-zero "default" slot; its key pointer is NULL.
    for i in 0..h.length {
        let e = raw.add(i * elemsize);
        let kp = *(e as *const *const c_char);
        keys.push(if kp.is_null() {
            Vec::new()
        } else {
            read_cstr(kp)
        });
        values.push(*(e.add(value_offset) as *const i32));
    }
    s.keys = Some(keys);
    s.values = Some(values);
    if s.has_table && read_temp_key {
        let table = h.hash_table as *const HashIndex;
        let tk = (*table).temp_key;
        s.temp_key = Some(if tk.is_null() {
            Vec::new()
        } else {
            read_cstr(tk)
        });
    }
    s
}

unsafe fn snapshot_common(t: *mut c_void, elemsize: usize) -> MapSnapshot {
    let raw = (t as *mut u8).sub(elemsize) as *mut c_void;
    let h = header(raw);
    let mut s = MapSnapshot {
        length: h.length,
        capacity: h.capacity,
        temp: h.temp,
        has_table: !h.hash_table.is_null(),
        slot_count: 0,
        used_count: 0,
        used_count_threshold: 0,
        used_count_shrink_threshold: 0,
        tombstone_count: 0,
        tombstone_count_threshold: 0,
        seed: 0,
        slot_count_log2: 0,
        string_mode: 0,
        string_block: 0,
        string_remaining: 0,
        string_has_storage: false,
        storage_aligned: true,
        buckets: Vec::new(),
        elem_bytes: None,
        keys: None,
        values: None,
        temp_key: None,
    };
    if s.has_table {
        let table = h.hash_table as *const HashIndex;
        s.slot_count = (*table).slot_count;
        s.used_count = (*table).used_count;
        s.used_count_threshold = (*table).used_count_threshold;
        s.used_count_shrink_threshold = (*table).used_count_shrink_threshold;
        s.tombstone_count = (*table).tombstone_count;
        s.tombstone_count_threshold = (*table).tombstone_count_threshold;
        s.seed = (*table).seed;
        s.slot_count_log2 = (*table).slot_count_log2;
        s.string_mode = (*table).string.mode;
        s.string_block = (*table).string.block;
        s.string_remaining = (*table).string.remaining;
        s.string_has_storage = !(*table).string.storage.is_null();
        s.storage_aligned = ((*table).storage as usize) % 64 == 0;
        let nbuckets = (*table).slot_count >> BUCKET_SHIFT;
        for i in 0..nbuckets {
            let b = (*table).storage.add(i);
            s.buckets
                .push(((*b).hash.to_vec(), (*b).index.to_vec()));
        }
    }
    let _ = elemsize;
    s
}

/// Snapshot of a `stbds_string_arena` that is comparable across libraries:
/// pointers are replaced by "is null" plus the offset of the last allocation
/// inside its block.
#[derive(Debug, PartialEq, Eq)]
pub struct ArenaSnapshot {
    pub has_storage: bool,
    pub remaining: usize,
    pub block: u8,
    pub mode: u8,
    pub block_chain_len: usize,
}

pub unsafe fn snapshot_arena(a: *const StringArena) -> ArenaSnapshot {
    let mut n = 0usize;
    let mut x = (*a).storage;
    while !x.is_null() {
        n += 1;
        x = (*x).next;
        assert!(n < 100000, "arena block chain looks cyclic");
    }
    ArenaSnapshot {
        has_storage: !(*a).storage.is_null(),
        remaining: (*a).remaining,
        block: (*a).block,
        mode: (*a).mode,
        block_chain_len: n,
    }
}

// ---------------------------------------------------------------------------
// stdout capture (both libraries print through the process-wide libc stdout)
// ---------------------------------------------------------------------------

extern "C" {
    fn dup(fd: c_int) -> c_int;
    fn dup2(old: c_int, new: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
}

/// Runs `f` with fd 1 redirected into a temporary file and returns the bytes
/// written. `fflush(NULL)` flushes every open stream, which covers the libc
/// `stdout` buffer shared by the test binary and both `.so`s.
/// Guards state that is *global to a loaded library* and therefore shared by
/// all tests in a process: the `static char buffer[256]` behind `strkey`, the
/// `stbds_hash_seed` global, and process fd 1 during stdout capture.
/// Tests that touch any of those must hold this lock.
pub fn shared_lock() -> std::sync::MutexGuard<'static, ()> {
    static LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
    LOCK.lock().unwrap_or_else(|e| e.into_inner())
}

/// Runs `f` with fd 1 redirected into a temporary file and returns the bytes
/// written. `fflush(NULL)` flushes every open stream, which covers the libc
/// `stdout` buffer shared by the test binary and both `.so`s.
///
/// The caller must hold [`shared_lock`].
pub unsafe fn capture_stdout<F: FnOnce()>(tag: &str, f: F) -> Vec<u8> {
    let path = std::env::temp_dir().join(format!(
        "shputs-capture-{}-{}-{:?}.txt",
        std::process::id(),
        tag,
        std::thread::current().id()
    ));
    let _ = std::fs::remove_file(&path);

    fflush(std::ptr::null_mut());
    let file = std::fs::File::create(&path).expect("create capture file");
    let fd = {
        use std::os::unix::io::AsRawFd;
        file.as_raw_fd()
    };
    let saved = dup(1);
    assert!(saved >= 0, "dup(1) failed");
    assert!(dup2(fd, 1) >= 0, "dup2 failed");

    f();

    fflush(std::ptr::null_mut());
    assert!(dup2(saved, 1) >= 0, "dup2 restore failed");
    close(saved);
    drop(file);

    let out = std::fs::read(&path).expect("read capture file");
    let _ = std::fs::remove_file(&path);
    out
}

// ---------------------------------------------------------------------------
// Faithful re-implementations of the stb_ds *macros*, so that the exported
// functions are exercised exactly the way real callers reach them.
// ---------------------------------------------------------------------------

/// `stbds_hmput(t, k, v)` for a binary-keyed map.
///
/// ```c
/// (t) = stbds_hmput_key((t), sizeof *(t), &(k), sizeof (t)->key, 0),
/// (t)[stbds_temp((t)-1)].key = (k),
/// (t)[stbds_temp((t)-1)].value = (v)
/// ```
pub unsafe fn hmput(
    api: &Api,
    t: *mut c_void,
    elemsize: usize,
    key: &[u8],
    keysize: usize,
    value: &[u8],
    value_offset: usize,
) -> *mut c_void {
    assert!(key.len() >= keysize);
    let t = api.hmput_key(
        t,
        elemsize,
        key.as_ptr() as *mut c_void,
        keysize,
        HM_BINARY,
    );
    let raw = (t as *mut u8).sub(elemsize);
    let i = header(raw as *mut c_void).temp;
    let e = (t as *mut u8).add(elemsize * i as usize);
    std::ptr::copy_nonoverlapping(key.as_ptr(), e, keysize);
    std::ptr::copy_nonoverlapping(value.as_ptr(), e.add(value_offset), value.len());
    t
}

/// `stbds_hmgeti(t, k)` -> `(t, index)`
pub unsafe fn hmgeti(
    api: &Api,
    t: *mut c_void,
    elemsize: usize,
    key: &[u8],
    keysize: usize,
) -> (*mut c_void, isize) {
    let t = api.hmget_key(
        t,
        elemsize,
        key.as_ptr() as *mut c_void,
        keysize,
        HM_BINARY,
    );
    let raw = (t as *mut u8).sub(elemsize);
    (t, header(raw as *mut c_void).temp)
}

/// `stbds_hmgeti_ts(t, k, temp)` -> `(t, temp)`
pub unsafe fn hmgeti_ts(
    api: &Api,
    t: *mut c_void,
    elemsize: usize,
    key: &[u8],
    keysize: usize,
) -> (*mut c_void, isize) {
    let mut temp: isize = 0x5555_5555;
    let t = api.hmget_key_ts(
        t,
        elemsize,
        key.as_ptr() as *mut c_void,
        keysize,
        &mut temp,
        HM_BINARY,
    );
    (t, temp)
}

/// `stbds_hmdel(t, k)` -> `(t, result)`
pub unsafe fn hmdel(
    api: &Api,
    t: *mut c_void,
    elemsize: usize,
    key: &[u8],
    keysize: usize,
    keyoffset: usize,
) -> (*mut c_void, isize) {
    let t = api.hmdel_key(
        t,
        elemsize,
        key.as_ptr() as *mut c_void,
        keysize,
        keyoffset,
        HM_BINARY,
    );
    if t.is_null() {
        (t, 0)
    } else {
        let raw = (t as *mut u8).sub(elemsize);
        (t, header(raw as *mut c_void).temp)
    }
}

/// `stbds_shput(t, k, v)`: the key pointer itself is handed to the library.
pub unsafe fn shput(
    api: &Api,
    t: *mut c_void,
    elemsize: usize,
    key: *mut c_char,
    value: i32,
    value_offset: usize,
) -> *mut c_void {
    let t = api.hmput_key(
        t,
        elemsize,
        key as *mut c_void,
        std::mem::size_of::<*mut c_char>(),
        HM_STRING,
    );
    let raw = (t as *mut u8).sub(elemsize);
    let i = header(raw as *mut c_void).temp;
    let e = (t as *mut u8).add(elemsize * i as usize);
    *(e.add(value_offset) as *mut i32) = value;
    t
}

/// `stbds_shputs(t, s)`: writes the whole struct, then restores the key from
/// `stbds_temp_key`.
pub unsafe fn shputs(
    api: &Api,
    t: *mut c_void,
    elemsize: usize,
    key: *mut c_char,
    value: i32,
    value_offset: usize,
) -> *mut c_void {
    let t = api.hmput_key(
        t,
        elemsize,
        key as *mut c_void,
        std::mem::size_of::<*mut c_char>(),
        HM_STRING,
    );
    let raw = (t as *mut u8).sub(elemsize) as *mut c_void;
    let i = header(raw).temp;
    let e = (t as *mut u8).add(elemsize * i as usize);
    // (t)[i] = s
    *(e as *mut *mut c_char) = key;
    *(e.add(value_offset) as *mut i32) = value;
    // (t)[i].key = stbds_temp_key((t)-1)
    let table = header(raw).hash_table as *mut *mut c_char;
    *(e as *mut *mut c_char) = *table;
    t
}

/// `stbds_shgeti(t, k)` -> `(t, index)`
pub unsafe fn shgeti(
    api: &Api,
    t: *mut c_void,
    elemsize: usize,
    key: *mut c_char,
) -> (*mut c_void, isize) {
    let t = api.hmget_key(
        t,
        elemsize,
        key as *mut c_void,
        std::mem::size_of::<*mut c_char>(),
        HM_STRING,
    );
    let raw = (t as *mut u8).sub(elemsize);
    (t, header(raw as *mut c_void).temp)
}

/// `stbds_shdel(t, k)` -> `(t, result)`
pub unsafe fn shdel(
    api: &Api,
    t: *mut c_void,
    elemsize: usize,
    key: *mut c_char,
) -> (*mut c_void, isize) {
    let t = api.hmdel_key(
        t,
        elemsize,
        key as *mut c_void,
        std::mem::size_of::<*mut c_char>(),
        0,
        HM_STRING,
    );
    if t.is_null() {
        (t, 0)
    } else {
        let raw = (t as *mut u8).sub(elemsize);
        (t, header(raw as *mut c_void).temp)
    }
}

/// `stbds_hmfree(p)`
pub unsafe fn hmfree(api: &Api, t: *mut c_void, elemsize: usize) {
    if !t.is_null() {
        api.hmfree_func((t as *mut u8).sub(elemsize) as *mut c_void, elemsize);
    }
}

/// `stbds_hmlen(t)`
pub unsafe fn hmlen(t: *mut c_void, elemsize: usize) -> isize {
    if t.is_null() {
        0
    } else {
        let raw = (t as *mut u8).sub(elemsize);
        header(raw as *mut c_void).length as isize - 1
    }
}

/// Deterministic PRNG shared by the test files.
pub struct Rng(pub u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng(seed)
    }
    pub fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    pub fn below(&mut self, n: u64) -> u64 {
        self.next() % n
    }
}
