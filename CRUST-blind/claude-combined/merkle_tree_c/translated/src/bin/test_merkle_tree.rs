use merkle_tree_c::merkle_tree::*;

fn int32_to_node(value: i32) -> CbmtNode {
    CbmtNode {
        bytes: value.to_le_bytes(),
    }
}
fn node_to_int32(n: &CbmtNode) -> i32 {
    i32::from_le_bytes(n.bytes)
}

fn node_merge(_ctx: Option<&mut ()>, left: &CbmtNode, right: &CbmtNode) -> CbmtNode {
    let l = i32::from_le_bytes(left.bytes);
    let r = i32::from_le_bytes(right.bytes);
    let v = r.wrapping_sub(l);
    CbmtNode {
        bytes: v.to_le_bytes(),
    }
}

fn node_merge_ctx<Ctx>(_ctx: &mut Ctx, left: &CbmtNode, right: &CbmtNode) -> CbmtNode {
    let l = i32::from_le_bytes(left.bytes);
    let r = i32::from_le_bytes(right.bytes);
    let v = r.wrapping_sub(l);
    CbmtNode {
        bytes: v.to_le_bytes(),
    }
}

#[test]
fn test_universal_swap() {
    let mut a = [1u8, 2, 3, 4, 5];
    let mut b = [10u8, 20, 30, 40, 50];
    cbmt_universal_swap(&mut a, &mut b, 5);
    assert_eq!(a, [10, 20, 30, 40, 50]);
    assert_eq!(b, [1, 2, 3, 4, 5]);
}

#[test]
fn test_universal_swap_partial() {
    let mut a = [1u8, 2, 3, 4, 5];
    let mut b = [10u8, 20, 30, 40, 50];
    cbmt_universal_swap(&mut a, &mut b, 3);
    assert_eq!(a, [10, 20, 30, 4, 5]);
    assert_eq!(b, [1, 2, 3, 40, 50]);
}

#[test]
fn test_simple_bubble_sort() {
    let mut v: Vec<i32> = vec![5, 3, 8, 1, 9, 2];
    cbmt_simple_bubble_sort(&mut v, |a, b| a - b);
    assert_eq!(v, vec![1, 2, 3, 5, 8, 9]);
}

#[test]
fn test_simple_bubble_sort_empty_or_single() {
    let mut v: Vec<i32> = vec![];
    cbmt_simple_bubble_sort(&mut v, |a, b| a - b);
    assert!(v.is_empty());

    let mut v2: Vec<i32> = vec![42];
    cbmt_simple_bubble_sort(&mut v2, |a, b| a - b);
    assert_eq!(v2, vec![42]);
}

#[test]
fn test_uint32_reverse_cmp() {
    // right - left
    assert_eq!(cbmt_uint32_reverse_cmp(&5, &10), 5);
    assert_eq!(cbmt_uint32_reverse_cmp(&10, &5), -5);
    assert_eq!(cbmt_uint32_reverse_cmp(&7, &7), 0);
}

#[test]
fn test_buffer_init() {
    let mut data = [0u8; 16];
    let mut buf = CbmtBuffer::default();
    cbmt_buffer_init(&mut buf, &mut data);
    assert_eq!(buf.capacity, 16);
    assert_eq!(buf.data.len(), 16);
}

#[test]
fn test_leaves_init() {
    let mut leaves = CbmtLeaves { nodes: vec![] };
    let nodes = vec![int32_to_node(1), int32_to_node(2)];
    cbmt_leaves_init(&mut leaves, nodes);
    assert_eq!(leaves.nodes.len(), 2);
    assert_eq!(node_to_int32(&leaves.nodes[0]), 1);
    assert_eq!(node_to_int32(&leaves.nodes[1]), 2);
}

#[test]
fn test_indices_init() {
    let mut idx = CbmtIndices {
        values: vec![],
        capacity: 0,
    };
    cbmt_indices_init(&mut idx, vec![1u32, 2, 3]);
    assert_eq!(idx.values, vec![1u32, 2, 3]);
    assert_eq!(idx.capacity, 3);
}

#[test]
fn test_queue_basic() {
    let mut data = [0u8; 16]; // 4 u32 entries
    let mut buf = CbmtBuffer::default();
    cbmt_buffer_init(&mut buf, &mut data);
    let mut queue = CbmtQueue::default();
    let r = cbmt_queue_init(&mut queue, buf, 4, 4);
    assert_eq!(r, 0);
    assert_eq!(queue.capacity, 4);
    assert_eq!(queue.width, 4);
    assert_eq!(queue.length, 0);
    assert_eq!(queue.head, 0);
    assert_eq!(queue.tail, 0);

    let v1: u32 = 100;
    let v2: u32 = 200;
    let v3: u32 = 300;
    let r1 = cbmt_queue_push_back(&mut queue, &v1.to_le_bytes());
    let r2 = cbmt_queue_push_back(&mut queue, &v2.to_le_bytes());
    let r3 = cbmt_queue_push_back(&mut queue, &v3.to_le_bytes());
    assert_eq!(r1, 0);
    assert_eq!(r2, 0);
    assert_eq!(r3, 0);
    assert_eq!(queue.length, 3);

    {
        let front = cbmt_queue_front(&queue).unwrap();
        let mut bytes = [0u8; 4];
        bytes.copy_from_slice(front);
        assert_eq!(u32::from_le_bytes(bytes), 100);
    }

    let mut item = [0u8; 4];
    let r = cbmt_queue_pop_front(&mut queue, &mut item);
    assert_eq!(r, 0);
    assert_eq!(u32::from_le_bytes(item), 100);
    assert_eq!(queue.length, 2);

    let r = cbmt_queue_pop_front(&mut queue, &mut item);
    assert_eq!(r, 0);
    assert_eq!(u32::from_le_bytes(item), 200);

    let r = cbmt_queue_pop_front(&mut queue, &mut item);
    assert_eq!(r, 0);
    assert_eq!(u32::from_le_bytes(item), 300);

    let r = cbmt_queue_pop_front(&mut queue, &mut item);
    assert_eq!(r, CBMT_ERROR_QUEUE_EMPTY);
}

#[test]
fn test_queue_push_front() {
    let mut data = [0u8; 16];
    let mut buf = CbmtBuffer::default();
    cbmt_buffer_init(&mut buf, &mut data);
    let mut queue = CbmtQueue::default();
    cbmt_queue_init(&mut queue, buf, 4, 4);
    let v1: u32 = 1;
    let v2: u32 = 2;
    let v3: u32 = 3;
    cbmt_queue_push_back(&mut queue, &v1.to_le_bytes());
    cbmt_queue_push_back(&mut queue, &v2.to_le_bytes());
    cbmt_queue_push_front(&mut queue, &v3.to_le_bytes());
    assert_eq!(queue.length, 3);

    let mut item = [0u8; 4];
    cbmt_queue_pop_front(&mut queue, &mut item);
    assert_eq!(u32::from_le_bytes(item), 3);
    cbmt_queue_pop_front(&mut queue, &mut item);
    assert_eq!(u32::from_le_bytes(item), 1);
    cbmt_queue_pop_front(&mut queue, &mut item);
    assert_eq!(u32::from_le_bytes(item), 2);
}

#[test]
fn test_queue_over_capacity() {
    let mut data = [0u8; 8];
    let mut buf = CbmtBuffer::default();
    cbmt_buffer_init(&mut buf, &mut data);
    let mut queue = CbmtQueue::default();
    cbmt_queue_init(&mut queue, buf, 4, 2);
    let v: u32 = 1;
    assert_eq!(cbmt_queue_push_back(&mut queue, &v.to_le_bytes()), 0);
    assert_eq!(cbmt_queue_push_back(&mut queue, &v.to_le_bytes()), 0);
    assert_eq!(
        cbmt_queue_push_back(&mut queue, &v.to_le_bytes()),
        CBMT_ERROR_OVER_CAPACITY
    );
    assert_eq!(
        cbmt_queue_push_front(&mut queue, &v.to_le_bytes()),
        CBMT_ERROR_OVER_CAPACITY
    );
}

#[test]
fn test_queue_init_invalid_capacity() {
    let mut data = [0u8; 7];
    let mut buf = CbmtBuffer::default();
    cbmt_buffer_init(&mut buf, &mut data);
    let mut queue = CbmtQueue::default();
    let r = cbmt_queue_init(&mut queue, buf, 4, 1);
    assert_eq!(r, CBMT_ERROR_INVALID_CAPACITY);
}

#[test]
fn test_queue_init_over_capacity() {
    let mut data = [0u8; 8];
    let mut buf = CbmtBuffer::default();
    cbmt_buffer_init(&mut buf, &mut data);
    let mut queue = CbmtQueue::default();
    let r = cbmt_queue_init(&mut queue, buf, 4, 3);
    assert_eq!(r, CBMT_ERROR_OVER_CAPACITY);
}

#[test]
fn test_node_copy() {
    let src = int32_to_node(42);
    let mut dest = CbmtNode::default();
    cbmt_node_copy(&mut dest, &src);
    assert_eq!(node_to_int32(&dest), 42);
    assert_eq!(dest.bytes, src.bytes);
}

#[test]
fn test_node_cmp() {
    let a = int32_to_node(5);
    let b = int32_to_node(7);
    assert_eq!(cbmt_node_cmp(&a, &b), -2);
    assert_eq!(cbmt_node_cmp(&b, &a), 2);
    assert_eq!(cbmt_node_cmp(&a, &a), 0);
}

#[test]
fn test_node_pair_reverse_cmp() {
    let a = CbmtNodePair {
        index: 1,
        node: int32_to_node(0),
    };
    let b = CbmtNodePair {
        index: 5,
        node: int32_to_node(0),
    };
    assert_eq!(cbmt_node_pair_reverse_cmp(&a, &b), 4);
    assert_eq!(cbmt_node_pair_reverse_cmp(&b, &a), -4);
}

#[test]
fn test_build_empty() {
    // C: test_build_empty
    let leaves = CbmtLeaves { nodes: vec![] };
    let mut tree = CbmtTree::default();
    let r = cbmt_build_merkle_tree(&mut tree, &leaves, node_merge);
    assert_eq!(r, 0);
    let root = cbmt_tree_root(&tree);
    assert_eq!(node_to_int32(&root), 0);
    assert_eq!(tree.length, 0);
}

#[test]
fn test_build_five() {
    // C: test_build_five
    let leaves = CbmtLeaves {
        nodes: vec![
            int32_to_node(2),
            int32_to_node(3),
            int32_to_node(5),
            int32_to_node(7),
            int32_to_node(11),
        ],
    };
    let mut tree = CbmtTree::default();
    let r = cbmt_build_merkle_tree(&mut tree, &leaves, node_merge);
    assert_eq!(r, 0);
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
    let root = cbmt_build_merkle_root(&leaves, node_merge).unwrap();
    assert_eq!(node_to_int32(&root), 1);
}

#[test]
fn test_build_root_directly_five() {
    let leaves = CbmtLeaves {
        nodes: vec![
            int32_to_node(2),
            int32_to_node(3),
            int32_to_node(5),
            int32_to_node(7),
            int32_to_node(11),
        ],
    };
    let root = cbmt_build_merkle_root(&leaves, node_merge).unwrap();
    assert_eq!(node_to_int32(&root), 4);
}

#[test]
fn test_build_root_empty() {
    let leaves = CbmtLeaves { nodes: vec![] };
    let root = cbmt_build_merkle_root(&leaves, node_merge).unwrap();
    assert_eq!(node_to_int32(&root), 0);
}

#[test]
fn test_rebuild_proof() {
    // C: test_rebuild_proof
    let leaves = CbmtLeaves {
        nodes: vec![
            int32_to_node(2),
            int32_to_node(3),
            int32_to_node(5),
            int32_to_node(7),
            int32_to_node(11),
        ],
    };
    let mut tree = CbmtTree::default();
    let r = cbmt_build_merkle_tree(&mut tree, &leaves, node_merge);
    assert_eq!(r, 0);
    let root = cbmt_tree_root(&tree);

    let leaf_indices = CbmtIndices {
        values: vec![0, 3],
        capacity: 2,
    };

    let proof = cbmt_tree_build_proof(&tree, &leaf_indices).unwrap();
    assert_eq!(proof.indices.values.len(), 2);
    assert_eq!(proof.indices.values[0], 4);
    assert_eq!(proof.indices.values[1], 7);
    assert_eq!(proof.lemmas.len(), 2);
    assert_eq!(node_to_int32(&proof.lemmas[0]), 11);
    assert_eq!(node_to_int32(&proof.lemmas[1]), 2);

    // Verify proof
    let needed_nodes: Vec<CbmtNode> = proof
        .indices
        .values
        .iter()
        .map(|&i| tree.nodes[i as usize].clone())
        .collect();
    let needed_leaves = CbmtLeaves {
        nodes: needed_nodes,
    };

    let r = cbmt_proof_verify(&proof, &root, &needed_leaves, node_merge);
    assert_eq!(r, 0);
}

#[test]
fn test_proof_verify_fails_on_wrong_root() {
    let leaves = CbmtLeaves {
        nodes: vec![
            int32_to_node(2),
            int32_to_node(3),
            int32_to_node(5),
            int32_to_node(7),
            int32_to_node(11),
        ],
    };
    let mut tree = CbmtTree::default();
    cbmt_build_merkle_tree(&mut tree, &leaves, node_merge);
    let leaf_indices = CbmtIndices {
        values: vec![0, 3],
        capacity: 2,
    };
    let proof = cbmt_tree_build_proof(&tree, &leaf_indices).unwrap();
    let needed_nodes: Vec<CbmtNode> = proof
        .indices
        .values
        .iter()
        .map(|&i| tree.nodes[i as usize].clone())
        .collect();
    let needed_leaves = CbmtLeaves {
        nodes: needed_nodes,
    };
    let wrong_root = int32_to_node(999);
    let r = cbmt_proof_verify(&proof, &wrong_root, &needed_leaves, node_merge);
    assert_eq!(r, CBMT_ERROR_VERIFY_FAILED);
}

#[test]
fn test_proof_root_with_buffers() {
    let leaves = CbmtLeaves {
        nodes: vec![
            int32_to_node(2),
            int32_to_node(3),
            int32_to_node(5),
            int32_to_node(7),
            int32_to_node(11),
        ],
    };
    let mut tree = CbmtTree::default();
    cbmt_build_merkle_tree(&mut tree, &leaves, node_merge);
    let leaf_indices = CbmtIndices {
        values: vec![0, 3],
        capacity: 2,
    };
    let proof = cbmt_tree_build_proof(&tree, &leaf_indices).unwrap();
    let needed_nodes: Vec<CbmtNode> = proof
        .indices
        .values
        .iter()
        .map(|&i| tree.nodes[i as usize].clone())
        .collect();
    let needed_leaves = CbmtLeaves {
        nodes: needed_nodes,
    };

    let mut nodes_data = [0u8; 1024];
    let mut pairs_data = [0u8; 1024];
    let nodes_buffer = CbmtBuffer {
        capacity: nodes_data.len(),
        data: &mut nodes_data,
    };
    let pairs_buffer = CbmtBuffer {
        capacity: pairs_data.len(),
        data: &mut pairs_data,
    };
    let mut root = CbmtNode::default();
    let mut ctx: () = ();
    let r = cbmt_proof_root(
        &proof,
        &mut root,
        &needed_leaves,
        node_merge_ctx::<()>,
        &mut ctx,
        nodes_buffer,
        pairs_buffer,
    );
    assert_eq!(r, 0);
    assert_eq!(node_to_int32(&root), 4);
}

#[test]
fn test_build_merkle_proof() {
    let leaves = CbmtLeaves {
        nodes: vec![
            int32_to_node(2),
            int32_to_node(3),
            int32_to_node(5),
            int32_to_node(7),
            int32_to_node(11),
        ],
    };
    let leaf_indices = CbmtIndices {
        values: vec![0, 3],
        capacity: 2,
    };

    let mut nodes_data = vec![0u8; 4096];
    let mut indices_data = vec![0u8; 1024];
    let mut lemmas_data = vec![0u8; 4096];
    let nodes_buffer = CbmtBuffer {
        capacity: nodes_data.len(),
        data: &mut nodes_data,
    };
    let indices_buffer = CbmtBuffer {
        capacity: indices_data.len(),
        data: &mut indices_data,
    };
    let lemmas_buffer = CbmtBuffer {
        capacity: lemmas_data.len(),
        data: &mut lemmas_data,
    };

    let mut proof = CbmtProof::default();
    let mut ctx: () = ();
    let r = cbmt_build_merkle_proof(
        &mut proof,
        &leaves,
        &leaf_indices,
        node_merge_ctx::<()>,
        &mut ctx,
        nodes_buffer,
        indices_buffer,
        lemmas_buffer,
    );
    assert_eq!(r, 0);
    assert_eq!(proof.indices.values.len(), 2);
    assert_eq!(proof.indices.values[0], 4);
    assert_eq!(proof.indices.values[1], 7);
    assert_eq!(proof.lemmas.len(), 2);
    assert_eq!(node_to_int32(&proof.lemmas[0]), 11);
    assert_eq!(node_to_int32(&proof.lemmas[1]), 2);
}

#[test]
fn test_tree_build_proof_empty_indices_error() {
    let leaves = CbmtLeaves {
        nodes: vec![int32_to_node(2), int32_to_node(3)],
    };
    let mut tree = CbmtTree::default();
    cbmt_build_merkle_tree(&mut tree, &leaves, node_merge);
    let leaf_indices = CbmtIndices {
        values: vec![],
        capacity: 0,
    };
    let res = cbmt_tree_build_proof(&tree, &leaf_indices);
    assert!(res.is_err());
    assert_eq!(res.err().unwrap(), CBMT_ERROR_BUILD_PROOF);
}

#[test]
fn test_tree_root_empty() {
    let tree = CbmtTree::default();
    let root = cbmt_tree_root(&tree);
    assert_eq!(node_to_int32(&root), 0);
}

#[test]
fn test_build_merkle_root_one_leaf() {
    // length=1: no merge calls; root is the leaf itself.
    let leaves = CbmtLeaves {
        nodes: vec![int32_to_node(42)],
    };
    let root = cbmt_build_merkle_root(&leaves, node_merge).unwrap();
    assert_eq!(node_to_int32(&root), 42);
}

fn main() {}
