//! Harness smoke test: both .so files load and every symbol resolves.
mod common;

use common::*;

#[test]
fn both_libraries_load_and_all_symbols_resolve() {
    let (c, r) = both();
    assert_eq!(c.name, "C");
    assert_eq!(r.name, "Rust");
}

#[test]
fn config_version_matches() {
    let (c, r) = both();
    // PCRE2_CONFIG_VERSION == 11, returns length of version string
    let mut cb = [0u8; 64];
    let mut rb = [0u8; 64];
    unsafe {
        let cl = (c.config)(11, cb.as_mut_ptr() as *mut _);
        let rl = (r.config)(11, rb.as_mut_ptr() as *mut _);
        assert_eq!(cl, rl, "config(VERSION) length");
        assert_eq!(cb, rb, "config(VERSION) string");
    }
}

#[test]
fn trivial_compile_and_match_agree() {
    let (c, r) = both();
    let pat = b"a(b)c";
    let subj = b"xxabcyy";
    unsafe {
        for api in [c, r] {
            let mut ec = 0i32;
            let mut eo = 0usize;
            let code = (api.compile)(
                pat.as_ptr(),
                pat.len(),
                0,
                &mut ec,
                &mut eo,
                std::ptr::null_mut(),
            );
            assert!(!code.is_null(), "{}: compile failed ec={}", api.name, ec);
            let md = (api.match_data_create_from_pattern)(code, std::ptr::null_mut());
            let rc = (api.do_match)(
                code,
                subj.as_ptr(),
                subj.len(),
                0,
                0,
                md,
                std::ptr::null_mut(),
            );
            assert_eq!(rc, 2, "{}: match rc", api.name);
            let ov = (api.get_ovector_pointer)(md);
            assert_eq!((*ov, *ov.add(1)), (2, 5), "{}: ovector", api.name);
            (api.match_data_free)(md);
            (api.code_free)(code);
        }
    }
}
