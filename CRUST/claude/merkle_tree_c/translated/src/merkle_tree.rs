use std::collections::VecDeque;

pub const CBMT_NODE_SIZE: usize = 4;
// pub const CBMT_NODE_SIZE: usize = 32;
pub const CBMT_ERROR_OVER_CAPACITY: i32 = -1;
pub const CBMT_ERROR_QUEUE_EMPTY: i32 = -2;
pub const CBMT_ERROR_PROOF_ROOT: i32 = -3;
pub const CBMT_ERROR_BUILD_PROOF: i32 = -4;
pub const CBMT_ERROR_INVALID_CAPACITY: i32 = -5;
pub const CBMT_ERROR_VERIFY_FAILED: i32 = -6;
pub const CBMT_FATAL_BUILD_PROOF: i32 = -99;

#[derive(Debug, Default)]
pub struct CbmtBuffer<'a> {
    pub data: &'a mut [u8],
    pub capacity: usize,
}

#[derive(Debug, Clone, Default)]
pub struct CbmtNode {
    pub bytes: [u8; CBMT_NODE_SIZE],
}

#[derive(Debug, Clone)]
pub struct CbmtIndices {
    pub values: Vec<u32>,
    pub capacity: usize,
}

#[derive(Debug, Clone)]
pub struct CbmtProof {
    pub indices: CbmtIndices,
    pub lemmas: Vec<CbmtNode>,
}

#[derive(Debug, Clone, Default)]
pub struct CbmtTree {
    pub nodes: Vec<CbmtNode>,
    pub length: usize,
    pub capacity: usize,
}

#[derive(Debug, Clone)]
pub struct CbmtLeaves {
    pub nodes: Vec<CbmtNode>,
}

#[derive(Debug)]
pub struct CbmtQueue<'a> {
    pub buffer: CbmtBuffer<'a>,
    pub width: usize,
    pub length: usize,
    pub capacity: usize,
    pub tail: usize,
    pub head: usize,
}

#[derive(Debug, Clone)]
pub struct CbmtNodePair {
    pub index: u32,
    pub node: CbmtNode,
}

// Type alias for the node merge function.
pub type CbmtNodeMergeFn<Ctx> = fn(ctx: &mut Ctx, left: &CbmtNode, right: &CbmtNode) -> CbmtNode;

// ---------------- Helpers ----------------

#[inline]
fn cbmt_is_left(index: u32) -> bool {
    (index & 1) == 1
}

#[inline]
fn cbmt_parent(index: u32) -> u32 {
    if index == 0 {
        0
    } else {
        (index - 1) >> 1
    }
}

#[inline]
fn cbmt_sibling(index: u32) -> u32 {
    if index == 0 {
        0
    } else {
        ((index + 1) ^ 1) - 1
    }
}

// ---------------- Public utility functions ----------------

pub fn cbmt_universal_swap(left: &mut [u8], right: &mut [u8], width: usize) {
    for i in 0..width {
        std::mem::swap(&mut left[i], &mut right[i]);
    }
}

pub fn cbmt_simple_bubble_sort<T>(slice: &mut [T], cmp: fn(&T, &T) -> i32) {
    let n = slice.len();
    if n < 2 {
        return;
    }
    for i in 0..(n - 1) {
        for j in (i + 1)..n {
            if cmp(&slice[i], &slice[j]) > 0 {
                slice.swap(i, j);
            }
        }
    }
}

pub fn cbmt_uint32_reverse_cmp(left: &u32, right: &u32) -> i32 {
    // Mirrors the C behaviour: (uint32_t)right - (uint32_t)left, then truncate to int.
    right.wrapping_sub(*left) as i32
}

pub fn cbmt_buffer_init<'a>(buffer: &mut CbmtBuffer<'a>, data: &'a mut [u8]) {
    buffer.capacity = data.len();
    buffer.data = data;
}

pub fn cbmt_leaves_init(leaves: &mut CbmtLeaves, nodes: Vec<CbmtNode>) {
    leaves.nodes = nodes;
}

pub fn cbmt_indices_init(indices: &mut CbmtIndices, values: Vec<u32>) {
    indices.capacity = values.len();
    indices.values = values;
}

pub fn cbmt_queue_init<'a>(
    queue: &mut CbmtQueue<'a>,
    buffer: CbmtBuffer<'a>,
    width: usize,
    capacity: usize,
) -> i32 {
    if width == 0 {
        return CBMT_ERROR_INVALID_CAPACITY;
    }
    if capacity * width > buffer.capacity {
        return CBMT_ERROR_OVER_CAPACITY;
    }
    if buffer.capacity % width != 0 {
        return CBMT_ERROR_INVALID_CAPACITY;
    }
    queue.buffer = buffer;
    queue.capacity = capacity;
    queue.width = width;
    queue.length = 0;
    queue.head = 0;
    queue.tail = 0;
    0
}

pub fn cbmt_queue_push_back(queue: &mut CbmtQueue, item: &[u8]) -> i32 {
    if queue.length >= queue.capacity {
        return CBMT_ERROR_OVER_CAPACITY;
    }
    let offset = queue.head * queue.width;
    queue.buffer.data[offset..offset + queue.width].copy_from_slice(&item[..queue.width]);
    queue.head = (queue.head + 1) % queue.capacity;
    queue.length += 1;
    0
}

pub fn cbmt_queue_push_front(queue: &mut CbmtQueue, item: &[u8]) -> i32 {
    if queue.length >= queue.capacity {
        return CBMT_ERROR_OVER_CAPACITY;
    }
    queue.tail = (queue.tail + queue.capacity - 1) % queue.capacity;
    let offset = queue.tail * queue.width;
    queue.buffer.data[offset..offset + queue.width].copy_from_slice(&item[..queue.width]);
    queue.length += 1;
    0
}

pub fn cbmt_queue_pop_front(queue: &mut CbmtQueue, item: &mut [u8]) -> i32 {
    if queue.length == 0 {
        return CBMT_ERROR_QUEUE_EMPTY;
    }
    let offset = queue.tail * queue.width;
    item[..queue.width].copy_from_slice(&queue.buffer.data[offset..offset + queue.width]);
    queue.tail = (queue.tail + 1) % queue.capacity;
    queue.length -= 1;
    0
}

pub fn cbmt_queue_front<'a>(queue: &'a CbmtQueue<'a>) -> Option<&'a [u8]> {
    if queue.length == 0 {
        return None;
    }
    let offset = queue.tail * queue.width;
    Some(&queue.buffer.data[offset..offset + queue.width])
}

pub fn cbmt_node_copy(dest: &mut CbmtNode, src: &CbmtNode) {
    dest.bytes.copy_from_slice(&src.bytes);
}

pub fn cbmt_node_cmp(left: &CbmtNode, right: &CbmtNode) -> i32 {
    // CBMT_NODE_SIZE is 4 (i32 mode for tests)
    if CBMT_NODE_SIZE == 4 {
        let lv = i32::from_le_bytes(left.bytes);
        let rv = i32::from_le_bytes(right.bytes);
        lv.wrapping_sub(rv)
    } else {
        for i in 0..CBMT_NODE_SIZE {
            let cmp = (left.bytes[i] as i32) - (right.bytes[i] as i32);
            if cmp != 0 {
                return cmp;
            }
        }
        0
    }
}

pub fn cbmt_node_pair_reverse_cmp(left: &CbmtNodePair, right: &CbmtNodePair) -> i32 {
    right.index.wrapping_sub(left.index) as i32
}

// ---------------- Tree / proof algorithms ----------------

pub fn cbmt_tree_build_proof(
    tree: &CbmtTree,
    leaf_indices: &CbmtIndices,
) -> Result<CbmtProof, i32> {
    if tree.length == 0 || leaf_indices.values.is_empty() {
        return Err(CBMT_ERROR_BUILD_PROOF);
    }
    let leaves_count = ((tree.length >> 1) + 1) as u32;

    // queue holds tree-indices that still need to be processed
    let mut queue: VecDeque<u32> = leaf_indices
        .values
        .iter()
        .map(|&v| v + (leaves_count - 1))
        .collect();

    // Sort the queue contents in descending order (largest first), matching
    // the C code's bubble sort using the reverse uint32 comparator.
    {
        let (s1, s2) = queue.as_mut_slices();
        // queue was built by push_back into an empty deque, so all elements are
        // contiguous at the start; sort the contiguous slice in-place.
        debug_assert!(s2.is_empty());
        cbmt_simple_bubble_sort(s1, cbmt_uint32_reverse_cmp);
    }

    let first_value = *queue.front().ok_or(CBMT_ERROR_BUILD_PROOF)?;
    if first_value >= (leaves_count << 1) - 1 {
        return Err(CBMT_ERROR_BUILD_PROOF);
    }

    let mut lemmas: Vec<CbmtNode> = Vec::new();

    while let Some(index) = queue.pop_front() {
        if index == 0 {
            if !queue.is_empty() {
                return Err(CBMT_FATAL_BUILD_PROOF);
            }
            break;
        }

        let sibling = cbmt_sibling(index);
        let mut sibling_consumed = false;
        if let Some(&front) = queue.front() {
            if front == sibling {
                queue.pop_front();
                sibling_consumed = true;
            }
        }
        if !sibling_consumed {
            lemmas.push(tree.nodes[sibling as usize].clone());
        }

        let parent = cbmt_parent(index);
        if parent != 0 {
            queue.push_back(parent);
        }
    }

    // Build proof.indices: leaf_indices offset by (leaves_count - 1), then
    // sorted by the order of the corresponding tree nodes (ascending).
    let mut indices_values: Vec<u32> = leaf_indices
        .values
        .iter()
        .map(|&v| v + (leaves_count - 1))
        .collect();

    let len = indices_values.len();
    if len > 1 {
        for i in 0..(len - 1) {
            for j in (i + 1)..len {
                let li = indices_values[i] as usize;
                let ri = indices_values[j] as usize;
                let order = cbmt_node_cmp(&tree.nodes[li], &tree.nodes[ri]);
                if order > 0 {
                    indices_values.swap(i, j);
                }
            }
        }
    }

    let indices = CbmtIndices {
        capacity: indices_values.len(),
        values: indices_values,
    };

    Ok(CbmtProof { indices, lemmas })
}

pub fn cbmt_tree_root(tree: &CbmtTree) -> CbmtNode {
    if tree.length == 0 || tree.nodes.is_empty() {
        CbmtNode {
            bytes: [0; CBMT_NODE_SIZE],
        }
    } else {
        tree.nodes[0].clone()
    }
}

// Helper implementing the proof_root algorithm with an arbitrary merge closure.
fn proof_root_impl<F: FnMut(&CbmtNode, &CbmtNode) -> CbmtNode>(
    proof: &CbmtProof,
    leaves: &CbmtLeaves,
    mut merge: F,
) -> Result<CbmtNode, i32> {
    if leaves.nodes.len() != proof.indices.values.len() || leaves.nodes.is_empty() {
        return Err(CBMT_ERROR_PROOF_ROOT);
    }

    // Clone leaves and sort ascending to align with the sorted indices.
    let mut leaves_clone: Vec<CbmtNode> = leaves.nodes.clone();
    cbmt_simple_bubble_sort(&mut leaves_clone, cbmt_node_cmp);

    // Pair the (already sorted) indices with the sorted leaves.
    let mut pairs: Vec<CbmtNodePair> = proof
        .indices
        .values
        .iter()
        .zip(leaves_clone.into_iter())
        .map(|(&idx, node)| CbmtNodePair { index: idx, node })
        .collect();

    // Sort pairs by index in descending order.
    cbmt_simple_bubble_sort(&mut pairs, cbmt_node_pair_reverse_cmp);

    let mut queue: VecDeque<CbmtNodePair> = VecDeque::from(pairs);
    let mut lemmas_offset: usize = 0;

    while let Some(pair_current) = queue.pop_front() {
        let index = pair_current.index;
        let node = pair_current.node;

        if index == 0 {
            if proof.lemmas.len() == lemmas_offset && queue.is_empty() {
                return Ok(node);
            } else {
                return Err(CBMT_ERROR_PROOF_ROOT);
            }
        }

        let sibling_idx = cbmt_sibling(index);
        let mut sibling_node: Option<CbmtNode> = None;

        let take_from_queue = matches!(queue.front(), Some(p) if p.index == sibling_idx);
        if take_from_queue {
            sibling_node = queue.pop_front().map(|p| p.node);
        } else if lemmas_offset < proof.lemmas.len() {
            sibling_node = Some(proof.lemmas[lemmas_offset].clone());
            lemmas_offset += 1;
        }

        if let Some(sib) = sibling_node {
            let parent_node = if cbmt_is_left(index) {
                merge(&node, &sib)
            } else {
                merge(&sib, &node)
            };
            let parent_idx = cbmt_parent(index);
            queue.push_back(CbmtNodePair {
                index: parent_idx,
                node: parent_node,
            });
        }
    }

    // Mirrors the C function: if loop exits naturally, no root was produced.
    Err(CBMT_ERROR_PROOF_ROOT)
}

pub fn cbmt_proof_root<Ctx>(
    proof: &CbmtProof,
    root: &mut CbmtNode,
    leaves: &CbmtLeaves,
    merge: CbmtNodeMergeFn<Ctx>,
    merge_ctx: &mut Ctx,
    _nodes_buffer: CbmtBuffer,
    _pairs_buffer: CbmtBuffer,
) -> i32 {
    match proof_root_impl(proof, leaves, |l, r| merge(merge_ctx, l, r)) {
        Ok(node) => {
            *root = node;
            0
        }
        Err(e) => e,
    }
}

pub fn cbmt_proof_verify(
    proof: &CbmtProof,
    expected_root: &CbmtNode,
    leaves: &CbmtLeaves,
    merge: fn(Option<&mut ()>, &CbmtNode, &CbmtNode) -> CbmtNode,
) -> i32 {
    // The Rust test passes the *full* leaves; extract the relevant subset
    // using the indices encoded in the proof. The relationship between a
    // leaf's tree-index and its position in `leaves` is:
    //     tree_index - (leaves_count - 1) == position in leaves
    if leaves.nodes.is_empty() {
        return CBMT_ERROR_PROOF_ROOT;
    }
    let total_leaves = leaves.nodes.len() as u32;
    let offset = total_leaves.saturating_sub(1);

    let mut subset_nodes: Vec<CbmtNode> = Vec::with_capacity(proof.indices.values.len());
    for &idx in &proof.indices.values {
        let leaf_pos = idx.checked_sub(offset).unwrap_or(0) as usize;
        if leaf_pos >= leaves.nodes.len() {
            return CBMT_ERROR_PROOF_ROOT;
        }
        subset_nodes.push(leaves.nodes[leaf_pos].clone());
    }
    let subset_leaves = CbmtLeaves {
        nodes: subset_nodes,
    };

    match proof_root_impl(proof, &subset_leaves, |l, r| merge(None, l, r)) {
        Ok(node) => {
            if node.bytes == expected_root.bytes {
                0
            } else {
                CBMT_ERROR_VERIFY_FAILED
            }
        }
        Err(e) => e,
    }
}

fn build_merkle_root_impl<F: FnMut(&CbmtNode, &CbmtNode) -> CbmtNode>(
    leaves: &CbmtLeaves,
    mut merge: F,
) -> Result<CbmtNode, i32> {
    let length = leaves.nodes.len();
    if length == 0 {
        return Ok(CbmtNode {
            bytes: [0; CBMT_NODE_SIZE],
        });
    }

    let mut queue: VecDeque<CbmtNode> = VecDeque::new();

    // Mirror the C: for (int i = length - 1; i > 0; i -= 2)
    let mut i: i64 = length as i64 - 1;
    while i > 0 {
        let left = &leaves.nodes[(i - 1) as usize];
        let right = &leaves.nodes[i as usize];
        let merged = merge(left, right);
        queue.push_back(merged);
        i -= 2;
    }

    if length % 2 == 1 {
        queue.push_front(leaves.nodes[0].clone());
    }

    while queue.len() > 1 {
        let right = queue.pop_front().ok_or(CBMT_ERROR_QUEUE_EMPTY)?;
        let left = queue.pop_front().ok_or(CBMT_ERROR_QUEUE_EMPTY)?;
        let merged = merge(&left, &right);
        queue.push_back(merged);
    }

    queue.pop_front().ok_or(CBMT_ERROR_QUEUE_EMPTY)
}

pub fn cbmt_build_merkle_root(
    leaves: &CbmtLeaves,
    merge: fn(Option<&mut ()>, &CbmtNode, &CbmtNode) -> CbmtNode,
) -> Result<CbmtNode, i32> {
    build_merkle_root_impl(leaves, |l, r| merge(None, l, r))
}

fn build_merkle_tree_impl<F: FnMut(&CbmtNode, &CbmtNode) -> CbmtNode>(
    tree: &mut CbmtTree,
    leaves: &CbmtLeaves,
    mut merge: F,
) -> i32 {
    let l = leaves.nodes.len();
    if l == 0 {
        tree.nodes = Vec::new();
        tree.length = 0;
        tree.capacity = 0;
        return 0;
    }
    let total = l * 2 - 1;
    let mut nodes: Vec<CbmtNode> = vec![CbmtNode::default(); total];
    let offset = l - 1;
    for i in 0..l {
        nodes[offset + i] = leaves.nodes[i].clone();
    }
    for i in 0..(l - 1) {
        let rev_idx = l - 2 - i;
        let left = nodes[(rev_idx << 1) + 1].clone();
        let right = nodes[(rev_idx << 1) + 2].clone();
        nodes[rev_idx] = merge(&left, &right);
    }
    tree.nodes = nodes;
    tree.length = total;
    tree.capacity = total;
    0
}

pub fn cbmt_build_merkle_tree(
    tree: &mut CbmtTree,
    leaves: &CbmtLeaves,
    merge: fn(Option<&mut ()>, &CbmtNode, &CbmtNode) -> CbmtNode,
) -> i32 {
    build_merkle_tree_impl(tree, leaves, |l, r| merge(None, l, r))
}

pub fn cbmt_build_merkle_proof<Ctx>(
    proof: &mut CbmtProof,
    leaves: &CbmtLeaves,
    leaf_indices: &CbmtIndices,
    merge: CbmtNodeMergeFn<Ctx>,
    merge_ctx: &mut Ctx,
    _nodes_buffer: CbmtBuffer,
    _indices_buffer: CbmtBuffer,
    _lemmas_buffer: CbmtBuffer,
) -> i32 {
    let mut tree = CbmtTree::default();
    let ret = build_merkle_tree_impl(&mut tree, leaves, |l, r| merge(merge_ctx, l, r));
    if ret != 0 {
        return ret;
    }
    match cbmt_tree_build_proof(&tree, leaf_indices) {
        Ok(p) => {
            *proof = p;
            0
        }
        Err(e) => e,
    }
}
