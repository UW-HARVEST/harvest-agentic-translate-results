//! Shared harness: loads BOTH the C `.so` and the Rust `.so` through
//! `libloading` and drives them exclusively through their exported symbols.
//!
//! Nothing in here calls a Rust function directly; the `translation` crate is
//! never `use`d.

#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::ffi::c_void;
use std::os::raw::{c_char, c_int};
use std::path::PathBuf;

// ---------------------------------------------------------------------------
// Mirrors of the C layouts, used only to *inspect* memory the libraries own.
// ---------------------------------------------------------------------------

pub const BUCKET_LENGTH: usize = 8;
pub const BUCKET_SHIFT: usize = 3;

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

pub const HM_BINARY: c_int = 0;
pub const HM_STRING: c_int = 1;

pub const SH_NONE: c_int = 0;
pub const SH_DEFAULT: c_int = 1;
pub const SH_STRDUP: c_int = 2;
pub const SH_ARENA: c_int = 3;

pub const STBDS_INDEX_EMPTY: isize = -1;
pub const STBDS_INDEX_DELETED: isize = -2;
pub const STBDS_HASH_EMPTY: usize = 0;
pub const STBDS_HASH_DELETED: usize = 1;

// ---------------------------------------------------------------------------
// Loaded library
// ---------------------------------------------------------------------------

type FnArrgrowf = unsafe extern "C" fn(*mut c_void, usize, usize, usize) -> *mut c_void;
type FnArrfreef = unsafe extern "C" fn(*mut c_void);
type FnRandSeed = unsafe extern "C" fn(usize);
type FnHashBytes = unsafe extern "C" fn(*mut c_void, usize, usize) -> usize;
type FnHashString = unsafe extern "C" fn(*mut c_char, usize) -> usize;
type FnHmfreeFunc = unsafe extern "C" fn(*mut c_void, usize);
type FnHmgetKey = unsafe extern "C" fn(*mut c_void, usize, *mut c_void, usize, c_int) -> *mut c_void;
type FnHmgetKeyTs =
    unsafe extern "C" fn(*mut c_void, usize, *mut c_void, usize, *mut isize, c_int) -> *mut c_void;
type FnHmputDefault = unsafe extern "C" fn(*mut c_void, usize) -> *mut c_void;
type FnHmputKey = unsafe extern "C" fn(*mut c_void, usize, *mut c_void, usize, c_int) -> *mut c_void;
type FnHmdelKey =
    unsafe extern "C" fn(*mut c_void, usize, *mut c_void, usize, usize, c_int) -> *mut c_void;
type FnShmodeFunc = unsafe extern "C" fn(usize, c_int) -> *mut c_void;
type FnStralloc = unsafe extern "C" fn(*mut StringArena, *mut c_char) -> *mut c_char;
type FnStrreset = unsafe extern "C" fn(*mut StringArena);
type FnArrPush = unsafe extern "C" fn(c_int);
type FnStrkey = unsafe extern "C" fn(c_int) -> *mut c_char;

pub struct Lib {
    pub name: &'static str,
    _lib: Library,
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
    pub arr_push: FnArrPush,
    pub strkey: FnStrkey,
}

macro_rules! sym {
    ($lib:expr, $ty:ty, $name:literal) => {{
        let s: Symbol<$ty> = unsafe {
            $lib.get(concat!($name, "\0").as_bytes())
                .unwrap_or_else(|e| panic!("missing symbol {}: {}", $name, e))
        };
        *s
    }};
}

impl Lib {
    fn open(name: &'static str, path: &PathBuf) -> Lib {
        let lib = unsafe {
            Library::new(path).unwrap_or_else(|e| panic!("cannot load {}: {}", path.display(), e))
        };
        let l = Lib {
            name,
            arrgrowf: sym!(lib, FnArrgrowf, "stbds_arrgrowf"),
            arrfreef: sym!(lib, FnArrfreef, "stbds_arrfreef"),
            rand_seed: sym!(lib, FnRandSeed, "stbds_rand_seed"),
            hash_bytes: sym!(lib, FnHashBytes, "stbds_hash_bytes"),
            hash_string: sym!(lib, FnHashString, "stbds_hash_string"),
            hmfree_func: sym!(lib, FnHmfreeFunc, "stbds_hmfree_func"),
            hmget_key: sym!(lib, FnHmgetKey, "stbds_hmget_key"),
            hmget_key_ts: sym!(lib, FnHmgetKeyTs, "stbds_hmget_key_ts"),
            hmput_default: sym!(lib, FnHmputDefault, "stbds_hmput_default"),
            hmput_key: sym!(lib, FnHmputKey, "stbds_hmput_key"),
            hmdel_key: sym!(lib, FnHmdelKey, "stbds_hmdel_key"),
            shmode_func: sym!(lib, FnShmodeFunc, "stbds_shmode_func"),
            stralloc: sym!(lib, FnStralloc, "stbds_stralloc"),
            strreset: sym!(lib, FnStrreset, "stbds_strreset"),
            arr_push: sym!(lib, FnArrPush, "arr_push"),
            strkey: sym!(lib, FnStrkey, "strkey"),
            _lib: lib,
        };
        l
    }
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn find_c_so() -> PathBuf {
    let build = manifest_dir().parent().unwrap().join("c_src").join("build");
    let mut found: Option<PathBuf> = None;
    for e in std::fs::read_dir(&build)
        .unwrap_or_else(|e| panic!("no c_src/build ({}); build the C lib first: {}", build.display(), e))
    {
        let p = e.unwrap().path();
        if p.extension().map(|x| x == "so").unwrap_or(false) {
            found = Some(p);
        }
    }
    found.unwrap_or_else(|| panic!("no .so in {}", build.display()))
}

fn find_rust_so() -> PathBuf {
    // Load the RELEASE cdylib: that is the artifact an external caller links
    // against, and it is the one whose codegen settings (`panic = "abort"`, no
    // debug assertions) match the C library's. A debug cdylib inserts Rust's
    // null-pointer-dereference check, which turns the deliberate UB paths the C
    // has (e.g. `realloc` failure in `stbds_arrgrowf`) into SIGABRT instead of
    // SIGSEGV. Override with RUST_SO if needed.
    if let Ok(p) = std::env::var("RUST_SO") {
        return PathBuf::from(p);
    }
    let t = manifest_dir().join("target");
    let mut cands = vec![
        t.join("release").join("libarr_push_lib.so"),
        t.join("debug").join("libarr_push_lib.so"),
    ];
    let exe = std::env::current_exe().expect("current_exe");
    if let Some(deps) = exe.parent() {
        if let Some(profile) = deps.parent() {
            cands.push(profile.join("libarr_push_lib.so"));
        }
    }
    for c in &cands {
        if c.exists() {
            return c.clone();
        }
    }
    panic!(
        "cannot find libarr_push_lib.so; run `cargo build --release` first. tried {:?}",
        cands
    );
}

pub fn load_pair() -> (Lib, Lib) {
    let c = Lib::open("C", &find_c_so());
    let r = Lib::open("RUST", &find_rust_so());
    (c, r)
}

// ---------------------------------------------------------------------------
// Serialisation + seed synchronisation.
//
// Both libraries keep a *mutable global* `stbds_hash_seed` that every freshly
// built hash index consumes and then advances. `dlopen` is process-wide, so
// concurrently running tests would interleave their consumption of that global
// and desynchronise the two libraries. Every test therefore takes this lock and
// re-seeds both libraries to a known value.
// ---------------------------------------------------------------------------

static SERIAL: std::sync::Mutex<()> = std::sync::Mutex::new(());

pub struct Harness {
    pub c: Lib,
    pub r: Lib,
    _g: std::sync::MutexGuard<'static, ()>,
}

/// Load both libraries, take the process-wide lock, and set both global hash
/// seeds to `seed`.
pub fn setup(seed: usize) -> Harness {
    let g = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let (c, r) = load_pair();
    unsafe {
        (c.rand_seed)(seed);
        (r.rand_seed)(seed);
    }
    Harness { c, r, _g: g }
}

// ---------------------------------------------------------------------------
// Deterministic RNG (SplitMix64) — fixed seed for reproducibility
// ---------------------------------------------------------------------------

pub struct Rng(pub u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng(seed)
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
    /// uniform-ish in `[0, n)`
    pub fn below(&mut self, n: usize) -> usize {
        if n == 0 {
            0
        } else {
            (self.next_u64() % (n as u64)) as usize
        }
    }
    pub fn byte(&mut self) -> u8 {
        (self.next_u64() >> 24) as u8
    }
    pub fn bytes(&mut self, n: usize) -> Vec<u8> {
        (0..n).map(|_| self.byte()).collect()
    }
}

// ---------------------------------------------------------------------------
// State description — everything observable, with no raw pointer values.
// ---------------------------------------------------------------------------

/// How the first 8 bytes of a map element should be rendered.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum KeyRepr {
    /// Raw bytes (binary keys).
    Bytes,
    /// A `char *`; render the pointed-to NUL-terminated string instead of the
    /// pointer value (which legitimately differs between the two libraries).
    CStr,
}

unsafe fn read_cstr(p: *const c_char) -> String {
    if p.is_null() {
        return "<null>".to_string();
    }
    let mut out = Vec::new();
    let mut i = 0usize;
    unsafe {
        loop {
            let b = *(p.add(i) as *const u8);
            if b == 0 {
                break;
            }
            out.push(b);
            i += 1;
            if i > 4096 {
                out.extend_from_slice(b"<trunc>");
                break;
            }
        }
    }
    format!("{:?}", out)
}

pub unsafe fn header_of(arr: *mut c_void) -> *mut ArrayHeader {
    unsafe { (arr as *mut ArrayHeader).offset(-1) }
}

/// Describe a plain dynamic array (as returned by `stbds_arrgrowf`).
pub unsafe fn describe_arr(arr: *mut c_void, elemsize: usize, dump_len: usize) -> String {
    if arr.is_null() {
        return "arr:<null>".to_string();
    }
    unsafe {
        let h = &*header_of(arr);
        let mut s = format!(
            "arr len={} cap={} temp={} ht={}",
            h.length,
            h.capacity,
            h.temp,
            if h.hash_table.is_null() { "null" } else { "set" }
        );
        let n = dump_len.min(h.length);
        if elemsize > 0 && n > 0 {
            let bytes = std::slice::from_raw_parts(arr as *const u8, n * elemsize);
            s.push_str(&format!(" data={:?}", bytes));
        }
        s
    }
}

/// Describe a hash map (the pointer the `hm*` functions return).
pub unsafe fn describe_map(hash_ptr: *mut c_void, elemsize: usize, key: KeyRepr) -> String {
    if hash_ptr.is_null() {
        return "map:<null>".to_string();
    }
    unsafe {
        let raw = (hash_ptr as *mut u8).sub(elemsize) as *mut c_void;
        let h = &*header_of(raw);
        let mut s = format!(
            "map len={} cap={} temp={} ht={}\n",
            h.length,
            h.capacity,
            h.temp,
            if h.hash_table.is_null() { "null" } else { "set" }
        );

        // Elements (index 0 is the hidden "default" slot).
        for i in 0..h.length {
            let base = (raw as *const u8).add(i.wrapping_mul(elemsize));
            match key {
                KeyRepr::Bytes => {
                    let bytes = std::slice::from_raw_parts(base, elemsize);
                    s.push_str(&format!("  e[{}]={:?}\n", i, bytes));
                }
                KeyRepr::CStr => {
                    let kp = *(base as *const *const c_char);
                    let rest = if elemsize > 8 {
                        format!("{:?}", std::slice::from_raw_parts(base.add(8), elemsize - 8))
                    } else {
                        "[]".to_string()
                    };
                    s.push_str(&format!("  e[{}]=key{} rest{}\n", i, read_cstr(kp), rest));
                }
            }
        }

        if !h.hash_table.is_null() {
            let t = &*(h.hash_table as *const HashIndex);
            s.push_str(&format!(
                "  tbl slots={} log2={} used={} used_thr={} shrink_thr={} tomb={} tomb_thr={} seed={:#x}\n",
                t.slot_count,
                t.slot_count_log2,
                t.used_count,
                t.used_count_threshold,
                t.used_count_shrink_threshold,
                t.tombstone_count,
                t.tombstone_count_threshold,
                t.seed
            ));
            s.push_str(&format!(
                "  arena remaining={} block={} mode={} storage={}\n",
                t.string.remaining,
                t.string.block,
                t.string.mode,
                if t.string.storage.is_null() { "null" } else { "set" }
            ));
            // NOTE: `stbds_hash_index::temp_key` is deliberately NOT rendered
            // here. `stbds_make_hash_index` never initialises it, and only the
            // string-mode branches of `stbds_hmput_key` ever write it, so in
            // every other state it is uninitialised realloc memory whose value
            // legitimately differs between the two libraries. It is compared
            // instead at the exact moment it is defined -- see
            // `MapPair::put` / `check_temp_key`.
            let nb = t.slot_count >> BUCKET_SHIFT;
            for b in 0..nb {
                let bk = &*t.storage.add(b);
                s.push_str(&format!("  b[{}] h={:?} i={:?}\n", b, bk.hash, bk.index));
            }
        }
        s
    }
}

/// Assert two descriptions are identical, printing a compact first-difference.
pub fn assert_same(label: &str, c: &str, r: &str) {
    if c != r {
        let cl: Vec<&str> = c.lines().collect();
        let rl: Vec<&str> = r.lines().collect();
        let mut diff = String::new();
        for i in 0..cl.len().max(rl.len()) {
            let a = cl.get(i).copied().unwrap_or("<eof>");
            let b = rl.get(i).copied().unwrap_or("<eof>");
            if a != b {
                diff.push_str(&format!("line {}:\n  C   : {}\n  RUST: {}\n", i, a, b));
                if diff.len() > 4000 {
                    break;
                }
            }
        }
        panic!("DIVERGENCE in {}:\n{}", label, diff);
    }
}

// ---------------------------------------------------------------------------
// Forked-child helper: run `f` in a child process and report how it died.
// Used for the crash/abort rows of ERRORS.md.
// ---------------------------------------------------------------------------

#[derive(Debug, PartialEq, Eq)]
pub enum Death {
    Exited(i32),
    Signaled(i32),
}

pub fn run_in_child<F: FnOnce()>(f: F) -> Death {
    unsafe {
        let pid = libc::fork();
        if pid == 0 {
            f();
            libc::_exit(0);
        }
        let mut status: c_int = 0;
        libc::waitpid(pid, &mut status, 0);
        if libc::WIFSIGNALED(status) {
            Death::Signaled(libc::WTERMSIG(status))
        } else {
            Death::Exited(libc::WEXITSTATUS(status))
        }
    }
}

// ---------------------------------------------------------------------------
// MapPair — drives the C and the Rust hash map in lock-step through the
// *low-level* exported entry points, reproducing exactly what the `hmput` /
// `shput` / `hmget` / `hmdel` macros in `lib.c` expand to.
// ---------------------------------------------------------------------------

pub struct MapPair<'a> {
    pub c: &'a Lib,
    pub r: &'a Lib,
    pub mc: *mut c_void,
    pub mr: *mut c_void,
    pub elemsize: usize,
    pub keysize: usize,
    /// element offset 0 holds a `char *` (any `string.mode != 0`)
    pub key_is_ptr: bool,
}

impl<'a> MapPair<'a> {
    pub fn new(c: &'a Lib, r: &'a Lib, elemsize: usize, keysize: usize, key_is_ptr: bool) -> Self {
        MapPair {
            c,
            r,
            mc: std::ptr::null_mut(),
            mr: std::ptr::null_mut(),
            elemsize,
            keysize,
            key_is_ptr,
        }
    }

    /// `stbds_sh_new_arena` / `stbds_sh_new_strdup` / any raw mode value.
    pub fn shmode(&mut self, mode: c_int) {
        self.mc = unsafe { (self.c.shmode_func)(self.elemsize, mode) };
        self.mr = unsafe { (self.r.shmode_func)(self.elemsize, mode) };
    }

    pub fn put_default(&mut self) {
        self.mc = unsafe { (self.c.hmput_default)(self.mc, self.elemsize) };
        self.mr = unsafe { (self.r.hmput_default)(self.mr, self.elemsize) };
    }

    pub unsafe fn raw(&self, m: *mut c_void) -> *mut c_void {
        unsafe { (m as *mut u8).sub(self.elemsize) as *mut c_void }
    }

    pub unsafe fn temp_c(&self) -> isize {
        unsafe { (*header_of(self.raw(self.mc))).temp }
    }
    pub unsafe fn temp_r(&self) -> isize {
        unsafe { (*header_of(self.raw(self.mr))).temp }
    }

    /// Deterministically initialise every byte of element `idx_raw` that the
    /// C macros would write, so that no *uninitialised* realloc byte ever
    /// enters a comparison.
    unsafe fn fill_element(&self, m: *mut c_void, idx_raw: usize, key: &[u8], tag: u8) {
        unsafe {
            let raw = self.raw(m) as *mut u8;
            let e = raw.add(idx_raw.wrapping_mul(self.elemsize));
            if self.key_is_ptr {
                // `shput` writes only `.value`; the `char *` at offset 0 is
                // owned by the library.
                let mut b = 8usize;
                while b < self.elemsize {
                    *e.add(b) = tag.wrapping_add(b as u8);
                    b += 1;
                }
            } else {
                // `hmput` rewrites `.key` and then `.value`.
                for (i, &kb) in key.iter().enumerate() {
                    if i < self.elemsize {
                        *e.add(i) = kb;
                    }
                }
                let mut b = self.keysize;
                while b < self.elemsize {
                    *e.add(b) = tag.wrapping_add(b as u8);
                    b += 1;
                }
            }
        }
    }

    /// `hmput` / `shput`: `t = hmput_key(...); t[temp(t-1)].key/.value = ...`
    ///
    /// Returns the `temp` index both libraries reported (asserted equal).
    pub fn put(&mut self, key: *mut c_void, key_bytes: &[u8], mode: c_int, tag: u8) -> isize {
        unsafe {
            // Poison `stbds_temp_key` in both libraries with the SAME pointer
            // first. `stbds_hmput_key` writes it only on a fresh insert and on
            // a duplicate hit in the *first* probe loop -- the wrap-around loop
            // deliberately does not. Poisoning makes "neither wrote it" compare
            // equal while still catching "one wrote it and the other did not".
            let tbl_c_before = self.table_ptr(self.mc);
            let tbl_r_before = self.table_ptr(self.mr);
            poison_temp_key(tbl_c_before);
            poison_temp_key(tbl_r_before);

            self.mc = (self.c.hmput_key)(self.mc, self.elemsize, key, self.keysize, mode);
            self.mr = (self.r.hmput_key)(self.mr, self.elemsize, key, self.keysize, mode);
            let tc = self.temp_c();
            let tr = self.temp_r();
            assert_eq!(tc, tr, "hmput_key temp mismatch (mode={})", mode);
            let grew = self.table_ptr(self.mc) != tbl_c_before
                || self.table_ptr(self.mr) != tbl_r_before;
            if mode >= HM_STRING && !grew {
                // A rebuilt table has an uninitialised temp_key, so only check
                // when `stbds_make_hash_index` did not run.
                self.check_temp_key(mode);
            }
            self.fill_element(self.mc, (tc + 1) as usize, key_bytes, tag);
            self.fill_element(self.mr, (tr + 1) as usize, key_bytes, tag);
            tc
        }
    }

    unsafe fn table_ptr(&self, m: *mut c_void) -> *mut HashIndex {
        unsafe {
            if m.is_null() {
                return std::ptr::null_mut();
            }
            (*header_of(self.raw(m))).hash_table as *mut HashIndex
        }
    }

    /// Compare `stbds_temp_key(t)` (== `hash_index::temp_key`) by *content*.
    /// The pointer value itself differs for STRDUP/ARENA modes.
    pub fn check_temp_key(&self, mode: c_int) {
        unsafe {
            let hc = &*header_of(self.raw(self.mc));
            let hr = &*header_of(self.raw(self.mr));
            assert!(!hc.hash_table.is_null() && !hr.hash_table.is_null());
            let tc = &*(hc.hash_table as *const HashIndex);
            let tr = &*(hr.hash_table as *const HashIndex);
            if !(1..=3).contains(&tc.string.mode) {
                // Only the SH_DEFAULT / SH_STRDUP / SH_ARENA arms of the
                // `switch` write temp_key; any other mode hits `default:`
                // (plain memcpy) and leaves temp_key untouched.
                return;
            }
            assert_eq!(
                read_cstr(tc.temp_key),
                read_cstr(tr.temp_key),
                "temp_key content mismatch (mode={}, string.mode={})",
                mode,
                tc.string.mode
            );
        }
    }

    /// `hmgeti` / `shgeti`
    pub fn get(&mut self, key: *mut c_void, mode: c_int) -> isize {
        unsafe {
            self.mc = (self.c.hmget_key)(self.mc, self.elemsize, key, self.keysize, mode);
            self.mr = (self.r.hmget_key)(self.mr, self.elemsize, key, self.keysize, mode);
            let tc = self.temp_c();
            let tr = self.temp_r();
            assert_eq!(tc, tr, "hmget_key temp mismatch (mode={})", mode);
            tc
        }
    }

    /// `hmgeti_ts` — the `temp` is returned out-of-band.
    pub fn get_ts(&mut self, key: *mut c_void, mode: c_int) -> isize {
        unsafe {
            let mut tc: isize = -999;
            let mut tr: isize = -999;
            self.mc =
                (self.c.hmget_key_ts)(self.mc, self.elemsize, key, self.keysize, &mut tc, mode);
            self.mr =
                (self.r.hmget_key_ts)(self.mr, self.elemsize, key, self.keysize, &mut tr, mode);
            assert_eq!(tc, tr, "hmget_key_ts out-param mismatch (mode={})", mode);
            tc
        }
    }

    /// `hmdel` / `shdel`: returns the `temp` flag (0 = not found, 1 = deleted),
    /// or `-1` when the function returned NULL.
    pub fn del(&mut self, key: *mut c_void, keyoffset: usize, mode: c_int) -> isize {
        unsafe {
            self.mc =
                (self.c.hmdel_key)(self.mc, self.elemsize, key, self.keysize, keyoffset, mode);
            self.mr =
                (self.r.hmdel_key)(self.mr, self.elemsize, key, self.keysize, keyoffset, mode);
            assert_eq!(
                self.mc.is_null(),
                self.mr.is_null(),
                "hmdel_key NULL-ness mismatch (mode={})",
                mode
            );
            if self.mc.is_null() {
                return -1;
            }
            let tc = self.temp_c();
            let tr = self.temp_r();
            assert_eq!(tc, tr, "hmdel_key temp mismatch (mode={})", mode);
            tc
        }
    }

    pub fn desc(&self) -> (String, String) {
        let repr = if self.key_is_ptr { KeyRepr::CStr } else { KeyRepr::Bytes };
        unsafe {
            (
                describe_map(self.mc, self.elemsize, repr),
                describe_map(self.mr, self.elemsize, repr),
            )
        }
    }

    /// Overwrite the array's *unused* capacity (bytes past `length`) with a
    /// fixed pattern in both libraries. `realloc` leaves that region
    /// uninitialised, so without this any C code that reads slightly past an
    /// element (e.g. `stbds_hmdel_key` with a large `keyoffset`) would compare
    /// garbage that legitimately differs between the two libraries.
    pub fn scrub_capacity(&self) {
        unsafe {
            for m in [self.mc, self.mr] {
                if m.is_null() || self.elemsize == 0 {
                    continue;
                }
                let raw = self.raw(m);
                let h = &*header_of(raw);
                if h.capacity > h.length {
                    std::ptr::write_bytes(
                        (raw as *mut u8).add(h.length * self.elemsize),
                        0xA5,
                        (h.capacity - h.length) * self.elemsize,
                    );
                }
            }
        }
    }

    pub fn check(&self, label: &str) {
        self.scrub_capacity();
        let (dc, dr) = self.desc();
        assert_same(label, &dc, &dr);
    }

    /// `hmfree` / `shfree`
    pub fn free(&mut self) {
        unsafe {
            if !self.mc.is_null() {
                (self.c.hmfree_func)(self.raw(self.mc), self.elemsize);
            }
            if !self.mr.is_null() {
                (self.r.hmfree_func)(self.raw(self.mr), self.elemsize);
            }
        }
        self.mc = std::ptr::null_mut();
        self.mr = std::ptr::null_mut();
    }
}

/// A shared, always-valid NUL-terminated sentinel written into both libraries'
/// `hash_index::temp_key` before a `put`, so that "the C did not write it" is
/// distinguishable from "the C wrote a different value".
static POISON: &[u8] = b"<temp_key-not-written>\0";

unsafe fn poison_temp_key(t: *mut HashIndex) {
    unsafe {
        if !t.is_null() {
            (*t).temp_key = POISON.as_ptr() as *mut c_char;
        }
    }
}

/// Owns the NUL-terminated key buffers handed to the libraries. For
/// `STBDS_SH_DEFAULT` the library stores the caller's pointer verbatim, so both
/// libraries must be given the *same* pointer.
pub struct Keys {
    pub bufs: Vec<Box<[u8]>>,
}

impl Keys {
    pub fn new() -> Keys {
        Keys { bufs: Vec::new() }
    }
    pub fn add_str(&mut self, s: &str) -> *mut c_void {
        let mut v: Vec<u8> = s.as_bytes().to_vec();
        v.push(0);
        self.bufs.push(v.into_boxed_slice());
        self.bufs.last_mut().unwrap().as_mut_ptr() as *mut c_void
    }
    pub fn add_bytes(&mut self, b: &[u8]) -> *mut c_void {
        self.bufs.push(b.to_vec().into_boxed_slice());
        self.bufs.last_mut().unwrap().as_mut_ptr() as *mut c_void
    }
}
