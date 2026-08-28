//! Phase C: the branches `main()` never reaches.
//!
//! `c_src/src/main.c` is `int main(void)` and reads no input, so no invocation
//! of the shipped executable can steer `tree.c`/`hashmap.c` down the branches
//! that `main()` happens not to call. To cover them anyway, this test builds a
//! *second* driver:
//!
//!   * `tests/probe/probe.c`  linked against the PRISTINE `c_src/src/tree.c`
//!     and `c_src/src/hashmap.c` (nothing in `c_src/` is touched or copied over)
//!   * `tests/probe/probe.rs` compiled against the translation's own
//!     `src/cio.rs`, `src/hashmap.rs`, `src/tree.rs`
//!
//! Both probes are then executed as subprocesses and their stdout, stderr and
//! exit status compared byte for byte -- the same contract as the main
//! differential test. The graded `driver` binary is not involved.

mod common;

use common::*;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

fn out_dir() -> PathBuf {
    // Reuse cargo's target dir so nothing is written outside the crate.
    let exe = std::env::current_exe().expect("current_exe");
    let dir = exe
        .parent()
        .and_then(|p| p.parent())
        .expect("target/<profile>")
        .join("probe-bins");
    std::fs::create_dir_all(&dir).expect("create probe output dir");
    dir
}

fn c_compiler() -> String {
    // Prefer whatever CMake picked for the real build.
    let cache = workspace_root().join("c_src/build/CMakeCache.txt");
    if let Ok(text) = std::fs::read_to_string(&cache) {
        for line in text.lines() {
            if let Some(rest) = line.strip_prefix("CMAKE_C_COMPILER:FILEPATH=") {
                if !rest.is_empty() {
                    return rest.to_string();
                }
            }
        }
    }
    std::env::var("CC").unwrap_or_else(|_| "cc".to_string())
}

/// Build the C probe against the untouched c_src sources.
fn c_probe() -> PathBuf {
    static P: OnceLock<PathBuf> = OnceLock::new();
    P.get_or_init(|| {
        // Make sure the C project has been configured, so CMakeCache exists.
        let _ = c_binary();

        let root = workspace_root();
        let bin = out_dir().join("probe_c");
        let out = Command::new(c_compiler())
            .arg("-std=c11")
            .arg("-O2")
            .arg("-I")
            .arg(root.join("c_src/include"))
            .arg(root.join("translation/tests/probe/probe.c"))
            .arg(root.join("c_src/src/tree.c"))
            .arg(root.join("c_src/src/hashmap.c"))
            .arg("-o")
            .arg(&bin)
            .output()
            .expect("invoke the C compiler for the probe");
        assert!(
            out.status.success(),
            "compiling the C probe failed:\n{}\n{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        );
        assert!(bin.is_file());
        bin
    })
    .clone()
}

/// Build the Rust probe from the translation's modules.
fn rust_probe() -> PathBuf {
    static P: OnceLock<PathBuf> = OnceLock::new();
    P.get_or_init(|| {
        let root = workspace_root();
        let bin = out_dir().join("probe_rs");
        let out = Command::new(std::env::var("RUSTC").unwrap_or_else(|_| "rustc".into()))
            .arg("--edition=2021")
            .arg("-O")
            .arg("--crate-name")
            .arg("probe_rs")
            .arg(root.join("translation/tests/probe/probe.rs"))
            .arg("-o")
            .arg(&bin)
            .output()
            .expect("invoke rustc for the probe");
        assert!(
            out.status.success(),
            "compiling the Rust probe failed:\n{}\n{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        );
        assert!(bin.is_file());
        bin
    })
    .clone()
}

fn run_probe(bin: &Path) -> RunResult {
    run(bin, &Spec::new())
}

#[test]
fn probes_agree_on_every_uncovered_branch() {
    let c = run_probe(&c_probe());
    let r = run_probe(&rust_probe());

    // Sanity: the probe really did produce a lot of output and succeeded.
    assert_eq!(c.raw_status, 0, "C probe {}", c.describe_status());
    let text = String::from_utf8_lossy(&c.stdout).to_string();
    assert!(
        text.starts_with("=== BRANCH PROBE ===\n"),
        "C probe stdout started unexpectedly: {:?}",
        &text[..text.len().min(80)]
    );
    assert!(
        text.ends_with("=== PROBE DONE ===\n"),
        "C probe did not run to completion"
    );
    assert!(
        c.stdout.len() > 10_000,
        "probe stdout suspiciously small ({} bytes)",
        c.stdout.len()
    );
    assert!(!c.stderr.is_empty(), "probe should reach stderr error paths");

    assert_identical("branch probe", &c, &r);
}

#[test]
fn probe_reaches_the_error_messages_main_never_prints() {
    let c = run_probe(&c_probe());
    let err = String::from_utf8_lossy(&c.stderr).to_string();
    for expected in [
        // tree_add_node: parent lookup failure -- unreachable from main.c
        "Error: Parent node 99 not found\n",
        // tree_remove_node: missing node -- unreachable from main.c
        "Error: Node 1 not found\n",
        // both of the paths main.c does reach, for completeness
        "Error: Node with ID 10 already exists\n",
        "Error: Parent has maximum children\n",
    ] {
        assert!(
            err.contains(expected),
            "probe stderr is missing {expected:?}\n--- stderr ---\n{err}"
        );
    }

    let r = run_probe(&rust_probe());
    assert_eq!(
        c.stderr, r.stderr,
        "stderr differs\nC:\n{}\nRust:\n{}",
        err,
        String::from_utf8_lossy(&r.stderr)
    );
}

#[test]
fn probe_reaches_the_empty_tree_print_and_truncation_paths() {
    let c = run_probe(&c_probe());
    let text = String::from_utf8_lossy(&c.stdout).to_string();

    // tree_print with no root -- unreachable from main.c
    assert!(text.contains("(empty tree)\n"), "no empty-tree print");

    // tree_add_node with data == NULL -- unreachable from main.c
    assert!(
        text.contains("datalen=0 data=\"\""),
        "no NULL/empty data node"
    );

    // strncpy truncation at MAX_DATA_LENGTH-1
    assert!(text.contains("datalen=255"), "no 255-byte truncated data");

    // tree_find_path returning -1, 0, and a truncated length
    assert!(text.contains("=-1 ["), "no failing find_path");
    assert!(text.contains("max=0)=0 []"), "no zero-length path");
    assert!(text.contains("max=3)=3 ["), "no truncated path");
    // the temp_path[1000] loop cap
    assert!(text.contains("max=64)=64 ["), "no 1000-entry cap hit");

    // hashmap resize happened (capacity grew past the initial 16)
    assert!(text.contains("cap=32"), "hashmap never resized to 32");
    assert!(text.contains("cap=64"), "hashmap never resized to 64");

    // hashmap tombstones and the NULL-value quirk
    assert!(text.contains("del=1"), "no tombstone state observed");
    assert!(
        text.contains("put(100, NULL)=0"),
        "NULL value never inserted"
    );
    assert!(
        text.contains("get(100)=(null) contains=0"),
        "NULL-value key should look absent to get/contains"
    );

    let r = run_probe(&rust_probe());
    assert_identical("branch probe (stdout detail)", &c, &r);
}

#[test]
fn probes_agree_on_buffer_boundary_flush_timing() {
    // The probe writes ~14 KiB, so a fully buffered stdout is flushed several
    // times mid-run at 4096-byte boundaries, while stderr goes out unbuffered.
    // Merging the two streams therefore pins down the exact *position* of every
    // flush -- the strictest available check on the stdio emulation.
    let c = run_merged(&c_probe(), &Spec::new());
    let r = run_merged(&rust_probe(), &Spec::new());
    assert!(
        c.stdout.len() > 4096 * 3,
        "probe must overflow the stdio buffer several times, got {} bytes",
        c.stdout.len()
    );
    assert_identical("probe, merged 2>&1", &c, &r);
}

#[test]
fn probes_agree_under_stream_error_conditions_too() {
    let c = c_probe();
    let r = rust_probe();

    let cf = run_stdout_unwritable(&c);
    let rf = run_stdout_unwritable(&r);
    assert_identical("probe, unwritable stdout", &cf, &rf);

    let cp = run_with_closed_reader(&c, Stream::Stdout);
    let rp = run_with_closed_reader(&r, Stream::Stdout);
    assert_identical("probe, stdout reader closed", &cp, &rp);
}
