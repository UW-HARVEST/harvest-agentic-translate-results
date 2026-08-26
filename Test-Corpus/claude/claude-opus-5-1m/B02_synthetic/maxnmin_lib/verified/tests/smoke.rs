//! Harness smoke test: both `.so`s load, all 7 exported symbols resolve, and the
//! `Node` struct layout observed through `find_node_by_id` matches the C ABI.

mod common;
use common::*;

#[test]
fn both_libraries_load_and_export_all_symbols() {
    let p = Pair::new("smoke");
    // Symbol resolution already happened in Lib::open; do one call each so the
    // linkage of every export is actually exercised.
    assert_eq!(p.safe_double_to_int(1.5), 1);
    assert_eq!(p.process_string(b"a\0"), 97);
    assert_eq!(p.add_node(1, -1, "root", 1.0), 0);
    assert!(p.find_node_by_id(1).is_some());
    assert_eq!(p.get_children_count(-1), 1);
    assert_eq!(p.calculate_subtree_sum(1), 1.0);
    let _ = p.maxnmin(1, 2, 3, 4);
}

#[test]
fn node_struct_layout_matches_c_abi() {
    let p = Pair::new("layout");
    // Two nodes in adjacent storage slots: the pointer delta must equal
    // sizeof(Node) == 80 in *both* libraries.
    p.add_node(11, -1, "a", 1.0);
    p.add_node(22, 11, "b", 2.0);
    let (cs, rs) = p.observed_stride(11, 22);
    assert_eq!(cs, SIZEOF_NODE as isize, "C sizeof(Node) != 80 (got {cs})");
    assert_eq!(rs, cs, "Rust struct stride {rs} != C struct stride {cs}");

    // Distinct sentinel values in every field, read back at hard-coded C
    // offsets: any field reordering or padding difference shows up here.
    let v = p.node_view(22).expect("node 22");
    assert_eq!(v.id, 22);
    assert_eq!(v.parent_id, 11);
    assert_eq!(&v.name[..2], b"b\0");
    assert_eq!(f64::from_bits(v.value_bits), 2.0);
    assert_eq!(v.active, 1);
}
