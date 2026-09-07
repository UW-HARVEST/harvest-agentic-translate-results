//! Harness smoke test: proves both `.so` files load and the simplest
//! low-level symbol agrees.

mod common;
use common::*;

#[test]
fn smoke_libraries_load_and_hash_agrees() {
    run(0x31415926, |c, r| unsafe {
        let mut buf = *b"hello\0";
        let hc = (c.hash_string)(buf.as_mut_ptr() as *mut i8, 12345);
        let hr = (r.hash_string)(buf.as_mut_ptr() as *mut i8, 12345);
        assert_eq!(hc, hr, "hash_string mismatch");

        let mut data = [1u8, 2, 3, 4, 5, 6, 7, 8, 9];
        let bc = (c.hash_bytes)(data.as_mut_ptr() as *mut _, 9, 777);
        let br = (r.hash_bytes)(data.as_mut_ptr() as *mut _, 9, 777);
        assert_eq!(bc, br, "hash_bytes mismatch");
    });
}

#[test]
fn smoke_sh_puts_stdout_matches() {
    run_locked(0x31415926, |c, r| unsafe {
        let oc = capture_stdout("c", || (c.sh_puts)(3));
        let or_ = capture_stdout("r", || (r.sh_puts)(3));
        assert_eq!(
            String::from_utf8_lossy(&oc),
            String::from_utf8_lossy(&or_),
            "sh_puts(3) stdout differs"
        );
        assert!(!oc.is_empty(), "expected sh_puts to print something");
    });
}
