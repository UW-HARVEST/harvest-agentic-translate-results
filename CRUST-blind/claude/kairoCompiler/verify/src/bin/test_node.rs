use kairoCompiler::node::{
    node_set_vector, node_push, node_create, node_peek, node_peek_or_null, node_pop, Node, NodeBinded,
};
use kairoCompiler::vector::{vector_create, vector_count};
use kairoCompiler::compiler::{NODE_TYPE_NUMBER, NODE_TYPE_IDENTIFIER, NODE_TYPE_STRING};

// Note: The node module uses global static state (NODES, NODE_VECTOR, NODE_VECTOR_ROOT).
// To avoid race conditions between parallel tests, we keep one comprehensive test that
// runs serially, with `--test-threads=1` recommended at runtime, but also use a Mutex
// to ensure ordering within this binary. Since cargo runs tests in parallel by default,
// we instead consolidate node testing into a single test function.

#[test]
fn test_node_full_lifecycle() {
    let nv = vector_create(std::mem::size_of::<usize>());
    let nv_root = vector_create(std::mem::size_of::<usize>());
    node_set_vector(nv, nv_root);

    // Create and verify first node
    let mut a = Node::default();
    a.r#type = NODE_TYPE_NUMBER;
    a.llnum = Some(100);
    let na = node_create(&a);
    assert_eq!(na.r#type, NODE_TYPE_NUMBER);
    assert_eq!(na.llnum, Some(100));

    // Create second node
    let mut b = Node::default();
    b.r#type = NODE_TYPE_IDENTIFIER;
    b.sval = Some("abc".to_string());
    let nb = node_create(&b);
    assert_eq!(nb.r#type, NODE_TYPE_IDENTIFIER);
    assert_eq!(nb.sval, Some("abc".to_string()));

    // node_peek_or_null - returns the last (most recent) node
    let peeked = node_peek_or_null().expect("should have a node");
    assert_eq!(peeked.r#type, NODE_TYPE_IDENTIFIER);
    assert_eq!(peeked.sval, Some("abc".to_string()));

    // node_peek - returns the last node, default if none
    let p2 = node_peek();
    assert_eq!(p2.r#type, NODE_TYPE_IDENTIFIER);

    // node_push - push a third node
    let mut c = Node::default();
    c.r#type = NODE_TYPE_STRING;
    c.sval = Some("hello".to_string());
    node_push(&c);

    let p3 = node_peek();
    assert_eq!(p3.r#type, NODE_TYPE_STRING);
    assert_eq!(p3.sval, Some("hello".to_string()));
}

#[test]
fn test_node_default() {
    let n = Node::default();
    assert_eq!(n.r#type, 0);
    assert_eq!(n.flags, 0);
    assert_eq!(n.cval, None);
    assert_eq!(n.sval, None);
    assert_eq!(n.inum, None);
    assert_eq!(n.lnum, None);
    assert_eq!(n.llnum, None);
}

#[test]
fn test_node_binded_default() {
    let nb = NodeBinded::default();
    assert!(nb.owner.is_none());
    assert!(nb.function.is_none());
}

#[test]
fn test_node_clone() {
    let mut n = Node::default();
    n.r#type = NODE_TYPE_NUMBER;
    n.llnum = Some(42);
    n.sval = Some("test".to_string());
    let cloned = n.clone();
    assert_eq!(cloned.r#type, NODE_TYPE_NUMBER);
    assert_eq!(cloned.llnum, Some(42));
    assert_eq!(cloned.sval, Some("test".to_string()));
}

// Verify node creation returns the expected type info even when count varies.
#[test]
fn test_node_create_preserves_fields() {
    // Use a fresh vector context
    let nv = vector_create(std::mem::size_of::<usize>());
    let nv_root = vector_create(std::mem::size_of::<usize>());
    node_set_vector(nv, nv_root);

    let mut template = Node::default();
    template.r#type = NODE_TYPE_NUMBER;
    template.flags = 7;
    template.llnum = Some(12345);
    let created = node_create(&template);
    assert_eq!(created.r#type, NODE_TYPE_NUMBER);
    assert_eq!(created.flags, 7);
    assert_eq!(created.llnum, Some(12345));
}

// vector_count helper sanity
#[test]
fn test_vector_count_sanity() {
    let v = vector_create(8);
    assert_eq!(vector_count(&v), 0);
}

fn main() {}
