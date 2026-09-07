mod common;
use common::*;

#[test]
fn harness_loads_both_libraries_and_captures_stdout() {
    let l = libs();
    eprintln!("C   .so: {}", l.c_path.display());
    eprintln!("Rust.so: {}", l.rust_path.display());

    let c = run(Impl::C, 9);
    let r = run(Impl::Rust, 9);
    assert_eq!(c, b"9\n".to_vec(), "C output for sieve(9)");
    assert_same_bytes("sieve(9)", &c, &r);

    let c = run(Impl::C, 0);
    let r = run(Impl::Rust, 0);
    assert_eq!(c, b"0\n1\n2\n3\n4\n5\n6\n7\n8\n9\n".to_vec());
    assert_same_bytes("sieve(0)", &c, &r);
}

#[test]
fn int_is_four_bytes() {
    assert_eq!(std::mem::size_of::<libc::c_int>(), 4);
}
