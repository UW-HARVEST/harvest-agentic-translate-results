//! Differential test: loads the C shared library and the Rust cdylib via
//! `libloading` and compares the exported `jumpnode` symbol byte-for-byte.
//!
//! Neither side is called directly as a Rust function; both go through the
//! dynamic-linker/FFI boundary exactly as an external caller would, so the
//! `#[no_mangle]` export wrapper is exercised too.

use libloading::{Library, Symbol};
use std::path::{Path, PathBuf};

type JumpNodeFn = unsafe extern "C" fn(
    operation_mode: std::ffi::c_int,
    node_id: std::ffi::c_int,
    depth: std::ffi::c_int,
    flags: std::ffi::c_int,
) -> std::ffi::c_int;

fn workspace_root() -> PathBuf {
    // CARGO_MANIFEST_DIR == <root>/translation
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation/ must have a parent")
        .to_path_buf()
}

/// Locate the C shared library produced by the CMake build. The CMake project
/// name is derived from the parent directory name, so the file name is not
/// fixed; glob for any `lib*.so` in `c_src/build`.
fn c_library_path() -> PathBuf {
    let build_dir = workspace_root().join("c_src").join("build");
    assert!(
        build_dir.is_dir(),
        "C build directory {} not found. Build it with:\n  cd c_src && mkdir -p build && cd build \
         && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
        build_dir.display()
    );

    let mut candidates: Vec<PathBuf> = std::fs::read_dir(&build_dir)
        .expect("readable c_src/build")
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| {
            p.is_file()
                && p.extension().map(|e| e == "so").unwrap_or(false)
                && p.file_name()
                    .and_then(|n| n.to_str())
                    .map(|n| n.starts_with("lib"))
                    .unwrap_or(false)
        })
        .collect();
    candidates.sort();

    candidates
        .pop()
        .unwrap_or_else(|| panic!("no lib*.so found in {}", build_dir.display()))
}

/// Locate the Rust cdylib. Prefers the profile the tests were built under and
/// falls back to the other one, so the harness works after either
/// `cargo test` or `cargo test --release`.
fn rust_library_paths() -> Vec<PathBuf> {
    let target = workspace_root().join("translation").join("target");
    let name = "libjumpnode_lib.so";

    let mut found = Vec::new();
    for profile in ["debug", "release"] {
        let p = target.join(profile).join(name);
        if p.is_file() {
            found.push(p);
        }
    }
    assert!(
        !found.is_empty(),
        "no Rust cdylib found under {}; run `cargo build` first",
        target.display()
    );
    found
}

struct Loaded {
    _lib: Library,
    jumpnode: JumpNodeFn,
    path: PathBuf,
}

impl Loaded {
    fn open(path: &Path) -> Loaded {
        let lib = unsafe { Library::new(path) }
            .unwrap_or_else(|e| panic!("failed to dlopen {}: {e}", path.display()));
        let sym: Symbol<JumpNodeFn> = unsafe { lib.get(b"jumpnode\0") }
            .unwrap_or_else(|e| panic!("`jumpnode` not exported by {}: {e}", path.display()));
        let f = *sym;
        Loaded {
            _lib: lib,
            jumpnode: f,
            path: path.to_path_buf(),
        }
    }

    fn call(&self, a: i32, b: i32, c: i32, d: i32) -> i32 {
        unsafe { (self.jumpnode)(a, b, c, d) }
    }
}

/// Interesting scalar values: octal case selectors, boundary ints, and values
/// that stress the `sprintf`/`strlen` path in operation mode 0003.
fn interesting_values() -> Vec<i32> {
    vec![
        i32::MIN,
        i32::MIN + 1,
        -2147483647,
        -1000000000,
        -100000,
        -1000,
        -128,
        -127,
        -100,
        -64,
        -10,
        -8,
        -3,
        -2,
        -1,
        0,
        1,
        2,
        3,
        4,
        5,
        6,
        7,
        8,
        9,
        10,
        15,
        16,
        17,
        20,
        31,
        32,
        63,
        64,
        99,
        100,
        127,
        128,
        255,
        256,
        1000,
        0o177,
        0o200,
        0o377,
        0o400,
        65535,
        65536,
        100000,
        1000000,
        999999999,
        1000000000,
        2147483646,
        i32::MAX,
    ]
}

/// Every operation mode the C `switch` distinguishes, plus values that must
/// land in `default`.
fn operation_modes() -> Vec<i32> {
    let mut v = vec![
        0o0, 0o1, 0o2, 0o3, 0o4, 0o5, 0o6, 0o7, 0o10, 0o17, 0o20, 0o100, 0o177, 0o200, 0o377, -1,
        -2, -3, -4, 1000, i32::MIN, i32::MAX,
    ];
    v.sort();
    v.dedup();
    v
}

fn compare_all(rust_path: &Path) {
    let c = Loaded::open(&c_library_path());
    let r = Loaded::open(rust_path);

    let mut checked: u64 = 0;
    let mut mismatches: Vec<String> = Vec::new();

    let values = interesting_values();

    for &mode in &operation_modes() {
        for &node_id in &values {
            for &depth in &values {
                // `flags` only participates in modes 0002 and 0003; sample a
                // few per (node_id, depth) pair to keep the matrix bounded,
                // then sweep it fully for the modes that use it.
                for &flags in &[0, 1, -1, 0o177, 0o200, i32::MIN, i32::MAX] {
                    let cv = c.call(mode, node_id, depth, flags);
                    let rv = r.call(mode, node_id, depth, flags);
                    checked += 1;
                    if cv != rv {
                        if mismatches.len() < 25 {
                            mismatches.push(format!(
                                "jumpnode({mode}, {node_id}, {depth}, {flags}): C={cv} Rust={rv}"
                            ));
                        }
                    }
                }
            }
        }
    }

    assert!(
        mismatches.is_empty(),
        "{} mismatches out of {} calls against {}:\n{}",
        mismatches.len(),
        checked,
        r.path.display(),
        mismatches.join("\n")
    );
    eprintln!(
        "{} calls matched ({} vs {})",
        checked,
        c.path.display(),
        r.path.display()
    );
}

#[test]
fn jumpnode_matches_c_across_input_matrix() {
    for rust_path in rust_library_paths() {
        compare_all(&rust_path);
    }
}

/// Full sweep of `flags` for the modes where it is actually mixed into the
/// result (0002 via `array_size * flags`, 0003 via `flags & 0177`).
#[test]
fn jumpnode_flags_sweep() {
    for rust_path in rust_library_paths() {
        let c = Loaded::open(&c_library_path());
        let r = Loaded::open(&rust_path);

        for mode in [0o2, 0o3] {
            for flags in -600i32..=600 {
                for depth in [0, 1, 7, 8, 15, 16, 17, -1, 100] {
                    let cv = c.call(mode, 1, depth, flags);
                    let rv = r.call(mode, 1, depth, flags);
                    assert_eq!(
                        cv, rv,
                        "jumpnode({mode}, 1, {depth}, {flags}) C={cv} Rust={rv} [{}]",
                        r.path.display()
                    );
                }
            }
            // Boundary flags values that stress wrapping multiplication.
            for flags in [
                i32::MIN,
                i32::MIN + 1,
                -134217728,
                134217727,
                134217728,
                i32::MAX - 1,
                i32::MAX,
            ] {
                let cv = c.call(mode, 1, 0, flags);
                let rv = r.call(mode, 1, 0, flags);
                assert_eq!(cv, rv, "jumpnode({mode}, 1, 0, {flags})");
            }
        }
    }
}

/// Mode 0003 formats `node_id`/`depth` with `sprintf("Node_%d_Depth_%d")` and
/// then derives the result from `strlen`. Sweep both operands densely across
/// digit-count boundaries and sign changes, where a `%d` formatting difference
/// would show up as a length difference.
#[test]
fn jumpnode_sprintf_length_boundaries() {
    for rust_path in rust_library_paths() {
        let c = Loaded::open(&c_library_path());
        let r = Loaded::open(&rust_path);

        let mut probes: Vec<i32> = Vec::new();
        for e in 0..10u32 {
            let p = 10i32.checked_pow(e).unwrap_or(i32::MAX);
            for delta in [-1i32, 0, 1] {
                if let Some(v) = p.checked_add(delta) {
                    probes.push(v);
                    probes.push(-v);
                }
            }
        }
        probes.extend_from_slice(&[0, i32::MIN, i32::MIN + 1, i32::MAX, i32::MAX - 1]);
        probes.sort();
        probes.dedup();

        for &node_id in &probes {
            for &depth in &probes {
                let cv = c.call(0o3, node_id, depth, 0);
                let rv = r.call(0o3, node_id, depth, 0);
                assert_eq!(
                    cv, rv,
                    "jumpnode(3, {node_id}, {depth}, 0) C={cv} Rust={rv} [{}]",
                    r.path.display()
                );
            }
        }
    }
}

/// Repeated calls must not drift: the C translation unit keeps mutable file
/// scope state (`node_storage` / `node_count`), so a stateful divergence in the
/// Rust `static mut` handling would surface as a difference on the second pass.
#[test]
fn jumpnode_is_stable_across_repeated_calls() {
    for rust_path in rust_library_paths() {
        let c = Loaded::open(&c_library_path());
        let r = Loaded::open(&rust_path);

        let mut first: Vec<i32> = Vec::new();
        for pass in 0..3 {
            let mut idx = 0usize;
            for mode in 0o0..=0o5 {
                for node_id in [-1, 0, 1, 2, 3, 4, 5, 6, 7, 8, 99] {
                    for depth in [0, 1, 2, 3, 5, 10, 16, 20] {
                        let cv = c.call(mode, node_id, depth, 3);
                        let rv = r.call(mode, node_id, depth, 3);
                        assert_eq!(cv, rv, "pass {pass}: jumpnode({mode}, {node_id}, {depth}, 3)");
                        if pass == 0 {
                            first.push(cv);
                        } else {
                            assert_eq!(
                                first[idx], cv,
                                "C drifted between passes at ({mode}, {node_id}, {depth})"
                            );
                            assert_eq!(
                                first[idx], rv,
                                "Rust drifted between passes at ({mode}, {node_id}, {depth})"
                            );
                        }
                        idx += 1;
                    }
                }
            }
        }
    }
}

/// Every symbol exported by the C `.so` must also be exported by the Rust
/// `.so`, under the exact same name.
#[test]
fn rust_exports_superset_of_c_exports() {
    fn exported(path: &Path) -> Vec<String> {
        let out = std::process::Command::new("nm")
            .args(["-D", "--defined-only", path.to_str().unwrap()])
            .output()
            .expect("`nm` must be available");
        assert!(
            out.status.success(),
            "nm failed on {}: {}",
            path.display(),
            String::from_utf8_lossy(&out.stderr)
        );
        let mut names: Vec<String> = String::from_utf8_lossy(&out.stdout)
            .lines()
            .filter_map(|l| {
                let mut it = l.split_whitespace();
                let (a, b) = (it.next()?, it.next()?);
                // "<addr> <type> <name>" or "         <type> <name>"
                let (ty, name) = match it.next() {
                    Some(n) => (b, n),
                    None => (a, b),
                };
                // Exported code/data, skipping linker-synthesised entries.
                matches!(ty, "T" | "t" | "D" | "B" | "R" | "W" | "V" | "G" | "S")
                    .then(|| name.to_string())
            })
            .filter(|n| {
                !matches!(
                    n.as_str(),
                    "_init"
                        | "_fini"
                        | "__bss_start"
                        | "_edata"
                        | "_end"
                        | "__odr_asan_gen_"
                        | "_ITM_deregisterTMCloneTable"
                        | "_ITM_registerTMCloneTable"
                        | "__gmon_start__"
                )
            })
            .collect();
        names.sort();
        names.dedup();
        names
    }

    let c_syms = exported(&c_library_path());
    assert!(
        c_syms.contains(&"jumpnode".to_string()),
        "sanity: C .so should export `jumpnode`, got {c_syms:?}"
    );

    for rust_path in rust_library_paths() {
        let r_syms = exported(&rust_path);
        let missing: Vec<&String> = c_syms.iter().filter(|s| !r_syms.contains(s)).collect();
        assert!(
            missing.is_empty(),
            "Rust .so {} is missing symbols exported by the C .so: {missing:?}",
            rust_path.display()
        );
    }
}
