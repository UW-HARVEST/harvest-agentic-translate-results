//! Shared harness: loads the C and Rust shared objects via `libloading` and
//! exposes a differential-testing helper for `wcscat`.
//!
//! Both implementations are reached *only* through their exported dynamic
//! symbols, so the `#[no_mangle]` wrapper is part of what gets tested.

use std::path::{Path, PathBuf};

use libloading::{Library, Symbol};

/// `wchar_t` is a 32-bit signed int on Linux/glibc.
pub type WcharT = i32;

/// Signature of the exported symbol under test.
pub type WcscatFn = unsafe extern "C" fn(*mut WcharT, usize, *const WcharT) -> i32;

/// Repository root (parent of the `translation` crate directory).
fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("crate dir has a parent")
        .to_path_buf()
}

fn find_so_in(dir: &Path) -> Option<PathBuf> {
    let entries = std::fs::read_dir(dir).ok()?;
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) == Some("so") && path.is_file() {
            return Some(path);
        }
    }
    None
}

/// Path to the C shared library produced by CMake in `c_src/build`.
pub fn c_so_path() -> PathBuf {
    let build = repo_root().join("c_src").join("build");
    find_so_in(&build).unwrap_or_else(|| {
        panic!(
            "no .so found in {}; build the C library first:\n  \
             cd c_src && mkdir -p build && cd build && \
             cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
            build.display()
        )
    })
}

/// Path to the Rust `cdylib`.
///
/// The test executable lives in `<target>/<profile>/deps/`, so the cdylib sits
/// one directory up. Fall back to the conventional locations if that lookup
/// fails (e.g. a custom `--target-dir` layout).
pub fn rust_so_path() -> PathBuf {
    const LIB_NAME: &str = "libwcscat_lib.so";

    let mut candidates: Vec<PathBuf> = Vec::new();

    if let Ok(exe) = std::env::current_exe() {
        // .../<profile>/deps/<test-bin>
        if let Some(deps) = exe.parent() {
            candidates.push(deps.join(LIB_NAME));
            if let Some(profile) = deps.parent() {
                candidates.push(profile.join(LIB_NAME));
            }
        }
    }

    let target = repo_root().join("translation").join("target");
    candidates.push(target.join("release").join(LIB_NAME));
    candidates.push(target.join("debug").join(LIB_NAME));

    for candidate in &candidates {
        if candidate.is_file() {
            return candidate.clone();
        }
    }

    panic!(
        "could not locate {LIB_NAME}; searched:\n{}",
        candidates
            .iter()
            .map(|p| format!("  {}", p.display()))
            .collect::<Vec<_>>()
            .join("\n")
    );
}

/// A loaded implementation of the library under test.
pub struct Impl {
    /// Kept alive so the loaded symbol stays valid.
    _lib: Library,
    func: WcscatFn,
    pub name: &'static str,
}

impl Impl {
    fn load(path: &Path, name: &'static str) -> Self {
        // SAFETY: loading a shared object built from the sources in this repo.
        let lib = unsafe { Library::new(path) }
            .unwrap_or_else(|e| panic!("failed to dlopen {}: {e}", path.display()));

        // SAFETY: the symbol's ABI matches `WcscatFn`.
        let func: Symbol<WcscatFn> = unsafe { lib.get(b"wcscat\0") }
            .unwrap_or_else(|e| panic!("`wcscat` missing from {}: {e}", path.display()));
        let func = *func;

        Impl {
            _lib: lib,
            func,
            name,
        }
    }

    /// Invokes the exported `wcscat` on a copy of `dst`.
    ///
    /// Returns the return code alongside the resulting buffer contents. `src`
    /// of `None` passes a null pointer; `num_elem` is passed through verbatim
    /// so out-of-range capacities can be exercised deliberately.
    pub fn call(
        &self,
        dst: &[WcharT],
        num_elem: usize,
        src: Option<&[WcharT]>,
        dst_null: bool,
    ) -> (i32, Vec<WcharT>) {
        let mut buf = dst.to_vec();
        let dst_ptr = if dst_null {
            std::ptr::null_mut()
        } else {
            buf.as_mut_ptr()
        };
        let src_ptr = match src {
            Some(s) => s.as_ptr(),
            None => std::ptr::null(),
        };

        // SAFETY: `num_elem` never exceeds `buf.len()` in the callers below
        // unless the case explicitly targets the null/zero early-return paths,
        // which touch no memory.
        let ret = unsafe { (self.func)(dst_ptr, num_elem, src_ptr) };
        (ret, buf)
    }
}

/// Both implementations, ready for differential comparison.
pub struct Pair {
    pub c: Impl,
    pub rust: Impl,
}

pub fn load_pair() -> Pair {
    Pair {
        c: Impl::load(&c_so_path(), "C"),
        rust: Impl::load(&rust_so_path(), "Rust"),
    }
}

impl Pair {
    /// Runs one case against both libraries and asserts byte-identical results.
    pub fn check(
        &self,
        label: &str,
        dst: &[WcharT],
        num_elem: usize,
        src: Option<&[WcharT]>,
        dst_null: bool,
    ) {
        let (c_ret, c_buf) = self.c.call(dst, num_elem, src, dst_null);
        let (r_ret, r_buf) = self.rust.call(dst, num_elem, src, dst_null);

        assert_eq!(
            c_ret, r_ret,
            "[{label}] return code mismatch: {}={c_ret} {}={r_ret}\n  \
             dst_in={dst:?} num_elem={num_elem} src={src:?} dst_null={dst_null}",
            self.c.name, self.rust.name
        );

        assert_eq!(
            bytes_of(&c_buf),
            bytes_of(&r_buf),
            "[{label}] destination buffer mismatch:\n  {}   ={c_buf:?}\n  {}={r_buf:?}\n  \
             dst_in={dst:?} num_elem={num_elem} src={src:?} dst_null={dst_null}",
            self.c.name, self.rust.name
        );
    }
}

/// Raw byte view of a `wchar_t` slice, for byte-for-byte comparison.
pub fn bytes_of(v: &[WcharT]) -> Vec<u8> {
    v.iter().flat_map(|w| w.to_ne_bytes()).collect()
}

/// A deterministic xorshift PRNG so fuzz cases are reproducible.
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Rng(seed | 1)
    }

    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }

    pub fn below(&mut self, n: usize) -> usize {
        (self.next_u64() % (n as u64)) as usize
    }
}
