//! Harness bring-up: proves both `.so`s load, the whole public API resolves,
//! and a trivial write plus an error path agree byte-for-byte.
mod common;
use common::*;

#[test]
fn version_number_matches() {
    let (c, r) = libs();
    unsafe {
        assert_eq!(
            (c.api.png_access_version_number)(),
            (r.api.png_access_version_number)()
        );
    }
}

#[test]
fn version_strings_match() {
    let (c, r) = libs();
    let mut run = |l: &Lib| unsafe {
        let n = std::ptr::null_mut();
        for (name, p) in [
            ("copyright", (l.api.png_get_copyright)(n)),
            ("header_ver", (l.api.png_get_header_ver)(n)),
            ("header_version", (l.api.png_get_header_version)(n)),
            ("libpng_ver", (l.api.png_get_libpng_ver)(n)),
        ] {
            let s = std::ffi::CStr::from_ptr(p).to_string_lossy().into_owned();
            log(format!("{name}={s}"));
        }
    };
    diff_bare("L1: version strings", &c, &r, &mut run);
}

#[test]
fn trivial_write_is_identical() {
    let (c, r) = libs();
    let mut run = |l: &Lib| -> Report {
        write_session(l, &mut |l, png, info| unsafe {
            (l.api.png_set_IHDR)(
                png,
                info,
                4,
                4,
                8,
                PNG_COLOR_TYPE_RGB,
                PNG_INTERLACE_NONE,
                PNG_COMPRESSION_TYPE_BASE,
                PNG_FILTER_TYPE_BASE,
            );
            (l.api.png_write_info)(png, info);
            let row: Vec<u8> = (0..12u8).collect();
            for _ in 0..4 {
                (l.api.png_write_row)(png, row.as_ptr());
            }
            (l.api.png_write_end)(png, info);
        })
    };
    diff("smoke: write 4x4 RGB8", &c, &r, &mut run);
    let rep = run(&c);
    assert!(rep.out.len() > 40, "no output: {}", rep.brief());
    assert_eq!(
        &rep.out[..8],
        &[0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a]
    );
}

#[test]
fn error_path_is_caught_and_matches() {
    let (c, r) = libs();
    let mut run = |l: &Lib| -> Report {
        write_session(l, &mut |l, png, info| unsafe {
            // bit depth 7 is not a legal PNG bit depth
            (l.api.png_set_IHDR)(
                png,
                info,
                1,
                1,
                7,
                PNG_COLOR_TYPE_GRAY,
                PNG_INTERLACE_NONE,
                PNG_COMPRESSION_TYPE_BASE,
                PNG_FILTER_TYPE_BASE,
            );
            log("survived set_IHDR");
        })
    };
    diff("smoke: invalid bit depth", &c, &r, &mut run);
    let rep = run(&c);
    assert!(rep.error.is_some(), "expected error: {}", rep.brief());
    assert!(!rep.log.contains(&"survived set_IHDR".to_string()));
}
