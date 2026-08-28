//! Differential tests for the public API entry point declared in
//! `c_src/include/goto.h`:
//!
//! ```c
//! int driver(int num, const char* filename);
//! ```
//!
//! Exercises all three returns (`-1` from the `forward_goto_example` error path,
//! `-2` from a NULL `open_with_cleanup`, and `0` on success) and checks that the
//! interleaving of the two callees' output matches.

mod common;

use common::*;
use std::ffi::{CString, c_int};

const NAME: &str = "driver";

#[test]
fn matches_c() {
    let c: libloading::Symbol<Driver> = sym(c_lib(), NAME);
    let r: libloading::Symbol<Driver> = sym(rust_lib(), NAME);

    let dir = TmpDir::new("driver");

    let mut files: Vec<(String, CString)> = Vec::new();
    let mut add = |name: &str, path: &std::path::Path| {
        files.push((name.to_string(), cstr(path.to_str().unwrap())));
    };

    add("empty-file", &dir.file("empty", b""));
    add("one-line", &dir.file("one", b"contents\n"));
    add("multi-line", &dir.file("multi", b"a\nb\nc\n"));
    add("no-trailing-newline", &dir.file("no_nl", b"tail"));
    add("long-line", &dir.file("long", &{
        let mut v = vec![b'z'; 260];
        v.push(b'\n');
        v
    }));
    add("embedded-nul", &dir.file("nul", b"x\0y\nz\n"));
    // Pins the read-loop chunk size through `driver` as well (see
    // tests/open_with_cleanup.rs for the rationale).
    add("nul-past-first-chunk", &dir.file("nul_chunk", &{
        let mut v = vec![b'a'; 150];
        v.push(0);
        v.extend_from_slice(b"tail\n");
        v
    }));
    add("dev-null", std::path::Path::new("/dev/null"));
    add("missing", &dir.path().join("nope"));
    add("name-with-percent", &dir.path().join("%s-%d"));
    add("directory", dir.path());
    files.push(("empty-filename".into(), cstr("")));

    // `num < 0` short-circuits before the file is ever touched; the rest fall
    // through to `open_with_cleanup`.
    let nums: Vec<c_int> = vec![
        i32::MIN,
        -100,
        -2,
        -1,
        0,
        1,
        2,
        21,
        1000,
        0x4000_0000,
        i32::MAX,
    ];

    let mut diffs = Diffs::new();
    let mut seen_ret = std::collections::BTreeSet::new();

    for num in &nums {
        for (fname, path) in &files {
            let case = format!("num={num}, file={fname}");
            let p = path.as_ptr();

            let got_c = capture(|| unsafe { c(*num, p) });
            let got_r = capture(|| unsafe { r(*num, p) });

            seen_ret.insert(got_c.ret);
            diffs.compare(&case, &got_c, &got_r);
        }
    }

    assert_eq!(
        seen_ret,
        [-2, -1, 0].into_iter().collect(),
        "the inputs did not cover every return value of the C `driver`"
    );

    diffs.assert_empty();
}
