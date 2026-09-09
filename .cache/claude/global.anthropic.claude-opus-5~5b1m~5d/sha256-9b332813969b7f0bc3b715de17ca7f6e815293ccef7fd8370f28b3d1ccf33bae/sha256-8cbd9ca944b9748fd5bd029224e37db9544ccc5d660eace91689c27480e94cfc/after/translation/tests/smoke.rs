//! Sanity check that both shared objects load and agree on the trivial cases.
mod common;
use common::*;

#[test]
fn both_libs_load_and_report_same_version() {
    let p = pair();
    unsafe {
        let cv = cstr_bytes(p.c.jansson_version_str());
        let rv = cstr_bytes(p.r.jansson_version_str());
        assert_eq!(cv, rv, "jansson_version_str mismatch");
        assert_eq!(cv, Some(b"2.15.0".to_vec()));
        for (a, b, c) in [
            (2, 15, 0),
            (2, 15, 1),
            (2, 14, 0),
            (3, 0, 0),
            (1, 0, 0),
            (2, 16, 0),
            (-1, -1, -1),
            (i32::MAX, i32::MIN, 0),
        ] {
            assert_eq!(
                p.c.jansson_version_cmp(a, b, c),
                p.r.jansson_version_cmp(a, b, c),
                "jansson_version_cmp({a},{b},{c})"
            );
        }
    }
}

#[test]
fn trivial_roundtrip() {
    let p = pair();
    unsafe {
        let src = cs("{\"a\":[1,2,3],\"b\":\"x\",\"c\":true,\"d\":null,\"e\":1.5}");
        let mut ce = json_error_t::default();
        let mut re = json_error_t::default();
        let cj = p.c.json_loads(src.as_ptr(), 0, &mut ce);
        let rj = p.r.json_loads(src.as_ptr(), 0, &mut re);
        assert!(!cj.is_null(), "C parse failed: {:?}", ce.snap().text);
        assert!(!rj.is_null(), "Rust parse failed: {:?}", re.snap().text);
        let cd = p.c.dumps(cj, 0);
        let rd = p.r.dumps(rj, 0);
        assert_eq!(cd, rd, "dump mismatch: {} vs {}", show(&cd), show(&rd));
        p.c.json_decref(cj);
        p.r.json_decref(rj);
    }
}
