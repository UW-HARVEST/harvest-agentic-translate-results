//! Shared differential-test harness.
//!
//! BOTH libraries are loaded through `libloading` (never by calling Rust
//! functions directly), so the `#[no_mangle] extern "C"` wrappers are part of
//! what is under test.

#![allow(dead_code)]

use libloading::Library;
use std::ffi::{c_char, c_int, c_void};
use std::path::PathBuf;
use std::sync::{Mutex, MutexGuard, OnceLock};

// ---------------------------------------------------------------------------
// layout mirrors of the C structs (used to inspect library-internal state)
// ---------------------------------------------------------------------------

pub const HDR: usize = std::mem::size_of::<Header>(); // 32

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
pub struct Bucket {
    pub hash: [usize; 8],
    pub index: [isize; 8],
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct Arena {
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
    pub string: Arena,
    pub storage: *mut Bucket,
}

pub const STBDS_HM_BINARY: c_int = 0;
pub const STBDS_HM_STRING: c_int = 1;
pub const STBDS_HM_PTR_TO_STRING: c_int = 2;

pub const STBDS_SH_NONE: c_int = 0;
pub const STBDS_SH_DEFAULT: c_int = 1;
pub const STBDS_SH_STRDUP: c_int = 2;
pub const STBDS_SH_ARENA: c_int = 3;

// ---------------------------------------------------------------------------
// the FFI surface
// ---------------------------------------------------------------------------

pub struct Api {
    pub name: &'static str,
    pub arrgrowf: unsafe extern "C" fn(*mut c_void, usize, usize, usize) -> *mut c_void,
    pub arrfreef: unsafe extern "C" fn(*mut c_void),
    pub rand_seed: unsafe extern "C" fn(usize),
    pub hash_string: unsafe extern "C" fn(*mut c_char, usize) -> usize,
    pub hash_bytes: unsafe extern "C" fn(*mut c_void, usize, usize) -> usize,
    pub hmfree_func: unsafe extern "C" fn(*mut c_void, usize),
    pub hmget_key_ts:
        unsafe extern "C" fn(*mut c_void, usize, *mut c_void, usize, *mut isize, c_int) -> *mut c_void,
    pub hmget_key: unsafe extern "C" fn(*mut c_void, usize, *mut c_void, usize, c_int) -> *mut c_void,
    pub hmput_default: unsafe extern "C" fn(*mut c_void, usize) -> *mut c_void,
    pub hmput_key: unsafe extern "C" fn(*mut c_void, usize, *mut c_void, usize, c_int) -> *mut c_void,
    pub shmode_func: unsafe extern "C" fn(usize, c_int) -> *mut c_void,
    pub hmdel_key:
        unsafe extern "C" fn(*mut c_void, usize, *mut c_void, usize, usize, c_int) -> *mut c_void,
    pub stralloc: unsafe extern "C" fn(*mut Arena, *mut c_char) -> *mut c_char,
    pub strreset: unsafe extern "C" fn(*mut Arena),
    pub strkey: unsafe extern "C" fn(c_int) -> *mut c_char,
    pub helxo: unsafe extern "C" fn(c_char),
}

macro_rules! sym {
    ($lib:expr, $name:literal) => {{
        let s: libloading::Symbol<_> = $lib
            .get(concat!($name, "\0").as_bytes())
            .unwrap_or_else(|e| panic!("missing symbol {}: {}", $name, e));
        *s
    }};
}

unsafe fn load(name: &'static str, path: &PathBuf) -> Api {
    let lib: &'static Library = Box::leak(Box::new(
        Library::new(path).unwrap_or_else(|e| panic!("dlopen {:?}: {}", path, e)),
    ));
    Api {
        name,
        arrgrowf: sym!(lib, "stbds_arrgrowf"),
        arrfreef: sym!(lib, "stbds_arrfreef"),
        rand_seed: sym!(lib, "stbds_rand_seed"),
        hash_string: sym!(lib, "stbds_hash_string"),
        hash_bytes: sym!(lib, "stbds_hash_bytes"),
        hmfree_func: sym!(lib, "stbds_hmfree_func"),
        hmget_key_ts: sym!(lib, "stbds_hmget_key_ts"),
        hmget_key: sym!(lib, "stbds_hmget_key"),
        hmput_default: sym!(lib, "stbds_hmput_default"),
        hmput_key: sym!(lib, "stbds_hmput_key"),
        shmode_func: sym!(lib, "stbds_shmode_func"),
        hmdel_key: sym!(lib, "stbds_hmdel_key"),
        stralloc: sym!(lib, "stbds_stralloc"),
        strreset: sym!(lib, "stbds_strreset"),
        strkey: sym!(lib, "strkey"),
        helxo: sym!(lib, "helxo"),
    }
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

pub fn c_lib_path() -> PathBuf {
    if let Ok(p) = std::env::var("HARVEST_C_LIB") {
        return PathBuf::from(p);
    }
    let dir = manifest_dir().parent().unwrap().join("c_src/build");
    let mut found: Vec<PathBuf> = std::fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("cannot read {:?} ({}): build the C library first", dir, e))
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().map(|x| x == "so").unwrap_or(false))
        .collect();
    found.sort();
    assert_eq!(found.len(), 1, "expected exactly one .so in {:?}, got {:?}", dir, found);
    found.pop().unwrap()
}

pub fn rust_lib_path() -> PathBuf {
    if let Ok(p) = std::env::var("HARVEST_RUST_LIB") {
        return PathBuf::from(p);
    }
    // test the cdylib of the *same* profile the tests were built with, so that
    // `cargo test` and `cargo test --release` both exercise a freshly built .so
    let (first, second) = if cfg!(debug_assertions) {
        ("target/debug", "target/release")
    } else {
        ("target/release", "target/debug")
    };
    let p = manifest_dir().join(first).join("libhelxo_lib.so");
    if p.exists() {
        return p;
    }
    manifest_dir().join(second).join("libhelxo_lib.so")
}

struct Libs {
    c: Api,
    r: Api,
}
unsafe impl Send for Libs {}
unsafe impl Sync for Libs {}

static LIBS: OnceLock<Libs> = OnceLock::new();
static LOCK: Mutex<()> = Mutex::new(());

/// Both libraries keep *process-global* state (`stbds_hash_seed`, `buffer`), so
/// every test serialises on this guard and the two libraries always receive the
/// exact same sequence of calls.
/// Load exactly one of the two libraries (used by the crash-parity child
/// processes, which must not have the other library in their address space).
pub fn load_single(which: &str) -> Api {
    unsafe {
        match which {
            "c" => load("C", &c_lib_path()),
            "rust" => load("Rust", &rust_lib_path()),
            other => panic!("unknown library {:?}", other),
        }
    }
}

/// NOTE: the returned guard is *not* reentrant - pass `&Api` down to helpers
/// instead of calling `libs()` again while a guard is alive.
pub fn libs() -> (&'static Api, &'static Api, MutexGuard<'static, ()>) {
    let g = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let l = LIBS.get_or_init(|| unsafe {
        Libs {
            c: load("C", &c_lib_path()),
            r: load("Rust", &rust_lib_path()),
        }
    });
    (&l.c, &l.r, g)
}

// ---------------------------------------------------------------------------
// deterministic PRNG (xorshift64*) - property-style testing with a fixed seed
// ---------------------------------------------------------------------------

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng(seed | 1)
    }
    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
    pub fn below(&mut self, n: usize) -> usize {
        (self.next_u64() % (n as u64)) as usize
    }
    pub fn byte(&mut self) -> u8 {
        (self.next_u64() >> 33) as u8
    }
    pub fn bytes(&mut self, n: usize) -> Vec<u8> {
        (0..n).map(|_| self.byte()).collect()
    }
    /// NUL-terminated random string; bytes are 1..=255 so high-bit bytes are
    /// exercised too (the C hashes `(unsigned char)*str`).
    pub fn cstring(&mut self, len: usize) -> Vec<u8> {
        let mut v: Vec<u8> = (0..len).map(|_| 1 + (self.byte() % 255)).collect();
        v.push(0);
        v
    }
}

// ---------------------------------------------------------------------------
// stable key storage
//
// Every key buffer is zero-padded to `PAD` bytes so that a `memcpy(elem, key,
// keysize)` performed by the library (the `switch default:` branch) can never
// read uninitialised/racy memory, whatever `keysize` the test uses.
// ---------------------------------------------------------------------------

pub const PAD: usize = 128;

#[derive(Default)]
pub struct Keys {
    store: Vec<Box<[u8]>>,
}

impl Keys {
    pub fn new() -> Keys {
        Keys { store: Vec::new() }
    }
    /// raw key bytes (binary mode)
    pub fn raw(&mut self, v: &[u8]) -> *mut c_void {
        let mut b = vec![0u8; PAD.max(v.len() + 8)];
        b[..v.len()].copy_from_slice(v);
        let b = b.into_boxed_slice();
        let p = b.as_ptr() as *mut c_void;
        self.store.push(b);
        p
    }
    /// NUL-terminated string key (`v` must not contain a NUL)
    pub fn string(&mut self, v: &[u8]) -> *mut c_void {
        assert!(!v.contains(&0));
        let mut b = vec![0u8; PAD.max(v.len() + 8)];
        b[..v.len()].copy_from_slice(v);
        let b = b.into_boxed_slice();
        let p = b.as_ptr() as *mut c_void;
        self.store.push(b);
        p
    }
    pub fn len(&self) -> usize {
        self.store.len()
    }
}

// ---------------------------------------------------------------------------
// map configuration + lock-step driver
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, Debug)]
pub struct MapCfg {
    /// `sizeof *t`
    pub elemsize: usize,
    /// `sizeof (t)->key` (bytes hashed / compared)
    pub keysize: usize,
    /// `STBDS_OFFSETOF(t,key)` handed to `stbds_hmdel_key`
    pub keyoffset: usize,
    /// offset of the value field the macros write
    pub valoff: usize,
    pub valsize: usize,
    /// `mode` argument handed to put/get/del
    pub mode: c_int,
    /// what `table->string.mode` is expected to be (drives snapshotting)
    pub smode: c_int,
}

impl MapCfg {
    /// `table->string.mode ∈ {DEFAULT, STRDUP, ARENA}` ⇒ the element holds a `char *`
    pub fn stores_pointer_key(&self) -> bool {
        matches!(self.smode, 1 | 2 | 3)
    }
    /// `mode >= STBDS_HM_STRING` ⇒ the *input* key must be a NUL-terminated string
    pub fn string_input(&self) -> bool {
        self.mode >= STBDS_HM_STRING
    }
    pub fn binary(elemsize: usize, keysize: usize, valoff: usize, valsize: usize, mode: c_int) -> MapCfg {
        MapCfg { elemsize, keysize, keyoffset: 0, valoff, valsize, mode, smode: 0 }
    }
    pub fn strmap(elemsize: usize, valoff: usize, valsize: usize, mode: c_int, smode: c_int) -> MapCfg {
        MapCfg { elemsize, keysize: 8, keyoffset: 0, valoff, valsize, mode, smode }
    }
}

/// A pair of maps (one per library) driven in lock step.
pub struct DualMap<'a> {
    pub c: *mut u8,
    pub r: *mut u8,
    pub cfg: MapCfg,
    pub api_c: &'a Api,
    pub api_r: &'a Api,
    pub trace: Vec<String>,
}

pub unsafe fn read_header(t: *mut u8) -> Header {
    std::ptr::read_unaligned(t.sub(HDR) as *const Header)
}

unsafe fn cstr(p: *const c_char) -> String {
    if p.is_null() {
        return "<null>".into();
    }
    let mut out = Vec::new();
    let mut i = 0isize;
    loop {
        let b = *(p.offset(i) as *const u8);
        if b == 0 {
            break;
        }
        out.push(b);
        i += 1;
        assert!(i < 1 << 20, "unterminated string");
    }
    format!("{:?}", out)
}

/// Canonical, pointer-free dump of everything the C library makes observable
/// about a map: array header, hash index, every bucket slot, and the payload.
pub unsafe fn snapshot(t: *mut u8, cfg: &MapCfg) -> String {
    if t.is_null() {
        return "MAP=NULL".to_string();
    }
    let mut s = String::new();
    // `t` is the hash-side pointer, the array-side pointer is t - elemsize
    let arr = t.sub(cfg.elemsize);
    let h = read_header(arr);
    s.push_str(&format!(
        "hdr{{len={},cap={},temp={},table={}}}\n",
        h.length,
        h.capacity,
        h.temp,
        if h.hash_table.is_null() { "null" } else { "some" }
    ));
    if !h.hash_table.is_null() {
        let ti = std::ptr::read_unaligned(h.hash_table as *const HashIndex);
        s.push_str(&format!(
            "idx{{slots={},used={},uct={},ucst={},tomb={},tct={},seed={:#x},log2={}}}\n",
            ti.slot_count,
            ti.used_count,
            ti.used_count_threshold,
            ti.used_count_shrink_threshold,
            ti.tombstone_count,
            ti.tombstone_count_threshold,
            ti.seed,
            ti.slot_count_log2
        ));
        s.push_str(&format!(
            "arena{{remaining={},block={},mode={},storage={}}}\n",
            ti.string.remaining,
            ti.string.block,
            ti.string.mode,
            if ti.string.storage.is_null() { "null" } else { "some" }
        ));
        assert!(!ti.storage.is_null());
        // STBDS_ALIGN_FWD((size_t)(t+1), STBDS_CACHE_LINE_SIZE): the bucket array
        // must be cache-line aligned and must not overlap the index struct
        assert_eq!(ti.storage as usize % 64, 0, "bucket storage is not 64-byte aligned");
        assert!(
            ti.storage as usize >= h.hash_table as usize + std::mem::size_of::<HashIndex>(),
            "bucket storage overlaps the hash index"
        );
        for b in 0..(ti.slot_count >> 3) {
            let bk = std::ptr::read_unaligned(ti.storage.add(b));
            s.push_str(&format!("bucket{}:", b));
            for j in 0..8 {
                s.push_str(&format!(" {:#x}/{}", bk.hash[j], bk.index[j]));
            }
            s.push('\n');
        }
    }
    // payload: raw element 0 is the memset(0) "default" slot, elements 1.. are
    // the user-visible entries t[0..len-1]
    for i in 0..h.length {
        let e = arr.add(cfg.elemsize * i);
        s.push_str(&format!("elem{}:", i));
        if i == 0 {
            let all = std::slice::from_raw_parts(e, cfg.elemsize);
            s.push_str(&format!(" default={:?}", all));
        } else {
            if cfg.stores_pointer_key() {
                let kp = std::ptr::read_unaligned(e as *const *const c_char);
                s.push_str(&format!(" key={}", cstr(kp)));
            } else {
                let k = std::slice::from_raw_parts(e, cfg.keysize);
                s.push_str(&format!(" key={:?}", k));
            }
            let v = std::slice::from_raw_parts(e.add(cfg.valoff), cfg.valsize);
            s.push_str(&format!(" val={:?}", v));
        }
        s.push('\n');
    }
    s
}

impl<'a> DualMap<'a> {
    pub fn new(api_c: &'a Api, api_r: &'a Api, cfg: MapCfg) -> DualMap<'a> {
        DualMap {
            c: std::ptr::null_mut(),
            r: std::ptr::null_mut(),
            cfg,
            api_c,
            api_r,
            trace: Vec::new(),
        }
    }

    /// Seed both libraries identically, then create the map through
    /// `stbds_shmode_func` (explicit `string.mode`).
    pub unsafe fn new_shmode(
        api_c: &'a Api,
        api_r: &'a Api,
        cfg: MapCfg,
        seed: usize,
        mode: c_int,
    ) -> DualMap<'a> {
        let mut m = DualMap::new(api_c, api_r, cfg);
        (api_c.rand_seed)(seed);
        (api_r.rand_seed)(seed);
        m.c = (api_c.shmode_func)(cfg.elemsize, mode) as *mut u8;
        m.r = (api_r.shmode_func)(cfg.elemsize, mode) as *mut u8;
        m.trace.push(format!("shmode_func(elemsize={},mode={})", cfg.elemsize, mode));
        m.check("after shmode_func");
        m
    }

    /// Seed both libraries identically; the map is created lazily by the first
    /// `stbds_hmput_key`/`stbds_hmget_key` (the `NULL` path).
    pub unsafe fn new_lazy(api_c: &'a Api, api_r: &'a Api, cfg: MapCfg, seed: usize) -> DualMap<'a> {
        (api_c.rand_seed)(seed);
        (api_r.rand_seed)(seed);
        DualMap::new(api_c, api_r, cfg)
    }

    pub unsafe fn check(&self, what: &str) {
        let sc = snapshot(self.c, &self.cfg);
        let sr = snapshot(self.r, &self.cfg);
        if sc != sr {
            let mut msg = format!(
                "DIVERGENCE at {}\ncfg={:?}\ntrace:\n  {}\n",
                what,
                self.cfg,
                self.trace.join("\n  ")
            );
            for (i, (a, b)) in sc.lines().zip(sr.lines()).enumerate() {
                if a != b {
                    msg.push_str(&format!("line {}:\n  C   : {}\n  Rust: {}\n", i, a, b));
                }
            }
            msg.push_str(&format!("--- full C ---\n{}\n--- full Rust ---\n{}", sc, sr));
            panic!("{}", msg);
        }
    }

    /// `stbds_hmput_key` + the value store the `hmput`/`shput` macros perform.
    pub unsafe fn put(&mut self, key: *mut c_void, val: &[u8]) -> isize {
        assert_eq!(val.len(), self.cfg.valsize);
        let cfg = self.cfg;
        self.c = (self.api_c.hmput_key)(self.c as *mut c_void, cfg.elemsize, key, cfg.keysize, cfg.mode)
            as *mut u8;
        self.r = (self.api_r.hmput_key)(self.r as *mut c_void, cfg.elemsize, key, cfg.keysize, cfg.mode)
            as *mut u8;
        let tc = read_header(self.c.sub(cfg.elemsize)).temp;
        let tr = read_header(self.r.sub(cfg.elemsize)).temp;
        assert_eq!(tc, tr, "hmput_key temp mismatch; trace:\n  {}", self.trace.join("\n  "));
        // the macro writes the value at t[temp]
        std::ptr::copy_nonoverlapping(
            val.as_ptr(),
            self.c.offset(cfg.elemsize as isize * tc).add(cfg.valoff),
            cfg.valsize,
        );
        std::ptr::copy_nonoverlapping(
            val.as_ptr(),
            self.r.offset(cfg.elemsize as isize * tr).add(cfg.valoff),
            cfg.valsize,
        );
        self.trace.push(format!("put(key={:?}) -> temp={}", key, tc));
        // `stbds_shputs` reads `table->temp_key`. The C sets it to the *stored*
        // key pointer of the entry it just wrote for every string mode -- except
        // on the wrap-around branch of the probe loop (lib.c:746..759), which
        // leaves the previous value in place. So instead of a fixed expectation
        // both libraries must agree on *which entry* `temp_key` designates
        // (`None` = a stale/never-set value, which is byte-garbage from
        // `realloc` and must not be compared).
        if cfg.stores_pointer_key() {
            let idx = |t: *mut u8| -> Option<usize> {
                let h = read_header(t.sub(cfg.elemsize));
                let ti = std::ptr::read_unaligned(h.hash_table as *const HashIndex);
                (0..h.length).find(|&i| {
                    let stored = std::ptr::read_unaligned(
                        t.offset(cfg.elemsize as isize * (i as isize - 1))
                            .add(cfg.keyoffset) as *const *mut c_char,
                    );
                    i > 0 && stored == ti.temp_key
                })
            };
            assert_eq!(
                idx(self.c),
                idx(self.r),
                "temp_key designates a different entry; trace:\n  {}",
                self.trace.join("\n  ")
            );
        }
        self.check("after hmput_key");
        tc
    }

    /// `stbds_hmgeti` / `stbds_shgeti`
    pub unsafe fn get(&mut self, key: *mut c_void) -> isize {
        let cfg = self.cfg;
        self.c = (self.api_c.hmget_key)(self.c as *mut c_void, cfg.elemsize, key, cfg.keysize, cfg.mode)
            as *mut u8;
        self.r = (self.api_r.hmget_key)(self.r as *mut c_void, cfg.elemsize, key, cfg.keysize, cfg.mode)
            as *mut u8;
        let tc = read_header(self.c.sub(cfg.elemsize)).temp;
        let tr = read_header(self.r.sub(cfg.elemsize)).temp;
        assert_eq!(tc, tr, "hmget_key temp mismatch; trace:\n  {}", self.trace.join("\n  "));
        self.trace.push(format!("get(key={:?}) -> {}", key, tc));
        self.check("after hmget_key");
        tc
    }

    /// `stbds_hmgeti_ts` / thread-safe lookup
    pub unsafe fn get_ts(&mut self, key: *mut c_void) -> isize {
        let cfg = self.cfg;
        let mut tc: isize = 0x5a5a;
        let mut tr: isize = 0x5a5a;
        self.c = (self.api_c.hmget_key_ts)(
            self.c as *mut c_void,
            cfg.elemsize,
            key,
            cfg.keysize,
            &mut tc,
            cfg.mode,
        ) as *mut u8;
        self.r = (self.api_r.hmget_key_ts)(
            self.r as *mut c_void,
            cfg.elemsize,
            key,
            cfg.keysize,
            &mut tr,
            cfg.mode,
        ) as *mut u8;
        assert_eq!(tc, tr, "hmget_key_ts temp mismatch; trace:\n  {}", self.trace.join("\n  "));
        self.trace.push(format!("get_ts(key={:?}) -> {}", key, tc));
        self.check("after hmget_key_ts");
        tc
    }

    /// `stbds_hmdel` / `stbds_shdel`
    pub unsafe fn del(&mut self, key: *mut c_void) -> isize {
        let cfg = self.cfg;
        self.c = (self.api_c.hmdel_key)(
            self.c as *mut c_void,
            cfg.elemsize,
            key,
            cfg.keysize,
            cfg.keyoffset,
            cfg.mode,
        ) as *mut u8;
        self.r = (self.api_r.hmdel_key)(
            self.r as *mut c_void,
            cfg.elemsize,
            key,
            cfg.keysize,
            cfg.keyoffset,
            cfg.mode,
        ) as *mut u8;
        assert_eq!(
            self.c.is_null(),
            self.r.is_null(),
            "hmdel_key NULL-ness mismatch; trace:\n  {}",
            self.trace.join("\n  ")
        );
        let (tc, tr) = if self.c.is_null() {
            (0, 0)
        } else {
            (
                read_header(self.c.sub(cfg.elemsize)).temp,
                read_header(self.r.sub(cfg.elemsize)).temp,
            )
        };
        assert_eq!(tc, tr, "hmdel_key temp mismatch; trace:\n  {}", self.trace.join("\n  "));
        self.trace.push(format!("del(key={:?}) -> {}", key, tc));
        self.check("after hmdel_key");
        tc
    }

    /// `stbds_hmdefault` (value only) via `stbds_hmput_default`
    pub unsafe fn put_default(&mut self, val: &[u8]) {
        let cfg = self.cfg;
        self.c = (self.api_c.hmput_default)(self.c as *mut c_void, cfg.elemsize) as *mut u8;
        self.r = (self.api_r.hmput_default)(self.r as *mut c_void, cfg.elemsize) as *mut u8;
        // (t)[-1].value = v
        std::ptr::copy_nonoverlapping(
            val.as_ptr(),
            self.c.sub(cfg.elemsize).add(cfg.valoff),
            cfg.valsize,
        );
        std::ptr::copy_nonoverlapping(
            val.as_ptr(),
            self.r.sub(cfg.elemsize).add(cfg.valoff),
            cfg.valsize,
        );
        self.trace.push("put_default()".into());
        self.check("after hmput_default");
    }

    pub unsafe fn len(&self) -> isize {
        if self.c.is_null() {
            0
        } else {
            read_header(self.c.sub(self.cfg.elemsize)).length as isize - 1
        }
    }

    pub unsafe fn free(&mut self) {
        let cfg = self.cfg;
        if !self.c.is_null() {
            (self.api_c.hmfree_func)(self.c.sub(cfg.elemsize) as *mut c_void, cfg.elemsize);
            (self.api_r.hmfree_func)(self.r.sub(cfg.elemsize) as *mut c_void, cfg.elemsize);
        }
        self.c = std::ptr::null_mut();
        self.r = std::ptr::null_mut();
    }
}

// ---------------------------------------------------------------------------
// stdout capture (for `helxo`)
// ---------------------------------------------------------------------------

extern "C" {
    fn dup(fd: c_int) -> c_int;
    fn dup2(old: c_int, new: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn fflush(f: *mut c_void) -> c_int;
}

pub unsafe fn capture_stdout<F: FnOnce()>(tag: &str, f: F) -> Vec<u8> {
    let path = std::env::temp_dir().join(format!(
        "harvest_capture_{}_{}.txt",
        std::process::id(),
        tag
    ));
    let file = std::fs::File::create(&path).unwrap();
    let fd = {
        use std::os::unix::io::AsRawFd;
        file.as_raw_fd()
    };
    fflush(std::ptr::null_mut());
    let saved = dup(1);
    assert!(saved >= 0);
    assert!(dup2(fd, 1) >= 0);
    f();
    fflush(std::ptr::null_mut());
    assert!(dup2(saved, 1) >= 0);
    close(saved);
    drop(file);
    let out = std::fs::read(&path).unwrap();
    let _ = std::fs::remove_file(&path);
    out
}
