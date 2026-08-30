mod common;
use common::*;

#[test]
fn smoke_both_libraries_load_and_agree() {
    let mut h = lock();
    let out = h.run(1, "smoke");
    let s = String::from_utf8(out).unwrap();
    println!("--- first captured output ---\n{s}---");
    assert!(s.contains("The house has"));
    h.driver(2, "smoke");
}
