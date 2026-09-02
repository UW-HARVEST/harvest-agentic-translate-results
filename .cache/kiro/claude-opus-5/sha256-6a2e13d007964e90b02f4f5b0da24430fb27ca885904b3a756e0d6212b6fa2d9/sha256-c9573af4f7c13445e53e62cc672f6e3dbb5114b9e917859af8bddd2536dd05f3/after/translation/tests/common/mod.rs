//! Shared harness for the C-vs-Rust differential tests.
//!
//! Both libraries are loaded as shared objects with `libloading`; the Rust
//! implementation is *never* called directly, so the `#[no_mangle] extern "C"`
//! export wrapper is under test too.

#![allow(dead_code)]
#![allow(non_camel_case_types)]

use std::ffi::c_int;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

// ---------------------------------------------------------------------------
// FFI mirrors of the C types (c_src/include/lib.h)
// ---------------------------------------------------------------------------

#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct cp_pixel_t {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct cp_image_t {
    pub w: c_int,
    pub h: c_int,
    pub pix: *mut cp_pixel_t,
}

pub type FlipFn = unsafe extern "C" fn(*mut cp_image_t);

// ---------------------------------------------------------------------------
// Library discovery + loading
// ---------------------------------------------------------------------------

/// `translation/`
fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn first_so_in(dir: &Path, name_hint: Option<&str>) -> Option<PathBuf> {
    let entries = std::fs::read_dir(dir).ok()?;
    let mut found: Vec<PathBuf> = Vec::new();
    for e in entries.flatten() {
        let p = e.path();
        if p.extension().and_then(|s| s.to_str()) != Some("so") {
            continue;
        }
        match name_hint {
            Some(h) => {
                if p.file_name()
                    .and_then(|s| s.to_str())
                    .map(|s| s.contains(h))
                    .unwrap_or(false)
                {
                    found.push(p);
                }
            }
            None => found.push(p),
        }
    }
    found.sort();
    found.into_iter().next()
}

/// Path to the C shared object built by `c_src/CMakeLists.txt`.
///
/// The CMake project name is derived from the parent directory name, so the
/// `.so` file name is not fixed; discover it instead of hardcoding it.
pub fn c_so_path() -> PathBuf {
    let build = manifest_dir().join("../c_src/build");
    let msg = format!(
        "C shared library not found in {}.\nBuild it with:\n  cd c_src && mkdir -p build && cd build \\\n    && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
        build.display()
    );
    let so = first_so_in(&build, None).unwrap_or_else(|| panic!("{msg}"));

    // Guard against verifying against a stale C build.
    let newest_src = newest_mtime(&[
        manifest_dir().join("../c_src/src/lib.c"),
        manifest_dir().join("../c_src/include/lib.h"),
        manifest_dir().join("../c_src/CMakeLists.txt"),
    ]);
    if let (Some(src), Some(obj)) = (newest_src, mtime(&so)) {
        assert!(
            obj >= src,
            "STALE C .so: {} is older than the C sources. Rebuild it:\n  cd c_src/build && cmake --build .",
            so.display()
        );
    }
    so
}

fn mtime(p: &Path) -> Option<std::time::SystemTime> {
    std::fs::metadata(p).ok()?.modified().ok()
}

fn newest_mtime(paths: &[PathBuf]) -> Option<std::time::SystemTime> {
    paths.iter().filter_map(|p| mtime(p)).max()
}

fn newest_rust_source_mtime() -> Option<std::time::SystemTime> {
    let mut newest = mtime(&manifest_dir().join("Cargo.toml"));
    let mut stack = vec![manifest_dir().join("src")];
    while let Some(dir) = stack.pop() {
        if let Ok(rd) = std::fs::read_dir(&dir) {
            for e in rd.flatten() {
                let p = e.path();
                if p.is_dir() {
                    stack.push(p);
                } else if p.extension().and_then(|s| s.to_str()) == Some("rs") {
                    let m = mtime(&p);
                    if newest.is_none() || (m.is_some() && m > newest) {
                        newest = m;
                    }
                }
            }
        }
    }
    newest
}

/// Path to the Rust `cdylib` under test.
///
/// Resolved strictly from the running test executable
/// (`target/<profile>/deps/<test>` -> `target/<profile>`).
///
/// **There is deliberately no fallback to another profile directory and no
/// automatic rebuild.** `cargo test` does *not* emit `cdylib` artifacts, so an
/// earlier `cargo build --release` can leave a stale `.so` lying around;
/// picking it up would make every differential test compare the C against an
/// outdated Rust build and silently pass (this actually happened during
/// bring-up: four semantic mutations of `src/lib.rs` went undetected).
///
/// So: if the `.so` for *this* profile is missing or older than the Rust
/// sources, the test suite fails loudly. Use `scripts/verify_all.sh`, or run
/// `cargo build` before `cargo test`.
pub fn rust_so_path() -> PathBuf {
    let exe = std::env::current_exe().expect("current_exe");
    // .../target/<profile>/deps/<testbin>  ->  .../target/<profile>
    let profile_dir = exe
        .parent()
        .and_then(|p| p.parent())
        .expect("test exe parent")
        .to_path_buf();

    let is_release = profile_dir
        .file_name()
        .and_then(|s| s.to_str())
        .map(|s| s == "release")
        .unwrap_or(false);
    let build_hint = if is_release {
        "cargo build --release"
    } else {
        "cargo build"
    };

    let expected = profile_dir.join("libflip_horizontal_lib.so");

    assert!(
        expected.is_file(),
        "Rust cdylib not found at {}.\n\
         `cargo test` does not emit cdylib artifacts — run `{build_hint}` first,\n\
         or use scripts/verify_all.sh which sequences everything correctly.",
        expected.display()
    );

    if let (Some(obj), Some(src)) = (mtime(&expected), newest_rust_source_mtime()) {
        assert!(
            obj >= src,
            "STALE Rust cdylib: {} is older than the Rust sources.\n\
             Re-run `{build_hint}`; testing against a stale .so silently passes.",
            expected.display()
        );
    }

    expected
}

pub struct Impls {
    pub c: FlipFn,
    pub rust: FlipFn,
    // Keep the libraries alive for the whole process lifetime.
    _libs: Vec<&'static libloading::Library>,
}

static IMPLS: OnceLock<Impls> = OnceLock::new();

/// Load both `.so`s once and resolve `flip_horizontal` from each.
pub fn impls() -> &'static Impls {
    IMPLS.get_or_init(|| unsafe {
        let cpath = c_so_path();
        let rpath = rust_so_path();

        let clib: &'static libloading::Library = Box::leak(Box::new(
            libloading::Library::new(&cpath)
                .unwrap_or_else(|e| panic!("dlopen {}: {e}", cpath.display())),
        ));
        let rlib: &'static libloading::Library = Box::leak(Box::new(
            libloading::Library::new(&rpath)
                .unwrap_or_else(|e| panic!("dlopen {}: {e}", rpath.display())),
        ));

        let csym: libloading::Symbol<FlipFn> = clib
            .get(b"flip_horizontal\0")
            .expect("C .so must export flip_horizontal");
        let rsym: libloading::Symbol<FlipFn> = rlib
            .get(b"flip_horizontal\0")
            .expect("Rust .so must export flip_horizontal");

        Impls {
            c: *csym,
            rust: *rsym,
            _libs: vec![clib, rlib],
        }
    })
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (SplitMix64)
// ---------------------------------------------------------------------------

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
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
    pub fn next_u8(&mut self) -> u8 {
        (self.next_u64() >> 56) as u8
    }
    /// Uniform-ish in `[lo, hi]` (inclusive).
    pub fn range_i32(&mut self, lo: i32, hi: i32) -> i32 {
        assert!(lo <= hi);
        let span = (hi as i64 - lo as i64 + 1) as u64;
        (lo as i64 + (self.next_u64() % span) as i64) as i32
    }
}

/// The single fixed seed for the whole suite, so every run is reproducible.
pub const SEED: u64 = 0x5EED_1234_ABCD_0001;

/// Randomized iterations per `CONFIGS.md` / `ERRORS.md` row.
pub const REPS: usize = 64;

// ---------------------------------------------------------------------------
// Guarded pixel buffer
// ---------------------------------------------------------------------------

/// Bytes of poison placed before and after the pixel payload. `flip_horizontal`
/// must never touch them, so comparing the *whole* backing store (not just the
/// payload) also catches out-of-bounds writes.
pub const GUARD_PIXELS: usize = 16; // 64 bytes

/// A pixel buffer whose `pix` pointer deliberately points into the interior of
/// a larger allocation, surrounded by poison guard bands.
pub struct Buf {
    store: Vec<cp_pixel_t>,
    /// Index of the first payload pixel inside `store`.
    start: usize,
    /// Number of payload pixels.
    len: usize,
}

impl Buf {
    /// `extra_byte_skew` shifts `pix` by that many *bytes* inside the
    /// allocation, which lets a test produce a 4-byte-but-not-8-byte-aligned
    /// `pix` (pixels are 4 bytes, so any pixel index gives 4-byte alignment;
    /// an odd pixel index gives a non-8-byte-aligned address).
    pub fn new(len: usize, guard: usize, odd_offset: bool) -> Self {
        let start = guard + usize::from(odd_offset);
        let store = vec![cp_pixel_t::default(); start + len + guard + 1];
        let mut b = Buf { store, start, len };
        b.poison_guards();
        b
    }

    fn poison_guards(&mut self) {
        let poison = cp_pixel_t {
            r: 0xDE,
            g: 0xAD,
            b: 0xBE,
            a: 0xEF,
        };
        for i in 0..self.start {
            self.store[i] = poison;
        }
        for i in (self.start + self.len)..self.store.len() {
            self.store[i] = poison;
        }
    }

    pub fn fill_random(&mut self, rng: &mut Rng) {
        for i in 0..self.len {
            let idx = self.start + i;
            self.store[idx] = cp_pixel_t {
                r: rng.next_u8(),
                g: rng.next_u8(),
                b: rng.next_u8(),
                a: rng.next_u8(),
            };
        }
    }

    pub fn fill_with(&mut self, mut f: impl FnMut(usize) -> cp_pixel_t) {
        for i in 0..self.len {
            let idx = self.start + i;
            self.store[idx] = f(i);
        }
    }

    pub fn pix_ptr(&mut self) -> *mut cp_pixel_t {
        // SAFETY: `start <= store.len()`, so this stays in bounds.
        unsafe { self.store.as_mut_ptr().add(self.start) }
    }

    /// Every byte of the backing allocation, guards included.
    pub fn all_bytes(&self) -> &[u8] {
        unsafe {
            std::slice::from_raw_parts(
                self.store.as_ptr() as *const u8,
                std::mem::size_of_val(&self.store[..]),
            )
        }
    }

    pub fn payload(&self) -> &[cp_pixel_t] {
        &self.store[self.start..self.start + self.len]
    }

    pub fn clone_layout(&self) -> Self {
        Buf {
            store: self.store.clone(),
            start: self.start,
            len: self.len,
        }
    }
}

// ---------------------------------------------------------------------------
// Differential driver
// ---------------------------------------------------------------------------

/// Description of one differential case.
#[derive(Clone, Copy, Debug)]
pub struct Case {
    /// The `w` field written into `cp_image_t` (may be nonsensical on purpose).
    pub w: c_int,
    /// The `h` field written into `cp_image_t`.
    pub h: c_int,
    /// Payload pixels to actually allocate.
    pub alloc_pixels: usize,
    /// Use a NULL `pix` pointer instead of the allocation.
    pub null_pix: bool,
    /// Place `pix` at an odd pixel index (4-byte but not 8-byte aligned).
    pub odd_offset: bool,
    /// How many times to invoke the function.
    pub calls: usize,
}

impl Case {
    pub fn new(w: c_int, h: c_int) -> Self {
        let pixels = (w.max(0) as i64 * h.max(0) as i64) as usize;
        Case {
            w,
            h,
            alloc_pixels: pixels,
            null_pix: false,
            odd_offset: false,
            calls: 1,
        }
    }
    pub fn alloc(mut self, pixels: usize) -> Self {
        self.alloc_pixels = pixels;
        self
    }
    pub fn null_pix(mut self) -> Self {
        self.null_pix = true;
        self
    }
    pub fn odd_offset(mut self) -> Self {
        self.odd_offset = true;
        self
    }
    pub fn calls(mut self, n: usize) -> Self {
        self.calls = n;
        self
    }
}

fn run_one(f: FlipFn, case: &Case, seed_payload: &Buf) -> Buf {
    let mut buf = seed_payload.clone_layout();
    let pix = if case.null_pix {
        std::ptr::null_mut()
    } else {
        buf.pix_ptr()
    };
    let mut img = cp_image_t {
        w: case.w,
        h: case.h,
        pix,
    };
    for _ in 0..case.calls {
        unsafe { f(&mut img) };
    }
    buf
}

/// Run the case against BOTH `.so`s from identical starting buffers and assert
/// the complete backing allocations (payload + guard bands) match byte for byte.
#[track_caller]
pub fn assert_same(label: &str, case: &Case, seed_payload: &Buf) {
    let im = impls();
    let c_out = run_one(im.c, case, seed_payload);
    let r_out = run_one(im.rust, case, seed_payload);

    let cb = c_out.all_bytes();
    let rb = r_out.all_bytes();
    assert_eq!(
        cb.len(),
        rb.len(),
        "{label}: internal harness error (buffer size mismatch)"
    );
    if cb != rb {
        let first = (0..cb.len()).find(|&i| cb[i] != rb[i]).unwrap();
        panic!(
            "{label}: C and Rust diverge for {case:?}\n  first differing byte index {first}: C=0x{:02x} Rust=0x{:02x}\n  C   payload[..min]={:?}\n  Rust payload[..min]={:?}",
            cb[first],
            rb[first],
            &c_out.payload()[..c_out.payload().len().min(12)],
            &r_out.payload()[..r_out.payload().len().min(12)],
        );
    }
}

/// Convenience: build a randomly filled guarded buffer for `case` and diff it.
#[track_caller]
pub fn assert_same_random(label: &str, case: &Case, rng: &mut Rng) {
    let mut seed = Buf::new(case.alloc_pixels, GUARD_PIXELS, case.odd_offset);
    seed.fill_random(rng);
    assert_same(label, case, &seed);
}

/// Assert the case is a complete no-op in BOTH implementations *and* that the
/// two agree — i.e. the buffer comes back bit-identical to the input.
#[track_caller]
pub fn assert_same_and_noop(label: &str, case: &Case, rng: &mut Rng) {
    let mut seed = Buf::new(case.alloc_pixels, GUARD_PIXELS, case.odd_offset);
    seed.fill_random(rng);
    assert_same(label, case, &seed);

    let im = impls();
    let c_out = run_one(im.c, case, &seed);
    let r_out = run_one(im.rust, case, &seed);
    assert_eq!(
        seed.all_bytes(),
        c_out.all_bytes(),
        "{label}: expected C to be a no-op for {case:?}"
    );
    assert_eq!(
        seed.all_bytes(),
        r_out.all_bytes(),
        "{label}: expected Rust to be a no-op for {case:?}"
    );
}

// ---------------------------------------------------------------------------
// Crash-equivalence helper (for the fatal-signal rows of ERRORS.md)
// ---------------------------------------------------------------------------

/// Outcome of running a closure in a forked child.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum Outcome {
    Exited(i32),
    Signaled(i32),
}

/// Run `body` in a forked child and report how the child terminated.
///
/// Used for the rows where the C code dereferences an invalid pointer: the
/// observable "error result" is a fatal signal, and the Rust must produce the
/// same one.
///
/// `body` must not allocate, `dlopen`, or touch a lazily-initialized global:
/// `cargo test` is multi-threaded, and a child forked while another thread holds
/// the allocator / loader / `OnceLock` lock would block forever on a lock that
/// no longer has an owner. `impls()` is therefore forced to completion in the
/// parent here, so a child can never be the one to initialize it.
pub fn run_in_child(body: impl FnOnce()) -> Outcome {
    let _ = impls();
    unsafe {
        let pid = libc::fork();
        assert!(pid >= 0, "fork failed");
        if pid == 0 {
            // Several of these children are *expected* to die on SIGSEGV.
            // Without this, each one hands a core dump to the system core
            // handler, which dominates the wall-clock time of the suite
            // (~5x) while contributing nothing.
            let no_core = libc::rlimit {
                rlim_cur: 0,
                rlim_max: 0,
            };
            libc::setrlimit(libc::RLIMIT_CORE, &no_core);
            body();
            libc::_exit(0);
        }
        let mut status: libc::c_int = 0;
        let r = libc::waitpid(pid, &mut status, 0);
        assert_eq!(r, pid, "waitpid failed");
        if libc::WIFSIGNALED(status) {
            Outcome::Signaled(libc::WTERMSIG(status))
        } else {
            Outcome::Exited(libc::WEXITSTATUS(status))
        }
    }
}

/// Assert both implementations terminate the same way for a case that is
/// expected to fault (or not) identically.
///
/// The buffer is allocated *before* forking so the child does nothing but the
/// FFI call and `_exit` — forking a multi-threaded test process and then
/// allocating would risk deadlocking on the allocator lock.
#[track_caller]
pub fn assert_same_outcome(label: &str, case: Case) {
    let im = impls();

    let mut c_buf = Buf::new(case.alloc_pixels, GUARD_PIXELS, case.odd_offset);
    let c_pix = if case.null_pix {
        std::ptr::null_mut()
    } else {
        c_buf.pix_ptr()
    };
    let mut c_img = cp_image_t {
        w: case.w,
        h: case.h,
        pix: c_pix,
    };

    let mut r_buf = Buf::new(case.alloc_pixels, GUARD_PIXELS, case.odd_offset);
    let r_pix = if case.null_pix {
        std::ptr::null_mut()
    } else {
        r_buf.pix_ptr()
    };
    let mut r_img = cp_image_t {
        w: case.w,
        h: case.h,
        pix: r_pix,
    };

    let c_img_ptr: usize = &mut c_img as *mut cp_image_t as usize;
    let r_img_ptr: usize = &mut r_img as *mut cp_image_t as usize;

    let c_out = run_in_child(move || unsafe { (im.c)(c_img_ptr as *mut cp_image_t) });
    let r_out = run_in_child(move || unsafe { (im.rust)(r_img_ptr as *mut cp_image_t) });

    // Keep the buffers alive across the forks.
    std::hint::black_box((&c_buf, &r_buf));

    assert_eq!(
        c_out, r_out,
        "{label}: termination outcome differs for {case:?} (C={c_out:?}, Rust={r_out:?})"
    );
}

/// Same, but passing a NULL `cp_image_t*` itself.
#[track_caller]
pub fn assert_same_outcome_null_img(label: &str) {
    let im = impls();
    let c_out = run_in_child(move || unsafe { (im.c)(std::ptr::null_mut()) });
    let r_out = run_in_child(move || unsafe { (im.rust)(std::ptr::null_mut()) });
    assert_eq!(
        c_out, r_out,
        "{label}: termination outcome differs for NULL img (C={c_out:?}, Rust={r_out:?})"
    );
}
