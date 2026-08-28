mod common;

#[test]
fn harness_loads_both_shared_objects() {
    let c = common::c();
    let r = common::rust();
    eprintln!("C    .so: {}", c.path.display());
    eprintln!("Rust .so: {}", r.path.display());
    assert_ne!(c.path, r.path);
}

#[test]
fn smoke_a_few_triples() {
    common::assert_same("smoke", 0.0, 0.0, 0.5);
    common::assert_same("smoke", 30.0, 1.0, 1.0);
    common::assert_same("smoke", 200.0, 0.5, 0.25);
    common::assert_same("smoke", f32::NAN, 0.5, 0.25);
    common::assert_same("smoke", f32::INFINITY, 0.5, 0.25);
    common::assert_same("smoke", -90.0, 0.5, 0.25);
}
