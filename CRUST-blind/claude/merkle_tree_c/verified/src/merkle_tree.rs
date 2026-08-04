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

// Sibling, parent, is_left helpers for tree-index arithmetic.
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
    // Reverse order: emulate C `right - left` with unsigned wrap then cast to int.
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
    if capacity.saturating_mul(width) > buffer.capacity {
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
    let start = queue.head * queue.width;
    queue.buffer.data[start..start + queue.width].copy_from_slice(&item[..queue.width]);
    queue.head = (queue.head + 1) % queue.capacity;
    queue.length += 1;
    0
}

pub fn cbmt_queue_push_front(queue: &mut CbmtQueue, item: &[u8]) -> i32 {
    if queue.length >= queue.capacity {
        return CBMT_ERROR_OVER_CAPACITY;
    }
    queue.tail = (queue.tail + queue.capacity - 1) % queue.capacity;
    let start = queue.tail * queue.width;
    queue.buffer.data[start..start + queue.width].copy_from_slice(&item[..queue.width]);
    queue.length += 1;
    0
}

pub fn cbmt_queue_pop_front(queue: &mut CbmtQueue, item: &mut [u8]) -> i32 {
    if queue.length == 0 {
        return CBMT_ERROR_QUEUE_EMPTY;
    }
    let start = queue.tail * queue.width;
    item[..queue.width].copy_from_slice(&queue.buffer.data[start..start + queue.width]);
    queue.tail = (queue.tail + 1) % queue.capacity;
    queue.length -= 1;
    0
}

pub fn cbmt_queue_front<'a>(queue: &'a CbmtQueue<'a>) -> Option<&'a [u8]> {
    if queue.length == 0 {
        return None;
    }
    let start = queue.tail * queue.width;
    Some(&queue.buffer.data[start..start + queue.width])
}

pub fn cbmt_node_copy(dest: &mut CbmtNode, src: &CbmtNode) {
    dest.bytes.copy_from_slice(&src.bytes);
}

pub fn cbmt_node_cmp(left: &CbmtNode, right: &CbmtNode) -> i32 {
    // CBMT_NODE_I32 mode: interpret bytes as little-endian i32.
    let l = i32::from_le_bytes(left.bytes);
    let r = i32::from_le_bytes(right.bytes);
    l.wrapping_sub(r)
}

pub fn cbmt_node_pair_reverse_cmp(left: &CbmtNodePair, right: &CbmtNodePair) -> i32 {
    right.index.wrapping_sub(left.index) as i32
}

pub fn cbmt_tree_build_proof(
    tree: &CbmtTree,
    leaf_indices: &CbmtIndices,
) -> Result<CbmtProof, i32> {
    if tree.length == 0 || leaf_indices.values.is_empty() {
        return Err(CBMT_ERROR_BUILD_PROOF);
    }

    let leaves_count: u32 = ((tree.length >> 1) + 1) as u32;

    // Build initial deque of (leaf_index + leaves_count - 1) values, sorted descending.
    let mut initial: Vec<u32> = leaf_indices
        .values
        .iter()
        .map(|&v| v + (leaves_count - 1))
        .collect();
    // Reverse-sort to align with C behavior (bubble sort with reverse cmp).
    initial.sort_by(|a, b| b.cmp(a));

    let first_value = initial[0];
    if first_value >= ((leaves_count << 1) - 1) {
        return Err(CBMT_ERROR_BUILD_PROOF);
    }

    let mut queue: VecDeque<u32> = VecDeque::from(initial);
    let mut lemmas: Vec<CbmtNode> = Vec::new();

    while let Some(index) = queue.pop_front() {
        if index == 0 {
            if !queue.is_empty() {
                return Err(CBMT_FATAL_BUILD_PROOF);
            }
            break;
        }

        let sibling = cbmt_sibling(index);
        if let Some(&front) = queue.front() {
            if front == sibling {
                queue.pop_front();
            } else {
                lemmas.push(tree.nodes[sibling as usize].clone());
            }
        } else {
            lemmas.push(tree.nodes[sibling as usize].clone());
        }

        let parent = cbmt_parent(index);
        if parent != 0 {
            queue.push_back(parent);
        }
    }

    // Compute final indices and sort them by tree node value (ascending).
    let mut indices_values: Vec<u32> = leaf_indices
        .values
        .iter()
        .map(|&v| v + (leaves_count - 1))
        .collect();
    let n = indices_values.len();
    if n > 1 {
        for i in 0..(n - 1) {
            for j in (i + 1)..n {
                let li = indices_values[i] as usize;
                let ri = indices_values[j] as usize;
                if cbmt_node_cmp(&tree.nodes[li], &tree.nodes[ri]) > 0 {
                    indices_values.swap(i, j);
                }
            }
        }
    }

    let capacity = indices_values.len();
    Ok(CbmtProof {
        indices: CbmtIndices {
            values: indices_values,
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

// Internal helper to compute a proof root using a closure-based merge function.
fn compute_proof_root_impl<F>(
    proof: &CbmtProof,
    leaves: &CbmtLeaves,
    mut merge: F,
) -> Result<CbmtNode, i32>
where
    F: FnMut(&CbmtNode, &CbmtNode) -> CbmtNode,
{
    if leaves.nodes.is_empty() || leaves.nodes.len() != proof.indices.values.len() {
        return Err(CBMT_ERROR_PROOF_ROOT);
    }

    // Clone and sort leaves to align with sorted indices.
    let mut leaves_clone: Vec<CbmtNode> = leaves.nodes.clone();
    cbmt_simple_bubble_sort(&mut leaves_clone, cbmt_node_cmp);

    // Pair each sorted leaf with the corresponding (already-sorted) index.
    let mut pairs: Vec<CbmtNodePair> = (0..leaves.nodes.len())
        .map(|i| CbmtNodePair {
            index: proof.indices.values[i],
            node: leaves_clone[i].clone(),
        })
        .collect();

    // Reverse-sort pairs by index (matches C bubble sort behavior).
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

        let sibling: Option<CbmtNode> = {
            let sib_index = cbmt_sibling(index);
            if let Some(front) = queue.front() {
                if front.index == sib_index {
                    Some(queue.pop_front().unwrap().node)
                } else if lemmas_offset < proof.lemmas.len() {
                    let s = proof.lemmas[lemmas_offset].clone();
                    lemmas_offset += 1;
                    Some(s)
                } else {
                    None
                }
            } else if lemmas_offset < proof.lemmas.len() {
                let s = proof.lemmas[lemmas_offset].clone();
                lemmas_offset += 1;
                Some(s)
            } else {
                None
            }
        };

        if let Some(sib) = sibling {
            let parent_node = if cbmt_is_left(index) {
                merge(&node, &sib)
            } else {
                merge(&sib, &node)
            };
            queue.push_back(CbmtNodePair {
                index: cbmt_parent(index),
                node: parent_node,
            });
        }
    }

    // If we exhaust the queue without seeing index 0, return Ok with default
    // (matching the C function's fall-through `return 0`).
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
    match compute_proof_root_impl(proof, leaves, |l, r| merge(merge_ctx, l, r)) {
        Ok(n) => {
            *root = n;
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
    let target_root = match compute_proof_root_impl(proof, leaves, |l, r| merge(None, l, r)) {
        Ok(n) => n,
        Err(e) => return e,
    };
    if target_root.bytes != expected_root.bytes {
        return CBMT_ERROR_VERIFY_FAILED;
    }
    0
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
    // Pair up from the right: (length-2,length-1), (length-4,length-3), ...
    if length >= 2 {
        let mut i: isize = (length - 1) as isize;
        while i > 0 {
            let left = &leaves.nodes[(i - 1) as usize];
            let right = &leaves.nodes[i as usize];
            let merged = merge(None, left, right);
            queue.push_back(merged);
            i -= 2;
        }
    }
    if length % 2 == 1 {
        queue.push_front(leaves.nodes[0].clone());
    }

    while queue.len() > 1 {
        let right = queue.pop_front().unwrap();
        let left = queue.pop_front().unwrap();
        let merged = merge(None, &left, &right);
        queue.push_back(merged);
    }

    Ok(queue.pop_front().unwrap_or_default())
}

// Internal helper: build merkle tree given a closure-based merge function.
fn build_merkle_tree_impl<F>(tree: &mut CbmtTree, leaves: &CbmtLeaves, mut merge: F) -> i32
where
    F: FnMut(&CbmtNode, &CbmtNode) -> CbmtNode,
{
    if leaves.nodes.is_empty() {
        tree.nodes = Vec::new();
        tree.length = 0;
        tree.capacity = 0;
        return 0;
    }
    let leaves_len = leaves.nodes.len();
    let length = leaves_len * 2 - 1;
    let mut nodes: Vec<CbmtNode> = vec![CbmtNode::default(); length];

    // Place leaves at the bottom of the tree.
    let offset = leaves_len - 1;
    for (i, leaf) in leaves.nodes.iter().enumerate() {
        nodes[offset + i] = leaf.clone();
    }

    // Build internal nodes upward.
    for i in 0..(leaves_len - 1) {
        let rev_idx = leaves_len - 2 - i;
        let li = (rev_idx << 1) + 1;
        let ri = (rev_idx << 1) + 2;
        let merged = {
            let left = &nodes[li];
            let right = &nodes[ri];
            merge(left, right)
        };
        nodes[rev_idx] = merged;
    }

    tree.length = length;
    tree.capacity = length;
    tree.nodes = nodes;
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
