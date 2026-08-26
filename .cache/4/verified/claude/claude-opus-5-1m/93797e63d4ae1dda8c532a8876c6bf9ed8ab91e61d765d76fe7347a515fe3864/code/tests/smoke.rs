mod common;
use common::*;

#[test]
fn smoke_ffi_run() {
    assert_run_matches(House::new(2, 5, 2.5), 7, "nominal");
}

#[test]
fn smoke_ffi_main() {
    assert_ffi_main_matches(b"7\n", "nominal");
}

#[test]
fn smoke_exe() {
    assert_exe_matches(b"7\n", "nominal");
}
