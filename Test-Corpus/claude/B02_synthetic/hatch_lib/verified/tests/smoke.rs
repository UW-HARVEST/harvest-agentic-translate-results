//! Harness self-check: both `.so`s load and the 12 symbols resolve.

mod common;
use common::*;

#[test]
fn both_libraries_load_and_resolve_all_symbols() {
    println!("C    .so: {}", c_so_path().display());
    println!("Rust .so: {}", rust_so_path().display());
    let h = harness();
    assert_eq!(h.c.tag, "C");
    assert_eq!(h.r.tag, "Rust");
    // A trivial pure call through each `.so`.
    assert_eq!(h.add_three(1, 2, 3), 6);
    assert_eq!(h.multiply_add(3, 4, 5), 17);
    assert_eq!(h.compute_with_dynamic_memory(0, 8), 84);
    assert_eq!(h.get_time_based_value(1), 37);
    assert_eq!(std::mem::size_of::<DataRecord>(), DATARECORD_SIZE);
    assert_eq!(std::mem::align_of::<DataRecord>(), 8);
}
