use libloading::{Library, Symbol};
use std::os::raw::c_int;
use std::ptr;

type WcharT = i32;
type WcscatFn = unsafe extern "C" fn(*mut WcharT, usize, *const WcharT) -> c_int;

const C_LIB: &str = "c_src/build/libtranslated_rust.so";
const RUST_LIB: &str = "target/debug/libwcscat_lib.so";

fn load_lib(path: &str) -> Library {
    unsafe { Library::new(path).unwrap_or_else(|e| panic!("failed to load {}: {}", path, e)) }
}

fn to_wide(s: &str) -> Vec<WcharT> {
    let mut v: Vec<WcharT> = s.chars().map(|c| c as WcharT).collect();
    v.push(0);
    v
}

fn run_case(initial_dst: &[WcharT], num_elem: usize, src: Option<&[WcharT]>) {
    let c_lib = load_lib(C_LIB);
    let r_lib = load_lib(RUST_LIB);

    let c_fn: Symbol<WcscatFn> = unsafe { c_lib.get(b"wcscat\0").unwrap() };
    let r_fn: Symbol<WcscatFn> = unsafe { r_lib.get(b"wcscat\0").unwrap() };

    // Make sure both buffers have the same allocation size to safely compare contents.
    let cap = initial_dst.len().max(num_elem).max(1);
    let mut c_buf: Vec<WcharT> = initial_dst.to_vec();
    c_buf.resize(cap, 0xDEADBEEFu32 as i32);
    let mut r_buf: Vec<WcharT> = initial_dst.to_vec();
    r_buf.resize(cap, 0xDEADBEEFu32 as i32);

    let (src_ptr_c, src_ptr_r): (*const WcharT, *const WcharT) = match src {
        Some(s) => (s.as_ptr(), s.as_ptr()),
        None => (ptr::null(), ptr::null()),
    };

    let c_ret = unsafe { c_fn(c_buf.as_mut_ptr(), num_elem, src_ptr_c) };
    let r_ret = unsafe { r_fn(r_buf.as_mut_ptr(), num_elem, src_ptr_r) };

    assert_eq!(
        c_ret, r_ret,
        "return value mismatch: c={} rust={}, dst_initial={:?}, num_elem={}, src={:?}",
        c_ret, r_ret, initial_dst, num_elem, src
    );
    assert_eq!(
        c_buf, r_buf,
        "buffer contents mismatch: c={:?} rust={:?}, dst_initial={:?}, num_elem={}, src={:?}",
        c_buf, r_buf, initial_dst, num_elem, src
    );
}

#[test]
fn null_dst() {
    // null dst -> EINVAL (22)
    let c_lib = load_lib(C_LIB);
    let r_lib = load_lib(RUST_LIB);
    let c_fn: Symbol<WcscatFn> = unsafe { c_lib.get(b"wcscat\0").unwrap() };
    let r_fn: Symbol<WcscatFn> = unsafe { r_lib.get(b"wcscat\0").unwrap() };

    let src = to_wide("hi");
    let c_ret = unsafe { c_fn(ptr::null_mut(), 10, src.as_ptr()) };
    let r_ret = unsafe { r_fn(ptr::null_mut(), 10, src.as_ptr()) };
    assert_eq!(c_ret, r_ret);
    assert_eq!(c_ret, 22);
}

#[test]
fn zero_num_elem() {
    let dst = to_wide("hello");
    run_case(&dst, 0, Some(&to_wide("world")));
}

#[test]
fn null_src_with_valid_dst() {
    let dst = to_wide("hello");
    // padded buffer
    let mut padded: Vec<WcharT> = dst.clone();
    padded.resize(20, 0x41);
    run_case(&padded, 20, None);
}

#[test]
fn basic_concat_fits() {
    let mut dst: Vec<WcharT> = to_wide("hello");
    dst.resize(20, 0);
    let src = to_wide(" world");
    run_case(&dst, 20, Some(&src));
}

#[test]
fn concat_exact_fit() {
    // "hello" + " world\0" requires len 12 (5 + 6 + 1)
    let mut dst: Vec<WcharT> = to_wide("hello");
    dst.resize(12, 0);
    let src = to_wide(" world");
    run_case(&dst, 12, Some(&src));
}

#[test]
fn concat_overflow_truncated() {
    // not enough space; should return 34 (ERANGE)
    let mut dst: Vec<WcharT> = to_wide("hello");
    dst.resize(8, 0);
    let src = to_wide(" world!");
    run_case(&dst, 8, Some(&src));
}

#[test]
fn empty_dst_empty_src() {
    let dst = vec![0i32; 10];
    let src = vec![0i32; 1];
    run_case(&dst, 10, Some(&src));
}

#[test]
fn empty_dst_with_src() {
    let mut dst = vec![0i32; 10];
    dst[0] = 0;
    let src = to_wide("abc");
    run_case(&dst, 10, Some(&src));
}

#[test]
fn dst_already_full_no_null_terminator() {
    // dst contains data with no null terminator within numElem
    let dst: Vec<WcharT> = vec![b'a' as i32; 5];
    let src = to_wide("xyz");
    run_case(&dst, 5, Some(&src));
}

#[test]
fn unicode_content() {
    let mut dst = to_wide("αβ");
    dst.resize(20, 0);
    let src = to_wide("γδε");
    run_case(&dst, 20, Some(&src));
}

#[test]
fn num_elem_one_with_empty_dst() {
    // numElem=1; dst[0]==0 (empty string). After scanning, ptr == end.
    // Loop body doesn't run. Sets dst[0]=0, returns 34.
    let dst = vec![0i32; 1];
    let src = to_wide("a");
    run_case(&dst, 1, Some(&src));
}

#[test]
fn num_elem_one_with_nonempty_dst() {
    // dst[0] != 0; while-scan moves ptr to dst+1 which equals end.
    // Inner loop doesn't run -> dst[0]=0, return 34.
    let dst: Vec<WcharT> = vec![b'x' as i32];
    let src = to_wide("abc");
    run_case(&dst, 1, Some(&src));
}

#[test]
fn empty_src_appended() {
    let mut dst = to_wide("hi");
    dst.resize(10, 0);
    let src: Vec<WcharT> = vec![0];
    run_case(&dst, 10, Some(&src));
}
