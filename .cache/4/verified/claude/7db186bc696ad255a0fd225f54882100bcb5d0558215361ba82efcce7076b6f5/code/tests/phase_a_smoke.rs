//! Phase A sanity: both shared objects load and export all three symbols.

mod common;
use common::*;

#[test]
fn both_shared_objects_load_and_export_every_symbol() {
    let p = pair();
    eprintln!("C   .so: {}", p.c.path.display());
    eprintln!("Rust.so: {}", p.rs.path.display());
    // `pair()` panics if any of get_os_arch / w_regexec / parse_uname_string is
    // missing from either object, so reaching here is the assertion.
    assert_eq!(p.c.name, "C");
    assert_eq!(p.rs.name, "Rust");
}

#[test]
fn smoke_all_three_entry_points_agree() {
    diff_get_os_arch("smoke", b"Linux |amd64 stuff");
    diff_w_regexec("smoke", Some(b"^([0-9]+)\\.*"), Some(b"10.0.19041"), 2, Some(4));
    diff_parse_uname("smoke", b"Microsoft Windows 10 [Ver: 10.0.19041.1234]", 0);
    diff_parse_uname("smoke", b"x86_64 [Ubuntu|ubuntu: 22.04.3 LTS (Jammy Jellyfish)]", 0);
}
