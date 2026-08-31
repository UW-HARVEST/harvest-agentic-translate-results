//! Harness smoke test: proves both shared objects load, every one of the 130
//! exported symbols resolves in BOTH of them, and a trivial round trip agrees.

mod common;
use common::*;

#[test]
fn both_libraries_expose_every_symbol() {
    let _g = global_state_lock();
    // Api::load panics with the offending symbol name if anything is missing,
    // so simply constructing both APIs is the parity check.
    let (c, r) = both();
    assert_eq!(c.which, "C");
    assert_eq!(r.which, "Rust");
}

#[test]
fn version_matches() {
    let _g = global_state_lock();
    let (c, r) = both();
    unsafe {
        diff_eq!(
            cbytes((c.jansson_version_str)()),
            cbytes((r.jansson_version_str)()),
            "jansson_version_str"
        );
        for (ma, mi, mu) in [
            (0, 0, 0),
            (2, 15, 0),
            (2, 15, 1),
            (2, 14, 9),
            (3, 0, 0),
            (1, 99, 99),
            (-1, -1, -1),
            (i32::MAX, i32::MAX, i32::MAX),
            (i32::MIN, i32::MIN, i32::MIN),
        ] {
            // Compare the EXACT int, not just its sign: the C returns the raw
            // component difference, and callers can observe the magnitude.
            diff_eq!(
                (c.jansson_version_cmp)(ma, mi, mu),
                (r.jansson_version_cmp)(ma, mi, mu),
                "jansson_version_cmp({ma},{mi},{mu})"
            );
        }
    }
}

#[test]
fn dtoa_divmax_data_symbol_matches() {
    let _g = global_state_lock();
    let (c, r) = both();
    diff_eq!(c.dtoa_divmax(), r.dtoa_divmax(), "dtoa_divmax");
}

#[test]
fn trivial_round_trip() {
    let _g = global_state_lock();
    let (c, r) = both();
    let input = cs(r#"{"a":[1,2,3],"b":"x","c":true,"d":null,"e":1.5}"#);
    unsafe {
        let mut ce = json_error_t::new();
        let mut re = json_error_t::new();
        let cj = (c.json_loads)(input.as_ptr(), 0, &mut ce);
        let rj = (r.json_loads)(input.as_ptr(), 0, &mut re);
        assert!(!cj.is_null(), "C failed to parse: {}", ce.text_str());
        assert!(!rj.is_null(), "Rust failed to parse: {}", re.text_str());

        let cd = (c.json_dumps)(cj, JSON_SORT_KEYS);
        let rd = (r.json_dumps)(rj, JSON_SORT_KEYS);
        diff_eq!(cbytes(cd), cbytes(rd), "json_dumps round trip");

        jfree(c, cd as *mut _);
        jfree(r, rd as *mut _);
        decref(c, cj);
        decref(r, rj);
    }
}
