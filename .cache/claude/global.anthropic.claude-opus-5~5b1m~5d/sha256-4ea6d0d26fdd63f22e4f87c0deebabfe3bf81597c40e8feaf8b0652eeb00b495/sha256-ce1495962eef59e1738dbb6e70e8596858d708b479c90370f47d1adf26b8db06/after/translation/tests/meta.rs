mod common;
#[test]
fn harness_loads_two_rust_variants_and_the_c_lib() {
    let n = common::rust_libs().len();
    eprintln!("rust variants: {:?}", common::rust_libs().iter().map(|l| l.name.clone()).collect::<Vec<_>>());
    eprintln!("c lib: {}", common::c_lib().path.display());
    assert_eq!(n, 2, "expected both debug and release Rust cdylibs to be tested");
}
