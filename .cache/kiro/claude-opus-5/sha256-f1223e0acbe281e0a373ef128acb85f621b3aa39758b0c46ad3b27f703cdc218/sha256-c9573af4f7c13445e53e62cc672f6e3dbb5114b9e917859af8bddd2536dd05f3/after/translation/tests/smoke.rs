mod common;
use common::*;

#[test]
fn smoke_both_libs_load_and_agree() {
    let p = Pair::fresh();
    assert_eq!(std::mem::size_of::<Node>(), 80, "Node layout assumption");
    // Fresh store: nothing there.
    assert!(p.find_node_by_id(1).is_none());
    assert_eq!(p.get_children_count(1), 0);
    // Drive the one-shot wrapper.
    let v = p.maxnmin(1, 2, 3, 4);
    eprintln!("maxnmin(1,2,3,4) = {v}");
    assert!(p.find_node_by_id(1).is_some());
}
