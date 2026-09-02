//! Smoke test: both `.so`s load, every symbol resolves, and a trivial
//! compile+match agrees.
mod common;
use common::*;

#[test]
fn both_libraries_load_and_all_symbols_resolve() {
    let p = libs();
    assert_eq!(p.c.tag, "C");
    assert_eq!(p.r.tag, "RUST");
}

#[test]
fn trivial_compile_and_match_agree() {
    let p = libs();
    let cp = compile_both(p, b"a(b)c", 5, 0, std::ptr::null_mut(), std::ptr::null_mut(), "smoke")
        .expect("pattern must compile");
    cmp_all_pattern_info(p, &cp, "smoke");
    unsafe {
        let mdc = (p.c.match_data_create_from_pattern)(cp.c, std::ptr::null_mut());
        let mdr = (p.r.match_data_create_from_pattern)(cp.r, std::ptr::null_mut());
        let subj = b"xxabcyy";
        let rc = (p.c.pcre2_match)(cp.c, subj.as_ptr(), subj.len(), 0, 0, mdc, std::ptr::null_mut());
        let rr = (p.r.pcre2_match)(cp.r, subj.as_ptr(), subj.len(), 0, 0, mdr, std::ptr::null_mut());
        assert_eq!(rc, rr);
        assert_eq!(rc, 2);
        let ovc = std::slice::from_raw_parts((p.c.get_ovector_pointer)(mdc), 4);
        let ovr = std::slice::from_raw_parts((p.r.get_ovector_pointer)(mdr), 4);
        assert_eq!(ovc, ovr);
        assert_eq!(ovc, &[2usize, 5, 3, 4]);
        (p.c.match_data_free)(mdc);
        (p.r.match_data_free)(mdr);
    }
    free_code_pair(p, cp);
}

#[test]
fn config_agrees() {
    let p = libs();
    for &w in &[
        cfg::BSR,
        cfg::JIT,
        cfg::LINKSIZE,
        cfg::MATCHLIMIT,
        cfg::NEWLINE,
        cfg::PARENSLIMIT,
        cfg::DEPTHLIMIT,
        cfg::STACKRECURSE,
        cfg::UNICODE,
        cfg::HEAPLIMIT,
        cfg::NEVER_BACKSLASH_C,
        cfg::COMPILED_WIDTHS,
        cfg::TABLES_LENGTH,
        cfg::EFFECTIVE_LINKSIZE,
    ] {
        let mut a: u32 = 0xAAAA_AAAA;
        let mut b: u32 = 0x5555_5555;
        let ra = unsafe { (p.c.config)(w, &mut a as *mut _ as *mut std::ffi::c_void) };
        let rb = unsafe { (p.r.config)(w, &mut b as *mut _ as *mut std::ffi::c_void) };
        assert_eq!(ra, rb, "config({}) rc", w);
        assert_eq!(a, b, "config({}) value", w);
    }
    // String configs
    for &w in &[cfg::UNICODE_VERSION, cfg::VERSION, cfg::JITTARGET] {
        let mut ba = [0u8; 128];
        let mut bb = [0u8; 128];
        let ra = unsafe { (p.c.config)(w, ba.as_mut_ptr() as *mut std::ffi::c_void) };
        let rb = unsafe { (p.r.config)(w, bb.as_mut_ptr() as *mut std::ffi::c_void) };
        assert_eq!(ra, rb, "config({}) rc", w);
        if ra > 0 {
            assert_eq!(&ba[..ra as usize], &bb[..rb as usize], "config({}) string", w);
        }
    }
}
