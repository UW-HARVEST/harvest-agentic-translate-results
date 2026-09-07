//! Shared differential-test harness.
//!
//! Both the C `.so` and the Rust `.so` are loaded with `libloading` and driven
//! ONLY through their exported `flip_horizontal` symbol. The Rust functions are
//! never called directly, so the `#[no_mangle] extern "C"` wrapper and the
//! `#[repr(C)]` struct layout are part of what is under test.

#![allow(dead_code)]

use std::path::{Path, PathBuf};

pub const GUARD_PIXELS: usize = 8;
pub const GUARD_BYTE: u8 = 0xA5;

/// Mirror of the C `cp_image_t` as the *caller* sees it. Declared here
/// independently of the crate under test so a layout mistake in the crate
/// cannot hide itself.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct CpImage {
    pub w: i32,
    pub h: i32,
    pub pix: *mut u8,
}

pub type FlipFn = unsafe extern "C" fn(*mut CpImage);

/// The two implementations, kept alive for the duration of the test.
pub struct Impls {
    _c_lib: libloading::Library,
    _rust_lib: libloading::Library,
    pub c: FlipFn,
    pub rust: FlipFn,
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// `c_src/build/*.so` — the CMake project names the library after the parent
/// directory, so the exact file name is discovered rather than hard-coded.
pub fn c_so_path() -> PathBuf {
    let build_dir = manifest_dir().join("../c_src/build");
    let mut found: Vec<PathBuf> = Vec::new();
    if let Ok(entries) = std::fs::read_dir(&build_dir) {
        for e in entries.flatten() {
            let p = e.path();
            if p.extension().and_then(|s| s.to_str()) == Some("so") {
                found.push(p);
            }
        }
    }
    found.sort();
    assert_eq!(
        found.len(),
        1,
        "expected exactly one .so in {}; found {:?}. Build the C library first:\n  \
         cd c_src && mkdir -p build && cd build && \
         cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
        build_dir.display(),
        found
    );
    found.pop().unwrap()
}

/// The Rust artifact under test.
///
/// Defaults to `target/release/libflip_horizontal_lib.so` — the *shipped*
/// cdylib, built by the project's documented `cargo build --release`, and the
/// exact file an external `dlopen` consumer would load. It is not the test
/// binary's own profile directory on purpose: a `debug` build enables Rust's
/// "unsafe precondition" / null-dereference UB checks, which deliberately abort
/// (SIGABRT) on inputs where the C has undefined behavior and merely faults
/// (SIGSEGV). Those checks are a Rust debugging feature, not translated
/// behavior, so error-path parity is asserted against the release artifact.
///
/// Override with `HARVEST_RUST_SO=<path>` to point at any other build (used to
/// re-run the valid-path phase against the debug artifact).
pub fn rust_so_path() -> PathBuf {
    let name = "libflip_horizontal_lib.so";

    if let Ok(p) = std::env::var("HARVEST_RUST_SO") {
        let p = PathBuf::from(p);
        assert!(p.exists(), "HARVEST_RUST_SO={} does not exist", p.display());
        return p;
    }

    let release = manifest_dir().join("target/release").join(name);
    assert!(
        release.exists(),
        "{} not found. Build the shipped artifact first:\n  \
         cd translation && cargo build --release",
        release.display()
    );

    // Staleness guard: a stale `.so` would silently verify old code.
    if let (Ok(so), Ok(src)) = (
        std::fs::metadata(&release).and_then(|m| m.modified()),
        std::fs::metadata(manifest_dir().join("src/lib.rs")).and_then(|m| m.modified()),
    ) {
        assert!(
            so >= src,
            "{} is older than src/lib.rs — re-run `cargo build --release`",
            release.display()
        );
    }

    release
}

unsafe fn load(path: &Path) -> (libloading::Library, FlipFn) {
    let lib = libloading::Library::new(path)
        .unwrap_or_else(|e| panic!("dlopen {} failed: {e}", path.display()));
    let f: libloading::Symbol<FlipFn> = lib
        .get(b"flip_horizontal\0")
        .unwrap_or_else(|e| panic!("dlsym flip_horizontal in {} failed: {e}", path.display()));
    let raw: FlipFn = *f;
    (lib, raw)
}

pub fn load_impls() -> Impls {
    unsafe {
        let (c_lib, c) = load(&c_so_path());
        let (rust_lib, rust) = load(&rust_so_path());
        Impls {
            _c_lib: c_lib,
            _rust_lib: rust_lib,
            c,
            rust,
        }
    }
}

/// xorshift64* — deterministic, seeded, no external dependency.
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
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
    pub fn next_u8(&mut self) -> u8 {
        (self.next_u64() >> 33) as u8
    }
    /// Uniform-ish in `0..n` (n > 0).
    pub fn below(&mut self, n: u64) -> u64 {
        self.next_u64() % n
    }
    /// Inclusive range over signed values.
    pub fn in_range_i32(&mut self, lo: i32, hi: i32) -> i32 {
        let span = (hi as i64 - lo as i64 + 1) as u64;
        (lo as i64 + self.below(span) as i64) as i32
    }
    pub fn fill(&mut self, buf: &mut [u8]) {
        for b in buf.iter_mut() {
            *b = self.next_u8();
        }
    }
}

pub const SEED: u64 = 0x2545_F491_4F6C_DD1D;

/// A pixel buffer sized `4 * pixels` bytes followed by a guard region that
/// neither implementation is allowed to touch.
#[derive(Clone)]
pub struct Buffer {
    pub bytes: Vec<u8>,
    pub payload_len: usize,
}

impl Buffer {
    pub fn new(pixels: usize) -> Self {
        let payload_len = pixels * 4;
        let mut bytes = vec![0u8; payload_len + GUARD_PIXELS * 4];
        for b in bytes[payload_len..].iter_mut() {
            *b = GUARD_BYTE;
        }
        Buffer {
            bytes,
            payload_len,
        }
    }

    pub fn randomized(pixels: usize, rng: &mut Rng) -> Self {
        let mut b = Buffer::new(pixels);
        let n = b.payload_len;
        rng.fill(&mut b.bytes[..n]);
        b
    }

    pub fn filled(pixels: usize, byte: u8) -> Self {
        let mut b = Buffer::new(pixels);
        let n = b.payload_len;
        for x in b.bytes[..n].iter_mut() {
            *x = byte;
        }
        b
    }

    pub fn guard_ok(&self) -> bool {
        self.bytes[self.payload_len..].iter().all(|b| *b == GUARD_BYTE)
    }
}

/// Result of one differential invocation.
pub struct RunResult {
    pub c_bytes: Vec<u8>,
    pub rust_bytes: Vec<u8>,
    pub c_struct: [u8; std::mem::size_of::<CpImage>()],
    pub rust_struct: [u8; std::mem::size_of::<CpImage>()],
}

fn struct_bytes(img: &CpImage) -> [u8; std::mem::size_of::<CpImage>()] {
    let mut out = [0u8; std::mem::size_of::<CpImage>()];
    unsafe {
        std::ptr::copy_nonoverlapping(
            img as *const CpImage as *const u8,
            out.as_mut_ptr(),
            out.len(),
        );
    }
    out
}

/// Run both implementations on independent copies of `template` and return the
/// resulting bytes plus the (possibly mutated) `cp_image_t` images.
pub fn run_both(impls: &Impls, w: i32, h: i32, template: &Buffer) -> RunResult {
    let mut c_buf = template.clone();
    let mut rust_buf = template.clone();

    let mut c_img = CpImage {
        w,
        h,
        pix: c_buf.bytes.as_mut_ptr(),
    };
    let mut rust_img = CpImage {
        w,
        h,
        pix: rust_buf.bytes.as_mut_ptr(),
    };

    unsafe {
        (impls.c)(&mut c_img);
        (impls.rust)(&mut rust_img);
    }

    RunResult {
        c_bytes: c_buf.bytes,
        rust_bytes: rust_buf.bytes,
        c_struct: struct_bytes(&c_img),
        rust_struct: struct_bytes(&rust_img),
    }
}

fn first_diff(a: &[u8], b: &[u8]) -> Option<usize> {
    a.iter().zip(b.iter()).position(|(x, y)| x != y)
}

/// Assert byte-for-byte agreement of the pixel buffer, the guard region and the
/// `cp_image_t` struct itself. `label` identifies the CONFIGS.md row.
#[track_caller]
pub fn assert_same(impls: &Impls, w: i32, h: i32, template: &Buffer, label: &str) {
    let r = run_both(impls, w, h, template);

    assert_eq!(
        r.c_bytes.len(),
        r.rust_bytes.len(),
        "{label}: buffer length mismatch"
    );

    if r.c_bytes != r.rust_bytes {
        let idx = first_diff(&r.c_bytes, &r.rust_bytes).unwrap();
        panic!(
            "{label}: w={w} h={h}: buffer diverges at byte {idx} \
             (pixel {}, channel {}): C=0x{:02x} Rust=0x{:02x}\n\
             C   : {:02x?}\nRust: {:02x?}",
            idx / 4,
            idx % 4,
            r.c_bytes[idx],
            r.rust_bytes[idx],
            &r.c_bytes[idx.saturating_sub(8)..(idx + 8).min(r.c_bytes.len())],
            &r.rust_bytes[idx.saturating_sub(8)..(idx + 8).min(r.rust_bytes.len())],
        );
    }

    // Row 21/22: neither implementation may write past `w * h` pixels.
    let payload = template.payload_len;
    assert!(
        r.c_bytes[payload..].iter().all(|b| *b == GUARD_BYTE),
        "{label}: w={w} h={h}: C wrote into the guard region"
    );
    assert!(
        r.rust_bytes[payload..].iter().all(|b| *b == GUARD_BYTE),
        "{label}: w={w} h={h}: Rust wrote into the guard region"
    );

    // Row 23: `w`, `h` and `pix` must be left alone by both.
    assert_eq!(
        &r.c_struct[..8],
        &r.rust_struct[..8],
        "{label}: w={w} h={h}: cp_image_t w/h fields diverge after the call"
    );
    let expect_wh = {
        let mut e = [0u8; 8];
        e[..4].copy_from_slice(&w.to_ne_bytes());
        e[4..].copy_from_slice(&h.to_ne_bytes());
        e
    };
    assert_eq!(
        &r.c_struct[..8],
        &expect_wh[..],
        "{label}: w={w} h={h}: C mutated the cp_image_t header"
    );
    assert_eq!(
        &r.rust_struct[..8],
        &expect_wh[..],
        "{label}: w={w} h={h}: Rust mutated the cp_image_t header"
    );
}

/// `assert_same` over `iters` randomized buffers for a fixed `w`/`h`.
#[track_caller]
pub fn assert_same_randomized(impls: &Impls, w: i32, h: i32, iters: usize, label: &str) {
    let pixels = pixel_count(w, h);
    let mut rng = Rng::new(SEED ^ ((w as u64) << 32) ^ (h as u64 as u32 as u64));
    for _ in 0..iters {
        let buf = Buffer::randomized(pixels, &mut rng);
        assert_same(impls, w, h, &buf, label);
    }
}

/// Pixels to allocate for a `w`x`h` image.
///
/// The C code only ever dereferences memory when `w >= 1 && h >= 2` (the inner
/// loop is gated on `j < w` and the outer on `i < h / 2`). For every other
/// shape it merely *forms* `pix + w*i` without loading or storing, so a
/// one-pixel allocation is sufficient and keeps `w = INT_MAX` testable without
/// an 8 GiB allocation.
pub fn pixel_count(w: i32, h: i32) -> usize {
    if w >= 1 && h >= 2 {
        let n = (w as usize) * (h as usize);
        assert!(
            n <= 1 << 24,
            "test would allocate {n} pixels for w={w} h={h}; \
             pick a shape that fits or prove no memory is touched"
        );
        n
    } else {
        1
    }
}
