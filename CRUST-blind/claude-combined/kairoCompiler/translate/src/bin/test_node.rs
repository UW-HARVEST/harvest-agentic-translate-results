use kairoCompiler::compiler::{NODE_TYPE_NUMBER, NODE_TYPE_IDENTIFIER};
use kairoCompiler::node::{
    node_create, node_peek, node_peek_or_null, node_pop, node_push, node_set_vector, Node,
};
use kairoCompiler::vector::vector_create;

fn fresh_setup() {
    let nv = vector_create(8);
    let nvr = vector_create(8);
    node_set_vector(nv, nvr);
}

#[test]
fn test_node_peek_or_null_empty() {
    fresh_setup();
    let p = node_peek_or_null();
    assert!(p.is_none());
}

#[test]
fn test_node_create_and_peek() {
    fresh_setup();
    let mut t = Node::default();
    t.r#type = NODE_TYPE_NUMBER;
    t.llnum = Some(42);
    let created = node_create(&t);
    assert_eq!(created.r#type, NODE_TYPE_NUMBER);
    assert_eq!(created.llnum, Some(42));

    let p = node_peek();
    assert_eq!(p.r#type, NODE_TYPE_NUMBER);
    assert_eq!(p.llnum, Some(42));
}

#[test]
fn test_node_peek_or_null_after_create() {
    fresh_setup();
    let mut t = Node::default();
    t.r#type = NODE_TYPE_NUMBER;
    t.llnum = Some(42);
    node_create(&t);
    let p = node_peek_or_null().unwrap();
    assert_eq!(p.r#type, NODE_TYPE_NUMBER);
    assert_eq!(p.llnum, Some(42));
}

#[test]
fn test_node_push_and_peek() {
    fresh_setup();
    let mut t = Node::default();
    t.r#type = NODE_TYPE_IDENTIFIER;
    t.sval = Some("foo".to_string());
    node_push(&t);
    let p = node_peek();
    assert_eq!(p.r#type, NODE_TYPE_IDENTIFIER);
    assert_eq!(p.sval, Some("foo".to_string()));
}

#[test]
fn test_node_pop_after_two_pushes() {
    fresh_setup();
    let mut t1 = Node::default();
    t1.r#type = NODE_TYPE_NUMBER;
    t1.llnum = Some(1);
    let mut t2 = Node::default();
    t2.r#type = NODE_TYPE_IDENTIFIER;
    t2.sval = Some("bar".to_string());

    node_create(&t1);
    node_create(&t2);

    let popped = node_pop();
    assert_eq!(popped.r#type, NODE_TYPE_IDENTIFIER);
    assert_eq!(popped.sval, Some("bar".to_string()));

    let p = node_peek();
    assert_eq!(p.r#type, NODE_TYPE_NUMBER);
    assert_eq!(p.llnum, Some(1));
}

fn main() {}
