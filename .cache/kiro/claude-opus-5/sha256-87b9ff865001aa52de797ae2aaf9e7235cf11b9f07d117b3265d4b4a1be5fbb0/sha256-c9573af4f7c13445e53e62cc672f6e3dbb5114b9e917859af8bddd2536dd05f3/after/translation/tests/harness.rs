//! Harness self-check: proves the capture machinery and the dual-`dlopen`
//! loading actually work before the real differential rows rely on them.

mod common;

use common::*;

#[test]
fn harness_finds_both_shared_objects() {
    let c = c_so_path();
    let r = rust_so_path();
    assert!(c.is_file(), "C .so missing at {}", c.display());
    assert!(r.is_file(), "Rust .so missing at {}", r.display());
    eprintln!("C   .so: {}", c.display());
    eprintln!("Rust.so: {}", r.display());
    let _ = load_pair();
}

#[test]
fn harness_capture_roundtrips_file_and_pipe() {
    for sink in [Sink::File, Sink::Pipe] {
        let ((), bytes) = capture(sink, Buffering::Default, || unsafe {
            libc::printf(b"probe-%d\n\0".as_ptr() as *const std::ffi::c_char, 7);
        });
        assert_eq!(
            bytes, b"probe-7\n",
            "capture({sink:?}) lost or mangled bytes: {:?}",
            show(&bytes)
        );
    }
}

#[test]
fn harness_capture_is_empty_when_nothing_is_written() {
    let ((), bytes) = capture(Sink::File, Buffering::Default, || {});
    assert!(bytes.is_empty(), "expected empty capture, got {:?}", show(&bytes));
}

#[test]
fn harness_rng_is_deterministic() {
    let a: Vec<u64> = (0..8).scan(Rng::new(SEED), |r, _| Some(r.next_u64())).collect();
    let b: Vec<u64> = (0..8).scan(Rng::new(SEED), |r, _| Some(r.next_u64())).collect();
    assert_eq!(a, b, "fixed seed must reproduce the same sequence");
}
