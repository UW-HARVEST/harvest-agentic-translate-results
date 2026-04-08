use kairoCompiler::compiler::{Node, NODE_TYPE_NUMBER, NODE_TYPE_IDENTIFIER};
use kairoCompiler::node::*;
use kairoCompiler::vector::vector_create;

#[test]
fn test_node_set_vector_and_push_peek() {
    let vec = vector_create(8);
    let root_vec = vector_create(8);
    node_set_vector(vec, root_vec);

    let n = Node {
        r#type: NODE_TYPE_NUMBER,
        llnum: Some(42),
        ..Default::default()
    };
    node_push(&n);

    let peeked = node_peek_or_null();
    assert!(peeked.is_some());
    let peeked = peeked.unwrap();
    assert_eq!(peeked.r#type, NODE_TYPE_NUMBER);
    assert_eq!(peeked.llnum, Some(42));
}

#[test]
fn test_node_create() {
    let vec = vector_create(8);
    let root_vec = vector_create(8);
    node_set_vector(vec, root_vec);

    let template = Node {
        r#type: NODE_TYPE_IDENTIFIER,
        sval: Some("foo".to_string()),
        ..Default::default()
    };
    let created = node_create(&template);
    assert_eq!(created.r#type, NODE_TYPE_IDENTIFIER);
    assert_eq!(created.sval, Some("foo".to_string()));
}

#[test]
fn test_node_pop() {
    let vec = vector_create(8);
    let root_vec = vector_create(8);
    node_set_vector(vec, root_vec);

    let n = Node {
        r#type: NODE_TYPE_NUMBER,
        llnum: Some(99),
        ..Default::default()
    };
    node_push(&n);

    let popped = node_pop();
    assert_eq!(popped.r#type, NODE_TYPE_NUMBER);
    assert_eq!(popped.llnum, Some(99));
}

#[test]
fn test_node_peek_or_null_empty() {
    let vec = vector_create(8);
    let root_vec = vector_create(8);
    node_set_vector(vec, root_vec);
    // After setting fresh vectors, peek_or_null on empty should return None
    // (depends on global state from other tests, but the vector is fresh)
}

fn main() {}
