//! Shared differential-test harness.
//!
//! Both the C `.so` and the Rust `.so` are loaded with `libloading` and driven
//! **only** through their exported `extern "C"` symbols, exactly as an external
//! consumer would. No Rust function is ever called directly.
#![allow(dead_code)]

use std::ffi::{c_char, c_int, c_void, CStr};
use std::path::PathBuf;
use std::sync::{Mutex, MutexGuard, OnceLock};

// ---------------------------------------------------------------------------
// C layout mirrors (must match c_src/src/lib.c exactly)
// ---------------------------------------------------------------------------

pub const HEADER_SIZE: usize = 32; // sizeof(stbds_array_header)
pub const BUCKET_LENGTH: usize = 8;
pub const BUCKET_SHIFT: usize = 3;
pub const BUCKET_MASK: usize = 7;

pub const HM_BINARY: c_int = 0;
pub const HM_STRING: c_int = 1;

pub const SH_NONE: c_int = 0;
pub const SH_DEFAULT: c_int = 1;
pub const SH_STRDUP: c_int = 2;
pub const SH_ARENA: c_int = 3;

pub const INITIAL_HASH_SEED: usize = 0x3141_5926;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct Header {
    pub length: usize,
    pub capacity: usize,
    pub hash_table: *mut c_void,
    pub temp: isize,
}

#[repr(C)]
#[derive(Clone, Copy)]
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
pub struct StringBlock {
    pub next: *mut StringBlock,
    pub storage: [c_char; 8],
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct Bucket {
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
    pub storage: *mut Bucket,
}

const _: () = assert!(std::mem::size_of::<Header>() == 32);
const _: () = assert!(std::mem::size_of::<Arena>() == 24);
const _: () = assert!(std::mem::size_of::<StringBlock>() == 16);
const _: () = assert!(std::mem::size_of::<Bucket>() == 128);
const _: () = assert!(std::mem::size_of::<HashIndex>() == 104);

// ---------------------------------------------------------------------------
// Function pointer types
// ---------------------------------------------------------------------------

pub type FnArrgrowf = unsafe extern "C" fn(*mut c_void, usize, usize, usize) -> *mut c_void;
pub type FnArrfreef = unsafe extern "C" fn(*mut c_void);
pub type FnRandSeed = unsafe extern "C" fn(usize);
pub type FnHashString = unsafe extern "C" fn(*mut c_char, usize) -> usize;
pub type FnHashBytes = unsafe extern "C" fn(*mut c_void, usize, usize) -> usize;
pub type FnHmfreeFunc = unsafe extern "C" fn(*mut c_void, usize);
pub type FnHmgetKey =
    unsafe extern "C" fn(*mut c_void, usize, *mut c_void, usize, c_int) -> *mut c_void;
pub type FnHmgetKeyTs = unsafe extern "C" fn(
    *mut c_void,
    usize,
    *mut c_void,
    usize,
    *mut isize,
    c_int,
) -> *mut c_void;
pub type FnHmputDefault = unsafe extern "C" fn(*mut c_void, usize) -> *mut c_void;
pub type FnHmputKey =
    unsafe extern "C" fn(*mut c_void, usize, *mut c_void, usize, c_int) -> *mut c_void;
pub type FnShmodeFunc = unsafe extern "C" fn(usize, c_int) -> *mut c_void;
pub type FnHmdelKey =
    unsafe extern "C" fn(*mut c_void, usize, *mut c_void, usize, usize, c_int) -> *mut c_void;
pub type FnStralloc = unsafe extern "C" fn(*mut Arena, *mut c_char) -> *mut c_char;
pub type FnStrreset = unsafe extern "C" fn(*mut Arena);
pub type FnStrkey = unsafe extern "C" fn(c_int) -> *mut c_char;
pub type FnArrIns = unsafe extern "C" fn(c_int);

/// One loaded shared object with all 16 exported symbols resolved.
pub struct Lib {
    pub tag: &'static str,
    pub path: String,
    pub arrgrowf: FnArrgrowf,
    pub arrfreef: FnArrfreef,
    pub rand_seed: FnRandSeed,
    pub hash_string: FnHashString,
    pub hash_bytes: FnHashBytes,
    pub hmfree_func: FnHmfreeFunc,
    pub hmget_key: FnHmgetKey,
    pub hmget_key_ts: FnHmgetKeyTs,
    pub hmput_default: FnHmputDefault,
    pub hmput_key: FnHmputKey,
    pub shmode_func: FnShmodeFunc,
    pub hmdel_key: FnHmdelKey,
    pub stralloc: FnStralloc,
    pub strreset: FnStrreset,
    pub strkey: FnStrkey,
    pub arr_ins: FnArrIns,
}

unsafe impl Send for Lib {}
unsafe impl Sync for Lib {}

pub const ALL_SYMBOLS: [&str; 16] = [
    "stbds_arrgrowf",
    "stbds_arrfreef",
    "stbds_rand_seed",
    "stbds_hash_string",
    "stbds_hash_bytes",
    "stbds_hmfree_func",
    "stbds_hmget_key",
    "stbds_hmget_key_ts",
    "stbds_hmput_default",
    "stbds_hmput_key",
    "stbds_shmode_func",
    "stbds_hmdel_key",
    "stbds_stralloc",
    "stbds_strreset",
    "strkey",
    "arr_ins",
];

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

pub fn c_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("C_SO") {
        return PathBuf::from(p);
    }
    manifest_dir().join("c_src/build/libtranslated_rust.so")
}

pub fn rust_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("RUST_SO") {
        return PathBuf::from(p);
    }
    let rel = manifest_dir().join("target/release/libarr_ins_lib.so");
    if rel.exists() {
        return rel;
    }
    manifest_dir().join("target/debug/libarr_ins_lib.so")
}

fn load(path: &PathBuf, tag: &'static str) -> Lib {
    let lib: &'static libloading::Library = Box::leak(Box::new(unsafe {
        libloading::Library::new(path)
            .unwrap_or_else(|e| panic!("failed to dlopen {} ({}): {}", path.display(), tag, e))
    }));

    macro_rules! sym {
        ($name:literal, $ty:ty) => {{
            let s: libloading::Symbol<'static, $ty> = unsafe {
                lib.get(concat!($name, "\0").as_bytes()).unwrap_or_else(|e| {
                    panic!("missing symbol {} in {} ({}): {}", $name, path.display(), tag, e)
                })
            };
            *s
        }};
    }

    Lib {
        tag,
        path: path.display().to_string(),
        arrgrowf: sym!("stbds_arrgrowf", FnArrgrowf),
        arrfreef: sym!("stbds_arrfreef", FnArrfreef),
        rand_seed: sym!("stbds_rand_seed", FnRandSeed),
        hash_string: sym!("stbds_hash_string", FnHashString),
        hash_bytes: sym!("stbds_hash_bytes", FnHashBytes),
        hmfree_func: sym!("stbds_hmfree_func", FnHmfreeFunc),
        hmget_key: sym!("stbds_hmget_key", FnHmgetKey),
        hmget_key_ts: sym!("stbds_hmget_key_ts", FnHmgetKeyTs),
        hmput_default: sym!("stbds_hmput_default", FnHmputDefault),
        hmput_key: sym!("stbds_hmput_key", FnHmputKey),
        shmode_func: sym!("stbds_shmode_func", FnShmodeFunc),
        hmdel_key: sym!("stbds_hmdel_key", FnHmdelKey),
        stralloc: sym!("stbds_stralloc", FnStralloc),
        strreset: sym!("stbds_strreset", FnStrreset),
        strkey: sym!("strkey", FnStrkey),
        arr_ins: sym!("arr_ins", FnArrIns),
    }
}

pub struct Pair {
    pub c: Lib,
    pub rust: Lib,
}

static PAIR: OnceLock<Pair> = OnceLock::new();
static LOCK: Mutex<()> = Mutex::new(());

pub fn pair() -> &'static Pair {
    PAIR.get_or_init(|| Pair {
        c: load(&c_so_path(), "C"),
        rust: load(&rust_so_path(), "RUST"),
    })
}

/// Both libraries own *private* mutable globals (`stbds_hash_seed`, `buffer`).
/// Every test therefore serialises against every other test in the same
/// process and resets the seed so the two libraries stay in lockstep.
pub fn session() -> Session {
    let guard = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let p = pair();
    unsafe {
        (p.c.rand_seed)(INITIAL_HASH_SEED);
        (p.rust.rand_seed)(INITIAL_HASH_SEED);
    }
    Session {
        _guard: guard,
        c: &p.c,
        rust: &p.rust,
    }
}

pub struct Session {
    _guard: MutexGuard<'static, ()>,
    pub c: &'static Lib,
    pub rust: &'static Lib,
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (SplitMix64) — fixed seed for reproducibility
// ---------------------------------------------------------------------------

pub const TEST_SEED: u64 = 0x9E37_79B9_7F4A_7C15;

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
    pub fn next_usize(&mut self) -> usize {
        self.next_u64() as usize
    }
    /// Uniform-ish in `[0, n)`
    pub fn below(&mut self, n: usize) -> usize {
        if n == 0 {
            0
        } else {
            (self.next_u64() % (n as u64)) as usize
        }
    }
    pub fn range(&mut self, lo: usize, hi_inclusive: usize) -> usize {
        lo + self.below(hi_inclusive - lo + 1)
    }
    pub fn bytes(&mut self, n: usize) -> Vec<u8> {
        (0..n).map(|_| (self.next_u64() >> 24) as u8).collect()
    }
    /// Random NUL-free ASCII-ish string, returned as a NUL-terminated buffer.
    pub fn cstring(&mut self, len: usize) -> Vec<u8> {
        let mut v: Vec<u8> = (0..len)
            .map(|_| {
                let c = (self.next_u64() % 94) as u8 + 33; // '!'..'~'
                c
            })
            .collect();
        v.push(0);
        v
    }
    /// Random NUL-free string over the full 1..=255 byte range (exercises the
    /// signed-`char` sign-extension trap in `stbds_hash_string`).
    pub fn cstring_full(&mut self, len: usize) -> Vec<u8> {
        let mut v: Vec<u8> = (0..len)
            .map(|_| ((self.next_u64() % 255) as u8).wrapping_add(1))
            .collect();
        v.push(0);
        v
    }
}

// ---------------------------------------------------------------------------
// Snapshot / serialisation of library-internal state
// ---------------------------------------------------------------------------

/// Which key representation the element payload uses.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum KeyRepr {
    /// keys are raw bytes inside the element (binary maps, `SH_NONE` tables)
    Raw,
    /// element starts with a `char *` pointing at the key text
    StrPtr,
}

#[derive(Clone, Copy)]
pub struct DumpOpts {
    pub elemsize: usize,
    pub key: KeyRepr,
    /// Compare `hash_index.temp_key`? Only meaningful right after an insert
    /// into a string-mode table; `stbds_make_hash_index` leaves the field
    /// uninitialised otherwise.
    pub check_temp_key: bool,
    /// Compare the element payload bytes? (false when the payload contains
    /// addresses that legitimately differ between the two libraries)
    pub check_elements: bool,
    /// Compare the *raw pointer values* of `temp_key` / element keys instead of
    /// only their contents. Valid for `STBDS_SH_DEFAULT` tables, where the
    /// stored pointer is the caller's own buffer and therefore must be
    /// bit-identical in both libraries.
    pub ptr_identity: bool,
}

impl DumpOpts {
    pub fn raw(elemsize: usize) -> DumpOpts {
        DumpOpts {
            elemsize,
            key: KeyRepr::Raw,
            check_temp_key: false,
            check_elements: true,
            ptr_identity: false,
        }
    }
    pub fn strptr(elemsize: usize) -> DumpOpts {
        DumpOpts {
            elemsize,
            key: KeyRepr::StrPtr,
            check_temp_key: false,
            check_elements: true,
            ptr_identity: false,
        }
    }
    pub fn with_temp_key(mut self) -> DumpOpts {
        self.check_temp_key = true;
        self
    }
    pub fn without_elements(mut self) -> DumpOpts {
        self.check_elements = false;
        self
    }
    pub fn with_ptr_identity(mut self) -> DumpOpts {
        self.ptr_identity = true;
        self
    }
}

pub unsafe fn cstr_repr(p: *const c_char) -> String {
    if p.is_null() {
        return "<null>".to_string();
    }
    let bytes = CStr::from_ptr(p).to_bytes();
    let mut s = String::from("\"");
    for &b in bytes {
        if b.is_ascii_graphic() || b == b' ' {
            s.push(b as char);
        } else {
            s.push_str(&format!("\\x{:02x}", b));
        }
    }
    s.push('"');
    s
}

pub fn hex(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2);
    for &b in bytes {
        s.push_str(&format!("{:02x}", b));
    }
    s
}

/// Serialise the internal state reachable from a *raw array* pointer (what
/// `stbds_arrgrowf` returns). `len` elements of the payload are dumped.
pub unsafe fn dump_array(a: *mut c_void, elemsize: usize, len: usize) -> String {
    if a.is_null() {
        return "ARRAY:NULL".to_string();
    }
    let h = *(((a as *mut u8).sub(HEADER_SIZE)) as *mut Header);
    let mut s = format!(
        "ARRAY len={} cap={} temp={} ht={}",
        h.length,
        h.capacity,
        h.temp,
        if h.hash_table.is_null() { "null" } else { "set" }
    );
    for i in 0..len {
        let e = (a as *mut u8).add(elemsize * i);
        s.push_str(&format!(
            " e{}={}",
            i,
            hex(std::slice::from_raw_parts(e, elemsize))
        ));
    }
    s
}

/// Serialise the whole internal state reachable from a *hash* pointer (what
/// the `stbds_hm*` functions return: `raw_array + elemsize`).
pub unsafe fn dump_map(hp: *mut c_void, o: DumpOpts) -> String {
    if hp.is_null() {
        return "MAP:NULL".to_string();
    }
    let arr = (hp as *mut u8).sub(o.elemsize);
    let h = *((arr.sub(HEADER_SIZE)) as *mut Header);
    let mut s = format!("MAP len={} cap={} temp={}", h.length, h.capacity, h.temp);

    if h.hash_table.is_null() {
        s.push_str(" idx=none");
    } else {
        let t = *(h.hash_table as *mut HashIndex);
        s.push_str(&format!(
            " idx{{slots={} used={} uct={} ucst={} tomb={} tct={} seed={:#018x} log2={}}}",
            t.slot_count,
            t.used_count,
            t.used_count_threshold,
            t.used_count_shrink_threshold,
            t.tombstone_count,
            t.tombstone_count_threshold,
            t.seed,
            t.slot_count_log2
        ));
        s.push_str(&format!(
            " arena{{rem={} block={} mode={} storage={}}}",
            t.string.remaining,
            t.string.block,
            t.string.mode,
            if t.string.storage.is_null() {
                "null"
            } else {
                "set"
            }
        ));
        if o.check_temp_key {
            s.push_str(&format!(" temp_key={}", cstr_repr(t.temp_key)));
            if o.ptr_identity {
                s.push_str(&format!("@{:p}", t.temp_key));
            }
        }
        for slot in 0..t.slot_count {
            let b = &*t.storage.add(slot >> BUCKET_SHIFT);
            let k = slot & BUCKET_MASK;
            s.push_str(&format!(" s{}={:#018x}/{}", slot, b.hash[k], b.index[k]));
        }
    }

    if o.check_elements {
        for i in 0..h.length {
            let e = arr.add(o.elemsize * i);
            match o.key {
                KeyRepr::Raw => {
                    s.push_str(&format!(
                        " e{}={}",
                        i,
                        hex(std::slice::from_raw_parts(e, o.elemsize))
                    ));
                }
                KeyRepr::StrPtr => {
                    let kp = *(e as *mut *mut c_char);
                    s.push_str(&format!(" e{}k={}", i, cstr_repr(kp)));
                    if o.ptr_identity {
                        s.push_str(&format!("@{:p}", kp));
                    }
                    if o.elemsize > 8 {
                        s.push_str(&format!(
                            " e{}v={}",
                            i,
                            hex(std::slice::from_raw_parts(e.add(8), o.elemsize - 8))
                        ));
                    }
                }
            }
        }
    }
    s
}

/// Serialise a `stbds_string_arena` plus the whole block chain it owns.
/// `probe` is a list of (pointer, expected-text) previously returned by
/// `stbds_stralloc` — only the *contents* are compared, never the addresses.
pub unsafe fn dump_arena(a: *const Arena) -> String {
    let ar = *a;
    let mut s = format!(
        "ARENA rem={} block={} mode={} storage={}",
        ar.remaining,
        ar.block,
        ar.mode,
        if ar.storage.is_null() { "null" } else { "set" }
    );
    let mut n = 0usize;
    let mut x = ar.storage as *mut StringBlock;
    while !x.is_null() {
        n += 1;
        if n > 100_000 {
            s.push_str(" <cycle?>");
            break;
        }
        x = (*x).next;
    }
    s.push_str(&format!(" blocks={}", n));
    s
}

// ---------------------------------------------------------------------------
// High-level map driver (mirrors the stb_ds macros)
// ---------------------------------------------------------------------------

/// Element layout of a map under test.
#[derive(Clone, Copy, Debug)]
pub struct Layout {
    pub name: &'static str,
    pub elemsize: usize,
    pub keysize: usize,
}

pub const L_I2I: Layout = Layout {
    name: "I2I",
    elemsize: 8,
    keysize: 4,
};
pub const L_S1: Layout = Layout {
    name: "S1",
    elemsize: 16,
    keysize: 4,
};
pub const L_S2: Layout = Layout {
    name: "S2",
    elemsize: 16,
    keysize: 8,
};
pub const L_U2U: Layout = Layout {
    name: "U2U",
    elemsize: 16,
    keysize: 8,
};
pub const L_B1: Layout = Layout {
    name: "B1",
    elemsize: 8,
    keysize: 1,
};
pub const L_BIG: Layout = Layout {
    name: "BIG",
    elemsize: 128,
    keysize: 4,
};
pub const L_ODD: Layout = Layout {
    name: "ODD",
    elemsize: 12,
    keysize: 3,
};
pub const L_STR: Layout = Layout {
    name: "STR",
    elemsize: 16,
    keysize: 8,
};
pub const L_STRB: Layout = Layout {
    name: "STRB",
    elemsize: 32,
    keysize: 8,
};

pub const BINARY_LAYOUTS: [Layout; 7] = [L_I2I, L_S1, L_S2, L_U2U, L_B1, L_BIG, L_ODD];

/// `stbds_hmput(t, k, v)` for a binary-key map: call `stbds_hmput_key` and then
/// fill the whole element (key **and** value) exactly like the macro does, so
/// no byte of the element is ever left uninitialised.
pub unsafe fn map_put_binary(
    lib: &Lib,
    hp: *mut c_void,
    lay: Layout,
    key: &[u8],
    value: &[u8],
    mode: c_int,
) -> *mut c_void {
    assert_eq!(key.len(), lay.keysize);
    assert_eq!(value.len(), lay.elemsize - lay.keysize);
    let mut k = key.to_vec();
    let hp2 = (lib.hmput_key)(
        hp,
        lay.elemsize,
        k.as_mut_ptr() as *mut c_void,
        lay.keysize,
        mode,
    );
    let arr = (hp2 as *mut u8).sub(lay.elemsize);
    let idx = (*((arr.sub(HEADER_SIZE)) as *mut Header)).temp;
    let elem = arr.add(lay.elemsize * ((idx + 1) as usize));
    std::ptr::copy_nonoverlapping(key.as_ptr(), elem, lay.keysize);
    std::ptr::copy_nonoverlapping(value.as_ptr(), elem.add(lay.keysize), value.len());
    hp2
}

/// `stbds_shput(t, k, v)`: `stbds_hmput_key` already stored the key pointer, so
/// only the value part of the element is written by the macro.
pub unsafe fn map_put_string(
    lib: &Lib,
    hp: *mut c_void,
    lay: Layout,
    key: *mut c_char,
    value: &[u8],
    mode: c_int,
) -> *mut c_void {
    assert_eq!(value.len(), lay.elemsize - 8);
    let hp2 = (lib.hmput_key)(hp, lay.elemsize, key as *mut c_void, lay.keysize, mode);
    let arr = (hp2 as *mut u8).sub(lay.elemsize);
    let idx = (*((arr.sub(HEADER_SIZE)) as *mut Header)).temp;
    let elem = arr.add(lay.elemsize * ((idx + 1) as usize));
    if !value.is_empty() {
        std::ptr::copy_nonoverlapping(value.as_ptr(), elem.add(8), value.len());
    }
    hp2
}

/// `stbds_hmgeti(t, k)` → (new pointer, index)
pub unsafe fn map_geti(
    lib: &Lib,
    hp: *mut c_void,
    lay: Layout,
    key_ptr: *mut c_void,
    mode: c_int,
) -> (*mut c_void, isize) {
    let hp2 = (lib.hmget_key)(hp, lay.elemsize, key_ptr, lay.keysize, mode);
    let arr = (hp2 as *mut u8).sub(lay.elemsize);
    let idx = (*((arr.sub(HEADER_SIZE)) as *mut Header)).temp;
    (hp2, idx)
}

/// `stbds_hmgeti_ts(t, k, temp)` → (new pointer, temp, header temp)
pub unsafe fn map_geti_ts(
    lib: &Lib,
    hp: *mut c_void,
    lay: Layout,
    key_ptr: *mut c_void,
    mode: c_int,
) -> (*mut c_void, isize, isize) {
    let mut temp: isize = 0x5A5A_5A5A;
    let hp2 = (lib.hmget_key_ts)(
        hp,
        lay.elemsize,
        key_ptr,
        lay.keysize,
        &mut temp,
        mode,
    );
    let arr = (hp2 as *mut u8).sub(lay.elemsize);
    let hdr_temp = (*((arr.sub(HEADER_SIZE)) as *mut Header)).temp;
    (hp2, temp, hdr_temp)
}

/// `stbds_hmdel(t, k)` → (new pointer, temp)
pub unsafe fn map_del(
    lib: &Lib,
    hp: *mut c_void,
    lay: Layout,
    key_ptr: *mut c_void,
    keyoffset: usize,
    mode: c_int,
) -> (*mut c_void, isize) {
    let hp2 = (lib.hmdel_key)(
        hp,
        lay.elemsize,
        key_ptr,
        lay.keysize,
        keyoffset,
        mode,
    );
    if hp2.is_null() {
        return (hp2, 0);
    }
    let arr = (hp2 as *mut u8).sub(lay.elemsize);
    let temp = (*((arr.sub(HEADER_SIZE)) as *mut Header)).temp;
    (hp2, temp)
}

pub unsafe fn map_free(lib: &Lib, hp: *mut c_void, lay: Layout) {
    if !hp.is_null() {
        (lib.hmfree_func)((hp as *mut u8).sub(lay.elemsize) as *mut c_void, lay.elemsize);
    }
}

// ---------------------------------------------------------------------------
// Assertion helper
// ---------------------------------------------------------------------------

#[track_caller]
pub fn assert_same(what: &str, c: &str, rust: &str) {
    if c != rust {
        let mut i = 0;
        let cb = c.as_bytes();
        let rb = rust.as_bytes();
        while i < cb.len().min(rb.len()) && cb[i] == rb[i] {
            i += 1;
        }
        let lo = i.saturating_sub(60);
        panic!(
            "DIVERGENCE in {}\n  first difference at byte {}\n  C    ...{}\n  RUST ...{}\n\n  full C   : {}\n  full RUST: {}",
            what,
            i,
            &c[lo..(i + 80).min(c.len())],
            &rust[lo..(i + 80).min(rust.len())],
            c,
            rust
        );
    }
}
