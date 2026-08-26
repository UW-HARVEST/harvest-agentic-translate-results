//! Phase D — symbol parity between the C and the Rust shared object.

mod common;

use common::*;
use std::process::Command;

fn nm_defined(path: &std::path::Path) -> Vec<String> {
    let out = Command::new("nm")
        .args(["-D", "--defined-only"])
        .arg(path)
        .output()
        .expect("run nm");
    assert!(out.status.success(), "nm failed on {:?}", path);
    let mut v: Vec<String> = String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().last().map(|s| s.to_string()))
        .collect();
    v.sort();
    v.dedup();
    v
}

fn nm_undefined(path: &std::path::Path) -> Vec<String> {
    let out = Command::new("nm")
        .args(["-D", "-u"])
        .arg(path)
        .output()
        .expect("run nm");
    let mut v: Vec<String> = String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().last().map(|s| s.to_string()))
        .collect();
    v.sort();
    v.dedup();
    v
}

/// Symbols the C runtime / unwinder legitimately imports.
fn is_runtime_import(s: &str) -> bool {
    let base = s.split('@').next().unwrap_or(s);
    base.starts_with("__")
        || base.starts_with("_ITM_")
        || base.starts_with("_Unwind_")
        || base.starts_with("pthread_")
        || base.starts_with("_dl_")
        || matches!(
            base,
            "malloc"
                | "calloc"
                | "realloc"
                | "free"
                | "posix_memalign"
                | "memcpy"
                | "memmove"
                | "memset"
                | "memcmp"
                | "bcmp"
                | "strlen"
                | "abort"
                | "exit"
                | "fwrite"
                | "fflush"
                | "fputs"
                | "fputc"
                | "putc"
                | "putchar"
                | "printf"
                | "fprintf"
                | "puts"
                | "stdout"
                | "stderr"
                | "write"
                | "writev"
                | "read"
                | "close"
                | "open"
                | "open64"
                | "lseek64"
                | "fstat64"
                | "statx"
                | "stat64"
                | "realpath"
                | "mmap"
                | "mmap64"
                | "munmap"
                | "mremap"
                | "getcwd"
                | "getenv"
                | "gettid"
                | "getrandom"
                | "sigaltstack"
                | "sigaction"
                | "sigaddset"
                | "sigemptyset"
                | "syscall"
                | "sysconf"
                | "dl_iterate_phdr"
                | "dlsym"
                | "poll"
                | "readlink"
                | "signal"
                | "raise"
                | "strerror_r"
                | "nanosleep"
                | "sched_yield"
                | "pipe2"
                | "environ"
                | "mprotect"
                | "madvise"
        )
}

fn main() {
    let cso = c_so_path();
    let rso = rust_so_path();
    let mut h = Harness::new("Phase D - symbol parity");

    let c = nm_defined(&cso);
    let r = nm_defined(&rso);

    h.row("D1-missing", |row| {
        let missing: Vec<&String> = c.iter().filter(|s| !r.contains(s)).collect();
        row.eq("C symbols missing from the Rust .so", missing.len(), 0);
        row.ok(&format!("missing = {:?}", missing), missing.is_empty());
        row.ok(
            &format!("C exports {} symbols (expected 35)", c.len()),
            c.len() == 35,
        );
    });

    h.row("D2-extra", |row| {
        let extra: Vec<&String> = r.iter().filter(|s| !c.contains(s)).collect();
        row.eq("Rust-only exported symbols", extra.len(), 0);
        row.ok(&format!("extra = {:?}", extra), extra.is_empty());
    });

    h.row("D3-statics", |row| {
        // the `static` C helpers must not be exported by either side
        for s in [
            "hash_function",
            "should_resize",
            "hashmap_resize",
            "tree_free_node",
            "tree_remove_subtree",
            "tree_print_helper",
        ] {
            row.ok(
                &format!("{} not exported by C", s),
                !c.iter().any(|x| x == s),
            );
            row.ok(
                &format!("{} not exported by Rust", s),
                !r.iter().any(|x| x == s),
            );
        }
    });

    h.row("D4-undefined", |row| {
        let u = nm_undefined(&rso);
        let bad: Vec<&String> = u.iter().filter(|s| !is_runtime_import(s)).collect();
        row.ok(
            &format!("non-libc undefined symbols in the Rust .so: {:?}", bad),
            bad.is_empty(),
        );
    });

    h.row("D5-executables", |row| {
        // whole-program equivalence of the two built binaries
        let c_exe = manifest_dir().join("c_src/build/driver");
        let r_exe = target_profile_dir().join("driver");
        if !c_exe.exists() {
            let bdir = manifest_dir().join("c_src/build");
            std::fs::create_dir_all(&bdir).unwrap();
            let ok = Command::new("cmake")
                .current_dir(&bdir)
                .args(["..", "-DCMAKE_POSITION_INDEPENDENT_CODE=ON"])
                .status()
                .map(|s| s.success())
                .unwrap_or(false);
            assert!(ok, "cmake configure failed");
            let ok = Command::new("cmake")
                .current_dir(&bdir)
                .args(["--build", "."])
                .status()
                .map(|s| s.success())
                .unwrap_or(false);
            assert!(ok, "cmake build failed");
        }
        row.ok(&format!("{:?} exists", c_exe), c_exe.exists());
        row.ok(&format!("{:?} exists", r_exe), r_exe.exists());
        if c_exe.exists() && r_exe.exists() {
            let a = Command::new(&c_exe).output().unwrap();
            let b = Command::new(&r_exe).output().unwrap();
            row.eq_bytes("driver stdout", &a.stdout, &b.stdout);
            row.eq_bytes("driver stderr", &a.stderr, &b.stderr);
            row.eq("driver exit", a.status.code(), b.status.code());
            row.ok("driver stdout non-empty", !a.stdout.is_empty());
        }
    });

    h.finish();
}
