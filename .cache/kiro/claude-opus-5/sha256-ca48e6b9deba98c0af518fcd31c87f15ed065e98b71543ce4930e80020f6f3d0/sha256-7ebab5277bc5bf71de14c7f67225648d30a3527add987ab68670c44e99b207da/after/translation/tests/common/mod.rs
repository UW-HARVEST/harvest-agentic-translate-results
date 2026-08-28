//! Shared harness: loads the C reference `.so` and the Rust `.so` and exposes
//! their `merge_sort` exports so both can be driven through the FFI boundary.

use std::ffi::c_int;
use std::path::{Path, PathBuf};

use libloading::{Library, Symbol};

/// Byte-for-byte layout mirror of the C `spritebatch_sprite_t`.
///
/// The trailing 4 bytes of tail padding the C compiler inserts after
/// `sort_bits` are modelled as an explicit `_pad` field so that whole-struct
/// byte comparisons are well defined on the Rust side.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Sprite {
    pub texture_id: u64,
    pub sort_bits: c_int,
    pub _pad: u32,
}

impl Sprite {
    pub fn new(texture_id: u64, sort_bits: i32) -> Self {
        Sprite {
            texture_id,
            sort_bits,
            _pad: 0,
        }
    }
}

pub type MergeSortFn = unsafe extern "C" fn(*mut Sprite, *mut Sprite, c_int);

pub struct Impls {
    _c_lib: Library,
    _rust_lib: Library,
    pub c_merge_sort: MergeSortFn,
    pub rust_merge_sort: MergeSortFn,
    pub c_path: PathBuf,
    pub rust_path: PathBuf,
}

fn workspace_root() -> PathBuf {
    // CARGO_MANIFEST_DIR == <root>/translation
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("manifest dir has a parent")
        .to_path_buf()
}

/// The Rust cdylib sits next to the integration-test executable's target dir:
/// `target/<profile>/deps/<test>-<hash>` -> `target/<profile>/`.
fn rust_so_path() -> PathBuf {
    let exe = std::env::current_exe().expect("current_exe");
    let profile_dir = exe
        .parent()
        .and_then(|deps| deps.parent())
        .expect("target/<profile>")
        .to_path_buf();

    let candidate = profile_dir.join("libmerge_sort_lib.so");
    if candidate.exists() {
        return candidate;
    }

    // Fall back to scanning both profiles for a freshly built artifact.
    let target = Path::new(env!("CARGO_MANIFEST_DIR")).join("target");
    for p in ["release", "debug"] {
        let c = target.join(p).join("libmerge_sort_lib.so");
        if c.exists() {
            return c;
        }
    }
    panic!(
        "could not locate libmerge_sort_lib.so (looked in {})",
        profile_dir.display()
    );
}

/// The CMake project is named after the parent directory, so the artifact name
/// is not fixed: glob `c_src/build/*.so` instead of hard-coding it.
fn c_so_path() -> PathBuf {
    // Allow pointing at an alternative C build (e.g. a -O2 one) so the same
    // suite can be replayed against several compiler configurations.
    if let Some(p) = std::env::var_os("C_SO_PATH") {
        let p = PathBuf::from(p);
        assert!(p.exists(), "C_SO_PATH does not exist: {}", p.display());
        return p;
    }

    let build_dir = workspace_root().join("c_src").join("build");
    let mut found: Vec<PathBuf> = std::fs::read_dir(&build_dir)
        .unwrap_or_else(|e| panic!("read {}: {e} -- did you run cmake?", build_dir.display()))
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().map(|x| x == "so").unwrap_or(false))
        .collect();
    found.sort();
    assert_eq!(
        found.len(),
        1,
        "expected exactly one .so in {}, got {found:?}",
        build_dir.display()
    );
    found.pop().unwrap()
}

impl Impls {
    pub fn load() -> Self {
        // Layout sanity: if these ever diverge, every byte comparison below is
        // meaningless.
        assert_eq!(std::mem::size_of::<Sprite>(), 16);
        assert_eq!(std::mem::align_of::<Sprite>(), 8);

        let c_path = c_so_path();
        let rust_path = rust_so_path();

        unsafe {
            let c_lib = Library::new(&c_path)
                .unwrap_or_else(|e| panic!("load {}: {e}", c_path.display()));
            let rust_lib = Library::new(&rust_path)
                .unwrap_or_else(|e| panic!("load {}: {e}", rust_path.display()));

            let c_sym: Symbol<MergeSortFn> = c_lib
                .get(b"merge_sort\0")
                .expect("C .so exports merge_sort");
            let rust_sym: Symbol<MergeSortFn> = rust_lib
                .get(b"merge_sort\0")
                .expect("Rust .so exports merge_sort");

            let c_merge_sort = *c_sym;
            let rust_merge_sort = *rust_sym;

            Impls {
                _c_lib: c_lib,
                _rust_lib: rust_lib,
                c_merge_sort,
                rust_merge_sort,
                c_path,
                rust_path,
            }
        }
    }

    /// Runs `merge_sort` in both implementations over identical copies of
    /// `input` and returns `((c_a, c_b), (rust_a, rust_b))`.
    pub fn run_both(
        &self,
        input: &[Sprite],
        size: c_int,
    ) -> ((Vec<Sprite>, Vec<Sprite>), (Vec<Sprite>, Vec<Sprite>)) {
        let mut c_a = input.to_vec();
        let mut c_b = vec![Sprite::new(0, 0); input.len()];
        let mut r_a = input.to_vec();
        let mut r_b = vec![Sprite::new(0, 0); input.len()];

        unsafe {
            (self.c_merge_sort)(c_a.as_mut_ptr(), c_b.as_mut_ptr(), size);
            (self.rust_merge_sort)(r_a.as_mut_ptr(), r_b.as_mut_ptr(), size);
        }

        ((c_a, c_b), (r_a, r_b))
    }

    /// Differential check: both output buffers must agree byte-for-byte.
    pub fn assert_matches(&self, input: &[Sprite], size: c_int, label: &str) {
        let ((c_a, c_b), (r_a, r_b)) = self.run_both(input, size);

        assert_eq!(
            as_bytes(&c_a),
            as_bytes(&r_a),
            "[{label}] buffer `a` mismatch (size={size})\n  C:    {:?}\n  Rust: {:?}",
            summarize(&c_a),
            summarize(&r_a),
        );
        assert_eq!(
            as_bytes(&c_b),
            as_bytes(&r_b),
            "[{label}] buffer `b` mismatch (size={size})\n  C:    {:?}\n  Rust: {:?}",
            summarize(&c_b),
            summarize(&r_b),
        );
    }
}

pub fn as_bytes(v: &[Sprite]) -> &[u8] {
    unsafe { std::slice::from_raw_parts(v.as_ptr().cast::<u8>(), std::mem::size_of_val(v)) }
}

fn summarize(v: &[Sprite]) -> Vec<(u64, i32, u32)> {
    v.iter()
        .take(24)
        .map(|s| (s.texture_id, s.sort_bits, s._pad))
        .collect()
}

/// Deterministic xorshift64* PRNG so failures are reproducible.
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
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

    pub fn next_i32(&mut self) -> i32 {
        self.next_u64() as i32
    }

    /// Uniform-ish value in `0..n`.
    pub fn below(&mut self, n: u64) -> u64 {
        if n == 0 { 0 } else { self.next_u64() % n }
    }
}
