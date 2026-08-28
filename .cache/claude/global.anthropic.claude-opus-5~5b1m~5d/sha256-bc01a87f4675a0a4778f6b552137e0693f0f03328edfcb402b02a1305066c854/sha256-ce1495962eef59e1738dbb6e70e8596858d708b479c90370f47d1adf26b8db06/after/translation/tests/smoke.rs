mod common;
use common::*;

#[test]
fn harness_loads_both_libraries() {
    let (c, r) = libs();
    println!("C   .so: {}", c.path.display());
    println!("Rust.so: {}", r.path.display());
}

#[test]
fn harness_captures_stdout_and_kernels_agree() {
    let (c, r) = libs();

    let (cv, co) = capture(|| unsafe { (c.checkshift)(1, 2, 3, 4) });
    let (rv, ro) = capture(|| unsafe { (r.checkshift)(1, 2, 3, 4) });

    assert!(!co.is_empty(), "expected C to print something");
    println!("--- C transcript ---\n{}", String::from_utf8_lossy(&co));
    assert_eq!(cv, rv, "checkshift(1,2,3,4) return value");
    assert_stdout_eq("checkshift(1,2,3,4)", &co, &ro);

    for k in 0..4usize {
        let (a, b) = (7i32, -19i32);
        let cvv = unsafe { (c.kernel(k))(a, b) };
        let rvv = unsafe { (r.kernel(k))(a, b) };
        assert_eq!(cvv, rvv, "kernel {k}");
    }
}
