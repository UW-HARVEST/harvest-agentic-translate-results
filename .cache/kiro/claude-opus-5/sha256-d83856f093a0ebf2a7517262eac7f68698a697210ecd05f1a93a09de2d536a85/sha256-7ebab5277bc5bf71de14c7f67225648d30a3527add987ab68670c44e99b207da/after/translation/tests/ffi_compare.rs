//! Differential tests: load the C `.so` and the Rust `cdylib` with `libloading`
//! and compare `hdr_compare` results through the FFI boundary only.
//!
//! Nothing here calls the Rust crate directly, so the `#[no_mangle]` export
//! wrapper is exercised exactly as an external C caller would exercise it.

use std::ffi::c_int;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

use libloading::{Library, Symbol};

type HdrCompareFn = unsafe extern "C" fn(*const u8, *const u8) -> c_int;

/// Workspace root: parent of `translation/`.
fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation/ must have a parent")
        .to_path_buf()
}

fn find_so(dir: &Path) -> Option<PathBuf> {
    let entries = std::fs::read_dir(dir).ok()?;
    let mut found: Vec<PathBuf> = entries
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().map(|x| x == "so").unwrap_or(false))
        .collect();
    found.sort();
    found.into_iter().next()
}

/// Build `c_src` with CMake (only if the shared object is not already there)
/// and return the path to the resulting `.so`.
fn c_lib_path() -> PathBuf {
    let c_src = workspace_root().join("c_src");
    let build = c_src.join("build");

    if let Some(p) = find_so(&build) {
        return p;
    }

    std::fs::create_dir_all(&build).expect("create c_src/build");
    let status = Command::new("cmake")
        .current_dir(&build)
        .args(["..", "-DCMAKE_POSITION_INDEPENDENT_CODE=ON"])
        .status()
        .expect("run cmake configure");
    assert!(status.success(), "cmake configure failed");
    let status = Command::new("cmake")
        .current_dir(&build)
        .args(["--build", "."])
        .status()
        .expect("run cmake build");
    assert!(status.success(), "cmake build failed");

    find_so(&build).expect("no .so produced in c_src/build")
}

/// Build the Rust `cdylib` into a dedicated target directory so that this
/// nested `cargo` invocation never contends with the outer `cargo test` lock,
/// then return the path to the produced `.so`.
fn rust_lib_path() -> PathBuf {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    let target_dir = manifest.join("target").join("ffi-selftest");

    let mut cmd = Command::new(std::env::var("CARGO").unwrap_or_else(|_| "cargo".into()));
    cmd.current_dir(manifest)
        .arg("build")
        .arg("--release")
        .arg("--target-dir")
        .arg(&target_dir);

    // Reproduce the feature selection this test binary was compiled with so
    // that the .so under test matches the configuration being tested.
    cmd.arg("--no-default-features");
    let feats = enabled_features();
    if !feats.is_empty() {
        cmd.arg("--features").arg(feats.join(","));
    }

    // Avoid inheriting the outer cargo's job-server / profile overrides.
    cmd.env_remove("CARGO_MAKEFLAGS");
    cmd.env_remove("RUSTC_WORKSPACE_WRAPPER");

    let status = cmd.status().expect("run cargo build for cdylib");
    assert!(status.success(), "cargo build --release of cdylib failed");

    let dir = target_dir.join("release");
    let p = dir.join("libhdr_compare_lib.so");
    assert!(p.exists(), "expected cdylib at {}", p.display());
    p
}

/// Features enabled for this test binary. `Cargo.toml` declares no
/// `[features]` table, so the only valid combination is the empty one; this
/// helper keeps the harness correct if features are ever added.
fn enabled_features() -> Vec<String> {
    let mut v = Vec::new();
    for (k, _) in std::env::vars() {
        if let Some(rest) = k.strip_prefix("CARGO_FEATURE_") {
            v.push(rest.to_ascii_lowercase().replace('_', "-"));
        }
    }
    v.sort();
    v
}

struct Libs {
    _c: Library,
    _r: Library,
    c_hdr_compare: HdrCompareFn,
    r_hdr_compare: HdrCompareFn,
    c_path: PathBuf,
    r_path: PathBuf,
}

// Raw `extern "C"` function pointers and `Library` handles are safe to share:
// the libraries are never unloaded and the C code is stateless.
unsafe impl Send for Libs {}
unsafe impl Sync for Libs {}

fn libs() -> &'static Libs {
    static LIBS: OnceLock<Libs> = OnceLock::new();
    LIBS.get_or_init(|| {
        let c_path = c_lib_path();
        let r_path = rust_lib_path();

        unsafe {
            let c = Library::new(&c_path).expect("dlopen C .so");
            let r = Library::new(&r_path).expect("dlopen Rust .so");

            let cs: Symbol<HdrCompareFn> =
                c.get(b"hdr_compare\0").expect("hdr_compare in C .so");
            let rs: Symbol<HdrCompareFn> =
                r.get(b"hdr_compare\0").expect("hdr_compare in Rust .so");
            let c_hdr_compare = *cs;
            let r_hdr_compare = *rs;

            Libs {
                _c: c,
                _r: r,
                c_hdr_compare,
                r_hdr_compare,
                c_path,
                r_path,
            }
        }
    })
}

/// Call both libraries with the same inputs and require byte-identical
/// `c_int` results. Buffers are 4 bytes so no read can go out of bounds
/// regardless of how far the short-circuit evaluation gets.
#[inline]
fn check(l: &Libs, h1: [u8; 4], h2: [u8; 4], mismatches: &mut Vec<String>) {
    let c = unsafe { (l.c_hdr_compare)(h1.as_ptr(), h2.as_ptr()) };
    let r = unsafe { (l.r_hdr_compare)(h1.as_ptr(), h2.as_ptr()) };
    if c != r && mismatches.len() < 32 {
        mismatches.push(format!(
            "hdr_compare(h1={:02x?}, h2={:02x?}): C={} Rust={}",
            &h1[..3],
            &h2[..3],
            c,
            r
        ));
    }
}

fn report(mismatches: Vec<String>, label: &str) {
    assert!(
        mismatches.is_empty(),
        "{} produced {} mismatch(es):\n{}",
        label,
        mismatches.len(),
        mismatches.join("\n")
    );
}

/// Hand-picked headers plus boundary values, used as the "other operand"
/// during the exhaustive `h2` sweep.
const H1_PROBES: [[u8; 4]; 4] = [
    [0xff, 0xfb, 0x90, 0x00], // MPEG1 Layer3, 128 kbps 44.1 kHz
    [0x00, 0xe2, 0x00, 0x00], // MPEG2.5-style byte 1, zero byte 2
    [0xaa, 0x00, 0xff, 0x00], // byte 1 = 0, byte 2 all ones
    [0xff, 0xff, 0x0c, 0x00], // saturated fields / bitrate index 0
];

/// `hdr_valid` inspects only `h2`, so sweeping all 2^24 possible `h2` values
/// exercises every one of its branches exhaustively.
#[test]
fn exhaustive_h2_24bit_space() {
    let l = libs();
    let mut bad = Vec::new();
    for v in 0u32..(1 << 24) {
        let h2 = [(v >> 16) as u8, (v >> 8) as u8, v as u8, 0];
        for h1 in H1_PROBES {
            check(l, h1, h2, &mut bad);
        }
    }
    report(bad, "exhaustive 24-bit h2 sweep");
}

/// Byte-1 comparison `(h1[1] ^ h2[1]) & 0xFE`: sweeps `h1[1]` and `h2[1]`
/// over every value while varying both byte-2 operands across a set that hits
/// every relevant bit pattern of the `0x0C` and `0xF0` masks.
#[test]
fn exhaustive_header_byte1_cross_product() {
    let l = libs();
    let mut bad = Vec::new();
    const B2: [u8; 8] = [0x00, 0x04, 0x08, 0x0c, 0x90, 0x94, 0x98, 0x9c];
    for a1 in 0u16..256 {
        for b1 in 0u16..256 {
            for a2 in B2 {
                for b2 in B2 {
                    check(
                        l,
                        [0xff, a1 as u8, a2, 0],
                        [0xff, b1 as u8, b2, 0],
                        &mut bad,
                    );
                }
            }
        }
    }
    report(bad, "exhaustive byte-1 cross product");
}

/// Byte-2 comparison `(h1[2] ^ h2[2]) & 0x0C` plus the
/// `(h1[2] & 0xF0) == 0` vs `(h2[2] & 0xF0) == 0` XOR: sweeps `h1[2]` and
/// `h2[2]` over every value while varying the byte-1 operands across valid,
/// invalid and boundary MPEG byte-1 encodings.
#[test]
fn exhaustive_header_byte2_cross_product() {
    let l = libs();
    let mut bad = Vec::new();
    const B1: [u8; 8] = [0x00, 0xe2, 0xe3, 0xf0, 0xf2, 0xf8, 0xfa, 0xff];
    for a2 in 0u16..256 {
        for b2 in 0u16..256 {
            for a1 in B1 {
                for b1 in B1 {
                    check(
                        l,
                        [0xff, a1, a2 as u8, 0],
                        [0xff, b1, b2 as u8, 0],
                        &mut bad,
                    );
                }
            }
        }
    }
    report(bad, "exhaustive byte-2 cross product");
}

/// Exhaustive sweep of *every* `(h1[1], h1[2], h2[1], h2[2])` quadruple
/// (2^32 pairs) with `h2[0] = 0xff` so `hdr_valid` always proceeds past the
/// sync byte. Together with `exhaustive_h2_24bit_space` (which covers every
/// `h2[0]`) this covers the complete effective input domain of
/// `hdr_compare`, since `h1[0]` is never read.
#[test]
fn exhaustive_all_quadruples() {
    let l = libs();
    const THREADS: u16 = 8;
    let mut bad = Vec::new();
    let results: Vec<Vec<String>> = std::thread::scope(|scope| {
        let handles: Vec<_> = (0..THREADS)
            .map(|t| {
                scope.spawn(move || {
                    let l = libs();
                    let mut local = Vec::new();
                    let mut b1 = t;
                    while b1 < 256 {
                        for b2 in 0u16..256 {
                            for a1 in 0u16..256 {
                                for a2 in 0u16..256 {
                                    check(
                                        l,
                                        [0xff, a1 as u8, a2 as u8, 0],
                                        [0xff, b1 as u8, b2 as u8, 0],
                                        &mut local,
                                    );
                                }
                            }
                        }
                        b1 += THREADS;
                    }
                    local
                })
            })
            .collect();
        handles.into_iter().map(|h| h.join().unwrap()).collect()
    });
    for r in results {
        bad.extend(r);
    }
    let _ = l;
    report(bad, "exhaustive sweep of all header quadruples");
}

/// Uniform random sampling of the whole 2^48 input space (both 3-byte
/// headers fully random), plus a `h2[0] = 0xff` biased half so that a large
/// share of samples actually get past the sync-byte check.
#[test]
fn randomized_full_input_space() {
    let l = libs();
    let mut bad = Vec::new();
    let mut s: u64 = 0x243f_6a88_85a3_08d3;
    let mut next = move || {
        s ^= s << 13;
        s ^= s >> 7;
        s ^= s << 17;
        s
    };
    for i in 0..20_000_000u32 {
        let v = next();
        let mut h1 = [v as u8, (v >> 8) as u8, (v >> 16) as u8, 0];
        let mut h2 = [(v >> 24) as u8, (v >> 32) as u8, (v >> 40) as u8, 0];
        if i % 2 == 0 {
            h2[0] = 0xff;
        }
        if i % 8 == 0 {
            // Force a compatible pair now and then to hit the "returns 1" path.
            h1[1] = h2[1] ^ ((v >> 48) as u8 & 0x01);
            h1[2] = h2[2];
        }
        check(l, h1, h2, &mut bad);
    }
    report(bad, "randomized full input space");
}

/// Explicit regression cases for the individual sub-conditions, kept separate
/// so a failure names the exact rule that diverged.
#[test]
fn targeted_edge_cases() {
    let l = libs();
    let mut bad = Vec::new();
    let cases: &[([u8; 4], [u8; 4])] = &[
        // h2[0] not the sync byte -> invalid regardless of everything else.
        ([0xff, 0xfb, 0x90, 0], [0xfe, 0xfb, 0x90, 0]),
        ([0xff, 0xfb, 0x90, 0], [0x00, 0xfb, 0x90, 0]),
        // Identical valid headers.
        ([0xff, 0xfb, 0x90, 0], [0xff, 0xfb, 0x90, 0]),
        // h2[1] == 0xe2 / 0xe3 branch of the validity test.
        ([0xff, 0xe2, 0x90, 0], [0xff, 0xe2, 0x90, 0]),
        ([0xff, 0xe3, 0x90, 0], [0xff, 0xe3, 0x90, 0]),
        // h2[1] rejected because ((h2[1] >> 1) & 3) == 0.
        ([0xff, 0xf0, 0x90, 0], [0xff, 0xf0, 0x90, 0]),
        ([0xff, 0xf1, 0x90, 0], [0xff, 0xf1, 0x90, 0]),
        ([0xff, 0xf8, 0x90, 0], [0xff, 0xf8, 0x90, 0]),
        // h2[1] rejected because neither mask matches.
        ([0xff, 0xe4, 0x90, 0], [0xff, 0xe4, 0x90, 0]),
        // h2[2] >> 4 == 15 -> invalid.
        ([0xff, 0xfb, 0xf0, 0], [0xff, 0xfb, 0xf0, 0]),
        // ((h2[2] >> 2) & 3) == 3 -> invalid.
        ([0xff, 0xfb, 0x0c, 0], [0xff, 0xfb, 0x0c, 0]),
        // Byte-1 differs only in bit 0 -> masked out by 0xFE, still equal.
        ([0xff, 0xfa, 0x90, 0], [0xff, 0xfb, 0x90, 0]),
        // Byte-1 differs above bit 0 -> mismatch.
        ([0xff, 0xf3, 0x90, 0], [0xff, 0xfb, 0x90, 0]),
        // Byte-2 differs inside the 0x0C mask -> mismatch.
        ([0xff, 0xfb, 0x94, 0], [0xff, 0xfb, 0x90, 0]),
        // Byte-2 differs only outside 0x0C but one side has 0xF0 == 0.
        ([0xff, 0xfb, 0x00, 0], [0xff, 0xfb, 0x90, 0]),
        ([0xff, 0xfb, 0x90, 0], [0xff, 0xfb, 0x00, 0]),
        // Both sides have 0xF0 == 0 -> the XOR term is satisfied.
        ([0xff, 0xfb, 0x00, 0], [0xff, 0xfb, 0x00, 0]),
        ([0xff, 0xfb, 0x02, 0], [0xff, 0xfb, 0x00, 0]),
        // h1 is never validated: garbage h1 with a valid, matching h2.
        ([0x00, 0xfb, 0x90, 0], [0xff, 0xfb, 0x90, 0]),
        // All-zero and all-ones extremes.
        ([0x00, 0x00, 0x00, 0], [0x00, 0x00, 0x00, 0]),
        ([0xff, 0xff, 0xff, 0], [0xff, 0xff, 0xff, 0]),
    ];
    for &(h1, h2) in cases {
        check(l, h1, h2, &mut bad);
    }
    report(bad, "targeted edge cases");
}

/// Every dynamic symbol exported by the C `.so` must also be exported by the
/// Rust `.so` under the exact same name.
#[test]
fn exported_symbols_superset() {
    let l = libs();
    let c = dynamic_symbols(&l.c_path);
    let r = dynamic_symbols(&l.r_path);
    assert!(
        !c.is_empty(),
        "no dynamic symbols found in {}",
        l.c_path.display()
    );
    let missing: Vec<&String> = c.iter().filter(|s| !r.contains(*s)).collect();
    assert!(
        missing.is_empty(),
        "Rust .so is missing symbols exported by the C .so: {:?}\nC: {:?}\nRust: {:?}",
        missing,
        c,
        r
    );
}

/// Global, defined dynamic symbols of a shared object, excluding the
/// toolchain-provided runtime entries that are not part of the library's API.
fn dynamic_symbols(path: &Path) -> std::collections::BTreeSet<String> {
    let out = Command::new("nm")
        .args(["-D", "--defined-only"])
        .arg(path)
        .output()
        .expect("run nm");
    assert!(out.status.success(), "nm failed on {}", path.display());
    let text = String::from_utf8_lossy(&out.stdout);
    let mut set = std::collections::BTreeSet::new();
    for line in text.lines() {
        let mut it = line.split_whitespace();
        let (kind, name) = match (it.next(), it.next(), it.next()) {
            (Some(_addr), Some(k), Some(n)) => (k, n),
            (Some(k), Some(n), None) => (k, n),
            _ => continue,
        };
        if !matches!(kind, "T" | "D" | "B" | "R" | "W" | "V" | "i" | "G" | "S") {
            continue;
        }
        // Runtime/linker-provided symbols, not part of the translated API.
        if matches!(
            name,
            "_init" | "_fini" | "_edata" | "_end" | "__bss_start" | "_IO_stdin_used"
        ) {
            continue;
        }
        set.insert(name.to_string());
    }
    set
}
