use kairoCompiler::compiler::{
    CompileProcess, PARSE_ALL_OK,
    NODE_TYPE_NUMBER, NODE_TYPE_IDENTIFIER, NODE_TYPE_STRING,
};
use kairoCompiler::lexer::tokens_build_for_string;
use kairoCompiler::parser::parse;
use kairoCompiler::node::{node_pop, node_set_vector, NODES};
use kairoCompiler::vector::vector_create;

// Parser tests must run serially because they use global state (NODES, TOKENS, etc.)
#[test]
fn test_parse_operations() {
    // Test 1: parse number
    {
        node_set_vector(vector_create(8), vector_create(8));
        NODES.lock().unwrap().clear();

        let cp = CompileProcess::default();
        let lp = tokens_build_for_string(cp, "42$").unwrap();

        let mut process = CompileProcess::default();
        process.token_vec = lp.token_vec;
        process.node_vec = Some(vector_create(8));
        process.node_tree_vec = Some(vector_create(8));

        let result = parse(&mut process);
        assert_eq!(result, PARSE_ALL_OK);

        let n = node_pop();
        assert_eq!(n.r#type, NODE_TYPE_NUMBER);
        assert_eq!(n.llnum, Some(42));
    }

    // Test 2: parse identifier
    {
        node_set_vector(vector_create(8), vector_create(8));
        NODES.lock().unwrap().clear();

        let cp = CompileProcess::default();
        let lp = tokens_build_for_string(cp, "hello$").unwrap();

        let mut process = CompileProcess::default();
        process.token_vec = lp.token_vec;
        process.node_vec = Some(vector_create(8));
        process.node_tree_vec = Some(vector_create(8));

        let result = parse(&mut process);
        assert_eq!(result, PARSE_ALL_OK);

        let n = node_pop();
        assert_eq!(n.r#type, NODE_TYPE_IDENTIFIER);
        assert_eq!(n.sval, Some("hello".to_string()));
    }

    // Test 3: parse string
    {
        node_set_vector(vector_create(8), vector_create(8));
        NODES.lock().unwrap().clear();

        let cp = CompileProcess::default();
        let lp = tokens_build_for_string(cp, "\"test\"$").unwrap();

        let mut process = CompileProcess::default();
        process.token_vec = lp.token_vec;
        process.node_vec = Some(vector_create(8));
        process.node_tree_vec = Some(vector_create(8));

        let result = parse(&mut process);
        assert_eq!(result, PARSE_ALL_OK);

        let n = node_pop();
        assert_eq!(n.r#type, NODE_TYPE_STRING);
        assert_eq!(n.sval, Some("test".to_string()));
    }
}

fn main() {}
