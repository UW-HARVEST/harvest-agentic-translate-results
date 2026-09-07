//! Shared harness: loads BOTH shared objects via `libloading` and calls every
//! function through its exported C symbol. Nothing in the Rust crate is ever
//! called directly, so the `#[no_mangle]` wrappers are under test too.

use libloading::{Library, Symbol};
use std::path::PathBuf;

pub const TFLAC_SIZE: usize = 28;

/// Raw byte image of `struct tflac` (size 28, align 4).
#[repr(C, align(4))]
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Tflac(pub [u8; TFLAC_SIZE]);

impl Tflac {
    pub fn zeroed() -> Self {
        Tflac([0u8; TFLAC_SIZE])
    }

    fn u32_at(&self, off: usize) -> u32 {
        u32::from_ne_bytes(self.0[off..off + 4].try_into().unwrap())
    }
    fn set_u32(&mut self, off: usize, v: u32) {
        self.0[off..off + 4].copy_from_slice(&v.to_ne_bytes());
    }

    pub fn blocksize(&self) -> u32 {
        self.u32_at(0)
    }
    pub fn samplerate(&self) -> u32 {
        self.u32_at(4)
    }
    pub fn channels(&self) -> u32 {
        self.u32_at(8)
    }
    pub fn bitdepth(&self) -> u32 {
        self.u32_at(12)
    }
    pub fn channel_mode(&self) -> u8 {
        self.0[16]
    }
    pub fn max_rice_value(&self) -> u8 {
        self.0[17]
    }
    pub fn min_partition_order(&self) -> u8 {
        self.0[18]
    }
    pub fn max_partition_order(&self) -> u8 {
        self.0[19]
    }
    pub fn partition_order(&self) -> u8 {
        self.0[20]
    }
    pub fn cur_blocksize(&self) -> u32 {
        self.u32_at(24)
    }

    pub fn set_blocksize(&mut self, v: u32) -> &mut Self {
        self.set_u32(0, v);
        self
    }
    pub fn set_samplerate(&mut self, v: u32) -> &mut Self {
        self.set_u32(4, v);
        self
    }
    pub fn set_channels(&mut self, v: u32) -> &mut Self {
        self.set_u32(8, v);
        self
    }
    pub fn set_bitdepth(&mut self, v: u32) -> &mut Self {
        self.set_u32(12, v);
        self
    }
    pub fn set_channel_mode(&mut self, v: u8) -> &mut Self {
        self.0[16] = v;
        self
    }
    pub fn set_max_rice_value(&mut self, v: u8) -> &mut Self {
        self.0[17] = v;
        self
    }
    pub fn set_min_partition_order(&mut self, v: u8) -> &mut Self {
        self.0[18] = v;
        self
    }
    pub fn set_max_partition_order(&mut self, v: u8) -> &mut Self {
        self.0[19] = v;
        self
    }
    pub fn set_partition_order(&mut self, v: u8) -> &mut Self {
        self.0[20] = v;
        self
    }
    pub fn set_cur_blocksize(&mut self, v: u32) -> &mut Self {
        self.set_u32(24, v);
        self
    }

    /// A configuration that passes every validation check.
    pub fn valid() -> Self {
        let mut t = Tflac::zeroed();
        t.set_blocksize(4096)
            .set_samplerate(44100)
            .set_channels(2)
            .set_bitdepth(16)
            .set_channel_mode(0)
            .set_max_rice_value(0)
            .set_min_partition_order(0)
            .set_max_partition_order(0);
        t
    }
}

impl std::fmt::Debug for Tflac {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "tflac{{blocksize:{}, samplerate:{}, channels:{}, bitdepth:{}, \
             channel_mode:{}, max_rice_value:{}, min_po:{}, max_po:{}, \
             partition_order:{}, cur_blocksize:{}, bytes:{:02x?}}}",
            self.blocksize(),
            self.samplerate(),
            self.channels(),
            self.bitdepth(),
            self.channel_mode(),
            self.max_rice_value(),
            self.min_partition_order(),
            self.max_partition_order(),
            self.partition_order(),
            self.cur_blocksize(),
            self.0
        )
    }
}

type FnSizeMemory = unsafe extern "C" fn(u32) -> u32;
type FnValidate = unsafe extern "C" fn(*mut Tflac) -> std::ffi::c_int;

/// One loaded implementation (either the C `.so` or the Rust `.so`).
pub struct Impl {
    pub name: &'static str,
    _lib: Library,
    size_memory: FnSizeMemory,
    validate: FnValidate,
}

impl Impl {
    fn load(name: &'static str, path: &PathBuf) -> Impl {
        let lib = unsafe {
            Library::new(path).unwrap_or_else(|e| panic!("failed to load {}: {e}", path.display()))
        };
        let size_memory: FnSizeMemory = unsafe {
            let s: Symbol<FnSizeMemory> = lib
                .get(b"tflac_size_memory\0")
                .unwrap_or_else(|e| panic!("{name}: missing symbol tflac_size_memory: {e}"));
            *s
        };
        let validate: FnValidate = unsafe {
            let s: Symbol<FnValidate> = lib
                .get(b"flac_validate\0")
                .unwrap_or_else(|e| panic!("{name}: missing symbol flac_validate: {e}"));
            *s
        };
        Impl {
            name,
            _lib: lib,
            size_memory,
            validate,
        }
    }

    pub fn size_memory(&self, blocksize: u32) -> u32 {
        unsafe { (self.size_memory)(blocksize) }
    }

    /// Runs `flac_validate` on a copy of `input`, returning `(rc, struct_after)`.
    pub fn validate(&self, input: &Tflac) -> (i32, Tflac) {
        let mut t = *input;
        let rc = unsafe { (self.validate)(&mut t as *mut Tflac) };
        (rc as i32, t)
    }

    /// Directly calls with a raw pointer (used for the null-pointer case).
    pub unsafe fn validate_raw(&self, p: *mut Tflac) -> i32 {
        unsafe { (self.validate)(p) as i32 }
    }
}

fn workspace_root() -> PathBuf {
    // CARGO_MANIFEST_DIR == <root>/translation
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("manifest dir has a parent")
        .to_path_buf()
}

fn find_c_so() -> PathBuf {
    let build = workspace_root().join("c_src/build");
    let mut candidates: Vec<PathBuf> = std::fs::read_dir(&build)
        .unwrap_or_else(|e| {
            panic!(
                "cannot read {} ({e}); build the C library first:\n  \
                 cd c_src && mkdir -p build && cd build && \
                 cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
                build.display()
            )
        })
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().map(|x| x == "so").unwrap_or(false))
        .collect();
    candidates.sort();
    candidates
        .pop()
        .unwrap_or_else(|| panic!("no .so found in {}", build.display()))
}

fn find_rust_so() -> PathBuf {
    // `cargo test` does NOT rebuild a `crate-type = ["cdylib"]` lib target, so
    // the .so on disk can easily be stale. Locate it, then hard-fail if it is
    // older than the sources — a stale .so would make every differential test
    // vacuously pass.
    let mut roots: Vec<PathBuf> = Vec::new();
    if let Ok(exe) = std::env::current_exe() {
        if let Some(deps) = exe.parent() {
            if let Some(profile) = deps.parent() {
                roots.push(profile.to_path_buf());
            }
        }
    }
    let target = workspace_root().join("translation/target");
    roots.push(target.join("release"));
    roots.push(target.join("debug"));

    let so = roots
        .iter()
        .map(|r| r.join("libflac_validate_lib.so"))
        .find(|p| p.is_file())
        .unwrap_or_else(|| {
            panic!(
                "libflac_validate_lib.so not found; run `cargo build --release` \
                 (searched: {roots:?})"
            )
        });

    assert_so_fresh(&so);
    so
}

/// Panics if `so` is older than any crate source file.
fn assert_so_fresh(so: &std::path::Path) {
    fn mtime(p: &std::path::Path) -> Option<std::time::SystemTime> {
        std::fs::metadata(p).ok()?.modified().ok()
    }
    fn newest_rs(dir: &std::path::Path, out: &mut Option<(std::time::SystemTime, PathBuf)>) {
        let Ok(rd) = std::fs::read_dir(dir) else { return };
        for e in rd.filter_map(|e| e.ok()) {
            let p = e.path();
            if p.is_dir() {
                newest_rs(&p, out);
            } else if p.extension().map(|x| x == "rs").unwrap_or(false) {
                if let Some(t) = mtime(&p) {
                    if out.as_ref().map(|(bt, _)| t > *bt).unwrap_or(true) {
                        *out = Some((t, p));
                    }
                }
            }
        }
    }

    let so_t = mtime(so).expect("stat .so");
    let crate_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let mut newest: Option<(std::time::SystemTime, PathBuf)> = None;
    newest_rs(&crate_root.join("src"), &mut newest);
    if let Some(t) = mtime(&crate_root.join("Cargo.toml")) {
        if newest.as_ref().map(|(bt, _)| t > *bt).unwrap_or(true) {
            newest = Some((t, crate_root.join("Cargo.toml")));
        }
    }

    if let Some((src_t, src_p)) = newest {
        assert!(
            so_t >= src_t,
            "STALE Rust .so: {} is older than {}.\n\
             `cargo test` does not rebuild a cdylib-only lib target, so the \
             differential tests would be comparing the C library against an \
             out-of-date Rust build. Run:\n    cargo build --release && cargo test",
            so.display(),
            src_p.display()
        );
    }
}

pub struct Pair {
    pub c: Impl,
    pub rs: Impl,
}

/// Loads both shared objects. Called once per test (dlopen is cheap and cached
/// by the loader, and each `Pair` keeps its own handles alive).
pub fn load_pair() -> Pair {
    Pair {
        c: Impl::load("C", &find_c_so()),
        rs: Impl::load("Rust", &find_rust_so()),
    }
}

impl Pair {
    /// Differential check of `tflac_size_memory`.
    pub fn cmp_size_memory(&self, blocksize: u32) {
        let a = self.c.size_memory(blocksize);
        let b = self.rs.size_memory(blocksize);
        assert_eq!(
            a, b,
            "tflac_size_memory({blocksize}) [0x{blocksize:08x}]: C={a} Rust={b}"
        );
    }

    /// Differential check of `flac_validate`: return code AND all 28 bytes.
    pub fn cmp_validate(&self, input: &Tflac) {
        let (rc_c, out_c) = self.c.validate(input);
        let (rc_rs, out_rs) = self.rs.validate(input);
        assert_eq!(
            rc_c, rc_rs,
            "flac_validate return code mismatch\n  input: {input:?}\n  C rc={rc_c} Rust rc={rc_rs}"
        );
        assert_eq!(
            out_c.0, out_rs.0,
            "flac_validate struct mismatch (rc={rc_c})\n  input: {input:?}\n  C   : {out_c:?}\n  Rust: {out_rs:?}"
        );
    }
}

/// Deterministic xorshift64* PRNG — fixed seed, reproducible across runs.
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
    pub fn next_u8(&mut self) -> u8 {
        (self.next_u64() >> 56) as u8
    }
    /// Uniform-ish in `lo..=hi`.
    pub fn range_u32(&mut self, lo: u32, hi: u32) -> u32 {
        debug_assert!(lo <= hi);
        let span = (hi - lo) as u64 + 1;
        lo + (self.next_u64() % span) as u32
    }
    pub fn range_u8(&mut self, lo: u8, hi: u8) -> u8 {
        self.range_u32(lo as u32, hi as u32) as u8
    }
    pub fn fill(&mut self, buf: &mut [u8]) {
        for b in buf.iter_mut() {
            *b = self.next_u8();
        }
    }
}
