//! Negative control — proves the differential harness can actually FAIL.
//!
//! Every row in `CONFIGS.md` and `ERRORS.md` passes. That is only meaningful if
//! the comparison is capable of detecting a divergence in the first place. This
//! test builds deliberately *wrong* `.so`s and asserts the same comparison the
//! real tests use rejects each of them. Without this, a broken capture (say,
//! one that always returns empty bytes) would make the whole suite pass
//! vacuously.

mod common;

use common::*;
use std::ffi::c_int;
use std::path::{Path, PathBuf};

/// Compiles a standalone cdylib exporting `helloworld` from the given Rust
/// body, returning its path. Returns `None` if `rustc` is unavailable.
fn build_mutant(tag: &str, body: &str) -> Option<PathBuf> {
    let dir = std::env::temp_dir().join(format!("hello_mutants_{}", std::process::id()));
    std::fs::create_dir_all(&dir).ok()?;
    let src = dir.join(format!("{tag}.rs"));
    let so = dir.join(format!("lib{tag}.so"));
    let source = format!(
        r#"
use std::ffi::{{c_char, c_int}};
extern "C" {{ fn printf(f: *const c_char, ...) -> c_int; }}
#[no_mangle]
pub extern "C" fn helloworld() -> c_int {{
{body}
}}
"#
    );
    std::fs::write(&src, source).ok()?;
    let ok = std::process::Command::new("rustc")
        .args(["--crate-type", "cdylib", "--edition", "2021", "-O"])
        .arg(&src)
        .arg("-o")
        .arg(&so)
        .output()
        .ok()
        .map(|o| o.status.success())
        .unwrap_or(false);
    if ok && so.is_file() {
        Some(so)
    } else {
        None
    }
}

/// The same comparisons the real tests perform, but returning a verdict
/// instead of panicking. This is a *battery*, mirroring several `CONFIGS.md`
/// rows, because different bugs are only visible in different rows: a wrong
/// message shows up in the plainest row, whereas an implementation that writes
/// through its own buffer instead of libc's `stdout` is invisible until output
/// is interleaved with pending caller-side stdio.
fn agrees(c: &Path, other: &Path) -> bool {
    let lc = open(c);
    let lo = open(other);
    plain_agrees(&lc, &lo) && interleaved_agrees(&lc, &lo)
}

fn sym(lib: &libloading::Library) -> libloading::Symbol<'_, unsafe extern "C" fn() -> c_int> {
    unsafe { lib.get(b"helloworld\0") }.expect("dlsym(helloworld)")
}

/// Row C2/C4 shape: call it a few times, compare bytes and returns.
fn plain_agrees(a: &libloading::Library, b: &libloading::Library) -> bool {
    let run = |lib: &libloading::Library| {
        let f = sym(lib);
        capture(Sink::File, Buffering::Default, || unsafe {
            (0..3).map(|_| f()).collect::<Vec<c_int>>()
        })
    };
    run(a) == run(b)
}

/// Row C9 shape: a fully buffered `stdout` with an unterminated caller-side
/// `printf` already sitting in libc's buffer when the library is called. An
/// implementation that flushes through a *different* buffer will emit its line
/// ahead of the caller's pending bytes, so the byte order diverges.
fn interleaved_agrees(a: &libloading::Library, b: &libloading::Library) -> bool {
    let run = |lib: &libloading::Library| {
        let f = sym(lib);
        capture(Sink::File, Buffering::Full(4096), || unsafe {
            let mut rets = Vec::new();
            for _ in 0..3 {
                // No trailing newline: stays parked in libc's buffer.
                libc::printf(b"<mark>\0".as_ptr() as *const std::ffi::c_char);
                rets.push(f());
            }
            rets
        })
    };
    run(a) == run(b)
}

#[test]
fn negative_control_harness_detects_divergences() {
    let c = c_so_path();

    // Sanity: the real Rust .so must agree.
    assert!(
        agrees(&c, &rust_so_path()),
        "the real Rust .so must agree with the C .so"
    );

    let mutants: &[(&str, &str)] = &[
        // Missing trailing newline.
        (
            "mut_nonewline",
            r#"    unsafe { printf(b"Hello World!\0".as_ptr() as *const c_char); }
    0"#,
        ),
        // Wrong capitalisation.
        (
            "mut_case",
            r#"    unsafe { printf(b"hello world!\n\0".as_ptr() as *const c_char); }
    0"#,
        ),
        // Missing the exclamation mark.
        (
            "mut_nobang",
            r#"    unsafe { printf(b"Hello World\n\0".as_ptr() as *const c_char); }
    0"#,
        ),
        // Right bytes, wrong return value.
        (
            "mut_ret",
            r#"    unsafe { printf(b"Hello World!\n\0".as_ptr() as *const c_char); }
    -1"#,
        ),
        // Prints nothing at all.
        ("mut_silent", "    0"),
        // Prints twice.
        (
            "mut_double",
            r#"    unsafe {
        printf(b"Hello World!\n\0".as_ptr() as *const c_char);
        printf(b"Hello World!\n\0".as_ptr() as *const c_char);
    }
    0"#,
        ),
        // Writes to Rust's own buffered stdout instead of the libc stream —
        // the exact bug the buffering/interleaving rows exist to catch.
        (
            "mut_ruststdout",
            r#"    use std::io::Write;
    let mut o = std::io::stdout();
    let _ = o.write_all(b"Hello World!\n");
    0"#,
        ),
    ];

    let mut built = 0usize;
    for (tag, body) in mutants {
        let Some(so) = build_mutant(tag, body) else {
            eprintln!("skipping mutant {tag}: rustc unavailable");
            continue;
        };
        built += 1;
        assert!(
            !agrees(&c, &so),
            "the harness FAILED TO DETECT mutant `{tag}` — the differential \
             comparison is not actually checking anything"
        );
    }
    assert!(
        built >= 6,
        "expected to build the mutants; only {built} compiled, so the negative \
         control did not really run"
    );

    // Clean up.
    let dir = std::env::temp_dir().join(format!("hello_mutants_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn negative_control_symbol_diff_detects_a_missing_export() {
    // A .so with no `helloworld` at all must be rejected by the same
    // dlsym-based access path the real tests use.
    let dir = std::env::temp_dir().join(format!("hello_nosym_{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("mkdir");
    let src = dir.join("nosym.rs");
    let so = dir.join("libnosym.so");
    std::fs::write(
        &src,
        "#[no_mangle] pub extern \"C\" fn something_else() -> i32 { 0 }\n",
    )
    .expect("write");
    let built = std::process::Command::new("rustc")
        .args(["--crate-type", "cdylib", "--edition", "2021"])
        .arg(&src)
        .arg("-o")
        .arg(&so)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);
    if !built {
        eprintln!("skipping: rustc unavailable");
        return;
    }
    let lib = open(&so);
    let sym: Result<libloading::Symbol<unsafe extern "C" fn() -> c_int>, _> =
        unsafe { lib.get(b"helloworld\0") };
    assert!(
        sym.is_err(),
        "dlsym must fail on a .so that does not export `helloworld`; if it \
         succeeded, symbol lookup is leaking to another loaded library and the \
         parity check is unsound"
    );
    let _ = std::fs::remove_dir_all(&dir);
}
