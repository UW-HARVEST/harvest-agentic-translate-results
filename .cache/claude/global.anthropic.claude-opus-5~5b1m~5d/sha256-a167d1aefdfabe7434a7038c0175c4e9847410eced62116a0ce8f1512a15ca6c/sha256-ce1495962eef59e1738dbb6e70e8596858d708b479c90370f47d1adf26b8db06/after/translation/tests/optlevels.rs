// Robustness suite: is the C reference behaviour stable across C compiler
// optimization levels?
//
// The single most load-bearing fact in this translation is that `helperBad()`
// returns the address of an automatic array (CWE-562, undefined behaviour) and
// that the compiler resolves that UB by emitting `mov $0x0,%eax` — i.e. it
// returns NULL, so `printLine`'s guard suppresses all output and `bad()` prints
// nothing. If that were only true at `-O0` (which is what CMake produces with no
// CMAKE_BUILD_TYPE), the translation would be pinned to one build configuration.
//
// This suite recompiles `c_src/src/driver.c` — WITHOUT modifying anything in
// c_src; every artifact goes to the temp directory — at -O0, -O1, -O2, -O3 and
// -Os, then runs the same differential battery against the Rust `.so` for each.
// It skips cleanly if no C compiler is available.
//
// `harness = false` — the cases must run sequentially; see
// `tests/common/mod.rs::Runner`.

mod common;

use common::*;
use std::path::PathBuf;
use std::process::Command;

const OPT_LEVELS: &[&str] = &["-O0", "-O1", "-O2", "-O3", "-Os"];

fn cc() -> Option<String> {
    for candidate in [std::env::var("CC").unwrap_or_default(), "cc".into(), "gcc".into()] {
        if candidate.is_empty() {
            continue;
        }
        if Command::new(&candidate)
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
        {
            return Some(candidate);
        }
    }
    None
}

fn c_src_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("crate has a parent")
        .join("c_src")
}

/// Compile the untouched C source at `opt` into a scratch `.so` under TMPDIR.
fn build_c_at(cc: &str, opt: &str) -> Option<PathBuf> {
    let root = c_src_root();
    let out = std::env::temp_dir().join(format!(
        "driver-optlevel-{}-{}.so",
        std::process::id(),
        opt.trim_start_matches('-')
    ));
    let status = Command::new(cc)
        .arg(opt)
        .args(["-fPIC", "-shared", "-w"])
        .arg("-I")
        .arg(root.join("include"))
        .arg("-o")
        .arg(&out)
        .arg(root.join("src/driver.c"))
        .status();
    match status {
        Ok(s) if s.success() => Some(out),
        _ => None,
    }
}

/// The full behavioural battery, run against one library.
fn battery(lib: &'static Lib) {
    unsafe {
        lib.good_raw();
        lib.bad_raw();
        lib.driver_raw(0);
        lib.driver_raw(1);
        lib.driver_raw(-1);
        lib.driver_raw(i32::MIN);
        lib.driver_raw(i32::MAX);
        lib.print_line_raw(std::ptr::null());
        with_cstr(b"", |p| lib.print_line_raw(p));
        with_cstr(b"plain ascii", |p| lib.print_line_raw(p));
        with_cstr(b"%s %d %n %%", |p| lib.print_line_raw(p));
        with_cstr(&[0x80, 0xff, 0xfe, 0xc2], |p| lib.print_line_raw(p));
        with_cstr(b"embedded\nnewline", |p| lib.print_line_raw(p));
        for _ in 0..10 {
            lib.good_raw();
            lib.bad_raw();
        }
    }
}

/// The exact bytes `battery` must produce, derived from the C source.
fn battery_expected() -> Vec<u8> {
    let mut e = Vec::new();
    e.extend_from_slice(GOOD_OUTPUT); // good()
    e.extend_from_slice(BAD_OUTPUT); // bad()          -> nothing
    e.extend_from_slice(BAD_OUTPUT); // driver(0)      -> nothing
    e.extend_from_slice(GOOD_OUTPUT); // driver(1)
    e.extend_from_slice(GOOD_OUTPUT); // driver(-1)
    e.extend_from_slice(GOOD_OUTPUT); // driver(i32::MIN)
    e.extend_from_slice(GOOD_OUTPUT); // driver(i32::MAX)
    // printLine(NULL)                                 -> nothing
    e.extend_from_slice(b"\n"); // printLine("")
    e.extend_from_slice(b"plain ascii\n");
    e.extend_from_slice(b"%s %d %n %%\n");
    e.extend_from_slice(&[0x80, 0xff, 0xfe, 0xc2, b'\n']);
    e.extend_from_slice(b"embedded\nnewline\n");
    for _ in 0..10 {
        e.extend_from_slice(GOOD_OUTPUT);
        e.extend_from_slice(BAD_OUTPUT);
    }
    e
}

fn main() {
    let mut r = Runner::new("optlevels (C build-configuration robustness)");

    let compiler = match cc() {
        Some(c) => c,
        None => {
            println!("no C compiler found; skipping the optimization-level suite");
            r.finish();
            return;
        }
    };
    println!("using C compiler: {compiler}");

    let expected = battery_expected();

    for opt in OPT_LEVELS {
        let name = format!("c_at_{}_matches_rust", opt.trim_start_matches('-'));
        let so = match build_c_at(&compiler, opt) {
            Some(p) => p,
            None => {
                println!("test {name} ... skipped (compile at {opt} failed)");
                continue;
            }
        };
        let leaked_opt: &'static str = Box::leak(format!("C{opt}").into_boxed_str());
        let exp = expected.clone();
        r.case(&name, move || {
            let c = load_from(leaked_opt, so.clone());
            let rust = rust_lib();
            assert_same_between(
                &format!("battery @ {leaked_opt}"),
                c,
                rust,
                Some(&exp),
                battery,
            );
            // Randomized printLine payloads at this optimization level too.
            let mut rng = Rng::new(99);
            for i in 0..300 {
                let len = rng.below(200);
                let payload = rng.nonzero_bytes(len);
                with_cstr(&payload, |p| {
                    assert_same_between(
                        &format!("printLine fuzz #{i} @ {leaked_opt}"),
                        c,
                        rust,
                        Some(&expected_line(&payload)),
                        |lib| unsafe { lib.print_line_raw(p) },
                    );
                });
            }
            let _ = std::fs::remove_file(&so);
        });
    }

    r.case("cmake_reference_build_matches_rust", || {
        assert_same_between(
            "battery @ cmake reference build",
            c_lib(),
            rust_lib(),
            Some(&battery_expected()),
            battery,
        );
    });

    r.finish();
}
