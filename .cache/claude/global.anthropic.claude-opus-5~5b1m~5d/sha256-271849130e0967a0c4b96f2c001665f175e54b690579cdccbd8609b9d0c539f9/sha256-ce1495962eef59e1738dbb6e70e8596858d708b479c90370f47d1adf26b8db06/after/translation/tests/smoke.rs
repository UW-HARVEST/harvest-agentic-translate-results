mod common;
#[test]
fn loads_both_and_agrees_on_one_call() {
    let (c, r) = common::both();
    println!("C   .so = {:?}", c.path);
    println!("Rust.so = {:?}", r.path);
    let cv = unsafe { (c.fallcalc)(1, 2, 3, 4) };
    let rv = unsafe { (r.fallcalc)(1, 2, 3, 4) };
    common::diff_eq("smoke", "fallcalc(1,2,3,4)", cv, rv);
    println!("fallcalc(1,2,3,4) = {cv}");
}
