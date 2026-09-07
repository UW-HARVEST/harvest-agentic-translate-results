//! Smoke test: both `.so`s load, export `hex2bin`, and agree on a trivial input.

mod common;
use common::*;

#[test]
fn both_libraries_load_and_agree() {
    let out = assert_same("smoke", &Case::new(b"00ff10AbcD"));
    assert_eq!(out.ret, 5, "expected 5 decoded bytes, got {}", out.ret);
    assert_eq!(&out.bin[..5], &[0x00, 0xff, 0x10, 0xab, 0xcd]);
    assert_eq!(out.hex_end, Some(10));
}
