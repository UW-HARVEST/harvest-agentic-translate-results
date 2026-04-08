use skp::skp;

// ============================================================
// AST creation and navigation
// ============================================================

#[test]
fn test_ast_new() {
    let ast = skp::ast_new();
    assert!(ast.is_some());
    let ast = ast.unwrap();
    assert_eq!(ast.par_cnt, 0);
    assert_eq!(ast.nodes_cnt, 0);
    assert_eq!(ast.fail, 0);
    assert_eq!(ast.err_pos, -1);
    assert_eq!(ast.cur_node, -1);
}

#[test]
fn test_ast_open_close() {
    let mut ast = skp::ast_new().unwrap();
    let o1 = skp::ast_open(&mut ast, 0, "root");
    assert_eq!(o1, 0);
    let o2 = skp::ast_open(&mut ast, 0, "child1");
    skp::ast_close(&mut ast, 3, o2);
    let o3 = skp::ast_open(&mut ast, 3, "child2");
    skp::ast_close(&mut ast, 5, o3);
    skp::ast_close(&mut ast, 5, o1);
    assert_eq!(ast.par_cnt, 6);
    assert_eq!(ast.nodes_cnt, 3);
}

#[test]
fn test_ast_navigation() {
    let mut ast = skp::ast_new().unwrap();
    let o1 = skp::ast_open(&mut ast, 0, "root");
    let o2 = skp::ast_open(&mut ast, 0, "child1");
    skp::ast_close(&mut ast, 3, o2);
    let o3 = skp::ast_open(&mut ast, 3, "child2");
    skp::ast_close(&mut ast, 5, o3);
    skp::ast_close(&mut ast, 5, o1);

    // down from root -> first child
    assert_eq!(skp::astdown(&ast, 0), 1);
    // right from child1 -> child2
    assert_eq!(skp::astright(&ast, 1), 3);
    // left from child2 -> child1
    assert_eq!(skp::astleft(&ast, 3), 1);
    // up from child1 -> root
    assert_eq!(skp::astup(&ast, 1), 0);
    // first sibling of child2 -> child1
    assert_eq!(skp::astfirst(&ast, 3), 1);
    // last sibling of child1 -> child2
    assert_eq!(skp::astlast(&ast, 1), 3);
}

#[test]
fn test_ast_node_info() {
    let mut ast = skp::ast_new().unwrap();
    ast.start = "hello".to_string();
    let o1 = skp::ast_open(&mut ast, 0, "root");
    let o2 = skp::ast_open(&mut ast, 0, "child1");
    skp::ast_close(&mut ast, 3, o2);
    let o3 = skp::ast_open(&mut ast, 3, "child2");
    skp::ast_close(&mut ast, 5, o3);
    skp::ast_close(&mut ast, 5, o1);

    assert_eq!(skp::astnoderule(&ast, 0), "root");
    assert_eq!(skp::astnoderule(&ast, 1), "child1");
    assert_eq!(skp::astnoderule(&ast, 3), "child2");
    assert_eq!(skp::astnodelen(&ast, 0), 5);
    assert_eq!(skp::astnodelen(&ast, 1), 3);
    assert_eq!(skp::astnodelen(&ast, 3), 2);
    assert!(skp::astisleaf(&ast, 1));
}

#[test]
fn test_ast_entry_exit() {
    let mut ast = skp::ast_new().unwrap();
    let o1 = skp::ast_open(&mut ast, 0, "root");
    let o2 = skp::ast_open(&mut ast, 0, "child1");
    skp::ast_close(&mut ast, 3, o2);
    skp::ast_close(&mut ast, 5, o1);

    assert!(skp::astisnodeentry(&ast, 0));
    assert!(!skp::astisnodeexit(&ast, 0));
    // par[2] should be close par of child1 (negative)
    assert!(skp::astisnodeexit(&ast, 2));
    assert!(!skp::astisnodeentry(&ast, 2));
}

#[test]
fn test_astnextdf() {
    let mut ast = skp::ast_new().unwrap();
    let o1 = skp::ast_open(&mut ast, 0, "root");
    let o2 = skp::ast_open(&mut ast, 0, "child1");
    skp::ast_close(&mut ast, 3, o2);
    let o3 = skp::ast_open(&mut ast, 3, "child2");
    skp::ast_close(&mut ast, 5, o3);
    skp::ast_close(&mut ast, 5, o1);

    assert_eq!(skp::astnextdf(&ast, -1), 0);
    assert_eq!(skp::astnextdf(&ast, 0), 1);
    assert_eq!(skp::astnextdf(&ast, 5), -1); // ASTNULL
}

#[test]
fn test_ast_is() {
    let mut ast = skp::ast_new().unwrap();
    let o1 = skp::ast_open(&mut ast, 0, "root");
    let o2 = skp::ast_open(&mut ast, 0, "child1");
    skp::ast_close(&mut ast, 3, o2);
    skp::ast_close(&mut ast, 5, o1);

    assert_eq!(skp::ast_is(&ast, 0, "root"), 1);
    assert_eq!(skp::ast_is(&ast, 1, "child1"), 1);
    assert_eq!(skp::ast_is(&ast, 1, "root"), 0);
}

#[test]
fn test_ast_lastnode() {
    let mut ast = skp::ast_new().unwrap();
    let o1 = skp::ast_open(&mut ast, 0, "root");
    let o2 = skp::ast_open(&mut ast, 0, "child1");
    skp::ast_close(&mut ast, 3, o2);
    let o3 = skp::ast_open(&mut ast, 3, "child2");
    skp::ast_close(&mut ast, 5, o3);
    skp::ast_close(&mut ast, 5, o1);

    // Last node: par_cnt-1=5, par[5] is close of root (delta=-5), o1=5+(-5)=0
    assert_eq!(skp::ast_lastnode(&ast), 0);
}

#[test]
fn test_ast_setinfo_nodeinfo() {
    let mut ast = skp::ast_new().unwrap();
    let o1 = skp::ast_open(&mut ast, 0, "root");
    let o2 = skp::ast_open(&mut ast, 0, "child1");
    skp::ast_close(&mut ast, 3, o2);
    skp::ast_close(&mut ast, 5, o1);

    skp::ast_setinfo(&mut ast, 42, 1);
    assert_eq!(skp::astnodeinfo(&ast, 1), 42);
    assert_eq!(skp::astnodeinfo(&ast, 0), 0); // root tag unchanged
}

#[test]
fn test_ast_delete() {
    let mut ast = skp::ast_new().unwrap();
    let o1 = skp::ast_open(&mut ast, 0, "root");
    let o2 = skp::ast_open(&mut ast, 0, "child1");
    skp::ast_close(&mut ast, 3, o2);
    let o3 = skp::ast_open(&mut ast, 3, "child2");
    skp::ast_close(&mut ast, 5, o3);
    skp::ast_close(&mut ast, 5, o1);

    let before = ast.par_cnt;
    skp::ast_delete(&mut ast);
    // Deletes the last node (root, which spans all 6 par entries)
    assert!(ast.par_cnt < before);
}

#[test]
fn test_ast_noleaf() {
    let mut ast = skp::ast_new().unwrap();
    let o1 = skp::ast_open(&mut ast, 0, "root");
    let o2 = skp::ast_open(&mut ast, 0, "leaf");
    skp::ast_close(&mut ast, 0, o2); // leaf (delta=1)
    skp::ast_close(&mut ast, 5, o1);

    let before = ast.par_cnt;
    skp::ast_noleaf(&mut ast);
    // Last node is root (not a leaf since it has children), so no change
    assert_eq!(ast.par_cnt, before);
}

#[test]
fn test_ast_lastnodeisempty() {
    let mut ast = skp::ast_new().unwrap();
    let o1 = skp::ast_open(&mut ast, 0, "leaf");
    skp::ast_close(&mut ast, 0, o1); // from=0, to=0 -> empty
    assert!(skp::ast_lastnodeisempty(&ast));
}

#[test]
fn test_ast_lastnodeisempty_false() {
    let mut ast = skp::ast_new().unwrap();
    let o1 = skp::ast_open(&mut ast, 0, "leaf");
    skp::ast_close(&mut ast, 3, o1); // from=0, to=3 -> not empty
    assert!(!skp::ast_lastnodeisempty(&ast));
}

#[test]
fn test_ast_nodefrom_nodeto() {
    let mut ast = skp::ast_new().unwrap();
    ast.start = "hello world".to_string();
    let o1 = skp::ast_open(&mut ast, 0, "root");
    skp::ast_close(&mut ast, 5, o1);

    let from = skp::astnodefrom(&ast, 0);
    let to = skp::astnodeto(&ast, 0);
    assert_eq!(&from[..5], "hello");
    assert_eq!(to, " world");
}

#[test]
fn test_ast_isn() {
    let mut ast = skp::ast_new().unwrap();
    let o1 = skp::ast_open(&mut ast, 0, "root");
    skp::ast_close(&mut ast, 5, o1);

    assert_eq!(skp::ast_isn(&ast, 0, "root", None, None, None, None), 1);
    assert_eq!(skp::ast_isn(&ast, 0, "other", Some("root"), None, None, None), 1);
    assert_eq!(skp::ast_isn(&ast, 0, "a", Some("b"), Some("c"), None, None), 0);
}

#[test]
fn test_astfree() {
    let ast = skp::ast_new().unwrap();
    let result = skp::astfree(ast);
    assert!(result.is_none());
}

#[test]
fn test_ast_navigation_boundaries() {
    let ast = skp::ast_new().unwrap();
    // Empty AST - par_cnt=0, so all navigation returns ASTNULL
    assert_eq!(skp::astdown(&ast, 0), -1);
    assert_eq!(skp::astup(&ast, 0), -1);
    assert_eq!(skp::astleft(&ast, 0), -1);
    assert_eq!(skp::astright(&ast, 0), -1);
    // astnextdf(-1): ndx becomes 0, but 0 >= par_cnt(0), so returns ASTNULL
    assert_eq!(skp::astnextdf(&ast, -1), -1);
}

#[test]
fn test_astnoderule_invalid() {
    let ast = skp::ast_new().unwrap();
    assert_eq!(skp::astnoderule(&ast, -1), "");
    assert_eq!(skp::astnoderule(&ast, 100), "");
}

#[test]
fn test_astnodelen_invalid() {
    let ast = skp::ast_new().unwrap();
    assert_eq!(skp::astnodelen(&ast, -1), 0);
    assert_eq!(skp::astnodelen(&ast, 100), 0);
}

#[test]
fn test_astnodeinfo_invalid() {
    let ast = skp::ast_new().unwrap();
    assert_eq!(skp::astnodeinfo(&ast, -1), 0);
    assert_eq!(skp::astnodeinfo(&ast, 100), 0);
}

fn main() {}
