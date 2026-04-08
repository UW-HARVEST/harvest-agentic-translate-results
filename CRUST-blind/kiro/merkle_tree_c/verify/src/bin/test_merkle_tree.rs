use merkle_tree_c::merkle_tree::*;

fn int32_to_node(v: i32) -> CbmtNode {
    CbmtNode { bytes: v.to_le_bytes() }
}

fn node_to_int32(n: &CbmtNode) -> i32 {
    i32::from_le_bytes(n.bytes)
}

fn node_merge(_ctx: Option<&mut ()>, left: &CbmtNode, right: &CbmtNode) -> CbmtNode {
    let lv = i32::from_le_bytes(left.bytes);
    let rv = i32::from_le_bytes(right.bytes);
    int32_to_node(rv - lv)
}

#[test]
fn test_build_empty_tree() {
    let mut tree = CbmtTree::default();
    let leaves = CbmtLeaves { nodes: vec![] };
    let ret = cbmt_build_merkle_tree(&mut tree, &leaves, node_merge);
    assert_eq!(ret, 0);
    assert_eq!(tree.length, 0);
    assert_eq!(node_to_int32(&cbmt_tree_root(&tree)), 0);
}

#[test]
fn test_build_five_leaves() {
    let mut tree = CbmtTree::default();
    let leaves = CbmtLeaves {
        nodes: vec![
            int32_to_node(2),
            int32_to_node(3),
            int32_to_node(5),
            int32_to_node(7),
            int32_to_node(11),
        ],
    };
    let ret = cbmt_build_merkle_tree(&mut tree, &leaves, node_merge);
    assert_eq!(ret, 0);
    assert_eq!(tree.length, 9);
    assert_eq!(node_to_int32(&tree.nodes[0]), 4);
    assert_eq!(node_to_int32(&tree.nodes[1]), -2);
    assert_eq!(node_to_int32(&tree.nodes[2]), 2);
    assert_eq!(node_to_int32(&tree.nodes[3]), 4);
    assert_eq!(node_to_int32(&tree.nodes[4]), 2);
    assert_eq!(node_to_int32(&tree.nodes[5]), 3);
    assert_eq!(node_to_int32(&tree.nodes[6]), 5);
    assert_eq!(node_to_int32(&tree.nodes[7]), 7);
    assert_eq!(node_to_int32(&tree.nodes[8]), 11);
}

#[test]
fn test_build_root_directly_2leaves() {
    let leaves = CbmtLeaves {
        nodes: vec![int32_to_node(2), int32_to_node(3)],
    };
    let result = cbmt_build_merkle_root(&leaves, node_merge);
    assert!(result.is_ok());
    assert_eq!(node_to_int32(&result.unwrap()), 1);
}

#[test]
fn test_build_root_directly_5leaves() {
    let leaves = CbmtLeaves {
        nodes: vec![
            int32_to_node(2),
            int32_to_node(3),
            int32_to_node(5),
            int32_to_node(7),
            int32_to_node(11),
        ],
    };
    let result = cbmt_build_merkle_root(&leaves, node_merge);
    assert!(result.is_ok());
    assert_eq!(node_to_int32(&result.unwrap()), 4);
}

#[test]
fn test_build_root_empty() {
    let leaves = CbmtLeaves { nodes: vec![] };
    let result = cbmt_build_merkle_root(&leaves, node_merge);
    assert!(result.is_ok());
    assert_eq!(node_to_int32(&result.unwrap()), 0);
}

#[test]
fn test_tree_build_proof() {
    let mut tree = CbmtTree::default();
    let leaves = CbmtLeaves {
        nodes: vec![
            int32_to_node(2),
            int32_to_node(3),
            int32_to_node(5),
            int32_to_node(7),
            int32_to_node(11),
        ],
    };
    cbmt_build_merkle_tree(&mut tree, &leaves, node_merge);

    let leaf_indices = CbmtIndices {
        values: vec![0, 3],
        capacity: 2,
    };
    let result = cbmt_tree_build_proof(&tree, &leaf_indices);
    assert!(result.is_ok());
    let proof = result.unwrap();
    assert_eq!(proof.indices.values.len(), 2);
    assert_eq!(proof.indices.values[0], 4);
    assert_eq!(proof.indices.values[1], 7);
    assert_eq!(proof.lemmas.len(), 2);
    assert_eq!(node_to_int32(&proof.lemmas[0]), 11);
    assert_eq!(node_to_int32(&proof.lemmas[1]), 2);
}

#[test]
fn test_proof_verify() {
    let mut tree = CbmtTree::default();
    let leaves = CbmtLeaves {
        nodes: vec![
            int32_to_node(2),
            int32_to_node(3),
            int32_to_node(5),
            int32_to_node(7),
            int32_to_node(11),
        ],
    };
    cbmt_build_merkle_tree(&mut tree, &leaves, node_merge);
    let root = cbmt_tree_root(&tree);

    let leaf_indices = CbmtIndices {
        values: vec![0, 3],
        capacity: 2,
    };
    let proof = cbmt_tree_build_proof(&tree, &leaf_indices).unwrap();

    // Build needed_leaves from proof indices (nodes at those tree positions)
    let needed_leaves = CbmtLeaves {
        nodes: proof.indices.values.iter().map(|&idx| tree.nodes[idx as usize].clone()).collect(),
    };

    let ret = cbmt_proof_verify(&proof, &root, &needed_leaves, node_merge);
    assert_eq!(ret, 0);
}

#[test]
fn test_proof_verify_wrong_root() {
    let mut tree = CbmtTree::default();
    let leaves = CbmtLeaves {
        nodes: vec![
            int32_to_node(2),
            int32_to_node(3),
            int32_to_node(5),
            int32_to_node(7),
            int32_to_node(11),
        ],
    };
    cbmt_build_merkle_tree(&mut tree, &leaves, node_merge);

    let leaf_indices = CbmtIndices {
        values: vec![0, 3],
        capacity: 2,
    };
    let proof = cbmt_tree_build_proof(&tree, &leaf_indices).unwrap();
    let needed_leaves = CbmtLeaves {
        nodes: proof.indices.values.iter().map(|&idx| tree.nodes[idx as usize].clone()).collect(),
    };

    let wrong_root = int32_to_node(999);
    let ret = cbmt_proof_verify(&proof, &wrong_root, &needed_leaves, node_merge);
    assert_eq!(ret, CBMT_ERROR_VERIFY_FAILED);
}

#[test]
fn test_tree_build_proof_empty_tree() {
    let tree = CbmtTree::default();
    let leaf_indices = CbmtIndices { values: vec![0], capacity: 1 };
    let result = cbmt_tree_build_proof(&tree, &leaf_indices);
    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), CBMT_ERROR_BUILD_PROOF);
}

#[test]
fn test_tree_build_proof_empty_indices() {
    let mut tree = CbmtTree::default();
    let leaves = CbmtLeaves { nodes: vec![int32_to_node(2)] };
    cbmt_build_merkle_tree(&mut tree, &leaves, node_merge);
    let leaf_indices = CbmtIndices { values: vec![], capacity: 0 };
    let result = cbmt_tree_build_proof(&tree, &leaf_indices);
    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), CBMT_ERROR_BUILD_PROOF);
}

#[test]
fn test_tree_root_empty() {
    let tree = CbmtTree::default();
    let root = cbmt_tree_root(&tree);
    assert_eq!(node_to_int32(&root), 0);
}

#[test]
fn test_node_cmp() {
    assert!(cbmt_node_cmp(&int32_to_node(1), &int32_to_node(2)) < 0);
    assert!(cbmt_node_cmp(&int32_to_node(5), &int32_to_node(3)) > 0);
    assert_eq!(cbmt_node_cmp(&int32_to_node(7), &int32_to_node(7)), 0);
}

#[test]
fn test_simple_bubble_sort() {
    let mut arr: Vec<u32> = vec![5, 3, 8, 1, 2];
    cbmt_simple_bubble_sort(&mut arr, |a, b| (*a as i32) - (*b as i32));
    assert_eq!(arr, vec![1, 2, 3, 5, 8]);
}

#[test]
fn test_single_leaf_tree() {
    let mut tree = CbmtTree::default();
    let leaves = CbmtLeaves { nodes: vec![int32_to_node(42)] };
    let ret = cbmt_build_merkle_tree(&mut tree, &leaves, node_merge);
    assert_eq!(ret, 0);
    assert_eq!(tree.length, 1);
    assert_eq!(node_to_int32(&cbmt_tree_root(&tree)), 42);
}

#[test]
fn test_single_leaf_root() {
    let leaves = CbmtLeaves { nodes: vec![int32_to_node(42)] };
    let result = cbmt_build_merkle_root(&leaves, node_merge);
    assert!(result.is_ok());
    assert_eq!(node_to_int32(&result.unwrap()), 42);
}

fn main() {}
