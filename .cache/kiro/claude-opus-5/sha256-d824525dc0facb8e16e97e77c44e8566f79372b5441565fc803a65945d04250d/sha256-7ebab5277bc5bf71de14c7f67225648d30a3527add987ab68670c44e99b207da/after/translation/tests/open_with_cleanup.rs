//! Differential tests for
//!
//! ```c
//! FILE* open_with_cleanup(const char *filename);
//! ```
//!
//! Covers all three exits in the C source: the early `goto cleanup` when
//! `fopen` fails, the `goto cleanup` taken when `ferror` is set after the read
//! loop, and the successful `return fp`.

mod common;

use common::*;
use std::ffi::{CString, c_void};

const NAME: &str = "open_with_cleanup";

#[test]
fn matches_c() {
    let c: libloading::Symbol<OpenWithCleanup> = sym(c_lib(), NAME);
    let r: libloading::Symbol<OpenWithCleanup> = sym(rust_lib(), NAME);

    let dir = TmpDir::new("owc");
    let mut cases: Vec<(String, CString)> = Vec::new();

    let mut add = |name: &str, path: &std::path::Path| {
        cases.push((name.to_string(), cstr(path.to_str().unwrap())));
    };

    // --- fopen succeeds, read loop runs, `return fp` -----------------------
    add("empty", &dir.file("empty", b""));
    add("one-line", &dir.file("one_line", b"hello\n"));
    add("no-trailing-newline", &dir.file("no_nl", b"hello"));
    add(
        "multi-line",
        &dir.file("multi", b"alpha\nbeta\ngamma\ndelta\n"),
    );
    add("blank-lines", &dir.file("blanks", b"\n\n\na\n\n"));
    add("crlf", &dir.file("crlf", b"a\r\nb\r\n"));
    add("single-newline", &dir.file("nl", b"\n"));

    // fgets reads at most sizeof(buffer)-1 == 99 bytes: exercise the boundary.
    for n in [98usize, 99, 100, 101, 199, 200, 201] {
        let mut body = vec![b'x'; n];
        body.push(b'\n');
        let file = dir.file(&format!("len_{n}"), &body);
        add(&format!("line-of-{n}-then-newline"), &file);
    }
    // A long line with no newline at all, spanning several fgets calls.
    add("long-unterminated", &dir.file("long_unterm", &vec![b'y'; 250]));

    // printf("%s", buffer) stops at the first NUL, so embedded NULs are
    // observable behaviour that both sides must reproduce.
    add("embedded-nul", &dir.file("nul", b"ab\0cd\nef\n"));
    add("leading-nul", &dir.file("nul2", b"\0abc\ndef\n"));

    // A NUL makes the *chunking* of the read loop observable: everything after
    // the first NUL in a given `fgets` chunk is dropped by `printf("%s", ...)`,
    // so these inputs pin `sizeof(buffer) == 100` (i.e. 99 bytes per chunk).
    for n in [
        0usize, 1, 50, 96, 97, 98, 99, 100, 101, 102, 150, 196, 197, 198, 199, 200, 201, 297,
    ] {
        let mut body = vec![b'a'; n];
        body.push(0);
        body.extend_from_slice(&vec![b'b'; 30]);
        body.push(b'\n');
        let file = dir.file(&format!("nul_at_{n}"), &body);
        add(&format!("nul-at-offset-{n}"), &file);
    }
    // NULs at a stride that is coprime with any plausible buffer size.
    let mut striped = Vec::new();
    for i in 0..400u32 {
        striped.push(if i % 37 == 36 { 0 } else { b'c' });
    }
    add("nul-every-37-no-newline", &dir.file("striped", &striped));
    // The same, but with newlines too, so chunk ends and line ends interact.
    let mut striped_nl = Vec::new();
    for i in 0..400u32 {
        striped_nl.push(match i % 41 {
            40 => b'\n',
            17 => 0,
            _ => b'd',
        });
    }
    add("nul-and-newline-stride", &dir.file("striped_nl", &striped_nl));

    // The data is passed as the `%s` argument, but conversion specifiers in the
    // *data* must still come out verbatim.
    add("percent-in-data", &dir.file("pct", b"100%% done %d %s %n\n"));

    // Non-UTF-8 bytes.
    add(
        "binary",
        &dir.file("bin", &[0xff, 0xfe, 0x00, 0x41, b'\n', 0x80, 0x0a]),
    );

    // Many lines: several hundred fgets/printf round trips.
    let many: Vec<u8> = (0..500)
        .flat_map(|i| format!("line {i}\n").into_bytes())
        .collect();
    add("many-lines", &dir.file("many", &many));

    add("dev-null", std::path::Path::new("/dev/null"));

    // --- fopen fails, immediate `goto cleanup` ----------------------------
    add("missing-file", &dir.path().join("does-not-exist"));
    add(
        "missing-in-missing-dir",
        &dir.path().join("no-such-dir/file"),
    );
    // The filename is printed with `%s`, so odd names must round-trip.
    add("name-with-percent", &dir.path().join("no-%d-%s-here"));
    add("name-with-newline", &dir.path().join("no\nsuch\nfile"));

    let unreadable = dir.file("unreadable", b"secret\n");
    let _ = std::fs::set_permissions(
        &unreadable,
        <std::fs::Permissions as std::os::unix::fs::PermissionsExt>::from_mode(0o000),
    );
    add("unreadable", &unreadable);

    // --- fopen succeeds but reading fails: `ferror` -> `goto cleanup` ------
    // glibc's fopen(dir, "r") succeeds; the first read() then fails with EISDIR.
    add("directory", dir.path());
    add("proc-self", std::path::Path::new("/proc/self"));

    // --- empty filename ---------------------------------------------------
    cases.push(("empty-filename".into(), cstr("")));

    let mut diffs = Diffs::new();
    let mut saw_success = false;
    let mut saw_open_failure = false;
    let mut saw_read_failure = false;

    for (case, path) in &cases {
        let p = path.as_ptr();

        let got_c = capture(|| {
            let fp: *mut c_void = unsafe { c(p) };
            let st = file_state(fp);
            close_file(fp);
            st
        });
        let got_r = capture(|| {
            let fp: *mut c_void = unsafe { r(p) };
            let st = file_state(fp);
            close_file(fp);
            st
        });

        diffs.compare(case, &got_c, &got_r);

        // Record which of the C function's three exits this input reached, so a
        // silently-vacuous test suite cannot pass.
        match (got_c.ret.is_null, got_c.stderr.is_empty()) {
            (false, true) => saw_success = true,
            (true, false) if case == "directory" || case == "proc-self" => saw_read_failure = true,
            (true, false) => saw_open_failure = true,
            other => panic!("[{case}] unexpected C outcome {other:?}: {}", got_c.describe()),
        }
    }

    assert!(saw_success, "no input reached the successful `return fp` exit");
    assert!(
        saw_open_failure,
        "no input reached the `fopen` failure `goto cleanup`"
    );
    assert!(
        saw_read_failure,
        "no input reached the `ferror` `goto cleanup`"
    );

    diffs.assert_empty();
}
