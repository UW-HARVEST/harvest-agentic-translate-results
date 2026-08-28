//! Lowest level exported function: `extractFilename`.
//!
//! Both implementations receive the *same* buffer pointer, so the returned
//! pointers must be bit-identical (they point into the input).

mod common;

use common::{GuardedCStr, Libs};
use std::os::raw::c_char;

fn paths() -> Vec<&'static [u8]> {
    vec![
        b"",
        b"/",
        b"//",
        b"///",
        b"a",
        b"ab",
        b"/a",
        b"a/",
        b"/a/",
        b"file.txt",
        b"/file.txt",
        b"dir/file.txt",
        b"/abs/dir/file.txt",
        b"./rel/file.txt",
        b"../up/file.txt",
        b"dir/",
        b"dir//file",
        b"dir/sub/",
        b"C:\\win\\path\\file.txt",
        b"C:/win/path/file.txt",
        b"mixed\\sep/path\\file",
        b"trailing\\",
        b"\\",
        b"\\\\server\\share\\file",
        b"no-separator-at-all",
        b"space in name/file name.bin",
        b".hidden",
        b"dir/.hidden",
        b"a/b/c/d/e/f/g/h/i/j/k",
        b"\xff\xfe/\x80\x81",
        b"\x7f/\x01\x02",
        b"tab\there/file",
        b"very/long/path/that/keeps/going/and/going/and/going/until/the/end/file.zst",
    ]
}

fn separators() -> Vec<c_char> {
    let mut v: Vec<c_char> = vec![];
    for b in [
        0u8, b'/', b'\\', b'a', b'.', b':', b' ', b'\t', 0x01, 0x7f, 0x80, 0x81, 0xfe, 0xff,
    ] {
        v.push(b as i8 as c_char);
    }
    v
}

#[test]
fn extract_filename_matches_c() {
    let libs = Libs::load();
    let (c_fn, r_fn) = libs.extract_filename();

    let mut cases = 0usize;
    for guard in [0u8, b'/', b'x'] {
        for p in paths() {
            let s = GuardedCStr::new(guard, p);
            let base = s.ptr();
            for sep in separators() {
                let c_ret = unsafe { c_fn(base, sep) };
                let r_ret = unsafe { r_fn(base, sep) };
                assert_eq!(
                    c_ret as usize as isize - base as usize as isize,
                    r_ret as usize as isize - base as usize as isize,
                    "extractFilename mismatch for path {:?} separator {}",
                    String::from_utf8_lossy(p),
                    sep as u8
                );
                assert_eq!(c_ret, r_ret);
                cases += 1;
            }
        }
    }
    assert!(cases > 0);
    eprintln!("extractFilename: {cases} cases compared");
}
