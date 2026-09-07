//! Shared differential-test harness.
//!
//! Loads BOTH shared objects (the C one built by CMake and the Rust `cdylib`)
//! through `libloading` and exposes them behind identical function-pointer
//! types, so every comparison goes through the real FFI boundary and exercises
//! the `#[no_mangle]` export wrappers.

#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

/* ------------------------------------------------------------------ */
/* Raw C ABI signatures                                                */
/* ------------------------------------------------------------------ */

pub type FnPackU64le = unsafe extern "C" fn(*mut u8, u64);
pub type FnMd5AddSample = unsafe extern "C" fn(*mut u8, u32, u64);
pub type FnUpdateMd5 = unsafe extern "C" fn(*mut u8, *const i32) -> u32;

/// One loaded implementation (either the C `.so` or the Rust `.so`).
pub struct Impl {
    pub name: &'static str,
    pub path: PathBuf,
    _lib: Library,
    pub pack_u64le: FnPackU64le,
    pub md5_addsample: FnMd5AddSample,
    pub update_md5: FnUpdateMd5,
}

impl Impl {
    unsafe fn open(name: &'static str, path: PathBuf) -> Impl {
        let lib = Library::new(&path)
            .unwrap_or_else(|e| panic!("cannot dlopen {} ({}): {e}", path.display(), name));
        let pack: Symbol<FnPackU64le> = lib
            .get(b"tflac_pack_u64le\0")
            .unwrap_or_else(|e| panic!("{name}: missing symbol tflac_pack_u64le: {e}"));
        let add: Symbol<FnMd5AddSample> = lib
            .get(b"tflac_md5_addsample\0")
            .unwrap_or_else(|e| panic!("{name}: missing symbol tflac_md5_addsample: {e}"));
        let upd: Symbol<FnUpdateMd5> = lib
            .get(b"update_md5\0")
            .unwrap_or_else(|e| panic!("{name}: missing symbol update_md5: {e}"));
        let (pack_u64le, md5_addsample, update_md5) = (*pack, *add, *upd);
        Impl { name, path, _lib: lib, pack_u64le, md5_addsample, update_md5 }
    }
}

pub struct Pair {
    pub c: Impl,
    pub rs: Impl,
}

fn repo_root() -> PathBuf {
    // CARGO_MANIFEST_DIR == <root>/translation
    Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap().to_path_buf()
}

fn find_c_so() -> PathBuf {
    // Allows the same test suite to be re-run against a differently-optimised
    // build of the C library (see run_tests.sh).
    if let Ok(p) = std::env::var("C_SO_OVERRIDE") {
        let p = PathBuf::from(p);
        assert!(p.is_file(), "C_SO_OVERRIDE={} is not a file", p.display());
        return p;
    }
    let build = repo_root().join("c_src/build");
    let mut cands: Vec<PathBuf> = std::fs::read_dir(&build)
        .unwrap_or_else(|e| {
            panic!(
                "c_src/build not found ({e}). Build the C library first:\n  \
                 cd c_src && mkdir -p build && cd build && \
                 cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build ."
            )
        })
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().map(|x| x == "so").unwrap_or(false))
        .collect();
    cands.sort();
    assert_eq!(cands.len(), 1, "expected exactly one .so in c_src/build, got {cands:?}");
    cands.pop().unwrap()
}

fn find_rust_so() -> PathBuf {
    // `[lib] crate-type = ["cdylib"]` means `cargo test` does NOT rebuild the
    // shared object, so the newest of the two profile outputs is used and a
    // staleness check guards against testing an out-of-date artifact.
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    let base = manifest.join("target");
    let name = "libupdate_md5_lib.so";

    // Explicit selection wins, so the harness never silently tests the wrong
    // profile (run_tests.sh always sets this).
    let so = if let Ok(p) = std::env::var("RS_SO_OVERRIDE") {
        let p = PathBuf::from(p);
        assert!(p.is_file(), "RS_SO_OVERRIDE={} is not a file", p.display());
        p
    } else {
        // Default: prefer the optimised artifact (that is what ships).
        ["release", "debug"]
            .iter()
            .map(|prof| base.join(prof).join(name))
            .find(|p| p.is_file())
            .unwrap_or_else(|| {
                panic!(
                    "Rust cdylib not found under {}/{{release,debug}}/{name}.\n\
                     Run `cargo build --release` (or ./run_tests.sh) first.",
                    base.display()
                )
            })
    };
    let so_time = std::fs::metadata(&so).unwrap().modified().unwrap();
    let src_time = std::fs::metadata(manifest.join("src/lib.rs")).unwrap().modified().unwrap();
    assert!(
        so_time >= src_time,
        "STALE Rust cdylib: {} is older than src/lib.rs. Rebuild with \
         `cargo build --release` (or use ./run_tests.sh).",
        so.display()
    );
    so
}

/// True when the loaded Rust `.so` is an unoptimised (`debug_assertions`)
/// build. Such a build turns a null dereference into a controlled
/// `SIGABRT` panic instead of the `SIGSEGV` the C produces, which is a
/// debug-instrumentation artefact rather than an ABI difference.
pub fn rust_so_is_debug() -> bool {
    pair().rs.path.components().any(|c| c.as_os_str() == "debug")
}

pub fn pair() -> &'static Pair {
    static P: OnceLock<Pair> = OnceLock::new();
    P.get_or_init(|| unsafe {
        Pair { c: Impl::open("C", find_c_so()), rs: Impl::open("Rust", find_rust_so()) }
    })
}

/* ------------------------------------------------------------------ */
/* Byte-exact context mirrors                                          */
/* ------------------------------------------------------------------ */

pub const MD5_SIZE: usize = 88;
pub const TFLAC_SIZE: usize = 96;
pub const BUF_LEN: usize = 72;

/// Bytes of deterministic guard memory appended after each context image.
///
/// The C fold-down loop is `while (bytes--) m->buffer[bytes] = m->buffer[64 + bytes];`
/// with `bytes = m->pos % 64`, so it can read as far as `buffer[64 + 62] ==
/// buffer[126]` — i.e. **past** the 72-byte array — whenever `pos % 64 > 8`.
/// The library never produces such a `pos` itself, but a caller can pass one,
/// and both implementations perform the identical out-of-bounds load. To make
/// that load *deterministic and identical on both sides* (instead of reading
/// whatever happens to follow each object on the stack), every context is
/// embedded in a larger, identically pre-filled allocation. 192 bytes is more
/// than the worst case: `16 + 126 == 142`, i.e. 54 bytes past `tflac_md5`.
pub const GUARD: usize = 192;
pub const MD5_TOTAL: usize = MD5_SIZE + GUARD;
pub const TFLAC_TOTAL: usize = TFLAC_SIZE + GUARD;

/// Deterministic guard filler — same value at the same offset for both impls.
fn guard_byte(i: usize) -> u8 {
    ((i as u32).wrapping_mul(0x9E) ^ 0x7E ^ ((i as u32) >> 3)) as u8
}

/// A raw, 8-byte-aligned image of `struct tflac_md5` (88 bytes) plus guard.
///
/// Working with the raw bytes (rather than a `#[repr(C)]` struct literal)
/// means the 4 padding bytes at offset 4 are *explicitly* initialised, so a
/// byte-for-byte comparison is meaningful and will catch either side
/// clobbering padding. The trailing guard bytes catch out-of-bounds *writes*
/// and make the C's out-of-bounds *reads* reproducible.
#[repr(C, align(8))]
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Md5Ctx(pub [u8; MD5_TOTAL]);

impl Md5Ctx {
    /// `pad` fills the struct padding with a recognisable pattern.
    pub fn new(pos: u32, total: u64, buffer: &[u8; BUF_LEN]) -> Md5Ctx {
        let mut c = Md5Ctx([0u8; MD5_TOTAL]);
        for i in MD5_SIZE..MD5_TOTAL {
            c.0[i] = guard_byte(i);
        }
        c.0[0..4].copy_from_slice(&pos.to_ne_bytes());
        c.0[4..8].copy_from_slice(&[0x5Au8, 0xA5, 0x3C, 0xC3]); // padding sentinel
        c.0[8..16].copy_from_slice(&total.to_ne_bytes());
        c.0[16..88].copy_from_slice(buffer);
        c
    }
    /// True iff the trailing guard region is still pristine (no OOB write).
    pub fn guard_intact(&self) -> bool {
        (MD5_SIZE..MD5_TOTAL).all(|i| self.0[i] == guard_byte(i))
    }
    pub fn pos(&self) -> u32 {
        u32::from_ne_bytes(self.0[0..4].try_into().unwrap())
    }
    pub fn total(&self) -> u64 {
        u64::from_ne_bytes(self.0[8..16].try_into().unwrap())
    }
    pub fn buffer(&self) -> &[u8] {
        &self.0[16..88]
    }
    pub fn as_mut_ptr(&mut self) -> *mut u8 {
        self.0.as_mut_ptr()
    }
}

impl std::fmt::Debug for Md5Ctx {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Md5Ctx {{ pos: {}, total: {}, pad: {:02x?}, buffer: {:02x?}, guard_intact: {} }}",
            self.pos(),
            self.total(),
            &self.0[4..8],
            self.buffer(),
            self.guard_intact()
        )
    }
}

/// A raw, 8-byte-aligned image of `struct tflac` (96 bytes) plus guard.
#[repr(C, align(8))]
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct TflacCtx(pub [u8; TFLAC_TOTAL]);

impl TflacCtx {
    pub fn new(
        pos: u32,
        total: u64,
        buffer: &[u8; BUF_LEN],
        cur_blocksize: u32,
        channels: u32,
    ) -> TflacCtx {
        let mut t = TflacCtx([0u8; TFLAC_TOTAL]);
        for i in TFLAC_SIZE..TFLAC_TOTAL {
            t.0[i] = guard_byte(i);
        }
        let m = Md5Ctx::new(pos, total, buffer);
        t.0[0..MD5_SIZE].copy_from_slice(&m.0[0..MD5_SIZE]);
        t.0[88..92].copy_from_slice(&cur_blocksize.to_ne_bytes());
        t.0[92..96].copy_from_slice(&channels.to_ne_bytes());
        t
    }
    /// The embedded `tflac_md5` image, re-wrapped (guard re-synthesised).
    pub fn md5(&self) -> Md5Ctx {
        let mut m = Md5Ctx([0u8; MD5_TOTAL]);
        m.0[0..MD5_SIZE].copy_from_slice(&self.0[0..MD5_SIZE]);
        for i in MD5_SIZE..MD5_TOTAL {
            m.0[i] = guard_byte(i);
        }
        m
    }
    /// True iff the trailing guard region is still pristine (no OOB write).
    pub fn guard_intact(&self) -> bool {
        (TFLAC_SIZE..TFLAC_TOTAL).all(|i| self.0[i] == guard_byte(i))
    }
    pub fn cur_blocksize(&self) -> u32 {
        u32::from_ne_bytes(self.0[88..92].try_into().unwrap())
    }
    pub fn channels(&self) -> u32 {
        u32::from_ne_bytes(self.0[92..96].try_into().unwrap())
    }
    pub fn as_mut_ptr(&mut self) -> *mut u8 {
        self.0.as_mut_ptr()
    }
}

impl std::fmt::Debug for TflacCtx {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "TflacCtx {{ md5: {:?}, cur_blocksize: {}, channels: {}, guard_intact: {} }}",
            self.md5(),
            self.cur_blocksize(),
            self.channels(),
            self.guard_intact()
        )
    }
}

/// 8-byte-aligned 72-byte scratch buffer plus a 32-byte guard region, used for
/// the standalone `tflac_pack_u64le` tests.
#[repr(C, align(8))]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct PackBuf(pub [u8; BUF_LEN + 32]);

impl PackBuf {
    pub fn new(fill: &[u8]) -> PackBuf {
        let mut b = PackBuf([0u8; BUF_LEN + 32]);
        for (i, s) in b.0.iter_mut().enumerate() {
            *s = fill[i % fill.len()];
        }
        b
    }
    pub fn as_mut_ptr(&mut self) -> *mut u8 {
        self.0.as_mut_ptr()
    }
}

/* ------------------------------------------------------------------ */
/* Deterministic PRNG (xorshift64*) — fixed seed for reproducibility   */
/* ------------------------------------------------------------------ */

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng(if seed == 0 { 0x9E37_79B9_7F4A_7C15 } else { seed })
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
    pub fn next_i32(&mut self) -> i32 {
        self.next_u32() as i32
    }
    pub fn below(&mut self, n: u32) -> u32 {
        self.next_u32() % n
    }
    pub fn fill_buf(&mut self) -> [u8; BUF_LEN] {
        let mut b = [0u8; BUF_LEN];
        for chunk in b.chunks_mut(8) {
            let v = self.next_u64().to_ne_bytes();
            chunk.copy_from_slice(&v[..chunk.len()]);
        }
        b
    }
    /// Random i32 sample vector of `n` elements.
    pub fn samples(&mut self, n: usize) -> Vec<i32> {
        (0..n).map(|_| self.next_i32()).collect()
    }
}

/* ------------------------------------------------------------------ */
/* Differential drivers                                                */
/* ------------------------------------------------------------------ */

/// `update_md5` reads `samples[0..8]` five times at a stride of
/// `8 * sizeof(tflac_s32) == 32` *elements*, so the highest index touched is
/// `4 * 32 + 7 == 135`. A conforming caller must supply at least this many.
pub const UPDATE_MD5_MIN_SAMPLES: usize = 4 * 32 + 8; // 136

/// Run `tflac_pack_u64le` on both impls and assert byte-identical buffers.
#[track_caller]
pub fn diff_pack(ctx: &str, buf: PackBuf, offset: usize, n: u64) {
    let p = pair();
    let mut cb = buf;
    let mut rb = buf;
    unsafe {
        (p.c.pack_u64le)(cb.as_mut_ptr().add(offset), n);
        (p.rs.pack_u64le)(rb.as_mut_ptr().add(offset), n);
    }
    assert_eq!(
        cb, rb,
        "tflac_pack_u64le divergence [{ctx}] offset={offset} n={n:#018x}\n C: {:02x?}\nRS: {:02x?}",
        cb.0, rb.0
    );
}

/// Run `tflac_md5_addsample` on both impls and assert byte-identical contexts.
#[track_caller]
pub fn diff_addsample(ctx: &str, start: Md5Ctx, bits: u32, val: u64) -> Md5Ctx {
    let p = pair();
    let mut cm = start;
    let mut rm = start;
    unsafe {
        (p.c.md5_addsample)(cm.as_mut_ptr(), bits, val);
        (p.rs.md5_addsample)(rm.as_mut_ptr(), bits, val);
    }
    assert_eq!(
        cm, rm,
        "tflac_md5_addsample divergence [{ctx}] start={start:?} bits={bits} val={val:#018x}\n \
         C: {cm:?}\nRS: {rm:?}"
    );
    assert!(cm.guard_intact(), "[{ctx}] C wrote past struct tflac_md5");
    assert!(rm.guard_intact(), "[{ctx}] Rust wrote past struct tflac_md5");
    cm
}

/// Run `update_md5` on both impls and assert identical return values *and*
/// byte-identical `struct tflac` images.
#[track_caller]
pub fn diff_update(ctx: &str, start: TflacCtx, samples: &[i32]) -> (u32, TflacCtx) {
    assert!(
        samples.len() >= UPDATE_MD5_MIN_SAMPLES,
        "[{ctx}] update_md5 reads 136 samples; got {}",
        samples.len()
    );
    let p = pair();
    let mut ct = start;
    let mut rt = start;
    let (cr, rr) = unsafe {
        (
            (p.c.update_md5)(ct.as_mut_ptr(), samples.as_ptr()),
            (p.rs.update_md5)(rt.as_mut_ptr(), samples.as_ptr()),
        )
    };
    assert_eq!(
        cr, rr,
        "update_md5 return-value divergence [{ctx}] start={start:?} \
         C={cr} ({cr:#010x}) RS={rr} ({rr:#010x})"
    );
    assert_eq!(
        ct, rt,
        "update_md5 context divergence [{ctx}] start={start:?}\n C: {ct:?}\nRS: {rt:?}"
    );
    assert!(ct.guard_intact(), "[{ctx}] C wrote past struct tflac");
    assert!(rt.guard_intact(), "[{ctx}] Rust wrote past struct tflac");
    (cr, ct)
}
