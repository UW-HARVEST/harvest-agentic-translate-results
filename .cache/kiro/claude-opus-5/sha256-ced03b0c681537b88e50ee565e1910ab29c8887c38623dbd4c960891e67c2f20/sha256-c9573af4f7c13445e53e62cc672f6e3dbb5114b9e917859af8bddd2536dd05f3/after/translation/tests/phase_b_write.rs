//! Phase B — valid-path differential tests for `write_to_file`
//! (rows 34–40 of `CONFIGS.md`).

mod common;

use common::*;
use std::ffi::CString;
use std::path::PathBuf;

fn tmpdir(tag: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("difftest-write-{}-{}", std::process::id(), tag));
    std::fs::create_dir_all(&d).unwrap();
    d
}

/// Calls `write_to_file` on both `.so`s with the same target path, comparing
/// the return code, the resulting file bytes and the stderr diagnostics.
/// `pre` (if given) is written to the target before each call, so truncation
/// semantics of mode `"w"` are exercised.
fn check_write(b: &Both, path: &PathBuf, content: &[u8], pre: Option<&[u8]>) {
    let fname = cs(path.to_str().unwrap());
    let cont = CString::new(content.to_vec()).expect("content must not contain NUL");

    let run = |api: &Api| {
        let _ = std::fs::remove_file(path);
        if let Some(p) = pre {
            std::fs::write(path, p).unwrap();
        }
        let (rc, err) =
            capture_stderr(|| unsafe { (api.write_to_file)(fname.as_ptr(), cont.as_ptr()) });
        let bytes = std::fs::read(path).ok();
        let _ = std::fs::remove_file(path);
        (rc, bytes, err)
    };

    let (rc_c, bytes_c, err_c) = run(&b.c);
    let (rc_r, bytes_r, err_r) = run(&b.rs);

    assert_eq!(rc_c, rc_r, "write_to_file return code mismatch (len {})", content.len());
    assert_eq!(bytes_c, bytes_r, "write_to_file file content mismatch (len {})", content.len());
    assert_eq!(
        String::from_utf8_lossy(&err_c),
        String::from_utf8_lossy(&err_r),
        "write_to_file stderr mismatch"
    );
    // Sanity: the happy path must actually have produced the content.
    if rc_c == 0 {
        assert_eq!(bytes_c.as_deref(), Some(content), "content not written verbatim");
    }
}

#[test]
fn row34_write_empty_content() {
    let b = load_both();
    let d = tmpdir("row34");
    check_write(&b, &d.join("out.txt"), b"", None);
}

#[test]
fn row35_write_single_line() {
    let b = load_both();
    let d = tmpdir("row35");
    check_write(&b, &d.join("out.txt"), b"1 2 3\n", None);
    check_write(&b, &d.join("out.txt"), b"no trailing newline", None);
    check_write(&b, &d.join("out.txt"), b"x", None);
}

#[test]
fn row36_write_multiline_and_high_bytes() {
    let b = load_both();
    let d = tmpdir("row36");
    check_write(&b, &d.join("out.txt"), b"a\nb\nc\n", None);
    check_write(&b, &d.join("out.txt"), b"tab\there\nand\r\ncrlf\n", None);
    let high: Vec<u8> = (1u8..=255).collect();
    check_write(&b, &d.join("out.txt"), &high, None);
}

#[test]
fn row37_write_truncates_existing() {
    let b = load_both();
    let d = tmpdir("row37");
    let long = vec![b'Z'; 5000];
    check_write(&b, &d.join("out.txt"), b"short\n", Some(&long));
    check_write(&b, &d.join("out.txt"), b"", Some(&long));
}

#[test]
fn row38_write_format_specifiers_are_literal() {
    let b = load_both();
    let d = tmpdir("row38");
    for c in [
        &b"%s"[..],
        &b"%d %d %d"[..],
        &b"%n"[..],
        &b"100%% done"[..],
        &b"%.*s%p%%"[..],
    ] {
        check_write(&b, &d.join("out.txt"), c, None);
    }
}

#[test]
fn row39_write_larger_than_bufsiz() {
    let b = load_both();
    let d = tmpdir("row39");
    let big: Vec<u8> = (0..200_000usize).map(|i| b'a' + (i % 26) as u8).collect();
    check_write(&b, &d.join("out.txt"), &big, None);
    let with_newlines: Vec<u8> = (0..100_000usize)
        .map(|i| if i % 80 == 79 { b'\n' } else { b'q' })
        .collect();
    check_write(&b, &d.join("out.txt"), &with_newlines, None);
}

#[test]
fn row40_write_randomized() {
    let b = load_both();
    let d = tmpdir("row40");
    let path = d.join("out.txt");
    let mut rng = Rng::new(0x5EED_0040);
    for _ in 0..200 {
        let len = rng.range(0, 6000) as usize;
        let content: Vec<u8> = (0..len).map(|_| rng.range(1, 255) as u8).collect();
        let pre: Option<Vec<u8>> = if rng.bool() {
            Some(vec![b'P'; rng.range(0, 9000) as usize])
        } else {
            None
        };
        check_write(&b, &path, &content, pre.as_deref());
    }
}

#[test]
fn row_write_relative_and_nested_paths() {
    let b = load_both();
    let d = tmpdir("rowpaths");
    let nested = d.join("sub");
    std::fs::create_dir_all(&nested).unwrap();
    check_write(&b, &nested.join("deep.txt"), b"nested\n", None);
    // A name with spaces and unusual (but legal) characters.
    check_write(&b, &d.join("a b%c-d.txt"), b"weird name\n", None);
}
