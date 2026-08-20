//! Infrastructure smoke test: both shared objects load, both entry points can
//! be called through the FFI boundary, and every capture mechanism works.

mod common;

use common::*;

#[test]
fn both_shared_objects_load_and_export_driver_and_main() {
    let p = pair();
    let c = capture_child(|| unsafe { (p.c.driver)(0x2a) });
    let r = capture_child(|| unsafe { (p.rs.driver)(0x2a) });
    assert_eq!(as_text(&c.out), "2a000000\n", "C driver(42)");
    assert_eq!(as_text(&r.out), "2a000000\n", "Rust driver(42)");
    assert_eq!(c.status, r.status);
}

#[test]
fn capture_mechanisms_agree() {
    let vals: Vec<i32> = vec![0, 1, -1, 0x0a0b0c0d];
    diff_driver_batch(&vals, "smoke/file");
    diff_driver_batch_piped(&vals, "smoke/pipe");
}

#[test]
fn forked_main_runner_works() {
    let run = diff_main_input(b"42\n");
    assert_eq!(as_text(&run.out), "2a000000\n");
    assert_eq!(run.status, Status::Exited(0));
}

#[test]
fn executables_agree_on_a_simple_input() {
    diff_exe_file_stdin(b"42\n");
    diff_exe_pipe_stdin(b"42\n");
}
