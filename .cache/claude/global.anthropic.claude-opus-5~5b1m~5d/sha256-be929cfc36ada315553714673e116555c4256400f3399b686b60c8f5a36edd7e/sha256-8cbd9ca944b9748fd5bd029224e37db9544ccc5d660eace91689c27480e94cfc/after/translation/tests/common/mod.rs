//! Shared differential-test harness.
//!
//! BOTH implementations are loaded as shared objects through `libloading`; the
//! Rust functions are never called directly, so the `#[no_mangle]` export
//! wrappers are under test too.

#![allow(dead_code)]

pub use std::ffi::{c_char, c_int, c_void, CString};
use std::path::PathBuf;

pub mod driver;

pub type ArrGrowF = unsafe extern "C" fn(*mut c_void, usize, usize, usize) -> *mut c_void;
pub type ArrFreeF = unsafe extern "C" fn(*mut c_void);
pub type RandSeed = unsafe extern "C" fn(usize);
pub type HashString = unsafe extern "C" fn(*mut c_char, usize) -> usize;
pub type HashBytes = unsafe extern "C" fn(*mut c_void, usize, usize) -> usize;
pub type HmFreeFunc = unsafe extern "C" fn(*mut c_void, usize);
pub type HmGetKeyTs =
    unsafe extern "C" fn(*mut c_void, usize, *mut c_void, usize, *mut isize, c_int) -> *mut c_void;
pub type HmGetKey =
    unsafe extern "C" fn(*mut c_void, usize, *mut c_void, usize, c_int) -> *mut c_void;
pub type HmPutDefault = unsafe extern "C" fn(*mut c_void, usize) -> *mut c_void;
pub type HmPutKey =
    unsafe extern "C" fn(*mut c_void, usize, *mut c_void, usize, c_int) -> *mut c_void;
pub type ShModeFunc = unsafe extern "C" fn(usize, c_int) -> *mut c_void;
pub type HmDelKey =
    unsafe extern "C" fn(*mut c_void, usize, *mut c_void, usize, usize, c_int) -> *mut c_void;
pub type StrAlloc = unsafe extern "C" fn(*mut c_void, *mut c_char) -> *mut c_char;
pub type StrReset = unsafe extern "C" fn(*mut c_void);
pub type StrKey = unsafe extern "C" fn(c_int) -> *mut c_char;
pub type Intput = unsafe extern "C" fn(c_int);

// ---------------------------------------------------------------------------
// layout-compatible mirrors of the (private) C structs, for inspecting state
// ---------------------------------------------------------------------------

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct ArrayHeader {
    pub length: usize,
    pub capacity: usize,
    pub hash_table: *mut c_void,
    pub temp: isize,
}

pub const HDR_SIZE: usize = std::mem::size_of::<ArrayHeader>();

#[repr(C)]
#[derive(Clone, Copy)]
pub struct StringArena {
    pub storage: *mut c_void,
    pub remaining: usize,
    pub block: u8,
    pub mode: u8,
}

pub const ARENA_SIZE: usize = std::mem::size_of::<StringArena>();

#[repr(C)]
#[derive(Clone, Copy)]
pub struct HashBucket {
    pub hash: [usize; 8],
    pub index: [isize; 8],
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
// library handle
// ---------------------------------------------------------------------------

pub struct Lib {
    pub name: &'static str,
    _lib: libloading::Library,
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
    pub shmode_func: ShModeFunc,
    pub hmdel_key: HmDelKey,
    pub stralloc: StrAlloc,
    pub strreset: StrReset,
    pub strkey: StrKey,
    pub intput: Intput,
}

fn repo_root() -> PathBuf {
    // CARGO_MANIFEST_DIR == <root>/translation
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf()
}

pub fn c_so_path() -> PathBuf {
    let dir = repo_root().join("c_src/build");
    let entries = std::fs::read_dir(&dir).unwrap_or_else(|e| {
        panic!(
            "cannot read {}: {e}. Build the C library first:\n  cd c_src && mkdir -p build && \
             cd build && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
            dir.display()
        )
    });
    let mut found = None;
    for e in entries.flatten() {
        let p = e.path();
        if p.extension().map(|x| x == "so").unwrap_or(false) {
            found = Some(p);
        }
    }
    found.unwrap_or_else(|| panic!("no .so found in {}", dir.display()))
}

pub fn rust_so_path() -> PathBuf {
    let p = repo_root().join("translation/target/release/libintput_lib.so");
    assert!(
        p.exists(),
        "{} missing. Build it first:  cd translation && cargo build --release",
        p.display()
    );
    p
}

macro_rules! sym {
    ($lib:expr, $name:literal, $ty:ty) => {{
        let s: libloading::Symbol<$ty> = $lib.get(concat!($name, "\0").as_bytes()).unwrap();
        *s
    }};
}

impl Lib {
    pub unsafe fn open(name: &'static str, path: &std::path::Path) -> Lib {
        let lib = libloading::Library::new(path).unwrap();
        let l = Lib {
            name,
            arrgrowf: sym!(lib, "stbds_arrgrowf", ArrGrowF),
            arrfreef: sym!(lib, "stbds_arrfreef", ArrFreeF),
            rand_seed: sym!(lib, "stbds_rand_seed", RandSeed),
            hash_string: sym!(lib, "stbds_hash_string", HashString),
            hash_bytes: sym!(lib, "stbds_hash_bytes", HashBytes),
            hmfree_func: sym!(lib, "stbds_hmfree_func", HmFreeFunc),
            hmget_key_ts: sym!(lib, "stbds_hmget_key_ts", HmGetKeyTs),
            hmget_key: sym!(lib, "stbds_hmget_key", HmGetKey),
            hmput_default: sym!(lib, "stbds_hmput_default", HmPutDefault),
            hmput_key: sym!(lib, "stbds_hmput_key", HmPutKey),
            shmode_func: sym!(lib, "stbds_shmode_func", ShModeFunc),
            hmdel_key: sym!(lib, "stbds_hmdel_key", HmDelKey),
            stralloc: sym!(lib, "stbds_stralloc", StrAlloc),
            strreset: sym!(lib, "stbds_strreset", StrReset),
            strkey: sym!(lib, "strkey", StrKey),
            intput: sym!(lib, "intput", Intput),
            _lib: lib,
        };
        l
    }
}

/// The pair of libraries under comparison: `(C, Rust)`.
pub fn both() -> (Lib, Lib) {
    unsafe {
        (
            Lib::open("C", &c_so_path()),
            Lib::open("Rust", &rust_so_path()),
        )
    }
}

// ---------------------------------------------------------------------------
// deterministic RNG (xorshift64*, fixed seed) — no external crates
// ---------------------------------------------------------------------------

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng(if seed == 0 { 0xdead_beef_c0ff_ee01 } else { seed })
    }
    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
    pub fn next_u32(&mut self) -> u32 {
        (self.next_u64() >> 32) as u32
    }
    pub fn below(&mut self, n: usize) -> usize {
        if n == 0 {
            0
        } else {
            (self.next_u64() % n as u64) as usize
        }
    }
    pub fn bytes(&mut self, n: usize) -> Vec<u8> {
        (0..n).map(|_| self.next_u32() as u8).collect()
    }
    /// Random NUL-free ASCII-ish string of the given length.
    pub fn ascii(&mut self, n: usize) -> CString {
        let v: Vec<u8> = (0..n).map(|_| 0x21 + (self.next_u32() % 94) as u8).collect();
        CString::new(v).unwrap()
    }
    /// Random NUL-free ASCII string whose length is in `min..=max_incl`.
    pub fn ascii_len(&mut self, min: usize, max_incl: usize) -> CString {
        let n = min + self.below(max_incl + 1 - min);
        self.ascii(n)
    }
    /// Random NUL-free string that also uses bytes >= 0x80.
    pub fn high_bytes_string(&mut self, n: usize) -> CString {
        let v: Vec<u8> = (0..n)
            .map(|_| {
                let b = self.next_u32() as u8;
                if b == 0 {
                    0x80
                } else {
                    b
                }
            })
            .collect();
        CString::new(v).unwrap()
    }
}

// ---------------------------------------------------------------------------
// state snapshots (implementation-independent, pointer-free)
// ---------------------------------------------------------------------------

pub unsafe fn header(arr: *mut c_void) -> ArrayHeader {
    *((arr as *mut u8).sub(HDR_SIZE) as *mut ArrayHeader)
}

/// `t` is the value returned by the hm* functions (array base + elemsize).
pub unsafe fn map_header(t: *mut c_void, elemsize: usize) -> ArrayHeader {
    header((t as *mut u8).sub(elemsize) as *mut c_void)
}

pub unsafe fn cstr(p: *const c_char) -> String {
    if p.is_null() {
        "<null>".to_string()
    } else {
        format!("{:?}", std::ffi::CStr::from_ptr(p))
    }
}

/// How the key field of a map element should be rendered for comparison.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum KeyKind {
    /// key bytes live inline in the element (binary / SH_NONE)
    Inline,
    /// key field is a `char *`; compare the pointed-to text, not the address
    Pointer,
}

/// A textual, pointer-free description of the entire map state.
pub unsafe fn snapshot_map(
    t: *mut c_void,
    elemsize: usize,
    keysize: usize,
    kind: KeyKind,
) -> String {
    let mut out = String::new();
    if t.is_null() {
        return "NULL".into();
    }
    let h = map_header(t, elemsize);
    out.push_str(&format!(
        "len={} cap={} temp={} table={}\n",
        h.length,
        h.capacity,
        h.temp,
        if h.hash_table.is_null() { "no" } else { "yes" }
    ));

    if !h.hash_table.is_null() {
        let ti = &*(h.hash_table as *mut HashIndex);
        out.push_str(&format!(
            "slots={} used={} uct={} ucst={} tomb={} tct={} seed={:#x} log2={} \
             arena(rem={} block={} mode={})\n",
            ti.slot_count,
            ti.used_count,
            ti.used_count_threshold,
            ti.used_count_shrink_threshold,
            ti.tombstone_count,
            ti.tombstone_count_threshold,
            ti.seed,
            ti.slot_count_log2,
            ti.string.remaining,
            ti.string.block,
            ti.string.mode,
        ));
        for b in 0..(ti.slot_count >> 3) {
            let bk = &*ti.storage.add(b);
            out.push_str(&format!("  b{b}: "));
            for s in 0..8 {
                out.push_str(&format!("({:#x},{}) ", bk.hash[s], bk.index[s]));
            }
            out.push('\n');
        }
    }

    // elements: raw index 0 is the "default" slot; hash-relative index i is raw i+1
    let base = (t as *mut u8).sub(elemsize);
    for i in 0..h.length {
        let e = base.add(elemsize * i);
        out.push_str(&format!("  e{i}: "));
        match kind {
            KeyKind::Inline => {
                for k in 0..elemsize {
                    out.push_str(&format!("{:02x}", *e.add(k)));
                }
            }
            KeyKind::Pointer => {
                let kp = std::ptr::read_unaligned(e as *const *const c_char);
                out.push_str(&format!("key={} rest=", cstr(kp)));
                for k in 8..elemsize {
                    out.push_str(&format!("{:02x}", *e.add(k)));
                }
            }
        }
        out.push('\n');
    }
    let _ = keysize;
    out
}

pub unsafe fn snapshot_arena(a: *const StringArena) -> String {
    let a = &*a;
    format!(
        "arena(storage={} remaining={} block={} mode={})",
        if a.storage.is_null() { "null" } else { "some" },
        a.remaining,
        a.block,
        a.mode
    )
}

/// Compares two snapshots and panics with a readable diff.
#[track_caller]
pub fn assert_same(what: &str, c: &str, r: &str) {
    if c != r {
        let cl: Vec<&str> = c.lines().collect();
        let rl: Vec<&str> = r.lines().collect();
        let mut diff = String::new();
        for i in 0..cl.len().max(rl.len()) {
            let a = cl.get(i).copied().unwrap_or("<missing>");
            let b = rl.get(i).copied().unwrap_or("<missing>");
            if a != b {
                diff.push_str(&format!("  line {i}:\n    C   : {a}\n    Rust: {b}\n"));
            }
        }
        panic!("DIVERGENCE in {what}\n{diff}");
    }
}
