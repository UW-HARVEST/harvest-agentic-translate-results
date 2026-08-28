//! Differential tests: run the C program and the Rust program as subprocesses
//! and require byte-identical stdout, byte-identical stderr and an identical
//! exit status.
//!
//! `c_src/src/main.c` declares `int main(void)` and never touches `stdin`, so
//! the executable's *input space* is not a stream of records — it is the set of
//! process-level conditions the program can be started under: argv, whatever
//! happens to be on stdin (which must be left unread), the environment, the cwd
//! and the state of the stdout/stderr descriptors. Every one of those is
//! covered below.
//!
//! The data-dependent branches inside `tree.c`/`hashmap.c` are driven by the
//! fixed call sequence in `main()`, so they are covered separately, by
//! `branch_coverage.rs`.

mod common;

use common::*;

// ---------------------------------------------------------------------------
// Baseline
// ---------------------------------------------------------------------------

#[test]
fn no_args_empty_stdin() {
    compare("no args, empty stdin", &Spec::new());
}

#[test]
fn output_is_not_empty() {
    // Guards against a harness bug where both programs silently produce
    // nothing and every comparison trivially "passes".
    let c = run(&c_binary(), &Spec::new());
    assert!(!c.stdout.is_empty(), "C produced no stdout");
    assert!(!c.stderr.is_empty(), "C produced no stderr");
    assert_eq!(c.raw_status, 0, "C should exit 0 on the happy path");
}

#[test]
fn deterministic_across_repeated_runs() {
    let first = run(&c_binary(), &Spec::new());
    for i in 0..5 {
        let c = run(&c_binary(), &Spec::new());
        let r = run(&rust_binary(), &Spec::new());
        assert_identical(&format!("repeat #{i}"), &c, &r);
        assert!(
            c.stdout == first.stdout && c.stderr == first.stderr,
            "C output is not deterministic across runs"
        );
    }
}

// ---------------------------------------------------------------------------
// The stdout content itself: exact expected bytes
// ---------------------------------------------------------------------------

#[test]
fn stdout_matches_expected_layout() {
    // Pin down the formatting details printf produces, so a regression in
    // spacing or a missing trailing newline cannot hide behind "both agree".
    let r = run(&rust_binary(), &Spec::new());
    let text = String::from_utf8(r.stdout.clone()).expect("stdout is UTF-8");

    // Banner: 40 U+2550 between corners, per the literals in main.c.
    let bar = "\u{2550}".repeat(40);
    assert!(
        text.starts_with(&format!(
            "\u{2554}{bar}\u{2557}\n\u{2551}  TREE WITH HASHMAP ID MAPPING TESTS   \u{2551}\n\u{255a}{bar}\u{255d}\n"
        )),
        "banner mismatch, got:\n{}",
        &text[..text.char_indices().nth(200).map_or(text.len(), |(i, _)| i)]
    );

    // Every test function reports a pass, and there are 14 of them.
    assert_eq!(
        text.matches("\u{2713} PASS: ").count(),
        14,
        "expected 14 PASS lines"
    );
    assert_eq!(text.matches("\u{2717} FAIL").count(), 0, "no FAIL expected");

    // tree_print output from test_tree_complex_structure: two-space indent per
    // level, "[id] data".
    for expect in [
        "[1] root\n",
        "  [2] child1\n",
        "    [5] gc1\n",
        "    [6] gc2\n",
        "  [3] child2\n",
        "    [7] gc3\n",
        "      [10] ggc1\n",
        "  [4] child3\n",
        "    [8] gc4\n",
        "    [9] gc5\n",
    ] {
        assert!(text.contains(expect), "missing tree_print line {expect:?}");
    }

    assert!(text.ends_with(
        "\n========================================\n  All tests passed successfully!\n========================================\n"
    ), "trailer mismatch, tail was:\n{:?}", &text[text.len().saturating_sub(120)..]);
}

#[test]
fn stderr_carries_exactly_the_two_expected_errors() {
    // main.c reaches exactly two error paths in tree_add_node: the duplicate-id
    // check (test_tree_duplicate_id) and the max-children check
    // (test_tree_max_children).
    let c = run(&c_binary(), &Spec::new());
    let r = run(&rust_binary(), &Spec::new());
    let expected = b"Error: Node with ID 2 already exists\nError: Parent has maximum children\n";
    assert_eq!(
        c.stderr, expected,
        "C stderr changed: {:?}",
        String::from_utf8_lossy(&c.stderr)
    );
    assert_eq!(r.stderr, c.stderr);
}

// ---------------------------------------------------------------------------
// argv: main(void) ignores everything
// ---------------------------------------------------------------------------

#[test]
fn arguments_are_ignored() {
    for args in [
        vec![],
        vec!["--help"],
        vec!["-h"],
        vec!["--version"],
        vec!["0"],
        vec!["-1"],
        vec!["99999999999999999999"],
        vec!["a", "b", "c", "d", "e"],
        vec![""],
        vec!["  "],
        vec!["--", "-x"],
        vec!["\u{00e9}\u{4e2d}\u{6587}"],
    ] {
        let label = format!("args {args:?}");
        compare(&label, &Spec::new().args(&args));
    }
}

#[test]
fn very_many_arguments() {
    let owned: Vec<String> = (0..500).map(|i| format!("arg{i}")).collect();
    let args: Vec<&str> = owned.iter().map(|s| s.as_str()).collect();
    compare("500 arguments", &Spec::new().args(&args));
}

// ---------------------------------------------------------------------------
// stdin: nothing reads it, so nothing may be consumed or reacted to
// ---------------------------------------------------------------------------

#[test]
fn empty_stdin() {
    compare("empty stdin", &Spec::new().stdin(StdinMode::Bytes(b"")));
}

#[test]
fn closed_stdin() {
    compare("closed stdin", &Spec::new().stdin(StdinMode::Closed));
}

#[test]
fn single_line_on_stdin() {
    compare("one line on stdin", &Spec::new().stdin(StdinMode::Bytes(b"1\n")));
}

#[test]
fn single_item_no_trailing_newline() {
    compare("no trailing newline", &Spec::new().stdin(StdinMode::Bytes(b"42")));
}

#[test]
fn structured_looking_stdin() {
    // Looks like input a tree driver might parse; must still be ignored.
    let payload = b"3\n1 0 root\n2 1 child\n3 2 grandchild\n";
    compare("structured stdin", &Spec::new().stdin(StdinMode::Bytes(payload)));
}

#[test]
fn stdin_with_nul_and_binary_bytes() {
    let payload: Vec<u8> = (0u8..=255).collect();
    compare(
        "all byte values on stdin",
        &Spec::new().stdin(StdinMode::Bytes(&payload)),
    );
}

#[test]
fn large_stdin_beyond_pipe_buffer() {
    // 256 KiB, well past the 64 KiB pipe buffer: proves the program never
    // blocks on a read it does not perform.
    let payload: Vec<u8> = std::iter::repeat(b"0123456789abcdef\n")
        .take(16 * 1024)
        .flat_map(|s| s.iter().copied())
        .collect();
    assert!(payload.len() > 200_000);
    compare(
        "256 KiB on stdin",
        &Spec::new().stdin(StdinMode::Bytes(&payload)),
    );
}

// ---------------------------------------------------------------------------
// Environment and working directory
// ---------------------------------------------------------------------------

#[test]
fn locale_does_not_change_formatting() {
    // The banner and the check marks are plain UTF-8 bytes in the source, and
    // printf uses no locale-sensitive conversions, so every locale must agree.
    for (k, v) in [
        ("LC_ALL", "C"),
        ("LC_ALL", "POSIX"),
        ("LC_ALL", "C.UTF-8"),
        ("LC_ALL", "en_US.UTF-8"),
        ("LC_ALL", "tr_TR.UTF-8"),
        ("LC_NUMERIC", "de_DE.UTF-8"),
        ("LANG", "ja_JP.UTF-8"),
    ] {
        compare(&format!("{k}={v}"), &Spec::new().env(k, v));
    }
}

#[test]
fn empty_environment() {
    compare("empty environment", &Spec::new().clear_env());
}

#[test]
fn runs_from_any_working_directory() {
    for dir in ["/", "/tmp", "/usr"] {
        compare(&format!("cwd {dir}"), &Spec::new().cwd(dir));
    }
}

// ---------------------------------------------------------------------------
// Stream buffering: fully buffered pipe vs line buffered terminal
// ---------------------------------------------------------------------------

#[test]
fn merged_streams_interleave_identically() {
    // With stdout on a pipe, C buffers it fully and flushes at exit, while
    // stderr is unbuffered. So in a merged capture both stderr lines land
    // *before* all of stdout. The Rust program must reproduce that ordering.
    let c = run_merged(&c_binary(), &Spec::new());
    let r = run_merged(&rust_binary(), &Spec::new());
    assert_identical("merged 2>&1", &c, &r);

    let text = String::from_utf8_lossy(&c.stdout).to_string();
    let err_pos = text
        .find("Error: Node with ID 2 already exists")
        .expect("stderr line present in merged capture");
    let banner_pos = text.find("TREE WITH HASHMAP").expect("banner present");
    assert!(
        err_pos < banner_pos,
        "expected unbuffered stderr to precede fully buffered stdout"
    );
}

#[test]
fn stdout_on_a_terminal_is_line_buffered_the_same_way() {
    match (run_on_pty(&c_binary()), run_on_pty(&rust_binary())) {
        (Some(c), Some(r)) => assert_identical("stdout is a tty", &c, &r),
        _ => {
            // script(1) unavailable: still compare the pipe case so this test
            // never becomes a no-op.
            compare("stdout is a pipe (pty fallback)", &Spec::new());
        }
    }
}

#[test]
fn stdout_redirected_to_a_regular_file() {
    let dir = std::env::temp_dir().join(format!("driver-diff-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let cpath = dir.join("c.out");
    let rpath = dir.join("r.out");
    std::fs::write(&cpath, b"").unwrap();
    std::fs::write(&rpath, b"").unwrap();

    let c = run_stdout_to_file(&c_binary(), cpath.to_str().unwrap());
    let r = run_stdout_to_file(&rust_binary(), rpath.to_str().unwrap());
    assert_identical("stdout to file (status+stderr)", &c, &r);

    let cb = std::fs::read(&cpath).unwrap();
    let rb = std::fs::read(&rpath).unwrap();
    assert_eq!(
        cb,
        rb,
        "file contents differ\nC:\n{}\nRust:\n{}",
        String::from_utf8_lossy(&cb),
        String::from_utf8_lossy(&rb)
    );
    let _ = std::fs::remove_dir_all(&dir);
}

// ---------------------------------------------------------------------------
// Failing descriptors
// ---------------------------------------------------------------------------

#[test]
fn stdout_to_dev_full() {
    // Every write fails with ENOSPC. Neither program checks printf's result,
    // so both must still exit 0 with the same stderr.
    if !std::path::Path::new("/dev/full").exists() {
        // Fall back to a descriptor that is open read-only, which fails with
        // EBADF; the point (write errors are ignored) is the same.
        let c = run_stdout_unwritable(&c_binary());
        let r = run_stdout_unwritable(&rust_binary());
        assert_identical("stdout unwritable (EBADF)", &c, &r);
        return;
    }
    let c = run_stdout_to_file(&c_binary(), "/dev/full");
    let r = run_stdout_to_file(&rust_binary(), "/dev/full");
    assert_identical("stdout to /dev/full", &c, &r);
}

#[test]
fn stdout_not_writable() {
    let c = run_stdout_unwritable(&c_binary());
    let r = run_stdout_unwritable(&rust_binary());
    assert_identical("stdout unwritable", &c, &r);
}

#[test]
fn stdout_reader_gone_dies_by_sigpipe() {
    // C keeps SIGPIPE at SIG_DFL, so the flush at exit kills it with signal 13
    // (wait status 141 in a shell). The Rust runtime sets SIGPIPE to SIG_IGN by
    // default, which would silently exit 0 instead.
    let c = run_with_closed_reader(&c_binary(), Stream::Stdout);
    let r = run_with_closed_reader(&rust_binary(), Stream::Stdout);
    assert_identical("stdout reader closed", &c, &r);

    use std::os::unix::process::ExitStatusExt;
    let st = std::process::ExitStatus::from_raw(c.raw_status);
    assert_eq!(
        st.signal(),
        Some(13),
        "expected the C program to die from SIGPIPE, it {}",
        c.describe_status()
    );
}

#[test]
fn stderr_reader_gone_dies_by_sigpipe() {
    // stderr is unbuffered, so this SIGPIPE arrives mid-run, at the first
    // fprintf(stderr, ...) inside test_tree_duplicate_id.
    let c = run_with_closed_reader(&c_binary(), Stream::Stderr);
    let r = run_with_closed_reader(&rust_binary(), Stream::Stderr);
    assert_identical("stderr reader closed", &c, &r);

    use std::os::unix::process::ExitStatusExt;
    let st = std::process::ExitStatus::from_raw(c.raw_status);
    assert_eq!(
        st.signal(),
        Some(13),
        "expected the C program to die from SIGPIPE, it {}",
        c.describe_status()
    );
}
