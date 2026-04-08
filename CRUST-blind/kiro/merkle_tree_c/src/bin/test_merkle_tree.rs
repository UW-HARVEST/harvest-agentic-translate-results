use merkle_tree_c::merkle_tree::*;

fn node_merge(_ctx: Option<&mut ()>, left: &CbmtNode, right: &CbmtNode) -> CbmtNode {
    let lv = i32::from_le_bytes(left.bytes);
    let rv = i32::from_le_bytes(right.bytes);
    let value = rv - lv;
    CbmtNode { bytes: value.to_le_bytes() }
}

fn node_merge_ctx(_ctx: &mut (), left: &CbmtNode, right: &CbmtNode) -> CbmtNode {
    let lv = i32::from_le_bytes(left.bytes);
    let rv = i32::from_le_bytes(right.bytes);
    CbmtNode { bytes: (rv - lv).to_le_bytes() }
}

fn i32_to_node(v: i32) -> CbmtNode {
    CbmtNode { bytes: v.to_le_bytes() }
}

fn node_to_i32(n: &CbmtNode) -> i32 {
    i32::from_le_bytes(n.bytes)
}

// --- cbmt_node_cmp ---

#[test]
fn test_node_cmp() {
    assert!(cbmt_node_cmp(&i32_to_node(5), &i32_to_node(10)) < 0);
    assert!(cbmt_node_cmp(&i32_to_node(10), &i32_to_node(5)) > 0);
    assert_eq!(cbmt_node_cmp(&i32_to_node(5), &i32_to_node(5)), 0);
}

// --- cbmt_node_copy ---

#[test]
fn test_node_copy() {
    let src = i32_to_node(99);
    let mut dest = CbmtNode::default();
    cbmt_node_copy(&mut dest, &src);
    assert_eq!(dest.bytes, src.bytes);
}

// --- cbmt_simple_bubble_sort ---

#[test]
fn test_bubble_sort_u32_reverse() {
    let mut arr: Vec<u32> = vec![3, 1, 4, 1, 5];
    cbmt_simple_bubble_sort(&mut arr, cbmt_uint32_reverse_cmp);
    assert_eq!(arr, vec![5, 4, 3, 1, 1]);
}

#[test]
fn test_bubble_sort_nodes() {
    let mut nodes = vec![i32_to_node(5), i32_to_node(1), i32_to_node(3)];
    cbmt_simple_bubble_sort(&mut nodes, cbmt_node_cmp);
    assert_eq!(node_to_i32(&nodes[0]), 1);
    assert_eq!(node_to_i32(&nodes[1]), 3);
    assert_eq!(node_to_i32(&nodes[2]), 5);
}

// --- cbmt_tree_root ---

#[test]
fn test_tree_root_empty() {
    let tree = CbmtTree::default();
    assert_eq!(node_to_i32(&cbmt_tree_root(&tree)), 0);
}

// --- cbmt_build_merkle_tree ---

#[test]
fn test_build_tree_empty() {
    let mut tree = CbmtTree::default();
    let leaves = CbmtLeaves { nodes: vec![] };
    let ret = cbmt_build_merkle_tree(&mut tree, &leaves, node_merge);
    assert_eq!(ret, 0);
    assert_eq!(tree.length, 0);
    assert_eq!(node_to_i32(&cbmt_tree_root(&tree)), 0);
}

#[test]
fn test_build_tree_single() {
    let mut tree = CbmtTree::default();
    let leaves = CbmtLeaves { nodes: vec![i32_to_node(42)] };
    let ret = cbmt_build_merkle_tree(&mut tree, &leaves, node_merge);
    assert_eq!(ret, 0);
    assert_eq!(tree.length, 1);
    assert_eq!(node_to_i32(&cbmt_tree_root(&tree)), 42);
}

#[test]
fn test_build_tree_two() {
    let mut tree = CbmtTree::default();
    let leaves = CbmtLeaves { nodes: vec![i32_to_node(10), i32_to_node(20)] };
    let ret = cbmt_build_merkle_tree(&mut tree, &leaves, node_merge);
    assert_eq!(ret, 0);
    assert_eq!(tree.length, 3);
    assert_eq!(node_to_i32(&tree.nodes[0]), 10);
    assert_eq!(node_to_i32(&tree.nodes[1]), 10);
    assert_eq!(node_to_i32(&tree.nodes[2]), 20);
}

#[test]
fn test_build_tree_three() {
    let mut tree = CbmtTree::default();
    let leaves = CbmtLeaves { nodes: vec![i32_to_node(1), i32_to_node(2), i32_to_node(3)] };
    let ret = cbmt_build_merkle_tree(&mut tree, &leaves, node_merge);
    assert_eq!(ret, 0);
    assert_eq!(tree.length, 5);
    let expected = [0, 1, 1, 2, 3];
    for (i, &e) in expected.iter().enumerate() {
        assert_eq!(node_to_i32(&tree.nodes[i]), e, "node[{}]", i);
    }
}

#[test]
fn test_build_tree_five() {
    let mut tree = CbmtTree::default();
    let leaves = CbmtLeaves {
        nodes: vec![i32_to_node(2), i32_to_node(3), i32_to_node(5), i32_to_node(7), i32_to_node(11)],
    };
    let ret = cbmt_build_merkle_tree(&mut tree, &leaves, node_merge);
    assert_eq!(ret, 0);
    assert_eq!(tree.length, 9);
    let expected = [4, -2, 2, 4, 2, 3, 5, 7, 11];
    for (i, &e) in expected.iter().enumerate() {
        assert_eq!(node_to_i32(&tree.nodes[i]), e, "node[{}]", i);
    }
}

// --- cbmt_build_merkle_root ---

#[test]
fn test_build_root_empty() {
    let leaves = CbmtLeaves { nodes: vec![] };
    let root = cbmt_build_merkle_root(&leaves, node_merge).unwrap();
    assert_eq!(node_to_i32(&root), 0);
}

#[test]
fn test_build_root_single() {
    let leaves = CbmtLeaves { nodes: vec![i32_to_node(42)] };
    let root = cbmt_build_merkle_root(&leaves, node_merge).unwrap();
    assert_eq!(node_to_i32(&root), 42);
}

#[test]
fn test_build_root_two() {
    let leaves = CbmtLeaves { nodes: vec![i32_to_node(2), i32_to_node(3)] };
    let root = cbmt_build_merkle_root(&leaves, node_merge).unwrap();
    assert_eq!(node_to_i32(&root), 1);
}

#[test]
fn test_build_root_three() {
    let leaves = CbmtLeaves { nodes: vec![i32_to_node(1), i32_to_node(2), i32_to_node(3)] };
    let root = cbmt_build_merkle_root(&leaves, node_merge).unwrap();
    assert_eq!(node_to_i32(&root), 0);
}

#[test]
fn test_build_root_five() {
    let leaves = CbmtLeaves {
        nodes: vec![i32_to_node(2), i32_to_node(3), i32_to_node(5), i32_to_node(7), i32_to_node(11)],
    };
    let root = cbmt_build_merkle_root(&leaves, node_merge).unwrap();
    assert_eq!(node_to_i32(&root), 4);
}

// --- cbmt_tree_build_proof ---

#[test]
fn test_tree_build_proof_two_indices() {
    let mut tree = CbmtTree::default();
    let leaves = CbmtLeaves {
        nodes: vec![i32_to_node(2), i32_to_node(3), i32_to_node(5), i32_to_node(7), i32_to_node(11)],
    };
    cbmt_build_merkle_tree(&mut tree, &leaves, node_merge);

    let leaf_indices = CbmtIndices { values: vec![0, 3], capacity: 2 };
    let proof = cbmt_tree_build_proof(&tree, &leaf_indices).unwrap();
    assert_eq!(proof.indices.values.len(), 2);
    assert_eq!(proof.indices.values[0], 4);
    assert_eq!(proof.indices.values[1], 7);
    assert_eq!(proof.lemmas.len(), 2);
    assert_eq!(node_to_i32(&proof.lemmas[0]), 11);
    assert_eq!(node_to_i32(&proof.lemmas[1]), 2);
}

#[test]
fn test_tree_build_proof_single_index() {
    let mut tree = CbmtTree::default();
    let leaves = CbmtLeaves {
        nodes: vec![i32_to_node(2), i32_to_node(3), i32_to_node(5), i32_to_node(7), i32_to_node(11)],
    };
    cbmt_build_merkle_tree(&mut tree, &leaves, node_merge);

    let leaf_indices = CbmtIndices { values: vec![2], capacity: 1 };
    let proof = cbmt_tree_build_proof(&tree, &leaf_indices).unwrap();
    assert_eq!(proof.indices.values.len(), 1);
    assert_eq!(proof.indices.values[0], 6);
    assert_eq!(proof.lemmas.len(), 2);
    assert_eq!(node_to_i32(&proof.lemmas[0]), 3);
    assert_eq!(node_to_i32(&proof.lemmas[1]), -2);
}

#[test]
fn test_tree_build_proof_empty_tree() {
    let tree = CbmtTree::default();
    let leaf_indices = CbmtIndices { values: vec![0], capacity: 1 };
    assert_eq!(cbmt_tree_build_proof(&tree, &leaf_indices).unwrap_err(), CBMT_ERROR_BUILD_PROOF);
}

#[test]
fn test_tree_build_proof_empty_indices() {
    let mut tree = CbmtTree::default();
    let leaves = CbmtLeaves { nodes: vec![i32_to_node(1)] };
    cbmt_build_merkle_tree(&mut tree, &leaves, node_merge);
    let leaf_indices = CbmtIndices { values: vec![], capacity: 0 };
    assert_eq!(cbmt_tree_build_proof(&tree, &leaf_indices).unwrap_err(), CBMT_ERROR_BUILD_PROOF);
}

// --- cbmt_proof_verify ---

#[test]
fn test_proof_verify_success() {
    let mut tree = CbmtTree::default();
    let leaves = CbmtLeaves {
        nodes: vec![i32_to_node(2), i32_to_node(3), i32_to_node(5), i32_to_node(7), i32_to_node(11)],
    };
    cbmt_build_merkle_tree(&mut tree, &leaves, node_merge);
    let root = cbmt_tree_root(&tree);

    let leaf_indices = CbmtIndices { values: vec![0, 3], capacity: 2 };
    let proof = cbmt_tree_build_proof(&tree, &leaf_indices).unwrap();

    // The needed leaves are the nodes at the proof indices
    let needed = CbmtLeaves {
        nodes: proof.indices.values.iter().map(|&i| tree.nodes[i as usize].clone()).collect(),
    };

    let ret = cbmt_proof_verify(&proof, &root, &needed, node_merge);
    assert_eq!(ret, 0);
}

#[test]
fn test_proof_verify_wrong_root() {
    let mut tree = CbmtTree::default();
    let leaves = CbmtLeaves {
        nodes: vec![i32_to_node(2), i32_to_node(3), i32_to_node(5), i32_to_node(7), i32_to_node(11)],
    };
    cbmt_build_merkle_tree(&mut tree, &leaves, node_merge);

    let leaf_indices = CbmtIndices { values: vec![0, 3], capacity: 2 };
    let proof = cbmt_tree_build_proof(&tree, &leaf_indices).unwrap();

    let needed = CbmtLeaves {
        nodes: proof.indices.values.iter().map(|&i| tree.nodes[i as usize].clone()).collect(),
    };

    let wrong_root = i32_to_node(999);
    let ret = cbmt_proof_verify(&proof, &wrong_root, &needed, node_merge);
    assert_eq!(ret, CBMT_ERROR_VERIFY_FAILED);
}

#[test]
fn test_proof_verify_empty_leaves() {
    let proof = CbmtProof {
        indices: CbmtIndices { values: vec![], capacity: 0 },
        lemmas: vec![],
    };
    let root = i32_to_node(0);
    let leaves = CbmtLeaves { nodes: vec![] };
    let ret = cbmt_proof_verify(&proof, &root, &leaves, node_merge);
    assert_eq!(ret, CBMT_ERROR_PROOF_ROOT);
}

// --- cbmt_proof_root ---

#[test]
fn test_proof_root() {
    let mut tree = CbmtTree::default();
    let leaves = CbmtLeaves {
        nodes: vec![i32_to_node(2), i32_to_node(3), i32_to_node(5), i32_to_node(7), i32_to_node(11)],
    };
    cbmt_build_merkle_tree(&mut tree, &leaves, node_merge);

    let leaf_indices = CbmtIndices { values: vec![0, 3], capacity: 2 };
    let proof = cbmt_tree_build_proof(&tree, &leaf_indices).unwrap();

    let needed = CbmtLeaves {
        nodes: proof.indices.values.iter().map(|&i| tree.nodes[i as usize].clone()).collect(),
    };

    let mut root = CbmtNode::default();
    let mut ctx = ();
    let nb = CbmtBuffer { data: &mut [0u8; 1024], capacity: 1024 };
    let pb = CbmtBuffer { data: &mut [0u8; 1024], capacity: 1024 };
    let ret = cbmt_proof_root(&proof, &mut root, &needed, node_merge_ctx, &mut ctx, nb, pb);
    assert_eq!(ret, 0);
    assert_eq!(node_to_i32(&root), 4);
}

// --- cbmt_build_merkle_proof ---

#[test]
fn test_build_merkle_proof() {
    let leaves = CbmtLeaves {
        nodes: vec![i32_to_node(2), i32_to_node(3), i32_to_node(5), i32_to_node(7), i32_to_node(11)],
    };
    let leaf_indices = CbmtIndices { values: vec![0, 3], capacity: 2 };

    let mut proof = CbmtProof {
        indices: CbmtIndices { values: vec![], capacity: 0 },
        lemmas: vec![],
    };
    let mut ctx = ();
    let nb = CbmtBuffer { data: &mut [0u8; 1024], capacity: 1024 };
    let ib = CbmtBuffer { data: &mut [0u8; 1024], capacity: 1024 };
    let lb = CbmtBuffer { data: &mut [0u8; 1024], capacity: 1024 };
    let ret = cbmt_build_merkle_proof(&mut proof, &leaves, &leaf_indices, node_merge_ctx, &mut ctx, nb, ib, lb);
    assert_eq!(ret, 0);
    assert_eq!(proof.indices.values.len(), 2);
    assert_eq!(proof.indices.values[0], 4);
    assert_eq!(proof.indices.values[1], 7);
    assert_eq!(proof.lemmas.len(), 2);
    assert_eq!(node_to_i32(&proof.lemmas[0]), 11);
    assert_eq!(node_to_i32(&proof.lemmas[1]), 2);
}

// --- cbmt_node_pair_reverse_cmp ---

#[test]
fn test_node_pair_reverse_cmp() {
    let a = CbmtNodePair { index: 3, node: CbmtNode::default() };
    let b = CbmtNodePair { index: 7, node: CbmtNode::default() };
    assert!(cbmt_node_pair_reverse_cmp(&a, &b) > 0); // reverse: 7-3 > 0
    assert!(cbmt_node_pair_reverse_cmp(&b, &a) < 0);
    let c = CbmtNodePair { index: 3, node: CbmtNode::default() };
    assert_eq!(cbmt_node_pair_reverse_cmp(&a, &c), 0);
}

// --- cbmt_universal_swap ---

#[test]
fn test_universal_swap() {
    let mut left = [1u8, 2, 3, 4];
    let mut right = [5u8, 6, 7, 8];
    cbmt_universal_swap(&mut left, &mut right, 4);
    assert_eq!(left, [5, 6, 7, 8]);
    assert_eq!(right, [1, 2, 3, 4]);
}

fn main() {}
