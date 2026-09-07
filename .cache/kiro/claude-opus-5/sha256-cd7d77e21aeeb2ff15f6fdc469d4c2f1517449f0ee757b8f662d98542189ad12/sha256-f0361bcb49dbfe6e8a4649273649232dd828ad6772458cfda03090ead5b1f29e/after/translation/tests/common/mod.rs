//! Shared differential-testing harness.
//!
//! BOTH libraries are loaded as shared objects through `libloading` and every
//! call goes through the `.so` export table — the Rust crate is never linked
//! or called directly, so the `#[no_mangle] extern "C"` wrappers are part of
//! what is under test.

#![allow(dead_code)]
#![allow(non_snake_case)]

use std::ffi::{c_char, c_int, c_void};
use std::path::PathBuf;
use std::sync::{Mutex, MutexGuard, OnceLock};

// ---------------------------------------------------------------------------
// Layout constants (verified against a C probe built from the definitions in
// c_src/src/lib.c: header 32, string_block 16, arena 24, bucket 128, index 104)
// ---------------------------------------------------------------------------
pub const HEADER: usize = 32;
pub const HI_SIZE: usize = 104;
pub const BUCKET_LEN: usize = 8;
pub const BUCKET_SHIFT: usize = 3;

pub const STBDS_HM_BINARY: c_int = 0;
pub const STBDS_HM_STRING: c_int = 1;

pub const STBDS_SH_NONE: c_int = 0;
pub const STBDS_SH_DEFAULT: c_int = 1;
pub const STBDS_SH_STRDUP: c_int = 2;
pub const STBDS_SH_ARENA: c_int = 3;

/// `struct stbds_string_arena` — 24 bytes.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct Arena {
    pub storage: *mut c_void,
    pub remaining: usize,
    pub block: u8,
    pub mode: u8,
    pub _pad: [u8; 6],
}

impl Arena {
    pub fn zeroed() -> Arena {
        Arena {
            storage: std::ptr::null_mut(),
            remaining: 0,
            block: 0,
            mode: 0,
            _pad: [0; 6],
        }
    }
}

const _: () = assert!(std::mem::size_of::<Arena>() == 24);

// ---------------------------------------------------------------------------
// Exported-function signatures
// ---------------------------------------------------------------------------
pub type FnArrGrowf = unsafe extern "C" fn(*mut c_void, usize, usize, usize) -> *mut c_void;
pub type FnArrFreef = unsafe extern "C" fn(*mut c_void);
pub type FnRandSeed = unsafe extern "C" fn(usize);
pub type FnHashString = unsafe extern "C" fn(*mut c_char, usize) -> usize;
pub type FnHashBytes = unsafe extern "C" fn(*mut c_void, usize, usize) -> usize;
pub type FnHmFreeFunc = unsafe extern "C" fn(*mut c_void, usize);
pub type FnHmGetKeyTs =
    unsafe extern "C" fn(*mut c_void, usize, *mut c_void, usize, *mut isize, c_int) -> *mut c_void;
pub type FnHmGetKey =
    unsafe extern "C" fn(*mut c_void, usize, *mut c_void, usize, c_int) -> *mut c_void;
pub type FnHmPutDefault = unsafe extern "C" fn(*mut c_void, usize) -> *mut c_void;
pub type FnHmPutKey =
    unsafe extern "C" fn(*mut c_void, usize, *mut c_void, usize, c_int) -> *mut c_void;
pub type FnShmodeFunc = unsafe extern "C" fn(usize, c_int) -> *mut c_void;
pub type FnHmDelKey =
    unsafe extern "C" fn(*mut c_void, usize, *mut c_void, usize, usize, c_int) -> *mut c_void;
pub type FnStralloc = unsafe extern "C" fn(*mut Arena, *mut c_char) -> *mut c_char;
pub type FnStrreset = unsafe extern "C" fn(*mut Arena);
pub type FnStrkey = unsafe extern "C" fn(c_int) -> *mut c_char;
pub type FnHmGeti = unsafe extern "C" fn(c_int);

pub struct Lib {
    pub name: &'static str,
    _lib: libloading::Library,
    pub arrgrowf: FnArrGrowf,
    pub arrfreef: FnArrFreef,
    pub rand_seed: FnRandSeed,
    pub hash_string: FnHashString,
    pub hash_bytes: FnHashBytes,
    pub hmfree_func: FnHmFreeFunc,
    pub hmget_key_ts: FnHmGetKeyTs,
    pub hmget_key: FnHmGetKey,
    pub hmput_default: FnHmPutDefault,
    pub hmput_key: FnHmPutKey,
    pub shmode_func: FnShmodeFunc,
    pub hmdel_key: FnHmDelKey,
    pub stralloc: FnStralloc,
    pub strreset: FnStrreset,
    pub strkey: FnStrkey,
    pub hm_geti: FnHmGeti,
}

macro_rules! sym {
    ($lib:expr, $ty:ty, $name:literal) => {{
        let s: libloading::Symbol<$ty> = $lib
            .get(concat!($name, "\0").as_bytes())
            .unwrap_or_else(|e| panic!("missing symbol {}: {}", $name, e));
        *s
    }};
}

impl Lib {
    pub unsafe fn open(name: &'static str, path: &PathBuf) -> Lib {
        let lib = libloading::Library::new(path)
            .unwrap_or_else(|e| panic!("cannot dlopen {}: {}", path.display(), e));
        let l = Lib {
            name,
            arrgrowf: sym!(lib, FnArrGrowf, "stbds_arrgrowf"),
            arrfreef: sym!(lib, FnArrFreef, "stbds_arrfreef"),
            rand_seed: sym!(lib, FnRandSeed, "stbds_rand_seed"),
            hash_string: sym!(lib, FnHashString, "stbds_hash_string"),
            hash_bytes: sym!(lib, FnHashBytes, "stbds_hash_bytes"),
            hmfree_func: sym!(lib, FnHmFreeFunc, "stbds_hmfree_func"),
            hmget_key_ts: sym!(lib, FnHmGetKeyTs, "stbds_hmget_key_ts"),
            hmget_key: sym!(lib, FnHmGetKey, "stbds_hmget_key"),
            hmput_default: sym!(lib, FnHmPutDefault, "stbds_hmput_default"),
            hmput_key: sym!(lib, FnHmPutKey, "stbds_hmput_key"),
            shmode_func: sym!(lib, FnShmodeFunc, "stbds_shmode_func"),
            hmdel_key: sym!(lib, FnHmDelKey, "stbds_hmdel_key"),
            stralloc: sym!(lib, FnStralloc, "stbds_stralloc"),
            strreset: sym!(lib, FnStrreset, "stbds_strreset"),
            strkey: sym!(lib, FnStrkey, "strkey"),
            hm_geti: sym!(lib, FnHmGeti, "hm_geti"),
            _lib: lib,
        };
        l
    }
}

fn c_so_path() -> PathBuf {
    let dir = PathBuf::from("../c_src/build");
    let mut found: Option<PathBuf> = None;
    if let Ok(rd) = std::fs::read_dir(&dir) {
        for e in rd.flatten() {
            let p = e.path();
            if p.extension().map(|x| x == "so").unwrap_or(false) {
                found = Some(p);
                break;
            }
        }
    }
    found.unwrap_or_else(|| {
        panic!(
            "no .so in {}; build it with: cd c_src && mkdir -p build && cd build && \
             cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
            dir.display()
        )
    })
}

fn rust_so_path() -> PathBuf {
    let p = PathBuf::from("target/release/libhm_geti_lib.so");
    if !p.exists() {
        panic!(
            "{} missing; build it with: cargo build --release",
            p.display()
        );
    }
    p
}

static LIBS: OnceLock<(Lib, Lib)> = OnceLock::new();
static GUARD: OnceLock<Mutex<()>> = OnceLock::new();

/// `(C, Rust)`
pub fn libs() -> &'static (Lib, Lib) {
    LIBS.get_or_init(|| unsafe {
        (
            Lib::open("C", &c_so_path()),
            Lib::open("Rust", &rust_so_path()),
        )
    })
}

/// Both libraries keep a mutable global (`stbds_hash_seed`); every test must
/// drive them in lockstep, so tests are serialized.
pub fn lock() -> MutexGuard<'static, ()> {
    match GUARD.get_or_init(|| Mutex::new(())).lock() {
        Ok(g) => g,
        Err(p) => p.into_inner(),
    }
}

/// Reset both libraries' global hash seed to the C initial value so that the
/// two are in identical global state at the start of every test.
pub fn reset_seeds(seed: usize) {
    let (c, r) = libs();
    unsafe {
        (c.rand_seed)(seed);
        (r.rand_seed)(seed);
    }
}

pub const DEFAULT_SEED: usize = 0x3141_5926;

// ---------------------------------------------------------------------------
// Deterministic PRNG (xorshift64*), fixed seed for reproducibility
// ---------------------------------------------------------------------------
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng(if seed == 0 { 0x2545_F491_4F6C_DD1D } else { seed })
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
    pub fn range(&mut self, lo: usize, hi_inclusive: usize) -> usize {
        lo + self.below(hi_inclusive - lo + 1)
    }
    pub fn bytes(&mut self, n: usize) -> Vec<u8> {
        (0..n).map(|_| (self.next_u64() >> 24) as u8).collect()
    }
    /// Non-zero bytes, NUL-terminated: a valid C string of `n` characters.
    pub fn cstring(&mut self, n: usize) -> Vec<u8> {
        let mut v: Vec<u8> = (0..n)
            .map(|_| {
                let b = (self.next_u64() >> 24) as u8;
                if b == 0 {
                    1
                } else {
                    b
                }
            })
            .collect();
        v.push(0);
        v
    }
    /// `ascii_cstring` with a randomly chosen length, avoiding a nested
    /// mutable borrow at the call site.
    pub fn ascii_cstring_range(&mut self, lo: usize, hi: usize) -> Vec<u8> {
        let n = self.range(lo, hi);
        self.ascii_cstring(n)
    }
    pub fn cstring_range(&mut self, lo: usize, hi: usize) -> Vec<u8> {
        let n = self.range(lo, hi);
        self.cstring(n)
    }
    pub fn bytes_range(&mut self, lo: usize, hi: usize) -> Vec<u8> {
        let n = self.range(lo, hi);
        self.bytes(n)
    }
    pub fn ascii_cstring(&mut self, n: usize) -> Vec<u8> {
        let mut v: Vec<u8> = (0..n)
            .map(|_| b'a' + (self.next_u64() % 26) as u8)
            .collect();
        v.push(0);
        v
    }
}

// ---------------------------------------------------------------------------
// State snapshots
// ---------------------------------------------------------------------------

/// How to interpret the first 8 bytes of an element when snapshotting.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum KeyKind {
    /// Raw bytes; compared verbatim.
    Binary,
    /// `char *` at `keyoffset`; the pointer differs between libraries, so the
    /// pointed-to NUL-terminated bytes are captured instead.
    StringPtr { keyoffset: usize },
}

#[derive(PartialEq, Eq, Debug, Clone)]
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
    pub arena_block_count: usize,
    /// `temp_key` field: `None` for NULL, else the string it points at.
    pub temp_key: Option<Vec<u8>>,
    /// `(hash, index)` for every slot, in slot order.
    pub slots: Vec<(usize, isize)>,
}

#[derive(PartialEq, Eq, Debug, Clone)]
pub struct MapSnap {
    pub is_null: bool,
    pub length: usize,
    pub capacity: usize,
    pub temp: isize,
    pub has_table: bool,
    /// element `i` in `0..length`, decoded per `KeyKind`
    pub elements: Vec<Vec<u8>>,
    pub table: Option<TableSnap>,
}

unsafe fn read<T: Copy>(p: *const u8) -> T {
    std::ptr::read_unaligned(p as *const T)
}

unsafe fn cstr_bytes(p: *const c_char) -> Vec<u8> {
    let mut v = Vec::new();
    let mut q = p as *const u8;
    while *q != 0 {
        v.push(*q);
        q = q.add(1);
    }
    v
}

unsafe fn snap_table(t: *const u8, kind: KeyKind, cmp_temp_key: bool) -> TableSnap {
    let slot_count: usize = read(t.add(8));
    let arena_storage: *mut c_void = read(t.add(72));
    let mut arena_block_count = 0usize;
    {
        let mut b = arena_storage as *const u8;
        while !b.is_null() {
            arena_block_count += 1;
            b = read::<*const u8>(b);
            if arena_block_count > 100_000 {
                break;
            }
        }
    }
    let storage: *const u8 = read(t.add(96));
    let mut slots = Vec::with_capacity(slot_count);
    for bi in 0..(slot_count >> BUCKET_SHIFT) {
        let bucket = storage.add(bi * 128);
        for j in 0..BUCKET_LEN {
            let h: usize = read(bucket.add(j * 8));
            let ix: isize = read(bucket.add(64 + j * 8));
            slots.push((h, ix));
        }
    }
    let temp_key_ptr: *const c_char = read(t);
    // `temp_key` is only written by `stbds_hmput_key` in string mode; before
    // the first such write it holds realloc garbage in BOTH libraries and must
    // not be dereferenced or compared.
    let temp_key = if !cmp_temp_key || temp_key_ptr.is_null() {
        None
    } else {
        match kind {
            KeyKind::Binary => None,
            KeyKind::StringPtr { .. } => Some(cstr_bytes(temp_key_ptr)),
        }
    };
    TableSnap {
        slot_count,
        used_count: read(t.add(16)),
        used_count_threshold: read(t.add(24)),
        used_count_shrink_threshold: read(t.add(32)),
        tombstone_count: read(t.add(40)),
        tombstone_count_threshold: read(t.add(48)),
        seed: read(t.add(56)),
        slot_count_log2: read(t.add(64)),
        arena_remaining: read(t.add(80)),
        arena_block: read(t.add(88)),
        arena_mode: read(t.add(89)),
        arena_has_storage: !arena_storage.is_null(),
        arena_block_count,
        temp_key,
        slots,
    }
}

/// Snapshot a *map handle* (the `void *` the `stbds_hm*` entry points return,
/// which points at element 1 of the underlying array).
pub unsafe fn snap_map(
    h: *mut c_void,
    elemsize: usize,
    kind: KeyKind,
    cmp_temp_key: bool,
) -> MapSnap {
    if h.is_null() {
        return MapSnap {
            is_null: true,
            length: 0,
            capacity: 0,
            temp: 0,
            has_table: false,
            elements: Vec::new(),
            table: None,
        };
    }
    let raw = (h as *mut u8).sub(elemsize);
    snap_arr(raw as *mut c_void, elemsize, kind, cmp_temp_key)
}

/// Snapshot a *raw array* pointer (what `stbds_arrgrowf` returns).
pub unsafe fn snap_arr(
    raw: *mut c_void,
    elemsize: usize,
    kind: KeyKind,
    cmp_temp_key: bool,
) -> MapSnap {
    if raw.is_null() {
        return MapSnap {
            is_null: true,
            length: 0,
            capacity: 0,
            temp: 0,
            has_table: false,
            elements: Vec::new(),
            table: None,
        };
    }
    let hdr = (raw as *mut u8).sub(HEADER);
    let length: usize = read(hdr);
    let capacity: usize = read(hdr.add(8));
    let table: *const u8 = read(hdr.add(16));
    let temp: isize = read(hdr.add(24));

    let mut elements = Vec::with_capacity(length);
    for i in 0..length {
        let e = (raw as *const u8).add(i * elemsize);
        match kind {
            KeyKind::Binary => elements.push(std::slice::from_raw_parts(e, elemsize).to_vec()),
            KeyKind::StringPtr { keyoffset } => {
                let mut v = Vec::with_capacity(elemsize + 8);
                // bytes before the key pointer
                v.extend_from_slice(std::slice::from_raw_parts(e, keyoffset));
                let kp: *const c_char = read(e.add(keyoffset));
                if kp.is_null() {
                    v.push(0xEE);
                } else {
                    v.push(0xFF);
                    v.extend_from_slice(&cstr_bytes(kp));
                    v.push(0);
                }
                // bytes after the key pointer
                if elemsize > keyoffset + 8 {
                    v.extend_from_slice(std::slice::from_raw_parts(
                        e.add(keyoffset + 8),
                        elemsize - keyoffset - 8,
                    ));
                }
                elements.push(v);
            }
        }
    }

    MapSnap {
        is_null: false,
        length,
        capacity,
        temp,
        has_table: !table.is_null(),
        elements,
        table: if table.is_null() {
            None
        } else {
            Some(snap_table(table, kind, cmp_temp_key))
        },
    }
}

/// Fill the non-key tail of element `idx` (0-based over the raw array) with a
/// deterministic pattern so that no `realloc` garbage is ever compared.
pub unsafe fn write_tail(raw: *mut c_void, elemsize: usize, idx: usize, keysize: usize, tag: u8) {
    let e = (raw as *mut u8).add(idx * elemsize);
    let start = keysize.min(elemsize);
    for k in start..elemsize {
        *e.add(k) = tag.wrapping_add(k as u8).wrapping_mul(31);
    }
}

/// Same, addressed through a map handle and a `temp` index.
pub unsafe fn write_tail_at(
    h: *mut c_void,
    elemsize: usize,
    temp: isize,
    keysize: usize,
    tag: u8,
) {
    if temp < 0 {
        return;
    }
    let e = (h as *mut u8).add(temp as usize * elemsize);
    let start = keysize.min(elemsize);
    for k in start..elemsize {
        *e.add(k) = tag.wrapping_add(k as u8).wrapping_mul(31);
    }
}

#[track_caller]
pub fn same<T: PartialEq + std::fmt::Debug>(what: &str, c: T, r: T) {
    assert!(c == r, "DIVERGENCE [{what}]\n  C   : {c:?}\n  Rust: {r:?}");
}

// ---------------------------------------------------------------------------
// Child-process execution, for the abort / fault rows of ERRORS.md
// ---------------------------------------------------------------------------
extern "C" {
    fn fork() -> c_int;
    fn waitpid(pid: c_int, status: *mut c_int, options: c_int) -> c_int;
    fn _exit(code: c_int) -> !;
    fn close(fd: c_int) -> c_int;
    fn setrlimit(resource: c_int, rlim: *const RLimit) -> c_int;
}

#[repr(C)]
struct RLimit {
    cur: u64,
    max: u64,
}

const RLIMIT_CORE: c_int = 4;

#[derive(PartialEq, Eq, Debug, Clone, Copy)]
pub enum Outcome {
    Exited(c_int),
    Signaled(c_int),
    Unknown(c_int),
}

/// Run `f` in a forked child with stderr closed; report how the child died.
/// This is how "C aborts / faults here" is compared with "Rust aborts / faults
/// here" — the comparison is on the exact termination status, not on "both
/// failed somehow".
pub fn in_child<F: FnOnce()>(f: F) -> Outcome {
    unsafe {
        let pid = fork();
        assert!(pid >= 0, "fork failed");
        if pid == 0 {
            close(2); // silence assert()'s diagnostic
            let z = RLimit { cur: 0, max: 0 };
            setrlimit(RLIMIT_CORE, &z); // do not spend time writing core files
            f();
            _exit(0);
        }
        let mut status: c_int = 0;
        let r = waitpid(pid, &mut status, 0);
        assert!(r == pid, "waitpid failed");
        let low = status & 0x7f;
        if low == 0 {
            Outcome::Exited((status >> 8) & 0xff)
        } else if low == 0x7f {
            Outcome::Unknown(status)
        } else {
            Outcome::Signaled(low)
        }
    }
}

pub const SIGABRT: c_int = 6;
pub const SIGSEGV: c_int = 11;
pub const SIGBUS: c_int = 7;

/// A fault outcome: either a segfault/bus error or an abort. Some rows are
/// "wild pointer handed to free()", where glibc may abort or fault depending
/// on the address; what matters is that both libraries do the *same* thing.
pub fn is_fatal(o: Outcome) -> bool {
    matches!(o, Outcome::Signaled(SIGABRT | SIGSEGV | SIGBUS))
}

// ---------------------------------------------------------------------------
// MapPair: drives the C and the Rust map through the identical call sequence
// and compares the complete observable state after every single operation.
// ---------------------------------------------------------------------------
pub struct MapPair {
    pub ch: *mut c_void,
    pub rh: *mut c_void,
    pub elemsize: usize,
    pub keysize: usize,
    pub kind: KeyKind,
    /// Keys whose *pointer* the library stores (`STBDS_SH_DEFAULT`) must stay
    /// alive for the lifetime of the map; one private copy per library so the
    /// two never share an address.
    keep: Vec<Box<[u8]>>,
    tag: u8,
    /// `table->temp_key` only becomes defined after the first string-mode
    /// `stbds_hmput_key`, and the pointer it holds becomes DANGLING as soon as
    /// a `STBDS_SH_STRDUP` key is deleted (the C code frees it without clearing
    /// `temp_key`). Tests therefore opt in explicitly, and only for
    /// put-only workloads on tables whose keys are never freed.
    pub cmp_temp_key: bool,
}

impl MapPair {
    pub fn new_null(elemsize: usize, keysize: usize, kind: KeyKind) -> MapPair {
        MapPair {
            ch: std::ptr::null_mut(),
            rh: std::ptr::null_mut(),
            elemsize,
            keysize,
            kind,
            keep: Vec::new(),
            tag: 1,
            cmp_temp_key: false,
        }
    }

    /// `stbds_shmode_func(elemsize, mode)` on both libraries.
    pub fn from_shmode(elemsize: usize, keysize: usize, mode: c_int, kind: KeyKind) -> MapPair {
        let (c, r) = libs();
        let mut m = MapPair::new_null(elemsize, keysize, kind);
        unsafe {
            m.ch = (c.shmode_func)(elemsize, mode);
            m.rh = (r.shmode_func)(elemsize, mode);
        }
        m.check("shmode_func");
        m
    }

    pub fn snap_c(&self) -> MapSnap {
        unsafe { snap_map(self.ch, self.elemsize, self.kind, self.cmp_temp_key) }
    }
    pub fn snap_r(&self) -> MapSnap {
        unsafe { snap_map(self.rh, self.elemsize, self.kind, self.cmp_temp_key) }
    }

    #[track_caller]
    pub fn check(&self, what: &str) {
        same(what, self.snap_c(), self.snap_r());
    }

    unsafe fn temp_of(&self, h: *mut c_void) -> isize {
        read::<isize>((h as *const u8).sub(self.elemsize + 8))
    }

    #[track_caller]
    pub fn put_default(&mut self, what: &str) {
        let (c, r) = libs();
        unsafe {
            self.ch = (c.hmput_default)(self.ch, self.elemsize);
            self.rh = (r.hmput_default)(self.rh, self.elemsize);
        }
        same(
            &format!("{what}: hmput_default null-ness"),
            self.ch.is_null(),
            self.rh.is_null(),
        );
        self.check(what);
    }

    /// Write the default-slot payload, mirroring what `hmdefault` does.
    pub fn set_default_tail(&mut self, tag: u8) {
        unsafe {
            if !self.ch.is_null() {
                let craw = (self.ch as *mut u8).sub(self.elemsize) as *mut c_void;
                let rraw = (self.rh as *mut u8).sub(self.elemsize) as *mut c_void;
                write_tail(craw, self.elemsize, 0, 0, tag);
                write_tail(rraw, self.elemsize, 0, 0, tag);
            }
        }
    }

    /// `stbds_hmput_key` + writing the value payload, as a real consumer does.
    #[track_caller]
    pub fn put(&mut self, what: &str, key: &[u8], mode: c_int) -> isize {
        let (c, r) = libs();
        let tag = self.tag;
        self.tag = self.tag.wrapping_mul(31).wrapping_add(17);
        let (ct, rt) = unsafe {
            match self.kind {
                KeyKind::Binary => {
                    let mut ck = key.to_vec();
                    let mut rk = key.to_vec();
                    self.ch = (c.hmput_key)(
                        self.ch,
                        self.elemsize,
                        ck.as_mut_ptr() as *mut c_void,
                        self.keysize,
                        mode,
                    );
                    self.rh = (r.hmput_key)(
                        self.rh,
                        self.elemsize,
                        rk.as_mut_ptr() as *mut c_void,
                        self.keysize,
                        mode,
                    );
                    let ct = self.temp_of(self.ch);
                    let rt = self.temp_of(self.rh);
                    write_tail_at(self.ch, self.elemsize, ct, self.keysize, tag);
                    write_tail_at(self.rh, self.elemsize, rt, self.keysize, tag);
                    (ct, rt)
                }
                KeyKind::StringPtr { .. } => {
                    let ck: Box<[u8]> = key.to_vec().into_boxed_slice();
                    let rk: Box<[u8]> = key.to_vec().into_boxed_slice();
                    let cp = ck.as_ptr() as *mut c_void;
                    let rp = rk.as_ptr() as *mut c_void;
                    self.keep.push(ck);
                    self.keep.push(rk);
                    self.ch = (c.hmput_key)(self.ch, self.elemsize, cp, self.keysize, mode);
                    self.rh = (r.hmput_key)(self.rh, self.elemsize, rp, self.keysize, mode);
                    let ct = self.temp_of(self.ch);
                    let rt = self.temp_of(self.rh);
                    write_tail_at(self.ch, self.elemsize, ct, 8, tag);
                    write_tail_at(self.rh, self.elemsize, rt, 8, tag);
                    (ct, rt)
                }
            }
        };
        same(&format!("{what}: put temp"), ct, rt);
        self.check(&format!("{what}: put state"));
        ct
    }

    #[track_caller]
    pub fn get(&mut self, what: &str, key: &[u8], mode: c_int) -> isize {
        let (c, r) = libs();
        let (ct, rt) = unsafe {
            let mut ck = key.to_vec();
            let mut rk = key.to_vec();
            self.ch = (c.hmget_key)(
                self.ch,
                self.elemsize,
                ck.as_mut_ptr() as *mut c_void,
                self.keysize,
                mode,
            );
            self.rh = (r.hmget_key)(
                self.rh,
                self.elemsize,
                rk.as_mut_ptr() as *mut c_void,
                self.keysize,
                mode,
            );
            (self.temp_of(self.ch), self.temp_of(self.rh))
        };
        same(&format!("{what}: get temp"), ct, rt);
        self.check(&format!("{what}: get state"));
        ct
    }

    #[track_caller]
    pub fn get_ts(&mut self, what: &str, key: &[u8], mode: c_int) -> isize {
        let (c, r) = libs();
        let mut ctemp: isize = 0x5555_5555;
        let mut rtemp: isize = 0x5555_5555;
        unsafe {
            let mut ck = key.to_vec();
            let mut rk = key.to_vec();
            self.ch = (c.hmget_key_ts)(
                self.ch,
                self.elemsize,
                ck.as_mut_ptr() as *mut c_void,
                self.keysize,
                &mut ctemp,
                mode,
            );
            self.rh = (r.hmget_key_ts)(
                self.rh,
                self.elemsize,
                rk.as_mut_ptr() as *mut c_void,
                self.keysize,
                &mut rtemp,
                mode,
            );
        }
        same(&format!("{what}: get_ts temp"), ctemp, rtemp);
        self.check(&format!("{what}: get_ts state"));
        ctemp
    }

    #[track_caller]
    pub fn del(&mut self, what: &str, key: &[u8], keyoffset: usize, mode: c_int) -> isize {
        let (c, r) = libs();
        let (ct, rt) = unsafe {
            let mut ck = key.to_vec();
            let mut rk = key.to_vec();
            self.ch = (c.hmdel_key)(
                self.ch,
                self.elemsize,
                ck.as_mut_ptr() as *mut c_void,
                self.keysize,
                keyoffset,
                mode,
            );
            self.rh = (r.hmdel_key)(
                self.rh,
                self.elemsize,
                rk.as_mut_ptr() as *mut c_void,
                self.keysize,
                keyoffset,
                mode,
            );
            let ct = if self.ch.is_null() {
                0
            } else {
                self.temp_of(self.ch)
            };
            let rt = if self.rh.is_null() {
                0
            } else {
                self.temp_of(self.rh)
            };
            (ct, rt)
        };
        same(
            &format!("{what}: del null-ness"),
            self.ch.is_null(),
            self.rh.is_null(),
        );
        same(&format!("{what}: del temp"), ct, rt);
        self.check(&format!("{what}: del state"));
        ct
    }

    #[track_caller]
    pub fn free(&mut self) {
        let (c, r) = libs();
        unsafe {
            if !self.ch.is_null() {
                (c.hmfree_func)(
                    (self.ch as *mut u8).sub(self.elemsize) as *mut c_void,
                    self.elemsize,
                );
            }
            if !self.rh.is_null() {
                (r.hmfree_func)(
                    (self.rh as *mut u8).sub(self.elemsize) as *mut c_void,
                    self.elemsize,
                );
            }
        }
        self.ch = std::ptr::null_mut();
        self.rh = std::ptr::null_mut();
        self.keep.clear();
        self.cmp_temp_key = false;
    }
}
