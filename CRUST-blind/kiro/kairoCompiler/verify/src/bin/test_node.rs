use kairoCompiler::node::*;
use kairoCompiler::vector::vector_create;

// Node tests must run serially because they use global state
// We use a single test function to avoid parallel execution issues

#[test]
fn test_node_operations() {
    // Test 1: create and peek
    {
        node_set_vector(vector_create(8), vector_create(8));
        NODES.lock().unwrap().clear();

        let template = Node {
            r#type: 2, // NODE_TYPE_NUMBER
            llnum: Some(42),
            ..Node::default()
        };
        let created = node_create(&template);
        assert_eq!(created.r#type, 2);
        assert_eq!(created.llnum, Some(42));

        let peeked = node_peek();
        assert_eq!(peeked.r#type, 2);
        assert_eq!(peeked.llnum, Some(42));
    }

    // Test 2: push and pop
    {
        node_set_vector(vector_create(8), vector_create(8));
        NODES.lock().unwrap().clear();

        let n = Node {
            r#type: 3,
            sval: Some("hello".to_string()),
            ..Node::default()
        };
        node_push(&n);
        let popped = node_pop();
        assert_eq!(popped.r#type, 3);
        assert_eq!(popped.sval, Some("hello".to_string()));
    }

    // Test 3: peek_or_null on empty
    {
        node_set_vector(vector_create(8), vector_create(8));
        NODES.lock().unwrap().clear();

        assert!(node_peek_or_null().is_none());
    }

    // Test 4: peek_or_null with node
    {
        node_set_vector(vector_create(8), vector_create(8));
        NODES.lock().unwrap().clear();

        let n = Node {
            r#type: 4,
            sval: Some("test".to_string()),
            ..Node::default()
        };
        node_push(&n);
        let result = node_peek_or_null();
        assert!(result.is_some());
        assert_eq!(result.unwrap().r#type, 4);
    }

    // Test 5: create pushes to stack, pop returns LIFO
    {
        node_set_vector(vector_create(8), vector_create(8));
        NODES.lock().unwrap().clear();

        let t1 = Node { r#type: 2, llnum: Some(1), ..Node::default() };
        let t2 = Node { r#type: 2, llnum: Some(2), ..Node::default() };
        node_create(&t1);
        node_create(&t2);
        let popped = node_pop();
        assert_eq!(popped.llnum, Some(2));
        let popped = node_pop();
        assert_eq!(popped.llnum, Some(1));
    }
}

fn main() {}
