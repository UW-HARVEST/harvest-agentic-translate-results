//! Shared differential-test harness.
//!
//! BOTH implementations are loaded as shared objects through `libloading` and
//! driven *only* through their exported C ABI — the Rust crate is never linked
//! directly, so the `#[unsafe(no_mangle)] extern "C"` wrappers are under test
//! as well.
#![allow(dead_code)]
#![allow(clippy::missing_safety_doc)]

use libloading::Library;
use std::ffi::{c_char, c_int, c_void, CStr, CString};
use std::fmt::Write as _;
use std::path::{Path, PathBuf};

// ---------------------------------------------------------------------------
// libc bits the harness itself needs
// ---------------------------------------------------------------------------
unsafe extern "C" {
    fn fflush(f: *mut c_void) -> c_int;
    fn dup(fd: c_int) -> c_int;
    fn dup2(old: c_int, new: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn free(p: *mut c_void);
    fn malloc(n: usize) -> *mut c_void;
}

// ---------------------------------------------------------------------------
// Layout-identical mirrors of the C structs (for state inspection only)
// ---------------------------------------------------------------------------

pub const BUCKET_LENGTH: usize = 8;
pub const BUCKET_SHIFT: usize = 3;
pub const BUCKET_MASK: usize = 7;
pub const HEADER_SIZE: usize = 32;

pub const HM_BINARY: c_int = 0;
pub const HM_STRING: c_int = 1;

pub const SH_NONE: c_int = 0;
pub const SH_DEFAULT: c_int = 1;
pub const SH_STRDUP: c_int = 2;
pub const SH_ARENA: c_int = 3;

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct ArrHeader {
    pub length: usize,
    pub capacity: usize,
    pub hash_table: *mut c_void,
    pub temp: isize,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct Arena {
    pub storage: *mut c_void,
    pub remaining: usize,
    pub block: u8,
    pub mode: u8,
}

impl Arena {
    pub fn zeroed() -> Arena {
        Arena {
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
    pub string: Arena,
    pub storage: *mut HashBucket,
}

#[repr(C)]
pub struct StringBlock {
    pub next: *mut StringBlock,
    pub storage: [c_char; 8],
}

const _: () = {
    use std::mem::size_of;
    assert!(size_of::<ArrHeader>() == 32);
    assert!(size_of::<Arena>() == 24);
    assert!(size_of::<HashBucket>() == 128);
    assert!(size_of::<HashIndex>() == 104);
    assert!(size_of::<StringBlock>() == 16);
};

// ---------------------------------------------------------------------------
// The loaded library
// ---------------------------------------------------------------------------

pub type FnArrGrowf = unsafe extern "C" fn(*mut c_void, usize, usize, usize) -> *mut c_void;
pub type FnArrFreef = unsafe extern "C" fn(*mut c_void);
pub type FnRandSeed = unsafe extern "C" fn(usize);
pub type FnHashString = unsafe extern "C" fn(*mut c_char, usize) -> usize;
pub type FnHashBytes = unsafe extern "C" fn(*mut c_void, usize, usize) -> usize;
pub type FnHmFree = unsafe extern "C" fn(*mut c_void, usize);
pub type FnHmGetKeyTs =
    unsafe extern "C" fn(*mut c_void, usize, *mut c_void, usize, *mut isize, c_int) -> *mut c_void;
pub type FnHmGetKey =
    unsafe extern "C" fn(*mut c_void, usize, *mut c_void, usize, c_int) -> *mut c_void;
pub type FnHmPutDefault = unsafe extern "C" fn(*mut c_void, usize) -> *mut c_void;
pub type FnHmPutKey =
    unsafe extern "C" fn(*mut c_void, usize, *mut c_void, usize, c_int) -> *mut c_void;
pub type FnShModeFunc = unsafe extern "C" fn(usize, c_int) -> *mut c_void;
pub type FnHmDelKey =
    unsafe extern "C" fn(*mut c_void, usize, *mut c_void, usize, usize, c_int) -> *mut c_void;
pub type FnStrAlloc = unsafe extern "C" fn(*mut Arena, *mut c_char) -> *mut c_char;
pub type FnStrReset = unsafe extern "C" fn(*mut Arena);
pub type FnStrKey = unsafe extern "C" fn(c_int) -> *mut c_char;
pub type FnStrPut = unsafe extern "C" fn(c_int);

pub struct Lib {
    pub name: &'static str,
    pub arrgrowf: FnArrGrowf,
    pub arrfreef: FnArrFreef,
    pub rand_seed: FnRandSeed,
    pub hash_string: FnHashString,
    pub hash_bytes: FnHashBytes,
    pub hmfree_func: FnHmFree,
    pub hmget_key_ts: FnHmGetKeyTs,
    pub hmget_key: FnHmGetKey,
    pub hmput_default: FnHmPutDefault,
    pub hmput_key: FnHmPutKey,
    pub shmode_func: FnShModeFunc,
    pub hmdel_key: FnHmDelKey,
    pub stralloc: FnStrAlloc,
    pub strreset: FnStrReset,
    pub strkey: FnStrKey,
    pub str_put: FnStrPut,
}

macro_rules! sym {
    ($lib:expr, $t:ty, $n:literal) => {{
        let s: libloading::Symbol<'static, $t> = $lib
            .get($n)
            .unwrap_or_else(|e| panic!("missing symbol {}: {e}", stringify!($n)));
        *s
    }};
}

impl Lib {
    pub fn load(name: &'static str, path: &Path) -> Lib {
        unsafe {
            // Leaked on purpose: the returned fn pointers must stay valid for
            // the whole test binary's lifetime.
            let lib: &'static Library = Box::leak(Box::new(
                Library::new(path)
                    .unwrap_or_else(|e| panic!("cannot dlopen {}: {e}", path.display())),
            ));
            Lib {
                name,
                arrgrowf: sym!(lib, FnArrGrowf, b"stbds_arrgrowf\0"),
                arrfreef: sym!(lib, FnArrFreef, b"stbds_arrfreef\0"),
                rand_seed: sym!(lib, FnRandSeed, b"stbds_rand_seed\0"),
                hash_string: sym!(lib, FnHashString, b"stbds_hash_string\0"),
                hash_bytes: sym!(lib, FnHashBytes, b"stbds_hash_bytes\0"),
                hmfree_func: sym!(lib, FnHmFree, b"stbds_hmfree_func\0"),
                hmget_key_ts: sym!(lib, FnHmGetKeyTs, b"stbds_hmget_key_ts\0"),
                hmget_key: sym!(lib, FnHmGetKey, b"stbds_hmget_key\0"),
                hmput_default: sym!(lib, FnHmPutDefault, b"stbds_hmput_default\0"),
                hmput_key: sym!(lib, FnHmPutKey, b"stbds_hmput_key\0"),
                shmode_func: sym!(lib, FnShModeFunc, b"stbds_shmode_func\0"),
                hmdel_key: sym!(lib, FnHmDelKey, b"stbds_hmdel_key\0"),
                stralloc: sym!(lib, FnStrAlloc, b"stbds_stralloc\0"),
                strreset: sym!(lib, FnStrReset, b"stbds_strreset\0"),
                strkey: sym!(lib, FnStrKey, b"strkey\0"),
                str_put: sym!(lib, FnStrPut, b"str_put\0"),
            }
        }
    }
}

pub fn c_so_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("c_src/build/libtranslated_rust.so")
}

pub fn rust_so_path() -> PathBuf {
    // Allow a specific artifact to be selected (used by the crash-parity tests,
    // which must exercise the *release* cdylib: debug builds insert
    // `debug_assertions` instrumentation that turns a NULL / misaligned raw
    // dereference into a Rust panic + `abort()` (SIGABRT) instead of letting it
    // fault like the C code does (SIGSEGV)).
    if let Ok(p) = std::env::var("DIFF_RUST_SO") {
        return PathBuf::from(p);
    }
    // <target>/<profile>/deps/<testbin>  ->  <target>/<profile>/libstr_put_lib.so
    let exe = std::env::current_exe().expect("current_exe");
    let profile_dir = exe
        .parent()
        .and_then(|p| p.parent())
        .expect("profile dir")
        .to_path_buf();
    let direct = profile_dir.join("libstr_put_lib.so");
    if direct.exists() {
        return direct;
    }
    // Fallbacks, in case the test is run from an unusual layout.
    for p in [
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/debug/libstr_put_lib.so"),
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/release/libstr_put_lib.so"),
    ] {
        if p.exists() {
            return p;
        }
    }
    panic!(
        "libstr_put_lib.so not found (looked in {})",
        profile_dir.display()
    );
}

/// The release cdylib — the artifact whose behaviour must match the C `.so`
/// exactly, including how it crashes.
pub fn release_so_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/release/libstr_put_lib.so")
}

/// `cargo test` does **not** rebuild a `crate-type = ["cdylib"]` library, so a
/// stale `.so` would silently be tested. Refuse to run in that case.
fn assert_fresh(so: &Path) {
    let mtime = |p: &Path| {
        std::fs::metadata(p)
            .and_then(|m| m.modified())
            .unwrap_or_else(|e| panic!("stat {}: {e}", p.display()))
    };
    let so_t = mtime(so);
    let src = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/lib.rs");
    let src_t = mtime(&src);
    assert!(
        so_t >= src_t,
        "STALE ARTIFACT: {} is older than {}.\n\
         `cargo test` does not rebuild a cdylib — run `cargo build` (or \
         `./run_tests.sh`) first.",
        so.display(),
        src.display()
    );
}

/// The two libraries under comparison. Loaded once per test binary.
pub struct Pair {
    pub c: Lib,
    pub r: Lib,
}

pub fn libs() -> &'static Pair {
    use std::sync::OnceLock;
    static P: OnceLock<Pair> = OnceLock::new();
    P.get_or_init(|| {
        let rp = rust_so_path();
        assert_fresh(&rp);
        Pair {
            c: Lib::load("C", &c_so_path()),
            r: Lib::load("RUST", &rp),
        }
    })
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (xorshift64*) — no external dependency, fixed seeds
// ---------------------------------------------------------------------------

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng(if seed == 0 { 0x9E3779B97F4A7C15 } else { seed })
    }
    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545F4914F6CDD1D)
    }
    pub fn next_u32(&mut self) -> u32 {
        (self.next_u64() >> 32) as u32
    }
    /// uniform-ish in `0..n` (n > 0)
    pub fn below(&mut self, n: usize) -> usize {
        (self.next_u64() % (n as u64)) as usize
    }
    pub fn bytes(&mut self, n: usize) -> Vec<u8> {
        (0..n).map(|_| (self.next_u64() >> 24) as u8).collect()
    }
    /// random NUL-free byte string of length `n`
    pub fn cstr_bytes(&mut self, n: usize, high_bits: bool) -> Vec<u8> {
        (0..n)
            .map(|_| {
                let v = (self.next_u64() >> 24) as u8;
                let v = if high_bits { v } else { v & 0x7f };
                if v == 0 {
                    1
                } else {
                    v
                }
            })
            .collect()
    }
}

// ---------------------------------------------------------------------------
// Element description + state snapshotting
// ---------------------------------------------------------------------------

/// How to render an element's bytes so that the two libraries can be compared
/// (heap addresses legitimately differ, so `char*` fields are compared by the
/// string they point at).
#[derive(Clone, Debug)]
pub struct ElemDesc {
    pub elemsize: usize,
    /// byte ranges `(offset, len)` compared literally
    pub raw: Vec<(usize, usize)>,
    /// offsets of `char*` fields compared by pointee
    pub cstr: Vec<usize>,
}

impl ElemDesc {
    /// `struct { <keysize bytes key>; <rest value> }`, fully raw.
    pub fn all_raw(elemsize: usize) -> ElemDesc {
        ElemDesc {
            elemsize,
            raw: vec![(0, elemsize)],
            cstr: vec![],
        }
    }
    /// `struct { char *key; <rest> }`
    pub fn ptr_key(elemsize: usize) -> ElemDesc {
        ElemDesc {
            elemsize,
            raw: vec![(8, elemsize - 8)],
            cstr: vec![0],
        }
    }
}

unsafe fn render_cstr(p: *const c_char) -> String {
    unsafe {
        if p.is_null() {
            "NULL".to_string()
        } else {
            format!("{:?}", CStr::from_ptr(p).to_bytes())
        }
    }
}

pub unsafe fn header_of(t: *mut u8, elemsize: usize) -> *mut ArrHeader {
    unsafe { (t as *mut u8).sub(elemsize).cast::<u8>().sub(HEADER_SIZE) as *mut ArrHeader }
}

/// Snapshot of everything observable about a hash-map handle `t`
/// (`t` is the *hash* pointer, i.e. `raw_array + elemsize`).
pub unsafe fn snapshot_map(t: *mut u8, d: &ElemDesc) -> String {
    unsafe {
        let mut s = String::new();
        if t.is_null() {
            return "t=NULL".to_string();
        }
        let raw = t.sub(d.elemsize);
        let h = &*((raw as *mut u8).sub(HEADER_SIZE) as *mut ArrHeader);
        let _ = writeln!(
            s,
            "hdr len={} cap={} temp={} table={}",
            h.length,
            h.capacity,
            h.temp,
            if h.hash_table.is_null() { "no" } else { "yes" }
        );
        for i in 0..h.length {
            let e = raw.add(d.elemsize * i);
            let _ = write!(s, "  e[{i}]");
            for &(off, len) in &d.raw {
                let bytes = std::slice::from_raw_parts(e.add(off), len);
                let _ = write!(s, " raw@{off}={bytes:02x?}");
            }
            for &off in &d.cstr {
                let p = *(e.add(off) as *const *const c_char);
                let _ = write!(s, " str@{off}={}", render_cstr(p));
            }
            let _ = writeln!(s);
        }
        if !h.hash_table.is_null() {
            let ti = &*(h.hash_table as *const HashIndex);
            let _ = writeln!(
                s,
                "  tbl slots={} log2={} used={} used_thr={} shrink_thr={} tomb={} tomb_thr={} seed={:#x}",
                ti.slot_count,
                ti.slot_count_log2,
                ti.used_count,
                ti.used_count_threshold,
                ti.used_count_shrink_threshold,
                ti.tombstone_count,
                ti.tombstone_count_threshold,
                ti.seed
            );
            let _ = writeln!(
                s,
                "  arena remaining={} block={} mode={} storage={}",
                ti.string.remaining,
                ti.string.block,
                ti.string.mode,
                if ti.string.storage.is_null() {
                    "no"
                } else {
                    "yes"
                }
            );
            let nbuckets = ti.slot_count >> BUCKET_SHIFT;
            for b in 0..nbuckets {
                let bk = &*ti.storage.add(b);
                let _ = writeln!(s, "  b[{b}] h={:#x?} i={:?}", bk.hash, bk.index);
            }
        }
        s
    }
}

/// `stbds_hmlen(t)`
pub unsafe fn hmlen(t: *mut u8, elemsize: usize) -> isize {
    unsafe {
        if t.is_null() {
            0
        } else {
            (*header_of(t, elemsize)).length as isize - 1
        }
    }
}

/// `stbds_temp((t)-1)`
pub unsafe fn temp_of(t: *mut u8, elemsize: usize) -> isize {
    unsafe { (*header_of(t, elemsize)).temp }
}

/// `stbds_temp_key((t)-1)` — only meaningful right after a string-mode put.
pub unsafe fn temp_key_of(t: *mut u8, elemsize: usize) -> *mut c_char {
    unsafe {
        let ht = (*header_of(t, elemsize)).hash_table;
        if ht.is_null() {
            std::ptr::null_mut()
        } else {
            *(ht as *mut *mut c_char)
        }
    }
}

// ---------------------------------------------------------------------------
// A pair of maps kept in lock-step
// ---------------------------------------------------------------------------

/// Write the caller-owned "value" part of an element into both maps.
unsafe fn write_value(ce: *mut u8, re: *mut u8, off: usize, value: &[u8]) {
    unsafe {
        if value.is_empty() {
            return;
        }
        std::ptr::copy_nonoverlapping(value.as_ptr(), ce.add(off), value.len());
        std::ptr::copy_nonoverlapping(value.as_ptr(), re.add(off), value.len());
    }
}

pub struct MapPair {
    pub ct: *mut u8,
    pub rt: *mut u8,
    pub desc: ElemDesc,
    /// bytes of the element that the caller (i.e. the stb macro) writes
    pub value_off: usize,
    pub label: String,
    pub steps: usize,
}

impl MapPair {
    pub fn new(desc: ElemDesc, value_off: usize, label: &str) -> MapPair {
        MapPair {
            ct: std::ptr::null_mut(),
            rt: std::ptr::null_mut(),
            desc,
            value_off,
            label: label.to_string(),
            steps: 0,
        }
    }

    /// Reset both global hash seeds so the two libraries stay in lock-step.
    pub fn seed(&self, s: usize) {
        let l = libs();
        unsafe {
            (l.c.rand_seed)(s);
            (l.r.rand_seed)(s);
        }
    }

    /// `stbds_sh_new_arena` / `stbds_sh_new_strdup` (mode is the raw int).
    pub fn shmode(&mut self, mode: c_int) {
        let l = libs();
        unsafe {
            self.ct = (l.c.shmode_func)(self.desc.elemsize, mode) as *mut u8;
            self.rt = (l.r.shmode_func)(self.desc.elemsize, mode) as *mut u8;
        }
        self.check("shmode_func");
    }

    pub fn hmput_default_raw(&mut self) {
        let l = libs();
        unsafe {
            self.ct = (l.c.hmput_default)(self.ct as *mut c_void, self.desc.elemsize) as *mut u8;
            self.rt = (l.r.hmput_default)(self.rt as *mut c_void, self.desc.elemsize) as *mut u8;
        }
        self.check("hmput_default");
    }

    /// Emulates `stbds_hmput`: put the key, then have the *caller* write the
    /// key bytes and the value bytes into the returned slot (exactly what the
    /// macro does), so that every byte of every live element is defined.
    pub fn put_binary(&mut self, key: &[u8], value: &[u8], mode: c_int) -> (isize, isize) {
        let l = libs();
        let es = self.desc.elemsize;
        let mut k = key.to_vec();
        unsafe {
            self.ct = (l.c.hmput_key)(
                self.ct as *mut c_void,
                es,
                k.as_mut_ptr() as *mut c_void,
                key.len(),
                mode,
            ) as *mut u8;
            self.rt = (l.r.hmput_key)(
                self.rt as *mut c_void,
                es,
                k.as_mut_ptr() as *mut c_void,
                key.len(),
                mode,
            ) as *mut u8;
            let ci = temp_of(self.ct, es);
            let ri = temp_of(self.rt, es);
            assert_eq!(ci, ri, "{}: temp index mismatch after put_binary", self.label);
            // (t)[temp].key = k ; (t)[temp].value = v
            let ce = self.ct.add(es * ci as usize);
            let re = self.rt.add(es * ri as usize);
            if !key.is_empty() {
                std::ptr::copy_nonoverlapping(key.as_ptr(), ce, key.len());
                std::ptr::copy_nonoverlapping(key.as_ptr(), re, key.len());
            }
            write_value(ce, re, self.value_off, value);
            self.check("hmput_key(binary)");
            (ci, ri)
        }
    }

    /// Emulates `stbds_shput`: put a `char*` key, then the caller writes only
    /// the value (the library owns the key field).
    pub fn put_string(&mut self, key: *mut c_char, value: &[u8], mode: c_int) -> isize {
        let l = libs();
        let es = self.desc.elemsize;
        unsafe {
            self.ct = (l.c.hmput_key)(self.ct as *mut c_void, es, key as *mut c_void, 8, mode)
                as *mut u8;
            self.rt = (l.r.hmput_key)(self.rt as *mut c_void, es, key as *mut c_void, 8, mode)
                as *mut u8;
            let ci = temp_of(self.ct, es);
            let ri = temp_of(self.rt, es);
            assert_eq!(ci, ri, "{}: temp index mismatch after put_string", self.label);
            let ce = self.ct.add(es * ci as usize);
            let re = self.rt.add(es * ri as usize);
            write_value(ce, re, self.value_off, value);
            self.check("hmput_key(string)");
            ci
        }
    }

    /// `stbds_hmput_key` with `keysize` bytes copied verbatim (used for the
    /// "STRING mode on a NONE table" mixed configuration where the library
    /// memcpy's the string bytes itself).
    pub fn put_raw_keysize(
        &mut self,
        key: *mut c_char,
        keysize: usize,
        value: &[u8],
        mode: c_int,
    ) -> isize {
        let l = libs();
        let es = self.desc.elemsize;
        unsafe {
            self.ct =
                (l.c.hmput_key)(self.ct as *mut c_void, es, key as *mut c_void, keysize, mode)
                    as *mut u8;
            self.rt =
                (l.r.hmput_key)(self.rt as *mut c_void, es, key as *mut c_void, keysize, mode)
                    as *mut u8;
            let ci = temp_of(self.ct, es);
            let ri = temp_of(self.rt, es);
            assert_eq!(ci, ri, "{}: temp mismatch after put_raw_keysize", self.label);
            let ce = self.ct.add(es * ci as usize);
            let re = self.rt.add(es * ri as usize);
            write_value(ce, re, self.value_off, value);
            self.check("hmput_key(raw keysize)");
            ci
        }
    }

    pub fn get_ts(&mut self, key: *mut c_void, keysize: usize, mode: c_int) -> isize {
        let l = libs();
        let es = self.desc.elemsize;
        unsafe {
            let mut ctemp: isize = 0x5a5a;
            let mut rtemp: isize = 0x5a5a;
            self.ct = (l.c.hmget_key_ts)(
                self.ct as *mut c_void,
                es,
                key,
                keysize,
                &mut ctemp,
                mode,
            ) as *mut u8;
            self.rt = (l.r.hmget_key_ts)(
                self.rt as *mut c_void,
                es,
                key,
                keysize,
                &mut rtemp,
                mode,
            ) as *mut u8;
            assert_eq!(
                ctemp, rtemp,
                "{}: hmget_key_ts *temp mismatch (C={ctemp} RUST={rtemp})",
                self.label
            );
            self.check("hmget_key_ts");
            ctemp
        }
    }

    pub fn get(&mut self, key: *mut c_void, keysize: usize, mode: c_int) -> isize {
        let l = libs();
        let es = self.desc.elemsize;
        unsafe {
            self.ct =
                (l.c.hmget_key)(self.ct as *mut c_void, es, key, keysize, mode) as *mut u8;
            self.rt =
                (l.r.hmget_key)(self.rt as *mut c_void, es, key, keysize, mode) as *mut u8;
            let ci = temp_of(self.ct, es);
            let ri = temp_of(self.rt, es);
            assert_eq!(ci, ri, "{}: hmget_key temp mismatch", self.label);
            self.check("hmget_key");
            ci
        }
    }

    pub fn del(&mut self, key: *mut c_void, keysize: usize, keyoffset: usize, mode: c_int) -> isize {
        let l = libs();
        let es = self.desc.elemsize;
        unsafe {
            let cbefore = self.ct;
            let rbefore = self.rt;
            self.ct = (l.c.hmdel_key)(
                self.ct as *mut c_void,
                es,
                key,
                keysize,
                keyoffset,
                mode,
            ) as *mut u8;
            self.rt = (l.r.hmdel_key)(
                self.rt as *mut c_void,
                es,
                key,
                keysize,
                keyoffset,
                mode,
            ) as *mut u8;
            assert_eq!(
                self.ct.is_null(),
                self.rt.is_null(),
                "{}: hmdel_key NULL-ness mismatch",
                self.label
            );
            // `stbds_hmdel` returns `(t) ? stbds_temp((t)-1) : 0`
            let (ci, ri) = if self.ct.is_null() {
                (0, 0)
            } else {
                (temp_of(self.ct, es), temp_of(self.rt, es))
            };
            assert_eq!(ci, ri, "{}: hmdel_key temp mismatch", self.label);
            let _ = (cbefore, rbefore);
            self.check("hmdel_key");
            ci
        }
    }

    pub fn free(&mut self) {
        let l = libs();
        let es = self.desc.elemsize;
        unsafe {
            if !self.ct.is_null() {
                (l.c.hmfree_func)(self.ct.sub(es) as *mut c_void, es);
            }
            if !self.rt.is_null() {
                (l.r.hmfree_func)(self.rt.sub(es) as *mut c_void, es);
            }
        }
        self.ct = std::ptr::null_mut();
        self.rt = std::ptr::null_mut();
    }

    /// Compare `stbds_temp_key((t)-1)` between the two libraries by the string
    /// it points at (the heap addresses legitimately differ for SH_STRDUP /
    /// SH_ARENA). Only valid right after a string-mode put — a table rebuild
    /// (grow / shrink / rehash) leaves `temp_key` uninitialised in the C code.
    pub fn assert_temp_key_matches(&self, ctx: &str) {
        unsafe {
            let ck = temp_key_of(self.ct, self.desc.elemsize);
            let rk = temp_key_of(self.rt, self.desc.elemsize);
            assert_eq!(
                ck.is_null(),
                rk.is_null(),
                "{}: temp_key NULL-ness mismatch ({ctx})",
                self.label
            );
            if !ck.is_null() {
                let cs = CStr::from_ptr(ck).to_bytes().to_vec();
                let rs = CStr::from_ptr(rk).to_bytes().to_vec();
                assert_eq!(
                    cs, rs,
                    "{}: temp_key pointee mismatch ({ctx}) C={:?} RUST={:?}",
                    self.label,
                    String::from_utf8_lossy(&cs),
                    String::from_utf8_lossy(&rs)
                );
            }
        }
    }

    pub fn snapshots(&self) -> (String, String) {
        unsafe {
            (
                snapshot_map(self.ct, &self.desc),
                snapshot_map(self.rt, &self.desc),
            )
        }
    }

    pub fn check(&mut self, what: &str) {
        self.steps += 1;
        let (cs, rs) = self.snapshots();
        if cs != rs {
            panic!(
                "DIVERGENCE in {} after {} ops ({}):\n--- C ---\n{}\n--- RUST ---\n{}\n--- first differing line ---\n{}",
                self.label,
                self.steps,
                what,
                cs,
                rs,
                first_diff(&cs, &rs)
            );
        }
    }
}

pub fn first_diff(a: &str, b: &str) -> String {
    for (i, (la, lb)) in a.lines().zip(b.lines()).enumerate() {
        if la != lb {
            return format!("line {i}:\n  C   : {la}\n  RUST: {lb}");
        }
    }
    format!(
        "line counts differ: C={} RUST={}",
        a.lines().count(),
        b.lines().count()
    )
}

// ---------------------------------------------------------------------------
// stdout capture (str_put writes with libc printf into the shared stdout)
// ---------------------------------------------------------------------------

pub fn capture_stdout<F: FnOnce()>(f: F) -> Vec<u8> {
    use std::io::Read;
    let dir = std::env::temp_dir();
    let path = dir.join(format!(
        "diffcap-{}-{:?}.txt",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let file = std::fs::File::create(&path).expect("create capture file");
    let fd = {
        use std::os::fd::AsRawFd;
        file.as_raw_fd()
    };
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
    drop(file);
    let mut out = Vec::new();
    std::fs::File::open(&path)
        .expect("open capture file")
        .read_to_end(&mut out)
        .expect("read capture file");
    let _ = std::fs::remove_file(&path);
    out
}

// ---------------------------------------------------------------------------
// helpers
// ---------------------------------------------------------------------------

/// Heap-allocate a NUL-terminated copy of `bytes` with libc `malloc` and leak
/// it. Needed for `STBDS_SH_DEFAULT` maps, which store the caller's pointer.
pub fn leak_cstr(bytes: &[u8]) -> *mut c_char {
    unsafe {
        let p = malloc(bytes.len() + 1) as *mut u8;
        assert!(!p.is_null());
        std::ptr::copy_nonoverlapping(bytes.as_ptr(), p, bytes.len());
        *p.add(bytes.len()) = 0;
        p as *mut c_char
    }
}

pub fn free_raw(p: *mut c_char) {
    unsafe { free(p as *mut c_void) }
}

pub fn cs(s: &str) -> CString {
    CString::new(s).unwrap()
}
