//! Minimal sanity check: both `.so`s load and export `parse_number`, and a
//! happy-path call agrees.

mod common;

use common::*;

#[test]
fn both_libraries_export_parse_number() {
    let _c = c_parse_number();
    let _r = rust_parse_number();
}

#[test]
fn simple_integer_agrees() {
    let o = diff_str("123");
    assert_eq!(o.ret, 1);
    assert_eq!(o.valueint, 123);
    assert_eq!(f64::from_bits(o.valuedouble_bits), 123.0);
    assert_eq!(o.type_, 1 << 3);
    assert_eq!(o.buf_offset, 3);
}
