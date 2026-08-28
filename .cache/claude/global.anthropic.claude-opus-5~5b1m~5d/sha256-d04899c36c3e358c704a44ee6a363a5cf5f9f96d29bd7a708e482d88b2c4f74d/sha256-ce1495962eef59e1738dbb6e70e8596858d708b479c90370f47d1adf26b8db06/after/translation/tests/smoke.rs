//! Sanity check that both shared objects load and expose the C ABI.

mod common;

#[test]
fn both_libraries_load_and_agree_on_a_trivial_input() {
    let p = common::pair();
    eprintln!("C  : {}", common::c_so_path().display());
    eprintln!("RUST: {}", common::rust_so_path().display());
    common::diff_all(&p, b"hello");
    common::diff_all(&p, b"\xC3\xA9\x80abc");
    common::diff_all(&p, b"");
}
