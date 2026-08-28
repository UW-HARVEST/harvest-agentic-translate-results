//! Shared differential-test harness.
//!
//! Both the C `libdriver.so` and the Rust `libdriver.so` are loaded with
//! `libloading` and driven purely through their exported `UTIL_createLinePointers`
//! symbol. The Rust crate is never linked or called directly, so the
//! `#[no_mangle] extern "C"` wrapper is part of what is under test.

#![allow(dead_code)]

use std::ffi::c_void;
use std::os::raw::c_char;
use std::path::PathBuf;

use libloading::{Library, Symbol};

/// `const char** UTIL_createLinePointers(char*, size_t, size_t)`
pub type CreateLinePointersFn =
    unsafe extern "C" fn(*mut c_char, usize, usize) -> *const *const c_char;

extern "C" {
    fn free(ptr: *mut c_void);
    fn atexit(cb: extern "C" fn()) -> i32;
}

/// Number of FFI invocations made through this harness (C + Rust combined).
/// Printed at process exit so the report can cite a measured number instead of
/// an estimate. Set `HARNESS_STATS=0` to silence.
pub static FFI_CALLS: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

extern "C" fn report_stats() {
    if std::env::var("HARNESS_STATS").as_deref() == Ok("0") {
        return;
    }
    let n = FFI_CALLS.load(std::sync::atomic::Ordering::Relaxed);
    eprintln!("[harness] FFI invocations through the two .so's: {n}");
}

// ---------------------------------------------------------------------------
// .so discovery
// ---------------------------------------------------------------------------

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// `<workdir>/c_src/build/libdriver.so`
fn c_so_path() -> PathBuf {
    let candidates = [
        manifest_dir().join("../c_src/build/libdriver.so"),
        manifest_dir().join("../c_src/build/libdriver.dylib"),
    ];
    for c in &candidates {
        if c.exists() {
            return c.clone();
        }
    }
    panic!(
        "C shared library not found. Build it with:\n  \
         cd c_src && mkdir -p build && cd build && \
         cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .\n\
         looked in: {:?}",
        candidates
    );
}

/// Profile of the running test binary, inferred from its path
/// (`target/<profile>/deps/<test>-<hash>`).
fn current_profile() -> String {
    let exe = std::env::current_exe().expect("current_exe");
    exe.parent()
        .and_then(|deps| deps.parent())
        .and_then(|p| p.file_name())
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| "debug".to_string())
}

/// The Rust cdylib under test — **built on demand** so it can never be stale.
///
/// `cargo test` does *not* rebuild a `crate-type = ["cdylib"]` artifact, so
/// simply looking inside `target/<profile>/` silently loads whatever `.so` an
/// earlier `cargo build` happened to leave behind. That makes the whole
/// differential suite vacuous. We therefore run a nested `cargo build --lib`
/// into a *separate* target directory (so it cannot deadlock against the
/// parent `cargo test`'s lock on `target/`) and then assert the artifact is
/// newer than every source file.
fn rust_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("RUST_DRIVER_SO") {
        let p = PathBuf::from(p);
        assert!(p.exists(), "RUST_DRIVER_SO does not exist: {}", p.display());
        return p;
    }

    let profile = current_profile();
    let target_dir = manifest_dir().join("target/so-under-test").join(&profile);

    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());
    let mut cmd = std::process::Command::new(&cargo);
    cmd.current_dir(manifest_dir())
        .arg("build")
        .arg("--lib")
        .arg("--offline")
        .arg("--target-dir")
        .arg(&target_dir);
    if profile != "debug" {
        cmd.arg("--profile").arg(&profile);
    }
    // Propagate the feature selection of the parent `cargo test` invocation so
    // that `--no-default-features --features X` really is what gets loaded.
    if let Ok(extra) = std::env::var("SO_UNDER_TEST_CARGO_ARGS") {
        for a in extra.split_whitespace() {
            cmd.arg(a);
        }
    }
    // Never inherit the parent's target dir / profile env.
    cmd.env_remove("CARGO_TARGET_DIR");
    cmd.env_remove("RUSTC_WORKSPACE_WRAPPER");

    let out = cmd
        .output()
        .unwrap_or_else(|e| panic!("failed to spawn `{cargo} build --lib`: {e}"));
    assert!(
        out.status.success(),
        "nested `cargo build --lib` failed:\n--- stdout ---\n{}\n--- stderr ---\n{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );

    let names = ["libdriver.so", "libdriver.dylib"];
    let dir = target_dir.join(&profile);
    let so = names
        .iter()
        .map(|n| dir.join(n))
        .find(|p| p.exists())
        .unwrap_or_else(|| {
            panic!(
                "nested build succeeded but no cdylib in {}: {:?}",
                dir.display(),
                std::fs::read_dir(&dir)
                    .map(|it| it.filter_map(|e| e.ok()).map(|e| e.file_name()).collect::<Vec<_>>())
                    .unwrap_or_default()
            )
        });

    assert_fresh(&so);
    so
}

/// Guard against ever testing a stale artifact again.
fn assert_fresh(so: &PathBuf) {
    let so_mtime = std::fs::metadata(so)
        .and_then(|m| m.modified())
        .expect("stat cdylib");
    let src_dir = manifest_dir().join("src");
    let mut newest: Option<(std::time::SystemTime, PathBuf)> = None;
    let mut stack = vec![src_dir];
    while let Some(d) = stack.pop() {
        if let Ok(rd) = std::fs::read_dir(&d) {
            for e in rd.filter_map(|e| e.ok()) {
                let p = e.path();
                if p.is_dir() {
                    stack.push(p);
                } else if p.extension().map(|x| x == "rs").unwrap_or(false) {
                    if let Ok(t) = e.metadata().and_then(|m| m.modified()) {
                        if newest.as_ref().map(|(nt, _)| t > *nt).unwrap_or(true) {
                            newest = Some((t, p));
                        }
                    }
                }
            }
        }
    }
    if let Some((t, p)) = newest {
        assert!(
            so_mtime >= t,
            "STALE ARTIFACT: {} is older than {} — the differential suite would be \
             testing outdated code",
            so.display(),
            p.display()
        );
    }
}

// ---------------------------------------------------------------------------
// Loaded pair
// ---------------------------------------------------------------------------

pub struct Impl {
    pub name: &'static str,
    pub path: PathBuf,
    _lib: Library,
    create: CreateLinePointersFn,
}

impl Impl {
    fn load(name: &'static str, path: PathBuf) -> Impl {
        let lib = unsafe { Library::new(&path) }
            .unwrap_or_else(|e| panic!("dlopen {} ({}) failed: {e}", name, path.display()));
        let create: CreateLinePointersFn = unsafe {
            let sym: Symbol<CreateLinePointersFn> = lib
                .get(b"UTIL_createLinePointers\0")
                .unwrap_or_else(|e| panic!("dlsym UTIL_createLinePointers in {name} failed: {e}"));
            *sym
        };
        Impl {
            name,
            path,
            _lib: lib,
            create,
        }
    }

    /// Raw FFI call.
    pub unsafe fn create_raw(
        &self,
        buffer: *mut c_char,
        num_lines: usize,
        buffer_size: usize,
    ) -> *const *const c_char {
        FFI_CALLS.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        (self.create)(buffer, num_lines, buffer_size)
    }
}

pub struct Pair {
    pub c: Impl,
    pub rust: Impl,
}

/// Load both libraries once per test binary.
pub fn pair() -> &'static Pair {
    use std::sync::OnceLock;
    static PAIR: OnceLock<Pair> = OnceLock::new();
    PAIR.get_or_init(|| {
        unsafe { atexit(report_stats) };
        Pair {
            c: Impl::load("C", c_so_path()),
            rust: Impl::load("Rust", rust_so_path()),
        }
    })
}

pub fn c_so_file() -> PathBuf {
    c_so_path()
}
pub fn rust_so_file() -> PathBuf {
    rust_so_path()
}
pub fn nm_available() -> bool {
    std::process::Command::new("nm")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

// ---------------------------------------------------------------------------
// Observed result: a NULL-ness flag plus the array contents
// ---------------------------------------------------------------------------

/// What an external caller can actually observe from one invocation.
///
/// The *identity* of the returned block differs between the two libraries
/// (two independent `malloc`s), so we normalise to:
///   * `None`            -> the function returned `NULL`
///   * `Some(offsets)`   -> the `numLines` stored pointers, expressed as
///                          byte offsets relative to the `buffer` argument.
///
/// Because both libraries are handed the *same* `buffer` pointer, equal
/// offsets is exactly equivalent to bit-identical stored pointers; the
/// raw pointers are additionally compared in `Observed::raw`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Observed {
    pub null: bool,
    pub offsets: Vec<isize>,
    pub raw: Vec<usize>,
}

impl Observed {
    pub fn null() -> Observed {
        Observed {
            null: true,
            offsets: Vec::new(),
            raw: Vec::new(),
        }
    }
}

/// Invoke `imp` and snapshot the observable result, then `free()` the block.
///
/// # Safety
/// `buffer` must be valid for `buffer_size` bytes (or be null when
/// `buffer_size == 0`), and `num_lines * 8` must not wrap to a value smaller
/// than the number of elements the C would write (see `ERRORS.md` rows 10/11).
pub unsafe fn observe(
    imp: &Impl,
    buffer: *mut c_char,
    num_lines: usize,
    buffer_size: usize,
) -> Observed {
    let ret = imp.create_raw(buffer, num_lines, buffer_size);
    if ret.is_null() {
        return Observed::null();
    }
    let mut offsets = Vec::with_capacity(num_lines.min(1 << 20));
    let mut raw = Vec::with_capacity(num_lines.min(1 << 20));
    for i in 0..num_lines {
        let p = *ret.add(i);
        raw.push(p as usize);
        offsets.push((p as isize) - (buffer as isize));
    }
    free(ret as *mut c_void);
    Observed {
        null: false,
        offsets,
        raw,
    }
}

/// Core differential assertion: run both libraries over the *same* buffer and
/// require byte-identical observable results.
///
/// The buffer contents are also compared afterwards — the C never writes to
/// `buffer`, so neither may the Rust.
pub fn assert_same(bytes: &[u8], num_lines: usize, buffer_size: usize, ctx: &str) {
    assert!(
        buffer_size <= bytes.len(),
        "test bug [{ctx}]: bufferSize {buffer_size} exceeds the real backing \
         allocation ({} bytes) — that is ERRORS.md row 10 (UB) and must not be run",
        bytes.len()
    );

    // ONE shared allocation handed to BOTH libraries, so the returned arrays
    // must be *bit-identical*, not merely equivalent modulo a base address.
    let mut buf: Vec<u8> = bytes.to_vec();
    let base = buf.as_mut_ptr() as *mut c_char;
    let p = pair();

    let (oc, or) = unsafe {
        let oc = observe(&p.c, base, num_lines, buffer_size);
        let or = observe(&p.rust, base, num_lines, buffer_size);
        (oc, or)
    };

    assert_eq!(
        oc.null, or.null,
        "NULL-ness mismatch [{ctx}]: C null={} Rust null={} (numLines={num_lines}, \
         bufferSize={buffer_size}, bytes={:?})",
        oc.null, or.null, bytes
    );
    assert_eq!(
        oc.raw, or.raw,
        "returned pointer array is not bit-identical [{ctx}] (numLines={num_lines}, \
         bufferSize={buffer_size}, bytes={:?})\n C   = {:?}\n Rust= {:?}",
        bytes, oc.offsets, or.offsets
    );
    assert_eq!(
        oc.offsets, or.offsets,
        "line-pointer offsets mismatch [{ctx}] (numLines={num_lines}, \
         bufferSize={buffer_size}, bytes={:?})\n C   = {:?}\n Rust= {:?}",
        bytes, oc.offsets, or.offsets
    );
    assert_eq!(
        &buf[..],
        bytes,
        "the caller's buffer was mutated [{ctx}] (numLines={num_lines}, \
         bufferSize={buffer_size}) — neither implementation may write to it"
    );

    // Every returned pointer must land inside [base, base+bufferSize).
    for (i, &off) in oc.offsets.iter().enumerate() {
        assert!(
            off >= 0 && (off as usize) < buffer_size.max(1),
            "C returned an out-of-range pointer at index {i}: offset {off} \
             (bufferSize={buffer_size}) [{ctx}]"
        );
    }
}

/// Same as [`assert_same`] but for scalar-only configurations where no real
/// buffer is dereferenced (`buffer_size == 0`), including `buffer == NULL`.
pub fn assert_same_null_buffer(num_lines: usize, buffer_size: usize, ctx: &str) {
    assert_eq!(
        buffer_size, 0,
        "assert_same_null_buffer is only safe with bufferSize == 0"
    );
    let p = pair();
    let (oc, or) = unsafe {
        (
            observe(&p.c, std::ptr::null_mut(), num_lines, buffer_size),
            observe(&p.rust, std::ptr::null_mut(), num_lines, buffer_size),
        )
    };
    assert_eq!(
        oc.null, or.null,
        "NULL-ness mismatch [{ctx}] (numLines={num_lines}, bufferSize={buffer_size})"
    );
    assert_eq!(
        oc.offsets, or.offsets,
        "offsets mismatch [{ctx}] (numLines={num_lines}, bufferSize={buffer_size})"
    );
}

// ---------------------------------------------------------------------------
// Reference model (independent re-derivation of the C algorithm)
// ---------------------------------------------------------------------------

/// Offsets the C algorithm must produce, or `None` for a `NULL` return.
/// Used as a third opinion so a *shared* misunderstanding cannot hide.
pub fn model(bytes: &[u8], num_lines: usize, buffer_size: usize) -> Option<Vec<isize>> {
    let mut out = Vec::new();
    let mut pos: usize = 0;
    let mut line_index: usize = 0;
    while line_index < num_lines && pos < buffer_size {
        out.push(pos as isize);
        line_index += 1;
        let mut len = 0usize;
        while pos + len < buffer_size && bytes[pos + len] != 0 {
            len += 1;
        }
        pos += len;
        if pos < buffer_size {
            pos += 1;
        }
    }
    if line_index != num_lines {
        None
    } else {
        Some(out)
    }
}

/// [`assert_same`] plus a check against the independent [`model`].
pub fn assert_same_and_model(bytes: &[u8], num_lines: usize, buffer_size: usize, ctx: &str) {
    assert_same(bytes, num_lines, buffer_size, ctx);

    let expected = model(bytes, num_lines, buffer_size);
    let mut buf: Vec<u8> = bytes.to_vec();
    let p = pair();
    let got = unsafe {
        observe(
            &p.c,
            buf.as_mut_ptr() as *mut c_char,
            num_lines,
            buffer_size,
        )
    };
    match expected {
        None => assert!(
            got.null,
            "model says NULL but C returned a block [{ctx}] \
             (numLines={num_lines}, bufferSize={buffer_size}, bytes={bytes:?})"
        ),
        Some(exp) => {
            assert!(
                !got.null,
                "model says success but C returned NULL [{ctx}] \
                 (numLines={num_lines}, bufferSize={buffer_size}, bytes={bytes:?})"
            );
            assert_eq!(
                got.offsets, exp,
                "model/C offsets disagree [{ctx}] (numLines={num_lines}, \
                 bufferSize={buffer_size}, bytes={bytes:?})"
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (PCG32) — fixed seed for reproducibility
// ---------------------------------------------------------------------------

pub const SEED: u64 = 0x5DEECE66D;

pub struct Rng {
    state: u64,
    inc: u64,
}

impl Rng {
    pub fn new(seed: u64) -> Rng {
        let mut r = Rng {
            state: 0,
            inc: (seed << 1) | 1,
        };
        r.next_u32();
        r.state = r.state.wrapping_add(seed);
        r.next_u32();
        r
    }

    pub fn next_u32(&mut self) -> u32 {
        let old = self.state;
        self.state = old
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(self.inc);
        let xorshifted = (((old >> 18) ^ old) >> 27) as u32;
        let rot = (old >> 59) as u32;
        xorshifted.rotate_right(rot)
    }

    pub fn next_u64(&mut self) -> u64 {
        ((self.next_u32() as u64) << 32) | self.next_u32() as u64
    }

    /// Uniform in `[0, n)`; `n == 0` yields 0.
    pub fn below(&mut self, n: usize) -> usize {
        if n == 0 {
            return 0;
        }
        (self.next_u64() % n as u64) as usize
    }

    /// Uniform in `[lo, hi]`.
    pub fn range(&mut self, lo: usize, hi: usize) -> usize {
        lo + self.below(hi - lo + 1)
    }

    pub fn byte(&mut self) -> u8 {
        (self.next_u32() >> 24) as u8
    }

    /// Random bytes; each byte is NUL with probability `nul_percent/100`,
    /// otherwise uniform over `1..=255`.
    pub fn bytes(&mut self, len: usize, nul_percent: u32) -> Vec<u8> {
        (0..len)
            .map(|_| {
                if self.next_u32() % 100 < nul_percent {
                    0u8
                } else {
                    let b = self.byte();
                    if b == 0 {
                        1
                    } else {
                        b
                    }
                }
            })
            .collect()
    }
}

/// Build a buffer of `k` NUL-terminated segments with lengths from `rng`.
/// If `terminate_last` is false the final segment has no NUL.
pub fn segments(rng: &mut Rng, k: usize, max_len: usize, terminate_last: bool) -> Vec<u8> {
    let mut out = Vec::new();
    for i in 0..k {
        let len = rng.below(max_len + 1);
        for _ in 0..len {
            let mut b = rng.byte();
            if b == 0 {
                b = 1;
            }
            out.push(b);
        }
        if i + 1 < k || terminate_last {
            out.push(0);
        }
    }
    out
}
