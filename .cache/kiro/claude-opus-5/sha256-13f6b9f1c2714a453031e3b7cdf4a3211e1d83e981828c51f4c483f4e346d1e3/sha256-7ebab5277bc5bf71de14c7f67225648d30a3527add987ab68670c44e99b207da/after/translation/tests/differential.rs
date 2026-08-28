//! Differential test: loads BOTH the C `libdriver.so` and the Rust
//! `libdriver.so` via `libloading` and compares the results of
//! `UTIL_createLinePointers` through the FFI boundary.
//!
//! Neither implementation is ever called directly; both go through the
//! dynamic-symbol lookup, so the `#[no_mangle]` export wrapper is covered too.

use std::ffi::{c_char, c_void};
use std::path::PathBuf;
use std::process::Command;
use std::sync::OnceLock;

use libloading::{Library, Symbol};

type CreateLinePointers =
    unsafe extern "C" fn(*mut c_char, usize, usize) -> *const *const c_char;

unsafe extern "C" {
    fn free(ptr: *mut c_void);
}

fn repo_root() -> PathBuf {
    // CARGO_MANIFEST_DIR == <root>/translation
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation dir has a parent")
        .to_path_buf()
}

fn c_lib_path() -> PathBuf {
    repo_root().join("c_src/build/libdriver.so")
}

/// The Rust cdylib for the crate under test.
///
/// `cargo test` does **not** build the `cdylib` artifact, and any leftover
/// `target/{debug,release}/libdriver.so` may be stale or built with a different
/// feature set. So build it explicitly, once per test binary, into a dedicated
/// target directory (a separate target dir avoids contending with the cargo
/// invocation that is currently running these tests).
fn rust_lib_path() -> &'static PathBuf {
    static LIB: OnceLock<PathBuf> = OnceLock::new();
    LIB.get_or_init(|| {
        let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let target_dir = manifest.join("target").join("ffi-cdylib");
        let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());

        let mut cmd = Command::new(cargo);
        cmd.current_dir(&manifest)
            .arg("build")
            .arg("--release")
            .arg("--lib")
            .arg("--target-dir")
            .arg(&target_dir);

        // Mirror the feature selection used for this test run. The harness is
        // told about it via FFI_FEATURES (see scripts/run_all_features.sh);
        // with no features declared in Cargo.toml this is a no-op.
        if std::env::var_os("FFI_NO_DEFAULT_FEATURES").is_some() {
            cmd.arg("--no-default-features");
        }
        if let Ok(features) = std::env::var("FFI_FEATURES") {
            if !features.trim().is_empty() {
                cmd.arg("--features").arg(features);
            }
        }

        let out = cmd.output().expect("failed to spawn cargo to build the cdylib");
        assert!(
            out.status.success(),
            "building the Rust cdylib failed:\n{}",
            String::from_utf8_lossy(&out.stderr)
        );

        let path = target_dir.join("release").join("libdriver.so");
        assert!(path.exists(), "cdylib not produced at {path:?}");
        path
    })
}

struct Impl {
    _lib: Library,
    create: CreateLinePointers,
    name: &'static str,
}

impl Impl {
    fn load(path: PathBuf, name: &'static str) -> Impl {
        assert!(path.exists(), "{name} library missing at {path:?}");
        let lib = unsafe { Library::new(&path) }
            .unwrap_or_else(|e| panic!("failed to load {name} lib {path:?}: {e}"));
        let create = {
            let sym: Symbol<CreateLinePointers> = unsafe {
                lib.get(b"UTIL_createLinePointers\0")
            }
            .unwrap_or_else(|e| {
                panic!("{name} lib does not export UTIL_createLinePointers: {e}")
            });
            *sym
        };
        Impl {
            _lib: lib,
            create,
            name,
        }
    }
}

fn impls() -> (Impl, Impl) {
    (
        Impl::load(c_lib_path(), "C"),
        Impl::load(rust_lib_path().clone(), "Rust"),
    )
}

/// Run both implementations against the same buffer and assert the results are
/// byte-identical. Returns `true` when both returned non-NULL.
fn compare(c: &Impl, r: &Impl, buffer: &mut [u8], num_lines: usize, buffer_size: usize) -> bool {
    let ptr = buffer.as_mut_ptr() as *mut c_char;

    let c_res = unsafe { (c.create)(ptr, num_lines, buffer_size) };
    let r_res = unsafe { (r.create)(ptr, num_lines, buffer_size) };

    let label = format!(
        "num_lines={num_lines} buffer_size={buffer_size} buffer={:?}",
        &buffer[..buffer.len().min(64)]
    );

    assert_eq!(
        c_res.is_null(),
        r_res.is_null(),
        "NULL-ness mismatch ({} vs {}): C={:?} Rust={:?} [{label}]",
        c.name,
        r.name,
        c_res,
        r_res
    );

    let both_ok = !c_res.is_null();
    if both_ok && num_lines > 0 {
        // The returned entries are absolute pointers into `buffer`, which is the
        // very same allocation for both calls, so they must compare equal.
        let c_slice = unsafe { std::slice::from_raw_parts(c_res, num_lines) };
        let r_slice = unsafe { std::slice::from_raw_parts(r_res, num_lines) };
        for i in 0..num_lines {
            assert_eq!(
                c_slice[i], r_slice[i],
                "line pointer {i} differs: C={:?} Rust={:?} [{label}]",
                c_slice[i], r_slice[i]
            );
        }
        // And byte-for-byte over the raw array bytes.
        let n = num_lines * std::mem::size_of::<*const c_char>();
        let c_bytes = unsafe { std::slice::from_raw_parts(c_res as *const u8, n) };
        let r_bytes = unsafe { std::slice::from_raw_parts(r_res as *const u8, n) };
        assert_eq!(c_bytes, r_bytes, "raw array bytes differ [{label}]");
    }

    if !c_res.is_null() {
        unsafe { free(c_res as *mut c_void) };
    }
    if !r_res.is_null() {
        unsafe { free(r_res as *mut c_void) };
    }

    both_ok
}

/// Expected offsets, derived independently from the C algorithm, so the tests
/// also pin down the absolute behaviour instead of only C-vs-Rust agreement.
fn expected_offsets(buffer: &[u8], num_lines: usize, buffer_size: usize) -> Option<Vec<usize>> {
    let mut offsets = Vec::new();
    let mut pos = 0usize;
    while offsets.len() < num_lines && pos < buffer_size {
        offsets.push(pos);
        let mut len = 0usize;
        while pos + len < buffer_size && buffer[pos + len] != 0 {
            len += 1;
        }
        pos += len;
        if pos < buffer_size {
            pos += 1;
        }
    }
    if offsets.len() != num_lines {
        None
    } else {
        Some(offsets)
    }
}

fn compare_and_check_offsets(
    c: &Impl,
    r: &Impl,
    buffer: &mut [u8],
    num_lines: usize,
    buffer_size: usize,
) {
    let expected = expected_offsets(buffer, num_lines, buffer_size);
    let base = buffer.as_ptr() as usize;
    let ptr = buffer.as_mut_ptr() as *mut c_char;

    let c_res = unsafe { (c.create)(ptr, num_lines, buffer_size) };
    assert_eq!(
        c_res.is_null(),
        expected.is_none(),
        "reference model disagrees with C for num_lines={num_lines} buffer_size={buffer_size}"
    );
    if let (Some(exp), false) = (expected.as_ref(), c_res.is_null()) {
        let got = unsafe { std::slice::from_raw_parts(c_res, num_lines) };
        for (i, off) in exp.iter().enumerate() {
            assert_eq!(got[i] as usize - base, *off, "C offset {i} unexpected");
        }
    }
    if !c_res.is_null() {
        unsafe { free(c_res as *mut c_void) };
    }

    compare(c, r, buffer, num_lines, buffer_size);
}

#[test]
fn exports_the_symbol_in_both_libraries() {
    let (_c, _r) = impls();
}

#[test]
fn simple_packed_lines() {
    let (c, r) = impls();
    let mut buf = b"alpha\0beta\0gamma\0".to_vec();
    let size = buf.len();
    for n in 0..=4 {
        compare_and_check_offsets(&c, &r, &mut buf, n, size);
    }
}

#[test]
fn no_trailing_nul() {
    let (c, r) = impls();
    let mut buf = b"one\0two\0three".to_vec();
    let size = buf.len();
    for n in 0..=5 {
        compare_and_check_offsets(&c, &r, &mut buf, n, size);
    }
}

#[test]
fn empty_lines_and_consecutive_nuls() {
    let (c, r) = impls();
    let mut buf = b"\0\0a\0\0\0b\0".to_vec();
    let size = buf.len();
    for n in 0..=9 {
        compare_and_check_offsets(&c, &r, &mut buf, n, size);
    }
}

#[test]
fn zero_num_lines_and_zero_buffer_size() {
    let (c, r) = impls();
    let mut buf = b"x\0y\0".to_vec();
    let size = buf.len();

    // numLines == 0: malloc(0) still hands back a pointer in glibc, so both
    // implementations must report success.
    assert!(compare(&c, &r, &mut buf, 0, size));
    assert!(compare(&c, &r, &mut buf, 0, 0));

    // bufferSize == 0 with numLines > 0: the loop never runs, so both must
    // free the allocation and return NULL.
    assert!(!compare(&c, &r, &mut buf, 1, 0));
    assert!(!compare(&c, &r, &mut buf, 5, 0));
}

#[test]
fn buffer_size_smaller_than_buffer() {
    let (c, r) = impls();
    let mut buf = b"aa\0bbb\0cccc\0d\0".to_vec();
    for size in 0..=buf.len() {
        for n in 0..=5 {
            compare_and_check_offsets(&c, &r, &mut buf, n, size);
        }
    }
}

#[test]
fn all_nul_buffer() {
    let (c, r) = impls();
    let mut buf = vec![0u8; 16];
    let size = buf.len();
    for n in 0..=18 {
        compare_and_check_offsets(&c, &r, &mut buf, n, size);
    }
}

#[test]
fn no_nul_at_all() {
    let (c, r) = impls();
    let mut buf = vec![b'z'; 12];
    let size = buf.len();
    for n in 0..=3 {
        compare_and_check_offsets(&c, &r, &mut buf, n, size);
    }
}

#[test]
fn high_bit_bytes_are_not_treated_as_terminators() {
    let (c, r) = impls();
    // 0xFF matters because C `char` may be signed; only 0x00 terminates.
    let mut buf = vec![0xFFu8; 10];
    buf[4] = 0;
    let size = buf.len();
    for n in 0..=4 {
        compare_and_check_offsets(&c, &r, &mut buf, n, size);
    }
}

#[test]
fn allocation_failure_path() {
    let (c, r) = impls();
    let mut buf = b"a\0".to_vec();
    // numLines * 8 is a ~2^60 byte request: malloc must fail in both libraries.
    let num_lines = usize::MAX / 16;
    let ptr = buf.as_mut_ptr() as *mut c_char;
    let c_res = unsafe { (c.create)(ptr, num_lines, buf.len()) };
    let r_res = unsafe { (r.create)(ptr, num_lines, buf.len()) };
    assert!(c_res.is_null(), "C unexpectedly allocated 2^60 bytes");
    assert_eq!(
        c_res.is_null(),
        r_res.is_null(),
        "allocation failure handling differs"
    );
}

/// Deterministic xorshift so failures are reproducible.
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }
    fn below(&mut self, n: usize) -> usize {
        (self.next() % (n as u64)) as usize
    }
}

#[test]
fn randomised_differential() {
    let (c, r) = impls();
    let mut rng = Rng(0x9E3779B97F4A7C15);

    for _ in 0..4000 {
        let len = 1 + rng.below(64);
        let mut buf: Vec<u8> = (0..len)
            .map(|_| {
                // Bias towards NUL so we get plenty of line breaks.
                match rng.below(4) {
                    0 | 1 => 0u8,
                    _ => (rng.below(255) + 1) as u8,
                }
            })
            .collect();
        let size = rng.below(len + 2); // may exceed nothing; <= len+1 is unsafe
        let size = size.min(len);
        let num_lines = rng.below(len + 3);
        compare_and_check_offsets(&c, &r, &mut buf, num_lines, size);
    }
}
