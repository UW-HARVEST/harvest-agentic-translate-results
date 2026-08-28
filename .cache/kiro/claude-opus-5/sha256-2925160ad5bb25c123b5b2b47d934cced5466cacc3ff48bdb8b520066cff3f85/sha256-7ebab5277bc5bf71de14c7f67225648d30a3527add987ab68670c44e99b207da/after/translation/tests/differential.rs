//! Differential test: loads BOTH the C shared library and the Rust cdylib via
//! `libloading` and compares the result of every exported symbol through the
//! FFI boundary. The Rust side is never called directly, so the `#[no_mangle]`
//! export wrapper is exercised too.

use libloading::{Library, Symbol};
use std::ffi::c_int;
use std::path::PathBuf;
use std::process::Command;

type DataEntryFn = unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int;

fn repo_root() -> PathBuf {
    // CARGO_MANIFEST_DIR == <root>/translation
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation/ must have a parent")
        .to_path_buf()
}

/// Locate (building if necessary) the C shared library produced from c_src/.
fn c_library_path() -> PathBuf {
    let build_dir = repo_root().join("c_src").join("build");

    if find_so(&build_dir).is_none() {
        std::fs::create_dir_all(&build_dir).expect("create c_src/build");
        let ok = Command::new("cmake")
            .current_dir(&build_dir)
            .args(["..", "-DCMAKE_POSITION_INDEPENDENT_CODE=ON"])
            .status()
            .map(|s| s.success())
            .unwrap_or(false);
        assert!(ok, "cmake configure of c_src failed");
        let ok = Command::new("cmake")
            .current_dir(&build_dir)
            .args(["--build", "."])
            .status()
            .map(|s| s.success())
            .unwrap_or(false);
        assert!(ok, "cmake build of c_src failed");
    }

    find_so(&build_dir).expect("no .so produced in c_src/build")
}

fn find_so(dir: &PathBuf) -> Option<PathBuf> {
    let entries = std::fs::read_dir(dir).ok()?;
    for e in entries.flatten() {
        let p = e.path();
        if p.extension().map(|x| x == "so").unwrap_or(false) {
            return Some(p);
        }
    }
    None
}

/// Locate the Rust cdylib. `cargo test` does not necessarily build the cdylib
/// target, so build it on demand if it is missing.
fn rust_library_path() -> PathBuf {
    let mut dir = std::env::current_exe().expect("current_exe");
    dir.pop(); // strip test binary name -> target/<profile>/deps
    let name = format!(
        "{}dataentry_lib{}",
        std::env::consts::DLL_PREFIX,
        std::env::consts::DLL_SUFFIX
    );

    let mut profile_dir = dir.clone();
    if profile_dir.file_name().map(|n| n == "deps").unwrap_or(false) {
        profile_dir.pop();
    }
    let is_release = profile_dir
        .file_name()
        .map(|n| n == "release")
        .unwrap_or(false);

    let candidates = [profile_dir.join(&name), dir.join(&name)];
    for c in candidates.iter() {
        if c.exists() {
            return c.clone();
        }
    }

    // Not built yet: build the cdylib target explicitly.
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());
    let mut cmd = Command::new(cargo);
    cmd.current_dir(env!("CARGO_MANIFEST_DIR"))
        .args(["build", "--lib", "--no-default-features"]);
    if is_release {
        cmd.arg("--release");
    }
    let ok = cmd.status().map(|s| s.success()).unwrap_or(false);
    assert!(ok, "failed to build the Rust cdylib");

    for c in candidates.iter() {
        if c.exists() {
            return c.clone();
        }
    }
    panic!("could not find Rust cdylib {name} near {profile_dir:?}");
}

struct Pair {
    _c_lib: Library,
    _rust_lib: Library,
    c: DataEntryFn,
    rust: DataEntryFn,
}

impl Pair {
    fn load() -> Self {
        let c_path = c_library_path();
        let r_path = rust_library_path();

        unsafe {
            let c_lib = Library::new(&c_path).expect("load C .so");
            let rust_lib = Library::new(&r_path).expect("load Rust .so");

            let c_sym: Symbol<DataEntryFn> =
                c_lib.get(b"dataentry\0").expect("C .so exports dataentry");
            let r_sym: Symbol<DataEntryFn> = rust_lib
                .get(b"dataentry\0")
                .expect("Rust .so exports dataentry");

            let c = *c_sym;
            let rust = *r_sym;

            Pair {
                _c_lib: c_lib,
                _rust_lib: rust_lib,
                c,
                rust,
            }
        }
    }

    #[track_caller]
    fn check(&self, mode: c_int, p1: c_int, p2: c_int, p3: c_int) {
        let expected = unsafe { (self.c)(mode, p1, p2, p3) };
        let actual = unsafe { (self.rust)(mode, p1, p2, p3) };
        assert_eq!(
            expected, actual,
            "dataentry({mode}, {p1}, {p2}, {p3}): C returned {expected}, Rust returned {actual}"
        );
    }
}

/// Both libraries must export the symbol under the exact same name.
#[test]
fn exports_match() {
    let p = Pair::load();
    // Loading already asserted both `dataentry` symbols resolve; touch them so
    // the symbols are definitely used.
    p.check(3, 0, 0, 0);
}

/// mode 1: create_entries(count, 100) -> find_entry -> value / -2
#[test]
fn mode1_find_entry() {
    let p = Pair::load();
    for p1 in [-3i32, -1, 0, 1, 2, 3, 5, 7, 10, 11, 64, 257] {
        for p2 in [-5i32, -1, 0, 1, 2, 4, 5, 6, 9, 10, 63, 256, 1000] {
            for p3 in [0i32, 1, -7, 12345] {
                p.check(1, p1, p2, p3);
            }
        }
    }
}

/// mode 2: create_entries(count, 200) -> modify_entries -> total (+param3)
#[test]
fn mode2_modify_entries() {
    let p = Pair::load();
    for p1 in [-3i32, -1, 0, 1, 2, 3, 4, 8, 17, 100] {
        for p2 in [-3i32, -1, 0, 1, 2, 3, 7, 1000, -1000] {
            for p3 in [0i32, 1, -1, 500, -500, i32::MAX, i32::MIN] {
                p.check(2, p1, p2, p3);
            }
        }
    }
}

/// mode 3: lookup_table bounds + calculate_lookup
#[test]
fn mode3_lookup() {
    let p = Pair::load();
    for p1 in -3i32..8 {
        for p2 in -3i32..7 {
            for p3 in [0i32, 1, -1, 7, -7, 100000] {
                p.check(3, p1, p2, p3);
            }
        }
    }
}

/// default branch: process_name + strlen("TestName") * param1
#[test]
fn default_branch() {
    let p = Pair::load();
    for mode in [i32::MIN, -100, -2, -1, 0, 4, 5, 6, 99, i32::MAX] {
        for p1 in [-3i32, -1, 0, 1, 2, 3, 1000, -1000, 268435456] {
            for p2 in [0i32, 5] {
                for p3 in [0i32, 5] {
                    p.check(mode, p1, p2, p3);
                }
            }
        }
    }
}

/// Signed-overflow-adjacent values: the C build wraps, the Rust translation
/// must wrap identically.
#[test]
fn overflow_edges() {
    let p = Pair::load();
    let extremes = [i32::MIN, i32::MIN + 1, -2, -1, 0, 1, 2, i32::MAX - 1, i32::MAX];
    for mode in [1i32, 2, 3, 4] {
        for &p2 in extremes.iter() {
            for &p3 in extremes.iter() {
                // keep param1 small so allocation sizes stay sane
                for p1 in [-1i32, 0, 1, 3, 5] {
                    p.check(mode, p1, p2, p3);
                }
            }
        }
    }
}

/// Broad deterministic sweep across all branches.
#[test]
fn exhaustive_small_grid() {
    let p = Pair::load();
    for mode in -2i32..=6 {
        for p1 in -2i32..=12 {
            for p2 in -2i32..=12 {
                for p3 in [-1i32, 0, 3] {
                    p.check(mode, p1, p2, p3);
                }
            }
        }
    }
}

/// Larger allocation counts, still bounded so the test stays fast.
#[test]
fn larger_counts() {
    let p = Pair::load();
    for p1 in [1000i32, 4096, 10000, 65536] {
        p.check(1, p1, p1 - 1, 3);
        p.check(1, p1, p1, 3);
        p.check(2, p1, 2, 3);
        p.check(2, p1, 0, 3);
    }
}

/// Deterministic pseudo-random sweep (xorshift) over the full int range for
/// param2/param3 and a bounded range for mode/param1.
#[test]
fn randomized_sweep() {
    let p = Pair::load();
    let mut state: u64 = 0x2545_F491_4F6C_DD1D;
    let mut next = || {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        state
    };

    for _ in 0..60_000 {
        // mode: mostly in 1..=3 so the interesting branches dominate, but
        // sometimes arbitrary to hit the default branch.
        let r = next();
        let mode = if r % 4 == 0 {
            r as i32
        } else {
            (r % 5) as i32 - 1
        };
        // keep param1 bounded: it drives malloc sizes in modes 1 and 2.
        let p1 = (next() % 4096) as i32 - 64;
        let p2 = next() as i32;
        let p3 = next() as i32;
        p.check(mode, p1, p2, p3);
    }
}

/// `create_entries` allocates `count * sizeof(DataEntry)` bytes. For counts
/// large enough that the allocation cannot be satisfied, C returns NULL and
/// `dataentry` yields -1; the Rust translation must agree.
#[test]
fn allocation_failure_edges() {
    let p = Pair::load();
    // These sizes (>= ~40 GB) fail to allocate on both sides, quickly.
    for p1 in [i32::MAX, i32::MAX - 1, 1 << 30] {
        p.check(1, p1, 0, 3);
        p.check(2, p1, 2, 3);
    }
}

/// Same idea but with a count whose allocation actually succeeds (~10 GB).
/// Verified to match; ignored by default because it takes minutes.
#[test]
#[ignore = "allocates ~10 GB on both sides; slow"]
fn allocation_large_success() {
    let p = Pair::load();
    p.check(1, 1 << 28, 0, 3);
    p.check(2, 1 << 28, 2, 3);
}
