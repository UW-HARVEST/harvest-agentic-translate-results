use libloading::{Library, Symbol};
use std::ptr;

type WcscatFn = unsafe extern "C" fn(*mut i32, usize, *const i32) -> i32;

fn load_libs() -> (Library, Library) {
    let manifest = env!("CARGO_MANIFEST_DIR");
    let c_path = format!("{}/c_src/libtranslated_rust.so", manifest);
    let rust_path = format!("{}/target/debug/libwcscat_lib.so", manifest);
    unsafe {
        (
            Library::new(&c_path).expect("load C .so"),
            Library::new(&rust_path).expect("load Rust .so"),
        )
    }
}

fn get_fns(c: &Library, r: &Library) -> (WcscatFn, WcscatFn) {
    unsafe {
        let cf: Symbol<WcscatFn> = c.get(b"wcscat").expect("C wcscat");
        let rf: Symbol<WcscatFn> = r.get(b"wcscat").expect("Rust wcscat");
        (*cf, *rf)
    }
}

fn call_with_bufs(
    f: WcscatFn,
    dst_init: &[i32],
    num_elem: usize,
    src: &[i32],
) -> (i32, Vec<i32>) {
    let mut dst = dst_init.to_vec();
    // Ensure dst has at least num_elem elements for safety
    dst.resize(dst.len().max(num_elem), 0);
    let ret = unsafe { f(dst.as_mut_ptr(), num_elem, src.as_ptr()) };
    (ret, dst)
}

fn assert_match(label: &str, dst_init: &[i32], num_elem: usize, src: &[i32]) {
    let (c_lib, r_lib) = load_libs();
    let (cf, rf) = get_fns(&c_lib, &r_lib);

    let (c_ret, c_dst) = call_with_bufs(cf, dst_init, num_elem, src);
    let (r_ret, r_dst) = call_with_bufs(rf, dst_init, num_elem, src);

    assert_eq!(c_ret, r_ret, "{label}: return mismatch");
    assert_eq!(c_dst, r_dst, "{label}: dst mismatch");
}

fn assert_match_null_src(label: &str, dst_init: &[i32], num_elem: usize) {
    let (c_lib, r_lib) = load_libs();
    let (cf, rf) = get_fns(&c_lib, &r_lib);

    let mut c_dst = dst_init.to_vec();
    c_dst.resize(c_dst.len().max(num_elem), 0);
    let mut r_dst = c_dst.clone();

    let c_ret = unsafe { cf(c_dst.as_mut_ptr(), num_elem, ptr::null()) };
    let r_ret = unsafe { rf(r_dst.as_mut_ptr(), num_elem, ptr::null()) };

    assert_eq!(c_ret, r_ret, "{label}: return mismatch");
    assert_eq!(c_dst, r_dst, "{label}: dst mismatch");
}

fn assert_match_null_dst(label: &str, num_elem: usize, src: &[i32]) {
    let (c_lib, r_lib) = load_libs();
    let (cf, rf) = get_fns(&c_lib, &r_lib);

    let c_ret = unsafe { cf(ptr::null_mut(), num_elem, src.as_ptr()) };
    let r_ret = unsafe { rf(ptr::null_mut(), num_elem, src.as_ptr()) };

    assert_eq!(c_ret, r_ret, "{label}: return mismatch");
}

#[test]
fn null_dst() {
    assert_match_null_dst("null_dst", 10, &[65, 0]);
}

#[test]
fn zero_num_elem() {
    let (c_lib, r_lib) = load_libs();
    let (cf, rf) = get_fns(&c_lib, &r_lib);
    let c_ret = unsafe { cf(ptr::null_mut(), 0, ptr::null()) };
    let r_ret = unsafe { rf(ptr::null_mut(), 0, ptr::null()) };
    assert_eq!(c_ret, r_ret, "zero_num_elem: return mismatch");
}

#[test]
fn null_src() {
    assert_match_null_src("null_src", &[72, 105, 0, 0, 0], 5);
}

#[test]
fn null_src_clears_dst() {
    assert_match_null_src("null_src_clears", &[65, 66, 67, 0, 0], 5);
}

#[test]
fn simple_concat() {
    // dst="Hi\0\0\0\0", src="!!\0"  → "Hi!!\0"
    assert_match("simple", &[72, 105, 0, 0, 0, 0], 6, &[33, 33, 0]);
}

#[test]
fn empty_dst() {
    // dst="\0\0\0\0", src="AB\0"
    assert_match("empty_dst", &[0, 0, 0, 0], 4, &[65, 66, 0]);
}

#[test]
fn empty_src() {
    // dst="AB\0\0", src="\0"
    assert_match("empty_src", &[65, 66, 0, 0], 4, &[0]);
}

#[test]
fn exact_fit() {
    // dst="AB\0", numElem=4, src="C\0" → "ABC\0" exactly fills buffer
    assert_match("exact_fit", &[65, 66, 0, 0], 4, &[67, 0]);
}

#[test]
fn overflow_by_one() {
    // dst="AB\0", numElem=4, src="CD\0" → overflow, dst[0]=0, ret=34
    assert_match("overflow_one", &[65, 66, 0, 0], 4, &[67, 68, 0]);
}

#[test]
fn overflow_large_src() {
    assert_match("overflow_large", &[65, 0, 0], 3, &[66, 67, 68, 69, 0]);
}

#[test]
fn dst_full_no_null() {
    // dst completely full (no null terminator found within numElem)
    // ptr will reach end without finding \0, then copy loop runs 0 times
    // since ptr == end already → overflow path
    assert_match("dst_full", &[65, 66, 67], 3, &[68, 0]);
}

#[test]
fn num_elem_one_empty_dst() {
    // dst="\0", numElem=1, src="A\0" → overflow (only room for existing \0, no room for src + \0)
    assert_match("nelem1_empty", &[0], 1, &[65, 0]);
}

#[test]
fn num_elem_one_with_char() {
    // dst="A", numElem=1, src="B\0" → ptr reaches end, overflow
    assert_match("nelem1_char", &[65], 1, &[66, 0]);
}

#[test]
fn unicode_values() {
    // Test with larger wchar_t values
    assert_match(
        "unicode",
        &[0x1F600, 0x1F601, 0, 0, 0, 0],
        6,
        &[0x1F602, 0x1F603, 0],
    );
}
