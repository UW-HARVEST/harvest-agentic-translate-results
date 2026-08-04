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

pub fn cbmt_universal_swap(left: &mut [u8], right: &mut [u8], width: usize) {
    let w = width.min(left.len()).min(right.len());
    for i in 0..w {
        std::mem::swap(&mut left[i], &mut right[i]);
    }
}

pub fn cbmt_simple_bubble_sort<T>(slice: &mut [T], cmp: fn(&T, &T) -> i32) {
    let length = slice.len();
    if length <= 1 {
        return;
    }
    for i in 0..length - 1 {
        for j in i + 1..length {
            if cmp(&slice[i], &slice[j]) > 0 {
                slice.swap(i, j);
            }
        }
    }
}

pub fn cbmt_uint32_reverse_cmp(left: &u32, right: &u32) -> i32 {
    // reverse order: returns right - left
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
    if capacity.checked_mul(width).map_or(true, |v| v > buffer.capacity) {
        return CBMT_ERROR_OVER_CAPACITY;
    }
    if width == 0 || buffer.capacity % width != 0 {
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
    let head = queue.head * queue.width;
    queue.buffer.data[head..head + queue.width].copy_from_slice(&item[..queue.width]);
    queue.head = (queue.head + 1) % queue.capacity;
    queue.length += 1;
    0
}

pub fn cbmt_queue_push_front(queue: &mut CbmtQueue, item: &[u8]) -> i32 {
    if queue.length >= queue.capacity {
        return CBMT_ERROR_OVER_CAPACITY;
    }
    queue.tail = (queue.tail + queue.capacity - 1) % queue.capacity;
    let tail = queue.tail * queue.width;
    queue.buffer.data[tail..tail + queue.width].copy_from_slice(&item[..queue.width]);
    queue.length += 1;
    0
}

pub fn cbmt_queue_pop_front(queue: &mut CbmtQueue, item: &mut [u8]) -> i32 {
    if queue.length == 0 {
        return CBMT_ERROR_QUEUE_EMPTY;
    }
    let tail = queue.tail * queue.width;
    item[..queue.width].copy_from_slice(&queue.buffer.data[tail..tail + queue.width]);
    queue.tail = (queue.tail + 1) % queue.capacity;
    queue.length -= 1;
    0
}

pub fn cbmt_queue_front<'a>(queue: &'a CbmtQueue<'a>) -> Option<&'a [u8]> {
    if queue.length == 0 {
        None
    } else {
        let start = queue.tail * queue.width;
        Some(&queue.buffer.data[start..start + queue.width])
    }
}

pub fn cbmt_node_copy(dest: &mut CbmtNode, src: &CbmtNode) {
    dest.bytes.copy_from_slice(&src.bytes);
}

pub fn cbmt_node_cmp(left: &CbmtNode, right: &CbmtNode) -> i32 {
    for i in 0..CBMT_NODE_SIZE {
        let cmp_result = left.bytes[i] as i32 - right.bytes[i] as i32;
        if cmp_result != 0 {
            return cmp_result;
        }
    }
    0
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
    let leaves_count: u32 = ((tree.length >> 1) + 1) as u32;

    // Build initial queue from offset leaf indices
    let mut initial: Vec<u32> = leaf_indices
        .values
        .iter()
        .map(|&v| v + (leaves_count - 1))
        .collect();
    // Sort descending (matches reverse cmp / bubble sort)
    cbmt_simple_bubble_sort(&mut initial, cbmt_uint32_reverse_cmp);
    let mut queue: VecDeque<u32> = initial.into();

    // Validate first (largest) value isn't beyond the tree
    if let Some(&first_value) = queue.front() {
        if first_value >= ((leaves_count << 1) - 1) {
            return Err(CBMT_ERROR_BUILD_PROOF);
        }
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
        let take_sibling = matches!(queue.front(), Some(&v) if v == sibling);
        if take_sibling {
            queue.pop_front();
        } else {
            lemmas.push(tree.nodes[sibling as usize].clone());
        }
        let parent = cbmt_parent(index);
        if parent != 0 {
            queue.push_back(parent);
        }
    }

    // Compute proof.indices: same offset values but sorted to match
    // sorted-by-node-value order used in cbmt_proof_root.
    let mut values: Vec<u32> = leaf_indices
        .values
        .iter()
        .map(|&v| v + (leaves_count - 1))
        .collect();
    let n = values.len();
    if n > 1 {
        for i in 0..n - 1 {
            for j in i + 1..n {
                let li = values[i] as usize;
                let ri = values[j] as usize;
                let order = cbmt_node_cmp(&tree.nodes[li], &tree.nodes[ri]);
                if order > 0 {
                    values.swap(i, j);
                }
            }
        }
    }
    Ok(CbmtProof {
        indices: CbmtIndices {
            values,
            capacity: n,
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

fn cbmt_proof_root_impl(
    proof: &CbmtProof,
    leaves: &CbmtLeaves,
    mut merge: impl FnMut(&CbmtNode, &CbmtNode) -> CbmtNode,
) -> Result<CbmtNode, i32> {
    let proof_len = proof.indices.values.len();
    if leaves.nodes.is_empty() || proof_len == 0 {
        return Err(CBMT_ERROR_PROOF_ROOT);
    }
    // The Rust API allows the caller to pass either the "needed" leaves
    // (matching proof.indices in count, as in the C original) or the full
    // list of leaves used to build the tree. Detect which case we have and,
    // when given the full list, extract the needed leaves using the
    // tree-space indices stored in `proof.indices.values`.
    let needed_leaves: Vec<CbmtNode> = if leaves.nodes.len() == proof_len {
        leaves.nodes.clone()
    } else {
        let leaves_count = leaves.nodes.len();
        let offset = (leaves_count - 1) as u32;
        let mut extracted = Vec::with_capacity(proof_len);
        for &idx in &proof.indices.values {
            if idx < offset {
                return Err(CBMT_ERROR_PROOF_ROOT);
            }
            let li = (idx - offset) as usize;
            if li >= leaves_count {
                return Err(CBMT_ERROR_PROOF_ROOT);
            }
            extracted.push(leaves.nodes[li].clone());
        }
        extracted
    };
    // Clone leaves and sort by node cmp ascending (to align with proof indices ordering)
    let mut leaves_clone: Vec<CbmtNode> = needed_leaves;
    cbmt_simple_bubble_sort(&mut leaves_clone, cbmt_node_cmp);

    // Pair each sorted leaf with its proof index
    let mut pairs_vec: Vec<CbmtNodePair> = (0..proof_len)
        .map(|i| CbmtNodePair {
            index: proof.indices.values[i],
            node: leaves_clone[i].clone(),
        })
        .collect();
    // Sort pairs by index descending
    cbmt_simple_bubble_sort(&mut pairs_vec, cbmt_node_pair_reverse_cmp);
    let mut pairs: VecDeque<CbmtNodePair> = pairs_vec.into();

    let mut lemmas_offset: usize = 0;
    while let Some(pair_current) = pairs.pop_front() {
        let index = pair_current.index;
        let node = pair_current.node;
        if index == 0 {
            if proof.lemmas.len() == lemmas_offset && pairs.is_empty() {
                return Ok(node);
            } else {
                return Err(CBMT_ERROR_PROOF_ROOT);
            }
        }
        let sibling_idx = cbmt_sibling(index);
        let sibling: Option<CbmtNode> = match pairs.front() {
            Some(front) if front.index == sibling_idx => {
                Some(pairs.pop_front().unwrap().node)
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
        if let Some(sib) = sibling {
            let parent_node = if cbmt_is_left(index) {
                merge(&node, &sib)
            } else {
                merge(&sib, &node)
            };
            let parent_idx = cbmt_parent(index);
            pairs.push_back(CbmtNodePair {
                index: parent_idx,
                node: parent_node,
            });
        }
    }
    // C falls through with success and root unchanged. Return zero node to mirror.
    Ok(CbmtNode::default())
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
    match cbmt_proof_root_impl(proof, leaves, |l, r| merge(merge_ctx, l, r)) {
        Ok(r) => {
            *root = r;
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
    let target_root = match cbmt_proof_root_impl(proof, leaves, |l, r| merge(None, l, r)) {
        Ok(r) => r,
        Err(e) => return e,
    };
    if target_root.bytes != expected_root.bytes {
        CBMT_ERROR_VERIFY_FAILED
    } else {
        0
    }
}

fn cbmt_build_merkle_root_impl(
    leaves: &CbmtLeaves,
    mut merge: impl FnMut(&CbmtNode, &CbmtNode) -> CbmtNode,
) -> Result<CbmtNode, i32> {
    let length = leaves.nodes.len();
    if length == 0 {
        return Ok(CbmtNode::default());
    }
    let mut queue: VecDeque<CbmtNode> = VecDeque::new();
    let mut i: isize = length as isize - 1;
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
        let right = queue.pop_front().unwrap();
        let left = queue.pop_front().unwrap();
        let merged = merge(&left, &right);
        queue.push_back(merged);
    }
    Ok(queue.pop_front().unwrap_or_default())
}

pub fn cbmt_build_merkle_root(
    leaves: &CbmtLeaves,
    merge: fn(Option<&mut ()>, &CbmtNode, &CbmtNode) -> CbmtNode,
) -> Result<CbmtNode, i32> {
    cbmt_build_merkle_root_impl(leaves, |l, r| merge(None, l, r))
}

fn cbmt_build_merkle_tree_impl(
    tree: &mut CbmtTree,
    leaves: &CbmtLeaves,
    mut merge: impl FnMut(&CbmtNode, &CbmtNode) -> CbmtNode,
) -> i32 {
    let length = leaves.nodes.len();
    if length == 0 {
        tree.nodes = Vec::new();
        tree.length = 0;
        tree.capacity = 0;
        return 0;
    }
    let total = length * 2 - 1;
    tree.nodes = vec![CbmtNode::default(); total];
    tree.length = total;
    tree.capacity = total;
    let offset = length - 1;
    for i in 0..length {
        tree.nodes[offset + i] = leaves.nodes[i].clone();
    }
    if length >= 2 {
        for i in 0..length - 1 {
            let rev_idx = length - 2 - i;
            let left_idx = rev_idx * 2 + 1;
            let right_idx = rev_idx * 2 + 2;
            let left = tree.nodes[left_idx].clone();
            let right = tree.nodes[right_idx].clone();
            tree.nodes[rev_idx] = merge(&left, &right);
        }
    }
    0
}

pub fn cbmt_build_merkle_tree(
    tree: &mut CbmtTree,
    leaves: &CbmtLeaves,
    merge: fn(Option<&mut ()>, &CbmtNode, &CbmtNode) -> CbmtNode,
) -> i32 {
    cbmt_build_merkle_tree_impl(tree, leaves, |l, r| merge(None, l, r))
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
    let ret = cbmt_build_merkle_tree_impl(&mut tree, leaves, |l, r| merge(merge_ctx, l, r));
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
