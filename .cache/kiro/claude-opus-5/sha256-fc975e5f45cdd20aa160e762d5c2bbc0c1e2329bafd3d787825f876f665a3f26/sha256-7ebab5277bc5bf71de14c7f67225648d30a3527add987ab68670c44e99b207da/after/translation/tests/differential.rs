//! Differential test: loads the C shared library and the Rust cdylib through
//! `libloading` and compares `pow43` bit-for-bit across its whole
//! well-defined input domain.
//!
//! Neither side is called directly; both go through `dlopen`/`dlsym`, so the
//! `#[no_mangle]` export wrapper is exercised exactly as an external C caller
//! would exercise it.

use libloading::{Library, Symbol};
use std::path::PathBuf;
use std::process::Command;
use std::sync::OnceLock;

type Pow43 = unsafe extern "C" fn(std::ffi::c_int) -> f32;

/// `<workspace>/` — the parent of the `translation/` crate directory.
fn workspace_root() -> PathBuf {
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.pop();
    p
}

/// Locate the C `.so`, building it with CMake the first time if necessary.
fn c_library_path() -> PathBuf {
    let build_dir = workspace_root().join("c_src").join("build");
    if find_shared_object(&build_dir).is_none() {
        build_c_library(&build_dir);
    }
    find_shared_object(&build_dir)
        .unwrap_or_else(|| panic!("no lib*.so found in {}", build_dir.display()))
}

fn find_shared_object(dir: &PathBuf) -> Option<PathBuf> {
    let mut found: Vec<PathBuf> = std::fs::read_dir(dir)
        .ok()?
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .map(|n| n.starts_with("lib") && n.ends_with(".so"))
                .unwrap_or(false)
        })
        .collect();
    found.sort();
    found.pop()
}

fn build_c_library(build_dir: &PathBuf) {
    std::fs::create_dir_all(build_dir).expect("create c_src/build");
    let configure = Command::new("cmake")
        .arg("..")
        .arg("-DCMAKE_POSITION_INDEPENDENT_CODE=ON")
        .current_dir(build_dir)
        .status();
    let compile = Command::new("cmake")
        .args(["--build", "."])
        .current_dir(build_dir)
        .status();
    assert!(
        matches!(configure, Ok(s) if s.success()) && matches!(compile, Ok(s) if s.success()),
        "failed to build the C library in {}. Build it manually:\n  \
         cd c_src && mkdir -p build && cd build && \
         cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
        build_dir.display()
    );
}

/// Locate the Rust cdylib for this test run. Integration-test binaries live in
/// `target/<profile>/deps/`, so the cdylib sits two levels up. A cdylib-only
/// crate is not built by `cargo test`, so build the `lib` target on demand.
fn rust_library_path() -> PathBuf {
    static BUILT: OnceLock<()> = OnceLock::new();

    let exe = std::env::current_exe().expect("current_exe");
    let profile_dir = exe
        .parent()
        .and_then(|deps| deps.parent())
        .expect("target/<profile>")
        .to_path_buf();

    let candidates = ["libpow43_lib.so", "libtranslation.so"];
    let locate = || {
        candidates
            .iter()
            .map(|n| profile_dir.join(n))
            .find(|p| p.exists())
    };

    if let Some(p) = locate() {
        return p;
    }

    BUILT.get_or_init(|| {
        // `cargo test` has released the build lock by the time tests run, so a
        // nested `cargo build` is safe here.
        let profile = profile_dir
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("debug");
        let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());
        let mut cmd = Command::new(cargo);
        cmd.arg("build")
            .arg("--lib")
            .current_dir(env!("CARGO_MANIFEST_DIR"));
        if profile != "debug" {
            cmd.args(["--profile", profile]);
        }
        let status = cmd.status();
        assert!(
            matches!(status, Ok(s) if s.success()),
            "failed to build the Rust cdylib"
        );
    });

    locate().unwrap_or_else(|| {
        panic!(
            "Rust cdylib not found in {} after building",
            profile_dir.display()
        )
    })
}

struct Libs {
    c: Library,
    rust: Library,
}

impl Libs {
    fn open() -> Self {
        unsafe {
            Libs {
                c: Library::new(c_library_path()).expect("dlopen C library"),
                rust: Library::new(rust_library_path()).expect("dlopen Rust cdylib"),
            }
        }
    }

    fn pow43(&self) -> (Symbol<'_, Pow43>, Symbol<'_, Pow43>) {
        unsafe {
            (
                self.c.get(b"pow43\0").expect("C pow43"),
                self.rust.get(b"pow43\0").expect("Rust pow43"),
            )
        }
    }
}

/// Compare raw IEEE-754 bits so that signed zeros and NaN payloads are also
/// required to agree, not just numeric equality.
fn assert_same(x: i32, c: f32, r: f32) {
    if c.to_bits() != r.to_bits() {
        panic!(
            "pow43({x}) mismatch: C = {c} (bits {:#010x}), Rust = {r} (bits {:#010x})",
            c.to_bits(),
            r.to_bits()
        );
    }
}

/// Largest `x` for which the C code's table index stays inside `g_pow43`
/// (145 entries, valid indices `0..=144`).
///
/// * `x < 129`  -> index `16 + x`, in bounds for `x >= -16`.
/// * `x < 1024` -> `x <<= 3` giving at most `8184`, so index `<= 128`. Always
///   in bounds.
/// * otherwise  -> index `16 + ((x + sign) >> 6)`, which needs
///   `x + sign <= 8255`. `sign` is `64` exactly when bit 5 of `x` is set, so
///   `x = 8224` already indexes entry 145 and runs off the end.
///
/// Past `8223` the C reads beyond a `static` array. That is undefined
/// behaviour with no defined value to match, so the sweeps stop here.
const DOMAIN_LO: i32 = -16;
const DOMAIN_HI: i32 = 8223;

#[test]
fn exhaustive_over_defined_domain() {
    let libs = Libs::open();
    let (c, r) = libs.pow43();
    for x in DOMAIN_LO..=DOMAIN_HI {
        let (cv, rv) = unsafe { (c(x), r(x)) };
        assert_same(x, cv, rv);
    }
}

/// The three branches of `pow43` and the exact indices where control flow
/// changes, called out separately so a failure names the boundary.
#[test]
fn branch_boundaries() {
    let libs = Libs::open();
    let (c, r) = libs.pow43();
    let cases = [
        -16, -15, -1, 0, 1, 2, 15, 16, 17, // direct table lookup
        63, 64, 65, 127, 128, // last lookup value
        129, 130, 131, // first shifted value
        191, 192, 193, 255, 256, 257, // sign-bit flips within the shifted path
        1022, 1023, // last shifted value
        1024, 1025, // first unshifted value
        2047, 2048, 4095, 4096, 8191, 8192, 8222, 8223, // last in-bounds value
    ];
    for x in cases {
        let (cv, rv) = unsafe { (c(x), r(x)) };
        assert_same(x, cv, rv);
    }
}

/// `sign = 2 * x & 64` selects between the two interpolation directions. Sweep
/// every residue of `x` modulo 64 in both the shifted and unshifted paths so
/// both settings of `sign` are covered at many magnitudes.
#[test]
fn every_residue_mod_64() {
    let libs = Libs::open();
    let (c, r) = libs.pow43();
    for base in [128, 256, 512, 1024, 2048, 4096, 8128] {
        for delta in 0..64 {
            let x = base + delta;
            if !(DOMAIN_LO..=DOMAIN_HI).contains(&x) {
                continue;
            }
            let (cv, rv) = unsafe { (c(x), r(x)) };
            assert_same(x, cv, rv);
        }
    }
}

/// Both libraries must export `pow43` under that exact name and nothing the
/// other one hides: the `static` table stays internal on both sides.
#[test]
fn exported_symbols_match() {
    let libs = Libs::open();
    let _ = libs.pow43(); // both `dlsym` lookups must succeed

    for lib in [c_library_path(), rust_library_path()] {
        unsafe {
            let handle = Library::new(&lib).expect("dlopen");
            assert!(
                handle.get::<Pow43>(b"pow43\0").is_ok(),
                "{} does not export pow43",
                lib.display()
            );
            assert!(
                handle.get::<*const f32>(b"g_pow43\0").is_err(),
                "{} unexpectedly exports the private table g_pow43",
                lib.display()
            );
        }
    }
}
