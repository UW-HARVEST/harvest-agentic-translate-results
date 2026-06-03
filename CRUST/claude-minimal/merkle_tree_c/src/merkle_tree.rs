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
// (In an idiomatic Rust implementation you might use generics or traits.)
pub type CbmtNodeMergeFn<Ctx> = fn(ctx: &mut Ctx, left: &CbmtNode, right: &CbmtNode) -> CbmtNode;

// Helper: cbmt_is_left(index)  => ((index & 1) == 1)
#[inline]
fn is_left(index: u32) -> bool {
    (index & 1) == 1
}

// Helper: cbmt_parent(index) => index == 0 ? 0 : ((index - 1) >> 1)
#[inline]
fn parent_idx(index: u32) -> u32 {
    if index == 0 {
        0
    } else {
        (index - 1) >> 1
    }
}

// Helper: cbmt_sibling(index) => index == 0 ? 0 : ((index + 1) ^ 1) - 1
#[inline]
fn sibling_idx(index: u32) -> u32 {
    if index == 0 {
        0
    } else {
        ((index + 1) ^ 1) - 1
    }
}

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
    // Reverse order: larger values first.  Use i64 to avoid u32 wrap.
    (*right as i64 - *left as i64) as i32
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
    let head_offset = queue.head * queue.width;
    queue.buffer.data[head_offset..head_offset + queue.width]
        .copy_from_slice(&item[..queue.width]);
    queue.head = (queue.head + 1) % queue.capacity;
    queue.length += 1;
    0
}

pub fn cbmt_queue_push_front(queue: &mut CbmtQueue, item: &[u8]) -> i32 {
    if queue.length >= queue.capacity {
        return CBMT_ERROR_OVER_CAPACITY;
    }
    queue.tail = (queue.tail + queue.capacity - 1) % queue.capacity;
    let tail_offset = queue.tail * queue.width;
    queue.buffer.data[tail_offset..tail_offset + queue.width]
        .copy_from_slice(&item[..queue.width]);
    queue.length += 1;
    0
}

pub fn cbmt_queue_pop_front(queue: &mut CbmtQueue, item: &mut [u8]) -> i32 {
    if queue.length == 0 {
        return CBMT_ERROR_QUEUE_EMPTY;
    }
    let tail_offset = queue.tail * queue.width;
    item[..queue.width].copy_from_slice(&queue.buffer.data[tail_offset..tail_offset + queue.width]);
    queue.tail = (queue.tail + 1) % queue.capacity;
    queue.length -= 1;
    0
}

pub fn cbmt_queue_front<'a>(queue: &'a CbmtQueue<'a>) -> Option<&'a [u8]> {
    if queue.length == 0 {
        return None;
    }
    let tail_offset = queue.tail * queue.width;
    Some(&queue.buffer.data[tail_offset..tail_offset + queue.width])
}

pub fn cbmt_node_copy(dest: &mut CbmtNode, src: &CbmtNode) {
    dest.bytes.copy_from_slice(&src.bytes);
}

pub fn cbmt_node_cmp(left: &CbmtNode, right: &CbmtNode) -> i32 {
    // Matches CBMT_NODE_I32 mode in the C reference.
    let l = i32::from_le_bytes(left.bytes);
    let r = i32::from_le_bytes(right.bytes);
    ((l as i64) - (r as i64)) as i32
}

pub fn cbmt_node_pair_reverse_cmp(left: &CbmtNodePair, right: &CbmtNodePair) -> i32 {
    (right.index as i64 - left.index as i64) as i32
}

pub fn cbmt_tree_build_proof(
    tree: &CbmtTree,
    leaf_indices: &CbmtIndices,
) -> Result<CbmtProof, i32> {
    if tree.length == 0 || leaf_indices.values.is_empty() {
        return Err(CBMT_ERROR_BUILD_PROOF);
    }
    let leaves_count = ((tree.length >> 1) + 1) as u32;

    // Build initial queue: each leaf-index shifted by (leaves_count - 1)
    let mut shifted: Vec<u32> = leaf_indices
        .values
        .iter()
        .map(|v| v + (leaves_count - 1))
        .collect();
    // Sort in reverse (descending) order to mirror the bubble sort + reverse_cmp in C.
    shifted.sort_by(|a, b| b.cmp(a));

    let first_value = shifted[0];
    if first_value >= (leaves_count << 1) - 1 {
        return Err(CBMT_ERROR_BUILD_PROOF);
    }

    let mut queue: VecDeque<u32> = shifted.into();
    let mut lemmas: Vec<CbmtNode> = Vec::new();

    while let Some(index) = queue.pop_front() {
        if index == 0 {
            if !queue.is_empty() {
                return Err(CBMT_FATAL_BUILD_PROOF);
            }
            break;
        }

        let sibling = sibling_idx(index);
        let consume_sibling = matches!(queue.front(), Some(&front) if front == sibling);
        if consume_sibling {
            queue.pop_front();
        } else {
            lemmas.push(tree.nodes[sibling as usize].clone());
        }

        let parent = parent_idx(index);
        if parent != 0 {
            queue.push_back(parent);
        }
    }

    // Re-compute the indices, sorted by tree[index] ascending (node_cmp).
    let mut indices: Vec<u32> = leaf_indices
        .values
        .iter()
        .map(|v| v + (leaves_count - 1))
        .collect();

    let n = indices.len();
    if n > 1 {
        for i in 0..(n - 1) {
            for j in (i + 1)..n {
                let left_index = indices[i];
                let right_index = indices[j];
                let order = cbmt_node_cmp(
                    &tree.nodes[left_index as usize],
                    &tree.nodes[right_index as usize],
                );
                if order > 0 {
                    indices[i] = right_index;
                    indices[j] = left_index;
                }
            }
        }
    }

    let capacity = indices.len();
    Ok(CbmtProof {
        indices: CbmtIndices {
            values: indices,
            capacity,
        },
        lemmas,
    })
}

pub fn cbmt_tree_root(tree: &CbmtTree) -> CbmtNode {
    if tree.length == 0 {
        CbmtNode::default()
    } else {
        tree.nodes[0].clone()
    }
}

/// Internal helper: compute the merkle root from a proof and the full leaf set.
/// `merge` is a closure that handles any context capture.
fn proof_root_compute<F>(
    proof: &CbmtProof,
    leaves: &CbmtLeaves,
    mut merge: F,
) -> Result<CbmtNode, i32>
where
    F: FnMut(&CbmtNode, &CbmtNode) -> CbmtNode,
{
    let total_leaves = leaves.nodes.len();
    let proof_len = proof.indices.values.len();
    if total_leaves == 0 || proof_len == 0 {
        return Err(CBMT_ERROR_PROOF_ROOT);
    }

    // Determine the leaves we actually need based on proof.indices.
    // proof.indices contains *tree* indices.  For a tree built from `total_leaves`
    // leaves, the offset between tree index and leaf index is (total_leaves - 1).
    // If `leaves.length == proof_len` we treat it as a pre-selected leaf set
    // (matching the C API contract).  Otherwise we extract the needed leaves
    // from the full leaf list.
    let needed_leaves: Vec<CbmtNode> = if total_leaves == proof_len {
        leaves.nodes.clone()
    } else {
        let leaves_offset = total_leaves - 1;
        let mut v = Vec::with_capacity(proof_len);
        for &tree_idx in &proof.indices.values {
            let idx = (tree_idx as usize).checked_sub(leaves_offset);
            match idx {
                Some(i) if i < total_leaves => v.push(leaves.nodes[i].clone()),
                _ => return Err(CBMT_ERROR_PROOF_ROOT),
            }
        }
        v
    };

    // Sort needed_leaves ascending by node_cmp (matches the C bubble sort).
    let mut needed_leaves_sorted = needed_leaves;
    let n = needed_leaves_sorted.len();
    if n > 1 {
        for i in 0..(n - 1) {
            for j in (i + 1)..n {
                if cbmt_node_cmp(&needed_leaves_sorted[i], &needed_leaves_sorted[j]) > 0 {
                    needed_leaves_sorted.swap(i, j);
                }
            }
        }
    }

    // Pair up sorted leaves with proof.indices (which are already sorted by
    // their corresponding tree node value ascending).
    let mut pairs: Vec<CbmtNodePair> = proof
        .indices
        .values
        .iter()
        .zip(needed_leaves_sorted.into_iter())
        .map(|(idx, node)| CbmtNodePair { index: *idx, node })
        .collect();

    // Sort pairs by index descending (reverse order).
    pairs.sort_by(|a, b| b.index.cmp(&a.index));

    let mut queue: VecDeque<CbmtNodePair> = pairs.into();
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

        let sib_idx = sibling_idx(index);
        let sibling: Option<CbmtNode> = match queue.front() {
            Some(front) if front.index == sib_idx => {
                let popped = queue.pop_front().unwrap();
                Some(popped.node)
            }
            _ => {
                if lemmas_offset < proof.lemmas.len() {
                    let s = proof.lemmas[lemmas_offset].clone();
                    lemmas_offset += 1;
                    Some(s)
                } else {
                    None
                }
            }
        };

        if let Some(sibling) = sibling {
            let parent_node = if is_left(index) {
                merge(&node, &sibling)
            } else {
                merge(&sibling, &node)
            };
            queue.push_back(CbmtNodePair {
                index: parent_idx(index),
                node: parent_node,
            });
        }
    }

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
    let result = proof_root_compute(proof, leaves, |l, r| merge(merge_ctx, l, r));
    match result {
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
    let result = proof_root_compute(proof, leaves, |l, r| merge(None, l, r));
    match result {
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

pub fn cbmt_build_merkle_root(
    leaves: &CbmtLeaves,
    merge: fn(Option<&mut ()>, &CbmtNode, &CbmtNode) -> CbmtNode,
) -> Result<CbmtNode, i32> {
    let length = leaves.nodes.len();
    if length == 0 {
        return Ok(CbmtNode::default());
    }

    let mut queue: VecDeque<CbmtNode> = VecDeque::new();

    // Pre-merge pass: pair up leaves from end towards the start.
    let mut i = length as isize - 1;
    while i > 0 {
        let left = &leaves.nodes[(i - 1) as usize];
        let right = &leaves.nodes[i as usize];
        let merged = merge(None, left, right);
        queue.push_back(merged);
        i -= 2;
    }
    if length % 2 == 1 {
        queue.push_front(leaves.nodes[0].clone());
    }

    while queue.len() > 1 {
        let right = queue.pop_front().ok_or(CBMT_ERROR_QUEUE_EMPTY)?;
        let left = queue.pop_front().ok_or(CBMT_ERROR_QUEUE_EMPTY)?;
        let merged = merge(None, &left, &right);
        queue.push_back(merged);
    }

    queue.pop_front().ok_or(CBMT_ERROR_QUEUE_EMPTY)
}

pub fn cbmt_build_merkle_tree(
    tree: &mut CbmtTree,
    leaves: &CbmtLeaves,
    merge: fn(Option<&mut ()>, &CbmtNode, &CbmtNode) -> CbmtNode,
) -> i32 {
    let leaves_len = leaves.nodes.len();
    if leaves_len > 0 {
        let length = leaves_len * 2 - 1;
        tree.nodes = vec![CbmtNode::default(); length];
        tree.length = length;
        tree.capacity = length;

        let offset = leaves_len - 1;
        for i in 0..leaves_len {
            tree.nodes[offset + i] = leaves.nodes[i].clone();
        }
        for i in 0..(leaves_len - 1) {
            let rev_idx = leaves_len - 2 - i;
            let left = tree.nodes[(rev_idx << 1) + 1].clone();
            let right = tree.nodes[(rev_idx << 1) + 2].clone();
            tree.nodes[rev_idx] = merge(None, &left, &right);
        }
    } else {
        tree.length = 0;
        tree.capacity = 0;
        tree.nodes = Vec::new();
    }
    0
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
    let leaves_len = leaves.nodes.len();
    if leaves_len == 0 {
        return CBMT_ERROR_BUILD_PROOF;
    }
    let length = leaves_len * 2 - 1;
    let mut tree_nodes = vec![CbmtNode::default(); length];
    let offset = leaves_len - 1;
    for i in 0..leaves_len {
        tree_nodes[offset + i] = leaves.nodes[i].clone();
    }
    if leaves_len > 1 {
        for i in 0..(leaves_len - 1) {
            let rev_idx = leaves_len - 2 - i;
            let left = tree_nodes[(rev_idx << 1) + 1].clone();
            let right = tree_nodes[(rev_idx << 1) + 2].clone();
            tree_nodes[rev_idx] = merge(merge_ctx, &left, &right);
        }
    }
    let tree = CbmtTree {
        nodes: tree_nodes,
        length,
        capacity: length,
    };
    match cbmt_tree_build_proof(&tree, leaf_indices) {
        Ok(p) => {
            *proof = p;
            0
        }
        Err(e) => e,
    }
}
