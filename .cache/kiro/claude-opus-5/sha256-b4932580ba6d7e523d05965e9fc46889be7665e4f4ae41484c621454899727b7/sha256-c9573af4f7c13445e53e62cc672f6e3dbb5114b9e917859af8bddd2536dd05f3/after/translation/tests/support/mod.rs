//! Shared differential-test harness.
//!
//! Both the C library and the Rust library are loaded as shared objects with
//! `libloading` and driven exclusively through their exported `premultiply`
//! symbol. The Rust code is never called directly, so the `#[no_mangle]`
//! `extern "C"` wrapper and the `#[repr(C)]` struct layout are part of what is
//! under test.

#![allow(dead_code)]
#![allow(non_camel_case_types)]

use std::os::raw::c_int;
use std::path::{Path, PathBuf};

/// Mirrors `cp_pixel_t` from `c_src/include/lib.h`.
#[repr(C)]
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct CPixel {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

/// Mirrors `cp_image_t` from `c_src/include/lib.h`.
#[repr(C)]
pub struct CImage {
    pub w: c_int,
    pub h: c_int,
    pub pix: *mut CPixel,
}

pub type PremultiplyFn = unsafe extern "C" fn(*mut CImage);

/// A loaded shared object plus its `premultiply` entry point.
pub struct Lib {
    pub name: &'static str,
    pub path: PathBuf,
    // `library` must outlive `premultiply`; keep it last-dropped by ordering.
    premultiply: PremultiplyFn,
    _library: libloading::Library,
}

impl Lib {
    fn open(name: &'static str, path: PathBuf) -> Lib {
        let library = unsafe { libloading::Library::new(&path) }
            .unwrap_or_else(|e| panic!("failed to dlopen {} ({}): {e}", name, path.display()));
        let sym: libloading::Symbol<PremultiplyFn> = unsafe { library.get(b"premultiply\0") }
            .unwrap_or_else(|e| {
                panic!(
                    "symbol `premultiply` missing from {} ({}): {e}",
                    name,
                    path.display()
                )
            });
        let premultiply = *sym;
        Lib {
            name,
            path,
            premultiply,
            _library: library,
        }
    }

    /// Call the library's exported `premultiply` on `pixels` with the given
    /// (possibly nonsensical) `w`/`h`. `pixels` is passed through unchanged.
    pub unsafe fn call_raw(&self, w: c_int, h: c_int, pix: *mut CPixel) {
        let mut img = CImage { w, h, pix };
        (self.premultiply)(&mut img);
    }

    /// Convenience: run on a byte buffer viewed as pixels.
    pub fn call_bytes(&self, w: c_int, h: c_int, bytes: &mut [u8]) {
        assert_eq!(bytes.len() % 4, 0, "byte buffer must be a whole pixel count");
        let p = bytes.as_mut_ptr() as *mut CPixel;
        unsafe { self.call_raw(w, h, p) }
    }

    /// Call the exported symbol with a NULL `cp_image_t *`. This is expected to
    /// fault; it exists so the fault can be compared across both libraries in a
    /// child process.
    pub unsafe fn call_null_img(&self) {
        (self.premultiply)(std::ptr::null_mut());
    }
}

fn workspace_root() -> PathBuf {
    // CARGO_MANIFEST_DIR = <root>/translation
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("manifest dir has a parent")
        .to_path_buf()
}

fn find_c_so() -> PathBuf {
    let build = workspace_root().join("c_src").join("build");
    let mut candidates: Vec<PathBuf> = Vec::new();
    if let Ok(entries) = std::fs::read_dir(&build) {
        for e in entries.flatten() {
            let p = e.path();
            let name = p.file_name().and_then(|s| s.to_str()).unwrap_or("");
            if name.starts_with("lib") && name.ends_with(".so") {
                candidates.push(p);
            }
        }
    }
    candidates.sort();
    candidates.into_iter().next().unwrap_or_else(|| {
        panic!(
            "no C shared object found in {} — build it with:\n  \
             cd c_src && mkdir -p build && cd build && \
             cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
            build.display()
        )
    })
}

fn find_rust_so() -> PathBuf {
    // current_exe() = <target>/<profile>/deps/<test bin>
    let exe = std::env::current_exe().expect("current_exe");
    let deps = exe.parent().expect("deps dir");
    let profile_dir = deps.parent().expect("profile dir");

    let mut search: Vec<PathBuf> = vec![profile_dir.to_path_buf()];
    if let Some(target) = profile_dir.parent() {
        search.push(target.join("debug"));
        search.push(target.join("release"));
    }

    for dir in &search {
        let p = dir.join("libpremultiply_lib.so");
        if p.is_file() {
            assert_not_stale(&p);
            return p;
        }
    }
    panic!(
        "libpremultiply_lib.so not found; searched {:?}.\n\
         NOTE: `cargo test` does NOT build a `cdylib`-only library target, so the \
         shared object must be produced explicitly first:\n  cargo build [--release]",
        search
    );
}

/// `cargo test` will happily run against a shared object left over from an
/// earlier `cargo build`. That would silently verify stale code, so refuse to
/// run if the `.so` is older than any Rust source file in the crate.
fn assert_not_stale(so: &Path) {
    let so_mtime = match std::fs::metadata(so).and_then(|m| m.modified()) {
        Ok(t) => t,
        Err(_) => return,
    };
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut newest: Option<(PathBuf, std::time::SystemTime)> = None;
    let mut stack = vec![src];
    while let Some(dir) = stack.pop() {
        let entries = match std::fs::read_dir(&dir) {
            Ok(e) => e,
            Err(_) => continue,
        };
        for e in entries.flatten() {
            let p = e.path();
            if p.is_dir() {
                stack.push(p);
                continue;
            }
            if p.extension().and_then(|s| s.to_str()) != Some("rs") {
                continue;
            }
            if let Ok(t) = e.metadata().and_then(|m| m.modified()) {
                if newest.as_ref().map_or(true, |(_, n)| t > *n) {
                    newest = Some((p, t));
                }
            }
        }
    }
    if let Some((newest_path, t)) = newest {
        assert!(
            t <= so_mtime,
            "STALE ARTIFACT: {} is older than {}.\n\
             `cargo test` does not rebuild a cdylib-only target — run \
             `cargo build [--release]` (or ./run_matrix.sh) first.",
            so.display(),
            newest_path.display()
        );
    }
}

/// Load both shared objects.
pub fn load_pair() -> (Lib, Lib) {
    let c = Lib::open("C", find_c_so());
    let r = Lib::open("Rust", find_rust_so());
    (c, r)
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (xorshift64*) — no external crates, fixed seeds.
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
    pub fn next_u8(&mut self) -> u8 {
        (self.next_u64() >> 56) as u8
    }
    pub fn next_i32(&mut self) -> i32 {
        self.next_u32() as i32
    }
    /// Uniform in `lo..=hi`.
    pub fn range(&mut self, lo: u32, hi: u32) -> u32 {
        assert!(lo <= hi);
        let span = (hi - lo) as u64 + 1;
        lo + (self.next_u64() % span) as u32
    }
    pub fn fill_bytes(&mut self, buf: &mut [u8]) {
        for b in buf.iter_mut() {
            *b = self.next_u8();
        }
    }
    /// Byte drawn from a distribution biased toward rounding boundaries.
    pub fn boundary_byte(&mut self) -> u8 {
        const HOT: [u8; 8] = [0, 1, 2, 126, 127, 128, 254, 255];
        match self.range(0, 2) {
            0 | 1 => HOT[self.range(0, 7) as usize],
            _ => self.next_u8(),
        }
    }
    pub fn fill_boundary(&mut self, buf: &mut [u8]) {
        for b in buf.iter_mut() {
            *b = self.boundary_byte();
        }
    }
}

// ---------------------------------------------------------------------------
// Differential assertion helpers
// ---------------------------------------------------------------------------

fn first_diff(a: &[u8], b: &[u8]) -> Option<usize> {
    a.iter().zip(b.iter()).position(|(x, y)| x != y)
}

/// Run the same `(w, h, bytes)` through both libraries and require the
/// resulting buffers to be byte-identical. Returns the (shared) output.
pub fn diff_bytes(
    c: &Lib,
    r: &Lib,
    label: &str,
    w: c_int,
    h: c_int,
    bytes: &[u8],
) -> Vec<u8> {
    let mut cb = bytes.to_vec();
    let mut rb = bytes.to_vec();
    c.call_bytes(w, h, &mut cb);
    r.call_bytes(w, h, &mut rb);
    if cb != rb {
        let i = first_diff(&cb, &rb).unwrap_or(0);
        let px = i / 4;
        let lo = px * 4;
        panic!(
            "[{label}] divergence: w={w} h={h} len={} bytes\n  \
             first differing byte index {i} (pixel {px}, channel {})\n  \
             input  pixel = {:?}\n  C      pixel = {:?}\n  Rust   pixel = {:?}",
            bytes.len(),
            i % 4,
            &bytes[lo..lo + 4],
            &cb[lo..lo + 4],
            &rb[lo..lo + 4],
        );
    }
    cb
}

/// Same as [`diff_bytes`] but for buffers with extra slack past `w*h`: also
/// asserts that neither implementation touched the slack region, i.e. both
/// walked the exact same byte range.
pub fn diff_bytes_with_slack(
    c: &Lib,
    r: &Lib,
    label: &str,
    w: c_int,
    h: c_int,
    bytes: &[u8],
    live_bytes: usize,
) -> Vec<u8> {
    let out = diff_bytes(c, r, label, w, h, bytes);
    assert_eq!(
        &out[live_bytes..],
        &bytes[live_bytes..],
        "[{label}] slack past {live_bytes} bytes was modified (w={w} h={h})"
    );
    out
}

/// The reference model of the C loop bound, used only to decide how large a
/// backing buffer a given `(w, h)` needs. Mirrors
/// `int stride = w * sizeof(cp_pixel_t); (int)stride * h`.
pub fn c_loop_bound(w: c_int, h: c_int) -> c_int {
    // `w * sizeof(cp_pixel_t)` is a size_t multiply truncated back to int,
    // which is indistinguishable from a wrapping 32-bit multiply.
    let stride = w.wrapping_mul(4);
    stride.wrapping_mul(h)
}

/// Number of bytes the C loop will touch for `(w, h)`: the loop starts at 0,
/// steps by 4 and touches `i..i+4`, so it covers exactly `max(bound, 0)` bytes
/// rounded up to a multiple of 4.
pub fn c_touched_bytes(w: c_int, h: c_int) -> usize {
    let bound = c_loop_bound(w, h);
    if bound <= 0 {
        0
    } else {
        // ceil to multiple of 4; bound is always a multiple of 4 in practice
        // because stride is w*4, but be safe.
        (((bound as u32) + 3) & !3u32) as usize
    }
}
