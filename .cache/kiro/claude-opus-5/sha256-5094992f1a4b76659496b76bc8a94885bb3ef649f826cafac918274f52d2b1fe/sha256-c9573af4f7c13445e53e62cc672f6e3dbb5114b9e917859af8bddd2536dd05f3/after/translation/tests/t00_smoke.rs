//! Phase A/D: smoke test — both libraries load, all symbols resolve.
mod common;
use common::*;

#[test]
fn both_libraries_load_and_all_symbols_resolve() {
    let (cc, rr) = (c(), r());
    assert_eq!(cc.tag, "C");
    assert_eq!(rr.tag, "RUST");
}

#[test]
fn version_symbols_match() {
    unsafe {
        let cv = from_cstr((c().jansson_version_str)()).unwrap();
        let rv = from_cstr((r().jansson_version_str)()).unwrap();
        assert_eq!(cv, rv, "jansson_version_str");
        assert_eq!(cv, "2.15.0");
    }
}

#[test]
fn hashtable_seed_variable_is_exported_and_seeded() {
    // pair() seeds both with 0x5eed1234.
    assert_eq!(c().hashtable_seed(), 0x5eed_1234, "C hashtable_seed");
    assert_eq!(r().hashtable_seed(), 0x5eed_1234, "RUST hashtable_seed");
}

#[test]
fn dtoa_divmax_variable_is_exported() {
    assert_eq!(c().dtoa_divmax(), r().dtoa_divmax());
}

#[test]
fn trivial_roundtrip_matches() {
    unsafe {
        let src = cs(r#"{"a":[1,2,3],"b":"x"}"#);
        for flags in [0usize, JSON_COMPACT, json_indent(2)] {
            let cj = (c().json_loads)(src.as_ptr(), 0, std::ptr::null_mut());
            let rj = (r().json_loads)(src.as_ptr(), 0, std::ptr::null_mut());
            assert!(!cj.is_null() && !rj.is_null());
            let cd = dumps(c(), cj, flags);
            let rd = dumps(r(), rj, flags);
            assert_bytes_eq(&format!("dumps flags={flags:#x}"), &cd, &rd);
            decref(c(), cj);
            decref(r(), rj);
        }
    }
}
