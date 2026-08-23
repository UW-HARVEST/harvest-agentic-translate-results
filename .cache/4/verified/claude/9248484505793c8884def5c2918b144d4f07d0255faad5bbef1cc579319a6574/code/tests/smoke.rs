// Harness self-check: both .so files load and every symbol the harness binds
// resolves in BOTH libraries.

mod common;
use common::*;

#[test]
fn both_libraries_load_and_all_bound_symbols_resolve() {
    let p = pair();
    // `Api::load` panics on any unresolved symbol, so reaching here proves all
    // of them resolved in both libraries.
    assert_eq!(p.c.name, "C");
    assert_eq!(p.r.name, "rust");
}

#[test]
fn trivial_compile_and_match_agree() {
    let p = pair();
    unsafe {
        let pat = b"(a+)(b*)c";
        let subj = b"xxaaabbc!";
        let mut out = Vec::new();
        for api in [&p.c, &p.r] {
            let mut ec: i32 = 0;
            let mut eo: usize = 0;
            let code = (api.compile)(
                pat.as_ptr(),
                pat.len(),
                0,
                &mut ec,
                &mut eo,
                std::ptr::null_mut(),
            );
            assert!(!code.is_null(), "[{}] compile failed ec={ec}", api.name);
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
            out.push(read_match_out(api, md, rc));
            (api.match_data_free)(md);
            (api.code_free)(code);
        }
        assert_eq!(out[0].rc, 3);
        assert_eq!(out[0], out[1]);
    }
}
