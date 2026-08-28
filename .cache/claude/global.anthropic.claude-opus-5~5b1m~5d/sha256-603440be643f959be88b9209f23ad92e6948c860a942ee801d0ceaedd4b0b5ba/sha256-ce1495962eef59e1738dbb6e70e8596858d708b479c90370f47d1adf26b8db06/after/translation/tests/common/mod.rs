//! Shared differential-test harness.
//!
//! BOTH libraries are loaded through `libloading` and every call goes through an
//! exported `.so` symbol — the Rust functions are never called directly, so the
//! `#[no_mangle] extern "C"` wrappers are exercised exactly as an external C
//! consumer would exercise them.

#![allow(dead_code)]
#![allow(non_camel_case_types)]

use std::ffi::{c_char, c_int, c_void};
use std::path::PathBuf;
use std::sync::OnceLock;

// ---------------------------------------------------------------------------
// Mirrors of the C structures (from c_src/src/lib.c).  These are the *C*
// definitions; using them to read state written by either library is what makes
// a Rust layout mismatch observable.
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
#[derive(Clone, Copy, Debug)]
pub struct StringArena {
    pub storage: *mut c_void,
    pub remaining: usize,
    pub block: u8,
    pub mode: u8,
}

impl StringArena {
    pub const fn zeroed() -> Self {
        StringArena {
            storage: std::ptr::null_mut(),
            remaining: 0,
            block: 0,
            mode: 0,
        }
    }
}

#[repr(C)]
pub struct StringBlock {
    pub next: *mut StringBlock,
    pub storage: [c_char; 8],
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
    pub string: StringArena,
    pub storage: *mut HashBucket,
}

pub const HDR: usize = std::mem::size_of::<ArrayHeader>(); // 32

// The `STBDS_HM_*` "enum" (#define'd ints in the C source).
pub const HM_BINARY: c_int = 0;
pub const HM_STRING: c_int = 1;

// enum { STBDS_SH_NONE, STBDS_SH_DEFAULT, STBDS_SH_STRDUP, STBDS_SH_ARENA }
pub const SH_NONE: c_int = 0;
pub const SH_DEFAULT: c_int = 1;
pub const SH_STRDUP: c_int = 2;
pub const SH_ARENA: c_int = 3;

pub const INDEX_EMPTY: isize = -1;
pub const INDEX_DELETED: isize = -2;

// ---------------------------------------------------------------------------
// Exported-symbol signatures
// ---------------------------------------------------------------------------

pub type FnArrgrowf = unsafe extern "C" fn(*mut c_void, usize, usize, usize) -> *mut c_void;
pub type FnArrfreef = unsafe extern "C" fn(*mut c_void);
pub type FnRandSeed = unsafe extern "C" fn(usize);
pub type FnHashString = unsafe extern "C" fn(*mut c_char, usize) -> usize;
pub type FnHashBytes = unsafe extern "C" fn(*mut c_void, usize, usize) -> usize;
pub type FnHmfreeFunc = unsafe extern "C" fn(*mut c_void, usize);
pub type FnHmgetKeyTs =
    unsafe extern "C" fn(*mut c_void, usize, *mut c_void, usize, *mut isize, c_int) -> *mut c_void;
pub type FnHmgetKey =
    unsafe extern "C" fn(*mut c_void, usize, *mut c_void, usize, c_int) -> *mut c_void;
pub type FnHmputDefault = unsafe extern "C" fn(*mut c_void, usize) -> *mut c_void;
pub type FnHmputKey =
    unsafe extern "C" fn(*mut c_void, usize, *mut c_void, usize, c_int) -> *mut c_void;
pub type FnShmodeFunc = unsafe extern "C" fn(usize, c_int) -> *mut c_void;
pub type FnHmdelKey =
    unsafe extern "C" fn(*mut c_void, usize, *mut c_void, usize, usize, c_int) -> *mut c_void;
pub type FnStralloc = unsafe extern "C" fn(*mut StringArena, *mut c_char) -> *mut c_char;
pub type FnStrreset = unsafe extern "C" fn(*mut StringArena);
pub type FnStrkey = unsafe extern "C" fn(c_int) -> *mut c_char;
pub type FnShPuts = unsafe extern "C" fn(c_int);

/// The 16 symbols the C `.so` exports (see SYMBOLS.md).
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
    "sh_puts",
];

/// Only raw `extern "C"` function pointers are kept, so `Lib` is trivially
/// `Send + Sync`; the `libloading::Library` handle is intentionally leaked so
/// the `.so` stays mapped for the whole test process.
#[derive(Clone, Copy)]
pub struct Lib {
    pub name: &'static str,
    pub arrgrowf: FnArrgrowf,
    pub arrfreef: FnArrfreef,
    pub rand_seed: FnRandSeed,
    pub hash_string: FnHashString,
    pub hash_bytes: FnHashBytes,
    pub hmfree_func: FnHmfreeFunc,
    pub hmget_key_ts: FnHmgetKeyTs,
    pub hmget_key: FnHmgetKey,
    pub hmput_default: FnHmputDefault,
    pub hmput_key: FnHmputKey,
    pub shmode_func: FnShmodeFunc,
    pub hmdel_key: FnHmdelKey,
    pub stralloc: FnStralloc,
    pub strreset: FnStrreset,
    pub strkey: FnStrkey,
    pub sh_puts: FnShPuts,
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

pub fn c_so_path() -> PathBuf {
    let dir = manifest_dir().parent().unwrap().join("c_src").join("build");
    let rd = std::fs::read_dir(&dir).unwrap_or_else(|e| {
        panic!(
            "cannot read {dir:?} ({e}). Build the C library first:\n  \
             cd c_src && mkdir -p build && cd build && \
             cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build ."
        )
    });
    let mut found: Option<PathBuf> = None;
    for e in rd {
        let p = e.unwrap().path();
        let n = match p.file_name().and_then(|s| s.to_str()) {
            Some(n) => n,
            None => continue,
        };
        if n.starts_with("lib") && n.ends_with(".so") {
            found = Some(p);
        }
    }
    found.unwrap_or_else(|| panic!("no lib*.so found in {dir:?}"))
}

pub fn rust_so_path() -> PathBuf {
    // Explicit override, so the whole suite can be re-run against a differently
    // built artifact (e.g. the debug profile, which enables overflow checks).
    if let Ok(p) = std::env::var("RUST_TRANSLATION_SO") {
        let p = PathBuf::from(p);
        assert!(p.exists(), "RUST_TRANSLATION_SO={p:?} does not exist");
        return p;
    }
    // Otherwise prefer the release artifact: `[profile.release] panic = "abort"`
    // and overflow-checks off are the semantics that mirror the C build.
    for profile in ["release", "debug"] {
        let p = manifest_dir()
            .join("target")
            .join(profile)
            .join("libsh_puts_lib.so");
        if p.exists() {
            return p;
        }
    }
    panic!("libsh_puts_lib.so not found; run `cargo build --release --offline` first");
}

macro_rules! load {
    ($lib:expr, $name:literal) => {{
        let sym: libloading::Symbol<_> = unsafe { $lib.get(concat!($name, "\0").as_bytes()) }
            .unwrap_or_else(|e| panic!("symbol {} missing: {e}", $name));
        *sym
    }};
}

fn load_lib(path: PathBuf, name: &'static str) -> Lib {
    let lib = unsafe { libloading::Library::new(&path) }
        .unwrap_or_else(|e| panic!("dlopen {path:?} failed: {e}"));
    let out = Lib {
        name,
        arrgrowf: load!(lib, "stbds_arrgrowf"),
        arrfreef: load!(lib, "stbds_arrfreef"),
        rand_seed: load!(lib, "stbds_rand_seed"),
        hash_string: load!(lib, "stbds_hash_string"),
        hash_bytes: load!(lib, "stbds_hash_bytes"),
        hmfree_func: load!(lib, "stbds_hmfree_func"),
        hmget_key_ts: load!(lib, "stbds_hmget_key_ts"),
        hmget_key: load!(lib, "stbds_hmget_key"),
        hmput_default: load!(lib, "stbds_hmput_default"),
        hmput_key: load!(lib, "stbds_hmput_key"),
        shmode_func: load!(lib, "stbds_shmode_func"),
        hmdel_key: load!(lib, "stbds_hmdel_key"),
        stralloc: load!(lib, "stbds_stralloc"),
        strreset: load!(lib, "stbds_strreset"),
        strkey: load!(lib, "strkey"),
        sh_puts: load!(lib, "sh_puts"),
    };
    std::mem::forget(lib); // keep mapped for the process lifetime
    out
}

static C_LIB: OnceLock<Lib> = OnceLock::new();
static R_LIB: OnceLock<Lib> = OnceLock::new();

/// The C ground-truth library.
pub fn c() -> &'static Lib {
    C_LIB.get_or_init(|| load_lib(c_so_path(), "C"))
}

/// The Rust translation, loaded as a plain shared object.
pub fn r() -> &'static Lib {
    R_LIB.get_or_init(|| load_lib(rust_so_path(), "Rust"))
}

/// Both libraries, plus a deterministic reset of the *global* hash seed that
/// `stbds_make_hash_index` consumes and advances.
pub fn both() -> (&'static Lib, &'static Lib) {
    (c(), r())
}

static GLOBAL_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// Both libraries keep **process-global mutable state** — the `stbds_hash_seed`
/// static that every fresh `stbds_make_hash_index` reads *and then advances*, and
/// (for `strkey`) a 256-byte static buffer.  Differential tests must therefore
/// run one at a time inside a test binary: every test takes this guard first.
/// It also serialises the fd-1 redirection used by `capture_stdout`.
#[must_use]
pub fn lock() -> std::sync::MutexGuard<'static, ()> {
    GLOBAL_LOCK.lock().unwrap_or_else(|e| e.into_inner())
}

/// Force both libraries' `stbds_hash_seed` statics to the same value so that
/// every subsequent table gets identical seeds (and therefore identical probe
/// positions and bucket contents).
pub fn sync_seed(seed: usize) {
    unsafe {
        (c().rand_seed)(seed);
        (r().rand_seed)(seed);
    }
}

/// The library's own compile-time default (`static size_t stbds_hash_seed = 0x31415926`).
pub const DEFAULT_SEED: usize = 0x31415926;

// ---------------------------------------------------------------------------
// Pointer helpers mirroring the C macros
// ---------------------------------------------------------------------------

pub unsafe fn header(t: *mut c_void) -> *mut ArrayHeader {
    (t as *mut u8).wrapping_sub(HDR) as *mut ArrayHeader
}

pub fn arr_to_hash(a: *mut c_void, elemsize: usize) -> *mut c_void {
    (a as *mut u8).wrapping_add(elemsize) as *mut c_void
}

pub fn hash_to_arr(a: *mut c_void, elemsize: usize) -> *mut c_void {
    (a as *mut u8).wrapping_sub(elemsize) as *mut c_void
}

// ---------------------------------------------------------------------------
// Snapshot / formatting helpers.  Everything address-dependent is reduced to
// "null" / "set" (heap addresses legitimately differ between the two libraries);
// everything else is compared byte-for-byte.
// ---------------------------------------------------------------------------

pub unsafe fn hex(p: *const u8, n: usize) -> String {
    let mut s = String::with_capacity(2 * n);
    for i in 0..n {
        s += &format!("{:02x}", unsafe { *p.add(i) });
    }
    s
}

pub unsafe fn cstr_opt(p: *const c_char) -> String {
    if p.is_null() {
        return "<null>".into();
    }
    let mut v = Vec::new();
    let mut i = 0usize;
    unsafe {
        while *p.add(i) != 0 {
            v.push(*p.add(i) as u8);
            i += 1;
            if i > 4_000_000 {
                return "<runaway>".into();
            }
        }
    }
    format!("{:?}", String::from_utf8_lossy(&v))
}

fn nullness(p: *const c_void) -> &'static str {
    if p.is_null() { "null" } else { "set" }
}

pub unsafe fn snap_hdr(a: *mut c_void) -> String {
    if a.is_null() {
        return "hdr=NULL".into();
    }
    let h = unsafe { &*header(a) };
    format!(
        "hdr[len={} cap={} ht={} temp={}]",
        h.length,
        h.capacity,
        nullness(h.hash_table),
        h.temp
    )
}

/// How to interpret the first bytes of each map element when dumping it.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum KeyKind {
    /// Raw bytes (binary keys, and string modes whose `switch` fell through to
    /// `memcpy` so the element holds copied *string data*, not a pointer).
    Bin,
    /// `char *` at offset 0 — dump the pointed-to string, not the address.
    StrPtr,
}

pub unsafe fn snap_arena(a: &StringArena) -> String {
    format!(
        "arena[storage={} remaining={} block={} mode={}]",
        nullness(a.storage),
        a.remaining,
        a.block,
        a.mode
    )
}

/// Full observable state of a hash map given its "hash pointer" `t`
/// (i.e. what the `stbds_hm*` functions return).
pub unsafe fn snap_map(t: *mut c_void, elemsize: usize, kk: KeyKind) -> String {
    if t.is_null() {
        return "MAP=NULL".into();
    }
    unsafe {
        let a = hash_to_arr(t, elemsize);
        let h = &*header(a);
        let mut s = String::new();
        s += &format!("{}\n", snap_hdr(a));
        if h.hash_table.is_null() {
            s += "table=NULL\n";
        } else {
            let ti = &*(h.hash_table as *mut HashIndex);
            s += &format!(
                "tbl[sc={} uc={} uct={} ucst={} tc={} tct={} seed={:#018x} log2={}]\n",
                ti.slot_count,
                ti.used_count,
                ti.used_count_threshold,
                ti.used_count_shrink_threshold,
                ti.tombstone_count,
                ti.tombstone_count_threshold,
                ti.seed,
                ti.slot_count_log2
            );
            s += &format!("tbl.string={}\n", snap_arena(&ti.string));
            // NOTE: `stbds_hash_index::temp_key` is deliberately NOT part of the
            // snapshot.  `stbds_make_hash_index` never initialises it (and never
            // copies it from the old table when growing/shrinking/rebuilding), so
            // it holds *uninitialised heap bytes* until the first
            // pointer-storing insert.  Both libraries reproduce that faithfully,
            // but the garbage naturally differs.  `Drv::temp_key_str` checks it
            // explicitly at the points where the C guarantees it is valid.
            // `t->storage = STBDS_ALIGN_FWD((size_t)(t+1), 64)`.  The *value* of
            // the offset depends on `malloc`'s base address mod 64, so only the
            // invariant is compared: 64-byte aligned, past the header, and still
            // inside the `(sc>>3)*128 + 104 + 63` allocation.
            let off = (ti.storage as usize).wrapping_sub(h.hash_table as usize);
            s += &format!(
                "tbl.storage_aligned={} off_ok={}\n",
                (ti.storage as usize) % 64 == 0,
                off >= std::mem::size_of::<HashIndex>()
                    && off < std::mem::size_of::<HashIndex>() + 64
            );
            for i in 0..(ti.slot_count >> 3) {
                let b = &*ti.storage.add(i);
                s += &format!("b{i}:");
                for j in 0..8 {
                    s += &format!(" {:#018x}/{}", b.hash[j], b.index[j]);
                }
                s.push('\n');
            }
        }
        for i in 0..h.length {
            let p = (a as *mut u8).add(elemsize * i);
            match kk {
                KeyKind::Bin => s += &format!("e{i}: {}\n", hex(p, elemsize)),
                KeyKind::StrPtr => {
                    let kp = *(p as *mut *mut c_char);
                    s += &format!(
                        "e{i}: key={} rest={}\n",
                        cstr_opt(kp),
                        hex(p.add(8), elemsize.saturating_sub(8))
                    );
                }
            }
        }
        s
    }
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (splitmix64) — fixed seeds keep every test reproducible.
// ---------------------------------------------------------------------------

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Rng(seed)
    }
    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    pub fn below(&mut self, n: u64) -> u64 {
        assert!(n > 0);
        self.next_u64() % n
    }
    pub fn byte(&mut self) -> u8 {
        (self.next_u64() >> 24) as u8
    }
    pub fn bytes(&mut self, n: usize) -> Vec<u8> {
        (0..n).map(|_| self.byte()).collect()
    }
    /// A NUL-terminated random string of `n` printable-ish non-zero bytes.
    pub fn cstring(&mut self, n: usize) -> Vec<u8> {
        let mut v: Vec<u8> = (0..n).map(|_| 1 + (self.next_u64() % 255) as u8).collect();
        v.push(0);
        v
    }
}

// ---------------------------------------------------------------------------
// A tiny model of the map macros so tests can drive the low-level entry points
// the way stb_ds.h's macros do.
// ---------------------------------------------------------------------------

/// `stbds_hmlen(t)` == `header((t)-1)->length - 1`
pub unsafe fn hmlen(t: *mut c_void, elemsize: usize) -> isize {
    if t.is_null() {
        0
    } else {
        unsafe { (*header(hash_to_arr(t, elemsize))).length as isize - 1 }
    }
}

/// `stbds_temp((t)-1)`
pub unsafe fn temp_of(t: *mut c_void, elemsize: usize) -> isize {
    unsafe { (*header(hash_to_arr(t, elemsize))).temp }
}

/// `stbds_temp_key((t)-1)`
pub unsafe fn temp_key_of(t: *mut c_void, elemsize: usize) -> *mut c_char {
    unsafe {
        let ht = (*header(hash_to_arr(t, elemsize))).hash_table;
        if ht.is_null() {
            std::ptr::null_mut()
        } else {
            *(ht as *mut *mut c_char)
        }
    }
}

/// Write the deterministic "value" part of element `idx` so that *no* byte of a
/// live element is ever uninitialised (uninitialised padding would differ
/// between the two heaps for reasons unrelated to correctness).
pub unsafe fn write_value(t: *mut c_void, elemsize: usize, idx: isize, valstart: usize, tag: u8) {
    unsafe {
        let a = hash_to_arr(t, elemsize) as *mut u8;
        let e = a.add(elemsize * (idx as usize + 1));
        for k in valstart..elemsize {
            *e.add(k) = tag.wrapping_add(k as u8);
        }
    }
}

// ---------------------------------------------------------------------------
// stdout capture (for sh_puts, which prints via libc printf).
// ---------------------------------------------------------------------------

unsafe extern "C" {
    fn dup(oldfd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
}

static CAPTURE_SEQ: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

/// Redirect fd 1 to a temp file, run `f`, flush libc's stdio, restore fd 1 and
/// return everything that was written.
///
/// The caller **must** be holding [`lock()`], which serialises fd-1 surgery.
pub fn capture_stdout<F: FnOnce()>(f: F) -> Vec<u8> {
    use std::io::{Read, Seek, SeekFrom};
    use std::os::unix::io::AsRawFd;
    use std::sync::atomic::Ordering;

    let n = CAPTURE_SEQ.fetch_add(1, Ordering::SeqCst);
    let path = std::env::temp_dir().join(format!("shputs_cap_{}_{}.txt", std::process::id(), n));
    let mut file = std::fs::File::options()
        .read(true)
        .write(true)
        .create(true)
        .truncate(true)
        .open(&path)
        .unwrap_or_else(|e| panic!("cannot create {path:?}: {e}"));
    let fd = file.as_raw_fd();
    unsafe {
        fflush(std::ptr::null_mut());
        let saved = dup(1);
        assert!(saved >= 0, "dup(1) failed");
        assert!(dup2(fd, 1) >= 0, "dup2 failed");
        f();
        fflush(std::ptr::null_mut());
        assert!(dup2(saved, 1) >= 0, "dup2 restore failed");
        close(saved);
    }
    let mut out = Vec::new();
    file.seek(SeekFrom::Start(0)).unwrap();
    file.read_to_end(&mut out).unwrap();
    let _ = std::fs::remove_file(&path);
    out
}

/// Pretty-print a byte string for assertion messages.
pub fn show(b: &[u8]) -> String {
    format!("{:?}", String::from_utf8_lossy(b))
}

/// Assert two snapshots are equal, printing a first-difference hint.
#[track_caller]
pub fn eqs(ctx: &str, c_snap: &str, r_snap: &str) {
    if c_snap == r_snap {
        return;
    }
    let cl: Vec<&str> = c_snap.lines().collect();
    let rl: Vec<&str> = r_snap.lines().collect();
    let mut diff = String::new();
    for i in 0..cl.len().max(rl.len()) {
        let a = cl.get(i).copied().unwrap_or("<missing>");
        let b = rl.get(i).copied().unwrap_or("<missing>");
        if a != b {
            diff += &format!("  line {i}:\n    C   : {a}\n    Rust: {b}\n");
        }
    }
    panic!("DIVERGENCE [{ctx}]\n{diff}");
}

// ---------------------------------------------------------------------------
// Hash-map driver.
//
// stb_ds.h drives the exported entry points from macros; the driver reproduces
// exactly what those macros do around each call:
//
//   shput(t,k,v)  -> t = hmput_key(t, sizeof*t, k, sizeof t->key, HM_STRING),
//                    t[temp(t-1)].value = v
//   hmgeti(t,k)   -> t = hmget_key (t, sizeof*t, &k, sizeof t->key, HM_BINARY),
//                    temp(t-1)
//   hmgeti_ts     -> t = hmget_key_ts(..., &temp, ...), temp
//   hmdel(t,k)    -> t = hmdel_key(...), t ? temp(t-1) : 0
//   hmdefault(t,v)-> t = hmput_default(t, sizeof*t), t[-1].value = v
//   hmfree(p)     -> hmfree_func(p-1, sizeof*p), p = NULL
// ---------------------------------------------------------------------------

pub struct Drv {
    pub lib: &'static Lib,
    pub t: *mut c_void,
    pub es: usize,
    pub ks: usize,
    pub mode: c_int,
}

impl Drv {
    /// A map that starts life as `NULL` (the plain `hmput`/`shput` path).
    pub fn empty(lib: &'static Lib, es: usize, ks: usize, mode: c_int) -> Self {
        Drv { lib, t: std::ptr::null_mut(), es, ks, mode }
    }

    /// A map created up-front by `stbds_shmode_func` (`sh_new_arena`,
    /// `sh_new_strdup`, or any out-of-range mode value).
    pub fn shmode(lib: &'static Lib, es: usize, ks: usize, mode: c_int, sh: c_int) -> Self {
        let t = unsafe { (lib.shmode_func)(es, sh) };
        Drv { lib, t, es, ks, mode }
    }

    pub unsafe fn table(&self) -> *mut HashIndex {
        if self.t.is_null() {
            return std::ptr::null_mut();
        }
        unsafe { (*header(hash_to_arr(self.t, self.es))).hash_table as *mut HashIndex }
    }

    /// `table->string.mode`, or 0 when there is no table yet.
    pub unsafe fn string_mode(&self) -> u8 {
        unsafe {
            let ti = self.table();
            if ti.is_null() { 0 } else { (*ti).string.mode }
        }
    }

    /// Does the library store a `char *` at element offset 0, or raw key bytes?
    ///
    /// `stbds_hmput_key`'s `switch (table->string.mode)` looks **only** at
    /// `table->string.mode` — it is completely independent of the `mode`
    /// argument.  So e.g. `shmode_func(es, STBDS_SH_STRDUP)` + `mode ==
    /// STBDS_HM_BINARY` stores a `strdup`ed pointer even though every *lookup*
    /// then `memcmp`s the raw key bytes against that pointer's bytes.
    pub unsafe fn stores_pointer(&self) -> bool {
        let sm = unsafe { self.string_mode() };
        sm == 1 || sm == 2 || sm == 3
    }

    /// First byte of the element the *library* does not initialise, i.e. where
    /// the caller's `value` member starts.
    pub unsafe fn valstart(&self) -> usize {
        if unsafe { self.stores_pointer() } { 8 } else { self.ks }
    }

    pub unsafe fn kind(&self) -> KeyKind {
        if unsafe { self.stores_pointer() } { KeyKind::StrPtr } else { KeyKind::Bin }
    }

    unsafe fn write_val(&self, idx: isize, tag: u8) {
        unsafe {
            let vs = self.valstart();
            let a = hash_to_arr(self.t, self.es) as *mut u8;
            let e = a.add(self.es * (idx as usize + 1));
            for k in vs..self.es {
                *e.add(k) = tag.wrapping_add((k as u8).wrapping_mul(7));
            }
        }
    }

    /// `hmput` / `shput`
    pub unsafe fn put(&mut self, key: &[u8], tag: u8) -> isize {
        unsafe {
            self.t = (self.lib.hmput_key)(
                self.t,
                self.es,
                key.as_ptr() as *mut c_void,
                self.ks,
                self.mode,
            );
            let idx = temp_of(self.t, self.es);
            self.write_val(idx, tag);
            idx
        }
    }

    /// `shputs` — writes the whole struct then re-reads the key from
    /// `stbds_temp_key`, exactly like the macro `sh_puts` uses.
    pub unsafe fn puts_struct(&mut self, key: &[u8], tag: u8) -> (isize, String) {
        unsafe {
            self.t = (self.lib.hmput_key)(
                self.t,
                self.es,
                key.as_ptr() as *mut c_void,
                self.ks,
                self.mode,
            );
            let idx = temp_of(self.t, self.es);
            let a = hash_to_arr(self.t, self.es) as *mut u8;
            let e = a.add(self.es * (idx as usize + 1));
            // (t)[temp] = s  -- whole struct, key member included
            for k in 0..self.es {
                *e.add(k) = if k < 8 { 0 } else { tag.wrapping_add(k as u8) };
            }
            // (t)[temp].key = stbds_temp_key((t)-1)
            let tk = temp_key_of(self.t, self.es);
            *(e as *mut *mut c_char) = tk;
            (idx, cstr_opt(tk))
        }
    }

    /// `hmgeti` / `shgeti`
    pub unsafe fn get(&mut self, key: &[u8]) -> isize {
        unsafe {
            self.t = (self.lib.hmget_key)(
                self.t,
                self.es,
                key.as_ptr() as *mut c_void,
                self.ks,
                self.mode,
            );
            temp_of(self.t, self.es)
        }
    }

    /// `hmgeti_ts`
    pub unsafe fn get_ts(&mut self, key: &[u8]) -> isize {
        unsafe {
            let mut tmp: isize = 0x5555_5555_5555_5555;
            self.t = (self.lib.hmget_key_ts)(
                self.t,
                self.es,
                key.as_ptr() as *mut c_void,
                self.ks,
                &mut tmp,
                self.mode,
            );
            tmp
        }
    }

    /// `hmdel` / `shdel`
    pub unsafe fn del(&mut self, key: &[u8], keyoffset: usize) -> isize {
        unsafe {
            self.t = (self.lib.hmdel_key)(
                self.t,
                self.es,
                key.as_ptr() as *mut c_void,
                self.ks,
                keyoffset,
                self.mode,
            );
            if self.t.is_null() { 0 } else { temp_of(self.t, self.es) }
        }
    }

    /// `hmdefault`
    pub unsafe fn put_default(&mut self, tag: u8) {
        unsafe {
            self.t = (self.lib.hmput_default)(self.t, self.es);
            // (t)[-1].value = v
            let vs = self.valstart();
            let a = hash_to_arr(self.t, self.es) as *mut u8;
            for k in vs..self.es {
                *a.add(k) = tag.wrapping_add(k as u8);
            }
        }
    }

    pub unsafe fn len(&self) -> isize {
        unsafe { hmlen(self.t, self.es) }
    }

    /// `stbds_temp_key((t)-1)` rendered as a string.
    ///
    /// Only valid right after a pointer-storing insert (`STBDS_SH_DEFAULT`,
    /// `_STRDUP`, `_ARENA`) that either found an empty slot or matched an
    /// existing key in the *first* probe scan — those are the only paths in
    /// `stbds_hmput_key` that assign it.  See the note in `snap_map`.
    pub unsafe fn temp_key_str(&self) -> String {
        unsafe { cstr_opt(temp_key_of(self.t, self.es)) }
    }

    pub unsafe fn snap(&self) -> String {
        unsafe { snap_map(self.t, self.es, self.kind()) }
    }

    pub unsafe fn free(&mut self) {
        unsafe {
            if !self.t.is_null() {
                (self.lib.hmfree_func)(hash_to_arr(self.t, self.es), self.es);
                self.t = std::ptr::null_mut();
            }
        }
    }
}

/// One driver operation, so an identical script can be replayed on both libraries.
#[derive(Clone, Copy, Debug)]
pub enum Op {
    Put(usize, u8),
    PutStruct(usize, u8),
    Get(usize),
    GetTs(usize),
    Del(usize, usize),
    Default(u8),
    Len,
}

/// Replay `ops` against both libraries, comparing the returned value *and* the
/// complete map state after every single step.
pub fn run_ops(ctx: &str, mut cd: Drv, mut rd: Drv, keys: &[Vec<u8>], ops: &[Op]) {
    unsafe {
        for (i, op) in ops.iter().enumerate() {
            let where_ = format!("{ctx} op#{i} {op:?}");
            match *op {
                Op::Put(k, tag) => {
                    let a = cd.put(&keys[k], tag);
                    let b = rd.put(&keys[k], tag);
                    assert_eq!(a, b, "{where_}: temp index");
                }
                Op::PutStruct(k, tag) => {
                    let a = cd.puts_struct(&keys[k], tag);
                    let b = rd.puts_struct(&keys[k], tag);
                    assert_eq!(a, b, "{where_}: (temp, temp_key)");
                }
                Op::Get(k) => {
                    let a = cd.get(&keys[k]);
                    let b = rd.get(&keys[k]);
                    assert_eq!(a, b, "{where_}: hmgeti");
                }
                Op::GetTs(k) => {
                    let a = cd.get_ts(&keys[k]);
                    let b = rd.get_ts(&keys[k]);
                    assert_eq!(a, b, "{where_}: hmgeti_ts");
                }
                Op::Del(k, off) => {
                    let a = cd.del(&keys[k], off);
                    let b = rd.del(&keys[k], off);
                    assert_eq!(a, b, "{where_}: hmdel");
                }
                Op::Default(tag) => {
                    cd.put_default(tag);
                    rd.put_default(tag);
                }
                Op::Len => {
                    assert_eq!(cd.len(), rd.len(), "{where_}: hmlen");
                }
            }
            eqs(&where_, &cd.snap(), &rd.snap());
        }
        cd.free();
        rd.free();
    }
}

/// A key buffer with 24 bytes of zero padding after the NUL, so that a
/// `memcpy(dst, key, keysize)` with `keysize` larger than the string is still an
/// in-bounds, deterministic read (the `STBDS_SH_NONE` + string-mode combination
/// does exactly that).
pub fn padded_key(bytes: &[u8]) -> Vec<u8> {
    let mut v = bytes.to_vec();
    v.push(0);
    v.extend_from_slice(&[0u8; 24]);
    v
}

/// `keysize`-byte binary key from a u64, zero padded.
pub fn bin_key(v: u64, keysize: usize) -> Vec<u8> {
    let mut b = vec![0u8; keysize.max(8)];
    b[..8].copy_from_slice(&v.to_le_bytes());
    b.truncate(keysize.max(1));
    b.extend_from_slice(&[0u8; 24]);
    b
}

/// Blank out the header's `temp` field in a snapshot.
///
/// `temp` is the library's out-parameter channel (`stbds_temp`), so *every*
/// lookup and delete legitimately rewrites it.  Tests that assert "this rejected
/// call changed nothing else" mask it out.
pub fn mask_temp(s: &str) -> String {
    let trailing_nl = s.ends_with('\n');
    let mut out: String = s
        .lines()
        .map(|l| match l.find(" temp=") {
            Some(i) if l.ends_with(']') => format!("{} temp=*]", &l[..i]),
            _ => l.to_string(),
        })
        .collect::<Vec<_>>()
        .join("\n");
    if trailing_nl {
        out.push('\n');
    }
    out
}
