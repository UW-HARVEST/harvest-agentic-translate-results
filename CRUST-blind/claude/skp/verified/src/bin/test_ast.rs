// Tests for the AST API.
// Where the C version uses pointers and manual setjmp/longjmp, the Rust API
// uses owned types — we verify behaviour rather than identical pointer math.
use skp::skp::*;

// ===== ast_new / astfree =====
#[test]
fn test_ast_new_initial_state() {
    let ast = ast_new().expect("ast_new returned None");
    assert_eq!(ast.nodes_cnt, 0);
    assert_eq!(ast.nodes_max, 8);
    assert_eq!(ast.par_cnt, 0);
    assert_eq!(ast.par_max, 16);
    assert_eq!(ast.mmz_cnt, 0);
    assert_eq!(ast.mmz_max, 64);
    assert_eq!(ast.lastpos, 0);
    assert_eq!(ast.pos, 0);
    assert_eq!(ast.fail, 0);
    assert_eq!(ast.depth, 0);
    assert_eq!(ast.err_pos, -1);
    assert_eq!(ast.cur_node, -1); // ASTNULL
    assert!(ast.cur_rule.is_none());
    assert!(ast.err_rule.is_none());
    assert_eq!(ast.lastinfo, 0);
    assert_eq!(ast.ret, 0);
    assert_eq!(ast.flg, 0);
}

#[test]
fn test_astfree_returns_none() {
    let ast = ast_new().unwrap();
    assert!(astfree(ast).is_none());
}

// ===== ast_open / ast_close =====
#[test]
fn test_ast_open_and_close_creates_node() {
    let mut ast = ast_new().unwrap();
    let par = ast_open(&mut ast, 0, "rootrule");
    assert_eq!(par, 0);
    assert_eq!(ast.par_cnt, 1);
    assert_eq!(ast.nodes_cnt, 1);
    assert_eq!(ast.par[0], 0);
    assert_eq!(ast.nodes[0].rule, "rootrule");
    assert_eq!(ast.nodes[0].from, 0);

    // close the node
    let p2 = ast_close(&mut ast, 5, par);
    assert_eq!(p2, 1);
    assert_eq!(ast.par_cnt, 2);
    // delta = par2 - open = 1 - 0 = 1
    assert_eq!(ast.par[1], -1);
    assert_eq!(ast.nodes[0].to, 5);
    assert_eq!(ast.nodes[0].delta, 1);
    assert_eq!(ast.cur_node, 1);
    assert_eq!(ast.cur_rule.as_deref(), Some("rootrule"));
}

#[test]
fn test_ast_open_when_failed_returns_negative() {
    let mut ast = ast_new().unwrap();
    ast.fail = 1;
    let par = ast_open(&mut ast, 0, "x");
    assert_eq!(par, -1);
    assert_eq!(ast.par_cnt, 0);
}

// ===== nested nodes via ast_open/close =====
#[test]
fn test_nested_nodes_have_correct_delta() {
    let mut ast = ast_new().unwrap();
    let p_root = ast_open(&mut ast, 0, "root");
    let p_child1 = ast_open(&mut ast, 0, "c1");
    ast_close(&mut ast, 1, p_child1);
    let p_child2 = ast_open(&mut ast, 1, "c2");
    ast_close(&mut ast, 2, p_child2);
    ast_close(&mut ast, 2, p_root);

    // 6 par entries: root_open, c1_open, c1_close, c2_open, c2_close, root_close
    assert_eq!(ast.par_cnt, 6);
    assert_eq!(ast.par[0], 0); // node idx for root
    assert_eq!(ast.par[1], 1); // c1 node
    assert_eq!(ast.par[2], -1); // c1 close
    assert_eq!(ast.par[3], 2); // c2 node
    assert_eq!(ast.par[4], -1); // c2 close
    assert_eq!(ast.par[5], -5); // root close, delta = 5-0 = 5
}

// ===== ast_lastnode =====
#[test]
fn test_ast_lastnode_returns_last_open() {
    let mut ast = ast_new().unwrap();
    let p_root = ast_open(&mut ast, 0, "root");
    let p_a = ast_open(&mut ast, 0, "a");
    ast_close(&mut ast, 2, p_a);
    let p_b = ast_open(&mut ast, 2, "b");
    ast_close(&mut ast, 3, p_b);
    ast_close(&mut ast, 3, p_root);
    // last node opens at par index 5? No, root closes at index 5.
    // The "last node" before root closes would be b, but after root closes
    // the last entry is the close of root; ast_lastnode returns its open.
    let last = ast_lastnode(&ast);
    // last close is at par_cnt-1=5; par[5] = -5; o1 = 5 + (-5) = 0
    assert_eq!(last, 0);
}

#[test]
fn test_ast_lastnode_empty_returns_null() {
    let ast = ast_new().unwrap();
    assert_eq!(ast_lastnode(&ast), -1);
}

#[test]
fn test_ast_lastnode_when_failed() {
    let mut ast = ast_new().unwrap();
    ast.fail = 1;
    assert_eq!(ast_lastnode(&ast), -1);
}

// ===== ast_lastnodeisempty =====
#[test]
fn test_ast_lastnodeisempty_true_for_zero_len() {
    let mut ast = ast_new().unwrap();
    let p = ast_open(&mut ast, 5, "leaf");
    ast_close(&mut ast, 5, p);
    assert!(ast_lastnodeisempty(&ast));
}

#[test]
fn test_ast_lastnodeisempty_false_for_nonempty() {
    let mut ast = ast_new().unwrap();
    let p = ast_open(&mut ast, 0, "leaf");
    ast_close(&mut ast, 5, p);
    assert!(!ast_lastnodeisempty(&ast));
}

// ===== ast_delete =====
#[test]
fn test_ast_delete_removes_last_node() {
    let mut ast = ast_new().unwrap();
    let p1 = ast_open(&mut ast, 0, "a");
    ast_close(&mut ast, 1, p1);
    let p2 = ast_open(&mut ast, 1, "b");
    ast_close(&mut ast, 2, p2);
    assert_eq!(ast.par_cnt, 4);
    ast_delete(&mut ast);
    // Removes the leaf "b"
    assert_eq!(ast.par_cnt, 2);
}

#[test]
fn test_ast_delete_when_empty_does_nothing() {
    let mut ast = ast_new().unwrap();
    ast_delete(&mut ast);
    assert_eq!(ast.par_cnt, 0);
}

// ===== ast_noleaf =====
#[test]
fn test_ast_noleaf_removes_leaf() {
    let mut ast = ast_new().unwrap();
    let p1 = ast_open(&mut ast, 0, "a");
    ast_close(&mut ast, 1, p1);
    assert_eq!(ast.par_cnt, 2);
    ast_noleaf(&mut ast);
    assert_eq!(ast.par_cnt, 0);
}

#[test]
fn test_ast_noleaf_keeps_non_leaf() {
    let mut ast = ast_new().unwrap();
    let p_root = ast_open(&mut ast, 0, "root");
    let p_a = ast_open(&mut ast, 0, "a");
    ast_close(&mut ast, 1, p_a);
    ast_close(&mut ast, 1, p_root);
    assert_eq!(ast.par_cnt, 4);
    // Last node is the root with one child, not a leaf — keeps it
    ast_noleaf(&mut ast);
    assert_eq!(ast.par_cnt, 4);
}

// ===== ast_noemptyleaf =====
#[test]
fn test_ast_noemptyleaf_removes_empty() {
    let mut ast = ast_new().unwrap();
    let p = ast_open(&mut ast, 0, "leaf");
    ast_close(&mut ast, 0, p);
    assert_eq!(ast.par_cnt, 2);
    ast_noemptyleaf(&mut ast);
    assert_eq!(ast.par_cnt, 0);
}

#[test]
fn test_ast_noemptyleaf_keeps_nonempty() {
    let mut ast = ast_new().unwrap();
    let p = ast_open(&mut ast, 0, "leaf");
    ast_close(&mut ast, 5, p);
    ast_noemptyleaf(&mut ast);
    assert_eq!(ast.par_cnt, 2);
}

// ===== ast_swap =====
#[test]
fn test_ast_swap_two_leaves() {
    let mut ast = ast_new().unwrap();
    let p_root = ast_open(&mut ast, 0, "r");
    let p_a = ast_open(&mut ast, 0, "a");
    ast_close(&mut ast, 1, p_a);
    let p_b = ast_open(&mut ast, 1, "b");
    ast_close(&mut ast, 2, p_b);
    // par_cnt now = 5; need to ast_close root for ast_swap to consider 4 things…
    // Actually ast_swap looks at par_cnt - 1 (close), then -2 etc.
    // It doesn't require root to be closed; it only swaps the last two top-level
    // nodes regardless. Let's call swap before closing root.
    assert_eq!(ast.par_cnt, 5);

    // par[1]=node_a(1); par[2]=-1; par[3]=node_b(2); par[4]=-1
    let before_a_node = ast.par[1];
    let before_b_node = ast.par[3];
    ast_swap(&mut ast);
    // After swap, b should be first, a second
    // o2=1, c2=2, o1=3, c1=4
    // tmp = [par[1]=node_a, par[2]=-1] (len2=2)
    // memmove par[o2=1] <- par[o1=3] for len1=2 -> par[1]=par[3]=node_b, par[2]=par[4]=-1
    // memcpy par[o2+len1=3] <- tmp -> par[3]=node_a, par[4]=-1
    assert_eq!(ast.par[1], before_b_node);
    assert_eq!(ast.par[3], before_a_node);
    let _ = p_root;
}

#[test]
fn test_ast_swap_does_nothing_when_failed() {
    let mut ast = ast_new().unwrap();
    let p_a = ast_open(&mut ast, 0, "a");
    ast_close(&mut ast, 1, p_a);
    let p_b = ast_open(&mut ast, 1, "b");
    ast_close(&mut ast, 2, p_b);
    ast.fail = 1;
    ast_swap(&mut ast);
    // node_a's index unchanged
    assert_eq!(ast.par[0], 0);
    assert_eq!(ast.par[2], 1);
}

// ===== ast_lift =====
#[test]
fn test_ast_lift_collapses_single_child() {
    let mut ast = ast_new().unwrap();
    let p_outer = ast_open(&mut ast, 0, "outer"); // par[0] = node 0
    let p_inner = ast_open(&mut ast, 0, "inner"); // par[1] = node 1
    ast_close(&mut ast, 5, p_inner);              // par[2] = -1 (close inner)
    ast_close(&mut ast, 5, p_outer);              // par[3] = -3 (close outer)
    // ast.nodes[0].tag is 0 by default, so ast_lift will collapse outer.
    assert_eq!(ast.par_cnt, 4);
    ast_lift(&mut ast);
    assert_eq!(ast.par_cnt, 2);
    // What remains is the inner node's open/close
    assert_eq!(ast.par[0], 1);
    assert_eq!(ast.par[1], -1);
}

#[test]
fn test_ast_lift_keeps_multi_child() {
    let mut ast = ast_new().unwrap();
    let p_outer = ast_open(&mut ast, 0, "outer");
    let p_a = ast_open(&mut ast, 0, "a");
    ast_close(&mut ast, 1, p_a);
    let p_b = ast_open(&mut ast, 1, "b");
    ast_close(&mut ast, 2, p_b);
    ast_close(&mut ast, 2, p_outer);
    // outer has two children → lift does nothing
    let before = ast.par_cnt;
    ast_lift(&mut ast);
    assert_eq!(ast.par_cnt, before);
}

// ===== ast_lift_all =====
#[test]
fn test_ast_lift_all_collapses_chain() {
    let mut ast = ast_new().unwrap();
    let p3 = ast_open(&mut ast, 0, "outer");
    let p2 = ast_open(&mut ast, 0, "mid");
    let p1 = ast_open(&mut ast, 0, "inner");
    ast_close(&mut ast, 3, p1);
    ast_close(&mut ast, 3, p2);
    ast_close(&mut ast, 3, p3);
    assert_eq!(ast.par_cnt, 6);
    ast_lift_all(&mut ast);
    // Should collapse to just the innermost
    assert_eq!(ast.par_cnt, 2);
}

// ===== siblings: astleft, astright, astup, astdown, astfirst, astlast =====
#[test]
fn test_ast_siblings_navigation() {
    // Build: root(a, b)
    let mut ast = ast_new().unwrap();
    let p_root = ast_open(&mut ast, 0, "root");
    let p_a = ast_open(&mut ast, 0, "a");
    ast_close(&mut ast, 1, p_a);
    let p_b = ast_open(&mut ast, 1, "b");
    ast_close(&mut ast, 2, p_b);
    ast_close(&mut ast, 2, p_root);

    // par layout: [0:root_open, 1:a_open, 2:a_close, 3:b_open, 4:b_close, 5:root_close]
    // a is at par index 1, b is at par index 3
    let a = 1;
    let b = 3;

    // astright(a) = b
    assert_eq!(astright(&ast, a), b);
    // astleft(b) = a
    assert_eq!(astleft(&ast, b), a);
    // astup(a) = root (par index 0)
    assert_eq!(astup(&ast, a), 0);
    // astdown(root) = a
    assert_eq!(astdown(&ast, 0), 1);
    // astfirst(b) = a
    assert_eq!(astfirst(&ast, b), a);
    // astlast(a) = b
    assert_eq!(astlast(&ast, a), b);
    // No further left/right
    assert_eq!(astleft(&ast, a), -1);
    assert_eq!(astright(&ast, b), -1);
}

// ===== ast_isn / ast_is =====
#[test]
fn test_ast_is_matches_rule() {
    let mut ast = ast_new().unwrap();
    let p = ast_open(&mut ast, 0, "myrule");
    ast_close(&mut ast, 5, p);
    assert_eq!(ast_is(&ast, 0, "myrule"), 1);
    assert_eq!(ast_is(&ast, 0, "other"), 0);
    // Closing par also resolves to same rule
    assert_eq!(ast_is(&ast, 1, "myrule"), 1);
}

#[test]
fn test_ast_isn_matches_any() {
    let mut ast = ast_new().unwrap();
    let p = ast_open(&mut ast, 0, "alpha");
    ast_close(&mut ast, 5, p);
    assert_eq!(ast_isn(&ast, 0, "alpha", None, None, None, None), 1);
    assert_eq!(ast_isn(&ast, 0, "x", Some("y"), Some("alpha"), None, None), 1);
    assert_eq!(ast_isn(&ast, 0, "x", Some("y"), Some("z"), None, None), 0);
}

// ===== astisleaf =====
#[test]
fn test_ast_isleaf_true() {
    let mut ast = ast_new().unwrap();
    let p = ast_open(&mut ast, 0, "leaf");
    ast_close(&mut ast, 5, p);
    assert!(astisleaf(&ast, 0));
}

#[test]
fn test_ast_isleaf_false_for_branch() {
    let mut ast = ast_new().unwrap();
    let p = ast_open(&mut ast, 0, "branch");
    let p_c = ast_open(&mut ast, 0, "c");
    ast_close(&mut ast, 1, p_c);
    ast_close(&mut ast, 1, p);
    assert!(!astisleaf(&ast, 0));
}

// ===== astnodeinfo / ast_setinfo / astnewinfo =====
#[test]
fn test_ast_setinfo_and_get() {
    let mut ast = ast_new().unwrap();
    let p = ast_open(&mut ast, 0, "x");
    ast_close(&mut ast, 5, p);
    ast_setinfo(&mut ast, 42, 0);
    assert_eq!(astnodeinfo(&ast, 0), 42);
    // ASTNULL means "last par"
    ast_setinfo(&mut ast, 99, -1);
    assert_eq!(astnodeinfo(&ast, 1), 99);
}

#[test]
fn test_astnewinfo_creates_info_node() {
    let mut ast = ast_new().unwrap();
    let cnt_before = ast.par_cnt;
    astnewinfo(&mut ast, 7);
    // Adds one open + one close
    assert_eq!(ast.par_cnt, cnt_before + 2);
    assert_eq!(ast.lastinfo, 7);
    // The new node has rule "#"
    let last = ast_lastnode(&ast);
    assert_eq!(astnoderule(&ast, last), "#");
}

// ===== astnoderule / astnodefrom / astnodeto / astnodelen =====
#[test]
fn test_node_rule_from_to_len() {
    let mut ast = ast_new().unwrap();
    ast.start = "Hello, World".to_string();
    let p = ast_open(&mut ast, 7, "word");
    ast_close(&mut ast, 12, p);

    assert_eq!(astnoderule(&ast, 0), "word");
    // from is byte offset 7 → "World"
    assert_eq!(astnodefrom(&ast, 0), "World");
    // to is byte offset 12 → ""
    assert_eq!(astnodeto(&ast, 0), "");
    assert_eq!(astnodelen(&ast, 0), 5);
}

#[test]
fn test_node_rule_invalid_node() {
    let ast = ast_new().unwrap();
    assert_eq!(astnoderule(&ast, 0), "");
    assert_eq!(astnodefrom(&ast, 0), "");
    assert_eq!(astnodeto(&ast, 0), "");
    assert_eq!(astnodelen(&ast, 0), 0);
    assert!(!astisleaf(&ast, 0));
}

// ===== astnextdf / astisnodeentry / astisnodeexit =====
#[test]
fn test_ast_traversal_helpers() {
    let mut ast = ast_new().unwrap();
    let p_root = ast_open(&mut ast, 0, "r");
    let p_a = ast_open(&mut ast, 0, "a");
    ast_close(&mut ast, 1, p_a);
    ast_close(&mut ast, 1, p_root);

    // astnextdf starts from -1 (ASTNULL) and returns 0
    let n0 = astnextdf(&ast, -1);
    assert_eq!(n0, 0);
    let n1 = astnextdf(&ast, n0);
    assert_eq!(n1, 1);
    let n2 = astnextdf(&ast, n1);
    assert_eq!(n2, 2);
    let n3 = astnextdf(&ast, n2);
    assert_eq!(n3, 3);
    let n4 = astnextdf(&ast, n3);
    assert_eq!(n4, -1);

    // entries vs exits
    assert!(astisnodeentry(&ast, 0));
    assert!(astisnodeentry(&ast, 1));
    assert!(astisnodeexit(&ast, 2));
    assert!(astisnodeexit(&ast, 3));
    assert!(!astisnodeentry(&ast, 2));
    assert!(!astisnodeexit(&ast, 1));

    // out of range
    assert!(!astisnodeentry(&ast, -1));
    assert!(!astisnodeexit(&ast, 100));
}

// ===== asthaserr / asterrrule / asterrpos / asterrline / asterrcolnum =====
#[test]
fn test_ast_err_helpers_no_error() {
    let ast = ast_new().unwrap();
    assert!(!asthaserr(&ast));
    assert_eq!(asterrrule(&ast), Some(""));
    assert_eq!(asterrpos(&ast), Some(""));
    assert_eq!(asterrline(&ast), "");
    assert_eq!(asterrcolnum(&ast), 0);
}

#[test]
fn test_ast_err_with_error() {
    let mut ast = ast_new().unwrap();
    ast.start = "line1\nline2\nline3".to_string();
    ast.err_pos = 10; // points into "line2"
    ast.err_rule = Some("rule".to_string());

    assert!(asthaserr(&ast));
    assert_eq!(asterrrule(&ast), Some("rule"));
    // err_pos 10 = byte 10 = 'e' of line2 (line1\n=6 then 'l','i','n','e' at 6,7,8,9, then '2' at 10)
    assert_eq!(asterrpos(&ast), Some("2\nline3"));
    // line containing pos 10 starts at byte 6
    assert_eq!(asterrline(&ast), "line2\nline3");
    // column = 10 - 6 = 4
    assert_eq!(asterrcolnum(&ast), 4);
}

// ===== skp_debug2 =====
#[test]
fn test_skp_debug2_set_clear_toggle() {
    let mut ast = ast_new().unwrap();
    assert_eq!(skp_debug2(&mut ast, 1), 1);
    assert_eq!(ast.flg & 0x01, 0x01);
    assert_eq!(skp_debug2(&mut ast, 0), 0);
    assert_eq!(ast.flg & 0x01, 0);
    // toggle (any value other than 0/1)
    skp_debug2(&mut ast, 2);
    assert_eq!(ast.flg & 0x01, 0x01);
    skp_debug2(&mut ast, 2);
    assert_eq!(ast.flg & 0x01, 0);
}

// ===== ast_lower =====
#[test]
fn test_ast_lower_wraps_children() {
    // Build: a, b, c at top level
    let mut ast = ast_new().unwrap();
    let p_a = ast_open(&mut ast, 0, "a");
    ast_close(&mut ast, 1, p_a);
    let p_b = ast_open(&mut ast, 1, "b");
    ast_close(&mut ast, 2, p_b);
    let p_c = ast_open(&mut ast, 2, "c");
    ast_close(&mut ast, 3, p_c);
    // par layout: [0,a_open=0; 1,a_close=-1; 2,b_open=1; 3,b_close=-1; 4,c_open=2; 5,c_close=-1]
    // Lower b and c into a wrapper "wrap"
    ast_lower(&mut ast, "wrap", 2, 4);
    // After lower, total par_cnt grows by 2 (one open + one close)
    assert_eq!(ast.par_cnt, 8);
    // The newly added wrap node has its rule
    // The wrap node opens at par index 2
    assert_eq!(astnoderule(&ast, 2), "wrap");
}

// ===== skp_parse =====
fn dummy_rule(_ast: &mut Ast, _ret: &mut i32) {
    // do nothing — succeeds
}

fn fail_rule(ast: &mut Ast, _ret: &mut i32) {
    ast.fail = 1;
}

#[test]
fn test_skp_parse_dummy() {
    let ast = skp_parse("hello", dummy_rule, "myrule", 0).expect("parse returned None");
    // The wrapping root par exists
    assert!(ast.par_cnt >= 2);
    assert_eq!(ast.start, "hello");
    // Debug flag was 0
    assert_eq!(ast.flg & 0x01, 0);
}

#[test]
fn test_skp_parse_with_debug() {
    let ast = skp_parse("hello", dummy_rule, "myrule", 1).unwrap();
    assert_eq!(ast.flg & 0x01, 0x01);
}

#[test]
fn test_skp_parse_failing_rule() {
    let ast = skp_parse("xyz", fail_rule, "rrr", 0).unwrap();
    // fail flag was set; err_rule should be set when err_pos < pos
    assert_eq!(ast.fail, 1);
}

// ===== skp__abort =====
#[test]
fn test_skp_abort_sets_fail_and_msg() {
    let mut ast = ast_new().unwrap();
    ast.pos = 5;
    skp__abort(&mut ast, "boom", "rule1");
    assert_eq!(ast.err_pos, 5);
    assert_eq!(ast.err_rule.as_deref(), Some("rule1"));
    assert_eq!(ast.err_msg.as_deref(), Some("boom"));
    assert_eq!(ast.fail, 1);
}

// ===== skp_memoize / skp_dememoize =====
#[test]
fn test_skp_memoize_dememoize_roundtrip() {
    let mut ast = ast_new().unwrap();
    let p = ast_open(&mut ast, 0, "x");
    ast_close(&mut ast, 4, p);
    // Memoize the current state
    let mut mmz = AstMmz::default();
    skp_memoize(&mut ast, &mut mmz, "x", 0, 0);
    assert_eq!(mmz.pos, 0);
    assert_eq!(mmz.endpos, 0);
    assert_eq!(mmz.numnodes, 1);

    // Now reset and try to dememoize at pos 0
    let mut ast2 = ast_new().unwrap();
    let result = skp_dememoize(&mut ast2, &mut mmz, "x");
    assert_eq!(result, 1);
    // par_cnt should now be 2 (open + close of the memoized node)
    assert_eq!(ast2.par_cnt, 2);
    assert_eq!(ast2.nodes_cnt, 1);
}

#[test]
fn test_skp_dememoize_pos_mismatch_returns_zero() {
    let mut mmz = AstMmz::default();
    mmz.pos = 5;
    mmz.endpos = 10;
    let mut ast = ast_new().unwrap();
    ast.pos = 0;
    let result = skp_dememoize(&mut ast, &mut mmz, "x");
    assert_eq!(result, 0);
}

// ===== astnext alias =====
#[test]
fn test_astnext_alias() {
    let mut ast = ast_new().unwrap();
    let p = ast_open(&mut ast, 0, "x");
    ast_close(&mut ast, 1, p);
    assert_eq!(astnext(&ast, -1), astnextdf(&ast, -1));
    assert_eq!(astnext(&ast, 0), astnextdf(&ast, 0));
}

fn main() {}
