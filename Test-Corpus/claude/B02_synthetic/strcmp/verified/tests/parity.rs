//! Phase D -- symbol parity and build-configuration parity.
//!
//! These tests mechanise the claims made in `SYMBOLS.md` so they cannot rot:
//! the artifacts are executables (not libraries), the C binary exports no
//! source-level symbol that the Rust binary lacks, every C function has a Rust
//! counterpart, and there is exactly one build configuration on both sides.

mod common;

use common::*;

use std::collections::BTreeSet;
use std::process::Command;

fn nm(args: &[&str]) -> String {
    let out = Command::new("nm")
        .args(args)
        .output()
        .expect("nm must be available");
    String::from_utf8_lossy(&out.stdout).to_string()
}

/// Names of symbols in an `nm` listing, keeping only the given type letters
/// (empty set = keep all).
fn syms(listing: &str, types: &[char]) -> BTreeSet<String> {
    let mut set = BTreeSet::new();
    for line in listing.lines() {
        let cols: Vec<&str> = line.split_whitespace().collect();
        if cols.len() < 2 {
            continue;
        }
        // "<addr> <type> <name>" or "<type> <name>" (undefined)
        let (ty, name) = if cols.len() >= 3 {
            (cols[cols.len() - 2], cols[cols.len() - 1])
        } else {
            (cols[0], cols[1])
        };
        let t = ty.chars().next().unwrap_or('?');
        if types.is_empty() || types.contains(&t) {
            set.insert(name.to_string());
        }
    }
    set
}

// ---------------------------------------------------------------------------
// the artifacts are executables, not shared libraries
// ---------------------------------------------------------------------------

#[test]
fn d01_c_target_is_an_executable() {
    let cmake = std::fs::read_to_string(manifest_dir().join("c_src/CMakeLists.txt")).unwrap();
    assert!(
        cmake.contains("add_executable(driver src/main.c)"),
        "c_src/CMakeLists.txt no longer builds a single executable:\n{cmake}"
    );
    assert!(
        !cmake.contains("add_library"),
        "c_src/CMakeLists.txt now builds a library -- the .so based comparison \
         would have to be used instead"
    );
    // ... and there is exactly one translation unit
    let n = std::fs::read_dir(manifest_dir().join("c_src/src"))
        .unwrap()
        .filter(|e| {
            e.as_ref()
                .unwrap()
                .path()
                .extension()
                .map(|x| x == "c" || x == "h")
                .unwrap_or(false)
        })
        .count();
    assert_eq!(n, 1, "c_src/src should hold exactly main.c");
}

#[test]
fn d02_neither_artifact_is_dlopenable() {
    // Mechanical proof that the "load both .so files with libloading"
    // comparison cannot apply here: both artifacts are executables, so dlopen
    // refuses them.  (This is also what makes the process-boundary comparison
    // used by the other test files the only possible one.)
    for bin in [c_bin(), rust_bin()] {
        let r = unsafe { libloading::Library::new(&bin) };
        match r {
            Ok(_) => panic!(
                "{} unexpectedly dlopen()ed -- it is a library, so the tests \
                 should compare exported symbols directly",
                bin.display()
            ),
            Err(e) => {
                let msg = format!("{e}");
                assert!(
                    msg.contains("cannot dynamically load"),
                    "{}: unexpected dlopen error: {msg}",
                    bin.display()
                );
            }
        }
    }
}

// ---------------------------------------------------------------------------
// nm -D parity
// ---------------------------------------------------------------------------

#[test]
fn d03_dynamic_symbol_parity() {
    let c = c_bin();
    let r = rust_bin();
    let c_defined = syms(&nm(&["-D", "--defined-only", c.to_str().unwrap()]), &[]);
    let r_defined = syms(&nm(&["-D", "--defined-only", r.to_str().unwrap()]), &[]);

    // The single defined dynamic symbol of the reference binary is glibc's
    // `stdin` object, pulled in by a copy relocation (`fgets(…, stdin)`), not a
    // function implemented by main.c.  Anything else appearing here would be a
    // real export that the Rust binary must provide too.
    let allowed: BTreeSet<String> = ["stdin", "stdin@GLIBC_2.2.5"]
        .iter()
        .map(|s| s.to_string())
        .collect();
    let missing: Vec<&String> = c_defined
        .difference(&r_defined)
        .filter(|s| !allowed.contains(*s))
        .collect();
    assert!(
        missing.is_empty(),
        "symbols exported by the C binary but not by the Rust binary: {missing:?}"
    );

    // Undefined symbols must all be libc/libgcc imports (the loader resolves
    // them -- proven by the fact that every other test actually runs the
    // binary).  Nothing may reference a missing user symbol.
    let r_undef = syms(&nm(&["-D", "--undefined-only", r.to_str().unwrap()]), &[]);
    for s in &r_undef {
        let base = s.split('@').next().unwrap();
        assert!(
            base.starts_with('_')
                || base.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_'),
            "Rust binary imports a non-libc symbol: {s}"
        );
    }
}

#[test]
fn d04_every_c_function_has_a_rust_counterpart() {
    let c = c_bin();
    let listing = nm(&["--defined-only", c.to_str().unwrap()]);
    let text = syms(&listing, &['T', 't']);
    // CRT / toolchain glue that does not come from main.c
    let crt: BTreeSet<&str> = [
        "_start",
        "_init",
        "_fini",
        "_dl_relocate_static_pie",
        "frame_dummy",
        "register_tm_clones",
        "deregister_tm_clones",
        "__do_global_dtors_aux",
        "__libc_csu_init",
        "__libc_csu_fini",
        "call_weak_fn",
    ]
    .into_iter()
    .collect();

    let rust_src = std::fs::read_to_string(manifest_dir().join("src/main.rs")).unwrap();
    let mut checked = 0;
    for name in &text {
        if crt.contains(name.as_str()) {
            continue;
        }
        // snake_case the one camelCase C name (`cmd_compareN`)
        let expect = name.replace("compareN", "compare_n");
        assert!(
            rust_src.contains(&format!("fn {expect}(")),
            "C function `{name}` has no Rust counterpart (`fn {expect}`) -- the \
             translation would be incomplete"
        );
        checked += 1;
    }
    assert!(
        checked >= 26,
        "expected at least 26 translated C functions, found {checked}"
    );
}

// ---------------------------------------------------------------------------
// build configuration parity
// ---------------------------------------------------------------------------

#[test]
fn d05_single_build_configuration() {
    let cargo = std::fs::read_to_string(manifest_dir().join("Cargo.toml")).unwrap();
    assert!(
        !cargo.contains("[features]"),
        "Cargo.toml grew a [features] section: every combination now has to be \
         verified separately\n{cargo}"
    );
    let c_src = std::fs::read_to_string(manifest_dir().join("c_src/src/main.c")).unwrap();
    for pat in ["#if", "#ifdef", "#ifndef", "#elif"] {
        assert!(
            !c_src.contains(pat),
            "c_src/src/main.c now contains `{pat}`: conditional compilation has \
             to be enumerated"
        );
    }
    let cmake = std::fs::read_to_string(manifest_dir().join("c_src/CMakeLists.txt")).unwrap();
    for pat in ["option(", "target_compile_definitions", "CMAKE_BUILD_TYPE"] {
        assert!(
            !cmake.contains(pat),
            "c_src/CMakeLists.txt now contains `{pat}`: build configurations have \
             to be enumerated"
        );
    }
}

#[test]
fn d06_same_behaviour_from_the_release_profile() {
    // The default `cargo test` binary is the debug build; make sure the release
    // profile (panic = "abort") behaves identically on a representative corpus,
    // including a crash case.
    let rel = manifest_dir().join("target/release/driver");
    if !rel.exists() {
        eprintln!("skipping: target/release/driver not built");
        return;
    }
    let corpus: Vec<Vec<u8>> = vec![
        b"help\nstatus\n".to_vec(),
        b"adduser a b 9\nlogin a b\ncreatefile f c\nreadfile f\nls\nexit\n".to_vec(),
        b"compare abc abd\ncmpn abc abd -1\nstartswith foobar foo\nmatch oo foo zoo\n".to_vec(),
        {
            // SIGSEGV case
            let mut v = Vec::new();
            for i in 0..9 {
                v.extend_from_slice(format!("adduser u{i} p{i} 1\n").as_bytes());
            }
            v.extend_from_slice(b"adduser last ");
            v.extend_from_slice(&vec![b'p'; 44]);
            v.extend_from_slice(b" 3\nstatus\n");
            v
        },
    ];
    for (i, input) in corpus.iter().enumerate() {
        let c = run(&c_bin(), input);
        let r = run(&rel, input);
        assert_eq!(
            normalize_time(&c.stdout),
            normalize_time(&r.stdout),
            "release build stdout differs on corpus #{i}"
        );
        assert_eq!(c.stderr, r.stderr, "release build stderr differs on corpus #{i}");
        assert_eq!(c.status, r.status, "release build status differs on corpus #{i}");
    }
}
