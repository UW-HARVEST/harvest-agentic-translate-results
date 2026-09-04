mod common;
use common::*;

#[test]
fn smoke_both_libs_load_and_agree() {
    let p = pair();

    // fma_array, distinct buffers.
    let mut out_c = vec![0i32; 4];
    let mut out_r = vec![0i32; 4];
    let m1 = [1i32, 2, 3, 4];
    let m2 = [10i32, 20, 30, 40];
    let ad = [100i32, 200, 300, 400];
    unsafe {
        (p.c.fma_array)(out_c.as_mut_ptr(), m1.as_ptr(), m2.as_ptr(), ad.as_ptr(), 4);
        (p.rs.fma_array)(out_r.as_mut_ptr(), m1.as_ptr(), m2.as_ptr(), ad.as_ptr(), 4);
    }
    assert_eq!(out_c, vec![110, 240, 390, 560], "C reference value check");
    assert_eq!(out_c, out_r);

    // driver: stdout capture must work and be identical.
    let data = [1i32, 2, 3];
    let bytes = diff_driver(p, &data, 3, "smoke driver");
    assert_eq!(
        String::from_utf8(bytes).unwrap(),
        "2\n6\n12\n",
        "C reference stdout check (x*x+x for 1,2,3)"
    );
}
