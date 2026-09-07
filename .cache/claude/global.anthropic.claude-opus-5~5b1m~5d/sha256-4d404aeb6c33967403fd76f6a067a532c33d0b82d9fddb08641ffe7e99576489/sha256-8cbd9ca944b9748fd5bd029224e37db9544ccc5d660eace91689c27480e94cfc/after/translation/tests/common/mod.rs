//! Shared harness: loads BOTH the C `.so` and the Rust `.so` through
//! `libloading` and exposes one struct per library.  No Rust function is ever
//! called directly — every call goes through the dynamic symbol, so the
//! `#[no_mangle]` export wrappers are under test too.

#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::ffi::c_int;
use std::path::PathBuf;

pub mod oracle;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct DataBlock {
    pub id: c_int,
    pub name: [u8; 32],
    pub flags: u8,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct MemoryBlock {
    pub data: *mut c_int,
    pub size: usize,
}

pub type FnCreateBlock = unsafe extern "C" fn(c_int, *const u8, u8) -> DataBlock;
pub type FnAllocateBlock = unsafe extern "C" fn(usize, c_int) -> *mut MemoryBlock;
pub type FnFreeBlock = unsafe extern "C" fn(*mut MemoryBlock);
pub type FnComputeHash = unsafe extern "C" fn(*mut MemoryBlock, *mut MemoryBlock) -> c_int;
pub type FnBetagamma = unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int;

pub struct Lib {
    pub name: &'static str,
    _lib: Library,
    pub create_block: FnCreateBlock,
    pub allocate_block: FnAllocateBlock,
    pub free_block: FnFreeBlock,
    pub compute_hash: FnComputeHash,
    pub betagamma: FnBetagamma,
}

impl Lib {
    unsafe fn open(name: &'static str, path: &PathBuf) -> Lib {
        let lib = Library::new(path).unwrap_or_else(|e| panic!("dlopen {:?}: {}", path, e));
        macro_rules! sym {
            ($t:ty, $s:literal) => {{
                let s: Symbol<$t> = lib
                    .get($s)
                    .unwrap_or_else(|e| panic!("{:?} missing symbol {:?}: {}", path, $s, e));
                *s
            }};
        }
        let create_block = sym!(FnCreateBlock, b"create_block\0");
        let allocate_block = sym!(FnAllocateBlock, b"allocate_block\0");
        let free_block = sym!(FnFreeBlock, b"free_block\0");
        let compute_hash = sym!(FnComputeHash, b"compute_hash\0");
        let betagamma = sym!(FnBetagamma, b"betagamma\0");
        Lib {
            name,
            _lib: lib,
            create_block,
            allocate_block,
            free_block,
            compute_hash,
            betagamma,
        }
    }
}

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("crate has a parent dir")
        .to_path_buf()
}

fn find_c_so() -> PathBuf {
    let build = workspace_root().join("c_src").join("build");
    let mut found: Vec<PathBuf> = Vec::new();
    if let Ok(rd) = std::fs::read_dir(&build) {
        for e in rd.flatten() {
            let p = e.path();
            if p.extension().map(|x| x == "so").unwrap_or(false) {
                found.push(p);
            }
        }
    }
    found.sort();
    found.pop().unwrap_or_else(|| {
        panic!(
            "no C .so under {:?} — build it with:\n  cd c_src && mkdir -p build && cd build && \
             cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
            build
        )
    })
}

fn find_rust_so() -> PathBuf {
    // The test binary lives in target/<profile>/deps/, so the cdylib built by
    // this same `cargo test` invocation is one directory up.
    let exe = std::env::current_exe().expect("current_exe");
    let deps = exe.parent().expect("deps dir");
    let profile = deps.parent().expect("profile dir");
    for dir in [profile, deps] {
        let p = dir.join("libbetagamma_lib.so");
        if p.is_file() {
            return p;
        }
    }
    // Fallbacks for out-of-band invocations.
    for prof in ["debug", "release"] {
        let p = workspace_root()
            .join("translation/target")
            .join(prof)
            .join("libbetagamma_lib.so");
        if p.is_file() {
            return p;
        }
    }
    panic!("libbetagamma_lib.so not found near {:?}", profile);
}

pub struct Pair {
    pub c: Lib,
    pub rs: Lib,
}

/// Open exactly ONE of the two libraries.
///
/// Needed by the out-of-process oracle: `betagamma` / `compute_hash` observe the
/// *relative addresses* returned by the platform allocator, so a fair
/// comparison has to give each library an identical, pristine heap history.
/// Loading both into one process makes the second one reuse the first one's
/// freed tcache chunks (LIFO), which changes the pointer ordering for reasons
/// that have nothing to do with the translation.
pub fn open_one(which: &str) -> Lib {
    unsafe {
        match which {
            "c" => Lib::open("C", &find_c_so()),
            "rust" => Lib::open("Rust", &find_rust_so()),
            other => panic!("unknown library role {:?}", other),
        }
    }
}

/// Both libraries, loaded once per test process.
pub fn libs() -> &'static Pair {
    use std::sync::OnceLock;
    static P: OnceLock<Pair> = OnceLock::new();
    P.get_or_init(|| unsafe {
        Pair {
            c: Lib::open("C", &find_c_so()),
            rs: Lib::open("Rust", &find_rust_so()),
        }
    })
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (xoshiro256**), so every property-style row is
// reproducible from a fixed seed without pulling in a dependency.
// ---------------------------------------------------------------------------
pub struct Rng(pub [u64; 4]);

impl Rng {
    pub fn new(seed: u64) -> Rng {
        // SplitMix64 expansion of the seed.
        let mut s = seed;
        let mut st = [0u64; 4];
        for slot in st.iter_mut() {
            s = s.wrapping_add(0x9E3779B97F4A7C15);
            let mut z = s;
            z = (z ^ (z >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
            z = (z ^ (z >> 27)).wrapping_mul(0x94D049BB133111EB);
            *slot = z ^ (z >> 31);
        }
        Rng(st)
    }

    pub fn next_u64(&mut self) -> u64 {
        let s = &mut self.0;
        let result = s[1].wrapping_mul(5).rotate_left(7).wrapping_mul(9);
        let t = s[1] << 17;
        s[2] ^= s[0];
        s[3] ^= s[1];
        s[1] ^= s[2];
        s[0] ^= s[3];
        s[2] ^= t;
        s[3] = s[3].rotate_left(45);
        result
    }

    pub fn i32(&mut self) -> i32 {
        self.next_u64() as u32 as i32
    }

    pub fn u8(&mut self) -> u8 {
        (self.next_u64() >> 24) as u8
    }

    /// Uniform in `lo..=hi`.
    pub fn range_i32(&mut self, lo: i32, hi: i32) -> i32 {
        let span = (hi as i64 - lo as i64 + 1) as u64;
        (lo as i64 + (self.next_u64() % span) as i64) as i32
    }

    pub fn range_usize(&mut self, lo: usize, hi: usize) -> usize {
        lo + (self.next_u64() % ((hi - lo + 1) as u64)) as usize
    }
}

// ---------------------------------------------------------------------------
// Comparison helpers
// ---------------------------------------------------------------------------

/// Compare the defined part of a returned `DataBlock`.
///
/// `name` is filled by `strcpy`, so only bytes `0..=strlen(src)` are defined;
/// the tail of the array and the 3 trailing padding bytes of the struct are
/// indeterminate in BOTH implementations and must not be compared.
pub fn assert_datablock_eq(ctx: &str, c: &DataBlock, r: &DataBlock, defined_name_len: usize) {
    assert_eq!(c.id, r.id, "{}: DataBlock.id", ctx);
    assert_eq!(c.flags, r.flags, "{}: DataBlock.flags", ctx);
    let n = defined_name_len.min(32);
    assert_eq!(
        &c.name[..n],
        &r.name[..n],
        "{}: DataBlock.name[..{}]\n  C   = {:?}\n  Rust= {:?}",
        ctx,
        n,
        &c.name[..n],
        &r.name[..n]
    );
}

/// Read back a `MemoryBlock` produced by `allocate_block` into an owned,
/// address-independent snapshot: `(size, elements, data_is_null)`.
pub unsafe fn snapshot(mb: *mut MemoryBlock) -> Option<(usize, Vec<c_int>, bool)> {
    if mb.is_null() {
        return None;
    }
    let size = (*mb).size;
    let data = (*mb).data;
    let mut v = Vec::with_capacity(size.min(1 << 22));
    if !data.is_null() {
        for i in 0..size {
            v.push(*data.add(i));
        }
    }
    Some((size, v, data.is_null()))
}

/// Run `allocate_block` on both libs with the same arguments, compare the
/// observable state, and free both.  Returns the shared snapshot.
pub fn diff_allocate(ctx: &str, count: usize, init: c_int) -> Option<(usize, Vec<c_int>, bool)> {
    let l = libs();
    unsafe {
        let cm = (l.c.allocate_block)(count, init);
        let rm = (l.rs.allocate_block)(count, init);

        let cs = snapshot(cm);
        let rs = snapshot(rm);

        match (&cs, &rs) {
            (None, None) => {}
            (Some(a), Some(b)) => {
                assert_eq!(a.0, b.0, "{}: MemoryBlock.size", ctx);
                assert_eq!(a.2, b.2, "{}: MemoryBlock.data null-ness", ctx);
                assert!(
                    a.1 == b.1,
                    "{}: element mismatch (count={}, init={})\n  first diff at {:?}",
                    ctx,
                    count,
                    init,
                    a.1.iter().zip(b.1.iter()).position(|(x, y)| x != y)
                );
            }
            _ => panic!(
                "{}: NULL-ness mismatch: C={:?} Rust={:?} (count={}, init={})",
                ctx,
                cs.is_some(),
                rs.is_some(),
                count,
                init
            ),
        }

        (l.c.free_block)(cm);
        (l.rs.free_block)(rm);
        cs
    }
}

/// `compute_hash` differential over caller-owned `MemoryBlock`s, so the
/// pointer ordering is fully controlled by the test (not by the allocator).
pub fn diff_compute_hash(
    ctx: &str,
    a: *mut MemoryBlock,
    b: *mut MemoryBlock,
    expected: Option<c_int>,
) -> c_int {
    let l = libs();
    unsafe {
        let hc = (l.c.compute_hash)(a, b);
        let hr = (l.rs.compute_hash)(a, b);
        assert_eq!(hc, hr, "{}: compute_hash C={} Rust={}", ctx, hc, hr);
        if let Some(e) = expected {
            assert_eq!(hc, e, "{}: compute_hash expected {}, got {}", ctx, e, hc);
        }
        hc
    }
}

pub fn diff_betagamma(p: (c_int, c_int, c_int, c_int)) -> c_int {
    let l = libs();
    unsafe {
        let c = (l.c.betagamma)(p.0, p.1, p.2, p.3);
        let r = (l.rs.betagamma)(p.0, p.1, p.2, p.3);
        assert_eq!(
            c, r,
            "betagamma({}, {}, {}, {}): C={} Rust={}",
            p.0, p.1, p.2, p.3, c, r
        );
        c
    }
}

pub fn diff_create_block(ctx: &str, id: c_int, name: &[u8], flags: u8) {
    assert!(name.len() < 32, "test would overflow DataBlock.name");
    let l = libs();
    let mut z: Vec<u8> = name.to_vec();
    z.push(0);
    unsafe {
        let c = (l.c.create_block)(id, z.as_ptr(), flags);
        let r = (l.rs.create_block)(id, z.as_ptr(), flags);
        assert_datablock_eq(ctx, &c, &r, name.len() + 1);
    }
}
