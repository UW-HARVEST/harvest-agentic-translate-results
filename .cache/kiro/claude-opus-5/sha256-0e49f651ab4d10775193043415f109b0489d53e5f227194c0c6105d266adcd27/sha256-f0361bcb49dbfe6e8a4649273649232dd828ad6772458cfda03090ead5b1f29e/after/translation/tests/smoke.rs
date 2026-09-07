//! Harness self-check: proves the capture machinery actually observes output
//! from *both* shared objects before any conclusions are drawn from it.

mod common;

use common::{assert_same, expected, Impl, Libs};

#[test]
fn both_libs_expose_driver_and_capture_works() {
    let libs = Libs::load();
    eprintln!("C   .so: {}", libs.c_path.display());
    eprintln!("Rust.so: {}", libs.rust_path.display());

    let c = libs.run(Impl::C, 3);
    let r = libs.run(Impl::Rust, 3);

    assert_eq!(
        c, b"0 0\n1 2\n2 4\n",
        "harness captured nothing / wrong bytes from the C .so: {:?}",
        String::from_utf8_lossy(&c)
    );
    assert_same("smoke", 3, &c, &r);
    assert_eq!(c, expected(3));
}

#[test]
fn capture_is_isolated_between_calls() {
    let libs = Libs::load();
    for x in [1, 0, 2, 0, 5] {
        let c = libs.run(Impl::C, x);
        let r = libs.run(Impl::Rust, x);
        assert_eq!(c, expected(x), "C output wrong for x={x}");
        assert_same("isolation", x, &c, &r);
    }
}
