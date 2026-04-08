use skp::skp::*;

// ============ AST creation and basic operations ============

#[test]
fn test_ast_new() {
    let ast = ast_new();
    assert!(ast.is_some());
    let ast = ast.unwrap();
    assert_eq!(ast.par_cnt, 0);
    assert_eq!(ast.nodes_cnt, 0);
    assert_eq!(ast.fail, 0);
    assert_eq!(ast.err_pos, -1);
    assert_eq!(ast.cur_node, ASTNULL);
}

#[test]
fn test_astfree() {
    let ast = ast_new().unwrap();
    let result = astfree(ast);
    assert!(result.is_none());
}

#[test]
fn test_ast_open_close() {
    let mut ast = ast_new().unwrap();
    let open = ast_open(&mut ast, 0, "test_rule");
    assert!(open >= 0);
    let close = ast_close(&mut ast, 5, open);
    assert!(close >= 0);
    assert_eq!(ast.par_cnt, 2);
    assert_eq!(ast.nodes_cnt, 1);
}

#[test]
fn test_ast_open_close_fail() {
    let mut ast = ast_new().unwrap();
    ast.fail = 1;
    let open = ast_open(&mut ast, 0, "test_rule");
    assert_eq!(open, -1);
}

#[test]
fn test_astnoderule() {
    let mut ast = ast_new().unwrap();
    ast.start = "hello".to_string();
    let open = ast_open(&mut ast, 0, "myrule");
    ast_close(&mut ast, 5, open);
    assert_eq!(astnoderule(&ast, 0), "myrule");
}

#[test]
fn test_astnodefrom_astnodeto() {
    let mut ast = ast_new().unwrap();
    ast.start = "hello world".to_string();
    let open = ast_open(&mut ast, 0, "r");
    ast_close(&mut ast, 5, open);
    assert_eq!(astnodefrom(&ast, 0), "hello world");
    assert_eq!(astnodeto(&ast, 0), " world");
}

#[test]
fn test_astnodelen() {
    let mut ast = ast_new().unwrap();
    ast.start = "hello".to_string();
    let open = ast_open(&mut ast, 0, "r");
    ast_close(&mut ast, 5, open);
    assert_eq!(astnodelen(&ast, 0), 5);
}

#[test]
fn test_astisleaf() {
    let mut ast = ast_new().unwrap();
    ast.start = "hello".to_string();
    let open = ast_open(&mut ast, 0, "r");
    ast_close(&mut ast, 5, open);
    // A node with no children has delta=1, so it's a leaf
    assert!(astisleaf(&ast, 0));
}

#[test]
fn test_astisnodeentry_exit() {
    let mut ast = ast_new().unwrap();
    ast.start = "hello".to_string();
    let open = ast_open(&mut ast, 0, "r");
    ast_close(&mut ast, 5, open);
    assert!(astisnodeentry(&ast, 0));
    assert!(astisnodeexit(&ast, 1));
    assert!(!astisnodeentry(&ast, 1));
    assert!(!astisnodeexit(&ast, 0));
}

#[test]
fn test_astnextdf() {
    let mut ast = ast_new().unwrap();
    ast.start = "hello".to_string();
    let open = ast_open(&mut ast, 0, "r");
    ast_close(&mut ast, 5, open);
    let n = astnextdf(&ast, ASTNULL);
    assert_eq!(n, 0);
    let n = astnextdf(&ast, 0);
    assert_eq!(n, 1);
    let n = astnextdf(&ast, 1);
    assert_eq!(n, ASTNULL);
}

// ============ AST tree navigation ============

#[test]
fn test_ast_navigation() {
    let mut ast = ast_new().unwrap();
    ast.start = "abcdef".to_string();
    // Create: (root (child1)(child2))
    let root = ast_open(&mut ast, 0, "root");
    let c1 = ast_open(&mut ast, 0, "child1");
    ast_close(&mut ast, 3, c1);
    let c2 = ast_open(&mut ast, 3, "child2");
    ast_close(&mut ast, 6, c2);
    ast_close(&mut ast, 6, root);

    // root is at par index 0
    // child1 starts at par index 1
    // child2 starts at par index 3
    let down = astdown(&ast, 0);
    assert_ne!(down, ASTNULL);
    assert_eq!(astnoderule(&ast, down), "child1");

    let right = astright(&ast, down);
    assert_ne!(right, ASTNULL);
    assert_eq!(astnoderule(&ast, right), "child2");

    let left = astleft(&ast, right);
    assert_ne!(left, ASTNULL);
    assert_eq!(astnoderule(&ast, left), "child1");

    let up = astup(&ast, down);
    assert_ne!(up, ASTNULL);
    assert_eq!(astnoderule(&ast, up), "root");
}

#[test]
fn test_astfirst_astlast() {
    let mut ast = ast_new().unwrap();
    ast.start = "abcdef".to_string();
    let root = ast_open(&mut ast, 0, "root");
    let c1 = ast_open(&mut ast, 0, "c1");
    ast_close(&mut ast, 2, c1);
    let c2 = ast_open(&mut ast, 2, "c2");
    ast_close(&mut ast, 4, c2);
    let c3 = ast_open(&mut ast, 4, "c3");
    ast_close(&mut ast, 6, c3);
    ast_close(&mut ast, 6, root);

    let down = astdown(&ast, 0);
    let first = astfirst(&ast, down);
    assert_eq!(astnoderule(&ast, first), "c1");

    let last = astlast(&ast, down);
    assert_eq!(astnoderule(&ast, last), "c3");
}

// ============ AST info ============

#[test]
fn test_astnodeinfo() {
    let mut ast = ast_new().unwrap();
    ast.start = "hello".to_string();
    let open = ast_open(&mut ast, 0, "r");
    ast_close(&mut ast, 5, open);
    ast_setinfo(&mut ast, 42, 0);
    assert_eq!(astnodeinfo(&ast, 0), 42);
}

#[test]
fn test_astnewinfo() {
    let mut ast = ast_new().unwrap();
    ast.start = "hello".to_string();
    astnewinfo(&mut ast, 99);
    assert_eq!(ast.lastinfo, 99);
}

// ============ AST delete ============

#[test]
fn test_ast_delete() {
    let mut ast = ast_new().unwrap();
    ast.start = "hello".to_string();
    let open = ast_open(&mut ast, 0, "r");
    ast_close(&mut ast, 5, open);
    assert_eq!(ast.par_cnt, 2);
    ast_delete(&mut ast);
    assert_eq!(ast.par_cnt, 0);
}

// ============ AST lastnode ============

#[test]
fn test_ast_lastnode() {
    let mut ast = ast_new().unwrap();
    ast.start = "hello".to_string();
    let open = ast_open(&mut ast, 0, "r");
    ast_close(&mut ast, 5, open);
    let last = ast_lastnode(&ast);
    assert_eq!(last, 0);
}

#[test]
fn test_ast_lastnode_empty() {
    let ast = ast_new().unwrap();
    assert_eq!(ast_lastnode(&ast), ASTNULL);
}

// ============ AST lastnodeisempty ============

#[test]
fn test_ast_lastnodeisempty_true() {
    let mut ast = ast_new().unwrap();
    ast.start = "hello".to_string();
    let open = ast_open(&mut ast, 0, "r");
    ast_close(&mut ast, 0, open); // from=0, to=0 => empty
    assert!(ast_lastnodeisempty(&ast));
}

#[test]
fn test_ast_lastnodeisempty_false() {
    let mut ast = ast_new().unwrap();
    ast.start = "hello".to_string();
    let open = ast_open(&mut ast, 0, "r");
    ast_close(&mut ast, 5, open);
    assert!(!ast_lastnodeisempty(&ast));
}

// ============ AST noleaf ============

#[test]
fn test_ast_noleaf() {
    let mut ast = ast_new().unwrap();
    ast.start = "hello".to_string();
    let open = ast_open(&mut ast, 0, "r");
    ast_close(&mut ast, 5, open);
    // This is a leaf (delta=1), so noleaf should remove it
    ast_noleaf(&mut ast);
    assert_eq!(ast.par_cnt, 0);
}

// ============ AST lift ============

#[test]
fn test_ast_lift() {
    let mut ast = ast_new().unwrap();
    ast.start = "hello".to_string();
    // Create ((child)) - outer wraps single child
    let outer = ast_open(&mut ast, 0, "outer");
    let inner = ast_open(&mut ast, 0, "inner");
    ast_close(&mut ast, 5, inner);
    ast_close(&mut ast, 5, outer);
    // outer has tag=0 by default, so lift should remove it
    ast_lift(&mut ast);
    assert_eq!(ast.par_cnt, 2); // only inner remains
    assert_eq!(astnoderule(&ast, 0), "inner");
}

// ============ AST swap ============

#[test]
fn test_ast_swap() {
    let mut ast = ast_new().unwrap();
    ast.start = "abcdef".to_string();
    // Create two sibling nodes at top level (no wrapping root)
    let c1 = ast_open(&mut ast, 0, "first");
    ast_close(&mut ast, 3, c1);
    let c2 = ast_open(&mut ast, 3, "second");
    ast_close(&mut ast, 6, c2);
    // Before swap: first, second
    ast_swap(&mut ast);
    // After swap: second, first
    assert_eq!(astnoderule(&ast, 0), "second");
}

// ============ AST error functions ============

#[test]
fn test_asthaserr_no_error() {
    let ast = ast_new().unwrap();
    assert!(!asthaserr(&ast));
}

#[test]
fn test_asthaserr_with_error() {
    let mut ast = ast_new().unwrap();
    ast.err_pos = 5;
    assert!(asthaserr(&ast));
}

#[test]
fn test_asterrrule_no_error() {
    let ast = ast_new().unwrap();
    assert_eq!(asterrrule(&ast), Some(""));
}

#[test]
fn test_asterrcolnum_no_error() {
    let ast = ast_new().unwrap();
    assert_eq!(asterrcolnum(&ast), 0);
}

// ============ AST is ============

#[test]
fn test_ast_is() {
    let mut ast = ast_new().unwrap();
    ast.start = "hello".to_string();
    let open = ast_open(&mut ast, 0, "myrule");
    ast_close(&mut ast, 5, open);
    assert_eq!(ast_is(&ast, 0, "myrule"), 1);
    assert_eq!(ast_is(&ast, 0, "other"), 0);
}

// ============ AST debug ============

#[test]
fn test_skp_debug2() {
    let mut ast = ast_new().unwrap();
    skp_debug2(&mut ast, 1);
    assert_ne!(ast.flg & 0x01, 0);
    skp_debug2(&mut ast, 0);
    assert_eq!(ast.flg & 0x01, 0);
}

// ============ AST print ============

#[test]
fn test_astprintsexpr() {
    let mut ast = ast_new().unwrap();
    ast.start = "hello".to_string();
    let open = ast_open(&mut ast, 0, "r");
    ast_close(&mut ast, 5, open);
    let mut buf = Vec::new();
    astprintsexpr(&ast, &mut buf);
    let output = String::from_utf8(buf).unwrap();
    assert!(output.contains("(r 'hello')"));
}

#[test]
fn test_astprinttree() {
    let mut ast = ast_new().unwrap();
    ast.start = "hello".to_string();
    let open = ast_open(&mut ast, 0, "r");
    ast_close(&mut ast, 5, open);
    let mut buf = Vec::new();
    astprinttree(&ast, &mut buf);
    let output = String::from_utf8(buf).unwrap();
    assert!(output.contains("[r]"));
    assert!(output.contains("'hello'"));
}

// ============ skp_parse ============

#[test]
fn test_skp_parse_basic() {
    fn dummy_rule(ast: &mut Ast, ret: &mut i32) {
        *ret = 42;
    }
    let ast = skp_parse("hello", dummy_rule, "start", 0);
    assert!(ast.is_some());
    let ast = ast.unwrap();
    assert_eq!(ast.ret, 42);
}

// ============ skp__abort ============

#[test]
fn test_skp_abort() {
    let mut ast = ast_new().unwrap();
    ast.start = "hello".to_string();
    ast.pos = 3;
    skp__abort(&mut ast, "test error", "test_rule");
    assert_eq!(ast.fail, 1);
    assert_eq!(ast.err_pos, 3);
    assert_eq!(ast.err_rule.as_deref(), Some("test_rule"));
    assert_eq!(ast.err_msg.as_deref(), Some("test error"));
}

// ============ Boundary: ASTNULL navigation ============

#[test]
fn test_navigation_astnull() {
    let ast = ast_new().unwrap();
    assert_eq!(astdown(&ast, ASTNULL), ASTNULL);
    assert_eq!(astup(&ast, ASTNULL), ASTNULL);
    assert_eq!(astleft(&ast, ASTNULL), ASTNULL);
    assert_eq!(astright(&ast, ASTNULL), ASTNULL);
    assert_eq!(astfirst(&ast, ASTNULL), ASTNULL);
    assert_eq!(astlast(&ast, ASTNULL), ASTNULL);
}

#[test]
fn test_astnoderule_invalid() {
    let ast = ast_new().unwrap();
    assert_eq!(astnoderule(&ast, ASTNULL), "");
    assert_eq!(astnoderule(&ast, 999), "");
}

#[test]
fn test_astnodelen_invalid() {
    let ast = ast_new().unwrap();
    assert_eq!(astnodelen(&ast, ASTNULL), 0);
}

fn main() {}
