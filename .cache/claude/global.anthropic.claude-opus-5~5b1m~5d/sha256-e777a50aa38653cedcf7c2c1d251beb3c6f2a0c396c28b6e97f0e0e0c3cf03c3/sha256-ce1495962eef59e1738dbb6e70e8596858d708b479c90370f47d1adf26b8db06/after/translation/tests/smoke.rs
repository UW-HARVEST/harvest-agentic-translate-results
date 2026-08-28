// Harness smoke test: both `.so`s load, all 7 symbols resolve, statics are
// pristine per `Pair::fresh()`, and the `Node` layout agrees.

mod common;
use common::*;

#[test]
fn harness_loads_both_libraries() {
    let p = Pair::fresh();
    // pristine state: nothing stored yet in either library
    let fc = unsafe { (p.c.find_node_by_id)(1) };
    let fr = unsafe { (p.rust.find_node_by_id)(1) };
    assert!(fc.is_null(), "C library not pristine");
    assert!(fr.is_null(), "Rust library not pristine");
    both_query(&p, "smoke", 1);
    let i = both_add(&p, "smoke", 1, -1, b"root", 10.5);
    assert_eq!(i, 0);
    both_query(&p, "smoke", 1);
    both_maxnmin(&p, "smoke", 1, 2, 3, 4);
}

#[test]
fn fresh_pairs_are_independent() {
    let a = Pair::fresh();
    both_add(&a, "smoke", 7, -1, b"a", 1.0);
    let b = Pair::fresh();
    // `b` must not see `a`'s node
    assert!(unsafe { (b.c.find_node_by_id)(7) }.is_null());
    assert!(unsafe { (b.rust.find_node_by_id)(7) }.is_null());
    both_query(&b, "smoke", 7);
}

#[test]
fn node_layout_matches_c() {
    assert_eq!(std::mem::size_of::<Node>(), 80);
    assert_eq!(std::mem::align_of::<Node>(), 8);
    let n = Node { id: 0, parent_id: 0, name: [0; MAX_NAME_LEN], value: 0.0, active: 0 };
    let b = &n as *const Node as usize;
    assert_eq!(&n.id as *const _ as usize - b, 0);
    assert_eq!(&n.parent_id as *const _ as usize - b, 4);
    assert_eq!(&n.name as *const _ as usize - b, 8);
    assert_eq!(&n.value as *const _ as usize - b, 64);
    assert_eq!(&n.active as *const _ as usize - b, 72);
}
