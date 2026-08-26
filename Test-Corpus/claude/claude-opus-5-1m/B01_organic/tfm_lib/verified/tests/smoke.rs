//! Harness self-test: both `.so`s load and export `tfm`.

mod common;

#[test]
fn both_libraries_export_tfm() {
    let im = common::impls();
    let _c = im.c();
    let _r = im.rust();
}

#[test]
fn trivial_agreement() {
    common::diff1("smoke", 1.0f32.to_bits(), 2.0f32.to_bits(), 3.0f32.to_bits());
    common::diff1("smoke", 2.0f32.to_bits(), 1.0f32.to_bits(), 3.0f32.to_bits());
}
