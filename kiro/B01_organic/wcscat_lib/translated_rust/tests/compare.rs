use libloading::{Library, Symbol};
use std::ptr;

type WcharT = i32;
type WcscatFn = unsafe extern "C" fn(*mut WcharT, usize, *const WcharT) -> i32;

fn c_lib() -> Library {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/c_src/build/libwcscat_lib.so");
    unsafe { Library::new(path).expect("Failed to load C library") }
}

fn rust_lib() -> Library {
    // cargo puts the built cdylib in target/debug/
    let manifest = env!("CARGO_MANIFEST_DIR");
    let path = format!("{manifest}/target/debug/libwcscat_lib.so");
    unsafe { Library::new(&path).expect("Failed to load Rust library") }
}

fn call_both(
    dst_init: &[WcharT],
    num_elem: usize,
    src: Option<&[WcharT]>,
    use_null_dst: bool,
) -> ((i32, Vec<WcharT>), (i32, Vec<WcharT>)) {
    let c = c_lib();
    let r = rust_lib();
    let c_fn: Symbol<WcscatFn> = unsafe { c.get(b"wcscat").unwrap() };
    let r_fn: Symbol<WcscatFn> = unsafe { r.get(b"wcscat").unwrap() };

    let run = |f: &Symbol<WcscatFn>| -> (i32, Vec<WcharT>) {
        let mut dst = dst_init.to_vec();
        let ret = unsafe {
            let dp = if use_null_dst { ptr::null_mut() } else { dst.as_mut_ptr() };
            let sp = match src { Some(s) => s.as_ptr(), None => ptr::null() };
            f(dp, num_elem, sp)
        };
        (ret, dst)
    };

    (run(&c_fn), run(&r_fn))
}

#[test]
fn test_null_dst() {
    let src = [b'a' as WcharT, 0];
    let ((cr, _), (rr, _)) = call_both(&[], 10, Some(&src), true);
    assert_eq!(cr, rr, "null dst: return c={cr} r={rr}");
}

#[test]
fn test_zero_num_elem() {
    let init = [b'x' as WcharT, 0, 0, 0];
    let src = [b'a' as WcharT, 0];
    let ((cr, cd), (rr, rd)) = call_both(&init, 0, Some(&src), false);
    assert_eq!(cr, rr, "zero numElem: return c={cr} r={rr}");
    assert_eq!(cd, rd, "zero numElem: dst mismatch");
}

#[test]
fn test_null_src() {
    let init = [b'x' as WcharT, 0, 0, 0];
    let ((cr, cd), (rr, rd)) = call_both(&init, 4, None, false);
    assert_eq!(cr, rr, "null src: return c={cr} r={rr}");
    assert_eq!(cd, rd, "null src: dst mismatch");
}

#[test]
fn test_normal_concat() {
    let init = [b'H' as WcharT, b'i' as WcharT, 0, 0, 0, 0];
    let src = [b'L' as WcharT, b'o' as WcharT, 0];
    let ((cr, cd), (rr, rd)) = call_both(&init, 6, Some(&src), false);
    assert_eq!(cr, rr, "normal: return c={cr} r={rr}");
    assert_eq!(cd, rd, "normal: dst mismatch");
}

#[test]
fn test_overflow() {
    let init = [b'A' as WcharT, b'B' as WcharT, 0, 0];
    let src = [b'C' as WcharT, b'D' as WcharT, 0];
    let ((cr, cd), (rr, rd)) = call_both(&init, 4, Some(&src), false);
    assert_eq!(cr, rr, "overflow: return c={cr} r={rr}");
    assert_eq!(cd, rd, "overflow: dst mismatch");
}

#[test]
fn test_concat_empty_src() {
    let init = [b'A' as WcharT, 0, 0, 0];
    let src = [0 as WcharT];
    let ((cr, cd), (rr, rd)) = call_both(&init, 4, Some(&src), false);
    assert_eq!(cr, rr, "empty src: return c={cr} r={rr}");
    assert_eq!(cd, rd, "empty src: dst mismatch");
}

#[test]
fn test_concat_to_empty_dst() {
    let init = [0 as WcharT, 0, 0, 0];
    let src = [b'X' as WcharT, b'Y' as WcharT, 0];
    let ((cr, cd), (rr, rd)) = call_both(&init, 4, Some(&src), false);
    assert_eq!(cr, rr, "empty dst: return c={cr} r={rr}");
    assert_eq!(cd, rd, "empty dst: dst mismatch");
}

#[test]
fn test_exact_fit() {
    let init = [b'A' as WcharT, 0, 0, 0];
    let src = [b'B' as WcharT, b'C' as WcharT, 0];
    let ((cr, cd), (rr, rd)) = call_both(&init, 4, Some(&src), false);
    assert_eq!(cr, rr, "exact fit: return c={cr} r={rr}");
    assert_eq!(cd, rd, "exact fit: dst mismatch");
}
