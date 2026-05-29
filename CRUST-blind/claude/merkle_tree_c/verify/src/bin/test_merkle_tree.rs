use merkle_tree_c::merkle_tree::*;

// === Helpers ===

fn int32_to_node(v: i32) -> CbmtNode {
    CbmtNode { bytes: v.to_le_bytes() }
}

fn node_to_int32(n: &CbmtNode) -> i32 {
    i32::from_le_bytes(n.bytes)
}

fn merge_no_ctx(_: Option<&mut ()>, left: &CbmtNode, right: &CbmtNode) -> CbmtNode {
    let l = i32::from_le_bytes(left.bytes);
    let r = i32::from_le_bytes(right.bytes);
    let v = r.wrapping_sub(l);
    CbmtNode { bytes: v.to_le_bytes() }
}

fn merge_ctx(_: &mut (), left: &CbmtNode, right: &CbmtNode) -> CbmtNode {
    let l = i32::from_le_bytes(left.bytes);
    let r = i32::from_le_bytes(right.bytes);
    let v = r.wrapping_sub(l);
    CbmtNode { bytes: v.to_le_bytes() }
}

// === Constants ===

#[test]
fn test_constants() {
    assert_eq!(CBMT_NODE_SIZE, 4);
    assert_eq!(CBMT_ERROR_OVER_CAPACITY, -1);
    assert_eq!(CBMT_ERROR_QUEUE_EMPTY, -2);
    assert_eq!(CBMT_ERROR_PROOF_ROOT, -3);
    assert_eq!(CBMT_ERROR_BUILD_PROOF, -4);
    assert_eq!(CBMT_ERROR_INVALID_CAPACITY, -5);
    assert_eq!(CBMT_ERROR_VERIFY_FAILED, -6);
    assert_eq!(CBMT_FATAL_BUILD_PROOF, -99);
}

// === cbmt_universal_swap ===

#[test]
fn test_universal_swap_basic() {
    let mut a = [1u8, 2, 3, 4];
    let mut b = [5u8, 6, 7, 8];
    cbmt_universal_swap(&mut a, &mut b, 4);
    assert_eq!(a, [5u8, 6, 7, 8]);
    assert_eq!(b, [1u8, 2, 3, 4]);
}

#[test]
fn test_universal_swap_partial_width() {
    let mut a = [1u8, 2, 3, 4];
    let mut b = [5u8, 6, 7, 8];
    cbmt_universal_swap(&mut a, &mut b, 2);
    assert_eq!(a, [5u8, 6, 3, 4]);
    assert_eq!(b, [1u8, 2, 7, 8]);
}

#[test]
fn test_universal_swap_zero_width() {
    let mut a = [1u8, 2, 3, 4];
    let mut b = [5u8, 6, 7, 8];
    cbmt_universal_swap(&mut a, &mut b, 0);
    assert_eq!(a, [1u8, 2, 3, 4]);
    assert_eq!(b, [5u8, 6, 7, 8]);
}

// === cbmt_simple_bubble_sort ===

#[test]
fn test_bubble_sort_u32_reverse() {
    let mut v: Vec<u32> = vec![3, 1, 4, 1, 5, 9, 2, 6];
    cbmt_simple_bubble_sort(&mut v, cbmt_uint32_reverse_cmp);
    // Reverse-sorted (descending)
    assert_eq!(v, vec![9, 6, 5, 4, 3, 2, 1, 1]);
}

#[test]
fn test_bubble_sort_empty() {
    let mut v: Vec<u32> = vec![];
    cbmt_simple_bubble_sort(&mut v, cbmt_uint32_reverse_cmp);
    assert_eq!(v, Vec::<u32>::new());
}

#[test]
fn test_bubble_sort_single() {
    let mut v: Vec<u32> = vec![42];
    cbmt_simple_bubble_sort(&mut v, cbmt_uint32_reverse_cmp);
    assert_eq!(v, vec![42]);
}

#[test]
fn test_bubble_sort_nodes() {
    let mut v: Vec<CbmtNode> = vec![
        int32_to_node(5),
        int32_to_node(2),
        int32_to_node(8),
        int32_to_node(1),
    ];
    cbmt_simple_bubble_sort(&mut v, cbmt_node_cmp);
    let result: Vec<i32> = v.iter().map(node_to_int32).collect();
    assert_eq!(result, vec![1, 2, 5, 8]);
}

// === cbmt_uint32_reverse_cmp ===

#[test]
fn test_uint32_reverse_cmp() {
    // right - left
    assert_eq!(cbmt_uint32_reverse_cmp(&5, &3), -2);
    assert_eq!(cbmt_uint32_reverse_cmp(&3, &5), 2);
    assert_eq!(cbmt_uint32_reverse_cmp(&7, &7), 0);
    // wrapping behavior:
    assert_eq!(cbmt_uint32_reverse_cmp(&0, &1), 1);
    assert_eq!(cbmt_uint32_reverse_cmp(&1, &0), -1);
}

// === cbmt_node_cmp ===

#[test]
fn test_node_cmp() {
    let a = int32_to_node(3);
    let b = int32_to_node(5);
    let c = int32_to_node(5);
    assert_eq!(cbmt_node_cmp(&a, &b), -2);
    assert_eq!(cbmt_node_cmp(&b, &a), 2);
    assert_eq!(cbmt_node_cmp(&b, &c), 0);
    let d = int32_to_node(-3);
    assert_eq!(cbmt_node_cmp(&d, &a), -6);
}

// === cbmt_node_pair_reverse_cmp ===

#[test]
fn test_node_pair_reverse_cmp() {
    let a = CbmtNodePair { index: 5, node: int32_to_node(0) };
    let b = CbmtNodePair { index: 3, node: int32_to_node(0) };
    assert_eq!(cbmt_node_pair_reverse_cmp(&a, &b), -2);
    assert_eq!(cbmt_node_pair_reverse_cmp(&b, &a), 2);
    let c = CbmtNodePair { index: 5, node: int32_to_node(0) };
    assert_eq!(cbmt_node_pair_reverse_cmp(&a, &c), 0);
}

// === cbmt_buffer_init ===

#[test]
fn test_buffer_init() {
    let mut data = [0u8; 32];
    let mut buf = CbmtBuffer::default();
    cbmt_buffer_init(&mut buf, &mut data);
    assert_eq!(buf.capacity, 32);
    assert_eq!(buf.data.len(), 32);
}

// === cbmt_leaves_init ===

#[test]
fn test_leaves_init() {
    let nodes = vec![int32_to_node(1), int32_to_node(2), int32_to_node(3)];
    let mut leaves = CbmtLeaves { nodes: vec![] };
    cbmt_leaves_init(&mut leaves, nodes);
    assert_eq!(leaves.nodes.len(), 3);
    assert_eq!(node_to_int32(&leaves.nodes[0]), 1);
    assert_eq!(node_to_int32(&leaves.nodes[1]), 2);
    assert_eq!(node_to_int32(&leaves.nodes[2]), 3);
}

// === cbmt_indices_init ===

#[test]
fn test_indices_init() {
    let values = vec![0u32, 1, 2, 3];
    let mut indices = CbmtIndices { values: vec![], capacity: 0 };
    cbmt_indices_init(&mut indices, values);
    assert_eq!(indices.values, vec![0u32, 1, 2, 3]);
    assert_eq!(indices.capacity, 4);
}

// === Queue ops ===

#[test]
fn test_queue_init_ok() {
    let mut data = [0u8; 32];
    let mut buf = CbmtBuffer::default();
    cbmt_buffer_init(&mut buf, &mut data);
    let mut queue = CbmtQueue {
        buffer: CbmtBuffer::default(),
        width: 0,
        length: 0,
        capacity: 0,
        tail: 0,
        head: 0,
    };
    let r = cbmt_queue_init(&mut queue, buf, 4, 8);
    assert_eq!(r, 0);
    assert_eq!(queue.width, 4);
    assert_eq!(queue.capacity, 8);
    assert_eq!(queue.length, 0);
    assert_eq!(queue.head, 0);
    assert_eq!(queue.tail, 0);
}

#[test]
fn test_queue_init_over_capacity() {
    let mut data = [0u8; 16];
    let mut buf = CbmtBuffer::default();
    cbmt_buffer_init(&mut buf, &mut data);
    let mut queue = CbmtQueue {
        buffer: CbmtBuffer::default(),
        width: 0,
        length: 0,
        capacity: 0,
        tail: 0,
        head: 0,
    };
    // capacity*width = 8*4 = 32 > 16
    let r = cbmt_queue_init(&mut queue, buf, 4, 8);
    assert_eq!(r, CBMT_ERROR_OVER_CAPACITY);
}

#[test]
fn test_queue_init_invalid_capacity() {
    let mut data = [0u8; 9];
    let mut buf = CbmtBuffer::default();
    cbmt_buffer_init(&mut buf, &mut data);
    let mut queue = CbmtQueue {
        buffer: CbmtBuffer::default(),
        width: 0,
        length: 0,
        capacity: 0,
        tail: 0,
        head: 0,
    };
    // capacity*width = 2*4 = 8 ≤ 9, but 9 % 4 = 1
    let r = cbmt_queue_init(&mut queue, buf, 4, 2);
    assert_eq!(r, CBMT_ERROR_INVALID_CAPACITY);
}

#[test]
fn test_queue_push_pop_back_front() {
    let mut data = [0u8; 16];
    let mut buf = CbmtBuffer::default();
    cbmt_buffer_init(&mut buf, &mut data);
    let mut queue = CbmtQueue {
        buffer: CbmtBuffer::default(),
        width: 0,
        length: 0,
        capacity: 0,
        tail: 0,
        head: 0,
    };
    assert_eq!(cbmt_queue_init(&mut queue, buf, 4, 4), 0);

    // push three items
    let a = 11u32.to_le_bytes();
    let b = 22u32.to_le_bytes();
    let c = 33u32.to_le_bytes();
    assert_eq!(cbmt_queue_push_back(&mut queue, &a), 0);
    assert_eq!(cbmt_queue_push_back(&mut queue, &b), 0);
    assert_eq!(cbmt_queue_push_back(&mut queue, &c), 0);
    assert_eq!(queue.length, 3);

    // front returns first pushed
    let front = cbmt_queue_front(&queue).unwrap();
    let front_val = u32::from_le_bytes([front[0], front[1], front[2], front[3]]);
    assert_eq!(front_val, 11);

    // pop_front returns 11
    let mut item = [0u8; 4];
    assert_eq!(cbmt_queue_pop_front(&mut queue, &mut item), 0);
    assert_eq!(u32::from_le_bytes(item), 11);
    assert_eq!(queue.length, 2);

    // push_front puts new item at front
    let z = 99u32.to_le_bytes();
    assert_eq!(cbmt_queue_push_front(&mut queue, &z), 0);
    assert_eq!(queue.length, 3);
    let mut item = [0u8; 4];
    cbmt_queue_pop_front(&mut queue, &mut item);
    assert_eq!(u32::from_le_bytes(item), 99);
    cbmt_queue_pop_front(&mut queue, &mut item);
    assert_eq!(u32::from_le_bytes(item), 22);
    cbmt_queue_pop_front(&mut queue, &mut item);
    assert_eq!(u32::from_le_bytes(item), 33);
    assert_eq!(queue.length, 0);
}

#[test]
fn test_queue_push_back_full() {
    let mut data = [0u8; 8];
    let mut buf = CbmtBuffer::default();
    cbmt_buffer_init(&mut buf, &mut data);
    let mut queue = CbmtQueue {
        buffer: CbmtBuffer::default(),
        width: 0,
        length: 0,
        capacity: 0,
        tail: 0,
        head: 0,
    };
    assert_eq!(cbmt_queue_init(&mut queue, buf, 4, 2), 0);

    let a = 1u32.to_le_bytes();
    let b = 2u32.to_le_bytes();
    let c = 3u32.to_le_bytes();
    assert_eq!(cbmt_queue_push_back(&mut queue, &a), 0);
    assert_eq!(cbmt_queue_push_back(&mut queue, &b), 0);
    assert_eq!(cbmt_queue_push_back(&mut queue, &c), CBMT_ERROR_OVER_CAPACITY);
}

#[test]
fn test_queue_pop_empty() {
    let mut data = [0u8; 8];
    let mut buf = CbmtBuffer::default();
    cbmt_buffer_init(&mut buf, &mut data);
    let mut queue = CbmtQueue {
        buffer: CbmtBuffer::default(),
        width: 0,
        length: 0,
        capacity: 0,
        tail: 0,
        head: 0,
    };
    cbmt_queue_init(&mut queue, buf, 4, 2);
    let mut item = [0u8; 4];
    assert_eq!(cbmt_queue_pop_front(&mut queue, &mut item), CBMT_ERROR_QUEUE_EMPTY);
}

#[test]
fn test_queue_front_empty() {
    let mut data = [0u8; 8];
    let mut buf = CbmtBuffer::default();
    cbmt_buffer_init(&mut buf, &mut data);
    let mut queue = CbmtQueue {
        buffer: CbmtBuffer::default(),
        width: 0,
        length: 0,
        capacity: 0,
        tail: 0,
        head: 0,
    };
    cbmt_queue_init(&mut queue, buf, 4, 2);
    assert!(cbmt_queue_front(&queue).is_none());
}

// === cbmt_node_copy ===

#[test]
fn test_node_copy() {
    let src = int32_to_node(123);
    let mut dest = CbmtNode::default();
    cbmt_node_copy(&mut dest, &src);
    assert_eq!(node_to_int32(&dest), 123);
    assert_eq!(dest.bytes, src.bytes);
}

// === cbmt_build_merkle_tree ===

#[test]
fn test_build_merkle_tree_empty() {
    let leaves = CbmtLeaves { nodes: vec![] };
    let mut tree = CbmtTree::default();
    let r = cbmt_build_merkle_tree(&mut tree, &leaves, merge_no_ctx);
    assert_eq!(r, 0);
    assert_eq!(tree.length, 0);
    assert_eq!(tree.capacity, 0);
    assert_eq!(tree.nodes.len(), 0);

    // Root of empty tree is zero node
    let root = cbmt_tree_root(&tree);
    assert_eq!(node_to_int32(&root), 0);
}

#[test]
fn test_build_merkle_tree_one() {
    let leaves = CbmtLeaves { nodes: vec![int32_to_node(42)] };
    let mut tree = CbmtTree::default();
    let r = cbmt_build_merkle_tree(&mut tree, &leaves, merge_no_ctx);
    assert_eq!(r, 0);
    assert_eq!(tree.length, 1);
    assert_eq!(tree.capacity, 1);
    assert_eq!(tree.nodes.len(), 1);
    assert_eq!(node_to_int32(&tree.nodes[0]), 42);

    let root = cbmt_tree_root(&tree);
    assert_eq!(node_to_int32(&root), 42);
}

#[test]
fn test_build_merkle_tree_two() {
    let leaves = CbmtLeaves { nodes: vec![int32_to_node(2), int32_to_node(3)] };
    let mut tree = CbmtTree::default();
    let r = cbmt_build_merkle_tree(&mut tree, &leaves, merge_no_ctx);
    assert_eq!(r, 0);
    assert_eq!(tree.length, 3);
    assert_eq!(tree.capacity, 3);
    assert_eq!(node_to_int32(&tree.nodes[0]), 1);
    assert_eq!(node_to_int32(&tree.nodes[1]), 2);
    assert_eq!(node_to_int32(&tree.nodes[2]), 3);
}

#[test]
fn test_build_merkle_tree_three() {
    let leaves = CbmtLeaves { nodes: vec![int32_to_node(2), int32_to_node(3), int32_to_node(5)] };
    let mut tree = CbmtTree::default();
    let r = cbmt_build_merkle_tree(&mut tree, &leaves, merge_no_ctx);
    assert_eq!(r, 0);
    assert_eq!(tree.length, 5);
    assert_eq!(node_to_int32(&tree.nodes[0]), 0);
    assert_eq!(node_to_int32(&tree.nodes[1]), 2);
    assert_eq!(node_to_int32(&tree.nodes[2]), 2);
    assert_eq!(node_to_int32(&tree.nodes[3]), 3);
    assert_eq!(node_to_int32(&tree.nodes[4]), 5);
}

#[test]
fn test_build_merkle_tree_four() {
    let leaves = CbmtLeaves {
        nodes: vec![
            int32_to_node(2),
            int32_to_node(3),
            int32_to_node(5),
            int32_to_node(7),
        ],
    };
    let mut tree = CbmtTree::default();
    let r = cbmt_build_merkle_tree(&mut tree, &leaves, merge_no_ctx);
    assert_eq!(r, 0);
    assert_eq!(tree.length, 7);
    assert_eq!(node_to_int32(&tree.nodes[0]), 1);
    assert_eq!(node_to_int32(&tree.nodes[1]), 1);
    assert_eq!(node_to_int32(&tree.nodes[2]), 2);
    assert_eq!(node_to_int32(&tree.nodes[3]), 2);
    assert_eq!(node_to_int32(&tree.nodes[4]), 3);
    assert_eq!(node_to_int32(&tree.nodes[5]), 5);
    assert_eq!(node_to_int32(&tree.nodes[6]), 7);
}

#[test]
fn test_build_merkle_tree_five() {
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
    let r = cbmt_build_merkle_tree(&mut tree, &leaves, merge_no_ctx);
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

// === cbmt_tree_root ===

#[test]
fn test_tree_root_empty() {
    let tree = CbmtTree::default();
    let r = cbmt_tree_root(&tree);
    assert_eq!(node_to_int32(&r), 0);
    assert_eq!(r.bytes, [0u8; 4]);
}

#[test]
fn test_tree_root_with_nodes() {
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
    cbmt_build_merkle_tree(&mut tree, &leaves, merge_no_ctx);
    let r = cbmt_tree_root(&tree);
    assert_eq!(node_to_int32(&r), 4);
}

// === cbmt_build_merkle_root ===

#[test]
fn test_build_merkle_root_empty() {
    let leaves = CbmtLeaves { nodes: vec![] };
    let r = cbmt_build_merkle_root(&leaves, merge_no_ctx).unwrap();
    assert_eq!(node_to_int32(&r), 0);
    assert_eq!(r.bytes, [0u8; 4]);
}

#[test]
fn test_build_merkle_root_two() {
    let leaves = CbmtLeaves { nodes: vec![int32_to_node(2), int32_to_node(3)] };
    let r = cbmt_build_merkle_root(&leaves, merge_no_ctx).unwrap();
    assert_eq!(node_to_int32(&r), 1);
}

#[test]
fn test_build_merkle_root_five() {
    let leaves = CbmtLeaves {
        nodes: vec![
            int32_to_node(2),
            int32_to_node(3),
            int32_to_node(5),
            int32_to_node(7),
            int32_to_node(11),
        ],
    };
    let r = cbmt_build_merkle_root(&leaves, merge_no_ctx).unwrap();
    assert_eq!(node_to_int32(&r), 4);
}

#[test]
fn test_build_merkle_root_one() {
    let leaves = CbmtLeaves { nodes: vec![int32_to_node(42)] };
    let r = cbmt_build_merkle_root(&leaves, merge_no_ctx).unwrap();
    assert_eq!(node_to_int32(&r), 42);
}

// === cbmt_tree_build_proof ===

#[test]
fn test_tree_build_proof_empty_tree_errs() {
    let tree = CbmtTree::default();
    let leaf_indices = CbmtIndices { values: vec![0], capacity: 1 };
    let r = cbmt_tree_build_proof(&tree, &leaf_indices);
    assert!(r.is_err());
    assert_eq!(r.unwrap_err(), CBMT_ERROR_BUILD_PROOF);
}

#[test]
fn test_tree_build_proof_empty_indices_errs() {
    let leaves = CbmtLeaves { nodes: vec![int32_to_node(2), int32_to_node(3)] };
    let mut tree = CbmtTree::default();
    cbmt_build_merkle_tree(&mut tree, &leaves, merge_no_ctx);
    let leaf_indices = CbmtIndices { values: vec![], capacity: 0 };
    let r = cbmt_tree_build_proof(&tree, &leaf_indices);
    assert!(r.is_err());
    assert_eq!(r.unwrap_err(), CBMT_ERROR_BUILD_PROOF);
}

#[test]
fn test_tree_build_proof_out_of_range_errs() {
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
    cbmt_build_merkle_tree(&mut tree, &leaves, merge_no_ctx);
    // index 5 is out of range for 5 leaves
    let leaf_indices = CbmtIndices { values: vec![5], capacity: 1 };
    let r = cbmt_tree_build_proof(&tree, &leaf_indices);
    assert!(r.is_err());
    assert_eq!(r.unwrap_err(), CBMT_ERROR_BUILD_PROOF);
}

#[test]
fn test_tree_build_proof_5_idx0() {
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
    cbmt_build_merkle_tree(&mut tree, &leaves, merge_no_ctx);
    let leaf_indices = CbmtIndices { values: vec![0], capacity: 1 };
    let proof = cbmt_tree_build_proof(&tree, &leaf_indices).unwrap();
    assert_eq!(proof.indices.values, vec![4u32]);
    assert_eq!(proof.indices.capacity, 1);
    assert_eq!(proof.lemmas.len(), 2);
    assert_eq!(node_to_int32(&proof.lemmas[0]), 4);
    assert_eq!(node_to_int32(&proof.lemmas[1]), 2);
}

#[test]
fn test_tree_build_proof_5_idx1() {
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
    cbmt_build_merkle_tree(&mut tree, &leaves, merge_no_ctx);
    let leaf_indices = CbmtIndices { values: vec![1], capacity: 1 };
    let proof = cbmt_tree_build_proof(&tree, &leaf_indices).unwrap();
    assert_eq!(proof.indices.values, vec![5u32]);
    assert_eq!(proof.lemmas.len(), 2);
    assert_eq!(node_to_int32(&proof.lemmas[0]), 5);
    assert_eq!(node_to_int32(&proof.lemmas[1]), -2);
}

#[test]
fn test_tree_build_proof_5_idx2() {
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
    cbmt_build_merkle_tree(&mut tree, &leaves, merge_no_ctx);
    let leaf_indices = CbmtIndices { values: vec![2], capacity: 1 };
    let proof = cbmt_tree_build_proof(&tree, &leaf_indices).unwrap();
    assert_eq!(proof.indices.values, vec![6u32]);
    assert_eq!(proof.lemmas.len(), 2);
    assert_eq!(node_to_int32(&proof.lemmas[0]), 3);
    assert_eq!(node_to_int32(&proof.lemmas[1]), -2);
}

#[test]
fn test_tree_build_proof_5_03() {
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
    cbmt_build_merkle_tree(&mut tree, &leaves, merge_no_ctx);
    let leaf_indices = CbmtIndices { values: vec![0, 3], capacity: 2 };
    let proof = cbmt_tree_build_proof(&tree, &leaf_indices).unwrap();
    assert_eq!(proof.indices.values, vec![4u32, 7u32]);
    assert_eq!(proof.indices.capacity, 2);
    assert_eq!(proof.lemmas.len(), 2);
    assert_eq!(node_to_int32(&proof.lemmas[0]), 11);
    assert_eq!(node_to_int32(&proof.lemmas[1]), 2);
}

#[test]
fn test_tree_build_proof_5_14() {
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
    cbmt_build_merkle_tree(&mut tree, &leaves, merge_no_ctx);
    let leaf_indices = CbmtIndices { values: vec![1, 4], capacity: 2 };
    let proof = cbmt_tree_build_proof(&tree, &leaf_indices).unwrap();
    assert_eq!(proof.indices.values, vec![5u32, 8u32]);
    assert_eq!(proof.lemmas.len(), 3);
    assert_eq!(node_to_int32(&proof.lemmas[0]), 7);
    assert_eq!(node_to_int32(&proof.lemmas[1]), 5);
    assert_eq!(node_to_int32(&proof.lemmas[2]), 2);
}

#[test]
fn test_tree_build_proof_5_all_indices() {
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
    cbmt_build_merkle_tree(&mut tree, &leaves, merge_no_ctx);
    let leaf_indices = CbmtIndices { values: vec![0, 1, 2, 3, 4], capacity: 5 };
    let proof = cbmt_tree_build_proof(&tree, &leaf_indices).unwrap();
    assert_eq!(proof.indices.values, vec![4u32, 5, 6, 7, 8]);
    assert_eq!(proof.lemmas.len(), 0);
}

#[test]
fn test_tree_build_proof_4_02() {
    let leaves = CbmtLeaves {
        nodes: vec![
            int32_to_node(2),
            int32_to_node(3),
            int32_to_node(5),
            int32_to_node(7),
        ],
    };
    let mut tree = CbmtTree::default();
    cbmt_build_merkle_tree(&mut tree, &leaves, merge_no_ctx);
    let leaf_indices = CbmtIndices { values: vec![0, 2], capacity: 2 };
    let proof = cbmt_tree_build_proof(&tree, &leaf_indices).unwrap();
    assert_eq!(proof.indices.values, vec![3u32, 5u32]);
    assert_eq!(proof.lemmas.len(), 2);
    assert_eq!(node_to_int32(&proof.lemmas[0]), 7);
    assert_eq!(node_to_int32(&proof.lemmas[1]), 3);
}

#[test]
fn test_tree_build_proof_1_idx0() {
    let leaves = CbmtLeaves { nodes: vec![int32_to_node(42)] };
    let mut tree = CbmtTree::default();
    cbmt_build_merkle_tree(&mut tree, &leaves, merge_no_ctx);
    let leaf_indices = CbmtIndices { values: vec![0], capacity: 1 };
    let proof = cbmt_tree_build_proof(&tree, &leaf_indices).unwrap();
    assert_eq!(proof.indices.values, vec![0u32]);
    assert_eq!(proof.lemmas.len(), 0);
}

// === cbmt_proof_root ===

#[test]
fn test_proof_root_basic() {
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
    cbmt_build_merkle_tree(&mut tree, &leaves, merge_no_ctx);
    let leaf_indices = CbmtIndices { values: vec![0, 3], capacity: 2 };
    let proof = cbmt_tree_build_proof(&tree, &leaf_indices).unwrap();

    // Build "needed_leaves" matching the sorted indices in proof
    let needed_leaves = CbmtLeaves {
        nodes: proof.indices.values.iter().map(|&i| tree.nodes[i as usize].clone()).collect(),
    };
    let mut root = CbmtNode::default();
    let mut empty_a = [0u8; 0];
    let mut empty_b = [0u8; 0];
    let nodes_buf = CbmtBuffer { data: &mut empty_a, capacity: 0 };
    let pairs_buf = CbmtBuffer { data: &mut empty_b, capacity: 0 };
    let mut ctx = ();
    let r = cbmt_proof_root(&proof, &mut root, &needed_leaves, merge_ctx, &mut ctx, nodes_buf, pairs_buf);
    assert_eq!(r, 0);
    assert_eq!(node_to_int32(&root), 4);
}

#[test]
fn test_proof_root_mismatch_leaves_count() {
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
    cbmt_build_merkle_tree(&mut tree, &leaves, merge_no_ctx);
    let leaf_indices = CbmtIndices { values: vec![0, 3], capacity: 2 };
    let proof = cbmt_tree_build_proof(&tree, &leaf_indices).unwrap();

    let only_one = CbmtLeaves { nodes: vec![tree.nodes[4].clone()] };
    let mut root = CbmtNode::default();
    let mut empty_a = [0u8; 0];
    let mut empty_b = [0u8; 0];
    let nodes_buf = CbmtBuffer { data: &mut empty_a, capacity: 0 };
    let pairs_buf = CbmtBuffer { data: &mut empty_b, capacity: 0 };
    let mut ctx = ();
    let r = cbmt_proof_root(&proof, &mut root, &only_one, merge_ctx, &mut ctx, nodes_buf, pairs_buf);
    assert_eq!(r, CBMT_ERROR_PROOF_ROOT);
}

#[test]
fn test_proof_root_empty_leaves() {
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
    cbmt_build_merkle_tree(&mut tree, &leaves, merge_no_ctx);
    let leaf_indices = CbmtIndices { values: vec![0, 3], capacity: 2 };
    let proof = cbmt_tree_build_proof(&tree, &leaf_indices).unwrap();

    let empty = CbmtLeaves { nodes: vec![] };
    let mut root = CbmtNode::default();
    let mut empty_a = [0u8; 0];
    let mut empty_b = [0u8; 0];
    let nodes_buf = CbmtBuffer { data: &mut empty_a, capacity: 0 };
    let pairs_buf = CbmtBuffer { data: &mut empty_b, capacity: 0 };
    let mut ctx = ();
    let r = cbmt_proof_root(&proof, &mut root, &empty, merge_ctx, &mut ctx, nodes_buf, pairs_buf);
    assert_eq!(r, CBMT_ERROR_PROOF_ROOT);
}

// === cbmt_proof_verify ===

#[test]
fn test_proof_verify_ok() {
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
    cbmt_build_merkle_tree(&mut tree, &leaves, merge_no_ctx);
    let root = cbmt_tree_root(&tree);

    let leaf_indices = CbmtIndices { values: vec![0, 3], capacity: 2 };
    let proof = cbmt_tree_build_proof(&tree, &leaf_indices).unwrap();

    let needed_leaves = CbmtLeaves {
        nodes: proof.indices.values.iter().map(|&i| tree.nodes[i as usize].clone()).collect(),
    };
    let r = cbmt_proof_verify(&proof, &root, &needed_leaves, merge_no_ctx);
    assert_eq!(r, 0);
}

#[test]
fn test_proof_verify_wrong_root() {
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
    cbmt_build_merkle_tree(&mut tree, &leaves, merge_no_ctx);
    let bad_root = int32_to_node(99);

    let leaf_indices = CbmtIndices { values: vec![0, 3], capacity: 2 };
    let proof = cbmt_tree_build_proof(&tree, &leaf_indices).unwrap();

    let needed_leaves = CbmtLeaves {
        nodes: proof.indices.values.iter().map(|&i| tree.nodes[i as usize].clone()).collect(),
    };
    let r = cbmt_proof_verify(&proof, &bad_root, &needed_leaves, merge_no_ctx);
    assert_eq!(r, CBMT_ERROR_VERIFY_FAILED);
}

#[test]
fn test_proof_verify_empty_leaves_fails() {
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
    cbmt_build_merkle_tree(&mut tree, &leaves, merge_no_ctx);
    let root = cbmt_tree_root(&tree);
    let leaf_indices = CbmtIndices { values: vec![0, 3], capacity: 2 };
    let proof = cbmt_tree_build_proof(&tree, &leaf_indices).unwrap();
    let empty = CbmtLeaves { nodes: vec![] };
    let r = cbmt_proof_verify(&proof, &root, &empty, merge_no_ctx);
    assert_eq!(r, CBMT_ERROR_PROOF_ROOT);
}

// === cbmt_build_merkle_proof (top-level convenience) ===

#[test]
fn test_build_merkle_proof_5_03() {
    let leaves = CbmtLeaves {
        nodes: vec![
            int32_to_node(2),
            int32_to_node(3),
            int32_to_node(5),
            int32_to_node(7),
            int32_to_node(11),
        ],
    };
    let leaf_indices = CbmtIndices { values: vec![0, 3], capacity: 2 };
    let mut proof = CbmtProof {
        indices: CbmtIndices { values: vec![], capacity: 0 },
        lemmas: vec![],
    };
    let mut empty_a = [0u8; 0];
    let mut empty_b = [0u8; 0];
    let mut empty_c = [0u8; 0];
    let nodes_buf = CbmtBuffer { data: &mut empty_a, capacity: 0 };
    let indices_buf = CbmtBuffer { data: &mut empty_b, capacity: 0 };
    let lemmas_buf = CbmtBuffer { data: &mut empty_c, capacity: 0 };
    let mut ctx = ();
    let r = cbmt_build_merkle_proof(
        &mut proof,
        &leaves,
        &leaf_indices,
        merge_ctx,
        &mut ctx,
        nodes_buf,
        indices_buf,
        lemmas_buf,
    );
    assert_eq!(r, 0);
    assert_eq!(proof.indices.values, vec![4u32, 7u32]);
    assert_eq!(proof.lemmas.len(), 2);
    assert_eq!(node_to_int32(&proof.lemmas[0]), 11);
    assert_eq!(node_to_int32(&proof.lemmas[1]), 2);
}

#[test]
fn test_build_merkle_proof_then_verify() {
    let leaves = CbmtLeaves {
        nodes: vec![
            int32_to_node(2),
            int32_to_node(3),
            int32_to_node(5),
            int32_to_node(7),
            int32_to_node(11),
        ],
    };
    let leaf_indices = CbmtIndices { values: vec![1, 4], capacity: 2 };
    let mut proof = CbmtProof {
        indices: CbmtIndices { values: vec![], capacity: 0 },
        lemmas: vec![],
    };
    let mut empty_a = [0u8; 0];
    let mut empty_b = [0u8; 0];
    let mut empty_c = [0u8; 0];
    let nodes_buf = CbmtBuffer { data: &mut empty_a, capacity: 0 };
    let indices_buf = CbmtBuffer { data: &mut empty_b, capacity: 0 };
    let lemmas_buf = CbmtBuffer { data: &mut empty_c, capacity: 0 };
    let mut ctx = ();
    let r = cbmt_build_merkle_proof(
        &mut proof,
        &leaves,
        &leaf_indices,
        merge_ctx,
        &mut ctx,
        nodes_buf,
        indices_buf,
        lemmas_buf,
    );
    assert_eq!(r, 0);

    // Compute root via build_merkle_root
    let root = cbmt_build_merkle_root(&leaves, merge_no_ctx).unwrap();
    assert_eq!(node_to_int32(&root), 4);

    // Verify
    let mut tree = CbmtTree::default();
    cbmt_build_merkle_tree(&mut tree, &leaves, merge_no_ctx);
    let needed_leaves = CbmtLeaves {
        nodes: proof.indices.values.iter().map(|&i| tree.nodes[i as usize].clone()).collect(),
    };
    let r = cbmt_proof_verify(&proof, &root, &needed_leaves, merge_no_ctx);
    assert_eq!(r, 0);
}

fn main() {}
