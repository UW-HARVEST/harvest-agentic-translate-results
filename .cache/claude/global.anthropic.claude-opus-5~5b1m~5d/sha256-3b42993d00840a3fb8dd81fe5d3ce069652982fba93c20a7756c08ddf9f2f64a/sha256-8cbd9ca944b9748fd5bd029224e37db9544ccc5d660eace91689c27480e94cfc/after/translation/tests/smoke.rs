// Harness smoke test: proves both .so's load, both export all five symbols,
// and that stdout/stderr capture actually observes the libraries' printf output.

mod common;
use common::*;

#[test]
fn both_libraries_load_and_export_all_five_symbols() {
    let _g = lock();
    let l = libs();
    // Taking each symbol panics if it is missing from either .so.
    let _ = l.c.envy();
    let _ = l.c.parse_env_numeric();
    let _ = l.c.init_config_from_env();
    let _ = l.c.perform_operation();
    let _ = l.c.apply_bit_operations();
    let _ = l.rs.envy();
    let _ = l.rs.parse_env_numeric();
    let _ = l.rs.init_config_from_env();
    let _ = l.rs.perform_operation();
    let _ = l.rs.apply_bit_operations();
    println!("C   .so: {}", l.c_path.display());
    println!("Rust.so: {}", l.rs_path.display());
}

#[test]
fn capture_observes_library_stdout() {
    let _g = lock();
    env_clear_all();
    env_set("PROG_VERBOSE", "1");
    let l = libs();
    let f = l.c.envy();
    let cap = capture(|| unsafe { f(1, 2, 3, 4) });
    env_clear_all();
    assert!(
        cap.out.starts_with(b"Verbose mode enabled\n"),
        "capture did not see the library's stdout, got: {:?}",
        String::from_utf8_lossy(&cap.out)
    );
    assert!(cap.err.is_empty());
}

#[test]
fn capture_observes_library_stderr() {
    let _g = lock();
    env_clear_all();
    env_set("PROG_BASE_OFFSET", "1,2");
    let l = libs();
    let f = l.c.parse_env_numeric();
    let name = std::ffi::CString::new("PROG_BASE_OFFSET").unwrap();
    let cap = capture(|| unsafe { f(name.as_ptr(), 64) });
    env_clear_all();
    assert_eq!(cap.ret, 64);
    assert_eq!(
        cap.err,
        b"Warning: Invalid character in PROG_BASE_OFFSET\n".to_vec(),
        "got {:?}",
        String::from_utf8_lossy(&cap.err)
    );
    assert!(cap.out.is_empty());
}
